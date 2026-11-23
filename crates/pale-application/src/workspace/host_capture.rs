//! Exact filesystem-to-portable capture for one canonical compiler entry.
//!
//! This module is the sole application-owned bridge from host paths to the
//! path-free source families consumed by `pale-sdk`. It composes existing
//! workspace admission and bounded Package snapshotting; it neither reparses
//! source into an alternate model nor serializes admitted records back to TOML.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use pale_core::records::{CapabilitiesFile, MachineBindingFile, WorkspaceManifestFile};
use pale_ports::SourceFilesystemPort;

use crate::packages::{DistributionSnapshot, PackageError};

use super::captured::{validate_private_document_count, validate_private_total_bytes};
use super::entry::confine_rel;
use super::portable::validate_workspace_document_count;
use super::source::WorkspaceSourceInventory;
use super::source_admission::{admit_toml_bytes, WORKSPACE_FILE_LIMIT};
use super::{ResolvedEntry, WorkspaceError};

/// One logical source label paired with its complete owned bytes.
///
/// The label is relative to its enclosing source family. It is not a host path
/// and grants no filesystem authority. Debug output reports lengths only.
#[derive(PartialEq, Eq)]
pub struct CapturedOneShotSourceDocument {
    label: String,
    bytes: Vec<u8>,
}

impl std::fmt::Debug for CapturedOneShotSourceDocument {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CapturedOneShotSourceDocument")
            .field("label_byte_len", &self.label.len())
            .field("byte_len", &self.bytes.len())
            .finish()
    }
}

impl CapturedOneShotSourceDocument {
    /// Consume this document into its logical label and exact bytes.
    #[must_use]
    pub fn into_parts(self) -> (String, Vec<u8>) {
        (self.label, self.bytes)
    }
}

/// Complete path-free source families selected by one canonical workspace entry.
///
/// Singular private roles remain separate from repeated logical documents so a
/// trusted transport adapter does not need to infer roles from labels. The
/// owned parts do not stop a caller from deliberately swapping byte vectors;
/// the public façade still admits every role semantically.
#[derive(PartialEq, Eq)]
pub struct CapturedOneShotSources {
    parts: CapturedOneShotSourceParts,
}

impl std::fmt::Debug for CapturedOneShotSources {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CapturedOneShotSources")
            .field("project_id_byte_len", &self.parts.project_id.len())
            .field(
                "execution_workspace_id_byte_len",
                &self.parts.execution_workspace_id.len(),
            )
            .field("machine_id_byte_len", &self.parts.machine_id.len())
            .field(
                "workspace_document_count",
                &self.parts.workspace_documents.len(),
            )
            .field("capabilities", &self.parts.capabilities)
            .field(
                "lifecycle_guidance_count",
                &self.parts.lifecycle_guidance.len(),
            )
            .field("machine_binding", &self.parts.machine_binding)
            .field("distribution_lock", &self.parts.distribution_lock)
            .field(
                "package_document_count",
                &self.parts.package_documents.len(),
            )
            .finish()
    }
}

impl CapturedOneShotSources {
    /// Consume the capture into its role-preserving owned parts.
    #[must_use]
    pub fn into_parts(self) -> CapturedOneShotSourceParts {
        self.parts
    }
}

/// Path-free candidate source selected for one configuration review.
///
/// The candidate retains the same workspace and private source roles as a
/// one-shot capture, but deliberately contains no Package documents. Candidate
/// admission reuses the baseline's admitted Package semantics rather than
/// reading a second distribution tree.
#[derive(PartialEq, Eq)]
pub struct CapturedConfigurationImprovementCandidateSources {
    parts: CapturedConfigurationImprovementCandidateSourceParts,
}

impl std::fmt::Debug for CapturedConfigurationImprovementCandidateSources {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CapturedConfigurationImprovementCandidateSources")
            .field("project_id_byte_len", &self.parts.project_id.len())
            .field(
                "execution_workspace_id_byte_len",
                &self.parts.execution_workspace_id.len(),
            )
            .field("machine_id_byte_len", &self.parts.machine_id.len())
            .field(
                "workspace_document_count",
                &self.parts.workspace_documents.len(),
            )
            .field("capabilities", &self.parts.capabilities)
            .field(
                "lifecycle_guidance_count",
                &self.parts.lifecycle_guidance.len(),
            )
            .field("machine_binding", &self.parts.machine_binding)
            .field("distribution_lock", &self.parts.distribution_lock)
            .finish()
    }
}

impl CapturedConfigurationImprovementCandidateSources {
    /// Consume the candidate into its role-preserving owned parts.
    #[must_use]
    pub fn into_parts(self) -> CapturedConfigurationImprovementCandidateSourceParts {
        self.parts
    }
}

/// Role-preserving owned parts of a configuration-improvement candidate.
#[derive(PartialEq, Eq)]
pub struct CapturedConfigurationImprovementCandidateSourceParts {
    /// Selected logical project identity.
    pub project_id: String,
    /// Selected logical execution-workspace identity.
    pub execution_workspace_id: String,
    /// Selected machine-binding identity.
    pub machine_id: String,
    /// Logical label of the portable workspace root manifest.
    pub workspace_root_manifest: String,
    /// Complete explicitly enumerated portable workspace source family.
    pub workspace_documents: Vec<CapturedOneShotSourceDocument>,
    /// Exact labelled private capability record.
    pub capabilities: CapturedOneShotSourceDocument,
    /// Exact private lifecycle-guidance documents.
    pub lifecycle_guidance: Vec<CapturedOneShotSourceDocument>,
    /// Exact selected machine-binding bytes.
    pub machine_binding: CapturedOneShotSourceDocument,
    /// Exact labelled distribution lock.
    pub distribution_lock: CapturedOneShotSourceDocument,
}

/// Role-preserving owned parts of [`CapturedOneShotSources`].
#[derive(PartialEq, Eq)]
pub struct CapturedOneShotSourceParts {
    /// Selected logical project identity.
    pub project_id: String,
    /// Selected logical execution-workspace identity.
    pub execution_workspace_id: String,
    /// Selected machine-binding identity.
    pub machine_id: String,
    /// Logical label of the portable workspace root manifest.
    pub workspace_root_manifest: String,
    /// Complete explicitly enumerated portable workspace source family.
    pub workspace_documents: Vec<CapturedOneShotSourceDocument>,
    /// Exact labelled private capability record.
    pub capabilities: CapturedOneShotSourceDocument,
    /// Exact private lifecycle-guidance documents, labelled relative to the
    /// capability record that references them.
    pub lifecycle_guidance: Vec<CapturedOneShotSourceDocument>,
    /// Exact selected machine-binding bytes.
    ///
    /// Absolute paths may occur as inert strings in this private record. This
    /// value grants no filesystem authority and is re-admitted by the façade.
    pub machine_binding: CapturedOneShotSourceDocument,
    /// Exact labelled distribution lock.
    pub distribution_lock: CapturedOneShotSourceDocument,
    /// Complete bounded Package source tree, labelled relative to `packages/`.
    pub package_documents: Vec<CapturedOneShotSourceDocument>,
}

/// Failure while converting one source-backed entry into path-free bytes.
#[derive(Debug, thiserror::Error)]
pub enum OneShotSourceCaptureError {
    /// Private workspace resolution, admission, or stability failed.
    #[error(transparent)]
    Workspace(#[from] WorkspaceError),
    /// Bounded Package source capture failed.
    #[error(transparent)]
    Package(#[from] PackageError),
}

struct CapturePlan<'entry> {
    project_id: String,
    execution_workspace_id: String,
    machine_id: String,
    workspace_root_manifest: String,
    workspace_root: PathBuf,
    expected_manifest: &'entry WorkspaceManifestFile,
    workspace_documents: BTreeMap<String, PathBuf>,
    capabilities_path: PathBuf,
    machine_path: PathBuf,
    expected_machine: &'entry MachineBindingFile,
    lock_path: PathBuf,
    distribution_root: PathBuf,
}

struct CapturedConfigurationSourceParts {
    project_id: String,
    execution_workspace_id: String,
    machine_id: String,
    workspace_root_manifest: String,
    workspace_documents: Vec<CapturedOneShotSourceDocument>,
    capabilities: CapturedOneShotSourceDocument,
    lifecycle_guidance: Vec<CapturedOneShotSourceDocument>,
    machine_binding: CapturedOneShotSourceDocument,
    distribution_lock: CapturedOneShotSourceDocument,
}

pub(super) fn capture_one_shot_sources(
    sources: &dyn SourceFilesystemPort,
    entry: ResolvedEntry,
) -> Result<CapturedOneShotSources, OneShotSourceCaptureError> {
    let plan = CapturePlan::from_entry(&entry)?;
    let before = plan.capture_configuration_sources(sources)?;
    let package_documents = capture_package_documents(sources, &plan.distribution_root)?;

    if !plan.matches_configuration_capture(sources, &before)?
        || capture_package_documents(sources, &plan.distribution_root)? != package_documents
    {
        return Err(WorkspaceError::InputChanged(plan.workspace_root.display().to_string()).into());
    }
    Ok(CapturedOneShotSources {
        parts: before.into_one_shot(package_documents),
    })
}

pub(super) fn capture_configuration_improvement_candidate_sources(
    sources: &dyn SourceFilesystemPort,
    entry: ResolvedEntry,
) -> Result<CapturedConfigurationImprovementCandidateSources, WorkspaceError> {
    let plan = CapturePlan::from_entry(&entry)?;
    let before = plan.capture_configuration_sources(sources)?;

    if !plan.matches_configuration_capture(sources, &before)? {
        return Err(WorkspaceError::InputChanged(
            plan.workspace_root.display().to_string(),
        ));
    }
    Ok(CapturedConfigurationImprovementCandidateSources {
        parts: before.into_candidate(),
    })
}

pub(super) fn capture_package_sources(
    sources: &dyn SourceFilesystemPort,
    distribution_root: &Path,
) -> Result<Vec<CapturedOneShotSourceDocument>, PackageError> {
    capture_package_documents(sources, distribution_root)
}

impl<'entry> CapturePlan<'entry> {
    fn from_entry(entry: &'entry ResolvedEntry) -> Result<Self, WorkspaceError> {
        let workspace_root_manifest = entry
            .workspace_manifest_path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                WorkspaceError::EscapedReference(
                    entry.workspace_manifest_path.display().to_string(),
                )
            })?
            .to_owned();
        let workspace_documents = WorkspaceSourceInventory::from_manifest(
            &entry.workspace_manifest_path,
            &entry.workspace_manifest,
        )?
        .into_documents();
        validate_workspace_document_count(workspace_documents.len())?;
        let capabilities_path = confine_rel(
            &entry.workspace_root,
            entry.selection.project.capabilities_file.as_str(),
        )?;
        Ok(Self {
            project_id: entry.selection.project.id.to_string(),
            execution_workspace_id: entry.selection.execution_workspace.id.to_string(),
            machine_id: entry.machine.machine.id.clone(),
            workspace_root_manifest,
            workspace_root: entry.workspace_root.clone(),
            expected_manifest: &entry.workspace_manifest,
            workspace_documents,
            capabilities_path,
            machine_path: entry.machine_path.clone(),
            expected_machine: &entry.machine,
            lock_path: entry.lock_path.clone(),
            distribution_root: entry.distribution_root.clone(),
        })
    }

    fn capture_configuration_sources(
        &self,
        sources: &dyn SourceFilesystemPort,
    ) -> Result<CapturedConfigurationSourceParts, WorkspaceError> {
        // Check the root before reading paths derived from the resolved revision.
        // Two later observations agreeing would not prove this earlier join.
        let manifest_path = self.workspace_root.join(&self.workspace_root_manifest);
        let manifest = capture_document(sources, &self.workspace_root, &manifest_path)?;
        let record: WorkspaceManifestFile = admit_toml_bytes(
            &manifest.bytes,
            &manifest_path,
            "pale-workspace.v1",
            "workspace manifest",
        )?;
        if &record != self.expected_manifest {
            return Err(WorkspaceError::InputChanged(
                manifest_path.display().to_string(),
            ));
        }
        let mut workspace_documents = Vec::with_capacity(self.workspace_documents.len());
        for (label, path) in &self.workspace_documents {
            if label != &self.workspace_root_manifest {
                workspace_documents.push(capture_document(sources, &self.workspace_root, path)?);
            }
        }
        workspace_documents.push(manifest);
        workspace_documents.sort_by(|left, right| left.label.cmp(&right.label));
        let mut private_bytes = 0;
        let capabilities = capture_private_document(
            sources,
            &self.workspace_root,
            &self.capabilities_path,
            &mut private_bytes,
        )?;
        let capability_record: CapabilitiesFile = admit_toml_bytes(
            &capabilities.bytes,
            &self.capabilities_path,
            "capabilities.v1",
            "capabilities",
        )?;
        let guidance_paths = lifecycle_guidance_paths(&self.capabilities_path, &capability_record)?;
        validate_private_document_count(guidance_paths.len().saturating_add(3))?;
        let lifecycle_guidance = capture_documents(
            sources,
            &self.workspace_root,
            &guidance_paths,
            &mut private_bytes,
        )?;
        let machine_binding = capture_private_document(
            sources,
            &self.workspace_root,
            &self.machine_path,
            &mut private_bytes,
        )?;
        let machine: MachineBindingFile = admit_toml_bytes(
            &machine_binding.bytes,
            &self.machine_path,
            "machine-binding.v1",
            "machine binding",
        )?;
        if &machine != self.expected_machine {
            return Err(WorkspaceError::InputChanged(
                self.machine_path.display().to_string(),
            ));
        }
        let distribution_lock = capture_private_document(
            sources,
            &self.workspace_root,
            &self.lock_path,
            &mut private_bytes,
        )?;
        Ok(CapturedConfigurationSourceParts {
            project_id: self.project_id.clone(),
            execution_workspace_id: self.execution_workspace_id.clone(),
            machine_id: self.machine_id.clone(),
            workspace_root_manifest: self.workspace_root_manifest.clone(),
            workspace_documents,
            capabilities,
            lifecycle_guidance,
            machine_binding,
            distribution_lock,
        })
    }

    /// Reobserve every selected member without constructing a second aggregate.
    ///
    /// The comparison is deliberately non-atomic: it proves that two complete
    /// bounded observations agree before the bytes cross the API boundary. It
    /// is not semantic admission, a host filesystem snapshot, or a lock against
    /// concurrent writers.
    fn matches_configuration_capture(
        &self,
        sources: &dyn SourceFilesystemPort,
        expected: &CapturedConfigurationSourceParts,
    ) -> Result<bool, WorkspaceError> {
        if !documents_match(
            sources,
            &self.workspace_root,
            &self.workspace_documents,
            &expected.workspace_documents,
        )? {
            return Ok(false);
        }
        let capabilities =
            capture_document(sources, &self.workspace_root, &self.capabilities_path)?;
        if capabilities != expected.capabilities {
            return Ok(false);
        }
        let capability_record: CapabilitiesFile = admit_toml_bytes(
            &capabilities.bytes,
            &self.capabilities_path,
            "capabilities.v1",
            "capabilities",
        )?;
        let guidance = lifecycle_guidance_paths(&self.capabilities_path, &capability_record)?;
        if !documents_match(
            sources,
            &self.workspace_root,
            &guidance,
            &expected.lifecycle_guidance,
        )? {
            return Ok(false);
        }
        Ok(
            capture_document(sources, &self.workspace_root, &self.machine_path)?
                == expected.machine_binding
                && capture_document(sources, &self.workspace_root, &self.lock_path)?
                    == expected.distribution_lock,
        )
    }
}

impl CapturedConfigurationSourceParts {
    fn into_one_shot(
        self,
        package_documents: Vec<CapturedOneShotSourceDocument>,
    ) -> CapturedOneShotSourceParts {
        let Self {
            project_id,
            execution_workspace_id,
            machine_id,
            workspace_root_manifest,
            workspace_documents,
            capabilities,
            lifecycle_guidance,
            machine_binding,
            distribution_lock,
        } = self;
        CapturedOneShotSourceParts {
            project_id,
            execution_workspace_id,
            machine_id,
            workspace_root_manifest,
            workspace_documents,
            capabilities,
            lifecycle_guidance,
            machine_binding,
            distribution_lock,
            package_documents,
        }
    }

    fn into_candidate(self) -> CapturedConfigurationImprovementCandidateSourceParts {
        let Self {
            project_id,
            execution_workspace_id,
            machine_id,
            workspace_root_manifest,
            workspace_documents,
            capabilities,
            lifecycle_guidance,
            machine_binding,
            distribution_lock,
        } = self;
        CapturedConfigurationImprovementCandidateSourceParts {
            project_id,
            execution_workspace_id,
            machine_id,
            workspace_root_manifest,
            workspace_documents,
            capabilities,
            lifecycle_guidance,
            machine_binding,
            distribution_lock,
        }
    }
}

fn lifecycle_guidance_paths(
    capabilities_path: &Path,
    capabilities: &CapabilitiesFile,
) -> Result<BTreeMap<String, PathBuf>, WorkspaceError> {
    let owner = capabilities_path
        .parent()
        .ok_or_else(|| WorkspaceError::MissingParent(capabilities_path.to_path_buf()))?;
    capabilities
        .lifecycle
        .iter()
        .flat_map(|catalog| &catalog.behaviours)
        .filter_map(|behaviour| behaviour.guidance.as_ref())
        .map(|label| {
            confine_rel(owner, label.as_str()).map(|path| (label.as_str().to_owned(), path))
        })
        .collect()
}

fn capture_documents(
    sources: &dyn SourceFilesystemPort,
    root: &Path,
    documents: &BTreeMap<String, PathBuf>,
    total_bytes: &mut u64,
) -> Result<Vec<CapturedOneShotSourceDocument>, WorkspaceError> {
    documents
        .iter()
        .map(|(label, path)| {
            capture_private_document(sources, root, path, total_bytes).map(|mut document| {
                document.label.clone_from(label);
                document
            })
        })
        .collect()
}

fn capture_private_document(
    sources: &dyn SourceFilesystemPort,
    root: &Path,
    path: &Path,
    total_bytes: &mut u64,
) -> Result<CapturedOneShotSourceDocument, WorkspaceError> {
    let document = capture_document(sources, root, path)?;
    let observed = total_bytes.saturating_add(document.bytes.len() as u64);
    validate_private_total_bytes(observed)?;
    *total_bytes = observed;
    Ok(document)
}

fn capture_document(
    sources: &dyn SourceFilesystemPort,
    root: &Path,
    path: &Path,
) -> Result<CapturedOneShotSourceDocument, WorkspaceError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| WorkspaceError::EscapedReference(path.display().to_string()))?;
    let label = relative
        .to_str()
        .ok_or_else(|| WorkspaceError::EscapedReference(path.display().to_string()))?
        .to_owned();
    read_workspace_document(sources, root, path)
        .map(|bytes| CapturedOneShotSourceDocument { label, bytes })
}

fn documents_match(
    sources: &dyn SourceFilesystemPort,
    root: &Path,
    documents: &BTreeMap<String, PathBuf>,
    expected: &[CapturedOneShotSourceDocument],
) -> Result<bool, WorkspaceError> {
    if documents.len() != expected.len() {
        return Ok(false);
    }
    for ((label, path), expected) in documents.iter().zip(expected) {
        if label != &expected.label
            || read_workspace_document(sources, root, path)? != expected.bytes
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn read_workspace_document(
    sources: &dyn SourceFilesystemPort,
    root: &Path,
    path: &Path,
) -> Result<Vec<u8>, WorkspaceError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| WorkspaceError::EscapedReference(path.display().to_string()))?;
    sources
        .read_confined_regular(root, relative, WORKSPACE_FILE_LIMIT)
        .map_err(WorkspaceError::from)
}

fn capture_package_documents(
    sources: &dyn SourceFilesystemPort,
    distribution_root: &Path,
) -> Result<Vec<CapturedOneShotSourceDocument>, PackageError> {
    let snapshot = DistributionSnapshot::capture(sources, distribution_root)?;
    snapshot
        .into_files()
        .into_iter()
        .map(|(path, bytes)| {
            let label = path
                .to_str()
                .ok_or_else(|| PackageError::InvalidEntry { path: path.clone() })?
                .to_owned();
            Ok(CapturedOneShotSourceDocument {
                label,
                bytes: bytes.into_vec(),
            })
        })
        .collect()
}
