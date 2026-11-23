//! Projection laws for persistent control-store contradictions.

use super::*;

#[test]
fn persistent_rejections_keep_their_distinctions_and_version_payload() {
    use LocalControlStorageRejectionKind as Kind;
    for (inner, expected) in [
        (ApiRejection::UnsafeEntryKind, Kind::UnsafeEntryKind),
        (ApiRejection::CorruptOrPartial, Kind::CorruptOrPartial),
        (ApiRejection::WrongStoreKind, Kind::WrongStoreKind),
        (ApiRejection::WrongLogicalStoreId, Kind::WrongLogicalStoreId),
        (
            ApiRejection::WrongControlIncarnation,
            Kind::WrongControlIncarnation,
        ),
        (ApiRejection::WrongPrincipalScope, Kind::WrongPrincipalScope),
        (
            ApiRejection::WrongDeploymentScope,
            Kind::WrongDeploymentScope,
        ),
        (
            ApiRejection::SourceIdentityMismatch,
            Kind::SourceIdentityMismatch,
        ),
        (ApiRejection::SqliteBuildMismatch, Kind::SqliteBuildMismatch),
        (
            ApiRejection::StructuralProfileMismatch,
            Kind::StructuralProfileMismatch,
        ),
    ] {
        let projected = LocalControlStorageRejection { inner };
        assert_eq!(projected.kind(), expected);
        assert_eq!(projected.observed_version(), None);
        assert_eq!(projected.supported_version(), None);
    }
    let projected = LocalControlStorageRejection {
        inner: ApiRejection::NewerVersion {
            observed: u32::MAX,
            supported: 1,
        },
    };
    assert_eq!(projected.kind(), Kind::NewerVersion);
    assert_eq!(projected.observed_version(), Some(u32::MAX));
    assert_eq!(projected.supported_version(), Some(1));
}

#[test]
fn requirement_projection_preserves_owner_order_without_sorting() {
    use pale_sdk::LocalControlStorageProfileRequirement as ApiRequirement;
    let requirements = [
        ApiRequirement::SqliteSynchronous,
        ApiRequirement::AuthorityFrameEncoding,
        ApiRequirement::SqliteCompileOptions,
        ApiRequirement::AuthorityCompactionPolicy,
        ApiRequirement::BackupRestorePolicy,
        ApiRequirement::SqliteFullFsync,
        ApiRequirement::SqliteCheckpointFullFsync,
        ApiRequirement::SqliteCheckpointMode,
        ApiRequirement::SqliteWalAutocheckpointFrames,
        ApiRequirement::SqliteDurabilityGrouping,
        ApiRequirement::AuthorityCorruptionChecksum,
        ApiRequirement::AuthorityCorruptionChecksumBytes,
        ApiRequirement::AuthorityMaximumEntryBytes,
        ApiRequirement::AuthorityMaximumCheckpointBytes,
        ApiRequirement::AuthorityMaximumCheckpointCandidates,
        ApiRequirement::AuthorityMaximumTailEntries,
        ApiRequirement::AuthorityCheckpointCadence,
        ApiRequirement::AuthorityRetentionPolicy,
    ];
    assert_eq!(
        requirements
            .into_iter()
            .map(LocalControlStorageProfileRequirement::from)
            .collect::<Vec<_>>(),
        [
            LocalControlStorageProfileRequirement::SqliteSynchronous,
            LocalControlStorageProfileRequirement::AuthorityFrameEncoding,
            LocalControlStorageProfileRequirement::SqliteCompileOptions,
            LocalControlStorageProfileRequirement::AuthorityCompactionPolicy,
            LocalControlStorageProfileRequirement::BackupRestorePolicy,
            LocalControlStorageProfileRequirement::SqliteFullFsync,
            LocalControlStorageProfileRequirement::SqliteCheckpointFullFsync,
            LocalControlStorageProfileRequirement::SqliteCheckpointMode,
            LocalControlStorageProfileRequirement::SqliteWalAutocheckpointFrames,
            LocalControlStorageProfileRequirement::SqliteDurabilityGrouping,
            LocalControlStorageProfileRequirement::AuthorityCorruptionChecksum,
            LocalControlStorageProfileRequirement::AuthorityCorruptionChecksumBytes,
            LocalControlStorageProfileRequirement::AuthorityMaximumEntryBytes,
            LocalControlStorageProfileRequirement::AuthorityMaximumCheckpointBytes,
            LocalControlStorageProfileRequirement::AuthorityMaximumCheckpointCandidates,
            LocalControlStorageProfileRequirement::AuthorityMaximumTailEntries,
            LocalControlStorageProfileRequirement::AuthorityCheckpointCadence,
            LocalControlStorageProfileRequirement::AuthorityRetentionPolicy,
        ]
    );
}
