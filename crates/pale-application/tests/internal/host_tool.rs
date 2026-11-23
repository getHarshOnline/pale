//! Host-tool binding proofs for observable identity, predicates, and command evidence.

use super::*;
use pale_core::records::{HostCommandConstraint, NonRootJsonPointer, ToolObservability};

fn binding(commands: Vec<HostCommandConstraint>) -> HostToolBinding {
    HostToolBinding {
        name: "custom_tool".into(),
        model_name: None,
        observability: ToolObservability::PreToolObservable,
        commands,
        command_source: None,
        input_equals: [(
            NonRootJsonPointer::try_from("/mode").expect("valid test pointer"),
            "inspect".into(),
        )]
        .into(),
    }
}

#[test]
fn input_only_observation_cannot_prove_a_command_constrained_binding() {
    let input = serde_json::json!({"mode": "inspect"});
    let command = HostCommandConstraint {
        executable: "rg".into(),
        argv_prefix: vec!["--no-config".into()],
        allow_trailing_args: true,
    };
    let effect = HostToolEffectBinding {
        effect_class: EffectClass::Read,
        tool: binding(vec![command.clone()]),
    };

    assert!(!has_matching_effect_binding(
        &[effect],
        "custom_tool",
        Some(&input)
    ));
    assert!(matches_pre_tool(
        &binding(vec![command]),
        "custom_tool",
        Some(&input),
        Some("rg --no-config needle src")
    ));
}

#[test]
fn input_only_binding_requires_exact_observable_identity_and_predicates() {
    let effect = HostToolEffectBinding {
        effect_class: EffectClass::Write,
        tool: binding(vec![]),
    };

    assert!(has_matching_effect_binding(
        std::slice::from_ref(&effect),
        "custom_tool",
        Some(&serde_json::json!({"mode": "inspect"}))
    ));
    assert!(!has_matching_effect_binding(
        &[effect],
        "custom_tool",
        Some(&serde_json::json!({"mode": "other"}))
    ));
}

#[test]
fn model_tool_name_does_not_widen_native_event_or_command_admission() {
    let mut shell = binding(vec![HostCommandConstraint {
        executable: "./verify-local".into(),
        argv_prefix: vec![],
        allow_trailing_args: false,
    }]);
    assert_eq!(shell.model_tool_name(), "custom_tool");
    assert!(serde_json::to_value(&shell)
        .unwrap()
        .get("model_name")
        .is_none());
    shell.name = "Bash".into();
    shell.model_name = Some("exec_command".into());
    shell.input_equals.clear();
    assert_eq!(shell.model_tool_name(), "exec_command");
    assert!(matches_pre_tool(
        &shell,
        "Bash",
        None,
        Some("./verify-local")
    ));
    for name in ["exec_command", "Other"] {
        assert!(!matches_pre_tool(
            &shell,
            name,
            None,
            Some("./verify-local")
        ));
    }
    assert!(!matches_pre_tool(
        &shell,
        "Bash",
        None,
        Some("./verify-local --extra")
    ));
    let encoded = serde_json::to_vec(&shell).unwrap();
    let decoded: HostToolBinding = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(decoded, shell);
}
