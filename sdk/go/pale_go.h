#ifndef PALE_GO_H
#define PALE_GO_H

#include <stddef.h>
#include <stdint.h>
uint64_t pale_go_improvement_inspection_state(uint64_t client, uint64_t request);
uint64_t pale_go_improvement_explanation_state(uint64_t client, uint64_t request);
uint64_t pale_go_improvement_evaluation_state(uint64_t client, uint64_t request);
uint64_t pale_go_improvement_diagnosis_state(uint64_t client, uint64_t request);
uint64_t pale_go_improvement_proposal_phase(uint64_t client, uint64_t request);
uint64_t pale_go_improvement_preview_phase(uint64_t client, uint64_t request);
uint64_t pale_go_improvement_inspect(uint64_t client, uint64_t request);
uint64_t pale_go_improvement_explain(uint64_t client, uint64_t request);
uint64_t pale_go_improvement_propose(uint64_t client, uint64_t request);
uint64_t pale_go_improvement_evaluate(uint64_t client, uint64_t request);
uint64_t pale_go_improvement_diagnose(uint64_t client, uint64_t request);
uint64_t pale_go_improvement_preview(uint64_t client, uint64_t request);
uint64_t pale_go_improvement_prepare(uint64_t client, uint64_t request);
uint64_t pale_go_improvement_commit(uint64_t client, uint64_t request);
uint64_t pale_go_improvement_prepared_summary(uint64_t client, uint64_t request);
uint64_t pale_go_improvement_summary(uint64_t client, uint64_t request);
uint64_t pale_go_improvement_comparison_summary(uint64_t client, uint64_t request);
uint64_t pale_go_improvement_change_roots(uint64_t client, uint64_t request);
uint64_t pale_go_improvement_impacts(uint64_t client, uint64_t request);
uint64_t pale_go_improvement_explain_comparison(uint64_t client, uint64_t request);
uint64_t pale_go_improvement_profile_page(uint64_t client, uint64_t request);
uint64_t pale_go_improvement_explain_profile(uint64_t client, uint64_t request);
uint64_t pale_go_improvement_accept(uint64_t client, uint64_t request);
uint64_t pale_go_improvement_reject(uint64_t client, uint64_t request);
uint64_t pale_go_improvement_cancel(uint64_t client, uint64_t request);
uint64_t pale_go_improvement_mark_stale(uint64_t client, uint64_t request);

/* Response disposition. Success metadata/payload are operation-specific. */
/* Diagnostic payload is exact diagnostic.v1; metadata retains SDK identity,
 * authoringCorrections and nullable authoringLocation, with no object handle. */
enum pale_go_response_kind {
  PALE_GO_SUCCESS = 0,
  PALE_GO_DIAGNOSTIC = 1,
  PALE_GO_BRIDGE_FAILURE = 2
};

uint64_t pale_go_request_create(const uint8_t *bytes, size_t length);
void pale_go_request_drop(uint64_t request);

uint64_t pale_go_client_new(void);
void pale_go_client_drop(uint64_t client);
/* Idempotent, client-scoped release; no cancellation or cross-object cascade. */
void pale_go_portable_workspace_drop(uint64_t client, uint64_t workspace);
void pale_go_workspace_comparison_drop(uint64_t client, uint64_t comparison);
void pale_go_effective_profile_drop(uint64_t client, uint64_t profile);
void pale_go_observation_snapshot_drop(uint64_t client, uint64_t snapshot);
void pale_go_observation_page_cursor_drop(uint64_t client, uint64_t cursor);
void pale_go_observation_lens_cursor_drop(uint64_t client, uint64_t cursor);
void pale_go_observation_profile_cursor_drop(uint64_t client, uint64_t cursor);
void pale_go_observation_subject_drop(uint64_t client, uint64_t subject);
/* Drop only this client's authoring identity; missing handles are harmless. */
void pale_go_authoring_identity_drop(uint64_t client, uint64_t identity);
void pale_go_authoring_occurrence_cursor_drop(uint64_t client, uint64_t cursor);
void pale_go_effective_authoring_cursor_drop(uint64_t client, uint64_t cursor);
/* Exact residual-input cleanup after Go surrenders an improvement handle. */
void pale_go_improvement_inspection_drop(uint64_t client, uint64_t inspection);
void pale_go_improvement_explanation_drop(uint64_t client, uint64_t explanation);
void pale_go_improvement_prepared_drop(uint64_t client, uint64_t preparation);

uint8_t pale_go_response_kind(uint64_t response);
uint64_t pale_go_response_object(uint64_t response);
size_t pale_go_response_metadata_length(uint64_t response);
size_t pale_go_response_payload_length(uint64_t response);
size_t pale_go_response_copy_metadata(uint64_t response, uint8_t *destination, size_t capacity);
size_t pale_go_response_copy_payload(uint64_t response, uint8_t *destination, size_t capacity);
void pale_go_response_drop(uint64_t response);
/* Claim decoded native values; consumes the response, returns 1 or 0. */
uint8_t pale_go_response_claim(uint64_t response);

uint64_t pale_go_capabilities(uint64_t client, uint64_t request);
uint64_t pale_go_contract_bundle_identity(uint64_t client, uint64_t request);
uint64_t pale_go_contract_schema(uint64_t client, uint64_t request);
uint64_t pale_go_validate_contract_document(uint64_t client, uint64_t request);
uint64_t pale_go_project_diagnostic(uint64_t client, uint64_t request);
uint64_t pale_go_admit_diagnostic(uint64_t client, uint64_t request);
uint64_t pale_go_authoring_ingress_limits(uint64_t client, uint64_t request);
uint64_t pale_go_validate_authoring_document(uint64_t client, uint64_t request);
uint64_t pale_go_inspect_standalone_authoring(uint64_t client, uint64_t request);
uint64_t pale_go_compile_portable_workspace(uint64_t client, uint64_t request);
uint64_t pale_go_inspect_portable_workspace_authoring(uint64_t client, uint64_t request);
uint64_t pale_go_navigate_portable_workspace_authoring(uint64_t client, uint64_t request);
uint64_t pale_go_portable_authoring_occurrences(uint64_t client, uint64_t request);
uint64_t pale_go_effective_authoring_occurrences(uint64_t client, uint64_t request);
uint64_t pale_go_effective_authoring_page(uint64_t client, uint64_t request);
uint64_t pale_go_compare_portable_workspaces(uint64_t client, uint64_t request);
uint64_t pale_go_portable_workspace_change_roots(uint64_t client, uint64_t request);
uint64_t pale_go_portable_workspace_impacts(uint64_t client, uint64_t request);
uint64_t pale_go_explain_portable_workspace(uint64_t client, uint64_t request);
uint64_t pale_go_compare_portable_workspace_subject(uint64_t client, uint64_t request);
uint64_t pale_go_compile_effective_profile(uint64_t client, uint64_t request);
uint64_t pale_go_inspect_effective_profile(uint64_t client, uint64_t request);
uint64_t pale_go_explain_effective_profile(uint64_t client, uint64_t request);
uint64_t pale_go_inspect_effective_profile_authoring(uint64_t client, uint64_t request);
uint64_t pale_go_navigate_effective_profile_authoring(uint64_t client, uint64_t request);
uint64_t pale_go_join_effective_profile_authoring(uint64_t client, uint64_t request);
uint64_t pale_go_observe_effective_profile(uint64_t client, uint64_t request);
uint64_t pale_go_observation_page(uint64_t client, uint64_t request);
uint64_t pale_go_observation_inspect_profile(uint64_t client, uint64_t request);
uint64_t pale_go_observation_delta(uint64_t client, uint64_t request);
uint64_t pale_go_observation_causal_path(uint64_t client, uint64_t request);
uint64_t pale_go_observation_visual_lens(uint64_t client, uint64_t request);
uint64_t pale_go_native_action_catalog(uint64_t client, uint64_t request);
uint64_t pale_go_invoke_native_action(uint64_t client, uint64_t request);
uint64_t pale_go_invoke_native_action_payload(uint64_t client, uint64_t request);
uint64_t pale_go_admit_repository_native_actions(uint64_t client, uint64_t request);
uint64_t pale_go_compile_one_shot(uint64_t client, uint64_t request);
uint64_t pale_go_one_shot_summary(uint64_t client, uint64_t request);
uint64_t pale_go_one_shot_observation_targets(uint64_t client, uint64_t request);
uint64_t pale_go_plan_one_shot(uint64_t client, uint64_t request);
uint64_t pale_go_bind_one_shot_machine_effects(uint64_t client, uint64_t request);
uint64_t pale_go_admit_one_shot_saved_plan(uint64_t client, uint64_t request);
/* Discard only a surrendered input in this client; no admission or effect. */
void pale_go_one_shot_candidate_drop(uint64_t client, uint64_t candidate);
void pale_go_one_shot_bound_plan_drop(uint64_t client, uint64_t plan);
uint64_t pale_go_inspect_package_distribution(uint64_t client, uint64_t request);
uint64_t pale_go_admit_resident_hook(uint64_t client, uint64_t request);
uint64_t pale_go_compile_local_one_shot(uint64_t client, uint64_t request);
uint64_t pale_go_preview_local_configuration_improvement(uint64_t client, uint64_t request);
uint64_t pale_go_inspect_local_package_distribution(uint64_t client, uint64_t request);
uint64_t pale_go_capture_local_native_action_request(uint64_t client, uint64_t request);
uint64_t pale_go_invoke_captured_native_action(uint64_t client, uint64_t request);
uint64_t pale_go_inspect_local_repository_status(uint64_t client, uint64_t request);
uint64_t pale_go_plan_local_one_shot(uint64_t client, uint64_t request);
uint64_t pale_go_publish_local_one_shot_plan(uint64_t client, uint64_t request);
uint64_t pale_go_admit_local_materialization_run(uint64_t client, uint64_t request);
uint64_t pale_go_settle_local_materialization_run(uint64_t client, uint64_t request);
uint64_t pale_go_inspect_local_materialization_reconciliation(uint64_t client, uint64_t request);
uint64_t pale_go_compiled_interface_identity(uint64_t client, uint64_t request);
uint64_t pale_go_verify_current_executable_binding(uint64_t client, uint64_t request);
uint64_t pale_go_inspect_local_control_store(uint64_t client, uint64_t request);

#endif
