//! One bounded native service lifetime, separate from remote operation custody.

use std::{
    num::NonZeroUsize,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    task::{Context, Poll},
    thread::{self, JoinHandle},
    time::Duration,
};

use pale_core::records::ArtifactDigest;
use pale_ports::{
    A2aExchange, A2aExchangeCoordinate, A2aExchangeError, A2aExchangeEvidence, A2aExchangeLimits,
    A2aExchangePort, A2aExchangeRequest, A2aPreparedExchange,
};
use tokio::sync::{mpsc, watch, OwnedSemaphorePermit, Semaphore};

use crate::{dns::BoundedResolver, transport, wire};

/// Fixed per-service resource ceilings; not a process-wide service-count limit.
#[derive(Clone, Copy, Debug)]
pub struct A2aServiceLimits {
    /// Maximum retained exchanges, including unread evidence readers.
    pub exchanges: NonZeroUsize,
    /// Maximum physical OS resolver jobs, including jobs outliving requests.
    pub dns_jobs: NonZeroUsize,
    /// Maximum addresses retained from one resolver result.
    pub dns_addresses: NonZeroUsize,
    /// Maximum request-specific ceilings accepted by this service.
    pub exchange: A2aExchangeLimits,
    /// Maximum explicitly supplied root-certificate bytes in aggregate.
    pub certificate_bytes: NonZeroUsize,
    /// Maximum explicitly supplied root certificates.
    pub certificates: NonZeroUsize,
}

/// The host-selected certificate-verification policy. Neither mode disables verification.
pub enum A2aTlsTrust {
    /// Use the platform verifier and its current root store.
    Platform,
    /// Use exactly these DER root certificates, excluding platform roots.
    CertificatesOnly(Vec<Vec<u8>>),
}

/// Observable lifetime of this native service, not remote task state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A2aServiceState {
    /// The sole executor is preparing its runtime and verified HTTP client.
    Starting,
    /// New bounded exchanges may be submitted.
    Running,
    /// New work is rejected; outstanding physical jobs still belong here.
    Draining {
        /// Retained exchanges still holding their service permit.
        exchanges: usize,
        /// Physical resolver jobs that have not returned.
        dns_jobs: usize,
    },
    /// The executor has exited. Failure is a physical service failure only.
    Stopped(Result<(), A2aExchangeError>),
}

struct Shared {
    stopping: AtomicBool,
    ready: AtomicBool,
    dns: Arc<AtomicUsize>,
    permits: Arc<Semaphore>,
    result: Mutex<Option<Result<(), A2aExchangeError>>>,
}

struct Job {
    submitted: std::time::Instant,
    prepared: A2aPreparedExchange,
    sender: mpsc::Sender<A2aExchangeEvidence>,
    cancelled: watch::Receiver<bool>,
    terminal: Arc<Mutex<Option<Result<(), A2aExchangeError>>>>,
    _permit: Arc<OwnedSemaphorePermit>,
}

struct TerminalGuard(Arc<Mutex<Option<Result<(), A2aExchangeError>>>>);

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if let Ok(mut terminal) = self.0.lock() {
            terminal.get_or_insert(Err(if std::thread::panicking() {
                A2aExchangeError::ServiceUnavailable
            } else {
                A2aExchangeError::Cancelled
            }));
        }
    }
}

/// Owned executor and transport configuration for the native SDK composition.
///
/// Call `begin_shutdown`, retain this value, and poll `state` until stopped.
/// Drop never waits for an uninterruptible OS resolver. If dropped while
/// draining, its executor retains its physical jobs but its join handle is
/// detached. Therefore this adapter proves bounds per retained service only;
/// repeated construction and abandonment of services is not resource-bounded.
/// The SDK composition must establish its single retained-service lifetime
/// before claiming whole-service cancellation or exposing this capability.
pub struct A2aService {
    limits: A2aServiceLimits,
    tls_trust_input_sha256: ArtifactDigest,
    shared: Arc<Shared>,
    sender: mpsc::Sender<Job>,
    shutdown: watch::Sender<bool>,
    thread: Option<JoinHandle<()>>,
}

impl A2aService {
    /// Start one owned executor without performing an A2A endpoint call.
    ///
    /// # Errors
    /// Rejects invalid limits, excessive certificate input, or thread creation.
    /// Runtime/client preparation errors are reported through `state`.
    pub fn spawn(limits: A2aServiceLimits, trust: A2aTlsTrust) -> Result<Self, A2aExchangeError> {
        limits.exchange.validate()?;
        if limits.exchanges.get() > Semaphore::MAX_PERMITS
            || limits.dns_jobs.get() > Semaphore::MAX_PERMITS
            || limits.exchange.queued_events.get() > Semaphore::MAX_PERMITS
            || limits
                .exchange
                .evidence_bytes
                .get()
                .checked_mul(limits.exchanges.get())
                .is_none()
        {
            return Err(A2aExchangeError::InvalidLimits);
        }
        let tls_trust_input_sha256 = tls_trust_input_digest(&trust, limits)?;
        let shared = Arc::new(Shared {
            stopping: AtomicBool::new(false),
            ready: AtomicBool::new(false),
            dns: Arc::new(AtomicUsize::new(0)),
            permits: Arc::new(Semaphore::new(limits.exchanges.get())),
            result: Mutex::new(None),
        });
        let (sender, receiver) = mpsc::channel(limits.exchanges.get());
        let (shutdown, shutdown_receiver) = watch::channel(false);
        let thread_shared = shared.clone();
        let thread = thread::Builder::new()
            .name("pale-a2a".to_owned())
            .spawn(move || {
                let result = serve(limits, trust, receiver, shutdown_receiver, &thread_shared);
                if let Ok(mut terminal) = thread_shared.result.lock() {
                    *terminal = Some(result);
                }
            })
            .map_err(|_| A2aExchangeError::ServiceUnavailable)?;
        Ok(Self {
            limits,
            tls_trust_input_sha256,
            shared,
            sender,
            shutdown,
            thread: Some(thread),
        })
    }

    /// Reject new work and cancel local HTTP reads, without blocking this caller.
    pub fn begin_shutdown(&self) {
        self.shared.stopping.store(true, Ordering::Release);
        let _ = self.shutdown.send(true);
    }

    /// Read lifecycle state and reap the executor only after it has exited.
    /// A draining result, however long-lived, never means shutdown completed.
    pub fn state(&mut self) -> A2aServiceState {
        if self.thread.as_ref().is_some_and(JoinHandle::is_finished)
            && self
                .thread
                .take()
                .is_some_and(|thread| thread.join().is_err())
        {
            return A2aServiceState::Stopped(Err(A2aExchangeError::ServiceUnavailable));
        }
        if self.thread.is_none() {
            return A2aServiceState::Stopped(
                self.shared
                    .result
                    .lock()
                    .ok()
                    .and_then(|result| *result)
                    .unwrap_or(Err(A2aExchangeError::ServiceUnavailable)),
            );
        }
        if self.shared.stopping.load(Ordering::Acquire) {
            A2aServiceState::Draining {
                exchanges: self.limits.exchanges.get() - self.shared.permits.available_permits(),
                dns_jobs: self.shared.dns.load(Ordering::Acquire),
            }
        } else if self.shared.ready.load(Ordering::Acquire) {
            A2aServiceState::Running
        } else {
            A2aServiceState::Starting
        }
    }
}

impl Drop for A2aService {
    fn drop(&mut self) {
        self.begin_shutdown();
    }
}

impl A2aExchangePort for A2aService {
    fn prepare(
        &self,
        request: A2aExchangeRequest,
    ) -> Result<A2aPreparedExchange, A2aExchangeError> {
        check_limits(request.limits(), self.limits.exchange)?;
        wire::prepare(request, self.tls_trust_input_sha256.clone())
    }

    fn start(
        &self,
        prepared: A2aPreparedExchange,
    ) -> Result<Box<dyn A2aExchange>, A2aExchangeError> {
        let submitted = std::time::Instant::now();
        check_limits(prepared.request().limits(), self.limits.exchange)?;
        if prepared.coordinate().tls_trust_input_sha256() != &self.tls_trust_input_sha256 {
            return Err(A2aExchangeError::CoordinateMismatch);
        }
        if self.shared.stopping.load(Ordering::Acquire)
            || !self.shared.ready.load(Ordering::Acquire)
        {
            return Err(A2aExchangeError::ServiceUnavailable);
        }
        wire::validate_prepared(&prepared)?;
        let permit = self
            .shared
            .permits
            .clone()
            .try_acquire_owned()
            .map_err(|_| A2aExchangeError::LimitExceeded)?;
        let permit = Arc::new(permit);
        let (sender, receiver) = mpsc::channel(prepared.request().limits().queued_events.get());
        let (cancel, cancelled) = watch::channel(false);
        let terminal = Arc::new(Mutex::new(None));
        let reader = ExchangeReader {
            coordinate: prepared.coordinate().clone(),
            receiver,
            cancel,
            terminal: terminal.clone(),
            ended: false,
            _permit: permit.clone(),
        };
        self.sender
            .try_send(Job {
                submitted,
                prepared,
                sender,
                cancelled,
                terminal,
                _permit: permit,
            })
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => A2aExchangeError::LimitExceeded,
                mpsc::error::TrySendError::Closed(_) => A2aExchangeError::ServiceUnavailable,
            })?;
        Ok(Box::new(reader))
    }
}

struct ExchangeReader {
    coordinate: A2aExchangeCoordinate,
    receiver: mpsc::Receiver<A2aExchangeEvidence>,
    cancel: watch::Sender<bool>,
    terminal: Arc<Mutex<Option<Result<(), A2aExchangeError>>>>,
    ended: bool,
    // A completed producer cannot admit replacements while its unread result
    // still consumes a retained exchange slot.
    _permit: Arc<OwnedSemaphorePermit>,
}

impl A2aExchange for ExchangeReader {
    fn coordinate(&self) -> &A2aExchangeCoordinate {
        &self.coordinate
    }
    fn poll_next(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<A2aExchangeEvidence, A2aExchangeError>>> {
        if self.ended {
            return Poll::Ready(None);
        }
        match self.receiver.poll_recv(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(evidence)) => Poll::Ready(Some(Ok(evidence))),
            Poll::Ready(None) => {
                self.ended = true;
                match self.terminal.lock().ok().and_then(|result| *result) {
                    Some(Ok(())) => Poll::Ready(None),
                    Some(Err(error)) => Poll::Ready(Some(Err(error))),
                    None => Poll::Ready(Some(Err(A2aExchangeError::ServiceUnavailable))),
                }
            }
        }
    }
    fn cancel(&mut self) {
        let _ = self.cancel.send(true);
    }
}

impl Drop for ExchangeReader {
    fn drop(&mut self) {
        self.cancel();
    }
}

fn serve(
    limits: A2aServiceLimits,
    trust: A2aTlsTrust,
    mut receiver: mpsc::Receiver<Job>,
    mut shutdown: watch::Receiver<bool>,
    shared: &Arc<Shared>,
) -> Result<(), A2aExchangeError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(limits.dns_jobs.get())
        .build()
        .map_err(|_| A2aExchangeError::ServiceUnavailable)?;
    runtime.block_on(async {
        let client = client(limits, trust, shared.dns.clone())?;
        shared.ready.store(true, Ordering::Release);
        let mut jobs = tokio::task::JoinSet::new();
        loop {
            tokio::select! {
                biased;
                _ = shutdown.changed() => break,
                joined = jobs.join_next(), if !jobs.is_empty() => {
                    if joined.is_some_and(|result| result.is_err()) {
                        shared.stopping.store(true, Ordering::Release);
                        break;
                    }
                }
                job = receiver.recv() => {
                    let Some(mut job) = job else { break; };
                    let client = client.clone();
                    let mut stop = shutdown.clone();
                    jobs.spawn(async move {
                        let _exchange_permit = job._permit;
                        let _terminal_guard = TerminalGuard(job.terminal.clone());
                        let result = if *job.cancelled.borrow() || *stop.borrow() {
                            Err(A2aExchangeError::Cancelled)
                        } else {
                            tokio::select! {
                                biased;
                                _ = stop.changed() => Err(A2aExchangeError::Cancelled),
                                _ = job.cancelled.changed() => Err(A2aExchangeError::Cancelled),
                                result = transport::run(job.prepared, client, job.sender.clone(), job.submitted) => result,
                            }
                        };
                        if let Ok(mut terminal) = job.terminal.lock() { *terminal = Some(result); }
                    });
                }
            }
        }
        shared.stopping.store(true, Ordering::Release);
        receiver.close();
        while let Ok(job) = receiver.try_recv() {
            if let Ok(mut terminal) = job.terminal.lock() { *terminal = Some(Err(A2aExchangeError::Cancelled)); }
        }
        jobs.abort_all();
        while jobs.join_next().await.is_some() {}
        drop(client);
        while shared.dns.load(Ordering::Acquire) != 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        Ok(())
    })
}

fn client(
    limits: A2aServiceLimits,
    trust: A2aTlsTrust,
    outstanding: Arc<AtomicUsize>,
) -> Result<reqwest::Client, A2aExchangeError> {
    let mut builder = reqwest::Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .no_proxy()
        .http1_only()
        .pool_max_idle_per_host(0)
        .no_gzip()
        .no_brotli()
        .no_deflate()
        .no_zstd()
        .tls_sslkeylogfile(false)
        .connection_verbose(false)
        .connect_timeout(limits.exchange.connect_timeout)
        .dns_resolver(Arc::new(BoundedResolver::new(
            limits.dns_jobs,
            limits.dns_addresses,
            outstanding,
        )));
    if let A2aTlsTrust::CertificatesOnly(certificates) = trust {
        let roots = certificates
            .iter()
            .map(|bytes| reqwest::Certificate::from_der(bytes))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| A2aExchangeError::Connection)?;
        builder = builder.tls_certs_only(roots);
    }
    builder.build().map_err(|_| A2aExchangeError::Connection)
}

fn tls_trust_input_digest(
    trust: &A2aTlsTrust,
    limits: A2aServiceLimits,
) -> Result<ArtifactDigest, A2aExchangeError> {
    let mut bytes = Vec::new();
    match trust {
        A2aTlsTrust::Platform => bytes.push(0),
        A2aTlsTrust::CertificatesOnly(certificates) => {
            if certificates.is_empty() || certificates.len() > limits.certificates.get() {
                return Err(A2aExchangeError::InvalidLimits);
            }
            let total = certificates
                .iter()
                .try_fold(0usize, |total, bytes| total.checked_add(bytes.len()))
                .filter(|total| *total <= limits.certificate_bytes.get())
                .ok_or(A2aExchangeError::LimitExceeded)?;
            let framed = certificates
                .len()
                .checked_mul(std::mem::size_of::<u64>())
                .and_then(|overhead| total.checked_add(overhead))
                .and_then(|total| total.checked_add(1))
                .ok_or(A2aExchangeError::LimitExceeded)?;
            bytes
                .try_reserve_exact(framed)
                .map_err(|_| A2aExchangeError::LimitExceeded)?;
            bytes.push(1);
            for certificate in certificates {
                bytes.extend_from_slice(&(certificate.len() as u64).to_be_bytes());
                bytes.extend_from_slice(certificate);
            }
        }
    }
    // This is exact private TLS trust-input identity, not a new public
    // signature domain and not a snapshot of the platform certificate store.
    Ok(ArtifactDigest::hash_bytes(&bytes))
}

fn check_limits(
    value: A2aExchangeLimits,
    maximum: A2aExchangeLimits,
) -> Result<(), A2aExchangeError> {
    value.validate()?;
    if value.request_bytes > maximum.request_bytes
        || value.response_bytes > maximum.response_bytes
        || value.evidence_bytes > maximum.evidence_bytes
        || value.line_bytes > maximum.line_bytes
        || value.event_bytes > maximum.event_bytes
        || value.events > maximum.events
        || value.queued_events > maximum.queued_events
        || value.connect_timeout > maximum.connect_timeout
        || value.read_timeout > maximum.read_timeout
        || value.total_timeout > maximum.total_timeout
    {
        return Err(A2aExchangeError::LimitExceeded);
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/internal/service.rs"]
mod tests;
