//! Orders policy families and renders their host-facing doctrine.

use crate::terminal_adapters::TerminalAdapterDescriptor;

use std::collections::{BTreeMap, BTreeSet};

use pale_core::records::{PackageResourceKindV1, PackageResourceRefV1, SemanticOrderPurposeV1};

use super::semantic_helpers::{host_name, short, yaml_string};
use super::RenderError;
use crate::semantic::{RepositorySemanticPlan, ResolvedPolicyFamily};

/// A policy family and disposition borrowed for one repository render.
pub(super) struct OrderedFamily<'a> {
    pub(super) family: &'a ResolvedPolicyFamily,
}

/// Order selected policy families by Package-authored presentation edges.
///
/// Independent nodes use their owner-qualified identity as a deterministic
/// tie-break. This linearization is a renderer projection; it is not persisted
/// as a new semantic relation.
pub(super) fn ordered_families<'a>(
    semantics: &'a RepositorySemanticPlan,
) -> Result<Vec<OrderedFamily<'a>>, RenderError> {
    let families = semantics
        .policy_families
        .iter()
        .map(|family| (family.id.clone(), family))
        .collect::<BTreeMap<_, _>>();
    let mut incoming = families
        .keys()
        .cloned()
        .map(|id| (id, 0_usize))
        .collect::<BTreeMap<_, _>>();
    let mut outgoing = BTreeMap::<PackageResourceRefV1, BTreeSet<PackageResourceRefV1>>::new();
    for relation in semantics.governing_order.order().iter().filter(|relation| {
        relation.purpose == SemanticOrderPurposeV1::Presentation
            && relation.before.kind == PackageResourceKindV1::PolicyFamily
            && relation.after.kind == PackageResourceKindV1::PolicyFamily
            && families.contains_key(&relation.before.resource)
            && families.contains_key(&relation.after.resource)
    }) {
        if outgoing
            .entry(relation.before.resource.clone())
            .or_default()
            .insert(relation.after.resource.clone())
        {
            *incoming
                .get_mut(&relation.after.resource)
                .expect("both relation endpoints were selected") += 1;
        }
    }
    let mut ready = incoming
        .iter()
        .filter_map(|(id, count)| (*count == 0).then_some(id.clone()))
        .collect::<BTreeSet<_>>();
    let mut ordered = Vec::with_capacity(families.len());
    while let Some(id) = ready.pop_first() {
        ordered.push(OrderedFamily {
            family: families[&id],
        });
        for dependent in outgoing.get(&id).into_iter().flatten() {
            let count = incoming
                .get_mut(dependent)
                .expect("selected edge target has an indegree");
            *count -= 1;
            if *count == 0 {
                ready.insert(dependent.clone());
            }
        }
    }
    if ordered.len() != families.len() {
        return Err(RenderError::invalid_input(
            "governing_order",
            "selected policy-family presentation order contains a cycle",
        ));
    }
    Ok(ordered)
}

#[cfg(test)]
#[path = "../../tests/internal/render/family_content.rs"]
mod tests;

/// Render one selected capability family from resolved semantic inputs.
pub(super) fn skill_content(
    family: &ResolvedPolicyFamily,
    digest: &str,
    host: TerminalAdapterDescriptor,
    semantics: &RepositorySemanticPlan,
) -> Result<String, RenderError> {
    let invocation = if host == TerminalAdapterDescriptor::claude_code() {
        "disable-model-invocation: true\n"
    } else {
        ""
    };
    let body = super::body_links::skill_body(&family.body, semantics)?;
    let official =
        super::semantic_helpers::official_source_guidance(&super::semantic_helpers::page_sources(
            &family.id,
            pale_core::records::PackageResourceKindV1::PolicyFamily,
            semantics,
            &[],
        ));
    let navigation = super::body_links::page_navigation(
        &family.id,
        pale_core::records::PackageResourceKindV1::PolicyFamily,
        semantics,
        "../",
    )?;
    let document = crate::markdown::project(&body);
    let title = document.title(&body).unwrap_or(&family.title);
    let content = &body[document.body_range()];
    super::closing_questions::finish(&family.id, pale_core::records::PackageResourceKindV1::PolicyFamily, semantics, "../", format!(
        "---\nname: {name}\ndescription: {desc}\n{invocation}---\n<!-- pale: generated family={id} profile={dg} -- edit the owning Package and recompile; do not edit here. -->\n\n# {title}\n\nPackage: `{package}`\n\n{navigation}{body}\n\n{official}",
        name = host_name(&family.id),
        desc = yaml_string(&describe(&family.purpose)),
        invocation = invocation,
        id = crate::semantic::resource_label(&family.id),
        dg = short(digest),
        title = title,
        package = family.id.package_id,
        body = content.trim_end(),
    ))
}

/// Normalizes listing text and enforces the renderer's 1,200-character ceiling.
fn describe(text: &str) -> String {
    let one_line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let clamped: String = one_line.chars().take(1200).collect();
    clamped
}
