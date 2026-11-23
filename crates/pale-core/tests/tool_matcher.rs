//! Tool-call matcher wire grammar and ownership tests.

use pale_core::records::ToolCallMatcher;

#[test]
fn matcher_exposes_one_validated_identity_to_wire_and_runtime() {
    let matcher = ToolCallMatcher::parse("Bash(git push*)").unwrap();
    assert_eq!(matcher.as_str(), "Bash(git push*)");
    assert_eq!(matcher.tool(), "Bash");
    assert_eq!(matcher.argument_glob(), Some("git push*"));
    assert_eq!(
        serde_json::to_string(&matcher).unwrap(),
        "\"Bash(git push*)\""
    );
    assert_eq!(
        serde_json::from_str::<ToolCallMatcher>("\"Bash(git push*)\"").unwrap(),
        matcher
    );

    let wildcard = ToolCallMatcher::parse("*").unwrap();
    assert_eq!(wildcard.tool(), "*");
    assert_eq!(wildcard.argument_glob(), None);
}

#[test]
fn malformed_or_ambiguous_matchers_fail_before_policy_admission() {
    for invalid in [
        "",
        " Bash",
        "Bash ",
        "Bash(",
        "Bash)",
        "Bash()",
        "Bash(foo(bar))",
        "Bash(foo)tail",
        "Ba sh",
        "Bash(foo\nbar)",
    ] {
        assert!(
            ToolCallMatcher::parse(invalid).is_err(),
            "`{invalid}` must not enter a sealed run policy"
        );
        assert!(
            serde_json::from_value::<ToolCallMatcher>(serde_json::json!(invalid)).is_err(),
            "serde must enforce the same grammar for `{invalid}`"
        );
    }
}

#[test]
fn scalar_bounds_precede_storage_and_errors_redact_rejected_values() {
    let at_limit = format!("Bash({})", "*".repeat(4_090));
    assert_eq!(at_limit.chars().count(), 4_096);
    ToolCallMatcher::parse(&at_limit).expect("the exact scalar ceiling is admitted");

    let oversized = "雪".repeat(4_097);
    let error = ToolCallMatcher::parse(&oversized).expect_err("oversized matcher must fail");
    assert!(!error.to_string().contains('雪'));
    assert!(serde_json::from_value::<ToolCallMatcher>(serde_json::json!(oversized)).is_err());
}
