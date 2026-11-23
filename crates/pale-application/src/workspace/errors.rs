//! Typed failures while resolving and snapshotting a private workspace.
//!
//! [`WorkspaceError`] preserves source admission, path confinement, declarative
//! identity, private binding, and cross-record invariant failures. It does not
//! treat a readable file as an admitted workspace input.

use std::ops::Range;
use std::path::PathBuf;

use super::WorkspaceAuthoringLocation;

use pale_core::{
    canon,
    records::{
        LogicalRepositoryIdV1, WorkspaceExportId, WorkspaceModuleId,
        WorkspacePolicyCombinationError, WorkspaceVisibilityV1,
    },
};

/// Best honest source coordinate retained while admitting one workspace file.
///
/// [`Self::line`], [`Self::column`], and [`Self::span`] are present only when
/// the owning parser reported a byte span into the original UTF-8 text. Later
/// TOML-to-JSON validation and typed decoding retain the file and value path but
/// do not invent source positions after that representation change.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceSourceCoordinate {
    path: PathBuf,
    line: Option<usize>,
    column: Option<usize>,
    span: Option<Range<usize>>,
}

impl WorkspaceSourceCoordinate {
    pub(crate) fn path_only(path: &std::path::Path) -> Self {
        Self {
            path: path.to_path_buf(),
            line: None,
            column: None,
            span: None,
        }
    }

    pub(crate) fn with_span(
        path: &std::path::Path,
        span: Range<usize>,
        line: usize,
        column: usize,
    ) -> Self {
        Self {
            path: path.to_path_buf(),
            line: Some(line),
            column: Some(column),
            span: Some(span),
        }
    }

    /// Borrow the exact workspace file path.
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// Return the one-based source line when the original parser supplied a span.
    pub fn line(&self) -> Option<usize> {
        self.line
    }

    /// Return the one-based Unicode-scalar column when available.
    pub fn column(&self) -> Option<usize> {
        self.column
    }

    /// Borrow the exact byte range supplied by the original parser.
    pub fn span(&self) -> Option<&Range<usize>> {
        self.span.as_ref()
    }
}

impl std::fmt::Display for WorkspaceSourceCoordinate {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.path.display())?;
        if let (Some(line), Some(column)) = (self.line, self.column) {
            write!(formatter, ":{line}:{column}")?;
        }
        Ok(())
    }
}

/// Errors from loading, resolving, and verifying one private workspace snapshot.
#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    /// A bounded authority-bearing workspace file could not be admitted.
    #[error(transparent)]
    Input(#[from] pale_ports::SourceFilesystemError),
    /// A bounded workspace source file is not valid UTF-8.
    #[error("workspace input is not UTF-8: `{path}`", path = .0.display())]
    InputNotUtf8(PathBuf),
    /// A selected Package page lacks a bounded header or an authored body.
    #[error("invalid Package page `{path}`: {reason}", path = .path.display())]
    PackagePageHeader {
        /// Exact captured source label.
        path: PathBuf,
        /// Shared page-reader failure.
        #[source]
        reason: crate::packages::PageHeaderError,
    },
    /// One workspace file is not syntactically valid TOML.
    #[error("cannot parse workspace TOML `{coordinate}`: {source}")]
    TomlSyntax {
        /// File and parser-owned source position.
        coordinate: WorkspaceSourceCoordinate,
        /// Typed parser failure after its retained source text has been removed.
        #[source]
        source: Box<toml::de::Error>,
    },
    /// A parsed TOML value could not be represented for JSON Schema admission.
    #[error("cannot project workspace TOML `{coordinate}` to JSON: {source}")]
    TomlToJson {
        /// Exact source file; no post-parse position is invented.
        coordinate: WorkspaceSourceCoordinate,
        /// Typed serialization failure.
        #[source]
        source: serde_json::Error,
    },
    /// A supplied record passed structural schema checks but failed typed validation.
    ///
    /// This is source rejection before semantic linking. Typed validation can
    /// check record invariants, including a claimed digest, beyond schema shape.
    #[error("cannot decode workspace {record} `{coordinate}` at `{value_path}`: {source}")]
    Decode {
        /// Exact source file; no TOML position is invented from the JSON value.
        coordinate: Box<WorkspaceSourceCoordinate>,
        /// Typed workspace record being decoded.
        record: &'static str,
        /// Nested Serde value path at which typed decoding stopped.
        value_path: String,
        /// Typed decoding failure with its structured Serde path intact.
        #[source]
        source: Box<serde_path_to_error::Error<serde_json::Error>>,
    },
    /// A valid record ID cannot be represented after adding its source-family prefix.
    ///
    /// The original ID passed record admission. This failure concerns the
    /// combined derivation source ID, not the original record's grammar.
    #[error(
        "cannot represent the {family} derivation source identity from `{coordinate}` after adding its prefix ({combined_byte_len} bytes)"
    )]
    UnrepresentableDerivationSourceId {
        /// Exact source file; no position is invented after typed decoding.
        coordinate: Box<WorkspaceSourceCoordinate>,
        /// Static field segments identifying the original record ID.
        field_path: &'static [&'static str],
        /// Source family whose prefix participates in the combined identity.
        family: &'static str,
        /// UTF-8 byte length of the combined identity rejected by its parser.
        combined_byte_len: usize,
        /// The existing derivation-source identity parser's rejection.
        #[source]
        source: pale_core::records::InvalidWorkspaceLanguageId,
    },
    /// The root workspace-manifest path has no parent directory from which to resolve references.
    #[error("workspace manifest has no parent directory: `{0}`")]
    MissingParent(PathBuf),
    /// The canonical desired workspace topology is structurally invalid.
    #[error("workspace desired topology is invalid: {0}")]
    InvalidDesiredTopology(#[source] pale_core::records::InvalidWorkspaceDesiredTopologyV1),
    /// The requested logical project is absent from the desired topology.
    #[error("workspace desired topology has no project `{0}`")]
    UnknownProject(String),
    /// The requested logical execution workspace is absent from the desired topology.
    #[error("workspace desired topology has no execution workspace `{0}`")]
    UnknownExecutionWorkspace(String),
    /// The selected project and execution workspace have no explicit membership edge.
    #[error("project `{project}` is not declared in execution workspace `{execution_workspace}`")]
    ProjectExecutionWorkspaceMembership {
        /// Requested project.
        project: String,
        /// Requested execution workspace.
        execution_workspace: String,
    },
    /// A selected package has no corresponding version pin in the distribution lock.
    #[error("selected package `{0}` has no version in the distribution lock")]
    MissingPackageVersion(String),
    /// The distribution lock repeats one package identity.
    #[error(
        "distribution lock repeats package `{id}` at packages[{first_index}] and packages[{second_index}]"
    )]
    DuplicateLockedPackage {
        /// Duplicate package identity.
        id: String,
        /// First index containing the identity.
        first_index: usize,
        /// Second index containing the identity.
        second_index: usize,
    },
    /// The project run-policy selector has no private policy declaration.
    #[error("selected run policy `{0}` has no project capability declaration")]
    MissingRunPolicy(String),
    /// A private capabilities file declares one run-policy identity more than once.
    #[error("private capabilities repeat run policy `{0}`")]
    DuplicateRunPolicy(String),
    /// A private run-policy declaration is internally contradictory.
    #[error("invalid private run policy `{id}`: {source}")]
    InvalidRunPolicy {
        /// Policy identity whose mechanics are invalid.
        id: String,
        /// Exact typed cross-field validation failure.
        #[source]
        source: pale_core::records::InvalidRunPolicy,
    },
    /// A private lifecycle declaration contradicts its typed grammar.
    #[error("invalid private lifecycle behavior `{id}`: {source}")]
    InvalidLifecycleBehavior {
        /// Private behavior identity.
        id: String,
        /// Exact lifecycle grammar failure.
        #[source]
        source: pale_core::records::InvalidLifecycleBehavior,
    },
    /// A private capabilities file repeats one lifecycle identity.
    #[error("private capabilities repeat lifecycle behavior `{0}`")]
    DuplicateLifecycleBehavior(String),
    /// The supplied portable profile fails schema or canonical self-digest verification.
    #[error("invalid effective-profile identity: {source}")]
    InvalidProfileIdentity {
        /// Exact profile identity verification failure.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// Canonical encoding or digest computation failed.
    #[error(transparent)]
    Canon(#[from] canon::CanonError),
    /// A machine binding does not resolve a repository selected by the portable profile.
    #[error("machine `{machine}` does not resolve repository `{repository}`")]
    UnresolvedRepository {
        /// Repository identity selected by the profile.
        repository: LogicalRepositoryIdV1,
        /// Machine binding in which resolution failed.
        machine: String,
    },
    /// A machine binding does not resolve an authority root selected by the portable profile.
    #[error("machine `{machine}` does not resolve authority root `{authority_root}`")]
    UnresolvedAuthorityRoot {
        /// Authority-root identity selected by the profile or session model.
        authority_root: String,
        /// Machine binding in which resolution failed.
        machine: String,
    },
    /// A repository selects a prompt family absent from the selected distribution.
    #[error("repository `{repository}` references unknown family `{family}`")]
    UnknownFamily {
        /// Unknown family identity.
        family: String,
        /// Repository selecting the family.
        repository: LogicalRepositoryIdV1,
    },
    /// Machine checkout projections repeat one selected repository identity.
    #[error("duplicate repository id `{0}`")]
    DuplicateRepository(LogicalRepositoryIdV1),
    /// Machine provider projections assign one provider twice to a selected repository.
    #[error("machine repeats provider `{provider}` for repository `{repository}`")]
    DuplicateProviderRepositoryProjection {
        /// Selected logical repository identity.
        repository: LogicalRepositoryIdV1,
        /// Repeated provider-binding identity.
        provider: String,
    },
    /// A command profile refers to no declared repository-role vocabulary entry.
    #[error("repository command profile references undeclared repository role `{0}`")]
    UnknownRepositoryCommandRole(String),
    /// One repository-role command appears in both admitted and mutating sets.
    #[error(
        "repository role `{role}` classifies command as both admitted and mutating: {command}"
    )]
    ConflictingRepositoryCommand {
        /// Repository-role vocabulary identity.
        role: String,
        /// Conflicting command token.
        command: String,
    },
    /// A machine-bound path is not absolute.
    #[error("machine path for `{binding}` is not absolute: {path}")]
    NonAbsolutePath {
        /// Logical binding whose path is invalid.
        binding: String,
        /// Non-absolute path value.
        path: String,
    },
    /// Two logical machine bindings resolve to the same filesystem path.
    #[error("machine binds two logical ids to the same path: {0}")]
    NonDistinctPath(String),
    /// A workspace reference is absolute, traversing, or otherwise escapes its owner.
    #[error("referenced file `{0}` escapes its workspace owner")]
    EscapedReference(String),
    /// An already-captured portable bundle repeats one logical source label.
    #[error("captured workspace source repeats logical document `{0}`")]
    DuplicateCapturedSource(String),
    /// The selected root manifest is absent from an already-captured portable bundle.
    #[error("captured workspace source omits root manifest `{0}`")]
    MissingCapturedSourceRoot(String),
    /// The selected captured root is outside the confined logical-path grammar.
    #[error("captured workspace root is outside the confined logical-path grammar")]
    InvalidCapturedSourceRoot,
    /// Host capture exceeds an existing private compiler-input ceiling.
    #[error(transparent)]
    CapturedPrivateInput(#[from] crate::captured_input::CapturedCompilationInputError),
    /// One captured workspace document exceeds its bounded ingress ceiling.
    #[error("captured workspace document contains {observed} bytes; limit is {limit}")]
    CapturedDocumentTooLarge {
        /// Supplied byte count rejected before the document is parsed.
        observed: usize,
        /// Maximum bytes accepted for one workspace source document.
        limit: usize,
    },
    /// A captured workspace bundle exceeds its bounded member ceiling.
    #[error("captured workspace source contains {observed} documents; limit is {limit}")]
    CapturedDocumentCountExceeded {
        /// Supplied document count rejected before members are converted or indexed.
        observed: usize,
        /// Maximum members accepted by one captured source bundle.
        limit: usize,
    },
    /// Repeated capture observed two different semantic workspace snapshots.
    #[error("workspace input changed while its snapshot was being captured: {0}")]
    InputChanged(String),
    /// Embedded schema loading or validation failed for one exact workspace file.
    #[error("workspace record `{coordinate}` failed schema admission: {source}")]
    SchemaCheck {
        /// Exact source file; no TOML position is invented from the JSON value.
        coordinate: WorkspaceSourceCoordinate,
        /// Typed embedded-schema failure.
        #[source]
        source: Box<pale_contracts::SchemaError>,
    },
    /// An application-generated record or its embedded schema identity failed validation.
    ///
    /// Unlike [`Self::SchemaCheck`], this failure has no workspace source file:
    /// the application constructed the record after source admission.
    #[error(transparent)]
    GeneratedRecordSchema(#[from] pale_contracts::SchemaError),
    /// A source component failed its typed shape, ordering, or digest contract.
    #[error("workspace source component is invalid: {0}")]
    InvalidWorkspaceComponent(#[source] pale_core::records::InvalidWorkspaceComponent),
    /// A module failed its typed shape, ordering, or digest contract.
    #[error("workspace module is invalid: {0}")]
    InvalidWorkspaceModule(#[source] pale_core::records::InvalidWorkspaceModule),
    /// A resolution or universe lock failed its typed shape, ordering, or digest contract.
    #[error("workspace lock is invalid: {0}")]
    InvalidWorkspaceLock(#[source] pale_core::records::InvalidWorkspaceLock),
    /// Selected Package or Normative-Pack occurrences conflict or exceed bounds.
    #[error("workspace release closure is invalid: {0}")]
    ReleaseClosure(#[from] pale_core::records::ReleaseClosureErrorV1),
    /// The root workspace manifest points to a required path that is absent.
    #[error("workspace manifest references `{reference}` which is missing at {path}")]
    MissingReference {
        /// Logical reference that could not be loaded.
        reference: String,
        /// Resolved filesystem path expected to contain it.
        path: String,
    },
    /// The selected machine omits the distribution root required by the canonical entry.
    #[error("machine `{0}` does not declare distribution_root (required by the canonical entry)")]
    MissingDistributionRoot(String),
    /// A selected record's declared identity differs from the canonical-entry selector.
    #[error("declared {kind} id `{declared}` does not match the requested `{requested}`")]
    IdentityMismatch {
        /// Kind of selected identity.
        kind: String,
        /// Identity declared inside the selected record.
        declared: String,
        /// Identity requested through the canonical entry.
        requested: String,
    },
    /// A session activation root names no selected project repository.
    #[error("activation_roots references unknown repository `{repository}`")]
    UnknownActivationRoot {
        /// Unknown activation-root repository identity.
        repository: LogicalRepositoryIdV1,
    },
    /// A session declares no repository at which project-local activation may occur.
    #[error("activation_roots must contain at least one repository")]
    EmptyActivationRoots,
    /// A session activation root occurs more than once.
    #[error("activation_roots contains duplicate repository `{repository}`")]
    DuplicateActivationRoot {
        /// Duplicated activation-root repository identity.
        repository: LogicalRepositoryIdV1,
    },
    /// A repository is ambiguously declared as both an entry and a non-entry grant.
    #[error("repository `{repository}` occurs in both activation_roots and additional_roots")]
    ActivationAdditionalRootOverlap {
        /// Ambiguously classified repository identity.
        repository: LogicalRepositoryIdV1,
    },
    /// An additional session root names no selected project repository.
    #[error("additional_roots references unknown repository `{repository}`")]
    UnknownAdditionalRoot {
        /// Unknown additional-root repository identity.
        repository: LogicalRepositoryIdV1,
    },
    /// A non-entry repository grant occurs more than once.
    #[error("additional_roots contains duplicate repository `{repository}`")]
    DuplicateAdditionalRoot {
        /// Duplicated additional-root repository identity.
        repository: LogicalRepositoryIdV1,
    },
    /// Session authority access names no declared authority root.
    #[error("session authority_access references unknown authority root `{0}`")]
    SessionUnknownAuthority(String),
    /// Direct activation was selected without a bound Pale executable.
    #[error("session activates a host but machine `{0}` declares no [surface_executable]")]
    ActivationRequiresSurfaceExecutable(String),
    /// Hook-bearing activation was selected without an external session-state root.
    #[error(
        "session activates a host but machine `{0}` declares no [session_state] (ephemeral hook state must live outside project repositories)"
    )]
    ActivationRequiresSessionState(String),
    /// Hook-bearing activation was selected without recoverable evidence storage.
    #[error(
        "session activates a host but machine `{0}` declares no [durable_evidence] (recoverable decisions and execution leases must live outside project repositories)"
    )]
    ActivationRequiresDurableEvidence(String),
    /// A selected execution path lacks its exact provider-bound executable.
    #[error(
        "session execution path `{execution_path}` requires a machine binding for native harness `{harness}`"
    )]
    ActivationRequiresNativeHarness {
        /// Selected neutral execution-path identity.
        execution_path: String,
        /// Missing provider-binding identifier.
        harness: String,
    },
    /// A selected execution path cannot be projected to an exact provider binding.
    #[error("session execution path `{execution_path}` has an invalid provider witness: {source}")]
    InvalidExecutionPathProviderWitness {
        /// Selected neutral execution-path identity.
        execution_path: String,
        /// Exact witness-construction failure.
        #[source]
        source: pale_core::records::InvalidExecutionPathWitnessV1,
    },
    /// Machine provider bindings are not in strict canonical order.
    #[error("machine native_harnesses must be sorted and unique by exact provider binding")]
    UnsortedOrDuplicateNativeHarnessBindings,
    /// The bounded native-harness version probe did not complete successfully.
    #[error("native harness `{harness}` identity probe failed: {source}")]
    NativeHarnessProbe {
        /// Native harness being verified.
        harness: String,
        /// Typed process or executable-identity failure.
        #[source]
        source: Box<pale_ports::ProcessProbeError>,
    },
    /// Native-harness version output was not valid UTF-8.
    #[error("native harness `{harness}` emitted non-UTF-8 version output: {source}")]
    NativeHarnessUtf8 {
        /// Native harness being verified.
        harness: String,
        /// Exact UTF-8 decoding failure.
        #[source]
        source: std::str::Utf8Error,
    },
    /// Native-harness reported version differs from its exact machine binding.
    #[error(
        "native harness `{harness}` reported version `{actual}`, expected exact bound version `{expected}`"
    )]
    NativeHarnessVersionMismatch {
        /// Native harness being verified.
        harness: String,
        /// Version reported by the executable.
        actual: String,
        /// Version pinned by the machine binding.
        expected: String,
    },
    /// A reading-order path is not a confined relative path.
    #[error(
        "session reading_order rel_path `{0}` is not a confined relative path (no traversal/absolute/control chars)"
    )]
    ReadingOrderNotConfined(String),
    /// Machine configuration omits the Pale executable required for activation.
    #[error("surface executable is missing")]
    SurfaceExecutableMissing,
    /// Installed executable bytes or filesystem identity did not match the binding.
    #[error("surface executable `{exec}` identity check failed: {source}")]
    SurfaceExecutableIdentity {
        /// Bound Pale executable path.
        exec: String,
        /// Typed executable-identity failure.
        #[source]
        source: Box<pale_ports::ExecutableIdentityError>,
    },
    /// The bounded interface probe did not complete successfully.
    #[error("surface executable `{exec}` interface probe failed: {source}")]
    SurfaceExecutableProbe {
        /// Bound Pale executable path.
        exec: String,
        /// Typed process failure.
        #[source]
        source: Box<pale_ports::ProcessProbeError>,
    },
    /// Executable identity output was not one strict JSON value.
    #[error("surface executable `{exec}` emitted invalid strict JSON: {source}")]
    SurfaceExecutableIdentityJson {
        /// Bound Pale executable path.
        exec: String,
        /// Typed strict-JSON parsing failure.
        #[source]
        source: Box<canon::CanonError>,
    },
    /// Executable identity output failed its embedded schema contract.
    #[error("surface executable `{exec}` identity failed schema admission: {source}")]
    SurfaceExecutableIdentitySchema {
        /// Bound Pale executable path.
        exec: String,
        /// Typed embedded-schema failure.
        #[source]
        source: Box<pale_contracts::SchemaError>,
    },
    /// Schema-valid executable identity could not be decoded into its typed record.
    #[error("cannot decode surface executable `{exec}` identity at `{value_path}`: {source}")]
    SurfaceExecutableIdentityDecode {
        /// Bound Pale executable path.
        exec: String,
        /// Nested JSON value path at which typed decoding stopped.
        value_path: String,
        /// Typed decoding failure with its structured path intact.
        #[source]
        source: Box<serde_path_to_error::Error<serde_json::Error>>,
    },
    /// Executable-reported platform facts differ from this Pale build.
    #[error(
        "surface executable `{exec}` reports platform {running_os}-{running_arch} with {running_publication:?} publication, expected {expected_os}-{expected_arch} with {expected_publication:?} publication"
    )]
    SurfaceExecutablePlatformMismatch {
        /// Bound Pale executable path.
        exec: String,
        /// Operating-system token reported by the executable.
        running_os: String,
        /// Architecture token reported by the executable.
        running_arch: String,
        /// Durable-publication mechanism reported by the executable.
        running_publication: pale_core::platform::DurablePublication,
        /// Operating-system token compiled into this executable.
        expected_os: &'static str,
        /// Architecture token compiled into this executable.
        expected_arch: &'static str,
        /// Durable-publication mechanism compiled into this executable.
        expected_publication: pale_core::platform::DurablePublication,
    },
    /// Session-state root is not absolute and lexically normalized.
    #[error("session-state root `{0}` is not an absolute normalized path")]
    SessionStateNotAbsolute(String),
    /// Session-state root is absent on this machine.
    #[error("session-state root `{0}` does not exist on this machine")]
    SessionStateNotFound(String),
    /// Session-state root is not a directory.
    #[error("session-state root `{0}` is not a directory")]
    SessionStateNotDirectory(String),
    /// Session-state root contains a symlink or canonicalizes elsewhere.
    #[error("session-state root `{0}` contains a symlink or does not resolve to itself")]
    SessionStateNotConfined(String),
    /// Session-state root permits group or world writes.
    #[error("session-state root `{0}` is group- or world-writable")]
    SessionStateUnsafePermissions(String),
    /// Durable-evidence root is not absolute and lexically normalized.
    #[error("durable-evidence root `{0}` is not an absolute normalized path")]
    DurableEvidenceNotAbsolute(String),
    /// Durable-evidence root is absent on this machine.
    #[error("durable-evidence root `{0}` does not exist on this machine")]
    DurableEvidenceNotFound(String),
    /// Durable-evidence root is not a directory.
    #[error("durable-evidence root `{0}` is not a directory")]
    DurableEvidenceNotDirectory(String),
    /// Durable-evidence root contains a symlink or canonicalizes elsewhere.
    #[error("durable-evidence root `{0}` contains a symlink or does not resolve to itself")]
    DurableEvidenceNotConfined(String),
    /// Durable-evidence root permits group or world writes.
    #[error("durable-evidence root `{0}` is group- or world-writable")]
    DurableEvidenceUnsafePermissions(String),
    /// Machine configuration omits the materialization-lineage root.
    #[error("machine `{0}` declares no [materialization_lineage] root")]
    MaterializationLineageMissing(String),
    /// Materialization-lineage root is not absolute and lexically normalized.
    #[error("materialization-lineage root `{0}` is not an absolute normalized path")]
    MaterializationLineageNotAbsolute(String),
    /// Materialization-lineage root is absent on this machine.
    #[error("materialization-lineage root `{0}` does not exist on this machine")]
    MaterializationLineageNotFound(String),
    /// Materialization-lineage root is not a directory.
    #[error("materialization-lineage root `{0}` is not a directory")]
    MaterializationLineageNotDirectory(String),
    /// Materialization-lineage root contains a symlink or canonicalizes elsewhere.
    #[error("materialization-lineage root `{0}` contains a symlink or does not resolve to itself")]
    MaterializationLineageNotConfined(String),
    /// Materialization-lineage root permits group or world writes.
    #[error("materialization-lineage root `{0}` is group- or world-writable")]
    MaterializationLineageUnsafePermissions(String),
    /// Running executable path differs from the machine-bound path.
    #[error("running executable `{running}` is not the bound surface executable `{bound}`")]
    SurfaceExecutablePathMismatch {
        /// Canonical path of the running executable.
        running: String,
        /// Canonical path pinned by the machine binding.
        bound: String,
    },
    /// Running executable version differs from the machine binding.
    #[error(
        "running executable version `{running}` does not match bound surface executable version `{bound}`"
    )]
    SurfaceExecutableVersionMismatch {
        /// Version reported by the running executable.
        running: String,
        /// Version pinned by the machine binding.
        bound: String,
    },
    /// Running executable protocol differs from the machine binding.
    #[error(
        "running executable protocol `{running}` does not match bound surface executable protocol `{bound}`"
    )]
    SurfaceExecutableProtocolMismatch {
        /// Protocol reported by the running executable.
        running: String,
        /// Protocol pinned by the machine binding.
        bound: String,
    },
    /// A reading-order file is absent or not regular beneath its authority root.
    #[error(
        "reading_order file `{0}` does not exist (or is not a regular file) beneath its authority root"
    )]
    ReadingOrderMissing(String),
    /// A reading-order file canonicalizes outside its declared authority root.
    #[error("reading_order file `{path}` escapes its authority root `{root}`")]
    ReadingOrderEscapesRoot {
        /// Declared repository-relative reading-order path.
        path: String,
        /// Authority root from which it must resolve.
        root: String,
    },
    /// A session-bound authority or workspace path is absent on this machine.
    #[error("session path `{path}` for `{binding}` does not exist on this machine")]
    SessionPathMissing {
        /// Resolved path expected to exist.
        path: String,
        /// Logical binding requiring the path.
        binding: String,
    },
    /// A machine-local session path exists or may exist, but could not be observed.
    #[error("cannot observe session path `{path}` for `{binding}`: {source}")]
    SessionPathObservation {
        /// Declared UTF-8 path whose machine observation failed.
        path: String,
        /// Logical binding requiring the observation.
        binding: String,
        /// Typed mechanical failure retained from the filesystem port.
        #[source]
        source: Box<pale_ports::MachineFilesystemError>,
    },
    /// A canonical machine-local path cannot enter the portable UTF-8 projection.
    #[error("canonical session path for `{path}` bound as `{binding}` is not UTF-8")]
    SessionPathNotUtf8 {
        /// Declared UTF-8 path whose canonical target is not representable.
        path: String,
        /// Logical binding requiring the canonical path.
        binding: String,
    },
    /// A workspace-language identity is duplicated in one bounded source set.
    #[error("workspace language repeats {kind} `{id}`")]
    DuplicateLanguageIdentity {
        /// Closed logical identity class.
        kind: &'static str,
        /// Repeated logical identity.
        id: String,
    },
    /// A logical-set vector is not in its one canonical order.
    #[error("workspace {kind} `{id}` is not canonically ordered")]
    NonCanonicalLanguageSet {
        /// Closed logical record kind.
        kind: &'static str,
        /// Stable record identity.
        id: String,
    },
    /// A retained semantic-content or lock digest differs from its typed preimage.
    #[error("workspace {kind} digest mismatch for `{id}`")]
    WorkspaceDigestMismatch {
        /// Closed identity domain.
        kind: &'static str,
        /// Subject whose digest failed.
        id: String,
    },
    /// A sealed application-owned semantic IR no longer matches its canonical preimage.
    #[error("workspace semantic IR digest changed after sealing for `{0}`")]
    WorkspaceSemanticIrDigestMismatch(String),
    /// An admitted universe observation cannot fit its portable projection width.
    #[error("portable workspace {kind} count {observed} exceeds the u32 projection")]
    PortableObservationOverflow {
        /// Closed projected observation class.
        kind: &'static str,
        /// Admitted count that cannot be represented.
        observed: usize,
    },
    /// A content-bearing export projection contradicts locally available owner source.
    #[error("workspace export projection differs from owner source for `{0}`")]
    ExportProjectionMismatch(String),
    /// Source delivery was selected but an exact referenced component was not loaded.
    #[error("source delivery for module `{module}` requires absent component `{component}`")]
    SourceDeliveryUnavailable {
        /// Module whose selected surface requires source delivery.
        module: String,
        /// Missing content-addressed source component.
        component: String,
    },
    /// The resolution lock and module/import closure disagree.
    #[error("workspace release resolution is incomplete or inconsistent: {0}")]
    ResolutionLockMismatch(String),
    /// The universe lock and local adoption frontier disagree.
    #[error("workspace universe lock is incomplete or inconsistent: {0}")]
    UniverseLockMismatch(String),
    /// One adopted workspace graph aggregate exceeds its workspace-language bound.
    #[error("workspace graph contains {observed} {kind}; limit is {limit}")]
    WorkspaceGraphLimit {
        /// Closed aggregate whose count exceeded the bound.
        kind: &'static str,
        /// Number of members in the selected adoption closure.
        observed: usize,
        /// Workspace-language limit enforced before the aggregate is consumed.
        limit: usize,
    },
    /// An import, extension, or responsibility module surface cannot resolve
    /// its exact selected module, release, export, or namespace closure.
    #[error("workspace namespace resolution failed: {0}")]
    NamespaceResolution(String),
    /// A visibility or prevent rule denies this import hop.
    #[error("workspace declaration delivery is denied: {0}")]
    VisibilityDenied(String),
    /// A declaration carrying `prevent` was selected for another delivery hop.
    #[error("workspace declaration cannot propagate onward: {0}")]
    OnwardPropagationDenied(String),
    /// A local extension weakens an upstream policy ceiling.
    #[error("workspace policy narrowing widens `{0}`")]
    PolicyWidening(String),
    /// One import visibility ceiling is broader than its selected export.
    #[error(
        "workspace import `{importer_module}` widens visibility for `{upstream_module}:{export_id}` from `{export_visibility}` to `{import_ceiling}`"
    )]
    ImportVisibilityWidening {
        /// Compiler logical label of the importing module source.
        source_label: String,
        /// Importing module whose edge must be narrowed.
        importer_module: WorkspaceModuleId,
        /// Exact import member in the authored module.
        import_index: usize,
        /// Upstream module owning the selected export.
        upstream_module: WorkspaceModuleId,
        /// Selected export whose visibility is authoritative.
        export_id: WorkspaceExportId,
        /// Owner-declared export visibility.
        export_visibility: WorkspaceVisibilityV1,
        /// Authored importer ceiling that attempted to widen the export.
        import_ceiling: WorkspaceVisibilityV1,
    },
    /// One named import-edge dimension widens the selected export policy.
    #[error(
        "workspace import policy widens {axis} on `{importer_module}` selecting `{upstream_module}` release `{release_id}` export `{export_id}` namespace `{source_namespace}` as `{local_namespace}`"
    )]
    ImportPolicyWidening {
        /// Independently ordered dimension that widened.
        axis: pale_core::records::WorkspacePolicyAxisV1,
        /// Module declaring the narrowing edge.
        importer_module: pale_core::records::WorkspaceModuleId,
        /// Module owning the selected release.
        upstream_module: pale_core::records::WorkspaceModuleId,
        /// Exact selected release.
        release_id: pale_core::records::WorkspaceReleaseId,
        /// Selected export.
        export_id: pale_core::records::WorkspaceExportId,
        /// Selected upstream namespace.
        source_namespace: Box<pale_core::records::WorkspaceNamespaceId>,
        /// Importer-local namespace.
        local_namespace: Box<pale_core::records::WorkspaceNamespaceId>,
    },
    /// Effective obligations or authority claims cannot form one bounded policy.
    #[error(transparent)]
    PolicyCombination(#[from] WorkspacePolicyCombinationError),
    /// A declaration cycle is not finite, positive, and monotone.
    #[error("workspace declaration cycle is inadmissible: {0}")]
    InadmissibleDeclarationCycle(String),
    /// A portable workspace source contains machine-local or secret material.
    #[error("portable workspace input contains forbidden material at `{0}`")]
    PortableInputLeakage(String),
    /// An attributable responsibility source cannot be lowered canonically.
    #[error(transparent)]
    ResponsibilityDerivation(Box<crate::composition::ResponsibilityDerivationError>),
    /// The explicit organizational source cannot be lowered canonically.
    #[error(transparent)]
    OrganizationalTranslation(Box<crate::composition::OrganizationalTranslationError>),
    /// One directional responsibility names no selected organizational service.
    #[error(
        "responsibility `{derivation}` references unavailable directional service `{contract}`"
    )]
    MissingDirectionalService {
        /// Affected responsibility derivation only.
        derivation: pale_core::records::ResponsibilityDerivationId,
        /// Exact namespace-qualified service coordinate.
        contract: Box<pale_core::records::DirectionalServiceContractRefV1>,
    },
    /// No selected publisher-verification owner can establish admission.
    #[error("publisher admission is unavailable for `{publisher}` release `{release}`")]
    PublisherAdmissionUnavailable {
        /// Publisher whose proof is missing.
        publisher: String,
        /// Exact release requiring verification.
        release: String,
    },
    /// A selected release lifecycle state is not admissible for a new universe.
    #[error("release `{release}` for module `{module}` is not published at this resolution")]
    ReleaseStateNotAdmissible {
        /// Module whose selected release cannot be newly admitted.
        module: String,
        /// Exact release coordinate.
        release: String,
    },
    /// No exact protected-delivery proof covers one selected import path.
    #[error("protected-delivery admission is unavailable for `{importer}` path `{path}`")]
    ProtectedDeliveryAdmissionUnavailable {
        /// Importing module.
        importer: String,
        /// Exact selected path coordinate.
        path: String,
    },
    /// An application-built derivation receipt failed its own contract.
    #[error("workspace derivation receipt is invalid: {0}")]
    InvalidDerivationReceipt(#[source] Box<pale_core::records::InvalidWorkspaceDerivationReceipt>),
    /// A self-consistent receipt does not describe its two verified IR frontiers.
    #[error("workspace derivation receipt does not match the compared semantic frontiers")]
    DerivationSemanticMismatch,
}

impl WorkspaceError {
    /// Return an exact editor coordinate only when the owning failure retains one.
    ///
    /// Structured Serde segments distinguish numeric map keys from sequence
    /// indices. JSON Schema segments were already resolved against the rejected
    /// instance by `pale-contracts`. Unknown decoder segments yield `None`
    /// rather than a guessed range.
    #[must_use]
    pub fn authoring_location(&self) -> Option<WorkspaceAuthoringLocation> {
        if let Some(correction) = self.authoring_correction() {
            return Some(WorkspaceAuthoringLocation::new(
                correction.source_label().to_owned(),
                correction.path().to_vec(),
            ));
        }
        let (coordinate, path) = match self {
            Self::UnrepresentableDerivationSourceId {
                coordinate,
                field_path,
                ..
            } => (
                coordinate.as_ref(),
                field_path
                    .iter()
                    .map(|field| super::AuthoringInstancePathSegment::Field((*field).to_owned()))
                    .collect(),
            ),
            Self::Decode {
                coordinate, source, ..
            } => (
                coordinate.as_ref(),
                source
                    .path()
                    .iter()
                    .map(|segment| match segment {
                        serde_path_to_error::Segment::Seq { index } => {
                            Some(super::AuthoringInstancePathSegment::Index(*index))
                        }
                        serde_path_to_error::Segment::Map { key } => {
                            Some(super::AuthoringInstancePathSegment::Field(key.clone()))
                        }
                        serde_path_to_error::Segment::Enum { .. }
                        | serde_path_to_error::Segment::Unknown => None,
                    })
                    .collect::<Option<Vec<_>>>()?,
            ),
            Self::SchemaCheck { coordinate, source } => {
                let pale_contracts::SchemaError::Invalid(violations) = &**source else {
                    return None;
                };
                let path = violations
                    .violations()
                    .first()?
                    .instance_path_segments()?
                    .iter()
                    .map(|segment| match segment {
                        pale_contracts::SchemaInstancePathSegment::Property(property) => {
                            super::AuthoringInstancePathSegment::Field(property.clone())
                        }
                        pale_contracts::SchemaInstancePathSegment::Index(index) => {
                            super::AuthoringInstancePathSegment::Index(*index)
                        }
                    })
                    .collect();
                (coordinate, path)
            }
            _ => return None,
        };
        Some(WorkspaceAuthoringLocation::new(
            coordinate.path().to_string_lossy().into_owned(),
            path,
        ))
    }
}

impl From<crate::composition::ResponsibilityDerivationError> for WorkspaceError {
    fn from(source: crate::composition::ResponsibilityDerivationError) -> Self {
        Self::ResponsibilityDerivation(Box::new(source))
    }
}

impl From<crate::composition::OrganizationalTranslationError> for WorkspaceError {
    fn from(source: crate::composition::OrganizationalTranslationError) -> Self {
        Self::OrganizationalTranslation(Box::new(source))
    }
}

#[cfg(test)]
#[path = "../../tests/internal/workspace/errors.rs"]
mod tests;
