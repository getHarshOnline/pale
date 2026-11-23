//! Hostile laws for selector-fenced durable local operation starts.

use super::*;
use crate::resident::consultation::tests::{
    issue_test_child_with_evidence, planned_test_child, prepare_collection_accumulator,
    with_test_child_start_input, CollectionCommitPort, CollectionRecoveryOnly,
};
use crate::resident::local_operation_coordinator::{
    ResidentLocalOperationCheckpointInput, ResidentLocalOperationCustodyFailure,
    ResidentLocalOperationStartInput,
};
use crate::resident::operation_observation_custody::tests::local_start::{
    absent_start_store, initial_anchor, initial_response, source_generation,
    source_generation_copies, with_issued_context,
};
use crate::resident::process_host::consult_start::{
    ResidentProcessConsultationChildStartError, ResidentProcessConsultationCollectionInput,
    ResidentProcessConsultationInitialization,
    ResidentProcessConsultationInitializationChildFailure,
    ResidentProcessConsultationOpenJourneyError, ResidentProcessConsultationResumeError,
};
use crate::resident::{
    ConsultationCollection, ConsultationCollectionAdmission, ConsultationCollectionLimits,
    ConsultationCollectionObservationInput, ConsultationContributionSource,
    ConsultationRequirement, ConsultationRosterSlot, ConsultationSlotId,
    OperationCheckpointAncestryLimits, OperationObservationRecoveryLimits,
};
use pale_core::records::{
    CapabilityRevocationCheckpointV1, CapabilitySessionGrantRevocationV1, CapabilitySessionGrantV1,
    CommonRequestAdmissionContextV1, CommonRequestEnvelopePreimageV1, ControlCheckpointDomainIdV1,
    ControlCheckpointIdV1, ControlTransactionSequenceV1, DataClassificationV1,
    DeploymentAcceptanceRevocationEpochV1, DeploymentAcceptanceRevocationV1, DeploymentBindingV1,
    DeploymentRuntimeAuthorityV1, ImmutableRecordReferenceV1, OperationClassV1,
    OperationObservationAdmissionContextV1, OperationObservationV1, ParticipantRefV1,
    PrincipalRevocationEpochV1, PrincipalTrustCheckpointV1, SessionAdmissionLeaseDigest,
    SessionAdmissionLeasePreimageV1, SessionAdmissionLeaseWireV1, SignedResidentAdmissionContextV1,
};
use pale_ports::{
    AmbientSessionAuthorityError, AmbientSessionAuthorityPort, ControlRecord,
    ControlRecordCoordinate, ControlStoreError, ControlStoreErrorKind,
    ControlTransactionPreparation, OperationCollectionFrontierEvidence,
    OperationCollectionHeadObservation, OperationCollectionSnapshot, OperationCustodyCandidate,
    OperationCustodyIndexObservation, OperationCustodyIndexQuery, OperationCustodyPort,
    OperationCustodyRecord, ResidentOperationFrameRecords, ResidentOperationFrameRequest,
};
use std::{
    cell::{Cell, RefCell},
    num::NonZeroUsize,
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
    time::Instant,
};

use crate::resident::process_host::operation_request_host::{
    ResidentOperationRequestHost, ResidentOperationRequestHostError, ResidentOperationRequestKind,
    ResidentOperationRequestOutcome, ResidentOperationRequestResolution,
};

#[test]
fn resident_operation_request_host_rejects_noncanonical_and_oversized_frames() {
    let deadline = Instant::now()
        .checked_add(std::time::Duration::from_secs(2))
        .expect("test deadline");
    for inbound in [
        br#"{ "kind": "start_consult" }"#.to_vec().into_boxed_slice(),
        vec![b'x'; 262_145].into_boxed_slice(),
    ] {
        let mut port = OperationFramePort {
            peer: LocalIpcPeerCredentials::new(501, 20, Some(8_081)),
            inbound: Some(inbound),
            sent: Vec::new(),
        };
        let error = ResidentOperationRequestHost::new(&mut port, deadline)
            .receive()
            .expect_err("invalid frame must fail before application dispatch");
        assert!(matches!(error, ResidentOperationRequestHostError::Frame));
        assert!(port.sent.is_empty());
    }
}

#[test]
fn resident_operation_request_host_denies_peer_before_semantic_resolution() {
    let frame = ResidentOperationFrameRequest::StartInvoke {
        signed_common_request: "{}".to_owned(),
    };
    let deadline = Instant::now()
        .checked_add(std::time::Duration::from_secs(2))
        .expect("test deadline");
    let mut port = OperationFramePort::new(&frame);
    let failure = {
        let mut host = ResidentOperationRequestHost::new(&mut port, deadline);
        let received = host.receive().expect("transport frame is canonical");
        host.admit_peer(
            received,
            &mut |_, _| -> Result<(), InvalidResidentRecordV1> {
                Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "resident operation peer",
                    invariant: "test peer is denied",
                })
            },
        )
        .expect_err("denied peer must stop before semantic resolution")
    };
    assert!(matches!(
        failure,
        ResidentOperationRequestHostError::PeerAdmission(_)
    ));
    assert!(port.sent.is_empty());
}

#[test]
fn resident_operation_request_host_retains_the_typed_peer_witness() {
    let frame = ResidentOperationFrameRequest::StartInvoke {
        signed_common_request: "{}".to_owned(),
    };
    let deadline = Instant::now()
        .checked_add(std::time::Duration::from_secs(2))
        .expect("test deadline");
    let mut port = OperationFramePort::new(&frame);
    let admitted = {
        let mut host = ResidentOperationRequestHost::new(&mut port, deadline);
        let received = host.receive().expect("transport frame is canonical");
        host.admit_peer(received, &mut |_, _| Ok(7_u8))
            .expect("peer is admitted")
    };
    assert_eq!(admitted.peer(), &7);
    assert_eq!(admitted.request(), &frame);
    assert!(port.sent.is_empty());
}

pub(super) fn start_records(
    operation: &crate::resident::IssuedLocalOperation,
    candidate: &OperationCustodyCandidate,
) -> ResidentOperationFrameRecords {
    let handle = candidate
        .records()
        .iter()
        .find_map(|record| match record {
            OperationCustodyRecord::OperationHandle(record) => Some(record),
            _ => None,
        })
        .expect("durable start retained its handle");
    let observation = candidate
        .records()
        .iter()
        .find_map(|record| match record {
            OperationCustodyRecord::OperationObservation(record) => Some(record),
            _ => None,
        })
        .expect("durable start retained its initial observation");
    assert_eq!(handle.coordinate().record_ref(), operation.handle().0);
    ResidentOperationFrameRecords::new(
        handle.coordinate().record_ref().as_str().to_owned(),
        std::str::from_utf8(handle.canonical_record_jcs().as_slice())
            .expect("handle is canonical UTF-8")
            .to_owned(),
        observation.coordinate().record_ref().as_str().to_owned(),
        std::str::from_utf8(observation.canonical_record_jcs().as_slice())
            .expect("observation is canonical UTF-8")
            .to_owned(),
    )
}

#[derive(Default)]
pub(in crate::resident::process_host) struct CountingAuthority {
    pub(in crate::resident) calls: usize,
    reject_signature: bool,
}

impl CountingAuthority {
    pub(super) const fn rejecting_signature() -> Self {
        Self {
            calls: 0,
            reject_signature: true,
        }
    }
}

impl AmbientSessionAuthorityPort for CountingAuthority {
    fn verify_session_admission_lease_signature(
        &mut self,
        _: &[u8],
        _: &SessionAdmissionLeasePreimageV1,
        _: &SessionAdmissionLeaseDigest,
        _: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        self.calls += 1;
        if self.reject_signature {
            Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "test session admission lease",
                invariant: "signature rejected by current authority",
            })
        } else {
            Ok(())
        }
    }

    fn prove_current_principal_trust(
        &mut self,
        _: &ResidentRecordRefV1,
        _: &PrincipalTrustCheckpointV1,
        _: &ResidentRecordRefV1,
        _: &PrincipalTrustCheckpointV1,
    ) -> Result<(), AmbientSessionAuthorityError> {
        self.calls += 1;
        Ok(())
    }

    fn prove_capability_revocation_frontier(
        &mut self,
        _: &ResidentRecordRefV1,
        _: &CapabilityRevocationCheckpointV1,
        _: &ResidentRecordRefV1,
        _: &CapabilitySessionGrantV1,
        _: Option<&ResidentRecordRefV1>,
        _: Option<&CapabilitySessionGrantRevocationV1>,
    ) -> Result<(), AmbientSessionAuthorityError> {
        self.calls += 1;
        Ok(())
    }

    fn prove_deployment_revocation_frontier(
        &mut self,
        _: &DeploymentBindingV1,
        _: DeploymentAcceptanceRevocationEpochV1,
        _: Option<&ResidentRecordRefV1>,
        _: Option<&DeploymentAcceptanceRevocationV1>,
        _: &PrincipalTrustCheckpointV1,
    ) -> Result<(), AmbientSessionAuthorityError> {
        self.calls += 1;
        Ok(())
    }

    fn prove_runtime_authority_head(
        &mut self,
        _: &ResidentRecordRefV1,
        _: &DeploymentRuntimeAuthorityV1,
        _: &PrincipalTrustCheckpointV1,
    ) -> Result<(), AmbientSessionAuthorityError> {
        self.calls += 1;
        Ok(())
    }

    fn prove_session_revocation_frontier(
        &mut self,
        _: &ResidentPrincipalIdV1,
        _: PrincipalRevocationEpochV1,
        _: Option<&ResidentRecordRefV1>,
        _: Option<&ArtifactDigest>,
        _: &PrincipalTrustCheckpointV1,
    ) -> Result<(), AmbientSessionAuthorityError> {
        self.calls += 1;
        Ok(())
    }

    fn prove_data_classification_ceiling(
        &mut self,
        _: &DataClassificationV1,
        _: &DataClassificationV1,
    ) -> Result<(), AmbientSessionAuthorityError> {
        self.calls += 1;
        Ok(())
    }
}

struct ConsultRecoverySession(pale_ports::SessionLeaseBody, AtomicUsize);

impl pale_ports::SessionLeaseCustodyPort for ConsultRecoverySession {
    fn compare_and_retain(
        &self,
        _: &pale_ports::SessionLeaseBody,
    ) -> Result<pale_ports::SessionLeaseBodyRetention, ControlStoreError> {
        panic!("consult restart must not renew a session")
    }

    fn read(
        &self,
        query: &pale_ports::SessionLeaseBodyRead,
    ) -> Result<Option<pale_ports::SessionLeaseBody>, ControlStoreError> {
        self.1.fetch_add(1, Ordering::Relaxed);
        assert_eq!(query.store(), self.0.store());
        assert_eq!(query.digest(), self.0.digest());
        assert!(self.0.canonical_wire_jcs().as_slice().len() <= query.maximum_body_bytes());
        Ok(Some(self.0.clone()))
    }
}

struct ConsultRecoveryAnchor {
    value: pale_ports::AntiRollbackAnchor,
    reads: usize,
    writes: usize,
}

impl ConsultRecoveryAnchor {
    fn select(&mut self, candidate: &OperationCustodyCandidate) {
        let head = candidate.checkpoint().successor();
        self.value = pale_ports::AntiRollbackAnchor::ControlDomain(
            pale_ports::ControlDomainAntiRollbackAnchor::new(
                candidate.store().store_id().clone(),
                candidate.store().incarnation_id().clone(),
                head.checkpoint_domain_id().clone(),
                head.domain_epoch(),
                head.domain_sequence(),
                head.coordinate().clone(),
            ),
        );
    }
}

impl pale_ports::AntiRollbackAnchorPort for ConsultRecoveryAnchor {
    fn read(
        &mut self,
        _: &pale_ports::AntiRollbackAnchorKey,
    ) -> Result<Option<pale_ports::AntiRollbackAnchor>, pale_ports::AntiRollbackAnchorError> {
        self.reads += 1;
        Ok(Some(self.value.clone()))
    }

    fn compare_and_advance(
        &mut self,
        _: &pale_ports::PreparedAntiRollbackAnchorTransition,
    ) -> Result<pale_ports::AntiRollbackAnchorAdvanceDisposition, pale_ports::AntiRollbackAnchorError>
    {
        self.writes += 1;
        Err(pale_ports::AntiRollbackAnchorError::new(
            pale_ports::AntiRollbackAnchorErrorKind::Unavailable,
        ))
    }

    fn observe(
        &mut self,
        _: &pale_ports::PreparedAntiRollbackAnchorTransition,
    ) -> Result<pale_ports::AntiRollbackAnchorObservation, pale_ports::AntiRollbackAnchorError>
    {
        self.writes += 1;
        Err(pale_ports::AntiRollbackAnchorError::new(
            pale_ports::AntiRollbackAnchorErrorKind::Unavailable,
        ))
    }
}

/// Read-only custody for one parent and its exact committed child prefix.
struct ConsultPrefixCustody {
    store: pale_ports::AuthorityStoreCoordinate,
    candidates: Vec<OperationCustodyCandidate>,
    leases: Vec<ControlRecord>,
    hidden_handles: Vec<ControlRecordCoordinate>,
    incomplete_handles: Vec<ControlRecordCoordinate>,
    substitution: Option<(ControlRecordCoordinate, ControlRecord)>,
    reads: usize,
    writes: usize,
}

impl ConsultPrefixCustody {
    fn new(
        parent: &crate::resident::IssuedLocalOperation,
        candidate: OperationCustodyCandidate,
    ) -> Self {
        let store = candidate.store().clone();
        let mut value = Self {
            store,
            candidates: Vec::new(),
            leases: Vec::new(),
            hidden_handles: Vec::new(),
            incomplete_handles: Vec::new(),
            substitution: None,
            reads: 0,
            writes: 0,
        };
        value.retain(parent, candidate);
        value
    }

    fn retain(
        &mut self,
        operation: &crate::resident::IssuedLocalOperation,
        candidate: OperationCustodyCandidate,
    ) {
        assert_eq!(candidate.store(), &self.store);
        let lease = operation.historical_operation_lease();
        let coordinate = crate::resident::record_projection::coordinate(
            operation.handle().1.operation_lease_ref(),
            lease.operation_lease_digest().as_str(),
        )
        .expect("operation lease coordinate");
        let record = crate::resident::record_projection::record(coordinate, lease)
            .expect("operation lease record");
        assert!(self
            .leases
            .iter()
            .all(|retained| retained.coordinate() != record.coordinate()));
        self.leases.push(record);
        self.candidates.push(candidate);
    }

    fn latest_candidate(&self) -> &OperationCustodyCandidate {
        self.candidates
            .last()
            .expect("at least the parent is retained")
    }

    fn handle_record(candidate: &OperationCustodyCandidate) -> &ControlRecord {
        candidate
            .record(pale_ports::OperationCustodyRecordKind::OperationHandle)
            .expect("committed operation has a handle")
    }

    fn hide(&mut self, coordinate: ControlRecordCoordinate) {
        self.hidden_handles.push(coordinate);
    }

    fn mark_incomplete(&mut self, coordinate: ControlRecordCoordinate) {
        self.incomplete_handles.push(coordinate);
    }

    fn substitute(&mut self, expected: ControlRecordCoordinate, actual: ControlRecord) {
        self.substitution = Some((expected, actual));
    }

    fn clear_faults(&mut self) {
        self.hidden_handles.clear();
        self.incomplete_handles.clear();
        self.substitution = None;
    }

    const fn io_counts(&self) -> (usize, usize) {
        (self.reads, self.writes)
    }

    fn candidate_is_incomplete(&self, candidate: &OperationCustodyCandidate) -> bool {
        let handle = Self::handle_record(candidate).coordinate();
        self.incomplete_handles.iter().any(|value| value == handle)
    }
}

impl pale_ports::ControlTransactionPort for ConsultPrefixCustody {
    fn prepare_transaction(
        &mut self,
        _: &pale_ports::AuthorityStoreCoordinate,
    ) -> Result<pale_ports::ControlTransactionPreparation, ControlStoreError> {
        self.writes += 1;
        Err(ControlStoreError::new(ControlStoreErrorKind::Storage))
    }
}

impl OperationCustodyPort for ConsultPrefixCustody {
    fn read_operation_session_basis(
        &mut self,
        store: &pale_ports::AuthorityStoreCoordinate,
        handle: &ControlRecordCoordinate,
        maximum_bytes: usize,
    ) -> Result<Option<pale_ports::OperationSessionBasis>, ControlStoreError> {
        self.reads += 1;
        assert_eq!(store, &self.store);
        let basis = self
            .candidates
            .iter()
            .find_map(|candidate| {
                candidate
                    .session_basis()
                    .filter(|basis| basis.handle() == handle)
            })
            .cloned();
        if let Some(basis) = &basis {
            assert!(basis.canonical_basis_jcs().len() <= maximum_bytes);
        }
        Ok(basis)
    }

    fn read_a2a_task_bindings(
        &mut self,
        _: &pale_ports::AuthorityStoreCoordinate,
        _: &pale_ports::OperationA2aBindingScope,
        _: Option<&pale_ports::OperationA2aBindingCursor>,
        _: usize,
        _: usize,
    ) -> Result<pale_ports::OperationA2aBindings, ControlStoreError> {
        panic!("A2A discovery is outside Consult prefix recovery")
    }

    fn read_publication_sources(
        &mut self,
        _: &pale_ports::AuthorityStoreCoordinate,
        _: Option<&pale_ports::OperationPublicationCursor>,
        _: usize,
        _: usize,
    ) -> Result<pale_ports::OperationPublicationSources, pale_ports::OperationPublicationReadError>
    {
        panic!("publication discovery is outside Consult prefix recovery")
    }

    fn read_checkpoint_head(
        &mut self,
        store: &pale_ports::AuthorityStoreCoordinate,
        domain: &ControlCheckpointDomainIdV1,
        maximum_record_bytes: usize,
    ) -> Result<Option<(pale_ports::ControlCheckpointHead, ControlRecord)>, ControlStoreError> {
        self.reads += 1;
        let candidate = self.latest_candidate();
        let checkpoint = candidate.checkpoint();
        assert_eq!(store, &self.store);
        assert_eq!(domain, checkpoint.successor().checkpoint_domain_id());
        assert!(
            checkpoint
                .successor_record()
                .canonical_record_jcs()
                .as_slice()
                .len()
                <= maximum_record_bytes
        );
        Ok(Some((
            checkpoint.successor().clone(),
            checkpoint.successor_record().clone(),
        )))
    }

    fn verify_accumulator_membership(
        &mut self,
        request: &pale_ports::OperationAccumulatorMembership,
    ) -> Result<
        pale_ports::VerifiedOperationAccumulatorMembership,
        pale_ports::OperationAccumulatorPreparationError,
    > {
        self.reads += 1;
        Ok(pale_ports::VerifiedOperationAccumulatorMembership::new(
            request.clone(),
        ))
    }

    fn prepare_accumulator(
        &mut self,
        _: &pale_ports::OperationAccumulatorPreparation,
    ) -> Result<
        pale_ports::PreparedOperationAccumulator,
        pale_ports::OperationAccumulatorPreparationError,
    > {
        self.writes += 1;
        panic!("Consult restart must not prepare a mutation")
    }

    fn compare_and_retain_operation_lease(
        &mut self,
        _: &pale_ports::AuthorityStoreCoordinate,
        _: &ControlRecord,
    ) -> Result<pale_ports::OperationCustodyCommitOutcome, ControlStoreError> {
        self.writes += 1;
        Err(ControlStoreError::new(ControlStoreErrorKind::Storage))
    }

    fn read_operation_record(
        &mut self,
        store: &pale_ports::AuthorityStoreCoordinate,
        coordinate: &ControlRecordCoordinate,
        maximum_body_bytes: usize,
    ) -> Result<Option<ControlRecord>, ControlStoreError> {
        self.reads += 1;
        assert_eq!(store, &self.store);
        if self
            .hidden_handles
            .iter()
            .any(|hidden| hidden == coordinate)
        {
            return Ok(None);
        }
        if let Some((expected, actual)) = &self.substitution {
            if expected == coordinate {
                return Ok(Some(actual.clone()));
            }
        }
        let record = self
            .candidates
            .iter()
            .flat_map(|candidate| {
                candidate
                    .records()
                    .iter()
                    .map(OperationCustodyRecord::record)
                    .chain(std::iter::once(candidate.checkpoint().successor_record()))
                    .chain(candidate.checkpoint().predecessor_record())
            })
            .chain(self.leases.iter())
            .find(|record| record.coordinate() == coordinate)
            .cloned();
        if let Some(record) = &record {
            assert!(record.canonical_record_jcs().as_slice().len() <= maximum_body_bytes);
        }
        Ok(record)
    }

    fn compare_and_commit(
        &mut self,
        _: &OperationCustodyCandidate,
    ) -> Result<pale_ports::OperationCustodyCommitOutcome, ControlStoreError> {
        self.writes += 1;
        Err(ControlStoreError::new(ControlStoreErrorKind::Storage))
    }

    fn observe_exact(
        &mut self,
        _: &OperationCustodyCandidate,
    ) -> Result<pale_ports::OperationCustodyReadBackOutcome, ControlStoreError> {
        self.writes += 1;
        Err(ControlStoreError::new(ControlStoreErrorKind::Storage))
    }

    fn read_index(
        &mut self,
        store: &pale_ports::AuthorityStoreCoordinate,
        query: &OperationCustodyIndexQuery,
        _: usize,
    ) -> Result<OperationCustodyIndexObservation, ControlStoreError> {
        self.reads += 1;
        assert_eq!(store, &self.store);
        for candidate in self.candidates.iter().rev() {
            if crate::resident::operation_observation_custody::tests::candidate_indexed_record(
                candidate, query,
            )
            .is_some()
                && self.candidate_is_incomplete(candidate)
            {
                return Ok(OperationCustodyIndexObservation::absent(query.clone()));
            }
            if let Some(record) =
                crate::resident::operation_observation_custody::tests::candidate_indexed_record(
                    candidate, query,
                )
            {
                return OperationCustodyIndexObservation::exact(query.clone(), record)
                    .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired));
            }
        }
        Ok(OperationCustodyIndexObservation::absent(query.clone()))
    }
}

fn consult_request_context(
    request: &CommonRequestEnvelopePreimageV1,
) -> CommonRequestAdmissionContextV1<'_> {
    CommonRequestAdmissionContextV1 {
        expected_schema_sha256: &request.envelope_schema_sha256,
        expected_body_schema_id: &request.body.schema_id,
        expected_body_schema_sha256: &request.body.schema_sha256,
        caller: &request.caller,
        signing_authority: &request.signing_authority_binding,
        route: &request.route,
        session_admission: &request.session_admission,
        operation_target: request.operation_target.as_ref(),
        execution_owner_fence: None,
        federation_claim: None,
        workspace_id: &request.workspace_id,
        capability_id: &request.capability_id,
        operation: &request.operation,
        scope: &request.scope,
        risk_class: &request.risk_class,
        resource_constraints: &request.resource_constraints,
        expected_precondition: None,
        idempotency_key: &request.idempotency_key,
        issued_at: request.issued_at,
        expires_at: request.expires_at,
        generation: request.generation.as_ref(),
        fence_token: request.fence_token.as_ref(),
    }
}

#[derive(Clone, Copy)]
enum PersistedIntentFault {
    ZeroLimit,
    InvalidTime,
    DuplicateSlot,
    DuplicateChild,
    WrongChildAssociation,
}

fn persisted_intent_fault(
    intent: &crate::resident::consultation::ConsultationStartIntent,
    fault: PersistedIntentFault,
) -> crate::resident::consultation::ConsultationStartIntent {
    let mut value = serde_json::to_value(intent).expect("intent is serializable");
    match fault {
        PersistedIntentFault::ZeroLimit => value["maximum_slots"] = serde_json::json!(0),
        PersistedIntentFault::InvalidTime => {
            let deadline = value["deadline"].as_u64().expect("numeric deadline");
            value["opened_at"] = serde_json::json!(deadline + 1);
        }
        PersistedIntentFault::DuplicateSlot => {
            value["slots"][1]["slot_id"] = value["slots"][0]["slot_id"].clone();
        }
        PersistedIntentFault::DuplicateChild => {
            value["slots"][1]["source"] = value["slots"][0]["source"].clone();
        }
        PersistedIntentFault::WrongChildAssociation => {
            value["slots"][0]["source"]["child"]["parent_operation"]["operation_handle_id"] =
                serde_json::json!("substituted-parent");
        }
    }
    let bytes =
        pale_core::canon::to_canonical_string(&value).expect("fault remains canonical JSON");
    serde_json::from_value(pale_core::canon::parse_strict(bytes.as_bytes()).unwrap())
        .expect("persisted private intent decodes")
}

fn persisted_basis_with_intent(
    basis: &pale_ports::OperationSessionBasis,
    intent: &crate::resident::consultation::ConsultationStartIntent,
) -> pale_ports::OperationSessionBasis {
    let mut body = pale_core::canon::parse_strict(basis.canonical_basis_jcs())
        .expect("retained session basis is strict JSON");
    body["consultation_start_intent"] =
        serde_json::to_value(intent).expect("intent is serializable");
    let bytes = pale_core::canon::to_canonical_string(&body)
        .expect("changed session basis remains canonical")
        .into_bytes();
    pale_ports::OperationSessionBasis::new(
        basis.handle().clone(),
        basis.request().clone(),
        basis.session_lease_digest().clone(),
        bytes,
        pale_ports::MAX_OPERATION_SESSION_BASIS_BYTES,
    )
    .expect("changed session basis remains bounded")
}

fn accept_request_signature(
    _: &CommonRequestEnvelopePreimageV1,
    _: &pale_core::records::CommonRequestEnvelopeDigest,
    _: &CanonicalSignatureV1,
) -> Result<(), InvalidResidentRecordV1> {
    Ok(())
}

fn accept_root_handle_authority(
    _: &OperationHandlePreimageV1,
    parent: Option<&OperationHandleV1>,
) -> Result<(), InvalidResidentRecordV1> {
    assert!(parent.is_none());
    Ok(())
}

fn accept_handle_signature(
    _: &[u8],
    _: &OperationHandlePreimageV1,
    _: &OperationHandleDigest,
    _: &CanonicalSignatureV1,
) -> Result<(), InvalidResidentRecordV1> {
    Ok(())
}

fn accept_checkpoint_signature(
    _: &[u8],
    _: &ControlStateCheckpointPreimageV1,
    _: &ControlStateCheckpointDigest,
    _: &CanonicalSignatureV1,
) -> Result<(), InvalidResidentRecordV1> {
    Ok(())
}

fn accept_observation_state(
    _: &OperationObservationStateV1,
) -> Result<(), InvalidResidentRecordV1> {
    Ok(())
}

fn accept_authority_rotation(
    _: &ResidentRecordRefV1,
    _: &ArtifactDigest,
    _: &ResidentRecordRefV1,
    _: &ArtifactDigest,
) -> Result<(), InvalidResidentRecordV1> {
    Ok(())
}

fn accept_observation_signature(
    _: &[u8],
    _: &OperationObservationPreimageV1,
    _: &OperationObservationDigest,
    _: &CanonicalSignatureV1,
) -> Result<(), InvalidResidentRecordV1> {
    Ok(())
}

#[must_use = "preserve a returned custody continuation"]
enum ResidentProcessInvokeStartError {
    Selection(ResidentProcessHostSelectionError),
    Preparation(crate::resident::LocalOperationStartError),
    Continuation(Box<ResidentProcessOperationStartContinuation>),
}

impl From<ResidentProcessHostSelectionError> for ResidentProcessInvokeStartError {
    fn from(reason: ResidentProcessHostSelectionError) -> Self {
        Self::Selection(reason)
    }
}

impl<'store, WriterGuard> ResidentProcessHost<'store, WriterGuard> {
    /// Exercise the former unallocated start seam only inside its hostile tests.
    #[allow(clippy::too_many_arguments)]
    fn start_invoke_test_only<
        Authority,
        CompiledAuthority,
        HandleSign,
        ObservationSign,
        CheckpointPreimage,
        CheckpointSign,
    >(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        input: ResidentLocalOperationStartInput<'_>,
        admit_compiled_authority: CompiledAuthority,
        sign_handle: HandleSign,
        sign_observation: ObservationSign,
        prepare_checkpoint_preimage: CheckpointPreimage,
        sign_checkpoint: CheckpointSign,
    ) -> Result<crate::resident::IssuedLocalOperation, ResidentProcessInvokeStartError>
    where
        Authority: AmbientSessionAuthorityPort,
        CompiledAuthority: FnOnce(
            &pale_core::records::CommonRequestEnvelopeV1,
            &crate::resident::AdmittedAmbientSession,
            &crate::resident::AmbientSessionAdmissionContext<'_>,
            &pale_core::records::OperationHandlePreimageV1,
            &pale_core::records::OperationLeaseV1,
            &crate::semantic::ResolvedOperation,
            &crate::resident::LocalOperationPublicationPolicyInput<'_>,
            &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        ) -> Result<(), InvalidResidentRecordV1>,
        HandleSign: FnOnce(
            &[u8],
            &pale_core::records::OperationHandlePreimageV1,
            &pale_core::records::OperationHandleDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointPreimage: FnOnce(
            &crate::resident::PreparedLocalOperationCheckpoint,
        ) -> Result<
            ControlStateCheckpointPreimageV1,
            crate::resident::LocalOperationStartError,
        >,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        self.validate_operation_start_associations(selector, access.store, &input)?;
        let selected = selector.clone();
        let maximum_record_bytes = input.maximum_record_bytes;
        let result = self
            .with_selected_operation_service(
                selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.start_invoke(
                        input,
                        admit_compiled_authority,
                        sign_handle,
                        sign_observation,
                        prepare_checkpoint_preimage,
                        sign_checkpoint,
                    )
                },
            )
            .expect("start deployment and access were selected before lending ports");
        result.map_err(|failure| match failure {
            ResidentLocalOperationStartFailure::Preparation(reason) => {
                ResidentProcessInvokeStartError::Preparation(reason)
            }
            failure => ResidentProcessInvokeStartError::Continuation(Box::new(
                ResidentProcessOperationStartContinuation {
                    owner_instance: self.operations.operation_task_owner_instance().clone(),
                    selector: selected,
                    maximum_record_bytes,
                    failure: Box::new(failure),
                },
            )),
        })
    }
}

fn host_for_start<'store>(
    selected_port: &'store MemoryAuthorityStore,
    other_port: &'store MemoryAuthorityStore,
    selector: &ResidentDeploymentAuthoritySelector,
    selected_store: &AuthorityStoreCoordinate,
    other_store: &AuthorityStoreCoordinate,
    retained_generation: crate::resident::AdmittedCompiledGeneration,
) -> ResidentProcessHost<'store, UnprovedHeldWriter> {
    host_for_start_retaining_generation(
        selected_port,
        other_port,
        selector,
        selected_store,
        other_store,
        Some(retained_generation),
    )
}

fn host_for_start_retaining_generation<'store>(
    selected_port: &'store MemoryAuthorityStore,
    other_port: &'store MemoryAuthorityStore,
    selector: &ResidentDeploymentAuthoritySelector,
    selected_store: &AuthorityStoreCoordinate,
    other_store: &AuthorityStoreCoordinate,
    retained_generation: Option<crate::resident::AdmittedCompiledGeneration>,
) -> ResidentProcessHost<'store, UnprovedHeldWriter> {
    let mut selected_application = ready_application(selected_port, selected_store.clone(), 1);
    if let Some(generation) = retained_generation {
        selected_application.fixture_retain_generation(generation);
    }
    let selected =
        unproved_deployment_for_selector(selector, selected_store.clone(), selected_application);
    let other = unproved_deployment(
        selector.principal_id.clone(),
        other_store,
        ready_application(other_port, other_store.clone(), 2),
    )
    .expect("other test deployment");
    ResidentProcessHost::new(
        unproved_writer(
            selector.principal_id.clone(),
            vec![selected_store.clone(), other_store.clone()],
        ),
        vec![other, selected],
        NonZeroUsize::new(2).expect("two deployment bound"),
        ResidentOperationHost::new(task_state()),
    )
    .expect("two exact start deployments")
}

pub(super) fn checkpoint_preimage(
    prepared: &crate::resident::PreparedLocalOperationCheckpoint,
    mut template: ControlStateCheckpointPreimageV1,
) -> Result<ControlStateCheckpointPreimageV1, crate::resident::LocalOperationStartError> {
    template.committed_transaction_sequence = prepared.transaction_sequence();
    template.issued_at = UnixTimeV1::new(61).expect("checkpoint time");
    template.checkpoint_profile_sha256 =
        crate::resident::SupportedOperationCheckpointProfile::content_digest()
            .expect("profile digest");
    template.state_root_sha256 = prepared.state_root().clone();
    template.unresolved_custody_root_sha256 = prepared.unresolved_root().clone();
    template.changed_leaf_keys = prepared.changed_keys().to_vec();
    template.leaf_count = prepared.leaf_count();
    Ok(template)
}

fn copy_semantic_start<'a>(
    source: &crate::resident::LocalOperationStartInput<'a>,
) -> crate::resident::LocalOperationStartInput<'a> {
    crate::resident::LocalOperationStartInput {
        checkpoint_profile: source.checkpoint_profile,
        request_ref: source.request_ref.clone(),
        request: source.request,
        handle_ref: source.handle_ref.clone(),
        handle: source.handle.clone(),
        context: source.context,
        session: source.session,
        current: source.current,
        generation: source.generation,
        publication_policy: crate::resident::LocalOperationPublicationPolicyInput {
            target_scope: source.publication_policy.target_scope,
            classification: source.publication_policy.classification,
            visibility: source.publication_policy.visibility,
            disclosure_ceiling: source.publication_policy.disclosure_ceiling,
            payload_resolution_policy: source.publication_policy.payload_resolution_policy,
            actor_provenance: source.publication_policy.actor_provenance,
        },
        checkpoint_authority: source.checkpoint_authority,
        observation_ref: source.observation_ref.clone(),
        observation_schema: source.observation_schema.clone(),
    }
}

fn invoke_start_input<'a>(
    semantic: crate::resident::LocalOperationStartInput<'a>,
    checkpoint_schema: &'a SchemaDigest,
    checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
) -> ResidentLocalOperationStartInput<'a> {
    ResidentLocalOperationStartInput {
        semantic,
        accumulator_limits: pale_ports::AccumulatorPreparationLimits::new(
            4, 1024, 1024, 512, 65_536,
        )
        .expect("bounded accumulator"),
        checkpoint: ResidentLocalOperationCheckpointInput {
            id: ControlCheckpointIdV1::parse("operation-checkpoint-1")
                .expect("checkpoint identity"),
            reference: local_operation_start_fixture::record_ref(
                "process-host/start/preflight-checkpoint",
            ),
            schema: checkpoint_schema,
            authority: checkpoint_authority,
        },
        maximum_record_bytes: 1 << 20,
    }
}

fn copy_resident_start_input<'a>(
    source: &ResidentLocalOperationStartInput<'a>,
    semantic: crate::resident::LocalOperationStartInput<'a>,
) -> ResidentLocalOperationStartInput<'a> {
    ResidentLocalOperationStartInput {
        semantic,
        accumulator_limits: source.accumulator_limits,
        checkpoint: ResidentLocalOperationCheckpointInput {
            id: source.checkpoint.id.clone(),
            reference: source.checkpoint.reference.clone(),
            schema: source.checkpoint.schema,
            authority: source.checkpoint.authority,
        },
        maximum_record_bytes: source.maximum_record_bytes,
    }
}

#[allow(clippy::too_many_arguments)]
fn attempt_start(
    host: &mut ResidentProcessHost<'_, UnprovedHeldWriter>,
    selector: &ResidentDeploymentAuthoritySelector,
    access: ResidentDeploymentOperationAccess<'_>,
    authority: &mut CountingAuthority,
    semantic: crate::resident::LocalOperationStartInput<'_>,
    checkpoint_authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    checkpoint_template: ControlStateCheckpointPreimageV1,
    checkpoint_ref: &str,
    owner_calls: &Cell<usize>,
    signatures: &Cell<usize>,
) -> Result<crate::resident::IssuedLocalOperation, ResidentProcessInvokeStartError> {
    let checkpoint_schema = checkpoint_template.schema_sha256.clone();
    let checkpoint_id = checkpoint_template.control_checkpoint_id.clone();
    host.start_invoke_test_only(
        selector,
        access,
        authority,
        ResidentLocalOperationStartInput {
            semantic,
            accumulator_limits: pale_ports::AccumulatorPreparationLimits::new(
                4, 1024, 1024, 512, 65_536,
            )
            .expect("bounded accumulator"),
            checkpoint: ResidentLocalOperationCheckpointInput {
                id: checkpoint_id,
                reference: local_operation_start_fixture::record_ref(checkpoint_ref),
                schema: &checkpoint_schema,
                authority: checkpoint_authority,
            },
            maximum_record_bytes: 1 << 20,
        },
        |_, _, _, _, _, _, _, _| {
            owner_calls.set(owner_calls.get() + 1);
            Ok(())
        },
        |_, _, _| {
            signatures.set(signatures.get() + 1);
            Ok(local_operation_start_fixture::signature())
        },
        |_, _, _| {
            signatures.set(signatures.get() + 1);
            Ok(local_operation_start_fixture::signature())
        },
        move |prepared| checkpoint_preimage(prepared, checkpoint_template),
        |_, _, _| {
            signatures.set(signatures.get() + 1);
            Ok(local_operation_start_fixture::signature())
        },
    )
}

fn assert_no_start_io(
    authority: &CountingAuthority,
    owner_calls: &Cell<usize>,
    signatures: &Cell<usize>,
    custody: &RecordingStore,
    anchor: &RecordingAnchor,
) {
    assert_eq!(authority.calls, 0);
    assert_eq!(owner_calls.get(), 0);
    assert_eq!(signatures.get(), 0);
    assert_eq!(custody.mutation_counts(), (0, 0));
    assert_eq!(anchor.access_counts(), (0, 0));
}

fn source_generation_with_host_copy() -> (
    crate::resident::AdmittedCompiledGeneration,
    pale_core::records::EffectiveProjectProfileV1,
    crate::resident::AdmittedCompiledGeneration,
) {
    let (mut generations, profile) = source_generation_copies(2);
    let host_generation = generations.pop().expect("retained host generation");
    let generation = generations.pop().expect("fixture generation");
    (generation, profile, host_generation)
}

fn assert_pairwise_delegate_child_start(parent_class: OperationClassV1, interrupt_anchor: bool) {
    let (mut generations, profile) = source_generation_copies(2);
    let retained_generation = generations.pop().expect("retained host generation");
    let generation = generations.pop().expect("fixture generation");
    let selected_store = crate::local_operation_start_fixture::store_coordinate();
    crate::local_operation_start_fixture::with_operation_start_input_at_store(
        &generation,
        &profile,
        &format!("delegate-parent-{parent_class:?}"),
        |_| {},
        if parent_class == OperationClassV1::Delegate {
            OperationClassV1::Invoke
        } else {
            parent_class
        },
        &selected_store,
        |semantic, request, checkpoint_template, parents| {
            let other_store = store("delegate-other-store", "delegate-other-incarnation");
            let selector = selector_from_signing_authority(parents.signing);
            let selected_port = MemoryAuthorityStore::default();
            let other_port = MemoryAuthorityStore::default();
            let mut host = host_for_start_retaining_generation(
                &selected_port,
                &other_port,
                &selector,
                &selected_store,
                &other_store,
                Some(retained_generation),
            );
            let mut parent_custody = absent_start_store(request)
                .with_preparation(
                    initial_response,
                    ControlTransactionSequenceV1::new(1).expect("initial transaction"),
                )
                .with_lease_retention(Ok(pale_ports::OperationCustodyCommitOutcome::Committed));
            let mut parent_anchor = initial_anchor();
            let mut authority = CountingAuthority::default();
            let checkpoint_schema = checkpoint_template.schema_sha256.clone();
            let checkpoint_authority = semantic.checkpoint_authority;
            let retained_checkpoint = Rc::new(RefCell::new(None));
            let checkpoint_capture = Rc::clone(&retained_checkpoint);
            assert_eq!(
                &semantic.request.session_admission().session_admission_id,
                semantic.session.lease().session_admission_id(),
                "fixture request must name its admitted session",
            );
            assert_eq!(
                &semantic
                    .request
                    .session_admission()
                    .session_admission_lease_digest,
                semantic.session.lease().session_admission_lease_digest(),
                "fixture request must bind the admitted session lease",
            );
            assert!(
                semantic
                    .generation
                    .operation(
                        semantic.session.lease().repository_id(),
                        semantic.request.operation(),
                    )
                    .is_some(),
                "fixture request operation must exist in its admitted generation",
            );
            let parent_input =
                invoke_start_input(semantic, &checkpoint_schema, checkpoint_authority);
            let parent = if parent_class == OperationClassV1::Consult {
                host.start_consult(
                    &selector,
                    ResidentDeploymentOperationAccess {
                        store: &selected_store,
                        custody: &mut parent_custody,
                        sessions: &selected_port,
                        anchor: &mut parent_anchor,
                    },
                    &mut authority,
                    parent_input,
                    None,
                    |_, _, _, _, _, _, _, _| Ok(()),
                    |_, _, _| Ok(local_operation_start_fixture::signature()),
                    |_, _, _| Ok(local_operation_start_fixture::signature()),
                    move |prepared| {
                        let preimage = checkpoint_preimage(prepared, checkpoint_template)?;
                        checkpoint_capture.replace(Some(preimage.clone()));
                        Ok(preimage)
                    },
                    |_, _, _| Ok(local_operation_start_fixture::signature()),
                )
                .unwrap_or_else(|_| panic!("Consult parent reaches durable custody"))
            } else {
                host.start_invoke_test_only(
                    &selector,
                    ResidentDeploymentOperationAccess {
                        store: &selected_store,
                        custody: &mut parent_custody,
                        sessions: &selected_port,
                        anchor: &mut parent_anchor,
                    },
                    &mut authority,
                    parent_input,
                    |_, _, _, _, _, _, _, _| Ok(()),
                    |_, _, _| Ok(local_operation_start_fixture::signature()),
                    |_, _, _| Ok(local_operation_start_fixture::signature()),
                    move |prepared| {
                        let preimage = checkpoint_preimage(prepared, checkpoint_template)?;
                        checkpoint_capture.replace(Some(preimage.clone()));
                        Ok(preimage)
                    },
                    |_, _, _| Ok(local_operation_start_fixture::signature()),
                )
                .unwrap_or_else(|error| match error {
                    ResidentProcessInvokeStartError::Selection(reason) => {
                        panic!("Invoke parent selection failed: {reason:?}")
                    }
                    ResidentProcessInvokeStartError::Preparation(_) => {
                        panic!("Invoke parent preparation failed")
                    }
                    ResidentProcessInvokeStartError::Continuation(_) => {
                        panic!("Invoke parent custody failed")
                    }
                })
            };
            let parent_checkpoint_preimage = retained_checkpoint
                .borrow()
                .clone()
                .expect("parent checkpoint was signed");
            let parent_checkpoint =
                crate::resident::operation_observation_custody::tests::admit_initial_checkpoint(
                    &parent_checkpoint_preimage,
                );
            let parent_checkpoint_ref = local_operation_start_fixture::record_ref(
                "process-host/start/preflight-checkpoint",
            );
            let (parent, parent_request, parent_checkpoint_ref, parent_checkpoint) =
                if parent_class == OperationClassV1::Delegate {
                    let issued = issue_test_child_with_evidence(
                        &parent,
                        request,
                        (&parent_checkpoint_ref, &parent_checkpoint),
                        &generation,
                        &parents,
                        OperationClassV1::Delegate,
                        "pairwise-parent",
                    );
                    (
                        issued.operation,
                        issued.request,
                        issued.checkpoint_ref,
                        issued.checkpoint,
                    )
                } else {
                    (
                        parent,
                        request.clone(),
                        parent_checkpoint_ref,
                        parent_checkpoint,
                    )
                };
            let child_owner_calls = Cell::new(0usize);
            let child_signatures = Cell::new(0usize);
            with_test_child_start_input(
                parent.handle(),
                parent.historical_operation_lease(),
                Some((&parent_checkpoint_ref, &parent_checkpoint)),
                &parent_request,
                &generation,
                &parents,
                OperationClassV1::Delegate,
                &format!("pairwise-child-{parent_class:?}"),
                |input, _| {
                    assert!(matches!(
                        crate::resident::consultation::ConsultationFreshChildPlan::capture(&input),
                        Err(crate::resident::consultation::ConsultationStartIntentError::Child)
                    ));
                    let child_request = input.semantic.request;
                    let child_checkpoint_id = input.checkpoint.id.clone();
                    let sequence = parent_checkpoint.domain_sequence().get() + 1;
                    let protected_parent = || {
                        PreparedLocalChildParent::fixture(
                            &parent,
                            (&parent_checkpoint_ref, &parent_checkpoint),
                            ControlTransactionPreparation::new(
                                selected_store.clone(),
                                ControlTransactionSequenceV1::new(sequence)
                                    .expect("child transaction sequence"),
                            )
                            .expect("selected child transaction"),
                            parents.current.now,
                        )
                    };
                    {
                        let preflight_signatures = Cell::new(0usize);
                        let mut preflight_custody =
                            absent_start_store(child_request).with_unlisted_indexes_absent();
                        let mut preflight_anchor = crate::resident::operation_observation_custody::tests::local_start::successor_anchor();
                        let attempt =
                            |host: &mut ResidentProcessHost<'_, UnprovedHeldWriter>,
                             selector: &ResidentDeploymentAuthoritySelector,
                             store: &AuthorityStoreCoordinate,
                             semantic: crate::resident::LocalOperationStartInput<'_>,
                             authority: &mut CountingAuthority,
                             custody: &mut RecordingStore,
                             anchor: &mut RecordingAnchor| {
                                host.start_delegate_child(
                                    selector,
                                    ResidentDeploymentOperationAccess {
                                        store,
                                        custody,
                                        sessions: &selected_port,
                                        anchor,
                                    },
                                    authority,
                                    copy_resident_start_input(&input, semantic),
                                    protected_parent(),
                                    |_, _, _, _, _, _, _, _| {
                                        preflight_signatures.set(preflight_signatures.get() + 1);
                                        Ok(())
                                    },
                                    |_, _, _| {
                                        preflight_signatures.set(preflight_signatures.get() + 1);
                                        Ok(local_operation_start_fixture::signature())
                                    },
                                    |_, _, _| {
                                        preflight_signatures.set(preflight_signatures.get() + 1);
                                        Ok(local_operation_start_fixture::signature())
                                    },
                                    |_| panic!("hostile preflight cannot prepare a checkpoint"),
                                    |_, _, _| panic!("hostile preflight cannot sign a checkpoint"),
                                )
                            };

                        let mut wrong_class = copy_semantic_start(&input.semantic);
                        wrong_class.handle.operation_class = OperationClassV1::Invoke;
                        assert!(matches!(
                            attempt(
                                &mut host,
                                &selector,
                                &selected_store,
                                wrong_class,
                                &mut authority,
                                &mut preflight_custody,
                                &mut preflight_anchor,
                            ),
                            Err(ResidentProcessDelegateStartError::Selection(
                                ResidentProcessHostSelectionError::DelegateChildAssociation
                            ))
                        ));

                        let mut wrong_parent = copy_semantic_start(&input.semantic);
                        wrong_parent
                            .handle
                            .parent_operation
                            .as_mut()
                            .expect("child parent")
                            .operation_handle_ref =
                            local_operation_start_fixture::record_ref("other-parent");
                        assert!(matches!(
                            attempt(
                                &mut host,
                                &selector,
                                &selected_store,
                                wrong_parent,
                                &mut authority,
                                &mut preflight_custody,
                                &mut preflight_anchor,
                            ),
                            Err(ResidentProcessDelegateStartError::Selection(
                                ResidentProcessHostSelectionError::DelegateChildAssociation
                            ))
                        ));

                        let mut foreign_selector = selector.clone();
                        foreign_selector.runtime_authority_ref =
                            local_operation_start_fixture::record_ref("foreign-runtime-authority");
                        assert!(matches!(
                            attempt(
                                &mut host,
                                &foreign_selector,
                                &selected_store,
                                copy_semantic_start(&input.semantic),
                                &mut authority,
                                &mut preflight_custody,
                                &mut preflight_anchor,
                            ),
                            Err(ResidentProcessDelegateStartError::Selection(_))
                        ));
                        assert!(matches!(
                            attempt(
                                &mut host,
                                &selector,
                                &other_store,
                                copy_semantic_start(&input.semantic),
                                &mut authority,
                                &mut preflight_custody,
                                &mut preflight_anchor,
                            ),
                            Err(ResidentProcessDelegateStartError::Selection(
                                ResidentProcessHostSelectionError::AccessStoreAssociation
                            ))
                        ));
                        assert_eq!(preflight_signatures.get(), 0);
                        assert_eq!(preflight_custody.mutation_counts(), (0, 0));
                        assert_eq!(preflight_anchor.access_counts(), (0, 0));
                    }
                    let protected = protected_parent();
                    let mut child_custody = absent_start_store(child_request)
                        .with_preparation(
                            prepare_collection_accumulator,
                            ControlTransactionSequenceV1::new(sequence)
                                .expect("child transaction sequence"),
                        )
                        .with_lease_retention(Ok(
                            pale_ports::OperationCustodyCommitOutcome::Committed,
                        ))
                        .with_unlisted_indexes_absent();
                    let mut child_anchor = if interrupt_anchor {
                        RecordingAnchor::unavailable_after_preflight()
                    } else {
                        crate::resident::operation_observation_custody::tests::local_start::successor_anchor()
                    };
                    let mut child_checkpoint_template =
                        local_operation_start_fixture::checkpoint_preimage(
                            sequence,
                            Some((
                                parent_checkpoint_ref.clone(),
                                parent_checkpoint
                                    .canonical_control_checkpoint_sha256()
                                    .clone(),
                            )),
                        );
                    child_checkpoint_template.control_checkpoint_id = child_checkpoint_id;
                    child_checkpoint_template.checkpoint_domain_id =
                        parent_checkpoint.checkpoint_domain_id().clone();
                    child_checkpoint_template.checkpoint_profile_ref =
                        parent_checkpoint.checkpoint_profile_ref().clone();
                    child_checkpoint_template.checkpoint_profile_sha256 =
                        parent_checkpoint.checkpoint_profile_sha256().clone();
                    child_checkpoint_template.domain_epoch = *parent_checkpoint.domain_epoch();
                    let child_checkpoint_issued_at =
                        UnixTimeV1::new(parent_checkpoint.issued_at().get() + 1)
                            .expect("successor checkpoint time");
                    let start =
                        |host: &mut ResidentProcessHost<'_, UnprovedHeldWriter>,
                         admitted_request: &CommonRequestEnvelopeV1| {
                            let mut semantic = copy_semantic_start(&input.semantic);
                            semantic.request = admitted_request;
                            let result = host.start_delegate_child(
                            &selector,
                            ResidentDeploymentOperationAccess {
                                store: &selected_store,
                                custody: &mut child_custody,
                                sessions: &selected_port,
                                anchor: &mut child_anchor,
                            },
                            &mut authority,
                            copy_resident_start_input(&input, semantic),
                            protected,
                            |_, _, _, _, _, _, _, _| {
                                child_owner_calls.set(child_owner_calls.get() + 1);
                                Ok(())
                            },
                            |_, _, _| {
                                child_signatures.set(child_signatures.get() + 1);
                                Ok(local_operation_start_fixture::signature())
                            },
                            |_, _, _| {
                                child_signatures.set(child_signatures.get() + 1);
                                Ok(local_operation_start_fixture::signature())
                            },
                            move |prepared| {
                                child_checkpoint_template.committed_transaction_sequence =
                                    prepared.transaction_sequence();
                                child_checkpoint_template.issued_at = child_checkpoint_issued_at;
                                child_checkpoint_template.checkpoint_profile_sha256 =
                                    crate::resident::SupportedOperationCheckpointProfile::content_digest()
                                        .expect("checkpoint profile digest");
                                child_checkpoint_template.state_root_sha256 =
                                    prepared.state_root().clone();
                                child_checkpoint_template.unresolved_custody_root_sha256 =
                                    prepared.unresolved_root().clone();
                                child_checkpoint_template.changed_leaf_keys =
                                    prepared.changed_keys().to_vec();
                                child_checkpoint_template.leaf_count = prepared.leaf_count();
                                Ok(child_checkpoint_template)
                            },
                            |_, _, _| {
                                child_signatures.set(child_signatures.get() + 1);
                                Ok(local_operation_start_fixture::signature())
                            },
                        );
                            let records = result.as_ref().ok().map(|operation| {
                                start_records(
                                    operation,
                                    child_custody
                                        .committed_candidate()
                                        .expect("Delegate child reached durable custody"),
                                )
                            });
                            (result, records)
                        };
                    if !interrupt_anchor {
                        let parent_locator = pale_ports::ResidentOperationFrameLocator::new(
                            parent.handle().0.as_str().to_owned(),
                            parent
                                .handle()
                                .1
                                .canonical_operation_handle_sha256()
                                .as_str()
                                .to_owned(),
                        );
                        let substituted_parent = pale_ports::ResidentOperationFrameLocator::new(
                            "pale://resident/operations/substituted-parent".to_owned(),
                            parent_locator.operation_handle_sha256().to_owned(),
                        );
                        let substituted_frame = ResidentOperationFrameRequest::StartDelegate {
                            parent: substituted_parent,
                            signed_common_request: pale_core::canon::to_canonical_string(
                                child_request,
                            )
                            .expect("signed Delegate request is canonical"),
                        };
                        let mut substituted_port = OperationFramePort::new(&substituted_frame);
                        let deadline = Instant::now()
                            .checked_add(std::time::Duration::from_secs(2))
                            .expect("test deadline");
                        let mut request_host =
                            ResidentOperationRequestHost::new(&mut substituted_port, deadline);
                        let received = request_host
                            .receive()
                            .expect("substituted Delegate frame is canonical");
                        let error = operation_request_host::request_host_test_support::answer(
                            &mut request_host,
                            received,
                            &mut host,
                            ResidentOperationRequestResolution::child_start(
                                child_request,
                                parent_locator,
                            ),
                            &mut |_, actual| {
                                assert_eq!(actual, &substituted_frame);
                                Ok(())
                            },
                            deadline,
                            |_, _| panic!("substitution reached the Delegate owner"),
                        )
                        .expect_err("a substituted parent cannot select the Delegate owner");
                        assert!(matches!(
                            error,
                            ResidentOperationRequestHostError::ResolutionAssociation
                        ));
                        assert!(substituted_port.sent.is_empty());
                    }
                    let result = if interrupt_anchor {
                        start(&mut host, child_request).0
                    } else {
                        let parent_locator = pale_ports::ResidentOperationFrameLocator::new(
                            parent.handle().0.as_str().to_owned(),
                            parent
                                .handle()
                                .1
                                .canonical_operation_handle_sha256()
                                .as_str()
                                .to_owned(),
                        );
                        let request_frame = ResidentOperationFrameRequest::StartDelegate {
                            parent: parent_locator.clone(),
                            signed_common_request: pale_core::canon::to_canonical_string(
                                child_request,
                            )
                            .expect("signed Delegate request is canonical"),
                        };
                        let mut frame_port = OperationFramePort::new(&request_frame);
                        let deadline = Instant::now()
                            .checked_add(std::time::Duration::from_secs(2))
                            .expect("test deadline");
                        let mut request_host =
                            ResidentOperationRequestHost::new(&mut frame_port, deadline);
                        let received = request_host.receive().expect("Delegate frame is exact");
                        let issued = RefCell::new(None);
                        let response = operation_request_host::request_host_test_support::answer(
                            &mut request_host,
                            received,
                            &mut host,
                            ResidentOperationRequestResolution::child_start(
                                child_request,
                                parent_locator.clone(),
                            ),
                            &mut |_, actual| {
                                assert_eq!(actual, &request_frame);
                                Ok(())
                            },
                            deadline,
                            |host, admitted_request| {
                                assert!(std::ptr::eq(admitted_request, child_request));
                                let (operation, records) = start(host, admitted_request);
                                let operation = operation.map_err(|_| {
                                    pale_ports::ResidentOperationFrameFailure::ResidentUnavailable
                                })?;
                                let records = records.expect("durable Delegate has records");
                                issued.replace(Some(operation));
                                Ok(ResidentOperationRequestOutcome::Started(records))
                            },
                        )
                        .expect("private request host returns the real Delegate start");
                        assert!(matches!(
                            response,
                            pale_ports::ResidentOperationFrameResponse::Started { .. }
                        ));
                        Ok(issued
                            .into_inner()
                            .expect("real Delegate start survives the transport projection"))
                    };
                    let issued = if interrupt_anchor {
                        let pending = match result {
                            Err(ResidentProcessDelegateStartError::Continuation(pending)) => {
                                pending
                            }
                            _ => panic!("anchor interruption must retain exact start custody"),
                        };
                        assert_eq!(child_custody.mutation_counts(), (1, 0));
                        assert_eq!(child_anchor.access_counts(), (1, 1));
                        let signatures_before_retry = child_signatures.get();
                        let owners_before_retry = child_owner_calls.get();
                        let mut retry_anchor = crate::resident::operation_observation_custody::tests::local_start::successor_anchor();
                        let issued = host
                            .continue_operation_start(
                                *pending,
                                ResidentDeploymentOperationAccess {
                                    store: &selected_store,
                                    custody: &mut child_custody,
                                    sessions: &selected_port,
                                    anchor: &mut retry_anchor,
                                },
                                &mut authority,
                            )
                            .unwrap_or_else(|_| {
                                panic!("exact custody continuation reaches the anchor")
                            });
                        assert_eq!(child_signatures.get(), signatures_before_retry);
                        assert_eq!(child_owner_calls.get(), owners_before_retry);
                        issued
                    } else {
                        match result {
                            Ok(issued) => issued,
                            Err(ResidentProcessDelegateStartError::Selection(reason)) => {
                                panic!("Delegate selection failed: {reason:?}")
                            }
                            Err(ResidentProcessDelegateStartError::Preparation(reason)) => {
                                panic!("Delegate preparation failed: {reason:?}")
                            }
                            Err(ResidentProcessDelegateStartError::Continuation(pending)) => {
                                panic!("Delegate custody failed: {:?}", pending.failure)
                            }
                        }
                    };
                    assert_eq!(
                        issued.handle().1.operation_class(),
                        &OperationClassV1::Delegate
                    );
                    let child_parent = issued
                        .handle()
                        .1
                        .parent_operation()
                        .as_ref()
                        .expect("Delegate retains its exact parent");
                    assert_eq!(child_parent.operation_handle_ref, *parent.handle().0);
                    assert_eq!(
                        child_parent.operation_handle_id,
                        *parent.handle().1.operation_handle_id()
                    );
                    assert_eq!(
                        child_parent.operation_handle_sha256,
                        *parent.handle().1.canonical_operation_handle_sha256()
                    );
                    assert_eq!(
                        issued.handle().1.compiled_generation_sha256(),
                        parent.handle().1.compiled_generation_sha256()
                    );
                    assert_eq!(child_owner_calls.get(), 1);
                    assert_eq!(child_signatures.get(), 3);
                    assert_eq!(host.operations.active_consultation_batches(), 0);
                    assert_eq!(host.operations.in_flight_consultation_tasks(), 0);
                },
            );
        },
    );
}

#[test]
fn pairwise_delegate_child_starts_under_each_active_parent_class() {
    assert_pairwise_delegate_child_start(OperationClassV1::Invoke, false);
    assert_pairwise_delegate_child_start(OperationClassV1::Consult, true);
    assert_pairwise_delegate_child_start(OperationClassV1::Delegate, false);
}

#[test]
fn existing_delegate_rejects_a_different_parent_and_substituted_parent_lease() {
    let (generation, profile) = source_generation();
    with_issued_context(
        &generation,
        &profile,
        "existing-delegate-parent-binding",
        |_| {},
        crate::session_admission_fixture::LocalStartContext::Inherited,
        |parent, parent_request, parents, parent_checkpoint, _, parent_candidate| {
            let parent_checkpoint_ref = parent_candidate
                .checkpoint()
                .successor_record()
                .coordinate()
                .record_ref()
                .clone();
            let child = issue_test_child_with_evidence(
                parent,
                parent_request,
                (&parent_checkpoint_ref, parent_checkpoint),
                &generation,
                &parents,
                OperationClassV1::Delegate,
                "existing-delegate-child",
            );
            let other_parent = issue_test_child_with_evidence(
                parent,
                parent_request,
                (&parent_checkpoint_ref, parent_checkpoint),
                &generation,
                &parents,
                OperationClassV1::Invoke,
                "existing-delegate-other-parent",
            );
            let store = crate::local_operation_start_fixture::store_coordinate();
            let selector = selector_from_signing_authority(parents.signing);
            let wrong_parent = ResidentProcessProtectedDelegateParent {
                selector: selector.clone(),
                store: store.clone(),
                lease: other_parent.operation.historical_operation_lease().clone(),
                parent: PreparedLocalChildParent::fixture(
                    &other_parent.operation,
                    (&other_parent.checkpoint_ref, &other_parent.checkpoint),
                    ControlTransactionPreparation::new(
                        store.clone(),
                        ControlTransactionSequenceV1::new(
                            other_parent.checkpoint.domain_sequence().get() + 1,
                        )
                        .expect("other-parent successor sequence"),
                    )
                    .expect("other-parent transaction"),
                    parents.current.now,
                ),
            };
            let actual_parent = child
                .operation
                .handle()
                .1
                .parent_operation()
                .as_ref()
                .expect("Delegate child retains its parent");
            assert_ne!(
                actual_parent.operation_handle_ref,
                *other_parent.operation.handle().0
            );
            assert_ne!(
                actual_parent.operation_handle_sha256,
                *other_parent
                    .operation
                    .handle()
                    .1
                    .canonical_operation_handle_sha256()
            );
            assert!(matches!(
                admit_existing_delegate_operation(&child.operation, &wrong_parent),
                Err(ResidentProcessOperationRestartError::State)
            ));

            let substituted_lease = ResidentProcessProtectedDelegateParent {
                selector,
                store: store.clone(),
                lease: other_parent.operation.historical_operation_lease().clone(),
                parent: PreparedLocalChildParent::fixture(
                    parent,
                    (&parent_checkpoint_ref, parent_checkpoint),
                    ControlTransactionPreparation::new(
                        store,
                        ControlTransactionSequenceV1::new(
                            parent_checkpoint.domain_sequence().get() + 1,
                        )
                        .expect("parent successor sequence"),
                    )
                    .expect("parent transaction"),
                    parents.current.now,
                ),
            };
            assert!(matches!(
                admit_existing_delegate_operation(&child.operation, &substituted_lease),
                Err(ResidentProcessOperationRestartError::State)
            ));
        },
    );
}

#[test]
fn existing_consult_rejects_an_invoke_handle_before_replay() {
    let (generation, profile) = source_generation();
    with_issued_context(
        &generation,
        &profile,
        "existing-consult-wrong-class",
        |_| {},
        crate::session_admission_fixture::LocalStartContext::Inherited,
        |invoke, _, _, _, _, _| {
            assert_eq!(
                invoke.handle().1.operation_class(),
                &OperationClassV1::Invoke
            );
            assert!(matches!(
                admit_existing_root_operation(invoke, OperationClassV1::Consult),
                Err(ResidentProcessOperationRestartError::State)
            ));
        },
    );
}

fn assert_selector_substitution_stops_before_start_io(
    suffix: &str,
    substitute: impl FnOnce(&mut ResidentDeploymentAuthoritySelector),
) {
    let (generation, profile, host_generation) = source_generation_with_host_copy();
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        suffix,
        |_| {},
        |semantic, request, checkpoint_template, parents| {
            let selected_store = crate::local_operation_start_fixture::store_coordinate();
            let other_store = store("other-start-store", "other-start-incarnation");
            let selector = selector_from_signing_authority(parents.signing);
            let selected_port = MemoryAuthorityStore::default();
            let other_port = MemoryAuthorityStore::default();
            let mut host = host_for_start(
                &selected_port,
                &other_port,
                &selector,
                &selected_store,
                &other_store,
                host_generation,
            );
            let mut custody = absent_start_store(request).with_preparation(
                initial_response,
                pale_core::records::ControlTransactionSequenceV1::new(1)
                    .expect("initial transaction"),
            );
            let mut anchor = initial_anchor();
            let mut authority = CountingAuthority::default();
            let owner_calls = Cell::new(0);
            let signatures = Cell::new(0);
            let checkpoint_authority = semantic.checkpoint_authority;
            let mut substituted = selector.clone();
            substitute(&mut substituted);
            let result = attempt_start(
                &mut host,
                &substituted,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut custody,
                    sessions: &selected_port,
                    anchor: &mut anchor,
                },
                &mut authority,
                semantic,
                checkpoint_authority,
                checkpoint_template,
                "process-host/start/selector-fence-checkpoint",
                &owner_calls,
                &signatures,
            );
            assert!(matches!(
                result,
                Err(ResidentProcessInvokeStartError::Selection(
                    ResidentProcessHostSelectionError::UnknownDeploymentAuthority
                ))
            ));
            assert_no_start_io(&authority, &owner_calls, &signatures, &custody, &anchor);
        },
    );
}

fn assert_handle_issuer_substitution_stops_before_start_io(
    suffix: &str,
    substitute: impl FnOnce(&mut pale_core::records::OperationHandlePreimageV1),
) {
    let (generation, profile, host_generation) = source_generation_with_host_copy();
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        suffix,
        |_| {},
        |mut semantic, request, checkpoint_template, parents| {
            let selected_store = crate::local_operation_start_fixture::store_coordinate();
            let other_store = store("other-start-store", "other-start-incarnation");
            let selector = selector_from_signing_authority(parents.signing);
            let selected_port = MemoryAuthorityStore::default();
            let other_port = MemoryAuthorityStore::default();
            let mut host = host_for_start(
                &selected_port,
                &other_port,
                &selector,
                &selected_store,
                &other_store,
                host_generation,
            );
            let mut custody = absent_start_store(request).with_preparation(
                initial_response,
                pale_core::records::ControlTransactionSequenceV1::new(1)
                    .expect("initial transaction"),
            );
            let mut anchor = initial_anchor();
            let mut authority = CountingAuthority::default();
            let owner_calls = Cell::new(0);
            let signatures = Cell::new(0);
            let checkpoint_authority = semantic.checkpoint_authority;
            substitute(&mut semantic.handle);

            let result = attempt_start(
                &mut host,
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut custody,
                    sessions: &selected_port,
                    anchor: &mut anchor,
                },
                &mut authority,
                semantic,
                checkpoint_authority,
                checkpoint_template,
                "process-host/start/handle-issuer-checkpoint",
                &owner_calls,
                &signatures,
            );

            assert!(matches!(
                result,
                Err(ResidentProcessInvokeStartError::Selection(
                    ResidentProcessHostSelectionError::StartHandleSigner
                ))
            ));
            assert_no_start_io(&authority, &owner_calls, &signatures, &custody, &anchor);
            assert_eq!(selected_port.entry_count(), 0);
            assert_eq!(other_port.entry_count(), 0);
            assert_eq!(host.provider_routes.reserved, 0);
            assert_eq!(host.provider_routes.checked_out, 0);
            assert!(host.provider_routes.active.is_empty());
        },
    );
}

#[test]
fn exact_process_host_start_reaches_database_and_anchor_before_exposure() {
    let (generation, profile, host_generation) = source_generation_with_host_copy();
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        "process-host-start",
        |_| {},
        |semantic, request, checkpoint_template, parents| {
            let selected_store = crate::local_operation_start_fixture::store_coordinate();
            let other_store = store("other-start-store", "other-start-incarnation");
            let selector = selector_from_signing_authority(parents.signing);
            let selected_port = MemoryAuthorityStore::default();
            let other_port = MemoryAuthorityStore::default();
            let mut host = host_for_start(
                &selected_port,
                &other_port,
                &selector,
                &selected_store,
                &other_store,
                host_generation,
            );
            let mut custody = absent_start_store(request).with_preparation(
                initial_response,
                pale_core::records::ControlTransactionSequenceV1::new(1)
                    .expect("initial transaction"),
            );
            let mut anchor = initial_anchor();
            let mut authority = CountingAuthority::default();
            let owner_calls = Cell::new(0);
            let signatures = Cell::new(0);
            let checkpoint_schema = checkpoint_template.schema_sha256.clone();
            let checkpoint_authority = semantic.checkpoint_authority;
            let substituted_frame = ResidentOperationFrameRequest::StartInvoke {
                signed_common_request: "{}".to_owned(),
            };
            let mut substituted_port = OperationFramePort::new(&substituted_frame);
            let deadline = Instant::now()
                .checked_add(std::time::Duration::from_secs(2))
                .expect("test deadline");
            let mut substituted_host =
                ResidentOperationRequestHost::new(&mut substituted_port, deadline);
            let received = substituted_host
                .receive()
                .expect("substituted request frame is canonical");
            let error = operation_request_host::request_host_test_support::answer(
                &mut substituted_host,
                received,
                &mut host,
                ResidentOperationRequestResolution::root_start(
                    ResidentOperationRequestKind::StartInvoke,
                    request,
                ),
                &mut |_, actual| {
                    assert_eq!(actual, &substituted_frame);
                    Ok(())
                },
                deadline,
                |_, _| panic!("substitution reached the Invoke owner"),
            )
            .expect_err("substituted request bytes cannot reach the Invoke owner");
            assert!(matches!(
                error,
                ResidentOperationRequestHostError::ResolutionAssociation
            ));
            assert!(substituted_port.sent.is_empty());
            let request_frame = ResidentOperationFrameRequest::StartInvoke {
                signed_common_request: pale_core::canon::to_canonical_string(request)
                    .expect("signed Invoke request is canonical"),
            };
            let mut frame_port = OperationFramePort::new(&request_frame);
            let deadline = Instant::now()
                .checked_add(std::time::Duration::from_secs(2))
                .expect("test deadline");
            let mut request_host = ResidentOperationRequestHost::new(&mut frame_port, deadline);
            let received = request_host.receive().expect("Invoke frame is exact");
            let issued = RefCell::new(None);
            let response = operation_request_host::request_host_test_support::answer(
                &mut request_host,
                received,
                &mut host,
                ResidentOperationRequestResolution::root_start(
                    ResidentOperationRequestKind::StartInvoke,
                    request,
                ),
                &mut |peer, actual| {
                    assert_eq!(peer, LocalIpcPeerCredentials::new(501, 20, Some(8_081)));
                    assert_eq!(actual, &request_frame);
                    Ok(())
                },
                deadline,
                |host, admitted_request| {
                    assert!(std::ptr::eq(admitted_request, request));
                    let mut semantic = semantic;
                    semantic.request = admitted_request;
                    let started = host
                        .start_invoke_test_only(
                            &selector,
                            ResidentDeploymentOperationAccess {
                                store: &selected_store,
                                custody: &mut custody,
                                sessions: &selected_port,
                                anchor: &mut anchor,
                            },
                            &mut authority,
                            ResidentLocalOperationStartInput {
                                semantic,
                                accumulator_limits: pale_ports::AccumulatorPreparationLimits::new(
                                    4, 1024, 1024, 512, 65_536,
                                )
                                .expect("bounded accumulator"),
                                checkpoint: ResidentLocalOperationCheckpointInput {
                                    id: checkpoint_template.control_checkpoint_id.clone(),
                                    reference: local_operation_start_fixture::record_ref(
                                        "process-host/start/checkpoint",
                                    ),
                                    schema: &checkpoint_schema,
                                    authority: checkpoint_authority,
                                },
                                maximum_record_bytes: 1 << 20,
                            },
                            |_, _, _, _, _, _, _, _| {
                                owner_calls.set(owner_calls.get() + 1);
                                Ok(())
                            },
                            |_, _, _| {
                                signatures.set(signatures.get() + 1);
                                Ok(local_operation_start_fixture::signature())
                            },
                            |_, _, _| {
                                signatures.set(signatures.get() + 1);
                                Ok(local_operation_start_fixture::signature())
                            },
                            move |prepared| checkpoint_preimage(prepared, checkpoint_template),
                            |_, _, _| {
                                signatures.set(signatures.get() + 1);
                                Ok(local_operation_start_fixture::signature())
                            },
                        )
                        .map_err(|_| {
                            pale_ports::ResidentOperationFrameFailure::ResidentUnavailable
                        })?;
                    let records = start_records(
                        &started,
                        custody
                            .committed_candidate()
                            .expect("Invoke reached durable custody before response"),
                    );
                    issued.replace(Some(started));
                    Ok(ResidentOperationRequestOutcome::Started(records))
                },
            )
            .expect("private request host returns the real durable Invoke start");
            assert!(matches!(
                response,
                pale_ports::ResidentOperationFrameResponse::Started { .. }
            ));
            let issued = issued
                .into_inner()
                .expect("durable Invoke survives the transport projection");

            assert_eq!(issued.handle().0.as_str(), "handle/process-host-start");
            let retained_handle = custody
                .committed_candidate()
                .expect("committed start candidate")
                .records()
                .iter()
                .find_map(|record| match record {
                    OperationCustodyRecord::OperationHandle(record) => Some(record),
                    _ => None,
                })
                .expect("retained operation handle");
            assert_eq!(
                retained_handle.canonical_record_jcs().as_slice(),
                pale_core::canon::to_canonical_string(issued.handle().1)
                    .expect("issued handle bytes")
                    .as_bytes()
            );
            assert_eq!(custody.mutation_counts(), (1, 0));
            assert_eq!(anchor.access_counts(), (1, 1));
            assert_eq!(owner_calls.get(), 1);
            assert_eq!(signatures.get(), 3);
            assert!(authority.calls > 0);
            assert_eq!(selected_port.entry_count(), 0);
            assert_eq!(other_port.entry_count(), 0);
            assert_eq!(host.provider_routes.reserved, 0);
            assert_eq!(host.provider_routes.checked_out, 0);
            assert!(host.provider_routes.active.is_empty());
        },
    );
}

#[test]
fn consult_parent_retains_complete_intent_before_any_child_or_collection() {
    let (generation, profile, host_generation) = source_generation_with_host_copy();
    crate::local_operation_start_fixture::with_operation_start_input_at_store(
        &generation,
        &profile,
        "process-host-consult-start",
        |_| {},
        OperationClassV1::Consult,
        &crate::local_operation_start_fixture::store_coordinate(),
        |semantic, request, checkpoint_template, parents| {
            let selected_store = crate::local_operation_start_fixture::store_coordinate();
            let other_store = store("other-consult-store", "other-consult-incarnation");
            let selector = selector_from_signing_authority(parents.signing);
            let selected_port = MemoryAuthorityStore::default();
            let other_port = MemoryAuthorityStore::default();
            let mut host = host_for_start(
                &selected_port,
                &other_port,
                &selector,
                &selected_store,
                &other_store,
                host_generation,
            );
            let mut custody = absent_start_store(request).with_preparation(
                initial_response,
                pale_core::records::ControlTransactionSequenceV1::new(1)
                    .expect("initial transaction"),
            );
            let mut anchor = initial_anchor();
            let mut authority = CountingAuthority::default();
            let owner_calls = Cell::new(0);
            let signatures = Cell::new(0);
            let checkpoint_schema = checkpoint_template.schema_sha256.clone();
            let checkpoint_authority = semantic.checkpoint_authority;
            let evidence = ImmutableRecordReferenceV1 {
                record_ref: local_operation_start_fixture::record_ref("evidence/consult-start"),
                record_sha256: ArtifactDigest::hash_bytes(b"evidence/consult-start"),
            };
            let slot = crate::resident::consultation::ConsultationStartIntentSlot::new(
                ConsultationSlotId::parse("existing-evidence").expect("slot"),
                ConsultationRequirement::Required,
                evidence.clone(),
                ParticipantRefV1::Application {
                    application_id: "existing-evidence".into(),
                },
                None,
                None,
                crate::resident::consultation::ConsultationStartIntentSource::EvidenceResolved {
                    evidence: evidence.clone(),
                },
            );
            assert!(matches!(
                crate::resident::consultation::ConsultationStartIntent::admit(
                    &selected_store,
                    &semantic.handle_ref,
                    &semantic.handle,
                    crate::resident::consultation::ConsultationStartIntentAdmission {
                        completion_policy: evidence.clone(),
                        opened_at: semantic.handle.created_at,
                        deadline: semantic.handle.expires_at,
                        slots: vec![slot.clone(), slot.clone()],
                        limits: ConsultationCollectionLimits::new(2, 16_384, 4096)
                            .expect("bounded collection"),
                    },
                ),
                Err(crate::resident::consultation::ConsultationStartIntentError::DuplicateSlot)
            ));
            let intent = crate::resident::consultation::ConsultationStartIntent::admit(
                &selected_store,
                &semantic.handle_ref,
                &semantic.handle,
                crate::resident::consultation::ConsultationStartIntentAdmission {
                    completion_policy: evidence.clone(),
                    opened_at: semantic.handle.created_at,
                    deadline: semantic.handle.expires_at,
                    slots: vec![slot],
                    limits: ConsultationCollectionLimits::new(1, 16_384, 4096)
                        .expect("bounded collection"),
                },
            )
            .expect("complete intent admits before start");
            let started = host
                .begin_consultation_start(
                    &selector,
                    ResidentDeploymentOperationAccess {
                        store: &selected_store,
                        custody: &mut custody,
                        sessions: &selected_port,
                        anchor: &mut anchor,
                    },
                    &mut authority,
                    invoke_start_input(semantic, &checkpoint_schema, checkpoint_authority),
                    intent,
                    |_, _, _, _, _, _, _, _| {
                        owner_calls.set(owner_calls.get() + 1);
                        Ok(())
                    },
                    |_, _, _| {
                        signatures.set(signatures.get() + 1);
                        Ok(local_operation_start_fixture::signature())
                    },
                    |_, _, _| {
                        signatures.set(signatures.get() + 1);
                        Ok(local_operation_start_fixture::signature())
                    },
                    move |prepared| checkpoint_preimage(prepared, checkpoint_template),
                    |_, _, _| {
                        signatures.set(signatures.get() + 1);
                        Ok(local_operation_start_fixture::signature())
                    },
                )
                .unwrap_or_else(|_| panic!("Consult parent reaches complete custody"));
            let issued = started.parent();

            assert_eq!(
                issued.handle().1.operation_class(),
                &OperationClassV1::Consult
            );
            assert_eq!(custody.mutation_counts(), (1, 0));
            assert_eq!(anchor.access_counts(), (1, 1));
            assert_eq!(owner_calls.get(), 1);
            assert_eq!(signatures.get(), 3);
            assert!(authority.calls > 0);
            assert_eq!(host.operations.active_consultation_batches(), 0);
            assert_eq!(host.operations.in_flight_consultation_tasks(), 0);
            assert_eq!(started.remaining_child_capacity(), 0);
        },
    );
}

#[test]
fn callable_consult_start_retains_exact_children_recovers_and_opens_one_finite_batch() {
    let (generation, profile, host_generation) = source_generation_with_host_copy();
    crate::local_operation_start_fixture::with_operation_start_input_at_store(
        &generation,
        &profile,
        "process-host-consult-journey",
        |_| {},
        OperationClassV1::Consult,
        &crate::local_operation_start_fixture::store_coordinate(),
        |semantic, request, checkpoint_template, parents| {
            let overflow_semantic = copy_semantic_start(&semantic);
            let overflow_checkpoint_template = checkpoint_template.clone();
            let selected_store = crate::local_operation_start_fixture::store_coordinate();
            let other_store = store("other-journey-store", "other-journey-incarnation");
            let collection_limits =
                ConsultationCollectionLimits::new(2, 16_384, 4096).expect("bounded collection");
            let immutable = |value: &str| ImmutableRecordReferenceV1 {
                record_ref: local_operation_start_fixture::record_ref(value),
                record_sha256: ArtifactDigest::hash_bytes(value.as_bytes()),
            };
            let selector = selector_from_signing_authority(parents.signing);
            let selected_port = MemoryAuthorityStore::default();
            let other_port = MemoryAuthorityStore::default();
            let mut host = host_for_start(
                &selected_port,
                &other_port,
                &selector,
                &selected_store,
                &other_store,
                host_generation,
            );
            let mut parent_custody = absent_start_store(request)
                .with_preparation(
                    initial_response,
                    pale_core::records::ControlTransactionSequenceV1::new(1)
                        .expect("initial transaction"),
                )
                .with_lease_retention(Ok(pale_ports::OperationCustodyCommitOutcome::Committed));
            let mut parent_anchor = initial_anchor();
            let mut authority = CountingAuthority::default();
            let owner_calls = Cell::new(0);
            let signatures = Cell::new(0);
            let checkpoint_schema = checkpoint_template.schema_sha256.clone();
            let checkpoint_authority = semantic.checkpoint_authority;
            let captured_checkpoint = Rc::new(RefCell::new(None));
            let checkpoint_capture = Rc::clone(&captured_checkpoint);
            let planned_children = Rc::new(RefCell::new(Vec::new()));
            let captured_plans = Rc::clone(&planned_children);
            let request_frame = ResidentOperationFrameRequest::StartConsult {
                signed_common_request: pale_core::canon::to_canonical_string(request)
                    .expect("signed Consult request is canonical"),
            };
            let mut frame_port = OperationFramePort::new(&request_frame);
            let deadline = Instant::now()
                .checked_add(std::time::Duration::from_secs(2))
                .expect("test deadline");
            let mut request_host = ResidentOperationRequestHost::new(&mut frame_port, deadline);
            let received = request_host.receive().expect("Consult frame is exact");
            let started = RefCell::new(None);
            let response = operation_request_host::request_host_test_support::answer(
                    &mut request_host,
                    received,
                    &mut host,
                    ResidentOperationRequestResolution::root_start(
                        ResidentOperationRequestKind::StartConsult,
                        request,
                    ),
                    &mut |peer, actual| {
                        assert_eq!(peer, LocalIpcPeerCredentials::new(501, 20, Some(8_081)));
                        assert_eq!(actual, &request_frame);
                        Ok(())
                    },
                    deadline,
                    |host, admitted_request| {
                        assert!(std::ptr::eq(admitted_request, request));
                        let mut semantic = semantic;
                        semantic.request = admitted_request;
                        let phase = host.begin_consultation_start_with_intent_builder(
                    &selector,
                    ResidentDeploymentOperationAccess {
                        store: &selected_store,
                        custody: &mut parent_custody,
                        sessions: &selected_port,
                        anchor: &mut parent_anchor,
                    },
                    &mut authority,
                    invoke_start_input(semantic, &checkpoint_schema, checkpoint_authority),
                    2,
                    |store, parent_ref, parent_handle| {
                        let plans = vec![
                            planned_test_child(
                                (parent_ref, parent_handle),
                                parents.lease,
                                admitted_request,
                                &generation,
                                &parents,
                                OperationClassV1::Invoke,
                                "journey-invoke",
                            ),
                            planned_test_child(
                                (parent_ref, parent_handle),
                                parents.lease,
                                admitted_request,
                                &generation,
                                &parents,
                                OperationClassV1::Consult,
                                "journey-consult",
                            ),
                        ];
                        captured_plans.replace(plans.clone());
                        crate::resident::consultation::ConsultationStartIntent::admit_signed(
                            store,
                            parent_ref,
                            parent_handle,
                            crate::resident::consultation::ConsultationStartIntentAdmission {
                                completion_policy: immutable("policy/process-consult-journey"),
                                opened_at: UnixTimeV1::new(62).expect("open time"),
                                deadline: UnixTimeV1::new(75).expect("deadline"),
                                slots: vec![
                                    crate::resident::consultation::ConsultationStartIntentSlot::new(
                                        ConsultationSlotId::parse("invoke-reviewer")
                                            .expect("slot"),
                                        ConsultationRequirement::Required,
                                        immutable("requirement/invoke-reviewer"),
                                        ParticipantRefV1::Application {
                                            application_id: "invoke-reviewer".into(),
                                        },
                                        None,
                                        Some(immutable("context/invoke-reviewer")),
                                        crate::resident::consultation::ConsultationStartIntentSource::FreshChild {
                                            child: Box::new(plans[0].clone()),
                                        },
                                    ),
                                    crate::resident::consultation::ConsultationStartIntentSlot::new(
                                        ConsultationSlotId::parse("nested-reviewer")
                                            .expect("slot"),
                                        ConsultationRequirement::Required,
                                        immutable("requirement/nested-reviewer"),
                                        ParticipantRefV1::Application {
                                            application_id: "nested-reviewer".into(),
                                        },
                                        None,
                                        Some(immutable("context/nested-reviewer")),
                                        crate::resident::consultation::ConsultationStartIntentSource::FreshChild {
                                            child: Box::new(plans[1].clone()),
                                        },
                                    ),
                                ],
                                limits: collection_limits,
                            },
                        )
                        .map_err(|_| crate::resident::LocalOperationStartError::Coordinate)
                    },
                    |_, _, _, _, _, _, _, _| {
                        owner_calls.set(owner_calls.get() + 1);
                        Ok(())
                    },
                    |_, _, _| {
                        signatures.set(signatures.get() + 1);
                        Ok(local_operation_start_fixture::signature())
                    },
                    |_, _, _| {
                        signatures.set(signatures.get() + 1);
                        Ok(local_operation_start_fixture::signature())
                    },
                    move |prepared| {
                        let preimage = checkpoint_preimage(prepared, checkpoint_template)?;
                        checkpoint_capture.replace(Some(preimage.clone()));
                        Ok(preimage)
                    },
                    |_, _, _| {
                        signatures.set(signatures.get() + 1);
                        Ok(local_operation_start_fixture::signature())
                    },
                        ).map_err(|_| pale_ports::ResidentOperationFrameFailure::ResidentUnavailable)?;
                        let records = start_records(
                            phase.parent(),
                            parent_custody
                                .committed_candidate()
                                .expect("Consult parent reached durable custody"),
                        );
                        started.replace(Some(phase));
                        Ok(ResidentOperationRequestOutcome::Started(records))
                    },
                )
                .expect("private request host returns the real durable start");
            assert!(matches!(
                response,
                pale_ports::ResidentOperationFrameResponse::Started { .. }
            ));
            let started = started
                .into_inner()
                .expect("real Consult start survives the transport projection");
            let first_child_pending = match started.into_initialization() {
                ResidentProcessConsultationInitialization::StartChild(pending) => {
                    assert!(pending.children().is_empty());
                    assert!(parent_custody.committed_candidate().is_some());
                    assert!(pending
                        .parent()
                        .consultation_start_intent()
                        .is_some_and(|intent| intent
                            .validate_parent(&selected_store, pending.parent())
                            .is_ok()));
                    assert_eq!(
                        pending.child().handle_coordinate(),
                        planned_children.borrow()[0].handle_coordinate()
                    );
                    pending
                }
                ResidentProcessConsultationInitialization::OpenCollection(_) => {
                    panic!("two retained fresh children cannot skip child initialization")
                }
            };
            let parent = first_child_pending.parent().fixture_clone();
            crate::resident::retain_operation_lease_body(
                &mut parent_custody,
                &selected_store,
                &crate::local_operation_start_fixture::operation_lease_ref(),
                parent.historical_operation_lease(),
                1_048_576,
            )
            .expect("Consult parent lease reaches durable custody");
            let retained_intent = parent
                .consultation_start_intent()
                .expect("normal parent custody retains its complete intent");
            assert!(matches!(
                persisted_intent_fault(retained_intent, PersistedIntentFault::ZeroLimit)
                    .validate_parent(&selected_store, &parent),
                Err(crate::resident::consultation::ConsultationStartIntentError::Bounds)
            ));
            assert!(matches!(
                persisted_intent_fault(retained_intent, PersistedIntentFault::InvalidTime)
                    .validate_parent(&selected_store, &parent),
                Err(crate::resident::consultation::ConsultationStartIntentError::Association)
            ));
            assert!(matches!(
                persisted_intent_fault(retained_intent, PersistedIntentFault::DuplicateSlot)
                    .validate_parent(&selected_store, &parent),
                Err(crate::resident::consultation::ConsultationStartIntentError::DuplicateSlot)
            ));
            assert!(matches!(
                persisted_intent_fault(retained_intent, PersistedIntentFault::DuplicateChild)
                    .validate_parent(&selected_store, &parent),
                Err(crate::resident::consultation::ConsultationStartIntentError::DuplicateChild)
            ));
            assert!(matches!(
                persisted_intent_fault(
                    retained_intent,
                    PersistedIntentFault::WrongChildAssociation
                )
                .validate_parent(&selected_store, &parent),
                Err(crate::resident::consultation::ConsultationStartIntentError::Child)
            ));

            let parent_checkpoint_preimage = captured_checkpoint
                .borrow()
                .clone()
                .expect("parent checkpoint preimage was signed");
            let parent_checkpoint =
                crate::resident::operation_observation_custody::tests::admit_initial_checkpoint(
                    &parent_checkpoint_preimage,
                );
            let parent_checkpoint_ref = local_operation_start_fixture::record_ref(
                "process-host/start/preflight-checkpoint",
            );
            let parent_candidate = parent_custody
                .committed_candidate()
                .expect("parent candidate reached database custody")
                .clone();
            let initial_record = parent_candidate
                .records()
                .iter()
                .find_map(|record| match record {
                    OperationCustodyRecord::OperationObservation(record) => Some(record.clone()),
                    _ => None,
                })
                .expect("parent initial observation");
            let observation_schema = local_operation_start_fixture::schema_digest(
                pale_core::records::OPERATION_OBSERVATION_V1_SCHEMA_ID,
            );
            let parent_observation = OperationObservationV1::admit_contextual_wire(
                serde_json::from_value(
                    pale_core::canon::parse_strict(
                        initial_record.canonical_record_jcs().as_slice(),
                    )
                    .expect("parent observation JSON"),
                )
                .expect("parent observation wire"),
                &OperationObservationAdmissionContextV1 {
                    signed: SignedResidentAdmissionContextV1 {
                        expected_schema_sha256: &observation_schema,
                        signing_authority: parents.signing,
                    },
                    handle_ref: parent.handle().0,
                    handle: parent.handle().1,
                    predecessor: None,
                },
                |_| Ok(()),
                |_, _, _, _| panic!("revision one cannot rotate authority"),
                |_, _, _, _| Ok(()),
            )
            .expect("parent observation admits");

            let child_allocations = Cell::new(0usize);
            let child_owner_calls = Cell::new(0usize);
            let child_signatures = Cell::new(0usize);
            let (
                second_child_pending,
                invoke_child,
                invoke_candidate,
                invoke_request_ref,
                invoke_request,
                invoke_checkpoint_ref,
                invoke_checkpoint,
            ) = with_test_child_start_input(
                parent.handle(),
                parent.historical_operation_lease(),
                Some((&parent_checkpoint_ref, &parent_checkpoint)),
                request,
                &generation,
                &parents,
                OperationClassV1::Invoke,
                "journey-invoke",
                |input, mut checkpoint_template| {
                    child_allocations.set(child_allocations.get() + 1);
                    checkpoint_template.control_checkpoint_id = input.checkpoint.id.clone();
                    let child_request_ref = input.semantic.request_ref.clone();
                    let child_request = input.semantic.request.clone();
                    let child_checkpoint_ref = input.checkpoint.reference.clone();
                    let sequence = parent_checkpoint.domain_sequence().get() + 1;
                    let protected_parent = PreparedLocalChildParent::fixture(
                        &parent,
                        (&parent_checkpoint_ref, &parent_checkpoint),
                        ControlTransactionPreparation::new(
                            selected_store.clone(),
                            ControlTransactionSequenceV1::new(sequence)
                                .expect("first child transaction"),
                        )
                        .expect("selected first child transaction"),
                        parents.current.now,
                    );
                    let mut child_custody = absent_start_store(&child_request)
                        .with_preparation(
                            prepare_collection_accumulator,
                            ControlTransactionSequenceV1::new(sequence)
                                .expect("first child transaction"),
                        )
                        .with_lease_retention(Ok(
                            pale_ports::OperationCustodyCommitOutcome::Committed,
                        ));
                    let mut interrupted_anchor = RecordingAnchor::unavailable_after_preflight();
                    let captured_checkpoint = Rc::new(RefCell::new(None));
                    let checkpoint_capture = Rc::clone(&captured_checkpoint);
                    let failure = match first_child_pending.start(
                        &mut host,
                        ResidentDeploymentOperationAccess {
                            store: &selected_store,
                            custody: &mut child_custody,
                            sessions: &selected_port,
                            anchor: &mut interrupted_anchor,
                        },
                        &mut authority,
                        input,
                        protected_parent,
                        |_, _, _, _, _, _, _, _| {
                            child_owner_calls.set(child_owner_calls.get() + 1);
                            Ok(())
                        },
                        |_, _, _| {
                            child_signatures.set(child_signatures.get() + 1);
                            Ok(local_operation_start_fixture::signature())
                        },
                        |_, _, _| {
                            child_signatures.set(child_signatures.get() + 1);
                            Ok(local_operation_start_fixture::signature())
                        },
                        move |prepared| {
                            let preimage = checkpoint_preimage(prepared, checkpoint_template)?;
                            checkpoint_capture.replace(Some(preimage.clone()));
                            Ok(preimage)
                        },
                        |_, _, _| {
                            child_signatures.set(child_signatures.get() + 1);
                            Ok(local_operation_start_fixture::signature())
                        },
                    ) {
                        Ok(_) => panic!("the interrupted child anchor cannot complete"),
                        Err(failure) => failure,
                    };
                    let ResidentProcessConsultationInitializationChildFailure { pending, reason } =
                        failure;
                    assert!(pending.children().is_empty());
                    let continuation = match reason {
                        ResidentProcessConsultationChildStartError::Start(
                            ResidentProcessConsultStartError::Continuation(continuation),
                        ) => continuation,
                        _ => panic!("the exact lower child-start continuation must survive"),
                    };
                    assert_eq!(child_allocations.get(), 1);
                    assert_eq!(child_owner_calls.get(), 1);
                    assert_eq!(child_signatures.get(), 3);
                    assert_eq!(child_custody.mutation_counts(), (1, 0));
                    assert_eq!(interrupted_anchor.access_counts(), (1, 1));

                    interrupted_anchor.allow_next_advance();
                    let child = host
                        .continue_operation_start(
                            *continuation,
                            ResidentDeploymentOperationAccess {
                                store: &selected_store,
                                custody: &mut child_custody,
                                sessions: &selected_port,
                                anchor: &mut interrupted_anchor,
                            },
                            &mut authority,
                        )
                        .unwrap_or_else(|_| {
                            panic!("the exact lower child-start continuation resumes")
                        });
                    assert!(pending.children().is_empty());
                    assert_eq!(child_allocations.get(), 1);
                    assert_eq!(child_owner_calls.get(), 1);
                    assert_eq!(child_signatures.get(), 3);
                    assert_eq!(child_custody.mutation_counts(), (1, 0));
                    assert_eq!(interrupted_anchor.access_counts(), (2, 1));

                    let mut started = pending.into_started();
                    started
                        .retain_recovered_child(child)
                        .unwrap_or_else(|_| panic!("the resumed child is the exact first plan"));
                    let second_child_pending = match started.into_initialization() {
                        ResidentProcessConsultationInitialization::StartChild(pending) => {
                            assert_eq!(pending.children().len(), 1);
                            assert_eq!(
                                pending.child().handle_coordinate(),
                                planned_children.borrow()[1].handle_coordinate()
                            );
                            pending
                        }
                        ResidentProcessConsultationInitialization::OpenCollection(_) => {
                            panic!("one child cannot open a two-child collection")
                        }
                    };
                    let invoke_child = second_child_pending.children()[0].fixture_clone();
                    let invoke_candidate = child_custody
                        .committed_candidate()
                        .expect("first child reached database custody once")
                        .clone();
                    let checkpoint_preimage = captured_checkpoint
                        .borrow()
                        .clone()
                        .expect("first child checkpoint was signed once");
                    let invoke_checkpoint = crate::resident::operation_observation_custody::tests::
                        admit_successor_checkpoint(
                            &checkpoint_preimage,
                            &parent_checkpoint_ref,
                            &parent_checkpoint,
                        );
                    (
                        second_child_pending,
                        invoke_child,
                        invoke_candidate,
                        child_request_ref,
                        child_request,
                        child_checkpoint_ref,
                        invoke_checkpoint,
                    )
                },
            );

            let (started, nested_consult, nested_candidate, nested_request_ref, nested_request) =
                with_test_child_start_input(
                    parent.handle(),
                    parent.historical_operation_lease(),
                    Some((&invoke_checkpoint_ref, &invoke_checkpoint)),
                    request,
                    &generation,
                    &parents,
                    OperationClassV1::Consult,
                    "journey-consult",
                    |input, mut checkpoint_template| {
                        child_allocations.set(child_allocations.get() + 1);
                        checkpoint_template.control_checkpoint_id = input.checkpoint.id.clone();
                        let child_request_ref = input.semantic.request_ref.clone();
                        let child_request = input.semantic.request.clone();
                        let sequence = invoke_checkpoint.domain_sequence().get() + 1;
                        let protected_parent = PreparedLocalChildParent::fixture(
                            &parent,
                            (&invoke_checkpoint_ref, &invoke_checkpoint),
                            ControlTransactionPreparation::new(
                                selected_store.clone(),
                                ControlTransactionSequenceV1::new(sequence)
                                    .expect("second child transaction"),
                            )
                            .expect("selected second child transaction"),
                            parents.current.now,
                        );
                        let mut child_custody = absent_start_store(&child_request)
                            .with_preparation(
                                prepare_collection_accumulator,
                                ControlTransactionSequenceV1::new(sequence)
                                    .expect("second child transaction"),
                            )
                            .with_lease_retention(Ok(
                                pale_ports::OperationCustodyCommitOutcome::Committed,
                            ));
                        let mut child_anchor = crate::resident::operation_observation_custody::tests::
                        local_start::successor_anchor();
                        let initialization = second_child_pending
                            .start(
                                &mut host,
                                ResidentDeploymentOperationAccess {
                                    store: &selected_store,
                                    custody: &mut child_custody,
                                    sessions: &selected_port,
                                    anchor: &mut child_anchor,
                                },
                                &mut authority,
                                input,
                                protected_parent,
                                |_, _, _, _, _, _, _, _| {
                                    child_owner_calls.set(child_owner_calls.get() + 1);
                                    Ok(())
                                },
                                |_, _, _| {
                                    child_signatures.set(child_signatures.get() + 1);
                                    Ok(local_operation_start_fixture::signature())
                                },
                                |_, _, _| {
                                    child_signatures.set(child_signatures.get() + 1);
                                    Ok(local_operation_start_fixture::signature())
                                },
                                move |prepared| checkpoint_preimage(prepared, checkpoint_template),
                                |_, _, _| {
                                    child_signatures.set(child_signatures.get() + 1);
                                    Ok(local_operation_start_fixture::signature())
                                },
                            )
                            .unwrap_or_else(|_| panic!("the exact second child reaches custody"));
                        let collection_pending = match initialization {
                            ResidentProcessConsultationInitialization::OpenCollection(pending) => {
                                assert_eq!(pending.children().len(), 2);
                                pending
                            }
                            ResidentProcessConsultationInitialization::StartChild(_) => {
                                panic!("the final child must stop at OpenCollection")
                            }
                        };
                        let nested_consult = collection_pending.children()[1].fixture_clone();
                        let nested_candidate = child_custody
                            .committed_candidate()
                            .expect("second child reached database custody once")
                            .clone();
                        let (initial_collection, initial_limits) = collection_pending
                            .admit_initial_collection()
                            .expect("retained intent admits the exact issued child prefix");
                        assert_eq!(initial_limits, collection_limits);
                        assert_eq!(initial_collection.slots().len(), 2);
                        assert!(collection_pending
                            .parent()
                            .consultation_start_intent()
                            .is_some_and(|intent| intent.matches_collection(&initial_collection)));
                        assert_eq!(child_custody.mutation_counts(), (1, 0));
                        assert_eq!(child_anchor.access_counts(), (1, 1));
                        (
                            collection_pending.into_started(),
                            nested_consult,
                            nested_candidate,
                            child_request_ref,
                            child_request,
                        )
                    },
                );
            assert_eq!(child_allocations.get(), 2);
            assert_eq!(child_owner_calls.get(), 2);
            assert_eq!(child_signatures.get(), 6);

            let session = parents.session.lease();
            let session_reader = ConsultRecoverySession(
                pale_ports::SessionLeaseBody::new(
                    selected_store.clone(),
                    session.session_admission_lease_digest().clone(),
                    session.schema_id().clone(),
                    session.schema_sha256().clone(),
                    pale_ports::ControlRecordBytes::new(
                        pale_core::canon::to_canonical_string(&SessionAdmissionLeaseWireV1::from(
                            session,
                        ))
                        .expect("session serializes")
                        .into_bytes(),
                    )
                    .expect("bounded session bytes"),
                ),
                AtomicUsize::new(0),
            );
            let mut historical = RecoveryResolver::default();
            historical.insert_generation(parents.current.generation.compiled_generation);
            historical.insert_active(parents.current.generation.active_head);
            historical.insert_fence(parents.current.generation.activation_fence);
            historical.insert_effective_profile(parents.current.generation.effective_profile);
            let parent_authority = ControlCheckpointSigningAuthorityAdmissionContextV1 {
                authority_ref: parent_checkpoint.checkpoint_signing_authority_ref(),
                authority_sha256: parent_checkpoint.checkpoint_signing_authority_sha256(),
                principal_id: parent_checkpoint.issuer_principal_id(),
                control_store_id: parent_checkpoint.control_store_id(),
                control_incarnation_id: parent_checkpoint.control_incarnation_id(),
                checkpoint_domain_id: parent_checkpoint.checkpoint_domain_id(),
                checkpoint_profile_ref: parent_checkpoint.checkpoint_profile_ref(),
                checkpoint_profile_sha256: parent_checkpoint.checkpoint_profile_sha256(),
                subject_key_id: parent_checkpoint.issuer_key_id(),
                subject_key_epoch: *parent_checkpoint.issuer_key_epoch(),
            };
            let head = parent_candidate.checkpoint().successor();
            let mut recovery_anchor = ConsultRecoveryAnchor {
                value: pale_ports::AntiRollbackAnchor::ControlDomain(
                    pale_ports::ControlDomainAntiRollbackAnchor::new(
                        selected_store.store_id().clone(),
                        selected_store.incarnation_id().clone(),
                        head.checkpoint_domain_id().clone(),
                        head.domain_epoch(),
                        head.domain_sequence(),
                        head.coordinate().clone(),
                    ),
                ),
                reads: 0,
                writes: 0,
            };
            let supported_profile = crate::local_operation_start_fixture::profile();
            let expected_parent = pale_ports::ControlRecordCoordinate::new(
                parent.handle().0.clone(),
                pale_ports::ControlRecordDigest::parse(
                    parent
                        .handle()
                        .1
                        .canonical_operation_handle_sha256()
                        .as_str(),
                )
                .expect("parent handle digest"),
            );
            let request_preimage = request.to_preimage();
            parent_custody.enable_committed_candidate_recovery();
            let recover_parent =
                |parent_custody: &mut dyn OperationCustodyPort,
                 recovery_anchor: &mut ConsultRecoveryAnchor| {
                    crate::resident::recover_local_operation_from_handle(
                crate::resident::LocalOperationHandleRecoveryInput {
                    context: crate::resident::LocalOperationRecoveryContext {
                        store: &selected_store,
                        profile: &supported_profile,
                        handle_signing_authority: parents.signing,
                        checkpoint_authority: &parent_authority,
                        historical: &historical,
                        maximum_record_bytes: 1_048_576,
                        maximum_operation_lease_bytes: 1_048_576,
                        maximum_session_lease_bytes: 1_048_576,
                        membership_limits: pale_ports::AccumulatorMembershipLimits::new(
                            3, 10_000, 100_000,
                        )
                        .expect("membership limits"),
                        observation_limits: OperationObservationRecoveryLimits::new(
                            2, 1_048_576, 1_048_576,
                        )
                        .expect("observation limits"),
                        checkpoint_limits: OperationCheckpointAncestryLimits::new(3, 1_048_576, 3)
                            .expect("checkpoint limits"),
                    },
                    expected_handle: &expected_parent,
                    maximum_parent_depth: 0,
                },
                parent_custody,
                &session_reader,
                recovery_anchor,
                |reference, wire| {
                    assert_eq!(reference, parent.request_coordinate().record_ref());
                    CommonRequestEnvelopeV1::admit_contextual_wire(
                        wire,
                        &consult_request_context(&request_preimage),
                        accept_request_signature,
                    )
                },
                crate::resident::LocalOperationRecoveryAdmission {
                    admit_historical_authority: accept_root_handle_authority,
                    verify_handle_signature: accept_handle_signature,
                    verify_checkpoint_signature: accept_checkpoint_signature,
                    initial_observation: crate::resident::OperationObservationRecoveryAdmission {
                        admit_checkpoint:
                            |_: &ResidentRecordRefV1, wire: ControlStateCheckpointWireV1| {
                                ControlStateCheckpointV1::admit_signed_wire(
                                    wire,
                                    &local_operation_start_fixture::schema_digest(
                                        pale_core::records::CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID,
                                    ),
                                    &parent_authority,
                                    accept_checkpoint_signature,
                                )
                            },
                        resolve_signer:
                            |_: &ResidentRecordRefV1, _: &OperationObservationWireV1| {
                                Ok(parents.signing.clone())
                            },
                        admit_state: accept_observation_state,
                        admit_rotation: accept_authority_rotation,
                        verify_signature: accept_observation_signature,
                    },
                },
            )
                };
            let recovered_parent = recover_parent(&mut parent_custody, &mut recovery_anchor)
                .expect("parent recovers from its persisted session basis");
            assert!(recovered_parent
                .operation()
                .consultation_start_intent()
                .is_some());
            let retained_basis = parent_candidate
                .session_basis()
                .expect("parent custody retained the exact session basis");
            for (fault, is_bound_failure) in [
                (PersistedIntentFault::ZeroLimit, true),
                (PersistedIntentFault::InvalidTime, false),
                (PersistedIntentFault::DuplicateSlot, false),
                (PersistedIntentFault::DuplicateChild, false),
                (PersistedIntentFault::WrongChildAssociation, false),
            ] {
                let malformed = persisted_intent_fault(retained_intent, fault);
                parent_custody.replace_session_basis_override(Some(persisted_basis_with_intent(
                    retained_basis,
                    &malformed,
                )));
                let anchor_reads = recovery_anchor.reads;
                let rejected = recover_parent(&mut parent_custody, &mut recovery_anchor);
                assert!(if is_bound_failure {
                    matches!(
                        rejected,
                        Err(crate::resident::LocalOperationRecoveryError::Bounds)
                    )
                } else {
                    matches!(
                        rejected,
                        Err(crate::resident::LocalOperationRecoveryError::Association)
                    )
                });
                assert!(
                    recovery_anchor.reads > anchor_reads,
                    "semantic intent admission must follow protected checkpoint membership"
                );
            }
            parent_custody.replace_session_basis_override(None);
            let recovery_context = crate::resident::LocalOperationRecoveryContext {
                store: &selected_store,
                profile: &supported_profile,
                handle_signing_authority: parents.signing,
                checkpoint_authority: &parent_authority,
                historical: &historical,
                maximum_record_bytes: 1_048_576,
                maximum_operation_lease_bytes: 1_048_576,
                maximum_session_lease_bytes: 1_048_576,
                membership_limits: pale_ports::AccumulatorMembershipLimits::new(3, 10_000, 100_000)
                    .expect("membership limits"),
                observation_limits: OperationObservationRecoveryLimits::new(
                    2, 1_048_576, 1_048_576,
                )
                .expect("observation limits"),
                checkpoint_limits: OperationCheckpointAncestryLimits::new(3, 1_048_576, 3)
                    .expect("checkpoint limits"),
            };
            let invoke_request_preimage = invoke_request.to_preimage();
            let nested_request_preimage = nested_request.to_preimage();
            let nested_handle_record =
                ConsultPrefixCustody::handle_record(&nested_candidate).clone();
            let child_coordinate =
                |child: &crate::resident::IssuedLocalOperation| ImmutableRecordReferenceV1 {
                    record_ref: child.handle().0.clone(),
                    record_sha256: ArtifactDigest::parse(
                        child
                            .handle()
                            .1
                            .canonical_operation_handle_sha256()
                            .as_str(),
                    )
                    .expect("child handle digest"),
                };
            let invoke_coordinate = child_coordinate(&invoke_child);
            let nested_coordinate = child_coordinate(&nested_consult);
            let captured_plans = planned_children.borrow();
            assert!(captured_plans[0].matches_operation(&invoke_child));
            assert!(captured_plans[1].matches_operation(&nested_consult));
            let first_handle = captured_plans[0].handle_coordinate();
            drop(captured_plans);
            drop(started);

            let live_authority_calls = authority.calls;
            let owner_admissions = owner_calls.get();
            let issued_signatures = signatures.get();
            let parent_mutations = parent_custody.mutation_counts();
            let parent_anchor_access = parent_anchor.access_counts();
            let dispatches = Cell::new(0usize);
            let historical_authority_admissions = Cell::new(0usize);
            let historical_signature_verifications = Cell::new(0usize);
            let mut prefix_custody = ConsultPrefixCustody::new(&parent, parent_candidate.clone());
            recovery_anchor.select(prefix_custody.latest_candidate());

            let recover_prefix =
                |custody: &mut ConsultPrefixCustody,
                 anchor: &mut ConsultRecoveryAnchor,
                 recovered_parent: crate::resident::IssuedLocalOperation| {
                    let mut admit_request = |reference: &ResidentRecordRefV1,
                                         wire: pale_core::records::CommonRequestEnvelopeWireV1| {
                    let preimage = if reference == parent.request_coordinate().record_ref() {
                        &request_preimage
                    } else if reference == &invoke_request_ref {
                        &invoke_request_preimage
                    } else if reference == &nested_request_ref {
                        &nested_request_preimage
                    } else {
                        panic!("protected recovery requested an unrelated request")
                    };
                    CommonRequestEnvelopeV1::admit_contextual_wire(
                        wire,
                        &consult_request_context(preimage),
                        accept_request_signature,
                    )
                };
                    let mut admit_handle_authority =
                        |_: &OperationHandlePreimageV1, _: Option<&OperationHandleV1>| {
                            historical_authority_admissions
                                .set(historical_authority_admissions.get() + 1);
                            Ok(())
                        };
                    let mut verify_handle_signature =
                        |bytes: &[u8],
                         preimage: &OperationHandlePreimageV1,
                         digest: &OperationHandleDigest,
                         signature: &CanonicalSignatureV1| {
                            historical_signature_verifications
                                .set(historical_signature_verifications.get() + 1);
                            accept_handle_signature(bytes, preimage, digest, signature)
                        };
                    let mut admit_checkpoint =
                        |_: &ResidentRecordRefV1, wire: ControlStateCheckpointWireV1| {
                            ControlStateCheckpointV1::admit_signed_wire(
                                wire,
                                &local_operation_start_fixture::schema_digest(
                                    pale_core::records::CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID,
                                ),
                                &parent_authority,
                                |bytes, preimage, digest, signature| {
                                    historical_signature_verifications
                                        .set(historical_signature_verifications.get() + 1);
                                    accept_checkpoint_signature(bytes, preimage, digest, signature)
                                },
                            )
                        };
                    let mut resolve_observation_signer =
                        |_: &ResidentRecordRefV1, _: &OperationObservationWireV1| {
                            Ok(parents.signing.clone())
                        };
                    let mut admit_observation_state = accept_observation_state;
                    let mut admit_rotation = accept_authority_rotation;
                    let mut verify_observation_signature =
                        |bytes: &[u8],
                         preimage: &OperationObservationPreimageV1,
                         digest: &OperationObservationDigest,
                         signature: &CanonicalSignatureV1| {
                            historical_signature_verifications
                                .set(historical_signature_verifications.get() + 1);
                            accept_observation_signature(bytes, preimage, digest, signature)
                        };
                    let mut verify_checkpoint_signature =
                        |bytes: &[u8],
                         preimage: &ControlStateCheckpointPreimageV1,
                         digest: &ControlStateCheckpointDigest,
                         signature: &CanonicalSignatureV1| {
                            historical_signature_verifications
                                .set(historical_signature_verifications.get() + 1);
                            accept_checkpoint_signature(bytes, preimage, digest, signature)
                        };
                    host.rebuild_consultation_start_from_protected_intent(
                        &selector,
                        &selected_store,
                        recovered_parent,
                        recovery_context,
                        1,
                        custody,
                        &session_reader,
                        anchor,
                        crate::resident::OperationControlRecoveryAdmission {
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
                    )
                };

            let recovered_parent = recover_parent(&mut prefix_custody, &mut recovery_anchor)
                .expect("parent recovers before the first planned child");
            let recovered_before_first = recover_prefix(
                &mut prefix_custody,
                &mut recovery_anchor,
                recovered_parent.into_operation(),
            )
            .unwrap_or_else(|_| panic!("missing planned children remain an exact suffix"));
            let recovered_before_first = match recovered_before_first.into_initialization() {
                ResidentProcessConsultationInitialization::StartChild(pending) => {
                    assert!(pending.children().is_empty());
                    assert_eq!(pending.child().handle_coordinate(), first_handle);
                    pending.into_started()
                }
                ResidentProcessConsultationInitialization::OpenCollection(_) => {
                    panic!("parent-only recovery must resume at the first child")
                }
            };
            assert_eq!(recovered_before_first.remaining_child_capacity(), 2);

            prefix_custody.retain(&invoke_child, invoke_candidate);
            recovery_anchor.select(prefix_custody.latest_candidate());
            let recovered_parent = recover_parent(&mut prefix_custody, &mut recovery_anchor)
                .expect("parent recovers between planned children");
            let recovered_between_children = recover_prefix(
                &mut prefix_custody,
                &mut recovery_anchor,
                recovered_parent.into_operation(),
            )
            .unwrap_or_else(|_| panic!("one committed child remains an exact prefix"));
            let recovered_between_children = match recovered_between_children.into_initialization()
            {
                ResidentProcessConsultationInitialization::StartChild(pending) => {
                    assert_eq!(pending.children().len(), 1);
                    assert_eq!(
                        pending.child().handle_coordinate(),
                        planned_children.borrow()[1].handle_coordinate()
                    );
                    pending.into_started()
                }
                ResidentProcessConsultationInitialization::OpenCollection(_) => {
                    panic!("one retained child cannot open a two-child collection")
                }
            };
            assert_eq!(recovered_between_children.remaining_child_capacity(), 1);

            let mut duplicate_attempt = recovered_between_children;
            assert!(matches!(
                duplicate_attempt.retain_recovered_child(invoke_child.fixture_clone()),
                Err(ResidentProcessConsultationChildStartError::Start(
                    ResidentProcessConsultStartError::Selection(
                        ResidentProcessHostSelectionError::ConsultationChildAssociation
                    )
                ))
            ));
            assert_eq!(duplicate_attempt.children().len(), 1);

            prefix_custody.retain(&nested_consult, nested_candidate);
            recovery_anchor.select(prefix_custody.latest_candidate());
            let recovered_parent = recover_parent(&mut prefix_custody, &mut recovery_anchor)
                .expect("parent recovers after both planned children");
            let recovered_after_last = recover_prefix(
                &mut prefix_custody,
                &mut recovery_anchor,
                recovered_parent.into_operation(),
            )
            .unwrap_or_else(|_| panic!("both committed children recover in retained order"));
            let recovered_after_last = match recovered_after_last.into_initialization() {
                ResidentProcessConsultationInitialization::OpenCollection(pending) => {
                    assert_eq!(pending.children().len(), 2);
                    assert_eq!(pending.parent().handle(), parent.handle());
                    pending.into_started()
                }
                ResidentProcessConsultationInitialization::StartChild(_) => {
                    panic!("the final child must stop at the unopened collection boundary")
                }
            };
            assert_eq!(recovered_after_last.remaining_child_capacity(), 0);

            prefix_custody.mark_incomplete(first_handle.clone());
            let recovered_parent = recover_parent(&mut prefix_custody, &mut recovery_anchor)
                .expect("parent remains recoverable when child evidence is incomplete");
            assert!(matches!(
                recover_prefix(
                    &mut prefix_custody,
                    &mut recovery_anchor,
                    recovered_parent.into_operation(),
                ),
                Err(ResidentProcessConsultationResumeError::Recovery(
                    crate::resident::LocalOperationRecoveryError::Missing
                ))
            ));
            prefix_custody.clear_faults();

            prefix_custody.hide(first_handle.clone());
            let recovered_parent = recover_parent(&mut prefix_custody, &mut recovery_anchor)
                .expect("parent remains recoverable when the first child is hidden");
            assert!(matches!(
                recover_prefix(
                    &mut prefix_custody,
                    &mut recovery_anchor,
                    recovered_parent.into_operation(),
                ),
                Err(ResidentProcessConsultationResumeError::Selection(
                    ResidentProcessHostSelectionError::ConsultationChildAssociation
                ))
            ));
            prefix_custody.clear_faults();

            prefix_custody.substitute(first_handle, nested_handle_record);
            let recovered_parent = recover_parent(&mut prefix_custody, &mut recovery_anchor)
                .expect("parent remains recoverable after child substitution");
            assert!(matches!(
                recover_prefix(
                    &mut prefix_custody,
                    &mut recovery_anchor,
                    recovered_parent.into_operation(),
                ),
                Err(ResidentProcessConsultationResumeError::Recovery(
                    crate::resident::LocalOperationRecoveryError::Association
                ))
            ));
            prefix_custody.clear_faults();
            assert_eq!(authority.calls, live_authority_calls);
            assert_eq!(owner_calls.get(), owner_admissions);
            assert_eq!(signatures.get(), issued_signatures);
            assert_eq!(parent_custody.mutation_counts(), parent_mutations);
            assert_eq!(parent_anchor.access_counts(), parent_anchor_access);
            assert_eq!(prefix_custody.io_counts().1, 0);
            assert_eq!(recovery_anchor.writes, 0);
            assert_eq!(child_allocations.get(), 2);
            assert_eq!(dispatches.get(), 0);
            assert!(historical_authority_admissions.get() > 0);
            assert!(historical_signature_verifications.get() > 0);

            let mut started = recovered_after_last;
            let collection = ConsultationCollection::admit_initial(
                started.parent().handle().0,
                started.parent().handle().1,
                ConsultationCollectionAdmission {
                    completion_policy: immutable("policy/process-consult-journey"),
                    opened_at: UnixTimeV1::new(62).expect("open time"),
                    deadline: UnixTimeV1::new(75).expect("deadline"),
                    slots: vec![
                        ConsultationRosterSlot::new(
                            ConsultationSlotId::parse("invoke-reviewer").expect("slot"),
                            ConsultationRequirement::Required,
                            immutable("requirement/invoke-reviewer"),
                            ParticipantRefV1::Application {
                                application_id: "invoke-reviewer".into(),
                            },
                            None,
                            Some(immutable("context/invoke-reviewer")),
                            ConsultationContributionSource::FreshChild {
                                child: invoke_coordinate.clone(),
                            },
                        ),
                        ConsultationRosterSlot::new(
                            ConsultationSlotId::parse("nested-reviewer").expect("slot"),
                            ConsultationRequirement::Required,
                            immutable("requirement/nested-reviewer"),
                            ParticipantRefV1::Application {
                                application_id: "nested-reviewer".into(),
                            },
                            None,
                            Some(immutable("context/nested-reviewer")),
                            ConsultationContributionSource::FreshChild {
                                child: nested_coordinate.clone(),
                            },
                        ),
                    ],
                },
                &[
                    (invoke_child.handle().0, invoke_child.handle().1),
                    (nested_consult.handle().0, nested_consult.handle().1),
                ],
                collection_limits,
            )
            .expect("exact durable children form the collection");
            let omitted_collection = ConsultationCollection::admit_initial(
                started.parent().handle().0,
                started.parent().handle().1,
                ConsultationCollectionAdmission {
                    completion_policy: immutable("policy/process-consult-journey"),
                    opened_at: UnixTimeV1::new(62).expect("open time"),
                    deadline: UnixTimeV1::new(75).expect("deadline"),
                    slots: vec![ConsultationRosterSlot::new(
                        ConsultationSlotId::parse("invoke-reviewer").expect("slot"),
                        ConsultationRequirement::Required,
                        immutable("requirement/invoke-reviewer"),
                        ParticipantRefV1::Application {
                            application_id: "invoke-reviewer".into(),
                        },
                        None,
                        Some(immutable("context/invoke-reviewer")),
                        ConsultationContributionSource::FreshChild {
                            child: invoke_coordinate.clone(),
                        },
                    )],
                },
                &[(invoke_child.handle().0, invoke_child.handle().1)],
                collection_limits,
            )
            .expect("one-child collection is independently valid");
            let substituted_collection = ConsultationCollection::admit_initial(
                started.parent().handle().0,
                started.parent().handle().1,
                ConsultationCollectionAdmission {
                    completion_policy: immutable("policy/process-consult-journey"),
                    opened_at: UnixTimeV1::new(62).expect("open time"),
                    deadline: UnixTimeV1::new(75).expect("deadline"),
                    slots: vec![
                        ConsultationRosterSlot::new(
                            ConsultationSlotId::parse("nested-reviewer").expect("slot"),
                            ConsultationRequirement::Required,
                            immutable("requirement/nested-reviewer"),
                            ParticipantRefV1::Application {
                                application_id: "nested-reviewer".into(),
                            },
                            None,
                            Some(immutable("context/nested-reviewer")),
                            ConsultationContributionSource::FreshChild {
                                child: nested_coordinate,
                            },
                        ),
                        ConsultationRosterSlot::new(
                            ConsultationSlotId::parse("invoke-reviewer").expect("slot"),
                            ConsultationRequirement::Required,
                            immutable("requirement/invoke-reviewer"),
                            ParticipantRefV1::Application {
                                application_id: "invoke-reviewer".into(),
                            },
                            None,
                            Some(immutable("context/invoke-reviewer")),
                            ConsultationContributionSource::FreshChild {
                                child: invoke_coordinate,
                            },
                        ),
                    ],
                },
                &[
                    (nested_consult.handle().0, nested_consult.handle().1),
                    (invoke_child.handle().0, invoke_child.handle().1),
                ],
                collection_limits,
            )
            .expect("substituted order remains a valid standalone collection");
            let overflow_parent = PreparedLocalChildParent::fixture(
                started.parent(),
                (&parent_checkpoint_ref, &parent_checkpoint),
                ControlTransactionPreparation::new(
                    selected_store.clone(),
                    pale_core::records::ControlTransactionSequenceV1::new(2)
                        .expect("child transaction"),
                )
                .expect("selected child transaction"),
                parents.current.now,
            );
            assert!(matches!(
                started.start_child(
                    &mut host,
                    ResidentDeploymentOperationAccess {
                        store: &selected_store,
                        custody: &mut parent_custody,
                        sessions: &selected_port,
                        anchor: &mut parent_anchor,
                    },
                    &mut authority,
                    invoke_start_input(
                        overflow_semantic,
                        &checkpoint_schema,
                        checkpoint_authority,
                    ),
                    overflow_parent,
                    |_, _, _, _, _, _, _, _| panic!("bound exhaustion precedes authority"),
                    |_, _, _| panic!("bound exhaustion precedes handle signing"),
                    |_, _, _| panic!("bound exhaustion precedes observation signing"),
                    move |_| Ok(overflow_checkpoint_template),
                    |_, _, _| panic!("bound exhaustion precedes checkpoint signing"),
                ),
                Err(ResidentProcessConsultationChildStartError::BoundExceeded)
            ));
            assert_eq!(parent_custody.mutation_counts(), (1, 0));
            assert_eq!(parent_anchor.access_counts(), (1, 1));

            let mut successor_template = local_operation_start_fixture::checkpoint_preimage(
                2,
                Some((
                    parent_checkpoint_ref.clone(),
                    parent_checkpoint
                        .canonical_control_checkpoint_sha256()
                        .clone(),
                )),
            );
            successor_template.checkpoint_domain_id =
                parent_checkpoint.checkpoint_domain_id().clone();
            successor_template.checkpoint_profile_ref =
                parent_checkpoint.checkpoint_profile_ref().clone();
            successor_template.checkpoint_profile_sha256 =
                parent_checkpoint.checkpoint_profile_sha256().clone();
            successor_template.domain_epoch = *parent_checkpoint.domain_epoch();
            let collection_authority =
                local_operation_start_fixture::checkpoint_authority(&successor_template);
            let collection_checkpoint_schema = local_operation_start_fixture::schema_digest(
                pale_core::records::CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID,
            );
            let mut collection_custody = CollectionCommitPort {
                prepare: prepare_collection_accumulator,
                transaction: pale_core::records::ControlTransactionSequenceV1::new(2)
                    .expect("successor transaction"),
                physical: RecordingStore::committing(
                    pale_ports::OperationCustodyCommitOutcome::Committed,
                ),
            };
            let parent_observation_ref = initial_record.coordinate().record_ref().clone();
            let collection_input =
                |collection, suffix: &str| ResidentProcessConsultationCollectionInput {
                    collection,
                    parent_observation: (&parent_observation_ref, &parent_observation),
                    parent_checkpoint: (&parent_checkpoint_ref, &parent_checkpoint),
                    observation: ConsultationCollectionObservationInput {
                        observation_ref: local_operation_start_fixture::record_ref(&format!(
                            "observation/process-consult-journey-{suffix}"
                        )),
                        predecessor_collection: None,
                        observed_at: UnixTimeV1::new(63).expect("observation time"),
                        signing: SignedResidentAdmissionContextV1 {
                            expected_schema_sha256: &observation_schema,
                            signing_authority: parents.signing,
                        },
                    },
                    limits: collection_limits,
                    checkpoint: ConsultationSuccessorCommitInput {
                        accumulator_limits: pale_ports::AccumulatorPreparationLimits::new(
                            4, 4096, 4096, 512, 65_536,
                        )
                        .expect("collection accumulator"),
                        identity: OperationCheckpointInput {
                            reference: local_operation_start_fixture::record_ref(&format!(
                                "operation/process-consult-journey-{suffix}"
                            )),
                            id: ControlCheckpointIdV1::parse(format!(
                                "process-consult-journey-{suffix}"
                            ))
                            .expect("checkpoint identity"),
                        },
                        issued_at: UnixTimeV1::new(64).expect("checkpoint time"),
                        schema: &collection_checkpoint_schema,
                        authority: &collection_authority,
                    },
                };

            let mut untouched_anchor = crate::resident::operation_observation_custody::tests::
                local_start::successor_anchor();
            let omission_failure = match started.open_collection(
                &mut host,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut collection_custody,
                    sessions: &selected_port,
                    anchor: &mut untouched_anchor,
                },
                &mut authority,
                collection_input(omitted_collection, "omission"),
                &crate::resident::operation_observation_custody::tests::local_start::StartStore(
                    parents.session,
                ),
                |_| panic!("child omission stops before collection admission"),
                |_, _, _, _| Ok(()),
                |_, _, _| Ok(local_operation_start_fixture::signature()),
                |_, _, _| Ok(local_operation_start_fixture::signature()),
            ) {
                Ok(_) => panic!("omitted durable child cannot match the started phase"),
                Err(failure) => failure,
            };
            assert!(matches!(
                &omission_failure.reason,
                ResidentProcessConsultationOpenJourneyError::ChildAssociation
            ));
            assert_eq!(collection_custody.physical.mutation_counts(), (0, 0));
            assert_eq!(untouched_anchor.access_counts(), (0, 0));
            started = *omission_failure.started;

            let association_failure = match started.open_collection(
                &mut host,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut collection_custody,
                    sessions: &selected_port,
                    anchor: &mut untouched_anchor,
                },
                &mut authority,
                collection_input(substituted_collection, "substitution"),
                &crate::resident::operation_observation_custody::tests::local_start::StartStore(
                    parents.session,
                ),
                |_| panic!("child substitution stops before collection admission"),
                |_, _, _, _| Ok(()),
                |_, _, _| Ok(local_operation_start_fixture::signature()),
                |_, _, _| Ok(local_operation_start_fixture::signature()),
            ) {
                Ok(_) => panic!("reordered children cannot match the started phase"),
                Err(failure) => failure,
            };
            assert!(matches!(
                &association_failure.reason,
                ResidentProcessConsultationOpenJourneyError::ChildAssociation
            ));
            assert_eq!(collection_custody.physical.mutation_counts(), (0, 0));
            assert_eq!(untouched_anchor.access_counts(), (0, 0));
            started = *association_failure.started;

            let mut cross_deployment_anchor = crate::resident::operation_observation_custody::tests::
                local_start::successor_anchor();
            let cross_deployment = match started.open_collection(
                &mut host,
                ResidentDeploymentOperationAccess {
                    store: &other_store,
                    custody: &mut collection_custody,
                    sessions: &other_port,
                    anchor: &mut cross_deployment_anchor,
                },
                &mut authority,
                collection_input(collection.clone(), "cross-deployment"),
                &crate::resident::operation_observation_custody::tests::local_start::StartStore(
                    parents.session,
                ),
                |_| panic!("cross-deployment input stops before admission"),
                |_, _, _, _| Ok(()),
                |_, _, _| Ok(local_operation_start_fixture::signature()),
                |_, _, _| Ok(local_operation_start_fixture::signature()),
            ) {
                Ok(_) => panic!("another deployment cannot receive this collection"),
                Err(failure) => failure,
            };
            assert!(matches!(
                &cross_deployment.reason,
                ResidentProcessConsultationOpenJourneyError::Open(
                    ResidentProcessConsultationOpenError::Selection(
                        ResidentProcessHostSelectionError::AccessStoreAssociation
                    )
                )
            ));
            assert_eq!(collection_custody.physical.mutation_counts(), (0, 0));
            assert_eq!(cross_deployment_anchor.access_counts(), (0, 0));
            started = *cross_deployment.started;

            let collection_pending = match started.into_initialization() {
                ResidentProcessConsultationInitialization::OpenCollection(pending) => pending,
                ResidentProcessConsultationInitialization::StartChild(_) => {
                    panic!("all retained children must precede collection custody")
                }
            };
            let mut unavailable_anchor = RecordingAnchor::unavailable_after_preflight();
            let interrupted = match collection_pending.open(
                &mut host,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut collection_custody,
                    sessions: &selected_port,
                    anchor: &mut unavailable_anchor,
                },
                &mut authority,
                collection_input(collection, "open"),
                &crate::resident::operation_observation_custody::tests::local_start::StartStore(
                    parents.session,
                ),
                |admitted| {
                    assert_eq!(admitted.slots().len(), 2);
                    Ok(())
                },
                |_, _, _, _| Ok(()),
                |_, _, _| Ok(local_operation_start_fixture::signature()),
                |_, _, _| Ok(local_operation_start_fixture::signature()),
            ) {
                Ok(_) => panic!("unavailable anchor cannot complete collection custody"),
                Err(failure) => failure,
            };
            assert!(matches!(
                &interrupted.reason,
                ResidentProcessConsultationOpenJourneyError::Open(
                    ResidentProcessConsultationOpenError::Custody(_)
                )
            ));
            assert_eq!(collection_custody.physical.mutation_counts(), (1, 1));
            let mut continuation_anchor = crate::resident::operation_observation_custody::tests::
                local_start::successor_anchor();
            let opened = interrupted
                .continue_custody(
                    &mut host,
                    ResidentDeploymentOperationAccess {
                        store: &selected_store,
                        custody: &mut collection_custody,
                        sessions: &selected_port,
                        anchor: &mut continuation_anchor,
                    },
                    &mut authority,
                )
                .unwrap_or_else(|_| panic!("exact collection custody continues without replay"));

            let frontier = opened.completed().journal().frontier();
            let frontier_evidence = OperationCollectionFrontierEvidence::admit(
                frontier,
                &selected_store,
                opened.completed().journal().operation_handle(),
                4096,
            )
            .expect("frontier evidence");
            let snapshot = OperationCollectionSnapshot::new(
                frontier.clone(),
                &frontier_evidence,
                OperationCollectionHeadObservation::for_handle(
                    opened.completed().journal().operation_handle(),
                    frontier.coordinate().clone(),
                ),
                opened
                    .completed()
                    .journal()
                    .append()
                    .entry()
                    .bytes()
                    .clone(),
            )
            .expect("collection snapshot");
            let open_observation = (
                opened.completed().observation().0.clone(),
                opened.completed().observation().1.clone(),
            );
            let open_checkpoint = (
                opened.completed().checkpoint().0.clone(),
                opened.completed().checkpoint().1.clone(),
            );
            let protected = RecoveredOperationObservation::fixture(
                opened.parent(),
                open_observation.clone(),
                open_observation,
                open_checkpoint,
                selected_store.clone(),
            );
            let mut recovery_custody = CollectionRecoveryOnly(Some(snapshot));
            let mut recovery_anchor =
                crate::resident::operation_observation_custody::tests::local_start::successor_anchor();
            let recovered = opened
                .recover(
                    &mut host,
                    ResidentDeploymentOperationAccess {
                        store: &selected_store,
                        custody: &mut recovery_custody,
                        sessions: &selected_port,
                        anchor: &mut recovery_anchor,
                    },
                    &mut authority,
                    &protected,
                    collection_limits,
                    &crate::resident::operation_observation_custody::tests::local_start::StartStore(
                        parents.session,
                    ),
                    |admitted| {
                        assert_eq!(admitted.slots().len(), 2);
                        Ok(())
                    },
                )
                .unwrap_or_else(|_| panic!("exact collection recovers"));
            assert!(recovered.current().collection().slots().iter().all(|slot| {
                matches!(
                    slot.disposition(),
                    crate::resident::consultation::ConsultationSlotDisposition::Outstanding
                )
            }));
            let ready = recovered
                .open_initial_batch(
                    &mut host,
                    UnixTimeV1::new(65).expect("batch time"),
                    ResidentConsultationTaskLimits::new(
                        NonZeroUsize::new(1).expect("one in-flight child"),
                        NonZeroUsize::new(2).expect("two batch members"),
                    ),
                )
                .unwrap_or_else(|_| panic!("finite initial batch is retained"));

            assert_eq!(ready.children().len(), 2);
            assert_eq!(ready.current().collection().slots().len(), 2);
            assert_eq!(
                ready.coordinate().parent(),
                ready.current().collection().parent()
            );
            assert_eq!(host.operations.active_consultation_batches(), 1);
            assert_eq!(host.operations.in_flight_consultation_tasks(), 0);
            assert_eq!(parent_custody.mutation_counts(), (1, 0));
            assert_eq!(parent_anchor.access_counts(), (1, 1));
            assert_eq!(owner_calls.get(), 1);
            assert_eq!(signatures.get(), 3);
        },
    );
}

#[test]
fn consult_parent_rejects_an_unselected_deployment_before_start_io() {
    let (generation, profile, host_generation) = source_generation_with_host_copy();
    crate::local_operation_start_fixture::with_operation_start_input_at_store(
        &generation,
        &profile,
        "process-host-consult-selector",
        |_| {},
        OperationClassV1::Consult,
        &crate::local_operation_start_fixture::store_coordinate(),
        |semantic, request, checkpoint_template, parents| {
            let selected_store = crate::local_operation_start_fixture::store_coordinate();
            let other_store = store("other-consult-store", "other-consult-incarnation");
            let selector = selector_from_signing_authority(parents.signing);
            let mut substituted = selector.clone();
            substituted.runtime_authority_ref =
                local_operation_start_fixture::record_ref("runtime-authority/substituted-consult");
            let selected_port = MemoryAuthorityStore::default();
            let other_port = MemoryAuthorityStore::default();
            let mut host = host_for_start(
                &selected_port,
                &other_port,
                &selector,
                &selected_store,
                &other_store,
                host_generation,
            );
            let mut custody = absent_start_store(request).with_preparation(
                initial_response,
                pale_core::records::ControlTransactionSequenceV1::new(1)
                    .expect("initial transaction"),
            );
            let mut anchor = initial_anchor();
            let mut authority = CountingAuthority::default();
            let owner_calls = Cell::new(0);
            let signatures = Cell::new(0);
            let checkpoint_schema = checkpoint_template.schema_sha256.clone();
            let checkpoint_authority = semantic.checkpoint_authority;
            let result = host.start_consult(
                &substituted,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut custody,
                    sessions: &selected_port,
                    anchor: &mut anchor,
                },
                &mut authority,
                invoke_start_input(semantic, &checkpoint_schema, checkpoint_authority),
                None,
                |_, _, _, _, _, _, _, _| {
                    owner_calls.set(owner_calls.get() + 1);
                    Ok(())
                },
                |_, _, _| {
                    signatures.set(signatures.get() + 1);
                    Ok(local_operation_start_fixture::signature())
                },
                |_, _, _| {
                    signatures.set(signatures.get() + 1);
                    Ok(local_operation_start_fixture::signature())
                },
                move |prepared| checkpoint_preimage(prepared, checkpoint_template),
                |_, _, _| {
                    signatures.set(signatures.get() + 1);
                    Ok(local_operation_start_fixture::signature())
                },
            );
            assert!(matches!(
                result,
                Err(ResidentProcessConsultStartError::Selection(
                    ResidentProcessHostSelectionError::UnknownDeploymentAuthority
                ))
            ));
            assert_no_start_io(&authority, &owner_calls, &signatures, &custody, &anchor);
        },
    );
}

#[test]
fn terminal_preparation_failure_never_enters_physical_or_task_custody() {
    let (generation, profile, host_generation) = source_generation_with_host_copy();
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        "process-host-start-terminal-preparation",
        |_| {},
        |semantic, request, checkpoint_template, parents| {
            let selected_store = crate::local_operation_start_fixture::store_coordinate();
            let other_store = store("other-start-store", "other-start-incarnation");
            let selector = selector_from_signing_authority(parents.signing);
            let selected_port = MemoryAuthorityStore::default();
            let other_port = MemoryAuthorityStore::default();
            let mut host = host_for_start(
                &selected_port,
                &other_port,
                &selector,
                &selected_store,
                &other_store,
                host_generation,
            );
            let mut custody = absent_start_store(request).with_preparation(
                initial_response,
                pale_core::records::ControlTransactionSequenceV1::new(1)
                    .expect("initial transaction"),
            );
            let mut anchor = initial_anchor();
            let mut authority = CountingAuthority::default();
            let owner_calls = Cell::new(0);
            let signatures = Cell::new(0);
            let checkpoint_schema = checkpoint_template.schema_sha256.clone();
            let checkpoint_authority = semantic.checkpoint_authority;

            let result = host.start_invoke_test_only(
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut custody,
                    sessions: &selected_port,
                    anchor: &mut anchor,
                },
                &mut authority,
                invoke_start_input(semantic, &checkpoint_schema, checkpoint_authority),
                |_, _, _, _, _, _, _, _| {
                    owner_calls.set(owner_calls.get() + 1);
                    Err(InvalidResidentRecordV1::ContextualAdmission {
                        record: "process-host start fixture",
                        invariant: "terminal owner rejection",
                    })
                },
                |_, _, _| {
                    signatures.set(signatures.get() + 1);
                    Ok(local_operation_start_fixture::signature())
                },
                |_, _, _| {
                    signatures.set(signatures.get() + 1);
                    Ok(local_operation_start_fixture::signature())
                },
                move |prepared| checkpoint_preimage(prepared, checkpoint_template),
                |_, _, _| {
                    signatures.set(signatures.get() + 1);
                    Ok(local_operation_start_fixture::signature())
                },
            );

            assert!(matches!(
                result,
                Err(ResidentProcessInvokeStartError::Preparation(
                    crate::resident::LocalOperationStartError::Admission(
                        InvalidResidentRecordV1::ContextualAdmission { .. }
                    )
                ))
            ));
            assert!(
                authority.calls > 0,
                "session revalidation precedes the owner check"
            );
            assert_eq!(owner_calls.get(), 1);
            assert_eq!(signatures.get(), 0);
            assert_eq!(custody.mutation_counts(), (0, 0));
            assert_eq!(anchor.access_counts(), (0, 0));
            assert_eq!(selected_port.entry_count(), 0);
            assert_eq!(other_port.entry_count(), 0);
            assert_eq!(host.provider_routes.reserved, 0);
            assert_eq!(host.provider_routes.checked_out, 0);
            assert!(host.provider_routes.active.is_empty());
        },
    );
}

#[test]
fn selector_and_access_substitutions_stop_before_start_io() {
    let (generation, profile, host_generation) = source_generation_with_host_copy();
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        "process-host-start-fence",
        |_| {},
        |semantic, request, checkpoint_template, parents| {
            let selected_store = crate::local_operation_start_fixture::store_coordinate();
            let other_store = store("other-start-store", "other-start-incarnation");
            let selector = selector_from_signing_authority(parents.signing);
            let selected_port = MemoryAuthorityStore::default();
            let other_port = MemoryAuthorityStore::default();
            let mut host = host_for_start(
                &selected_port,
                &other_port,
                &selector,
                &selected_store,
                &other_store,
                host_generation,
            );
            let mut custody = absent_start_store(request).with_preparation(
                initial_response,
                pale_core::records::ControlTransactionSequenceV1::new(1)
                    .expect("initial transaction"),
            );
            let mut anchor = initial_anchor();
            let mut authority = CountingAuthority::default();
            let owner_calls = Cell::new(0);
            let signatures = Cell::new(0);
            let checkpoint_schema = checkpoint_template.schema_sha256.clone();
            let checkpoint_authority = semantic.checkpoint_authority;
            let mut substituted = selector.clone();
            substituted.runtime_authority_ref =
                ResidentRecordRefV1::parse("runtime/foreign").expect("foreign runtime");
            let result = host.start_invoke_test_only(
                &substituted,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut custody,
                    sessions: &selected_port,
                    anchor: &mut anchor,
                },
                &mut authority,
                ResidentLocalOperationStartInput {
                    semantic,
                    accumulator_limits: pale_ports::AccumulatorPreparationLimits::new(
                        4, 1024, 1024, 512, 65_536,
                    )
                    .expect("bounded accumulator"),
                    checkpoint: ResidentLocalOperationCheckpointInput {
                        id: checkpoint_template.control_checkpoint_id.clone(),
                        reference: local_operation_start_fixture::record_ref(
                            "process-host/start/fenced-checkpoint",
                        ),
                        schema: &checkpoint_schema,
                        authority: checkpoint_authority,
                    },
                    maximum_record_bytes: 1 << 20,
                },
                |_, _, _, _, _, _, _, _| {
                    owner_calls.set(owner_calls.get() + 1);
                    Ok(())
                },
                |_, _, _| {
                    signatures.set(signatures.get() + 1);
                    Ok(local_operation_start_fixture::signature())
                },
                |_, _, _| {
                    signatures.set(signatures.get() + 1);
                    Ok(local_operation_start_fixture::signature())
                },
                move |prepared| checkpoint_preimage(prepared, checkpoint_template),
                |_, _, _| {
                    signatures.set(signatures.get() + 1);
                    Ok(local_operation_start_fixture::signature())
                },
            );
            assert!(matches!(
                result,
                Err(ResidentProcessInvokeStartError::Selection(
                    ResidentProcessHostSelectionError::UnknownDeploymentAuthority
                ))
            ));
            assert_eq!(authority.calls, 0);
            assert_eq!(owner_calls.get(), 0);
            assert_eq!(signatures.get(), 0);
            assert_eq!(custody.mutation_counts(), (0, 0));
            assert_eq!(anchor.access_counts(), (0, 0));
        },
    );
}

#[test]
fn every_deployment_selector_axis_stops_before_start_io() {
    assert_selector_substitution_stops_before_start_io(
        "process-host-start-foreign-principal",
        |selector| selector.principal_id = principal("principal-foreign"),
    );
    assert_selector_substitution_stops_before_start_io(
        "process-host-start-foreign-binding-ref",
        |selector| {
            selector.deployment_binding_ref =
                ResidentRecordRefV1::parse("binding/foreign").expect("foreign binding")
        },
    );
    assert_selector_substitution_stops_before_start_io(
        "process-host-start-foreign-binding-digest",
        |selector| {
            selector.deployment_binding_sha256 =
                DeploymentBindingDigest::hash_canonical("binding/foreign")
                    .expect("foreign binding digest")
        },
    );
    assert_selector_substitution_stops_before_start_io(
        "process-host-start-foreign-runtime-ref",
        |selector| {
            selector.runtime_authority_ref =
                ResidentRecordRefV1::parse("authority/foreign").expect("foreign authority")
        },
    );
    assert_selector_substitution_stops_before_start_io(
        "process-host-start-foreign-runtime-digest",
        |selector| {
            selector.runtime_authority_sha256 =
                DeploymentRuntimeAuthorityDigest::hash_canonical("authority/foreign")
                    .expect("foreign authority digest")
        },
    );
}

#[test]
fn handle_issuer_key_and_epoch_stop_before_start_io() {
    assert_handle_issuer_substitution_stops_before_start_io(
        "process-host-start-foreign-handle-key",
        |handle| {
            handle.issuer_key_id =
                ResidentKeyIdV1::parse("runtime-key-foreign").expect("foreign issuer key")
        },
    );
    assert_handle_issuer_substitution_stops_before_start_io(
        "process-host-start-foreign-handle-key-epoch",
        |handle| {
            handle.issuer_key_epoch = SigningKeyEpochV1::new(2).expect("foreign issuer key epoch")
        },
    );
}

#[test]
fn access_store_substitution_stops_before_start_io() {
    let (generation, profile, host_generation) = source_generation_with_host_copy();
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        "process-host-start-access-fence",
        |_| {},
        |semantic, request, checkpoint_template, parents| {
            let selected_store = crate::local_operation_start_fixture::store_coordinate();
            let other_store = store("other-start-store", "other-start-incarnation");
            let selector = selector_from_signing_authority(parents.signing);
            let selected_port = MemoryAuthorityStore::default();
            let other_port = MemoryAuthorityStore::default();
            let mut host = host_for_start(
                &selected_port,
                &other_port,
                &selector,
                &selected_store,
                &other_store,
                host_generation,
            );
            let mut custody = absent_start_store(request).with_preparation(
                initial_response,
                pale_core::records::ControlTransactionSequenceV1::new(1)
                    .expect("initial transaction"),
            );
            let mut anchor = initial_anchor();
            let mut authority = CountingAuthority::default();
            let owner_calls = Cell::new(0);
            let signatures = Cell::new(0);
            let checkpoint_authority = semantic.checkpoint_authority;
            let result = attempt_start(
                &mut host,
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &other_store,
                    custody: &mut custody,
                    sessions: &selected_port,
                    anchor: &mut anchor,
                },
                &mut authority,
                semantic,
                checkpoint_authority,
                checkpoint_template,
                "process-host/start/access-fence-checkpoint",
                &owner_calls,
                &signatures,
            );
            assert!(matches!(
                result,
                Err(ResidentProcessInvokeStartError::Selection(
                    ResidentProcessHostSelectionError::AccessStoreAssociation
                ))
            ));
            assert_no_start_io(&authority, &owner_calls, &signatures, &custody, &anchor);
        },
    );
}

#[test]
fn request_handle_and_checkpoint_substitutions_fail_exact_start_preflight() {
    let (generation, profile, host_generation) = source_generation_with_host_copy();
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        "process-host-start-owner-fences",
        |_| {},
        |semantic, request, checkpoint_template, parents| {
            let selected_store = crate::local_operation_start_fixture::store_coordinate();
            let other_store = store("other-start-store", "other-start-incarnation");
            let selector = selector_from_signing_authority(parents.signing);
            let selected_port = MemoryAuthorityStore::default();
            let other_port = MemoryAuthorityStore::default();
            let host = host_for_start(
                &selected_port,
                &other_port,
                &selector,
                &selected_store,
                &other_store,
                host_generation,
            );
            let checkpoint_schema = checkpoint_template.schema_sha256.clone();

            let foreign_request = request_with_signing_authority(
                request,
                owner_native_signing_authority(selector.principal_id.clone()),
            );
            let mut request_substitution = copy_semantic_start(&semantic);
            request_substitution.request = &foreign_request;
            assert!(matches!(
                host.validate_operation_start_associations(
                    &selector,
                    &selected_store,
                    &invoke_start_input(
                        request_substitution,
                        &checkpoint_schema,
                        semantic.checkpoint_authority,
                    ),
                ),
                Err(ResidentProcessHostSelectionError::CurrentRequestAuthority)
            ));

            let mut foreign_selector = selector.clone();
            foreign_selector.runtime_authority_ref =
                ResidentRecordRefV1::parse("authority/foreign-handle-signer")
                    .expect("foreign handle signer");
            let foreign_signing = integrated_signing_authority(&foreign_selector);
            let foreign_context = pale_core::records::OperationHandleAdmissionContextV1 {
                signed: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: semantic.context.signed.expected_schema_sha256,
                    signing_authority: &foreign_signing,
                },
                ..*semantic.context
            };
            let mut handle_signer_substitution = copy_semantic_start(&semantic);
            handle_signer_substitution.context = &foreign_context;
            assert!(matches!(
                host.validate_operation_start_associations(
                    &selector,
                    &selected_store,
                    &invoke_start_input(
                        handle_signer_substitution,
                        &checkpoint_schema,
                        semantic.checkpoint_authority,
                    ),
                ),
                Err(ResidentProcessHostSelectionError::StartHandleAuthority)
            ));

            let mut handle_preimage_substitution = copy_semantic_start(&semantic);
            handle_preimage_substitution
                .handle
                .deployment_runtime_authority_ref =
                ResidentRecordRefV1::parse("authority/foreign-handle-preimage")
                    .expect("foreign handle preimage authority");
            assert!(matches!(
                host.validate_operation_start_associations(
                    &selector,
                    &selected_store,
                    &invoke_start_input(
                        handle_preimage_substitution,
                        &checkpoint_schema,
                        semantic.checkpoint_authority,
                    ),
                ),
                Err(ResidentProcessHostSelectionError::StartHandlePreimageAuthority)
            ));

            let mut foreign_checkpoint_preimage = checkpoint_template.clone();
            foreign_checkpoint_preimage.checkpoint_signing_authority_ref =
                ResidentRecordRefV1::parse("checkpoint-authority/foreign")
                    .expect("foreign checkpoint authority");
            let foreign_checkpoint =
                local_operation_start_fixture::checkpoint_authority(&foreign_checkpoint_preimage);
            assert!(matches!(
                host.validate_operation_start_associations(
                    &selector,
                    &selected_store,
                    &invoke_start_input(
                        copy_semantic_start(&semantic),
                        &checkpoint_schema,
                        &foreign_checkpoint,
                    ),
                ),
                Err(ResidentProcessHostSelectionError::StartCheckpointAuthority)
            ));
        },
    );
}

#[test]
fn current_and_admitted_session_substitutions_fail_exact_start_preflight() {
    let (generation, profile, host_generation) = source_generation_with_host_copy();
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        "process-host-start-session-fences",
        |_| {},
        |semantic, _, checkpoint_template, parents| {
            let selected_store = crate::local_operation_start_fixture::store_coordinate();
            let other_store = store("other-start-store", "other-start-incarnation");
            let selector = selector_from_signing_authority(parents.signing);
            let selected_port = MemoryAuthorityStore::default();
            let other_port = MemoryAuthorityStore::default();
            let host = host_for_start(
                &selected_port,
                &other_port,
                &selector,
                &selected_store,
                &other_store,
                host_generation,
            );
            let checkpoint_schema = checkpoint_template.schema_sha256.clone();

            let foreign_signing = owner_native_signing_authority(selector.principal_id.clone());
            let current = semantic.current;
            let foreign_current = crate::resident::AmbientSessionAdmissionContext {
                now: current.now,
                subject: current.subject,
                grant: current.grant,
                current_principal_trust: current.current_principal_trust,
                capability_revocations: current.capability_revocations,
                deployment_binding_ref: current.deployment_binding_ref,
                deployment_binding: current.deployment_binding,
                deployment_revocations: current.deployment_revocations,
                runtime_authority: current.runtime_authority,
                session_revocations: current.session_revocations,
                route_use: current.route_use,
                generation: pale_core::records::SessionAdmissionLeaseContextV1 {
                    active_head: current.generation.active_head,
                    compiled_generation: current.generation.compiled_generation,
                    effective_profile: current.generation.effective_profile,
                    activation_fence: current.generation.activation_fence,
                    signed: SignedResidentAdmissionContextV1 {
                        expected_schema_sha256: current.generation.signed.expected_schema_sha256,
                        signing_authority: &foreign_signing,
                    },
                },
            };
            let mut current_substitution = copy_semantic_start(&semantic);
            current_substitution.current = &foreign_current;
            assert!(matches!(
                host.validate_operation_start_associations(
                    &selector,
                    &selected_store,
                    &invoke_start_input(
                        current_substitution,
                        &checkpoint_schema,
                        semantic.checkpoint_authority,
                    ),
                ),
                Err(ResidentProcessHostSelectionError::StartCurrentAuthority)
            ));

            crate::local_operation_start_fixture::with_invoke_start_input_at_store(
                &generation,
                &profile,
                "process-host-start-foreign-admitted-session",
                |_| {},
                &other_store,
                |foreign, _, _, _| {
                    let mut session_substitution = copy_semantic_start(&semantic);
                    session_substitution.session = foreign.session;
                    assert!(matches!(
                        host.validate_operation_start_associations(
                            &selector,
                            &selected_store,
                            &invoke_start_input(
                                session_substitution,
                                &checkpoint_schema,
                                semantic.checkpoint_authority,
                            ),
                        ),
                        Err(ResidentProcessHostSelectionError::StartSessionAuthority)
                    ));
                },
            );
        },
    );
}

#[test]
fn checkpoint_authority_comparison_covers_every_witness_axis() {
    let baseline = local_operation_start_fixture::checkpoint_preimage(1, None);
    let baseline_authority = local_operation_start_fixture::checkpoint_authority(&baseline);
    let mut substitutions = Vec::new();

    let mut value = baseline.clone();
    value.checkpoint_signing_authority_ref =
        ResidentRecordRefV1::parse("checkpoint-authority/foreign").expect("authority reference");
    substitutions.push(value);
    let mut value = baseline.clone();
    value.checkpoint_signing_authority_sha256 =
        pale_core::records::CheckpointSigningAuthorityDigest::parse("2".repeat(64))
            .expect("authority digest");
    substitutions.push(value);
    let mut value = baseline.clone();
    value.issuer_principal_id = principal("principal-foreign");
    substitutions.push(value);
    let mut value = baseline.clone();
    value.control_store_id = ControlStoreIdV1::parse("control-store-foreign").expect("store");
    substitutions.push(value);
    let mut value = baseline.clone();
    value.control_incarnation_id =
        ControlIncarnationIdV1::parse("incarnation-foreign").expect("incarnation");
    substitutions.push(value);
    let mut value = baseline.clone();
    value.checkpoint_domain_id =
        pale_core::records::ControlCheckpointDomainIdV1::parse("operation:foreign")
            .expect("checkpoint domain");
    substitutions.push(value);
    let mut value = baseline.clone();
    value.checkpoint_profile_ref =
        ResidentRecordRefV1::parse("checkpoint-profile/foreign").expect("profile reference");
    substitutions.push(value);
    let mut value = baseline.clone();
    value.checkpoint_profile_sha256 =
        pale_core::records::CheckpointProfileDigest::parse("3".repeat(64)).expect("profile digest");
    substitutions.push(value);
    let mut value = baseline.clone();
    value.issuer_key_id = ResidentKeyIdV1::parse("key-foreign").expect("key");
    substitutions.push(value);
    let mut value = baseline.clone();
    value.issuer_key_epoch = SigningKeyEpochV1::new(2).expect("key epoch");
    substitutions.push(value);

    for substitution in &substitutions {
        assert!(!same_checkpoint_authority(
            &baseline_authority,
            &local_operation_start_fixture::checkpoint_authority(substitution),
        ));
    }
}

#[test]
fn anchor_retry_preserves_exact_start_and_rejects_foreign_host_or_access() {
    let (generation, profile, host_generation) = source_generation_with_host_copy();
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        "process-host-start-anchor-retry",
        |_| {},
        |semantic, request, checkpoint_template, parents| {
            let selected_store = crate::local_operation_start_fixture::store_coordinate();
            let other_store = store("other-start-store", "other-start-incarnation");
            let selector = selector_from_signing_authority(parents.signing);
            let selected_port = MemoryAuthorityStore::default();
            let other_port = MemoryAuthorityStore::default();
            let mut host = host_for_start(
                &selected_port,
                &other_port,
                &selector,
                &selected_store,
                &other_store,
                host_generation,
            );
            let mut custody = absent_start_store(request).with_preparation(
                initial_response,
                pale_core::records::ControlTransactionSequenceV1::new(1)
                    .expect("initial transaction"),
            );
            let mut anchor = RecordingAnchor::unavailable_initial_after_preflight();
            let mut authority = CountingAuthority::default();
            let signatures = Cell::new(0);
            let checkpoint_schema = checkpoint_template.schema_sha256.clone();
            let checkpoint_authority = semantic.checkpoint_authority;
            let pending = match host.start_invoke_test_only(
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut custody,
                    sessions: &selected_port,
                    anchor: &mut anchor,
                },
                &mut authority,
                ResidentLocalOperationStartInput {
                    semantic,
                    accumulator_limits: pale_ports::AccumulatorPreparationLimits::new(
                        4, 1024, 1024, 512, 65_536,
                    )
                    .expect("bounded accumulator"),
                    checkpoint: ResidentLocalOperationCheckpointInput {
                        id: checkpoint_template.control_checkpoint_id.clone(),
                        reference: local_operation_start_fixture::record_ref(
                            "process-host/start/anchor-retry-checkpoint",
                        ),
                        schema: &checkpoint_schema,
                        authority: checkpoint_authority,
                    },
                    maximum_record_bytes: 1 << 20,
                },
                |_, _, _, _, _, _, _, _| Ok(()),
                |_, _, _| {
                    signatures.set(signatures.get() + 1);
                    Ok(local_operation_start_fixture::signature())
                },
                |_, _, _| {
                    signatures.set(signatures.get() + 1);
                    Ok(local_operation_start_fixture::signature())
                },
                move |prepared| checkpoint_preimage(prepared, checkpoint_template),
                |_, _, _| {
                    signatures.set(signatures.get() + 1);
                    Ok(local_operation_start_fixture::signature())
                },
            ) {
                Err(ResidentProcessInvokeStartError::Continuation(pending)) => *pending,
                _ => panic!("unavailable anchor retains exact start custody"),
            };
            assert_eq!(custody.mutation_counts(), (1, 0));
            assert_eq!(anchor.access_counts(), (1, 1));
            assert_eq!(signatures.get(), 3);

            let pending = match host.continue_operation_start(
                pending,
                ResidentDeploymentOperationAccess {
                    store: &other_store,
                    custody: &mut custody,
                    sessions: &selected_port,
                    anchor: &mut anchor,
                },
                &mut authority,
            ) {
                Err(failure) => {
                    assert!(matches!(
                        failure.reason,
                        ResidentProcessOperationStartContinuationError::Selection(
                            ResidentProcessHostSelectionError::AccessStoreAssociation
                        )
                    ));
                    assert_eq!(failure.pending.selector, selector);
                    assert_eq!(failure.pending.maximum_record_bytes, 1 << 20);
                    failure.pending
                }
                Ok(_) => panic!("foreign access cannot continue custody"),
            };
            assert_eq!(custody.mutation_counts(), (1, 0));
            assert_eq!(anchor.access_counts(), (1, 1));

            let rebuilt_selected_port = MemoryAuthorityStore::default();
            let rebuilt_other_port = MemoryAuthorityStore::default();
            let mut rebuilt = host_for_start_retaining_generation(
                &rebuilt_selected_port,
                &rebuilt_other_port,
                &selector,
                &selected_store,
                &other_store,
                None,
            );
            let pending = match rebuilt.continue_operation_start(
                *pending,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut custody,
                    sessions: &rebuilt_selected_port,
                    anchor: &mut anchor,
                },
                &mut authority,
            ) {
                Err(failure) => {
                    assert!(matches!(
                        failure.reason,
                        ResidentProcessOperationStartContinuationError::Selection(
                            ResidentProcessHostSelectionError::StartProcessOwner
                        )
                    ));
                    assert_eq!(failure.pending.selector, selector);
                    assert_eq!(failure.pending.maximum_record_bytes, 1 << 20);
                    failure.pending
                }
                Ok(_) => panic!("rebuilt host cannot consume process-owned continuation"),
            };
            assert_eq!(custody.mutation_counts(), (1, 0));
            assert_eq!(anchor.access_counts(), (1, 1));

            anchor.allow_next_advance();
            let issued = host
                .continue_operation_start(
                    *pending,
                    ResidentDeploymentOperationAccess {
                        store: &selected_store,
                        custody: &mut custody,
                        sessions: &selected_port,
                        anchor: &mut anchor,
                    },
                    &mut authority,
                )
                .unwrap_or_else(|_| panic!("same host advances exact retained anchor"));
            assert_eq!(
                issued.handle().0.as_str(),
                "handle/process-host-start-anchor-retry"
            );
            assert_eq!(custody.mutation_counts(), (1, 0));
            assert_eq!(anchor.access_counts(), (2, 1));
            assert_eq!(signatures.get(), 3);
            assert_eq!(selected_port.entry_count(), 0);
            assert_eq!(other_port.entry_count(), 0);
            assert_eq!(host.provider_routes.reserved, 0);
            assert_eq!(host.provider_routes.checked_out, 0);
            assert!(host.provider_routes.active.is_empty());
        },
    );
}

#[test]
fn unsafe_commit_uncertainty_retains_one_host_fenced_start_without_retry() {
    let (generation, profile, host_generation) = source_generation_with_host_copy();
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        "process-host-start-unsafe-commit",
        |_| {},
        |semantic, request, checkpoint_template, parents| {
            let selected_store = crate::local_operation_start_fixture::store_coordinate();
            let other_store = store("other-start-store", "other-start-incarnation");
            let selector = selector_from_signing_authority(parents.signing);
            let selected_port = MemoryAuthorityStore::default();
            let other_port = MemoryAuthorityStore::default();
            let mut host = host_for_start(
                &selected_port,
                &other_port,
                &selector,
                &selected_store,
                &other_store,
                host_generation,
            );
            let mut custody = absent_start_store(request).with_preparation(
                initial_response,
                pale_core::records::ControlTransactionSequenceV1::new(1)
                    .expect("initial transaction"),
            );
            custody.replace_commit_result(Err(ControlStoreError::new(
                ControlStoreErrorKind::OutcomeUnknown,
            )));
            custody.replace_read_back_results([
                Err(ControlStoreError::new(ControlStoreErrorKind::Storage)),
                Err(ControlStoreError::new(ControlStoreErrorKind::Storage)),
            ]);
            let mut anchor = initial_anchor();
            let mut authority = CountingAuthority::default();
            let owner_calls = Cell::new(0);
            let signatures = Cell::new(0);
            let checkpoint_schema = checkpoint_template.schema_sha256.clone();
            let checkpoint_authority = semantic.checkpoint_authority;
            let pending = match host.start_invoke_test_only(
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut custody,
                    sessions: &selected_port,
                    anchor: &mut anchor,
                },
                &mut authority,
                invoke_start_input(semantic, &checkpoint_schema, checkpoint_authority),
                |_, _, _, _, _, _, _, _| {
                    owner_calls.set(owner_calls.get() + 1);
                    Ok(())
                },
                |_, _, _| {
                    signatures.set(signatures.get() + 1);
                    Ok(local_operation_start_fixture::signature())
                },
                |_, _, _| {
                    signatures.set(signatures.get() + 1);
                    Ok(local_operation_start_fixture::signature())
                },
                move |prepared| checkpoint_preimage(prepared, checkpoint_template),
                |_, _, _| {
                    signatures.set(signatures.get() + 1);
                    Ok(local_operation_start_fixture::signature())
                },
            ) {
                Err(ResidentProcessInvokeStartError::Continuation(pending)) => *pending,
                _ => panic!("unclassified commit uncertainty retains exact start custody"),
            };
            assert_eq!(pending.selector, selector);
            assert_eq!(pending.maximum_record_bytes, 1 << 20);
            assert!(pending
                .owner_instance
                .is_same(host.operations.operation_task_owner_instance()));
            assert!(matches!(
                pending.failure.as_ref(),
                ResidentLocalOperationStartFailure::Custody(
                    ResidentLocalOperationCustodyFailure::Commit { failure, .. }
                )
                    if matches!(
                        failure.reason(),
                        crate::resident::OperationStartCommitFailureReason::Database(
                            crate::resident::OperationCustodyDatabaseFailureReason::ReadBackFailed { .. }
                        )
                    )
            ));
            let retained_bytes = custody
                .committed_candidate()
                .cloned()
                .expect("uncertain commit retains exact candidate bytes");
            assert_eq!(custody.mutation_counts(), (1, 1));
            assert_eq!(anchor.access_counts(), (0, 1));

            let pending = match host.continue_operation_start(
                pending,
                ResidentDeploymentOperationAccess {
                    store: &other_store,
                    custody: &mut custody,
                    sessions: &selected_port,
                    anchor: &mut anchor,
                },
                &mut authority,
            ) {
                Err(failure) => {
                    assert!(matches!(
                        failure.reason,
                        ResidentProcessOperationStartContinuationError::Selection(
                            ResidentProcessHostSelectionError::AccessStoreAssociation
                        )
                    ));
                    *failure.pending
                }
                Ok(_) => panic!("foreign access cannot consume uncertain commit custody"),
            };
            assert_eq!(custody.mutation_counts(), (1, 1));
            assert_eq!(anchor.access_counts(), (0, 1));

            let rebuilt_selected_port = MemoryAuthorityStore::default();
            let rebuilt_other_port = MemoryAuthorityStore::default();
            let mut rebuilt = host_for_start_retaining_generation(
                &rebuilt_selected_port,
                &rebuilt_other_port,
                &selector,
                &selected_store,
                &other_store,
                None,
            );
            let pending = match rebuilt.continue_operation_start(
                pending,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut custody,
                    sessions: &rebuilt_selected_port,
                    anchor: &mut anchor,
                },
                &mut authority,
            ) {
                Err(failure) => {
                    assert!(matches!(
                        failure.reason,
                        ResidentProcessOperationStartContinuationError::Selection(
                            ResidentProcessHostSelectionError::StartProcessOwner
                        )
                    ));
                    *failure.pending
                }
                Ok(_) => panic!("rebuilt host cannot consume uncertain commit custody"),
            };

            let pending = match host.continue_operation_start(
                pending,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut custody,
                    sessions: &selected_port,
                    anchor: &mut anchor,
                },
                &mut authority,
            ) {
                Err(failure) => {
                    assert!(matches!(
                        failure.reason,
                        ResidentProcessOperationStartContinuationError::Phase
                    ));
                    *failure.pending
                }
                Ok(_) => panic!("unsafe commit uncertainty cannot become retry authority"),
            };
            assert_eq!(pending.selector, selector);
            assert_eq!(pending.maximum_record_bytes, 1 << 20);
            assert!(matches!(
                pending.failure.as_ref(),
                ResidentLocalOperationStartFailure::Custody(
                    ResidentLocalOperationCustodyFailure::Commit { failure, .. }
                )
                    if matches!(
                        failure.reason(),
                        crate::resident::OperationStartCommitFailureReason::Database(
                            crate::resident::OperationCustodyDatabaseFailureReason::ReadBackFailed { .. }
                        )
                    )
            ));
            assert_eq!(custody.mutation_counts(), (1, 2));
            assert_eq!(custody.committed_candidate(), Some(&retained_bytes));
            assert_eq!(anchor.access_counts(), (0, 1));
            assert_eq!(owner_calls.get(), 1);
            assert_eq!(signatures.get(), 3);
            assert_eq!(selected_port.entry_count(), 0);
            assert_eq!(other_port.entry_count(), 0);
            assert_eq!(host.provider_routes.reserved, 0);
            assert_eq!(host.provider_routes.checked_out, 0);
            assert!(host.provider_routes.active.is_empty());
        },
    );
}

#[test]
fn unlike_anchor_retains_one_host_fenced_start_without_retry() {
    let (generation, profile, host_generation) = source_generation_with_host_copy();
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        "process-host-start-unlike-anchor",
        |_| {},
        |semantic, request, checkpoint_template, parents| {
            let selected_store = crate::local_operation_start_fixture::store_coordinate();
            let other_store = store("other-start-store", "other-start-incarnation");
            let selector = selector_from_signing_authority(parents.signing);
            let selected_port = MemoryAuthorityStore::default();
            let other_port = MemoryAuthorityStore::default();
            let mut host = host_for_start(
                &selected_port,
                &other_port,
                &selector,
                &selected_store,
                &other_store,
                host_generation,
            );
            let mut custody = absent_start_store(request).with_preparation(
                initial_response,
                pale_core::records::ControlTransactionSequenceV1::new(1)
                    .expect("initial transaction"),
            );
            let mut anchor = RecordingAnchor::conflicting_initial_after_preflight();
            let mut authority = CountingAuthority::default();
            let owner_calls = Cell::new(0);
            let signatures = Cell::new(0);
            let checkpoint_schema = checkpoint_template.schema_sha256.clone();
            let checkpoint_authority = semantic.checkpoint_authority;
            let pending = match host.start_invoke_test_only(
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut custody,
                    sessions: &selected_port,
                    anchor: &mut anchor,
                },
                &mut authority,
                invoke_start_input(semantic, &checkpoint_schema, checkpoint_authority),
                |_, _, _, _, _, _, _, _| {
                    owner_calls.set(owner_calls.get() + 1);
                    Ok(())
                },
                |_, _, _| {
                    signatures.set(signatures.get() + 1);
                    Ok(local_operation_start_fixture::signature())
                },
                |_, _, _| {
                    signatures.set(signatures.get() + 1);
                    Ok(local_operation_start_fixture::signature())
                },
                move |prepared| checkpoint_preimage(prepared, checkpoint_template),
                |_, _, _| {
                    signatures.set(signatures.get() + 1);
                    Ok(local_operation_start_fixture::signature())
                },
            ) {
                Err(ResidentProcessInvokeStartError::Continuation(pending)) => *pending,
                _ => panic!("unlike independent anchor retains exact start custody"),
            };
            assert!(matches!(
                pending.failure.as_ref(),
                ResidentLocalOperationStartFailure::Custody(
                    ResidentLocalOperationCustodyFailure::Anchor { failure, .. }
                )
                    if matches!(
                        failure.reason(),
                        crate::resident::OperationCustodyAnchorFailureReason::NotAdvanced(_)
                    )
            ));
            assert_eq!(pending.selector, selector);
            assert_eq!(pending.maximum_record_bytes, 1 << 20);
            assert!(pending
                .owner_instance
                .is_same(host.operations.operation_task_owner_instance()));
            let retained_bytes = custody
                .committed_candidate()
                .cloned()
                .expect("database commit retains exact candidate bytes");
            assert_eq!(custody.mutation_counts(), (1, 0));
            assert_eq!(anchor.access_counts(), (1, 1));

            let pending = match host.continue_operation_start(
                pending,
                ResidentDeploymentOperationAccess {
                    store: &other_store,
                    custody: &mut custody,
                    sessions: &selected_port,
                    anchor: &mut anchor,
                },
                &mut authority,
            ) {
                Err(failure) => {
                    assert!(matches!(
                        failure.reason,
                        ResidentProcessOperationStartContinuationError::Selection(
                            ResidentProcessHostSelectionError::AccessStoreAssociation
                        )
                    ));
                    *failure.pending
                }
                Ok(_) => panic!("foreign access cannot consume unlike-anchor custody"),
            };

            let rebuilt_selected_port = MemoryAuthorityStore::default();
            let rebuilt_other_port = MemoryAuthorityStore::default();
            let mut rebuilt = host_for_start_retaining_generation(
                &rebuilt_selected_port,
                &rebuilt_other_port,
                &selector,
                &selected_store,
                &other_store,
                None,
            );
            let pending = match rebuilt.continue_operation_start(
                pending,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut custody,
                    sessions: &rebuilt_selected_port,
                    anchor: &mut anchor,
                },
                &mut authority,
            ) {
                Err(failure) => {
                    assert!(matches!(
                        failure.reason,
                        ResidentProcessOperationStartContinuationError::Selection(
                            ResidentProcessHostSelectionError::StartProcessOwner
                        )
                    ));
                    *failure.pending
                }
                Ok(_) => panic!("rebuilt host cannot consume unlike-anchor custody"),
            };

            let pending = match host.continue_operation_start(
                pending,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut custody,
                    sessions: &selected_port,
                    anchor: &mut anchor,
                },
                &mut authority,
            ) {
                Err(failure) => {
                    assert!(matches!(
                        failure.reason,
                        ResidentProcessOperationStartContinuationError::Phase
                    ));
                    *failure.pending
                }
                Ok(_) => panic!("unlike anchor cannot become retry authority"),
            };
            assert_eq!(pending.selector, selector);
            assert_eq!(pending.maximum_record_bytes, 1 << 20);
            assert!(matches!(
                pending.failure.as_ref(),
                ResidentLocalOperationStartFailure::Custody(
                    ResidentLocalOperationCustodyFailure::Anchor { failure, .. }
                )
                    if matches!(
                        failure.reason(),
                        crate::resident::OperationCustodyAnchorFailureReason::NotAdvanced(_)
                    )
            ));
            assert_eq!(custody.mutation_counts(), (1, 0));
            assert_eq!(custody.committed_candidate(), Some(&retained_bytes));
            assert_eq!(anchor.access_counts(), (1, 1));
            assert_eq!(owner_calls.get(), 1);
            assert_eq!(signatures.get(), 3);
            assert_eq!(selected_port.entry_count(), 0);
            assert_eq!(other_port.entry_count(), 0);
            assert_eq!(host.provider_routes.reserved, 0);
            assert_eq!(host.provider_routes.checked_out, 0);
            assert!(host.provider_routes.active.is_empty());
        },
    );
}

#[derive(Clone, Copy)]
enum AmbiguousAnchorFailureKind {
    NotExact,
    ObservationFailed,
}

impl AmbiguousAnchorFailureKind {
    fn matches(self, failure: &ResidentLocalOperationStartFailure) -> bool {
        matches!(
            (self, failure),
            (
                Self::NotExact,
                ResidentLocalOperationStartFailure::Custody(
                    ResidentLocalOperationCustodyFailure::Anchor { failure, .. }
                )
            ) if matches!(
                failure.reason(),
                crate::resident::OperationCustodyAnchorFailureReason::AmbiguousNotExact { .. }
            )
        ) || matches!(
            (self, failure),
            (
                Self::ObservationFailed,
                ResidentLocalOperationStartFailure::Custody(
                    ResidentLocalOperationCustodyFailure::Anchor { failure, .. }
                )
            ) if matches!(
                failure.reason(),
                crate::resident::OperationCustodyAnchorFailureReason::AmbiguousObservationFailed { .. }
            )
        )
    }

    const fn final_observation_count(self) -> usize {
        match self {
            Self::NotExact => 2,
            Self::ObservationFailed => 3,
        }
    }
}

fn assert_ambiguous_anchor_failure_stays_host_fenced(
    activity: &str,
    mut anchor: RecordingAnchor,
    expected: AmbiguousAnchorFailureKind,
) {
    let (generation, profile, host_generation) = source_generation_with_host_copy();
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        activity,
        |_| {},
        |semantic, request, checkpoint_template, parents| {
            let selected_store = crate::local_operation_start_fixture::store_coordinate();
            let other_store = store("other-start-store", "other-start-incarnation");
            let selector = selector_from_signing_authority(parents.signing);
            let selected_port = MemoryAuthorityStore::default();
            let other_port = MemoryAuthorityStore::default();
            let mut host = host_for_start(
                &selected_port,
                &other_port,
                &selector,
                &selected_store,
                &other_store,
                host_generation,
            );
            let mut custody = absent_start_store(request).with_preparation(
                initial_response,
                pale_core::records::ControlTransactionSequenceV1::new(1)
                    .expect("initial transaction"),
            );
            let mut authority = CountingAuthority::default();
            let owner_calls = Cell::new(0);
            let signatures = Cell::new(0);
            let checkpoint_builds = Cell::new(0);
            let checkpoint_schema = checkpoint_template.schema_sha256.clone();
            let checkpoint_authority = semantic.checkpoint_authority;
            let pending = match host.start_invoke_test_only(
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut custody,
                    sessions: &selected_port,
                    anchor: &mut anchor,
                },
                &mut authority,
                invoke_start_input(semantic, &checkpoint_schema, checkpoint_authority),
                |_, _, _, _, _, _, _, _| {
                    owner_calls.set(owner_calls.get() + 1);
                    Ok(())
                },
                |_, _, _| {
                    signatures.set(signatures.get() + 1);
                    Ok(local_operation_start_fixture::signature())
                },
                |_, _, _| {
                    signatures.set(signatures.get() + 1);
                    Ok(local_operation_start_fixture::signature())
                },
                |prepared| {
                    checkpoint_builds.set(checkpoint_builds.get() + 1);
                    checkpoint_preimage(prepared, checkpoint_template)
                },
                |_, _, _| {
                    signatures.set(signatures.get() + 1);
                    Ok(local_operation_start_fixture::signature())
                },
            ) {
                Err(ResidentProcessInvokeStartError::Continuation(pending)) => *pending,
                _ => panic!("ambiguous anchor result retains exact start custody"),
            };
            assert!(expected.matches(pending.failure.as_ref()));
            assert_eq!(pending.selector, selector);
            assert_eq!(pending.maximum_record_bytes, 1 << 20);
            assert!(pending
                .owner_instance
                .is_same(host.operations.operation_task_owner_instance()));
            let retained_bytes = custody
                .committed_candidate()
                .cloned()
                .expect("database commit retains exact candidate bytes");
            assert_eq!(custody.mutation_counts(), (1, 0));
            assert_eq!(anchor.access_counts(), (1, 2));

            let pending = match host.continue_operation_start(
                pending,
                ResidentDeploymentOperationAccess {
                    store: &other_store,
                    custody: &mut custody,
                    sessions: &selected_port,
                    anchor: &mut anchor,
                },
                &mut authority,
            ) {
                Err(failure) => {
                    assert!(matches!(
                        failure.reason,
                        ResidentProcessOperationStartContinuationError::Selection(
                            ResidentProcessHostSelectionError::AccessStoreAssociation
                        )
                    ));
                    *failure.pending
                }
                Ok(_) => panic!("foreign access cannot consume ambiguous anchor custody"),
            };
            assert!(expected.matches(pending.failure.as_ref()));

            let rebuilt_selected_port = MemoryAuthorityStore::default();
            let rebuilt_other_port = MemoryAuthorityStore::default();
            let mut rebuilt = host_for_start_retaining_generation(
                &rebuilt_selected_port,
                &rebuilt_other_port,
                &selector,
                &selected_store,
                &other_store,
                None,
            );
            let pending = match rebuilt.continue_operation_start(
                pending,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut custody,
                    sessions: &rebuilt_selected_port,
                    anchor: &mut anchor,
                },
                &mut authority,
            ) {
                Err(failure) => {
                    assert!(matches!(
                        failure.reason,
                        ResidentProcessOperationStartContinuationError::Selection(
                            ResidentProcessHostSelectionError::StartProcessOwner
                        )
                    ));
                    *failure.pending
                }
                Ok(_) => panic!("rebuilt host cannot consume ambiguous anchor custody"),
            };
            assert!(expected.matches(pending.failure.as_ref()));

            if matches!(expected, AmbiguousAnchorFailureKind::ObservationFailed) {
                anchor.allow_unavailable_observation();
            }

            let pending = match host.continue_operation_start(
                pending,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut custody,
                    sessions: &selected_port,
                    anchor: &mut anchor,
                },
                &mut authority,
            ) {
                Err(failure) => {
                    assert!(matches!(
                        failure.reason,
                        ResidentProcessOperationStartContinuationError::Phase
                    ));
                    *failure.pending
                }
                Ok(_) => panic!("ambiguous anchor cannot become retry authority"),
            };
            assert!(expected.matches(pending.failure.as_ref()));
            assert_eq!(pending.selector, selector);
            assert_eq!(pending.maximum_record_bytes, 1 << 20);
            assert!(pending
                .owner_instance
                .is_same(host.operations.operation_task_owner_instance()));
            assert_eq!(custody.mutation_counts(), (1, 0));
            assert_eq!(custody.committed_candidate(), Some(&retained_bytes));
            assert_eq!(
                anchor.access_counts(),
                (1, expected.final_observation_count())
            );
            assert_eq!(owner_calls.get(), 1);
            assert_eq!(checkpoint_builds.get(), 1);
            assert_eq!(signatures.get(), 3);
            assert_eq!(selected_port.entry_count(), 0);
            assert_eq!(other_port.entry_count(), 0);
            assert_eq!(host.provider_routes.reserved, 0);
            assert_eq!(host.provider_routes.checked_out, 0);
            assert!(host.provider_routes.active.is_empty());
        },
    );
}

#[test]
fn ambiguous_later_anchor_retains_one_host_fenced_start_without_retry() {
    assert_ambiguous_anchor_failure_stays_host_fenced(
        "process-host-start-ambiguous-anchor",
        RecordingAnchor::ambiguous_ahead_after_preflight(),
        AmbiguousAnchorFailureKind::NotExact,
    );
}

#[test]
fn failed_ambiguity_observation_retains_one_host_fenced_start_without_retry() {
    assert_ambiguous_anchor_failure_stays_host_fenced(
        "process-host-start-ambiguous-observation-failed",
        RecordingAnchor::ambiguous_observation_failure_after_preflight(),
        AmbiguousAnchorFailureKind::ObservationFailed,
    );
}

#[test]
fn database_retry_reuses_signed_candidate_without_rebuilding() {
    let (generation, profile, host_generation) = source_generation_with_host_copy();
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        "process-host-start-database-retry",
        |_| {},
        |semantic, request, checkpoint_template, parents| {
            let selected_store = crate::local_operation_start_fixture::store_coordinate();
            let other_store = store("other-start-store", "other-start-incarnation");
            let selector = selector_from_signing_authority(parents.signing);
            let selected_port = MemoryAuthorityStore::default();
            let other_port = MemoryAuthorityStore::default();
            let mut host = host_for_start(
                &selected_port,
                &other_port,
                &selector,
                &selected_store,
                &other_store,
                host_generation,
            );
            let mut custody = absent_start_store(request)
                .with_unlisted_indexes_absent()
                .with_preparation(
                    initial_response,
                    pale_core::records::ControlTransactionSequenceV1::new(1)
                        .expect("initial transaction"),
                );
            custody
                .replace_commit_result(Err(ControlStoreError::new(ControlStoreErrorKind::Storage)));
            let mut anchor = initial_anchor();
            let mut authority = CountingAuthority::default();
            let owner_calls = Cell::new(0);
            let signatures = Cell::new(0);
            let checkpoint_schema = checkpoint_template.schema_sha256.clone();
            let checkpoint_authority = semantic.checkpoint_authority;
            let pending = match host.start_invoke_test_only(
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut custody,
                    sessions: &selected_port,
                    anchor: &mut anchor,
                },
                &mut authority,
                ResidentLocalOperationStartInput {
                    semantic,
                    accumulator_limits: pale_ports::AccumulatorPreparationLimits::new(
                        4, 1024, 1024, 512, 65_536,
                    )
                    .expect("bounded accumulator"),
                    checkpoint: ResidentLocalOperationCheckpointInput {
                        id: checkpoint_template.control_checkpoint_id.clone(),
                        reference: local_operation_start_fixture::record_ref(
                            "process-host/start/database-retry-checkpoint",
                        ),
                        schema: &checkpoint_schema,
                        authority: checkpoint_authority,
                    },
                    maximum_record_bytes: 1 << 20,
                },
                |_, _, _, _, _, _, _, _| {
                    owner_calls.set(owner_calls.get() + 1);
                    Ok(())
                },
                |_, _, _| {
                    signatures.set(signatures.get() + 1);
                    Ok(local_operation_start_fixture::signature())
                },
                |_, _, _| {
                    signatures.set(signatures.get() + 1);
                    Ok(local_operation_start_fixture::signature())
                },
                move |prepared| checkpoint_preimage(prepared, checkpoint_template),
                |_, _, _| {
                    signatures.set(signatures.get() + 1);
                    Ok(local_operation_start_fixture::signature())
                },
            ) {
                Err(ResidentProcessInvokeStartError::Continuation(pending)) => *pending,
                _ => panic!("storage failure retains exact start custody"),
            };
            assert_eq!(custody.mutation_counts(), (1, 0));
            assert_eq!(signatures.get(), 3);
            assert_eq!(owner_calls.get(), 1);
            let signed_candidate = custody
                .committed_candidate()
                .cloned()
                .expect("failed commit retains the exact signed candidate bytes");

            custody.replace_commit_result(Ok(OperationCustodyCommitOutcome::Committed));
            anchor.allow_initial_preflight();
            let issued = host
                .continue_operation_start(
                    pending,
                    ResidentDeploymentOperationAccess {
                        store: &selected_store,
                        custody: &mut custody,
                        sessions: &selected_port,
                        anchor: &mut anchor,
                    },
                    &mut authority,
                )
                .unwrap_or_else(|_| panic!("safe storage retry completes exact candidate"));
            assert_eq!(
                issued.handle().0.as_str(),
                "handle/process-host-start-database-retry"
            );
            assert_eq!(custody.mutation_counts(), (2, 0));
            assert_eq!(custody.committed_candidate(), Some(&signed_candidate));
            assert_eq!(signatures.get(), 3);
            assert_eq!(owner_calls.get(), 1);
            assert_eq!(selected_port.entry_count(), 0);
            assert_eq!(other_port.entry_count(), 0);
            assert_eq!(host.provider_routes.reserved, 0);
            assert_eq!(host.provider_routes.checked_out, 0);
            assert!(host.provider_routes.active.is_empty());
        },
    );
}

#[test]
fn handle_issued_continuation_never_reopens_the_handle_signer() {
    let (generation, profile, host_generation) = source_generation_with_host_copy();
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        "process-host-handle-issued-continuation",
        |_| {},
        |semantic, request, checkpoint_template, parents| {
            let selected_store = crate::local_operation_start_fixture::store_coordinate();
            let other_store = store("other-start-store", "other-start-incarnation");
            let selector = selector_from_signing_authority(parents.signing);
            let selected_port = MemoryAuthorityStore::default();
            let other_port = MemoryAuthorityStore::default();
            let mut host = host_for_start(
                &selected_port,
                &other_port,
                &selector,
                &selected_store,
                &other_store,
                host_generation,
            );
            let mut custody = absent_start_store(request).with_preparation(
                initial_response,
                ControlTransactionSequenceV1::new(1).expect("initial transaction"),
            );
            let mut anchor = initial_anchor();
            let mut authority = CountingAuthority::default();
            let handle_signatures = Cell::new(0);
            let observation_attempts = Cell::new(0);
            let checkpoint_signatures = Cell::new(0);
            let checkpoint_schema = checkpoint_template.schema_sha256.clone();
            let checkpoint_authority = semantic.checkpoint_authority;
            let checkpoint_id = checkpoint_template.control_checkpoint_id.clone();
            let checkpoint_reference = local_operation_start_fixture::record_ref(
                "process-host/start/handle-issued-checkpoint",
            );
            let accumulator_limits =
                pale_ports::AccumulatorPreparationLimits::new(4, 1024, 1024, 512, 65_536)
                    .expect("bounded accumulator");

            let pending = match host.start_invoke_test_only(
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut custody,
                    sessions: &selected_port,
                    anchor: &mut anchor,
                },
                &mut authority,
                ResidentLocalOperationStartInput {
                    semantic,
                    accumulator_limits,
                    checkpoint: ResidentLocalOperationCheckpointInput {
                        id: checkpoint_id.clone(),
                        reference: checkpoint_reference.clone(),
                        schema: &checkpoint_schema,
                        authority: checkpoint_authority,
                    },
                    maximum_record_bytes: 1 << 20,
                },
                |_, _, _, _, _, _, _, _| Ok(()),
                |_, _, _| {
                    handle_signatures.set(handle_signatures.get() + 1);
                    Ok(local_operation_start_fixture::signature())
                },
                |_, _, _| {
                    observation_attempts.set(observation_attempts.get() + 1);
                    Err(InvalidResidentRecordV1::ContextualAdmission {
                        record: "initial operation observation",
                        invariant: "fixture interrupts first observation signing",
                    })
                },
                |_| panic!("checkpoint preparation cannot precede observation signing"),
                |_, _, _| panic!("checkpoint signing cannot precede observation signing"),
            ) {
                Err(ResidentProcessInvokeStartError::Continuation(pending)) => *pending,
                _ => panic!("failed observation signing retains the issued handle"),
            };
            assert!(matches!(
                pending.failure.as_ref(),
                ResidentLocalOperationStartFailure::HandleIssued(_)
            ));
            assert_eq!(handle_signatures.get(), 1);
            assert_eq!(observation_attempts.get(), 1);
            assert_eq!(custody.mutation_counts(), (0, 0));
            let authority_calls = authority.calls;

            let issued = host
                .continue_handle_issued_operation_start(
                    pending,
                    ResidentDeploymentOperationAccess {
                        store: &selected_store,
                        custody: &mut custody,
                        sessions: &selected_port,
                        anchor: &mut anchor,
                    },
                    &mut authority,
                    |_, _, _| {
                        observation_attempts.set(observation_attempts.get() + 1);
                        Ok(local_operation_start_fixture::signature())
                    },
                    |_, _, _| Ok(None),
                    accumulator_limits,
                    ResidentLocalOperationCheckpointInput {
                        id: checkpoint_id,
                        reference: checkpoint_reference,
                        schema: &checkpoint_schema,
                        authority: checkpoint_authority,
                    },
                    move |prepared| checkpoint_preimage(prepared, checkpoint_template),
                    |_, _, _| {
                        checkpoint_signatures.set(checkpoint_signatures.get() + 1);
                        Ok(local_operation_start_fixture::signature())
                    },
                )
                .unwrap_or_else(|_| panic!("remaining phases complete around the issued handle"));

            assert_eq!(
                issued.handle().0.as_str(),
                "handle/process-host-handle-issued-continuation"
            );
            assert_eq!(handle_signatures.get(), 1);
            assert_eq!(observation_attempts.get(), 2);
            assert_eq!(checkpoint_signatures.get(), 1);
            assert_eq!(authority.calls, authority_calls);
            assert_eq!(custody.mutation_counts(), (1, 0));
            assert_eq!(anchor.access_counts(), (1, 1));
        },
    );
}

#[test]
fn checkpoint_issuance_continuation_reuses_the_prepared_preimage() {
    let (generation, profile, host_generation) = source_generation_with_host_copy();
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        "process-host-checkpoint-issuance-continuation",
        |_| {},
        |semantic, request, checkpoint_template, parents| {
            let selected_store = crate::local_operation_start_fixture::store_coordinate();
            let other_store = store("other-start-store", "other-start-incarnation");
            let selector = selector_from_signing_authority(parents.signing);
            let selected_port = MemoryAuthorityStore::default();
            let other_port = MemoryAuthorityStore::default();
            let mut host = host_for_start(
                &selected_port,
                &other_port,
                &selector,
                &selected_store,
                &other_store,
                host_generation,
            );
            let mut custody = absent_start_store(request).with_preparation(
                initial_response,
                ControlTransactionSequenceV1::new(1).expect("initial transaction"),
            );
            let mut anchor = initial_anchor();
            let mut authority = CountingAuthority::default();
            let start_signatures = Cell::new(0);
            let preimage_builds = Cell::new(0);
            let checkpoint_attempts = Cell::new(0);
            let checkpoint_schema = checkpoint_template.schema_sha256.clone();
            let checkpoint_authority = semantic.checkpoint_authority;
            let checkpoint_id = checkpoint_template.control_checkpoint_id.clone();
            let checkpoint_reference = local_operation_start_fixture::record_ref(
                "process-host/start/checkpoint-issuance-continuation",
            );

            let pending = match host.start_invoke_test_only(
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut custody,
                    sessions: &selected_port,
                    anchor: &mut anchor,
                },
                &mut authority,
                ResidentLocalOperationStartInput {
                    semantic,
                    accumulator_limits: pale_ports::AccumulatorPreparationLimits::new(
                        4, 1024, 1024, 512, 65_536,
                    )
                    .expect("bounded accumulator"),
                    checkpoint: ResidentLocalOperationCheckpointInput {
                        id: checkpoint_id.clone(),
                        reference: checkpoint_reference.clone(),
                        schema: &checkpoint_schema,
                        authority: checkpoint_authority,
                    },
                    maximum_record_bytes: 1 << 20,
                },
                |_, _, _, _, _, _, _, _| Ok(()),
                |_, _, _| {
                    start_signatures.set(start_signatures.get() + 1);
                    Ok(local_operation_start_fixture::signature())
                },
                |_, _, _| {
                    start_signatures.set(start_signatures.get() + 1);
                    Ok(local_operation_start_fixture::signature())
                },
                |prepared| {
                    preimage_builds.set(preimage_builds.get() + 1);
                    checkpoint_preimage(prepared, checkpoint_template)
                },
                |_, _, _| {
                    checkpoint_attempts.set(checkpoint_attempts.get() + 1);
                    Err(InvalidResidentRecordV1::ContextualAdmission {
                        record: "initial control checkpoint",
                        invariant: "fixture interrupts first checkpoint signing",
                    })
                },
            ) {
                Err(ResidentProcessInvokeStartError::Continuation(pending)) => *pending,
                _ => panic!("failed checkpoint signing retains its exact preimage"),
            };
            assert!(matches!(
                pending.failure.as_ref(),
                ResidentLocalOperationStartFailure::CheckpointIssuance { .. }
            ));
            assert_eq!(start_signatures.get(), 2);
            assert_eq!(preimage_builds.get(), 1);
            assert_eq!(checkpoint_attempts.get(), 1);
            assert_eq!(custody.mutation_counts(), (0, 0));

            let issued = host
                .continue_checkpoint_issuance_operation_start(
                    pending,
                    ResidentDeploymentOperationAccess {
                        store: &selected_store,
                        custody: &mut custody,
                        sessions: &selected_port,
                        anchor: &mut anchor,
                    },
                    &mut authority,
                    ResidentLocalOperationCheckpointInput {
                        id: checkpoint_id,
                        reference: checkpoint_reference,
                        schema: &checkpoint_schema,
                        authority: checkpoint_authority,
                    },
                    |_, _, _| {
                        checkpoint_attempts.set(checkpoint_attempts.get() + 1);
                        Ok(local_operation_start_fixture::signature())
                    },
                )
                .unwrap_or_else(|_| panic!("retained preimage completes checkpoint issuance"));

            assert_eq!(
                issued.handle().0.as_str(),
                "handle/process-host-checkpoint-issuance-continuation"
            );
            assert_eq!(start_signatures.get(), 2);
            assert_eq!(preimage_builds.get(), 1);
            assert_eq!(checkpoint_attempts.get(), 2);
            assert_eq!(custody.mutation_counts(), (1, 0));
            assert_eq!(anchor.access_counts(), (1, 1));
        },
    );
}

#[test]
fn association_retry_is_pure_and_preserves_both_start_owners() {
    let (generation, profile) = source_generation();
    let first = crate::local_operation_start_fixture::with_start(
        &generation,
        &profile,
        "process-host-association-first",
        |_| {},
        crate::local_operation_start_fixture::StartAdmission::AcceptedOwnerModel,
        |prepared, request, _, _, parents| {
            let (physical, pending, _) = crate::resident::operation_observation_custody::tests::local_start::issue_initial(
                crate::resident::operation_observation_custody::tests::local_start::prepared_initial(
                    prepared.expect("prepare first start"),
                ),
                request,
                |_| {},
                &Cell::new(0),
            )
            .expect("issue first start");
            let mut store = absent_start_store(request);
            let mut anchor = initial_anchor();
            let completed = physical
                .commit(&mut store, &mut anchor, 1 << 20)
                .expect("commit first start")
                .advance_anchor(&mut anchor)
                .expect("anchor first start");
            (
                pending,
                completed,
                selector_from_signing_authority(parents.signing),
            )
        },
    );
    crate::local_operation_start_fixture::with_start(
        &generation,
        &profile,
        "process-host-association-second",
        |_| {},
        crate::local_operation_start_fixture::StartAdmission::AcceptedOwnerModel,
        |prepared, request, _, _, _| {
            let (physical, second_pending, _) = crate::resident::operation_observation_custody::tests::local_start::issue_initial(
                crate::resident::operation_observation_custody::tests::local_start::prepared_initial(
                    prepared.expect("prepare second start"),
                ),
                request,
                |_| {},
                &Cell::new(0),
            )
            .expect("issue second start");
            let mut second_store = absent_start_store(request);
            let mut second_anchor = initial_anchor();
            let second_completed = physical
                .commit(&mut second_store, &mut second_anchor, 1 << 20)
                .expect("commit second start")
                .advance_anchor(&mut second_anchor)
                .expect("anchor second start");

            let selected_store = crate::local_operation_start_fixture::store_coordinate();
            let other_store = store("other-start-store", "other-start-incarnation");
            let selected_port = MemoryAuthorityStore::default();
            let other_port = MemoryAuthorityStore::default();
            let mut host = host_for_start_retaining_generation(
                &selected_port,
                &other_port,
                &first.2,
                &selected_store,
                &other_store,
                None,
            );
            let pending = ResidentProcessOperationStartContinuation {
                owner_instance: host.operations.operation_task_owner_instance().clone(),
                selector: first.2.clone(),
                maximum_record_bytes: 1 << 20,
                failure: Box::new(ResidentLocalOperationStartFailure::Custody(
                    ResidentLocalOperationCustodyFailure::Association {
                        pending: Box::new(first.0),
                        completed: Box::new(second_completed),
                    },
                )),
            };
            let mut unused_custody = absent_start_store(request);
            let mut unused_anchor = initial_anchor();
            let mut authority = CountingAuthority::default();
            let failure = match host.continue_operation_start(
                pending,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut unused_custody,
                    sessions: &selected_port,
                    anchor: &mut unused_anchor,
                },
                &mut authority,
            ) {
                Err(failure) => failure,
                Ok(_) => panic!("unlike association cannot issue a locator"),
            };
            assert!(matches!(
                failure.reason,
                ResidentProcessOperationStartContinuationError::Phase
            ));
            assert_eq!(authority.calls, 0);
            assert_eq!(unused_custody.mutation_counts(), (0, 0));
            assert_eq!(unused_anchor.access_counts(), (0, 0));

            let ResidentLocalOperationStartFailure::Custody(
                ResidentLocalOperationCustodyFailure::Association {
                    pending: first_pending,
                    completed: second_completed,
                },
            ) = *failure.pending.failure
            else {
                panic!("exact association owners remain retained")
            };
            assert_eq!(
                first_pending
                    .finish(first.1)
                    .unwrap_or_else(|_| panic!("first exact association"))
                    .handle()
                    .0
                    .as_str(),
                "handle/process-host-association-first"
            );
            assert_eq!(
                second_pending
                    .finish(*second_completed)
                    .unwrap_or_else(|_| panic!("second exact association"))
                    .handle()
                    .0
                    .as_str(),
                "handle/process-host-association-second"
            );
        },
    );
}
