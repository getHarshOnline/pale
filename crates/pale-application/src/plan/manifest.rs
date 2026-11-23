//! Canonical manifest construction over rendered surface bytes.
//!
//! The builder sorts and seals exact surface identities, then re-verifies the
//! result through its schema. It does not observe or mutate a target checkout.

use pale_core::records::{
    CompilationInputDigest, GeneratedAgentSurfaceManifestPreimageV1,
    GeneratedAgentSurfaceManifestV1, LogicalRepositoryIdV1, SchemaRegistryIdentity,
};

use crate::identity::ProjectIdentity;
use crate::render::ProjectRender;

use super::materialization::PreparedSurfaceSet;
use super::{validate, PlanError, COMPILER_VERSION, RENDERER_VERSION};

/// Seal a repository's complete rendered target set using its verified owners.
///
/// [`ProjectIdentity`] carries the exact immutable profile and instance; callers
/// cannot substitute a record after identity admission. Repository selection,
/// target confinement, output uniqueness and the resulting schema are still
/// checked for each manifest. Output order is canonical by relative path.
pub(crate) fn build_manifest_with_identity(
    schema_registry: &SchemaRegistryIdentity,
    compilation_input_digest: &CompilationInputDigest,
    render: &ProjectRender,
    repository_id: &LogicalRepositoryIdV1,
    identity: ProjectIdentity<'_>,
) -> Result<GeneratedAgentSurfaceManifestV1, PlanError> {
    let profile = identity.profile_record();
    if !profile
        .repositories
        .iter()
        .any(|repository| &repository.id == repository_id)
    {
        return Err(PlanError::InvalidRunTarget(repository_id.to_string()));
    }
    let prepared = PreparedSurfaceSet::from_render(render)?;
    let outputs = prepared
        .surfaces()
        .map(|surface| surface.digest_record().map_err(PlanError::InvalidSurface))
        .collect::<Result<Vec<_>, _>>()?;
    let schema_identity = pale_contracts::record_schema_identity("generated-manifest.v1")?;
    let manifest =
        GeneratedAgentSurfaceManifestV1::seal(GeneratedAgentSurfaceManifestPreimageV1 {
            schema: schema_identity.schema,
            schema_sha256: schema_identity.schema_sha256,
            schema_manifest_sha256: schema_registry.schema_manifest_sha256.clone(),
            schema_set_sha256: schema_registry.schema_set_sha256.clone(),
            compilation_input_digest: compilation_input_digest.clone(),
            compiler_version: COMPILER_VERSION.to_string(),
            renderer_version: RENDERER_VERSION.to_string(),
            project_id: profile.project_id.clone(),
            execution_workspace_id: profile.execution_workspace_id.clone(),
            repository_id: repository_id.clone(),
            profile_digest: identity.profile().clone(),
            instance_digest: identity.instance().clone(),
            outputs,
        })?;
    validate(&manifest, "generated-manifest.v1")?;
    Ok(manifest)
}
