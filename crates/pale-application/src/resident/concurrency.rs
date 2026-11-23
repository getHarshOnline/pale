//! Transport-neutral concurrency mechanics for resident compilation work.
//!
//! This module owns one deliberately small process-local algebra:
//!
//! - a caller-supplied [`ResidentRequestControl`] binds request identity,
//!   resolved caller/workspace admission coordinates, monotonic deadline, and
//!   cooperative cancellation;
//! - [`ResidentCompilationCoordinator`] admits at most an explicitly supplied
//!   number of requests per caller, per workspace, globally, and an independent
//!   number of distinct computations; and
//! - overlapping work with the same [`CompilationInputDigest`] has one producer
//!   and any number of independently interruptible joiners.
//!
//! For one digest, the law below bounds overlapping computation to one producer
//! and independently bounds caller, workspace, global-request, and producer
//! occupancy. `C(d)` is the active computation set for digest `d`; `q_c`,
//! `q_w`, and `q` are admitted request counts per caller, per workspace, and
//! globally, with limits `L_c`, `L_w`, and `L`. `p` is distinct producer
//! occupancy with limit `P`. This is coalescing, not caching: a later
//! non-overlapping request may compute the digest again. Cancellation and
//! deadlines close only the affected waiter; once a producer starts, this
//! coordinator never pretends that synchronous Rust work was preempted.
//!
//! No default capacity, queue, fairness order, worker pool, IPC carrier,
//! operation-retention store, runtime activation, or native effect is selected
//! here. A future resident host must supply policy values and call this mechanic
//! only with a compiler-produced digest for the admitted pre-render input.

#![doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/resident-concurrency-laws.display.mathml"
))]
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroUsize;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, Weak};
use std::time::Instant;

use pale_core::records::{
    CompilationInputDigest, RequestCallerIdV1, ResidentRequestIdV1, ResidentWorkspaceIdV1,
};

/// Create the sole cancellation trigger and a clonable observation token.
///
/// Cancellation is process-local control state. It is neither durable
/// cancellation authority nor proof that synchronous work stopped.
#[must_use]
pub fn resident_cancellation_pair() -> (ResidentCancellationTrigger, ResidentCancellation) {
    let shared = Arc::new(CancellationState::default());
    (
        ResidentCancellationTrigger {
            shared: Arc::clone(&shared),
        },
        ResidentCancellation { shared },
    )
}

#[derive(Default)]
/// Shared monotonic cancellation bit plus weak wake observers.
struct CancellationState {
    /// One-way process-local cancellation transition.
    cancelled: AtomicBool,
    /// Wait points notified when cancellation becomes observable.
    observers: Mutex<Vec<Weak<FlightWake>>>,
}

/// Sole process-local capability that can request cooperative cancellation.
pub struct ResidentCancellationTrigger {
    shared: Arc<CancellationState>,
}

impl ResidentCancellationTrigger {
    /// Request cancellation and wake registered coordination waiters.
    ///
    /// Returns `true` only for the transition from active to cancelled.
    /// Dropping this trigger does not imply cancellation.
    #[must_use]
    pub fn cancel(&self) -> bool {
        let first = !self.shared.cancelled.swap(true, Ordering::AcqRel);
        let observers = match self.shared.observers.lock() {
            Ok(mut observers) => {
                let live = observers
                    .iter()
                    .filter_map(Weak::upgrade)
                    .collect::<Vec<_>>();
                observers.retain(|observer| observer.strong_count() > 0);
                live
            }
            Err(poisoned) => poisoned
                .into_inner()
                .iter()
                .filter_map(Weak::upgrade)
                .collect(),
        };
        for observer in observers {
            observer.notify_all();
        }
        first
    }
}

/// Clonable process-local observation of one cancellation request.
#[derive(Clone)]
pub struct ResidentCancellation {
    shared: Arc<CancellationState>,
}

impl ResidentCancellation {
    /// Report whether cancellation has been requested.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.shared.cancelled.load(Ordering::Acquire)
    }

    fn observe(&self, observer: &Arc<FlightWake>) -> Result<(), ResidentWorkCoordinationError> {
        let mut observers = self.shared.observers.lock().map_err(|_| {
            ResidentWorkCoordinationError::SynchronizationPoisoned {
                region: "cancellation observers",
            }
        })?;
        observers.retain(|registered| registered.strong_count() > 0);
        observers.push(Arc::downgrade(observer));
        Ok(())
    }
}

/// Exact process-local control coordinates for one resident request.
///
/// [`Instant`] deliberately keeps the deadline monotonic and private to the
/// resident process. A transport adapter must translate its own wire deadline
/// before constructing this value; this type defines no clock-skew policy.
pub struct ResidentRequestControl {
    request_id: ResidentRequestIdV1,
    caller_id: RequestCallerIdV1,
    workspace_id: ResidentWorkspaceIdV1,
    deadline: Instant,
    cancellation: ResidentCancellation,
}

impl ResidentRequestControl {
    /// Bind caller-generated identity to an explicit deadline and cancellation token.
    #[must_use]
    pub fn new(
        request_id: ResidentRequestIdV1,
        caller_id: RequestCallerIdV1,
        workspace_id: ResidentWorkspaceIdV1,
        deadline: Instant,
        cancellation: ResidentCancellation,
    ) -> Self {
        Self {
            request_id,
            caller_id,
            workspace_id,
            deadline,
            cancellation,
        }
    }

    /// Borrow the caller-generated correlation identity.
    #[must_use]
    pub const fn request_id(&self) -> &ResidentRequestIdV1 {
        &self.request_id
    }

    /// Borrow the resolved caller identity used only for admission accounting.
    ///
    /// Carrying this typed identity does not authenticate the caller.
    #[must_use]
    pub const fn caller_id(&self) -> &RequestCallerIdV1 {
        &self.caller_id
    }

    /// Borrow the workspace identity used only for admission accounting.
    ///
    /// This coordinate does not select ambient workspace state or authority.
    #[must_use]
    pub const fn workspace_id(&self) -> &ResidentWorkspaceIdV1 {
        &self.workspace_id
    }

    /// Return the monotonic deadline used inside this process.
    #[must_use]
    pub const fn deadline(&self) -> Instant {
        self.deadline
    }

    /// Classify every interruption that is true at `now` without precedence loss.
    #[must_use]
    pub fn interruption_at(&self, now: Instant) -> Option<ResidentRequestInterruption> {
        match (self.cancellation.is_cancelled(), now >= self.deadline) {
            (false, false) => None,
            (true, false) => Some(ResidentRequestInterruption::Cancelled),
            (false, true) => Some(ResidentRequestInterruption::DeadlineElapsed),
            (true, true) => Some(ResidentRequestInterruption::CancelledAfterDeadline),
        }
    }

    fn interruption(&self) -> Option<ResidentRequestInterruption> {
        self.interruption_at(Instant::now())
    }
}

/// Closed process-local interruption state for one request waiter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResidentRequestInterruption {
    /// Cancellation was observed before the deadline elapsed.
    Cancelled,
    /// The monotonic deadline elapsed without observed cancellation.
    DeadlineElapsed,
    /// Both cancellation and an elapsed deadline were observed.
    CancelledAfterDeadline,
}

/// Explicit positive ceilings for non-queuing resident work admission.
///
/// The three request ceilings bound retained waiter state. The computation
/// ceiling separately bounds distinct producers; exact-digest joiners consume
/// request admission but no additional computation slot. Values are host inputs
/// and this type defines no defaults or relationship between them.
///
/// Request admission evaluates its caller, workspace, and global bounds as one
/// atomic meet and reports every bound that rejects the request. Computation
/// admission is deliberately separate because exact-digest joiners do not
/// consume another producer slot. The module-level resident concurrency law
/// defines the corresponding symbols and inequalities once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResidentAdmissionLimits {
    maximum_requests_per_caller: NonZeroUsize,
    maximum_requests_per_workspace: NonZeroUsize,
    maximum_requests_global: NonZeroUsize,
    maximum_parallel_computations: NonZeroUsize,
}

impl ResidentAdmissionLimits {
    /// Bind all four explicit positive ceilings without selecting policy values.
    #[must_use]
    pub const fn new(
        maximum_requests_per_caller: NonZeroUsize,
        maximum_requests_per_workspace: NonZeroUsize,
        maximum_requests_global: NonZeroUsize,
        maximum_parallel_computations: NonZeroUsize,
    ) -> Self {
        Self {
            maximum_requests_per_caller,
            maximum_requests_per_workspace,
            maximum_requests_global,
            maximum_parallel_computations,
        }
    }

    /// Return the per-caller in-flight request ceiling.
    #[must_use]
    pub const fn maximum_requests_per_caller(self) -> NonZeroUsize {
        self.maximum_requests_per_caller
    }

    /// Return the per-workspace in-flight request ceiling.
    #[must_use]
    pub const fn maximum_requests_per_workspace(self) -> NonZeroUsize {
        self.maximum_requests_per_workspace
    }

    /// Return the process-global in-flight request ceiling.
    #[must_use]
    pub const fn maximum_requests_global(self) -> NonZeroUsize {
        self.maximum_requests_global
    }

    /// Return the distinct-computation ceiling.
    #[must_use]
    pub const fn maximum_parallel_computations(self) -> NonZeroUsize {
        self.maximum_parallel_computations
    }
}

/// One independently enforced non-queuing admission dimension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResidentAdmissionScope {
    /// One resolved caller reached its explicit request ceiling.
    Caller,
    /// One workspace reached its explicit request ceiling.
    Workspace,
    /// The process-global request ceiling was reached.
    Global,
    /// Distinct computations reached the explicit CPU ceiling.
    Computation,
}

/// Complete nonempty set of admission dimensions saturated by one request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResidentAdmissionSaturation(BTreeSet<ResidentAdmissionScope>);

impl ResidentAdmissionSaturation {
    /// Iterate every saturated dimension in deterministic scope order.
    pub fn scopes(&self) -> impl ExactSizeIterator<Item = ResidentAdmissionScope> + '_ {
        self.0.iter().copied()
    }

    /// Report whether one exact dimension was saturated.
    #[must_use]
    pub fn contains(&self, scope: ResidentAdmissionScope) -> bool {
        self.0.contains(&scope)
    }
}

/// Process-local admission occupancy exposed for tests and load measurement.
///
/// This snapshot is observational only. It carries no caller identities,
/// scheduling order, durable state, or readiness claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResidentAdmissionSnapshot {
    active_requests: usize,
    active_callers: usize,
    active_workspaces: usize,
    active_computations: usize,
}

impl ResidentAdmissionSnapshot {
    /// Return all currently admitted requests, including single-flight joiners.
    #[must_use]
    pub const fn active_requests(self) -> usize {
        self.active_requests
    }

    /// Return the number of caller identities with admitted requests.
    #[must_use]
    pub const fn active_callers(self) -> usize {
        self.active_callers
    }

    /// Return the number of workspaces with admitted requests.
    #[must_use]
    pub const fn active_workspaces(self) -> usize {
        self.active_workspaces
    }

    /// Return the number of distinct computations currently admitted.
    #[must_use]
    pub const fn active_computations(self) -> usize {
        self.active_computations
    }
}

/// Whether this caller produced or joined one overlapping digest computation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResidentCompilationExecution {
    /// This caller ran the admitted computation.
    Produced,
    /// This caller consumed the result of an already-running computation.
    Joined,
}

/// One independently correlated view of a single-flight compilation result.
pub struct ResidentCompilationOutcome<T, E> {
    execution: ResidentCompilationExecution,
    result: Arc<Result<T, E>>,
}

impl<T, E> ResidentCompilationOutcome<T, E> {
    /// Report whether this request produced or joined the computation.
    #[must_use]
    pub const fn execution(&self) -> ResidentCompilationExecution {
        self.execution
    }

    /// Borrow the one shared immutable computation result.
    pub fn result(&self) -> &Result<T, E> {
        self.result.as_ref()
    }

    /// Clone the shared result handle without cloning either result branch.
    #[must_use]
    pub fn shared_result(&self) -> Arc<Result<T, E>> {
        Arc::clone(&self.result)
    }
}

/// Failure before one caller obtains a compilation result.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ResidentWorkCoordinationError {
    /// This caller's cancellation or deadline closed only its wait.
    #[error(
        "resident request `{}` was interrupted: {reason:?}",
        request_id.as_str()
    )]
    Interrupted {
        /// Caller-generated request identity.
        request_id: ResidentRequestIdV1,
        /// Complete interruption state observed at the boundary.
        reason: ResidentRequestInterruption,
    },
    /// One or more explicit non-queuing admission ceilings were reached.
    #[error(
        "resident request `{}` was rejected by resident admission limits",
        request_id.as_str()
    )]
    Saturated {
        /// Caller-generated request identity.
        request_id: ResidentRequestIdV1,
        /// Complete set of saturated dimensions.
        saturation: ResidentAdmissionSaturation,
    },
    /// A process-local coordination lock was poisoned.
    #[error("resident coordination lock `{region}` was poisoned")]
    SynchronizationPoisoned {
        /// Small stable name of the failed internal region.
        region: &'static str,
    },
}

enum FlightState<T, E> {
    /// The producer still owns the computation.
    Pending,
    /// The producer published one immutable shared result.
    Completed(Arc<Result<T, E>>),
    /// The producer ended without a result, allowing a joiner to retry.
    Abandoned,
}

/// Lost-wake-free coordination point shared by one compilation flight.
///
/// Waiters hold `gate` while checking cancellation and flight state, then
/// atomically release it through [`Condvar::wait_timeout`]. Cancellation
/// notifications and completion or abandonment state transitions hold that same
/// gate, so none can pass between the checks and the wait. When both flight locks
/// are needed, `gate` is acquired before [`Flight::state`]. Abandonment may then
/// remove the flight from the registry; re-election drops both flight locks before
/// entering coordination again.
#[derive(Default)]
struct FlightWake {
    gate: Mutex<()>,
    changed: Condvar,
}

impl FlightWake {
    /// Wake waiters after cancellation while preserving the check-to-park edge.
    fn notify_all(&self) {
        let _gate = self
            .gate
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.changed.notify_all();
    }
}

/// Shared state for one exact digest while its computation overlaps callers.
struct Flight<T, E> {
    /// Current producer/result state.
    state: Mutex<FlightState<T, E>>,
    /// Wake source for result publication, abandonment, and cancellation.
    wake: Arc<FlightWake>,
}

impl<T, E> Flight<T, E> {
    fn pending() -> Self {
        Self {
            state: Mutex::new(FlightState::Pending),
            wake: Arc::new(FlightWake::default()),
        }
    }
}

/// Mutable occupancy behind one atomic non-queuing admission decision.
#[derive(Default)]
struct AdmissionState {
    /// Process-global in-flight request count.
    requests: usize,
    /// In-flight request counts by resolved caller identity.
    callers: BTreeMap<RequestCallerIdV1, usize>,
    /// In-flight request counts by workspace identity.
    workspaces: BTreeMap<ResidentWorkspaceIdV1, usize>,
    /// Distinct computations currently consuming CPU admission.
    computations: usize,
}

/// Non-queuing owner of caller, workspace, global, and computation limits.
struct AdmissionLane {
    /// Exact host-supplied limits.
    limits: ResidentAdmissionLimits,
    /// Atomically observed and advanced occupancy.
    state: Mutex<AdmissionState>,
}

impl AdmissionLane {
    fn admit_request(
        &self,
        control: &ResidentRequestControl,
    ) -> Result<RequestPermit<'_>, ResidentWorkCoordinationError> {
        let mut state = lock(&self.state, "resident admission state")?;
        let mut saturated = BTreeSet::new();
        if state.requests >= self.limits.maximum_requests_global.get() {
            saturated.insert(ResidentAdmissionScope::Global);
        }
        if state.callers.get(control.caller_id()).copied().unwrap_or(0)
            >= self.limits.maximum_requests_per_caller.get()
        {
            saturated.insert(ResidentAdmissionScope::Caller);
        }
        if state
            .workspaces
            .get(control.workspace_id())
            .copied()
            .unwrap_or(0)
            >= self.limits.maximum_requests_per_workspace.get()
        {
            saturated.insert(ResidentAdmissionScope::Workspace);
        }
        if !saturated.is_empty() {
            return Err(ResidentWorkCoordinationError::Saturated {
                request_id: control.request_id.clone(),
                saturation: ResidentAdmissionSaturation(saturated),
            });
        }
        state.requests += 1;
        *state.callers.entry(control.caller_id.clone()).or_default() += 1;
        *state
            .workspaces
            .entry(control.workspace_id.clone())
            .or_default() += 1;
        Ok(RequestPermit {
            lane: self,
            caller_id: control.caller_id.clone(),
            workspace_id: control.workspace_id.clone(),
        })
    }

    fn admit_computation(
        &self,
        request_id: &ResidentRequestIdV1,
    ) -> Result<ComputationPermit<'_>, ResidentWorkCoordinationError> {
        let mut state = lock(&self.state, "resident admission state")?;
        if state.computations >= self.limits.maximum_parallel_computations.get() {
            return Err(ResidentWorkCoordinationError::Saturated {
                request_id: request_id.clone(),
                saturation: ResidentAdmissionSaturation(BTreeSet::from([
                    ResidentAdmissionScope::Computation,
                ])),
            });
        }
        state.computations += 1;
        Ok(ComputationPermit { lane: self })
    }

    fn snapshot(&self) -> Result<ResidentAdmissionSnapshot, ResidentWorkCoordinationError> {
        let state = lock(&self.state, "resident admission state")?;
        Ok(ResidentAdmissionSnapshot {
            active_requests: state.requests,
            active_callers: state.callers.len(),
            active_workspaces: state.workspaces.len(),
            active_computations: state.computations,
        })
    }
}

/// Scoped proof that one caller/workspace request occupies all three limits.
struct RequestPermit<'a> {
    /// Admission lane released when this request leaves scope.
    lane: &'a AdmissionLane,
    /// Exact caller counter advanced by this permit.
    caller_id: RequestCallerIdV1,
    /// Exact workspace counter advanced by this permit.
    workspace_id: ResidentWorkspaceIdV1,
}

impl Drop for RequestPermit<'_> {
    fn drop(&mut self) {
        let mut state = self
            .lane
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.requests = state
            .requests
            .checked_sub(1)
            .expect("a live request permit owns one global admission count");
        decrement_and_remove(&mut state.callers, &self.caller_id);
        decrement_and_remove(&mut state.workspaces, &self.workspace_id);
    }
}

/// Scoped proof that one distinct computation occupies the CPU limit.
struct ComputationPermit<'a> {
    /// Admission lane released when computation ends.
    lane: &'a AdmissionLane,
}

impl Drop for ComputationPermit<'_> {
    fn drop(&mut self) {
        let mut state = self
            .lane
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.computations = state
            .computations
            .checked_sub(1)
            .expect("a live computation permit owns one admission count");
    }
}

/// Process-local single-flight owner for exact compilation-input digests.
///
/// [`ResidentAdmissionLimits`] is an explicit host input, not an application default.
/// The coordinator rejects work when any owning ceiling is occupied; it does
/// not silently create a queue or select a fairness order. A caller that finds
/// an existing matching digest consumes caller/workspace/global request
/// admission but no additional computation slot.
pub struct ResidentCompilationCoordinator<T, E> {
    admission: AdmissionLane,
    flights: Mutex<BTreeMap<CompilationInputDigest, Arc<Flight<T, E>>>>,
}

impl<T, E> ResidentCompilationCoordinator<T, E> {
    /// Construct one coordinator from four explicit positive ceilings.
    #[must_use]
    pub fn new(limits: ResidentAdmissionLimits) -> Self {
        Self {
            admission: AdmissionLane {
                limits,
                state: Mutex::new(AdmissionState::default()),
            },
            flights: Mutex::new(BTreeMap::new()),
        }
    }

    /// Return the exact four configured admission ceilings.
    #[must_use]
    pub const fn limits(&self) -> ResidentAdmissionLimits {
        self.admission.limits
    }

    /// Snapshot current process-local occupancy for load measurement.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentWorkCoordinationError::SynchronizationPoisoned`] when
    /// process-local admission state can no longer be trusted.
    pub fn admission_snapshot(
        &self,
    ) -> Result<ResidentAdmissionSnapshot, ResidentWorkCoordinationError> {
        self.admission.snapshot()
    }

    /// Produce or join one overlapping exact-digest computation.
    ///
    /// The computation runs synchronously on the producer's thread. Its result
    /// is shared only with callers that overlap the flight; no completed-result
    /// cache is retained. A producer panic abandons and wakes the flight before
    /// the original panic resumes, so joiners can retry rather than wait forever.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentWorkCoordinationError::Interrupted`] for this caller's
    /// cancellation/deadline, [`ResidentWorkCoordinationError::Saturated`]
    /// when an owning request or computation ceiling is reached, or
    /// [`ResidentWorkCoordinationError::SynchronizationPoisoned`] when internal
    /// process-local synchronization can no longer be trusted.
    pub fn run<F>(
        &self,
        digest: CompilationInputDigest,
        control: &ResidentRequestControl,
        compute: F,
    ) -> Result<ResidentCompilationOutcome<T, E>, ResidentWorkCoordinationError>
    where
        F: FnOnce() -> Result<T, E>,
    {
        self.require_active(control)?;
        let _request_permit = self.admission.admit_request(control)?;
        self.run_admitted(digest, control, compute)
    }

    /// Run work after the caller/workspace/global request admission is held.
    fn run_admitted<F>(
        &self,
        digest: CompilationInputDigest,
        control: &ResidentRequestControl,
        compute: F,
    ) -> Result<ResidentCompilationOutcome<T, E>, ResidentWorkCoordinationError>
    where
        F: FnOnce() -> Result<T, E>,
    {
        self.require_active(control)?;
        let (flight, producer) = {
            let mut flights = lock(&self.flights, "compilation flight registry")?;
            if let Some(flight) = flights.get(&digest) {
                (Arc::clone(flight), false)
            } else {
                let flight = Arc::new(Flight::pending());
                flights.insert(digest.clone(), Arc::clone(&flight));
                (flight, true)
            }
        };

        if !producer {
            return self.join(digest, flight, control, compute);
        }

        let permit = match self.admission.admit_computation(control.request_id()) {
            Ok(permit) => permit,
            Err(error) => {
                self.abandon(&digest, &flight)?;
                return Err(error);
            }
        };
        if let Err(error) = self.require_active(control) {
            drop(permit);
            self.abandon(&digest, &flight)?;
            return Err(error);
        }

        let computed = match catch_unwind(AssertUnwindSafe(compute)) {
            Ok(computed) => Arc::new(computed),
            Err(payload) => {
                drop(permit);
                let _ = self.abandon(&digest, &flight);
                resume_unwind(payload);
            }
        };
        drop(permit);
        {
            let _wake = lock(&flight.wake.gate, "compilation flight wake gate")?;
            let mut state = lock(&flight.state, "compilation flight state")?;
            *state = FlightState::Completed(Arc::clone(&computed));
            drop(state);
            flight.wake.changed.notify_all();
        }
        self.remove(&digest, &flight)?;
        self.require_active(control)?;
        Ok(ResidentCompilationOutcome {
            execution: ResidentCompilationExecution::Produced,
            result: computed,
        })
    }

    fn join<F>(
        &self,
        digest: CompilationInputDigest,
        flight: Arc<Flight<T, E>>,
        control: &ResidentRequestControl,
        compute: F,
    ) -> Result<ResidentCompilationOutcome<T, E>, ResidentWorkCoordinationError>
    where
        F: FnOnce() -> Result<T, E>,
    {
        control.cancellation.observe(&flight.wake)?;
        let mut compute = Some(compute);
        let mut wake = lock(&flight.wake.gate, "compilation flight wake gate")?;
        loop {
            self.require_active(control)?;
            let state = lock(&flight.state, "compilation flight state")?;
            match &*state {
                FlightState::Completed(result) => {
                    return Ok(ResidentCompilationOutcome {
                        execution: ResidentCompilationExecution::Joined,
                        result: Arc::clone(result),
                    });
                }
                FlightState::Abandoned => {
                    drop(state);
                    drop(wake);
                    return self.run_admitted(
                        digest,
                        control,
                        compute.take().expect("join retry owns its computation"),
                    );
                }
                FlightState::Pending => {
                    drop(state);
                    let remaining = control.deadline.saturating_duration_since(Instant::now());
                    let (next, _) =
                        flight
                            .wake
                            .changed
                            .wait_timeout(wake, remaining)
                            .map_err(|_| {
                                ResidentWorkCoordinationError::SynchronizationPoisoned {
                                    region: "compilation flight wake gate",
                                }
                            })?;
                    wake = next;
                }
            }
        }
    }

    fn require_active(
        &self,
        control: &ResidentRequestControl,
    ) -> Result<(), ResidentWorkCoordinationError> {
        control.interruption().map_or(Ok(()), |reason| {
            Err(ResidentWorkCoordinationError::Interrupted {
                request_id: control.request_id.clone(),
                reason,
            })
        })
    }

    fn abandon(
        &self,
        digest: &CompilationInputDigest,
        flight: &Arc<Flight<T, E>>,
    ) -> Result<(), ResidentWorkCoordinationError> {
        let _wake = lock(&flight.wake.gate, "compilation flight wake gate")?;
        let mut state = lock(&flight.state, "compilation flight state")?;
        *state = FlightState::Abandoned;
        // Keep the abandoned state unavailable to observers until this exact
        // flight is no longer discoverable. Otherwise a newly arriving caller
        // can reacquire the abandoned entry and recursively retry it before
        // removal wins the race.
        let removal = self.remove(digest, flight);
        drop(state);
        flight.wake.changed.notify_all();
        removal
    }

    fn remove(
        &self,
        digest: &CompilationInputDigest,
        flight: &Arc<Flight<T, E>>,
    ) -> Result<(), ResidentWorkCoordinationError> {
        let mut flights = lock(&self.flights, "compilation flight registry")?;
        if flights
            .get(digest)
            .is_some_and(|current| Arc::ptr_eq(current, flight))
        {
            flights.remove(digest);
        }
        Ok(())
    }
}

fn lock<'a, T>(
    mutex: &'a Mutex<T>,
    region: &'static str,
) -> Result<MutexGuard<'a, T>, ResidentWorkCoordinationError> {
    mutex
        .lock()
        .map_err(|_| ResidentWorkCoordinationError::SynchronizationPoisoned { region })
}

/// Decrement one positive keyed counter and remove its zero entry.
fn decrement_and_remove<K: Ord>(counts: &mut BTreeMap<K, usize>, key: &K) {
    let count = counts
        .get_mut(key)
        .expect("a live request permit owns each keyed admission count");
    *count = count
        .checked_sub(1)
        .expect("a keyed admission count is positive while its permit is live");
    let remove = *count == 0;
    if remove {
        counts.remove(key);
    }
}

#[cfg(test)]
#[path = "../../tests/internal/resident/concurrency.rs"]
mod tests;
