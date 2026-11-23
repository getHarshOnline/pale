//! Captured review inputs from the managed cross-leaf conformance fixture.

use serde_json::{json, Value};

/// Package, private, and workspace roles for the common baseline.
pub(crate) fn baseline() -> Value {
    fn document(label: &str, content: &str) -> Value {
        json!({"label": label, "content": content})
    }
    json!({
        "projectId": "demo",
        "executionWorkspaceId": "default",
        "machineId": "machine",
        "privateInputs": {
            "capabilities": document("capabilities.toml", include_str!("../fixtures/cross-leaf.v1/one-shot/private/capabilities.toml")),
            "machineBinding": document("machine-binding.toml", include_str!("../fixtures/cross-leaf.v1/one-shot/private/machine-binding.toml")),
            "distributionLock": document("distribution-lock.toml", include_str!("../fixtures/cross-leaf.v1/one-shot/private/distribution-lock.toml")),
            "lifecycleGuidance": [],
        },
        "packageDocuments": [
            document("engine/package.toml", include_str!("../fixtures/cross-leaf.v1/one-shot/packages/engine/package.toml")),
            document("engine/role.md", include_str!("../fixtures/cross-leaf.v1/one-shot/packages/engine/role.md")),
            document("engine/roles.toml", include_str!("../fixtures/cross-leaf.v1/one-shot/packages/engine/roles.toml")),
        ],
        "workspace": {
            "rootDocument": "pale-workspace.toml",
            "documents": [
                document("composition.toml", include_str!("../fixtures/cross-leaf.v1/one-shot/workspace/composition.toml")),
                document("pale-workspace.toml", include_str!("../fixtures/cross-leaf.v1/one-shot/workspace/pale-workspace.toml")),
                document("workspace-resolution.lock.toml", include_str!("../fixtures/cross-leaf.v1/one-shot/workspace/workspace-resolution.lock.toml")),
                document("workspace-universe.lock.toml", include_str!("../fixtures/cross-leaf.v1/one-shot/workspace/workspace-universe.lock.toml")),
            ],
        },
    })
}

/// Independent API-owned expected review facts and candidate selectors.
pub(crate) fn expected() -> Value {
    serde_json::from_str(include_str!(
        "../fixtures/cross-leaf.v1/improvement-journey.json"
    ))
    .unwrap()
}

/// Preview input with the canonical access and machine-root candidate changes.
pub(crate) fn preview() -> Value {
    let baseline = baseline();
    let mut candidate = baseline.clone();
    candidate
        .as_object_mut()
        .unwrap()
        .remove("packageDocuments");
    let expected = expected();
    let binding = &mut candidate["privateInputs"]["machineBinding"]["content"];
    *binding = Value::String(binding.as_str().unwrap().replace(
        "/captured/repository",
        expected["candidate_machine_root"].as_str().unwrap(),
    ));
    for document in candidate["workspace"]["documents"].as_array_mut().unwrap() {
        if document["label"] == "pale-workspace.toml" {
            document["content"] = Value::String(document["content"].as_str().unwrap().replace(
                "access = \"mutate\"",
                &format!(
                    "access = \"{}\"",
                    expected["candidate_repository_access"].as_str().unwrap()
                ),
            ));
        }
    }
    json!({"action": "preview", "baseline": baseline, "candidate": candidate})
}

/// A new request reconstructs the same sources and checks the complete summary object.
pub(crate) fn finish(summary: Value, disposition: &str) -> Value {
    let mut input = preview();
    input["action"] = json!("finish");
    input["summary"] = summary;
    input["disposition"] = json!(disposition);
    input
}

/// Inspect one checked subject through the same reconstructed review.
pub(crate) fn inspect(summary: Value, subject: Value) -> Value {
    let mut input = preview();
    input["action"] = json!("inspect");
    input["summary"] = summary;
    input["subject"] = subject;
    input["inputLimit"] = json!(64);
    input["maximumInspected"] = json!(64);
    input["maximumReturned"] = json!(64);
    input
}
