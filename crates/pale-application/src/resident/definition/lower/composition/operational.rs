//! Resident operational composition relationships and retained edge evidence.

use std::collections::{BTreeMap, BTreeSet};

use pale_core::records::{
    CompositionEdgeV1, ExperienceCompositionDeclarationsV1, ParticipantRefV1,
    WorkspaceDerivationSubjectV1,
};

use super::super::{insert_idempotent, pending_edge, ResidentDefinitionDraft};
use crate::resident::definition::types::{
    operational_relationship, PendingResidentEdge, ResidentDefinition, ResidentEdgeEvidence,
    ResidentNodeKey, ResidentNodeKind, ResidentRelation,
};

pub(super) fn lower(
    declarations: &mut ExperienceCompositionDeclarationsV1,
    mut participants: BTreeSet<ParticipantRefV1>,
    composition_plan_key: &ResidentNodeKey,
    definitions: &mut BTreeMap<ResidentNodeKey, (ResidentNodeKind, ResidentDefinitionDraft)>,
    edges: &mut Vec<PendingResidentEdge>,
) -> Vec<CompositionEdgeV1> {
    let mut non_operational_edges = Vec::new();
    for edge in std::mem::take(&mut declarations.edge_contracts) {
        if matches!(
            edge,
            CompositionEdgeV1::UiEmbedding { .. } | CompositionEdgeV1::PluginDependency { .. }
        ) {
            non_operational_edges.push(edge);
        } else {
            lower_edge(edge, &mut participants, edges);
        }
    }
    for participant in participants {
        insert_idempotent(
            definitions,
            ResidentNodeKey::Participant(participant),
            ResidentNodeKind::Participant,
            ResidentDefinitionDraft::Complete(ResidentDefinition::Participant),
        );
    }
    let composition_subject =
        ResidentNodeKey::WorkspaceCausal(WorkspaceDerivationSubjectV1::ExperienceComposition);
    let composition_keys = definitions
        .iter()
        .filter_map(|(key, (kind, _))| composition_kind(*kind).then_some(key.clone()))
        .collect::<Vec<_>>();
    for key in composition_keys {
        pending_edge(
            edges,
            key.clone(),
            composition_subject.clone(),
            ResidentRelation::CompositionSourceDependency,
        );
        pending_edge(
            edges,
            key,
            composition_plan_key.clone(),
            ResidentRelation::CompositionDefinitionPlan,
        );
    }
    non_operational_edges
}

const fn composition_kind(kind: ResidentNodeKind) -> bool {
    matches!(
        kind,
        ResidentNodeKind::Responsibility
            | ResidentNodeKind::Participant
            | ResidentNodeKind::ProviderBinding
            | ResidentNodeKind::ExperiencePath
            | ResidentNodeKind::EntryContract
            | ResidentNodeKind::RequestTranslation
            | ResidentNodeKind::ResultTranslation
            | ResidentNodeKind::TerminationContract
            | ResidentNodeKind::SettlementContract
            | ResidentNodeKind::ContinuityContract
            | ResidentNodeKind::CompiledContent
            | ResidentNodeKind::SourceContent
    )
}

fn lower_edge(
    edge: CompositionEdgeV1,
    participants: &mut BTreeSet<ParticipantRefV1>,
    edges: &mut Vec<PendingResidentEdge>,
) {
    lower_contract_dependencies(&edge, edges);
    let Some((source, target, relation)) = operational_relationship(&edge) else {
        return;
    };
    if let ResidentNodeKey::Participant(owner) = &source {
        participants.insert(owner.clone());
    }
    if let ResidentNodeKey::Participant(dependency) = &target {
        participants.insert(dependency.clone());
    }
    edges.push(PendingResidentEdge {
        source,
        target,
        relation,
        evidence: ResidentEdgeEvidence::OperationalContract(Box::new(edge)),
    });
}

fn lower_contract_dependencies(edge: &CompositionEdgeV1, edges: &mut Vec<PendingResidentEdge>) {
    match edge {
        CompositionEdgeV1::WorkDelegation {
            from,
            responsibility_contract_id,
            request_translation_id,
            result_translation_id,
            ..
        } => {
            let owner = ResidentNodeKey::Participant(from.clone());
            pending_edge(
                edges,
                owner.clone(),
                ResidentNodeKey::Responsibility(responsibility_contract_id.clone()),
                ResidentRelation::DelegationResponsibility,
            );
            pending_edge(
                edges,
                owner.clone(),
                ResidentNodeKey::RequestTranslation(request_translation_id.clone()),
                ResidentRelation::OperationalRequestTranslation,
            );
            pending_edge(
                edges,
                owner,
                ResidentNodeKey::ResultTranslation(result_translation_id.clone()),
                ResidentRelation::OperationalResultTranslation,
            );
        }
        CompositionEdgeV1::NativeApplicationCall {
            caller,
            provider_binding_id,
            request_translation_id,
            result_translation_id,
            ..
        } => {
            let owner = ResidentNodeKey::Participant(caller.clone());
            pending_edge(
                edges,
                owner.clone(),
                ResidentNodeKey::ProviderBinding(provider_binding_id.clone()),
                ResidentRelation::NativeCallProvider,
            );
            pending_edge(
                edges,
                owner.clone(),
                ResidentNodeKey::RequestTranslation(request_translation_id.clone()),
                ResidentRelation::OperationalRequestTranslation,
            );
            pending_edge(
                edges,
                owner,
                ResidentNodeKey::ResultTranslation(result_translation_id.clone()),
                ResidentRelation::OperationalResultTranslation,
            );
        }
        CompositionEdgeV1::TerminalLeaf {
            owner,
            termination_contract_id,
            ..
        } => pending_edge(
            edges,
            ResidentNodeKey::Participant(owner.clone()),
            ResidentNodeKey::TerminationContract(termination_contract_id.clone()),
            ResidentRelation::TerminalContract,
        ),
        CompositionEdgeV1::ObservationSettlement {
            observer,
            settlement_contract_id,
            ..
        } => pending_edge(
            edges,
            ResidentNodeKey::Participant(observer.clone()),
            ResidentNodeKey::SettlementContract(settlement_contract_id.clone()),
            ResidentRelation::ObservationSettlementContract,
        ),
        CompositionEdgeV1::UiEmbedding { .. } | CompositionEdgeV1::PluginDependency { .. } => {}
    }
}
