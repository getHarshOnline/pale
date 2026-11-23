// WebAssembly façade-method fragment included by the crate projection owner.
#[wasm_bindgen]
extern "C" {
    /// Construct the host's native `Error` object.
    #[wasm_bindgen(catch, js_name = Error)]
    fn javascript_error(message: &str) -> Result<JsValue, JsValue>;

    /// Parse the already admitted canonical diagnostic for native inspection.
    #[wasm_bindgen(catch, js_namespace = JSON, js_name = parse)]
    fn parse_javascript_json(value: &str) -> Result<JsValue, JsValue>;

    /// Attach one presentation property to a JavaScript object.
    #[wasm_bindgen(catch, js_namespace = Reflect, js_name = set)]
    fn set_javascript_property(
        target: &JsValue,
        key: &JsValue,
        value: &JsValue,
    ) -> Result<bool, JsValue>;

}

/// Consume an exported handle after entering a fallible Rust method.
///
/// Generated argument conversion can throw after borrowing the receiver and
/// skip its destructor. This conversion returns normally on failure. It is
/// only for arguments that the public method already consumes, not borrows.
/// The pinned internal trait requires actual JavaScript ownership/release tests
/// when wasm-bindgen changes.
fn consume_exported_argument<T: wasm_bindgen::convert::TryFromJsValue>(
    value: JsValue,
    message: &'static str,
) -> Result<T, JsValue> {
    T::try_from_js_value(value)
        .map_err(|_| javascript_error(message).unwrap_or_else(std::convert::identity))
}

/// Admit one exact unsigned JavaScript integer before wasm-bindgen can coerce it.
fn javascript_u32(
    value: &JsValue,
    maximum: u32,
    label: &'static str,
) -> Result<u32, JsValue> {
    value
        .as_f64()
        .and_then(|number| exact_javascript_u32(number, maximum))
        .ok_or_else(|| {
            javascript_error(&format!(
                "{label} must be an integer from 0 through {maximum}"
            ))
            .unwrap_or_else(std::convert::identity)
        })
}

/// Preserve a JavaScript number until its integer and range laws are proved.
fn exact_javascript_u32(number: f64, maximum: u32) -> Option<u32> {
    (number.is_finite()
        && number.fract() == 0.0
        && (0.0..=f64::from(maximum)).contains(&number))
    .then_some(number as u32)
}

/// Convert one API failure without replacing its portable error ontology.
///
/// The native `Error.message` is the concise canonical summary. The complete
/// admitted record is exposed separately as `error.diagnostic`, while
/// `error.diagnosticJson` retains the exact RFC 8785 text.
/// `authoringCorrections` contains compiler-proved proposals; `authoringLocation`
/// identifies the rejected source value when available and is otherwise undefined.
/// Neither property adds information to the canonical diagnostic bytes.
/// If a host call fails, return that host failure, not a partial API error.
fn diagnostic_error(error: ApiError) -> JsValue {
    let diagnostic = error.diagnostic();
    let project = || {
        let projected = javascript_error(diagnostic.summary())?;
        let structured = parse_javascript_json(diagnostic.canonical_json())?;
        let canonical = JsValue::from_str(diagnostic.canonical_json());
        let corrections: Vec<_> = error
            .authoring_corrections()
            .iter()
            .map(WasmAuthoringCorrection::from_api)
            .collect();
        let location = error
            .authoring_location()
            .map(WasmAuthoringSourceLocation::from_api);
        for (name, value) in [
            ("diagnostic", structured),
            ("diagnosticJson", canonical),
            ("authoringCorrections", JsValue::from(corrections)),
            ("authoringLocation", JsValue::from(location)),
        ] {
            if !set_javascript_property(&projected, &JsValue::from_str(name), &value)? {
                return Err(JsValue::from_str(
                    "Pale could not attach diagnostic data to the host error",
                ));
            }
        }
        Ok(projected)
    };
    project().unwrap_or_else(std::convert::identity)
}

/// Convert an admitted diagnostic into one owned `Uint8Array` boundary value.
fn diagnostic_bytes(diagnostic: DiagnosticV1) -> Box<[u8]> {
    diagnostic.canonical_bytes().to_vec().into_boxed_slice()
}

/// Return the API-owned WASM eligibility set in stable inventory order.
fn eligible_capability_ids() -> Vec<&'static str> {
    capability_inventory()
        .iter()
        .filter(|capability| capability.eligible_for(ApiLeaf::Wasm))
        .map(|capability| capability.id().as_str())
        .collect()
}

/// Pretty-print the fixed capability-ID grammar without a second JSON library.
fn capability_ids_json() -> String {
    let ids = eligible_capability_ids();
    let body = ids
        .iter()
        .map(|id| format!("\"{id}\""))
        .collect::<Vec<_>>()
        .join(",\n  ");
    if body.is_empty() {
        "[]".to_owned()
    } else {
        format!("[\n  {body}\n]")
    }
}

/// Portable effect-free Pale façade for browser, editor, and plugin-UI hosts.
#[wasm_bindgen(js_name = Pale)]
pub struct PaleWasm {
    api: PaleApi,
}

#[wasm_bindgen(js_class = Pale)]
impl PaleWasm {
    /// Verify embedded contracts and construct an effect-free façade.
    ///
    /// # Errors
    ///
    /// Returns the stable initialization code and message when the embedded
    /// contract bundle contradicts the compiled façade.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<PaleWasm, JsValue> {
        PaleApi::new()
            .map(|api| Self { api })
            .map_err(|error| JsValue::from_str(&format!("{}: {error}", error.code())))
    }

    /// List the API-owned WASM capabilities as pretty-printed JSON.
    #[wasm_bindgen(js_name = capabilityIdsJson)]
    pub fn capability_ids_json(&self) -> String {
        capability_ids_json()
    }

    /// Inspect the complete embedded public-contract bundle coordinate.
    #[wasm_bindgen(js_name = contractBundleIdentity)]
    pub fn contract_bundle_identity(&self) -> WasmContractBundleIdentity {
        let identity = self.api.contract_bundle_identity();
        WasmContractBundleIdentity {
            schema_manifest_sha256: identity.schema_manifest_sha256().to_owned(),
            schema_set_sha256: identity.schema_set_sha256().to_owned(),
            member_count: identity.member_count(),
        }
    }

    /// Look up one façade-selected generated schema by registry identity.
    ///
    /// # Errors
    ///
    /// Returns a native `Error` whose message is the canonical summary and
    /// whose `diagnostic` property carries the complete record.
    #[wasm_bindgen(js_name = contractSchema)]
    pub fn contract_schema(&self, schema_id: &str) -> Result<WasmContractSchema, JsValue> {
        self.api
            .contract_schema(schema_id)
            .map(|schema| WasmContractSchema {
                registry_id: schema.registry_id().to_owned(),
                schema_uri: schema.schema_uri().to_owned(),
                wire_schema_id: schema.wire_schema_id().map(str::to_owned),
                schema_sha256: schema.schema_sha256().to_owned(),
                bytes: schema.bytes().to_vec(),
            })
            .map_err(diagnostic_error)
    }

    /// Structurally validate and canonicalize one selected contract document.
    ///
    /// # Errors
    ///
    /// Returns a native `Error` with separate structured `diagnostic` data when
    /// the schema identity or document fails API-owned admission.
    #[wasm_bindgen(js_name = validateContractDocument)]
    pub fn validate_contract_document(
        &self,
        schema_id: String,
        bytes: Box<[u8]>,
    ) -> Result<WasmValidatedContractDocument, JsValue> {
        self.api
            .validate_contract_document(ContractDocument::new(schema_id, bytes.into_vec()))
            .map(|document| WasmValidatedContractDocument {
                schema_id: document.schema_id().to_owned(),
                schema_sha256: document.schema_sha256().to_owned(),
                canonical_bytes: document.canonical_bytes().to_vec(),
            })
            .map_err(diagnostic_error)
    }

    /// Admit exact canonical `diagnostic.v1` bytes and return the same bytes.
    ///
    /// # Errors
    ///
    /// Returns a native `Error` with separate structured `diagnostic` data when
    /// the input is not the canonical initial-v1 diagnostic contract.
    #[wasm_bindgen(js_name = admitDiagnostic)]
    pub fn admit_diagnostic(&self, bytes: Box<[u8]>) -> Result<Box<[u8]>, JsValue> {
        self.api
            .admit_diagnostic(&bytes)
            .map(diagnostic_bytes)
            .map_err(diagnostic_error)
    }

    /// Consume one typed classification and admit it through the diagnostic owner.
    ///
    /// # Errors
    ///
    /// Returns a native `Error` with separate structured `diagnostic` data when
    /// the projection cannot be admitted or canonically encoded. An invalid or
    /// already consumed handle returns a native `Error` without a diagnostic.
    #[wasm_bindgen(js_name = projectDiagnostic)]
    pub fn project_diagnostic(
        &self,
        #[wasm_bindgen(unchecked_param_type = "DiagnosticProjection")] projection: JsValue,
    ) -> Result<Box<[u8]>, JsValue> {
        let projection: WasmDiagnosticProjection = consume_exported_argument(
            projection,
            "projection must be an unconsumed diagnostic projection",
        )?;
        self.api
            .project_diagnostic(projection.inner)
            .map(diagnostic_bytes)
            .map_err(diagnostic_error)
    }

    /// Consume and compile one complete path-free workspace source batch.
    ///
    /// # Errors
    ///
    /// Returns a native `Error` with separate structured `diagnostic` data when
    /// source or workspace meaning fails API-owned admission. An invalid or
    /// already consumed request returns a native `Error` without a diagnostic.
    #[wasm_bindgen(js_name = compilePortableWorkspace)]
    pub fn compile_portable_workspace(
        &self,
        #[wasm_bindgen(unchecked_param_type = "PortableWorkspaceRequest")] request: JsValue,
    ) -> Result<WasmPortableWorkspaceCompilation, JsValue> {
        let request: WasmPortableWorkspaceRequest = consume_exported_argument(
            request,
            "request must be an unconsumed portable workspace request",
        )?;
        self.api
            .compile_portable_workspace(request.into_api())
            .map(WasmPortableWorkspaceCompilation::from_api)
            .map_err(diagnostic_error)
    }

    /// Compare two admitted path-free workspace handles.
    ///
    /// # Errors
    ///
    /// Returns a native `Error` with separate structured `diagnostic` data when
    /// the workspaces cannot form one valid comparison coordinate.
    #[wasm_bindgen(js_name = comparePortableWorkspaces)]
    pub fn compare_portable_workspaces(
        &self,
        prior: &WasmPortableWorkspaceCompilation,
        current: &WasmPortableWorkspaceCompilation,
    ) -> Result<WasmPortableWorkspaceComparison, JsValue> {
        self.api
            .compare_portable_workspaces(&prior.inner, &current.inner)
            .map(|inner| WasmPortableWorkspaceComparison { inner })
            .map_err(diagnostic_error)
    }

    /// Return the complete effect-free native-action catalog as one batch.
    ///
    /// # Errors
    ///
    /// Returns a native `Error` with separate structured `diagnostic` data if
    /// the compiled registry contradicts its action contracts.
    #[wasm_bindgen(js_name = nativeActionCatalog)]
    pub fn native_action_catalog(&self) -> Result<Vec<WasmNativeActionDescriptor>, JsValue> {
        self.api
            .native_action_descriptors()
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(WasmNativeActionDescriptor::from_api)
                    .collect()
            })
            .map_err(diagnostic_error)
    }

    /// Execute one complete tagged effect-free native-action request.
    ///
    /// # Errors
    ///
    /// Returns a native `Error` with separate structured `diagnostic` data when
    /// the tagged request fails admission or pure execution.
    #[wasm_bindgen(js_name = invokeNativeAction)]
    pub fn invoke_native_action(
        &self,
        action_id: String,
        input_bytes: Box<[u8]>,
    ) -> Result<Box<[u8]>, JsValue> {
        self.api
            .invoke_native_action(NativeActionRequest::new(action_id, input_bytes.into_vec()))
            .map(|execution| execution.canonical_bytes().to_vec().into_boxed_slice())
            .map_err(diagnostic_error)
    }

    /// Execute one carrier-selected effect-free native-action payload.
    ///
    /// # Errors
    ///
    /// Returns a native `Error` with separate structured `diagnostic` data when
    /// the action, payload, or pure execution fails API-owned admission.
    #[wasm_bindgen(js_name = invokeNativeActionPayload)]
    pub fn invoke_native_action_payload(
        &self,
        action_id: String,
        payload_bytes: Box<[u8]>,
    ) -> Result<Box<[u8]>, JsValue> {
        self.api
            .invoke_native_action_payload(NativeActionPayloadRequest::new(
                action_id,
                payload_bytes.into_vec(),
            ))
            .map(|execution| execution.canonical_bytes().to_vec().into_boxed_slice())
            .map_err(diagnostic_error)
    }
}
