//! Workspace bodies, declaration dependencies, and causal-fact lowering.

use std::collections::BTreeMap;

use pale_core::records::ExperienceCompositionDeclarationsV1;

use crate::workspace::{ResidentWorkspaceDerivation, ResidentWorkspaceParts};

use super::{insert_unique, lower_workspace_causality, pending_edge, ResidentDefinitionDraft};
use crate::resident::definition::types::{
    PendingResidentEdge, ResidentDefinition, ResidentEdgeEvidence, ResidentNodeKey,
    ResidentNodeKind, ResidentRelation,
};
use crate::resident::definition::DefinitionStoreError;

pub(super) fn lower(
    workspace: ResidentWorkspaceParts,
    definitions: &mut BTreeMap<ResidentNodeKey, (ResidentNodeKind, ResidentDefinitionDraft)>,
    edges: &mut Vec<PendingResidentEdge>,
) -> Result<
    (
        ResidentWorkspaceDerivation,
        ExperienceCompositionDeclarationsV1,
    ),
    DefinitionStoreError,
> {
    let crate::workspace::ResidentWorkspaceParts {
        principal_digest,
        admission_evidence,
        derivation,
        declarations: workspace_declarations,
        composition,
        definition: workspace_definition,
    } = workspace;
    let semantic_key = ResidentNodeKey::WorkspaceSemanticIr(workspace_definition.digest().clone());
    let evidence_key =
        ResidentNodeKey::WorkspaceAdmissionEvidence(admission_evidence.digest().clone());
    let principal_key = ResidentNodeKey::PrincipalUniverse(principal_digest);
    for declaration in workspace_declarations {
        let key = ResidentNodeKey::WorkspaceDeclaration {
            owner_module: declaration.owner_module().clone(),
            declaration: declaration.declaration_ref().clone(),
        };
        for dependency in declaration.resident_dependencies() {
            let (relation, evidence) = match dependency.kind {
                crate::workspace::ResidentWorkspaceDependencyKind::Explicit(kind) => (
                    ResidentRelation::WorkspaceExplicitDependency,
                    ResidentEdgeEvidence::WorkspaceDependency(kind),
                ),
                crate::workspace::ResidentWorkspaceDependencyKind::RelationshipFrom => (
                    ResidentRelation::WorkspaceRelationshipFrom,
                    ResidentEdgeEvidence::None,
                ),
                crate::workspace::ResidentWorkspaceDependencyKind::RelationshipTo => (
                    ResidentRelation::WorkspaceRelationshipTo,
                    ResidentEdgeEvidence::None,
                ),
            };
            edges.push(PendingResidentEdge {
                source: key.clone(),
                target: ResidentNodeKey::WorkspaceDeclaration {
                    owner_module: dependency.owner_module,
                    declaration: dependency.declaration,
                },
                relation,
                evidence,
            });
        }
        insert_unique(
            definitions,
            key,
            ResidentNodeKind::WorkspaceDeclaration,
            ResidentDefinitionDraft::Complete(ResidentDefinition::WorkspaceDeclaration(Box::new(
                declaration,
            ))),
        )?;
    }
    lower_workspace_causality(&derivation, definitions, edges)?;
    for node in derivation.causal_nodes() {
        pending_edge(
            edges,
            semantic_key.clone(),
            ResidentNodeKey::from_workspace_subject(&node.subject),
            ResidentRelation::WorkspaceSemanticIrSource,
        );
    }
    insert_unique(
        definitions,
        semantic_key.clone(),
        ResidentNodeKind::WorkspaceSemanticIr,
        ResidentDefinitionDraft::Complete(ResidentDefinition::WorkspaceSemanticIr(Box::new(
            workspace_definition,
        ))),
    )?;
    insert_unique(
        definitions,
        evidence_key.clone(),
        ResidentNodeKind::WorkspaceAdmissionEvidence,
        ResidentDefinitionDraft::Complete(ResidentDefinition::WorkspaceAdmissionEvidence(
            Box::new(admission_evidence),
        )),
    )?;
    pending_edge(
        edges,
        principal_key.clone(),
        semantic_key,
        ResidentRelation::PrincipalUniverseSemanticIr,
    );
    pending_edge(
        edges,
        principal_key.clone(),
        evidence_key,
        ResidentRelation::PrincipalUniverseAdmissionEvidence,
    );
    insert_unique(
        definitions,
        principal_key,
        ResidentNodeKind::PrincipalUniverse,
        ResidentDefinitionDraft::Complete(ResidentDefinition::PrincipalUniverse),
    )?;

    Ok((derivation, composition))
}
