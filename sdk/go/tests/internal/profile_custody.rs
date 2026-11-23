//! Newly compiled profiles stay response-owned until their coordinates are decoded.

use super::*;

fn compile(fixture: &fixtures::AdmittedProfile) -> u64 {
    let request = bridge_state::insert_request(fixture.profile_request.clone());
    let response = pale_go_compile_effective_profile(fixture.client, request);
    assert!(bridge_state::take_request(request).is_none());
    assert_eq!(pale_go_response_kind(response), ResponseKind::Success as u8);
    response
}

fn profile_count(client: u64) -> usize {
    bridge_state::with_client(client, |state| Ok(state.profiles.len())).unwrap()
}

fn inspect(client: u64, profile: u64) -> u64 {
    let request = bridge_state::insert_request(
        serde_json::to_vec(&serde_json::json!({
            "handle": profile, "section": "machine-realization", "offset": 0, "limit": 1,
        }))
        .unwrap(),
    );
    pale_go_inspect_effective_profile(client, request)
}

#[test]
fn compiled_profile_is_unusable_until_claim_and_drop_preserves_prior_profile() {
    let fixture = fixtures::admitted_profile();
    let response = compile(&fixture);
    let profile = pale_go_response_object(response);
    assert_ne!(profile, 0);
    assert_eq!(profile_count(fixture.client), 1);
    let unavailable = inspect(fixture.client, profile);
    assert_eq!(
        pale_go_response_kind(unavailable),
        ResponseKind::BridgeFailure as u8
    );
    pale_go_response_drop(unavailable);
    pale_go_response_drop(response);
    assert_eq!(pale_go_response_claim(response), 0);
    assert_eq!(profile_count(fixture.client), 1);
    let prior = inspect(fixture.client, fixture.profile);
    assert_eq!(pale_go_response_kind(prior), ResponseKind::Success as u8);
    pale_go_response_drop(prior);
    let discarded = inspect(fixture.client, profile);
    assert_eq!(
        pale_go_response_kind(discarded),
        ResponseKind::BridgeFailure as u8
    );
    pale_go_response_drop(discarded);
    pale_go_client_drop(fixture.client);
}

#[test]
fn compiled_profile_claim_transfers_once_to_its_exact_owner() {
    let fixture = fixtures::admitted_profile();
    let foreign = fixtures::admitted_profile();
    let response = compile(&fixture);
    let profile = pale_go_response_object(response);
    let expected = bridge_state::with_client(fixture.client, |client| {
        Ok(serde_json::to_value(client.profiles[&fixture.profile].coordinates()).unwrap())
    })
    .unwrap();
    bridge_state::with_response(response, |response| {
        let actual: serde_json::Value = serde_json::from_slice(&response.metadata).unwrap();
        assert_eq!(actual, expected);
        assert_eq!(actual.as_object().unwrap().len(), 11);
    })
    .unwrap();
    assert_eq!(pale_go_response_claim(response), 1);
    assert_eq!(pale_go_response_claim(response), 0);
    pale_go_response_drop(response);
    assert_eq!(profile_count(fixture.client), 2);
    let admitted = inspect(fixture.client, profile);
    assert_eq!(pale_go_response_kind(admitted), ResponseKind::Success as u8);
    pale_go_response_drop(admitted);
    let rejected = inspect(foreign.client, profile);
    assert_eq!(
        pale_go_response_kind(rejected),
        ResponseKind::BridgeFailure as u8
    );
    pale_go_response_drop(rejected);
    assert_eq!(profile_count(foreign.client), 1);
    pale_go_client_drop(fixture.client);
    pale_go_client_drop(foreign.client);
}

#[test]
fn closing_profile_owner_before_claim_cannot_redirect_the_pending_value() {
    let fixture = fixtures::admitted_profile();
    let foreign = fixtures::admitted_profile();
    let response = compile(&fixture);
    let profile = pale_go_response_object(response);
    pale_go_client_drop(fixture.client);
    assert_eq!(pale_go_response_claim(response), 0);
    assert_eq!(pale_go_response_claim(response), 0);
    let rejected = inspect(foreign.client, profile);
    assert_eq!(
        pale_go_response_kind(rejected),
        ResponseKind::BridgeFailure as u8
    );
    pale_go_response_drop(rejected);
    assert_eq!(profile_count(foreign.client), 1);
    pale_go_client_drop(foreign.client);
}
