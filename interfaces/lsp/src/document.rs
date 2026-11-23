//! Versioned, path-free configuration documents.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use lsp_types::{Position, Range, TextDocumentContentChangeEvent, Uri};
use pale_sdk::{AuthoringIngressClass, AuthoringIngressLimits, PaleApi};
use ropey::Rope;
use sha2::{Digest, Sha256};

/// Compiler source family assigned by the initializing editor host.
///
/// The role is protocol metadata, not a fact inferred from a URI, filename, or
/// document body. It selects only the existing [`pale_sdk`] input boundary;
/// parsing and semantic admission remain owned by that boundary.
#[derive(Clone, Copy, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
pub(super) enum DocumentRole {
    /// Root portable workspace manifest.
    WorkspaceManifest,
    /// Reusable workspace module.
    WorkspaceModule,
    /// Workspace-language primitive declarations.
    WorkspaceLanguagePrimitives,
    /// Responsibility derivation declarations.
    ResponsibilityDerivation,
    /// Organization-to-responsibility translation declarations.
    OrganizationalTranslation,
    /// Experience-composition declarations.
    ExperienceComposition,
    /// Exact module-resolution lock.
    WorkspaceResolutionLock,
    /// Exact workspace-universe lock.
    WorkspaceUniverseLock,
    /// Singular private capability-policy source.
    Capabilities,
    /// Singular private machine-binding source.
    MachineBinding,
    /// Singular distribution-lock source.
    DistributionLock,
    /// Repeated lifecycle-guidance source.
    LifecycleGuidance,
    /// Package manifest and resource inventory.
    PackageManifest,
    /// Package doctrine declarations.
    PackageDoctrine,
    /// Package operation declarations.
    PackageOperations,
    /// Package role declarations.
    PackageRoles,
    /// Package stack declarations.
    PackageStacks,
    /// Package run-mode declarations.
    PackageRunModes,
    /// Package policy-family declaration.
    PackagePolicyFamily,
    /// Package governing-order declaration.
    PackageGoverningOrder,
    /// Package capability-policy declaration.
    PackageCapabilityPolicy,
    /// Package tool-recipe declaration.
    PackageToolRecipe,
    /// Package page with a TOML declaration and Markdown body.
    PackagePage,
    /// Typed Package data document.
    PackageData,
    /// Immutable Package Markdown body or documentation.
    PackageText,
}

/// Explicit lexical treatment selected by the initializing editor host.
///
/// This is intentionally independent of [`DocumentRole`]: immutable Package
/// members may be either substantive Markdown or TOML declarations. Neither a
/// filename nor document content may silently select the tolerant TOML parser.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DocumentLanguage {
    /// Substantive text with no TOML authoring grammar.
    Text,
    /// Pale configuration TOML.
    Toml,
    /// A Package page whose TOML header has compiler-owned bounds.
    Page,
}

impl DocumentRole {
    /// Every admitted initialization role in stable protocol order.
    const ALL: [Self; 25] = [
        Self::WorkspaceManifest,
        Self::WorkspaceModule,
        Self::WorkspaceLanguagePrimitives,
        Self::ResponsibilityDerivation,
        Self::OrganizationalTranslation,
        Self::ExperienceComposition,
        Self::WorkspaceResolutionLock,
        Self::WorkspaceUniverseLock,
        Self::Capabilities,
        Self::MachineBinding,
        Self::DistributionLock,
        Self::LifecycleGuidance,
        Self::PackageManifest,
        Self::PackageDoctrine,
        Self::PackageOperations,
        Self::PackageRoles,
        Self::PackageStacks,
        Self::PackageRunModes,
        Self::PackagePolicyFamily,
        Self::PackageGoverningOrder,
        Self::PackageCapabilityPolicy,
        Self::PackageToolRecipe,
        Self::PackagePage,
        Self::PackageData,
        Self::PackageText,
    ];

    /// Serialize the closed role inventory through the same owner as request decoding.
    ///
    /// # Panics
    ///
    /// Panics only if Serde cannot represent a unit variant as its configured
    /// string, which would make this process's initialization protocol invalid.
    pub(super) fn protocol_names() -> Vec<String> {
        Self::ALL
            .into_iter()
            .map(|role| {
                serde_json::to_value(role)
                    .expect("DocumentRole serialization is infallible")
                    .as_str()
                    .expect("DocumentRole serializes as a string")
                    .to_owned()
            })
            .collect()
    }

    /// Whether this source participates in portable workspace admission.
    pub(super) const fn is_workspace(self) -> bool {
        matches!(
            self,
            Self::WorkspaceManifest
                | Self::WorkspaceModule
                | Self::WorkspaceLanguagePrimitives
                | Self::ResponsibilityDerivation
                | Self::OrganizationalTranslation
                | Self::ExperienceComposition
                | Self::WorkspaceResolutionLock
                | Self::WorkspaceUniverseLock
        )
    }

    /// Whether this source participates in immutable Package admission.
    pub(super) const fn is_package(self) -> bool {
        matches!(
            self,
            Self::PackageManifest
                | Self::PackageDoctrine
                | Self::PackageOperations
                | Self::PackageRoles
                | Self::PackageStacks
                | Self::PackageRunModes
                | Self::PackagePolicyFamily
                | Self::PackageGoverningOrder
                | Self::PackageCapabilityPolicy
                | Self::PackageToolRecipe
                | Self::PackagePage
                | Self::PackageData
                | Self::PackageText
        )
    }

    /// Canonical captured-input class that owns this role's resource limits.
    pub(super) const fn ingress_class(self) -> AuthoringIngressClass {
        if self.is_workspace() {
            AuthoringIngressClass::Workspace
        } else if self.is_package() {
            AuthoringIngressClass::Package
        } else {
            AuthoringIngressClass::Private
        }
    }

    /// Lexical editor identity required by this compiler source role.
    ///
    /// Role assignment remains explicit host metadata. This check only keeps
    /// that assignment coherent with the application-owned distinction between
    /// substantive text and TOML grammar; it does not infer either value from
    /// a filename or document body.
    pub(super) const fn language(self) -> DocumentLanguage {
        match self {
            Self::LifecycleGuidance | Self::PackageText => DocumentLanguage::Text,
            Self::PackagePage => DocumentLanguage::Page,
            Self::WorkspaceManifest
            | Self::WorkspaceModule
            | Self::WorkspaceLanguagePrimitives
            | Self::ResponsibilityDerivation
            | Self::OrganizationalTranslation
            | Self::ExperienceComposition
            | Self::WorkspaceResolutionLock
            | Self::WorkspaceUniverseLock
            | Self::Capabilities
            | Self::MachineBinding
            | Self::DistributionLock
            | Self::PackageManifest
            | Self::PackageDoctrine
            | Self::PackageOperations
            | Self::PackageRoles
            | Self::PackageStacks
            | Self::PackageRunModes
            | Self::PackagePolicyFamily
            | Self::PackageGoverningOrder
            | Self::PackageCapabilityPolicy
            | Self::PackageToolRecipe
            | Self::PackageData => DocumentLanguage::Toml,
        }
    }
}

/// One immutable configuration document captured during initialization.
#[derive(Clone, Debug, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
pub(super) struct BaselineDocument {
    /// Document URI used only for editor correlation and lexical confinement.
    #[cfg_attr(feature = "protocol-codegen", schemars(with = "String"))]
    pub(super) uri: Uri,
    /// Language identifier supplied by the editor.
    pub(super) language_id: String,
    /// Exact compiler source family selected without path or prose inference.
    pub(super) role: DocumentRole,
    /// Complete initial document text.
    pub(super) text: String,
}

/// One current open-buffer overlay.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct OpenDocument {
    uri: Uri,
    logical_label: String,
    role: DocumentRole,
    language: DocumentLanguage,
    version: i32,
    text: Rope,
}

impl OpenDocument {
    const fn version(&self) -> i32 {
        self.version
    }
}

/// One effective input in the saved-baseline-plus-overlay snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct AnalysisDocument {
    uri: Uri,
    logical_label: String,
    role: DocumentRole,
    language: DocumentLanguage,
    version: Option<i32>,
    text: SnapshotText,
}

/// Immutable text custody selected for one analysis snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
enum SnapshotText {
    /// Initialization source already contiguous at the protocol boundary.
    Contiguous(Arc<str>),
    /// Persistent open-buffer snapshot sharing Ropey tree nodes and chunks.
    Rope(Rope),
}

/// Exact saved-source content fence; availability remains a separate state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SourceFingerprint([u8; 32]);

impl SourceFingerprint {
    fn for_text(text: &str) -> Self {
        Self(Sha256::digest(text.as_bytes()).into())
    }

    fn for_rope(text: &Rope) -> Self {
        let mut digest = Sha256::new();
        for chunk in text.chunks() {
            digest.update(chunk.as_bytes());
        }
        Self(digest.finalize().into())
    }
}

impl AnalysisDocument {
    pub(super) const fn uri(&self) -> &Uri {
        &self.uri
    }

    pub(super) fn logical_label(&self) -> &str {
        &self.logical_label
    }

    pub(super) const fn role(&self) -> DocumentRole {
        self.role
    }

    /// Explicit lexical treatment supplied during initialization.
    pub(super) const fn language(&self) -> DocumentLanguage {
        self.language
    }

    pub(super) const fn version(&self) -> Option<i32> {
        self.version
    }

    /// Exact UTF-8 byte length without flattening persistent text.
    fn byte_len(&self) -> usize {
        match &self.text {
            SnapshotText::Contiguous(text) => text.len(),
            SnapshotText::Rope(text) => text.len_bytes(),
        }
    }

    pub(super) fn text(&self) -> Cow<'_, str> {
        match &self.text {
            SnapshotText::Contiguous(text) => Cow::Borrowed(text),
            SnapshotText::Rope(text) => Cow::Owned(text.to_string()),
        }
    }

    /// Flatten exact bytes once at the API parser or protocol-diagnostic boundary.
    pub(super) fn bytes(&self) -> Vec<u8> {
        match &self.text {
            SnapshotText::Contiguous(text) => text.as_bytes().to_vec(),
            SnapshotText::Rope(text) => {
                let mut bytes = Vec::with_capacity(text.len_bytes());
                for chunk in text.chunks() {
                    bytes.extend_from_slice(chunk.as_bytes());
                }
                bytes
            }
        }
    }

    /// Convert one LSP UTF-16 position into an exact UTF-8 byte offset.
    ///
    /// Positions inside a surrogate pair, after a line ending, or outside the
    /// retained snapshot are rejected rather than rounded.
    pub(super) fn byte_at_position(&self, position: Position) -> Option<usize> {
        match &self.text {
            SnapshotText::Contiguous(text) => contiguous_byte_at_position(text, position),
            SnapshotText::Rope(text) => {
                let character = utf16_char_offset(text, position)?;
                Some(text.char_to_byte(character))
            }
        }
    }

    /// Convert one exact UTF-8 byte range into LSP UTF-16 coordinates.
    pub(super) fn range_at_bytes(&self, start: usize, end: usize) -> Option<Range> {
        if start > end {
            return None;
        }
        match &self.text {
            SnapshotText::Contiguous(text) => Some(Range::new(
                contiguous_position_at_byte(text, start)?,
                contiguous_position_at_byte(text, end)?,
            )),
            SnapshotText::Rope(text) => Some(Range::new(
                rope_position_at_byte(text, start)?,
                rope_position_at_byte(text, end)?,
            )),
        }
    }
}

/// Immutable input coordinate for one analysis attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Snapshot {
    revision: u64,
    documents: Vec<AnalysisDocument>,
}

impl Snapshot {
    /// Process-local document-store revision captured by this snapshot.
    #[must_use]
    pub(super) const fn revision(&self) -> u64 {
        self.revision
    }

    /// Documents in deterministic logical-label order.
    #[must_use]
    pub(super) fn documents(&self) -> &[AnalysisDocument] {
        &self.documents
    }

    /// Documents assigned to one compiler source family, in snapshot order.
    pub(super) fn documents_with_role(
        &self,
        role: DocumentRole,
    ) -> impl Iterator<Item = &AnalysisDocument> {
        self.documents
            .iter()
            .filter(move |document| document.role() == role)
    }

    /// Immutable Package members in deterministic logical-label order.
    pub(super) fn package_documents(&self) -> impl Iterator<Item = &AnalysisDocument> {
        self.documents
            .iter()
            .filter(|document| document.role().is_package())
    }

    /// Find one document by its exact protocol URI.
    pub(super) fn document(&self, uri: &Uri) -> Option<&AnalysisDocument> {
        self.documents.iter().find(|document| document.uri() == uri)
    }
}

/// Exact mutation-amplification counters visible at the LSP-owned boundary.
///
/// Ropey's internal node allocation remains its implementation detail. These
/// counters cover bytes logically touched and bytes this adapter necessarily
/// copies into Ropey; parser/protocol flattening is measured separately by the
/// returned contiguous vector length. The rendered boundary law distinguishes
/// this LSP-owned copy count from copy accounting in every other adapter.
/// Aggregate replacement input is copied into the rope, snapshots share
/// buffers, and flattening copies the complete current text. In that law, `I`
/// is the admitted replacement input, `T` is the complete current text, and
/// every `B` value is a byte count.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/rope-mutation-amplification.display.mathml"
))]
pub(super) struct TextMutationMetrics {
    /// Deleted plus inserted UTF-8 bytes in the admitted change batch.
    pub(super) logical_bytes_touched: usize,
    /// Replacement bytes copied into the mutable rope by this boundary.
    pub(super) replacement_bytes_copied: usize,
}

/// Failure while admitting an editor overlay transition.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub(super) enum DocumentError {
    /// The editor attempted to route a downstream programming language to Pale.
    #[error("language `{0}` is outside the Pale configuration LSP")]
    UnsupportedLanguage(String),
    /// The document is not lexically contained by the initialized root URI.
    #[error("document URI is outside the initialized Pale workspace root")]
    OutsideRoot,
    /// Another URI already owns the derived logical label.
    #[error("logical document label `{0}` is already present at another URI")]
    DuplicateLogicalLabel(String),
    /// An opened document lacks an initialization-time compiler role.
    #[error("document has no explicitly registered Pale compiler role")]
    UnregisteredRole,
    /// An open notification attempted to change the registered lexical role.
    #[error("document language does not match its initialization-time registration")]
    LanguageMismatch,
    /// Initialization paired a compiler role with the wrong lexical language.
    #[error("document language does not match its explicitly registered compiler role")]
    RoleLanguageMismatch,
    /// The initialization root carries a query, fragment, or non-absolute path.
    #[error("initialized Pale workspace root URI is not a plain absolute URI")]
    InvalidRoot,
    /// A document path contains non-UTF-8 percent-decoded bytes.
    #[error("document URI path is not valid decoded UTF-8")]
    InvalidEncoding,
    /// A change referred to a document that is not open.
    #[error("document is not open")]
    NotOpen,
    /// Save text disagreed with the synchronized open-buffer overlay.
    #[error("saved document text does not match the synchronized open buffer")]
    SaveTextMismatch,
    /// The incoming version did not advance the current overlay.
    #[error("document version {incoming} does not advance {current}")]
    StaleVersion { current: i32, incoming: i32 },
    /// A UTF-16 range did not identify a valid text boundary.
    #[error("change range is outside the current UTF-16 document")]
    InvalidRange,
    /// The process-local overlay revision has no representable successor.
    #[error("document-store revision is exhausted")]
    RevisionExhausted,
    /// A source body exceeds the application-owned per-document ceiling.
    #[error("{class} document bytes exceed the authoring ingress limit: {observed} > {maximum}")]
    DocumentBytesExceeded {
        class: &'static str,
        observed: u64,
        maximum: u64,
    },
    /// A custody class contains too many logical documents.
    #[error("{class} document count exceeds the authoring ingress limit: {observed} > {maximum}")]
    DocumentCountExceeded {
        class: &'static str,
        observed: usize,
        maximum: usize,
    },
    /// Aggregate source bytes for one custody class exceed its ceiling.
    #[error("{class} source bytes exceed the authoring ingress limit: {observed} > {maximum}")]
    TotalBytesExceeded {
        class: &'static str,
        observed: u64,
        maximum: u64,
    },
}

/// In-memory saved-baseline-plus-overlay store with no filesystem capability.
///
/// In the rendered law, `A` is the immutable initialization comparison anchor,
/// `B_saved` is the sparse map of latest client-observed saved replacements,
/// `O` is the open overlay map, and `r` is the effective snapshot revision.
/// The right-biased union means an overlay replaces its saved document, while
/// [`Self::save`] advances only `B_saved` and [`Self::close`] reveals those
/// latest saved bytes. None of these transport states is interpreted here;
/// [`pale_sdk::PaleApi`] remains the semantic owner.
#[doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/document-overlay-snapshot.display.mathml"
))]
pub(super) struct DocumentStore {
    root_uri: Uri,
    baseline: BTreeMap<String, AnalysisDocument>,
    saved_replacements: BTreeMap<String, AnalysisDocument>,
    saved_fingerprints: BTreeMap<String, SourceFingerprint>,
    unavailable: BTreeSet<String>,
    overlays: BTreeMap<String, OpenDocument>,
    revision: u64,
    last_change_metrics: TextMutationMetrics,
}

impl DocumentStore {
    /// Admit one complete initialization baseline under a lexical root URI.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] when the root or a baseline document is
    /// unconfined, duplicated, or not a Pale configuration language.
    pub(super) fn new(
        root_uri: &Uri,
        documents: Vec<BaselineDocument>,
    ) -> Result<Self, DocumentError> {
        if root_uri.query().is_some()
            || root_uri.fragment().is_some()
            || !root_uri.path().as_str().starts_with('/')
        {
            return Err(DocumentError::InvalidRoot);
        }
        let root_uri = if root_uri.path().as_str().ends_with('/') {
            root_uri.clone()
        } else {
            format!("{}/", root_uri.as_str())
                .parse()
                .map_err(|_| DocumentError::InvalidRoot)?
        };
        let mut store = Self {
            root_uri,
            baseline: BTreeMap::new(),
            saved_replacements: BTreeMap::new(),
            saved_fingerprints: BTreeMap::new(),
            unavailable: BTreeSet::new(),
            overlays: BTreeMap::new(),
            revision: 0,
            last_change_metrics: TextMutationMetrics::default(),
        };
        validate_baseline_limits(&documents)?;
        for document in documents {
            let language = configuration_language(&document.language_id)?;
            if language != document.role.language() {
                return Err(DocumentError::RoleLanguageMismatch);
            }
            let logical_label = store.logical_label(&document.uri)?;
            if store.baseline.contains_key(&logical_label) {
                return Err(DocumentError::DuplicateLogicalLabel(logical_label));
            }
            let fingerprint = SourceFingerprint::for_text(&document.text);
            store.baseline.insert(
                logical_label.clone(),
                AnalysisDocument {
                    uri: document.uri,
                    logical_label: logical_label.clone(),
                    role: document.role,
                    language,
                    version: None,
                    text: SnapshotText::Contiguous(Arc::from(document.text)),
                },
            );
            store.saved_fingerprints.insert(logical_label, fingerprint);
        }
        Ok(store)
    }

    /// Admit one opened Pale configuration buffer.
    ///
    /// A repeated open for the same URI is a versioned replacement, not
    /// permission to roll the buffer back. The overlay transition therefore
    /// remains monotone even when an editor repeats lifecycle events.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] for an unsupported language, an unconfined or
    /// colliding label, non-advancing duplicate open, or revision exhaustion.
    pub(super) fn open(
        &mut self,
        uri: Uri,
        language_id: String,
        version: i32,
        text: String,
    ) -> Result<(), DocumentError> {
        let language = configuration_language(&language_id)?;
        let logical_label = self.logical_label(&uri)?;
        if self
            .overlays
            .values()
            .any(|document| document.logical_label == logical_label && document.uri != uri)
            || self
                .baseline
                .get(&logical_label)
                .is_some_and(|document| document.uri != uri)
        {
            return Err(DocumentError::DuplicateLogicalLabel(logical_label));
        }
        let registered = self
            .baseline
            .get(&logical_label)
            .filter(|document| document.uri == uri)
            .ok_or(DocumentError::UnregisteredRole)?;
        if language != registered.language() {
            return Err(DocumentError::LanguageMismatch);
        }
        let role = registered.role();
        self.validate_replacement(role, &logical_label, text.len())?;
        if let Some(current) = self.overlays.get(&logical_label) {
            if version <= current.version {
                return Err(DocumentError::StaleVersion {
                    current: current.version,
                    incoming: version,
                });
            }
        }
        let next_revision = self
            .revision
            .checked_add(1)
            .ok_or(DocumentError::RevisionExhausted)?;
        self.overlays.insert(
            logical_label.clone(),
            OpenDocument {
                uri,
                logical_label,
                role,
                language,
                version,
                text: Rope::from_str(&text),
            },
        );
        self.revision = next_revision;
        Ok(())
    }

    /// Apply ordered incremental or full-document changes to a newer version.
    ///
    /// The complete batch is evaluated against a structurally shared candidate
    /// rope and committed
    /// only if every UTF-16 range is valid. A malformed notification therefore
    /// cannot leave a partially updated editor overlay.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] for a missing document, stale version, invalid
    /// UTF-16 range, or exhausted store revision.
    pub(super) fn change(
        &mut self,
        uri: &Uri,
        version: i32,
        changes: Vec<TextDocumentContentChangeEvent>,
    ) -> Result<(), DocumentError> {
        let logical_label = self.logical_label(uri)?;
        let current_version = self
            .overlays
            .get(&logical_label)
            .map(OpenDocument::version)
            .ok_or(DocumentError::NotOpen)?;
        if version <= current_version {
            return Err(DocumentError::StaleVersion {
                current: current_version,
                incoming: version,
            });
        }
        let next_revision = self
            .revision
            .checked_add(1)
            .ok_or(DocumentError::RevisionExhausted)?;
        let document = self
            .overlays
            .get(&logical_label)
            .expect("the admitted open document remains present during this transition");
        let role = document.role;
        let logical_label = document.logical_label.clone();
        let mut candidate = document.text.clone();
        let mut metrics = TextMutationMetrics::default();
        for change in changes {
            match change.range {
                Some(range) => {
                    let (start, end) = utf16_char_range(&candidate, range)?;
                    let removed = candidate.char_to_byte(end) - candidate.char_to_byte(start);
                    let candidate_bytes = candidate
                        .len_bytes()
                        .checked_sub(removed)
                        .and_then(|bytes| bytes.checked_add(change.text.len()))
                        .ok_or(DocumentError::DocumentBytesExceeded {
                            class: ingress_class_name(role.ingress_class()),
                            observed: u64::MAX,
                            maximum: PaleApi::authoring_ingress_limits(role.ingress_class())
                                .max_document_bytes(),
                        })?;
                    self.validate_replacement(role, &logical_label, candidate_bytes)?;
                    let removed = replace_utf16_range(&mut candidate, range, &change.text)?;
                    metrics.logical_bytes_touched = metrics
                        .logical_bytes_touched
                        .saturating_add(removed)
                        .saturating_add(change.text.len());
                    metrics.replacement_bytes_copied = metrics
                        .replacement_bytes_copied
                        .saturating_add(change.text.len());
                }
                None => {
                    self.validate_replacement(role, &logical_label, change.text.len())?;
                    metrics.logical_bytes_touched = metrics
                        .logical_bytes_touched
                        .saturating_add(candidate.len_bytes())
                        .saturating_add(change.text.len());
                    metrics.replacement_bytes_copied = metrics
                        .replacement_bytes_copied
                        .saturating_add(change.text.len());
                    candidate = Rope::from_str(&change.text);
                }
            }
        }
        let document = self
            .overlays
            .get_mut(&logical_label)
            .expect("the admitted open document remains present during this transition");
        document.text = candidate;
        document.version = version;
        self.revision = next_revision;
        self.last_change_metrics = metrics;
        Ok(())
    }

    /// Return source-visible copy counters from the last admitted edit.
    #[cfg(feature = "bench")]
    pub(super) fn last_change_metrics(&self) -> TextMutationMetrics {
        self.last_change_metrics
    }

    /// Check a prospective body against the same class limits as initialization.
    fn validate_replacement(
        &self,
        role: DocumentRole,
        logical_label: &str,
        byte_len: usize,
    ) -> Result<(), DocumentError> {
        let class = role.ingress_class();
        let limits = PaleApi::authoring_ingress_limits(class);
        validate_document_bytes(class, limits, byte_len)?;
        let total = self
            .baseline
            .iter()
            .filter(|(_, document)| document.role().ingress_class() == class)
            .try_fold(0_u64, |total, (label, baseline)| {
                let bytes = if label == logical_label {
                    byte_len
                } else if let Some(overlay) = self.overlays.get(label) {
                    overlay.text.len_bytes()
                } else if self.unavailable.contains(label) {
                    0
                } else {
                    self.saved_replacements
                        .get(label)
                        .map_or_else(|| baseline.byte_len(), AnalysisDocument::byte_len)
                };
                total.checked_add(bytes as u64)
            })
            .ok_or(DocumentError::TotalBytesExceeded {
                class: ingress_class_name(class),
                observed: u64::MAX,
                maximum: limits.max_total_bytes(),
            })?;
        if total > limits.max_total_bytes() {
            return Err(DocumentError::TotalBytesExceeded {
                class: ingress_class_name(class),
                observed: total,
                maximum: limits.max_total_bytes(),
            });
        }
        Ok(())
    }

    /// Promote the current overlay to the latest client-observed saved baseline.
    ///
    /// The optional text supplied by the protocol must match the synchronized
    /// overlay exactly. Omitting it is valid because the save notification still
    /// identifies the already synchronized buffer as saved. This transition does
    /// not advance [`Self::revision`] or schedule analysis: the effective open
    /// snapshot is byte-identical before and after it, and the immutable
    /// comparison anchor returned by [`Self::baseline_snapshot`] never moves.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError::NotOpen`] when the document has no current
    /// overlay, or [`DocumentError::SaveTextMismatch`] when supplied save text
    /// differs from that overlay. Either failure leaves all custody unchanged.
    pub(super) fn save(
        &mut self,
        uri: &Uri,
        supplied_text: Option<&str>,
    ) -> Result<(), DocumentError> {
        let logical_label = self.logical_label(uri)?;
        let overlay = self
            .overlays
            .get(&logical_label)
            .filter(|document| &document.uri == uri)
            .ok_or(DocumentError::NotOpen)?;
        if supplied_text.is_some_and(|text| overlay.text != text) {
            return Err(DocumentError::SaveTextMismatch);
        }
        self.saved_replacements.insert(
            logical_label.clone(),
            AnalysisDocument {
                uri: overlay.uri.clone(),
                logical_label: logical_label.clone(),
                role: overlay.role,
                language: overlay.language,
                version: None,
                text: SnapshotText::Rope(overlay.text.clone()),
            },
        );
        self.saved_fingerprints.insert(
            logical_label.clone(),
            SourceFingerprint::for_rope(&overlay.text),
        );
        self.unavailable.remove(&logical_label);
        Ok(())
    }

    /// Return whether one URI is an exact initialization-registered source.
    pub(super) fn is_registered(&self, uri: &Uri) -> bool {
        self.logical_label(uri)
            .ok()
            .and_then(|label| self.baseline.get(&label))
            .is_some_and(|document| document.uri() == uri)
    }

    /// Borrow the exact initialization root used to confine registered sources.
    #[must_use]
    pub(super) const fn root_uri(&self) -> &Uri {
        &self.root_uri
    }

    /// Replace the saved bytes beneath any open overlay.
    ///
    /// The fingerprint suppresses duplicate change/create events, while exact
    /// text equality keeps the fence collision-safe. A changed saved source
    /// advances the document revision even when an open overlay remains the
    /// effective compiler input, so already-running work cannot publish across
    /// a changed close fallback.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] for an unregistered URI, an ingress-limit
    /// violation, or revision exhaustion. Failures retain no candidate bytes.
    pub(super) fn replace_saved(&mut self, uri: &Uri, text: String) -> Result<bool, DocumentError> {
        let logical_label = self.logical_label(uri)?;
        let registered = self
            .baseline
            .get(&logical_label)
            .filter(|document| document.uri() == uri)
            .cloned()
            .ok_or(DocumentError::UnregisteredRole)?;
        self.validate_replacement(registered.role(), &logical_label, text.len())?;
        let fingerprint = SourceFingerprint::for_text(&text);
        let current = self
            .saved_replacements
            .get(&logical_label)
            .unwrap_or(&registered);
        if !self.unavailable.contains(&logical_label)
            && self.saved_fingerprints.get(&logical_label) == Some(&fingerprint)
            && current.text().as_ref() == text
        {
            return Ok(false);
        }
        let next_revision = self
            .revision
            .checked_add(1)
            .ok_or(DocumentError::RevisionExhausted)?;
        self.saved_replacements.insert(
            logical_label.clone(),
            AnalysisDocument {
                uri: registered.uri,
                logical_label: logical_label.clone(),
                role: registered.role,
                language: registered.language,
                version: None,
                text: SnapshotText::Contiguous(Arc::from(text)),
            },
        );
        self.saved_fingerprints
            .insert(logical_label.clone(), fingerprint);
        self.unavailable.remove(&logical_label);
        self.revision = next_revision;
        Ok(true)
    }

    /// Mark an exact configured source unavailable without forgetting its role.
    ///
    /// An open overlay remains effective. Once it closes, the unavailable source
    /// is absent from the compiler snapshot rather than falling back to stale
    /// saved bytes.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] for an unregistered URI or revision exhaustion.
    pub(super) fn mark_unavailable(&mut self, uri: &Uri) -> Result<bool, DocumentError> {
        let logical_label = self.logical_label(uri)?;
        self.baseline
            .get(&logical_label)
            .filter(|document| document.uri() == uri)
            .ok_or(DocumentError::UnregisteredRole)?;
        if self.unavailable.contains(&logical_label) {
            return Ok(false);
        }
        let next_revision = self
            .revision
            .checked_add(1)
            .ok_or(DocumentError::RevisionExhausted)?;
        self.saved_replacements.remove(&logical_label);
        self.saved_fingerprints.remove(&logical_label);
        self.unavailable.insert(logical_label);
        self.revision = next_revision;
        Ok(true)
    }

    /// Drop one overlay, revealing its latest saved baseline document.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError::RevisionExhausted`] when removing a present
    /// document would require an unrepresentable successor revision.
    pub(super) fn close(&mut self, uri: &Uri) -> Result<(), DocumentError> {
        let logical_label = self.logical_label(uri)?;
        if self.overlays.contains_key(&logical_label) {
            let next_revision = self
                .revision
                .checked_add(1)
                .ok_or(DocumentError::RevisionExhausted)?;
            self.overlays.remove(&logical_label);
            self.revision = next_revision;
        }
        Ok(())
    }

    /// Capture the rendered saved-baseline-overlay union in deterministic order.
    pub(super) fn snapshot(&self) -> Snapshot {
        let mut documents = self.baseline.clone();
        documents.extend(self.saved_replacements.clone());
        for unavailable in &self.unavailable {
            documents.remove(unavailable);
        }
        for overlay in self.overlays.values() {
            documents.insert(
                overlay.logical_label.clone(),
                AnalysisDocument {
                    uri: overlay.uri.clone(),
                    logical_label: overlay.logical_label.clone(),
                    role: overlay.role,
                    language: overlay.language,
                    version: Some(overlay.version),
                    text: SnapshotText::Rope(overlay.text.clone()),
                },
            );
        }
        Snapshot {
            revision: self.revision,
            documents: documents.into_values().collect(),
        }
    }

    /// Return the immutable initialization baseline `B`.
    ///
    /// Unlike [`Self::snapshot`], this projection never contains open-buffer
    /// overlays. The analysis owner uses it as the fixed predecessor of every
    /// admitted comparison returned by [`Self::snapshot`]; editor changes
    /// cannot silently move the comparison coordinate.
    #[must_use]
    pub(super) fn baseline_snapshot(&self) -> Snapshot {
        Snapshot {
            revision: 0,
            documents: self.baseline.values().cloned().collect(),
        }
    }

    /// Current process-local revision without cloning document bodies.
    #[must_use]
    pub(super) const fn revision(&self) -> u64 {
        self.revision
    }

    /// Return whether a result still describes the current overlays.
    pub(super) const fn is_current(&self, snapshot: &Snapshot) -> bool {
        self.revision == snapshot.revision
    }

    /// Derive a root-relative label without resolving a filesystem path.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError::OutsideRoot`] unless the URI is a confined,
    /// nonempty descendant without traversal or encoded separators.
    fn logical_label(&self, uri: &Uri) -> Result<String, DocumentError> {
        uri.as_str()
            .strip_prefix(self.root_uri.as_str())
            .ok_or(DocumentError::OutsideRoot)?;
        if uri.query().is_some() || uri.fragment().is_some() {
            return Err(DocumentError::OutsideRoot);
        }
        let root_prefix_segments = self.root_uri.path().segments().count().saturating_sub(1);
        let decoded = uri
            .path()
            .segments()
            .skip(root_prefix_segments)
            .map(|segment| {
                segment
                    .decode()
                    .into_string()
                    .map(|segment| segment.into_owned())
                    .map_err(|_| DocumentError::InvalidEncoding)
            })
            .collect::<Result<Vec<_>, _>>()?;
        if decoded.is_empty()
            || decoded.iter().any(|segment| {
                segment.is_empty()
                    || segment == "."
                    || segment == ".."
                    || segment.contains('/')
                    || segment.contains('\\')
                    || segment.contains('\0')
            })
        {
            return Err(DocumentError::OutsideRoot);
        }
        Ok(decoded.join("/"))
    }
}

fn configuration_language(language_id: &str) -> Result<DocumentLanguage, DocumentError> {
    match language_id {
        "pale" => Ok(DocumentLanguage::Text),
        "pale-toml" => Ok(DocumentLanguage::Toml),
        "pale-page" => Ok(DocumentLanguage::Page),
        other => Err(DocumentError::UnsupportedLanguage(other.to_owned())),
    }
}

fn validate_baseline_limits(documents: &[BaselineDocument]) -> Result<(), DocumentError> {
    for class in [
        AuthoringIngressClass::Workspace,
        AuthoringIngressClass::Private,
        AuthoringIngressClass::Package,
    ] {
        let limits = PaleApi::authoring_ingress_limits(class);
        let mut count = 0_usize;
        let mut total = 0_u64;
        for document in documents
            .iter()
            .filter(|document| document.role.ingress_class() == class)
        {
            count = count.saturating_add(1);
            validate_document_bytes(class, limits, document.text.len())?;
            total = total.checked_add(document.text.len() as u64).ok_or(
                DocumentError::TotalBytesExceeded {
                    class: ingress_class_name(class),
                    observed: u64::MAX,
                    maximum: limits.max_total_bytes(),
                },
            )?;
        }
        if count > limits.max_documents() {
            return Err(DocumentError::DocumentCountExceeded {
                class: ingress_class_name(class),
                observed: count,
                maximum: limits.max_documents(),
            });
        }
        if total > limits.max_total_bytes() {
            return Err(DocumentError::TotalBytesExceeded {
                class: ingress_class_name(class),
                observed: total,
                maximum: limits.max_total_bytes(),
            });
        }
    }
    Ok(())
}

fn validate_document_bytes(
    class: AuthoringIngressClass,
    limits: AuthoringIngressLimits,
    byte_len: usize,
) -> Result<(), DocumentError> {
    let observed = byte_len as u64;
    if observed > limits.max_document_bytes() {
        Err(DocumentError::DocumentBytesExceeded {
            class: ingress_class_name(class),
            observed,
            maximum: limits.max_document_bytes(),
        })
    } else {
        Ok(())
    }
}

const fn ingress_class_name(class: AuthoringIngressClass) -> &'static str {
    match class {
        AuthoringIngressClass::Workspace => "workspace",
        AuthoringIngressClass::Private => "private",
        AuthoringIngressClass::Package => "Package",
    }
}

fn utf16_char_range(text: &Rope, range: Range) -> Result<(usize, usize), DocumentError> {
    let start = utf16_char_offset(text, range.start).ok_or(DocumentError::InvalidRange)?;
    let end = utf16_char_offset(text, range.end).ok_or(DocumentError::InvalidRange)?;
    (start <= end)
        .then_some((start, end))
        .ok_or(DocumentError::InvalidRange)
}

fn replace_utf16_range(
    text: &mut Rope,
    range: Range,
    replacement: &str,
) -> Result<usize, DocumentError> {
    let (start, end) = utf16_char_range(text, range)?;
    let removed = text.char_to_byte(end) - text.char_to_byte(start);
    text.remove(start..end);
    text.insert(start, replacement);
    Ok(removed)
}

fn utf16_char_offset(text: &Rope, position: Position) -> Option<usize> {
    let line_index = usize::try_from(position.line).ok()?;
    let line = text.get_line(line_index)?;
    let line_start = text.line_to_char(line_index);
    let mut units = 0_u32;
    let mut offset = 0_usize;
    for ch in line.chars() {
        if matches!(ch, '\r' | '\n') {
            break;
        }
        if units == position.character {
            return Some(line_start + offset);
        }
        units = units.checked_add(ch.len_utf16() as u32)?;
        if units > position.character {
            return None;
        }
        offset += 1;
    }
    (units == position.character).then_some(line_start + offset)
}

fn contiguous_byte_at_position(text: &str, position: Position) -> Option<usize> {
    let line = usize::try_from(position.line).ok()?;
    let line_start = if line == 0 {
        0
    } else {
        text.match_indices('\n').nth(line - 1)?.0 + 1
    };
    let line_text = text.get(line_start..)?.split('\n').next()?;
    let mut units = 0_u32;
    for (offset, character) in line_text.char_indices() {
        if character == '\r' {
            break;
        }
        if units == position.character {
            return Some(line_start + offset);
        }
        units = units.checked_add(character.len_utf16() as u32)?;
        if units > position.character {
            return None;
        }
    }
    (units == position.character).then_some(line_start + line_text.trim_end_matches('\r').len())
}

fn contiguous_position_at_byte(text: &str, byte: usize) -> Option<Position> {
    let prefix = text.get(..byte)?;
    let line = u32::try_from(prefix.bytes().filter(|byte| *byte == b'\n').count()).ok()?;
    let line_start = prefix.rfind('\n').map_or(0, |position| position + 1);
    let character = u32::try_from(prefix.get(line_start..)?.encode_utf16().count()).ok()?;
    Some(Position::new(line, character))
}

fn rope_position_at_byte(text: &Rope, byte: usize) -> Option<Position> {
    if byte > text.len_bytes() {
        return None;
    }
    let character = text.byte_to_char(byte);
    if text.char_to_byte(character) != byte {
        return None;
    }
    let line = text.char_to_line(character);
    let line_start = text.line_to_char(line);
    let utf16 = text
        .slice(line_start..character)
        .chars()
        .map(char::len_utf16)
        .sum::<usize>();
    Some(Position::new(
        u32::try_from(line).ok()?,
        u32::try_from(utf16).ok()?,
    ))
}

#[cfg(test)]
#[path = "../tests/internal/document.rs"]
mod tests;
