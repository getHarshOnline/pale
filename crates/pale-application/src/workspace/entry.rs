//! Canonical resolution of a private workspace compilation entry.
//!
//! [`resolve_entry`] derives every compiler input from one root declaration and
//! returns a [`ResolvedEntry`]. Callers cannot splice independently chosen
//! project, execution-workspace, lock, machine, and distribution coordinates
//! into one compilation.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use pale_core::records::{
    LogicalRepositoryIdV1, MachineBindingFile, MaterializationLineageBinding,
    SurfaceExecutableBinding, WorkspaceManifestFile,
};
use pale_ports::SourceFilesystemPort;

use super::load::{load_input_snapshot_from_records, load_machine};
use super::source_admission::{admit_toml, SourceBackedWorkspaceDocuments};
use super::{
    select_workspace_topology, CapturedConfigurationImprovementCandidateSources,
    CapturedOneShotSources, OneShotSourceCaptureError, SelectedWorkspaceTopology, WorkspaceError,
    WorkspaceInputSnapshot,
};

/// Resolved inputs for one `(project, execution workspace, machine)` selection.
///
/// Every portable declaration comes from the root manifest's desired topology;
/// every machine path comes from the selected machine binding. This is the
/// single canonical entry, so projections cannot manufacture topology.
#[derive(Debug)]
pub struct ResolvedEntry {
    pub(super) workspace_manifest_path: PathBuf,
    pub(super) workspace_manifest: WorkspaceManifestFile,
    pub(super) workspace_root: PathBuf,
    pub(super) distribution_root: PathBuf,
    pub(super) machine_path: PathBuf,
    pub(super) lock_path: PathBuf,
    pub(super) selection: SelectedWorkspaceTopology,
    pub(super) machine: MachineBindingFile,
}

/// Machine-local paths retained beside one path-free compiler capture.
///
/// This value is a host composition coordinate, not semantic IR. It carries
/// only already-selected repository, Git-metadata, executable, and lineage
/// bindings needed at the final filesystem/process boundary. It cannot add a
/// repository to the compiled profile or grant mutation by itself.
#[derive(Debug)]
pub struct OneShotHostEffectContext {
    repositories: BTreeMap<LogicalRepositoryIdV1, PathBuf>,
    git_metadata_roots: BTreeMap<LogicalRepositoryIdV1, PathBuf>,
    surface_executable: Option<SurfaceExecutableBinding>,
    materialization_lineage: Option<MaterializationLineageBinding>,
}

impl OneShotHostEffectContext {
    /// Borrow the selected checkout root for one compiled repository.
    #[must_use]
    pub fn repository_root(&self, repository_id: &str) -> Option<&Path> {
        self.repositories
            .iter()
            .find_map(|(id, root)| (id.as_str() == repository_id).then_some(root.as_path()))
    }

    /// Borrow an explicitly configured common-Git-directory root when present.
    #[must_use]
    pub fn git_metadata_root(&self, repository_id: &str) -> Option<&Path> {
        self.git_metadata_roots
            .iter()
            .find_map(|(id, root)| (id.as_str() == repository_id).then_some(root.as_path()))
    }

    /// Borrow the selected Pale executable for last-boundary verification.
    #[must_use]
    pub const fn surface_executable(&self) -> Option<&SurfaceExecutableBinding> {
        self.surface_executable.as_ref()
    }

    /// Borrow the selected durable materialization-lineage binding.
    #[must_use]
    pub const fn materialization_lineage(&self) -> Option<&MaterializationLineageBinding> {
        self.materialization_lineage.as_ref()
    }
}

impl ResolvedEntry {
    /// Borrow the confined distribution root selected by the workspace manifest.
    pub fn distribution_root(&self) -> &Path {
        &self.distribution_root
    }

    /// Consume the resolved entry and capture one bounded, rechecked input set.
    ///
    /// # Errors
    ///
    /// Returns a workspace error when a referenced record cannot be loaded or
    /// validated, or when the second read proves that inputs changed during the
    /// bounded snapshot capture.
    pub fn capture_inputs(
        self,
        sources: &dyn SourceFilesystemPort,
    ) -> Result<WorkspaceInputSnapshot, WorkspaceError> {
        load_input_snapshot_from_records(sources, self)
    }

    /// Consume this canonical entry into exact path-free one-shot source bytes.
    ///
    /// The host filesystem is observed only at this boundary. The returned
    /// value carries logical labels and owned bytes, so API, CLI, MCP, and
    /// language projections cannot reopen paths or derive a second compiler
    /// input policy. Capture proves bounded byte stability but deliberately
    /// leaves semantic admission to the consuming façade.
    ///
    /// # Errors
    ///
    /// Returns [`OneShotSourceCaptureError`] when source-role derivation,
    /// bounded Package capture, or the before/after stability check fails.
    pub fn capture_one_shot_sources(
        self,
        sources: &dyn SourceFilesystemPort,
    ) -> Result<CapturedOneShotSources, OneShotSourceCaptureError> {
        super::host_capture::capture_one_shot_sources(sources, self)
    }

    /// Consume this entry into a rechecked configuration-review candidate.
    ///
    /// The capture retains the portable workspace and private source roles but
    /// never observes the selected Package tree. Candidate admission must reuse
    /// the separately compiled baseline's Package semantics.
    ///
    /// # Errors
    ///
    /// Returns a workspace error when source-role derivation, a bounded read,
    /// private-record validation, or the before/after stability check fails.
    pub fn capture_configuration_improvement_candidate_sources(
        self,
        sources: &dyn SourceFilesystemPort,
    ) -> Result<CapturedConfigurationImprovementCandidateSources, WorkspaceError> {
        super::host_capture::capture_configuration_improvement_candidate_sources(sources, self)
    }

    /// Capture path-free compiler sources and their separate host effect context.
    ///
    /// The two results come from the same resolved entry. Stable source capture
    /// rechecks the machine-binding bytes before either value is returned, so a
    /// host path projection cannot silently drift from the API input.
    ///
    /// # Errors
    ///
    /// Returns [`OneShotSourceCaptureError`] for incoherent machine checkout
    /// bindings or any ordinary bounded source-capture failure.
    pub fn capture_one_shot_with_host_context(
        self,
        sources: &dyn SourceFilesystemPort,
    ) -> Result<(CapturedOneShotSources, OneShotHostEffectContext), OneShotSourceCaptureError> {
        let context = OneShotHostEffectContext::from_entry(&self)?;
        self.capture_one_shot_sources(sources)
            .map(|captured| (captured, context))
    }

    /// Capture only the bounded Package tree selected by this machine entry.
    ///
    /// Unlike [`Self::capture_one_shot_sources`], this operation deliberately
    /// does not read or admit the distribution lock. It is the host-capture
    /// boundary for deriving replacement lock coordinates from Package bytes.
    ///
    /// # Errors
    ///
    /// Returns [`crate::packages::PackageError`] when the selected Package tree
    /// cannot be captured as confined regular files within its finite limits.
    pub fn capture_package_sources(
        &self,
        sources: &dyn SourceFilesystemPort,
    ) -> Result<Vec<super::CapturedOneShotSourceDocument>, crate::packages::PackageError> {
        super::host_capture::capture_package_sources(sources, &self.distribution_root)
    }
}

impl OneShotHostEffectContext {
    fn from_entry(entry: &ResolvedEntry) -> Result<Self, WorkspaceError> {
        let selected = entry
            .selection
            .repositories
            .iter()
            .map(|membership| membership.project.repository_id.clone())
            .collect::<BTreeSet<_>>();
        let mut repositories = BTreeMap::new();
        let mut git_metadata_roots = BTreeMap::new();
        let mut seen_roots = BTreeSet::new();
        for checkout in &entry.machine.repository_checkouts {
            if !selected.contains(&checkout.repository_id) {
                continue;
            }
            let root = PathBuf::from(&checkout.checkout_root);
            if !root.is_absolute() {
                return Err(WorkspaceError::NonAbsolutePath {
                    binding: checkout.repository_id.to_string(),
                    path: checkout.checkout_root.clone(),
                });
            }
            if !seen_roots.insert(root.clone()) {
                return Err(WorkspaceError::NonDistinctPath(
                    checkout.checkout_root.clone(),
                ));
            }
            if repositories
                .insert(checkout.repository_id.clone(), root)
                .is_some()
            {
                return Err(WorkspaceError::DuplicateRepository(
                    checkout.repository_id.clone(),
                ));
            }
            if let Some(git_root) = &checkout.git_metadata_root {
                let git_root = PathBuf::from(git_root);
                if !git_root.is_absolute() {
                    return Err(WorkspaceError::NonAbsolutePath {
                        binding: format!("{} Git metadata", checkout.repository_id),
                        path: git_root.display().to_string(),
                    });
                }
                git_metadata_roots.insert(checkout.repository_id.clone(), git_root);
            }
        }
        if let Some(repository_id) = selected
            .iter()
            .find(|repository_id| !repositories.contains_key(*repository_id))
        {
            return Err(WorkspaceError::UnresolvedRepository {
                repository: repository_id.clone(),
                machine: entry.machine.machine.id.clone(),
            });
        }
        Ok(Self {
            repositories,
            git_metadata_roots,
            surface_executable: entry.machine.surface_executable.clone(),
            materialization_lineage: entry.machine.materialization_lineage.clone(),
        })
    }
}

/// Join `rel` under `base` after lexical confinement.
///
/// This function grants no filesystem authority and deliberately performs no
/// path-following canonicalization. The selected [`SourceFilesystemPort`]
/// resolves every later observation descriptor-relatively and rejects symlinked
/// ancestors or leaves at the actual read boundary.
pub(super) fn confine_rel(base: &Path, rel: impl AsRef<str>) -> Result<PathBuf, WorkspaceError> {
    let rel = rel.as_ref();
    let rel = pale_core::paths::ConfinedRelativePath::parse(rel)
        .ok_or_else(|| WorkspaceError::EscapedReference(rel.to_string()))?;
    Ok(base.join(rel.as_str()))
}

/// Load + validate the root workspace manifest and resolve the absolute inputs
/// for one `(project_id, execution_workspace_id, machine_id)` selection. Fails closed on: an
/// unreadable/invalid manifest; a relative field or id that escapes its base; a
/// missing desired declaration or membership, machine binding, or lock; a
/// machine id that does not match the request; and a machine binding that omits
/// an absolute `distribution_root`. The Package snapshot owner admits the
/// directory itself when compilation captures that tree.
///
/// # Errors
///
/// Returns a workspace error for malformed declarations, escaped or missing
/// references, requested/declarative identity mismatch, or an unusable
/// machine-bound distribution root.
pub fn resolve_entry(
    sources: &dyn SourceFilesystemPort,
    workspace_manifest: &Path,
    project_id: &str,
    execution_workspace_id: &str,
    machine_id: &str,
) -> Result<ResolvedEntry, WorkspaceError> {
    let ws_dir = workspace_manifest
        .parent()
        .ok_or_else(|| WorkspaceError::MissingParent(workspace_manifest.to_path_buf()))?;
    let workspace_documents = SourceBackedWorkspaceDocuments::new(sources, ws_dir);
    let manifest: WorkspaceManifestFile = admit_toml(
        &workspace_documents,
        workspace_manifest,
        "pale-workspace.v1",
        "workspace manifest",
    )?;
    let workspace = &manifest.workspace;
    let selection = select_workspace_topology(
        &workspace.desired_topology,
        project_id,
        execution_workspace_id,
    )?;

    let lock_path = confine_rel(ws_dir, &workspace.distribution_lock)?;
    let machine_binding_root = confine_rel(ws_dir, &workspace.machine_binding_root)?;

    let machine_path = confine_rel(&machine_binding_root, format!("{machine_id}.toml"))?;

    let machine = load_machine(&workspace_documents, &machine_path)?;
    if machine.machine.id != machine_id {
        return Err(WorkspaceError::IdentityMismatch {
            kind: "machine".to_string(),
            declared: machine.machine.id,
            requested: machine_id.to_string(),
        });
    }

    // distribution_root is machine-specific + absolute; required by this entry.
    let dr = machine
        .machine
        .distribution_root
        .as_ref()
        .ok_or_else(|| WorkspaceError::MissingDistributionRoot(machine_id.to_string()))?;
    let distribution_root = PathBuf::from(dr);
    if !distribution_root.is_absolute() {
        return Err(WorkspaceError::NonAbsolutePath {
            binding: "distribution_root".to_string(),
            path: dr.clone(),
        });
    }
    Ok(ResolvedEntry {
        workspace_manifest_path: workspace_manifest.to_path_buf(),
        workspace_manifest: manifest,
        workspace_root: ws_dir.to_path_buf(),
        distribution_root,
        machine_path,
        lock_path,
        selection,
        machine,
    })
}
