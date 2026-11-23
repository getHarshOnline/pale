//! Request-first root-Invoke resolution through one process host.

use super::*;
use crate::resident::operation_observation_custody::tests::local_recovery::{
    with_process_host_invoke_request_fixtures, ProcessHostInvokeRequestFixture,
};
use crate::resident::operation_observation_custody::tests::local_start::{
    absent_start_store, initial_anchor, initial_response,
};
use crate::resident::operation_observation_custody::tests::RecordingStore;
use crate::resident::process_host::restart::{
    ResidentProcessOperationRestartDisposition, ResidentProcessOperationRestartInput,
    ResidentProcessRestartReadLimits,
};
use crate::resident::process_host::tests::start::{checkpoint_preimage, CountingAuthority};
use crate::resident::{
    OperationCheckpointAncestryLimits, OperationObservationRecoveryLimits,
    OperationObservationSelection,
};
use crate::resident_fixture::RecoveryResolver;
use pale_ports::{
    AntiRollbackAnchor, AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError,
    AntiRollbackAnchorKey, AntiRollbackAnchorObservation, AntiRollbackAnchorPort,
    ControlRecordCoordinate, ControlRecordDigest, ControlStoreError, ControlStoreErrorKind,
    CryptographicRandomSourceError, CryptographicRandomSourcePort,
    PreparedAntiRollbackAnchorTransition, SessionLeaseBody, SessionLeaseBodyRead,
    SessionLeaseBodyRetention, SessionLeaseCustodyPort,
};
use std::cell::Cell;
use std::num::NonZeroUsize;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

fn invoke_lease_basis(
    lease: &pale_core::records::OperationLeaseV1,
) -> ResidentProcessOperationLeaseBasis {
    ResidentProcessOperationLeaseBasis {
        capabilities: lease.capabilities().to_vec(),
        effects: lease.effects().to_vec(),
        read_set: lease.read_set().to_vec(),
        evidence_obligations: lease.evidence_obligations().to_vec(),
        budget_units: *lease.budget_units(),
        timeout_millis: *lease.timeout_millis(),
        expires_at: lease.expires_at(),
        invocation_correlation: lease.invocation_correlation().clone(),
    }
}

pub(super) fn root_invoke_input<'a>(
    semantic: &'a crate::resident::LocalOperationStartInput<'a>,
    request: &'a CommonRequestEnvelopeV1,
) -> ResidentProcessRootInput<'a> {
    ResidentProcessRootInput {
        request,
        session: semantic.session,
        current: semantic.current,
        handle_signing_authority: semantic.context.signed.signing_authority,
        checkpoint_authority: semantic.checkpoint_authority,
        fresh_start_admission: ResidentProcessFreshStartAdmission::Available,
    }
}

pub(super) struct ExactInvokePolicy(Option<ResidentProcessInvokePolicy>);

impl ExactInvokePolicy {
    pub(super) fn with_handle_expiry(mut self, expires_at: UnixTimeV1) -> Self {
        self.0
            .as_mut()
            .expect("unconsumed policy")
            .handle_expires_at = expires_at;
        self
    }
}

impl ResidentProcessInvokePolicyEvaluator for ExactInvokePolicy {
    fn evaluate_invoke(
        &mut self,
        context: ResidentProcessInvokePolicyContext<'_>,
    ) -> Result<ResidentProcessInvokePolicy, ResidentProcessHostSelectionError> {
        assert_eq!(context.request.operation(), &context.operation.record.id);
        assert_eq!(
            &context.request.session_admission().session_admission_id,
            context.session.lease().session_admission_id()
        );
        self.0
            .take()
            .ok_or(ResidentProcessHostSelectionError::StartRequestAssociation)
    }
}

impl super::super::consult_recovery::ResidentConsultationPolicyEvaluator<()> for ExactInvokePolicy {
    type Error = ();

    fn evaluate(
        &mut self,
        _: &ImmutableRecordReferenceV1,
        _: &crate::resident::consultation::ResidentConsultationPolicyFrontier,
    ) -> Result<ResidentConsultationSupervisionDecision<()>, Self::Error> {
        panic!("root Invoke test policy cannot supervise Consult")
    }
}

struct RejectingInvokePolicy<'a>(&'a Cell<usize>);

impl ResidentProcessInvokePolicyEvaluator for RejectingInvokePolicy<'_> {
    fn evaluate_invoke(
        &mut self,
        _: ResidentProcessInvokePolicyContext<'_>,
    ) -> Result<ResidentProcessInvokePolicy, ResidentProcessHostSelectionError> {
        self.0.set(self.0.get() + 1);
        Err(ResidentProcessHostSelectionError::StartRequestAssociation)
    }
}

pub(super) fn exact_invoke_policy(
    semantic: &crate::resident::LocalOperationStartInput<'_>,
    maximum_record_bytes: usize,
) -> ExactInvokePolicy {
    ExactInvokePolicy(Some(ResidentProcessInvokePolicy {
        checkpoint_profile: semantic.checkpoint_profile.clone(),
        publication_policy: crate::resident::LocalOperationPublicationPolicy::from_admitted_input(
            crate::resident::LocalOperationPublicationPolicyInput {
                target_scope: semantic.publication_policy.target_scope,
                classification: semantic.publication_policy.classification,
                visibility: semantic.publication_policy.visibility,
                disclosure_ceiling: semantic.publication_policy.disclosure_ceiling,
                payload_resolution_policy: semantic.publication_policy.payload_resolution_policy,
                actor_provenance: semantic.publication_policy.actor_provenance,
            },
        ),
        maximum_record_bytes,
        lease: invoke_lease_basis(semantic.context.operation_lease),
        handle_expires_at: semantic.handle.expires_at,
        handle_retain_until: semantic.handle.retain_until,
        accumulator_limits: pale_ports::AccumulatorPreparationLimits::new(
            4, 1024, 1024, 512, 65_536,
        )
        .expect("bounded accumulator"),
    }))
}

struct RejectEntropy;

impl CryptographicRandomSourcePort for RejectEntropy {
    fn fill_bytes(&self, _: &mut [u8]) -> Result<(), CryptographicRandomSourceError> {
        panic!("an existing request must not draw entropy")
    }
}

fn reject_request_admission(
    _: &ResidentRecordRefV1,
    _: CommonRequestEnvelopeWireV1,
) -> Result<CommonRequestEnvelopeV1, InvalidResidentRecordV1> {
    panic!("fresh resolution cannot admit a retained request")
}

fn reject_handle_authority_admission(
    _: &OperationHandlePreimageV1,
    _: Option<&OperationHandleV1>,
) -> Result<(), InvalidResidentRecordV1> {
    panic!("fresh resolution cannot admit a handle")
}

fn reject_handle_signature_verification(
    _: &[u8],
    _: &OperationHandlePreimageV1,
    _: &OperationHandleDigest,
    _: &CanonicalSignatureV1,
) -> Result<(), InvalidResidentRecordV1> {
    panic!("fresh resolution cannot verify a handle")
}

fn reject_checkpoint_admission(
    _: &ResidentRecordRefV1,
    _: ControlStateCheckpointWireV1,
) -> Result<ControlStateCheckpointV1, InvalidResidentRecordV1> {
    panic!("fresh resolution cannot admit a checkpoint")
}

fn reject_observation_signer_resolution(
    _: &ResidentRecordRefV1,
    _: &OperationObservationWireV1,
) -> Result<SigningAuthorityBindingV1, InvalidResidentRecordV1> {
    panic!("fresh resolution cannot resolve a signer")
}

fn reject_observation_state_admission(
    _: &OperationObservationStateV1,
) -> Result<(), InvalidResidentRecordV1> {
    panic!("fresh resolution cannot admit an observation")
}

fn reject_rotation_admission(
    _: &ResidentRecordRefV1,
    _: &ArtifactDigest,
    _: &ResidentRecordRefV1,
    _: &ArtifactDigest,
) -> Result<(), InvalidResidentRecordV1> {
    panic!("fresh resolution cannot admit a rotation")
}

fn reject_observation_signature_verification(
    _: &[u8],
    _: &OperationObservationPreimageV1,
    _: &OperationObservationDigest,
    _: &CanonicalSignatureV1,
) -> Result<(), InvalidResidentRecordV1> {
    panic!("fresh resolution cannot verify an observation")
}

fn reject_checkpoint_signature_verification(
    _: &[u8],
    _: &ControlStateCheckpointPreimageV1,
    _: &ControlStateCheckpointDigest,
    _: &CanonicalSignatureV1,
) -> Result<(), InvalidResidentRecordV1> {
    panic!("fresh resolution cannot verify a checkpoint")
}

pub(super) struct FreshEntropy {
    lookup_reads: Arc<AtomicUsize>,
    calls: AtomicUsize,
}

impl FreshEntropy {
    pub(super) fn new(lookup_reads: Arc<AtomicUsize>) -> Self {
        Self {
            lookup_reads,
            calls: AtomicUsize::new(0),
        }
    }
}

impl CryptographicRandomSourcePort for FreshEntropy {
    fn fill_bytes(&self, destination: &mut [u8]) -> Result<(), CryptographicRandomSourceError> {
        let lookup_reads = self.lookup_reads.load(Ordering::SeqCst);
        assert!(
            lookup_reads >= 2 && lookup_reads.is_multiple_of(2),
            "the exact request and evidence index pair is read before identity allocation",
        );
        let call = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        destination.fill(u8::try_from(call).expect("bounded allocation draw"));
        Ok(())
    }
}

struct RejectSessionAccess;

impl SessionLeaseCustodyPort for RejectSessionAccess {
    fn compare_and_retain(
        &self,
        _: &SessionLeaseBody,
    ) -> Result<SessionLeaseBodyRetention, ControlStoreError> {
        panic!("fresh request resolution must not retain a session body")
    }

    fn read(
        &self,
        _: &SessionLeaseBodyRead,
    ) -> Result<Option<SessionLeaseBody>, ControlStoreError> {
        panic!("fresh request resolution must not read a session body")
    }
}

struct RejectAnchorAccess;

impl AntiRollbackAnchorPort for RejectAnchorAccess {
    fn read(
        &mut self,
        _: &AntiRollbackAnchorKey,
    ) -> Result<Option<AntiRollbackAnchor>, AntiRollbackAnchorError> {
        panic!("fresh request resolution must not read an anchor")
    }

    fn compare_and_advance(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError> {
        panic!("request resolution must not advance an anchor")
    }

    fn observe(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError> {
        panic!("request resolution must not observe an anchor transition")
    }
}

pub(super) fn host_with_generation<'store>(
    port: &'store MemoryAuthorityStore,
    selector: &ResidentDeploymentAuthoritySelector,
    store: &AuthorityStoreCoordinate,
    generation: crate::resident::AdmittedCompiledGeneration,
) -> ResidentProcessHost<'store, UnprovedHeldWriter> {
    let mut application = ready_application(port, store.clone(), 81);
    application.fixture_retain_generation(generation);
    ResidentProcessHost::new(
        unproved_writer(selector.principal_id.clone(), vec![store.clone()]),
        vec![unproved_deployment_for_selector(
            selector,
            store.clone(),
            application,
        )],
        NonZeroUsize::new(1).expect("one deployment"),
        ResidentOperationHost::new(task_state()),
    )
    .expect("one exact request-resolution deployment")
}

fn restart_input<'a>(
    recovery: OperationControlRecoveryInput<'a>,
) -> ResidentProcessOperationRestartInput<'a> {
    ResidentProcessOperationRestartInput {
        committed_observation_limits: None,
        operation: recovery.operation.context,
        maximum_parent_depth: recovery.operation.maximum_parent_depth,
        current_head: ResidentProcessRestartReadLimits {
            observation: recovery.observation_limits,
            checkpoints: recovery.checkpoint_limits,
            membership: recovery.membership_limits,
        },
        maximum_selection_bytes: 1024 * 1024,
        a2a_task: None,
        a2a_cancel_task: None,
    }
}

pub(super) fn with_unreachable_restart_inputs<R>(
    semantic: &crate::resident::LocalOperationStartInput<'_>,
    run: impl FnOnce(
        ResidentProcessOperationRestartInput<'_>,
        OperationControlRecoveryAdmission<'_>,
    ) -> R,
) -> R {
    let profile = crate::local_operation_start_fixture::profile();
    let historical = RecoveryResolver::default();
    let store = crate::local_operation_start_fixture::store_coordinate();
    let locator = ControlRecordCoordinate::new(
        semantic.handle_ref.clone(),
        ControlRecordDigest::parse("0".repeat(64)).expect("dummy locator digest"),
    );
    let recovery = OperationControlRecoveryInput {
        operation: LocalOperationHandleRecoveryInput {
            context: LocalOperationRecoveryContext {
                store: &store,
                profile: &profile,
                handle_signing_authority: semantic.context.signed.signing_authority,
                checkpoint_authority: semantic.checkpoint_authority,
                historical: &historical,
                maximum_record_bytes: 1 << 20,
                maximum_operation_lease_bytes: 1 << 20,
                maximum_session_lease_bytes: 1 << 20,
                membership_limits: pale_ports::AccumulatorMembershipLimits::new(3, 1024, 1 << 20)
                    .expect("membership limits"),
                observation_limits: OperationObservationRecoveryLimits::new(1, 1 << 20, 1 << 20)
                    .expect("observation limits"),
                checkpoint_limits: OperationCheckpointAncestryLimits::new(1, 1 << 20, 1)
                    .expect("checkpoint limits"),
            },
            expected_handle: &locator,
            maximum_parent_depth: 0,
        },
        observation: OperationObservationSelection::SelectedHead,
        observation_limits: OperationObservationRecoveryLimits::new(1, 1 << 20, 1 << 20)
            .expect("observation limits"),
        checkpoint_limits: OperationCheckpointAncestryLimits::new(1, 1 << 20, 1)
            .expect("checkpoint limits"),
        membership_limits: pale_ports::AccumulatorMembershipLimits::new(1, 1024, 1 << 20)
            .expect("membership limits"),
    };
    let mut admit_request = reject_request_admission;
    let mut admit_handle_authority = reject_handle_authority_admission;
    let mut verify_handle_signature = reject_handle_signature_verification;
    let mut admit_checkpoint = reject_checkpoint_admission;
    let mut resolve_observation_signer = reject_observation_signer_resolution;
    let mut admit_observation_state = reject_observation_state_admission;
    let mut admit_rotation = reject_rotation_admission;
    let mut verify_observation_signature = reject_observation_signature_verification;
    let mut verify_checkpoint_signature = reject_checkpoint_signature_verification;
    run(
        restart_input(recovery),
        OperationControlRecoveryAdmission {
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
}

fn host_without_generation<'store>(
    port: &'store MemoryAuthorityStore,
    selector: &ResidentDeploymentAuthoritySelector,
    store: &AuthorityStoreCoordinate,
) -> ResidentProcessHost<'store, UnprovedHeldWriter> {
    let application = ready_application(port, store.clone(), 80);
    ResidentProcessHost::new(
        unproved_writer(selector.principal_id.clone(), vec![store.clone()]),
        vec![unproved_deployment_for_selector(
            selector,
            store.clone(),
            application,
        )],
        NonZeroUsize::new(1).expect("one deployment"),
        ResidentOperationHost::new(task_state()),
    )
    .expect("one exact request-resolution deployment")
}

#[test]
fn fresh_request_composes_resolution_lease_and_durable_start_once() {
    let (mut generations, profile) = crate::resident::operation_observation_custody::tests::local_start::source_generation_copies(2);
    let retained_generation = generations.pop().expect("retained generation");
    let generation = generations.pop().expect("semantic generation");
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        "process-host-call-fresh-invoke",
        |_| {},
        |semantic, request, mut checkpoint_template, parents| {
            let store = crate::local_operation_start_fixture::store_coordinate();
            let selector = selector_from_signing_authority(parents.signing);
            let port = MemoryAuthorityStore::default();
            let mut host = host_with_generation(&port, &selector, &store, retained_generation);
            let reads = Arc::new(AtomicUsize::new(0));
            let mut custody = absent_start_store(request)
                .with_index_read_counter(Arc::clone(&reads))
                .with_unlisted_indexes_absent()
                .with_lease_retention(Ok(pale_ports::OperationCustodyCommitOutcome::Committed))
                .with_preparation(
                    initial_response,
                    pale_core::records::ControlTransactionSequenceV1::new(1)
                        .expect("initial transaction"),
                );
            let random = FreshEntropy {
                lookup_reads: Arc::clone(&reads),
                calls: AtomicUsize::new(0),
            };
            let mut authority = CountingAuthority::default();
            let mut anchor = initial_anchor();
            let mut policy = exact_invoke_policy(&semantic, 1 << 20);
            with_unreachable_restart_inputs(&semantic, |restart, admission| {
                let outcome = host
                    .invoke_request(
                        &selector,
                        ResidentDeploymentOperationAccess {
                            store: &store,
                            custody: &mut custody,
                            sessions: &port,
                            anchor: &mut anchor,
                        },
                        root_invoke_input(&semantic, request),
                        &random,
                        restart,
                        admission,
                        &mut authority,
                        &mut policy,
                        |_, _, _, _, _, _, _, _| Ok(()),
                        |_, _, _| Ok(local_operation_start_fixture::signature()),
                        |_, _, _| Ok(local_operation_start_fixture::signature()),
                        move |prepared, checkpoint_id| {
                            checkpoint_template.control_checkpoint_id = checkpoint_id.clone();
                            checkpoint_preimage(prepared, checkpoint_template)
                        },
                        |_, _, _| Ok(local_operation_start_fixture::signature()),
                    )
                    .unwrap_or_else(|error| match error {
                        ResidentProcessRootError::Capacity => {
                            panic!("fresh root Invoke capacity")
                        }
                        ResidentProcessRootError::Policy(reason) => {
                            panic!("fresh root Invoke policy: {reason}")
                        }
                        ResidentProcessRootError::Resolution(reason) => {
                            panic!("fresh root Invoke resolution: {reason:?}")
                        }
                        ResidentProcessRootError::Lease(_) => panic!("fresh root Invoke lease"),
                        ResidentProcessRootError::Start(_) => panic!("fresh root Invoke start"),
                    });
                let ResidentProcessRootOutcome::Started(issued) = outcome else {
                    panic!("an absent request must start once")
                };
                assert_eq!(
                    issued.request_coordinate().record_sha256().as_str(),
                    request.canonical_request_sha256().as_str()
                );
            });
            assert_eq!(reads.load(Ordering::SeqCst), 6);
            assert_eq!(random.calls.load(Ordering::SeqCst), 7);
            assert!(custody.retained_lease().is_some());
            assert!(authority.calls > 0);
        },
    );
}

#[test]
fn exhausted_fresh_start_capacity_refuses_only_an_absent_request() {
    let (mut generations, profile) = crate::resident::operation_observation_custody::tests::local_start::source_generation_copies(2);
    let retained_generation = generations.pop().expect("retained generation");
    let generation = generations.pop().expect("semantic generation");
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        "process-host-capacity-before-fresh-invoke",
        |_| {},
        |semantic, request, _, parents| {
            let store = crate::local_operation_start_fixture::store_coordinate();
            let selector = selector_from_signing_authority(parents.signing);
            let port = MemoryAuthorityStore::default();
            let mut host = host_with_generation(&port, &selector, &store, retained_generation);
            let reads = Arc::new(AtomicUsize::new(0));
            let mut custody =
                absent_start_store(request).with_index_read_counter(Arc::clone(&reads));
            let random = FreshEntropy {
                lookup_reads: Arc::clone(&reads),
                calls: AtomicUsize::new(0),
            };
            let policy_calls = Cell::new(0);
            let mut policy = RejectingInvokePolicy(&policy_calls);
            let mut authority = CountingAuthority::default();
            let mut anchor = RejectAnchorAccess;
            let mut input = root_invoke_input(&semantic, request);
            input.fresh_start_admission = ResidentProcessFreshStartAdmission::CapacityExhausted;

            with_unreachable_restart_inputs(&semantic, |restart, admission| {
                let result = host.invoke_request(
                    &selector,
                    ResidentDeploymentOperationAccess {
                        store: &store,
                        custody: &mut custody,
                        sessions: &port,
                        anchor: &mut anchor,
                    },
                    input,
                    &random,
                    restart,
                    admission,
                    &mut authority,
                    &mut policy,
                    |_, _, _, _, _, _, _, _| panic!("capacity cannot admit a fresh start"),
                    |_, _, _| panic!("capacity cannot sign a handle"),
                    |_, _, _| panic!("capacity cannot sign an observation"),
                    |_, _| panic!("capacity cannot prepare a checkpoint"),
                    |_, _, _| panic!("capacity cannot sign a checkpoint"),
                );
                assert!(matches!(result, Err(ResidentProcessRootError::Capacity)));
            });

            assert_eq!(policy_calls.get(), 0);
            assert_eq!(random.calls.load(Ordering::SeqCst), 0);
            assert_eq!(custody.mutation_counts(), (0, 0));
            assert_eq!(authority.calls, 0);
        },
    );
}

#[test]
fn unavailable_retained_policy_stops_after_replay_lookup_before_allocation_or_effect() {
    let (mut generations, profile) = crate::resident::operation_observation_custody::tests::local_start::source_generation_copies(2);
    let retained_generation = generations.pop().expect("retained generation");
    let generation = generations.pop().expect("semantic generation");
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        "process-host-policy-before-allocation",
        |_| {},
        |semantic, request, _, parents| {
            let store = crate::local_operation_start_fixture::store_coordinate();
            let selector = selector_from_signing_authority(parents.signing);
            let port = MemoryAuthorityStore::default();
            let mut host = host_with_generation(&port, &selector, &store, retained_generation);
            let reads = Arc::new(AtomicUsize::new(0));
            let mut custody =
                absent_start_store(request).with_index_read_counter(Arc::clone(&reads));
            let random = FreshEntropy {
                lookup_reads: Arc::clone(&reads),
                calls: AtomicUsize::new(0),
            };
            let calls = Cell::new(0);
            let mut policy = RejectingInvokePolicy(&calls);
            let mut authority = CountingAuthority::default();
            let mut anchor = RejectAnchorAccess;
            with_unreachable_restart_inputs(&semantic, |restart, admission| {
                let result = host.invoke_request(
                    &selector,
                    ResidentDeploymentOperationAccess {
                        store: &store,
                        custody: &mut custody,
                        sessions: &port,
                        anchor: &mut anchor,
                    },
                    root_invoke_input(&semantic, request),
                    &random,
                    restart,
                    admission,
                    &mut authority,
                    &mut policy,
                    |_, _, _, _, _, _, _, _| panic!("policy rejection cannot admit start"),
                    |_, _, _| panic!("policy rejection cannot sign handle"),
                    |_, _, _| panic!("policy rejection cannot sign observation"),
                    |_, _| panic!("policy rejection cannot prepare checkpoint"),
                    |_, _, _| panic!("policy rejection cannot sign checkpoint"),
                );
                assert!(matches!(
                    result,
                    Err(ResidentProcessRootError::Policy(
                        ResidentProcessHostSelectionError::StartRequestAssociation
                    ))
                ));
            });
            assert_eq!(calls.get(), 1);
            assert_eq!(reads.load(Ordering::SeqCst), 2);
            assert_eq!(random.calls.load(Ordering::SeqCst), 0);
            assert_eq!(custody.mutation_counts(), (0, 0));
            assert_eq!(authority.calls, 0);
        },
    );
}

#[test]
fn existing_request_returns_before_lease_or_start_work_at_exhausted_fresh_capacity() {
    let calls = Rc::new(Cell::new(0));
    let counted_calls = Rc::clone(&calls);
    with_process_host_invoke_request_fixtures(
        &mut move |fixture: ProcessHostInvokeRequestFixture<'_>| {
            if counted_calls.replace(counted_calls.get() + 1) != 0 {
                return;
            }
            let ProcessHostInvokeRequestFixture {
                recovery: recovery_input,
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
            let store = recovery_input.operation.context.store.clone();
            let checkpoint_authority = recovery_input.operation.context.checkpoint_authority;
            let selector = selector_from_signing_authority(signing_authority);
            let port = MemoryAuthorityStore::default();
            let mut host = host_with_generation(&port, &selector, &store, generation);
            let mut authority = CountingAuthority::default();
            let start_calls = Cell::new(0);
            let policy_calls = Cell::new(0);
            let (fresh_generation, fresh_profile) = crate::resident::operation_observation_custody::tests::local_start::source_generation();
            crate::local_operation_start_fixture::with_invoke_start_input(
                &fresh_generation,
                &fresh_profile,
                "process-host-call-existing-invoke",
                |_| {},
                |_fresh_semantic, _, _, _| {
                    let mut policy = RejectingInvokePolicy(&policy_calls);
                    let result = host
                        .invoke_request(
                            &selector,
                            ResidentDeploymentOperationAccess {
                                store: &store,
                                custody,
                                sessions,
                                anchor,
                            },
                            ResidentProcessRootInput {
                                request,
                                session,
                                current,
                                handle_signing_authority: signing_authority,
                                checkpoint_authority,
                                fresh_start_admission:
                                    ResidentProcessFreshStartAdmission::CapacityExhausted,
                            },
                            &RejectEntropy,
                            restart_input(recovery_input),
                            admission,
                            &mut authority,
                            &mut policy,
                            |_, _, _, _, _, _, _, _| {
                                start_calls.set(start_calls.get() + 1);
                                panic!("existing work cannot enter start admission")
                            },
                            |_, _, _| {
                                start_calls.set(start_calls.get() + 1);
                                panic!("existing work cannot sign a handle")
                            },
                            |_, _, _| {
                                start_calls.set(start_calls.get() + 1);
                                panic!("existing work cannot sign an observation")
                            },
                            |_, _| {
                                start_calls.set(start_calls.get() + 1);
                                panic!("existing work cannot prepare a checkpoint")
                            },
                            |_, _, _| {
                                start_calls.set(start_calls.get() + 1);
                                panic!("existing work cannot sign a checkpoint")
                            },
                        )
                        .unwrap_or_else(|_| panic!("existing root Invoke call"));
                    assert!(matches!(
                        result,
                        ResidentProcessRootOutcome::ExistingOperation(_)
                    ));
                },
            );
            assert_eq!(authority.calls, 0);
            assert_eq!(start_calls.get(), 0);
            assert_eq!(policy_calls.get(), 0);
        },
    );
    assert_eq!(calls.get(), 8);
}

#[test]
fn fresh_request_resolves_after_lookup_without_session_or_anchor_access() {
    let (mut generations, profile) = crate::resident::operation_observation_custody::tests::local_start::source_generation_copies(2);
    let retained_generation = generations.pop().expect("retained generation");
    let generation = generations.pop().expect("semantic generation");
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        "process-host-resolve-fresh-invoke",
        |_| {},
        |semantic, request, _, parents| {
            let store = crate::local_operation_start_fixture::store_coordinate();
            let selector = selector_from_signing_authority(parents.signing);
            let port = MemoryAuthorityStore::default();
            let host = host_with_generation(&port, &selector, &store, retained_generation);
            let reads = Arc::new(AtomicUsize::new(0));
            let mut custody =
                absent_start_store(request).with_index_read_counter(Arc::clone(&reads));
            let random = FreshEntropy {
                lookup_reads: Arc::clone(&reads),
                calls: AtomicUsize::new(0),
            };
            let sessions = RejectSessionAccess;
            let mut anchor = RejectAnchorAccess;
            assert_eq!(
                &request.session_admission().session_admission_id,
                semantic.session.lease().session_admission_id()
            );
            assert_eq!(
                &request.session_admission().session_admission_lease_digest,
                semantic.session.lease().session_admission_lease_digest()
            );
            let retained = host.deployments[0]
                .application
                .generation(semantic.session.lease().compiled_generation_digest())
                .expect("retained exact generation");
            assert!(retained
                .operation(
                    semantic.session.lease().repository_id(),
                    request.operation()
                )
                .is_some());
            with_unreachable_restart_inputs(&semantic, |restart, admission| {
                let outcome = host
                    .resolve_invoke_request(
                        &selector,
                        OperationClassV1::Invoke,
                        ResidentDeploymentOperationAccess {
                            store: &store,
                            custody: &mut custody,
                            sessions: &sessions,
                            anchor: &mut anchor,
                        },
                        ResidentProcessOperationStartContext {
                            session: semantic.session,
                            current: semantic.current,
                            request,
                            handle_signing_authority: semantic.context.signed.signing_authority,
                            semantic_checkpoint_authority: semantic.checkpoint_authority,
                            issuance_checkpoint_authority: semantic.checkpoint_authority,
                            maximum_record_bytes: 1 << 20,
                        },
                        &random,
                        restart,
                        admission,
                    )
                    .unwrap_or_else(|error| panic!("fresh request resolution: {error:?}"));
                let ResidentProcessInvokeRequestResolution::FreshStart(fresh) = outcome else {
                    panic!("absent request must return a fresh inert allocation")
                };
                assert_eq!(fresh.selector, selector);
                assert_eq!(fresh.request_sha256, *request.canonical_request_sha256());
            });
            assert_eq!(reads.load(Ordering::SeqCst), 2);
            assert_eq!(random.calls.load(Ordering::SeqCst), 7);
            assert_eq!(custody.mutation_counts(), (0, 0));
            assert_eq!(host.provider_routes.reserved, 0);
            assert_eq!(host.provider_routes.checked_out, 0);
            assert!(host.provider_routes.active.is_empty());
        },
    );
}

#[test]
fn lookup_failure_never_enters_restart_or_fresh_allocation() {
    let (mut generations, profile) = crate::resident::operation_observation_custody::tests::local_start::source_generation_copies(2);
    let retained_generation = generations.pop().expect("retained generation");
    let generation = generations.pop().expect("semantic generation");
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        "process-host-resolve-lookup-failure",
        |_| {},
        |semantic, request, _, parents| {
            let store = crate::local_operation_start_fixture::store_coordinate();
            let selector = selector_from_signing_authority(parents.signing);
            let port = MemoryAuthorityStore::default();
            let host = host_with_generation(&port, &selector, &store, retained_generation);
            let mut custody = RecordingStore::recovering(Vec::new())
                .with_index_error(ControlStoreError::new(ControlStoreErrorKind::Storage));
            let sessions = RejectSessionAccess;
            let mut anchor = RejectAnchorAccess;
            with_unreachable_restart_inputs(&semantic, |restart, admission| {
                let result = host.resolve_invoke_request(
                    &selector,
                    OperationClassV1::Invoke,
                    ResidentDeploymentOperationAccess {
                        store: &store,
                        custody: &mut custody,
                        sessions: &sessions,
                        anchor: &mut anchor,
                    },
                    ResidentProcessOperationStartContext {
                        session: semantic.session,
                        current: semantic.current,
                        request,
                        handle_signing_authority: semantic.context.signed.signing_authority,
                        semantic_checkpoint_authority: semantic.checkpoint_authority,
                        issuance_checkpoint_authority: semantic.checkpoint_authority,
                        maximum_record_bytes: 1 << 20,
                    },
                    &RejectEntropy,
                    restart,
                    admission,
                );
                assert!(matches!(
                    result,
                    Err(ResidentProcessInvokeRequestResolutionError::Allocation(
                        ResidentProcessOperationAllocationError::Allocation(
                            OperationStartAllocationError::Recovery(_)
                        )
                    ))
                ));
            });
            assert_eq!(custody.mutation_counts(), (0, 0));
            assert_eq!(host.provider_routes.reserved, 0);
            assert_eq!(host.provider_routes.checked_out, 0);
            assert!(host.provider_routes.active.is_empty());
        },
    );
}

#[test]
fn existing_invoke_rejects_a_consult_handle_and_preserves_valid_dispositions_without_entropy() {
    let terminal = Rc::new(Cell::new(0));
    let recovery = Rc::new(Cell::new(0));
    let consultation = Rc::new(Cell::new(0));
    let local_cancellation = Rc::new(Cell::new(0));
    let native = Rc::new(Cell::new(0));
    let provider = Rc::new(Cell::new(0));
    let provider_resolved = Rc::new(Cell::new(0));
    let prepared = Rc::new(Cell::new(0));
    let unlike_class = Rc::new(Cell::new(0));
    let counted_terminal = Rc::clone(&terminal);
    let counted_recovery = Rc::clone(&recovery);
    let counted_consultation = Rc::clone(&consultation);
    let counted_local_cancellation = Rc::clone(&local_cancellation);
    let counted_native = Rc::clone(&native);
    let counted_provider = Rc::clone(&provider);
    let counted_provider_resolved = Rc::clone(&provider_resolved);
    let counted_prepared = Rc::clone(&prepared);
    let counted_unlike_class = Rc::clone(&unlike_class);
    with_process_host_invoke_request_fixtures(
        &mut move |fixture: ProcessHostInvokeRequestFixture<'_>| {
            let ProcessHostInvokeRequestFixture {
                recovery: recovery_input,
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
            let store = recovery_input.operation.context.store.clone();
            let selector = selector_from_signing_authority(signing_authority);
            let port = MemoryAuthorityStore::default();
            let host = host_with_generation(&port, &selector, &store, generation);
            let maximum_record_bytes = recovery_input.operation.context.maximum_record_bytes;
            let checkpoint_authority = recovery_input.operation.context.checkpoint_authority;
            let result = host.resolve_invoke_request(
                &selector,
                OperationClassV1::Invoke,
                ResidentDeploymentOperationAccess {
                    store: &store,
                    custody,
                    sessions,
                    anchor,
                },
                ResidentProcessOperationStartContext {
                    session,
                    current,
                    request,
                    handle_signing_authority: signing_authority,
                    semantic_checkpoint_authority: checkpoint_authority,
                    issuance_checkpoint_authority: checkpoint_authority,
                    maximum_record_bytes,
                },
                &RejectEntropy,
                restart_input(recovery_input),
                admission,
            );
            if matches!(
                result,
                Err(
                    ResidentProcessInvokeRequestResolutionError::ExistingOperation(
                        ResidentProcessOperationRestartError::State
                    )
                )
            ) {
                counted_unlike_class.set(counted_unlike_class.get() + 1);
                return;
            }
            let result = result.unwrap_or_else(|_| panic!("retained request must resolve"));
            let ResidentProcessInvokeRequestResolution::ExistingOperation(disposition) = result
            else {
                panic!("retained request must not allocate a second start")
            };
            match *disposition {
                ResidentProcessOperationRestartDisposition::NativeHookResponse(_) => {
                    panic!("ordinary Invoke fixture cannot become native hook response recovery")
                }
                ResidentProcessOperationRestartDisposition::Terminal(_) => {
                    counted_terminal.set(counted_terminal.get() + 1);
                }
                ResidentProcessOperationRestartDisposition::RecoveryOnly(_) => {
                    counted_recovery.set(counted_recovery.get() + 1);
                }
                ResidentProcessOperationRestartDisposition::LocalCancellationRequested(_) => {
                    counted_local_cancellation.set(counted_local_cancellation.get() + 1);
                }
                ResidentProcessOperationRestartDisposition::OperationResolution(_) => {
                    counted_recovery.set(counted_recovery.get() + 1);
                }
                ResidentProcessOperationRestartDisposition::Consult(_) => {
                    counted_consultation.set(counted_consultation.get() + 1);
                }
                ResidentProcessOperationRestartDisposition::Native(_) => {
                    counted_native.set(counted_native.get() + 1);
                }
                ResidentProcessOperationRestartDisposition::Provider(_) => {
                    counted_provider.set(counted_provider.get() + 1);
                }
                ResidentProcessOperationRestartDisposition::ProviderResolved(_) => {
                    counted_provider_resolved.set(counted_provider_resolved.get() + 1);
                }
                ResidentProcessOperationRestartDisposition::NativePrepared(_)
                | ResidentProcessOperationRestartDisposition::ProviderPrepared(_) => {
                    counted_prepared.set(counted_prepared.get() + 1);
                }
                ResidentProcessOperationRestartDisposition::A2aCancelTaskRequestRecoveryOnly(_) => {
                    panic!("general Invoke fixtures contain no A2A CancelTask request")
                }
                ResidentProcessOperationRestartDisposition::A2aDelegateSendRecoveryOnly(_) => {
                    panic!("general Invoke fixtures contain no A2A Delegate send")
                }
                ResidentProcessOperationRestartDisposition::A2aTask(_) => {
                    panic!("general Invoke fixtures contain no A2A Task")
                }
            }
            assert_eq!(host.provider_routes.reserved, 0);
            assert_eq!(host.provider_routes.checked_out, 0);
            assert!(host.provider_routes.active.is_empty());
        },
    );
    assert_eq!(terminal.get(), 1);
    assert_eq!(recovery.get(), 3);
    assert_eq!(local_cancellation.get(), 1);
    assert_eq!(consultation.get(), 0);
    assert_eq!(native.get(), 1);
    assert_eq!(provider.get(), 1);
    assert_eq!(provider_resolved.get(), 0);
    assert_eq!(prepared.get(), 0);
    assert_eq!(unlike_class.get(), 1);
}

#[test]
fn classification_failure_never_becomes_a_fresh_start() {
    let calls = Rc::new(Cell::new(0));
    let counted_calls = Rc::clone(&calls);
    with_process_host_invoke_request_fixtures(
        &mut move |fixture: ProcessHostInvokeRequestFixture<'_>| {
            if counted_calls.replace(counted_calls.get() + 1) != 5 {
                return;
            }
            let ProcessHostInvokeRequestFixture {
                recovery: recovery_input,
                admission,
                request,
                session,
                current,
                signing_authority,
                generation: _,
                custody,
                sessions,
                anchor,
                ..
            } = fixture;
            let store = recovery_input.operation.context.store.clone();
            let selector = selector_from_signing_authority(signing_authority);
            let port = MemoryAuthorityStore::default();
            let application = ready_application(&port, store.clone(), 82);
            let host = ResidentProcessHost::new(
                unproved_writer(selector.principal_id.clone(), vec![store.clone()]),
                vec![unproved_deployment_for_selector(
                    &selector,
                    store.clone(),
                    application,
                )],
                NonZeroUsize::new(1).expect("one deployment"),
                ResidentOperationHost::new(task_state()),
            )
            .expect("one empty request-resolution deployment");
            let maximum_record_bytes = recovery_input.operation.context.maximum_record_bytes;
            let checkpoint_authority = recovery_input.operation.context.checkpoint_authority;
            let result = host.resolve_invoke_request(
                &selector,
                OperationClassV1::Invoke,
                ResidentDeploymentOperationAccess {
                    store: &store,
                    custody,
                    sessions,
                    anchor,
                },
                ResidentProcessOperationStartContext {
                    session,
                    current,
                    request,
                    handle_signing_authority: signing_authority,
                    semantic_checkpoint_authority: checkpoint_authority,
                    issuance_checkpoint_authority: checkpoint_authority,
                    maximum_record_bytes,
                },
                &RejectEntropy,
                restart_input(recovery_input),
                admission,
            );
            assert!(matches!(
                result,
                Err(ResidentProcessInvokeRequestResolutionError::Allocation(
                    ResidentProcessOperationAllocationError::Selection(
                        ResidentProcessHostSelectionError::StartRequestAssociation
                    )
                ))
            ));
            assert_eq!(host.provider_routes.reserved, 0);
            assert_eq!(host.provider_routes.checked_out, 0);
            assert!(host.provider_routes.active.is_empty());
        },
    );
    assert_eq!(calls.get(), 8);
}
