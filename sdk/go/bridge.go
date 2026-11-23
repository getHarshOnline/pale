package pale

/*
#cgo darwin LDFLAGS: -L${SRCDIR}/target/release -lpale_go_ffi -Wl,-rpath,${SRCDIR}/target/release
#cgo linux LDFLAGS: -L${SRCDIR}/target/release -lpale_go_ffi -Wl,-rpath,${SRCDIR}/target/release
#include "pale_go.h"
*/
import "C"

import (
	"encoding/json"
	"fmt"
	"runtime"
	"sync"
	"unsafe"
)

const (
	responseSuccess       = 0
	responseDiagnostic    = 1
	responseBridgeFailure = 2
)

type operation func(client, request uint64) uint64

func opImprovementInspectionState(client, request uint64) uint64 {
	return uint64(C.pale_go_improvement_inspection_state(C.uint64_t(client), C.uint64_t(request)))
}
func opImprovementExplanationState(client, request uint64) uint64 {
	return uint64(C.pale_go_improvement_explanation_state(C.uint64_t(client), C.uint64_t(request)))
}
func opImprovementEvaluationState(client, request uint64) uint64 {
	return uint64(C.pale_go_improvement_evaluation_state(C.uint64_t(client), C.uint64_t(request)))
}
func opImprovementDiagnosisState(client, request uint64) uint64 {
	return uint64(C.pale_go_improvement_diagnosis_state(C.uint64_t(client), C.uint64_t(request)))
}
func opImprovementProposalPhase(client, request uint64) uint64 {
	return uint64(C.pale_go_improvement_proposal_phase(C.uint64_t(client), C.uint64_t(request)))
}
func opImprovementPreviewPhase(client, request uint64) uint64 {
	return uint64(C.pale_go_improvement_preview_phase(C.uint64_t(client), C.uint64_t(request)))
}

func opImprovementInspect(client, request uint64) uint64 {
	return uint64(C.pale_go_improvement_inspect(C.uint64_t(client), C.uint64_t(request)))
}
func opImprovementExplain(client, request uint64) uint64 {
	return uint64(C.pale_go_improvement_explain(C.uint64_t(client), C.uint64_t(request)))
}
func opImprovementPropose(client, request uint64) uint64 {
	return uint64(C.pale_go_improvement_propose(C.uint64_t(client), C.uint64_t(request)))
}
func opImprovementEvaluate(client, request uint64) uint64 {
	return uint64(C.pale_go_improvement_evaluate(C.uint64_t(client), C.uint64_t(request)))
}
func opImprovementDiagnose(client, request uint64) uint64 {
	return uint64(C.pale_go_improvement_diagnose(C.uint64_t(client), C.uint64_t(request)))
}
func opImprovementPreview(client, request uint64) uint64 {
	return uint64(C.pale_go_improvement_preview(C.uint64_t(client), C.uint64_t(request)))
}
func opImprovementPrepare(client, request uint64) uint64 {
	return uint64(C.pale_go_improvement_prepare(C.uint64_t(client), C.uint64_t(request)))
}
func opImprovementCommit(client, request uint64) uint64 {
	return uint64(C.pale_go_improvement_commit(C.uint64_t(client), C.uint64_t(request)))
}
func opImprovementPreparedSummary(client, request uint64) uint64 {
	return uint64(C.pale_go_improvement_prepared_summary(C.uint64_t(client), C.uint64_t(request)))
}
func opImprovementSummary(client, request uint64) uint64 {
	return uint64(C.pale_go_improvement_summary(C.uint64_t(client), C.uint64_t(request)))
}
func opImprovementComparisonSummary(client, request uint64) uint64 {
	return uint64(C.pale_go_improvement_comparison_summary(C.uint64_t(client), C.uint64_t(request)))
}
func opImprovementChangeRoots(client, request uint64) uint64 {
	return uint64(C.pale_go_improvement_change_roots(C.uint64_t(client), C.uint64_t(request)))
}
func opImprovementImpacts(client, request uint64) uint64 {
	return uint64(C.pale_go_improvement_impacts(C.uint64_t(client), C.uint64_t(request)))
}
func opImprovementExplainComparison(client, request uint64) uint64 {
	return uint64(C.pale_go_improvement_explain_comparison(C.uint64_t(client), C.uint64_t(request)))
}
func opImprovementProfilePage(client, request uint64) uint64 {
	return uint64(C.pale_go_improvement_profile_page(C.uint64_t(client), C.uint64_t(request)))
}
func opImprovementExplainProfile(client, request uint64) uint64 {
	return uint64(C.pale_go_improvement_explain_profile(C.uint64_t(client), C.uint64_t(request)))
}
func opImprovementAccept(client, request uint64) uint64 {
	return uint64(C.pale_go_improvement_accept(C.uint64_t(client), C.uint64_t(request)))
}
func opImprovementReject(client, request uint64) uint64 {
	return uint64(C.pale_go_improvement_reject(C.uint64_t(client), C.uint64_t(request)))
}
func opImprovementCancel(client, request uint64) uint64 {
	return uint64(C.pale_go_improvement_cancel(C.uint64_t(client), C.uint64_t(request)))
}
func opImprovementMarkStale(client, request uint64) uint64 {
	return uint64(C.pale_go_improvement_mark_stale(C.uint64_t(client), C.uint64_t(request)))
}

type response struct {
	object   uint64
	metadata []byte
	payload  []byte
}

// Client retains opaque Rust-owned values produced by one API instance.
//
// Share the returned *Client between callers; do not copy the Client value.
// A Client is safe for concurrent use. Close it when finished; all values
// obtained from it become invalid together. Closing does not cancel a call or
// perform an Engine effect.
type Client struct {
	mu     sync.Mutex
	handle uint64
}

// NewClient verifies the embedded contract bundle and native-action registry.
func NewClient() (*Client, error) {
	result, err := takeResponse(uint64(C.pale_go_client_new()))
	if err != nil {
		return nil, err
	}
	client := &Client{handle: result.object}
	runtime.SetFinalizer(client, (*Client).finalize)
	return client, nil
}

func (client *Client) finalize() {
	client.Close()
}

// Close drops the API instance and all opaque values retained by it.
func (client *Client) Close() {
	if client == nil {
		return
	}
	client.mu.Lock()
	defer client.mu.Unlock()
	if client.handle == 0 {
		return
	}
	C.pale_go_client_drop(C.uint64_t(client.handle))
	client.handle = 0
	runtime.SetFinalizer(client, nil)
}

func (client *Client) call(request any, invoke operation) (response, error) {
	return client.callDecoded(request, invoke, nil)
}

// Copies of a handle share its atomic coordinate. Serialize consumption and
// native removal with calls and Client.Close; closing a stale copy is harmless.
func closeNativeHandle(value *handle, drop func(uint64, uint64)) {
	if value == nil || value.client == nil {
		return
	}
	client := value.client
	client.mu.Lock()
	defer client.mu.Unlock()
	object := value.consume()
	if object != 0 && client.handle != 0 {
		drop(client.handle, object)
	}
}

func dropPortableWorkspace(client, object uint64) {
	C.pale_go_portable_workspace_drop(C.uint64_t(client), C.uint64_t(object))
}

func dropWorkspaceComparison(client, object uint64) {
	C.pale_go_workspace_comparison_drop(C.uint64_t(client), C.uint64_t(object))
}

func dropEffectiveProfile(client, object uint64) {
	C.pale_go_effective_profile_drop(C.uint64_t(client), C.uint64_t(object))
}

// Canonical documents remain opaque SDK bytes; only envelope presence is
// checked here. No Go projection recomputes a digest or re-admits these bytes.
func decodeCanonicalHandleResponse(result response) error {
	if result.object == 0 || len(result.metadata) == 0 || len(result.payload) != 0 {
		return fmt.Errorf("incomplete canonical document and handle response")
	}
	return nil
}

func dropObservationSnapshot(client, object uint64) {
	C.pale_go_observation_snapshot_drop(C.uint64_t(client), C.uint64_t(object))
}

func dropAuthoringIdentity(client, object uint64) {
	C.pale_go_authoring_identity_drop(C.uint64_t(client), C.uint64_t(object))
}

func dropAuthoringOccurrenceCursor(client, object uint64) {
	C.pale_go_authoring_occurrence_cursor_drop(C.uint64_t(client), C.uint64_t(object))
}

func dropEffectiveAuthoringCursor(client, object uint64) {
	C.pale_go_effective_authoring_cursor_drop(C.uint64_t(client), C.uint64_t(object))
}

func dropObservationPageCursor(client, object uint64) {
	C.pale_go_observation_page_cursor_drop(C.uint64_t(client), C.uint64_t(object))
}

func dropObservationLensCursor(client, object uint64) {
	C.pale_go_observation_lens_cursor_drop(C.uint64_t(client), C.uint64_t(object))
}

func dropObservationProfileCursor(client, object uint64) {
	C.pale_go_observation_profile_cursor_drop(C.uint64_t(client), C.uint64_t(object))
}

func opObservationInspectProfile(client, request uint64) uint64 {
	return uint64(C.pale_go_observation_inspect_profile(C.uint64_t(client), C.uint64_t(request)))
}

func dropObservationSubject(client, object uint64) {
	C.pale_go_observation_subject_drop(C.uint64_t(client), C.uint64_t(object))
}

func dropImprovementInspection(client, object uint64) {
	C.pale_go_improvement_inspection_drop(C.uint64_t(client), C.uint64_t(object))
}

func dropImprovementExplanation(client, object uint64) {
	C.pale_go_improvement_explanation_drop(C.uint64_t(client), C.uint64_t(object))
}

func dropImprovementPrepared(client, object uint64) {
	C.pale_go_improvement_prepared_drop(C.uint64_t(client), C.uint64_t(object))
}

func dropOneShotCandidate(client, object uint64) {
	C.pale_go_one_shot_candidate_drop(C.uint64_t(client), C.uint64_t(object))
}

func dropOneShotBoundPlan(client, object uint64) {
	C.pale_go_one_shot_bound_plan_drop(C.uint64_t(client), C.uint64_t(object))
}

// Keep pending native values response-owned until the complete Go
// projection passes. Close cannot race between invocation, decoding and claim.
func (client *Client) callDecoded(request any, invoke operation, decode func(response) error) (response, error) {
	if client == nil {
		return response{}, &BridgeError{Code: "closed_client", Message: "Pale client is nil"}
	}
	encoded, err := json.Marshal(request)
	if err != nil {
		return response{}, &BridgeError{Code: "invalid_go_request", Message: err.Error()}
	}
	client.mu.Lock()
	defer client.mu.Unlock()
	return client.callEncodedLocked(encoded, invoke, decode)
}

// The caller holds client.mu while validating related handles. invoke is not
// entered until request encoding and native request creation have succeeded.
func (client *Client) callLocked(request any, invoke operation) (response, error) {
	return client.callDecodedLocked(request, invoke, nil)
}

// The caller retains client.mu through incoming handover and output decoding.
func (client *Client) callDecodedLocked(request any, invoke operation, decode func(response) error) (response, error) {
	encoded, err := json.Marshal(request)
	if err != nil {
		return response{}, &BridgeError{Code: "invalid_go_request", Message: err.Error()}
	}
	return client.callEncodedLocked(encoded, invoke, decode)
}

// client.mu protects the entire native invocation, decoding and response claim.
func (client *Client) callEncodedLocked(encoded []byte, invoke operation, decode func(response) error) (response, error) {
	if client.handle == 0 {
		return response{}, &BridgeError{Code: "closed_client", Message: "Pale client is closed"}
	}
	var pointer *C.uint8_t
	if len(encoded) != 0 {
		pointer = (*C.uint8_t)(unsafe.Pointer(&encoded[0]))
	}
	requestHandle := uint64(C.pale_go_request_create(pointer, C.size_t(len(encoded))))
	if requestHandle == 0 {
		return response{}, &BridgeError{Code: "request_allocation_failed", Message: "Rust did not retain the request"}
	}
	responseHandle := invoke(client.handle, requestHandle)
	defer C.pale_go_response_drop(C.uint64_t(responseHandle))
	result, err := readResponse(responseHandle)
	if err != nil {
		return response{}, err
	}
	if decode != nil {
		if err := decode(result); err != nil {
			return response{}, &BridgeError{Code: "invalid_response_metadata", Message: err.Error()}
		}
	}
	if C.pale_go_response_claim(C.uint64_t(responseHandle)) != 1 {
		return response{}, &BridgeError{Code: "invalid_response_custody", Message: "native response custody could not be transferred"}
	}
	return result, nil
}

func takeResponse(handle uint64) (response, error) {
	defer C.pale_go_response_drop(C.uint64_t(handle))
	return readResponse(handle)
}

// Copies native buffers into independent Go allocations. A caller may return
// those slices directly; they survive response removal and Client.Close.
func readResponse(handle uint64) (response, error) {
	if handle == 0 {
		return response{}, &BridgeError{Code: "missing_response", Message: "Rust returned no response handle"}
	}
	metadata, err := copyResponsePart(handle, true)
	if err != nil {
		return response{}, err
	}
	payload, err := copyResponsePart(handle, false)
	if err != nil {
		return response{}, err
	}
	result := response{
		object:   uint64(C.pale_go_response_object(C.uint64_t(handle))),
		metadata: metadata,
		payload:  payload,
	}
	switch uint8(C.pale_go_response_kind(C.uint64_t(handle))) {
	case responseSuccess:
		return result, nil
	case responseDiagnostic:
		return response{}, newDiagnosticError(result.payload, result.metadata)
	case responseBridgeFailure:
		var failure BridgeError
		if err := json.Unmarshal(result.metadata, &failure); err != nil {
			return response{}, &BridgeError{Code: "invalid_bridge_failure", Message: fmt.Sprintf("Rust returned malformed bridge failure: %v", err)}
		}
		return response{}, &failure
	default:
		return response{}, &BridgeError{Code: "unknown_response_kind", Message: "Rust returned an unknown response class"}
	}
}

func copyResponsePart(handle uint64, metadata bool) ([]byte, error) {
	var length C.size_t
	if metadata {
		length = C.pale_go_response_metadata_length(C.uint64_t(handle))
	} else {
		length = C.pale_go_response_payload_length(C.uint64_t(handle))
	}
	if length == 0 {
		return nil, nil
	}
	bytes := make([]byte, int(length))
	pointer := (*C.uint8_t)(unsafe.Pointer(&bytes[0]))
	var copied C.size_t
	if metadata {
		copied = C.pale_go_response_copy_metadata(C.uint64_t(handle), pointer, length)
	} else {
		copied = C.pale_go_response_copy_payload(C.uint64_t(handle), pointer, length)
	}
	if copied != length {
		return nil, &BridgeError{Code: "incomplete_response_copy", Message: "Rust response changed while Go copied it"}
	}
	return bytes, nil
}

func decodeMetadata[T any](result response) (T, error) {
	var value T
	if err := json.Unmarshal(result.metadata, &value); err != nil {
		return value, &BridgeError{Code: "invalid_response_metadata", Message: err.Error()}
	}
	return value, nil
}

// BridgeError reports a failure in Go/Rust translation before an API-owned
// diagnostic exists.
type BridgeError struct {
	Code    string `json:"code"`
	Message string `json:"message"`
}

func (failure *BridgeError) Error() string {
	return failure.Code + ": " + failure.Message
}

func opCapabilities(client, request uint64) uint64 {
	return uint64(C.pale_go_capabilities(C.uint64_t(client), C.uint64_t(request)))
}
func opContractBundleIdentity(client, request uint64) uint64 {
	return uint64(C.pale_go_contract_bundle_identity(C.uint64_t(client), C.uint64_t(request)))
}
func opContractSchema(client, request uint64) uint64 {
	return uint64(C.pale_go_contract_schema(C.uint64_t(client), C.uint64_t(request)))
}
func opValidateContractDocument(client, request uint64) uint64 {
	return uint64(C.pale_go_validate_contract_document(C.uint64_t(client), C.uint64_t(request)))
}
func opProjectDiagnostic(client, request uint64) uint64 {
	return uint64(C.pale_go_project_diagnostic(C.uint64_t(client), C.uint64_t(request)))
}
func opAdmitDiagnostic(client, request uint64) uint64 {
	return uint64(C.pale_go_admit_diagnostic(C.uint64_t(client), C.uint64_t(request)))
}
func opPackageDistribution(client, request uint64) uint64 {
	return uint64(C.pale_go_inspect_package_distribution(C.uint64_t(client), C.uint64_t(request)))
}
func opAuthoringIngressLimits(client, request uint64) uint64 {
	return uint64(C.pale_go_authoring_ingress_limits(C.uint64_t(client), C.uint64_t(request)))
}
func opValidateAuthoringDocument(client, request uint64) uint64 {
	return uint64(C.pale_go_validate_authoring_document(C.uint64_t(client), C.uint64_t(request)))
}
func opInspectStandaloneAuthoring(client, request uint64) uint64 {
	return uint64(C.pale_go_inspect_standalone_authoring(C.uint64_t(client), C.uint64_t(request)))
}
func opCompilePortableWorkspace(client, request uint64) uint64 {
	return uint64(C.pale_go_compile_portable_workspace(C.uint64_t(client), C.uint64_t(request)))
}
func opInspectPortableAuthoring(client, request uint64) uint64 {
	return uint64(C.pale_go_inspect_portable_workspace_authoring(C.uint64_t(client), C.uint64_t(request)))
}
func opNavigatePortableAuthoring(client, request uint64) uint64 {
	return uint64(C.pale_go_navigate_portable_workspace_authoring(C.uint64_t(client), C.uint64_t(request)))
}
func opPortableAuthoringOccurrences(client, request uint64) uint64 {
	return uint64(C.pale_go_portable_authoring_occurrences(C.uint64_t(client), C.uint64_t(request)))
}
func opEffectiveAuthoringOccurrences(client, request uint64) uint64 {
	return uint64(C.pale_go_effective_authoring_occurrences(C.uint64_t(client), C.uint64_t(request)))
}
func opEffectiveAuthoringPage(client, request uint64) uint64 {
	return uint64(C.pale_go_effective_authoring_page(C.uint64_t(client), C.uint64_t(request)))
}
func opComparePortableWorkspaces(client, request uint64) uint64 {
	return uint64(C.pale_go_compare_portable_workspaces(C.uint64_t(client), C.uint64_t(request)))
}
func opPortableChangeRoots(client, request uint64) uint64 {
	return uint64(C.pale_go_portable_workspace_change_roots(C.uint64_t(client), C.uint64_t(request)))
}
func opPortableImpacts(client, request uint64) uint64 {
	return uint64(C.pale_go_portable_workspace_impacts(C.uint64_t(client), C.uint64_t(request)))
}
func opExplainPortableWorkspace(client, request uint64) uint64 {
	return uint64(C.pale_go_explain_portable_workspace(C.uint64_t(client), C.uint64_t(request)))
}
func opComparePortableWorkspaceSubject(client, request uint64) uint64 {
	return uint64(C.pale_go_compare_portable_workspace_subject(C.uint64_t(client), C.uint64_t(request)))
}
func opCompileEffectiveProfile(client, request uint64) uint64 {
	return uint64(C.pale_go_compile_effective_profile(C.uint64_t(client), C.uint64_t(request)))
}
func opInspectEffectiveProfile(client, request uint64) uint64 {
	return uint64(C.pale_go_inspect_effective_profile(C.uint64_t(client), C.uint64_t(request)))
}
func opExplainEffectiveProfile(client, request uint64) uint64 {
	return uint64(C.pale_go_explain_effective_profile(C.uint64_t(client), C.uint64_t(request)))
}
func opInspectEffectiveAuthoring(client, request uint64) uint64 {
	return uint64(C.pale_go_inspect_effective_profile_authoring(C.uint64_t(client), C.uint64_t(request)))
}
func opNavigateEffectiveAuthoring(client, request uint64) uint64 {
	return uint64(C.pale_go_navigate_effective_profile_authoring(C.uint64_t(client), C.uint64_t(request)))
}
func opJoinEffectiveAuthoring(client, request uint64) uint64 {
	return uint64(C.pale_go_join_effective_profile_authoring(C.uint64_t(client), C.uint64_t(request)))
}
func opObserveEffectiveProfile(client, request uint64) uint64 {
	return uint64(C.pale_go_observe_effective_profile(C.uint64_t(client), C.uint64_t(request)))
}
func opObservationPage(client, request uint64) uint64 {
	return uint64(C.pale_go_observation_page(C.uint64_t(client), C.uint64_t(request)))
}
func opObservationDelta(client, request uint64) uint64 {
	return uint64(C.pale_go_observation_delta(C.uint64_t(client), C.uint64_t(request)))
}
func opObservationCausalPath(client, request uint64) uint64 {
	return uint64(C.pale_go_observation_causal_path(C.uint64_t(client), C.uint64_t(request)))
}
func opObservationVisualLens(client, request uint64) uint64 {
	return uint64(C.pale_go_observation_visual_lens(C.uint64_t(client), C.uint64_t(request)))
}
func opNativeActionCatalog(client, request uint64) uint64 {
	return uint64(C.pale_go_native_action_catalog(C.uint64_t(client), C.uint64_t(request)))
}
func opInvokeNativeAction(client, request uint64) uint64 {
	return uint64(C.pale_go_invoke_native_action(C.uint64_t(client), C.uint64_t(request)))
}
func opInvokeNativeActionPayload(client, request uint64) uint64 {
	return uint64(C.pale_go_invoke_native_action_payload(C.uint64_t(client), C.uint64_t(request)))
}
func opAdmitRepositoryNativeActions(client, request uint64) uint64 {
	return uint64(C.pale_go_admit_repository_native_actions(C.uint64_t(client), C.uint64_t(request)))
}
func opResidentHookAdmission(client, request uint64) uint64 {
	return uint64(C.pale_go_admit_resident_hook(C.uint64_t(client), C.uint64_t(request)))
}
func opCompileOneShot(client, request uint64) uint64 {
	return uint64(C.pale_go_compile_one_shot(C.uint64_t(client), C.uint64_t(request)))
}
func opOneShotSummary(client, request uint64) uint64 {
	return uint64(C.pale_go_one_shot_summary(C.uint64_t(client), C.uint64_t(request)))
}
func opOneShotObservationTargets(client, request uint64) uint64 {
	return uint64(C.pale_go_one_shot_observation_targets(C.uint64_t(client), C.uint64_t(request)))
}
func opPlanOneShot(client, request uint64) uint64 {
	return uint64(C.pale_go_plan_one_shot(C.uint64_t(client), C.uint64_t(request)))
}
func opBindOneShotMachineEffects(client, request uint64) uint64 {
	return uint64(C.pale_go_bind_one_shot_machine_effects(C.uint64_t(client), C.uint64_t(request)))
}
func opAdmitOneShotSavedPlan(client, request uint64) uint64 {
	return uint64(C.pale_go_admit_one_shot_saved_plan(C.uint64_t(client), C.uint64_t(request)))
}
func opCompileLocalOneShot(client, request uint64) uint64 {
	return uint64(C.pale_go_compile_local_one_shot(C.uint64_t(client), C.uint64_t(request)))
}
func opPreviewLocalConfigurationImprovement(client, request uint64) uint64 {
	return uint64(C.pale_go_preview_local_configuration_improvement(C.uint64_t(client), C.uint64_t(request)))
}
func opInspectLocalPackageDistribution(client, request uint64) uint64 {
	return uint64(C.pale_go_inspect_local_package_distribution(C.uint64_t(client), C.uint64_t(request)))
}
func opCaptureLocalNativeAction(client, request uint64) uint64 {
	return uint64(C.pale_go_capture_local_native_action_request(C.uint64_t(client), C.uint64_t(request)))
}
func opInvokeCapturedNativeAction(client, request uint64) uint64 {
	return uint64(C.pale_go_invoke_captured_native_action(C.uint64_t(client), C.uint64_t(request)))
}
func opInspectLocalRepositoryStatus(client, request uint64) uint64 {
	return uint64(C.pale_go_inspect_local_repository_status(C.uint64_t(client), C.uint64_t(request)))
}
func opPlanLocalOneShot(client, request uint64) uint64 {
	return uint64(C.pale_go_plan_local_one_shot(C.uint64_t(client), C.uint64_t(request)))
}
func opPublishLocalOneShotPlan(client, request uint64) uint64 {
	return uint64(C.pale_go_publish_local_one_shot_plan(C.uint64_t(client), C.uint64_t(request)))
}
func opAdmitLocalMaterializationRun(client, request uint64) uint64 {
	return uint64(C.pale_go_admit_local_materialization_run(C.uint64_t(client), C.uint64_t(request)))
}
func opSettleLocalMaterializationRun(client, request uint64) uint64 {
	return uint64(C.pale_go_settle_local_materialization_run(C.uint64_t(client), C.uint64_t(request)))
}
func opInspectLocalReconciliation(client, request uint64) uint64 {
	return uint64(C.pale_go_inspect_local_materialization_reconciliation(C.uint64_t(client), C.uint64_t(request)))
}
func opCompiledInterfaceIdentity(client, request uint64) uint64 {
	return uint64(C.pale_go_compiled_interface_identity(C.uint64_t(client), C.uint64_t(request)))
}
func opVerifyCurrentExecutableBinding(client, request uint64) uint64 {
	return uint64(C.pale_go_verify_current_executable_binding(C.uint64_t(client), C.uint64_t(request)))
}
func opInspectLocalControlStore(client, request uint64) uint64 {
	return uint64(C.pale_go_inspect_local_control_store(C.uint64_t(client), C.uint64_t(request)))
}
