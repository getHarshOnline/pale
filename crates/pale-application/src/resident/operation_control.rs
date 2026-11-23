//! Current-authority admission for same-handle operation observation and cancellation.

use super::{RecoveredLocalOperation, RecoveredOperationObservation};
use pale_core::records::*;

mod recovery;
pub(in crate::resident) use recovery::{
    recover_child_operation_control, ChildOperationControlRecoveryInput,
};
pub use recovery::{
    recover_operation_control, OperationControlRecoveryAdmission, OperationControlRecoveryError,
    OperationControlRecoveryInput, RecoveredOperationControl,
};
mod cancellation;
pub(in crate::resident) use cancellation::OwnedLocalOperationCancellationCompletion;
pub use cancellation::{
    DispatchedOperationCancellationRequestInput, LocalOperationCancellationCompletionInput,
    LocalOperationCancellationRequestInput, OperationCancellationCompletionError,
    OperationCancellationRecoveryError, OperationCancellationRequestError,
    PreparedDispatchedOperationCancellationRequest, PreparedLocalOperationCancellationCompletion,
    PreparedLocalOperationCancellationRequest, RecoveredLocalOperationCancellationRequest,
};

/// Capability identity for a read-only current-head observation request.
pub const OPERATION_OBSERVE_CAPABILITY_ID: &str = "operation.observe";
/// Capability identity for a durable cancellation request.
pub const OPERATION_CANCEL_CAPABILITY_ID: &str = "operation.cancel";

/// Current facts presented to the owner of inspection or cancellation authority.
///
/// Protected recovery establishes history. This context asks a separate owner
/// whether the present caller may learn or act on that history now.
pub struct OperationControlAuthorizationContext<'a> {
    /// Fresh caller-signed control request.
    pub request: &'a CommonRequestEnvelopeV1,
    /// Original admitted request that started the operation.
    pub original_request: &'a CommonRequestEnvelopeV1,
    /// Exact durable operation locator.
    pub operation: (&'a ResidentRecordRefV1, &'a OperationHandleV1),
    /// Current anchored observation head.
    pub observation: (&'a ResidentRecordRefV1, &'a OperationObservationV1),
    /// Time at which current authority is evaluated.
    pub authorized_at: UnixTimeV1,
}

/// Inputs already recovered through their existing storage and signature owners.
pub struct OperationControlInput<'a> {
    /// Fresh, contextually admitted request for this control operation.
    pub request: &'a CommonRequestEnvelopeV1,
    /// Protected recovery of the exact operation handle and original request.
    pub operation: &'a RecoveredLocalOperation,
    /// Bounded recovery of the current anchored observation head.
    pub observation: &'a RecoveredOperationObservation,
    /// Time at which current authority is evaluated.
    pub authorized_at: UnixTimeV1,
}

/// Current read authority over one exact operation observation.
#[must_use = "project the admitted observation without reclassifying its state"]
pub struct AuthorizedOperationObservation<'a> {
    request: &'a CommonRequestEnvelopeV1,
    operation: (&'a ResidentRecordRefV1, &'a OperationHandleV1),
    observation: (&'a ResidentRecordRefV1, &'a OperationObservationV1),
}

/// Owned result of one currently authorized operation observation.
///
/// The two signed public records retain their own semantic identities. Their
/// storage references remain separate because neither a record digest nor an
/// operation identity identifies physical custody. This result contains no
/// store handle, recovery callback, caller credential, or renewed authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationControlObservation {
    operation_ref: ResidentRecordRefV1,
    operation: OperationHandleV1,
    observation_ref: ResidentRecordRefV1,
    observation: OperationObservationV1,
}

impl OperationControlObservation {
    /// Borrow the immutable operation-handle record and its storage reference.
    #[must_use]
    pub const fn operation(&self) -> (&ResidentRecordRefV1, &OperationHandleV1) {
        (&self.operation_ref, &self.operation)
    }

    /// Borrow the current signed observation and its storage reference.
    #[must_use]
    pub const fn observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.observation_ref, &self.observation)
    }
}

impl AuthorizedOperationObservation<'_> {
    /// Borrow the fresh request that established current inspection authority.
    #[must_use]
    pub const fn request(&self) -> &CommonRequestEnvelopeV1 {
        self.request
    }

    /// Borrow the exact immutable operation locator.
    #[must_use]
    pub const fn operation(&self) -> (&ResidentRecordRefV1, &OperationHandleV1) {
        self.operation
    }

    /// Borrow the exact current anchored observation.
    #[must_use]
    pub const fn observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        self.observation
    }

    /// Copy the authorized public records into an owned carrier-safe result.
    ///
    /// The projection does not repeat authorization, read storage, advance the
    /// operation, or imply that the observation's referenced result or effect
    /// evidence has been fetched.
    #[must_use]
    pub fn to_owned_observation(&self) -> OperationControlObservation {
        OperationControlObservation {
            operation_ref: self.operation.0.clone(),
            operation: self.operation.1.clone(),
            observation_ref: self.observation.0.clone(),
            observation: self.observation.1.clone(),
        }
    }
}

/// Current cancellation authority over one exact operation observation.
#[must_use = "route cancellation from the admitted current state without replay"]
pub struct AuthorizedOperationCancellation<'a> {
    request: &'a CommonRequestEnvelopeV1,
    recovered_operation: &'a RecoveredLocalOperation,
    recovered_observation: &'a RecoveredOperationObservation,
    operation: (&'a ResidentRecordRefV1, &'a OperationHandleV1),
    observation: (&'a ResidentRecordRefV1, &'a OperationObservationV1),
    authorized_at: UnixTimeV1,
}

impl<'a> AuthorizedOperationCancellation<'a> {
    /// Borrow the fresh request that established current cancellation authority.
    #[must_use]
    pub const fn request(&self) -> &'a CommonRequestEnvelopeV1 {
        self.request
    }

    /// Borrow the exact immutable operation locator.
    #[must_use]
    pub const fn operation(&self) -> (&'a ResidentRecordRefV1, &'a OperationHandleV1) {
        self.operation
    }

    /// Borrow the protected operation recovered before current cancellation admission.
    pub(crate) const fn recovered_operation(&self) -> &'a RecoveredLocalOperation {
        self.recovered_operation
    }

    /// Borrow the protected current observation recovered before cancellation admission.
    pub(crate) const fn recovered_observation(&self) -> &'a RecoveredOperationObservation {
        self.recovered_observation
    }

    /// Return the admitted time that fences this cancellation decision.
    pub(crate) const fn authorized_at(&self) -> UnixTimeV1 {
        self.authorized_at
    }

    /// Classify the current state without sending, signing, writing, or retrying.
    #[must_use]
    pub fn disposition(&self) -> OperationCancellationDisposition<'_> {
        cancellation_disposition(self.operation.1, self.observation, self.authorized_at)
    }
}

fn cancellation_disposition<'a>(
    operation: &OperationHandleV1,
    observation: (&'a ResidentRecordRefV1, &'a OperationObservationV1),
    authorized_at: UnixTimeV1,
) -> OperationCancellationDisposition<'a> {
    if authorized_at > *operation.expires_at() {
        return OperationCancellationDisposition::ReconciliationOnly(observation);
    }
    match observation.1.state() {
        OperationObservationStateV1::Active { phase } => match phase {
            OperationActivePhaseV1::Resolving
            | OperationActivePhaseV1::ContextBound { .. }
            | OperationActivePhaseV1::ProviderResolved { .. }
            | OperationActivePhaseV1::DispatchPrepared { .. } => {
                OperationCancellationDisposition::BeforeDispatch
            }
            OperationActivePhaseV1::Dispatched {
                attempt_id,
                evidence,
            } => OperationCancellationDisposition::Dispatched {
                attempt_id,
                evidence,
            },
            OperationActivePhaseV1::CollectingConsultation { evidence } => {
                OperationCancellationDisposition::CollectingConsultation { evidence }
            }
            OperationActivePhaseV1::ConsultationCollected { evidence } => {
                OperationCancellationDisposition::ConsultationCollected { evidence }
            }
        },
        OperationObservationStateV1::CancellationRequested { request } => {
            OperationCancellationDisposition::AlreadyRequested { request }
        }
        OperationObservationStateV1::Succeeded { .. }
        | OperationObservationStateV1::Failed { .. }
        | OperationObservationStateV1::Cancelled { .. } => {
            OperationCancellationDisposition::Established(observation)
        }
        OperationObservationStateV1::Recovering { .. }
        | OperationObservationStateV1::OutcomeUnknown { .. }
        | OperationObservationStateV1::Expired { .. } => {
            OperationCancellationDisposition::ReconciliationOnly(observation)
        }
    }
}

/// Effect-free cancellation routing from the current admitted observation.
pub enum OperationCancellationDisposition<'a> {
    /// A terminal state already exists; cancellation performs no action.
    Established((&'a ResidentRecordRefV1, &'a OperationObservationV1)),
    /// The request is durable already; another Cancel call must not replay it.
    AlreadyRequested {
        /// Exact immutable cancellation-request evidence.
        request: &'a ImmutableRecordReferenceV1,
    },
    /// The operation has not crossed dispatch and may use local cancellation custody.
    BeforeDispatch,
    /// One provider attempt crossed dispatch and requires owner-specific cancellation.
    Dispatched {
        /// Exact dispatched attempt.
        attempt_id: &'a CapabilityAttemptIdV1,
        /// Existing private selection evidence needed by the application router.
        evidence: &'a ImmutableRecordReferenceV1,
    },
    /// Consultation cancellation remains owned by its durable roster and join.
    CollectingConsultation {
        /// Exact immutable evidence for the current collection frontier.
        evidence: &'a ImmutableRecordReferenceV1,
    },
    /// Collection is frozen; cancellation applies to remaining parent work.
    ConsultationCollected {
        /// Exact immutable evidence for the stopped collection frontier.
        evidence: &'a ImmutableRecordReferenceV1,
    },
    /// Recovery, uncertainty, or expiry must be reconciled without a new effect.
    ReconciliationOnly((&'a ResidentRecordRefV1, &'a OperationObservationV1)),
}

/// Failure before current operation-control authority is established.
#[derive(Debug, thiserror::Error)]
pub enum OperationControlError {
    /// The signed request body or another record-owned invariant failed.
    #[error(transparent)]
    Admission(#[from] InvalidResidentRecordV1),
    /// The request capability, operation, store, generation, fence, or head differs.
    #[error("operation control inputs do not identify one current operation")]
    Association,
    /// The current request is not valid at the supplied authorization time.
    #[error("operation control request is outside its admitted time interval")]
    Time,
}

/// Check the admitted fresh request interval without reading operation history.
///
/// This is the only time check available before protected recovery. Operation
/// creation, observation, and retention times remain owned by the complete
/// post-recovery control check.
pub(in crate::resident) fn validate_fresh_operation_control_request_time(
    request: &CommonRequestEnvelopeV1,
    authorized_at: UnixTimeV1,
) -> Result<(), OperationControlError> {
    if authorized_at < request.issued_at() || authorized_at > request.expires_at() {
        Err(OperationControlError::Time)
    } else {
        Ok(())
    }
}

/// Complete input for observing one retained local operation.
pub struct LocalOperationObservationInput<'a> {
    /// Saved handle and bounded protected-recovery inputs.
    pub recovery: OperationControlRecoveryInput<'a>,
    /// Fresh caller-signed request with the operation handle as its body.
    pub request: &'a CommonRequestEnvelopeV1,
    /// Time at which current inspection authority is evaluated.
    pub authorized_at: UnixTimeV1,
}

/// Existing recovery and present-authority owners used by local observation.
pub struct LocalOperationObservationAdmission<'a> {
    /// Historical request, handle, checkpoint, observation, and signature admission.
    pub recovery: OperationControlRecoveryAdmission<'a>,
    /// Present disclosure decision for the exact recovered operation and head.
    pub authorize: &'a mut dyn FnMut(
        OperationControlAuthorizationContext<'_>,
    ) -> Result<(), InvalidResidentRecordV1>,
}

/// Recover and observe one retained local operation without changing it.
///
/// This is the application-owned `Observe` use case. Protected recovery first
/// proves the exact handle, current observation, checkpoint membership, and
/// independent anchor. The fresh signed request is then joined to that history
/// and passed to the present disclosure owner. Only the two signed public
/// records and their immutable references leave the use case.
///
/// # Errors
///
/// Returns the exact protected-recovery or current-authorization failure. No
/// write, provider call, retry, cancellation, authority renewal, or evidence
/// dereference occurs.
pub fn observe_local_operation(
    input: LocalOperationObservationInput<'_>,
    operation_port: &mut dyn pale_ports::OperationCustodyPort,
    session_port: &dyn pale_ports::SessionLeaseCustodyPort,
    anchor_port: &mut dyn pale_ports::AntiRollbackAnchorPort,
    admission: LocalOperationObservationAdmission<'_>,
) -> Result<OperationControlObservation, LocalOperationObservationError> {
    let recovered = recover_operation_control(
        input.recovery,
        operation_port,
        session_port,
        anchor_port,
        admission.recovery,
    )?;
    let authorized =
        recovered.authorize_observation(input.request, input.authorized_at, |context| {
            (admission.authorize)(context)
        })?;
    Ok(authorized.to_owned_observation())
}

/// Failure before an owned local operation observation exists.
#[derive(Debug, thiserror::Error)]
pub enum LocalOperationObservationError {
    /// Protected operation or observation recovery failed.
    #[error(transparent)]
    Recovery(#[from] OperationControlRecoveryError),
    /// Present inspection authority was rejected.
    #[error(transparent)]
    Authorization(#[from] OperationControlError),
}

/// Establish current read authority for the exact anchored observation head.
///
/// # Errors
///
/// Returns an error for an unlike capability, handle body, store, generation,
/// activation fence, workspace, observation head, request time, or authority
/// decision. No write or provider call occurs.
pub fn authorize_operation_observation<'a, Authorize>(
    input: OperationControlInput<'a>,
    authorize: Authorize,
) -> Result<AuthorizedOperationObservation<'a>, OperationControlError>
where
    Authorize:
        FnOnce(OperationControlAuthorizationContext<'_>) -> Result<(), InvalidResidentRecordV1>,
{
    let facts = authorize_control(input, OPERATION_OBSERVE_CAPABILITY_ID, authorize)?;
    Ok(AuthorizedOperationObservation {
        request: facts.request,
        operation: facts.operation,
        observation: facts.observation,
    })
}

/// Establish current cancellation authority without performing cancellation.
///
/// # Errors
///
/// Returns an error for an unlike capability, handle body, store, generation,
/// activation fence, workspace, observation head, request time, or authority
/// decision. No write or provider call occurs.
pub fn authorize_operation_cancellation<'a, Authorize>(
    input: OperationControlInput<'a>,
    authorize: Authorize,
) -> Result<AuthorizedOperationCancellation<'a>, OperationControlError>
where
    Authorize:
        FnOnce(OperationControlAuthorizationContext<'_>) -> Result<(), InvalidResidentRecordV1>,
{
    let recovered_operation = input.operation;
    let recovered_observation = input.observation;
    let facts = authorize_control(input, OPERATION_CANCEL_CAPABILITY_ID, authorize)?;
    Ok(AuthorizedOperationCancellation {
        request: facts.request,
        recovered_operation,
        recovered_observation,
        operation: facts.operation,
        observation: facts.observation,
        authorized_at: facts.authorized_at,
    })
}

fn authorize_control<'a, Authorize>(
    input: OperationControlInput<'a>,
    expected_capability: &str,
    authorize: Authorize,
) -> Result<OperationControlAuthorizationContext<'a>, OperationControlError>
where
    Authorize:
        FnOnce(OperationControlAuthorizationContext<'_>) -> Result<(), InvalidResidentRecordV1>,
{
    let operation = input.operation.operation().handle();
    let observation = input.observation.selected();
    let facts = validate_control_facts(
        OperationControlFactsInput {
            request: input.request,
            original_request: input.operation.request(),
            operation_store: input.operation.store(),
            observation_store: input.observation.store(),
            operation,
            observation,
            observation_head: input.observation.selected_head(),
            authorized_at: input.authorized_at,
        },
        expected_capability,
    )?;
    authorize(OperationControlAuthorizationContext {
        request: facts.request,
        original_request: facts.original_request,
        operation: facts.operation,
        observation: facts.observation,
        authorized_at: facts.authorized_at,
    })?;
    Ok(facts)
}

struct OperationControlFactsInput<'a> {
    request: &'a CommonRequestEnvelopeV1,
    original_request: &'a CommonRequestEnvelopeV1,
    operation_store: &'a pale_ports::AuthorityStoreCoordinate,
    observation_store: &'a pale_ports::AuthorityStoreCoordinate,
    operation: (&'a ResidentRecordRefV1, &'a OperationHandleV1),
    observation: (&'a ResidentRecordRefV1, &'a OperationObservationV1),
    observation_head: (&'a ResidentRecordRefV1, &'a OperationObservationV1),
    authorized_at: UnixTimeV1,
}

fn validate_control_facts<'a>(
    input: OperationControlFactsInput<'a>,
    expected_capability: &str,
) -> Result<OperationControlAuthorizationContext<'a>, OperationControlError> {
    input
        .request
        .verify_operation_handle_body(input.operation.1)?;
    if input.request.capability_id().as_str() != expected_capability
        || input.operation_store != input.observation_store
        || input.request.workspace_id() != input.original_request.workspace_id()
        || input.request.generation() != Some(input.operation.1.compiled_generation_sha256())
        || input.request.fence_token() != Some(input.operation.1.activation_fence_sha256())
        || input.observation != input.observation_head
        || input.observation.1.operation_handle_id() != input.operation.1.operation_handle_id()
        || input.observation.1.operation_handle_ref() != input.operation.0
        || input.observation.1.operation_handle_sha256()
            != input.operation.1.canonical_operation_handle_sha256()
    {
        return Err(OperationControlError::Association);
    }
    validate_fresh_operation_control_request_time(input.request, input.authorized_at)?;
    if input.request.issued_at() < *input.operation.1.created_at()
        || input.authorized_at < *input.operation.1.created_at()
        || input.authorized_at < *input.observation_head.1.observed_at()
        || input.authorized_at > *input.operation.1.retain_until()
    {
        return Err(OperationControlError::Time);
    }
    Ok(OperationControlAuthorizationContext {
        request: input.request,
        original_request: input.original_request,
        operation: input.operation,
        observation: input.observation,
        authorized_at: input.authorized_at,
    })
}

/// Validate one signed Cancel request against already protected operation history.
///
/// Carrier-specific owners use this join after they recover their own retained
/// history. It establishes the same handle, current-head, time, generation, and
/// fence facts as [`authorize_operation_cancellation`] without granting a
/// carrier call or replacing the carrier's additional authorization policy.
pub(in crate::resident) fn validate_recovered_operation_cancellation<'a>(
    request: &'a CommonRequestEnvelopeV1,
    original_request: &'a CommonRequestEnvelopeV1,
    operation: &'a crate::resident::IssuedLocalOperation,
    observation: &'a RecoveredOperationObservation,
    authorized_at: UnixTimeV1,
) -> Result<OperationControlAuthorizationContext<'a>, OperationControlError> {
    if !operation
        .checkpoint_selection()
        .matches_store(observation.store())
    {
        return Err(OperationControlError::Association);
    }
    let head = observation.selected_head();
    validate_control_facts(
        OperationControlFactsInput {
            request,
            original_request,
            operation_store: observation.store(),
            observation_store: observation.store(),
            operation: operation.handle(),
            observation: head,
            observation_head: head,
            authorized_at,
        },
        OPERATION_CANCEL_CAPABILITY_ID,
    )
}

#[cfg(test)]
#[path = "../../tests/internal/resident/operation_control.rs"]
mod tests;
