//! Host-specific mechanical projection shared by operation prompts and admission.

use crate::terminal_adapters::TerminalAdapterDescriptor;

use std::collections::BTreeSet;

use pale_core::records::{
    native_action_host_tool_name, CapabilityProvider, EffectClass, HostToolBinding,
    HostToolEffectBinding, HostedProviderExecutionV1, NativeRustProviderExecutionV1,
    OperationCapabilityContractV1, OperationCapabilityProviderStateV1,
    OperationContractMechanicsV1, OperationSupport, ProviderBindingRefV1,
    ProviderExecutionAvailability, ResolvedCapabilityRequirementPolicyV1,
    RouteCapabilityProviderV1, ToolObservability,
};

use super::operation_contract::operation_requires_manual_invocation;
use super::types::{HarnessSurfaceRefs, HostOperationProjection};
use crate::semantic::{
    ProviderUnavailability, ResolvedCapability, ResolvedOperation, ResolvedProviderSelection,
};

/// Join one configured capability with exact host and machine observations.
///
/// The returned boolean means the provider is usable for this route; it never
/// upgrades unknown execution state from configuration alone.
fn project_capability(
    capability: &ResolvedCapability,
    host: TerminalAdapterDescriptor,
    provider_binding: &ProviderBindingRefV1,
    native_tools_ready: bool,
    machine_observed: bool,
    require_observable: bool,
) -> (OperationCapabilityContractV1, bool) {
    let binding = capability.binding();
    let declared_native_action = binding
        .filter(|binding| binding.provider() == CapabilityProvider::NativeRust)
        .and_then(pale_core::records::CapabilityBinding::target)
        .filter(|target| native_action_host_tool_name(target).is_some())
        .map(str::to_owned);
    let binding_supports_host = binding.is_some_and(|binding| match binding.provider() {
        CapabilityProvider::HostNative | CapabilityProvider::Connector => {
            binding.provider_tools().is_some_and(|rows| {
                rows.iter()
                    .any(|row| row.provider_binding == *provider_binding && !row.tools.is_empty())
            })
        }
        CapabilityProvider::NativeRust => declared_native_action.is_some(),
    });
    // The verified Pale runtime is the carrier for a compiled native action.
    // Host-native providers instead require their exact per-harness observation.
    let pale_runtime_supplies_native_action = native_tools_ready
        && capability
            .binding()
            .is_some_and(|binding| binding.provider() == CapabilityProvider::NativeRust);
    let observed_availability = match machine_observed || pale_runtime_supplies_native_action {
        true => ProviderExecutionAvailability::Available,
        false => ProviderExecutionAvailability::Unknown,
    };
    let provider_available = native_tools_ready
        && binding_supports_host
        && observed_availability == ProviderExecutionAvailability::Available;
    let mut host_tools = if provider_available {
        capability
            .binding()
            .and_then(pale_core::records::CapabilityBinding::provider_tools)
            .and_then(|rows| {
                rows.iter()
                    .find(|row| row.provider_binding == *provider_binding)
            })
            .map(|row| row.tools.clone())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    if provider_available
        && capability
            .binding()
            .is_some_and(|binding| binding.provider() == CapabilityProvider::NativeRust)
        && native_tools_ready
    {
        if let Some(name) = declared_native_action
            .as_deref()
            .and_then(native_action_host_tool_name)
        {
            host_tools.push(HostToolBinding {
                name,
                model_name: None,
                observability: ToolObservability::PreToolObservable,
                commands: Vec::new(),
                command_source: None,
                input_equals: Default::default(),
            });
        }
    }
    let base_available = provider_available
        && (capability
            .binding()
            .is_some_and(|binding| binding.provider() != CapabilityProvider::NativeRust)
            || (native_tools_ready && !host_tools.is_empty()));
    let observable = host_tools
        .iter()
        .any(|tool| tool.observability == ToolObservability::PreToolObservable);
    let usable = base_available && (!require_observable || observable);
    let note = if base_available && !usable {
        Some(format!(
            "provider is not visible to the {host} pre-tool boundary and cannot satisfy an enforced operation"
        ))
    } else {
        capability
            .binding()
            .and_then(|binding| binding.note.clone())
    };
    let execution = if usable {
        HostedProviderExecutionV1::Available {
            host_tools: host_tools.clone(),
        }
    } else {
        HostedProviderExecutionV1::Unknown
    };
    let provider_state = match binding {
        None => OperationCapabilityProviderStateV1::Unbound,
        Some(binding) => {
            let provider = match binding.provider() {
                CapabilityProvider::HostNative => {
                    RouteCapabilityProviderV1::HostNative { execution }
                }
                CapabilityProvider::Connector => RouteCapabilityProviderV1::Connector {
                    target: binding
                        .target()
                        .expect("validated connector binding has a target")
                        .to_string(),
                    execution,
                },
                CapabilityProvider::NativeRust => RouteCapabilityProviderV1::NativeRust {
                    target: binding
                        .target()
                        .expect("validated native binding has a target")
                        .to_string(),
                    execution: if usable {
                        NativeRustProviderExecutionV1::Available {
                            host_tools,
                            native_action: declared_native_action
                                .expect("usable native binding has a compiled action"),
                        }
                    } else {
                        NativeRustProviderExecutionV1::Unknown
                    },
                },
            };
            OperationCapabilityProviderStateV1::Bound {
                approval: binding.approval,
                provider,
                note,
            }
        }
    };
    (
        OperationCapabilityContractV1 {
            requirement_id: capability.requirement_id.clone(),
            capability_id: capability.capability_id.clone(),
            recipe: capability.recipe.clone(),
            effect_class: capability.effect_class,
            policy: capability.policy.clone(),
            failure_posture: capability.failure_posture,
            provider_state,
        },
        usable,
    )
}

/// Explain one route-local capability failure without collapsing selection and execution state.
fn unavailable_reason(
    semantic: &ResolvedCapability,
    capability: &OperationCapabilityContractV1,
    host: TerminalAdapterDescriptor,
) -> String {
    let selection = match semantic.provider {
        ResolvedProviderSelection::Bound(_) => "provider selected".to_string(),
        ResolvedProviderSelection::Unbound(reason) => format!(
            "unbound: {}",
            match reason {
                ProviderUnavailability::MissingBinding => "no private provider binding",
                ProviderUnavailability::ExcludedByNarrowing => {
                    "excluded by private capability narrowing"
                }
                ProviderUnavailability::EffectMismatch => {
                    "provider effect differs from Package procedure"
                }
                ProviderUnavailability::RecipeContractMismatch => {
                    "provider does not implement the selected Package recipe"
                }
            }
        ),
    };
    format!(
        "capability `{}` is not usable for `{host}` ({selection}, execution observation {})",
        capability.capability_id,
        capability.provider_state.execution_availability(),
    )
}

/// Resolve the exact observable host tools admitted to the read-only boundary.
/// This deliberately shares provider availability with operation contracts so
/// prompts, leases, and the sealed hook context cannot disagree.
pub(super) fn project_host_tool_effects(
    artifacts: &HarnessSurfaceRefs,
) -> Vec<HostToolEffectBinding> {
    let mut effects = artifacts
        .operations
        .values()
        .filter(|operation| operation.support.support != OperationSupport::Blocked)
        .flat_map(|operation| &operation.support.capabilities)
        .filter(|capability| capability.available())
        .flat_map(|capability| {
            capability
                .provider_state
                .host_tools()
                .iter()
                .filter(|tool| tool.observability == ToolObservability::PreToolObservable)
                .cloned()
                .map(move |tool| HostToolEffectBinding {
                    effect_class: capability.effect_class,
                    tool,
                })
        })
        .collect::<Vec<_>>();
    effects.sort();
    effects.dedup();
    effects
}

/// Project requirement support for one operation and native harness.
///
/// This function folds only the requirements exposed by
/// [`ResolvedOperation::capabilities`]. No requirements yields
/// [`OperationSupport::Full`]; that result does not establish that an operation
/// is bound or exposed. [`crate::semantic::ResolvedOperationMechanics`] remains
/// the owner of the unbound, semantic-only, and capability-bearing distinction.
pub(super) fn project_operation_support(
    operation: &ResolvedOperation,
    host: TerminalAdapterDescriptor,
    provider_binding: &ProviderBindingRefV1,
    native_tools_ready: bool,
    capability_observations: Option<&BTreeSet<pale_core::records::CapabilityIdV1>>,
) -> HostOperationProjection {
    let manual = operation_requires_manual_invocation(operation);
    let projected = operation
        .capabilities()
        .iter()
        .map(|capability| {
            let effect_requires_observation = !matches!(
                capability.effect_class,
                EffectClass::Compute | EffectClass::Read
            ) || capability
                .binding()
                .is_some_and(|binding| binding.approval != pale_core::records::Approval::None);
            project_capability(
                capability,
                host,
                provider_binding,
                native_tools_ready,
                capability_observations
                    .is_some_and(|observed| observed.contains(&capability.capability_id)),
                manual && effect_requires_observation,
            )
        })
        .collect::<Vec<_>>();
    let (capabilities, usable): (Vec<_>, Vec<_>) = projected.into_iter().unzip();
    let state = if capabilities.is_empty() {
        OperationContractMechanicsV1::SemanticOnly
    } else {
        OperationContractMechanicsV1::CapabilityBearing {
            capabilities: capabilities.clone(),
        }
    };
    let reasons = |policy: fn(&ResolvedCapabilityRequirementPolicyV1) -> bool| {
        operation
            .capabilities()
            .iter()
            .zip(&capabilities)
            .zip(&usable)
            .filter(|(_, usable)| !**usable)
            .map(|((semantic, capability), _)| (semantic, capability))
            .filter(|(_, capability)| policy(&capability.policy))
            .map(|(semantic, capability)| unavailable_reason(semantic, capability, host))
            .collect()
    };
    let optional_reasons = operation
        .capabilities()
        .iter()
        .zip(&capabilities)
        .zip(&usable)
        .filter(|((_, capability), usable)| !capability.required() && !**usable)
        .map(|((semantic, capability), _)| (semantic, capability))
        .map(|(semantic, capability)| unavailable_reason(semantic, capability, host))
        .collect();
    let machine_bound = capabilities
        .iter()
        .zip(&usable)
        .any(|(capability, usable)| {
            *usable
                && capability.provider_state.provider() == Some(CapabilityProvider::NativeRust)
                && !capability.provider_state.host_tools().is_empty()
        });
    let blocked_reasons =
        reasons(|policy| matches!(policy, ResolvedCapabilityRequirementPolicyV1::RequiredBlock));
    let degraded_reasons = reasons(|policy| {
        matches!(
            policy,
            ResolvedCapabilityRequirementPolicyV1::RequiredDegrade { .. }
        )
    });
    HostOperationProjection {
        capabilities,
        support: state.support(),
        blocked_reasons,
        degraded_reasons,
        optional_reasons,
        machine_bound,
    }
}

#[cfg(test)]
#[path = "../../tests/internal/render/operation_support.rs"]
mod tests;
