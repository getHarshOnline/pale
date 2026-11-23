//! Run-policy closed-value and cross-field contract tests.

use pale_core::records::{
    ChildPropagation, InvalidRunPolicy, RunPolicyEntry, RunPolicyIdV1, ToolCallMatcher,
};

fn strict_policy() -> RunPolicyEntry {
    RunPolicyEntry {
        id: RunPolicyIdV1::parse("strict").expect("test id must be valid"),
        description: "Gate selected consequential commands.".into(),
        session_inject: true,
        child_propagation: ChildPropagation::Propagate,
        child_type_selectors: vec![],
        action_gate: true,
        gated_tool_matchers: vec![ToolCallMatcher::parse("Bash(git push*)").unwrap()],
    }
}

#[test]
fn policy_relations_are_one_explicit_admission_contract() {
    strict_policy().validate_contract().unwrap();

    let mut inert = strict_policy();
    inert.action_gate = false;
    assert!(inert.validate_contract().is_err());

    let mut selectors_without_context = strict_policy();
    selectors_without_context.child_propagation = ChildPropagation::Exempt;
    selectors_without_context.child_type_selectors = vec!["reviewer*".into()];
    assert!(selectors_without_context.validate_contract().is_err());

    let mut duplicate = strict_policy();
    duplicate
        .gated_tool_matchers
        .push(ToolCallMatcher::parse("Bash(git push*)").unwrap());
    assert!(duplicate.validate_contract().is_err());
}

#[test]
fn child_propagation_is_closed_at_the_wire_boundary() {
    assert_eq!(
        serde_json::to_string(&ChildPropagation::Exempt).unwrap(),
        "\"exempt\""
    );
    assert!(serde_json::from_str::<ChildPropagation>("\"inherit-maybe\"").is_err());
}

#[test]
fn gated_tool_matcher_inventory_has_one_fixed_admission_bound() {
    let mut policy = strict_policy();
    policy.gated_tool_matchers = vec![ToolCallMatcher::parse("*").unwrap(); 257];
    assert!(matches!(
        policy.validate_contract(),
        Err(InvalidRunPolicy::TooManyToolMatchers { .. })
    ));
}
