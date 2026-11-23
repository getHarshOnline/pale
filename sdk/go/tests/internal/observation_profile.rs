//! Profile association forwards through the actual bridge and retains only cursors.

use super::*;
use serde_json::{json, Value};

fn metadata(response: u64) -> Value {
    bridge_state::with_response(response, |value| {
        serde_json::from_slice(&value.metadata).unwrap()
    })
    .unwrap()
}

fn call(client: u64, request: &Value) -> u64 {
    pale_go_observation_inspect_profile(
        client,
        bridge_state::insert_request(serde_json::to_vec(request).unwrap()),
    )
}

fn query(fixture: &fixtures::AdmittedProfile) -> Value {
    let response = operations::observe_effective_profile(
        fixture.client,
        &serde_json::to_vec(&json!({"profile": fixture.profile})).unwrap(),
    )
    .unwrap();
    let snapshot = response.object;
    let response = bridge_state::insert_response(response);
    assert_eq!(pale_go_response_claim(response), 1);
    pale_go_response_drop(response);
    let response = operations::observation_page(
        fixture.client,
        &serde_json::to_vec(&json!({
            "snapshot": snapshot,
            "filter": {},
            "maximumInspected": 0,
            "maximumReturned": 256,
            "maximumCopiedStringBytes": 0,
            "cancellation": "active",
            "after": null
        }))
        .unwrap(),
    )
    .unwrap();
    let response = bridge_state::insert_response(response);
    let page = metadata(response);
    let subject = page["subjectHandles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["subject"]["kind"] == "machine")
        .unwrap()["handle"]
        .as_u64()
        .unwrap();
    assert_eq!(pale_go_response_claim(response), 1);
    pale_go_response_drop(response);
    json!({"snapshot":snapshot,"profile":fixture.profile,"workspace":fixture.workspace,
        "subject":subject,"maximumInspected":1,"maximumReturned":1,"after":null})
}

fn cursor_count(client: u64) -> usize {
    bridge_state::with_client(client, |state| Ok(state.observation_profile_cursors.len())).unwrap()
}

#[test]
fn profile_cursor_is_response_owned_until_claim_and_client_scoped_on_release() {
    let fixture = fixtures::admitted_profile();
    let other = fixtures::admitted_profile();
    let request = query(&fixture);
    for claim in [false, true] {
        let response = call(fixture.client, &request);
        assert_eq!(pale_go_response_kind(response), ResponseKind::Success as u8);
        let cursor = metadata(response)["continuationHandle"].as_u64().unwrap();
        assert_eq!(cursor_count(fixture.client), 0);
        let mut next = request.clone();
        next["after"] = cursor.into();
        let absent = call(fixture.client, &next);
        assert_eq!(
            pale_go_response_kind(absent),
            ResponseKind::BridgeFailure as u8
        );
        pale_go_response_drop(absent);
        if claim {
            assert_eq!(pale_go_response_claim(response), 1);
            assert_eq!(pale_go_response_claim(response), 0);
            pale_go_response_drop(response);
            assert_eq!(cursor_count(fixture.client), 1);
            pale_go_observation_profile_cursor_drop(other.client, cursor);
            pale_go_observation_page_cursor_drop(fixture.client, cursor);
            assert_eq!(cursor_count(fixture.client), 1);
            let successor = call(fixture.client, &next);
            assert_eq!(
                pale_go_response_kind(successor),
                ResponseKind::Success as u8
            );
            pale_go_response_drop(successor);
            assert_eq!(cursor_count(fixture.client), 1);
            pale_go_observation_profile_cursor_drop(fixture.client, cursor);
            pale_go_observation_profile_cursor_drop(fixture.client, cursor);
        } else {
            // The Go decoder drops this response instead of claiming rejected metadata.
            pale_go_response_drop(response);
            assert_eq!(pale_go_response_claim(response), 0);
        }
        assert_eq!(cursor_count(fixture.client), 0);
    }
    let pending = call(fixture.client, &request);
    pale_go_client_drop(fixture.client);
    assert_eq!(pale_go_response_claim(pending), 0);
    assert_eq!(cursor_count(other.client), 0);
    pale_go_response_drop(pending);
    pale_go_client_drop(other.client);
}

#[test]
fn each_page_preserves_sdk_rows_handles_and_charged_raw_visits() {
    let fixture = fixtures::admitted_profile();
    let mut request = query(&fixture);
    let mut previous = None;
    let mut matched = 0;
    let mut complete = false;
    for _ in 0..4096 {
        let expected = bridge_state::with_client(fixture.client, |state| {
            let after = request["after"]
                .as_u64()
                .map(|id| &state.observation_profile_cursors[&id]);
            let page = state.observation_snapshots[&request["snapshot"].as_u64().unwrap()]
                .inspect_profile(
                    &state.profiles[&fixture.profile],
                    &state.workspaces[&fixture.workspace],
                    &state.observation_subjects[&request["subject"].as_u64().unwrap()],
                    after,
                    1,
                    1,
                )
                .unwrap();
            Ok((
                serde_json::to_value(page.coordinates).unwrap(),
                serde_json::to_value(page.entries).unwrap(),
                page.inspected,
            ))
        })
        .unwrap();
        let response = call(fixture.client, &request);
        assert_eq!(pale_go_response_kind(response), ResponseKind::Success as u8);
        let page = metadata(response);
        assert_eq!(page["state"], "ready");
        assert_eq!(page["mapping"], "primary-rows");
        assert_eq!(page["coordinates"], expected.0);
        assert_eq!(page["entries"], expected.1);
        assert_eq!(page["inspected"], expected.2);
        matched += page["entries"].as_array().unwrap().len();
        let next = page["continuationHandle"].as_u64();
        assert_eq!(pale_go_response_claim(response), 1);
        pale_go_response_drop(response);
        assert!(cursor_count(fixture.client) <= 2);
        if let Some(cursor) = previous {
            pale_go_observation_profile_cursor_drop(fixture.client, cursor);
        }
        previous = next;
        request["after"] = json!(next);
        if next.is_none() {
            complete = true;
            break;
        }
    }
    assert!(complete && matched > 0);
    assert_eq!(cursor_count(fixture.client), 0);
    pale_go_client_drop(fixture.client);
}

#[test]
fn binding_rejection_and_typed_refusals_remain_distinct() {
    let fixture = fixtures::admitted_profile();
    let request = query(&fixture);
    for field in ["snapshot", "profile", "workspace", "subject", "after"] {
        let mut invalid = request.clone();
        invalid[field] = json!(0);
        let response = call(fixture.client, &invalid);
        assert_eq!(
            pale_go_response_kind(response),
            ResponseKind::BridgeFailure as u8
        );
        pale_go_response_drop(response);
    }
    for field in ["maximumInspected", "maximumReturned"] {
        for value in [0, 257] {
            let mut invalid = request.clone();
            invalid[field] = json!(value);
            let response = call(fixture.client, &invalid);
            assert_eq!(pale_go_response_kind(response), ResponseKind::Success as u8);
            assert_eq!(
                metadata(response),
                json!({"state":"refused","reason":"invalid-limit"})
            );
            assert_eq!(pale_go_response_claim(response), 1);
            pale_go_response_drop(response);
        }
    }
    let mut malformed = request.clone();
    malformed["extra"] = true.into();
    let response = call(fixture.client, &malformed);
    assert_eq!(
        pale_go_response_kind(response),
        ResponseKind::BridgeFailure as u8
    );
    pale_go_response_drop(response);
    let response = call(fixture.client, &request);
    let cursor = metadata(response)["continuationHandle"].as_u64().unwrap();
    assert_eq!(pale_go_response_claim(response), 1);
    pale_go_response_drop(response);
    let mut changed = request.clone();
    changed["after"] = cursor.into();
    changed["maximumInspected"] = 2.into();
    let response = call(fixture.client, &changed);
    assert_eq!(
        metadata(response),
        json!({"state":"refused","reason":"continuation-mismatch"})
    );
    pale_go_response_drop(response);
    assert_eq!(cursor_count(fixture.client), 1);
    pale_go_client_drop(fixture.client);
}

#[test]
fn continuation_does_not_keep_original_compilations_alive() {
    let fixture = fixtures::admitted_profile();
    let foreign = fixtures::admitted_profile();
    let request = query(&fixture);
    let response = call(fixture.client, &request);
    assert_eq!(pale_go_response_claim(response), 1);
    pale_go_response_drop(response);
    let (other_profile, other_workspace) = bridge_state::with_client_mut(foreign.client, |state| {
        Ok((
            state.profiles.remove(&foreign.profile).unwrap(),
            state.workspaces.remove(&foreign.workspace).unwrap(),
        ))
    })
    .unwrap();
    bridge_state::with_client_mut(fixture.client, |state| {
        state.profiles.insert(foreign.profile, other_profile);
        state.workspaces.insert(foreign.workspace, other_workspace);
        Ok(())
    })
    .unwrap();
    let mut replaced = request.clone();
    replaced["profile"] = foreign.profile.into();
    replaced["workspace"] = foreign.workspace.into();
    let response = call(fixture.client, &replaced);
    assert_eq!(
        metadata(response),
        json!({"state":"refused","reason":"source-mismatch"})
    );
    pale_go_response_drop(response);
    pale_go_effective_profile_drop(fixture.client, fixture.profile);
    pale_go_portable_workspace_drop(fixture.client, fixture.workspace);
    let response = call(fixture.client, &replaced);
    assert_eq!(
        metadata(response),
        json!({"state":"refused","reason":"expired-source"})
    );
    pale_go_response_drop(response);
    let response = operations::observation_page(
        fixture.client,
        &serde_json::to_vec(&json!({
            "snapshot": request["snapshot"],
            "filter": {},
            "maximumInspected": 0,
            "maximumReturned": 1,
            "maximumCopiedStringBytes": 0,
            "cancellation": "active",
            "after": null
        }))
        .unwrap(),
    )
    .unwrap();
    assert!(
        !serde_json::from_slice::<Value>(&response.metadata).unwrap()["page"]["facts"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    drop(response);
    pale_go_client_drop(fixture.client);
    pale_go_client_drop(foreign.client);
}
