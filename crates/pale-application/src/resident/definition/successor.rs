//! Successor input differencing and graph-derived semantic impact.

use pale_core::records::{
    WorkspaceDerivationImpactStateV1, WorkspaceDerivationReceiptV1, WorkspaceDerivationRootChangeV1,
};
use pale_kernel::{GraphAccessError, NodeHandle};

use crate::packages::AdmittedMarkdown;
use crate::resident::generation::{
    resident_definition_coordinate, resident_evidence_coordinate, GenerationInputChange,
    GenerationSuccessorImpact, PackageResourceResidentCoordinate, RemovedPackageResourceBridge,
    RemovedRepositoryBridge, RemovedWorkspaceRootBridge, RepositoryResidentCoordinate,
    ResidentDefinitionCoordinate, ResidentImpactReasonPath, ResidentImpactReasonStep,
};

use super::types::{ResidentDefinition, ResidentNodeKey};
use super::{DefinitionStoreError, ResidentDefinitionStore};

/// Comparison-only coordinates derived in one pass over a resident generation.
struct SuccessorCoordinateIndex {
    workspace_semantic_ir: ResidentNodeKey,
    workspace_admission_evidence: ResidentNodeKey,
    principal_universe: ResidentNodeKey,
    package_closure: ResidentNodeKey,
    profile: ResidentNodeKey,
    project_instance: ResidentNodeKey,
    composition_plan: ResidentNodeKey,
    package_resources: std::collections::BTreeMap<
        crate::packages::PackageResourceUse,
        PackageResourceResidentCoordinate,
    >,
    repositories: std::collections::BTreeMap<
        pale_core::records::LogicalRepositoryIdV1,
        RepositoryResidentCoordinate,
    >,
}

impl ResidentDefinitionStore {
    /// Derive successor impact from one verified workspace transition receipt.
    pub(in crate::resident) fn successor_impact(
        &self,
        prior: &Self,
        receipt: &WorkspaceDerivationReceiptV1,
    ) -> Result<GenerationSuccessorImpact, DefinitionStoreError> {
        let mut seeds = std::collections::BTreeSet::new();
        let mut removed_bridges = std::collections::BTreeMap::new();
        let mut input_changes = Vec::new();
        for root in &receipt.roots {
            input_changes.push(GenerationInputChange::WorkspaceRoot {
                root_digest: root.root_digest.clone(),
                change: Box::new(root.change.clone()),
            });
            match root.change {
                WorkspaceDerivationRootChangeV1::Added { .. }
                | WorkspaceDerivationRootChangeV1::Modified { .. } => {
                    let key = ResidentNodeKey::from_workspace_subject(&root.subject);
                    seeds.insert(self.graph.find(&key).ok_or_else(|| {
                        DefinitionStoreError::MissingImpactSeed {
                            key: Box::new(key.clone()),
                        }
                    })?);
                }
                WorkspaceDerivationRootChangeV1::Removed { .. } => {
                    removed_bridges.insert(root.root_digest.clone(), Vec::new());
                }
            }
        }
        for impact in &receipt.impacts {
            let WorkspaceDerivationImpactStateV1::Affected { root_digests } = &impact.state else {
                continue;
            };
            let matching_removed = root_digests
                .iter()
                .filter(|root| removed_bridges.contains_key(*root))
                .cloned()
                .collect::<Vec<_>>();
            if matching_removed.is_empty() {
                continue;
            }
            let key = ResidentNodeKey::from_workspace_subject(&impact.subject);
            seeds.insert(self.graph.find(&key).ok_or_else(|| {
                DefinitionStoreError::MissingImpactSeed {
                    key: Box::new(key.clone()),
                }
            })?);
            for root in matching_removed {
                if let Some(subjects) = removed_bridges.get_mut(&root) {
                    subjects.push(impact.subject.clone());
                }
            }
        }

        let prior_coordinates = prior.successor_coordinate_index()?;
        let current_coordinates = self.successor_coordinate_index()?;

        self.seed_changed_singleton(
            &prior_coordinates.workspace_semantic_ir,
            &current_coordinates.workspace_semantic_ir,
            |prior, current| match (prior, current) {
                (
                    ResidentNodeKey::WorkspaceSemanticIr(prior),
                    ResidentNodeKey::WorkspaceSemanticIr(current),
                ) => GenerationInputChange::WorkspaceSemanticIr {
                    prior: prior.clone(),
                    current: current.clone(),
                },
                _ => unreachable!("singleton selector fixes the key family"),
            },
            &mut seeds,
            &mut input_changes,
        )?;
        self.seed_changed_singleton(
            &prior_coordinates.workspace_admission_evidence,
            &current_coordinates.workspace_admission_evidence,
            |prior, current| match (prior, current) {
                (
                    ResidentNodeKey::WorkspaceAdmissionEvidence(prior),
                    ResidentNodeKey::WorkspaceAdmissionEvidence(current),
                ) => GenerationInputChange::WorkspaceAdmissionEvidence {
                    prior: prior.clone(),
                    current: current.clone(),
                },
                _ => unreachable!("singleton selector fixes the key family"),
            },
            &mut seeds,
            &mut input_changes,
        )?;
        self.seed_changed_singleton(
            &prior_coordinates.principal_universe,
            &current_coordinates.principal_universe,
            |prior, current| match (prior, current) {
                (
                    ResidentNodeKey::PrincipalUniverse(prior),
                    ResidentNodeKey::PrincipalUniverse(current),
                ) => GenerationInputChange::PrincipalUniverse {
                    prior: prior.clone(),
                    current: current.clone(),
                },
                _ => unreachable!("singleton selector fixes the key family"),
            },
            &mut seeds,
            &mut input_changes,
        )?;

        self.seed_changed_singleton(
            &prior_coordinates.package_closure,
            &current_coordinates.package_closure,
            |prior, current| match (prior, current) {
                (
                    ResidentNodeKey::PackageClosure(prior),
                    ResidentNodeKey::PackageClosure(current),
                ) => GenerationInputChange::PackageClosure {
                    prior: prior.clone(),
                    current: current.clone(),
                },
                _ => unreachable!("singleton selector fixes the key family"),
            },
            &mut seeds,
            &mut input_changes,
        )?;

        let prior_package_resources = prior_coordinates.package_resources;
        let current_package_resources = current_coordinates.package_resources;
        let mut removed_package_resource_bridges = Vec::new();
        for (resource, current) in &current_package_resources {
            match prior_package_resources.get(resource) {
                None => {
                    self.seed_key(
                        &ResidentNodeKey::PackageResource(resource.clone()),
                        &mut seeds,
                    )?;
                    input_changes.push(GenerationInputChange::PackageResourceAdded {
                        current: Box::new(current.clone()),
                    });
                }
                Some(prior) if prior != current => {
                    self.seed_key(
                        &ResidentNodeKey::PackageResource(resource.clone()),
                        &mut seeds,
                    )?;
                    input_changes.push(GenerationInputChange::PackageResourceModified {
                        prior: Box::new(prior.clone()),
                        current: Box::new(current.clone()),
                    });
                }
                Some(_) => {}
            }
        }
        for (resource, prior) in &prior_package_resources {
            if !current_package_resources.contains_key(resource) {
                input_changes.push(GenerationInputChange::PackageResourceRemoved {
                    prior: Box::new(prior.clone()),
                });
                removed_package_resource_bridges.push(RemovedPackageResourceBridge {
                    prior: prior.clone(),
                });
            }
        }
        self.seed_changed_singleton(
            &prior_coordinates.profile,
            &current_coordinates.profile,
            |prior, current| match (prior, current) {
                (ResidentNodeKey::Profile(prior), ResidentNodeKey::Profile(current)) => {
                    GenerationInputChange::Profile {
                        prior: prior.clone(),
                        current: current.clone(),
                    }
                }
                _ => unreachable!("singleton selector fixes the key family"),
            },
            &mut seeds,
            &mut input_changes,
        )?;
        self.seed_changed_singleton(
            &prior_coordinates.project_instance,
            &current_coordinates.project_instance,
            |prior, current| match (prior, current) {
                (
                    ResidentNodeKey::ProjectInstance(prior),
                    ResidentNodeKey::ProjectInstance(current),
                ) => GenerationInputChange::ProjectInstance {
                    prior: prior.clone(),
                    current: current.clone(),
                },
                _ => unreachable!("singleton selector fixes the key family"),
            },
            &mut seeds,
            &mut input_changes,
        )?;
        self.seed_changed_singleton(
            &prior_coordinates.composition_plan,
            &current_coordinates.composition_plan,
            |prior, current| match (prior, current) {
                (
                    ResidentNodeKey::CompositionPlan(prior),
                    ResidentNodeKey::CompositionPlan(current),
                ) => GenerationInputChange::CompositionPlan {
                    prior: prior.clone(),
                    current: current.clone(),
                },
                _ => unreachable!("singleton selector fixes the key family"),
            },
            &mut seeds,
            &mut input_changes,
        )?;

        let prior_repositories = prior_coordinates.repositories;
        let current_repositories = current_coordinates.repositories;
        let mut removed_repository_bridges = Vec::new();
        for (repository_id, current) in &current_repositories {
            match prior_repositories.get(repository_id) {
                None => {
                    self.seed_key(
                        &ResidentNodeKey::Repository(repository_id.clone()),
                        &mut seeds,
                    )?;
                    input_changes.push(GenerationInputChange::RepositoryAdded {
                        current: Box::new(current.clone()),
                    });
                }
                Some(prior) if prior != current => {
                    self.seed_key(
                        &ResidentNodeKey::Repository(repository_id.clone()),
                        &mut seeds,
                    )?;
                    input_changes.push(GenerationInputChange::RepositoryModified {
                        prior: Box::new(prior.clone()),
                        current: Box::new(current.clone()),
                    });
                }
                Some(_) => {}
            }
        }
        for (repository_id, prior) in &prior_repositories {
            if !current_repositories.contains_key(repository_id) {
                input_changes.push(GenerationInputChange::RepositoryRemoved {
                    prior: Box::new(prior.clone()),
                });
                removed_repository_bridges.push(RemovedRepositoryBridge {
                    prior: prior.clone(),
                });
            }
        }

        let closure = self.graph.reverse_impact(seeds)?;
        let (affected, unaffected) = self.semantic_impact(&closure)?;
        let removed_root_bridges = removed_bridges
            .into_iter()
            .map(
                |(root_digest, current_subjects)| RemovedWorkspaceRootBridge {
                    root_digest,
                    current_subjects,
                },
            )
            .collect();
        Ok(GenerationSuccessorImpact {
            input_changes,
            affected,
            unaffected,
            closure,
            removed_root_bridges,
            removed_repository_bridges,
            removed_package_resource_bridges,
        })
    }

    fn semantic_impact(
        &self,
        closure: &pale_kernel::ImpactClosure,
    ) -> Result<
        (
            Vec<ResidentDefinitionCoordinate>,
            Vec<ResidentDefinitionCoordinate>,
        ),
        DefinitionStoreError,
    > {
        Ok((
            self.semantic_coordinates(closure.affected())?,
            self.semantic_coordinates(closure.unaffected())?,
        ))
    }

    pub(in crate::resident) fn impact_reason_path(
        &self,
        closure: &pale_kernel::ImpactClosure,
        target: &ResidentDefinitionCoordinate,
    ) -> Result<Option<ResidentImpactReasonPath>, DefinitionStoreError> {
        let Some(target_handle) = self.graph.find_by(|key| key.cmp_coordinate(target)) else {
            return Ok(None);
        };
        if !closure
            .affected()
            .contains(target_handle)
            .map_err(GraphAccessError::NodeSet)?
        {
            return Ok(None);
        }
        let path = closure.proof().path_to(target_handle)?.ok_or_else(|| {
            DefinitionStoreError::BodyIdentityMismatch {
                key: self
                    .graph
                    .node_key(target_handle)
                    .ok()
                    .cloned()
                    .map(Box::new),
            }
        })?;
        let nodes = path
            .nodes
            .into_iter()
            .map(|handle| {
                resident_definition_coordinate(self.graph.node_key(handle)?).ok_or_else(|| {
                    DefinitionStoreError::BodyIdentityMismatch {
                        key: self.graph.node_key(handle).ok().cloned().map(Box::new),
                    }
                })
            })
            .collect::<Result<Vec<_>, DefinitionStoreError>>()?;
        let steps = path
            .edges
            .into_iter()
            .map(|edge| {
                let (evidence, relation) = self.graph.edge_payload(edge)?;
                Ok(ResidentImpactReasonStep {
                    relation: *relation,
                    evidence: resident_evidence_coordinate(evidence),
                })
            })
            .collect::<Result<Vec<_>, DefinitionStoreError>>()?;
        Ok(Some(ResidentImpactReasonPath {
            target: target.clone(),
            nodes,
            steps,
        }))
    }

    fn semantic_coordinates(
        &self,
        set: &pale_kernel::AdaptiveNodeSet,
    ) -> Result<Vec<ResidentDefinitionCoordinate>, DefinitionStoreError> {
        set.iter()
            .map(|handle| {
                resident_definition_coordinate(self.graph.node_key(handle)?).ok_or_else(|| {
                    DefinitionStoreError::BodyIdentityMismatch {
                        key: self.graph.node_key(handle).ok().cloned().map(Box::new),
                    }
                })
            })
            .collect()
    }

    fn seed_changed_singleton(
        &self,
        prior_key: &ResidentNodeKey,
        current_key: &ResidentNodeKey,
        change: impl FnOnce(&ResidentNodeKey, &ResidentNodeKey) -> GenerationInputChange,
        seeds: &mut std::collections::BTreeSet<NodeHandle>,
        changes: &mut Vec<GenerationInputChange>,
    ) -> Result<(), DefinitionStoreError> {
        if prior_key != current_key {
            self.seed_key(current_key, seeds)?;
            changes.push(change(prior_key, current_key));
        }
        Ok(())
    }

    fn seed_key(
        &self,
        key: &ResidentNodeKey,
        seeds: &mut std::collections::BTreeSet<NodeHandle>,
    ) -> Result<(), DefinitionStoreError> {
        seeds.insert(self.graph.find(key).ok_or_else(|| {
            DefinitionStoreError::MissingImpactSeed {
                key: Box::new(key.clone()),
            }
        })?);
        Ok(())
    }

    /// Derive the coordinates needed to compare two admitted generations.
    ///
    /// The index is disposable comparison state. It neither replaces the
    /// resident graph nor admits identities that the graph does not contain.
    /// Building all coordinate families together keeps successor comparison to
    /// one graph scan per generation.
    fn successor_coordinate_index(&self) -> Result<SuccessorCoordinateIndex, DefinitionStoreError> {
        let mut workspace_semantic_ir = None;
        let mut workspace_admission_evidence = None;
        let mut principal_universe = None;
        let mut package_closure = None;
        let mut profile = None;
        let mut project_instance = None;
        let mut composition_plan = None;
        let mut package_resources = std::collections::BTreeMap::new();
        let mut repositories = std::collections::BTreeMap::new();

        for index in 0..self.graph.node_count() {
            let handle = self
                .graph
                .node_at(u32::try_from(index).map_err(|_| DefinitionStoreError::HandleSpace)?)?;
            let key = self.graph.node_key(handle)?;
            match key {
                ResidentNodeKey::WorkspaceSemanticIr(_) => {
                    Self::set_successor_singleton(&mut workspace_semantic_ir, key)?;
                }
                ResidentNodeKey::WorkspaceAdmissionEvidence(_) => {
                    Self::set_successor_singleton(&mut workspace_admission_evidence, key)?;
                }
                ResidentNodeKey::PrincipalUniverse(_) => {
                    Self::set_successor_singleton(&mut principal_universe, key)?;
                }
                ResidentNodeKey::PackageClosure(_) => {
                    Self::set_successor_singleton(&mut package_closure, key)?;
                }
                ResidentNodeKey::Profile(_) => {
                    Self::set_successor_singleton(&mut profile, key)?;
                }
                ResidentNodeKey::ProjectInstance(_) => {
                    Self::set_successor_singleton(&mut project_instance, key)?;
                }
                ResidentNodeKey::CompositionPlan(_) => {
                    Self::set_successor_singleton(&mut composition_plan, key)?;
                }
                ResidentNodeKey::PackageResource(resource) => {
                    let ResidentDefinition::PackageResource(definition) = self.resolve(handle)?
                    else {
                        return Err(DefinitionStoreError::BodyKindMismatch {
                            key: Some(Box::new(key.clone())),
                        });
                    };
                    let canonical = pale_core::canon::to_canonical_string(&(
                        "pale.resident-package-resource-coordinate.v1",
                        &definition.node,
                        definition.markdown.as_deref().map(AdmittedMarkdown::digest),
                        &definition.capability_policy,
                        &definition.recipe,
                    ))?;
                    let coordinate = PackageResourceResidentCoordinate {
                        resource: resource.clone(),
                        definition_sha256: pale_core::records::ArtifactDigest::hash_bytes(
                            canonical.as_bytes(),
                        ),
                    };
                    if package_resources
                        .insert(resource.clone(), coordinate)
                        .is_some()
                    {
                        return Err(DefinitionStoreError::BodyIdentityMismatch {
                            key: Some(Box::new(key.clone())),
                        });
                    }
                }
                ResidentNodeKey::Repository(repository_id) => {
                    let ResidentDefinition::Repository(repository) = self.resolve(handle)? else {
                        return Err(DefinitionStoreError::BodyKindMismatch {
                            key: Some(Box::new(key.clone())),
                        });
                    };
                    let coordinate = RepositoryResidentCoordinate {
                        repository_id: repository
                            .materialization
                            .source_frontier()
                            .repository_id
                            .clone(),
                        manifest_digest: repository.compiled.manifest().manifest_digest.clone(),
                        receipt_digest: repository
                            .materialization
                            .receipt()
                            .record()
                            .receipt_digest
                            .clone(),
                        state_digest: repository.materialization.state().state_digest.clone(),
                        lineage_digest: repository.materialization.lineage().lineage_digest.clone(),
                        source_frontier: repository.materialization.source_frontier().clone(),
                    };
                    if repositories
                        .insert(repository_id.clone(), coordinate)
                        .is_some()
                    {
                        return Err(DefinitionStoreError::BodyIdentityMismatch {
                            key: Some(Box::new(key.clone())),
                        });
                    }
                }
                _ => {}
            }
        }

        Ok(SuccessorCoordinateIndex {
            workspace_semantic_ir: Self::required_successor_singleton(workspace_semantic_ir)?,
            workspace_admission_evidence: Self::required_successor_singleton(
                workspace_admission_evidence,
            )?,
            principal_universe: Self::required_successor_singleton(principal_universe)?,
            package_closure: Self::required_successor_singleton(package_closure)?,
            profile: Self::required_successor_singleton(profile)?,
            project_instance: Self::required_successor_singleton(project_instance)?,
            composition_plan: Self::required_successor_singleton(composition_plan)?,
            package_resources,
            repositories,
        })
    }

    /// Reject a second member of a resident singleton identity family.
    fn set_successor_singleton(
        slot: &mut Option<ResidentNodeKey>,
        key: &ResidentNodeKey,
    ) -> Result<(), DefinitionStoreError> {
        if slot.replace(key.clone()).is_some() {
            return Err(DefinitionStoreError::BodyIdentityMismatch {
                key: Some(Box::new(key.clone())),
            });
        }
        Ok(())
    }

    /// Require the one member of a resident singleton identity family.
    fn required_successor_singleton(
        key: Option<ResidentNodeKey>,
    ) -> Result<ResidentNodeKey, DefinitionStoreError> {
        key.ok_or(DefinitionStoreError::MissingOwnerNode { key: None })
    }
}
