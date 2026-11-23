//! Experience-composition lowering into typed resident nodes and edges.

use std::collections::{BTreeMap, BTreeSet};

use pale_core::records::{ExperienceCompositionDeclarationsV1, ExperienceCompositionPlanV1};

mod contracts;
mod operational;
mod paths;

use super::{insert_unique, ResidentDefinitionDraft};
use crate::resident::definition::types::{
    PendingResidentEdge, ResidentCompositionPlanDefinition, ResidentDefinition, ResidentNodeKey,
    ResidentNodeKind,
};
use crate::resident::definition::DefinitionStoreError;

pub(super) fn lower(
    mut composition: ExperienceCompositionDeclarationsV1,
    experience_composition_plan: ExperienceCompositionPlanV1,
    definitions: &mut BTreeMap<ResidentNodeKey, (ResidentNodeKind, ResidentDefinitionDraft)>,
    edges: &mut Vec<PendingResidentEdge>,
) -> Result<(), DefinitionStoreError> {
    crate::composition::normalize::normalize_source(&mut composition).map_err(|source| {
        DefinitionStoreError::CompositionNormalization {
            source: Box::new(source),
        }
    })?;
    if composition != experience_composition_plan.declarations {
        return Err(DefinitionStoreError::CompositionSourceMismatch);
    }
    let ExperienceCompositionPlanV1 {
        schema,
        schema_sha256,
        schema_version,
        mut declarations,
        provider_compatibility_graph,
        delegation_graph,
        ui_embedding_graph,
        plugin_dependency_graph,
        native_application_graph,
        terminal_leaf_graph,
        evidence_settlement_graph,
        reason_paths,
        plan_digest,
    } = experience_composition_plan;
    let composition_plan_key = ResidentNodeKey::CompositionPlan(plan_digest.clone());
    let participant_provider_manifests =
        std::mem::take(&mut declarations.participant_provider_manifests);
    let mut participants = BTreeSet::new();
    for manifest in &participant_provider_manifests {
        participants.extend(manifest.participants.iter().cloned());
    }
    contracts::lower(&mut declarations, &mut participants, definitions, edges)?;

    paths::lower(&mut declarations, definitions, edges)?;

    let non_operational_edges = operational::lower(
        &mut declarations,
        participants,
        &composition_plan_key,
        definitions,
        edges,
    );

    insert_unique(
        definitions,
        composition_plan_key,
        ResidentNodeKind::CompositionPlan,
        ResidentDefinitionDraft::Complete(ResidentDefinition::CompositionPlan(Box::new(
            ResidentCompositionPlanDefinition {
                schema,
                schema_sha256,
                schema_version,
                participant_provider_manifests,
                non_operational_edges,
                graph_views: vec![
                    provider_compatibility_graph,
                    delegation_graph,
                    ui_embedding_graph,
                    plugin_dependency_graph,
                    native_application_graph,
                    terminal_leaf_graph,
                    evidence_settlement_graph,
                ],
                reason_paths,
                plan_digest,
            },
        ))),
    )?;

    Ok(())
}
