//! Package-definition lowering and closure relationships.

use std::collections::BTreeMap;

use pale_core::records::{LogicalRepositoryIdV1, PackageResourceKindV1};

use crate::packages::{RepositoryPackageSelection, SealedPackageGraph};

use super::{insert_unique, pending_edge, ResidentDefinitionDraft};
use crate::resident::definition::types::{
    PendingResidentEdge, ResidentDefinition, ResidentNodeKey, ResidentNodeKind,
    ResidentPackageClosureDefinition, ResidentPackageDefinition, ResidentRelation,
};
use crate::resident::definition::DefinitionStoreError;

pub(super) fn lower(
    package_semantic_ir: SealedPackageGraph,
    definitions: &mut BTreeMap<ResidentNodeKey, (ResidentNodeKind, ResidentDefinitionDraft)>,
    edges: &mut Vec<PendingResidentEdge>,
) -> Result<
    (
        BTreeMap<LogicalRepositoryIdV1, RepositoryPackageSelection>,
        ResidentNodeKey,
    ),
    DefinitionStoreError,
> {
    let mut package = package_semantic_ir.into_resident_parts();
    let package_closure_key = ResidentNodeKey::PackageClosure(package.digest.clone());
    let seed_custody_is_complete = package
        .global_seeds
        .iter()
        .all(|seed| package.resources.contains_key(seed))
        && package.repository_seeds.iter().all(|(repository, seeds)| {
            package
                .repository_scopes
                .get(repository)
                .is_some_and(|scope| {
                    seeds.iter().all(|seed| {
                        package.resources.contains_key(seed) && scope.resources.contains(seed)
                    })
                })
        })
        && package.repository_seeds.len() == package.repository_scopes.len();
    if !seed_custody_is_complete {
        return Err(DefinitionStoreError::OrphanPackageBody);
    }
    insert_unique(
        definitions,
        package_closure_key.clone(),
        ResidentNodeKind::PackageClosure,
        ResidentDefinitionDraft::Complete(ResidentDefinition::PackageClosure(Box::new(
            ResidentPackageClosureDefinition {
                global_seeds: package.global_seeds,
                repository_seeds: package.repository_seeds,
            },
        ))),
    )?;
    let bodies = package
        .bodies
        .into_iter()
        .map(|(page, body)| (page, std::sync::Arc::new(body)))
        .collect::<BTreeMap<_, _>>();
    let mut used_bodies = std::collections::BTreeSet::new();
    for (resource_id, node) in package.resources {
        let dependencies = node.required_uses();
        let definition = ResidentPackageDefinition {
            node,
            markdown: bodies.get(&resource_id.resource).map(|body| {
                used_bodies.insert(resource_id.resource.clone());
                std::sync::Arc::clone(body)
            }),
            capability_policy: if resource_id.kind == PackageResourceKindV1::CapabilityPolicy {
                package.capability_policies.remove(&resource_id.resource)
            } else {
                None
            },
            recipe: if resource_id.kind == PackageResourceKindV1::ToolRecipe {
                package.recipes.remove(&resource_id.resource)
            } else {
                None
            },
        };
        insert_unique(
            definitions,
            ResidentNodeKey::PackageResource(resource_id.clone()),
            ResidentNodeKind::PackageResource,
            ResidentDefinitionDraft::Complete(ResidentDefinition::PackageResource(Box::new(
                definition,
            ))),
        )?;
        pending_edge(
            edges,
            package_closure_key.clone(),
            ResidentNodeKey::PackageResource(resource_id.clone()),
            ResidentRelation::PackageClosureResource,
        );
        for dependency in dependencies {
            pending_edge(
                edges,
                ResidentNodeKey::PackageResource(resource_id.clone()),
                ResidentNodeKey::PackageResource(dependency),
                ResidentRelation::PackageDependency,
            );
        }
    }
    if !(bodies.len() == used_bodies.len()
        && package.capability_policies.is_empty()
        && package.recipes.is_empty())
    {
        return Err(DefinitionStoreError::OrphanPackageBody);
    }
    Ok((package.repository_scopes, package_closure_key))
}
