//! Family navigation derived from checked resource relationships.

use std::collections::{BTreeMap, BTreeSet};

use pale_core::records::{ResolvedGoverningContributionV1, SemanticOrderPurposeV1};

use super::*;

/// One generated family index, with the source bytes observed by the reader.
#[derive(Debug, Serialize)]
pub struct CatalogIndex {
    path: String,
    content: String,
    current: Option<String>,
}

impl CatalogIndex {
    /// Confined path relative to the distribution's `packages/` folder.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Index generated from checked declarations.
    #[must_use]
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Source text captured alongside those declarations, if an index exists.
    #[must_use]
    pub fn current(&self) -> Option<&str> {
        self.current.as_deref()
    }

    /// Whether this source index needs regeneration.
    #[must_use]
    pub fn is_stale(&self) -> bool {
        self.current.as_deref() != Some(&self.content)
    }
}

impl PackageCatalog {
    pub(super) fn render_indexes(
        &self,
        snapshot: &DistributionSnapshot,
        graph: &AdmittedPackageGraph,
    ) -> Result<Vec<CatalogIndex>, PackageError> {
        self.packages
            .iter()
            .map(|package| {
                let meta = &package.manifest.package;
                let folder = Path::new(&package.directory);
                let path = folder.join("INDEX.md");
                let current = if snapshot.files_under(folder).any(|file| file == path) {
                    Some(snapshot.text(&path)?.to_owned())
                } else {
                    None
                };
                let mut lines = vec![
                    format!("# {}", meta.title),
                    String::new(),
                    format!("Package ID: `{}` · [Manifest](package.toml)", meta.id),
                    String::new(),
                    meta.description.clone(),
                    String::new(),
                    "A workspace selects entries from this family for the duties its people and agents perform. To read it directly, open the page for your task and follow its reading links.".to_owned(),
                ];
                let contributions = graph
                    .governing
                    .iter()
                    .filter(|(reference, _)| reference.package_id == meta.id)
                    .map(|(_, contribution)| ResolvedGoverningContributionV1 {
                        constraints: Vec::new(),
                        obligations: Vec::new(),
                        exclusive_authority: Vec::new(),
                        narrowing: Vec::new(),
                        order: contribution
                            .order
                            .iter()
                            .filter(|edge| edge.purpose == SemanticOrderPurposeV1::Reasoning)
                            .cloned()
                            .collect(),
                    });
                let order =
                    ResolvedGoverningContributionV1::compose(contributions, &BTreeMap::new())?;
                let mut scopes = BTreeMap::<
                    crate::packages::PackageResourceUse,
                    BTreeMap<
                        crate::packages::PackageResourceUse,
                        BTreeSet<crate::packages::PackageResourceUse>,
                    >,
                >::new();
                for edge in order.order() {
                    let scope = scopes.entry(edge.scope.clone()).or_default();
                    scope.entry(edge.before.clone()).or_default();
                    scope
                        .entry(edge.after.clone())
                        .or_default()
                        .insert(edge.before.clone());
                }
                for (scope, mut pending) in scopes {
                    lines.extend([
                        String::new(),
                        format!(
                            "Start with {}. Its reasoning order is:",
                            self.reference_link(&scope.resource, folder)
                        ),
                        String::new(),
                    ]);
                    let mut position = 1;
                    while !pending.is_empty() {
                        // The core order algebra has already rejected cycles.
                        let ready = pending
                            .iter()
                            .filter(|(_, predecessors)| predecessors.is_empty())
                            .map(|(key, _)| key.clone())
                            .collect::<Vec<_>>();
                        for key in ready {
                            pending.remove(&key);
                            for predecessors in pending.values_mut() {
                                predecessors.remove(&key);
                            }
                            lines.push(format!(
                                "{position}. {}",
                                self.reference_link(&key.resource, folder)
                            ));
                            position += 1;
                        }
                    }
                }
                let mut members = self
                    .resources
                    .iter()
                    .filter(|resource| resource.package == meta.id)
                    .collect::<Vec<_>>();
                members
                    .sort_by_key(|resource| (resource.title.to_lowercase(), resource.id.clone()));
                for (group, name) in [
                    "Guides",
                    "Tasks",
                    "Roles",
                    "Stacks and tools",
                    "Policies and other resources",
                ]
                .into_iter()
                .enumerate()
                {
                    let entries = members
                        .iter()
                        .copied()
                        .filter(|resource| {
                            resource.exposure == PackageResourceExposureV1::Exported
                                && resource_group(resource.kind) == group
                        })
                        .collect::<Vec<_>>();
                    if entries.is_empty() {
                        continue;
                    }
                    lines.extend([String::new(), format!("## {name}"), String::new()]);
                    for resource in entries {
                        self.index_row(resource, folder, &mut lines);
                    }
                }
                let mut reached = BTreeSet::new();
                let mut pending = members
                    .iter()
                    .filter(|resource| resource.exposure == PackageResourceExposureV1::Exported)
                    .copied()
                    .collect::<Vec<_>>();
                while let Some(resource) = pending.pop() {
                    if !reached.insert(resource.id.clone()) {
                        continue;
                    }
                    for reference in resource.read_first.iter().chain(&resource.related) {
                        if reference.package_id == meta.id {
                            pending.push(self.resource(reference));
                        }
                    }
                }
                let internal = members
                    .iter()
                    .copied()
                    .filter(|resource| {
                        resource.exposure == PackageResourceExposureV1::Internal
                            && !reached.contains(&resource.id)
                    })
                    .collect::<Vec<_>>();
                if !internal.is_empty() {
                    lines.extend([
                        String::new(),
                        "## Other supporting resources".to_owned(),
                        String::new(),
                    ]);
                    for resource in internal {
                        self.index_row(resource, folder, &mut lines);
                    }
                }
                let notices = meta
                    .sources
                    .iter()
                    .filter(|source| {
                        source.role == pale_core::records::PackageSourceRoleV1::Documentation
                            && Path::new(source.path.as_str())
                                .file_name()
                                .is_some_and(|name| name == "LICENSE")
                    })
                    .collect::<Vec<_>>();
                if !notices.is_empty() {
                    lines.extend([
                        String::new(),
                        "## Licences that travel with the guides".to_owned(),
                        String::new(),
                    ]);
                    for source in notices {
                        let path = Path::new(source.path.as_str());
                        let parent = path.parent().unwrap_or(Path::new(""));
                        let label = if parent.file_name().is_some_and(|name| name == "attribution")
                        {
                            parent.parent().unwrap_or(parent)
                        } else {
                            parent
                        };
                        lines.push(format!(
                            "- [{}](<{}>)",
                            label.file_name().unwrap_or_default().to_string_lossy(),
                            source.path
                        ));
                    }
                }
                Ok(CatalogIndex {
                    path: path.to_string_lossy().into_owned(),
                    content: lines.join("\n") + "\n",
                    current,
                })
            })
            .collect()
    }

    fn resource(&self, reference: &PackageResourceRefV1) -> &CatalogResource {
        self.resources
            .iter()
            .find(|resource| {
                resource.package == reference.package_id && resource.id == reference.resource_id
            })
            .expect("checked resource reference")
    }

    fn reference_link(&self, reference: &PackageResourceRefV1, origin: &Path) -> String {
        self.resource_link(self.resource(reference), origin)
    }

    fn resource_link(&self, resource: &CatalogResource, origin: &Path) -> String {
        let path = self.source_path(resource).expect("checked source owner");
        format!(
            "[{}](<{}>)",
            resource.title.replace('[', "\\[").replace(']', "\\]"),
            relative_path(origin, &path)
        )
    }

    fn index_row(&self, resource: &CatalogResource, folder: &Path, lines: &mut Vec<String>) {
        let kind = serde_json::to_value(resource.kind).expect("resource kind is a wire string");
        lines.push(format!(
            "- {} · `{}` · `{}`",
            self.resource_link(resource, folder),
            resource.id,
            kind.as_str().expect("resource kind string")
        ));
        if let Some(description) = resource
            .record
            .get("description")
            .or_else(|| resource.record.get("purpose"))
            .and_then(serde_json::Value::as_str)
        {
            lines.push(format!(
                "  {}",
                description.split_whitespace().collect::<Vec<_>>().join(" ")
            ));
        }
        for (label, references) in [
            ("Read first", &resource.read_first),
            ("Read as needed", &resource.related),
        ] {
            let mut seen = BTreeSet::new();
            let links = references
                .iter()
                .filter(|reference| seen.insert(*reference))
                .map(|reference| self.reference_link(reference, folder))
                .collect::<Vec<_>>();
            if !links.is_empty() {
                lines.push(format!("  {label}: {}.", links.join(", ")));
            }
        }
    }
}

fn resource_group(kind: PackageResourceKindV1) -> usize {
    use PackageResourceKindV1 as Kind;
    match kind {
        Kind::Doctrine => 0,
        Kind::Operation => 1,
        Kind::Role => 2,
        Kind::Stack | Kind::ToolRecipe => 3,
        _ => 4,
    }
}
