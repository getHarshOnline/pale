//! Semantic admission for the portable diagnostic wire contract.

use pale_application::diagnostic::{AdmittedDiagnostic, DiagnosticAdmissionError};
use pale_core::records::{
    DiagnosticDispositionV1, DiagnosticEffectCertaintyV1, DiagnosticFactV1Wire, DiagnosticRetryV1,
    DiagnosticSchemaCoordinateV1Wire, DiagnosticSeverityV1, DiagnosticSourceV1Wire,
    DiagnosticStageV1, DiagnosticV1Wire, RecordSchemaIdentity, DIAGNOSTIC_V1_SCHEMA_ID,
};

use crate::contracts::schema_violation_projection;
use crate::error::{ApiError, ApiInitializationError, BoundaryError, FailureSpecification};
use crate::PaleApi;

/// Maximum diagnostic document admitted before strict parsing allocates its value tree.
const DIAGNOSTIC_INPUT_LIMIT: usize = 16 << 20;

/// Narrow owner for canonical diagnostic projection.
///
/// This handle verifies only the embedded diagnostic contract. It is useful to
/// protocol leaves that need Pale's shared diagnostic policy but do not invoke
/// compilation or native actions. [`PaleApi`] exposes the same projection
/// behavior through its broader façade.
pub struct DiagnosticProjector {
    diagnostic_identity: RecordSchemaIdentity,
}

impl DiagnosticProjector {
    /// Verify the embedded diagnostic contract and construct its projector.
    ///
    /// # Errors
    ///
    /// Returns [`ApiInitializationError`] when the embedded diagnostic contract
    /// lacks a coherent self-carried identity.
    pub fn new() -> Result<Self, ApiInitializationError> {
        let diagnostic_identity =
            pale_contracts::record_schema_identity(pale_core::records::DIAGNOSTIC_V1_SCHEMA_ID)
                .map_err(ApiInitializationError::contracts)?;
        Ok(Self {
            diagnostic_identity,
        })
    }

    /// Project one owner classification through Pale's diagnostic policy.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when the projection violates the generated contract
    /// or its semantic invariants. No partial diagnostic is returned.
    pub fn project(&self, projection: DiagnosticProjection) -> Result<DiagnosticV1, ApiError> {
        project_diagnostic(&self.diagnostic_identity, projection)
    }
}

/// Owner classification plus structured coordinates for one admission failure.
struct DiagnosticFailure<'a> {
    /// Stable machine-readable failure code.
    code: &'a str,
    /// Admission boundary that established the failure.
    stage: DiagnosticStageV1,
    /// Recovery-relevant failure class.
    disposition: DiagnosticDispositionV1,
    /// Concise explanation independent of the typed source text.
    summary: &'a str,
    /// Optional rejected input coordinate.
    location: Option<DiagnosticSourceV1Wire>,
    /// Optional governing schema-keyword pointer.
    schema_path: Option<String>,
    /// Bounded presentation details that require no prose parsing.
    facts: Vec<DiagnosticFactV1Wire>,
}

/// Preserve the selected embedded-schema consequence across diagnostic routes.
fn diagnostic_contract_failure<'a>(
    error: &pale_contracts::SchemaError,
    invalid_code: &'a str,
    invalid_summary: &'a str,
) -> DiagnosticFailure<'a> {
    match pale_application::embedded_schema_failure_disposition(error) {
        pale_application::ApplicationFailureDisposition::InvalidInput => {
            let projection = schema_violation_projection(error);
            DiagnosticFailure {
                code: invalid_code,
                stage: DiagnosticStageV1::ContractValidation,
                disposition: DiagnosticDispositionV1::InvalidInput,
                summary: invalid_summary,
                location: projection.location,
                schema_path: projection.schema_path,
                facts: projection.facts,
            }
        }
        pale_application::ApplicationFailureDisposition::Internal => DiagnosticFailure {
            code: "diagnostic_contract_invariant",
            stage: DiagnosticStageV1::Internal,
            disposition: DiagnosticDispositionV1::InternalFailure,
            summary: "embedded diagnostic contract registry is incoherent",
            location: None,
            schema_path: None,
            facts: Vec::new(),
        },
        pale_application::ApplicationFailureDisposition::Operational
        | pale_application::ApplicationFailureDisposition::Blocked => {
            unreachable!("embedded schema admission has only authored or internal failure")
        }
    }
}

/// Unadmitted owner projection for one initial-v1 diagnostic.
///
/// This type is intentionally not a wire record. A consumer supplies the
/// classification it owns and optional presentation candidates, then passes the
/// value to [`DiagnosticProjector::project`] or [`PaleApi::project_diagnostic`].
/// Both routes use the same application-owned admission path. That path redacts
/// and bounds text, resolves schema identity, accounts for omissions, validates
/// the generated contract, and admits cross-field semantics before returning
/// [`DiagnosticV1`].
///
/// The projection carries no retry authority beyond its explicit
/// [`DiagnosticRetry`] and [`DiagnosticEffectCertainty`] values. In particular,
/// a fact, subject, or source label is presentation context, not evidence that an
/// effect occurred or is safe to repeat.
///
/// The rendered implication uses `outcome_unknown` for
/// [`DiagnosticEffectCertainty::OutcomeUnknown`] and `reconcile_first` for
/// [`DiagnosticRetry::ReconcileFirst`].
#[doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/diagnostic-reconciliation-implication.display.mathml"
))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticProjection {
    code: String,
    stage: DiagnosticStage,
    disposition: DiagnosticDisposition,
    summary: String,
    subject: Option<(String, String)>,
    source: Option<DiagnosticProjectionSource>,
    schema_reference: Option<(String, Option<String>)>,
    causes: Vec<String>,
    notes: Vec<String>,
    help: Vec<String>,
    facts: Vec<(String, String)>,
    retry: DiagnosticRetry,
    effect: DiagnosticEffectCertainty,
    request: Option<(String, String)>,
    generation: Option<(String, String)>,
    receipt: Option<(String, String)>,
    omitted_causes: u32,
}

impl DiagnosticProjection {
    /// Start an owner projection with its complete policy classification.
    ///
    /// Projection is fallible when this value enters either diagnostic
    /// projection route: codes must satisfy the generated grammar, explanatory
    /// text must remain nonempty after bounding, and
    /// [`DiagnosticEffectCertainty::OutcomeUnknown`] requires both
    /// [`DiagnosticDisposition::EffectOutcomeUnknown`] and
    /// [`DiagnosticRetry::ReconcileFirst`]. The converse does not hold:
    /// ambiguous durable state may require reconciliation even when no external
    /// effect began. The implication and its non-converse are rendered on
    /// [`DiagnosticProjection`].
    #[must_use]
    pub fn new(
        code: String,
        stage: DiagnosticStage,
        disposition: DiagnosticDisposition,
        summary: String,
        retry: DiagnosticRetry,
        effect: DiagnosticEffectCertainty,
    ) -> Self {
        Self {
            code,
            stage,
            disposition,
            summary,
            subject: None,
            source: None,
            schema_reference: None,
            causes: Vec::new(),
            notes: Vec::new(),
            help: Vec::new(),
            facts: Vec::new(),
            retry,
            effect,
            request: None,
            generation: None,
            receipt: None,
            omitted_causes: 0,
        }
    }

    /// Attach the principally affected typed identity.
    ///
    /// The subject is correlation only; it does not prove ownership or grant
    /// authority over the named item.
    #[must_use]
    pub fn with_subject(mut self, kind: String, identity: String) -> Self {
        self.subject = Some((kind, identity));
        self
    }

    /// Attach one logical or root-relative source coordinate.
    #[must_use]
    pub fn with_source(mut self, source: DiagnosticProjectionSource) -> Self {
        self.source = Some(source);
        self
    }

    /// Attach an embedded schema by descriptor-registry ID.
    ///
    /// The façade resolves the exact wire identity and raw-schema digest. The
    /// optional path must be an RFC 6901 JSON Pointer; callers cannot supply or
    /// override either identity.
    #[must_use]
    pub fn with_schema_reference(
        mut self,
        registry_id: String,
        schema_path: Option<String>,
    ) -> Self {
        self.schema_reference = Some((registry_id, schema_path));
        self
    }

    /// Attach raw ordered cause messages plus any omissions already established
    /// by the caller's typed source-chain traversal.
    ///
    /// The façade retains at most sixteen entries and adds further omissions to
    /// the supplied count.
    #[must_use]
    pub fn with_causes(mut self, causes: Vec<String>, omitted: u32) -> Self {
        self.causes = causes;
        self.omitted_causes = omitted;
        self
    }

    /// Attach ordered sibling or contextual explanations.
    #[must_use]
    pub fn with_notes(mut self, notes: Vec<String>) -> Self {
        self.notes = notes;
        self
    }

    /// Attach ordered recovery suggestions.
    #[must_use]
    pub fn with_help(mut self, help: Vec<String>) -> Self {
        self.help = help;
        self
    }

    /// Attach presentation-only key/value context.
    ///
    /// Keys must be unique and satisfy the generated lower-snake-case grammar.
    /// Values are centrally redacted and bounded; they never become evidence or
    /// authority.
    #[must_use]
    pub fn with_facts(mut self, facts: Vec<(String, String)>) -> Self {
        self.facts = facts;
        self
    }

    /// Attach a request identity for correlation only.
    #[must_use]
    pub fn with_request_correlation(mut self, kind: String, identity: String) -> Self {
        self.request = Some((kind, identity));
        self
    }

    /// Attach a generation identity for correlation only.
    ///
    /// This correlation does not establish generation admission, activation, or
    /// freshness.
    #[must_use]
    pub fn with_generation_correlation(mut self, kind: String, identity: String) -> Self {
        self.generation = Some((kind, identity));
        self
    }

    /// Attach a receipt identity for correlation only.
    ///
    /// This correlation does not verify the receipt or establish an effect.
    #[must_use]
    pub fn with_receipt_correlation(mut self, kind: String, identity: String) -> Self {
        self.receipt = Some((kind, identity));
        self
    }

    /// Send this candidate to the application's shared diagnostic owner.
    pub(crate) fn admit(
        self,
        diagnostic_identity: &RecordSchemaIdentity,
    ) -> Result<DiagnosticV1, DiagnosticAdmissionError> {
        pale_application::diagnostic::DiagnosticProjection {
            code: self.code,
            stage: self.stage.into_wire(),
            disposition: self.disposition.into_wire(),
            summary: self.summary,
            subject: self.subject,
            source: self.source.map(|source| DiagnosticSourceV1Wire {
                label: source.label,
                line: source.line,
                column: source.column,
                byte_start: source.byte_start,
                byte_end: source.byte_end,
                value_path: source.value_path,
            }),
            schema_reference: self.schema_reference,
            causes: self.causes,
            notes: self.notes,
            help: self.help,
            facts: self.facts,
            retry: self.retry.into_wire(),
            effect: self.effect.into_wire(),
            request: self.request,
            generation: self.generation,
            receipt: self.receipt,
            omitted_causes: self.omitted_causes,
        }
        .admit(diagnostic_identity)
        .map(DiagnosticV1::from_admitted)
    }
}

/// Unadmitted safe-source candidate for [`DiagnosticProjection`].
///
/// Admission rejects absolute, parent-traversing, or control-bearing labels;
/// zero positions; columns without lines; and incomplete or reversed spans.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticProjectionSource {
    label: String,
    line: Option<u32>,
    column: Option<u32>,
    byte_start: Option<u32>,
    byte_end: Option<u32>,
    value_path: Option<String>,
}

impl DiagnosticProjectionSource {
    /// Start a logical or root-relative source coordinate.
    #[must_use]
    pub fn new(label: String) -> Self {
        Self {
            label,
            line: None,
            column: None,
            byte_start: None,
            byte_end: None,
            value_path: None,
        }
    }

    /// Attach a one-based line and optional one-based column.
    #[must_use]
    pub fn with_position(mut self, line: u32, column: Option<u32>) -> Self {
        self.line = Some(line);
        self.column = column;
        self
    }

    /// Attach the half-open byte range `start..end` within the bounded source.
    #[must_use]
    pub fn with_byte_span(mut self, start: u32, end: u32) -> Self {
        self.byte_start = Some(start);
        self.byte_end = Some(end);
        self
    }

    /// Attach a JSON Pointer or typed decoder path within the source value.
    #[must_use]
    pub fn with_value_path(mut self, value_path: String) -> Self {
        self.value_path = Some(value_path);
        self
    }
}

/// Semantically admitted initial-v1 diagnostic.
///
/// Fields remain private so consumers cannot bypass admission or treat a
/// display fact as authority. Focused accessors expose classification and
/// correlation needed by protocol and terminal views; consumers that need the
/// complete portable record use [`Self::canonical_bytes`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticV1 {
    wire: DiagnosticV1Wire,
    canonical_json: String,
    identity: DiagnosticIdentity,
}

/// Opaque content identity of one semantically admitted diagnostic.
///
/// The API derives this domain-separated identity from the exact canonical
/// `diagnostic.v1` bytes. Outer leaves may correlate the same diagnostic but
/// cannot construct or reinterpret its semantic classification.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
#[serde(transparent)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
pub struct DiagnosticIdentity(
    #[cfg_attr(
        feature = "wire-schema",
        schemars(length(equal = 64), regex(pattern = r"^[0-9a-f]{64}$"))
    )]
    String,
);

impl DiagnosticIdentity {
    /// Borrow the opaque lowercase SHA-256 representation.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Closed recovery class of an admitted [`DiagnosticV1`].
///
/// This façade-owned view lets Rust consumers make exhaustive policy decisions
/// without depending on the broader wire-record module or parsing wire strings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticDisposition {
    /// A runtime prerequisite or observation failed operationally.
    OperationalFailure,
    /// Supplied bytes or authored meaning must be corrected.
    InvalidInput,
    /// Current admitted state blocks an otherwise meaningful request.
    Blocked,
    /// Pale application state or behavior contradicts an invariant.
    InternalFailure,
    /// An external effect may have occurred and requires reconciliation.
    EffectOutcomeUnknown,
}

impl std::fmt::Display for DiagnosticDisposition {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::OperationalFailure => "operational_failure",
            Self::InvalidInput => "invalid_input",
            Self::Blocked => "blocked",
            Self::InternalFailure => "internal_failure",
            Self::EffectOutcomeUnknown => "effect_outcome_unknown",
        })
    }
}

impl DiagnosticDisposition {
    const fn into_wire(self) -> DiagnosticDispositionV1 {
        match self {
            Self::OperationalFailure => DiagnosticDispositionV1::OperationalFailure,
            Self::InvalidInput => DiagnosticDispositionV1::InvalidInput,
            Self::Blocked => DiagnosticDispositionV1::Blocked,
            Self::InternalFailure => DiagnosticDispositionV1::InternalFailure,
            Self::EffectOutcomeUnknown => DiagnosticDispositionV1::EffectOutcomeUnknown,
        }
    }
}

/// Closed processing stage at which an admitted [`DiagnosticV1`] was established.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticStage {
    /// External bytes or command values failed initial admission.
    InputAdmission,
    /// A value failed an embedded structural contract.
    ContractValidation,
    /// A structurally valid value failed owner-specific meaning.
    SemanticAdmission,
    /// Package sources failed loading or verification.
    PackageLoading,
    /// Principal-workspace sources failed loading or verification.
    WorkspaceLoading,
    /// Compiler resolution, projection, or sealing failed.
    Compilation,
    /// A Pale-rendered surface could not be produced.
    Rendering,
    /// An operation was not admitted under current compiled state.
    OperationAdmission,
    /// A resident generation or durable runtime transition failed admission.
    ResidentAdmission,
    /// Recovery could not establish a safe continuation or retry posture.
    ResidentRecovery,
    /// An effect-free plan could not be constructed or admitted.
    Planning,
    /// An admitted plan could not be materialized or reconciled.
    Materialization,
    /// A pure built-in action failed admission or computation.
    NativeAction,
    /// An external prerequisite could not be observed reliably.
    HostObservation,
    /// A Pale application invariant failed outside a narrower stage.
    Internal,
}

impl std::fmt::Display for DiagnosticStage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InputAdmission => "input_admission",
            Self::ContractValidation => "contract_validation",
            Self::SemanticAdmission => "semantic_admission",
            Self::PackageLoading => "package_loading",
            Self::WorkspaceLoading => "workspace_loading",
            Self::Compilation => "compilation",
            Self::Rendering => "rendering",
            Self::OperationAdmission => "operation_admission",
            Self::ResidentAdmission => "resident_admission",
            Self::ResidentRecovery => "resident_recovery",
            Self::Planning => "planning",
            Self::Materialization => "materialization",
            Self::NativeAction => "native_action",
            Self::HostObservation => "host_observation",
            Self::Internal => "internal",
        })
    }
}

impl DiagnosticStage {
    const fn into_wire(self) -> DiagnosticStageV1 {
        match self {
            Self::InputAdmission => DiagnosticStageV1::InputAdmission,
            Self::ContractValidation => DiagnosticStageV1::ContractValidation,
            Self::SemanticAdmission => DiagnosticStageV1::SemanticAdmission,
            Self::PackageLoading => DiagnosticStageV1::PackageLoading,
            Self::WorkspaceLoading => DiagnosticStageV1::WorkspaceLoading,
            Self::Compilation => DiagnosticStageV1::Compilation,
            Self::Rendering => DiagnosticStageV1::Rendering,
            Self::OperationAdmission => DiagnosticStageV1::OperationAdmission,
            Self::ResidentAdmission => DiagnosticStageV1::ResidentAdmission,
            Self::ResidentRecovery => DiagnosticStageV1::ResidentRecovery,
            Self::Planning => DiagnosticStageV1::Planning,
            Self::Materialization => DiagnosticStageV1::Materialization,
            Self::NativeAction => DiagnosticStageV1::NativeAction,
            Self::HostObservation => DiagnosticStageV1::HostObservation,
            Self::Internal => DiagnosticStageV1::Internal,
        }
    }
}

/// Closed importance class of an admitted [`DiagnosticV1`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    /// The requested operation did not complete successfully.
    Error,
}

impl std::fmt::Display for DiagnosticSeverity {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("error")
    }
}

/// Closed retry posture of an admitted [`DiagnosticV1`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticRetry {
    /// No retry instruction is established.
    NotIndicated,
    /// Retry only after correcting rejected input or authored meaning.
    AfterCorrection,
    /// Retry only after admitted state or a prerequisite changes.
    AfterStateChange,
    /// Reconcile retained evidence before deciding whether to retry.
    ReconcileFirst,
}

impl std::fmt::Display for DiagnosticRetry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::NotIndicated => "not_indicated",
            Self::AfterCorrection => "after_correction",
            Self::AfterStateChange => "after_state_change",
            Self::ReconcileFirst => "reconcile_first",
        })
    }
}

impl DiagnosticRetry {
    const fn into_wire(self) -> DiagnosticRetryV1 {
        match self {
            Self::NotIndicated => DiagnosticRetryV1::NotIndicated,
            Self::AfterCorrection => DiagnosticRetryV1::AfterCorrection,
            Self::AfterStateChange => DiagnosticRetryV1::AfterStateChange,
            Self::ReconcileFirst => DiagnosticRetryV1::ReconcileFirst,
        }
    }
}

/// Closed effect-certainty class of an admitted [`DiagnosticV1`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticEffectCertainty {
    /// The operation has no external-effect boundary.
    NotApplicable,
    /// The selected external-effect boundary was proven not to have started.
    NotStarted,
    /// The external effect and result were confirmed by owner evidence.
    Confirmed,
    /// Available evidence cannot establish whether the effect occurred.
    OutcomeUnknown,
}

impl std::fmt::Display for DiagnosticEffectCertainty {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::NotApplicable => "not_applicable",
            Self::NotStarted => "not_started",
            Self::Confirmed => "confirmed",
            Self::OutcomeUnknown => "outcome_unknown",
        })
    }
}

impl DiagnosticEffectCertainty {
    pub(crate) const fn from_wire(value: DiagnosticEffectCertaintyV1) -> Self {
        match value {
            DiagnosticEffectCertaintyV1::NotApplicable => Self::NotApplicable,
            DiagnosticEffectCertaintyV1::NotStarted => Self::NotStarted,
            DiagnosticEffectCertaintyV1::Confirmed => Self::Confirmed,
            DiagnosticEffectCertaintyV1::OutcomeUnknown => Self::OutcomeUnknown,
        }
    }

    const fn into_wire(self) -> DiagnosticEffectCertaintyV1 {
        match self {
            Self::NotApplicable => DiagnosticEffectCertaintyV1::NotApplicable,
            Self::NotStarted => DiagnosticEffectCertaintyV1::NotStarted,
            Self::Confirmed => DiagnosticEffectCertaintyV1::Confirmed,
            Self::OutcomeUnknown => DiagnosticEffectCertaintyV1::OutcomeUnknown,
        }
    }
}

/// Owned safe source coordinate from an admitted [`DiagnosticV1`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticSource {
    label: String,
    line: Option<u32>,
    column: Option<u32>,
    byte_start: Option<u32>,
    byte_end: Option<u32>,
    value_path: Option<String>,
}

impl DiagnosticSource {
    /// Root-relative or opaque source label with no operating-system path authority.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// One-based source line when supplied by the diagnostic owner.
    #[must_use]
    pub fn line(&self) -> Option<u32> {
        self.line
    }

    /// One-based source column when supplied by the diagnostic owner.
    #[must_use]
    pub fn column(&self) -> Option<u32> {
        self.column
    }

    /// Inclusive source byte offset when the owner supplied a complete span.
    #[must_use]
    pub fn byte_start(&self) -> Option<u32> {
        self.byte_start
    }

    /// Exclusive source byte offset when the owner supplied a complete span.
    #[must_use]
    pub fn byte_end(&self) -> Option<u32> {
        self.byte_end
    }

    /// JSON Pointer or typed decoder path within the safe source label.
    #[must_use]
    pub fn value_path(&self) -> Option<&str> {
        self.value_path.as_deref()
    }
}

/// Owned generated-schema coordinate from an admitted [`DiagnosticV1`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticSchemaCoordinate {
    registry_id: String,
    wire_schema_id: Option<String>,
    schema_sha256: String,
    schema_path: Option<String>,
}

impl DiagnosticSchemaCoordinate {
    /// Descriptor-manifest identity of the implicated generated schema.
    #[must_use]
    pub fn registry_id(&self) -> &str {
        &self.registry_id
    }

    /// Self-carried wire identity when the generated descriptor declares one.
    #[must_use]
    pub fn wire_schema_id(&self) -> Option<&str> {
        self.wire_schema_id.as_deref()
    }

    /// SHA-256 of the exact generated schema bytes selected by the diagnostic.
    #[must_use]
    pub fn schema_sha256(&self) -> &str {
        &self.schema_sha256
    }

    /// JSON Pointer to the governing schema keyword when known.
    #[must_use]
    pub fn schema_path(&self) -> Option<&str> {
        self.schema_path.as_deref()
    }
}

/// Owned presentation-only fact from an admitted [`DiagnosticV1`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticFact {
    key: String,
    value: String,
}

impl DiagnosticFact {
    /// Stable presentation key; it is not evidence or authority.
    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Bounded redaction-safe presentation value.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

/// Omission accounting carried by an admitted [`DiagnosticV1`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiagnosticTruncation {
    shortened_fields: u32,
    omitted_causes: u32,
    omitted_notes: u32,
    omitted_help: u32,
    omitted_facts: u32,
}

impl DiagnosticTruncation {
    /// Scalar fields shortened to their contract ceiling.
    #[must_use]
    pub fn shortened_fields(self) -> u32 {
        self.shortened_fields
    }

    /// Causal entries omitted from the bounded retained chain.
    #[must_use]
    pub fn omitted_causes(self) -> u32 {
        self.omitted_causes
    }

    /// Sibling notes omitted from the bounded retained set.
    #[must_use]
    pub fn omitted_notes(self) -> u32 {
        self.omitted_notes
    }

    /// Recovery suggestions omitted from the bounded retained set.
    #[must_use]
    pub fn omitted_help(self) -> u32 {
        self.omitted_help
    }

    /// Presentation facts omitted from the bounded retained set.
    #[must_use]
    pub fn omitted_facts(self) -> u32 {
        self.omitted_facts
    }

    /// Whether any diagnostic detail was shortened or omitted.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.shortened_fields == 0
            && self.omitted_causes == 0
            && self.omitted_notes == 0
            && self.omitted_help == 0
            && self.omitted_facts == 0
    }
}

impl DiagnosticV1 {
    /// Validate stored bytes through the shared application owner.
    fn from_bytes(bytes: &[u8]) -> Result<Self, DiagnosticAdmissionError> {
        AdmittedDiagnostic::from_bytes(bytes).map(Self::from_admitted)
    }

    /// Transfer checked fields without parsing or deriving their identity again.
    pub(crate) fn from_admitted(admitted: AdmittedDiagnostic) -> Self {
        let (wire, canonical_json, identity) = admitted.into_parts();
        Self {
            wire,
            canonical_json,
            identity: DiagnosticIdentity(identity),
        }
    }

    /// Stable wire schema carried by the admitted diagnostic.
    #[must_use]
    pub fn schema(&self) -> &str {
        self.wire.schema.as_str()
    }

    /// Initial-v1 wire major carried by the admitted diagnostic.
    #[must_use]
    pub fn schema_version(&self) -> u32 {
        self.wire.schema_version
    }

    /// Stable machine-readable failure code.
    #[must_use]
    pub fn code(&self) -> &str {
        &self.wire.code
    }

    /// Closed stage for exhaustive Rust policy decisions.
    #[must_use]
    pub fn stage(&self) -> DiagnosticStage {
        match self.wire.stage {
            DiagnosticStageV1::InputAdmission => DiagnosticStage::InputAdmission,
            DiagnosticStageV1::ContractValidation => DiagnosticStage::ContractValidation,
            DiagnosticStageV1::SemanticAdmission => DiagnosticStage::SemanticAdmission,
            DiagnosticStageV1::PackageLoading => DiagnosticStage::PackageLoading,
            DiagnosticStageV1::WorkspaceLoading => DiagnosticStage::WorkspaceLoading,
            DiagnosticStageV1::Compilation => DiagnosticStage::Compilation,
            DiagnosticStageV1::Rendering => DiagnosticStage::Rendering,
            DiagnosticStageV1::OperationAdmission => DiagnosticStage::OperationAdmission,
            DiagnosticStageV1::ResidentAdmission => DiagnosticStage::ResidentAdmission,
            DiagnosticStageV1::ResidentRecovery => DiagnosticStage::ResidentRecovery,
            DiagnosticStageV1::Planning => DiagnosticStage::Planning,
            DiagnosticStageV1::Materialization => DiagnosticStage::Materialization,
            DiagnosticStageV1::NativeAction => DiagnosticStage::NativeAction,
            DiagnosticStageV1::HostObservation => DiagnosticStage::HostObservation,
            DiagnosticStageV1::Internal => DiagnosticStage::Internal,
        }
    }

    /// Closed recovery class for exhaustive Rust policy decisions.
    #[must_use]
    pub fn disposition(&self) -> DiagnosticDisposition {
        match self.wire.disposition {
            DiagnosticDispositionV1::OperationalFailure => {
                DiagnosticDisposition::OperationalFailure
            }
            DiagnosticDispositionV1::InvalidInput => DiagnosticDisposition::InvalidInput,
            DiagnosticDispositionV1::Blocked => DiagnosticDisposition::Blocked,
            DiagnosticDispositionV1::InternalFailure => DiagnosticDisposition::InternalFailure,
            DiagnosticDispositionV1::EffectOutcomeUnknown => {
                DiagnosticDisposition::EffectOutcomeUnknown
            }
        }
    }

    /// Closed retry posture for exhaustive Rust policy decisions.
    #[must_use]
    pub fn retry(&self) -> DiagnosticRetry {
        match self.wire.retry {
            DiagnosticRetryV1::NotIndicated => DiagnosticRetry::NotIndicated,
            DiagnosticRetryV1::AfterCorrection => DiagnosticRetry::AfterCorrection,
            DiagnosticRetryV1::AfterStateChange => DiagnosticRetry::AfterStateChange,
            DiagnosticRetryV1::ReconcileFirst => DiagnosticRetry::ReconcileFirst,
        }
    }

    /// Closed effect certainty for exhaustive Rust policy decisions.
    #[must_use]
    pub fn effect(&self) -> DiagnosticEffectCertainty {
        match self.wire.effect {
            DiagnosticEffectCertaintyV1::NotApplicable => DiagnosticEffectCertainty::NotApplicable,
            DiagnosticEffectCertaintyV1::NotStarted => DiagnosticEffectCertainty::NotStarted,
            DiagnosticEffectCertaintyV1::Confirmed => DiagnosticEffectCertainty::Confirmed,
            DiagnosticEffectCertaintyV1::OutcomeUnknown => {
                DiagnosticEffectCertainty::OutcomeUnknown
            }
        }
    }

    /// Closed severity for exhaustive Rust policy decisions.
    #[must_use]
    pub fn severity(&self) -> DiagnosticSeverity {
        match self.wire.severity {
            DiagnosticSeverityV1::Error => DiagnosticSeverity::Error,
        }
    }

    /// Concise redaction-safe explanation.
    #[must_use]
    pub fn summary(&self) -> &str {
        &self.wire.summary
    }

    /// Optional typed subject as `(kind, identity)`.
    ///
    /// This is presentation correlation only; it neither proves ownership nor
    /// authorizes an operation on the named subject.
    #[must_use]
    pub fn subject(&self) -> Option<(&str, &str)> {
        self.wire
            .subject
            .as_ref()
            .map(|subject| (subject.kind.as_str(), subject.id.as_str()))
    }

    /// Optional typed request correlation as `(kind, identity)`.
    ///
    /// Correlation does not establish that an effect occurred or that retry is
    /// safe; those decisions remain governed by disposition and effect state.
    #[must_use]
    pub fn request_correlation(&self) -> Option<(&str, &str)> {
        self.wire
            .request
            .as_ref()
            .map(|request| (request.kind.as_str(), request.id.as_str()))
    }

    /// Optional typed generation correlation as `(kind, identity)`.
    #[must_use]
    pub fn generation_correlation(&self) -> Option<(&str, &str)> {
        self.wire
            .generation
            .as_ref()
            .map(|generation| (generation.kind.as_str(), generation.id.as_str()))
    }

    /// Optional typed receipt correlation as `(kind, identity)`.
    #[must_use]
    pub fn receipt_correlation(&self) -> Option<(&str, &str)> {
        self.wire
            .receipt
            .as_ref()
            .map(|receipt| (receipt.kind.as_str(), receipt.id.as_str()))
    }

    /// Copy the optional safe source coordinate for a presentation leaf.
    #[must_use]
    pub fn source(&self) -> Option<DiagnosticSource> {
        self.wire.source.as_ref().map(|source| DiagnosticSource {
            label: source.label.clone(),
            line: source.line,
            column: source.column,
            byte_start: source.byte_start,
            byte_end: source.byte_end,
            value_path: source.value_path.clone(),
        })
    }

    /// Copy the optional generated-schema coordinate for a presentation leaf.
    #[must_use]
    pub fn schema_coordinate(&self) -> Option<DiagnosticSchemaCoordinate> {
        self.wire
            .schema_coordinate
            .as_ref()
            .map(|coordinate| DiagnosticSchemaCoordinate {
                registry_id: coordinate.registry_id.clone(),
                wire_schema_id: coordinate.wire_schema_id.as_ref().map(ToString::to_string),
                schema_sha256: coordinate.schema_sha256.to_string(),
                schema_path: coordinate.schema_path.clone(),
            })
    }

    /// Ordered redaction-safe source-chain messages after [`Self::summary`].
    #[must_use]
    pub fn causes(&self) -> &[String] {
        &self.wire.causes
    }

    /// Ordered sibling or contextual explanations.
    #[must_use]
    pub fn notes(&self) -> &[String] {
        &self.wire.notes
    }

    /// Ordered bounded recovery suggestions.
    #[must_use]
    pub fn help(&self) -> &[String] {
        &self.wire.help
    }

    /// Copy presentation-only facts without exposing the wire record module.
    #[must_use]
    pub fn facts(&self) -> Vec<DiagnosticFact> {
        self.wire
            .facts
            .iter()
            .map(|fact| DiagnosticFact {
                key: fact.key.clone(),
                value: fact.value.clone(),
            })
            .collect()
    }

    /// Exact omission accounting established by the diagnostic owner.
    #[must_use]
    pub fn truncation(&self) -> DiagnosticTruncation {
        DiagnosticTruncation {
            shortened_fields: self.wire.truncation.shortened_fields,
            omitted_causes: self.wire.truncation.omitted_causes,
            omitted_notes: self.wire.truncation.omitted_notes,
            omitted_help: self.wire.truncation.omitted_help,
            omitted_facts: self.wire.truncation.omitted_facts,
        }
    }

    /// Borrow the RFC 8785 canonical `diagnostic.v1` JSON text.
    #[must_use]
    pub fn canonical_json(&self) -> &str {
        &self.canonical_json
    }

    /// Borrow the RFC 8785 canonical `diagnostic.v1` bytes established at admission.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        self.canonical_json.as_bytes()
    }

    /// Opaque API-owned identity of these exact admitted diagnostic bytes.
    #[must_use]
    pub const fn identity(&self) -> &DiagnosticIdentity {
        &self.identity
    }
}

impl PaleApi {
    /// Project a typed owner classification into an admitted initial-v1 diagnostic.
    ///
    /// This is the Rust-consumer counterpart to [`Self::admit_diagnostic`]. It
    /// accepts no wire identities from the caller: schema identity and digest
    /// come from this façade's verified embedded registry. The returned
    /// [`DiagnosticV1`] is the single source for both machine and human views.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when the projection violates generated shape,
    /// logical-source safety, schema-coordinate integrity, unique-fact, or
    /// effect/retry semantics. In particular, an unknown external-effect
    /// outcome requires reconciliation before retry, while reconciliation may
    /// also be required for ambiguous durable state when no external effect
    /// started. No partially projected diagnostic is returned.
    pub fn project_diagnostic(
        &self,
        projection: DiagnosticProjection,
    ) -> Result<DiagnosticV1, ApiError> {
        project_diagnostic(&self.diagnostic_identity, projection)
    }

    /// Admit untrusted initial-v1 diagnostic bytes through the complete boundary.
    ///
    /// Admission applies strict duplicate-free JSON parsing, the generated
    /// `diagnostic.v1` schema, typed decoding, and the semantic invariants that
    /// JSON Schema cannot express. The returned fields remain private.
    ///
    /// # Examples
    ///
    /// Diagnostics emitted by another façade call re-enter through the same
    /// byte boundary used by non-Rust consumers:
    ///
    /// ```
    /// use pale_sdk::PaleApi;
    ///
    /// let api = PaleApi::new()?;
    /// let failure = match api.contract_schema("not-registered.v1") {
    ///     Ok(_) => return Err("unexpected registered schema".into()),
    ///     Err(failure) => failure,
    /// };
    /// let admitted = api.admit_diagnostic(failure.diagnostic().canonical_bytes())?;
    /// assert_eq!(admitted.code(), failure.diagnostic().code());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when the input exceeds the bounded ingress size or
    /// fails any stage of diagnostic admission. The error preserves the typed
    /// source while presenting a separately admitted diagnostic.
    pub fn admit_diagnostic(&self, bytes: &[u8]) -> Result<DiagnosticV1, ApiError> {
        if bytes.len() > DIAGNOSTIC_INPUT_LIMIT {
            let error = BoundaryError::InputTooLarge {
                limit: DIAGNOSTIC_INPUT_LIMIT,
            };
            return Err(self.diagnostic_failure(
                error,
                DiagnosticFailure {
                    code: "input_too_large",
                    stage: DiagnosticStageV1::InputAdmission,
                    disposition: DiagnosticDispositionV1::InvalidInput,
                    summary: "diagnostic input exceeds the bounded API input",
                    location: None,
                    schema_path: None,
                    facts: Vec::new(),
                },
            ));
        }
        DiagnosticV1::from_bytes(bytes).map_err(|error| {
            let failure = match &error {
                DiagnosticAdmissionError::StrictJson(_) => DiagnosticFailure {
                    code: "invalid_json",
                    stage: DiagnosticStageV1::InputAdmission,
                    disposition: DiagnosticDispositionV1::InvalidInput,
                    summary: "diagnostic input is not strict JSON",
                    location: None,
                    schema_path: None,
                    facts: Vec::new(),
                },
                DiagnosticAdmissionError::Contract(schema_error) => diagnostic_contract_failure(
                    schema_error,
                    "diagnostic_contract_rejected",
                    "diagnostic input does not satisfy the initial-v1 contract",
                ),
                DiagnosticAdmissionError::Decode(_) => DiagnosticFailure {
                    code: "diagnostic_decode_invariant",
                    stage: DiagnosticStageV1::Internal,
                    disposition: DiagnosticDispositionV1::InternalFailure,
                    summary: "schema-valid diagnostic input contradicts its Rust wire owner",
                    location: None,
                    schema_path: None,
                    facts: Vec::new(),
                },
                DiagnosticAdmissionError::Canonicalize(_) => DiagnosticFailure {
                    code: "diagnostic_encoding_invariant",
                    stage: DiagnosticStageV1::Internal,
                    disposition: DiagnosticDispositionV1::InternalFailure,
                    summary: "admitted diagnostic input contradicts the canonical record grammar",
                    location: None,
                    schema_path: None,
                    facts: Vec::new(),
                },
                DiagnosticAdmissionError::Registry(_) => DiagnosticFailure {
                    code: "diagnostic_registry_invariant",
                    stage: DiagnosticStageV1::Internal,
                    disposition: DiagnosticDispositionV1::InternalFailure,
                    summary: "diagnostic admission could not resolve an embedded schema coordinate",
                    location: None,
                    schema_path: None,
                    facts: Vec::new(),
                },
                DiagnosticAdmissionError::ColumnWithoutLine
                | DiagnosticAdmissionError::ZeroSourcePosition
                | DiagnosticAdmissionError::IncompleteByteSpan
                | DiagnosticAdmissionError::ReversedByteSpan
                | DiagnosticAdmissionError::UnsafeSourceLabel
                | DiagnosticAdmissionError::EffectSemanticsMismatch
                | DiagnosticAdmissionError::DuplicateFactKey
                | DiagnosticAdmissionError::InvalidSchemaPath
                | DiagnosticAdmissionError::UnknownSchemaCoordinate
                | DiagnosticAdmissionError::SchemaCoordinateMismatch => DiagnosticFailure {
                    code: "diagnostic_semantic_rejected",
                    stage: DiagnosticStageV1::SemanticAdmission,
                    disposition: DiagnosticDispositionV1::InvalidInput,
                    summary: "diagnostic input violates initial-v1 semantic invariants",
                    location: None,
                    schema_path: None,
                    facts: Vec::new(),
                },
            };
            self.diagnostic_failure(error, failure)
        })
    }

    /// Project one admission source into a separately admitted API diagnostic.
    fn diagnostic_failure<E>(&self, error: E, failure: DiagnosticFailure<'_>) -> ApiError
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        diagnostic_failure(&self.diagnostic_identity, error, failure)
    }
}

fn project_diagnostic(
    diagnostic_identity: &RecordSchemaIdentity,
    projection: DiagnosticProjection,
) -> Result<DiagnosticV1, ApiError> {
    projection.admit(diagnostic_identity).map_err(|error| {
        let failure = match &error {
            DiagnosticAdmissionError::Registry(_)
            | DiagnosticAdmissionError::Decode(_)
            | DiagnosticAdmissionError::Canonicalize(_) => DiagnosticFailure {
                code: "diagnostic_projection_invariant",
                stage: DiagnosticStageV1::Internal,
                disposition: DiagnosticDispositionV1::InternalFailure,
                summary: "diagnostic projection contradicted a Pale application invariant",
                location: None,
                schema_path: None,
                facts: Vec::new(),
            },
            DiagnosticAdmissionError::Contract(schema_error) => diagnostic_contract_failure(
                schema_error,
                "diagnostic_projection_rejected",
                "diagnostic projection violates the initial-v1 contract",
            ),
            DiagnosticAdmissionError::StrictJson(_)
            | DiagnosticAdmissionError::ColumnWithoutLine
            | DiagnosticAdmissionError::ZeroSourcePosition
            | DiagnosticAdmissionError::IncompleteByteSpan
            | DiagnosticAdmissionError::ReversedByteSpan
            | DiagnosticAdmissionError::UnsafeSourceLabel
            | DiagnosticAdmissionError::EffectSemanticsMismatch
            | DiagnosticAdmissionError::DuplicateFactKey
            | DiagnosticAdmissionError::InvalidSchemaPath
            | DiagnosticAdmissionError::UnknownSchemaCoordinate
            | DiagnosticAdmissionError::SchemaCoordinateMismatch => DiagnosticFailure {
                code: "diagnostic_projection_rejected",
                stage: DiagnosticStageV1::SemanticAdmission,
                disposition: DiagnosticDispositionV1::InvalidInput,
                summary: "diagnostic projection violates initial-v1 semantic invariants",
                location: None,
                schema_path: None,
                facts: Vec::new(),
            },
        };
        diagnostic_failure(diagnostic_identity, error, failure)
    })
}

fn diagnostic_failure<E>(
    diagnostic_identity: &RecordSchemaIdentity,
    error: E,
    failure: DiagnosticFailure<'_>,
) -> ApiError
where
    E: std::error::Error + Send + Sync + 'static,
{
    ApiError::new(
        diagnostic_identity,
        error,
        FailureSpecification {
            code: failure.code,
            stage: failure.stage,
            disposition: failure.disposition,
            summary: failure.summary,
            subject: Some(pale_core::records::DiagnosticSubjectV1Wire {
                kind: "schema".to_string(),
                id: DIAGNOSTIC_V1_SCHEMA_ID.to_string(),
            }),
            schema_coordinate: Some(DiagnosticSchemaCoordinateV1Wire {
                registry_id: DIAGNOSTIC_V1_SCHEMA_ID.to_string(),
                wire_schema_id: Some(diagnostic_identity.schema.clone()),
                schema_sha256: diagnostic_identity.schema_sha256.clone(),
                schema_path: failure.schema_path,
            }),
            location: failure.location,
            retry: if failure.disposition == DiagnosticDispositionV1::InvalidInput {
                pale_core::records::DiagnosticRetryV1::AfterCorrection
            } else {
                pale_core::records::DiagnosticRetryV1::NotIndicated
            },
            effect: DiagnosticEffectCertaintyV1::NotApplicable,
            request: None,
            notes: Vec::new(),
            help: Vec::new(),
            facts: failure.facts,
        },
    )
}

#[cfg(test)]
#[path = "../tests/internal/diagnostic.rs"]
mod tests;
