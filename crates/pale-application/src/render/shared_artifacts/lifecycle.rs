//! Runtime-mode, harness-injection, and lifecycle-guidance artifacts.

use crate::terminal_adapters::TerminalAdapterDescriptor;

use std::collections::BTreeMap;

use pale_core::records::{
    harness_injection_path, ArtifactDigest, EffectiveProjectProfileV1, ExperienceCompositionPlanV1,
    LifecycleBehaviorIdV1, PackageResourceRefV1, ProfileRepository, SealedArtifactRef,
};

use super::super::family_content::OrderedFamily;
use super::super::{
    all_execution_path_ids, artifact_ref, execution_path_ids, execution_paths_for_harness,
    family_ids, injection_content, runtime_mode_content, GeneratedSurface, RenderError,
    RenderIdentity, SurfaceKind,
};
use crate::semantic::RepositorySemanticPlan;

/// References consumed when each execution path is sealed.
pub(super) struct LifecycleArtifacts {
    pub(super) injection_digests: BTreeMap<TerminalAdapterDescriptor, ArtifactDigest>,
    pub(super) run_mode_bodies: BTreeMap<PackageResourceRefV1, SealedArtifactRef>,
    pub(super) behaviour_guidance: BTreeMap<LifecycleBehaviorIdV1, SealedArtifactRef>,
}

/// Render selected lifecycle policy and run-mode artifacts as one coherent set.
pub(super) fn render_lifecycle_artifacts(
    profile: &EffectiveProjectProfileV1,
    composition_plan: &ExperienceCompositionPlanV1,
    identity: &RenderIdentity<'_>,
    repo: &ProfileRepository,
    semantics: &RepositorySemanticPlan,
    by_harness: &BTreeMap<TerminalAdapterDescriptor, Vec<OrderedFamily<'_>>>,
    surfaces: &mut Vec<GeneratedSurface>,
) -> Result<LifecycleArtifacts, RenderError> {
    let mut run_mode_bodies = BTreeMap::new();
    for mode in &semantics.run_modes {
        if let Some(content) = runtime_mode_content(mode) {
            let path = format!(
                ".pale/modes/{}.md",
                crate::render::semantic_helpers::resource_slug(&mode.record.id)
            );
            run_mode_bodies.insert(mode.record.id.clone(), artifact_ref(&path, &content));
            surfaces.push(
                GeneratedSurface::whole_file(
                    path,
                    SurfaceKind::RunModeContext,
                    content,
                    all_execution_path_ids(profile),
                    vec![format!(
                        "run-mode:{}",
                        crate::semantic::resource_label(&mode.record.id)
                    )],
                    true,
                )
                .with_note("Digest-bound active-mode payload; loaded only on context-bearing lifecycle events."),
            );
        }
    }

    let mut injection_digests = BTreeMap::new();
    for harness in crate::terminal_adapters::descriptors() {
        let paths = execution_paths_for_harness(profile, composition_plan, harness)?;
        if paths.is_empty() {
            continue;
        }
        let ordered = by_harness.get(&harness).ok_or_else(|| {
            RenderError::projection(
                "lifecycle injection",
                format!("selected path harness {harness} has no ordered policy families"),
            )
        })?;
        let provider_binding = crate::terminal_adapters::for_selection(paths[0], composition_plan)
            .map_err(|error| {
                RenderError::invalid_input_error("execution-path terminal adapter", error)
            })?
            .0;
        let injection_path = harness_injection_path(&provider_binding);
        let injection = injection_content(
            profile,
            repo,
            ordered,
            semantics,
            identity,
            &injection_path,
            harness.projection_layout().skills_root(),
        )?;
        let digest = ArtifactDigest::hash_bytes(injection.as_bytes());
        surfaces.push(
            GeneratedSurface::whole_file(
                injection_path,
                SurfaceKind::Injection,
                injection,
                execution_path_ids(&paths),
                family_ids(ordered),
                true,
            )
            .with_note(
                "Native-harness lifecycle injection shared by exact neutral path selections.",
            ),
        );
        injection_digests.insert(harness, digest);
    }

    let mut behaviour_guidance = BTreeMap::new();
    for behaviour in &profile.lifecycle {
        let Some(guidance) = &behaviour.guidance else {
            continue;
        };
        let behaviour_id = &behaviour.id;
        let path = format!(".pale/lifecycle/{behaviour_id}.md");
        behaviour_guidance.insert(behaviour_id.clone(), artifact_ref(&path, guidance));
        surfaces.push(
            GeneratedSurface::whole_file(
                path,
                SurfaceKind::LifecycleGuidance,
                guidance.clone(),
                all_execution_path_ids(profile),
                vec![format!("lifecycle:{behaviour_id}")],
                true,
            )
            .with_note("Digest-bound guidance loaded only when this lifecycle behavior fires."),
        );
    }

    Ok(LifecycleArtifacts {
        injection_digests,
        run_mode_bodies,
        behaviour_guidance,
    })
}
