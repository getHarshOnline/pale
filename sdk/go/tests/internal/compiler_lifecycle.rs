//! Compiler views retain independent custody before and after response claim.

use super::*;

fn counts(client: u64) -> (usize, usize, usize) {
    bridge_state::with_client(client, |state| {
        Ok((
            state.workspaces.len(),
            state.comparisons.len(),
            state.profiles.len(),
        ))
    })
    .unwrap()
}

fn compile(client: u64) -> u64 {
    let captured = fixtures::captured_one_shot_request();
    pale_go_compile_portable_workspace(
        client,
        bridge_state::insert_request(serde_json::to_vec(&captured["workspace"]).unwrap()),
    )
}

fn compare(client: u64, workspace: u64) -> u64 {
    pale_go_compare_portable_workspaces(
        client,
        bridge_state::insert_request(
            serde_json::to_vec(&serde_json::json!({"prior": workspace, "current": workspace}))
                .unwrap(),
        ),
    )
}

fn assert_discarded(fixture: fixtures::AdmittedProfile, response: u64) {
    assert_eq!(pale_go_response_kind(response), ResponseKind::Success as u8);
    let before = counts(fixture.client);
    pale_go_response_drop(response);
    let after = counts(fixture.client);
    let claimed = pale_go_response_claim(response);
    pale_go_client_drop(fixture.client);
    assert_eq!(
        before,
        (1, 0, 1),
        "unclaimed compiler output entered a client map"
    );
    assert_eq!(
        after,
        (1, 0, 1),
        "discarded output retained a compiler view"
    );
    assert_eq!(claimed, 0);
}

#[test]
fn portable_compilation_is_response_owned_until_claim() {
    let fixture = fixtures::admitted_profile();
    let response = compile(fixture.client);
    assert_discarded(fixture, response);
}

#[test]
fn portable_comparison_is_response_owned_until_claim() {
    let fixture = fixtures::admitted_profile();
    let response = compare(fixture.client, fixture.workspace);
    assert_discarded(fixture, response);
}

fn inspect_workspace(client: u64, workspace: u64) -> u64 {
    pale_go_inspect_portable_workspace_authoring(
        client,
        bridge_state::insert_request(
            serde_json::to_vec(&serde_json::json!({
                "handle": workspace, "sourceLabel": "pale-workspace.toml", "path": [],
            }))
            .unwrap(),
        ),
    )
}

fn inspect_comparison(client: u64, comparison: u64) -> u64 {
    pale_go_portable_workspace_impacts(
        client,
        bridge_state::insert_request(
            serde_json::to_vec(&serde_json::json!({"handle": comparison, "offset": 0, "limit": 1}))
                .unwrap(),
        ),
    )
}

fn assert_kind(response: u64, expected: ResponseKind) {
    let actual = pale_go_response_kind(response);
    pale_go_response_drop(response);
    assert_eq!(actual, expected as u8);
}

#[test]
fn compiler_claims_are_once_only_and_drops_are_exactly_scoped() {
    let fixture = fixtures::admitted_profile();
    let foreign = fixtures::admitted_profile();
    let workspace_response = compile(fixture.client);
    let comparison_response = compare(fixture.client, fixture.workspace);
    let workspace = pale_go_response_object(workspace_response);
    let comparison = pale_go_response_object(comparison_response);
    for (response, query) in [
        (workspace_response, inspect_workspace as fn(u64, u64) -> u64),
        (comparison_response, inspect_comparison),
    ] {
        assert_eq!(pale_go_response_kind(response), ResponseKind::Success as u8);
        let object = pale_go_response_object(response);
        assert_ne!(object, 0);
        assert!(pale_go_response_metadata_length(response) > 0);
        assert_eq!(pale_go_response_payload_length(response), 0);
        assert_kind(query(fixture.client, object), ResponseKind::BridgeFailure);
        assert_eq!(pale_go_response_claim(response), 1);
        assert_eq!(pale_go_response_claim(response), 0);
        assert_kind(query(fixture.client, object), ResponseKind::Success);
        assert_kind(query(foreign.client, object), ResponseKind::BridgeFailure);
    }
    assert_eq!(counts(fixture.client), (2, 1, 1));
    pale_go_portable_workspace_drop(foreign.client, workspace);
    pale_go_workspace_comparison_drop(foreign.client, comparison);
    pale_go_effective_profile_drop(foreign.client, fixture.profile);
    pale_go_portable_workspace_drop(fixture.client, comparison);
    pale_go_workspace_comparison_drop(fixture.client, fixture.profile);
    pale_go_effective_profile_drop(fixture.client, workspace);
    assert_eq!(counts(fixture.client), (2, 1, 1));
    assert_eq!(counts(foreign.client), (1, 0, 1));
    for _ in 0..2 {
        pale_go_portable_workspace_drop(fixture.client, workspace);
        pale_go_workspace_comparison_drop(fixture.client, comparison);
        pale_go_effective_profile_drop(fixture.client, fixture.profile);
    }
    assert_eq!(counts(fixture.client), (1, 0, 0));
    assert_kind(
        inspect_workspace(fixture.client, workspace),
        ResponseKind::BridgeFailure,
    );
    assert_kind(
        inspect_comparison(fixture.client, comparison),
        ResponseKind::BridgeFailure,
    );
    assert_kind(
        inspect_workspace(fixture.client, fixture.workspace),
        ResponseKind::Success,
    );
    pale_go_client_drop(fixture.client);
    pale_go_client_drop(foreign.client);
    pale_go_portable_workspace_drop(fixture.client, workspace);
    pale_go_workspace_comparison_drop(fixture.client, comparison);
    pale_go_effective_profile_drop(fixture.client, fixture.profile);
}

#[test]
fn closed_client_cannot_claim_pending_compiler_views() {
    let fixture = fixtures::admitted_profile();
    let foreign = fixtures::admitted_profile();
    let responses = [
        compile(fixture.client),
        compare(fixture.client, fixture.workspace),
    ];
    assert_eq!(counts(fixture.client), (1, 0, 1));
    pale_go_client_drop(fixture.client);
    for response in responses {
        assert_eq!(pale_go_response_claim(response), 0);
        assert_eq!(pale_go_response_claim(response), 0);
    }
    assert_eq!(counts(foreign.client), (1, 0, 1));
    pale_go_client_drop(foreign.client);
}

#[test]
fn repeated_compilation_and_comparison_release_returns_to_baseline() {
    let fixture = fixtures::admitted_profile();
    for _ in 0..8 {
        let response = compile(fixture.client);
        let workspace = pale_go_response_object(response);
        assert_eq!(pale_go_response_claim(response), 1);
        let response = compare(fixture.client, workspace);
        let comparison = pale_go_response_object(response);
        assert_eq!(pale_go_response_claim(response), 1);
        pale_go_portable_workspace_drop(fixture.client, workspace);
        assert_kind(
            inspect_comparison(fixture.client, comparison),
            ResponseKind::Success,
        );
        pale_go_workspace_comparison_drop(fixture.client, comparison);
        assert_eq!(counts(fixture.client), (1, 0, 1));
    }
    pale_go_client_drop(fixture.client);
}

#[test]
fn subject_comparison_copies_sdk_output_without_new_handle_custody() {
    let fixture = fixtures::admitted_profile();
    let foreign = fixtures::admitted_profile();
    let response = compare(fixture.client, fixture.workspace);
    let comparison = pale_go_response_object(response);
    assert_eq!(pale_go_response_claim(response), 1);
    let request = serde_json::to_vec(&serde_json::json!({
        "handle": comparison,
        "subject": {"kind": "workspace", "workspaceId": "act-ws"},
    }))
    .unwrap();
    let expected = bridge_state::with_client(fixture.client, |state| {
        Ok(state.comparisons[&comparison]
            .compare_subject_selector(pale_sdk::PortableWorkspaceSubjectSelector::Workspace {
                workspace_id: "act-ws".to_owned(),
            })
            .unwrap()
            .canonical_bytes()
            .to_vec())
    })
    .unwrap();
    let before = counts(fixture.client);
    for claim in [false, true] {
        let response = pale_go_compare_portable_workspace_subject(
            fixture.client,
            bridge_state::insert_request(request.clone()),
        );
        assert_eq!(pale_go_response_kind(response), ResponseKind::Success as u8);
        assert_eq!(pale_go_response_object(response), 0);
        assert_eq!(pale_go_response_metadata_length(response), 0);
        bridge_state::with_response(response, |value| {
            assert_eq!(value.payload, expected);
            assert!(value.pending_values.is_none());
        })
        .unwrap();
        if claim {
            assert_eq!(pale_go_response_claim(response), 1);
        } else {
            pale_go_response_drop(response);
        }
        assert_eq!(counts(fixture.client), before);
    }
    assert_kind(
        pale_go_compare_portable_workspace_subject(
            foreign.client,
            bridge_state::insert_request(request.clone()),
        ),
        ResponseKind::BridgeFailure,
    );
    pale_go_workspace_comparison_drop(fixture.client, comparison);
    assert_kind(
        pale_go_compare_portable_workspace_subject(
            fixture.client,
            bridge_state::insert_request(request),
        ),
        ResponseKind::BridgeFailure,
    );
    assert_eq!(counts(foreign.client), (1, 0, 1));
    pale_go_client_drop(fixture.client);
    pale_go_client_drop(foreign.client);
}
