//! Real authoring cursors follow response claim and client-scoped release.

use super::*;

#[derive(Clone, Copy)]
enum Route {
    Portable,
    Effective,
    Profile,
}

impl Route {
    fn request(self, fixture: &fixtures::AdmittedProfile, identity: u64) -> serde_json::Value {
        let handle = match self {
            Self::Portable => fixture.workspace,
            _ => fixture.profile,
        };
        match self {
            Self::Profile => {
                serde_json::json!({"handle":handle,"identity":identity,"limit":1,"after":null})
            }
            _ => {
                serde_json::json!({"handle":handle,"identity":identity,"maximumInspected":1,"maximumReturned":1,"after":null})
            }
        }
    }

    fn call(self, client: u64, request: &serde_json::Value) -> u64 {
        let request = bridge_state::insert_request(serde_json::to_vec(request).unwrap());
        match self {
            Self::Portable => pale_go_portable_authoring_occurrences(client, request),
            Self::Effective => pale_go_effective_authoring_occurrences(client, request),
            Self::Profile => pale_go_effective_authoring_page(client, request),
        }
    }

    fn drop(self, client: u64, handle: u64) {
        match self {
            Self::Profile => pale_go_effective_authoring_cursor_drop(client, handle),
            _ => pale_go_authoring_occurrence_cursor_drop(client, handle),
        }
    }
}

fn identity(fixture: &fixtures::AdmittedProfile) -> u64 {
    let request = bridge_state::insert_request(serde_json::to_vec(&serde_json::json!({
        "handle":fixture.workspace,"sourceLabel":"pale-workspace.toml","path":[
            {"kind":"field","value":"workspace"},{"kind":"field","value":"desired_topology"},
            {"kind":"field","value":"repositories"},{"kind":"index","value":0},{"kind":"field","value":"id"}
        ]
    })).unwrap());
    let response = pale_go_navigate_portable_workspace_authoring(fixture.client, request);
    assert_eq!(pale_go_response_kind(response), ResponseKind::Success as u8);
    let identity = pale_go_response_object(response);
    assert_ne!(identity, 0);
    assert_eq!(pale_go_response_claim(response), 1);
    identity
}

fn counts(client: u64) -> (usize, usize) {
    bridge_state::with_client(client, |state| {
        Ok((
            state.authoring_occurrence_cursors.len(),
            state.effective_authoring_cursors.len(),
        ))
    })
    .unwrap()
}

fn metadata(response: u64) -> serde_json::Value {
    bridge_state::with_response(response, |value| {
        serde_json::from_slice(&value.metadata).unwrap()
    })
    .unwrap()
}

#[test]
fn actual_pages_transfer_once_only_after_claim_and_drop_unclaimed_cursors() {
    for route in [Route::Portable, Route::Effective, Route::Profile] {
        let fixture = fixtures::admitted_profile();
        let selected = identity(&fixture);
        let request = route.request(&fixture, selected);
        for claim in [false, true] {
            let response = route.call(fixture.client, &request);
            assert_eq!(pale_go_response_kind(response), ResponseKind::Success as u8);
            let cursor = metadata(response)["continuationHandle"]
                .as_u64()
                .expect("fixture must page");
            assert_eq!(counts(fixture.client), (0, 0));
            let mut next = request.clone();
            next["after"] = cursor.into();
            let unavailable = route.call(fixture.client, &next);
            assert_eq!(
                pale_go_response_kind(unavailable),
                ResponseKind::BridgeFailure as u8
            );
            pale_go_response_drop(unavailable);
            if claim {
                assert_eq!(pale_go_response_claim(response), 1);
                assert_eq!(pale_go_response_claim(response), 0);
                let expected = match route {
                    Route::Profile => (0, 1),
                    _ => (1, 0),
                };
                assert_eq!(counts(fixture.client), expected);
                let resumed = route.call(fixture.client, &next);
                assert_eq!(pale_go_response_kind(resumed), ResponseKind::Success as u8);
                // Failed delivery drops a successor without consuming its predecessor.
                pale_go_response_drop(resumed);
                assert_eq!(counts(fixture.client), expected);
                route.drop(fixture.client, cursor);
                route.drop(fixture.client, cursor);
            } else {
                pale_go_response_drop(response);
                assert_eq!(pale_go_response_claim(response), 0);
            }
            assert_eq!(counts(fixture.client), (0, 0));
        }
        pale_go_client_drop(fixture.client);
    }
}

#[test]
fn sequential_pages_release_to_baseline_without_an_aggregate_quota_claim() {
    for route in [Route::Portable, Route::Effective, Route::Profile] {
        let fixture = fixtures::admitted_profile();
        let selected = identity(&fixture);
        let mut request = route.request(&fixture, selected);
        let mut previous = None;
        let mut complete = false;
        for _ in 0..4096 {
            let response = route.call(fixture.client, &request);
            assert_eq!(pale_go_response_kind(response), ResponseKind::Success as u8);
            let next = metadata(response)["continuationHandle"].as_u64();
            assert_eq!(pale_go_response_claim(response), 1);
            let (occurrences, profiles) = counts(fixture.client);
            assert!(occurrences + profiles <= 2);
            if let Some(cursor) = previous {
                route.drop(fixture.client, cursor);
            }
            previous = next;
            request["after"] = serde_json::json!(next);
            if next.is_none() {
                complete = true;
                break;
            }
        }
        assert!(complete);
        assert_eq!(counts(fixture.client), (0, 0));
        // Retention is explicit caller choice, not an enforced map capacity.
        request["after"] = serde_json::Value::Null;
        for _ in 0..3 {
            assert_eq!(
                pale_go_response_claim(route.call(fixture.client, &request)),
                1
            );
        }
        let (occurrences, profiles) = counts(fixture.client);
        assert_eq!(occurrences + profiles, 3);
        pale_go_client_drop(fixture.client);
    }
}

#[test]
fn cursor_release_is_client_and_kind_scoped_and_closed_clients_cannot_claim() {
    for route in [Route::Portable, Route::Effective, Route::Profile] {
        let fixture = fixtures::admitted_profile();
        let foreign = fixtures::admitted_profile();
        let selected = identity(&fixture);
        let request = route.request(&fixture, selected);
        let response = route.call(fixture.client, &request);
        let cursor = metadata(response)["continuationHandle"].as_u64().unwrap();
        assert_eq!(pale_go_response_claim(response), 1);
        let expected = counts(fixture.client);
        route.drop(foreign.client, cursor);
        route.drop(fixture.client, selected);
        route.drop(fixture.client, fixture.workspace);
        route.drop(fixture.client, fixture.profile);
        match route {
            Route::Profile => pale_go_authoring_occurrence_cursor_drop(fixture.client, cursor),
            _ => pale_go_effective_authoring_cursor_drop(fixture.client, cursor),
        }
        assert_eq!(counts(fixture.client), expected);
        let pending = route.call(fixture.client, &request);
        pale_go_client_drop(fixture.client);
        assert_eq!(pale_go_response_claim(pending), 0);
        assert_eq!(counts(foreign.client), (0, 0));
        route.drop(fixture.client, cursor);
        pale_go_client_drop(foreign.client);
    }
}

#[test]
fn private_numeric_requests_reject_malformed_limits_before_allocating_cursors() {
    for route in [Route::Portable, Route::Effective, Route::Profile] {
        let fixture = fixtures::admitted_profile();
        let selected = identity(&fixture);
        let valid = route.request(&fixture, selected);
        let fields: &[&str] = match route {
            Route::Profile => &["limit"],
            _ => &["maximumInspected", "maximumReturned"],
        };
        for field in fields {
            for invalid in [
                serde_json::json!(-1),
                serde_json::json!(65536),
                serde_json::json!(1.5),
                serde_json::json!(true),
                serde_json::json!("1"),
                serde_json::Value::Null,
            ] {
                let mut request = valid.clone();
                request[*field] = invalid;
                let response = route.call(fixture.client, &request);
                assert_eq!(
                    pale_go_response_kind(response),
                    ResponseKind::BridgeFailure as u8
                );
                assert_eq!(metadata(response)["code"], "invalid_binding_request");
                pale_go_response_drop(response);
                assert_eq!(counts(fixture.client), (0, 0));
            }
        }
        pale_go_client_drop(fixture.client);
    }
}
