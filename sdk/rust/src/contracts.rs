//! Owned contract lookup and structural-validation DTOs.

use pale_core::records::{
    DiagnosticDispositionV1, DiagnosticEffectCertaintyV1, DiagnosticRetryV1, DiagnosticStageV1,
    DiagnosticSubjectV1Wire, WorkspaceModulePreimageV1, WorkspaceModuleV1,
};

use crate::error::{ApiError, BoundaryError, FailureSpecification};
use crate::PaleApi;

/// Maximum contract document admitted before strict parsing allocates its value tree.
const DOCUMENT_LIMIT: usize = 16 << 20;

/// Whether one generated root belongs to an owned current façade use case.
///
/// The closed match makes deny-by-default selection exhaustive without a
/// parallel ordering invariant. Every other embedded producer remains private
/// even though its bytes contribute to [`ContractBundleIdentity`].
fn facade_contract_selected(schema_id: &str) -> bool {
    matches!(
        schema_id,
        "action-contract.v1"
            | "action-execution.v1"
            | "action-input.v1"
            | "diagnostic.v1"
            | "repository-native-action-catalog.v1"
    )
}

pub(crate) use pale_application::diagnostic::{
    schema_diagnostic_coordinate, schema_violation_projection,
};

/// Exact identity of the complete embedded contract bundle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractBundleIdentity {
    schema_manifest_sha256: String,
    schema_set_sha256: String,
    member_count: u32,
    canonical_bytes: Vec<u8>,
}

impl ContractBundleIdentity {
    /// RFC 8785 digest of the descriptor manifest preimage.
    #[must_use]
    pub fn schema_manifest_sha256(&self) -> &str {
        &self.schema_manifest_sha256
    }

    /// Domain-separated digest of the active raw-schema set.
    #[must_use]
    pub fn schema_set_sha256(&self) -> &str {
        &self.schema_set_sha256
    }

    /// Number of active embedded schema descriptors.
    #[must_use]
    pub fn member_count(&self) -> u32 {
        self.member_count
    }

    /// RFC 8785 JSON projection shared by process and language transports.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }
}

/// Owned exact bytes and identity for one embedded generated JSON Schema.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractSchema {
    registry_id: String,
    schema_uri: String,
    wire_schema_id: Option<String>,
    schema_sha256: String,
    bytes: Vec<u8>,
}

impl ContractSchema {
    /// Descriptor-manifest registry identity used for lookup.
    #[must_use]
    pub fn registry_id(&self) -> &str {
        &self.registry_id
    }

    /// Manifest-owned canonical `$id` URI.
    #[must_use]
    pub fn schema_uri(&self) -> &str {
        &self.schema_uri
    }

    /// Exact top-level wire identity when the contract declares one.
    #[must_use]
    pub fn wire_schema_id(&self) -> Option<&str> {
        self.wire_schema_id.as_deref()
    }

    /// SHA-256 of [`Self::bytes`].
    #[must_use]
    pub fn schema_sha256(&self) -> &str {
        &self.schema_sha256
    }

    /// Exact generated UTF-8 schema bytes captured by the current binary.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Owned request to structurally validate one JSON document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractDocument {
    schema_id: String,
    bytes: Vec<u8>,
}

/// Owned JSON preimage for one effect-free workspace-module candidate.
///
/// The bytes carry only the `module` member of `workspace-module.v1`; the Rust
/// construction kernel supplies the schema coordinate and content digest. This
/// request performs no filesystem, environment, credential, publication, or
/// admission lookup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceModuleCandidate {
    bytes: Vec<u8>,
}

impl WorkspaceModuleCandidate {
    /// Bind one caller-owned JSON preimage to the workspace-module builder.
    ///
    /// Construction deliberately performs no decoding. Pass this value to
    /// [`PaleApi::build_workspace_module`] for strict parsing, typed sealing,
    /// generated-schema validation, and canonical encoding.
    #[must_use]
    pub fn new(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }
}

impl ContractDocument {
    /// Bind untrusted bytes to an explicitly selected registry identity.
    ///
    /// This constructor performs no lookup or validation. Pass the value to
    /// [`PaleApi::validate_contract_document`] to admit the selected schema,
    /// strict JSON grammar, and canonical representation.
    #[must_use]
    pub fn new(schema_id: String, bytes: Vec<u8>) -> Self {
        Self { schema_id, bytes }
    }
}

/// Structurally validated JSON document in canonical byte form.
///
/// This value proves strict JSON, the selected generated shape, any
/// descriptor-declared self-carried identity, and RFC 8785 encoding only. It
/// does not prove the owner-specific meaning represented by the document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedContractDocument {
    schema_id: String,
    schema_sha256: String,
    canonical_bytes: Vec<u8>,
}

impl ValidatedContractDocument {
    /// Descriptor-manifest identity whose shape admitted the document.
    #[must_use]
    pub fn schema_id(&self) -> &str {
        &self.schema_id
    }

    /// SHA-256 of the exact generated schema used for validation.
    #[must_use]
    pub fn schema_sha256(&self) -> &str {
        &self.schema_sha256
    }

    /// RFC 8785 canonical document bytes.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }
}

impl PaleApi {
    /// Return the exact embedded descriptor-manifest and schema-set coordinate.
    ///
    /// # Panics
    ///
    /// Panics only if the three already-validated scalar identity fields cannot
    /// be represented as canonical JSON. Their closed string and integer types
    /// make that state unreachable for a successfully constructed [`PaleApi`].
    #[must_use]
    pub fn contract_bundle_identity(&self) -> ContractBundleIdentity {
        let canonical_bytes = pale_core::canon::to_canonical_string(&serde_json::json!({
            "memberCount": self.schema_count,
            "schemaManifestSha256": self.schema_registry.schema_manifest_sha256,
            "schemaSetSha256": self.schema_registry.schema_set_sha256,
        }))
        .expect("contract bundle identity is canonical JSON")
        .into_bytes();
        ContractBundleIdentity {
            schema_manifest_sha256: self.schema_registry.schema_manifest_sha256.to_string(),
            schema_set_sha256: self.schema_registry.schema_set_sha256.to_string(),
            member_count: self.schema_count,
            canonical_bytes,
        }
    }

    /// Copy one exact generated schema out of the embedded bundle.
    ///
    /// # Examples
    ///
    /// ```
    /// use pale_sdk::PaleApi;
    ///
    /// let api = PaleApi::new()?;
    /// let schema = api.contract_schema("diagnostic.v1")?;
    /// assert_eq!(schema.wire_schema_id(), Some("pale/diagnostic.v1"));
    /// assert_eq!(schema.schema_sha256().len(), 64);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] for an unknown registry identity or an embedded
    /// registry contradiction. Known contracts outside the explicit façade
    /// allowlist are indistinguishable from unavailable identities. No
    /// filesystem or network lookup is attempted.
    pub fn contract_schema(&self, schema_id: &str) -> Result<ContractSchema, ApiError> {
        self.require_facade_contract(schema_id)?;
        let descriptor = pale_contracts::schema_descriptor(schema_id)
            .map_err(|error| self.contract_failure(schema_id, error))?;
        let bytes = pale_contracts::embedded_schema(schema_id)
            .map_err(|error| self.contract_failure(schema_id, error))?
            .as_bytes()
            .to_vec();
        Ok(ContractSchema {
            registry_id: descriptor.schema_id.to_string(),
            schema_uri: descriptor.schema_uri.to_string(),
            wire_schema_id: descriptor.wire_schema_id.map(str::to_owned),
            schema_sha256: descriptor.schema_sha256.to_string(),
            bytes,
        })
    }

    /// Validate strict JSON against one embedded schema and return canonical bytes.
    ///
    /// This is structural validation only. A successful result does not claim
    /// owner-specific semantic admission, authority, activation, or effect.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when the request exceeds the bounded ingress size,
    /// violates strict JSON or the selected generated schema, names an unknown
    /// schema, or cannot satisfy the canonical record grammar.
    pub fn validate_contract_document(
        &self,
        document: ContractDocument,
    ) -> Result<ValidatedContractDocument, ApiError> {
        let ContractDocument { schema_id, bytes } = document;
        self.require_facade_contract(&schema_id)?;
        if bytes.len() > DOCUMENT_LIMIT {
            let error = BoundaryError::InputTooLarge {
                limit: DOCUMENT_LIMIT,
            };
            return Err(ApiError::new(
                &self.diagnostic_identity,
                error,
                FailureSpecification {
                    code: "input_too_large",
                    stage: DiagnosticStageV1::InputAdmission,
                    disposition: DiagnosticDispositionV1::InvalidInput,
                    summary: "contract document exceeds the bounded API input",
                    subject: schema_subject(&schema_id),
                    schema_coordinate: None,
                    location: None,
                    retry: DiagnosticRetryV1::AfterCorrection,
                    effect: DiagnosticEffectCertaintyV1::NotApplicable,
                    request: None,
                    notes: Vec::new(),
                    help: Vec::new(),
                    facts: Vec::new(),
                },
            ));
        }
        let value = pale_core::canon::parse_strict(&bytes).map_err(|error| {
            ApiError::new(
                &self.diagnostic_identity,
                error,
                FailureSpecification {
                    code: "invalid_json",
                    stage: DiagnosticStageV1::InputAdmission,
                    disposition: DiagnosticDispositionV1::InvalidInput,
                    summary: "contract document is not strict JSON",
                    subject: schema_subject(&schema_id),
                    schema_coordinate: None,
                    location: None,
                    retry: DiagnosticRetryV1::AfterCorrection,
                    effect: DiagnosticEffectCertaintyV1::NotApplicable,
                    request: None,
                    notes: Vec::new(),
                    help: Vec::new(),
                    facts: Vec::new(),
                },
            )
        })?;
        self.validate_value(schema_id, &value)
    }

    /// Seal one pure workspace-module preimage and return its canonical record.
    ///
    /// The same Rust record that owns `workspace-module.v1` sorting, uniqueness,
    /// bounds, and digest construction performs this transition. The generated
    /// schema then validates the sealed value before canonical bytes leave the
    /// façade. Language builders therefore provide ergonomics only: they never
    /// own the digest or RFC 8785 implementation.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when the input exceeds the bounded document limit,
    /// is not strict JSON, cannot decode as the exact workspace-module preimage,
    /// violates the record's construction invariants, fails generated-schema
    /// validation, or cannot be canonically encoded.
    pub fn build_workspace_module(
        &self,
        candidate: WorkspaceModuleCandidate,
    ) -> Result<ValidatedContractDocument, ApiError> {
        const SCHEMA_ID: &str = "workspace-module.v1";

        if candidate.bytes.len() > DOCUMENT_LIMIT {
            return Err(self.module_candidate_failure(
                BoundaryError::InputTooLarge {
                    limit: DOCUMENT_LIMIT,
                },
                "workspace module candidate exceeds the bounded API input",
            ));
        }
        let value = pale_core::canon::parse_strict(&candidate.bytes).map_err(|error| {
            self.module_candidate_failure(error, "workspace module candidate is not strict JSON")
        })?;
        let preimage =
            serde_json::from_value::<WorkspaceModulePreimageV1>(value).map_err(|error| {
                self.module_candidate_failure(
                    error,
                    "workspace module candidate does not match the typed preimage",
                )
            })?;
        let module = WorkspaceModuleV1::seal(preimage).map_err(|error| {
            self.module_candidate_failure(
                error,
                "workspace module candidate violates construction invariants",
            )
        })?;
        let value = serde_json::to_value(module).map_err(|error| {
            ApiError::new(
                &self.diagnostic_identity,
                error,
                FailureSpecification {
                    code: "workspace_module_projection_failed",
                    stage: DiagnosticStageV1::Internal,
                    disposition: DiagnosticDispositionV1::InternalFailure,
                    summary: "sealed workspace module could not be projected",
                    subject: schema_subject(SCHEMA_ID),
                    schema_coordinate: None,
                    location: None,
                    retry: DiagnosticRetryV1::NotIndicated,
                    effect: DiagnosticEffectCertaintyV1::NotApplicable,
                    request: None,
                    notes: Vec::new(),
                    help: Vec::new(),
                    facts: Vec::new(),
                },
            )
        })?;
        self.validate_value(SCHEMA_ID.to_string(), &value)
    }

    /// Validate and canonically encode one already parsed JSON value.
    fn validate_value(
        &self,
        schema_id: String,
        value: &serde_json::Value,
    ) -> Result<ValidatedContractDocument, ApiError> {
        pale_contracts::validate_value(value, &schema_id)
            .map_err(|error| self.contract_failure(&schema_id, error))?;
        let descriptor = pale_contracts::schema_descriptor(&schema_id)
            .map_err(|error| self.contract_failure(&schema_id, error))?;
        let canonical_bytes = pale_core::canon::to_canonical_string(value)
            .map(String::into_bytes)
            .map_err(|error| {
                ApiError::new(
                    &self.diagnostic_identity,
                    error,
                    FailureSpecification {
                        code: "canonicalization_failed",
                        stage: DiagnosticStageV1::Internal,
                        disposition: DiagnosticDispositionV1::InternalFailure,
                        summary: "validated contract document could not be canonicalized",
                        subject: schema_subject(&schema_id),
                        schema_coordinate: None,
                        location: None,
                        retry: DiagnosticRetryV1::NotIndicated,
                        effect: DiagnosticEffectCertaintyV1::NotApplicable,
                        request: None,
                        notes: Vec::new(),
                        help: Vec::new(),
                        facts: Vec::new(),
                    },
                )
            })?;
        Ok(ValidatedContractDocument {
            schema_id,
            schema_sha256: descriptor.schema_sha256.to_string(),
            canonical_bytes,
        })
    }

    /// Project one typed module-construction failure without retaining input bytes.
    fn module_candidate_failure<E>(&self, error: E, summary: &'static str) -> ApiError
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        ApiError::new_with_opaque_source(
            &self.diagnostic_identity,
            error,
            FailureSpecification {
                code: "workspace_module_candidate_rejected",
                stage: DiagnosticStageV1::InputAdmission,
                disposition: DiagnosticDispositionV1::InvalidInput,
                summary,
                subject: schema_subject("workspace-module.v1"),
                schema_coordinate: None,
                location: None,
                retry: DiagnosticRetryV1::AfterCorrection,
                effect: DiagnosticEffectCertaintyV1::NotApplicable,
                request: None,
                notes: Vec::new(),
                help: Vec::new(),
                facts: Vec::new(),
            },
        )
    }

    /// Enforce deny-by-default contract exposure independently of manifest custody.
    fn require_facade_contract(&self, schema_id: &str) -> Result<(), ApiError> {
        if facade_contract_selected(schema_id) {
            return Ok(());
        }
        Err(ApiError::new(
            &self.diagnostic_identity,
            BoundaryError::ContractNotExposed,
            FailureSpecification {
                code: "contract_rejected",
                stage: DiagnosticStageV1::ContractValidation,
                disposition: DiagnosticDispositionV1::InvalidInput,
                summary: "requested contract is unavailable from this API",
                subject: schema_subject(schema_id),
                schema_coordinate: None,
                location: None,
                retry: DiagnosticRetryV1::AfterCorrection,
                effect: DiagnosticEffectCertaintyV1::NotApplicable,
                request: None,
                notes: Vec::new(),
                help: Vec::new(),
                facts: Vec::new(),
            },
        ))
    }

    /// Preserve the registry's typed failure while projecting one portable diagnostic.
    pub(crate) fn contract_failure(
        &self,
        schema_id: &str,
        error: pale_contracts::SchemaError,
    ) -> ApiError {
        let application_disposition = pale_application::embedded_schema_failure_disposition(&error);
        let (disposition, retry) =
            crate::error::application_diagnostic_policy(application_disposition);
        let (code, stage, summary) = match application_disposition {
            pale_application::ApplicationFailureDisposition::InvalidInput => (
                "contract_rejected",
                DiagnosticStageV1::ContractValidation,
                "document does not satisfy the selected contract",
            ),
            pale_application::ApplicationFailureDisposition::Internal => (
                "contract_registry_failure",
                DiagnosticStageV1::Internal,
                "embedded contract registry could not validate the document",
            ),
            pale_application::ApplicationFailureDisposition::Operational
            | pale_application::ApplicationFailureDisposition::Blocked => {
                unreachable!("embedded schema admission has only authored or internal failure")
            }
        };
        let projection = schema_violation_projection(&error);
        let schema_coordinate = schema_diagnostic_coordinate(schema_id, projection.schema_path);
        ApiError::new(
            &self.diagnostic_identity,
            error,
            FailureSpecification {
                code,
                stage,
                disposition,
                summary,
                subject: schema_subject(schema_id),
                schema_coordinate,
                location: projection.location,
                retry,
                effect: DiagnosticEffectCertaintyV1::NotApplicable,
                request: None,
                notes: Vec::new(),
                help: Vec::new(),
                facts: projection.facts,
            },
        )
    }
}

/// Build a bounded schema subject when the caller supplied a nonempty identity.
fn schema_subject(schema_id: &str) -> Option<DiagnosticSubjectV1Wire> {
    (!schema_id.is_empty()).then(|| DiagnosticSubjectV1Wire {
        kind: "schema".to_string(),
        id: schema_id.to_string(),
    })
}
