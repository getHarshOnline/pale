//! Model-facing tool text must not replace the native matching coordinate.
use super::host_tool_contract;
use pale_core::records::{HostCommandConstraint, HostToolBinding, ToolObservability};

#[test]
fn callable_name_is_rendered_with_the_same_exact_command_ceiling() {
    let tool = HostToolBinding {
        name: "Bash".into(),
        model_name: Some("exec_command".into()),
        observability: ToolObservability::PreToolObservable,
        commands: vec![HostCommandConstraint {
            executable: "./verify-local".into(),
            argv_prefix: vec![],
            allow_trailing_args: false,
        }],
        command_source: None,
        input_equals: Default::default(),
    };
    assert_eq!(
        host_tool_contract(&tool),
        "exec_command (pre_tool_observable; commands: ./verify-local)"
    );
    assert_eq!(tool.name, "Bash");
}
