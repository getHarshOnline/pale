//! Owned path-free invocation of current pure native actions.

use pale_application::actions::{
    project_action_failure, project_invocation_failure, ActionFailure, NativeActionInvocationError,
};
use pale_application::diagnostic::CauseProjection;
use pale_core::records::{
    ActionContract, ActionExecution, DiagnosticDispositionV1, DiagnosticRetryV1,
    NativeActionCatalogV1,
};

use crate::error::{ApiError, BoundaryError};
use crate::PaleApi;

/// Maximum strict-JSON action request accepted at the public byte boundary.
const ACTION_INPUT_LIMIT: usize = 16 << 20;

/// Which exact public action ingress the façade is admitting.
#[derive(Clone, Copy)]
enum NativeActionIngress {
    /// A complete tagged `action-input.v1` document.
    Complete,
    /// The payload object for an action whose tag is owned by the registry.
    Payload,
}

/// A compiled action identity could not enter the MCP tool-name grammar.
#[derive(Debug, thiserror::Error)]
#[error("compiled native-action identity has no valid MCP tool projection")]
struct InvalidNativeActionToolProjection;

/// Owned request for one deterministic built-in action.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeActionRequest {
    action_id: String,
    input_bytes: Vec<u8>,
}

impl NativeActionRequest {
    /// Maximum strict-JSON request size accepted by the action façade.
    ///
    /// Process and language leaves may use this value to bound reads before
    /// allocating their complete request. The façade rechecks the bound and
    /// remains the admission authority.
    pub const MAX_BYTES: usize = ACTION_INPUT_LIMIT;

    /// Bind strict-JSON input bytes to one exact compiled action identity.
    ///
    /// Construction preserves the untrusted bytes without parsing them or
    /// resolving `action_id`. [`PaleApi::invoke_native_action`] owns those
    /// admission steps and the input-size ceiling.
    #[must_use]
    pub fn new(action_id: String, input_bytes: Vec<u8>) -> Self {
        Self {
            action_id,
            input_bytes,
        }
    }
}

/// Owned payload-only request for one deterministic built-in action.
///
/// This form is intended for carriers, such as MCP, that already select the
/// action out of band. The façade binds the selected action's exact input tag;
/// a caller cannot submit a different tag through this request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeActionPayloadRequest {
    action_id: String,
    payload_bytes: Vec<u8>,
}

impl NativeActionPayloadRequest {
    /// Maximum raw JSON-object size accepted before the action tag is attached.
    ///
    /// Carriers may apply a narrower full-message bound. The façade rechecks
    /// this payload bound before parsing and registry-owned tag attachment.
    pub const MAX_BYTES: usize = ACTION_INPUT_LIMIT;

    /// Bind one untrusted JSON object to an explicitly selected action.
    ///
    /// Construction performs no parsing or action lookup. Pass the value to
    /// [`PaleApi::invoke_native_action_payload`] for complete admission.
    #[must_use]
    pub fn new(action_id: String, payload_bytes: Vec<u8>) -> Self {
        Self {
            action_id,
            payload_bytes,
        }
    }
}

/// Owned effect-free action contract projected through [`PaleApi`].
///
/// Exact contract and payload-schema bytes let language and protocol leaves
/// generate ergonomic views without exposing application registry types or
/// making those views a second authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeActionDescriptor {
    action_id: String,
    mcp_tool_name: String,
    description: String,
    canonical_contract_bytes: Vec<u8>,
    canonical_payload_schema_bytes: Vec<u8>,
}

impl NativeActionDescriptor {
    /// Exact compiled action identity.
    #[must_use]
    pub fn action_id(&self) -> &str {
        &self.action_id
    }

    /// MCP tool name derived from the admitted [`Self::action_id`].
    ///
    /// The name is safe to publish for this descriptor because construction
    /// follows compiled-registry admission. It does not prove that a repository
    /// selected the action or that an MCP server is active.
    #[must_use]
    pub fn mcp_tool_name(&self) -> &str {
        &self.mcp_tool_name
    }

    /// Registry-authored concise human-facing action description.
    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }

    /// RFC 8785 bytes of the exact [`ActionContract`] owned by the registry.
    #[must_use]
    pub fn canonical_contract_bytes(&self) -> &[u8] {
        &self.canonical_contract_bytes
    }

    /// RFC 8785 bytes of the self-contained JSON Schema for this action's payload.
    #[must_use]
    pub fn canonical_payload_schema_bytes(&self) -> &[u8] {
        &self.canonical_payload_schema_bytes
    }
}

/// Canonical successful execution of one pure native action.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeActionExecution {
    action_id: String,
    request_digest: String,
    schema_sha256: String,
    canonical_bytes: Vec<u8>,
}

#[cfg(feature = "wire-schema")]
impl schemars::JsonSchema for NativeActionExecution {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        <ActionExecution as schemars::JsonSchema>::schema_name()
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        <ActionExecution as schemars::JsonSchema>::schema_id()
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        <ActionExecution as schemars::JsonSchema>::json_schema(generator)
    }
}

impl NativeActionExecution {
    /// Exact compiled action identity that produced this execution.
    #[must_use]
    pub fn action_id(&self) -> &str {
        &self.action_id
    }

    /// Digest of the sealed request executed by the pure registry.
    #[must_use]
    pub fn request_digest(&self) -> &str {
        &self.request_digest
    }

    /// Raw-schema digest carried by the admitted execution record.
    #[must_use]
    pub fn schema_sha256(&self) -> &str {
        &self.schema_sha256
    }

    /// RFC 8785 canonical `action-execution.v1` bytes.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }
}

impl PaleApi {
    /// Return every effect-free built-in action in stable identity order.
    ///
    /// Descriptors are owned projections of the same sealed registry used by
    /// invocation. They establish mechanics and payload shape only; project
    /// selection, route admission, actor authority, and effects remain outside
    /// this façade.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] if a compiled contract or its generated payload
    /// schema cannot be projected canonically. Such a failure is a Pale
    /// application invariant, never evidence that an action is unavailable at
    /// runtime.
    pub fn native_action_descriptors(&self) -> Result<Vec<NativeActionDescriptor>, ApiError> {
        let catalog_value = pale_core::canon::parse_strict(self.actions.canonical_catalog_bytes())
            .map_err(|error| {
                self.descriptor_failure(
                    "",
                    error,
                    "sealed native-action catalog is not strict canonical JSON",
                )
            })?;
        let catalog: NativeActionCatalogV1 =
            serde_json::from_value(catalog_value).map_err(|error| {
                self.descriptor_failure(
                    "",
                    error,
                    "sealed native-action catalog could not be decoded",
                )
            })?;
        catalog
            .contracts
            .iter()
            .map(|contract| self.native_action_descriptor(contract))
            .collect()
    }

    /// Execute one bounded pure built-in action from strict JSON bytes.
    ///
    /// The returned receipt proves only deterministic computation from supplied
    /// input. This method has no filesystem, process, network, clock, model,
    /// provider, installation, activation, or native-effect capability.
    ///
    /// # Examples
    ///
    /// ```
    /// use pale_sdk::{PaleApi, NativeActionRequest};
    ///
    /// let api = PaleApi::new()?;
    /// let request = NativeActionRequest::new(
    ///     "pale.compute.path-inventory".to_string(),
    ///     br#"{"kind":"path_inventory","payload":{"style":"posix_relative","paths":["src/./lib.rs"]}}"#.to_vec(),
    /// );
    /// let execution = api.invoke_native_action(request)?;
    /// assert_eq!(execution.action_id(), "pale.compute.path-inventory");
    /// assert!(!execution.canonical_bytes().is_empty());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] for oversized or malformed input, an unknown action,
    /// a typed action rejection, or a Pale registry or schema invariant.
    pub fn invoke_native_action(
        &self,
        request: NativeActionRequest,
    ) -> Result<NativeActionExecution, ApiError> {
        let NativeActionRequest {
            action_id,
            input_bytes,
        } = request;
        self.invoke_action(action_id, input_bytes, NativeActionIngress::Complete)
    }

    /// Execute one action from a payload object supplied by a carrier.
    ///
    /// The selected action's input tag is resolved from the sealed registry,
    /// so a carrier cannot transpose one action's tag onto another payload.
    /// Like [`Self::invoke_native_action`], this operation is deterministic and
    /// effect-free.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] for oversized or malformed input, a non-object
    /// payload, an unknown action, a typed action rejection, or a Pale
    /// registry/schema invariant.
    pub fn invoke_native_action_payload(
        &self,
        request: NativeActionPayloadRequest,
    ) -> Result<NativeActionExecution, ApiError> {
        let NativeActionPayloadRequest {
            action_id,
            payload_bytes,
        } = request;
        self.invoke_action(action_id, payload_bytes, NativeActionIngress::Payload)
    }

    /// Admit either public ingress form and execute it through one registry path.
    fn invoke_action(
        &self,
        action_id: String,
        input_bytes: Vec<u8>,
        ingress: NativeActionIngress,
    ) -> Result<NativeActionExecution, ApiError> {
        if input_bytes.len() > ACTION_INPUT_LIMIT {
            let error = BoundaryError::InputTooLarge {
                limit: ACTION_INPUT_LIMIT,
            };
            return Err(self.action_failure(
                &action_id,
                error,
                ActionFailure {
                    code: "input_too_large",
                    disposition: DiagnosticDispositionV1::InvalidInput,
                    summary: "native-action input exceeds the bounded API input",
                    retry: DiagnosticRetryV1::AfterCorrection,
                    request_digest: None,
                    schema: None,
                },
            ));
        }
        let value = pale_core::canon::parse_strict(&input_bytes).map_err(|error| {
            self.action_failure(
                &action_id,
                error,
                ActionFailure {
                    code: "invalid_json",
                    disposition: DiagnosticDispositionV1::InvalidInput,
                    summary: "native-action input is not strict JSON",
                    retry: DiagnosticRetryV1::AfterCorrection,
                    request_digest: None,
                    schema: None,
                },
            )
        })?;
        let execution = match ingress {
            NativeActionIngress::Complete => self.actions.execute_value(&action_id, value),
            NativeActionIngress::Payload => {
                let serde_json::Value::Object(payload) = value else {
                    return Err(self.action_failure(
                        &action_id,
                        BoundaryError::PayloadNotObject,
                        ActionFailure {
                            code: "malformed_input",
                            disposition: DiagnosticDispositionV1::InvalidInput,
                            summary: "native-action payload must be a JSON object",
                            retry: DiagnosticRetryV1::AfterCorrection,
                            request_digest: None,
                            schema: None,
                        },
                    ));
                };
                self.actions.execute_payload(&action_id, payload)
            }
        }
        .map_err(|error| self.invocation_failure(&action_id, error))?;
        self.finish_execution(action_id, execution)
    }

    /// Canonicalize an admitted execution into the owned façade result.
    fn finish_execution(
        &self,
        action_id: String,
        execution: ActionExecution,
    ) -> Result<NativeActionExecution, ApiError> {
        let canonical_bytes = pale_core::canon::to_canonical_string(&execution)
            .map(String::into_bytes)
            .map_err(|error| {
                self.action_failure(
                    &action_id,
                    error,
                    ActionFailure {
                        code: "canonicalization_failed",
                        disposition: DiagnosticDispositionV1::InternalFailure,
                        summary: "native-action execution could not be canonicalized",
                        retry: DiagnosticRetryV1::NotIndicated,
                        request_digest: Some(execution.request.request_digest.to_string()),
                        schema: None,
                    },
                )
            })?;
        Ok(NativeActionExecution {
            action_id,
            request_digest: execution.request.request_digest.to_string(),
            schema_sha256: execution.schema_sha256.to_string(),
            canonical_bytes,
        })
    }

    /// Project one sealed contract and its exact payload schema without authority drift.
    fn native_action_descriptor(
        &self,
        contract: &ActionContract,
    ) -> Result<NativeActionDescriptor, ApiError> {
        let action_id = contract.id.clone();
        let mcp_tool_name =
            pale_core::records::native_action_tool_name(&action_id).ok_or_else(|| {
                self.descriptor_failure(
                    &action_id,
                    InvalidNativeActionToolProjection,
                    "compiled native-action identity has no valid MCP tool projection",
                )
            })?;
        let canonical_contract_bytes = pale_core::canon::to_canonical_string(contract)
            .map(String::into_bytes)
            .map_err(|error| {
                self.descriptor_failure(
                    &action_id,
                    error,
                    "native-action contract could not be projected canonically",
                )
            })?;
        let payload_schema = self
            .actions
            .payload_schema(&action_id)
            .map_err(|error| self.invocation_failure(&action_id, error))?;
        let canonical_payload_schema_bytes =
            pale_core::canon::to_canonical_string(&serde_json::Value::Object(payload_schema))
                .map(String::into_bytes)
                .map_err(|error| {
                    self.descriptor_failure(
                        &action_id,
                        error,
                        "native-action payload schema could not be projected canonically",
                    )
                })?;
        Ok(NativeActionDescriptor {
            action_id,
            mcp_tool_name,
            description: contract.description.clone(),
            canonical_contract_bytes,
            canonical_payload_schema_bytes,
        })
    }

    /// Project one descriptor invariant through the owning diagnostic grammar.
    fn descriptor_failure<E>(&self, action_id: &str, error: E, summary: &str) -> ApiError
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        self.action_failure(
            action_id,
            error,
            ActionFailure {
                code: "native_action_descriptor_invariant",
                disposition: DiagnosticDispositionV1::InternalFailure,
                summary,
                retry: DiagnosticRetryV1::NotIndicated,
                request_digest: None,
                schema: None,
            },
        )
    }

    /// Preserve the typed invocation cause beside the shared action diagnostic.
    fn invocation_failure(&self, action_id: &str, error: NativeActionInvocationError) -> ApiError {
        let diagnostic = project_invocation_failure(&self.diagnostic_identity, action_id, &error)
            .expect("API-owned action projection preserves diagnostic.v1");
        ApiError::from_admitted(error, diagnostic, CauseProjection::Redacted)
    }

    /// Wrap one boundary failure without selecting a second action policy.
    fn action_failure<E>(&self, action_id: &str, error: E, failure: ActionFailure<'_>) -> ApiError
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        let diagnostic =
            project_action_failure(&self.diagnostic_identity, action_id, &error, failure)
                .expect("API-owned action projection preserves diagnostic.v1");
        ApiError::from_admitted(error, diagnostic, CauseProjection::Redacted)
    }
}
