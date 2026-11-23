//! Editor coordinates derived from parsed Package declarations.

use super::*;

#[derive(Debug, Clone)]
pub(super) struct PackageAuthoringSource {
    pub(super) page: Option<super::pages::PageUseSource>,
    pub(super) source_label: String,
    pub(super) content_path: Vec<crate::workspace::AuthoringInstancePathSegment>,
    pub(super) definition_path: Vec<crate::workspace::AuthoringInstancePathSegment>,
}

#[derive(Debug, Clone)]
pub(super) struct PackageAuthoringReference {
    target: PackageResourceRefV1,
    expected: Option<PackageResourceKindV1>,
    pub(super) path: Vec<crate::workspace::AuthoringInstancePathSegment>,
}

pub(super) fn package_authoring_source(
    resource: &ParsedResource,
    ordinal: usize,
    source_label: String,
) -> PackageAuthoringSource {
    use crate::workspace::AuthoringInstancePathSegment::{Field, Index};

    let (content_path, has_authored_id) = match resource {
        ParsedResource::Doctrine(_) => (
            vec![
                Field("doctrine".into()),
                Field("entry".into()),
                Index(ordinal),
            ],
            true,
        ),
        ParsedResource::OperationArchetype(_) => (
            vec![
                Field("operations".into()),
                Field("archetype".into()),
                Index(ordinal),
            ],
            true,
        ),
        ParsedResource::Operation(_) => (
            vec![
                Field("operations".into()),
                Field("operation".into()),
                Index(ordinal),
            ],
            true,
        ),
        ParsedResource::RoleArchetype(_) => (
            vec![
                Field("roles".into()),
                Field("archetype".into()),
                Index(ordinal),
            ],
            true,
        ),
        ParsedResource::Role(_) => (
            vec![Field("roles".into()), Field("role".into()), Index(ordinal)],
            true,
        ),
        ParsedResource::Stack(_) => (
            vec![
                Field("stacks".into()),
                Field("stack".into()),
                Index(ordinal),
            ],
            true,
        ),
        ParsedResource::RunMode(_) => (
            vec![
                Field("run_modes".into()),
                Field("mode".into()),
                Index(ordinal),
            ],
            true,
        ),
        ParsedResource::PolicyFamily(_) => (Vec::new(), true),
        ParsedResource::GoverningOrder { .. } => (vec![Field("governing_order".into())], false),
        ParsedResource::CapabilityPolicy { .. } | ParsedResource::ToolRecipe { .. } => {
            (Vec::new(), false)
        }
    };
    let definition_path = if has_authored_id {
        nested_authoring_path(&content_path, [Field("id".into())])
    } else {
        Vec::new()
    };
    PackageAuthoringSource {
        page: None,
        source_label,
        content_path,
        definition_path,
    }
}

pub(super) fn package_resource_references(
    source: &ParsedResource,
    local_package: &PackageIdV1,
    base: &[crate::workspace::AuthoringInstancePathSegment],
) -> Vec<PackageAuthoringReference> {
    let mut references = Vec::new();
    match source {
        ParsedResource::Doctrine(record) => {
            push_link_array(
                &mut references,
                local_package,
                base,
                "extends",
                &record.extends,
                Some(PackageResourceKindV1::Doctrine),
            );
            push_link_array(
                &mut references,
                local_package,
                base,
                "requires",
                &record.requires,
                Some(PackageResourceKindV1::Doctrine),
            );
            push_link_array(
                &mut references,
                local_package,
                base,
                "see_also",
                &record.see_also,
                Some(PackageResourceKindV1::Doctrine),
            );
        }
        ParsedResource::OperationArchetype(record) => {
            push_link_array(
                &mut references,
                local_package,
                base,
                "extends",
                &record.extends,
                Some(PackageResourceKindV1::OperationArchetype),
            );
            push_link_array(
                &mut references,
                local_package,
                base,
                "doctrine_refs",
                &record.doctrine_refs,
                Some(PackageResourceKindV1::Doctrine),
            );
        }
        ParsedResource::Operation(record) => {
            push_link_array(
                &mut references,
                local_package,
                base,
                "extends",
                &record.extends,
                Some(PackageResourceKindV1::OperationArchetype),
            );
            push_link_array(
                &mut references,
                local_package,
                base,
                "doctrine_refs",
                &record.doctrine_refs,
                Some(PackageResourceKindV1::Doctrine),
            );
            push_link_array(
                &mut references,
                local_package,
                base,
                "roles",
                &record.roles,
                Some(PackageResourceKindV1::Role),
            );
            for (requirement_index, requirement) in
                record.capability_requirements.iter().enumerate()
            {
                let requirement_path = nested_authoring_path(
                    base,
                    [
                        authoring_field("capability_requirements"),
                        authoring_index(requirement_index),
                    ],
                );
                if let Some(recipe) = &requirement.recipe {
                    push_link(
                        &mut references,
                        local_package,
                        nested_authoring_path(&requirement_path, [authoring_field("recipe")]),
                        recipe,
                        Some(PackageResourceKindV1::ToolRecipe),
                    );
                }
                if let Some(recipe) = requirement.fallback_recipe() {
                    push_link(
                        &mut references,
                        local_package,
                        nested_authoring_path(
                            &requirement_path,
                            [
                                authoring_field("policy"),
                                authoring_field("fallback"),
                                authoring_field("recipe"),
                            ],
                        ),
                        recipe,
                        Some(PackageResourceKindV1::ToolRecipe),
                    );
                }
            }
            if let Some(policy) = &record.capability_policy {
                push_link(
                    &mut references,
                    local_package,
                    nested_authoring_path(base, [authoring_field("capability_policy")]),
                    policy,
                    Some(PackageResourceKindV1::CapabilityPolicy),
                );
            }
        }
        ParsedResource::RoleArchetype(record) => {
            push_link_array(
                &mut references,
                local_package,
                base,
                "extends",
                &record.extends,
                Some(PackageResourceKindV1::RoleArchetype),
            );
            push_link_array(
                &mut references,
                local_package,
                base,
                "doctrine_refs",
                &record.doctrine_refs,
                Some(PackageResourceKindV1::Doctrine),
            );
        }
        ParsedResource::Role(record) => {
            push_link_array(
                &mut references,
                local_package,
                base,
                "extends",
                &record.extends,
                Some(PackageResourceKindV1::RoleArchetype),
            );
            push_link_array(
                &mut references,
                local_package,
                base,
                "inherits_lenses",
                &record.inherits_lenses,
                Some(PackageResourceKindV1::Role),
            );
            push_link_array(
                &mut references,
                local_package,
                base,
                "doctrine_refs",
                &record.doctrine_refs,
                Some(PackageResourceKindV1::Doctrine),
            );
            push_link_array(
                &mut references,
                local_package,
                base,
                "applies_to",
                &record.applies_to,
                Some(PackageResourceKindV1::Stack),
            );
        }
        ParsedResource::Stack(record) => {
            push_link_array(
                &mut references,
                local_package,
                base,
                "extends",
                &record.extends,
                Some(PackageResourceKindV1::Stack),
            );
            push_link_array(
                &mut references,
                local_package,
                base,
                "doctrine_refs",
                &record.doctrine_refs,
                Some(PackageResourceKindV1::Doctrine),
            );
            push_link_array(
                &mut references,
                local_package,
                base,
                "role_doctrine_refs",
                &record.role_doctrine_refs,
                Some(PackageResourceKindV1::Doctrine),
            );
        }
        ParsedResource::RunMode(record) => {
            push_link_array(
                &mut references,
                local_package,
                base,
                "extends",
                &record.extends,
                Some(PackageResourceKindV1::RunMode),
            );
        }
        ParsedResource::GoverningOrder { record, .. } => {
            for (index, relation) in record.constraints.iter().enumerate() {
                let row = governing_row(base, "constraints", index);
                push_named_link(
                    &mut references,
                    local_package,
                    &row,
                    "scope",
                    &relation.scope.resource_link(),
                    relation.scope.kind(),
                );
                push_named_link(
                    &mut references,
                    local_package,
                    &row,
                    "subject",
                    &relation.subject.resource_link(),
                    relation.subject.kind(),
                );
                push_named_link(
                    &mut references,
                    local_package,
                    &row,
                    "policy",
                    &relation.policy,
                    Some(PackageResourceKindV1::CapabilityPolicy),
                );
            }
            for (index, relation) in record.obligations.iter().enumerate() {
                let row = governing_row(base, "obligations", index);
                push_named_link(
                    &mut references,
                    local_package,
                    &row,
                    "scope",
                    &relation.scope.resource_link(),
                    relation.scope.kind(),
                );
                push_named_link(
                    &mut references,
                    local_package,
                    &row,
                    "subject",
                    &relation.subject.resource_link(),
                    relation.subject.kind(),
                );
                push_named_link(
                    &mut references,
                    local_package,
                    &row,
                    "obligation",
                    &relation.obligation.resource_link(),
                    relation.obligation.kind(),
                );
            }
            for (index, relation) in record.exclusive_authority.iter().enumerate() {
                let row = governing_row(base, "exclusive_authority", index);
                push_named_link(
                    &mut references,
                    local_package,
                    &row,
                    "scope",
                    &relation.scope.resource_link(),
                    relation.scope.kind(),
                );
                push_named_link(
                    &mut references,
                    local_package,
                    &row,
                    "authority",
                    &relation.authority.resource_link(),
                    relation.authority.kind(),
                );
            }
            for (index, relation) in record.narrowing.iter().enumerate() {
                let row = governing_row(base, "narrowing", index);
                push_named_link(
                    &mut references,
                    local_package,
                    &row,
                    "scope",
                    &relation.scope.resource_link(),
                    relation.scope.kind(),
                );
                push_named_link(
                    &mut references,
                    local_package,
                    &row,
                    "narrower",
                    &relation.narrower.resource_link(),
                    relation.narrower.kind(),
                );
                push_named_link(
                    &mut references,
                    local_package,
                    &row,
                    "broader",
                    &relation.broader.resource_link(),
                    relation.broader.kind(),
                );
            }
            for (index, relation) in record.order.iter().enumerate() {
                let row = governing_row(base, "order", index);
                push_named_link(
                    &mut references,
                    local_package,
                    &row,
                    "scope",
                    &relation.scope.resource_link(),
                    relation.scope.kind(),
                );
                push_named_link(
                    &mut references,
                    local_package,
                    &row,
                    "before",
                    &relation.before.resource_link(),
                    relation.before.kind(),
                );
                push_named_link(
                    &mut references,
                    local_package,
                    &row,
                    "after",
                    &relation.after.resource_link(),
                    relation.after.kind(),
                );
            }
        }
        ParsedResource::PolicyFamily(_)
        | ParsedResource::CapabilityPolicy { .. }
        | ParsedResource::ToolRecipe { .. } => {}
    }
    references
}

pub(super) fn package_capability_references(
    source: &ParsedResource,
    base: &[crate::workspace::AuthoringInstancePathSegment],
) -> Vec<(
    pale_core::records::CapabilityIdV1,
    Vec<crate::workspace::AuthoringInstancePathSegment>,
)> {
    match source {
        ParsedResource::Operation(record) => record
            .capability_requirements
            .iter()
            .enumerate()
            .map(|(index, requirement)| {
                (
                    requirement.capability.clone(),
                    nested_authoring_path(
                        base,
                        [
                            authoring_field("capability_requirements"),
                            authoring_index(index),
                            authoring_field("capability"),
                        ],
                    ),
                )
            })
            .collect(),
        ParsedResource::CapabilityPolicy { record, .. } => [
            ("permitted_capabilities", &record.permitted_capabilities),
            ("forbidden_capabilities", &record.forbidden_capabilities),
        ]
        .into_iter()
        .flat_map(|(field, capabilities)| {
            capabilities.iter().enumerate().map(move |(index, value)| {
                (
                    value.clone(),
                    nested_authoring_path(base, [authoring_field(field), authoring_index(index)]),
                )
            })
        })
        .collect(),
        ParsedResource::ToolRecipe { record, .. } => vec![(
            record.capability.clone(),
            nested_authoring_path(base, [authoring_field("capability")]),
        )],
        ParsedResource::Doctrine(_)
        | ParsedResource::OperationArchetype(_)
        | ParsedResource::RoleArchetype(_)
        | ParsedResource::Role(_)
        | ParsedResource::Stack(_)
        | ParsedResource::RunMode(_)
        | ParsedResource::PolicyFamily(_)
        | ParsedResource::GoverningOrder { .. } => Vec::new(),
    }
}

pub(super) fn index_package_references(
    authoring: &mut crate::workspace::AuthoringSemanticIndex,
    declarations: &BTreeMap<PackageResourceUse, PackageResourceV1>,
    source_label: &str,
    references: Vec<PackageAuthoringReference>,
) {
    for reference in references {
        let declaration = super::qualify::declaration_for_use(
            declarations,
            &reference.target,
            reference.expected,
        )
        .expect("successful Package qualification proved the typed reference target");
        authoring.reference_package_resource(
            &reference.target,
            declaration.kind,
            source_label,
            reference.path,
        );
    }
}

pub(super) fn push_link_array(
    references: &mut Vec<PackageAuthoringReference>,
    local_package: &PackageIdV1,
    base: &[crate::workspace::AuthoringInstancePathSegment],
    field: &str,
    links: &[PackageResourceLinkV1],
    expected: Option<PackageResourceKindV1>,
) {
    for (index, link) in links.iter().enumerate() {
        push_link(
            references,
            local_package,
            nested_authoring_path(base, [authoring_field(field), authoring_index(index)]),
            link,
            expected,
        );
    }
}

fn push_named_link(
    references: &mut Vec<PackageAuthoringReference>,
    local_package: &PackageIdV1,
    base: &[crate::workspace::AuthoringInstancePathSegment],
    field: &str,
    link: &PackageResourceLinkV1,
    expected: Option<PackageResourceKindV1>,
) {
    push_link(
        references,
        local_package,
        nested_authoring_path(base, [authoring_field(field)]),
        link,
        expected,
    );
}

fn push_link(
    references: &mut Vec<PackageAuthoringReference>,
    local_package: &PackageIdV1,
    mut path: Vec<crate::workspace::AuthoringInstancePathSegment>,
    link: &PackageResourceLinkV1,
    expected: Option<PackageResourceKindV1>,
) {
    match link {
        PackageResourceLinkV1::Local { .. } => path.push(authoring_field("resource_id")),
        PackageResourceLinkV1::External { .. } => {
            path.push(authoring_field("resource"));
            path.push(authoring_field("resource_id"));
        }
    }
    references.push(PackageAuthoringReference {
        target: link.qualify(local_package),
        expected,
        path,
    });
}

fn governing_row(
    base: &[crate::workspace::AuthoringInstancePathSegment],
    field: &str,
    index: usize,
) -> Vec<crate::workspace::AuthoringInstancePathSegment> {
    nested_authoring_path(base, [authoring_field(field), authoring_index(index)])
}

fn authoring_field(value: &str) -> crate::workspace::AuthoringInstancePathSegment {
    crate::workspace::AuthoringInstancePathSegment::Field(value.to_owned())
}

const fn authoring_index(index: usize) -> crate::workspace::AuthoringInstancePathSegment {
    crate::workspace::AuthoringInstancePathSegment::Index(index)
}

fn nested_authoring_path<const N: usize>(
    base: &[crate::workspace::AuthoringInstancePathSegment],
    tail: [crate::workspace::AuthoringInstancePathSegment; N],
) -> Vec<crate::workspace::AuthoringInstancePathSegment> {
    base.iter().cloned().chain(tail).collect()
}

#[cfg(test)]
#[path = "../../../tests/internal/packages/graph/authoring.rs"]
mod tests;
