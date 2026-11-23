//! Contract matching and post-failure policy enforcement.

use std::collections::BTreeSet;

use pale_core::records::{
    CapabilityRequirementIdV1, EffectClass, FailurePosture, OperationCapabilityContractV1,
    OperationContractV1,
};

use crate::session_state::ActiveOperationLease;

use super::super::{OperationAdmissionError, ToolInvocation};

/// Unique contract role assigned to one observed host tool call.
pub(super) enum MatchedTool<'a> {
    Auxiliary,
    Capability(&'a OperationCapabilityContractV1),
}

impl MatchedTool<'_> {
    /// Return the capability requirement, or `None` for an auxiliary tool.
    pub(super) fn requirement_id(&self) -> Option<&CapabilityRequirementIdV1> {
        match self {
            Self::Auxiliary => None,
            Self::Capability(capability) => Some(&capability.requirement_id),
        }
    }
}

fn matching_capability<'a>(
    contract: &'a OperationContractV1,
    invocation: &ToolInvocation<'_>,
) -> Result<Option<&'a OperationCapabilityContractV1>, OperationAdmissionError> {
    let mut matching = contract
        .capabilities()
        .iter()
        .filter(|capability| capability.available())
        .filter(|capability| {
            capability.provider_state.host_tools().iter().any(|tool| {
                crate::host_tool::matches_pre_tool(
                    tool,
                    invocation.tool_name,
                    invocation.tool_input,
                    invocation.command,
                )
            })
        });
    let Some(first) = matching.next() else {
        return Ok(None);
    };
    let Some(second) = matching.next() else {
        return Ok(Some(first));
    };
    let mut requirements = BTreeSet::from([
        first.requirement_id.as_str(),
        second.requirement_id.as_str(),
    ]);
    requirements.extend(matching.map(|capability| capability.requirement_id.as_str()));
    Err(OperationAdmissionError::AmbiguousToolContract {
        operation: contract.id.clone(),
        tool: invocation.tool_name.to_string(),
        requirements: requirements.into_iter().collect::<Vec<_>>().join(","),
    })
}

/// Match exactly one auxiliary or available capability contract.
pub(super) fn match_tool<'a>(
    contract: &'a OperationContractV1,
    invocation: &ToolInvocation<'_>,
) -> Result<MatchedTool<'a>, OperationAdmissionError> {
    let capability = matching_capability(contract, invocation)?;
    let auxiliary = contract.auxiliary_tools.iter().any(|tool| {
        crate::host_tool::matches_pre_tool(
            tool,
            invocation.tool_name,
            invocation.tool_input,
            invocation.command,
        )
    });
    match (auxiliary, capability) {
        (true, Some(capability)) => Err(OperationAdmissionError::AmbiguousToolContract {
            operation: contract.id.clone(),
            tool: invocation.tool_name.to_string(),
            requirements: format!("auxiliary,{}", capability.requirement_id),
        }),
        (true, None) => Ok(MatchedTool::Auxiliary),
        (false, Some(capability)) => Ok(MatchedTool::Capability(capability)),
        (false, None) => Err(OperationAdmissionError::ToolOutsideContract {
            operation: contract.id.clone(),
            tool: invocation.tool_name.to_string(),
        }),
    }
}

/// Apply the prior failure posture before admitting another tool call.
pub(super) fn enforce_prior_failure(
    contract: &OperationContractV1,
    lease: &ActiveOperationLease,
    capability: Option<&OperationCapabilityContractV1>,
) -> Result<(), OperationAdmissionError> {
    let Some(failure) = lease.failure() else {
        return Ok(());
    };
    match failure.posture() {
        FailurePosture::FailClosed => Err(OperationAdmissionError::OperationHalted {
            operation: contract.id.clone(),
            requirement: failure.requirement_id().clone(),
        }),
        FailurePosture::FailOpen => Ok(()),
        FailurePosture::Retry => {
            if capability
                .is_some_and(|capability| &capability.requirement_id == failure.requirement_id())
            {
                Ok(())
            } else {
                Err(OperationAdmissionError::OperationRetrying {
                    operation: contract.id.clone(),
                    requirement: failure.requirement_id().clone(),
                    attempted: capability.map_or_else(
                        || "auxiliary tool".to_string(),
                        |capability| capability.requirement_id.to_string(),
                    ),
                })
            }
        }
        FailurePosture::Steer => match capability {
            Some(capability)
                if matches!(
                    capability.effect_class,
                    EffectClass::Write | EffectClass::Network | EffectClass::Publish
                ) =>
            {
                Err(OperationAdmissionError::OperationSteering {
                    operation: contract.id.clone(),
                    requirement: failure.requirement_id().clone(),
                    effect: capability.effect_class,
                })
            }
            _ => Ok(()),
        },
        FailurePosture::Escalate => Err(OperationAdmissionError::OperationEscalated {
            operation: contract.id.clone(),
            requirement: failure.requirement_id().clone(),
        }),
    }
}

/// Decide whether retained failure state permits operation completion.
pub(super) fn enforce_terminal_failure(
    contract: &OperationContractV1,
    lease: &ActiveOperationLease,
) -> Result<(), OperationAdmissionError> {
    let Some(failure) = lease.failure() else {
        return Ok(());
    };
    match failure.posture() {
        FailurePosture::FailClosed => Err(OperationAdmissionError::OperationHalted {
            operation: contract.id.clone(),
            requirement: failure.requirement_id().clone(),
        }),
        FailurePosture::Escalate => Err(OperationAdmissionError::OperationEscalated {
            operation: contract.id.clone(),
            requirement: failure.requirement_id().clone(),
        }),
        FailurePosture::FailOpen | FailurePosture::Retry | FailurePosture::Steer => Ok(()),
    }
}
