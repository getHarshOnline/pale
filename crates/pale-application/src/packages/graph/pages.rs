//! Lower one page declaration into its uses while retaining one source body.

use std::ops::Range;

use pale_core::paths::OwnedConfinedRelativePath;
use pale_core::records::PackageResourceIdV1;

use super::authoring::{push_link_array, PackageAuthoringReference, PackageAuthoringSource};
use super::*;

/// Source-only information consumed during qualification and editor indexing.
#[derive(Debug, Clone)]
pub(super) struct PageUseSource {
    pub(super) header: std::sync::Arc<pale_core::records::PackagePageV1>,
    pub(super) path: OwnedConfinedRelativePath,
    pub(super) body: Range<usize>,
    pub(super) required: Vec<PackageResourceLinkV1>,
    pub(super) related: Vec<PackageResourceLinkV1>,
    pub(super) references: Vec<PackageAuthoringReference>,
}

pub(super) fn parse(
    text: &str,
    source: &std::path::Path,
    package: &PackageIdV1,
    path: &OwnedConfinedRelativePath,
) -> Result<Vec<(PackageResourceV1, ParsedResource, PackageAuthoringSource)>, PackageError> {
    let page = crate::packages::page::read_page(text, source)?;
    let header = std::sync::Arc::new(page.header);
    let id = path
        .as_str()
        .strip_suffix(".md")
        .and_then(|value| PackageResourceIdV1::parse(value).ok())
        .ok_or_else(|| PackageError::ResourceDeclarationMismatch {
            package: package.clone(),
            source_path: path.to_string(),
            resources: Vec::new(),
        })?;
    let mut uses = Vec::new();
    if header.is_guide() {
        uses.push((
            "guide",
            header.guide.as_ref().and_then(|usage| usage.exposure),
            Vec::new(),
            ParsedResource::Doctrine(DoctrineEntry {
                id: id.clone(),
                description: header.description.clone(),
                doctrine: path.clone(),
                placement: DoctrinePlacement::StandaloneJudgment,
                extends: Vec::new(),
                requires: Vec::new(),
                see_also: Vec::new(),
                official_sources: header.official_sources.clone(),
            }),
        ));
    }
    if let Some(task) = &header.task {
        uses.push((
            "task",
            task.exposure,
            task.uses.clone(),
            ParsedResource::Operation(OnDemandOperation {
                id: id.clone(),
                description: header.description.clone(),
                doctrine: path.clone(),
                extends: Vec::new(),
                doctrine_refs: Vec::new(),
                roles: task.roles.clone(),
                outputs: task.outputs.clone(),
                capability_requirements: task.capability_requirements.clone(),
                capability_policy: task.capability_policy.clone(),
            }),
        ));
    }
    if let Some(role) = &header.role {
        uses.push((
            "role",
            role.exposure,
            role.uses.clone(),
            ParsedResource::Role(SpecialistRole {
                id: id.clone(),
                description: header.description.clone(),
                doctrine: path.clone(),
                extends: Vec::new(),
                doctrine_refs: Vec::new(),
                inherits_lenses: role.extends.clone(),
                applies_to: role.applies_to.clone(),
            }),
        ));
    }
    if let Some(policy) = &header.policy {
        uses.push((
            "policy",
            policy.exposure,
            Vec::new(),
            ParsedResource::PolicyFamily(PolicyFamilyV1 {
                id: id.clone(),
                title: header.description.clone(),
                purpose: header.description.clone(),
                body: path.clone(),
            }),
        ));
    }
    if let Some(stack) = &header.stack {
        uses.push((
            "stack",
            stack.exposure,
            stack.uses.clone(),
            ParsedResource::Stack(Stack {
                id: id.clone(),
                description: header.description.clone(),
                extends: stack.extends.clone(),
                doctrine_refs: Vec::new(),
                role_doctrine_refs: stack.role_uses.clone(),
                official_sources: header.official_sources.clone(),
            }),
        ));
    }
    let mut rows = Vec::new();
    for (name, exposure, extra_reading, resource) in uses {
        use crate::workspace::AuthoringInstancePathSegment::Field;
        let content_path = if name == "guide" && header.guide.is_none() {
            Vec::new()
        } else {
            vec![Field(name.into())]
        };
        let mut references =
            super::authoring::package_resource_references(&resource, package, &content_path);
        // These two authored fields have different names from the engine's
        // composed role/stack records. Keep editor positions in the header.
        for reference in &mut references {
            for segment in &mut reference.path {
                if let Field(field) = segment {
                    if name == "role" && field == "inherits_lenses" {
                        *field = "extends".into();
                    }
                    if name == "stack" && field == "role_doctrine_refs" {
                        *field = "role_uses".into();
                    }
                }
            }
        }
        push_link_array(
            &mut references,
            package,
            &[],
            "uses",
            &header.uses,
            Some(PackageResourceKindV1::Doctrine),
        );
        push_link_array(
            &mut references,
            package,
            &[],
            "see_also",
            &header.see_also,
            Some(PackageResourceKindV1::Doctrine),
        );
        if !extra_reading.is_empty() {
            push_link_array(
                &mut references,
                package,
                &content_path,
                "uses",
                &extra_reading,
                Some(PackageResourceKindV1::Doctrine),
            );
        }
        let required = header.uses.iter().cloned().chain(extra_reading).collect();
        let declaration = PackageResourceV1 {
            id: id.clone(),
            kind: resource.kind(),
            definition_source: path.clone(),
            exposure: exposure.unwrap_or(header.exposure),
        };
        rows.push((
            declaration,
            resource,
            PackageAuthoringSource {
                source_label: source.to_string_lossy().into_owned(),
                definition_path: content_path.clone(),
                content_path,
                page: Some(PageUseSource {
                    header: header.clone(),
                    path: path.clone(),
                    body: page.parts.markdown.clone(),
                    required,
                    related: header.see_also.clone(),
                    references,
                }),
            },
        ));
    }
    Ok(rows)
}

pub(super) fn qualify_reading(
    page: &PageUseSource,
    from: &PackageResourceRefV1,
    owner: &RetainedPackageRelease,
    declarations: &BTreeMap<PackageResourceUse, PackageResourceV1>,
) -> Result<types::AdmittedPageReading, PackageError> {
    let qualify = |links: &[PackageResourceLinkV1], relation| {
        links
            .iter()
            .map(|link| {
                qualify::check_link(
                    link,
                    from,
                    owner,
                    declarations,
                    Some(PackageResourceKindV1::Doctrine),
                    relation,
                )
            })
            .collect::<Result<Vec<_>, _>>()
    };
    Ok(types::AdmittedPageReading {
        official_sources: page.header.official_sources.clone(),
        required: qualify(&page.required, ResourceLinkRelation::DoctrineRequires)?,
        related: qualify(&page.related, ResourceLinkRelation::DoctrineSeeAlso)?,
    })
}

#[cfg(all(test, unix))]
#[path = "../../../tests/internal/packages/graph/pages.rs"]
mod tests;
