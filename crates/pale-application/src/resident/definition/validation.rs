//! Admission-time graph, body, and relationship validation.

use pale_core::records::{
    CompositionEdgeV1, CompositionGraphKindV1, EXPERIENCE_COMPOSITION_PLAN_V1_SCHEMA_ID,
    EXPERIENCE_COMPOSITION_PLAN_V1_SCHEMA_VERSION,
};

use super::lower::ResidentDefinitionDraft;
use super::types::{
    relationship_is_valid, PendingResidentEdge, ResidentCompositionPlanDefinition,
    ResidentDefinition, ResidentEdgeEvidence, ResidentNodeKey, ResidentNodeKind,
};
use super::{
    DefinitionGraph, DefinitionStoreError, ResidentDefinitionStore, ResidentRelationshipFailure,
};

impl ResidentDefinitionStore {
    pub(super) fn validate_child_coordinates(&self) -> Result<(), DefinitionStoreError> {
        for index in 0..self.graph.node_count() {
            let handle = self
                .graph
                .node_at(u32::try_from(index).map_err(|_| DefinitionStoreError::HandleSpace)?)?;
            let key = self.graph.node_key(handle)?;
            let body = self.resolve(handle)?;
            let valid = match (key, body) {
                (
                    ResidentNodeKey::Operation { operation, .. },
                    ResidentDefinition::Operation {
                        repository,
                        position,
                    },
                ) => match self.resolve(*repository)? {
                    ResidentDefinition::Repository(owner) => owner
                        .compiled
                        .semantic_plan()
                        .operations
                        .get(*position)
                        .is_some_and(|candidate| &candidate.record.id == operation),
                    _ => false,
                },
                (
                    ResidentNodeKey::RunMode { mode, .. },
                    ResidentDefinition::RunMode {
                        repository,
                        position,
                    },
                ) => match self.resolve(*repository)? {
                    ResidentDefinition::Repository(owner) => owner
                        .compiled
                        .semantic_plan()
                        .run_modes
                        .get(*position)
                        .is_some_and(|candidate| &candidate.record.id == mode),
                    _ => false,
                },
                (
                    ResidentNodeKey::Lifecycle(id),
                    ResidentDefinition::Lifecycle { profile, position },
                ) => match self.resolve(*profile)? {
                    ResidentDefinition::Profile(owner) => owner
                        .lifecycle
                        .get(*position)
                        .is_some_and(|candidate| &candidate.id == id),
                    _ => false,
                },
                (
                    ResidentNodeKey::ExecutionPathSelection(id),
                    ResidentDefinition::ExecutionPathSelection { profile, position },
                ) => match self.resolve(*profile)? {
                    ResidentDefinition::Profile(owner) => owner
                        .execution_paths
                        .get(*position)
                        .is_some_and(|candidate| &candidate.id == id),
                    _ => false,
                },
                (
                    ResidentNodeKey::CompositionPlan(_),
                    ResidentDefinition::CompositionPlan(plan),
                ) => self.composition_plan_is_complete(plan),
                _ => true,
            };
            if !valid {
                return Err(DefinitionStoreError::BodyIdentityMismatch {
                    key: Some(Box::new(key.clone())),
                });
            }
        }
        Ok(())
    }

    fn composition_plan_is_complete(&self, plan: &ResidentCompositionPlanDefinition) -> bool {
        let Ok(schema) =
            pale_contracts::record_schema_identity(EXPERIENCE_COMPOSITION_PLAN_V1_SCHEMA_ID)
        else {
            return false;
        };
        let graph_kinds = plan
            .graph_views
            .iter()
            .map(|view| view.kind)
            .collect::<Vec<_>>();
        let participants_exist = plan
            .participant_provider_manifests
            .iter()
            .flat_map(|manifest| &manifest.participants)
            .chain(plan.graph_views.iter().flat_map(|view| &view.nodes))
            .all(|participant| {
                self.graph
                    .find(&ResidentNodeKey::Participant(participant.clone()))
                    .is_some()
            });
        let non_operational_edges_are_complete = plan.non_operational_edges.iter().all(|edge| {
            matches!(
                edge,
                CompositionEdgeV1::UiEmbedding { .. } | CompositionEdgeV1::PluginDependency { .. }
            ) && {
                let (owner, dependency) = edge.endpoints();
                self.graph
                    .find(&ResidentNodeKey::Participant(owner.clone()))
                    .is_some()
                    && self
                        .graph
                        .find(&ResidentNodeKey::Participant(dependency.clone()))
                        .is_some()
            }
        });
        let paths_exist = plan.reason_paths.iter().all(|reason| {
            self.graph
                .find(&ResidentNodeKey::ExperiencePath(reason.path_id.clone()))
                .is_some()
        });
        plan.schema == schema.schema
            && plan.schema_sha256 == schema.schema_sha256
            && plan.schema_version == EXPERIENCE_COMPOSITION_PLAN_V1_SCHEMA_VERSION
            && graph_kinds == CompositionGraphKindV1::supported()
            && participants_exist
            && non_operational_edges_are_complete
            && paths_exist
    }
}

fn compare_edges(left: &PendingResidentEdge, right: &PendingResidentEdge) -> std::cmp::Ordering {
    (&left.source, &left.target, &left.relation)
        .cmp(&(&right.source, &right.target, &right.relation))
        .then_with(|| left.evidence.cmp_canonical(&right.evidence))
}

pub(super) fn canonicalize_edges(
    edges: &mut Vec<PendingResidentEdge>,
) -> Result<(), DefinitionStoreError> {
    edges.sort_by(compare_edges);
    edges.dedup_by(|right, left| {
        compare_edges(left, right).is_eq()
            && matches!(left.evidence, ResidentEdgeEvidence::None)
            && matches!(right.evidence, ResidentEdgeEvidence::None)
    });
    if let Some(duplicate) = edges
        .windows(2)
        .find(|pair| compare_edges(&pair[0], &pair[1]).is_eq())
        .map(|pair| &pair[0])
    {
        return Err(DefinitionStoreError::DuplicateRelationship(Box::new(
            ResidentRelationshipFailure {
                owner: duplicate.source.clone(),
                dependency: duplicate.target.clone(),
                relation: duplicate.relation,
                evidence: duplicate.evidence.clone(),
            },
        )));
    }
    Ok(())
}

pub(super) fn validate_relationships(
    definitions: &[(ResidentNodeKey, ResidentNodeKind, ResidentDefinitionDraft)],
    edges: &[PendingResidentEdge],
) -> Result<(), DefinitionStoreError> {
    let kinds = definitions
        .iter()
        .map(|(key, kind, _)| (key, *kind))
        .collect::<std::collections::BTreeMap<_, _>>();
    for edge in edges {
        let (Some(&source_kind), Some(&target_kind)) =
            (kinds.get(&edge.source), kinds.get(&edge.target))
        else {
            continue;
        };
        if !relationship_is_valid(edge, source_kind, target_kind) {
            return Err(DefinitionStoreError::InvalidRelationship(Box::new(
                ResidentRelationshipFailure {
                    owner: edge.source.clone(),
                    dependency: edge.target.clone(),
                    relation: edge.relation,
                    evidence: edge.evidence.clone(),
                },
            )));
        }
    }
    Ok(())
}

pub(super) fn body_matches_key(
    graph: &DefinitionGraph,
    key: &ResidentNodeKey,
    body: &ResidentDefinition,
) -> Result<bool, DefinitionStoreError> {
    let matches = match (key, body) {
        (ResidentNodeKey::Repository(id), ResidentDefinition::Repository(definition)) => {
            id == definition.compiled.id()
                && id == &definition.materialization.receipt().record().repository_id
        }
        (ResidentNodeKey::Profile(digest), ResidentDefinition::Profile(profile)) => {
            digest == &profile.profile_digest
        }
        (
            ResidentNodeKey::ProjectInstance(digest),
            ResidentDefinition::ProjectInstance(instance),
        ) => digest == &instance.instance_digest,
        (ResidentNodeKey::Participant(_), ResidentDefinition::Participant)
        | (ResidentNodeKey::CompiledContent(_), ResidentDefinition::CompiledContent)
        | (ResidentNodeKey::SourceContent(_), ResidentDefinition::SourceContent)
        | (ResidentNodeKey::WorkspaceCausal(_), ResidentDefinition::WorkspaceCausal)
        | (ResidentNodeKey::PrincipalUniverse(_), ResidentDefinition::PrincipalUniverse) => true,
        (ResidentNodeKey::PackageResource(_), ResidentDefinition::PackageResource(definition)) => {
            definition.custody_is_complete()
        }
        (ResidentNodeKey::PackageClosure(_), ResidentDefinition::PackageClosure(closure)) => {
            closure
                .global_seeds
                .iter()
                .chain(closure.repository_seeds.values().flatten())
                .all(|seed| {
                    graph
                        .find(&ResidentNodeKey::PackageResource(seed.clone()))
                        .is_some()
                })
        }
        (
            ResidentNodeKey::WorkspaceSemanticIr(digest),
            ResidentDefinition::WorkspaceSemanticIr(definition),
        ) => digest == definition.digest() && definition.custody_is_complete(),
        (
            ResidentNodeKey::WorkspaceAdmissionEvidence(digest),
            ResidentDefinition::WorkspaceAdmissionEvidence(evidence),
        ) => digest == evidence.digest() && evidence.custody_is_complete()?,
        (
            ResidentNodeKey::CompositionPlan(digest),
            ResidentDefinition::CompositionPlan(definition),
        ) => digest == &definition.plan_digest,
        (
            ResidentNodeKey::WorkspaceDeclaration {
                owner_module,
                declaration,
            },
            ResidentDefinition::WorkspaceDeclaration(definition),
        ) => {
            owner_module == definition.owner_module() && declaration == definition.declaration_ref()
        }
        (ResidentNodeKey::Responsibility(id), ResidentDefinition::Responsibility(definition)) => {
            id == &definition.id
        }
        (ResidentNodeKey::ProviderBinding(id), ResidentDefinition::ProviderBinding(definition)) => {
            id == &definition.id
        }
        (ResidentNodeKey::ExperiencePath(id), ResidentDefinition::ExperiencePath(definition)) => {
            id == &definition.id
        }
        (ResidentNodeKey::EntryContract(id), ResidentDefinition::EntryContract(definition)) => {
            id == &definition.id
        }
        (
            ResidentNodeKey::RequestTranslation(id),
            ResidentDefinition::RequestTranslation(definition),
        ) => id == &definition.id,
        (
            ResidentNodeKey::ResultTranslation(id),
            ResidentDefinition::ResultTranslation(definition),
        ) => id == &definition.id,
        (
            ResidentNodeKey::TerminationContract(id),
            ResidentDefinition::TerminationContract(definition),
        ) => id == &definition.id,
        (
            ResidentNodeKey::SettlementContract(id),
            ResidentDefinition::SettlementContract(definition),
        ) => id == &definition.id,
        (
            ResidentNodeKey::ContinuityContract(id),
            ResidentDefinition::ContinuityContract(definition),
        ) => id == &definition.id,
        (
            ResidentNodeKey::Operation { repository, .. },
            ResidentDefinition::Operation {
                repository: owner,
                position: _,
            },
        )
        | (
            ResidentNodeKey::RunMode { repository, .. },
            ResidentDefinition::RunMode {
                repository: owner,
                position: _,
            },
        ) => matches!(
            graph.node_key(*owner)?,
            ResidentNodeKey::Repository(owner_id) if owner_id == repository
        ),
        (
            ResidentNodeKey::Lifecycle(_),
            ResidentDefinition::Lifecycle {
                profile,
                position: _,
            },
        ) => matches!(graph.node_key(*profile)?, ResidentNodeKey::Profile(_)),
        (
            ResidentNodeKey::ExecutionPathSelection(_),
            ResidentDefinition::ExecutionPathSelection {
                profile,
                position: _,
            },
        ) => matches!(graph.node_key(*profile)?, ResidentNodeKey::Profile(_)),
        _ => false,
    };
    Ok(matches)
}
