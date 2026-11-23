//! Exact client-scoped cleanup of surrendered one-shot inputs.

use super::*;

fn claim(response: bridge_state::Response) -> u64 {
    let object = response.object;
    let response = bridge_state::insert_response(response);
    assert_eq!(pale_go_response_claim(response), 1);
    object
}

#[test]
fn one_shot_compilation_response_owns_output_until_claim() {
    let client = bridge_state::insert_client(bridge_state::ClientState::new(
        pale_sdk::PaleApi::new().unwrap(),
    ));
    let response = pale_go_compile_one_shot(
        client,
        bridge_state::insert_request(
            serde_json::to_vec(&fixtures::captured_one_shot_request()).unwrap(),
        ),
    );
    assert_eq!(pale_go_response_kind(response), ResponseKind::Success as u8);
    let count = bridge_state::with_client(client, |state| Ok(state.compilations.len())).unwrap();
    // Record the count before cleanup so a RED test cannot retain a client.
    pale_go_response_drop(response);
    let remaining =
        bridge_state::with_client(client, |state| Ok(state.compilations.len())).unwrap();
    pale_go_client_drop(client);
    assert_eq!(count, 0, "unclaimed output entered the client map");
    assert_eq!(remaining, 0, "discarded response retained its compilation");
}

#[test]
fn surrendered_one_shot_cleanup_preserves_raw_c_admission_and_independent_values() {
    let client = bridge_state::insert_client(bridge_state::ClientState::new(
        pale_sdk::PaleApi::new().unwrap(),
    ));
    let foreign = bridge_state::insert_client(bridge_state::ClientState::new(
        pale_sdk::PaleApi::new().unwrap(),
    ));
    let compilation = claim(
        operations::compile_one_shot(
            client,
            &serde_json::to_vec(&fixtures::captured_one_shot_request()).unwrap(),
        )
        .unwrap(),
    );
    let plan_request = fixtures::one_shot_plan_request(client, compilation);
    let candidate = claim(operations::plan_one_shot(client, &plan_request).unwrap());
    let sibling = claim(operations::plan_one_shot(client, &plan_request).unwrap());
    let bound = claim(
        operations::bind_one_shot_machine_effects(
            client,
            &serde_json::to_vec(&serde_json::json!({"candidate":sibling,"effects":[]})).unwrap(),
        )
        .unwrap(),
    );

    // Raw callers retain custody on private decoding failure. Only the Go
    // invocation wrapper separately discards its explicitly surrendered input.
    for (invoke, request) in [
        (
            pale_go_bind_one_shot_machine_effects as extern "C" fn(u64, u64) -> u64,
            serde_json::json!({"candidate":candidate,"effects":[null]}),
        ),
        (
            pale_go_admit_one_shot_saved_plan,
            serde_json::json!({"boundPlan":bound,"savedBytes":null}),
        ),
    ] {
        let response = invoke(
            client,
            bridge_state::insert_request(serde_json::to_vec(&request).unwrap()),
        );
        assert_eq!(
            pale_go_response_kind(response),
            ResponseKind::BridgeFailure as u8
        );
        pale_go_response_drop(response);
    }
    pale_go_one_shot_candidate_drop(foreign, candidate);
    pale_go_one_shot_bound_plan_drop(foreign, bound);
    pale_go_one_shot_candidate_drop(client, bound);
    pale_go_one_shot_bound_plan_drop(client, candidate);
    bridge_state::with_client(client, |state| {
        assert_eq!(state.candidates.len(), 1);
        assert!(state.candidates.contains_key(&candidate));
        assert_eq!(state.bound_plans.len(), 1);
        assert!(state.bound_plans.contains_key(&bound));
        Ok(())
    })
    .unwrap();
    for _ in 0..2 {
        pale_go_one_shot_candidate_drop(client, candidate);
    }
    bridge_state::with_client(client, |state| {
        assert!(state.candidates.is_empty());
        assert!(state.bound_plans.contains_key(&bound));
        assert!(state.compilations.contains_key(&compilation));
        Ok(())
    })
    .unwrap();
    for _ in 0..2 {
        pale_go_one_shot_bound_plan_drop(client, bound);
    }
    bridge_state::with_client(client, |state| {
        assert!(state.bound_plans.is_empty());
        assert!(state.admitted_plans.is_empty());
        assert!(state.compilations.contains_key(&compilation));
        Ok(())
    })
    .unwrap();
    pale_go_client_drop(client);
    pale_go_client_drop(foreign);
    pale_go_one_shot_candidate_drop(client, candidate);
    pale_go_one_shot_bound_plan_drop(client, bound);
}
