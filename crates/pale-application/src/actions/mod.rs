//! Built-in actions that return the same result for the same input, such as
//! normalizing a path list or ordering services.
//!
//! The registry builds on `diagnostic` and `service_topology`; `compile`, `render`
//! and `resident` build on it. It accepts only closed typed inputs and dispatches
//! to compiled handlers with no shell, filesystem, network, model, clock, or
//! random access.

mod compliance;
mod config_invariant;
mod cost;
mod diagnostic;
mod error;
mod evidence_selection;
mod invocation;
mod loop_status;
mod math;
mod path_inventory;
mod ranking;
mod registry;
mod review_convergence;
mod rubric;
mod service_definition;
mod skill_health;
mod validation;

pub(crate) use diagnostic::matches_native_failure_diagnostic;
pub use diagnostic::{project_action_failure, project_invocation_failure, ActionFailure};
pub use invocation::NativeActionInvocationError;
pub use registry::{NativeActionRegistry, PreparedNativeAction};

// Successful and failed actions share this closed v1 record family; digest
// preimages use the same identities as schema admission.
/// Wire major shared by every record in one native-action exchange.
const ACTION_RECORD_SCHEMA_VERSION: u32 = 1;
// Standalone wire identities for the five records in one action exchange.
/// Contract record identity sealed before request admission.
const CONTRACT_RECORD: &str = "pale/action-contract.v1";
/// Request record identity included in its canonical digest preimage.
const REQUEST_RECORD: &str = "pale/action-request.v1";
/// Successful result record identity included in its digest preimage.
const RESULT_RECORD: &str = "pale/action-result.v1";
/// Receipt record identity binding contract, request, and result.
const RECEIPT_RECORD: &str = "pale/action-receipt.v1";
/// Structured failure record identity; errors never masquerade as results.
const ERROR_RECORD: &str = "pale/action-error.v1";

#[cfg(test)]
#[path = "../../tests/internal/actions.rs"]
mod tests;

/// Largest integer represented exactly by RFC 8785's ECMAScript number model.
const MAX_CANONICAL_INTEGER: u64 = (1_u64 << 53) - 1;

/// Shared integer scale for ratios and fixed-point scores.
const MICRO_SCALE: u32 = 1_000_000;
