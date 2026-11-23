//! Exact prompt dependency closure for one operation contract.

use crate::terminal_adapters::TerminalAdapterDescriptor;

use std::collections::{BTreeMap, BTreeSet};

use pale_core::records::{
    HostToolBinding, NonRootJsonPointer, OperationSemanticDependencyV1, PackageResourceRefV1,
    SealedArtifactRef, SemanticDependencyKind, ToolObservability,
};

use super::super::semantic_content::role_name;
use super::super::types::{HarnessSurfaceRefs, RenderError};
use crate::semantic::{RepositorySemanticPlan, ResolvedOperation};

/// Seal the operation's selected semantic bodies as explicit dependencies.
pub(super) fn semantic_dependencies(
    operation: &ResolvedOperation,
    artifacts: &HarnessSurfaceRefs,
) -> Result<Vec<OperationSemanticDependencyV1>, RenderError> {
    let mut dependencies = BTreeSet::new();
    for role_id in &operation.record.roles {
        insert_dependency(
            &mut dependencies,
            SemanticDependencyKind::Role,
            role_id,
            &artifacts.roles,
        )?;
    }
    Ok(dependencies.into_iter().collect())
}

/// Insert one exact rendered artifact reference into the deduplicated dependency set.
fn insert_dependency(
    dependencies: &mut BTreeSet<OperationSemanticDependencyV1>,
    kind: SemanticDependencyKind,
    id: &PackageResourceRefV1,
    artifacts: &BTreeMap<PackageResourceRefV1, SealedArtifactRef>,
) -> Result<(), RenderError> {
    let artifact = artifacts.get(id).cloned().ok_or_else(|| {
        RenderError::invalid_input(
            "semantic dependencies",
            format!(
                "dependency `{}` has no rendered artifact",
                crate::semantic::resource_label(id)
            ),
        )
    })?;
    dependencies.insert(OperationSemanticDependencyV1 {
        kind,
        id: id.clone(),
        artifact,
    });
    Ok(())
}

/// Resolve non-capability host tools that the operation may use without claiming effects.
pub(super) fn auxiliary_tools(
    operation: &ResolvedOperation,
    semantics: &RepositorySemanticPlan,
    host: TerminalAdapterDescriptor,
) -> Result<Vec<HostToolBinding>, RenderError> {
    if host != TerminalAdapterDescriptor::claude_code() {
        return Ok(Vec::new());
    }
    let mut tools = BTreeSet::new();
    for role_id in &operation.record.roles {
        let role = semantics
            .roles
            .iter()
            .find(|role| role.record.id == *role_id)
            .ok_or_else(|| {
                RenderError::invalid_input(
                    "operation roles",
                    format!(
                        "operation {} references missing role {}",
                        crate::semantic::resource_label(&operation.record.id),
                        crate::semantic::resource_label(role_id)
                    ),
                )
            })?;
        tools.insert(auxiliary_binding(
            "Agent",
            "/subagent_type",
            role_name(role),
        )?);
    }
    Ok(tools.into_iter().collect())
}

/// Construct one pre-tool-observable host binding from a validated discriminator.
fn auxiliary_binding(
    name: &str,
    pointer: &str,
    value: String,
) -> Result<HostToolBinding, RenderError> {
    let pointer = NonRootJsonPointer::try_from(pointer).map_err(|error| {
        RenderError::invalid_input(
            "operation auxiliary tools",
            format!("internal input discriminator is invalid: {error}"),
        )
    })?;
    Ok(HostToolBinding {
        name: name.to_string(),
        model_name: None,
        observability: ToolObservability::PreToolObservable,
        commands: Vec::new(),
        command_source: None,
        input_equals: [(pointer, value)].into(),
    })
}
