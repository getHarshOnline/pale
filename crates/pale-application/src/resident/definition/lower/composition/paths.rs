//! Selected experience-path definitions.

use std::collections::BTreeMap;

use pale_core::records::{ExperienceCompositionDeclarationsV1, ExperiencePathV1};

use super::super::{insert_idempotent, insert_unique, pending_edge, ResidentDefinitionDraft};
use crate::resident::definition::types::{
    PendingResidentEdge, ResidentCompiledContentKey, ResidentDefinition, ResidentNodeKey,
    ResidentNodeKind, ResidentRelation, ResidentSourceContentKey,
};
use crate::resident::definition::DefinitionStoreError;

pub(super) fn lower(
    declarations: &mut ExperienceCompositionDeclarationsV1,
    definitions: &mut BTreeMap<ResidentNodeKey, (ResidentNodeKind, ResidentDefinitionDraft)>,
    edges: &mut Vec<PendingResidentEdge>,
) -> Result<(), DefinitionStoreError> {
    for path in std::mem::take(&mut declarations.selected_experience_paths) {
        lower_path(path, definitions, edges)?;
    }
    Ok(())
}

fn lower_path(
    path: ExperiencePathV1,
    definitions: &mut BTreeMap<ResidentNodeKey, (ResidentNodeKind, ResidentDefinitionDraft)>,
    edges: &mut Vec<PendingResidentEdge>,
) -> Result<(), DefinitionStoreError> {
    let key = ResidentNodeKey::ExperiencePath(path.id.clone());
    for (dependency, relation) in [
        (
            ResidentNodeKey::EntryContract(path.entry_contract_id.clone()),
            ResidentRelation::PathEntryContract,
        ),
        (
            ResidentNodeKey::TerminationContract(path.termination_contract_id.clone()),
            ResidentRelation::PathTerminationContract,
        ),
        (
            ResidentNodeKey::SettlementContract(path.settlement_contract_id.clone()),
            ResidentRelation::PathSettlementContract,
        ),
        (
            ResidentNodeKey::ContinuityContract(path.continuity_contract_id.clone()),
            ResidentRelation::PathContinuityContract,
        ),
    ] {
        pending_edge(edges, key.clone(), dependency, relation);
    }
    let compiled_content = ResidentNodeKey::CompiledContent(ResidentCompiledContentKey(
        path.compiled_content.compiled_content_identity.clone(),
    ));
    insert_idempotent(
        definitions,
        compiled_content.clone(),
        ResidentNodeKind::CompiledContent,
        ResidentDefinitionDraft::Complete(ResidentDefinition::CompiledContent),
    );
    pending_edge(
        edges,
        key.clone(),
        compiled_content,
        ResidentRelation::PathCompiledContent,
    );
    let source_content = ResidentNodeKey::SourceContent(ResidentSourceContentKey(
        path.compiled_content.source_content_identity.clone(),
    ));
    insert_idempotent(
        definitions,
        source_content.clone(),
        ResidentNodeKind::SourceContent,
        ResidentDefinitionDraft::Complete(ResidentDefinition::SourceContent),
    );
    pending_edge(
        edges,
        key.clone(),
        source_content,
        ResidentRelation::PathSourceContent,
    );
    for responsibility in &path.responsibility_contract_ids {
        pending_edge(
            edges,
            key.clone(),
            ResidentNodeKey::Responsibility(responsibility.clone()),
            ResidentRelation::PathResponsibility,
        );
    }
    for provider in &path.provider_binding_ids {
        pending_edge(
            edges,
            key.clone(),
            ResidentNodeKey::ProviderBinding(provider.clone()),
            ResidentRelation::PathProvider,
        );
    }
    insert_unique(
        definitions,
        ResidentNodeKey::ExperiencePath(path.id.clone()),
        ResidentNodeKind::ExperiencePath,
        ResidentDefinitionDraft::Complete(ResidentDefinition::ExperiencePath(Box::new(path))),
    )
}
