//! Portable JCS integer-domain and lossless filesystem-coordinate contracts.

use pale_core::canon::JSON_SAFE_INTEGER_MAX;
use pale_core::records::{
    ActivationFenceGenerationV1, CheckpointGenerationV1, ControlDomainEpochV1,
    ControlDomainSequenceV1, ControlLeafCountV1, ControlTransactionSequenceV1,
    EffectConflictClaimGenerationV1, EvidenceRetentionSeconds, ExecutionControlEpochV1,
    ExecutionOwnerGenerationV1, FederationGovernanceEpochV1, FederationMembershipEpochV1,
    FilesystemDeviceIdV1, FilesystemInodeIdV1, JournalSequenceV1, LeaseBudgetUnitsV1,
    LeaseTimeoutMillisV1, OwnerSignerRecordSequenceV1, PrincipalKeyEpochV1,
    PrincipalResidentWriterGenerationV1, PrincipalRevocationEpochV1, PrincipalSessionFloorEpochV1,
    ReconciliationCaseRevisionV1, ReconciliationCustodyGenerationV1,
    RecoveryPointObjectiveSecondsV1, RecoveryTimeObjectiveSecondsV1, RouteGenerationV1,
    SessionTtlSeconds, SigningKeyEpochV1, UnixTimeV1,
};

macro_rules! assert_coordinate_domain {
    ($coordinate:ty, $minimum:expr) => {{
        let minimum = $minimum;
        assert_eq!(
            <$coordinate>::new(minimum)
                .expect("minimum coordinate")
                .get(),
            minimum
        );
        assert_eq!(
            <$coordinate>::new(JSON_SAFE_INTEGER_MAX)
                .expect("maximum coordinate")
                .get(),
            JSON_SAFE_INTEGER_MAX
        );
        assert!(
            serde_json::from_value::<$coordinate>(serde_json::json!(minimum)).is_ok(),
            "the lawful minimum must deserialize"
        );
        assert!(<$coordinate>::new(JSON_SAFE_INTEGER_MAX + 1).is_err());
        assert!(
            serde_json::from_value::<$coordinate>(serde_json::json!(JSON_SAFE_INTEGER_MAX)).is_ok()
        );
        assert!(serde_json::from_value::<$coordinate>(serde_json::json!(
            JSON_SAFE_INTEGER_MAX + 1
        ))
        .is_err());
        if minimum > 0 {
            assert!(<$coordinate>::new(0).is_err());
            assert!(serde_json::from_value::<$coordinate>(serde_json::json!(0)).is_err());
        }
    }};
}

macro_rules! assert_successor_exhaustion {
    ($coordinate:ty) => {{
        let maximum = <$coordinate>::new(JSON_SAFE_INTEGER_MAX).expect("terminal coordinate");
        assert_eq!(
            <$coordinate>::new(JSON_SAFE_INTEGER_MAX - 1)
                .expect("penultimate coordinate")
                .checked_successor(),
            Some(maximum),
            "the final lawful transition must reach the ceiling"
        );
        assert_eq!(maximum.checked_successor(), None);
    }};
}

#[test]
fn every_portable_resident_coordinate_enforces_its_exact_numeric_domain() {
    assert_coordinate_domain!(PrincipalKeyEpochV1, 1);
    assert_coordinate_domain!(SigningKeyEpochV1, 1);
    assert_coordinate_domain!(UnixTimeV1, 0);
    assert_coordinate_domain!(OwnerSignerRecordSequenceV1, 1);
    assert_coordinate_domain!(RouteGenerationV1, 1);
    assert_coordinate_domain!(PrincipalRevocationEpochV1, 1);
    assert_coordinate_domain!(ExecutionOwnerGenerationV1, 1);
    assert_coordinate_domain!(ExecutionControlEpochV1, 1);
    assert_coordinate_domain!(FederationGovernanceEpochV1, 1);
    assert_coordinate_domain!(FederationMembershipEpochV1, 1);
    assert_coordinate_domain!(EffectConflictClaimGenerationV1, 1);
    assert_coordinate_domain!(ControlDomainEpochV1, 1);
    assert_coordinate_domain!(ControlDomainSequenceV1, 1);
    assert_coordinate_domain!(ControlTransactionSequenceV1, 1);
    assert_coordinate_domain!(ControlLeafCountV1, 0);
    assert_coordinate_domain!(LeaseBudgetUnitsV1, 1);
    assert_coordinate_domain!(LeaseTimeoutMillisV1, 1);
    assert_coordinate_domain!(ReconciliationCaseRevisionV1, 0);
    assert_coordinate_domain!(ReconciliationCustodyGenerationV1, 1);
    assert_coordinate_domain!(ActivationFenceGenerationV1, 1);
    assert_coordinate_domain!(JournalSequenceV1, 1);
    assert_coordinate_domain!(CheckpointGenerationV1, 1);
    assert_coordinate_domain!(PrincipalResidentWriterGenerationV1, 1);
    assert_coordinate_domain!(PrincipalSessionFloorEpochV1, 1);
    assert_coordinate_domain!(RecoveryTimeObjectiveSecondsV1, 0);
    assert_coordinate_domain!(RecoveryPointObjectiveSecondsV1, 0);
}

#[test]
fn every_portable_successor_coordinate_stops_at_the_jcs_ceiling() {
    assert_successor_exhaustion!(EffectConflictClaimGenerationV1);
    assert_successor_exhaustion!(ControlDomainSequenceV1);
    assert_successor_exhaustion!(ReconciliationCaseRevisionV1);
    assert_successor_exhaustion!(ReconciliationCustodyGenerationV1);
    assert_successor_exhaustion!(ActivationFenceGenerationV1);
    assert_successor_exhaustion!(JournalSequenceV1);
    assert_successor_exhaustion!(CheckpointGenerationV1);
    assert_successor_exhaustion!(PrincipalResidentWriterGenerationV1);
    assert_successor_exhaustion!(PrincipalSessionFloorEpochV1);
}

#[test]
fn binding_durations_reject_zero_and_nonportable_values_on_every_entry_path() {
    for seconds in [0, JSON_SAFE_INTEGER_MAX + 1] {
        assert!(EvidenceRetentionSeconds::new(seconds).is_none());
        assert!(SessionTtlSeconds::new(seconds).is_none());
        assert!(
            serde_json::from_value::<EvidenceRetentionSeconds>(serde_json::json!(seconds)).is_err()
        );
        assert!(serde_json::from_value::<SessionTtlSeconds>(serde_json::json!(seconds)).is_err());
    }
    assert_eq!(
        EvidenceRetentionSeconds::new(JSON_SAFE_INTEGER_MAX)
            .expect("maximum retention")
            .get(),
        JSON_SAFE_INTEGER_MAX
    );
    assert_eq!(
        SessionTtlSeconds::new(JSON_SAFE_INTEGER_MAX)
            .expect("maximum TTL")
            .get(),
        JSON_SAFE_INTEGER_MAX
    );
}

#[test]
fn filesystem_coordinates_round_trip_the_full_u64_domain_as_canonical_decimal_text() {
    let maximum = u64::MAX.to_string();
    let device = FilesystemDeviceIdV1::new(u64::MAX);
    let inode = FilesystemInodeIdV1::new(u64::MAX);
    assert_eq!(
        serde_json::to_value(device).expect("device serializes"),
        maximum
    );
    assert_eq!(
        serde_json::to_value(inode).expect("inode serializes"),
        maximum
    );
    assert_eq!(
        serde_json::from_value::<FilesystemDeviceIdV1>(serde_json::json!(maximum))
            .expect("maximum device decodes")
            .get(),
        u64::MAX
    );
    assert_eq!(
        serde_json::from_value::<FilesystemInodeIdV1>(serde_json::json!("0"))
            .expect("zero inode decodes")
            .get(),
        0
    );

    for rejected in [
        serde_json::json!(0),
        serde_json::json!(""),
        serde_json::json!("00"),
        serde_json::json!("+1"),
        serde_json::json!(" 1"),
        serde_json::json!("18446744073709551616"),
    ] {
        assert!(serde_json::from_value::<FilesystemDeviceIdV1>(rejected).is_err());
    }
}
