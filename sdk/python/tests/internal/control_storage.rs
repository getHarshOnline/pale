//! Laws of the Python physical-inspection projection.

use super::*;
use pyo3::types::{PyAnyMethods, PyBytesMethods};

#[test]
fn requirements_preserve_the_owner_sequence_without_sorting() {
    use LocalControlStorageProfileRequirement as Requirement;
    use PyLocalControlStorageProfileRequirement as Projected;
    let pairs = [
        (
            Requirement::BackupRestorePolicy,
            Projected::BackupRestorePolicy,
        ),
        (
            Requirement::AuthorityCompactionPolicy,
            Projected::AuthorityCompactionPolicy,
        ),
        (
            Requirement::AuthorityRetentionPolicy,
            Projected::AuthorityRetentionPolicy,
        ),
        (
            Requirement::AuthorityCheckpointCadence,
            Projected::AuthorityCheckpointCadence,
        ),
        (
            Requirement::AuthorityMaximumTailEntries,
            Projected::AuthorityMaximumTailEntries,
        ),
        (
            Requirement::AuthorityMaximumCheckpointCandidates,
            Projected::AuthorityMaximumCheckpointCandidates,
        ),
        (
            Requirement::AuthorityMaximumCheckpointBytes,
            Projected::AuthorityMaximumCheckpointBytes,
        ),
        (
            Requirement::AuthorityMaximumEntryBytes,
            Projected::AuthorityMaximumEntryBytes,
        ),
        (
            Requirement::AuthorityCorruptionChecksumBytes,
            Projected::AuthorityCorruptionChecksumBytes,
        ),
        (
            Requirement::AuthorityCorruptionChecksum,
            Projected::AuthorityCorruptionChecksum,
        ),
        (
            Requirement::AuthorityFrameEncoding,
            Projected::AuthorityFrameEncoding,
        ),
        (
            Requirement::SqliteDurabilityGrouping,
            Projected::SqliteDurabilityGrouping,
        ),
        (
            Requirement::SqliteWalAutocheckpointFrames,
            Projected::SqliteWalAutocheckpointFrames,
        ),
        (
            Requirement::SqliteCheckpointMode,
            Projected::SqliteCheckpointMode,
        ),
        (
            Requirement::SqliteCheckpointFullFsync,
            Projected::SqliteCheckpointFullFsync,
        ),
        (Requirement::SqliteFullFsync, Projected::SqliteFullFsync),
        (Requirement::SqliteSynchronous, Projected::SqliteSynchronous),
        (
            Requirement::SqliteCompileOptions,
            Projected::SqliteCompileOptions,
        ),
    ];
    let state = PyLocalControlStorageInspectionState {
        inner: LocalControlStorageInspectionState::ProfileUnselected {
            unresolved: pairs.iter().map(|(owner, _)| *owner).collect(),
        },
    };
    assert_eq!(
        state.kind(),
        PyLocalControlStorageInspectionStateKind::ProfileUnselected
    );
    assert_eq!(
        state.unresolved(),
        Some(pairs.iter().map(|(_, projected)| *projected).collect())
    );
    assert!(state.rejection().is_none());
}

#[test]
fn rejection_variants_preserve_discriminants_and_only_version_evidence() {
    use LocalControlStorageInspectionRejection as Rejection;
    use PyLocalControlStorageInspectionRejectionKind as Kind;
    for (inner, kind) in [
        (Rejection::UnsafeEntryKind, Kind::UnsafeEntryKind),
        (Rejection::CorruptOrPartial, Kind::CorruptOrPartial),
        (Rejection::WrongStoreKind, Kind::WrongStoreKind),
        (Rejection::WrongLogicalStoreId, Kind::WrongLogicalStoreId),
        (
            Rejection::WrongControlIncarnation,
            Kind::WrongControlIncarnation,
        ),
        (Rejection::WrongPrincipalScope, Kind::WrongPrincipalScope),
        (Rejection::WrongDeploymentScope, Kind::WrongDeploymentScope),
        (
            Rejection::SourceIdentityMismatch,
            Kind::SourceIdentityMismatch,
        ),
        (
            Rejection::NewerVersion {
                observed: 9,
                supported: 1,
            },
            Kind::NewerVersion,
        ),
        (Rejection::SqliteBuildMismatch, Kind::SqliteBuildMismatch),
        (
            Rejection::StructuralProfileMismatch,
            Kind::StructuralProfileMismatch,
        ),
    ] {
        let state = PyLocalControlStorageInspectionState {
            inner: LocalControlStorageInspectionState::Rejected(inner),
        };
        assert_eq!(
            state.kind(),
            PyLocalControlStorageInspectionStateKind::Rejected
        );
        assert!(state.unresolved().is_none());
        let rejection = state.rejection().expect("rejected states retain evidence");
        assert_eq!(rejection.kind(), kind);
        assert_eq!(
            rejection.observed(),
            (kind == Kind::NewerVersion).then_some(9)
        );
        assert_eq!(
            rejection.supported(),
            (kind == Kind::NewerVersion).then_some(1)
        );
    }
}

#[test]
fn physical_states_are_distinct_and_contain_no_variant_payload() {
    use LocalControlStorageInspectionState as State;
    use PyLocalControlStorageInspectionStateKind as Kind;
    for (inner, kind) in [
        (State::Missing, Kind::Missing),
        (State::Empty, Kind::Empty),
        (State::Ready, Kind::Ready),
    ] {
        let state = PyLocalControlStorageInspectionState { inner };
        assert_eq!(state.kind(), kind);
        assert!(state.unresolved().is_none());
        assert!(state.rejection().is_none());
    }
}

#[test]
fn invalid_input_preserves_exact_api_diagnostic_and_redacts_private_root() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<PyLocalControlStorageInspectionRequest>();
    assert_send_sync::<PyLocalControlStorageInspection>();
    let root = "private-relative-runtime".to_owned();
    let request = PyLocalControlStorageInspectionRequest::principal(
        root.clone(),
        "principal-inspection".to_owned(),
        "store-inspection".to_owned(),
        "incarnation-inspection".to_owned(),
    );
    assert!(!format!("{request:?}").contains(&root));
    let api = Arc::new(pale_sdk::PaleApi::new().expect("embedded API initializes"));
    let expected = api
        .inspect_local_control_store(request.inner.clone())
        .expect_err("relative root is rejected");
    pyo3::Python::initialize();
    pyo3::Python::attach(|py| {
        let binding = PalePython { api };
        let error = binding
            .inspect_local_control_store(py, request)
            .err()
            .expect("same API error reaches Python");
        let bytes = error
            .value(py)
            .getattr("diagnostic_bytes")
            .expect("canonical bytes are retained");
        let bytes = bytes
            .cast::<pyo3::types::PyBytes>()
            .expect("diagnostic bytes");
        assert_eq!(bytes.as_bytes(), expected.diagnostic().canonical_bytes());
        assert!(!error.to_string().contains(&root));
    });
}
