//! Persistence and transition journeys for bounded session continuity.

use pale_application::session_state::{
    SessionRuntimeState, SessionStateError, StateEffect, VerificationRequirement,
};
use pale_core::records::{ModeAxis, PackageIdV1, PackageResourceIdV1, PackageResourceRefV1};

fn resource(id: &str) -> PackageResourceRefV1 {
    PackageResourceRefV1 {
        package_id: PackageIdV1::parse("test").expect("test Package identity is valid"),
        resource_id: PackageResourceIdV1::parse(id).expect("test resource identity is valid"),
    }
}

fn selected_mode(state: &SessionRuntimeState, axis: ModeAxis) -> Option<&PackageResourceRefV1> {
    state.active_modes().get(&axis)
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
fn run_modes_change_independent_axes_and_reject_invalid_sets_atomically() {
    let defaults = [
        (ModeAxis::Intensity, resource("full")),
        (ModeAxis::TaskStance, resource("dev")),
    ];
    let mut state = SessionRuntimeState::default();
    state.ensure_defaults(&defaults).unwrap();

    assert_eq!(
        state
            .activate(ModeAxis::Intensity, &resource("lite"))
            .unwrap(),
        Some(StateEffect::Activated {
            axis: ModeAxis::Intensity,
            mode: resource("lite"),
        })
    );
    assert_eq!(
        selected_mode(&state, ModeAxis::Intensity),
        Some(&resource("lite"))
    );
    assert_eq!(
        selected_mode(&state, ModeAxis::TaskStance),
        Some(&resource("dev"))
    );
    assert_eq!(
        state
            .activate(ModeAxis::Intensity, &resource("lite"))
            .unwrap(),
        None
    );

    let before = state.clone();

    assert_eq!(
        state.restore_defaults(&[
            (ModeAxis::Intensity, resource("full")),
            (ModeAxis::Intensity, resource("lite")),
        ]),
        Err(SessionStateError::ActiveModesBound)
    );
    assert_eq!(state, before);

    assert_eq!(
        state.restore_defaults(&defaults).unwrap(),
        Some(StateEffect::DefaultsRestored)
    );
    assert_eq!(
        selected_mode(&state, ModeAxis::Intensity),
        Some(&resource("full"))
    );
    assert_eq!(
        selected_mode(&state, ModeAxis::TaskStance),
        Some(&resource("dev"))
    );
    assert_eq!(state.restore_defaults(&defaults).unwrap(), None);
}

#[test]
fn optimistic_epochs_preserve_newer_and_stronger_continuity_debt() {
    let mut state = SessionRuntimeState::default();
    assert_eq!(
        state
            .mark_verification_pending(VerificationRequirement::ProtectedConfiguration)
            .unwrap(),
        StateEffect::VerificationPending {
            epoch: 1,
            requirement: VerificationRequirement::ProtectedConfiguration,
        }
    );
    assert_eq!(
        state.mark_verification_prompted(1),
        Some(StateEffect::VerificationPrompted { epoch: 1 })
    );
    assert_eq!(state.mark_verification_prompted(1), None);

    assert_eq!(
        state
            .mark_verification_pending(VerificationRequirement::Standard)
            .unwrap(),
        StateEffect::VerificationPending {
            epoch: 2,
            requirement: VerificationRequirement::ProtectedConfiguration,
        }
    );
    assert_eq!(state.clear_verification_if_epoch(1), None);
    assert_eq!(state.verification_debt().unwrap().epoch(), 2);
    assert_eq!(
        state.verification_debt().unwrap().requirement(),
        VerificationRequirement::ProtectedConfiguration
    );
    assert_eq!(
        state.clear_verification_if_epoch(2),
        Some(StateEffect::VerificationCleared { epoch: 2 })
    );

    assert_eq!(
        state.mark_rebaseline_pending().unwrap(),
        StateEffect::RebaselinePending { epoch: 1 }
    );
    assert_eq!(
        state.mark_rebaseline_pending().unwrap(),
        StateEffect::RebaselinePending { epoch: 2 }
    );
    assert_eq!(state.clear_rebaseline_if_epoch(1), None);
    assert_eq!(state.pending_rebaseline_epoch(), Some(2));
    assert_eq!(
        state.clear_rebaseline_if_epoch(2),
        Some(StateEffect::RebaselineCleared { epoch: 2 })
    );
}

#[test]
fn persisted_debt_must_be_nonzero_and_match_the_latest_epoch() {
    for invalid in [
        state_with(serde_json::json!({
            "verification_epoch": 2,
            "verification_debt": { "epoch": 1, "requirement": "standard" }
        })),
        state_with(serde_json::json!({
            "verification_epoch": 1,
            "verification_debt": { "epoch": 1 }
        })),
        state_with(serde_json::json!({
            "verification_epoch": 0,
            "verification_debt": { "epoch": 0, "requirement": "standard" }
        })),
        state_with(serde_json::json!({
            "rebaseline_epoch": 1,
            "rebaseline_debt": { "epoch": 2 }
        })),
        state_with(serde_json::json!({
            "rebaseline_epoch": 0,
            "rebaseline_debt": { "epoch": 0 }
        })),
    ] {
        assert!(serde_json::from_value::<SessionRuntimeState>(invalid).is_err());
    }

    let mut state = SessionRuntimeState::default();
    let _ = state
        .mark_verification_pending(VerificationRequirement::Standard)
        .unwrap();
    let _ = state.mark_rebaseline_pending().unwrap();
    let persisted = serde_json::to_value(&state).unwrap();
    assert_eq!(persisted["verification_debt"]["epoch"], 1);
    assert_eq!(persisted["rebaseline_debt"]["epoch"], 1);
    assert!(serde_json::from_value::<SessionRuntimeState>(persisted).is_ok());
}

#[test]
fn active_operation_rejects_missing_or_misaligned_activation_identity() {
    for invalid in [
        state_with(serde_json::json!({
            "operation_epoch": 1,
            "active_operation": {
                "operation_id": "review",
                "contract_digest": "a".repeat(64),
                "catalog_digest": "b".repeat(64),
                "binding_digest": "c".repeat(64)
            }
        })),
        state_with(serde_json::json!({
            "operation_epoch": 2,
            "active_operation": {
                "activation_epoch": 1,
                "operation_id": "review",
                "contract_digest": "a".repeat(64),
                "catalog_digest": "b".repeat(64),
                "binding_digest": "c".repeat(64)
            }
        })),
    ] {
        assert!(serde_json::from_value::<SessionRuntimeState>(invalid).is_err());
    }
}
