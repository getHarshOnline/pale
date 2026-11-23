//! Retain the reading needed by each page use and any method it includes.

use std::collections::{BTreeMap, BTreeSet};

use pale_core::records::PackageResourceKindV1;

use super::{ResolvedDoctrine, ResolvedOperation, ResolvedPageReading, ResolvedRole};
use crate::packages::{PackageResource, PackageResourceUse, SealedPackageGraph};

pub(super) fn project_page_reading(
    graph: &SealedPackageGraph,
    scope: &BTreeSet<PackageResourceUse>,
    doctrine: &[ResolvedDoctrine],
    startup_policies: &[ResolvedDoctrine],
    operations: &[ResolvedOperation],
    roles: &[ResolvedRole],
) -> BTreeMap<PackageResourceUse, ResolvedPageReading> {
    let mut projected = BTreeMap::new();
    for usage in scope {
        let mut methods = vec![usage.clone()];
        let mut linked_reading = Vec::new();
        if usage.kind == PackageResourceKindV1::Doctrine {
            if let Some(guide) = doctrine
                .iter()
                .chain(startup_policies)
                .find(|guide| guide.record.id == usage.resource)
            {
                methods.extend(guide.body_sources.iter().map(|source| {
                    PackageResourceUse::new(source.owner.clone(), PackageResourceKindV1::Doctrine)
                }));
            }
        }
        if usage.kind == PackageResourceKindV1::Operation {
            if let Some(operation) = operations
                .iter()
                .find(|operation| operation.record.id == usage.resource)
            {
                methods.extend(operation.doctrine_ids.iter().map(|id| {
                    PackageResourceUse::new(id.clone(), PackageResourceKindV1::Doctrine)
                }));
            }
        }
        if usage.kind == PackageResourceKindV1::Role {
            let mut pending = vec![usage.clone()];
            let mut seen = BTreeSet::new();
            while let Some(role) = pending.pop() {
                if !seen.insert(role.clone()) {
                    continue;
                }
                let Some(node) = graph.resources().get(&role) else {
                    continue;
                };
                if let PackageResource::Role {
                    inherited_lenses, ..
                } = &node.resource
                {
                    for parent in inherited_lenses {
                        let parent =
                            PackageResourceUse::new(parent.clone(), PackageResourceKindV1::Role);
                        methods.push(parent.clone());
                        pending.push(parent);
                    }
                }
            }
            if let Some(role) = roles.iter().find(|role| role.record.id == usage.resource) {
                methods.extend(role.doctrine_ids.iter().map(|id| {
                    PackageResourceUse::new(id.clone(), PackageResourceKindV1::Doctrine)
                }));
                for guide in &role.required_reading {
                    methods.push(PackageResourceUse::new(
                        guide.clone(),
                        PackageResourceKindV1::Doctrine,
                    ));
                    if !role
                        .body_sources
                        .iter()
                        .any(|source| source.owner == *guide)
                    {
                        linked_reading.push(guide.clone());
                    }
                }
            }
        }
        let mut reading = ResolvedPageReading {
            authored_page: graph
                .resources()
                .get(usage)
                .is_some_and(|node| node.page.is_some()),
            required: linked_reading,
            related: Vec::new(),
            official_sources: Vec::new(),
        };
        let mut has_page = !reading.required.is_empty();
        let mut seen_methods = BTreeSet::new();
        for method in methods {
            if !seen_methods.insert(method.clone()) {
                continue;
            }
            if let Some(page) = graph
                .resources()
                .get(&method)
                .and_then(|node| node.page.as_ref())
            {
                has_page = true;
                for source in &page.official_sources {
                    if !reading.official_sources.contains(source) {
                        reading.official_sources.push(source.clone());
                    }
                }
                for required in &page.required {
                    if !reading.required.contains(required) {
                        reading.required.push(required.clone());
                    }
                }
            }
            if let Some(body) = graph.body(&method.resource) {
                for reference in body.related_reading() {
                    if !reading.related.contains(reference) {
                        reading.related.push(reference.clone());
                    }
                }
            }
        }
        if has_page {
            reading
                .related
                .retain(|reference| !reading.required.contains(&reference.target));
            projected.insert(usage.clone(), reading);
        }
    }
    projected
}
