//! C ABI bridge behind the idiomatic `pale-go` package.
//!
//! This crate builds on [`pale_sdk`], which remains the sole semantic owner, and
//! the Go package links it. It translates bounded Go-owned values, retains API
//! objects behind process-local opaque handles, and transports exact canonical
//! diagnostics. It defines no compiler, validator, policy, graph, admission,
//! cancellation, or effect semantics.
//!
//! The ABI intentionally has one small unsafe surface: copying bytes across the
//! C boundary. Requests and responses otherwise use integer handles, so neither
//! Rust pointers nor C types escape into the exported Go package.

mod authoring_pages;
mod bridge_state;
mod improvement;
mod observation_profile;
mod operations;
mod wire;

use std::panic::{catch_unwind, AssertUnwindSafe};

use bridge_state::{Response, ResponseKind};
use serde::Serialize;

/// A failure in host translation before an API-owned diagnostic exists.
///
/// Go exposes this separately from its diagnostic error type. Consequently an
/// invalid opaque handle, malformed private bridge request, or caught Rust
/// panic cannot masquerade as a Pale application admission result.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BridgeFailure {
    code: &'static str,
    message: String,
}

impl BridgeFailure {
    pub(crate) fn request(message: impl Into<String>) -> Self {
        Self {
            code: "invalid_binding_request",
            message: message.into(),
        }
    }

    pub(crate) fn invalid_handle(kind: &'static str, handle: u64) -> Self {
        Self {
            code: "invalid_binding_handle",
            message: format!("{kind} handle {handle} is absent or belongs to another client"),
        }
    }

    fn panic() -> Self {
        Self {
            code: "binding_panic",
            message: "the Rust binding aborted the call before an API result existed".to_owned(),
        }
    }

    fn initialization(error: pale_sdk::ApiInitializationError) -> Self {
        Self {
            code: error.code(),
            message: error.to_string(),
        }
    }
}

pub(crate) fn success(metadata: Vec<u8>, payload: Vec<u8>, object: u64) -> Response {
    Response {
        kind: ResponseKind::Success,
        object,
        metadata,
        payload,
        pending_values: None,
    }
}

pub(crate) fn empty_success() -> Response {
    success(Vec::new(), Vec::new(), 0)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticMetadata<'a> {
    diagnostic_identity: &'a pale_sdk::DiagnosticIdentity,
    authoring_corrections: &'a [pale_sdk::AuthoringCorrection],
    authoring_location: Option<&'a pale_sdk::AuthoringSourceLocation>,
}

pub(crate) fn diagnostic(error: pale_sdk::ApiError) -> Result<Response, BridgeFailure> {
    let metadata = encode(&DiagnosticMetadata {
        diagnostic_identity: error.diagnostic().identity(),
        authoring_corrections: error.authoring_corrections(),
        authoring_location: error.authoring_location(),
    })?;
    Ok(Response {
        kind: ResponseKind::Diagnostic,
        object: 0,
        metadata,
        payload: error.diagnostic().canonical_bytes().to_vec(),
        pending_values: None,
    })
}

pub(crate) fn bridge_failure(error: BridgeFailure) -> Response {
    let metadata = serde_json::to_vec(&error).unwrap_or_else(|_| {
        br#"{"code":"binding_failure","message":"binding error projection failed"}"#.to_vec()
    });
    Response {
        kind: ResponseKind::BridgeFailure,
        object: 0,
        metadata,
        payload: Vec::new(),
        pending_values: None,
    }
}

pub(crate) fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, BridgeFailure> {
    serde_json::to_vec(value)
        .map_err(|error| BridgeFailure::request(format!("binding projection failed: {error}")))
}

pub(crate) fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, BridgeFailure> {
    serde_json::from_slice(bytes)
        .map_err(|error| BridgeFailure::request(format!("invalid binding request: {error}")))
}

#[cfg(test)]
#[path = "../tests/internal/lib.rs"]
mod tests;

fn response_from_call(call: impl FnOnce() -> Result<Response, BridgeFailure>) -> u64 {
    let response = catch_unwind(AssertUnwindSafe(call))
        .unwrap_or_else(|_| Err(BridgeFailure::panic()))
        .unwrap_or_else(bridge_failure);
    bridge_state::insert_response(response)
}

fn operation(
    client: u64,
    request: u64,
    call: fn(u64, &[u8]) -> Result<Response, BridgeFailure>,
) -> u64 {
    response_from_call(|| {
        let request = bridge_state::take_request(request)
            .ok_or_else(|| BridgeFailure::invalid_handle("request", request))?;
        call(client, &request)
    })
}

/// Copy a complete Go request into Rust-owned memory.
///
/// Returns zero only when a non-empty request is paired with a null pointer.
/// The returned handle is consumed by exactly one operation call.
///
/// # Safety
///
/// For a non-zero `length`, `bytes` must be readable for exactly `length`
/// bytes for the duration of this call.
#[no_mangle]
pub unsafe extern "C" fn pale_go_request_create(bytes: *const u8, length: usize) -> u64 {
    if length != 0 && bytes.is_null() {
        return 0;
    }
    let owned = if length == 0 {
        Vec::new()
    } else {
        // SAFETY: the caller contract above requires a live `length`-byte
        // region. The slice is copied before this function returns.
        unsafe { std::slice::from_raw_parts(bytes, length) }.to_vec()
    };
    bridge_state::insert_request(owned)
}

/// Drop an unconsumed request handle. Unknown handles are inert.
#[no_mangle]
pub extern "C" fn pale_go_request_drop(request: u64) {
    bridge_state::drop_request(request);
}

/// Verify the embedded façade and create one process-local Go client.
///
/// The returned value is a response handle. On success its object coordinate
/// is the client handle; initialization failures remain pre-diagnostic bridge
/// failures because [`pale_sdk::PaleApi::new`] could not establish the
/// diagnostic contract.
#[no_mangle]
pub extern "C" fn pale_go_client_new() -> u64 {
    response_from_call(|| {
        pale_sdk::PaleApi::new()
            .map(bridge_state::ClientState::new)
            .map(bridge_state::insert_client)
            .map(|client| success(Vec::new(), Vec::new(), client))
            .map_err(BridgeFailure::initialization)
    })
}

/// Drop a Go client and every opaque API value retained by it.
///
/// Dropping a handle performs no Pale operation or cancellation transition.
#[no_mangle]
pub extern "C" fn pale_go_client_drop(client: u64) {
    bridge_state::drop_client(client);
}

/// Drop only this client's portable workspace; independently retained views survive.
/// Missing clients and handles are harmless. No cancellation or Pale operation occurs.
#[no_mangle]
pub extern "C" fn pale_go_portable_workspace_drop(client: u64, workspace: u64) {
    let _ = bridge_state::with_client_mut(client, |state| {
        state.workspaces.remove(&workspace);
        Ok(())
    });
}

/// Drop only this client's comparison; its original workspace handles survive.
/// Missing clients and handles are harmless. No cancellation or Pale operation occurs.
#[no_mangle]
pub extern "C" fn pale_go_workspace_comparison_drop(client: u64, comparison: u64) {
    let _ = bridge_state::with_client_mut(client, |state| {
        state.comparisons.remove(&comparison);
        Ok(())
    });
}

/// Drop only this client's profile; independent snapshots and review stages survive.
/// Missing clients and handles are harmless. No cancellation or Pale operation occurs.
#[no_mangle]
pub extern "C" fn pale_go_effective_profile_drop(client: u64, profile: u64) {
    let _ = bridge_state::with_client_mut(client, |state| {
        state.profiles.remove(&profile);
        Ok(())
    });
}

/// Drop only this client's authoring identity; other identities and profiles survive.
/// Missing clients and handles are harmless. No cancellation or Pale operation occurs.
#[no_mangle]
pub extern "C" fn pale_go_authoring_identity_drop(client: u64, identity: u64) {
    let _ = bridge_state::with_client_mut(client, |state| {
        state.authoring_identities.remove(&identity);
        Ok(())
    });
}

/// Release only this client's occurrence cursor; missing handles are harmless.
#[no_mangle]
pub extern "C" fn pale_go_authoring_occurrence_cursor_drop(client: u64, cursor: u64) {
    let _ = bridge_state::with_client_mut(client, |state| {
        state.authoring_occurrence_cursors.remove(&cursor);
        Ok(())
    });
}

/// Release only this client's profile association cursor; missing handles are harmless.
#[no_mangle]
pub extern "C" fn pale_go_effective_authoring_cursor_drop(client: u64, cursor: u64) {
    let _ = bridge_state::with_client_mut(client, |state| {
        state.effective_authoring_cursors.remove(&cursor);
        Ok(())
    });
}

/// Drop only this client's snapshot; independently retained selectors survive.
/// Missing clients and handles are harmless. No cancellation or Pale operation occurs.
#[no_mangle]
pub extern "C" fn pale_go_observation_snapshot_drop(client: u64, snapshot: u64) {
    let _ = bridge_state::with_client_mut(client, |state| {
        state.observation_snapshots.remove(&snapshot);
        Ok(())
    });
}

/// Drop only this client's page cursor; missing clients and handles are harmless.
#[no_mangle]
pub extern "C" fn pale_go_observation_page_cursor_drop(client: u64, cursor: u64) {
    let _ = bridge_state::with_client_mut(client, |state| {
        state.observation_page_cursors.remove(&cursor);
        Ok(())
    });
}

/// Drop only this client's lens cursor; missing clients and handles are harmless.
#[no_mangle]
pub extern "C" fn pale_go_observation_lens_cursor_drop(client: u64, cursor: u64) {
    let _ = bridge_state::with_client_mut(client, |state| {
        state.observation_lens_cursors.remove(&cursor);
        Ok(())
    });
}

/// Release only this client's profile-association continuation; no profile is closed.
#[no_mangle]
pub extern "C" fn pale_go_observation_profile_cursor_drop(client: u64, cursor: u64) {
    let _ = bridge_state::with_client_mut(client, |state| {
        state.observation_profile_cursors.remove(&cursor);
        Ok(())
    });
}

/// Drop only this client's observation subject; missing clients and handles are harmless.
#[no_mangle]
pub extern "C" fn pale_go_observation_subject_drop(client: u64, subject: u64) {
    let _ = bridge_state::with_client_mut(client, |state| {
        state.observation_subjects.remove(&subject);
        Ok(())
    });
}

/// Discard only an inspection explicitly surrendered by the Go caller.
/// This does not advance its semantic lifecycle; missing handles are harmless.
#[no_mangle]
pub extern "C" fn pale_go_improvement_inspection_drop(client: u64, inspection: u64) {
    let _ = bridge_state::with_client_mut(client, |state| {
        state.improvement_inspections.remove(&inspection);
        Ok(())
    });
}

/// Discard only a surrendered explanation, without consuming its preparations.
#[no_mangle]
pub extern "C" fn pale_go_improvement_explanation_drop(client: u64, explanation: u64) {
    let _ = bridge_state::with_client_mut(client, |state| {
        state.improvement_explanations.remove(&explanation);
        Ok(())
    });
}

/// Discard only a surrendered preparation, without consuming its explanation.
#[no_mangle]
pub extern "C" fn pale_go_improvement_prepared_drop(client: u64, preparation: u64) {
    let _ = bridge_state::with_client_mut(client, |state| {
        state.improvement_prepared.remove(&preparation);
        Ok(())
    });
}

/// Discard only a surrendered candidate; this does not bind or apply its effects.
/// Missing clients and handles are harmless.
#[no_mangle]
pub extern "C" fn pale_go_one_shot_candidate_drop(client: u64, candidate: u64) {
    let _ = bridge_state::with_client_mut(client, |state| {
        state.candidates.remove(&candidate);
        Ok(())
    });
}

/// Discard only a surrendered bound plan; this does not admit or apply it.
/// Missing clients and handles are harmless.
#[no_mangle]
pub extern "C" fn pale_go_one_shot_bound_plan_drop(client: u64, plan: u64) {
    let _ = bridge_state::with_client_mut(client, |state| {
        state.bound_plans.remove(&plan);
        Ok(())
    });
}

/// Return the response class, or bridge-failure for an unknown handle.
#[no_mangle]
pub extern "C" fn pale_go_response_kind(response: u64) -> u8 {
    bridge_state::with_response(response, |response| response.kind as u8)
        .unwrap_or(ResponseKind::BridgeFailure as u8)
}

/// Return an opaque result-object handle, or zero when none exists.
#[no_mangle]
pub extern "C" fn pale_go_response_object(response: u64) -> u64 {
    bridge_state::with_response(response, |response| response.object).unwrap_or(0)
}

/// Return the exact metadata byte length, or zero for an unknown response.
#[no_mangle]
pub extern "C" fn pale_go_response_metadata_length(response: u64) -> usize {
    bridge_state::with_response(response, |response| response.metadata.len()).unwrap_or(0)
}

/// Return the exact payload byte length, or zero for an unknown response.
#[no_mangle]
pub extern "C" fn pale_go_response_payload_length(response: u64) -> usize {
    bridge_state::with_response(response, |response| response.payload.len()).unwrap_or(0)
}

unsafe fn copy_part(
    response: u64,
    destination: *mut u8,
    capacity: usize,
    select: impl FnOnce(&Response) -> &[u8],
) -> usize {
    bridge_state::with_response(response, |response| {
        let bytes = select(response);
        if bytes.is_empty() || destination.is_null() || capacity < bytes.len() {
            return bytes.len();
        }
        // SAFETY: the function contract requires `destination` to be writable
        // for `capacity` bytes, and the branch proves the source fits.
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), destination, bytes.len()) };
        bytes.len()
    })
    .unwrap_or(0)
}

/// Copy response metadata into a Go-owned buffer and return its required size.
///
/// # Safety
///
/// A non-null `destination` must be writable for `capacity` bytes. No copy is
/// attempted when the capacity is smaller than the returned required size.
#[no_mangle]
pub unsafe extern "C" fn pale_go_response_copy_metadata(
    response: u64,
    destination: *mut u8,
    capacity: usize,
) -> usize {
    // SAFETY: this function forwards its caller contract unchanged.
    unsafe {
        copy_part(response, destination, capacity, |response| {
            &response.metadata
        })
    }
}

/// Copy the response payload into a Go-owned buffer and return its required size.
///
/// # Safety
///
/// A non-null `destination` must be writable for `capacity` bytes. No copy is
/// attempted when the capacity is smaller than the returned required size.
#[no_mangle]
pub unsafe extern "C" fn pale_go_response_copy_payload(
    response: u64,
    destination: *mut u8,
    capacity: usize,
) -> usize {
    // SAFETY: this function forwards its caller contract unchanged.
    unsafe {
        copy_part(response, destination, capacity, |response| {
            &response.payload
        })
    }
}

/// Discard a response and any native values not yet claimed by Go.
///
/// Unknown handles are inert. Discarding does not close the client or remove
/// any previously claimed object.
#[no_mangle]
pub extern "C" fn pale_go_response_drop(response: u64) {
    bridge_state::drop_response(response);
}

/// Consume a decoded response and transfer pending native values to its client.
///
/// Returns one on success and zero for a missing response, an error response,
/// or a closed owning client. This transfers only process-local object custody;
/// it grants no Pale authority, acceptance, cancellation, or native effect.
/// Go must finish decoding before calling this function. Success and failure
/// both consume the response; failed transfer drops its pending values. A later
/// drop is inert.
#[no_mangle]
pub extern "C" fn pale_go_response_claim(response: u64) -> u8 {
    u8::from(bridge_state::claim_response(response))
}

macro_rules! ffi_operation {
    ($name:ident, $implementation:path, $doc:literal) => {
        #[doc = $doc]
        #[no_mangle]
        pub extern "C" fn $name(client: u64, request: u64) -> u64 {
            operation(client, request, $implementation)
        }
    };
}

ffi_operation!(
    pale_go_capabilities,
    operations::capabilities,
    "Project the exact API-owned Go capability set."
);
ffi_operation!(
    pale_go_contract_bundle_identity,
    operations::contract_bundle_identity,
    "Inspect the embedded public-contract bundle identity."
);
ffi_operation!(
    pale_go_contract_schema,
    operations::contract_schema,
    "Look up one selected generated contract schema."
);
ffi_operation!(
    pale_go_validate_contract_document,
    operations::validate_contract_document,
    "Validate and canonicalize one contract document."
);
ffi_operation!(
    pale_go_project_diagnostic,
    operations::project_diagnostic,
    "Project one typed diagnostic candidate."
);
ffi_operation!(
    pale_go_admit_diagnostic,
    operations::admit_diagnostic,
    "Admit canonical diagnostic bytes."
);
ffi_operation!(
    pale_go_authoring_ingress_limits,
    operations::authoring_ingress_limits,
    "Inspect one authoring ingress ceiling."
);
ffi_operation!(
    pale_go_validate_authoring_document,
    operations::validate_authoring_document,
    "Validate one standalone authoring document."
);
ffi_operation!(
    pale_go_inspect_standalone_authoring,
    operations::inspect_standalone_authoring,
    "Inspect generated standalone authoring facts."
);
ffi_operation!(
    pale_go_compile_portable_workspace,
    operations::compile_portable_workspace,
    "Compile a path-free workspace source family."
);
ffi_operation!(
    pale_go_inspect_portable_workspace_authoring,
    operations::inspect_portable_workspace_authoring,
    "Inspect retained workspace authoring facts."
);
ffi_operation!(
    pale_go_navigate_portable_workspace_authoring,
    operations::navigate_portable_workspace_authoring,
    "Resolve one retained workspace source occurrence."
);
ffi_operation!(
    pale_go_compare_portable_workspaces,
    operations::compare_portable_workspaces,
    "Compare two admitted portable workspaces."
);
ffi_operation!(
    pale_go_portable_authoring_occurrences,
    authoring_pages::portable_occurrences,
    "Page source occurrences from the retained portable compilation."
);
ffi_operation!(
    pale_go_effective_authoring_occurrences,
    authoring_pages::effective_occurrences,
    "Page source occurrences from the retained effective compilation."
);
ffi_operation!(
    pale_go_effective_authoring_page,
    authoring_pages::effective_page,
    "Page retained profile associations for a compiler-resolved identity."
);
ffi_operation!(
    pale_go_portable_workspace_change_roots,
    operations::portable_workspace_change_roots,
    "Page one comparison's change roots."
);
ffi_operation!(
    pale_go_portable_workspace_impacts,
    operations::portable_workspace_impacts,
    "Page one comparison's impact partition."
);
ffi_operation!(
    pale_go_compare_portable_workspace_subject,
    operations::compare_portable_workspace_subject,
    "Compare one subject's selected values without creating another handle."
);
ffi_operation!(
    pale_go_explain_portable_workspace,
    operations::explain_portable_workspace,
    "Explain one comparison subject."
);
ffi_operation!(
    pale_go_compile_effective_profile,
    operations::compile_effective_profile,
    "Compile one selected effective profile."
);
ffi_operation!(
    pale_go_inspect_effective_profile,
    operations::inspect_effective_profile,
    "Page an effective-profile inspection."
);
ffi_operation!(
    pale_go_explain_effective_profile,
    operations::explain_effective_profile,
    "Explain one effective-profile finding."
);
ffi_operation!(
    pale_go_inspect_effective_profile_authoring,
    operations::inspect_effective_profile_authoring,
    "Inspect authoring facts retained by a profile."
);
ffi_operation!(
    pale_go_navigate_effective_profile_authoring,
    operations::navigate_effective_profile_authoring,
    "Resolve a profile-retained source occurrence."
);
ffi_operation!(
    pale_go_join_effective_profile_authoring,
    operations::join_effective_profile_authoring,
    "Join one resolved source identity to effective facts."
);
ffi_operation!(
    pale_go_observe_effective_profile,
    operations::observe_effective_profile,
    "Project one admitted effective profile into an opaque observation snapshot."
);
ffi_operation!(
    pale_go_observation_page,
    operations::observation_page,
    "Query one bounded page from an opaque observation snapshot."
);
ffi_operation!(
    pale_go_observation_inspect_profile,
    observation_profile::inspect,
    "Associate a subject with rows from its explicitly supplied retained compilations."
);
ffi_operation!(
    pale_go_observation_delta,
    operations::observation_delta,
    "Derive one complete adjacent observation delta or recovery disposition."
);
ffi_operation!(
    pale_go_observation_causal_path,
    operations::observation_causal_path,
    "Resolve one exact retained causal path from an opaque subject."
);
ffi_operation!(
    pale_go_observation_visual_lens,
    operations::observation_visual_lens,
    "Build one bounded non-authoritative observation lens."
);
ffi_operation!(
    pale_go_native_action_catalog,
    operations::native_action_catalog,
    "Inspect the effect-free native-action catalog."
);
ffi_operation!(
    pale_go_invoke_native_action,
    operations::invoke_native_action,
    "Invoke one tagged effect-free native action."
);
ffi_operation!(
    pale_go_invoke_native_action_payload,
    operations::invoke_native_action_payload,
    "Invoke one carrier-selected native-action payload."
);
ffi_operation!(
    pale_go_admit_repository_native_actions,
    operations::admit_repository_native_actions,
    "Admit one repository-bound native-action catalog."
);
ffi_operation!(
    pale_go_compile_one_shot,
    operations::compile_one_shot,
    "Compile one complete captured source family."
);
ffi_operation!(
    pale_go_one_shot_summary,
    operations::one_shot_summary,
    "Inspect an admitted one-shot compilation."
);
ffi_operation!(
    pale_go_one_shot_observation_targets,
    operations::one_shot_observation_targets,
    "Derive one effect-free observation target set."
);
ffi_operation!(
    pale_go_plan_one_shot,
    operations::plan_one_shot,
    "Construct one effect-free materialization plan candidate."
);
ffi_operation!(
    pale_go_bind_one_shot_machine_effects,
    operations::bind_one_shot_machine_effects,
    "Bind machine-local effects into one complete plan."
);
ffi_operation!(
    pale_go_admit_one_shot_saved_plan,
    operations::admit_one_shot_saved_plan,
    "Admit saved plan bytes against a rebuilt plan."
);
ffi_operation!(
    pale_go_inspect_package_distribution,
    operations::inspect_package_distribution,
    "Inspect one bounded Package source distribution."
);
ffi_operation!(
    pale_go_admit_resident_hook,
    operations::admit_resident_hook,
    "Admit one repository-native resident hook event."
);
ffi_operation!(
    pale_go_preview_local_configuration_improvement,
    operations::preview_local_configuration_improvement,
    "Capture local configurations and retain an effect-free review preview."
);
ffi_operation!(
    pale_go_compile_local_one_shot,
    operations::compile_local_one_shot,
    "Resolve, capture, and compile one local workspace."
);
ffi_operation!(
    pale_go_inspect_local_package_distribution,
    operations::inspect_local_package_distribution,
    "Capture and inspect one local Package distribution."
);
ffi_operation!(
    pale_go_capture_local_native_action_request,
    operations::capture_local_native_action_request,
    "Capture a confined local file as a native-action request."
);
ffi_operation!(
    pale_go_invoke_captured_native_action,
    operations::invoke_captured_native_action,
    "Consume and invoke one captured native-action request."
);
ffi_operation!(
    pale_go_inspect_local_repository_status,
    operations::inspect_local_repository_status,
    "Inspect one admitted local repository projection."
);
ffi_operation!(
    pale_go_plan_local_one_shot,
    operations::plan_local_one_shot,
    "Construct one complete local materialization plan."
);
ffi_operation!(
    pale_go_publish_local_one_shot_plan,
    operations::publish_local_one_shot_plan,
    "Publish one complete local plan into a confined directory."
);
ffi_operation!(
    pale_go_admit_local_materialization_run,
    operations::admit_local_materialization_run,
    "Admit one complete saved-plan materialization run."
);
ffi_operation!(
    pale_go_settle_local_materialization_run,
    operations::settle_local_materialization_run,
    "Settle one admitted local materialization run."
);
ffi_operation!(
    pale_go_inspect_local_materialization_reconciliation,
    operations::inspect_local_materialization_reconciliation,
    "Inspect an outcome-unknown local settlement."
);
ffi_operation!(
    pale_go_compiled_interface_identity,
    operations::compiled_interface_identity,
    "Report compiled SDK and interface claims without reading an executable."
);
ffi_operation!(
    pale_go_verify_current_executable_binding,
    operations::verify_current_executable_binding,
    "Verify the current host executable against a captured workspace binding."
);
ffi_operation!(
    pale_go_inspect_local_control_store,
    operations::inspect_local_control_store,
    "Inspect a canonical control store without creating, repairing or admitting it."
);
ffi_operation!(
    pale_go_improvement_inspection_state,
    improvement::inspection_state,
    "Inspect the retained configuration improvement inspection state."
);
ffi_operation!(
    pale_go_improvement_explanation_state,
    improvement::explanation_state,
    "Inspect the retained configuration improvement explanation state."
);
ffi_operation!(
    pale_go_improvement_evaluation_state,
    improvement::evaluation_state,
    "Inspect the retained configuration improvement evaluation state."
);
ffi_operation!(
    pale_go_improvement_diagnosis_state,
    improvement::diagnosis_state,
    "Inspect the retained configuration improvement diagnosis state."
);
ffi_operation!(
    pale_go_improvement_proposal_phase,
    improvement::proposal_phase,
    "Inspect the retained configuration improvement proposal phase."
);
ffi_operation!(
    pale_go_improvement_preview_phase,
    improvement::preview_phase,
    "Inspect the retained configuration improvement preview phase."
);
ffi_operation!(
    pale_go_improvement_inspect,
    improvement::inspect,
    "Project the effect-free configuration improvement inspect operation."
);
ffi_operation!(
    pale_go_improvement_explain,
    improvement::explain,
    "Project the effect-free configuration improvement explain operation."
);
ffi_operation!(
    pale_go_improvement_propose,
    improvement::propose,
    "Project the effect-free configuration improvement propose operation."
);
ffi_operation!(
    pale_go_improvement_evaluate,
    improvement::evaluate,
    "Project the effect-free configuration improvement evaluate operation."
);
ffi_operation!(
    pale_go_improvement_diagnose,
    improvement::diagnose,
    "Project the effect-free configuration improvement diagnose operation."
);
ffi_operation!(
    pale_go_improvement_preview,
    improvement::preview,
    "Project the effect-free configuration improvement preview operation."
);
ffi_operation!(
    pale_go_improvement_prepare,
    improvement::prepare,
    "Project the effect-free configuration improvement prepare operation."
);
ffi_operation!(
    pale_go_improvement_commit,
    improvement::commit,
    "Project the effect-free configuration improvement commit operation."
);
ffi_operation!(
    pale_go_improvement_prepared_summary,
    improvement::prepared_summary,
    "Project the effect-free configuration improvement prepared summary operation."
);
ffi_operation!(
    pale_go_improvement_summary,
    improvement::summary,
    "Project the effect-free configuration improvement summary operation."
);
ffi_operation!(
    pale_go_improvement_comparison_summary,
    improvement::comparison_summary,
    "Project the effect-free configuration improvement comparison summary operation."
);
ffi_operation!(
    pale_go_improvement_change_roots,
    improvement::change_roots,
    "Project the effect-free configuration improvement change roots operation."
);
ffi_operation!(
    pale_go_improvement_impacts,
    improvement::impacts,
    "Project the effect-free configuration improvement impacts operation."
);
ffi_operation!(
    pale_go_improvement_explain_comparison,
    improvement::explain_comparison,
    "Project the effect-free configuration improvement explain comparison operation."
);
ffi_operation!(
    pale_go_improvement_profile_page,
    improvement::profile_page,
    "Project the effect-free configuration improvement profile page operation."
);
ffi_operation!(
    pale_go_improvement_explain_profile,
    improvement::explain_profile,
    "Project the effect-free configuration improvement explain profile operation."
);
ffi_operation!(
    pale_go_improvement_accept,
    improvement::accept,
    "Project the effect-free configuration improvement accept operation."
);
ffi_operation!(
    pale_go_improvement_reject,
    improvement::reject,
    "Project the effect-free configuration improvement reject operation."
);
ffi_operation!(
    pale_go_improvement_cancel,
    improvement::cancel,
    "Project the effect-free configuration improvement cancel operation."
);
ffi_operation!(
    pale_go_improvement_mark_stale,
    improvement::mark_stale,
    "Project the effect-free configuration improvement mark stale operation."
);
