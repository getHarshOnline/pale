//! Validation and canonical ordering of provider-neutral service definitions.
//!
//! Dependencies must point to lower-precedence services, ports are globally
//! unique, and output order is precedence then service name.

use pale_core::records::{
    ActionError, ActionErrorCode, ServiceDefinitionInput, ServiceDefinitionOutput,
    SERVICE_DEFINITION_NORMALIZE_ACTION_ID,
};

use super::error::action_error;
use crate::service_topology::normalize_service_definitions;

/// Validate one service definition as data; this action does not instantiate it.
pub(super) fn execute(
    input: &ServiceDefinitionInput,
) -> Result<ServiceDefinitionOutput, ActionError> {
    normalize_service_definitions(input).map_err(|error| {
        let code = if error.is_duplicate() {
            ActionErrorCode::DuplicateIdentifier
        } else {
            ActionErrorCode::MalformedInput
        };
        action_error(
            Some(SERVICE_DEFINITION_NORMALIZE_ACTION_ID),
            code,
            error.to_string(),
        )
    })
}

#[cfg(test)]
#[path = "../../tests/internal/actions/service_definition.rs"]
mod tests;
