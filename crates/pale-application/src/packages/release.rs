//! Exact Package-release admission over one retained distribution snapshot.
//!
//! This module owns the transition from discoverable `package.toml` files to a
//! dependency-closed set of exact [`PackageReleaseV1`] coordinates. A Package
//! directory is not admitted because it exists: its manifest, complete source
//! inventory, lock coordinate, and every direct dependency must agree.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use pale_contracts::CompiledSchema;
use pale_core::paths::OwnedConfinedRelativePath;
use pale_core::records::{
    canonicalize_release_closures, AgentSurfacePackageV1, ArtifactDigest, DistributionLockFile,
    PackageDigest, PackageIdV1, PackageReleaseV1, PackageSourceRoleV1,
};

use super::source_reader::parse_source_with_schema;
use super::{DistributionSnapshot, PackageError};
use crate::workspace::{AuthoringDocumentKind, AuthoringSourceMap, AuthoringTomlGrammar};

/// Digest-domain separator for exact Package release content.
const PACKAGE_RELEASE_CONTENT_DOMAIN: &[u8] = b"pale.package-release-content.v1\0";

/// Manifest and source coordinates retained only until semantic admission.
#[derive(Debug, Clone)]
pub(crate) struct RetainedPackageRelease {
    directory: PathBuf,
    manifest: AgentSurfacePackageV1,
    release: PackageReleaseV1,
    typed_data_sources: BTreeSet<OwnedConfinedRelativePath>,
}

impl RetainedPackageRelease {
    pub(super) fn is_typed_data(&self, source: &OwnedConfinedRelativePath) -> bool {
        self.typed_data_sources.contains(source)
    }

    pub(super) fn directory(&self) -> &Path {
        &self.directory
    }

    /// Logical label of the manifest that owns this retained release.
    pub(crate) fn manifest_label(&self) -> Result<String, String> {
        self.directory
            .join("package.toml")
            .to_str()
            .map(str::to_owned)
            .ok_or_else(|| "Package manifest label is not UTF-8".to_owned())
    }
    /// Exact admitted release coordinate derived from retained bytes.
    pub(crate) fn release(&self) -> &PackageReleaseV1 {
        &self.release
    }

    /// Source manifest admitted for this exact release.
    pub(crate) fn manifest(&self) -> &AgentSurfacePackageV1 {
        &self.manifest
    }

    /// Resolve one manifest-owned source path inside the retained snapshot.
    pub(crate) fn source_path(
        &self,
        relative: &pale_core::paths::OwnedConfinedRelativePath,
    ) -> PathBuf {
        self.directory.join(relative.as_str())
    }

    /// Borrow one declared source member as admitted UTF-8.
    pub(crate) fn source_text<'a>(
        &self,
        snapshot: &'a DistributionSnapshot,
        relative: &pale_core::paths::OwnedConfinedRelativePath,
    ) -> Result<&'a str, PackageError> {
        snapshot.text(&self.source_path(relative))
    }
}

/// Selected exact Package dependency closure in canonical owner order.
#[derive(Debug)]
pub(crate) struct ResourceSet<'a> {
    releases: Vec<PackageReleaseV1>,
    packages: BTreeMap<PackageIdV1, &'a RetainedPackageRelease>,
}

impl<'a> ResourceSet<'a> {
    /// Canonical exact release coordinates carried into profiles and seals.
    pub(crate) fn releases(&self) -> &[PackageReleaseV1] {
        &self.releases
    }

    /// Iterate selected retained Packages in canonical owner order.
    pub(crate) fn packages(
        &self,
    ) -> impl Iterator<Item = (&PackageIdV1, &&'a RetainedPackageRelease)> {
        self.packages.iter()
    }

    /// Project exact manifest-owned source grammars for editor navigation.
    ///
    /// The same declaration classifier used by semantic parsing selects each
    /// TOML grammar. Markdown bodies remain text, and no source bytes survive
    /// this projection.
    pub(crate) fn authoring_sources(&self) -> Result<AuthoringSourceMap, String> {
        let mut sources = AuthoringSourceMap::default();
        for package in self.packages.values() {
            let manifest_label = package.manifest_label()?;
            sources.insert(
                manifest_label,
                AuthoringDocumentKind::Toml(AuthoringTomlGrammar::record(
                    "agent-surface-package.v1",
                )),
            )?;
            let mut declarations = BTreeMap::<_, Vec<_>>::new();
            for resource in &package.manifest.package.resources {
                declarations
                    .entry(&resource.definition_source)
                    .or_default()
                    .push(resource.clone());
            }
            for source in &package.manifest.package.sources {
                let label = package
                    .source_path(&source.path)
                    .to_str()
                    .ok_or_else(|| "Package source label is not UTF-8".to_owned())?
                    .to_owned();
                let kind = match source.role {
                    PackageSourceRoleV1::Declaration if source.path.as_str().ends_with(".md") => {
                        AuthoringDocumentKind::PackagePage
                    }
                    PackageSourceRoleV1::Declaration => {
                        if package.is_typed_data(&source.path) {
                            sources.insert(
                                label,
                                AuthoringDocumentKind::Toml(AuthoringTomlGrammar::record(
                                    "package-data.v1",
                                )),
                            )?;
                            continue;
                        }
                        let declared = declarations.get(&source.path).ok_or_else(|| {
                            format!(
                                "Package declaration `{}` has no manifest-owned resources",
                                source.path
                            )
                        })?;
                        let grammar =
                            super::authoring::declaration_grammar(declared).ok_or_else(|| {
                                format!(
                                    "Package declaration `{}` has no registered grammar",
                                    source.path
                                )
                            })?;
                        AuthoringDocumentKind::Toml(grammar.authoring())
                    }
                    PackageSourceRoleV1::Body | PackageSourceRoleV1::Documentation => {
                        AuthoringDocumentKind::Text
                    }
                };
                sources.insert(label, kind)?;
            }
        }
        Ok(sources)
    }
}

/// Duplicate-free manifest index over one retained distribution snapshot.
#[derive(Debug)]
pub(crate) struct PackageReleaseIndex {
    packages: BTreeMap<PackageIdV1, RetainedPackageRelease>,
}

impl PackageReleaseIndex {
    /// Admit every manifest identity and exact source inventory once.
    ///
    /// # Errors
    ///
    /// Returns [`PackageError`] for schema-invalid manifests, duplicate Package
    /// owners, nested Package roots, incomplete source inventories, or a source
    /// byte set that cannot be represented by one deterministic release digest.
    pub(crate) fn capture(snapshot: &DistributionSnapshot) -> Result<Self, PackageError> {
        let schema = CompiledSchema::for_id("agent-surface-package.v1")?;
        let package_dirs = snapshot.package_dirs().collect::<Vec<_>>();
        let mut packages = BTreeMap::new();

        for directory in &package_dirs {
            let manifest_path = directory.join("package.toml");
            let mut manifest: AgentSurfacePackageV1 = parse_source_with_schema(
                snapshot.text(&manifest_path)?,
                &manifest_path,
                &schema,
                "Package manifest",
            )?;
            if let Some(nested) = package_dirs
                .iter()
                .find(|candidate| *candidate != directory && candidate.starts_with(directory))
            {
                return Err(PackageError::NestedPackageManifest {
                    package: manifest.package.id.clone(),
                    path: nested.join("package.toml"),
                });
            }

            if manifest.package.sources.is_empty() && manifest.package.resources.is_empty() {
                super::inventory::derive(snapshot, directory, &mut manifest)?;
            }
            manifest
                .validate_contract()
                .map_err(|source| PackageError::InvalidManifest {
                    path: manifest_path.clone(),
                    source,
                })?;
            validate_source_bijection(snapshot, directory, &manifest)?;
            let mut typed_data_sources = BTreeSet::new();
            for source in &manifest.package.sources {
                if source.role != PackageSourceRoleV1::Declaration
                    || !source.path.as_str().ends_with(".toml")
                {
                    continue;
                }
                let path = directory.join(source.path.as_str());
                // Classification does not admit the source. Preserve syntax errors for
                // semantic parsing of the selected closure, including its diagnostics.
                if super::source_reader::toml_to_json(snapshot.text(&path)?, &path)
                    .is_ok_and(|value| value.get("kind").is_some())
                {
                    typed_data_sources.insert(source.path.clone());
                }
            }
            let release = exact_release(snapshot, directory, &manifest)?;
            let id = manifest.package.id.clone();
            if let Some(previous) = packages.insert(
                id.clone(),
                RetainedPackageRelease {
                    directory: directory.clone(),
                    manifest,
                    release,
                    typed_data_sources,
                },
            ) {
                return Err(PackageError::DuplicatePackage {
                    id: id.to_string(),
                    first: previous.directory.join("package.toml"),
                    second: manifest_path,
                });
            }
        }
        Ok(Self { packages })
    }

    /// Resolve Package seeds into the exact finite dependency closure.
    ///
    /// The computation is a monotone least fixed point over the finite manifest
    /// inventory: it starts with the principal-selected seeds and adds only exact
    /// direct dependencies until no new Package owner can be added.
    ///
    /// # Errors
    ///
    /// Returns [`PackageError`] when the lock is non-canonical, a seed is absent,
    /// its manifest excludes the lock's compiler, a retained release differs
    /// from its lock coordinate, or a dependency's exact `(id, version)` edge is
    /// unavailable.
    pub(crate) fn select<'a>(
        &'a self,
        lock: &DistributionLockFile,
        seeds: &[PackageIdV1],
    ) -> Result<ResourceSet<'a>, PackageError> {
        let canonical = canonicalize_release_closures(lock.packages.clone(), Vec::new())?;
        let locked = canonical
            .packages
            .iter()
            .map(|release| (release.id.clone(), release))
            .collect::<BTreeMap<_, _>>();
        let mut frontier = seeds.iter().cloned().collect::<BTreeSet<_>>();
        let mut admitted = BTreeMap::new();

        while let Some(id) = frontier.pop_first() {
            if admitted.contains_key(&id) {
                continue;
            }
            let retained =
                self.packages
                    .get(&id)
                    .ok_or_else(|| PackageError::MissingLockedPackage {
                        package: id.clone(),
                    })?;
            if !retained
                .manifest()
                .package
                .compatible_compilers
                .iter()
                .any(|candidate| candidate == &lock.distribution.compiler)
            {
                return Err(PackageError::IncompatibleCompiler {
                    package: id.clone(),
                    compiler: lock.distribution.compiler.clone(),
                });
            }
            let selected = locked
                .get(&id)
                .ok_or_else(|| PackageError::MissingLockedPackage {
                    package: id.clone(),
                })?;
            if *selected != retained.release() {
                return Err(PackageError::ReleaseMismatch {
                    package: id.clone(),
                });
            }
            for dependency in &retained.manifest().package.dependencies {
                let exact = locked
                    .get(&dependency.id)
                    .is_some_and(|release| release.version == dependency.version);
                if !exact {
                    return Err(PackageError::MissingExactDependency {
                        package: id.clone(),
                        dependency: dependency.id.clone(),
                        version: dependency.version.clone(),
                    });
                }
                frontier.insert(dependency.id.clone());
            }
            admitted.insert(id, retained);
        }

        let releases = admitted
            .values()
            .map(|package| package.release().clone())
            .collect();
        Ok(ResourceSet {
            releases,
            packages: admitted,
        })
    }

    /// Iterate every observed release for distribution inspection.
    pub(crate) fn releases(&self) -> impl Iterator<Item = &PackageReleaseV1> {
        self.packages.values().map(RetainedPackageRelease::release)
    }
}

fn validate_source_bijection(
    snapshot: &DistributionSnapshot,
    directory: &Path,
    manifest: &AgentSurfacePackageV1,
) -> Result<(), PackageError> {
    let declared = manifest
        .package
        .sources
        .iter()
        .map(|source| source.path.as_str().to_string())
        .collect::<BTreeSet<_>>();
    let retained = snapshot
        .files_under(directory)
        .filter_map(|path| path.strip_prefix(directory).ok())
        .map(|path| path.to_string_lossy().into_owned())
        .filter(|path| path != "package.toml")
        .collect::<BTreeSet<_>>();
    let missing = declared.difference(&retained).cloned().collect::<Vec<_>>();
    let undeclared = retained.difference(&declared).cloned().collect::<Vec<_>>();
    if missing.is_empty() && undeclared.is_empty() {
        Ok(())
    } else {
        Err(PackageError::SourceInventoryMismatch {
            package: manifest.package.id.clone(),
            missing,
            undeclared,
        })
    }
}

fn exact_release(
    snapshot: &DistributionSnapshot,
    directory: &Path,
    manifest: &AgentSurfacePackageV1,
) -> Result<PackageReleaseV1, PackageError> {
    let manifest_bytes = snapshot.bytes(&directory.join("package.toml"))?;
    let members = manifest
        .package
        .sources
        .iter()
        .map(|source| {
            let bytes = snapshot.bytes(&directory.join(source.path.as_str()))?;
            Ok((
                source.path.as_str(),
                source.role,
                ArtifactDigest::hash_bytes(bytes),
            ))
        })
        .collect::<Result<Vec<_>, PackageError>>()?;
    Ok(PackageReleaseV1 {
        id: manifest.package.id.clone(),
        version: manifest.package.version.clone(),
        content_digest: package_content_digest(
            &ArtifactDigest::hash_bytes(manifest_bytes),
            members,
        ),
    })
}

/// Apply the one Package-release byte identity law after source custody passes.
///
/// Source order has no force. The preimage contains the exact manifest digest
/// followed by source rows sorted by confined path; each row retains its typed
/// role so a declaration/body/documentation reclassification changes release
/// identity even when bytes do not.
fn package_content_digest<'a>(
    manifest_digest: &ArtifactDigest,
    members: impl IntoIterator<Item = (&'a str, PackageSourceRoleV1, ArtifactDigest)>,
) -> PackageDigest {
    let mut members = members.into_iter().collect::<Vec<_>>();
    members.sort_unstable_by(|left, right| left.0.cmp(right.0));
    let mut preimage = Vec::new();
    preimage.extend_from_slice(PACKAGE_RELEASE_CONTENT_DOMAIN);
    append_digest_row(&mut preimage, "package.toml", "manifest", manifest_digest);
    for (path, role, digest) in members {
        append_digest_row(&mut preimage, path, source_role_name(role), &digest);
    }
    PackageDigest::hash_bytes(&preimage)
}

fn append_digest_row(preimage: &mut Vec<u8>, path: &str, role: &str, digest: &ArtifactDigest) {
    preimage.extend_from_slice(path.as_bytes());
    preimage.push(0);
    preimage.extend_from_slice(role.as_bytes());
    preimage.push(0);
    preimage.extend_from_slice(digest.as_str().as_bytes());
    preimage.push(b'\n');
}

const fn source_role_name(role: PackageSourceRoleV1) -> &'static str {
    match role {
        PackageSourceRoleV1::Declaration => "declaration",
        PackageSourceRoleV1::Body => "body",
        PackageSourceRoleV1::Documentation => "documentation",
    }
}

#[cfg(test)]
#[path = "../../tests/internal/packages/release.rs"]
mod tests;
