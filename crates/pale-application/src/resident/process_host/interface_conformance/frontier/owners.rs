//! Minimal physical owners for the in-memory interface conformance frontier.
//!
//! These owners use the ordinary request, session, start, custody, recovery,
//! and observation APIs. Their signatures, clock, entropy, and authority
//! checks are deterministic development models. They do not represent a
//! production key store, clock, random source, or disclosure service.

mod consult;
pub(in super::super) use consult::*;

use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    num::NonZeroUsize,
    sync::{atomic::Ordering, Arc},
    time::Instant,
};

use pale_core::records::*;
use pale_ports::*;

use crate::resident::*;

use super::super::{conformance_ingress_time, ConsultConformanceCounters};

use super::{
    anchor::ConformanceAntiRollbackAnchors,
    application::ConformanceResidentState,
    custody::ConformanceOperationCustody,
    lifecycle::{ConformanceGenerationSource, ConformanceHistorical},
    principal::{record_ref, signature},
    request::{admit_historical_request, ConformanceRequestPhase},
    session::{ConformanceSessionFixture, ProvingAuthority},
    storage::UnavailableEventCustody,
    writer::{ConformanceWriterAdmission, ConformanceWriterGuard},
    OperationConformanceFrontierError,
};

const MAXIMUM_RECORD_BYTES: usize = 1 << 20;

fn accumulator_limits() -> AccumulatorPreparationLimits {
    // Four leaves can occupy more than 512 sparse-tree positions. These
    // ceilings cover the native adapter as well as the in-memory model.
    AccumulatorPreparationLimits::new(4, 10_000, 10_000, 4096, 100_000)
        .expect("bounded conformance accumulator")
}

struct ConformancePeerAdmission(LocalIpcPeerCredentials);

impl ResidentPeerAdmissionPort for ConformancePeerAdmission {
    fn admit_peer(
        &mut self,
        credentials: LocalIpcPeerCredentials,
    ) -> Result<ResidentPeerAdmissionWitness, InvalidResidentRecordV1> {
        if credentials != self.0 {
            return Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "resident local peer",
                invariant: "kernel peer coordinates do not match the conformance peer",
            });
        }
        Ok(ResidentPeerAdmissionWitness::new(credentials))
    }
}

fn schema(id: &str) -> SchemaDigest {
    pale_contracts::record_schema_identity(id)
        .expect("registered conformance schema")
        .schema_sha256
}

pub(in super::super) fn supported_profile() -> SupportedOperationCheckpointProfile {
    let bytes = SupportedOperationCheckpointProfile::canonical_content()
        .expect("checkpoint profile canonicalizes");
    let digest =
        SupportedOperationCheckpointProfile::content_digest().expect("checkpoint profile digest");
    SupportedOperationCheckpointProfile::admit(
        record_ref("operation-checkpoint-profile"),
        &digest,
        bytes.as_bytes(),
    )
    .expect("source-owned checkpoint profile admits")
}

pub(in super::super) struct ConformanceCheckpointAuthority {
    authority_ref: ResidentRecordRefV1,
    authority_sha256: CheckpointSigningAuthorityDigest,
    profile_ref: ResidentRecordRefV1,
    profile_sha256: CheckpointProfileDigest,
}

impl ConformanceCheckpointAuthority {
    pub(in super::super) fn new(session: &ConformanceSessionFixture) -> Self {
        Self {
            authority_ref: session.runtime_ref.clone(),
            authority_sha256: CheckpointSigningAuthorityDigest::parse(
                session
                    .runtime
                    .canonical_deployment_runtime_authority_sha256()
                    .as_str(),
            )
            .expect("checkpoint authority digest"),
            profile_ref: record_ref("operation-checkpoint-profile"),
            profile_sha256: SupportedOperationCheckpointProfile::content_digest()
                .expect("checkpoint profile digest"),
        }
    }

    pub(in super::super) fn context<'a>(
        &'a self,
        store: &'a AuthorityStoreCoordinate,
        domain: &'a ControlCheckpointDomainIdV1,
        session: &'a ConformanceSessionFixture,
    ) -> ControlCheckpointSigningAuthorityAdmissionContextV1<'a> {
        checkpoint_authority(self, store, domain, session)
    }
}

fn checkpoint_authority<'a>(
    coordinates: &'a ConformanceCheckpointAuthority,
    store: &'a AuthorityStoreCoordinate,
    domain: &'a ControlCheckpointDomainIdV1,
    session: &'a ConformanceSessionFixture,
) -> ControlCheckpointSigningAuthorityAdmissionContextV1<'a> {
    ControlCheckpointSigningAuthorityAdmissionContextV1 {
        authority_ref: &coordinates.authority_ref,
        authority_sha256: &coordinates.authority_sha256,
        principal_id: &session.principal,
        control_store_id: store.store_id(),
        control_incarnation_id: store.incarnation_id(),
        checkpoint_domain_id: domain,
        checkpoint_profile_ref: &coordinates.profile_ref,
        checkpoint_profile_sha256: &coordinates.profile_sha256,
        subject_key_id: session.runtime.subject_key_id(),
        subject_key_epoch: *session.runtime.subject_key_epoch(),
    }
}

fn checkpoint_template(
    store: &AuthorityStoreCoordinate,
    domain: ControlCheckpointDomainIdV1,
    session: &ConformanceSessionFixture,
) -> ControlStateCheckpointPreimageV1 {
    ControlStateCheckpointPreimageV1 {
        schema_id: SchemaId::parse(CONTROL_STATE_CHECKPOINT_V1_SCHEMA).expect("checkpoint schema"),
        schema_sha256: schema(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID),
        control_checkpoint_id: ControlCheckpointIdV1::parse("operation-checkpoint-pending")
            .expect("checkpoint identity"),
        control_store_id: store.store_id().clone(),
        control_incarnation_id: store.incarnation_id().clone(),
        checkpoint_domain_id: domain,
        domain_epoch: ControlDomainEpochV1::new(1).expect("domain epoch"),
        domain_sequence: ControlDomainSequenceV1::new(1).expect("domain sequence"),
        previous_control_checkpoint_ref: None,
        previous_control_checkpoint_sha256: None,
        checkpoint_profile_ref: record_ref("operation-checkpoint-profile"),
        checkpoint_profile_sha256: SupportedOperationCheckpointProfile::content_digest()
            .expect("checkpoint profile digest"),
        control_schema_version: 1,
        committed_transaction_sequence: ControlTransactionSequenceV1::new(1)
            .expect("transaction sequence"),
        state_root_sha256: ArtifactDigest::hash_bytes(b"pending operation state"),
        leaf_count: ControlLeafCountV1::new(4).expect("operation leaves"),
        changed_leaf_keys: Vec::new(),
        unresolved_custody_root_sha256: ArtifactDigest::hash_bytes(
            b"pending unresolved operation custody",
        ),
        issued_at: UnixTimeV1::new(61).expect("checkpoint issue time"),
        checkpoint_signing_authority_ref: session.runtime_ref.clone(),
        checkpoint_signing_authority_sha256: CheckpointSigningAuthorityDigest::parse(
            session
                .runtime
                .canonical_deployment_runtime_authority_sha256()
                .as_str(),
        )
        .expect("checkpoint authority digest"),
        issuer_principal_id: session.principal.clone(),
        issuer_key_id: session.runtime.subject_key_id().clone(),
        issuer_key_epoch: *session.runtime.subject_key_epoch(),
    }
}

pub(in super::super) fn recovery_limits() -> ResidentServiceInvokeRecoveryLimits {
    ResidentServiceInvokeRecoveryLimits {
        observation: OperationObservationRecoveryLimits::new(
            // This journey retains the Start observation and one Advance successor.
            2,
            MAXIMUM_RECORD_BYTES,
            MAXIMUM_RECORD_BYTES,
        )
        .expect("bounded observation recovery"),
        checkpoints: OperationCheckpointAncestryLimits::new(1, MAXIMUM_RECORD_BYTES, 1)
            .expect("bounded checkpoint recovery"),
        membership: AccumulatorMembershipLimits::new(3, 1024, MAXIMUM_RECORD_BYTES)
            .expect("bounded membership verification"),
        maximum_selection_bytes: MAXIMUM_RECORD_BYTES,
    }
}

pub(in super::super) fn consult_recovery_limits() -> ResidentServiceInvokeRecoveryLimits {
    let mut limits = recovery_limits();
    // Collection custody adds one checkpoint after the parent Start.
    limits.checkpoints = OperationCheckpointAncestryLimits::new(2, MAXIMUM_RECORD_BYTES, 2)
        .expect("bounded Consult checkpoint recovery");
    limits
}

fn recovery_input<'a>(
    store: &'a AuthorityStoreCoordinate,
    profile: &'a SupportedOperationCheckpointProfile,
    signing: &'a SigningAuthorityBindingV1,
    checkpoint: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    historical: &'a ConformanceHistorical,
    expected_handle: &'a ControlRecordCoordinate,
    limits: ResidentServiceInvokeRecoveryLimits,
) -> OperationControlRecoveryInput<'a> {
    OperationControlRecoveryInput {
        operation: LocalOperationHandleRecoveryInput {
            context: LocalOperationRecoveryContext {
                store,
                profile,
                handle_signing_authority: signing,
                checkpoint_authority: checkpoint,
                historical,
                maximum_record_bytes: MAXIMUM_RECORD_BYTES,
                maximum_operation_lease_bytes: MAXIMUM_RECORD_BYTES,
                maximum_session_lease_bytes: MAXIMUM_RECORD_BYTES,
                membership_limits: limits.membership,
                observation_limits: limits.observation,
                checkpoint_limits: limits.checkpoints,
            },
            expected_handle,
            maximum_parent_depth: 0,
        },
        observation: OperationObservationSelection::SelectedHead,
        observation_limits: limits.observation,
        checkpoint_limits: limits.checkpoints,
        membership_limits: AccumulatorMembershipLimits::new(1, 1024, MAXIMUM_RECORD_BYTES)
            .expect("bounded selected-observation membership"),
    }
}

fn invalid(record: &'static str, invariant: &'static str) -> InvalidResidentRecordV1 {
    InvalidResidentRecordV1::ContextualAdmission { record, invariant }
}

macro_rules! with_recovery_admissions {
    ($session:expr, $checkpoint_schema:expr, $checkpoint_authority:expr, |$admissions:ident| $body:block $(,)?) => {{
        let session = $session;
        let checkpoint_schema = $checkpoint_schema;
        let checkpoint_authority = $checkpoint_authority;
        let mut admit_request = admit_historical_request;
        let mut admit_handle_authority =
            |preimage: &OperationHandlePreimageV1, parent: Option<&OperationHandleV1>| {
                (parent.is_none()
                    && preimage.issuer_principal_id == session.principal
                    && preimage.deployment_runtime_authority_ref == session.runtime_ref
                    && preimage.issuer_key_id == *session.runtime.subject_key_id()
                    && preimage.issuer_key_epoch == *session.runtime.subject_key_epoch())
                .then_some(())
                .ok_or_else(|| invalid("operation handle", "deterministic issuance authority"))
            };
        let mut verify_handle_signature =
            |_: &[u8],
             _: &OperationHandlePreimageV1,
             _: &OperationHandleDigest,
             candidate: &CanonicalSignatureV1| {
                (candidate == &signature())
                    .then_some(())
                    .ok_or_else(|| invalid("operation handle", "deterministic signature"))
            };
        let mut admit_checkpoint = |_: &ResidentRecordRefV1, wire: ControlStateCheckpointWireV1| {
            ControlStateCheckpointV1::admit_signed_wire(
                wire,
                checkpoint_schema,
                checkpoint_authority,
                |_, _, _, candidate| {
                    (candidate == &signature())
                        .then_some(())
                        .ok_or_else(|| invalid("control checkpoint", "deterministic signature"))
                },
            )
        };
        let mut resolve_observation_signer =
            |_: &ResidentRecordRefV1, _: &OperationObservationWireV1| Ok(session.signing.clone());
        let mut admit_observation_state = |_: &OperationObservationStateV1| Ok(());
        let mut admit_rotation = |old_ref: &ResidentRecordRefV1,
                                  old_sha: &ArtifactDigest,
                                  new_ref: &ResidentRecordRefV1,
                                  new_sha: &ArtifactDigest| {
            ((old_ref == new_ref && old_sha == new_sha)
                || (old_ref == &session.runtime_ref
                    && new_ref == &session.runtime_ref
                    && old_sha.as_str()
                        == session
                            .runtime
                            .canonical_deployment_runtime_authority_sha256()
                            .as_str()
                    && old_sha == new_sha))
                .then_some(())
                .ok_or_else(|| invalid("operation observation", "unadmitted authority rotation"))
        };
        let mut verify_observation_signature =
            |_: &[u8],
             _: &OperationObservationPreimageV1,
             _: &OperationObservationDigest,
             candidate: &CanonicalSignatureV1| {
                (candidate == &signature())
                    .then_some(())
                    .ok_or_else(|| invalid("operation observation", "deterministic signature"))
            };
        let mut verify_checkpoint_signature =
            |_: &[u8],
             _: &ControlStateCheckpointPreimageV1,
             _: &ControlStateCheckpointDigest,
             candidate: &CanonicalSignatureV1| {
                (candidate == &signature())
                    .then_some(())
                    .ok_or_else(|| invalid("control checkpoint", "deterministic signature"))
            };
        let mut authorize_disclosure = |_: OperationControlAuthorizationContext<'_>| Ok(());
        let $admissions = ConformanceRecoveryAdmissions {
            recovery: OperationControlRecoveryAdmission {
                admit_request: &mut admit_request,
                admit_handle_authority: &mut admit_handle_authority,
                verify_handle_signature: &mut verify_handle_signature,
                admit_checkpoint: &mut admit_checkpoint,
                resolve_observation_signer: &mut resolve_observation_signer,
                admit_observation_state: &mut admit_observation_state,
                admit_rotation: &mut admit_rotation,
                verify_observation_signature: &mut verify_observation_signature,
                verify_checkpoint_signature: &mut verify_checkpoint_signature,
            },
            disclosure: &mut authorize_disclosure,
        };
        $body
    }};
}

pub(in super::super) struct ConformanceEvidence(ResidentServiceCurrentRequestEvidence);

impl ConformanceEvidence {
    fn new(value: ResidentServiceCurrentRequestEvidence) -> Self {
        Self(value)
    }
}

impl ResidentServiceCurrentRequestEvidencePort for ConformanceEvidence {
    fn read_current_request_evidence(
        &mut self,
        _: ResidentServiceCurrentRequestSelector<'_>,
    ) -> Result<ResidentServiceCurrentRequestEvidence, InvalidResidentRecordV1> {
        Ok(self.0.clone())
    }
}

pub(in super::super) fn current_evidence(
    current: &AmbientSessionAdmissionContext<'_>,
) -> ResidentServiceCurrentRequestEvidence {
    let grant = current.grant.as_ref().expect("conformance grant");
    let capability = current
        .capability_revocations
        .as_ref()
        .expect("conformance capability frontier");
    ResidentServiceCurrentRequestEvidence {
        grant: ResidentServiceGrantEvidence {
            grant_ref: grant.grant_ref.clone(),
            grant: grant.grant.clone(),
            issued_trust_checkpoint_ref: grant.issued_trust_checkpoint_ref.clone(),
            issued_trust_checkpoint: grant.issued_trust_checkpoint.clone(),
        },
        capability_revocations: ResidentServiceCapabilityRevocationEvidence {
            checkpoint_ref: capability.checkpoint_ref.clone(),
            checkpoint: capability.checkpoint.clone(),
            grant_head: capability
                .grant_head
                .map(|head| ResidentServiceGrantRevocationEvidence {
                    revocation_ref: head.revocation_ref.clone(),
                    revocation: head.revocation.clone(),
                }),
        },
        deployment_revocations: ResidentServiceDeploymentRevocationEvidence {
            epoch: current.deployment_revocations.epoch,
            binding_head: current.deployment_revocations.binding_head.map(|head| {
                ResidentServiceDeploymentRevocationHeadEvidence {
                    revocation_ref: head.revocation_ref.clone(),
                    revocation: head.revocation.clone(),
                }
            }),
        },
        session_revocations: ResidentServiceSessionRevocationEvidence {
            epoch: current.session_revocations.epoch,
            revocation_ref: current.session_revocations.revocation_ref.cloned(),
            revocation_sha256: current.session_revocations.revocation_sha256.cloned(),
        },
    }
}

struct ConformanceClock;

impl WallClockSourcePort for ConformanceClock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        Ok(conformance_ingress_time())
    }
}

struct ConformanceConsultClock {
    counters: Arc<ConsultConformanceCounters>,
}

impl WallClockSourcePort for ConformanceConsultClock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        self.counters.clock_reads.fetch_add(1, Ordering::SeqCst);
        Ok(conformance_ingress_time())
    }
}

struct ConformanceCallerSignature;

impl ResidentServiceCallerSignaturePort for ConformanceCallerSignature {
    fn verify_caller_signature(
        &mut self,
        _: &CommonRequestEnvelopePreimageV1,
        _: &CommonRequestEnvelopeDigest,
        candidate: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        (candidate == &signature())
            .then_some(())
            .ok_or_else(|| invalid("common request", "deterministic caller signature"))
    }
}

pub(in super::super) struct ConformanceEntropy {
    next: std::sync::atomic::AtomicU8,
    counters: Option<Arc<ConsultConformanceCounters>>,
}

impl Default for ConformanceEntropy {
    fn default() -> Self {
        Self {
            next: std::sync::atomic::AtomicU8::new(0),
            counters: None,
        }
    }
}

impl ConformanceEntropy {
    fn for_consult(counters: Arc<ConsultConformanceCounters>) -> Self {
        Self {
            next: std::sync::atomic::AtomicU8::new(0),
            counters: Some(counters),
        }
    }
}

impl CryptographicRandomSourcePort for ConformanceEntropy {
    fn fill_bytes(&self, destination: &mut [u8]) -> Result<(), CryptographicRandomSourceError> {
        if let Some(counters) = &self.counters {
            counters
                .identity_entropy_fills
                .fetch_add(1, Ordering::SeqCst);
        }
        let byte = self
            .next
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            .checked_add(1)
            .expect("bounded conformance entropy");
        destination.fill(byte);
        Ok(())
    }
}

pub(in super::super) struct ConformanceInvokePolicy(Option<ResidentServiceOperationStartPolicy>);

impl ConformanceInvokePolicy {
    pub(in super::super) fn new(policy: ResidentServiceOperationStartPolicy) -> Self {
        Self(Some(policy))
    }

    pub(in super::super) const fn from(
        policy: Option<ResidentServiceOperationStartPolicy>,
    ) -> Self {
        Self(policy)
    }
}

impl ResidentServiceInvokePolicyPort for ConformanceInvokePolicy {
    fn evaluate_invoke(
        &mut self,
        _: ResidentServiceInvokePolicyContext<'_>,
    ) -> Result<ResidentServiceOperationStartPolicy, InvalidResidentRecordV1> {
        self.0.take().ok_or_else(|| {
            invalid(
                "resident Invoke policy",
                "the deterministic start policy was already consumed",
            )
        })
    }
}

pub(in super::super) fn invoke_policy(
    lease: &OperationLeaseV1,
    request: &CommonRequestEnvelopeV1,
) -> ResidentServiceOperationStartPolicy {
    ResidentServiceOperationStartPolicy {
        checkpoint_profile: supported_profile(),
        publication_policy: LocalOperationPublicationPolicy::from_admitted_input(
            LocalOperationPublicationPolicyInput {
                target_scope: &EventScopeV1 {
                    principal_scope: request.scope().principal_scope.clone(),
                    controller_scope: request.scope().controller_scope.clone(),
                    relationship_scope: request.scope().relationship_scope.clone(),
                    environment: DeploymentEnvironmentV1::parse(
                        request.scope().environment.as_str(),
                    )
                    .expect("publication environment"),
                },
                classification: &DataClassificationV1::parse(
                    request.scope().data_classification.as_str(),
                )
                .expect("publication classification"),
                visibility: &InteractionVisibilityV1::parse("private")
                    .expect("publication visibility"),
                disclosure_ceiling: &InteractionDisclosureCeilingV1::parse("private")
                    .expect("publication ceiling"),
                payload_resolution_policy: &PayloadResolutionPolicyV1::parse("reference-only")
                    .expect("payload policy"),
                actor_provenance: &OwnerRecordCoordinateV1 {
                    owner_system_id: OwnerSystemIdV1::parse("pale-interface-conformance")
                        .expect("owner system"),
                    record_ref: record_ref("interface-conformance-actor"),
                    record_schema_id: SchemaId::parse("interface-conformance/actor.v1")
                        .expect("actor schema"),
                    record_schema_sha256: schema(COMMON_REQUEST_ENVELOPE_V1_SCHEMA_ID),
                    record_content_sha256: ArtifactDigest::hash_bytes(
                        request.caller().identity.as_str().as_bytes(),
                    ),
                    native_revision: NativeRevisionV1 {
                        native_revision_owner: NativeRevisionOwnerV1::parse(
                            "pale-interface-conformance",
                        )
                        .expect("revision owner"),
                        native_revision_token: NativeRevisionTokenV1::parse("revision-1")
                            .expect("revision token"),
                    },
                },
            },
        ),
        maximum_record_bytes: MAXIMUM_RECORD_BYTES,
        lease: ResidentServiceOperationLeasePolicy {
            capabilities: lease.capabilities().to_vec(),
            effects: lease.effects().to_vec(),
            read_set: lease.read_set().to_vec(),
            evidence_obligations: lease.evidence_obligations().to_vec(),
            budget_units: *lease.budget_units(),
            timeout_millis: *lease.timeout_millis(),
            expires_at: lease.expires_at(),
            invocation_correlation: lease.invocation_correlation().clone(),
        },
        handle_expires_at: UnixTimeV1::new(80).expect("handle expiry"),
        handle_retain_until: UnixTimeV1::new(120).expect("handle retention"),
        accumulator_limits: accumulator_limits(),
    }
}

pub(in super::super) struct ConformanceCryptography {
    checkpoint: RefCell<Option<ControlStateCheckpointPreimageV1>>,
    consult_counters: Option<Arc<ConsultConformanceCounters>>,
}

impl ConformanceCryptography {
    pub(in super::super) fn new(
        store: &AuthorityStoreCoordinate,
        request: &CommonRequestEnvelopeV1,
        session: &ConformanceSessionFixture,
    ) -> Self {
        let domain = SupportedOperationCheckpointProfile::conflict_domain(request)
            .expect("request conflict domain");
        Self {
            checkpoint: RefCell::new(Some(checkpoint_template(store, domain, session))),
            consult_counters: None,
        }
    }

    pub(in super::super) fn new_for_consult(
        store: &AuthorityStoreCoordinate,
        request: &CommonRequestEnvelopeV1,
        session: &ConformanceSessionFixture,
        counters: Arc<ConsultConformanceCounters>,
    ) -> Self {
        let mut cryptography = Self::new(store, request, session);
        cryptography.consult_counters = Some(counters);
        cryptography
    }
}

impl ResidentServiceInvokeCryptographyPort for ConformanceCryptography {
    fn admit_compiled_authority(
        &self,
        _: &CommonRequestEnvelopeV1,
        _: &AdmittedAmbientSession,
        _: &AmbientSessionAdmissionContext<'_>,
        _: &OperationHandlePreimageV1,
        _: &OperationLeaseV1,
        _: &crate::semantic::ResolvedOperation,
        _: &LocalOperationPublicationPolicyInput<'_>,
        _: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    ) -> Result<(), InvalidResidentRecordV1> {
        Ok(())
    }

    fn sign_handle(
        &self,
        _: &[u8],
        _: &OperationHandlePreimageV1,
        _: &OperationHandleDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1> {
        if let Some(counters) = &self.consult_counters {
            counters.handle_signatures.fetch_add(1, Ordering::SeqCst);
        }
        Ok(signature())
    }

    fn sign_observation(
        &self,
        _: &[u8],
        _: &OperationObservationPreimageV1,
        _: &OperationObservationDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1> {
        Ok(signature())
    }

    fn prepare_checkpoint(
        &self,
        prepared: &PreparedLocalOperationCheckpoint,
        checkpoint_id: &ControlCheckpointIdV1,
    ) -> Result<ControlStateCheckpointPreimageV1, LocalOperationStartError> {
        let mut checkpoint = self
            .checkpoint
            .borrow_mut()
            .take()
            .ok_or(LocalOperationStartError::Coordinate)?;
        checkpoint.control_checkpoint_id = checkpoint_id.clone();
        checkpoint.committed_transaction_sequence = prepared.transaction_sequence();
        checkpoint.state_root_sha256 = prepared.state_root().clone();
        checkpoint.unresolved_custody_root_sha256 = prepared.unresolved_root().clone();
        checkpoint.changed_leaf_keys = prepared.changed_keys().to_vec();
        checkpoint.leaf_count = prepared.leaf_count();
        Ok(checkpoint)
    }

    fn sign_checkpoint(
        &self,
        _: &[u8],
        _: &ControlStateCheckpointPreimageV1,
        _: &ControlStateCheckpointDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1> {
        Ok(signature())
    }
}

pub(in super::super) struct ConformanceInvokeInputs<'a> {
    pub(in super::super) store: &'a AuthorityStoreCoordinate,
    pub(in super::super) historical: &'a ConformanceHistorical,
    pub(in super::super) session: &'a ConformanceSessionFixture,
    profile: SupportedOperationCheckpointProfile,
    domain: ControlCheckpointDomainIdV1,
    checkpoint: ConformanceCheckpointAuthority,
    limits: ResidentServiceInvokeRecoveryLimits,
    background_at: Option<UnixTimeV1>,
}

impl<'a> ConformanceInvokeInputs<'a> {
    pub(in super::super) fn new(
        store: &'a AuthorityStoreCoordinate,
        historical: &'a ConformanceHistorical,
        session: &'a ConformanceSessionFixture,
        request: &CommonRequestEnvelopeV1,
    ) -> Self {
        Self {
            store,
            historical,
            session,
            profile: supported_profile(),
            domain: SupportedOperationCheckpointProfile::conflict_domain(request)
                .expect("request conflict domain"),
            checkpoint: ConformanceCheckpointAuthority::new(session),
            limits: recovery_limits(),
            background_at: None,
        }
    }

    pub(in super::super) fn new_for_consult(
        store: &'a AuthorityStoreCoordinate,
        historical: &'a ConformanceHistorical,
        session: &'a ConformanceSessionFixture,
        request: &CommonRequestEnvelopeV1,
    ) -> Self {
        let mut inputs = Self::new(store, historical, session, request);
        inputs.limits = consult_recovery_limits();
        inputs
    }
    pub(in super::super) fn with_background_at(mut self, observed_at: UnixTimeV1) -> Self {
        self.background_at = Some(observed_at);
        self.limits.observation =
            OperationObservationRecoveryLimits::new(3, MAXIMUM_RECORD_BYTES, MAXIMUM_RECORD_BYTES)
                .unwrap();
        self.limits.checkpoints =
            OperationCheckpointAncestryLimits::new(3, MAXIMUM_RECORD_BYTES, 3).unwrap();
        self
    }

    fn with_recovery_inputs<Output>(
        &mut self,
        locator: Option<&ControlRecordCoordinate>,
        use_inputs: impl for<'operation, 'admission, 'checkpoint> FnOnce(
            ResidentServiceInvokeRecoveryInputs<'operation, 'admission, 'checkpoint>,
        ) -> Output,
    ) -> Result<Output, ResidentOperationFrameFailure> {
        let fallback = ControlRecordCoordinate::new(
            record_ref("unallocated-operation-handle"),
            ControlRecordDigest::parse("0".repeat(64)).expect("fallback handle digest"),
        );
        let expected = locator.unwrap_or(&fallback);
        let checkpoint = self
            .checkpoint
            .context(self.store, &self.domain, self.session);
        let input = recovery_input(
            self.store,
            &self.profile,
            &self.session.signing,
            &checkpoint,
            self.historical,
            expected,
            self.limits,
        );
        let limits = self.limits;
        with_recovery_admissions!(
            self.session,
            &schema(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID),
            &checkpoint,
            |admissions| {
                Ok(use_inputs(ResidentServiceInvokeRecoveryInputs {
                    operation: input.operation.context,
                    maximum_parent_depth: 0,
                    limits,
                    recovery_admission: admissions.recovery,
                    checkpoint_authority: &checkpoint,
                }))
            },
        )
    }
}

impl ResidentServiceInvokeInputPort for ConformanceInvokeInputs<'_> {
    fn with_invoke_inputs<Output, UseInputs>(
        &mut self,
        _: &ResidentServiceDeploymentSelector,
        _: &CommonRequestEnvelopeV1,
        locator: Option<&ControlRecordCoordinate>,
        use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'operation, 'admission, 'checkpoint> FnOnce(
            ResidentServiceInvokeRecoveryInputs<'operation, 'admission, 'checkpoint>,
        ) -> Output,
    {
        self.with_recovery_inputs(locator, use_inputs)
    }

    fn with_background_invoke_inputs<Output, UseInputs>(
        &mut self,
        _: &ResidentServiceDeploymentSelector,
        use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'operation, 'admission> FnOnce(
            ResidentServiceInvokeBackgroundRecoveryInputs<'operation, 'admission>,
        ) -> Output,
    {
        let observed_at = self
            .background_at
            .ok_or(ResidentOperationFrameFailure::ResidentUnavailable)?;
        self.with_recovery_inputs(None, |inputs| {
            use_inputs(ResidentServiceInvokeBackgroundRecoveryInputs {
                committed_observation_limits: accumulator_limits(),
                operation: inputs.operation,
                maximum_parent_depth: inputs.maximum_parent_depth,
                limits: inputs.limits,
                recovery_admission: inputs.recovery_admission,
                maximum_session_basis_bytes: MAXIMUM_RECORD_BYTES,
                observed_at,
            })
        })
    }
}

pub(in super::super) struct UnavailableCancellationInputs;

impl<'service> ResidentServiceCancellationInputPort<'service> for UnavailableCancellationInputs {
    fn with_cancellation_inputs<UseInputs>(
        &mut self,
        _: &ResidentServiceDeploymentSelector,
        _: &CommonRequestEnvelopeV1,
        _: &ControlRecordCoordinate,
        _: UseInputs,
    ) -> Result<(), ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(ResidentServiceCancellationInputs<'input>),
    {
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    fn with_a2a_cancellation_outcome_inputs<Output, UseInputs>(
        self,
        _: &ResidentServiceDeploymentSelector,
        _: &ResidentRequestIdV1,
        _: &CommonRequestEnvelopeDigest,
        _: &ControlRecordCoordinate,
        outcome: Box<A2aSendOutcomeUnknown>,
        _: UseInputs,
    ) -> (
        Self,
        Result<Output, (ResidentOperationFrameFailure, Box<A2aSendOutcomeUnknown>)>,
    )
    where
        UseInputs: FnOnce(A2aUnknownOutcomeRetention<'service>) -> Output,
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

pub(in super::super) struct UnavailableObserveInputs;

impl<'recovery> ResidentServiceObserveInputPort<'recovery> for UnavailableObserveInputs {
    fn with_observe_inputs<Output, UseInputs>(
        &mut self,
        _: &ControlRecordCoordinate,
        _: UseInputs,
    ) -> Result<Output, ResidentObserveFrameFailure>
    where
        UseInputs:
            for<'request> FnOnce(ResidentServiceObserveInputs<'request, 'recovery>) -> Output,
    {
        Err(ResidentObserveFrameFailure::ResidentUnavailable)
    }
}

struct ConformanceObserveInputs<'a> {
    recovery: Option<OperationControlRecoveryInput<'a>>,
    admission: Option<OperationControlRecoveryAdmission<'a>>,
    authorize_disclosure: Option<&'a mut ConformanceDisclosureAuthorization<'a>>,
    replay: BTreeMap<ResidentRequestIdV1, (CommonRequestEnvelopeDigest, Vec<u8>)>,
}

type ConformanceDisclosureAuthorization<'a> = dyn for<'context> FnMut(
        OperationControlAuthorizationContext<'context>,
    ) -> Result<(), InvalidResidentRecordV1>
    + 'a;

struct ConformanceRecoveryAdmissions<'a> {
    recovery: OperationControlRecoveryAdmission<'a>,
    disclosure: &'a mut ConformanceDisclosureAuthorization<'a>,
}

impl<'a> ConformanceObserveInputs<'a> {
    fn new(
        recovery: OperationControlRecoveryInput<'a>,
        admission: OperationControlRecoveryAdmission<'a>,
        authorize_disclosure: &'a mut ConformanceDisclosureAuthorization<'a>,
    ) -> Self {
        Self {
            recovery: Some(recovery),
            admission: Some(admission),
            authorize_disclosure: Some(authorize_disclosure),
            replay: BTreeMap::new(),
        }
    }
}

impl<'a> ResidentServiceObserveInputPort<'a> for ConformanceObserveInputs<'a> {
    fn with_observe_inputs<Output, UseInputs>(
        &mut self,
        locator: &ControlRecordCoordinate,
        use_inputs: UseInputs,
    ) -> Result<Output, ResidentObserveFrameFailure>
    where
        UseInputs: for<'request> FnOnce(ResidentServiceObserveInputs<'request, 'a>) -> Output,
    {
        let recovery = self
            .recovery
            .take()
            .ok_or(ResidentObserveFrameFailure::ReplayConflict)?;
        if recovery.operation.expected_handle != locator {
            return Err(ResidentObserveFrameFailure::InvalidRequest);
        }
        let recovery_admission = self
            .admission
            .take()
            .ok_or(ResidentObserveFrameFailure::ReplayConflict)?;
        let authorize_disclosure = self
            .authorize_disclosure
            .take()
            .ok_or(ResidentObserveFrameFailure::ReplayConflict)?;
        let mut verify = |_: &CommonRequestEnvelopePreimageV1,
                          _: &CommonRequestEnvelopeDigest,
                          candidate: &CanonicalSignatureV1| {
            (candidate == &signature())
                .then_some(())
                .ok_or_else(|| invalid("Observe request", "deterministic caller signature"))
        };
        let replay = &mut self.replay;
        let mut admit_replay = |request_id: &ResidentRequestIdV1,
                                digest: &CommonRequestEnvelopeDigest,
                                canonical: &[u8]| {
            let result = match replay.get(request_id) {
                None => {
                    replay.insert(request_id.clone(), (digest.clone(), canonical.to_vec()));
                    Ok(ResidentServiceObserveReplayDisposition::FirstAdmission)
                }
                Some((retained_digest, retained_canonical))
                    if retained_digest == digest && retained_canonical.as_slice() == canonical =>
                {
                    Ok(ResidentServiceObserveReplayDisposition::ExactReplay)
                }
                Some(_) => Err(ResidentServiceObserveReplayError::ChangedBytes),
            };
            result
        };
        let output = use_inputs(ResidentServiceObserveInputs {
            recovery,
            verify_caller_signature: &mut verify,
            admit_replay: &mut admit_replay,
            recovery_admission,
            authorize_disclosure,
            response_deadline: Instant::now()
                .checked_add(std::time::Duration::from_secs(2))
                .expect("bounded response deadline"),
        });
        Ok(output)
    }
}

macro_rules! with_cold_recovered_process {
    (
        $state:expr,
        $generation_source:expr,
        $events:expr,
        $custody:expr,
        $anchors:expr,
        |$process:ident| $body:block
    ) => {{
        let state = $state;
        let (writer_admission, writer_snapshot) = ConformanceWriterAdmission::construct();
        let trust = ResidentServiceTrustFrontier::for_interface_conformance(
            state.trust_ref.clone(),
            state.trust.clone(),
            state.session.binding_ref.clone(),
            state.session.binding.clone(),
            state.session.runtime_ref.clone(),
            state.session.runtime.clone(),
        )
        .map_err(|_| OperationConformanceFrontierError::Construction)?;
        let limits = AuthorityRecoveryReadLimits::new(1_000_000, 1_000_000, 8, 64)
            .expect("bounded authority recovery");
        let cold_open = ResidentColdOpenInput::new(
            state.store.clone(),
            limits,
            limits.maximum_entry_bytes(),
            RecoveryReferenceKnowledge::Known(BTreeSet::new()),
            &state.historical,
            $generation_source,
            ResidentReconstructionParameters::new(
                pale_kernel::AdaptiveNodeSetPolicy::new(32, 2_500)
                    .expect("bounded conformance generation"),
                4,
                4,
                WakeEpochV1::new(2),
            ),
        );
        let $process = ResidentServiceProcessInput {
            held_writer: ConformanceWriterGuard(state.session.principal.clone()),
            writer_snapshot: &writer_snapshot,
            writer_admission,
            trust: &trust,
            deployments: vec![ResidentServiceDeploymentOpen {
                store: state.store.clone(),
                durable: &state.store_owner,
                sessions: &state.store_owner,
                events: $events,
                maximum_event_custody_bytes: 65_536,
                cold_open,
                operations: $custody,
                anti_rollback: $anchors,
            }],
            maximum_ready_deployments: NonZeroUsize::new(1).expect("one deployment"),
            task_limits: ResidentServiceTaskLimits {
                maximum_provider_tasks: NonZeroUsize::new(1).expect("one provider task"),
                maximum_consultation_batches: NonZeroUsize::new(1).expect("one consultation batch"),
                maximum_aggregate_tasks: NonZeroUsize::new(1).expect("one aggregate task"),
            },
        };
        $body
    }};
}

/// Cold-recover Observe over the custody used by the earlier Start service.
///
/// Historical recovery callbacks, present disclosure authority, and the
/// recovered service share this lexical scope. No admission callback or
/// private fixture value escapes it. The caller supplies the bound for the
/// operation's actual checkpoint ancestry; Consult has one more retained
/// checkpoint than Invoke because its collection custody is separate.
#[allow(clippy::too_many_arguments)]
pub(in super::super) fn with_cold_recovered_observe_service<Output>(
    state: &ConformanceResidentState,
    generation_source: &mut ConformanceGenerationSource,
    events: &mut UnavailableEventCustody,
    custody: &mut ConformanceOperationCustody,
    anchors: &mut ConformanceAntiRollbackAnchors,
    evidence: ResidentServiceCurrentRequestEvidence,
    observe_policy: ResidentServicePolicyProjection,
    start: &CommonRequestEnvelopeV1,
    locator: &ResidentOperationFrameLocator,
    recovery_limits: ResidentServiceInvokeRecoveryLimits,
    use_service: impl FnOnce(&mut ResidentFrameService<'_>) -> Output,
) -> Result<Output, OperationConformanceFrontierError> {
    let coordinate = ControlRecordCoordinate::new(
        ResidentRecordRefV1::parse(locator.operation_handle_record_ref().to_owned())
            .map_err(|_| OperationConformanceFrontierError::Construction)?,
        ControlRecordDigest::parse(locator.operation_handle_sha256().to_owned())
            .map_err(|_| OperationConformanceFrontierError::Construction)?,
    );
    let domain = SupportedOperationCheckpointProfile::conflict_domain(start)
        .map_err(|_| OperationConformanceFrontierError::Construction)?;
    let profile = supported_profile();
    let checkpoint_coordinates = ConformanceCheckpointAuthority::new(&state.session);
    let checkpoint_context = checkpoint_coordinates.context(&state.store, &domain, &state.session);
    let checkpoint_schema = schema(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID);
    with_recovery_admissions!(
        &state.session,
        &checkpoint_schema,
        &checkpoint_context,
        |admissions| {
            let observe_inputs = ConformanceObserveInputs::new(
                recovery_input(
                    &state.store,
                    &profile,
                    &state.session.signing,
                    &checkpoint_context,
                    &state.historical,
                    &coordinate,
                    recovery_limits,
                ),
                admissions.recovery,
                admissions.disclosure,
            );
            with_cold_recovered_process!(
                state,
                generation_source,
                events,
                custody,
                anchors,
                |process| {
                    let owners = ResidentServiceObserveOwners {
                        peer_admission: Box::new(ConformancePeerAdmission(state.expected_peer)),
                        maximum_request_bytes: 1 << 20,
                        policies: vec![observe_policy],
                        historical: &state.historical,
                        evidence: ConformanceEvidence::new(evidence),
                        clock: ConformanceClock,
                        session_authority: proving_authority(),
                        caller_signature: ConformanceCallerSignature,
                        operation_authority: proving_authority(),
                        observe_inputs,
                    };
                    let mut service = ResidentFrameService::recover_observe(process, owners)?;
                    Ok(use_service(&mut service))
                }
            )
        },
    )
}

/// Cold-recover the ordinary Invoke and Observe frame service.
///
/// The caller supplies admitted evidence and callback-scoped custody. This
/// helper adds no interface semantics and returns no private owner.
#[allow(clippy::too_many_arguments)]
pub(in super::super) fn with_cold_recovered_invoke_observe_service<
    'service,
    ObserveInputs,
    Output,
>(
    state: &'service ConformanceResidentState,
    generation_source: &'service mut ConformanceGenerationSource,
    events: &'service mut UnavailableEventCustody,
    custody: &'service mut ConformanceOperationCustody,
    anchors: &'service mut ConformanceAntiRollbackAnchors,
    evidence: ResidentServiceCurrentRequestEvidence,
    request_policies: ConformanceInvokeRequestPolicies,
    invoke_policy: Option<ResidentServiceOperationStartPolicy>,
    cryptography: ConformanceCryptography,
    invoke_inputs: ConformanceInvokeInputs<'service>,
    observe_inputs: ObserveInputs,
    use_service: impl FnOnce(&mut ResidentFrameService<'_>) -> Output,
) -> Result<Output, OperationConformanceFrontierError>
where
    ObserveInputs: ResidentServiceObserveInputPort<'service> + 'service,
{
    with_cold_recovered_process!(
        state,
        generation_source,
        events,
        custody,
        anchors,
        |process| {
            let request_policies = ResidentServiceOperationRequestPolicies {
                start: vec![request_policies.start],
                advance: vec![request_policies.advance],
                start_consult: Vec::new(),
                advance_consult: Vec::new(),
                start_delegate: Vec::new(),
                advance_delegate: Vec::new(),
                observe: vec![request_policies.observe],
                cancel: Vec::new(),
            };
            let owners = ResidentServiceInvokeObserveCancelOwners {
                peer_admission: Box::new(ConformancePeerAdmission(state.expected_peer)),
                maximum_request_bytes: 1 << 20,
                request_policies,
                historical: &state.historical,
                evidence: ConformanceEvidence::new(evidence),
                clock: ConformanceClock,
                session_authority: proving_authority(),
                caller_signature: ConformanceCallerSignature,
                random: ConformanceEntropy::default(),
                operation_authority: proving_authority(),
                policy: ConformanceInvokePolicy::from(invoke_policy),
                cryptography,
                invoke_inputs,
                cancel_inputs: UnavailableCancellationInputs,
                observe_inputs,
            };
            let mut service = ResidentFrameService::recover_invoke_observe_cancel(process, owners)?;
            Ok(use_service(&mut service))
        }
    )
}

/// Cold-recover the ordinary Consult and Observe frame service.
#[allow(clippy::too_many_arguments)]
pub(in super::super) fn with_cold_recovered_consult_observe_service<
    'service,
    ObserveInputs,
    Output,
>(
    state: &'service ConformanceResidentState,
    generation_source: &'service mut ConformanceGenerationSource,
    events: &'service mut UnavailableEventCustody,
    custody: &'service mut dyn OperationCustodyPort,
    anchors: &'service mut ConformanceAntiRollbackAnchors,
    evidence: ResidentServiceCurrentRequestEvidence,
    request_policies: ConformanceConsultRequestPolicies,
    policy: ConformanceConsultPolicy,
    cryptography: ConformanceCryptography,
    invoke_inputs: ConformanceInvokeInputs<'service>,
    consult_inputs: ConformanceConsultInputs<'service>,
    observe_inputs: ObserveInputs,
    use_service: impl FnOnce(&mut ResidentFrameService<'_>) -> Output,
) -> Result<Output, OperationConformanceFrontierError>
where
    ObserveInputs: ResidentServiceObserveInputPort<'service> + 'service,
{
    with_cold_recovered_process!(
        state,
        generation_source,
        events,
        custody,
        anchors,
        |process| {
            let request_policies = ResidentServiceOperationRequestPolicies {
                start: Vec::new(),
                advance: Vec::new(),
                start_consult: vec![request_policies.start_consult],
                advance_consult: vec![request_policies.advance_consult],
                start_delegate: Vec::new(),
                advance_delegate: Vec::new(),
                observe: vec![request_policies.observe],
                cancel: Vec::new(),
            };
            let owners = ResidentServiceInvokeConsultObserveCancelOwners {
                peer_admission: Box::new(ConformancePeerAdmission(state.expected_peer)),
                maximum_request_bytes: 1 << 20,
                request_policies,
                historical: &state.historical,
                evidence: ConformanceEvidence::new(evidence),
                clock: ConformanceConsultClock {
                    counters: Arc::clone(&consult_inputs.counters),
                },
                session_authority: proving_authority(),
                caller_signature: ConformanceCallerSignature,
                random: ConformanceEntropy::for_consult(Arc::clone(&consult_inputs.counters)),
                operation_authority: proving_authority(),
                policy,
                cryptography,
                invoke_inputs,
                consult_inputs,
                cancel_inputs: UnavailableCancellationInputs,
                observe_inputs,
            };
            let mut service =
                ResidentFrameService::recover_invoke_consult_observe_cancel(process, owners)?;
            Ok(use_service(&mut service))
        }
    )
}

#[derive(Clone)]
pub(in super::super) struct ConformanceInvokeRequestPolicies {
    start: ResidentServicePolicyProjection,
    advance: ResidentServicePolicyProjection,
    observe: ResidentServicePolicyProjection,
}

impl ConformanceInvokeRequestPolicies {
    pub(in super::super) const fn new(
        start: ResidentServicePolicyProjection,
        advance: ResidentServicePolicyProjection,
        observe: ResidentServicePolicyProjection,
    ) -> Self {
        Self {
            start,
            advance,
            observe,
        }
    }
}

#[derive(Clone)]
pub(in super::super) struct ConformanceConsultRequestPolicies {
    start_consult: ResidentServicePolicyProjection,
    advance_consult: ResidentServicePolicyProjection,
    observe: ResidentServicePolicyProjection,
}

impl ConformanceConsultRequestPolicies {
    pub(in super::super) const fn new(
        start_consult: ResidentServicePolicyProjection,
        advance_consult: ResidentServicePolicyProjection,
        observe: ResidentServicePolicyProjection,
    ) -> Self {
        Self {
            start_consult,
            advance_consult,
            observe,
        }
    }
}

pub(in super::super) fn service_policy(
    generation: &AdmittedCompiledGeneration,
    repository_id: LogicalRepositoryIdV1,
    request: &CommonRequestEnvelopeV1,
) -> ResidentServicePolicyProjection {
    let value = request.to_preimage();
    ResidentServicePolicyProjection::admit(
        generation,
        ResidentServicePolicyInput {
            repository_id,
            body_schema_id: value.body.schema_id,
            body_schema_sha256: value.body.schema_sha256,
            caller: value.caller,
            signing_authority: value.signing_authority_binding,
            route: value.route,
            execution_owner_fence: None,
            federation_claim: value.federation_claim,
            workspace_id: value.workspace_id,
            capability_id: value.capability_id,
            operation: value.operation,
            scope: value.scope,
            risk_class: value.risk_class,
            resource_constraints: value.resource_constraints,
            expected_precondition: value.expected_precondition,
            maximum_request_lifetime_seconds: 30,
            fence_token: value.fence_token,
        },
    )
    .expect("resident request policy admits")
}

pub(in super::super) fn observe_service_policy(
    generation: &AdmittedCompiledGeneration,
    repository_id: LogicalRepositoryIdV1,
    start: &CommonRequestEnvelopeV1,
    activation_fence: &ActivationFenceDigest,
) -> ResidentServicePolicyProjection {
    let mut value = start.to_preimage();
    value.capability_id =
        CapabilityIdV1::parse(OPERATION_OBSERVE_CAPABILITY_ID).expect("Observe capability");
    value.scope.purpose = BoundedResidentTextV1::parse(ConformanceRequestPhase::Observe.purpose())
        .expect("Observe purpose");
    value.fence_token = Some(activation_fence.clone());
    ResidentServicePolicyProjection::admit(
        generation,
        ResidentServicePolicyInput {
            repository_id,
            body_schema_id: SchemaId::parse(OPERATION_HANDLE_V1_SCHEMA)
                .expect("operation-handle schema"),
            body_schema_sha256: schema(OPERATION_HANDLE_V1_SCHEMA_ID),
            caller: value.caller,
            signing_authority: value.signing_authority_binding,
            route: value.route,
            execution_owner_fence: None,
            federation_claim: value.federation_claim,
            workspace_id: value.workspace_id,
            capability_id: value.capability_id,
            operation: value.operation,
            scope: value.scope,
            risk_class: value.risk_class,
            resource_constraints: value.resource_constraints,
            expected_precondition: value.expected_precondition,
            maximum_request_lifetime_seconds: 30,
            fence_token: value.fence_token,
        },
    )
    .expect("resident Observe request policy admits")
}

pub(in super::super) fn proving_authority() -> ProvingAuthority {
    ProvingAuthority
}

#[path = "../../../../../tests/support/interface_consult_expiry.rs"]
mod expiry_test_support;
pub(in super::super) use expiry_test_support::prepare_interrupted_consult_expiry;

#[path = "../../../../../tests/support/interface_consult_child_restart.rs"]
mod child_restart_test_support;
pub use child_restart_test_support::{
    exercise_consult_child_ambiguous_restart, exercise_consult_child_collection_anchor_restart,
    exercise_consult_child_prefix_service_restart, exercise_consult_child_service_restart,
    exercise_consult_child_suffix_service_restart,
};
