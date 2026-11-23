//! Shared discovery artifacts and compiled native-action contracts.
//!
//! The projection renders deterministic catalogs once per repository. Path
//! bindings reference those emitted artifacts; rendering here does not expose
//! an operation, publish a release, or authorize an action invocation.

use crate::terminal_adapters::TerminalAdapterDescriptor;

use std::collections::{BTreeMap, BTreeSet};

use pale_core::records::{CapabilityProvider, RepositoryNativeActionCatalogV1, SealedArtifactRef};

use super::super::family_content::OrderedFamily;
use super::super::{
    all_execution_path_ids, all_family_ids, artifact_ref, capabilities_content,
    capability_binding_record, execution_path_ids, execution_paths_for_harness, json_pretty,
    operation_is_bound, semantic_index, GeneratedSurface, RenderContext, RenderError,
    RenderIdentity, SurfaceKind,
};

/// Render discovery records from semantic declarations and independently observed mechanics.
pub(super) fn render_discovery_artifacts(
    input: &RenderContext<'_, '_>,
    identity: &RenderIdentity<'_>,
    by_harness: &BTreeMap<TerminalAdapterDescriptor, Vec<OrderedFamily<'_>>>,
    harness_artifacts: &BTreeMap<TerminalAdapterDescriptor, super::super::HarnessSurfaceRefs>,
    surfaces: &mut Vec<GeneratedSurface>,
) -> Result<(), RenderError> {
    let projection = &input.projection;
    let profile = projection.identity.profile_record();
    let repo = projection.repository;
    let semantics = projection.semantics;
    let consumers = all_execution_path_ids(profile);
    let ids = all_family_ids(by_harness);
    surfaces.push(
        GeneratedSurface::whole_file(
            ".pale/semantic-index.md",
            SurfaceKind::SemanticIndex,
            semantic_index(repo, semantics),
            consumers.clone(),
            semantics
                .operations
                .iter()
                .map(|operation| {
                    format!(
                        "operation:{}",
                        crate::semantic::resource_label(&operation.record.id)
                    )
                })
                .collect(),
            true,
        )
        .with_note("Discovery index; detailed judgment loads through native skills and agents."),
    );
    surfaces.push(
        GeneratedSurface::whole_file(
            ".pale/capabilities.md",
            SurfaceKind::Capabilities,
            capabilities_content(repo, semantics, identity)?,
            consumers.clone(),
            ids.clone(),
            true,
        )
        .with_note("Resolved mechanical grants; doctrine retains harness judgment."),
    );
    surfaces.push(GeneratedSurface::whole_file(
        ".pale/capability-bindings.json",
        SurfaceKind::CapabilityBindings,
        capability_binding_record(identity, semantics)?,
        consumers,
        ids,
        true,
    ));

    render_native_action_contracts(input, identity, harness_artifacts, surfaces)
}

/// Render only action contracts that are selected and usable in a host projection.
fn render_native_action_contracts(
    input: &RenderContext<'_, '_>,
    identity: &RenderIdentity<'_>,
    harness_artifacts: &BTreeMap<TerminalAdapterDescriptor, super::super::HarnessSurfaceRefs>,
    surfaces: &mut Vec<GeneratedSurface>,
) -> Result<(), RenderError> {
    let projection = &input.projection;
    let profile = projection.identity.profile_record();
    validate_declared_native_actions(projection.semantics, input.native_actions)?;
    let native_action_ids = selected_native_action_ids(harness_artifacts);
    if native_action_ids.is_empty() {
        return Ok(());
    }

    let input_schema = pale_contracts::embedded_schema("action-input.v1")
        .map_err(|error| RenderError::projection_error("native action input schema", error))?
        .to_string();
    let input_schema_ref = artifact_ref(".pale/native-action-input.schema.json", &input_schema);
    surfaces.push(
        GeneratedSurface::whole_file(
            ".pale/native-action-input.schema.json",
            SurfaceKind::NativeActionInputSchema,
            input_schema,
            all_execution_path_ids(profile),
            native_action_ids.iter().cloned().collect(),
            true,
        )
        .with_note("Exact strict JSON payload grammar for every selected native action."),
    );
    render_native_action_tool_server_contributions(
        input,
        identity,
        harness_artifacts,
        &input_schema_ref,
        surfaces,
    )
}

/// Render one inert tool-server contribution for each harness using the catalog.
fn render_native_action_tool_server_contributions(
    input: &RenderContext<'_, '_>,
    identity: &RenderIdentity<'_>,
    harness_artifacts: &BTreeMap<TerminalAdapterDescriptor, super::super::HarnessSurfaceRefs>,
    input_schema: &SealedArtifactRef,
    surfaces: &mut Vec<GeneratedSurface>,
) -> Result<(), RenderError> {
    let profile = input.projection.identity.profile_record();
    for (harness, artifacts) in harness_artifacts {
        let action_ids = selected_native_action_ids_for_harness(artifacts);
        if action_ids.is_empty() {
            continue;
        }
        let paths = execution_paths_for_harness(
            profile,
            input.projection.experience_composition_plan,
            *harness,
        )?;
        let execution_paths = execution_path_ids(&paths);
        let catalog_ref = render_host_native_action_catalog(
            input,
            identity,
            *harness,
            &action_ids,
            execution_paths.clone(),
            surfaces,
        )?;
        surfaces.push(
            GeneratedSurface::whole_file(
                tool_server_contribution_path(*harness),
                SurfaceKind::NativeActionToolServerContribution,
                tool_server_contribution(*harness, &catalog_ref, input_schema)?,
                execution_paths,
                action_ids.into_iter().collect(),
                false,
            )
            .with_note(
                "Pale-owned input for a later structural native-config edit. It neither replaces native settings nor proves that pale-mcp is installed, attached, or running.",
            ),
        );
    }
    Ok(())
}

/// Seal only the native contracts usable by this exact harness.
fn render_host_native_action_catalog(
    input: &RenderContext<'_, '_>,
    identity: &RenderIdentity<'_>,
    harness: TerminalAdapterDescriptor,
    native_action_ids: &BTreeSet<String>,
    execution_paths: Vec<pale_core::records::ExecutionPathSelectionIdV1>,
    surfaces: &mut Vec<GeneratedSurface>,
) -> Result<SealedArtifactRef, RenderError> {
    let repo = input.projection.repository;
    let contracts: Vec<_> = input
        .native_actions
        .contracts()
        .filter(|contract| native_action_ids.contains(&contract.id))
        .cloned()
        .collect();
    let resolved_action_ids = contracts
        .iter()
        .map(|contract| contract.id.clone())
        .collect::<BTreeSet<_>>();
    if &resolved_action_ids != native_action_ids {
        let missing = native_action_ids
            .difference(&resolved_action_ids)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        return Err(RenderError::invalid_input(
            "native action catalog",
            format!(
                "selected native action contracts are absent from the runtime registry: {missing}"
            ),
        ));
    }
    let schema_identity = pale_contracts::record_schema_identity(
        "repository-native-action-catalog.v1",
    )
    .map_err(|error| RenderError::projection_error("native action schema identity", error))?;
    let catalog = RepositoryNativeActionCatalogV1 {
        schema: schema_identity.schema,
        schema_sha256: schema_identity.schema_sha256,
        schema_version: 1,
        profile_digest: identity.profile().clone(),
        instance_digest: identity.instance().clone(),
        repository_id: repo.id.clone(),
        contracts,
    };
    pale_contracts::validate_record(&catalog, "repository-native-action-catalog.v1")
        .map_err(|error| RenderError::projection_error("native action catalog", error))?;
    let content = json_pretty(&catalog, "native action catalog")?;
    let catalog_path = format!(".pale/tools/{harness}/native-actions.json");
    let catalog_ref = artifact_ref(&catalog_path, &content);
    surfaces.push(
        GeneratedSurface::whole_file(
            catalog_path,
            SurfaceKind::NativeActionCatalog,
            content,
            execution_paths,
            native_action_ids.iter().cloned().collect(),
            false,
        )
        .with_note("Closed deterministic host-selected action contracts. Attachment remains inert until an admitted executable binding selects this catalog."),
    );
    Ok(catalog_ref)
}

/// Stable Pale-owned target for one harness-specific tool-server contribution.
fn tool_server_contribution_path(harness: TerminalAdapterDescriptor) -> String {
    format!(".pale/tools/{harness}/mcp-server-contribution.json")
}

/// Render exact editor input without claiming that its native target was changed.
fn tool_server_contribution(
    harness: TerminalAdapterDescriptor,
    catalog: &SealedArtifactRef,
    input_schema: &SealedArtifactRef,
) -> Result<String, RenderError> {
    let (native_path, native_entry, fragment) = if harness.is_claude_code() {
        (
            ".mcp.json",
            "mcpServers.pale",
            serde_json::json!({
                "mcpServers": {
                    "pale": {
                        "args": [],
                        "command": "pale-mcp"
                    }
                }
            }),
        )
    } else {
        (
            ".codex/config.toml",
            "mcp_servers.pale",
            serde_json::json!("[mcp_servers.pale]\ncommand = \"pale-mcp\"\nargs = []\n"),
        )
    };
    json_pretty(
        &serde_json::json!({
            "attachment_state": "not-established",
            "catalog": catalog,
            "fragment": fragment,
            "harness": harness.as_str(),
            "input_schema": input_schema,
            "native_target": {
                "entry": native_entry,
                "path": native_path
            },
            "transport": "stdio"
        }),
        "native-action tool-server contribution",
    )
}

/// Validate selected native-action identities independently of transport admission.
///
/// Repository catalog attachment requires a separately admitted executable
/// binding, but its absence must not make an unknown Pale action contract
/// semantically valid. This check owns that distinction;
/// [`selected_native_action_ids`] owns only the selected effect-free catalog.
fn validate_declared_native_actions(
    semantics: &crate::semantic::RepositorySemanticPlan,
    registry: &crate::actions::NativeActionRegistry,
) -> Result<(), RenderError> {
    let registered = registry
        .contracts()
        .map(|contract| contract.id.as_str())
        .collect::<BTreeSet<_>>();
    for capability in semantics
        .operations
        .iter()
        .filter(|operation| operation_is_bound(operation))
        .flat_map(|operation| operation.capabilities())
        .filter(|capability| {
            capability
                .binding()
                .is_some_and(|binding| binding.provider() == CapabilityProvider::NativeRust)
        })
    {
        let target = capability
            .binding()
            .and_then(pale_core::records::CapabilityBinding::target)
            .filter(|target| !target.trim().is_empty())
            .ok_or_else(|| {
                RenderError::invalid_input(
                    "native action catalog",
                    format!(
                        "native Rust capability `{}` has no bounded-tool target",
                        capability.capability_id
                    ),
                )
            })?;
        if !registered.contains(target) {
            return Err(RenderError::invalid_input(
                "native action catalog",
                format!(
                    "selected native action contract is absent from the runtime registry: {target}"
                ),
            ));
        }
    }
    Ok(())
}

/// Collect only actions proven usable by at least one host-specific contract.
///
/// The per-harness projection is the SSOT for the joined private binding,
/// verified machine carrier, bounded-tool declaration, and runtime observation.
/// Re-reading pre-projection semantics here would either miss real machine
/// evidence or manufacture availability from configuration alone.
fn selected_native_action_ids(
    harness_artifacts: &BTreeMap<TerminalAdapterDescriptor, super::super::HarnessSurfaceRefs>,
) -> BTreeSet<String> {
    harness_artifacts
        .values()
        .flat_map(selected_native_action_ids_for_harness)
        .collect()
}

/// Collect actions selected by one exact native-harness projection.
fn selected_native_action_ids_for_harness(
    artifacts: &super::super::HarnessSurfaceRefs,
) -> BTreeSet<String> {
    artifacts
        .operations
        .values()
        .flat_map(|operation| &operation.support.capabilities)
        .filter_map(|capability| {
            capability
                .provider_state
                .native_action()
                .map(str::to_string)
        })
        .collect()
}

#[cfg(test)]
#[path = "../../../tests/internal/render/discovery.rs"]
mod tests;
