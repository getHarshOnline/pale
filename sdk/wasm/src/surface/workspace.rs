// WebAssembly workspace-source projection fragment included by the crate façade.

/// One logical workspace document captured entirely in host memory.
#[wasm_bindgen(js_name = PortableWorkspaceDocument)]
pub struct WasmPortableWorkspaceDocument {
    label: String,
    bytes: Vec<u8>,
}

#[wasm_bindgen(js_class = PortableWorkspaceDocument)]
impl WasmPortableWorkspaceDocument {
    /// Capture a logical relative label and its complete bytes without I/O.
    #[wasm_bindgen(constructor)]
    pub fn new(label: String, bytes: Box<[u8]>) -> Self {
        Self {
            label,
            bytes: bytes.into_vec(),
        }
    }
}

/// Batched source request for portable workspace compilation.
#[wasm_bindgen(js_name = PortableWorkspaceRequest)]
pub struct WasmPortableWorkspaceRequest {
    root_manifest: String,
    documents: Vec<WasmPortableWorkspaceDocument>,
}

#[wasm_bindgen(js_class = PortableWorkspaceRequest)]
impl WasmPortableWorkspaceRequest {
    /// Bind a complete document batch to its logical root manifest.
    ///
    /// Documents are consumed in order. If an element is invalid, earlier
    /// elements remain consumed and are released; later elements are untouched.
    ///
    /// # Errors
    /// Returns a native `Error` for an invalid or already consumed document.
    #[wasm_bindgen(constructor)]
    pub fn new(
        root_manifest: String,
        #[wasm_bindgen(unchecked_param_type = "PortableWorkspaceDocument[]")] documents: Vec<
            JsValue,
        >,
    ) -> Result<Self, JsValue> {
        let documents = documents
            .into_iter()
            .map(|document| {
                consume_exported_argument(
                    document,
                    "documents must contain unconsumed portable workspace documents",
                )
            })
            .collect::<Result<_, _>>()?;
        Ok(Self {
            root_manifest,
            documents,
        })
    }
}

impl WasmPortableWorkspaceRequest {
    /// Consume host-owned values into the API-owned request contract.
    fn into_api(self) -> PortableWorkspaceRequest {
        PortableWorkspaceRequest::new(
            self.root_manifest,
            self.documents
                .into_iter()
                .map(|document| PortableWorkspaceDocument::new(document.label, document.bytes))
                .collect(),
        )
    }
}

/// Opaque admitted portable workspace; dropping it has no semantic effect.
#[wasm_bindgen(js_name = PortableWorkspaceCompilation)]
pub struct WasmPortableWorkspaceCompilation {
    inner: PortableWorkspaceCompilation,
}

impl WasmPortableWorkspaceCompilation {
    /// Wrap the sole API-owned admitted workspace value.
    fn from_api(inner: PortableWorkspaceCompilation) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = PortableWorkspaceCompilation)]
impl WasmPortableWorkspaceCompilation {
    /// Workspace identity declared by the admitted root manifest.
    #[wasm_bindgen(getter = workspaceId)]
    pub fn workspace_id(&self) -> String {
        self.inner.workspace_id().to_owned()
    }

    /// Principal-relative admitted-universe digest.
    #[wasm_bindgen(getter = principalUniverseDigest)]
    pub fn principal_universe_digest(&self) -> String {
        self.inner.principal_universe_digest().to_owned()
    }

    /// Verified semantic-IR digest.
    #[wasm_bindgen(getter = semanticIrDigest)]
    pub fn semantic_ir_digest(&self) -> String {
        self.inner.semantic_ir_digest().to_owned()
    }

    /// Exact owner-proof frontier consumed by admission.
    #[wasm_bindgen(getter = admissionEvidenceDigest)]
    pub fn admission_evidence_digest(&self) -> String {
        self.inner.admission_evidence_digest().to_owned()
    }

    /// Number of admitted source documents.
    #[wasm_bindgen(getter = sourceDocumentCount)]
    pub fn source_document_count(&self) -> u32 {
        self.inner.source_document_count()
    }
    /// Number of derivation sources.
    #[wasm_bindgen(getter = derivationSourceCount)]
    pub fn derivation_source_count(&self) -> u32 {
        self.inner.derivation_source_count()
    }
    /// Number of derivation nodes.
    #[wasm_bindgen(getter = derivationNodeCount)]
    pub fn derivation_node_count(&self) -> u32 {
        self.inner.derivation_node_count()
    }
    /// Number of admission-evidence records.
    #[wasm_bindgen(getter = admissionEvidenceCount)]
    pub fn admission_evidence_count(&self) -> u32 {
        self.inner.admission_evidence_count()
    }
    /// Number of adopted releases.
    #[wasm_bindgen(getter = adoptedReleaseCount)]
    pub fn adopted_release_count(&self) -> u32 {
        self.inner.adopted_release_count()
    }
    /// Number of Package releases.
    #[wasm_bindgen(getter = packageReleaseCount)]
    pub fn package_release_count(&self) -> u32 {
        self.inner.package_release_count()
    }
    /// Number of normative Pack releases.
    #[wasm_bindgen(getter = normativePackReleaseCount)]
    pub fn normative_pack_release_count(&self) -> u32 {
        self.inner.normative_pack_release_count()
    }

    /// Return all bounded cardinalities in one exact JSON object.
    #[wasm_bindgen(js_name = countsJson)]
    pub fn counts_json(&self) -> String {
        format!(
            concat!(
                "{{\"admission_evidence\":{},\"adopted_releases\":{},",
                "\"derivation_nodes\":{},\"derivation_sources\":{},",
                "\"normative_pack_releases\":{},\"package_releases\":{},",
                "\"source_documents\":{}}}"
            ),
            self.inner.admission_evidence_count(),
            self.inner.adopted_release_count(),
            self.inner.derivation_node_count(),
            self.inner.derivation_source_count(),
            self.inner.normative_pack_release_count(),
            self.inner.package_release_count(),
            self.inner.source_document_count(),
        )
    }
}
