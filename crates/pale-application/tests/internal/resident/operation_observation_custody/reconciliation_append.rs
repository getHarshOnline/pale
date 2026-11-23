//! Proof-bearing current-head reconciliation append behavior.

use super::*;
use crate::resident::operation_dispatch::{OperationAppendFailure, OperationCheckpointInput};
use crate::resident::operation_observation_custody::reconciliation_append::{
    append_current_head_reconciliation, CurrentHeadReconciliationAppendFailure,
    CurrentHeadReconciliationAppendInput,
};
use crate::resident::operation_service::{ResidentOperationService, ResidentOperationTaskState};
use crate::resident::task_owner::ResidentOperationTaskLimits;
use crate::resident::task_reservation::ResidentAggregateTaskLimits;
use crate::resident::{OperationAttemptError, RecoveredOperationObservation};
use pale_ports::{
    AccumulatorMutation, AccumulatorPosition, AccumulatorPreparationBasis,
    AccumulatorPreparationLimits, PreparedOperationAccumulator, StateAccumulatorBatch,
    UnresolvedCustodyAccumulatorBatch,
};
use std::num::NonZeroUsize;

fn resident_tasks() -> ResidentOperationTaskState {
    ResidentOperationTaskState::new(
        ResidentOperationTaskLimits::new(NonZeroUsize::new(1).unwrap()),
        NonZeroUsize::new(1).unwrap(),
        ResidentAggregateTaskLimits::new(NonZeroUsize::new(1).unwrap()),
    )
    .expect("one bounded resident task slot")
}

fn uncertain_state(kind: &str) -> OperationObservationStateV1 {
    match kind {
        "unknown" => OperationObservationStateV1::OutcomeUnknown {
            diagnostic: evidence("diagnostic/unknown", 'a'),
            reconciliation: ReconciliationFrontierReferenceV1 {
                record_ref: record_ref("reconciliation/unknown"),
                record_sha256: ReconciliationFrontierDigest::parse(sha('b')).unwrap(),
            },
            effect_receipt: None,
        },
        "expired" => OperationObservationStateV1::Expired {
            diagnostic: evidence("diagnostic/expired", 'c'),
        },
        "recovering" => OperationObservationStateV1::Recovering {
            reconciliation: ReconciliationFrontierReferenceV1 {
                record_ref: record_ref("reconciliation/recovering"),
                record_sha256: ReconciliationFrontierDigest::parse(sha('d')).unwrap(),
            },
        },
        _ => panic!("unknown fixture state"),
    }
}

fn conclusive_state(kind: &str) -> OperationObservationStateV1 {
    match kind {
        "succeeded" => OperationObservationStateV1::Succeeded {
            result: CapabilityResultReferenceV1 {
                record_ref: record_ref("result/reconciled"),
                record_sha256: CapabilityResultDigest::parse(sha('e')).unwrap(),
            },
            effect_receipt: None,
        },
        "failed" => OperationObservationStateV1::Failed {
            result: None,
            diagnostic: evidence("diagnostic/failed", 'f'),
        },
        "cancelled" => OperationObservationStateV1::Cancelled {
            diagnostic: evidence("diagnostic/cancelled", '1'),
        },
        _ => panic!("unknown conclusive fixture state"),
    }
}

fn append_response(
    request: &pale_ports::OperationAccumulatorPreparation,
) -> Result<PreparedOperationAccumulator, pale_ports::OperationAccumulatorPreparationError> {
    let AccumulatorPreparationBasis::Successor(prior) = request.basis() else {
        panic!("reconciliation is an observation successor")
    };
    assert_eq!(request.state().changes().len(), 1);
    assert!(request.unresolved().changes().is_empty());
    Ok(PreparedOperationAccumulator::new(
        request.clone(),
        StateAccumulatorBatch::new(
            prior.state_root().clone(),
            artifact_digest('2'),
            vec![AccumulatorMutation::new(
                AccumulatorPosition::Leaf([9; 32]),
                Some(vec![3; 32]),
                Some(vec![4; 32]),
                32,
            )
            .unwrap()],
            1,
        )
        .unwrap(),
        UnresolvedCustodyAccumulatorBatch::new(
            prior.unresolved_root().clone(),
            prior.unresolved_root().clone(),
            vec![],
            1,
        )
        .unwrap(),
    )
    .unwrap())
}

fn reject_accumulator_preparation(
    _: &pale_ports::OperationAccumulatorPreparation,
) -> Result<PreparedOperationAccumulator, pale_ports::OperationAccumulatorPreparationError> {
    Err(pale_ports::OperationAccumulatorPreparationError::Limit)
}

struct CountingResolver<'a>(&'a Cell<usize>);

impl OperationObservationAuthorityStoreResolver for CountingResolver<'_> {
    fn resolve(
        &self,
        observation: &OperationObservationV1,
    ) -> Result<AuthorityStoreCoordinate, OperationObservationAuthorityStoreRejection> {
        self.0.set(self.0.get() + 1);
        assert_eq!(
            observation.deployment_runtime_authority_ref(),
            &record_ref("runtime-authority")
        );
        Ok(store_coordinate())
    }
}

struct RejectingResolver<'a>(&'a Cell<usize>);

impl OperationObservationAuthorityStoreResolver for RejectingResolver<'_> {
    fn resolve(
        &self,
        _: &OperationObservationV1,
    ) -> Result<AuthorityStoreCoordinate, OperationObservationAuthorityStoreRejection> {
        self.0.set(self.0.get() + 1);
        Err(OperationObservationAuthorityStoreRejection)
    }
}

struct NoCustodyCalls {
    transaction_preparations: Cell<usize>,
    accumulator_preparations: Cell<usize>,
}

impl pale_ports::ControlTransactionPort for NoCustodyCalls {
    fn prepare_transaction(
        &mut self,
        _: &AuthorityStoreCoordinate,
    ) -> Result<pale_ports::ControlTransactionPreparation, ControlStoreError> {
        self.transaction_preparations
            .set(self.transaction_preparations.get() + 1);
        panic!("rejected reconciliation must not prepare accumulator custody")
    }
}

impl OperationCustodyPort for NoCustodyCalls {
    fn read_a2a_task_bindings(
        &mut self,
        _: &AuthorityStoreCoordinate,
        _: &pale_ports::OperationA2aBindingScope,
        _: Option<&pale_ports::OperationA2aBindingCursor>,
        _: usize,
        _: usize,
    ) -> Result<pale_ports::OperationA2aBindings, ControlStoreError> {
        panic!("reconciliation append does not discover remote tasks")
    }

    fn read_publication_sources(
        &mut self,
        _: &AuthorityStoreCoordinate,
        _: Option<&pale_ports::OperationPublicationCursor>,
        _: usize,
        _: usize,
    ) -> Result<pale_ports::OperationPublicationSources, pale_ports::OperationPublicationReadError>
    {
        panic!("reconciliation append does not discover publications")
    }

    fn read_index(
        &mut self,
        _: &AuthorityStoreCoordinate,
        _: &OperationCustodyIndexQuery,
        _: usize,
    ) -> Result<pale_ports::OperationCustodyIndexObservation, ControlStoreError> {
        panic!("rejected reconciliation must not read custody indexes")
    }

    fn read_operation_record(
        &mut self,
        _: &AuthorityStoreCoordinate,
        _: &ControlRecordCoordinate,
        _: usize,
    ) -> Result<Option<ControlRecord>, ControlStoreError> {
        panic!("rejected reconciliation must not read custody records")
    }

    fn read_checkpoint_head(
        &mut self,
        _: &AuthorityStoreCoordinate,
        _: &ControlCheckpointDomainIdV1,
        _: usize,
    ) -> Result<Option<(ControlCheckpointHead, ControlRecord)>, ControlStoreError> {
        panic!("rejected reconciliation must not read checkpoint custody")
    }

    fn verify_accumulator_membership(
        &mut self,
        _: &pale_ports::OperationAccumulatorMembership,
    ) -> Result<
        pale_ports::VerifiedOperationAccumulatorMembership,
        pale_ports::OperationAccumulatorPreparationError,
    > {
        panic!("reconciliation append does not repeat recovery membership")
    }

    fn prepare_accumulator(
        &mut self,
        _: &pale_ports::OperationAccumulatorPreparation,
    ) -> Result<PreparedOperationAccumulator, pale_ports::OperationAccumulatorPreparationError>
    {
        self.accumulator_preparations
            .set(self.accumulator_preparations.get() + 1);
        panic!("rejected reconciliation must not prepare an accumulator")
    }

    fn compare_and_retain_operation_lease(
        &mut self,
        _: &AuthorityStoreCoordinate,
        _: &ControlRecord,
    ) -> Result<OperationCustodyCommitOutcome, ControlStoreError> {
        panic!("reconciliation append does not retain a lease")
    }

    fn compare_and_commit(
        &mut self,
        _: &OperationCustodyCandidate,
    ) -> Result<OperationCustodyCommitOutcome, ControlStoreError> {
        panic!("rejected reconciliation must not commit")
    }

    fn observe_exact(
        &mut self,
        _: &OperationCustodyCandidate,
    ) -> Result<OperationCustodyReadBackOutcome, ControlStoreError> {
        panic!("rejected reconciliation must not read back a candidate")
    }
}

struct FailureAnchor {
    ambiguous_advance: bool,
    observations: usize,
}

struct RecoveringAmbiguousAnchor {
    observations: usize,
    advances: usize,
}

impl AntiRollbackAnchorPort for RecoveringAmbiguousAnchor {
    fn read(
        &mut self,
        _: &pale_ports::AntiRollbackAnchorKey,
    ) -> Result<Option<pale_ports::AntiRollbackAnchor>, AntiRollbackAnchorError> {
        panic!("observation custody uses the prepared transition")
    }

    fn compare_and_advance(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError> {
        self.advances += 1;
        Err(AntiRollbackAnchorError::new(
            AntiRollbackAnchorErrorKind::OutcomeUnknown,
        ))
    }

    fn observe(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError> {
        self.observations += 1;
        match self.observations {
            1 => Ok(AntiRollbackAnchorObservation::ExactPredecessor),
            2 => Err(AntiRollbackAnchorError::new(
                AntiRollbackAnchorErrorKind::Unavailable,
            )),
            3 => Ok(AntiRollbackAnchorObservation::ExactSuccessor),
            _ => panic!("ambiguous anchor recovery performs three observations"),
        }
    }
}

impl AntiRollbackAnchorPort for FailureAnchor {
    fn read(
        &mut self,
        _: &pale_ports::AntiRollbackAnchorKey,
    ) -> Result<Option<pale_ports::AntiRollbackAnchor>, AntiRollbackAnchorError> {
        panic!("observation custody uses the exact prepared transition")
    }

    fn compare_and_advance(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError> {
        if self.ambiguous_advance {
            Err(AntiRollbackAnchorError::new(
                AntiRollbackAnchorErrorKind::OutcomeUnknown,
            ))
        } else {
            Ok(AntiRollbackAnchorAdvanceDisposition::Advanced)
        }
    }

    fn observe(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError> {
        self.observations += 1;
        if self.ambiguous_advance && self.observations == 2 {
            Err(AntiRollbackAnchorError::new(
                AntiRollbackAnchorErrorKind::Unavailable,
            ))
        } else {
            Ok(AntiRollbackAnchorObservation::ExactPredecessor)
        }
    }
}

struct Case {
    fixture: OperationFixture,
    recovered: Option<RecoveredOperationObservation>,
    authority: SigningAuthorityBindingV1,
    observation_schema: SchemaDigest,
    checkpoint_schema: SchemaDigest,
    checkpoint_authority_source: ControlStateCheckpointPreimageV1,
}

impl Case {
    fn new(current: OperationObservationStateV1) -> Self {
        let fixture = checkpoint_preparation::profile_fixture();
        let authority = signing_authority();
        let current_ref = record_ref("operation/observation-reconciliation-current");
        let current = issue_observation(
            &authority,
            &fixture.handle_ref,
            &fixture.handle,
            fixture.successor.schema_sha256().clone(),
            OperationObservationRevisionV1::new(3).unwrap(),
            Some((&fixture.successor_ref, &fixture.successor)),
            current,
            70,
        );
        let checkpoints = checkpoint_preparation::profile_checkpoints();
        let coordinate = record_projection::coordinate(
            &fixture.handle_ref,
            fixture.handle.canonical_operation_handle_sha256().as_str(),
        )
        .unwrap();
        let recovered = RecoveredOperationObservation::fixture_for_handle(
            coordinate,
            (fixture.successor_ref.clone(), fixture.successor.clone()),
            (current_ref, current),
            (checkpoints.successor_ref, checkpoints.successor.clone()),
            store_coordinate(),
        );
        let mut checkpoint_authority_source = checkpoint_preimage(
            3,
            Some((
                checkpoints.predecessor_ref,
                checkpoints
                    .predecessor
                    .canonical_control_checkpoint_sha256()
                    .clone(),
            )),
        );
        checkpoint_authority_source.checkpoint_domain_id =
            checkpoints.successor.checkpoint_domain_id().clone();
        checkpoint_authority_source.checkpoint_profile_sha256 =
            checkpoints.successor.checkpoint_profile_sha256().clone();
        let checkpoint_schema = checkpoint_authority_source.schema_sha256.clone();
        let observation_schema = fixture.successor.schema_sha256().clone();
        Self {
            fixture,
            recovered: Some(recovered),
            authority,
            observation_schema,
            checkpoint_schema,
            checkpoint_authority_source,
        }
    }

    fn take_recovered(&mut self) -> RecoveredOperationObservation {
        self.recovered
            .take()
            .expect("recovered proof is single-use")
    }

    fn input<'a>(
        &'a self,
        recovered: RecoveredOperationObservation,
        state: OperationObservationStateV1,
        checkpoint_signer: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    ) -> CurrentHeadReconciliationAppendInput<'a> {
        CurrentHeadReconciliationAppendInput {
            recovered,
            handle_ref: &self.fixture.handle_ref,
            handle: &self.fixture.handle,
            successor_ref: record_ref("operation/observation-reconciliation-conclusive"),
            state,
            observed_at: UnixTimeV1::new(71).unwrap(),
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &self.observation_schema,
                signing_authority: &self.authority,
            },
            selection: checkpoint_preparation::selection(),
            checkpoint: OperationCheckpointInput {
                reference: record_ref("operation/checkpoint-reconciliation"),
                id: ControlCheckpointIdV1::parse("checkpoint-reconciliation").unwrap(),
            },
            checkpoint_schema: &self.checkpoint_schema,
            checkpoint_authority: checkpoint_signer,
            accumulator_limits: AccumulatorPreparationLimits::new(1, 10_000, 10_000, 4096, 100_000)
                .unwrap(),
        }
    }
}

#[test]
fn exact_unknown_expired_and_recovering_heads_append_only_owner_admitted_conclusions() {
    for (current, conclusive) in [
        ("unknown", "succeeded"),
        ("expired", "failed"),
        ("recovering", "cancelled"),
    ] {
        let mut case = Case::new(uncertain_state(current));
        let evidence_calls = Cell::new(0);
        let observation_signs = Cell::new(0);
        let checkpoint_signs = Cell::new(0);
        let resolver_calls = Cell::new(0);
        let selected_ref = case.recovered.as_ref().unwrap().selected().0.clone();
        let predecessor_ref = case.recovered.as_ref().unwrap().selected_head().0.clone();
        let recovered = case.take_recovered();
        let checkpoint_signer = checkpoint_authority(&case.checkpoint_authority_source);
        let mut store = RecordingStore::committing(OperationCustodyCommitOutcome::Committed)
            .with_preparation(
                append_response,
                ControlTransactionSequenceV1::new(3).unwrap(),
            );
        let mut anchor = RecordingAnchor::advancing();
        let completed = append_current_head_reconciliation(
            case.input(recovered, conclusive_state(conclusive), &checkpoint_signer),
            &CountingResolver(&resolver_calls),
            &mut store,
            &mut anchor,
            |state| {
                evidence_calls.set(evidence_calls.get() + 1);
                assert_eq!(state, &conclusive_state(conclusive));
                Ok(())
            },
            |_, _, _, _| panic!("the fixture does not rotate runtime authority"),
            |domain, preimage, digest| {
                observation_signs.set(observation_signs.get() + 1);
                assert_eq!(domain, OPERATION_OBSERVATION_V1_SIGNATURE_DOMAIN);
                assert_eq!(
                    digest,
                    &OperationObservationDigest::hash_canonical(preimage).unwrap()
                );
                Ok(signature())
            },
            |domain, preimage, digest| {
                checkpoint_signs.set(checkpoint_signs.get() + 1);
                assert_eq!(domain, CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN);
                assert_eq!(
                    digest,
                    &ControlStateCheckpointDigest::hash_canonical(preimage).unwrap()
                );
                Ok(signature())
            },
        )
        .expect("exact admitted conclusion commits and anchors");
        assert_ne!(selected_ref, predecessor_ref);
        assert_eq!(completed.successor().1.revision().get(), 4);
        assert_eq!(
            completed.successor().1.predecessor_ref(),
            &Some(predecessor_ref)
        );
        assert_eq!(
            completed.successor().1.state(),
            &conclusive_state(conclusive)
        );
        assert_eq!(completed.checkpoint().1.domain_sequence().get(), 3);
        assert_eq!(
            (
                evidence_calls.get(),
                observation_signs.get(),
                checkpoint_signs.get()
            ),
            (1, 1, 1)
        );
        assert_eq!(store.mutation_counts(), (1, 1));
        assert_eq!(anchor.access_counts(), (1, 1));
        assert_eq!(resolver_calls.get(), 1);
        let _ = completed.into_custody();
    }
}

#[test]
fn signed_successors_cross_projection_root_and_attachment_failures_without_reissue() {
    for (current, conclusive) in [
        ("unknown", "succeeded"),
        ("expired", "failed"),
        ("recovering", "cancelled"),
    ] {
        let mut case = Case::new(uncertain_state(current));
        let recovered = case.take_recovered();
        let checkpoint_signer = checkpoint_authority(&case.checkpoint_authority_source);
        let evidence_calls = Cell::new(0);
        let observation_signs = Cell::new(0);
        let checkpoint_signs = Cell::new(0);
        let resolver_calls = Cell::new(0);
        let mut rejected_store = NoCustodyCalls {
            transaction_preparations: Cell::new(0),
            accumulator_preparations: Cell::new(0),
        };
        let mut rejected_anchor = RecordingAnchor::advancing();

        let failure = append_current_head_reconciliation(
            case.input(recovered, conclusive_state(conclusive), &checkpoint_signer),
            &RejectingResolver(&resolver_calls),
            &mut rejected_store,
            &mut rejected_anchor,
            |state| {
                evidence_calls.set(evidence_calls.get() + 1);
                assert_eq!(state, &conclusive_state(conclusive));
                Ok(())
            },
            |_, _, _, _| panic!("the fixture does not rotate runtime authority"),
            |_, _, _| {
                observation_signs.set(observation_signs.get() + 1);
                Ok(signature())
            },
            |_, _, _| {
                checkpoint_signs.set(checkpoint_signs.get() + 1);
                Ok(signature())
            },
        )
        .expect_err("runtime-authority projection stops after successor issuance");
        let continuation = match failure {
            CurrentHeadReconciliationAppendFailure::Prepared {
                continuation,
                failure,
            } => {
                assert!(matches!(
                    *failure,
                    OperationAppendFailure::Projection(
                        OperationObservationCustodyProjectionError::RuntimeAuthority(_)
                    )
                ));
                continuation
            }
            other => panic!("unexpected projection phase: {other:?}"),
        };
        let successor_bytes = canon::to_canonical_string(continuation.successor().1).unwrap();
        assert_eq!(
            (
                evidence_calls.get(),
                observation_signs.get(),
                checkpoint_signs.get()
            ),
            (1, 1, 0)
        );

        let mut root_store = RecordingStore::committing(OperationCustodyCommitOutcome::Committed)
            .with_preparation(
                reject_accumulator_preparation,
                ControlTransactionSequenceV1::new(3).unwrap(),
            );
        let mut root_anchor = RecordingAnchor::advancing();
        let failure = continuation
            .continue_custody(
                &CountingResolver(&resolver_calls),
                &mut root_store,
                &mut root_anchor,
                |_, _, _| {
                    checkpoint_signs.set(checkpoint_signs.get() + 1);
                    Ok(signature())
                },
            )
            .expect_err("accumulator preparation keeps the pre-checkpoint phase");
        let continuation = match failure {
            CurrentHeadReconciliationAppendFailure::Prepared {
                continuation,
                failure,
            } => {
                assert!(matches!(
                    *failure,
                    OperationAppendFailure::Projection(
                        OperationObservationCustodyProjectionError::AccumulatorPreparation(
                            pale_ports::OperationAccumulatorPreparationError::Limit
                        )
                    )
                ));
                continuation
            }
            other => panic!("unexpected root-preparation phase: {other:?}"),
        };
        assert_eq!(
            canon::to_canonical_string(continuation.successor().1).unwrap(),
            successor_bytes
        );
        assert_eq!(checkpoint_signs.get(), 0);

        let mut attachment_store =
            RecordingStore::committing(OperationCustodyCommitOutcome::Committed).with_preparation(
                append_response,
                ControlTransactionSequenceV1::new(3).unwrap(),
            );
        let mut attachment_anchor = RecordingAnchor::advancing();
        let failure = continuation
            .continue_custody_with_checkpoint_attachment(
                &CountingResolver(&resolver_calls),
                &mut attachment_store,
                &mut attachment_anchor,
                |_, _, _| {
                    checkpoint_signs.set(checkpoint_signs.get() + 1);
                    Ok(signature())
                },
                |_, _, _| Err(OperationObservationCheckpointConstructionRejection.into()),
            )
            .expect_err("attachment failure retains the issued checkpoint");
        let issued = match failure {
            CurrentHeadReconciliationAppendFailure::Issued {
                continuation,
                failure,
            } => {
                assert!(matches!(
                    *failure,
                    OperationAppendFailure::Projection(
                        OperationObservationCustodyProjectionError::Checkpoint(_)
                    )
                ));
                continuation
            }
            other => panic!("unexpected post-sign phase: {other:?}"),
        };
        let checkpoint_bytes = canon::to_canonical_string(issued.checkpoint().1).unwrap();
        assert_eq!(
            canon::to_canonical_string(issued.successor().1).unwrap(),
            successor_bytes
        );
        assert_eq!(checkpoint_signs.get(), 1);

        let mut commit_store = RecordingStore::committing(OperationCustodyCommitOutcome::Committed)
            .with_preparation(
                append_response,
                ControlTransactionSequenceV1::new(3).unwrap(),
            );
        let mut commit_anchor = RecordingAnchor::advancing();
        let completed = issued
            .continue_custody(
                &CountingResolver(&resolver_calls),
                &mut commit_store,
                &mut commit_anchor,
            )
            .expect("the issued values enter custody without another signature");
        assert_eq!(
            canon::to_canonical_string(completed.successor().1).unwrap(),
            successor_bytes
        );
        assert_eq!(
            canon::to_canonical_string(completed.checkpoint().1).unwrap(),
            checkpoint_bytes
        );
        assert_eq!(
            (
                evidence_calls.get(),
                observation_signs.get(),
                checkpoint_signs.get(),
                resolver_calls.get()
            ),
            (1, 1, 1, 4)
        );
    }
}

#[test]
fn ambiguous_database_outcomes_use_only_exact_readback_of_the_retained_candidate() {
    for (current, conclusive) in [
        ("unknown", "succeeded"),
        ("expired", "failed"),
        ("recovering", "cancelled"),
    ] {
        let mut case = Case::new(uncertain_state(current));
        let recovered = case.take_recovered();
        let checkpoint_signer = checkpoint_authority(&case.checkpoint_authority_source);
        let evidence_calls = Cell::new(0);
        let observation_signs = Cell::new(0);
        let checkpoint_signs = Cell::new(0);
        let resolver_calls = Cell::new(0);
        let mut store = RecordingStore::failing_commit(
            ControlStoreError::new(ControlStoreErrorKind::OutcomeUnknown),
            [
                Ok(OperationCustodyReadBackOutcome::Predecessor),
                Err(ControlStoreError::new(ControlStoreErrorKind::Storage)),
            ],
        )
        .with_preparation(
            append_response,
            ControlTransactionSequenceV1::new(3).unwrap(),
        );
        let mut anchor = RecordingAnchor::advancing();
        let failure = append_current_head_reconciliation(
            case.input(recovered, conclusive_state(conclusive), &checkpoint_signer),
            &CountingResolver(&resolver_calls),
            &mut store,
            &mut anchor,
            |_| {
                evidence_calls.set(evidence_calls.get() + 1);
                Ok(())
            },
            |_, _, _, _| panic!("the fixture does not rotate runtime authority"),
            |_, _, _| {
                observation_signs.set(observation_signs.get() + 1);
                Ok(signature())
            },
            |_, _, _| {
                checkpoint_signs.set(checkpoint_signs.get() + 1);
                Ok(signature())
            },
        )
        .expect_err("failed ambiguity readback retains the exact database candidate");
        let failure = match failure {
            CurrentHeadReconciliationAppendFailure::Commit(failure) => *failure,
            other => panic!("unexpected database phase: {other:?}"),
        };
        let successor_bytes = canon::to_canonical_string(failure.successor().1).unwrap();
        let checkpoint_bytes = canon::to_canonical_string(failure.checkpoint().1).unwrap();
        let failure = match failure.try_into_retryable() {
            Ok(_) => panic!("outcome uncertainty cannot become a mutation token"),
            Err(failure) => failure,
        };
        store.replace_read_back_results([Ok(OperationCustodyReadBackOutcome::ExactSuccessor)]);
        let pending = failure
            .reconcile_exact(&mut store)
            .expect("a later readback proves only the retained exact candidate");
        let completed = pending
            .continue_anchor(&mut RecordingAnchor::advancing())
            .expect("the read-back-proved candidate reaches its anchor");
        assert_eq!(
            canon::to_canonical_string(completed.successor().1).unwrap(),
            successor_bytes
        );
        assert_eq!(
            canon::to_canonical_string(completed.checkpoint().1).unwrap(),
            checkpoint_bytes
        );
        assert_eq!(
            (
                evidence_calls.get(),
                observation_signs.get(),
                checkpoint_signs.get(),
                resolver_calls.get()
            ),
            (1, 1, 1, 1)
        );
        assert_eq!(store.mutation_counts(), (1, 3));
    }
}

#[test]
fn definitive_noncommit_retries_only_the_retained_candidate() {
    let mut case = Case::new(uncertain_state("unknown"));
    let recovered = case.take_recovered();
    let checkpoint_signer = checkpoint_authority(&case.checkpoint_authority_source);
    let evidence_calls = Cell::new(0);
    let observation_signs = Cell::new(0);
    let checkpoint_signs = Cell::new(0);
    let resolver_calls = Cell::new(0);
    let mut store = RecordingStore::failing_commit(
        ControlStoreError::new(ControlStoreErrorKind::Storage),
        [Ok(OperationCustodyReadBackOutcome::Predecessor)],
    )
    .with_preparation(
        append_response,
        ControlTransactionSequenceV1::new(3).unwrap(),
    );
    let mut anchor = RecordingAnchor::advancing();
    let failure = append_current_head_reconciliation(
        case.input(recovered, conclusive_state("succeeded"), &checkpoint_signer),
        &CountingResolver(&resolver_calls),
        &mut store,
        &mut anchor,
        |_| {
            evidence_calls.set(evidence_calls.get() + 1);
            Ok(())
        },
        |_, _, _, _| panic!("the fixture does not rotate runtime authority"),
        |_, _, _| {
            observation_signs.set(observation_signs.get() + 1);
            Ok(signature())
        },
        |_, _, _| {
            checkpoint_signs.set(checkpoint_signs.get() + 1);
            Ok(signature())
        },
    )
    .expect_err("definitive storage failure retains a retryable candidate");
    let failure = match failure {
        CurrentHeadReconciliationAppendFailure::Commit(failure) => *failure,
        other => panic!("unexpected noncommit phase: {other:?}"),
    };
    let successor_bytes = canon::to_canonical_string(failure.successor().1).unwrap();
    let checkpoint_bytes = canon::to_canonical_string(failure.checkpoint().1).unwrap();
    let retryable = failure
        .try_into_retryable()
        .unwrap_or_else(|_| panic!("definitive noncommit permits the exact retained candidate"));
    store.replace_read_back_results([Ok(OperationCustodyReadBackOutcome::Predecessor)]);
    store.replace_commit_result(Ok(OperationCustodyCommitOutcome::Committed));
    let mut retry_anchor = RecordingAnchor::advancing();
    let completed = retryable
        .commit(&mut store, &mut retry_anchor)
        .expect("the exact retained candidate commits once after proven noncommit");
    assert_eq!(
        canon::to_canonical_string(completed.successor().1).unwrap(),
        successor_bytes
    );
    assert_eq!(
        canon::to_canonical_string(completed.checkpoint().1).unwrap(),
        checkpoint_bytes
    );
    assert_eq!(
        (
            evidence_calls.get(),
            observation_signs.get(),
            checkpoint_signs.get(),
            resolver_calls.get()
        ),
        (1, 1, 1, 1)
    );
    assert_eq!(store.mutation_counts(), (2, 2));
}

#[test]
fn ambiguous_anchor_recovery_reuses_the_database_committed_successor() {
    for (current, conclusive) in [
        ("unknown", "succeeded"),
        ("expired", "failed"),
        ("recovering", "cancelled"),
    ] {
        let mut case = Case::new(uncertain_state(current));
        let recovered = case.take_recovered();
        let checkpoint_signer = checkpoint_authority(&case.checkpoint_authority_source);
        let evidence_calls = Cell::new(0);
        let observation_signs = Cell::new(0);
        let checkpoint_signs = Cell::new(0);
        let resolver_calls = Cell::new(0);
        let mut store = RecordingStore::committing(OperationCustodyCommitOutcome::Committed)
            .with_preparation(
                append_response,
                ControlTransactionSequenceV1::new(3).unwrap(),
            );
        let mut anchor = RecoveringAmbiguousAnchor {
            observations: 0,
            advances: 0,
        };
        let failure = append_current_head_reconciliation(
            case.input(recovered, conclusive_state(conclusive), &checkpoint_signer),
            &CountingResolver(&resolver_calls),
            &mut store,
            &mut anchor,
            |_| {
                evidence_calls.set(evidence_calls.get() + 1);
                Ok(())
            },
            |_, _, _, _| panic!("the fixture does not rotate runtime authority"),
            |_, _, _| {
                observation_signs.set(observation_signs.get() + 1);
                Ok(signature())
            },
            |_, _, _| {
                checkpoint_signs.set(checkpoint_signs.get() + 1);
                Ok(signature())
            },
        )
        .expect_err("failed post-advance observation retains the anchor phase");
        let pending = match failure {
            CurrentHeadReconciliationAppendFailure::Anchor(pending) => *pending,
            other => panic!("unexpected anchor phase: {other:?}"),
        };
        let successor_bytes = canon::to_canonical_string(pending.successor().1).unwrap();
        let checkpoint_bytes = canon::to_canonical_string(pending.checkpoint().1).unwrap();
        let completed = pending
            .continue_anchor(&mut anchor)
            .expect("later exact-successor observation completes without another advance");
        assert_eq!(
            canon::to_canonical_string(completed.successor().1).unwrap(),
            successor_bytes
        );
        assert_eq!(
            canon::to_canonical_string(completed.checkpoint().1).unwrap(),
            checkpoint_bytes
        );
        assert_eq!((anchor.observations, anchor.advances), (3, 1));
        assert_eq!(
            (
                evidence_calls.get(),
                observation_signs.get(),
                checkpoint_signs.get(),
                resolver_calls.get()
            ),
            (1, 1, 1, 1)
        );
        assert_eq!(store.mutation_counts(), (1, 1));
    }
}

#[test]
fn association_and_evidence_rejections_stop_before_storage_or_signing() {
    for fault in [
        "wrong-handle",
        "wrong-store",
        "wrong-checkpoint-store",
        "wrong-current-checkpoint",
        "wrong-authenticated-head",
        "nonconclusive-evidence",
        "terminal-predecessor",
        "wrong-attempt-evidence",
    ] {
        let mut case = Case::new(uncertain_state("unknown"));
        let mut state = conclusive_state("succeeded");
        if fault == "wrong-handle" {
            case.fixture = checkpoint_preparation::profile_fixture();
            case.fixture.handle_ref = record_ref("operation/wrong-handle");
        } else if fault == "wrong-store" {
            let recovered = case.recovered.as_ref().unwrap();
            case.recovered = Some(RecoveredOperationObservation::fixture_for_handle(
                recovered.handle.clone(),
                (
                    recovered.selected_head().0.clone(),
                    recovered.selected_head().1.clone(),
                ),
                (
                    recovered.selected_head().0.clone(),
                    recovered.selected_head().1.clone(),
                ),
                (
                    recovered.current_checkpoint().0.clone(),
                    recovered.current_checkpoint().1.clone(),
                ),
                AuthorityStoreCoordinate::new(
                    ControlStoreIdV1::parse("other-store").unwrap(),
                    ControlIncarnationIdV1::parse("other-incarnation").unwrap(),
                ),
            ));
        } else if fault == "nonconclusive-evidence" {
            state = OperationObservationStateV1::CancellationRequested {
                request: evidence("cancellation/acknowledgement", '5'),
            };
        } else if fault == "terminal-predecessor" {
            case = Case::new(conclusive_state("failed"));
        } else if fault == "wrong-checkpoint-store" {
            case.checkpoint_authority_source.control_store_id =
                ControlStoreIdV1::parse("other-store").unwrap();
        } else if fault == "wrong-current-checkpoint" {
            let unrelated = CheckpointPair::exact();
            let recovered = case.recovered.as_ref().unwrap();
            case.recovered = Some(RecoveredOperationObservation::fixture_for_handle(
                recovered.handle.clone(),
                (
                    recovered.selected().0.clone(),
                    recovered.selected().1.clone(),
                ),
                (
                    recovered.selected_head().0.clone(),
                    recovered.selected_head().1.clone(),
                ),
                (unrelated.successor_ref, unrelated.successor),
                store_coordinate(),
            ));
        } else if fault == "wrong-authenticated-head" {
            let foreign = OperationFixture::with_handle_identity(
                "foreign-head",
                case.fixture.handle.operation_handle_id().as_str(),
                120,
            );
            assert_ne!(foreign.handle_ref, case.fixture.handle_ref);
            let foreign_head_ref = record_ref("operation/foreign-authenticated-head");
            let foreign_head = issue_observation(
                &case.authority,
                &foreign.handle_ref,
                &foreign.handle,
                case.observation_schema.clone(),
                OperationObservationRevisionV1::new(3).unwrap(),
                Some((&foreign.successor_ref, &foreign.successor)),
                uncertain_state("unknown"),
                70,
            );
            let recovered = case.recovered.as_ref().unwrap();
            case.recovered = Some(RecoveredOperationObservation::fixture_for_handle(
                recovered.handle.clone(),
                (foreign.successor_ref, foreign.successor),
                (foreign_head_ref, foreign_head),
                (
                    recovered.current_checkpoint().0.clone(),
                    recovered.current_checkpoint().1.clone(),
                ),
                store_coordinate(),
            ));
        }
        let signs = Cell::new(0);
        let resolver_calls = Cell::new(0);
        let recovered = case.take_recovered();
        let checkpoint_signer = checkpoint_authority(&case.checkpoint_authority_source);
        let mut store = NoCustodyCalls {
            transaction_preparations: Cell::new(0),
            accumulator_preparations: Cell::new(0),
        };
        let mut anchor = RecordingAnchor::advancing();
        let result = append_current_head_reconciliation(
            case.input(recovered, state, &checkpoint_signer),
            &CountingResolver(&resolver_calls),
            &mut store,
            &mut anchor,
            |_| {
                if fault == "wrong-attempt-evidence" {
                    Err(InvalidResidentRecordV1::ContextualAdmission {
                        record: "reconciliation evidence",
                        invariant: "exact admitted owner evidence",
                    })
                } else {
                    Ok(())
                }
            },
            |_, _, _, _| panic!("no rotation"),
            |_, _, _| {
                signs.set(signs.get() + 1);
                Ok(signature())
            },
            |_, _, _| panic!("checkpoint signing follows accepted evidence"),
        );
        assert!(result.is_err(), "{fault}");
        assert_eq!(store.transaction_preparations.get(), 0, "{fault}");
        assert_eq!(store.accumulator_preparations.get(), 0, "{fault}");
        assert_eq!(anchor.access_counts(), (0, 0), "{fault}");
        assert_eq!(signs.get(), 0, "{fault}");
        assert_eq!(resolver_calls.get(), 0, "{fault}");
    }
}

#[test]
fn remote_cancellation_acknowledgement_cannot_authorize_a_terminal_cancellation() {
    let mut case = Case::new(uncertain_state("unknown"));
    let recovered = case.take_recovered();
    let evidence_calls = Cell::new(0);
    let signs = Cell::new(0);
    let resolver_calls = Cell::new(0);
    let checkpoint_signer = checkpoint_authority(&case.checkpoint_authority_source);
    let mut store = NoCustodyCalls {
        transaction_preparations: Cell::new(0),
        accumulator_preparations: Cell::new(0),
    };
    let mut anchor = RecordingAnchor::advancing();
    let result = append_current_head_reconciliation(
        case.input(
            recovered,
            OperationObservationStateV1::Cancelled {
                diagnostic: evidence("a2a/remote-cancellation-acknowledgement", '6'),
            },
            &checkpoint_signer,
        ),
        &CountingResolver(&resolver_calls),
        &mut store,
        &mut anchor,
        |_| {
            evidence_calls.set(evidence_calls.get() + 1);
            Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "reconciliation evidence",
                invariant: "remote cancellation acknowledgement is not owner cancellation evidence",
            })
        },
        |_, _, _, _| panic!("rejected evidence cannot rotate authority"),
        |_, _, _| {
            signs.set(signs.get() + 1);
            Ok(signature())
        },
        |_, _, _| panic!("rejected evidence cannot reach checkpoint signing"),
    );
    assert!(matches!(
        result,
        Err(CurrentHeadReconciliationAppendFailure::BeforeSuccessor(
            OperationAppendFailure::Operation(OperationAttemptError::Admission(
                InvalidResidentRecordV1::ContextualAdmission { .. }
            ))
        ))
    ));
    assert_eq!(evidence_calls.get(), 1);
    assert_eq!(signs.get(), 0);
    assert_eq!(resolver_calls.get(), 0);
    assert_eq!(store.transaction_preparations.get(), 0);
    assert_eq!(store.accumulator_preparations.get(), 0);
    assert_eq!(anchor.access_counts(), (0, 0));
}

#[test]
fn ambiguous_commit_and_anchor_retain_the_exact_signed_continuation() {
    for fault in ["commit", "anchor"] {
        let mut case = Case::new(uncertain_state("unknown"));
        let signs = Cell::new(0);
        let resolver_calls = Cell::new(0);
        let recovered = case.take_recovered();
        let checkpoint_signer = checkpoint_authority(&case.checkpoint_authority_source);
        let mut store = if fault == "commit" {
            RecordingStore::failing_commit(
                ControlStoreError::new(ControlStoreErrorKind::OutcomeUnknown),
                [
                    Ok(OperationCustodyReadBackOutcome::Predecessor),
                    Err(ControlStoreError::new(ControlStoreErrorKind::Storage)),
                ],
            )
        } else {
            RecordingStore::committing(OperationCustodyCommitOutcome::Committed)
        }
        .with_preparation(
            append_response,
            ControlTransactionSequenceV1::new(3).unwrap(),
        );
        let mut anchor = FailureAnchor {
            ambiguous_advance: fault == "anchor",
            observations: 0,
        };
        let result = append_current_head_reconciliation(
            case.input(recovered, conclusive_state("succeeded"), &checkpoint_signer),
            &CountingResolver(&resolver_calls),
            &mut store,
            &mut anchor,
            |_| Ok(()),
            |_, _, _, _| panic!("no rotation"),
            |_, _, _| {
                signs.set(signs.get() + 1);
                Ok(signature())
            },
            |_, _, _| {
                signs.set(signs.get() + 1);
                Ok(signature())
            },
        );
        let result = match result {
            Ok(_) => panic!("physical uncertainty remains explicit"),
            Err(failure) => failure,
        };
        let successor = result.successor().expect("issued successor is retained");
        assert_eq!(successor.1.revision().get(), 4);
        assert_eq!(signs.get(), 2);
        assert_eq!(resolver_calls.get(), 1);
        match result {
            CurrentHeadReconciliationAppendFailure::Commit(failure) if fault == "commit" => {
                assert_eq!(
                    failure.checkpoint().0,
                    &record_ref("operation/checkpoint-reconciliation")
                );
                let successor = failure.successor().1.clone();
                let mut read_back = RecordingStore::failing_commit(
                    ControlStoreError::new(ControlStoreErrorKind::Storage),
                    [Ok(OperationCustodyReadBackOutcome::ExactSuccessor)],
                );
                let pending = failure
                    .reconcile_exact(&mut read_back)
                    .expect("the original exact candidate can be recovered without reissue");
                let completed = pending
                    .continue_anchor(&mut RecordingAnchor::advancing())
                    .expect("recovered exact database custody can anchor");
                assert_eq!(
                    canon::to_canonical_string(completed.successor().1).unwrap(),
                    canon::to_canonical_string(&successor).unwrap()
                );
                assert_eq!(signs.get(), 2, "recovery does not sign again");
            }
            CurrentHeadReconciliationAppendFailure::Anchor(pending) if fault == "anchor" => {
                assert_eq!(
                    pending.checkpoint().0,
                    &record_ref("operation/checkpoint-reconciliation")
                );
                assert!(
                    pending
                        .continue_anchor(&mut FailureAnchor {
                            ambiguous_advance: true,
                            observations: 2,
                        })
                        .is_err(),
                    "unproved ambiguous anchor outcome remains pending"
                );
                assert_eq!(signs.get(), 2, "uncertainty does not sign again");
            }
            other => panic!("{fault}: unexpected continuation {other:?}"),
        }
    }
}

#[test]
fn remote_cancellation_and_expiry_facts_are_not_local_conclusive_evidence() {
    for state in [
        OperationObservationStateV1::CancellationRequested {
            request: evidence("a2a/remote-cancelled", '6'),
        },
        OperationObservationStateV1::Expired {
            diagnostic: evidence("diagnostic/still-expired", '7'),
        },
        OperationObservationStateV1::OutcomeUnknown {
            diagnostic: evidence("diagnostic/remote-state", '8'),
            reconciliation: ReconciliationFrontierReferenceV1 {
                record_ref: record_ref("reconciliation/remote-state"),
                record_sha256: ReconciliationFrontierDigest::parse(sha('9')).unwrap(),
            },
            effect_receipt: None,
        },
    ] {
        let mut case = Case::new(uncertain_state("expired"));
        let recovered = case.take_recovered();
        let resolver_calls = Cell::new(0);
        let checkpoint_signer = checkpoint_authority(&case.checkpoint_authority_source);
        let mut store = NoCustodyCalls {
            transaction_preparations: Cell::new(0),
            accumulator_preparations: Cell::new(0),
        };
        let mut anchor = RecordingAnchor::advancing();
        let result = append_current_head_reconciliation(
            case.input(recovered, state, &checkpoint_signer),
            &CountingResolver(&resolver_calls),
            &mut store,
            &mut anchor,
            |_| panic!("nonconclusive state never reaches owner evidence admission"),
            |_, _, _, _| panic!("no rotation"),
            |_, _, _| panic!("nonconclusive state is never signed"),
            |_, _, _| panic!("nonconclusive state is never checkpointed"),
        );
        assert!(matches!(
            result,
            Err(CurrentHeadReconciliationAppendFailure::BeforeSuccessor(
                OperationAppendFailure::Operation(OperationAttemptError::Association)
            ))
        ));
        assert_eq!(store.transaction_preparations.get(), 0);
        assert_eq!(store.accumulator_preparations.get(), 0);
        assert_eq!(anchor.access_counts(), (0, 0));
        assert_eq!(resolver_calls.get(), 0);
    }
}

#[test]
fn operation_service_consumes_one_recovered_head_and_commits_owner_admitted_evidence() {
    let mut case = Case::new(uncertain_state("unknown"));
    let recovered = case.take_recovered();
    assert!(
        case.recovered.is_none(),
        "the recovered proof leaves its fixture once"
    );
    let checkpoint_signer = checkpoint_authority(&case.checkpoint_authority_source);
    let resolver_calls = Cell::new(0);
    let evidence_calls = Cell::new(0);
    let observation_signs = Cell::new(0);
    let checkpoint_signs = Cell::new(0);
    let mut store = RecordingStore::committing(OperationCustodyCommitOutcome::Committed)
        .with_preparation(
            append_response,
            ControlTransactionSequenceV1::new(3).unwrap(),
        );
    let mut anchor = RecordingAnchor::advancing();
    let mut authority = ();
    let mut tasks = resident_tasks();
    let mut service =
        ResidentOperationService::new(&mut authority, &mut store, &mut anchor, &mut tasks);

    let completed = service
        .append_current_head_reconciliation(
            case.input(recovered, conclusive_state("succeeded"), &checkpoint_signer),
            &CountingResolver(&resolver_calls),
            |state| {
                evidence_calls.set(evidence_calls.get() + 1);
                assert_eq!(state, &conclusive_state("succeeded"));
                Ok(())
            },
            |_, _, _, _| panic!("the fixture does not rotate runtime authority"),
            |_, _, _| {
                observation_signs.set(observation_signs.get() + 1);
                Ok(signature())
            },
            |_, _, _| {
                checkpoint_signs.set(checkpoint_signs.get() + 1);
                Ok(signature())
            },
        )
        .expect("the selected resident owners retain the conclusive successor");

    assert_eq!(completed.successor().1.revision().get(), 4);
    assert_eq!(
        completed.successor().1.state(),
        &conclusive_state("succeeded")
    );
    assert_eq!(
        (
            evidence_calls.get(),
            observation_signs.get(),
            checkpoint_signs.get()
        ),
        (1, 1, 1)
    );
    assert_eq!(resolver_calls.get(), 1);
    assert_eq!(store.mutation_counts(), (1, 1));
    assert_eq!(anchor.access_counts(), (1, 1));
    let _ = completed.into_custody();
}

#[test]
fn operation_service_rejects_foreign_store_head_and_evidence_before_io() {
    for fault in ["store", "head", "evidence"] {
        let mut case = Case::new(uncertain_state("unknown"));
        if fault == "store" {
            let recovered = case.recovered.as_ref().unwrap();
            case.recovered = Some(RecoveredOperationObservation::fixture_for_handle(
                recovered.handle.clone(),
                (
                    recovered.selected().0.clone(),
                    recovered.selected().1.clone(),
                ),
                (
                    recovered.selected_head().0.clone(),
                    recovered.selected_head().1.clone(),
                ),
                (
                    recovered.current_checkpoint().0.clone(),
                    recovered.current_checkpoint().1.clone(),
                ),
                AuthorityStoreCoordinate::new(
                    ControlStoreIdV1::parse("other-store").unwrap(),
                    ControlIncarnationIdV1::parse("other-incarnation").unwrap(),
                ),
            ));
        } else if fault == "head" {
            let foreign = OperationFixture::with_handle_identity(
                "service-foreign-head",
                case.fixture.handle.operation_handle_id().as_str(),
                120,
            );
            let foreign_head_ref = record_ref("operation/service-foreign-head");
            let foreign_head = issue_observation(
                &case.authority,
                &foreign.handle_ref,
                &foreign.handle,
                case.observation_schema.clone(),
                OperationObservationRevisionV1::new(3).unwrap(),
                Some((&foreign.successor_ref, &foreign.successor)),
                uncertain_state("unknown"),
                70,
            );
            let recovered = case.recovered.as_ref().unwrap();
            case.recovered = Some(RecoveredOperationObservation::fixture_for_handle(
                recovered.handle.clone(),
                (foreign.successor_ref, foreign.successor),
                (foreign_head_ref, foreign_head),
                (
                    recovered.current_checkpoint().0.clone(),
                    recovered.current_checkpoint().1.clone(),
                ),
                store_coordinate(),
            ));
        }

        let recovered = case.take_recovered();
        let checkpoint_signer = checkpoint_authority(&case.checkpoint_authority_source);
        let resolver_calls = Cell::new(0);
        let signs = Cell::new(0);
        let mut store = NoCustodyCalls {
            transaction_preparations: Cell::new(0),
            accumulator_preparations: Cell::new(0),
        };
        let mut anchor = RecordingAnchor::advancing();
        let mut authority = ();
        let mut tasks = resident_tasks();
        let mut service =
            ResidentOperationService::new(&mut authority, &mut store, &mut anchor, &mut tasks);
        let result = service.append_current_head_reconciliation(
            case.input(recovered, conclusive_state("succeeded"), &checkpoint_signer),
            &CountingResolver(&resolver_calls),
            |_| {
                if fault == "evidence" {
                    Err(InvalidResidentRecordV1::ContextualAdmission {
                        record: "reconciliation evidence",
                        invariant: "exact admitted owner evidence",
                    })
                } else {
                    Ok(())
                }
            },
            |_, _, _, _| panic!("rejected association cannot rotate authority"),
            |_, _, _| {
                signs.set(signs.get() + 1);
                Ok(signature())
            },
            |_, _, _| panic!("rejected association cannot sign a checkpoint"),
        );

        assert!(result.is_err(), "{fault}");
        assert_eq!(store.transaction_preparations.get(), 0, "{fault}");
        assert_eq!(store.accumulator_preparations.get(), 0, "{fault}");
        assert_eq!(anchor.access_counts(), (0, 0), "{fault}");
        assert_eq!(signs.get(), 0, "{fault}");
        assert_eq!(resolver_calls.get(), 0, "{fault}");
    }
}

#[test]
fn operation_service_preserves_ambiguous_custody_without_resigning() {
    for fault in ["commit", "anchor"] {
        let mut case = Case::new(uncertain_state("unknown"));
        let recovered = case.take_recovered();
        let checkpoint_signer = checkpoint_authority(&case.checkpoint_authority_source);
        let resolver_calls = Cell::new(0);
        let signs = Cell::new(0);
        let mut store = if fault == "commit" {
            RecordingStore::failing_commit(
                ControlStoreError::new(ControlStoreErrorKind::OutcomeUnknown),
                [
                    Ok(OperationCustodyReadBackOutcome::Predecessor),
                    Err(ControlStoreError::new(ControlStoreErrorKind::Storage)),
                ],
            )
        } else {
            RecordingStore::committing(OperationCustodyCommitOutcome::Committed)
        }
        .with_preparation(
            append_response,
            ControlTransactionSequenceV1::new(3).unwrap(),
        );
        let mut anchor = FailureAnchor {
            ambiguous_advance: fault == "anchor",
            observations: 0,
        };
        let mut authority = ();
        let mut tasks = resident_tasks();
        let mut service =
            ResidentOperationService::new(&mut authority, &mut store, &mut anchor, &mut tasks);
        let failure = match service.append_current_head_reconciliation(
            case.input(recovered, conclusive_state("succeeded"), &checkpoint_signer),
            &CountingResolver(&resolver_calls),
            |_| Ok(()),
            |_, _, _, _| panic!("the fixture does not rotate runtime authority"),
            |_, _, _| {
                signs.set(signs.get() + 1);
                Ok(signature())
            },
            |_, _, _| {
                signs.set(signs.get() + 1);
                Ok(signature())
            },
        ) {
            Ok(_) => panic!("physical uncertainty remains explicit"),
            Err(failure) => failure,
        };
        let successor = failure
            .successor()
            .expect("the exact issued successor crosses the facade");
        assert_eq!(successor.1.revision().get(), 4);
        assert_eq!(signs.get(), 2);
        assert_eq!(resolver_calls.get(), 1);

        match failure {
            CurrentHeadReconciliationAppendFailure::Commit(failure) if fault == "commit" => {
                let successor = failure.successor().1.clone();
                let mut read_back = RecordingStore::failing_commit(
                    ControlStoreError::new(ControlStoreErrorKind::Storage),
                    [Ok(OperationCustodyReadBackOutcome::ExactSuccessor)],
                );
                let pending = failure
                    .reconcile_exact(&mut read_back)
                    .expect("the retained candidate reaches exact database custody");
                let completed = pending
                    .continue_anchor(&mut RecordingAnchor::advancing())
                    .expect("the existing candidate reaches its anchor");
                assert_eq!(completed.successor().1, &successor);
            }
            CurrentHeadReconciliationAppendFailure::Anchor(pending) if fault == "anchor" => {
                assert!(
                    pending
                        .continue_anchor(&mut FailureAnchor {
                            ambiguous_advance: true,
                            observations: 2,
                        })
                        .is_err(),
                    "ambiguous anchor evidence cannot replay the write"
                );
            }
            other => panic!("{fault}: unexpected continuation {other:?}"),
        }
        assert_eq!(signs.get(), 2, "custody recovery never signs again");
    }
}
