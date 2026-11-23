//! Tree proofs only; the operation tests cover signed records and anchor custody.

use super::*;

#[test]
fn committed_recovery_reconstructs_exact_updates_insertions_and_deletions() {
    for (before, after) in [
        (
            vec![(key(0), value(1)), (key(128), value(2))],
            vec![(key(0), value(3)), (key(128), value(2))],
        ),
        (vec![], vec![(key(0), value(1))]),
        (vec![(key(0), value(1))], vec![]),
        (
            vec![(key(0), value(1)), (key(128), value(2))],
            vec![(key(64), value(3)), (key(128), value(2))],
        ),
    ] {
        let old = tree(&before);
        let next = tree(&after);
        let unresolved_old = tree(&[(key(16), value(4))]);
        let unresolved_next = tree(&[(key(16), value(5))]);
        let changes = [key(0), key(64), key(128)]
            .into_iter()
            .filter_map(|key| {
                let a = before
                    .iter()
                    .find(|entry| entry.0 == key)
                    .map(|entry| entry.1);
                let b = after
                    .iter()
                    .find(|entry| entry.0 == key)
                    .map(|entry| entry.1);
                (a != b).then(|| change(key, a, b))
            })
            .collect();
        let request = request(
            prior(&old, &unresolved_old),
            changes,
            vec![change(key(16), Some(value(4)), Some(value(5)))],
            limits(),
        );
        let original = Connection::open_in_memory().unwrap();
        seed(&original, &old, &unresolved_old);
        let expected = prepare(&original, Store::DeploymentOperation, &request).unwrap();
        let committed = Connection::open_in_memory().unwrap();
        seed(&committed, &next, &unresolved_next);
        committed.pragma_update(None, "query_only", true).unwrap();
        let writes = committed.total_changes();
        let recovered = recover_committed(
            &committed,
            Store::DeploymentOperation,
            &request,
            &prior(&next, &unresolved_next),
        )
        .unwrap();
        assert_eq!(recovered, expected);
        assert_eq!(committed.total_changes(), writes);
    }
}

#[test]
fn committed_recovery_rejects_wrong_leaves_roots_frontier_and_corruption() {
    let old = tree(&[(key(0), value(1)), (key(128), value(2))]);
    let next = tree(&[(key(0), value(3)), (key(128), value(2))]);
    let unresolved = tree(&[]);
    let successor = prior(&next, &unresolved);
    let make_request = |expected, old_root, limits| {
        request(
            old_root,
            vec![change(key(0), Some(value(1)), Some(expected))],
            vec![],
            limits,
        )
    };
    let connection = Connection::open_in_memory().unwrap();
    seed(&connection, &next, &unresolved);
    let valid = make_request(value(3), prior(&old, &unresolved), limits());
    for invalid_request in [
        make_request(value(4), prior(&old, &unresolved), limits()),
        make_request(
            value(3),
            prior(&tree(&[(key(0), value(1))]), &unresolved),
            limits(),
        ),
        make_request(
            value(3),
            prior(&old, &unresolved),
            AccumulatorPreparationLimits::new(1, 1, 1, 1, 1).unwrap(),
        ),
    ] {
        assert!(recover_committed(
            &connection,
            Store::DeploymentOperation,
            &invalid_request,
            &successor
        )
        .is_err());
    }
    assert!(recover_committed(
        &connection,
        Store::DeploymentOperation,
        &valid,
        &prior(&old, &unresolved)
    )
    .is_err());
    connection.execute("UPDATE pale_deployment_operation_accumulator_position SET node_bytes=zeroblob(32) WHERE tree_kind='state' AND position_kind='leaf'", []).unwrap();
    let writes = connection.total_changes();
    assert!(
        recover_committed(&connection, Store::DeploymentOperation, &valid, &successor).is_err()
    );
    assert_eq!(connection.total_changes(), writes);
}
