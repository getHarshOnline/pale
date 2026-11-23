// Node diagnostic projection fragment included by the shallow façade.

/// Node representation of API-owned [`DiagnosticStage`].
#[napi(js_name = "DiagnosticStage")]
pub enum DiagnosticStage {
    /// External input admission.
    InputAdmission,
    /// Generated contract validation.
    ContractValidation,
    /// Owner-specific semantic admission.
    SemanticAdmission,
    /// Package source loading.
    PackageLoading,
    /// Workspace source loading.
    WorkspaceLoading,
    /// Compiler resolution or sealing.
    Compilation,
    /// Generated-surface rendering.
    Rendering,
    /// Operation admission.
    OperationAdmission,
    /// Resident generation or durable transition admission.
    ResidentAdmission,
    /// Resident recovery and safe-continuation proof.
    ResidentRecovery,
    /// Effect-free planning.
    Planning,
    /// Materialization or reconciliation.
    Materialization,
    /// Pure native-action computation.
    NativeAction,
    /// External prerequisite observation.
    HostObservation,
    /// Other Pale application invariant.
    Internal,
}

impl From<DiagnosticStage> for ApiDiagnosticStage {
    fn from(value: DiagnosticStage) -> Self {
        match value {
            DiagnosticStage::InputAdmission => Self::InputAdmission,
            DiagnosticStage::ContractValidation => Self::ContractValidation,
            DiagnosticStage::SemanticAdmission => Self::SemanticAdmission,
            DiagnosticStage::PackageLoading => Self::PackageLoading,
            DiagnosticStage::WorkspaceLoading => Self::WorkspaceLoading,
            DiagnosticStage::Compilation => Self::Compilation,
            DiagnosticStage::Rendering => Self::Rendering,
            DiagnosticStage::OperationAdmission => Self::OperationAdmission,
            DiagnosticStage::ResidentAdmission => Self::ResidentAdmission,
            DiagnosticStage::ResidentRecovery => Self::ResidentRecovery,
            DiagnosticStage::Planning => Self::Planning,
            DiagnosticStage::Materialization => Self::Materialization,
            DiagnosticStage::NativeAction => Self::NativeAction,
            DiagnosticStage::HostObservation => Self::HostObservation,
            DiagnosticStage::Internal => Self::Internal,
        }
    }
}

impl From<ApiDiagnosticStage> for DiagnosticStage {
    fn from(value: ApiDiagnosticStage) -> Self {
        match value {
            ApiDiagnosticStage::InputAdmission => Self::InputAdmission,
            ApiDiagnosticStage::ContractValidation => Self::ContractValidation,
            ApiDiagnosticStage::SemanticAdmission => Self::SemanticAdmission,
            ApiDiagnosticStage::PackageLoading => Self::PackageLoading,
            ApiDiagnosticStage::WorkspaceLoading => Self::WorkspaceLoading,
            ApiDiagnosticStage::Compilation => Self::Compilation,
            ApiDiagnosticStage::Rendering => Self::Rendering,
            ApiDiagnosticStage::OperationAdmission => Self::OperationAdmission,
            ApiDiagnosticStage::ResidentAdmission => Self::ResidentAdmission,
            ApiDiagnosticStage::ResidentRecovery => Self::ResidentRecovery,
            ApiDiagnosticStage::Planning => Self::Planning,
            ApiDiagnosticStage::Materialization => Self::Materialization,
            ApiDiagnosticStage::NativeAction => Self::NativeAction,
            ApiDiagnosticStage::HostObservation => Self::HostObservation,
            ApiDiagnosticStage::Internal => Self::Internal,
        }
    }
}

/// Node representation of API-owned [`DiagnosticDisposition`].
#[napi(js_name = "DiagnosticDisposition")]
pub enum DiagnosticDisposition {
    /// Runtime prerequisite failure.
    OperationalFailure,
    /// Input or authored meaning requires correction.
    InvalidInput,
    /// Current admitted state blocks the request.
    Blocked,
    /// Pale application invariant failure.
    InternalFailure,
    /// An external effect may have occurred and requires reconciliation.
    EffectOutcomeUnknown,
}

impl From<DiagnosticDisposition> for ApiDiagnosticDisposition {
    fn from(value: DiagnosticDisposition) -> Self {
        match value {
            DiagnosticDisposition::OperationalFailure => Self::OperationalFailure,
            DiagnosticDisposition::InvalidInput => Self::InvalidInput,
            DiagnosticDisposition::Blocked => Self::Blocked,
            DiagnosticDisposition::InternalFailure => Self::InternalFailure,
            DiagnosticDisposition::EffectOutcomeUnknown => Self::EffectOutcomeUnknown,
        }
    }
}

impl From<ApiDiagnosticDisposition> for DiagnosticDisposition {
    fn from(value: ApiDiagnosticDisposition) -> Self {
        match value {
            ApiDiagnosticDisposition::OperationalFailure => Self::OperationalFailure,
            ApiDiagnosticDisposition::InvalidInput => Self::InvalidInput,
            ApiDiagnosticDisposition::Blocked => Self::Blocked,
            ApiDiagnosticDisposition::InternalFailure => Self::InternalFailure,
            ApiDiagnosticDisposition::EffectOutcomeUnknown => Self::EffectOutcomeUnknown,
        }
    }
}

/// Node representation of API-owned [`DiagnosticRetry`].
#[napi(js_name = "DiagnosticRetry")]
pub enum DiagnosticRetry {
    /// No retry instruction is established.
    NotIndicated,
    /// Retry after correction.
    AfterCorrection,
    /// Retry after state changes.
    AfterStateChange,
    /// Reconcile before deciding whether to retry.
    ReconcileFirst,
}

impl From<DiagnosticRetry> for ApiDiagnosticRetry {
    fn from(value: DiagnosticRetry) -> Self {
        match value {
            DiagnosticRetry::NotIndicated => Self::NotIndicated,
            DiagnosticRetry::AfterCorrection => Self::AfterCorrection,
            DiagnosticRetry::AfterStateChange => Self::AfterStateChange,
            DiagnosticRetry::ReconcileFirst => Self::ReconcileFirst,
        }
    }
}

impl From<ApiDiagnosticRetry> for DiagnosticRetry {
    fn from(value: ApiDiagnosticRetry) -> Self {
        match value {
            ApiDiagnosticRetry::NotIndicated => Self::NotIndicated,
            ApiDiagnosticRetry::AfterCorrection => Self::AfterCorrection,
            ApiDiagnosticRetry::AfterStateChange => Self::AfterStateChange,
            ApiDiagnosticRetry::ReconcileFirst => Self::ReconcileFirst,
        }
    }
}

/// Node representation of API-owned [`DiagnosticEffectCertainty`].
#[napi(js_name = "DiagnosticEffectCertainty")]
pub enum DiagnosticEffectCertainty {
    /// No external effect boundary exists.
    NotApplicable,
    /// Effect execution is proven not to have started.
    NotStarted,
    /// Owner evidence confirms the effect and result.
    Confirmed,
    /// Available evidence cannot establish whether the external effect occurred.
    OutcomeUnknown,
}

impl From<DiagnosticEffectCertainty> for ApiDiagnosticEffectCertainty {
    fn from(value: DiagnosticEffectCertainty) -> Self {
        match value {
            DiagnosticEffectCertainty::NotApplicable => Self::NotApplicable,
            DiagnosticEffectCertainty::NotStarted => Self::NotStarted,
            DiagnosticEffectCertainty::Confirmed => Self::Confirmed,
            DiagnosticEffectCertainty::OutcomeUnknown => Self::OutcomeUnknown,
        }
    }
}

impl From<ApiDiagnosticEffectCertainty> for DiagnosticEffectCertainty {
    fn from(value: ApiDiagnosticEffectCertainty) -> Self {
        match value {
            ApiDiagnosticEffectCertainty::NotApplicable => Self::NotApplicable,
            ApiDiagnosticEffectCertainty::NotStarted => Self::NotStarted,
            ApiDiagnosticEffectCertainty::Confirmed => Self::Confirmed,
            ApiDiagnosticEffectCertainty::OutcomeUnknown => Self::OutcomeUnknown,
        }
    }
}

/// One presentation fact from an admitted diagnostic.
#[napi(object, js_name = "DiagnosticFact")]
pub struct DiagnosticFact {
    /// Stable fact key.
    pub key: String,
    /// Redaction-safe fact value.
    pub value: String,
}

/// Immutable Node.js view of one API-admitted diagnostic.
///
/// The projection copies only presentation-safe fields from the API-admitted
/// diagnostic. `canonicalBytes` retains the exact admitted record; JavaScript
/// never parses the error message or recreates diagnostic policy.
#[napi(object, js_name = "Diagnostic")]
pub struct Diagnostic {
    /// Opaque identity of the exact admitted bytes.
    pub identity: String,
    /// Stable machine-readable failure code.
    pub code: String,
    /// Processing stage that established the failure.
    pub stage: DiagnosticStage,
    /// Recovery-relevant failure class.
    pub disposition: DiagnosticDisposition,
    /// Stable severity string.
    pub severity: String,
    /// Concise redaction-safe explanation.
    pub summary: String,
    /// Typed retry posture.
    pub retry: DiagnosticRetry,
    /// Typed external-effect certainty.
    pub effect: DiagnosticEffectCertainty,
    /// Ordered source-chain messages.
    pub causes: Vec<String>,
    /// Ordered contextual explanations.
    pub notes: Vec<String>,
    /// Ordered recovery suggestions.
    pub help: Vec<String>,
    /// Ordered presentation facts.
    pub facts: Vec<DiagnosticFact>,
    /// Exact RFC 8785 `diagnostic.v1` bytes.
    pub canonical_bytes: Buffer,
}

impl Diagnostic {
    pub(crate) fn from_api(value: &DiagnosticV1) -> Self {
        Self {
            identity: value.identity().as_str().to_owned(),
            code: value.code().to_owned(),
            stage: value.stage().into(),
            disposition: value.disposition().into(),
            severity: value.severity().to_string(),
            summary: value.summary().to_owned(),
            retry: value.retry().into(),
            effect: value.effect().into(),
            causes: value.causes().to_vec(),
            notes: value.notes().to_vec(),
            help: value.help().to_vec(),
            facts: value
                .facts()
                .into_iter()
                .map(|fact| DiagnosticFact {
                    key: fact.key().to_owned(),
                    value: fact.value().to_owned(),
                })
                .collect(),
            canonical_bytes: value.canonical_bytes().to_vec().into(),
        }
    }
}

/// Typed unadmitted candidate forwarded to [`DiagnosticProjection`].
#[napi(object, object_to_js = false, js_name = "DiagnosticProjection")]
pub struct DiagnosticProjection {
    /// Stable diagnostic code candidate.
    pub code: String,
    /// Processing stage that established the failure.
    pub stage: DiagnosticStage,
    /// Recovery-relevant failure class.
    pub disposition: DiagnosticDisposition,
    /// Concise diagnostic summary candidate.
    pub summary: String,
    /// Retry posture.
    pub retry: DiagnosticRetry,
    /// Relationship to any external effect.
    pub effect: DiagnosticEffectCertainty,
    /// Ordered cause messages.
    pub causes: Vec<String>,
    /// Causes already omitted by the source-chain owner.
    pub omitted_causes: u32,
    /// Ordered recovery suggestions.
    pub help: Vec<String>,
}

impl DiagnosticProjection {
    /// Consume the host value into the API-owned diagnostic projection.
    fn into_api(self) -> ApiDiagnosticProjection {
        ApiDiagnosticProjection::new(
            self.code,
            self.stage.into(),
            self.disposition.into(),
            self.summary,
            self.retry.into(),
            self.effect.into(),
        )
        .with_causes(self.causes, self.omitted_causes)
        .with_help(self.help)
    }
}
