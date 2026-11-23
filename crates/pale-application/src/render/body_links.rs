//! Project qualified Package navigation relative to the actual host file kind.

use super::semantic_content::operation_is_model_facing;
use std::sync::Arc;

use super::{
    semantic_helpers::{doctrine_skill_name, host_name, operation_name, stack_name},
    RenderError,
};
use crate::packages::{PackageResourceUse, ResourceTextReference, TextRecipe};
use crate::semantic::RepositorySemanticPlan;
use pale_core::records::{PackageResourceKindV1, PackageResourceRefV1};

pub(super) fn navigation_at(
    resource: &PackageResourceRefV1,
    kind: PackageResourceKindV1,
    semantics: &RepositorySemanticPlan,
    origin_file: &str,
    skills_root: &str,
) -> Result<String, RenderError> {
    page_navigation(
        resource,
        kind,
        semantics,
        &skills_prefix(origin_file, skills_root),
    )
}

/// Keep required reading explicit without composing its prose into every use.
pub(super) fn page_navigation(
    resource: &PackageResourceRefV1,
    kind: PackageResourceKindV1,
    semantics: &RepositorySemanticPlan,
    prefix: &str,
) -> Result<String, RenderError> {
    let Some(reading) = semantics
        .page_reading
        .get(&PackageResourceUse::new(resource.clone(), kind))
    else {
        return Ok(String::new());
    };
    let mut navigation = String::new();
    if !reading.required.is_empty() {
        navigation.push_str("## Read first\n\n");
        for target in &reading.required {
            let guide = semantics
                .doctrine
                .iter()
                .find(|guide| guide.record.id == *target)
                .ok_or_else(|| {
                    RenderError::invalid_input(
                        "required reading",
                        format!("page `{resource}` requires missing guide `{target}`"),
                    )
                })?;
            let href = crate::markdown::escape_link_destination(&format!(
                "{prefix}{}/SKILL.md",
                doctrine_skill_name(target)
            ));
            navigation.push_str(&format!("- [{}]({href})\n", link_title(&guide.title)));
        }
        navigation.push('\n');
    }
    if !reading.related.is_empty() {
        navigation.push_str("## Related reading\n\n");
        for reference in &reading.related {
            navigation.push_str(&format!(
                "- {}\n",
                reading_reference(reference, semantics, prefix)
            ));
        }
        navigation.push('\n');
    }
    Ok(navigation)
}

pub(super) fn skills_prefix(origin_file: &str, skills_root: &str) -> String {
    let parent_count = origin_file.bytes().filter(|byte| *byte == b'/').count();
    format!("{}{skills_root}/", "../".repeat(parent_count))
}

fn link_title(title: &str) -> String {
    title
        .replace('\\', "\\\\")
        .replace('[', "\\[")
        .replace(']', "\\]")
}

pub(super) fn skill_body(
    recipe: &TextRecipe,
    semantics: &RepositorySemanticPlan,
) -> Result<Arc<String>, RenderError> {
    linked_body(recipe, semantics, "../")
}

pub(super) fn agent_body(
    recipe: &TextRecipe,
    semantics: &RepositorySemanticPlan,
) -> Result<Arc<String>, RenderError> {
    linked_body(recipe, semantics, "../skills/")
}

/// Runtime and internal-operation files have provider-qualified directories.
/// Resolve from their actual emitted file rather than assuming a skill folder.
pub(super) fn body_at(
    recipe: &TextRecipe,
    semantics: &RepositorySemanticPlan,
    origin_file: &str,
    skills_root: &str,
) -> Result<Arc<String>, RenderError> {
    let prefix = skills_prefix(origin_file, skills_root);
    linked_body(recipe, semantics, &prefix)
}

pub(super) fn linked_body(
    recipe: &TextRecipe,
    semantics: &RepositorySemanticPlan,
    prefix: &str,
) -> Result<Arc<String>, RenderError> {
    recipe.materialize_links(|target, fragment| {
        Ok(selected_href(target, fragment, semantics, prefix))
    })
}

/// Declared reading uses the same target metadata and installed-link resolution as prose.
pub(super) fn related_reading(
    reference: &ResourceTextReference,
    semantics: &RepositorySemanticPlan,
) -> String {
    reading_reference(reference, semantics, "../")
}

fn reading_reference(
    reference: &ResourceTextReference,
    semantics: &RepositorySemanticPlan,
    prefix: &str,
) -> String {
    match selected_href(
        &reference.target,
        reference.fragment.as_deref(),
        semantics,
        prefix,
    ) {
        Some(href) => {
            let title = link_title(&reference.title);
            format!(
                "[{title}]({href}), `{}`, `{}`",
                reference.target.package_id.as_str(),
                reference.path
            )
        }
        None => reference.location(),
    }
}

fn selected_href(
    target: &PackageResourceRefV1,
    fragment: Option<&str>,
    semantics: &RepositorySemanticPlan,
    prefix: &str,
) -> Option<String> {
    let name =
        if semantics
            .doctrine
            .iter()
            .any(|doctrine| doctrine.record.id == *target)
        {
            doctrine_skill_name(target)
        } else if let Some(operation) = semantics.operations.iter().find(|operation| {
            operation.record.id == *target && operation_is_model_facing(operation)
        }) {
            operation_name(operation)
        } else if semantics
            .policy_families
            .iter()
            .any(|family| family.id == *target)
        {
            host_name(target)
        } else if semantics
            .stacks
            .iter()
            .any(|stack| stack.id == *target && stack.body.is_some())
        {
            stack_name(target)
        } else {
            return None;
        };
    let mut href = format!("{prefix}{name}/SKILL.md");
    if let Some(fragment) = fragment {
        href.push('#');
        href.push_str(fragment);
    }
    Some(crate::markdown::escape_link_destination(&href))
}
