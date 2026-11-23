//! Real tree and `SQLite` mechanics over the existing synthetic record fixture.
//! These tests do not establish signing, semantic profile admission or activation.

use super::*;
use pale_core::records::{ArtifactDigest, CheckpointProfileDigest, ControlTransactionSequenceV1};
use pale_ports::{
    AccumulatorImplementation, AccumulatorInitialDomain, AccumulatorLeafChange,
    AccumulatorPreparationBasis, AccumulatorPreparationLimits, ControlTransactionPreparation,
    OperationAccumulatorBatch, OperationAccumulatorCheckpoint, OperationAccumulatorPreparation,
    OperationAccumulatorPreparationError, PrincipalProducerIncarnationCandidateError,
    StateLeafChanges, UnresolvedCustodyLeafChanges,
};

fn digest(byte: char) -> ArtifactDigest {
    ArtifactDigest::parse(sha(byte)).unwrap()
}

fn limits() -> AccumulatorPreparationLimits {
    AccumulatorPreparationLimits::new(4, 4096, 4096, 4096, 65536).unwrap()
}

fn request(
    basis: AccumulatorPreparationBasis,
    transaction: ControlTransactionPreparation,
    before: Option<char>,
    after: char,
    limits: AccumulatorPreparationLimits,
) -> OperationAccumulatorPreparation {
    OperationAccumulatorPreparation::new(
        basis,
        StateLeafChanges::new(
            vec![
                AccumulatorLeafChange::new(digest('a'), before.map(digest), Some(digest(after)))
                    .unwrap(),
            ],
            limits.maximum_leaf_changes(),
        )
        .unwrap(),
        UnresolvedCustodyLeafChanges::new(Vec::new(), limits.maximum_leaf_changes()).unwrap(),
        limits,
        AccumulatorImplementation::SparseMerkleTree061Sha256PaleV1,
        transaction,
    )
    .unwrap()
}

fn frontier(batch: &OperationAccumulatorBatch) -> OperationAccumulatorCheckpoint {
    let binding = batch.binding();
    OperationAccumulatorCheckpoint::new(
        binding.store().clone(),
        binding.profile_ref().clone(),
        binding.profile_sha256().clone(),
        binding.successor().clone(),
        batch.state().successor_root().clone(),
        batch.unresolved().successor_root().clone(),
    )
}

/// Install a real initial tree beside the test's synthetic predecessor records.
fn seed(connection: &Connection) -> OperationAccumulatorCheckpoint {
    let head = checkpoint(1);
    let store = crate::principal_trust::store_coordinate(&binding());
    connection
        .execute("DELETE FROM pale_principal_control_checkpoint_head", [])
        .unwrap();
    let initial = request(
        AccumulatorPreparationBasis::Initial(AccumulatorInitialDomain::new(
            store.clone(),
            ResidentRecordRefV1::parse("producer-profile-1").unwrap(),
            CheckpointProfileDigest::parse(sha('e')).unwrap(),
            head.checkpoint_domain_id().clone(),
            head.domain_epoch(),
        )),
        ControlTransactionPreparation::new(store, ControlTransactionSequenceV1::new(20).unwrap())
            .unwrap(),
        None,
        'b',
        limits(),
    );
    let batch = crate::accumulator::preparation::prepare(
        connection,
        AccumulatorStore::PrincipalTrust,
        &initial,
    )
    .unwrap()
    .bind_successor(head.clone())
    .unwrap();
    crate::accumulator::apply(connection, AccumulatorStore::PrincipalTrust, &batch).unwrap();
    connection.execute(
        "INSERT INTO pale_principal_control_checkpoint_head \
         (checkpoint_domain_id,domain_epoch,domain_sequence,control_checkpoint_ref,control_checkpoint_sha256) \
         VALUES (?1,1,1,?2,?3)",
        params![head.checkpoint_domain_id().as_str(),head.coordinate().record_ref().as_str(),head.coordinate().record_sha256().as_str()],
    ).unwrap();
    frontier(&batch)
}

fn prepare(
    connection: &mut Connection,
    prior: OperationAccumulatorCheckpoint,
    candidate: &PreparedPrincipalProducerIncarnationTransition,
    before: char,
    after: char,
) -> OperationAccumulatorBatch {
    let transaction = crate::store::transaction::prepare(connection, prior.store()).unwrap();
    let request = request(
        AccumulatorPreparationBasis::Successor(prior),
        transaction,
        Some(before),
        after,
        limits(),
    );
    let writes = connection.total_changes();
    let prepared = SqlitePrincipalProducerIncarnationCustody { connection }
        .prepare_accumulator(&request)
        .unwrap();
    assert_eq!(prepared.request(), &request);
    assert_eq!(
        connection.total_changes(),
        writes,
        "preparation does not write"
    );
    prepared
        .bind_successor(candidate.next_checkpoint().clone())
        .unwrap()
}

#[test]
fn allocation_retirement_and_successor_commit_trees_and_replay_after_reopen() {
    let directory = crate::test_support::private_tempdir();
    let path = directory.path().join("producer-trees.sqlite3");
    let last;
    {
        let mut connection = initialize_connection(Connection::open(&path).unwrap());
        let mut prior = seed(&connection);
        let mut retained = None;
        for (candidate, before, after) in [
            (initial_candidate(), 'b', 'c'),
            (retirement_candidate(), 'c', 'd'),
            (successor_candidate(), 'd', 'e'),
        ] {
            let batch = prepare(&mut connection, prior, &candidate, before, after);
            prior = frontier(&batch);
            let candidate = candidate.with_accumulator(batch).unwrap();
            assert_eq!(
                commit(&mut connection, &candidate),
                PrincipalProducerIncarnationCommitDisposition::Committed
            );
            assert!(crate::accumulator::matches(
                &connection,
                AccumulatorStore::PrincipalTrust,
                candidate.accumulator().unwrap(),
                true
            )
            .unwrap());
            let writes = connection.total_changes();
            assert_eq!(
                commit(&mut connection, &candidate),
                PrincipalProducerIncarnationCommitDisposition::ExactReplay
            );
            assert_eq!(connection.total_changes(), writes);
            retained = Some(candidate);
        }
        last = retained.unwrap();
    }
    let mut connection = Connection::open(path).unwrap();
    let writes = connection.total_changes();
    assert_eq!(
        commit(&mut connection, &last),
        PrincipalProducerIncarnationCommitDisposition::ExactReplay
    );
    assert_eq!(connection.total_changes(), writes);
}

#[test]
fn attachment_rejects_unlike_checkpoints_stores_and_repeated_batches() {
    let mut connection = initialize_store();
    let prior = seed(&connection);
    let candidate = initial_candidate();
    let batch = prepare(&mut connection, prior, &candidate, 'b', 'c');
    let retained = candidate.clone().with_accumulator(batch.clone()).unwrap();
    assert_eq!(
        retained.with_accumulator(batch.clone()).unwrap_err(),
        PrincipalProducerIncarnationCandidateError::AccumulatorClosure
    );
    assert_eq!(
        retirement_candidate()
            .with_accumulator(batch.clone())
            .unwrap_err(),
        PrincipalProducerIncarnationCandidateError::AccumulatorClosure
    );
    let old = batch.binding();
    let other_store = pale_ports::AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse("other-store").unwrap(),
        old.store().incarnation_id().clone(),
    );
    let binding = pale_ports::AccumulatorCheckpointBinding::new(
        other_store.clone(),
        old.profile_ref().clone(),
        old.profile_sha256().clone(),
        old.phase().clone(),
        old.successor().clone(),
        ControlTransactionPreparation::new(other_store, old.transaction().expected()).unwrap(),
    )
    .unwrap();
    let unlike =
        OperationAccumulatorBatch::new(binding, batch.state().clone(), batch.unresolved().clone())
            .unwrap();
    assert_eq!(
        candidate.with_accumulator(unlike).unwrap_err(),
        PrincipalProducerIncarnationCandidateError::AccumulatorClosure
    );
}

#[test]
fn existing_tree_cannot_be_bypassed_by_omitting_its_batch() {
    let mut connection = initialize_store();
    seed(&connection);
    let candidate = initial_candidate();
    let writes = connection.total_changes();
    let mut custody = SqlitePrincipalProducerIncarnationCustody {
        connection: &mut connection,
    };
    assert_eq!(
        custody.observe_exact(&candidate).unwrap_err().kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
    assert_eq!(
        custody.compare_and_commit(&candidate).unwrap_err().kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
    assert_eq!(connection.total_changes(), writes);
}

#[test]
fn tree_and_head_write_failures_roll_back_the_complete_transition() {
    for trigger in [
        "CREATE TRIGGER fail BEFORE UPDATE ON pale_principal_control_accumulator_position BEGIN SELECT RAISE(ABORT,'node failure'); END;",
        "CREATE TRIGGER fail BEFORE UPDATE ON pale_principal_control_accumulator_frontier BEGIN SELECT RAISE(ABORT,'root failure'); END;",
        "CREATE TRIGGER fail BEFORE UPDATE ON pale_principal_control_checkpoint_head BEGIN SELECT RAISE(ABORT,'head failure'); END;",
    ] {
        let mut connection = initialize_store();
        let prior = seed(&connection);
        let candidate = initial_candidate();
        let batch = prepare(&mut connection, prior, &candidate, 'b', 'c');
        assert_injected_failure_rolls_back(connection, &candidate.with_accumulator(batch).unwrap(), trigger);
    }
}

#[test]
fn damaged_committed_tree_never_counts_as_exact_replay() {
    let mut connection = initialize_store();
    let prior = seed(&connection);
    let candidate = initial_candidate();
    let batch = prepare(&mut connection, prior, &candidate, 'b', 'c');
    let candidate = candidate.with_accumulator(batch).unwrap();
    commit(&mut connection, &candidate);
    connection
        .execute(
            "DELETE FROM pale_principal_control_accumulator_position WHERE position_kind='leaf'",
            [],
        )
        .unwrap();
    let writes = connection.total_changes();
    let mut custody = SqlitePrincipalProducerIncarnationCustody {
        connection: &mut connection,
    };
    assert_eq!(
        custody.observe_exact(&candidate).unwrap(),
        PrincipalProducerIncarnationDatabaseObservation::ConflictingOrPartial
    );
    assert_eq!(
        custody.compare_and_commit(&candidate).unwrap_err().kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
    assert_eq!(
        connection.total_changes(),
        writes,
        "no repair or replay write"
    );
}

#[test]
fn preparation_rejects_unlike_transactions_wrong_values_and_exhausted_work() {
    let mut connection = initialize_store();
    let prior = seed(&connection);
    let transaction = crate::store::transaction::prepare(&connection, prior.store()).unwrap();
    let unlike_transaction = ControlTransactionPreparation::new(
        prior.store().clone(),
        ControlTransactionSequenceV1::new(22).unwrap(),
    )
    .unwrap();
    let requests = [
        request(
            AccumulatorPreparationBasis::Successor(prior.clone()),
            unlike_transaction,
            Some('b'),
            'c',
            limits(),
        ),
        request(
            AccumulatorPreparationBasis::Successor(prior.clone()),
            transaction.clone(),
            Some('d'),
            'c',
            limits(),
        ),
        request(
            AccumulatorPreparationBasis::Successor(prior),
            transaction,
            Some('b'),
            'c',
            AccumulatorPreparationLimits::new(1, 1, 1, 1, 1).unwrap(),
        ),
    ];
    let writes = connection.total_changes();
    for (index, request) in requests.iter().enumerate() {
        let error = SqlitePrincipalProducerIncarnationCustody {
            connection: &mut connection,
        }
        .prepare_accumulator(request)
        .unwrap_err();
        if index == 2 {
            assert!(matches!(error, OperationAccumulatorPreparationError::Limit));
        }
        assert_eq!(connection.total_changes(), writes);
    }
}

#[test]
fn preparation_does_not_substitute_store_profile_or_checkpoint_identity() {
    let mut connection = initialize_store();
    let prior = seed(&connection);
    let other_store = pale_ports::AuthorityStoreCoordinate::new(
        prior.store().store_id().clone(),
        ControlIncarnationIdV1::parse("other-incarnation").unwrap(),
    );
    let other_head = ControlCheckpointHead::new(
        prior.checkpoint().checkpoint_domain_id().clone(),
        prior.checkpoint().domain_epoch(),
        prior.checkpoint().domain_sequence(),
        prior.checkpoint().committed_transaction_sequence(),
        coordinate("other-checkpoint", 'f'),
    );
    let variants = [
        (
            other_store,
            prior.profile_ref().clone(),
            prior.checkpoint().clone(),
        ),
        (
            prior.store().clone(),
            ResidentRecordRefV1::parse("other-profile").unwrap(),
            prior.checkpoint().clone(),
        ),
        (
            prior.store().clone(),
            prior.profile_ref().clone(),
            other_head,
        ),
    ];
    let writes = connection.total_changes();
    for (store, profile_ref, checkpoint) in variants {
        let transaction = ControlTransactionPreparation::new(
            store.clone(),
            ControlTransactionSequenceV1::new(21).unwrap(),
        )
        .unwrap();
        let altered = OperationAccumulatorCheckpoint::new(
            store,
            profile_ref,
            prior.profile_sha256().clone(),
            checkpoint,
            prior.state_root().clone(),
            prior.unresolved_root().clone(),
        );
        let request = request(
            AccumulatorPreparationBasis::Successor(altered),
            transaction,
            Some('b'),
            'c',
            limits(),
        );
        assert!(SqlitePrincipalProducerIncarnationCustody {
            connection: &mut connection
        }
        .prepare_accumulator(&request)
        .is_err());
        assert_eq!(connection.total_changes(), writes);
    }
}

#[test]
fn intervening_store_transaction_invalidates_the_prepared_commit_without_writes() {
    let mut connection = initialize_store();
    let prior = seed(&connection);
    let candidate = initial_candidate();
    let batch = prepare(&mut connection, prior, &candidate, 'b', 'c');
    let candidate = candidate.with_accumulator(batch).unwrap();
    let store = crate::principal_trust::store_coordinate(candidate.binding());
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .unwrap();
    let prepared = crate::store::transaction::prepare(&transaction, &store).unwrap();
    crate::store::transaction::advance(&transaction, &prepared).unwrap();
    transaction.commit().unwrap();
    let writes = connection.total_changes();
    let error = SqlitePrincipalProducerIncarnationCustody {
        connection: &mut connection,
    }
    .compare_and_commit(&candidate)
    .unwrap_err();
    assert_eq!(error.kind(), ControlStoreErrorKind::CandidateConflict);
    assert!(crate::accumulator::matches(
        &connection,
        AccumulatorStore::PrincipalTrust,
        candidate.accumulator().unwrap(),
        false
    )
    .unwrap());
    assert_eq!(connection.total_changes(), writes);
}
