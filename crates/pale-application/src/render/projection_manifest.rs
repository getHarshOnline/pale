//! Harness-wide integrity closure over materialized projection artifacts.

use crate::terminal_adapters::TerminalAdapterDescriptor;

use std::collections::{BTreeMap, BTreeSet};

use pale_core::managed::split_managed_block;
use pale_core::records::{
    harness_projection_manifest_path, EffectiveProjectProfileV1, ExperienceCompositionPlanV1,
    HarnessProjectionManifestV1, ManagedFrontDoorDigest, ProfileRepository, SealedArtifactRef,
    SurfaceOwnership, HARNESS_PROJECTION_SCHEMA_VERSION,
};

use super::RenderIdentity;
use super::{
    artifact_ref, execution_path_ids, execution_paths_for_harness, json_pretty, GeneratedSurface,
    RenderError, SurfaceKind,
};

/// Seal every already-rendered, materialized artifact consumed by each native
/// harness. This runs before path contexts and activation wiring are rendered,
/// keeping those sealing records outside their own integrity closure.
pub(super) fn render_projection_manifests(
    profile: &EffectiveProjectProfileV1,
    composition_plan: &ExperienceCompositionPlanV1,
    identity: &RenderIdentity<'_>,
    repository: &ProfileRepository,
    surfaces: &mut Vec<GeneratedSurface>,
) -> Result<BTreeMap<TerminalAdapterDescriptor, SealedArtifactRef>, RenderError> {
    let mut manifests = BTreeMap::new();
    for harness in crate::terminal_adapters::descriptors() {
        let paths = execution_paths_for_harness(profile, composition_plan, harness)?;
        if paths.is_empty() {
            continue;
        }
        let consumers = execution_path_ids(&paths);
        let provider_binding = paths
            .iter()
            .map(|path| {
                crate::terminal_adapters::for_selection(path, composition_plan).map_err(|error| {
                    RenderError::invalid_input_error("execution-path terminal adapter", error)
                })
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|(provider, _)| provider)
            .try_fold(None, |selected, provider| match selected {
                None => Ok(Some(provider)),
                Some(selected) if selected == provider => Ok(Some(selected)),
                Some(_) => Err(RenderError::projection(
                    "harness projection manifest",
                    format!("execution paths reaching {harness} select multiple provider bindings"),
                )),
            })?
            .ok_or_else(|| {
                RenderError::projection(
                    "harness projection manifest",
                    format!("execution paths reaching {harness} have no provider binding"),
                )
            })?;
        let consumer_set = consumers.iter().collect::<BTreeSet<_>>();

        let mut front_door = None;
        let mut artifacts = Vec::new();
        let mut policy_ids = BTreeSet::new();
        for surface in surfaces.iter().filter(|surface| {
            !surface.deferred
                && surface
                    .execution_path_consumers
                    .iter()
                    .any(|path| consumer_set.contains(path))
        }) {
            policy_ids.extend(surface.referenced_policy_ids.iter().cloned());
            if surface.ownership == SurfaceOwnership::ManagedBlock {
                if front_door.is_some() {
                    return Err(RenderError::projection(
                        "harness projection manifest",
                        format!("more than one front door was rendered for {harness}"),
                    ));
                }
                let managed = split_managed_block(surface.content()).map_err(|error| {
                    RenderError::projection_error(
                        format!("harness projection front door `{}`", surface.rel_path),
                        error,
                    )
                })?;
                front_door = Some(ManagedFrontDoorDigest {
                    rel_path: surface.rel_path.clone(),
                    kind: surface.kind.clone(),
                    ownership: surface.ownership,
                    managed_block_sha256: pale_core::records::ArtifactDigest::hash_bytes(
                        managed.block().as_bytes(),
                    ),
                });
            } else {
                artifacts.push(surface.digest_record()?);
            }
        }
        artifacts.sort_by(|left, right| left.rel_path.cmp(&right.rel_path));
        let front_door = front_door.ok_or_else(|| {
            RenderError::projection(
                "harness projection manifest",
                format!("no front door was rendered for {harness}"),
            )
        })?;
        let schema_identity = pale_contracts::record_schema_identity("harness-projection.v1")
            .map_err(|error| {
                RenderError::projection_error("harness projection schema identity", error)
            })?;
        let manifest = HarnessProjectionManifestV1 {
            schema: schema_identity.schema,
            schema_sha256: schema_identity.schema_sha256,
            schema_version: HARNESS_PROJECTION_SCHEMA_VERSION,
            repository_id: repository.id.clone(),
            provider_binding: provider_binding.clone(),
            profile_digest: identity.profile().clone(),
            instance_digest: identity.instance().clone(),
            front_door,
            artifacts,
        };
        pale_contracts::validate_record(&manifest, "harness-projection.v1")
            .map_err(|error| RenderError::projection_error("harness projection manifest", error))?;
        let content = json_pretty(&manifest, "harness projection manifest")?;
        let path = harness_projection_manifest_path(&provider_binding);
        let reference = artifact_ref(&path, &content);
        surfaces.push(
            GeneratedSurface::whole_file(
                path,
                SurfaceKind::HarnessProjectionManifest,
                content,
                consumers,
                policy_ids.into_iter().collect(),
                false,
            )
            .with_note("Harness-wide integrity closure over materialized projection artifacts."),
        );
        manifests.insert(harness, reference);
    }
    Ok(manifests)
}
