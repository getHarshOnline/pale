//! Observation values stay response-owned until the host claims a decoded result.

use super::*;

fn observe(client: u64, profile: u64) -> Response {
    operations::observe_effective_profile(
        client,
        &serde_json::to_vec(&serde_json::json!({"profile": profile})).unwrap(),
    )
    .unwrap()
}

fn page_request(snapshot: u64, maximum_returned: u16) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "snapshot": snapshot,
        "filter": {},
        "maximumInspected": 0,
        "maximumReturned": maximum_returned,
        "maximumCopiedStringBytes": 0,
        "cancellation": "active",
        "after": null
    }))
    .unwrap()
}

fn counts(client: u64) -> (usize, usize, usize, usize) {
    bridge_state::with_client(client, |state| {
        Ok((
            state.profiles.len(),
            state.observation_snapshots.len(),
            state.observation_page_cursors.len(),
            state.observation_subjects.len(),
        ))
    })
    .unwrap()
}

#[test]
fn discard_malformed_observation_response_preserves_existing_client_objects() {
    let fixtures::AdmittedProfile {
        client, profile, ..
    } = fixtures::admitted_profile();
    let mut response = observe(client, profile);
    let snapshot = response.object;
    response.metadata = b"{".to_vec();
    assert_eq!(counts(client), (1, 0, 0, 0));
    let response = bridge_state::insert_response(response);
    pale_go_response_drop(response);
    assert_eq!(pale_go_response_claim(response), 0);
    assert_eq!(counts(client), (1, 0, 0, 0));
    bridge_state::with_client(client, |state| {
        assert!(state.profiles.contains_key(&profile));
        assert!(!state.observation_snapshots.contains_key(&snapshot));
        Ok(())
    })
    .unwrap();
    pale_go_client_drop(client);
}

#[test]
fn observation_claim_is_once_only_and_keeps_exact_client_custody() {
    let fixtures::AdmittedProfile {
        client, profile, ..
    } = fixtures::admitted_profile();
    let fixtures::AdmittedProfile {
        client: foreign, ..
    } = fixtures::admitted_profile();
    let response = observe(client, profile);
    let snapshot = response.object;
    let response = bridge_state::insert_response(response);
    assert_eq!(pale_go_response_claim(response), 1);
    assert_eq!(pale_go_response_claim(response), 0);
    pale_go_response_drop(response);
    assert_eq!(counts(client), (1, 1, 0, 0));
    assert_eq!(counts(foreign), (1, 0, 0, 0));
    let foreign_request = page_request(snapshot, 2);
    let failure = operations::observation_page(foreign, &foreign_request)
        .err()
        .expect("claimed snapshot stays inaccessible to another client");
    assert_eq!(failure.code, "invalid_binding_handle");
    assert_eq!(counts(client), (1, 1, 0, 0));
    assert_eq!(counts(foreign), (1, 0, 0, 0));
    pale_go_client_drop(client);
    pale_go_client_drop(foreign);
}

#[test]
fn closed_owner_discards_pending_values_without_redirecting_them() {
    let fixtures::AdmittedProfile {
        client, profile, ..
    } = fixtures::admitted_profile();
    let fixtures::AdmittedProfile {
        client: foreign, ..
    } = fixtures::admitted_profile();
    let response = bridge_state::insert_response(observe(client, profile));
    pale_go_client_drop(client);
    assert_eq!(pale_go_response_claim(response), 0);
    assert_eq!(pale_go_response_claim(response), 0);
    assert_eq!(counts(foreign), (1, 0, 0, 0));
    pale_go_client_drop(foreign);
}

#[test]
fn page_subjects_and_continuation_transfer_together_or_are_discarded_together() {
    let fixtures::AdmittedProfile {
        client, profile, ..
    } = fixtures::admitted_profile();
    let response = observe(client, profile);
    let snapshot = response.object;
    assert_eq!(
        pale_go_response_claim(bridge_state::insert_response(response)),
        1
    );
    let request = page_request(snapshot, 2);
    let response = operations::observation_page(client, &request).unwrap();
    let metadata: serde_json::Value = serde_json::from_slice(&response.metadata).unwrap();
    let cursor = metadata["continuationHandle"].as_u64().unwrap();
    let subjects = metadata["subjectHandles"].as_array().unwrap();
    assert!(!subjects.is_empty());
    assert_eq!(counts(client), (1, 1, 0, 0));
    assert_eq!(
        pale_go_response_claim(bridge_state::insert_response(response)),
        1
    );
    assert_eq!(counts(client), (1, 1, 1, subjects.len()));
    bridge_state::with_client(client, |state| {
        assert!(state.observation_page_cursors.contains_key(&cursor));
        for subject in subjects {
            assert!(state
                .observation_subjects
                .contains_key(&subject["handle"].as_u64().unwrap()));
        }
        Ok(())
    })
    .unwrap();

    let mut rejected = operations::observation_page(client, &request).unwrap();
    rejected.metadata = b"not JSON".to_vec();
    let rejected = bridge_state::insert_response(rejected);
    pale_go_response_drop(rejected);
    assert_eq!(counts(client), (1, 1, 1, subjects.len()));
    pale_go_client_drop(client);
}

#[test]
fn ordinary_success_claims_without_pending_values_and_errors_do_not() {
    assert_eq!(
        pale_go_response_claim(bridge_state::insert_response(empty_success())),
        1
    );
    let error =
        bridge_state::insert_response(bridge_failure(BridgeFailure::invalid_handle("client", 0)));
    assert_eq!(pale_go_response_claim(error), 0);
    assert_eq!(pale_go_response_claim(0), 0);
}

#[test]
fn repeated_observation_pages_release_back_to_the_original_client_allocation() {
    let fixtures::AdmittedProfile {
        client, profile, ..
    } = fixtures::admitted_profile();
    for _ in 0..32 {
        let response = observe(client, profile);
        let snapshot = response.object;
        assert_eq!(
            pale_go_response_claim(bridge_state::insert_response(response)),
            1
        );
        let request = page_request(snapshot, 1);
        for _ in 0..8 {
            let response = operations::observation_page(client, &request).unwrap();
            let metadata: serde_json::Value = serde_json::from_slice(&response.metadata).unwrap();
            let cursor = metadata["continuationHandle"].as_u64().unwrap();
            let subject = metadata["subjectHandles"][0]["handle"].as_u64().unwrap();
            assert_eq!(
                pale_go_response_claim(bridge_state::insert_response(response)),
                1
            );
            assert_eq!(counts(client), (1, 1, 1, 1));
            pale_go_observation_page_cursor_drop(client, cursor);
            pale_go_observation_subject_drop(client, subject);
            pale_go_observation_page_cursor_drop(client, cursor);
            pale_go_observation_subject_drop(client, subject);
            assert_eq!(counts(client), (1, 1, 0, 0));
        }
        pale_go_observation_snapshot_drop(client, snapshot);
        pale_go_observation_snapshot_drop(client, snapshot);
        assert_eq!(counts(client), (1, 0, 0, 0));
    }
    pale_go_client_drop(client);
}

#[test]
fn observation_release_is_exactly_client_and_object_kind_scoped_without_cascading() {
    let fixtures::AdmittedProfile {
        client, profile, ..
    } = fixtures::admitted_profile();
    let fixtures::AdmittedProfile {
        client: foreign, ..
    } = fixtures::admitted_profile();
    let response = observe(client, profile);
    let snapshot = response.object;
    assert_eq!(
        pale_go_response_claim(bridge_state::insert_response(response)),
        1
    );
    let request = page_request(snapshot, 1);
    let response = operations::observation_page(client, &request).unwrap();
    let metadata: serde_json::Value = serde_json::from_slice(&response.metadata).unwrap();
    let cursor = metadata["continuationHandle"].as_u64().unwrap();
    let subject = metadata["subjectHandles"][0]["handle"].as_u64().unwrap();
    assert_eq!(
        pale_go_response_claim(bridge_state::insert_response(response)),
        1
    );
    for wrong_client in [foreign, 0] {
        pale_go_observation_snapshot_drop(wrong_client, snapshot);
        pale_go_observation_page_cursor_drop(wrong_client, cursor);
        pale_go_observation_subject_drop(wrong_client, subject);
    }
    for unrelated in [profile, cursor, subject] {
        pale_go_observation_snapshot_drop(client, unrelated);
    }
    for unrelated in [profile, snapshot, subject] {
        pale_go_observation_page_cursor_drop(client, unrelated);
    }
    for unrelated in [profile, snapshot, cursor] {
        pale_go_observation_subject_drop(client, unrelated);
    }
    assert_eq!(counts(client), (1, 1, 1, 1));
    assert_eq!(counts(foreign), (1, 0, 0, 0));
    pale_go_observation_snapshot_drop(client, snapshot);
    assert_eq!(counts(client), (1, 0, 1, 1));
    pale_go_observation_page_cursor_drop(client, cursor);
    pale_go_observation_subject_drop(client, subject);
    assert_eq!(counts(client), (1, 0, 0, 0));
    pale_go_client_drop(client);
    pale_go_observation_snapshot_drop(client, snapshot);
    pale_go_observation_page_cursor_drop(client, cursor);
    pale_go_observation_subject_drop(client, subject);
    assert_eq!(counts(foreign), (1, 0, 0, 0));
    pale_go_client_drop(foreign);
}
