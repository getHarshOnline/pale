//! Raw C request and client-custody laws for improvement projections.

use super::*;

#[test]
fn improvement_wire_rejects_unknown_stage_and_nested_fields_before_custody() {
    for invoke in [
        pale_go_improvement_inspect as extern "C" fn(u64, u64) -> u64,
        pale_go_improvement_evaluate,
        pale_go_improvement_diagnose,
        pale_go_improvement_preview,
        pale_go_improvement_accept,
    ] {
        let request =
            bridge_state::insert_request(br#"{\"handle\":1,\"stage\":\"previewed\"}"#.to_vec());
        let response = invoke(0, request);
        bridge_state::with_response(response, |response| {
            let failure: serde_json::Value = serde_json::from_slice(&response.metadata).unwrap();
            assert_eq!(failure["code"], "invalid_binding_request");
            assert!(response.payload.is_empty());
            assert_eq!(response.object, 0);
        })
        .unwrap();
        assert!(bridge_state::take_request(request).is_none());
        pale_go_response_drop(response);
    }
    let request = bridge_state::insert_request(br#"{
        "handle":1,"candidate":2,"projectId":"demo","executionWorkspaceId":"default","machineId":"machine",
        "privateInputs":{"capabilities":{"label":"c","bytes":"","extra":true},
            "machineBinding":{"label":"m","bytes":""},"distributionLock":{"label":"d","bytes":""}}
    }"#.to_vec());
    let response = pale_go_improvement_prepare(0, request);
    bridge_state::with_response(response, |response| {
        let failure: serde_json::Value = serde_json::from_slice(&response.metadata).unwrap();
        assert_eq!(failure["code"], "invalid_binding_request");
    })
    .unwrap();
    pale_go_response_drop(response);
}

#[test]
fn c_client_scope_rejection_preserves_unconsumed_inspection() {
    let api = pale_sdk::PaleApi::new().expect("embedded API");
    let root = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/cross-leaf.v1/portable"
    );
    let documents = [
        "pale-workspace.toml",
        "resolution.lock.toml",
        "universe.lock.toml",
    ]
    .into_iter()
    .map(|name| {
        pale_sdk::PortableWorkspaceDocument::new(
            name.to_owned(),
            std::fs::read(std::path::Path::new(root).join(name)).unwrap(),
        )
    })
    .collect();
    let baseline = api
        .compile_portable_workspace(pale_sdk::PortableWorkspaceRequest::new(
            "pale-workspace.toml".to_owned(),
            documents,
        ))
        .expect("canonical portable fixture");
    let inspection = api.inspect_configuration_improvement(&baseline);
    let inspection_id = bridge_state::next_object_handle();
    let mut owner = bridge_state::ClientState::new(api);
    owner
        .improvement_inspections
        .insert(inspection_id, inspection);
    let owner_id = bridge_state::insert_client(owner);
    let foreign_id = bridge_state::insert_client(bridge_state::ClientState::new(
        pale_sdk::PaleApi::new().unwrap(),
    ));
    for client in [foreign_id, owner_id] {
        let request = bridge_state::insert_request(
            serde_json::to_vec(&serde_json::json!({
                "handle":inspection_id,"profile":0
            }))
            .unwrap(),
        );
        let response = pale_go_improvement_explain(client, request);
        assert_eq!(
            pale_go_response_kind(response),
            bridge_state::ResponseKind::BridgeFailure as u8
        );
        assert_eq!(pale_go_response_object(response), 0);
        assert!(bridge_state::take_request(request).is_none());
        pale_go_response_drop(response);
        bridge_state::with_client(owner_id, |owner| {
            assert!(owner.improvement_inspections.contains_key(&inspection_id));
            assert!(owner.improvement_explanations.is_empty());
            Ok(())
        })
        .unwrap();
    }
    pale_go_client_drop(foreign_id);
    pale_go_client_drop(owner_id);
}

#[test]
fn surrendered_review_input_cleanup_is_exact_and_preserves_independent_values() {
    let fixture = fixtures::admitted_profile();
    let client = fixture.client;
    let (inspection, explanation, prepared) = bridge_state::with_client_mut(client, |state| {
        let baseline = state.workspaces.get(&fixture.workspace).unwrap();
        let profile = state.profiles.get(&fixture.profile).unwrap();
        let inspection = state.api.inspect_configuration_improvement(baseline);
        let explained = state
            .api
            .explain_configuration_improvement(
                state.api.inspect_configuration_improvement(baseline),
                profile,
            )
            .unwrap();
        let preparation = state
            .api
            .prepare_configuration_improvement_preview(
                &explained,
                baseline,
                fixture.candidate_request,
            )
            .unwrap();
        let ids = (
            bridge_state::next_object_handle(),
            bridge_state::next_object_handle(),
            bridge_state::next_object_handle(),
        );
        state.improvement_inspections.insert(ids.0, inspection);
        state.improvement_explanations.insert(ids.1, explained);
        state.improvement_prepared.insert(ids.2, preparation);
        Ok(ids)
    })
    .unwrap();
    let foreign = bridge_state::insert_client(bridge_state::ClientState::new(
        pale_sdk::PaleApi::new().unwrap(),
    ));
    // Raw C still preserves input custody on request rejection. Go's invocation
    // wrapper separately drops only the inputs it deliberately surrendered.
    let request =
        bridge_state::insert_request(br#"{"handle":0,"workspace":{"documents":null}}"#.to_vec());
    let response = pale_go_improvement_propose(client, request);
    assert_eq!(
        pale_go_response_kind(response),
        ResponseKind::BridgeFailure as u8
    );
    pale_go_response_drop(response);
    for wrong_client in [foreign, 0] {
        pale_go_improvement_inspection_drop(wrong_client, inspection);
        pale_go_improvement_explanation_drop(wrong_client, explanation);
        pale_go_improvement_prepared_drop(wrong_client, prepared);
    }
    pale_go_improvement_inspection_drop(client, explanation);
    pale_go_improvement_explanation_drop(client, prepared);
    pale_go_improvement_prepared_drop(client, inspection);
    bridge_state::with_client(client, |state| {
        assert_eq!(state.improvement_inspections.len(), 1);
        assert_eq!(state.improvement_explanations.len(), 1);
        assert_eq!(state.improvement_prepared.len(), 1);
        Ok(())
    })
    .unwrap();
    pale_go_improvement_inspection_drop(client, inspection);
    pale_go_improvement_inspection_drop(client, inspection);
    pale_go_improvement_explanation_drop(client, explanation);
    pale_go_improvement_explanation_drop(client, explanation);
    bridge_state::with_client(client, |state| {
        assert!(state.improvement_inspections.is_empty());
        assert!(state.improvement_explanations.is_empty());
        assert!(!state
            .improvement_prepared
            .get(&prepared)
            .unwrap()
            .summary()
            .canonical_bytes()
            .is_empty());
        Ok(())
    })
    .unwrap();
    pale_go_improvement_prepared_drop(client, prepared);
    pale_go_improvement_prepared_drop(client, prepared);
    bridge_state::with_client(client, |state| {
        assert!(state.improvement_prepared.is_empty());
        assert!(state.workspaces.contains_key(&fixture.workspace));
        assert!(state.profiles.contains_key(&fixture.profile));
        Ok(())
    })
    .unwrap();
    pale_go_client_drop(client);
    pale_go_improvement_inspection_drop(client, inspection);
    pale_go_improvement_explanation_drop(client, explanation);
    pale_go_improvement_prepared_drop(client, prepared);
    bridge_state::with_client(foreign, |state| {
        assert!(state.improvement_inspections.is_empty());
        assert!(state.improvement_explanations.is_empty());
        assert!(state.improvement_prepared.is_empty());
        Ok(())
    })
    .unwrap();
    pale_go_client_drop(foreign);
}
