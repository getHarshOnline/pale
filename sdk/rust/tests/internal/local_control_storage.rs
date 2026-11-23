//! Internal projection laws for local control-storage inspection.

use super::*;

#[test]
fn serialized_physical_states_preserve_discriminators_and_order_without_authority() {
    use serde_json::json;
    for (state, expected) in [
        (
            LocalControlStorageInspectionState::Missing,
            json!({"kind": "missing"}),
        ),
        (
            LocalControlStorageInspectionState::Empty,
            json!({"kind": "empty"}),
        ),
        (
            LocalControlStorageInspectionState::Ready,
            json!({"kind": "ready"}),
        ),
        (
            LocalControlStorageInspectionState::ProfileUnselected {
                unresolved: vec![
                    LocalControlStorageProfileRequirement::SqliteSynchronous,
                    LocalControlStorageProfileRequirement::AuthorityFrameEncoding,
                ],
            },
            json!({"kind": "profile-unselected", "details": {"unresolved": [
                "sqlite-synchronous", "authority-frame-encoding"
            ]}}),
        ),
        (
            LocalControlStorageInspectionState::Rejected(
                LocalControlStorageInspectionRejection::NewerVersion {
                    observed: 3,
                    supported: 1,
                },
            ),
            json!({"kind": "rejected", "details": {
                "kind": "newer-version", "observed": 3, "supported": 1
            }}),
        ),
    ] {
        let value = LocalControlStorageInspection {
            kind: LocalControlStoreKind::PrincipalTrust,
            state,
        };
        assert_eq!(
            serde_json::to_value(value).unwrap(),
            json!({
                "kind": "principal-trust", "state": expected
            })
        );
    }
}

#[test]
fn profile_requirements_preserve_the_adapter_owned_order_and_identity() {
    let projected = ControlStorageProfileRequirement::ALL
        .into_iter()
        .map(LocalControlStorageProfileRequirement::from)
        .collect::<Vec<_>>();
    assert_eq!(
        projected,
        vec![
            LocalControlStorageProfileRequirement::SqliteCompileOptions,
            LocalControlStorageProfileRequirement::SqliteSynchronous,
            LocalControlStorageProfileRequirement::SqliteFullFsync,
            LocalControlStorageProfileRequirement::SqliteCheckpointFullFsync,
            LocalControlStorageProfileRequirement::SqliteCheckpointMode,
            LocalControlStorageProfileRequirement::SqliteWalAutocheckpointFrames,
            LocalControlStorageProfileRequirement::SqliteDurabilityGrouping,
            LocalControlStorageProfileRequirement::AuthorityFrameEncoding,
            LocalControlStorageProfileRequirement::AuthorityCorruptionChecksum,
            LocalControlStorageProfileRequirement::AuthorityCorruptionChecksumBytes,
            LocalControlStorageProfileRequirement::AuthorityMaximumEntryBytes,
            LocalControlStorageProfileRequirement::AuthorityMaximumCheckpointBytes,
            LocalControlStorageProfileRequirement::AuthorityMaximumCheckpointCandidates,
            LocalControlStorageProfileRequirement::AuthorityMaximumTailEntries,
            LocalControlStorageProfileRequirement::AuthorityCheckpointCadence,
            LocalControlStorageProfileRequirement::AuthorityRetentionPolicy,
            LocalControlStorageProfileRequirement::AuthorityCompactionPolicy,
            LocalControlStorageProfileRequirement::BackupRestorePolicy,
        ]
    );
}

#[test]
fn every_persistent_rejection_keeps_its_distinct_public_disposition() {
    let projected = [
        ControlStoreInspectionRejection::UnsafeEntryKind,
        ControlStoreInspectionRejection::CorruptOrPartial,
        ControlStoreInspectionRejection::WrongStoreKind,
        ControlStoreInspectionRejection::WrongLogicalStoreId,
        ControlStoreInspectionRejection::WrongControlIncarnation,
        ControlStoreInspectionRejection::WrongPrincipalScope,
        ControlStoreInspectionRejection::WrongDeploymentScope,
        ControlStoreInspectionRejection::SourceIdentityMismatch,
        ControlStoreInspectionRejection::NewerVersion {
            observed: 3,
            supported: 1,
        },
        ControlStoreInspectionRejection::SqliteBuildMismatch,
        ControlStoreInspectionRejection::StructuralProfileMismatch,
    ]
    .into_iter()
    .map(LocalControlStorageInspectionRejection::from)
    .collect::<Vec<_>>();

    assert!(matches!(
        projected.as_slice(),
        [
            LocalControlStorageInspectionRejection::UnsafeEntryKind,
            LocalControlStorageInspectionRejection::CorruptOrPartial,
            LocalControlStorageInspectionRejection::WrongStoreKind,
            LocalControlStorageInspectionRejection::WrongLogicalStoreId,
            LocalControlStorageInspectionRejection::WrongControlIncarnation,
            LocalControlStorageInspectionRejection::WrongPrincipalScope,
            LocalControlStorageInspectionRejection::WrongDeploymentScope,
            LocalControlStorageInspectionRejection::SourceIdentityMismatch,
            LocalControlStorageInspectionRejection::NewerVersion {
                observed: 3,
                supported: 1,
            },
            LocalControlStorageInspectionRejection::SqliteBuildMismatch,
            LocalControlStorageInspectionRejection::StructuralProfileMismatch,
        ]
    ));
}
