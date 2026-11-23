//! Lifecycle-behaviour record tests: safe defaults, neutral fallback carriage,
//! deny-unknown-fields, and event/axes serde round-trip.

use pale_core::records::*;

fn semantic(resource_id: &str) -> PackageResourceRefV1 {
    PackageResourceRefV1 {
        package_id: PackageIdV1::parse("example.lifecycle")
            .expect("synthetic Package id must be valid"),
        resource_id: PackageResourceIdV1::parse(resource_id)
            .expect("synthetic resource id must be valid"),
    }
}

#[test]
fn minimal_behaviour_uses_safe_defaults() {
    // A minimal behaviour (only the required fields) must deserialize with the safe
    // declared defaults — a gate is fail-closed, stateless, and always-on.
    let json = serde_json::json!({
        "id": "boundary-safety",
        "description": "block edits to protected config",
        "event": "pre-tool-use",
        "action": "deny"
    });
    let b: LifecycleBehavior = serde_json::from_value(json).expect("minimal behaviour parses");
    assert_eq!(b.predicate_kind, PredicateKind::Always);
    assert_eq!(b.failure_posture, FailurePosture::FailClosed);
    assert_eq!(b.state_scope, StateScope::Stateless);
    assert!(b.fallback.is_none());
    assert!(b.matchers.is_empty() && b.tool_reqs.is_empty() && b.modes.is_empty());
}

#[test]
fn fallback_is_carried_only_by_its_declared_event() {
    let b = LifecycleBehavior {
        id: LifecycleBehaviorIdV1::parse("capture").expect("test id must be valid"),
        description: "durable capture".into(),
        guidance: None,
        event: LifecycleEvent::PostToolUse,
        predicate_kind: PredicateKind::Always,
        matchers: vec![],
        case_insensitive: false,
        existing_targets_only: false,
        allow_admitted_operation: false,
        action: LifecycleAction::Capture,
        activate_mode: None,
        modes: vec![],
        tool_reqs: vec![],
        failure_posture: FailurePosture::FailOpen,
        state_scope: StateScope::Session,
        governed_by: None,
        timeout_ms: None,
        fallback: Some(LifecycleFallbackV1 {
            guidance: "record nothing; note the gap".into(),
            carrier_event: LifecycleEvent::SessionStart,
        }),
    };
    assert_eq!(
        b.fallback_for(LifecycleEvent::SessionStart),
        Some("record nothing; note the gap")
    );
    assert_eq!(b.fallback_for(LifecycleEvent::PostToolUse), None);
}

#[test]
fn unknown_field_is_rejected() {
    // deny_unknown_fields: a typo'd or injected field must fail, not be ignored.
    let json = serde_json::json!({
        "id": "x", "description": "d", "event": "stop", "action": "no_op",
        "bogus": true
    });
    assert!(serde_json::from_value::<LifecycleBehavior>(json).is_err());
}

#[test]
fn event_and_axes_round_trip() {
    let b = LifecycleBehavior {
        id: LifecycleBehaviorIdV1::parse("prompt-command-activation")
            .expect("test id must be valid"),
        description: "prompt-command activation".into(),
        guidance: None,
        event: LifecycleEvent::UserPromptSubmit,
        predicate_kind: PredicateKind::CommandPrefix,
        matchers: vec!["/lazy".into(), "/yagni".into()],
        case_insensitive: false,
        existing_targets_only: false,
        allow_admitted_operation: false,
        action: LifecycleAction::Activate,
        activate_mode: Some(semantic("intensity")),
        modes: vec![],
        tool_reqs: vec![],
        failure_posture: FailurePosture::FailOpen,
        state_scope: StateScope::Message,
        governed_by: None,
        timeout_ms: Some(1000),
        fallback: None,
    };
    let v = serde_json::to_value(&b).unwrap();
    assert_eq!(v["event"], "user-prompt-submit");
    assert_eq!(v["predicate_kind"], "command_prefix");
    assert_eq!(v["action"], "activate");
    assert_eq!(v["state_scope"], "message");
    let back: LifecycleBehavior = serde_json::from_value(v).unwrap();
    assert_eq!(back, b);
}

#[test]
fn lifecycle_event_inventory_round_trips_the_closed_wire_protocol() {
    for event in LifecycleEvent::vocabulary() {
        let wire = serde_json::to_string(&event).unwrap();
        assert_eq!(
            serde_json::from_str::<LifecycleEvent>(&wire).unwrap(),
            event
        );
    }
    assert!(serde_json::from_str::<LifecycleEvent>("\"unknown-event\"").is_err());
}

#[test]
fn cross_field_contract_rejects_inert_or_misclassified_options() {
    let baseline: LifecycleBehavior = serde_json::from_value(serde_json::json!({
        "id": "valid",
        "description": "valid path gate",
        "event": "pre-tool-use",
        "predicate_kind": "path_match",
        "matchers": ["*.toml"],
        "case_insensitive": true,
        "action": "deny"
    }))
    .unwrap();
    baseline.validate_contract().unwrap();

    let mut ignored_matcher = baseline.clone();
    ignored_matcher.predicate_kind = PredicateKind::Always;
    assert!(ignored_matcher.validate_contract().is_err());

    let mut malformed_tool = baseline.clone();
    malformed_tool.predicate_kind = PredicateKind::ToolMatch;
    malformed_tool.matchers = vec!["Bash(".into()];
    malformed_tool.case_insensitive = false;
    assert!(malformed_tool.validate_contract().is_err());

    let mut orphan_target = baseline;
    orphan_target.activate_mode = Some(semantic("strict"));
    assert!(orphan_target.validate_contract().is_err());
}

#[test]
fn matcher_inventories_and_values_are_bounded_before_predicate_work() {
    let mut behavior: LifecycleBehavior = serde_json::from_value(serde_json::json!({
        "id": "bounded",
        "description": "bounded matcher inventory",
        "event": "pre-tool-use",
        "predicate_kind": "exact_message",
        "matchers": ["message"],
        "action": "deny"
    }))
    .unwrap();

    behavior.matchers = vec!["message".into(); 257];
    assert!(matches!(
        behavior.validate_contract(),
        Err(InvalidLifecycleBehavior::TooManyMatchers { .. })
    ));

    behavior.matchers = vec!["雪".repeat(4_097)];
    assert!(matches!(
        behavior.validate_contract(),
        Err(InvalidLifecycleBehavior::MatcherTooLong { .. })
    ));
}

#[test]
fn lifecycle_state_and_fallbacks_cannot_contradict_their_declared_class() {
    let mut behavior: LifecycleBehavior = serde_json::from_value(serde_json::json!({
        "id": "capture",
        "description": "capture governed evidence",
        "event": "post-tool-use",
        "action": "capture",
        "state_scope": "durable",
        "governed_by": "evidence-store",
        "fallback": {
            "guidance": "carry guidance at session start",
            "carrier_event": "session-start"
        }
    }))
    .unwrap();
    behavior.validate_contract().unwrap();

    behavior.governed_by = None;
    assert!(matches!(
        behavior.validate_contract(),
        Err(InvalidLifecycleBehavior::DurableOwnerMismatch)
    ));
    behavior.governed_by = Some("evidence-store".into());

    behavior.fallback.as_mut().unwrap().guidance.clear();
    assert!(matches!(
        behavior.validate_contract(),
        Err(InvalidLifecycleBehavior::EmptyFallbackGuidance)
    ));
}
