//! Process-lifetime resident composition laws.

use super::*;
use crate::resident::deployment_finalization::tests::recovery::{with_evidence, StoredChange};
use crate::resident::journal::tests::event_custody::UnavailableEventCustody;
use crate::resident::operation_observation_custody::tests::local_recovery::{
    with_process_host_invoke_request_fixtures, ProcessHostInvokeRequestFixture,
};
use crate::resident::operation_observation_custody::tests::local_start::{
    absent_start_store, initial_anchor, initial_response,
};
use crate::resident::principal_trust_frontier::tests::recovered_cold_start_trust_frontier;
use crate::resident::process_host::operation_request_host::ResidentOperationRequestOutcome;
use crate::resident::process_host::program::*;
use crate::resident::process_host::tests::invoke_request::{
    exact_invoke_policy, host_with_generation, root_invoke_input, with_unreachable_restart_inputs,
    FreshEntropy,
};
use crate::resident::process_host::tests::start::{checkpoint_preimage, CountingAuthority};
use crate::resident_fixture::{synthetic_set_policy, FixedInputSource, RecoveryResolver};
use pale_core::records::{
    ControlCheckpointDomainIdV1, ControlCheckpointIdV1, ControlRecordClassV1,
    ControlStateCheckpointV1, PrincipalResidentWriterFenceV1,
};
use pale_ports::{
    ControlCheckpointHead, CryptographicRandomSourceError, PrincipalResidentWriterHead,
    PrincipalResidentWriterSnapshot, PrincipalTrustStoreBinding, PrincipalWriterGuardPort,
    ResidentOperationFrameFailure,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub(super) struct HeldWriter(pub(super) ResidentPrincipalIdV1);

impl PrincipalWriterGuardPort for HeldWriter {
    fn principal_id(&self) -> &ResidentPrincipalIdV1 {
        &self.0
    }
}

impl PrincipalWriterGuardPort for UnprovedHeldWriter {
    fn principal_id(&self) -> &ResidentPrincipalIdV1 {
        panic!("the serve-turn fixture never reopens its already-built writer")
    }
}

struct AcceptProgramPeer;

impl ResidentProgramPeerAdmissionPort for AcceptProgramPeer {
    type Witness = ();

    fn admit_peer(
        &mut self,
        _: LocalIpcPeerCredentials,
        _: &ResidentOperationFrameRequest,
    ) -> Result<Self::Witness, InvalidResidentRecordV1> {
        Ok(())
    }
}

pub(super) struct RejectProgramEntropy;

impl CryptographicRandomSourcePort for RejectProgramEntropy {
    fn fill_bytes(&self, _: &mut [u8]) -> Result<(), CryptographicRandomSourceError> {
        panic!("protected existing operation cannot allocate another operation identity")
    }
}

struct FixtureInvokeCryptography {
    checkpoint: RefCell<Option<ControlStateCheckpointPreimageV1>>,
}

impl ResidentProgramInvokeCryptographyPort for FixtureInvokeCryptography {
    fn admit_compiled_authority(
        &self,
        _: &CommonRequestEnvelopeV1,
        _: &crate::resident::AdmittedAmbientSession,
        _: &crate::resident::AmbientSessionAdmissionContext<'_>,
        _: &OperationHandlePreimageV1,
        _: &OperationLeaseV1,
        _: &crate::semantic::ResolvedOperation,
        _: &crate::resident::LocalOperationPublicationPolicyInput<'_>,
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
        Ok(crate::local_operation_start_fixture::signature())
    }

    fn sign_observation(
        &self,
        _: &[u8],
        _: &OperationObservationPreimageV1,
        _: &OperationObservationDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1> {
        Ok(crate::local_operation_start_fixture::signature())
    }

    fn prepare_checkpoint(
        &self,
        prepared: &crate::resident::PreparedLocalOperationCheckpoint,
        checkpoint_id: &ControlCheckpointIdV1,
    ) -> Result<ControlStateCheckpointPreimageV1, crate::resident::LocalOperationStartError> {
        let mut checkpoint = self
            .checkpoint
            .borrow_mut()
            .take()
            .ok_or(crate::resident::LocalOperationStartError::Coordinate)?;
        checkpoint.control_checkpoint_id = checkpoint_id.clone();
        checkpoint_preimage(prepared, checkpoint)
    }

    fn sign_checkpoint(
        &self,
        _: &[u8],
        _: &ControlStateCheckpointPreimageV1,
        _: &ControlStateCheckpointDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1> {
        Ok(crate::local_operation_start_fixture::signature())
    }
}

#[derive(Default)]
struct RequiredPolicy {
    turns: usize,
}

impl RequiredPolicy {
    fn retain_turn(&mut self) {
        self.turns += 1;
    }
}

struct UnavailableTurnOwner;

impl ResidentProgramTurnOwner<HeldWriter, RequiredPolicy, ()> for UnavailableTurnOwner {
    type PeerWitness = ();

    fn admit_operation_peer(
        &mut self,
        _: LocalIpcPeerCredentials,
        _: &ResidentOperationFrameRequest,
    ) -> Result<Self::PeerWitness, InvalidResidentRecordV1> {
        panic!("assembly must retain but not run peer admission")
    }

    fn resolve_operation<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, HeldWriter>,
        _: &mut RequiredPolicy,
        _: &[ResidentProgramDeploymentAuthority],
        _: &Self::PeerWitness,
        _authenticated_key: Option<[u8; 32]>,
        _: &ResidentOperationFrameRequest,
    ) -> Result<ResidentProgramOperationResolution, ResidentOperationFrameFailure> {
        panic!("assembly must retain but not resolve a turn")
    }

    fn answer_operation<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, HeldWriter>,
        _: &mut RequiredPolicy,
        _: &ResidentProgramOperationResolution,
    ) -> Result<ResidentOperationRequestOutcome, ResidentOperationFrameFailure> {
        panic!("assembly must retain but not execute a turn")
    }

    fn recover_root_provider<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, HeldWriter>,
        _: &ResidentDeploymentAuthoritySelector,
    ) -> Result<(), ResidentProgramTurnError> {
        panic!("assembly must retain but not recover")
    }

    fn recover_root_consult<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, HeldWriter>,
        _: &mut RequiredPolicy,
        _: &ResidentDeploymentAuthoritySelector,
    ) -> Result<(), ResidentProgramTurnError> {
        panic!("assembly must retain but not recover")
    }

    fn recover_consult_child<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, HeldWriter>,
        _: &mut RequiredPolicy,
        _: &ResidentDeploymentAuthoritySelector,
    ) -> Result<(), ResidentProgramTurnError> {
        panic!("assembly must retain but not recover")
    }

    fn recover_delegate<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, HeldWriter>,
        _: &ResidentDeploymentAuthoritySelector,
    ) -> Result<(), ResidentProgramTurnError> {
        panic!("assembly must retain but not recover")
    }

    fn recover_publication<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, HeldWriter>,
        _: &ResidentDeploymentAuthoritySelector,
    ) -> Result<(), ResidentProgramTurnError> {
        panic!("assembly must retain but not recover")
    }
}

impl crate::resident::process_host::consult_recovery::ResidentConsultationPolicyEvaluator<()>
    for RequiredPolicy
{
    type Error = ();

    fn evaluate(
        &mut self,
        _: &ImmutableRecordReferenceV1,
        _: &crate::resident::consultation::ResidentConsultationPolicyFrontier,
    ) -> Result<ResidentConsultationSupervisionDecision<()>, Self::Error> {
        panic!("assembly must retain but not run Consult policy")
    }
}

impl ResidentProcessInvokePolicyEvaluator for RequiredPolicy {
    fn evaluate_invoke(
        &mut self,
        _: ResidentProcessInvokePolicyContext<'_>,
    ) -> Result<ResidentProcessInvokePolicy, ResidentProcessHostSelectionError> {
        panic!("assembly must retain but not run Invoke policy")
    }
}

struct RecordingTurnOwner {
    request: CommonRequestEnvelopeV1,
    events: Rc<RefCell<Vec<&'static str>>>,
    deny_peer: bool,
    remaining_recovery_failures: usize,
}

impl ResidentProgramTurnOwner<HeldWriter, RequiredPolicy, ()> for RecordingTurnOwner {
    type PeerWitness = ();

    fn admit_operation_peer(
        &mut self,
        _: LocalIpcPeerCredentials,
        _: &ResidentOperationFrameRequest,
    ) -> Result<Self::PeerWitness, InvalidResidentRecordV1> {
        self.events.borrow_mut().push("peer");
        if self.deny_peer {
            return Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "resident operation peer",
                invariant: "test peer is denied",
            });
        }
        Ok(())
    }

    fn resolve_operation<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, HeldWriter>,
        _: &mut RequiredPolicy,
        _: &[ResidentProgramDeploymentAuthority],
        _: &Self::PeerWitness,
        _authenticated_key: Option<[u8; 32]>,
        frame: &ResidentOperationFrameRequest,
    ) -> Result<ResidentProgramOperationResolution, ResidentOperationFrameFailure> {
        self.events.borrow_mut().push("resolve");
        ResidentProgramOperationResolution::admit(
            frame,
            self.request.clone(),
            crate::resident::process_host::operation_request_host::ResidentOperationRequestKind::StartInvoke,
            None,
        )
    }

    fn answer_operation<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, HeldWriter>,
        policy: &mut RequiredPolicy,
        _: &ResidentProgramOperationResolution,
    ) -> Result<ResidentOperationRequestOutcome, ResidentOperationFrameFailure> {
        self.events.borrow_mut().push("owner");
        policy.retain_turn();
        self.events.borrow_mut().push("policy");
        Ok(ResidentOperationRequestOutcome::Pending(
            ResidentOperationFrameLocator::new("handle".to_owned(), "digest".to_owned()),
        ))
    }

    fn recover_root_provider<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, HeldWriter>,
        _: &ResidentDeploymentAuthoritySelector,
    ) -> Result<(), ResidentProgramTurnError> {
        self.events.borrow_mut().push("recover_root_provider");
        if self.remaining_recovery_failures > 0 {
            self.remaining_recovery_failures -= 1;
            Err(ResidentProgramTurnError::RecoveryUnavailable)
        } else {
            Ok(())
        }
    }

    fn recover_root_consult<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, HeldWriter>,
        policy: &mut RequiredPolicy,
        _: &ResidentDeploymentAuthoritySelector,
    ) -> Result<(), ResidentProgramTurnError> {
        self.events.borrow_mut().push("recover_root_consult");
        policy.retain_turn();
        Ok(())
    }

    fn recover_consult_child<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, HeldWriter>,
        _: &mut RequiredPolicy,
        _: &ResidentDeploymentAuthoritySelector,
    ) -> Result<(), ResidentProgramTurnError> {
        self.events.borrow_mut().push("recover_consult_child");
        Ok(())
    }

    fn recover_delegate<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, HeldWriter>,
        _: &ResidentDeploymentAuthoritySelector,
    ) -> Result<(), ResidentProgramTurnError> {
        self.events.borrow_mut().push("recover_delegate");
        Ok(())
    }

    fn recover_publication<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, HeldWriter>,
        _: &ResidentDeploymentAuthoritySelector,
    ) -> Result<(), ResidentProgramTurnError> {
        self.events.borrow_mut().push("recover_publication");
        Ok(())
    }
}

impl ResidentProgramObserveTurnOwner<HeldWriter> for RecordingTurnOwner {
    type PeerWitness = ();

    fn admit_observe_peer(
        &mut self,
        _: LocalIpcPeerCredentials,
        _: &ResidentObserveFrameRequest,
    ) -> Result<Self::PeerWitness, InvalidResidentRecordV1> {
        self.events.borrow_mut().push("observe_peer");
        if self.deny_peer {
            return Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "resident Observe peer",
                invariant: "test peer is denied",
            });
        }
        Ok(())
    }

    fn answer_observe<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, HeldWriter>,
        _: &[ResidentProgramDeploymentAuthority],
        _: &Self::PeerWitness,
        _: &ResidentObserveFrameRequest,
    ) -> Result<
        (
            crate::resident::OperationControlObservation,
            std::time::Instant,
        ),
        pale_ports::ResidentObserveFrameFailure,
    > {
        self.events.borrow_mut().push("observe_resolve");
        Err(pale_ports::ResidentObserveFrameFailure::AdmissionDenied)
    }
}

pub(super) fn checkpoint_for_writer(
    writer: &PrincipalResidentWriterFenceV1,
    state_root: char,
) -> ControlStateCheckpointV1 {
    let store = AuthorityStoreCoordinate::new(
        writer.trust_store_id().clone(),
        writer.trust_store_incarnation_id().clone(),
    );
    let mut preimage =
        crate::local_operation_start_fixture::checkpoint_preimage_at_store(1, None, &store);
    preimage.checkpoint_domain_id =
        ControlCheckpointDomainIdV1::parse("principal-writer:principal-1")
            .expect("writer checkpoint domain");
    preimage.changed_leaf_keys[0].record_class =
        ControlRecordClassV1::parse("principal-resident-writer").expect("writer record class");
    preimage.state_root_sha256 = crate::local_operation_start_fixture::artifact_digest(state_root);
    crate::resident::operation_observation_custody::tests::admit_initial_checkpoint(&preimage)
}

pub(super) fn writer_snapshot(
    writer: &PrincipalResidentWriterFenceV1,
    checkpoint: &ControlStateCheckpointV1,
) -> PrincipalResidentWriterSnapshot {
    let writer_ref = ResidentRecordRefV1::parse("current-principal-writer").expect("writer ref");
    let checkpoint_ref =
        ResidentRecordRefV1::parse("current-principal-writer-checkpoint").expect("checkpoint ref");
    let writer_coordinate = crate::resident::record_projection::coordinate(
        &writer_ref,
        writer
            .canonical_principal_resident_writer_fence_sha256()
            .as_str(),
    )
    .expect("writer coordinate");
    let checkpoint_coordinate = crate::resident::record_projection::coordinate(
        &checkpoint_ref,
        checkpoint.canonical_control_checkpoint_sha256().as_str(),
    )
    .expect("checkpoint coordinate");
    let writer_record =
        crate::resident::record_projection::record(writer_coordinate.clone(), writer)
            .expect("writer record");
    let checkpoint_record =
        crate::resident::record_projection::record(checkpoint_coordinate.clone(), checkpoint)
            .expect("checkpoint record");
    PrincipalResidentWriterSnapshot::new(
        PrincipalTrustStoreBinding::new(
            writer.trust_store_id().clone(),
            writer.trust_store_incarnation_id().clone(),
            writer.principal_id().clone(),
        ),
        PrincipalResidentWriterHead::new(
            writer.writer_fence_id().clone(),
            writer.principal_id().clone(),
            *writer.writer_generation(),
            *writer.state(),
            writer_coordinate,
            ControlCheckpointHead::new(
                checkpoint.checkpoint_domain_id().clone(),
                *checkpoint.domain_epoch(),
                *checkpoint.domain_sequence(),
                *checkpoint.committed_transaction_sequence(),
                checkpoint_coordinate,
            ),
        ),
        writer_record,
        checkpoint_record,
    )
    .expect("physical writer snapshot")
}

fn with_ready_program<TurnOwner>(
    turn_owner: TurnOwner,
    test: impl FnOnce(&mut ResidentProgram<'_, HeldWriter, RequiredPolicy, TurnOwner, ()>),
) where
    TurnOwner: ResidentProgramTurnOwner<HeldWriter, RequiredPolicy, ()>,
{
    with_evidence(
        StoredChange::None,
        |finalization_snapshot, finalization, _, _, _, _, _| {
            let trust = recovered_cold_start_trust_frontier(finalization_snapshot, finalization);
            let runtime = &trust.runtime_authorities()[0];
            let store = AuthorityStoreCoordinate::new(
                runtime.authority().control_store_id().clone(),
                runtime.authority().control_incarnation_id().clone(),
            );
            let writer = crate::local_operation_start_fixture::publication_writer();
            let checkpoint = checkpoint_for_writer(&writer, 'c');
            let snapshot = writer_snapshot(&writer, &checkpoint);
            let mut writer_admission = move |_: &PrincipalResidentWriterSnapshot| {
                Ok(AdmittedCurrentPrincipalWriter {
                    writer: writer.clone(),
                    checkpoint: checkpoint.clone(),
                })
            };
            let authority_store = MemoryAuthorityStore::default();
            let mut event_custody = UnavailableEventCustody;
            let resolver = RecoveryResolver::default();
            let mut source = FixedInputSource::default();
            let open = ResidentProgramDeploymentOpen {
                durable: &authority_store,
                sessions: &authority_store,
                events: &mut event_custody,
                maximum_event_custody_bytes: 65_536,
                input: ResidentColdOpenInput::new(
                    store,
                    limits(),
                    limits().maximum_entry_bytes(),
                    RecoveryReferenceKnowledge::Known(BTreeSet::new()),
                    &resolver,
                    &mut source,
                    ResidentReconstructionParameters::new(
                        synthetic_set_policy(),
                        4,
                        4,
                        WakeEpochV1::new(91),
                    ),
                ),
            };
            let mut program = ResidentProgram::recover(
                HeldWriter(snapshot.binding().principal_id().clone()),
                &snapshot,
                &mut writer_admission,
                &trust,
                vec![open],
                NonZeroUsize::new(1).expect("one deployment"),
                task_state(),
                RequiredPolicy::default(),
                turn_owner,
            )
            .expect("one exact deployment assembles");
            test(&mut program);
        },
    )
}

#[test]
fn writer_join_rejects_guard_and_physical_substitution() {
    let writer = crate::local_operation_start_fixture::publication_writer();
    let checkpoint = checkpoint_for_writer(&writer, 'c');
    let snapshot = writer_snapshot(&writer, &checkpoint);
    let admitted = AdmittedCurrentPrincipalWriter {
        writer: writer.clone(),
        checkpoint: checkpoint.clone(),
    };
    let wrong = HeldWriter(ResidentPrincipalIdV1::parse("wrong-principal").expect("principal"));
    assert!(matches!(
        ReadyPrincipalResidentWriter::recover(wrong, &snapshot, admitted, Vec::new()),
        Err(ResidentPrincipalWriterJoinError::PrincipalAssociation)
    ));

    let wrong_checkpoint = checkpoint_for_writer(&writer, '9');
    let admitted = AdmittedCurrentPrincipalWriter {
        writer,
        checkpoint: wrong_checkpoint,
    };
    let guard = HeldWriter(snapshot.binding().principal_id().clone());
    assert!(matches!(
        ReadyPrincipalResidentWriter::recover(guard, &snapshot, admitted, Vec::new()),
        Err(ResidentPrincipalWriterJoinError::PhysicalRecordAssociation)
    ));
}

#[test]
fn program_derives_the_exact_deployment_and_cold_opens_it() {
    with_evidence(
        StoredChange::None,
        |finalization_snapshot, finalization, _, _, _, _, _| {
            let trust = recovered_cold_start_trust_frontier(finalization_snapshot, finalization);
            let runtime = &trust.runtime_authorities()[0];
            let store = AuthorityStoreCoordinate::new(
                runtime.authority().control_store_id().clone(),
                runtime.authority().control_incarnation_id().clone(),
            );
            let writer = crate::local_operation_start_fixture::publication_writer();
            let checkpoint = checkpoint_for_writer(&writer, 'c');
            let snapshot = writer_snapshot(&writer, &checkpoint);
            let mut writer_admission = move |_: &PrincipalResidentWriterSnapshot| {
                Ok(AdmittedCurrentPrincipalWriter {
                    writer: writer.clone(),
                    checkpoint: checkpoint.clone(),
                })
            };
            let authority_store = MemoryAuthorityStore::default();
            let mut event_custody = UnavailableEventCustody;
            let resolver = RecoveryResolver::default();
            let mut source = FixedInputSource::default();
            let open = ResidentProgramDeploymentOpen {
                durable: &authority_store,
                sessions: &authority_store,
                events: &mut event_custody,
                maximum_event_custody_bytes: 65_536,
                input: ResidentColdOpenInput::new(
                    store,
                    limits(),
                    limits().maximum_entry_bytes(),
                    RecoveryReferenceKnowledge::Known(BTreeSet::new()),
                    &resolver,
                    &mut source,
                    ResidentReconstructionParameters::new(
                        synthetic_set_policy(),
                        4,
                        4,
                        WakeEpochV1::new(91),
                    ),
                ),
            };
            let _program = ResidentProgram::<_, _, _, ()>::recover(
                HeldWriter(snapshot.binding().principal_id().clone()),
                &snapshot,
                &mut writer_admission,
                &trust,
                vec![open],
                NonZeroUsize::new(1).expect("one deployment"),
                task_state(),
                RequiredPolicy::default(),
                UnavailableTurnOwner,
            )
            .expect("one exact deployment assembles");
        },
    );
}

#[test]
fn program_rejects_a_missing_open_store_before_cold_open() {
    with_evidence(
        StoredChange::None,
        |finalization_snapshot, finalization, _, _, _, _, _| {
            let trust = recovered_cold_start_trust_frontier(finalization_snapshot, finalization);
            let writer = crate::local_operation_start_fixture::publication_writer();
            let checkpoint = checkpoint_for_writer(&writer, 'c');
            let snapshot = writer_snapshot(&writer, &checkpoint);
            let mut writer_admission = move |_: &PrincipalResidentWriterSnapshot| {
                Ok(AdmittedCurrentPrincipalWriter {
                    writer: writer.clone(),
                    checkpoint: checkpoint.clone(),
                })
            };
            assert!(matches!(
                ResidentProgram::<_, RequiredPolicy, UnavailableTurnOwner, ()>::recover(
                    HeldWriter(snapshot.binding().principal_id().clone()),
                    &snapshot,
                    &mut writer_admission,
                    &trust,
                    Vec::new(),
                    NonZeroUsize::new(1).expect("one deployment"),
                    task_state(),
                    RequiredPolicy::default(),
                    UnavailableTurnOwner,
                ),
                Err(ResidentProgramRecoveryError::OpenedStoreSet)
            ));
        },
    );
}

#[test]
fn program_admits_peer_before_resolution_and_sends_one_response() {
    let (generation, profile) =
        crate::resident::operation_observation_custody::tests::local_start::source_generation();
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        "program-turn",
        |_| {},
        |_, request, _, _| {
            let frame = ResidentOperationFrameRequest::StartInvoke {
                signed_common_request: pale_core::canon::to_canonical_string(request)
                    .expect("request is canonical"),
            };
            let events = Rc::new(RefCell::new(Vec::new()));
            let owner = RecordingTurnOwner {
                request: request.clone(),
                events: Rc::clone(&events),
                deny_peer: false,
                remaining_recovery_failures: 0,
            };
            with_ready_program(owner, |program| {
                let mut port = OperationFramePort::new(&frame);
                let deadline = Instant::now()
                    .checked_add(std::time::Duration::from_secs(2))
                    .expect("test deadline");
                let response = program
                    .serve_operation_frame(&mut port, deadline, deadline)
                    .expect("one admitted frame is answered");
                assert!(matches!(
                    response,
                    pale_ports::ResidentOperationFrameResponse::Pending { .. }
                ));
                let sent: pale_ports::ResidentOperationFrameResponse =
                    serde_json::from_slice(&port.sent).expect("one canonical response decodes");
                assert_eq!(sent, response);
                assert_eq!(
                    events.borrow().as_slice(),
                    ["peer", "resolve", "owner", "policy"]
                );
            });
        },
    );
}

#[test]
fn program_peer_denial_precedes_protected_resolution_and_response() {
    let (generation, profile) =
        crate::resident::operation_observation_custody::tests::local_start::source_generation();
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        "program-peer-denial",
        |_| {},
        |_, request, _, _| {
            let frame = ResidentOperationFrameRequest::StartInvoke {
                signed_common_request: pale_core::canon::to_canonical_string(request)
                    .expect("request is canonical"),
            };
            let events = Rc::new(RefCell::new(Vec::new()));
            let owner = RecordingTurnOwner {
                request: request.clone(),
                events: Rc::clone(&events),
                deny_peer: true,
                remaining_recovery_failures: 0,
            };
            with_ready_program(owner, |program| {
                let mut port = OperationFramePort::new(&frame);
                let deadline = Instant::now()
                    .checked_add(std::time::Duration::from_secs(2))
                    .expect("test deadline");
                assert!(matches!(
                    program.serve_operation_frame(&mut port, deadline, deadline),
                    Err(ResidentProgramTurnError::OperationFrame(
                        crate::resident::process_host::operation_request_host::ResidentOperationRequestHostError::PeerAdmission(_)
                    ))
                ));
                assert!(port.sent.is_empty());
                assert_eq!(events.borrow().as_slice(), ["peer"]);
            });
        },
    );
}

#[test]
fn program_keeps_observe_on_its_separate_peer_first_frame() {
    let (generation, profile) =
        crate::resident::operation_observation_custody::tests::local_start::source_generation();
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        "program-observe-rejection",
        |_| {},
        |_, request, _, _| {
            let frame = ResidentObserveFrameRequest::new(
                "operation-handle".to_owned(),
                "operation-digest".to_owned(),
                pale_core::canon::to_canonical_string(request).expect("request is canonical"),
            );
            let events = Rc::new(RefCell::new(Vec::new()));
            let owner = RecordingTurnOwner {
                request: request.clone(),
                events: Rc::clone(&events),
                deny_peer: false,
                remaining_recovery_failures: 0,
            };
            with_ready_program(owner, |program| {
                let mut port = OperationFramePort::new(&frame);
                let deadline = Instant::now()
                    .checked_add(std::time::Duration::from_secs(2))
                    .expect("test deadline");
                let observed = program
                    .serve_observe_frame(&mut port, deadline)
                    .expect("Observe resolution rejection is framed");
                assert!(observed.is_none());
                let response: pale_ports::ResidentObserveFrameResponse =
                    serde_json::from_slice(&port.sent).expect("one Observe response decodes");
                assert!(matches!(
                    response,
                    pale_ports::ResidentObserveFrameResponse::Rejected {
                        failure: pale_ports::ResidentObserveFrameFailure::AdmissionDenied
                    }
                ));
                assert_eq!(
                    events.borrow().as_slice(),
                    ["observe_peer", "observe_resolve"]
                );
            });
        },
    );
}

#[test]
fn program_recovery_retries_a_failed_source_without_starving_other_lanes() {
    let (generation, profile) =
        crate::resident::operation_observation_custody::tests::local_start::source_generation();
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        "program-recovery",
        |_| {},
        |_, request, _, _| {
            let events = Rc::new(RefCell::new(Vec::new()));
            let owner = RecordingTurnOwner {
                request: request.clone(),
                events: Rc::clone(&events),
                deny_peer: false,
                remaining_recovery_failures: 1,
            };
            with_ready_program(owner, |program| {
                assert!(matches!(
                    program.recover_one_bounded_source(),
                    Err(ResidentProgramTurnError::RecoveryUnavailable)
                ));
                program
                    .recover_one_bounded_source()
                    .expect("root Consult is not starved by root-provider failure");
                program
                    .recover_one_bounded_source()
                    .expect("Consult-child is not starved by root-provider failure");
                program
                    .recover_one_bounded_source()
                    .expect("Delegate is not starved by root-provider failure");
                program
                    .recover_one_bounded_source()
                    .expect("Event publication is not starved by root-provider failure");
                program
                    .recover_one_bounded_source()
                    .expect("the failed root-provider lane remains retryable");
                assert_eq!(
                    events.borrow().as_slice(),
                    [
                        "recover_root_provider",
                        "recover_root_consult",
                        "recover_consult_child",
                        "recover_delegate",
                        "recover_publication",
                        "recover_root_provider"
                    ]
                );
            });
        },
    );
}

#[test]
fn cold_program_recreation_restarts_the_bounded_recovery_schedule() {
    let (generation, profile) =
        crate::resident::operation_observation_custody::tests::local_start::source_generation();
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        "program-cold-recovery-schedule",
        |_| {},
        |_, request, _, _| {
            with_evidence(
                StoredChange::None,
                |finalization_snapshot, finalization, _, _, _, _, _| {
                    let trust =
                        recovered_cold_start_trust_frontier(finalization_snapshot, finalization);
                    let runtime = &trust.runtime_authorities()[0];
                    let store = AuthorityStoreCoordinate::new(
                        runtime.authority().control_store_id().clone(),
                        runtime.authority().control_incarnation_id().clone(),
                    );
                    let authority_store = MemoryAuthorityStore::default();
                    let events = Rc::new(RefCell::new(Vec::new()));

                    for _ in 0..2 {
                        let writer = crate::local_operation_start_fixture::publication_writer();
                        let checkpoint = checkpoint_for_writer(&writer, 'c');
                        let snapshot = writer_snapshot(&writer, &checkpoint);
                        let mut writer_admission = move |_: &PrincipalResidentWriterSnapshot| {
                            Ok(AdmittedCurrentPrincipalWriter {
                                writer: writer.clone(),
                                checkpoint: checkpoint.clone(),
                            })
                        };
                        let mut event_custody = UnavailableEventCustody;
                        let resolver = RecoveryResolver::default();
                        let mut source = FixedInputSource::default();
                        let open = ResidentProgramDeploymentOpen {
                            durable: &authority_store,
                            sessions: &authority_store,
                            events: &mut event_custody,
                            maximum_event_custody_bytes: 65_536,
                            input: ResidentColdOpenInput::new(
                                store.clone(),
                                limits(),
                                limits().maximum_entry_bytes(),
                                RecoveryReferenceKnowledge::Known(BTreeSet::new()),
                                &resolver,
                                &mut source,
                                ResidentReconstructionParameters::new(
                                    synthetic_set_policy(),
                                    4,
                                    4,
                                    WakeEpochV1::new(91),
                                ),
                            ),
                        };
                        let owner = RecordingTurnOwner {
                            request: request.clone(),
                            events: Rc::clone(&events),
                            deny_peer: false,
                            remaining_recovery_failures: 0,
                        };
                        let mut program = ResidentProgram::recover(
                            HeldWriter(snapshot.binding().principal_id().clone()),
                            &snapshot,
                            &mut writer_admission,
                            &trust,
                            vec![open],
                            NonZeroUsize::new(1).expect("one deployment"),
                            task_state(),
                            RequiredPolicy::default(),
                            owner,
                        )
                        .expect("cold resident program recreates");
                        program
                            .recover_one_bounded_source()
                            .expect("the first bounded source safely hands off");
                    }

                    assert_eq!(
                        events.borrow().as_slice(),
                        ["recover_root_provider", "recover_root_provider"]
                    );
                },
            );
        },
    );
}

#[test]
fn program_frame_runs_the_source_owned_invoke_to_durable_records() {
    let (mut generations, profile) = crate::resident::operation_observation_custody::tests::local_start::source_generation_copies(2);
    let retained_generation = generations.pop().expect("retained generation");
    let semantic_generation = generations.pop().expect("semantic generation");
    let store = crate::local_operation_start_fixture::store_coordinate();
    crate::local_operation_start_fixture::with_invoke_start_input_at_store(
        &semantic_generation,
        &profile,
        "program-real-invoke",
        |_| {},
        &store,
        |semantic, request, checkpoint, parents| {
            let selector = selector_from_signing_authority(parents.signing);
            let reads = Arc::new(AtomicUsize::new(0));
            let mut custody = absent_start_store(request)
                .at_store(&store)
                .with_index_read_counter(Arc::clone(&reads))
                .with_unlisted_indexes_absent()
                .with_lease_retention(Ok(pale_ports::OperationCustodyCommitOutcome::Committed))
                .with_preparation(
                    initial_response,
                    pale_core::records::ControlTransactionSequenceV1::new(1)
                        .expect("initial transaction"),
                );
            let policy = exact_invoke_policy(&semantic, 1 << 20);
            with_unreachable_restart_inputs(&semantic, |restart, admission| {
                let sessions = MemoryAuthorityStore::default();
                let mut anchor = initial_anchor();
                let turn_owner = ResidentProgramInvokeTurnOwner::new(
                    AcceptProgramPeer,
                    request.clone(),
                    root_invoke_input(&semantic, request),
                    restart,
                    admission,
                    selector.clone(),
                    store.clone(),
                    &mut custody,
                    &sessions,
                    &mut anchor,
                    FreshEntropy::new(Arc::clone(&reads)),
                    CountingAuthority::default(),
                    FixtureInvokeCryptography {
                        checkpoint: RefCell::new(Some(checkpoint)),
                    },
                );
                let application_store = MemoryAuthorityStore::default();
                let process = host_with_generation(
                    &application_store,
                    &selector,
                    &store,
                    retained_generation,
                );
                let mut program =
                    ResidentProgram::fixture_from_process(process, policy, turn_owner);

                let frame = ResidentOperationFrameRequest::StartInvoke {
                    signed_common_request: pale_core::canon::to_canonical_string(request)
                        .expect("request is canonical"),
                };
                let mut port = OperationFramePort::new(&frame);
                let deadline = Instant::now()
                    .checked_add(std::time::Duration::from_secs(2))
                    .expect("test deadline");
                let response = program
                    .serve_operation_frame(&mut port, deadline, deadline)
                    .expect("real Invoke reaches one framed response");
                let records = match response {
                    pale_ports::ResidentOperationFrameResponse::Started { records } => records,
                    other => {
                        panic!("fresh Invoke must return durable start records: {other:?}")
                    }
                };
                assert!(!records.operation_handle_canonical().is_empty());
                assert!(!records.operation_observation_canonical().is_empty());
                assert_eq!(reads.load(Ordering::SeqCst), 6);
                let decoded: pale_ports::ResidentOperationFrameResponse =
                    serde_json::from_slice(&port.sent).expect("one response decodes");
                assert!(matches!(
                    decoded,
                    pale_ports::ResidentOperationFrameResponse::Started { .. }
                ));
                drop(program);
            });
        },
    );
}

#[test]
fn program_frame_returns_existing_start_receipt_and_rejects_wrong_class_without_replay() {
    let calls = Rc::new(Cell::new(0));
    let started = Rc::new(Cell::new(0));
    let pending = Rc::new(Cell::new(0));
    let rejected = Rc::new(Cell::new(0));
    let counted_calls = Rc::clone(&calls);
    let counted_started = Rc::clone(&started);
    let counted_pending = Rc::clone(&pending);
    let counted_rejected = Rc::clone(&rejected);
    with_process_host_invoke_request_fixtures(
        &mut move |fixture: ProcessHostInvokeRequestFixture<'_>| {
            counted_calls.set(counted_calls.get() + 1);
            let ProcessHostInvokeRequestFixture {
                recovery,
                admission,
                request,
                session,
                current,
                signing_authority,
                generation,
                custody,
                sessions,
                anchor,
            } = fixture;
            let store = recovery.operation.context.store.clone();
            let checkpoint_authority = recovery.operation.context.checkpoint_authority;
            let maximum_record_bytes = recovery.operation.context.maximum_record_bytes;
            let selector = selector_from_signing_authority(signing_authority);
            let application_store = MemoryAuthorityStore::default();
            let process = host_with_generation(&application_store, &selector, &store, generation);
            let (policy_generation, policy_profile) =
                crate::resident::operation_observation_custody::tests::local_start::source_generation();
            crate::local_operation_start_fixture::with_invoke_start_input(
                &policy_generation,
                &policy_profile,
                "program-existing-operation",
                |_| {},
                |semantic, _, checkpoint, _| {
                    let owner = ResidentProgramInvokeTurnOwner::new(
                        AcceptProgramPeer,
                        request.clone(),
                        ResidentProcessRootInput {
                            request,
                            session,
                            current,
                            handle_signing_authority: signing_authority,
                            checkpoint_authority,
                            fresh_start_admission: ResidentProcessFreshStartAdmission::Available,
                        },
                        super::restart::restart_input(recovery, 1 << 20),
                        admission,
                        selector,
                        store,
                        custody,
                        sessions,
                        anchor,
                        RejectProgramEntropy,
                        CountingAuthority::default(),
                        FixtureInvokeCryptography {
                            checkpoint: RefCell::new(Some(checkpoint)),
                        },
                    );
                    let policy = exact_invoke_policy(&semantic, maximum_record_bytes);
                    let mut program = ResidentProgram::fixture_from_process(process, policy, owner);
                    let frame = ResidentOperationFrameRequest::StartInvoke {
                        signed_common_request: pale_core::canon::to_canonical_string(request)
                            .expect("request is canonical"),
                    };
                    let mut port = OperationFramePort::new(&frame);
                    let deadline = Instant::now()
                        .checked_add(std::time::Duration::from_secs(2))
                        .expect("test deadline");
                    let response = program
                        .serve_operation_frame(&mut port, deadline, deadline)
                        .expect("protected operation is rediscovered through the framed owner");
                    match &response {
                        pale_ports::ResidentOperationFrameResponse::Started { .. } => {
                            counted_started.set(counted_started.get() + 1);
                        }
                        pale_ports::ResidentOperationFrameResponse::Pending { .. } => {
                            counted_pending.set(counted_pending.get() + 1);
                        }
                        pale_ports::ResidentOperationFrameResponse::Rejected {
                            failure:
                                pale_ports::ResidentOperationFrameFailure::ConfigurationUnavailable,
                        } => {
                            counted_rejected.set(counted_rejected.get() + 1);
                        }
                        other => panic!("unexpected existing Invoke response: {other:?}"),
                    }
                    let decoded: pale_ports::ResidentOperationFrameResponse =
                        serde_json::from_slice(&port.sent).expect("one response decodes");
                    assert_eq!(decoded, response);
                },
            );
        },
    );
    assert_eq!(calls.get(), 8);
    assert_eq!(started.get(), 7);
    assert_eq!(pending.get(), 0);
    assert_eq!(rejected.get(), 1);
}
