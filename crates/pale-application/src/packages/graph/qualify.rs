//! Authored-link qualification and global governing contribution admission.

use super::*;

/// Qualify one parsed resource against its owning release and declaration graph.
pub(super) fn qualify_resource(
    source: ParsedResource,
    resource_ref: &PackageResourceRefV1,
    owner: &RetainedPackageRelease,
    declarations: &BTreeMap<PackageResourceUse, PackageResourceV1>,
) -> Result<PackageResource, PackageError> {
    let admitted = match source {
        ParsedResource::Doctrine(record) => PackageResource::Doctrine {
            description: record.description,
            placement: record.placement,
            extends: qualify_many(
                &record.extends,
                resource_ref,
                owner,
                declarations,
                PackageResourceKindV1::Doctrine,
                ResourceLinkRelation::DoctrineExtends,
            )?,
            requires: qualify_many(
                &record.requires,
                resource_ref,
                owner,
                declarations,
                PackageResourceKindV1::Doctrine,
                ResourceLinkRelation::DoctrineRequires,
            )?,
            see_also: qualify_many(
                &record.see_also,
                resource_ref,
                owner,
                declarations,
                PackageResourceKindV1::Doctrine,
                ResourceLinkRelation::DoctrineSeeAlso,
            )?,
            official_sources: record.official_sources,
        },
        ParsedResource::OperationArchetype(record) => PackageResource::OperationArchetype {
            description: record.description,
            extends: qualify_many(
                &record.extends,
                resource_ref,
                owner,
                declarations,
                PackageResourceKindV1::OperationArchetype,
                ResourceLinkRelation::OperationArchetypeExtends,
            )?,
            doctrine: qualify_many(
                &record.doctrine_refs,
                resource_ref,
                owner,
                declarations,
                PackageResourceKindV1::Doctrine,
                ResourceLinkRelation::OperationArchetypeDoctrine,
            )?,
        },
        ParsedResource::Operation(record) => PackageResource::Operation {
            description: record.description,
            extends: qualify_many(
                &record.extends,
                resource_ref,
                owner,
                declarations,
                PackageResourceKindV1::OperationArchetype,
                ResourceLinkRelation::OperationExtends,
            )?,
            doctrine: qualify_many(
                &record.doctrine_refs,
                resource_ref,
                owner,
                declarations,
                PackageResourceKindV1::Doctrine,
                ResourceLinkRelation::OperationDoctrine,
            )?,
            roles: qualify_many(
                &record.roles,
                resource_ref,
                owner,
                declarations,
                PackageResourceKindV1::Role,
                ResourceLinkRelation::OperationRole,
            )?,
            outputs: record.outputs,
            requirements: record
                .capability_requirements
                .into_iter()
                .map(|requirement| {
                    qualify_requirement(requirement, resource_ref, owner, declarations)
                })
                .collect::<Result<_, _>>()?,
            capability_policy: record
                .capability_policy
                .as_ref()
                .map(|link| {
                    check_link(
                        link,
                        resource_ref,
                        owner,
                        declarations,
                        Some(PackageResourceKindV1::CapabilityPolicy),
                        ResourceLinkRelation::OperationCapabilityPolicy,
                    )
                })
                .transpose()?,
        },
        ParsedResource::RoleArchetype(record) => PackageResource::RoleArchetype {
            description: record.description,
            extends: qualify_many(
                &record.extends,
                resource_ref,
                owner,
                declarations,
                PackageResourceKindV1::RoleArchetype,
                ResourceLinkRelation::RoleArchetypeExtends,
            )?,
            doctrine: qualify_many(
                &record.doctrine_refs,
                resource_ref,
                owner,
                declarations,
                PackageResourceKindV1::Doctrine,
                ResourceLinkRelation::RoleArchetypeDoctrine,
            )?,
        },
        ParsedResource::Role(record) => PackageResource::Role {
            description: record.description,
            extends: qualify_many(
                &record.extends,
                resource_ref,
                owner,
                declarations,
                PackageResourceKindV1::RoleArchetype,
                ResourceLinkRelation::RoleExtends,
            )?,
            inherited_lenses: qualify_many(
                &record.inherits_lenses,
                resource_ref,
                owner,
                declarations,
                PackageResourceKindV1::Role,
                ResourceLinkRelation::RoleInheritedLens,
            )?,
            doctrine: qualify_many(
                &record.doctrine_refs,
                resource_ref,
                owner,
                declarations,
                PackageResourceKindV1::Doctrine,
                ResourceLinkRelation::RoleDoctrine,
            )?,
            stacks: qualify_many(
                &record.applies_to,
                resource_ref,
                owner,
                declarations,
                PackageResourceKindV1::Stack,
                ResourceLinkRelation::RoleStack,
            )?,
        },
        ParsedResource::Stack(record) => PackageResource::Stack {
            description: record.description,
            extends: qualify_many(
                &record.extends,
                resource_ref,
                owner,
                declarations,
                PackageResourceKindV1::Stack,
                ResourceLinkRelation::StackExtends,
            )?,
            doctrine: qualify_many(
                &record.doctrine_refs,
                resource_ref,
                owner,
                declarations,
                PackageResourceKindV1::Doctrine,
                ResourceLinkRelation::StackDoctrine,
            )?,
            role_doctrine: qualify_many(
                &record.role_doctrine_refs,
                resource_ref,
                owner,
                declarations,
                PackageResourceKindV1::Doctrine,
                ResourceLinkRelation::StackRoleDoctrine,
            )?,
            official_sources: record.official_sources,
        },
        ParsedResource::RunMode(record) => PackageResource::RunMode {
            description: record.description,
            axis: record.axis,
            aliases: record.aliases,
            exact_triggers: record.exact_triggers,
            extends: qualify_many(
                &record.extends,
                resource_ref,
                owner,
                declarations,
                PackageResourceKindV1::RunMode,
                ResourceLinkRelation::RunModeExtends,
            )?,
            scope: record.scope,
        },
        ParsedResource::PolicyFamily(record) => PackageResource::PolicyFamily {
            title: record.title,
            purpose: record.purpose,
        },
        ParsedResource::GoverningOrder { .. } => PackageResource::GoverningOrder,
        ParsedResource::CapabilityPolicy { .. } => PackageResource::CapabilityPolicy,
        ParsedResource::ToolRecipe { .. } => PackageResource::ToolRecipe,
    };
    Ok(admitted)
}

fn qualify_requirement(
    requirement: OperationCapabilityRequirement,
    owner_resource: &PackageResourceRefV1,
    owner: &RetainedPackageRelease,
    declarations: &BTreeMap<PackageResourceUse, PackageResourceV1>,
) -> Result<AdmittedOperationRequirement, PackageError> {
    let recipe = requirement
        .recipe
        .as_ref()
        .map(|link| {
            check_link(
                link,
                owner_resource,
                owner,
                declarations,
                Some(PackageResourceKindV1::ToolRecipe),
                ResourceLinkRelation::OperationRequirementRecipe,
            )
        })
        .transpose()?;
    let policy = match requirement.policy {
        CapabilityRequirementPolicy::RequiredBlock => AdmittedRequirementPolicy::RequiredBlock,
        CapabilityRequirementPolicy::RequiredDegrade {
            fallback,
            degraded_outcome,
        } => AdmittedRequirementPolicy::RequiredDegrade {
            fallback: qualify_fallback(fallback, owner_resource, owner, declarations)?,
            degraded_outcome,
        },
        CapabilityRequirementPolicy::Optional { fallback } => AdmittedRequirementPolicy::Optional {
            fallback: qualify_fallback(fallback, owner_resource, owner, declarations)?,
        },
    };
    Ok(AdmittedOperationRequirement {
        id: requirement.id,
        description: requirement.description,
        capability: requirement.capability,
        recipe,
        effect_class: requirement.effect_class,
        policy,
        failure_posture: requirement.failure_posture,
    })
}

fn qualify_fallback(
    fallback: CapabilityFallback,
    owner_resource: &PackageResourceRefV1,
    owner: &RetainedPackageRelease,
    declarations: &BTreeMap<PackageResourceUse, PackageResourceV1>,
) -> Result<AdmittedFallback, PackageError> {
    let recipe = fallback
        .recipe
        .as_ref()
        .map(|link| {
            check_link(
                link,
                owner_resource,
                owner,
                declarations,
                Some(PackageResourceKindV1::ToolRecipe),
                ResourceLinkRelation::OperationFallbackRecipe,
            )
        })
        .transpose()?;
    Ok(AdmittedFallback {
        guidance: fallback.guidance,
        recipe,
    })
}

fn qualify_many(
    links: &[PackageResourceLinkV1],
    from: &PackageResourceRefV1,
    owner: &RetainedPackageRelease,
    declarations: &BTreeMap<PackageResourceUse, PackageResourceV1>,
    expected: PackageResourceKindV1,
    relation: ResourceLinkRelation,
) -> Result<Vec<PackageResourceRefV1>, PackageError> {
    links
        .iter()
        .map(|link| check_link(link, from, owner, declarations, Some(expected), relation))
        .collect()
}

pub(super) fn check_link(
    link: &PackageResourceLinkV1,
    from: &PackageResourceRefV1,
    owner: &RetainedPackageRelease,
    declarations: &BTreeMap<PackageResourceUse, PackageResourceV1>,
    expected: Option<PackageResourceKindV1>,
    relation: ResourceLinkRelation,
) -> Result<PackageResourceRefV1, PackageError> {
    check_link_against(
        link,
        from,
        &owner.manifest().package.dependencies,
        declarations,
        expected,
        relation,
    )
}

/// Qualify one link against an exact direct-dependency inventory.
///
/// Keeping this owner law pure makes the complete local/external matrix
/// directly testable without manufacturing filesystem or retained-source state.
fn check_link_against(
    link: &PackageResourceLinkV1,
    from: &PackageResourceRefV1,
    direct_dependencies: &[pale_core::records::PackageDependencyV1],
    declarations: &BTreeMap<PackageResourceUse, PackageResourceV1>,
    expected: Option<PackageResourceKindV1>,
    relation: ResourceLinkRelation,
) -> Result<PackageResourceRefV1, PackageError> {
    let target = link.qualify(&from.package_id);
    match link {
        PackageResourceLinkV1::Local { .. } => {}
        PackageResourceLinkV1::External { resource } if resource.package_id == from.package_id => {
            return Err(invalid_link(
                from,
                &target,
                relation,
                ResourceLinkViolation::SelfExternal,
            ));
        }
        PackageResourceLinkV1::External { resource } => {
            if !direct_dependencies
                .iter()
                .any(|dependency| dependency.id == resource.package_id)
            {
                return Err(invalid_link(
                    from,
                    &target,
                    relation,
                    ResourceLinkViolation::NotDirectDependency,
                ));
            }
        }
    }
    let declaration = declaration_for_use(declarations, &target, expected)
        .map_err(|reason| invalid_link(from, &target, relation, reason))?;
    if matches!(link, PackageResourceLinkV1::External { .. })
        && declaration.exposure != PackageResourceExposureV1::Exported
    {
        return Err(invalid_link(
            from,
            &target,
            relation,
            ResourceLinkViolation::InternalTarget,
        ));
    }
    Ok(target)
}

/// Resolve a consuming field's kind without merging another use's visibility.
pub(super) fn declaration_for_use<'a>(
    declarations: &'a BTreeMap<PackageResourceUse, PackageResourceV1>,
    target: &PackageResourceRefV1,
    expected: Option<PackageResourceKindV1>,
) -> Result<&'a PackageResourceV1, ResourceLinkViolation> {
    if let Some(kind) = expected {
        return declarations
            .get(&PackageResourceUse::new(target.clone(), kind))
            .ok_or_else(|| {
                if declarations.keys().any(|usage| &usage.resource == target) {
                    ResourceLinkViolation::WrongKind
                } else {
                    ResourceLinkViolation::MissingTarget
                }
            });
    }
    let mut matches = declarations
        .iter()
        .filter(|(usage, _)| &usage.resource == target);
    match (matches.next(), matches.next()) {
        (Some((_, declaration)), None) => Ok(declaration),
        (None, _) => Err(ResourceLinkViolation::MissingTarget),
        _ => Err(ResourceLinkViolation::AmbiguousTarget),
    }
}

fn invalid_link(
    from: &PackageResourceRefV1,
    target: &PackageResourceRefV1,
    relation: ResourceLinkRelation,
    reason: ResourceLinkViolation,
) -> PackageError {
    PackageError::InvalidResourceLink {
        from: from.clone(),
        target: target.clone(),
        relation,
        reason,
    }
}

pub(super) fn validate_operation_procedures(
    resources: &BTreeMap<PackageResourceUse, AdmittedResourceNode>,
    policies: &BTreeMap<PackageResourceRefV1, CapabilityPolicy>,
    recipes: &BTreeMap<PackageResourceRefV1, ToolRecipe>,
) -> Result<(), PackageError> {
    let mut diagnostics = PackageDiagnosticAccumulator::default();
    for (operation, node) in resources {
        let PackageResource::Operation {
            requirements,
            capability_policy,
            ..
        } = &node.resource
        else {
            continue;
        };
        match (requirements.is_empty(), capability_policy) {
            (true, None) | (false, Some(_)) => {}
            (true, Some(_)) => {
                diagnostics.push(PackageError::InvalidOperationProcedure {
                    operation: operation.resource.clone(),
                    reason: OperationProcedureViolation::UnusedCapabilityCeiling,
                });
                continue;
            }
            (false, None) => {
                diagnostics.push(PackageError::InvalidOperationProcedure {
                    operation: operation.resource.clone(),
                    reason: OperationProcedureViolation::MissingCapabilityCeiling,
                });
                continue;
            }
        }
        let Some(policy_ref) = capability_policy else {
            continue;
        };
        let Some(policy) = policies.get(policy_ref) else {
            diagnostics.push(PackageError::InvalidOperationProcedure {
                operation: operation.resource.clone(),
                reason: OperationProcedureViolation::MissingPolicy,
            });
            continue;
        };
        for requirement in requirements {
            if !policy
                .permitted_capabilities
                .contains(&requirement.capability)
                || policy
                    .forbidden_capabilities
                    .contains(&requirement.capability)
                || !policy.permitted_effects.contains(&requirement.effect_class)
                || policy.forbidden_effects.contains(&requirement.effect_class)
            {
                diagnostics.push(PackageError::InvalidOperationProcedure {
                    operation: operation.resource.clone(),
                    reason: OperationProcedureViolation::RequirementExceedsPolicy,
                });
            }
            for recipe_ref in requirement
                .recipe
                .iter()
                .chain(requirement_policy_recipe(&requirement.policy))
            {
                let Some(recipe) = recipes.get(recipe_ref) else {
                    diagnostics.push(PackageError::InvalidOperationProcedure {
                        operation: operation.resource.clone(),
                        reason: OperationProcedureViolation::MissingRecipe,
                    });
                    continue;
                };
                if recipe.capability != requirement.capability
                    || recipe.effect_class != requirement.effect_class
                {
                    diagnostics.push(PackageError::InvalidOperationProcedure {
                        operation: operation.resource.clone(),
                        reason: OperationProcedureViolation::RecipeMismatch,
                    });
                }
            }
        }
    }
    diagnostics.finish(())
}

#[cfg(test)]
#[path = "../../../tests/internal/packages/graph/qualify.rs"]
mod tests;
