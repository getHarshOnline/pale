// Python diagnostic projection fragment included by the shallow façade.

/// Python representation of API-owned [`DiagnosticStage`].
#[pyclass(name = "DiagnosticStage", eq, eq_int, frozen, from_py_object)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyDiagnosticStage {
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
    /// Pure native-action execution.
    NativeAction,
    /// Caller-owned observation admission.
    HostObservation,
    /// Internal invariant failure.
    Internal,
}

impl From<PyDiagnosticStage> for DiagnosticStage {
    fn from(value: PyDiagnosticStage) -> Self {
        match value {
            PyDiagnosticStage::InputAdmission => Self::InputAdmission,
            PyDiagnosticStage::ContractValidation => Self::ContractValidation,
            PyDiagnosticStage::SemanticAdmission => Self::SemanticAdmission,
            PyDiagnosticStage::PackageLoading => Self::PackageLoading,
            PyDiagnosticStage::WorkspaceLoading => Self::WorkspaceLoading,
            PyDiagnosticStage::Compilation => Self::Compilation,
            PyDiagnosticStage::Rendering => Self::Rendering,
            PyDiagnosticStage::OperationAdmission => Self::OperationAdmission,
            PyDiagnosticStage::ResidentAdmission => Self::ResidentAdmission,
            PyDiagnosticStage::ResidentRecovery => Self::ResidentRecovery,
            PyDiagnosticStage::Planning => Self::Planning,
            PyDiagnosticStage::Materialization => Self::Materialization,
            PyDiagnosticStage::NativeAction => Self::NativeAction,
            PyDiagnosticStage::HostObservation => Self::HostObservation,
            PyDiagnosticStage::Internal => Self::Internal,
        }
    }
}

impl From<DiagnosticStage> for PyDiagnosticStage {
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

/// Python representation of API-owned [`DiagnosticDisposition`].
#[pyclass(name = "DiagnosticDisposition", eq, eq_int, frozen, from_py_object)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyDiagnosticDisposition {
    /// An environmental or operational failure.
    OperationalFailure,
    /// Caller-controlled invalid input.
    InvalidInput,
    /// A valid request blocked on state or authority.
    Blocked,
    /// A Pale application invariant failure.
    InternalFailure,
    /// An external effect may have occurred and requires reconciliation.
    EffectOutcomeUnknown,
}

impl From<PyDiagnosticDisposition> for DiagnosticDisposition {
    fn from(value: PyDiagnosticDisposition) -> Self {
        match value {
            PyDiagnosticDisposition::OperationalFailure => Self::OperationalFailure,
            PyDiagnosticDisposition::InvalidInput => Self::InvalidInput,
            PyDiagnosticDisposition::Blocked => Self::Blocked,
            PyDiagnosticDisposition::InternalFailure => Self::InternalFailure,
            PyDiagnosticDisposition::EffectOutcomeUnknown => Self::EffectOutcomeUnknown,
        }
    }
}

impl From<DiagnosticDisposition> for PyDiagnosticDisposition {
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

/// Python representation of API-owned [`DiagnosticRetry`].
#[pyclass(name = "DiagnosticRetry", eq, eq_int, frozen, from_py_object)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyDiagnosticRetry {
    /// No retry is indicated.
    NotIndicated,
    /// Retry after correcting caller-controlled input.
    AfterCorrection,
    /// Retry after the governing state changes.
    AfterStateChange,
    /// Reconcile retained evidence before deciding whether to retry.
    ReconcileFirst,
}

impl From<PyDiagnosticRetry> for DiagnosticRetry {
    fn from(value: PyDiagnosticRetry) -> Self {
        match value {
            PyDiagnosticRetry::NotIndicated => Self::NotIndicated,
            PyDiagnosticRetry::AfterCorrection => Self::AfterCorrection,
            PyDiagnosticRetry::AfterStateChange => Self::AfterStateChange,
            PyDiagnosticRetry::ReconcileFirst => Self::ReconcileFirst,
        }
    }
}

impl From<DiagnosticRetry> for PyDiagnosticRetry {
    fn from(value: DiagnosticRetry) -> Self {
        match value {
            DiagnosticRetry::NotIndicated => Self::NotIndicated,
            DiagnosticRetry::AfterCorrection => Self::AfterCorrection,
            DiagnosticRetry::AfterStateChange => Self::AfterStateChange,
            DiagnosticRetry::ReconcileFirst => Self::ReconcileFirst,
        }
    }
}

/// Python representation of API-owned [`DiagnosticEffectCertainty`].
#[pyclass(name = "DiagnosticEffectCertainty", eq, eq_int, frozen, from_py_object)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyDiagnosticEffectCertainty {
    /// The operation has no effect boundary.
    NotApplicable,
    /// The selected external-effect boundary is proven not to have started.
    NotStarted,
    /// The effect outcome is confirmed.
    Confirmed,
    /// Available evidence cannot establish whether the external effect occurred.
    OutcomeUnknown,
}

impl From<PyDiagnosticEffectCertainty> for DiagnosticEffectCertainty {
    fn from(value: PyDiagnosticEffectCertainty) -> Self {
        match value {
            PyDiagnosticEffectCertainty::NotApplicable => Self::NotApplicable,
            PyDiagnosticEffectCertainty::NotStarted => Self::NotStarted,
            PyDiagnosticEffectCertainty::Confirmed => Self::Confirmed,
            PyDiagnosticEffectCertainty::OutcomeUnknown => Self::OutcomeUnknown,
        }
    }
}

impl From<DiagnosticEffectCertainty> for PyDiagnosticEffectCertainty {
    fn from(value: DiagnosticEffectCertainty) -> Self {
        match value {
            DiagnosticEffectCertainty::NotApplicable => Self::NotApplicable,
            DiagnosticEffectCertainty::NotStarted => Self::NotStarted,
            DiagnosticEffectCertainty::Confirmed => Self::Confirmed,
            DiagnosticEffectCertainty::OutcomeUnknown => Self::OutcomeUnknown,
        }
    }
}

/// Immutable typed view of one API-admitted diagnostic.
///
/// The view copies only presentation-safe fields already classified by
/// [`DiagnosticV1`]. Its `canonical_bytes` property retains the exact admitted
/// record; Python never reparses prose or recreates diagnostic policy.
#[pyclass(name = "Diagnostic", frozen)]
pub struct PyDiagnostic {
    identity: String,
    code: String,
    stage: PyDiagnosticStage,
    disposition: PyDiagnosticDisposition,
    severity: String,
    summary: String,
    retry: PyDiagnosticRetry,
    effect: PyDiagnosticEffectCertainty,
    causes: Vec<String>,
    notes: Vec<String>,
    help: Vec<String>,
    facts: Vec<(String, String)>,
    canonical_bytes: Vec<u8>,
}

#[pymethods]
impl PyDiagnostic {
    /// Opaque identity of the exact admitted diagnostic bytes.
    #[getter]
    pub fn identity(&self) -> &str {
        &self.identity
    }

    /// Stable machine-readable failure code.
    #[getter]
    pub fn code(&self) -> &str {
        &self.code
    }

    /// Processing stage that established the failure.
    #[getter]
    pub fn stage(&self) -> PyDiagnosticStage {
        self.stage
    }

    /// Recovery-relevant failure class.
    #[getter]
    pub fn disposition(&self) -> PyDiagnosticDisposition {
        self.disposition
    }

    /// Stable severity projection.
    #[getter]
    pub fn severity(&self) -> &str {
        &self.severity
    }

    /// Concise redaction-safe explanation used as the exception message.
    #[getter]
    pub fn summary(&self) -> &str {
        &self.summary
    }

    /// Typed retry posture.
    #[getter]
    pub fn retry(&self) -> PyDiagnosticRetry {
        self.retry
    }

    /// Typed external-effect certainty.
    #[getter]
    pub fn effect(&self) -> PyDiagnosticEffectCertainty {
        self.effect
    }

    /// Ordered redaction-safe source-chain messages.
    #[getter]
    pub fn causes(&self) -> Vec<String> {
        self.causes.clone()
    }

    /// Ordered contextual explanations.
    #[getter]
    pub fn notes(&self) -> Vec<String> {
        self.notes.clone()
    }

    /// Ordered bounded recovery suggestions.
    #[getter]
    pub fn help(&self) -> Vec<String> {
        self.help.clone()
    }

    /// Ordered presentation facts as `(key, value)` pairs.
    #[getter]
    pub fn facts(&self) -> Vec<(String, String)> {
        self.facts.clone()
    }

    /// Exact RFC 8785 `diagnostic.v1` bytes.
    #[getter]
    pub fn canonical_bytes(&self, py: Python<'_>) -> Py<PyBytes> {
        crate::python_bytes(py, &self.canonical_bytes)
    }
}

impl PyDiagnostic {
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
                .map(|fact| (fact.key().to_owned(), fact.value().to_owned()))
                .collect(),
            canonical_bytes: value.canonical_bytes().to_vec(),
        }
    }
}

/// Cohesive typed candidate forwarded to [`DiagnosticProjection`].
#[pyclass(name = "DiagnosticProjection", from_py_object)]
#[derive(Clone)]
pub struct PyDiagnosticProjection {
    code: String,
    stage: PyDiagnosticStage,
    disposition: PyDiagnosticDisposition,
    summary: String,
    retry: PyDiagnosticRetry,
    effect: PyDiagnosticEffectCertainty,
    causes: Vec<String>,
    omitted_causes: u32,
    help: Vec<String>,
}

#[pymethods]
impl PyDiagnosticProjection {
    /// Capture one complete diagnostic candidate without admitting it.
    #[new]
    #[pyo3(signature = (*, code, stage, disposition, summary, retry, effect))]
    pub fn new(
        code: String,
        stage: PyDiagnosticStage,
        disposition: PyDiagnosticDisposition,
        summary: String,
        retry: PyDiagnosticRetry,
        effect: PyDiagnosticEffectCertainty,
    ) -> Self {
        Self {
            code,
            stage,
            disposition,
            summary,
            retry,
            effect,
            causes: Vec::new(),
            omitted_causes: 0,
            help: Vec::new(),
        }
    }

    /// Replace the bounded ordered cause projection.
    pub fn set_causes(&mut self, causes: Vec<String>, omitted_causes: u32) {
        self.causes = causes;
        self.omitted_causes = omitted_causes;
    }

    /// Replace the ordered recovery suggestions.
    pub fn set_help(&mut self, help: Vec<String>) {
        self.help = help;
    }
}

impl PyDiagnosticProjection {
    fn into_api(self) -> DiagnosticProjection {
        DiagnosticProjection::new(
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
