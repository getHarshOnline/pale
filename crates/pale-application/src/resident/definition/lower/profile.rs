//! Profile, project-instance, execution-path, and lifecycle lowering.

use std::collections::BTreeMap;

use pale_core::records::{EffectiveProjectProfileV1, ResolvedProjectInstanceV1};

use super::{insert_unique, pending_edge, ResidentDefinitionDraft};
use crate::resident::definition::types::{
    PendingResidentEdge, ResidentDefinition, ResidentNodeKey, ResidentNodeKind, ResidentRelation,
};
use crate::resident::definition::DefinitionStoreError;

pub(super) fn lower(
    profile: EffectiveProjectProfileV1,
    instance: ResolvedProjectInstanceV1,
    package_closure_key: &ResidentNodeKey,
    definitions: &mut BTreeMap<ResidentNodeKey, (ResidentNodeKind, ResidentDefinitionDraft)>,
    edges: &mut Vec<PendingResidentEdge>,
) -> Result<(ResidentNodeKey, ResidentNodeKey), DefinitionStoreError> {
    let instance_key = ResidentNodeKey::ProjectInstance(instance.instance_digest.clone());
    insert_unique(
        definitions,
        instance_key.clone(),
        ResidentNodeKind::ProjectInstance,
        ResidentDefinitionDraft::Complete(ResidentDefinition::ProjectInstance(Box::new(instance))),
    )?;

    let profile_key = ResidentNodeKey::Profile(profile.profile_digest.clone());
    for (position, selection) in profile.execution_paths.iter().enumerate() {
        let key = ResidentNodeKey::ExecutionPathSelection(selection.id.clone());
        insert_unique(
            definitions,
            key.clone(),
            ResidentNodeKind::ExecutionPathSelection,
            ResidentDefinitionDraft::ExecutionPathSelection {
                profile: profile_key.clone(),
                position,
            },
        )?;
        pending_edge(
            edges,
            key.clone(),
            profile_key.clone(),
            ResidentRelation::ExecutionPathSelectionProfile,
        );
        pending_edge(
            edges,
            key,
            ResidentNodeKey::ExperiencePath(selection.experience_path_id.clone()),
            ResidentRelation::ExecutionPathSelectionExperiencePath,
        );
    }
    for (position, behavior) in profile.lifecycle.iter().enumerate() {
        let key = ResidentNodeKey::Lifecycle(behavior.id.clone());
        insert_unique(
            definitions,
            key.clone(),
            ResidentNodeKind::Lifecycle,
            ResidentDefinitionDraft::Lifecycle {
                profile: profile_key.clone(),
                position,
            },
        )?;
        pending_edge(
            edges,
            key,
            profile_key.clone(),
            ResidentRelation::LifecycleProfile,
        );
    }
    pending_edge(
        edges,
        instance_key.clone(),
        profile_key.clone(),
        ResidentRelation::ProjectInstanceProfile,
    );
    pending_edge(
        edges,
        profile_key.clone(),
        package_closure_key.clone(),
        ResidentRelation::ProfilePackageClosure,
    );
    insert_unique(
        definitions,
        profile_key.clone(),
        ResidentNodeKind::Profile,
        ResidentDefinitionDraft::Complete(ResidentDefinition::Profile(Box::new(profile))),
    )?;
    Ok((profile_key, instance_key))
}
