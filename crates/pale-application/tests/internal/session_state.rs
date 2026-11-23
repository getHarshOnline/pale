//! Internal session-state invariants that are not part of the public API.

use std::collections::BTreeMap;

use pale_core::records::{
    CapabilityRequirementIdV1, FailurePosture, OperationBindingDigest, OperationCatalogDigest,
    OperationContractDigest, PackageIdV1, PackageResourceIdV1, PackageResourceRefV1,
};

use crate::operation_admission::OperationActivation;

use super::{SessionRuntimeState, SessionStateError};

fn digest<T>(value: char) -> T
where
    T: TryFrom<String>,
    T::Error: std::fmt::Debug,
{
    T::try_from(value.to_string().repeat(64)).unwrap()
}

fn operation(id: &str) -> PackageResourceRefV1 {
    PackageResourceRefV1 {
        package_id: PackageIdV1::parse("test").unwrap(),
        resource_id: PackageResourceIdV1::parse(id).unwrap(),
    }
}

fn requirement(id: &str) -> CapabilityRequirementIdV1 {
    CapabilityRequirementIdV1::parse(id).unwrap()
}

fn state_with(fields: serde_json::Value) -> serde_json::Value {
    let mut state = serde_json::to_value(SessionRuntimeState::default()).unwrap();
    state
        .as_object_mut()
        .unwrap()
        .extend(fields.as_object().unwrap().clone());
    state
}

#[test]
fn recency_state_is_collision_safe_bounded_and_unique() {
    let mut state = SessionRuntimeState::default();
    let long = "x".repeat(600);
    let digest_key = format!("sha256:{}", pale_core::sha256_hex(long.as_bytes()));

    assert!(matches!(
        state.note_first_touch(&long),
        Some(super::StateEffect::FirstTouch { key }) if key == digest_key
    ));
    assert!(matches!(
        state.note_first_touch(&digest_key),
        Some(super::StateEffect::FirstTouch { key }) if key == format!("text:{digest_key}")
    ));
    assert_eq!(state.note_first_touch(&long), None);

    for index in 0..1_100 {
        let _ = state.note_first_touch(&format!("src/generated/{index}.rs"));
    }
    for index in 0..160 {
        let _ = state.capture(&format!("capture-{index}"));
    }
    let _ = state.capture("capture-40");
    assert_eq!(state.capture("capture-40"), None);

    let persisted = serde_json::to_value(&state).unwrap();
    let touched = persisted["touched"].as_array().unwrap();
    let captures = persisted["captures"].as_array().unwrap();
    assert_eq!(touched.len(), 1_024);
    assert_eq!(captures.len(), 128);
    assert_eq!(captures.last(), Some(&serde_json::json!("capture-40")));
    assert_eq!(
        captures
            .iter()
            .filter(|entry| entry.as_str() == Some("capture-40"))
            .count(),
        1
    );

    assert!(
        serde_json::from_value::<SessionRuntimeState>(state_with(serde_json::json!({
            "captures": ["same", "same"]
        })))
        .is_err()
    );
    assert!(
        serde_json::from_value::<SessionRuntimeState>(state_with(serde_json::json!({
            "captures": ["x".repeat(513)]
        })))
        .is_err()
    );
}

#[test]
fn repeated_activation_fences_stale_leases_and_failure_posture_only_strengthens() {
    let activation = || OperationActivation {
        operation_id: operation("review"),
        contract_digest: digest::<OperationContractDigest>('a'),
        catalog_digest: digest::<OperationCatalogDigest>('b'),
        binding_digest: digest::<OperationBindingDigest>('c'),
        turn_digest: None,
        authorized_actions: BTreeMap::new(),
    };
    let mut state = SessionRuntimeState::default();
    let first = state.activate_operation(activation()).unwrap();
    let mut lease = state.activate_operation(activation()).unwrap();
    assert_ne!(
        first, lease,
        "repeated activation must receive a fresh epoch"
    );
    assert!(!state
        .replace_operation_if_current(Some(&first), None)
        .unwrap());

    lease
        .record_failure(requirement("optional-lint"), FailurePosture::FailOpen)
        .unwrap();
    lease
        .record_failure(requirement("compile"), FailurePosture::Retry)
        .unwrap();
    lease
        .record_failure(requirement("compile"), FailurePosture::Retry)
        .unwrap();
    let before = lease.clone();

    assert_eq!(
        lease.record_failure(requirement("tests"), FailurePosture::FailClosed),
        Err(SessionStateError::OperationFailureSticky)
    );
    assert_eq!(lease, before);
}
