//! Repository-local projection from the sealed Package semantic graph.
//!
//! This module performs no source I/O. It consumes owner-qualified resources,
//! already parsed Markdown projections, and the private effective profile. The
//! only fixed-point computed here is dependency closure for body composition;
//! Package occurrence reachability was sealed earlier by
//! [`SealedPackageGraph`].

use std::collections::{BTreeMap, BTreeSet};

use pale_core::records::{
    CapabilityBinding, DoctrinePlacement, EffectiveProjectProfileV1, HostCommandSource, ModeAxis,
    ModeDefaultSource, ModeScope, OperationBinding, PackageResourceKindV1, PackageResourceRefV1,
    ProfileRepository,
};

use crate::packages::{
    AdmittedMarkdown, AdmittedOperationRequirement, PackageResource, PackageResourceUse,
    SealedPackageGraph, TextRecipe, TextRecipeBuilder,
};

use super::{
    BodyLineage, DiagnosticAccumulator, InvalidCapabilityBindingReason, InvalidModeDefaultReason,
    ProviderUnavailability, RepositorySemanticPlan, ResolvedCapability, ResolvedDoctrine,
    ResolvedDoctrineRecord, ResolvedOperation, ResolvedOperationMechanics, ResolvedOperationRecord,
    ResolvedPolicyFamily, ResolvedProviderSelection, ResolvedRecipe, ResolvedRole,
    ResolvedRoleRecord, ResolvedRunMode, ResolvedRunModeRecord, ResolvedStack, SemanticCause,
    SemanticComposition, SemanticDiagnostic, SemanticError, SemanticResourceExpectation,
};

type ProjectionResult<T> = Result<T, SemanticDiagnostic>;
type ResolvedModeDefaults = (
    BTreeMap<ModeAxis, PackageResourceRefV1>,
    BTreeMap<ModeAxis, ModeDefaultSource>,
);

fn diagnostic(cause: SemanticCause) -> SemanticDiagnostic {
    SemanticDiagnostic::new(cause)
}

/// One explicit depth-first transition in a deterministic dependency walk.
enum Traversal {
    Enter(PackageResourceRefV1),
    Exit(PackageResourceRefV1),
}

/// Return dependency-first order for a finite set of owner-qualified roots.
///
/// Applies the finite dependency-closure law owned by
/// [`crate::packages::AdmittedPackageGraph::select_and_seal`] to the supplied
/// roots and emits each resource after its dependencies. A resource therefore
/// appears exactly once, independent of root or edge authoring order.
fn dependency_order(
    composition: SemanticComposition,
    roots: impl IntoIterator<Item = PackageResourceRefV1>,
    dependencies: impl Fn(&PackageResourceRefV1) -> ProjectionResult<Vec<PackageResourceRefV1>>,
) -> ProjectionResult<Vec<PackageResourceRefV1>> {
    let mut roots = roots.into_iter().collect::<Vec<_>>();
    roots.sort();
    roots.dedup();
    let mut complete = BTreeSet::new();
    let mut active = BTreeMap::<PackageResourceRefV1, usize>::new();
    let mut path = Vec::<PackageResourceRefV1>::new();
    let mut ordered = Vec::new();
    for root in roots {
        let mut traversal = vec![Traversal::Enter(root)];
        while let Some(step) = traversal.pop() {
            match step {
                Traversal::Enter(resource) => {
                    if complete.contains(&resource) {
                        continue;
                    }
                    if let Some(position) = active.get(&resource).copied() {
                        let cycle = path[position..]
                            .iter()
                            .chain(std::iter::once(&resource))
                            .cloned()
                            .collect::<Vec<_>>();
                        return Err(diagnostic(SemanticCause::CompositionCycle {
                            composition,
                            path: cycle,
                        }));
                    }
                    let mut children = dependencies(&resource)?;
                    children.sort();
                    children.dedup();
                    active.insert(resource.clone(), path.len());
                    path.push(resource.clone());
                    traversal.push(Traversal::Exit(resource));
                    traversal.extend(children.into_iter().rev().map(Traversal::Enter));
                }
                Traversal::Exit(resource) => {
                    debug_assert_eq!(path.last(), Some(&resource));
                    path.pop();
                    active.remove(&resource);
                    if complete.insert(resource.clone()) {
                        ordered.push(resource);
                    }
                }
            }
        }
    }
    Ok(ordered)
}

fn node<'a>(
    graph: &'a SealedPackageGraph,
    resource: &PackageResourceRefV1,
    kind: PackageResourceKindV1,
) -> ProjectionResult<&'a PackageResource> {
    graph
        .resources()
        .get(&PackageResourceUse::new(resource.clone(), kind))
        .map(|node| &node.resource)
        .ok_or_else(|| {
            diagnostic(SemanticCause::WrongResourceKind {
                resource: resource.clone(),
                expected: SemanticResourceExpectation::SelectedResource,
            })
        })
}

fn body<'a>(
    graph: &'a SealedPackageGraph,
    resource: &PackageResourceRefV1,
) -> ProjectionResult<&'a AdmittedMarkdown> {
    graph.body(resource).ok_or_else(|| {
        diagnostic(SemanticCause::MissingBody {
            resource: resource.clone(),
        })
    })
}

fn append_section(target: &mut TextRecipeBuilder, section: &TextRecipe) {
    if section.is_empty() {
        return;
    }
    if !target.is_empty() {
        target.push_static("\n\n");
    }
    target.append(section);
}

/// Collect independent projections without losing later actionable failures.
fn collect_projection<T>(
    diagnostics: &mut DiagnosticAccumulator,
    results: impl IntoIterator<Item = ProjectionResult<T>>,
) -> Vec<T> {
    results
        .into_iter()
        .filter_map(|result| diagnostics.capture(result))
        .collect()
}

fn append_body(
    target: &mut TextRecipeBuilder,
    lineage: &mut Vec<BodyLineage>,
    resource: &PackageResourceRefV1,
    admitted: &AdmittedMarkdown,
) {
    append_section(target, &admitted.body_recipe());
    lineage.push(BodyLineage {
        owner: resource.clone(),
        sha256: admitted.digest().clone(),
    });
}

fn append_shifted_body(
    target: &mut TextRecipeBuilder,
    lineage: &mut Vec<BodyLineage>,
    resource: &PackageResourceRefV1,
    admitted: &AdmittedMarkdown,
) {
    append_section(target, &admitted.shifted_body_recipe(2));
    lineage.push(BodyLineage {
        owner: resource.clone(),
        sha256: admitted.digest().clone(),
    });
}

fn doctrine_dependencies(
    graph: &SealedPackageGraph,
    resource: &PackageResourceRefV1,
) -> ProjectionResult<Vec<PackageResourceRefV1>> {
    match node(graph, resource, PackageResourceKindV1::Doctrine)? {
        PackageResource::Doctrine {
            extends, requires, ..
        } => Ok(extends.iter().chain(requires).cloned().collect()),
        _ => Err(diagnostic(SemanticCause::WrongResourceKind {
            resource: resource.clone(),
            expected: SemanticResourceExpectation::Kind(PackageResourceKindV1::Doctrine),
        })),
    }
}

fn compose_doctrine_body(
    graph: &SealedPackageGraph,
    root: &PackageResourceRefV1,
) -> ProjectionResult<(String, TextRecipe, Vec<BodyLineage>)> {
    let order = dependency_order(SemanticComposition::Doctrine, [root.clone()], |resource| {
        doctrine_dependencies(graph, resource)
    })?;
    let root_body = body(graph, root)?;
    let title = root_body
        .title()
        .map(str::to_owned)
        .unwrap_or_else(|| super::resource_label(root));
    let mut composed = TextRecipeBuilder::default();
    let mut lineage = Vec::with_capacity(order.len());
    for resource in order {
        let admitted = body(graph, &resource)?;
        if &resource == root {
            append_body(&mut composed, &mut lineage, &resource, admitted);
        } else {
            let dependency_title = admitted
                .title()
                .map(str::to_owned)
                .unwrap_or_else(|| super::resource_label(&resource));
            let mut header = TextRecipeBuilder::default();
            header.push_static("## Inherited: ");
            header.push_owned(dependency_title);
            append_section(&mut composed, &header.finish());
            append_section(&mut composed, &admitted.shifted_body_recipe(1));
            lineage.push(BodyLineage {
                owner: resource,
                sha256: admitted.digest().clone(),
            });
        }
    }
    Ok((title, composed.finish(), lineage))
}

fn resolved_doctrine(
    graph: &SealedPackageGraph,
    resource: &PackageResourceRefV1,
) -> ProjectionResult<ResolvedDoctrine> {
    let PackageResource::Doctrine {
        description,
        placement,
        requires,
        see_also,
        official_sources,
        ..
    } = node(graph, resource, PackageResourceKindV1::Doctrine)?
    else {
        return Err(diagnostic(SemanticCause::WrongResourceKind {
            resource: resource.clone(),
            expected: SemanticResourceExpectation::Kind(PackageResourceKindV1::Doctrine),
        }));
    };
    let related_reading = body(graph, resource)?.related_reading().to_vec();
    let (title, body, body_sources) = compose_doctrine_body(graph, resource)?;
    Ok(ResolvedDoctrine {
        title,
        related_reading,
        record: ResolvedDoctrineRecord {
            id: resource.clone(),
            description: description.clone(),
            placement: *placement,
            requires: requires.clone(),
            see_also: see_also.clone(),
            official_sources: official_sources.clone(),
        },
        body,
        body_sources,
    })
}

fn operation_archetype_dependencies(
    graph: &SealedPackageGraph,
    resource: &PackageResourceRefV1,
) -> ProjectionResult<Vec<PackageResourceRefV1>> {
    match node(graph, resource, PackageResourceKindV1::OperationArchetype)? {
        PackageResource::OperationArchetype { extends, .. } => Ok(extends.clone()),
        _ => Err(diagnostic(SemanticCause::WrongResourceKind {
            resource: resource.clone(),
            expected: SemanticResourceExpectation::Kind(PackageResourceKindV1::OperationArchetype),
        })),
    }
}

fn composed_doctrine_text(
    graph: &SealedPackageGraph,
    roots: impl IntoIterator<Item = PackageResourceRefV1>,
) -> ProjectionResult<(TextRecipe, Vec<PackageResourceRefV1>, Vec<BodyLineage>)> {
    let order = dependency_order(SemanticComposition::RequiredDoctrine, roots, |resource| {
        doctrine_dependencies(graph, resource)
    })?;
    let mut text = TextRecipeBuilder::default();
    let mut lineage = Vec::with_capacity(order.len());
    for resource in &order {
        append_shifted_body(&mut text, &mut lineage, resource, body(graph, resource)?);
    }
    Ok((text.finish(), order, lineage))
}

fn binding_index<'a>(
    profile: &'a EffectiveProjectProfileV1,
    diagnostics: &mut DiagnosticAccumulator,
) -> BTreeMap<&'a PackageResourceRefV1, &'a OperationBinding> {
    let mut bindings = BTreeMap::new();
    for entry in &profile.operation_bindings {
        if bindings.insert(&entry.operation, &entry.binding).is_some() {
            diagnostics.push(diagnostic(SemanticCause::DuplicateOperationBinding {
                resource: entry.operation.clone(),
            }));
        }
    }
    bindings
}

/// Decide whether the admitted private narrowing retains one requirement.
///
/// This predicate owns provider-resolution eligibility. The retained
/// [`crate::compile::EffectiveProfileInspection`] later projects the same
/// admitted sets into [`crate::compile::EffectiveProfileNarrowingReason`] or
/// [`crate::compile::EffectiveProfilePolicyBlock`] solely for explanation; it
/// does not rerun provider selection.
fn narrowing_allows(
    profile: &EffectiveProjectProfileV1,
    requirement: &AdmittedOperationRequirement,
) -> bool {
    profile
        .capability_narrowing
        .as_ref()
        .is_none_or(|narrowing| {
            narrowing
                .permitted_capabilities
                .contains(&requirement.capability)
                && !narrowing
                    .forbidden_capabilities
                    .contains(&requirement.capability)
                && narrowing
                    .permitted_effects
                    .contains(&requirement.effect_class)
                && !narrowing
                    .forbidden_effects
                    .contains(&requirement.effect_class)
        })
}

fn provider_selection(
    profile: &EffectiveProjectProfileV1,
    repository: &ProfileRepository,
    graph: &SealedPackageGraph,
    requirement: &AdmittedOperationRequirement,
) -> ProjectionResult<ResolvedProviderSelection> {
    if !narrowing_allows(profile, requirement) {
        return Ok(ResolvedProviderSelection::Unbound(
            ProviderUnavailability::ExcludedByNarrowing,
        ));
    }
    let Some(binding) = profile.capability_bindings.get(&requirement.capability) else {
        return Ok(ResolvedProviderSelection::Unbound(
            ProviderUnavailability::MissingBinding,
        ));
    };
    if binding.effect_class != requirement.effect_class {
        return Ok(ResolvedProviderSelection::Unbound(
            ProviderUnavailability::EffectMismatch,
        ));
    }
    if let Some(recipe) = requirement
        .recipe
        .as_ref()
        .and_then(|id| graph.recipes().get(id))
    {
        if binding.tool_contract.as_ref() != Some(&recipe.tool_contract) {
            return Ok(ResolvedProviderSelection::Unbound(
                ProviderUnavailability::RecipeContractMismatch,
            ));
        }
    }
    Ok(ResolvedProviderSelection::Bound(
        expand_repository_commands(&requirement.capability, binding, repository)?,
    ))
}

/// Resolve repository command sources without inventing a shell grammar.
fn expand_repository_commands(
    capability: &pale_core::records::CapabilityIdV1,
    binding: &CapabilityBinding,
    repository: &ProfileRepository,
) -> ProjectionResult<CapabilityBinding> {
    let mut resolved = binding.clone();
    let Some(provider_tools) = resolved.provider_tools_mut() else {
        return Ok(resolved);
    };
    for row in provider_tools.iter_mut() {
        let mut expanded = Vec::with_capacity(row.tools.len());
        for mut tool in std::mem::take(&mut row.tools) {
            let Some(source) = tool.command_source.take() else {
                expanded.push(tool);
                continue;
            };
            tool.commands = match source {
                HostCommandSource::RepositoryCapabilityCommands => repository
                    .capability_commands
                    .get(capability)
                    .into_iter()
                    .flatten()
                    .enumerate()
                    .map(|(index, command)| {
                        crate::host_command::exact_constraint(command).ok_or_else(|| {
                            diagnostic(SemanticCause::InvalidCapabilityBinding {
                                capability: capability.clone(),
                                repository: repository.id.clone(),
                                reason: InvalidCapabilityBindingReason::CommandOutsideGrammar {
                                    command_index: index + 1,
                                },
                            })
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            };
            tool.commands.sort();
            tool.commands.dedup();
            if !tool.commands.is_empty() {
                expanded.push(tool);
            }
        }
        row.tools = expanded;
    }
    provider_tools.retain(|row| !row.tools.is_empty());
    Ok(resolved)
}

fn resolved_capabilities(
    graph: &SealedPackageGraph,
    profile: &EffectiveProjectProfileV1,
    repository: &ProfileRepository,
    requirements: &[AdmittedOperationRequirement],
) -> ProjectionResult<Vec<ResolvedCapability>> {
    requirements
        .iter()
        .map(|requirement| {
            Ok(ResolvedCapability {
                requirement_id: requirement.id.clone(),
                description: requirement.description.clone(),
                capability_id: requirement.capability.clone(),
                recipe: requirement.recipe.clone(),
                provider: provider_selection(profile, repository, graph, requirement)?,
                effect_class: requirement.effect_class,
                policy: requirement.policy.as_contract(),
                failure_posture: requirement.failure_posture,
            })
        })
        .collect()
}

fn resolved_operation(
    graph: &SealedPackageGraph,
    profile: &EffectiveProjectProfileV1,
    repository: &ProfileRepository,
    bindings: &BTreeMap<&PackageResourceRefV1, &OperationBinding>,
    resource: &PackageResourceRefV1,
) -> ProjectionResult<ResolvedOperation> {
    let PackageResource::Operation {
        description,
        extends,
        doctrine,
        roles,
        outputs,
        requirements,
        ..
    } = node(graph, resource, PackageResourceKindV1::Operation)?
    else {
        return Err(diagnostic(SemanticCause::WrongResourceKind {
            resource: resource.clone(),
            expected: SemanticResourceExpectation::Kind(PackageResourceKindV1::Operation),
        }));
    };

    let archetypes = dependency_order(
        SemanticComposition::OperationArchetype,
        extends.clone(),
        |candidate| operation_archetype_dependencies(graph, candidate),
    )?;
    let mut procedure = TextRecipeBuilder::default();
    let mut body_sources = Vec::new();
    let mut doctrine_roots = doctrine.iter().cloned().collect::<BTreeSet<_>>();
    for archetype in archetypes {
        let PackageResource::OperationArchetype { doctrine, .. } =
            node(graph, &archetype, PackageResourceKindV1::OperationArchetype)?
        else {
            return Err(diagnostic(SemanticCause::WrongResourceKind {
                resource: archetype,
                expected: SemanticResourceExpectation::Kind(
                    PackageResourceKindV1::OperationArchetype,
                ),
            }));
        };
        doctrine_roots.extend(doctrine.iter().cloned());
        append_shifted_body(
            &mut procedure,
            &mut body_sources,
            &archetype,
            body(graph, &archetype)?,
        );
    }
    append_shifted_body(
        &mut procedure,
        &mut body_sources,
        resource,
        body(graph, resource)?,
    );
    let (required_doctrine, doctrine_ids, doctrine_sources) =
        composed_doctrine_text(graph, doctrine_roots)?;
    body_sources.extend(doctrine_sources);

    let mechanics = match bindings.get(resource) {
        None => ResolvedOperationMechanics::Unbound,
        Some(binding) if requirements.is_empty() => ResolvedOperationMechanics::SemanticOnly {
            selection: binding.selection.clone(),
        },
        Some(binding) => ResolvedOperationMechanics::CapabilityBearing {
            selection: binding.selection.clone(),
            capabilities: resolved_capabilities(graph, profile, repository, requirements)?,
        },
    };
    Ok(ResolvedOperation {
        record: ResolvedOperationRecord {
            id: resource.clone(),
            description: description.clone(),
            roles: roles.clone(),
            outputs: outputs.clone(),
        },
        body: procedure.finish(),
        required_doctrine,
        body_sources,
        doctrine_ids,
        mechanics,
    })
}

fn role_lens_dependencies(
    graph: &SealedPackageGraph,
    resource: &PackageResourceRefV1,
) -> ProjectionResult<Vec<PackageResourceRefV1>> {
    match node(graph, resource, PackageResourceKindV1::Role)? {
        PackageResource::Role {
            inherited_lenses, ..
        } => Ok(inherited_lenses.clone()),
        _ => Err(diagnostic(SemanticCause::WrongResourceKind {
            resource: resource.clone(),
            expected: SemanticResourceExpectation::Kind(PackageResourceKindV1::Role),
        })),
    }
}

fn role_archetype_dependencies(
    graph: &SealedPackageGraph,
    resource: &PackageResourceRefV1,
) -> ProjectionResult<Vec<PackageResourceRefV1>> {
    match node(graph, resource, PackageResourceKindV1::RoleArchetype)? {
        PackageResource::RoleArchetype { extends, .. } => Ok(extends.clone()),
        _ => Err(diagnostic(SemanticCause::WrongResourceKind {
            resource: resource.clone(),
            expected: SemanticResourceExpectation::Kind(PackageResourceKindV1::RoleArchetype),
        })),
    }
}

fn resolved_role(
    graph: &SealedPackageGraph,
    selected_stacks: &BTreeSet<PackageResourceRefV1>,
    resource: &PackageResourceRefV1,
) -> ProjectionResult<ResolvedRole> {
    let roles = dependency_order(
        SemanticComposition::RoleLens,
        [resource.clone()],
        |candidate| role_lens_dependencies(graph, candidate),
    )?;
    let mut composed = TextRecipeBuilder::default();
    let mut body_sources = Vec::new();
    let mut doctrine_roots = BTreeSet::new();
    let mut required_reading = BTreeSet::new();
    let mut seen_archetypes = BTreeSet::new();
    for role_id in &roles {
        let PackageResource::Role {
            extends,
            doctrine,
            stacks,
            ..
        } = node(graph, role_id, PackageResourceKindV1::Role)?
        else {
            return Err(diagnostic(SemanticCause::WrongResourceKind {
                resource: role_id.clone(),
                expected: SemanticResourceExpectation::Kind(PackageResourceKindV1::Role),
            }));
        };
        doctrine_roots.extend(doctrine.iter().cloned());
        for stack_id in selected_stacks
            .iter()
            .filter(|stack_id| stacks.is_empty() || stacks.contains(*stack_id))
        {
            let PackageResource::Stack { role_doctrine, .. } =
                node(graph, stack_id, PackageResourceKindV1::Stack)?
            else {
                return Err(diagnostic(SemanticCause::WrongResourceKind {
                    resource: stack_id.clone(),
                    expected: SemanticResourceExpectation::Kind(PackageResourceKindV1::Stack),
                }));
            };
            let stack_use = PackageResourceUse::new(stack_id.clone(), PackageResourceKindV1::Stack);
            let page_stack = graph
                .resources()
                .get(&stack_use)
                .is_some_and(|node| node.page.is_some());
            for guide in role_doctrine {
                if page_stack
                    && matches!(
                        node(graph, guide, PackageResourceKindV1::Doctrine)?,
                        PackageResource::Doctrine {
                            placement: DoctrinePlacement::StandaloneJudgment,
                            ..
                        }
                    )
                {
                    required_reading.insert(guide.clone());
                } else {
                    doctrine_roots.insert(guide.clone());
                }
            }
        }
        let archetypes = dependency_order(
            SemanticComposition::RoleArchetype,
            extends.clone(),
            |candidate| role_archetype_dependencies(graph, candidate),
        )?;
        for archetype in archetypes {
            let PackageResource::RoleArchetype { doctrine, .. } =
                node(graph, &archetype, PackageResourceKindV1::RoleArchetype)?
            else {
                return Err(diagnostic(SemanticCause::WrongResourceKind {
                    resource: archetype,
                    expected: SemanticResourceExpectation::Kind(
                        PackageResourceKindV1::RoleArchetype,
                    ),
                }));
            };
            doctrine_roots.extend(doctrine.iter().cloned());
            if seen_archetypes.insert(archetype.clone()) {
                append_shifted_body(
                    &mut composed,
                    &mut body_sources,
                    &archetype,
                    body(graph, &archetype)?,
                );
            }
        }
        append_shifted_body(
            &mut composed,
            &mut body_sources,
            role_id,
            body(graph, role_id)?,
        );
    }
    let (doctrine, doctrine_ids, doctrine_sources) = composed_doctrine_text(graph, doctrine_roots)?;
    if !doctrine.is_empty() {
        append_section(
            &mut composed,
            &TextRecipe::owned("## Required judgment doctrine"),
        );
        append_section(&mut composed, &doctrine);
        body_sources.extend(doctrine_sources);
    }
    let PackageResource::Role {
        description,
        inherited_lenses,
        stacks,
        ..
    } = node(graph, resource, PackageResourceKindV1::Role)?
    else {
        unreachable!("root role was validated by the dependency walk")
    };
    let mut official_sources = Vec::new();
    for id in &doctrine_ids {
        if let PackageResource::Doctrine {
            official_sources: sources,
            ..
        } = node(graph, id, PackageResourceKindV1::Doctrine)?
        {
            official_sources.extend(sources.iter().cloned());
        }
    }
    Ok(ResolvedRole {
        record: ResolvedRoleRecord {
            id: resource.clone(),
            description: description.clone(),
            inherits_lenses: inherited_lenses.clone(),
            applies_to: stacks.clone(),
        },
        body: composed.finish(),
        body_sources,
        doctrine_ids,
        required_reading: required_reading.into_iter().collect(),
        official_sources,
    })
}

fn run_mode_dependencies(
    graph: &SealedPackageGraph,
    resource: &PackageResourceRefV1,
) -> ProjectionResult<Vec<PackageResourceRefV1>> {
    match node(graph, resource, PackageResourceKindV1::RunMode)? {
        PackageResource::RunMode { extends, .. } => Ok(extends.clone()),
        _ => Err(diagnostic(SemanticCause::WrongResourceKind {
            resource: resource.clone(),
            expected: SemanticResourceExpectation::Kind(PackageResourceKindV1::RunMode),
        })),
    }
}

fn resolved_run_mode(
    graph: &SealedPackageGraph,
    resource: &PackageResourceRefV1,
) -> ProjectionResult<ResolvedRunMode> {
    let PackageResource::RunMode {
        description,
        axis,
        aliases,
        exact_triggers,
        scope,
        ..
    } = node(graph, resource, PackageResourceKindV1::RunMode)?
    else {
        return Err(diagnostic(SemanticCause::WrongResourceKind {
            resource: resource.clone(),
            expected: SemanticResourceExpectation::Kind(PackageResourceKindV1::RunMode),
        }));
    };
    let order = dependency_order(
        SemanticComposition::RunMode,
        [resource.clone()],
        |candidate| run_mode_dependencies(graph, candidate),
    )?;
    let mut composed = TextRecipeBuilder::default();
    let mut runtime_composed = TextRecipeBuilder::default();
    let mut body_sources = Vec::new();
    for candidate in order {
        let PackageResource::RunMode {
            axis: candidate_axis,
            ..
        } = node(graph, &candidate, PackageResourceKindV1::RunMode)?
        else {
            return Err(diagnostic(SemanticCause::WrongResourceKind {
                resource: candidate,
                expected: SemanticResourceExpectation::Kind(PackageResourceKindV1::RunMode),
            }));
        };
        if candidate_axis != axis {
            return Err(diagnostic(SemanticCause::WrongResourceKind {
                resource: candidate,
                expected: SemanticResourceExpectation::SameAxisRunMode,
            }));
        }
        if let Some(admitted) = graph.body(&candidate) {
            append_body(&mut composed, &mut body_sources, &candidate, admitted);
            append_section(&mut runtime_composed, &admitted.shifted_body_recipe(1));
        }
    }
    Ok(ResolvedRunMode {
        record: ResolvedRunModeRecord {
            id: resource.clone(),
            description: description.clone(),
            axis: *axis,
            aliases: aliases.clone(),
            exact_triggers: exact_triggers.clone(),
            scope: *scope,
        },
        body: (!composed.is_empty()).then(|| composed.finish()),
        runtime_body: (!runtime_composed.is_empty()).then(|| runtime_composed.finish()),
        body_sources,
    })
}

fn resolve_mode_defaults(
    profile: &EffectiveProjectProfileV1,
    selected_modes: &BTreeMap<&PackageResourceRefV1, &ResolvedRunMode>,
    user_defaults: &BTreeMap<ModeAxis, PackageResourceRefV1>,
) -> ProjectionResult<ResolvedModeDefaults> {
    let mut defaults = BTreeMap::new();
    let mut sources = BTreeMap::new();
    for axis in [ModeAxis::Intensity, ModeAxis::TaskStance] {
        let selected = preferred_mode_default(
            profile.semantic.mode_defaults.get(&axis),
            user_defaults.get(&axis),
        );
        let Some((resource, source)) = selected else {
            continue;
        };
        let mode = selected_modes.get(resource).ok_or_else(|| {
            diagnostic(SemanticCause::InvalidModeDefault {
                axis,
                resource: resource.clone(),
                reason: InvalidModeDefaultReason::OutsideSelectedClosure,
            })
        })?;
        if mode.record.axis != axis {
            return Err(diagnostic(SemanticCause::InvalidModeDefault {
                axis,
                resource: resource.clone(),
                reason: InvalidModeDefaultReason::WrongAxis,
            }));
        }
        if mode.record.scope != ModeScope::Runtime {
            return Err(diagnostic(SemanticCause::InvalidModeDefault {
                axis,
                resource: resource.clone(),
                reason: InvalidModeDefaultReason::SessionOnly,
            }));
        }
        defaults.insert(axis, resource.clone());
        sources.insert(axis, source);
    }
    Ok((defaults, sources))
}

/// Select one declarative default without erasing its authority layer.
///
/// The project profile is the repository-local decision and therefore
/// outranks the user's distribution-level fallback for the same axis.
fn preferred_mode_default<'a>(
    project: Option<&'a PackageResourceRefV1>,
    user: Option<&'a PackageResourceRefV1>,
) -> Option<(&'a PackageResourceRefV1, ModeDefaultSource)> {
    project
        .map(|resource| (resource, ModeDefaultSource::Project))
        .or_else(|| user.map(|resource| (resource, ModeDefaultSource::User)))
}

/// Project every profile repository from one verified, source-free Package graph.
///
/// Private bindings join only after semantic selection is sealed. An absent
/// operation binding remains unbound; a present binding with no Package
/// requirements is semantic-only; a present binding with requirements is
/// capability-bearing. Missing or incompatible provider selection remains
/// local to its requirement and never invalidates unrelated operations.
///
/// # Errors
///
/// Returns [`SemanticError`] when the graph seal is invalid, a repository is not
/// an exact profile member, body composition cycles, or a mode default escapes
/// its selected owner-qualified scope.
pub(crate) fn project_repository_semantic_plans(
    profile: &EffectiveProjectProfileV1,
    graph: &SealedPackageGraph,
    user_mode_defaults: &BTreeMap<ModeAxis, PackageResourceRefV1>,
) -> Result<Vec<RepositorySemanticPlan>, SemanticError> {
    graph.verify()?;
    profile
        .repositories
        .iter()
        .map(|repository| {
            project_verified_repository_semantic_plan(
                profile,
                repository,
                graph,
                user_mode_defaults,
            )
        })
        .collect()
}

/// Project one member after [`project_repository_semantic_plans`] verifies the graph.
fn project_verified_repository_semantic_plan(
    profile: &EffectiveProjectProfileV1,
    repository: &ProfileRepository,
    graph: &SealedPackageGraph,
    user_mode_defaults: &BTreeMap<ModeAxis, PackageResourceRefV1>,
) -> Result<RepositorySemanticPlan, SemanticError> {
    let mut matches = profile
        .repositories
        .iter()
        .filter(|candidate| candidate.id == repository.id);
    if matches.next() != Some(repository) || matches.next().is_some() {
        return Err(diagnostic(SemanticCause::RepositoryProfileMismatch {
            profile: profile.profile_digest.clone(),
            repository: repository.id.clone(),
        })
        .into());
    }
    let scope = graph.repository_scope(&repository.id)?;
    let governing_order = graph.repository_governing_order(&repository.id)?.clone();
    let mut diagnostics = DiagnosticAccumulator::default();
    let bindings = binding_index(profile, &mut diagnostics);
    for operation in bindings.keys().filter(|operation| {
        !scope.contains(&PackageResourceUse::new(
            (**operation).clone(),
            PackageResourceKindV1::Operation,
        ))
    }) {
        diagnostics.push(diagnostic(
            SemanticCause::OperationBindingOutsideSelection {
                resource: (*operation).clone(),
            },
        ));
    }

    let selected_stacks = scope
        .iter()
        .filter(|resource| {
            matches!(
                graph.resources().get(*resource).map(|node| &node.resource),
                Some(PackageResource::Stack { .. })
            )
        })
        .map(|usage| usage.resource.clone())
        .collect::<BTreeSet<_>>();
    let mut projection_diagnostics = DiagnosticAccumulator::default();
    let operations = collect_projection(
        &mut projection_diagnostics,
        scope
            .iter()
            .filter(|resource| {
                matches!(
                    graph.resources().get(*resource).map(|node| &node.resource),
                    Some(PackageResource::Operation { .. })
                )
            })
            .map(|resource| {
                resolved_operation(graph, profile, repository, &bindings, &resource.resource)
            }),
    );
    let roles = collect_projection(
        &mut projection_diagnostics,
        scope
            .iter()
            .filter(|resource| {
                matches!(
                    graph.resources().get(*resource).map(|node| &node.resource),
                    Some(PackageResource::Role { .. })
                )
            })
            .map(|resource| resolved_role(graph, &selected_stacks, &resource.resource)),
    );

    // Header pages link required reading instead of embedding it. A legacy
    // procedure or startup policy therefore also needs a guide surface when a
    // selected page requires it, without changing its original placement.
    let linked_guides = scope
        .iter()
        .filter_map(|usage| graph.resources().get(usage)?.page.as_ref())
        .flat_map(|page| page.required.iter())
        .collect::<BTreeSet<_>>();
    let mut doctrine = Vec::new();
    let mut startup_policies = Vec::new();
    for resource in scope.iter().filter(|resource| {
        matches!(
            graph.resources().get(*resource).map(|node| &node.resource),
            Some(PackageResource::Doctrine { .. })
        )
    }) {
        let Some(resolved) =
            projection_diagnostics.capture(resolved_doctrine(graph, &resource.resource))
        else {
            continue;
        };
        let linked = linked_guides.contains(&resolved.record.id);
        match resolved.record.placement {
            DoctrinePlacement::StandaloneJudgment => doctrine.push(resolved),
            DoctrinePlacement::StartupPolicy => {
                if linked {
                    doctrine.push(resolved.clone());
                }
                startup_policies.push(resolved);
            }
            DoctrinePlacement::OperationProcedure if linked => doctrine.push(resolved),
            DoctrinePlacement::OperationProcedure => {}
        }
    }

    let run_modes = collect_projection(
        &mut projection_diagnostics,
        scope
            .iter()
            .filter(|resource| {
                matches!(
                    graph.resources().get(*resource).map(|node| &node.resource),
                    Some(PackageResource::RunMode { .. })
                )
            })
            .map(|resource| resolved_run_mode(graph, &resource.resource)),
    );
    let run_mode_index = run_modes
        .iter()
        .map(|mode| (&mode.record.id, mode))
        .collect::<BTreeMap<_, _>>();
    let (mode_defaults, mode_default_sources) = projection_diagnostics
        .capture(resolve_mode_defaults(
            profile,
            &run_mode_index,
            user_mode_defaults,
        ))
        .unwrap_or_default();

    let stacks = collect_projection(
        &mut projection_diagnostics,
        selected_stacks.iter().map(|resource| {
            let PackageResource::Stack {
                description,
                extends,
                doctrine,
                role_doctrine,
                official_sources,
                ..
            } = node(graph, resource, PackageResourceKindV1::Stack)?
            else {
                unreachable!("selected stack kind was established above")
            };
            Ok(ResolvedStack {
                id: resource.clone(),
                description: description.clone(),
                extends: extends.clone(),
                doctrine: doctrine.clone(),
                role_doctrine: role_doctrine.clone(),
                official_sources: official_sources.clone(),
                body: graph.body(resource).map(AdmittedMarkdown::text_recipe),
            })
        }),
    );

    let policy_families = collect_projection(
        &mut projection_diagnostics,
        scope
            .iter()
            .filter_map(|resource| {
                let node = graph.resources().get(resource)?;
                let PackageResource::PolicyFamily { title, purpose } = &node.resource else {
                    return None;
                };
                Some((&resource.resource, title, purpose))
            })
            .map(|(resource, title, purpose)| {
                let admitted = body(graph, resource)?;
                Ok(ResolvedPolicyFamily {
                    id: resource.clone(),
                    title: title.clone(),
                    purpose: purpose.clone(),
                    body: admitted.text_recipe(),
                    body_sources: vec![BodyLineage {
                        owner: resource.clone(),
                        sha256: admitted.digest().clone(),
                    }],
                })
            }),
    );

    let recipes = graph
        .recipes()
        .iter()
        .filter(|(resource, _)| {
            scope.contains(&PackageResourceUse::new(
                (*resource).clone(),
                PackageResourceKindV1::ToolRecipe,
            ))
        })
        .map(|(resource, procedure)| ResolvedRecipe {
            resource: resource.clone(),
            procedure: procedure.clone(),
        })
        .collect::<Vec<_>>();
    let mut repository_verifier_capabilities = recipes
        .iter()
        .filter(|recipe| recipe.procedure.evidence.repository_verifier)
        .map(|recipe| recipe.procedure.capability.clone())
        .collect::<Vec<_>>();
    repository_verifier_capabilities.sort();
    repository_verifier_capabilities.dedup();
    diagnostics.merge(projection_diagnostics);

    diagnostics
        .finish(RepositorySemanticPlan {
            page_reading: super::page_reading::project_page_reading(
                graph,
                &scope,
                &doctrine,
                &startup_policies,
                &operations,
                &roles,
            ),
            governing_order,
            operations,
            roles,
            doctrine,
            startup_policies,
            run_modes,
            mode_defaults,
            mode_default_sources,
            stacks,
            policy_families,
            recipes,
            repository_verifier_capabilities,
        })
        .map_err(SemanticError::from)
}

#[cfg(test)]
#[path = "../../tests/internal/semantic/projection.rs"]
mod tests;
