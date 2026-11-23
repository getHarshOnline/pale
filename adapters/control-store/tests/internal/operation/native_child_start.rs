//! Child admission races with parent cancellation over actual database custody.
//! Signatures and independent anchoring remain the shared fixture's test models.

use super::*;
use crate::location::ControlStoreLocation;
use crate::resident_fixture::local_operation_start_fixture::{profile, RecoveryParents};
use crate::resident_fixture::shared_resident::RecoveryResolver;
use std::cell::{Cell, RefCell};

#[path = "native_child_start/fixture.rs"]
mod fixture;
use fixture::with_child_start;
#[path = "native_child_start/service.rs"]
mod child_service;
#[path = "native_child_start/ingress.rs"]
mod ingress;
#[path = "native_child_start/interleaving.rs"]
mod interleaving;

pub(in crate::operation::tests) struct Fixture<'a> {
    pub(in crate::operation::tests) operation: &'a IssuedLocalOperation,
    pub(in crate::operation::tests) request: &'a CommonRequestEnvelopeV1,
    pub(in crate::operation::tests) parents: &'a RecoveryParents<'a>,
    pub(in crate::operation::tests) generation: &'a AdmittedCompiledGeneration,
    pub(in crate::operation::tests) checkpoint: &'a ControlStateCheckpointPreimageV1,
    pub(in crate::operation::tests) file: &'a std::path::Path,
    pub(in crate::operation::tests) registry: &'a NativeActionRegistry,
}

pub(super) fn with_delegate_start<R>(
    fixture: &Fixture<'_>,
    prepare_parent: impl FnOnce(&ResidentRecordRefV1, &OperationLeaseV1) -> PreparedLocalChildParent,
    suffix: &str,
    run: impl FnOnce(
        Result<PreparedLocalOperationStart, LocalOperationStartError>,
        &CommonRequestEnvelopeV1,
        &OperationLeaseV1,
        usize,
        usize,
    ) -> R,
) -> R {
    fixture::with_child_start(
        fixture,
        fixture::Admission::Delegate,
        prepare_parent,
        suffix,
        |_| {},
        run,
    )
}

pub(super) fn with_admitted_child_session<R>(
    parents: &RecoveryParents<'_>,
    run: impl FnOnce(&AdmittedAmbientSession, &AmbientSessionAdmissionContext<'_>) -> R,
) -> R {
    fixture::with_child_session(parents, run)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Order {
    ChildFirst,
    ChildService,
    CancellationFirst,
    TransactionChangesDuringParentRead,
}

fn verify_checkpoint_signature(
    domain: &[u8],
    _: &ControlStateCheckpointPreimageV1,
    _: &ControlStateCheckpointDigest,
    actual: &CanonicalSignatureV1,
) -> Result<(), InvalidResidentRecordV1> {
    assert_eq!(domain, CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN);
    assert_eq!(actual, &signature());
    Ok(())
}

fn admits_parent_recovery_state(state: &OperationObservationStateV1) -> bool {
    matches!(
        state,
        OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::Resolving
                | OperationActivePhaseV1::DispatchPrepared { .. }
        } | OperationObservationStateV1::CancellationRequested { .. }
            | OperationObservationStateV1::Cancelled { .. }
    )
}

fn admits_child_recovery_state(state: &OperationObservationStateV1) -> bool {
    matches!(
        state,
        OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::Resolving
                | OperationActivePhaseV1::ContextBound { .. }
                | OperationActivePhaseV1::DispatchPrepared { .. }
                | OperationActivePhaseV1::Dispatched { .. }
        } | OperationObservationStateV1::Succeeded { .. }
    )
}

fn admits_parent_or_child_recovery_state(state: &OperationObservationStateV1) -> bool {
    admits_parent_recovery_state(state) || admits_child_recovery_state(state)
}

macro_rules! observation_admission {
    ($parents:expr, $checkpoint:expr, $authority:expr, $selected:expr, $admit_state:path) => {
        OperationObservationRecoveryAdmission {
            admit_checkpoint: |_: &ResidentRecordRefV1, wire: ControlStateCheckpointWireV1| {
                ControlStateCheckpointV1::admit_signed_wire(
                    wire,
                    &$checkpoint.schema_sha256,
                    $authority,
                    verify_checkpoint_signature,
                )
            },
            resolve_signer: |reference: &ResidentRecordRefV1, wire: &OperationObservationWireV1| {
                if let Some(record) = $selected
                    .iter()
                    .find(|record| record.coordinate().record_ref() == reference)
                {
                    let expected: OperationObservationWireV1 =
                        serde_json::from_slice(record.canonical_record_jcs().as_slice()).unwrap();
                    assert_eq!(
                        wire, &expected,
                        "selected observation must retain exact bytes"
                    );
                }
                assert_eq!(
                    wire.deployment_runtime_authority_ref(),
                    $parents.session.runtime_authority_ref()
                );
                assert_eq!(
                    wire.deployment_runtime_authority_sha256().as_str(),
                    $parents.session.runtime_authority_sha256().as_str()
                );
                Ok($parents.signing.clone())
            },
            admit_state: |state: &OperationObservationStateV1| {
                assert!($admit_state(state), "unexpected recovered state: {state:?}");
                Ok(())
            },
            admit_rotation: |_: &ResidentRecordRefV1,
                             _: &ArtifactDigest,
                             _: &ResidentRecordRefV1,
                             _: &ArtifactDigest| {
                panic!("native child fixture has no authority rotation")
            },
            verify_signature: |domain: &[u8],
                               _: &OperationObservationPreimageV1,
                               _: &OperationObservationDigest,
                               actual: &CanonicalSignatureV1| {
                assert_eq!(domain, OPERATION_OBSERVATION_V1_SIGNATURE_DOMAIN);
                assert_eq!(actual, &signature());
                Ok(())
            },
        }
    };
}

pub(in crate::operation::tests) fn prepare_parent(
    fixture: &Fixture<'_>,
    expected: &[&OperationObservationStateV1],
    port: &mut SqliteOperationCustody<'_>,
    anchor: &mut IndependentAnchor,
    mut on_signer_read: impl FnMut(),
) -> Result<PreparedLocalChildParent, LocalOperationStartError> {
    let coordinate = port.coordinate.clone();
    let authority = checkpoint_authority(fixture.checkpoint);
    prepare_local_child_parent(
        OperationObservationRecoveryInput {
            store: &coordinate,
            operation: fixture.operation,
            observation: OperationObservationSelection::SelectedHead,
            checkpoint_authority: &authority,
            limits: OperationObservationRecoveryLimits::new(4, 1 << 20, 1 << 20).unwrap(),
            checkpoint_limits: OperationCheckpointAncestryLimits::new(8, 1 << 20, 8).unwrap(),
            membership_limits: AccumulatorMembershipLimits::new(1, 10_000, 100_000).unwrap(),
        },
        fixture.parents.lease,
        port,
        anchor,
        OperationObservationRecoveryAdmission {
            admit_checkpoint: |_: &ResidentRecordRefV1, wire: ControlStateCheckpointWireV1| {
                ControlStateCheckpointV1::admit_signed_wire(
                    wire,
                    &schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID),
                    &authority,
                    verify_checkpoint_signature,
                )
            },
            resolve_signer: |_: &ResidentRecordRefV1, wire: &OperationObservationWireV1| {
                on_signer_read();
                assert_eq!(
                    wire.deployment_runtime_authority_ref(),
                    fixture.parents.session.runtime_authority_ref()
                );
                assert_eq!(
                    wire.deployment_runtime_authority_sha256().as_str(),
                    fixture.parents.session.runtime_authority_sha256().as_str()
                );
                assert_eq!(
                    wire.issuer_principal_id(),
                    fixture.parents.signing.principal_id()
                );
                assert_eq!(
                    wire.issuer_key_id().as_str(),
                    fixture.parents.signing.signer_key_id()
                );
                assert_eq!(
                    wire.issuer_key_epoch(),
                    fixture.parents.signing.signer_key_epoch()
                );
                Ok(fixture.parents.signing.clone())
            },
            admit_state: |state: &OperationObservationStateV1| {
                assert!(
                    expected.contains(&state)
                        || matches!(
                            state,
                            OperationObservationStateV1::Active {
                                phase: OperationActivePhaseV1::Resolving
                            }
                        )
                );
                Ok(())
            },
            admit_rotation: |_: &ResidentRecordRefV1,
                             _: &ArtifactDigest,
                             _: &ResidentRecordRefV1,
                             _: &ArtifactDigest| {
                panic!("this fixture does not rotate runtime authority")
            },
            verify_signature: |domain: &[u8],
                               _: &OperationObservationPreimageV1,
                               _: &OperationObservationDigest,
                               actual: &CanonicalSignatureV1| {
                assert_eq!(domain, OPERATION_OBSERVATION_V1_SIGNATURE_DOMAIN);
                assert_eq!(actual, &signature());
                Ok(())
            },
        },
        verify_checkpoint_signature,
    )
}

fn assert_committed_rows(
    port: &mut SqliteOperationCustody<'_>,
    candidate: &OperationCustodyCandidate,
) {
    for record in candidate
        .records()
        .iter()
        .map(OperationCustodyRecord::record)
        .chain(std::iter::once(candidate.checkpoint().successor_record()))
    {
        assert_eq!(
            port.read_operation_record(candidate.store(), record.coordinate(), 1 << 20)
                .unwrap()
                .as_ref(),
            Some(record)
        );
        let sequence: i64 = port.connection.query_row(
            "SELECT inserted_transaction_sequence FROM pale_deployment_control_record WHERE record_ref=?1",
            [record.coordinate().record_ref().as_str()], |row| row.get(0),
        ).unwrap();
        assert_eq!(
            u64::try_from(sequence).unwrap(),
            candidate
                .checkpoint()
                .successor()
                .committed_transaction_sequence()
                .get()
        );
    }
    let expected = candidate
        .checkpoint()
        .successor()
        .committed_transaction_sequence()
        .get();
    let count: i64 = port.connection.query_row(
        "SELECT COUNT(*) FROM pale_deployment_operation_session_basis WHERE inserted_transaction_sequence=?1",
        [i64::try_from(expected).unwrap()], |row| row.get(0),
    ).unwrap();
    assert_eq!(count, i64::from(candidate.session_basis().is_some()));
    if let Some(basis) = candidate.session_basis() {
        assert_eq!(
            port.read_operation_session_basis(
                candidate.store(),
                basis.handle(),
                basis.canonical_basis_jcs().len()
            )
            .unwrap()
            .as_ref(),
            Some(basis)
        );
        let stamp: i64 = port.connection.query_row(
            "SELECT inserted_transaction_sequence FROM pale_deployment_operation_session_basis WHERE handle_ref=?1",
            [basis.handle().record_ref().as_str()], |row| row.get(0),
        ).unwrap();
        assert_eq!(u64::try_from(stamp).unwrap(), expected);
    }
}

fn assert_child_basis_absent(port: &SqliteOperationCustody<'_>) {
    let retained: bool = port.connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM pale_deployment_operation_session_basis WHERE handle_ref='child-handle/native-child')",
        [], |row| row.get(0),
    ).unwrap();
    assert!(!retained, "a losing child cannot retain its session basis");
}

pub(super) struct RecoveryCase {
    winner: OperationCustodyCandidate,
    checkpoint: ControlStateCheckpointPreimageV1,
    child_request: CommonRequestEnvelopeV1,
    child_handle: Option<OperationHandleV1>,
    parent_head: (OperationCustodyIndexKey, ControlRecord),
    service: Option<CompletedNativeOperation>,
}

pub(super) fn run(
    fixture: Fixture<'_>,
    attempt: PreparedNativeOperationAttempt<'_, '_>,
    input: NativeOperationCancellationInput<'_>,
    checkpoint: &ControlStateCheckpointV1,
    port: &mut SqliteOperationCustody<'_>,
    anchor: &mut IndependentAnchor,
    order: Order,
) -> RecoveryCase {
    let expected = attempt.observation().1.state().clone();
    let child_wins = matches!(order, Order::ChildFirst | Order::ChildService);
    if order == Order::ChildFirst {
        ingress::check(&fixture, &expected, port, anchor);
    }
    // Sequential callbacks borrow the same store and anchor without duplicating
    // ownership. Only the timing case opens a second connection to advance the
    // transaction during the protected parent read.
    let custody = RefCell::new((port, anchor));
    with_child_start(
        &fixture,
        match order {
            Order::TransactionChangesDuringParentRead => fixture::Admission::Generic,
            Order::ChildFirst | Order::ChildService | Order::CancellationFirst => {
                fixture::Admission::Native
            }
        },
        |lease_ref, lease| {
            let mut borrowed = custody.borrow_mut();
            let (port, anchor) = &mut *borrowed;
            let coordinate = port.coordinate.clone();
            retain_operation_lease_body(*port, &coordinate, lease_ref, lease, 1 << 20).unwrap();
            let mut injected = false;
            let protected = prepare_parent(&fixture, &[&expected], port, anchor, || {
                if order == Order::TransactionChangesDuringParentRead && !injected {
                    let mut writer = connection(fixture.file);
                    let before = crate::store::transaction::read(&writer, &coordinate).unwrap();
                    let mut other = SqliteOperationCustody {
                        connection: &mut writer,
                        coordinate: coordinate.clone(),
                    };
                    // Change only the store transaction clock. The parent's
                    // observation, checkpoint, tree and anchor stay unchanged.
                    retain_operation_lease_body(
                        &mut other,
                        &coordinate,
                        &record_ref("native/interleaved-lease"),
                        fixture.parents.lease,
                        1 << 20,
                    )
                    .unwrap();
                    let after =
                        crate::store::transaction::read(other.connection, &coordinate).unwrap();
                    assert_eq!(after.get(), before.get() + 1);
                    injected = true;
                }
            })
            .unwrap();
            assert_eq!(injected, order == Order::TransactionChangesDuringParentRead);
            protected
        },
        "native-child",
        |_| {},
        |prepared, child_request, _, owners, signs| {
            assert_eq!((owners, signs), (1, 2));
            let mut borrowed = custody.borrow_mut();
            let (port, anchor) = &mut *borrowed;
            if order == Order::TransactionChangesDuringParentRead {
                let writes = port.connection.total_changes();
                let refused = prepared.unwrap().prepare_checkpoint(
                    AccumulatorPreparationLimits::new(4, 10_000, 10_000, 4096, 100_000).unwrap(),
                    *port,
                );
                assert!(
                    matches!(
                        &refused,
                        Err(failure) if matches!(
                            failure.reason(),
                            LocalOperationStartError::AccumulatorPreparation(
                                OperationAccumulatorPreparationError::Store(error)
                            ) if error.kind() == ControlStoreErrorKind::CandidateConflict
                        )
                    ),
                    "the transaction captured before parent recovery must not be refreshed"
                );
                assert_eq!(port.connection.total_changes(), writes);
                assert_child_basis_absent(port);
                let winner = input.completed.candidate().clone();
                let parent_head = retained_parent_head(input.completed.candidate(), &winner);
                return RecoveryCase {
                    winner,
                    checkpoint: fixture.checkpoint.clone(),
                    child_request: child_request.clone(),
                    child_handle: None,
                    parent_head,
                    service: None,
                };
            }
            let cancellation = attempt
                .prepare_cancellation(
                    NativeOperationCancellationInput {
                        completed: input.completed,
                        current: input.current,
                        signing: input.signing,
                        diagnostic_ref: input.diagnostic_ref.clone(),
                        observation_ref: input.observation_ref.clone(),
                        maximum_diagnostic_bytes: input.maximum_diagnostic_bytes,
                        resolve_store: input.resolve_store,
                    },
                    &mut ProvingAuthority,
                    |_, _, _, _| Ok(()),
                    |_, _, _| Ok(signature()),
                )
                .unwrap();
            let (cancel_custody, cancel_fields) = prepare_append(
                cancellation.prepare_append(input.resolve_store).unwrap(),
                fixture.operation,
                (
                    input
                        .completed
                        .candidate()
                        .checkpoint()
                        .successor()
                        .coordinate()
                        .record_ref(),
                    checkpoint,
                ),
                *port,
                64,
            );
            let prepared = prepared
                .unwrap()
                .prepare_checkpoint(
                    AccumulatorPreparationLimits::new(4, 10_000, 10_000, 4096, 100_000).unwrap(),
                    *port,
                )
                .unwrap();
            let (previous_ref, previous) =
                prepared.predecessor().expect("child uses existing domain");
            assert_eq!(previous, checkpoint);
            let mut child_fields = fixture.checkpoint.clone();
            child_fields.control_checkpoint_id =
                ControlCheckpointIdV1::parse("native-child-checkpoint").unwrap();
            child_fields.domain_sequence =
                ControlDomainSequenceV1::new(previous.domain_sequence().get() + 1).unwrap();
            child_fields.previous_control_checkpoint_ref = Some(previous_ref.clone());
            child_fields.previous_control_checkpoint_sha256 =
                Some(previous.canonical_control_checkpoint_sha256().clone());
            child_fields.committed_transaction_sequence = prepared.transaction_sequence();
            child_fields.state_root_sha256 = prepared.state_root().clone();
            child_fields.unresolved_custody_root_sha256 = prepared.unresolved_root().clone();
            child_fields.changed_leaf_keys = prepared.changed_keys().to_vec();
            child_fields.leaf_count = prepared.leaf_count();
            child_fields.issued_at = UnixTimeV1::new(66).unwrap();
            assert_eq!(
                child_fields.committed_transaction_sequence,
                cancel_fields.committed_transaction_sequence
            );
            let (child_custody, pending_child) = prepared
                .issue_checkpoint(
                    &record_ref("native/child-checkpoint"),
                    child_fields.clone(),
                    &child_fields.schema_sha256,
                    &checkpoint_authority(&child_fields),
                    |_, _, _| Ok(signature()),
                )
                .unwrap();
            let (winner, fields, child_handle, service) = if child_wins {
                let writes = port.connection.total_changes();
                let mut committed = None;
                let competing_coordinate = port.coordinate.clone();
                let refused = {
                    // Cancellation reads its actual database predecessor first.
                    // Commit the child after its actual anchor-predecessor read,
                    // forcing cancellation to reach the native transaction CAS.
                    let mut interleaved = interleaving::AfterPredecessor::new(
                        anchor,
                        |anchor: &mut IndependentAnchor| {
                            let mut writer = connection(fixture.file);
                            let mut other = SqliteOperationCustody {
                                connection: &mut writer,
                                coordinate: competing_coordinate,
                            };
                            committed =
                                Some(child_custody.commit(&mut other, anchor, 1 << 20).unwrap());
                        },
                    );
                    let refused = cancel_custody.commit(*port, &mut interleaved).unwrap_err();
                    interleaved.assert_used();
                    refused
                };
                assert!(
                    matches!(refused.reason(), OperationObservationCustodyCommitFailureReason::Database(OperationCustodyDatabaseFailureReason::Store(error)) if error.kind() == ControlStoreErrorKind::CandidateConflict),
                    "actual cancellation CAS refusal: {:?}",
                    refused.reason()
                );
                assert!(refused.try_into_retryable().is_err());
                assert_eq!(port.connection.total_changes(), writes);
                let completed = committed
                    .expect("child committed during predecessor observation")
                    .advance_anchor(*anchor)
                    .unwrap();
                assert_eq!(
                    completed.database_evidence(),
                    OperationCustodyDatabaseEvidence::New
                );
                let winner = completed.candidate().clone();
                let child = pending_child
                    .finish(completed)
                    .unwrap_or_else(|_| panic!("exact child custody"));
                assert_eq!(
                    child
                        .handle()
                        .1
                        .parent_operation()
                        .as_ref()
                        .unwrap()
                        .operation_handle_sha256,
                    *fixture
                        .operation
                        .handle()
                        .1
                        .canonical_operation_handle_sha256()
                );
                assert_eq!(
                    winner.checkpoint().successor().checkpoint_domain_id(),
                    checkpoint.checkpoint_domain_id()
                );
                let (service, fields) = if order == Order::ChildService {
                    let (completed, fields) = child_service::run(
                        &fixture,
                        &child,
                        child_request,
                        &winner,
                        &child_fields,
                        port,
                        anchor,
                    );
                    (Some(completed), fields)
                } else {
                    (None, child_fields)
                };
                (winner, fields, Some(child.handle().1.clone()), service)
            } else {
                let committed = cancel_custody.commit(*port, *anchor).unwrap();
                let writes = port.connection.total_changes();
                let refused = child_custody.commit(*port, *anchor, 1 << 20).unwrap_err();
                assert!(
                    matches!(refused.reason(), OperationStartCommitFailureReason::Database(OperationCustodyDatabaseFailureReason::Store(error)) if error.kind() == ControlStoreErrorKind::CandidateConflict),
                    "actual child CAS refusal: {:?}",
                    refused.reason()
                );
                assert!(refused.try_into_retryable().is_err());
                assert_eq!(port.connection.total_changes(), writes);
                let completed = committed.advance_anchor(*anchor).unwrap();
                assert_eq!(
                    completed.database_evidence(),
                    OperationCustodyDatabaseEvidence::New
                );
                (completed.candidate().clone(), cancel_fields, None, None)
            };
            assert_committed_rows(port, &winner);
            if !child_wins {
                assert_child_basis_absent(port);
                let writes = port.connection.total_changes();
                assert!(
                    matches!(
                        prepare_parent(
                            &fixture,
                            &[&expected, cancellation.observation().1.state()],
                            port,
                            anchor,
                            || {}
                        ),
                        Err(LocalOperationStartError::Coordinate)
                    ),
                    "a committed cancelled parent cannot admit another child"
                );
                assert_eq!(port.connection.total_changes(), writes);
            }
            let parent_candidate = if child_wins {
                input.completed.candidate()
            } else {
                &winner
            };
            let parent_head = retained_parent_head(input.completed.candidate(), parent_candidate);
            RecoveryCase {
                winner,
                checkpoint: fields,
                child_request: child_request.clone(),
                child_handle,
                parent_head,
                service,
            }
        },
    )
}

fn retained_parent_head(
    prepared: &OperationCustodyCandidate,
    current: &OperationCustodyCandidate,
) -> (OperationCustodyIndexKey, ControlRecord) {
    let key = prepared
        .indexes()
        .iter()
        .find_map(|index| {
            if let OperationCustodyIndexMutation::LatestObservation { key, .. } = index {
                Some(key.clone())
            } else {
                None
            }
        })
        .unwrap();
    (
        key,
        current
            .record(OperationCustodyRecordKind::OperationObservation)
            .unwrap()
            .clone(),
    )
}

fn verify_handle_signature(
    domain: &[u8],
    _: &OperationHandlePreimageV1,
    _: &OperationHandleDigest,
    actual: &CanonicalSignatureV1,
) -> Result<(), InvalidResidentRecordV1> {
    assert_eq!(domain, OPERATION_HANDLE_V1_SIGNATURE_DOMAIN);
    assert_eq!(actual, &signature());
    Ok(())
}

pub(super) fn recover(
    evidence: RecoveryCase,
    parent_request: &CommonRequestEnvelopeV1,
    parents: &RecoveryParents<'_>,
    generation: &AdmittedCompiledGeneration,
    location: &ControlStoreLocation,
    file: &std::path::Path,
    anchor: &mut IndependentAnchor,
) {
    let coordinate = evidence.winner.store().clone();
    let sessions =
        DeploymentControlSqliteStore::open_unadmitted_candidate(location, coordinate.clone())
            .unwrap();
    let mut sql = connection(file);
    sql.pragma_update(None, "query_only", true).unwrap();
    anchor.require_read_only();
    let mut port = SqliteOperationCustody {
        connection: &mut sql,
        coordinate: coordinate.clone(),
    };
    let writes = port.connection.total_changes();
    if let Some(completed) = &evidence.service {
        assert_eq!(
            port.observe_exact(completed.custody().candidate()).unwrap(),
            OperationCustodyReadBackOutcome::ExactSuccessor
        );
    } else {
        assert_eq!(
            port.observe_exact(&evidence.winner).unwrap(),
            OperationCustodyReadBackOutcome::ExactSuccessor
        );
    }
    assert_committed_rows(&mut port, &evidence.winner);
    assert_eq!(
        port.read_index(
            &coordinate,
            &OperationCustodyIndexQuery::LatestObservation(evidence.parent_head.0.clone()),
            1 << 20
        )
        .unwrap()
        .record(),
        Some(&evidence.parent_head.1)
    );
    let mut resolver = RecoveryResolver::default();
    resolver.insert_generation(parents.current.generation.compiled_generation);
    resolver.insert_active(parents.current.generation.active_head);
    resolver.insert_fence(parents.current.generation.activation_fence);
    resolver.insert_effective_profile(parents.current.generation.effective_profile);
    let profile = profile();
    let authority = checkpoint_authority(&evidence.checkpoint);
    let parent_selected = [&evidence.parent_head.1];
    let child_selected = evidence
        .child_handle
        .as_ref()
        .map(|_| {
            evidence.service.as_ref().map_or_else(
                || {
                    evidence
                        .winner
                        .record(OperationCustodyRecordKind::OperationObservation)
                        .unwrap()
                },
                |completed| {
                    completed
                        .custody()
                        .candidate()
                        .record(OperationCustodyRecordKind::OperationObservation)
                        .unwrap()
                },
            )
        })
        .into_iter()
        .collect::<Vec<_>>();
    let parent_and_child_selected = parent_selected
        .iter()
        .copied()
        .chain(child_selected.iter().copied())
        .collect::<Vec<_>>();
    // A completed child service retains one additional nonterminal observation
    // for its durably staged native outcome before terminal projection.
    let maximum_observation_revisions = if evidence.service.is_some() { 5 } else { 4 };
    let context = || LocalOperationRecoveryContext {
        store: &coordinate,
        profile: &profile,
        handle_signing_authority: parents.signing,
        checkpoint_authority: &authority,
        historical: &resolver,
        maximum_record_bytes: 1 << 20,
        maximum_operation_lease_bytes: 1 << 20,
        maximum_session_lease_bytes: 1 << 20,
        membership_limits: AccumulatorMembershipLimits::new(3, 10_000, 100_000).unwrap(),
        observation_limits: OperationObservationRecoveryLimits::new(
            maximum_observation_revisions,
            1 << 20,
            1 << 20,
        )
        .unwrap(),
        checkpoint_limits: OperationCheckpointAncestryLimits::new(8, 1 << 20, 8).unwrap(),
    };
    // Recover the parent from disk too. No issued parent or child locator from
    // the original connection enters either recovery call.
    let parent = recover_local_operation(
        context(),
        parent_request,
        &mut port,
        &sessions,
        anchor,
        LocalOperationRecoveryAdmission {
            admit_historical_authority:
                |handle: &OperationHandlePreimageV1, parent: Option<&OperationHandleV1>| {
                    assert!(parent.is_none());
                    assert!(handle.parent_operation.is_none());
                    Ok(())
                },
            verify_handle_signature,
            verify_checkpoint_signature,
            initial_observation: observation_admission!(
                parents,
                evidence.checkpoint,
                &authority,
                &parent_selected,
                admits_parent_recovery_state
            ),
        },
    )
    .unwrap();
    if evidence.child_handle.is_some() {
        assert!(
            evidence
                .winner
                .checkpoint()
                .successor()
                .domain_sequence()
                .get()
                > 1,
            "child recovery must cover starts after earlier domain checkpoints"
        );
    }
    let child = recover_local_child_operation(
        LocalChildOperationRecoveryContext {
            context: context(),
            parent: &parent,
            parent_lease: parent.historical_operation_lease(),
        },
        &evidence.child_request,
        &mut port,
        &sessions,
        anchor,
        LocalOperationRecoveryAdmission {
            admit_historical_authority:
                |handle: &OperationHandlePreimageV1, selected: Option<&OperationHandleV1>| {
                    assert_eq!(selected, Some(parent.handle().1));
                    assert_eq!(
                        handle
                            .parent_operation
                            .as_ref()
                            .unwrap()
                            .operation_handle_sha256,
                        *parent.handle().1.canonical_operation_handle_sha256()
                    );
                    Ok(())
                },
            verify_handle_signature,
            verify_checkpoint_signature,
            initial_observation: observation_admission!(
                parents,
                evidence.checkpoint,
                &authority,
                &child_selected,
                admits_child_recovery_state
            ),
        },
    );
    match evidence.child_handle {
        Some(expected) => {
            let child = child.unwrap();
            assert_eq!(parent.historical_operation_lease(), parents.lease);
            assert_eq!(parent.historical_session_lease(), parents.session.lease());
            assert_ne!(
                child.historical_operation_lease(),
                parent.historical_operation_lease(),
                "child retains its own narrowed lease, never the parent's"
            );
            assert_eq!(
                child.historical_operation_lease().operation_lease_digest(),
                expected.operation_lease_sha256()
            );
            child
                .historical_operation_lease()
                .verify_against_parent_operation(parent.historical_operation_lease())
                .unwrap();
            child
                .historical_operation_lease()
                .verify_against_parent(child.historical_session_lease())
                .unwrap();
            fixture::with_child_session(parents, |session, current| {
                assert_eq!(child.historical_session_lease(), session.lease());
                let child_parents = RecoveryParents {
                    session,
                    current,
                    lease: child.historical_operation_lease(),
                    signing: parents.signing,
                };
                crate::resident_fixture::local_operation_start_fixture::assert_original_session_revalidation(&child, &child_parents);
                assert!(matches!(
                    child.revalidate_session(parents.current, &mut ProvingAuthority),
                    Err(pale_application::resident::AmbientSessionAdmissionError::SessionAuthorityChanged)
                ), "recovered child must not adopt its parent's different original basis");
                assert!(matches!(
                    parent.revalidate_session(current, &mut ProvingAuthority),
                    Err(pale_application::resident::AmbientSessionAdmissionError::SessionAuthorityChanged)
                ), "recovered parent must not adopt the child's independently admitted basis");
            });
            assert_eq!(child.handle().0, &record_ref("child-handle/native-child"));
            assert_eq!(child.handle().1, &expected);
            let child_locator = ControlRecordCoordinate::new(
                child.handle().0.clone(),
                ControlRecordDigest::parse(
                    child
                        .handle()
                        .1
                        .canonical_operation_handle_sha256()
                        .as_str(),
                )
                .unwrap(),
            );
            let depth_zero = recover_local_operation_from_handle(
                LocalOperationHandleRecoveryInput {
                    context: context(),
                    expected_handle: &child_locator,
                    maximum_parent_depth: 0,
                },
                &mut port,
                &sessions,
                anchor,
                |reference, _| {
                    assert_eq!(reference, child.request_coordinate().record_ref());
                    Ok(evidence.child_request.clone())
                },
                LocalOperationRecoveryAdmission {
                    admit_historical_authority:
                        |_: &OperationHandlePreimageV1, _: Option<&OperationHandleV1>| {
                            panic!("a depth refusal must precede handle admission")
                        },
                    verify_handle_signature:
                        |_: &[u8],
                         _: &OperationHandlePreimageV1,
                         _: &OperationHandleDigest,
                         _: &CanonicalSignatureV1| {
                            panic!("a depth refusal must precede signature verification")
                        },
                    verify_checkpoint_signature:
                        |_: &[u8],
                         _: &ControlStateCheckpointPreimageV1,
                         _: &ControlStateCheckpointDigest,
                         _: &CanonicalSignatureV1| {
                            panic!("a depth refusal must precede checkpoint recovery")
                        },
                    initial_observation: observation_admission!(
                        parents,
                        evidence.checkpoint,
                        &authority,
                        &parent_and_child_selected,
                        admits_parent_or_child_recovery_state
                    ),
                },
            );
            assert!(matches!(
                depth_zero,
                Err(LocalOperationRecoveryError::Bounds)
            ));

            let request_reads = Cell::new(0);
            let authority_admissions = Cell::new(0);
            let handle_signatures = Cell::new(0);
            let checkpoint_signatures = Cell::new(0);
            let recovered = recover_local_operation_from_handle(
                LocalOperationHandleRecoveryInput {
                    context: context(),
                    expected_handle: &child_locator,
                    maximum_parent_depth: 1,
                },
                &mut port,
                &sessions,
                anchor,
                |reference, wire| {
                    request_reads.set(request_reads.get() + 1);
                    let expected_request = if reference == child.request_coordinate().record_ref() {
                        &evidence.child_request
                    } else if reference == parent.request_coordinate().record_ref() {
                        parent_request
                    } else {
                        return Err(InvalidResidentRecordV1::ContextualAdmission {
                            record: "saved operation request",
                            invariant: "known request reference",
                        });
                    };
                    if pale_core::canon::to_canonical_string(&wire)?
                        != pale_core::canon::to_canonical_string(expected_request)?
                    {
                        return Err(InvalidResidentRecordV1::ContextualAdmission {
                            record: "saved operation request",
                            invariant: "exact retained request bytes",
                        });
                    }
                    Ok(expected_request.clone())
                },
                LocalOperationRecoveryAdmission {
                    admit_historical_authority:
                        |handle: &OperationHandlePreimageV1,
                         selected: Option<&OperationHandleV1>| {
                            let call = authority_admissions.get();
                            authority_admissions.set(call + 1);
                            if call == 0 {
                                assert!(handle.parent_operation.is_none());
                                assert!(selected.is_none());
                            } else {
                                assert_eq!(call, 1);
                                assert_eq!(selected, Some(parent.handle().1));
                                assert_eq!(
                                    handle
                                        .parent_operation
                                        .as_ref()
                                        .unwrap()
                                        .operation_handle_sha256,
                                    *parent.handle().1.canonical_operation_handle_sha256()
                                );
                            }
                            Ok(())
                        },
                    verify_handle_signature:
                        |domain: &[u8],
                         _: &OperationHandlePreimageV1,
                         _: &OperationHandleDigest,
                         actual: &CanonicalSignatureV1| {
                            handle_signatures.set(handle_signatures.get() + 1);
                            assert_eq!(domain, OPERATION_HANDLE_V1_SIGNATURE_DOMAIN);
                            assert_eq!(actual, &signature());
                            Ok(())
                        },
                    verify_checkpoint_signature:
                        |domain: &[u8],
                         _: &ControlStateCheckpointPreimageV1,
                         _: &ControlStateCheckpointDigest,
                         actual: &CanonicalSignatureV1| {
                            checkpoint_signatures.set(checkpoint_signatures.get() + 1);
                            assert_eq!(domain, CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN);
                            assert_eq!(actual, &signature());
                            Ok(())
                        },
                    initial_observation: observation_admission!(
                        parents,
                        evidence.checkpoint,
                        &authority,
                        &parent_and_child_selected,
                        admits_parent_or_child_recovery_state
                    ),
                },
            )
            .unwrap();
            assert_eq!(request_reads.get(), 2);
            assert_eq!(authority_admissions.get(), 2);
            assert_eq!(handle_signatures.get(), 2);
            assert_eq!(
                checkpoint_signatures.get(),
                4,
                "parent and child each verify current and selected-start checkpoints"
            );
            assert_eq!(recovered.request(), &evidence.child_request);
            assert_eq!(recovered.operation().handle(), child.handle());
            if evidence.service.is_none() {
                let mut admit_request =
                    |reference: &ResidentRecordRefV1, wire: CommonRequestEnvelopeWireV1| {
                        let expected_request =
                            if reference == child.request_coordinate().record_ref() {
                                &evidence.child_request
                            } else if reference == parent.request_coordinate().record_ref() {
                                parent_request
                            } else {
                                return Err(InvalidResidentRecordV1::ContextualAdmission {
                                    record: "saved operation request",
                                    invariant: "known request reference",
                                });
                            };
                        if pale_core::canon::to_canonical_string(&wire)?
                            != pale_core::canon::to_canonical_string(expected_request)?
                        {
                            return Err(InvalidResidentRecordV1::ContextualAdmission {
                                record: "saved operation request",
                                invariant: "exact retained request bytes",
                            });
                        }
                        Ok(expected_request.clone())
                    };
                let handle_authorities = Cell::new(0);
                let mut admit_handle_authority =
                    |handle: &OperationHandlePreimageV1, selected: Option<&OperationHandleV1>| {
                        let call = handle_authorities.get();
                        handle_authorities.set(call + 1);
                        assert_eq!(handle.parent_operation.is_some(), call == 1);
                        assert_eq!(selected.is_some(), call == 1);
                        Ok(())
                    };
                let mut verify_handle =
                    |domain: &[u8],
                     _: &OperationHandlePreimageV1,
                     _: &OperationHandleDigest,
                     actual: &CanonicalSignatureV1| {
                        assert_eq!(domain, OPERATION_HANDLE_V1_SIGNATURE_DOMAIN);
                        assert_eq!(actual, &signature());
                        Ok(())
                    };
                let mut admit_checkpoint =
                    |_: &ResidentRecordRefV1, wire: ControlStateCheckpointWireV1| {
                        ControlStateCheckpointV1::admit_signed_wire(
                            wire,
                            &evidence.checkpoint.schema_sha256,
                            &authority,
                            verify_checkpoint_signature,
                        )
                    };
                let mut resolve_observation_signer =
                    |reference: &ResidentRecordRefV1, wire: &OperationObservationWireV1| {
                        if let Some(record) = parent_and_child_selected
                            .iter()
                            .find(|record| record.coordinate().record_ref() == reference)
                        {
                            let expected: OperationObservationWireV1 =
                                serde_json::from_slice(record.canonical_record_jcs().as_slice())
                                    .unwrap();
                            assert_eq!(
                                wire, &expected,
                                "selected control observation must retain exact bytes"
                            );
                        }
                        assert_eq!(
                            wire.deployment_runtime_authority_ref(),
                            parents.session.runtime_authority_ref()
                        );
                        assert_eq!(
                            wire.deployment_runtime_authority_sha256().as_str(),
                            parents.session.runtime_authority_sha256().as_str()
                        );
                        Ok(parents.signing.clone())
                    };
                let mut admit_observation_state = |state: &OperationObservationStateV1| {
                    assert!(
                        admits_parent_or_child_recovery_state(state),
                        "unexpected recovered control state: {state:?}"
                    );
                    Ok(())
                };
                let mut admit_rotation =
                    |_: &ResidentRecordRefV1,
                     _: &ArtifactDigest,
                     _: &ResidentRecordRefV1,
                     _: &ArtifactDigest| {
                        panic!("the child fixture has no runtime-authority rotation")
                    };
                let mut verify_observation_signature =
                    |domain: &[u8],
                     _: &OperationObservationPreimageV1,
                     _: &OperationObservationDigest,
                     actual: &CanonicalSignatureV1| {
                        assert_eq!(domain, OPERATION_OBSERVATION_V1_SIGNATURE_DOMAIN);
                        assert_eq!(actual, &signature());
                        Ok(())
                    };
                let mut verify_control_checkpoint_signature =
                    |domain: &[u8],
                     preimage: &ControlStateCheckpointPreimageV1,
                     digest: &ControlStateCheckpointDigest,
                     actual: &CanonicalSignatureV1| {
                        verify_checkpoint_signature(domain, preimage, digest, actual)
                    };
                let recovered_control = recover_operation_control(
                    OperationControlRecoveryInput {
                        operation: LocalOperationHandleRecoveryInput {
                            context: context(),
                            expected_handle: &child_locator,
                            maximum_parent_depth: 1,
                        },
                        observation: OperationObservationSelection::SelectedHead,
                        observation_limits: OperationObservationRecoveryLimits::new(
                            1,
                            1 << 20,
                            1 << 20,
                        )
                        .unwrap(),
                        checkpoint_limits: OperationCheckpointAncestryLimits::new(8, 1 << 20, 8)
                            .unwrap(),
                        membership_limits: AccumulatorMembershipLimits::new(1, 10_000, 100_000)
                            .unwrap(),
                    },
                    &mut port,
                    &sessions,
                    anchor,
                    OperationControlRecoveryAdmission {
                        admit_request: &mut admit_request,
                        admit_handle_authority: &mut admit_handle_authority,
                        verify_handle_signature: &mut verify_handle,
                        admit_checkpoint: &mut admit_checkpoint,
                        resolve_observation_signer: &mut resolve_observation_signer,
                        admit_observation_state: &mut admit_observation_state,
                        admit_rotation: &mut admit_rotation,
                        verify_observation_signature: &mut verify_observation_signature,
                        verify_checkpoint_signature: &mut verify_control_checkpoint_signature,
                    },
                )
                .unwrap();
                assert_eq!(handle_authorities.get(), 2);
                assert_eq!(
                    recovered_control.operation().operation().handle(),
                    child.handle()
                );
                assert_eq!(
                    recovered_control.observation().selected_head().1.state(),
                    &OperationObservationStateV1::Active {
                        phase: OperationActivePhaseV1::Resolving,
                    }
                );
            }
            let membership = evidence
                .winner
                .indexes()
                .iter()
                .find_map(|index| {
                    if let OperationCustodyIndexMutation::ParentChildMembership { key, .. } = index
                    {
                        Some(key.clone())
                    } else {
                        None
                    }
                })
                .expect("child start commits exact parent membership");
            assert_eq!(
                port.read_index(
                    &coordinate,
                    &OperationCustodyIndexQuery::ParentChildMembership(membership),
                    1 << 20
                )
                .unwrap()
                .record(),
                evidence
                    .winner
                    .record(OperationCustodyRecordKind::OperationHandle)
            );
            if let Some(completed) = &evidence.service {
                child_service::recover(
                    &child,
                    &evidence.child_request,
                    parents,
                    generation,
                    (completed, &evidence.checkpoint),
                    &mut port,
                    anchor,
                );
                assert_eq!(
                    port.read_index(
                        &coordinate,
                        &OperationCustodyIndexQuery::LatestObservation(evidence.parent_head.0),
                        1 << 20,
                    )
                    .unwrap()
                    .record(),
                    Some(&evidence.parent_head.1),
                    "child completion never promotes its parent's observation"
                );
            }
        }
        None => {
            assert!(matches!(child, Err(LocalOperationRecoveryError::Missing)));
            // The separately retained child lease is only a prerequisite; it
            // must not manufacture a request, locator, observation or index.
            for reference in [
                "child-request/native-child",
                "child-handle/native-child",
                "child-observation/native-child",
                "native/child-checkpoint",
            ] {
                let records: i64 = port
                    .connection
                    .query_row(
                        "SELECT COUNT(*) FROM pale_deployment_control_record WHERE record_ref=?1",
                        [reference],
                        |row| row.get(0),
                    )
                    .unwrap();
                let indexes: i64 = port
                    .connection
                    .query_row(
                        "SELECT COUNT(*) FROM pale_deployment_operation_index WHERE record_ref=?1",
                        [reference],
                        |row| row.get(0),
                    )
                    .unwrap();
                assert_eq!((records, indexes), (0, 0));
            }
        }
    }
    assert_eq!(
        port.connection.total_changes(),
        writes,
        "cold recovery is read-only"
    );
}
