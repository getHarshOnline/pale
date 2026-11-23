//! Application-produced roots and bootstrap records share real `SQLite` custody.

use super::*;
use crate::resident_fixture::principal_checkpoint_fixture as fixture;
use pale_application::resident::{
    PreparedPrincipalBootstrapCustody, PrincipalBootstrapCommitEvidence,
    PrincipalBootstrapCustodyPhase,
};
use pale_core::records::{ArtifactDigest, ControlLeafCountV1, ControlStateCheckpointPreimageV1};

fn prepare(
    connection: &mut Connection,
) -> (
    PreparedPrincipalBootstrapCustody,
    ControlStateCheckpointPreimageV1,
) {
    fixture::prepare(&mut SqlitePrincipalBootstrapCustody { connection }, |_| {})
        .expect("admitted records feed the native tree producer")
}

fn transaction_sequence(connection: &Connection) -> i64 {
    connection
        .query_row(
            "SELECT logical_transaction_sequence FROM pale_control_store_header WHERE singleton=1",
            [],
            |row| row.get(0),
        )
        .unwrap()
}

#[test]
fn two_initial_domains_commit_and_replay_as_one_physical_atom() {
    let mut connection = in_memory_store();
    let parts = candidate_parts("bootstrap-two-domains", r#"{"record":"receipt"}"#);
    let batches = vec![
        synthetic_accumulator(&parts.binding, &parts.checkpoints, 0, 1),
        synthetic_empty_accumulator(&parts.binding, &parts.checkpoints, 1),
    ];
    let candidate = PreparedPrincipalBootstrapAtom::new(
        parts.binding.clone(),
        parts.records.clone(),
        parts.checkpoints.clone(),
        parts.head.clone(),
        batches.clone(),
    )
    .unwrap();
    {
        let mut custody = SqlitePrincipalBootstrapCustody {
            connection: &mut connection,
        };
        assert_eq!(
            custody.compare_and_commit(&candidate).unwrap(),
            PrincipalBootstrapCommitDisposition::New
        );
        assert_eq!(
            custody.observe_exact(&candidate).unwrap(),
            PrincipalBootstrapDatabaseObservation::Exact
        );
        assert_eq!(
            custody.compare_and_commit(&candidate).unwrap(),
            PrincipalBootstrapCommitDisposition::ExactReplay
        );
    }
    let frontiers: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM pale_principal_control_accumulator_frontier",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(frontiers, 2);
    for batch in &batches {
        assert!(crate::accumulator::matches(
            &connection,
            crate::accumulator::Store::PrincipalTrust,
            batch,
            true,
        )
        .unwrap());
    }

    let unlike = PreparedPrincipalBootstrapAtom::new(
        parts.binding.clone(),
        parts.records,
        parts.checkpoints.clone(),
        parts.head,
        vec![
            batches[0].clone(),
            synthetic_accumulator(&parts.binding, &parts.checkpoints, 1, 3),
        ],
    )
    .unwrap();
    let mut custody = SqlitePrincipalBootstrapCustody {
        connection: &mut connection,
    };
    assert_eq!(
        custody.observe_exact(&unlike).unwrap(),
        PrincipalBootstrapDatabaseObservation::ConflictingOrPartial
    );
    assert_eq!(
        custody.compare_and_commit(&unlike).unwrap_err().kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
}

#[test]
fn failure_in_the_second_initial_domain_rolls_back_the_whole_set() {
    let mut connection = in_memory_store();
    let parts = candidate_parts("bootstrap-two-domains", r#"{"record":"receipt"}"#);
    let batches = vec![
        synthetic_accumulator(&parts.binding, &parts.checkpoints, 0, 1),
        synthetic_empty_accumulator(&parts.binding, &parts.checkpoints, 1),
    ];
    let candidate = PreparedPrincipalBootstrapAtom::new(
        parts.binding,
        parts.records,
        parts.checkpoints,
        parts.head,
        batches,
    )
    .unwrap();
    connection
        .execute_batch(
            "CREATE TRIGGER fail_second_frontier BEFORE INSERT ON \
             pale_principal_control_accumulator_frontier \
             WHEN NEW.checkpoint_domain_id = 'principal-head' \
             BEGIN SELECT RAISE(ABORT, 'injected second-domain failure'); END;",
        )
        .unwrap();
    assert!(SqlitePrincipalBootstrapCustody {
        connection: &mut connection,
    }
    .compare_and_commit(&candidate)
    .is_err());
    assert_empty(&connection);
    assert_eq!(
        SqlitePrincipalBootstrapCustody {
            connection: &mut connection,
        }
        .observe_exact(&candidate)
        .unwrap(),
        PrincipalBootstrapDatabaseObservation::Empty
    );
}

#[test]
fn signed_bootstrap_completes_its_own_anchors_after_reopen_and_exact_replay() {
    use pale_application::resident::AnchorAdvanceEvidence;
    use pale_ports::{AntiRollbackAnchor, AntiRollbackAnchorPort};
    let directory = crate::test_support::private_tempdir();
    let path = directory.path().join("anchored-principal.sqlite3");
    let prepared;
    let control;
    {
        let mut connection = initialize_store(Connection::open(&path).unwrap());
        (prepared, control) = prepare(&mut connection);
        let committed = prepared
            .clone()
            .commit(&mut SqlitePrincipalBootstrapCustody {
                connection: &mut connection,
            })
            .unwrap();
        assert_eq!(committed.evidence(), PrincipalBootstrapCommitEvidence::New);
        // Simulate exit before anchor completion. No anchor lives in this database.
    }
    let mut anchors = crate::test_support::RetainedAntiRollbackAnchors::default();
    let mut connection = Connection::open(&path).unwrap();
    connection
        .pragma_update(None, "foreign_keys", true)
        .unwrap();
    let before = connection.total_changes();
    let complete = prepared
        .clone()
        .commit(&mut SqlitePrincipalBootstrapCustody {
            connection: &mut connection,
        })
        .unwrap()
        .advance_anchors(&mut anchors)
        .unwrap();
    assert_eq!(
        complete.database_evidence(),
        PrincipalBootstrapCommitEvidence::ExactReplay
    );
    assert_eq!(complete.phase(), PrincipalBootstrapCustodyPhase::Complete);
    assert_eq!(complete.anchors().len(), 2);
    for (anchor, evidence) in complete.anchors() {
        assert_eq!(*evidence, AnchorAdvanceEvidence::Advanced);
        assert_eq!(anchors.read(&anchor.key()).unwrap().as_ref(), Some(anchor));
    }
    let AntiRollbackAnchor::ControlDomain(domain) = &complete.anchors()[1].0 else {
        panic!("control domain")
    };
    assert_eq!(domain.control_store_id(), &control.control_store_id);
    assert_eq!(
        domain.control_incarnation_id(),
        &control.control_incarnation_id
    );
    assert_ne!(
        domain.control_store_id(),
        complete.candidate().binding().trust_store_id()
    );
    assert_eq!(
        domain.coordinate(),
        complete.candidate().control_checkpoints()[0].coordinate()
    );
    let replay = prepared
        .commit(&mut SqlitePrincipalBootstrapCustody {
            connection: &mut connection,
        })
        .unwrap()
        .advance_anchors(&mut anchors)
        .unwrap();
    assert_eq!(replay.candidate(), complete.candidate());
    assert!(replay
        .anchors()
        .iter()
        .all(|(_, evidence)| *evidence == AnchorAdvanceEvidence::ExactReplay));
    assert_eq!(connection.total_changes(), before);
}

fn assert_empty(connection: &Connection) {
    assert_eq!(row_counts(connection), (0, 0, 0, 0, 0));
    for table in [
        "pale_principal_control_accumulator_position",
        "pale_principal_control_accumulator_frontier",
    ] {
        let count: i64 = connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0, "no partial tree remains in {table}");
    }
    assert_eq!(transaction_sequence(connection), 1);
}

#[test]
fn produced_roots_commit_and_reopen_with_distinct_physical_and_logical_stores() {
    let directory = crate::test_support::private_tempdir();
    let path = directory.path().join("principal.sqlite3");
    let candidate;
    let control;
    {
        let mut connection = initialize_store(Connection::open(&path).unwrap());
        let before = connection.total_changes();
        let (prepared, checkpoint) = prepare(&mut connection);
        assert_eq!(
            connection.total_changes(),
            before,
            "preparation is read-only"
        );
        assert_empty(&connection);
        candidate = prepared.candidate().clone();
        control = checkpoint;
        assert_eq!(
            candidate.binding().trust_store_id().as_str(),
            "trust-store-1"
        );
        assert_eq!(
            control.control_store_id.as_str(),
            "principal-control-store-1"
        );
        assert_ne!(
            candidate.binding().trust_store_id(),
            &control.control_store_id
        );
        assert_eq!(control.committed_transaction_sequence.get(), 2);
        assert_eq!(control.leaf_count.get(), 2);
        assert_eq!(control.changed_leaf_keys.len(), 2);
        assert_eq!(candidate.accumulators().len(), 1);
        let committed = prepared
            .commit(&mut SqlitePrincipalBootstrapCustody {
                connection: &mut connection,
            })
            .unwrap();
        assert_eq!(committed.evidence(), PrincipalBootstrapCommitEvidence::New);
        assert_eq!(
            committed.phase(),
            PrincipalBootstrapCustodyPhase::DatabaseCommittedAnchorsPending
        );
        assert_eq!(transaction_sequence(&connection), 2);
        let roots: (String, String) = connection.query_row(
            "SELECT state_root_sha256, unresolved_root_sha256 FROM pale_principal_control_accumulator_frontier",
            [], |row| Ok((row.get(0)?, row.get(1)?)),
        ).unwrap();
        assert_eq!(roots.0, control.state_root_sha256.as_str());
        assert_eq!(roots.1, control.unresolved_custody_root_sha256.as_str());
    }
    let mut connection = Connection::open(&path).unwrap();
    connection
        .pragma_update(None, "foreign_keys", true)
        .unwrap();
    let before = connection.total_changes();
    let mut custody = SqlitePrincipalBootstrapCustody {
        connection: &mut connection,
    };
    assert_eq!(
        custody.observe_exact(&candidate).unwrap(),
        PrincipalBootstrapDatabaseObservation::Exact
    );
    assert_eq!(
        custody.compare_and_commit(&candidate).unwrap(),
        PrincipalBootstrapCommitDisposition::ExactReplay
    );
    assert!(
        fixture::prepare(&mut custody, |_| {}).is_err(),
        "existing roots cannot be prepared as an absent domain"
    );
    assert_eq!(connection.total_changes(), before);
    assert_eq!(
        transaction_sequence(&connection),
        i64::try_from(control.committed_transaction_sequence.get()).unwrap()
    );
}

#[test]
fn complete_competing_checkpoint_does_not_look_like_partial_custody() {
    let mut connection = in_memory_store();
    let (winner, _) = prepare(&mut connection);
    let (other, _) = fixture::prepare(
        &mut SqlitePrincipalBootstrapCustody {
            connection: &mut connection,
        },
        |record| record.issued_at = UnixTimeV1::new(46).unwrap(),
    )
    .unwrap();
    let other = other.candidate().clone();
    let committed = winner
        .commit(&mut SqlitePrincipalBootstrapCustody {
            connection: &mut connection,
        })
        .unwrap();
    assert_eq!(
        committed.phase(),
        PrincipalBootstrapCustodyPhase::DatabaseCommittedAnchorsPending
    );
    let before = connection.total_changes();
    let mut custody = SqlitePrincipalBootstrapCustody {
        connection: &mut connection,
    };
    assert_eq!(
        custody.compare_and_commit(&other).unwrap_err().kind(),
        ControlStoreErrorKind::CandidateConflict
    );
    assert_eq!(
        custody.observe_exact(&other).unwrap(),
        PrincipalBootstrapDatabaseObservation::ConflictingOrPartial
    );
    assert_eq!(connection.total_changes(), before);
    assert_eq!(transaction_sequence(&connection), 2);
}

#[test]
fn signed_checkpoint_changes_cannot_replace_the_native_construction() {
    let faults: [fn(&mut ControlStateCheckpointPreimageV1); 6] = [
        |record| record.state_root_sha256 = ArtifactDigest::hash_bytes(b"wrong state root"),
        |record| {
            record.unresolved_custody_root_sha256 =
                ArtifactDigest::hash_bytes(b"wrong unresolved root")
        },
        |record| record.changed_leaf_keys.clear(),
        |record| record.leaf_count = ControlLeafCountV1::new(3).unwrap(),
        |record| {
            record.committed_transaction_sequence = ControlTransactionSequenceV1::new(3).unwrap()
        },
        |record| {
            record.control_store_id = ControlStoreIdV1::parse("different-logical-store").unwrap()
        },
    ];
    for (index, fault) in faults.into_iter().enumerate() {
        let mut connection = in_memory_store();
        let before = connection.total_changes();
        assert!(
            fixture::prepare(
                &mut SqlitePrincipalBootstrapCustody {
                    connection: &mut connection,
                },
                fault
            )
            .is_err(),
            "changed signed construction {index}"
        );
        assert_eq!(connection.total_changes(), before);
        assert_empty(&connection);
    }
}

#[test]
fn physical_store_mismatch_cannot_prepare_principal_roots() {
    for column in ["logical_store_id", "control_incarnation_id"] {
        let mut connection = in_memory_store();
        connection.execute(
            &format!("UPDATE pale_control_store_header SET {column}='different-store' WHERE singleton=1"), [],
        ).unwrap();
        let before = connection.total_changes();
        assert!(
            fixture::prepare(
                &mut SqlitePrincipalBootstrapCustody {
                    connection: &mut connection,
                },
                |_| {}
            )
            .is_err(),
            "different physical {column}"
        );
        assert_eq!(connection.total_changes(), before);
        assert_empty(&connection);
    }
}

#[test]
fn bootstrap_record_tree_and_head_writes_roll_back_together() {
    for (table, action) in [
        ("pale_principal_trust_record", "INSERT"),
        ("pale_principal_trust_bootstrap_head", "INSERT"),
        ("pale_principal_trust_checkpoint_head", "INSERT"),
        ("pale_principal_control_checkpoint_head", "INSERT"),
        ("pale_principal_control_accumulator_position", "INSERT"),
        ("pale_principal_control_accumulator_frontier", "INSERT"),
        ("pale_control_store_header", "UPDATE"),
    ] {
        let mut connection = in_memory_store();
        let (prepared, _) = prepare(&mut connection);
        let candidate = prepared.candidate().clone();
        connection.execute_batch(&format!(
            "CREATE TRIGGER fail_bootstrap BEFORE {action} ON {table} BEGIN SELECT RAISE(ABORT,'injected bootstrap fault'); END;"
        )).unwrap();
        assert!(
            prepared
                .commit(&mut SqlitePrincipalBootstrapCustody {
                    connection: &mut connection,
                })
                .is_err(),
            "fault at {table}"
        );
        assert_empty(&connection);
        assert_eq!(
            SqlitePrincipalBootstrapCustody {
                connection: &mut connection
            }
            .observe_exact(&candidate)
            .unwrap(),
            PrincipalBootstrapDatabaseObservation::Empty
        );
    }
}

#[test]
fn exact_readback_requires_retained_nodes_and_frontier_roots() {
    for fault in [
        "UPDATE pale_principal_control_accumulator_position SET node_bytes=zeroblob(32) WHERE position_kind='leaf'",
        "DELETE FROM pale_principal_control_accumulator_position WHERE rowid=(SELECT MIN(rowid) FROM pale_principal_control_accumulator_position)",
        "UPDATE pale_principal_control_accumulator_frontier SET state_root_sha256=printf('%064d',1)",
        "DELETE FROM pale_principal_control_accumulator_frontier",
    ] {
        let mut connection = in_memory_store();
        let (prepared, _) = prepare(&mut connection);
        let candidate = prepared.candidate().clone();
        let committed = prepared.commit(&mut SqlitePrincipalBootstrapCustody { connection: &mut connection }).unwrap();
        assert_eq!(committed.phase(), PrincipalBootstrapCustodyPhase::DatabaseCommittedAnchorsPending);
        assert!(connection.execute(fault, []).unwrap() > 0);
        let before = connection.total_changes();
        let mut custody = SqlitePrincipalBootstrapCustody { connection: &mut connection };
        assert_eq!(custody.observe_exact(&candidate).unwrap(), PrincipalBootstrapDatabaseObservation::ConflictingOrPartial);
        assert_eq!(custody.compare_and_commit(&candidate).unwrap_err().kind(), ControlStoreErrorKind::RecoveryRequired);
        assert_eq!(connection.total_changes(), before, "corrupt custody is not repaired by replay");
    }
}
