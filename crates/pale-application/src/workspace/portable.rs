//! Path-free admission of an already-captured sovereign workspace source.
//!
//! This boundary reuses the same parser, linker, normalizer, sealer, verifier,
//! and owner-proof admission path as filesystem-backed compilation. It owns no
//! filesystem traversal, Package distribution, machine binding, provider,
//! resident-generation, activation, or effect capability.

use std::borrow::Cow;
use std::collections::{btree_map::Entry, BTreeMap};
use std::path::Path;
use std::sync::Arc;

use pale_core::paths::{
    ConfinedRelativePath, OwnedConfinedRelativePath, MAX_CONFINED_RELATIVE_PATH_BYTES,
};
use pale_core::records::{
    PrincipalAdmittedUniverseDigest, WorkspaceDerivationNodeV1, WorkspaceDerivationReceiptV1,
    WorkspaceDerivationRootV1, WorkspaceDerivationSubjectV1, WorkspaceDesiredTopologyV1,
    WorkspaceId, MAX_EXPERIENCE_COMPOSITION_SOURCES, MAX_ORGANIZATIONAL_TRANSLATION_SOURCES,
    MAX_RESPONSIBILITY_DERIVATION_SOURCES, MAX_WORKSPACE_COMPONENTS, MAX_WORKSPACE_MODULES,
};
use pale_ports::SourceReadLimit;

use super::source_admission::{WorkspaceDocumentSource, WORKSPACE_FILE_LIMIT};
use super::universe::PrincipalAdmittedUniverse;
use super::{
    admit_workspace_source_with_authoring, select_workspace_topology,
    source::parse_workspace_source, AuthoringDocumentKind, AuthoringSourceRole,
    SelectedWorkspaceTopology, WorkspaceError,
};

/// Maximum members in one already-captured workspace-language bundle.
///
/// The bound is the root manifest, every permitted manifest-selected family,
/// and the two mandatory locks. Unreferenced members gain no semantic authority,
/// but still consume this ingress resource ceiling.
const MAX_CAPTURED_WORKSPACE_DOCUMENTS: usize = 1
    + MAX_WORKSPACE_MODULES
    + MAX_WORKSPACE_COMPONENTS
    + MAX_RESPONSIBILITY_DERIVATION_SOURCES
    + MAX_ORGANIZATIONAL_TRANSLATION_SOURCES
    + MAX_EXPERIENCE_COMPOSITION_SOURCES
    + 2;

/// Closed failure semantics for the already-captured workspace pipeline.
///
/// This algebra prevents the broad private [`WorkspaceError`] domain from
/// silently defining portable API policy. A newly introduced private failure
/// remains an [`InternalInvariant`](Self::InternalInvariant) until the owning
/// portable stage explicitly classifies it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PortableWorkspaceFailureClass {
    /// Captured labels, bytes, syntax, or required members are invalid.
    SourceRejected,
    /// One parsed source-shaped record violates its selected contract.
    ContractRejected,
    /// Linked or normalized workspace meaning is inconsistent.
    SemanticRejected,
    /// A workspace-language aggregate exceeds its semantic ceiling.
    SemanticLimitExceeded,
    /// Source delivery names an exact component absent from the captured set.
    SourceIncomplete,
    /// Admission requires exact owner-published evidence not present here.
    OwnerProofUnavailable,
    /// A selected release lifecycle state forbids new admission.
    ReleaseNotAdmissible,
    /// The Pale application contradicted an internal projection, identity, or seal law.
    InternalInvariant,
}

impl PortableWorkspaceFailureClass {
    /// Collapse the stage-specific class only at a generic recovery boundary.
    ///
    /// The richer portable class remains authoritative for API codes and stage
    /// summaries; this projection prevents its consequence from drifting from
    /// other application failures.
    #[must_use]
    pub const fn failure_disposition(self) -> crate::ApplicationFailureDisposition {
        match self {
            Self::SourceRejected
            | Self::ContractRejected
            | Self::SemanticRejected
            | Self::SemanticLimitExceeded
            | Self::SourceIncomplete => crate::ApplicationFailureDisposition::InvalidInput,
            Self::OwnerProofUnavailable | Self::ReleaseNotAdmissible => {
                crate::ApplicationFailureDisposition::Blocked
            }
            Self::InternalInvariant => crate::ApplicationFailureDisposition::Internal,
        }
    }
}

/// One typed portable failure with its exact private workspace cause retained.
///
/// Consumers classify the closed [`PortableWorkspaceFailureClass`] rather than
/// matching the much broader [`WorkspaceError`] enum. The source remains
/// available to the Rust façade for safe coordinate projection, but no source
/// text or host capability is part of the portable contract.
#[derive(Debug, thiserror::Error)]
#[error("{source}")]
pub struct PortableWorkspaceFailure {
    class: PortableWorkspaceFailureClass,
    #[source]
    source: Box<WorkspaceError>,
}

impl PortableWorkspaceFailure {
    /// Return the closed recovery-relevant class selected by the owning stage.
    #[must_use]
    pub const fn class(&self) -> PortableWorkspaceFailureClass {
        self.class
    }

    /// Borrow the exact private cause for bounded diagnostic projection.
    #[must_use]
    pub fn workspace_error(&self) -> &WorkspaceError {
        &self.source
    }

    fn source_rejected(source: WorkspaceError) -> Self {
        Self::new(PortableWorkspaceFailureClass::SourceRejected, source)
    }

    fn from_workspace_error(source: WorkspaceError) -> Self {
        let class = match &source {
            WorkspaceError::InputNotUtf8(_)
            | WorkspaceError::PackagePageHeader { .. }
            | WorkspaceError::TomlSyntax { .. }
            | WorkspaceError::MissingParent(_)
            | WorkspaceError::EscapedReference(_)
            | WorkspaceError::DuplicateCapturedSource(_)
            | WorkspaceError::MissingCapturedSourceRoot(_)
            | WorkspaceError::InvalidCapturedSourceRoot
            | WorkspaceError::CapturedDocumentTooLarge { .. }
            | WorkspaceError::CapturedDocumentCountExceeded { .. }
            | WorkspaceError::CapturedPrivateInput(_)
            | WorkspaceError::InputChanged(_)
            | WorkspaceError::UnrepresentableDerivationSourceId { .. }
            | WorkspaceError::MissingReference { .. } => {
                PortableWorkspaceFailureClass::SourceRejected
            }
            WorkspaceError::SchemaCheck { source, .. } => {
                if crate::failure::embedded_schema_failure_disposition(source)
                    == crate::ApplicationFailureDisposition::InvalidInput
                {
                    PortableWorkspaceFailureClass::ContractRejected
                } else {
                    PortableWorkspaceFailureClass::InternalInvariant
                }
            }
            WorkspaceError::Decode { .. } => PortableWorkspaceFailureClass::ContractRejected,
            WorkspaceError::InvalidWorkspaceComponent(_)
            | WorkspaceError::InvalidWorkspaceModule(_)
            | WorkspaceError::InvalidWorkspaceLock(_)
            | WorkspaceError::InvalidDesiredTopology(_)
            | WorkspaceError::UnknownProject(_)
            | WorkspaceError::UnknownExecutionWorkspace(_)
            | WorkspaceError::ProjectExecutionWorkspaceMembership { .. }
            | WorkspaceError::MissingPackageVersion(_)
            | WorkspaceError::DuplicateLockedPackage { .. }
            | WorkspaceError::MissingRunPolicy(_)
            | WorkspaceError::DuplicateRunPolicy(_)
            | WorkspaceError::InvalidRunPolicy { .. }
            | WorkspaceError::InvalidLifecycleBehavior { .. }
            | WorkspaceError::DuplicateLifecycleBehavior(_)
            | WorkspaceError::UnknownFamily { .. }
            | WorkspaceError::UnknownRepositoryCommandRole(_)
            | WorkspaceError::ConflictingRepositoryCommand { .. }
            | WorkspaceError::UnknownActivationRoot { .. }
            | WorkspaceError::EmptyActivationRoots
            | WorkspaceError::DuplicateActivationRoot { .. }
            | WorkspaceError::ActivationAdditionalRootOverlap { .. }
            | WorkspaceError::UnknownAdditionalRoot { .. }
            | WorkspaceError::DuplicateAdditionalRoot { .. }
            | WorkspaceError::SessionUnknownAuthority(_)
            | WorkspaceError::ReadingOrderNotConfined(_)
            | WorkspaceError::ReleaseClosure(_)
            | WorkspaceError::DuplicateLanguageIdentity { .. }
            | WorkspaceError::NonCanonicalLanguageSet { .. }
            | WorkspaceError::WorkspaceDigestMismatch { .. }
            | WorkspaceError::ExportProjectionMismatch(_)
            | WorkspaceError::ResolutionLockMismatch(_)
            | WorkspaceError::UniverseLockMismatch(_)
            | WorkspaceError::NamespaceResolution(_)
            | WorkspaceError::VisibilityDenied(_)
            | WorkspaceError::OnwardPropagationDenied(_)
            | WorkspaceError::PolicyWidening(_)
            | WorkspaceError::ImportVisibilityWidening { .. }
            | WorkspaceError::ImportPolicyWidening { .. }
            | WorkspaceError::PolicyCombination(_)
            | WorkspaceError::InadmissibleDeclarationCycle(_)
            | WorkspaceError::PortableInputLeakage(_)
            | WorkspaceError::ResponsibilityDerivation(_)
            | WorkspaceError::OrganizationalTranslation(_)
            | WorkspaceError::MissingDirectionalService { .. } => {
                PortableWorkspaceFailureClass::SemanticRejected
            }
            WorkspaceError::WorkspaceGraphLimit { .. } => {
                PortableWorkspaceFailureClass::SemanticLimitExceeded
            }
            WorkspaceError::SourceDeliveryUnavailable { .. } => {
                PortableWorkspaceFailureClass::SourceIncomplete
            }
            WorkspaceError::PublisherAdmissionUnavailable { .. }
            | WorkspaceError::ProtectedDeliveryAdmissionUnavailable { .. } => {
                PortableWorkspaceFailureClass::OwnerProofUnavailable
            }
            WorkspaceError::ReleaseStateNotAdmissible { .. } => {
                PortableWorkspaceFailureClass::ReleaseNotAdmissible
            }
            WorkspaceError::TomlToJson { .. }
            | WorkspaceError::Input(_)
            | WorkspaceError::InvalidProfileIdentity { .. }
            | WorkspaceError::Canon(_)
            | WorkspaceError::GeneratedRecordSchema(_)
            | WorkspaceError::UnresolvedRepository { .. }
            | WorkspaceError::UnresolvedAuthorityRoot { .. }
            | WorkspaceError::DuplicateRepository(_)
            | WorkspaceError::DuplicateProviderRepositoryProjection { .. }
            | WorkspaceError::NonAbsolutePath { .. }
            | WorkspaceError::NonDistinctPath(_)
            | WorkspaceError::MissingDistributionRoot(_)
            | WorkspaceError::IdentityMismatch { .. }
            | WorkspaceError::ActivationRequiresSurfaceExecutable(_)
            | WorkspaceError::ActivationRequiresSessionState(_)
            | WorkspaceError::ActivationRequiresDurableEvidence(_)
            | WorkspaceError::ActivationRequiresNativeHarness { .. }
            | WorkspaceError::NativeHarnessProbe { .. }
            | WorkspaceError::NativeHarnessUtf8 { .. }
            | WorkspaceError::NativeHarnessVersionMismatch { .. }
            | WorkspaceError::InvalidExecutionPathProviderWitness { .. }
            | WorkspaceError::UnsortedOrDuplicateNativeHarnessBindings
            | WorkspaceError::SurfaceExecutableMissing
            | WorkspaceError::SurfaceExecutableIdentity { .. }
            | WorkspaceError::SurfaceExecutableProbe { .. }
            | WorkspaceError::SurfaceExecutableIdentityJson { .. }
            | WorkspaceError::SurfaceExecutableIdentitySchema { .. }
            | WorkspaceError::SurfaceExecutableIdentityDecode { .. }
            | WorkspaceError::SurfaceExecutablePlatformMismatch { .. }
            | WorkspaceError::SessionStateNotAbsolute(_)
            | WorkspaceError::SessionStateNotFound(_)
            | WorkspaceError::SessionStateNotDirectory(_)
            | WorkspaceError::SessionStateNotConfined(_)
            | WorkspaceError::SessionStateUnsafePermissions(_)
            | WorkspaceError::DurableEvidenceNotAbsolute(_)
            | WorkspaceError::DurableEvidenceNotFound(_)
            | WorkspaceError::DurableEvidenceNotDirectory(_)
            | WorkspaceError::DurableEvidenceNotConfined(_)
            | WorkspaceError::DurableEvidenceUnsafePermissions(_)
            | WorkspaceError::MaterializationLineageMissing(_)
            | WorkspaceError::MaterializationLineageNotAbsolute(_)
            | WorkspaceError::MaterializationLineageNotFound(_)
            | WorkspaceError::MaterializationLineageNotDirectory(_)
            | WorkspaceError::MaterializationLineageNotConfined(_)
            | WorkspaceError::MaterializationLineageUnsafePermissions(_)
            | WorkspaceError::SurfaceExecutablePathMismatch { .. }
            | WorkspaceError::SurfaceExecutableVersionMismatch { .. }
            | WorkspaceError::SurfaceExecutableProtocolMismatch { .. }
            | WorkspaceError::ReadingOrderMissing(_)
            | WorkspaceError::ReadingOrderEscapesRoot { .. }
            | WorkspaceError::SessionPathMissing { .. }
            | WorkspaceError::SessionPathObservation { .. }
            | WorkspaceError::SessionPathNotUtf8 { .. }
            | WorkspaceError::WorkspaceSemanticIrDigestMismatch(_)
            | WorkspaceError::PortableObservationOverflow { .. }
            | WorkspaceError::InvalidDerivationReceipt(_)
            | WorkspaceError::DerivationSemanticMismatch => {
                PortableWorkspaceFailureClass::InternalInvariant
            }
        };
        Self::new(class, source)
    }

    fn new(class: PortableWorkspaceFailureClass, source: WorkspaceError) -> Self {
        Self {
            class,
            source: Box::new(source),
        }
    }
}

/// One untrusted, already-captured workspace document with a logical path.
///
/// The confined relative path has no host-path authority. Construction enforces the same
/// per-document byte ceiling used by filesystem-backed workspace admission;
/// syntax, schema, identity, and semantic checks occur only during
/// [`compile_portable_workspace`].
pub struct CapturedWorkspaceDocument {
    label: String,
    bytes: Vec<u8>,
}

impl std::fmt::Debug for CapturedWorkspaceDocument {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CapturedWorkspaceDocument")
            .field("label_byte_len", &self.label.len())
            .field("byte_len", &self.bytes.len())
            .finish()
    }
}

impl CapturedWorkspaceDocument {
    /// Maximum bytes accepted for one portable workspace document.
    pub const MAX_BYTES: usize = WORKSPACE_FILE_LIMIT.max_bytes();

    /// Admit one confined logical path and its complete captured bytes.
    ///
    /// # Errors
    ///
    /// Returns a [`PortableWorkspaceFailure`] classified as
    /// [`PortableWorkspaceFailureClass::SourceRejected`]. Its retained cause is
    /// [`WorkspaceError::EscapedReference`] for a non-confined label or
    /// [`WorkspaceError::CapturedDocumentTooLarge`] when `bytes` exceeds the
    /// source-record ceiling.
    pub fn new(label: String, bytes: Vec<u8>) -> Result<Self, PortableWorkspaceFailure> {
        if !is_captured_path(&label) {
            return Err(PortableWorkspaceFailure::source_rejected(
                WorkspaceError::EscapedReference(label),
            ));
        }
        if bytes.len() > WORKSPACE_FILE_LIMIT.max_bytes() {
            return Err(PortableWorkspaceFailure::source_rejected(
                WorkspaceError::CapturedDocumentTooLarge {
                    observed: bytes.len(),
                    limit: WORKSPACE_FILE_LIMIT.max_bytes(),
                },
            ));
        }
        Ok(Self { label, bytes })
    }
}

/// Apply context-free syntax and schema admission to one explicitly typed source.
///
/// This helper exists for editor feedback before a complete project/execution
/// context is selected. Text roles are accepted without interpreting their
/// prose. TOML roles use the same parser and registered grammar as compilation;
/// successful validation does not admit cross-record meaning.
///
/// # Errors
///
/// Returns a source- or contract-classified [`PortableWorkspaceFailure`] for an
/// unsafe label, oversized body, invalid UTF-8/TOML, or schema violation.
pub fn validate_standalone_authoring_document(
    role: AuthoringSourceRole,
    label: &str,
    bytes: &[u8],
) -> Result<(), PortableWorkspaceFailure> {
    if !is_captured_path(label) {
        return Err(PortableWorkspaceFailure::source_rejected(
            WorkspaceError::EscapedReference(label.to_owned()),
        ));
    }
    if bytes.len() > WORKSPACE_FILE_LIMIT.max_bytes() {
        return Err(PortableWorkspaceFailure::source_rejected(
            WorkspaceError::CapturedDocumentTooLarge {
                observed: bytes.len(),
                limit: WORKSPACE_FILE_LIMIT.max_bytes(),
            },
        ));
    }
    let declaration;
    let (grammar, bytes) = match role.document_kind() {
        AuthoringDocumentKind::Text => return Ok(()),
        AuthoringDocumentKind::Toml(grammar) => (grammar, bytes),
        AuthoringDocumentKind::PackagePage => {
            let source = std::str::from_utf8(bytes).map_err(|_| {
                PortableWorkspaceFailure::source_rejected(WorkspaceError::InputNotUtf8(
                    label.into(),
                ))
            })?;
            let parts = crate::packages::split_page(source).map_err(|reason| {
                PortableWorkspaceFailure::source_rejected(WorkspaceError::PackagePageHeader {
                    path: label.into(),
                    reason,
                })
            })?;
            declaration = crate::packages::page_declaration_text(source, &parts.declaration);
            (
                super::AuthoringTomlGrammar::record("package-page.v1"),
                declaration.as_bytes(),
            )
        }
    };
    super::source_admission::validate_authoring_toml(bytes, Path::new(label), grammar)
        .map_err(PortableWorkspaceFailure::from_workspace_error)
}

/// Exact bounded documents selected for one path-free workspace admission.
///
/// Duplicate labels and a missing root fail before parsing. The bundle is
/// consumed by [`compile_portable_workspace`]; successful admission retains
/// only typed path-free semantics and causal source coordinates, not these
/// source bytes.
pub struct CapturedWorkspaceSource {
    root_manifest: OwnedConfinedRelativePath,
    documents: BTreeMap<String, Vec<u8>>,
}

impl std::fmt::Debug for CapturedWorkspaceSource {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CapturedWorkspaceSource")
            .field("root_manifest_byte_len", &self.root_manifest.as_str().len())
            .field("document_count", &self.documents.len())
            .finish()
    }
}

impl CapturedWorkspaceSource {
    /// Maximum logical members accepted by one portable source bundle.
    pub const MAX_DOCUMENTS: usize = MAX_CAPTURED_WORKSPACE_DOCUMENTS;

    /// Reject a member count before a caller converts or indexes documents.
    ///
    /// # Errors
    ///
    /// Returns a [`PortableWorkspaceFailure`] classified as
    /// [`PortableWorkspaceFailureClass::SourceRejected`] with retained cause
    /// [`WorkspaceError::CapturedDocumentCountExceeded`] when `document_count`
    /// exceeds [`Self::MAX_DOCUMENTS`].
    pub fn validate_document_count(document_count: usize) -> Result<(), PortableWorkspaceFailure> {
        validate_workspace_document_count(document_count)
            .map_err(PortableWorkspaceFailure::source_rejected)
    }

    /// Admit one owned root label before any captured document is converted.
    ///
    /// # Errors
    ///
    /// Returns a [`PortableWorkspaceFailure`] classified as
    /// [`PortableWorkspaceFailureClass::SourceRejected`] with retained cause
    /// [`WorkspaceError::InvalidCapturedSourceRoot`] when `root_manifest` is
    /// outside the confined logical-path grammar. Rejection retains no copy of
    /// the untrusted label.
    pub fn admit_root_manifest(
        root_manifest: String,
    ) -> Result<OwnedConfinedRelativePath, PortableWorkspaceFailure> {
        OwnedConfinedRelativePath::parse(root_manifest).map_err(|_| {
            PortableWorkspaceFailure::source_rejected(WorkspaceError::InvalidCapturedSourceRoot)
        })
    }

    /// Bind captured documents to one logical root manifest.
    ///
    /// # Errors
    ///
    /// Returns a [`PortableWorkspaceFailure`] classified as
    /// [`PortableWorkspaceFailureClass::SourceRejected`] when the document count
    /// exceeds the finite grammar bound, a logical label repeats, or the
    /// admitted root document is absent.
    pub fn new(
        root_manifest: OwnedConfinedRelativePath,
        documents: Vec<CapturedWorkspaceDocument>,
    ) -> Result<Self, PortableWorkspaceFailure> {
        Self::validate_document_count(documents.len())?;
        let mut indexed = BTreeMap::new();
        for document in documents {
            match indexed.entry(document.label) {
                Entry::Vacant(entry) => {
                    entry.insert(document.bytes);
                }
                Entry::Occupied(entry) => {
                    return Err(PortableWorkspaceFailure::source_rejected(
                        WorkspaceError::DuplicateCapturedSource(entry.key().clone()),
                    ));
                }
            }
        }
        if !indexed.contains_key(root_manifest.as_str()) {
            return Err(PortableWorkspaceFailure::source_rejected(
                WorkspaceError::MissingCapturedSourceRoot(root_manifest.to_string()),
            ));
        }
        Ok(Self {
            root_manifest,
            documents: indexed,
        })
    }

    pub(super) fn root_manifest(&self) -> &Path {
        Path::new(self.root_manifest.as_str())
    }

    fn document_count(&self) -> Result<u32, WorkspaceError> {
        portable_count(
            "portable source document observations",
            self.documents.len(),
        )
    }
}

/// Share the member ceiling with host capture before either path retains bodies.
pub(super) fn validate_workspace_document_count(observed: usize) -> Result<(), WorkspaceError> {
    if observed > CapturedWorkspaceSource::MAX_DOCUMENTS {
        return Err(WorkspaceError::CapturedDocumentCountExceeded {
            observed,
            limit: CapturedWorkspaceSource::MAX_DOCUMENTS,
        });
    }
    Ok(())
}

impl WorkspaceDocumentSource for CapturedWorkspaceSource {
    fn read_regular<'source>(
        &'source self,
        path: &Path,
        limit: SourceReadLimit,
    ) -> Result<Cow<'source, [u8]>, WorkspaceError> {
        let rendered = path
            .to_str()
            .ok_or_else(|| WorkspaceError::EscapedReference(path.display().to_string()))?;
        let portable = if std::path::MAIN_SEPARATOR == '/' {
            Cow::Borrowed(rendered)
        } else {
            Cow::Owned(rendered.replace(std::path::MAIN_SEPARATOR, "/"))
        };
        let label = portable.strip_prefix("./").unwrap_or(&portable);
        if !is_captured_path(label) {
            return Err(WorkspaceError::EscapedReference(label.to_string()));
        }
        let bytes = self
            .documents
            .get(label)
            .ok_or_else(|| WorkspaceError::MissingReference {
                reference: label.to_string(),
                path: label.to_string(),
            })?;
        if bytes.len() > limit.max_bytes() {
            return Err(WorkspaceError::CapturedDocumentTooLarge {
                observed: bytes.len(),
                limit: limit.max_bytes(),
            });
        }
        Ok(Cow::Borrowed(bytes.as_slice()))
    }
}

/// Opaque admitted result of one path-free workspace-language compilation.
///
/// This value is not the one-shot Package [`crate::compile::Compilation`] and
/// is not a resident compiled generation. It proves only that the supplied
/// workspace documents passed the complete current workspace trust pipeline.
/// The underlying admitted universe remains private; the accessors expose
/// stable owned-boundary observations rather than compiler IR.
pub struct PortableWorkspaceUniverse {
    universe: Arc<PrincipalAdmittedUniverse>,
    desired_topology: WorkspaceDesiredTopologyV1,
    authoring_sources: super::AuthoringSourceMap,
    authoring_semantics: super::AuthoringSemanticIndex,
    source_document_count: u32,
    derivation_source_count: u32,
    derivation_node_count: u32,
    admission_evidence_count: u32,
    adopted_release_count: u32,
    package_release_count: u32,
    normative_pack_release_count: u32,
}

/// The compiler subject directly named by one indexed source occurrence.
///
/// Capturing a name and selecting its subject for the compiled proof are
/// separate facts. These results borrow this compilation; they grant no
/// recipient disclosure permission or native execution authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortableWorkspaceSourceSubject<'a> {
    /// No semantic occurrence is indexed at this path. The source may still exist.
    NoOccurrence,
    /// The occurrence has no supported direct subject mapping.
    NoDenotation {
        /// Captured occurrence, including its original role.
        occurrence: &'a super::AuthoringOccurrence,
    },
    /// The captured name denotes a subject absent from the selected proof.
    Unselected {
        /// Captured occurrence, not necessarily the subject's own definition.
        occurrence: &'a super::AuthoringOccurrence,
        /// Full typed subject named by the source producer.
        subject: &'a WorkspaceDerivationSubjectV1,
    },
    /// The captured name denotes a subject present in the selected proof.
    Selected {
        /// Captured occurrence, including its original role.
        occurrence: &'a super::AuthoringOccurrence,
        /// Full typed subject present in this compilation's proof map.
        subject: &'a WorkspaceDerivationSubjectV1,
    },
}

/// A subject's proof membership and one page of its direct named occurrences.
///
/// Both states can have zero or many occurrences. For example, a selected
/// policy has proof dependencies but no direct named occurrence in this index.
/// An empty incomplete page establishes only that its examined rows did not
/// match. Even complete index absence is not absence from native systems.
#[derive(Debug)]
pub enum PortableWorkspaceSubjectOccurrences<'a> {
    /// No selected proof subject; captured source can still name it.
    Unselected(super::AuthoringOccurrencePage<'a>),
    /// The proof subject is selected, independently of its occurrence count.
    Selected(super::AuthoringOccurrencePage<'a>),
}

/// Verified causal comparison between two admitted portable workspaces.
///
/// The receipt is reconstructed from both exact semantic owners; it is not a
/// caller-supplied diff and it contains no resident generation, native state,
/// or effect claim. Principal identities bind subsequent explanation queries
/// to the same compared owners without rebuilding the receipt.
pub struct PortableWorkspaceDerivation {
    prior_principal_digest: PrincipalAdmittedUniverseDigest,
    current_principal_digest: PrincipalAdmittedUniverseDigest,
    receipt: WorkspaceDerivationReceiptV1,
    root_count: u32,
    impact_count: u32,
    affected_count: u32,
    unaffected_count: u32,
}

impl std::fmt::Debug for PortableWorkspaceDerivation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PortableWorkspaceDerivation")
            .field("prior_principal_digest", &self.prior_principal_digest)
            .field("current_principal_digest", &self.current_principal_digest)
            .field("receipt_digest", &self.receipt.receipt_digest)
            .field("root_count", &self.root_count)
            .field("impact_count", &self.impact_count)
            .field("affected_count", &self.affected_count)
            .field("unaffected_count", &self.unaffected_count)
            .finish()
    }
}

/// Which immutable proof frontier owns an affected-subject witness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortableWorkspaceExplanationFrontier {
    /// The cause was removed, so the witness belongs to the predecessor DAG.
    Predecessor,
    /// The cause is present in the current DAG.
    Current,
}

/// Equality of one subject's typed output in two admitted compilations.
///
/// This does not classify dependency impact or native state. Source output
/// includes its logical label, byte digest and length; other variants retain
/// their compiler-owned digest domains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortableWorkspaceValueComparison {
    /// Both typed outputs are equal, even if their proof ancestry differs.
    Equal,
    /// The typed outputs differ.
    Changed,
}

/// Presence and borrowed outputs of one exact subject in a compilation pair.
///
/// Only a shared subject has two values to compare. Absence is not equality,
/// uncertainty, or evidence that a subject is unaffected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortableWorkspaceSubjectComparison<'a> {
    /// Neither compilation contains the subject.
    Absent,
    /// Only the predecessor contains the subject.
    PriorOnly {
        /// Predecessor's compiler-owned output.
        prior: &'a pale_core::records::WorkspaceDerivationOutputV1,
    },
    /// Only the current compilation contains the subject.
    CurrentOnly {
        /// Current compiler-owned output.
        current: &'a pale_core::records::WorkspaceDerivationOutputV1,
    },
    /// Both compilations contain this same full subject identity.
    Shared {
        /// Predecessor's compiler-owned output.
        prior: &'a pale_core::records::WorkspaceDerivationOutputV1,
        /// Current compiler-owned output.
        current: &'a pale_core::records::WorkspaceDerivationOutputV1,
        /// Application-owned comparison of the two typed outputs.
        comparison: PortableWorkspaceValueComparison,
    },
}

/// Exact causal classification for one queried subject.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortableWorkspaceDerivationExplanation {
    /// The subject is not part of the current receipt partition.
    NotPresent,
    /// The subject is current but unreachable from every exact change root.
    Unaffected,
    /// The subject is reachable from one deterministically selected root.
    Affected {
        /// Exact predecessor/current change that seeds the witness.
        root: Box<WorkspaceDerivationRootV1>,
        /// Admitted proof DAG traversed by `path`.
        frontier: PortableWorkspaceExplanationFrontier,
        /// Shortest path from the root fact through the queried subject.
        path: Vec<WorkspaceDerivationNodeV1>,
    },
}

impl PortableWorkspaceDerivation {
    /// Borrow the exact compiler-verified derivation receipt.
    ///
    /// The narrow public façade projects this record into owned use-case types;
    /// external consumers do not receive the raw proof graph.
    pub fn receipt(&self) -> &WorkspaceDerivationReceiptV1 {
        &self.receipt
    }

    /// Return exact source-change roots in this comparison.
    pub const fn root_count(&self) -> u32 {
        self.root_count
    }

    /// Return every current subject in the complete impact partition.
    pub const fn impact_count(&self) -> u32 {
        self.impact_count
    }

    /// Return current subjects reachable from at least one change root.
    pub const fn affected_count(&self) -> u32 {
        self.affected_count
    }

    /// Return the exact current complement outside every change-root closure.
    pub const fn unaffected_count(&self) -> u32 {
        self.unaffected_count
    }

    /// Explain one subject against the same compared admitted universes.
    ///
    /// Affected subjects return one shortest path. Root order is removed,
    /// modified, then added, with digest order breaking same-kind ties. This
    /// preserves a predecessor-only discontinuity when one contributes.
    /// Equal-length paths use their complete node-digest sequence. Other valid
    /// causes may still exist. Removed causes are explicitly labeled
    /// [`PortableWorkspaceExplanationFrontier::Predecessor`].
    ///
    /// # Errors
    ///
    /// Returns [`PortableWorkspaceFailure`] if the supplied universes are not
    /// the exact owners bound at comparison time, or if their already-verified
    /// proof contradicts its causal invariants.
    pub fn explain(
        &self,
        prior: &PortableWorkspaceUniverse,
        current: &PortableWorkspaceUniverse,
        subject: &WorkspaceDerivationSubjectV1,
    ) -> Result<PortableWorkspaceDerivationExplanation, PortableWorkspaceFailure> {
        if prior.universe.digest() != &self.prior_principal_digest
            || current.universe.digest() != &self.current_principal_digest
        {
            return Err(PortableWorkspaceFailure::from_workspace_error(
                WorkspaceError::DerivationSemanticMismatch,
            ));
        }
        current
            .universe
            .explain_portable_derivation(&prior.universe, &self.receipt, subject)
            .map(|explanation| match explanation {
                super::universe::WorkspaceDerivationExplanation::NotPresent => {
                    PortableWorkspaceDerivationExplanation::NotPresent
                }
                super::universe::WorkspaceDerivationExplanation::Unaffected => {
                    PortableWorkspaceDerivationExplanation::Unaffected
                }
                super::universe::WorkspaceDerivationExplanation::Affected {
                    root,
                    frontier,
                    path,
                } => PortableWorkspaceDerivationExplanation::Affected {
                    root,
                    frontier: match frontier {
                        super::universe::WorkspaceDerivationExplanationFrontier::Predecessor => {
                            PortableWorkspaceExplanationFrontier::Predecessor
                        }
                        super::universe::WorkspaceDerivationExplanationFrontier::Current => {
                            PortableWorkspaceExplanationFrontier::Current
                        }
                    },
                    path,
                },
            })
            .map_err(PortableWorkspaceFailure::from_workspace_error)
    }
}

impl std::fmt::Debug for PortableWorkspaceUniverse {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PortableWorkspaceUniverse")
            .field("workspace_id", &self.workspace_id())
            .field(
                "principal_universe_digest",
                &self.principal_universe_digest(),
            )
            .field("semantic_ir_digest", &self.semantic_ir_digest())
            .field("source_document_count", &self.source_document_count)
            .finish()
    }
}

impl PortableWorkspaceUniverse {
    pub(crate) fn shares_admitted_owner(&self, owner: &Arc<PrincipalAdmittedUniverse>) -> bool {
        Arc::ptr_eq(&self.universe, owner)
    }

    /// Enumerate the exact project and execution-workspace pairs admitted here.
    ///
    /// The desired-topology verifier already bounds this canonical relationship
    /// set. This projection does not infer pairs from matching names, repository
    /// membership, paths, or native state.
    pub fn profile_contexts(&self) -> impl ExactSizeIterator<Item = (&str, &str)> {
        self.desired_topology
            .project_execution_workspaces
            .iter()
            .map(|edge| {
                (
                    edge.project_id.as_str(),
                    edge.execution_workspace_id.as_str(),
                )
            })
    }

    /// Exact private owner allocated once with this compilation's source maps.
    /// No constructor accepts a caller-selected owner or replaces either map.
    pub(super) fn organization_owner(&self) -> &Arc<PrincipalAdmittedUniverse> {
        &self.universe
    }

    /// Borrow the selected root without restarting a completed input section.
    pub(super) fn subject_root(
        &self,
        subject: &WorkspaceDerivationSubjectV1,
    ) -> Option<&WorkspaceDerivationNodeV1> {
        self.universe.subject_node(subject)
    }

    /// Find the authored import path or extension record for this proof subject.
    /// Matching uses the complete subject. It does not retag named references
    /// inside that record, copy a source body, or invent a byte range.
    ///
    /// `None` means this owner has no supported record anchor; it does not prove
    /// subject absence. The borrow lasts only while this retained owner is alive.
    #[must_use]
    pub fn subject_record_anchor(
        &self,
        subject: &WorkspaceDerivationSubjectV1,
    ) -> Option<&super::WorkspaceAuthoringLocation> {
        self.authoring_semantics.subject_record_anchor(subject)
    }

    /// Read the direct compiler subject named at an exact source path.
    ///
    /// This performs a source-coordinate lookup, a full authoring-identity
    /// lookup and, when mapped, a selected-proof membership lookup. It does not
    /// derive subjects from labels or follow causal dependencies.
    #[must_use]
    pub fn source_subject(
        &self,
        source_label: &str,
        path: &[super::AuthoringInstancePathSegment],
    ) -> PortableWorkspaceSourceSubject<'_> {
        let Some(occurrence) = self.authoring_semantics.occurrence(source_label, path) else {
            return PortableWorkspaceSourceSubject::NoOccurrence;
        };
        let Some(subject) = self
            .authoring_semantics
            .proof_subject(occurrence.identity())
        else {
            return PortableWorkspaceSourceSubject::NoDenotation { occurrence };
        };
        if self.universe.contains_derivation_subject(subject) {
            PortableWorkspaceSourceSubject::Selected {
                occurrence,
                subject,
            }
        } else {
            PortableWorkspaceSourceSubject::Unselected {
                occurrence,
                subject,
            }
        }
    }

    /// Read proof membership and a bounded page of direct source occurrences.
    ///
    /// Zero selects 64 rows; either limit is independently capped at 256.
    /// Every inspected row is charged before a candidate-map lookup and full
    /// subject comparison. Work also includes a proof-membership lookup and an
    /// ordered-coordinate seek; it is not a constant-cost or byte-bound query.
    /// Hitting either limit returns a continuation without probing exhaustion.
    /// Definition, reference and projection roles remain distinct.
    ///
    /// The application-local cursor must be bound by its caller to this exact
    /// compilation, query kind, full subject and both raw requested limits.
    /// It is not interchangeable with identity-occurrence paging. Neither the
    /// cursor nor this supplied-input query establishes disclosure permission.
    #[must_use]
    pub fn subject_occurrences(
        &self,
        subject: &WorkspaceDerivationSubjectV1,
        after: Option<&super::AuthoringOccurrencePageCursor>,
        maximum_inspected: usize,
        maximum_returned: usize,
    ) -> PortableWorkspaceSubjectOccurrences<'_> {
        let selected = self.universe.contains_derivation_subject(subject);
        let page = self.authoring_semantics.subject_occurrences_page(
            subject,
            after,
            maximum_inspected,
            maximum_returned,
        );
        if selected {
            PortableWorkspaceSubjectOccurrences::Selected(page)
        } else {
            PortableWorkspaceSubjectOccurrences::Unselected(page)
        }
    }

    /// Read a page of one subject's direct compiler-proof dependencies.
    ///
    /// A missing subject returns `None`; a present source fact may have no
    /// dependencies. The query uses the retained subject index and sorted proof
    /// array. It neither builds a comparison receipt nor traverses closure.
    /// Zero selects 64 inputs; values above 256 are capped. The position is
    /// application-local: outer callers must use an owner-bound SDK cursor.
    ///
    /// # Errors
    ///
    /// Returns [`super::InvalidDependencyOffset`] when a present subject's
    /// requested position is beyond its input list.
    pub fn direct_dependencies(
        &self,
        subject: &WorkspaceDerivationSubjectV1,
        offset: usize,
        limit: u16,
    ) -> Result<Option<super::PortableWorkspaceDependencyPage<'_>>, super::InvalidDependencyOffset>
    {
        self.universe.direct_dependency_page(subject, offset, limit)
    }

    /// Find direct consumers of one selected compiler fact.
    ///
    /// Missing subjects return `None`. Otherwise candidates are inspected in
    /// sealed node-digest order, including nonmatches, until either independent
    /// limit is reached. Zero selects 64 for each limit; values above 256 cap at
    /// 256. Matching uses a binary search of each candidate's sorted inputs.
    /// The result reports actual input-digest comparisons and inspected nodes.
    /// A fixed aggregate comparison ceiling can stop the page before either row
    /// limit. Its next candidate remains in the continuation and is not skipped.
    ///
    /// An empty page can have more candidates. Exact-end positions return an
    /// empty complete page. This scans one-hop proof edges, not closure or a
    /// reverse index, and does not derive a comparison receipt. Limits bound
    /// candidate count, returned count and aggregate input comparisons; they do
    /// not bound elapsed time, payload bytes or disclosure. No resident
    /// materialization or recipient authority is implied.
    /// The local offset must be bound to owner, subject and both raw limits by
    /// any outer continuation carrier.
    ///
    /// # Errors
    /// Returns [`super::InvalidDependentOffset`] when a selected subject's
    /// position exceeds the retained candidate array.
    pub fn direct_dependents(
        &self,
        subject: &WorkspaceDerivationSubjectV1,
        offset: usize,
        maximum_inspected: u16,
        maximum_returned: u16,
    ) -> Result<Option<super::PortableWorkspaceDependentPage<'_>>, super::InvalidDependentOffset>
    {
        self.universe
            .direct_dependent_page(subject, offset, maximum_inspected, maximum_returned)
    }

    /// Enumerate selected subject identities from this input and its predecessor.
    ///
    /// The application opens two ordered ranges strictly after `after` and merges
    /// only enough entries for this page. Zero selects 64 rows; larger requests
    /// cap at 256. It reads at most two entries per returned row plus one look-ahead
    /// entry per side. It neither builds the full union nor computes its size.
    /// No graph traversal, value comparison, source join or receipt rebuild occurs.
    ///
    /// Work also includes two range seeks and variable-length full-key comparisons.
    /// Row/entry bounds do not bound bytes, elapsed time or retained output memory.
    /// The result borrows both inputs. Supplied positions are ordered lower bounds,
    /// not authority or portable cursors; the SDK must retain and bind their owners.
    #[must_use]
    pub fn comparison_subjects<'a>(
        &'a self,
        prior: &'a Self,
        after: Option<&WorkspaceDerivationSubjectV1>,
        requested_limit: u16,
    ) -> super::PortableWorkspaceComparisonSubjectPage<'a> {
        self.universe
            .comparison_subjects(&prior.universe, after, requested_limit)
    }

    /// Compare one subject's output with the supplied admitted predecessor.
    ///
    /// This reads the two retained fact maps. It does not rebuild a receipt or
    /// traverse a graph. Work includes two logarithmic lookups, full subject-key
    /// comparisons and typed output equality. The result borrows the outputs;
    /// source bytes are neither retained nor reconstructed.
    ///
    /// The supplied owners determine the comparison. Equal semantic digests do
    /// not substitute for them: comments or logical source labels can change
    /// source output without changing workspace meaning.
    #[must_use]
    pub fn compare_subject<'a>(
        &'a self,
        prior: &'a Self,
        subject: &WorkspaceDerivationSubjectV1,
    ) -> PortableWorkspaceSubjectComparison<'a> {
        self.universe
            .compare_portable_subject(&prior.universe, subject)
    }

    /// Borrow the exact source grammar selected during successful admission.
    ///
    /// The map retains logical labels and grammar identities only. It contains
    /// no source body or compiler IR and cannot be used as admission evidence.
    #[must_use]
    pub fn authoring_sources(&self) -> &super::AuthoringSourceMap {
        &self.authoring_sources
    }

    pub(super) fn organization_record_at(
        &self,
        position: super::organization_inspection::OrganizationPosition,
    ) -> Result<Option<super::OrganizationInspectionRecord<'_>>, super::OrganizationInspectionError>
    {
        self.universe.organization_record_at(position)
    }

    pub(super) fn organization_selected(&self) -> bool {
        self.universe.organization_selected()
    }

    /// Borrow source-free navigation evidence emitted by the canonical linker.
    ///
    /// Exact paths identify admitted TOML members, while the editor retains the
    /// corresponding text and ranges. This separation preserves post-admission
    /// source dropping and prevents a protocol leaf from re-resolving identity.
    #[must_use]
    pub fn authoring_semantics(&self) -> &super::AuthoringSemanticIndex {
        &self.authoring_semantics
    }

    /// Share the admitted semantic owner and select one compiler entry.
    ///
    /// The returned [`Arc`] remains private to the application layer. It lets a
    /// portable comparison handle and an effective-profile handle retain the
    /// exact same admitted universe without reparsing source or exposing shared
    /// ownership through the public façade.
    pub(crate) fn shared_compilation_parts(
        &self,
        project_id: &str,
        execution_workspace_id: &str,
    ) -> Result<
        (
            Arc<PrincipalAdmittedUniverse>,
            WorkspaceId,
            SelectedWorkspaceTopology,
        ),
        WorkspaceError,
    > {
        let selection =
            select_workspace_topology(&self.desired_topology, project_id, execution_workspace_id)?;
        let workspace_id = self.universe.workspace_id().clone();
        Ok((Arc::clone(&self.universe), workspace_id, selection))
    }

    pub(crate) fn selected_topology_digest(
        &self,
        project_id: &str,
        execution_workspace_id: &str,
    ) -> Result<String, WorkspaceError> {
        let selection =
            select_workspace_topology(&self.desired_topology, project_id, execution_workspace_id)?;
        super::topology::selected_topology_digest(&selection)
    }

    /// Borrow the workspace identity declared by the admitted root manifest.
    pub fn workspace_id(&self) -> &str {
        self.universe.workspace_id().as_str()
    }

    /// Borrow the principal-relative admitted universe digest.
    pub fn principal_universe_digest(&self) -> &str {
        self.universe.digest().as_str()
    }

    /// Borrow the verified workspace semantic-IR digest.
    pub fn semantic_ir_digest(&self) -> &str {
        self.universe.workspace_semantic_ir_digest().as_str()
    }

    /// Borrow the proof frontier consumed during owner-gated admission.
    pub fn admission_evidence_digest(&self) -> &str {
        self.universe.admission_evidence_digest().as_str()
    }

    /// Return the number of exact source documents consumed at ingress.
    pub const fn source_document_count(&self) -> u32 {
        self.source_document_count
    }

    /// Return the number of retained causal source coordinates.
    pub const fn derivation_source_count(&self) -> u32 {
        self.derivation_source_count
    }

    /// Return the number of retained causal derivation nodes.
    pub const fn derivation_node_count(&self) -> u32 {
        self.derivation_node_count
    }

    /// Return the number of exact owner-proof records consumed at admission.
    pub const fn admission_evidence_count(&self) -> u32 {
        self.admission_evidence_count
    }

    /// Return exact adopted module releases in the admitted closure.
    pub const fn adopted_release_count(&self) -> u32 {
        self.adopted_release_count
    }

    /// Return Package releases selected through workspace logical resources.
    pub const fn package_release_count(&self) -> u32 {
        self.package_release_count
    }

    /// Return attestor-scoped normative-Pack releases in the admitted closure.
    pub const fn normative_pack_release_count(&self) -> u32 {
        self.normative_pack_release_count
    }

    /// Reconstruct the exact causal comparison from `prior` into `self`.
    ///
    /// Source additions, modifications, and removals seed the receipt. Its
    /// impact partition classifies reachability, not merely output inequality:
    /// a fact may be affected even when its typed output remains equal.
    ///
    /// # Errors
    ///
    /// Returns [`PortableWorkspaceFailure`] only if two previously admitted
    /// universes contradict the derivation contract or canonical identity law.
    pub fn compare_with_predecessor(
        &self,
        prior: &Self,
    ) -> Result<PortableWorkspaceDerivation, PortableWorkspaceFailure> {
        let receipt = self
            .universe
            .portable_derivation_receipt(&prior.universe)
            .map_err(PortableWorkspaceFailure::from_workspace_error)?;
        let root_count = portable_count("portable derivation roots", receipt.roots.len())
            .map_err(PortableWorkspaceFailure::from_workspace_error)?;
        let impact_count = portable_count("portable derivation impacts", receipt.impacts.len())
            .map_err(PortableWorkspaceFailure::from_workspace_error)?;
        let affected_count = portable_count(
            "portable affected derivation subjects",
            receipt
                .impacts
                .iter()
                .filter(|impact| {
                    matches!(
                        &impact.state,
                        pale_core::records::WorkspaceDerivationImpactStateV1::Affected { .. }
                    )
                })
                .count(),
        )
        .map_err(PortableWorkspaceFailure::from_workspace_error)?;
        let unaffected_count = portable_count(
            "portable unaffected derivation subjects",
            receipt
                .impacts
                .iter()
                .filter(|impact| {
                    matches!(
                        &impact.state,
                        pale_core::records::WorkspaceDerivationImpactStateV1::Unaffected
                    )
                })
                .count(),
        )
        .map_err(PortableWorkspaceFailure::from_workspace_error)?;
        Ok(PortableWorkspaceDerivation {
            prior_principal_digest: prior.universe.digest().clone(),
            current_principal_digest: self.universe.digest().clone(),
            receipt,
            root_count,
            impact_count,
            affected_count,
            unaffected_count,
        })
    }
}

/// Compile captured workspace documents without reading files.
///
/// The transformation is deterministic and effect-free:
///
/// ```text
/// captured documents
///     -> parsed source
///     -> linked and normalized semantics
///     -> admitted universe
/// ```
///
/// Successful admission drops all captured source bytes. The returned universe
/// carries only typed semantic bodies, digests, and bounded causal coordinates.
/// It performs no Package distribution load, private machine resolution,
/// provider admission, resident activation, or native effect.
///
/// # Errors
///
/// Returns [`PortableWorkspaceFailure`] for malformed, structurally invalid,
/// unresolved, policy-widening, noncanonical, or owner-proof-blocked workspace
/// declarations.
pub fn compile_portable_workspace(
    source: CapturedWorkspaceSource,
) -> Result<PortableWorkspaceUniverse, PortableWorkspaceFailure> {
    let source_document_count = source
        .document_count()
        .map_err(PortableWorkspaceFailure::from_workspace_error)?;
    let parsed = parse_workspace_source(&source, source.root_manifest())
        .map_err(PortableWorkspaceFailure::from_workspace_error)?;
    let authoring_sources = super::AuthoringSourceMap::from_workspace(&parsed)
        .map_err(PortableWorkspaceFailure::from_workspace_error)?;
    let desired_topology = parsed.manifest.workspace.desired_topology.clone();
    let (universe, authoring_semantics) = admit_workspace_source_with_authoring(parsed)
        .map_err(PortableWorkspaceFailure::from_workspace_error)?;
    let derivation_source_count = portable_count(
        "portable derivation source observations",
        universe.derivation_sources().len(),
    )
    .map_err(PortableWorkspaceFailure::from_workspace_error)?;
    let derivation_node_count = portable_count(
        "portable derivation node observations",
        universe.derivation_nodes().len(),
    )
    .map_err(PortableWorkspaceFailure::from_workspace_error)?;
    let admission_evidence_count = portable_count(
        "portable admission evidence observations",
        universe.admission_evidence_count(),
    )
    .map_err(PortableWorkspaceFailure::from_workspace_error)?;
    let adopted_release_count = portable_count(
        "portable adopted release observations",
        universe.adopted_releases().len(),
    )
    .map_err(PortableWorkspaceFailure::from_workspace_error)?;
    let package_release_count = portable_count(
        "portable Package release observations",
        universe.package_releases().len(),
    )
    .map_err(PortableWorkspaceFailure::from_workspace_error)?;
    let normative_pack_release_count = portable_count(
        "portable normative-Pack release observations",
        universe.normative_pack_releases().len(),
    )
    .map_err(PortableWorkspaceFailure::from_workspace_error)?;
    Ok(PortableWorkspaceUniverse {
        universe: Arc::new(universe),
        desired_topology,
        authoring_sources,
        authoring_semantics,
        source_document_count,
        derivation_source_count,
        derivation_node_count,
        admission_evidence_count,
        adopted_release_count,
        package_release_count,
        normative_pack_release_count,
    })
}

fn portable_count(kind: &'static str, observed: usize) -> Result<u32, WorkspaceError> {
    u32::try_from(observed)
        .map_err(|_| WorkspaceError::PortableObservationOverflow { kind, observed })
}

fn is_captured_path(value: &str) -> bool {
    value.len() <= MAX_CONFINED_RELATIVE_PATH_BYTES && ConfinedRelativePath::parse(value).is_some()
}

#[cfg(test)]
#[path = "../../tests/internal/workspace/portable.rs"]
mod tests;
