// WebAssembly contract-identity projection fragment included by the crate façade.

/// Exact embedded bundle coordinate, with only JavaScript-safe integers.
#[wasm_bindgen(js_name = ContractBundleIdentity)]
pub struct WasmContractBundleIdentity {
    /// RFC 8785 descriptor-manifest digest.
    schema_manifest_sha256: String,
    /// Domain-separated active schema-set digest.
    schema_set_sha256: String,
    /// Embedded schema count; `u32` is exactly representable by JavaScript.
    member_count: u32,
}

#[wasm_bindgen(js_class = ContractBundleIdentity)]
impl WasmContractBundleIdentity {
    /// RFC 8785 descriptor-manifest digest.
    #[wasm_bindgen(getter = schemaManifestSha256)]
    pub fn schema_manifest_sha256(&self) -> String {
        self.schema_manifest_sha256.clone()
    }

    /// Domain-separated active schema-set digest.
    #[wasm_bindgen(getter = schemaSetSha256)]
    pub fn schema_set_sha256(&self) -> String {
        self.schema_set_sha256.clone()
    }

    /// Number of schemas in the embedded bundle.
    #[wasm_bindgen(getter = memberCount)]
    pub fn member_count(&self) -> u32 {
        self.member_count
    }
}

/// One selected generated schema and its exact bytes.
#[wasm_bindgen(js_name = ContractSchema)]
pub struct WasmContractSchema {
    registry_id: String,
    schema_uri: String,
    wire_schema_id: Option<String>,
    schema_sha256: String,
    bytes: Vec<u8>,
}

#[wasm_bindgen(js_class = ContractSchema)]
impl WasmContractSchema {
    /// Descriptor-registry identity used for lookup.
    #[wasm_bindgen(getter = registryId)]
    pub fn registry_id(&self) -> String {
        self.registry_id.clone()
    }

    /// Canonical schema URI.
    #[wasm_bindgen(getter = schemaUri)]
    pub fn schema_uri(&self) -> String {
        self.schema_uri.clone()
    }

    /// Wire identity carried by standalone documents, when one exists.
    #[wasm_bindgen(getter = wireSchemaId)]
    pub fn wire_schema_id(&self) -> Option<String> {
        self.wire_schema_id.clone()
    }

    /// SHA-256 of the exact raw schema bytes.
    #[wasm_bindgen(getter = schemaSha256)]
    pub fn schema_sha256(&self) -> String {
        self.schema_sha256.clone()
    }

    /// Copy the exact raw schema into one owned `Uint8Array`.
    pub fn bytes(&self) -> Box<[u8]> {
        self.bytes.clone().into_boxed_slice()
    }
}

/// Canonical result of structural document validation.
#[wasm_bindgen(js_name = ValidatedContractDocument)]
pub struct WasmValidatedContractDocument {
    schema_id: String,
    schema_sha256: String,
    canonical_bytes: Vec<u8>,
}

#[wasm_bindgen(js_class = ValidatedContractDocument)]
impl WasmValidatedContractDocument {
    /// Admitted descriptor-registry identity.
    #[wasm_bindgen(getter = schemaId)]
    pub fn schema_id(&self) -> String {
        self.schema_id.clone()
    }

    /// SHA-256 of the schema that admitted the document.
    #[wasm_bindgen(getter = schemaSha256)]
    pub fn schema_sha256(&self) -> String {
        self.schema_sha256.clone()
    }

    /// Copy the RFC 8785 document bytes into one owned `Uint8Array`.
    #[wasm_bindgen(getter = canonicalBytes)]
    pub fn canonical_bytes(&self) -> Box<[u8]> {
        self.canonical_bytes.clone().into_boxed_slice()
    }
}
