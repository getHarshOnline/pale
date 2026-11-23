//! Verifies that rendered operation support preserves unbound, semantic-only, and capability-bearing states.

use pale_core::records::{
    ArtifactDigest, CapabilityIdV1, CapabilityRequirementIdV1, DegradedOutcomeIdV1, FailurePosture,
    OperationSelection, OperationSupport, PackageIdV1, PackageResourceIdV1, PackageResourceRefV1,
    ProviderBindingIdentityV1, ProviderBindingRefV1, ResolvedCapabilityFallbackV1,
    ResolvedCapabilityRequirementPolicyV1,
};

use super::project_operation_support;
use crate::render::operation_is_bound;
use crate::semantic::{
    ProviderUnavailability, ResolvedCapability, ResolvedOperation, ResolvedOperationMechanics,
    ResolvedOperationRecord, ResolvedProviderSelection,
};
use crate::terminal_adapters::TerminalAdapterDescriptor;

fn resource(id: &str) -> PackageResourceRefV1 {
    PackageResourceRefV1 {
        package_id: PackageIdV1::parse("test-package").expect("valid test Package id"),
        resource_id: PackageResourceIdV1::parse(id).expect("valid test resource id"),
    }
}

fn operation(mechanics: ResolvedOperationMechanics) -> ResolvedOperation {
    ResolvedOperation {
        record: ResolvedOperationRecord {
            id: resource("operation"),
            description: "Synthetic operation.".into(),
            roles: Vec::new(),
            outputs: Vec::new(),
        },
        body: "Procedure.".into(),
        required_doctrine: String::new().into(),
        body_sources: Vec::new(),
        doctrine_ids: Vec::new(),
        mechanics,
    }
}

fn unavailable(policy: ResolvedCapabilityRequirementPolicyV1) -> ResolvedCapability {
    ResolvedCapability {
        requirement_id: CapabilityRequirementIdV1::parse("requirement")
            .expect("valid test requirement id"),
        description: "Synthetic requirement.".into(),
        capability_id: CapabilityIdV1::parse("inspect").expect("valid test capability id"),
        recipe: None,
        provider: ResolvedProviderSelection::Unbound(ProviderUnavailability::ExcludedByNarrowing),
        effect_class: pale_core::records::EffectClass::Read,
        policy,
        failure_posture: FailurePosture::FailClosed,
    }
}

fn fallback() -> ResolvedCapabilityFallbackV1 {
    ResolvedCapabilityFallbackV1 {
        guidance: "Proceed using admitted semantic evidence only.".into(),
        recipe: None,
    }
}

fn provider() -> ProviderBindingRefV1 {
    ProviderBindingRefV1 {
        provider_binding_id: "terminal-provider".into(),
        provider_binding_identity: ProviderBindingIdentityV1 {
            provider_binding_sha256: ArtifactDigest::hash_bytes(b"terminal-provider"),
        },
    }
}

#[test]
fn operation_binding_presence_owns_the_three_exact_states() {
    let unbound = operation(ResolvedOperationMechanics::Unbound);
    let semantic_only = operation(ResolvedOperationMechanics::SemanticOnly {
        selection: OperationSelection::Internal,
    });
    let capability_bearing = operation(ResolvedOperationMechanics::CapabilityBearing {
        selection: OperationSelection::Internal,
        capabilities: vec![unavailable(
            ResolvedCapabilityRequirementPolicyV1::RequiredBlock,
        )],
    });

    assert!(!operation_is_bound(&unbound));
    assert!(operation_is_bound(&semantic_only));
    assert!(semantic_only.capabilities().is_empty());
    assert!(operation_is_bound(&capability_bearing));
    assert_eq!(capability_bearing.capabilities().len(), 1);
}

#[test]
fn unavailable_policy_affects_only_the_owning_operation() {
    let projection = |policy| {
        let operation = operation(ResolvedOperationMechanics::CapabilityBearing {
            selection: OperationSelection::Internal,
            capabilities: vec![unavailable(policy)],
        });
        project_operation_support(
            &operation,
            TerminalAdapterDescriptor::codex(),
            &provider(),
            false,
            None,
        )
    };

    let blocked = projection(ResolvedCapabilityRequirementPolicyV1::RequiredBlock);
    assert_eq!(blocked.support, OperationSupport::Blocked);
    assert_eq!(blocked.blocked_reasons.len(), 1);
    assert!(blocked.blocked_reasons[0].contains("excluded by private capability narrowing"));

    let degraded = projection(ResolvedCapabilityRequirementPolicyV1::RequiredDegrade {
        fallback: fallback(),
        degraded_outcome: DegradedOutcomeIdV1::parse("semantic-evidence-only")
            .expect("valid degraded outcome id"),
    });
    assert_eq!(degraded.support, OperationSupport::Degraded);
    assert_eq!(degraded.degraded_reasons.len(), 1);
    assert!(degraded.blocked_reasons.is_empty());

    let optional = projection(ResolvedCapabilityRequirementPolicyV1::Optional {
        fallback: fallback(),
    });
    assert_eq!(optional.support, OperationSupport::Full);
    assert_eq!(optional.optional_reasons.len(), 1);
    assert!(optional.blocked_reasons.is_empty());
    assert!(optional.degraded_reasons.is_empty());
}

#[test]
fn model_name_keeps_native_identity_and_unknown_machine_state_separate() {
    use pale_core::records::{
        Approval, CapabilityBinding, CapabilityProviderBinding, HostToolBinding,
        ProviderToolBindingV1, ToolObservability,
    };
    let provider = provider();
    let mut capability = unavailable(ResolvedCapabilityRequirementPolicyV1::RequiredBlock);
    capability.provider = ResolvedProviderSelection::Bound(CapabilityBinding {
        effect_class: pale_core::records::EffectClass::Read,
        approval: Approval::None,
        tool_contract: None,
        implementation: CapabilityProviderBinding::HostNative {
            provider_tools: vec![ProviderToolBindingV1 {
                provider_binding: provider.clone(),
                tools: vec![HostToolBinding {
                    name: "Bash".into(),
                    model_name: Some("exec_command".into()),
                    observability: ToolObservability::PreToolObservable,
                    commands: Vec::new(),
                    command_source: None,
                    input_equals: Default::default(),
                }],
            }],
        },
        note: None,
    });
    let (available, usable) = super::project_capability(
        &capability,
        TerminalAdapterDescriptor::codex(),
        &provider,
        true,
        true,
        true,
    );
    assert!(usable);
    let tool = &available.provider_state.host_tools()[0];
    assert_eq!(tool.name, "Bash");
    assert_eq!(tool.model_tool_name(), "exec_command");
    let (unknown, usable) = super::project_capability(
        &capability,
        TerminalAdapterDescriptor::codex(),
        &provider,
        true,
        false,
        true,
    );
    assert!(!usable);
    assert!(unknown.provider_state.host_tools().is_empty());
}
