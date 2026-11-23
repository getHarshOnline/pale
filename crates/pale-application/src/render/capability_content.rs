//! Resolved capability grants and the sealed machine-readable binding record.

use std::collections::{btree_map::Entry, BTreeMap};

use pale_core::records::{
    CapabilityBindingsV1, CapabilityIdV1, CapabilityUseKind, PackageResourceRefV1,
    ProfileRepository, ProviderToolBindingV1, ResolvedCapabilityProviderV1,
    ResolvedCapabilityUseV1, ResolvedToolRecipeV1,
};

use super::semantic_helpers::{json_pretty, short};
use super::types::RenderError;
use super::RenderIdentity;
use crate::semantic::RepositorySemanticPlan;

/// Summarize configured host tools without implying that any tool is reachable.
fn host_tool_summary(row: &ProviderToolBindingV1) -> String {
    let tools = &row.tools;
    let names = tools
        .iter()
        .map(|tool| {
            if tool.commands.is_empty() {
                tool.model_tool_name().to_owned()
            } else {
                format!(
                    "{} ({} command forms)",
                    tool.model_tool_name(),
                    tool.commands.len()
                )
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("{}: {names}", row.provider_binding.provider_binding_id)
}

/// Render the private provider projection without treating it as Package meaning.
pub(super) fn capabilities_content(
    repo: &ProfileRepository,
    semantics: &RepositorySemanticPlan,
    identity: &RenderIdentity<'_>,
) -> Result<String, RenderError> {
    let dg = short(identity.profile().as_str());
    let by_capability = capability_providers(semantics)?;
    let mut caps = String::new();
    for (capability_id, capability) in by_capability {
        let provider = match capability.implementation.provider() {
            pale_core::records::CapabilityProvider::HostNative => {
                let tools = capability
                    .implementation
                    .provider_tools()
                    .into_iter()
                    .flatten()
                    .map(host_tool_summary)
                    .collect::<Vec<_>>()
                    .join("; ");
                format!("host-native [{tools}]")
            }
            pale_core::records::CapabilityProvider::NativeRust => format!(
                "native-rust `{}`",
                required_target(&capability, "native Rust")?
            ),
            pale_core::records::CapabilityProvider::Connector => {
                format!("connector `{}`", required_target(&capability, "connector")?)
            }
        };
        caps.push_str(&format!(
            "  - {capability_id} [{}; {}; approval {}]\n",
            capability.effect_class, provider, capability.approval,
        ));
    }
    let mutating = if repo.mutating_not_readonly.is_empty() {
        "  (none declared)\n".to_string()
    } else {
        repo.mutating_not_readonly
            .iter()
            .map(|c| format!("  - {c} (never automatic)\n"))
            .collect()
    };
    Ok(format!(
        "<!-- pale: generated capabilities profile={dg} -- binding data; do not edit here. -->\n\
         # Pale tool configuration: {role}\n\n\
         Packages describe reusable methods; this configuration chooses their providers.\n\
         Check that a chosen provider is available, then inspect what its run returned.\n\
         Use the guides for decisions that need the model's interpretation.\n\n\
         Configured providers:\n{caps}\n\
         Commands that change state (never automatic):\n{mutating}",
        role = repo.role,
        caps = caps,
        mutating = mutating,
    ))
}

/// Require the concrete bounded target owned by a target-bearing provider kind.
fn required_target<'a>(
    capability: &'a ResolvedCapabilityProviderV1,
    provider: &str,
) -> Result<&'a str, RenderError> {
    capability
        .implementation
        .target()
        .filter(|target| !target.trim().is_empty())
        .ok_or_else(|| {
            RenderError::invalid_input(
                "capability bindings",
                format!(
                    "{provider} capability `{}` has no concrete target",
                    capability.capability_id
                ),
            )
        })
}

/// Collapse equal provider selections while rejecting cross-operation conflicts.
///
/// Unbound requirements are intentionally absent: this record describes private
/// selections, not the complete semantic requirement inventory.
fn capability_providers(
    semantics: &RepositorySemanticPlan,
) -> Result<BTreeMap<CapabilityIdV1, ResolvedCapabilityProviderV1>, RenderError> {
    let mut providers = BTreeMap::new();
    for capability in semantics
        .operations
        .iter()
        .flat_map(|operation| operation.capabilities())
    {
        let Some(binding) = capability.binding() else {
            continue;
        };
        let candidate = ResolvedCapabilityProviderV1 {
            capability_id: capability.capability_id.clone(),
            effect_class: capability.effect_class,
            approval: binding.approval,
            tool_contract: binding.tool_contract.clone(),
            implementation: binding.implementation.clone(),
            note: binding.note.clone(),
        };
        match providers.entry(candidate.capability_id.clone()) {
            Entry::Vacant(entry) => {
                entry.insert(candidate);
            }
            Entry::Occupied(entry) if entry.get() != &candidate => {
                return Err(RenderError::invalid_input(
                    "capability bindings",
                    format!(
                        "capability `{}` resolves to conflicting providers",
                        candidate.capability_id
                    ),
                ));
            }
            Entry::Occupied(_) => {}
        }
    }
    Ok(providers)
}

/// A typed record of the capability bindings actually used by this repository's
/// selected semantic graph.
pub(super) fn capability_binding_record(
    identity: &RenderIdentity<'_>,
    semantics: &RepositorySemanticPlan,
) -> Result<String, RenderError> {
    let providers = capability_providers(semantics)?;
    let mut requirements = Vec::new();
    let mut record =
        |kind, owner: &PackageResourceRefV1, capability: &crate::semantic::ResolvedCapability| {
            requirements.push(ResolvedCapabilityUseV1 {
                kind,
                owner: owner.clone(),
                requirement_id: capability.requirement_id.clone(),
                capability_id: capability.capability_id.clone(),
                recipe: capability.recipe.clone(),
                effect_class: capability.effect_class,
                policy: capability.policy.clone(),
                failure_posture: capability.failure_posture,
            });
        };
    for operation in &semantics.operations {
        for capability in operation.capabilities() {
            record(
                CapabilityUseKind::OperationRequirement,
                &operation.record.id,
                capability,
            );
        }
    }
    requirements.sort_by(|left, right| {
        (
            &left.kind,
            &left.owner,
            &left.requirement_id,
            &left.capability_id,
        )
            .cmp(&(
                &right.kind,
                &right.owner,
                &right.requirement_id,
                &right.capability_id,
            ))
    });
    let schema_identity = pale_contracts::record_schema_identity("capability-bindings.v1")
        .map_err(|error| {
            RenderError::projection_error("capability bindings schema identity", error)
        })?;
    let doc = CapabilityBindingsV1 {
        schema: schema_identity.schema,
        schema_sha256: schema_identity.schema_sha256,
        schema_version: 1,
        profile_digest: identity.profile().clone(),
        recipes: semantics
            .recipes
            .iter()
            .map(|recipe| ResolvedToolRecipeV1 {
                resource: recipe.resource.clone(),
                procedure: recipe.procedure.clone(),
            })
            .collect(),
        providers: providers.into_values().collect(),
        requirements,
    };
    pale_contracts::validate_record(&doc, "capability-bindings.v1")
        .map_err(|error| RenderError::projection_error("generated capability bindings", error))?;
    json_pretty(&doc, "generated JSON surface")
}
