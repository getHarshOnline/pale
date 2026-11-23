//! Admission of the three private one-shot input roles from owned bytes.
//!
//! Logical labels exist only to preserve the same reference semantics used by
//! source-backed loading. They are not host paths and establish neither common
//! filesystem-root provenance nor machine readiness.

use std::borrow::Cow;
use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};
use std::path::Path;

use pale_core::paths::{ConfinedRelativePath, MAX_CONFINED_RELATIVE_PATH_BYTES};
use pale_core::records::{
    CapabilitiesFile, DistributionLockFile, ExecutionPathSelectionV1, LogicalExecutionWorkspaceV1,
    LogicalProjectV1, MachineBindingFile,
};
use pale_ports::SourceReadLimit;

use crate::captured_input::{CapturedCompilationInputError, CapturedInputLimit};

use super::load::{load_capabilities_from_record, load_lock, load_machine};
use super::source_admission::{admit_toml, WorkspaceDocumentSource, WORKSPACE_FILE_LIMIT};
use super::{
    AuthoringDocumentKind, AuthoringSourceMap, AuthoringTomlGrammar, SelectedRepositoryMembership,
    SelectedWorkspaceTopology, WorkspaceError,
};

/// Maximum unique lifecycle-guidance documents retained with private inputs.
///
/// The ceiling bounds the cardinality of an untrusted captured bundle before
/// its capabilities record is parsed. It does not expand the set declared by
/// that record: admission still requires exact equality with the uniquely
/// referenced guidance labels.
const MAX_CAPTURED_LIFECYCLE_GUIDANCE_DOCUMENTS: usize = 4_096;
/// Maximum private documents retained by one captured compilation.
const MAX_CAPTURED_PRIVATE_DOCUMENTS: usize = MAX_CAPTURED_LIFECYCLE_GUIDANCE_DOCUMENTS + 3;
/// Maximum aggregate bytes retained across one private captured bundle.
const MAX_CAPTURED_PRIVATE_TOTAL_BYTES: u64 = 64 * 1024 * 1024;
/// Maximum bytes retained from one private TOML or guidance document.
const MAX_CAPTURED_PRIVATE_FILE_BYTES: usize = WORKSPACE_FILE_LIMIT.max_bytes();

/// One singular private TOML document with its logical source label.
pub struct CapturedPrivateDocument {
    label: String,
    bytes: Vec<u8>,
}

impl std::fmt::Debug for CapturedPrivateDocument {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CapturedPrivateDocument")
            .field("label_byte_len", &self.label.len())
            .field("byte_len", &self.bytes.len())
            .finish()
    }
}

impl CapturedPrivateDocument {
    /// Maximum bytes accepted for one private TOML document.
    pub const MAX_BYTES: usize = MAX_CAPTURED_PRIVATE_FILE_BYTES;

    /// Admit one confined label and complete private TOML body.
    ///
    /// The label preserves editor and diagnostic correlation. It grants no
    /// host-path or filesystem capability.
    ///
    /// # Errors
    ///
    /// Returns [`CapturedCompilationInputError`] when `label` is not confined
    /// or `bytes` exceeds the private source-record ceiling.
    pub fn new(label: String, bytes: Vec<u8>) -> Result<Self, CapturedCompilationInputError> {
        validate_label("private TOML role", &label)?;
        validate_private_bytes(bytes.len())?;
        Ok(Self { label, bytes })
    }

    fn into_parts(self) -> (String, Vec<u8>) {
        (self.label, self.bytes)
    }
}

/// One lifecycle-guidance body addressed by its declared logical label.
pub struct CapturedLifecycleGuidance {
    label: String,
    bytes: Vec<u8>,
}

impl std::fmt::Debug for CapturedLifecycleGuidance {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CapturedLifecycleGuidance")
            .field("label_byte_len", &self.label.len())
            .field("byte_len", &self.bytes.len())
            .finish()
    }
}

impl CapturedLifecycleGuidance {
    /// Maximum bytes accepted for one lifecycle-guidance document.
    pub const MAX_BYTES: usize = MAX_CAPTURED_PRIVATE_FILE_BYTES;

    /// Admit one confined lifecycle-guidance label and its complete bytes.
    ///
    /// Multiple behaviors may reference the same label. The enclosing private
    /// bundle accepts exactly one document for each unique referenced label.
    ///
    /// # Errors
    ///
    /// Returns [`CapturedCompilationInputError`] when `label` is not confined or
    /// `bytes` exceeds the private source-record ceiling.
    pub fn new(label: String, bytes: Vec<u8>) -> Result<Self, CapturedCompilationInputError> {
        validate_label("lifecycle guidance", &label)?;
        validate_private_bytes(bytes.len())?;
        Ok(Self { label, bytes })
    }
}

/// Exact owned bytes for the three private compiler record roles.
///
/// Construction validates only finite byte and guidance-label ingress. Typed
/// TOML/schema admission, the selected project's capability reference, machine
/// identity, and exact set of unique referenced guidance labels are
/// checked when the value is consumed by
/// [`crate::compile::compile_captured_portable_inputs`].
pub struct CapturedPrivateWorkspaceInputs {
    capabilities: CapturedPrivateDocument,
    machine_binding: CapturedPrivateDocument,
    distribution_lock: CapturedPrivateDocument,
    lifecycle_guidance: BTreeMap<String, Vec<u8>>,
}

impl std::fmt::Debug for CapturedPrivateWorkspaceInputs {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CapturedPrivateWorkspaceInputs")
            .field("capabilities", &self.capabilities)
            .field("machine_binding", &self.machine_binding)
            .field("distribution_lock", &self.distribution_lock)
            .field("lifecycle_guidance_count", &self.lifecycle_guidance.len())
            .finish()
    }
}

impl CapturedPrivateWorkspaceInputs {
    /// Maximum singular and repeated documents in one private bundle.
    pub const MAX_DOCUMENTS: usize = MAX_CAPTURED_PRIVATE_DOCUMENTS;
    /// Maximum aggregate bytes retained by one private bundle.
    pub const MAX_TOTAL_BYTES: u64 = MAX_CAPTURED_PRIVATE_TOTAL_BYTES;

    /// Bind the fixed private TOML roles and uniquely labelled guidance bodies.
    ///
    /// # Errors
    ///
    /// Returns [`CapturedCompilationInputError`] when any private document is
    /// oversized or two guidance documents use the same admitted label.
    pub fn new(
        capabilities: CapturedPrivateDocument,
        machine_binding: CapturedPrivateDocument,
        distribution_lock: CapturedPrivateDocument,
        lifecycle_guidance: Vec<CapturedLifecycleGuidance>,
    ) -> Result<Self, CapturedCompilationInputError> {
        let observed_documents = lifecycle_guidance.len().saturating_add(3);
        validate_private_document_count(observed_documents)?;
        let total_bytes = lifecycle_guidance
            .iter()
            .map(|document| document.bytes.len() as u64)
            .chain([
                capabilities.bytes.len() as u64,
                machine_binding.bytes.len() as u64,
                distribution_lock.bytes.len() as u64,
            ])
            .try_fold(0_u64, u64::checked_add)
            .ok_or(CapturedCompilationInputError::Limit {
                limit: CapturedInputLimit::PrivateTotalBytes,
                observed: u64::MAX,
                maximum: Self::MAX_TOTAL_BYTES,
            })?;
        validate_private_total_bytes(total_bytes)?;
        let mut indexed = BTreeMap::new();
        let mut duplicate_labels = BTreeSet::new();
        for document in lifecycle_guidance {
            match indexed.entry(document.label) {
                Entry::Vacant(entry) => {
                    entry.insert(document.bytes);
                }
                Entry::Occupied(entry) => {
                    duplicate_labels.insert(entry.key().clone());
                }
            }
        }
        if let Some(label) = duplicate_labels.into_iter().next() {
            return Err(CapturedCompilationInputError::DuplicateLogicalLabel {
                kind: "lifecycle guidance",
                label,
            });
        }
        Ok(Self {
            capabilities,
            machine_binding,
            distribution_lock,
            lifecycle_guidance: indexed,
        })
    }
}

/// Share the bundle ceiling with host capture before guidance bodies are read.
pub(super) fn validate_private_document_count(
    observed: usize,
) -> Result<(), CapturedCompilationInputError> {
    if observed > CapturedPrivateWorkspaceInputs::MAX_DOCUMENTS {
        return Err(CapturedCompilationInputError::Limit {
            limit: CapturedInputLimit::PrivateDocuments,
            observed: observed as u64,
            maximum: CapturedPrivateWorkspaceInputs::MAX_DOCUMENTS as u64,
        });
    }
    Ok(())
}

/// Check each host-read increment and the already-captured bundle with one law.
pub(super) fn validate_private_total_bytes(
    observed: u64,
) -> Result<(), CapturedCompilationInputError> {
    if observed > CapturedPrivateWorkspaceInputs::MAX_TOTAL_BYTES {
        return Err(CapturedCompilationInputError::Limit {
            limit: CapturedInputLimit::PrivateTotalBytes,
            observed,
            maximum: CapturedPrivateWorkspaceInputs::MAX_TOTAL_BYTES,
        });
    }
    Ok(())
}

pub(crate) struct CapturedPrivateRecords {
    pub(crate) project: LogicalProjectV1,
    pub(crate) execution_workspace: LogicalExecutionWorkspaceV1,
    pub(crate) execution_paths: Vec<ExecutionPathSelectionV1>,
    pub(crate) repositories: Vec<SelectedRepositoryMembership>,
    pub(crate) capabilities: CapabilitiesFile,
    pub(crate) lifecycle: Vec<pale_core::records::AdmittedLifecycleBehaviorV1>,
    pub(crate) machine: MachineBindingFile,
    pub(crate) lock: DistributionLockFile,
    pub(crate) authoring_sources: AuthoringSourceMap,
    pub(crate) authoring_semantics: super::AuthoringSemanticIndex,
}

pub(crate) enum CapturedPrivateAdmissionError {
    Captured(CapturedCompilationInputError),
    Workspace(WorkspaceError),
}

impl From<CapturedCompilationInputError> for CapturedPrivateAdmissionError {
    fn from(source: CapturedCompilationInputError) -> Self {
        Self::Captured(source)
    }
}

impl From<WorkspaceError> for CapturedPrivateAdmissionError {
    fn from(source: WorkspaceError) -> Self {
        Self::Workspace(source)
    }
}

pub(crate) fn admit_captured_private_inputs(
    selection: SelectedWorkspaceTopology,
    requested_machine_id: &str,
    input: CapturedPrivateWorkspaceInputs,
) -> Result<CapturedPrivateRecords, CapturedPrivateAdmissionError> {
    let CapturedPrivateWorkspaceInputs {
        capabilities,
        machine_binding,
        distribution_lock,
        lifecycle_guidance,
    } = input;
    let (capabilities_label, capabilities_toml) = capabilities.into_parts();
    let (machine_binding_label, machine_binding_toml) = machine_binding.into_parts();
    let (distribution_lock_label, distribution_lock_toml) = distribution_lock.into_parts();
    let mut source = CapturedPrivateSource::default();
    source.insert_role(
        "private TOML role",
        &machine_binding_label,
        machine_binding_toml,
    )?;
    source.insert_role(
        "private TOML role",
        &distribution_lock_label,
        distribution_lock_toml,
    )?;
    source.insert_role("private TOML role", &capabilities_label, capabilities_toml)?;

    let capabilities: CapabilitiesFile = admit_toml(
        &source,
        Path::new(selection.project.capabilities_file.as_str()),
        "capabilities.v1",
        "capabilities",
    )?;
    let expected_guidance = capabilities
        .lifecycle
        .iter()
        .flat_map(|catalog| {
            catalog.behaviours.iter().filter_map(|behavior| {
                catalog
                    .project(behavior)
                    .guidance
                    .map(|label| label.as_str().to_owned())
            })
        })
        .collect::<BTreeSet<_>>();
    let captured_guidance = lifecycle_guidance.keys().cloned().collect::<BTreeSet<_>>();
    if expected_guidance != captured_guidance {
        return Err(CapturedCompilationInputError::LifecycleGuidanceMismatch {
            missing: expected_guidance
                .difference(&captured_guidance)
                .cloned()
                .collect(),
            unexpected: captured_guidance
                .difference(&expected_guidance)
                .cloned()
                .collect(),
        }
        .into());
    }
    let capabilities_owner = Path::new(selection.project.capabilities_file.as_str())
        .parent()
        .unwrap_or_else(|| Path::new(""));
    let mut authoring_sources = AuthoringSourceMap::default();
    authoring_sources
        .insert(
            selection.project.capabilities_file.as_str(),
            AuthoringDocumentKind::Toml(AuthoringTomlGrammar::record("capabilities.v1")),
        )
        .map_err(
            |label| CapturedCompilationInputError::DuplicateLogicalLabel {
                kind: "private TOML role",
                label,
            },
        )?;
    authoring_sources
        .insert(
            &machine_binding_label,
            AuthoringDocumentKind::Toml(AuthoringTomlGrammar::record("machine-binding.v1")),
        )
        .map_err(
            |label| CapturedCompilationInputError::DuplicateLogicalLabel {
                kind: "private TOML role",
                label,
            },
        )?;
    authoring_sources
        .insert(
            &distribution_lock_label,
            AuthoringDocumentKind::Toml(AuthoringTomlGrammar::record("distribution-lock.v1")),
        )
        .map_err(
            |label| CapturedCompilationInputError::DuplicateLogicalLabel {
                kind: "private TOML role",
                label,
            },
        )?;
    for (label, bytes) in lifecycle_guidance {
        let path = capabilities_owner.join(label);
        let label = path
            .to_str()
            .ok_or(CapturedCompilationInputError::InvalidLogicalLabel {
                kind: "lifecycle guidance",
            })?;
        authoring_sources
            .insert(label, AuthoringDocumentKind::Text)
            .map_err(
                |label| CapturedCompilationInputError::DuplicateLogicalLabel {
                    kind: "lifecycle guidance",
                    label,
                },
            )?;
        source.insert_role("lifecycle guidance", label, bytes)?;
    }

    let loaded_capabilities =
        load_capabilities_from_record(&source, capabilities_owner, capabilities)?;
    let machine = load_machine(&source, Path::new(&machine_binding_label))?;
    verify_identity("machine", &machine.machine.id, requested_machine_id)?;
    let lock = load_lock(&source, Path::new(&distribution_lock_label))?;
    let authoring_semantics = super::AuthoringSemanticIndex::from_private(
        &capabilities_label,
        &loaded_capabilities.record,
        &machine_binding_label,
        &machine,
        &distribution_lock_label,
        &lock,
    );
    Ok(CapturedPrivateRecords {
        project: selection.project,
        execution_workspace: selection.execution_workspace,
        execution_paths: selection.execution_paths,
        repositories: selection.repositories,
        capabilities: loaded_capabilities.record,
        lifecycle: loaded_capabilities.lifecycle,
        machine,
        lock,
        authoring_sources,
        authoring_semantics,
    })
}

fn verify_identity(
    kind: &'static str,
    declared: &str,
    requested: &str,
) -> Result<(), WorkspaceError> {
    if declared == requested {
        Ok(())
    } else {
        Err(WorkspaceError::IdentityMismatch {
            kind: kind.to_owned(),
            declared: declared.to_owned(),
            requested: requested.to_owned(),
        })
    }
}

#[derive(Default)]
struct CapturedPrivateSource {
    documents: BTreeMap<String, Vec<u8>>,
}

impl CapturedPrivateSource {
    fn insert_role(
        &mut self,
        kind: &'static str,
        label: &str,
        bytes: Vec<u8>,
    ) -> Result<(), CapturedCompilationInputError> {
        validate_label(kind, label)?;
        match self.documents.entry(label.to_owned()) {
            Entry::Vacant(entry) => {
                entry.insert(bytes);
                Ok(())
            }
            Entry::Occupied(entry) => Err(CapturedCompilationInputError::DuplicateLogicalLabel {
                kind,
                label: entry.key().clone(),
            }),
        }
    }
}

impl WorkspaceDocumentSource for CapturedPrivateSource {
    fn read_regular<'source>(
        &'source self,
        path: &Path,
        limit: SourceReadLimit,
    ) -> Result<Cow<'source, [u8]>, WorkspaceError> {
        let label = path
            .to_str()
            .filter(|label| ConfinedRelativePath::parse(label).is_some())
            .ok_or_else(|| WorkspaceError::EscapedReference(path.display().to_string()))?;
        let bytes = self
            .documents
            .get(label)
            .ok_or_else(|| WorkspaceError::MissingReference {
                reference: label.to_owned(),
                path: label.to_owned(),
            })?;
        if bytes.len() > limit.max_bytes() {
            return Err(WorkspaceError::CapturedDocumentTooLarge {
                observed: bytes.len(),
                limit: limit.max_bytes(),
            });
        }
        Ok(Cow::Borrowed(bytes))
    }
}

fn validate_label(kind: &'static str, label: &str) -> Result<(), CapturedCompilationInputError> {
    if label.len() <= MAX_CONFINED_RELATIVE_PATH_BYTES
        && ConfinedRelativePath::parse(label).is_some()
    {
        Ok(())
    } else {
        Err(CapturedCompilationInputError::InvalidLogicalLabel { kind })
    }
}

fn validate_private_bytes(byte_len: usize) -> Result<(), CapturedCompilationInputError> {
    if byte_len <= MAX_CAPTURED_PRIVATE_FILE_BYTES {
        Ok(())
    } else {
        Err(CapturedCompilationInputError::Limit {
            limit: CapturedInputLimit::PrivateFileBytes,
            observed: byte_len as u64,
            maximum: MAX_CAPTURED_PRIVATE_FILE_BYTES as u64,
        })
    }
}
