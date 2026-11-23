//! Validation and construction of exact target ownership state.

use std::collections::BTreeMap;

use pale_core::records::{
    LogicalRepositoryIdV1, ManifestDigest, MaterializationStateDigest,
    MaterializationStatePreimageV1, MaterializationStateV1, OneShotCompilationDigest,
    PriorMaterializationStateV1, SchemaRegistryIdentity, SurfaceDigest, MATERIALIZATION_STATE_PATH,
    MATERIALIZATION_STATE_SCHEMA, MATERIALIZATION_STATE_SCHEMA_VERSION,
};

use super::{validate_target, PreparedSurfaceSet};
use crate::identity::ProjectIdentity;
use crate::plan::{validate, PlanError};

/// Verified prior-state identity paired with its repository-relative surface index.
type IndexedState<'a> = (
    Option<PriorMaterializationStateV1>,
    BTreeMap<&'a str, &'a SurfaceDigest>,
);

/// Re-admit a retained state record before any ownership comparison uses it.
///
/// The optional compilation digest is required for newly constructed state and
/// intentionally omitted when inspecting prior state from an earlier compilation.
fn validate_state(
    state: &MaterializationStateV1,
    repository_id: &LogicalRepositoryIdV1,
    schema_registry: &SchemaRegistryIdentity,
    one_shot_compilation_digest: Option<&OneShotCompilationDigest>,
) -> Result<MaterializationStateDigest, PlanError> {
    validate(state, "materialization-state.v1")?;
    let carried_identity = pale_core::records::RecordSchemaIdentity {
        schema: state.schema.clone(),
        schema_sha256: state.schema_sha256.clone(),
    };
    if pale_contracts::admit_record_schema_identity("materialization-state.v1", &carried_identity)
        .is_err()
        || state.schema.as_str() != MATERIALIZATION_STATE_SCHEMA
        || state.schema_version != MATERIALIZATION_STATE_SCHEMA_VERSION
        || &state.repository_id != repository_id
        || state.schema_manifest_sha256 != schema_registry.schema_manifest_sha256
        || state.schema_set_sha256 != schema_registry.schema_set_sha256
        || one_shot_compilation_digest
            .is_some_and(|expected| &state.one_shot_compilation_digest != expected)
    {
        return Err(PlanError::InvalidMaterializationState(
            "record identity does not match the target repository".into(),
        ));
    }
    let actual = state.recompute_digest()?;
    if state.state_digest != actual {
        return Err(PlanError::InvalidMaterializationState(
            "record self-digest does not match its content".into(),
        ));
    }
    let mut previous = None;
    for surface in &state.surfaces {
        validate_target(&surface.rel_path).map_err(|invalid| {
            PlanError::InvalidMaterializationState(format!(
                "owned target `{}` is not confined",
                invalid.into_path()
            ))
        })?;
        if surface.rel_path == MATERIALIZATION_STATE_PATH {
            return Err(PlanError::InvalidMaterializationState(
                "state record cannot own itself as a generated surface".into(),
            ));
        }
        if previous.is_some_and(|path: &str| path >= surface.rel_path.as_str()) {
            return Err(PlanError::InvalidMaterializationState(
                "surface paths are not unique and canonically ordered".into(),
            ));
        }
        previous = Some(surface.rel_path.as_str());
    }
    Ok(actual)
}

/// Build the next immutable state record from non-deferred filesystem surfaces.
pub(super) fn desired_state(
    identity: ProjectIdentity<'_>,
    prepared: &PreparedSurfaceSet<'_>,
    repository_id: &LogicalRepositoryIdV1,
    manifest_digest: &ManifestDigest,
    schema_registry: &SchemaRegistryIdentity,
    one_shot_compilation_digest: &OneShotCompilationDigest,
) -> Result<MaterializationStateV1, PlanError> {
    let surfaces = desired_surfaces(prepared)?;
    let schema_identity = pale_contracts::record_schema_identity("materialization-state.v1")?;
    let state = MaterializationStateV1::seal(MaterializationStatePreimageV1 {
        schema: schema_identity.schema,
        schema_sha256: schema_identity.schema_sha256,
        schema_version: MATERIALIZATION_STATE_SCHEMA_VERSION,
        schema_manifest_sha256: schema_registry.schema_manifest_sha256.clone(),
        schema_set_sha256: schema_registry.schema_set_sha256.clone(),
        one_shot_compilation_digest: one_shot_compilation_digest.clone(),
        repository_id: repository_id.clone(),
        profile_digest: identity.profile().clone(),
        instance_digest: identity.instance().clone(),
        manifest_digest: manifest_digest.clone(),
        surfaces,
    })?;
    validate_state(
        &state,
        repository_id,
        schema_registry,
        Some(one_shot_compilation_digest),
    )?;
    Ok(state)
}

/// Project sorted non-deferred filesystem surfaces through the digest owner.
///
/// # Errors
///
/// Returns [`PlanError::InvalidSurface`] when a rendered filesystem surface
/// contradicts its ownership or digest contract.
pub(super) fn desired_surfaces(
    prepared: &PreparedSurfaceSet<'_>,
) -> Result<Vec<SurfaceDigest>, PlanError> {
    prepared
        .materializable_surfaces()
        .map(|surface| surface.digest_record().map_err(PlanError::InvalidSurface))
        .collect()
}

/// Validate prior state and expose its state digest, compilation digest, and target index.
pub(super) fn index_state<'a>(
    state: Option<&'a MaterializationStateV1>,
    repository_id: &LogicalRepositoryIdV1,
    schema_registry: &SchemaRegistryIdentity,
) -> Result<IndexedState<'a>, PlanError> {
    let Some(state) = state else {
        return Ok((None, BTreeMap::new()));
    };
    let digest = validate_state(state, repository_id, schema_registry, None)?;
    Ok((
        Some(PriorMaterializationStateV1 {
            state_digest: digest,
            one_shot_compilation_digest: state.one_shot_compilation_digest.clone(),
        }),
        state
            .surfaces
            .iter()
            .map(|surface| (surface.rel_path.as_str(), surface))
            .collect(),
    ))
}
