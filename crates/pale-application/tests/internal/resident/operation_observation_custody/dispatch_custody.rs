//! Exact dispatch joins using admitted records and controlled storage/anchor ports.
//!
//! These tests prove application sequencing, not native storage or anchor hardware.

use super::*;
use crate::resident::CompletedOperationObservationCustody;
use pale_ports::{
    AntiRollbackAnchorKey, ControlRecordBytes, ControlRecordDigest, OperationCustodyRecordKind,
};

fn dispatched() -> OperationFixture {
    let mut fixture = OperationFixture::exact();
    fixture.successor = issue_observation(
        &signing_authority(),
        &fixture.handle_ref,
        &fixture.handle,
        schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID),
        OperationObservationRevisionV1::new(2).unwrap(),
        Some((&fixture.predecessor_ref, &fixture.predecessor)),
        OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::Dispatched {
                attempt_id: CapabilityAttemptIdV1::parse("attempt-1").unwrap(),
                evidence: evidence("attempt/selection", 'a'),
            },
        },
        31,
    );
    fixture
}

fn member(reference: &str, kind: OperationCustodyRecordKind) -> OperationCustodyRecord {
    // The physical optional role is opaque here; dispatch semantics are tested
    // by the native-action phase rather than asserted by this custody fixture.
    let schema = match kind {
        OperationCustodyRecordKind::AttemptSelection => {
            pale_ports::PRIVATE_OPERATION_ATTEMPT_SELECTION_V1_SCHEMA
        }
        OperationCustodyRecordKind::CapabilityResult => CAPABILITY_RESULT_V1_SCHEMA,
        _ => panic!("optional fixture role"),
    };
    let record = ControlRecord::new(
        ControlRecordCoordinate::new(
            record_ref(reference),
            ControlRecordDigest::parse(pale_core::sha256_hex(b"{}")).unwrap(),
        ),
        SchemaId::parse(schema).unwrap(),
        SchemaDigest::parse(sha('b')).unwrap(),
        ControlRecordBytes::new(b"{}".to_vec()).unwrap(),
    );
    match kind {
        OperationCustodyRecordKind::AttemptSelection => {
            OperationCustodyRecord::AttemptSelection(record)
        }
        OperationCustodyRecordKind::CapabilityResult => {
            OperationCustodyRecord::CapabilityResult(record)
        }
        _ => unreachable!("optional fixture role"),
    }
}

fn append(
    fixture: &OperationFixture,
    associated: Option<OperationCustodyRecord>,
) -> PreparedOperationObservationAppend<'_> {
    let append = prepare_operation_observation_append(fixture.transition(), &ResolveStore).unwrap();
    match associated {
        Some(record) => append.with_associated_record(record),
        None => append,
    }
}

pub(in crate::resident) fn complete(
    fixture: &OperationFixture,
    associated: Option<OperationCustodyRecord>,
    database: OperationCustodyDatabaseEvidence,
) -> CompletedOperationObservationCustody {
    let checkpoints = CheckpointPair::exact();
    let prepared = append(fixture, associated)
        .attach_checkpoint(
            checkpoints.transition(&AcceptCheckpoint),
            checkpoints.physical_batch(),
        )
        .unwrap();
    let mut store = RecordingStore::committing(match database {
        OperationCustodyDatabaseEvidence::ExactReplay => OperationCustodyCommitOutcome::ExactReplay,
        _ => OperationCustodyCommitOutcome::Committed,
    });
    if database == OperationCustodyDatabaseEvidence::RecoveredExactAfterAmbiguousCommit {
        store.commit_result = Some(Err(ControlStoreError::new(
            ControlStoreErrorKind::OutcomeUnknown,
        )));
        store
            .read_back_results
            .push_back(Ok(OperationCustodyReadBackOutcome::ExactSuccessor));
    }
    let pending = commit_observation(prepared, &mut store).unwrap();
    assert_eq!(pending.database_evidence(), database);
    let mut anchor = RecordingAnchor::advancing();
    if database == OperationCustodyDatabaseEvidence::ExactReplay {
        anchor.observation_result = Some(Ok(AntiRollbackAnchorObservation::ExactSuccessor));
    }
    pending.advance_anchor(&mut anchor).unwrap()
}

struct ReadAnchor {
    expected: AntiRollbackAnchorKey,
    result: Option<Result<Option<AntiRollbackAnchor>, AntiRollbackAnchorError>>,
    reads: usize,
}

impl AntiRollbackAnchorPort for ReadAnchor {
    fn read(
        &mut self,
        key: &AntiRollbackAnchorKey,
    ) -> Result<Option<AntiRollbackAnchor>, AntiRollbackAnchorError> {
        assert_eq!(key, &self.expected);
        self.reads += 1;
        self.result.take().expect("at most one exact anchor read")
    }
    fn compare_and_advance(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError> {
        panic!("dispatch guard must not advance an anchor")
    }
    fn observe(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError> {
        panic!("dispatch guard reads the exact retained anchor")
    }
}

fn selected(
    fixture: &OperationFixture,
    completed: &CompletedOperationObservationCustody,
) -> (RecordingStore, ReadAnchor) {
    let candidate = completed.candidate();
    let checkpoint = candidate.checkpoint();
    let mut store = RecordingStore::recovering(observation_recovery_reads(fixture));
    store.checkpoint_reads.push_back(Ok(Some((
        checkpoint.successor().clone(),
        checkpoint.successor_record().clone(),
    ))));
    let head = checkpoint.successor();
    let expected = AntiRollbackAnchor::from(ControlDomainAntiRollbackAnchor::new(
        candidate.store().store_id().clone(),
        candidate.store().incarnation_id().clone(),
        head.checkpoint_domain_id().clone(),
        head.domain_epoch(),
        head.domain_sequence(),
        head.coordinate().clone(),
    ));
    let anchor = ReadAnchor {
        expected: expected.key(),
        result: Some(Ok(Some(expected))),
        reads: 0,
    };
    (store, anchor)
}

#[test]
fn fresh_dispatch_requires_exact_custody_then_four_reads_and_the_exact_anchor() {
    let fixture = dispatched();
    for associated in [
        None,
        Some(member(
            "attempt/selection",
            OperationCustodyRecordKind::AttemptSelection,
        )),
    ] {
        let completed = complete(
            &fixture,
            associated.clone(),
            OperationCustodyDatabaseEvidence::New,
        );
        let (mut store, mut anchor) = selected(&fixture, &completed);
        append(&fixture, associated)
            .verify_fresh_dispatch_custody(&completed, &mut store, &mut anchor, usize::MAX)
            .unwrap();
        assert_eq!(store.index_queries.len(), 3);
        assert_eq!(
            store.checkpoint_queries,
            vec![(
                store_coordinate(),
                completed
                    .candidate()
                    .checkpoint()
                    .successor()
                    .checkpoint_domain_id()
                    .clone(),
                usize::MAX
            )]
        );
        assert_eq!(anchor.reads, 1);
        assert_eq!((store.commits, store.read_backs), (0, 0));
    }
}

#[test]
fn replay_and_recovered_uncertainty_remain_history_without_releasing_dispatch() {
    let fixture = dispatched();
    for database in [
        OperationCustodyDatabaseEvidence::ExactReplay,
        OperationCustodyDatabaseEvidence::RecoveredExactAfterAmbiguousCommit,
    ] {
        let completed = complete(&fixture, None, database);
        let expected = append(&fixture, None);
        expected.verify_completed_custody(&completed).unwrap();
        let (mut store, mut anchor) = selected(&fixture, &completed);
        assert!(matches!(
            expected.verify_fresh_dispatch_custody(&completed, &mut store, &mut anchor, usize::MAX),
            Err(OperationDispatchCustodyError::NotNewCommit)
        ));
        assert!(store.index_queries.is_empty());
        assert!(store.checkpoint_queries.is_empty());
        assert_eq!(anchor.reads, 0);
    }
}

#[test]
fn complete_join_rejects_another_operation_store_primary_mutation_and_optional_record() {
    let fixture = dispatched();
    let associated = member(
        "attempt/selection",
        OperationCustodyRecordKind::AttemptSelection,
    );
    let completed = complete(
        &fixture,
        Some(associated.clone()),
        OperationCustodyDatabaseEvidence::New,
    );
    let other = OperationFixture::named("other");
    assert!(append(&other, Some(associated.clone()))
        .verify_completed_custody(&completed)
        .is_err());
    let mut wrong_store = append(&fixture, Some(associated.clone()));
    wrong_store.store = AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse("other-store").unwrap(),
        ControlIncarnationIdV1::parse("incarnation-1").unwrap(),
    );
    assert!(wrong_store.verify_completed_custody(&completed).is_err());
    let mut wrong_primary = append(&fixture, Some(associated.clone()));
    let OperationCustodyIndexMutation::LatestObservation { expected, .. } =
        &mut wrong_primary.latest_observation_mutation
    else {
        panic!("append's primary mutation");
    };
    *expected = OperationCustodyIndexPredecessor::Exact(
        project_observation(&other.predecessor_ref, &other.predecessor).unwrap(),
    );
    assert!(wrong_primary.verify_completed_custody(&completed).is_err());
    let record = associated.record();
    let changed_bytes = OperationCustodyRecord::AttemptSelection(ControlRecord::new(
        record.coordinate().clone(),
        record.schema_id().clone(),
        record.schema_sha256().clone(),
        ControlRecordBytes::new(b"{\"changed\":true}".to_vec()).unwrap(),
    ));
    for replacement in [
        None,
        Some(changed_bytes),
        Some(member(
            "different-selection",
            OperationCustodyRecordKind::AttemptSelection,
        )),
        Some(member(
            "attempt/selection",
            OperationCustodyRecordKind::CapabilityResult,
        )),
    ] {
        assert!(append(&fixture, replacement)
            .verify_completed_custody(&completed)
            .is_err());
    }
    let without_member = complete(&fixture, None, OperationCustodyDatabaseEvidence::New);
    assert!(append(
        &fixture,
        Some(member(
            "extra-selection",
            OperationCustodyRecordKind::AttemptSelection
        ))
    )
    .verify_completed_custody(&without_member)
    .is_err());
}

#[test]
fn missing_stale_or_oversized_selected_records_stop_before_the_anchor() {
    let fixture = dispatched();
    let completed = complete(&fixture, None, OperationCustodyDatabaseEvidence::New);
    for position in 0..3 {
        let (mut store, mut anchor) = selected(&fixture, &completed);
        let query = store.index_reads[position].query().clone();
        store.index_reads[position] = pale_ports::OperationCustodyIndexObservation::absent(query);
        assert!(append(&fixture, None)
            .verify_fresh_dispatch_custody(&completed, &mut store, &mut anchor, usize::MAX)
            .is_err());
        assert!(store.checkpoint_queries.is_empty());
        assert_eq!(anchor.reads, 0);
    }
    let (mut store, mut anchor) = selected(&fixture, &completed);
    assert!(matches!(
        append(&fixture, None).verify_fresh_dispatch_custody(
            &completed,
            &mut store,
            &mut anchor,
            1
        ),
        Err(OperationDispatchCustodyError::Recovery(
            OperationRecoveryError::RecordBoundExceeded
        ))
    ));
    assert_eq!(anchor.reads, 0);
    let (mut store, mut anchor) = selected(&fixture, &completed);
    let stale = project_observation(&fixture.predecessor_ref, &fixture.predecessor).unwrap();
    store.index_reads[1] = pale_ports::OperationCustodyIndexObservation::exact(
        store.index_reads[1].query().clone(),
        stale.clone(),
    )
    .unwrap();
    store.index_reads[2] = pale_ports::OperationCustodyIndexObservation::exact(
        OperationCustodyIndexQuery::ObservationCheckpoint(
            OperationCustodyIndexKey::for_observation(stale.coordinate()),
        ),
        completed
            .candidate()
            .checkpoint()
            .successor_record()
            .clone(),
    )
    .unwrap();
    assert!(matches!(
        append(&fixture, None).verify_fresh_dispatch_custody(
            &completed,
            &mut store,
            &mut anchor,
            usize::MAX
        ),
        Err(OperationDispatchCustodyError::HeadChanged)
    ));
    assert_eq!(anchor.reads, 0);
}

#[test]
fn historical_observation_and_anchor_do_not_hide_a_changed_database_head() {
    let fixture = dispatched();
    let completed = complete(&fixture, None, OperationCustodyDatabaseEvidence::New);
    let old_record = completed
        .candidate()
        .checkpoint()
        .predecessor_record()
        .unwrap();
    let checkpoints = CheckpointPair::exact();
    let next_preimage = checkpoint_preimage(
        3,
        Some((
            checkpoints.successor_ref.clone(),
            checkpoints
                .successor
                .canonical_control_checkpoint_sha256()
                .clone(),
        )),
    );
    let next = admit_successor_checkpoint(
        &next_preimage,
        &checkpoints.successor_ref,
        &checkpoints.successor,
    );
    let next_record =
        project_checkpoint_record(&record_ref("operation/checkpoint-3"), &next).unwrap();
    let next_head = ControlCheckpointHead::new(
        next.checkpoint_domain_id().clone(),
        *next.domain_epoch(),
        *next.domain_sequence(),
        *next.committed_transaction_sequence(),
        next_record.coordinate().clone(),
    );
    for result in [
        Ok(None),
        Ok(Some((
            completed.candidate().checkpoint().successor().clone(),
            old_record.clone(),
        ))),
        Ok(Some((next_head, next_record))),
        Err(ControlStoreError::new(ControlStoreErrorKind::Storage)),
    ] {
        let (mut store, mut anchor) = selected(&fixture, &completed);
        store.checkpoint_reads = VecDeque::from([result]);
        let failure = append(&fixture, None)
            .verify_fresh_dispatch_custody(&completed, &mut store, &mut anchor, usize::MAX)
            .unwrap_err();
        assert!(matches!(
            failure,
            OperationDispatchCustodyError::HeadChanged | OperationDispatchCustodyError::Store(_)
        ));
        assert_eq!(store.index_queries.len(), 3);
        assert_eq!(store.checkpoint_queries.len(), 1);
        assert_eq!(anchor.reads, 0);
    }
}

#[test]
fn missing_conflicting_or_later_anchors_never_release_the_selected_dispatch() {
    let fixture = dispatched();
    let completed = complete(&fixture, None, OperationCustodyDatabaseEvidence::New);
    for result in [
        Ok(None),
        Ok(Some(operation_anchor(1, "old", 'a'))),
        Ok(Some(operation_anchor(2, "different", 'b'))),
        Ok(Some(operation_anchor(3, "later", 'c'))),
        Err(AntiRollbackAnchorError::new(
            AntiRollbackAnchorErrorKind::Unavailable,
        )),
    ] {
        let (mut store, mut anchor) = selected(&fixture, &completed);
        anchor.result = Some(result);
        let failure = append(&fixture, None)
            .verify_fresh_dispatch_custody(&completed, &mut store, &mut anchor, usize::MAX)
            .unwrap_err();
        assert!(matches!(
            failure,
            OperationDispatchCustodyError::AnchorChanged | OperationDispatchCustodyError::Anchor(_)
        ));
        assert_eq!(anchor.reads, 1);
        assert_eq!((store.commits, store.read_backs), (0, 0));
    }
    let preparation = OperationFixture::exact();
    let completed = complete(&preparation, None, OperationCustodyDatabaseEvidence::New);
    let (mut store, mut anchor) = selected(&preparation, &completed);
    append(&preparation, None)
        .verify_completed_custody(&completed)
        .unwrap();
    assert!(matches!(
        append(&preparation, None).verify_fresh_dispatch_custody(
            &completed,
            &mut store,
            &mut anchor,
            usize::MAX
        ),
        Err(OperationDispatchCustodyError::Association)
    ));
    assert!(store.index_queries.is_empty());
    assert_eq!(anchor.reads, 0);
}
