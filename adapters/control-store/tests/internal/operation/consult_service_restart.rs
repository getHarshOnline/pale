//! Public Consult service recovery across a closed `SQLite` operation connection.
//! Authority history, signing and the independent anchor use explicit fixtures.

use crate::authority::DeploymentControlSqliteStore;
use crate::location::{ControlStorageRoot, ControlStoreScope, DeploymentStoreScope};
use crate::store::{initialize_unadmitted_store, ControlStoreIdentity};
use pale_core::records::{ArtifactDigest, ResidentWorkspaceIdV1};

#[test]
fn consult_service_resumes_saved_collection_after_reopening_sqlite() {
    run_service_restart(false);
}

#[test]
fn consult_service_recovers_committed_expiry_in_background_after_reopening_sqlite() {
    run_service_restart(true);
}

fn run_service_restart(expiry: bool) {
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
            if openings > 0 {
                store
                    .record_custody_connection()
                    .unwrap()
                    .execute_batch("PRAGMA query_only = ON")
                    .unwrap();
            }
            let changes = store.record_custody_connection().unwrap().total_changes();
            use_store(&mut &store);
            if openings > 0 {
                assert_eq!(
                    store.record_custody_connection().unwrap().total_changes(),
                    changes,
                    "cold service recovery must not rewrite custody"
                );
            }
            drop(store);
            openings += 1;
        };
    if expiry {
        pale_application::resident::exercise_consult_service_expiry_restart(&mut lend);
    } else {
        pale_application::resident::exercise_consult_service_restart(&mut lend);
    }
    assert_eq!(
        openings, 2,
        "each service owns a separate SQLite connection"
    );
}
