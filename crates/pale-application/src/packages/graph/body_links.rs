//! Consume local Markdown addresses while manifest/body ownership is available.
//!
//! Navigation never selects a resource. Header-authored pages may link directly
//! to another page; older declarations name their readable doctrine targets.
//! Exact source bytes survive; only the qualified target enters rendering.

use super::*;
use crate::packages::{ResourceTextLink, ResourceTextReference};
use pale_core::records::ArtifactDigest;

fn has_uri_scheme(href: &str) -> bool {
    let Some((scheme, _)) = href.split_once(':') else {
        return false;
    };
    let mut bytes = scheme.bytes();
    bytes.next().is_some_and(|byte| byte.is_ascii_alphabetic())
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.'))
}

fn local_path(base: &str, href: &str) -> Option<String> {
    if href.starts_with('/') || href.contains(['\\', '?']) {
        return None;
    }
    let mut parts = base.split('/').collect::<Vec<_>>();
    parts.pop();
    for part in href.split('/') {
        match part {
            "." => {}
            ".." => {
                parts.pop()?;
            }
            "" => return None,
            part => parts.push(part),
        }
    }
    Some(parts.join("/"))
}

pub(super) fn qualify_body_links(
    owners: &BTreeMap<(PackageIdV1, String), PackageResourceRefV1>,
    resources: &BTreeMap<PackageResourceUse, AdmittedResourceNode>,
    bodies: &mut BTreeMap<PackageResourceRefV1, AdmittedMarkdown>,
) -> Result<(), PackageError> {
    let references = owners
        .iter()
        .map(|((_, path), target)| {
            (
                target.clone(),
                ResourceTextReference {
                    title: bodies[target]
                        .title()
                        .unwrap_or(target.resource_id.as_str())
                        .to_owned(),
                    path: path.clone(),
                    target: target.clone(),
                    fragment: None,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut diagnostics = PackageDiagnosticAccumulator::default();
    for ((package, path), from) in owners {
        let body = &bodies[from];
        let result = (|| {
            let mut links = Vec::new();
            for link in body.authored_links() {
                let href = link.destination.as_str();
                if href.is_empty() || href.starts_with("//") || has_uri_scheme(href) {
                    continue;
                }
                let error = |reason| PackageError::InvalidMarkdownLink {
                    from: from.clone(),
                    destination_sha256: ArtifactDigest::hash_bytes(href.as_bytes()),
                    reason,
                };
                if let Some(fragment) = href.strip_prefix('#') {
                    if !body.has_fragment(fragment) {
                        return Err(error(MarkdownLinkViolation::MissingHeading));
                    }
                    continue;
                }
                let range = link
                    .range
                    .as_ref()
                    .filter(|range| body.link_is_in_body(range))
                    .ok_or_else(|| error(MarkdownLinkViolation::UnsupportedSyntax))?;
                let link_range = link
                    .link_range
                    .as_ref()
                    .filter(|range| body.link_is_in_body(range))
                    .ok_or_else(|| error(MarkdownLinkViolation::UnsupportedSyntax))?;
                let (destination, fragment) = href
                    .split_once('#')
                    .map_or((href, None), |(path, fragment)| {
                        (path, Some(fragment.to_owned()))
                    });
                let target_path = local_path(path, destination)
                    .ok_or_else(|| error(MarkdownLinkViolation::UnconfinedDestination))?;
                let target = owners
                    .get(&(package.clone(), target_path.clone()))
                    .ok_or_else(|| error(MarkdownLinkViolation::MissingTarget))?;
                let target_is_page = resources
                    .iter()
                    .any(|(usage, node)| &usage.resource == target && node.page.is_some());
                if !target_is_page
                    && !matches!(
                        resources
                            .get(&PackageResourceUse::new(
                                target.clone(),
                                PackageResourceKindV1::Doctrine
                            ))
                            .map(|node| &node.resource),
                        Some(PackageResource::Doctrine {
                            placement: DoctrinePlacement::StandaloneJudgment,
                            ..
                        })
                    )
                {
                    return Err(error(MarkdownLinkViolation::UnrenderedTarget));
                }
                if target != from
                    && !resources
                        .iter()
                        .filter(|(usage, _)| &usage.resource == from)
                        .all(|(_, node)| {
                            node.page.is_some() || node.dependencies().contains(target)
                        })
                {
                    return Err(error(MarkdownLinkViolation::UndeclaredRelationship));
                }
                if fragment
                    .as_ref()
                    .is_some_and(|fragment| !bodies[target].has_fragment(fragment))
                {
                    return Err(error(MarkdownLinkViolation::MissingHeading));
                }
                links.push(ResourceTextLink {
                    range: link_range.clone(),
                    destination_range: range.start - link_range.start..range.end - link_range.start,
                    reference: ResourceTextReference {
                        fragment,
                        ..references[target].clone()
                    },
                });
            }
            links.sort_by_key(|link| link.range.start);
            Ok(links)
        })();
        if let Some(links) = diagnostics.capture(result) {
            let page = resources.iter().find_map(|(usage, node)| {
                (&usage.resource == from)
                    .then_some(node.page.as_ref())
                    .flatten()
            });
            let page_related = page.map(|page| {
                let mut targets = page.related.iter().cloned().collect::<BTreeSet<_>>();
                targets.extend(
                    links
                        .iter()
                        .map(|link| link.reference.target.clone())
                        .filter(|target| target != from),
                );
                targets
                    .into_iter()
                    .map(|target| references[&target].clone())
                    .collect()
            });
            let body = bodies.get_mut(from).expect("admitted body owner");
            body.bind_links(links);
            if let Some(related) = page_related {
                body.bind_related_reading(related);
                continue;
            }
            if let Some(AdmittedResourceNode {
                resource: PackageResource::Doctrine { see_also, .. },
                ..
            }) = resources.get(&PackageResourceUse::new(
                from.clone(),
                PackageResourceKindV1::Doctrine,
            )) {
                body.bind_related_reading(
                    see_also
                        .iter()
                        .map(|target| references[target].clone())
                        .collect(),
                );
            }
        }
    }
    diagnostics.finish(())
}
