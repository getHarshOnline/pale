//! Canonical captured inputs shared by native Go bridge custody tests.

use super::*;

pub(super) struct AdmittedProfile {
    pub(super) client: u64,
    pub(super) workspace: u64,
    pub(super) profile: u64,
    pub(super) profile_request: Vec<u8>,
    pub(super) candidate_request: pale_sdk::ConfigurationImprovementCandidateRequest,
}

pub(super) fn captured_one_shot_request() -> serde_json::Value {
    use base64::Engine as _;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/cross-leaf.v1/one-shot");
    let document = |label: &str, path: &str| {
        serde_json::json!({
            "label": label,
            "bytes": base64::engine::general_purpose::STANDARD.encode(
                std::fs::read(root.join(path)).expect("canonical fixture member")),
        })
    };
    serde_json::json!({
        "projectId": "demo", "executionWorkspaceId": "default", "machineId": "machine",
        "privateInputs": {
            "capabilities": document("capabilities.toml", "private/capabilities.toml"),
            "machineBinding": document("machines/machine.toml", "private/machine-binding.toml"),
            "distributionLock": document("distribution-lock.toml", "private/distribution-lock.toml"),
        },
        "packageDocuments": [
            document("engine/package.toml", "packages/engine/package.toml"),
            document("engine/roles.toml", "packages/engine/roles.toml"),
            document("engine/role.md", "packages/engine/role.md"),
        ],
        "workspace": {
            "rootManifest": "pale-workspace.toml",
            "documents": ([
                "pale-workspace.toml", "composition.toml",
                "workspace-resolution.lock.toml", "workspace-universe.lock.toml",
            ].map(|name| document(name, &format!("workspace/{name}")))),
        },
    })
}

pub(super) fn admitted_profile() -> AdmittedProfile {
    let mut request = captured_one_shot_request();
    let captured: wire::OneShotRequest = serde_json::from_value(request.clone()).unwrap();
    let api = pale_sdk::PaleApi::new().unwrap();
    let workspace = api
        .compile_portable_workspace(captured.workspace.into_api().unwrap())
        .unwrap();
    let workspace_id = bridge_state::next_object_handle();
    request["workspace"] = serde_json::json!(workspace_id);
    let profile_request = serde_json::to_vec(&request).unwrap();
    let profile = api
        .compile_effective_profile(
            &workspace,
            decode::<wire::EffectiveProfileRequest>(&profile_request)
                .unwrap()
                .into_api()
                .unwrap(),
        )
        .unwrap();
    let profile_id = bridge_state::next_object_handle();
    let mut state = bridge_state::ClientState::new(api);
    state.profiles.insert(profile_id, profile);
    state.workspaces.insert(workspace_id, workspace);
    AdmittedProfile {
        client: bridge_state::insert_client(state),
        workspace: workspace_id,
        profile: profile_id,
        profile_request,
        candidate_request: pale_sdk::ConfigurationImprovementCandidateRequest::new(
            captured.project_id,
            captured.execution_workspace_id,
            captured.machine_id,
            captured.private_inputs.into_api().unwrap(),
        ),
    }
}

pub(super) fn one_shot_plan_request(client: u64, compilation: u64) -> Vec<u8> {
    let targets = operations::one_shot_observation_targets(client, &serde_json::to_vec(&serde_json::json!({
        "compilation":compilation,"repositoryId":"engine","kind":"apply","prior":{"state":"unowned"}
    })).unwrap()).unwrap();
    let targets: serde_json::Value = serde_json::from_slice(&targets.payload).unwrap();
    let baseline: Vec<_> = targets["targets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|target| {
            serde_json::json!({
                "relativePath":target,"observation":{"state":"absent"}
            })
        })
        .collect();
    serde_json::to_vec(&serde_json::json!({
        "compilation":compilation,"repositoryId":"engine","kind":"apply","prior":{"state":"unowned"},"baseline":baseline
    })).unwrap()
}
