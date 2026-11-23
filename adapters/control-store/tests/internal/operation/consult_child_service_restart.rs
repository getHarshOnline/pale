//! Public Consult service recovery across a closed `SQLite` operation connection.
//! Authority history, signing and the independent anchor use explicit fixtures.

use crate::authority::DeploymentControlSqliteStore;
use crate::location::{ControlStorageRoot, ControlStoreScope, DeploymentStoreScope};
use crate::store::{initialize_unadmitted_store, ControlStoreIdentity};
use pale_core::records::{ArtifactDigest, ResidentWorkspaceIdV1};

#[test]
fn consult_service_recovers_original_fresh_child_after_reopening_sqlite() {
    exercise_sqlite_restart(Boundary::Collecting);
}

#[test]
fn consult_service_opens_collection_after_reopening_completed_child_prefix() {
    exercise_sqlite_restart(Boundary::BeforeCollection);
}

#[test]
fn consult_service_does_not_infer_unsigned_child_from_absent_custody() {
    exercise_sqlite_restart(Boundary::BeforeChild);
}

#[test]
fn consult_service_does_not_resign_child_after_lost_signature() {
    exercise_sqlite_restart(Boundary::AfterChildSignature);
}

#[test]
fn consult_service_repairs_committed_collection_anchor_after_reopening_sqlite() {
    exercise_sqlite_restart(Boundary::AfterCollectionCommit);
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Boundary {
    Collecting,
    BeforeCollection,
    BeforeChild,
    AfterChildSignature,
    AfterCollectionCommit,
}

fn exercise_sqlite_restart(boundary: Boundary) {
    let directory = crate::test_support::private_tempdir();
    let root = ControlStorageRoot::retain(directory.path()).unwrap();
    let mut openings = 0;
    let mut lend =
        |coordinate: &pale_ports::AuthorityStoreCoordinate,
         binding_ref: &pale_core::records::ResidentRecordRefV1,
         binding: &pale_core::records::DeploymentBindingV1,
         use_store: &mut dyn FnMut(&mut dyn pale_ports::OperationCustodyPort)| {
            let scope = ControlStoreScope::Deployment(DeploymentStoreScope::new(
                ResidentWorkspaceIdV1::parse(binding.deployment_terms().workspace_id().as_str())
                    .unwrap(),
                binding_ref.clone(),
                ArtifactDigest::parse(binding.canonical_binding_sha256().as_str()).unwrap(),
            ));
            let location = root.prepare_store_location(scope.clone()).unwrap();
            if openings == 0 {
                initialize_unadmitted_store(
                    &location,
                    ControlStoreIdentity::new(
                        scope,
                        coordinate.store_id().clone(),
                        coordinate.incarnation_id().clone(),
                    ),
                )
                .unwrap();
            }
            let store = DeploymentControlSqliteStore::open_unadmitted_candidate(
                &location,
                coordinate.clone(),
            )
            .unwrap();
            if openings > 0 && boundary != Boundary::BeforeCollection {
                store
                    .record_custody_connection()
                    .unwrap()
                    .execute_batch("PRAGMA query_only = ON")
                    .unwrap();
            }
            let changes = store.record_custody_connection().unwrap().total_changes();
            use_store(&mut &store);
            if openings > 0 && boundary != Boundary::BeforeCollection {
                assert_eq!(
                    store.record_custody_connection().unwrap().total_changes(),
                    changes,
                    "cold service recovery must not rewrite custody"
                );
            }
            drop(store);
            openings += 1;
        };
    match boundary {
        Boundary::Collecting => {
            pale_application::resident::exercise_consult_child_service_restart(&mut lend)
        }
        Boundary::BeforeCollection => {
            pale_application::resident::exercise_consult_child_prefix_service_restart(&mut lend)
        }
        Boundary::BeforeChild => {
            pale_application::resident::exercise_consult_child_suffix_service_restart(&mut lend)
        }
        Boundary::AfterChildSignature => {
            pale_application::resident::exercise_consult_child_ambiguous_restart(&mut lend)
        }
        Boundary::AfterCollectionCommit => {
            pale_application::resident::exercise_consult_child_collection_anchor_restart(&mut lend)
        }
    }
    assert_eq!(
        openings, 2,
        "each service owns a separate SQLite connection"
    );
}
