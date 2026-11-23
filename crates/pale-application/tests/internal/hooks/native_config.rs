//! Native hook-configuration merge and removal laws.

use crate::terminal_adapters::TerminalAdapterDescriptor;
use pale_core::records::{
    ArtifactDigest, LifecycleEvent, LogicalRepositoryIdV1, ProviderBindingIdentityV1,
    ProviderBindingRefV1, WorkspaceId,
};
use serde_json::{json, Value};

use super::{
    merge_codex_hook_config, merge_native_json_hook_config, remove_codex_hook_config,
    remove_native_json_hook_config, CodexHookRepresentation, RepositoryHookRoutingV1,
};

fn provider() -> ProviderBindingRefV1 {
    ProviderBindingRefV1 {
        provider_binding_id: "terminal-provider".into(),
        provider_binding_identity: ProviderBindingIdentityV1 {
            provider_binding_sha256: ArtifactDigest::hash_bytes(b"terminal-provider"),
        },
    }
}

fn routing(event: LifecycleEvent) -> RepositoryHookRoutingV1 {
    RepositoryHookRoutingV1::new(
        event,
        WorkspaceId::parse("ws.main").unwrap(),
        LogicalRepositoryIdV1::parse("repo.main").unwrap(),
        provider(),
    )
    .unwrap()
}

fn claude_handler(executable: &str, event: LifecycleEvent) -> Value {
    json!({
        "type": "command",
        "command": executable,
        "args": routing(event).arguments(),
    })
}

fn claude_contribution(executable: &str, events: &[LifecycleEvent]) -> String {
    let hooks = events
        .iter()
        .map(|event| {
            (
                super::host_event_name(*event).to_owned(),
                json!([{ "hooks": [claude_handler(executable, *event)] }]),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    serde_json::to_string(&json!({ "hooks": hooks })).unwrap()
}

fn codex_contribution(executable: &str) -> String {
    let command = std::iter::once(executable.to_owned())
        .chain(routing(LifecycleEvent::PreToolUse).arguments())
        .map(|word| crate::host_command::quote_posix_word(&word))
        .collect::<Vec<_>>()
        .join(" ");
    serde_json::to_string(&json!({
        "hooks": {
            "PreToolUse": [{ "hooks": [{
                "type": "command",
                "command": command,
            }] }]
        }
    }))
    .unwrap()
}

fn timed_contribution(
    executable: &str,
    event: LifecycleEvent,
    harness: TerminalAdapterDescriptor,
    timeout: u64,
) -> String {
    let routing = routing(event);
    let handler = if harness.is_claude_code() {
        json!({
            "type": "command",
            "command": executable,
            "args": routing.arguments(),
            "timeout": timeout,
        })
    } else {
        let command = std::iter::once(executable.to_owned())
            .chain(routing.arguments())
            .map(|word| crate::host_command::quote_posix_word(&word))
            .collect::<Vec<_>>()
            .join(" ");
        json!({
            "type": "command",
            "command": command,
            "timeout": timeout,
        })
    };
    serde_json::to_string(&json!({
        "hooks": {
            (super::host_event_name(event)): [{ "hooks": [handler] }]
        }
    }))
    .unwrap()
}

#[test]
fn claude_merge_preserves_native_owner_state_and_replaces_only_its_identity() {
    let existing = json!({
        "permissions": { "allow": ["Read"] },
        "env": { "OWNER_VALUE": "preserved" },
        "hooks": {
            "PreToolUse": [
                { "matcher": "Bash", "hooks": [{ "type": "command", "command": "owner-check" }] },
                { "hooks": [{
                    "type": "command",
                    "command": "/old/pale",
                    "args": routing(LifecycleEvent::PreToolUse).arguments(),
                }] }
            ],
            "OwnerExtension": [{ "custom": true }]
        }
    });
    let desired = claude_contribution(
        "/new/pale",
        &[LifecycleEvent::PreToolUse, LifecycleEvent::PostToolUse],
    );
    let merged =
        merge_native_json_hook_config(Some(&serde_json::to_string(&existing).unwrap()), &desired)
            .unwrap();
    let merged: Value = serde_json::from_str(&merged).unwrap();

    assert_eq!(merged["permissions"], existing["permissions"]);
    assert_eq!(merged["env"], existing["env"]);
    assert_eq!(
        merged["hooks"]["OwnerExtension"],
        existing["hooks"]["OwnerExtension"]
    );
    assert_eq!(merged["hooks"]["PreToolUse"].as_array().unwrap().len(), 2);
    assert_eq!(
        merged["hooks"]["PreToolUse"][1]["hooks"][0]["command"],
        "/new/pale"
    );
    assert!(merged["hooks"].get("PostToolUse").is_some());
}

#[test]
fn merge_is_idempotent_for_both_native_command_encodings() {
    let claude = claude_contribution("pale", &[LifecycleEvent::PreToolUse, LifecycleEvent::Stop]);
    let once = merge_native_json_hook_config(None, &claude).unwrap();
    let twice = merge_native_json_hook_config(Some(&once), &claude).unwrap();
    assert_eq!(once, twice);

    let codex = codex_contribution("/opt/pale bin/pale");
    let once = merge_native_json_hook_config(None, &codex).unwrap();
    let twice = merge_native_json_hook_config(Some(&once), &codex).unwrap();
    assert_eq!(once, twice);
}

#[test]
fn removal_leaves_unrelated_handlers_and_settings_intact() {
    let desired = claude_contribution("pale", &[LifecycleEvent::PreToolUse]);
    let merged = merge_native_json_hook_config(
        Some(r#"{"theme":"owner","hooks":{"Stop":[{"hooks":[{"type":"command","command":"owner-stop"}]}]}}"#),
        &desired,
    )
    .unwrap();
    let removed = remove_native_json_hook_config(
        &merged,
        &WorkspaceId::parse("ws.main").unwrap(),
        &LogicalRepositoryIdV1::parse("repo.main").unwrap(),
        &provider(),
    )
    .unwrap();
    let removed: Value = serde_json::from_str(&removed).unwrap();
    assert_eq!(removed["theme"], "owner");
    assert_eq!(
        removed["hooks"]["Stop"][0]["hooks"][0]["command"],
        "owner-stop"
    );
    assert!(removed["hooks"].get("PreToolUse").is_none());
}

#[test]
fn json_merge_and_removal_preserve_owner_extended_pale_like_handlers() {
    let event = LifecycleEvent::PreToolUse;
    let mut owner_handler = claude_handler("pale", event);
    owner_handler["statusMessage"] = Value::String("Owner presentation".into());
    let existing = serde_json::to_string(&json!({
        "hooks": {
            "PreToolUse": [{ "hooks": [owner_handler] }]
        }
    }))
    .unwrap();
    let contribution = claude_contribution("pale", &[event]);

    let once = merge_native_json_hook_config(Some(&existing), &contribution).unwrap();
    let twice = merge_native_json_hook_config(Some(&once), &contribution).unwrap();
    assert_eq!(once, twice);
    let merged: Value = serde_json::from_str(&once).unwrap();
    let groups = merged["hooks"]["PreToolUse"].as_array().unwrap();
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0]["hooks"][0]["statusMessage"], "Owner presentation");
    assert!(groups[1]["hooks"][0].get("statusMessage").is_none());

    let removed = remove_native_json_hook_config(
        &once,
        &WorkspaceId::parse("ws.main").unwrap(),
        &LogicalRepositoryIdV1::parse("repo.main").unwrap(),
        &provider(),
    )
    .unwrap();
    let removed: Value = serde_json::from_str(&removed).unwrap();
    let groups = removed["hooks"]["PreToolUse"].as_array().unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0]["hooks"][0]["statusMessage"], "Owner presentation");
}

#[test]
fn merge_rejects_ambiguous_or_non_contribution_input() {
    let desired = claude_contribution("pale", &[LifecycleEvent::PreToolUse]);
    assert!(merge_native_json_hook_config(Some(r#"{"hooks":{},"hooks":{}}"#), &desired,).is_err());
    assert!(
        merge_native_json_hook_config(Some(r#"{"hooks":{"PreToolUse":{}}}"#), &desired,).is_err()
    );
    assert!(merge_native_json_hook_config(
        None,
        r#"{"hooks":{"PreToolUse":[{"hooks":[{"type":"command","command":"pale","args":["hook","pre-tool-use","--workspace","ws.main","--repository","repo.main","--provider-binding","provider-binding.codex"]}]}]}}"#,
    )
    .is_err());
    assert!(merge_native_json_hook_config(
        None,
        r#"{"hooks":{"UserPromptExpansion":[{"hooks":[{"type":"command","command":"'pale' 'hook' 'user-prompt-expansion' '--workspace' 'ws.main' '--repository' 'repo.main' '--provider-binding' 'provider-binding.codex'"}]}]}}"#,
    )
    .is_err());

    let mut extended: Value = serde_json::from_str(&desired).unwrap();
    extended["hooks"]["PreToolUse"][0]["hooks"][0]["statusMessage"] =
        Value::String("must remain native-owner state".into());
    assert!(
        merge_native_json_hook_config(None, &serde_json::to_string(&extended).unwrap(),).is_err()
    );
}

#[test]
fn contribution_admission_enforces_each_host_event_timeout_ceiling() {
    for (harness, maximum) in [
        (TerminalAdapterDescriptor::codex(), 3_u64),
        (TerminalAdapterDescriptor::claude_code(), 60_u64),
    ] {
        let admitted = timed_contribution("pale", LifecycleEvent::SessionEnd, harness, maximum);
        merge_native_json_hook_config(None, &admitted).unwrap();

        let excessive =
            timed_contribution("pale", LifecycleEvent::SessionEnd, harness, maximum + 1);
        assert!(merge_native_json_hook_config(None, &excessive).is_err());
    }
}

#[test]
fn codex_inline_merge_is_idempotent_and_preserves_native_toml() {
    let existing = r#"# repository-owned configuration
model = "gpt-owner"

[features]
hooks = true

[[hooks.PreToolUse]]
matcher = "^Bash$"

[[hooks.PreToolUse.hooks]]
type = "command"
command = "owner-check"
statusMessage = "Owner policy"
"#;
    let contribution = codex_contribution("/opt/pale bin/pale");
    let once = merge_codex_hook_config(None, Some(existing), &contribution).unwrap();
    assert_eq!(
        once.representation(),
        CodexHookRepresentation::InlineConfigToml
    );
    assert!(once.content().contains("# repository-owned configuration"));
    assert!(once.content().contains("statusMessage = \"Owner policy\""));

    let decoded: toml::Value = toml::from_str(once.content()).unwrap();
    let groups = decoded["hooks"]["PreToolUse"].as_array().unwrap();
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0]["matcher"].as_str(), Some("^Bash$"));
    let generated = groups[1]["hooks"][0]["command"].as_str().unwrap();
    assert!(generated.contains("'--provider-binding' 'terminal-provider'"));
    assert!(generated.contains("'--provider-binding-identity'"));

    let twice = merge_codex_hook_config(None, Some(once.content()), &contribution).unwrap();
    assert_eq!(once, twice);
}

#[test]
fn codex_inline_removal_preserves_unrelated_settings_and_handlers() {
    let existing = r#"theme = "owner"

[[hooks.PreToolUse]]
matcher = "^Bash$"

[[hooks.PreToolUse.hooks]]
type = "command"
command = "owner-check"
"#;
    let contribution = codex_contribution("pale");
    let merged = merge_codex_hook_config(None, Some(existing), &contribution).unwrap();
    let removed = remove_codex_hook_config(
        None,
        Some(merged.content()),
        &WorkspaceId::parse("ws.main").unwrap(),
        &LogicalRepositoryIdV1::parse("repo.main").unwrap(),
        &provider(),
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        removed.representation(),
        CodexHookRepresentation::InlineConfigToml
    );
    let decoded: toml::Value = toml::from_str(removed.content()).unwrap();
    assert_eq!(decoded["theme"].as_str(), Some("owner"));
    let groups = decoded["hooks"]["PreToolUse"].as_array().unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0]["matcher"].as_str(), Some("^Bash$"));
    assert_eq!(
        groups[0]["hooks"][0]["command"].as_str(),
        Some("owner-check")
    );
}

#[test]
fn codex_inline_merge_and_removal_preserve_owner_extended_pale_like_handlers() {
    let contribution = codex_contribution("pale");
    let contribution_json: Value = serde_json::from_str(&contribution).unwrap();
    let command = contribution_json["hooks"]["PreToolUse"][0]["hooks"][0]["command"]
        .as_str()
        .unwrap();
    let existing = format!(
        "[[hooks.PreToolUse]]\n\
         [[hooks.PreToolUse.hooks]]\n\
         type = \"command\"\n\
         command = {command:?}\n\
         statusMessage = \"Owner presentation\"\n"
    );

    let once = merge_codex_hook_config(None, Some(&existing), &contribution).unwrap();
    let twice = merge_codex_hook_config(None, Some(once.content()), &contribution).unwrap();
    assert_eq!(once, twice);
    let merged: toml::Value = toml::from_str(once.content()).unwrap();
    let handlers = merged["hooks"]["PreToolUse"].as_array().unwrap();
    assert_eq!(handlers.len(), 2);
    assert_eq!(
        handlers[0]["hooks"][0]["statusMessage"].as_str(),
        Some("Owner presentation")
    );
    assert!(handlers[1]["hooks"][0].get("statusMessage").is_none());

    let removed = remove_codex_hook_config(
        None,
        Some(once.content()),
        &WorkspaceId::parse("ws.main").unwrap(),
        &LogicalRepositoryIdV1::parse("repo.main").unwrap(),
        &provider(),
    )
    .unwrap()
    .unwrap();
    let removed: toml::Value = toml::from_str(removed.content()).unwrap();
    let handlers = removed["hooks"]["PreToolUse"].as_array().unwrap();
    assert_eq!(handlers.len(), 1);
    assert_eq!(
        handlers[0]["hooks"][0]["statusMessage"].as_str(),
        Some("Owner presentation")
    );
}

#[test]
fn codex_representation_selection_rejects_dual_sources() {
    let contribution = codex_contribution("pale");
    assert!(
        merge_codex_hook_config(Some(r#"{"hooks":{}}"#), Some("[hooks]\n"), &contribution,)
            .is_err()
    );

    let edit = merge_codex_hook_config(None, Some("model = \"owner\"\n"), &contribution).unwrap();
    assert_eq!(edit.representation(), CodexHookRepresentation::HooksJson);
    assert!(remove_codex_hook_config(
        None,
        Some("model = \"owner\"\n"),
        &WorkspaceId::parse("ws.main").unwrap(),
        &LogicalRepositoryIdV1::parse("repo.main").unwrap(),
        &provider(),
    )
    .unwrap()
    .is_none());
}

#[test]
fn codex_inline_merge_rejects_uneditable_hook_shapes() {
    let contribution = codex_contribution("pale");
    assert!(
        merge_codex_hook_config(None, Some("hooks = { PreToolUse = [] }\n"), &contribution,)
            .is_err()
    );
    assert!(merge_codex_hook_config(
        None,
        Some("[[hooks.PreToolUse]]\nhooks = [{ type = \"command\", command = \"owner\" }]\n",),
        &contribution,
    )
    .is_err());
}
