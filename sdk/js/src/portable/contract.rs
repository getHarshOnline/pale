// Node contract-identity projection fragment included by the shallow façade.

/// Exact embedded bundle coordinate.
#[napi(object, object_from_js = false, js_name = "ContractBundleIdentity")]
pub struct ContractBundleIdentity {
    /// RFC 8785 descriptor-manifest digest.
    pub schema_manifest_sha256: String,
    /// Domain-separated active schema-set digest.
    pub schema_set_sha256: String,
    /// Embedded schema count.
    pub member_count: u32,
}

/// One selected generated schema and its exact bytes.
#[napi(object, object_from_js = false, js_name = "ContractSchema")]
pub struct ContractSchema {
    /// Descriptor-registry identity.
    pub registry_id: String,
    /// Canonical schema URI.
    pub schema_uri: String,
    /// Standalone wire identity, when present.
    pub wire_schema_id: Option<String>,
    /// SHA-256 of the exact schema bytes.
    pub schema_sha256: String,
    /// Exact raw schema bytes.
    pub bytes: Buffer,
}

/// Canonical result of structural document validation.
#[napi(object, object_from_js = false, js_name = "ValidatedContractDocument")]
pub struct ValidatedContractDocument {
    /// Admitted registry identity.
    pub schema_id: String,
    /// SHA-256 of the admitting schema.
    pub schema_sha256: String,
    /// RFC 8785 document bytes.
    pub canonical_bytes: Buffer,
}
