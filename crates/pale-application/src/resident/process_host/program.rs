//! Finite process-lifetime composition for one admitted Pale resident.
//!
//! This owner joins a held native writer guard, the exact current writer
//! snapshot, the admitted principal trust frontier, cold-opened deployment
//! applications, and the single process operation owner. It discovers no
//! endpoint, store, key, principal, deployment, provider, or policy.

use std::{collections::VecDeque, marker::PhantomData, num::NonZeroUsize, time::Instant};

use pale_core::records::{
    CommonRequestAdmissionContextV1, CommonRequestEnvelopeV1, InvalidResidentRecordV1,
    OperationHandleDigest, OperationTargetReferenceV1, ResidentRecordRefV1,
};
use pale_ports::{
    AntiRollbackAnchorPort, CryptographicRandomSourcePort, DurableAuthorityStorePort,
    EventCustodyPort, LocalIpcFramePort, LocalIpcPeerCredentials, OperationCustodyPort,
    PrincipalResidentWriterSnapshot, PrincipalWriterGuardPort, ResidentObserveFrameFailure,
    ResidentObserveFrameRequest, ResidentOperationFrameCancellationDisposition,
    ResidentOperationFrameFailure, ResidentOperationFrameLocator, ResidentOperationFrameRequest,
    ResidentOperationFrameResponse, SessionLeaseCustodyPort,
};

use super::current_request::{
    ResidentCurrentRequestEvidence, ResidentCurrentRequestEvidencePort,
    ResidentCurrentRequestSelector, ResidentRetainedCommonRequestPolicy,
};
use super::*;
#[path = "consult_initialization_recovery.rs"]
mod consult_initialization_recovery;
#[path = "native_hook_turn.rs"]
pub(super) mod native_hook_turn;
use crate::resident::{
    principal_trust_frontier::RecoveredCurrentPrincipalTrustFrontier,
    OperationControlRecoveryInput, ResidentApplicationError, ResidentColdOpenInput,
    ResidentColdOpenOutcome,
};

/// One caller-opened deployment store with all cold-recovery dependencies.
///
/// The program consumes this value once. The store coordinate comes from the
/// cold-open input and must equal one authority derived from the admitted trust
/// frontier. This input cannot discover or substitute another store.
pub(in crate::resident) struct ResidentProgramDeploymentOpen<'store, 'input> {
    pub(in crate::resident) durable: &'store dyn DurableAuthorityStorePort,
    pub(in crate::resident) sessions: &'store dyn SessionLeaseCustodyPort,
    pub(in crate::resident) events: &'input mut dyn EventCustodyPort,
    pub(in crate::resident) maximum_event_custody_bytes: usize,
    pub(in crate::resident) input: ResidentColdOpenInput<'input>,
}

/// Process owner assembled from already-selected physical and authority ports.
///
/// The policy evaluator is mandatory and retained with the resident. A caller
/// cannot create a policy-free Consult program and cannot replace the evaluator
/// between bounded turns.
pub(in crate::resident) struct ResidentProgram<
    'store,
    WriterGuard,
    PolicyEvaluator,
    TurnOwner,
    StopInput,
> {
    process: ResidentProcessHost<'store, WriterGuard>,
    deployment_authorities: Box<[ResidentProgramDeploymentAuthority]>,
    consult_policy: PolicyEvaluator,
    turn_owner: TurnOwner,
    recovery_deployment: usize,
    recovery_source: ResidentProgramRecoverySource,
    stop_input: PhantomData<fn() -> StopInput>,
}

/// Full admitted trust projection retained for one ready deployment.
///
/// [`AdmittedResidentDeploymentAssociation`] intentionally keeps only the
/// coordinates needed to fence process-host access. Request admission also
/// needs the already-admitted binding, runtime authority, and current principal
/// trust checkpoint bodies. Retaining them here avoids a second store read and
/// prevents an untrusted request from selecting replacement authority records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::resident) struct ResidentProgramDeploymentAuthority {
    selector: ResidentDeploymentAuthoritySelector,
    deployment_binding: pale_core::records::DeploymentBindingV1,
    runtime_authority: pale_core::records::DeploymentRuntimeAuthorityV1,
    current_trust_ref: pale_core::records::ResidentRecordRefV1,
    current_trust: pale_core::records::PrincipalTrustCheckpointV1,
}

impl ResidentProgramDeploymentAuthority {
    pub(super) fn from_recovered(
        recovered: &crate::resident::principal_trust_frontier::RecoveredDeploymentRuntimeAuthority,
        current: &crate::resident::principal_trust_recovery::RecoveredSignedPrincipalTrustCheckpoint,
    ) -> Self {
        let association = AdmittedResidentDeploymentAssociation::from_recovered(recovered);
        Self {
            selector: ResidentDeploymentAuthoritySelector::from_association(&association),
            deployment_binding: recovered.binding().clone(),
            runtime_authority: recovered.authority().clone(),
            current_trust_ref: current.coordinate().record_ref().clone(),
            current_trust: current.checkpoint().clone(),
        }
    }

    /// Borrow the complete private selector for this ready deployment.
    pub(in crate::resident) const fn selector(&self) -> &ResidentDeploymentAuthoritySelector {
        &self.selector
    }

    /// Borrow the admitted deployment binding body.
    const fn deployment_binding(&self) -> &pale_core::records::DeploymentBindingV1 {
        &self.deployment_binding
    }

    /// Borrow the admitted current runtime-authority body.
    const fn runtime_authority(&self) -> &pale_core::records::DeploymentRuntimeAuthorityV1 {
        &self.runtime_authority
    }

    /// Borrow the exact immutable current principal-trust reference.
    const fn current_trust_ref(&self) -> &pale_core::records::ResidentRecordRefV1 {
        &self.current_trust_ref
    }

    /// Borrow the admitted current principal-trust checkpoint.
    const fn current_trust(&self) -> &pale_core::records::PrincipalTrustCheckpointV1 {
        &self.current_trust
    }
}

/// Exact retained session, generation, and operation selected after peer admission.
///
/// Every body is copied from application or authority custody so one bounded
/// turn can rebuild borrowed admission contexts without self-references.
struct ResidentProgramRetainedRequest {
    selector: ResidentDeploymentAuthoritySelector,
    authority: ResidentProgramDeploymentAuthority,
    session: crate::resident::session_admission::RetainedAmbientSession,
    active_head: pale_core::records::ActiveGenerationHeadV1,
    compiled_generation: pale_core::records::CompiledGenerationV1,
    effective_profile: pale_core::records::EffectiveProjectProfileV1,
    activation_fence: pale_core::records::ActivationFenceV1,
    session_signing_authority: pale_core::records::SigningAuthorityBindingV1,
    session_schema_sha256: pale_core::records::SchemaDigest,
    operation: crate::semantic::ResolvedOperation,
}

impl ResidentProgramRetainedRequest {
    /// Rebuild the immutable generation context retained by this session.
    fn generation_context(&self) -> pale_core::records::SessionAdmissionLeaseContextV1<'_> {
        pale_core::records::SessionAdmissionLeaseContextV1 {
            active_head: &self.active_head,
            compiled_generation: &self.compiled_generation,
            effective_profile: &self.effective_profile,
            activation_fence: &self.activation_fence,
            signed: pale_core::records::SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &self.session_schema_sha256,
                signing_authority: &self.session_signing_authority,
            },
        }
    }

    /// Lend one ambient context assembled from retained and freshly read evidence.
    fn with_ambient_context_from<Output>(
        &self,
        evidence: &ResidentCurrentRequestEvidence,
        now: pale_core::records::UnixTimeV1,
        use_context: impl FnOnce(crate::resident::AmbientSessionAdmissionContext<'_>) -> Output,
    ) -> Output {
        let deployment_revocations = evidence.deployment_revocation_frontier();
        let session_revocations = evidence.session_revocation_frontier();
        let route_use = self.session.basis().route_use();
        use_context(crate::resident::AmbientSessionAdmissionContext {
            now,
            subject: evidence.grant.grant.subject(),
            grant: Some(evidence.grant_authority()),
            current_principal_trust: Some(crate::resident::CurrentPrincipalTrustHead {
                checkpoint_ref: self.authority.current_trust_ref(),
                checkpoint: self.authority.current_trust(),
            }),
            capability_revocations: Some(evidence.capability_revocation_frontier()),
            deployment_binding_ref: &self.authority.selector.deployment_binding_ref,
            deployment_binding: self.authority.deployment_binding(),
            deployment_revocations: &deployment_revocations,
            runtime_authority: Some(crate::resident::DeploymentRuntimeAuthorityHead {
                authority_ref: &self.authority.selector.runtime_authority_ref,
                authority: self.authority.runtime_authority(),
            }),
            session_revocations: &session_revocations,
            route_use: &route_use,
            generation: self.generation_context(),
        })
    }
}

/// Why post-peer retained request selection stopped before current admission.
#[derive(Debug, thiserror::Error)]
enum ResidentProgramRetainedRequestError {
    /// No unique admitted deployment matches the signed session coordinate.
    #[error("request does not select one admitted deployment authority")]
    Deployment,
    /// The exact retained session is absent or substituted.
    #[error(transparent)]
    Session(#[from] crate::resident::application::ExactRetainedSessionRecoveryError),
    /// The retained session no longer matches the selected deployment authority.
    #[error("retained session does not match the selected deployment authority")]
    SessionAuthority,
    /// The signed generation or operation is absent from the retained session closure.
    #[error("request operation is absent from the exact retained generation")]
    Operation,
    /// Required retained generation-admission evidence is absent or substituted.
    #[error("retained session generation evidence is unavailable")]
    GenerationEvidence,
    /// The embedded session contract identity is unavailable.
    #[error(transparent)]
    Contract(#[from] pale_contracts::SchemaError),
}

fn select_retained_request(
    process: &ResidentProcessHost<'_, impl PrincipalWriterGuardPort>,
    deployment_authorities: &[ResidentProgramDeploymentAuthority],
    preimage: &pale_core::records::CommonRequestEnvelopePreimageV1,
    historical: &dyn crate::resident::GenerationLifecycleRecoveryResolver,
) -> Result<ResidentProgramRetainedRequest, ResidentProgramRetainedRequestError> {
    let claimed_session = &preimage.session_admission;
    let mut matches = deployment_authorities.iter().filter(|authority| {
        authority.selector.deployment_binding_ref == claimed_session.deployment_binding_ref
            && authority.selector.deployment_binding_sha256.as_str()
                == claimed_session.deployment_binding_sha256.as_str()
    });
    let authority = matches
        .next()
        .ok_or(ResidentProgramRetainedRequestError::Deployment)?;
    if matches.next().is_some() {
        return Err(ResidentProgramRetainedRequestError::Deployment);
    }
    let deployment = process
        .deployment_authority_index(authority.selector())
        .map_err(|_| ResidentProgramRetainedRequestError::Deployment)?;
    let application = &process.deployments[deployment].application;
    let session = application.recover_exact_retained_session(
        &claimed_session.session_admission_id,
        &claimed_session.session_admission_lease_digest,
        historical,
    )?;
    let lease = session.lease();
    if lease.deployment_binding_ref() != &authority.selector().deployment_binding_ref
        || lease.deployment_binding_sha256().as_str()
            != authority.selector().deployment_binding_sha256.as_str()
        || lease.deployment_runtime_authority_ref() != &authority.selector().runtime_authority_ref
        || lease.deployment_runtime_authority_sha256().as_str()
            != authority.selector().runtime_authority_sha256.as_str()
        || lease.issuer_principal_id() != &authority.selector().principal_id
    {
        return Err(ResidentProgramRetainedRequestError::SessionAuthority);
    }
    let Some(request_generation) = preimage.generation.as_ref() else {
        return Err(ResidentProgramRetainedRequestError::Operation);
    };
    if request_generation != lease.compiled_generation_digest() {
        return Err(ResidentProgramRetainedRequestError::Operation);
    }
    let retained_generation = application
        .generation(request_generation)
        .ok_or(ResidentProgramRetainedRequestError::Operation)?;
    let operation = retained_generation
        .operation(lease.repository_id(), &preimage.operation)
        .cloned()
        .ok_or(ResidentProgramRetainedRequestError::Operation)?;
    let compiled_generation = retained_generation.record().clone();
    if historical.compiled_generation(request_generation) != Some(&compiled_generation) {
        return Err(ResidentProgramRetainedRequestError::GenerationEvidence);
    }
    let lease_wire = pale_core::records::SessionAdmissionLeaseWireV1::from(lease);
    let active_head = historical
        .active_head(lease.active_generation_head_digest())
        .cloned()
        .ok_or(ResidentProgramRetainedRequestError::GenerationEvidence)?;
    let effective_profile = historical
        .effective_profile(lease_wire.profile_digest())
        .cloned()
        .ok_or(ResidentProgramRetainedRequestError::GenerationEvidence)?;
    let activation_fence = historical
        .activation_fence(lease.activation_fence_digest())
        .cloned()
        .ok_or(ResidentProgramRetainedRequestError::GenerationEvidence)?;
    let session_signing_authority = historical
        .session_signing_authority(
            lease.deployment_runtime_authority_ref(),
            lease.deployment_runtime_authority_sha256(),
            lease.issuer_principal_id(),
            lease.issuer_key_id(),
            *lease.issuer_key_epoch(),
        )
        .ok_or(ResidentProgramRetainedRequestError::GenerationEvidence)?;
    let session_schema_sha256 = pale_contracts::record_schema_identity(
        pale_core::records::SESSION_ADMISSION_LEASE_V1_SCHEMA_ID,
    )?
    .schema_sha256;
    Ok(ResidentProgramRetainedRequest {
        selector: authority.selector().clone(),
        authority: authority.clone(),
        session,
        active_head,
        compiled_generation,
        effective_profile,
        activation_fence,
        session_signing_authority,
        session_schema_sha256,
        operation,
    })
}

/// Exact recovery owner selected for one bounded program turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::resident) enum ResidentProgramRecoverySource {
    /// One committed root provider start.
    RootProvider,
    /// One committed root consultation.
    RootConsult,
    /// One committed consultation child.
    ConsultChild,
    /// One committed pairwise delegation.
    Delegate,
    /// One bounded Event-publication stage.
    Publication,
}

impl ResidentProgramRecoverySource {
    const fn next(self) -> Self {
        match self {
            Self::RootProvider => Self::RootConsult,
            Self::RootConsult => Self::ConsultChild,
            Self::ConsultChild => Self::Delegate,
            Self::Delegate => Self::Publication,
            Self::Publication => Self::RootProvider,
        }
    }
}

/// Application-owned resolution of one admitted operation frame.
///
/// The retained turn owner creates this value only after supplementary peer
/// admission and complete common-request admission. The frame tag alone cannot
/// construct it or select a semantic owner.
pub(in crate::resident) struct ResidentProgramOperationResolution {
    request: CommonRequestEnvelopeV1,
    kind: operation_request_host::ResidentOperationRequestKind,
    locator: Option<ResidentOperationFrameLocator>,
}

impl ResidentProgramOperationResolution {
    /// Join one admitted common request to the exact received frame shape.
    pub(in crate::resident) fn admit(
        frame: &ResidentOperationFrameRequest,
        request: CommonRequestEnvelopeV1,
        kind: operation_request_host::ResidentOperationRequestKind,
        locator: Option<ResidentOperationFrameLocator>,
    ) -> Result<Self, ResidentOperationFrameFailure> {
        let candidate = Self {
            request,
            kind,
            locator,
        };
        let admitted = candidate
            .request_resolution()
            .admits(frame)
            .map_err(|_| ResidentOperationFrameFailure::InvalidRequest)?;
        if !admitted {
            return Err(ResidentOperationFrameFailure::InvalidRequest);
        }
        if let Some(target) = operation_frame_target(candidate.kind, candidate.locator.as_ref())? {
            if !request_admission::request_binds_operation_target(
                &candidate.request.to_preimage(),
                &target,
            ) {
                return Err(ResidentOperationFrameFailure::InvalidRequest);
            }
        }
        Ok(candidate)
    }

    fn request_resolution(&self) -> operation_request_host::ResidentOperationRequestResolution<'_> {
        use operation_request_host::ResidentOperationRequestKind;
        match (&self.kind, self.locator.clone()) {
            (ResidentOperationRequestKind::StartInvoke, None)
            | (ResidentOperationRequestKind::StartConsult, None) => {
                operation_request_host::ResidentOperationRequestResolution::root_start(
                    self.kind,
                    &self.request,
                )
            }
            (ResidentOperationRequestKind::StartDelegate, Some(parent)) => {
                operation_request_host::ResidentOperationRequestResolution::child_start(
                    &self.request,
                    parent,
                )
            }
            (_, Some(locator)) => {
                operation_request_host::ResidentOperationRequestResolution::retained(
                    self.kind,
                    &self.request,
                    locator,
                )
            }
            _ => operation_request_host::ResidentOperationRequestResolution::root_start(
                self.kind,
                &self.request,
            ),
        }
    }

    pub(in crate::resident) fn request(&self) -> &CommonRequestEnvelopeV1 {
        &self.request
    }

    /// Return the application-admitted operation journey.
    const fn kind(&self) -> operation_request_host::ResidentOperationRequestKind {
        self.kind
    }

    /// Borrow the exact caller-supplied locator after frame association.
    fn locator(&self) -> Option<&ResidentOperationFrameLocator> {
        self.locator.as_ref()
    }
}

/// Current admitted facts for one point-addressed Delegate resolution.
///
/// The application creates this value only after peer admission, exact request
/// association, operation recovery, and pairwise-lineage admission. It carries
/// no route selector, record allocator, exchange port, or reusable send
/// capability. The injected Delegate owner must join these facts to its own
/// already-admitted exact dispatch and route inputs.
pub(in crate::resident) struct ResidentProgramDelegateResolutionInput<'input, 'context> {
    request: &'input CommonRequestEnvelopeV1,
    session: &'input crate::resident::AdmittedAmbientSession,
    current: &'input crate::resident::AmbientSessionAdmissionContext<'context>,
    maximum_selection_bytes: usize,
    maximum_record_bytes: usize,
}

impl<'input, 'context> ResidentProgramDelegateResolutionInput<'input, 'context> {
    /// Bundle facts that the current resident turn has already admitted.
    ///
    /// This value grants no route, transport, or effect authority. The selected
    /// Delegate owner must still join these facts to its admitted physical
    /// inputs before it can prepare external work.
    pub(in crate::resident) const fn new(
        request: &'input CommonRequestEnvelopeV1,
        session: &'input crate::resident::AdmittedAmbientSession,
        current: &'input crate::resident::AmbientSessionAdmissionContext<'context>,
        maximum_selection_bytes: usize,
        maximum_record_bytes: usize,
    ) -> Self {
        Self {
            request,
            session,
            current,
            maximum_selection_bytes,
            maximum_record_bytes,
        }
    }

    /// Borrow the exact request admitted for this resident turn.
    pub(in crate::resident) const fn request(&self) -> &'input CommonRequestEnvelopeV1 {
        self.request
    }

    /// Borrow the exact admitted ambient session.
    pub(in crate::resident) const fn session(
        &self,
    ) -> &'input crate::resident::AdmittedAmbientSession {
        self.session
    }

    /// Borrow the current authority and generation evidence for that session.
    pub(in crate::resident) const fn current(
        &self,
    ) -> &'input crate::resident::AmbientSessionAdmissionContext<'context> {
        self.current
    }

    /// Return the maximum bytes permitted for one retained selection record.
    pub(in crate::resident) const fn maximum_selection_bytes(&self) -> usize {
        self.maximum_selection_bytes
    }

    /// Return the maximum bytes permitted for one retained resident record.
    pub(in crate::resident) const fn maximum_record_bytes(&self) -> usize {
        self.maximum_record_bytes
    }
}

/// Supplementary local-peer admission used by the concrete application turn owner.
///
/// This port may inspect only transport credentials and the received frame. It
/// cannot read a deployment, resolve a session, or select an operation.
pub(in crate::resident) trait ResidentProgramPeerAdmissionPort {
    /// Witness retained until request association completes.
    type Witness;

    /// Admit the current peer for a captured hook without inventing an operation frame.
    fn admit_hook_peer(
        &mut self,
        _peer: LocalIpcPeerCredentials,
    ) -> Result<Self::Witness, InvalidResidentRecordV1> {
        Err(InvalidResidentRecordV1::ContextualAdmission {
            record: "native hook peer",
            invariant: "native hook peer policy is unavailable",
        })
    }

    /// Admit the connected peer before protected application state is used.
    fn admit_peer(
        &mut self,
        peer: LocalIpcPeerCredentials,
        request: &ResidentOperationFrameRequest,
    ) -> Result<Self::Witness, InvalidResidentRecordV1>;
}

/// Supplementary local-peer admission for the separate Observe frame.
///
/// Observe is not represented as an operation-frame kind. Keeping this port
/// separate prevents a peer owner from seeing a fabricated Cancel request.
pub(in crate::resident) trait ResidentProgramObservePeerAdmissionPort:
    ResidentProgramPeerAdmissionPort
{
    /// Admit the connected peer against the received Observe frame.
    fn admit_observe_peer(
        &mut self,
        peer: LocalIpcPeerCredentials,
        request: &ResidentObserveFrameRequest,
    ) -> Result<Self::Witness, InvalidResidentRecordV1>;
}

/// Verify the caller signature selected by retained request policy.
pub(in crate::resident) trait ResidentProgramCallerSignaturePort {
    /// Bind this frame's transport key to the independently admitted caller.
    fn admit_native_caller_key(
        &mut self,
        _caller: &pale_core::records::RequestCallerV1,
        _authenticated_key: &[u8; 32],
    ) -> Result<(), InvalidResidentRecordV1> {
        Err(InvalidResidentRecordV1::ContextualAdmission {
            record: "native tool caller",
            invariant: "authenticated connection is not bound to the signed caller",
        })
    }

    /// Verify the exact canonical request identity under the selected caller key.
    /// The signature covers the common-request domain and decoded digest,
    /// independently of the connection's authentication proof.
    fn verify_caller_signature(
        &mut self,
        preimage: &pale_core::records::CommonRequestEnvelopePreimageV1,
        digest: &pale_core::records::CommonRequestEnvelopeDigest,
        signature: &pale_core::records::CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>;
}

impl<F> ResidentProgramCallerSignaturePort for F
where
    F: FnMut(
        &pale_core::records::CommonRequestEnvelopePreimageV1,
        &pale_core::records::CommonRequestEnvelopeDigest,
        &pale_core::records::CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>,
{
    fn verify_caller_signature(
        &mut self,
        preimage: &pale_core::records::CommonRequestEnvelopePreimageV1,
        digest: &pale_core::records::CommonRequestEnvelopeDigest,
        signature: &pale_core::records::CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        self(preimage, digest, signature)
    }
}

/// One frame-kind-specific exact request policy retained by the application.
#[derive(Debug, Clone)]
pub(in crate::resident) struct ResidentProgramRequestPolicy {
    kind: ResidentProgramRequestPolicyKind,
    policy: ResidentRetainedCommonRequestPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResidentProgramRequestPolicyKind {
    Operation(operation_request_host::ResidentOperationRequestKind),
    Observe,
}

impl ResidentProgramRequestPolicy {
    /// Retain one compiled policy projection for one existing request journey.
    pub(in crate::resident) const fn new(
        kind: operation_request_host::ResidentOperationRequestKind,
        policy: ResidentRetainedCommonRequestPolicy,
    ) -> Self {
        Self {
            kind: ResidentProgramRequestPolicyKind::Operation(kind),
            policy,
        }
    }

    /// Retain the exact policy projection for the separate Observe ingress.
    pub(in crate::resident) const fn observe(policy: ResidentRetainedCommonRequestPolicy) -> Self {
        Self {
            kind: ResidentProgramRequestPolicyKind::Observe,
            policy,
        }
    }
}

/// Complete admitted request state retained between resolution and one answer.
pub(super) struct ResidentProgramAdmittedRequest {
    request: CommonRequestEnvelopeV1,
    selected: ResidentProgramRetainedRequest,
    evidence: ResidentCurrentRequestEvidence,
    session: crate::resident::AdmittedAmbientSession,
    now: pale_core::records::UnixTimeV1,
    policy: ResidentRetainedCommonRequestPolicy,
    session_admission: current_request::ResidentCurrentRequestSessionAdmission,
    window: current_request::ResidentAdmittedRequestWindow,
}

impl ResidentProgramAdmittedRequest {
    /// Reuse this turn's current admission for native correlation, without
    /// inventing another session or refreshing only part of its evidence.
    fn with_native_context<WriterGuard: PrincipalWriterGuardPort, Output>(
        &self,
        process: &ResidentProcessHost<'_, WriterGuard>,
        historical: &dyn crate::resident::GenerationLifecycleRecoveryResolver,
        use_context: impl FnOnce(native_hook_start::NativeHookStartContext<'_>) -> Output,
    ) -> Result<Output, ResidentOperationFrameFailure> {
        let index = process
            .deployment_authority_index(self.selector())
            .map_err(|_| ResidentOperationFrameFailure::ConfigurationUnavailable)?;
        Ok(self.with_ambient_context(|current| {
            use_context(native_hook_start::NativeHookStartContext {
                application: &process.deployments[index].application,
                session: self.session(),
                historical,
                current: &current,
            })
        }))
    }

    fn register_native_start<
        WriterGuard: PrincipalWriterGuardPort,
        Authority: crate::resident::AmbientSessionAuthorityPort,
    >(
        &self,
        process: &ResidentProcessHost<'_, WriterGuard>,
        native: &mut Option<native_hook_start::NativeHookStarts>,
        operation: &crate::resident::IssuedLocalOperation,
        historical: &dyn crate::resident::GenerationLifecycleRecoveryResolver,
        authority: &mut Authority,
    ) -> Result<(), ResidentOperationFrameFailure> {
        if self.request().body().schema_id.as_str()
            != pale_core::records::NATIVE_TOOL_REQUEST_V1_SCHEMA_ID
        {
            return Ok(());
        }
        let native = native
            .as_mut()
            .ok_or(ResidentOperationFrameFailure::AdmissionDenied)?;
        self.with_native_context(process, historical, |context| {
            native.issued(self.request(), operation, context, authority)
        })?
    }

    /// Borrow the exact caller-signed request after contextual admission.
    const fn request(&self) -> &CommonRequestEnvelopeV1 {
        &self.request
    }

    /// Borrow the deployment authority selected by the admitted session.
    const fn selector(&self) -> &ResidentDeploymentAuthoritySelector {
        &self.selected.selector
    }

    /// Borrow the revalidated ambient session for this bounded turn.
    const fn session(&self) -> &crate::resident::AdmittedAmbientSession {
        &self.session
    }

    /// Borrow the exact compiled operation selected by the request.
    const fn operation(&self) -> &crate::semantic::ResolvedOperation {
        &self.selected.operation
    }

    /// Lend the current ambient context without refreshing its evidence.
    ///
    /// This is valid only inside the same bounded turn. A later frame must read
    /// physical time and mutable frontiers again.
    fn with_ambient_context<Output>(
        &self,
        use_context: impl FnOnce(crate::resident::AmbientSessionAdmissionContext<'_>) -> Output,
    ) -> Output {
        self.selected
            .with_ambient_context_from(&self.evidence, self.now, use_context)
    }

    /// Lend the owner-resolved request context retained by this admission.
    fn with_request_context<Output>(
        &self,
        use_context: impl FnOnce(&CommonRequestAdmissionContextV1<'_>) -> Output,
    ) -> Output {
        let context = self
            .policy
            .admission_context(&self.session_admission, &self.window);
        use_context(&context)
    }

    /// Lend the existing start factory its exact admitted inputs.
    ///
    /// The generation is reselected from the ready deployment by the session's
    /// retained digest. No frame value can substitute a generation or operation.
    fn with_start_factory_basis<WriterGuard, Output>(
        &self,
        process: &ResidentProcessHost<'_, WriterGuard>,
        handle_signing_authority: &pale_core::records::SigningAuthorityBindingV1,
        checkpoint_authority: &pale_core::records::ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        use_basis: impl for<'basis> FnOnce(
            start_factories::ResidentOperationStartFactoryBasis<'basis>,
        ) -> Output,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        WriterGuard: PrincipalWriterGuardPort,
    {
        let deployment = process
            .deployment_authority_index(self.selector())
            .map_err(|_| ResidentOperationFrameFailure::ConfigurationUnavailable)?;
        let generation = process.deployments[deployment]
            .application
            .generation(self.session.lease().compiled_generation_digest())
            .ok_or(ResidentOperationFrameFailure::ConfigurationUnavailable)?;
        Ok(self.with_ambient_context(|current| {
            use_basis(start_factories::ResidentOperationStartFactoryBasis::new(
                self.selector(),
                self.request(),
                self.session(),
                &current,
                generation,
                handle_signing_authority,
                checkpoint_authority,
            ))
        }))
    }
}

/// Application-owned admission chain for every operation-frame request.
///
/// The component is retained across turns, but each call observes fresh time
/// and current frontiers. It owns no operation execution or recovery policy.
pub(in crate::resident) struct ResidentProgramCurrentRequestAdmission<
    'historical,
    PeerWitness,
    Evidence,
    Clock,
    Authority,
    Signature,
> {
    maximum_request_bytes: usize,
    policies: Box<[ResidentProgramRequestPolicy]>,
    evidence: Evidence,
    clock: Clock,
    historical: &'historical dyn crate::resident::GenerationLifecycleRecoveryResolver,
    authority: Authority,
    signature: Signature,
    peer: PhantomData<fn(&PeerWitness)>,
}

impl<'historical, PeerWitness, Evidence, Clock, Authority, Signature>
    ResidentProgramCurrentRequestAdmission<
        'historical,
        PeerWitness,
        Evidence,
        Clock,
        Authority,
        Signature,
    >
where
    Evidence: ResidentCurrentRequestEvidencePort<PeerWitness>,
    Clock: pale_ports::WallClockSourcePort,
    Authority: crate::resident::AmbientSessionAuthorityPort,
    Signature: ResidentProgramCallerSignaturePort,
{
    /// Retain exact request policies and physical/current authority owners.
    pub(in crate::resident) fn new(
        maximum_request_bytes: usize,
        policies: Vec<ResidentProgramRequestPolicy>,
        evidence: Evidence,
        clock: Clock,
        historical: &'historical dyn crate::resident::GenerationLifecycleRecoveryResolver,
        authority: Authority,
        signature: Signature,
    ) -> Result<Self, ResidentProgramCurrentRequestBuildError> {
        if maximum_request_bytes == 0 || policies.is_empty() {
            return Err(ResidentProgramCurrentRequestBuildError::Bounds);
        }
        for (index, entry) in policies.iter().enumerate() {
            if policies[..index].iter().any(|existing| {
                existing
                    .policy
                    .overlaps_fixed_admission_domain(&entry.policy)
            }) {
                return Err(ResidentProgramCurrentRequestBuildError::DuplicatePolicy);
            }
        }
        Ok(Self {
            maximum_request_bytes,
            policies: policies.into_boxed_slice(),
            evidence,
            clock,
            historical,
            authority,
            signature,
            peer: PhantomData,
        })
    }

    /// Read physical time for one application-owned background phase.
    ///
    /// This does not re-admit a request. It prevents a retained Consult from
    /// reusing the time captured by an earlier frame when it enters the task
    /// owner during a later resident work turn.
    fn observe_time(&mut self) -> Result<pale_core::records::UnixTimeV1, ()> {
        self.clock.observe_unix_time().map_err(|_| ())
    }

    /// Admit one peer-approved frame against retained policy and fresh evidence.
    pub(super) fn admit<WriterGuard>(
        &mut self,
        process: &ResidentProcessHost<'_, WriterGuard>,
        deployment_authorities: &[ResidentProgramDeploymentAuthority],
        peer: &PeerWitness,
        frame: &ResidentOperationFrameRequest,
    ) -> Result<ResidentProgramAdmittedRequest, ResidentOperationFrameFailure>
    where
        WriterGuard: PrincipalWriterGuardPort,
    {
        let (kind, locator) = operation_frame_kind_and_locator(frame);
        let target = operation_frame_target(kind, locator.as_ref())?;
        self.admit_bytes(
            process,
            deployment_authorities,
            peer,
            frame.signed_common_request().as_bytes(),
            ResidentProgramRequestPolicyKind::Operation(kind),
            target.as_ref(),
        )
    }

    /// Admit one peer-approved Observe request through the same current chain.
    fn admit_observe<WriterGuard>(
        &mut self,
        process: &ResidentProcessHost<'_, WriterGuard>,
        deployment_authorities: &[ResidentProgramDeploymentAuthority],
        peer: &PeerWitness,
        frame: &ResidentObserveFrameRequest,
    ) -> Result<ResidentProgramAdmittedRequest, ResidentObserveFrameFailure>
    where
        WriterGuard: PrincipalWriterGuardPort,
    {
        self.admit_bytes(
            process,
            deployment_authorities,
            peer,
            frame.signed_common_request().as_bytes(),
            ResidentProgramRequestPolicyKind::Observe,
            None,
        )
        .map_err(|failure| match failure {
            ResidentOperationFrameFailure::InvalidRequest => {
                ResidentObserveFrameFailure::InvalidRequest
            }
            ResidentOperationFrameFailure::AdmissionDenied => {
                ResidentObserveFrameFailure::AdmissionDenied
            }
            ResidentOperationFrameFailure::ReplayConflict => {
                ResidentObserveFrameFailure::ReplayConflict
            }
            ResidentOperationFrameFailure::ConfigurationUnavailable
            | ResidentOperationFrameFailure::ResidentUnavailable => {
                ResidentObserveFrameFailure::ResidentUnavailable
            }
        })
    }

    fn admit_bytes<WriterGuard>(
        &mut self,
        process: &ResidentProcessHost<'_, WriterGuard>,
        deployment_authorities: &[ResidentProgramDeploymentAuthority],
        peer: &PeerWitness,
        request_bytes: &[u8],
        kind: ResidentProgramRequestPolicyKind,
        target: Option<&OperationTargetReferenceV1>,
    ) -> Result<ResidentProgramAdmittedRequest, ResidentOperationFrameFailure>
    where
        WriterGuard: PrincipalWriterGuardPort,
    {
        let decoded = request_admission::decode_common_request_bytes(
            request_bytes,
            self.maximum_request_bytes,
        )
        .map_err(|_| ResidentOperationFrameFailure::InvalidRequest)?;
        let preimage = decoded.untrusted_preimage();
        if target.is_some_and(|target| {
            !request_admission::request_binds_operation_target(preimage, target)
        }) {
            return Err(ResidentOperationFrameFailure::InvalidRequest);
        }
        let policy = self
            .select_policy(kind, preimage, target)
            .ok_or(ResidentOperationFrameFailure::ConfigurationUnavailable)?;
        let selected =
            select_retained_request(process, deployment_authorities, preimage, self.historical)
                .map_err(|error| match error {
                    ResidentProgramRetainedRequestError::GenerationEvidence
                    | ResidentProgramRetainedRequestError::Contract(_) => {
                        ResidentOperationFrameFailure::ConfigurationUnavailable
                    }
                    ResidentProgramRetainedRequestError::Session(
                        crate::resident::application::ExactRetainedSessionRecoveryError::Recovery(
                            _,
                        ),
                    ) => ResidentOperationFrameFailure::ResidentUnavailable,
                    _ => ResidentOperationFrameFailure::AdmissionDenied,
                })?;
        if policy.operation != selected.operation.record.id {
            return Err(ResidentOperationFrameFailure::ConfigurationUnavailable);
        }
        let evidence = self
            .evidence
            .read_current_request_evidence(ResidentCurrentRequestSelector {
                peer_witness: peer,
                session_admission_id: &preimage.session_admission.session_admission_id,
                session_admission_lease_digest: &preimage
                    .session_admission
                    .session_admission_lease_digest,
                session_grant_id: &preimage.session_admission.session_grant_id,
                deployment_binding_ref: &preimage.session_admission.deployment_binding_ref,
                deployment_binding_sha256: &preimage.session_admission.deployment_binding_sha256,
            })
            .map_err(|_| ResidentOperationFrameFailure::AdmissionDenied)?;
        let now = self
            .clock
            .observe_unix_time()
            .map_err(|_| ResidentOperationFrameFailure::ResidentUnavailable)?;
        let session = selected
            .with_ambient_context_from(&evidence, now, |context| {
                selected.session.revalidate(&context, &mut self.authority)
            })
            .map_err(|_| ResidentOperationFrameFailure::AdmissionDenied)?;
        let session_admission = evidence
            .session_admission(&session, now)
            .map_err(|_| ResidentOperationFrameFailure::AdmissionDenied)?;
        let window = policy
            .admit_request_window(preimage, now)
            .map_err(|_| ResidentOperationFrameFailure::AdmissionDenied)?;
        let context = policy.admission_context(&session_admission, &window);
        let request = decoded
            .admit(&context, &mut |preimage, digest, signature| {
                self.signature
                    .verify_caller_signature(preimage, digest, signature)
            })
            .map_err(|error| match error {
                request_admission::ResidentCommonRequestAdmissionError::Admission(_) => {
                    ResidentOperationFrameFailure::AdmissionDenied
                }
                _ => ResidentOperationFrameFailure::InvalidRequest,
            })?;
        Ok(ResidentProgramAdmittedRequest {
            request,
            selected,
            evidence,
            session,
            now,
            policy,
            session_admission,
            window,
        })
    }

    fn select_policy(
        &self,
        kind: ResidentProgramRequestPolicyKind,
        claimed: &pale_core::records::CommonRequestEnvelopePreimageV1,
        target: Option<&OperationTargetReferenceV1>,
    ) -> Option<ResidentRetainedCommonRequestPolicy> {
        let mut selected = None;
        for entry in self.policies.iter().filter(|entry| entry.kind == kind) {
            let candidate = match target {
                Some(target) => entry.policy.with_operation_target(target.clone()),
                None => entry.policy.clone(),
            };
            if candidate.matches_fixed_claim(claimed) {
                if selected.is_some() {
                    return None;
                }
                selected = Some(candidate);
            }
        }
        selected
    }
}

/// Why retained current-request admission could not be constructed.
#[derive(Debug, thiserror::Error)]
pub(in crate::resident) enum ResidentProgramCurrentRequestBuildError {
    /// The byte ceiling or policy set is empty.
    #[error("current request admission requires a positive byte limit and policy set")]
    Bounds,
    /// Two frame kinds claim the same signed request-selection domain.
    #[error("current request policy selection is ambiguous")]
    DuplicatePolicy,
}

fn operation_frame_kind_and_locator(
    frame: &ResidentOperationFrameRequest,
) -> (
    operation_request_host::ResidentOperationRequestKind,
    Option<ResidentOperationFrameLocator>,
) {
    use operation_request_host::ResidentOperationRequestKind;
    match frame {
        ResidentOperationFrameRequest::StartInvoke { .. } => {
            (ResidentOperationRequestKind::StartInvoke, None)
        }
        ResidentOperationFrameRequest::AdvanceInvoke { locator, .. } => (
            ResidentOperationRequestKind::AdvanceInvoke,
            Some(locator.clone()),
        ),
        ResidentOperationFrameRequest::StartConsult { .. } => {
            (ResidentOperationRequestKind::StartConsult, None)
        }
        ResidentOperationFrameRequest::AdvanceConsult { locator, .. } => (
            ResidentOperationRequestKind::AdvanceConsult,
            Some(locator.clone()),
        ),
        ResidentOperationFrameRequest::StartDelegate { parent, .. } => (
            ResidentOperationRequestKind::StartDelegate,
            Some(parent.clone()),
        ),
        ResidentOperationFrameRequest::AdvanceDelegate { locator, .. } => (
            ResidentOperationRequestKind::AdvanceDelegate,
            Some(locator.clone()),
        ),
        ResidentOperationFrameRequest::Cancel { locator, .. } => {
            (ResidentOperationRequestKind::Cancel, Some(locator.clone()))
        }
    }
}

/// Project one point-addressed frame locator into its signed request coordinate.
///
/// Root starts have no existing target. Cancel keeps its existing operation-
/// handle request body and therefore does not acquire this additional request
/// coordinate.
fn operation_frame_target(
    kind: operation_request_host::ResidentOperationRequestKind,
    locator: Option<&ResidentOperationFrameLocator>,
) -> Result<Option<OperationTargetReferenceV1>, ResidentOperationFrameFailure> {
    if !matches!(
        kind,
        operation_request_host::ResidentOperationRequestKind::AdvanceInvoke
            | operation_request_host::ResidentOperationRequestKind::AdvanceConsult
            | operation_request_host::ResidentOperationRequestKind::StartDelegate
            | operation_request_host::ResidentOperationRequestKind::AdvanceDelegate
    ) {
        return Ok(None);
    }
    let locator = locator.ok_or(ResidentOperationFrameFailure::InvalidRequest)?;
    Ok(Some(OperationTargetReferenceV1 {
        operation_handle_ref: ResidentRecordRefV1::parse(
            locator.operation_handle_record_ref().to_owned(),
        )
        .map_err(|_| ResidentOperationFrameFailure::InvalidRequest)?,
        operation_handle_sha256: OperationHandleDigest::parse(
            locator.operation_handle_sha256().to_owned(),
        )
        .map_err(|_| ResidentOperationFrameFailure::InvalidRequest)?,
    }))
}

/// Physical signing and checkpoint construction for one durable Invoke start.
///
/// The application owner selects every semantic input before lending this port.
/// Implementations can sign or construct bytes; they cannot select a request,
/// session, generation, operation, policy, store, or recovery route.
pub(in crate::resident) trait ResidentProgramInvokeCryptographyPort {
    /// Admit the compiled operation against the exact retained authority.
    #[allow(clippy::too_many_arguments)]
    fn admit_compiled_authority(
        &self,
        request: &CommonRequestEnvelopeV1,
        session: &crate::resident::AdmittedAmbientSession,
        current: &crate::resident::AmbientSessionAdmissionContext<'_>,
        handle: &pale_core::records::OperationHandlePreimageV1,
        lease: &pale_core::records::OperationLeaseV1,
        operation: &crate::semantic::ResolvedOperation,
        publication: &crate::resident::LocalOperationPublicationPolicyInput<'_>,
        checkpoint: &pale_core::records::ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    ) -> Result<(), InvalidResidentRecordV1>;

    /// Sign the exact application-constructed handle.
    fn sign_handle(
        &self,
        bytes: &[u8],
        preimage: &pale_core::records::OperationHandlePreimageV1,
        digest: &pale_core::records::OperationHandleDigest,
    ) -> Result<pale_core::records::CanonicalSignatureV1, InvalidResidentRecordV1>;

    /// Sign the exact application-constructed initial observation.
    fn sign_observation(
        &self,
        bytes: &[u8],
        preimage: &pale_core::records::OperationObservationPreimageV1,
        digest: &pale_core::records::OperationObservationDigest,
    ) -> Result<pale_core::records::CanonicalSignatureV1, InvalidResidentRecordV1>;

    /// Construct the exact checkpoint preimage for the prepared start.
    fn prepare_checkpoint(
        &self,
        prepared: &crate::resident::PreparedLocalOperationCheckpoint,
        checkpoint_id: &pale_core::records::ControlCheckpointIdV1,
    ) -> Result<
        pale_core::records::ControlStateCheckpointPreimageV1,
        crate::resident::LocalOperationStartError,
    >;

    /// Sign the exact application-constructed control checkpoint.
    fn sign_checkpoint(
        &self,
        bytes: &[u8],
        preimage: &pale_core::records::ControlStateCheckpointPreimageV1,
        digest: &pale_core::records::ControlStateCheckpointDigest,
    ) -> Result<pale_core::records::CanonicalSignatureV1, InvalidResidentRecordV1>;
}

/// Exact store-bound operation ports retained for one admitted deployment.
///
/// The selector is compared before any port is lent. An owner with multiple
/// deployments cannot silently use the first or only store as a fallback.
pub(in crate::resident) struct ResidentProgramDeploymentOperationPorts<'ports> {
    selector: ResidentDeploymentAuthoritySelector,
    store: pale_ports::AuthorityStoreCoordinate,
    custody: &'ports mut dyn OperationCustodyPort,
    sessions: &'ports dyn SessionLeaseCustodyPort,
    anchor: &'ports mut dyn AntiRollbackAnchorPort,
}

impl<'ports> ResidentProgramDeploymentOperationPorts<'ports> {
    /// Bind caller-opened physical ports to one already admitted selector.
    pub(in crate::resident) fn new(
        selector: ResidentDeploymentAuthoritySelector,
        store: pale_ports::AuthorityStoreCoordinate,
        custody: &'ports mut dyn OperationCustodyPort,
        sessions: &'ports dyn SessionLeaseCustodyPort,
        anchor: &'ports mut dyn AntiRollbackAnchorPort,
    ) -> Self {
        Self {
            selector,
            store,
            custody,
            sessions,
            anchor,
        }
    }

    fn matches(&self, selector: &ResidentDeploymentAuthoritySelector) -> bool {
        self.selector == *selector
    }

    fn access(&mut self) -> ResidentDeploymentOperationAccess<'_> {
        ResidentDeploymentOperationAccess {
            store: &self.store,
            custody: &mut *self.custody,
            sessions: self.sessions,
            anchor: &mut *self.anchor,
        }
    }
}

/// Lend exact protected-recovery inputs for one already selected root request.
///
/// Implementations provide only physical readers, historical admission owners,
/// and fixed bounds. The application still performs the lookup, classification,
/// policy selection, and no-replay decision through existing process-host code.
pub(in crate::resident) trait ResidentProgramRootRecoveryInputPort {
    fn with_native_hook_response_inputs<Output>(
        &mut self,
        _selector: &ResidentDeploymentAuthoritySelector,
        _request: &CommonRequestEnvelopeV1,
        _use_inputs: impl for<'a> FnOnce(
            crate::resident::operation_dispatch::NativeHookResponseInputs<'a>,
        ) -> Output,
    ) -> Result<Output, ResidentOperationFrameFailure> {
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    /// Report whether one absent request may enter the fresh-start path.
    ///
    /// Durable duplicate lookup runs before this admission is enforced. A full
    /// retained-phase set therefore cannot hide an exact existing operation.
    fn fresh_start_admission(&self) -> ResidentProcessFreshStartAdmission {
        ResidentProcessFreshStartAdmission::Available
    }

    fn with_root_recovery<Output, UseInputs>(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        request: &CommonRequestEnvelopeV1,
        locator: Option<&operation_locator::ResidentProgramOperationLocator>,
        use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'restart, 'admission, 'checkpoint> FnOnce(
            ResidentProcessOperationRestartInput<'restart>,
            OperationControlRecoveryAdmission<'admission>,
            &'checkpoint pale_core::records::ControlCheckpointSigningAuthorityAdmissionContextV1<
                'checkpoint,
            >,
            &mut Self,
        ) -> Output;

    /// Lend one bounded recovery sweep its physical and historical inputs.
    ///
    /// The application already selected the deployment and semantic source.
    /// Implementations provide no cursor, route, or outcome decision.
    fn with_background_recovery<UseInputs>(
        &mut self,
        _selector: &ResidentDeploymentAuthoritySelector,
        _use_inputs: UseInputs,
    ) -> Result<(), ResidentProgramTurnError>
    where
        UseInputs: for<'restart, 'admission> FnOnce(
            ResidentProcessOperationRestartInput<'restart>,
            OperationControlRecoveryAdmission<'admission>,
            usize,
            pale_core::records::UnixTimeV1,
            &mut Self,
        )
            -> Result<(), ResidentProgramTurnError>,
    {
        Err(ResidentProgramTurnError::RecoveryUnavailable)
    }

    /// Advance one publication stage for an already selected deployment.
    ///
    /// The physical input owner lends current Event authority and custody. The
    /// existing process publication owner keeps the scan cursor and performs the
    /// bounded semantic transition. An unavailable owner leaves that cursor
    /// unchanged.
    fn recover_publication<WriterGuard>(
        &mut self,
        _process: &mut ResidentProcessHost<'_, WriterGuard>,
        _selector: &ResidentDeploymentAuthoritySelector,
        _access: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<(), ResidentProgramTurnError>
    where
        WriterGuard: PrincipalWriterGuardPort,
    {
        Err(ResidentProgramTurnError::RecoveryUnavailable)
    }

    /// Retain a native or A2A disposition that has no configured background
    /// owner. Its discovery cursor remains fixed until an exact owner accepts it.
    fn retain_deferred_recovery(
        &mut self,
        disposition: Box<restart::ResidentProcessOperationRestartDisposition>,
    );

    /// Whether one retained root Invoke phase has a configured continuation owner.
    fn has_runnable_retained_invoke(&self) -> bool {
        false
    }

    /// Select the deployment required by the next runnable root Invoke phase.
    fn retained_invoke_selector(&self) -> Option<&ResidentDeploymentAuthoritySelector> {
        None
    }

    /// Advance at most one runnable root Invoke phase through its existing owner.
    fn continue_one_retained_invoke<WriterGuard, Authority>(
        &mut self,
        _process: &mut ResidentProcessHost<'_, WriterGuard>,
        _access: ResidentDeploymentOperationAccess<'_>,
        _authority: &mut Authority,
    ) -> ResidentProgramRetainedPhaseTurn
    where
        WriterGuard: PrincipalWriterGuardPort,
        Authority: crate::resident::AmbientSessionAuthorityPort,
    {
        ResidentProgramRetainedPhaseTurn::Idle
    }

    /// Whether one retained A2A Delegate phase has a safe continuation owner.
    fn has_runnable_retained_delegate(&self) -> bool {
        false
    }

    /// Select the deployment required by the next runnable A2A Delegate phase.
    fn retained_delegate_selector(&self) -> Option<&ResidentDeploymentAuthoritySelector> {
        None
    }

    /// Advance at most one retained A2A Delegate phase without replaying it.
    fn continue_one_retained_delegate<WriterGuard, Authority>(
        &mut self,
        _process: &mut ResidentProcessHost<'_, WriterGuard>,
        _access: ResidentDeploymentOperationAccess<'_>,
        _authority: &mut Authority,
    ) -> ResidentProgramRetainedPhaseTurn
    where
        WriterGuard: PrincipalWriterGuardPort,
        Authority: crate::resident::AmbientSessionAuthorityPort,
    {
        ResidentProgramRetainedPhaseTurn::Idle
    }
}

/// Exact application-owned continuation retained after one Consult turn.
///
/// A Consult start, point-addressed Advance, or recovery turn creates this value
/// only after an existing Consult owner selects the next phase. The physical
/// input owner retains it so a response never discards unfinished work.
#[must_use = "retain the exact Consult turn until its existing owner continues it"]
pub(in crate::resident) enum ResidentProgramConsultTurn<
    Work,
    Output,
    ChildRecoveryError,
    RetentionFailure,
    StopInput,
> {
    /// The durable Consult start entered the bounded task owner but has not yet
    /// run its first mechanical pre-policy turn.
    StartReady {
        ready: Box<consult_start::ResidentProcessConsultationReady>,
    },
    /// Mechanical recovery stopped before a Package decision was applied.
    PrePolicy(consult_recovery::ResidentProcessConsultRecoveryTurn<Work, Output>),
    /// The retained Package policy was evaluated and applied exactly once.
    Policy(
        Box<
            consult_recovery::ResidentProcessConsultationPolicyContinuation<
                Work,
                Output,
                ChildRecoveryError,
                RetentionFailure,
                StopInput,
            >,
        >,
    ),
    /// Package policy was evaluated, but the exact physical continuation inputs
    /// were unavailable before the decision could be applied.
    PolicyReady {
        handoff: Box<consult_recovery::ResidentProcessConsultationPolicyHandoff>,
        decision: crate::resident::consultation::ResidentConsultationSupervisionDecision<StopInput>,
    },
    /// The exact handoff and selected decision failed an association check.
    PolicyRefusal(Box<consult_recovery::ResidentProcessConsultationPolicyRefusal<StopInput>>),
    /// An executor completion was polled, but could not bind to its deployment.
    CompletionBinding(
        Box<consult_completion::ResidentProcessConsultationCompletionBindingFailure<Work, Output>>,
    ),
    /// The selected Package policy was unavailable; the exact handoff is unchanged.
    PolicyUnavailable(Box<consult_recovery::ResidentProcessConsultationPolicyHandoff>),
}

/// Exact process-local identity of one retained Consult parent.
///
/// The deployment selector prevents a handle coordinate from crossing an
/// admitted runtime boundary. The coordinate is the already-issued parent
/// handle; it grants no authority to recover or mutate that parent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::resident) struct ResidentProgramConsultTurnKey {
    selector: ResidentDeploymentAuthoritySelector,
    parent: pale_ports::ControlRecordCoordinate,
}

impl ResidentProgramConsultTurnKey {
    pub(in crate::resident) fn new(
        selector: ResidentDeploymentAuthoritySelector,
        parent: pale_ports::ControlRecordCoordinate,
    ) -> Self {
        Self { selector, parent }
    }

    /// Borrow the admitted deployment selected for this exact parent.
    pub(in crate::resident) const fn selector(&self) -> &ResidentDeploymentAuthoritySelector {
        &self.selector
    }

    /// Borrow the already-issued parent-handle coordinate.
    pub(in crate::resident) const fn parent(&self) -> &pale_ports::ControlRecordCoordinate {
        &self.parent
    }
}

fn consult_turn_key_from_started(
    started: &consult_start::ResidentProcessConsultationStarted,
) -> Result<ResidentProgramConsultTurnKey, ()> {
    let parent = operation_locator::ResidentProgramOperationLocator::from_issued(started.parent())
        .map_err(|_| ())?;
    Ok(ResidentProgramConsultTurnKey::new(
        started.selector().clone(),
        parent.coordinate().clone(),
    ))
}

fn consult_turn_key_from_recovered(
    recovered: &restart::ResidentProcessRecoveredOperationStart,
) -> Result<ResidentProgramConsultTurnKey, ()> {
    let parent = operation_locator::ResidentProgramOperationLocator::from_issued(
        recovered.control.operation().operation(),
    )
    .map_err(|_| ())?;
    Ok(ResidentProgramConsultTurnKey::new(
        recovered.selector.clone(),
        parent.coordinate().clone(),
    ))
}

fn consult_turn_key_from_reference(
    selector: ResidentDeploymentAuthoritySelector,
    parent: &pale_core::records::ImmutableRecordReferenceV1,
) -> Result<ResidentProgramConsultTurnKey, ()> {
    let digest = pale_ports::ControlRecordDigest::parse(parent.record_sha256.as_str().to_owned())
        .map_err(|_| ())?;
    Ok(ResidentProgramConsultTurnKey::new(
        selector,
        pale_ports::ControlRecordCoordinate::new(parent.record_ref.clone(), digest),
    ))
}

/// One retained Consult turn paired with its exact parent identity.
#[must_use = "continue or re-retain this exact Consult turn"]
pub(in crate::resident) struct ResidentProgramKeyedConsultTurn<Turn> {
    key: ResidentProgramConsultTurnKey,
    turn: Turn,
}

/// Unfinished Consult stop custody paired with its exact parent identity.
pub(in crate::resident) type ResidentProgramKeyedConsultStopCustody =
    ResidentProgramKeyedConsultTurn<Box<ResidentProcessConsultationStopCustody>>;

impl<Turn> ResidentProgramKeyedConsultTurn<Turn> {
    /// Pair one continuation with its exact deployment and parent identity.
    pub(in crate::resident) fn new(key: ResidentProgramConsultTurnKey, turn: Turn) -> Self {
        Self { key, turn }
    }

    /// Borrow the exact deployment and parent key.
    pub(in crate::resident) const fn key(&self) -> &ResidentProgramConsultTurnKey {
        &self.key
    }

    /// Consume the process-local carrier without separating its identity.
    pub(in crate::resident) fn into_parts(self) -> (ResidentProgramConsultTurnKey, Turn) {
        (self.key, self.turn)
    }
}

/// Why a bounded Consult continuation owner refused one exact turn.
#[must_use = "the refused turn remains the caller's sole continuation"]
pub(in crate::resident) enum ResidentProgramConsultRetentionFailure<Turn> {
    /// This exact parent already has a retained continuation.
    Duplicate(Box<ResidentProgramKeyedConsultTurn<Turn>>),
    /// The configured process-local continuation ceiling is full.
    Saturated(Box<ResidentProgramKeyedConsultTurn<Turn>>),
}

impl<Turn> ResidentProgramConsultRetentionFailure<Turn> {
    /// Recover the unchanged parent key and continuation.
    pub(in crate::resident) fn into_retained(self) -> ResidentProgramKeyedConsultTurn<Turn> {
        match self {
            Self::Duplicate(retained) | Self::Saturated(retained) => *retained,
        }
    }
}

/// Bounded FIFO custody for unfinished Consult turns.
///
/// This owner contains process-local continuations only. It does not persist
/// meaning, allocate an identity, choose policy, or imply that a retained turn
/// is currently runnable. A rejected insertion returns the exact turn.
pub(in crate::resident) struct ResidentProgramRetainedConsultTurns<Turn> {
    maximum_turns: NonZeroUsize,
    turns: VecDeque<ResidentProgramKeyedConsultTurn<Turn>>,
}

impl<Turn> ResidentProgramRetainedConsultTurns<Turn> {
    /// Create one explicit bounded owner. No default capacity is inferred.
    pub(in crate::resident) fn new(maximum_turns: NonZeroUsize) -> Self {
        Self {
            maximum_turns,
            turns: VecDeque::new(),
        }
    }

    /// Retain one exact parent continuation without replacing an existing turn.
    pub(in crate::resident) fn retain(
        &mut self,
        key: ResidentProgramConsultTurnKey,
        turn: Turn,
    ) -> Result<(), ResidentProgramConsultRetentionFailure<Turn>> {
        let retained = ResidentProgramKeyedConsultTurn { key, turn };
        if self.turns.iter().any(|current| current.key == retained.key) {
            return Err(ResidentProgramConsultRetentionFailure::Duplicate(Box::new(
                retained,
            )));
        }
        if self.turns.len() == self.maximum_turns.get() {
            return Err(ResidentProgramConsultRetentionFailure::Saturated(Box::new(
                retained,
            )));
        }
        self.turns.push_back(retained);
        Ok(())
    }

    /// Whether at least one exact continuation is scheduler-visible.
    pub(in crate::resident) fn has_retained(&self) -> bool {
        !self.turns.is_empty()
    }

    /// Return whether this owner can accept another distinct parent.
    #[must_use]
    pub(in crate::resident) fn has_capacity(&self) -> bool {
        self.turns.len() < self.maximum_turns.get()
    }

    /// Return whether the exact parent already has a retained continuation.
    #[must_use]
    pub(in crate::resident) fn contains(&self, key: &ResidentProgramConsultTurnKey) -> bool {
        self.turns.iter().any(|retained| retained.key() == key)
    }

    /// Copy the bounded parent-key set for one exclusive recovery admission.
    pub(in crate::resident) fn keys(&self) -> Vec<ResidentProgramConsultTurnKey> {
        self.turns
            .iter()
            .map(|retained| retained.key().clone())
            .collect()
    }

    /// Borrow the deployment selected by the next retained turn.
    pub(in crate::resident) fn next_selector(
        &self,
    ) -> Option<&ResidentDeploymentAuthoritySelector> {
        self.turns.front().map(|retained| retained.key.selector())
    }

    /// Take at most one retained turn in stable insertion order.
    pub(in crate::resident) fn take_next(
        &mut self,
    ) -> Option<ResidentProgramKeyedConsultTurn<Turn>> {
        self.turns.pop_front()
    }
}

/// Immutable view of keyed Consult capacity during one exclusive owner loan.
///
/// The physical input owner cannot change while it is mutably lent to the
/// recovery turn. This bounded view therefore admits the exact parent before
/// any executor poll or policy action without creating a second custody owner.
pub(in crate::resident) struct ResidentProgramConsultRetentionAdmission {
    has_capacity: bool,
    retained_keys: Vec<ResidentProgramConsultTurnKey>,
}

impl ResidentProgramConsultRetentionAdmission {
    /// Create one bounded view while its source is exclusively borrowed.
    pub(in crate::resident) fn new(
        has_capacity: bool,
        retained_keys: Vec<ResidentProgramConsultTurnKey>,
    ) -> Self {
        Self {
            has_capacity,
            retained_keys,
        }
    }

    /// Return whether this exclusive turn may create one continuation for `key`.
    #[must_use]
    pub(in crate::resident) fn admits(&self, key: &ResidentProgramConsultTurnKey) -> bool {
        self.has_capacity && !self.retained_keys.iter().any(|current| current == key)
    }
}

impl<Work, Output, ChildRecoveryError, RetentionFailure, StopInput>
    ResidentProgramConsultTurn<Work, Output, ChildRecoveryError, RetentionFailure, StopInput>
{
    fn transfers_recovery_source(&self) -> bool {
        !matches!(
            self,
            Self::PrePolicy(consult_recovery::ResidentProcessConsultRecoveryTurn::NeedsRecovery(_))
                | Self::CompletionBinding(_)
                | Self::PolicyUnavailable(_)
        )
    }
}

/// Existing semantic fields selected by a Consult completion policy.
///
/// This value contains no parent, signer, record coordinate, store, checkpoint,
/// or custody authority. The physical lender supplies those values only after
/// the exact retained parent is selected again.
pub(in crate::resident) struct ResidentProgramConsultStopMeaning {
    pub(in crate::resident) cause: crate::resident::consultation::ConsultationStopCause,
    pub(in crate::resident) reason: pale_core::records::ImmutableRecordReferenceV1,
    pub(in crate::resident) stopped_at: pale_core::records::UnixTimeV1,
}

/// Project an application-selected stop into the existing stop vocabulary.
///
/// Implementations cannot supply physical inputs. Returning `None` keeps the
/// exact policy turn retained and fails closed.
pub(in crate::resident) trait ResidentProgramConsultStopInput {
    fn stop_meaning(&self) -> Option<ResidentProgramConsultStopMeaning>;
}

impl ResidentProgramConsultStopInput for () {
    fn stop_meaning(&self) -> Option<ResidentProgramConsultStopMeaning> {
        None
    }
}

/// Fresh physical inputs for one exact Package-selected Consult stop.
///
/// The semantic cause, evidence, and stop time are deliberately absent. The
/// program reads them from [`ResidentProgramConsultStopInput`] and combines
/// both halves immediately before it calls the existing stop owner.
pub(in crate::resident) struct ResidentProgramConsultStopInputs<'a> {
    pub(in crate::resident) association: ResidentOperationControlAssociation<'a>,
    pub(in crate::resident) parent: &'a crate::resident::IssuedLocalOperation,
    pub(in crate::resident) parent_observation: (
        &'a pale_core::records::ResidentRecordRefV1,
        &'a pale_core::records::OperationObservationV1,
    ),
    pub(in crate::resident) parent_checkpoint: (
        &'a pale_core::records::ResidentRecordRefV1,
        &'a pale_core::records::ControlStateCheckpointV1,
    ),
    pub(in crate::resident) observation_ref: pale_core::records::ResidentRecordRefV1,
    pub(in crate::resident) observed_at: pale_core::records::UnixTimeV1,
    pub(in crate::resident) signing: pale_core::records::SignedResidentAdmissionContextV1<'a>,
    pub(in crate::resident) limits: crate::resident::consultation::ConsultationCollectionLimits,
    pub(in crate::resident) checkpoint:
        crate::resident::consultation::ConsultationSuccessorCommitInput<'a>,
    pub(in crate::resident) resolve_store:
        &'a dyn crate::resident::OperationObservationAuthorityStoreResolver,
    pub(in crate::resident) admit_stop: &'a mut ResidentProgramConsultStopAdmission<'a>,
    pub(in crate::resident) admit_collection: &'a mut (dyn FnMut(
        &crate::resident::consultation::ConsultationCollection,
    ) -> Result<(), InvalidResidentRecordV1>
                 + 'a),
    pub(in crate::resident) admit_rotation: &'a mut ResidentProgramRotationAdmission<'a>,
    pub(in crate::resident) sign_observation: &'a mut ResidentProgramObservationSigner<'a>,
    pub(in crate::resident) sign_checkpoint: &'a mut ResidentProgramCheckpointSigner<'a>,
}

type ResidentProgramConsultStopAdmission<'a> = dyn FnMut(
        &crate::resident::consultation::ConsultationCollection,
        &crate::resident::consultation::ConsultationRequirementSummary,
    ) -> Result<(), InvalidResidentRecordV1>
    + 'a;

/// Fresh physical inputs for one exact executor completion join.
///
/// The retained completion still owns its work and output. These values contain
/// only the protected child read, new parent successor coordinates, admitted
/// signers, validators, and bounds required by the existing terminal owner.
pub(in crate::resident) struct ResidentProgramConsultJoinInputs<'a> {
    pub(in crate::resident) association: ResidentOperationControlAssociation<'a>,
    pub(in crate::resident) input: crate::resident::consultation::ConsultationChildJoinInput<'a>,
    pub(in crate::resident) checkpoint:
        crate::resident::consultation::ConsultationSuccessorCommitInput<'a>,
    pub(in crate::resident) resolve_store:
        &'a dyn crate::resident::OperationObservationAuthorityStoreResolver,
    pub(in crate::resident) admit_evidence:
        &'a mut ResidentProgramConsultChildEvidenceAdmission<'a>,
    pub(in crate::resident) admit_collection: &'a mut (dyn FnMut(
        &crate::resident::consultation::ConsultationCollection,
    ) -> Result<(), InvalidResidentRecordV1>
                 + 'a),
    pub(in crate::resident) admit_rotation: &'a mut ResidentProgramRotationAdmission<'a>,
    pub(in crate::resident) sign_observation: &'a mut ResidentProgramObservationSigner<'a>,
    pub(in crate::resident) sign_checkpoint: &'a mut ResidentProgramCheckpointSigner<'a>,
}

/// Immutable coordinates needed to lend terminal-join physical inputs.
///
/// This selection is copied from the retained task owner. It contains no work,
/// output, recovered child body, signer, or custody authority.
#[derive(Clone)]
pub(in crate::resident) struct ResidentProgramConsultJoinSelection {
    pub(in crate::resident) parent: ResidentProgramConsultTurnKey,
    pub(in crate::resident) slot_id: crate::resident::consultation::ConsultationSlotId,
    pub(in crate::resident) child: pale_core::records::ImmutableRecordReferenceV1,
    pub(in crate::resident) dispatch: pale_core::records::ImmutableRecordReferenceV1,
}

/// Physical inputs for one exact terminal Consult-child join.
///
/// The application has already recovered and classified the child. These
/// values supply only fresh record coordinates, admitted signers, bounded
/// custody, and record validators required by the existing join owner.
pub(in crate::resident) struct ResidentProgramConsultChildTerminalInputs<'a> {
    pub(in crate::resident) input:
        consult_child_recovery_sweep::ResidentProcessConsultChildTerminalCommitInput<'a>,
    pub(in crate::resident) resolve_store:
        &'a dyn crate::resident::OperationObservationAuthorityStoreResolver,
    pub(in crate::resident) admit_evidence:
        &'a mut ResidentProgramConsultChildEvidenceAdmission<'a>,
    pub(in crate::resident) admit_collection: &'a mut (dyn FnMut(
        &crate::resident::consultation::ConsultationCollection,
    ) -> Result<(), InvalidResidentRecordV1>
                 + 'a),
    pub(in crate::resident) admit_rotation: &'a mut ResidentProgramRotationAdmission<'a>,
    pub(in crate::resident) sign_observation: &'a mut ResidentProgramObservationSigner<'a>,
    pub(in crate::resident) sign_checkpoint: &'a mut ResidentProgramCheckpointSigner<'a>,
}

type ResidentProgramConsultChildEvidenceAdmission<'a> = dyn FnMut(
        &crate::resident::consultation::ConsultationRosterSlot,
        &pale_core::records::ImmutableRecordReferenceV1,
    ) -> Result<(), InvalidResidentRecordV1>
    + 'a;

/// Fresh physical inputs for the first durable Consult collection.
///
/// The retained parent intent owns the roster and completion meaning. These
/// inputs supply only new record coordinates, admitted signing contexts,
/// physical validators, and bounded checkpoint preparation for that exact
/// collection. Historical recovery inputs cannot substitute for them.
pub(in crate::resident) struct ResidentProgramConsultInitialCollectionInputs<'a> {
    pub(in crate::resident) observation_ref: pale_core::records::ResidentRecordRefV1,
    pub(in crate::resident) observed_at: pale_core::records::UnixTimeV1,
    pub(in crate::resident) signing: pale_core::records::SignedResidentAdmissionContextV1<'a>,
    pub(in crate::resident) checkpoint:
        crate::resident::consultation::ConsultationSuccessorCommitInput<'a>,
    pub(in crate::resident) resolve_store:
        &'a dyn crate::resident::OperationObservationAuthorityStoreResolver,
    pub(in crate::resident) admit_collection: &'a mut (dyn FnMut(
        &crate::resident::consultation::ConsultationCollection,
    ) -> Result<(), InvalidResidentRecordV1>
                 + 'a),
    pub(in crate::resident) admit_rotation: &'a mut ResidentProgramRotationAdmission<'a>,
    pub(in crate::resident) sign_observation: &'a mut ResidentProgramObservationSigner<'a>,
    pub(in crate::resident) sign_checkpoint: &'a mut ResidentProgramCheckpointSigner<'a>,
}

/// The turn retained by one concrete Consult input owner.
pub(in crate::resident) type ResidentProgramConsultPortTurn<Port, StopInput> =
    ResidentProgramConsultTurn<
        <Port as ResidentProgramConsultRecoveryInputPort<StopInput>>::Work,
        <Port as ResidentProgramConsultRecoveryInputPort<StopInput>>::Output,
        <Port as ResidentProgramConsultRecoveryInputPort<StopInput>>::ChildRecoveryError,
        <Port as ResidentProgramConsultRecoveryInputPort<StopInput>>::RetentionFailure,
        StopInput,
    >;

/// Physical and process-local inputs for one point-addressed Consult turn.
///
/// This port does not select a request, deployment, operation, collection,
/// policy, or outcome. It lends the already-selected resolver, validators,
/// executor, and child-custody callbacks to the application-owned route and
/// retains the exact linear result that route produces.
pub(in crate::resident) trait ResidentProgramConsultRecoveryInputPort<StopInput> {
    type Work;
    type Output;
    type ChildRecoveryError;
    type RetentionFailure;
    type Executor: crate::resident::task_owner::ResidentOperationTaskExecutor<
        Self::Work,
        Self::Output,
    >;
    type StoreResolver: crate::resident::OperationObservationAuthorityStoreResolver;
    type AdmitCollection: FnMut(
        &crate::resident::consultation::ConsultationCollection,
    ) -> Result<(), InvalidResidentRecordV1>;
    type RecoverChild: FnMut(
        &crate::resident::consultation::ConsultationChildDispatchCandidate,
    ) -> Result<
        crate::resident::RecoveredOperationObservation,
        Self::ChildRecoveryError,
    >;
    type RetainChild: FnMut(
        &crate::resident::consultation::ConsultationChildDispatchCandidate,
        crate::resident::RecoveredOperationObservation,
        &mut dyn OperationCustodyPort,
        &mut dyn AntiRollbackAnchorPort,
    )
        -> crate::resident::consultation::ConsultationChildRetentionOutcome<
        Self::Work,
        Self::RetentionFailure,
    >;

    /// Lend the bounded physical inputs without selecting Consult semantics.
    fn with_consult_inputs<Return, UseInputs>(
        &mut self,
        use_inputs: UseInputs,
    ) -> Result<Return, ResidentOperationFrameFailure>
    where
        UseInputs: FnOnce(
            crate::resident::consultation::ConsultationCollectionLimits,
            crate::resident::consultation::ResidentConsultationTaskLimits,
            &Self::StoreResolver,
            &mut Self::AdmitCollection,
            &mut Self::Executor,
            &mut Self::RecoverChild,
            &mut Self::RetainChild,
            Option<&mut crate::resident::HistoricalConsultationAdmission<'_>>,
        ) -> Return;

    /// Use the lent inputs without allowing a post-callback error to discard the result.
    ///
    /// The callback can produce a linear Consult continuation. Once it runs, its
    /// result takes precedence over an error returned afterward by the physical
    /// input lender. This is the same custody rule used by retained provider
    /// recovery.
    fn use_consult_inputs<Return, UseInputs>(
        &mut self,
        use_inputs: UseInputs,
    ) -> Result<Return, ResidentOperationFrameFailure>
    where
        UseInputs: FnOnce(
            crate::resident::consultation::ConsultationCollectionLimits,
            crate::resident::consultation::ResidentConsultationTaskLimits,
            &Self::StoreResolver,
            &mut Self::AdmitCollection,
            &mut Self::Executor,
            &mut Self::RecoverChild,
            &mut Self::RetainChild,
            Option<&mut crate::resident::HistoricalConsultationAdmission<'_>>,
        ) -> Return,
    {
        let mut output = None;
        let lender = self.with_consult_inputs(
            |collection_limits,
             task_limits,
             resolve_store,
             admit_collection,
             executor,
             recover_child,
             retain_child,
             historical_collection| {
                output = Some(use_inputs(
                    collection_limits,
                    task_limits,
                    resolve_store,
                    admit_collection,
                    executor,
                    recover_child,
                    retain_child,
                    historical_collection,
                ));
            },
        );
        match output {
            Some(output) => Ok(output),
            None => Err(lender
                .err()
                .unwrap_or(ResidentOperationFrameFailure::ConfigurationUnavailable)),
        }
    }

    /// Lend fresh physical inputs for one exact retained initial collection.
    ///
    /// The default remains unavailable. An implementation must bind the fresh
    /// coordinates and signers to the already-selected deployment and parent;
    /// it cannot select roster meaning or replace the retained child prefix.
    fn with_initial_collection_inputs<Return, UseInputs>(
        &mut self,
        _selector: &ResidentDeploymentAuthoritySelector,
        _parent: &pale_ports::ControlRecordCoordinate,
        _use_inputs: UseInputs,
    ) -> Result<Return, ResidentOperationFrameFailure>
    where
        UseInputs:
            for<'input> FnOnce(ResidentProgramConsultInitialCollectionInputs<'input>) -> Return,
    {
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    /// Use fresh collection inputs without losing a linear callback result.
    fn use_initial_collection_inputs<Return, UseInputs>(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        parent: &pale_ports::ControlRecordCoordinate,
        use_inputs: UseInputs,
    ) -> Result<Return, ResidentOperationFrameFailure>
    where
        UseInputs:
            for<'input> FnOnce(ResidentProgramConsultInitialCollectionInputs<'input>) -> Return,
    {
        let mut output = None;
        let lender = self.with_initial_collection_inputs(selector, parent, |inputs| {
            output = Some(use_inputs(inputs));
        });
        match output {
            Some(output) => Ok(output),
            None => Err(lender
                .err()
                .unwrap_or(ResidentOperationFrameFailure::ConfigurationUnavailable)),
        }
    }

    /// Lend fresh physical inputs for one exact Package-selected stop.
    ///
    /// The key and semantic stop meaning are already fixed. Implementations may
    /// recover only that parent and must not replace the cause, reason, or time.
    fn with_consult_stop_inputs<Return, UseInputs>(
        &mut self,
        _key: &ResidentProgramConsultTurnKey,
        _meaning: &ResidentProgramConsultStopMeaning,
        _use_inputs: UseInputs,
    ) -> Result<Return, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(ResidentProgramConsultStopInputs<'input>) -> Return,
    {
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    /// Lend fresh physical inputs for one exact executor completion.
    ///
    /// The selection is derived from the retained completion. The lender may
    /// recover only that child and parent; it cannot poll an executor or choose
    /// another roster member.
    fn with_consult_join_inputs<Return, UseInputs>(
        &mut self,
        _selection: &ResidentProgramConsultJoinSelection,
        _use_inputs: UseInputs,
    ) -> Result<Return, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(ResidentProgramConsultJoinInputs<'input>) -> Return,
    {
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    /// Take an unfinished stop database/anchor continuation before any re-admission.
    fn take_consult_stop_custody(&mut self) -> Option<ResidentProgramKeyedConsultStopCustody> {
        None
    }

    /// Re-retain the exact unfinished stop custody phase.
    #[allow(clippy::boxed_local)]
    fn retain_consult_stop_custody(&mut self, _pending: ResidentProgramKeyedConsultStopCustody) {
        unreachable!("stop custody cannot exist without stop input support")
    }

    /// Take one unfinished terminal join, custody, or reservation-release phase.
    fn take_consult_terminal_failure(
        &mut self,
    ) -> Option<
        ResidentProgramKeyedConsultTurn<
            ResidentProcessConsultationTerminalJoinFailure<Self::Work, Self::Output>,
        >,
    > {
        None
    }

    /// Re-retain one exact terminal join continuation without polling again.
    fn retain_consult_terminal_failure(
        &mut self,
        _failure: ResidentProgramKeyedConsultTurn<
            ResidentProcessConsultationTerminalJoinFailure<Self::Work, Self::Output>,
        >,
    ) {
        unreachable!("terminal join failure cannot exist without join input support")
    }

    /// Retain one exact unfinished application phase before responding.
    fn retain_consult_turn(&mut self, turn: ResidentProgramConsultPortTurn<Self, StopInput>);

    /// Retain one exact parent-keyed continuation.
    ///
    /// A focused input owner can use its single-turn custody. A scheduler-owned
    /// implementation must return the unchanged key and turn when its explicit
    /// bound is full or the parent already has a continuation.
    fn retain_keyed_consult_turn(
        &mut self,
        retained: ResidentProgramKeyedConsultTurn<ResidentProgramConsultPortTurn<Self, StopInput>>,
    ) -> Result<
        (),
        ResidentProgramConsultRetentionFailure<ResidentProgramConsultPortTurn<Self, StopInput>>,
    > {
        let (_, turn) = retained.into_parts();
        self.retain_consult_turn(turn);
        Ok(())
    }

    /// Whether one retained Consult phase has a configured continuation owner.
    fn has_runnable_retained_consult(&self) -> bool {
        false
    }

    /// Return whether the exact parent already has a retained continuation.
    fn has_retained_consult_key(&self, _key: &ResidentProgramConsultTurnKey) -> bool {
        false
    }

    /// Return whether a new keyed continuation can be retained before work starts.
    ///
    /// Point-addressed and background recovery must call this before they
    /// consume a process-local result. A full owner fails closed without
    /// creating an overflow continuation.
    fn can_retain_consult_key(&self, _key: &ResidentProgramConsultTurnKey) -> bool {
        false
    }

    /// Return whether background recovery may create one new keyed continuation.
    fn has_consult_turn_capacity(&self) -> bool {
        false
    }

    /// Capture exact keyed capacity before lending this owner to recovery.
    fn consult_retention_admission(&self) -> ResidentProgramConsultRetentionAdmission {
        ResidentProgramConsultRetentionAdmission::new(false, Vec::new())
    }

    /// Select the first Consult subowner considered by this bounded turn.
    ///
    /// A stateful owner advances its cursor before physical work. One
    /// unavailable custody source therefore cannot indefinitely block another
    /// ready keyed continuation.
    fn next_retained_consult_lane(&mut self) -> ResidentProgramRetainedConsultLane {
        ResidentProgramRetainedConsultLane::StopCustody
    }

    /// Take the next exact parent continuation in stable retained order.
    fn take_runnable_retained_consult(
        &mut self,
    ) -> Option<ResidentProgramKeyedConsultTurn<ResidentProgramConsultPortTurn<Self, StopInput>>>
    {
        None
    }

    /// Restore the exact turn just taken by this owner.
    ///
    /// A bounded owner has already released this turn's slot, so restoration is
    /// infallible. Implementations must not evaluate policy or repeat physical
    /// work while restoring it.
    fn restore_taken_consult_turn(
        &mut self,
        retained: ResidentProgramKeyedConsultTurn<ResidentProgramConsultPortTurn<Self, StopInput>>,
    ) {
        let (_, turn) = retained.into_parts();
        self.retain_consult_turn(turn);
    }

    /// Advance at most one exact retained Consult phase.
    ///
    /// The physical owner must continue through the existing policy, stop,
    /// terminal-join, offer-recovery, or child-successor owner. A deferred turn
    /// remains in the same bounded keyed owner; this method cannot reconstruct a
    /// phase from durable evidence or allocate a replacement identity.
    fn continue_one_retained_consult<
        WriterGuard,
        PolicyEvaluator,
        OperationAuthority,
        ObserveTime,
    >(
        &mut self,
        process: &mut ResidentProcessHost<'_, WriterGuard>,
        deployments: &mut [ResidentProgramDeploymentOperationPorts<'_>],
        operation_authority: &mut OperationAuthority,
        policy: &mut PolicyEvaluator,
        observe_time: &mut ObserveTime,
    ) -> ResidentProgramRetainedPhaseTurn
    where
        WriterGuard: PrincipalWriterGuardPort,
        OperationAuthority: crate::resident::AmbientSessionAuthorityPort,
        PolicyEvaluator: consult_recovery::ResidentConsultationPolicyEvaluator<StopInput>,
        StopInput: ResidentProgramConsultStopInput,
        ObserveTime: FnMut() -> Option<pale_core::records::UnixTimeV1>,
        Self: Sized,
    {
        let mut lane = self.next_retained_consult_lane();
        for _ in 0..3 {
            match lane {
                ResidentProgramRetainedConsultLane::StopCustody => {
                    if let Some(pending) = self.take_consult_stop_custody() {
                        return continue_retained_consult_stop_custody(
                            process,
                            deployments,
                            operation_authority,
                            self,
                            pending,
                        );
                    }
                }
                ResidentProgramRetainedConsultLane::TerminalFailure => {
                    if let Some(pending) = self.take_consult_terminal_failure() {
                        return continue_retained_consult_terminal_failure(
                            process,
                            deployments,
                            operation_authority,
                            self,
                            pending,
                        );
                    }
                }
                ResidentProgramRetainedConsultLane::KeyedTurn => {
                    if let Some(retained) = self.take_runnable_retained_consult() {
                        let Some(observed_at) = observe_time() else {
                            self.restore_taken_consult_turn(retained);
                            return ResidentProgramRetainedPhaseTurn::Deferred;
                        };
                        return continue_one_keyed_consult_turn(
                            process,
                            deployments,
                            operation_authority,
                            policy,
                            self,
                            retained,
                            observed_at,
                        );
                    }
                }
            }
            lane = lane.next();
        }
        ResidentProgramRetainedPhaseTurn::Idle
    }

    /// Retain one exact Consult-child recovery handoff before the sweep yields.
    fn retain_consult_child_turn(
        &mut self,
        turn: consult_child_recovery_sweep::ResidentProcessConsultChildRecoveryTurn,
    );

    /// Take an exact child-recovery handoff before scanning its source again.
    fn take_consult_child_turn(
        &mut self,
    ) -> Option<consult_child_recovery_sweep::ResidentProcessConsultChildRecoveryTurn> {
        None
    }

    /// Lend fresh physical inputs for one already-classified terminal child.
    fn with_consult_child_terminal_inputs<Return, UseInputs>(
        &mut self,
        _use_inputs: UseInputs,
    ) -> Result<Return, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(ResidentProgramConsultChildTerminalInputs<'input>) -> Return,
    {
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    /// Take a retained database/anchor continuation before scanning again.
    fn take_consult_child_terminal_custody(
        &mut self,
    ) -> Option<Box<consult_child_recovery_sweep::ResidentProcessConsultChildTerminalCustodyPending>>
    {
        None
    }

    /// Retain an unfinished database/anchor continuation with its cursor fixed.
    #[allow(clippy::boxed_local)]
    fn retain_consult_child_terminal_custody(
        &mut self,
        _pending: Box<
            consult_child_recovery_sweep::ResidentProcessConsultChildTerminalCustodyPending,
        >,
    ) {
        unreachable!("terminal custody cannot exist without terminal input support")
    }
}

fn retain_point_addressed_consult_turn<Port, StopInput>(
    port: &mut Port,
    key: ResidentProgramConsultTurnKey,
    turn: ResidentProgramConsultPortTurn<Port, StopInput>,
) where
    Port: ResidentProgramConsultRecoveryInputPort<StopInput>,
{
    let retained = ResidentProgramKeyedConsultTurn { key, turn };
    if port.retain_keyed_consult_turn(retained).is_err() {
        unreachable!("a Consult continuation reservation must remain valid until retention")
    }
}

pub(in crate::resident) fn admit_consult_child_recovery_turn<Port, StopInput>(
    port: &mut Port,
    turn: consult_child_recovery_sweep::ResidentProcessConsultChildRecoveryTurn,
) -> Result<
    consult_child_recovery_sweep::ResidentProcessConsultChildRecoveryTurn,
    ResidentProgramTurnError,
>
where
    Port: ResidentProgramConsultRecoveryInputPort<StopInput>,
{
    if turn.requires_keyed_consult_turn() && !port.has_consult_turn_capacity() {
        port.retain_consult_child_turn(turn);
        return Err(ResidentProgramTurnError::RecoveryUnavailable);
    }
    Ok(turn)
}

/// Retention decision that keeps one fully prepared Consult handoff linear.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::resident) enum ResidentProgramConsultReadyRetention<Ready> {
    /// The exact parent already has a continuation, so this Ready handoff is stale.
    Subsumed,
    /// The bounded keyed owner can accept this exact Ready handoff.
    Retain(Ready),
    /// Another parent occupies the configured keyed-turn ceiling. The unchanged
    /// handoff returns to its operation-start owner.
    Deferred(Ready),
}

pub(in crate::resident) fn classify_consult_ready_retention<Port, StopInput, Ready>(
    port: &Port,
    key: &ResidentProgramConsultTurnKey,
    ready: Ready,
) -> ResidentProgramConsultReadyRetention<Ready>
where
    Port: ResidentProgramConsultRecoveryInputPort<StopInput>,
{
    if port.has_retained_consult_key(key) {
        ResidentProgramConsultReadyRetention::Subsumed
    } else if port.can_retain_consult_key(key) {
        ResidentProgramConsultReadyRetention::Retain(ready)
    } else {
        ResidentProgramConsultReadyRetention::Deferred(ready)
    }
}

fn selected_consult_deployment_mut<'ports, 'deployment>(
    deployments: &'deployment mut [ResidentProgramDeploymentOperationPorts<'ports>],
    selector: &ResidentDeploymentAuthoritySelector,
) -> Option<&'deployment mut ResidentProgramDeploymentOperationPorts<'ports>> {
    let mut selected = None;
    for (index, deployment) in deployments.iter().enumerate() {
        if deployment.matches(selector) {
            if selected.is_some() {
                return None;
            }
            selected = Some(index);
        }
    }
    selected.map(|index| &mut deployments[index])
}

fn restore_consult_turn<StopInput, Port>(
    port: &mut Port,
    key: ResidentProgramConsultTurnKey,
    turn: ResidentProgramConsultPortTurn<Port, StopInput>,
) where
    Port: ResidentProgramConsultRecoveryInputPort<StopInput>,
{
    port.restore_taken_consult_turn(ResidentProgramKeyedConsultTurn { key, turn });
}

fn continue_retained_consult_stop_custody<WriterGuard, OperationAuthority, StopInput, Port>(
    process: &mut ResidentProcessHost<'_, WriterGuard>,
    deployments: &mut [ResidentProgramDeploymentOperationPorts<'_>],
    operation_authority: &mut OperationAuthority,
    port: &mut Port,
    retained: ResidentProgramKeyedConsultStopCustody,
) -> ResidentProgramRetainedPhaseTurn
where
    WriterGuard: PrincipalWriterGuardPort,
    OperationAuthority: crate::resident::AmbientSessionAuthorityPort,
    Port: ResidentProgramConsultRecoveryInputPort<StopInput>,
{
    let (key, pending) = retained.into_parts();
    let selector = pending.selector.clone();
    let Some(selected) = selected_consult_deployment_mut(deployments, &selector) else {
        port.retain_consult_stop_custody(ResidentProgramKeyedConsultTurn { key, turn: pending });
        return ResidentProgramRetainedPhaseTurn::Deferred;
    };
    match process.continue_consultation_stop_custody(
        selected.access(),
        operation_authority,
        *pending,
    ) {
        Ok(_) => ResidentProgramRetainedPhaseTurn::Advanced,
        Err(failure) => {
            port.retain_consult_stop_custody(ResidentProgramKeyedConsultTurn {
                key,
                turn: failure.pending,
            });
            ResidentProgramRetainedPhaseTurn::Deferred
        }
    }
}

fn continue_retained_consult_terminal_failure<WriterGuard, OperationAuthority, StopInput, Port>(
    process: &mut ResidentProcessHost<'_, WriterGuard>,
    deployments: &mut [ResidentProgramDeploymentOperationPorts<'_>],
    operation_authority: &mut OperationAuthority,
    port: &mut Port,
    retained: ResidentProgramKeyedConsultTurn<
        ResidentProcessConsultationTerminalJoinFailure<Port::Work, Port::Output>,
    >,
) -> ResidentProgramRetainedPhaseTurn
where
    WriterGuard: PrincipalWriterGuardPort,
    OperationAuthority: crate::resident::AmbientSessionAuthorityPort,
    Port: ResidentProgramConsultRecoveryInputPort<StopInput>,
{
    let (key, failure) = retained.into_parts();
    match failure {
        ResidentProcessConsultationTerminalJoinFailure::Custody(pending)
        | ResidentProcessConsultationTerminalJoinFailure::CustodyPreflight { pending, .. }
        | ResidentProcessConsultationTerminalJoinFailure::CustodySelection { pending, .. } => {
            let selector = pending.selector.clone();
            let Some(selected) = selected_consult_deployment_mut(deployments, &selector) else {
                port.retain_consult_terminal_failure(ResidentProgramKeyedConsultTurn {
                    key,
                    turn: ResidentProcessConsultationTerminalJoinFailure::Custody(pending),
                });
                return ResidentProgramRetainedPhaseTurn::Deferred;
            };
            match process.continue_consultation_terminal_custody(
                selected.access(),
                operation_authority,
                *pending,
            ) {
                Ok(_) => ResidentProgramRetainedPhaseTurn::Advanced,
                Err(failure) => {
                    port.retain_consult_terminal_failure(ResidentProgramKeyedConsultTurn {
                        key,
                        turn: failure,
                    });
                    ResidentProgramRetainedPhaseTurn::Deferred
                }
            }
        }
        ResidentProcessConsultationTerminalJoinFailure::Release { joined, .. }
        | ResidentProcessConsultationTerminalJoinFailure::ReleasePreflight { joined, .. }
        | ResidentProcessConsultationTerminalJoinFailure::ReleaseSelection { joined, .. } => {
            match process.retry_consultation_terminal_release(*joined) {
                Ok(_) => ResidentProgramRetainedPhaseTurn::Advanced,
                Err(failure) => {
                    port.retain_consult_terminal_failure(ResidentProgramKeyedConsultTurn {
                        key,
                        turn: failure,
                    });
                    ResidentProgramRetainedPhaseTurn::Deferred
                }
            }
        }
        ResidentProcessConsultationTerminalJoinFailure::Selection { ready, .. }
        | ResidentProcessConsultationTerminalJoinFailure::Preflight { ready, .. }
        | ResidentProcessConsultationTerminalJoinFailure::Admission { ready, .. } => {
            continue_ready_consult_join(
                process,
                deployments,
                operation_authority,
                port,
                key,
                *ready,
            )
        }
    }
}

fn continue_ready_consult_join<WriterGuard, OperationAuthority, StopInput, Port>(
    process: &mut ResidentProcessHost<'_, WriterGuard>,
    deployments: &mut [ResidentProgramDeploymentOperationPorts<'_>],
    operation_authority: &mut OperationAuthority,
    port: &mut Port,
    key: ResidentProgramConsultTurnKey,
    ready: consult_completion::ResidentProcessConsultationCompletionReady<Port::Work, Port::Output>,
) -> ResidentProgramRetainedPhaseTurn
where
    WriterGuard: PrincipalWriterGuardPort,
    OperationAuthority: crate::resident::AmbientSessionAuthorityPort,
    Port: ResidentProgramConsultRecoveryInputPort<StopInput>,
{
    let Some((candidate, dispatch)) = ready.ready.join_selection() else {
        restore_consult_turn(
            port,
            key,
            ResidentProgramConsultTurn::PrePolicy(
                consult_recovery::ResidentProcessConsultRecoveryTurn::Progress(Box::new(ready)),
            ),
        );
        return ResidentProgramRetainedPhaseTurn::Deferred;
    };
    let selection = ResidentProgramConsultJoinSelection {
        parent: key.clone(),
        slot_id: candidate.slot_id().clone(),
        child: candidate.child().clone(),
        dispatch: dispatch.dispatch().clone(),
    };
    let selector = ready.selector.clone();
    let Some(selected) = selected_consult_deployment_mut(deployments, &selector) else {
        restore_consult_turn(
            port,
            key,
            ResidentProgramConsultTurn::PrePolicy(
                consult_recovery::ResidentProcessConsultRecoveryTurn::Progress(Box::new(ready)),
            ),
        );
        return ResidentProgramRetainedPhaseTurn::Deferred;
    };

    let mut ready = Some(ready);
    let mut result = None;
    let lender = port.with_consult_join_inputs(&selection, |inputs| {
        let ResidentProgramConsultJoinInputs {
            association,
            input,
            checkpoint,
            resolve_store,
            admit_evidence,
            admit_collection,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        } = inputs;
        result = Some(process.join_consultation_terminal(
            association,
            selected.access(),
            operation_authority,
            ready.take().expect("the join input is consumed once"),
            input,
            checkpoint,
            resolve_store,
            |slot, evidence| admit_evidence(slot, evidence),
            |collection| admit_collection(collection),
            |previous_ref, previous_digest, current_ref, current_digest| {
                admit_rotation(previous_ref, previous_digest, current_ref, current_digest)
            },
            |key_id, preimage, digest| sign_observation(key_id, preimage, digest),
            |key_id, preimage, digest| sign_checkpoint(key_id, preimage, digest),
        ));
    });
    match result {
        Some(Ok(_)) => ResidentProgramRetainedPhaseTurn::Advanced,
        Some(Err(failure)) => {
            port.retain_consult_terminal_failure(ResidentProgramKeyedConsultTurn {
                key,
                turn: failure,
            });
            ResidentProgramRetainedPhaseTurn::Deferred
        }
        None => {
            debug_assert!(lender.is_err());
            restore_consult_turn(
                port,
                key,
                ResidentProgramConsultTurn::PrePolicy(
                    consult_recovery::ResidentProcessConsultRecoveryTurn::Progress(Box::new(
                        ready.expect("a refused lender preserves the exact completion"),
                    )),
                ),
            );
            ResidentProgramRetainedPhaseTurn::Deferred
        }
    }
}

fn continue_selected_consult_stop<WriterGuard, OperationAuthority, StopInput, Port>(
    process: &mut ResidentProcessHost<'_, WriterGuard>,
    deployments: &mut [ResidentProgramDeploymentOperationPorts<'_>],
    operation_authority: &mut OperationAuthority,
    port: &mut Port,
    key: ResidentProgramConsultTurnKey,
    started: consult_start::ResidentProcessConsultationStarted,
    selected: crate::resident::consultation::ResidentConsultationStopSelected<StopInput>,
) -> ResidentProgramRetainedPhaseTurn
where
    WriterGuard: PrincipalWriterGuardPort,
    OperationAuthority: crate::resident::AmbientSessionAuthorityPort,
    StopInput: ResidentProgramConsultStopInput,
    Port: ResidentProgramConsultRecoveryInputPort<StopInput>,
{
    let Some(meaning) = selected.input().stop_meaning() else {
        restore_consult_turn(
            port,
            key,
            ResidentProgramConsultTurn::Policy(Box::new(
                consult_recovery::ResidentProcessConsultationPolicyContinuation::from_parts(
                    started,
                    crate::resident::consultation::ResidentConsultationTurnDisposition::Stopped(
                        crate::resident::consultation::ResidentConsultationTurnStopped::Selected(
                            Box::new(selected),
                        ),
                    ),
                ),
            )),
        );
        return ResidentProgramRetainedPhaseTurn::Deferred;
    };
    let selector = key.selector().clone();
    let Some(deployment) = selected_consult_deployment_mut(deployments, &selector) else {
        restore_consult_turn(
            port,
            key,
            ResidentProgramConsultTurn::Policy(Box::new(
                consult_recovery::ResidentProcessConsultationPolicyContinuation::from_parts(
                    started,
                    crate::resident::consultation::ResidentConsultationTurnDisposition::Stopped(
                        crate::resident::consultation::ResidentConsultationTurnStopped::Selected(
                            Box::new(selected),
                        ),
                    ),
                ),
            )),
        );
        return ResidentProgramRetainedPhaseTurn::Deferred;
    };

    let mut started = Some(started);
    let mut selected = Some(selected);
    let mut result = None;
    let lender = port.with_consult_stop_inputs(&key, &meaning, |inputs| {
        let ResidentProgramConsultStopInputs {
            association,
            parent,
            parent_observation,
            parent_checkpoint,
            observation_ref,
            observed_at,
            signing,
            limits,
            checkpoint,
            resolve_store,
            admit_stop,
            admit_collection,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        } = inputs;
        let (current, semantic_input) = selected
            .take()
            .expect("the stop input is consumed once")
            .into_parts();
        let full = crate::resident::consultation::ConsultationStopInput {
            cause: meaning.cause,
            reason: meaning.reason.clone(),
            stopped_at: meaning.stopped_at,
            parent,
            parent_observation,
            parent_checkpoint,
            observation_ref,
            observed_at,
            signing,
            limits,
        };
        let selected = crate::resident::consultation::ResidentConsultationStopSelected::from_parts(
            current, full,
        );
        let committed = match process.bind_consultation_stop(selector.clone(), selected) {
            Ok(selected) => match process.commit_consultation_stop(
                association,
                deployment.access(),
                operation_authority,
                selected,
                checkpoint,
                resolve_store,
                |collection, summary| admit_stop(collection, summary),
                |collection| admit_collection(collection),
                |previous_ref, previous_digest, current_ref, current_digest| {
                    admit_rotation(previous_ref, previous_digest, current_ref, current_digest)
                },
                |key_id, preimage, digest| sign_observation(key_id, preimage, digest),
                |key_id, preimage, digest| sign_checkpoint(key_id, preimage, digest),
            ) {
                Ok(_) => ResidentProgramConsultStopAttempt::Completed,
                Err(ResidentProcessConsultationStopCommitFailure::Custody(pending)) => {
                    ResidentProgramConsultStopAttempt::Custody(pending)
                }
                Err(ResidentProcessConsultationStopCommitFailure::Selection {
                    selected, ..
                })
                | Err(ResidentProcessConsultationStopCommitFailure::Admission {
                    selected, ..
                }) => {
                    let ResidentProcessConsultationStopSelected { selected, .. } = *selected;
                    let (current, _) = (*selected).into_parts();
                    ResidentProgramConsultStopAttempt::Refused {
                        current: Box::new(current),
                        input: semantic_input,
                    }
                }
            },
            Err(failure) => {
                let (current, _) = (*failure.selected).into_parts();
                ResidentProgramConsultStopAttempt::Refused {
                    current: Box::new(current),
                    input: semantic_input,
                }
            }
        };
        result = Some(committed);
    });
    match result {
        Some(ResidentProgramConsultStopAttempt::Completed) => {
            ResidentProgramRetainedPhaseTurn::Advanced
        }
        Some(ResidentProgramConsultStopAttempt::Custody(pending)) => {
            port.retain_consult_stop_custody(ResidentProgramKeyedConsultTurn {
                key,
                turn: pending,
            });
            ResidentProgramRetainedPhaseTurn::Deferred
        }
        Some(ResidentProgramConsultStopAttempt::Refused { current, input }) => {
            let selected =
                crate::resident::consultation::ResidentConsultationStopSelected::from_parts(
                    *current, input,
                );
            restore_consult_turn(
                port,
                key,
                ResidentProgramConsultTurn::Policy(Box::new(
                    consult_recovery::ResidentProcessConsultationPolicyContinuation::from_parts(
                        started.take().expect("a refused stop preserves its start"),
                        crate::resident::consultation::ResidentConsultationTurnDisposition::Stopped(
                            crate::resident::consultation::ResidentConsultationTurnStopped::Selected(
                                Box::new(selected),
                            ),
                        ),
                    ),
                )),
            );
            ResidentProgramRetainedPhaseTurn::Deferred
        }
        None => {
            debug_assert!(lender.is_err());
            restore_consult_turn(
                port,
                key,
                ResidentProgramConsultTurn::Policy(Box::new(
                    consult_recovery::ResidentProcessConsultationPolicyContinuation::from_parts(
                        started.expect("a refused lender preserves the recovered start"),
                        crate::resident::consultation::ResidentConsultationTurnDisposition::Stopped(
                            crate::resident::consultation::ResidentConsultationTurnStopped::Selected(
                                Box::new(
                                    selected.expect("a refused lender preserves the selected stop"),
                                ),
                            ),
                        ),
                    ),
                )),
            );
            ResidentProgramRetainedPhaseTurn::Deferred
        }
    }
}

enum ResidentProgramConsultStopAttempt<StopInput> {
    Completed,
    Custody(Box<ResidentProcessConsultationStopCustody>),
    Refused {
        current: Box<crate::resident::consultation::RecoveredConsultationCollection>,
        input: StopInput,
    },
}

fn apply_ready_consult_policy<WriterGuard, StopInput, Port>(
    process: &mut ResidentProcessHost<'_, WriterGuard>,
    deployments: &mut [ResidentProgramDeploymentOperationPorts<'_>],
    port: &mut Port,
    key: ResidentProgramConsultTurnKey,
    handoff: Box<consult_recovery::ResidentProcessConsultationPolicyHandoff>,
    decision: crate::resident::consultation::ResidentConsultationSupervisionDecision<StopInput>,
) -> ResidentProgramRetainedPhaseTurn
where
    WriterGuard: PrincipalWriterGuardPort,
    StopInput: ResidentProgramConsultStopInput,
    Port: ResidentProgramConsultRecoveryInputPort<StopInput>,
{
    let Some(selected) = selected_consult_deployment_mut(deployments, key.selector()) else {
        restore_consult_turn(
            port,
            key,
            ResidentProgramConsultTurn::PolicyReady { handoff, decision },
        );
        return ResidentProgramRetainedPhaseTurn::Deferred;
    };
    let mut handoff = Some(handoff);
    let mut decision = Some(decision);
    let mut continuation = None;
    let lender = port.use_consult_inputs(
        |_, _, _, _, executor, recover_child, retain_child, _historical_collection| {
            continuation = Some(
                process.continue_recovered_consultation_policy(
                    key.selector(),
                    selected.access(),
                    handoff.take().expect("the policy handoff is consumed once"),
                    decision
                        .take()
                        .expect("the policy decision is consumed once"),
                    executor,
                    |candidate| recover_child(candidate),
                    |candidate, child, custody, anchor| {
                        retain_child(candidate, child, custody, anchor)
                    },
                ),
            );
        },
    );
    match continuation {
        Some(Ok(continuation)) => {
            restore_consult_turn(
                port,
                key,
                ResidentProgramConsultTurn::Policy(Box::new(continuation)),
            );
            ResidentProgramRetainedPhaseTurn::Advanced
        }
        Some(Err(refusal)) => {
            restore_consult_turn(
                port,
                key,
                ResidentProgramConsultTurn::PolicyRefusal(Box::new(refusal)),
            );
            ResidentProgramRetainedPhaseTurn::Deferred
        }
        None => {
            debug_assert!(lender.is_err());
            restore_consult_turn(
                port,
                key,
                ResidentProgramConsultTurn::PolicyReady {
                    handoff: handoff.expect("a refused lender preserves the policy handoff"),
                    decision: decision.expect("a refused lender preserves the policy decision"),
                },
            );
            ResidentProgramRetainedPhaseTurn::Deferred
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn continue_one_keyed_consult_turn<
    WriterGuard,
    PolicyEvaluator,
    OperationAuthority,
    StopInput,
    Port,
>(
    process: &mut ResidentProcessHost<'_, WriterGuard>,
    deployments: &mut [ResidentProgramDeploymentOperationPorts<'_>],
    operation_authority: &mut OperationAuthority,
    policy: &mut PolicyEvaluator,
    port: &mut Port,
    retained: ResidentProgramKeyedConsultTurn<ResidentProgramConsultPortTurn<Port, StopInput>>,
    observed_at: pale_core::records::UnixTimeV1,
) -> ResidentProgramRetainedPhaseTurn
where
    WriterGuard: PrincipalWriterGuardPort,
    OperationAuthority: crate::resident::AmbientSessionAuthorityPort,
    PolicyEvaluator: consult_recovery::ResidentConsultationPolicyEvaluator<StopInput>,
    StopInput: ResidentProgramConsultStopInput,
    Port: ResidentProgramConsultRecoveryInputPort<StopInput>,
{
    let (key, turn) = retained.into_parts();
    match turn {
        ResidentProgramConsultTurn::StartReady { ready } => {
            if !ready_matches_consult_turn(process, &key, &ready) {
                restore_consult_turn(port, key, ResidentProgramConsultTurn::StartReady { ready });
                return ResidentProgramRetainedPhaseTurn::Deferred;
            }
            let deployment_index = match process.deployment_authority_index(ready.selector()) {
                Ok(index) => index,
                Err(_) => {
                    restore_consult_turn(
                        port,
                        key,
                        ResidentProgramConsultTurn::StartReady { ready },
                    );
                    return ResidentProgramRetainedPhaseTurn::Deferred;
                }
            };
            if process.deployments[deployment_index].store() != ready.store()
                || ready.current().store() != ready.store()
            {
                restore_consult_turn(port, key, ResidentProgramConsultTurn::StartReady { ready });
                return ResidentProgramRetainedPhaseTurn::Deferred;
            }
            let mut ready = Some(ready);
            let mut advanced = None;
            let lender = port.use_consult_inputs(
                |_, task_limits, _, _, executor, _, _, _historical_collection| {
                    let (started, current) = (*ready
                        .take()
                        .expect("the Consult input lender runs at most once"))
                    .into_pre_policy_inputs();
                    advanced = Some(process.advance_recovered_consultation_pre_policy(
                        started,
                        task_limits,
                        observed_at,
                        current,
                        executor,
                    ));
                },
            );
            match advanced {
                Some(Ok(turn)) => {
                    restore_consult_turn(port, key, ResidentProgramConsultTurn::PrePolicy(turn));
                    ResidentProgramRetainedPhaseTurn::Advanced
                }
                Some(Err(
                    consult_recovery::ResidentProcessConsultRecoveryError::CompletionBinding(
                        failure,
                    ),
                )) => {
                    restore_consult_turn(
                        port,
                        key,
                        ResidentProgramConsultTurn::CompletionBinding(failure),
                    );
                    ResidentProgramRetainedPhaseTurn::Advanced
                }
                Some(Err(_)) => {
                    unreachable!("a retained Ready phase passed all pre-policy associations")
                }
                None => {
                    debug_assert!(lender.is_err());
                    restore_consult_turn(
                        port,
                        key,
                        ResidentProgramConsultTurn::StartReady {
                            ready: ready.expect("an unavailable lender preserves the Ready phase"),
                        },
                    );
                    ResidentProgramRetainedPhaseTurn::Deferred
                }
            }
        }
        ResidentProgramConsultTurn::PolicyReady { handoff, decision } => {
            apply_ready_consult_policy(process, deployments, port, key, handoff, decision)
        }
        ResidentProgramConsultTurn::PolicyUnavailable(handoff) => {
            match process.evaluate_recovered_consultation_policy(handoff, policy) {
                Ok((handoff, decision)) => {
                    restore_consult_turn(
                        port,
                        key,
                        ResidentProgramConsultTurn::PolicyReady { handoff, decision },
                    );
                    ResidentProgramRetainedPhaseTurn::Advanced
                }
                Err(failure) => {
                    let (handoff, _) = failure.into_parts();
                    restore_consult_turn(
                        port,
                        key,
                        ResidentProgramConsultTurn::PolicyUnavailable(handoff),
                    );
                    ResidentProgramRetainedPhaseTurn::Deferred
                }
            }
        }
        ResidentProgramConsultTurn::PolicyRefusal(refusal) => {
            let (handoff, decision, _) = (*refusal).into_parts();
            restore_consult_turn(
                port,
                key,
                ResidentProgramConsultTurn::PolicyReady { handoff, decision },
            );
            ResidentProgramRetainedPhaseTurn::Advanced
        }
        ResidentProgramConsultTurn::CompletionBinding(failure) => {
            let consult_completion::ResidentProcessConsultationCompletionBindingFailure {
                selector,
                ready,
                ..
            } = *failure;
            match process.bind_consultation_completion(selector, *ready) {
                Ok(ready) => {
                    restore_consult_turn(
                        port,
                        key,
                        ResidentProgramConsultTurn::PrePolicy(
                            consult_recovery::ResidentProcessConsultRecoveryTurn::Progress(
                                Box::new(ready),
                            ),
                        ),
                    );
                    ResidentProgramRetainedPhaseTurn::Advanced
                }
                Err(failure) => {
                    restore_consult_turn(
                        port,
                        key,
                        ResidentProgramConsultTurn::CompletionBinding(failure),
                    );
                    ResidentProgramRetainedPhaseTurn::Deferred
                }
            }
        }
        ResidentProgramConsultTurn::PrePolicy(turn) => match turn {
            consult_recovery::ResidentProcessConsultRecoveryTurn::PolicyRequired(handoff) => {
                match process.evaluate_recovered_consultation_policy(handoff, policy) {
                    Ok((handoff, decision)) => {
                        restore_consult_turn(
                            port,
                            key,
                            ResidentProgramConsultTurn::PolicyReady { handoff, decision },
                        );
                        ResidentProgramRetainedPhaseTurn::Advanced
                    }
                    Err(failure) => {
                        let (handoff, _) = failure.into_parts();
                        restore_consult_turn(
                            port,
                            key,
                            ResidentProgramConsultTurn::PolicyUnavailable(handoff),
                        );
                        ResidentProgramRetainedPhaseTurn::Deferred
                    }
                }
            }
            consult_recovery::ResidentProcessConsultRecoveryTurn::Progress(ready) => {
                continue_ready_consult_join(
                    process,
                    deployments,
                    operation_authority,
                    port,
                    key,
                    *ready,
                )
            }
            turn @ consult_recovery::ResidentProcessConsultRecoveryTurn::NeedsRecovery(_) => {
                restore_consult_turn(port, key, ResidentProgramConsultTurn::PrePolicy(turn));
                ResidentProgramRetainedPhaseTurn::Deferred
            }
            consult_recovery::ResidentProcessConsultRecoveryTurn::Exhausted
            | consult_recovery::ResidentProcessConsultRecoveryTurn::SkippedOtherOwner(_)
            | consult_recovery::ResidentProcessConsultRecoveryTurn::ReportedTerminal(_)
            | consult_recovery::ResidentProcessConsultRecoveryTurn::Pending(_)
            | consult_recovery::ResidentProcessConsultRecoveryTurn::StoppedEstablished(_) => {
                ResidentProgramRetainedPhaseTurn::Advanced
            }
        },
        ResidentProgramConsultTurn::Policy(continuation) => {
            let (started, disposition) = (*continuation).into_parts();
            match disposition {
                crate::resident::consultation::ResidentConsultationTurnDisposition::Progress(
                    crate::resident::consultation::ResidentConsultationTurnProgress::CompletionReady(
                        ready,
                    ),
                ) => match process
                    .bind_consultation_completion(started.selector().clone(), *ready)
                {
                    Ok(ready) => {
                        restore_consult_turn(
                            port,
                            key,
                            ResidentProgramConsultTurn::PrePolicy(
                                consult_recovery::ResidentProcessConsultRecoveryTurn::Progress(
                                    Box::new(ready),
                                ),
                            ),
                        );
                        ResidentProgramRetainedPhaseTurn::Advanced
                    }
                    Err(failure) => {
                        restore_consult_turn(
                            port,
                            key,
                            ResidentProgramConsultTurn::CompletionBinding(failure),
                        );
                        ResidentProgramRetainedPhaseTurn::Deferred
                    }
                },
                crate::resident::consultation::ResidentConsultationTurnDisposition::Stopped(
                    crate::resident::consultation::ResidentConsultationTurnStopped::Selected(
                        selected,
                    ),
                ) => continue_selected_consult_stop(
                    process,
                    deployments,
                    operation_authority,
                    port,
                    key,
                    started,
                    *selected,
                ),
                crate::resident::consultation::ResidentConsultationTurnDisposition::Progress(
                    crate::resident::consultation::ResidentConsultationTurnProgress::Offered {
                        ..
                    },
                )
                | crate::resident::consultation::ResidentConsultationTurnDisposition::Stopped(
                    crate::resident::consultation::ResidentConsultationTurnStopped::Established(_),
                ) => ResidentProgramRetainedPhaseTurn::Advanced,
                disposition @ (
                    crate::resident::consultation::ResidentConsultationTurnDisposition::Pending(_)
                    | crate::resident::consultation::ResidentConsultationTurnDisposition::NeedsRecovery(_)
                ) => {
                    restore_consult_turn(
                        port,
                        key,
                        ResidentProgramConsultTurn::Policy(Box::new(
                            consult_recovery::ResidentProcessConsultationPolicyContinuation::from_parts(
                                started,
                                disposition,
                            ),
                        )),
                    );
                    ResidentProgramRetainedPhaseTurn::Deferred
                }
            }
        }
    }
}

fn ready_matches_consult_turn<WriterGuard>(
    process: &ResidentProcessHost<'_, WriterGuard>,
    key: &ResidentProgramConsultTurnKey,
    ready: &consult_start::ResidentProcessConsultationReady,
) -> bool
where
    WriterGuard: PrincipalWriterGuardPort,
{
    let batch = ready.coordinate();
    let parent = key.parent();
    let current_parent = ready.current().collection().parent();
    key.selector() == ready.selector()
        && parent.record_ref() == &batch.parent().record_ref
        && parent.record_sha256().as_str() == batch.parent().record_sha256.as_str()
        && batch.store() == ready.store()
        && current_parent == batch.parent()
        && process.retains_consultation_batch(batch)
}

/// Preserve a terminal Consult-child handoff when its physical inputs are unavailable.
pub(in crate::resident) fn retain_consult_child_terminal_input_refusal<StopInput, Port>(
    port: &mut Port,
    handoff: Box<consult_child_recovery_sweep::ResidentProcessConsultChildRecoveryHandoff>,
) -> ResidentProgramTurnError
where
    Port: ResidentProgramConsultRecoveryInputPort<StopInput>,
{
    port.retain_consult_child_turn(
        consult_child_recovery_sweep::ResidentProcessConsultChildRecoveryTurn::Recovered(handoff),
    );
    ResidentProgramTurnError::RecoveryUnavailable
}

/// Fresh identities, bounds, and signing selection for one cancellation append.
///
/// The program supplies the admitted request, exact locator recovery, current
/// time, and retained deployment. This value supplies only fresh record
/// coordinates and already-selected signing/bounds; it cannot choose the
/// cancellation target or semantic route.
pub(in crate::resident) struct ResidentProgramCancellationAllocation<'a> {
    pub(in crate::resident) request_ref: pale_core::records::ResidentRecordRefV1,
    pub(in crate::resident) observation_ref: pale_core::records::ResidentRecordRefV1,
    pub(in crate::resident) signing: pale_core::records::SignedResidentAdmissionContextV1<'a>,
    pub(in crate::resident) checkpoint: crate::resident::OperationCheckpointInput,
    pub(in crate::resident) checkpoint_issued_at: pale_core::records::UnixTimeV1,
    pub(in crate::resident) checkpoint_schema: &'a pale_core::records::SchemaDigest,
    pub(in crate::resident) accumulator_limits: pale_ports::AccumulatorPreparationLimits,
    pub(in crate::resident) local_completion:
        crate::resident::operation_service::ResidentLocalCancellationCompletionInput,
    pub(in crate::resident) consultation_limits:
        crate::resident::consultation::ConsultationCollectionLimits,
}

pub(in crate::resident) struct ResidentProgramCancellationInputs<'a> {
    pub(in crate::resident) allocation: ResidentProgramCancellationAllocation<'a>,
    pub(in crate::resident) authorize: &'a mut (dyn for<'context> FnMut(
        crate::resident::OperationControlAuthorizationContext<'context>,
    ) -> Result<(), InvalidResidentRecordV1>
                 + 'a),
    pub(in crate::resident) resolve_store:
        &'a (dyn crate::resident::OperationObservationAuthorityStoreResolver + 'a),
    pub(in crate::resident) admit_rotation: &'a mut ResidentProgramRotationAdmission<'a>,
    pub(in crate::resident) sign_observation: &'a mut ResidentProgramObservationSigner<'a>,
    pub(in crate::resident) sign_checkpoint: &'a mut ResidentProgramCheckpointSigner<'a>,
    pub(in crate::resident) admit_stop: &'a mut (dyn FnMut(
        &crate::resident::consultation::ConsultationCollection,
        &crate::resident::consultation::ConsultationRequirementSummary,
    ) -> Result<(), InvalidResidentRecordV1>
                 + 'a),
    pub(in crate::resident) admit_collection: &'a mut (dyn FnMut(
        &crate::resident::consultation::ConsultationCollection,
    ) -> Result<(), InvalidResidentRecordV1>
                 + 'a),
}

type ResidentProgramRotationAdmission<'a> = dyn FnMut(
        &pale_core::records::ResidentRecordRefV1,
        &pale_core::records::ArtifactDigest,
        &pale_core::records::ResidentRecordRefV1,
        &pale_core::records::ArtifactDigest,
    ) -> Result<(), InvalidResidentRecordV1>
    + 'a;
type ResidentProgramObservationSigner<'a> = dyn FnMut(
        &[u8],
        &pale_core::records::OperationObservationPreimageV1,
        &pale_core::records::OperationObservationDigest,
    ) -> Result<pale_core::records::CanonicalSignatureV1, InvalidResidentRecordV1>
    + 'a;
type ResidentProgramCheckpointSigner<'a> = dyn FnMut(
        &[u8],
        &pale_core::records::ControlStateCheckpointPreimageV1,
        &pale_core::records::ControlStateCheckpointDigest,
    ) -> Result<pale_core::records::CanonicalSignatureV1, InvalidResidentRecordV1>
    + 'a;

/// Physical authority and custody inputs for one exact Cancel turn.
///
/// Implementations cannot select the request, locator, operation class, or
/// response. The application builds the existing cancellation input and calls
/// the existing general or consultation-specific owner after exact point
/// recovery. Any returned linear custody is retained here before a failure is
/// exposed to the frame caller.
pub(in crate::resident) trait ResidentProgramCancellationInputPort {
    /// Linear occupancy proof for one exact admitted Cancel target.
    type Reservation;

    /// Reserve one bounded retained-phase slot before any cancellation input is lent.
    fn reserve_cancellation(
        &mut self,
        deployment: &ResidentDeploymentAuthoritySelector,
        request: &CommonRequestEnvelopeV1,
        locator: &pale_ports::ControlRecordCoordinate,
    ) -> Result<Self::Reservation, ResidentOperationFrameFailure>;

    /// Release a reservation when no typed continuation leaves the turn.
    fn release_cancellation(&mut self, reservation: Self::Reservation);

    /// Lend physical inputs for one exact application-selected Cancel target.
    ///
    /// The default fails closed so an owner that has not joined its physical
    /// inputs to the selected deployment, admitted request, and exact handle
    /// coordinate cannot run cancellation mutation.
    fn with_selected_cancellation_inputs<Return, UseInputs>(
        &mut self,
        _deployment: &ResidentDeploymentAuthoritySelector,
        _request: &CommonRequestEnvelopeV1,
        _locator: &pale_ports::ControlRecordCoordinate,
        _use_inputs: UseInputs,
    ) -> Result<Return, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(ResidentProgramCancellationInputs<'input>) -> Return,
    {
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    fn retain_dispatched_cancellation(
        &mut self,
        reservation: Self::Reservation,
        pending: Box<ResidentProcessDispatchedCancellationCustody>,
    );

    fn retain_cancellation_custody(
        &mut self,
        reservation: Self::Reservation,
        pending: Box<ResidentProcessCancellationCustody>,
    );

    fn retain_a2a_cancellation(
        &mut self,
        reservation: Self::Reservation,
        pending: ResidentProcessA2aDelegatePoll,
    );

    fn retain_consultation_cancellation(
        &mut self,
        reservation: Self::Reservation,
        pending: Box<ResidentProcessConsultationCancellationCustody>,
    );

    /// Retain the exact signed Consult cancellation before its first write.
    fn retain_consultation_cancellation_preparation(
        &mut self,
        reservation: Self::Reservation,
        pending: Box<ResidentProcessConsultationCancellationPreparation>,
    );

    /// Whether one typed cancellation continuation occupies the shared pool.
    fn has_retained_cancellation(&self) -> bool {
        false
    }

    /// Select the deployment required by the next retained cancellation phase.
    fn retained_cancellation_selector(&self) -> Option<&ResidentDeploymentAuthoritySelector> {
        None
    }

    /// Advance at most one retained cancellation phase through its lower owner.
    fn continue_one_retained_cancellation<WriterGuard, Authority>(
        &mut self,
        _process: &mut ResidentProcessHost<'_, WriterGuard>,
        _access: ResidentDeploymentOperationAccess<'_>,
        _authority: &mut Authority,
    ) -> ResidentProgramRetainedPhaseTurn
    where
        WriterGuard: PrincipalWriterGuardPort,
        Authority: crate::resident::AmbientSessionAuthorityPort,
    {
        ResidentProgramRetainedPhaseTurn::Idle
    }
}

/// Physical recovery and disclosure inputs for one read-only Observe turn.
///
/// The application selects the peer, request policy, deployment, session,
/// operation, and locator before borrowing these inputs. This port cannot
/// choose an Observe target or return a semantic result.
pub(in crate::resident) struct ResidentProgramObserveInputs<'request, 'recovery> {
    pub(in crate::resident) recovery: OperationControlRecoveryInput<'recovery>,
    pub(in crate::resident) verify_caller_signature:
        &'request mut request_admission::CallerSignatureVerification<'request>,
    pub(in crate::resident) admit_replay:
        &'request mut ResidentProgramObserveReplayAdmission<'request>,
    pub(in crate::resident) recovery_admission: OperationControlRecoveryAdmission<'recovery>,
    pub(in crate::resident) authorize_disclosure:
        &'recovery mut (dyn for<'context> FnMut(
            crate::resident::OperationControlAuthorizationContext<'context>,
        ) -> Result<(), InvalidResidentRecordV1>
                            + 'recovery),
    pub(in crate::resident) response_deadline: Instant,
}

type ResidentProgramObserveReplayAdmission<'a> = dyn FnMut(
        &pale_core::records::ResidentRequestIdV1,
        &pale_core::records::CommonRequestEnvelopeDigest,
        &[u8],
    ) -> Result<
        observe_request::ResidentObserveReplayDisposition,
        observe_request::ResidentObserveReplayAdmissionError,
    > + 'a;

/// Lend physical Observe inputs after application-owned request admission.
pub(in crate::resident) trait ResidentProgramObserveInputPort<'recovery> {
    fn with_observe_inputs<Output, UseInputs>(
        &mut self,
        locator: &pale_ports::ControlRecordCoordinate,
        use_inputs: UseInputs,
    ) -> Result<Output, ResidentObserveFrameFailure>
    where
        UseInputs:
            for<'request> FnOnce(ResidentProgramObserveInputs<'request, 'recovery>) -> Output;
}

/// Explicit absence of the physical Consult continuation inputs.
///
/// Start-only compositions use this value and fail closed if an admitted
/// `AdvanceConsult` reaches them.
pub(in crate::resident) struct ResidentProgramConsultRecoveryUnavailable;

pub(in crate::resident) struct ResidentProgramUnavailableConsultExecutor;

impl crate::resident::task_owner::ResidentOperationTaskExecutor<(), ()>
    for ResidentProgramUnavailableConsultExecutor
{
    fn try_accept(
        &mut self,
        task: crate::resident::task_owner::ResidentOperationTask<()>,
    ) -> Result<(), crate::resident::task_owner::ResidentOperationTask<()>> {
        Err(task)
    }

    fn try_completion(
        &mut self,
    ) -> Option<crate::resident::task_owner::ResidentOperationTaskCompletion<(), ()>> {
        None
    }
}

impl<StopInput> ResidentProgramConsultRecoveryInputPort<StopInput>
    for ResidentProgramConsultRecoveryUnavailable
{
    type Work = ();
    type Output = ();
    type ChildRecoveryError = InvalidResidentRecordV1;
    type RetentionFailure = InvalidResidentRecordV1;
    type Executor = ResidentProgramUnavailableConsultExecutor;
    type StoreResolver = fn(
        &pale_core::records::OperationObservationV1,
    ) -> Result<
        pale_ports::AuthorityStoreCoordinate,
        crate::resident::OperationObservationAuthorityStoreRejection,
    >;
    type AdmitCollection = fn(
        &crate::resident::consultation::ConsultationCollection,
    ) -> Result<(), InvalidResidentRecordV1>;
    type RecoverChild =
        fn(
            &crate::resident::consultation::ConsultationChildDispatchCandidate,
        )
            -> Result<crate::resident::RecoveredOperationObservation, InvalidResidentRecordV1>;
    type RetainChild = fn(
        &crate::resident::consultation::ConsultationChildDispatchCandidate,
        crate::resident::RecoveredOperationObservation,
        &mut dyn OperationCustodyPort,
        &mut dyn AntiRollbackAnchorPort,
    ) -> crate::resident::consultation::ConsultationChildRetentionOutcome<
        (),
        InvalidResidentRecordV1,
    >;

    fn with_consult_inputs<Return, UseInputs>(
        &mut self,
        _: UseInputs,
    ) -> Result<Return, ResidentOperationFrameFailure>
    where
        UseInputs: FnOnce(
            crate::resident::consultation::ConsultationCollectionLimits,
            crate::resident::consultation::ResidentConsultationTaskLimits,
            &Self::StoreResolver,
            &mut Self::AdmitCollection,
            &mut Self::Executor,
            &mut Self::RecoverChild,
            &mut Self::RetainChild,
            Option<&mut crate::resident::HistoricalConsultationAdmission<'_>>,
        ) -> Return,
    {
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    fn retain_consult_turn(&mut self, _: ResidentProgramConsultPortTurn<Self, StopInput>) {
        unreachable!("unavailable Consult inputs cannot produce a retained turn")
    }

    fn retain_consult_child_turn(
        &mut self,
        _: consult_child_recovery_sweep::ResidentProcessConsultChildRecoveryTurn,
    ) {
        unreachable!("unavailable Consult inputs cannot produce a child recovery handoff")
    }
}

/// Concrete operation-turn owner for peer-first admission, bounded progress,
/// and recovery.
///
/// This owner is reusable across frames. It retains physical ports and current
/// authority readers, but delegates all operation semantics to the existing
/// process-host owners. The in-flight admitted request is consumed by exactly
/// one response and never survives into a later turn.
pub(in crate::resident) struct ResidentProgramOperationTurnOwner<
    'historical,
    'ports,
    PeerWitness,
    Peer,
    Evidence,
    Clock,
    SessionAuthority,
    Signature,
    Random,
    OperationAuthority,
    Cryptography,
    Recovery,
    ConsultRecovery,
> {
    peer: Peer,
    current: ResidentProgramCurrentRequestAdmission<
        'historical,
        PeerWitness,
        Evidence,
        Clock,
        SessionAuthority,
        Signature,
    >,
    deployments: Box<[ResidentProgramDeploymentOperationPorts<'ports>]>,
    random: Random,
    operation_authority: OperationAuthority,
    cryptography: Cryptography,
    recovery: Recovery,
    consult_recovery: ConsultRecovery,
    admitted: Option<Box<ResidentProgramAdmittedRequest>>,
    admitted_peer: Option<PeerWitness>,
    pending_consult_peer: Option<PeerWitness>,
    native_hooks: Option<native_hook_start::NativeHookStarts>,
    retained_start_exchange: Option<Box<ResidentProgramRetainedStartExchange>>,
    next_retained_phase_lane: ResidentProgramRetainedPhaseLane,
    next_recovery_phase: ResidentProgramRecoveryPhaseLane,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ResidentProgramRetainedPhaseLane {
    Start,
    Consult,
    Recovery,
}

impl ResidentProgramRetainedPhaseLane {
    const fn next(self) -> Self {
        match self {
            Self::Start => Self::Consult,
            Self::Consult => Self::Recovery,
            Self::Recovery => Self::Start,
        }
    }
}

/// Fair selection cursor across independent retained Consult subowners.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::resident) enum ResidentProgramRetainedConsultLane {
    /// An unfinished durable stop append.
    StopCustody,
    /// An unfinished terminal completion join.
    TerminalFailure,
    /// The bounded FIFO of ordinary parent-keyed turns.
    KeyedTurn,
}

impl ResidentProgramRetainedConsultLane {
    /// Return the next subowner in stable round-robin order.
    pub(in crate::resident) const fn next(self) -> Self {
        match self {
            Self::StopCustody => Self::TerminalFailure,
            Self::TerminalFailure => Self::KeyedTurn,
            Self::KeyedTurn => Self::StopCustody,
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ResidentProgramRecoveryPhaseLane {
    Invoke,
    Delegate,
    Cancellation,
}

impl ResidentProgramRecoveryPhaseLane {
    const fn other(self) -> Self {
        match self {
            Self::Invoke => Self::Delegate,
            Self::Delegate => Self::Cancellation,
            Self::Cancellation => Self::Invoke,
        }
    }
}

/// One exact operation-start exchange retained by its application owner.
///
/// The request, locator, selector, and operation kind prevent unrelated work
/// from consuming this linear state. Unfinished phases repeat their
/// process-owner and store fences before they borrow a physical port. An issued
/// phase already has durable custody and waits only for the matching signed
/// Start request to receive its exact response.
#[must_use = "retain until the matching signed Start receives this exact exchange"]
struct ResidentProgramRetainedStartExchange {
    kind: operation_request_host::ResidentOperationRequestKind,
    selector: ResidentDeploymentAuthoritySelector,
    request: CommonRequestEnvelopeV1,
    locator: Option<ResidentOperationFrameLocator>,
    phase: ResidentProgramOperationStartPhase,
}

/// Exact retained state of one operation-start exchange.
enum ResidentProgramOperationStartPhase {
    /// A physical owner must continue the exact unfinished custody phase.
    Pending(ResidentProcessOperationStartContinuation),
    /// Custody completed without a waiting frame; retain the issued result for
    /// the matching request rather than replaying any start work.
    Issued(Box<crate::resident::IssuedLocalOperation>),
    /// The signed Consult parent is waiting for current admission of its exact
    /// ordered roster. No parent or child custody is available in this phase.
    ConsultRoster(Box<ResidentProgramConsultRosterPhase>),
    /// Parent meaning and every allocated child plan are fixed. A later
    /// protected-custody turn must commit this exact linear phase.
    ConsultPrepared(Box<ResidentProgramPreparedConsultationStart>),
    /// The prepared parent is waiting for durable custody. No child start is
    /// available until this phase yields an issued parent.
    ConsultParent(Box<ResidentProgramConsultParentPhase>),
    /// The parent is durable and the exact allocated child suffix is advancing
    /// in retained roster order.
    ConsultChildren(Box<ResidentProgramConsultChildSchedulePhase>),
    /// Parent and fresh children are durable; initial collection custody is the
    /// next distinct owner boundary.
    ConsultCollection(Box<consult_start::ResidentProcessConsultationCollectionPending>),
    /// Initial collection custody began and must continue without rebuilding,
    /// resigning, or reallocating any parent or child identity.
    ConsultCollectionCustody(Box<consult_start::ResidentProcessConsultationOpenJourneyFailure>),
    /// Initial collection custody completed. The exact protected recovery or
    /// finite-batch continuation is retained below this boundary.
    ConsultOpened(Box<ResidentProgramConsultOpenedPhase>),
}

/// Exact post-custody Consult state before any child dispatch is allowed.
enum ResidentProgramConsultOpenedPhase {
    /// Protected recovery of the committed collection has not started.
    Opened(consult_start::ResidentProcessConsultationOpened),
    /// Read-only recovery did not complete; retry the same opened collection.
    Recovery(consult_start::ResidentProcessConsultationRecoveryFailure),
    /// Protected recovery completed; task-owner admission has not started.
    Recovered(consult_start::ResidentProcessConsultationRecovered),
    /// The exact recovered collection could not enter the bounded task owner.
    Batch(consult_start::ResidentProcessConsultationBatchFailure),
    /// The finite first batch is retained and ready for child-specific recovery.
    Ready(consult_start::ResidentProcessConsultationReady),
}

#[allow(clippy::too_many_arguments)]
fn retain_consult_start_exchange<PeerWitness>(
    retained_start_exchange: &mut Option<Box<ResidentProgramRetainedStartExchange>>,
    pending_peer: &mut Option<PeerWitness>,
    kind: operation_request_host::ResidentOperationRequestKind,
    selector: ResidentDeploymentAuthoritySelector,
    request: CommonRequestEnvelopeV1,
    locator: Option<ResidentOperationFrameLocator>,
    peer: PeerWitness,
    phase: ResidentProgramOperationStartPhase,
) {
    *pending_peer = Some(peer);
    *retained_start_exchange = Some(Box::new(ResidentProgramRetainedStartExchange {
        kind,
        selector,
        request,
        locator,
        phase,
    }));
}

/// Program-owned state for one signed Consult roster preparation.
enum ResidentProgramConsultRosterPhase {
    /// No child slot has failed yet.
    Pending(ResidentProcessConsultationRosterStartContinuation),
    /// The exact materialized prefix and failed slot remain available to retry.
    Retry {
        owner_instance: ResidentOperationTaskOwnerInstance,
        selector: ResidentDeploymentAuthoritySelector,
        checkpoint: ResidentProcessPreparedStartCheckpoint,
        failure: ResidentProgramConsultRosterPreparationFailure,
    },
}

type ResidentProgramConsultRosterPreparationFailure =
    start_factories::ResidentConsultationRosterPreparationFailure<
        ResidentProcessConsultFreshChildPolicy,
        ResidentProgramPreparedConsultChild,
        ResidentProgramConsultChildPreparationFailure,
    >;

/// One allocated child whose private plan is frozen into the parent intent.
///
/// This value retains the same lease phase used to capture the plan. It has no
/// dispatch, provider, collection, or response authority.
struct ResidentProgramPreparedConsultChild {
    admitted: Box<ResidentProgramAdmittedRequest>,
    pending: Box<ResidentProcessRetainedOperationLease>,
    policy: Box<ResidentProcessConsultFreshChildPolicy>,
}

/// Exact child-preparation state retained at an incomplete roster slot.
enum ResidentProgramConsultChildPreparationFailure {
    /// The declarative roster selected a class that Consult cannot own.
    UnsupportedClass,
    /// Current request admission failed before identity allocation.
    Admission(ResidentOperationFrameFailure),
    /// Request-first allocation failed before returning an identity phase.
    Allocation(ResidentProcessOperationAllocationError),
    /// An existing operation was found where this roster requires a fresh child.
    Existing(Box<ResidentProcessExistingOperationStart>),
    /// Lease admission or retention stopped after identity allocation.
    Lease {
        policy: Box<ResidentProcessConsultFreshChildPolicy>,
        failure: Box<ResidentProcessOperationLeaseFailure>,
    },
    /// Input construction stopped after exact lease retention.
    Factory {
        child: ResidentProgramPreparedConsultChild,
        reason: start_factories::ResidentOperationStartFactoryError,
    },
}

/// Prepared parent and ordered child plans plus the original checkpoint inputs.
struct ResidentProgramPreparedConsultationStart {
    owner_instance: ResidentOperationTaskOwnerInstance,
    selector: ResidentDeploymentAuthoritySelector,
    checkpoint: ResidentProcessPreparedStartCheckpoint,
    prepared:
        start_factories::ResidentPreparedConsultationStart<ResidentProgramPreparedConsultChild>,
}

/// One exact allocated child while its durable start advances.
enum ResidentProgramConsultChildStart {
    /// No child signer or physical owner has run yet.
    Prepared(ResidentProgramPreparedConsultChild),
    /// Physical child start began; this continuation is the only retry path.
    Continuation(Box<ResidentProgramConsultChildContinuation>),
}

/// Physical child-start inputs not carried by the ordinary continuation.
struct ResidentProgramConsultChildContinuation {
    checkpoint: ResidentProcessPreparedStartCheckpoint,
    continuation: ResidentProcessOperationStartContinuation,
}

type ResidentProgramConsultParentCommitFailure =
    start_factories::ResidentConsultationParentCommitFailure<
        ResidentProgramConsultChildStart,
        ResidentProcessPreparedOperationStartFailure,
    >;

/// Parent-custody phase plus every already allocated child identity.
enum ResidentProgramConsultParentPhase {
    Pending {
        owner_instance: ResidentOperationTaskOwnerInstance,
        selector: ResidentDeploymentAuthoritySelector,
        checkpoint: ResidentProcessPreparedStartCheckpoint,
        pending: start_factories::ResidentConsultationParentCommitPending<
            ResidentProgramConsultChildStart,
        >,
    },
    Retry {
        owner_instance: ResidentOperationTaskOwnerInstance,
        selector: ResidentDeploymentAuthoritySelector,
        checkpoint: ResidentProcessPreparedStartCheckpoint,
        failure: ResidentProgramConsultParentCommitFailure,
    },
    /// The retained parent intent and allocated child count disagree. This is
    /// fail-closed state, not authority to rebuild either value.
    Invalid {
        checkpoint: ResidentProcessPreparedStartCheckpoint,
        prepared:
            start_factories::ResidentPreparedConsultationStart<ResidentProgramConsultChildStart>,
    },
}

type ResidentProgramConsultChildSchedule = start_factories::ResidentConsultationChildSchedule<
    crate::resident::IssuedLocalOperation,
    ResidentProgramConsultChildStart,
    crate::resident::IssuedLocalOperation,
>;

/// Deployment fence around one durable parent and its exact child suffix.
struct ResidentProgramConsultChildSchedulePhase {
    selector: ResidentDeploymentAuthoritySelector,
    store: AuthorityStoreCoordinate,
    schedule: ResidentProgramConsultChildSchedule,
}

#[allow(clippy::too_many_arguments)]
fn prepare_program_consult_roster<
    WriterGuard,
    PeerWitness,
    Evidence,
    Clock,
    SessionAuthority,
    Signature,
    Random,
    OperationAuthority,
    Cryptography,
>(
    process: &mut ResidentProcessHost<'_, WriterGuard>,
    current: &mut ResidentProgramCurrentRequestAdmission<
        '_,
        PeerWitness,
        Evidence,
        Clock,
        SessionAuthority,
        Signature,
    >,
    deployment_authority: &ResidentProgramDeploymentAuthority,
    deployment: &mut ResidentProgramDeploymentOperationPorts<'_>,
    peer: &PeerWitness,
    random: &Random,
    operation_authority: &mut OperationAuthority,
    cryptography: &Cryptography,
    checkpoint_authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    phase: ResidentProgramConsultRosterPhase,
) -> Result<ResidentProgramPreparedConsultationStart, Box<ResidentProgramConsultRosterPhase>>
where
    WriterGuard: PrincipalWriterGuardPort,
    Evidence: ResidentCurrentRequestEvidencePort<PeerWitness>,
    Clock: pale_ports::WallClockSourcePort,
    SessionAuthority: crate::resident::AmbientSessionAuthorityPort,
    Signature: ResidentProgramCallerSignaturePort,
    Random: CryptographicRandomSourcePort,
    OperationAuthority: crate::resident::AmbientSessionAuthorityPort,
    Cryptography: ResidentProgramInvokeCryptographyPort,
{
    let (owner_instance, selector, checkpoint, preparation) = match phase {
        ResidentProgramConsultRosterPhase::Pending(pending) => {
            let (owner, selector, checkpoint, pending) = pending.into_parts();
            (owner, selector, checkpoint, Ok(pending))
        }
        ResidentProgramConsultRosterPhase::Retry {
            owner_instance,
            selector,
            checkpoint,
            failure,
        } => (owner_instance, selector, checkpoint, Err(failure)),
    };
    if !owner_instance.is_same(process.operations.operation_task_owner_instance())
        || deployment.selector != selector
        || deployment_authority.selector() != &selector
        || deployment.store
            != AuthorityStoreCoordinate::new(
                checkpoint_authority.control_store_id.clone(),
                checkpoint_authority.control_incarnation_id.clone(),
            )
    {
        return Err(Box::new(match preparation {
            Ok(pending) => ResidentProgramConsultRosterPhase::Pending(
                ResidentProcessConsultationRosterStartContinuation {
                    owner_instance,
                    selector,
                    parent_checkpoint: checkpoint,
                    pending,
                },
            ),
            Err(failure) => ResidentProgramConsultRosterPhase::Retry {
                owner_instance,
                selector,
                checkpoint,
                failure,
            },
        }));
    }

    let prepared = match preparation {
        Ok(pending) => pending.prepare(
            |parent, child| {
                prepare_program_consult_child(
                    process,
                    current,
                    deployment_authority,
                    deployment,
                    peer,
                    random,
                    operation_authority,
                    checkpoint_authority,
                    parent,
                    child,
                    None,
                )
            },
            |bytes, preimage, digest| cryptography.sign_observation(bytes, preimage, digest),
        ),
        Err(failure) => failure.continue_prepare(
            |parent, child, prior| {
                prepare_program_consult_child(
                    process,
                    current,
                    deployment_authority,
                    deployment,
                    peer,
                    random,
                    operation_authority,
                    checkpoint_authority,
                    parent,
                    child,
                    prior,
                )
            },
            |bytes, preimage, digest| cryptography.sign_observation(bytes, preimage, digest),
        ),
    };
    match prepared {
        Ok(prepared) => Ok(ResidentProgramPreparedConsultationStart {
            owner_instance,
            selector,
            checkpoint,
            prepared,
        }),
        Err(failure) => Err(Box::new(ResidentProgramConsultRosterPhase::Retry {
            owner_instance,
            selector,
            checkpoint,
            failure,
        })),
    }
}

#[allow(clippy::too_many_arguments)]
fn prepare_program_consult_child<
    WriterGuard,
    PeerWitness,
    Evidence,
    Clock,
    SessionAuthority,
    Signature,
    Random,
    OperationAuthority,
>(
    process: &mut ResidentProcessHost<'_, WriterGuard>,
    current: &mut ResidentProgramCurrentRequestAdmission<
        '_,
        PeerWitness,
        Evidence,
        Clock,
        SessionAuthority,
        Signature,
    >,
    deployment_authority: &ResidentProgramDeploymentAuthority,
    deployment: &mut ResidentProgramDeploymentOperationPorts<'_>,
    peer: &PeerWitness,
    random: &Random,
    operation_authority: &mut OperationAuthority,
    checkpoint_authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    parent: (
        &AuthorityStoreCoordinate,
        &ResidentRecordRefV1,
        &pale_core::records::OperationHandleV1,
        &pale_core::records::OperationLeaseV1,
    ),
    policy: &ResidentProcessConsultFreshChildPolicy,
    prior: Option<
        start_factories::ResidentConsultationChildAllocationFailure<
            ResidentProgramPreparedConsultChild,
            ResidentProgramConsultChildPreparationFailure,
        >,
    >,
) -> Result<
    start_factories::ResidentConsultationAllocatedChild<ResidentProgramPreparedConsultChild>,
    start_factories::ResidentConsultationChildAllocationFailure<
        ResidentProgramPreparedConsultChild,
        ResidentProgramConsultChildPreparationFailure,
    >,
>
where
    WriterGuard: PrincipalWriterGuardPort,
    Evidence: ResidentCurrentRequestEvidencePort<PeerWitness>,
    Clock: pale_ports::WallClockSourcePort,
    SessionAuthority: crate::resident::AmbientSessionAuthorityPort,
    Signature: ResidentProgramCallerSignaturePort,
    Random: CryptographicRandomSourcePort,
    OperationAuthority: crate::resident::AmbientSessionAuthorityPort,
{
    use start_factories::ResidentConsultationChildAllocationFailure::{Capture, Owner};

    let retained_child = match prior {
        Some(Capture(failure)) => Some(failure.child),
        Some(Owner(ResidentProgramConsultChildPreparationFailure::Admission(_)))
        | Some(Owner(ResidentProgramConsultChildPreparationFailure::Allocation(_))) => None,
        Some(Owner(ResidentProgramConsultChildPreparationFailure::Factory { child, .. })) => {
            Some(child)
        }
        Some(failure) => return Err(failure),
        None => None,
    };

    if !matches!(
        policy.operation_class,
        pale_core::records::OperationClassV1::Invoke
            | pale_core::records::OperationClassV1::Consult
    ) {
        return Err(Owner(
            ResidentProgramConsultChildPreparationFailure::UnsupportedClass,
        ));
    }

    let child = match retained_child {
        Some(child) => child,
        None => {
            let request_bytes = pale_core::canon::to_canonical_string(policy.request.as_ref())
                .map_err(|_| {
                    Owner(ResidentProgramConsultChildPreparationFailure::Admission(
                        ResidentOperationFrameFailure::InvalidRequest,
                    ))
                })?;
            let kind = match policy.operation_class {
                pale_core::records::OperationClassV1::Invoke => {
                    operation_request_host::ResidentOperationRequestKind::StartInvoke
                }
                pale_core::records::OperationClassV1::Consult => {
                    operation_request_host::ResidentOperationRequestKind::StartConsult
                }
                _ => unreachable!("unsupported child class returned above"),
            };
            let admitted = current
                .admit_bytes(
                    process,
                    std::slice::from_ref(deployment_authority),
                    peer,
                    request_bytes.as_bytes(),
                    ResidentProgramRequestPolicyKind::Operation(kind),
                    None,
                )
                .map_err(|reason| {
                    Owner(ResidentProgramConsultChildPreparationFailure::Admission(
                        reason,
                    ))
                })?;
            if admitted.selector() != &deployment.selector {
                return Err(Owner(
                    ResidentProgramConsultChildPreparationFailure::Admission(
                        ResidentOperationFrameFailure::ConfigurationUnavailable,
                    ),
                ));
            }
            let maximum_record_bytes = policy.start.maximum_record_bytes();
            let allocation = admitted.with_ambient_context(|ambient| {
                process.allocate_operation_start(
                    admitted.selector(),
                    ResidentDeploymentOperationLookup {
                        store: &deployment.store,
                        custody: &mut *deployment.custody,
                    },
                    ResidentProcessOperationStartContext {
                        session: admitted.session(),
                        current: &ambient,
                        request: admitted.request(),
                        handle_signing_authority: &admitted.selected.session_signing_authority,
                        semantic_checkpoint_authority: checkpoint_authority,
                        issuance_checkpoint_authority: checkpoint_authority,
                        maximum_record_bytes,
                    },
                    random,
                )
            });
            let fresh = match allocation {
                Ok(ResidentProcessOperationStartAllocation::Fresh(fresh)) => fresh,
                Ok(ResidentProcessOperationStartAllocation::Existing(existing)) => {
                    return Err(Owner(
                        ResidentProgramConsultChildPreparationFailure::Existing(existing),
                    ));
                }
                Err(reason) => {
                    return Err(Owner(
                        ResidentProgramConsultChildPreparationFailure::Allocation(reason),
                    ));
                }
            };
            let retained = admitted.with_ambient_context(|ambient| {
                process.admit_and_retain_fresh_operation_lease_with_lineage(
                    *fresh,
                    ResidentDeploymentOperationLeaseAccess {
                        store: &deployment.store,
                        custody: &mut *deployment.custody,
                    },
                    ResidentProcessOperationStartContext {
                        session: admitted.session(),
                        current: &ambient,
                        request: admitted.request(),
                        handle_signing_authority: &admitted.selected.session_signing_authority,
                        semantic_checkpoint_authority: checkpoint_authority,
                        issuance_checkpoint_authority: checkpoint_authority,
                        maximum_record_bytes,
                    },
                    operation_authority,
                    policy.lease.clone(),
                    ResidentProcessOperationLeaseLineage::DelegateChild(parent.3),
                )
            });
            match retained {
                Ok(pending) => ResidentProgramPreparedConsultChild {
                    admitted: Box::new(admitted),
                    pending: Box::new(pending),
                    policy: Box::new(policy.clone()),
                },
                Err(failure) => {
                    return Err(Owner(
                        ResidentProgramConsultChildPreparationFailure::Lease {
                            policy: Box::new(policy.clone()),
                            failure: Box::new(failure),
                        },
                    ));
                }
            }
        }
    };

    let plan = child.admitted.with_start_factory_basis(
        process,
        &child.admitted.selected.session_signing_authority,
        checkpoint_authority,
        |basis| {
            start_factories::with_start_consult_child_owner_input(
                &child.pending,
                basis,
                child.policy.start.clone(),
                child.policy.operation_class,
                parent,
                |input| crate::resident::consultation::ConsultationFreshChildPlan::capture(&input),
            )
        },
    );
    match plan {
        Err(_) => Err(Owner(
            ResidentProgramConsultChildPreparationFailure::Factory {
                child,
                reason: start_factories::ResidentOperationStartFactoryError::Association,
            },
        )),
        Ok(Err(reason)) => Err(Owner(
            ResidentProgramConsultChildPreparationFailure::Factory { child, reason },
        )),
        Ok(Ok(Err(reason))) => Err(Capture(
            start_factories::ResidentConsultationChildCaptureFailure { child, reason },
        )),
        Ok(Ok(Ok(plan))) => {
            Ok(start_factories::ResidentConsultationAllocatedChild::from_captured(child, plan))
        }
    }
}

/// Move a fully prepared Consult into the parent-custody state machine.
///
/// Changing the bounded private child representation preserves the original
/// prepared value on failure. A child-count mismatch is retained as invalid
/// state and cannot trigger reconstruction.
fn into_program_consult_parent(
    prepared: ResidentProgramPreparedConsultationStart,
) -> Result<ResidentProgramConsultParentPhase, Box<ResidentProgramPreparedConsultationStart>> {
    let ResidentProgramPreparedConsultationStart {
        owner_instance,
        selector,
        checkpoint,
        prepared,
    } = prepared;
    let prepared = match prepared.try_map_children(ResidentProgramConsultChildStart::Prepared) {
        Ok(prepared) => prepared,
        Err(prepared) => {
            return Err(Box::new(ResidentProgramPreparedConsultationStart {
                owner_instance,
                selector,
                checkpoint,
                prepared,
            }));
        }
    };
    match prepared.into_parent_commit() {
        Ok(pending) => Ok(ResidentProgramConsultParentPhase::Pending {
            owner_instance,
            selector,
            checkpoint,
            pending,
        }),
        Err(prepared) => Ok(ResidentProgramConsultParentPhase::Invalid {
            checkpoint,
            prepared,
        }),
    }
}

#[derive(Clone, Copy)]
enum ResidentProgramOperationStartContinuationKind {
    HandleIssued,
    SignedSemantic,
    Signed,
    PreparedCheckpoint,
    CheckpointIssuance,
    Physical,
    Unsupported,
}

fn resident_program_operation_start_continuation_kind(
    continuation: &ResidentProcessOperationStartContinuation,
) -> ResidentProgramOperationStartContinuationKind {
    match continuation.failure.as_ref() {
        ResidentLocalOperationStartFailure::HandleIssued(_) => {
            ResidentProgramOperationStartContinuationKind::HandleIssued
        }
        ResidentLocalOperationStartFailure::SignedSemanticStart(_) => {
            ResidentProgramOperationStartContinuationKind::SignedSemantic
        }
        ResidentLocalOperationStartFailure::SignedStart(_) => {
            ResidentProgramOperationStartContinuationKind::Signed
        }
        ResidentLocalOperationStartFailure::PreparedCheckpoint { .. } => {
            ResidentProgramOperationStartContinuationKind::PreparedCheckpoint
        }
        ResidentLocalOperationStartFailure::CheckpointIssuance { .. } => {
            ResidentProgramOperationStartContinuationKind::CheckpointIssuance
        }
        ResidentLocalOperationStartFailure::CheckpointAttachment(_)
        | ResidentLocalOperationStartFailure::Custody(_) => {
            ResidentProgramOperationStartContinuationKind::Physical
        }
        ResidentLocalOperationStartFailure::Preparation(_) => {
            ResidentProgramOperationStartContinuationKind::Unsupported
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn continue_program_operation_start<WriterGuard, OperationAuthority, Cryptography>(
    process: &mut ResidentProcessHost<'_, WriterGuard>,
    deployment: &mut ResidentProgramDeploymentOperationPorts<'_>,
    operation_authority: &mut OperationAuthority,
    cryptography: &Cryptography,
    checkpoint_authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    checkpoint: &ResidentProcessPreparedStartCheckpoint,
    continuation: ResidentProcessOperationStartContinuation,
) -> Result<crate::resident::IssuedLocalOperation, Box<ResidentProcessOperationStartContinuation>>
where
    OperationAuthority: crate::resident::AmbientSessionAuthorityPort,
    Cryptography: ResidentProgramInvokeCryptographyPort,
{
    let checkpoint_input = || ResidentLocalOperationCheckpointInput {
        id: checkpoint.checkpoint_id.clone(),
        reference: checkpoint.checkpoint_ref.clone(),
        schema: &checkpoint.checkpoint_schema,
        authority: checkpoint_authority,
    };
    let kind = resident_program_operation_start_continuation_kind(&continuation);
    let result = match kind {
        ResidentProgramOperationStartContinuationKind::HandleIssued => process
            .continue_handle_issued_operation_start(
                continuation,
                deployment.access(),
                operation_authority,
                |bytes, preimage, digest| cryptography.sign_observation(bytes, preimage, digest),
                |_, _, _| Ok(None),
                checkpoint.accumulator_limits,
                checkpoint_input(),
                |prepared| cryptography.prepare_checkpoint(prepared, &checkpoint.checkpoint_id),
                |bytes, preimage, digest| cryptography.sign_checkpoint(bytes, preimage, digest),
            ),
        ResidentProgramOperationStartContinuationKind::SignedSemantic => process
            .continue_signed_semantic_operation_start(
                continuation,
                deployment.access(),
                operation_authority,
                |_, _, _| Ok(None),
                checkpoint.accumulator_limits,
                checkpoint_input(),
                |prepared| cryptography.prepare_checkpoint(prepared, &checkpoint.checkpoint_id),
                |bytes, preimage, digest| cryptography.sign_checkpoint(bytes, preimage, digest),
            ),
        ResidentProgramOperationStartContinuationKind::Signed => process
            .continue_signed_operation_start(
                continuation,
                deployment.access(),
                operation_authority,
                checkpoint.accumulator_limits,
                checkpoint_input(),
                |prepared| cryptography.prepare_checkpoint(prepared, &checkpoint.checkpoint_id),
                |bytes, preimage, digest| cryptography.sign_checkpoint(bytes, preimage, digest),
            ),
        ResidentProgramOperationStartContinuationKind::PreparedCheckpoint => process
            .continue_prepared_checkpoint_operation_start(
                continuation,
                deployment.access(),
                operation_authority,
                checkpoint_input(),
                |prepared| cryptography.prepare_checkpoint(prepared, &checkpoint.checkpoint_id),
                |bytes, preimage, digest| cryptography.sign_checkpoint(bytes, preimage, digest),
            ),
        ResidentProgramOperationStartContinuationKind::CheckpointIssuance => process
            .continue_checkpoint_issuance_operation_start(
                continuation,
                deployment.access(),
                operation_authority,
                checkpoint_input(),
                |bytes, preimage, digest| cryptography.sign_checkpoint(bytes, preimage, digest),
            ),
        ResidentProgramOperationStartContinuationKind::Physical => {
            process.continue_operation_start(continuation, deployment.access(), operation_authority)
        }
        ResidentProgramOperationStartContinuationKind::Unsupported => {
            return Err(Box::new(continuation));
        }
    };
    result.map_err(|failure| failure.pending)
}

#[allow(clippy::too_many_arguments)]
fn advance_program_consult_parent<WriterGuard, OperationAuthority, Cryptography>(
    process: &mut ResidentProcessHost<'_, WriterGuard>,
    deployment: &mut ResidentProgramDeploymentOperationPorts<'_>,
    operation_authority: &mut OperationAuthority,
    cryptography: &Cryptography,
    checkpoint_authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    phase: ResidentProgramConsultParentPhase,
) -> Result<
    start_factories::ResidentConsultationChildAdvance<
        crate::resident::IssuedLocalOperation,
        ResidentProgramConsultChildStart,
        crate::resident::IssuedLocalOperation,
    >,
    Box<ResidentProgramConsultParentPhase>,
>
where
    OperationAuthority: crate::resident::AmbientSessionAuthorityPort,
    Cryptography: ResidentProgramInvokeCryptographyPort,
{
    let (owner_instance, selector, checkpoint, result) = match phase {
        ResidentProgramConsultParentPhase::Pending {
            owner_instance,
            selector,
            checkpoint,
            pending,
        } => {
            let result = pending.commit(|prepared| {
                process.commit_prepared_operation_start(
                    owner_instance.clone(),
                    selector.clone(),
                    deployment.access(),
                    operation_authority,
                    prepared,
                    checkpoint.accumulator_limits,
                    ResidentLocalOperationCheckpointInput {
                        id: checkpoint.checkpoint_id.clone(),
                        reference: checkpoint.checkpoint_ref.clone(),
                        schema: &checkpoint.checkpoint_schema,
                        authority: checkpoint_authority,
                    },
                    checkpoint.maximum_record_bytes,
                    |prepared| cryptography.prepare_checkpoint(prepared, &checkpoint.checkpoint_id),
                    |bytes, preimage, digest| cryptography.sign_checkpoint(bytes, preimage, digest),
                )
            });
            (owner_instance, selector, checkpoint, result)
        }
        ResidentProgramConsultParentPhase::Retry {
            owner_instance,
            selector,
            checkpoint,
            failure,
        } => {
            let result = failure.continue_commit(|input| match input {
                start_factories::ResidentConsultationParentCommitInput::Prepared(prepared) => {
                    process.commit_prepared_operation_start(
                        owner_instance.clone(),
                        selector.clone(),
                        deployment.access(),
                        operation_authority,
                        *prepared,
                        checkpoint.accumulator_limits,
                        ResidentLocalOperationCheckpointInput {
                            id: checkpoint.checkpoint_id.clone(),
                            reference: checkpoint.checkpoint_ref.clone(),
                            schema: &checkpoint.checkpoint_schema,
                            authority: checkpoint_authority,
                        },
                        checkpoint.maximum_record_bytes,
                        |prepared| {
                            cryptography.prepare_checkpoint(prepared, &checkpoint.checkpoint_id)
                        },
                        |bytes, preimage, digest| {
                            cryptography.sign_checkpoint(bytes, preimage, digest)
                        },
                    )
                }
                start_factories::ResidentConsultationParentCommitInput::Continue(failure) => {
                    match failure {
                        ResidentProcessPreparedOperationStartFailure::Prepared {
                            owner_instance,
                            selector,
                            prepared,
                        } => process.commit_prepared_operation_start(
                            owner_instance,
                            *selector,
                            deployment.access(),
                            operation_authority,
                            *prepared,
                            checkpoint.accumulator_limits,
                            ResidentLocalOperationCheckpointInput {
                                id: checkpoint.checkpoint_id.clone(),
                                reference: checkpoint.checkpoint_ref.clone(),
                                schema: &checkpoint.checkpoint_schema,
                                authority: checkpoint_authority,
                            },
                            checkpoint.maximum_record_bytes,
                            |prepared| {
                                cryptography.prepare_checkpoint(prepared, &checkpoint.checkpoint_id)
                            },
                            |bytes, preimage, digest| {
                                cryptography.sign_checkpoint(bytes, preimage, digest)
                            },
                        ),
                        ResidentProcessPreparedOperationStartFailure::Continuation(
                            continuation,
                        ) => continue_program_operation_start(
                            process,
                            deployment,
                            operation_authority,
                            cryptography,
                            checkpoint_authority,
                            &checkpoint,
                            *continuation,
                        )
                        .map_err(|continuation| {
                            ResidentProcessPreparedOperationStartFailure::Continuation(continuation)
                        }),
                    }
                }
            });
            (owner_instance, selector, checkpoint, result)
        }
        invalid @ ResidentProgramConsultParentPhase::Invalid { .. } => {
            return Err(Box::new(invalid))
        }
    };
    result.map_err(|failure| {
        Box::new(ResidentProgramConsultParentPhase::Retry {
            owner_instance,
            selector,
            checkpoint,
            failure,
        })
    })
}

fn retain_program_consult_child_advance(
    selector: ResidentDeploymentAuthoritySelector,
    store: AuthorityStoreCoordinate,
    advance: start_factories::ResidentConsultationChildAdvance<
        crate::resident::IssuedLocalOperation,
        ResidentProgramConsultChildStart,
        crate::resident::IssuedLocalOperation,
    >,
) -> ResidentProgramOperationStartPhase {
    match advance {
        start_factories::ResidentConsultationChildAdvance::Next(schedule) => {
            ResidentProgramOperationStartPhase::ConsultChildren(Box::new(
                ResidentProgramConsultChildSchedulePhase {
                    selector,
                    store,
                    schedule,
                },
            ))
        }
        start_factories::ResidentConsultationChildAdvance::InitializeCollection(initialization) => {
            let maximum_children = initialization.children.len();
            let started = consult_start::ResidentProcessConsultationStarted::from_started_parent(
                selector,
                store,
                initialization.parent,
                initialization.children,
                maximum_children,
            );
            match started.into_initialization() {
                consult_start::ResidentProcessConsultationInitialization::OpenCollection(
                    pending,
                ) => ResidentProgramOperationStartPhase::ConsultCollection(Box::new(pending)),
                consult_start::ResidentProcessConsultationInitialization::StartChild(_) => {
                    unreachable!("a complete child schedule cannot retain an unstarted child")
                }
            }
        }
    }
}

fn prepare_program_consult_child_parent(
    parent: &crate::resident::IssuedLocalOperation,
    deployment: &mut ResidentProgramDeploymentOperationPorts<'_>,
    restart: &ResidentProcessOperationRestartInput<'_>,
    recovery_admission: &mut OperationControlRecoveryAdmission<'_>,
) -> Result<crate::resident::PreparedLocalChildParent, ResidentOperationFrameFailure> {
    let recovered =
        recover_program_consult_parent(parent, deployment, restart, recovery_admission)?;
    let transaction = deployment
        .custody
        .prepare_transaction(&deployment.store)
        .map_err(|_| ResidentOperationFrameFailure::ResidentUnavailable)?;
    crate::resident::local_operation_start::prepare_local_child_parent_from_recovered(
        &deployment.store,
        transaction,
        recovered,
    )
    .map_err(|_| ResidentOperationFrameFailure::ResidentUnavailable)
}

fn recover_program_consult_parent(
    parent: &crate::resident::IssuedLocalOperation,
    deployment: &mut ResidentProgramDeploymentOperationPorts<'_>,
    restart: &ResidentProcessOperationRestartInput<'_>,
    recovery_admission: &mut OperationControlRecoveryAdmission<'_>,
) -> Result<crate::resident::RecoveredOperationControl, ResidentOperationFrameFailure> {
    let locator = operation_locator::ResidentProgramOperationLocator::from_issued(parent)
        .map_err(|_| ResidentOperationFrameFailure::ResidentUnavailable)?;
    let recovered = crate::resident::recover_operation_control(
        locator.recovery_input(
            restart.operation,
            restart.maximum_parent_depth,
            restart.current_head,
        ),
        &mut *deployment.custody,
        deployment.sessions,
        &mut *deployment.anchor,
        reborrow_operation_recovery_admission(recovery_admission),
    )
    .map_err(|_| ResidentOperationFrameFailure::ResidentUnavailable)?;
    locator
        .admit_recovered(&recovered)
        .map_err(|_| ResidentOperationFrameFailure::ResidentUnavailable)?;
    Ok(recovered)
}

enum ResidentProgramConsultCollectionAdvanceFailure {
    Pending(Box<consult_start::ResidentProcessConsultationCollectionPending>),
    Custody(consult_start::ResidentProcessConsultationOpenJourneyFailure),
}

#[allow(clippy::too_many_arguments)]
fn advance_program_consult_collection<WriterGuard, OperationAuthority, ConsultRecovery, StopInput>(
    process: &mut ResidentProcessHost<'_, WriterGuard>,
    deployment: &mut ResidentProgramDeploymentOperationPorts<'_>,
    operation_authority: &mut OperationAuthority,
    consult_recovery: &mut ConsultRecovery,
    restart: &ResidentProcessOperationRestartInput<'_>,
    recovery_admission: &mut OperationControlRecoveryAdmission<'_>,
    pending: consult_start::ResidentProcessConsultationCollectionPending,
) -> ResidentProgramOperationStartPhase
where
    WriterGuard: PrincipalWriterGuardPort,
    OperationAuthority: crate::resident::AmbientSessionAuthorityPort,
    ConsultRecovery: ResidentProgramConsultRecoveryInputPort<StopInput>,
{
    let parent_locator =
        match operation_locator::ResidentProgramOperationLocator::from_issued(pending.parent()) {
            Ok(locator) => locator,
            Err(_) => {
                return ResidentProgramOperationStartPhase::ConsultCollection(Box::new(pending));
            }
        };
    let parent_coordinate = parent_locator.coordinate().clone();
    let recovered = match recover_program_consult_parent(
        pending.parent(),
        deployment,
        restart,
        recovery_admission,
    ) {
        Ok(recovered) => recovered,
        Err(_) => {
            return ResidentProgramOperationStartPhase::ConsultCollection(Box::new(pending));
        }
    };
    let selector = deployment.selector.clone();
    let mut pending = Some(pending);
    let attempt =
        consult_recovery.use_initial_collection_inputs(&selector, &parent_coordinate, |inputs| {
            let pending = pending
                .take()
                .expect("the initial collection lender runs at most once");
            let (collection, limits) = match pending.admit_initial_collection() {
                Ok(admitted) => admitted,
                Err(_) => {
                    return Err(ResidentProgramConsultCollectionAdvanceFailure::Pending(
                        Box::new(pending),
                    ));
                }
            };
            let parent_observation = recovered.observation().selected();
            // Child starts advance the shared checkpoint while the parent's
            // observation remains at its original revision.
            let parent_checkpoint = recovered.observation().current_checkpoint();
            let input = consult_start::ResidentProcessConsultationCollectionInput {
                collection,
                parent_observation,
                parent_checkpoint,
                observation: crate::resident::ConsultationCollectionObservationInput {
                    observation_ref: inputs.observation_ref,
                    predecessor_collection: None,
                    observed_at: inputs.observed_at,
                    signing: inputs.signing,
                },
                limits,
                checkpoint: inputs.checkpoint,
            };
            pending
                .open(
                    process,
                    deployment.access(),
                    operation_authority,
                    input,
                    inputs.resolve_store,
                    |collection| (inputs.admit_collection)(collection),
                    |previous_ref, previous_digest, next_ref, next_digest| {
                        (inputs.admit_rotation)(
                            previous_ref,
                            previous_digest,
                            next_ref,
                            next_digest,
                        )
                    },
                    |bytes, preimage, digest| (inputs.sign_observation)(bytes, preimage, digest),
                    |bytes, preimage, digest| (inputs.sign_checkpoint)(bytes, preimage, digest),
                )
                .map_err(ResidentProgramConsultCollectionAdvanceFailure::Custody)
        });
    match attempt {
        Ok(Ok(opened)) => ResidentProgramOperationStartPhase::ConsultOpened(Box::new(
            ResidentProgramConsultOpenedPhase::Opened(opened),
        )),
        Ok(Err(ResidentProgramConsultCollectionAdvanceFailure::Pending(pending))) => {
            ResidentProgramOperationStartPhase::ConsultCollection(pending)
        }
        Ok(Err(ResidentProgramConsultCollectionAdvanceFailure::Custody(failure))) => {
            ResidentProgramOperationStartPhase::ConsultCollectionCustody(Box::new(failure))
        }
        Err(_) => ResidentProgramOperationStartPhase::ConsultCollection(Box::new(
            pending.expect("unavailable lender cannot consume the collection phase"),
        )),
    }
}

fn continue_program_consult_collection_custody<WriterGuard, OperationAuthority>(
    process: &mut ResidentProcessHost<'_, WriterGuard>,
    deployment: &mut ResidentProgramDeploymentOperationPorts<'_>,
    operation_authority: &mut OperationAuthority,
    failure: consult_start::ResidentProcessConsultationOpenJourneyFailure,
) -> ResidentProgramOperationStartPhase
where
    WriterGuard: PrincipalWriterGuardPort,
    OperationAuthority: crate::resident::AmbientSessionAuthorityPort,
{
    match failure.continue_custody(process, deployment.access(), operation_authority) {
        Ok(opened) => ResidentProgramOperationStartPhase::ConsultOpened(Box::new(
            ResidentProgramConsultOpenedPhase::Opened(opened),
        )),
        Err(failure) => {
            ResidentProgramOperationStartPhase::ConsultCollectionCustody(Box::new(failure))
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn advance_program_consult_opened<WriterGuard, OperationAuthority, ConsultRecovery, StopInput>(
    process: &mut ResidentProcessHost<'_, WriterGuard>,
    deployment: &mut ResidentProgramDeploymentOperationPorts<'_>,
    operation_authority: &mut OperationAuthority,
    consult_recovery: &mut ConsultRecovery,
    restart: &ResidentProcessOperationRestartInput<'_>,
    recovery_admission: &mut OperationControlRecoveryAdmission<'_>,
    now: pale_core::records::UnixTimeV1,
    phase: ResidentProgramConsultOpenedPhase,
) -> ResidentProgramConsultOpenedPhase
where
    WriterGuard: PrincipalWriterGuardPort,
    OperationAuthority: crate::resident::AmbientSessionAuthorityPort,
    ConsultRecovery: ResidentProgramConsultRecoveryInputPort<StopInput>,
{
    let mut recovered_collection = match phase {
        ResidentProgramConsultOpenedPhase::Ready(ready) => {
            return ResidentProgramConsultOpenedPhase::Ready(ready);
        }
        ResidentProgramConsultOpenedPhase::Recovered(recovered) => Some(recovered),
        ResidentProgramConsultOpenedPhase::Batch(failure) => Some(failure.into_recovered()),
        ResidentProgramConsultOpenedPhase::Opened(opened) => {
            return recover_program_consult_collection(
                process,
                deployment,
                operation_authority,
                consult_recovery,
                restart,
                recovery_admission,
                opened,
            );
        }
        ResidentProgramConsultOpenedPhase::Recovery(failure) => {
            return recover_program_consult_collection(
                process,
                deployment,
                operation_authority,
                consult_recovery,
                restart,
                recovery_admission,
                failure.into_opened(),
            );
        }
    };
    let attempt = consult_recovery.use_consult_inputs(
        |_, task_limits, _, _, _, _, _, _historical_collection| {
            let recovered = recovered_collection
                .take()
                .expect("the Consult input lender runs at most once");
            recovered.open_initial_batch(process, now, task_limits)
        },
    );
    match attempt {
        Ok(Ok(ready)) => ResidentProgramConsultOpenedPhase::Ready(ready),
        Ok(Err(failure)) => ResidentProgramConsultOpenedPhase::Batch(failure),
        Err(_) => ResidentProgramConsultOpenedPhase::Recovered(
            recovered_collection
                .expect("an unavailable lender cannot consume the recovered collection"),
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn recover_program_consult_collection<WriterGuard, OperationAuthority, ConsultRecovery, StopInput>(
    process: &mut ResidentProcessHost<'_, WriterGuard>,
    deployment: &mut ResidentProgramDeploymentOperationPorts<'_>,
    operation_authority: &mut OperationAuthority,
    consult_recovery: &mut ConsultRecovery,
    restart: &ResidentProcessOperationRestartInput<'_>,
    recovery_admission: &mut OperationControlRecoveryAdmission<'_>,
    opened: consult_start::ResidentProcessConsultationOpened,
) -> ResidentProgramConsultOpenedPhase
where
    WriterGuard: PrincipalWriterGuardPort,
    OperationAuthority: crate::resident::AmbientSessionAuthorityPort,
    ConsultRecovery: ResidentProgramConsultRecoveryInputPort<StopInput>,
{
    let recovered_parent = match recover_program_consult_parent(
        opened.parent(),
        deployment,
        restart,
        recovery_admission,
    ) {
        Ok(recovered) => recovered,
        Err(_) => return ResidentProgramConsultOpenedPhase::Opened(opened),
    };
    let completed_observation = opened.completed().observation();
    let recovered_observation = recovered_parent.observation().selected();
    if completed_observation.0 != recovered_observation.0
        || completed_observation
            .1
            .canonical_operation_observation_sha256()
            != recovered_observation
                .1
                .canonical_operation_observation_sha256()
    {
        return ResidentProgramConsultOpenedPhase::Opened(opened);
    }
    let mut opened = Some(opened);
    let attempt = consult_recovery.use_consult_inputs(
        |collection_limits, _, resolve_store, admit_collection, _, _, _, _historical_collection| {
            let opened = opened
                .take()
                .expect("the Consult input lender runs at most once");
            opened
                .recover(
                    process,
                    deployment.access(),
                    operation_authority,
                    recovered_parent.observation(),
                    collection_limits,
                    resolve_store,
                    |collection| admit_collection(collection),
                )
                .map(ResidentProgramConsultOpenedPhase::Recovered)
                .map_err(|failure| Box::new(ResidentProgramConsultOpenedPhase::Recovery(failure)))
        },
    );
    match attempt {
        Ok(Ok(phase)) => phase,
        Ok(Err(phase)) => *phase,
        Err(_) => ResidentProgramConsultOpenedPhase::Opened(
            opened.expect("an unavailable lender cannot consume the opened collection"),
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn advance_program_consult_child<WriterGuard, OperationAuthority, Cryptography>(
    process: &mut ResidentProcessHost<'_, WriterGuard>,
    deployment: &mut ResidentProgramDeploymentOperationPorts<'_>,
    operation_authority: &mut OperationAuthority,
    cryptography: &Cryptography,
    checkpoint_authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    restart: &ResidentProcessOperationRestartInput<'_>,
    recovery_admission: &mut OperationControlRecoveryAdmission<'_>,
    phase: ResidentProgramConsultChildSchedulePhase,
) -> Result<ResidentProgramOperationStartPhase, Box<ResidentProgramConsultChildSchedulePhase>>
where
    WriterGuard: PrincipalWriterGuardPort,
    OperationAuthority: crate::resident::AmbientSessionAuthorityPort,
    Cryptography: ResidentProgramInvokeCryptographyPort,
{
    let ResidentProgramConsultChildSchedulePhase {
        selector,
        store,
        schedule,
    } = phase;
    if deployment.selector != selector || deployment.store != store {
        return Err(Box::new(ResidentProgramConsultChildSchedulePhase {
            selector,
            store,
            schedule,
        }));
    }
    let advanced = schedule.advance_owned(|parent, child| match child {
        ResidentProgramConsultChildStart::Continuation(continuation) => {
            let ResidentProgramConsultChildContinuation {
                checkpoint,
                continuation,
            } = *continuation;
            continue_program_operation_start(
                process,
                deployment,
                operation_authority,
                cryptography,
                checkpoint_authority,
                &checkpoint,
                continuation,
            )
            .map_err(|continuation| {
                (
                    ResidentProgramConsultChildStart::Continuation(Box::new(
                        ResidentProgramConsultChildContinuation {
                            checkpoint,
                            continuation: *continuation,
                        },
                    )),
                    (),
                )
            })
        }
        ResidentProgramConsultChildStart::Prepared(child) => {
            let protected = match prepare_program_consult_child_parent(
                parent,
                deployment,
                restart,
                recovery_admission,
            ) {
                Ok(protected) => protected,
                Err(_) => return Err((ResidentProgramConsultChildStart::Prepared(child), ())),
            };
            let result = child.admitted.with_start_factory_basis(
                process,
                &child.admitted.selected.session_signing_authority,
                checkpoint_authority,
                |basis| {
                    start_factories::with_start_consult_child_owner_input(
                        &child.pending,
                        basis,
                        child.policy.start.clone(),
                        child.policy.operation_class,
                        (
                            &store,
                            parent.handle().0,
                            parent.handle().1,
                            parent.historical_operation_lease(),
                        ),
                        |input| {
                            let checkpoint = ResidentProcessPreparedStartCheckpoint {
                                accumulator_limits: input.accumulator_limits,
                                checkpoint_id: input.checkpoint.id.clone(),
                                checkpoint_ref: input.checkpoint.reference.clone(),
                                checkpoint_schema: input.checkpoint.schema.clone(),
                                maximum_record_bytes: input.maximum_record_bytes,
                            };
                            process
                                .start_consultation_child(
                                    &selector,
                                    deployment.access(),
                                    operation_authority,
                                    input,
                                    protected,
                                    |request,
                                     session,
                                     current,
                                     handle,
                                     lease,
                                     operation,
                                     publication,
                                     checkpoint| {
                                        cryptography.admit_compiled_authority(
                                            request,
                                            session,
                                            current,
                                            handle,
                                            lease,
                                            operation,
                                            publication,
                                            checkpoint,
                                        )
                                    },
                                    |bytes, preimage, digest| {
                                        cryptography.sign_handle(bytes, preimage, digest)
                                    },
                                    |bytes, preimage, digest| {
                                        cryptography.sign_observation(bytes, preimage, digest)
                                    },
                                    |prepared| {
                                        cryptography
                                            .prepare_checkpoint(prepared, &checkpoint.checkpoint_id)
                                    },
                                    |bytes, preimage, digest| {
                                        cryptography.sign_checkpoint(bytes, preimage, digest)
                                    },
                                )
                                .map_err(|reason| (Box::new(checkpoint), reason))
                        },
                    )
                },
            );
            match result {
                Ok(Ok(Ok(issued))) => Ok(issued),
                Ok(Ok(Err((
                    checkpoint,
                    ResidentProcessConsultStartError::Continuation(continuation),
                )))) => Err((
                    ResidentProgramConsultChildStart::Continuation(Box::new(
                        ResidentProgramConsultChildContinuation {
                            checkpoint: *checkpoint,
                            continuation: *continuation,
                        },
                    )),
                    (),
                )),
                Ok(Ok(Err((
                    _,
                    ResidentProcessConsultStartError::Selection(_)
                    | ResidentProcessConsultStartError::Preparation(_),
                ))))
                | Ok(Err(_))
                | Err(_) => Err((ResidentProgramConsultChildStart::Prepared(child), ())),
            }
        }
    });
    match advanced {
        Ok(advance) => Ok(retain_program_consult_child_advance(
            selector, store, advance,
        )),
        Err(failure) => Err(Box::new(ResidentProgramConsultChildSchedulePhase {
            selector,
            store,
            schedule: failure.schedule,
        })),
    }
}

/// Result of one caller-driven retained-phase opportunity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::resident) enum ResidentProgramRetainedPhaseTurn {
    /// No interrupted phase is ready to advance.
    Idle,
    /// One exact retained phase advanced; the final result can remain unavailable.
    Advanced,
    /// A required owner was unavailable; the exact phase remains retained.
    Deferred,
}

impl<
        'historical,
        'ports,
        PeerWitness,
        Peer,
        Evidence,
        Clock,
        SessionAuthority,
        Signature,
        Random,
        OperationAuthority,
        Cryptography,
        Recovery,
        ConsultRecovery,
    >
    ResidentProgramOperationTurnOwner<
        'historical,
        'ports,
        PeerWitness,
        Peer,
        Evidence,
        Clock,
        SessionAuthority,
        Signature,
        Random,
        OperationAuthority,
        Cryptography,
        Recovery,
        ConsultRecovery,
    >
{
    /// Retain exact application policy, physical ports, and authority owners.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn new(
        peer: Peer,
        current: ResidentProgramCurrentRequestAdmission<
            'historical,
            PeerWitness,
            Evidence,
            Clock,
            SessionAuthority,
            Signature,
        >,
        deployments: Vec<ResidentProgramDeploymentOperationPorts<'ports>>,
        random: Random,
        operation_authority: OperationAuthority,
        cryptography: Cryptography,
        recovery: Recovery,
        consult_recovery: ConsultRecovery,
    ) -> Result<Self, ResidentProgramCurrentRequestBuildError> {
        if deployments.is_empty() {
            return Err(ResidentProgramCurrentRequestBuildError::Bounds);
        }
        for (index, deployment) in deployments.iter().enumerate() {
            if deployments[..index]
                .iter()
                .any(|existing| existing.selector == deployment.selector)
            {
                return Err(ResidentProgramCurrentRequestBuildError::DuplicatePolicy);
            }
        }
        Ok(Self {
            peer,
            current,
            deployments: deployments.into_boxed_slice(),
            random,
            operation_authority,
            cryptography,
            recovery,
            consult_recovery,
            admitted: None,
            admitted_peer: None,
            pending_consult_peer: None,
            native_hooks: None,
            retained_start_exchange: None,
            next_retained_phase_lane: ResidentProgramRetainedPhaseLane::Start,
            next_recovery_phase: ResidentProgramRecoveryPhaseLane::Invoke,
        })
    }

    fn selected_ports(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
    ) -> Result<&mut ResidentProgramDeploymentOperationPorts<'ports>, ResidentOperationFrameFailure>
    {
        let mut matches = self
            .deployments
            .iter_mut()
            .filter(|deployment| deployment.matches(selector));
        let selected = matches
            .next()
            .ok_or(ResidentOperationFrameFailure::ConfigurationUnavailable)?;
        if matches.next().is_some() {
            return Err(ResidentOperationFrameFailure::ConfigurationUnavailable);
        }
        Ok(selected)
    }
}

/// Concrete application-owned owner for one exact root Invoke turn.
///
/// This type retains the admitted request, session/generation input, exact
/// deployment access, and physical authority ports. Its semantic method calls
/// [`ResidentProcessHost::invoke_request`] directly. It does not expose a
/// callback that can replace request admission, operation selection, policy,
/// replay handling, allocation, custody, or response projection.
pub(in crate::resident) struct ResidentProgramInvokeTurnOwner<
    'root,
    'restart,
    'admission,
    'ports,
    Peer,
    Random,
    Authority,
    Cryptography,
> {
    peer: Peer,
    request: CommonRequestEnvelopeV1,
    root: Option<ResidentProcessRootInput<'root>>,
    restart: Option<ResidentProcessOperationRestartInput<'restart>>,
    admission: Option<OperationControlRecoveryAdmission<'admission>>,
    selector: ResidentDeploymentAuthoritySelector,
    store: pale_ports::AuthorityStoreCoordinate,
    custody: &'ports mut dyn OperationCustodyPort,
    sessions: &'ports dyn SessionLeaseCustodyPort,
    anchor: &'ports mut dyn AntiRollbackAnchorPort,
    random: Random,
    authority: Authority,
    cryptography: Cryptography,
    pending_start: Option<ResidentProcessOperationStartContinuation>,
}

impl<'root, 'restart, 'admission, 'ports, Peer, Random, Authority, Cryptography>
    ResidentProgramInvokeTurnOwner<
        'root,
        'restart,
        'admission,
        'ports,
        Peer,
        Random,
        Authority,
        Cryptography,
    >
{
    /// Retain one already admitted root-Invoke application input.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn new(
        peer: Peer,
        request: CommonRequestEnvelopeV1,
        root: ResidentProcessRootInput<'root>,
        restart: ResidentProcessOperationRestartInput<'restart>,
        admission: OperationControlRecoveryAdmission<'admission>,
        selector: ResidentDeploymentAuthoritySelector,
        store: pale_ports::AuthorityStoreCoordinate,
        custody: &'ports mut dyn OperationCustodyPort,
        sessions: &'ports dyn SessionLeaseCustodyPort,
        anchor: &'ports mut dyn AntiRollbackAnchorPort,
        random: Random,
        authority: Authority,
        cryptography: Cryptography,
    ) -> Self {
        Self {
            peer,
            request,
            root: Some(root),
            restart: Some(restart),
            admission: Some(admission),
            selector,
            store,
            custody,
            sessions,
            anchor,
            random,
            authority,
            cryptography,
            pending_start: None,
        }
    }
}

/// Retained application owner for authenticated turn inputs and recovery.
///
/// Implementations live in the application composition root. They retain the
/// exact store-bound custody, session, anchor, signer, authority, replay, and
/// disclosure owners needed by the existing process-host journeys. Methods may
/// call only those existing journeys. They must not select a sole deployment,
/// retry an uncertain effect, or manufacture policy from a frame tag.
pub(in crate::resident) trait ResidentProgramTurnOwner<WriterGuard, PolicyEvaluator, StopInput> {
    /// Peer-admission witness retained until semantic resolution completes.
    type PeerWitness;

    /// Prepare and durably dispatch the response for one authenticated hook,
    /// rechecking current permission, disclosure, custody and expiry before return.
    /// The returned frame still requires independent caller-side admission;
    /// returning it does not establish delivery or native host execution.
    fn admit_native_hook_turn(
        &mut self,
        _process: &mut ResidentProcessHost<'_, WriterGuard>,
        _deployment_authorities: &[ResidentProgramDeploymentAuthority],
        _peer: LocalIpcPeerCredentials,
        _request: &crate::resident::AuthenticatedResidentHookRequest,
    ) -> Result<pale_ports::ResidentHookFrameResponse, ResidentOperationFrameFailure> {
        Err(ResidentOperationFrameFailure::ResidentUnavailable)
    }

    /// Admit supplementary local peer evidence without reading protected state.
    fn admit_operation_peer(
        &mut self,
        peer: LocalIpcPeerCredentials,
        request: &ResidentOperationFrameRequest,
    ) -> Result<Self::PeerWitness, InvalidResidentRecordV1>;

    /// Resolve and contextually admit the signed request after peer admission.
    fn resolve_operation<'store>(
        &mut self,
        process: &mut ResidentProcessHost<'store, WriterGuard>,
        policy: &mut PolicyEvaluator,
        deployment_authorities: &[ResidentProgramDeploymentAuthority],
        peer: &Self::PeerWitness,
        authenticated_key: Option<[u8; 32]>,
        request: &ResidentOperationFrameRequest,
    ) -> Result<ResidentProgramOperationResolution, ResidentOperationFrameFailure>;

    /// Execute exactly one already-resolved existing process-host journey.
    fn answer_operation<'store>(
        &mut self,
        process: &mut ResidentProcessHost<'store, WriterGuard>,
        policy: &mut PolicyEvaluator,
        resolution: &ResidentProgramOperationResolution,
    ) -> Result<
        operation_request_host::ResidentOperationRequestOutcome,
        ResidentOperationFrameFailure,
    >;

    /// Whether this owner retains one runnable operation phase.
    fn has_retained_phase(&self) -> bool {
        false
    }

    /// Advance at most one exact retained operation phase.
    ///
    /// The default owner has no such phase. An owner that returns `Deferred`
    /// must retain the same linear continuation unchanged or at its exact next
    /// phase; it must not allocate another identity or replay external work.
    fn continue_retained_phase<'store>(
        &mut self,
        _process: &mut ResidentProcessHost<'store, WriterGuard>,
        _policy: &mut PolicyEvaluator,
        _deployment_authorities: &[ResidentProgramDeploymentAuthority],
    ) -> ResidentProgramRetainedPhaseTurn {
        ResidentProgramRetainedPhaseTurn::Idle
    }

    /// Advance one exact root-provider recovery source to a safe handoff.
    fn recover_root_provider<'store>(
        &mut self,
        process: &mut ResidentProcessHost<'store, WriterGuard>,
        selector: &ResidentDeploymentAuthoritySelector,
    ) -> Result<(), ResidentProgramTurnError>;

    /// Advance one exact root-Consult recovery source to a safe handoff.
    fn recover_root_consult<'store>(
        &mut self,
        process: &mut ResidentProcessHost<'store, WriterGuard>,
        policy: &mut PolicyEvaluator,
        selector: &ResidentDeploymentAuthoritySelector,
    ) -> Result<(), ResidentProgramTurnError>;

    /// Advance one exact Consult-child recovery source to a safe handoff.
    fn recover_consult_child<'store>(
        &mut self,
        process: &mut ResidentProcessHost<'store, WriterGuard>,
        policy: &mut PolicyEvaluator,
        selector: &ResidentDeploymentAuthoritySelector,
    ) -> Result<(), ResidentProgramTurnError>;

    /// Advance one exact pairwise-Delegate recovery source to a safe handoff.
    fn recover_delegate<'store>(
        &mut self,
        process: &mut ResidentProcessHost<'store, WriterGuard>,
        selector: &ResidentDeploymentAuthoritySelector,
    ) -> Result<(), ResidentProgramTurnError>;

    /// Advance one bounded Event-publication stage.
    fn recover_publication<'store>(
        &mut self,
        _process: &mut ResidentProcessHost<'store, WriterGuard>,
        _selector: &ResidentDeploymentAuthoritySelector,
    ) -> Result<(), ResidentProgramTurnError> {
        Err(ResidentProgramTurnError::RecoveryUnavailable)
    }
}

/// Continue one admitted, point-addressed root Consult without constructing
/// any operation-start context.
///
/// The request has already passed peer, signature, deployment, signed-target,
/// and locator-syntax admission. This owner admits the locator against recovered
/// evidence, runs one bounded Consult turn, and retains the exact keyed result.
#[allow(clippy::too_many_arguments)]
fn answer_advance_consult_request<
    WriterGuard,
    PolicyEvaluator,
    StopInput,
    OperationAuthority,
    Recovery,
    ConsultRecovery,
>(
    process: &mut ResidentProcessHost<'_, WriterGuard>,
    policy: &mut PolicyEvaluator,
    selector: &ResidentDeploymentAuthoritySelector,
    selected: &mut ResidentProgramDeploymentOperationPorts<'_>,
    operation_authority: &mut OperationAuthority,
    recovery: &mut Recovery,
    consult_recovery: &mut ConsultRecovery,
    request: &CommonRequestEnvelopeV1,
    locator: &operation_locator::ResidentProgramOperationLocator,
    pending_locator: ResidentOperationFrameLocator,
    observed_at: pale_core::records::UnixTimeV1,
) -> Result<operation_request_host::ResidentOperationRequestOutcome, ResidentOperationFrameFailure>
where
    WriterGuard: PrincipalWriterGuardPort,
    PolicyEvaluator: consult_recovery::ResidentConsultationPolicyEvaluator<StopInput>,
    StopInput: ResidentProgramConsultStopInput,
    OperationAuthority: crate::resident::AmbientSessionAuthorityPort,
    Recovery: ResidentProgramRootRecoveryInputPort,
    ConsultRecovery: ResidentProgramConsultRecoveryInputPort<StopInput>,
{
    recovery.with_root_recovery(
        selector,
        request,
        Some(locator),
        |restart, mut recovery_admission, _, _| {
            let ResidentDeploymentOperationAccess {
                store,
                custody,
                sessions,
                anchor,
            } = selected.access();
            let operation_context = restart.operation;
            let maximum_parent_depth = restart.maximum_parent_depth;
            let recovered = crate::resident::recover_operation_control(
                locator.recovery_input(
                    operation_context,
                    maximum_parent_depth,
                    restart.current_head,
                ),
                &mut *custody,
                sessions,
                &mut *anchor,
                reborrow_operation_recovery_admission(&mut recovery_admission),
            )
            .map_err(|_| ResidentOperationFrameFailure::ResidentUnavailable)?;
            locator
                .admit_recovered(&recovered)
                .map_err(|_| ResidentOperationFrameFailure::InvalidRequest)?;
            let records = operation_request_host::recovered_control_records(&recovered)
                .map_err(|_| ResidentOperationFrameFailure::ResidentUnavailable)?;
            let issued = recovered.operation().operation();
            if issued.handle().1.operation_class()
                != &pale_core::records::OperationClassV1::Consult
                || issued.handle().1.parent_operation().is_some()
            {
                return Err(ResidentOperationFrameFailure::InvalidRequest);
            }
            let consult_turn_key = ResidentProgramConsultTurnKey::new(
                selector.clone(),
                locator.coordinate().clone(),
            );
            if consult_recovery.has_retained_consult_key(&consult_turn_key) {
                return Ok(
                    operation_request_host::ResidentOperationRequestOutcome::Pending(
                        pending_locator,
                    ),
                );
            }
            if !consult_recovery.can_retain_consult_key(&consult_turn_key) {
                return Err(ResidentOperationFrameFailure::ResidentUnavailable);
            }
            let original_request = recovered.operation().request().clone();
            let disposition = process
                .classify_started_operation(
                    selector,
                    issued,
                    &original_request,
                    ResidentDeploymentOperationAccess {
                        store,
                        custody: &mut *custody,
                        sessions,
                        anchor: &mut *anchor,
                    },
                    restart,
                    reborrow_operation_recovery_admission(&mut recovery_admission),
                )
                .map_err(|_| ResidentOperationFrameFailure::ResidentUnavailable)?;
            locator
                .admit_restart_disposition(&disposition)
                .map_err(|_| ResidentOperationFrameFailure::InvalidRequest)?;
            match disposition {
                ResidentProcessOperationRestartDisposition::Terminal(_) => Ok(
                    operation_request_host::ResidentOperationRequestOutcome::Observed(records),
                ),
                ResidentProcessOperationRestartDisposition::RecoveryOnly(start) => {
                    retain_point_addressed_consult_turn(
                        consult_recovery,
                        consult_turn_key,
                        ResidentProgramConsultTurn::PrePolicy(
                            consult_recovery::ResidentProcessConsultRecoveryTurn::Pending(
                                Box::new(
                                    consult_recovery::ResidentProcessConsultRecoveryPending::RecoveryOnly(
                                        start,
                                    ),
                                ),
                            ),
                        ),
                    );
                    Ok(
                        operation_request_host::ResidentOperationRequestOutcome::Pending(
                            pending_locator,
                        ),
                    )
                }
                ResidentProcessOperationRestartDisposition::LocalCancellationRequested(
                    recovered,
                ) => {
                    retain_point_addressed_consult_turn(
                        consult_recovery,
                        consult_turn_key,
                        ResidentProgramConsultTurn::PrePolicy(
                            consult_recovery::ResidentProcessConsultRecoveryTurn::Pending(
                                Box::new(
                                    consult_recovery::ResidentProcessConsultRecoveryPending::LocalCancellation(
                                        recovered,
                                    ),
                                ),
                            ),
                        ),
                    );
                    Ok(
                        operation_request_host::ResidentOperationRequestOutcome::Pending(
                            pending_locator,
                        ),
                    )
                }
                ResidentProcessOperationRestartDisposition::Consult(start) => {
                    let restart::ResidentProcessRecoveredOperationStart {
                        selector: recovered_selector,
                        control,
                        generation_digest: _,
                        lineage: _,
                    } = *start;
                    if recovered_selector != *selector {
                        return Err(ResidentOperationFrameFailure::InvalidRequest);
                    }
                    let (parent, protected) = control.into_parts();
                    let (outcome, retained_turn) = consult_recovery.use_consult_inputs(
                        |collection_limits,
                         task_limits,
                         resolve_store,
                         admit_collection,
                         executor,
                         recover_child,
                         retain_child, _historical_collection| {
                            let mut retained_turn = None;
                            let outcome = (|| {
                                let started = process
                                    .rebuild_consultation_start_from_protected_intent(
                                        selector,
                                        store,
                                        parent.into_operation(),
                                        operation_context,
                                        maximum_parent_depth,
                                        &mut *custody,
                                        sessions,
                                        &mut *anchor,
                                        reborrow_operation_recovery_admission(
                                            &mut recovery_admission,
                                        ),
                                    )
                                    .map_err(|_| {
                                        ResidentOperationFrameFailure::ResidentUnavailable
                                    })?;
                                let current = process
                                    .recover_consultation_collection(
                                        selector,
                                        ResidentDeploymentOperationAccess {
                                            store,
                                            custody: &mut *custody,
                                            sessions,
                                            anchor: &mut *anchor,
                                        },
                                        operation_authority,
                                        started.parent(),
                                        &protected,
                                        collection_limits,
                                        resolve_store,
                                        |collection| admit_collection(collection),
                                    )
                                    .map_err(|_| {
                                        ResidentOperationFrameFailure::ResidentUnavailable
                                    })?
                                    .ok_or(
                                        ResidentOperationFrameFailure::ResidentUnavailable,
                                    )?;
                                if !started.matches_collection(current.collection()) {
                                    return Err(ResidentOperationFrameFailure::InvalidRequest);
                                }
                                let turn = process
                                    .advance_recovered_consultation_pre_policy(
                                        started,
                                        task_limits,
                                        observed_at,
                                        current,
                                        executor,
                                    )
                                    .map_err(|_| {
                                        ResidentOperationFrameFailure::ResidentUnavailable
                                    })?;
                                let turn = match turn {
                                    consult_recovery::ResidentProcessConsultRecoveryTurn::PolicyRequired(
                                        handoff,
                                    ) => {
                                        let (handoff, decision) = match process
                                            .evaluate_recovered_consultation_policy(
                                                handoff, policy,
                                            )
                                        {
                                            Ok(evaluated) => evaluated,
                                            Err(failure) => {
                                                let (handoff, _) = failure.into_parts();
                                                retained_turn = Some(
                                                    ResidentProgramConsultTurn::PrePolicy(
                                                        consult_recovery::ResidentProcessConsultRecoveryTurn::PolicyRequired(handoff),
                                                    ),
                                                );
                                                return Err(
                                                    ResidentOperationFrameFailure::ConfigurationUnavailable,
                                                );
                                            }
                                        };
                                        match process.continue_recovered_consultation_policy(
                                            selector,
                                            ResidentDeploymentOperationAccess {
                                                store,
                                                custody: &mut *custody,
                                                sessions,
                                                anchor: &mut *anchor,
                                            },
                                            handoff,
                                            decision,
                                            executor,
                                            |candidate| recover_child(candidate),
                                            |candidate, child, custody, anchor| {
                                                retain_child(candidate, child, custody, anchor)
                                            },
                                        ) {
                                            Ok(continuation) => {
                                                ResidentProgramConsultTurn::Policy(Box::new(
                                                    continuation,
                                                ))
                                            }
                                            Err(refusal) => {
                                                ResidentProgramConsultTurn::PolicyRefusal(
                                                    Box::new(refusal),
                                                )
                                            }
                                        }
                                    }
                                    other => ResidentProgramConsultTurn::PrePolicy(other),
                                };
                                retained_turn = Some(turn);
                                Ok(
                                    operation_request_host::ResidentOperationRequestOutcome::Pending(
                                        pending_locator,
                                    ),
                                )
                            })();
                            (outcome, retained_turn)
                        },
                    )?;
                    if let Some(turn) = retained_turn {
                        retain_point_addressed_consult_turn(
                            consult_recovery,
                            consult_turn_key,
                            turn,
                        );
                    }
                    outcome
                }
                ResidentProcessOperationRestartDisposition::NativeHookResponse(_)
                | ResidentProcessOperationRestartDisposition::OperationResolution(_)
                | ResidentProcessOperationRestartDisposition::ProviderPrepared(_)
                | ResidentProcessOperationRestartDisposition::ProviderResolved(_)
                | ResidentProcessOperationRestartDisposition::Provider(_)
                | ResidentProcessOperationRestartDisposition::NativePrepared(_)
                | ResidentProcessOperationRestartDisposition::Native(_)
                | ResidentProcessOperationRestartDisposition::A2aDelegateSendRecoveryOnly(_)
                | ResidentProcessOperationRestartDisposition::A2aTask(_)
                | ResidentProcessOperationRestartDisposition::A2aCancelTaskRequestRecoveryOnly(
                    _,
                ) => Err(ResidentOperationFrameFailure::InvalidRequest),
            }
        },
    )?
}

impl<
        'historical,
        'ports,
        WriterGuard,
        PolicyEvaluator,
        StopInput,
        PeerWitness,
        Peer,
        Evidence,
        Clock,
        SessionAuthority,
        Signature,
        Random,
        OperationAuthority,
        Cryptography,
        Recovery,
        ConsultRecovery,
    > ResidentProgramTurnOwner<WriterGuard, PolicyEvaluator, StopInput>
    for ResidentProgramOperationTurnOwner<
        'historical,
        'ports,
        PeerWitness,
        Peer,
        Evidence,
        Clock,
        SessionAuthority,
        Signature,
        Random,
        OperationAuthority,
        Cryptography,
        Recovery,
        ConsultRecovery,
    >
where
    WriterGuard: PrincipalWriterGuardPort,
    Peer: ResidentProgramPeerAdmissionPort<Witness = PeerWitness>
        + ResidentProgramObservePeerAdmissionPort,
    PeerWitness: Clone,
    Evidence: ResidentCurrentRequestEvidencePort<PeerWitness>,
    Clock: pale_ports::WallClockSourcePort,
    SessionAuthority: crate::resident::AmbientSessionAuthorityPort,
    Signature: ResidentProgramCallerSignaturePort,
    Random: CryptographicRandomSourcePort,
    OperationAuthority: crate::resident::AmbientSessionAuthorityPort,
    Cryptography: ResidentProgramInvokeCryptographyPort,
    Recovery: ResidentProgramRootRecoveryInputPort,
    Recovery: operation_recovery_sweep::ResidentProcessRootProviderRecoveryOwner<
        WriterGuard,
        Output = operation_request_host::ResidentOperationRequestOutcome,
    >,
    Recovery: delegate_recovery_sweep::ResidentProcessDelegateRecoveryOwner<WriterGuard>,
    Recovery: ResidentProgramCancellationInputPort,
    ConsultRecovery: ResidentProgramConsultRecoveryInputPort<StopInput>,
    PolicyEvaluator: ResidentProcessInvokePolicyEvaluator
        + ResidentProcessConsultPolicyEvaluator
        + ResidentProcessDelegatePolicyEvaluator
        + consult_recovery::ResidentConsultationPolicyEvaluator<StopInput>,
    StopInput: ResidentProgramConsultStopInput,
{
    type PeerWitness = PeerWitness;

    fn admit_native_hook_turn(
        &mut self,
        process: &mut ResidentProcessHost<'_, WriterGuard>,
        deployment_authorities: &[ResidentProgramDeploymentAuthority],
        peer: LocalIpcPeerCredentials,
        request: &crate::resident::AuthenticatedResidentHookRequest,
    ) -> Result<pale_ports::ResidentHookFrameResponse, ResidentOperationFrameFailure> {
        native_hook_turn::admit_hook_turn(self, process, deployment_authorities, peer, request)
    }

    fn admit_operation_peer(
        &mut self,
        peer: LocalIpcPeerCredentials,
        request: &ResidentOperationFrameRequest,
    ) -> Result<Self::PeerWitness, InvalidResidentRecordV1> {
        self.peer.admit_peer(peer, request)
    }

    fn resolve_operation<'store>(
        &mut self,
        process: &mut ResidentProcessHost<'store, WriterGuard>,
        _: &mut PolicyEvaluator,
        deployment_authorities: &[ResidentProgramDeploymentAuthority],
        peer: &Self::PeerWitness,
        authenticated_key: Option<[u8; 32]>,
        frame: &ResidentOperationFrameRequest,
    ) -> Result<ResidentProgramOperationResolution, ResidentOperationFrameFailure> {
        if self.admitted.is_some() {
            return Err(ResidentOperationFrameFailure::ReplayConflict);
        }
        let admitted = self
            .current
            .admit(process, deployment_authorities, peer, frame)?;
        let (kind, locator) = operation_frame_kind_and_locator(frame);
        let resolution = ResidentProgramOperationResolution::admit(
            frame,
            admitted.request().clone(),
            kind,
            locator,
        )?;
        if admitted.request().body().schema_id.as_str()
            == pale_core::records::NATIVE_TOOL_REQUEST_V1_SCHEMA_ID
        {
            if kind != operation_request_host::ResidentOperationRequestKind::StartInvoke {
                return Err(ResidentOperationFrameFailure::InvalidRequest);
            }
            let key = authenticated_key.ok_or(ResidentOperationFrameFailure::AdmissionDenied)?;
            self.current
                .signature
                .admit_native_caller_key(admitted.request().caller(), &key)
                .map_err(|_| ResidentOperationFrameFailure::AdmissionDenied)?;
            let candidate =
                crate::resident::ResidentNativeToolStart::admit(admitted.request().body())
                    .map_err(|_| ResidentOperationFrameFailure::InvalidRequest)?;
            let native_hooks = self.native_hooks.get_or_insert_with(|| {
                native_hook_start::NativeHookStarts::new(
                    process
                        .operations
                        .aggregate_task_limits()
                        .maximum_outstanding(),
                )
            });
            admitted.with_native_context(process, self.current.historical, |context| {
                native_hooks.reserve(
                    admitted.request(),
                    candidate,
                    key,
                    context,
                    &mut self.operation_authority,
                )
            })??;
        }
        self.admitted_peer = Some(peer.clone());
        self.admitted = Some(Box::new(admitted));
        Ok(resolution)
    }

    fn answer_operation<'store>(
        &mut self,
        process: &mut ResidentProcessHost<'store, WriterGuard>,
        policy: &mut PolicyEvaluator,
        resolution: &ResidentProgramOperationResolution,
    ) -> Result<
        operation_request_host::ResidentOperationRequestOutcome,
        ResidentOperationFrameFailure,
    > {
        let admitted = self
            .admitted
            .take()
            .ok_or(ResidentOperationFrameFailure::ReplayConflict)?;
        if admitted.request() != resolution.request() {
            return Err(ResidentOperationFrameFailure::InvalidRequest);
        }
        let admitted_peer = self
            .admitted_peer
            .take()
            .ok_or(ResidentOperationFrameFailure::ReplayConflict)?;
        let selector = admitted.selector().clone();
        let deployment_authority = admitted.selected.authority.clone();
        let handle_authority = admitted.selected.session_signing_authority.clone();
        let Self {
            current: current_admission,
            native_hooks,
            deployments,
            random,
            operation_authority,
            cryptography,
            recovery,
            consult_recovery,
            pending_consult_peer,
            retained_start_exchange,
            ..
        } = self;
        let mut matching = deployments
            .iter_mut()
            .filter(|deployment| deployment.matches(&selector));
        let selected = matching
            .next()
            .ok_or(ResidentOperationFrameFailure::ConfigurationUnavailable)?;
        if matching.next().is_some() {
            return Err(ResidentOperationFrameFailure::ConfigurationUnavailable);
        }
        let request = admitted.request().clone();
        let cryptography = &*cryptography;
        let operation_locator = resolution
            .locator()
            .map(operation_locator::ResidentProgramOperationLocator::parse)
            .transpose()
            .map_err(|_| ResidentOperationFrameFailure::InvalidRequest)?;
        if resolution.kind() == operation_request_host::ResidentOperationRequestKind::AdvanceConsult
        {
            let locator = operation_locator
                .as_ref()
                .ok_or(ResidentOperationFrameFailure::InvalidRequest)?;
            let pending_locator = resolution
                .locator()
                .cloned()
                .ok_or(ResidentOperationFrameFailure::InvalidRequest)?;
            return answer_advance_consult_request(
                process,
                policy,
                &selector,
                selected,
                operation_authority,
                recovery,
                consult_recovery,
                &request,
                locator,
                pending_locator,
                admitted.now,
            );
        }
        let mut retained_consult_phase = None;
        let mut retained_consult_parent = None;
        let mut retained_consult_children = None;
        let mut retained_consult_collection = None;
        let mut retained_consult_collection_custody = None;
        let mut retained_consult_opened = None;
        let mut consult_peer = admitted_peer;
        if let Some(pending) = retained_start_exchange.take() {
            let ResidentProgramRetainedStartExchange {
                kind,
                selector: pending_selector,
                request: pending_request,
                locator,
                phase,
            } = *pending;
            let same_start = kind == resolution.kind()
                && pending_selector == selector
                && pending_request == request
                && locator.as_ref() == resolution.locator();
            if !same_start {
                let replay_conflict = pending_selector == selector
                    && pending_request != request
                    && same_operation_request_identity(&pending_request, &request)?;
                *retained_start_exchange = Some(Box::new(ResidentProgramRetainedStartExchange {
                    kind,
                    selector: pending_selector,
                    request: pending_request,
                    locator,
                    phase,
                }));
                if matches!(
                    resolution.kind(),
                    operation_request_host::ResidentOperationRequestKind::StartInvoke
                        | operation_request_host::ResidentOperationRequestKind::StartConsult
                        | operation_request_host::ResidentOperationRequestKind::StartDelegate
                ) {
                    return Err(if replay_conflict {
                        ResidentOperationFrameFailure::ReplayConflict
                    } else {
                        ResidentOperationFrameFailure::ResidentUnavailable
                    });
                }
            } else {
                if matches!(
                    &phase,
                    ResidentProgramOperationStartPhase::ConsultRoster(_)
                        | ResidentProgramOperationStartPhase::ConsultPrepared(_)
                        | ResidentProgramOperationStartPhase::ConsultParent(_)
                        | ResidentProgramOperationStartPhase::ConsultChildren(_)
                        | ResidentProgramOperationStartPhase::ConsultCollection(_)
                        | ResidentProgramOperationStartPhase::ConsultCollectionCustody(_)
                        | ResidentProgramOperationStartPhase::ConsultOpened(_)
                ) {
                    consult_peer = pending_consult_peer.take().unwrap_or(consult_peer);
                }
                match phase {
                    ResidentProgramOperationStartPhase::Pending(continuation) => {
                        match process.continue_operation_start(
                            continuation,
                            selected.access(),
                            operation_authority,
                        ) {
                            Ok(issued) => {
                                if let Err(failure) = admitted.register_native_start(
                                    process,
                                    native_hooks,
                                    &issued,
                                    current_admission.historical,
                                    operation_authority,
                                ) {
                                    *retained_start_exchange =
                                        Some(Box::new(ResidentProgramRetainedStartExchange {
                                            kind,
                                            selector: pending_selector,
                                            request: pending_request,
                                            locator,
                                            phase: ResidentProgramOperationStartPhase::Issued(
                                                Box::new(issued),
                                            ),
                                        }));
                                    return Err(failure);
                                }
                                return operation_request_host::issued_start_records(&issued)
                                    .map(operation_request_host::ResidentOperationRequestOutcome::Started)
                                    .map_err(|_| ResidentOperationFrameFailure::ResidentUnavailable);
                            }
                            Err(failure) => {
                                *retained_start_exchange =
                                    Some(Box::new(ResidentProgramRetainedStartExchange {
                                        kind,
                                        selector: pending_selector,
                                        request: pending_request,
                                        locator,
                                        phase: ResidentProgramOperationStartPhase::Pending(
                                            *failure.pending,
                                        ),
                                    }));
                                return Err(ResidentOperationFrameFailure::ResidentUnavailable);
                            }
                        }
                    }
                    ResidentProgramOperationStartPhase::Issued(issued) => {
                        return match admitted
                            .register_native_start(
                                process,
                                native_hooks,
                                &issued,
                                current_admission.historical,
                                operation_authority,
                            )
                            .and_then(|()| {
                                operation_request_host::issued_start_records(&issued)
                                    .map_err(|_| ResidentOperationFrameFailure::ResidentUnavailable)
                            }) {
                            Ok(records) => Ok(
                                operation_request_host::ResidentOperationRequestOutcome::Started(
                                    records,
                                ),
                            ),
                            Err(_) => {
                                *retained_start_exchange =
                                    Some(Box::new(ResidentProgramRetainedStartExchange {
                                        kind,
                                        selector: pending_selector,
                                        request: pending_request,
                                        locator,
                                        phase: ResidentProgramOperationStartPhase::Issued(issued),
                                    }));
                                Err(ResidentOperationFrameFailure::ResidentUnavailable)
                            }
                        };
                    }
                    ResidentProgramOperationStartPhase::ConsultRoster(phase) => {
                        retained_consult_phase = Some(phase);
                    }
                    ResidentProgramOperationStartPhase::ConsultPrepared(prepared) => {
                        match into_program_consult_parent(*prepared) {
                            Ok(parent) => retained_consult_parent = Some(Box::new(parent)),
                            Err(prepared) => {
                                *pending_consult_peer = Some(consult_peer);
                                *retained_start_exchange =
                                    Some(Box::new(ResidentProgramRetainedStartExchange {
                                        kind,
                                        selector: pending_selector,
                                        request: pending_request,
                                        locator,
                                        phase: ResidentProgramOperationStartPhase::ConsultPrepared(
                                            prepared,
                                        ),
                                    }));
                                return Err(ResidentOperationFrameFailure::ResidentUnavailable);
                            }
                        }
                    }
                    ResidentProgramOperationStartPhase::ConsultParent(parent) => {
                        retained_consult_parent = Some(parent);
                    }
                    ResidentProgramOperationStartPhase::ConsultChildren(children) => {
                        retained_consult_children = Some(children);
                    }
                    ResidentProgramOperationStartPhase::ConsultCollection(collection) => {
                        retained_consult_collection = Some(collection);
                    }
                    ResidentProgramOperationStartPhase::ConsultCollectionCustody(failure) => {
                        retained_consult_collection_custody = Some(failure);
                    }
                    ResidentProgramOperationStartPhase::ConsultOpened(opened) => {
                        retained_consult_opened = Some(opened);
                    }
                }
            }
        }
        recovery.with_root_recovery(
            &selector,
            &request,
            operation_locator.as_ref(),
            |restart, mut recovery_admission, checkpoint_authority, recovery_owner| {
                if let Some(opened) = retained_consult_opened.take() {
                    let phase = advance_program_consult_opened(
                        process,
                        selected,
                        operation_authority,
                        consult_recovery,
                        &restart,
                        &mut recovery_admission,
                        admitted.now,
                        *opened,
                    );
                    *pending_consult_peer = Some(consult_peer.clone());
                    *retained_start_exchange = Some(Box::new(ResidentProgramRetainedStartExchange {
                        kind: resolution.kind(),
                        selector: selector.clone(),
                        request: request.clone(),
                        locator: resolution.locator().cloned(),
                        phase: ResidentProgramOperationStartPhase::ConsultOpened(Box::new(phase)),
                    }));
                    return Err(ResidentOperationFrameFailure::ResidentUnavailable);
                }
                if let Some(failure) = retained_consult_collection_custody.take() {
                    let phase = continue_program_consult_collection_custody(
                        process,
                        selected,
                        operation_authority,
                        *failure,
                    );
                    *pending_consult_peer = Some(consult_peer.clone());
                    *retained_start_exchange = Some(Box::new(ResidentProgramRetainedStartExchange {
                        kind: resolution.kind(),
                        selector: selector.clone(),
                        request: request.clone(),
                        locator: resolution.locator().cloned(),
                        phase,
                    }));
                    return Err(ResidentOperationFrameFailure::ResidentUnavailable);
                }
                if let Some(collection) = retained_consult_collection.take() {
                    let phase = advance_program_consult_collection(
                        process,
                        selected,
                        operation_authority,
                        consult_recovery,
                        &restart,
                        &mut recovery_admission,
                        *collection,
                    );
                    *pending_consult_peer = Some(consult_peer.clone());
                    *retained_start_exchange = Some(Box::new(ResidentProgramRetainedStartExchange {
                        kind: resolution.kind(),
                        selector: selector.clone(),
                        request: request.clone(),
                        locator: resolution.locator().cloned(),
                        phase,
                    }));
                    return Err(ResidentOperationFrameFailure::ResidentUnavailable);
                }
                if let Some(parent) = retained_consult_parent.take() {
                    let phase = match advance_program_consult_parent(
                        process,
                        selected,
                        operation_authority,
                        cryptography,
                        checkpoint_authority,
                        *parent,
                    ) {
                        Ok(advance) => retain_program_consult_child_advance(
                            selector.clone(),
                            selected.store.clone(),
                            advance,
                        ),
                        Err(parent) => {
                            ResidentProgramOperationStartPhase::ConsultParent(parent)
                        }
                    };
                    *pending_consult_peer = Some(consult_peer.clone());
                    *retained_start_exchange = Some(Box::new(ResidentProgramRetainedStartExchange {
                        kind: resolution.kind(),
                        selector: selector.clone(),
                        request: request.clone(),
                        locator: resolution.locator().cloned(),
                        phase,
                    }));
                    return Err(ResidentOperationFrameFailure::ResidentUnavailable);
                }
                if let Some(children) = retained_consult_children.take() {
                    let phase = match advance_program_consult_child(
                        process,
                        selected,
                        operation_authority,
                        cryptography,
                        checkpoint_authority,
                        &restart,
                        &mut recovery_admission,
                        *children,
                    ) {
                        Ok(phase) => phase,
                        Err(children) => {
                            ResidentProgramOperationStartPhase::ConsultChildren(children)
                        }
                    };
                    *pending_consult_peer = Some(consult_peer.clone());
                    *retained_start_exchange = Some(Box::new(ResidentProgramRetainedStartExchange {
                        kind: resolution.kind(),
                        selector: selector.clone(),
                        request: request.clone(),
                        locator: resolution.locator().cloned(),
                        phase,
                    }));
                    return Err(ResidentOperationFrameFailure::ResidentUnavailable);
                }
                if let Some(phase) = retained_consult_phase.take() {
                    let prepared = prepare_program_consult_roster(
                        process,
                        current_admission,
                        &deployment_authority,
                        selected,
                        &consult_peer,
                        random,
                        operation_authority,
                        cryptography,
                        checkpoint_authority,
                        *phase,
                    );
                    *pending_consult_peer = Some(consult_peer.clone());
                    let phase = match prepared {
                        Ok(prepared) => ResidentProgramOperationStartPhase::ConsultPrepared(
                            Box::new(prepared),
                        ),
                        Err(phase) => {
                            ResidentProgramOperationStartPhase::ConsultRoster(phase)
                        }
                    };
                    *retained_start_exchange = Some(Box::new(ResidentProgramRetainedStartExchange {
                        kind: resolution.kind(),
                        selector: selector.clone(),
                        request: request.clone(),
                        locator: resolution.locator().cloned(),
                        phase,
                    }));
                    return Err(ResidentOperationFrameFailure::ResidentUnavailable);
                }
                admitted.with_ambient_context(|current| {
                    let root = ResidentProcessRootInput {
                        request: &request,
                        session: admitted.session(),
                        current: &current,
                        handle_signing_authority: &handle_authority,
                        checkpoint_authority,
                        fresh_start_admission: recovery_owner.fresh_start_admission(),
                    };
                    let access = selected.access();
                    match resolution.kind() {
                        operation_request_host::ResidentOperationRequestKind::StartInvoke => {
                            process
                                .invoke_request(
                                    &selector,
                                    access,
                                    root,
                                    random,
                                    restart,
                                    recovery_admission,
                                    operation_authority,
                                    policy,
                                    |request, session, current, handle, lease, operation, publication, checkpoint| {
                                        cryptography.admit_compiled_authority(request, session, current, handle, lease, operation, publication, checkpoint)
                                    },
                                    |bytes, preimage, digest| cryptography.sign_handle(bytes, preimage, digest),
                                    |bytes, preimage, digest| cryptography.sign_observation(bytes, preimage, digest),
                                    |prepared, checkpoint_id| cryptography.prepare_checkpoint(prepared, checkpoint_id),
                                    |bytes, preimage, digest| cryptography.sign_checkpoint(bytes, preimage, digest),
                                )
                                .map(|outcome| match outcome {
                                    ResidentProcessRootOutcome::Started(issued) => {
                                        admitted.register_native_start(process, native_hooks, &issued, current_admission.historical, operation_authority)?;
                                        operation_request_host::issued_start_records(&issued)
                                            .map(operation_request_host::ResidentOperationRequestOutcome::Started)
                                            .map_err(|_| ResidentOperationFrameFailure::ResidentUnavailable)
                                    }
                                    ResidentProcessRootOutcome::ExistingOperation(disposition) => {
                                        let issued = operation_locator::restart_disposition_operation(&disposition);
                                        admitted.register_native_start(process, native_hooks, issued, current_admission.historical, operation_authority)?;
                                        operation_request_host::issued_start_records(issued)
                                        .map(
                                            operation_request_host::ResidentOperationRequestOutcome::Started,
                                        )
                                        .map_err(|_| {
                                            ResidentOperationFrameFailure::ResidentUnavailable
                                        })
                                    }
                                })
                                .map_err(|error| match error {
                                    ResidentProcessRootError::Start(
                                        ResidentProcessAllocatedInvokeStartError::Continuation(
                                            continuation,
                                        ),
                                    ) => {
                                        *retained_start_exchange = Some(Box::new(
                                            ResidentProgramRetainedStartExchange {
                                                kind: resolution.kind(),
                                                selector: selector.clone(),
                                                request: request.clone(),
                                                locator: resolution.locator().cloned(),
                                                phase:
                                                    ResidentProgramOperationStartPhase::Pending(
                                                        *continuation,
                                                    ),
                                            },
                                        ));
                                        ResidentOperationFrameFailure::ResidentUnavailable
                                    }
                                    other => map_root_start_error(other),
                                })
                                .and_then(|outcome| outcome)
                        }
                        operation_request_host::ResidentOperationRequestKind::StartConsult => {
                            let operation_context = restart.operation;
                            let maximum_parent_depth = restart.maximum_parent_depth;
                            let result = process.consult_request(
                                    &selector,
                                    access,
                                    root,
                                    random,
                                    restart,
                                    reborrow_operation_recovery_admission(&mut recovery_admission),
                                    operation_authority,
                                    policy,
                                    |request, session, current, handle, lease, operation, publication, checkpoint| {
                                        cryptography.admit_compiled_authority(request, session, current, handle, lease, operation, publication, checkpoint)
                                    },
                                    |bytes, preimage, digest| cryptography.sign_handle(bytes, preimage, digest),
                                    |bytes, preimage, digest| cryptography.sign_observation(bytes, preimage, digest),
                                    |prepared, checkpoint_id| cryptography.prepare_checkpoint(prepared, checkpoint_id),
                                    |bytes, preimage, digest| cryptography.sign_checkpoint(bytes, preimage, digest),
                                );
                            match result {
                                Ok(outcome) => match outcome {
                                    ResidentProcessConsultRootOutcome::Started(started) => {
                                        operation_request_host::issued_start_records(started.parent())
                                            .map(operation_request_host::ResidentOperationRequestOutcome::Started)
                                            .map_err(|_| ResidentOperationFrameFailure::ResidentUnavailable)
                                    }
                                    ResidentProcessConsultRootOutcome::ExistingOperation(disposition) => {
                                        let disposition = consult_initialization_recovery::resume(
                                            process,
                                            &selector,
                                            selected.access(),
                                            operation_context,
                                            maximum_parent_depth,
                                            recovery_admission,
                                            disposition,
                                        )?;
                                        let disposition = match disposition {
                                            consult_initialization_recovery::RecoveredStart::Existing(disposition) => disposition,
                                            consult_initialization_recovery::RecoveredStart::OpenCollection(pending) => {
                                                *pending_consult_peer = Some(consult_peer.clone());
                                                *retained_start_exchange = Some(Box::new(ResidentProgramRetainedStartExchange {
                                                    kind: resolution.kind(),
                                                    selector: selector.clone(),
                                                    request: request.clone(),
                                                    locator: resolution.locator().cloned(),
                                                    phase: ResidentProgramOperationStartPhase::ConsultCollection(pending),
                                                }));
                                                return Err(ResidentOperationFrameFailure::ResidentUnavailable);
                                            }
                                        };
                                        operation_request_host::issued_start_records(
                                            operation_locator::restart_disposition_operation(
                                                &disposition,
                                            ),
                                        )
                                        .map(
                                            operation_request_host::ResidentOperationRequestOutcome::Started,
                                        )
                                        .map_err(|_| {
                                            ResidentOperationFrameFailure::ResidentUnavailable
                                        })
                                    }
                                },
                                Err(ResidentProcessConsultRootError::Start(
                                    ResidentProcessAllocatedConsultStartError::Roster(
                                        continuation,
                                    ),
                                )) => {
                                    let prepared = prepare_program_consult_roster(
                                        process,
                                        current_admission,
                                        &deployment_authority,
                                        selected,
                                        &consult_peer,
                                        random,
                                        operation_authority,
                                        cryptography,
                                        checkpoint_authority,
                                        ResidentProgramConsultRosterPhase::Pending(*continuation),
                                    );
                                    *pending_consult_peer = Some(consult_peer.clone());
                                    let phase = match prepared {
                                        Ok(prepared) => {
                                            ResidentProgramOperationStartPhase::ConsultPrepared(
                                                Box::new(prepared),
                                            )
                                        }
                                        Err(phase) => {
                                            ResidentProgramOperationStartPhase::ConsultRoster(
                                                phase,
                                            )
                                        }
                                    };
                                    *retained_start_exchange = Some(Box::new(
                                        ResidentProgramRetainedStartExchange {
                                            kind: resolution.kind(),
                                            selector: selector.clone(),
                                            request: request.clone(),
                                            locator: resolution.locator().cloned(),
                                            phase,
                                        },
                                    ));
                                    Err(ResidentOperationFrameFailure::ResidentUnavailable)
                                }
                                Err(ResidentProcessConsultRootError::Start(
                                    ResidentProcessAllocatedConsultStartError::Continuation(
                                        continuation,
                                    ),
                                )) => {
                                        *retained_start_exchange = Some(Box::new(
                                            ResidentProgramRetainedStartExchange {
                                                kind: resolution.kind(),
                                                selector: selector.clone(),
                                                request: request.clone(),
                                                locator: resolution.locator().cloned(),
                                                phase:
                                                    ResidentProgramOperationStartPhase::Pending(
                                                        *continuation,
                                                    ),
                                            },
                                        ));
                                        Err(ResidentOperationFrameFailure::ResidentUnavailable)
                                }
                                Err(other) => Err(map_consult_start_error(other)),
                            }
                        }
                        operation_request_host::ResidentOperationRequestKind::StartDelegate => {
                            let parent_locator = operation_locator.as_ref().ok_or(
                                ResidentOperationFrameFailure::InvalidRequest,
                            )?;
                            let ResidentDeploymentOperationAccess {
                                store,
                                custody,
                                sessions,
                                anchor,
                            } = access;
                            let mut recovery_admission = recovery_admission;
                            let recovered = crate::resident::recover_operation_control(
                                parent_locator.recovery_input(
                                    restart.operation,
                                    restart.maximum_parent_depth,
                                    restart.current_head,
                                ),
                                &mut *custody,
                                sessions,
                                &mut *anchor,
                                reborrow_operation_recovery_admission(
                                    &mut recovery_admission,
                                ),
                            )
                            .map_err(|_| ResidentOperationFrameFailure::ResidentUnavailable)?;
                            parent_locator
                                .admit_recovered(&recovered)
                                .map_err(|_| ResidentOperationFrameFailure::InvalidRequest)?;
                            let transaction = custody
                                .prepare_transaction(store)
                                .map_err(|_| ResidentOperationFrameFailure::ResidentUnavailable)?;
                            let parent = crate::resident::local_operation_start::prepare_local_child_parent_from_recovered(
                                store,
                                transaction,
                                recovered,
                            )
                            .map_err(|_| ResidentOperationFrameFailure::ResidentUnavailable)?;
                            process
                                .delegate_request(
                                    &selector,
                                    ResidentDeploymentOperationAccess {
                                        store,
                                        custody,
                                        sessions,
                                        anchor,
                                    },
                                    root,
                                    parent,
                                    random,
                                    restart,
                                    recovery_admission,
                                    operation_authority,
                                    policy,
                                    |request, session, current, handle, lease, operation, publication, checkpoint| {
                                        cryptography.admit_compiled_authority(request, session, current, handle, lease, operation, publication, checkpoint)
                                    },
                                    |bytes, preimage, digest| cryptography.sign_handle(bytes, preimage, digest),
                                    |bytes, preimage, digest| cryptography.sign_observation(bytes, preimage, digest),
                                    |prepared, checkpoint_id| cryptography.prepare_checkpoint(prepared, checkpoint_id),
                                    |bytes, preimage, digest| cryptography.sign_checkpoint(bytes, preimage, digest),
                                )
                                .map(|outcome| match outcome {
                                    ResidentProcessRootOutcome::Started(issued) => {
                                        operation_request_host::issued_start_records(&issued)
                                            .map(operation_request_host::ResidentOperationRequestOutcome::Started)
                                            .map_err(|_| ResidentOperationFrameFailure::ResidentUnavailable)
                                    }
                                    ResidentProcessRootOutcome::ExistingOperation(disposition) => {
                                        operation_request_host::issued_start_records(
                                            operation_locator::restart_disposition_operation(
                                                &disposition,
                                            ),
                                        )
                                        .map(
                                            operation_request_host::ResidentOperationRequestOutcome::Started,
                                        )
                                        .map_err(|_| {
                                            ResidentOperationFrameFailure::ResidentUnavailable
                                        })
                                    }
                                })
                                .map_err(|error| match error {
                                    ResidentProcessDelegateRootError::Start(
                                        ResidentProcessAllocatedDelegateStartError::Continuation(
                                            continuation,
                                        ),
                                    ) => {
                                    *retained_start_exchange = Some(Box::new(
                                        ResidentProgramRetainedStartExchange {
                                                kind: resolution.kind(),
                                                selector: selector.clone(),
                                                request: request.clone(),
                                                locator: resolution.locator().cloned(),
                                                phase:
                                                    ResidentProgramOperationStartPhase::Pending(
                                                        *continuation,
                                                    ),
                                            },
                                        ));
                                        ResidentOperationFrameFailure::ResidentUnavailable
                                    }
                                    other => map_delegate_start_error(other),
                                })
                                .and_then(|outcome| outcome)
                        }
                        operation_request_host::ResidentOperationRequestKind::AdvanceInvoke => {
                            let locator = operation_locator.as_ref().ok_or(
                                ResidentOperationFrameFailure::InvalidRequest,
                            )?;
                            let ResidentDeploymentOperationAccess {
                                store,
                                custody,
                                sessions,
                                anchor,
                            } = access;
                            let mut recovery_admission = recovery_admission;
                            let recovered = crate::resident::recover_operation_control(
                                locator.recovery_input(
                                    restart.operation,
                                    restart.maximum_parent_depth,
                                    restart.current_head,
                                ),
                                &mut *custody,
                                sessions,
                                &mut *anchor,
                                reborrow_operation_recovery_admission(
                                    &mut recovery_admission,
                                ),
                            )
                            .map_err(|_| ResidentOperationFrameFailure::ResidentUnavailable)?;
                            locator
                                .admit_recovered(&recovered)
                                .map_err(|_| ResidentOperationFrameFailure::InvalidRequest)?;
                            let records = operation_request_host::recovered_control_records(
                                &recovered,
                            )
                            .map_err(|_| ResidentOperationFrameFailure::ResidentUnavailable)?;
                            let issued = recovered.operation().operation();
                            if issued.handle().1.operation_class()
                                != &pale_core::records::OperationClassV1::Invoke
                                || issued.handle().1.parent_operation().is_some()
                            {
                                return Err(ResidentOperationFrameFailure::InvalidRequest);
                            }
                            let original_request = recovered.operation().request().clone();
                            let disposition = process
                                .classify_started_operation(
                                    &selector,
                                    issued,
                                    &original_request,
                                    ResidentDeploymentOperationAccess {
                                        store,
                                        custody: &mut *custody,
                                        sessions,
                                        anchor: &mut *anchor,
                                    },
                                    restart,
                                    reborrow_operation_recovery_admission(
                                        &mut recovery_admission,
                                    ),
                                )
                                .map_err(|_| ResidentOperationFrameFailure::ResidentUnavailable)?;
                            locator
                                .admit_restart_disposition(&disposition)
                                .map_err(|_| ResidentOperationFrameFailure::InvalidRequest)?;
                            let turn = operation_recovery_sweep::route_root_provider_disposition(
                                process,
                                disposition,
                                ResidentDeploymentOperationAccess {
                                    store,
                                    custody,
                                    sessions,
                                    anchor,
                                },
                                recovery_owner,
                            )
                            .map_err(|_| ResidentOperationFrameFailure::ResidentUnavailable)?;
                            match turn {
                                operation_recovery_sweep::ResidentProcessOperationRecoveryTurn::Scheduled(
                                    outcome,
                                ) => Ok(outcome),
                                operation_recovery_sweep::ResidentProcessOperationRecoveryTurn::ReportedTerminal
                                | operation_recovery_sweep::ResidentProcessOperationRecoveryTurn::ReportedRecoveryOnly(_) => {
                                    Ok(operation_request_host::ResidentOperationRequestOutcome::Observed(records))
                                }
                                operation_recovery_sweep::ResidentProcessOperationRecoveryTurn::DeferredUnsupported { .. }
                                | operation_recovery_sweep::ResidentProcessOperationRecoveryTurn::Exhausted => {
                                    Err(ResidentOperationFrameFailure::InvalidRequest)
                                }
                            }
                        }
                        operation_request_host::ResidentOperationRequestKind::AdvanceConsult => {
                            unreachable!(
                                "AdvanceConsult is dispatched before start-only recovery"
                            )
                        }
                        operation_request_host::ResidentOperationRequestKind::AdvanceDelegate => {
                            let locator = operation_locator.as_ref().ok_or(
                                ResidentOperationFrameFailure::InvalidRequest,
                            )?;
                            let ResidentDeploymentOperationAccess {
                                store,
                                custody,
                                sessions,
                                anchor,
                            } = access;
                            let mut recovery_admission = recovery_admission;
                            let maximum_selection_bytes = restart.maximum_selection_bytes;
                            let maximum_record_bytes = restart.operation.maximum_record_bytes;
                            let recovered = crate::resident::recover_operation_control(
                                locator.recovery_input(
                                    restart.operation,
                                    restart.maximum_parent_depth,
                                    restart.current_head,
                                ),
                                &mut *custody,
                                sessions,
                                &mut *anchor,
                                reborrow_operation_recovery_admission(
                                    &mut recovery_admission,
                                ),
                            )
                            .map_err(|_| ResidentOperationFrameFailure::ResidentUnavailable)?;
                            locator
                                .admit_recovered(&recovered)
                                .map_err(|_| ResidentOperationFrameFailure::InvalidRequest)?;
                            let records = operation_request_host::recovered_control_records(
                                &recovered,
                            )
                            .map_err(|_| ResidentOperationFrameFailure::ResidentUnavailable)?;
                            let issued = recovered.operation().operation();
                            if issued.handle().1.operation_class()
                                != &pale_core::records::OperationClassV1::Delegate
                                || issued.handle().1.parent_operation().is_none()
                            {
                                return Err(ResidentOperationFrameFailure::InvalidRequest);
                            }
                            let protected = process
                                .protect_point_recovered_operation_start(
                                    &selector,
                                    recovered,
                                    ResidentDeploymentOperationAccess {
                                        store,
                                        custody: &mut *custody,
                                        sessions,
                                        anchor: &mut *anchor,
                                    },
                                    &restart,
                                    &mut recovery_admission,
                                )
                                .map_err(|_| ResidentOperationFrameFailure::ResidentUnavailable)?;
                            let disposition = process
                                .classify_protected_delegate_child_operation(
                                    protected,
                                    ResidentDeploymentOperationAccess {
                                        store,
                                        custody: &mut *custody,
                                        sessions,
                                        anchor: &mut *anchor,
                                    },
                                    restart,
                                    &mut recovery_admission,
                                )
                                .map_err(|_| ResidentOperationFrameFailure::ResidentUnavailable)?;
                            locator
                                .admit_restart_disposition(&disposition)
                                .map_err(|_| ResidentOperationFrameFailure::InvalidRequest)?;
                            let turn = match disposition {
                                ResidentProcessOperationRestartDisposition::OperationResolution(
                                    recovered,
                                ) => {
                                    if !matches!(
                                        &recovered.start.lineage,
                                        restart::ResidentProcessOperationLineage::PairwiseDelegate(_)
                                    ) {
                                        return Err(ResidentOperationFrameFailure::InvalidRequest);
                                    }
                                    let input = ResidentProgramDelegateResolutionInput::new(
                                        &request,
                                        admitted.session(),
                                        &current,
                                        maximum_selection_bytes,
                                        maximum_record_bytes,
                                    );
                                    let outcome = recovery_owner
                                        .recover_delegate_operation_resolution(
                                            process,
                                            recovered,
                                            ResidentDeploymentOperationAccess {
                                                store,
                                                custody,
                                                sessions,
                                                anchor,
                                            },
                                            input,
                                        )
                                        .map_err(|_| {
                                            ResidentOperationFrameFailure::ResidentUnavailable
                                        })?;
                                    delegate_recovery_sweep::ResidentProcessDelegateRecoveryTurn::Scheduled(
                                        outcome,
                                    )
                                }
                                other => process
                                    .route_delegate_restart_disposition(
                                        other,
                                        ResidentDeploymentOperationAccess {
                                            store,
                                            custody,
                                            sessions,
                                            anchor,
                                        },
                                        recovery_owner,
                                    )
                                    .map_err(|_| {
                                        ResidentOperationFrameFailure::ResidentUnavailable
                                    })?,
                            };
                            match turn {
                                delegate_recovery_sweep::ResidentProcessDelegateRecoveryTurn::Scheduled(
                                    outcome,
                                ) => Ok(outcome),
                                delegate_recovery_sweep::ResidentProcessDelegateRecoveryTurn::ReportedTerminal(_)
                                | delegate_recovery_sweep::ResidentProcessDelegateRecoveryTurn::ReportedRecoveryOnly(_) => {
                                    Ok(operation_request_host::ResidentOperationRequestOutcome::Observed(records))
                                }
                                delegate_recovery_sweep::ResidentProcessDelegateRecoveryTurn::DeferredResolution(_) => {
                                    Err(ResidentOperationFrameFailure::InvalidRequest)
                                }
                                delegate_recovery_sweep::ResidentProcessDelegateRecoveryTurn::SkippedOtherOwner(_)
                                | delegate_recovery_sweep::ResidentProcessDelegateRecoveryTurn::Exhausted => {
                                    Err(ResidentOperationFrameFailure::InvalidRequest)
                                }
                            }
                        }
                        operation_request_host::ResidentOperationRequestKind::Cancel => {
                            answer_cancel_request(
                                process,
                                &selector,
                                access,
                                operation_authority,
                                recovery_owner,
                                admitted.now,
                                &request,
                                resolution,
                                operation_locator.as_ref(),
                                restart,
                                recovery_admission,
                                checkpoint_authority,
                            )
                        }
                    }
                })
            },
        )?
    }

    fn has_retained_phase(&self) -> bool {
        matches!(
            self.retained_start_exchange
                .as_deref()
                .map(|retained| &retained.phase),
            Some(
                ResidentProgramOperationStartPhase::Pending(_)
                    | ResidentProgramOperationStartPhase::ConsultRoster(_)
                    | ResidentProgramOperationStartPhase::ConsultPrepared(_)
                    | ResidentProgramOperationStartPhase::ConsultParent(_)
                    | ResidentProgramOperationStartPhase::ConsultChildren(_)
                    | ResidentProgramOperationStartPhase::ConsultCollection(_)
                    | ResidentProgramOperationStartPhase::ConsultCollectionCustody(_)
                    | ResidentProgramOperationStartPhase::ConsultOpened(_)
            )
        ) || self.recovery.has_runnable_retained_invoke()
            || self.recovery.has_runnable_retained_delegate()
            || self.recovery.has_retained_cancellation()
            || self.consult_recovery.has_runnable_retained_consult()
    }

    fn continue_retained_phase<'store>(
        &mut self,
        process: &mut ResidentProcessHost<'store, WriterGuard>,
        policy: &mut PolicyEvaluator,
        deployment_authorities: &[ResidentProgramDeploymentAuthority],
    ) -> ResidentProgramRetainedPhaseTurn {
        let Self {
            current,
            deployments,
            random,
            operation_authority,
            cryptography,
            recovery,
            consult_recovery,
            pending_consult_peer,
            retained_start_exchange,
            next_retained_phase_lane,
            next_recovery_phase,
            ..
        } = self;
        let start_is_runnable = matches!(
            retained_start_exchange
                .as_deref()
                .map(|retained| &retained.phase),
            Some(
                ResidentProgramOperationStartPhase::Pending(_)
                    | ResidentProgramOperationStartPhase::ConsultRoster(_)
                    | ResidentProgramOperationStartPhase::ConsultPrepared(_)
                    | ResidentProgramOperationStartPhase::ConsultParent(_)
                    | ResidentProgramOperationStartPhase::ConsultChildren(_)
                    | ResidentProgramOperationStartPhase::ConsultCollection(_)
                    | ResidentProgramOperationStartPhase::ConsultCollectionCustody(_)
                    | ResidentProgramOperationStartPhase::ConsultOpened(_)
            )
        );
        let mut selected_lane = *next_retained_phase_lane;
        let mut selected_start = false;
        for _ in 0..3 {
            match selected_lane {
                ResidentProgramRetainedPhaseLane::Start if start_is_runnable => {
                    selected_start = true;
                    break;
                }
                ResidentProgramRetainedPhaseLane::Consult
                    if consult_recovery.has_runnable_retained_consult() =>
                {
                    *next_retained_phase_lane = selected_lane.next();
                    return consult_recovery.continue_one_retained_consult(
                        process,
                        deployments,
                        operation_authority,
                        policy,
                        &mut || current.observe_time().ok(),
                    );
                }
                ResidentProgramRetainedPhaseLane::Recovery
                    if recovery.has_runnable_retained_invoke()
                        || recovery.has_runnable_retained_delegate()
                        || recovery.has_retained_cancellation() =>
                {
                    *next_retained_phase_lane = selected_lane.next();
                    return continue_one_recovery_phase(
                        process,
                        deployments,
                        operation_authority,
                        recovery,
                        next_recovery_phase,
                    );
                }
                _ => selected_lane = selected_lane.next(),
            }
        }
        if !selected_start {
            return ResidentProgramRetainedPhaseTurn::Idle;
        }
        *next_retained_phase_lane = selected_lane.next();
        let Some(pending) = retained_start_exchange.take() else {
            return ResidentProgramRetainedPhaseTurn::Idle;
        };
        let ResidentProgramRetainedStartExchange {
            kind,
            selector,
            request,
            locator,
            phase,
        } = *pending;
        match phase {
            ResidentProgramOperationStartPhase::Issued(issued) => {
                *retained_start_exchange = Some(Box::new(ResidentProgramRetainedStartExchange {
                    kind,
                    selector,
                    request,
                    locator,
                    phase: ResidentProgramOperationStartPhase::Issued(issued),
                }));
                continue_one_recovery_phase(
                    process,
                    deployments,
                    operation_authority,
                    recovery,
                    next_recovery_phase,
                )
            }
            ResidentProgramOperationStartPhase::Pending(continuation) => {
                let mut matching = deployments
                    .iter_mut()
                    .filter(|deployment| deployment.matches(&selector));
                let Some(selected) = matching.next() else {
                    *retained_start_exchange =
                        Some(Box::new(ResidentProgramRetainedStartExchange {
                            kind,
                            selector,
                            request,
                            locator,
                            phase: ResidentProgramOperationStartPhase::Pending(continuation),
                        }));
                    return ResidentProgramRetainedPhaseTurn::Deferred;
                };
                if matching.next().is_some() {
                    *retained_start_exchange =
                        Some(Box::new(ResidentProgramRetainedStartExchange {
                            kind,
                            selector,
                            request,
                            locator,
                            phase: ResidentProgramOperationStartPhase::Pending(continuation),
                        }));
                    return ResidentProgramRetainedPhaseTurn::Deferred;
                }
                match process.continue_operation_start(
                    continuation,
                    selected.access(),
                    operation_authority,
                ) {
                    Ok(issued) => {
                        *retained_start_exchange =
                            Some(Box::new(ResidentProgramRetainedStartExchange {
                                kind,
                                selector,
                                request,
                                locator,
                                phase: ResidentProgramOperationStartPhase::Issued(Box::new(issued)),
                            }));
                        ResidentProgramRetainedPhaseTurn::Advanced
                    }
                    Err(failure) => {
                        *retained_start_exchange =
                            Some(Box::new(ResidentProgramRetainedStartExchange {
                                kind,
                                selector,
                                request,
                                locator,
                                phase: ResidentProgramOperationStartPhase::Pending(
                                    *failure.pending,
                                ),
                            }));
                        ResidentProgramRetainedPhaseTurn::Deferred
                    }
                }
            }
            ResidentProgramOperationStartPhase::ConsultOpened(opened)
                if matches!(&*opened, ResidentProgramConsultOpenedPhase::Ready(_)) =>
            {
                let ResidentProgramConsultOpenedPhase::Ready(ready) = *opened else {
                    unreachable!("the guard selected one Ready Consult phase")
                };
                let parent = match operation_locator::ResidentProgramOperationLocator::from_issued(
                    ready.parent(),
                ) {
                    Ok(parent) => parent.coordinate().clone(),
                    Err(_) => {
                        *retained_start_exchange =
                            Some(Box::new(ResidentProgramRetainedStartExchange {
                                kind,
                                selector,
                                request,
                                locator,
                                phase: ResidentProgramOperationStartPhase::ConsultOpened(Box::new(
                                    ResidentProgramConsultOpenedPhase::Ready(ready),
                                )),
                            }));
                        return ResidentProgramRetainedPhaseTurn::Deferred;
                    }
                };
                let key = ResidentProgramConsultTurnKey::new(selector.clone(), parent);
                if !ready_matches_consult_turn(process, &key, &ready) {
                    *retained_start_exchange =
                        Some(Box::new(ResidentProgramRetainedStartExchange {
                            kind,
                            selector,
                            request,
                            locator,
                            phase: ResidentProgramOperationStartPhase::ConsultOpened(Box::new(
                                ResidentProgramConsultOpenedPhase::Ready(ready),
                            )),
                        }));
                    return ResidentProgramRetainedPhaseTurn::Deferred;
                }
                let ready = match classify_consult_ready_retention(consult_recovery, &key, ready) {
                    ResidentProgramConsultReadyRetention::Subsumed => {
                        pending_consult_peer.take();
                        return ResidentProgramRetainedPhaseTurn::Advanced;
                    }
                    ResidentProgramConsultReadyRetention::Deferred(ready) => {
                        *retained_start_exchange =
                            Some(Box::new(ResidentProgramRetainedStartExchange {
                                kind,
                                selector,
                                request,
                                locator,
                                phase: ResidentProgramOperationStartPhase::ConsultOpened(Box::new(
                                    ResidentProgramConsultOpenedPhase::Ready(ready),
                                )),
                            }));
                        return ResidentProgramRetainedPhaseTurn::Deferred;
                    }
                    ResidentProgramConsultReadyRetention::Retain(ready) => ready,
                };
                let retained = ResidentProgramKeyedConsultTurn {
                    key,
                    turn: ResidentProgramConsultTurn::StartReady {
                        ready: Box::new(ready),
                    },
                };
                if consult_recovery
                    .retain_keyed_consult_turn(retained)
                    .is_err()
                {
                    unreachable!(
                        "exclusive Ready admission must remain valid until keyed retention"
                    )
                }
                pending_consult_peer.take();
                ResidentProgramRetainedPhaseTurn::Advanced
            }
            ResidentProgramOperationStartPhase::ConsultPrepared(prepared) => {
                let Some(peer) = pending_consult_peer.take() else {
                    *retained_start_exchange =
                        Some(Box::new(ResidentProgramRetainedStartExchange {
                            kind,
                            selector,
                            request,
                            locator,
                            phase: ResidentProgramOperationStartPhase::ConsultPrepared(prepared),
                        }));
                    return ResidentProgramRetainedPhaseTurn::Deferred;
                };
                match into_program_consult_parent(*prepared) {
                    Ok(parent) => {
                        retain_consult_start_exchange(
                            retained_start_exchange,
                            pending_consult_peer,
                            kind,
                            selector,
                            request,
                            locator,
                            peer,
                            ResidentProgramOperationStartPhase::ConsultParent(Box::new(parent)),
                        );
                        ResidentProgramRetainedPhaseTurn::Advanced
                    }
                    Err(prepared) => {
                        retain_consult_start_exchange(
                            retained_start_exchange,
                            pending_consult_peer,
                            kind,
                            selector,
                            request,
                            locator,
                            peer,
                            ResidentProgramOperationStartPhase::ConsultPrepared(prepared),
                        );
                        ResidentProgramRetainedPhaseTurn::Deferred
                    }
                }
            }
            phase @ (ResidentProgramOperationStartPhase::ConsultRoster(_)
            | ResidentProgramOperationStartPhase::ConsultParent(_)
            | ResidentProgramOperationStartPhase::ConsultChildren(_)
            | ResidentProgramOperationStartPhase::ConsultCollection(_)
            | ResidentProgramOperationStartPhase::ConsultCollectionCustody(_)
            | ResidentProgramOperationStartPhase::ConsultOpened(_)) => {
                let Some(peer) = pending_consult_peer.take() else {
                    *retained_start_exchange =
                        Some(Box::new(ResidentProgramRetainedStartExchange {
                            kind,
                            selector,
                            request,
                            locator,
                            phase,
                        }));
                    return ResidentProgramRetainedPhaseTurn::Deferred;
                };
                let mut matching = deployments
                    .iter_mut()
                    .filter(|deployment| deployment.matches(&selector));
                let Some(selected) = matching.next() else {
                    retain_consult_start_exchange(
                        retained_start_exchange,
                        pending_consult_peer,
                        kind,
                        selector,
                        request,
                        locator,
                        peer,
                        phase,
                    );
                    return ResidentProgramRetainedPhaseTurn::Deferred;
                };
                if matching.next().is_some() {
                    retain_consult_start_exchange(
                        retained_start_exchange,
                        pending_consult_peer,
                        kind,
                        selector,
                        request,
                        locator,
                        peer,
                        phase,
                    );
                    return ResidentProgramRetainedPhaseTurn::Deferred;
                }
                let mut authority_matches = deployment_authorities
                    .iter()
                    .filter(|authority| authority.selector() == &selector);
                let Some(authority) = authority_matches.next() else {
                    retain_consult_start_exchange(
                        retained_start_exchange,
                        pending_consult_peer,
                        kind,
                        selector,
                        request,
                        locator,
                        peer,
                        phase,
                    );
                    return ResidentProgramRetainedPhaseTurn::Deferred;
                };
                if authority_matches.next().is_some() {
                    retain_consult_start_exchange(
                        retained_start_exchange,
                        pending_consult_peer,
                        kind,
                        selector,
                        request,
                        locator,
                        peer,
                        phase,
                    );
                    return ResidentProgramRetainedPhaseTurn::Deferred;
                }
                let operation_locator = match locator
                    .as_ref()
                    .map(operation_locator::ResidentProgramOperationLocator::parse)
                    .transpose()
                {
                    Ok(locator) => locator,
                    Err(_) => {
                        retain_consult_start_exchange(
                            retained_start_exchange,
                            pending_consult_peer,
                            kind,
                            selector,
                            request,
                            locator,
                            peer,
                            phase,
                        );
                        return ResidentProgramRetainedPhaseTurn::Deferred;
                    }
                };
                let mut retained_phase = Some(phase);
                let mut advanced = None;
                let lender = recovery.with_root_recovery(
                    &selector,
                    &request,
                    operation_locator.as_ref(),
                    |restart, mut recovery_admission, checkpoint_authority, _recovery_owner| {
                        let phase = retained_phase
                            .take()
                            .expect("the root-recovery lender runs at most once");
                        advanced = Some(match phase {
                            ResidentProgramOperationStartPhase::ConsultRoster(phase) => {
                                match prepare_program_consult_roster(
                                    process,
                                    current,
                                    authority,
                                    selected,
                                    &peer,
                                    random,
                                    operation_authority,
                                    cryptography,
                                    checkpoint_authority,
                                    *phase,
                                ) {
                                    Ok(prepared) => (
                                        ResidentProgramOperationStartPhase::ConsultPrepared(
                                            Box::new(prepared),
                                        ),
                                        ResidentProgramRetainedPhaseTurn::Advanced,
                                    ),
                                    Err(phase) => (
                                        ResidentProgramOperationStartPhase::ConsultRoster(phase),
                                        ResidentProgramRetainedPhaseTurn::Deferred,
                                    ),
                                }
                            }
                            ResidentProgramOperationStartPhase::ConsultParent(parent) => {
                                match advance_program_consult_parent(
                                    process,
                                    selected,
                                    operation_authority,
                                    cryptography,
                                    checkpoint_authority,
                                    *parent,
                                ) {
                                    Ok(advance) => (
                                        retain_program_consult_child_advance(
                                            selector.clone(),
                                            selected.store.clone(),
                                            advance,
                                        ),
                                        ResidentProgramRetainedPhaseTurn::Advanced,
                                    ),
                                    Err(parent) => (
                                        ResidentProgramOperationStartPhase::ConsultParent(parent),
                                        ResidentProgramRetainedPhaseTurn::Deferred,
                                    ),
                                }
                            }
                            ResidentProgramOperationStartPhase::ConsultChildren(children) => {
                                match advance_program_consult_child(
                                    process,
                                    selected,
                                    operation_authority,
                                    cryptography,
                                    checkpoint_authority,
                                    &restart,
                                    &mut recovery_admission,
                                    *children,
                                ) {
                                    Ok(phase) => {
                                        (phase, ResidentProgramRetainedPhaseTurn::Advanced)
                                    }
                                    Err(children) => (
                                        ResidentProgramOperationStartPhase::ConsultChildren(
                                            children,
                                        ),
                                        ResidentProgramRetainedPhaseTurn::Deferred,
                                    ),
                                }
                            }
                            ResidentProgramOperationStartPhase::ConsultCollection(collection) => {
                                let phase = advance_program_consult_collection(
                                    process,
                                    selected,
                                    operation_authority,
                                    consult_recovery,
                                    &restart,
                                    &mut recovery_admission,
                                    *collection,
                                );
                                let turn = if matches!(
                                    &phase,
                                    ResidentProgramOperationStartPhase::ConsultCollection(_)
                                ) {
                                    ResidentProgramRetainedPhaseTurn::Deferred
                                } else {
                                    ResidentProgramRetainedPhaseTurn::Advanced
                                };
                                (phase, turn)
                            }
                            ResidentProgramOperationStartPhase::ConsultCollectionCustody(
                                failure,
                            ) => {
                                let phase = continue_program_consult_collection_custody(
                                    process,
                                    selected,
                                    operation_authority,
                                    *failure,
                                );
                                let turn = if matches!(
                                    &phase,
                                    ResidentProgramOperationStartPhase::ConsultOpened(_)
                                ) {
                                    ResidentProgramRetainedPhaseTurn::Advanced
                                } else {
                                    ResidentProgramRetainedPhaseTurn::Deferred
                                };
                                (phase, turn)
                            }
                            ResidentProgramOperationStartPhase::ConsultOpened(opened) => {
                                let recovering_collection = matches!(
                                    &*opened,
                                    ResidentProgramConsultOpenedPhase::Opened(_)
                                        | ResidentProgramConsultOpenedPhase::Recovery(_)
                                );
                                let opening_batch = matches!(
                                    &*opened,
                                    ResidentProgramConsultOpenedPhase::Recovered(_)
                                );
                                let retrying_batch =
                                    matches!(&*opened, ResidentProgramConsultOpenedPhase::Batch(_));
                                let Ok(observed_at) = current.observe_time() else {
                                    return advanced = Some((
                                        ResidentProgramOperationStartPhase::ConsultOpened(opened),
                                        ResidentProgramRetainedPhaseTurn::Deferred,
                                    ));
                                };
                                let phase = advance_program_consult_opened(
                                    process,
                                    selected,
                                    operation_authority,
                                    consult_recovery,
                                    &restart,
                                    &mut recovery_admission,
                                    observed_at,
                                    *opened,
                                );
                                let phase_advanced = (recovering_collection
                                    && matches!(
                                        &phase,
                                        ResidentProgramConsultOpenedPhase::Recovered(_)
                                    ))
                                    || (opening_batch
                                        && matches!(
                                            &phase,
                                            ResidentProgramConsultOpenedPhase::Ready(_)
                                                | ResidentProgramConsultOpenedPhase::Batch(_)
                                        ))
                                    || (retrying_batch
                                        && matches!(
                                            &phase,
                                            ResidentProgramConsultOpenedPhase::Ready(_)
                                        ));
                                let turn = if phase_advanced {
                                    ResidentProgramRetainedPhaseTurn::Advanced
                                } else {
                                    ResidentProgramRetainedPhaseTurn::Deferred
                                };
                                (
                                    ResidentProgramOperationStartPhase::ConsultOpened(Box::new(
                                        phase,
                                    )),
                                    turn,
                                )
                            }
                            _ => unreachable!("only retained Consult phases enter this owner"),
                        });
                    },
                );
                let (phase, turn) = match advanced {
                    Some(advanced) => advanced,
                    None => {
                        debug_assert!(lender.is_err());
                        (
                            retained_phase
                                .expect("an unavailable root-recovery lender preserves the phase"),
                            ResidentProgramRetainedPhaseTurn::Deferred,
                        )
                    }
                };
                retain_consult_start_exchange(
                    retained_start_exchange,
                    pending_consult_peer,
                    kind,
                    selector,
                    request,
                    locator,
                    peer,
                    phase,
                );
                turn
            }
        }
    }

    fn recover_root_provider<'store>(
        &mut self,
        process: &mut ResidentProcessHost<'store, WriterGuard>,
        selector: &ResidentDeploymentAuthoritySelector,
    ) -> Result<(), ResidentProgramTurnError> {
        let Self {
            deployments,
            recovery,
            ..
        } = self;
        let mut matching = deployments
            .iter_mut()
            .filter(|deployment| deployment.matches(selector));
        let selected = matching
            .next()
            .ok_or(ResidentProgramTurnError::RecoveryUnavailable)?;
        if matching.next().is_some() {
            return Err(ResidentProgramTurnError::RecoveryUnavailable);
        }
        recovery.with_background_recovery(
            selector,
            |restart, admission, maximum_session_basis_bytes, _observed_at, owner| {
                match process.recover_one_root_provider_start(
                        selector,
                        selected.access(),
                        restart,
                        admission,
                        maximum_session_basis_bytes,
                        owner,
                    ) {
                    Ok(operation_recovery_sweep::ResidentProcessOperationRecoveryTurn::ReportedRecoveryOnly(_)) => Ok(()),
                    Ok(operation_recovery_sweep::ResidentProcessOperationRecoveryTurn::DeferredUnsupported {
                        owner: operation_recovery_sweep::ResidentProcessOperationRecoveryDeferred::A2a
                            | operation_recovery_sweep::ResidentProcessOperationRecoveryDeferred::Native,
                        disposition: Some(disposition),
                    }) => {
                        owner.retain_deferred_recovery(disposition);
                        Err(ResidentProgramTurnError::RecoveryUnavailable)
                    }
                    Ok(_) => Ok(()),
                    Err(_) => Err(ResidentProgramTurnError::RecoveryUnavailable),
                }
            },
        )
    }

    fn recover_root_consult<'store>(
        &mut self,
        process: &mut ResidentProcessHost<'store, WriterGuard>,
        policy: &mut PolicyEvaluator,
        selector: &ResidentDeploymentAuthoritySelector,
    ) -> Result<(), ResidentProgramTurnError> {
        let Self {
            deployments,
            operation_authority,
            recovery,
            consult_recovery,
            ..
        } = self;
        let mut matching = deployments
            .iter_mut()
            .filter(|deployment| deployment.matches(selector));
        let selected = matching
            .next()
            .ok_or(ResidentProgramTurnError::RecoveryUnavailable)?;
        if matching.next().is_some() {
            return Err(ResidentProgramTurnError::RecoveryUnavailable);
        }
        if !consult_recovery.has_consult_turn_capacity() {
            return Err(ResidentProgramTurnError::RecoveryUnavailable);
        }
        recovery.with_background_recovery(
            selector,
            |restart, admission, maximum_session_basis_bytes, observed_at, _owner| {
                let retention_admission = consult_recovery.consult_retention_admission();
                let (result, retained) = consult_recovery
                    .use_consult_inputs(
                        |collection_limits,
                         task_limits,
                         resolve_store,
                         admit_collection,
                         executor,
                         recover_child,
                         retain_child, historical_collection| {
                            let turn = match process.recover_one_root_consult_start_with_retention(
                                    selector,
                                    selected.access(),
                                    restart,
                                    admission,
                                    maximum_session_basis_bytes,
                                    operation_authority,
                                    collection_limits,
                                    task_limits,
                                    observed_at,
                                    resolve_store,
                                    |collection| admit_collection(collection),
                                    historical_collection,
                                    |started| {
                                        let key = consult_turn_key_from_started(started)?;
                                        retention_admission.admits(&key).then_some(()).ok_or(())
                                    },
                                    executor,
                                ) {
                                    Ok(turn) => turn,
                                    Err(consult_recovery::ResidentProcessConsultRecoveryError::CompletionBinding(failure)) => {
                                        let key = consult_turn_key_from_reference(
                                            failure.selector.clone(),
                                            failure.ready.parent(),
                                        )
                                        .map_err(|_| ResidentProgramTurnError::RecoveryUnavailable);
                                        return (
                                            Err(ResidentProgramTurnError::RecoveryUnavailable),
                                            key.ok().map(|key| ResidentProgramKeyedConsultTurn {
                                                key,
                                                turn: ResidentProgramConsultTurn::CompletionBinding(failure),
                                            }),
                                        );
                                    }
                                    Err(_) => {
                                        return (
                                            Err(ResidentProgramTurnError::RecoveryUnavailable),
                                            None,
                                        );
                                    }
                                };
                            match turn {
                                consult_recovery::ResidentProcessConsultRecoveryTurn::PolicyRequired(
                                    handoff,
                                ) => {
                                    let key = match consult_turn_key_from_started(handoff.started()) {
                                        Ok(key) => key,
                                        Err(()) => {
                                            return (
                                                Err(ResidentProgramTurnError::RecoveryUnavailable),
                                                None,
                                            );
                                        }
                                    };
                                    let (handoff, decision) = match process
                                        .evaluate_recovered_consultation_policy(handoff, policy)
                                    {
                                        Ok(evaluated) => evaluated,
                                        Err(failure) => {
                                            let (handoff, _) = failure.into_parts();
                                            return (
                                                Err(ResidentProgramTurnError::RecoveryUnavailable),
                                                Some(ResidentProgramKeyedConsultTurn {
                                                    key,
                                                    turn: ResidentProgramConsultTurn::PrePolicy(
                                                        consult_recovery::ResidentProcessConsultRecoveryTurn::PolicyRequired(
                                                            handoff,
                                                        ),
                                                    ),
                                                }),
                                            );
                                        }
                                    };
                                    let retained = match process
                                        .continue_recovered_consultation_policy(
                                            selector,
                                            selected.access(),
                                            handoff,
                                            decision,
                                            executor,
                                            |candidate| recover_child(candidate),
                                            |candidate, child, custody, anchor| {
                                                retain_child(candidate, child, custody, anchor)
                                            },
                                        ) {
                                        Ok(continuation) => {
                                            ResidentProgramConsultTurn::Policy(Box::new(continuation))
                                        }
                                        Err(refusal) => {
                                            ResidentProgramConsultTurn::PolicyRefusal(Box::new(refusal))
                                        }
                                    };
                                    (
                                        Ok(()),
                                        Some(ResidentProgramKeyedConsultTurn {
                                            key,
                                            turn: retained,
                                        }),
                                    )
                                }
                                consult_recovery::ResidentProcessConsultRecoveryTurn::Progress(
                                    ready,
                                ) => {
                                    let key = consult_turn_key_from_reference(
                                        ready.selector.clone(),
                                        ready.ready.parent(),
                                    )
                                    .map_err(|_| ResidentProgramTurnError::RecoveryUnavailable);
                                    match key {
                                        Ok(key) => (
                                            Ok(()),
                                            Some(ResidentProgramKeyedConsultTurn {
                                                key,
                                                turn: ResidentProgramConsultTurn::PrePolicy(
                                                    consult_recovery::ResidentProcessConsultRecoveryTurn::Progress(
                                                        ready,
                                                    ),
                                                ),
                                            }),
                                        ),
                                        Err(error) => (Err(error), None),
                                    }
                                }
                                consult_recovery::ResidentProcessConsultRecoveryTurn::NeedsRecovery(
                                    _,
                                ) => (Err(ResidentProgramTurnError::RecoveryUnavailable), None),
                                consult_recovery::ResidentProcessConsultRecoveryTurn::Exhausted
                                | consult_recovery::ResidentProcessConsultRecoveryTurn::SkippedOtherOwner(_)
                                | consult_recovery::ResidentProcessConsultRecoveryTurn::ReportedTerminal(_)
                                | consult_recovery::ResidentProcessConsultRecoveryTurn::Pending(_)
                                | consult_recovery::ResidentProcessConsultRecoveryTurn::StoppedEstablished(_) => {
                                    (Ok(()), None)
                                }
                            }
                        },
                    )
                    .map_err(|_| ResidentProgramTurnError::RecoveryUnavailable)?;
                if let Some(retained) = retained {
                    consult_recovery
                        .retain_keyed_consult_turn(retained)
                        .unwrap_or_else(|_| {
                            unreachable!(
                                "exclusive root Consult admission must remain valid until retention"
                            )
                        });
                }
                result
            },
        )
    }

    fn recover_consult_child<'store>(
        &mut self,
        process: &mut ResidentProcessHost<'store, WriterGuard>,
        policy: &mut PolicyEvaluator,
        selector: &ResidentDeploymentAuthoritySelector,
    ) -> Result<(), ResidentProgramTurnError> {
        let Self {
            deployments,
            operation_authority,
            recovery,
            consult_recovery,
            ..
        } = self;
        let mut matching = deployments
            .iter_mut()
            .filter(|deployment| deployment.matches(selector));
        let selected = matching
            .next()
            .ok_or(ResidentProgramTurnError::RecoveryUnavailable)?;
        if matching.next().is_some() {
            return Err(ResidentProgramTurnError::RecoveryUnavailable);
        }
        if let Some(pending) = consult_recovery.take_consult_child_terminal_custody() {
            return match process
                .continue_recovered_consult_child_terminal_custody(selected.access(), pending)
            {
                Ok(_) => Ok(()),
                Err(pending) => {
                    consult_recovery.retain_consult_child_terminal_custody(pending);
                    Err(ResidentProgramTurnError::RecoveryUnavailable)
                }
            };
        }
        let mut retained_child = consult_recovery.take_consult_child_turn();
        let result = recovery.with_background_recovery(
            selector,
            |restart, mut admission, maximum_session_basis_bytes, observed_at, owner| {
                let operation_context = restart.operation;
                let maximum_parent_depth = restart.maximum_parent_depth;
                let current_head = restart.current_head;
                let maximum_selection_bytes = restart.maximum_selection_bytes;
                let turn = match retained_child.take() {
                    Some(turn) => turn,
                    None => consult_recovery
                        .use_consult_inputs(
                            |_, _, resolve_store, admit_collection, _, _, _, _historical_collection| {
                                process.recover_one_consult_child_start(
                                    selector,
                                    selected.access(),
                                    ResidentProcessOperationRestartInput {
                                        committed_observation_limits: None,
                                        operation: operation_context,
                                        maximum_parent_depth,
                                        current_head,
                                        maximum_selection_bytes,
                                        a2a_task: None,
                                        a2a_cancel_task: None,
                                    },
                                    reborrow_operation_recovery_admission(&mut admission),
                                    maximum_session_basis_bytes,
                                    operation_authority,
                                    resolve_store,
                                    |collection| admit_collection(collection),
                                )
                            },
                        )
                        .map_err(|_| ResidentProgramTurnError::RecoveryUnavailable)?
                        .map_err(|_| ResidentProgramTurnError::RecoveryUnavailable)?,
                };
                let turn = admit_consult_child_recovery_turn(consult_recovery, turn)?;
                match turn {
                    consult_child_recovery_sweep::ResidentProcessConsultChildRecoveryTurn::Exhausted
                    | consult_child_recovery_sweep::ResidentProcessConsultChildRecoveryTurn::SkippedOtherOwner(_)
                    | consult_child_recovery_sweep::ResidentProcessConsultChildRecoveryTurn::AlreadyRetained(_) => Ok(()),
                    consult_child_recovery_sweep::ResidentProcessConsultChildRecoveryTurn::Recovered(handoff)
                        if matches!(
                            handoff.state(),
                            consult_child_recovery_sweep::ResidentProcessConsultChildRecoveryState::Pending(
                                pending
                            ) if matches!(
                                pending.as_ref(),
                                consult_child_recovery_sweep::ResidentProcessConsultChildPending::NestedConsult(_)
                            )
                        ) => {
                            let retention_admission =
                                consult_recovery.consult_retention_admission();
                            let Some(recovered_start) = handoff.nested_consult_start() else {
                                unreachable!("the match guard selected one nested Consult start")
                            };
                            let reserved_key = consult_turn_key_from_recovered(recovered_start)
                                .map_err(|_| ResidentProgramTurnError::RecoveryUnavailable)?;
                            if !retention_admission.admits(&reserved_key) {
                                consult_recovery.retain_consult_child_turn(
                                    consult_child_recovery_sweep::ResidentProcessConsultChildRecoveryTurn::Recovered(
                                        handoff,
                                    ),
                                );
                                return Err(ResidentProgramTurnError::RecoveryUnavailable);
                            }
                            let (cursor, start) = handoff
                                .into_nested_consult()
                                .map_err(|_| ResidentProgramTurnError::RecoveryUnavailable)?;
                            let turn = consult_recovery
                                .use_consult_inputs(
                                    |collection_limits,
                                     task_limits,
                                     resolve_store,
                                     admit_collection,
                                     executor,
                                     recover_child,
                                     retain_child, _historical_collection| {
                                        let restart::ResidentProcessRecoveredOperationStart {
                                            selector: recovered_selector,
                                            control,
                                            generation_digest: _,
                                            lineage: _,
                                        } = *start;
                                        if &recovered_selector != selector {
                                            return Err(ResidentProgramTurnError::RecoveryUnavailable);
                                        }
                                        let (parent, protected) = control.into_parts();
                                        let started = process
                                            .rebuild_consultation_start_from_protected_intent(
                                                selector,
                                                &selected.store,
                                                parent.into_operation(),
                                                operation_context,
                                                maximum_parent_depth,
                                                &mut *selected.custody,
                                                selected.sessions,
                                                &mut *selected.anchor,
                                                reborrow_operation_recovery_admission(
                                                    &mut admission,
                                                ),
                                            )
                                            .map_err(|_| {
                                                ResidentProgramTurnError::RecoveryUnavailable
                                            })?;
                                        let key = reserved_key.clone();
                                        let current = process
                                            .recover_consultation_collection(
                                                selector,
                                                selected.access(),
                                                operation_authority,
                                                started.parent(),
                                                &protected,
                                                collection_limits,
                                                resolve_store,
                                                |collection| admit_collection(collection),
                                            )
                                            .map_err(|_| {
                                                ResidentProgramTurnError::RecoveryUnavailable
                                            })?
                                            .ok_or(
                                                ResidentProgramTurnError::RecoveryUnavailable,
                                            )?;
                                        if !started.matches_collection(current.collection()) {
                                            return Err(
                                                ResidentProgramTurnError::RecoveryUnavailable,
                                            );
                                        }
                                        let turn = match process
                                            .advance_recovered_consultation_pre_policy(
                                                started,
                                                task_limits,
                                                observed_at,
                                                current,
                                                executor,
                                            ) {
                                            Ok(turn) => turn,
                                            Err(consult_recovery::ResidentProcessConsultRecoveryError::CompletionBinding(failure)) => {
                                                return Ok((
                                                    key,
                                                    ResidentProgramConsultTurn::CompletionBinding(failure),
                                                ));
                                            }
                                            Err(_) => {
                                                return Err(ResidentProgramTurnError::RecoveryUnavailable);
                                            }
                                        };
                                        let turn = match turn {
                                            consult_recovery::ResidentProcessConsultRecoveryTurn::PolicyRequired(handoff) => {
                                                let (handoff, decision) = match process
                                                    .evaluate_recovered_consultation_policy(
                                                        handoff,
                                                        policy,
                                                    ) {
                                                    Ok(evaluated) => evaluated,
                                                    Err(failure) => {
                                                        let (handoff, _) = failure.into_parts();
                                                        return Ok((
                                                            key,
                                                            ResidentProgramConsultTurn::PolicyUnavailable(handoff),
                                                        ));
                                                    }
                                                };
                                                match process
                                                    .continue_recovered_consultation_policy(
                                                        selector,
                                                        selected.access(),
                                                        handoff,
                                                        decision,
                                                        executor,
                                                        |candidate| recover_child(candidate),
                                                        |candidate, child, custody, anchor| {
                                                            retain_child(
                                                                candidate,
                                                                child,
                                                                custody,
                                                                anchor,
                                                            )
                                                        },
                                                    ) {
                                                    Ok(continuation) => {
                                                        ResidentProgramConsultTurn::Policy(
                                                            Box::new(continuation),
                                                        )
                                                    }
                                                    Err(refusal) => {
                                                        ResidentProgramConsultTurn::PolicyRefusal(
                                                            Box::new(refusal),
                                                        )
                                                    }
                                                }
                                            }
                                            other => ResidentProgramConsultTurn::PrePolicy(other),
                                        };
                                        Ok((key, turn))
                                    },
                                )
                                .map_err(|_| ResidentProgramTurnError::RecoveryUnavailable)??;
                            let (key, turn) = turn;
                            let transfers = turn.transfers_recovery_source();
                            consult_recovery
                                .retain_keyed_consult_turn(ResidentProgramKeyedConsultTurn {
                                    key,
                                    turn,
                                })
                                .unwrap_or_else(|_| {
                                    unreachable!(
                                        "exclusive child Consult admission must remain valid until retention"
                                    )
                                });
                            if transfers {
                                process
                                    .complete_recovered_consult_child_pending(
                                        &selected.store,
                                        cursor,
                                    )
                                    .map_err(|_| {
                                        ResidentProgramTurnError::RecoveryUnavailable
                                    })?;
                            }
                            if transfers {
                                Ok(())
                            } else {
                                Err(ResidentProgramTurnError::RecoveryUnavailable)
                            }
                        }
                    consult_child_recovery_sweep::ResidentProcessConsultChildRecoveryTurn::Recovered(handoff)
                        if matches!(
                            handoff.state(),
                            consult_child_recovery_sweep::ResidentProcessConsultChildRecoveryState::Terminal(_)
                        ) => {
                            let mut terminal_handoff = Some(handoff);
                            let committed = match consult_recovery
                                .with_consult_child_terminal_inputs(|inputs| {
                                    let handoff = terminal_handoff
                                        .take()
                                        .expect("terminal child handoff is consumed once");
                                    process
                                        .commit_recovered_consult_child_terminal(
                                        selected.access(),
                                        handoff,
                                        inputs.input,
                                        inputs.resolve_store,
                                        |slot, evidence| {
                                            (inputs.admit_evidence)(slot, evidence)
                                        },
                                        |collection| {
                                            (inputs.admit_collection)(collection)
                                        },
                                        |old_ref, old_digest, new_ref, new_digest| {
                                            (inputs.admit_rotation)(
                                                old_ref,
                                                old_digest,
                                                new_ref,
                                                new_digest,
                                            )
                                        },
                                        |bytes, preimage, digest| {
                                            (inputs.sign_observation)(bytes, preimage, digest)
                                        },
                                        |bytes, preimage, digest| {
                                            (inputs.sign_checkpoint)(bytes, preimage, digest)
                                        },
                                    )
                                        .map_err(|error| error.into_handoff())
                                }) {
                                Ok(committed) => committed,
                                Err(_) => {
                                    if let Some(handoff) = terminal_handoff.take() {
                                        return Err(
                                            retain_consult_child_terminal_input_refusal::<
                                                StopInput,
                                                _,
                                            >(consult_recovery, handoff),
                                        );
                                    }
                                    return Err(ResidentProgramTurnError::RecoveryUnavailable);
                                }
                            };
                            let turn = match committed {
                                Ok(turn) => turn,
                                Err(handoff) => {
                                    consult_recovery.retain_consult_child_turn(
                                        consult_child_recovery_sweep::ResidentProcessConsultChildRecoveryTurn::Recovered(
                                            handoff,
                                        ),
                                    );
                                    return Err(ResidentProgramTurnError::RecoveryUnavailable);
                                }
                            };
                            match turn {
                                consult_child_recovery_sweep::ResidentProcessConsultChildTerminalCommitTurn::Completed(_) => Ok(()),
                                consult_child_recovery_sweep::ResidentProcessConsultChildTerminalCommitTurn::CustodyPending(pending) => {
                                    consult_recovery.retain_consult_child_terminal_custody(pending);
                                    Err(ResidentProgramTurnError::RecoveryUnavailable)
                                }
                            }
                        }
                    consult_child_recovery_sweep::ResidentProcessConsultChildRecoveryTurn::Recovered(handoff) => {
                        let (cursor, disposition) = match handoff.into_pending_disposition() {
                            Ok(pending) => pending,
                            Err(handoff) => {
                                consult_recovery.retain_consult_child_turn(
                                    consult_child_recovery_sweep::ResidentProcessConsultChildRecoveryTurn::Recovered(handoff),
                                );
                                return Err(ResidentProgramTurnError::RecoveryUnavailable);
                            }
                        };
                        let store = selected.store.clone();
                        let routed = operation_recovery_sweep::route_root_provider_disposition(
                            process,
                            disposition,
                            selected.access(),
                            owner,
                        )
                        .map_err(|_| ResidentProgramTurnError::RecoveryUnavailable)?;
                        match routed {
                            operation_recovery_sweep::ResidentProcessOperationRecoveryTurn::ReportedRecoveryOnly(_) => {}
                            operation_recovery_sweep::ResidentProcessOperationRecoveryTurn::DeferredUnsupported {
                                disposition: Some(disposition),
                                ..
                            } => {
                                owner.retain_deferred_recovery(disposition);
                                return Err(ResidentProgramTurnError::RecoveryUnavailable);
                            }
                            _ => {}
                        }
                        process
                            .complete_recovered_consult_child_pending(&store, cursor)
                            .map_err(|_| ResidentProgramTurnError::RecoveryUnavailable)
                    }
                }
            },
        );
        if let Some(turn) = retained_child {
            consult_recovery.retain_consult_child_turn(turn);
        }
        result
    }

    fn recover_delegate<'store>(
        &mut self,
        process: &mut ResidentProcessHost<'store, WriterGuard>,
        selector: &ResidentDeploymentAuthoritySelector,
    ) -> Result<(), ResidentProgramTurnError> {
        let Self {
            deployments,
            recovery,
            ..
        } = self;
        let mut matching = deployments
            .iter_mut()
            .filter(|deployment| deployment.matches(selector));
        let selected = matching
            .next()
            .ok_or(ResidentProgramTurnError::RecoveryUnavailable)?;
        if matching.next().is_some() {
            return Err(ResidentProgramTurnError::RecoveryUnavailable);
        }
        recovery.with_background_recovery(
            selector,
            |restart, admission, maximum_session_basis_bytes, _observed_at, owner| {
                match process.recover_one_delegate_start(
                        selector,
                        selected.access(),
                        restart,
                        admission,
                        maximum_session_basis_bytes,
                        owner,
                    ) {
                    Ok(delegate_recovery_sweep::ResidentProcessDelegateRecoveryTurn::ReportedRecoveryOnly(_)) => Ok(()),
                    Ok(delegate_recovery_sweep::ResidentProcessDelegateRecoveryTurn::DeferredResolution(recovered)) => {
                        owner.retain_deferred_recovery(Box::new(
                            ResidentProcessOperationRestartDisposition::OperationResolution(
                                recovered,
                            ),
                        ));
                        Err(ResidentProgramTurnError::RecoveryUnavailable)
                    }
                    Ok(_) => Ok(()),
                    Err(_) => Err(ResidentProgramTurnError::RecoveryUnavailable),
                }
            },
        )
    }

    fn recover_publication<'store>(
        &mut self,
        process: &mut ResidentProcessHost<'store, WriterGuard>,
        selector: &ResidentDeploymentAuthoritySelector,
    ) -> Result<(), ResidentProgramTurnError> {
        let Self {
            deployments,
            recovery,
            ..
        } = self;
        let mut matching = deployments
            .iter_mut()
            .filter(|deployment| deployment.matches(selector));
        let selected = matching
            .next()
            .ok_or(ResidentProgramTurnError::RecoveryUnavailable)?;
        if matching.next().is_some() {
            return Err(ResidentProgramTurnError::RecoveryUnavailable);
        }
        recovery.recover_publication(process, selector, selected.access())
    }
}

fn continue_one_retained_cancellation<WriterGuard, Authority, Recovery>(
    process: &mut ResidentProcessHost<'_, WriterGuard>,
    deployments: &mut [ResidentProgramDeploymentOperationPorts<'_>],
    authority: &mut Authority,
    recovery: &mut Recovery,
) -> ResidentProgramRetainedPhaseTurn
where
    WriterGuard: PrincipalWriterGuardPort,
    Authority: crate::resident::AmbientSessionAuthorityPort,
    Recovery: ResidentProgramCancellationInputPort,
{
    let Some(selector) = recovery.retained_cancellation_selector().cloned() else {
        return ResidentProgramRetainedPhaseTurn::Idle;
    };
    let mut matching = deployments
        .iter_mut()
        .filter(|deployment| deployment.matches(&selector));
    let Some(selected) = matching.next() else {
        return ResidentProgramRetainedPhaseTurn::Deferred;
    };
    if matching.next().is_some() {
        return ResidentProgramRetainedPhaseTurn::Deferred;
    }
    recovery.continue_one_retained_cancellation(process, selected.access(), authority)
}

fn continue_one_retained_invoke<WriterGuard, Authority, Recovery>(
    process: &mut ResidentProcessHost<'_, WriterGuard>,
    deployments: &mut [ResidentProgramDeploymentOperationPorts<'_>],
    authority: &mut Authority,
    recovery: &mut Recovery,
) -> ResidentProgramRetainedPhaseTurn
where
    WriterGuard: PrincipalWriterGuardPort,
    Authority: crate::resident::AmbientSessionAuthorityPort,
    Recovery: ResidentProgramRootRecoveryInputPort,
{
    let Some(selector) = recovery.retained_invoke_selector().cloned() else {
        return ResidentProgramRetainedPhaseTurn::Idle;
    };
    let mut matching = deployments
        .iter_mut()
        .filter(|deployment| deployment.matches(&selector));
    let Some(selected) = matching.next() else {
        return ResidentProgramRetainedPhaseTurn::Deferred;
    };
    if matching.next().is_some() {
        return ResidentProgramRetainedPhaseTurn::Deferred;
    }
    recovery.continue_one_retained_invoke(process, selected.access(), authority)
}

fn continue_one_recovery_phase<WriterGuard, Authority, Recovery>(
    process: &mut ResidentProcessHost<'_, WriterGuard>,
    deployments: &mut [ResidentProgramDeploymentOperationPorts<'_>],
    authority: &mut Authority,
    recovery: &mut Recovery,
    next: &mut ResidentProgramRecoveryPhaseLane,
) -> ResidentProgramRetainedPhaseTurn
where
    WriterGuard: PrincipalWriterGuardPort,
    Authority: crate::resident::AmbientSessionAuthorityPort,
    Recovery: ResidentProgramRootRecoveryInputPort + ResidentProgramCancellationInputPort,
{
    let available = [
        (
            ResidentProgramRecoveryPhaseLane::Invoke,
            recovery.has_runnable_retained_invoke(),
        ),
        (
            ResidentProgramRecoveryPhaseLane::Delegate,
            recovery.has_runnable_retained_delegate(),
        ),
        (
            ResidentProgramRecoveryPhaseLane::Cancellation,
            recovery.has_retained_cancellation(),
        ),
    ];
    let mut selected = None;
    let mut candidate = *next;
    for _ in 0..available.len() {
        if available
            .iter()
            .any(|(lane, runnable)| *lane == candidate && *runnable)
        {
            selected = Some(candidate);
            *next = candidate.other();
            break;
        }
        candidate = candidate.other();
    }
    let Some(selected) = selected else {
        return ResidentProgramRetainedPhaseTurn::Idle;
    };
    match selected {
        ResidentProgramRecoveryPhaseLane::Invoke => {
            continue_one_retained_invoke(process, deployments, authority, recovery)
        }
        ResidentProgramRecoveryPhaseLane::Delegate => {
            continue_one_retained_delegate(process, deployments, authority, recovery)
        }
        ResidentProgramRecoveryPhaseLane::Cancellation => {
            continue_one_retained_cancellation(process, deployments, authority, recovery)
        }
    }
}

fn continue_one_retained_delegate<WriterGuard, Authority, Recovery>(
    process: &mut ResidentProcessHost<'_, WriterGuard>,
    deployments: &mut [ResidentProgramDeploymentOperationPorts<'_>],
    authority: &mut Authority,
    recovery: &mut Recovery,
) -> ResidentProgramRetainedPhaseTurn
where
    WriterGuard: PrincipalWriterGuardPort,
    Authority: crate::resident::AmbientSessionAuthorityPort,
    Recovery: ResidentProgramRootRecoveryInputPort,
{
    let Some(selector) = recovery.retained_delegate_selector().cloned() else {
        return ResidentProgramRetainedPhaseTurn::Idle;
    };
    let mut matching = deployments
        .iter_mut()
        .filter(|deployment| deployment.matches(&selector));
    let Some(selected) = matching.next() else {
        return ResidentProgramRetainedPhaseTurn::Deferred;
    };
    if matching.next().is_some() {
        return ResidentProgramRetainedPhaseTurn::Deferred;
    }
    recovery.continue_one_retained_delegate(process, selected.access(), authority)
}

impl<
        'historical,
        'ports,
        WriterGuard,
        PeerWitness,
        Peer,
        Evidence,
        Clock,
        SessionAuthority,
        Signature,
        Random,
        OperationAuthority,
        Cryptography,
        Recovery,
        ConsultRecovery,
    > ResidentProgramObserveTurnOwner<WriterGuard>
    for ResidentProgramOperationTurnOwner<
        'historical,
        'ports,
        PeerWitness,
        Peer,
        Evidence,
        Clock,
        SessionAuthority,
        Signature,
        Random,
        OperationAuthority,
        Cryptography,
        Recovery,
        ConsultRecovery,
    >
where
    WriterGuard: PrincipalWriterGuardPort,
    Peer: ResidentProgramPeerAdmissionPort<Witness = PeerWitness>
        + ResidentProgramObservePeerAdmissionPort,
    Evidence: ResidentCurrentRequestEvidencePort<PeerWitness>,
    Clock: pale_ports::WallClockSourcePort,
    SessionAuthority: crate::resident::AmbientSessionAuthorityPort,
    Signature: ResidentProgramCallerSignaturePort,
    OperationAuthority: crate::resident::AmbientSessionAuthorityPort,
    Recovery: ResidentProgramObserveInputPort<'ports>,
{
    type PeerWitness = PeerWitness;

    fn admit_observe_peer(
        &mut self,
        peer: LocalIpcPeerCredentials,
        request: &ResidentObserveFrameRequest,
    ) -> Result<Self::PeerWitness, InvalidResidentRecordV1> {
        self.peer.admit_observe_peer(peer, request)
    }

    fn answer_observe<'store>(
        &mut self,
        process: &mut ResidentProcessHost<'store, WriterGuard>,
        deployment_authorities: &[ResidentProgramDeploymentAuthority],
        peer: &Self::PeerWitness,
        frame: &ResidentObserveFrameRequest,
    ) -> Result<(OperationControlObservation, Instant), ResidentObserveFrameFailure> {
        answer_observe_with(
            &mut self.current,
            &mut self.deployments,
            &mut self.operation_authority,
            &mut self.recovery,
            process,
            deployment_authorities,
            peer,
            frame,
        )
    }
}

#[allow(clippy::too_many_arguments)]
pub(in crate::resident) fn answer_observe_with<
    'historical,
    'ports,
    'store,
    WriterGuard,
    PeerWitness,
    Evidence,
    Clock,
    SessionAuthority,
    Signature,
    OperationAuthority,
    Recovery,
>(
    current: &mut ResidentProgramCurrentRequestAdmission<
        'historical,
        PeerWitness,
        Evidence,
        Clock,
        SessionAuthority,
        Signature,
    >,
    deployments: &mut [ResidentProgramDeploymentOperationPorts<'ports>],
    operation_authority: &mut OperationAuthority,
    recovery: &mut Recovery,
    process: &mut ResidentProcessHost<'store, WriterGuard>,
    deployment_authorities: &[ResidentProgramDeploymentAuthority],
    peer: &PeerWitness,
    frame: &ResidentObserveFrameRequest,
) -> Result<(OperationControlObservation, Instant), ResidentObserveFrameFailure>
where
    WriterGuard: PrincipalWriterGuardPort,
    Evidence: ResidentCurrentRequestEvidencePort<PeerWitness>,
    Clock: pale_ports::WallClockSourcePort,
    SessionAuthority: crate::resident::AmbientSessionAuthorityPort,
    Signature: ResidentProgramCallerSignaturePort,
    OperationAuthority: crate::resident::AmbientSessionAuthorityPort,
    Recovery: ResidentProgramObserveInputPort<'ports>,
{
    let admitted = current.admit_observe(process, deployment_authorities, peer, frame)?;
    let locator = pale_ports::ControlRecordCoordinate::new(
        pale_core::records::ResidentRecordRefV1::parse(
            frame.operation_handle_record_ref().to_owned(),
        )
        .map_err(|_| ResidentObserveFrameFailure::InvalidRequest)?,
        pale_ports::ControlRecordDigest::parse(frame.operation_handle_sha256().to_owned())
            .map_err(|_| ResidentObserveFrameFailure::InvalidRequest)?,
    );
    let selector = admitted.selector().clone();
    let maximum_request_bytes = current.maximum_request_bytes;
    let mut matching = deployments
        .iter_mut()
        .filter(|deployment| deployment.matches(&selector));
    let selected = matching
        .next()
        .ok_or(ResidentObserveFrameFailure::ResidentUnavailable)?;
    if matching.next().is_some() {
        return Err(ResidentObserveFrameFailure::ResidentUnavailable);
    }
    recovery.with_observe_inputs(&locator, |inputs| {
        admitted.with_request_context(|request_context| {
            process
                .observe_request(
                    &selector,
                    selected.access(),
                    operation_authority,
                    observe_request::ResidentProcessObserveRequestInput {
                        request_bytes: frame.signed_common_request().as_bytes(),
                        maximum_request_bytes,
                        locator: &locator,
                        recovery: inputs.recovery,
                        authorized_at: admitted.now,
                    },
                    observe_request::ResidentProcessObserveRequestAdmission {
                        request: request_context,
                        verify_caller_signature: inputs.verify_caller_signature,
                        admit_replay: inputs.admit_replay,
                        recovery: inputs.recovery_admission,
                        authorize_disclosure: inputs.authorize_disclosure,
                    },
                )
                .map(|observed| (observed, inputs.response_deadline))
                .map_err(|error| observe_request_host::frame_failure(&error))
        })
    })?
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn answer_cancel_request<WriterGuard, Authority, Recovery>(
    process: &mut ResidentProcessHost<'_, WriterGuard>,
    selector: &ResidentDeploymentAuthoritySelector,
    access: ResidentDeploymentOperationAccess<'_>,
    operation_authority: &mut Authority,
    recovery_owner: &mut Recovery,
    authorized_at: pale_core::records::UnixTimeV1,
    request: &CommonRequestEnvelopeV1,
    resolution: &ResidentProgramOperationResolution,
    locator: Option<&operation_locator::ResidentProgramOperationLocator>,
    restart: ResidentProcessOperationRestartInput<'_>,
    mut recovery_admission: OperationControlRecoveryAdmission<'_>,
    checkpoint_authority: &pale_core::records::ControlCheckpointSigningAuthorityAdmissionContextV1<
        '_,
    >,
) -> Result<operation_request_host::ResidentOperationRequestOutcome, ResidentOperationFrameFailure>
where
    WriterGuard: PrincipalWriterGuardPort,
    Authority: crate::resident::AmbientSessionAuthorityPort,
    Recovery: ResidentProgramCancellationInputPort,
{
    let locator = locator.ok_or(ResidentOperationFrameFailure::InvalidRequest)?;
    let response_locator = resolution
        .locator()
        .cloned()
        .ok_or(ResidentOperationFrameFailure::InvalidRequest)?;
    let ResidentDeploymentOperationAccess {
        store,
        custody,
        sessions,
        anchor,
    } = access;
    let operation_context = restart.operation;
    let maximum_parent_depth = restart.maximum_parent_depth;
    let current_head = restart.current_head;
    let target = crate::resident::recover_operation_control(
        locator.recovery_input(operation_context, maximum_parent_depth, current_head),
        &mut *custody,
        sessions,
        &mut *anchor,
        reborrow_operation_recovery_admission(&mut recovery_admission),
    )
    .map_err(|_| ResidentOperationFrameFailure::ResidentUnavailable)?;
    locator
        .admit_recovered(&target)
        .map_err(|_| ResidentOperationFrameFailure::InvalidRequest)?;
    let operation_class = *target.operation().operation().handle().1.operation_class();
    let reservation =
        recovery_owner.reserve_cancellation(selector, request, locator.coordinate())?;
    let handoff = match recovery_owner.with_selected_cancellation_inputs(
        selector,
        request,
        locator.coordinate(),
        |ResidentProgramCancellationInputs {
             allocation,
             authorize,
             resolve_store,
             admit_rotation,
             sign_observation,
             sign_checkpoint,
             admit_stop,
             admit_collection,
         }| {
            let input =
                crate::resident::operation_service::ResidentOperationCancellationInput {
                    recovery: locator.recovery_input(
                        operation_context,
                        maximum_parent_depth,
                        current_head,
                    ),
                    request,
                    authorized_at,
                    request_ref: allocation.request_ref,
                    observation_ref: allocation.observation_ref,
                    signing: allocation.signing,
                    checkpoint: allocation.checkpoint,
                    checkpoint_issued_at: allocation.checkpoint_issued_at,
                    checkpoint_schema: allocation.checkpoint_schema,
                    checkpoint_authority,
                    accumulator_limits: allocation.accumulator_limits,
                };
            let cancellation_admission =
                crate::resident::operation_service::ResidentOperationCancellationAdmission {
                    recovery: reborrow_operation_recovery_admission(&mut recovery_admission),
                    authorize,
                    resolve_store,
                    admit_rotation,
                    sign_observation,
                    sign_checkpoint,
                };
            let disposition = if operation_class == pale_core::records::OperationClassV1::Consult {
                match process.request_consultation_cancellation_from_recovered(
                    selector,
                    ResidentDeploymentOperationAccess {
                        store,
                        custody: &mut *custody,
                        sessions,
                        anchor: &mut *anchor,
                    },
                    operation_authority,
                    target,
                    input,
                    cancellation_admission,
                    allocation.consultation_limits,
                    |collection, summary| admit_stop(collection, summary),
                    |collection| admit_collection(collection),
                ) {
                    Ok(crate::resident::operation_service::ResidentConsultationCancellationDisposition::Retained(_)) => {
                        ResidentOperationFrameCancellationDisposition::ConsultationStopped
                    }
                    Ok(crate::resident::operation_service::ResidentConsultationCancellationDisposition::Established(_)) => {
                        ResidentOperationFrameCancellationDisposition::TerminalAlreadyEstablished
                    }
                    Ok(crate::resident::operation_service::ResidentConsultationCancellationDisposition::AlreadyRequested(_)) => {
                        ResidentOperationFrameCancellationDisposition::AlreadyRequested
                    }
                    Ok(crate::resident::operation_service::ResidentConsultationCancellationDisposition::ConsultationCollected(_)) => {
                        ResidentOperationFrameCancellationDisposition::ConsultationCollected
                    }
                    Ok(crate::resident::operation_service::ResidentConsultationCancellationDisposition::ReconciliationOnly(_)) => {
                        ResidentOperationFrameCancellationDisposition::ReconciliationOnly
                    }
                    Ok(crate::resident::operation_service::ResidentConsultationCancellationDisposition::NotCollecting(_)) => {
                        ResidentOperationFrameCancellationDisposition::NotCollecting
                    }
                    Err(ResidentProcessConsultationCancellationError::Custody(pending)) => {
                        return ResidentProgramCancellationHandoff::ConsultationCustody(pending);
                    }
                    Err(ResidentProcessConsultationCancellationError::Preparation(pending)) => {
                        return ResidentProgramCancellationHandoff::ConsultationPreparation(
                            pending,
                        );
                    }
                    Err(
                        ResidentProcessConsultationCancellationError::Selection(_)
                        | ResidentProcessConsultationCancellationError::Cancellation(_),
                    ) => {
                        return ResidentProgramCancellationHandoff::Failure(
                            ResidentOperationFrameFailure::ResidentUnavailable,
                        );
                    }
                }
            } else {
                match process.request_cancellation_from_recovered(
                    selector,
                    ResidentDeploymentOperationAccess {
                        store,
                        custody: &mut *custody,
                        sessions,
                        anchor: &mut *anchor,
                    },
                    operation_authority,
                    target,
                    input,
                    allocation.local_completion,
                    cancellation_admission,
                ) {
                    Ok(ResidentProcessCancellationOutcome::Operation(outcome)) => match outcome {
                        crate::resident::operation_service::ResidentOperationCancellationOutcome::Established(_) => ResidentOperationFrameCancellationDisposition::TerminalAlreadyEstablished,
                        crate::resident::operation_service::ResidentOperationCancellationOutcome::AlreadyRequested(_) => ResidentOperationFrameCancellationDisposition::AlreadyRequested,
                        crate::resident::operation_service::ResidentOperationCancellationOutcome::LocalRequestRetained { .. } => ResidentOperationFrameCancellationDisposition::RequestRetained,
                        crate::resident::operation_service::ResidentOperationCancellationOutcome::LocallyCancelled { .. } => ResidentOperationFrameCancellationDisposition::LocallyCancelled,
                        crate::resident::operation_service::ResidentOperationCancellationOutcome::DispatchedRequestRetained { .. } => ResidentOperationFrameCancellationDisposition::DispatchedRequestRetained,
                        crate::resident::operation_service::ResidentOperationCancellationOutcome::A2aDelegateReadStopped { .. } => return ResidentProgramCancellationHandoff::Failure(ResidentOperationFrameFailure::ResidentUnavailable),
                        crate::resident::operation_service::ResidentOperationCancellationOutcome::ConsultationCollected(_) => ResidentOperationFrameCancellationDisposition::ConsultationCollected,
                        crate::resident::operation_service::ResidentOperationCancellationOutcome::ReconciliationOnly(_) => ResidentOperationFrameCancellationDisposition::ReconciliationOnly,
                        crate::resident::operation_service::ResidentOperationCancellationOutcome::DispatchOwnerRequired(_) => ResidentOperationFrameCancellationDisposition::DispatchOwnerRequired,
                    },
                    Ok(ResidentProcessCancellationOutcome::A2aDelegateReadStopped {
                        continuation,
                        ..
                    }) => {
                        return ResidentProgramCancellationHandoff::A2aCancellation {
                            pending: Box::new(continuation),
                            outcome: operation_request_host::ResidentOperationRequestOutcome::CancellationDisposition {
                                locator: response_locator,
                                disposition: ResidentOperationFrameCancellationDisposition::A2aResponseReadStopped,
                            },
                        };
                    }
                    Err(ResidentProcessCancellationError::DispatchedCustody(pending)) => {
                        return ResidentProgramCancellationHandoff::DispatchedCustody(pending);
                    }
                    Err(ResidentProcessCancellationError::Custody(pending)) => {
                        return ResidentProgramCancellationHandoff::CancellationCustody(pending);
                    }
                    Err(
                        ResidentProcessCancellationError::Selection(_)
                        | ResidentProcessCancellationError::Cancellation(_),
                    ) => {
                        return ResidentProgramCancellationHandoff::Failure(
                            ResidentOperationFrameFailure::ResidentUnavailable,
                        );
                    }
                }
            };
            ResidentProgramCancellationHandoff::Outcome(
                operation_request_host::ResidentOperationRequestOutcome::CancellationDisposition {
                    locator: response_locator,
                    disposition,
                },
            )
        },
    ) {
        Ok(handoff) => handoff,
        Err(failure) => {
            recovery_owner.release_cancellation(reservation);
            return Err(failure);
        }
    };
    settle_cancellation_handoff(recovery_owner, reservation, handoff)
}

#[must_use = "retain every cancellation custody handoff before returning a frame result"]
pub(super) enum ResidentProgramCancellationHandoff {
    Outcome(operation_request_host::ResidentOperationRequestOutcome),
    A2aCancellation {
        pending: Box<ResidentProcessA2aDelegatePoll>,
        outcome: operation_request_host::ResidentOperationRequestOutcome,
    },
    DispatchedCustody(Box<ResidentProcessDispatchedCancellationCustody>),
    CancellationCustody(Box<ResidentProcessCancellationCustody>),
    ConsultationCustody(Box<ResidentProcessConsultationCancellationCustody>),
    ConsultationPreparation(Box<ResidentProcessConsultationCancellationPreparation>),
    Failure(ResidentOperationFrameFailure),
}

pub(super) fn settle_cancellation_handoff<Recovery>(
    recovery_owner: &mut Recovery,
    reservation: Recovery::Reservation,
    handoff: ResidentProgramCancellationHandoff,
) -> Result<operation_request_host::ResidentOperationRequestOutcome, ResidentOperationFrameFailure>
where
    Recovery: ResidentProgramCancellationInputPort,
{
    match handoff {
        ResidentProgramCancellationHandoff::Outcome(outcome) => {
            recovery_owner.release_cancellation(reservation);
            Ok(outcome)
        }
        ResidentProgramCancellationHandoff::A2aCancellation { pending, outcome } => {
            recovery_owner.retain_a2a_cancellation(reservation, *pending);
            Ok(outcome)
        }
        ResidentProgramCancellationHandoff::DispatchedCustody(pending) => {
            recovery_owner.retain_dispatched_cancellation(reservation, pending);
            Err(ResidentOperationFrameFailure::ResidentUnavailable)
        }
        ResidentProgramCancellationHandoff::CancellationCustody(pending) => {
            recovery_owner.retain_cancellation_custody(reservation, pending);
            Err(ResidentOperationFrameFailure::ResidentUnavailable)
        }
        ResidentProgramCancellationHandoff::ConsultationCustody(pending) => {
            recovery_owner.retain_consultation_cancellation(reservation, pending);
            Err(ResidentOperationFrameFailure::ResidentUnavailable)
        }
        ResidentProgramCancellationHandoff::ConsultationPreparation(pending) => {
            recovery_owner.retain_consultation_cancellation_preparation(reservation, pending);
            Err(ResidentOperationFrameFailure::ResidentUnavailable)
        }
        ResidentProgramCancellationHandoff::Failure(failure) => {
            recovery_owner.release_cancellation(reservation);
            Err(failure)
        }
    }
}

fn same_operation_request_identity(
    left: &CommonRequestEnvelopeV1,
    right: &CommonRequestEnvelopeV1,
) -> Result<bool, ResidentOperationFrameFailure> {
    let left =
        operation_request_key(left).map_err(|_| ResidentOperationFrameFailure::InvalidRequest)?;
    let right =
        operation_request_key(right).map_err(|_| ResidentOperationFrameFailure::InvalidRequest)?;
    Ok(left == right)
}

fn map_request_resolution_error(
    error: &ResidentProcessInvokeRequestResolutionError,
) -> ResidentOperationFrameFailure {
    match error {
        ResidentProcessInvokeRequestResolutionError::Allocation(
            ResidentProcessOperationAllocationError::Allocation(
                OperationStartAllocationError::Recovery(OperationRecoveryError::RequestConflict),
            ),
        ) => ResidentOperationFrameFailure::ReplayConflict,
        _ => ResidentOperationFrameFailure::ResidentUnavailable,
    }
}

fn map_root_start_error(error: ResidentProcessRootError) -> ResidentOperationFrameFailure {
    match &error {
        ResidentProcessRootError::Capacity => ResidentOperationFrameFailure::ResidentUnavailable,
        ResidentProcessRootError::Resolution(reason) => map_request_resolution_error(reason),
        _ => ResidentOperationFrameFailure::ResidentUnavailable,
    }
}

fn map_consult_start_error(
    error: ResidentProcessConsultRootError,
) -> ResidentOperationFrameFailure {
    match &error {
        ResidentProcessConsultRootError::Capacity => {
            ResidentOperationFrameFailure::ResidentUnavailable
        }
        ResidentProcessConsultRootError::Resolution(reason) => map_request_resolution_error(reason),
        _ => ResidentOperationFrameFailure::ResidentUnavailable,
    }
}

fn map_delegate_start_error(_: ResidentProcessDelegateRootError) -> ResidentOperationFrameFailure {
    ResidentOperationFrameFailure::ResidentUnavailable
}

fn reborrow_operation_recovery_admission<'short>(
    admission: &'short mut OperationControlRecoveryAdmission<'_>,
) -> OperationControlRecoveryAdmission<'short> {
    OperationControlRecoveryAdmission {
        admit_request: &mut *admission.admit_request,
        admit_handle_authority: &mut *admission.admit_handle_authority,
        verify_handle_signature: &mut *admission.verify_handle_signature,
        admit_checkpoint: &mut *admission.admit_checkpoint,
        resolve_observation_signer: &mut *admission.resolve_observation_signer,
        admit_observation_state: &mut *admission.admit_observation_state,
        admit_rotation: &mut *admission.admit_rotation,
        verify_observation_signature: &mut *admission.verify_observation_signature,
        verify_checkpoint_signature: &mut *admission.verify_checkpoint_signature,
    }
}

impl<
        'root,
        'restart,
        'admission,
        'ports,
        WriterGuard,
        PolicyEvaluator,
        StopInput,
        Peer,
        Random,
        Authority,
        Cryptography,
    > ResidentProgramTurnOwner<WriterGuard, PolicyEvaluator, StopInput>
    for ResidentProgramInvokeTurnOwner<
        'root,
        'restart,
        'admission,
        'ports,
        Peer,
        Random,
        Authority,
        Cryptography,
    >
where
    Peer: ResidentProgramPeerAdmissionPort,
    Random: CryptographicRandomSourcePort,
    Authority: crate::resident::AmbientSessionAuthorityPort,
    Cryptography: ResidentProgramInvokeCryptographyPort,
    PolicyEvaluator: ResidentProcessInvokePolicyEvaluator,
{
    type PeerWitness = Peer::Witness;

    fn admit_operation_peer(
        &mut self,
        peer: LocalIpcPeerCredentials,
        request: &ResidentOperationFrameRequest,
    ) -> Result<Self::PeerWitness, InvalidResidentRecordV1> {
        self.peer.admit_peer(peer, request)
    }

    fn resolve_operation<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, WriterGuard>,
        _: &mut PolicyEvaluator,
        _: &[ResidentProgramDeploymentAuthority],
        _: &Self::PeerWitness,
        authenticated_key: Option<[u8; 32]>,
        frame: &ResidentOperationFrameRequest,
    ) -> Result<ResidentProgramOperationResolution, ResidentOperationFrameFailure> {
        let _ = authenticated_key;
        if self.request.body().schema_id.as_str()
            == pale_core::records::NATIVE_TOOL_REQUEST_V1_SCHEMA_ID
        {
            return Err(ResidentOperationFrameFailure::AdmissionDenied);
        }
        ResidentProgramOperationResolution::admit(
            frame,
            self.request.clone(),
            operation_request_host::ResidentOperationRequestKind::StartInvoke,
            None,
        )
    }

    fn answer_operation<'store>(
        &mut self,
        process: &mut ResidentProcessHost<'store, WriterGuard>,
        policy: &mut PolicyEvaluator,
        resolution: &ResidentProgramOperationResolution,
    ) -> Result<
        operation_request_host::ResidentOperationRequestOutcome,
        ResidentOperationFrameFailure,
    > {
        if let Some(pending) = self.pending_start.take() {
            let result = process.continue_operation_start(
                pending,
                ResidentDeploymentOperationAccess {
                    store: &self.store,
                    custody: &mut *self.custody,
                    sessions: self.sessions,
                    anchor: &mut *self.anchor,
                },
                &mut self.authority,
            );
            return match result {
                Ok(issued) => operation_request_host::issued_start_records(&issued)
                    .map(operation_request_host::ResidentOperationRequestOutcome::Started)
                    .map_err(|_| ResidentOperationFrameFailure::ResidentUnavailable),
                Err(failure) => {
                    self.pending_start = Some(*failure.pending);
                    Err(ResidentOperationFrameFailure::ResidentUnavailable)
                }
            };
        }
        let root = self
            .root
            .take()
            .ok_or(ResidentOperationFrameFailure::ReplayConflict)?;
        let request = resolution.request();
        if root.request != request {
            return Err(ResidentOperationFrameFailure::InvalidRequest);
        }
        let restart = self
            .restart
            .take()
            .ok_or(ResidentOperationFrameFailure::ReplayConflict)?;
        let admission = self
            .admission
            .take()
            .ok_or(ResidentOperationFrameFailure::ReplayConflict)?;
        let cryptography = &self.cryptography;
        let outcome = match process.invoke_request(
            &self.selector,
            ResidentDeploymentOperationAccess {
                store: &self.store,
                custody: &mut *self.custody,
                sessions: self.sessions,
                anchor: &mut *self.anchor,
            },
            root,
            &self.random,
            restart,
            admission,
            &mut self.authority,
            policy,
            |request, session, current, handle, lease, operation, publication, checkpoint| {
                cryptography.admit_compiled_authority(
                    request,
                    session,
                    current,
                    handle,
                    lease,
                    operation,
                    publication,
                    checkpoint,
                )
            },
            |bytes, preimage, digest| cryptography.sign_handle(bytes, preimage, digest),
            |bytes, preimage, digest| cryptography.sign_observation(bytes, preimage, digest),
            |prepared, checkpoint_id| cryptography.prepare_checkpoint(prepared, checkpoint_id),
            |bytes, preimage, digest| cryptography.sign_checkpoint(bytes, preimage, digest),
        ) {
            Ok(outcome) => outcome,
            Err(ResidentProcessRootError::Start(
                ResidentProcessAllocatedInvokeStartError::Continuation(continuation),
            )) => {
                self.pending_start = Some(*continuation);
                return Err(ResidentOperationFrameFailure::ResidentUnavailable);
            }
            Err(error) => {
                return Err(match error {
                    ResidentProcessRootError::Capacity => {
                        ResidentOperationFrameFailure::ResidentUnavailable
                    }
                    ResidentProcessRootError::Policy(_)
                    | ResidentProcessRootError::Resolution(_) => {
                        ResidentOperationFrameFailure::ConfigurationUnavailable
                    }
                    ResidentProcessRootError::Lease(_) | ResidentProcessRootError::Start(_) => {
                        ResidentOperationFrameFailure::ResidentUnavailable
                    }
                });
            }
        };
        match outcome {
            ResidentProcessRootOutcome::Started(issued) => {
                let records = operation_request_host::issued_start_records(&issued)
                    .map_err(|_| ResidentOperationFrameFailure::ResidentUnavailable)?;
                Ok(operation_request_host::ResidentOperationRequestOutcome::Started(records))
            }
            ResidentProcessRootOutcome::ExistingOperation(disposition) => {
                operation_request_host::issued_start_records(
                    operation_locator::restart_disposition_operation(&disposition),
                )
                .map(operation_request_host::ResidentOperationRequestOutcome::Started)
                .map_err(|_| ResidentOperationFrameFailure::ResidentUnavailable)
            }
        }
    }

    fn recover_root_provider<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, WriterGuard>,
        _: &ResidentDeploymentAuthoritySelector,
    ) -> Result<(), ResidentProgramTurnError> {
        Err(ResidentProgramTurnError::RecoveryUnavailable)
    }

    fn recover_root_consult<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, WriterGuard>,
        _: &mut PolicyEvaluator,
        _: &ResidentDeploymentAuthoritySelector,
    ) -> Result<(), ResidentProgramTurnError> {
        Err(ResidentProgramTurnError::RecoveryUnavailable)
    }

    fn recover_consult_child<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, WriterGuard>,
        _: &mut PolicyEvaluator,
        _: &ResidentDeploymentAuthoritySelector,
    ) -> Result<(), ResidentProgramTurnError> {
        Err(ResidentProgramTurnError::RecoveryUnavailable)
    }

    fn recover_delegate<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, WriterGuard>,
        _: &ResidentDeploymentAuthoritySelector,
    ) -> Result<(), ResidentProgramTurnError> {
        Err(ResidentProgramTurnError::RecoveryUnavailable)
    }
}

/// Application-owned Observe inputs retained beside the process host.
///
/// This is separate from operation-turn ownership because Observe has its own
/// public frame, replay admission, disclosure authorization, and read-only
/// process-host journey. Implementations supply physical and authority owners;
/// the program preserves peer-first admission and the existing Observe owner.
pub(in crate::resident) trait ResidentProgramObserveTurnOwner<WriterGuard> {
    /// Peer-admission witness retained until Observe resolution completes.
    type PeerWitness;

    /// Admit supplementary local peer evidence without reading protected state.
    fn admit_observe_peer(
        &mut self,
        peer: LocalIpcPeerCredentials,
        request: &ResidentObserveFrameRequest,
    ) -> Result<Self::PeerWitness, InvalidResidentRecordV1>;

    /// Admit and answer one Observe request through the existing process owner.
    fn answer_observe<'store>(
        &mut self,
        process: &mut ResidentProcessHost<'store, WriterGuard>,
        deployment_authorities: &[ResidentProgramDeploymentAuthority],
        peer: &Self::PeerWitness,
        request: &ResidentObserveFrameRequest,
    ) -> Result<(OperationControlObservation, Instant), ResidentObserveFrameFailure>;
}

impl<'store, WriterGuard, PolicyEvaluator, TurnOwner, StopInput>
    ResidentProgram<'store, WriterGuard, PolicyEvaluator, TurnOwner, StopInput>
where
    WriterGuard: PrincipalWriterGuardPort,
    PolicyEvaluator: consult_recovery::ResidentConsultationPolicyEvaluator<StopInput>
        + ResidentProcessInvokePolicyEvaluator,
    TurnOwner: ResidentProgramTurnOwner<WriterGuard, PolicyEvaluator, StopInput>,
{
    /// Recover every selected deployment and construct the sole process host.
    ///
    /// The admitted trust frontier is the only source of deployment selectors
    /// and associations. Every caller-opened store must match that complete set.
    /// A degraded cold-open stops assembly; it never enters a live program.
    ///
    /// # Errors
    ///
    /// Returns the first writer admission, physical association, cold-open,
    /// degraded recovery, deployment join, or process-host construction failure.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn recover(
        held_writer: WriterGuard,
        writer_snapshot: &PrincipalResidentWriterSnapshot,
        writer_admission: &mut dyn CurrentPrincipalWriterAdmissionPort,
        trust: &RecoveredCurrentPrincipalTrustFrontier,
        deployment_opens: Vec<ResidentProgramDeploymentOpen<'store, '_>>,
        maximum_ready_deployments: NonZeroUsize,
        operation_tasks: super::super::operation_service::ResidentOperationTaskState,
        consult_policy: PolicyEvaluator,
        turn_owner: TurnOwner,
    ) -> Result<Self, ResidentProgramRecoveryError> {
        let mut associations = Vec::new();
        associations
            .try_reserve_exact(trust.runtime_authorities().len())
            .map_err(|_| ResidentProgramRecoveryError::DeploymentAllocation)?;
        associations.extend(
            trust
                .runtime_authorities()
                .iter()
                .map(AdmittedResidentDeploymentAssociation::from_recovered),
        );
        associations.sort_by(|left, right| left.store.cmp(&right.store));

        let current_trust = trust.current_checkpoint();
        let mut deployment_authorities = trust
            .runtime_authorities()
            .iter()
            .map(|runtime| {
                ResidentProgramDeploymentAuthority::from_recovered(runtime, current_trust)
            })
            .collect::<Vec<_>>();
        deployment_authorities.sort_by(|left, right| {
            left.selector
                .deployment_binding_ref
                .cmp(&right.selector.deployment_binding_ref)
        });

        let opened_stores = associations
            .iter()
            .map(|association| association.store.clone())
            .collect::<Vec<_>>();
        let admitted_writer = writer_admission
            .admit_current_writer(writer_snapshot)
            .map_err(ResidentProgramRecoveryError::WriterAdmission)?;
        let writer = ReadyPrincipalResidentWriter::recover(
            held_writer,
            writer_snapshot,
            admitted_writer,
            opened_stores,
        )?;

        let mut selected_open_stores = deployment_opens
            .iter()
            .map(|open| open.input.store().clone())
            .collect::<Vec<_>>();
        selected_open_stores.sort();
        if deployment_opens.len() != associations.len()
            || selected_open_stores
                .iter()
                .zip(associations.iter())
                .any(|(opened, admitted)| opened != &admitted.store)
        {
            return Err(ResidentProgramRecoveryError::OpenedStoreSet);
        }
        let mut deployments = Vec::new();
        deployments
            .try_reserve_exact(associations.len())
            .map_err(|_| ResidentProgramRecoveryError::DeploymentAllocation)?;
        for open in deployment_opens {
            let ResidentProgramDeploymentOpen {
                durable,
                sessions,
                events,
                maximum_event_custody_bytes,
                input,
            } = open;
            let application = ResidentApplication::cold_open(
                durable,
                sessions,
                events,
                maximum_event_custody_bytes,
                input,
            )?;
            let ResidentColdOpenOutcome::Ready(application) = application else {
                return Err(ResidentProgramRecoveryError::DegradedDeployment);
            };
            let index = associations
                .binary_search_by(|association| association.store.cmp(application.store()))
                .map_err(|_| ResidentProgramRecoveryError::OpenedStoreSet)?;
            let association = associations.remove(index);
            deployments.push(ReadyResidentDeployment::new(association, application)?);
        }
        if !associations.is_empty() {
            return Err(ResidentProgramRecoveryError::OpenedStoreSet);
        }

        let operations = ResidentOperationHost::new(operation_tasks);
        let process =
            ResidentProcessHost::new(writer, deployments, maximum_ready_deployments, operations)?;
        Ok(Self {
            process,
            deployment_authorities: deployment_authorities.into_boxed_slice(),
            consult_policy,
            turn_owner,
            recovery_deployment: 0,
            recovery_source: ResidentProgramRecoverySource::RootProvider,
            stop_input: PhantomData,
        })
    }

    /// Serve one operation frame through peer-first application admission.
    ///
    /// Exactly one frame is received and at most one response is sent. The
    /// retained turn owner resolves protected state only after peer admission;
    /// `Pending`, `Unsupported`, and application failure are never retried.
    pub(in crate::resident) fn serve_operation_frame(
        &mut self,
        carrier: &mut dyn LocalIpcFramePort,
        carrier_deadline: Instant,
        response_deadline: Instant,
    ) -> Result<ResidentOperationFrameResponse, ResidentProgramTurnError> {
        let Self {
            process,
            deployment_authorities,
            consult_policy,
            turn_owner,
            ..
        } = self;
        let mut host =
            operation_request_host::ResidentOperationRequestHost::new(carrier, carrier_deadline);
        let received = host.receive()?;
        let admitted = host.admit_peer(received, &mut |peer, request| {
            turn_owner.admit_operation_peer(peer, request)
        })?;
        let resolution = match turn_owner.resolve_operation(
            process,
            consult_policy,
            deployment_authorities,
            admitted.peer(),
            admitted.caller_key(),
            admitted.request(),
        ) {
            Ok(resolution) => resolution,
            Err(failure) => {
                return host
                    .reject_admitted(admitted, response_deadline, failure)
                    .map_err(Into::into);
            }
        };
        let request_resolution = resolution.request_resolution();
        host.answer_admitted(
            admitted,
            process,
            request_resolution,
            response_deadline,
            |process, request| {
                debug_assert!(std::ptr::eq(request, &resolution.request));
                turn_owner.answer_operation(process, consult_policy, &resolution)
            },
        )
        .map_err(Into::into)
    }

    /// Whether the turn owner retains one runnable operation phase.
    pub(in crate::resident) fn has_retained_phase(&self) -> bool {
        self.turn_owner.has_retained_phase()
    }

    /// Advance at most one exact retained operation phase without another frame.
    pub(in crate::resident) fn continue_one_retained_phase(
        &mut self,
    ) -> ResidentProgramRetainedPhaseTurn {
        self.turn_owner.continue_retained_phase(
            &mut self.process,
            &mut self.consult_policy,
            &self.deployment_authorities,
        )
    }

    /// Advance at most one retained recovery source.
    ///
    /// Every bounded attempt advances the scheduling cursor. A failed owner
    /// keeps its own discovery cursor or linear handoff, but cannot starve the
    /// other recovery classes or deployments. The scheduler returns the exact
    /// attempt result so the caller still observes the failure.
    pub(in crate::resident) fn recover_one_bounded_source(
        &mut self,
    ) -> Result<(), ResidentProgramTurnError> {
        let deployment_count = self.process.deployments.len();
        if deployment_count == 0 {
            return Err(ResidentProgramTurnError::RecoveryUnavailable);
        }
        let deployment = self.recovery_deployment % deployment_count;
        let selector = ResidentDeploymentAuthoritySelector::from_association(
            &self.process.deployments[deployment].association,
        );
        let source = self.recovery_source;
        let result =
            match source {
                ResidentProgramRecoverySource::RootProvider => self
                    .turn_owner
                    .recover_root_provider(&mut self.process, &selector),
                ResidentProgramRecoverySource::RootConsult => self.turn_owner.recover_root_consult(
                    &mut self.process,
                    &mut self.consult_policy,
                    &selector,
                ),
                ResidentProgramRecoverySource::ConsultChild => self
                    .turn_owner
                    .recover_consult_child(&mut self.process, &mut self.consult_policy, &selector),
                ResidentProgramRecoverySource::Delegate => self
                    .turn_owner
                    .recover_delegate(&mut self.process, &selector),
                ResidentProgramRecoverySource::Publication => self
                    .turn_owner
                    .recover_publication(&mut self.process, &selector),
            };
        self.recovery_source = source.next();
        if self.recovery_source == ResidentProgramRecoverySource::RootProvider {
            self.recovery_deployment = (deployment + 1) % deployment_count;
        }
        result
    }

    /// Borrow the retained Consult policy for the existing recovery owner.
    fn consult_policy(&mut self) -> &mut PolicyEvaluator {
        &mut self.consult_policy
    }

    /// Borrow the single process host for one program-owned bounded turn.
    fn process(&mut self) -> &mut ResidentProcessHost<'store, WriterGuard> {
        &mut self.process
    }
}

impl<'store, WriterGuard, PolicyEvaluator, TurnOwner, StopInput>
    ResidentProgram<'store, WriterGuard, PolicyEvaluator, TurnOwner, StopInput>
where
    WriterGuard: PrincipalWriterGuardPort,
    TurnOwner: ResidentProgramObserveTurnOwner<WriterGuard>,
{
    /// Serve one Observe frame through its separate peer-first ingress.
    ///
    /// Exactly one frame is received and at most one response is sent. Observe
    /// remains read-only and does not pass through the operation-frame router.
    pub(in crate::resident) fn serve_observe_frame(
        &mut self,
        carrier: &mut dyn LocalIpcFramePort,
        carrier_deadline: Instant,
    ) -> Result<Option<OperationControlObservation>, ResidentProgramTurnError> {
        let Self {
            process,
            deployment_authorities,
            turn_owner,
            ..
        } = self;
        let mut host =
            observe_request_host::ResidentObserveRequestHost::new(carrier, carrier_deadline);
        let received = host.receive()?;
        let admitted = host.admit_peer(received, &mut |peer, request| {
            turn_owner.admit_observe_peer(peer, request)
        })?;
        host.answer_admitted_with(&admitted, carrier_deadline, |request| {
            turn_owner.answer_observe(process, deployment_authorities, admitted.peer(), request)
        })
        .map_err(Into::into)
    }
}

/// Failure of one bounded program turn.
#[derive(Debug, thiserror::Error)]
pub(in crate::resident) enum ResidentProgramTurnError {
    /// Operation framing, peer admission, association, or carrier work failed.
    #[error(transparent)]
    OperationFrame(#[from] operation_request_host::ResidentOperationRequestHostError),
    /// Observe framing, peer admission, association, or carrier work failed.
    #[error(transparent)]
    ObserveFrame(#[from] observe_request_host::ResidentObserveRequestHostError),
    /// The selected recovery owner did not reach a safe durable handoff.
    #[error("resident recovery owner did not reach a safe handoff")]
    RecoveryUnavailable,
}

#[cfg(test)]
#[path = "../../../tests/support/resident_program.rs"]
mod program_test_support;

/// Why process-lifetime resident assembly stopped.
#[derive(Debug, thiserror::Error)]
pub(in crate::resident) enum ResidentProgramRecoveryError {
    /// The exact physical writer records did not pass record-specific admission.
    #[error("current principal writer admission failed")]
    WriterAdmission(#[source] InvalidResidentRecordV1),
    /// The held guard and admitted physical writer head did not form one owner.
    #[error(transparent)]
    WriterJoin(#[from] ResidentPrincipalWriterJoinError),
    /// The admitted deployment set could not be retained within memory bounds.
    #[error("resident deployment set could not be allocated")]
    DeploymentAllocation,
    /// Caller-opened stores did not equal the admitted runtime-authority set.
    #[error("opened deployment stores do not equal admitted runtime authorities")]
    OpenedStoreSet,
    /// One deployment recovered only a quarantined read-only prefix.
    #[error("a selected deployment cold-opened in degraded read-only state")]
    DegradedDeployment,
    /// One deployment application did not join its derived runtime authority.
    #[error(transparent)]
    DeploymentJoin(#[from] ResidentDeploymentJoinError),
    /// The complete process ownership graph failed its structural laws.
    #[error(transparent)]
    ProcessHost(#[from] ResidentProcessHostBuildError),
    /// One deployment could not complete bounded cold recovery.
    #[error(transparent)]
    ColdOpen(#[from] ResidentApplicationError),
}
