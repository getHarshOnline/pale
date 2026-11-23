//! Sealed operation contracts and compact native-harness catalogs.

use crate::terminal_adapters::TerminalAdapterDescriptor;

use pale_core::records::{
    provider_binding_projection_root, ExecutionPathSelectionIdV1, ExecutionPathWitnessV1,
    LogicalRepositoryIdV1, OperationCatalogDigest, OperationCatalogEntryV1,
    OperationContractCatalogPreimageV1, OperationContractCatalogV1, OperationContractMechanicsV1,
    OperationContractPreimageV1, OperationContractV1, OperationDispatchV1,
    OperationRouteBindingPreimageV1, OperationRouteBindingV1, OperationSelection, ProfileDigest,
    ProviderBindingRefV1, SealedArtifactRef,
};

use super::semantic_content::operation_name;
use super::types::{HarnessSurfaceRefs, RenderError};
use super::{
    artifact_ref, json_pretty, GeneratedSurface, RenderContext, RenderIdentity, SurfaceKind,
};
use crate::semantic::OperationObligationState;

mod capability;
mod dependencies;

/// Opaque catalog identity carried between a shared projection and path binding.
pub(super) struct OperationCatalogProjection {
    profile_digest: ProfileDigest,
    repository_id: LogicalRepositoryIdV1,
    provider_binding: ProviderBindingRefV1,
    catalog: SealedArtifactRef,
    catalog_digest: OperationCatalogDigest,
    committable: bool,
}

/// One execution path's sealed binding to a shared native-harness operation catalog.
pub(super) struct OperationRouteBindingProjection {
    content: String,
    artifact: SealedArtifactRef,
    committable: bool,
}

impl OperationRouteBindingProjection {
    /// Canonical JSON bytes whose digest is carried by `artifact`.
    pub(super) fn content(&self) -> &str {
        &self.content
    }

    /// Sealed repository-relative identity of this path-specific binding.
    pub(super) fn artifact(&self) -> &SealedArtifactRef {
        &self.artifact
    }

    /// Whether the binding and its transitive catalog are machine-independent.
    pub(super) fn committable(&self) -> bool {
        self.committable
    }
}

pub(super) use capability::{
    operation_is_bound, operation_is_model_facing, operation_requires_manual_invocation,
};
use dependencies::{auxiliary_tools, semantic_dependencies};

/// Render one full contract per operation and one compact catalog per native
/// harness. Selected execution paths bind this projection without copying it.
///
/// The compiler-owned [`crate::semantic::RepositoryObligationClosure`] excludes unbound
/// operations before selection is required or an empty capability slice is
/// interpreted as semantic-only. Capability emptiness alone cannot distinguish
/// unbound from semantic-only mechanics.
pub(super) fn operation_projection(
    input: &RenderContext<'_, '_>,
    identity: &RenderIdentity<'_>,
    host: TerminalAdapterDescriptor,
    provider_binding: &ProviderBindingRefV1,
    artifacts: &HarnessSurfaceRefs,
    consumers: &[ExecutionPathSelectionIdV1],
    surfaces: &mut Vec<GeneratedSurface>,
) -> Result<OperationCatalogProjection, RenderError> {
    let semantics = input.projection.semantics;
    let obligations = input.projection.package_obligations;
    let repository_id = &input.projection.repository.id;
    let profile_digest = identity.profile().clone();
    surfaces.reserve(semantics.operations.len() + 1);
    let mut entries = Vec::with_capacity(semantics.operations.len());
    let mut catalog_committable = true;
    for obligation in &obligations.operations {
        let Some(selection) = obligation.state.selection() else {
            continue;
        };
        let operation = semantics
            .operations
            .iter()
            .find(|operation| operation.record.id == obligation.operation)
            .ok_or_else(|| {
                RenderError::invalid_input(
                    "operation contract",
                    format!(
                        "operation {} has no semantic record",
                        crate::semantic::resource_label(&obligation.operation)
                    ),
                )
            })?;
        let operation_artifacts =
            artifacts
                .operations
                .get(&operation.record.id)
                .ok_or_else(|| {
                    RenderError::projection(
                        "operation contract",
                        format!(
                            "operation {} has no rendered {host} skill",
                            crate::semantic::resource_label(&operation.record.id)
                        ),
                    )
                })?;
        let schema_identity = pale_contracts::record_schema_identity("operation-contract.v1")
            .map_err(|error| {
                RenderError::projection_error("operation contract schema identity", error)
            })?;
        let dispatch = match selection {
            OperationSelection::ModelOrUser { aliases } => OperationDispatchV1::ModelOrUser {
                invocation: operation_name(operation),
                aliases: aliases.clone(),
            },
            OperationSelection::UserOnly { aliases } => OperationDispatchV1::UserOnly {
                invocation: operation_name(operation),
                aliases: aliases.clone(),
            },
            OperationSelection::Internal => OperationDispatchV1::Internal,
        };
        let mechanics = match &obligation.state {
            OperationObligationState::SemanticOnly { .. } => {
                OperationContractMechanicsV1::SemanticOnly
            }
            OperationObligationState::CapabilityBearing { .. } => {
                OperationContractMechanicsV1::CapabilityBearing {
                    capabilities: operation_artifacts.support.capabilities.clone(),
                }
            }
            OperationObligationState::Unbound => unreachable!("unbound state returned selection"),
        };
        let contract = OperationContractV1::seal(OperationContractPreimageV1 {
            schema: schema_identity.schema,
            schema_sha256: schema_identity.schema_sha256,
            schema_version: 1,
            id: operation.record.id.clone(),
            dispatch,
            mechanics,
            skill: operation_artifacts.skill.clone(),
            invocation_policy: operation_artifacts.invocation_policy.clone(),
            semantic_dependencies: semantic_dependencies(operation, artifacts)?,
            auxiliary_tools: auxiliary_tools(operation, semantics, host)?,
            roles: operation.record.roles.clone(),
        })
        .map_err(|error| {
            RenderError::projection_error(
                format!(
                    "operation contract `{}`",
                    crate::semantic::resource_label(&operation.record.id)
                ),
                error,
            )
        })?;
        pale_contracts::validate_record(&contract, "operation-contract.v1").map_err(|error| {
            RenderError::projection_error(
                format!(
                    "operation contract `{}`",
                    crate::semantic::resource_label(&contract.id)
                ),
                error,
            )
        })?;
        let contract_content = json_pretty(&contract, "operation contract")?;
        let contract_path = format!(
            "{}/operations/{}.json",
            provider_binding_projection_root(provider_binding),
            crate::render::semantic_helpers::resource_slug(&operation.record.id)
        );
        let contract_ref = artifact_ref(&contract_path, &contract_content);
        let contract_committable = !operation_artifacts.support.machine_bound;
        catalog_committable &= contract_committable;
        surfaces.push(
            GeneratedSurface::whole_file(
                contract_path,
                SurfaceKind::OperationContract,
                contract_content,
                consumers.to_vec(),
                vec![format!(
                    "operation:{}",
                    crate::semantic::resource_label(&operation.record.id)
                )],
                contract_committable,
            )
            .with_note("Exact mechanical contract and sealed prompt dependency closure."),
        );
        entries.push(OperationCatalogEntryV1::from_contract(
            &contract,
            contract_ref,
        ));
    }

    entries.sort_by(|left, right| left.id.cmp(&right.id));
    let schema_identity =
        pale_contracts::record_schema_identity("operation-catalog.v1").map_err(|error| {
            RenderError::projection_error("operation catalog schema identity", error)
        })?;
    let catalog = OperationContractCatalogV1::seal(OperationContractCatalogPreimageV1 {
        schema: schema_identity.schema,
        schema_sha256: schema_identity.schema_sha256,
        schema_version: 1,
        profile_digest: profile_digest.clone(),
        repository_id: repository_id.clone(),
        provider_binding: provider_binding.clone(),
        entries,
    })
    .map_err(|error| RenderError::projection_error("operation catalog", error))?;
    let catalog_digest = catalog.catalog_digest.clone();
    pale_contracts::validate_record(&catalog, "operation-catalog.v1")
        .map_err(|error| RenderError::projection_error("operation catalog", error))?;
    let catalog_content = json_pretty(&catalog, "operation catalog")?;
    let catalog_path = format!(
        "{}/operation-catalog.json",
        provider_binding_projection_root(provider_binding)
    );
    let catalog_ref = artifact_ref(&catalog_path, &catalog_content);
    surfaces.push(
        GeneratedSurface::whole_file(
            catalog_path,
            SurfaceKind::OperationCatalog,
            catalog_content,
            consumers.to_vec(),
            obligations
                .operations
                .iter()
                .filter(|obligation| obligation.state.selection().is_some())
                .map(|obligation| {
                    format!(
                        "operation:{}",
                        crate::semantic::resource_label(&obligation.operation)
                    )
                })
                .collect(),
            catalog_committable,
        )
        .with_note("Compact native-harness lookup catalog; full contracts load on demand."),
    );
    Ok(OperationCatalogProjection {
        profile_digest,
        repository_id: repository_id.clone(),
        provider_binding: provider_binding.clone(),
        catalog: catalog_ref,
        catalog_digest,
        committable: catalog_committable,
    })
}

/// Bind one selected execution path to its native-harness catalog.
pub(super) fn operation_route_binding(
    witness: &ExecutionPathWitnessV1,
    projection: &OperationCatalogProjection,
) -> Result<OperationRouteBindingProjection, RenderError> {
    if witness.terminal_provider_binding.as_ref() != Some(&projection.provider_binding) {
        return Err(RenderError::invalid_input(
            "operation route binding",
            format!(
                "execution path `{}` and its operation catalog carry different terminal provider bindings",
                witness.selection.id(),
            ),
        ));
    }
    let schema_identity = pale_contracts::record_schema_identity("operation-route-binding.v1")
        .map_err(|error| {
            RenderError::projection_error("operation route binding schema identity", error)
        })?;
    let binding = OperationRouteBindingV1::seal(OperationRouteBindingPreimageV1 {
        schema: schema_identity.schema,
        schema_sha256: schema_identity.schema_sha256,
        schema_version: 1,
        profile_digest: projection.profile_digest.clone(),
        repository_id: projection.repository_id.clone(),
        execution_path_witness: witness.clone(),
        catalog: projection.catalog.clone(),
        catalog_digest: projection.catalog_digest.clone(),
    })
    .map_err(|error| RenderError::projection_error("operation route binding", error))?;
    pale_contracts::validate_record(&binding, "operation-route-binding.v1")
        .map_err(|error| RenderError::projection_error("operation route binding", error))?;
    let content = json_pretty(&binding, "operation route binding")?;
    let rel_path = format!(
        ".pale/execution-paths/{}/operation-binding.json",
        witness.selection.id()
    );
    let artifact = artifact_ref(rel_path, &content);
    Ok(OperationRouteBindingProjection {
        content,
        artifact,
        committable: projection.committable,
    })
}
