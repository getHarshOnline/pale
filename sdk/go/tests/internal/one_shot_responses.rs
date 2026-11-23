//! Each one-shot output transfers once; discarding it never restores an input.

use super::*;
use base64::Engine as _;

#[derive(Clone, Copy, Debug)]
enum Output {
    Compilation,
    Candidate,
    Bound,
    Admitted,
}

fn client() -> u64 {
    bridge_state::insert_client(bridge_state::ClientState::new(
        pale_sdk::PaleApi::new().unwrap(),
    ))
}

fn counts(client: u64) -> [usize; 4] {
    bridge_state::with_client(client, |state| {
        Ok([
            state.compilations.len(),
            state.candidates.len(),
            state.bound_plans.len(),
            state.admitted_plans.len(),
        ])
    })
    .unwrap()
}

fn claim(response: bridge_state::Response) -> u64 {
    let object = response.object;
    assert_eq!(
        pale_go_response_claim(bridge_state::insert_response(response)),
        1
    );
    object
}

// Build predecessors through the actual owners; only the selected output is
// left pending. This helper creates no fake plan or materialization state.
fn pending(client: u64, output: Output) -> bridge_state::Response {
    let compilation = operations::compile_one_shot(
        client,
        &serde_json::to_vec(&fixtures::captured_one_shot_request()).unwrap(),
    )
    .unwrap();
    if matches!(output, Output::Compilation) {
        return compilation;
    }
    let compilation = claim(compilation);
    let candidate = operations::plan_one_shot(
        client,
        &fixtures::one_shot_plan_request(client, compilation),
    )
    .unwrap();
    if matches!(output, Output::Candidate) {
        return candidate;
    }
    let candidate = claim(candidate);
    let bound = operations::bind_one_shot_machine_effects(
        client,
        &serde_json::to_vec(&serde_json::json!({"candidate":candidate,"effects":[]})).unwrap(),
    )
    .unwrap();
    if matches!(output, Output::Bound) {
        return bound;
    }
    let saved = base64::engine::general_purpose::STANDARD.encode(&bound.payload);
    let bound = claim(bound);
    operations::admit_one_shot_saved_plan(
        client,
        &serde_json::to_vec(&serde_json::json!({"boundPlan":bound,"savedBytes":saved})).unwrap(),
    )
    .unwrap()
}

fn expected_before(output: Output) -> [usize; 4] {
    match output {
        Output::Compilation => [0, 0, 0, 0],
        _ => [1, 0, 0, 0],
    }
}

#[test]
fn discarded_one_shot_results_leave_no_output_or_revived_input() {
    for output in [
        Output::Compilation,
        Output::Candidate,
        Output::Bound,
        Output::Admitted,
    ] {
        let owner = client();
        let response = pending(owner, output);
        assert_ne!(response.object, 0);
        assert_eq!(counts(owner), expected_before(output), "{output:?}");
        let response = bridge_state::insert_response(response);
        pale_go_response_drop(response);
        assert_eq!(pale_go_response_claim(response), 0);
        assert_eq!(counts(owner), expected_before(output), "{output:?}");
        pale_go_client_drop(owner);
    }
}

#[test]
fn one_shot_results_transfer_once_to_only_the_original_client() {
    for (index, output) in [
        Output::Compilation,
        Output::Candidate,
        Output::Bound,
        Output::Admitted,
    ]
    .into_iter()
    .enumerate()
    {
        let owner = client();
        let foreign = client();
        let response = pending(owner, output);
        let object = response.object;
        let metadata = response.metadata.clone();
        let payload = response.payload.clone();
        let response = bridge_state::insert_response(response);
        assert_eq!(counts(owner), expected_before(output));
        assert_eq!(pale_go_response_claim(response), 1);
        assert_eq!(pale_go_response_claim(response), 0);
        pale_go_response_drop(response);
        let mut expected = expected_before(output);
        expected[index] += 1;
        assert_eq!(counts(owner), expected);
        assert_eq!(counts(foreign), [0; 4]);
        bridge_state::with_client(owner, |state| {
            let value: serde_json::Value = serde_json::from_slice(&metadata).unwrap();
            match output {
                Output::Compilation => {
                    assert_eq!(
                        metadata,
                        state.compilations[&object].summary().canonical_bytes()
                    );
                    assert!(payload.is_empty());
                }
                Output::Candidate => {
                    let candidate = &state.candidates[&object];
                    assert_eq!(payload, candidate.canonical_bytes());
                    assert_eq!(value.as_object().unwrap().len(), 14);
                    assert_eq!(value["kind"], "apply");
                    assert_eq!(value["repositoryId"], candidate.repository_id());
                    assert_eq!(value["planDigest"], candidate.plan_digest());
                    assert_eq!(
                        value["oneShotCompilationDigest"],
                        candidate.one_shot_compilation_digest()
                    );
                    assert_eq!(value["baselineCount"], candidate.baseline_count());
                    assert_eq!(value["operationCount"], candidate.operation_count());
                    assert_eq!(value["createCount"], candidate.create_count());
                    assert_eq!(value["adoptCount"], candidate.adopt_count());
                    assert_eq!(value["updateCount"], candidate.update_count());
                    assert_eq!(value["unchangedCount"], candidate.unchanged_count());
                    assert_eq!(value["deleteCount"], candidate.delete_count());
                    assert_eq!(value["spliceCount"], candidate.splice_count());
                    assert_eq!(
                        value["deferredEffectCount"],
                        candidate.deferred_effect_count()
                    );
                    assert_eq!(
                        value["localOnlyTargets"],
                        serde_json::to_value(candidate.local_only_targets()).unwrap()
                    );
                }
                Output::Bound => {
                    let plan = &state.bound_plans[&object];
                    assert_eq!(payload, plan.canonical_bytes());
                    assert_eq!(value.as_object().unwrap().len(), 2);
                    assert_eq!(value["kind"], "apply");
                    assert_eq!(value["repositoryId"], plan.repository_id());
                }
                Output::Admitted => {
                    let plan = &state.admitted_plans[&object];
                    assert!(payload.is_empty());
                    assert_eq!(value.as_object().unwrap().len(), 3);
                    assert_eq!(value["kind"], "apply");
                    assert_eq!(value["repositoryId"], plan.repository_id());
                    assert_eq!(value["planDigest"], plan.plan_digest());
                }
            }
            Ok(())
        })
        .unwrap();
        pale_go_client_drop(owner);
        pale_go_client_drop(foreign);
    }
}

#[test]
fn closing_a_pending_one_shot_owner_cannot_redirect_custody() {
    for output in [
        Output::Compilation,
        Output::Candidate,
        Output::Bound,
        Output::Admitted,
    ] {
        let owner = client();
        let foreign = client();
        let response = bridge_state::insert_response(pending(owner, output));
        pale_go_client_drop(owner);
        assert_eq!(pale_go_response_claim(response), 0);
        assert_eq!(pale_go_response_claim(response), 0);
        assert_eq!(counts(foreign), [0; 4]);
        pale_go_client_drop(foreign);
    }
}
