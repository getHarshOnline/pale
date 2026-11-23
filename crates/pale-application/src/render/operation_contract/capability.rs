//! Route-local exposure and invocation policy for operation contracts.
//!
//! These predicates separate model discovery from manual availability. They do
//! not upgrade a blocked requirement or turn private provider selection into a
//! runtime observation.

use pale_core::records::OperationSelection;

use crate::semantic::ResolvedOperation;

/// Return whether any selected capability forbids model-origin invocation.
pub(in crate::render) fn operation_requires_manual_invocation(
    operation: &ResolvedOperation,
) -> bool {
    operation
        .selection()
        .is_some_and(|selection| matches!(selection, OperationSelection::UserOnly { .. }))
}

/// Return whether the operation may appear on a model-facing discovery surface.
pub(in crate::render) fn operation_is_model_facing(operation: &ResolvedOperation) -> bool {
    operation
        .selection()
        .is_some_and(OperationSelection::user_selectable)
}

/// Return whether private configuration bound the operation for any selector.
pub(in crate::render) fn operation_is_bound(operation: &ResolvedOperation) -> bool {
    operation.mechanics.is_bound()
}
