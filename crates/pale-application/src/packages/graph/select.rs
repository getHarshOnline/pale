//! Positive occurrence closure, governing composition, and semantic-IR sealing.

use super::*;

impl AdmittedPackageGraph {
    /// Close principal-selected occurrence seeds and seal the exact used graph.
    ///
    /// `repository_resources` carries owner-qualified role, stack, and policy-
    /// family selections from portable repository declarations. Package release
    /// availability alone never inserts a semantic node.
    ///
    /// The dependency phase computes the least fixed point rendered below over
    /// the finite admitted resource universe. Here `S` contains only explicit
    /// global and repository occurrence seeds, `D` contains already-qualified
    /// positive dependencies, and `X` is the resulting closure.
    #[doc = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/docs/math/package-selection-closure.display.mathml"
    ))]
    ///
    /// Governing rows
    /// then enter through the same monotone closure exactly when their own
    /// scope intersects `X`. Activating one row never activates
    /// unrelated rows merely because they share a contributor. Ordered sets
    /// make traversal and the final digest independent of source and map
    /// insertion order.
    ///
    /// # Errors
    ///
    /// Returns [`PackageError`] for unselected Package owners, internal or wrong-
    /// kind workspace seeds, missing reachable nodes, or a conflict/cycle in a
    /// repository-local or whole-profile governing algebra.
    pub(crate) fn select_and_seal(
        self,
        selection: &SemanticSelection,
        repository_resources: &BTreeMap<
            LogicalRepositoryIdV1,
            Vec<(PackageResourceRefV1, PackageResourceKindV1)>,
        >,
    ) -> Result<SealedPackageGraph, PackageError> {
        let mut frontier = BTreeSet::new();
        let mut selection_diagnostics = PackageDiagnosticAccumulator::default();
        self.select_field(
            &selection.operations,
            PackageResourceKindV1::Operation,
            &mut frontier,
            &mut selection_diagnostics,
        );
        self.select_field(
            &selection.roles,
            PackageResourceKindV1::Role,
            &mut frontier,
            &mut selection_diagnostics,
        );
        self.select_field(
            &selection.doctrine,
            PackageResourceKindV1::Doctrine,
            &mut frontier,
            &mut selection_diagnostics,
        );
        self.select_field(
            &selection.run_modes,
            PackageResourceKindV1::RunMode,
            &mut frontier,
            &mut selection_diagnostics,
        );
        self.select_field(
            &selection.stacks,
            PackageResourceKindV1::Stack,
            &mut frontier,
            &mut selection_diagnostics,
        );
        for mode in selection.mode_defaults.values() {
            selection_diagnostics.capture(self.select_exact(
                mode,
                PackageResourceKindV1::RunMode,
                &mut frontier,
            ));
        }
        let global_seeds = frontier.clone();
        let mut repository_seeds = BTreeMap::new();
        for (repository, resources) in repository_resources {
            let mut seeds = BTreeSet::new();
            for (resource, kind) in resources {
                selection_diagnostics.capture(self.select_exact(resource, *kind, &mut seeds));
            }
            frontier.extend(seeds.iter().cloned());
            repository_seeds.insert(repository.clone(), seeds);
        }
        selection_diagnostics.finish(())?;

        let (selected, active_governing) =
            close_governing_fixed_point(&self.resources, &self.governing, frontier)?;
        let selected_policies = retain_kind(
            self.capability_policies.clone(),
            &selected,
            PackageResourceKindV1::CapabilityPolicy,
        );
        let governing_order = compose_governing(active_governing, &selected_policies)?;

        // Repository projections must never inherit a row merely because
        // another repository selected its scope. Close each repository from
        // the global seeds plus only its own occurrence seeds.
        let mut repository_selections = BTreeMap::new();
        for (repository, seeds) in &repository_seeds {
            let mut local_frontier = global_seeds.clone();
            local_frontier.extend(seeds.iter().cloned());
            let (resources, active_governing) =
                close_governing_fixed_point(&self.resources, &self.governing, local_frontier)?;
            let policies = self
                .capability_policies
                .iter()
                .filter(|(resource, _)| {
                    resources.contains(&PackageResourceUse::new(
                        (*resource).clone(),
                        PackageResourceKindV1::CapabilityPolicy,
                    ))
                })
                .map(|(resource, policy)| (resource.clone(), policy.clone()))
                .collect::<BTreeMap<_, _>>();
            let governing_order = compose_governing(active_governing, &policies)?;
            repository_selections.insert(
                repository.clone(),
                RepositoryPackageSelection {
                    resources,
                    governing_order,
                },
            );
        }
        let resources = retain_uses(self.resources, &selected);
        let bodies = repack_markdown_bodies(retain_bodies(self.bodies, &selected));
        let recipes = retain_kind(self.recipes, &selected, PackageResourceKindV1::ToolRecipe);
        let sealed = SealedPackageGraph::seal(SelectedPackageGraphState {
            releases: self.releases,
            global_seeds,
            repository_seeds,
            repository_selections,
            resources,
            bodies,
            capability_policies: selected_policies,
            recipes,
            governing_order,
        })?;
        Ok(sealed)
    }

    fn select_field(
        &self,
        selectors: &[PackageResourceSelectorV1],
        kind: PackageResourceKindV1,
        frontier: &mut BTreeSet<PackageResourceUse>,
        diagnostics: &mut PackageDiagnosticAccumulator,
    ) {
        let mut selectors = selectors.iter().collect::<Vec<_>>();
        selectors.sort_unstable_by(|left, right| selector_key(left).cmp(&selector_key(right)));
        for selector in selectors {
            match selector {
                PackageResourceSelectorV1::Exact { resource } => {
                    diagnostics.capture(self.select_exact(resource, kind, frontier));
                }
                PackageResourceSelectorV1::AllFromPackage { package_id } => {
                    let mut matched_owner = false;
                    for (resource, node) in &self.resources {
                        if &resource.resource.package_id == package_id {
                            matched_owner = true;
                            if node.exposure == PackageResourceExposureV1::Exported
                                && resource_kind(&node.resource) == kind
                            {
                                frontier.insert(resource.clone());
                            }
                        }
                    }
                    if !matched_owner {
                        diagnostics.push(PackageError::InvalidResourceSelector {
                            kind,
                            package: package_id.clone(),
                            resource: None,
                            reason: ResourceSelectorViolation::OwnerOutsideClosure,
                        });
                    }
                }
            }
        }
    }

    fn select_exact(
        &self,
        resource: &PackageResourceRefV1,
        kind: PackageResourceKindV1,
        frontier: &mut BTreeSet<PackageResourceUse>,
    ) -> Result<(), PackageError> {
        let selected_use = PackageResourceUse::new(resource.clone(), kind);
        let node = self.resources.get(&selected_use).ok_or_else(|| {
            PackageError::InvalidResourceSelector {
                kind,
                package: resource.package_id.clone(),
                resource: Some(resource.resource_id.clone()),
                reason: if self
                    .resources
                    .keys()
                    .any(|usage| &usage.resource == resource)
                {
                    ResourceSelectorViolation::WrongKind
                } else {
                    ResourceSelectorViolation::MissingResource
                },
            }
        })?;
        if node.exposure != PackageResourceExposureV1::Exported {
            return Err(PackageError::InvalidResourceSelector {
                kind,
                package: resource.package_id.clone(),
                resource: Some(resource.resource_id.clone()),
                reason: ResourceSelectorViolation::InternalResource,
            });
        }
        if resource_kind(&node.resource) != kind {
            return Err(PackageError::InvalidResourceSelector {
                kind,
                package: resource.package_id.clone(),
                resource: Some(resource.resource_id.clone()),
                reason: ResourceSelectorViolation::WrongKind,
            });
        }
        frontier.insert(selected_use);
        Ok(())
    }
}

fn close_governing_fixed_point(
    resources: &BTreeMap<PackageResourceUse, AdmittedResourceNode>,
    governing: &BTreeMap<PackageResourceRefV1, ResolvedGoverningContributionV1>,
    mut frontier: BTreeSet<PackageResourceUse>,
) -> Result<
    (
        BTreeSet<PackageResourceUse>,
        BTreeMap<PackageResourceRefV1, ResolvedGoverningContributionV1>,
    ),
    PackageError,
> {
    let mut selected = BTreeSet::new();
    close_resource_dependencies(resources, &mut frontier, &mut selected)?;
    loop {
        let active_governing = active_governing_rows(governing, &selected);
        let previous_size = selected.len();
        for (contributor, contribution) in &active_governing {
            frontier.insert(PackageResourceUse::new(
                contributor.clone(),
                PackageResourceKindV1::GoverningOrder,
            ));
            for dependency in governing_dependencies(contribution) {
                frontier.insert(dependency);
            }
        }
        close_resource_dependencies(resources, &mut frontier, &mut selected)?;
        if selected.len() == previous_size {
            break;
        }
    }

    // All dependencies of a newly active row can already be present, so row
    // activity itself must be recomputed once at the fixed point.
    let active_governing = active_governing_rows(governing, &selected);
    Ok((selected, active_governing))
}

fn active_governing_rows(
    governing: &BTreeMap<PackageResourceRefV1, ResolvedGoverningContributionV1>,
    selected: &BTreeSet<PackageResourceUse>,
) -> BTreeMap<PackageResourceRefV1, ResolvedGoverningContributionV1> {
    governing
        .iter()
        .map(|(contributor, contribution)| {
            (contributor, activate_governing_rows(contribution, selected))
        })
        .filter(|(_, active)| !governing_is_empty(active))
        .map(|(contributor, active)| (contributor.clone(), active))
        .collect()
}

fn compose_governing(
    active: BTreeMap<PackageResourceRefV1, ResolvedGoverningContributionV1>,
    policies: &BTreeMap<PackageResourceRefV1, CapabilityPolicy>,
) -> Result<AdmittedGoverningOrderV1, PackageError> {
    ResolvedGoverningContributionV1::compose(active.into_values(), policies)
        .map_err(PackageError::from)
}

fn selector_key(selector: &PackageResourceSelectorV1) -> (&str, &str) {
    match selector {
        PackageResourceSelectorV1::Exact { resource } => {
            (resource.package_id.as_str(), resource.resource_id.as_str())
        }
        PackageResourceSelectorV1::AllFromPackage { package_id } => (package_id.as_str(), ""),
    }
}

fn close_resource_dependencies(
    resources: &BTreeMap<PackageResourceUse, AdmittedResourceNode>,
    frontier: &mut BTreeSet<PackageResourceUse>,
    selected: &mut BTreeSet<PackageResourceUse>,
) -> Result<(), PackageError> {
    while let Some(resource) = frontier.pop_first() {
        if !selected.insert(resource.clone()) {
            continue;
        }
        let node =
            resources
                .get(&resource)
                .ok_or_else(|| PackageError::InvalidResourceSelector {
                    kind: resource.kind,
                    package: resource.resource.package_id.clone(),
                    resource: Some(resource.resource.resource_id.clone()),
                    reason: ResourceSelectorViolation::MissingQualifiedDependency,
                })?;
        frontier.extend(node.required_uses());
    }
    Ok(())
}

fn activate_governing_rows(
    contribution: &ResolvedGoverningContributionV1,
    selected: &BTreeSet<PackageResourceUse>,
) -> ResolvedGoverningContributionV1 {
    ResolvedGoverningContributionV1 {
        constraints: contribution
            .constraints
            .iter()
            .filter(|row| selected.contains(&row.scope))
            .cloned()
            .collect(),
        obligations: contribution
            .obligations
            .iter()
            .filter(|row| selected.contains(&row.scope))
            .cloned()
            .collect(),
        exclusive_authority: contribution
            .exclusive_authority
            .iter()
            .filter(|row| selected.contains(&row.scope))
            .cloned()
            .collect(),
        narrowing: contribution
            .narrowing
            .iter()
            .filter(|row| selected.contains(&row.scope))
            .cloned()
            .collect(),
        order: contribution
            .order
            .iter()
            .filter(|row| selected.contains(&row.scope))
            .cloned()
            .collect(),
    }
}

fn governing_is_empty(contribution: &ResolvedGoverningContributionV1) -> bool {
    contribution.constraints.is_empty()
        && contribution.obligations.is_empty()
        && contribution.exclusive_authority.is_empty()
        && contribution.narrowing.is_empty()
        && contribution.order.is_empty()
}

fn governing_dependencies(
    contribution: &ResolvedGoverningContributionV1,
) -> impl Iterator<Item = PackageResourceUse> + '_ {
    // The fixed-point caller already selects each enclosing GoverningOrder.
    let mut uses = BTreeSet::new();
    for row in &contribution.constraints {
        uses.extend([row.scope.clone(), row.subject.clone()]);
        uses.insert(PackageResourceUse::new(
            row.policy.clone(),
            PackageResourceKindV1::CapabilityPolicy,
        ));
    }
    for row in &contribution.obligations {
        uses.extend([
            row.scope.clone(),
            row.subject.clone(),
            row.obligation.clone(),
        ]);
    }
    for row in &contribution.exclusive_authority {
        uses.extend([row.scope.clone(), row.authority.clone()]);
    }
    for row in &contribution.narrowing {
        uses.extend([row.scope.clone(), row.narrower.clone(), row.broader.clone()]);
    }
    for row in &contribution.order {
        uses.extend([row.scope.clone(), row.before.clone(), row.after.clone()]);
    }
    uses.into_iter()
}

fn retain_uses<T>(
    values: BTreeMap<PackageResourceUse, T>,
    selected: &BTreeSet<PackageResourceUse>,
) -> BTreeMap<PackageResourceUse, T> {
    values
        .into_iter()
        .filter(|(usage, _)| selected.contains(usage))
        .collect()
}

fn retain_bodies<T>(
    values: BTreeMap<PackageResourceRefV1, T>,
    selected: &BTreeSet<PackageResourceUse>,
) -> BTreeMap<PackageResourceRefV1, T> {
    let pages = selected
        .iter()
        .map(|usage| &usage.resource)
        .collect::<BTreeSet<_>>();
    values
        .into_iter()
        .filter(|(resource, _)| pages.contains(resource))
        .collect()
}

fn retain_kind<T>(
    values: BTreeMap<PackageResourceRefV1, T>,
    selected: &BTreeSet<PackageResourceUse>,
    kind: PackageResourceKindV1,
) -> BTreeMap<PackageResourceRefV1, T> {
    values
        .into_iter()
        .filter(|(resource, _)| selected.contains(&PackageResourceUse::new(resource.clone(), kind)))
        .collect()
}

const fn resource_kind(resource: &PackageResource) -> PackageResourceKindV1 {
    match resource {
        PackageResource::Doctrine { .. } => PackageResourceKindV1::Doctrine,
        PackageResource::OperationArchetype { .. } => PackageResourceKindV1::OperationArchetype,
        PackageResource::Operation { .. } => PackageResourceKindV1::Operation,
        PackageResource::RoleArchetype { .. } => PackageResourceKindV1::RoleArchetype,
        PackageResource::Role { .. } => PackageResourceKindV1::Role,
        PackageResource::Stack { .. } => PackageResourceKindV1::Stack,
        PackageResource::RunMode { .. } => PackageResourceKindV1::RunMode,
        PackageResource::PolicyFamily { .. } => PackageResourceKindV1::PolicyFamily,
        PackageResource::GoverningOrder => PackageResourceKindV1::GoverningOrder,
        PackageResource::CapabilityPolicy => PackageResourceKindV1::CapabilityPolicy,
        PackageResource::ToolRecipe => PackageResourceKindV1::ToolRecipe,
    }
}

impl SealedPackageGraph {
    /// Consume selected Package bodies into the resident-definition boundary.
    pub(crate) fn into_resident_parts(self) -> ResidentPackageParts {
        let SelectedPackageGraphState {
            global_seeds,
            repository_seeds,
            repository_selections,
            resources,
            bodies,
            capability_policies,
            recipes,
            ..
        } = self.state;
        ResidentPackageParts {
            global_seeds,
            repository_seeds,
            resources,
            bodies,
            capability_policies,
            recipes,
            repository_scopes: repository_selections,
            digest: self.digest,
        }
    }

    fn seal(state: SelectedPackageGraphState) -> Result<Self, PackageError> {
        let digest = package_graph_digest(&state)?;
        Ok(Self { state, digest })
    }

    /// Recompute the complete selected closure and compare its canonical seal.
    ///
    /// # Errors
    ///
    /// Returns [`PackageError`] when canonical encoding fails or the recomputed
    /// digest differs from the retained seal.
    pub fn verify(&self) -> Result<(), PackageError> {
        let observed = package_graph_digest(&self.state)?;
        if observed == self.digest {
            Ok(())
        } else {
            Err(PackageError::GraphInvariant(
                PackageGraphInvariant::SealMismatch,
            ))
        }
    }

    /// Digest binding the exact selected releases, occurrence seeds, qualified
    /// resource edges, repository-local closures, admitted bodies, procedures,
    /// policies, and whole-profile governing composition.
    #[must_use]
    pub fn digest(&self) -> &PackageSemanticIrDigest {
        &self.digest
    }

    /// Exact Package releases whose selected resources contribute to this IR.
    #[must_use]
    pub fn releases(&self) -> &[PackageReleaseV1] {
        &self.state.releases
    }

    /// Number of selected typed Package resources in this sealed closure.
    #[must_use]
    pub fn resource_count(&self) -> usize {
        self.state.resources.len()
    }

    pub(crate) fn resources(&self) -> &BTreeMap<PackageResourceUse, AdmittedResourceNode> {
        &self.state.resources
    }

    /// Dependency-closed semantic resources visible to one repository.
    pub(crate) fn repository_scope(
        &self,
        repository: &LogicalRepositoryIdV1,
    ) -> Result<BTreeSet<PackageResourceUse>, PackageError> {
        self.state
            .repository_selections
            .get(repository)
            .map(|selection| selection.resources.clone())
            .ok_or_else(|| {
                PackageError::GraphInvariant(PackageGraphInvariant::MissingRepositorySeeds {
                    repository: repository.clone(),
                })
            })
    }

    /// Governing result activated by global and repository-local scopes only.
    pub(crate) fn repository_governing_order(
        &self,
        repository: &LogicalRepositoryIdV1,
    ) -> Result<&AdmittedGoverningOrderV1, PackageError> {
        self.state
            .repository_selections
            .get(repository)
            .map(|selection| &selection.governing_order)
            .ok_or_else(|| {
                PackageError::GraphInvariant(PackageGraphInvariant::MissingRepositorySeeds {
                    repository: repository.clone(),
                })
            })
    }

    pub(crate) fn body(&self, resource: &PackageResourceRefV1) -> Option<&AdmittedMarkdown> {
        self.state.bodies.get(resource)
    }

    pub(crate) fn recipes(&self) -> &BTreeMap<PackageResourceRefV1, ToolRecipe> {
        &self.state.recipes
    }

    /// Whole-profile governing result used only for cross-repository
    /// contradiction proof, never as repository render authority.
    pub(crate) fn profile_governing_order(&self) -> &AdmittedGoverningOrderV1 {
        &self.state.governing_order
    }
}

fn resource_records<T, K>(records: &BTreeMap<K, T>) -> Vec<ResourceRecord<'_, T, K>> {
    records
        .iter()
        .map(|(resource, record)| ResourceRecord { resource, record })
        .collect()
}

fn package_graph_digest(
    state: &SelectedPackageGraphState,
) -> Result<PackageSemanticIrDigest, PackageError> {
    let bodies = state
        .bodies
        .iter()
        .map(|(resource, body)| MarkdownSeal {
            resource,
            digest: body.digest(),
            text: body.text(),
            links: body.links(),
            related_reading: body.related_reading(),
        })
        .collect();
    PackageSemanticIrDigest::hash_canonical(&PackageGraphPreimage {
        schema: PACKAGE_SEMANTIC_IR_DOMAIN,
        releases: &state.releases,
        global_seeds: &state.global_seeds,
        repository_seeds: &state.repository_seeds,
        repository_selections: &state.repository_selections,
        resources: resource_records(&state.resources),
        bodies,
        capability_policies: resource_records(&state.capability_policies),
        recipes: resource_records(&state.recipes),
        governing_order: &state.governing_order,
    })
    .map_err(|error| PackageError::GraphInvariant(PackageGraphInvariant::CanonicalEncoding(error)))
}

#[cfg(test)]
#[path = "../../../tests/internal/packages/graph/select.rs"]
mod tests;

#[cfg(test)]
#[path = "../../../tests/internal/packages/graph/multiple_uses.rs"]
mod multiple_uses;
