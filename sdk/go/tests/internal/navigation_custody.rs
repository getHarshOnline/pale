//! Real navigation producers retain identities only after complete response claim.

use super::*;

fn navigate(fixture: &fixtures::AdmittedProfile, effective: bool, unresolved: bool) -> u64 {
    let path = if unresolved {
        serde_json::json!([
            {"kind":"field","value":"workspace"},
            {"kind":"field","value":"desired_topology"},
            {"kind":"field","value":"project_repositories"},
            {"kind":"index","value":1},
            {"kind":"field","value":"project_id"},
        ])
    } else {
        serde_json::json!([
            {"kind":"field","value":"workspace"},
            {"kind":"field","value":"desired_topology"},
            {"kind":"field","value":"project_execution_workspaces"},
            {"kind":"index","value":0},
            {"kind":"field","value":"project_id"},
        ])
    };
    let request = bridge_state::insert_request(
        serde_json::to_vec(&serde_json::json!({
            "handle": if effective { fixture.profile } else { fixture.workspace },
            "sourceLabel":"pale-workspace.toml", "path":path,
        }))
        .unwrap(),
    );
    let response = if effective {
        pale_go_navigate_effective_profile_authoring(fixture.client, request)
    } else {
        pale_go_navigate_portable_workspace_authoring(fixture.client, request)
    };
    assert!(bridge_state::take_request(request).is_none());
    assert_eq!(pale_go_response_kind(response), ResponseKind::Success as u8);
    response
}

fn identities(client: u64) -> usize {
    bridge_state::with_client(client, |state| Ok(state.authoring_identities.len())).unwrap()
}

fn join(client: u64, profile: u64, identity: u64) -> u64 {
    let request = bridge_state::insert_request(
        serde_json::to_vec(&serde_json::json!({
            "handle":profile, "identity":identity,
        }))
        .unwrap(),
    );
    pale_go_join_effective_profile_authoring(client, request)
}

#[test]
fn actual_navigation_values_are_unusable_until_claim_and_discarded_on_drop() {
    for effective in [false, true] {
        let fixture = fixtures::admitted_profile();
        let response = navigate(&fixture, effective, false);
        let identity = pale_go_response_object(response);
        assert_ne!(identity, 0);
        assert_eq!(identities(fixture.client), 0);
        let unavailable = join(fixture.client, fixture.profile, identity);
        assert_eq!(
            pale_go_response_kind(unavailable),
            ResponseKind::BridgeFailure as u8
        );
        pale_go_response_drop(unavailable);
        pale_go_response_drop(response);
        assert_eq!(pale_go_response_claim(response), 0);
        assert_eq!(identities(fixture.client), 0);
        pale_go_client_drop(fixture.client);
    }
}

#[test]
fn resolved_and_completion_identities_transfer_once_and_release_independently() {
    for effective in [false, true] {
        for unresolved in [false, true] {
            let fixture = fixtures::admitted_profile();
            let foreign = fixtures::admitted_profile();
            let response = navigate(&fixture, effective, unresolved);
            let object = pale_go_response_object(response);
            let metadata: serde_json::Value = bridge_state::with_response(response, |response| {
                serde_json::from_slice(&response.metadata).unwrap()
            })
            .unwrap();
            let navigation = &metadata["navigation"];
            assert_eq!(navigation.as_object().unwrap().len(), 7);
            assert_eq!(object == 0, unresolved);
            assert_eq!(navigation["identity"].is_null(), unresolved);
            let alternatives = navigation["alternatives"].as_array().unwrap();
            assert!(!alternatives.is_empty());
            let handles = metadata["alternativeHandles"].as_array().unwrap();
            assert_eq!(handles.len(), alternatives.len());
            assert_eq!(identities(fixture.client), 0);
            assert_eq!(pale_go_response_claim(response), 1);
            assert_eq!(pale_go_response_claim(response), 0);
            assert_eq!(
                identities(fixture.client),
                alternatives.len() + usize::from(object != 0)
            );
            for (index, entry) in handles.iter().enumerate() {
                assert_eq!(entry["alternativeIndex"], index);
                let handle = entry["handle"].as_u64().unwrap();
                assert_ne!(handle, 0);
                bridge_state::with_client(fixture.client, |state| {
                    assert_eq!(
                        serde_json::to_value(&state.authoring_identities[&handle]).unwrap(),
                        alternatives[index]
                    );
                    Ok(())
                })
                .unwrap();
                let joined = join(fixture.client, fixture.profile, handle);
                assert_eq!(pale_go_response_kind(joined), ResponseKind::Success as u8);
                pale_go_response_drop(joined);
                pale_go_authoring_identity_drop(foreign.client, handle);
                let rejected = join(foreign.client, foreign.profile, handle);
                assert_eq!(
                    pale_go_response_kind(rejected),
                    ResponseKind::BridgeFailure as u8
                );
                pale_go_response_drop(rejected);
                pale_go_authoring_identity_drop(fixture.client, handle);
                pale_go_authoring_identity_drop(fixture.client, handle);
            }
            assert_eq!(identities(fixture.client), usize::from(object != 0));
            if object != 0 {
                let joined = join(fixture.client, fixture.profile, object);
                assert_eq!(pale_go_response_kind(joined), ResponseKind::Success as u8);
                pale_go_response_drop(joined);
                pale_go_authoring_identity_drop(fixture.client, object);
            }
            pale_go_authoring_identity_drop(fixture.client, fixture.profile);
            bridge_state::with_client(fixture.client, |state| {
                assert!(state.authoring_identities.is_empty());
                assert!(state.profiles.contains_key(&fixture.profile));
                assert!(state.workspaces.contains_key(&fixture.workspace));
                Ok(())
            })
            .unwrap();
            pale_go_client_drop(fixture.client);
            pale_go_client_drop(foreign.client);
        }
    }
}

#[test]
fn closed_navigation_owner_cannot_transfer_pending_alternatives() {
    let fixture = fixtures::admitted_profile();
    let foreign = fixtures::admitted_profile();
    let response = navigate(&fixture, true, true);
    assert_eq!(pale_go_response_object(response), 0);
    assert_eq!(identities(fixture.client), 0);
    pale_go_client_drop(fixture.client);
    assert_eq!(pale_go_response_claim(response), 0);
    assert_eq!(pale_go_response_claim(response), 0);
    assert_eq!(identities(foreign.client), 0);
    pale_go_client_drop(foreign.client);
}
