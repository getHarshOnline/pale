// WebAssembly native-action projection fragment included by the crate façade.

/// One effect-free native-action descriptor owned by the API registry.
#[wasm_bindgen(js_name = NativeActionDescriptor)]
pub struct WasmNativeActionDescriptor {
    inner: NativeActionDescriptor,
}

impl WasmNativeActionDescriptor {
    /// Retain one API-owned descriptor without reparsing contract bytes.
    fn from_api(inner: NativeActionDescriptor) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = NativeActionDescriptor)]
impl WasmNativeActionDescriptor {
    /// Exact compiled action identity.
    #[wasm_bindgen(getter = actionId)]
    pub fn action_id(&self) -> String {
        self.inner.action_id().to_owned()
    }

    /// Registry-owned concise action description.
    #[wasm_bindgen(getter)]
    pub fn description(&self) -> String {
        self.inner.description().to_owned()
    }

    /// Copy the exact canonical action contract into one `Uint8Array`.
    #[wasm_bindgen(js_name = contractBytes)]
    pub fn contract_bytes(&self) -> Box<[u8]> {
        self.inner
            .canonical_contract_bytes()
            .to_vec()
            .into_boxed_slice()
    }

    /// Copy the self-contained payload schema into one `Uint8Array`.
    #[wasm_bindgen(js_name = payloadSchemaBytes)]
    pub fn payload_schema_bytes(&self) -> Box<[u8]> {
        self.inner
            .canonical_payload_schema_bytes()
            .to_vec()
            .into_boxed_slice()
    }
}
