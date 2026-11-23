//! Real trie/SQLite preparation; opaque checkpoint fixtures are not admission.

use super::*;
use crate::operation::{
    tests::{checkpoint, initialize_store, start_candidate, store},
    SqliteOperationCustody,
};
use pale_core::records::{CheckpointProfileDigest, ResidentRecordRefV1};
use pale_ports::{
    AccumulatorPreparationLimits, OperationAccumulatorCheckpoint, OperationCustodyPort,
    StateLeafChanges, UnresolvedCustodyLeafChanges,
};
use sparse_merkle_tree::default_store::DefaultStore;

#[path = "committed_recovery.rs"]
mod committed_recovery;

type Tree = SparseMerkleTree<CheckpointHasher, H256, DefaultStore<H256>>;

fn key(byte: u8) -> H256 {
    let mut key = [0; 32];
    key[31] = byte;
    key.into()
}
fn value(byte: u8) -> H256 {
    let bytes: [u8; 32] = Sha256::digest([byte]).into();
    bytes.into()
}
fn tree(leaves: &[(H256, H256)]) -> Tree {
    let mut tree = Tree::default();
    tree.update_all(leaves.to_vec()).unwrap();
    tree
}
fn limits() -> AccumulatorPreparationLimits {
    AccumulatorPreparationLimits::new(32, 100_000, 100_000, 20_000, 1_000_000).unwrap()
}
fn change(key: H256, before: Option<H256>, after: Option<H256>) -> AccumulatorLeafChange {
    AccumulatorLeafChange::new(
        root_digest(key).unwrap(),
        before.map(|hash| root_digest(hash).unwrap()),
        after.map(|hash| root_digest(hash).unwrap()),
    )
    .unwrap()
}
fn prior(state: &Tree, unresolved: &Tree) -> OperationAccumulatorCheckpoint {
    OperationAccumulatorCheckpoint::new(
        store(),
        ResidentRecordRefV1::parse("checkpoint-profile").unwrap(),
        CheckpointProfileDigest::parse("b".repeat(64)).unwrap(),
        checkpoint(8, "checkpoint-8", '8'),
        root_digest(*state.root()).unwrap(),
        root_digest(*unresolved.root()).unwrap(),
    )
}
fn request(
    prior: OperationAccumulatorCheckpoint,
    state: Vec<AccumulatorLeafChange>,
    unresolved: Vec<AccumulatorLeafChange>,
    limits: AccumulatorPreparationLimits,
) -> OperationAccumulatorPreparation {
    OperationAccumulatorPreparation::new(
        pale_ports::AccumulatorPreparationBasis::Successor(prior),
        StateLeafChanges::new(state, limits.maximum_leaf_changes()).unwrap(),
        UnresolvedCustodyLeafChanges::new(unresolved, limits.maximum_leaf_changes()).unwrap(),
        limits,
        pale_ports::AccumulatorImplementation::SparseMerkleTree061Sha256PaleV1,
        pale_ports::ControlTransactionPreparation::new(
            store(),
            pale_core::records::ControlTransactionSequenceV1::new(28).unwrap(),
        )
        .unwrap(),
    )
    .unwrap()
}

fn seed_tree(connection: &Connection, name: &str, tree: &Tree) {
    let rows = tree
        .store()
        .branches_map()
        .iter()
        .map(|(key, node)| (branch_position(key), codec::encode_branch(node)))
        .chain(tree.store().leaves_map().iter().map(|(key, value)| {
            (
                AccumulatorPosition::Leaf((*key).into()),
                value.as_slice().to_vec(),
            )
        }));
    for (position, bytes) in rows {
        let (kind, key) = position_parts(position);
        connection.execute("INSERT INTO pale_deployment_operation_accumulator_position \
            (checkpoint_domain_id,domain_epoch,checkpoint_profile_ref,checkpoint_profile_sha256,tree_kind,position_kind,position_key,node_bytes) \
            VALUES('operation:principal-1',4,'checkpoint-profile',?1,?2,?3,?4,?5)",
            params!["b".repeat(64), name, kind, key.as_slice(), bytes]).unwrap();
    }
}
fn seed(connection: &Connection, state: &Tree, unresolved: &Tree) {
    initialize_store(connection);
    seed_tree(connection, "state", state);
    seed_tree(connection, "unresolved", unresolved);
    connection.execute("UPDATE pale_deployment_operation_accumulator_frontier SET state_root_sha256=?1,unresolved_root_sha256=?2",
        params![root_digest(*state.root()).unwrap().as_str(), root_digest(*unresolved.root()).unwrap().as_str()]).unwrap();
}
fn prepare_public(
    connection: &mut Connection,
    request: &OperationAccumulatorPreparation,
) -> Result<PreparedOperationAccumulator, OperationAccumulatorPreparationError> {
    SqliteOperationCustody {
        connection,
        coordinate: store(),
    }
    .prepare_accumulator(request)
}

fn membership_request(
    state: &Tree,
    unresolved: &Tree,
    reads: usize,
    proof_bytes: usize,
) -> pale_ports::OperationAccumulatorMembership {
    pale_ports::OperationAccumulatorMembership::new(
        prior(state, unresolved),
        vec![
            pale_ports::AccumulatorMembershipLeaf::new(
                root_digest(key(0)).unwrap(),
                root_digest(value(1)).unwrap(),
            )
            .unwrap(),
            pale_ports::AccumulatorMembershipLeaf::new(
                root_digest(key(128)).unwrap(),
                root_digest(value(2)).unwrap(),
            )
            .unwrap(),
        ],
        pale_ports::AccumulatorMembershipLimits::new(2, reads, proof_bytes).unwrap(),
        pale_ports::AccumulatorImplementation::SparseMerkleTree061Sha256PaleV1,
    )
    .unwrap()
}

#[test]
fn current_membership_reopens_and_verifies_actual_proof_without_mutations() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("membership.sqlite");
    let state = tree(&[(key(0), value(1)), (key(128), value(2))]);
    let unresolved = tree(&[(key(64), value(3))]);
    {
        let connection = Connection::open(&path).unwrap();
        connection
            .pragma_update(None, "journal_mode", "WAL")
            .unwrap();
        connection
            .pragma_update(None, "synchronous", "FULL")
            .unwrap();
        seed(&connection, &state, &unresolved);
    }
    let mut connection = Connection::open(&path).unwrap();
    let before = connection.total_changes();
    let request = membership_request(&state, &unresolved, 100_000, 1_000_000);
    let mut port = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    let (head, record) = port
        .read_checkpoint_head(
            &store(),
            request.checkpoint().checkpoint().checkpoint_domain_id(),
            100_000,
        )
        .unwrap()
        .unwrap();
    assert_eq!(&head, request.checkpoint().checkpoint());
    assert_eq!(record.coordinate(), head.coordinate());
    let report = port.verify_accumulator_membership(&request).unwrap();
    assert_eq!(report.request(), &request);
    assert!(matches!(
        port.verify_accumulator_membership(&membership_request(&state, &unresolved, 1, 1_000_000)),
        Err(OperationAccumulatorPreparationError::Limit)
    ));
    assert!(matches!(
        port.verify_accumulator_membership(&membership_request(&state, &unresolved, 100_000, 1)),
        Err(OperationAccumulatorPreparationError::Limit)
    ));
    assert_eq!(connection.total_changes(), before);
}

#[test]
fn membership_exact_trie_read_and_conservative_proof_limits_are_enforced() {
    let state = tree(&[(key(0), value(1)), (key(128), value(2))]);
    let unresolved = tree(&[]);
    let mut connection = Connection::open_in_memory().unwrap();
    seed(&connection, &state, &unresolved);
    let before = connection.total_changes();
    let mut accepted = |reads, proof| {
        let request = membership_request(&state, &unresolved, reads, proof);
        match (SqliteOperationCustody {
            connection: &mut connection,
            coordinate: store(),
        })
        .verify_accumulator_membership(&request)
        {
            Ok(report) => {
                assert_eq!(report.request(), &request);
                true
            }
            Err(OperationAccumulatorPreparationError::Limit) => false,
            Err(error) => panic!("unexpected proof fixture failure: {error:?}"),
        }
    };
    // Find the exact boundary of this actual implementation, not a guessed
    // constant. The proof limit includes its conservative precompile ceiling.
    let mut minimum = |read_axis| {
        let (mut low, mut high) = (1usize, 100_000usize);
        while low < high {
            let middle = low + (high - low) / 2;
            let pass = if read_axis {
                accepted(middle, 100_000)
            } else {
                accepted(100_000, middle)
            };
            if pass {
                high = middle
            } else {
                low = middle + 1
            }
        }
        low
    };
    let reads = minimum(true);
    let proof = minimum(false);
    assert!(reads > 1 && proof > 1);
    assert!(accepted(reads, proof));
    assert!(!accepted(reads - 1, proof));
    assert!(!accepted(reads, proof - 1));
    assert_eq!(connection.total_changes(), before);
}

#[test]
fn current_head_reads_distinguish_whole_domain_absence_partial_state_and_body_bounds() {
    let state = tree(&[(key(0), value(1)), (key(128), value(2))]);
    let unresolved = tree(&[]);
    let mut connection = Connection::open_in_memory().unwrap();
    seed(&connection, &state, &unresolved);
    let selected = prior(&state, &unresolved);
    let absent =
        pale_core::records::ControlCheckpointDomainIdV1::parse("operation:missing").unwrap();
    let before = connection.total_changes();
    let mut port = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    assert!(port
        .read_checkpoint_head(&store(), &absent, 100_000)
        .unwrap()
        .is_none());
    let (_, record) = port
        .read_checkpoint_head(
            &store(),
            selected.checkpoint().checkpoint_domain_id(),
            100_000,
        )
        .unwrap()
        .unwrap();
    let bytes = record.canonical_record_jcs().as_slice().len();
    assert!(bytes > 1);
    assert!(port
        .read_checkpoint_head(
            &store(),
            selected.checkpoint().checkpoint_domain_id(),
            bytes
        )
        .unwrap()
        .is_some());
    assert_eq!(
        port.read_checkpoint_head(
            &store(),
            selected.checkpoint().checkpoint_domain_id(),
            bytes - 1
        )
        .unwrap_err()
        .kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
    assert_eq!(connection.total_changes(), before);
    connection
        .execute("DELETE FROM pale_deployment_operation_checkpoint_head", [])
        .unwrap();
    let before = connection.total_changes();
    let mut port = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    assert!(
        matches!(port.read_checkpoint_head(&store(),selected.checkpoint().checkpoint_domain_id(),100_000),Err(error) if error.kind()==ControlStoreErrorKind::RecoveryRequired)
    );
    assert_eq!(connection.total_changes(), before);
}

#[test]
fn membership_rejects_corrupt_missing_foreign_leaves_and_current_head_races() {
    let state = tree(&[(key(0), value(1)), (key(128), value(2))]);
    let unresolved = tree(&[]);
    for corruption in [
        "DELETE FROM pale_deployment_operation_accumulator_position WHERE position_kind='leaf'",
        "UPDATE pale_deployment_operation_accumulator_position SET node_bytes=zeroblob(133) WHERE position_kind='leaf'",
        "UPDATE pale_deployment_operation_accumulator_frontier SET state_root_sha256=printf('%064d',0)",
        "UPDATE pale_deployment_operation_accumulator_frontier SET checkpoint_profile_ref='different-profile'",
        "DELETE FROM pale_deployment_operation_accumulator_frontier",
        "DELETE FROM pale_deployment_operation_checkpoint_head",
    ] {
        let mut connection = Connection::open_in_memory().unwrap();
        seed(&connection, &state, &unresolved);
        let request = membership_request(&state, &unresolved, 100_000, 1_000_000);
        // Inject one otherwise schema-forbidden malformed storage row, then
        // restore CHECK enforcement before invoking the production decoder.
        let malformed_bytes = corruption.contains("zeroblob(133)");
        if malformed_bytes {
            connection
                .pragma_update(None, "ignore_check_constraints", true)
                .unwrap();
        }
        let before_injection = connection.total_changes();
        connection.execute_batch(corruption).unwrap();
        assert!(
            connection.total_changes() > before_injection,
            "{corruption}: no row changed"
        );
        if malformed_bytes {
            connection
                .pragma_update(None, "ignore_check_constraints", false)
                .unwrap();
        }
        assert_eq!(
            connection
                .pragma_query_value(None, "ignore_check_constraints", |row| row.get::<_, i64>(0))
                .unwrap(),
            0
        );
        let before = connection.total_changes();
        let mut port = SqliteOperationCustody {
            connection: &mut connection,
            coordinate: store()
        };
        assert!(
            matches!(port.verify_accumulator_membership(&request),
                Err(OperationAccumulatorPreparationError::Store(error))
                    if error.kind() == ControlStoreErrorKind::RecoveryRequired),
            "{corruption}"
        );
        assert_eq!(connection.total_changes(), before);
    }
    let mut connection = Connection::open_in_memory().unwrap();
    seed(&connection, &state, &unresolved);
    let old = membership_request(&state, &unresolved, 100_000, 1_000_000);
    let wrong = pale_ports::OperationAccumulatorMembership::new(
        old.checkpoint().clone(),
        vec![pale_ports::AccumulatorMembershipLeaf::new(
            root_digest(key(0)).unwrap(),
            root_digest(value(9)).unwrap(),
        )
        .unwrap()],
        old.limits(),
        old.implementation(),
    )
    .unwrap();
    let mut port = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    assert!(matches!(port.verify_accumulator_membership(&wrong),
        Err(OperationAccumulatorPreparationError::Store(error)) if error.kind() == ControlStoreErrorKind::RecoveryRequired));
    // A real successor commit changes the selected head; old mutable positions
    // are never presented as historical proof material.
    let prepared = prepare_public(
        &mut connection,
        &request(
            prior(&state, &unresolved),
            vec![change(key(0), Some(value(1)), Some(value(4)))],
            vec![],
            limits(),
        ),
    )
    .unwrap();
    let template = start_candidate("membership", ['1', '2', '3', '9']);
    let batch = prepared
        .bind_successor(template.checkpoint().successor().clone())
        .unwrap();
    let indexes = template
        .indexes()
        .iter()
        .filter(|index| {
            !matches!(
                index,
                pale_ports::OperationCustodyIndexMutation::RequestForHandle { .. }
                    | pale_ports::OperationCustodyIndexMutation::StartObservation { .. }
                    | pale_ports::OperationCustodyIndexMutation::ObservationCheckpoint { .. }
            )
        })
        .cloned()
        .collect();
    let candidate = pale_ports::OperationCustodyCandidate::new_start(
        store(),
        template.kind(),
        template.records().to_vec(),
        indexes,
        template.checkpoint().clone(),
        batch,
        template.session_basis().unwrap().clone(),
    )
    .unwrap();
    let mut port = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    port.compare_and_commit(&candidate).unwrap();
    assert!(
        matches!(port.verify_accumulator_membership(&old), Err(OperationAccumulatorPreparationError::Store(error)) if error.kind()==ControlStoreErrorKind::CandidateConflict)
    );
}

fn initial_request(domain: &str, epoch: u64, replacement: u8) -> OperationAccumulatorPreparation {
    OperationAccumulatorPreparation::new(
        pale_ports::AccumulatorPreparationBasis::Initial(
            pale_ports::AccumulatorInitialDomain::new(
                store(),
                ResidentRecordRefV1::parse("checkpoint-profile").unwrap(),
                CheckpointProfileDigest::parse("b".repeat(64)).unwrap(),
                pale_core::records::ControlCheckpointDomainIdV1::parse(domain).unwrap(),
                pale_core::records::ControlDomainEpochV1::new(epoch).unwrap(),
            ),
        ),
        StateLeafChanges::new(
            vec![
                change(key(0), None, Some(value(replacement))),
                change(key(64), None, Some(value(2))),
                change(key(128), None, Some(value(3))),
            ],
            3,
        )
        .unwrap(),
        UnresolvedCustodyLeafChanges::new(vec![], 3).unwrap(),
        limits(),
        pale_ports::AccumulatorImplementation::SparseMerkleTree061Sha256PaleV1,
        pale_ports::ControlTransactionPreparation::new(
            store(),
            pale_core::records::ControlTransactionSequenceV1::new(28).unwrap(),
        )
        .unwrap(),
    )
    .unwrap()
}

fn empty_initial_request(domain: &str, epoch: u64) -> OperationAccumulatorPreparation {
    OperationAccumulatorPreparation::new(
        pale_ports::AccumulatorPreparationBasis::Initial(
            pale_ports::AccumulatorInitialDomain::new(
                store(),
                ResidentRecordRefV1::parse("checkpoint-profile").unwrap(),
                CheckpointProfileDigest::parse("b".repeat(64)).unwrap(),
                pale_core::records::ControlCheckpointDomainIdV1::parse(domain).unwrap(),
                pale_core::records::ControlDomainEpochV1::new(epoch).unwrap(),
            ),
        ),
        StateLeafChanges::new(Vec::new(), 1).unwrap(),
        UnresolvedCustodyLeafChanges::new(Vec::new(), 1).unwrap(),
        limits(),
        pale_ports::AccumulatorImplementation::SparseMerkleTree061Sha256PaleV1,
        pale_ports::ControlTransactionPreparation::new(
            store(),
            pale_core::records::ControlTransactionSequenceV1::new(28).unwrap(),
        )
        .unwrap(),
    )
    .unwrap()
}

// Reuses only the existing opaque record-role fixture. This checks real native
// construction and atomic physical custody, not application record admission.
fn initial_candidate(
    connection: &mut Connection,
    tag: &str,
    bytes: [char; 4],
    replacement: u8,
) -> pale_ports::OperationCustodyCandidate {
    let request = initial_request("operation:fresh", 1, replacement);
    let prepared = prepare_public(connection, &request).unwrap();
    assert_eq!(
        prepared.state_root(),
        &root_digest(
            *tree(&[
                (key(0), value(replacement)),
                (key(64), value(2)),
                (key(128), value(3)),
            ])
            .root()
        )
        .unwrap()
    );
    assert_eq!(
        prepared.unresolved_root(),
        &root_digest(H256::zero()).unwrap()
    );
    let template = start_candidate(tag, bytes);
    let head = pale_ports::ControlCheckpointHead::new(
        request.basis().domain().clone(),
        request.basis().epoch(),
        pale_core::records::ControlDomainSequenceV1::new(1).unwrap(),
        request.transaction().successor(),
        template.checkpoint().successor().coordinate().clone(),
    );
    let batch = prepared.bind_successor(head.clone()).unwrap();
    let indexes = template
        .indexes()
        .iter()
        .filter(|index| {
            !matches!(
                index,
                pale_ports::OperationCustodyIndexMutation::RequestForHandle { .. }
                    | pale_ports::OperationCustodyIndexMutation::StartObservation { .. }
                    | pale_ports::OperationCustodyIndexMutation::ObservationCheckpoint { .. }
            )
        })
        .cloned()
        .collect();
    pale_ports::OperationCustodyCandidate::new_start(
        store(),
        template.kind(),
        template.records().to_vec(),
        indexes,
        pale_ports::OperationCustodyCheckpoint::new(
            pale_ports::OperationCheckpointPhase::Initial,
            head,
            template.checkpoint().successor_record().clone(),
        )
        .unwrap(),
        batch,
        template.session_basis().unwrap().clone(),
    )
    .unwrap()
}

#[test]
fn native_preparation_retains_the_exact_request_not_only_its_absent_domain() {
    let mut connection = Connection::open_in_memory().unwrap();
    initialize_store(&connection);
    let before = connection.total_changes();
    let original = initial_request("operation:fresh", 1, 1);
    let expected = prepare_public(&mut connection, &original).unwrap();
    assert_eq!(expected.request(), &original);
    let different_leaf = initial_request("operation:fresh", 1, 4);
    let prepared = prepare_public(&mut connection, &different_leaf).unwrap();
    assert_eq!(prepared.basis(), expected.basis());
    assert_eq!(prepared.request(), &different_leaf);
    assert_ne!(prepared.request(), &original);
    assert_ne!(prepared.state_root(), expected.state_root());
    let different_limit = OperationAccumulatorPreparation::new(
        original.basis().clone(),
        original.state().clone(),
        original.unresolved().clone(),
        AccumulatorPreparationLimits::new(33, 100_000, 100_000, 20_000, 1_000_000).unwrap(),
        original.implementation(),
        original.transaction().clone(),
    )
    .unwrap();
    let prepared = prepare_public(&mut connection, &different_limit).unwrap();
    assert_eq!(prepared.basis(), expected.basis());
    assert_eq!(prepared.state_root(), expected.state_root());
    assert_eq!(prepared.request(), &different_limit);
    assert_ne!(
        prepared.request(),
        &original,
        "equal computed roots do not erase raw limits"
    );
    assert_eq!(connection.total_changes(), before);
    assert!(connection.is_autocommit());
}

#[test]
fn empty_initial_preparation_preserves_the_canonical_empty_trees() {
    let mut connection = Connection::open_in_memory().unwrap();
    initialize_store(&connection);
    let before = connection.total_changes();
    let request = empty_initial_request("operation:empty", 1);

    let prepared = prepare_public(&mut connection, &request).unwrap();
    let zero = root_digest(H256::zero()).unwrap();
    assert_eq!(prepared.request(), &request);
    assert_eq!(prepared.state_root(), &zero);
    assert_eq!(prepared.unresolved_root(), &zero);
    assert_eq!(
        request.state().changes().len() + request.unresolved().changes().len(),
        0
    );
    assert!(request.state().changes().is_empty());
    assert!(request.unresolved().changes().is_empty());
    assert_eq!(connection.total_changes(), before);
    assert!(connection.is_autocommit());
}

#[test]
fn initial_domain_commits_real_insertions_and_exact_replay_after_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("initial.sqlite");
    let mut connection = Connection::open(&path).unwrap();
    connection
        .pragma_update(None, "journal_mode", "WAL")
        .unwrap();
    connection
        .pragma_update(None, "synchronous", "FULL")
        .unwrap();
    initialize_store(&connection); // An existing independent domain remains admitted.
    let before = connection.total_changes();
    let winner = initial_candidate(&mut connection, "initial-one", ['1', '2', '3', '9'], 1);
    let competitor = initial_candidate(&mut connection, "initial-two", ['4', '5', '6', 'a'], 4);
    assert_eq!(
        connection.total_changes(),
        before,
        "preparation writes nothing"
    );
    let mut port = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    assert_eq!(
        port.compare_and_commit(&winner).unwrap(),
        pale_ports::OperationCustodyCommitOutcome::Committed
    );
    assert_eq!(
        port.compare_and_commit(&competitor).unwrap_err().kind(),
        ControlStoreErrorKind::CandidateConflict
    );
    assert_eq!(
        port.observe_exact(&winner).unwrap(),
        pale_ports::OperationCustodyReadBackOutcome::ExactSuccessor
    );
    drop(connection);
    let mut connection = Connection::open(&path).unwrap();
    connection
        .pragma_update(None, "foreign_keys", true)
        .unwrap();
    let mut port = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    assert_eq!(
        port.observe_exact(&winner).unwrap(),
        pale_ports::OperationCustodyReadBackOutcome::ExactSuccessor
    );
    assert_eq!(
        port.compare_and_commit(&winner).unwrap(),
        pale_ports::OperationCustodyCommitOutcome::ExactReplay
    );
    let mut statement = connection.prepare("SELECT position_key,node_bytes FROM pale_deployment_operation_accumulator_position WHERE checkpoint_domain_id='operation:fresh' AND tree_kind='state' AND position_kind='leaf'").unwrap();
    let leaves = statement
        .query_map([], |row| {
            let ValueRef::Blob(position) = row.get_ref(0)? else {
                panic!("position type")
            };
            let ValueRef::Blob(value) = row.get_ref(1)? else {
                panic!("value type")
            };
            assert_eq!(position.len(), 33);
            assert_eq!(value.len(), 32);
            Ok((
                codec::hash(&position[1..]).unwrap(),
                codec::hash(value).unwrap(),
            ))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(leaves.len(), 3);
    assert_eq!(
        root_digest(*tree(&leaves).root()).unwrap(),
        *winner.accumulator().state().successor_root()
    );
    drop(statement);
    connection.execute("DELETE FROM pale_deployment_operation_accumulator_position WHERE checkpoint_domain_id='operation:fresh' AND tree_kind='state' AND position_kind='leaf'", []).unwrap();
    let mut port = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    assert_eq!(
        port.observe_exact(&winner).unwrap(),
        pale_ports::OperationCustodyReadBackOutcome::ConflictingOrPartial
    );
    assert_eq!(
        port.compare_and_commit(&winner).unwrap_err().kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
}

#[test]
fn initial_domain_cannot_hide_existing_evidence_in_another_partition() {
    for table in ["head", "frontier", "node"] {
        let mut connection = Connection::open_in_memory().unwrap();
        initialize_store(&connection);
        match table {
            "head" => {
                connection.execute("INSERT INTO pale_deployment_operation_checkpoint_head SELECT 'operation:fresh',domain_epoch,domain_sequence,control_checkpoint_ref,control_checkpoint_sha256 FROM pale_deployment_operation_checkpoint_head WHERE checkpoint_domain_id='operation:principal-1'", []).unwrap();
            }
            "frontier" => {
                connection.execute("INSERT INTO pale_deployment_operation_accumulator_frontier SELECT 'operation:fresh',domain_epoch,checkpoint_profile_ref,checkpoint_profile_sha256,control_checkpoint_ref,control_checkpoint_sha256,state_root_sha256,unresolved_root_sha256 FROM pale_deployment_operation_accumulator_frontier WHERE checkpoint_domain_id='operation:principal-1'", []).unwrap();
            }
            "node" => {
                connection.execute("INSERT INTO pale_deployment_operation_accumulator_position (checkpoint_domain_id,domain_epoch,checkpoint_profile_ref,checkpoint_profile_sha256,tree_kind,position_kind,position_key,node_bytes) VALUES('operation:fresh',99,'other-profile',?1,'state','leaf',?2,?3)", params!["a".repeat(64), vec![0_u8; 33], vec![1_u8; 32]]).unwrap();
            }
            _ => unreachable!(),
        }
        let before = connection.total_changes();
        assert!(
            matches!(
                prepare_public(&mut connection, &initial_request("operation:fresh", 1, 1)),
                Err(OperationAccumulatorPreparationError::Store(_))
            ),
            "existing {table} is not an empty domain"
        );
        assert_eq!(connection.total_changes(), before);
        assert!(connection.is_autocommit());
    }
}

#[test]
fn initial_domain_failure_rolls_back_records_nodes_frontier_and_head_across_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("initial-rollback.sqlite");
    let mut connection = Connection::open(&path).unwrap();
    initialize_store(&connection);
    let candidate = initial_candidate(&mut connection, "rollback", ['1', '2', '3', '9'], 1);
    connection.execute_batch("CREATE TEMP TRIGGER reject_initial_head BEFORE INSERT ON pale_deployment_operation_checkpoint_head WHEN NEW.checkpoint_domain_id='operation:fresh' BEGIN SELECT RAISE(ABORT,'injected initial head failure'); END;").unwrap();
    let mut port = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    assert!(port.compare_and_commit(&candidate).is_err());
    assert!(connection.is_autocommit());
    drop(connection);
    let mut connection = Connection::open(&path).unwrap();
    assert!(domain_is_absent(
        &connection,
        Store::DeploymentOperation,
        candidate.checkpoint().successor().checkpoint_domain_id()
    )
    .unwrap());
    let mut port = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    assert_eq!(
        port.observe_exact(&candidate).unwrap(),
        pale_ports::OperationCustodyReadBackOutcome::Predecessor
    );
    assert_eq!(
        port.compare_and_commit(&candidate).unwrap(),
        pale_ports::OperationCustodyCommitOutcome::Committed
    );
}

#[test]
fn read_snapshot_prepares_both_trees_without_sql_writes_and_releases_snapshot() {
    let state = tree(&[(key(0), value(1)), (key(128), value(2))]);
    let unresolved = tree(&[(key(64), value(3))]);
    let mut connection = Connection::open_in_memory().unwrap();
    seed(&connection, &state, &unresolved);
    let request = request(
        prior(&state, &unresolved),
        vec![
            change(key(0), Some(value(1)), Some(value(4))),
            change(key(64), None, Some(value(5))),
            change(key(128), Some(value(2)), None),
        ],
        vec![change(key(64), Some(value(3)), None)],
        limits(),
    );
    let changes = connection.total_changes();
    // SQLite itself rejects any SQL write during the public preparation call.
    connection.pragma_update(None, "query_only", true).unwrap();
    let result = prepare_public(&mut connection, &request).unwrap();
    let again = prepare_public(&mut connection, &request).unwrap();
    assert_eq!(result, again);
    assert_eq!(connection.total_changes(), changes);
    assert!(connection.is_autocommit(), "read transaction leaked");
    assert_eq!(
        result.state_root(),
        &root_digest(*tree(&[(key(0), value(4)), (key(64), value(5))]).root()).unwrap()
    );
    assert_eq!(
        result.unresolved_root(),
        &root_digest(H256::zero()).unwrap()
    );
    let batch = result
        .bind_successor(checkpoint(9, "checkpoint-9", '9'))
        .unwrap();
    assert!(!batch.state().mutations().is_empty());
    assert!(!batch.unresolved().mutations().is_empty());
    assert!(batch
        .state()
        .mutations()
        .iter()
        .any(|mutation| mutation.successor().is_none()));
    connection.pragma_update(None, "query_only", false).unwrap();
    let tx = connection
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .unwrap();
    tx.rollback().unwrap();
    drop(connection);
    let pale_ports::AccumulatorPreparationBasis::Successor(prior) = request.basis() else {
        panic!("replacement fixture requires an admitted predecessor")
    };
    assert_eq!(
        batch.binding().phase(),
        &pale_ports::AccumulatorCheckpointPhase::Successor(prior.checkpoint().clone())
    );
}

#[test]
fn empty_tree_insert_and_unchanged_other_tree_preserve_explicit_roots() {
    let state = tree(&[]);
    let unresolved = tree(&[(key(128), value(8))]);
    let mut connection = Connection::open_in_memory().unwrap();
    seed(&connection, &state, &unresolved);
    let result = prepare_public(
        &mut connection,
        &request(
            prior(&state, &unresolved),
            vec![change(key(0), None, Some(value(1)))],
            vec![],
            limits(),
        ),
    )
    .unwrap();
    assert_eq!(
        result.state_root(),
        &root_digest(*tree(&[(key(0), value(1))]).root()).unwrap()
    );
    assert_eq!(
        result.unresolved_root(),
        &root_digest(*unresolved.root()).unwrap()
    );
    assert!(result
        .bind_successor(checkpoint(9, "checkpoint-9", '9'))
        .unwrap()
        .unresolved()
        .mutations()
        .is_empty());
}

#[test]
fn selected_predecessor_coordinates_and_corrupt_rows_fail_without_writes() {
    for corruption in [
        "UPDATE pale_control_store_header SET control_incarnation_id='foreign-incarnation'",
        "UPDATE pale_deployment_operation_checkpoint_head SET domain_sequence=9",
        "UPDATE pale_deployment_operation_accumulator_frontier SET checkpoint_profile_ref='foreign-profile'",
        "UPDATE pale_deployment_operation_accumulator_frontier SET checkpoint_profile_sha256=printf('%064d',0)",
        "UPDATE pale_deployment_operation_accumulator_frontier SET state_root_sha256=printf('%064d',0)",
        "UPDATE pale_deployment_operation_accumulator_frontier SET unresolved_root_sha256=printf('%064d',0)",
        "DELETE FROM pale_deployment_operation_accumulator_position WHERE position_kind='leaf' AND tree_kind='state'",
        "DELETE FROM pale_deployment_operation_accumulator_position WHERE position_kind='branch' AND tree_kind='state'",
        "UPDATE pale_deployment_operation_accumulator_position SET node_bytes=zeroblob(32) WHERE position_kind='leaf' AND tree_kind='state'",
        "UPDATE pale_deployment_operation_accumulator_position SET node_bytes=zeroblob(66) WHERE position_kind='branch' AND tree_kind='state'",
    ] {
        let state = tree(&[(key(0), value(1)), (key(128), value(2))]);
        let unresolved = tree(&[(key(64), value(3))]);
        let mut connection = Connection::open_in_memory().unwrap();
        seed(&connection, &state, &unresolved);
        let request = request(
            prior(&state, &unresolved),
            vec![change(key(0), Some(value(1)), Some(value(4)))],
            vec![],
            limits(),
        );
        connection.execute_batch(corruption).unwrap();
        let changes = connection.total_changes();
        match prepare_public(&mut connection, &request).unwrap_err() {
            OperationAccumulatorPreparationError::Store(error) => assert_eq!(
                error.kind(),
                ControlStoreErrorKind::RecoveryRequired,
                "{corruption}"
            ),
            other => panic!("wrong failure for {corruption}: {other:?}"),
        }
        assert_eq!(connection.total_changes(), changes);
        assert!(connection.is_autocommit());
    }
}

#[test]
fn target_shortcut_height_rejects_even_when_predecessor_membership_passes() {
    let mut state = tree(&[(key(0), value(1)), (key(128), value(2))]);
    let trusted_root = *state.root();
    let root_key = BranchKey::new(255, H256::zero());
    let mut branch = state.store().branches_map()[&root_key].clone();
    branch.left = MergeValue::ShortCut {
        key: key(0),
        value: value(1),
        height: 254,
    };
    state.store_mut().insert_branch(root_key, branch).unwrap();
    let proof = state
        .merkle_proof(vec![key(0)])
        .unwrap()
        .compile(vec![key(0)])
        .unwrap();
    assert!(proof
        .verify::<CheckpointHasher>(&trusted_root, vec![(key(0), value(1))])
        .unwrap());
    let unresolved = tree(&[]);
    let mut connection = Connection::open_in_memory().unwrap();
    seed(&connection, &state, &unresolved);
    let changes = connection.total_changes();
    assert!(matches!(
        prepare_public(
            &mut connection,
            &request(
                prior(&state, &unresolved),
                vec![change(key(0), Some(value(1)), Some(value(4)))],
                vec![],
                limits()
            )
        ),
        Err(OperationAccumulatorPreparationError::Store(_))
    ));
    assert_eq!(connection.total_changes(), changes);
}

#[test]
fn combined_budgets_charge_repeated_work_and_never_publish_partial_results() {
    let state = tree(&[(key(0), value(1)), (key(128), value(2))]);
    let unresolved = tree(&[(key(0), value(1)), (key(128), value(2))]);
    let mut connection = Connection::open_in_memory().unwrap();
    seed(&connection, &state, &unresolved);
    let prior = prior(&state, &unresolved);
    let changes = vec![
        change(key(0), Some(value(1)), Some(value(3))),
        change(key(64), None, Some(value(4))),
    ];
    // Count one real proof/update through the exact implementation, not a guessed
    // magic budget. The second identical tree must share every finite ceiling.
    let budget = Budget {
        maximum_reads: limits().maximum_reads(),
        maximum_writes: limits().maximum_writes(),
        maximum_positions: limits().maximum_positions(),
        maximum_proof_bytes: limits().maximum_proof_bytes(),
        work: RefCell::new(Work::default()),
    };
    prepare_tree(
        SnapshotOverlay {
            storage: Store::DeploymentOperation,
            connection: &connection,
            domain: prior.checkpoint().checkpoint_domain_id(),
            epoch: prior.checkpoint().domain_epoch(),
            profile_ref: prior.profile_ref(),
            profile_sha256: prior.profile_sha256(),
            tree: "state",
            budget: &budget,
            touched: BTreeMap::new(),
        },
        prior.state_root(),
        &changes,
    )
    .unwrap();
    let work = budget.work.borrow();
    assert!(work.reads > 1 && work.writes > 1 && work.positions > 1 && work.proof_bytes > 1);
    assert!(
        work.writes > work.positions,
        "fixture must overwrite intermediate nodes"
    );
    let proof = state
        .merkle_proof(
            changes
                .iter()
                .map(|change| root_hash(change.key()).unwrap())
                .collect(),
        )
        .unwrap();
    let bound = compiled_proof_bound(&proof).unwrap();
    assert!(bound >= work.proof_bytes);
    let measured = [work.reads, work.writes, work.positions, bound];
    drop(work);
    for index in 0..4 {
        let mut ceilings = [100_000, 100_000, 20_000, 1_000_000];
        ceilings[index] = measured[index];
        let limits = AccumulatorPreparationLimits::new(
            4,
            ceilings[0],
            ceilings[1],
            ceilings[2],
            ceilings[3],
        )
        .unwrap();
        // One tree fits its measured work or conservative precompile bound;
        // both exceed it, including the first tree's actual compiled bytes.
        prepare_public(
            &mut connection,
            &request(prior.clone(), changes.clone(), vec![], limits),
        )
        .unwrap();
        let before = connection.total_changes();
        assert!(
            matches!(
                prepare_public(
                    &mut connection,
                    &request(prior.clone(), changes.clone(), changes.clone(), limits)
                ),
                Err(OperationAccumulatorPreparationError::Limit)
            ),
            "budget index {index}"
        );
        assert_eq!(connection.total_changes(), before);
        assert!(connection.is_autocommit());
    }
    prepare_public(
        &mut connection,
        &request(prior, changes.clone(), changes, limits()),
    )
    .unwrap();
}

#[test]
fn compiled_proof_encoded_bound_is_checked_before_compile_and_updates() {
    for count in [0_u8, 1, 2, 8, 32] {
        let leaves: Vec<_> = (0..count)
            .map(|byte| (key(byte * 4), value(byte)))
            .collect();
        let state = tree(&leaves);
        let keys = vec![key(0), key(65), key(128), key(255)];
        let proof = state.merkle_proof(keys.clone()).unwrap();
        let bound = compiled_proof_bound(&proof).unwrap();
        let compiled = proof.compile(keys).unwrap();
        assert!(compiled.0.len() <= bound);
    }
    let state = tree(&[(key(0), value(1)), (key(128), value(2))]);
    let unresolved = tree(&[]);
    let mut connection = Connection::open_in_memory().unwrap();
    seed(&connection, &state, &unresolved);
    let prior = prior(&state, &unresolved);
    let bound = compiled_proof_bound(&state.merkle_proof(vec![key(0)]).unwrap()).unwrap();
    let limits = AccumulatorPreparationLimits::new(1, 100_000, 100_000, 20_000, bound - 1).unwrap();
    let budget = Budget {
        maximum_reads: limits.maximum_reads(),
        maximum_writes: limits.maximum_writes(),
        maximum_positions: limits.maximum_positions(),
        maximum_proof_bytes: limits.maximum_proof_bytes(),
        work: RefCell::new(Work::default()),
    };
    let result = prepare_tree(
        SnapshotOverlay {
            storage: Store::DeploymentOperation,
            connection: &connection,
            domain: prior.checkpoint().checkpoint_domain_id(),
            epoch: prior.checkpoint().domain_epoch(),
            profile_ref: prior.profile_ref(),
            profile_sha256: prior.profile_sha256(),
            tree: "state",
            budget: &budget,
            touched: BTreeMap::new(),
        },
        prior.state_root(),
        &[change(key(0), Some(value(1)), Some(value(3)))],
    );
    assert!(matches!(
        budget.error(result.unwrap_err()),
        OperationAccumulatorPreparationError::Limit
    ));
    let work = budget.work.borrow();
    assert_eq!(work.proof_bytes, 0);
    assert_eq!(work.writes, 0);
    assert_eq!(work.positions, 0);
    drop(work);
    let exact = AccumulatorPreparationLimits::new(1, 100_000, 100_000, 20_000, bound).unwrap();
    prepare_public(
        &mut connection,
        &request(
            prior,
            vec![change(key(0), Some(value(1)), Some(value(3)))],
            vec![],
            exact,
        ),
    )
    .unwrap();
}

#[test]
fn sql_failure_remains_storage_error_not_a_limit_or_absence() {
    let state = tree(&[(key(0), value(1))]);
    let unresolved = tree(&[]);
    let mut connection = Connection::open_in_memory().unwrap();
    seed(&connection, &state, &unresolved);
    connection
        .execute_batch("DROP TABLE pale_deployment_operation_accumulator_position")
        .unwrap();
    let error = prepare_public(
        &mut connection,
        &request(
            prior(&state, &unresolved),
            vec![change(key(0), Some(value(1)), Some(value(2)))],
            vec![],
            limits(),
        ),
    )
    .unwrap_err();
    match error {
        OperationAccumulatorPreparationError::Store(error) => {
            assert_eq!(error.kind(), ControlStoreErrorKind::Storage)
        }
        other => panic!("wrong error class: {other:?}"),
    }
    assert!(connection.is_autocommit());
}

#[test]
fn both_trees_read_one_wal_snapshot_despite_another_connection_committing() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("snapshot.sqlite");
    let mut connection = Connection::open(&path).unwrap();
    connection
        .pragma_update(None, "journal_mode", "WAL")
        .unwrap();
    connection
        .pragma_update(None, "synchronous", "FULL")
        .unwrap();
    let state = tree(&[(key(0), value(1)), (key(128), value(2))]);
    let unresolved = tree(&[(key(0), value(1)), (key(128), value(2))]);
    seed(&connection, &state, &unresolved);
    let prior = prior(&state, &unresolved);
    let changes = vec![change(key(0), Some(value(1)), Some(value(3)))];
    let mut writer = Connection::open(&path).unwrap();
    let tx = connection
        .transaction_with_behavior(rusqlite::TransactionBehavior::Deferred)
        .unwrap();
    let budget = Budget {
        maximum_reads: limits().maximum_reads(),
        maximum_writes: limits().maximum_writes(),
        maximum_positions: limits().maximum_positions(),
        maximum_proof_bytes: limits().maximum_proof_bytes(),
        work: RefCell::new(Work::default()),
    };
    let first = prepare_tree(
        SnapshotOverlay {
            storage: Store::DeploymentOperation,
            connection: &tx,
            domain: prior.checkpoint().checkpoint_domain_id(),
            epoch: prior.checkpoint().domain_epoch(),
            profile_ref: prior.profile_ref(),
            profile_sha256: prior.profile_sha256(),
            tree: "state",
            budget: &budget,
            touched: BTreeMap::new(),
        },
        prior.state_root(),
        &changes,
    )
    .unwrap();
    // A committed corrupt replacement is visible to later snapshots but cannot
    // alter the second tree in the already-open preparation snapshot.
    let write = writer.transaction().unwrap();
    write
        .execute(
            "UPDATE pale_deployment_operation_accumulator_position SET node_bytes=zeroblob(32) \
        WHERE tree_kind='unresolved' AND position_kind='leaf'",
            [],
        )
        .unwrap();
    write.commit().unwrap();
    let second = prepare_tree(
        SnapshotOverlay {
            storage: Store::DeploymentOperation,
            connection: &tx,
            domain: prior.checkpoint().checkpoint_domain_id(),
            epoch: prior.checkpoint().domain_epoch(),
            profile_ref: prior.profile_ref(),
            profile_sha256: prior.profile_sha256(),
            tree: "unresolved",
            budget: &budget,
            touched: BTreeMap::new(),
        },
        prior.unresolved_root(),
        &changes,
    )
    .unwrap();
    assert_eq!(first, second);
    tx.rollback().unwrap();
    assert!(matches!(
        prepare_public(
            &mut connection,
            &request(prior, changes.clone(), changes, limits())
        ),
        Err(OperationAccumulatorPreparationError::Store(_))
    ));
    assert!(connection.is_autocommit());
}

#[test]
fn real_preparation_commits_reopens_rebuilds_and_rejects_competing_stale_batch() {
    use pale_ports::{
        OperationCustodyCandidate, OperationCustodyCommitOutcome, OperationCustodyIndexMutation,
        OperationCustodyReadBackOutcome,
    };
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("prepared-custody.sqlite");
    let state = tree(&[(key(0), value(1)), (key(128), value(2))]);
    let unresolved = tree(&[(key(64), value(3))]);
    let winner;
    let competitor;
    {
        let mut connection = Connection::open(&path).unwrap();
        connection
            .pragma_update(None, "journal_mode", "WAL")
            .unwrap();
        connection
            .pragma_update(None, "synchronous", "FULL")
            .unwrap();
        seed(&connection, &state, &unresolved);
        let mut prepare = |tag, bytes, replacement| {
            let template = start_candidate(tag, bytes);
            let prepared = prepare_public(
                &mut connection,
                &request(
                    prior(&state, &unresolved),
                    vec![change(key(0), Some(value(1)), Some(value(replacement)))],
                    vec![change(key(64), Some(value(3)), Some(value(replacement)))],
                    limits(),
                ),
            )
            .unwrap();
            let batch = prepared
                .bind_successor(template.checkpoint().successor().clone())
                .unwrap();
            let indexes = template
                .indexes()
                .iter()
                .filter(|index| {
                    !matches!(
                        index,
                        OperationCustodyIndexMutation::RequestForHandle { .. }
                            | OperationCustodyIndexMutation::StartObservation { .. }
                            | OperationCustodyIndexMutation::ObservationCheckpoint { .. }
                    )
                })
                .cloned()
                .collect();
            // Only the existing opaque record fixture is reused. The physical
            // batch now comes from actual verified native preparation.
            OperationCustodyCandidate::new_start(
                store(),
                template.kind(),
                template.records().to_vec(),
                indexes,
                template.checkpoint().clone(),
                batch,
                template.session_basis().unwrap().clone(),
            )
            .unwrap()
        };
        winner = prepare("winner", ['1', '2', '3', '9'], 4);
        competitor = prepare("competitor", ['4', '5', '6', 'a'], 5);
        let mut custody = SqliteOperationCustody {
            connection: &mut connection,
            coordinate: store(),
        };
        assert_eq!(
            custody.compare_and_commit(&winner).unwrap(),
            OperationCustodyCommitOutcome::Committed
        );
        assert_eq!(
            custody.compare_and_commit(&competitor).unwrap_err().kind(),
            ControlStoreErrorKind::CandidateConflict
        );
        assert_eq!(
            custody.observe_exact(&winner).unwrap(),
            OperationCustodyReadBackOutcome::ExactSuccessor
        );
    }
    let mut connection = Connection::open(&path).unwrap();
    connection
        .pragma_update(None, "foreign_keys", true)
        .unwrap();
    connection
        .pragma_update(None, "synchronous", "FULL")
        .unwrap();
    for (name, expected) in [
        ("state", winner.accumulator().state().successor_root()),
        (
            "unresolved",
            winner.accumulator().unresolved().successor_root(),
        ),
    ] {
        let mut statement = connection.prepare("SELECT position_key,node_bytes FROM pale_deployment_operation_accumulator_position \
            WHERE tree_kind=?1 AND position_kind='leaf'").unwrap();
        let mut rows = statement.query([name]).unwrap();
        let mut leaves = Vec::new();
        while let Some(row) = rows.next().unwrap() {
            let ValueRef::Blob(position) = row.get_ref(0).unwrap() else {
                panic!("leaf position type")
            };
            let ValueRef::Blob(bytes) = row.get_ref(1).unwrap() else {
                panic!("leaf value type")
            };
            assert_eq!(position.len(), 33);
            assert_eq!(position[0], 0);
            leaves.push((
                codec::hash(&position[1..]).unwrap(),
                codec::hash(bytes).unwrap(),
            ));
        }
        // Full rebuild from the persisted leaves, not copied expected roots.
        assert_eq!(&root_digest(*tree(&leaves).root()).unwrap(), expected);
    }
    let before = connection.total_changes();
    let mut custody = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    assert_eq!(
        custody.observe_exact(&winner).unwrap(),
        OperationCustodyReadBackOutcome::ExactSuccessor
    );
    assert_eq!(
        custody.compare_and_commit(&winner).unwrap(),
        OperationCustodyCommitOutcome::ExactReplay
    );
    assert_eq!(
        custody.compare_and_commit(&competitor).unwrap_err().kind(),
        ControlStoreErrorKind::CandidateConflict
    );
    assert_eq!(
        custody.observe_exact(&winner).unwrap(),
        OperationCustodyReadBackOutcome::ExactSuccessor
    );
    assert_eq!(connection.total_changes(), before);
    assert!(connection.is_autocommit());

    let stale = request(
        prior(&state, &unresolved),
        vec![change(key(0), Some(value(1)), Some(value(7)))],
        vec![],
        limits(),
    );
    let classify_prepare =
        |connection: &mut Connection| match prepare_public(connection, &stale).unwrap_err() {
            OperationAccumulatorPreparationError::Store(error) => error.kind(),
            other => panic!("wrong failure: {other:?}"),
        };
    assert_eq!(
        classify_prepare(&mut connection),
        ControlStoreErrorKind::CandidateConflict
    );
    for sequence in [7, 8] {
        // The complete winning immutable record remains present, but its
        // claimed lower/same-position head cannot become a stale conflict.
        connection
            .execute(
                "UPDATE pale_deployment_operation_checkpoint_head SET domain_sequence=?1",
                [sequence],
            )
            .unwrap();
        let before = connection.total_changes();
        assert_eq!(
            classify_prepare(&mut connection),
            ControlStoreErrorKind::RecoveryRequired
        );
        assert_eq!(
            SqliteOperationCustody {
                connection: &mut connection,
                coordinate: store()
            }
            .compare_and_commit(&competitor)
            .unwrap_err()
            .kind(),
            ControlStoreErrorKind::RecoveryRequired
        );
        assert_eq!(connection.total_changes(), before);
        connection
            .execute(
                "UPDATE pale_deployment_operation_checkpoint_head SET domain_sequence=9",
                [],
            )
            .unwrap();
    }
    assert_eq!(
        classify_prepare(&mut connection),
        ControlStoreErrorKind::CandidateConflict
    );
    connection
        .execute(
            "DELETE FROM pale_deployment_control_record WHERE record_ref='checkpoint-8'",
            [],
        )
        .unwrap();
    assert_eq!(
        classify_prepare(&mut connection),
        ControlStoreErrorKind::RecoveryRequired
    );
}
