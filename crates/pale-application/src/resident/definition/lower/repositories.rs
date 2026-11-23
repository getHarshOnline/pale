//! Repository projections, child positions, and applied-evidence lowering.

use std::collections::BTreeMap;

use pale_core::records::LogicalRepositoryIdV1;

use crate::compile::CompiledRepo;
use crate::packages::RepositoryPackageSelection;

use super::{insert_unique, pending_edge, ResidentDefinitionDraft};
use crate::resident::definition::types::{
    PendingResidentEdge, ResidentDefinition, ResidentNodeKey, ResidentNodeKind, ResidentRelation,
    ResidentRepositoryDefinition,
};
use crate::resident::definition::DefinitionStoreError;
use crate::resident::generation::AppliedRepositoryMaterialization;

pub(super) struct RepositoryLoweringInputs {
    pub(super) repositories: Vec<CompiledRepo>,
    pub(super) materializations: Vec<AppliedRepositoryMaterialization>,
    pub(super) scopes: BTreeMap<LogicalRepositoryIdV1, RepositoryPackageSelection>,
    pub(super) package_closure: ResidentNodeKey,
    pub(super) profile: ResidentNodeKey,
    pub(super) instance: ResidentNodeKey,
}

pub(super) fn lower(
    inputs: RepositoryLoweringInputs,
    definitions: &mut BTreeMap<ResidentNodeKey, (ResidentNodeKind, ResidentDefinitionDraft)>,
    edges: &mut Vec<PendingResidentEdge>,
) -> Result<(), DefinitionStoreError> {
    let RepositoryLoweringInputs {
        repositories,
        materializations,
        mut scopes,
        package_closure,
        profile,
        instance,
    } = inputs;
    for (repository, materialization) in repositories.into_iter().zip(materializations) {
        lower_repository(
            repository,
            materialization,
            &mut scopes,
            [&package_closure, &profile, &instance],
            definitions,
            edges,
        )?;
    }
    if !scopes.is_empty() {
        return Err(DefinitionStoreError::OrphanRepositoryPackageScope);
    }
    Ok(())
}

fn lower_repository(
    repository: CompiledRepo,
    materialization: AppliedRepositoryMaterialization,
    scopes: &mut BTreeMap<LogicalRepositoryIdV1, RepositoryPackageSelection>,
    roots: [&ResidentNodeKey; 3],
    definitions: &mut BTreeMap<ResidentNodeKey, (ResidentNodeKind, ResidentDefinitionDraft)>,
    edges: &mut Vec<PendingResidentEdge>,
) -> Result<(), DefinitionStoreError> {
    let repository_id = repository.id().clone();
    let repository_key = ResidentNodeKey::Repository(repository_id.clone());
    // Render support is compiler evidence, not a second semantic-definition
    // source. The admitted composition plan owns experience-path definitions;
    // lowering a repository projection must not recreate them from support rows.
    let scope = scopes.remove(&repository_id).ok_or_else(|| {
        DefinitionStoreError::MissingRepositoryPackageScope {
            repository: repository_id.clone(),
        }
    })?;
    if scope.governing_order != repository.semantic_plan().governing_order {
        return Err(DefinitionStoreError::RepositoryPackageProjectionMismatch {
            repository: repository_id,
        });
    }
    for (dependency, relation) in [
        (roots[0], ResidentRelation::RepositoryPackageClosure),
        (roots[1], ResidentRelation::RepositoryProfile),
        (roots[2], ResidentRelation::RepositoryProjectInstance),
    ] {
        pending_edge(edges, repository_key.clone(), dependency.clone(), relation);
    }
    for resource in &scope.resources {
        pending_edge(
            edges,
            repository_key.clone(),
            ResidentNodeKey::PackageResource(resource.clone()),
            ResidentRelation::RepositoryPackageScope,
        );
    }
    for (position, operation) in repository.semantic_plan().operations.iter().enumerate() {
        let operation_key = ResidentNodeKey::Operation {
            repository: repository_id.clone(),
            operation: operation.record.id.clone(),
        };
        pending_edge(
            edges,
            operation_key.clone(),
            repository_key.clone(),
            ResidentRelation::OperationRepository,
        );
        pending_edge(
            edges,
            operation_key.clone(),
            ResidentNodeKey::PackageResource(crate::packages::PackageResourceUse::new(
                operation.record.id.clone(),
                pale_core::records::PackageResourceKindV1::Operation,
            )),
            ResidentRelation::OperationPackageResource,
        );
        insert_unique(
            definitions,
            operation_key,
            ResidentNodeKind::Operation,
            ResidentDefinitionDraft::Operation {
                repository: repository_key.clone(),
                position,
            },
        )?;
    }
    for (position, mode) in repository.semantic_plan().run_modes.iter().enumerate() {
        let mode_key = ResidentNodeKey::RunMode {
            repository: repository_id.clone(),
            mode: mode.record.id.clone(),
        };
        pending_edge(
            edges,
            mode_key.clone(),
            repository_key.clone(),
            ResidentRelation::RunModeRepository,
        );
        pending_edge(
            edges,
            mode_key.clone(),
            ResidentNodeKey::PackageResource(crate::packages::PackageResourceUse::new(
                mode.record.id.clone(),
                pale_core::records::PackageResourceKindV1::RunMode,
            )),
            ResidentRelation::RunModePackageResource,
        );
        insert_unique(
            definitions,
            mode_key,
            ResidentNodeKind::RunMode,
            ResidentDefinitionDraft::RunMode {
                repository: repository_key.clone(),
                position,
            },
        )?;
    }
    for behavior in definitions
        .keys()
        .filter(|key| matches!(key, ResidentNodeKey::Lifecycle(_)))
        .cloned()
        .collect::<Vec<_>>()
    {
        pending_edge(
            edges,
            repository_key.clone(),
            behavior,
            ResidentRelation::RepositoryLifecycle,
        );
    }
    insert_unique(
        definitions,
        repository_key,
        ResidentNodeKind::Repository,
        ResidentDefinitionDraft::Complete(ResidentDefinition::Repository(Box::new(
            ResidentRepositoryDefinition {
                compiled: repository,
                materialization,
            },
        ))),
    )
}
