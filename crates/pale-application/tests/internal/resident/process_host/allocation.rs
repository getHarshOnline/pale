//! Hostile laws for selector-bound Invoke identity allocation.

use super::*;
use crate::resident::operation_observation_custody::tests::local_start::{
    absent_start_store, initial_anchor, initial_response, source_generation_copies,
};
use crate::resident::operation_start_allocation::tests::existing_reads;
use crate::resident::process_host::tests::start::{checkpoint_preimage, CountingAuthority};
use pale_core::records::{
    CommonRequestEnvelopePreimageV1, OperationLeaseV1, PackageIdV1, PackageResourceIdV1,
    SessionAdmissionIdV1, SessionAdmissionLeaseDigest,
};
use pale_ports::{
    ControlStoreError, ControlStoreErrorKind, CryptographicRandomSourceError,
    CryptographicRandomSourcePort, OperationCustodyIndexObservation,
};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

struct OrderedRandom {
    calls: AtomicUsize,
    completed_lookup_reads: Arc<AtomicUsize>,
    fail_at_draw: Option<usize>,
}

impl OrderedRandom {
    fn succeeding(completed_lookup_reads: Arc<AtomicUsize>) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            completed_lookup_reads,
            fail_at_draw: None,
        }
    }

    fn failing_at(completed_lookup_reads: Arc<AtomicUsize>, draw: usize) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            completed_lookup_reads,
            fail_at_draw: Some(draw),
        }
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl CryptographicRandomSourcePort for OrderedRandom {
    fn fill_bytes(&self, destination: &mut [u8]) -> Result<(), CryptographicRandomSourceError> {
        assert_eq!(
            self.completed_lookup_reads.load(Ordering::SeqCst),
            2,
            "both request indexes must be read before the first entropy draw"
        );
        let call = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        if self.fail_at_draw == Some(call) {
            return Err(CryptographicRandomSourceError::unavailable());
        }
        destination.fill(u8::try_from(call).expect("bounded allocation draw"));
        Ok(())
    }
}

fn host_for_allocation<'store>(
    selected_port: &'store MemoryAuthorityStore,
    other_port: &'store MemoryAuthorityStore,
    selector: &ResidentDeploymentAuthoritySelector,
    selected_store: &AuthorityStoreCoordinate,
    other_store: &AuthorityStoreCoordinate,
    generation: crate::resident::AdmittedCompiledGeneration,
) -> ResidentProcessHost<'store, UnprovedHeldWriter> {
    let mut application = ready_application(selected_port, selected_store.clone(), 1);
    application.fixture_retain_generation(generation);
    let selected = unproved_deployment_for_selector(selector, selected_store.clone(), application);
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
    .expect("two exact allocation deployments")
}

fn allocation_context<'a>(
    semantic: &'a crate::resident::LocalOperationStartInput<'a>,
    request: &'a CommonRequestEnvelopeV1,
    maximum_record_bytes: usize,
) -> ResidentProcessOperationStartContext<'a> {
    ResidentProcessOperationStartContext {
        session: semantic.session,
        current: semantic.current,
        request,
        handle_signing_authority: semantic.context.signed.signing_authority,
        semantic_checkpoint_authority: semantic.checkpoint_authority,
        issuance_checkpoint_authority: semantic.checkpoint_authority,
        maximum_record_bytes,
    }
}

fn request_with_change(
    request: &CommonRequestEnvelopeV1,
    change: impl FnOnce(&mut CommonRequestEnvelopePreimageV1),
) -> CommonRequestEnvelopeV1 {
    let mut preimage = request.to_preimage();
    change(&mut preimage);
    crate::effect_case_lineage_fixture::request(&preimage)
}

fn lease_basis(lease: &OperationLeaseV1) -> ResidentProcessOperationLeaseBasis {
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

fn start_basis<'a>(
    semantic: &'a crate::resident::LocalOperationStartInput<'a>,
) -> ResidentProcessInvokeStartBasis<'a> {
    ResidentProcessInvokeStartBasis {
        checkpoint_profile: semantic.checkpoint_profile,
        request: semantic.request,
        session: semantic.session,
        current: semantic.current,
        publication_policy: crate::resident::LocalOperationPublicationPolicyInput {
            target_scope: semantic.publication_policy.target_scope,
            classification: semantic.publication_policy.classification,
            visibility: semantic.publication_policy.visibility,
            disclosure_ceiling: semantic.publication_policy.disclosure_ceiling,
            payload_resolution_policy: semantic.publication_policy.payload_resolution_policy,
            actor_provenance: semantic.publication_policy.actor_provenance,
        },
        handle_signing_authority: semantic.context.signed.signing_authority,
        checkpoint_authority: semantic.checkpoint_authority,
        handle_expires_at: semantic.handle.expires_at,
        handle_retain_until: semantic.handle.retain_until,
        accumulator_limits: pale_ports::AccumulatorPreparationLimits::new(
            4, 1024, 1024, 512, 65_536,
        )
        .expect("bounded accumulator"),
    }
}

fn assert_no_non_lookup_effects(
    host: &ResidentProcessHost<'_, UnprovedHeldWriter>,
    custody: &RecordingStore,
    other_custody: &RecordingStore,
    selected_port: &MemoryAuthorityStore,
    other_port: &MemoryAuthorityStore,
) {
    assert_eq!(custody.mutation_counts(), (0, 0));
    assert_eq!(other_custody.mutation_counts(), (0, 0));
    assert_eq!(selected_port.entry_count(), 0);
    assert_eq!(other_port.entry_count(), 0);
    assert_eq!(host.provider_routes.reserved, 0);
    assert_eq!(host.provider_routes.checked_out, 0);
    assert!(host.provider_routes.active.is_empty());
}

fn source_generation_triplet() -> (
    crate::resident::AdmittedCompiledGeneration,
    crate::resident::AdmittedCompiledGeneration,
    crate::resident::AdmittedCompiledGeneration,
    pale_core::records::EffectiveProjectProfileV1,
) {
    let (mut generations, profile) = source_generation_copies(3);
    let replacement = generations.pop().expect("replacement generation");
    let retained = generations.pop().expect("retained generation");
    let semantic = generations.pop().expect("semantic generation");
    (semantic, retained, replacement, profile)
}

#[test]
fn fresh_allocation_reads_the_selected_request_before_exactly_seven_draws() {
    let (generation, retained_generation, replacement_generation, profile) =
        source_generation_triplet();
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        "process-host-allocation-fresh",
        |_| {},
        |semantic, request, _, parents| {
            let selected_store = crate::local_operation_start_fixture::store_coordinate();
            let other_store = store("other-allocation-store", "other-allocation-incarnation");
            let selector = selector_from_signing_authority(parents.signing);
            let selected_port = MemoryAuthorityStore::default();
            let other_port = MemoryAuthorityStore::default();
            let host = host_for_allocation(
                &selected_port,
                &other_port,
                &selector,
                &selected_store,
                &other_store,
                retained_generation,
            );
            let reads = Arc::new(AtomicUsize::new(0));
            let mut custody =
                absent_start_store(request).with_index_read_counter(Arc::clone(&reads));
            let other_reads = Arc::new(AtomicUsize::new(0));
            let other_custody = RecordingStore::recovering(Vec::new())
                .at_store(&other_store)
                .with_index_read_counter(Arc::clone(&other_reads));
            let random = OrderedRandom::succeeding(Arc::clone(&reads));

            let outcome = host
                .allocate_operation_start(
                    &selector,
                    ResidentDeploymentOperationLookup {
                        store: &selected_store,
                        custody: &mut custody,
                    },
                    allocation_context(&semantic, request, 1 << 20),
                    &random,
                )
                .unwrap_or_else(|_| panic!("fresh allocation"));
            let ResidentProcessOperationStartAllocation::Fresh(pending) = outcome else {
                panic!("absent request must allocate once")
            };
            assert_eq!(random.calls(), 7);
            assert_eq!(reads.load(Ordering::SeqCst), 2);
            assert_eq!(custody.index_query_count(), 2);
            assert_eq!(other_reads.load(Ordering::SeqCst), 0);
            assert_eq!(other_custody.index_query_count(), 0);
            assert_eq!(pending.selector, selector);
            assert!(pending
                .owner_instance
                .is_same(host.operations.operation_task_owner_instance()));
            assert_eq!(pending.request_ref, *pending.allocation.request_ref());
            assert_eq!(pending.request_sha256, *request.canonical_request_sha256());
            assert_eq!(pending.maximum_record_bytes, 1 << 20);
            assert_eq!(pending.allocation.store(), &selected_store);
            let allocation_address = pending.allocation.as_ref() as *const _;

            let pending = match host.reselect_fresh_operation_start(*pending, &other_store) {
                Err(failure) => {
                    assert!(matches!(
                        failure.reason,
                        ResidentProcessHostSelectionError::AccessStoreAssociation
                    ));
                    *failure.pending
                }
                Ok(_) => panic!("another deployment cannot consume the fresh allocation"),
            };
            assert_eq!(pending.allocation.as_ref() as *const _, allocation_address);

            let rebuilt_selected_port = MemoryAuthorityStore::default();
            let rebuilt_other_port = MemoryAuthorityStore::default();
            let rebuilt = host_for_allocation(
                &rebuilt_selected_port,
                &rebuilt_other_port,
                &selector,
                &selected_store,
                &other_store,
                replacement_generation,
            );
            let pending = match rebuilt.reselect_fresh_operation_start(pending, &selected_store) {
                Err(failure) => {
                    assert!(matches!(
                        failure.reason,
                        ResidentProcessHostSelectionError::StartProcessOwner
                    ));
                    *failure.pending
                }
                Ok(_) => panic!("rebuilt host cannot consume the fresh allocation"),
            };
            assert_eq!(pending.allocation.as_ref() as *const _, allocation_address);
            let pending = host
                .reselect_fresh_operation_start(pending, &selected_store)
                .unwrap_or_else(|_| panic!("original host retains the exact allocation"));
            assert_eq!(pending.allocation.as_ref() as *const _, allocation_address);
            assert_no_non_lookup_effects(
                &host,
                &custody,
                &other_custody,
                &selected_port,
                &other_port,
            );
        },
    );
}

#[test]
fn fresh_allocation_owns_every_identity_through_retained_lease_and_durable_start() {
    let (generation, retained_generation, _, profile) = source_generation_triplet();
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        "process-host-allocated-start",
        |_| {},
        |semantic, request, mut checkpoint_template, parents| {
            let selected_store = crate::local_operation_start_fixture::store_coordinate();
            let other_store = store("other-allocation-store", "other-allocation-incarnation");
            let selector = selector_from_signing_authority(parents.signing);
            let selected_port = MemoryAuthorityStore::default();
            let other_port = MemoryAuthorityStore::default();
            let mut host = host_for_allocation(
                &selected_port,
                &other_port,
                &selector,
                &selected_store,
                &other_store,
                retained_generation,
            );
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
            let random = OrderedRandom::succeeding(Arc::clone(&reads));
            let fresh = match host
                .allocate_operation_start(
                    &selector,
                    ResidentDeploymentOperationLookup {
                        store: &selected_store,
                        custody: &mut custody,
                    },
                    allocation_context(&semantic, request, 1 << 20),
                    &random,
                )
                .unwrap_or_else(|_| panic!("fresh allocation"))
            {
                ResidentProcessOperationStartAllocation::Fresh(fresh) => *fresh,
                ResidentProcessOperationStartAllocation::Existing(_) => {
                    panic!("absent request must allocate")
                }
            };
            let expected_request_ref = fresh.allocation.request_ref().clone();
            let expected_lease_id = fresh.allocation.lease_id().clone();
            let expected_lease_ref = fresh.allocation.lease_ref().clone();
            let expected_handle_id = fresh.allocation.handle_id().clone();
            let expected_handle_ref = fresh.allocation.handle_ref().clone();
            let expected_observation_ref = fresh.allocation.observation_ref().clone();
            let expected_checkpoint_id = fresh.allocation.checkpoint_id().clone();
            let expected_checkpoint_ref = fresh.allocation.checkpoint_ref().clone();
            let mut authority = CountingAuthority::default();
            let retained = host
                .admit_and_retain_fresh_operation_lease(
                    fresh,
                    ResidentDeploymentOperationLeaseAccess {
                        store: &selected_store,
                        custody: &mut custody,
                    },
                    allocation_context(&semantic, request, 1 << 20),
                    &mut authority,
                    lease_basis(semantic.context.operation_lease),
                )
                .unwrap_or_else(|_| panic!("allocated lease admission and retention"));
            assert_eq!(retained.lease.operation_lease_id(), &expected_lease_id);
            assert_eq!(retained.fresh.allocation.lease_ref(), &expected_lease_ref);
            let retained_record = custody.retained_lease().expect("retained lease body");
            assert_eq!(
                retained_record.coordinate().record_ref(),
                &expected_lease_ref
            );
            assert_eq!(
                retained_record.coordinate().record_sha256().as_str(),
                retained.lease.operation_lease_digest().as_str()
            );

            checkpoint_template.control_checkpoint_id = expected_checkpoint_id.clone();
            let mut anchor = initial_anchor();
            let issued = host
                .start_allocated_invoke(
                    retained,
                    ResidentDeploymentOperationAccess {
                        store: &selected_store,
                        custody: &mut custody,
                        sessions: &selected_port,
                        anchor: &mut anchor,
                    },
                    &mut authority,
                    start_basis(&semantic),
                    |actual_request, _, _, handle, lease, selected, _, _| {
                        assert_eq!(
                            actual_request.canonical_request_sha256(),
                            request.canonical_request_sha256()
                        );
                        assert_eq!(handle.operation_handle_id, expected_handle_id);
                        assert_eq!(handle.operation_lease_ref, expected_lease_ref);
                        assert_eq!(lease.operation_lease_id(), &expected_lease_id);
                        assert_eq!(selected.record.id, *lease.operation());
                        Ok(())
                    },
                    |_, handle, _| {
                        assert_eq!(handle.operation_handle_id, expected_handle_id);
                        Ok(local_operation_start_fixture::signature())
                    },
                    |_, observation, _| {
                        assert_eq!(observation.operation_handle_id, expected_handle_id);
                        Ok(local_operation_start_fixture::signature())
                    },
                    move |prepared| checkpoint_preimage(prepared, checkpoint_template),
                    |_, checkpoint, _| {
                        assert_eq!(checkpoint.control_checkpoint_id, expected_checkpoint_id);
                        Ok(local_operation_start_fixture::signature())
                    },
                )
                .unwrap_or_else(|_| panic!("allocated start reaches durable custody"));
            assert_eq!(issued.handle().0, &expected_handle_ref);
            assert_eq!(issued.handle().1.operation_handle_id(), &expected_handle_id);
            assert_eq!(issued.handle().1.operation_lease_ref(), &expected_lease_ref);
            assert_eq!(
                issued.historical_operation_lease().operation_lease_id(),
                &expected_lease_id
            );
            let committed = custody.committed_candidate().expect("committed start");
            assert_eq!(
                committed
                    .record(pale_ports::OperationCustodyRecordKind::CommonRequest)
                    .expect("request record")
                    .coordinate()
                    .record_ref(),
                &expected_request_ref
            );
            assert_eq!(
                committed
                    .record(pale_ports::OperationCustodyRecordKind::OperationObservation)
                    .expect("observation record")
                    .coordinate()
                    .record_ref(),
                &expected_observation_ref
            );
            assert_eq!(
                committed
                    .checkpoint()
                    .successor_record()
                    .coordinate()
                    .record_ref(),
                &expected_checkpoint_ref
            );
        },
    );
}

#[test]
fn changed_request_cannot_admit_or_retain_an_existing_fresh_allocation() {
    let (generation, retained_generation, _, profile) = source_generation_triplet();
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        "process-host-allocated-request-fence",
        |_| {},
        |semantic, request, _, parents| {
            let selected_store = crate::local_operation_start_fixture::store_coordinate();
            let other_store = store("other-allocation-store", "other-allocation-incarnation");
            let selector = selector_from_signing_authority(parents.signing);
            let selected_port = MemoryAuthorityStore::default();
            let other_port = MemoryAuthorityStore::default();
            let host = host_for_allocation(
                &selected_port,
                &other_port,
                &selector,
                &selected_store,
                &other_store,
                retained_generation,
            );
            let reads = Arc::new(AtomicUsize::new(0));
            let mut custody =
                absent_start_store(request).with_index_read_counter(Arc::clone(&reads));
            let random = OrderedRandom::succeeding(Arc::clone(&reads));
            let fresh = match host
                .allocate_operation_start(
                    &selector,
                    ResidentDeploymentOperationLookup {
                        store: &selected_store,
                        custody: &mut custody,
                    },
                    allocation_context(&semantic, request, 1 << 20),
                    &random,
                )
                .unwrap_or_else(|_| panic!("fresh allocation"))
            {
                ResidentProcessOperationStartAllocation::Fresh(fresh) => *fresh,
                ResidentProcessOperationStartAllocation::Existing(_) => {
                    panic!("absent request must allocate")
                }
            };
            let allocation_address = fresh.allocation.as_ref() as *const _;
            let mut changed = request.to_preimage();
            changed.scope.purpose =
                pale_core::records::BoundedResidentTextV1::parse("changed after allocation")
                    .expect("changed purpose");
            let changed = crate::effect_case_lineage_fixture::request(&changed);
            let mut authority = CountingAuthority::default();
            let failure = match host.admit_and_retain_fresh_operation_lease(
                fresh,
                ResidentDeploymentOperationLeaseAccess {
                    store: &selected_store,
                    custody: &mut custody,
                },
                allocation_context(&semantic, &changed, 1 << 20),
                &mut authority,
                lease_basis(semantic.context.operation_lease),
            ) {
                Err(failure) => failure,
                Ok(_) => panic!("changed request must not consume the allocation"),
            };
            let ResidentProcessOperationLeaseFailure::Selection { pending, reason } = failure
            else {
                panic!("changed request is a selection fence")
            };
            assert!(matches!(
                reason,
                ResidentProcessHostSelectionError::StartAllocationAssociation
            ));
            assert_eq!(pending.allocation.as_ref() as *const _, allocation_address);
            assert_eq!(authority.calls, 0);
            assert_eq!(custody.mutation_counts(), (0, 0));
            assert!(custody.retained_lease().is_none());
        },
    );
}

#[test]
fn lease_admission_failure_returns_the_exact_fresh_allocation() {
    let (generation, retained_generation, _, profile) = source_generation_triplet();
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        "process-host-allocated-lease-admission-failure",
        |_| {},
        |semantic, request, _, parents| {
            let selected_store = crate::local_operation_start_fixture::store_coordinate();
            let other_store = store("other-allocation-store", "other-allocation-incarnation");
            let selector = selector_from_signing_authority(parents.signing);
            let selected_port = MemoryAuthorityStore::default();
            let other_port = MemoryAuthorityStore::default();
            let host = host_for_allocation(
                &selected_port,
                &other_port,
                &selector,
                &selected_store,
                &other_store,
                retained_generation,
            );
            let reads = Arc::new(AtomicUsize::new(0));
            let mut custody =
                absent_start_store(request).with_index_read_counter(Arc::clone(&reads));
            let random = OrderedRandom::succeeding(Arc::clone(&reads));
            let fresh = match host
                .allocate_operation_start(
                    &selector,
                    ResidentDeploymentOperationLookup {
                        store: &selected_store,
                        custody: &mut custody,
                    },
                    allocation_context(&semantic, request, 1 << 20),
                    &random,
                )
                .unwrap_or_else(|_| panic!("fresh allocation"))
            {
                ResidentProcessOperationStartAllocation::Fresh(fresh) => *fresh,
                ResidentProcessOperationStartAllocation::Existing(_) => {
                    panic!("absent request must allocate")
                }
            };
            let allocation_address = fresh.allocation.as_ref() as *const _;
            let mut basis = lease_basis(semantic.context.operation_lease);
            basis.expires_at = UnixTimeV1::new(101).expect("expiry outside the parent lease");
            let mut authority = CountingAuthority::default();
            let failure = match host.admit_and_retain_fresh_operation_lease(
                fresh,
                ResidentDeploymentOperationLeaseAccess {
                    store: &selected_store,
                    custody: &mut custody,
                },
                allocation_context(&semantic, request, 1 << 20),
                &mut authority,
                basis,
            ) {
                Err(failure) => failure,
                Ok(_) => panic!("widened expiry must fail lease admission"),
            };
            let ResidentProcessOperationLeaseFailure::Admission { pending, .. } = failure else {
                panic!("widened expiry must be an admission failure")
            };
            assert_eq!(pending.allocation.as_ref() as *const _, allocation_address);
            assert_eq!(custody.mutation_counts(), (0, 0));
            assert!(custody.retained_lease().is_none());
        },
    );
}

#[test]
fn lease_retention_failure_preserves_the_exact_admitted_lease_phase() {
    let (generation, retained_generation, _, profile) = source_generation_triplet();
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        "process-host-allocated-lease-failure",
        |_| {},
        |semantic, request, _, parents| {
            let selected_store = crate::local_operation_start_fixture::store_coordinate();
            let other_store = store("other-allocation-store", "other-allocation-incarnation");
            let selector = selector_from_signing_authority(parents.signing);
            let selected_port = MemoryAuthorityStore::default();
            let other_port = MemoryAuthorityStore::default();
            let host = host_for_allocation(
                &selected_port,
                &other_port,
                &selector,
                &selected_store,
                &other_store,
                retained_generation,
            );
            let reads = Arc::new(AtomicUsize::new(0));
            let mut custody = absent_start_store(request)
                .with_index_read_counter(Arc::clone(&reads))
                .with_lease_retention(Err(ControlStoreError::new(ControlStoreErrorKind::Storage)));
            let random = OrderedRandom::succeeding(Arc::clone(&reads));
            let fresh = match host
                .allocate_operation_start(
                    &selector,
                    ResidentDeploymentOperationLookup {
                        store: &selected_store,
                        custody: &mut custody,
                    },
                    allocation_context(&semantic, request, 1 << 20),
                    &random,
                )
                .unwrap_or_else(|_| panic!("fresh allocation"))
            {
                ResidentProcessOperationStartAllocation::Fresh(fresh) => *fresh,
                ResidentProcessOperationStartAllocation::Existing(_) => {
                    panic!("absent request must allocate")
                }
            };
            let allocation_address = fresh.allocation.as_ref() as *const _;
            let expected_lease_id = fresh.allocation.lease_id().clone();
            let expected_lease_ref = fresh.allocation.lease_ref().clone();
            let mut authority = CountingAuthority::default();
            let failure = match host.admit_and_retain_fresh_operation_lease(
                fresh,
                ResidentDeploymentOperationLeaseAccess {
                    store: &selected_store,
                    custody: &mut custody,
                },
                allocation_context(&semantic, request, 1 << 20),
                &mut authority,
                lease_basis(semantic.context.operation_lease),
            ) {
                Err(failure) => failure,
                Ok(_) => panic!("definitive store failure retains admitted custody"),
            };
            let ResidentProcessOperationLeaseFailure::Custody { pending, reason } = failure else {
                panic!("retention failure must preserve admitted custody")
            };
            assert!(matches!(
                reason,
                crate::resident::OperationCustodyDatabaseFailureReason::Store(_)
            ));
            assert_eq!(
                pending.fresh.allocation.as_ref() as *const _,
                allocation_address
            );
            assert_eq!(pending.lease.operation_lease_id(), &expected_lease_id);
            assert_eq!(pending.fresh.allocation.lease_ref(), &expected_lease_ref);
            assert!(authority.calls > 0);
            assert!(custody.retained_lease().is_none());
        },
    );
}

#[test]
fn allocated_checkpoint_identity_cannot_be_substituted_before_signing_or_commit() {
    let (generation, retained_generation, _, profile) = source_generation_triplet();
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        "process-host-allocated-checkpoint-fence",
        |_| {},
        |semantic, request, checkpoint_template, parents| {
            let selected_store = crate::local_operation_start_fixture::store_coordinate();
            let other_store = store("other-allocation-store", "other-allocation-incarnation");
            let selector = selector_from_signing_authority(parents.signing);
            let selected_port = MemoryAuthorityStore::default();
            let other_port = MemoryAuthorityStore::default();
            let mut host = host_for_allocation(
                &selected_port,
                &other_port,
                &selector,
                &selected_store,
                &other_store,
                retained_generation,
            );
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
            let random = OrderedRandom::succeeding(Arc::clone(&reads));
            let fresh = match host
                .allocate_operation_start(
                    &selector,
                    ResidentDeploymentOperationLookup {
                        store: &selected_store,
                        custody: &mut custody,
                    },
                    allocation_context(&semantic, request, 1 << 20),
                    &random,
                )
                .unwrap_or_else(|_| panic!("fresh allocation"))
            {
                ResidentProcessOperationStartAllocation::Fresh(fresh) => *fresh,
                ResidentProcessOperationStartAllocation::Existing(_) => {
                    panic!("absent request must allocate")
                }
            };
            assert_ne!(
                fresh.allocation.checkpoint_id(),
                &checkpoint_template.control_checkpoint_id
            );
            let mut authority = CountingAuthority::default();
            let retained = host
                .admit_and_retain_fresh_operation_lease(
                    fresh,
                    ResidentDeploymentOperationLeaseAccess {
                        store: &selected_store,
                        custody: &mut custody,
                    },
                    allocation_context(&semantic, request, 1 << 20),
                    &mut authority,
                    lease_basis(semantic.context.operation_lease),
                )
                .unwrap_or_else(|_| panic!("retained allocated lease"));
            let signer_calls = Cell::new(0);
            let mut anchor = initial_anchor();
            let failure = match host.start_allocated_invoke(
                retained,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut custody,
                    sessions: &selected_port,
                    anchor: &mut anchor,
                },
                &mut authority,
                start_basis(&semantic),
                |_, _, _, _, _, _, _, _| Ok(()),
                |_, _, _| {
                    signer_calls.set(signer_calls.get() + 1);
                    Ok(local_operation_start_fixture::signature())
                },
                |_, _, _| {
                    signer_calls.set(signer_calls.get() + 1);
                    Ok(local_operation_start_fixture::signature())
                },
                move |prepared| checkpoint_preimage(prepared, checkpoint_template),
                |_, _, _| {
                    signer_calls.set(signer_calls.get() + 1);
                    Ok(local_operation_start_fixture::signature())
                },
            ) {
                Err(failure) => failure,
                Ok(_) => panic!("foreign checkpoint identity must not start"),
            };
            let ResidentProcessAllocatedInvokeStartError::Continuation(continuation) = failure
            else {
                panic!("checkpoint substitution must retain the signed start continuation")
            };
            assert!(matches!(
                continuation.failure.as_ref(),
                ResidentLocalOperationStartFailure::PreparedCheckpoint {
                    failure,
                    ..
                } if matches!(
                    failure.as_ref(),
                    crate::resident::LocalOperationStartError::Coordinate
                )
            ));
            assert_eq!(signer_calls.get(), 2);
            assert_eq!(custody.mutation_counts(), (0, 0));
        },
    );
}

#[test]
fn retained_start_rechecks_request_identity_before_signing_or_start_custody() {
    let (generation, retained_generation, _, profile) = source_generation_triplet();
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        "process-host-retained-request-fence",
        |_| {},
        |semantic, request, _, parents| {
            let selected_store = crate::local_operation_start_fixture::store_coordinate();
            let other_store = store("other-allocation-store", "other-allocation-incarnation");
            let selector = selector_from_signing_authority(parents.signing);
            let selected_port = MemoryAuthorityStore::default();
            let other_port = MemoryAuthorityStore::default();
            let mut host = host_for_allocation(
                &selected_port,
                &other_port,
                &selector,
                &selected_store,
                &other_store,
                retained_generation,
            );
            let reads = Arc::new(AtomicUsize::new(0));
            let mut custody = absent_start_store(request)
                .with_index_read_counter(Arc::clone(&reads))
                .with_lease_retention(Ok(pale_ports::OperationCustodyCommitOutcome::Committed));
            let random = OrderedRandom::succeeding(Arc::clone(&reads));
            let fresh = match host
                .allocate_operation_start(
                    &selector,
                    ResidentDeploymentOperationLookup {
                        store: &selected_store,
                        custody: &mut custody,
                    },
                    allocation_context(&semantic, request, 1 << 20),
                    &random,
                )
                .unwrap_or_else(|_| panic!("fresh allocation"))
            {
                ResidentProcessOperationStartAllocation::Fresh(fresh) => *fresh,
                ResidentProcessOperationStartAllocation::Existing(_) => {
                    panic!("absent request must allocate")
                }
            };
            let mut authority = CountingAuthority::default();
            let retained = host
                .admit_and_retain_fresh_operation_lease(
                    fresh,
                    ResidentDeploymentOperationLeaseAccess {
                        store: &selected_store,
                        custody: &mut custody,
                    },
                    allocation_context(&semantic, request, 1 << 20),
                    &mut authority,
                    lease_basis(semantic.context.operation_lease),
                )
                .unwrap_or_else(|_| panic!("retained allocated lease"));
            let expected_lease_id = retained.lease.operation_lease_id().clone();
            let mut changed = request.to_preimage();
            changed.scope.purpose =
                pale_core::records::BoundedResidentTextV1::parse("changed after lease retention")
                    .expect("changed purpose");
            let changed = crate::effect_case_lineage_fixture::request(&changed);
            let mut changed_basis = start_basis(&semantic);
            changed_basis.request = &changed;
            let signer_calls = Cell::new(0);
            let mut anchor = initial_anchor();
            let failure = match host.start_allocated_invoke(
                retained,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut custody,
                    sessions: &selected_port,
                    anchor: &mut anchor,
                },
                &mut authority,
                changed_basis,
                |_, _, _, _, _, _, _, _| Ok(()),
                |_, _, _| {
                    signer_calls.set(signer_calls.get() + 1);
                    Ok(local_operation_start_fixture::signature())
                },
                |_, _, _| {
                    signer_calls.set(signer_calls.get() + 1);
                    Ok(local_operation_start_fixture::signature())
                },
                |_| panic!("request substitution must precede checkpoint preparation"),
                |_, _, _| {
                    signer_calls.set(signer_calls.get() + 1);
                    Ok(local_operation_start_fixture::signature())
                },
            ) {
                Err(failure) => failure,
                Ok(_) => panic!("changed request must not start"),
            };
            let ResidentProcessAllocatedInvokeStartError::Selection { pending, reason } = failure
            else {
                panic!("changed request is a retained allocation fence")
            };
            assert!(matches!(
                reason,
                ResidentProcessHostSelectionError::StartAllocationAssociation
            ));
            assert_eq!(pending.lease.operation_lease_id(), &expected_lease_id);
            assert_eq!(signer_calls.get(), 0);
            assert_eq!(custody.mutation_counts(), (0, 0));
        },
    );
}

#[test]
fn allocated_start_custody_continues_without_reallocation_readmission_or_resigning() {
    let (generation, retained_generation, _, profile) = source_generation_triplet();
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        "process-host-allocated-custody-continuation",
        |_| {},
        |semantic, request, mut checkpoint_template, parents| {
            let selected_store = crate::local_operation_start_fixture::store_coordinate();
            let other_store = store("other-allocation-store", "other-allocation-incarnation");
            let selector = selector_from_signing_authority(parents.signing);
            let selected_port = MemoryAuthorityStore::default();
            let other_port = MemoryAuthorityStore::default();
            let mut host = host_for_allocation(
                &selected_port,
                &other_port,
                &selector,
                &selected_store,
                &other_store,
                retained_generation,
            );
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
            let random = OrderedRandom::succeeding(Arc::clone(&reads));
            let fresh = match host
                .allocate_operation_start(
                    &selector,
                    ResidentDeploymentOperationLookup {
                        store: &selected_store,
                        custody: &mut custody,
                    },
                    allocation_context(&semantic, request, 1 << 20),
                    &random,
                )
                .unwrap_or_else(|_| panic!("fresh allocation"))
            {
                ResidentProcessOperationStartAllocation::Fresh(fresh) => *fresh,
                ResidentProcessOperationStartAllocation::Existing(_) => {
                    panic!("absent request must allocate")
                }
            };
            checkpoint_template.control_checkpoint_id = fresh.allocation.checkpoint_id().clone();
            let expected_handle_ref = fresh.allocation.handle_ref().clone();
            let mut authority = CountingAuthority::default();
            let retained = host
                .admit_and_retain_fresh_operation_lease(
                    fresh,
                    ResidentDeploymentOperationLeaseAccess {
                        store: &selected_store,
                        custody: &mut custody,
                    },
                    allocation_context(&semantic, request, 1 << 20),
                    &mut authority,
                    lease_basis(semantic.context.operation_lease),
                )
                .unwrap_or_else(|_| panic!("retained allocated lease"));
            custody
                .replace_commit_result(Err(ControlStoreError::new(ControlStoreErrorKind::Storage)));
            let signer_calls = Cell::new(0);
            let owner_calls = Cell::new(0);
            let mut anchor = initial_anchor();
            let pending = match host.start_allocated_invoke(
                retained,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut custody,
                    sessions: &selected_port,
                    anchor: &mut anchor,
                },
                &mut authority,
                start_basis(&semantic),
                |_, _, _, _, _, _, _, _| {
                    owner_calls.set(owner_calls.get() + 1);
                    Ok(())
                },
                |_, _, _| {
                    signer_calls.set(signer_calls.get() + 1);
                    Ok(local_operation_start_fixture::signature())
                },
                |_, _, _| {
                    signer_calls.set(signer_calls.get() + 1);
                    Ok(local_operation_start_fixture::signature())
                },
                move |prepared| checkpoint_preimage(prepared, checkpoint_template),
                |_, _, _| {
                    signer_calls.set(signer_calls.get() + 1);
                    Ok(local_operation_start_fixture::signature())
                },
            ) {
                Err(ResidentProcessAllocatedInvokeStartError::Continuation(pending)) => *pending,
                _ => panic!("definitive database failure retains exact custody"),
            };
            assert_eq!(random.calls(), 7);
            assert_eq!(signer_calls.get(), 3);
            assert_eq!(owner_calls.get(), 1);
            let authority_calls = authority.calls;
            let signed_candidate = custody
                .committed_candidate()
                .cloned()
                .expect("failed commit retains signed candidate");
            custody.replace_commit_result(Ok(pale_ports::OperationCustodyCommitOutcome::Committed));
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
                .unwrap_or_else(|_| panic!("exact custody continuation"));
            assert_eq!(issued.handle().0, &expected_handle_ref);
            assert_eq!(custody.committed_candidate(), Some(&signed_candidate));
            assert_eq!(random.calls(), 7);
            assert_eq!(signer_calls.get(), 3);
            assert_eq!(owner_calls.get(), 1);
            assert_eq!(authority.calls, authority_calls);
        },
    );
}

#[test]
fn existing_start_returns_only_selector_bound_physical_evidence_without_entropy() {
    let (generation, retained_generation, _, profile) = source_generation_triplet();
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        "process-host-allocation-existing",
        |_| {},
        |semantic, request, _, parents| {
            let selected_store = crate::local_operation_start_fixture::store_coordinate();
            let other_store = store("other-allocation-store", "other-allocation-incarnation");
            let selector = selector_from_signing_authority(parents.signing);
            let selected_port = MemoryAuthorityStore::default();
            let other_port = MemoryAuthorityStore::default();
            let host = host_for_allocation(
                &selected_port,
                &other_port,
                &selector,
                &selected_store,
                &other_store,
                retained_generation,
            );
            let reads = Arc::new(AtomicUsize::new(0));
            let mut custody = RecordingStore::recovering(existing_reads(request))
                .with_index_read_counter(Arc::clone(&reads));
            let other_custody = RecordingStore::recovering(Vec::new()).at_store(&other_store);
            let random = OrderedRandom::succeeding(Arc::clone(&reads));

            let outcome = host
                .allocate_operation_start(
                    &selector,
                    ResidentDeploymentOperationLookup {
                        store: &selected_store,
                        custody: &mut custody,
                    },
                    allocation_context(&semantic, request, 1 << 20),
                    &random,
                )
                .unwrap_or_else(|_| panic!("existing request lookup"));
            let ResidentProcessOperationStartAllocation::Existing(existing) = outcome else {
                panic!("existing request must not allocate")
            };
            assert_eq!(existing.selector, selector);
            assert_eq!(existing.evidence.store(), &selected_store);
            assert_eq!(
                existing
                    .evidence
                    .request()
                    .coordinate()
                    .record_sha256()
                    .as_str(),
                request.canonical_request_sha256().as_str()
            );
            assert_eq!(random.calls(), 0);
            assert_eq!(reads.load(Ordering::SeqCst), 4);
            assert_eq!(custody.index_query_count(), 4);
            assert_no_non_lookup_effects(
                &host,
                &custody,
                &other_custody,
                &selected_port,
                &other_port,
            );
        },
    );
}

#[test]
fn lookup_and_random_failures_never_write_or_draw_a_replacement() {
    let (generation, retained_generation, _, profile) = source_generation_triplet();
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        "process-host-allocation-failures",
        |_| {},
        |semantic, request, _, parents| {
            let selected_store = crate::local_operation_start_fixture::store_coordinate();
            let other_store = store("other-allocation-store", "other-allocation-incarnation");
            let selector = selector_from_signing_authority(parents.signing);
            let selected_port = MemoryAuthorityStore::default();
            let other_port = MemoryAuthorityStore::default();
            let host = host_for_allocation(
                &selected_port,
                &other_port,
                &selector,
                &selected_store,
                &other_store,
                retained_generation,
            );
            let other_custody = RecordingStore::recovering(Vec::new()).at_store(&other_store);
            let mut lookup_failure = RecordingStore::recovering(Vec::new())
                .with_index_error(ControlStoreError::new(ControlStoreErrorKind::Storage));
            let failed_reads = Arc::new(AtomicUsize::new(0));
            lookup_failure = lookup_failure.with_index_read_counter(Arc::clone(&failed_reads));
            let random = OrderedRandom::succeeding(Arc::clone(&failed_reads));
            assert!(matches!(
                host.allocate_operation_start(
                    &selector,
                    ResidentDeploymentOperationLookup {
                        store: &selected_store,
                        custody: &mut lookup_failure,
                    },
                    allocation_context(&semantic, request, 1 << 20),
                    &random,
                ),
                Err(ResidentProcessOperationAllocationError::Allocation(
                    OperationStartAllocationError::Recovery(_)
                ))
            ));
            assert_eq!(random.calls(), 0);
            assert_eq!(lookup_failure.mutation_counts(), (0, 0));

            let key = crate::resident::operation_custody::operation_request_key(request)
                .expect("request key");
            let misdirected = OperationCustodyIndexObservation::absent(
                pale_ports::OperationCustodyIndexQuery::RequestEvidence(key),
            );
            let misdirected_reads = Arc::new(AtomicUsize::new(0));
            let mut misdirected_store = RecordingStore::recovering(vec![misdirected])
                .with_index_read_counter(Arc::clone(&misdirected_reads));
            let random = OrderedRandom::succeeding(Arc::clone(&misdirected_reads));
            assert!(matches!(
                host.allocate_operation_start(
                    &selector,
                    ResidentDeploymentOperationLookup {
                        store: &selected_store,
                        custody: &mut misdirected_store,
                    },
                    allocation_context(&semantic, request, 1 << 20),
                    &random,
                ),
                Err(ResidentProcessOperationAllocationError::Allocation(
                    OperationStartAllocationError::Recovery(_)
                ))
            ));
            assert_eq!(random.calls(), 0);
            assert_eq!(misdirected_store.mutation_counts(), (0, 0));

            let incomplete_reads = Arc::new(AtomicUsize::new(0));
            let mut incomplete = existing_reads(request);
            let missing_request = incomplete[1].query().clone();
            incomplete[1] = OperationCustodyIndexObservation::absent(missing_request);
            let mut incomplete_store = RecordingStore::recovering(incomplete)
                .with_index_read_counter(Arc::clone(&incomplete_reads));
            let random = OrderedRandom::succeeding(Arc::clone(&incomplete_reads));
            assert!(matches!(
                host.allocate_operation_start(
                    &selector,
                    ResidentDeploymentOperationLookup {
                        store: &selected_store,
                        custody: &mut incomplete_store,
                    },
                    allocation_context(&semantic, request, 1 << 20),
                    &random,
                ),
                Err(ResidentProcessOperationAllocationError::Allocation(
                    OperationStartAllocationError::Recovery(_)
                ))
            ));
            assert_eq!(random.calls(), 0);
            assert_eq!(incomplete_store.mutation_counts(), (0, 0));

            let bounded_reads = Arc::new(AtomicUsize::new(0));
            let mut oversized_store = RecordingStore::recovering(existing_reads(request))
                .with_index_read_counter(Arc::clone(&bounded_reads));
            let random = OrderedRandom::succeeding(Arc::clone(&bounded_reads));
            assert!(matches!(
                host.allocate_operation_start(
                    &selector,
                    ResidentDeploymentOperationLookup {
                        store: &selected_store,
                        custody: &mut oversized_store,
                    },
                    allocation_context(&semantic, request, 1),
                    &random,
                ),
                Err(ResidentProcessOperationAllocationError::Allocation(
                    OperationStartAllocationError::Recovery(_)
                ))
            ));
            assert_eq!(random.calls(), 0);
            assert_eq!(oversized_store.mutation_counts(), (0, 0));

            let random_reads = Arc::new(AtomicUsize::new(0));
            let mut random_failure =
                absent_start_store(request).with_index_read_counter(Arc::clone(&random_reads));
            let random = OrderedRandom::failing_at(Arc::clone(&random_reads), 1);
            assert!(matches!(
                host.allocate_operation_start(
                    &selector,
                    ResidentDeploymentOperationLookup {
                        store: &selected_store,
                        custody: &mut random_failure,
                    },
                    allocation_context(&semantic, request, 1 << 20),
                    &random,
                ),
                Err(ResidentProcessOperationAllocationError::Allocation(
                    OperationStartAllocationError::Random(_)
                ))
            ));
            assert_eq!(random.calls(), 1);
            assert_eq!(random_failure.mutation_counts(), (0, 0));
            assert_eq!(other_custody.index_query_count(), 0);
            assert_eq!(selected_port.entry_count(), 0);
            assert_eq!(other_port.entry_count(), 0);
            assert_eq!(host.provider_routes.reserved, 0);
            assert_eq!(host.provider_routes.checked_out, 0);
            assert!(host.provider_routes.active.is_empty());
        },
    );
}

#[test]
fn every_entropy_failure_position_returns_no_fresh_phase_or_mutation() {
    let (generation, retained_generation, _, profile) = source_generation_triplet();
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        "process-host-allocation-each-entropy-failure",
        |_| {},
        |semantic, request, _, parents| {
            let selected_store = crate::local_operation_start_fixture::store_coordinate();
            let other_store = store("other-allocation-store", "other-allocation-incarnation");
            let selector = selector_from_signing_authority(parents.signing);
            let selected_port = MemoryAuthorityStore::default();
            let other_port = MemoryAuthorityStore::default();
            let host = host_for_allocation(
                &selected_port,
                &other_port,
                &selector,
                &selected_store,
                &other_store,
                retained_generation,
            );

            for failing_draw in 1..=7 {
                let reads = Arc::new(AtomicUsize::new(0));
                let mut custody =
                    absent_start_store(request).with_index_read_counter(Arc::clone(&reads));
                let random = OrderedRandom::failing_at(Arc::clone(&reads), failing_draw);
                assert!(matches!(
                    host.allocate_operation_start(
                        &selector,
                        ResidentDeploymentOperationLookup {
                            store: &selected_store,
                            custody: &mut custody,
                        },
                        allocation_context(&semantic, request, 1 << 20),
                        &random,
                    ),
                    Err(ResidentProcessOperationAllocationError::Allocation(
                        OperationStartAllocationError::Random(_)
                    ))
                ));
                assert_eq!(reads.load(Ordering::SeqCst), 2);
                assert_eq!(random.calls(), failing_draw);
                assert_eq!(custody.mutation_counts(), (0, 0));
            }
            assert_eq!(selected_port.entry_count(), 0);
            assert_eq!(other_port.entry_count(), 0);
            assert_eq!(host.provider_routes.reserved, 0);
            assert_eq!(host.provider_routes.checked_out, 0);
            assert!(host.provider_routes.active.is_empty());
        },
    );
}

#[test]
fn foreign_access_fails_before_request_lookup_or_entropy() {
    let (generation, retained_generation, _, profile) = source_generation_triplet();
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        "process-host-allocation-foreign-access",
        |_| {},
        |semantic, request, _, parents| {
            let selected_store = crate::local_operation_start_fixture::store_coordinate();
            let other_store = store("other-allocation-store", "other-allocation-incarnation");
            let selector = selector_from_signing_authority(parents.signing);
            let selected_port = MemoryAuthorityStore::default();
            let other_port = MemoryAuthorityStore::default();
            let host = host_for_allocation(
                &selected_port,
                &other_port,
                &selector,
                &selected_store,
                &other_store,
                retained_generation,
            );
            let reads = Arc::new(AtomicUsize::new(0));
            let mut custody =
                absent_start_store(request).with_index_read_counter(Arc::clone(&reads));
            let random = OrderedRandom::succeeding(Arc::clone(&reads));

            assert!(matches!(
                host.allocate_operation_start(
                    &selector,
                    ResidentDeploymentOperationLookup {
                        store: &other_store,
                        custody: &mut custody,
                    },
                    allocation_context(&semantic, request, 1 << 20),
                    &random,
                ),
                Err(ResidentProcessOperationAllocationError::Selection(
                    ResidentProcessHostSelectionError::AccessStoreAssociation
                ))
            ));
            assert_eq!(reads.load(Ordering::SeqCst), 0);
            assert_eq!(random.calls(), 0);
            assert_eq!(custody.mutation_counts(), (0, 0));
            assert_eq!(selected_port.entry_count(), 0);
            assert_eq!(other_port.entry_count(), 0);
        },
    );
}

#[test]
fn request_authority_substitution_fails_before_lookup_or_entropy() {
    let (generation, retained_generation, _, profile) = source_generation_triplet();
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        "process-host-allocation-authority-substitution",
        |_| {},
        |semantic, request, _, parents| {
            let selected_store = crate::local_operation_start_fixture::store_coordinate();
            let other_store = store("other-allocation-store", "other-allocation-incarnation");
            let selector = selector_from_signing_authority(parents.signing);
            let selected_port = MemoryAuthorityStore::default();
            let other_port = MemoryAuthorityStore::default();
            let host = host_for_allocation(
                &selected_port,
                &other_port,
                &selector,
                &selected_store,
                &other_store,
                retained_generation,
            );
            let request_reads = Arc::new(AtomicUsize::new(0));
            let mut request_store =
                absent_start_store(request).with_index_read_counter(Arc::clone(&request_reads));
            let random = OrderedRandom::succeeding(Arc::clone(&request_reads));
            let other_selector = unproved_selector(selector.principal_id.clone(), other_store);
            let unlike_request = request_with_signing_authority(
                request,
                integrated_signing_authority(&other_selector),
            );
            let request_context = ResidentProcessOperationStartContext {
                session: semantic.session,
                current: semantic.current,
                request: &unlike_request,
                handle_signing_authority: semantic.context.signed.signing_authority,
                semantic_checkpoint_authority: semantic.checkpoint_authority,
                issuance_checkpoint_authority: semantic.checkpoint_authority,
                maximum_record_bytes: 1 << 20,
            };
            assert!(matches!(
                host.allocate_operation_start(
                    &selector,
                    ResidentDeploymentOperationLookup {
                        store: &selected_store,
                        custody: &mut request_store,
                    },
                    request_context,
                    &random,
                ),
                Err(ResidentProcessOperationAllocationError::Selection(
                    ResidentProcessHostSelectionError::CurrentRequestAuthority
                ))
            ));
            assert_eq!(request_reads.load(Ordering::SeqCst), 0);
            assert_eq!(random.calls(), 0);
            assert_eq!(request_store.mutation_counts(), (0, 0));
            assert_eq!(selected_port.entry_count(), 0);
            assert_eq!(other_port.entry_count(), 0);
            assert_eq!(host.provider_routes.reserved, 0);
            assert_eq!(host.provider_routes.checked_out, 0);
            assert!(host.provider_routes.active.is_empty());
        },
    );
}

#[test]
fn every_other_derivable_authority_join_precedes_lookup_and_entropy() {
    let (generation, retained_generation, _, profile) = source_generation_triplet();
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        "process-host-allocation-complete-preflight",
        |_| {},
        |semantic, request, checkpoint_template, parents| {
            let selected_store = crate::local_operation_start_fixture::store_coordinate();
            let other_store = store("other-allocation-store", "other-allocation-incarnation");
            let selector = selector_from_signing_authority(parents.signing);
            let selected_port = MemoryAuthorityStore::default();
            let other_port = MemoryAuthorityStore::default();
            let host = host_for_allocation(
                &selected_port,
                &other_port,
                &selector,
                &selected_store,
                &other_store,
                retained_generation,
            );

            macro_rules! reject_before_io {
                ($context:expr, $reason:pat) => {{
                    let reads = Arc::new(AtomicUsize::new(0));
                    let mut custody =
                        absent_start_store(request).with_index_read_counter(Arc::clone(&reads));
                    let random = OrderedRandom::succeeding(Arc::clone(&reads));
                    assert!(matches!(
                        host.allocate_operation_start(
                            &selector,
                            ResidentDeploymentOperationLookup {
                                store: &selected_store,
                                custody: &mut custody,
                            },
                            $context,
                            &random,
                        ),
                        Err(ResidentProcessOperationAllocationError::Selection($reason))
                    ));
                    assert_eq!(reads.load(Ordering::SeqCst), 0);
                    assert_eq!(random.calls(), 0);
                    assert_eq!(custody.mutation_counts(), (0, 0));
                }};
            }

            let foreign_current_signing =
                owner_native_signing_authority(selector.principal_id.clone());
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
                        signing_authority: &foreign_current_signing,
                    },
                },
            };
            reject_before_io!(
                ResidentProcessOperationStartContext {
                    current: &foreign_current,
                    ..allocation_context(&semantic, request, 1 << 20)
                },
                ResidentProcessHostSelectionError::StartCurrentAuthority
            );

            let unlike_session_id = request_with_change(request, |preimage| {
                preimage.session_admission.session_admission_id =
                    SessionAdmissionIdV1::parse("substituted-session")
                        .expect("substituted session");
            });
            reject_before_io!(
                ResidentProcessOperationStartContext {
                    request: &unlike_session_id,
                    ..allocation_context(&semantic, request, 1 << 20)
                },
                ResidentProcessHostSelectionError::StartRequestAssociation
            );

            let unlike_session_lease = request_with_change(request, |preimage| {
                preimage.session_admission.session_admission_lease_digest =
                    SessionAdmissionLeaseDigest::parse("f".repeat(64))
                        .expect("substituted session lease");
            });
            reject_before_io!(
                ResidentProcessOperationStartContext {
                    request: &unlike_session_lease,
                    ..allocation_context(&semantic, request, 1 << 20)
                },
                ResidentProcessHostSelectionError::StartRequestAssociation
            );

            let other_package = request_with_change(request, |preimage| {
                preimage.operation.package_id =
                    PackageIdV1::parse("other-package").expect("other Package");
            });
            reject_before_io!(
                ResidentProcessOperationStartContext {
                    request: &other_package,
                    ..allocation_context(&semantic, request, 1 << 20)
                },
                ResidentProcessHostSelectionError::StartRequestAssociation
            );

            let absent_operation = request_with_change(request, |preimage| {
                preimage.operation.resource_id =
                    PackageResourceIdV1::parse("absent-operation").expect("absent operation");
            });
            reject_before_io!(
                ResidentProcessOperationStartContext {
                    request: &absent_operation,
                    ..allocation_context(&semantic, request, 1 << 20)
                },
                ResidentProcessHostSelectionError::StartRequestAssociation
            );

            let mut foreign_selector = selector.clone();
            foreign_selector.runtime_authority_ref =
                ResidentRecordRefV1::parse("authority/foreign-allocation-handle")
                    .expect("foreign handle authority");
            let foreign_handle_signing = integrated_signing_authority(&foreign_selector);
            reject_before_io!(
                ResidentProcessOperationStartContext {
                    handle_signing_authority: &foreign_handle_signing,
                    ..allocation_context(&semantic, request, 1 << 20)
                },
                ResidentProcessHostSelectionError::StartHandleAuthority
            );

            let mut foreign_checkpoint_preimage = checkpoint_template.clone();
            foreign_checkpoint_preimage.control_store_id = other_store.store_id().clone();
            foreign_checkpoint_preimage.control_incarnation_id =
                other_store.incarnation_id().clone();
            let foreign_checkpoint = crate::local_operation_start_fixture::checkpoint_authority(
                &foreign_checkpoint_preimage,
            );
            reject_before_io!(
                ResidentProcessOperationStartContext {
                    semantic_checkpoint_authority: &foreign_checkpoint,
                    ..allocation_context(&semantic, request, 1 << 20)
                },
                ResidentProcessHostSelectionError::CheckpointAuthorityAssociation
            );
            reject_before_io!(
                ResidentProcessOperationStartContext {
                    issuance_checkpoint_authority: &foreign_checkpoint,
                    ..allocation_context(&semantic, request, 1 << 20)
                },
                ResidentProcessHostSelectionError::CheckpointAuthorityAssociation
            );

            let mut unlike_checkpoint_preimage = checkpoint_template.clone();
            unlike_checkpoint_preimage.checkpoint_signing_authority_ref =
                ResidentRecordRefV1::parse("checkpoint-authority/allocation-unlike")
                    .expect("unlike checkpoint authority");
            let unlike_checkpoint = crate::local_operation_start_fixture::checkpoint_authority(
                &unlike_checkpoint_preimage,
            );
            reject_before_io!(
                ResidentProcessOperationStartContext {
                    issuance_checkpoint_authority: &unlike_checkpoint,
                    ..allocation_context(&semantic, request, 1 << 20)
                },
                ResidentProcessHostSelectionError::StartCheckpointAuthority
            );

            crate::local_operation_start_fixture::with_invoke_start_input_at_store(
                &generation,
                &profile,
                "process-host-allocation-foreign-admitted-session",
                |_| {},
                &other_store,
                |foreign, _, _, _| {
                    reject_before_io!(
                        ResidentProcessOperationStartContext {
                            session: foreign.session,
                            ..allocation_context(&semantic, request, 1 << 20)
                        },
                        ResidentProcessHostSelectionError::StartSessionAuthority
                    );
                },
            );

            assert_eq!(selected_port.entry_count(), 0);
            assert_eq!(other_port.entry_count(), 0);
            assert_eq!(host.provider_routes.reserved, 0);
            assert_eq!(host.provider_routes.checked_out, 0);
            assert!(host.provider_routes.active.is_empty());
        },
    );
}

#[test]
fn consult_reuses_the_request_first_allocation_and_lease_custody_phase() {
    let (generation, retained_generation, _, profile) = source_generation_triplet();
    let selected_store = crate::local_operation_start_fixture::store_coordinate();
    crate::local_operation_start_fixture::with_operation_start_input_at_store(
        &generation,
        &profile,
        "process-host-consult-allocation",
        |_| {},
        pale_core::records::OperationClassV1::Consult,
        &selected_store,
        |semantic, request, _, parents| {
            let other_store = store("other-allocation-store", "other-allocation-incarnation");
            let selector = selector_from_signing_authority(parents.signing);
            let selected_port = MemoryAuthorityStore::default();
            let other_port = MemoryAuthorityStore::default();
            let host = host_for_allocation(
                &selected_port,
                &other_port,
                &selector,
                &selected_store,
                &other_store,
                retained_generation,
            );
            let reads = Arc::new(AtomicUsize::new(0));
            let mut custody = absent_start_store(request)
                .with_index_read_counter(Arc::clone(&reads))
                .with_lease_retention(Ok(pale_ports::OperationCustodyCommitOutcome::Committed));
            let random = OrderedRandom::succeeding(reads);
            let fresh = match host
                .allocate_operation_start(
                    &selector,
                    ResidentDeploymentOperationLookup {
                        store: &selected_store,
                        custody: &mut custody,
                    },
                    allocation_context(&semantic, request, 1 << 20),
                    &random,
                )
                .expect("Consult request allocation")
            {
                ResidentProcessOperationStartAllocation::Fresh(fresh) => *fresh,
                ResidentProcessOperationStartAllocation::Existing(_) => {
                    panic!("absent Consult request must allocate")
                }
            };
            let mut authority = CountingAuthority::default();
            let retained = match host.admit_and_retain_fresh_operation_lease(
                fresh,
                ResidentDeploymentOperationLeaseAccess {
                    store: &selected_store,
                    custody: &mut custody,
                },
                allocation_context(&semantic, request, 1 << 20),
                &mut authority,
                lease_basis(semantic.context.operation_lease),
            ) {
                Ok(retained) => retained,
                Err(_) => panic!("Consult lease custody"),
            };
            assert_eq!(
                retained.lease.operation_lease_id(),
                retained.fresh.allocation.lease_id()
            );
            assert!(custody.retained_lease().is_some());
        },
    );
}

#[test]
fn delegate_lease_narrowing_precedes_lease_custody() {
    let (generation, retained_generation, _, profile) = source_generation_triplet();
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        "process-host-delegate-narrowing-before-custody",
        |_| {},
        |semantic, request, _, parents| {
            let selected_store = crate::local_operation_start_fixture::store_coordinate();
            let other_store = store("other-allocation-store", "other-allocation-incarnation");
            let selector = selector_from_signing_authority(parents.signing);
            let selected_port = MemoryAuthorityStore::default();
            let other_port = MemoryAuthorityStore::default();
            let host = host_for_allocation(
                &selected_port,
                &other_port,
                &selector,
                &selected_store,
                &other_store,
                retained_generation,
            );
            let reads = Arc::new(AtomicUsize::new(0));
            let mut custody =
                absent_start_store(request).with_index_read_counter(Arc::clone(&reads));
            let random = OrderedRandom::succeeding(reads);
            let fresh = match host
                .allocate_operation_start(
                    &selector,
                    ResidentDeploymentOperationLookup {
                        store: &selected_store,
                        custody: &mut custody,
                    },
                    allocation_context(&semantic, request, 1 << 20),
                    &random,
                )
                .expect("Delegate child allocation")
            {
                ResidentProcessOperationStartAllocation::Fresh(fresh) => *fresh,
                ResidentProcessOperationStartAllocation::Existing(_) => {
                    panic!("absent Delegate request must allocate")
                }
            };
            let schema = pale_contracts::record_schema_identity(
                pale_core::records::OPERATION_LEASE_V1_SCHEMA_ID,
            )
            .expect("operation lease schema");
            let child = semantic.context.operation_lease;
            let parent = OperationLeaseV1::seal(
                schema.schema_sha256,
                semantic.session.lease(),
                pale_core::records::OperationLeaseRequestV1 {
                    operation_lease_id: pale_core::records::OperationLeaseIdV1::parse(
                        "operation-lease:v1:delegate-narrow-parent",
                    )
                    .expect("parent lease id"),
                    operation: child.operation().clone(),
                    capabilities: child.capabilities().to_vec(),
                    effects: child.effects().to_vec(),
                    read_set: child.read_set().to_vec(),
                    evidence_obligations: child.evidence_obligations().to_vec(),
                    budget_units: pale_core::records::LeaseBudgetUnitsV1::new(
                        child.budget_units().get() - 1,
                    )
                    .expect("narrow parent budget"),
                    timeout_millis: *child.timeout_millis(),
                    expires_at: child.expires_at(),
                    invocation_correlation: child.invocation_correlation().clone(),
                },
            )
            .expect("narrower parent lease");
            let mut authority = CountingAuthority::default();
            let failure = match host.admit_and_retain_fresh_operation_lease_with_lineage(
                fresh,
                ResidentDeploymentOperationLeaseAccess {
                    store: &selected_store,
                    custody: &mut custody,
                },
                allocation_context(&semantic, request, 1 << 20),
                &mut authority,
                lease_basis(child),
                ResidentProcessOperationLeaseLineage::DelegateChild(&parent),
            ) {
                Err(failure) => failure,
                Ok(_) => panic!("widened Delegate child must fail"),
            };
            assert!(matches!(
                failure,
                ResidentProcessOperationLeaseFailure::Narrowing { .. }
            ));
            assert_eq!(custody.mutation_counts(), (0, 0));
            assert!(custody.retained_lease().is_none());
        },
    );
}
