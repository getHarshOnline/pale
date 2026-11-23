// WebAssembly diagnostic projection fragment included by the crate façade.

/// WebAssembly representation of API-owned [`DiagnosticStage`].
#[wasm_bindgen(js_name = DiagnosticStage)]
#[derive(Clone, Copy)]
pub enum WasmDiagnosticStage {
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

impl From<WasmDiagnosticStage> for DiagnosticStage {
    fn from(value: WasmDiagnosticStage) -> Self {
        match value {
            WasmDiagnosticStage::InputAdmission => Self::InputAdmission,
            WasmDiagnosticStage::ContractValidation => Self::ContractValidation,
            WasmDiagnosticStage::SemanticAdmission => Self::SemanticAdmission,
            WasmDiagnosticStage::PackageLoading => Self::PackageLoading,
            WasmDiagnosticStage::WorkspaceLoading => Self::WorkspaceLoading,
            WasmDiagnosticStage::Compilation => Self::Compilation,
            WasmDiagnosticStage::Rendering => Self::Rendering,
            WasmDiagnosticStage::OperationAdmission => Self::OperationAdmission,
            WasmDiagnosticStage::ResidentAdmission => Self::ResidentAdmission,
            WasmDiagnosticStage::ResidentRecovery => Self::ResidentRecovery,
            WasmDiagnosticStage::Planning => Self::Planning,
            WasmDiagnosticStage::Materialization => Self::Materialization,
            WasmDiagnosticStage::NativeAction => Self::NativeAction,
            WasmDiagnosticStage::HostObservation => Self::HostObservation,
            WasmDiagnosticStage::Internal => Self::Internal,
        }
    }
}

/// WebAssembly representation of API-owned [`DiagnosticDisposition`].
#[wasm_bindgen(js_name = DiagnosticDisposition)]
#[derive(Clone, Copy)]
pub enum WasmDiagnosticDisposition {
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

impl From<WasmDiagnosticDisposition> for DiagnosticDisposition {
    fn from(value: WasmDiagnosticDisposition) -> Self {
        match value {
            WasmDiagnosticDisposition::OperationalFailure => Self::OperationalFailure,
            WasmDiagnosticDisposition::InvalidInput => Self::InvalidInput,
            WasmDiagnosticDisposition::Blocked => Self::Blocked,
            WasmDiagnosticDisposition::InternalFailure => Self::InternalFailure,
            WasmDiagnosticDisposition::EffectOutcomeUnknown => Self::EffectOutcomeUnknown,
        }
    }
}

/// WebAssembly representation of API-owned [`DiagnosticRetry`].
#[wasm_bindgen(js_name = DiagnosticRetry)]
#[derive(Clone, Copy)]
pub enum WasmDiagnosticRetry {
    /// No retry instruction is established.
    NotIndicated,
    /// Retry after correcting input or authored meaning.
    AfterCorrection,
    /// Retry after admitted state changes.
    AfterStateChange,
    /// Reconcile evidence before deciding whether to retry.
    ReconcileFirst,
}

impl From<WasmDiagnosticRetry> for DiagnosticRetry {
    fn from(value: WasmDiagnosticRetry) -> Self {
        match value {
            WasmDiagnosticRetry::NotIndicated => Self::NotIndicated,
            WasmDiagnosticRetry::AfterCorrection => Self::AfterCorrection,
            WasmDiagnosticRetry::AfterStateChange => Self::AfterStateChange,
            WasmDiagnosticRetry::ReconcileFirst => Self::ReconcileFirst,
        }
    }
}

/// WebAssembly representation of API-owned [`DiagnosticEffectCertainty`].
#[wasm_bindgen(js_name = DiagnosticEffectCertainty)]
#[derive(Clone, Copy)]
pub enum WasmDiagnosticEffectCertainty {
    /// No external-effect boundary exists.
    NotApplicable,
    /// The effect boundary is proven not to have started.
    NotStarted,
    /// Owner evidence confirms the effect and result.
    Confirmed,
    /// Available evidence cannot establish whether the external effect occurred.
    OutcomeUnknown,
}

impl From<WasmDiagnosticEffectCertainty> for DiagnosticEffectCertainty {
    fn from(value: WasmDiagnosticEffectCertainty) -> Self {
        match value {
            WasmDiagnosticEffectCertainty::NotApplicable => Self::NotApplicable,
            WasmDiagnosticEffectCertainty::NotStarted => Self::NotStarted,
            WasmDiagnosticEffectCertainty::Confirmed => Self::Confirmed,
            WasmDiagnosticEffectCertainty::OutcomeUnknown => Self::OutcomeUnknown,
        }
    }
}

/// Typed unadmitted candidate forwarded to [`DiagnosticProjection`].
#[wasm_bindgen(js_name = DiagnosticProjection)]
pub struct WasmDiagnosticProjection {
    inner: DiagnosticProjection,
}

#[wasm_bindgen(js_class = DiagnosticProjection)]
impl WasmDiagnosticProjection {
    /// Start one complete owner classification; API admission remains fallible.
    #[wasm_bindgen(constructor)]
    pub fn new(
        code: String,
        stage: WasmDiagnosticStage,
        disposition: WasmDiagnosticDisposition,
        summary: String,
        retry: WasmDiagnosticRetry,
        effect: WasmDiagnosticEffectCertainty,
    ) -> Self {
        Self {
            inner: DiagnosticProjection::new(
                code,
                stage.into(),
                disposition.into(),
                summary,
                retry.into(),
                effect.into(),
            ),
        }
    }

    /// Attach the principally affected typed identity.
    #[wasm_bindgen(js_name = setSubject)]
    pub fn set_subject(&mut self, kind: String, identity: String) {
        self.inner = self.inner.clone().with_subject(kind, identity);
    }

    /// Attach bounded ordered cause messages and an exact omission count.
    ///
    /// # Errors
    ///
    /// Returns a native `Error` when the omitted count is not a JavaScript
    /// number representing an integer from zero through `u32::MAX`.
    #[wasm_bindgen(js_name = setCauses)]
    pub fn set_causes(
        &mut self,
        causes: Vec<String>,
        #[wasm_bindgen(unchecked_param_type = "number")] omitted: JsValue,
    ) -> Result<(), JsValue> {
        let omitted = javascript_u32(&omitted, u32::MAX, "diagnostic omitted-cause count")?;
        self.inner = self.inner.clone().with_causes(causes, omitted);
        Ok(())
    }

    /// Attach ordered recovery suggestions.
    #[wasm_bindgen(js_name = setHelp)]
    pub fn set_help(&mut self, help: Vec<String>) {
        self.inner = self.inner.clone().with_help(help);
    }
}
