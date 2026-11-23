//! Public composition of resident frame services.
//!
//! Deployment code supplies exact admitted evidence and physical owners. This
//! module constructs the existing resident program; it does not discover an
//! endpoint, select production credentials, or implement a second request path.

use std::{num::NonZeroUsize, sync::Arc, task::Context, time::Instant};

use pale_core::records::*;
use pale_ports::*;

use super::a2a_cancel_task::ResidentProcessRecoveredA2aCancelTaskFailure;
use super::a2a_delegate::{
    ResidentProcessA2aDelegateCompletion, ResidentProcessA2aDelegateLaunch,
    ResidentProcessA2aDelegateLaunchFailure, ResidentProcessA2aDelegateLaunchPlan,
    ResidentProcessA2aDelegateRestartFailure, ResidentProcessA2aDelegateRestartRetry,
    ResidentProcessA2aDelegateRetention, ResidentProcessA2aDelegateRetentionFailure,
    ResidentProcessPairwiseA2aDelegateLaunchFailure, ResidentProcessRecoveredA2aDelegate,
};
use super::a2a_delegate_recovery_owner::{
    ResidentA2aDelegateRecoveryOwner, ResidentA2aDelegateRecoveryPort,
};
use super::{
    consult_recovery, current_request, delegate_recovery_sweep, operation_recovery_sweep,
    operation_request_host, program, restart, ResidentProcessFreshStartAdmission,
    ResidentProcessHost, ResidentProcessHostSelectionError, ResidentProcessInvokePolicy,
    ResidentProcessInvokePolicyContext, ResidentProcessInvokePolicyEvaluator,
};
use crate::resident::{
    consultation::{ResidentConsultationPolicyFrontier, ResidentConsultationSupervisionDecision},
    operation_service::ResidentOperationTaskState,
    task_owner::ResidentOperationTaskLimits,
    task_reservation::ResidentAggregateTaskLimits,
    A2aTaskPublicationRecoveryLimits, A2aTaskPublicationRecoveryRequest,
    A2aTaskPublicationSourceRecoveryError, AmbientSessionAuthorityPort,
    GenerationLifecycleRecoveryResolver, LocalOperationPublicationCurrent,
    LocalOperationPublicationIssuance, LocalOperationPublicationPolicy,
    LocalOperationPublicationReadmission, LocalOperationPublicationRecoveryLimits,
    LocalOperationPublicationRecoveryRequest, LocalOperationPublicationSourceRecoveryError,
    LocalOperationPublicationSweepLimits, LocalOperationRecoveryContext,
    OperationCheckpointAncestryLimits, OperationControlObservation,
    OperationControlRecoveryAdmission, OperationControlRecoveryInput,
    OperationObservationRecoveryLimits, RecoveredA2aTaskPublicationSource,
    RecoveredOperationControl, ResidentColdOpenInput, ResidentFrameService,
    ResidentServicePolicyProjection, ResidentServiceTrustFrontier,
};

#[path = "service_assembly/callbacks.rs"]
mod callbacks;
pub use callbacks::*;

#[path = "service_assembly/expiry.rs"]
mod expiry;

#[cfg(any(test, feature = "control-store-test-support"))]
#[path = "../../../tests/support/resident_hook_response_restart.rs"]
#[doc(hidden)]
pub mod hook_response_restart_test_support;

#[cfg(test)]
#[path = "../../../tests/internal/resident/process_host/public_consult_service_assembly.rs"]
mod public_consult_tests;

/// Exact writer records admitted from one physical current-writer snapshot.
pub struct ResidentServiceCurrentWriterEvidence {
    /// Admitted writer-fence record.
    pub writer: PrincipalResidentWriterFenceV1,
    /// Admitted control checkpoint named by the writer head.
    pub checkpoint: ControlStateCheckpointV1,
}

/// Record-specific admission for the physical current-writer snapshot.
pub trait ResidentServiceCurrentWriterAdmissionPort {
    /// Admit the exact writer and checkpoint bytes represented by `snapshot`.
    ///
    /// # Errors
    ///
    /// Returns an error when either record is invalid or the checkpoint does
    /// not establish the writer head in the supplied snapshot.
    fn admit_current_writer(
        &mut self,
        snapshot: &PrincipalResidentWriterSnapshot,
    ) -> Result<ResidentServiceCurrentWriterEvidence, InvalidResidentRecordV1>;
}

/// Store-bound ports and recovery input for one admitted deployment.
pub struct ResidentServiceDeploymentOpen<'a> {
    /// Exact store coordinate named by the deployment recovery input.
    pub store: AuthorityStoreCoordinate,
    /// Durable authority journal and record store.
    pub durable: &'a dyn DurableAuthorityStorePort,
    /// Durable session-lease custody for this store.
    pub sessions: &'a dyn SessionLeaseCustodyPort,
    /// Event custody used during resident recovery.
    pub events: &'a mut dyn EventCustodyPort,
    /// Aggregate bytes accepted for one recovered event closure.
    pub maximum_event_custody_bytes: usize,
    /// Exact semantic and generation recovery input for this store.
    pub cold_open: ResidentColdOpenInput<'a>,
    /// Durable operation custody for admitted frame turns.
    pub operations: &'a mut dyn OperationCustodyPort,
    /// Independent anti-rollback owner for admitted operation state.
    pub anti_rollback: &'a mut dyn AntiRollbackAnchorPort,
}

/// Process-local occupancy bounds for one resident program.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResidentServiceTaskLimits {
    /// Maximum accepted provider tasks.
    pub maximum_provider_tasks: NonZeroUsize,
    /// Maximum active consultation batches.
    pub maximum_consultation_batches: NonZeroUsize,
    /// Maximum work retained across every task route.
    pub maximum_aggregate_tasks: NonZeroUsize,
}

/// Process recovery inputs retained by the public resident service.
pub struct ResidentServiceProcessInput<'evidence, 'service, WriterGuard, WriterAdmission> {
    /// Native guard that already holds the principal writer lock.
    pub held_writer: WriterGuard,
    /// Physical writer head observed while that guard is held.
    pub writer_snapshot: &'evidence PrincipalResidentWriterSnapshot,
    /// Record-specific admission owner for the writer evidence.
    pub writer_admission: WriterAdmission,
    /// Opaque trust frontier admitted by the existing trust recovery owner.
    pub trust: &'evidence ResidentServiceTrustFrontier,
    /// Complete caller-opened deployment set.
    pub deployments: Vec<ResidentServiceDeploymentOpen<'service>>,
    /// Maximum number of ready deployments retained by the process.
    pub maximum_ready_deployments: NonZeroUsize,
    /// Bounded process-local task ownership.
    pub task_limits: ResidentServiceTaskLimits,
}

/// Evidence that the selected local-peer policy admitted one connection.
///
/// This value carries only the kernel-reported coordinates accepted by the
/// peer-admission owner. It is not a Pale principal, session, signature,
/// disclosure decision, or operation authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResidentPeerAdmissionWitness {
    credentials: LocalIpcPeerCredentials,
}

impl ResidentPeerAdmissionWitness {
    /// Retain the exact kernel-reported coordinates accepted by local policy.
    #[must_use]
    pub const fn new(credentials: LocalIpcPeerCredentials) -> Self {
        Self { credentials }
    }

    /// Return the admitted kernel-reported coordinates.
    #[must_use]
    pub const fn credentials(self) -> LocalIpcPeerCredentials {
        self.credentials
    }
}

/// Admit kernel-reported credentials for each accepted local connection.
///
/// Implementations own local endpoint policy. Admission happens before any
/// protected authority record is read. A returned witness is supplementary
/// transport evidence only; every request still requires its independent
/// session, signature, disclosure, and operation-authority checks. The mutable
/// receiver lets a policy advance its own bounded admission state.
pub trait ResidentPeerAdmissionPort {
    /// Admit one exact kernel observation without reading protected state.
    ///
    /// # Errors
    ///
    /// Returns an error when the connected peer does not satisfy the selected
    /// local endpoint policy.
    fn admit_peer(
        &mut self,
        credentials: LocalIpcPeerCredentials,
    ) -> Result<ResidentPeerAdmissionWitness, InvalidResidentRecordV1>;
}

/// Exact selector used to read current request authority after peer admission.
pub struct ResidentServiceCurrentRequestSelector<'a> {
    /// Per-connection peer-admission witness used only to select current records.
    pub peer_witness: &'a ResidentPeerAdmissionWitness,
    /// Claimed retained session ID.
    pub session_admission_id: &'a SessionAdmissionIdV1,
    /// Claimed retained session digest.
    pub session_admission_lease_digest: &'a SessionAdmissionLeaseDigest,
    /// Claimed capability-grant ID.
    pub session_grant_id: &'a SessionGrantIdV1,
    /// Claimed deployment-binding reference.
    pub deployment_binding_ref: &'a ResidentRecordRefV1,
    /// Claimed deployment-binding digest.
    pub deployment_binding_sha256: &'a ArtifactDigest,
}

/// An admitted capability grant and its issuance trust checkpoint.
#[derive(Clone)]
pub struct ResidentServiceGrantEvidence {
    /// Immutable grant reference.
    pub grant_ref: ResidentRecordRefV1,
    /// Admitted grant record.
    pub grant: CapabilitySessionGrantV1,
    /// Immutable reference of the trust checkpoint used at issuance.
    pub issued_trust_checkpoint_ref: ResidentRecordRefV1,
    /// Admitted trust checkpoint used at issuance.
    pub issued_trust_checkpoint: PrincipalTrustCheckpointV1,
}

/// Current grant-specific revocation evidence.
#[derive(Clone)]
pub struct ResidentServiceGrantRevocationEvidence {
    /// Immutable revocation reference.
    pub revocation_ref: ResidentRecordRefV1,
    /// Admitted revocation record.
    pub revocation: CapabilitySessionGrantRevocationV1,
}

/// Current capability-revocation checkpoint and optional grant head.
#[derive(Clone)]
pub struct ResidentServiceCapabilityRevocationEvidence {
    /// Immutable checkpoint reference.
    pub checkpoint_ref: ResidentRecordRefV1,
    /// Admitted current checkpoint.
    pub checkpoint: CapabilityRevocationCheckpointV1,
    /// Current revocation for the selected grant, when one exists.
    pub grant_head: Option<ResidentServiceGrantRevocationEvidence>,
}

/// Current deployment-binding revocation evidence.
#[derive(Clone)]
pub struct ResidentServiceDeploymentRevocationHeadEvidence {
    /// Immutable revocation reference.
    pub revocation_ref: ResidentRecordRefV1,
    /// Admitted revocation record.
    pub revocation: DeploymentAcceptanceRevocationV1,
}

/// Current deployment-acceptance revocation frontier.
#[derive(Clone)]
pub struct ResidentServiceDeploymentRevocationEvidence {
    /// Current deployment-acceptance revocation epoch.
    pub epoch: DeploymentAcceptanceRevocationEpochV1,
    /// Current revocation for the selected binding, when one exists.
    pub binding_head: Option<ResidentServiceDeploymentRevocationHeadEvidence>,
}

/// Current principal-scoped session-revocation frontier.
#[derive(Clone)]
pub struct ResidentServiceSessionRevocationEvidence {
    /// Current principal revocation epoch.
    pub epoch: PrincipalRevocationEpochV1,
    /// Current revocation reference, when one exists.
    pub revocation_ref: Option<ResidentRecordRefV1>,
    /// Current revocation digest, when one exists.
    pub revocation_sha256: Option<ArtifactDigest>,
}

/// Raw current authority records for one admitted peer and session selector.
#[derive(Clone)]
pub struct ResidentServiceCurrentRequestEvidence {
    /// Current capability grant evidence.
    pub grant: ResidentServiceGrantEvidence,
    /// Current capability-revocation evidence.
    pub capability_revocations: ResidentServiceCapabilityRevocationEvidence,
    /// Current deployment-revocation evidence.
    pub deployment_revocations: ResidentServiceDeploymentRevocationEvidence,
    /// Current session-revocation evidence.
    pub session_revocations: ResidentServiceSessionRevocationEvidence,
}

/// Physical reader of current authority records.
pub trait ResidentServiceCurrentRequestEvidencePort {
    /// Read exact current records; the application performs their semantic join.
    ///
    /// # Errors
    ///
    /// Returns an error when the selected evidence is missing, invalid, or
    /// cannot be read from its physical owner.
    fn read_current_request_evidence(
        &mut self,
        selector: ResidentServiceCurrentRequestSelector<'_>,
    ) -> Result<ResidentServiceCurrentRequestEvidence, InvalidResidentRecordV1>;
}

/// Caller-signature verifier selected by deployment composition.
pub trait ResidentServiceCallerSignaturePort {
    /// Establish that this verified connection key belongs to the signed caller.
    ///
    /// Native tool starts require this additional association. Kernel peer
    /// coordinates and successful verification of a replayable request do not
    /// establish it. Implementations must use their admitted caller-key records.
    /// The default refuses native starts until that owner is supplied.
    ///
    /// # Errors
    /// Returns an error when the exact caller/key association is unavailable.
    fn admit_native_caller_key(
        &mut self,
        _caller: &RequestCallerV1,
        _authenticated_key: &[u8; 32],
    ) -> Result<(), InvalidResidentRecordV1> {
        Err(InvalidResidentRecordV1::ContextualAdmission {
            record: "native tool caller",
            invariant: "authenticated connection is not bound to the signed caller",
        })
    }

    /// Verify one exact canonical common-request signature.
    /// Verify the fixed `COMMON_REQUEST_ENVELOPE_V1_SIGNATURE_DOMAIN`, including
    /// its NUL, followed by the decoded 32-byte digest under an independently
    /// authorized caller key. A connection-proof signature is insufficient.
    ///
    /// # Errors
    ///
    /// Returns an error when the signature does not verify under the selected
    /// historical caller authority.
    fn verify_caller_signature(
        &mut self,
        preimage: &CommonRequestEnvelopePreimageV1,
        digest: &CommonRequestEnvelopeDigest,
        signature: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>;
}

/// Durable replay decision for one read-only Observe request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResidentServiceObserveReplayDisposition {
    /// This exact request identity and byte sequence is new.
    FirstAdmission,
    /// The same request identity was retained previously with equal bytes.
    ExactReplay,
}

/// Why Observe replay admission could not establish one answer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResidentServiceObserveReplayError {
    /// The request identity is already bound to different signed bytes.
    ChangedBytes,
    /// Durable replay evidence is unavailable.
    Unavailable,
}

/// Durable replay admission for one exact read-only Observe request.
pub type ResidentServiceObserveReplayAdmission<'request> = dyn FnMut(
        &ResidentRequestIdV1,
        &CommonRequestEnvelopeDigest,
        &[u8],
    ) -> Result<ResidentServiceObserveReplayDisposition, ResidentServiceObserveReplayError>
    + 'request;

/// Physical inputs for one application-selected Observe request.
pub struct ResidentServiceObserveInputs<'request, 'recovery> {
    /// Exact operation recovery input selected for the requested locator.
    pub recovery: OperationControlRecoveryInput<'recovery>,
    /// Historical caller-signature verification owner.
    pub verify_caller_signature: &'request mut (dyn FnMut(
        &CommonRequestEnvelopePreimageV1,
        &CommonRequestEnvelopeDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>
                       + 'request),
    /// Durable Observe replay admission owner.
    pub admit_replay: &'request mut ResidentServiceObserveReplayAdmission<'request>,
    /// Existing historical operation-recovery admission owners.
    pub recovery_admission: OperationControlRecoveryAdmission<'recovery>,
    /// Present disclosure authorization owner.
    pub authorize_disclosure: &'recovery mut (dyn for<'context> FnMut(
        crate::resident::OperationControlAuthorizationContext<'context>,
    ) -> Result<(), InvalidResidentRecordV1>
                        + 'recovery),
    /// Deadline used to deliver the admitted response.
    pub response_deadline: Instant,
}

/// Physical recovery and disclosure owner for Observe.
pub trait ResidentServiceObserveInputPort<'recovery> {
    /// Lend exact inputs after the application has selected the locator.
    ///
    /// # Errors
    ///
    /// Returns an error when the physical recovery, replay, signing, or
    /// disclosure inputs cannot be provided for the selected locator.
    fn with_observe_inputs<Output, UseInputs>(
        &mut self,
        locator: &ControlRecordCoordinate,
        use_inputs: UseInputs,
    ) -> Result<Output, ResidentObserveFrameFailure>
    where
        UseInputs:
            for<'request> FnOnce(ResidentServiceObserveInputs<'request, 'recovery>) -> Output;
}

/// Owners retained for read-only Observe service composition.
pub struct ResidentServiceObserveOwners<
    'a,
    Evidence,
    Clock,
    SessionAuthority,
    CallerSignature,
    OperationAuthority,
    ObserveInputs,
> {
    /// Per-connection admission owner for kernel-reported local peers.
    pub peer_admission: Box<dyn ResidentPeerAdmissionPort + 'a>,
    /// Maximum caller-signed request bytes.
    pub maximum_request_bytes: usize,
    /// Generation-bound policies admitted before ingress.
    pub policies: Vec<ResidentServicePolicyProjection>,
    /// Historical generation and session resolver.
    pub historical: &'a dyn GenerationLifecycleRecoveryResolver,
    /// Current authority evidence owner.
    pub evidence: Evidence,
    /// Physical wall-clock source.
    pub clock: Clock,
    /// Current ambient-session authority owner.
    pub session_authority: SessionAuthority,
    /// Current caller-signature verifier.
    pub caller_signature: CallerSignature,
    /// Current operation authority owner used for disclosure.
    pub operation_authority: OperationAuthority,
    /// Historical operation recovery and disclosure inputs.
    pub observe_inputs: ObserveInputs,
}

/// Request policies retained by one operation-capable resident service.
///
/// The sets are separate because ingress kind is part of request
/// admission. Their signed-request domains must also remain disjoint so one
/// request cannot change meaning by moving between frame kinds.
pub struct ResidentServiceOperationRequestPolicies {
    /// Policies accepted for a new root Invoke.
    pub start: Vec<ResidentServicePolicyProjection>,
    /// Policies accepted for point-addressed Invoke continuation.
    pub advance: Vec<ResidentServicePolicyProjection>,
    /// Policies accepted for a new root Consult.
    pub start_consult: Vec<ResidentServicePolicyProjection>,
    /// Policies accepted for point-addressed Consult continuation.
    pub advance_consult: Vec<ResidentServicePolicyProjection>,
    /// Policies accepted for a new Delegate child beneath an exact parent.
    pub start_delegate: Vec<ResidentServicePolicyProjection>,
    /// Policies accepted for point-addressed Delegate continuation.
    pub advance_delegate: Vec<ResidentServicePolicyProjection>,
    /// Policies accepted for read-only Observe.
    pub observe: Vec<ResidentServicePolicyProjection>,
    /// Policies accepted for point-addressed Cancel.
    pub cancel: Vec<ResidentServicePolicyProjection>,
}

/// The application-selected source for one ordered Consult roster slot.
///
/// This value contains no operation identity. A fresh child carries the exact
/// signed request and bounded start policy from which the process owner will
/// allocate its identity beneath the parent. Existing evidence passes through
/// without creating a child.
#[derive(Clone)]
pub enum ResidentServiceConsultationRosterSource {
    /// Existing immutable evidence already satisfies this slot.
    EvidenceResolved {
        /// Exact evidence selected by the application policy.
        evidence: ImmutableRecordReferenceV1,
    },
    /// Allocate one narrower Invoke or nested Consult beneath the new parent.
    FreshChild {
        /// Exact caller-signed child request.
        request: Box<CommonRequestEnvelopeV1>,
        /// Child class fixed before identity allocation.
        operation_class: OperationClassV1,
        /// Bounded child start and lease policy.
        policy: Box<ResidentServiceOperationStartPolicy>,
    },
}

/// One ordered, identity-free slot selected for a new root Consult.
#[derive(Clone)]
pub struct ResidentServiceConsultationRosterSlot {
    /// Stable slot identity within this roster.
    pub slot_id: crate::resident::ConsultationSlotId,
    /// Whether the slot is required or optional.
    pub requirement: crate::resident::ConsultationRequirement,
    /// Evidence that established the slot requirement.
    pub requirement_evidence: ImmutableRecordReferenceV1,
    /// Participant selected by the admitted application policy.
    pub participant: ParticipantRefV1,
    /// Optional provider binding selected for this participant.
    pub provider_binding: Option<ProviderBindingRefV1>,
    /// Optional immutable context supplied to this participant.
    pub context: Option<ImmutableRecordReferenceV1>,
    /// Existing evidence or a fresh child request.
    pub source: ResidentServiceConsultationRosterSource,
}

/// Bounded, ordered meaning for one new root Consult.
///
/// This is declarative application policy. It contains no parent or child
/// identity, private start intent, custody continuation, route, or signer.
#[derive(Clone)]
pub struct ResidentServiceConsultationRosterPolicy {
    /// Immutable completion policy evaluated after each bounded mechanical turn.
    pub completion_policy: ImmutableRecordReferenceV1,
    /// Time at which collection starts.
    pub opened_at: UnixTimeV1,
    /// Last time at which ordinary collection may continue.
    pub deadline: UnixTimeV1,
    /// Ordered finite roster.
    pub slots: Vec<ResidentServiceConsultationRosterSlot>,
    /// Aggregate collection and serialized-size bounds.
    pub limits: crate::resident::ConsultationCollectionLimits,
}

/// Application-selected start policy for one root Consult.
#[derive(Clone)]
pub struct ResidentServiceConsultPolicy {
    /// Parent operation limits and publication policy.
    pub parent: ResidentServiceOperationStartPolicy,
    /// Ordered roster meaning, without allocated identities.
    pub roster: ResidentServiceConsultationRosterPolicy,
}

/// Non-identity lease limits selected by the application for one root Invoke.
#[derive(Clone)]
pub struct ResidentServiceOperationLeasePolicy {
    /// Capabilities retained by the narrower operation lease.
    pub capabilities: Vec<CapabilityIdV1>,
    /// Effect classes retained by the narrower operation lease.
    pub effects: Vec<EffectClass>,
    /// Immutable records the operation may read.
    pub read_set: Vec<ResidentRecordRefV1>,
    /// Evidence records the operation must produce.
    pub evidence_obligations: Vec<ResidentRecordRefV1>,
    /// Total admitted budget for the operation.
    pub budget_units: LeaseBudgetUnitsV1,
    /// Maximum admitted running time.
    pub timeout_millis: LeaseTimeoutMillisV1,
    /// Operation-lease expiry.
    pub expires_at: UnixTimeV1,
    /// Exact invocation correlation selected before durable start.
    pub invocation_correlation: InvocationCorrelationV1,
}

/// Application-selected limits and publication policy for one operation start.
#[derive(Clone)]
pub struct ResidentServiceOperationStartPolicy {
    /// Checkpoint leaves required for the durable operation.
    pub checkpoint_profile: crate::resident::SupportedOperationCheckpointProfile,
    /// Admitted publication policy retained with the operation.
    pub publication_policy: LocalOperationPublicationPolicy,
    /// Maximum bytes accepted for one retained resident record.
    pub maximum_record_bytes: usize,
    /// Narrower operation-lease meaning.
    pub lease: ResidentServiceOperationLeasePolicy,
    /// Public operation-handle expiry.
    pub handle_expires_at: UnixTimeV1,
    /// Last time through which operation custody must be retained.
    pub handle_retain_until: UnixTimeV1,
    /// Bounds for checkpoint accumulator preparation.
    pub accumulator_limits: AccumulatorPreparationLimits,
}

/// Admitted facts visible to the application-owned Invoke policy.
pub struct ResidentServiceInvokePolicyContext<'a> {
    /// Exact caller-signed request.
    pub request: &'a CommonRequestEnvelopeV1,
    /// Revalidated ambient session.
    pub session: &'a crate::resident::AdmittedAmbientSession,
    /// Exact operation resolved from the retained generation.
    pub operation: &'a crate::semantic::ResolvedOperation,
}

/// Select bounded Invoke policy after request and generation admission.
pub trait ResidentServiceInvokePolicyPort {
    /// Return policy for this exact request, session, and operation.
    ///
    /// # Errors
    ///
    /// Returns an error when the retained application policy does not admit
    /// this root Invoke.
    fn evaluate_invoke(
        &mut self,
        context: ResidentServiceInvokePolicyContext<'_>,
    ) -> Result<ResidentServiceOperationStartPolicy, InvalidResidentRecordV1>;
}

/// Admitted facts visible while selecting a new root Consult policy.
pub struct ResidentServiceConsultPolicyContext<'a> {
    /// Exact caller-signed parent request.
    pub request: &'a CommonRequestEnvelopeV1,
    /// Revalidated ambient session.
    pub session: &'a crate::resident::AdmittedAmbientSession,
    /// Exact parent operation resolved from the retained generation.
    pub operation: &'a crate::semantic::ResolvedOperation,
}

/// Bounded mechanical state visible to the retained Consult completion policy.
///
/// The recovered collection is application meaning plus exact point-recovery
/// evidence. It grants no custody, dispatch, signing, or mutation authority.
pub struct ResidentServiceConsultCompletionContext<'a> {
    /// Immutable Package policy selected when the collection opened.
    pub completion_policy: &'a ImmutableRecordReferenceV1,
    /// Current recovered collection head.
    pub current: &'a crate::resident::RecoveredConsultationCollection,
    /// Time used for this finite supervision turn.
    pub observed_at: UnixTimeV1,
}

/// One explicit Consult policy decision after a finite mechanical turn.
#[must_use = "apply or retain this exact Consult policy decision"]
pub enum ResidentServiceConsultDecision<StopInput> {
    /// Keep collecting and optionally offer this exact child candidate.
    Continue(Option<Box<crate::resident::ConsultationChildDispatchCandidate>>),
    /// Apply the caller-owned stop input through the existing stop owner.
    Stop(StopInput),
}

/// Select new-Consult meaning and evaluate its retained completion policy.
///
/// This port owns application policy only. It cannot allocate an identity,
/// access custody, select a signer, or inspect a private start continuation.
pub trait ResidentServiceConsultPolicyPort {
    /// Application-owned stop input consumed by the existing Consult stop owner.
    type StopInput;

    /// Return parent and ordered roster policy for this exact admitted request.
    ///
    /// # Errors
    ///
    /// Returns an error when retained application policy does not admit this
    /// root Consult or cannot select a complete finite roster.
    fn evaluate_consult(
        &mut self,
        context: ResidentServiceConsultPolicyContext<'_>,
    ) -> Result<ResidentServiceConsultPolicy, InvalidResidentRecordV1>;

    /// Apply the exact immutable completion policy to one recovered frontier.
    ///
    /// # Errors
    ///
    /// Returns an error when the retained policy is unavailable or cannot decide
    /// from this bounded collection state.
    fn evaluate_consult_completion(
        &mut self,
        context: ResidentServiceConsultCompletionContext<'_>,
    ) -> Result<ResidentServiceConsultDecision<Self::StopInput>, InvalidResidentRecordV1>;
}

/// Semantic fields selected when a Consult completion policy stops collection.
#[derive(Clone)]
pub struct ResidentServiceConsultStopMeaning {
    /// Why collection stops.
    pub cause: crate::resident::ConsultationStopCause,
    /// Immutable evidence for the stop decision.
    pub reason: ImmutableRecordReferenceV1,
    /// Time selected for the stop.
    pub stopped_at: UnixTimeV1,
}

/// Project an application stop value into the existing Consult stop meaning.
pub trait ResidentServiceConsultStopInput {
    /// Return the semantic stop fields. Physical coordinates and signers are
    /// supplied later by [`ResidentServiceConsultInputPort`].
    fn stop_meaning(&self) -> ResidentServiceConsultStopMeaning;
}

/// Positive process-local ceilings for one finite Consult turn.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResidentServiceConsultTaskLimits {
    /// Maximum work items accepted by the executor at one time.
    pub maximum_in_flight: NonZeroUsize,
    /// Maximum members in the admitted finite batch.
    pub maximum_batch_members: NonZeroUsize,
}

/// One opaque Consult task after its child dispatch reaches durable custody.
pub struct ResidentServiceConsultTask<Work> {
    inner: crate::resident::task_owner::ResidentOperationTask<Work>,
}

impl<Work> ResidentServiceConsultTask<Work> {
    /// Borrow the mechanical work without creating another executor offer.
    #[must_use]
    pub fn work(&self) -> &Work {
        self.inner.work()
    }

    /// Return this exact task with its mechanical output.
    #[must_use]
    pub fn completed<Output>(
        self,
        output: Output,
    ) -> ResidentServiceConsultTaskCompletion<Work, Output> {
        ResidentServiceConsultTaskCompletion {
            inner: crate::resident::task_owner::ResidentOperationTaskCompletion::Completed {
                task: self.inner,
                output,
            },
        }
    }

    /// Report that the accepted task was lost without a trustworthy output.
    #[must_use]
    pub fn lost<Output>(self) -> ResidentServiceConsultTaskCompletion<Work, Output> {
        ResidentServiceConsultTaskCompletion {
            inner: self.inner.into_lost(),
        }
    }
}

/// One completion returned by a Consult task executor.
pub struct ResidentServiceConsultTaskCompletion<Work, Output> {
    inner: crate::resident::task_owner::ResidentOperationTaskCompletion<Work, Output>,
}

/// Non-blocking execution of already-custodied Consult child work.
pub trait ResidentServiceConsultTaskExecutor<Work, Output> {
    /// Accept one unopened task or return it unchanged.
    ///
    /// # Errors
    /// Returns the original unopened task if executor capacity or availability
    /// prevents acceptance; no execution attempt has begun.
    fn try_accept(
        &mut self,
        task: ResidentServiceConsultTask<Work>,
    ) -> Result<(), ResidentServiceConsultTask<Work>>;

    /// Poll one completion without waiting.
    fn try_completion(&mut self) -> Option<ResidentServiceConsultTaskCompletion<Work, Output>>;
}

/// Application-owned executor adapted to the private resident task protocol.
///
/// The wrapper prevents a crate-wide blanket implementation from changing the
/// meaning of unrelated executors. A Consult input owner constructs it once and
/// lends it with the other finite-turn inputs.
pub struct ResidentServiceConsultExecutor<Executor> {
    inner: Executor,
}

impl<Executor> ResidentServiceConsultExecutor<Executor> {
    /// Wrap one application-owned non-blocking executor.
    #[must_use]
    pub const fn new(inner: Executor) -> Self {
        Self { inner }
    }

    /// Recover the application executor when the resident service is dismantled.
    #[must_use]
    pub fn into_inner(self) -> Executor {
        self.inner
    }
}

impl<Work, Output, Executor>
    crate::resident::task_owner::ResidentOperationTaskExecutor<Work, Output>
    for ResidentServiceConsultExecutor<Executor>
where
    Executor: ResidentServiceConsultTaskExecutor<Work, Output>,
{
    fn try_accept(
        &mut self,
        task: crate::resident::task_owner::ResidentOperationTask<Work>,
    ) -> Result<(), crate::resident::task_owner::ResidentOperationTask<Work>> {
        ResidentServiceConsultTaskExecutor::try_accept(
            &mut self.inner,
            ResidentServiceConsultTask { inner: task },
        )
        .map_err(|task| task.inner)
    }

    fn try_completion(
        &mut self,
    ) -> Option<crate::resident::task_owner::ResidentOperationTaskCompletion<Work, Output>> {
        ResidentServiceConsultTaskExecutor::try_completion(&mut self.inner)
            .map(|completion| completion.inner)
    }
}

/// Physical association required for one protected Consult mutation.
pub struct ResidentServiceConsultControlAssociation<'a> {
    /// Authority on the current signed request.
    pub current_request_authority: &'a SigningAuthorityBindingV1,
    /// Authority that signed the historical operation handle.
    pub historical_handle_authority: &'a SigningAuthorityBindingV1,
    /// Store coordinate proved by protected recovery.
    pub recovery_store: &'a AuthorityStoreCoordinate,
    /// Current admitted checkpoint authority.
    pub checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
}

/// Fresh inputs for the first durable collection of one exact Consult parent.
pub struct ResidentServiceConsultInitialCollectionInputs<'a> {
    /// Fresh observation reference.
    pub observation_ref: ResidentRecordRefV1,
    /// Time attached to the initial collection observation.
    pub observed_at: UnixTimeV1,
    /// Admitted observation signing context.
    pub signing: SignedResidentAdmissionContextV1<'a>,
    /// Fresh successor checkpoint input.
    pub checkpoint: crate::resident::ConsultationSuccessorCommitInput<'a>,
    /// Observation-store resolver.
    pub resolve_store: &'a dyn crate::resident::OperationObservationAuthorityStoreResolver,
    /// Admit the application-constructed collection.
    pub admit_collection: &'a mut (dyn FnMut(
        &crate::resident::ConsultationCollection,
    ) -> Result<(), InvalidResidentRecordV1>
                 + 'a),
    /// Admit an observation signing-key rotation.
    pub admit_rotation: &'a mut ResidentServiceRotationAdmission<'a>,
    /// Sign the application-constructed observation.
    pub sign_observation: &'a mut ResidentServiceObservationSigner<'a>,
    /// Sign the application-constructed checkpoint.
    pub sign_checkpoint: &'a mut ResidentServiceCheckpointSigner<'a>,
}

/// Fresh inputs for one policy-selected Consult stop.
pub struct ResidentServiceConsultStopInputs<'a> {
    /// Protected current and historical association.
    pub association: ResidentServiceConsultControlAssociation<'a>,
    /// Issued Consult parent.
    pub parent: &'a crate::resident::IssuedLocalOperation,
    /// Current parent observation.
    pub parent_observation: (&'a ResidentRecordRefV1, &'a OperationObservationV1),
    /// Current parent checkpoint.
    pub parent_checkpoint: (&'a ResidentRecordRefV1, &'a ControlStateCheckpointV1),
    /// Fresh stop observation reference.
    pub observation_ref: ResidentRecordRefV1,
    /// Time attached to the stop observation.
    pub observed_at: UnixTimeV1,
    /// Admitted observation signing context.
    pub signing: SignedResidentAdmissionContextV1<'a>,
    /// Complete collection bounds.
    pub limits: crate::resident::ConsultationCollectionLimits,
    /// Fresh successor checkpoint input.
    pub checkpoint: crate::resident::ConsultationSuccessorCommitInput<'a>,
    /// Observation-store resolver.
    pub resolve_store: &'a dyn crate::resident::OperationObservationAuthorityStoreResolver,
    /// Admit the exact stop summary.
    pub admit_stop: &'a mut (dyn FnMut(
        &crate::resident::ConsultationCollection,
        &crate::resident::ConsultationRequirementSummary,
    ) -> Result<(), InvalidResidentRecordV1>
                 + 'a),
    /// Admit the successor collection.
    pub admit_collection: &'a mut (dyn FnMut(
        &crate::resident::ConsultationCollection,
    ) -> Result<(), InvalidResidentRecordV1>
                 + 'a),
    /// Admit an observation signing-key rotation.
    pub admit_rotation: &'a mut ResidentServiceRotationAdmission<'a>,
    /// Sign the application-constructed observation.
    pub sign_observation: &'a mut ResidentServiceObservationSigner<'a>,
    /// Sign the application-constructed checkpoint.
    pub sign_checkpoint: &'a mut ResidentServiceCheckpointSigner<'a>,
}

/// Exact retained selection for one executor completion join.
pub struct ResidentServiceConsultJoinSelection<'a> {
    /// Deployment that owns the parent.
    pub deployment: ResidentServiceDeploymentSelector,
    /// Issued parent handle coordinate.
    pub parent: &'a ControlRecordCoordinate,
    /// Roster slot completed by the child.
    pub slot_id: &'a crate::resident::ConsultationSlotId,
    /// Immutable child operation reference.
    pub child: &'a ImmutableRecordReferenceV1,
    /// Immutable child dispatch reference.
    pub dispatch: &'a ImmutableRecordReferenceV1,
}

/// Fresh inputs for one executor completion join.
pub struct ResidentServiceConsultJoinInputs<'a> {
    /// Protected current and historical association.
    pub association: ResidentServiceConsultControlAssociation<'a>,
    /// Protected child and parent successor input.
    pub input: crate::resident::ConsultationChildJoinInput<'a>,
    /// Fresh successor checkpoint input.
    pub checkpoint: crate::resident::ConsultationSuccessorCommitInput<'a>,
    /// Observation-store resolver.
    pub resolve_store: &'a dyn crate::resident::OperationObservationAuthorityStoreResolver,
    /// Admit the selected child evidence.
    pub admit_evidence: &'a mut (dyn FnMut(
        &crate::resident::ConsultationRosterSlot,
        &ImmutableRecordReferenceV1,
    ) -> Result<(), InvalidResidentRecordV1>
                 + 'a),
    /// Admit the successor collection.
    pub admit_collection: &'a mut (dyn FnMut(
        &crate::resident::ConsultationCollection,
    ) -> Result<(), InvalidResidentRecordV1>
                 + 'a),
    /// Admit an observation signing-key rotation.
    pub admit_rotation: &'a mut ResidentServiceRotationAdmission<'a>,
    /// Sign the application-constructed observation.
    pub sign_observation: &'a mut ResidentServiceObservationSigner<'a>,
    /// Sign the application-constructed checkpoint.
    pub sign_checkpoint: &'a mut ResidentServiceCheckpointSigner<'a>,
}

/// Fresh inputs for one already-classified terminal Consult child.
pub struct ResidentServiceConsultTerminalChildInputs<'a> {
    /// Protected current and historical association.
    pub association: ResidentServiceConsultControlAssociation<'a>,
    /// Time attached to the child contribution.
    pub recorded_at: UnixTimeV1,
    /// Fresh parent observation reference.
    pub observation_ref: ResidentRecordRefV1,
    /// Time attached to the parent observation.
    pub observed_at: UnixTimeV1,
    /// Admitted observation signing context.
    pub signing: SignedResidentAdmissionContextV1<'a>,
    /// Complete collection bounds.
    pub limits: crate::resident::ConsultationCollectionLimits,
    /// Fresh successor checkpoint input.
    pub checkpoint: crate::resident::ConsultationSuccessorCommitInput<'a>,
    /// Observation-store resolver.
    pub resolve_store: &'a dyn crate::resident::OperationObservationAuthorityStoreResolver,
    /// Admit the selected child evidence.
    pub admit_evidence: &'a mut (dyn FnMut(
        &crate::resident::ConsultationRosterSlot,
        &ImmutableRecordReferenceV1,
    ) -> Result<(), InvalidResidentRecordV1>
                 + 'a),
    /// Admit the successor collection.
    pub admit_collection: &'a mut (dyn FnMut(
        &crate::resident::ConsultationCollection,
    ) -> Result<(), InvalidResidentRecordV1>
                 + 'a),
    /// Admit an observation signing-key rotation.
    pub admit_rotation: &'a mut ResidentServiceRotationAdmission<'a>,
    /// Sign the application-constructed observation.
    pub sign_observation: &'a mut ResidentServiceObservationSigner<'a>,
    /// Sign the application-constructed checkpoint.
    pub sign_checkpoint: &'a mut ResidentServiceCheckpointSigner<'a>,
}

/// Physical owners required to start and continue Consult operations.
pub trait ResidentServiceConsultInputPort {
    /// Mechanical work produced only after a child dispatch reaches custody.
    type Work;
    /// Mechanical output returned by the executor.
    type Output;
    /// Error returned while recovering one selected child.
    type ChildRecoveryError;
    /// Error returned while retaining one selected child dispatch.
    type RetentionFailure;
    /// Non-blocking mechanical executor implementation.
    type Executor: ResidentServiceConsultTaskExecutor<Self::Work, Self::Output>;
    /// Observation-store resolver.
    type StoreResolver: crate::resident::OperationObservationAuthorityStoreResolver;
    /// Admit a collection under current policy before continuation.
    type AdmitCollection: FnMut(
        &crate::resident::ConsultationCollection,
    ) -> Result<(), InvalidResidentRecordV1>;
    /// Protected child recovery callback.
    type RecoverChild: FnMut(
        &crate::resident::ConsultationChildDispatchCandidate,
    ) -> Result<
        crate::resident::RecoveredOperationObservation,
        Self::ChildRecoveryError,
    >;
    /// Durable child-dispatch callback.
    type RetainChild: FnMut(
        &crate::resident::ConsultationChildDispatchCandidate,
        crate::resident::RecoveredOperationObservation,
        &mut dyn OperationCustodyPort,
        &mut dyn AntiRollbackAnchorPort,
    ) -> crate::resident::ConsultationChildRetentionOutcome<
        Self::Work,
        Self::RetentionFailure,
    >;

    /// Lend bounded finite-turn inputs without selecting Consult policy.
    /// The final callback independently admits historical collections for cold
    /// opening or expiry repair. Supply `None` when that authority is unavailable; current
    /// collection admission must never substitute for historical admission.
    ///
    /// # Errors
    /// Returns an error if the bounded executor, recovery or custody owners cannot be supplied.
    fn with_finite_turn_inputs<Return, UseInputs>(
        &mut self,
        use_inputs: UseInputs,
    ) -> Result<Return, ResidentOperationFrameFailure>
    where
        UseInputs: FnOnce(
            crate::resident::ConsultationCollectionLimits,
            ResidentServiceConsultTaskLimits,
            &Self::StoreResolver,
            &mut Self::AdmitCollection,
            &mut ResidentServiceConsultExecutor<Self::Executor>,
            &mut Self::RecoverChild,
            &mut Self::RetainChild,
            Option<&mut crate::resident::HistoricalConsultationAdmission<'_>>,
        ) -> Return;

    /// Lend fresh inputs for the first collection of one retained parent.
    ///
    /// # Errors
    /// Returns an error if current signing or custody inputs cannot be supplied for this retained parent.
    fn with_initial_collection_inputs<Return, UseInputs>(
        &mut self,
        deployment: &ResidentServiceDeploymentSelector,
        parent: &ControlRecordCoordinate,
        use_inputs: UseInputs,
    ) -> Result<Return, ResidentOperationFrameFailure>
    where
        UseInputs:
            for<'input> FnOnce(ResidentServiceConsultInitialCollectionInputs<'input>) -> Return;

    /// Lend fresh inputs for one exact policy-selected stop.
    ///
    /// # Errors
    /// Returns an error if the selected stop cannot obtain current authority or exact parent custody inputs.
    fn with_stop_inputs<Return, UseInputs>(
        &mut self,
        deployment: &ResidentServiceDeploymentSelector,
        parent: &ControlRecordCoordinate,
        meaning: &ResidentServiceConsultStopMeaning,
        use_inputs: UseInputs,
    ) -> Result<Return, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(ResidentServiceConsultStopInputs<'input>) -> Return;

    /// Lend fresh inputs for one exact executor completion join.
    ///
    /// # Errors
    /// Returns an error if the selected completion cannot obtain its current child and parent custody inputs.
    fn with_completion_join_inputs<Return, UseInputs>(
        &mut self,
        selection: ResidentServiceConsultJoinSelection<'_>,
        use_inputs: UseInputs,
    ) -> Result<Return, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(ResidentServiceConsultJoinInputs<'input>) -> Return;

    /// Lend fresh inputs for one already-classified terminal child join.
    ///
    /// # Errors
    /// Returns an error if the terminal child cannot obtain current evidence admission or parent signing inputs.
    fn with_terminal_child_join_inputs<Return, UseInputs>(
        &mut self,
        use_inputs: UseInputs,
    ) -> Result<Return, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(ResidentServiceConsultTerminalChildInputs<'input>) -> Return;
}

/// Admitted facts visible to the application-owned Delegate policy.
pub struct ResidentServiceDelegatePolicyContext<'a> {
    /// Exact caller-signed child request.
    pub request: &'a CommonRequestEnvelopeV1,
    /// Revalidated ambient session.
    pub session: &'a crate::resident::AdmittedAmbientSession,
    /// Exact child operation resolved from the retained generation.
    pub operation: &'a crate::semantic::ResolvedOperation,
    /// Protected parent beneath which the child will be allocated.
    pub parent: &'a crate::resident::PreparedLocalChildParent,
}

/// Select bounded Delegate policy after request, generation, and parent admission.
pub trait ResidentServiceDelegatePolicyPort {
    /// Return policy for this exact child request and protected parent.
    ///
    /// # Errors
    ///
    /// Returns an error when retained application policy does not admit this
    /// Delegate child or its authority is not narrower than the parent.
    fn evaluate_delegate(
        &mut self,
        context: ResidentServiceDelegatePolicyContext<'_>,
    ) -> Result<ResidentServiceOperationStartPolicy, InvalidResidentRecordV1>;
}

/// Carrier-independent fields for one admitted pairwise A2A launch.
///
/// The application injects the already-retained provider dispatch. This plan
/// supplies only the exact admitted route, request, evidence, checkpoint, and
/// finite bounds needed to construct one A2A request. It cannot replay a send.
pub struct ResidentServiceA2aDelegateLaunchPlan<'a> {
    /// Current admitted session and generation authority.
    pub current: &'a crate::resident::AmbientSessionAdmissionContext<'a>,
    /// Exact route admission selected for this child.
    pub route: (&'a ResidentRecordRefV1, &'a A2aRouteAdmissionV1),
    /// Expected predecessor of the private preparation index.
    pub route_predecessor: OperationCustodyIndexPredecessor,
    /// Required route-index mutation.
    pub route_association: crate::resident::A2aRouteAssociation,
    /// One exact JSON-RPC request identity.
    pub request_id: &'a A2aJsonRpcRequestIdV1,
    /// Stable A2A message identity.
    pub message_id: &'a str,
    /// Bounded text sent to the admitted remote participant.
    pub text: &'a str,
    /// Fresh private request-evidence reference.
    pub request_evidence_ref: ResidentRecordRefV1,
    /// Fresh private preparation reference.
    pub preparation_ref: ResidentRecordRefV1,
    /// Mechanical exchange ceilings.
    pub exchange_limits: A2aExchangeLimits,
    /// Maximum canonical request-evidence bytes.
    pub maximum_evidence_bytes: usize,
    /// Maximum protected record bytes.
    pub maximum_record_bytes: usize,
    /// Maximum provider-dispatch record bytes.
    pub maximum_dispatch_record_bytes: usize,
    /// Existing owner of the selected observation store.
    pub resolve_store: &'a dyn crate::resident::OperationObservationAuthorityStoreResolver,
    /// Fresh checkpoint identity for request custody.
    pub checkpoint: crate::resident::OperationCheckpointInput,
    /// Checkpoint schema admitted by the deployment.
    pub checkpoint_schema: &'a SchemaDigest,
    /// Checkpoint signing authority admitted by the deployment.
    pub checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    /// Fresh response-evidence reference.
    pub response_ref: ResidentRecordRefV1,
    /// Selected physical time source.
    pub clock: &'a dyn WallClockSourcePort,
    /// Finite checkpoint-accumulator bounds.
    pub accumulator_limits: AccumulatorPreparationLimits,
    /// Finite checkpoint-membership bounds.
    pub membership_limits: AccumulatorMembershipLimits,
}

/// One-shot inputs for admitted resolution and initial pairwise A2A launch.
///
/// Every callable authority is borrowed for the resident service lifetime. A
/// launch failure can therefore remain in the service without shortening its
/// custody to one frame call.
pub struct ResidentServiceA2aDelegateLaunchInputs<'a, Authority> {
    /// Current operation authority owner.
    pub authority: &'a mut Authority,
    /// Exact provider dispatch selected for this Delegate child.
    pub dispatch: crate::resident::ProviderOperationDispatchInput<'a, 'a>,
    /// Carrier-independent request construction inputs.
    pub plan: ResidentServiceA2aDelegateLaunchPlan<'a>,
    /// Mechanical A2A exchange. The application retains request custody first.
    pub exchange: &'a dyn A2aExchangePort,
    /// Admit historical observation-signing rotation.
    pub admit_rotation: &'a mut ResidentServiceRotationAdmission<'a>,
    /// Sign the provider-dispatch observation.
    pub sign_observation: &'a mut ResidentServiceObservationSigner<'a>,
    /// Sign the provider-dispatch checkpoint.
    pub sign_provider_checkpoint: &'a mut ResidentServiceCheckpointSigner<'a>,
    /// Admit disclosure for this exact route and payload.
    pub admit_disclosure: &'a mut (dyn FnMut(
        &ProviderInvocationRequest<'_>,
        &A2aRouteAdmissionV1,
        &str,
    ) -> Result<(), InvalidResidentRecordV1>
                 + 'a),
    /// Admit the route transport selected for this exact provider request.
    pub admit_transport: Box<ResidentServiceTransportAdmission<'a>>,
    /// Sign the A2A request-custody checkpoint.
    pub sign_a2a_checkpoint: Box<ResidentServiceOneShotCheckpointSigner<'a>>,
}

/// Inputs for restart recovery before a remote A2A Task is identified.
///
/// This value intentionally has no exchange port. It can only prove the
/// retained send and store explicit outcome uncertainty.
pub struct ResidentServiceA2aPreTaskRecoveryInputs<'a, Authority> {
    /// Current operation authority owner.
    pub authority: &'a mut Authority,
    /// Exact historical send and finite recovery limits.
    pub recovery: crate::resident::A2aSendRecoveryInput<'a>,
    /// Historical checkpoint and route admissions.
    pub admission: crate::resident::A2aSendRecoveryAdmission<'a>,
    /// Build the only permitted durable successor from the recovered send.
    pub retain: Box<
        dyn FnOnce(
                Box<crate::resident::RecoveredA2aSendOutcomeUnknown<'a>>,
            ) -> crate::resident::A2aRecoveredUnknownOutcomeRetention<'a>
            + 'a,
    >,
}

/// Inputs for read-only recovery of one exact identified A2A Task.
///
/// This path classifies existing `GetTask` custody only. It does not create a
/// fresh request, call a carrier, or infer a Pale result from remote state.
pub struct ResidentServiceA2aTaskObservationInputs<'a, Authority> {
    /// Current operation authority owner.
    pub authority: &'a mut Authority,
    /// Maximum bytes accepted from protected operation records.
    pub maximum_record_bytes: usize,
}

/// Inputs for retained `CancelTask` uncertainty recovery.
///
/// No exchange port or request builder is present, so the application cannot
/// issue a second remote cancellation request.
pub struct ResidentServiceA2aCancelRecoveryInputs<'a> {
    /// Existing local disposition selected from retained cancellation custody.
    pub disposition: crate::resident::A2aCancelTaskDispositionInput<'a>,
    /// Fresh checkpoint identity for the uncertainty successor.
    pub checkpoint: crate::resident::OperationCheckpointInput,
    /// Time at which the successor checkpoint is issued.
    pub checkpoint_issued_at: UnixTimeV1,
    /// Checkpoint schema admitted by the deployment.
    pub checkpoint_schema: &'a SchemaDigest,
    /// Checkpoint signing authority admitted by the deployment.
    pub checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    /// Finite checkpoint-accumulator bounds.
    pub accumulator_limits: AccumulatorPreparationLimits,
    /// Maximum bytes accepted from protected operation records.
    pub maximum_record_bytes: usize,
    /// Existing owner of the selected observation store.
    pub resolve_store: &'a dyn crate::resident::OperationObservationAuthorityStoreResolver,
    /// Admit the reconciliation frontier used for explicit uncertainty.
    pub admit_reconciliation: Box<ResidentServiceReconciliationAdmission<'a>>,
    /// Admit historical observation-signing rotation.
    pub admit_rotation: Box<ResidentServiceOneShotRotationAdmission<'a>>,
    /// Sign the application-constructed uncertainty observation.
    pub sign_observation: Box<ResidentServiceOneShotObservationSigner<'a>>,
    /// Sign the application-constructed successor checkpoint.
    pub sign_checkpoint: Box<ResidentServiceOneShotCheckpointSigner<'a>>,
}

/// Physical one-shot inputs for every pairwise Delegate recovery phase.
///
/// The application selects the phase from durable evidence before calling one
/// method. Implementations provide physical owners only for that phase. The
/// pre-Task and `CancelTask` methods expose no carrier, which prevents replay of
/// `SendMessage` and `CancelTask` after an uncertain outcome.
pub trait ResidentServiceDelegateInputPort<'service>: Sized + 'service {
    /// Current operation authority used by the existing process-host owners.
    type Authority: AmbientSessionAuthorityPort + 'service;

    /// Lend fresh identities and custody owners before an A2A launch.
    ///
    /// The default fails closed. The callback receives no A2A exchange and
    /// therefore cannot prepare or send `SendMessage`.
    ///
    /// # Errors
    /// Returns an error when expiry preparation inputs are unavailable. The default returns `ConfigurationUnavailable` without calling the callback.
    fn with_pre_dispatch_expiry_preparation_inputs<Output, UseInputs>(
        &mut self,
        _deployment: &ResidentServiceDeploymentSelector,
        _locator: &ResidentOperationFrameLocator,
        _use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs:
            for<'input> FnOnce(ResidentServicePreDispatchExpiryPreparationInputs<'input>) -> Output,
    {
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    /// Lend only the custody inputs required by a retained expiry continuation.
    ///
    /// The default fails closed. The callback receives no clock, fresh record
    /// identity, observation signer, or A2A exchange.
    ///
    /// # Errors
    /// Returns an error when the retained custody phase cannot obtain its inputs. The default returns `ConfigurationUnavailable` without calling the callback.
    fn with_pre_dispatch_expiry_continuation_inputs<Output, UseInputs>(
        &mut self,
        _deployment: &ResidentServiceDeploymentSelector,
        _locator: &ResidentOperationFrameLocator,
        _use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(
            ResidentServicePreDispatchExpiryContinuationInputs<'input>,
        ) -> Output,
    {
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    /// Lend one exact admitted launch and its one-shot carrier.
    ///
    /// # Errors
    /// Returns an error if the selected launch lacks its admitted route, authority or one-shot carrier.
    fn with_initial_launch_inputs<Output, UseInputs>(
        &mut self,
        deployment: &ResidentServiceDeploymentSelector,
        locator: &ResidentOperationFrameLocator,
        use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs:
            FnOnce(ResidentServiceA2aDelegateLaunchInputs<'service, Self::Authority>) -> Output;

    /// Lend recovery inputs for a retained send with no identified remote Task.
    ///
    /// # Errors
    /// Returns an error if the exact retained send cannot obtain its historical evidence and current authority.
    fn with_pre_task_recovery_inputs<Output, UseInputs>(
        &mut self,
        deployment: &ResidentServiceDeploymentSelector,
        locator: &ResidentOperationFrameLocator,
        use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs:
            FnOnce(ResidentServiceA2aPreTaskRecoveryInputs<'service, Self::Authority>) -> Output;

    /// Lend bounded read-only inputs for one exact identified remote Task.
    ///
    /// # Errors
    /// Returns an error if the identified remote task cannot obtain its protected records and current authority.
    fn with_task_observation_inputs<Output, UseInputs>(
        &mut self,
        deployment: &ResidentServiceDeploymentSelector,
        locator: &ResidentOperationFrameLocator,
        use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs:
            FnOnce(ResidentServiceA2aTaskObservationInputs<'service, Self::Authority>) -> Output;

    /// Lend recovery-only inputs for one retained remote cancellation request.
    ///
    /// # Errors
    /// Returns an error if the retained cancellation lacks its reconciliation, signing or custody inputs.
    fn with_cancel_recovery_inputs<Output, UseInputs>(
        &mut self,
        deployment: &ResidentServiceDeploymentSelector,
        locator: &ResidentOperationFrameLocator,
        use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: FnOnce(ResidentServiceA2aCancelRecoveryInputs<'service>) -> Output;

    /// Poll one already-started response without issuing another `SendMessage`.
    ///
    /// The input owner creates one fresh polling context for this call. The
    /// callback receives only the evidence protocol and clock needed to poll
    /// the retained exchange. It receives no request builder or send carrier.
    fn with_response_poll_inputs<Output, UseInputs>(
        self,
        _: &ResidentServiceDeploymentSelector,
        _: &ResidentOperationFrameLocator,
        _: UseInputs,
    ) -> (Self, Result<Output, ResidentOperationFrameFailure>)
    where
        UseInputs: for<'poll, 'context> FnOnce(
            &'poll mut Context<'context>,
            &'poll dyn A2aSendMessageEvidencePort,
            &'poll dyn WallClockSourcePort,
        ) -> Output,
    {
        (
            self,
            Err(ResidentOperationFrameFailure::ConfigurationUnavailable),
        )
    }

    /// Lend durable inputs for a response that identified a remote Task.
    fn with_task_outcome_inputs<Output, UseInputs>(
        self,
        _: &ResidentServiceDeploymentSelector,
        _: &ResidentOperationFrameLocator,
        response: Box<crate::resident::CompletedA2aTaskResponse>,
        _: UseInputs,
    ) -> (
        Self,
        Result<
            Output,
            (
                ResidentOperationFrameFailure,
                Box<crate::resident::CompletedA2aTaskResponse>,
            ),
        >,
    )
    where
        UseInputs: FnOnce(crate::resident::A2aTaskOutcomeRetention<'service>) -> Output,
    {
        (
            self,
            Err((
                ResidentOperationFrameFailure::ConfigurationUnavailable,
                response,
            )),
        )
    }

    /// Lend durable inputs for a valid response that created no remote Task.
    fn with_no_task_outcome_inputs<Output, UseInputs>(
        self,
        _: &ResidentServiceDeploymentSelector,
        _: &ResidentOperationFrameLocator,
        response: Box<crate::resident::CompletedA2aNoTaskResponse>,
        _: UseInputs,
    ) -> (
        Self,
        Result<
            Output,
            (
                ResidentOperationFrameFailure,
                Box<crate::resident::CompletedA2aNoTaskResponse>,
            ),
        >,
    )
    where
        UseInputs: FnOnce(crate::resident::A2aNoTaskOutcomeRetention<'service>) -> Output,
    {
        (
            self,
            Err((
                ResidentOperationFrameFailure::ConfigurationUnavailable,
                response,
            )),
        )
    }

    /// Lend durable inputs for a request rejected before remote acceptance.
    fn with_not_started_outcome_inputs<Output, UseInputs>(
        self,
        _: &ResidentServiceDeploymentSelector,
        _: &ResidentOperationFrameLocator,
        response: Box<crate::resident::A2aSendNotStarted>,
        _: UseInputs,
    ) -> (
        Self,
        Result<
            Output,
            (
                ResidentOperationFrameFailure,
                Box<crate::resident::A2aSendNotStarted>,
            ),
        >,
    )
    where
        UseInputs: FnOnce(crate::resident::A2aNotStartedOutcomeRetention<'service>) -> Output,
    {
        (
            self,
            Err((
                ResidentOperationFrameFailure::ConfigurationUnavailable,
                response,
            )),
        )
    }

    /// Lend durable inputs for an accepted send with an unknown remote outcome.
    fn with_unknown_outcome_inputs<Output, UseInputs>(
        self,
        _: &ResidentServiceDeploymentSelector,
        _: &ResidentOperationFrameLocator,
        outcome: Box<crate::resident::A2aSendOutcomeUnknown>,
        _: UseInputs,
    ) -> (
        Self,
        Result<
            Output,
            (
                ResidentOperationFrameFailure,
                Box<crate::resident::A2aSendOutcomeUnknown>,
            ),
        >,
    )
    where
        UseInputs: FnOnce(crate::resident::A2aUnknownOutcomeRetention<'service>) -> Output,
    {
        (
            self,
            Err((
                ResidentOperationFrameFailure::ConfigurationUnavailable,
                outcome,
            )),
        )
    }
}

/// Physical signing and checkpoint construction for a durable Invoke start.
pub trait ResidentServiceInvokeCryptographyPort {
    /// Admit the compiled operation against the exact retained authority.
    #[allow(clippy::too_many_arguments)]
    ///
    /// # Errors
    /// Returns an error if the compiled operation, session, lease, publication or checkpoint authority does not admit the exact request and handle.
    fn admit_compiled_authority(
        &self,
        request: &CommonRequestEnvelopeV1,
        session: &crate::resident::AdmittedAmbientSession,
        current: &crate::resident::AmbientSessionAdmissionContext<'_>,
        handle: &OperationHandlePreimageV1,
        lease: &OperationLeaseV1,
        operation: &crate::semantic::ResolvedOperation,
        publication: &crate::resident::LocalOperationPublicationPolicyInput<'_>,
        checkpoint: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    ) -> Result<(), InvalidResidentRecordV1>;

    /// Sign the exact application-constructed handle.
    ///
    /// Successful return does not make the signature durable in operation custody.
    /// This port has no issuance-retrieval contract: an absent handle after process
    /// loss does not authorize a second signing call.
    ///
    /// # Errors
    /// Returns an error if the signer rejects the handle, its signing purpose or its admitted authority, or cannot produce a signature.
    fn sign_handle(
        &self,
        bytes: &[u8],
        preimage: &OperationHandlePreimageV1,
        digest: &OperationHandleDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>;

    /// Sign the exact application-constructed initial observation.
    ///
    /// # Errors
    /// Returns an error if the signer rejects the observation, its signing purpose or its admitted authority, or cannot produce a signature.
    fn sign_observation(
        &self,
        bytes: &[u8],
        preimage: &OperationObservationPreimageV1,
        digest: &OperationObservationDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>;

    /// Construct the checkpoint preimage for the prepared start.
    ///
    /// # Errors
    /// Returns an error if the prepared roots, domain or allocated checkpoint identity cannot form the selected checkpoint preimage.
    fn prepare_checkpoint(
        &self,
        prepared: &crate::resident::PreparedLocalOperationCheckpoint,
        checkpoint_id: &ControlCheckpointIdV1,
    ) -> Result<ControlStateCheckpointPreimageV1, crate::resident::LocalOperationStartError>;

    /// Sign the exact application-constructed control checkpoint.
    ///
    /// # Errors
    /// Returns an error if the signer rejects the checkpoint, its signing purpose or its admitted authority, or cannot produce a signature.
    fn sign_checkpoint(
        &self,
        bytes: &[u8],
        preimage: &ControlStateCheckpointPreimageV1,
        digest: &ControlStateCheckpointDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>;
}

/// Bounds used while classifying one existing Invoke head.
#[derive(Clone, Copy)]
pub struct ResidentServiceInvokeRecoveryLimits {
    /// Observation-history bounds.
    pub observation: OperationObservationRecoveryLimits,
    /// Checkpoint-ancestry bounds.
    pub checkpoints: OperationCheckpointAncestryLimits,
    /// Accumulator-membership bounds.
    pub membership: AccumulatorMembershipLimits,
    /// Maximum bytes accepted for a retained implementation selection.
    pub maximum_selection_bytes: usize,
}

/// Physical and historical inputs for one selected Invoke request.
pub struct ResidentServiceInvokeRecoveryInputs<'operation, 'admission, 'checkpoint> {
    /// Exact deployment and historical recovery context.
    pub operation: LocalOperationRecoveryContext<'operation>,
    /// Maximum parent depth accepted during protected recovery.
    pub maximum_parent_depth: usize,
    /// Bounded current-head reads.
    pub limits: ResidentServiceInvokeRecoveryLimits,
    /// Existing historical record and signature admissions.
    pub recovery_admission: OperationControlRecoveryAdmission<'admission>,
    /// Admitted checkpoint-signing authority for a new start.
    pub checkpoint_authority:
        &'checkpoint ControlCheckpointSigningAuthorityAdmissionContextV1<'checkpoint>,
}

/// Physical inputs for one bounded cold-recovery turn.
pub struct ResidentServiceInvokeBackgroundRecoveryInputs<'operation, 'admission> {
    /// Work ceilings for reconstructing a committed expiry or initial Consult collection.
    pub committed_observation_limits: AccumulatorPreparationLimits,
    /// Exact deployment and historical recovery context for the selected source.
    pub operation: LocalOperationRecoveryContext<'operation>,
    /// Maximum parent depth accepted during protected recovery.
    pub maximum_parent_depth: usize,
    /// Bounded current-head reads.
    pub limits: ResidentServiceInvokeRecoveryLimits,
    /// Existing historical record and signature admissions.
    pub recovery_admission: OperationControlRecoveryAdmission<'admission>,
    /// Maximum bytes accepted for the recovered ambient-session basis.
    pub maximum_session_basis_bytes: usize,
    /// Physical time observed for this recovery turn.
    pub observed_at: UnixTimeV1,
}

/// Fresh custody inputs for one retained provider dispatch after restart.
///
/// The process host already proved that the provider call crossed its dispatch
/// boundary. These inputs can record only explicit outcome uncertainty; they do
/// not expose a provider port and cannot replay the call.
pub struct ResidentServiceProviderRecoveryInputs<'a> {
    /// Fresh observation, checkpoint, diagnostic, and custody inputs.
    pub recording: crate::resident::ProviderOperationInvokeInput<'a>,
    /// Maximum bytes accepted for the recovered implementation selection.
    pub maximum_selection_bytes: usize,
    /// Admit the exact reconciliation frontier used by the uncertainty result.
    pub admit_reconciliation: &'a mut (dyn FnMut(&ReconciliationFrontierReferenceV1) -> Result<(), InvalidResidentRecordV1>
                 + 'a),
    /// Admit an exact observation signing-key rotation.
    pub admit_rotation: &'a mut ResidentServiceRotationAdmission<'a>,
    /// Sign the application-constructed uncertainty observation.
    pub sign_observation: &'a mut ResidentServiceObservationSigner<'a>,
    /// Sign the application-constructed successor checkpoint.
    pub sign_checkpoint: &'a mut ResidentServiceCheckpointSigner<'a>,
}

/// Fresh physical inputs for one retained built-in Invoke turn.
///
/// The process host retains the recovered dispatch or its exact post-handler
/// recording continuation. This input supplies only current authority,
/// bounded recording identities, and existing physical owners. It never
/// exposes the native handler or grants replay authority.
pub struct ResidentServiceNativeRecoveryInputs<'a> {
    /// Current admitted session authority.
    pub current: &'a crate::resident::AmbientSessionAdmissionContext<'a>,
    /// Existing native action registry selected by the deployment.
    pub registry: &'a crate::actions::NativeActionRegistry,
    /// Contract set admitted for native result validation.
    pub contract_set: &'a ContractSetV1,
    /// Fresh native-outcome record reference.
    pub outcome_ref: ResidentRecordRefV1,
    /// Fresh staged-outcome observation reference.
    pub outcome_observation_ref: ResidentRecordRefV1,
    /// Fresh successful-result reference.
    pub result_ref: ResidentRecordRefV1,
    /// Fresh failed-handler diagnostic reference.
    pub diagnostic_ref: ResidentRecordRefV1,
    /// Fresh terminal observation reference.
    pub terminal_ref: ResidentRecordRefV1,
    /// Checkpoint identity for the staged-outcome append.
    pub outcome_checkpoint: crate::resident::OperationCheckpointInput,
    /// Checkpoint identity for the terminal append.
    pub terminal_checkpoint: crate::resident::OperationCheckpointInput,
    /// Selected record-time source.
    pub clock: &'a dyn WallClockSourcePort,
    /// Existing owner of the selected observation store.
    pub resolve_store: &'a dyn crate::resident::OperationObservationAuthorityStoreResolver,
    /// Independently admitted checkpoint schema.
    pub checkpoint_schema: &'a SchemaDigest,
    /// Resolved checkpoint signer.
    pub checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    /// Resolved observation signer.
    pub signing: SignedResidentAdmissionContextV1<'a>,
    /// Maximum bytes accepted for the retained implementation selection.
    pub maximum_selection_bytes: usize,
    /// Maximum bytes accepted for protected recovery records.
    pub maximum_record_bytes: usize,
    /// Maximum canonical diagnostic bytes.
    pub maximum_diagnostic_bytes: usize,
    /// Maximum private staged-outcome bytes.
    pub maximum_outcome_bytes: usize,
    /// Existing accumulator preparation bounds.
    pub accumulator_limits: AccumulatorPreparationLimits,
    /// Admit an exact observation signing-key rotation.
    pub admit_rotation: &'a mut ResidentServiceRotationAdmission<'a>,
    /// Sign an application-constructed observation.
    pub sign_observation: &'a mut ResidentServiceObservationSigner<'a>,
    /// Sign an application-constructed checkpoint.
    pub sign_checkpoint: &'a mut ResidentServiceCheckpointSigner<'a>,
    /// Sign an application-constructed successful result.
    pub sign_result: &'a mut ResidentServiceResultSigner<'a>,
}

/// Physical implementation offered for one retained `Resolving` Invoke.
///
/// The existing application owners still validate the request against the
/// compiled generation and construct the private selection. This value grants
/// neither execution nor provider-call authority.
pub enum ResidentServiceOperationResolutionRoute<'a> {
    /// One built-in Rust action offered by the selected deployment.
    Native {
        /// Existing sealed native-action registry.
        registry: &'a crate::actions::NativeActionRegistry,
        /// Fresh attempt identity.
        attempt_id: &'a CapabilityAttemptIdV1,
        /// Result admission selected for the exact native action.
        result_admission: CapabilitySuccessAdmissionContextV1<'a>,
    },
    /// One external-provider selection offered by the selected deployment.
    Provider {
        /// Exact provider request proposed for the retained operation.
        invocation: ProviderInvocationRequest<'a>,
        /// Provider result admission for this exact request.
        result_admission: crate::provider_invocation::ProviderInvocationAdmissionContext<'a>,
        /// Fresh reference for the `ProviderResolved` observation.
        resolution_ref: ResidentRecordRefV1,
    },
}

/// Fresh physical inputs for one retained operation-resolution turn.
pub struct ResidentServiceOperationResolutionInputs<'a> {
    /// Historical ambient session retained by the operation.
    pub session: &'a crate::resident::AdmittedAmbientSession,
    /// Current admitted session authority.
    pub current: &'a crate::resident::AmbientSessionAdmissionContext<'a>,
    /// Physical implementation offered to the existing semantic owner.
    pub route: ResidentServiceOperationResolutionRoute<'a>,
    /// Observation signing authority.
    pub signing: SignedResidentAdmissionContextV1<'a>,
    /// Fresh private-selection reference.
    pub selection_ref: ResidentRecordRefV1,
    /// Fresh observation reference for native preparation or later provider preparation.
    pub observation_ref: ResidentRecordRefV1,
    /// Fresh identity for the immediate successor checkpoint.
    pub checkpoint: crate::resident::OperationCheckpointInput,
    /// Checkpoint schema admitted by the deployment.
    pub checkpoint_schema: &'a SchemaDigest,
    /// Checkpoint signing authority admitted by the deployment.
    pub checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    /// Observation-store resolver.
    pub resolve_store: &'a dyn crate::resident::OperationObservationAuthorityStoreResolver,
    /// Bounds for accumulator preparation.
    pub accumulator_limits: AccumulatorPreparationLimits,
    /// Maximum retained implementation-selection bytes.
    pub maximum_selection_bytes: usize,
    /// Maximum bytes read for protected recovery records.
    pub maximum_record_bytes: usize,
    /// Admit an exact observation signing-key rotation.
    pub admit_rotation: &'a mut ResidentServiceRotationAdmission<'a>,
    /// Sign the application-constructed successor observation.
    pub sign_observation: &'a mut ResidentServiceObservationSigner<'a>,
    /// Sign the application-constructed successor checkpoint.
    pub sign_checkpoint: &'a mut ResidentServiceCheckpointSigner<'a>,
}

/// Fresh authority and custody inputs for one retained provider resolution.
///
/// The input owner supplies physical identities, signers, and the result
/// admission already selected for this provider binding. The process host
/// still allocates the attempt and constructs its private dispatch selection.
pub struct ResidentServiceProviderResolvedInputs<'a> {
    /// Historical ambient session retained by the operation.
    pub session: &'a crate::resident::AdmittedAmbientSession,
    /// Current admitted session authority.
    pub current: &'a crate::resident::AmbientSessionAdmissionContext<'a>,
    /// Provider implementation kind selected by the application.
    pub implementation_kind: &'a BoundedResidentTextV1,
    /// Provider implementation version selected by the application.
    pub implementation_version: &'a BoundedResidentTextV1,
    /// Expected success schema.
    pub result_schema: &'a SchemaDigest,
    /// Authority allowed to sign the provider result.
    pub result_authority: &'a SigningAuthorityBindingV1,
    /// Contract set used to admit a successful result.
    pub contract_set: &'a ContractSetV1,
    /// Expected safe-failure schema.
    pub failure_schema: &'a SchemaDigest,
    /// Authority allowed to sign a safe failure.
    pub failure_authority: &'a SigningAuthorityBindingV1,
    /// Exact safe-failure proof selected for this attempt.
    pub safe_failure_proof: crate::provider_invocation::ProviderInvocationSafeFailureProof,
    /// Observation signing authority.
    pub signing: SignedResidentAdmissionContextV1<'a>,
    /// Checkpoint schema admitted by the deployment.
    pub checkpoint_schema: &'a SchemaDigest,
    /// Checkpoint signing authority admitted by the deployment.
    pub checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    /// Observation-store resolver.
    pub resolve_store: &'a dyn crate::resident::OperationObservationAuthorityStoreResolver,
    /// Bounds for accumulator preparation.
    pub accumulator_limits: AccumulatorPreparationLimits,
    /// Maximum retained provider-selection bytes.
    pub maximum_selection_bytes: usize,
    /// Maximum bytes read for protected recovery records.
    pub maximum_record_bytes: usize,
    /// Cryptographic random source used for the fresh attempt identities.
    pub random: &'a dyn CryptographicRandomSourcePort,
    /// Admit an exact observation signing-key rotation.
    pub admit_rotation: &'a mut ResidentServiceRotationAdmission<'a>,
    /// Sign the application-constructed attempt observation.
    pub sign_observation: &'a mut ResidentServiceObservationSigner<'a>,
    /// Sign the application-constructed checkpoint.
    pub sign_checkpoint: &'a mut ResidentServiceCheckpointSigner<'a>,
}

/// One opaque provider task after every application check and durable dispatch.
///
/// Consuming [`Self::execute`] or [`Self::lost`] prevents a host from using the
/// wrapper to submit the same accepted call twice. No selection, signer,
/// custody, or replay authority is exposed.
pub struct ResidentServiceProviderTask {
    inner: Box<
        crate::resident::task_owner::ResidentOperationTask<
            crate::resident::operation_dispatch::OwnedProviderInvocationRequest,
        >,
    >,
}

impl ResidentServiceProviderTask {
    /// Execute this one task through the supplied mechanical provider adapter.
    #[must_use]
    pub fn execute(
        self,
        provider: &dyn ProviderInvocationPort,
    ) -> ResidentServiceProviderTaskCompletion {
        let output = self
            .inner
            .work()
            .borrowed()
            .map(|request| provider.invoke(request));
        ResidentServiceProviderTaskCompletion {
            inner: crate::resident::task_owner::ResidentOperationTaskCompletion::Completed {
                task: *self.inner,
                output,
            },
        }
    }

    /// Report that the accepted mechanical task was lost without an outcome.
    #[must_use]
    pub fn lost(self) -> ResidentServiceProviderTaskCompletion {
        ResidentServiceProviderTaskCompletion {
            inner: self.inner.into_lost(),
        }
    }
}

/// Opaque completion of one task previously accepted by the host executor.
pub struct ResidentServiceProviderTaskCompletion {
    inner: crate::resident::task_owner::ResidentOperationTaskCompletion<
        crate::resident::operation_dispatch::OwnedProviderInvocationRequest,
        Result<
            ProviderInvocationObservation,
            crate::provider_invocation::ProviderInvocationAdmissionError,
        >,
    >,
}

/// Non-blocking host owner for provider task acceptance and completion.
pub trait ResidentServiceProviderTaskExecutor {
    /// Accept the only ordinary execution attempt, or return it unopened.
    ///
    /// # Errors
    /// Returns the original unopened task if executor capacity or availability
    /// prevents acceptance; no execution attempt has begun.
    fn try_accept(
        &mut self,
        task: ResidentServiceProviderTask,
    ) -> Result<(), ResidentServiceProviderTask>;

    /// Poll one completion without waiting.
    fn try_completion(&mut self) -> Option<ResidentServiceProviderTaskCompletion>;
}

struct ResidentServiceProviderTaskExecutorAdapter<'a> {
    executor: &'a mut dyn ResidentServiceProviderTaskExecutor,
}

impl
    crate::resident::task_owner::ResidentOperationTaskExecutor<
        crate::resident::operation_dispatch::OwnedProviderInvocationRequest,
        Result<
            ProviderInvocationObservation,
            crate::provider_invocation::ProviderInvocationAdmissionError,
        >,
    > for ResidentServiceProviderTaskExecutorAdapter<'_>
{
    fn try_accept(
        &mut self,
        task: crate::resident::task_owner::ResidentOperationTask<
            crate::resident::operation_dispatch::OwnedProviderInvocationRequest,
        >,
    ) -> Result<
        (),
        crate::resident::task_owner::ResidentOperationTask<
            crate::resident::operation_dispatch::OwnedProviderInvocationRequest,
        >,
    > {
        self.executor
            .try_accept(ResidentServiceProviderTask {
                inner: Box::new(task),
            })
            .map_err(|task| *task.inner)
    }

    fn try_completion(
        &mut self,
    ) -> Option<
        crate::resident::task_owner::ResidentOperationTaskCompletion<
            crate::resident::operation_dispatch::OwnedProviderInvocationRequest,
            Result<
                ProviderInvocationObservation,
                crate::provider_invocation::ProviderInvocationAdmissionError,
            >,
        >,
    > {
        self.executor
            .try_completion()
            .map(|completion| completion.inner)
    }
}

/// Fresh physical inputs for one retained built-in `DispatchPrepared` turn.
pub struct ResidentServiceNativePreparedInputs<'a> {
    /// Current admitted session authority.
    pub current: &'a crate::resident::AmbientSessionAdmissionContext<'a>,
    /// Observation signing authority.
    pub signing: SignedResidentAdmissionContextV1<'a>,
    /// Fresh reference for the `Dispatched` observation.
    pub observation_ref: ResidentRecordRefV1,
    /// Fresh identity for its checkpoint.
    pub checkpoint: crate::resident::OperationCheckpointInput,
    /// Checkpoint issue time.
    pub checkpoint_issued_at: UnixTimeV1,
    /// Checkpoint schema admitted by the deployment.
    pub checkpoint_schema: &'a SchemaDigest,
    /// Checkpoint signing authority admitted by the deployment.
    pub checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    /// Observation-store resolver.
    pub resolve_store: &'a dyn crate::resident::OperationObservationAuthorityStoreResolver,
    /// Bounds for accumulator preparation.
    pub accumulator_limits: AccumulatorPreparationLimits,
    /// Maximum retained selection bytes.
    pub maximum_selection_bytes: usize,
    /// Maximum bytes read for protected recovery records.
    pub maximum_record_bytes: usize,
    /// Native action registry selected by the deployment.
    pub registry: &'a crate::actions::NativeActionRegistry,
    /// Contract set used to admit the native result.
    pub contract_set: &'a ContractSetV1,
    /// Admit an exact observation signing-key rotation.
    pub admit_rotation: &'a mut ResidentServiceRotationAdmission<'a>,
    /// Sign the application-constructed dispatch observation.
    pub sign_observation: &'a mut ResidentServiceObservationSigner<'a>,
    /// Sign the application-constructed dispatch checkpoint.
    pub sign_checkpoint: &'a mut ResidentServiceCheckpointSigner<'a>,
}

/// Fresh physical inputs for one retained provider `DispatchPrepared` turn.
pub struct ResidentServiceProviderPreparedInputs<'a> {
    /// Current admitted session authority.
    pub current: &'a crate::resident::AmbientSessionAdmissionContext<'a>,
    /// Observation signing authority.
    pub signing: SignedResidentAdmissionContextV1<'a>,
    /// Fresh reference for the `Dispatched` observation.
    pub observation_ref: ResidentRecordRefV1,
    /// Fresh identity for its checkpoint.
    pub checkpoint: crate::resident::OperationCheckpointInput,
    /// Checkpoint issue time.
    pub checkpoint_issued_at: UnixTimeV1,
    /// Checkpoint schema admitted by the deployment.
    pub checkpoint_schema: &'a SchemaDigest,
    /// Checkpoint signing authority admitted by the deployment.
    pub checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    /// Observation-store resolver.
    pub resolve_store: &'a dyn crate::resident::OperationObservationAuthorityStoreResolver,
    /// Bounds for accumulator preparation.
    pub accumulator_limits: AccumulatorPreparationLimits,
    /// Maximum retained selection bytes.
    pub maximum_selection_bytes: usize,
    /// Maximum bytes read for protected recovery records.
    pub maximum_record_bytes: usize,
    /// Provider result admission selected for this exact attempt.
    pub admission: crate::provider_invocation::ProviderInvocationAdmissionContext<'a>,
    /// Terminal recording input used after mechanical completion.
    pub invocation: crate::resident::ProviderOperationInvokeInput<'a>,
    /// Independent provider-evidence admission owner.
    pub evidence: &'a dyn crate::provider_invocation::ProviderInvocationEvidenceAdmission,
    /// Mechanical executor that can accept the opaque one-shot task.
    pub executor: &'a mut dyn ResidentServiceProviderTaskExecutor,
    /// Admit an exact observation signing-key rotation.
    pub admit_rotation: &'a mut ResidentServiceRotationAdmission<'a>,
    /// Admit the reconciliation frontier used by an uncertain outcome.
    pub admit_reconciliation: &'a mut (dyn FnMut(&ReconciliationFrontierReferenceV1) -> Result<(), InvalidResidentRecordV1>
                 + 'a),
    /// Sign the application-constructed dispatch or terminal observation.
    pub sign_observation: &'a mut ResidentServiceObservationSigner<'a>,
    /// Sign the application-constructed dispatch or terminal checkpoint.
    pub sign_checkpoint: &'a mut ResidentServiceCheckpointSigner<'a>,
}

/// Exact admitted deployment coordinates supplied to a physical input owner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResidentServiceDeploymentSelector {
    /// Principal that owns the resident deployment.
    pub principal_id: ResidentPrincipalIdV1,
    /// Admitted deployment-binding reference.
    pub deployment_binding_ref: ResidentRecordRefV1,
    /// Canonical deployment-binding digest.
    pub deployment_binding_sha256: DeploymentBindingDigest,
    /// Admitted runtime-authority reference.
    pub runtime_authority_ref: ResidentRecordRefV1,
    /// Canonical runtime-authority digest.
    pub runtime_authority_sha256: DeploymentRuntimeAuthorityDigest,
}

/// Event custody used by one bounded publication turn.
///
/// Publication needs both point custody and an atomic control transaction. The
/// combined port keeps those operations on one physical owner.
pub trait ResidentServicePublicationCustodyPort: EventCustodyPort + ControlTransactionPort {}

impl<T> ResidentServicePublicationCustodyPort for T where
    T: EventCustodyPort + ControlTransactionPort + ?Sized
{
}

/// Sized forwarding view over the caller's combined publication custody.
///
/// The semantic publication owner remains generic over its physical port. This
/// view erases the deployment-specific port type without moving any decision
/// into service assembly.
struct ResidentServicePublicationCustody<'a>(&'a mut dyn ResidentServicePublicationCustodyPort);

impl ControlTransactionPort for ResidentServicePublicationCustody<'_> {
    fn prepare_transaction(
        &mut self,
        store: &AuthorityStoreCoordinate,
    ) -> Result<ControlTransactionPreparation, ControlStoreError> {
        self.0.prepare_transaction(store)
    }
}

impl EventCustodyPort for ResidentServicePublicationCustody<'_> {
    fn read_committed_members(
        &mut self,
        request: &EventCustodyHistoryRead,
    ) -> Result<EventCustodyHistoryObservation, ControlStoreError> {
        self.0.read_committed_members(request)
    }

    fn compare_and_commit(
        &mut self,
        transition: &EventCustodyTransition,
        maximum_journal_payload_bytes: usize,
    ) -> Result<EventCustodyCommitOutcome, ControlStoreError> {
        self.0
            .compare_and_commit(transition, maximum_journal_payload_bytes)
    }

    fn observe_exact(
        &mut self,
        transition: &EventCustodyTransition,
        maximum_journal_payload_bytes: usize,
    ) -> Result<EventCustodyReadBackOutcome, ControlStoreError> {
        self.0
            .observe_exact(transition, maximum_journal_payload_bytes)
    }

    fn read_index(
        &mut self,
        store: &AuthorityStoreCoordinate,
        query: &EventIndexQuery,
        maximum_body_bytes: usize,
    ) -> Result<EventIndexObservation, ControlStoreError> {
        self.0.read_index(store, query, maximum_body_bytes)
    }

    fn read_record(
        &mut self,
        store: &AuthorityStoreCoordinate,
        coordinate: &ControlRecordCoordinate,
        maximum_body_bytes: usize,
    ) -> Result<EventRecordObservation, ControlStoreError> {
        self.0.read_record(store, coordinate, maximum_body_bytes)
    }

    fn read_inbox_page(
        &mut self,
        request: &EventInboxPageRead,
    ) -> Result<EventInboxPage, ControlStoreError> {
        self.0.read_inbox_page(request)
    }

    fn read_pale_derived_outbox_page(
        &mut self,
        request: &PaleDerivedOutboxPageRead,
    ) -> Result<PaleDerivedOutboxPage, ControlStoreError> {
        self.0.read_pale_derived_outbox_page(request)
    }
}

/// Physical inputs for one application-selected publication stage.
///
/// This value supplies current authority, bounds, and custody. The existing
/// publication owner still selects the source, retains its cursor, derives
/// Event meaning, and performs the durable transition.
pub struct ResidentServicePublicationInputs<'a> {
    /// Event and transaction custody for the selected deployment.
    pub event_custody: &'a mut dyn ResidentServicePublicationCustodyPort,
    /// Aggregate scan and publication bounds for this turn.
    pub sweep_limits: LocalOperationPublicationSweepLimits,
    /// Protected local-operation recovery bounds.
    pub local_recovery_limits: LocalOperationPublicationRecoveryLimits,
    /// Protected A2A task recovery bounds.
    pub a2a_recovery_limits: A2aTaskPublicationRecoveryLimits,
    /// Current producer, writer, and signing authority.
    pub current: LocalOperationPublicationCurrent<'a>,
    /// Current signers used only when the selected stage is absent.
    pub issuance: LocalOperationPublicationIssuance<'a>,
    /// Historical authority admission for retained stages.
    pub readmission: LocalOperationPublicationReadmission<'a>,
    /// Recover one exact local operation from protected custody.
    pub recover_local: &'a mut (dyn for<'request> FnMut(
        &LocalOperationPublicationRecoveryRequest<'request>,
    ) -> Result<
        RecoveredOperationControl,
        LocalOperationPublicationSourceRecoveryError,
    > + 'a),
    /// Recover one exact A2A task history from protected custody.
    pub recover_a2a: &'a mut (dyn for<'request> FnMut(
        &A2aTaskPublicationRecoveryRequest<'request>,
    ) -> Result<
        RecoveredA2aTaskPublicationSource,
        A2aTaskPublicationSourceRecoveryError,
    > + 'a),
}

/// Lend physical inputs for one already selected publication stage.
pub trait ResidentServicePublicationInputPort {
    /// Lend inputs bound to the exact admitted deployment.
    ///
    /// # Errors
    ///
    /// Returns an error when current publication authority or physical custody
    /// is unavailable. The application keeps the existing publication cursor.
    fn publication_inputs<'a>(
        &'a mut self,
        deployment: &ResidentServiceDeploymentSelector,
    ) -> Result<ResidentServicePublicationInputs<'a>, ResidentOperationFrameFailure>;
}

pub(crate) struct UnavailableResidentServicePublicationInputs;

impl ResidentServicePublicationInputPort for UnavailableResidentServicePublicationInputs {
    fn publication_inputs<'a>(
        &'a mut self,
        _: &ResidentServiceDeploymentSelector,
    ) -> Result<ResidentServicePublicationInputs<'a>, ResidentOperationFrameFailure> {
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    }
}

/// Fresh custody inputs for one retained pre-dispatch expiry turn.
///
/// The application uses these values only after protected recovery has proved
/// the exact operation head. This input contains no action, provider, A2A
/// exchange, or other external-call owner.
pub struct ResidentServicePreDispatchExpiryPreparationInputs<'a> {
    /// Clock sampled once by the existing expiry owner.
    pub clock: &'a dyn WallClockSourcePort,
    /// Fresh reference for the expiry diagnostic.
    pub diagnostic_ref: ResidentRecordRefV1,
    /// Fresh reference for the signed expiry observation.
    pub observation_ref: ResidentRecordRefV1,
    /// Admitted observation signer.
    pub signing: SignedResidentAdmissionContextV1<'a>,
    /// Fresh identity for the successor checkpoint.
    pub checkpoint: crate::resident::OperationCheckpointInput,
    /// Schema admitted for the successor checkpoint.
    pub checkpoint_schema: &'a SchemaDigest,
    /// Checkpoint signer admitted for this deployment.
    pub checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    /// Existing owner of the selected observation store.
    pub resolve_store: &'a dyn crate::resident::OperationObservationAuthorityStoreResolver,
    /// Maximum canonical diagnostic bytes.
    pub maximum_diagnostic_bytes: usize,
    /// Maximum bytes read from protected operation custody.
    pub maximum_record_bytes: usize,
    /// Finite checkpoint-accumulator bounds.
    pub accumulator_limits: AccumulatorPreparationLimits,
    /// Admit an exact observation-signing rotation.
    pub admit_rotation: &'a mut ResidentServiceRotationAdmission<'a>,
    /// Sign the application-constructed expiry observation.
    pub sign_observation: &'a mut ResidentServiceObservationSigner<'a>,
    /// Sign the exact successor checkpoint at most once.
    pub sign_checkpoint: &'a mut ResidentServiceCheckpointSigner<'a>,
}

/// Custody inputs for one retained pre-dispatch expiry continuation.
///
/// The application already owns the signed observation and its exact retained
/// continuation. These inputs cannot allocate another record identity, sample
/// a clock, sign another observation, or open external work.
pub struct ResidentServicePreDispatchExpiryContinuationInputs<'a> {
    /// Schema admitted for the successor checkpoint.
    pub checkpoint_schema: &'a SchemaDigest,
    /// Checkpoint signer admitted for this deployment.
    pub checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    /// Existing owner of the selected observation store.
    pub resolve_store: &'a dyn crate::resident::OperationObservationAuthorityStoreResolver,
    /// Finite checkpoint-accumulator bounds.
    pub accumulator_limits: AccumulatorPreparationLimits,
    /// Sign the exact retained successor checkpoint at most once.
    pub sign_checkpoint: &'a mut ResidentServiceCheckpointSigner<'a>,
}

/// Lend physical recovery inputs for one already selected Invoke request.
pub trait ResidentServiceInvokeInputPort {
    /// Lend explicit permission, filesystem and signing inputs for this native event.
    /// Missing deployment choices refuse the request without choosing credentials.
    ///
    /// # Errors
    /// Returns configuration unavailability when no native response owner is installed.
    fn with_native_hook_response_inputs<Output>(
        &mut self,
        _deployment: &ResidentServiceDeploymentSelector,
        _request: &CommonRequestEnvelopeV1,
        _use_inputs: impl for<'a> FnOnce(
            crate::resident::operation_dispatch::NativeHookResponseInputs<'a>,
        ) -> Output,
    ) -> Result<Output, ResidentOperationFrameFailure> {
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    /// Lend exact inputs after peer, request, session, operation, and deployment
    /// admission. `locator` is absent only for a new root Invoke.
    ///
    /// # Errors
    ///
    /// Returns an error when the selected physical or historical inputs are
    /// unavailable.
    fn with_invoke_inputs<Output, UseInputs>(
        &mut self,
        deployment: &ResidentServiceDeploymentSelector,
        request: &CommonRequestEnvelopeV1,
        locator: Option<&ControlRecordCoordinate>,
        use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'operation, 'admission, 'checkpoint> FnOnce(
            ResidentServiceInvokeRecoveryInputs<'operation, 'admission, 'checkpoint>,
        ) -> Output;

    /// Lend physical inputs for one application-selected recovery scan.
    ///
    /// The application owns durable source discovery, its bounded cursor, and
    /// source classification. A retained in-memory phase for this deployment
    /// prevents another scan until an owner can continue that exact phase.
    ///
    /// # Errors
    ///
    /// Returns an error when the physical or historical inputs are unavailable
    /// for the selected deployment.
    fn with_background_invoke_inputs<Output, UseInputs>(
        &mut self,
        deployment: &ResidentServiceDeploymentSelector,
        use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'operation, 'admission> FnOnce(
            ResidentServiceInvokeBackgroundRecoveryInputs<'operation, 'admission>,
        ) -> Output;

    /// Lend fresh identities and custody owners for one expiry check.
    ///
    /// The default fails closed. An implementation must not expose an action,
    /// provider, or A2A transport through this callback.
    ///
    /// # Errors
    /// Returns an error when expiry preparation inputs are unavailable. The default returns `ConfigurationUnavailable` without calling the callback.
    fn with_pre_dispatch_expiry_preparation_inputs<Output, UseInputs>(
        &mut self,
        _deployment: &ResidentServiceDeploymentSelector,
        _locator: &ResidentOperationFrameLocator,
        _use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs:
            for<'input> FnOnce(ResidentServicePreDispatchExpiryPreparationInputs<'input>) -> Output,
    {
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    /// Lend only the custody inputs required by a retained expiry continuation.
    ///
    /// The default fails closed. An implementation must not expose a clock,
    /// fresh record identity, observation signer, action, provider, or A2A
    /// transport through this callback.
    ///
    /// # Errors
    /// Returns an error when the retained custody phase cannot obtain its inputs. The default returns `ConfigurationUnavailable` without calling the callback.
    fn with_pre_dispatch_expiry_continuation_inputs<Output, UseInputs>(
        &mut self,
        _deployment: &ResidentServiceDeploymentSelector,
        _locator: &ResidentOperationFrameLocator,
        _use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(
            ResidentServicePreDispatchExpiryContinuationInputs<'input>,
        ) -> Output,
    {
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    /// Lend current authority and physical implementation inputs for `Resolving`.
    ///
    /// # Errors
    /// Returns an error when current resolution inputs are unavailable. The default returns `ConfigurationUnavailable` without calling the callback.
    fn with_operation_resolution_inputs<Output, UseInputs>(
        &mut self,
        _deployment: &ResidentServiceDeploymentSelector,
        _locator: &ResidentOperationFrameLocator,
        _use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(ResidentServiceOperationResolutionInputs<'input>) -> Output,
    {
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    /// Lend current authority and custody owners for `ProviderResolved`.
    ///
    /// # Errors
    /// Returns an error when the resolved provider cannot obtain its current authority and custody inputs. The default returns `ConfigurationUnavailable`.
    fn with_provider_resolved_inputs<Output, UseInputs>(
        &mut self,
        _deployment: &ResidentServiceDeploymentSelector,
        _locator: &ResidentOperationFrameLocator,
        _use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(ResidentServiceProviderResolvedInputs<'input>) -> Output,
    {
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    /// Lend current authority and custody owners for built-in `DispatchPrepared`.
    ///
    /// # Errors
    /// Returns an error when the prepared native dispatch cannot obtain its current authority and custody inputs. The default returns `ConfigurationUnavailable`.
    fn with_native_prepared_inputs<Output, UseInputs>(
        &mut self,
        _deployment: &ResidentServiceDeploymentSelector,
        _locator: &ResidentOperationFrameLocator,
        _use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(ResidentServiceNativePreparedInputs<'input>) -> Output,
    {
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    /// Lend current authority, custody, and mechanical task owners for provider
    /// `DispatchPrepared`.
    ///
    /// # Errors
    /// Returns an error when the prepared provider dispatch cannot obtain its current authority and custody inputs. The default returns `ConfigurationUnavailable`.
    fn with_provider_prepared_inputs<Output, UseInputs>(
        &mut self,
        _deployment: &ResidentServiceDeploymentSelector,
        _locator: &ResidentOperationFrameLocator,
        _use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(ResidentServiceProviderPreparedInputs<'input>) -> Output,
    {
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    /// Lend fresh custody inputs for one retained provider dispatch.
    ///
    /// The default fails closed so existing input owners do not accidentally
    /// claim restart recovery. An implementation must not expose a provider
    /// invocation port through this callback.
    ///
    /// # Errors
    ///
    /// Returns an error when exact provider-recovery inputs are unavailable.
    fn with_provider_recovery_inputs<Output, UseInputs>(
        &mut self,
        _deployment: &ResidentServiceDeploymentSelector,
        _locator: &ResidentOperationFrameLocator,
        _use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(ResidentServiceProviderRecoveryInputs<'input>) -> Output,
    {
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    /// Lend current authority and recording inputs for one retained native turn.
    ///
    /// The default fails closed. The process host, not the input owner, retains
    /// the exact dispatch and every post-handler continuation.
    ///
    /// # Errors
    ///
    /// Returns an error when exact native-recovery inputs are unavailable.
    fn with_native_recovery_inputs<Output, UseInputs>(
        &mut self,
        _deployment: &ResidentServiceDeploymentSelector,
        _locator: &ResidentOperationFrameLocator,
        _use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(ResidentServiceNativeRecoveryInputs<'input>) -> Output,
    {
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    }
}

/// Fresh terminal identities used only when Cancel wins before dispatch.
pub struct ResidentServiceLocalCancellationCompletionAllocation {
    /// Immutable reference for the terminal cancellation diagnostic.
    pub diagnostic_ref: ResidentRecordRefV1,
    /// Immutable reference for the terminal cancellation observation.
    pub observation_ref: ResidentRecordRefV1,
    /// Time at which absence of dispatch was established.
    pub established_at: UnixTimeV1,
    /// Fresh checkpoint identity for the terminal append.
    pub checkpoint: crate::resident::OperationCheckpointInput,
    /// Physical time attached to the terminal checkpoint.
    pub checkpoint_issued_at: UnixTimeV1,
    /// Maximum canonical diagnostic bytes.
    pub maximum_diagnostic_bytes: usize,
}

/// Fresh identities, bounds, and admitted signers for one selected Cancel append.
pub struct ResidentServiceCancellationAllocation<'a> {
    /// Immutable reference allocated for the cancellation request evidence.
    pub request_ref: ResidentRecordRefV1,
    /// Immutable reference allocated for the successor observation.
    pub observation_ref: ResidentRecordRefV1,
    /// Already-selected observation signing authority.
    pub signing: SignedResidentAdmissionContextV1<'a>,
    /// Fresh checkpoint identity and writer evidence.
    pub checkpoint: crate::resident::OperationCheckpointInput,
    /// Physical time attached to the successor checkpoint.
    pub checkpoint_issued_at: UnixTimeV1,
    /// Schema admitted for the successor checkpoint.
    pub checkpoint_schema: &'a SchemaDigest,
    /// Bounds for checkpoint accumulator preparation.
    pub accumulator_limits: AccumulatorPreparationLimits,
    /// Existing local pre-dispatch completion input.
    pub local_completion: ResidentServiceLocalCancellationCompletionAllocation,
    /// Existing consultation collection bounds.
    pub consultation_limits: crate::resident::ConsultationCollectionLimits,
}

/// Physical authority, signing, and custody inputs for one selected Cancel target.
pub struct ResidentServiceCancellationInputs<'a> {
    /// Fresh application-owned allocation for this append.
    pub allocation: ResidentServiceCancellationAllocation<'a>,
    /// Present operation-control cancellation authority.
    pub authorize: &'a mut (dyn for<'context> FnMut(
        crate::resident::OperationControlAuthorizationContext<'context>,
    ) -> Result<(), InvalidResidentRecordV1>
                 + 'a),
    /// Immutable observation-record resolver.
    pub resolve_store: &'a (dyn crate::resident::OperationObservationAuthorityStoreResolver + 'a),
    /// Admission for an exact observation signing-key rotation.
    pub admit_rotation: &'a mut ResidentServiceRotationAdmission<'a>,
    /// Sign the application-constructed cancellation observation.
    pub sign_observation: &'a mut ResidentServiceObservationSigner<'a>,
    /// Sign the application-constructed successor checkpoint.
    pub sign_checkpoint: &'a mut ResidentServiceCheckpointSigner<'a>,
    /// Admit the exact consultation stop summary when the target is Consult.
    pub admit_stop: &'a mut (dyn FnMut(
        &crate::resident::ConsultationCollection,
        &crate::resident::ConsultationRequirementSummary,
    ) -> Result<(), InvalidResidentRecordV1>
                 + 'a),
    /// Admit the exact consultation collection when the target is Consult.
    pub admit_collection: &'a mut (dyn FnMut(
        &crate::resident::ConsultationCollection,
    ) -> Result<(), InvalidResidentRecordV1>
                 + 'a),
}

/// Lend cancellation mutation inputs after exact protected recovery and reservation.
pub trait ResidentServiceCancellationInputPort<'service>: Sized {
    /// Lend physical inputs for one exact deployment, signed request, and locator.
    ///
    /// The application has already recovered and admitted the target and has
    /// occupied one bounded retained-phase slot before this method is called.
    /// An error means only that the lender itself could not complete; it does
    /// not revoke a callback that was already invoked. The adapter records the
    /// callback-produced handoff independently and gives it precedence over a
    /// post-callback error, so an implementation cannot discard linear custody.
    /// Returning success without invoking the callback fails closed.
    ///
    /// # Errors
    /// Returns an error if the selected cancellation lacks current authority, fresh identities or custody inputs.
    fn with_cancellation_inputs<UseInputs>(
        &mut self,
        deployment: &ResidentServiceDeploymentSelector,
        request: &CommonRequestEnvelopeV1,
        locator: &ControlRecordCoordinate,
        use_inputs: UseInputs,
    ) -> Result<(), ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(ResidentServiceCancellationInputs<'input>);

    /// Lend the exact terminal inputs for uncertainty created by stopping one
    /// A2A response read.
    ///
    /// The input owner consumes and returns itself so the lent references can
    /// retain their service lifetime when physical custody needs another turn.
    /// An error before lending returns the unchanged outcome. Once the callback
    /// runs, its output takes precedence and cannot be replaced by a lender
    /// error.
    fn with_a2a_cancellation_outcome_inputs<Output, UseInputs>(
        self,
        deployment: &ResidentServiceDeploymentSelector,
        request_id: &ResidentRequestIdV1,
        request_sha256: &CommonRequestEnvelopeDigest,
        locator: &ControlRecordCoordinate,
        outcome: Box<crate::resident::A2aSendOutcomeUnknown>,
        use_inputs: UseInputs,
    ) -> (
        Self,
        Result<
            Output,
            (
                ResidentOperationFrameFailure,
                Box<crate::resident::A2aSendOutcomeUnknown>,
            ),
        >,
    )
    where
        UseInputs: FnOnce(crate::resident::A2aUnknownOutcomeRetention<'service>) -> Output;
}

#[cfg(test)]
#[path = "../../../tests/support/resident_service_cancellation.rs"]
pub(in crate::resident::process_host) mod cancellation_test_support;

/// Internal owners retained by the complete operation-frame service.
///
/// Public deployment code uses the operation-kind-specific owner bundle that
/// matches its configured policy. The complete bundle is
/// [`ResidentServiceInvokeConsultDelegateObserveCancelOwners`].
pub(crate) struct ResidentServiceInvokeOwners<
    'a,
    Evidence,
    Clock,
    SessionAuthority,
    CallerSignature,
    Random,
    OperationAuthority,
    Policy,
    Cryptography,
    InvokeInputs,
    DelegateInputs,
    CancellationInputs,
    ObserveInputs,
> {
    /// Per-connection admission owner for kernel-reported local peers.
    pub peer_admission: Box<dyn ResidentPeerAdmissionPort + 'a>,
    /// Maximum caller-signed request bytes.
    pub maximum_request_bytes: usize,
    /// Frame-kind-specific request policies.
    pub request_policies: ResidentServiceOperationRequestPolicies,
    /// Historical generation and session resolver.
    pub historical: &'a dyn GenerationLifecycleRecoveryResolver,
    /// Current authority evidence owner.
    pub evidence: Evidence,
    /// Physical wall-clock source.
    pub clock: Clock,
    /// Current ambient-session authority owner.
    pub session_authority: SessionAuthority,
    /// Current caller-signature verifier.
    pub caller_signature: CallerSignature,
    /// Cryptographic random source for fresh operation identities.
    pub random: Random,
    /// Current operation authority owner.
    pub operation_authority: OperationAuthority,
    /// Application-owned Invoke policy.
    pub policy: Policy,
    /// Start signing and checkpoint construction.
    pub cryptography: Cryptography,
    /// Physical Invoke recovery inputs.
    pub invoke_inputs: InvokeInputs,
    /// Physical Event-publication inputs.
    pub(crate) publication_inputs: Box<dyn ResidentServicePublicationInputPort + 'a>,
    /// Already-admitted Delegate continuation inputs.
    ///
    /// This is an application-internal phase owner. It receives no route
    /// selector and no reusable carrier; exact pairwise lineage and phase
    /// classification occur before any handoff.
    pub(crate) delegate_inputs: DelegateInputs,
    /// Independent physical cancellation mutation inputs.
    pub cancel_inputs: CancellationInputs,
    /// Historical operation recovery and disclosure inputs.
    pub observe_inputs: ObserveInputs,
}

/// Public owners for an Invoke, Observe, and Cancel resident service.
///
/// This is the deployable local-operation subset. Consult and Delegate remain
/// disabled: their request-policy sets must be empty, and their private phase
/// owners are not exposed through this type.
pub struct ResidentServiceInvokeObserveCancelOwners<
    'a,
    Evidence,
    Clock,
    SessionAuthority,
    CallerSignature,
    Random,
    OperationAuthority,
    Policy,
    Cryptography,
    InvokeInputs,
    CancellationInputs,
    ObserveInputs,
> {
    /// Per-connection admission owner for kernel-reported local peers.
    pub peer_admission: Box<dyn ResidentPeerAdmissionPort + 'a>,
    /// Maximum caller-signed request bytes.
    pub maximum_request_bytes: usize,
    /// Frame-kind-specific request policies.
    ///
    /// `start_consult`, `advance_consult`, `start_delegate`, and
    /// `advance_delegate` must be empty.
    pub request_policies: ResidentServiceOperationRequestPolicies,
    /// Historical generation and session resolver.
    pub historical: &'a dyn GenerationLifecycleRecoveryResolver,
    /// Current authority evidence owner.
    pub evidence: Evidence,
    /// Physical wall-clock source.
    pub clock: Clock,
    /// Current ambient-session authority owner.
    pub session_authority: SessionAuthority,
    /// Current caller-signature verifier.
    pub caller_signature: CallerSignature,
    /// Cryptographic random source for fresh operation identities.
    pub random: Random,
    /// Current operation authority owner.
    pub operation_authority: OperationAuthority,
    /// Application-owned Invoke policy.
    pub policy: Policy,
    /// Start signing and checkpoint construction.
    pub cryptography: Cryptography,
    /// Physical Invoke recovery inputs.
    pub invoke_inputs: InvokeInputs,
    /// Independent physical cancellation mutation inputs.
    pub cancel_inputs: CancellationInputs,
    /// Historical operation recovery and disclosure inputs.
    pub observe_inputs: ObserveInputs,
}

/// Public owners for an Invoke, Consult, Observe, and Cancel resident service.
///
/// Delegate remains disabled. Its request-policy sets must be empty, so this
/// composition cannot route a pairwise child through a hidden owner.
pub struct ResidentServiceInvokeConsultObserveCancelOwners<
    'a,
    Evidence,
    Clock,
    SessionAuthority,
    CallerSignature,
    Random,
    OperationAuthority,
    Policy,
    Cryptography,
    InvokeInputs,
    ConsultInputs,
    CancellationInputs,
    ObserveInputs,
> {
    /// Per-connection admission owner for kernel-reported local peers.
    pub peer_admission: Box<dyn ResidentPeerAdmissionPort + 'a>,
    /// Maximum caller-signed request bytes.
    pub maximum_request_bytes: usize,
    /// Frame-kind-specific request policies.
    ///
    /// `start_delegate` and `advance_delegate` must be empty.
    pub request_policies: ResidentServiceOperationRequestPolicies,
    /// Historical generation and session resolver.
    pub historical: &'a dyn GenerationLifecycleRecoveryResolver,
    /// Current authority evidence owner.
    pub evidence: Evidence,
    /// Physical wall-clock source.
    pub clock: Clock,
    /// Current ambient-session authority owner.
    pub session_authority: SessionAuthority,
    /// Current caller-signature verifier.
    pub caller_signature: CallerSignature,
    /// Cryptographic random source for fresh operation identities.
    pub random: Random,
    /// Current operation authority owner.
    pub operation_authority: OperationAuthority,
    /// Application-owned Invoke and Consult policy.
    pub policy: Policy,
    /// Start signing and checkpoint construction.
    pub cryptography: Cryptography,
    /// Physical Invoke recovery inputs.
    pub invoke_inputs: InvokeInputs,
    /// Physical Consult start and continuation inputs.
    pub consult_inputs: ConsultInputs,
    /// Independent physical cancellation mutation inputs.
    pub cancel_inputs: CancellationInputs,
    /// Historical operation recovery and disclosure inputs.
    pub observe_inputs: ObserveInputs,
}

/// Public owners for the complete local constitutional operation service.
///
/// Each semantic owner remains separate. Policy selects Invoke, Consult, and
/// Delegate meaning. The corresponding input owner lends only physical inputs
/// for the phase already selected from protected evidence. Observe and Cancel
/// keep their existing independent paths.
pub struct ResidentServiceInvokeConsultDelegateObserveCancelOwners<
    'a,
    Evidence,
    Clock,
    SessionAuthority,
    CallerSignature,
    Random,
    OperationAuthority,
    Policy,
    Cryptography,
    InvokeInputs,
    ConsultInputs,
    DelegateInputs,
    PublicationInputs,
    CancellationInputs,
    ObserveInputs,
> {
    /// Per-connection admission owner for kernel-reported local peers.
    pub peer_admission: Box<dyn ResidentPeerAdmissionPort + 'a>,
    /// Maximum caller-signed request bytes.
    pub maximum_request_bytes: usize,
    /// Frame-kind-specific request policies.
    pub request_policies: ResidentServiceOperationRequestPolicies,
    /// Historical generation and session resolver.
    pub historical: &'a dyn GenerationLifecycleRecoveryResolver,
    /// Current authority evidence owner.
    pub evidence: Evidence,
    /// Physical wall-clock source.
    pub clock: Clock,
    /// Current ambient-session authority owner.
    pub session_authority: SessionAuthority,
    /// Current caller-signature verifier.
    pub caller_signature: CallerSignature,
    /// Cryptographic random source for fresh operation identities.
    pub random: Random,
    /// Current operation authority owner.
    pub operation_authority: OperationAuthority,
    /// Application-owned Invoke, Consult, and Delegate policy.
    pub policy: Policy,
    /// Start signing and checkpoint construction.
    pub cryptography: Cryptography,
    /// Physical Invoke recovery inputs.
    pub invoke_inputs: InvokeInputs,
    /// Physical Consult start and continuation inputs.
    pub consult_inputs: ConsultInputs,
    /// One-shot A2A Delegate phase inputs.
    pub delegate_inputs: DelegateInputs,
    /// Physical inputs for bounded terminal Event publication.
    pub publication_inputs: PublicationInputs,
    /// Independent physical cancellation mutation inputs.
    pub cancel_inputs: CancellationInputs,
    /// Historical operation recovery and disclosure inputs.
    pub observe_inputs: ObserveInputs,
}

impl<
        'service,
        Evidence,
        Clock,
        SessionAuthority,
        CallerSignature,
        Random,
        OperationAuthority,
        Policy,
        Cryptography,
        InvokeInputs,
        ConsultInputs,
        DelegateInputs,
        PublicationInputs,
        CancellationInputs,
        ObserveInputs,
    >
    ResidentServiceInvokeConsultDelegateObserveCancelOwners<
        'service,
        Evidence,
        Clock,
        SessionAuthority,
        CallerSignature,
        Random,
        OperationAuthority,
        Policy,
        Cryptography,
        InvokeInputs,
        ConsultInputs,
        DelegateInputs,
        PublicationInputs,
        CancellationInputs,
        ObserveInputs,
    >
where
    Evidence: ResidentServiceCurrentRequestEvidencePort + 'service,
    Clock: WallClockSourcePort + 'service,
    SessionAuthority: AmbientSessionAuthorityPort + 'service,
    CallerSignature: ResidentServiceCallerSignaturePort + 'service,
    Random: CryptographicRandomSourcePort + 'service,
    OperationAuthority: AmbientSessionAuthorityPort + 'service,
    Policy: ResidentServiceInvokePolicyPort
        + ResidentServiceConsultPolicyPort
        + ResidentServiceDelegatePolicyPort
        + 'service,
    Policy::StopInput: ResidentServiceConsultStopInput + 'service,
    Cryptography: ResidentServiceInvokeCryptographyPort + 'service,
    InvokeInputs: ResidentServiceInvokeInputPort + 'service,
    ConsultInputs: ResidentServiceConsultInputPort + 'service,
    DelegateInputs: ResidentServiceDelegateInputPort<'service> + 'service,
    PublicationInputs: ResidentServicePublicationInputPort + 'service,
    CancellationInputs: ResidentServiceCancellationInputPort<'service> + 'service,
    ObserveInputs: ResidentServiceObserveInputPort<'service> + 'service,
{
    /// Recover one callable Invoke, Consult, Delegate, Observe, and Cancel service.
    ///
    /// Delegate inputs are lent only after protected recovery identifies the
    /// exact A2A phase. A recovered send and a recovered cancellation receive
    /// no carrier, so neither remote request can be issued twice.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid request policy, incomplete deployment
    /// association, unavailable task capacity, or failed resident recovery.
    pub fn recover<WriterGuard, WriterAdmission>(
        self,
        process: ResidentServiceProcessInput<'_, 'service, WriterGuard, WriterAdmission>,
    ) -> Result<ResidentFrameService<'service>, ResidentServiceAssemblyError>
    where
        WriterGuard: PrincipalWriterGuardPort + 'service,
        WriterAdmission: ResidentServiceCurrentWriterAdmissionPort,
    {
        let maximum_consultation_turns = process.task_limits.maximum_consultation_batches;
        let delegate_capacity = process.task_limits.maximum_provider_tasks.get();
        let ResidentServiceInvokeConsultDelegateObserveCancelOwners {
            peer_admission,
            maximum_request_bytes,
            request_policies,
            historical,
            evidence,
            clock,
            session_authority,
            caller_signature,
            random,
            operation_authority,
            policy,
            cryptography,
            invoke_inputs,
            consult_inputs,
            delegate_inputs,
            publication_inputs,
            cancel_inputs,
            observe_inputs,
        } = self;
        ResidentFrameService::recover_operation_service(
            process,
            ResidentServiceInvokeOwners {
                peer_admission,
                maximum_request_bytes,
                request_policies,
                historical,
                evidence,
                clock,
                session_authority,
                caller_signature,
                random,
                operation_authority,
                policy: InvokeConsultDelegatePolicyAdapter(policy),
                cryptography,
                invoke_inputs,
                publication_inputs: Box::new(publication_inputs),
                delegate_inputs: ResidentServiceDelegateRecoveryAdapter::new(
                    delegate_inputs,
                    delegate_capacity,
                ),
                cancel_inputs,
                observe_inputs,
            },
            ResidentServiceConsultRecoveryAdapter::new(consult_inputs, maximum_consultation_turns),
        )
    }
}

fn admit_invoke_consult_request_kinds(
    has_start_delegate_policy: bool,
    has_advance_delegate_policy: bool,
) -> Result<(), ResidentServiceAssemblyError> {
    if has_start_delegate_policy || has_advance_delegate_policy {
        Err(ResidentServiceAssemblyError::RequestPolicy)
    } else {
        Ok(())
    }
}

/// Why the public resident frame service could not be assembled.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ResidentServiceAssemblyError {
    /// Current-request policy or byte bounds are empty or ambiguous.
    #[error("resident service request policy is invalid")]
    RequestPolicy,
    /// A caller-opened deployment does not match the admitted trust frontier.
    #[error("resident service deployment set is invalid")]
    Deployment,
    /// Process-local task bounds could not be reserved.
    #[error("resident service task bounds are unavailable")]
    TaskBounds,
    /// Writer, trust, cold-open, or deployment recovery failed.
    #[error("resident service recovery failed")]
    Recovery,
}

impl<'service> ResidentFrameService<'service> {
    /// Recover one callable Observe-capable service from exact deployment inputs.
    ///
    /// Operation frames fail before protected request resolution because this
    /// first composition retains only Observe policies and owners.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid policy, incomplete deployment association,
    /// unavailable task capacity, or failed resident recovery.
    #[allow(clippy::too_many_arguments)]
    pub fn recover_observe<
        WriterGuard,
        WriterAdmission,
        Evidence,
        Clock,
        SessionAuthority,
        CallerSignature,
        OperationAuthority,
        ObserveInputs,
    >(
        process: ResidentServiceProcessInput<'_, 'service, WriterGuard, WriterAdmission>,
        observe: ResidentServiceObserveOwners<
            'service,
            Evidence,
            Clock,
            SessionAuthority,
            CallerSignature,
            OperationAuthority,
            ObserveInputs,
        >,
    ) -> Result<Self, ResidentServiceAssemblyError>
    where
        WriterGuard: PrincipalWriterGuardPort + 'service,
        WriterAdmission: ResidentServiceCurrentWriterAdmissionPort,
        Evidence: ResidentServiceCurrentRequestEvidencePort + 'service,
        Clock: WallClockSourcePort + 'service,
        SessionAuthority: AmbientSessionAuthorityPort + 'service,
        CallerSignature: ResidentServiceCallerSignaturePort + 'service,
        OperationAuthority: AmbientSessionAuthorityPort + 'service,
        ObserveInputs: ResidentServiceObserveInputPort<'service> + 'service,
    {
        let ResidentServiceProcessInput {
            held_writer,
            writer_snapshot,
            mut writer_admission,
            trust,
            deployments,
            maximum_ready_deployments,
            task_limits,
        } = process;
        let ResidentServiceObserveOwners {
            peer_admission,
            maximum_request_bytes,
            policies,
            historical,
            evidence,
            clock,
            session_authority,
            caller_signature,
            operation_authority,
            observe_inputs,
        } = observe;
        if policies.is_empty() {
            return Err(ResidentServiceAssemblyError::RequestPolicy);
        }
        let request_policies = policies
            .into_iter()
            .map(|policy| program::ResidentProgramRequestPolicy::observe(policy.into_inner()))
            .collect::<Vec<_>>();
        let current = program::ResidentProgramCurrentRequestAdmission::new(
            maximum_request_bytes,
            request_policies,
            CurrentEvidenceAdapter(evidence),
            clock,
            historical,
            session_authority,
            CallerSignatureAdapter(caller_signature),
        )
        .map_err(|_| ResidentServiceAssemblyError::RequestPolicy)?;

        let mut program_opens = Vec::with_capacity(deployments.len());
        let mut operation_ports = Vec::with_capacity(deployments.len());
        for deployment in deployments {
            let ResidentServiceDeploymentOpen {
                store,
                durable,
                sessions,
                events,
                maximum_event_custody_bytes,
                cold_open,
                operations,
                anti_rollback,
            } = deployment;
            let selector = service_selector(trust, &store)?;
            operation_ports.push(program::ResidentProgramDeploymentOperationPorts::new(
                selector,
                store,
                operations,
                sessions,
                anti_rollback,
            ));
            program_opens.push(program::ResidentProgramDeploymentOpen {
                durable,
                sessions,
                events,
                maximum_event_custody_bytes,
                input: cold_open,
            });
        }

        let task_state = ResidentOperationTaskState::new(
            ResidentOperationTaskLimits::new(task_limits.maximum_provider_tasks),
            task_limits.maximum_consultation_batches,
            ResidentAggregateTaskLimits::new(task_limits.maximum_aggregate_tasks),
        )
        .map_err(|_| ResidentServiceAssemblyError::TaskBounds)?;
        let turn_owner = ObserveTurnOwner {
            peer_admission: PeerAdmissionAdapter(peer_admission),
            current,
            deployments: operation_ports.into_boxed_slice(),
            operation_authority,
            recovery: ObserveInputAdapter(observe_inputs),
        };
        let mut writer_adapter = CurrentWriterAdmissionAdapter(&mut writer_admission);
        let program = program::ResidentProgram::recover(
            held_writer,
            writer_snapshot,
            &mut writer_adapter,
            trust.inner(),
            program_opens,
            maximum_ready_deployments,
            task_state,
            UnavailableOperationPolicy,
            turn_owner,
        )
        .map_err(|_| ResidentServiceAssemblyError::Recovery)?;
        Ok(Self::from_program(program))
    }

    /// Recover a callable Invoke, Observe, and Cancel service.
    ///
    /// Consult and Delegate are deliberately unavailable in this composition.
    /// Their policy sets must be empty. Requests for those frame kinds then
    /// fail during request admission instead of reaching a hidden phase owner.
    ///
    /// # Errors
    ///
    /// Returns an error for enabled Consult or Delegate policy, invalid local
    /// policy, incomplete deployment association, unavailable task capacity,
    /// or failed resident recovery.
    #[allow(clippy::too_many_arguments)]
    pub fn recover_invoke_observe_cancel<
        WriterGuard,
        WriterAdmission,
        Evidence,
        Clock,
        SessionAuthority,
        CallerSignature,
        Random,
        OperationAuthority,
        Policy,
        Cryptography,
        InvokeInputs,
        CancellationInputs,
        ObserveInputs,
    >(
        process: ResidentServiceProcessInput<'_, 'service, WriterGuard, WriterAdmission>,
        owners: ResidentServiceInvokeObserveCancelOwners<
            'service,
            Evidence,
            Clock,
            SessionAuthority,
            CallerSignature,
            Random,
            OperationAuthority,
            Policy,
            Cryptography,
            InvokeInputs,
            CancellationInputs,
            ObserveInputs,
        >,
    ) -> Result<Self, ResidentServiceAssemblyError>
    where
        WriterGuard: PrincipalWriterGuardPort + 'service,
        WriterAdmission: ResidentServiceCurrentWriterAdmissionPort,
        Evidence: ResidentServiceCurrentRequestEvidencePort + 'service,
        Clock: WallClockSourcePort + 'service,
        SessionAuthority: AmbientSessionAuthorityPort + 'service,
        CallerSignature: ResidentServiceCallerSignaturePort + 'service,
        Random: CryptographicRandomSourcePort + 'service,
        OperationAuthority: AmbientSessionAuthorityPort + 'service,
        Policy: ResidentServiceInvokePolicyPort + 'service,
        Cryptography: ResidentServiceInvokeCryptographyPort + 'service,
        InvokeInputs: ResidentServiceInvokeInputPort + 'service,
        CancellationInputs: ResidentServiceCancellationInputPort<'service> + 'service,
        ObserveInputs: ResidentServiceObserveInputPort<'service> + 'service,
    {
        let ResidentServiceInvokeObserveCancelOwners {
            peer_admission,
            maximum_request_bytes,
            request_policies,
            historical,
            evidence,
            clock,
            session_authority,
            caller_signature,
            random,
            operation_authority,
            policy,
            cryptography,
            invoke_inputs,
            cancel_inputs,
            observe_inputs,
        } = owners;
        if !request_policies.start_consult.is_empty()
            || !request_policies.advance_consult.is_empty()
            || !request_policies.start_delegate.is_empty()
            || !request_policies.advance_delegate.is_empty()
        {
            return Err(ResidentServiceAssemblyError::RequestPolicy);
        }
        Self::recover_invoke(
            process,
            ResidentServiceInvokeOwners {
                peer_admission,
                maximum_request_bytes,
                request_policies,
                historical,
                evidence,
                clock,
                session_authority,
                caller_signature,
                random,
                operation_authority,
                policy: InvokeOnlyPolicy(policy),
                cryptography,
                invoke_inputs,
                publication_inputs: Box::new(UnavailableResidentServicePublicationInputs),
                delegate_inputs: UnavailableResidentServiceDelegateInputs,
                cancel_inputs,
                observe_inputs,
            },
        )
    }

    /// Recover a callable Invoke, Consult, Observe, and Cancel service.
    ///
    /// Delegate remains unavailable. Its two request-policy sets must be empty.
    /// Consult policy and physical inputs remain separate: the policy selects
    /// roster and completion meaning, while the input owner lends only fresh
    /// coordinates, signers, bounded execution, and custody callbacks.
    ///
    /// # Errors
    ///
    /// Returns an error for enabled Delegate policy, invalid local policy,
    /// incomplete deployment association, unavailable task capacity, or failed
    /// resident recovery.
    #[allow(clippy::too_many_arguments)]
    pub fn recover_invoke_consult_observe_cancel<
        WriterGuard,
        WriterAdmission,
        Evidence,
        Clock,
        SessionAuthority,
        CallerSignature,
        Random,
        OperationAuthority,
        Policy,
        Cryptography,
        InvokeInputs,
        ConsultInputs,
        CancellationInputs,
        ObserveInputs,
    >(
        process: ResidentServiceProcessInput<'_, 'service, WriterGuard, WriterAdmission>,
        owners: ResidentServiceInvokeConsultObserveCancelOwners<
            'service,
            Evidence,
            Clock,
            SessionAuthority,
            CallerSignature,
            Random,
            OperationAuthority,
            Policy,
            Cryptography,
            InvokeInputs,
            ConsultInputs,
            CancellationInputs,
            ObserveInputs,
        >,
    ) -> Result<Self, ResidentServiceAssemblyError>
    where
        WriterGuard: PrincipalWriterGuardPort + 'service,
        WriterAdmission: ResidentServiceCurrentWriterAdmissionPort,
        Evidence: ResidentServiceCurrentRequestEvidencePort + 'service,
        Clock: WallClockSourcePort + 'service,
        SessionAuthority: AmbientSessionAuthorityPort + 'service,
        CallerSignature: ResidentServiceCallerSignaturePort + 'service,
        Random: CryptographicRandomSourcePort + 'service,
        OperationAuthority: AmbientSessionAuthorityPort + 'service,
        Policy: ResidentServiceInvokePolicyPort + ResidentServiceConsultPolicyPort + 'service,
        Policy::StopInput: ResidentServiceConsultStopInput + 'service,
        Cryptography: ResidentServiceInvokeCryptographyPort + 'service,
        InvokeInputs: ResidentServiceInvokeInputPort + 'service,
        ConsultInputs: ResidentServiceConsultInputPort + 'service,
        CancellationInputs: ResidentServiceCancellationInputPort<'service> + 'service,
        ObserveInputs: ResidentServiceObserveInputPort<'service> + 'service,
    {
        let maximum_consultation_turns = process.task_limits.maximum_consultation_batches;
        let ResidentServiceInvokeConsultObserveCancelOwners {
            peer_admission,
            maximum_request_bytes,
            request_policies,
            historical,
            evidence,
            clock,
            session_authority,
            caller_signature,
            random,
            operation_authority,
            policy,
            cryptography,
            invoke_inputs,
            consult_inputs,
            cancel_inputs,
            observe_inputs,
        } = owners;
        admit_invoke_consult_request_kinds(
            !request_policies.start_delegate.is_empty(),
            !request_policies.advance_delegate.is_empty(),
        )?;
        Self::recover_operation_service(
            process,
            ResidentServiceInvokeOwners {
                peer_admission,
                maximum_request_bytes,
                request_policies,
                historical,
                evidence,
                clock,
                session_authority,
                caller_signature,
                random,
                operation_authority,
                policy: InvokeConsultPolicyAdapter(policy),
                cryptography,
                invoke_inputs,
                publication_inputs: Box::new(UnavailableResidentServicePublicationInputs),
                delegate_inputs: UnavailableResidentServiceDelegateInputs,
                cancel_inputs,
                observe_inputs,
            },
            ResidentServiceConsultRecoveryAdapter::new(consult_inputs, maximum_consultation_turns),
        )
    }

    /// Recover the complete in-crate operation service.
    ///
    /// This composition admits its configured operation-kind policies through
    /// one retained owner. Cancel uses an independent physical input owner and
    /// the same protected-recovery, retained-phase, and work-turn lanes. A
    /// point-addressed continuation that needs a provider or native execution
    /// owner remains retained and is reported as pending. Background recovery
    /// retains the same exact phase, reports recovery unavailable, and does not
    /// advance the durable discovery cursor. This composition never replays the
    /// protected work.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid policy, incomplete deployment association,
    /// unavailable task capacity, or failed resident recovery.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident::process_host) fn recover_invoke<
        WriterGuard,
        WriterAdmission,
        Evidence,
        Clock,
        SessionAuthority,
        CallerSignature,
        Random,
        OperationAuthority,
        Policy,
        Cryptography,
        InvokeInputs,
        DelegateInputs,
        CancellationInputs,
        ObserveInputs,
    >(
        process: ResidentServiceProcessInput<'_, 'service, WriterGuard, WriterAdmission>,
        invoke: ResidentServiceInvokeOwners<
            'service,
            Evidence,
            Clock,
            SessionAuthority,
            CallerSignature,
            Random,
            OperationAuthority,
            Policy,
            Cryptography,
            InvokeInputs,
            DelegateInputs,
            CancellationInputs,
            ObserveInputs,
        >,
    ) -> Result<Self, ResidentServiceAssemblyError>
    where
        WriterGuard: PrincipalWriterGuardPort + 'service,
        WriterAdmission: ResidentServiceCurrentWriterAdmissionPort,
        Evidence: ResidentServiceCurrentRequestEvidencePort + 'service,
        Clock: WallClockSourcePort + 'service,
        SessionAuthority: AmbientSessionAuthorityPort + 'service,
        CallerSignature: ResidentServiceCallerSignaturePort + 'service,
        Random: CryptographicRandomSourcePort + 'service,
        OperationAuthority: AmbientSessionAuthorityPort + 'service,
        Policy: ResidentServiceInvokePolicyPort + ResidentServiceDelegatePolicyPort + 'service,
        Cryptography: ResidentServiceInvokeCryptographyPort + 'service,
        InvokeInputs: ResidentServiceInvokeInputPort + 'service,
        DelegateInputs: ResidentA2aDelegateRecoveryPort<
                WriterGuard,
                Output = operation_request_host::ResidentOperationRequestOutcome,
                Error = ResidentOperationFrameFailure,
            > + ResidentServiceRetainedDelegatePhasePort
            + 'service,
        CancellationInputs: ResidentServiceCancellationInputPort<'service> + 'service,
        ObserveInputs: ResidentServiceObserveInputPort<'service> + 'service,
    {
        let ResidentServiceInvokeOwners {
            peer_admission,
            maximum_request_bytes,
            request_policies,
            historical,
            evidence,
            clock,
            session_authority,
            caller_signature,
            random,
            operation_authority,
            policy,
            cryptography,
            invoke_inputs,
            publication_inputs,
            delegate_inputs,
            cancel_inputs,
            observe_inputs,
        } = invoke;
        Self::recover_operation_service(
            process,
            ResidentServiceInvokeOwners {
                peer_admission,
                maximum_request_bytes,
                request_policies,
                historical,
                evidence,
                clock,
                session_authority,
                caller_signature,
                random,
                operation_authority,
                policy: InvokePolicyAdapter(policy),
                cryptography,
                invoke_inputs,
                publication_inputs,
                delegate_inputs,
                cancel_inputs,
                observe_inputs,
            },
            program::ResidentProgramConsultRecoveryUnavailable,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn recover_operation_service<
        WriterGuard,
        WriterAdmission,
        Evidence,
        Clock,
        SessionAuthority,
        CallerSignature,
        Random,
        OperationAuthority,
        PolicyEvaluator,
        Cryptography,
        InvokeInputs,
        DelegateInputs,
        CancellationInputs,
        ObserveInputs,
        ConsultRecovery,
        StopInput,
    >(
        process: ResidentServiceProcessInput<'_, 'service, WriterGuard, WriterAdmission>,
        invoke: ResidentServiceInvokeOwners<
            'service,
            Evidence,
            Clock,
            SessionAuthority,
            CallerSignature,
            Random,
            OperationAuthority,
            PolicyEvaluator,
            Cryptography,
            InvokeInputs,
            DelegateInputs,
            CancellationInputs,
            ObserveInputs,
        >,
        consult_recovery: ConsultRecovery,
    ) -> Result<Self, ResidentServiceAssemblyError>
    where
        WriterGuard: PrincipalWriterGuardPort + 'service,
        WriterAdmission: ResidentServiceCurrentWriterAdmissionPort,
        Evidence: ResidentServiceCurrentRequestEvidencePort + 'service,
        Clock: WallClockSourcePort + 'service,
        SessionAuthority: AmbientSessionAuthorityPort + 'service,
        CallerSignature: ResidentServiceCallerSignaturePort + 'service,
        Random: CryptographicRandomSourcePort + 'service,
        OperationAuthority: AmbientSessionAuthorityPort + 'service,
        PolicyEvaluator: ResidentProcessInvokePolicyEvaluator
            + super::ResidentProcessConsultPolicyEvaluator
            + super::ResidentProcessDelegatePolicyEvaluator
            + consult_recovery::ResidentConsultationPolicyEvaluator<StopInput>
            + 'service,
        StopInput: program::ResidentProgramConsultStopInput + 'service,
        Cryptography: ResidentServiceInvokeCryptographyPort + 'service,
        InvokeInputs: ResidentServiceInvokeInputPort + 'service,
        DelegateInputs: ResidentA2aDelegateRecoveryPort<
                WriterGuard,
                Output = operation_request_host::ResidentOperationRequestOutcome,
                Error = ResidentOperationFrameFailure,
            > + ResidentServiceRetainedDelegatePhasePort
            + 'service,
        CancellationInputs: ResidentServiceCancellationInputPort<'service> + 'service,
        ObserveInputs: ResidentServiceObserveInputPort<'service> + 'service,
        ConsultRecovery: program::ResidentProgramConsultRecoveryInputPort<StopInput> + 'service,
    {
        let ResidentServiceProcessInput {
            held_writer,
            writer_snapshot,
            mut writer_admission,
            trust,
            deployments,
            maximum_ready_deployments,
            task_limits,
        } = process;
        let ResidentServiceInvokeOwners {
            peer_admission,
            maximum_request_bytes,
            request_policies,
            historical,
            evidence,
            clock,
            session_authority,
            caller_signature,
            random,
            operation_authority,
            policy,
            cryptography,
            invoke_inputs,
            publication_inputs,
            delegate_inputs,
            cancel_inputs,
            observe_inputs,
        } = invoke;
        if request_policies.start.is_empty()
            && request_policies.advance.is_empty()
            && request_policies.start_consult.is_empty()
            && request_policies.advance_consult.is_empty()
            && request_policies.start_delegate.is_empty()
            && request_policies.advance_delegate.is_empty()
            && request_policies.observe.is_empty()
            && request_policies.cancel.is_empty()
        {
            return Err(ResidentServiceAssemblyError::RequestPolicy);
        }
        let mut request_policies_internal = Vec::with_capacity(
            request_policies.start.len()
                + request_policies.advance.len()
                + request_policies.start_consult.len()
                + request_policies.advance_consult.len()
                + request_policies.start_delegate.len()
                + request_policies.advance_delegate.len()
                + request_policies.observe.len()
                + request_policies.cancel.len(),
        );
        request_policies_internal.extend(request_policies.start.into_iter().map(|policy| {
            program::ResidentProgramRequestPolicy::new(
                operation_request_host::ResidentOperationRequestKind::StartInvoke,
                policy.into_inner(),
            )
        }));
        request_policies_internal.extend(request_policies.advance.into_iter().map(|policy| {
            program::ResidentProgramRequestPolicy::new(
                operation_request_host::ResidentOperationRequestKind::AdvanceInvoke,
                policy.into_inner(),
            )
        }));
        request_policies_internal.extend(request_policies.start_consult.into_iter().map(
            |policy| {
                program::ResidentProgramRequestPolicy::new(
                    operation_request_host::ResidentOperationRequestKind::StartConsult,
                    policy.into_inner(),
                )
            },
        ));
        request_policies_internal.extend(request_policies.advance_consult.into_iter().map(
            |policy| {
                program::ResidentProgramRequestPolicy::new(
                    operation_request_host::ResidentOperationRequestKind::AdvanceConsult,
                    policy.into_inner(),
                )
            },
        ));
        request_policies_internal.extend(request_policies.start_delegate.into_iter().map(
            |policy| {
                program::ResidentProgramRequestPolicy::new(
                    operation_request_host::ResidentOperationRequestKind::StartDelegate,
                    policy.into_inner(),
                )
            },
        ));
        request_policies_internal.extend(request_policies.advance_delegate.into_iter().map(
            |policy| {
                program::ResidentProgramRequestPolicy::new(
                    operation_request_host::ResidentOperationRequestKind::AdvanceDelegate,
                    policy.into_inner(),
                )
            },
        ));
        request_policies_internal.extend(
            request_policies
                .observe
                .into_iter()
                .map(|policy| program::ResidentProgramRequestPolicy::observe(policy.into_inner())),
        );
        request_policies_internal.extend(request_policies.cancel.into_iter().map(|policy| {
            program::ResidentProgramRequestPolicy::new(
                operation_request_host::ResidentOperationRequestKind::Cancel,
                policy.into_inner(),
            )
        }));
        let current = program::ResidentProgramCurrentRequestAdmission::new(
            maximum_request_bytes,
            request_policies_internal,
            CurrentEvidenceAdapter(evidence),
            clock,
            historical,
            session_authority,
            CallerSignatureAdapter(caller_signature),
        )
        .map_err(|_| ResidentServiceAssemblyError::RequestPolicy)?;

        let mut program_opens = Vec::with_capacity(deployments.len());
        let mut operation_ports = Vec::with_capacity(deployments.len());
        for deployment in deployments {
            let ResidentServiceDeploymentOpen {
                store,
                durable,
                sessions,
                events,
                maximum_event_custody_bytes,
                cold_open,
                operations,
                anti_rollback,
            } = deployment;
            let selector = service_selector(trust, &store)?;
            operation_ports.push(program::ResidentProgramDeploymentOperationPorts::new(
                selector,
                store,
                operations,
                sessions,
                anti_rollback,
            ));
            program_opens.push(program::ResidentProgramDeploymentOpen {
                durable,
                sessions,
                events,
                maximum_event_custody_bytes,
                input: cold_open,
            });
        }

        let task_state = ResidentOperationTaskState::new(
            ResidentOperationTaskLimits::new(task_limits.maximum_provider_tasks),
            task_limits.maximum_consultation_batches,
            ResidentAggregateTaskLimits::new(task_limits.maximum_aggregate_tasks),
        )
        .map_err(|_| ResidentServiceAssemblyError::TaskBounds)?;
        let recovery = InvokeRecoveryAdapter::new(
            invoke_inputs,
            publication_inputs,
            delegate_inputs,
            cancel_inputs,
            observe_inputs,
            task_limits.maximum_provider_tasks.get(),
        );
        let turn_owner = program::ResidentProgramOperationTurnOwner::new(
            PeerAdmissionAdapter(peer_admission),
            current,
            operation_ports,
            random,
            operation_authority,
            InvokeCryptographyAdapter(cryptography),
            recovery,
            consult_recovery,
        )
        .map_err(|_| ResidentServiceAssemblyError::RequestPolicy)?;
        let mut writer_adapter = CurrentWriterAdmissionAdapter(&mut writer_admission);
        let program = program::ResidentProgram::recover(
            held_writer,
            writer_snapshot,
            &mut writer_adapter,
            trust.inner(),
            program_opens,
            maximum_ready_deployments,
            task_state,
            policy,
            turn_owner,
        )
        .map_err(|_| ResidentServiceAssemblyError::Recovery)?;
        Ok(Self::from_program(program))
    }
}

fn service_selector(
    trust: &ResidentServiceTrustFrontier,
    store: &AuthorityStoreCoordinate,
) -> Result<super::ResidentDeploymentAuthoritySelector, ResidentServiceAssemblyError> {
    let current = trust.inner().current_checkpoint();
    let mut matching = trust
        .inner()
        .runtime_authorities()
        .iter()
        .filter(|runtime| {
            runtime.authority().control_store_id() == store.store_id()
                && runtime.authority().control_incarnation_id() == store.incarnation_id()
        });
    let runtime = matching
        .next()
        .ok_or(ResidentServiceAssemblyError::Deployment)?;
    if matching.next().is_some() {
        return Err(ResidentServiceAssemblyError::Deployment);
    }
    Ok(
        program::ResidentProgramDeploymentAuthority::from_recovered(runtime, current)
            .selector()
            .clone(),
    )
}

struct CurrentWriterAdmissionAdapter<'a, Admission>(&'a mut Admission);

impl<Admission> super::CurrentPrincipalWriterAdmissionPort
    for CurrentWriterAdmissionAdapter<'_, Admission>
where
    Admission: ResidentServiceCurrentWriterAdmissionPort,
{
    fn admit_current_writer(
        &mut self,
        snapshot: &PrincipalResidentWriterSnapshot,
    ) -> Result<super::AdmittedCurrentPrincipalWriter, InvalidResidentRecordV1> {
        let evidence = self.0.admit_current_writer(snapshot)?;
        Ok(super::AdmittedCurrentPrincipalWriter {
            writer: evidence.writer,
            checkpoint: evidence.checkpoint,
        })
    }
}

struct CurrentEvidenceAdapter<Evidence>(Evidence);

impl<Evidence> current_request::ResidentCurrentRequestEvidencePort<ResidentPeerAdmissionWitness>
    for CurrentEvidenceAdapter<Evidence>
where
    Evidence: ResidentServiceCurrentRequestEvidencePort,
{
    fn read_current_request_evidence(
        &mut self,
        selector: current_request::ResidentCurrentRequestSelector<'_, ResidentPeerAdmissionWitness>,
    ) -> Result<
        current_request::ResidentCurrentRequestEvidence,
        current_request::ResidentCurrentRequestEvidenceError,
    > {
        self.0
            .read_current_request_evidence(ResidentServiceCurrentRequestSelector {
                peer_witness: selector.peer_witness,
                session_admission_id: selector.session_admission_id,
                session_admission_lease_digest: selector.session_admission_lease_digest,
                session_grant_id: selector.session_grant_id,
                deployment_binding_ref: selector.deployment_binding_ref,
                deployment_binding_sha256: selector.deployment_binding_sha256,
            })
            .map(Into::into)
            .map_err(current_request::ResidentCurrentRequestEvidenceError::Record)
    }
}

impl From<ResidentServiceCurrentRequestEvidence>
    for current_request::ResidentCurrentRequestEvidence
{
    fn from(evidence: ResidentServiceCurrentRequestEvidence) -> Self {
        Self {
            grant: current_request::ResidentCurrentGrantEvidence {
                grant_ref: evidence.grant.grant_ref,
                grant: evidence.grant.grant,
                issued_trust_checkpoint_ref: evidence.grant.issued_trust_checkpoint_ref,
                issued_trust_checkpoint: evidence.grant.issued_trust_checkpoint,
            },
            capability_revocations: current_request::ResidentCurrentCapabilityRevocationEvidence {
                checkpoint_ref: evidence.capability_revocations.checkpoint_ref,
                checkpoint: evidence.capability_revocations.checkpoint,
                grant_head: evidence.capability_revocations.grant_head.map(|head| {
                    current_request::ResidentCurrentGrantRevocationEvidence {
                        revocation_ref: head.revocation_ref,
                        revocation: head.revocation,
                    }
                }),
            },
            deployment_revocations: current_request::ResidentCurrentDeploymentRevocationEvidence {
                epoch: evidence.deployment_revocations.epoch,
                binding_head: evidence.deployment_revocations.binding_head.map(|head| {
                    current_request::ResidentCurrentDeploymentRevocationHeadEvidence {
                        revocation_ref: head.revocation_ref,
                        revocation: head.revocation,
                    }
                }),
            },
            session_revocations: current_request::ResidentCurrentSessionRevocationEvidence {
                epoch: evidence.session_revocations.epoch,
                revocation_ref: evidence.session_revocations.revocation_ref,
                revocation_sha256: evidence.session_revocations.revocation_sha256,
            },
        }
    }
}

struct CallerSignatureAdapter<Signature>(Signature);

impl<Signature> program::ResidentProgramCallerSignaturePort for CallerSignatureAdapter<Signature>
where
    Signature: ResidentServiceCallerSignaturePort,
{
    fn admit_native_caller_key(
        &mut self,
        caller: &RequestCallerV1,
        authenticated_key: &[u8; 32],
    ) -> Result<(), InvalidResidentRecordV1> {
        self.0.admit_native_caller_key(caller, authenticated_key)
    }

    fn verify_caller_signature(
        &mut self,
        preimage: &CommonRequestEnvelopePreimageV1,
        digest: &CommonRequestEnvelopeDigest,
        signature: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        self.0.verify_caller_signature(preimage, digest, signature)
    }
}

struct InvokePolicyAdapter<Policy>(Policy);

struct InvokeConsultPolicyAdapter<Policy>(Policy);

struct InvokeConsultDelegatePolicyAdapter<Policy>(Policy);

struct InvokeOnlyPolicy<Policy>(Policy);

impl<Policy> ResidentServiceInvokePolicyPort for InvokeOnlyPolicy<Policy>
where
    Policy: ResidentServiceInvokePolicyPort,
{
    fn evaluate_invoke(
        &mut self,
        context: ResidentServiceInvokePolicyContext<'_>,
    ) -> Result<ResidentServiceOperationStartPolicy, InvalidResidentRecordV1> {
        self.0.evaluate_invoke(context)
    }
}

impl<Policy> ResidentServiceDelegatePolicyPort for InvokeOnlyPolicy<Policy> {
    fn evaluate_delegate(
        &mut self,
        _: ResidentServiceDelegatePolicyContext<'_>,
    ) -> Result<ResidentServiceOperationStartPolicy, InvalidResidentRecordV1> {
        Err(InvalidResidentRecordV1::ContextualAdmission {
            record: "resident operation policy",
            invariant: "Delegate is not enabled for this resident service",
        })
    }
}

fn operation_lease_basis(
    policy: ResidentServiceOperationLeasePolicy,
) -> super::ResidentProcessOperationLeaseBasis {
    super::ResidentProcessOperationLeaseBasis {
        capabilities: policy.capabilities,
        effects: policy.effects,
        read_set: policy.read_set,
        evidence_obligations: policy.evidence_obligations,
        budget_units: policy.budget_units,
        timeout_millis: policy.timeout_millis,
        expires_at: policy.expires_at,
        invocation_correlation: policy.invocation_correlation,
    }
}

fn compiled_operation_start(
    policy: &ResidentServiceOperationStartPolicy,
) -> super::start_factories::ResidentCompiledOperationStartPolicy {
    super::start_factories::ResidentCompiledOperationStartPolicy::from_compiled_projection(
        policy.checkpoint_profile.clone(),
        policy.publication_policy.clone(),
        policy.maximum_record_bytes,
        policy.handle_expires_at,
        policy.handle_retain_until,
        policy.accumulator_limits,
    )
}

fn evaluate_service_consult_policy<Policy>(
    policy_owner: &mut Policy,
    context: super::ResidentProcessConsultPolicyContext<'_>,
) -> Result<super::ResidentProcessConsultPolicy, ResidentProcessHostSelectionError>
where
    Policy: ResidentServiceConsultPolicyPort,
{
    let policy = policy_owner
        .evaluate_consult(ResidentServiceConsultPolicyContext {
            request: context.request,
            session: context.session,
            operation: context.operation,
        })
        .map_err(|_| ResidentProcessHostSelectionError::UnknownDeploymentAuthority)?;
    let parent_start = compiled_operation_start(&policy.parent);
    let parent_lease = operation_lease_basis(policy.parent.lease);
    let mut slots = Vec::with_capacity(policy.roster.slots.len());
    for slot in policy.roster.slots {
        let source = match slot.source {
            ResidentServiceConsultationRosterSource::EvidenceResolved { evidence } => {
                crate::resident::consultation::ConsultationRosterTemplateSource::EvidenceResolved {
                    evidence,
                }
            }
            ResidentServiceConsultationRosterSource::FreshChild {
                request,
                operation_class,
                policy,
            } => {
                if !matches!(
                    operation_class,
                    OperationClassV1::Invoke | OperationClassV1::Consult
                ) {
                    return Err(ResidentProcessHostSelectionError::UnknownDeploymentAuthority);
                }
                let start = compiled_operation_start(&policy);
                let lease = operation_lease_basis(policy.lease);
                crate::resident::consultation::ConsultationRosterTemplateSource::FreshChild {
                    child: super::ResidentProcessConsultFreshChildPolicy {
                        request,
                        operation_class,
                        start,
                        lease,
                    },
                }
            }
        };
        slots.push(
            crate::resident::consultation::ConsultationRosterTemplateSlot::new(
                slot.slot_id,
                slot.requirement,
                slot.requirement_evidence,
                slot.participant,
                slot.provider_binding,
                slot.context,
                source,
            ),
        );
    }
    let roster = crate::resident::consultation::ConsultationRosterTemplate::new(
        policy.roster.completion_policy,
        policy.roster.opened_at,
        policy.roster.deadline,
        slots,
        policy.roster.limits,
    )
    .map_err(|_| ResidentProcessHostSelectionError::UnknownDeploymentAuthority)?;
    Ok(super::ResidentProcessConsultPolicy {
        start: super::start_factories::ResidentStartConsultPolicy::from_compiled_projection(
            parent_start,
            roster,
        ),
        lease: parent_lease,
    })
}

fn evaluate_service_delegate_policy<Policy>(
    policy_owner: &mut Policy,
    context: super::ResidentProcessDelegatePolicyContext<'_>,
) -> Result<super::ResidentProcessDelegatePolicy, ResidentProcessHostSelectionError>
where
    Policy: ResidentServiceDelegatePolicyPort,
{
    let policy = policy_owner
        .evaluate_delegate(ResidentServiceDelegatePolicyContext {
            request: context.request,
            session: context.session,
            operation: context.operation,
            parent: context.parent,
        })
        .map_err(|_| ResidentProcessHostSelectionError::UnknownDeploymentAuthority)?;
    Ok(super::ResidentProcessDelegatePolicy {
        start: super::start_factories::ResidentStartDelegatePolicy::from_compiled_projection(
            compiled_operation_start(&policy),
        ),
        lease: operation_lease_basis(policy.lease),
    })
}

fn evaluate_service_consult_completion<Policy>(
    policy_owner: &mut Policy,
    completion_policy: &ImmutableRecordReferenceV1,
    frontier: &ResidentConsultationPolicyFrontier,
) -> Result<ResidentConsultationSupervisionDecision<Policy::StopInput>, InvalidResidentRecordV1>
where
    Policy: ResidentServiceConsultPolicyPort,
    Policy::StopInput: ResidentServiceConsultStopInput,
{
    match policy_owner.evaluate_consult_completion(ResidentServiceConsultCompletionContext {
        completion_policy,
        current: frontier.current(),
        observed_at: frontier.observed_at(),
    })? {
        ResidentServiceConsultDecision::Continue(candidate) => {
            Ok(ResidentConsultationSupervisionDecision::Continue(candidate))
        }
        ResidentServiceConsultDecision::Stop(input) => {
            Ok(ResidentConsultationSupervisionDecision::Stop(input))
        }
    }
}

impl<Policy> ResidentProcessInvokePolicyEvaluator for InvokePolicyAdapter<Policy>
where
    Policy: ResidentServiceInvokePolicyPort,
{
    fn evaluate_invoke(
        &mut self,
        context: ResidentProcessInvokePolicyContext<'_>,
    ) -> Result<ResidentProcessInvokePolicy, ResidentProcessHostSelectionError> {
        let policy = self
            .0
            .evaluate_invoke(ResidentServiceInvokePolicyContext {
                request: context.request,
                session: context.session,
                operation: context.operation,
            })
            .map_err(|_| ResidentProcessHostSelectionError::UnknownDeploymentAuthority)?;
        let ResidentServiceOperationLeasePolicy {
            capabilities,
            effects,
            read_set,
            evidence_obligations,
            budget_units,
            timeout_millis,
            expires_at,
            invocation_correlation,
        } = policy.lease;
        Ok(ResidentProcessInvokePolicy {
            checkpoint_profile: policy.checkpoint_profile,
            publication_policy: policy.publication_policy,
            maximum_record_bytes: policy.maximum_record_bytes,
            lease: super::ResidentProcessOperationLeaseBasis {
                capabilities,
                effects,
                read_set,
                evidence_obligations,
                budget_units,
                timeout_millis,
                expires_at,
                invocation_correlation,
            },
            handle_expires_at: policy.handle_expires_at,
            handle_retain_until: policy.handle_retain_until,
            accumulator_limits: policy.accumulator_limits,
        })
    }
}

impl<Policy> super::ResidentProcessConsultPolicyEvaluator for InvokePolicyAdapter<Policy> {
    fn evaluate_consult(
        &mut self,
        _: super::ResidentProcessConsultPolicyContext<'_>,
    ) -> Result<super::ResidentProcessConsultPolicy, ResidentProcessHostSelectionError> {
        Err(ResidentProcessHostSelectionError::UnknownDeploymentAuthority)
    }
}

impl<Policy> ResidentProcessInvokePolicyEvaluator for InvokeConsultPolicyAdapter<Policy>
where
    Policy: ResidentServiceInvokePolicyPort,
{
    fn evaluate_invoke(
        &mut self,
        context: ResidentProcessInvokePolicyContext<'_>,
    ) -> Result<ResidentProcessInvokePolicy, ResidentProcessHostSelectionError> {
        let policy = self
            .0
            .evaluate_invoke(ResidentServiceInvokePolicyContext {
                request: context.request,
                session: context.session,
                operation: context.operation,
            })
            .map_err(|_| ResidentProcessHostSelectionError::UnknownDeploymentAuthority)?;
        let lease = operation_lease_basis(policy.lease);
        Ok(ResidentProcessInvokePolicy {
            checkpoint_profile: policy.checkpoint_profile,
            publication_policy: policy.publication_policy,
            maximum_record_bytes: policy.maximum_record_bytes,
            lease,
            handle_expires_at: policy.handle_expires_at,
            handle_retain_until: policy.handle_retain_until,
            accumulator_limits: policy.accumulator_limits,
        })
    }
}

impl<Policy> super::ResidentProcessConsultPolicyEvaluator for InvokeConsultPolicyAdapter<Policy>
where
    Policy: ResidentServiceConsultPolicyPort,
{
    fn evaluate_consult(
        &mut self,
        context: super::ResidentProcessConsultPolicyContext<'_>,
    ) -> Result<super::ResidentProcessConsultPolicy, ResidentProcessHostSelectionError> {
        let policy = self
            .0
            .evaluate_consult(ResidentServiceConsultPolicyContext {
                request: context.request,
                session: context.session,
                operation: context.operation,
            })
            .map_err(|_| ResidentProcessHostSelectionError::UnknownDeploymentAuthority)?;
        let parent_start = compiled_operation_start(&policy.parent);
        let parent_lease = operation_lease_basis(policy.parent.lease);
        let mut slots = Vec::with_capacity(policy.roster.slots.len());
        for slot in policy.roster.slots {
            let source = match slot.source {
                ResidentServiceConsultationRosterSource::EvidenceResolved { evidence } => {
                    crate::resident::consultation::ConsultationRosterTemplateSource::EvidenceResolved {
                        evidence,
                    }
                }
                ResidentServiceConsultationRosterSource::FreshChild {
                    request,
                    operation_class,
                    policy,
                } => {
                    if !matches!(
                        operation_class,
                        OperationClassV1::Invoke | OperationClassV1::Consult
                    ) {
                        return Err(
                            ResidentProcessHostSelectionError::UnknownDeploymentAuthority,
                        );
                    }
                    let start = compiled_operation_start(&policy);
                    let lease = operation_lease_basis(policy.lease);
                    crate::resident::consultation::ConsultationRosterTemplateSource::FreshChild {
                        child: super::ResidentProcessConsultFreshChildPolicy {
                            request,
                            operation_class,
                            start,
                            lease,
                        },
                    }
                }
            };
            slots.push(
                crate::resident::consultation::ConsultationRosterTemplateSlot::new(
                    slot.slot_id,
                    slot.requirement,
                    slot.requirement_evidence,
                    slot.participant,
                    slot.provider_binding,
                    slot.context,
                    source,
                ),
            );
        }
        let roster = crate::resident::consultation::ConsultationRosterTemplate::new(
            policy.roster.completion_policy,
            policy.roster.opened_at,
            policy.roster.deadline,
            slots,
            policy.roster.limits,
        )
        .map_err(|_| ResidentProcessHostSelectionError::UnknownDeploymentAuthority)?;
        Ok(super::ResidentProcessConsultPolicy {
            start: super::start_factories::ResidentStartConsultPolicy::from_compiled_projection(
                parent_start,
                roster,
            ),
            lease: parent_lease,
        })
    }
}

impl<Policy> super::ResidentProcessDelegatePolicyEvaluator for InvokeConsultPolicyAdapter<Policy> {
    fn evaluate_delegate(
        &mut self,
        _: super::ResidentProcessDelegatePolicyContext<'_>,
    ) -> Result<super::ResidentProcessDelegatePolicy, ResidentProcessHostSelectionError> {
        Err(ResidentProcessHostSelectionError::UnknownDeploymentAuthority)
    }
}

impl<Policy> ResidentProcessInvokePolicyEvaluator for InvokeConsultDelegatePolicyAdapter<Policy>
where
    Policy: ResidentServiceInvokePolicyPort,
{
    fn evaluate_invoke(
        &mut self,
        context: ResidentProcessInvokePolicyContext<'_>,
    ) -> Result<ResidentProcessInvokePolicy, ResidentProcessHostSelectionError> {
        let policy = self
            .0
            .evaluate_invoke(ResidentServiceInvokePolicyContext {
                request: context.request,
                session: context.session,
                operation: context.operation,
            })
            .map_err(|_| ResidentProcessHostSelectionError::UnknownDeploymentAuthority)?;
        Ok(ResidentProcessInvokePolicy {
            checkpoint_profile: policy.checkpoint_profile,
            publication_policy: policy.publication_policy,
            maximum_record_bytes: policy.maximum_record_bytes,
            lease: operation_lease_basis(policy.lease),
            handle_expires_at: policy.handle_expires_at,
            handle_retain_until: policy.handle_retain_until,
            accumulator_limits: policy.accumulator_limits,
        })
    }
}

impl<Policy> super::ResidentProcessConsultPolicyEvaluator
    for InvokeConsultDelegatePolicyAdapter<Policy>
where
    Policy: ResidentServiceConsultPolicyPort,
{
    fn evaluate_consult(
        &mut self,
        context: super::ResidentProcessConsultPolicyContext<'_>,
    ) -> Result<super::ResidentProcessConsultPolicy, ResidentProcessHostSelectionError> {
        evaluate_service_consult_policy(&mut self.0, context)
    }
}

impl<Policy> super::ResidentProcessDelegatePolicyEvaluator
    for InvokeConsultDelegatePolicyAdapter<Policy>
where
    Policy: ResidentServiceDelegatePolicyPort,
{
    fn evaluate_delegate(
        &mut self,
        context: super::ResidentProcessDelegatePolicyContext<'_>,
    ) -> Result<super::ResidentProcessDelegatePolicy, ResidentProcessHostSelectionError> {
        evaluate_service_delegate_policy(&mut self.0, context)
    }
}

impl<Policy> consult_recovery::ResidentConsultationPolicyEvaluator<Policy::StopInput>
    for InvokeConsultDelegatePolicyAdapter<Policy>
where
    Policy: ResidentServiceConsultPolicyPort,
    Policy::StopInput: ResidentServiceConsultStopInput,
{
    type Error = InvalidResidentRecordV1;

    fn evaluate(
        &mut self,
        completion_policy: &ImmutableRecordReferenceV1,
        frontier: &ResidentConsultationPolicyFrontier,
    ) -> Result<ResidentConsultationSupervisionDecision<Policy::StopInput>, Self::Error> {
        evaluate_service_consult_completion(&mut self.0, completion_policy, frontier)
    }
}

impl<Policy> consult_recovery::ResidentConsultationPolicyEvaluator<Policy::StopInput>
    for InvokeConsultPolicyAdapter<Policy>
where
    Policy: ResidentServiceConsultPolicyPort,
    Policy::StopInput: ResidentServiceConsultStopInput,
{
    type Error = InvalidResidentRecordV1;

    fn evaluate(
        &mut self,
        completion_policy: &ImmutableRecordReferenceV1,
        frontier: &ResidentConsultationPolicyFrontier,
    ) -> Result<ResidentConsultationSupervisionDecision<Policy::StopInput>, Self::Error> {
        match self
            .0
            .evaluate_consult_completion(ResidentServiceConsultCompletionContext {
                completion_policy,
                current: frontier.current(),
                observed_at: frontier.observed_at(),
            })? {
            ResidentServiceConsultDecision::Continue(candidate) => {
                Ok(ResidentConsultationSupervisionDecision::Continue(candidate))
            }
            ResidentServiceConsultDecision::Stop(input) => {
                Ok(ResidentConsultationSupervisionDecision::Stop(input))
            }
        }
    }
}

impl<StopInput> program::ResidentProgramConsultStopInput for StopInput
where
    StopInput: ResidentServiceConsultStopInput,
{
    fn stop_meaning(&self) -> Option<program::ResidentProgramConsultStopMeaning> {
        let meaning = ResidentServiceConsultStopInput::stop_meaning(self);
        Some(program::ResidentProgramConsultStopMeaning {
            cause: meaning.cause,
            reason: meaning.reason,
            stopped_at: meaning.stopped_at,
        })
    }
}

impl<Policy> super::ResidentProcessDelegatePolicyEvaluator for InvokePolicyAdapter<Policy>
where
    Policy: ResidentServiceDelegatePolicyPort,
{
    fn evaluate_delegate(
        &mut self,
        context: super::ResidentProcessDelegatePolicyContext<'_>,
    ) -> Result<super::ResidentProcessDelegatePolicy, ResidentProcessHostSelectionError> {
        let policy = self
            .0
            .evaluate_delegate(ResidentServiceDelegatePolicyContext {
                request: context.request,
                session: context.session,
                operation: context.operation,
                parent: context.parent,
            })
            .map_err(|_| ResidentProcessHostSelectionError::UnknownDeploymentAuthority)?;
        let ResidentServiceOperationLeasePolicy {
            capabilities,
            effects,
            read_set,
            evidence_obligations,
            budget_units,
            timeout_millis,
            expires_at,
            invocation_correlation,
        } = policy.lease;
        Ok(super::ResidentProcessDelegatePolicy {
            start: super::start_factories::ResidentStartDelegatePolicy::from_compiled_projection(
                super::start_factories::ResidentCompiledOperationStartPolicy::from_compiled_projection(
                    policy.checkpoint_profile,
                    policy.publication_policy,
                    policy.maximum_record_bytes,
                    policy.handle_expires_at,
                    policy.handle_retain_until,
                    policy.accumulator_limits,
                ),
            ),
            lease: super::ResidentProcessOperationLeaseBasis {
                capabilities,
                effects,
                read_set,
                evidence_obligations,
                budget_units,
                timeout_millis,
                expires_at,
                invocation_correlation,
            },
        })
    }
}

impl<Policy> consult_recovery::ResidentConsultationPolicyEvaluator<()>
    for InvokePolicyAdapter<Policy>
{
    type Error = InvalidResidentRecordV1;

    fn evaluate(
        &mut self,
        _: &ImmutableRecordReferenceV1,
        _: &ResidentConsultationPolicyFrontier,
    ) -> Result<ResidentConsultationSupervisionDecision<()>, Self::Error> {
        Err(InvalidResidentRecordV1::ContextualAdmission {
            record: "resident operation policy",
            invariant: "Consult continuation is not yet available through this service",
        })
    }
}

type ResidentServiceRetainedConsultTurn<Inputs, StopInput> = program::ResidentProgramConsultTurn<
    <Inputs as ResidentServiceConsultInputPort>::Work,
    <Inputs as ResidentServiceConsultInputPort>::Output,
    <Inputs as ResidentServiceConsultInputPort>::ChildRecoveryError,
    <Inputs as ResidentServiceConsultInputPort>::RetentionFailure,
    StopInput,
>;

type ResidentServiceRetainedConsultTerminalFailure<Inputs> =
    super::ResidentProcessConsultationTerminalJoinFailure<
        <Inputs as ResidentServiceConsultInputPort>::Work,
        <Inputs as ResidentServiceConsultInputPort>::Output,
    >;

fn retained_consult_custody_contains_key<StopCustody, TerminalFailure>(
    stop_custody: Option<&program::ResidentProgramKeyedConsultTurn<StopCustody>>,
    terminal_failure: Option<&program::ResidentProgramKeyedConsultTurn<TerminalFailure>>,
    key: &program::ResidentProgramConsultTurnKey,
) -> bool {
    stop_custody.is_some_and(|retained| retained.key() == key)
        || terminal_failure.is_some_and(|retained| retained.key() == key)
}

fn retain_unique_consult_turn<Turn, StopCustody, TerminalFailure>(
    turns: &mut program::ResidentProgramRetainedConsultTurns<Turn>,
    stop_custody: Option<&program::ResidentProgramKeyedConsultTurn<StopCustody>>,
    terminal_failure: Option<&program::ResidentProgramKeyedConsultTurn<TerminalFailure>>,
    retained: program::ResidentProgramKeyedConsultTurn<Turn>,
) -> Result<(), program::ResidentProgramConsultRetentionFailure<Turn>> {
    let (key, turn) = retained.into_parts();
    if turns.contains(&key)
        || retained_consult_custody_contains_key(stop_custody, terminal_failure, &key)
    {
        return Err(program::ResidentProgramConsultRetentionFailure::Duplicate(
            Box::new(program::ResidentProgramKeyedConsultTurn::new(key, turn)),
        ));
    }
    turns.retain(key, turn)
}

/// Private retention around a public physical Consult input owner.
struct ResidentServiceConsultRecoveryAdapter<Inputs, StopInput>
where
    Inputs: ResidentServiceConsultInputPort,
{
    inputs: Inputs,
    turns: program::ResidentProgramRetainedConsultTurns<
        ResidentServiceRetainedConsultTurn<Inputs, StopInput>,
    >,
    stop_custody: Option<program::ResidentProgramKeyedConsultStopCustody>,
    terminal_failure: Option<
        program::ResidentProgramKeyedConsultTurn<
            ResidentServiceRetainedConsultTerminalFailure<Inputs>,
        >,
    >,
    next_retained_lane: program::ResidentProgramRetainedConsultLane,
    child_turn: Option<
        crate::resident::process_host::consult_child_recovery_sweep::ResidentProcessConsultChildRecoveryTurn,
    >,
    child_terminal_custody: Option<
        Box<
            crate::resident::process_host::consult_child_recovery_sweep::ResidentProcessConsultChildTerminalCustodyPending,
        >,
    >,
}

impl<Inputs, StopInput> ResidentServiceConsultRecoveryAdapter<Inputs, StopInput>
where
    Inputs: ResidentServiceConsultInputPort,
{
    fn new(inputs: Inputs, maximum_turns: NonZeroUsize) -> Self {
        Self {
            inputs,
            turns: program::ResidentProgramRetainedConsultTurns::new(maximum_turns),
            stop_custody: None,
            terminal_failure: None,
            next_retained_lane: program::ResidentProgramRetainedConsultLane::StopCustody,
            child_turn: None,
            child_terminal_custody: None,
        }
    }
}

fn consult_control_association(
    association: ResidentServiceConsultControlAssociation<'_>,
) -> super::ResidentOperationControlAssociation<'_> {
    super::ResidentOperationControlAssociation {
        current_request_authority: association.current_request_authority,
        historical_handle_authority: association.historical_handle_authority,
        recovery_store: association.recovery_store,
        checkpoint_authority: association.checkpoint_authority,
    }
}

impl<Inputs, StopInput> program::ResidentProgramConsultRecoveryInputPort<StopInput>
    for ResidentServiceConsultRecoveryAdapter<Inputs, StopInput>
where
    Inputs: ResidentServiceConsultInputPort,
    StopInput: ResidentServiceConsultStopInput,
{
    type Work = Inputs::Work;
    type Output = Inputs::Output;
    type ChildRecoveryError = Inputs::ChildRecoveryError;
    type RetentionFailure = Inputs::RetentionFailure;
    type Executor = ResidentServiceConsultExecutor<Inputs::Executor>;
    type StoreResolver = Inputs::StoreResolver;
    type AdmitCollection = Inputs::AdmitCollection;
    type RecoverChild = Inputs::RecoverChild;
    type RetainChild = Inputs::RetainChild;

    fn with_consult_inputs<Return, UseInputs>(
        &mut self,
        use_inputs: UseInputs,
    ) -> Result<Return, ResidentOperationFrameFailure>
    where
        UseInputs: FnOnce(
            crate::resident::ConsultationCollectionLimits,
            crate::resident::consultation::ResidentConsultationTaskLimits,
            &Self::StoreResolver,
            &mut Self::AdmitCollection,
            &mut Self::Executor,
            &mut Self::RecoverChild,
            &mut Self::RetainChild,
            Option<&mut crate::resident::HistoricalConsultationAdmission<'_>>,
        ) -> Return,
    {
        self.inputs.with_finite_turn_inputs(
            |collection_limits,
             task_limits,
             resolve_store,
             admit_collection,
             executor,
             recover_child,
             retain_child,
             historical_collection| {
                use_inputs(
                    collection_limits,
                    crate::resident::consultation::ResidentConsultationTaskLimits::new(
                        task_limits.maximum_in_flight,
                        task_limits.maximum_batch_members,
                    ),
                    resolve_store,
                    admit_collection,
                    executor,
                    recover_child,
                    retain_child,
                    historical_collection,
                )
            },
        )
    }

    fn with_initial_collection_inputs<Return, UseInputs>(
        &mut self,
        selector: &super::ResidentDeploymentAuthoritySelector,
        parent: &ControlRecordCoordinate,
        use_inputs: UseInputs,
    ) -> Result<Return, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(
            program::ResidentProgramConsultInitialCollectionInputs<'input>,
        ) -> Return,
    {
        self.inputs.with_initial_collection_inputs(
            &service_deployment_selector(selector),
            parent,
            |inputs| {
                use_inputs(program::ResidentProgramConsultInitialCollectionInputs {
                    observation_ref: inputs.observation_ref,
                    observed_at: inputs.observed_at,
                    signing: inputs.signing,
                    checkpoint: inputs.checkpoint,
                    resolve_store: inputs.resolve_store,
                    admit_collection: inputs.admit_collection,
                    admit_rotation: inputs.admit_rotation,
                    sign_observation: inputs.sign_observation,
                    sign_checkpoint: inputs.sign_checkpoint,
                })
            },
        )
    }

    fn with_consult_stop_inputs<Return, UseInputs>(
        &mut self,
        key: &program::ResidentProgramConsultTurnKey,
        meaning: &program::ResidentProgramConsultStopMeaning,
        use_inputs: UseInputs,
    ) -> Result<Return, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(program::ResidentProgramConsultStopInputs<'input>) -> Return,
    {
        let public_meaning = ResidentServiceConsultStopMeaning {
            cause: meaning.cause,
            reason: meaning.reason.clone(),
            stopped_at: meaning.stopped_at,
        };
        self.inputs.with_stop_inputs(
            &service_deployment_selector(key.selector()),
            key.parent(),
            &public_meaning,
            |inputs| {
                use_inputs(program::ResidentProgramConsultStopInputs {
                    association: consult_control_association(inputs.association),
                    parent: inputs.parent,
                    parent_observation: inputs.parent_observation,
                    parent_checkpoint: inputs.parent_checkpoint,
                    observation_ref: inputs.observation_ref,
                    observed_at: inputs.observed_at,
                    signing: inputs.signing,
                    limits: inputs.limits,
                    checkpoint: inputs.checkpoint,
                    resolve_store: inputs.resolve_store,
                    admit_stop: inputs.admit_stop,
                    admit_collection: inputs.admit_collection,
                    admit_rotation: inputs.admit_rotation,
                    sign_observation: inputs.sign_observation,
                    sign_checkpoint: inputs.sign_checkpoint,
                })
            },
        )
    }

    fn with_consult_join_inputs<Return, UseInputs>(
        &mut self,
        selection: &program::ResidentProgramConsultJoinSelection,
        use_inputs: UseInputs,
    ) -> Result<Return, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(program::ResidentProgramConsultJoinInputs<'input>) -> Return,
    {
        self.inputs.with_completion_join_inputs(
            ResidentServiceConsultJoinSelection {
                deployment: service_deployment_selector(selection.parent.selector()),
                parent: selection.parent.parent(),
                slot_id: &selection.slot_id,
                child: &selection.child,
                dispatch: &selection.dispatch,
            },
            |inputs| {
                use_inputs(program::ResidentProgramConsultJoinInputs {
                    association: consult_control_association(inputs.association),
                    input: inputs.input,
                    checkpoint: inputs.checkpoint,
                    resolve_store: inputs.resolve_store,
                    admit_evidence: inputs.admit_evidence,
                    admit_collection: inputs.admit_collection,
                    admit_rotation: inputs.admit_rotation,
                    sign_observation: inputs.sign_observation,
                    sign_checkpoint: inputs.sign_checkpoint,
                })
            },
        )
    }

    fn take_consult_stop_custody(
        &mut self,
    ) -> Option<program::ResidentProgramKeyedConsultStopCustody> {
        self.stop_custody.take()
    }

    fn retain_consult_stop_custody(
        &mut self,
        pending: program::ResidentProgramKeyedConsultStopCustody,
    ) {
        assert!(
            self.stop_custody.replace(pending).is_none(),
            "Consult stop custody must remain linear",
        );
    }

    fn take_consult_terminal_failure(
        &mut self,
    ) -> Option<
        program::ResidentProgramKeyedConsultTurn<
            super::ResidentProcessConsultationTerminalJoinFailure<Self::Work, Self::Output>,
        >,
    > {
        self.terminal_failure.take()
    }

    fn retain_consult_terminal_failure(
        &mut self,
        failure: program::ResidentProgramKeyedConsultTurn<
            super::ResidentProcessConsultationTerminalJoinFailure<Self::Work, Self::Output>,
        >,
    ) {
        assert!(
            self.terminal_failure.replace(failure).is_none(),
            "Consult terminal join custody must remain linear",
        );
    }

    fn retain_consult_turn(
        &mut self,
        _turn: ResidentServiceRetainedConsultTurn<Inputs, StopInput>,
    ) {
        unreachable!("the resident service retains every Consult continuation by parent key")
    }

    fn retain_keyed_consult_turn(
        &mut self,
        retained: program::ResidentProgramKeyedConsultTurn<
            ResidentServiceRetainedConsultTurn<Inputs, StopInput>,
        >,
    ) -> Result<
        (),
        program::ResidentProgramConsultRetentionFailure<
            ResidentServiceRetainedConsultTurn<Inputs, StopInput>,
        >,
    > {
        retain_unique_consult_turn(
            &mut self.turns,
            self.stop_custody.as_ref(),
            self.terminal_failure.as_ref(),
            retained,
        )
    }

    fn has_runnable_retained_consult(&self) -> bool {
        self.stop_custody.is_some() || self.terminal_failure.is_some() || self.turns.has_retained()
    }

    fn has_retained_consult_key(&self, key: &program::ResidentProgramConsultTurnKey) -> bool {
        self.turns.contains(key)
            || retained_consult_custody_contains_key(
                self.stop_custody.as_ref(),
                self.terminal_failure.as_ref(),
                key,
            )
    }

    fn can_retain_consult_key(&self, key: &program::ResidentProgramConsultTurnKey) -> bool {
        !self.has_retained_consult_key(key) && self.turns.has_capacity()
    }

    fn has_consult_turn_capacity(&self) -> bool {
        self.turns.has_capacity()
    }

    fn consult_retention_admission(&self) -> program::ResidentProgramConsultRetentionAdmission {
        let mut retained_keys = self.turns.keys();
        if let Some(retained) = self.stop_custody.as_ref() {
            retained_keys.push(retained.key().clone());
        }
        if let Some(retained) = self.terminal_failure.as_ref() {
            retained_keys.push(retained.key().clone());
        }
        program::ResidentProgramConsultRetentionAdmission::new(
            self.turns.has_capacity(),
            retained_keys,
        )
    }

    fn next_retained_consult_lane(&mut self) -> program::ResidentProgramRetainedConsultLane {
        let selected = self.next_retained_lane;
        self.next_retained_lane = selected.next();
        selected
    }

    fn take_runnable_retained_consult(
        &mut self,
    ) -> Option<
        program::ResidentProgramKeyedConsultTurn<
            ResidentServiceRetainedConsultTurn<Inputs, StopInput>,
        >,
    > {
        self.turns.take_next()
    }

    fn restore_taken_consult_turn(
        &mut self,
        retained: program::ResidentProgramKeyedConsultTurn<
            ResidentServiceRetainedConsultTurn<Inputs, StopInput>,
        >,
    ) {
        let (key, turn) = retained.into_parts();
        self.turns
            .retain(key, turn)
            .unwrap_or_else(|_| unreachable!("a taken Consult slot must remain available"));
    }

    fn retain_consult_child_turn(
        &mut self,
        turn: crate::resident::process_host::consult_child_recovery_sweep::ResidentProcessConsultChildRecoveryTurn,
    ) {
        assert!(
            self.child_turn.replace(turn).is_none(),
            "Consult child recovery must remain linear",
        );
    }

    fn take_consult_child_turn(
        &mut self,
    ) -> Option<
        crate::resident::process_host::consult_child_recovery_sweep::ResidentProcessConsultChildRecoveryTurn,
    >{
        self.child_turn.take()
    }

    fn with_consult_child_terminal_inputs<Return, UseInputs>(
        &mut self,
        use_inputs: UseInputs,
    ) -> Result<Return, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(
            program::ResidentProgramConsultChildTerminalInputs<'input>,
        ) -> Return,
    {
        self.inputs
            .with_terminal_child_join_inputs(|inputs| {
                use_inputs(program::ResidentProgramConsultChildTerminalInputs {
                    input: crate::resident::process_host::consult_child_recovery_sweep::ResidentProcessConsultChildTerminalCommitInput {
                        association: consult_control_association(inputs.association),
                        recorded_at: inputs.recorded_at,
                        observation_ref: inputs.observation_ref,
                        observed_at: inputs.observed_at,
                        signing: inputs.signing,
                        limits: inputs.limits,
                        checkpoint: inputs.checkpoint,
                    },
                    resolve_store: inputs.resolve_store,
                    admit_evidence: inputs.admit_evidence,
                    admit_collection: inputs.admit_collection,
                    admit_rotation: inputs.admit_rotation,
                    sign_observation: inputs.sign_observation,
                    sign_checkpoint: inputs.sign_checkpoint,
                })
            })
    }

    fn take_consult_child_terminal_custody(
        &mut self,
    ) -> Option<
        Box<
            crate::resident::process_host::consult_child_recovery_sweep::ResidentProcessConsultChildTerminalCustodyPending,
        >,
    >{
        self.child_terminal_custody.take()
    }

    fn retain_consult_child_terminal_custody(
        &mut self,
        pending: Box<
            crate::resident::process_host::consult_child_recovery_sweep::ResidentProcessConsultChildTerminalCustodyPending,
        >,
    ) {
        assert!(
            self.child_terminal_custody.replace(pending).is_none(),
            "Consult child terminal custody must remain linear",
        );
    }
}

struct InvokeCryptographyAdapter<Cryptography>(Cryptography);

impl<Cryptography> program::ResidentProgramInvokeCryptographyPort
    for InvokeCryptographyAdapter<Cryptography>
where
    Cryptography: ResidentServiceInvokeCryptographyPort,
{
    fn admit_compiled_authority(
        &self,
        request: &CommonRequestEnvelopeV1,
        session: &crate::resident::AdmittedAmbientSession,
        current: &crate::resident::AmbientSessionAdmissionContext<'_>,
        handle: &OperationHandlePreimageV1,
        lease: &OperationLeaseV1,
        operation: &crate::semantic::ResolvedOperation,
        publication: &crate::resident::LocalOperationPublicationPolicyInput<'_>,
        checkpoint: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    ) -> Result<(), InvalidResidentRecordV1> {
        self.0.admit_compiled_authority(
            request,
            session,
            current,
            handle,
            lease,
            operation,
            publication,
            checkpoint,
        )
    }

    fn sign_handle(
        &self,
        bytes: &[u8],
        preimage: &OperationHandlePreimageV1,
        digest: &OperationHandleDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1> {
        self.0.sign_handle(bytes, preimage, digest)
    }

    fn sign_observation(
        &self,
        bytes: &[u8],
        preimage: &OperationObservationPreimageV1,
        digest: &OperationObservationDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1> {
        self.0.sign_observation(bytes, preimage, digest)
    }

    fn prepare_checkpoint(
        &self,
        prepared: &crate::resident::PreparedLocalOperationCheckpoint,
        checkpoint_id: &ControlCheckpointIdV1,
    ) -> Result<ControlStateCheckpointPreimageV1, crate::resident::LocalOperationStartError> {
        self.0.prepare_checkpoint(prepared, checkpoint_id)
    }

    fn sign_checkpoint(
        &self,
        bytes: &[u8],
        preimage: &ControlStateCheckpointPreimageV1,
        digest: &ControlStateCheckpointDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1> {
        self.0.sign_checkpoint(bytes, preimage, digest)
    }
}

struct PeerAdmissionAdapter<'a>(Box<dyn ResidentPeerAdmissionPort + 'a>);

impl program::ResidentProgramPeerAdmissionPort for PeerAdmissionAdapter<'_> {
    type Witness = ResidentPeerAdmissionWitness;

    fn admit_hook_peer(
        &mut self,
        peer: LocalIpcPeerCredentials,
    ) -> Result<Self::Witness, InvalidResidentRecordV1> {
        self.0.admit_peer(peer)
    }

    fn admit_peer(
        &mut self,
        peer: LocalIpcPeerCredentials,
        _: &ResidentOperationFrameRequest,
    ) -> Result<Self::Witness, InvalidResidentRecordV1> {
        self.0.admit_peer(peer)
    }
}

impl program::ResidentProgramObservePeerAdmissionPort for PeerAdmissionAdapter<'_> {
    fn admit_observe_peer(
        &mut self,
        peer: LocalIpcPeerCredentials,
        _: &ResidentObserveFrameRequest,
    ) -> Result<Self::Witness, InvalidResidentRecordV1> {
        self.0.admit_peer(peer)
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/resident/process_host/service_assembly_peer_admission.rs"]
mod peer_admission_tests;

struct ObserveInputAdapter<Inputs>(Inputs);

impl<'recovery, Inputs> program::ResidentProgramObserveInputPort<'recovery>
    for ObserveInputAdapter<Inputs>
where
    Inputs: ResidentServiceObserveInputPort<'recovery>,
{
    fn with_observe_inputs<Output, UseInputs>(
        &mut self,
        locator: &ControlRecordCoordinate,
        use_inputs: UseInputs,
    ) -> Result<Output, ResidentObserveFrameFailure>
    where
        UseInputs: for<'request> FnOnce(
            program::ResidentProgramObserveInputs<'request, 'recovery>,
        ) -> Output,
    {
        self.0.with_observe_inputs(locator, |inputs| {
            let mut replay = |request_id: &ResidentRequestIdV1,
                              digest: &CommonRequestEnvelopeDigest,
                              bytes: &[u8]| {
                (inputs.admit_replay)(request_id, digest, bytes).map(|outcome| match outcome {
                    ResidentServiceObserveReplayDisposition::FirstAdmission => {
                        super::observe_request::ResidentObserveReplayDisposition::FirstAdmission
                    }
                    ResidentServiceObserveReplayDisposition::ExactReplay => {
                        super::observe_request::ResidentObserveReplayDisposition::ExactReplay
                    }
                }).map_err(|error| match error {
                    ResidentServiceObserveReplayError::ChangedBytes => super::observe_request::ResidentObserveReplayAdmissionError::ChangedBytes,
                    ResidentServiceObserveReplayError::Unavailable => super::observe_request::ResidentObserveReplayAdmissionError::Unavailable,
                })
            };
            use_inputs(program::ResidentProgramObserveInputs {
                recovery: inputs.recovery,
                verify_caller_signature: inputs.verify_caller_signature,
                admit_replay: &mut replay,
                recovery_admission: inputs.recovery_admission,
                authorize_disclosure: inputs.authorize_disclosure,
                response_deadline: inputs.response_deadline,
            })
        })
    }
}

struct InvokeRecoveryAdapter<
    'service,
    InvokeInputs,
    DelegateInputs,
    CancellationInputs,
    ObserveInputs,
> {
    invoke_inputs: Option<InvokeInputs>,
    publication_inputs: Box<dyn ResidentServicePublicationInputPort + 'service>,
    delegate_inputs: DelegateInputs,
    cancel_inputs: Option<CancellationInputs>,
    observe_inputs: Option<ObserveInputs>,
    retained: BoundedRetainedPhases<RetainedServicePhaseKey, RetainedServicePhase<'service>>,
    recovery_mode: InvokeRecoveryMode,
}

impl<'service, InvokeInputs, DelegateInputs, CancellationInputs, ObserveInputs>
    InvokeRecoveryAdapter<'service, InvokeInputs, DelegateInputs, CancellationInputs, ObserveInputs>
{
    fn new(
        invoke_inputs: InvokeInputs,
        publication_inputs: Box<dyn ResidentServicePublicationInputPort + 'service>,
        delegate_inputs: DelegateInputs,
        cancel_inputs: CancellationInputs,
        observe_inputs: ObserveInputs,
        retained_capacity: usize,
    ) -> Self {
        Self {
            invoke_inputs: Some(invoke_inputs),
            publication_inputs,
            delegate_inputs,
            cancel_inputs: Some(cancel_inputs),
            observe_inputs: Some(observe_inputs),
            retained: BoundedRetainedPhases::new(retained_capacity),
            recovery_mode: InvokeRecoveryMode::Idle,
        }
    }

    fn retain(
        &mut self,
        phase: RetainedInvokePhase,
    ) -> Result<
        operation_request_host::ResidentOperationRequestOutcome,
        ResidentOperationFrameFailure,
    > {
        let outcome =
            operation_request_host::ResidentOperationRequestOutcome::Pending(phase.locator());
        let key = RetainedServicePhaseKey::Invoke(phase.key());
        match self
            .retained
            .retain(key, RetainedServicePhase::Invoke(phase))
        {
            Ok(RetainedPhaseAdmission::Retained) => {}
            Ok(RetainedPhaseAdmission::KeyConflict(_phase)) => {
                return Err(ResidentOperationFrameFailure::ResidentUnavailable);
            }
            Err(_phase) => return Err(ResidentOperationFrameFailure::ResidentUnavailable),
        }
        match self.recovery_mode {
            InvokeRecoveryMode::PointAddressed => Ok(outcome),
            InvokeRecoveryMode::Background | InvokeRecoveryMode::Idle => {
                Err(ResidentOperationFrameFailure::ResidentUnavailable)
            }
        }
    }

    fn retain_without_response(&mut self, phase: RetainedInvokePhase) {
        let key = RetainedServicePhaseKey::Invoke(phase.key());
        match self
            .retained
            .retain(key, RetainedServicePhase::Invoke(phase))
        {
            Ok(RetainedPhaseAdmission::Retained) => {}
            Ok(RetainedPhaseAdmission::KeyConflict(_phase)) => unreachable!(
                "background recovery is not scanned while the same phase key is retained"
            ),
            Err(_) => unreachable!(
                "background recovery is not scanned while retained Invoke custody is full"
            ),
        }
    }
}

struct BoundedRetainedPhases<Key, Phase> {
    entries: Vec<(Key, Option<Phase>)>,
    capacity: usize,
}

impl<Key, Phase> BoundedRetainedPhases<Key, Phase>
where
    Key: Clone + Eq,
{
    fn new(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
            capacity,
        }
    }

    fn retain(&mut self, key: Key, phase: Phase) -> Result<RetainedPhaseAdmission<Phase>, Phase> {
        if self.entries.iter().any(|(retained, _)| retained == &key) {
            return Ok(RetainedPhaseAdmission::KeyConflict(phase));
        }
        if self.entries.len() >= self.capacity {
            return Err(phase);
        }
        self.entries.push((key, Some(phase)));
        Ok(RetainedPhaseAdmission::Retained)
    }

    fn reserve(
        &mut self,
        key: Key,
    ) -> Result<RetainedPhaseReservation<Key>, RetainedPhaseReservationError> {
        if self.entries.iter().any(|(retained, _)| retained == &key) {
            return Err(RetainedPhaseReservationError::KeyConflict);
        }
        if self.entries.len() >= self.capacity {
            return Err(RetainedPhaseReservationError::Full);
        }
        self.entries.push((key.clone(), None));
        Ok(RetainedPhaseReservation { key })
    }

    fn release(&mut self, reservation: RetainedPhaseReservation<Key>) -> bool {
        let Some(index) = self
            .entries
            .iter()
            .position(|(key, phase)| key == &reservation.key && phase.is_none())
        else {
            return false;
        };
        self.entries.swap_remove(index);
        true
    }

    fn retain_reserved(
        &mut self,
        reservation: RetainedPhaseReservation<Key>,
        phase: Phase,
    ) -> Result<(), Phase> {
        let Some((_, retained)) = self
            .entries
            .iter_mut()
            .find(|(key, _)| key == &reservation.key)
        else {
            return Err(phase);
        };
        if retained.is_some() {
            return Err(phase);
        }
        *retained = Some(phase);
        Ok(())
    }

    fn take_first_where(
        &mut self,
        mut selected: impl FnMut(&Phase) -> bool,
    ) -> Option<(RetainedPhaseReservation<Key>, Phase)> {
        let (key, phase) = self
            .entries
            .iter_mut()
            .find(|(_, phase)| phase.as_ref().is_some_and(&mut selected))?;
        Some((
            RetainedPhaseReservation { key: key.clone() },
            phase
                .take()
                .expect("the selected retained phase is present"),
        ))
    }

    fn move_to_back(&mut self, key: &Key) {
        if let Some(index) = self
            .entries
            .iter()
            .position(|(retained, _)| retained == key)
        {
            self.entries[index..].rotate_left(1);
        }
    }

    fn is_full(&self) -> bool {
        self.entries.len() >= self.capacity
    }

    fn iter(&self) -> impl Iterator<Item = &Phase> {
        self.entries.iter().filter_map(|(_, phase)| phase.as_ref())
    }

    fn first_key_where(&self, mut selected: impl FnMut(&Phase) -> bool) -> Option<&Key> {
        self.entries
            .iter()
            .find(|(_, phase)| phase.as_ref().is_some_and(&mut selected))
            .map(|(key, _)| key)
    }
}

#[must_use = "release this reservation or consume it with one retained phase"]
struct RetainedPhaseReservation<Key> {
    key: Key,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RetainedPhaseReservationError {
    KeyConflict,
    Full,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RetainedPhaseAdmission<Phase> {
    Retained,
    KeyConflict(Phase),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InvokeRecoveryMode {
    Idle,
    PointAddressed,
    Background,
}

enum RetainedServicePhase<'service> {
    Invoke(RetainedInvokePhase),
    Cancellation(RetainedCancellationPhase<'service>),
}

impl RetainedServicePhase<'_> {
    fn invoke(&self) -> Option<&RetainedInvokePhase> {
        match self {
            Self::Invoke(phase) => Some(phase),
            Self::Cancellation(_) => None,
        }
    }

    fn is_cancellation(&self) -> bool {
        matches!(self, Self::Cancellation(_))
    }
}

enum RetainedCancellationPhase<'service> {
    A2a(Box<super::ResidentProcessA2aDelegatePoll>),
    A2aOutcome(Box<ResidentProcessA2aDelegateRetentionFailure<'service>>),
    Cancellation(Box<super::ResidentProcessCancellationCustody>),
    Dispatched(Box<super::ResidentProcessDispatchedCancellationCustody>),
    Consultation(Box<super::ResidentProcessConsultationCancellationCustody>),
    ConsultationPreparation(Box<super::ResidentProcessConsultationCancellationPreparation>),
}

impl RetainedCancellationPhase<'_> {
    fn a2a(poll: super::ResidentProcessA2aDelegatePoll) -> Self {
        Self::A2a(Box::new(poll))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum RetainedServicePhaseKey {
    Invoke(RetainedInvokePhaseKey),
    Cancellation(RetainedCancellationPhaseKey),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RetainedCancellationPhaseKey {
    selector: super::ResidentDeploymentAuthoritySelector,
    request_id: ResidentRequestIdV1,
    request_sha256: CommonRequestEnvelopeDigest,
    locator: ControlRecordCoordinate,
}

enum RetainedInvokePhase {
    Deferred(Box<restart::ResidentProcessOperationRestartDisposition>),
    LocalCancellation(Box<restart::ResidentProcessRecoveredLocalCancellation>),
    OperationResolution(Box<restart::ResidentProcessRecoveredOperationResolution>),
    OperationResolutionContinuation(Box<RetainedOperationResolutionPhase>),
    ProviderResolved(Box<restart::ResidentProcessRecoveredProviderResolution>),
    ProviderResolvedContinuation(Box<RetainedProviderResolvedPhase>),
    ProviderPrepared(Box<restart::ResidentProcessRecoveredPreparedAttempt>),
    ProviderPreparedContinuation(Box<RetainedProviderPreparedPhase>),
    ProviderDispatched(Box<restart::ResidentProcessRecoveredAttempt>),
    ProviderRecovery(Box<RetainedProviderRecoveryPhase>),
    NativePrepared(Box<restart::ResidentProcessRecoveredPreparedAttempt>),
    NativePreparedContinuation(Box<RetainedNativePreparedPhase>),
    NativeDispatched(Box<restart::ResidentProcessRecoveredAttempt>),
    NativeRecording(Box<RetainedNativeRecordingPhase>),
    PreDispatchExpiry(Box<RetainedPreDispatchExpiryPhase>),
}

struct RetainedPreDispatchExpiryPhase {
    key: RetainedInvokePhaseKey,
    selector: super::ResidentDeploymentAuthoritySelector,
    locator: ResidentOperationFrameLocator,
    observed_at: UnixTimeV1,
    state: crate::resident::operation_dispatch::OwnedOperationObservationCustodyFailure,
}

enum PreDispatchExpiryTurn {
    NotExpired,
    Complete,
    Pending {
        observed_at: UnixTimeV1,
        state: crate::resident::operation_dispatch::OwnedOperationObservationCustodyFailure,
    },
}

fn run_pre_dispatch_expiry_turn<WriterGuard>(
    process: &ResidentProcessHost<'_, WriterGuard>,
    start: &restart::ResidentProcessRecoveredOperationStart,
    access: &mut super::ResidentDeploymentOperationAccess<'_>,
    inputs: ResidentServicePreDispatchExpiryPreparationInputs<'_>,
) -> Result<PreDispatchExpiryTurn, super::expiry::ResidentProcessPreDispatchExpiryFailure> {
    let ResidentServicePreDispatchExpiryPreparationInputs {
        clock,
        diagnostic_ref,
        observation_ref,
        signing,
        checkpoint,
        checkpoint_schema,
        checkpoint_authority,
        resolve_store,
        maximum_diagnostic_bytes,
        maximum_record_bytes,
        accumulator_limits,
        admit_rotation,
        sign_observation,
        sign_checkpoint,
    } = inputs;
    let prepared = match process.prepare_recovered_pre_dispatch_expiry(
        start,
        reborrow_deployment_operation_access(access),
        super::expiry::ResidentProcessPreDispatchExpiryInput {
            clock,
            diagnostic_ref,
            observation_ref,
            signing,
            maximum_diagnostic_bytes,
            maximum_record_bytes,
        },
        &mut *admit_rotation,
        &mut *sign_observation,
    ) {
        Err(super::expiry::ResidentProcessPreDispatchExpiryFailure::Expiry(
            crate::resident::operation_expiry::PreDispatchOperationExpiryError::NotExpired,
        )) => return Ok(PreDispatchExpiryTurn::NotExpired),
        Err(failure) => return Err(failure),
        Ok(prepared) => prepared,
    };
    let observed_at = prepared.observed_at();
    let checkpoint_predecessor = start.control.observation().current_checkpoint();
    let checkpoint_predecessor = (
        checkpoint_predecessor.0.clone(),
        checkpoint_predecessor.1.clone(),
    );
    let transition = prepared.into_owned_observation();
    Ok(
        match crate::resident::operation_dispatch::retain_owned_operation_observation(
            transition,
            checkpoint_predecessor,
            checkpoint,
            observed_at,
            checkpoint_schema,
            checkpoint_authority,
            resolve_store,
            accumulator_limits,
            access.custody,
            access.anchor,
            &mut *sign_checkpoint,
        ) {
            Ok(_) => PreDispatchExpiryTurn::Complete,
            Err(state) => PreDispatchExpiryTurn::Pending { observed_at, state },
        },
    )
}

struct RetainedOperationResolutionPhase {
    key: RetainedInvokePhaseKey,
    selector: super::ResidentDeploymentAuthoritySelector,
    locator: ResidentOperationFrameLocator,
    state: crate::resident::operation_dispatch::OwnedOperationObservationCustodyFailure,
}

struct RetainedProviderResolvedPhase {
    key: RetainedInvokePhaseKey,
    selector: super::ResidentDeploymentAuthoritySelector,
    locator: ResidentOperationFrameLocator,
    state: RetainedProviderResolvedState,
}

enum RetainedProviderResolvedState {
    Preparation(
        Box<super::provider_resolution_recovery::ResidentProcessPreparedProviderResolution>,
    ),
    Prepared {
        attempt: Box<crate::resident::operation_dispatch::PreparedRecoveredProviderAttempt>,
        checkpoint: crate::resident::OperationCheckpointInput,
    },
    Commit(Box<crate::resident::operation_dispatch::RecoveredProviderAttemptCommitFailure>),
    Anchor(Box<crate::resident::operation_dispatch::RecoveredProviderAttemptAnchorPending>),
}

struct RetainedNativePreparedPhase {
    key: RetainedInvokePhaseKey,
    selector: super::ResidentDeploymentAuthoritySelector,
    locator: ResidentOperationFrameLocator,
    state: RetainedNativePreparedState,
}

enum RetainedNativePreparedState {
    Dispatch(Arc<crate::resident::operation_dispatch::PreparedRecoveredOperationDispatch>),
    Issued(Box<crate::resident::operation_dispatch::IssuedRecoveredDispatchCheckpoint>),
    Commit(Box<crate::resident::operation_dispatch::RecoveredDispatchCommitFailure>),
    Anchor(Box<crate::resident::operation_dispatch::RecoveredDispatchAnchorPending>),
}

struct RetainedProviderPreparedPhase {
    key: RetainedInvokePhaseKey,
    selector: super::ResidentDeploymentAuthoritySelector,
    locator: ResidentOperationFrameLocator,
    state: RetainedProviderPreparedState,
}

enum RetainedProviderPreparedState {
    Dispatch(Box<crate::resident::operation_dispatch::PreparedRecoveredProviderDispatch>),
    Issued {
        dispatch: Box<crate::resident::operation_dispatch::PreparedRecoveredProviderDispatch>,
        pending: Box<crate::resident::operation_dispatch::IssuedRecoveredDispatchCheckpoint>,
    },
    Commit {
        dispatch: Box<crate::resident::operation_dispatch::PreparedRecoveredProviderDispatch>,
        pending: Box<crate::resident::operation_dispatch::RecoveredDispatchCommitFailure>,
    },
    Anchor {
        dispatch: Box<crate::resident::operation_dispatch::PreparedRecoveredProviderDispatch>,
        pending: Box<crate::resident::operation_dispatch::RecoveredDispatchAnchorPending>,
    },
    Release(Box<crate::resident::operation_dispatch::RecoveredProviderDispatchRelease>),
    Launch(Box<super::ResidentProcessRecoveredProviderLaunchFailure>),
}

enum ProviderPreparedTurn {
    Accepted,
    Complete,
}

struct RetainedNativeRecordingPhase {
    key: RetainedInvokePhaseKey,
    selector: super::ResidentDeploymentAuthoritySelector,
    locator: ResidentOperationFrameLocator,
    predecessor_checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    state: RetainedNativeRecordingState,
}

enum RetainedNativeRecordingState {
    Unstaged(Box<crate::resident::operation_dispatch::UnstagedNativeOperationOutcome>),
    StagePreparation(
        Box<crate::resident::operation_dispatch::NativeOperationStagePreparationContinuation>,
    ),
    PreparedStage(Box<crate::resident::operation_dispatch::PreparedNativeOperationStageCustody>),
    IssuedStage(Box<crate::resident::operation_dispatch::IssuedNativeOperationStageCheckpoint>),
    StagingCommit(Box<crate::resident::NativeOperationStagingCommitFailure>),
    StagingAnchor(Box<crate::resident::NativeOperationStagingAnchorPending>),
    StageFinalization(Box<crate::resident::NativeOperationStageFinalization>),
    TerminalPreparation(Box<crate::resident::AdmittedStagedNativeOperationOutcome>),
    TerminalCustody(Box<crate::resident::OperationAppendFailure>),
}

enum RetainedNativeRecordingTurn {
    Complete,
    Continue(RetainedNativeRecordingState),
    RestartRequired,
}

fn reborrow_deployment_operation_access<'borrow, 'store>(
    access: &'borrow mut super::ResidentDeploymentOperationAccess<'store>,
) -> super::ResidentDeploymentOperationAccess<'borrow>
where
    'store: 'borrow,
{
    super::ResidentDeploymentOperationAccess {
        store: access.store,
        custody: &mut *access.custody,
        sessions: access.sessions,
        anchor: &mut *access.anchor,
    }
}

fn continue_retained_native_recording<WriterGuard>(
    process: &ResidentProcessHost<'_, WriterGuard>,
    selector: &super::ResidentDeploymentAuthoritySelector,
    mut access: super::ResidentDeploymentOperationAccess<'_>,
    predecessor_checkpoint: &(ResidentRecordRefV1, ControlStateCheckpointV1),
    state: RetainedNativeRecordingState,
    inputs: ResidentServiceNativeRecoveryInputs<'_>,
) -> RetainedNativeRecordingTurn {
    use crate::resident::operation_dispatch::{
        record_native_operation_outcome, NativeOperationOutcomeRecordingInput,
    };

    let next = match state {
        RetainedNativeRecordingState::Unstaged(outcome) => record_native_operation_outcome(
            *outcome,
            NativeOperationOutcomeRecordingInput {
                checkpoint: (&predecessor_checkpoint.0, &predecessor_checkpoint.1),
                outcome_ref: inputs.outcome_ref,
                outcome_observation_ref: inputs.outcome_observation_ref,
                result_ref: inputs.result_ref,
                diagnostic_ref: inputs.diagnostic_ref,
                terminal_ref: inputs.terminal_ref,
                outcome_checkpoint: inputs.outcome_checkpoint,
                terminal_checkpoint: inputs.terminal_checkpoint,
                clock: inputs.clock,
                resolve_store: inputs.resolve_store,
                checkpoint_schema: inputs.checkpoint_schema,
                checkpoint_authority: inputs.checkpoint_authority,
                signing: inputs.signing,
                maximum_diagnostic_bytes: inputs.maximum_diagnostic_bytes,
                maximum_outcome_bytes: inputs.maximum_outcome_bytes,
                accumulator_limits: inputs.accumulator_limits,
            },
            access.custody,
            access.anchor,
            &mut *inputs.admit_rotation,
            &mut *inputs.sign_observation,
            &mut *inputs.sign_checkpoint,
            &mut *inputs.sign_result,
        )
        .map(|_| RetainedNativeRecordingTurn::Complete)
        .unwrap_or_else(|failure| {
            retained_native_recording_state(failure)
                .map(RetainedNativeRecordingTurn::Continue)
                .unwrap_or(RetainedNativeRecordingTurn::RestartRequired)
        }),
        RetainedNativeRecordingState::StagePreparation(continuation) => {
            match process.continue_native_stage_preparation(
                selector,
                reborrow_deployment_operation_access(&mut access),
                *continuation,
                inputs.signing,
                &mut *inputs.admit_rotation,
                &mut *inputs.sign_observation,
            ) {
                Err(failure) => {
                    let (continuation, _) = failure.into_parts();
                    RetainedNativeRecordingTurn::Continue(
                        RetainedNativeRecordingState::StagePreparation(Box::new(continuation)),
                    )
                }
                Ok(Ok(continuation)) => RetainedNativeRecordingTurn::Continue(
                    RetainedNativeRecordingState::PreparedStage(Box::new(continuation)),
                ),
                Ok(Err(failure)) => retained_native_recording_state(failure)
                    .map(RetainedNativeRecordingTurn::Continue)
                    .unwrap_or(RetainedNativeRecordingTurn::RestartRequired),
            }
        }
        RetainedNativeRecordingState::PreparedStage(continuation) => {
            match process.continue_native_prepared_stage(
                selector,
                reborrow_deployment_operation_access(&mut access),
                *continuation,
                inputs.resolve_store,
                &mut *inputs.sign_checkpoint,
            ) {
                Err(failure) => {
                    let (continuation, _) = failure.into_parts();
                    RetainedNativeRecordingTurn::Continue(
                        RetainedNativeRecordingState::PreparedStage(Box::new(continuation)),
                    )
                }
                Ok(Ok(finalization)) => RetainedNativeRecordingTurn::Continue(
                    RetainedNativeRecordingState::StageFinalization(Box::new(finalization)),
                ),
                Ok(Err(failure)) => retained_native_recording_state(failure)
                    .map(RetainedNativeRecordingTurn::Continue)
                    .unwrap_or(RetainedNativeRecordingTurn::RestartRequired),
            }
        }
        RetainedNativeRecordingState::IssuedStage(continuation) => {
            match process.continue_native_issued_stage(
                selector,
                reborrow_deployment_operation_access(&mut access),
                *continuation,
                inputs.resolve_store,
            ) {
                Err(failure) => {
                    let (continuation, _) = failure.into_parts();
                    RetainedNativeRecordingTurn::Continue(
                        RetainedNativeRecordingState::IssuedStage(Box::new(continuation)),
                    )
                }
                Ok(Ok(finalization)) => RetainedNativeRecordingTurn::Continue(
                    RetainedNativeRecordingState::StageFinalization(Box::new(finalization)),
                ),
                Ok(Err(failure)) => retained_native_recording_state(failure)
                    .map(RetainedNativeRecordingTurn::Continue)
                    .unwrap_or(RetainedNativeRecordingTurn::RestartRequired),
            }
        }
        RetainedNativeRecordingState::StagingCommit(continuation) => {
            match process.reconcile_native_stage_commit(
                selector,
                reborrow_deployment_operation_access(&mut access),
                *continuation,
            ) {
                Err(failure) => {
                    let (continuation, _) = failure.into_parts();
                    RetainedNativeRecordingTurn::Continue(
                        RetainedNativeRecordingState::StagingCommit(Box::new(continuation)),
                    )
                }
                Ok(Ok(pending)) => RetainedNativeRecordingTurn::Continue(
                    RetainedNativeRecordingState::StagingAnchor(Box::new(pending)),
                ),
                Ok(Err(continuation)) => RetainedNativeRecordingTurn::Continue(
                    RetainedNativeRecordingState::StagingCommit(Box::new(continuation)),
                ),
            }
        }
        RetainedNativeRecordingState::StagingAnchor(continuation) => {
            match process.continue_native_stage_anchor(
                selector,
                reborrow_deployment_operation_access(&mut access),
                *continuation,
            ) {
                Err(failure) => {
                    let (continuation, _) = failure.into_parts();
                    RetainedNativeRecordingTurn::Continue(
                        RetainedNativeRecordingState::StagingAnchor(Box::new(continuation)),
                    )
                }
                Ok(Ok(finalization)) => RetainedNativeRecordingTurn::Continue(
                    RetainedNativeRecordingState::StageFinalization(Box::new(finalization)),
                ),
                Ok(Err(continuation)) => RetainedNativeRecordingTurn::Continue(
                    RetainedNativeRecordingState::StagingAnchor(Box::new(continuation)),
                ),
            }
        }
        RetainedNativeRecordingState::StageFinalization(finalization) => {
            match process.admit_native_stage(
                selector,
                reborrow_deployment_operation_access(&mut access),
                *finalization,
            ) {
                Err(failure) => {
                    let (continuation, _) = failure.into_parts();
                    RetainedNativeRecordingTurn::Continue(
                        RetainedNativeRecordingState::StageFinalization(Box::new(continuation)),
                    )
                }
                Ok(Ok(staged)) => RetainedNativeRecordingTurn::Continue(
                    RetainedNativeRecordingState::TerminalPreparation(Box::new(staged)),
                ),
                Ok(Err(_)) => RetainedNativeRecordingTurn::RestartRequired,
            }
        }
        RetainedNativeRecordingState::TerminalPreparation(staged) => staged
            .record_terminal(
                crate::resident::NativeOperationTerminalRecordingInput {
                    result_ref: inputs.result_ref,
                    diagnostic_ref: inputs.diagnostic_ref,
                    terminal_ref: inputs.terminal_ref,
                    terminal_checkpoint: inputs.terminal_checkpoint,
                    clock: inputs.clock,
                    resolve_store: inputs.resolve_store,
                    checkpoint_schema: inputs.checkpoint_schema,
                    checkpoint_authority: inputs.checkpoint_authority,
                    signing: inputs.signing,
                    maximum_diagnostic_bytes: inputs.maximum_diagnostic_bytes,
                    accumulator_limits: inputs.accumulator_limits,
                },
                access.custody,
                access.anchor,
                &mut *inputs.admit_rotation,
                &mut *inputs.sign_observation,
                &mut *inputs.sign_checkpoint,
                &mut *inputs.sign_result,
            )
            .map(|_| RetainedNativeRecordingTurn::Complete)
            .unwrap_or_else(|failure| {
                retained_native_recording_state(failure)
                    .map(RetainedNativeRecordingTurn::Continue)
                    .unwrap_or(RetainedNativeRecordingTurn::RestartRequired)
            }),
        RetainedNativeRecordingState::TerminalCustody(failure) => {
            match failure.continue_custody(access.custody, access.anchor) {
                Ok(_) => RetainedNativeRecordingTurn::Complete,
                Err(failure) => RetainedNativeRecordingTurn::Continue(
                    RetainedNativeRecordingState::TerminalCustody(Box::new(failure)),
                ),
            }
        }
    };
    next
}

struct RetainedProviderRecoveryPhase {
    key: RetainedInvokePhaseKey,
    selector: super::ResidentDeploymentAuthoritySelector,
    locator: ResidentOperationFrameLocator,
    state: RetainedProviderRecoveryState,
}

enum RetainedProviderRecoveryState {
    Prepared(Box<super::provider_recovery::ResidentProcessProviderRecoveryPrepared>),
    Issued(Box<super::provider_recovery::ResidentProcessProviderRecoveryIssued>),
    Custody(Box<super::provider_recovery::ResidentProcessProviderRecoveryCustody>),
    Finalization(Box<super::provider_recovery::ResidentProcessProviderRecoveryFinalization>),
}

impl RetainedProviderRecoveryState {
    const fn kind(&self) -> RetainedProviderRecoveryStateKind {
        match self {
            Self::Prepared(_) => RetainedProviderRecoveryStateKind::Prepared,
            Self::Issued(_) => RetainedProviderRecoveryStateKind::Issued,
            Self::Custody(_) => RetainedProviderRecoveryStateKind::Custody,
            Self::Finalization(_) => RetainedProviderRecoveryStateKind::Finalization,
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum RetainedProviderRecoveryStateKind {
    Prepared,
    Issued,
    Custody,
    Finalization,
}

impl RetainedInvokePhase {
    fn operation(&self) -> &crate::resident::IssuedLocalOperation {
        match self {
            Self::Deferred(disposition) => {
                super::operation_locator::restart_disposition_operation(disposition)
            }
            Self::LocalCancellation(recovered) => recovered.start.control.operation().operation(),
            Self::OperationResolution(recovered) => recovered.start.control.operation().operation(),
            Self::OperationResolutionContinuation(_) => {
                unreachable!("owned operation-resolution custody uses its retained locator")
            }
            Self::ProviderResolved(recovered) => recovered.start.control.operation().operation(),
            Self::ProviderResolvedContinuation(_)
            | Self::ProviderPreparedContinuation(_)
            | Self::NativePreparedContinuation(_) => {
                unreachable!("owned continuations retain their locator directly")
            }
            Self::ProviderPrepared(recovered) | Self::NativePrepared(recovered) => {
                recovered.start.control.operation().operation()
            }
            Self::NativeDispatched(recovered) | Self::ProviderDispatched(recovered) => {
                recovered.start.control.operation().operation()
            }
            Self::ProviderRecovery(_) => {
                unreachable!("provider recovery retains its locator directly")
            }
            Self::NativeRecording(_) => {
                unreachable!("native recording retains its locator directly")
            }
            Self::PreDispatchExpiry(_) => {
                unreachable!("expiry custody retains its locator directly")
            }
        }
    }

    fn locator(&self) -> ResidentOperationFrameLocator {
        match self {
            Self::ProviderRecovery(recovery) => recovery.locator.clone(),
            Self::NativeRecording(recovery) => recovery.locator.clone(),
            Self::ProviderResolvedContinuation(recovery) => recovery.locator.clone(),
            Self::OperationResolutionContinuation(recovery) => recovery.locator.clone(),
            Self::ProviderPreparedContinuation(recovery) => recovery.locator.clone(),
            Self::NativePreparedContinuation(recovery) => recovery.locator.clone(),
            Self::PreDispatchExpiry(recovery) => recovery.locator.clone(),
            _ => operation_locator_from_operation(self.operation()),
        }
    }

    fn key(&self) -> RetainedInvokePhaseKey {
        if let Self::ProviderRecovery(recovery) = self {
            return recovery.key.clone();
        }
        if let Self::NativeRecording(recovery) = self {
            return recovery.key.clone();
        }
        if let Self::ProviderResolvedContinuation(recovery) = self {
            return recovery.key.clone();
        }
        if let Self::OperationResolutionContinuation(recovery) = self {
            return recovery.key.clone();
        }
        if let Self::ProviderPreparedContinuation(recovery) = self {
            return recovery.key.clone();
        }
        if let Self::NativePreparedContinuation(recovery) = self {
            return recovery.key.clone();
        }
        if let Self::PreDispatchExpiry(recovery) = self {
            return recovery.key.clone();
        }
        let (observation_ref, observation) = self.observation();
        RetainedInvokePhaseKey {
            locator: self.locator(),
            observation_ref: observation_ref.as_str().to_owned(),
            observation_sha256: observation
                .canonical_operation_observation_sha256()
                .as_str()
                .to_owned(),
            phase: match self {
                Self::Deferred(_) => RetainedInvokePhaseKind::Deferred,
                Self::LocalCancellation(_) => RetainedInvokePhaseKind::LocalCancellation,
                Self::OperationResolution(_) => RetainedInvokePhaseKind::OperationResolution,
                Self::OperationResolutionContinuation(_) => {
                    RetainedInvokePhaseKind::OperationResolution
                }
                Self::ProviderResolved(_) => RetainedInvokePhaseKind::ProviderResolved,
                Self::ProviderResolvedContinuation(_)
                | Self::ProviderPreparedContinuation(_)
                | Self::NativePreparedContinuation(_) => {
                    unreachable!("handled above")
                }
                Self::ProviderPrepared(_) => RetainedInvokePhaseKind::ProviderPrepared,
                Self::ProviderDispatched(_) => RetainedInvokePhaseKind::ProviderDispatched,
                Self::NativePrepared(_) => RetainedInvokePhaseKind::NativePrepared,
                Self::NativeDispatched(_) => RetainedInvokePhaseKind::NativeDispatched,
                Self::ProviderRecovery(_) | Self::NativeRecording(_) => {
                    unreachable!("handled above")
                }
                Self::PreDispatchExpiry(_) => unreachable!("handled above"),
            },
        }
    }

    fn observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        self.start().control.observation().selected()
    }

    fn selector(&self) -> &super::ResidentDeploymentAuthoritySelector {
        match self {
            Self::ProviderRecovery(recovery) => &recovery.selector,
            Self::NativeRecording(recovery) => &recovery.selector,
            Self::ProviderResolvedContinuation(recovery) => &recovery.selector,
            Self::OperationResolutionContinuation(recovery) => &recovery.selector,
            Self::ProviderPreparedContinuation(recovery) => &recovery.selector,
            Self::NativePreparedContinuation(recovery) => &recovery.selector,
            Self::PreDispatchExpiry(recovery) => &recovery.selector,
            _ => &self.start().selector,
        }
    }

    fn is_runnable(&self) -> bool {
        self.native_hook_expiry_start().is_some()
            || matches!(
                self,
                Self::OperationResolution(_)
                    | Self::OperationResolutionContinuation(_)
                    | Self::ProviderResolved(_)
                    | Self::ProviderResolvedContinuation(_)
                    | Self::ProviderPrepared(_)
                    | Self::ProviderPreparedContinuation(_)
                    | Self::NativePrepared(_)
                    | Self::NativePreparedContinuation(_)
                    | Self::ProviderDispatched(_)
                    | Self::ProviderRecovery(_)
                    | Self::NativeDispatched(_)
                    | Self::NativeRecording(_)
                    | Self::PreDispatchExpiry(_)
            )
    }

    fn start(&self) -> &restart::ResidentProcessRecoveredOperationStart {
        match self {
            Self::Deferred(disposition) => restart_disposition_start(disposition),
            Self::LocalCancellation(recovered) => &recovered.start,
            Self::OperationResolution(recovered) => &recovered.start,
            Self::OperationResolutionContinuation(_) => {
                unreachable!("owned operation-resolution custody has no borrowed restart start")
            }
            Self::ProviderResolved(recovered) => &recovered.start,
            Self::ProviderResolvedContinuation(_)
            | Self::ProviderPreparedContinuation(_)
            | Self::NativePreparedContinuation(_) => {
                unreachable!("owned continuations retain selector and locator directly")
            }
            Self::ProviderPrepared(recovered) | Self::NativePrepared(recovered) => &recovered.start,
            Self::ProviderDispatched(recovered) | Self::NativeDispatched(recovered) => {
                &recovered.start
            }
            Self::ProviderRecovery(_) => {
                unreachable!("provider recovery retains its selector and locator directly")
            }
            Self::NativeRecording(_) => {
                unreachable!("native recording retains its selector and locator directly")
            }
            Self::PreDispatchExpiry(_) => {
                unreachable!("expiry custody retains its selector and locator directly")
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RetainedInvokePhaseKey {
    locator: ResidentOperationFrameLocator,
    observation_ref: String,
    observation_sha256: String,
    phase: RetainedInvokePhaseKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RetainedInvokePhaseKind {
    Deferred,
    LocalCancellation,
    OperationResolution,
    ProviderResolved,
    ProviderPrepared,
    ProviderDispatched,
    NativePrepared,
    NativeDispatched,
    NativeRecording,
    PreDispatchExpiry,
}

fn restart_disposition_start(
    disposition: &restart::ResidentProcessOperationRestartDisposition,
) -> &restart::ResidentProcessRecoveredOperationStart {
    match disposition {
        restart::ResidentProcessOperationRestartDisposition::NativeHookResponse(value) => {
            &value.start
        }
        restart::ResidentProcessOperationRestartDisposition::Terminal(value)
        | restart::ResidentProcessOperationRestartDisposition::RecoveryOnly(value)
        | restart::ResidentProcessOperationRestartDisposition::Consult(value) => value,
        restart::ResidentProcessOperationRestartDisposition::LocalCancellationRequested(value) => {
            &value.start
        }
        restart::ResidentProcessOperationRestartDisposition::OperationResolution(value) => {
            &value.start
        }
        restart::ResidentProcessOperationRestartDisposition::NativePrepared(value)
        | restart::ResidentProcessOperationRestartDisposition::ProviderPrepared(value) => {
            &value.start
        }
        restart::ResidentProcessOperationRestartDisposition::ProviderResolved(value) => {
            &value.start
        }
        restart::ResidentProcessOperationRestartDisposition::Native(value)
        | restart::ResidentProcessOperationRestartDisposition::Provider(value)
        | restart::ResidentProcessOperationRestartDisposition::A2aDelegateSendRecoveryOnly(value) => {
            &value.start
        }
        restart::ResidentProcessOperationRestartDisposition::A2aTask(value) => &value.attempt.start,
        restart::ResidentProcessOperationRestartDisposition::A2aCancelTaskRequestRecoveryOnly(
            value,
        ) => &value.attempt.start,
    }
}

fn operation_locator_from_operation(
    operation: &crate::resident::IssuedLocalOperation,
) -> ResidentOperationFrameLocator {
    let (handle_ref, handle) = operation.handle();
    ResidentOperationFrameLocator::new(
        handle_ref.as_str().to_owned(),
        handle
            .canonical_operation_handle_sha256()
            .as_str()
            .to_owned(),
    )
}

fn service_deployment_selector(
    selector: &super::ResidentDeploymentAuthoritySelector,
) -> ResidentServiceDeploymentSelector {
    ResidentServiceDeploymentSelector {
        principal_id: selector.principal_id.clone(),
        deployment_binding_ref: selector.deployment_binding_ref.clone(),
        deployment_binding_sha256: selector.deployment_binding_sha256.clone(),
        runtime_authority_ref: selector.runtime_authority_ref.clone(),
        runtime_authority_sha256: selector.runtime_authority_sha256.clone(),
    }
}

impl<'service, InvokeInputs, DelegateInputs, CancellationInputs, ObserveInputs>
    program::ResidentProgramRootRecoveryInputPort
    for InvokeRecoveryAdapter<
        'service,
        InvokeInputs,
        DelegateInputs,
        CancellationInputs,
        ObserveInputs,
    >
where
    InvokeInputs: ResidentServiceInvokeInputPort,
    DelegateInputs: ResidentServiceRetainedDelegatePhasePort,
{
    fn with_root_recovery<Output, UseInputs>(
        &mut self,
        selector: &super::ResidentDeploymentAuthoritySelector,
        request: &CommonRequestEnvelopeV1,
        locator: Option<&super::operation_locator::ResidentProgramOperationLocator>,
        use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'restart, 'admission, 'checkpoint> FnOnce(
            restart::ResidentProcessOperationRestartInput<'restart>,
            OperationControlRecoveryAdmission<'admission>,
            &'checkpoint ControlCheckpointSigningAuthorityAdmissionContextV1<'checkpoint>,
            &mut Self,
        ) -> Output,
    {
        if locator.is_some_and(|locator| {
            let coordinate = locator.coordinate();
            self.retained.iter().any(|phase| {
                phase.invoke().is_some_and(|phase| {
                    let retained = phase.locator();
                    retained.operation_handle_record_ref() == coordinate.record_ref().as_str()
                        && retained.operation_handle_sha256() == coordinate.record_sha256().as_str()
                })
            })
        }) {
            return Err(ResidentOperationFrameFailure::ResidentUnavailable);
        }
        let mut input_owner = self
            .invoke_inputs
            .take()
            .ok_or(ResidentOperationFrameFailure::ResidentUnavailable)?;
        let deployment = ResidentServiceDeploymentSelector {
            principal_id: selector.principal_id.clone(),
            deployment_binding_ref: selector.deployment_binding_ref.clone(),
            deployment_binding_sha256: selector.deployment_binding_sha256.clone(),
            runtime_authority_ref: selector.runtime_authority_ref.clone(),
            runtime_authority_sha256: selector.runtime_authority_sha256.clone(),
        };
        let result = input_owner.with_invoke_inputs(
            &deployment,
            request,
            locator.map(super::operation_locator::ResidentProgramOperationLocator::coordinate),
            |inputs| {
                let restart = restart::ResidentProcessOperationRestartInput {
                    committed_observation_limits: None,
                    operation: inputs.operation,
                    maximum_parent_depth: inputs.maximum_parent_depth,
                    current_head: restart::ResidentProcessRestartReadLimits {
                        observation: inputs.limits.observation,
                        checkpoints: inputs.limits.checkpoints,
                        membership: inputs.limits.membership,
                    },
                    maximum_selection_bytes: inputs.limits.maximum_selection_bytes,
                    a2a_task: None,
                    a2a_cancel_task: None,
                };
                self.recovery_mode = InvokeRecoveryMode::PointAddressed;
                let result = use_inputs(
                    restart,
                    inputs.recovery_admission,
                    inputs.checkpoint_authority,
                    self,
                );
                self.recovery_mode = InvokeRecoveryMode::Idle;
                result
            },
        );
        self.invoke_inputs = Some(input_owner);
        result
    }

    fn with_native_hook_response_inputs<Output>(
        &mut self,
        selector: &super::ResidentDeploymentAuthoritySelector,
        request: &CommonRequestEnvelopeV1,
        use_inputs: impl for<'a> FnOnce(
            crate::resident::operation_dispatch::NativeHookResponseInputs<'a>,
        ) -> Output,
    ) -> Result<Output, ResidentOperationFrameFailure> {
        self.invoke_inputs
            .as_mut()
            .ok_or(ResidentOperationFrameFailure::ResidentUnavailable)?
            .with_native_hook_response_inputs(
                &service_deployment_selector(selector),
                request,
                use_inputs,
            )
    }

    fn fresh_start_admission(&self) -> ResidentProcessFreshStartAdmission {
        if self.retained.is_full() {
            ResidentProcessFreshStartAdmission::CapacityExhausted
        } else {
            ResidentProcessFreshStartAdmission::Available
        }
    }

    fn with_background_recovery<UseInputs>(
        &mut self,
        selector: &super::ResidentDeploymentAuthoritySelector,
        use_inputs: UseInputs,
    ) -> Result<(), program::ResidentProgramTurnError>
    where
        UseInputs:
            for<'restart, 'admission> FnOnce(
                restart::ResidentProcessOperationRestartInput<'restart>,
                OperationControlRecoveryAdmission<'admission>,
                usize,
                UnixTimeV1,
                &mut Self,
            )
                -> Result<(), program::ResidentProgramTurnError>,
    {
        if self.retained.is_full()
            || self.retained.iter().any(|phase| {
                phase
                    .invoke()
                    .is_some_and(|phase| phase.selector() == selector)
            })
        {
            return Err(program::ResidentProgramTurnError::RecoveryUnavailable);
        }
        let mut input_owner = self
            .invoke_inputs
            .take()
            .ok_or(program::ResidentProgramTurnError::RecoveryUnavailable)?;
        let deployment = ResidentServiceDeploymentSelector {
            principal_id: selector.principal_id.clone(),
            deployment_binding_ref: selector.deployment_binding_ref.clone(),
            deployment_binding_sha256: selector.deployment_binding_sha256.clone(),
            runtime_authority_ref: selector.runtime_authority_ref.clone(),
            runtime_authority_sha256: selector.runtime_authority_sha256.clone(),
        };
        let result = input_owner.with_background_invoke_inputs(&deployment, |inputs| {
            let restart = restart::ResidentProcessOperationRestartInput {
                committed_observation_limits: Some(inputs.committed_observation_limits),
                operation: inputs.operation,
                maximum_parent_depth: inputs.maximum_parent_depth,
                current_head: restart::ResidentProcessRestartReadLimits {
                    observation: inputs.limits.observation,
                    checkpoints: inputs.limits.checkpoints,
                    membership: inputs.limits.membership,
                },
                maximum_selection_bytes: inputs.limits.maximum_selection_bytes,
                a2a_task: None,
                a2a_cancel_task: None,
            };
            self.recovery_mode = InvokeRecoveryMode::Background;
            let result = use_inputs(
                restart,
                inputs.recovery_admission,
                inputs.maximum_session_basis_bytes,
                inputs.observed_at,
                self,
            );
            self.recovery_mode = InvokeRecoveryMode::Idle;
            result
        });
        self.invoke_inputs = Some(input_owner);
        result.map_err(|_| program::ResidentProgramTurnError::RecoveryUnavailable)?
    }

    fn recover_publication<WriterGuard>(
        &mut self,
        process: &mut ResidentProcessHost<'_, WriterGuard>,
        selector: &super::ResidentDeploymentAuthoritySelector,
        access: super::ResidentDeploymentOperationAccess<'_>,
    ) -> Result<(), program::ResidentProgramTurnError>
    where
        WriterGuard: PrincipalWriterGuardPort,
    {
        let deployment = service_deployment_selector(selector);
        let inputs = self
            .publication_inputs
            .publication_inputs(&deployment)
            .map_err(|_| program::ResidentProgramTurnError::RecoveryUnavailable)?;
        let ResidentServicePublicationInputs {
            event_custody,
            sweep_limits,
            local_recovery_limits,
            a2a_recovery_limits,
            current,
            issuance,
            readmission,
            recover_local,
            recover_a2a,
        } = inputs;
        let mut event_custody = ResidentServicePublicationCustody(event_custody);
        let mut recover_local =
            |request: &LocalOperationPublicationRecoveryRequest<'_>| recover_local(request);
        let mut recover_a2a =
            |request: &A2aTaskPublicationRecoveryRequest<'_>| recover_a2a(request);
        process
            .run_event_publication_turn(
                selector,
                super::publication::ResidentDeploymentPublicationAccess {
                    source_store: access.store,
                    source_custody: access.custody,
                    event_custody: &mut event_custody,
                },
                super::publication::ResidentProcessPublicationTurn {
                    sweep_limits,
                    local_recovery_limits,
                    a2a_recovery_limits,
                    current,
                    issuance,
                    readmission,
                },
                &mut recover_local,
                &mut recover_a2a,
            )
            .map(|_| ())
            .map_err(|_| program::ResidentProgramTurnError::RecoveryUnavailable)
    }

    fn retain_deferred_recovery(
        &mut self,
        disposition: Box<restart::ResidentProcessOperationRestartDisposition>,
    ) {
        self.retain_without_response(RetainedInvokePhase::Deferred(disposition));
    }

    fn has_runnable_retained_invoke(&self) -> bool {
        self.retained
            .iter()
            .any(|phase| phase.invoke().is_some_and(RetainedInvokePhase::is_runnable))
    }

    fn retained_invoke_selector(&self) -> Option<&super::ResidentDeploymentAuthoritySelector> {
        self.retained
            .iter()
            .filter_map(RetainedServicePhase::invoke)
            .find(|phase| phase.is_runnable())
            .map(RetainedInvokePhase::selector)
    }

    fn continue_one_retained_invoke<WriterGuard, Authority>(
        &mut self,
        process: &mut ResidentProcessHost<'_, WriterGuard>,
        access: super::ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
    ) -> program::ResidentProgramRetainedPhaseTurn
    where
        WriterGuard: PrincipalWriterGuardPort,
        Authority: AmbientSessionAuthorityPort,
    {
        self.continue_retained_provider_invoke(process, access, authority)
    }

    fn has_runnable_retained_delegate(&self) -> bool {
        self.delegate_inputs.has_runnable_retained_delegate()
    }

    fn retained_delegate_selector(&self) -> Option<&super::ResidentDeploymentAuthoritySelector> {
        self.delegate_inputs.retained_delegate_selector()
    }

    fn continue_one_retained_delegate<WriterGuard, Authority>(
        &mut self,
        process: &mut ResidentProcessHost<'_, WriterGuard>,
        access: super::ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
    ) -> program::ResidentProgramRetainedPhaseTurn
    where
        WriterGuard: PrincipalWriterGuardPort,
        Authority: AmbientSessionAuthorityPort,
    {
        self.delegate_inputs
            .continue_one_retained_delegate(process, access, authority)
    }
}

impl<'service, InvokeInputs, DelegateInputs, CancellationInputs, ObserveInputs>
    InvokeRecoveryAdapter<'service, InvokeInputs, DelegateInputs, CancellationInputs, ObserveInputs>
where
    InvokeInputs: ResidentServiceInvokeInputPort,
{
    fn lend_pre_dispatch_expiry_preparation_inputs<Output, UseInputs>(
        &mut self,
        selector: &super::ResidentDeploymentAuthoritySelector,
        locator: &ResidentOperationFrameLocator,
        use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs:
            for<'input> FnOnce(ResidentServicePreDispatchExpiryPreparationInputs<'input>) -> Output,
    {
        let mut input_owner = self
            .invoke_inputs
            .take()
            .ok_or(ResidentOperationFrameFailure::ResidentUnavailable)?;
        let deployment = service_deployment_selector(selector);
        let mut output = None;
        let lender = input_owner.with_pre_dispatch_expiry_preparation_inputs(
            &deployment,
            locator,
            |inputs| output = Some(use_inputs(inputs)),
        );
        self.invoke_inputs = Some(input_owner);
        output.ok_or_else(|| {
            lender
                .err()
                .unwrap_or(ResidentOperationFrameFailure::ConfigurationUnavailable)
        })
    }

    fn lend_pre_dispatch_expiry_continuation_inputs<Output, UseInputs>(
        &mut self,
        selector: &super::ResidentDeploymentAuthoritySelector,
        locator: &ResidentOperationFrameLocator,
        use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(
            ResidentServicePreDispatchExpiryContinuationInputs<'input>,
        ) -> Output,
    {
        let mut input_owner = self
            .invoke_inputs
            .take()
            .ok_or(ResidentOperationFrameFailure::ResidentUnavailable)?;
        let deployment = service_deployment_selector(selector);
        let mut output = None;
        let lender = input_owner.with_pre_dispatch_expiry_continuation_inputs(
            &deployment,
            locator,
            |inputs| output = Some(use_inputs(inputs)),
        );
        self.invoke_inputs = Some(input_owner);
        output.ok_or_else(|| {
            lender
                .err()
                .unwrap_or(ResidentOperationFrameFailure::ConfigurationUnavailable)
        })
    }

    fn lend_operation_resolution_inputs<Output, UseInputs>(
        &mut self,
        selector: &super::ResidentDeploymentAuthoritySelector,
        locator: &ResidentOperationFrameLocator,
        use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(ResidentServiceOperationResolutionInputs<'input>) -> Output,
    {
        let mut input_owner = self
            .invoke_inputs
            .take()
            .ok_or(ResidentOperationFrameFailure::ResidentUnavailable)?;
        let deployment = service_deployment_selector(selector);
        let mut output = None;
        let lender = input_owner.with_operation_resolution_inputs(&deployment, locator, |inputs| {
            output = Some(use_inputs(inputs));
        });
        self.invoke_inputs = Some(input_owner);
        output.ok_or_else(|| {
            lender
                .err()
                .unwrap_or(ResidentOperationFrameFailure::ConfigurationUnavailable)
        })
    }

    fn lend_provider_resolved_inputs<Output, UseInputs>(
        &mut self,
        selector: &super::ResidentDeploymentAuthoritySelector,
        locator: &ResidentOperationFrameLocator,
        use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(ResidentServiceProviderResolvedInputs<'input>) -> Output,
    {
        let mut input_owner = self
            .invoke_inputs
            .take()
            .ok_or(ResidentOperationFrameFailure::ResidentUnavailable)?;
        let deployment = service_deployment_selector(selector);
        let mut output = None;
        let lender = input_owner.with_provider_resolved_inputs(&deployment, locator, |inputs| {
            output = Some(use_inputs(inputs));
        });
        self.invoke_inputs = Some(input_owner);
        output.ok_or_else(|| {
            lender
                .err()
                .unwrap_or(ResidentOperationFrameFailure::ConfigurationUnavailable)
        })
    }

    fn lend_native_prepared_inputs<Output, UseInputs>(
        &mut self,
        selector: &super::ResidentDeploymentAuthoritySelector,
        locator: &ResidentOperationFrameLocator,
        use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(ResidentServiceNativePreparedInputs<'input>) -> Output,
    {
        let mut input_owner = self
            .invoke_inputs
            .take()
            .ok_or(ResidentOperationFrameFailure::ResidentUnavailable)?;
        let deployment = service_deployment_selector(selector);
        let mut output = None;
        let lender = input_owner.with_native_prepared_inputs(&deployment, locator, |inputs| {
            output = Some(use_inputs(inputs));
        });
        self.invoke_inputs = Some(input_owner);
        output.ok_or_else(|| {
            lender
                .err()
                .unwrap_or(ResidentOperationFrameFailure::ConfigurationUnavailable)
        })
    }

    fn lend_provider_prepared_inputs<Output, UseInputs>(
        &mut self,
        selector: &super::ResidentDeploymentAuthoritySelector,
        locator: &ResidentOperationFrameLocator,
        use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(ResidentServiceProviderPreparedInputs<'input>) -> Output,
    {
        let mut input_owner = self
            .invoke_inputs
            .take()
            .ok_or(ResidentOperationFrameFailure::ResidentUnavailable)?;
        let deployment = service_deployment_selector(selector);
        let mut output = None;
        let lender = input_owner.with_provider_prepared_inputs(&deployment, locator, |inputs| {
            output = Some(use_inputs(inputs));
        });
        self.invoke_inputs = Some(input_owner);
        output.ok_or_else(|| {
            lender
                .err()
                .unwrap_or(ResidentOperationFrameFailure::ConfigurationUnavailable)
        })
    }

    fn lend_provider_recovery_inputs<Output, UseInputs>(
        &mut self,
        selector: &super::ResidentDeploymentAuthoritySelector,
        locator: &ResidentOperationFrameLocator,
        use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(ResidentServiceProviderRecoveryInputs<'input>) -> Output,
    {
        let mut input_owner = self
            .invoke_inputs
            .take()
            .ok_or(ResidentOperationFrameFailure::ResidentUnavailable)?;
        let deployment = service_deployment_selector(selector);
        let mut output = None;
        let lender = input_owner.with_provider_recovery_inputs(&deployment, locator, |inputs| {
            output = Some(use_inputs(inputs))
        });
        self.invoke_inputs = Some(input_owner);
        match output {
            // Once the callback ran, its linear result takes precedence over a
            // post-callback lender error.
            Some(output) => Ok(output),
            None => Err(lender
                .err()
                .unwrap_or(ResidentOperationFrameFailure::ConfigurationUnavailable)),
        }
    }

    fn lend_native_recovery_inputs<Output, UseInputs>(
        &mut self,
        selector: &super::ResidentDeploymentAuthoritySelector,
        locator: &ResidentOperationFrameLocator,
        use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(ResidentServiceNativeRecoveryInputs<'input>) -> Output,
    {
        let mut input_owner = self
            .invoke_inputs
            .take()
            .ok_or(ResidentOperationFrameFailure::ResidentUnavailable)?;
        let deployment = service_deployment_selector(selector);
        let mut output = None;
        let lender = input_owner.with_native_recovery_inputs(&deployment, locator, |inputs| {
            output = Some(use_inputs(inputs));
        });
        self.invoke_inputs = Some(input_owner);
        match output {
            Some(output) => Ok(output),
            None => Err(lender
                .err()
                .unwrap_or(ResidentOperationFrameFailure::ConfigurationUnavailable)),
        }
    }

    fn retain_reserved_invoke(
        &mut self,
        reservation: RetainedPhaseReservation<RetainedServicePhaseKey>,
        phase: RetainedInvokePhase,
        turn: program::ResidentProgramRetainedPhaseTurn,
    ) -> program::ResidentProgramRetainedPhaseTurn {
        let deferred_hook = phase
            .native_hook_expiry_start()
            .is_some()
            .then(|| reservation.key.clone());
        self.retained
            .retain_reserved(reservation, RetainedServicePhase::Invoke(phase))
            .unwrap_or_else(|_| unreachable!("the retained Invoke reservation remains linear"));
        if let Some(key) = deferred_hook {
            self.retained.move_to_back(&key);
        }
        turn
    }

    fn retain_provider_recovery(
        &mut self,
        reservation: RetainedPhaseReservation<RetainedServicePhaseKey>,
        key: RetainedInvokePhaseKey,
        selector: super::ResidentDeploymentAuthoritySelector,
        locator: ResidentOperationFrameLocator,
        state: RetainedProviderRecoveryState,
        turn: program::ResidentProgramRetainedPhaseTurn,
    ) -> program::ResidentProgramRetainedPhaseTurn {
        self.retain_reserved_invoke(
            reservation,
            RetainedInvokePhase::ProviderRecovery(Box::new(RetainedProviderRecoveryPhase {
                key,
                selector,
                locator,
                state,
            })),
            turn,
        )
    }

    fn retain_provider_resolved_continuation(
        &mut self,
        reservation: RetainedPhaseReservation<RetainedServicePhaseKey>,
        key: RetainedInvokePhaseKey,
        selector: super::ResidentDeploymentAuthoritySelector,
        locator: ResidentOperationFrameLocator,
        state: RetainedProviderResolvedState,
        turn: program::ResidentProgramRetainedPhaseTurn,
    ) -> program::ResidentProgramRetainedPhaseTurn {
        self.retain_reserved_invoke(
            reservation,
            RetainedInvokePhase::ProviderResolvedContinuation(Box::new(
                RetainedProviderResolvedPhase {
                    key,
                    selector,
                    locator,
                    state,
                },
            )),
            turn,
        )
    }

    fn retain_operation_resolution_continuation(
        &mut self,
        reservation: RetainedPhaseReservation<RetainedServicePhaseKey>,
        key: RetainedInvokePhaseKey,
        selector: super::ResidentDeploymentAuthoritySelector,
        locator: ResidentOperationFrameLocator,
        state: crate::resident::operation_dispatch::OwnedOperationObservationCustodyFailure,
        turn: program::ResidentProgramRetainedPhaseTurn,
    ) -> program::ResidentProgramRetainedPhaseTurn {
        self.retain_reserved_invoke(
            reservation,
            RetainedInvokePhase::OperationResolutionContinuation(Box::new(
                RetainedOperationResolutionPhase {
                    key,
                    selector,
                    locator,
                    state,
                },
            )),
            turn,
        )
    }

    fn retain_pre_dispatch_expiry(
        &mut self,
        reservation: RetainedPhaseReservation<RetainedServicePhaseKey>,
        phase: RetainedPreDispatchExpiryPhase,
        turn: program::ResidentProgramRetainedPhaseTurn,
    ) -> program::ResidentProgramRetainedPhaseTurn {
        self.retain_reserved_invoke(
            reservation,
            RetainedInvokePhase::PreDispatchExpiry(Box::new(phase)),
            turn,
        )
    }

    fn retain_native_prepared_continuation(
        &mut self,
        reservation: RetainedPhaseReservation<RetainedServicePhaseKey>,
        key: RetainedInvokePhaseKey,
        selector: super::ResidentDeploymentAuthoritySelector,
        locator: ResidentOperationFrameLocator,
        state: RetainedNativePreparedState,
        turn: program::ResidentProgramRetainedPhaseTurn,
    ) -> program::ResidentProgramRetainedPhaseTurn {
        self.retain_reserved_invoke(
            reservation,
            RetainedInvokePhase::NativePreparedContinuation(Box::new(
                RetainedNativePreparedPhase {
                    key,
                    selector,
                    locator,
                    state,
                },
            )),
            turn,
        )
    }

    fn retain_provider_prepared_continuation(
        &mut self,
        reservation: RetainedPhaseReservation<RetainedServicePhaseKey>,
        key: RetainedInvokePhaseKey,
        selector: super::ResidentDeploymentAuthoritySelector,
        locator: ResidentOperationFrameLocator,
        state: RetainedProviderPreparedState,
        turn: program::ResidentProgramRetainedPhaseTurn,
    ) -> program::ResidentProgramRetainedPhaseTurn {
        self.retain_reserved_invoke(
            reservation,
            RetainedInvokePhase::ProviderPreparedContinuation(Box::new(
                RetainedProviderPreparedPhase {
                    key,
                    selector,
                    locator,
                    state,
                },
            )),
            turn,
        )
    }

    fn retain_native_recording(
        &mut self,
        reservation: RetainedPhaseReservation<RetainedServicePhaseKey>,
        phase: RetainedNativeRecordingPhase,
        turn: program::ResidentProgramRetainedPhaseTurn,
    ) -> program::ResidentProgramRetainedPhaseTurn {
        self.retain_reserved_invoke(
            reservation,
            RetainedInvokePhase::NativeRecording(Box::new(phase)),
            turn,
        )
    }

    fn release_completed_invoke_reservation(
        &mut self,
        reservation: RetainedPhaseReservation<RetainedServicePhaseKey>,
    ) -> program::ResidentProgramRetainedPhaseTurn {
        assert!(
            self.retained.release(reservation),
            "the completed Invoke reservation remains linear"
        );
        program::ResidentProgramRetainedPhaseTurn::Advanced
    }

    fn gate_pre_dispatch_expiry<WriterGuard>(
        &mut self,
        process: &ResidentProcessHost<'_, WriterGuard>,
        access: &mut super::ResidentDeploymentOperationAccess<'_>,
        reservation: RetainedPhaseReservation<RetainedServicePhaseKey>,
        phase: RetainedInvokePhase,
    ) -> Result<
        (
            RetainedPhaseReservation<RetainedServicePhaseKey>,
            RetainedInvokePhase,
        ),
        program::ResidentProgramRetainedPhaseTurn,
    > {
        let Some(start) = phase.pre_dispatch_expiry_start() else {
            return Ok((reservation, phase));
        };
        let expiry_only = matches!(&phase, RetainedInvokePhase::Deferred(_));
        let selector = start.selector.clone();
        let locator = operation_locator_from_operation(start.control.operation().operation());
        let key = retained_source_key(
            start,
            locator.clone(),
            RetainedInvokePhaseKind::PreDispatchExpiry,
        );
        let attempted =
            self.lend_pre_dispatch_expiry_preparation_inputs(&selector, &locator, |inputs| {
                run_pre_dispatch_expiry_turn(process, start, access, inputs)
            });
        match attempted {
            Ok(Ok(PreDispatchExpiryTurn::NotExpired)) if !expiry_only => Ok((reservation, phase)),
            Ok(Ok(PreDispatchExpiryTurn::Complete)) => {
                Err(self.release_completed_invoke_reservation(reservation))
            }
            Ok(Ok(PreDispatchExpiryTurn::Pending { observed_at, state })) => Err(self
                .retain_pre_dispatch_expiry(
                    reservation,
                    RetainedPreDispatchExpiryPhase {
                        key,
                        selector,
                        locator,
                        observed_at,
                        state,
                    },
                    program::ResidentProgramRetainedPhaseTurn::Advanced,
                )),
            Ok(Ok(PreDispatchExpiryTurn::NotExpired)) | Ok(Err(_)) | Err(_) => Err(self
                .retain_reserved_invoke(
                    reservation,
                    phase,
                    program::ResidentProgramRetainedPhaseTurn::Deferred,
                )),
        }
    }

    fn continue_retained_provider_invoke<WriterGuard, Authority>(
        &mut self,
        process: &mut ResidentProcessHost<'_, WriterGuard>,
        mut access: super::ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
    ) -> program::ResidentProgramRetainedPhaseTurn
    where
        Authority: AmbientSessionAuthorityPort,
    {
        let Some((reservation, retained)) = self
            .retained
            .take_first_where(|phase| phase.invoke().is_some_and(RetainedInvokePhase::is_runnable))
        else {
            return program::ResidentProgramRetainedPhaseTurn::Idle;
        };
        let RetainedServicePhase::Invoke(phase) = retained else {
            unreachable!("only a runnable Invoke phase was selected")
        };
        let (reservation, phase) =
            match self.gate_pre_dispatch_expiry(process, &mut access, reservation, phase) {
                Ok(retained) => retained,
                Err(turn) => return turn,
            };
        match phase {
            RetainedInvokePhase::OperationResolution(recovered) => {
                let selector = recovered.start.selector.clone();
                let locator = operation_locator_from_operation(
                    recovered.start.control.operation().operation(),
                );
                let key = retained_source_key(
                    &recovered.start,
                    locator.clone(),
                    RetainedInvokePhaseKind::OperationResolution,
                );
                let result = self.lend_operation_resolution_inputs(&selector, &locator, |inputs| {
                    let ResidentServiceOperationResolutionInputs {
                        session,
                        current,
                        route,
                        signing,
                        selection_ref,
                        observation_ref,
                        checkpoint,
                        checkpoint_schema,
                        checkpoint_authority,
                        resolve_store,
                        accumulator_limits,
                        maximum_selection_bytes,
                        maximum_record_bytes,
                        admit_rotation,
                        sign_observation,
                        sign_checkpoint,
                    } = inputs;
                    process.with_recovered_operation_resolution_access(
                        &recovered,
                        reborrow_deployment_operation_access(&mut access),
                        super::operation_resolution::ResidentProcessOperationResolutionInput {
                            session,
                            current,
                            signing,
                            selection_ref,
                            observation_ref,
                            maximum_selection_bytes,
                            maximum_record_bytes,
                        },
                        authority,
                        |context, prior_checkpoint, access, authority| {
                            let transition = match route {
                                ResidentServiceOperationResolutionRoute::Native {
                                    registry,
                                    attempt_id,
                                    result_admission,
                                } => crate::resident::prepare_native_operation_attempt(
                                    crate::resident::NativeOperationAttemptInput {
                                        context,
                                        registry,
                                        attempt_id,
                                        result_admission,
                                    },
                                    authority,
                                    &mut *admit_rotation,
                                    &mut *sign_observation,
                                )
                                .map(|prepared| prepared.into_owned_observation()),
                                ResidentServiceOperationResolutionRoute::Provider {
                                    invocation,
                                    result_admission,
                                    resolution_ref,
                                } => crate::resident::prepare_provider_resolution(
                                    crate::resident::ProviderOperationInput {
                                        context,
                                        invocation,
                                        result_admission,
                                        resolution_ref,
                                    },
                                    authority,
                                    &mut *admit_rotation,
                                    &mut *sign_observation,
                                )
                                .map(|prepared| prepared.into_owned_observation()),
                            }
                            .map_err(OperationResolutionTurnFailure::Preparation)?;
                            crate::resident::operation_dispatch::retain_owned_operation_observation(
                                transition,
                                (prior_checkpoint.0.clone(), prior_checkpoint.1.clone()),
                                checkpoint,
                                current.now,
                                checkpoint_schema,
                                checkpoint_authority,
                                resolve_store,
                                accumulator_limits,
                                access.custody,
                                access.anchor,
                                &mut *sign_checkpoint,
                            )
                            .map_err(OperationResolutionTurnFailure::Custody)
                        },
                    )
                });
                match result {
                    Ok(Ok(Ok(_))) => self.release_completed_invoke_reservation(reservation),
                    Ok(Ok(Err(OperationResolutionTurnFailure::Custody(state)))) => self
                        .retain_operation_resolution_continuation(
                            reservation,
                            key,
                            selector,
                            locator,
                            state,
                            program::ResidentProgramRetainedPhaseTurn::Advanced,
                        ),
                    Ok(Ok(Err(OperationResolutionTurnFailure::Preparation(_))))
                    | Ok(Err(_))
                    | Err(_) => self.retain_reserved_invoke(
                        reservation,
                        RetainedInvokePhase::OperationResolution(recovered),
                        program::ResidentProgramRetainedPhaseTurn::Deferred,
                    ),
                }
            }
            RetainedInvokePhase::OperationResolutionContinuation(recovery) => {
                let RetainedOperationResolutionPhase {
                    key,
                    selector,
                    locator,
                    state,
                } = *recovery;
                let mut state = Some(state);
                let result = self.lend_operation_resolution_inputs(
                    &selector,
                    &locator,
                    |inputs| {
                        let index = match process.deployment_authority_index(&selector) {
                            Ok(index) => index,
                            Err(_) => return Err(()),
                        };
                        if process.deployments[index].store() != access.store {
                            return Err(());
                        }
                        crate::resident::operation_dispatch::continue_owned_operation_observation_custody(
                            state
                                .take()
                                .expect("the operation-resolution lender runs once"),
                            inputs.current.now,
                            inputs.checkpoint_schema,
                            inputs.checkpoint_authority,
                            inputs.resolve_store,
                            inputs.accumulator_limits,
                            access.custody,
                            access.anchor,
                            &mut *inputs.sign_checkpoint,
                        )
                        .map_err(|pending| {
                            state = Some(pending);
                        })
                    },
                );
                match result {
                    Ok(Ok(_)) => self.release_completed_invoke_reservation(reservation),
                    Err(_) | Ok(Err(())) => self.retain_operation_resolution_continuation(
                        reservation,
                        key,
                        selector,
                        locator,
                        state.expect("an incomplete custody turn retains its continuation"),
                        program::ResidentProgramRetainedPhaseTurn::Deferred,
                    ),
                }
            }
            RetainedInvokePhase::ProviderResolved(recovered) => {
                let selector = recovered.start.selector.clone();
                let locator = operation_locator_from_operation(
                    recovered.start.control.operation().operation(),
                );
                let key = retained_source_key(
                    &recovered.start,
                    locator.clone(),
                    RetainedInvokePhaseKind::ProviderResolved,
                );
                let result = self.lend_provider_resolved_inputs(&selector, &locator, |inputs| {
                    let meaning =
                        crate::resident::operation_dispatch::RecoveredProviderAttemptMeaning {
                            implementation_kind: inputs.implementation_kind,
                            implementation_version: inputs.implementation_version,
                            admission:
                                crate::provider_invocation::ProviderInvocationAttemptAdmission {
                                    result_schema_sha256: inputs.result_schema,
                                    result_signing_authority: inputs.result_authority,
                                    contract_set: inputs.contract_set,
                                    failure_schema_sha256: inputs.failure_schema,
                                    failure_signing_authority: inputs.failure_authority,
                                    safe_failure_proof: inputs.safe_failure_proof,
                                },
                        };
                    let owner_input =
                        super::provider_resolution_recovery::ResidentProcessProviderResolvedInput {
                            session: inputs.session,
                            current: inputs.current,
                            meaning,
                            signing: inputs.signing,
                            checkpoint_schema: inputs.checkpoint_schema,
                            checkpoint_authority: inputs.checkpoint_authority,
                            resolve_store: inputs.resolve_store,
                            accumulator_limits: inputs.accumulator_limits,
                            maximum_selection_bytes: inputs.maximum_selection_bytes,
                            maximum_record_bytes: inputs.maximum_record_bytes,
                        };
                    let allocation = process.allocate_provider_resolved_attempt(
                        &recovered,
                        reborrow_deployment_operation_access(&mut access),
                        &owner_input,
                        authority,
                        inputs.random,
                    )?;
                    let preparation = process.prepare_provider_resolved_attempt(
                        &recovered,
                        &owner_input,
                        allocation,
                        authority,
                        &mut *inputs.admit_rotation,
                        &mut *inputs.sign_observation,
                    )?;
                    process
                        .retain_provider_resolved_attempt(
                            &recovered,
                            reborrow_deployment_operation_access(&mut access),
                            &owner_input,
                            preparation,
                            &mut *inputs.sign_checkpoint,
                        )
                        .map_err(ProviderResolvedTurnFailure::Retention)
                });
                match result {
                    Ok(Ok(_)) => self.release_completed_invoke_reservation(reservation),
                    Ok(Err(ProviderResolvedTurnFailure::Preparation(_))) | Err(_) => self
                        .retain_reserved_invoke(
                            reservation,
                            RetainedInvokePhase::ProviderResolved(recovered),
                            program::ResidentProgramRetainedPhaseTurn::Deferred,
                        ),
                    Ok(Err(ProviderResolvedTurnFailure::Retention(failure))) => {
                        let state = retained_provider_resolved_state(failure);
                        self.retain_provider_resolved_continuation(
                            reservation,
                            key,
                            selector,
                            locator,
                            state,
                            program::ResidentProgramRetainedPhaseTurn::Advanced,
                        )
                    }
                }
            }
            RetainedInvokePhase::ProviderResolvedContinuation(recovery) => self
                .continue_provider_resolved_continuation(process, reservation, *recovery, access),
            RetainedInvokePhase::NativePrepared(recovered) => {
                let selector = recovered.start.selector.clone();
                let locator = operation_locator_from_operation(
                    recovered.start.control.operation().operation(),
                );
                let key = retained_source_key(
                    &recovered.start,
                    locator.clone(),
                    RetainedInvokePhaseKind::NativePrepared,
                );
                let result = self.lend_native_prepared_inputs(&selector, &locator, |mut inputs| {
                    let dispatch = process.recover_native_prepared_dispatch(
                        &recovered,
                        reborrow_deployment_operation_access(&mut access),
                        super::dispatch_prepared_recovery::ResidentProcessPreparedDispatchInput {
                            current: inputs.current,
                            signing: inputs.signing,
                            observation_ref: inputs.observation_ref.clone(),
                            maximum_selection_bytes: inputs.maximum_selection_bytes,
                            maximum_record_bytes: inputs.maximum_record_bytes,
                        },
                        inputs.registry,
                        inputs.contract_set,
                        authority,
                        &mut *inputs.admit_rotation,
                        &mut *inputs.sign_observation,
                    )?;
                    Ok::<_, super::dispatch_prepared_recovery::ResidentProcessPreparedDispatchFailure>(
                        continue_native_prepared_custody(
                            process,
                            &selector,
                            reborrow_deployment_operation_access(&mut access),
                            RetainedNativePreparedState::Dispatch(dispatch),
                            &mut inputs,
                        ),
                    )
                });
                match result {
                    Ok(Ok(Ok(()))) => self.release_completed_invoke_reservation(reservation),
                    Ok(Ok(Err(state))) => self.retain_native_prepared_continuation(
                        reservation,
                        key,
                        selector,
                        locator,
                        state,
                        program::ResidentProgramRetainedPhaseTurn::Advanced,
                    ),
                    Ok(Err(_)) | Err(_) => self.retain_reserved_invoke(
                        reservation,
                        RetainedInvokePhase::NativePrepared(recovered),
                        program::ResidentProgramRetainedPhaseTurn::Deferred,
                    ),
                }
            }
            RetainedInvokePhase::NativePreparedContinuation(recovery) => {
                let RetainedNativePreparedPhase {
                    key,
                    selector,
                    locator,
                    state,
                } = *recovery;
                let mut state = Some(state);
                let result = self.lend_native_prepared_inputs(&selector, &locator, |mut inputs| {
                    continue_native_prepared_custody(
                        process,
                        &selector,
                        access,
                        state.take().expect("the native prepared lender runs once"),
                        &mut inputs,
                    )
                });
                match result {
                    Err(_) => self.retain_native_prepared_continuation(
                        reservation,
                        key,
                        selector,
                        locator,
                        state.take().expect("an uncalled lender preserves custody"),
                        program::ResidentProgramRetainedPhaseTurn::Deferred,
                    ),
                    Ok(Ok(())) => self.release_completed_invoke_reservation(reservation),
                    Ok(Err(state)) => self.retain_native_prepared_continuation(
                        reservation,
                        key,
                        selector,
                        locator,
                        state,
                        program::ResidentProgramRetainedPhaseTurn::Advanced,
                    ),
                }
            }
            RetainedInvokePhase::ProviderPrepared(recovered) => {
                let selector = recovered.start.selector.clone();
                let locator = operation_locator_from_operation(
                    recovered.start.control.operation().operation(),
                );
                let key = retained_source_key(
                    &recovered.start,
                    locator.clone(),
                    RetainedInvokePhaseKind::ProviderPrepared,
                );
                let result = self.lend_provider_prepared_inputs(
                    &selector,
                    &locator,
                    |mut inputs| {
                        let dispatch = process
                            .provider_recovery_driver(&selector)?
                            .recover_prepared_dispatch(
                                &recovered,
                                reborrow_deployment_operation_access(&mut access),
                                super::dispatch_prepared_recovery::ResidentProcessPreparedDispatchInput {
                                    current: inputs.current,
                                    signing: inputs.signing,
                                    observation_ref: inputs.observation_ref.clone(),
                                    maximum_selection_bytes: inputs.maximum_selection_bytes,
                                    maximum_record_bytes: inputs.maximum_record_bytes,
                                },
                                inputs.admission,
                                authority,
                                &mut *inputs.admit_rotation,
                                &mut *inputs.sign_observation,
                            )
                            .map_err(|_| ResidentProcessHostSelectionError::ProviderOperationAuthority)?;
                        Ok::<_, ResidentProcessHostSelectionError>(
                            continue_provider_prepared_custody(
                                process,
                                &selector,
                                reborrow_deployment_operation_access(&mut access),
                                RetainedProviderPreparedState::Dispatch(Box::new(dispatch)),
                                &mut inputs,
                                authority,
                            ),
                        )
                    },
                );
                match result {
                    Ok(Ok(Ok(ProviderPreparedTurn::Accepted))) => {
                        self.release_completed_invoke_reservation(reservation)
                    }
                    Ok(Ok(Ok(ProviderPreparedTurn::Complete))) => {
                        self.release_completed_invoke_reservation(reservation)
                    }
                    Ok(Ok(Err(state))) => self.retain_provider_prepared_continuation(
                        reservation,
                        key,
                        selector,
                        locator,
                        state,
                        program::ResidentProgramRetainedPhaseTurn::Advanced,
                    ),
                    Ok(Err(_)) | Err(_) => self.retain_reserved_invoke(
                        reservation,
                        RetainedInvokePhase::ProviderPrepared(recovered),
                        program::ResidentProgramRetainedPhaseTurn::Deferred,
                    ),
                }
            }
            RetainedInvokePhase::ProviderPreparedContinuation(recovery) => {
                let RetainedProviderPreparedPhase {
                    key,
                    selector,
                    locator,
                    state,
                } = *recovery;
                let mut state = Some(state);
                let result =
                    self.lend_provider_prepared_inputs(&selector, &locator, |mut inputs| {
                        Ok::<_, ResidentProcessHostSelectionError>(
                            continue_provider_prepared_custody(
                                process,
                                &selector,
                                access,
                                state
                                    .take()
                                    .expect("the provider prepared lender runs once"),
                                &mut inputs,
                                authority,
                            ),
                        )
                    });
                match result {
                    Err(_) | Ok(Err(_)) => self.retain_provider_prepared_continuation(
                        reservation,
                        key,
                        selector,
                        locator,
                        state.take().expect("an uncalled lender preserves custody"),
                        program::ResidentProgramRetainedPhaseTurn::Deferred,
                    ),
                    Ok(Ok(Ok(_))) => self.release_completed_invoke_reservation(reservation),
                    Ok(Ok(Err(state))) => self.retain_provider_prepared_continuation(
                        reservation,
                        key,
                        selector,
                        locator,
                        state,
                        program::ResidentProgramRetainedPhaseTurn::Advanced,
                    ),
                }
            }
            RetainedInvokePhase::ProviderDispatched(recovered) => {
                let selector = recovered.start.selector.clone();
                let locator = operation_locator_from_operation(
                    recovered.start.control.operation().operation(),
                );
                let (observation_ref, observation) =
                    recovered.start.control.observation().selected();
                let key = RetainedInvokePhaseKey {
                    locator: locator.clone(),
                    observation_ref: observation_ref.as_str().to_owned(),
                    observation_sha256: observation
                        .canonical_operation_observation_sha256()
                        .as_str()
                        .to_owned(),
                    phase: RetainedInvokePhaseKind::ProviderDispatched,
                };
                let result = self.lend_provider_recovery_inputs(&selector, &locator, |inputs| {
                    process.recover_provider(
                        access,
                        authority,
                        super::provider_recovery::ResidentProcessProviderRecoveryInput {
                            attempt: &recovered,
                            recording: inputs.recording,
                            maximum_selection_bytes: inputs.maximum_selection_bytes,
                        },
                        inputs.admit_reconciliation,
                        inputs.admit_rotation,
                        inputs.sign_observation,
                        inputs.sign_checkpoint,
                    )
                });
                match result {
                    Ok(Ok(_)) => self.release_completed_invoke_reservation(reservation),
                    Ok(Err(
                        super::provider_recovery::ResidentProcessProviderRecoveryFailure::Selection(
                            _,
                        )
                        | super::provider_recovery::ResidentProcessProviderRecoveryFailure::Preparation(
                            _,
                        ),
                    ))
                    | Err(_) => self.retain_reserved_invoke(
                        reservation,
                        RetainedInvokePhase::ProviderDispatched(recovered),
                        program::ResidentProgramRetainedPhaseTurn::Deferred,
                    ),
                    Ok(Err(failure)) => {
                        let state = retained_provider_recovery_state(failure)
                            .expect("advanced provider recovery retains its exact continuation");
                        self.retain_provider_recovery(
                            reservation,
                            key,
                            selector,
                            locator,
                            state,
                            program::ResidentProgramRetainedPhaseTurn::Advanced,
                        )
                    }
                }
            }
            RetainedInvokePhase::NativeDispatched(recovered) => {
                let selector = recovered.start.selector.clone();
                let locator = operation_locator_from_operation(
                    recovered.start.control.operation().operation(),
                );
                let (observation_ref, observation) =
                    recovered.start.control.observation().selected();
                let key = RetainedInvokePhaseKey {
                    locator: locator.clone(),
                    observation_ref: observation_ref.as_str().to_owned(),
                    observation_sha256: observation
                        .canonical_operation_observation_sha256()
                        .as_str()
                        .to_owned(),
                    phase: RetainedInvokePhaseKind::NativeRecording,
                };
                let checkpoint = recovered.start.control.observation().current_checkpoint();
                let predecessor_checkpoint = (checkpoint.0.clone(), checkpoint.1.clone());
                let result = self.lend_native_recovery_inputs(&selector, &locator, |inputs| {
                    process.recover_native(
                        access,
                        authority,
                        super::native_invoke::ResidentProcessNativeRecoveryInput {
                            attempt: &recovered,
                            current: inputs.current,
                            registry: inputs.registry,
                            contract_set: inputs.contract_set,
                            maximum_selection_bytes: inputs.maximum_selection_bytes,
                            maximum_record_bytes: inputs.maximum_record_bytes,
                            recording:
                                super::native_invoke::ResidentProcessNativeRecoveryRecordingInput {
                                    outcome_ref: inputs.outcome_ref,
                                    outcome_observation_ref: inputs.outcome_observation_ref,
                                    result_ref: inputs.result_ref,
                                    diagnostic_ref: inputs.diagnostic_ref,
                                    terminal_ref: inputs.terminal_ref,
                                    outcome_checkpoint: inputs.outcome_checkpoint,
                                    terminal_checkpoint: inputs.terminal_checkpoint,
                                    clock: inputs.clock,
                                    resolve_store: inputs.resolve_store,
                                    checkpoint_schema: inputs.checkpoint_schema,
                                    checkpoint_authority: inputs.checkpoint_authority,
                                    signing: inputs.signing,
                                    maximum_diagnostic_bytes: inputs.maximum_diagnostic_bytes,
                                    maximum_outcome_bytes: inputs.maximum_outcome_bytes,
                                    accumulator_limits: inputs.accumulator_limits,
                                },
                        },
                        &mut *inputs.admit_rotation,
                        &mut *inputs.sign_observation,
                        &mut *inputs.sign_checkpoint,
                        &mut *inputs.sign_result,
                    )
                });
                match result {
                    Ok(Ok(_)) => self.release_completed_invoke_reservation(reservation),
                    Ok(Err(
                        super::native_invoke::ResidentProcessNativeRecoveryFailure::Recording(
                            failure,
                        ),
                    )) => match retained_native_recording_state(failure) {
                        Some(state) => self.retain_native_recording(
                            reservation,
                            RetainedNativeRecordingPhase {
                                key,
                                selector,
                                locator,
                                predecessor_checkpoint,
                                state,
                            },
                            program::ResidentProgramRetainedPhaseTurn::Advanced,
                        ),
                        None => self.release_completed_invoke_reservation(reservation),
                    },
                    Ok(Err(_)) | Err(_) => self.retain_reserved_invoke(
                        reservation,
                        RetainedInvokePhase::NativeDispatched(recovered),
                        program::ResidentProgramRetainedPhaseTurn::Deferred,
                    ),
                }
            }
            RetainedInvokePhase::NativeRecording(recovery) => {
                let RetainedNativeRecordingPhase {
                    key,
                    selector,
                    locator,
                    predecessor_checkpoint,
                    state,
                } = *recovery;
                let mut state = Some(state);
                let result = self.lend_native_recovery_inputs(&selector, &locator, |inputs| {
                    continue_retained_native_recording(
                        process,
                        &selector,
                        access,
                        &predecessor_checkpoint,
                        state
                            .take()
                            .expect("the native recovery callback runs once"),
                        inputs,
                    )
                });
                match result {
                    Err(_) => self.retain_native_recording(
                        reservation,
                        RetainedNativeRecordingPhase {
                            key,
                            selector,
                            locator,
                            predecessor_checkpoint,
                            state: state
                                .take()
                                .expect("an uncalled lender preserves the exact native phase"),
                        },
                        program::ResidentProgramRetainedPhaseTurn::Deferred,
                    ),
                    Ok(RetainedNativeRecordingTurn::Complete) => {
                        self.release_completed_invoke_reservation(reservation)
                    }
                    Ok(RetainedNativeRecordingTurn::RestartRequired) => {
                        self.release_completed_invoke_reservation(reservation)
                    }
                    Ok(RetainedNativeRecordingTurn::Continue(state)) => self
                        .retain_native_recording(
                            reservation,
                            RetainedNativeRecordingPhase {
                                key,
                                selector,
                                locator,
                                predecessor_checkpoint,
                                state,
                            },
                            program::ResidentProgramRetainedPhaseTurn::Advanced,
                        ),
                }
            }
            RetainedInvokePhase::PreDispatchExpiry(recovery) => {
                let RetainedPreDispatchExpiryPhase {
                    key,
                    selector,
                    locator,
                    observed_at,
                    state,
                } = *recovery;
                let mut state = Some(state);
                let result = self.lend_pre_dispatch_expiry_continuation_inputs(
                    &selector,
                    &locator,
                    |inputs| {
                        let index = match process.deployment_authority_index(&selector) {
                            Ok(index) => index,
                            Err(_) => return Err(()),
                        };
                        if process.deployments[index].store() != access.store {
                            return Err(());
                        }
                        match crate::resident::operation_dispatch::continue_owned_operation_observation_custody(
                            state
                                .take()
                                .expect("the expiry custody lender runs once"),
                            observed_at,
                            inputs.checkpoint_schema,
                            inputs.checkpoint_authority,
                            inputs.resolve_store,
                            inputs.accumulator_limits,
                            access.custody,
                            access.anchor,
                            &mut *inputs.sign_checkpoint,
                        ) {
                            Ok(_) => Ok(()),
                            Err(pending) => {
                                state = Some(pending);
                                Err(())
                            }
                        }
                    },
                );
                match result {
                    Ok(Ok(())) => self.release_completed_invoke_reservation(reservation),
                    Err(_) | Ok(Err(())) => self.retain_pre_dispatch_expiry(
                        reservation,
                        RetainedPreDispatchExpiryPhase {
                            key,
                            selector,
                            locator,
                            observed_at,
                            state: state.expect("an incomplete expiry turn retains exact custody"),
                        },
                        program::ResidentProgramRetainedPhaseTurn::Deferred,
                    ),
                }
            }
            RetainedInvokePhase::ProviderRecovery(recovery) => {
                let RetainedProviderRecoveryPhase {
                    key,
                    selector,
                    locator,
                    state,
                } = *recovery;
                let previous = state.kind();
                match state {
                    RetainedProviderRecoveryState::Prepared(pending) => {
                        let mut pending = Some(pending);
                        let result =
                            self.lend_provider_recovery_inputs(&selector, &locator, |inputs| {
                                process.continue_prepared_provider_recovery(
                                    *pending
                                        .take()
                                        .expect("the provider recovery callback runs once"),
                                    access,
                                    inputs.recording.resolve_store,
                                    inputs.recording.checkpoint_authority,
                                    inputs.sign_checkpoint,
                                )
                            });
                        if result.is_err() {
                            return self.retain_provider_recovery(
                                reservation,
                                key,
                                selector,
                                locator,
                                RetainedProviderRecoveryState::Prepared(
                                    pending
                                        .take()
                                        .expect("an uncalled lender preserves the exact phase"),
                                ),
                                program::ResidentProgramRetainedPhaseTurn::Deferred,
                            );
                        }
                        self.settle_provider_recovery_result(
                            reservation,
                            key,
                            selector,
                            locator,
                            previous,
                            result.expect("handled above"),
                        )
                    }
                    RetainedProviderRecoveryState::Issued(pending) => {
                        let mut pending = Some(pending);
                        let result =
                            self.lend_provider_recovery_inputs(&selector, &locator, |inputs| {
                                process.continue_issued_provider_recovery(
                                    *pending
                                        .take()
                                        .expect("the provider recovery callback runs once"),
                                    access,
                                    inputs.recording.resolve_store,
                                )
                            });
                        if result.is_err() {
                            return self.retain_provider_recovery(
                                reservation,
                                key,
                                selector,
                                locator,
                                RetainedProviderRecoveryState::Issued(
                                    pending
                                        .take()
                                        .expect("an uncalled lender preserves the exact phase"),
                                ),
                                program::ResidentProgramRetainedPhaseTurn::Deferred,
                            );
                        }
                        self.settle_provider_recovery_result(
                            reservation,
                            key,
                            selector,
                            locator,
                            previous,
                            result.expect("handled above"),
                        )
                    }
                    RetainedProviderRecoveryState::Custody(pending) => {
                        match process
                            .continue_provider_recovery_custody(*pending, access, authority)
                        {
                            Ok(finalization) => self.retain_provider_recovery(
                                reservation,
                                key,
                                selector,
                                locator,
                                RetainedProviderRecoveryState::Finalization(Box::new(finalization)),
                                program::ResidentProgramRetainedPhaseTurn::Advanced,
                            ),
                            Err(failure) => self.retain_provider_recovery(
                                reservation,
                                key,
                                selector,
                                locator,
                                RetainedProviderRecoveryState::Custody(failure.pending),
                                program::ResidentProgramRetainedPhaseTurn::Deferred,
                            ),
                        }
                    }
                    RetainedProviderRecoveryState::Finalization(pending) => {
                        match process
                            .continue_provider_recovery_finalization(*pending, access.store)
                        {
                            Ok(_) => self.release_completed_invoke_reservation(reservation),
                            Err(failure) => self.retain_provider_recovery(
                                reservation,
                                key,
                                selector,
                                locator,
                                RetainedProviderRecoveryState::Finalization(failure.pending),
                                program::ResidentProgramRetainedPhaseTurn::Deferred,
                            ),
                        }
                    }
                }
            }
            _ => unreachable!("only explicitly runnable retained phases are selected"),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn settle_provider_recovery_result(
        &mut self,
        reservation: RetainedPhaseReservation<RetainedServicePhaseKey>,
        key: RetainedInvokePhaseKey,
        selector: super::ResidentDeploymentAuthoritySelector,
        locator: ResidentOperationFrameLocator,
        previous: RetainedProviderRecoveryStateKind,
        result: Result<
            crate::resident::ProviderOperationInvokeResult,
            super::provider_recovery::ResidentProcessProviderRecoveryFailure,
        >,
    ) -> program::ResidentProgramRetainedPhaseTurn {
        match result {
            Ok(_) => self.release_completed_invoke_reservation(reservation),
            Err(failure) => {
                let state = retained_provider_recovery_state(failure)
                    .expect("a provider continuation failure retains its exact continuation phase");
                let turn = if state.kind() == previous {
                    program::ResidentProgramRetainedPhaseTurn::Deferred
                } else {
                    program::ResidentProgramRetainedPhaseTurn::Advanced
                };
                self.retain_provider_recovery(reservation, key, selector, locator, state, turn)
            }
        }
    }

    fn continue_provider_resolved_continuation<WriterGuard>(
        &mut self,
        process: &ResidentProcessHost<'_, WriterGuard>,
        reservation: RetainedPhaseReservation<RetainedServicePhaseKey>,
        recovery: RetainedProviderResolvedPhase,
        mut access: super::ResidentDeploymentOperationAccess<'_>,
    ) -> program::ResidentProgramRetainedPhaseTurn {
        let RetainedProviderResolvedPhase {
            key,
            selector,
            locator,
            state,
        } = recovery;
        match state {
            RetainedProviderResolvedState::Preparation(preparation) => {
                let (attempt, checkpoint) = preparation.into_parts();
                self.continue_prepared_provider_resolved(
                    process,
                    reservation,
                    key,
                    selector,
                    locator,
                    access,
                    attempt,
                    checkpoint,
                )
            }
            RetainedProviderResolvedState::Prepared {
                attempt,
                checkpoint,
            } => self.continue_prepared_provider_resolved(
                process,
                reservation,
                key,
                selector,
                locator,
                access,
                *attempt,
                checkpoint,
            ),
            RetainedProviderResolvedState::Commit(pending) => {
                match process.reconcile_provider_resolved_attempt(
                    &selector,
                    reborrow_deployment_operation_access(&mut access),
                    *pending,
                ) {
                    Ok(Ok(pending)) => self.retain_provider_resolved_continuation(
                        reservation,
                        key,
                        selector,
                        locator,
                        RetainedProviderResolvedState::Anchor(Box::new(pending)),
                        program::ResidentProgramRetainedPhaseTurn::Advanced,
                    ),
                    Ok(Err(pending)) => self.retain_provider_resolved_continuation(
                        reservation,
                        key,
                        selector,
                        locator,
                        RetainedProviderResolvedState::Commit(pending),
                        program::ResidentProgramRetainedPhaseTurn::Deferred,
                    ),
                    Err((pending, _)) => self.retain_provider_resolved_continuation(
                        reservation,
                        key,
                        selector,
                        locator,
                        RetainedProviderResolvedState::Commit(pending),
                        program::ResidentProgramRetainedPhaseTurn::Deferred,
                    ),
                }
            }
            RetainedProviderResolvedState::Anchor(pending) => {
                match process.continue_provider_resolved_attempt_anchor(&selector, access, *pending)
                {
                    Ok(Ok(_)) => self.release_completed_invoke_reservation(reservation),
                    Ok(Err(pending)) => self.retain_provider_resolved_continuation(
                        reservation,
                        key,
                        selector,
                        locator,
                        RetainedProviderResolvedState::Anchor(pending),
                        program::ResidentProgramRetainedPhaseTurn::Deferred,
                    ),
                    Err((pending, _)) => self.retain_provider_resolved_continuation(
                        reservation,
                        key,
                        selector,
                        locator,
                        RetainedProviderResolvedState::Anchor(pending),
                        program::ResidentProgramRetainedPhaseTurn::Deferred,
                    ),
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn continue_prepared_provider_resolved<WriterGuard>(
        &mut self,
        process: &ResidentProcessHost<'_, WriterGuard>,
        reservation: RetainedPhaseReservation<RetainedServicePhaseKey>,
        key: RetainedInvokePhaseKey,
        selector: super::ResidentDeploymentAuthoritySelector,
        locator: ResidentOperationFrameLocator,
        mut access: super::ResidentDeploymentOperationAccess<'_>,
        attempt: crate::resident::operation_dispatch::PreparedRecoveredProviderAttempt,
        checkpoint: crate::resident::OperationCheckpointInput,
    ) -> program::ResidentProgramRetainedPhaseTurn {
        let mut pending = Some((attempt, checkpoint));
        let result = self.lend_provider_resolved_inputs(&selector, &locator, |inputs| {
            let (attempt, checkpoint) = pending
                .take()
                .expect("the provider-resolved lender runs once");
            process.continue_prepared_provider_resolved_attempt(
                &selector,
                reborrow_deployment_operation_access(&mut access),
                attempt,
                checkpoint,
                inputs.current.now,
                inputs.checkpoint_schema,
                inputs.checkpoint_authority,
                inputs.resolve_store,
                inputs.accumulator_limits,
                &mut *inputs.sign_checkpoint,
            )
        });
        let outcome = match result {
            Err(_) => {
                let (attempt, checkpoint) = pending
                    .take()
                    .expect("an uncalled lender preserves the signed attempt");
                return self.retain_provider_resolved_continuation(
                    reservation,
                    key,
                    selector,
                    locator,
                    RetainedProviderResolvedState::Prepared {
                        attempt: Box::new(attempt),
                        checkpoint,
                    },
                    program::ResidentProgramRetainedPhaseTurn::Deferred,
                );
            }
            Ok(Err((attempt, checkpoint, _))) => {
                return self.retain_provider_resolved_continuation(
                    reservation,
                    key,
                    selector,
                    locator,
                    RetainedProviderResolvedState::Prepared {
                        attempt,
                        checkpoint,
                    },
                    program::ResidentProgramRetainedPhaseTurn::Deferred,
                );
            }
            Ok(Ok(outcome)) => outcome,
        };
        match outcome {
            Ok(_) => self.release_completed_invoke_reservation(reservation),
            Err(failure) => self.retain_provider_resolved_continuation(
                reservation,
                key,
                selector,
                locator,
                retained_provider_attempt_custody_state(failure),
                program::ResidentProgramRetainedPhaseTurn::Advanced,
            ),
        }
    }
}

enum OperationResolutionTurnFailure {
    Preparation(crate::resident::operation_dispatch::OperationAttemptError),
    Custody(crate::resident::operation_dispatch::OwnedOperationObservationCustodyFailure),
}

enum ProviderResolvedTurnFailure {
    Preparation(
        super::provider_resolution_recovery::ResidentProcessProviderResolvedPreparationError,
    ),
    Retention(super::provider_resolution_recovery::ResidentProcessProviderResolvedRetentionFailure),
}

impl From<super::provider_resolution_recovery::ResidentProcessProviderResolvedPreparationError>
    for ProviderResolvedTurnFailure
{
    fn from(
        value: super::provider_resolution_recovery::ResidentProcessProviderResolvedPreparationError,
    ) -> Self {
        Self::Preparation(value)
    }
}

fn retained_provider_resolved_state(
    failure: super::provider_resolution_recovery::ResidentProcessProviderResolvedRetentionFailure,
) -> RetainedProviderResolvedState {
    match failure {
        super::provider_resolution_recovery::ResidentProcessProviderResolvedRetentionFailure::Selection {
            preparation,
            ..
        } => RetainedProviderResolvedState::Preparation(preparation),
        super::provider_resolution_recovery::ResidentProcessProviderResolvedRetentionFailure::Custody(
            failure,
        ) => retained_provider_attempt_custody_state(failure),
    }
}

fn retained_provider_attempt_custody_state(
    failure: crate::resident::operation_dispatch::RecoveredProviderAttemptCustodyFailure,
) -> RetainedProviderResolvedState {
    match failure {
        crate::resident::operation_dispatch::RecoveredProviderAttemptCustodyFailure::Prepared {
            attempt,
            checkpoint,
            ..
        } => RetainedProviderResolvedState::Prepared {
            attempt,
            checkpoint,
        },
        crate::resident::operation_dispatch::RecoveredProviderAttemptCustodyFailure::Commit(
            pending,
        ) => RetainedProviderResolvedState::Commit(pending),
        crate::resident::operation_dispatch::RecoveredProviderAttemptCustodyFailure::Anchor(
            pending,
        ) => RetainedProviderResolvedState::Anchor(pending),
    }
}

fn continue_native_prepared_custody<WriterGuard>(
    process: &ResidentProcessHost<'_, WriterGuard>,
    selector: &super::ResidentDeploymentAuthoritySelector,
    mut access: super::ResidentDeploymentOperationAccess<'_>,
    state: RetainedNativePreparedState,
    inputs: &mut ResidentServiceNativePreparedInputs<'_>,
) -> Result<(), RetainedNativePreparedState> {
    match state {
        RetainedNativePreparedState::Dispatch(dispatch) => {
            let prepared = match process.prepare_recovered_dispatch_checkpoint(
                selector,
                reborrow_deployment_operation_access(&mut access),
                &dispatch,
                inputs.resolve_store,
                inputs.accumulator_limits,
            ) {
                Ok(Ok(prepared)) => prepared,
                Ok(Err(_)) | Err(_) => {
                    return Err(RetainedNativePreparedState::Dispatch(dispatch));
                }
            };
            let issued = match process.issue_recovered_dispatch_checkpoint(
                selector,
                reborrow_deployment_operation_access(&mut access),
                prepared,
                crate::resident::OperationCheckpointInput {
                    reference: inputs.checkpoint.reference.clone(),
                    id: inputs.checkpoint.id.clone(),
                },
                inputs.checkpoint_issued_at,
                inputs.checkpoint_schema,
                inputs.checkpoint_authority,
                &mut *inputs.sign_checkpoint,
            ) {
                Ok(Ok(issued)) => issued,
                Ok(Err(_)) | Err(_) => {
                    return Err(RetainedNativePreparedState::Dispatch(dispatch));
                }
            };
            match process.retain_recovered_dispatch(
                selector,
                reborrow_deployment_operation_access(&mut access),
                issued,
                inputs.resolve_store,
                inputs.accumulator_limits,
            ) {
                Ok(Ok(_)) => Ok(()),
                Ok(Err(failure)) => Err(retained_native_prepared_custody_state(failure)),
                Err((issued, _)) => Err(RetainedNativePreparedState::Issued(issued)),
            }
        }
        RetainedNativePreparedState::Issued(continuation) => {
            match process.retain_recovered_dispatch(
                selector,
                reborrow_deployment_operation_access(&mut access),
                *continuation,
                inputs.resolve_store,
                inputs.accumulator_limits,
            ) {
                Ok(Ok(_)) => Ok(()),
                Ok(Err(failure)) => Err(retained_native_prepared_custody_state(failure)),
                Err((issued, _)) => Err(RetainedNativePreparedState::Issued(issued)),
            }
        }
        RetainedNativePreparedState::Commit(pending) => {
            match process.reconcile_recovered_dispatch(
                selector,
                reborrow_deployment_operation_access(&mut access),
                *pending,
            ) {
                Ok(Ok(pending)) => Err(RetainedNativePreparedState::Anchor(Box::new(pending))),
                Ok(Err(pending)) => Err(RetainedNativePreparedState::Commit(pending)),
                Err((pending, _)) => Err(RetainedNativePreparedState::Commit(pending)),
            }
        }
        RetainedNativePreparedState::Anchor(pending) => {
            match process.continue_recovered_dispatch_anchor(selector, access, *pending) {
                Ok(Ok(_)) => Ok(()),
                Ok(Err(pending)) => Err(RetainedNativePreparedState::Anchor(pending)),
                Err((pending, _)) => Err(RetainedNativePreparedState::Anchor(pending)),
            }
        }
    }
}

fn retained_native_prepared_custody_state(
    failure: crate::resident::operation_dispatch::RecoveredDispatchCustodyFailure,
) -> RetainedNativePreparedState {
    match failure {
        crate::resident::operation_dispatch::RecoveredDispatchCustodyFailure::Issued {
            continuation,
            ..
        } => RetainedNativePreparedState::Issued(continuation),
        crate::resident::operation_dispatch::RecoveredDispatchCustodyFailure::Commit(pending) => {
            RetainedNativePreparedState::Commit(pending)
        }
        crate::resident::operation_dispatch::RecoveredDispatchCustodyFailure::Anchor(pending) => {
            RetainedNativePreparedState::Anchor(pending)
        }
    }
}

fn continue_provider_prepared_custody<WriterGuard, Authority>(
    process: &mut ResidentProcessHost<'_, WriterGuard>,
    selector: &super::ResidentDeploymentAuthoritySelector,
    mut access: super::ResidentDeploymentOperationAccess<'_>,
    state: RetainedProviderPreparedState,
    inputs: &mut ResidentServiceProviderPreparedInputs<'_>,
    authority: &mut Authority,
) -> Result<ProviderPreparedTurn, RetainedProviderPreparedState>
where
    Authority: AmbientSessionAuthorityPort,
{
    let state = match state {
        RetainedProviderPreparedState::Dispatch(dispatch) => {
            let prepared = match process.prepare_recovered_dispatch_checkpoint(
                selector,
                reborrow_deployment_operation_access(&mut access),
                dispatch.dispatch(),
                inputs.resolve_store,
                inputs.accumulator_limits,
            ) {
                Ok(Ok(prepared)) => prepared,
                Ok(Err(_)) | Err(_) => {
                    return Err(RetainedProviderPreparedState::Dispatch(dispatch));
                }
            };
            let issued = match process.issue_recovered_dispatch_checkpoint(
                selector,
                reborrow_deployment_operation_access(&mut access),
                prepared,
                crate::resident::OperationCheckpointInput {
                    reference: inputs.checkpoint.reference.clone(),
                    id: inputs.checkpoint.id.clone(),
                },
                inputs.checkpoint_issued_at,
                inputs.checkpoint_schema,
                inputs.checkpoint_authority,
                &mut *inputs.sign_checkpoint,
            ) {
                Ok(Ok(issued)) => issued,
                Ok(Err(_)) | Err(_) => {
                    return Err(RetainedProviderPreparedState::Dispatch(dispatch));
                }
            };
            match process.retain_recovered_dispatch(
                selector,
                reborrow_deployment_operation_access(&mut access),
                issued,
                inputs.resolve_store,
                inputs.accumulator_limits,
            ) {
                Ok(Ok(custody)) => match process.complete_recovered_provider_release(
                    selector,
                    access.store,
                    *dispatch,
                    custody,
                ) {
                    Ok(release) => RetainedProviderPreparedState::Release(Box::new(release)),
                    Err(failure) => {
                        return Err(RetainedProviderPreparedState::Dispatch(failure.dispatch));
                    }
                },
                Ok(Err(failure)) => {
                    return Err(retained_provider_prepared_custody_state(dispatch, failure));
                }
                Err((issued, _)) => {
                    return Err(RetainedProviderPreparedState::Issued {
                        dispatch,
                        pending: issued,
                    });
                }
            }
        }
        RetainedProviderPreparedState::Issued { dispatch, pending } => {
            match process.retain_recovered_dispatch(
                selector,
                reborrow_deployment_operation_access(&mut access),
                *pending,
                inputs.resolve_store,
                inputs.accumulator_limits,
            ) {
                Ok(Ok(custody)) => match process.complete_recovered_provider_release(
                    selector,
                    access.store,
                    *dispatch,
                    custody,
                ) {
                    Ok(release) => RetainedProviderPreparedState::Release(Box::new(release)),
                    Err(failure) => {
                        return Err(RetainedProviderPreparedState::Dispatch(failure.dispatch));
                    }
                },
                Ok(Err(failure)) => {
                    return Err(retained_provider_prepared_custody_state(dispatch, failure));
                }
                Err((pending, _)) => {
                    return Err(RetainedProviderPreparedState::Issued { dispatch, pending });
                }
            }
        }
        RetainedProviderPreparedState::Commit { dispatch, pending } => {
            match process.reconcile_recovered_dispatch(
                selector,
                reborrow_deployment_operation_access(&mut access),
                *pending,
            ) {
                Ok(Ok(pending)) => {
                    return Err(RetainedProviderPreparedState::Anchor {
                        dispatch,
                        pending: Box::new(pending),
                    });
                }
                Ok(Err(pending)) => {
                    return Err(RetainedProviderPreparedState::Commit { dispatch, pending });
                }
                Err((pending, _)) => {
                    return Err(RetainedProviderPreparedState::Commit { dispatch, pending });
                }
            }
        }
        RetainedProviderPreparedState::Anchor { dispatch, pending } => {
            match process.continue_recovered_dispatch_anchor(
                selector,
                reborrow_deployment_operation_access(&mut access),
                *pending,
            ) {
                Ok(Ok(custody)) => match process.complete_recovered_provider_release(
                    selector,
                    access.store,
                    *dispatch,
                    custody,
                ) {
                    Ok(release) => RetainedProviderPreparedState::Release(Box::new(release)),
                    Err(failure) => {
                        return Err(RetainedProviderPreparedState::Dispatch(failure.dispatch));
                    }
                },
                Ok(Err(pending)) => {
                    return Err(RetainedProviderPreparedState::Anchor { dispatch, pending });
                }
                Err((pending, _)) => {
                    return Err(RetainedProviderPreparedState::Anchor { dispatch, pending });
                }
            }
        }
        state @ RetainedProviderPreparedState::Release(_)
        | state @ RetainedProviderPreparedState::Launch(_) => state,
    };
    let release = match state {
        RetainedProviderPreparedState::Release(release) => *release,
        RetainedProviderPreparedState::Launch(failure) => match *failure {
            super::ResidentProcessRecoveredProviderLaunchFailure::Selection { release, .. }
            | super::ResidentProcessRecoveredProviderLaunchFailure::RouteSaturated(release) => {
                *release
            }
            failure @ super::ResidentProcessRecoveredProviderLaunchFailure::Acceptance {
                ..
            } => {
                return Err(RetainedProviderPreparedState::Launch(Box::new(failure)));
            }
            super::ResidentProcessRecoveredProviderLaunchFailure::AcceptedWithFailure {
                ..
            } => return Ok(ProviderPreparedTurn::Accepted),
        },
        _ => unreachable!("custody completion produces a provider release"),
    };
    let mut executor = ResidentServiceProviderTaskExecutorAdapter {
        executor: inputs.executor,
    };
    let mut driver = match process.provider_recovery_driver(selector) {
        Ok(driver) => driver,
        Err(_) => return Err(RetainedProviderPreparedState::Release(Box::new(release))),
    };
    match driver.launch(
        access,
        authority,
        release,
        &inputs.invocation,
        &mut executor,
    ) {
        Ok(_) => Ok(ProviderPreparedTurn::Accepted),
        Err(failure) => match failure.as_ref() {
            super::ResidentProcessRecoveredProviderLaunchFailure::AcceptedWithFailure {
                ..
            } => Ok(ProviderPreparedTurn::Accepted),
            _ => Err(RetainedProviderPreparedState::Launch(failure)),
        },
    }
}

fn retained_provider_prepared_custody_state(
    dispatch: Box<crate::resident::operation_dispatch::PreparedRecoveredProviderDispatch>,
    failure: crate::resident::operation_dispatch::RecoveredDispatchCustodyFailure,
) -> RetainedProviderPreparedState {
    match failure {
        crate::resident::operation_dispatch::RecoveredDispatchCustodyFailure::Issued {
            continuation,
            ..
        } => RetainedProviderPreparedState::Issued {
            dispatch,
            pending: continuation,
        },
        crate::resident::operation_dispatch::RecoveredDispatchCustodyFailure::Commit(pending) => {
            RetainedProviderPreparedState::Commit { dispatch, pending }
        }
        crate::resident::operation_dispatch::RecoveredDispatchCustodyFailure::Anchor(pending) => {
            RetainedProviderPreparedState::Anchor { dispatch, pending }
        }
    }
}

fn retained_source_key(
    start: &restart::ResidentProcessRecoveredOperationStart,
    locator: ResidentOperationFrameLocator,
    phase: RetainedInvokePhaseKind,
) -> RetainedInvokePhaseKey {
    let (observation_ref, observation) = start.control.observation().selected();
    RetainedInvokePhaseKey {
        locator,
        observation_ref: observation_ref.as_str().to_owned(),
        observation_sha256: observation
            .canonical_operation_observation_sha256()
            .as_str()
            .to_owned(),
        phase,
    }
}

fn retained_provider_recovery_state(
    failure: super::provider_recovery::ResidentProcessProviderRecoveryFailure,
) -> Option<RetainedProviderRecoveryState> {
    match failure {
        super::provider_recovery::ResidentProcessProviderRecoveryFailure::Selection(_)
        | super::provider_recovery::ResidentProcessProviderRecoveryFailure::Preparation(_) => None,
        super::provider_recovery::ResidentProcessProviderRecoveryFailure::Prepared(pending) => {
            Some(RetainedProviderRecoveryState::Prepared(pending))
        }
        super::provider_recovery::ResidentProcessProviderRecoveryFailure::Issued(pending) => {
            Some(RetainedProviderRecoveryState::Issued(pending))
        }
        super::provider_recovery::ResidentProcessProviderRecoveryFailure::Custody(pending) => {
            Some(RetainedProviderRecoveryState::Custody(pending))
        }
        super::provider_recovery::ResidentProcessProviderRecoveryFailure::Finalization(failure) => {
            Some(RetainedProviderRecoveryState::Finalization(failure.pending))
        }
    }
}

fn retained_native_recording_state(
    failure: crate::resident::NativeOperationRecordingFailure,
) -> Option<RetainedNativeRecordingState> {
    use crate::resident::NativeOperationRecordingFailure as Failure;
    Some(match failure {
        Failure::Unstaged { outcome, .. } => RetainedNativeRecordingState::Unstaged(outcome),
        Failure::StagePreparation { continuation, .. } => {
            RetainedNativeRecordingState::StagePreparation(continuation)
        }
        Failure::PreparedStage { continuation, .. } => {
            RetainedNativeRecordingState::PreparedStage(continuation)
        }
        Failure::IssuedStage { continuation, .. } => {
            RetainedNativeRecordingState::IssuedStage(continuation)
        }
        Failure::StagingCommit(failure) => RetainedNativeRecordingState::StagingCommit(failure),
        Failure::StagingAnchor(pending) => RetainedNativeRecordingState::StagingAnchor(pending),
        Failure::TerminalPreparation { staged, .. } => {
            RetainedNativeRecordingState::TerminalPreparation(staged)
        }
        Failure::TerminalCustody { failure } => {
            RetainedNativeRecordingState::TerminalCustody(failure)
        }
        // A failed semantic admission retains no callable continuation. The
        // durable staged head remains recoverable through the ordinary restart
        // owner, so the in-memory phase must not fabricate one.
        Failure::StageFinalization(_) => return None,
    })
}

impl<'service, 'recovery, InvokeInputs, DelegateInputs, CancellationInputs, ObserveInputs>
    program::ResidentProgramObserveInputPort<'recovery>
    for InvokeRecoveryAdapter<
        'service,
        InvokeInputs,
        DelegateInputs,
        CancellationInputs,
        ObserveInputs,
    >
where
    ObserveInputs: ResidentServiceObserveInputPort<'recovery>,
{
    fn with_observe_inputs<Output, UseInputs>(
        &mut self,
        locator: &ControlRecordCoordinate,
        use_inputs: UseInputs,
    ) -> Result<Output, ResidentObserveFrameFailure>
    where
        UseInputs: for<'request> FnOnce(
            program::ResidentProgramObserveInputs<'request, 'recovery>,
        ) -> Output,
    {
        let mut input_owner = self
            .observe_inputs
            .take()
            .ok_or(ResidentObserveFrameFailure::ResidentUnavailable)?;
        let result = input_owner.with_observe_inputs(locator, |inputs| {
            let mut replay = |request_id: &ResidentRequestIdV1,
                              digest: &CommonRequestEnvelopeDigest,
                              bytes: &[u8]| {
                (inputs.admit_replay)(request_id, digest, bytes)
                    .map(|outcome| match outcome {
                        ResidentServiceObserveReplayDisposition::FirstAdmission => {
                            super::observe_request::ResidentObserveReplayDisposition::FirstAdmission
                        }
                        ResidentServiceObserveReplayDisposition::ExactReplay => {
                            super::observe_request::ResidentObserveReplayDisposition::ExactReplay
                        }
                    })
                    .map_err(|error| match error {
                        ResidentServiceObserveReplayError::ChangedBytes => {
                            super::observe_request::ResidentObserveReplayAdmissionError::ChangedBytes
                        }
                        ResidentServiceObserveReplayError::Unavailable => {
                            super::observe_request::ResidentObserveReplayAdmissionError::Unavailable
                        }
                    })
            };
            use_inputs(program::ResidentProgramObserveInputs {
                recovery: inputs.recovery,
                verify_caller_signature: inputs.verify_caller_signature,
                admit_replay: &mut replay,
                recovery_admission: inputs.recovery_admission,
                authorize_disclosure: inputs.authorize_disclosure,
                response_deadline: inputs.response_deadline,
            })
        });
        self.observe_inputs = Some(input_owner);
        result
    }
}

impl<'service, WriterGuard, InvokeInputs, DelegateInputs, CancellationInputs, ObserveInputs>
    operation_recovery_sweep::ResidentProcessRootProviderRecoveryOwner<WriterGuard>
    for InvokeRecoveryAdapter<
        'service,
        InvokeInputs,
        DelegateInputs,
        CancellationInputs,
        ObserveInputs,
    >
{
    type Output = operation_request_host::ResidentOperationRequestOutcome;
    type Error = ResidentOperationFrameFailure;

    fn complete_local_cancellation<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<restart::ResidentProcessRecoveredLocalCancellation>,
        _: super::ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        self.retain(RetainedInvokePhase::LocalCancellation(recovered))
    }

    fn recover_operation_resolution<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<restart::ResidentProcessRecoveredOperationResolution>,
        _: super::ResidentDeploymentOperationAccess<'_>,
    ) -> Result<
        operation_recovery_sweep::ResidentProcessRootProviderResolution<Self::Output>,
        Self::Error,
    > {
        self.retain(RetainedInvokePhase::OperationResolution(recovered))
            .map(operation_recovery_sweep::ResidentProcessRootProviderResolution::Provider)
    }

    fn recover_provider_resolved<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<restart::ResidentProcessRecoveredProviderResolution>,
        _: super::ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        self.retain(RetainedInvokePhase::ProviderResolved(recovered))
    }

    fn recover_provider_prepared<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<restart::ResidentProcessRecoveredPreparedAttempt>,
        _: super::ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        self.retain(RetainedInvokePhase::ProviderPrepared(recovered))
    }

    fn recover_provider_dispatched<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<restart::ResidentProcessRecoveredAttempt>,
        _: super::ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        self.retain(RetainedInvokePhase::ProviderDispatched(recovered))
    }

    fn recover_native_prepared<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<restart::ResidentProcessRecoveredPreparedAttempt>,
        _: super::ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        self.retain(RetainedInvokePhase::NativePrepared(recovered))
    }

    fn recover_native_dispatched<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<restart::ResidentProcessRecoveredAttempt>,
        _: super::ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        self.retain(RetainedInvokePhase::NativeDispatched(recovered))
    }
}

/// Fail-closed Delegate continuation owner for compositions without A2A inputs.
pub(crate) struct UnavailableResidentServiceDelegateInputs;

enum ResidentServiceRetainedDelegatePhase<'service> {
    Resolution(Box<restart::ResidentProcessRecoveredOperationResolution>),
    PreDispatchExpiry(Box<RetainedDelegatePreDispatchExpiryPhase>),
    Launch(ResidentProcessA2aDelegateLaunch),
    Poll(super::ResidentProcessA2aDelegatePoll),
    LaunchFailure(Box<ResidentProcessPairwiseA2aDelegateLaunchFailure<'service, 'service>>),
    RestartFailure(Box<ResidentProcessA2aDelegateRestartFailure<'service>>),
    Recovered(Box<ResidentProcessRecoveredA2aDelegate<'service>>),
    RetentionFailure(Box<ResidentProcessA2aDelegateRetentionFailure<'service>>),
    Task(Box<restart::ResidentProcessRecoveredA2aTask>),
    CancelPrephysical(Box<restart::ResidentProcessRecoveredA2aCancelTaskRequest>),
    CancelCustody(Box<super::a2a_cancel_task::ResidentProcessRecoveredA2aCancelTaskCustody>),
}

struct RetainedDelegatePreDispatchExpiryPhase {
    observed_at: UnixTimeV1,
    state: crate::resident::operation_dispatch::OwnedOperationObservationCustodyFailure,
}

struct ResidentServiceRetainedDelegate<'service> {
    selector: super::ResidentDeploymentAuthoritySelector,
    locator: ResidentOperationFrameLocator,
    phase: Option<ResidentServiceRetainedDelegatePhase<'service>>,
}

impl ResidentServiceRetainedDelegatePhase<'_> {
    /// Whether this phase can move without issuing or replaying a request.
    ///
    /// An awaiting response is runnable only through fresh poll inputs. That
    /// poll continues the retained exchange and cannot construct `SendMessage`.
    fn is_runnable(&self) -> bool {
        match self {
            Self::Resolution(_) => false,
            Self::PreDispatchExpiry(_) => true,
            Self::Launch(
                ResidentProcessA2aDelegateLaunch::Awaiting(_)
                | ResidentProcessA2aDelegateLaunch::Completed(_)
                | ResidentProcessA2aDelegateLaunch::Diagnostic(_),
            ) => true,
            Self::Poll(_) => true,
            Self::LaunchFailure(failure) => matches!(
                failure.as_ref(),
                ResidentProcessPairwiseA2aDelegateLaunchFailure::A2a(lower)
                    if matches!(
                        lower.as_ref(),
                        ResidentProcessA2aDelegateLaunchFailure::Rollback { .. }
                            | ResidentProcessA2aDelegateLaunchFailure::Custody { .. }
                            | ResidentProcessA2aDelegateLaunchFailure::StartedAssociation { .. }
                            | ResidentProcessA2aDelegateLaunchFailure::OfferSettlement { .. }
                    )
            ),
            Self::RestartFailure(failure) => matches!(
                failure.as_ref(),
                ResidentProcessA2aDelegateRestartFailure::Rollback { .. }
                    | ResidentProcessA2aDelegateRestartFailure::Loss { .. }
            ),
            Self::Recovered(_)
            | Self::RetentionFailure(_)
            | Self::Task(_)
            | Self::CancelPrephysical(_)
            | Self::CancelCustody(_) => true,
        }
    }
}

fn retained_cancel_recovery_phase<'service>(
    recovered: Box<restart::ResidentProcessRecoveredA2aCancelTaskRequest>,
    failure: ResidentProcessRecoveredA2aCancelTaskFailure,
) -> ResidentServiceRetainedDelegatePhase<'service> {
    match failure {
        ResidentProcessRecoveredA2aCancelTaskFailure::Selection(_)
        | ResidentProcessRecoveredA2aCancelTaskFailure::Preparation(_) => {
            ResidentServiceRetainedDelegatePhase::CancelPrephysical(recovered)
        }
        ResidentProcessRecoveredA2aCancelTaskFailure::Custody(pending) => {
            ResidentServiceRetainedDelegatePhase::CancelCustody(pending)
        }
    }
}

/// Service-lifetime access to retained A2A phases.
///
/// This private port exists separately from cold-recovery routing. It advances
/// only an exact phase already held by the resident service.
pub(in crate::resident::process_host) trait ResidentServiceRetainedDelegatePhasePort {
    fn has_runnable_retained_delegate(&self) -> bool;

    fn retained_delegate_selector(&self) -> Option<&super::ResidentDeploymentAuthoritySelector>;

    fn continue_one_retained_delegate<WriterGuard, Authority>(
        &mut self,
        process: &mut ResidentProcessHost<'_, WriterGuard>,
        access: super::ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
    ) -> program::ResidentProgramRetainedPhaseTurn
    where
        WriterGuard: PrincipalWriterGuardPort,
        Authority: AmbientSessionAuthorityPort;
}

/// Private adapter from public physical inputs to exact resident A2A phases.
pub(in crate::resident::process_host) struct ResidentServiceDelegateRecoveryAdapter<
    'service,
    Inputs,
> {
    inputs: Option<Inputs>,
    retained: Vec<ResidentServiceRetainedDelegate<'service>>,
    maximum_retained: usize,
}

impl<'service, Inputs> ResidentServiceDelegateRecoveryAdapter<'service, Inputs> {
    pub(in crate::resident::process_host) fn new(inputs: Inputs, maximum_retained: usize) -> Self {
        Self {
            inputs: Some(inputs),
            retained: Vec::with_capacity(maximum_retained),
            maximum_retained,
        }
    }

    fn retain(
        &mut self,
        selector: super::ResidentDeploymentAuthoritySelector,
        locator: ResidentOperationFrameLocator,
        phase: ResidentServiceRetainedDelegatePhase<'service>,
    ) -> Result<
        operation_request_host::ResidentOperationRequestOutcome,
        ResidentOperationFrameFailure,
    > {
        if self.retained.len() >= self.maximum_retained
            || self
                .retained
                .iter()
                .any(|retained| retained.selector == selector && retained.locator == locator)
        {
            return Err(ResidentOperationFrameFailure::ResidentUnavailable);
        }
        self.retained.push(ResidentServiceRetainedDelegate {
            selector,
            locator: locator.clone(),
            phase: Some(phase),
        });
        Ok(operation_request_host::ResidentOperationRequestOutcome::Pending(locator))
    }

    fn next_runnable_retained_delegate(
        &self,
    ) -> Option<&ResidentServiceRetainedDelegate<'service>> {
        self.retained.iter().find(|retained| {
            retained
                .phase
                .as_ref()
                .is_some_and(ResidentServiceRetainedDelegatePhase::is_runnable)
        })
    }
}

#[cfg(test)]
#[path = "../../../tests/support/resident_service_delegate_recovery.rs"]
pub(in crate::resident::process_host) mod delegate_recovery_test_support;

impl<'service, WriterGuard, Inputs> ResidentA2aDelegateRecoveryPort<WriterGuard>
    for ResidentServiceDelegateRecoveryAdapter<'service, Inputs>
where
    Inputs: ResidentServiceDelegateInputPort<'service>,
{
    type Output = operation_request_host::ResidentOperationRequestOutcome;
    type Error = ResidentOperationFrameFailure;

    fn recover_delegate_resolution<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<restart::ResidentProcessRecoveredOperationResolution>,
        mut access: super::ResidentDeploymentOperationAccess<'_>,
        input: program::ResidentProgramDelegateResolutionInput<'_, '_>,
    ) -> Result<Self::Output, Self::Error> {
        let selector = recovered.start.selector.clone();
        let deployment = service_deployment_selector(&selector);
        let locator =
            operation_locator_from_operation(recovered.start.control.operation().operation());
        let expiry = self
            .inputs
            .as_mut()
            .expect("the Delegate input owner is returned after every loan")
            .with_pre_dispatch_expiry_preparation_inputs(&deployment, &locator, |inputs| {
                run_pre_dispatch_expiry_turn(host, &recovered.start, &mut access, inputs)
            });
        match expiry {
            Ok(Ok(PreDispatchExpiryTurn::NotExpired)) => {}
            Ok(Ok(PreDispatchExpiryTurn::Complete)) => {
                return Ok(
                    operation_request_host::ResidentOperationRequestOutcome::Pending(locator),
                );
            }
            Ok(Ok(PreDispatchExpiryTurn::Pending { observed_at, state })) => {
                return self.retain(
                    selector,
                    locator,
                    ResidentServiceRetainedDelegatePhase::PreDispatchExpiry(Box::new(
                        RetainedDelegatePreDispatchExpiryPhase { observed_at, state },
                    )),
                );
            }
            Ok(Err(_)) | Err(_) => {
                return self.retain(
                    selector,
                    locator,
                    ResidentServiceRetainedDelegatePhase::Resolution(recovered),
                );
            }
        }
        let attempted = self
            .inputs
            .as_mut()
            .expect("the Delegate input owner is returned after every loan")
            .with_initial_launch_inputs(&deployment, &locator, |inputs| {
                let ResidentServiceA2aDelegateLaunchInputs {
                    authority,
                    dispatch,
                    plan,
                    exchange,
                    admit_rotation,
                    sign_observation,
                    sign_provider_checkpoint,
                    admit_disclosure,
                    admit_transport,
                    sign_a2a_checkpoint,
                } = inputs;
                let resolution =
                    super::operation_resolution::ResidentProcessOperationResolutionInput {
                        session: input.session(),
                        current: input.current(),
                        signing: dispatch.signing,
                        selection_ref: dispatch.operation.context.selection_ref.clone(),
                        observation_ref: dispatch.operation.context.observation_ref.clone(),
                        maximum_selection_bytes: input
                            .maximum_selection_bytes()
                            .min(dispatch.operation.context.maximum_selection_bytes),
                        maximum_record_bytes: input.maximum_record_bytes(),
                    };
                host.launch_recovered_pairwise_a2a_delegate(
                    &recovered,
                    reborrow_deployment_operation_access(&mut access),
                    resolution,
                    None,
                    authority,
                    dispatch,
                    || ResidentProcessA2aDelegateLaunchPlan {
                        current: plan.current,
                        route: plan.route,
                        route_predecessor: plan.route_predecessor,
                        route_association: plan.route_association,
                        request_id: plan.request_id,
                        message_id: plan.message_id,
                        text: plan.text,
                        request_evidence_ref: plan.request_evidence_ref,
                        preparation_ref: plan.preparation_ref,
                        exchange_limits: plan.exchange_limits,
                        maximum_evidence_bytes: plan.maximum_evidence_bytes,
                        maximum_record_bytes: plan.maximum_record_bytes,
                        maximum_dispatch_record_bytes: plan.maximum_dispatch_record_bytes,
                        resolve_store: plan.resolve_store,
                        checkpoint: plan.checkpoint,
                        checkpoint_schema: plan.checkpoint_schema,
                        checkpoint_authority: plan.checkpoint_authority,
                        response_ref: plan.response_ref,
                        clock: plan.clock,
                        accumulator_limits: plan.accumulator_limits,
                        membership_limits: plan.membership_limits,
                    },
                    exchange,
                    admit_rotation,
                    sign_observation,
                    sign_provider_checkpoint,
                    admit_disclosure,
                    move |reference, request, digest| admit_transport(reference, request, digest),
                    move |bytes, preimage, digest| sign_a2a_checkpoint(bytes, preimage, digest),
                )
            })?;
        match attempted {
            Ok(Ok(launch)) => self.retain(
                selector,
                locator,
                ResidentServiceRetainedDelegatePhase::Launch(launch),
            ),
            Ok(Err(failure)) => self.retain(
                selector,
                locator,
                ResidentServiceRetainedDelegatePhase::LaunchFailure(Box::new(failure)),
            ),
            Err(_) => self.retain(
                selector,
                locator,
                ResidentServiceRetainedDelegatePhase::Resolution(recovered),
            ),
        }
    }

    fn recover_pre_task<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<restart::ResidentProcessRecoveredAttempt>,
        mut access: super::ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        let selector = recovered.start.selector.clone();
        let deployment = service_deployment_selector(&selector);
        let locator =
            operation_locator_from_operation(recovered.start.control.operation().operation());
        let attempted = self
            .inputs
            .as_mut()
            .expect("the Delegate input owner is returned after every loan")
            .with_pre_task_recovery_inputs(&deployment, &locator, |inputs| {
                let recovered = host.recover_restarted_a2a_delegate(
                    *recovered,
                    reborrow_deployment_operation_access(&mut access),
                    inputs.authority,
                    inputs.recovery,
                    inputs.admission,
                )?;
                let ResidentProcessRecoveredA2aDelegate { route, recovered } = recovered;
                let crate::resident::a2a_delegate_task_service::ResidentRecoveredA2aDelegate {
                    outcome,
                    finalization,
                } = recovered;
                let retention = (inputs.retain)(outcome);
                host.retain_a2a_delegate_outcome(
                    reborrow_deployment_operation_access(&mut access),
                    inputs.authority,
                    ResidentProcessA2aDelegateRetention::RecoveredOutcomeUnknown {
                        route,
                        finalization,
                        input: retention,
                    },
                )
                .map_err(ResidentServicePreTaskFailure::Retention)
            })?;
        match attempted {
            Ok(_) => Ok(operation_request_host::ResidentOperationRequestOutcome::Pending(locator)),
            Err(ResidentServicePreTaskFailure::Recovery(failure)) => self.retain(
                selector,
                locator,
                ResidentServiceRetainedDelegatePhase::RestartFailure(failure),
            ),
            Err(ResidentServicePreTaskFailure::Retention(failure)) => self.retain(
                selector,
                locator,
                ResidentServiceRetainedDelegatePhase::RetentionFailure(failure),
            ),
        }
    }

    fn recover_task<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<restart::ResidentProcessRecoveredA2aTask>,
        access: super::ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        let selector = recovered.attempt.start.selector.clone();
        let deployment = service_deployment_selector(&selector);
        let locator = operation_locator_from_operation(
            recovered.attempt.start.control.operation().operation(),
        );
        let attempted = self
            .inputs
            .as_mut()
            .expect("the Delegate input owner is returned after every loan")
            .with_task_observation_inputs(&deployment, &locator, |inputs| {
                host.recover_a2a_get_task_disposition(
                    &recovered,
                    &recovered.attempt.start.selector,
                    access,
                    inputs.authority,
                    inputs.maximum_record_bytes,
                )
            })?;
        match attempted {
            Ok(_) => Ok(operation_request_host::ResidentOperationRequestOutcome::Pending(locator)),
            Err(_) => self.retain(
                selector,
                locator,
                ResidentServiceRetainedDelegatePhase::Task(recovered),
            ),
        }
    }

    fn recover_cancel_task<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<restart::ResidentProcessRecoveredA2aCancelTaskRequest>,
        access: super::ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        let selector = recovered.attempt.start.selector.clone();
        let deployment = service_deployment_selector(&selector);
        let locator = operation_locator_from_operation(
            recovered.attempt.start.control.operation().operation(),
        );
        let attempted = self
            .inputs
            .as_mut()
            .expect("the Delegate input owner is returned after every loan")
            .with_cancel_recovery_inputs(&deployment, &locator, |inputs| {
                host.recover_restarted_a2a_cancel_task(
                    recovered.as_ref(),
                    access,
                    crate::resident::a2a_task_custody::A2aCancelTaskDispositionCommitInput {
                        disposition: inputs.disposition,
                        checkpoint: inputs.checkpoint,
                        checkpoint_issued_at: inputs.checkpoint_issued_at,
                        checkpoint_schema: inputs.checkpoint_schema,
                        checkpoint_authority: inputs.checkpoint_authority,
                        accumulator_limits: inputs.accumulator_limits,
                    },
                    inputs.maximum_record_bytes,
                    inputs.resolve_store,
                    inputs.admit_reconciliation,
                    inputs.admit_rotation,
                    inputs.sign_observation,
                    inputs.sign_checkpoint,
                )
            })?;
        match attempted {
            Ok(_) => Ok(operation_request_host::ResidentOperationRequestOutcome::Pending(locator)),
            Err(failure) => self.retain(
                selector,
                locator,
                retained_cancel_recovery_phase(recovered, failure),
            ),
        }
    }
}

impl<'service, Inputs> ResidentServiceDelegateRecoveryAdapter<'service, Inputs>
where
    Inputs: ResidentServiceDelegateInputPort<'service>,
{
    fn retain_after_turn(
        &mut self,
        retained: ResidentServiceRetainedDelegate<'service>,
        turn: program::ResidentProgramRetainedPhaseTurn,
    ) -> program::ResidentProgramRetainedPhaseTurn {
        self.retained.push(retained);
        turn
    }

    fn retain_a2a_outcome<WriterGuard>(
        &mut self,
        process: &mut ResidentProcessHost<'_, WriterGuard>,
        mut access: super::ResidentDeploymentOperationAccess<'_>,
        mut retained: ResidentServiceRetainedDelegate<'service>,
        recovered: Box<ResidentProcessRecoveredA2aDelegate<'service>>,
    ) -> program::ResidentProgramRetainedPhaseTurn
    where
        WriterGuard: PrincipalWriterGuardPort,
    {
        let deployment = service_deployment_selector(&retained.selector);
        let mut recovered = Some(recovered);
        let mut result = None;
        let lender = self
            .inputs
            .as_mut()
            .expect("the Delegate input owner is returned after every loan")
            .with_pre_task_recovery_inputs(&deployment, &retained.locator, |inputs| {
                let recovered = recovered
                    .take()
                    .expect("the retained A2A recovery is lent once");
                let ResidentProcessRecoveredA2aDelegate { route, recovered } = *recovered;
                let crate::resident::a2a_delegate_task_service::ResidentRecoveredA2aDelegate {
                    outcome,
                    finalization,
                } = recovered;
                let retention = (inputs.retain)(outcome);
                result = Some(process.retain_a2a_delegate_outcome(
                    reborrow_deployment_operation_access(&mut access),
                    inputs.authority,
                    ResidentProcessA2aDelegateRetention::RecoveredOutcomeUnknown {
                        route,
                        finalization,
                        input: retention,
                    },
                ));
            });
        match result {
            Some(Ok(_)) => program::ResidentProgramRetainedPhaseTurn::Advanced,
            Some(Err(failure)) => {
                retained.phase = Some(ResidentServiceRetainedDelegatePhase::RetentionFailure(
                    failure,
                ));
                self.retain_after_turn(
                    retained,
                    program::ResidentProgramRetainedPhaseTurn::Deferred,
                )
            }
            None => {
                let recovered =
                    recovered.expect("the input owner did not consume the retained A2A recovery");
                retained.phase = Some(ResidentServiceRetainedDelegatePhase::Recovered(recovered));
                let turn = if lender.is_err() {
                    program::ResidentProgramRetainedPhaseTurn::Deferred
                } else {
                    program::ResidentProgramRetainedPhaseTurn::Idle
                };
                self.retain_after_turn(retained, turn)
            }
        }
    }

    fn poll_retained_a2a_response<WriterGuard, Authority>(
        &mut self,
        process: &mut ResidentProcessHost<'_, WriterGuard>,
        mut access: super::ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        mut retained: ResidentServiceRetainedDelegate<'service>,
        phase: ResidentServiceRetainedDelegatePhase<'service>,
        key: crate::resident::task_owner::ResidentOperationTaskKey,
    ) -> program::ResidentProgramRetainedPhaseTurn
    where
        WriterGuard: PrincipalWriterGuardPort,
        Authority: AmbientSessionAuthorityPort,
    {
        let input_owner = self
            .inputs
            .take()
            .expect("the Delegate input owner is returned after every loan");
        let deployment = service_deployment_selector(&retained.selector);
        let (input_owner, polled) = input_owner.with_response_poll_inputs(
            &deployment,
            &retained.locator,
            |context, protocol, clock| {
                process.poll_a2a_delegate(
                    &retained.selector,
                    reborrow_deployment_operation_access(&mut access),
                    authority,
                    &key,
                    context,
                    protocol,
                    clock,
                )
            },
        );
        self.inputs = Some(input_owner);
        match polled {
            Ok(Ok(poll)) => {
                retained.phase = Some(ResidentServiceRetainedDelegatePhase::Poll(poll));
                self.retain_after_turn(
                    retained,
                    program::ResidentProgramRetainedPhaseTurn::Advanced,
                )
            }
            Ok(Err(_)) | Err(_) => {
                retained.phase = Some(phase);
                self.retain_after_turn(
                    retained,
                    program::ResidentProgramRetainedPhaseTurn::Deferred,
                )
            }
        }
    }

    fn retain_completed_a2a_response<WriterGuard, Authority>(
        &mut self,
        process: &mut ResidentProcessHost<'_, WriterGuard>,
        mut access: super::ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        mut retained: ResidentServiceRetainedDelegate<'service>,
        completed: ResidentProcessA2aDelegateCompletion,
    ) -> program::ResidentProgramRetainedPhaseTurn
    where
        WriterGuard: PrincipalWriterGuardPort,
        Authority: AmbientSessionAuthorityPort,
    {
        let deployment = service_deployment_selector(&retained.selector);
        let ResidentProcessA2aDelegateCompletion { route, completion } = completed;
        match completion {
            crate::resident::a2a_delegate_task_service::ResidentA2aDelegateCompletion::Task {
                response,
                finalization,
            } => {
                let input_owner = self
                    .inputs
                    .take()
                    .expect("the Delegate input owner is returned after every loan");
                let mut process_custody = Some((route, finalization));
                let (input_owner, result) = input_owner.with_task_outcome_inputs(
                    &deployment,
                    &retained.locator,
                    response,
                    |input| {
                        let (route, finalization) = process_custody
                            .take()
                            .expect("the completed A2A response is lent exactly once");
                        process.retain_a2a_delegate_outcome(
                            reborrow_deployment_operation_access(&mut access),
                            authority,
                            ResidentProcessA2aDelegateRetention::Task {
                                route,
                                finalization,
                                input: Box::new(input),
                            },
                        )
                    },
                );
                self.inputs = Some(input_owner);
                match result {
                    Ok(Ok(_)) => program::ResidentProgramRetainedPhaseTurn::Advanced,
                    Ok(Err(failure)) => {
                        retained.phase = Some(
                            ResidentServiceRetainedDelegatePhase::RetentionFailure(failure),
                        );
                        self.retain_after_turn(
                            retained,
                            program::ResidentProgramRetainedPhaseTurn::Deferred,
                        )
                    }
                    Err((_reason, response)) => {
                        let (route, finalization) = process_custody
                            .take()
                            .expect("a failed lender returns the untouched A2A response");
                        retained.phase = Some(ResidentServiceRetainedDelegatePhase::Launch(
                            ResidentProcessA2aDelegateLaunch::Completed(
                                ResidentProcessA2aDelegateCompletion {
                                    route,
                                    completion: crate::resident::a2a_delegate_task_service::ResidentA2aDelegateCompletion::Task {
                                        response,
                                        finalization,
                                    },
                                },
                            ),
                        ));
                        self.retain_after_turn(
                            retained,
                            program::ResidentProgramRetainedPhaseTurn::Deferred,
                        )
                    }
                }
            }
            crate::resident::a2a_delegate_task_service::ResidentA2aDelegateCompletion::NoTask {
                response,
                finalization,
            } => {
                let input_owner = self
                    .inputs
                    .take()
                    .expect("the Delegate input owner is returned after every loan");
                let mut process_custody = Some((route, finalization));
                let (input_owner, result) = input_owner.with_no_task_outcome_inputs(
                    &deployment,
                    &retained.locator,
                    response,
                    |input| {
                        let (route, finalization) = process_custody
                            .take()
                            .expect("the completed A2A response is lent exactly once");
                        process.retain_a2a_delegate_outcome(
                            reborrow_deployment_operation_access(&mut access),
                            authority,
                            ResidentProcessA2aDelegateRetention::NoTask {
                                route,
                                finalization,
                                input,
                            },
                        )
                    },
                );
                self.inputs = Some(input_owner);
                match result {
                    Ok(Ok(_)) => program::ResidentProgramRetainedPhaseTurn::Advanced,
                    Ok(Err(failure)) => {
                        retained.phase = Some(
                            ResidentServiceRetainedDelegatePhase::RetentionFailure(failure),
                        );
                        self.retain_after_turn(
                            retained,
                            program::ResidentProgramRetainedPhaseTurn::Deferred,
                        )
                    }
                    Err((_reason, response)) => {
                        let (route, finalization) = process_custody
                            .take()
                            .expect("a failed lender returns the untouched A2A response");
                        retained.phase = Some(ResidentServiceRetainedDelegatePhase::Launch(
                            ResidentProcessA2aDelegateLaunch::Completed(
                                ResidentProcessA2aDelegateCompletion {
                                    route,
                                    completion: crate::resident::a2a_delegate_task_service::ResidentA2aDelegateCompletion::NoTask {
                                        response,
                                        finalization,
                                    },
                                },
                            ),
                        ));
                        self.retain_after_turn(
                            retained,
                            program::ResidentProgramRetainedPhaseTurn::Deferred,
                        )
                    }
                }
            }
            crate::resident::a2a_delegate_task_service::ResidentA2aDelegateCompletion::NotStarted {
                response,
                finalization,
            } => {
                let input_owner = self
                    .inputs
                    .take()
                    .expect("the Delegate input owner is returned after every loan");
                let mut process_custody = Some((route, finalization));
                let (input_owner, result) = input_owner.with_not_started_outcome_inputs(
                    &deployment,
                    &retained.locator,
                    response,
                    |input| {
                        let (route, finalization) = process_custody
                            .take()
                            .expect("the completed A2A response is lent exactly once");
                        process.retain_a2a_delegate_outcome(
                            reborrow_deployment_operation_access(&mut access),
                            authority,
                            ResidentProcessA2aDelegateRetention::NotStarted {
                                route,
                                finalization,
                                input,
                            },
                        )
                    },
                );
                self.inputs = Some(input_owner);
                match result {
                    Ok(Ok(_)) => program::ResidentProgramRetainedPhaseTurn::Advanced,
                    Ok(Err(failure)) => {
                        retained.phase = Some(
                            ResidentServiceRetainedDelegatePhase::RetentionFailure(failure),
                        );
                        self.retain_after_turn(
                            retained,
                            program::ResidentProgramRetainedPhaseTurn::Deferred,
                        )
                    }
                    Err((_reason, response)) => {
                        let (route, finalization) = process_custody
                            .take()
                            .expect("a failed lender returns the untouched A2A response");
                        retained.phase = Some(ResidentServiceRetainedDelegatePhase::Launch(
                            ResidentProcessA2aDelegateLaunch::Completed(
                                ResidentProcessA2aDelegateCompletion {
                                    route,
                                    completion: crate::resident::a2a_delegate_task_service::ResidentA2aDelegateCompletion::NotStarted {
                                        response,
                                        finalization,
                                    },
                                },
                            ),
                        ));
                        self.retain_after_turn(
                            retained,
                            program::ResidentProgramRetainedPhaseTurn::Deferred,
                        )
                    }
                }
            }
            crate::resident::a2a_delegate_task_service::ResidentA2aDelegateCompletion::OutcomeUnknown {
                outcome,
                finalization,
            } => {
                let input_owner = self
                    .inputs
                    .take()
                    .expect("the Delegate input owner is returned after every loan");
                let mut process_custody = Some((route, finalization));
                let (input_owner, result) = input_owner.with_unknown_outcome_inputs(
                    &deployment,
                    &retained.locator,
                    outcome,
                    |input| {
                        let (route, finalization) = process_custody
                            .take()
                            .expect("the completed A2A response is lent exactly once");
                        process.retain_a2a_delegate_outcome(
                            reborrow_deployment_operation_access(&mut access),
                            authority,
                            ResidentProcessA2aDelegateRetention::OutcomeUnknown {
                                route,
                                finalization,
                                input,
                            },
                        )
                    },
                );
                self.inputs = Some(input_owner);
                match result {
                    Ok(Ok(_)) => program::ResidentProgramRetainedPhaseTurn::Advanced,
                    Ok(Err(failure)) => {
                        retained.phase = Some(
                            ResidentServiceRetainedDelegatePhase::RetentionFailure(failure),
                        );
                        self.retain_after_turn(
                            retained,
                            program::ResidentProgramRetainedPhaseTurn::Deferred,
                        )
                    }
                    Err((_reason, outcome)) => {
                        let (route, finalization) = process_custody
                            .take()
                            .expect("a failed lender returns the untouched A2A outcome");
                        retained.phase = Some(ResidentServiceRetainedDelegatePhase::Launch(
                            ResidentProcessA2aDelegateLaunch::Completed(
                                ResidentProcessA2aDelegateCompletion {
                                    route,
                                    completion: crate::resident::a2a_delegate_task_service::ResidentA2aDelegateCompletion::OutcomeUnknown {
                                        outcome,
                                        finalization,
                                    },
                                },
                            ),
                        ));
                        self.retain_after_turn(
                            retained,
                            program::ResidentProgramRetainedPhaseTurn::Deferred,
                        )
                    }
                }
            }
        }
    }

    fn continue_retained_launch_failure<WriterGuard, Authority>(
        &mut self,
        process: &mut ResidentProcessHost<'_, WriterGuard>,
        mut access: super::ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        mut retained: ResidentServiceRetainedDelegate<'service>,
        failure: Box<ResidentProcessPairwiseA2aDelegateLaunchFailure<'service, 'service>>,
    ) -> program::ResidentProgramRetainedPhaseTurn
    where
        WriterGuard: PrincipalWriterGuardPort,
        Authority: AmbientSessionAuthorityPort,
    {
        let ResidentProcessPairwiseA2aDelegateLaunchFailure::A2a(lower) = *failure else {
            retained.phase = Some(ResidentServiceRetainedDelegatePhase::LaunchFailure(failure));
            return self.retain_after_turn(
                retained,
                program::ResidentProgramRetainedPhaseTurn::Deferred,
            );
        };
        let lower = *lower;
        match lower {
            lower @ ResidentProcessA2aDelegateLaunchFailure::Rollback { .. } => {
                match process.retry_a2a_delegate_launch_rollback(
                    reborrow_deployment_operation_access(&mut access),
                    authority,
                    lower,
                ) {
                    Ok(()) => program::ResidentProgramRetainedPhaseTurn::Advanced,
                    Err(failure) => {
                        retained.phase = Some(ResidentServiceRetainedDelegatePhase::LaunchFailure(
                            Box::new(ResidentProcessPairwiseA2aDelegateLaunchFailure::A2a(
                                failure,
                            )),
                        ));
                        self.retain_after_turn(
                            retained,
                            program::ResidentProgramRetainedPhaseTurn::Deferred,
                        )
                    }
                }
            }
            lower @ ResidentProcessA2aDelegateLaunchFailure::Custody { .. } => {
                let next = process.continue_a2a_delegate_custody(
                    reborrow_deployment_operation_access(&mut access),
                    authority,
                    lower,
                );
                let (failure, turn) = match next {
                    Ok(failure) => (
                        Box::new(failure),
                        program::ResidentProgramRetainedPhaseTurn::Advanced,
                    ),
                    Err(failure) => (failure, program::ResidentProgramRetainedPhaseTurn::Deferred),
                };
                retained.phase = Some(ResidentServiceRetainedDelegatePhase::LaunchFailure(
                    Box::new(ResidentProcessPairwiseA2aDelegateLaunchFailure::A2a(
                        failure,
                    )),
                ));
                self.retain_after_turn(retained, turn)
            }
            lower @ ResidentProcessA2aDelegateLaunchFailure::StartedAssociation { .. } => {
                match process.settle_a2a_delegate_started_association(
                    reborrow_deployment_operation_access(&mut access),
                    authority,
                    lower,
                ) {
                    Ok(launch) => {
                        retained.phase = Some(ResidentServiceRetainedDelegatePhase::Launch(launch));
                        self.retain_after_turn(
                            retained,
                            program::ResidentProgramRetainedPhaseTurn::Advanced,
                        )
                    }
                    Err(failure) => {
                        retained.phase = Some(ResidentServiceRetainedDelegatePhase::LaunchFailure(
                            Box::new(ResidentProcessPairwiseA2aDelegateLaunchFailure::A2a(
                                failure,
                            )),
                        ));
                        self.retain_after_turn(
                            retained,
                            program::ResidentProgramRetainedPhaseTurn::Deferred,
                        )
                    }
                }
            }
            lower @ ResidentProcessA2aDelegateLaunchFailure::OfferSettlement { .. } => {
                match process.retry_a2a_delegate_offered_completion(
                    reborrow_deployment_operation_access(&mut access),
                    authority,
                    lower,
                ) {
                    Ok(poll) => {
                        retained.phase = Some(ResidentServiceRetainedDelegatePhase::Poll(poll));
                        self.retain_after_turn(
                            retained,
                            program::ResidentProgramRetainedPhaseTurn::Advanced,
                        )
                    }
                    Err(failure) => {
                        retained.phase = Some(ResidentServiceRetainedDelegatePhase::LaunchFailure(
                            Box::new(ResidentProcessPairwiseA2aDelegateLaunchFailure::A2a(
                                failure,
                            )),
                        ));
                        self.retain_after_turn(
                            retained,
                            program::ResidentProgramRetainedPhaseTurn::Deferred,
                        )
                    }
                }
            }
            lower => {
                retained.phase = Some(ResidentServiceRetainedDelegatePhase::LaunchFailure(
                    Box::new(ResidentProcessPairwiseA2aDelegateLaunchFailure::A2a(
                        Box::new(lower),
                    )),
                ));
                self.retain_after_turn(
                    retained,
                    program::ResidentProgramRetainedPhaseTurn::Deferred,
                )
            }
        }
    }

    fn continue_retained_poll<WriterGuard, Authority>(
        &mut self,
        process: &mut ResidentProcessHost<'_, WriterGuard>,
        mut access: super::ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        mut retained: ResidentServiceRetainedDelegate<'service>,
        poll: super::ResidentProcessA2aDelegatePoll,
    ) -> program::ResidentProgramRetainedPhaseTurn
    where
        WriterGuard: PrincipalWriterGuardPort,
        Authority: AmbientSessionAuthorityPort,
    {
        match poll {
            poll @ super::ResidentProcessA2aDelegatePoll::SettlementRejected { .. } => {
                match process.retry_a2a_delegate_settlement(
                    reborrow_deployment_operation_access(&mut access),
                    authority,
                    poll,
                ) {
                    Ok(next) => {
                        retained.phase = Some(ResidentServiceRetainedDelegatePhase::Poll(next));
                        self.retain_after_turn(
                            retained,
                            program::ResidentProgramRetainedPhaseTurn::Advanced,
                        )
                    }
                    Err(poll) => {
                        retained.phase = Some(ResidentServiceRetainedDelegatePhase::Poll(*poll));
                        self.retain_after_turn(
                            retained,
                            program::ResidentProgramRetainedPhaseTurn::Deferred,
                        )
                    }
                }
            }
            super::ResidentProcessA2aDelegatePoll::Diagnostic(diagnostic) => {
                match process.recover_a2a_delegate_diagnostic(
                    reborrow_deployment_operation_access(&mut access),
                    authority,
                    diagnostic,
                ) {
                    Ok(completed) => {
                        retained.phase = Some(ResidentServiceRetainedDelegatePhase::Launch(
                            ResidentProcessA2aDelegateLaunch::Completed(completed),
                        ));
                        self.retain_after_turn(
                            retained,
                            program::ResidentProgramRetainedPhaseTurn::Advanced,
                        )
                    }
                    Err(diagnostic) => {
                        retained.phase = Some(ResidentServiceRetainedDelegatePhase::Poll(
                            super::ResidentProcessA2aDelegatePoll::Diagnostic(*diagnostic),
                        ));
                        self.retain_after_turn(
                            retained,
                            program::ResidentProgramRetainedPhaseTurn::Deferred,
                        )
                    }
                }
            }
            poll @ super::ResidentProcessA2aDelegatePoll::Unrouted { .. } => {
                match process.relinquish_unrouted_a2a_delegate_to_recovery(access.store, poll) {
                    Ok(()) => program::ResidentProgramRetainedPhaseTurn::Advanced,
                    Err(poll) => {
                        retained.phase = Some(ResidentServiceRetainedDelegatePhase::Poll(*poll));
                        self.retain_after_turn(
                            retained,
                            program::ResidentProgramRetainedPhaseTurn::Deferred,
                        )
                    }
                }
            }
            super::ResidentProcessA2aDelegatePoll::Completed(completed) => {
                retained.phase = Some(ResidentServiceRetainedDelegatePhase::Launch(
                    ResidentProcessA2aDelegateLaunch::Completed(completed),
                ));
                self.retain_after_turn(
                    retained,
                    program::ResidentProgramRetainedPhaseTurn::Deferred,
                )
            }
            super::ResidentProcessA2aDelegatePoll::Pending(key) => {
                retained.phase = Some(ResidentServiceRetainedDelegatePhase::Poll(
                    super::ResidentProcessA2aDelegatePoll::Pending(key),
                ));
                self.retain_after_turn(
                    retained,
                    program::ResidentProgramRetainedPhaseTurn::Deferred,
                )
            }
        }
    }
}

impl<'service, Inputs> ResidentServiceRetainedDelegatePhasePort
    for ResidentServiceDelegateRecoveryAdapter<'service, Inputs>
where
    Inputs: ResidentServiceDelegateInputPort<'service>,
{
    fn has_runnable_retained_delegate(&self) -> bool {
        self.next_runnable_retained_delegate().is_some()
    }

    fn retained_delegate_selector(&self) -> Option<&super::ResidentDeploymentAuthoritySelector> {
        self.next_runnable_retained_delegate()
            .map(|retained| &retained.selector)
    }

    fn continue_one_retained_delegate<WriterGuard, Authority>(
        &mut self,
        process: &mut ResidentProcessHost<'_, WriterGuard>,
        mut access: super::ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
    ) -> program::ResidentProgramRetainedPhaseTurn
    where
        WriterGuard: PrincipalWriterGuardPort,
        Authority: AmbientSessionAuthorityPort,
    {
        let Some(index) = self.retained.iter().position(|retained| {
            retained
                .phase
                .as_ref()
                .is_some_and(ResidentServiceRetainedDelegatePhase::is_runnable)
        }) else {
            return program::ResidentProgramRetainedPhaseTurn::Idle;
        };
        let mut retained = self.retained.swap_remove(index);
        let phase = retained
            .phase
            .take()
            .expect("the selected retained Delegate phase is present");
        match phase {
            ResidentServiceRetainedDelegatePhase::PreDispatchExpiry(expiry) => {
                let RetainedDelegatePreDispatchExpiryPhase { observed_at, state } = *expiry;
                let deployment = service_deployment_selector(&retained.selector);
                let mut state = Some(state);
                let attempted = self
                    .inputs
                    .as_mut()
                    .expect("the Delegate input owner is returned after every loan")
                    .with_pre_dispatch_expiry_continuation_inputs(
                        &deployment,
                        &retained.locator,
                        |inputs| {
                            let index = match process
                                .deployment_authority_index(&retained.selector)
                            {
                                Ok(index) => index,
                                Err(_) => return Err(()),
                            };
                            if process.deployments[index].store() != access.store {
                                return Err(());
                            }
                            match crate::resident::operation_dispatch::continue_owned_operation_observation_custody(
                                state
                                    .take()
                                    .expect("the Delegate expiry lender runs once"),
                                observed_at,
                                inputs.checkpoint_schema,
                                inputs.checkpoint_authority,
                                inputs.resolve_store,
                                inputs.accumulator_limits,
                                access.custody,
                                access.anchor,
                                &mut *inputs.sign_checkpoint,
                            ) {
                                Ok(_) => Ok(()),
                                Err(pending) => {
                                    state = Some(pending);
                                    Err(())
                                }
                            }
                        },
                    );
                match attempted {
                    Ok(Ok(())) => program::ResidentProgramRetainedPhaseTurn::Advanced,
                    Err(_) | Ok(Err(())) => {
                        retained.phase =
                            Some(ResidentServiceRetainedDelegatePhase::PreDispatchExpiry(
                                Box::new(RetainedDelegatePreDispatchExpiryPhase {
                                    observed_at,
                                    state: state.expect(
                                        "an incomplete Delegate expiry retains exact custody",
                                    ),
                                }),
                            ));
                        self.retain_after_turn(
                            retained,
                            program::ResidentProgramRetainedPhaseTurn::Deferred,
                        )
                    }
                }
            }
            ResidentServiceRetainedDelegatePhase::Launch(
                ResidentProcessA2aDelegateLaunch::Awaiting(key),
            ) => self.poll_retained_a2a_response(
                process,
                access,
                authority,
                retained,
                ResidentServiceRetainedDelegatePhase::Launch(
                    ResidentProcessA2aDelegateLaunch::Awaiting(key.clone()),
                ),
                key,
            ),
            ResidentServiceRetainedDelegatePhase::Launch(
                ResidentProcessA2aDelegateLaunch::Completed(completed),
            ) => {
                self.retain_completed_a2a_response(process, access, authority, retained, completed)
            }
            ResidentServiceRetainedDelegatePhase::Launch(
                ResidentProcessA2aDelegateLaunch::Diagnostic(diagnostic),
            ) => match process.recover_a2a_delegate_diagnostic(
                reborrow_deployment_operation_access(&mut access),
                authority,
                diagnostic,
            ) {
                Ok(completed) => {
                    retained.phase = Some(ResidentServiceRetainedDelegatePhase::Launch(
                        ResidentProcessA2aDelegateLaunch::Completed(completed),
                    ));
                    self.retain_after_turn(
                        retained,
                        program::ResidentProgramRetainedPhaseTurn::Advanced,
                    )
                }
                Err(diagnostic) => {
                    retained.phase = Some(ResidentServiceRetainedDelegatePhase::Launch(
                        ResidentProcessA2aDelegateLaunch::Diagnostic(*diagnostic),
                    ));
                    self.retain_after_turn(
                        retained,
                        program::ResidentProgramRetainedPhaseTurn::Deferred,
                    )
                }
            },
            ResidentServiceRetainedDelegatePhase::Poll(
                super::ResidentProcessA2aDelegatePoll::Pending(key),
            ) => self.poll_retained_a2a_response(
                process,
                access,
                authority,
                retained,
                ResidentServiceRetainedDelegatePhase::Poll(
                    super::ResidentProcessA2aDelegatePoll::Pending(key.clone()),
                ),
                key,
            ),
            ResidentServiceRetainedDelegatePhase::Poll(poll) => {
                self.continue_retained_poll(process, access, authority, retained, poll)
            }
            ResidentServiceRetainedDelegatePhase::LaunchFailure(failure) => {
                self.continue_retained_launch_failure(process, access, authority, retained, failure)
            }
            ResidentServiceRetainedDelegatePhase::RestartFailure(failure) => {
                match process.retry_restarted_a2a_delegate(
                    reborrow_deployment_operation_access(&mut access),
                    authority,
                    *failure,
                ) {
                    Ok(ResidentProcessA2aDelegateRestartRetry::Rejected(_)) => {
                        program::ResidentProgramRetainedPhaseTurn::Advanced
                    }
                    Ok(ResidentProcessA2aDelegateRestartRetry::Recovered(recovered)) => {
                        retained.phase =
                            Some(ResidentServiceRetainedDelegatePhase::Recovered(recovered));
                        self.retain_after_turn(
                            retained,
                            program::ResidentProgramRetainedPhaseTurn::Advanced,
                        )
                    }
                    Err(failure) => {
                        retained.phase = Some(
                            ResidentServiceRetainedDelegatePhase::RestartFailure(failure),
                        );
                        self.retain_after_turn(
                            retained,
                            program::ResidentProgramRetainedPhaseTurn::Deferred,
                        )
                    }
                }
            }
            ResidentServiceRetainedDelegatePhase::Recovered(recovered) => {
                self.retain_a2a_outcome(process, access, retained, recovered)
            }
            ResidentServiceRetainedDelegatePhase::RetentionFailure(failure) => {
                let result = match *failure {
                    pending @ ResidentProcessA2aDelegateRetentionFailure::Selection { .. } => {
                        let ResidentProcessA2aDelegateRetentionFailure::Selection {
                            pending, ..
                        } = pending
                        else {
                            unreachable!()
                        };
                        process.retain_a2a_delegate_outcome(
                            reborrow_deployment_operation_access(&mut access),
                            authority,
                            *pending,
                        )
                    }
                    pending @ ResidentProcessA2aDelegateRetentionFailure::Retention { .. } => {
                        process.continue_a2a_delegate_outcome_custody(
                            reborrow_deployment_operation_access(&mut access),
                            authority,
                            pending,
                        )
                    }
                    pending @ ResidentProcessA2aDelegateRetentionFailure::Finalization {
                        ..
                    } => process.retry_a2a_delegate_finalization(
                        reborrow_deployment_operation_access(&mut access),
                        authority,
                        pending,
                    ),
                    pending @ ResidentProcessA2aDelegateRetentionFailure::Route { .. } => {
                        process.retry_a2a_delegate_route_release(pending)
                    }
                };
                match result {
                    Ok(_) => program::ResidentProgramRetainedPhaseTurn::Advanced,
                    Err(failure) => {
                        retained.phase = Some(
                            ResidentServiceRetainedDelegatePhase::RetentionFailure(failure),
                        );
                        self.retain_after_turn(
                            retained,
                            program::ResidentProgramRetainedPhaseTurn::Deferred,
                        )
                    }
                }
            }
            ResidentServiceRetainedDelegatePhase::Task(recovered) => {
                let deployment = service_deployment_selector(&retained.selector);
                let attempted = self
                    .inputs
                    .as_mut()
                    .expect("the Delegate input owner is returned after every loan")
                    .with_task_observation_inputs(&deployment, &retained.locator, |inputs| {
                        process.recover_a2a_get_task_disposition(
                            &recovered,
                            &recovered.attempt.start.selector,
                            access,
                            inputs.authority,
                            inputs.maximum_record_bytes,
                        )
                    });
                match attempted {
                    Ok(Ok(_)) => program::ResidentProgramRetainedPhaseTurn::Advanced,
                    _ => {
                        retained.phase =
                            Some(ResidentServiceRetainedDelegatePhase::Task(recovered));
                        self.retain_after_turn(
                            retained,
                            program::ResidentProgramRetainedPhaseTurn::Deferred,
                        )
                    }
                }
            }
            ResidentServiceRetainedDelegatePhase::CancelPrephysical(recovered) => {
                let deployment = service_deployment_selector(&retained.selector);
                let mut recovered = Some(recovered);
                let attempted = self
                    .inputs
                    .as_mut()
                    .expect("the Delegate input owner is returned after every loan")
                    .with_cancel_recovery_inputs(&deployment, &retained.locator, |inputs| {
                        process.recover_restarted_a2a_cancel_task(
                            recovered
                                .as_ref()
                                .expect("the recovered CancelTask request remains retained"),
                            access,
                            crate::resident::a2a_task_custody::A2aCancelTaskDispositionCommitInput {
                                disposition: inputs.disposition,
                                checkpoint: inputs.checkpoint,
                                checkpoint_issued_at: inputs.checkpoint_issued_at,
                                checkpoint_schema: inputs.checkpoint_schema,
                                checkpoint_authority: inputs.checkpoint_authority,
                                accumulator_limits: inputs.accumulator_limits,
                            },
                            inputs.maximum_record_bytes,
                            inputs.resolve_store,
                            inputs.admit_reconciliation,
                            inputs.admit_rotation,
                            inputs.sign_observation,
                            inputs.sign_checkpoint,
                        )
                    });
                match attempted {
                    Ok(Ok(_)) => program::ResidentProgramRetainedPhaseTurn::Advanced,
                    Ok(Err(failure)) => {
                        retained.phase = Some(retained_cancel_recovery_phase(
                            recovered
                                .take()
                                .expect("the recovered CancelTask request remains retained"),
                            failure,
                        ));
                        self.retain_after_turn(
                            retained,
                            program::ResidentProgramRetainedPhaseTurn::Deferred,
                        )
                    }
                    Err(_) => {
                        retained.phase =
                            Some(ResidentServiceRetainedDelegatePhase::CancelPrephysical(
                                recovered
                                    .take()
                                    .expect("the recovered CancelTask request remains retained"),
                            ));
                        self.retain_after_turn(
                            retained,
                            program::ResidentProgramRetainedPhaseTurn::Deferred,
                        )
                    }
                }
            }
            ResidentServiceRetainedDelegatePhase::CancelCustody(pending) => {
                match process.continue_restarted_a2a_cancel_task_custody(*pending, access) {
                    Ok(_) => program::ResidentProgramRetainedPhaseTurn::Advanced,
                    Err(failure) => {
                        retained.phase = Some(ResidentServiceRetainedDelegatePhase::CancelCustody(
                            failure.pending,
                        ));
                        self.retain_after_turn(
                            retained,
                            program::ResidentProgramRetainedPhaseTurn::Deferred,
                        )
                    }
                }
            }
            phase => {
                retained.phase = Some(phase);
                self.retain_after_turn(
                    retained,
                    program::ResidentProgramRetainedPhaseTurn::Deferred,
                )
            }
        }
    }
}

enum ResidentServicePreTaskFailure<'service> {
    Recovery(Box<ResidentProcessA2aDelegateRestartFailure<'service>>),
    Retention(Box<ResidentProcessA2aDelegateRetentionFailure<'service>>),
}

impl<'service> From<Box<ResidentProcessA2aDelegateRestartFailure<'service>>>
    for ResidentServicePreTaskFailure<'service>
{
    fn from(failure: Box<ResidentProcessA2aDelegateRestartFailure<'service>>) -> Self {
        Self::Recovery(failure)
    }
}

impl<WriterGuard> ResidentA2aDelegateRecoveryPort<WriterGuard>
    for UnavailableResidentServiceDelegateInputs
{
    type Output = operation_request_host::ResidentOperationRequestOutcome;
    type Error = ResidentOperationFrameFailure;

    fn recover_delegate_resolution<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, WriterGuard>,
        _: Box<restart::ResidentProcessRecoveredOperationResolution>,
        _: super::ResidentDeploymentOperationAccess<'_>,
        _: program::ResidentProgramDelegateResolutionInput<'_, '_>,
    ) -> Result<Self::Output, Self::Error> {
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    fn recover_pre_task<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, WriterGuard>,
        _: Box<restart::ResidentProcessRecoveredAttempt>,
        _: super::ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    fn recover_task<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, WriterGuard>,
        _: Box<restart::ResidentProcessRecoveredA2aTask>,
        _: super::ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    fn recover_cancel_task<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, WriterGuard>,
        _: Box<restart::ResidentProcessRecoveredA2aCancelTaskRequest>,
        _: super::ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    }
}

impl ResidentServiceRetainedDelegatePhasePort for UnavailableResidentServiceDelegateInputs {
    fn has_runnable_retained_delegate(&self) -> bool {
        false
    }

    fn retained_delegate_selector(&self) -> Option<&super::ResidentDeploymentAuthoritySelector> {
        None
    }

    fn continue_one_retained_delegate<WriterGuard, Authority>(
        &mut self,
        _: &mut ResidentProcessHost<'_, WriterGuard>,
        _: super::ResidentDeploymentOperationAccess<'_>,
        _: &mut Authority,
    ) -> program::ResidentProgramRetainedPhaseTurn
    where
        WriterGuard: PrincipalWriterGuardPort,
        Authority: AmbientSessionAuthorityPort,
    {
        program::ResidentProgramRetainedPhaseTurn::Idle
    }
}

pub(in crate::resident::process_host) struct ResidentServiceDelegateProviderUnavailable<
    Output,
    Error,
>(std::marker::PhantomData<(Output, Error)>);

impl<WriterGuard, Output, Error>
    operation_recovery_sweep::ResidentProcessRootProviderRecoveryOwner<WriterGuard>
    for ResidentServiceDelegateProviderUnavailable<Output, Error>
{
    type Output = Output;
    type Error = Error;

    fn complete_local_cancellation<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, WriterGuard>,
        _: Box<restart::ResidentProcessRecoveredLocalCancellation>,
        _: super::ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        unreachable!("an A2A Delegate phase cannot route through the generic provider owner")
    }

    fn recover_operation_resolution<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, WriterGuard>,
        _: Box<restart::ResidentProcessRecoveredOperationResolution>,
        _: super::ResidentDeploymentOperationAccess<'_>,
    ) -> Result<
        operation_recovery_sweep::ResidentProcessRootProviderResolution<Self::Output>,
        Self::Error,
    > {
        unreachable!("an A2A Delegate phase cannot route through the generic provider owner")
    }

    fn recover_provider_resolved<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, WriterGuard>,
        _: Box<restart::ResidentProcessRecoveredProviderResolution>,
        _: super::ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        unreachable!("an A2A Delegate phase cannot route through the generic provider owner")
    }

    fn recover_provider_prepared<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, WriterGuard>,
        _: Box<restart::ResidentProcessRecoveredPreparedAttempt>,
        _: super::ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        unreachable!("an A2A Delegate phase cannot route through the generic provider owner")
    }

    fn recover_provider_dispatched<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, WriterGuard>,
        _: Box<restart::ResidentProcessRecoveredAttempt>,
        _: super::ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        unreachable!("an A2A Delegate phase cannot route through the generic provider owner")
    }

    fn recover_native_prepared<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, WriterGuard>,
        _: Box<restart::ResidentProcessRecoveredPreparedAttempt>,
        _: super::ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        unreachable!("an A2A Delegate phase cannot route through the generic provider owner")
    }

    fn recover_native_dispatched<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, WriterGuard>,
        _: Box<restart::ResidentProcessRecoveredAttempt>,
        _: super::ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        unreachable!("an A2A Delegate phase cannot route through the generic provider owner")
    }
}

pub(in crate::resident::process_host) fn resident_service_delegate_recovery_owner<
    A2a,
    WriterGuard,
>(
    a2a: &mut A2a,
) -> ResidentA2aDelegateRecoveryOwner<
    ResidentServiceDelegateProviderUnavailable<A2a::Output, A2a::Error>,
    &mut A2a,
>
where
    A2a: ResidentA2aDelegateRecoveryPort<WriterGuard>,
{
    ResidentA2aDelegateRecoveryOwner::new(
        ResidentServiceDelegateProviderUnavailable(std::marker::PhantomData),
        a2a,
    )
}

impl<WriterGuard, Port> ResidentA2aDelegateRecoveryPort<WriterGuard> for &mut Port
where
    Port: ResidentA2aDelegateRecoveryPort<WriterGuard>,
{
    type Output = Port::Output;
    type Error = Port::Error;

    fn recover_delegate_resolution<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<restart::ResidentProcessRecoveredOperationResolution>,
        access: super::ResidentDeploymentOperationAccess<'_>,
        input: program::ResidentProgramDelegateResolutionInput<'_, '_>,
    ) -> Result<Self::Output, Self::Error> {
        (**self).recover_delegate_resolution(host, recovered, access, input)
    }

    fn recover_pre_task<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<restart::ResidentProcessRecoveredAttempt>,
        access: super::ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        (**self).recover_pre_task(host, recovered, access)
    }

    fn recover_task<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<restart::ResidentProcessRecoveredA2aTask>,
        access: super::ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        (**self).recover_task(host, recovered, access)
    }

    fn recover_cancel_task<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<restart::ResidentProcessRecoveredA2aCancelTaskRequest>,
        access: super::ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        (**self).recover_cancel_task(host, recovered, access)
    }
}

impl<'service, WriterGuard, InvokeInputs, DelegateInputs, CancellationInputs, ObserveInputs>
    delegate_recovery_sweep::ResidentProcessDelegateRecoveryOwner<WriterGuard>
    for InvokeRecoveryAdapter<
        'service,
        InvokeInputs,
        DelegateInputs,
        CancellationInputs,
        ObserveInputs,
    >
where
    DelegateInputs: ResidentA2aDelegateRecoveryPort<
        WriterGuard,
        Output = operation_request_host::ResidentOperationRequestOutcome,
        Error = ResidentOperationFrameFailure,
    >,
{
    fn recover_delegate_operation_resolution<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<restart::ResidentProcessRecoveredOperationResolution>,
        access: super::ResidentDeploymentOperationAccess<'_>,
        input: program::ResidentProgramDelegateResolutionInput<'_, '_>,
    ) -> Result<Self::Output, Self::Error> {
        let mut owner =
            resident_service_delegate_recovery_owner::<_, WriterGuard>(&mut self.delegate_inputs);
        delegate_recovery_sweep::ResidentProcessDelegateRecoveryOwner::recover_delegate_operation_resolution(
            &mut owner,
            host,
            recovered,
            access,
            input,
        )
    }

    fn recover_a2a_pre_task<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<restart::ResidentProcessRecoveredAttempt>,
        access: super::ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        let mut owner =
            resident_service_delegate_recovery_owner::<_, WriterGuard>(&mut self.delegate_inputs);
        delegate_recovery_sweep::ResidentProcessDelegateRecoveryOwner::recover_a2a_pre_task(
            &mut owner, host, recovered, access,
        )
    }

    fn recover_a2a_task<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<restart::ResidentProcessRecoveredA2aTask>,
        access: super::ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        let mut owner =
            resident_service_delegate_recovery_owner::<_, WriterGuard>(&mut self.delegate_inputs);
        delegate_recovery_sweep::ResidentProcessDelegateRecoveryOwner::recover_a2a_task(
            &mut owner, host, recovered, access,
        )
    }

    fn recover_a2a_cancel_task<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<restart::ResidentProcessRecoveredA2aCancelTaskRequest>,
        access: super::ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        let mut owner =
            resident_service_delegate_recovery_owner::<_, WriterGuard>(&mut self.delegate_inputs);
        delegate_recovery_sweep::ResidentProcessDelegateRecoveryOwner::recover_a2a_cancel_task(
            &mut owner, host, recovered, access,
        )
    }
}

impl<'service, InvokeInputs, DelegateInputs, CancellationInputs, ObserveInputs>
    InvokeRecoveryAdapter<'service, InvokeInputs, DelegateInputs, CancellationInputs, ObserveInputs>
where
    CancellationInputs: ResidentServiceCancellationInputPort<'service>,
{
    fn retain_cancellation_phase(
        &mut self,
        reservation: RetainedPhaseReservation<RetainedServicePhaseKey>,
        phase: RetainedCancellationPhase<'service>,
        turn: program::ResidentProgramRetainedPhaseTurn,
    ) -> program::ResidentProgramRetainedPhaseTurn {
        self.retained
            .retain_reserved(reservation, RetainedServicePhase::Cancellation(phase))
            .unwrap_or_else(|_| unreachable!("the cancellation reservation remains linear"));
        turn
    }

    fn retain_a2a_cancellation_completion<WriterGuard, Authority>(
        &mut self,
        process: &mut ResidentProcessHost<'_, WriterGuard>,
        access: super::ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        reservation: RetainedPhaseReservation<RetainedServicePhaseKey>,
        exact: &RetainedCancellationPhaseKey,
        completed: ResidentProcessA2aDelegateCompletion,
    ) -> program::ResidentProgramRetainedPhaseTurn
    where
        WriterGuard: PrincipalWriterGuardPort,
        Authority: AmbientSessionAuthorityPort,
    {
        let ResidentProcessA2aDelegateCompletion { route, completion } = completed;
        let crate::resident::a2a_delegate_task_service::ResidentA2aDelegateCompletion::OutcomeUnknown {
            outcome,
            finalization,
        } = completion
        else {
            unreachable!("stopping an A2A response read creates only explicit uncertainty")
        };
        let Some(input_owner) = self.cancel_inputs.take() else {
            return self.retain_cancellation_phase(
                reservation,
                RetainedCancellationPhase::a2a(
                    super::ResidentProcessA2aDelegatePoll::Completed(
                        ResidentProcessA2aDelegateCompletion {
                            route,
                            completion: crate::resident::a2a_delegate_task_service::ResidentA2aDelegateCompletion::OutcomeUnknown {
                                outcome,
                                finalization,
                            },
                        },
                    ),
                ),
                program::ResidentProgramRetainedPhaseTurn::Deferred,
            );
        };
        let deployment = ResidentServiceDeploymentSelector {
            principal_id: exact.selector.principal_id.clone(),
            deployment_binding_ref: exact.selector.deployment_binding_ref.clone(),
            deployment_binding_sha256: exact.selector.deployment_binding_sha256.clone(),
            runtime_authority_ref: exact.selector.runtime_authority_ref.clone(),
            runtime_authority_sha256: exact.selector.runtime_authority_sha256.clone(),
        };
        let mut process_custody = Some((route, finalization));
        let (input_owner, retained) = input_owner.with_a2a_cancellation_outcome_inputs(
            &deployment,
            &exact.request_id,
            &exact.request_sha256,
            &exact.locator,
            outcome,
            |input| {
                let (route, finalization) = process_custody
                    .take()
                    .expect("the A2A completion is lent exactly once");
                process.retain_a2a_delegate_outcome(
                    access,
                    authority,
                    ResidentProcessA2aDelegateRetention::OutcomeUnknown {
                        route,
                        finalization,
                        input,
                    },
                )
            },
        );
        self.cancel_inputs = Some(input_owner);
        match retained {
            Ok(Ok(_)) => {
                assert!(self.retained.release(reservation));
                program::ResidentProgramRetainedPhaseTurn::Advanced
            }
            Ok(Err(failure)) => self.retain_cancellation_phase(
                reservation,
                RetainedCancellationPhase::A2aOutcome(failure),
                program::ResidentProgramRetainedPhaseTurn::Deferred,
            ),
            Err((_reason, outcome)) => {
                let (route, finalization) = process_custody
                    .take()
                    .expect("a failed lender returns the untouched A2A completion");
                self.retain_cancellation_phase(
                    reservation,
                    RetainedCancellationPhase::a2a(
                        super::ResidentProcessA2aDelegatePoll::Completed(
                            ResidentProcessA2aDelegateCompletion {
                                route,
                                completion: crate::resident::a2a_delegate_task_service::ResidentA2aDelegateCompletion::OutcomeUnknown {
                                    outcome,
                                    finalization,
                                },
                            },
                        ),
                    ),
                    program::ResidentProgramRetainedPhaseTurn::Deferred,
                )
            }
        }
    }
}

impl<'service, InvokeInputs, DelegateInputs, CancellationInputs, ObserveInputs>
    program::ResidentProgramCancellationInputPort
    for InvokeRecoveryAdapter<
        'service,
        InvokeInputs,
        DelegateInputs,
        CancellationInputs,
        ObserveInputs,
    >
where
    CancellationInputs: ResidentServiceCancellationInputPort<'service>,
{
    type Reservation = RetainedPhaseReservation<RetainedServicePhaseKey>;

    fn reserve_cancellation(
        &mut self,
        selector: &super::ResidentDeploymentAuthoritySelector,
        request: &CommonRequestEnvelopeV1,
        locator: &ControlRecordCoordinate,
    ) -> Result<Self::Reservation, ResidentOperationFrameFailure> {
        self.retained
            .reserve(RetainedServicePhaseKey::Cancellation(
                RetainedCancellationPhaseKey {
                    selector: selector.clone(),
                    request_id: request.request_id().clone(),
                    request_sha256: request.canonical_request_sha256().clone(),
                    locator: locator.clone(),
                },
            ))
            .map_err(|_| ResidentOperationFrameFailure::ResidentUnavailable)
    }

    fn release_cancellation(&mut self, reservation: Self::Reservation) {
        assert!(
            self.retained.release(reservation),
            "the exact cancellation reservation must remain vacant"
        );
    }

    fn with_selected_cancellation_inputs<Return, UseInputs>(
        &mut self,
        selector: &super::ResidentDeploymentAuthoritySelector,
        request: &CommonRequestEnvelopeV1,
        locator: &ControlRecordCoordinate,
        use_inputs: UseInputs,
    ) -> Result<Return, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(program::ResidentProgramCancellationInputs<'input>) -> Return,
    {
        let mut input_owner = self
            .cancel_inputs
            .take()
            .ok_or(ResidentOperationFrameFailure::ResidentUnavailable)?;
        let deployment = ResidentServiceDeploymentSelector {
            principal_id: selector.principal_id.clone(),
            deployment_binding_ref: selector.deployment_binding_ref.clone(),
            deployment_binding_sha256: selector.deployment_binding_sha256.clone(),
            runtime_authority_ref: selector.runtime_authority_ref.clone(),
            runtime_authority_sha256: selector.runtime_authority_sha256.clone(),
        };
        let mut output = None;
        let lender =
            input_owner.with_cancellation_inputs(&deployment, request, locator, |inputs| {
                let allocation = inputs.allocation;
                output = Some(use_inputs(program::ResidentProgramCancellationInputs {
                    allocation: program::ResidentProgramCancellationAllocation {
                        request_ref: allocation.request_ref,
                        observation_ref: allocation.observation_ref,
                        signing: allocation.signing,
                        checkpoint: allocation.checkpoint,
                        checkpoint_issued_at: allocation.checkpoint_issued_at,
                        checkpoint_schema: allocation.checkpoint_schema,
                        accumulator_limits: allocation.accumulator_limits,
                        local_completion: crate::resident::operation_service::ResidentLocalCancellationCompletionInput {
                            diagnostic_ref: allocation.local_completion.diagnostic_ref,
                            observation_ref: allocation.local_completion.observation_ref,
                            established_at: allocation.local_completion.established_at,
                            checkpoint: allocation.local_completion.checkpoint,
                            checkpoint_issued_at: allocation.local_completion.checkpoint_issued_at,
                            maximum_diagnostic_bytes: allocation.local_completion.maximum_diagnostic_bytes,
                        },
                        consultation_limits: allocation.consultation_limits,
                    },
                    authorize: inputs.authorize,
                    resolve_store: inputs.resolve_store,
                    admit_rotation: inputs.admit_rotation,
                    sign_observation: inputs.sign_observation,
                    sign_checkpoint: inputs.sign_checkpoint,
                    admit_stop: inputs.admit_stop,
                    admit_collection: inputs.admit_collection,
                }));
            });
        self.cancel_inputs = Some(input_owner);
        settle_cancellation_lend(output, lender)
    }

    fn retain_dispatched_cancellation(
        &mut self,
        reservation: Self::Reservation,
        pending: Box<super::ResidentProcessDispatchedCancellationCustody>,
    ) {
        self.retained
            .retain_reserved(
                reservation,
                RetainedServicePhase::Cancellation(RetainedCancellationPhase::Dispatched(pending)),
            )
            .unwrap_or_else(|_| unreachable!("the cancellation reservation is linear"));
    }

    fn retain_cancellation_custody(
        &mut self,
        reservation: Self::Reservation,
        pending: Box<super::ResidentProcessCancellationCustody>,
    ) {
        self.retained
            .retain_reserved(
                reservation,
                RetainedServicePhase::Cancellation(RetainedCancellationPhase::Cancellation(
                    pending,
                )),
            )
            .unwrap_or_else(|_| unreachable!("the cancellation reservation is linear"));
    }

    fn retain_a2a_cancellation(
        &mut self,
        reservation: Self::Reservation,
        pending: super::ResidentProcessA2aDelegatePoll,
    ) {
        self.retained
            .retain_reserved(
                reservation,
                RetainedServicePhase::Cancellation(RetainedCancellationPhase::a2a(pending)),
            )
            .unwrap_or_else(|_| unreachable!("the cancellation reservation is linear"));
    }

    fn retain_consultation_cancellation(
        &mut self,
        reservation: Self::Reservation,
        pending: Box<super::ResidentProcessConsultationCancellationCustody>,
    ) {
        self.retained
            .retain_reserved(
                reservation,
                RetainedServicePhase::Cancellation(RetainedCancellationPhase::Consultation(
                    pending,
                )),
            )
            .unwrap_or_else(|_| unreachable!("the cancellation reservation is linear"));
    }

    fn retain_consultation_cancellation_preparation(
        &mut self,
        reservation: Self::Reservation,
        pending: Box<super::ResidentProcessConsultationCancellationPreparation>,
    ) {
        self.retained
            .retain_reserved(
                reservation,
                RetainedServicePhase::Cancellation(
                    RetainedCancellationPhase::ConsultationPreparation(pending),
                ),
            )
            .unwrap_or_else(|_| unreachable!("the cancellation reservation is linear"));
    }

    fn has_retained_cancellation(&self) -> bool {
        self.retained
            .iter()
            .any(RetainedServicePhase::is_cancellation)
    }

    fn retained_cancellation_selector(
        &self,
    ) -> Option<&super::ResidentDeploymentAuthoritySelector> {
        let key = self
            .retained
            .first_key_where(RetainedServicePhase::is_cancellation)?;
        match key {
            RetainedServicePhaseKey::Cancellation(key) => Some(&key.selector),
            RetainedServicePhaseKey::Invoke(_) => None,
        }
    }

    fn continue_one_retained_cancellation<WriterGuard, Authority>(
        &mut self,
        process: &mut ResidentProcessHost<'_, WriterGuard>,
        access: super::ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
    ) -> program::ResidentProgramRetainedPhaseTurn
    where
        WriterGuard: PrincipalWriterGuardPort,
        Authority: AmbientSessionAuthorityPort,
    {
        let Some((reservation, phase)) = self
            .retained
            .take_first_where(RetainedServicePhase::is_cancellation)
        else {
            return program::ResidentProgramRetainedPhaseTurn::Idle;
        };
        let RetainedServicePhase::Cancellation(phase) = phase else {
            unreachable!("the selected phase is cancellation")
        };
        let RetainedServicePhaseKey::Cancellation(exact) = &reservation.key else {
            unreachable!("the cancellation reservation keeps its exact key")
        };
        let exact = exact.clone();
        match phase {
            RetainedCancellationPhase::Cancellation(pending) => {
                match process.continue_cancellation_custody(access, authority, pending) {
                    Ok(()) => {
                        assert!(self.retained.release(reservation));
                        program::ResidentProgramRetainedPhaseTurn::Advanced
                    }
                    Err(failure) => {
                        self.retained
                            .retain_reserved(
                                reservation,
                                RetainedServicePhase::Cancellation(
                                    RetainedCancellationPhase::Cancellation(failure.pending),
                                ),
                            )
                            .unwrap_or_else(|_| {
                                unreachable!("the cancellation reservation remains linear")
                            });
                        program::ResidentProgramRetainedPhaseTurn::Deferred
                    }
                }
            }
            RetainedCancellationPhase::Dispatched(pending) => {
                match process.continue_dispatched_cancellation_custody(access, authority, pending) {
                    Ok(super::ResidentProcessCancellationOutcome::Operation(_)) => {
                        assert!(self.retained.release(reservation));
                        program::ResidentProgramRetainedPhaseTurn::Advanced
                    }
                    Ok(super::ResidentProcessCancellationOutcome::A2aDelegateReadStopped {
                        continuation,
                        ..
                    }) => {
                        self.retained
                            .retain_reserved(
                                reservation,
                                RetainedServicePhase::Cancellation(RetainedCancellationPhase::a2a(
                                    continuation,
                                )),
                            )
                            .unwrap_or_else(|_| {
                                unreachable!("the cancellation reservation remains linear")
                            });
                        program::ResidentProgramRetainedPhaseTurn::Advanced
                    }
                    Err(failure) => {
                        self.retained
                            .retain_reserved(
                                reservation,
                                RetainedServicePhase::Cancellation(
                                    RetainedCancellationPhase::Dispatched(failure.pending),
                                ),
                            )
                            .unwrap_or_else(|_| {
                                unreachable!("the cancellation reservation remains linear")
                            });
                        program::ResidentProgramRetainedPhaseTurn::Deferred
                    }
                }
            }
            RetainedCancellationPhase::Consultation(pending) => {
                match process.continue_consultation_cancellation_custody(access, authority, pending)
                {
                    Ok(_) => {
                        assert!(self.retained.release(reservation));
                        program::ResidentProgramRetainedPhaseTurn::Advanced
                    }
                    Err(failure) => {
                        self.retained
                            .retain_reserved(
                                reservation,
                                RetainedServicePhase::Cancellation(
                                    RetainedCancellationPhase::Consultation(failure.pending),
                                ),
                            )
                            .unwrap_or_else(|_| {
                                unreachable!("the cancellation reservation remains linear")
                            });
                        program::ResidentProgramRetainedPhaseTurn::Deferred
                    }
                }
            }
            RetainedCancellationPhase::ConsultationPreparation(pending) => {
                match process
                    .continue_consultation_cancellation_preparation(access, authority, pending)
                {
                    Ok(()) => {
                        assert!(self.retained.release(reservation));
                        program::ResidentProgramRetainedPhaseTurn::Advanced
                    }
                    Err(failure) => {
                        self.retained
                            .retain_reserved(
                                reservation,
                                RetainedServicePhase::Cancellation(
                                    RetainedCancellationPhase::ConsultationPreparation(
                                        failure.pending,
                                    ),
                                ),
                            )
                            .unwrap_or_else(|_| {
                                unreachable!("the cancellation reservation remains linear")
                            });
                        program::ResidentProgramRetainedPhaseTurn::Deferred
                    }
                }
            }
            RetainedCancellationPhase::A2a(pending) => {
                match *pending {
                    pending @ super::ResidentProcessA2aDelegatePoll::Pending(_) => {
                        // Cancel never polls the remote task. This exact opaque
                        // continuation remains with its existing owner.
                        self.retain_cancellation_phase(
                            reservation,
                            RetainedCancellationPhase::a2a(pending),
                            program::ResidentProgramRetainedPhaseTurn::Deferred,
                        )
                    }
                    pending @ super::ResidentProcessA2aDelegatePoll::SettlementRejected {
                        ..
                    } => match process.retry_a2a_delegate_settlement(access, authority, pending) {
                        Ok(next) => self.retain_cancellation_phase(
                            reservation,
                            RetainedCancellationPhase::a2a(next),
                            program::ResidentProgramRetainedPhaseTurn::Advanced,
                        ),
                        Err(pending) => self.retain_cancellation_phase(
                            reservation,
                            RetainedCancellationPhase::a2a(*pending),
                            program::ResidentProgramRetainedPhaseTurn::Deferred,
                        ),
                    },
                    super::ResidentProcessA2aDelegatePoll::Diagnostic(pending) => {
                        match process.recover_a2a_delegate_diagnostic(access, authority, pending) {
                            Ok(completed) => self.retain_cancellation_phase(
                                reservation,
                                RetainedCancellationPhase::a2a(
                                    super::ResidentProcessA2aDelegatePoll::Completed(completed),
                                ),
                                program::ResidentProgramRetainedPhaseTurn::Advanced,
                            ),
                            Err(pending) => self.retain_cancellation_phase(
                                reservation,
                                RetainedCancellationPhase::a2a(
                                    super::ResidentProcessA2aDelegatePoll::Diagnostic(*pending),
                                ),
                                program::ResidentProgramRetainedPhaseTurn::Deferred,
                            ),
                        }
                    }
                    super::ResidentProcessA2aDelegatePoll::Completed(completed) => self
                        .retain_a2a_cancellation_completion(
                            process,
                            access,
                            authority,
                            reservation,
                            &exact,
                            completed,
                        ),
                    pending @ super::ResidentProcessA2aDelegatePoll::Unrouted { .. } => {
                        match process
                            .relinquish_unrouted_a2a_delegate_to_recovery(access.store, pending)
                        {
                            Ok(()) => {
                                assert!(self.retained.release(reservation));
                                program::ResidentProgramRetainedPhaseTurn::Advanced
                            }
                            Err(pending) => self.retain_cancellation_phase(
                                reservation,
                                RetainedCancellationPhase::a2a(*pending),
                                program::ResidentProgramRetainedPhaseTurn::Deferred,
                            ),
                        }
                    }
                }
            }
            RetainedCancellationPhase::A2aOutcome(pending) => {
                let result = match *pending {
                    pending @ ResidentProcessA2aDelegateRetentionFailure::Selection { .. } => {
                        let ResidentProcessA2aDelegateRetentionFailure::Selection {
                            pending, ..
                        } = pending
                        else {
                            unreachable!()
                        };
                        process.retain_a2a_delegate_outcome(access, authority, *pending)
                    }
                    pending @ ResidentProcessA2aDelegateRetentionFailure::Retention { .. } => {
                        process.continue_a2a_delegate_outcome_custody(access, authority, pending)
                    }
                    pending @ ResidentProcessA2aDelegateRetentionFailure::Finalization {
                        ..
                    } => process.retry_a2a_delegate_finalization(access, authority, pending),
                    pending @ ResidentProcessA2aDelegateRetentionFailure::Route { .. } => {
                        process.retry_a2a_delegate_route_release(pending)
                    }
                };
                match result {
                    Ok(_) => {
                        assert!(self.retained.release(reservation));
                        program::ResidentProgramRetainedPhaseTurn::Advanced
                    }
                    Err(pending) => self.retain_cancellation_phase(
                        reservation,
                        RetainedCancellationPhase::A2aOutcome(pending),
                        program::ResidentProgramRetainedPhaseTurn::Deferred,
                    ),
                }
            }
        }
    }
}

fn settle_cancellation_lend<Output>(
    output: Option<Output>,
    lender: Result<(), ResidentOperationFrameFailure>,
) -> Result<Output, ResidentOperationFrameFailure> {
    match output {
        // Once inputs were lent, the produced linear handoff takes
        // precedence over any post-callback lender error.
        Some(output) => Ok(output),
        None => Err(lender
            .err()
            .unwrap_or(ResidentOperationFrameFailure::ConfigurationUnavailable)),
    }
}

struct ObserveTurnOwner<
    'historical,
    'ports,
    Evidence,
    Clock,
    SessionAuthority,
    Signature,
    OperationAuthority,
    Recovery,
> {
    peer_admission: PeerAdmissionAdapter<'historical>,
    current: program::ResidentProgramCurrentRequestAdmission<
        'historical,
        ResidentPeerAdmissionWitness,
        CurrentEvidenceAdapter<Evidence>,
        Clock,
        SessionAuthority,
        CallerSignatureAdapter<Signature>,
    >,
    deployments: Box<[program::ResidentProgramDeploymentOperationPorts<'ports>]>,
    operation_authority: OperationAuthority,
    recovery: ObserveInputAdapter<Recovery>,
}

impl<
        'historical,
        'ports,
        WriterGuard,
        Evidence,
        Clock,
        SessionAuthority,
        Signature,
        OperationAuthority,
        Recovery,
    > program::ResidentProgramTurnOwner<WriterGuard, UnavailableOperationPolicy, ()>
    for ObserveTurnOwner<
        'historical,
        'ports,
        Evidence,
        Clock,
        SessionAuthority,
        Signature,
        OperationAuthority,
        Recovery,
    >
where
    WriterGuard: PrincipalWriterGuardPort,
{
    type PeerWitness = ResidentPeerAdmissionWitness;

    fn admit_operation_peer(
        &mut self,
        peer: LocalIpcPeerCredentials,
        request: &ResidentOperationFrameRequest,
    ) -> Result<Self::PeerWitness, InvalidResidentRecordV1> {
        program::ResidentProgramPeerAdmissionPort::admit_peer(
            &mut self.peer_admission,
            peer,
            request,
        )
    }

    fn resolve_operation<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, WriterGuard>,
        _: &mut UnavailableOperationPolicy,
        _: &[program::ResidentProgramDeploymentAuthority],
        _: &Self::PeerWitness,
        _authenticated_key: Option<[u8; 32]>,
        _: &ResidentOperationFrameRequest,
    ) -> Result<program::ResidentProgramOperationResolution, ResidentOperationFrameFailure> {
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    fn answer_operation<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, WriterGuard>,
        _: &mut UnavailableOperationPolicy,
        _: &program::ResidentProgramOperationResolution,
    ) -> Result<
        operation_request_host::ResidentOperationRequestOutcome,
        ResidentOperationFrameFailure,
    > {
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    fn recover_root_provider<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, WriterGuard>,
        _: &super::ResidentDeploymentAuthoritySelector,
    ) -> Result<(), program::ResidentProgramTurnError> {
        Err(program::ResidentProgramTurnError::RecoveryUnavailable)
    }
    fn recover_root_consult<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, WriterGuard>,
        _: &mut UnavailableOperationPolicy,
        _: &super::ResidentDeploymentAuthoritySelector,
    ) -> Result<(), program::ResidentProgramTurnError> {
        Err(program::ResidentProgramTurnError::RecoveryUnavailable)
    }
    fn recover_consult_child<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, WriterGuard>,
        _: &mut UnavailableOperationPolicy,
        _: &super::ResidentDeploymentAuthoritySelector,
    ) -> Result<(), program::ResidentProgramTurnError> {
        Err(program::ResidentProgramTurnError::RecoveryUnavailable)
    }
    fn recover_delegate<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, WriterGuard>,
        _: &super::ResidentDeploymentAuthoritySelector,
    ) -> Result<(), program::ResidentProgramTurnError> {
        Err(program::ResidentProgramTurnError::RecoveryUnavailable)
    }
}

impl<
        'historical,
        'ports,
        WriterGuard,
        Evidence,
        Clock,
        SessionAuthority,
        Signature,
        OperationAuthority,
        Recovery,
    > program::ResidentProgramObserveTurnOwner<WriterGuard>
    for ObserveTurnOwner<
        'historical,
        'ports,
        Evidence,
        Clock,
        SessionAuthority,
        Signature,
        OperationAuthority,
        Recovery,
    >
where
    WriterGuard: PrincipalWriterGuardPort,
    Evidence: ResidentServiceCurrentRequestEvidencePort,
    Clock: WallClockSourcePort,
    SessionAuthority: AmbientSessionAuthorityPort,
    Signature: ResidentServiceCallerSignaturePort,
    OperationAuthority: AmbientSessionAuthorityPort,
    Recovery: ResidentServiceObserveInputPort<'ports>,
{
    type PeerWitness = ResidentPeerAdmissionWitness;

    fn admit_observe_peer(
        &mut self,
        peer: LocalIpcPeerCredentials,
        request: &ResidentObserveFrameRequest,
    ) -> Result<Self::PeerWitness, InvalidResidentRecordV1> {
        program::ResidentProgramObservePeerAdmissionPort::admit_observe_peer(
            &mut self.peer_admission,
            peer,
            request,
        )
    }

    fn answer_observe<'store>(
        &mut self,
        process: &mut ResidentProcessHost<'store, WriterGuard>,
        authorities: &[program::ResidentProgramDeploymentAuthority],
        peer: &Self::PeerWitness,
        frame: &ResidentObserveFrameRequest,
    ) -> Result<(OperationControlObservation, Instant), ResidentObserveFrameFailure> {
        program::answer_observe_with(
            &mut self.current,
            &mut self.deployments,
            &mut self.operation_authority,
            &mut self.recovery,
            process,
            authorities,
            peer,
            frame,
        )
    }
}

struct UnavailableOperationPolicy;

impl ResidentProcessInvokePolicyEvaluator for UnavailableOperationPolicy {
    fn evaluate_invoke(
        &mut self,
        _: ResidentProcessInvokePolicyContext<'_>,
    ) -> Result<ResidentProcessInvokePolicy, ResidentProcessHostSelectionError> {
        Err(ResidentProcessHostSelectionError::UnknownDeploymentAuthority)
    }
}

impl consult_recovery::ResidentConsultationPolicyEvaluator<()> for UnavailableOperationPolicy {
    type Error = InvalidResidentRecordV1;

    fn evaluate(
        &mut self,
        _: &ImmutableRecordReferenceV1,
        _: &ResidentConsultationPolicyFrontier,
    ) -> Result<ResidentConsultationSupervisionDecision<()>, Self::Error> {
        Err(InvalidResidentRecordV1::ContextualAdmission {
            record: "resident operation policy",
            invariant: "Observe-only service has no Consult policy",
        })
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/resident/process_host/service_assembly.rs"]
mod tests;

#[cfg(test)]
#[path = "../../../tests/internal/resident/process_host/retained_provider_invoke.rs"]
pub(in crate::resident::process_host) mod retained_provider_invoke_tests;
