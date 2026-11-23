//! Consuming Package compilation stage.

use pale_core::records::{
    CapabilitiesFile, DistributionLockFile, LogicalProjectV1, LogicalRepositoryIdV1,
    PackageResourceKindV1, PackageResourceRefV1,
};
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::packages::{
    AdmittedPackageGraph, DistributionSnapshot, PackageReleaseIndex, SealedPackageGraph,
};
use crate::workspace::SelectedRepositoryMembership;

use super::{CompileError, DISTRIBUTION_COMPILER_ID};

/// Source-free Package semantics plus the exact declaration-grammar map used
/// to admit their source.
pub(super) struct CompiledPackageGraph {
    pub(super) graph: SealedPackageGraph,
    pub(super) universe: Arc<AdmittedPackageGraph>,
    pub(super) authoring_sources: crate::workspace::AuthoringSourceMap,
    pub(super) authoring_semantics: crate::workspace::AuthoringSemanticIndex,
}

/// Consume retained Package source into one source-free selected semantic IR.
pub(super) fn compile_package_graph(
    snapshot: &DistributionSnapshot,
    lock: &DistributionLockFile,
    project: &LogicalProjectV1,
    repositories: &[SelectedRepositoryMembership],
    capabilities: &CapabilitiesFile,
) -> Result<CompiledPackageGraph, CompileError> {
    let compiler = &lock.distribution.compiler;
    if compiler != DISTRIBUTION_COMPILER_ID {
        return Err(CompileError::CompilerIdentityMismatch {
            pinned: compiler.to_string(),
            running: DISTRIBUTION_COMPILER_ID.to_string(),
        });
    }
    let index = PackageReleaseIndex::capture(snapshot)?;
    let closure = index.select(lock, &project.package_seeds)?;
    let authoring_sources = closure.authoring_sources().map_err(|source| {
        super::CompilationInvariantError::structure("Package authoring-source map", source)
    })?;
    let mut authoring_semantics = crate::workspace::AuthoringSemanticIndex::from_package_manifests(
        &closure,
    )
    .map_err(|source| {
        super::CompilationInvariantError::structure("Package authoring-semantic index", source)
    })?;
    let admitted = AdmittedPackageGraph::admit(snapshot, &closure, &mut authoring_semantics)?;
    let repository_resources = repository_resources(repositories);
    let universe = Arc::new(admitted.clone());
    let graph = admitted
        .select_and_seal(&capabilities.semantic, &repository_resources)
        .map_err(CompileError::from)?;
    Ok(CompiledPackageGraph {
        graph,
        universe,
        authoring_sources,
        authoring_semantics,
    })
}

/// Re-select one semantic graph from an already admitted Package universe.
///
/// Cloning the universe clones typed metadata and immutable text references;
/// it does not retain or reconstruct dropped Package source.
pub(super) fn select_package_graph(
    universe: &AdmittedPackageGraph,
    repositories: &[SelectedRepositoryMembership],
    capabilities: &CapabilitiesFile,
) -> Result<SealedPackageGraph, CompileError> {
    universe
        .clone()
        .select_and_seal(&capabilities.semantic, &repository_resources(repositories))
        .map_err(CompileError::from)
}

fn repository_resources(
    repositories: &[SelectedRepositoryMembership],
) -> BTreeMap<LogicalRepositoryIdV1, Vec<(PackageResourceRefV1, PackageResourceKindV1)>> {
    repositories
        .iter()
        .map(|repository| {
            let repository = &repository.project;
            let resources = std::iter::once((repository.role.clone(), PackageResourceKindV1::Role))
                .chain(
                    repository
                        .stacks
                        .iter()
                        .cloned()
                        .map(|resource| (resource, PackageResourceKindV1::Stack)),
                )
                .chain(
                    repository
                        .families
                        .iter()
                        .cloned()
                        .map(|resource| (resource, PackageResourceKindV1::PolicyFamily)),
                )
                .collect();
            (repository.repository_id.clone(), resources)
        })
        .collect()
}
