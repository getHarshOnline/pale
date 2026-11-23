//! Recover a frozen Consult stop from real `SQLite` after losing the pending phase.
//! Historical semantic admission, signatures and the anchor are fixture models.

use super::*;
use crate::resident_fixture::local_operation_start_fixture::RecoveryParents;
use pale_application::resident::consultation_recovery_test_support::recover_collection_fixture;

#[test]
fn consult_expiry_recovers_its_frozen_collection_after_anchor_interruption() {
    run_native_action_journeys(&[Journey::ConsultExpiry]);
}

#[test]
fn consult_expiry_with_retained_intent_recovers_its_established_stop() {
    run_native_action_journeys(&[Journey::ConsultExpiryWithIntent]);
}

struct InterruptedAnchor<'a>(&'a mut IndependentAnchor);

impl AntiRollbackAnchorPort for InterruptedAnchor<'_> {
    fn read(
        &mut self,
        key: &AntiRollbackAnchorKey,
    ) -> Result<Option<AntiRollbackAnchor>, AntiRollbackAnchorError> {
        self.0.read(key)
    }

    fn observe(
        &mut self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError> {
        self.0.observe(transition)
    }

    fn compare_and_advance(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError> {
        Err(AntiRollbackAnchorError::new(
            AntiRollbackAnchorErrorKind::Unavailable,
        ))
    }
}

fn evidence(name: &str) -> ImmutableRecordReferenceV1 {
    ImmutableRecordReferenceV1 {
        record_ref: record_ref(name),
        record_sha256: ArtifactDigest::hash_bytes(name.as_bytes()),
    }
}

fn identity(number: u64) -> OperationCheckpointInput {
    OperationCheckpointInput {
        reference: record_ref(&format!("consult/checkpoint-{number}")),
        id: ControlCheckpointIdV1::parse(format!("consult-checkpoint-{number}")).unwrap(),
    }
}

pub(super) fn collection_admission(handle: &OperationHandleV1) -> ConsultationCollectionAdmission {
    ConsultationCollectionAdmission {
        completion_policy: evidence("consult/completion-policy"),
        opened_at: UnixTimeV1::new(62).unwrap(),
        deadline: *handle.expires_at(),
        slots: vec![ConsultationRosterSlot::new(
            ConsultationSlotId::parse("existing-assessment").unwrap(),
            ConsultationRequirement::Required,
            evidence("consult/required"),
            ParticipantRefV1::Application {
                application_id: "fixture-assessor".into(),
            },
            None,
            None,
            ConsultationContributionSource::EvidenceResolved {
                evidence: evidence("consult/assessment"),
            },
        )],
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn run(
    operation: &IssuedLocalOperation,
    signing: &SigningAuthorityBindingV1,
    initial: (&ResidentRecordRefV1, &OperationObservationV1),
    checkpoint: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    fields: &ControlStateCheckpointPreimageV1,
    file: &std::path::Path,
    port: &mut SqliteOperationCustody<'_>,
    anchor: &mut IndependentAnchor,
    request: &CommonRequestEnvelopeV1,
    parents: &RecoveryParents<'_>,
    generation: super::super::local_recovery::GenerationInputs,
    sessions: &DeploymentControlSqliteStore,
    retained_intent: bool,
) {
    let (handle_ref, handle) = operation.handle();
    let coordinate = port.coordinate.clone();
    let resolve = |_: &OperationObservationV1| Ok(coordinate.clone());
    let limits = ConsultationCollectionLimits::new(4, 1 << 20, 1 << 20).unwrap();
    let bounds = AccumulatorPreparationLimits::new(4, 10_000, 10_000, 4096, 100_000).unwrap();
    let schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
    let authority = checkpoint_authority(fields);
    let collection = ConsultationCollection::admit_initial(
        handle_ref,
        handle,
        collection_admission(handle),
        &[],
        limits,
    )
    .unwrap();
    let prepared = collection
        .prepare_observation(
            operation,
            checkpoint,
            initial,
            ConsultationCollectionObservationInput {
                observation_ref: record_ref("consult/collecting"),
                predecessor_collection: None,
                observed_at: UnixTimeV1::new(62).unwrap(),
                signing: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &schema,
                    signing_authority: signing,
                },
            },
            limits,
            &resolve,
            port,
            |_| Ok(()),
            |_, _, _, _| panic!("fixture has no authority rotation"),
            |_, _, _| Ok(signature()),
        )
        .unwrap();
    let collecting = commit_consultation_collection(
        prepared,
        ConsultationCollectionCommitInput {
            accumulator_limits: bounds,
            predecessor: checkpoint,
            identity: identity(2),
            issued_at: UnixTimeV1::new(63).unwrap(),
            schema: &fields.schema_sha256,
            authority: &authority,
        },
        &resolve,
        port,
        anchor,
        |_, _, _| Ok(signature()),
    )
    .unwrap();
    // This historical collection is independently read before its successor exists.
    let previous = recover_collection_fixture(
        operation,
        collecting.observation().1,
        limits,
        &resolve,
        port,
    )
    .unwrap()
    .unwrap();
    let stop_time = *handle.expires_at();
    let reason = evidence("consult/expired");
    let expected = previous
        .stop(
            ConsultationStopCause::OperationExpiry,
            reason.clone(),
            stop_time,
            limits,
            |_, _| Ok(()),
        )
        .unwrap();
    let failed = commit_recovered_consultation_stop(
        &previous,
        ConsultationStopInput {
            cause: ConsultationStopCause::OperationExpiry,
            reason,
            stopped_at: stop_time,
            parent: operation,
            parent_observation: collecting.observation(),
            parent_checkpoint: collecting.checkpoint(),
            observation_ref: record_ref("consult/stopped"),
            observed_at: stop_time,
            signing: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &schema,
                signing_authority: signing,
            },
            limits,
        },
        ConsultationSuccessorCommitInput {
            accumulator_limits: bounds,
            identity: identity(3),
            issued_at: stop_time,
            schema: &fields.schema_sha256,
            authority: &authority,
        },
        &resolve,
        port,
        &mut InterruptedAnchor(anchor),
        |_, summary| {
            assert!(summary.requirements_met());
            Ok(())
        },
        |_| Ok(()),
        |_, _, _, _| panic!("fixture has no authority rotation"),
        |_, _, _| Ok(signature()),
        |_, _, _| Ok(signature()),
    )
    .expect_err("database commits before the anchor interruption");
    let (stopped_observation, stopped_checkpoint) = match failed {
        ConsultationStopFailure::Custody(ConsultationCollectionCommitFailure::Anchor {
            prepared,
            checkpoint,
            failure,
        }) => {
            let observation = (
                prepared.observation().0.clone(),
                prepared.observation().1.clone(),
            );
            drop(prepared);
            drop(failure);
            (observation, *checkpoint)
        }
        other => panic!("unexpected failure phase: {other:?}"),
    };
    // Only independently admitted historical inputs remain. There is no live
    // pending candidate, signer or policy callback in the reconstruction call.
    let mut reopened = connection(file);
    reopened.execute_batch("PRAGMA query_only = ON").unwrap();
    let mut cold = SqliteOperationCustody {
        connection: &mut reopened,
        coordinate: coordinate.clone(),
    };
    let changes = cold.connection.total_changes();
    let read_history = |custody: &mut dyn OperationCustodyPort, observation, limits| {
        ConsultationCollection::recover_historical(
            operation.handle(),
            observation,
            limits,
            &resolve,
            custody,
            |_| Ok(()),
        )
    };
    let prior_snapshot = previous.snapshot().clone();
    drop(previous);
    let previous = read_history(&mut cold, collecting.observation().1, limits).unwrap();
    let stopped = read_history(&mut cold, &stopped_observation.1, limits).unwrap();
    let through_deployment =
        read_history(&mut &*sessions, collecting.observation().1, limits).unwrap();
    assert_eq!(through_deployment.snapshot(), previous.snapshot());
    assert!(read_history(
        &mut &*sessions,
        collecting.observation().1,
        ConsultationCollectionLimits::new(4, 1, 1).unwrap(),
    )
    .is_err());
    assert_eq!(previous.snapshot().frontier(), prior_snapshot.frontier());
    assert_eq!(previous.snapshot().head(), stopped.snapshot().head());
    assert_ne!(
        previous.snapshot().head().current_frontier(),
        previous.snapshot().frontier().coordinate(),
        "historical reading preserves the actual current head"
    );
    assert!(
        recover_collection_fixture(
            operation,
            collecting.observation().1,
            limits,
            &resolve,
            &mut cold,
        )
        .is_err(),
        "a historical collection is not the current collection"
    );
    assert!(read_history(&mut cold, initial.1, limits).is_err());
    assert!(read_history(
        &mut cold,
        collecting.observation().1,
        ConsultationCollectionLimits::new(4, 1, 1).unwrap(),
    )
    .is_err());
    let mut admission_called = false;
    assert!(ConsultationCollection::recover_historical(
        operation.handle(),
        collecting.observation().1,
        limits,
        &resolve,
        &mut cold,
        |_| {
            admission_called = true;
            Err(InvalidResidentRecordV1::InvalidIdentity {
                kind: "historical policy",
            })
        },
    )
    .is_err());
    assert!(
        admission_called,
        "physical custody cannot bypass historical policy admission"
    );
    assert_eq!(stopped.collection(), &expected);
    assert_eq!(
        stopped.collection().stopped().unwrap().cause(),
        ConsultationStopCause::OperationExpiry
    );
    assert!(
        recover_committed_consultation_expiry(
            &previous,
            &stopped,
            OperationObservationTransition::new(
                handle_ref,
                handle,
                initial.0,
                initial.1,
                &stopped_observation.0,
                &stopped_observation.1,
            ),
            CommittedObservationCheckpointRecovery {
                selection: operation.checkpoint_selection(),
                predecessor: collecting.checkpoint(),
                successor: (&stopped_checkpoint.0, &stopped_checkpoint.1),
                limits: bounds,
            },
            limits,
            &resolve,
            &mut cold,
        )
        .is_err(),
        "a different observation cannot stand for the collection predecessor"
    );
    let transition = OperationObservationTransition::new(
        handle_ref,
        handle,
        collecting.observation().0,
        collecting.observation().1,
        &stopped_observation.0,
        &stopped_observation.1,
    );
    let recover = |custody: &mut dyn OperationCustodyPort, proof_limits, collection_limits| {
        recover_committed_consultation_expiry(
            &previous,
            &stopped,
            transition,
            CommittedObservationCheckpointRecovery {
                selection: operation.checkpoint_selection(),
                predecessor: collecting.checkpoint(),
                successor: (&stopped_checkpoint.0, &stopped_checkpoint.1),
                limits: proof_limits,
            },
            collection_limits,
            &resolve,
            custody,
        )
    };
    assert!(recover(
        &mut cold,
        bounds,
        ConsultationCollectionLimits::new(4, 1, 1).unwrap()
    )
    .is_err());
    assert!(recover(
        &mut cold,
        AccumulatorPreparationLimits::new(1, 1, 1, 1, 1).unwrap(),
        limits
    )
    .is_err());
    let pending = recover(&mut cold, bounds, limits)
        .expect("exact collection journal is restored before physical read-back");
    let mut absent_anchor = IndependentAnchor::default();
    assert!(pending.advance_anchor(&mut absent_anchor).is_err());
    assert_eq!(absent_anchor.advance_count(), 0);
    let advances = anchor.advance_count();
    let input = history::Input {
        operation,
        coordinate: &coordinate,
        selected_member: (&stopped_observation.0, &stopped_observation.1),
        known: &[
            initial,
            collecting.observation(),
            (&stopped_observation.0, &stopped_observation.1),
        ],
        checkpoint: fields,
        signing,
    };
    let mut process =
        service::hook_response::expiry::restart_process(sessions, &coordinate, signing, generation);
    let historical_calls = std::cell::Cell::new(0);
    let mut admit_historical = |collection: &ConsultationCollection| {
        historical_calls.set(historical_calls.get() + 1);
        assert!(collection == previous.collection() || collection == stopped.collection());
        Ok(())
    };
    let failed = history::with_sweep_admission(&input, request, parents, |context, admission| {
        process.cold_consult_sweep_with_history(
            (context, bounds),
            0,
            &mut cold,
            sessions,
            anchor,
            admission,
            None,
            None,
        )
    });
    assert!(
        failed.is_err(),
        "missing historical collection authority must refuse repair"
    );
    let mut reject_historical = |_: &ConsultationCollection| {
        Err(InvalidResidentRecordV1::InvalidIdentity {
            kind: "historical policy",
        })
    };
    let failed = history::with_sweep_admission(&input, request, parents, |context, admission| {
        process.cold_consult_sweep_with_history(
            (context, bounds),
            0,
            &mut cold,
            sessions,
            anchor,
            admission,
            Some(&mut reject_historical),
            None,
        )
    });
    assert!(failed.is_err());
    assert_eq!(anchor.advance_count(), advances);
    let continuation =
        history::with_sweep_admission(&input, request, parents, |context, admission| {
            process.cold_consult_sweep_with_history(
                (context, bounds),
                0,
                &mut cold,
                sessions,
                anchor,
                admission,
                Some(&mut admit_historical),
                retained_intent.then_some((&stopped, stop_time)),
            )
        });
    if retained_intent {
        assert_eq!(continuation.unwrap(), OperationClassV1::Consult);
    } else {
        // The negative fixture has a collection but no start plan. The real
        // sweep repairs its anchor and its separate start owner refuses.
        assert_eq!(
            continuation.unwrap_err(),
            "consult start selection: ConsultationParentAuthority"
        );
    }
    assert_eq!(anchor.advance_count(), advances + 1);
    assert_eq!(historical_calls.get(), 2);
    if !retained_intent {
        let replay =
            history::with_sweep_admission(&input, request, parents, |context, admission| {
                process.cold_consult_sweep_with_history(
                    (context, bounds),
                    0,
                    &mut cold,
                    sessions,
                    anchor,
                    admission,
                    Some(&mut admit_historical),
                    retained_intent.then_some((&stopped, stop_time)),
                )
            });
        assert_eq!(
            replay.unwrap_err(),
            "consult start selection: ConsultationParentAuthority"
        );
        assert_eq!(anchor.advance_count(), advances + 1);
        assert_eq!(
            historical_calls.get(),
            2,
            "an already anchored stop needs no new historical admission"
        );
    }

    let completed = recover(&mut cold, bounds, limits)
        .unwrap()
        .advance_anchor(anchor)
        .unwrap();
    assert_eq!(anchor.advance_count(), advances + 1);
    assert_eq!(
        completed
            .candidate()
            .checkpoint()
            .successor_record()
            .coordinate()
            .record_ref(),
        &stopped_checkpoint.0
    );
    recover(&mut cold, bounds, limits)
        .unwrap()
        .advance_anchor(anchor)
        .unwrap();
    assert_eq!(
        anchor.advance_count(),
        advances + 1,
        "repeat recovery does not advance again"
    );
    assert_eq!(
        cold.connection.total_changes(),
        changes,
        "recovery writes no SQLite data"
    );
    assert_eq!(
        recover_collection_fixture(
            operation,
            &stopped_observation.1,
            limits,
            &resolve,
            &mut cold
        )
        .unwrap()
        .unwrap()
        .collection(),
        &expected
    );
    for fault in [
        "DELETE FROM pale_deployment_collection_journal WHERE journal_sequence=1",
        "DELETE FROM pale_deployment_operation_index WHERE index_role='collection_head'",
    ] {
        cold.connection
            .execute_batch(
                "PRAGMA query_only = OFF; PRAGMA foreign_keys = OFF; SAVEPOINT corrupt_history;",
            )
            .unwrap();
        assert!(cold.connection.execute(fault, []).unwrap() > 0);
        cold.connection
            .execute_batch("PRAGMA query_only = ON")
            .unwrap();
        let changes = cold.connection.total_changes();
        assert!(read_history(&mut cold, collecting.observation().1, limits).is_err());
        assert_eq!(cold.connection.total_changes(), changes);
        cold.connection.execute_batch("PRAGMA query_only = OFF; ROLLBACK TO corrupt_history; RELEASE corrupt_history; PRAGMA foreign_keys = ON; PRAGMA query_only = ON;").unwrap();
    }
    // Deliberately corrupt the fixture database below its normal SQL protections.
    for fault in [
        "DELETE FROM pale_deployment_collection_journal WHERE journal_sequence=2",
        "DELETE FROM pale_deployment_control_record WHERE record_ref='consult/stopped'",
        "DELETE FROM pale_deployment_operation_index WHERE index_role='collection_head'",
        "DELETE FROM pale_deployment_control_record WHERE record_ref='consult/checkpoint-3'",
    ] {
        cold.connection
            .execute_batch(
                "PRAGMA query_only = OFF; PRAGMA foreign_keys = OFF; SAVEPOINT corrupt_collection;",
            )
            .unwrap();
        assert!(
            cold.connection.execute(fault, []).unwrap() > 0,
            "fault must alter stored evidence: {fault}"
        );
        cold.connection
            .execute_batch("PRAGMA query_only = ON")
            .unwrap();
        let changes = cold.connection.total_changes();
        assert!(
            recover(&mut cold, bounds, limits).is_err(),
            "missing custody must refuse: {fault}"
        );
        assert_eq!(cold.connection.total_changes(), changes);
        assert_eq!(anchor.advance_count(), advances + 1);
        cold.connection.execute_batch("PRAGMA query_only = OFF; ROLLBACK TO corrupt_collection; RELEASE corrupt_collection; PRAGMA foreign_keys = ON; PRAGMA query_only = ON;").unwrap();
    }
    // An intact physical frame with different content is still not this stop.
    cold.connection
        .execute_batch(
            "PRAGMA query_only = OFF; PRAGMA foreign_keys = OFF; SAVEPOINT corrupt_collection;",
        )
        .unwrap();
    let changed = crate::frame::encode(
        b"changed collection payload",
        AuthorityByteClass::JournalEntry,
    )
    .unwrap();
    assert_eq!(cold.connection.execute("UPDATE pale_deployment_collection_journal SET physical_frame=?1 WHERE journal_sequence=2", [changed]).unwrap(), 1);
    cold.connection
        .execute_batch("PRAGMA query_only = ON")
        .unwrap();
    assert!(recover_collection_fixture(
        operation,
        &stopped_observation.1,
        limits,
        &resolve,
        &mut cold
    )
    .is_err());
    assert!(recover(&mut cold, bounds, limits).is_err());
    cold.connection.execute_batch("PRAGMA query_only = OFF; ROLLBACK TO corrupt_collection; RELEASE corrupt_collection; PRAGMA foreign_keys = ON; PRAGMA query_only = ON;").unwrap();
}
