//! Consuming lowering from one-shot compiler bodies into resident definitions.

use std::collections::BTreeMap;

mod composition;
mod package;
mod profile;
mod repositories;
mod workspace;

use crate::compile::ResidentCompilationParts;

use super::super::generation::AppliedRepositoryMaterialization;
use super::types::{
    PendingResidentEdge, ResidentDefinition, ResidentEdgeEvidence, ResidentNodeKey,
    ResidentNodeKind, ResidentRelation,
};
use super::DefinitionStoreError;

pub(super) struct LoweredDefinitions {
    pub(super) definitions: Vec<(ResidentNodeKey, ResidentNodeKind, ResidentDefinitionDraft)>,
    pub(super) edges: Vec<PendingResidentEdge>,
    pub(super) derivation: crate::workspace::ResidentWorkspaceDerivation,
}

pub(super) enum ResidentDefinitionDraft {
    Complete(ResidentDefinition),
    Operation {
        repository: ResidentNodeKey,
        position: usize,
    },
    RunMode {
        repository: ResidentNodeKey,
        position: usize,
    },
    Lifecycle {
        profile: ResidentNodeKey,
        position: usize,
    },
    ExecutionPathSelection {
        profile: ResidentNodeKey,
        position: usize,
    },
}

pub(super) fn lower(
    parts: ResidentCompilationParts,
    materializations: Vec<AppliedRepositoryMaterialization>,
) -> Result<LoweredDefinitions, DefinitionStoreError> {
    let mut definitions = BTreeMap::new();
    let mut edges = Vec::new();

    let ResidentCompilationParts {
        profile,
        instance,
        package_semantic_ir,
        workspace,
        experience_composition_plan,
        repositories,
    } = parts;

    let (repository_scopes, package_closure_key) =
        package::lower(package_semantic_ir, &mut definitions, &mut edges)?;

    let (derivation, composition) = workspace::lower(workspace, &mut definitions, &mut edges)?;
    composition::lower(
        composition,
        experience_composition_plan,
        &mut definitions,
        &mut edges,
    )?;
    let (profile_key, instance_key) = profile::lower(
        profile,
        instance,
        &package_closure_key,
        &mut definitions,
        &mut edges,
    )?;

    repositories::lower(
        repositories::RepositoryLoweringInputs {
            repositories,
            materializations,
            scopes: repository_scopes,
            package_closure: package_closure_key,
            profile: profile_key,
            instance: instance_key,
        },
        &mut definitions,
        &mut edges,
    )?;

    Ok(LoweredDefinitions {
        definitions: definitions
            .into_iter()
            .map(|(key, (kind, body))| (key, kind, body))
            .collect(),
        edges,
        derivation,
    })
}

fn insert_unique(
    definitions: &mut BTreeMap<ResidentNodeKey, (ResidentNodeKind, ResidentDefinitionDraft)>,
    key: ResidentNodeKey,
    kind: ResidentNodeKind,
    body: ResidentDefinitionDraft,
) -> Result<(), DefinitionStoreError> {
    if definitions.insert(key.clone(), (kind, body)).is_some() {
        return Err(DefinitionStoreError::DuplicateDefinition { key: Box::new(key) });
    }
    Ok(())
}

fn insert_idempotent(
    definitions: &mut BTreeMap<ResidentNodeKey, (ResidentNodeKind, ResidentDefinitionDraft)>,
    key: ResidentNodeKey,
    kind: ResidentNodeKind,
    body: ResidentDefinitionDraft,
) {
    definitions.entry(key).or_insert((kind, body));
}

fn pending_edge(
    edges: &mut Vec<PendingResidentEdge>,
    source: ResidentNodeKey,
    target: ResidentNodeKey,
    relation: ResidentRelation,
) {
    edges.push(PendingResidentEdge {
        source,
        target,
        relation,
        evidence: ResidentEdgeEvidence::None,
    });
}

fn lower_workspace_causality(
    derivation: &crate::workspace::ResidentWorkspaceDerivation,
    definitions: &mut BTreeMap<ResidentNodeKey, (ResidentNodeKind, ResidentDefinitionDraft)>,
    edges: &mut Vec<PendingResidentEdge>,
) -> Result<(), DefinitionStoreError> {
    let nodes = derivation.causal_nodes();
    let subjects = nodes
        .iter()
        .map(|node| (node.node_digest.clone(), node.subject.clone()))
        .collect::<BTreeMap<_, _>>();
    for node in nodes {
        let owner = ResidentNodeKey::from_workspace_subject(&node.subject);
        if matches!(
            node.subject,
            pale_core::records::WorkspaceDerivationSubjectV1::Declaration { .. }
        ) {
            if !definitions.contains_key(&owner) {
                return Err(DefinitionStoreError::MissingWorkspaceDeclaration {
                    key: Box::new(owner.clone()),
                });
            }
        } else {
            insert_unique(
                definitions,
                owner.clone(),
                ResidentNodeKind::WorkspaceCausal,
                ResidentDefinitionDraft::Complete(ResidentDefinition::WorkspaceCausal),
            )?;
        }
        for input in &node.inputs {
            let subject =
                subjects
                    .get(input)
                    .ok_or_else(|| DefinitionStoreError::MissingCausalInput {
                        owner: Box::new(owner.clone()),
                    })?;
            pending_edge(
                edges,
                owner.clone(),
                ResidentNodeKey::from_workspace_subject(subject),
                ResidentRelation::WorkspaceCausalInput(node.rule),
            );
        }
    }
    Ok(())
}
