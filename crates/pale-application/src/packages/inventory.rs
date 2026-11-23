//! Derive Package membership from retained pages and typed data sources.

use std::path::Path;

use pale_contracts::CompiledSchema;
use pale_core::paths::OwnedConfinedRelativePath;
use pale_core::records::{
    AgentSurfacePackageV1, PackageDataV1, PackageIdV1, PackageResourceIdV1, PackageResourceV1,
    PackageSourceRoleV1, PackageSourceV1,
};

use super::{DistributionSnapshot, PackageError};

/// Populate the internal inventory without a second authored declaration list.
/// Every retained member still contributes its exact bytes to release identity.
pub(super) fn derive(
    snapshot: &DistributionSnapshot,
    directory: &Path,
    manifest: &mut AgentSurfacePackageV1,
) -> Result<(), PackageError> {
    for source in snapshot.files_under(directory) {
        let relative = source
            .strip_prefix(directory)
            .expect("snapshot member is below its owner");
        if relative == Path::new("package.toml") {
            continue;
        }
        let path = relative
            .to_str()
            .and_then(|path| OwnedConfinedRelativePath::parse(path).ok())
            .ok_or_else(|| invalid_source(&manifest.package.id, relative))?;
        let mut role = PackageSourceRoleV1::Documentation;
        match relative
            .extension()
            .and_then(|extension| extension.to_str())
        {
            Some("md") => {
                let text = snapshot.text(source)?;
                if text.starts_with("+++") {
                    let page = super::page::read_page(text, source)?;
                    let id = source_id(&manifest.package.id, &path, "md")?;
                    role = PackageSourceRoleV1::Declaration;
                    manifest
                        .package
                        .resources
                        .extend(
                            page.header
                                .declared_uses()
                                .into_iter()
                                .map(|(kind, exposure)| PackageResourceV1 {
                                    id: id.clone(),
                                    kind,
                                    definition_source: path.clone(),
                                    exposure,
                                }),
                        );
                }
            }
            Some("toml") => {
                let data = read_data(snapshot.text(source)?, source)?;
                role = PackageSourceRoleV1::Declaration;
                manifest.package.resources.push(data_declaration(
                    &manifest.package.id,
                    &path,
                    &data,
                )?);
            }
            _ => {}
        }
        manifest
            .package
            .sources
            .push(PackageSourceV1 { path, role });
    }
    Ok(())
}

pub(super) fn read_data(text: &str, source: &Path) -> Result<PackageDataV1, PackageError> {
    super::source_reader::parse_source_with_schema(
        text,
        source,
        &CompiledSchema::for_id("package-data.v1")?,
        "Package data",
    )
}

/// A typed data file owns one resource, identified by its path and discriminator.
pub(super) fn data_declaration(
    package: &PackageIdV1,
    source: &OwnedConfinedRelativePath,
    data: &PackageDataV1,
) -> Result<PackageResourceV1, PackageError> {
    Ok(PackageResourceV1 {
        id: source_id(package, source, "toml")?,
        kind: data.resource_kind(),
        definition_source: source.clone(),
        exposure: data.exposure(),
    })
}

fn source_id(
    package: &PackageIdV1,
    source: &OwnedConfinedRelativePath,
    extension: &str,
) -> Result<PackageResourceIdV1, PackageError> {
    source
        .as_str()
        .strip_suffix(&format!(".{extension}"))
        .and_then(|stem| PackageResourceIdV1::parse(stem).ok())
        .ok_or_else(|| invalid_source(package, Path::new(source.as_str())))
}

fn invalid_source(package: &PackageIdV1, path: &Path) -> PackageError {
    PackageError::ResourceDeclarationMismatch {
        package: package.clone(),
        source_path: path.to_string_lossy().into_owned(),
        resources: Vec::new(),
    }
}
