//! Contextual preparation and custody-gated exposure of local operations.
//!
//! Signed records must exist before the atomic transaction can retain them.
//! They are preparation data, not an issued application locator. Only the
//! exact completed database-and-anchor phase can close the pending locator.
//! Protected recovery instead re-admits the existing handle and proves its
//! immutable request association in the exactly anchored current root.
//! Native ingress also validates its selected body. External provider resolution,
//! dispatch and execution remain separate.

use pale_core::{canon, records::*};
use pale_ports::{
    AuthorityStoreCoordinate, ControlCheckpointHead, ControlRecord, ControlRecordError,
    OperationCustodyCandidate, OperationCustodyCandidateError, OperationCustodyIndexMutation,
    OperationCustodyRecord,
};

use super::event_custody::{LocalOperationPublicationPolicy, LocalOperationPublicationPolicyInput};
use super::generation::AdmittedCompiledGeneration;
use super::native_request::{prepare_native_request, NativeRequestError};
use super::operation_custody::{
    CompletedOperationStartCustody, OperationRecoveryError, OperationStartPreparationError,
};
use super::record_projection::{self, ControlRecordProjectionError};
use super::session_admission::{
    AdmittedAmbientSession, AmbientSessionAdmissionContext, AmbientSessionAdmissionError,
    AmbientSessionAuthorityPort, RetainedAmbientSession,
};
use crate::actions::{NativeActionInvocationError, NativeActionRegistry};
use crate::semantic::{
    obligation_disposition, CapabilityObligationDisposition, ObligationSubject, ResolvedOperation,
    ResolvedOperationMechanics,
};

mod checkpoint;
mod child;
mod recovery;
pub(super) mod session_basis;
pub(in crate::resident) use checkpoint::{
    IssuedLocalOperationCheckpointFailure, PreparedLocalOperationCheckpointFailure,
};
pub use checkpoint::{
    LocalOperationCheckpointFailure, PreparedLocalOperationCheckpoint,
    PreparedLocalOperationStartFailure,
};
pub use child::{prepare_local_child_parent, PreparedLocalChildParent};
pub(in crate::resident) use child::{
    prepare_local_child_parent_from_recovered, prepare_local_child_start_retained,
};
pub(in crate::resident) use recovery::discovery::{
    discover_local_operation_start, LocalOperationStartDiscoveryAttempt,
    LocalOperationStartDiscoveryError, LocalOperationStartDiscoveryTurn,
};
pub use recovery::{
    recover_local_child_operation, recover_local_operation, recover_local_operation_from_handle,
    LocalChildOperationRecoveryContext, LocalOperationHandleRecoveryInput,
    LocalOperationRecoveryAdmission, LocalOperationRecoveryContext, LocalOperationRecoveryError,
    RecoveredLocalOperation,
};
pub(in crate::resident) use recovery::{
    recover_local_child_operation_from_handle, recover_pending_root_observation,
    LocalChildOperationHandleRecoveryInput, PendingRootObservationError,
};

/// Exact admitted request and owner-resolved coordinates for a local operation.
///
/// The request type is produced by its core contextual admission API. Neither
/// a wire object nor a caller-constructed binding substitutes for that request.
pub struct LocalOperationStartInput<'a> {
    /// Verified root/child start and observation-continuation content selected
    /// independently of the signer witness. This does not
    /// establish physical domain absence or admit a deployment control store.
    pub checkpoint_profile: &'a super::checkpoint_profile::SupportedOperationCheckpointProfile,
    /// Immutable reference allocated for this exact admitted request.
    pub request_ref: ResidentRecordRefV1,
    /// Request already admitted against its caller, route, session and policy.
    pub request: &'a CommonRequestEnvelopeV1,
    /// Immutable reference allocated for the signed locator record.
    pub handle_ref: ResidentRecordRefV1,
    /// Full frozen operation meaning to admit and sign before custody.
    pub handle: OperationHandlePreimageV1,
    /// Existing core parent/lease/generation/signing admission context.
    pub context: &'a OperationHandleAdmissionContextV1<'a>,
    /// Actual application-admitted parent, not a self-consistent lease alone.
    pub session: &'a AdmittedAmbientSession,
    /// Current owner-resolved authority and exact generation for that parent.
    pub current: &'a AmbientSessionAdmissionContext<'a>,
    /// Actual retained definitions owned by the current compiled generation.
    pub generation: &'a AdmittedCompiledGeneration,
    /// Owner-selected publication policy admitted with this exact start.
    /// The private start basis retains it for crash-safe later projection.
    pub publication_policy: LocalOperationPublicationPolicyInput<'a>,
    /// Existing owner-resolved signer witness selecting this operation's store,
    /// domain and accumulator profile before any checkpoint pair is supplied.
    pub checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    /// Immutable reference allocated for the revision-one observation.
    pub observation_ref: ResidentRecordRefV1,
    /// Exact selected observation schema digest.
    pub observation_schema: SchemaDigest,
}

/// Signed local start records not yet paired with a checkpoint.
#[must_use = "preparation is not an issued operation"]
pub struct PreparedLocalOperationStart {
    store: AuthorityStoreCoordinate,
    handle_ref: ResidentRecordRefV1,
    handle: OperationHandleV1,
    observation_ref: ResidentRecordRefV1,
    observation: OperationObservationV1,
    records: Vec<OperationCustodyRecord>,
    indexes: Vec<OperationCustodyIndexMutation>,
    leaves: super::checkpoint_profile::InitialOperationLeaves,
    checkpoint_selection: super::checkpoint_profile::OperationCheckpointSelection,
    basis: StartBasis,
    session_basis: pale_ports::OperationSessionBasis,
    retained_session: RetainedAmbientSession,
    operation_lease: OperationLeaseV1,
    publication_policy: LocalOperationPublicationPolicy,
    consultation_start_intent: Option<super::consultation::ConsultationStartIntent>,
}

impl PreparedLocalOperationStart {
    /// Borrow the private roster frozen into a prepared Consult parent.
    ///
    /// This grants no operation locator or custody authority. The value is
    /// exposed only to the resident composition that starts the exact retained
    /// child prefix after parent commitment.
    pub(in crate::resident) const fn consultation_start_intent(
        &self,
    ) -> Option<&super::consultation::ConsultationStartIntent> {
        self.consultation_start_intent.as_ref()
    }
}

/// Signed handle awaiting its initial signed observation.
///
/// The handle signer has already been consumed. Continuing this phase can only
/// issue the observation around the same handle; it cannot replace or re-sign
/// the retained locator.
#[must_use = "the signed handle still requires its initial observation"]
pub(in crate::resident) struct HandleIssuedLocalOperationStart {
    store: AuthorityStoreCoordinate,
    request_ref: ResidentRecordRefV1,
    request: CommonRequestEnvelopeV1,
    handle_ref: ResidentRecordRefV1,
    handle: OperationHandleV1,
    observation_ref: ResidentRecordRefV1,
    observation_schema: SchemaDigest,
    observation_signing_authority: SigningAuthorityBindingV1,
    checkpoint_selection: super::checkpoint_profile::OperationCheckpointSelection,
    basis: StartBasis,
    retained_session: RetainedAmbientSession,
    operation_lease: OperationLeaseV1,
    publication_policy: LocalOperationPublicationPolicy,
}

/// A signed Consult parent whose roster identities are not yet frozen.
///
/// The handle signature fixes the parent coordinate. No observation,
/// checkpoint, database commit, or anchor mutation has occurred. This phase is
/// held while the process owner allocates the exact fresh child identities that
/// the private roster intent will retain.
#[must_use = "freeze the exact roster and continue this signed parent"]
pub(in crate::resident) struct SignedConsultationParentStart {
    start: Box<HandleIssuedLocalOperationStart>,
}

impl SignedConsultationParentStart {
    /// Borrow the exact signed parent identity and narrowed lease.
    pub(in crate::resident) fn parent(
        &self,
    ) -> (
        &AuthorityStoreCoordinate,
        &ResidentRecordRefV1,
        &OperationHandleV1,
        &OperationLeaseV1,
    ) {
        (
            &self.start.store,
            &self.start.handle_ref,
            &self.start.handle,
            &self.start.operation_lease,
        )
    }

    /// Issue the initial observation and freeze the complete private roster.
    pub(in crate::resident) fn continue_with_intent<ObservationSign>(
        self,
        intent: super::consultation::ConsultationStartIntent,
        sign_observation: ObservationSign,
    ) -> Result<PreparedLocalOperationStart, RetainedLocalOperationStartFailure>
    where
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        self.start
            .issue_observation(sign_observation)
            .map_err(RetainedLocalOperationStartFailure::HandleIssued)?
            .complete(move |_, _, _| Ok(Some(intent)))
            .map_err(RetainedLocalOperationStartFailure::Signed)
    }
}

/// Observation issuance failed after the exact operation handle was signed.
#[must_use = "the exact signed handle can continue without its signer"]
pub(crate) struct HandleIssuedLocalOperationStartFailure {
    continuation: Box<HandleIssuedLocalOperationStart>,
    failure: LocalOperationStartError,
}

impl HandleIssuedLocalOperationStartFailure {
    pub(in crate::resident) fn continue_observation<ObservationSign>(
        self,
        sign_observation: ObservationSign,
    ) -> Result<SignedLocalOperationStart, Self>
    where
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        self.continuation.issue_observation(sign_observation)
    }
}

impl std::fmt::Debug for HandleIssuedLocalOperationStartFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("HandleIssuedLocalOperationStartFailure")
            .field("failure", &self.failure)
            .finish_non_exhaustive()
    }
}

/// Exact signed start records awaiting their fallible physical projection.
///
/// Both signers have already been consumed. Continuing this phase can rebuild
/// only the deterministic records, indexes, and initial leaves around those
/// same signatures.
#[must_use = "signed start records still require deterministic preparation"]
pub(in crate::resident) struct SignedLocalOperationStart {
    store: AuthorityStoreCoordinate,
    request_ref: ResidentRecordRefV1,
    request: CommonRequestEnvelopeV1,
    handle_ref: ResidentRecordRefV1,
    handle: OperationHandleV1,
    observation_ref: ResidentRecordRefV1,
    observation: OperationObservationV1,
    checkpoint_selection: super::checkpoint_profile::OperationCheckpointSelection,
    basis: StartBasis,
    retained_session: RetainedAmbientSession,
    operation_lease: OperationLeaseV1,
    publication_policy: LocalOperationPublicationPolicy,
}

/// Failed deterministic preparation after both start records were signed.
#[must_use = "the exact signed start can be continued without either signer"]
pub(crate) struct SignedLocalOperationStartFailure {
    continuation: Box<SignedLocalOperationStart>,
    failure: LocalOperationStartError,
}

impl SignedLocalOperationStartFailure {
    pub(in crate::resident) fn continue_preparation<Intent>(
        self,
        build_consultation_start_intent: Intent,
    ) -> Result<PreparedLocalOperationStart, Self>
    where
        Intent: FnOnce(
            &AuthorityStoreCoordinate,
            &ResidentRecordRefV1,
            &OperationHandleV1,
        ) -> Result<
            Option<super::consultation::ConsultationStartIntent>,
            LocalOperationStartError,
        >,
    {
        self.continuation.complete(build_consultation_start_intent)
    }
}

impl std::fmt::Debug for SignedLocalOperationStartFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SignedLocalOperationStartFailure")
            .field("failure", &self.failure)
            .finish_non_exhaustive()
    }
}

pub(crate) enum RetainedLocalOperationStartFailure {
    BeforeSigned(LocalOperationStartError),
    HandleIssued(HandleIssuedLocalOperationStartFailure),
    Signed(SignedLocalOperationStartFailure),
}

impl RetainedLocalOperationStartFailure {
    #[allow(
        dead_code,
        reason = "used by the external resident test-support module"
    )]
    pub(in crate::resident) fn into_reason(self) -> LocalOperationStartError {
        match self {
            Self::BeforeSigned(error) => error,
            Self::HandleIssued(failure) => failure.failure,
            Self::Signed(failure) => failure.failure,
        }
    }
}

impl From<LocalOperationStartError> for RetainedLocalOperationStartFailure {
    fn from(error: LocalOperationStartError) -> Self {
        Self::BeforeSigned(error)
    }
}

pub(in crate::resident) enum StartBasis {
    Root,
    Child(Box<PreparedLocalChildParent>),
}

pub(in crate::resident) fn prepare_local_invoke_start_retained<
    Authority,
    HandleSign,
    ObservationSign,
    SessionAuthority,
>(
    input: LocalOperationStartInput<'_>,
    session_authority: &mut SessionAuthority,
    admit_compiled_authority: Authority,
    sign_handle: HandleSign,
    sign_observation: ObservationSign,
) -> Result<PreparedLocalOperationStart, RetainedLocalOperationStartFailure>
where
    Authority: FnOnce(
        &CommonRequestEnvelopeV1,
        &AdmittedAmbientSession,
        &AmbientSessionAdmissionContext<'_>,
        &OperationHandlePreimageV1,
        &OperationLeaseV1,
        &ResolvedOperation,
        &LocalOperationPublicationPolicyInput<'_>,
        &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    ) -> Result<(), InvalidResidentRecordV1>,
    SessionAuthority: AmbientSessionAuthorityPort,
    HandleSign: FnOnce(
        &[u8],
        &OperationHandlePreimageV1,
        &OperationHandleDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    ObservationSign: FnOnce(
        &[u8],
        &OperationObservationPreimageV1,
        &OperationObservationDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
{
    prepare_start_retained(
        input,
        (StartBasis::Root, OperationClassV1::Invoke, None),
        |_, _, _| Ok(None),
        session_authority,
        admit_compiled_authority,
        sign_handle,
        sign_observation,
    )
}

pub(in crate::resident) fn prepare_local_consult_start_retained<
    Authority,
    HandleSign,
    ObservationSign,
    SessionAuthority,
>(
    input: LocalOperationStartInput<'_>,
    intent: Option<super::consultation::ConsultationStartIntent>,
    session_authority: &mut SessionAuthority,
    admit_compiled_authority: Authority,
    sign_handle: HandleSign,
    sign_observation: ObservationSign,
) -> Result<PreparedLocalOperationStart, RetainedLocalOperationStartFailure>
where
    Authority: FnOnce(
        &CommonRequestEnvelopeV1,
        &AdmittedAmbientSession,
        &AmbientSessionAdmissionContext<'_>,
        &OperationHandlePreimageV1,
        &OperationLeaseV1,
        &ResolvedOperation,
        &LocalOperationPublicationPolicyInput<'_>,
        &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    ) -> Result<(), InvalidResidentRecordV1>,
    SessionAuthority: AmbientSessionAuthorityPort,
    HandleSign: FnOnce(
        &[u8],
        &OperationHandlePreimageV1,
        &OperationHandleDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    ObservationSign: FnOnce(
        &[u8],
        &OperationObservationPreimageV1,
        &OperationObservationDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
{
    prepare_start_retained(
        input,
        (StartBasis::Root, OperationClassV1::Consult, None),
        move |_, _, _| Ok(intent),
        session_authority,
        admit_compiled_authority,
        sign_handle,
        sign_observation,
    )
}

/// Admit and sign one root Consult parent without creating its observation.
///
/// This is the narrow phase needed by fresh-child roster construction. The
/// signed handle fixes the parent identity; the caller must next allocate and
/// freeze the exact child plans before it can continue the parent start.
pub(in crate::resident) fn issue_local_consult_parent_start<
    Authority,
    HandleSign,
    SessionAuthority,
>(
    input: LocalOperationStartInput<'_>,
    session_authority: &mut SessionAuthority,
    admit_compiled_authority: Authority,
    sign_handle: HandleSign,
) -> Result<SignedConsultationParentStart, LocalOperationStartError>
where
    Authority: FnOnce(
        &CommonRequestEnvelopeV1,
        &AdmittedAmbientSession,
        &AmbientSessionAdmissionContext<'_>,
        &OperationHandlePreimageV1,
        &OperationLeaseV1,
        &ResolvedOperation,
        &LocalOperationPublicationPolicyInput<'_>,
        &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    ) -> Result<(), InvalidResidentRecordV1>,
    SessionAuthority: AmbientSessionAuthorityPort,
    HandleSign: FnOnce(
        &[u8],
        &OperationHandlePreimageV1,
        &OperationHandleDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
{
    issue_start_handle(
        input,
        (StartBasis::Root, OperationClassV1::Consult, None),
        session_authority,
        admit_compiled_authority,
        sign_handle,
    )
    .map(|start| SignedConsultationParentStart {
        start: Box::new(start),
    })
}

pub(crate) fn prepare_local_consult_start_with_intent_builder_retained<
    Authority,
    HandleSign,
    ObservationSign,
    SessionAuthority,
    Intent,
>(
    input: LocalOperationStartInput<'_>,
    session_authority: &mut SessionAuthority,
    admit_compiled_authority: Authority,
    sign_handle: HandleSign,
    sign_observation: ObservationSign,
    build_intent: Intent,
) -> Result<PreparedLocalOperationStart, RetainedLocalOperationStartFailure>
where
    Authority: FnOnce(
        &CommonRequestEnvelopeV1,
        &AdmittedAmbientSession,
        &AmbientSessionAdmissionContext<'_>,
        &OperationHandlePreimageV1,
        &OperationLeaseV1,
        &ResolvedOperation,
        &LocalOperationPublicationPolicyInput<'_>,
        &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    ) -> Result<(), InvalidResidentRecordV1>,
    SessionAuthority: AmbientSessionAuthorityPort,
    HandleSign: FnOnce(
        &[u8],
        &OperationHandlePreimageV1,
        &OperationHandleDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    ObservationSign: FnOnce(
        &[u8],
        &OperationObservationPreimageV1,
        &OperationObservationDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    Intent:
        FnOnce(
            &AuthorityStoreCoordinate,
            &ResidentRecordRefV1,
            &OperationHandleV1,
        )
            -> Result<super::consultation::ConsultationStartIntent, LocalOperationStartError>,
{
    prepare_start_retained(
        input,
        (StartBasis::Root, OperationClassV1::Consult, None),
        |store, reference, handle| build_intent(store, reference, handle).map(Some),
        session_authority,
        admit_compiled_authority,
        sign_handle,
        sign_observation,
    )
}

pub(in crate::resident) fn prepare_start_retained<
    Authority,
    HandleSign,
    ObservationSign,
    SessionAuthority,
    Intent,
>(
    input: LocalOperationStartInput<'_>,
    preparation: (StartBasis, OperationClassV1, Option<&NativeActionRegistry>),
    build_consultation_start_intent: Intent,
    session_authority: &mut SessionAuthority,
    admit_compiled_authority: Authority,
    sign_handle: HandleSign,
    sign_observation: ObservationSign,
) -> Result<PreparedLocalOperationStart, RetainedLocalOperationStartFailure>
where
    Authority: FnOnce(
        &CommonRequestEnvelopeV1,
        &AdmittedAmbientSession,
        &AmbientSessionAdmissionContext<'_>,
        &OperationHandlePreimageV1,
        &OperationLeaseV1,
        &ResolvedOperation,
        &LocalOperationPublicationPolicyInput<'_>,
        &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    ) -> Result<(), InvalidResidentRecordV1>,
    SessionAuthority: AmbientSessionAuthorityPort,
    HandleSign: FnOnce(
        &[u8],
        &OperationHandlePreimageV1,
        &OperationHandleDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    ObservationSign: FnOnce(
        &[u8],
        &OperationObservationPreimageV1,
        &OperationObservationDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    Intent: FnOnce(
        &AuthorityStoreCoordinate,
        &ResidentRecordRefV1,
        &OperationHandleV1,
    ) -> Result<
        Option<super::consultation::ConsultationStartIntent>,
        LocalOperationStartError,
    >,
{
    issue_start_handle(
        input,
        preparation,
        session_authority,
        admit_compiled_authority,
        sign_handle,
    )
    .map_err(RetainedLocalOperationStartFailure::BeforeSigned)?
    .issue_observation(sign_observation)
    .map_err(RetainedLocalOperationStartFailure::HandleIssued)?
    .complete(build_consultation_start_intent)
    .map_err(RetainedLocalOperationStartFailure::Signed)
}

fn issue_start_handle<Authority, HandleSign, SessionAuthority>(
    input: LocalOperationStartInput<'_>,
    preparation: (StartBasis, OperationClassV1, Option<&NativeActionRegistry>),
    session_authority: &mut SessionAuthority,
    admit_compiled_authority: Authority,
    sign_handle: HandleSign,
) -> Result<HandleIssuedLocalOperationStart, LocalOperationStartError>
where
    Authority: FnOnce(
        &CommonRequestEnvelopeV1,
        &AdmittedAmbientSession,
        &AmbientSessionAdmissionContext<'_>,
        &OperationHandlePreimageV1,
        &OperationLeaseV1,
        &ResolvedOperation,
        &LocalOperationPublicationPolicyInput<'_>,
        &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    ) -> Result<(), InvalidResidentRecordV1>,
    SessionAuthority: AmbientSessionAuthorityPort,
    HandleSign: FnOnce(
        &[u8],
        &OperationHandlePreimageV1,
        &OperationHandleDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
{
    let (basis, expected_class, native_registry) = preparation;
    let binding = input
        .request
        .local_request_binding()
        .map_err(LocalOperationStartError::from)?;
    input
        .session
        .revalidate(input.current, session_authority)
        .map_err(LocalOperationStartError::from)?;
    input
        .context
        .operation_lease
        .verify_against_parent(input.session.lease())
        .map_err(LocalOperationStartError::from)?;
    join_current_request(&input, &binding)?;
    let operation = selected_operation(&input, &binding)?;
    require(input.handle.operation_class == expected_class)?;
    require(input.handle.request_binding == binding && *input.context.request_binding == binding)?;
    if let Some(registry) = native_registry {
        drop(
            prepare_native_request(operation, binding.capability_id(), input.request, registry)
                .map_err(LocalOperationStartError::from)?,
        );
    }
    let checkpoint_selection = match &basis {
        StartBasis::Root => {
            require(
                input.handle.parent_operation.is_none() && input.context.parent_operation.is_none(),
            )?;
            input
                .checkpoint_profile
                .select(
                    &AuthorityStoreCoordinate::new(
                        input.checkpoint_authority.control_store_id.clone(),
                        input.checkpoint_authority.control_incarnation_id.clone(),
                    ),
                    input.request,
                )
                .map_err(LocalOperationStartError::from)?
        }
        StartBasis::Child(parent) => {
            parent.verify_child(&input)?;
            parent
                .selection
                .child(input.checkpoint_profile, input.request)
                .map_err(LocalOperationStartError::from)?
        }
    };
    require(checkpoint_selection.matches_authority(input.checkpoint_authority))?;
    let runtime = input
        .current
        .runtime_authority
        .as_ref()
        .ok_or(LocalOperationStartError::Coordinate)?
        .authority;
    let store = AuthorityStoreCoordinate::new(
        runtime.control_store_id().clone(),
        runtime.control_incarnation_id().clone(),
    );
    let handle = OperationHandleV1::issue_contextual(
        input.handle,
        input.context,
        |handle, _| {
            admit_compiled_authority(
                input.request,
                input.session,
                input.current,
                handle,
                input.context.operation_lease,
                operation,
                &input.publication_policy,
                input.checkpoint_authority,
            )
        },
        sign_handle,
    )
    .map_err(LocalOperationStartError::from)?;
    // Revalidation fixed this authority before the handle signature. Retain
    // the exact signed handle and its observation witness before attempting the
    // second signature so a later observation failure cannot reopen the handle.
    Ok(HandleIssuedLocalOperationStart {
        store,
        request_ref: input.request_ref,
        request: input.request.clone(),
        handle_ref: input.handle_ref,
        handle,
        observation_ref: input.observation_ref,
        observation_schema: input.observation_schema,
        observation_signing_authority: input.context.signed.signing_authority.clone(),
        checkpoint_selection,
        basis,
        retained_session: RetainedAmbientSession::capture(input.session),
        operation_lease: input.context.operation_lease.clone(),
        publication_policy: LocalOperationPublicationPolicy::from_admitted_input(
            input.publication_policy,
        ),
    })
}

impl HandleIssuedLocalOperationStart {
    fn issue_observation<ObservationSign>(
        self,
        sign_observation: ObservationSign,
    ) -> Result<SignedLocalOperationStart, HandleIssuedLocalOperationStartFailure>
    where
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let result = OperationObservationV1::issue_contextual(
            OperationObservationPreimageV1 {
                schema_id: SchemaId::parse(OPERATION_OBSERVATION_V1_SCHEMA)
                    .expect("compiled observation schema identity is valid"),
                schema_sha256: self.observation_schema.clone(),
                operation_handle_id: self.handle.operation_handle_id().clone(),
                operation_handle_ref: self.handle_ref.clone(),
                operation_handle_sha256: self.handle.canonical_operation_handle_sha256().clone(),
                revision: OperationObservationRevisionV1::new(1)
                    .expect("initial observation revision is positive"),
                predecessor_ref: None,
                predecessor_sha256: None,
                state: OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::Resolving,
                },
                observed_at: *self.handle.created_at(),
                deployment_runtime_authority_ref: self
                    .handle
                    .deployment_runtime_authority_ref()
                    .clone(),
                deployment_runtime_authority_sha256: self
                    .handle
                    .deployment_runtime_authority_sha256()
                    .clone(),
                issuer_principal_id: self.handle.issuer_principal_id().clone(),
                issuer_key_id: self.handle.issuer_key_id().clone(),
                issuer_key_epoch: *self.handle.issuer_key_epoch(),
            },
            &OperationObservationAdmissionContextV1 {
                signed: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &self.observation_schema,
                    signing_authority: &self.observation_signing_authority,
                },
                handle_ref: &self.handle_ref,
                handle: &self.handle,
                predecessor: None,
            },
            // Resolving is constructed here and names no external state evidence.
            |_| Ok(()),
            // Revision one has no predecessor authority to rotate.
            |_, _, _, _| {
                Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "local operation start",
                    invariant: "initial observation has no rotation",
                })
            },
            sign_observation,
        )
        .map_err(LocalOperationStartError::from);
        let observation = match result {
            Ok(observation) => observation,
            Err(failure) => {
                return Err(HandleIssuedLocalOperationStartFailure {
                    continuation: Box::new(self),
                    failure,
                });
            }
        };
        Ok(SignedLocalOperationStart {
            store: self.store,
            request_ref: self.request_ref,
            request: self.request,
            handle_ref: self.handle_ref,
            handle: self.handle,
            observation_ref: self.observation_ref,
            observation,
            checkpoint_selection: self.checkpoint_selection,
            basis: self.basis,
            retained_session: self.retained_session,
            operation_lease: self.operation_lease,
            publication_policy: self.publication_policy,
        })
    }
}

impl SignedLocalOperationStart {
    pub(in crate::resident) fn complete<Intent>(
        self,
        build_consultation_start_intent: Intent,
    ) -> Result<PreparedLocalOperationStart, SignedLocalOperationStartFailure>
    where
        Intent: FnOnce(
            &AuthorityStoreCoordinate,
            &ResidentRecordRefV1,
            &OperationHandleV1,
        ) -> Result<
            Option<super::consultation::ConsultationStartIntent>,
            LocalOperationStartError,
        >,
    {
        let completion = (|| {
            let consultation_start_intent =
                build_consultation_start_intent(&self.store, &self.handle_ref, &self.handle)?;
            if let Some(intent) = &consultation_start_intent {
                intent
                    .validate_signed_parent(&self.store, &self.handle_ref, &self.handle)
                    .map_err(|_| LocalOperationStartError::Coordinate)?;
            }
            let request_record =
                record_projection::request_record(&self.request_ref, &self.request)?;
            let handle_record = record_projection::record(
                record_projection::coordinate(
                    &self.handle_ref,
                    self.handle.canonical_operation_handle_sha256().as_str(),
                )?,
                &self.handle,
            )?;
            let observation_record = record_projection::record(
                record_projection::coordinate(
                    &self.observation_ref,
                    self.observation
                        .canonical_operation_observation_sha256()
                        .as_str(),
                )?,
                &self.observation,
            )?;
            let session_basis = session_basis::capture(
                &self.checkpoint_selection,
                (&self.handle_ref, &self.handle),
                (&self.request_ref, &self.request),
                &self.retained_session,
                &self.publication_policy,
                consultation_start_intent.as_ref(),
            )?;
            let leaves = super::checkpoint_profile::initial_operation_leaves(
                &self.checkpoint_selection,
                (&self.request_ref, &self.request),
                (&self.handle_ref, &self.handle),
                (&self.observation_ref, &self.observation),
                &session_basis,
                self.retained_session.lease(),
            )?;
            let mut indexes = leaves.indexes()?;
            if let StartBasis::Child(parent) = &self.basis {
                indexes.push(child::membership_index(
                    parent,
                    &self.handle_ref,
                    &self.handle,
                )?);
            }
            Ok::<_, LocalOperationStartError>((
                consultation_start_intent,
                request_record,
                handle_record,
                observation_record,
                session_basis,
                leaves,
                indexes,
            ))
        })();
        let (
            consultation_start_intent,
            request_record,
            handle_record,
            observation_record,
            session_basis,
            leaves,
            indexes,
        ) = match completion {
            Ok(completion) => completion,
            Err(failure) => {
                return Err(SignedLocalOperationStartFailure {
                    continuation: Box::new(self),
                    failure,
                });
            }
        };
        Ok(PreparedLocalOperationStart {
            store: self.store,
            handle_ref: self.handle_ref,
            handle: self.handle,
            observation_ref: self.observation_ref,
            observation: self.observation,
            records: vec![
                OperationCustodyRecord::CommonRequest(request_record),
                OperationCustodyRecord::OperationHandle(handle_record),
                OperationCustodyRecord::OperationObservation(observation_record),
            ],
            indexes,
            leaves,
            checkpoint_selection: self.checkpoint_selection,
            basis: self.basis,
            session_basis,
            retained_session: self.retained_session,
            operation_lease: self.operation_lease,
            publication_policy: self.publication_policy,
            consultation_start_intent,
        })
    }
}

#[cfg(test)]
#[path = "../../tests/support/local_operation_records.rs"]
mod local_operation_records_test_support;

/// Contextually admitted local locator awaiting exact database and independent-anchor custody.
/// No method on this phase exposes an issued locator or dispatch capability.
///
/// Database custody cannot substitute for completed custody:
/// ```compile_fail
/// use pale_application::resident::{PendingLocalOperationStart,
///     DatabaseCommittedOperationStartAnchorPending};
/// fn expose(pending: PendingLocalOperationStart,
///     database: DatabaseCommittedOperationStartAnchorPending) {
///     let _ = pending.finish(database);
/// }
/// ```
#[must_use = "only exact completed custody can expose this locator"]
pub struct PendingLocalOperationStart {
    checkpoint_selection: super::checkpoint_profile::OperationCheckpointSelection,
    expected: OperationCustodyCandidate,
    handle_ref: ResidentRecordRefV1,
    handle: OperationHandleV1,
    observation_ref: ResidentRecordRefV1,
    observation: OperationObservationV1,
    retained_session: RetainedAmbientSession,
    operation_lease: OperationLeaseV1,
    publication_policy: LocalOperationPublicationPolicy,
    consultation_start_intent: Option<super::consultation::ConsultationStartIntent>,
}

impl PendingLocalOperationStart {
    /// Expose the locator only after the exact prepared start completed both physical steps.
    ///
    /// # Errors
    /// A different completed operation, index set, checkpoint or store is rejected.
    /// On mismatch both owners are returned intact; no locator is issued.
    pub fn finish(
        self,
        completed: CompletedOperationStartCustody,
    ) -> Result<IssuedLocalOperation, Box<(Self, CompletedOperationStartCustody)>> {
        let actual = completed.candidate();
        if actual != &self.expected {
            return Err(Box::new((self, completed)));
        }
        let Some(request) = self
            .expected
            .record(pale_ports::OperationCustodyRecordKind::CommonRequest)
        else {
            return Err(Box::new((self, completed)));
        };
        let request_coordinate = request.coordinate().clone();
        Ok(IssuedLocalOperation {
            checkpoint_selection: self.checkpoint_selection,
            request_coordinate,
            handle_ref: self.handle_ref,
            handle: self.handle,
            initial_observation_ref: self.observation_ref,
            initial_observation: self.observation,
            retained_session: self.retained_session,
            operation_lease: self.operation_lease,
            publication_policy: self.publication_policy,
            consultation_start_intent: self.consultation_start_intent,
        })
    }
}

/// Immutable local operation locator issued after exact custody and confirmation.
/// This proves start, not completion, current continuation authority or an effect.
///
/// A signed core record cannot construct an issued application locator:
/// ```compile_fail
/// use pale_application::resident::IssuedLocalOperation;
/// use pale_core::records::{ResidentRecordRefV1, OperationHandleV1};
/// fn substitute(handle_ref: ResidentRecordRefV1, handle: OperationHandleV1) {
///     let _ = IssuedLocalOperation { handle_ref, handle };
/// }
/// ```
pub struct IssuedLocalOperation {
    checkpoint_selection: super::checkpoint_profile::OperationCheckpointSelection,
    request_coordinate: pale_ports::ControlRecordCoordinate,
    handle_ref: ResidentRecordRefV1,
    handle: OperationHandleV1,
    initial_observation_ref: ResidentRecordRefV1,
    initial_observation: OperationObservationV1,
    retained_session: RetainedAmbientSession,
    operation_lease: OperationLeaseV1,
    publication_policy: LocalOperationPublicationPolicy,
    consultation_start_intent: Option<super::consultation::ConsultationStartIntent>,
}

impl IssuedLocalOperation {
    /// Copy the immutable operation basis into one exact recovery continuation.
    ///
    /// This private copy preserves an already issued identity after a signed
    /// phase. It does not issue another handle, renew authority, or grant a
    /// provider or native call.
    pub(in crate::resident) fn clone_for_recovery_continuation(&self) -> Self {
        Self {
            checkpoint_selection: self.checkpoint_selection.clone(),
            request_coordinate: self.request_coordinate.clone(),
            handle_ref: self.handle_ref.clone(),
            handle: self.handle.clone(),
            initial_observation_ref: self.initial_observation_ref.clone(),
            initial_observation: self.initial_observation.clone(),
            retained_session: self.retained_session.clone_for_recovery_continuation(),
            operation_lease: self.operation_lease.clone(),
            publication_policy: self.publication_policy.clone(),
            consultation_start_intent: self.consultation_start_intent.clone(),
        }
    }

    pub(in crate::resident) const fn consultation_start_intent(
        &self,
    ) -> Option<&super::consultation::ConsultationStartIntent> {
        self.consultation_start_intent.as_ref()
    }
    /// Borrow the immutable publication policy admitted and retained at start.
    ///
    /// This is historical projection meaning. It does not authenticate a
    /// recipient, deliver an Event, resolve a payload, or grant current access.
    #[must_use]
    pub const fn publication_policy(&self) -> &LocalOperationPublicationPolicy {
        &self.publication_policy
    }
    pub(in crate::resident) fn same_session(&self, session: &AdmittedAmbientSession) -> bool {
        self.retained_session.same_session(session)
    }

    /// Borrow the exact historical operation lease named by this handle.
    ///
    /// Start captures its admitted input; protected recovery retains the value
    /// already re-admitted against its original session. This immutable evidence
    /// may outlive execution validity. It grants no retry, dispatch, cancellation
    /// or current admission and performs no additional read or admission.
    #[must_use]
    pub const fn historical_operation_lease(&self) -> &OperationLeaseV1 {
        &self.operation_lease
    }

    /// Borrow the original core session lease retained with this operation.
    ///
    /// This is historical evidence, not an application-admitted ambient session.
    /// Expiry or revocation does not erase history; current continuation still
    /// requires [`Self::revalidate_session`] and the operation's existing checks.
    /// This borrows the existing retained-session owner without another read.
    #[must_use]
    pub fn historical_session_lease(&self) -> &SessionAdmissionLeaseV1 {
        self.retained_session.lease()
    }

    /// Revalidate the exact ambient-session basis consumed by this operation.
    ///
    /// Historical custody remains readable without this check. Only success
    /// constructs a current admitted session; expired, revoked, stale or unlike
    /// owner-resolved context cannot replace the original basis. This does not
    /// authenticate a caller or transport, grant disclosure, renew the lease or
    /// recover a session that never started an operation.
    ///
    /// # Errors
    /// Returns the existing session admission failure without changing custody.
    pub fn revalidate_session<P: AmbientSessionAuthorityPort>(
        &self,
        context: &AmbientSessionAdmissionContext<'_>,
        authority: &mut P,
    ) -> Result<AdmittedAmbientSession, AmbientSessionAdmissionError> {
        self.retained_session.revalidate(context, authority)
    }

    /// Borrow the original request location proved at start or protected recovery.
    /// Identical request bytes under another reference are not this association.
    #[must_use]
    pub const fn request_coordinate(&self) -> &pale_ports::ControlRecordCoordinate {
        &self.request_coordinate
    }
    /// Retain the exact start selection for a separately admitted observation.
    /// This does not grant current continuation or signing authority.
    #[must_use]
    pub fn checkpoint_selection(&self) -> super::checkpoint_profile::OperationCheckpointSelection {
        self.checkpoint_selection.clone()
    }
    /// Borrow the immutable issued locator and its complete signed coordinate.
    #[must_use]
    pub fn handle(&self) -> (&ResidentRecordRefV1, &OperationHandleV1) {
        (&self.handle_ref, &self.handle)
    }

    /// Borrow the exact revision-one observation retained with this start.
    ///
    /// This is the signed durable start record, not a current observation or a
    /// claim that the operation has progressed beyond its initial phase.
    #[must_use]
    pub(in crate::resident) fn initial_observation(
        &self,
    ) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.initial_observation_ref, &self.initial_observation)
    }
}

/// Failure before an application locator can be issued.
#[derive(Debug, thiserror::Error)]
pub enum LocalOperationStartError {
    /// The selected native request failed its canonical registry/input checks.
    #[error(transparent)]
    NativeAction(#[from] NativeActionInvocationError),
    /// Bounded protected parent history or its current anchored membership failed.
    #[error(transparent)]
    ParentObservation(#[from] super::OperationObservationReadmissionError),
    /// Bounded logical leaf preparation was rejected before a native read.
    #[error(transparent)]
    AccumulatorInput(#[from] pale_ports::AccumulatorPreparationInputError),
    /// Native proof preparation failed without committing operation state.
    #[error(transparent)]
    AccumulatorPreparation(#[from] pale_ports::OperationAccumulatorPreparationError),
    /// Prepared physical changes do not form the exact selected checkpoint.
    #[error(transparent)]
    AccumulatorBatch(#[from] pale_ports::AccumulatorBatchError),
    /// The source-supported profile or its exact operation association differs.
    #[error(transparent)]
    Profile(#[from] super::checkpoint_profile::OperationCheckpointProfileError),
    /// Current application session admission or parent narrowing failed.
    #[error(transparent)]
    Session(#[from] AmbientSessionAdmissionError),
    /// An existing contextual admission or signature owner rejected its input.
    #[error(transparent)]
    Admission(#[from] InvalidResidentRecordV1),
    /// The supplied records do not belong to this local operation and its parent.
    #[error("local operation start coordinate mismatch")]
    Coordinate,
    /// The exact repository/Package operation is absent from this generation.
    #[error("local operation is absent from the admitted generation")]
    OperationAbsent,
    /// The operation exists but has no private binding.
    #[error("local operation is unbound")]
    OperationUnbound,
    /// The selected operation has no capability requirements to invoke.
    #[error("local operation is semantic-only")]
    OperationSemanticOnly,
    /// No exact capability requirement is present or its effect exceeds the lease.
    #[error("local operation capability is outside the selected requirement or lease")]
    OperationCapability,
    /// Existing Package/private closure prevents this requirement from proceeding.
    #[error("local operation requirement {subject:?} cannot proceed: {disposition:?}")]
    OperationRequirement {
        /// Exact owner-qualified operation and requirement, including a blocking sibling.
        subject: ObligationSubject,
        /// The capability of that requirement, not necessarily the requested capability.
        capability: CapabilityIdV1,
        /// Unchanged owner disposition, including its unavailable reason and fallback.
        disposition: Box<CapabilityObligationDisposition>,
    },
    /// Canonical record projection failed.
    #[error(transparent)]
    Canonicalize(#[from] canon::CanonError),
    /// A physical record coordinate or byte bound was invalid.
    #[error(transparent)]
    Record(#[from] ControlRecordError),
    /// The transaction's closed record/index/checkpoint shape was invalid.
    #[error(transparent)]
    Candidate(#[from] OperationCustodyCandidateError),
    /// Exact key or recovery projection failed.
    #[error(transparent)]
    Recovery(#[from] OperationRecoveryError),
    /// The prepared physical start or anchor relation was invalid.
    #[error(transparent)]
    Preparation(#[from] OperationStartPreparationError),
}

impl From<NativeRequestError> for LocalOperationStartError {
    fn from(error: NativeRequestError) -> Self {
        match error {
            NativeRequestError::Association => Self::Coordinate,
            NativeRequestError::Implementation => Self::OperationCapability,
            NativeRequestError::NativeAction(error) => Self::NativeAction(error),
        }
    }
}

impl From<super::checkpoint_profile::InitialOperationLeafError> for LocalOperationStartError {
    fn from(error: super::checkpoint_profile::InitialOperationLeafError) -> Self {
        use super::checkpoint_profile::{InitialOperationLeafError, OperationLeafProjectionError};
        match error {
            InitialOperationLeafError::Profile(error) => Self::Profile(error),
            InitialOperationLeafError::Lookup(error) => Self::Recovery(error),
            InitialOperationLeafError::Leaf(OperationLeafProjectionError::Store) => {
                Self::Coordinate
            }
            InitialOperationLeafError::Leaf(OperationLeafProjectionError::Identity(error)) => {
                Self::Admission(error)
            }
            InitialOperationLeafError::Leaf(OperationLeafProjectionError::Canonicalize(error)) => {
                Self::Canonicalize(error)
            }
        }
    }
}

impl From<ControlRecordProjectionError> for LocalOperationStartError {
    fn from(error: ControlRecordProjectionError) -> Self {
        match error {
            ControlRecordProjectionError::Canonicalize(error) => Self::Canonicalize(error),
            ControlRecordProjectionError::Record(error) => Self::Record(error),
        }
    }
}

fn require(valid: bool) -> Result<(), LocalOperationStartError> {
    valid
        .then_some(())
        .ok_or(LocalOperationStartError::Coordinate)
}

/// Join axes with existing identical meanings; do not reinterpret workspace or transport labels.
fn join_current_request(
    input: &LocalOperationStartInput<'_>,
    binding: &RequestBindingV1,
) -> Result<(), LocalOperationStartError> {
    let session = input.session.lease();
    let caller = input.request.caller();
    let subject = input.current.subject;
    let route = input.request.route();
    let selected = input.current.route_use;
    let scope = input.request.scope();
    let grant = input
        .current
        .grant
        .as_ref()
        .ok_or(LocalOperationStartError::Coordinate)?
        .grant;
    let admission = input.request.session_admission();
    let runtime = input
        .current
        .runtime_authority
        .as_ref()
        .ok_or(LocalOperationStartError::Coordinate)?
        .authority;
    // These are two representations of the principal workspace. Project and
    // execution-workspace identities remain joined by the session owner.
    require(
        input.request.workspace_id().as_str()
            == input
                .current
                .generation
                .effective_profile
                .workspace_id
                .as_str(),
    )?;
    require(
        caller.identity == subject.identity
            && caller.role == subject.role
            && caller.surface == subject.surface
            && caller.signer_key_id == subject.subject_key_id
            && caller.signer_key_epoch == subject.subject_key_epoch,
    )?;
    require(
        binding.session_grant_id() == input.session.grant_id().as_str()
            && binding.session_grant_sha256().as_str() == input.session.grant_sha256().as_str()
            && binding.deployment_binding_ref() == session.deployment_binding_ref()
            && binding.deployment_binding_sha256() == session.deployment_binding_sha256(),
    )?;
    require(
        &admission.session_admission_id == session.session_admission_id()
            && &admission.session_admission_lease_digest
                == session.session_admission_lease_digest()
            && &admission.session_floor_epoch == grant.session_floor_epoch()
            && &admission.grant_issuer_principal_id == grant.issuer_principal_id()
            && &admission.grant_issuer_key_id == grant.issuer_key_id()
            && admission.grant_issuer_key_epoch.get() == grant.issuer_key_epoch().get()
            && admission.checked_revocation_epoch == input.current.session_revocations.epoch
            && admission.checked_revocation_ref.as_ref()
                == input.current.session_revocations.revocation_ref
            && admission.checked_revocation_sha256.as_ref()
                == input.current.session_revocations.revocation_sha256
            && admission.checked_at <= input.current.now,
    )?;
    require(
        &scope.principal_scope == session.principal_id()
            && &scope.controller_scope == session.controller_id()
            && scope.relationship_scope.as_ref() == session.relationship_scope().as_ref(),
    )?;
    require(
        &route.origin_principal_id == session.principal_id()
            && &route.origin_node_id == selected.origin_node_id
            && &route.destination_principal_id == selected.destination_principal_id
            && route.destination_node_id.as_ref() == selected.destination_node_id
            && route.route_binding_schema_id == selected.route.route_binding_schema_id
            && route.route_binding_ref == selected.route.route_binding_ref
            && route.route_binding_sha256 == selected.route.route_binding_sha256
            && route.route_id == selected.route.route_id
            && route.route_generation == selected.route.route_generation
            && route.contract_set_ref == selected.contract_set.contract_set_ref
            && route.contract_set_sha256.as_str()
                == selected.contract_set.contract_set_sha256.as_str(),
    )?;
    require(
        input.context.compiled_generation_sha256 == session.compiled_generation_digest()
            && input.generation.record().compiled_generation_digest()
                == input
                    .current
                    .generation
                    .compiled_generation
                    .compiled_generation_digest()
            && input.context.activation_fence_sha256 == session.activation_fence_digest()
            && input.context.instance_sha256 == session.instance_digest()
            && &input.handle.deployment_runtime_authority_ref
                == input.session.runtime_authority_ref()
            && input.handle.deployment_runtime_authority_sha256.as_str()
                == input.session.runtime_authority_sha256().as_str(),
    )?;
    require(input.request.operation() == input.context.operation_lease.operation())?;
    require(
        runtime
            .permitted_record_schema_ids()
            .iter()
            .any(|schema| schema.as_str() == OPERATION_HANDLE_V1_SCHEMA)
            && runtime
                .permitted_record_schema_ids()
                .iter()
                .any(|schema| schema.as_str() == OPERATION_OBSERVATION_V1_SCHEMA)
            && input.checkpoint_authority.control_store_id == runtime.control_store_id()
            && input.checkpoint_authority.control_incarnation_id
                == runtime.control_incarnation_id()
            && input.checkpoint_authority.principal_id == runtime.principal_id(),
    )?;
    require(
        &input.handle.issuer_principal_id == runtime.principal_id()
            && &input.handle.issuer_key_id == runtime.subject_key_id()
            && input.handle.issuer_key_epoch == *runtime.subject_key_epoch(),
    )?;
    require(
        input
            .context
            .operation_lease
            .capabilities()
            .binary_search(binding.capability_id())
            .is_ok()
            && input.request.issued_at() <= input.current.now
            && input.current.now < input.request.expires_at()
            && input.handle.created_at == input.current.now
            && input.handle.expires_at <= input.request.expires_at()
            && input.current.now < input.context.operation_lease.expires_at(),
    )?;
    Ok(())
}

fn selected_operation<'a>(
    input: &LocalOperationStartInput<'a>,
    binding: &RequestBindingV1,
) -> Result<&'a ResolvedOperation, LocalOperationStartError> {
    let operation = input
        .generation
        .operation(
            input.session.lease().repository_id(),
            input.request.operation(),
        )
        .ok_or(LocalOperationStartError::OperationAbsent)?;
    let capabilities = match &operation.mechanics {
        ResolvedOperationMechanics::Unbound => {
            return Err(LocalOperationStartError::OperationUnbound);
        }
        ResolvedOperationMechanics::SemanticOnly { .. } => {
            return Err(LocalOperationStartError::OperationSemanticOnly);
        }
        ResolvedOperationMechanics::CapabilityBearing { capabilities, .. } => capabilities,
    };
    let mut found = false;
    for requirement in capabilities {
        // The established owner decides unavailable policy. Its owned result
        // is transient here; no second policy table or readiness vector is retained.
        let disposition = obligation_disposition(&requirement.provider, &requirement.policy);
        if matches!(disposition, CapabilityObligationDisposition::Blocked { .. })
            || (&requirement.capability_id == binding.capability_id()
                && !matches!(disposition, CapabilityObligationDisposition::Bound(_)))
        {
            return Err(LocalOperationStartError::OperationRequirement {
                subject: ObligationSubject {
                    operation: operation.record.id.clone(),
                    requirement: requirement.requirement_id.clone(),
                },
                capability: requirement.capability_id.clone(),
                disposition: Box::new(disposition),
            });
        }
        if &requirement.capability_id == binding.capability_id() {
            found = true;
            if !input
                .context
                .operation_lease
                .effects()
                .contains(&requirement.effect_class)
            {
                return Err(LocalOperationStartError::OperationCapability);
            }
        }
    }
    if !found {
        return Err(LocalOperationStartError::OperationCapability);
    }
    // Preserve every requirement and provider disposition for the actual owner;
    // finding a declared capability is not current provider readiness.
    Ok(operation)
}

fn checkpoint_record(
    (reference, checkpoint): (&ResidentRecordRefV1, &ControlStateCheckpointV1),
) -> Result<(ControlCheckpointHead, ControlRecord), LocalOperationStartError> {
    let coordinate = record_projection::coordinate(
        reference,
        checkpoint.canonical_control_checkpoint_sha256().as_str(),
    )?;
    let head = ControlCheckpointHead::new(
        checkpoint.checkpoint_domain_id().clone(),
        *checkpoint.domain_epoch(),
        *checkpoint.domain_sequence(),
        *checkpoint.committed_transaction_sequence(),
        coordinate.clone(),
    );
    Ok((head, record_projection::record(coordinate, checkpoint)?))
}

#[cfg(test)]
#[path = "../../tests/internal/resident/local_operation_start/phase.rs"]
mod tests;
