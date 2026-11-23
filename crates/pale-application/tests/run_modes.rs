//! Host-local run-mode command and session-state conformance.

use pale_application::hooks::{resolve_mode_command, ModeCommand};
use pale_application::session_state::{SessionRuntimeState, StateEffect};
use pale_core::records::{
    ModeAxis, PackageIdV1, PackageResourceIdV1, PackageResourceRefV1, SealedRunMode,
};

fn resource(id: &str) -> PackageResourceRefV1 {
    PackageResourceRefV1 {
        package_id: PackageIdV1::parse("pkg").expect("test Package identity is valid"),
        resource_id: PackageResourceIdV1::parse(id).expect("test resource identity is valid"),
    }
}

fn mode(id: &str, axis: ModeAxis, aliases: &[&str], exact_triggers: &[&str]) -> SealedRunMode {
    SealedRunMode {
        id: resource(id),
        axis,
        aliases: aliases.iter().map(|value| (*value).to_string()).collect(),
        exact_triggers: exact_triggers
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        body: None,
    }
}

fn modes() -> Vec<SealedRunMode> {
    vec![
        mode("modes.full", ModeAxis::Intensity, &["/full"], &["Do more!"]),
        mode("modes.lite", ModeAxis::Intensity, &["/lite"], &["Do less!"]),
        mode(
            "modes.review",
            ModeAxis::TaskStance,
            &["/review"],
            &["Review mode."],
        ),
    ]
}

#[test]
fn declared_alias_or_exact_trigger_activates_only_its_axis() {
    let modes = modes();
    assert_eq!(
        resolve_mode_command("/lite", &modes),
        Some(ModeCommand::Activate {
            axis: ModeAxis::Intensity,
            id: resource("modes.lite"),
        })
    );
    assert_eq!(
        resolve_mode_command("Do less!", &modes),
        Some(ModeCommand::Activate {
            axis: ModeAxis::Intensity,
            id: resource("modes.lite"),
        })
    );

    let mut state = SessionRuntimeState::default();
    state
        .ensure_defaults(&[
            (ModeAxis::Intensity, resource("modes.full")),
            (ModeAxis::TaskStance, resource("modes.review")),
        ])
        .expect("qualified defaults are coherent");
    let Some(ModeCommand::Activate { axis, id }) = resolve_mode_command("/lite", &modes) else {
        panic!("declared alias must activate one mode")
    };
    assert_eq!(
        state.activate(axis, &id).expect("activation is bounded"),
        Some(StateEffect::Activated {
            axis: ModeAxis::Intensity,
            mode: resource("modes.lite"),
        })
    );
    assert_eq!(
        state.active_modes().get(&ModeAxis::TaskStance).cloned(),
        Some(resource("modes.review")),
        "an intensity transition cannot perturb the independent task-stance axis"
    );
}

#[test]
fn undeclared_reset_phrases_have_no_magic_engine_meaning() {
    let modes = modes();
    for message in ["/mode reset", "normal mode", "reset all modes"] {
        assert_eq!(resolve_mode_command(message, &modes), None);
    }
}
