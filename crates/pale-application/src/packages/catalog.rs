//! Authoring views produced by the same reader that admits compiled Packages.

use std::path::Path;

use pale_core::paths::OwnedConfinedRelativePath;
use pale_core::records::{
    AgentSurfacePackageV1, DistributionLockFile, DistributionMeta, PackageIdV1,
    PackageResourceExposureV1, PackageResourceIdV1, PackageResourceKindV1, PackageResourceRefV1,
    PackageResourceV1,
};
use pale_ports::SourceFilesystemPort;
use serde::Serialize;

use super::{AdmittedPackageGraph, DistributionSnapshot, PackageError, PackageReleaseIndex};

mod formatting;
mod incoming;
mod navigation;
pub use formatting::PackageFormattingIssue;
pub use incoming::{PackageCallerIssue, PackageCallerIssueKind, PackageCallerReport};
pub use navigation::CatalogIndex;

/// One checked Package and its location relative to the distribution's `packages/` folder.
#[derive(Debug, Serialize)]
pub struct CatalogPackage {
    directory: String,
    manifest: AgentSurfacePackageV1,
}

/// A checked resource, its authored declaration and the body it owns.
///
/// This is an authoring view, not a workspace selection or permission to run a tool.
#[derive(Debug, Serialize)]
pub struct CatalogResource {
    package: PackageIdV1,
    id: PackageResourceIdV1,
    kind: PackageResourceKindV1,
    exposure: PackageResourceExposureV1,
    source: OwnedConfinedRelativePath,
    body: Option<OwnedConfinedRelativePath>,
    record: serde_json::Value,
    title: String,
    text: Option<String>,
    read_first: Vec<PackageResourceRefV1>,
    related: Vec<PackageResourceRefV1>,
}

impl CatalogResource {
    pub(super) fn from_source(
        package: &PackageIdV1,
        declaration: &PackageResourceV1,
        body: Option<&OwnedConfinedRelativePath>,
        record: serde_json::Value,
    ) -> Self {
        Self {
            package: package.clone(),
            id: declaration.id.clone(),
            kind: declaration.kind,
            exposure: declaration.exposure,
            source: declaration.definition_source.clone(),
            body: body.cloned(),
            record,
            title: declaration.id.to_string(),
            text: None,
            read_first: Vec::new(),
            related: Vec::new(),
        }
    }
}

/// Complete authoring inventory after source, relationship and body-link checks.
///
/// Consumers use this view instead of independently interpreting Package TOML.
/// The compiler's reader checks all declared resources, including internal ones.
/// Successful inspection establishes no workspace selection or native effect.
#[derive(Debug, Serialize)]
pub struct PackageCatalog {
    packages: Vec<CatalogPackage>,
    resources: Vec<CatalogResource>,
    indexes: Vec<CatalogIndex>,
    #[serde(skip)]
    formatting_issues: Vec<PackageFormattingIssue>,
    #[serde(skip)]
    incoming_targets:
        std::collections::BTreeMap<std::path::PathBuf, Option<crate::markdown::MarkdownProjection>>,
}

impl PackageCatalog {
    /// Wrapped paragraphs in live resource bodies, checked from captured source.
    /// Documentation-only originals and generated indexes are outside this check.
    #[must_use]
    pub fn formatting_issues(&self) -> &[PackageFormattingIssue] {
        &self.formatting_issues
    }

    /// Generated family indexes and their captured source text.
    #[must_use]
    pub fn indexes(&self) -> &[CatalogIndex] {
        &self.indexes
    }
    /// Show one authored body together with its declared reading route.
    ///
    /// The key is `package-id:resource-id`. Data-only resources return no body.
    #[must_use]
    pub fn show(&self, key: &str) -> Option<String> {
        let resource = self
            .resources
            .iter()
            .find(|resource| format!("{}:{}", resource.package, resource.id) == key)?;
        let text = resource.text.as_ref()?;
        let origin = self.source_path(resource)?;
        let origin = origin.parent()?;
        let mut reading = Vec::new();
        for (label, references) in [
            ("Read first", &resource.read_first),
            ("Read as needed", &resource.related),
        ] {
            let links = references
                .iter()
                .filter_map(|reference| {
                    let target = self.resources.iter().find(|target| {
                        target.package == reference.package_id && target.id == reference.resource_id
                    })?;
                    let path = self.source_path(target)?;
                    let label = target.title.replace('[', "\\[").replace(']', "\\]");
                    Some(format!("[{label}](<{}>)", relative_path(origin, &path)))
                })
                .collect::<Vec<_>>();
            if !links.is_empty() {
                reading.push(format!("{label}: {}.", links.join(", ")));
            }
        }
        if reading.is_empty() {
            return Some(text.clone());
        }
        let display = text.trim_start_matches(['\r', '\n']);
        let (title, body) = display.split_once('\n').unwrap_or((display, ""));
        Some(format!(
            "{title}\n\n{}\n\n{}",
            reading.join("\n\n"),
            body.trim_start_matches('\n')
        ))
    }

    fn source_path(&self, resource: &CatalogResource) -> Option<std::path::PathBuf> {
        let package = self
            .packages
            .iter()
            .find(|package| package.manifest.package.id == resource.package)?;
        Some(
            Path::new(&package.directory)
                .join(resource.body.as_ref().unwrap_or(&resource.source).as_str()),
        )
    }
}

fn relative_path(origin: &Path, target: &Path) -> String {
    let origin = origin.components().collect::<Vec<_>>();
    let target = target.components().collect::<Vec<_>>();
    let shared = origin
        .iter()
        .zip(&target)
        .take_while(|(a, b)| a == b)
        .count();
    let mut path = std::path::PathBuf::new();
    for _ in shared..origin.len() {
        path.push("..");
    }
    for part in &target[shared..] {
        path.push(part.as_os_str());
    }
    path.to_string_lossy().into_owned()
}

/// Read and check a distribution for Package navigation and authoring tools.
///
/// The capture and reader are shared with compilation. All reads after capture
/// use the same bounded source bytes, including reference and heading checks.
///
/// # Errors
///
/// Returns [`PackageError`] for rejected filesystem input, declarations,
/// dependencies, bodies or links.
pub fn read_catalog(
    sources: &dyn SourceFilesystemPort,
    distribution_root: &Path,
) -> Result<PackageCatalog, PackageError> {
    read_snapshot(&DistributionSnapshot::capture(sources, distribution_root)?)
}

pub(crate) fn read_snapshot(
    snapshot: &DistributionSnapshot,
) -> Result<PackageCatalog, PackageError> {
    let index = PackageReleaseIndex::capture(snapshot)?;
    let registry = pale_contracts::schema_registry_identity()?;
    let lock = DistributionLockFile {
        distribution: DistributionMeta {
            compiler: crate::compile::DISTRIBUTION_COMPILER_ID.to_owned(),
            schema_manifest_sha256: registry.schema_manifest_sha256,
            schema_set_sha256: registry.schema_set_sha256,
        },
        packages: index.releases().cloned().collect(),
    };
    let seeds = lock
        .packages
        .iter()
        .map(|release| release.id.clone())
        .collect::<Vec<_>>();
    let sources = index.select(&lock, &seeds)?;
    let packages = sources
        .packages()
        .map(|(_, package)| CatalogPackage {
            directory: package.directory().to_string_lossy().into_owned(),
            manifest: package.manifest().clone(),
        })
        .collect();
    let mut resources = Vec::new();
    let graph = AdmittedPackageGraph::read(
        snapshot,
        &sources,
        &mut Default::default(),
        Some(&mut resources),
    )?;
    for resource in &mut resources {
        let key = PackageResourceRefV1 {
            package_id: resource.package.clone(),
            resource_id: resource.id.clone(),
        };
        let node = &graph.resources[&super::PackageResourceUse::new(key.clone(), resource.kind)];
        let required = node
            .page
            .as_ref()
            .map_or_else(|| node.requirements(), |page| page.required.clone());
        resource.related = node
            .dependencies()
            .into_iter()
            .filter(|reference| {
                !required.contains(reference) && graph.bodies.contains_key(reference)
            })
            .collect();
        if let Some(body) = graph.bodies.get(&key) {
            resource.related.extend(
                body.related_reading()
                    .iter()
                    .map(|reference| reference.target.clone())
                    .filter(|target| !required.contains(target)),
            );
            resource.related.sort();
            resource.related.dedup();
        }
        resource.read_first = required
            .into_iter()
            .filter(|reference| graph.bodies.contains_key(reference))
            .collect();
        if let Some(body) = graph.bodies.get(&key) {
            resource.title = body.title().unwrap_or(resource.id.as_str()).to_owned();
            resource.text = Some(body.text().to_owned());
        }
    }
    let mut catalog = PackageCatalog {
        packages,
        resources,
        indexes: Vec::new(),
        formatting_issues: Vec::new(),
        incoming_targets: incoming::targets(snapshot)?,
    };
    catalog.formatting_issues = formatting::check(snapshot, &catalog)?;
    for resource in &catalog.resources {
        if let (Some(path), Some(text)) = (catalog.source_path(resource), resource.text.as_ref()) {
            catalog
                .incoming_targets
                .insert(path, Some(crate::markdown::project(text)));
        }
    }
    catalog.indexes = catalog.render_indexes(snapshot, &graph)?;
    // Caller checks must also hold after regeneration removes an old heading.
    for index in &catalog.indexes {
        catalog.incoming_targets.insert(
            std::path::PathBuf::from(index.path()),
            Some(crate::markdown::project(index.content())),
        );
    }
    Ok(catalog)
}
