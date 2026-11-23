//! Composition contract definitions and their direct typed dependencies.

use std::collections::{BTreeMap, BTreeSet};

use pale_core::records::{ExperienceCompositionDeclarationsV1, ParticipantRefV1};

use super::super::{insert_unique, pending_edge, ResidentDefinitionDraft};
use crate::resident::definition::types::{
    PendingResidentEdge, ResidentDefinition, ResidentNodeKey, ResidentNodeKind, ResidentRelation,
};
use crate::resident::definition::DefinitionStoreError;

pub(super) fn lower(
    declarations: &mut ExperienceCompositionDeclarationsV1,
    participants: &mut BTreeSet<ParticipantRefV1>,
    definitions: &mut BTreeMap<ResidentNodeKey, (ResidentNodeKind, ResidentDefinitionDraft)>,
    edges: &mut Vec<PendingResidentEdge>,
) -> Result<(), DefinitionStoreError> {
    for responsibility in std::mem::take(&mut declarations.responsibility_contracts) {
        let key = ResidentNodeKey::Responsibility(responsibility.id.clone());
        participants.insert(responsibility.participant.clone());
        pending_edge(
            edges,
            key.clone(),
            ResidentNodeKey::Participant(responsibility.participant.clone()),
            ResidentRelation::ResponsibilityParticipant,
        );
        insert_unique(
            definitions,
            key,
            ResidentNodeKind::Responsibility,
            ResidentDefinitionDraft::Complete(ResidentDefinition::Responsibility(Box::new(
                responsibility,
            ))),
        )?;
    }
    for binding in std::mem::take(&mut declarations.provider_bindings) {
        let key = ResidentNodeKey::ProviderBinding(binding.id.clone());
        participants.insert(binding.participant.clone());
        pending_edge(
            edges,
            key.clone(),
            ResidentNodeKey::Participant(binding.participant.clone()),
            ResidentRelation::ProviderParticipant,
        );
        for responsibility in &binding.responsibility_contract_ids {
            pending_edge(
                edges,
                key.clone(),
                ResidentNodeKey::Responsibility(responsibility.clone()),
                ResidentRelation::ProviderResponsibility,
            );
        }
        insert_unique(
            definitions,
            key,
            ResidentNodeKind::ProviderBinding,
            ResidentDefinitionDraft::Complete(ResidentDefinition::ProviderBinding(Box::new(
                binding,
            ))),
        )?;
    }
    lower_translation_contracts(declarations, participants, definitions, edges)?;
    lower_boundary_contracts(declarations, participants, definitions, edges)
}

fn lower_translation_contracts(
    declarations: &mut ExperienceCompositionDeclarationsV1,
    participants: &mut BTreeSet<ParticipantRefV1>,
    definitions: &mut BTreeMap<ResidentNodeKey, (ResidentNodeKind, ResidentDefinitionDraft)>,
    edges: &mut Vec<PendingResidentEdge>,
) -> Result<(), DefinitionStoreError> {
    for translation in std::mem::take(&mut declarations.request_translations) {
        let key = ResidentNodeKey::RequestTranslation(translation.id.clone());
        participants.extend([translation.from.clone(), translation.to.clone()]);
        pending_edge(
            edges,
            key.clone(),
            ResidentNodeKey::Participant(translation.from.clone()),
            ResidentRelation::RequestTranslationSource,
        );
        pending_edge(
            edges,
            key.clone(),
            ResidentNodeKey::Participant(translation.to.clone()),
            ResidentRelation::RequestTranslationTarget,
        );
        insert_unique(
            definitions,
            key,
            ResidentNodeKind::RequestTranslation,
            ResidentDefinitionDraft::Complete(ResidentDefinition::RequestTranslation(Box::new(
                translation,
            ))),
        )?;
    }
    for translation in std::mem::take(&mut declarations.result_translations) {
        let key = ResidentNodeKey::ResultTranslation(translation.id.clone());
        participants.extend([translation.from.clone(), translation.to.clone()]);
        pending_edge(
            edges,
            key.clone(),
            ResidentNodeKey::Participant(translation.from.clone()),
            ResidentRelation::ResultTranslationSource,
        );
        pending_edge(
            edges,
            key.clone(),
            ResidentNodeKey::Participant(translation.to.clone()),
            ResidentRelation::ResultTranslationTarget,
        );
        insert_unique(
            definitions,
            key,
            ResidentNodeKind::ResultTranslation,
            ResidentDefinitionDraft::Complete(ResidentDefinition::ResultTranslation(Box::new(
                translation,
            ))),
        )?;
    }
    Ok(())
}

fn lower_boundary_contracts(
    declarations: &mut ExperienceCompositionDeclarationsV1,
    participants: &mut BTreeSet<ParticipantRefV1>,
    definitions: &mut BTreeMap<ResidentNodeKey, (ResidentNodeKind, ResidentDefinitionDraft)>,
    edges: &mut Vec<PendingResidentEdge>,
) -> Result<(), DefinitionStoreError> {
    for contract in std::mem::take(&mut declarations.entry_contracts) {
        let key = ResidentNodeKey::EntryContract(contract.id.clone());
        participants.insert(contract.entry.clone());
        pending_edge(
            edges,
            key.clone(),
            ResidentNodeKey::Participant(contract.entry.clone()),
            ResidentRelation::EntryParticipant,
        );
        pending_edge(
            edges,
            key.clone(),
            ResidentNodeKey::Responsibility(contract.responsibility_contract_id.clone()),
            ResidentRelation::EntryResponsibility,
        );
        insert_unique(
            definitions,
            key,
            ResidentNodeKind::EntryContract,
            ResidentDefinitionDraft::Complete(ResidentDefinition::EntryContract(Box::new(
                contract,
            ))),
        )?;
    }
    for contract in std::mem::take(&mut declarations.termination_contracts) {
        let key = ResidentNodeKey::TerminationContract(contract.id.clone());
        participants.insert(contract.owner.clone());
        pending_edge(
            edges,
            key.clone(),
            ResidentNodeKey::Participant(contract.owner.clone()),
            ResidentRelation::TerminationOwner,
        );
        if let Some(terminal_leaf) = &contract.terminal_leaf {
            participants.insert(terminal_leaf.clone());
            pending_edge(
                edges,
                key.clone(),
                ResidentNodeKey::Participant(terminal_leaf.clone()),
                ResidentRelation::TerminationLeaf,
            );
        }
        insert_unique(
            definitions,
            key,
            ResidentNodeKind::TerminationContract,
            ResidentDefinitionDraft::Complete(ResidentDefinition::TerminationContract(Box::new(
                contract,
            ))),
        )?;
    }
    for contract in std::mem::take(&mut declarations.settlement_contracts) {
        let key = ResidentNodeKey::SettlementContract(contract.id.clone());
        participants.insert(contract.observer.clone());
        pending_edge(
            edges,
            key.clone(),
            ResidentNodeKey::Participant(contract.observer.clone()),
            ResidentRelation::SettlementObserver,
        );
        insert_unique(
            definitions,
            key,
            ResidentNodeKind::SettlementContract,
            ResidentDefinitionDraft::Complete(ResidentDefinition::SettlementContract(Box::new(
                contract,
            ))),
        )?;
    }
    for contract in std::mem::take(&mut declarations.continuity_contracts) {
        let key = ResidentNodeKey::ContinuityContract(contract.id.clone());
        participants.insert(contract.owner.clone());
        pending_edge(
            edges,
            key.clone(),
            ResidentNodeKey::Participant(contract.owner.clone()),
            ResidentRelation::ContinuityOwner,
        );
        insert_unique(
            definitions,
            key,
            ResidentNodeKind::ContinuityContract,
            ResidentDefinitionDraft::Complete(ResidentDefinition::ContinuityContract(Box::new(
                contract,
            ))),
        )?;
    }
    Ok(())
}
