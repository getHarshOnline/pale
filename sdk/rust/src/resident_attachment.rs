//! Send requests to a resident through connections supplied by the caller.

use std::error::Error;
use std::fmt;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use pale_ports::ResidentOperationFrameRecords;

/// Maximum canonical bytes accepted for one signed request or returned record.
///
/// Interfaces can use this value to bound input before they construct an SDK
/// request. The SDK repeats the check at its own public boundary.
pub const RESIDENT_RECORD_BYTE_LIMIT: usize = 65_536;

/// Connection handle for a resident whose authentication the caller supplies.
///
/// The trusted calling application supplies the connection configuration or
/// factory. Operation and observation attachments also require independent
/// checks for signed records; a hook-only attachment exposes neither. This
/// handle discovers no endpoint, opens no resident store
/// and retries no request. A failed request can close its connection; a later,
/// separately issued request may ask the factory for one fresh connection.
pub struct ResidentAttachment {
    pub(crate) client: Option<Arc<dyn crate::resident_observation::ResidentObservationClient>>,
    pub(crate) operation_client:
        Option<Arc<dyn crate::resident_operation::ResidentOperationClient>>,
    pub(crate) hook_client: Option<Arc<FramedResidentClient>>,
}

/// Exchange a request on a connection the caller asserts is authenticated.
///
/// The SDK cannot verify that assertion. The implementation authenticates the
/// peer, frames messages and manages the connection. It must attempt each
/// supplied request at most once.
pub trait AuthenticatedResidentCarrier: Send {
    /// Send one canonical request frame and receive one response within its size limit.
    ///
    /// `deadline` is one absolute monotonic time limit for the whole request:
    /// waiting in the queue, opening the connection, sending, receiving and
    /// checking the response all share it.
    ///
    /// # Errors
    ///
    /// Reports whether the request was certainly not sent or may have reached
    /// the resident. An uncertain outcome must not be treated as a safe retry.
    fn exchange(
        &mut self,
        request: &[u8],
        deadline: Instant,
    ) -> Result<Box<[u8]>, AuthenticatedResidentCarrierFailure>;
}

/// Open authenticated resident connections for separately issued SDK requests.
///
/// The implementation chooses the endpoint, connects and authenticates the peer.
/// Each call must return a ready connection or fail before sending any Pale
/// request. The SDK asks this factory at most once for each request.
pub trait AuthenticatedResidentCarrierSource: Send {
    /// Open one authenticated connection before the supplied deadline.
    ///
    /// # Errors
    ///
    /// Returns [`AuthenticatedResidentCarrierSourceFailure`] if a connection
    /// cannot be opened. This failure guarantees that the current request was not sent.
    fn open(
        &mut self,
        deadline: Instant,
    ) -> Result<Box<dyn AuthenticatedResidentCarrier>, AuthenticatedResidentCarrierSourceFailure>;
}

/// The connection factory could not open an authenticated connection.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("authenticated resident carrier is unavailable")]
pub struct AuthenticatedResidentCarrierSourceFailure;

/// Whether a failed exchange could have delivered the request to the resident.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum AuthenticatedResidentCarrierFailure {
    /// The connection established that no request byte was sent.
    #[error("resident request was not sent")]
    NotSent,
    /// The connection cannot establish whether the resident accepted the request.
    #[error("resident request outcome is unknown")]
    OutcomeUnknown,
    /// The connection was unusable before any request byte was sent.
    #[error("authenticated resident connection is unavailable")]
    ConnectionUnavailable,
}

/// Check the authority of signed resident responses before the SDK exposes them.
///
/// The caller's implementation must resolve the signer at the time of the records,
/// verify both signatures and check authority changes. Its rollback check must use
/// independently retained progress, not just the returned records. Recomputing a
/// digest alone cannot establish these guarantees.
pub trait ResidentResponseAdmission: Send + Sync {
    /// Admit and consume one native response against independently retained progress.
    /// Verify the signed dispatch and its authority, response commitment and
    /// exact request; reject an already consumed occurrence or uncertain replay.
    /// Returning success permits immediate emission of both response channels.
    ///
    /// # Errors
    /// The default refuses native release; ordinary record inspection is insufficient.
    fn admit_hook_response(
        &self,
        _operation_handle_record_ref: &str,
        _operation_handle_canonical_bytes: &[u8],
        _operation_observation_record_ref: &str,
        _operation_observation_canonical_bytes: &[u8],
        _response_canonical: &[u8],
        _deadline: Instant,
    ) -> Result<(), ResidentResponseAdmissionFailure> {
        Err(ResidentResponseAdmissionFailure)
    }

    /// Check one exact handle and observation pair before the SDK exposes it.
    ///
    /// `deadline` is the same absolute time limit supplied to the connection.
    /// Reject an expired call before advancing any state.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentResponseAdmissionFailure`] if authentication, authority
    /// changes or protection against rollback cannot be established.
    fn admit(
        &self,
        operation_handle_record_ref: &str,
        operation_handle_canonical_bytes: &[u8],
        operation_observation_record_ref: &str,
        operation_observation_canonical_bytes: &[u8],
        deadline: Instant,
    ) -> Result<(), ResidentResponseAdmissionFailure>;
}

/// A resident response failed its authority or rollback-protection checks.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("resident response authority admission failed")]
pub struct ResidentResponseAdmissionFailure;

/// The resident connection handle could not be configured.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ResidentAttachmentBuildError {
    /// A zero timeout leaves no time to exchange a request.
    #[error("resident call timeout must be positive")]
    ZeroTimeout,
    /// The relative timeout cannot be represented as a monotonic deadline.
    #[error("resident call timeout exceeds the monotonic clock range")]
    DeadlineOverflow,
}

impl fmt::Debug for ResidentAttachment {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ResidentAttachment(<sealed>)")
    }
}

/// Serialized client shared by Observe and operation requests.
pub(crate) struct FramedResidentClient {
    state: Mutex<FramedResidentState>,
    available: Condvar,
    call_timeout: Duration,
    response_admission: Option<Arc<dyn ResidentResponseAdmission>>,
}

/// One serialized carrier slot and its retained source.
enum FramedResidentState {
    /// A healthy carrier can serve the next request.
    Available {
        source: Box<dyn AuthenticatedResidentCarrierSource>,
        carrier: Box<dyn AuthenticatedResidentCarrier>,
    },
    /// No carrier is usable. The next request can open one fresh carrier.
    Closed {
        source: Box<dyn AuthenticatedResidentCarrierSource>,
    },
    /// One request owns the source and any current carrier.
    InUse,
}

impl FramedResidentClient {
    pub(crate) fn new(
        carrier_source: Box<dyn AuthenticatedResidentCarrierSource>,
        call_timeout: Duration,
        response_admission: Option<Arc<dyn ResidentResponseAdmission>>,
    ) -> Result<Self, ResidentAttachmentBuildError> {
        if call_timeout.is_zero() {
            return Err(ResidentAttachmentBuildError::ZeroTimeout);
        }
        if Instant::now().checked_add(call_timeout).is_none() {
            return Err(ResidentAttachmentBuildError::DeadlineOverflow);
        }
        Ok(Self {
            state: Mutex::new(FramedResidentState::Closed {
                source: carrier_source,
            }),
            available: Condvar::new(),
            call_timeout,
            response_admission,
        })
    }

    pub(crate) fn deadline(&self) -> Result<Instant, ResidentAttachmentClientError> {
        Instant::now()
            .checked_add(self.call_timeout)
            .ok_or_else(|| resident_not_sent_error("resident call deadline cannot be represented"))
    }

    pub(crate) fn exchange_with<T>(
        &self,
        frame: &[u8],
        deadline: Instant,
        admit_response: impl FnOnce(&[u8]) -> Result<T, ResidentAttachmentClientError>,
    ) -> Result<T, ResidentAttachmentClientError> {
        if frame.is_empty() || frame.len() > RESIDENT_RECORD_BYTE_LIMIT.saturating_mul(4) {
            return Err(resident_not_sent_error(
                "resident request frame exceeds its bound",
            ));
        }

        let mut guard = self
            .state
            .lock()
            .map_err(|_| resident_not_sent_error("resident client lock is poisoned"))?;
        let (mut source, existing_carrier) = loop {
            match &*guard {
                FramedResidentState::Available { .. } => {
                    let FramedResidentState::Available { source, carrier } =
                        std::mem::replace(&mut *guard, FramedResidentState::InUse)
                    else {
                        unreachable!("available state was matched")
                    };
                    break (source, Some(carrier));
                }
                FramedResidentState::Closed { .. } => {
                    let FramedResidentState::Closed { source } =
                        std::mem::replace(&mut *guard, FramedResidentState::InUse)
                    else {
                        unreachable!("closed state was matched")
                    };
                    break (source, None);
                }
                FramedResidentState::InUse => {
                    let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                        return Err(resident_not_sent_error(
                            "resident call expired in the queue",
                        ));
                    };
                    let (next, timeout) = self
                        .available
                        .wait_timeout(guard, remaining)
                        .map_err(|_| resident_not_sent_error("resident client lock is poisoned"))?;
                    guard = next;
                    if timeout.timed_out() {
                        return Err(resident_not_sent_error(
                            "resident call expired in the queue",
                        ));
                    }
                }
            }
        };
        drop(guard);

        let mut carrier = match existing_carrier {
            Some(carrier) => carrier,
            None => match source.open(deadline) {
                Ok(carrier) => carrier,
                Err(error) => {
                    self.restore_closed(source);
                    return Err(resident_source_error(error));
                }
            },
        };
        if Instant::now() >= deadline {
            drop(carrier);
            self.restore_closed(source);
            return Err(resident_not_sent_error(
                "resident call expired before transport",
            ));
        }

        let response = match carrier.exchange(frame, deadline) {
            Ok(response) => response,
            Err(error) => {
                drop(carrier);
                self.restore_closed(source);
                return Err(ResidentAttachmentClientError::carrier(error));
            }
        };
        if response.is_empty() || response.len() > RESIDENT_RECORD_BYTE_LIMIT.saturating_mul(4) {
            drop(carrier);
            self.restore_closed(source);
            return Err(resident_may_have_been_sent_error(
                "resident response frame exceeds its bound",
            ));
        }
        match admit_response(&response) {
            Ok(value) if Instant::now() < deadline => {
                self.restore_available(source, carrier);
                Ok(value)
            }
            Ok(_) => {
                drop(carrier);
                self.restore_closed(source);
                Err(resident_may_have_been_sent_error(
                    "resident response admission exceeded its deadline",
                ))
            }
            Err(error) => {
                drop(carrier);
                self.restore_closed(source);
                Err(error)
            }
        }
    }

    fn restore_available(
        &self,
        source: Box<dyn AuthenticatedResidentCarrierSource>,
        carrier: Box<dyn AuthenticatedResidentCarrier>,
    ) {
        if let Ok(mut guard) = self.state.lock() {
            *guard = FramedResidentState::Available { source, carrier };
            self.available.notify_one();
        }
    }

    fn restore_closed(&self, source: Box<dyn AuthenticatedResidentCarrierSource>) {
        if let Ok(mut guard) = self.state.lock() {
            *guard = FramedResidentState::Closed { source };
            self.available.notify_one();
        }
    }

    pub(crate) fn decode<T: serde::de::DeserializeOwned>(
        &self,
        response: &[u8],
    ) -> Result<T, ResidentAttachmentClientError> {
        let value = pale_core::canon::parse_strict(response)
            .map_err(|_| resident_may_have_been_sent_error("resident response is malformed"))?;
        let canonical = pale_core::canon::to_canonical_string(&value)
            .map_err(|_| resident_may_have_been_sent_error("resident response is not canonical"))?;
        if canonical.as_bytes() != response {
            return Err(resident_may_have_been_sent_error(
                "resident response is not exact canonical JSON",
            ));
        }
        serde_json::from_value(value).map_err(|_| {
            resident_may_have_been_sent_error("resident response has an invalid shape")
        })
    }

    pub(crate) fn admit_records(
        &self,
        records: &ResidentOperationFrameRecords,
        deadline: Instant,
    ) -> Result<(), ResidentAttachmentClientError> {
        self.response_admission()?
            .admit(
                records.operation_handle_record_ref(),
                records.operation_handle_canonical().as_bytes(),
                records.operation_observation_record_ref(),
                records.operation_observation_canonical().as_bytes(),
                deadline,
            )
            .map_err(response_admission_error)
    }

    pub(crate) fn response_admission(
        &self,
    ) -> Result<&dyn ResidentResponseAdmission, ResidentAttachmentClientError> {
        self.response_admission
            .as_deref()
            .ok_or_else(|| response_admission_error(ResidentResponseAdmissionFailure))
    }
}

impl ResidentAttachment {
    /// Configure hook delivery with an explicit independent response authority.
    /// This does not enable operation-start or observation requests.
    ///
    /// # Errors
    /// Refuses a zero timeout or a duration outside the monotonic clock range.
    pub fn provision_admitted_hooks(
        carrier_source: Box<dyn AuthenticatedResidentCarrierSource>,
        call_timeout: Duration,
        response_admission: Arc<dyn ResidentResponseAdmission>,
    ) -> Result<Self, ResidentAttachmentBuildError> {
        Ok(Self {
            client: None,
            operation_client: None,
            hook_client: Some(Arc::new(FramedResidentClient::new(
                carrier_source,
                call_timeout,
                Some(response_admission),
            )?)),
        })
    }

    /// Configure authenticated hook delivery without operation or observation access.
    ///
    /// Hook replies currently carry only blocking results. This attachment has
    /// no signed-record admission owner and cannot expose operation records.
    /// The supplied source must authenticate the resident before sending private input.
    ///
    /// # Errors
    /// Refuses a zero timeout or one that exceeds the monotonic clock range.
    pub fn provision_hooks(
        carrier_source: Box<dyn AuthenticatedResidentCarrierSource>,
        call_timeout: Duration,
    ) -> Result<Self, ResidentAttachmentBuildError> {
        Ok(Self {
            client: None,
            operation_client: None,
            hook_client: Some(Arc::new(FramedResidentClient::new(
                carrier_source,
                call_timeout,
                None,
            )?)),
        })
    }

    /// Whether this attachment can request independently admitted operation records.
    #[must_use]
    pub fn supports_observation_requests(&self) -> bool {
        self.client.is_some()
    }

    /// Configure a read-only handle using the caller's connection and response checks.
    ///
    /// The connection factory chooses the endpoint and authenticates the peer.
    /// The response checker verifies signatures, authority changes and protection
    /// against rollback. This function opens no connection and sends no request.
    /// Observe calls run one at a time; this handle cannot start or change operations.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentAttachmentBuildError::ZeroTimeout`] when `call_timeout`
    /// is zero and [`ResidentAttachmentBuildError::DeadlineOverflow`] when the
    /// timeout cannot be represented as a monotonic deadline.
    pub fn provision_observation(
        carrier_source: Box<dyn AuthenticatedResidentCarrierSource>,
        call_timeout: Duration,
        response_admission: Arc<dyn ResidentResponseAdmission>,
    ) -> Result<Self, ResidentAttachmentBuildError> {
        let client = Arc::new(FramedResidentClient::new(
            carrier_source,
            call_timeout,
            Some(response_admission),
        )?);
        Ok(Self {
            client: Some(client),
            operation_client: None,
            hook_client: None,
        })
    }

    /// Configure a handle for resident operations using the caller's connection and checks.
    ///
    /// The caller asserts that each supplied resident serves Observe, Invoke,
    /// Consult, Delegate, Cancel, and repository-hook frames. The SDK does not
    /// negotiate or infer those capabilities. Connection and response checks have the same requirements
    /// as [`Self::provision_observation`].
    ///
    /// # Errors
    ///
    /// Returns [`ResidentAttachmentBuildError::ZeroTimeout`] when `call_timeout`
    /// is zero and [`ResidentAttachmentBuildError::DeadlineOverflow`] when the
    /// timeout cannot be represented as a monotonic deadline.
    pub fn provision_operations(
        carrier_source: Box<dyn AuthenticatedResidentCarrierSource>,
        call_timeout: Duration,
        response_admission: Arc<dyn ResidentResponseAdmission>,
    ) -> Result<Self, ResidentAttachmentBuildError> {
        let client = Arc::new(FramedResidentClient::new(
            carrier_source,
            call_timeout,
            Some(response_admission),
        )?);
        Ok(Self {
            client: Some(client.clone()),
            operation_client: Some(client.clone()),
            hook_client: Some(client),
        })
    }
}

/// Opaque attachment failure retained behind the canonical API diagnostic.
#[derive(Debug, thiserror::Error)]
#[error("resident attachment failed")]
pub(crate) struct ResidentAttachmentClientError {
    #[source]
    pub(crate) source: Box<dyn Error + Send + Sync>,
    pub(crate) transmission: ResidentRequestTransmissionCertainty,
}

/// What the attachment proves about one request crossing its carrier boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ResidentRequestTransmissionCertainty {
    /// The request did not reach the carrier.
    NotSent,
    /// The carrier may have handed the request to the resident.
    MayHaveBeenSent,
}

impl ResidentAttachmentClientError {
    fn carrier(error: AuthenticatedResidentCarrierFailure) -> Self {
        let transmission = match error {
            AuthenticatedResidentCarrierFailure::NotSent
            | AuthenticatedResidentCarrierFailure::ConnectionUnavailable => {
                ResidentRequestTransmissionCertainty::NotSent
            }
            AuthenticatedResidentCarrierFailure::OutcomeUnknown => {
                ResidentRequestTransmissionCertainty::MayHaveBeenSent
            }
        };
        Self {
            transmission,
            source: Box::new(error),
        }
    }

    pub(crate) const fn transmission(&self) -> ResidentRequestTransmissionCertainty {
        self.transmission
    }
}

pub(crate) fn resident_not_sent_error(message: &'static str) -> ResidentAttachmentClientError {
    ResidentAttachmentClientError {
        source: std::io::Error::other(message).into(),
        transmission: ResidentRequestTransmissionCertainty::NotSent,
    }
}

pub(crate) fn resident_may_have_been_sent_error(
    message: &'static str,
) -> ResidentAttachmentClientError {
    ResidentAttachmentClientError {
        source: std::io::Error::other(message).into(),
        transmission: ResidentRequestTransmissionCertainty::MayHaveBeenSent,
    }
}

pub(crate) fn response_admission_error(
    error: ResidentResponseAdmissionFailure,
) -> ResidentAttachmentClientError {
    ResidentAttachmentClientError {
        source: Box::new(error),
        transmission: ResidentRequestTransmissionCertainty::MayHaveBeenSent,
    }
}

fn resident_source_error(
    error: AuthenticatedResidentCarrierSourceFailure,
) -> ResidentAttachmentClientError {
    ResidentAttachmentClientError {
        source: Box::new(error),
        transmission: ResidentRequestTransmissionCertainty::NotSent,
    }
}
