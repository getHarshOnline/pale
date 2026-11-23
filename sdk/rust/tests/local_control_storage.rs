//! Native-local physical control-store inspection behavior.

#![cfg(feature = "native-local")]
use std::os::unix::fs::PermissionsExt;

use pale_sdk::{
    DiagnosticDisposition, DiagnosticEffectCertainty, DiagnosticRetry,
    LocalControlStorageInspectionRequest, LocalControlStorageInspectionState,
    LocalControlStoreKind, PaleApi,
};

fn private_tempdir() -> tempfile::TempDir {
    let root = std::fs::canonicalize(std::env::temp_dir()).expect("temporary root is canonical");
    let directory = tempfile::tempdir_in(root).expect("temporary runtime root is created");
    std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700))
        .expect("runtime root is private");
    directory
}

fn principal_request(root: String) -> LocalControlStorageInspectionRequest {
    LocalControlStorageInspectionRequest::principal(
        root,
        "principal-inspection".to_owned(),
        "store-inspection".to_owned(),
        "incarnation-inspection".to_owned(),
    )
}

#[test]
fn missing_store_is_observed_without_creating_canonical_descendants() {
    let directory = private_tempdir();
    let root = directory.path().to_string_lossy().into_owned();
    let api = PaleApi::new().expect("embedded API dependencies are coherent");

    let inspection = api
        .inspect_local_control_store(principal_request(root.clone()))
        .expect("a missing canonical store is a valid physical observation");

    assert_eq!(inspection.kind(), LocalControlStoreKind::PrincipalTrust);
    assert_eq!(
        inspection.state(),
        &LocalControlStorageInspectionState::Missing
    );
    assert!(!directory.path().join("pale").exists());
    assert!(!format!("{:?}", principal_request(root.clone())).contains(&root));
    assert!(!format!("{inspection:?}").contains(&root));
}

#[test]
fn malformed_or_relative_coordinates_use_one_non_effectful_diagnostic_owner() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    for request in [
        principal_request("relative/runtime".to_owned()),
        LocalControlStorageInspectionRequest::principal(
            "/tmp".to_owned(),
            "".to_owned(),
            "store-inspection".to_owned(),
            "incarnation-inspection".to_owned(),
        ),
    ] {
        let error = api
            .inspect_local_control_store(request)
            .expect_err("invalid local inspection coordinates fail before host observation");
        assert_eq!(
            error.diagnostic().code(),
            "local_control_storage_inspection_failed"
        );
        assert_eq!(
            error.diagnostic().disposition(),
            DiagnosticDisposition::InvalidInput
        );
        assert_eq!(error.diagnostic().retry(), DiagnosticRetry::AfterCorrection);
        assert_eq!(
            error.diagnostic().effect(),
            DiagnosticEffectCertainty::NotApplicable
        );
    }
}

#[test]
fn deployment_scope_remains_distinct_at_the_public_boundary() {
    let directory = private_tempdir();
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let inspection = api
        .inspect_local_control_store(LocalControlStorageInspectionRequest::deployment(
            directory.path().to_string_lossy().into_owned(),
            "workspace-inspection".to_owned(),
            "deployment-binding-inspection".to_owned(),
            "0".repeat(64),
            "store-inspection".to_owned(),
            "incarnation-inspection".to_owned(),
        ))
        .expect("a missing deployment store is a valid physical observation");

    assert_eq!(inspection.kind(), LocalControlStoreKind::DeploymentControl);
    assert_eq!(
        inspection.state(),
        &LocalControlStorageInspectionState::Missing
    );
}

#[test]
fn unsafe_persistent_descendant_is_blocked_and_redacts_its_host_path() {
    let directory = private_tempdir();
    let unsafe_descendant = directory.path().join("pale");
    std::fs::create_dir(&unsafe_descendant).expect("unsafe descendant fixture is created");
    std::fs::set_permissions(&unsafe_descendant, std::fs::Permissions::from_mode(0o755))
        .expect("unsafe descendant fixture is world-readable");
    let root = directory.path().to_string_lossy().into_owned();
    let api = PaleApi::new().expect("embedded API dependencies are coherent");

    let error = api
        .inspect_local_control_store(principal_request(root.clone()))
        .expect_err("an unsafe persistent descendant must fail closed");

    assert_eq!(
        error.diagnostic().disposition(),
        DiagnosticDisposition::Blocked
    );
    assert_eq!(
        error.diagnostic().retry(),
        DiagnosticRetry::AfterStateChange
    );
    assert_eq!(
        error.diagnostic().effect(),
        DiagnosticEffectCertainty::NotApplicable
    );
    assert!(!format!("{error:?}").contains(&root));
}
