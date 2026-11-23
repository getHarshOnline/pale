//! Neutral v1 responsibility and experience-composition contracts.
//!
//! These records describe already-constituted participants and bounded paths.
//! They do not mint authority, select a provider implementation, prove activation,
//! represent native application state, or settle an effect.

use serde::{Deserialize, Serialize};

use super::{
    ArtifactDigest, EffectClass, ExperienceCompositionPlanDigest, ProviderBindingClosureDigest,
    SchemaDigest, SchemaId,
};

/// Hash one canonical composition body under an explicit semantic domain.
fn hash_composition_identity<T: Serialize + ?Sized>(
    domain: &'static [u8],
    body: &T,
) -> Result<ArtifactDigest, crate::canon::CanonError> {
    let canonical = crate::canon::to_canonical_string(body)?;
    let mut preimage = Vec::with_capacity(domain.len() + canonical.len());
    preimage.extend_from_slice(domain);
    preimage.extend_from_slice(canonical.as_bytes());
    Ok(ArtifactDigest::hash_bytes(&preimage))
}

/// Hash every serialized provider-binding field except its own claimed digest.
fn hash_provider_binding_identity(
    binding: &ProviderBindingV1,
) -> Result<ProviderBindingIdentityV1, crate::canon::CanonError> {
    let mut body = serde_json::to_value(binding).map_err(crate::canon::CanonError::Canonicalize)?;
    let object = body.as_object_mut().ok_or_else(|| {
        crate::canon::CanonError::Canonicalize(<serde_json::Error as serde::ser::Error>::custom(
            "ProviderBindingV1 must serialize as an object",
        ))
    })?;
    object.remove("binding_identity").ok_or_else(|| {
        crate::canon::CanonError::Canonicalize(<serde_json::Error as serde::ser::Error>::custom(
            "ProviderBindingV1 must carry binding_identity",
        ))
    })?;
    Ok(ProviderBindingIdentityV1 {
        provider_binding_sha256: hash_composition_identity(
            b"PALE_PROVIDER_BINDING_IDENTITY_V1\0",
            &body,
        )?,
    })
}

/// JSON Schema for an opaque composition identity.
///
/// The wire family intentionally permits mixed-case owner-native identities;
/// [`SchemaId`] remains separately constrained to its lowercase grammar.
fn composition_id_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::Schema::try_from(serde_json::json!({
        "type": "string",
        "minLength": 1,
        "maxLength": 256,
        "pattern": "^[A-Za-z0-9._/-]+$"
    }))
    .expect("the static composition identity schema is valid")
}

/// JSON Schema for a bounded provider-owned coordinate or explanation.
fn composition_text_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::Schema::try_from(serde_json::json!({
        "type": "string",
        "minLength": 1,
        "maxLength": 1_024
    }))
    .expect("the static composition text schema is valid")
}

/// Lift one schema into an explicitly present nullable wire member.
fn nullable_schema(inner: schemars::Schema) -> schemars::Schema {
    schemars::Schema::try_from(serde_json::json!({
        "oneOf": [inner, { "type": "null" }]
    }))
    .expect("the static nullable schema is valid")
}

/// JSON Schema for a present nullable composition identity.
fn nullable_composition_id_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    nullable_schema(composition_id_schema(generator))
}

/// JSON Schema for a present nullable bounded text coordinate.
fn nullable_composition_text_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    nullable_schema(composition_text_schema(generator))
}

/// JSON Schema for a present nullable communication-actor handle.
fn nullable_communication_actor_handle_schema(
    generator: &mut schemars::SchemaGenerator,
) -> schemars::Schema {
    <Option<CommunicationActorHandleV1> as schemars::JsonSchema>::json_schema(generator)
}

/// JSON Schema for a present nullable participant reference.
fn nullable_participant_ref_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    <Option<ParticipantRefV1> as schemars::JsonSchema>::json_schema(generator)
}

/// Project one graph view while fixing the semantic domain named by its field.
fn fixed_graph_view_schema(
    generator: &mut schemars::SchemaGenerator,
    kind: CompositionGraphKindV1,
) -> schemars::Schema {
    let view = <CompositionGraphViewV1 as schemars::JsonSchema>::json_schema(generator);
    schemars::Schema::try_from(serde_json::json!({
        "allOf": [
            view,
            {
                "type": "object",
                "properties": {
                    "kind": { "const": kind.as_str() }
                }
            }
        ]
    }))
    .expect("the static fixed graph-view schema is valid")
}

macro_rules! fixed_graph_schema {
    ($($(#[$meta:meta])* $name:ident => $kind:ident),+ $(,)?) => {
        $(
            $(#[$meta])*
            fn $name(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
                fixed_graph_view_schema(generator, CompositionGraphKindV1::$kind)
            }
        )+
    };
}

fixed_graph_schema!(
    /// JSON Schema for the provider-compatibility graph field.
    provider_compatibility_graph_schema => ProviderCompatibility,
    /// JSON Schema for the work-delegation graph field.
    delegation_graph_schema => WorkDelegation,
    /// JSON Schema for the UI-embedding graph field.
    ui_embedding_graph_schema => UiEmbedding,
    /// JSON Schema for the plugin-dependency graph field.
    plugin_dependency_graph_schema => PluginDependency,
    /// JSON Schema for the native-application-call graph field.
    native_application_graph_schema => NativeApplicationCall,
    /// JSON Schema for the terminal-leaf graph field.
    terminal_leaf_graph_schema => TerminalLeaf,
    /// JSON Schema for the observation-and-settlement graph field.
    evidence_settlement_graph_schema => ObservationSettlement,
);

/// Registry identity of the standalone compiler-derived composition plan.
pub const EXPERIENCE_COMPOSITION_PLAN_V1_SCHEMA_ID: &str = "experience-composition-plan.v1";
/// Stable wire identity carried by [`ExperienceCompositionPlanV1`].
pub const EXPERIENCE_COMPOSITION_PLAN_V1_SCHEMA: &str = "pale/experience-composition-plan.v1";
/// Wire major of [`ExperienceCompositionPlanV1`].
pub const EXPERIENCE_COMPOSITION_PLAN_V1_SCHEMA_VERSION: u32 = 1;
/// Stable wire identity of one explicit workspace composition source.
pub const WORKSPACE_COMPOSITION_SOURCE_V1_SCHEMA: &str = "pale/workspace-composition-source.v1";
/// Generated-registry identity of [`WorkspaceCompositionSourceV1`].
pub const WORKSPACE_COMPOSITION_SOURCE_V1_SCHEMA_ID: &str = "workspace-composition-source.v1";
/// Initial public wire version of [`WorkspaceCompositionSourceV1`].
pub const WORKSPACE_COMPOSITION_SOURCE_V1_SCHEMA_VERSION: u32 = 1;
/// Registry identity of every nested [`ResponsibilityContractV1`] declaration.
pub const RESPONSIBILITY_CONTRACT_V1_SCHEMA_ID: &str = "responsibility-contract.v1";

/// A separately tagged neutral participant reference.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ParticipantRefV1 {
    /// Contract-facing interface participant.
    Interface {
        /// Stable interface identity.
        #[schemars(schema_with = "composition_id_schema")]
        interface_id: String,
    },
    /// Native application participant.
    Application {
        /// Stable native-application identity.
        #[schemars(schema_with = "composition_id_schema")]
        application_id: String,
    },
    /// Participant encoded by the initial-v1 `engine` wire variant.
    Engine {
        /// Stable identity carried by the `engine` wire variant.
        #[schemars(schema_with = "composition_id_schema")]
        engine_id: String,
    },
    /// Neutral capability participant.
    Capability {
        /// Stable capability identity.
        #[schemars(schema_with = "composition_id_schema")]
        capability_id: String,
    },
    /// Terminal leaf participant.
    TerminalLeaf {
        /// Stable terminal-leaf identity.
        #[schemars(schema_with = "composition_id_schema")]
        terminal_leaf_id: String,
    },
    /// Plugin participant.
    Plugin {
        /// Stable plugin identity.
        #[schemars(schema_with = "composition_id_schema")]
        plugin_id: String,
    },
    /// Skill participant.
    Skill {
        /// Stable skill identity.
        #[schemars(schema_with = "composition_id_schema")]
        skill_id: String,
    },
}

/// Explicit origin of an entry without inferring caller, provider, or owner.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EntryOriginRefV1 {
    /// A user explicitly selected the entry.
    User {
        /// Stable identity of the user-selected entry.
        #[schemars(schema_with = "composition_id_schema")]
        user_entry_id: String,
    },
    /// An admitted interface selected the entry.
    Interface {
        /// Stable identity of the interface-selected entry.
        #[schemars(schema_with = "composition_id_schema")]
        interface_entry_id: String,
    },
    /// A separately constituted participant selected the entry.
    Participant {
        /// Stable identity of the participant-selected entry.
        #[schemars(schema_with = "composition_id_schema")]
        participant_entry_id: String,
    },
}

wire_enum_all!(
    /// Seven semantically separate composition graph kinds.
    CompositionGraphKindV1 {
        /// Reciprocal provider-substitution declarations.
        ProviderCompatibility => "provider_compatibility",
        /// Directional responsibility delegation.
        WorkDelegation => "work_delegation",
        /// Directional interface containment.
        UiEmbedding => "ui_embedding",
        /// Directional plugin dependency.
        PluginDependency => "plugin_dependency",
        /// Directional native-application invocation.
        NativeApplicationCall => "native_application_call",
        /// Directional termination at an explicit leaf.
        TerminalLeaf => "terminal_leaf",
        /// Directional evidence observation and settlement.
        ObservationSettlement => "observation_settlement"
    }
);

impl CompositionGraphKindV1 {
    /// Borrow the exact seven v1 graph kinds in declaration order.
    #[must_use]
    pub const fn supported() -> &'static [Self] {
        Self::declared_variants()
    }
}

wire_enum!(
    /// Declared terminal posture of one sparse experience path.
    TerminationKindV1 {
        /// Work completed under its native owner's contract.
        Completed => "completed",
        /// Work was cancelled before completion.
        Cancelled => "cancelled",
        /// Work terminated with a known failure.
        Failed => "failed",
        /// Work exceeded its admitted time boundary.
        Expired => "expired",
        /// Consequential effect state remains unresolved.
        EffectUnknown => "effect_unknown"
    }
);

wire_enum!(
    /// Declared disposition required before an observation can be settled.
    SettlementDispositionV1 {
        /// The native owner must publish an admissible receipt.
        NativeReceiptRequired => "native_receipt_required",
        /// Native state must be read back after the attempted effect.
        ReadBackRequired => "read_back_required",
        /// The native owner must separately accept the outcome.
        OwnerAcceptanceRequired => "owner_acceptance_required",
        /// No settlement condition has yet been satisfied.
        Unsettled => "unsettled"
    }
);

/// Finite compile-time limits carried by a selected experience path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PathBudgetV1 {
    /// Maximum participant-to-participant edge depth.
    #[schemars(range(min = 1, max = 256))]
    pub max_depth: u32,
    /// Maximum outgoing edges admitted at any visited participant.
    #[schemars(range(min = 1, max = 256))]
    pub max_fan_out: u32,
    /// Declared duration ceiling in milliseconds. A path ceiling bounds the sum
    /// of its selected work-edge ceilings.
    #[schemars(schema_with = "crate::canon::formatted_positive_json_safe_u64_schema")]
    pub max_time_ms: u64,
    /// Declared abstract cost ceiling for the path.
    #[schemars(schema_with = "crate::canon::formatted_positive_json_safe_u64_schema")]
    pub max_cost_units: u64,
    /// Declared abstract resource ceiling for the path.
    #[schemars(schema_with = "crate::canon::formatted_positive_json_safe_u64_schema")]
    pub max_resource_units: u64,
    /// Maximum consequential effects the path may request.
    #[schemars(range(min = 0, max = 65_535))]
    pub max_effect_count: u32,
    /// Maximum bounded recursive composition passes.
    #[schemars(range(min = 1, max = 65_535))]
    pub max_recursion_iterations: u32,
}

macro_rules! distinct_digest_identity {
    ($(#[$meta:meta])* $name:ident, $field:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
        #[serde(deny_unknown_fields)]
        pub struct $name {
            /// Domain-separated digest for this identity coordinate.
            pub $field: ArtifactDigest,
        }
    };
}

distinct_digest_identity!(
    /// Identity of a responsibility path; never a provider-binding identity.
    ResponsibilityPathIdentityV1,
    responsibility_path_sha256
);
distinct_digest_identity!(
    /// Identity of a provider binding; never a responsibility-path identity.
    ///
    /// ```compile_fail
    /// use pale_core::records::{
    ///     ArtifactDigest, ProviderBindingIdentityV1, ResponsibilityPathIdentityV1,
    /// };
    /// let responsibility = ResponsibilityPathIdentityV1 {
    ///     responsibility_path_sha256: ArtifactDigest::hash_bytes(b"responsibility"),
    /// };
    /// let _provider: ProviderBindingIdentityV1 = responsibility;
    /// ```
    ProviderBindingIdentityV1,
    provider_binding_sha256
);
distinct_digest_identity!(
    /// Identity of a deployment/activation coordinate; never an invocation.
    DeploymentActivationIdentityV1,
    deployment_activation_sha256
);
distinct_digest_identity!(
    /// Identity of an invocation coordinate; never a native-effect identity.
    InvocationIdentityV1,
    invocation_sha256
);
distinct_digest_identity!(
    /// Identity of native effect evidence; never settlement.
    NativeEffectIdentityV1,
    native_effect_sha256
);
distinct_digest_identity!(
    /// Identity of evidence settlement; never continuity.
    EvidenceSettlementIdentityV1,
    evidence_settlement_sha256
);
distinct_digest_identity!(
    /// Identity of continuity state; never deployment/activation.
    ContinuityIdentityV1,
    continuity_sha256
);
distinct_digest_identity!(
    /// Identity of one provider-native data generation; never Pale admission.
    NativeDataGenerationIdentityV1,
    native_data_generation_sha256
);
distinct_digest_identity!(
    /// Identity of one Pale admission generation; never native data state.
    PaleAdmissionGenerationIdentityV1,
    pale_admission_generation_sha256
);
distinct_digest_identity!(
    /// Identity of exact compiled content; never a lifecycle generation.
    CompiledContentIdentityV1,
    compiled_content_sha256
);
distinct_digest_identity!(
    /// Identity of the exact source body used to compile content.
    SourceContentIdentityV1,
    source_content_sha256
);

/// Exact identity of one schema member admitted by the composition compiler.
///
/// The registry ID alone is not a content identity. Both members must resolve
/// to the same active schema member before a responsibility extension may be
/// admitted.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct ContractSchemaIdentityV1 {
    /// Registry identity of the schema member.
    pub schema_id: SchemaId,
    /// SHA-256 of the exact raw UTF-8 schema bytes.
    pub schema_sha256: SchemaDigest,
}

/// Neutral responsibility carried by a contract or provider binding.
///
/// Built-in variants keep core policy independent of product names. The
/// [`Self::Extension`] variant admits additional semantics only through an
/// explicit namespace and schema-bound contract.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ResponsibilityV1 {
    /// Human or agent communication.
    Communication,
    /// Organizational planning, delegation, or acceptance.
    OrganizationalWork,
    /// Customer-relationship or other business-owned state.
    CrmBusinessState,
    /// Engineering work performed through its native execution owner.
    EngineeringExecution,
    /// Context or memory resolution without ownership transfer.
    ContextMemory,
    /// Terminal harness or model-backed execution.
    TerminalHarnessExecution,
    /// Publication through a native event or outbox boundary.
    NativeEventPublication,
    /// Native effect execution followed by owner read-back.
    NativeEffectReadBack,
    /// A namespaced responsibility whose schema remains separately governed.
    Extension {
        /// Stable namespace that owns the extension vocabulary.
        #[schemars(schema_with = "composition_id_schema")]
        namespace_id: String,
        /// Stable contract identity inside [`Self::Extension::namespace_id`].
        #[schemars(schema_with = "composition_id_schema")]
        contract_id: String,
        /// Exact admitted schema member that defines the extension contract.
        contract_schema_identity: ContractSchemaIdentityV1,
    },
}

/// Authority and responsibility ceiling of one already-constituted participant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResponsibilityContractV1 {
    /// Stable responsibility-contract identity.
    #[schemars(schema_with = "composition_id_schema")]
    pub id: String,
    /// Participant whose responsibility and authority are bounded.
    pub participant: ParticipantRefV1,
    /// Typed responsibilities already assigned by the semantic owner.
    #[schemars(length(min = 1, max = 256), extend("uniqueItems" = true))]
    pub responsibilities: Vec<ResponsibilityV1>,
    /// Maximum effect classes this contract permits.
    #[schemars(length(max = 6), extend("uniqueItems" = true))]
    pub authority_ceiling: Vec<EffectClass>,
}

/// One positive compatibility declaration inside a participant manifest.
///
/// The compiler proves reciprocity across the complete manifest and derives
/// component closure from this closed positive relation. Authors do not supply
/// a Boolean claiming that their own declaration is monotone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProviderCompatibilityDeclarationV1 {
    /// Stable compatibility-declaration identity.
    #[schemars(schema_with = "composition_id_schema")]
    pub id: String,
    /// Participant represented by this compatibility declaration.
    pub participant: ParticipantRefV1,
    /// Capabilities included in the compatibility relation.
    #[schemars(
        length(max = 256),
        inner(length(min = 1, max = 256), regex(pattern = r"^[A-Za-z0-9._/-]+$")),
        extend("uniqueItems" = true)
    )]
    pub capability_ids: Vec<String>,
    /// Other declarations that must reciprocate this compatibility claim.
    #[schemars(
        length(max = 256),
        inner(length(min = 1, max = 256), regex(pattern = r"^[A-Za-z0-9._/-]+$")),
        extend("uniqueItems" = true)
    )]
    pub compatible_provider_ids: Vec<String>,
}

/// Provider-family identity, distinct from a selected logical instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProviderFamilyRefV1 {
    /// Stable neutral provider-family identity.
    #[schemars(schema_with = "composition_id_schema")]
    pub provider_family_id: String,
}

/// Logical provider instance, distinct from its physical deployment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LogicalProviderInstanceRefV1 {
    /// Stable logical provider-instance identity.
    #[schemars(schema_with = "composition_id_schema")]
    pub logical_provider_instance_id: String,
}

/// Native authority origin retained from the provider-owned system.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NativeAuthorityOriginRefV1 {
    /// Stable native authority-origin identity.
    #[schemars(schema_with = "composition_id_schema")]
    pub native_authority_origin_id: String,
}

/// Exact physical deployment serving a logical provider instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PhysicalDeploymentRefV1 {
    /// Stable physical-deployment identity.
    #[schemars(schema_with = "composition_id_schema")]
    pub physical_deployment_id: String,
}

impl DeploymentActivationIdentityV1 {
    /// Derive the deployment-activation identity bound to one exact deployment.
    ///
    /// This identity does not prove that the deployment is active. It only
    /// prevents a binding from carrying an activation coordinate for a different
    /// physical deployment.
    ///
    /// # Errors
    ///
    /// Returns [`crate::canon::CanonError`] when the deployment coordinate cannot
    /// satisfy the canonical JSON contract.
    pub fn for_deployment(
        deployment: &PhysicalDeploymentRefV1,
    ) -> Result<Self, crate::canon::CanonError> {
        Ok(Self {
            deployment_activation_sha256: hash_composition_identity(
                b"PALE_DEPLOYMENT_ACTIVATION_IDENTITY_V1\0",
                deployment,
            )?,
        })
    }
}

/// Adapter or plugin selected for one provider binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AdapterPluginRefV1 {
    /// Stable adapter/plugin identity.
    #[schemars(schema_with = "composition_id_schema")]
    pub adapter_plugin_id: String,
}

/// Independently named source-to-executable artifact coordinates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProviderArtifactCoordinatesV1 {
    /// Exact source coordinate.
    #[schemars(schema_with = "composition_text_schema")]
    pub source: String,
    /// Exact build coordinate derived from [`Self::source`].
    #[schemars(schema_with = "composition_text_schema")]
    pub build: String,
    /// Exact package coordinate, independent of the build coordinate.
    #[schemars(schema_with = "composition_text_schema")]
    pub package: String,
    /// Exact image coordinate when the deployment uses an image artifact.
    #[schemars(schema_with = "composition_text_schema")]
    pub image: String,
    /// Exact executable coordinate admitted for invocation.
    #[schemars(schema_with = "composition_text_schema")]
    pub executable: String,
}

/// Provider protocol, configuration, and extension coordinates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProviderProtocolCoordinatesV1 {
    /// Exact native or adapter protocol coordinate.
    #[schemars(schema_with = "composition_text_schema")]
    pub protocol: String,
    /// Exact configuration coordinate used with [`Self::protocol`].
    #[schemars(schema_with = "composition_text_schema")]
    pub configuration: String,
    /// Explicit protocol extensions; absence is represented by an empty list.
    #[schemars(
        length(max = 256),
        inner(length(min = 1, max = 256), regex(pattern = r"^[A-Za-z0-9._/-]+$")),
        extend("uniqueItems" = true)
    )]
    pub extensions: Vec<String>,
}

/// Provider-native terminology and record mapping coordinates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProviderMappingCoordinatesV1 {
    /// Terminology mapping contract.
    #[schemars(schema_with = "composition_text_schema")]
    pub terminology: String,
    /// Entity mapping contract.
    #[schemars(schema_with = "composition_text_schema")]
    pub entity: String,
    /// Field mapping contract.
    #[schemars(schema_with = "composition_text_schema")]
    pub field: String,
    /// Event mapping contract.
    #[schemars(schema_with = "composition_text_schema")]
    pub event: String,
}

/// Field-level and native-owner authority coordinates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProviderAuthorityCoordinatesV1 {
    /// Field-authority contract.
    #[schemars(schema_with = "composition_text_schema")]
    pub field: String,
    /// Native-authority contract.
    #[schemars(schema_with = "composition_text_schema")]
    pub native: String,
}

/// Provider lifecycle and failure contracts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProviderLifecycleCoordinatesV1 {
    /// Lifecycle contract for the selected deployment.
    #[schemars(schema_with = "composition_text_schema")]
    pub lifecycle: String,
    /// Failure contract, including bounded unavailable states.
    #[schemars(schema_with = "composition_text_schema")]
    pub failure: String,
}

/// Provider exchange contracts kept distinct by operation direction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProviderExchangeCoordinatesV1 {
    /// Native outbox or publication contract.
    #[schemars(schema_with = "composition_text_schema")]
    pub outbox: String,
    /// Native cursor or frontier contract.
    #[schemars(schema_with = "composition_text_schema")]
    pub cursor: String,
    /// Request exchange contract.
    #[schemars(schema_with = "composition_text_schema")]
    pub request: String,
    /// Result exchange contract.
    #[schemars(schema_with = "composition_text_schema")]
    pub result: String,
    /// Cancellation exchange contract.
    #[schemars(schema_with = "composition_text_schema")]
    pub cancel: String,
    /// Drain exchange contract.
    #[schemars(schema_with = "composition_text_schema")]
    pub drain: String,
}

/// Provider behavior when capability is reduced or removed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProviderBehaviorCoordinatesV1 {
    /// Bounded degraded-mode contract.
    #[schemars(schema_with = "composition_text_schema")]
    pub degraded: String,
    /// Clean removal or replacement contract.
    #[schemars(schema_with = "composition_text_schema")]
    pub removal: String,
}

/// Evidence contract for one exact provider capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapabilityEvidenceCoordinateV1 {
    /// Capability whose evidence is described.
    #[schemars(schema_with = "composition_id_schema")]
    pub capability_id: String,
    /// Contract governing evidence for [`Self::capability_id`].
    #[schemars(schema_with = "composition_id_schema")]
    pub evidence_contract_id: String,
}

/// Provider conformance and evidence coordinates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProviderConformanceCoordinatesV1 {
    /// Whole-binding conformance contract.
    #[schemars(schema_with = "composition_text_schema")]
    pub conformance: String,
    /// General evidence contract that does not replace capability evidence.
    #[schemars(schema_with = "composition_text_schema")]
    pub evidence: String,
    /// Evidence contracts indexed by exact capability.
    #[schemars(length(max = 256))]
    pub per_capability_evidence: Vec<CapabilityEvidenceCoordinateV1>,
}

/// Independently typed provider lifecycle generations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProviderGenerationCoordinatesV1 {
    /// Deployment/activation generation selected for the binding.
    pub deployment_activation_identity: DeploymentActivationIdentityV1,
    /// Provider-native data generation observed at admission.
    pub native_data_generation_identity: NativeDataGenerationIdentityV1,
    /// Pale admission generation; never inferred from native data freshness.
    pub pale_admission_generation_identity: PaleAdmissionGenerationIdentityV1,
}

/// Owner-issued generation references used while sealing a provider binding.
///
/// Deployment activation is omitted because [`ProviderBindingV1::seal`] derives
/// it from the selected [`PhysicalDeploymentRefV1`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProviderGenerationPreimageV1 {
    /// Provider-native data generation observed at admission.
    pub native_data_generation_identity: NativeDataGenerationIdentityV1,
    /// Pale admission generation; never inferred from native data freshness.
    pub pale_admission_generation_identity: PaleAdmissionGenerationIdentityV1,
}

/// Stable provider-native communication actor coordinate.
///
/// After owner-authenticated admission, equality under [`Self::equality_scope`]
/// establishes only the same provider-native actor handle. Raw value equality
/// does not authenticate the handle and never establishes a person, principal,
/// role, membership, session, route, authority, or effect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CommunicationActorHandleV1 {
    /// Provider or identity namespace governing the handle.
    #[schemars(schema_with = "composition_id_schema")]
    pub namespace_id: String,
    /// Canonical opaque value used for equality checks.
    #[schemars(schema_with = "composition_text_schema")]
    pub canonical_opaque_handle: String,
    /// Declared context within which handle equality is meaningful.
    #[schemars(schema_with = "composition_text_schema")]
    pub equality_scope: String,
    /// Optional provider-native display representation.
    #[schemars(required, schema_with = "nullable_composition_text_schema")]
    pub display_form: Option<String>,
    /// Contract for verifying provider-native authorship.
    #[schemars(schema_with = "composition_id_schema")]
    pub native_authorship_verifier_id: String,
    /// Contract mapping native references to this handle.
    #[schemars(schema_with = "composition_id_schema")]
    pub reference_mapping_id: String,
    /// Contract carrying custody and lifecycle evidence.
    #[schemars(schema_with = "composition_id_schema")]
    pub lifecycle_evidence_contract_id: String,
    /// Contract carrying replacement or succession evidence.
    #[schemars(schema_with = "composition_id_schema")]
    pub succession_evidence_contract_id: String,
}

/// Exact provider binding for one participant and responsibility.
///
/// This record names independently governed axes. Presence does not prove
/// readiness, activation, native authority, or successful conformance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProviderBindingV1 {
    /// Stable provider-binding record identity.
    #[schemars(schema_with = "composition_id_schema")]
    pub id: String,
    /// Participant using the bound provider.
    pub participant: ParticipantRefV1,
    /// Exact responsibility served by this binding.
    pub responsibility: ResponsibilityV1,
    /// Responsibility contracts governing the binding.
    #[schemars(
        length(min = 1, max = 256),
        inner(length(min = 1, max = 256), regex(pattern = r"^[A-Za-z0-9._/-]+$")),
        extend("uniqueItems" = true)
    )]
    pub responsibility_contract_ids: Vec<String>,
    /// Schemas governing the responsibility contracts.
    #[schemars(length(min = 1, max = 256), extend("uniqueItems" = true))]
    pub responsibility_schema_identities: Vec<ContractSchemaIdentityV1>,
    /// Stable native actor handle when communication continuity requires one.
    #[schemars(required, schema_with = "nullable_communication_actor_handle_schema")]
    pub communication_actor_handle: Option<CommunicationActorHandleV1>,
    /// Domain-separated identity of the complete binding.
    pub binding_identity: ProviderBindingIdentityV1,
    /// Provider family, separate from logical and physical identity.
    pub provider_family: ProviderFamilyRefV1,
    /// Logical provider instance.
    pub logical_provider_instance: LogicalProviderInstanceRefV1,
    /// Provider-native authority origin.
    pub native_authority_origin: NativeAuthorityOriginRefV1,
    /// Physical deployment selected for this binding.
    pub physical_deployment: PhysicalDeploymentRefV1,
    /// Adapter or plugin mediating the provider contract.
    pub adapter_plugin: AdapterPluginRefV1,
    /// Source, build, package, image, and executable coordinates.
    pub artifact_coordinates: ProviderArtifactCoordinatesV1,
    /// Protocol and configuration coordinates.
    pub protocol_coordinates: ProviderProtocolCoordinatesV1,
    /// Provider-native semantic mapping coordinates.
    pub mapping_coordinates: ProviderMappingCoordinatesV1,
    /// Field and native authority coordinates.
    pub authority_coordinates: ProviderAuthorityCoordinatesV1,
    /// Lifecycle and failure coordinates.
    pub lifecycle_coordinates: ProviderLifecycleCoordinatesV1,
    /// Outbox, cursor, and directional exchange coordinates.
    pub exchange_coordinates: ProviderExchangeCoordinatesV1,
    /// Degraded and removal behavior coordinates.
    pub behavior_coordinates: ProviderBehaviorCoordinatesV1,
    /// Binding-wide and per-capability conformance evidence.
    pub conformance_coordinates: ProviderConformanceCoordinatesV1,
    /// Independently typed activation, native-data, and admission generations.
    pub generation_coordinates: ProviderGenerationCoordinatesV1,
}

/// Exact reference to one provider binding in an admitted composition plan.
///
/// The stable record ID is only a locator. Consumers must compare the
/// domain-separated identity before using the referenced binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProviderBindingRefV1 {
    /// Stable provider-binding record identity.
    #[schemars(schema_with = "composition_id_schema")]
    pub provider_binding_id: String,
    /// Domain-separated identity of the complete provider-binding body.
    pub provider_binding_identity: ProviderBindingIdentityV1,
}

impl ProviderBindingRefV1 {
    /// Compare exact references by stable ID and then canonical binding digest.
    ///
    /// This comparator is intentionally explicit: the digest identity itself
    /// does not claim a domain-wide semantic order.
    #[must_use]
    pub fn canonical_cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.provider_binding_id
            .cmp(&other.provider_binding_id)
            .then_with(|| {
                self.provider_binding_identity
                    .provider_binding_sha256
                    .as_str()
                    .cmp(
                        other
                            .provider_binding_identity
                            .provider_binding_sha256
                            .as_str(),
                    )
            })
    }
}

/// Caller-supplied provider-binding body used to derive both Pale-owned identities.
///
/// Owner-issued native-data and Pale-admission generation references remain
/// inputs. The deployment-activation identity and complete binding identity are
/// derived by [`ProviderBindingV1::seal`] and therefore cannot be supplied as
/// unrelated claims.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProviderBindingPreimageV1 {
    /// Stable provider-binding record identity.
    pub id: String,
    /// Participant using the bound provider.
    pub participant: ParticipantRefV1,
    /// Exact responsibility served by this binding.
    pub responsibility: ResponsibilityV1,
    /// Responsibility contracts governing the binding.
    pub responsibility_contract_ids: Vec<String>,
    /// Exact schemas governing the responsibility contracts.
    pub responsibility_schema_identities: Vec<ContractSchemaIdentityV1>,
    /// Stable native actor handle when communication continuity requires one.
    pub communication_actor_handle: Option<CommunicationActorHandleV1>,
    /// Provider family, separate from logical and physical identity.
    pub provider_family: ProviderFamilyRefV1,
    /// Logical provider instance.
    pub logical_provider_instance: LogicalProviderInstanceRefV1,
    /// Provider-native authority origin.
    pub native_authority_origin: NativeAuthorityOriginRefV1,
    /// Physical deployment selected for this binding.
    pub physical_deployment: PhysicalDeploymentRefV1,
    /// Adapter or plugin mediating the provider contract.
    pub adapter_plugin: AdapterPluginRefV1,
    /// Source, build, package, image, and executable coordinates.
    pub artifact_coordinates: ProviderArtifactCoordinatesV1,
    /// Protocol and configuration coordinates.
    pub protocol_coordinates: ProviderProtocolCoordinatesV1,
    /// Provider-native semantic mapping coordinates.
    pub mapping_coordinates: ProviderMappingCoordinatesV1,
    /// Field and native authority coordinates.
    pub authority_coordinates: ProviderAuthorityCoordinatesV1,
    /// Lifecycle and failure coordinates.
    pub lifecycle_coordinates: ProviderLifecycleCoordinatesV1,
    /// Outbox, cursor, and directional exchange coordinates.
    pub exchange_coordinates: ProviderExchangeCoordinatesV1,
    /// Degraded and removal behavior coordinates.
    pub behavior_coordinates: ProviderBehaviorCoordinatesV1,
    /// Binding-wide and per-capability conformance evidence.
    pub conformance_coordinates: ProviderConformanceCoordinatesV1,
    /// Owner-issued native-data and Pale-admission generation references.
    pub generation_coordinates: ProviderGenerationPreimageV1,
}

impl ProviderBindingV1 {
    /// Derive deployment identity, then seal the complete serialized binding body.
    ///
    /// The binding digest includes the derived deployment identity and every
    /// other serialized field except [`Self::binding_identity`] itself.
    ///
    /// # Errors
    ///
    /// Returns [`crate::canon::CanonError`] when the body cannot satisfy the
    /// canonical JSON contract.
    pub fn seal(preimage: ProviderBindingPreimageV1) -> Result<Self, crate::canon::CanonError> {
        let deployment_activation_identity =
            DeploymentActivationIdentityV1::for_deployment(&preimage.physical_deployment)?;
        let ProviderBindingPreimageV1 {
            id,
            participant,
            responsibility,
            responsibility_contract_ids,
            responsibility_schema_identities,
            communication_actor_handle,
            provider_family,
            logical_provider_instance,
            native_authority_origin,
            physical_deployment,
            adapter_plugin,
            artifact_coordinates,
            protocol_coordinates,
            mapping_coordinates,
            authority_coordinates,
            lifecycle_coordinates,
            exchange_coordinates,
            behavior_coordinates,
            conformance_coordinates,
            generation_coordinates,
        } = preimage;
        let mut binding = Self {
            id,
            participant,
            responsibility,
            responsibility_contract_ids,
            responsibility_schema_identities,
            communication_actor_handle,
            // The placeholder never leaves this function and is excluded from
            // the canonical binding-identity preimage below.
            binding_identity: ProviderBindingIdentityV1 {
                provider_binding_sha256: ArtifactDigest::hash_bytes(&[]),
            },
            provider_family,
            logical_provider_instance,
            native_authority_origin,
            physical_deployment,
            adapter_plugin,
            artifact_coordinates,
            protocol_coordinates,
            mapping_coordinates,
            authority_coordinates,
            lifecycle_coordinates,
            exchange_coordinates,
            behavior_coordinates,
            conformance_coordinates,
            generation_coordinates: ProviderGenerationCoordinatesV1 {
                deployment_activation_identity,
                native_data_generation_identity: generation_coordinates
                    .native_data_generation_identity,
                pale_admission_generation_identity: generation_coordinates
                    .pale_admission_generation_identity,
            },
        };
        binding.binding_identity = hash_provider_binding_identity(&binding)?;
        Ok(binding)
    }

    /// Clone the caller-supplied body for amendment and resealing.
    ///
    /// The returned value omits both Pale-derived identities. Use
    /// [`Self::recompute_identity`] when only verification is required.
    #[must_use]
    pub fn to_preimage(&self) -> ProviderBindingPreimageV1 {
        ProviderBindingPreimageV1 {
            id: self.id.clone(),
            participant: self.participant.clone(),
            responsibility: self.responsibility.clone(),
            responsibility_contract_ids: self.responsibility_contract_ids.clone(),
            responsibility_schema_identities: self.responsibility_schema_identities.clone(),
            communication_actor_handle: self.communication_actor_handle.clone(),
            provider_family: self.provider_family.clone(),
            logical_provider_instance: self.logical_provider_instance.clone(),
            native_authority_origin: self.native_authority_origin.clone(),
            physical_deployment: self.physical_deployment.clone(),
            adapter_plugin: self.adapter_plugin.clone(),
            artifact_coordinates: self.artifact_coordinates.clone(),
            protocol_coordinates: self.protocol_coordinates.clone(),
            mapping_coordinates: self.mapping_coordinates.clone(),
            authority_coordinates: self.authority_coordinates.clone(),
            lifecycle_coordinates: self.lifecycle_coordinates.clone(),
            exchange_coordinates: self.exchange_coordinates.clone(),
            behavior_coordinates: self.behavior_coordinates.clone(),
            conformance_coordinates: self.conformance_coordinates.clone(),
            generation_coordinates: ProviderGenerationPreimageV1 {
                native_data_generation_identity: self
                    .generation_coordinates
                    .native_data_generation_identity
                    .clone(),
                pale_admission_generation_identity: self
                    .generation_coordinates
                    .pale_admission_generation_identity
                    .clone(),
            },
        }
    }

    /// Recompute the domain-separated identity of the complete binding body.
    ///
    /// # Errors
    ///
    /// Returns [`crate::canon::CanonError`] when the body cannot satisfy the
    /// canonical JSON contract.
    pub fn recompute_identity(
        &self,
    ) -> Result<ProviderBindingIdentityV1, crate::canon::CanonError> {
        hash_provider_binding_identity(self)
    }

    /// Recompute the deployment identity required by the selected physical deployment.
    ///
    /// # Errors
    ///
    /// Returns [`crate::canon::CanonError`] when the deployment coordinate cannot
    /// satisfy the canonical JSON contract.
    pub fn recompute_deployment_activation_identity(
        &self,
    ) -> Result<DeploymentActivationIdentityV1, crate::canon::CanonError> {
        DeploymentActivationIdentityV1::for_deployment(&self.physical_deployment)
    }
}

/// Exact participant/provider declarations without provider selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ParticipantProviderManifestV1 {
    /// Stable manifest identity.
    #[schemars(schema_with = "composition_id_schema")]
    pub id: String,
    /// Exact participants declared by the manifest.
    #[schemars(length(max = 1_024), extend("uniqueItems" = true))]
    pub participants: Vec<ParticipantRefV1>,
    /// Candidate providers; presence does not select one.
    #[schemars(length(max = 1_024))]
    pub providers: Vec<ProviderCompatibilityDeclarationV1>,
}

/// One explicit entry surface and its non-inferential origin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EntryContractV1 {
    /// Stable entry-contract identity.
    #[schemars(schema_with = "composition_id_schema")]
    pub id: String,
    /// Participant exposed at the entry boundary.
    pub entry: ParticipantRefV1,
    /// Explicit source of the entry selection.
    pub origin: EntryOriginRefV1,
    /// Responsibility contract governing the entry participant.
    #[schemars(schema_with = "composition_id_schema")]
    pub responsibility_contract_id: String,
}

/// Directional request translation between exact participants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequestTranslationV1 {
    /// Stable request-translation identity.
    #[schemars(schema_with = "composition_id_schema")]
    pub id: String,
    /// Participant whose request contract is translated.
    pub from: ParticipantRefV1,
    /// Participant receiving the translated request.
    pub to: ParticipantRefV1,
    /// Input contract understood by the source participant.
    #[schemars(schema_with = "composition_id_schema")]
    pub input_contract: String,
    /// Output contract understood by the destination participant.
    #[schemars(schema_with = "composition_id_schema")]
    pub output_contract: String,
    /// Whether translation preserves the caller's authority ceiling.
    pub preserves_authority: bool,
}

/// Directional result translation between exact participants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResultTranslationV1 {
    /// Stable result-translation identity.
    #[schemars(schema_with = "composition_id_schema")]
    pub id: String,
    /// Participant whose result contract is translated.
    pub from: ParticipantRefV1,
    /// Participant receiving the translated result.
    pub to: ParticipantRefV1,
    /// Input contract understood by the source participant.
    #[schemars(schema_with = "composition_id_schema")]
    pub input_contract: String,
    /// Output contract understood by the destination participant.
    #[schemars(schema_with = "composition_id_schema")]
    pub output_contract: String,
    /// Whether translation preserves native evidence.
    pub preserves_evidence: bool,
}

/// Explicit terminal contract for one finite selected path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TerminationContractV1 {
    /// Stable termination-contract identity.
    #[schemars(schema_with = "composition_id_schema")]
    pub id: String,
    /// Participant responsible for declaring terminal posture.
    pub owner: ParticipantRefV1,
    /// Terminal postures admitted by the contract.
    #[schemars(length(min = 1, max = 5), extend("uniqueItems" = true))]
    pub kinds: Vec<TerminationKindV1>,
    /// Optional explicit terminal-leaf participant.
    #[schemars(required, schema_with = "nullable_participant_ref_schema")]
    pub terminal_leaf: Option<ParticipantRefV1>,
}

/// Explicit settlement requirement that does not claim settlement occurrence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SettlementContractV1 {
    /// Stable settlement-contract identity.
    #[schemars(schema_with = "composition_id_schema")]
    pub id: String,
    /// Participant observing the evidence boundary.
    pub observer: ParticipantRefV1,
    /// Evidence condition required before settlement.
    pub disposition: SettlementDispositionV1,
    /// Optional native receipt contract required at settlement.
    #[schemars(required, schema_with = "nullable_composition_id_schema")]
    pub native_receipt_contract: Option<String>,
    /// Domain-separated identity of the settlement coordinate.
    pub identity: EvidenceSettlementIdentityV1,
}

/// Explicit continuity obligations owned outside the composition compiler.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ContinuityContractV1 {
    /// Stable continuity-contract identity.
    #[schemars(schema_with = "composition_id_schema")]
    pub id: String,
    /// Participant that owns the declared obligations.
    pub owner: ParticipantRefV1,
    /// Continuity obligations to be implemented by the later resident owner.
    #[schemars(
        length(max = 256),
        inner(length(min = 1, max = 1_024)),
        extend("uniqueItems" = true)
    )]
    pub obligations: Vec<String>,
    /// Domain-separated identity of the continuity coordinate.
    pub identity: ContinuityIdentityV1,
}

/// A typed composition edge whose variant fixes its graph semantics.
///
/// Consumers may inspect only the common [`Self::id`], [`Self::kind`], and
/// [`Self::endpoints`] projections before matching the variant-specific
/// contract. No variant can be relabelled as another graph kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CompositionEdgeV1 {
    /// Directional, authority-bounded work delegation.
    WorkDelegation {
        /// Stable edge identity.
        #[schemars(schema_with = "composition_id_schema")]
        id: String,
        /// Participant delegating bounded work.
        from: ParticipantRefV1,
        /// Participant receiving bounded work.
        to: ParticipantRefV1,
        /// Responsibility contract authorizing the delegation.
        #[schemars(schema_with = "composition_id_schema")]
        responsibility_contract_id: String,
        /// Directional request translation used by the delegation.
        #[schemars(schema_with = "composition_id_schema")]
        request_translation_id: String,
        /// Directional result translation used by the delegation.
        #[schemars(schema_with = "composition_id_schema")]
        result_translation_id: String,
        /// Maximum effect classes delegated across this edge.
        #[schemars(length(max = 6), extend("uniqueItems" = true))]
        authority_ceiling: Vec<EffectClass>,
        /// Finite budget applied to this delegation.
        budget: PathBudgetV1,
    },
    /// UI containment without delegation or native-effect authority.
    UiEmbedding {
        /// Stable edge identity.
        #[schemars(schema_with = "composition_id_schema")]
        id: String,
        /// Participant hosting the embedded surface.
        host: ParticipantRefV1,
        /// Participant whose surface is embedded.
        embedded: ParticipantRefV1,
    },
    /// Directional dependency between two plugins.
    PluginDependency {
        /// Stable edge identity.
        #[schemars(schema_with = "composition_id_schema")]
        id: String,
        /// Plugin that declares the dependency.
        plugin: ParticipantRefV1,
        /// Participant required by [`Self::PluginDependency::plugin`].
        dependency: ParticipantRefV1,
    },
    /// Native application call through one exact provider binding.
    NativeApplicationCall {
        /// Stable edge identity.
        #[schemars(schema_with = "composition_id_schema")]
        id: String,
        /// Participant initiating the native call.
        caller: ParticipantRefV1,
        /// Native application receiving the call.
        application: ParticipantRefV1,
        /// Provider binding used for this call.
        #[schemars(schema_with = "composition_id_schema")]
        provider_binding_id: String,
        /// Directional request translation used by the call.
        #[schemars(schema_with = "composition_id_schema")]
        request_translation_id: String,
        /// Directional result translation used by the call.
        #[schemars(schema_with = "composition_id_schema")]
        result_translation_id: String,
    },
    /// Explicit termination at a finite terminal leaf.
    TerminalLeaf {
        /// Stable edge identity.
        #[schemars(schema_with = "composition_id_schema")]
        id: String,
        /// Participant that owns termination.
        owner: ParticipantRefV1,
        /// Terminal-leaf participant reached by the edge.
        terminal_leaf: ParticipantRefV1,
        /// Contract governing the terminal posture.
        #[schemars(schema_with = "composition_id_schema")]
        termination_contract_id: String,
    },
    /// Directional observation under an explicit settlement contract.
    ObservationSettlement {
        /// Stable edge identity.
        #[schemars(schema_with = "composition_id_schema")]
        id: String,
        /// Participant or state boundary being observed.
        subject: ParticipantRefV1,
        /// Participant observing the evidence boundary.
        observer: ParticipantRefV1,
        /// Contract governing settlement of the observation.
        #[schemars(schema_with = "composition_id_schema")]
        settlement_contract_id: String,
    },
}

impl CompositionEdgeV1 {
    /// Return this edge's stable identity.
    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            Self::WorkDelegation { id, .. }
            | Self::UiEmbedding { id, .. }
            | Self::PluginDependency { id, .. }
            | Self::NativeApplicationCall { id, .. }
            | Self::TerminalLeaf { id, .. }
            | Self::ObservationSettlement { id, .. } => id,
        }
    }

    /// Return the graph kind fixed by this edge's variant.
    #[must_use]
    pub fn kind(&self) -> CompositionGraphKindV1 {
        match self {
            Self::WorkDelegation { .. } => CompositionGraphKindV1::WorkDelegation,
            Self::UiEmbedding { .. } => CompositionGraphKindV1::UiEmbedding,
            Self::PluginDependency { .. } => CompositionGraphKindV1::PluginDependency,
            Self::NativeApplicationCall { .. } => CompositionGraphKindV1::NativeApplicationCall,
            Self::TerminalLeaf { .. } => CompositionGraphKindV1::TerminalLeaf,
            Self::ObservationSettlement { .. } => CompositionGraphKindV1::ObservationSettlement,
        }
    }

    /// Borrow the directional source and destination participants.
    #[must_use]
    pub fn endpoints(&self) -> (&ParticipantRefV1, &ParticipantRefV1) {
        match self {
            Self::WorkDelegation { from, to, .. } => (from, to),
            Self::UiEmbedding { host, embedded, .. } => (host, embedded),
            Self::PluginDependency {
                plugin, dependency, ..
            } => (plugin, dependency),
            Self::NativeApplicationCall {
                caller,
                application,
                ..
            } => (caller, application),
            Self::TerminalLeaf {
                owner,
                terminal_leaf,
                ..
            } => (owner, terminal_leaf),
            Self::ObservationSettlement {
                subject, observer, ..
            } => (subject, observer),
        }
    }
}

/// One edge invocation carried by a sparse experience path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PathInvocationV1 {
    /// Edge invoked by this path step.
    #[schemars(schema_with = "composition_id_schema")]
    pub edge_id: String,
    /// Domain-separated invocation identity.
    pub identity: InvocationIdentityV1,
}

/// One explicit transition through a native owner's contract boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NativeOwnerTransitionV1 {
    /// Edge whose traversal crosses the native-owner boundary.
    #[schemars(schema_with = "composition_id_schema")]
    pub edge_id: String,
    /// Native owner responsible for the transition.
    #[schemars(schema_with = "composition_id_schema")]
    pub native_owner_id: String,
    /// Contract defining the native transition.
    #[schemars(schema_with = "composition_id_schema")]
    pub transition_contract_id: String,
    /// Receipt contract required from the native owner.
    #[schemars(schema_with = "composition_id_schema")]
    pub receipt_contract_id: String,
}

/// Claimed native outcome associated with one edge invocation.
///
/// Read-only results and effectful outcomes are disjoint wire states. An
/// invocation or provider result never acquires a native-effect identity merely
/// because it completed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum NativeOutcomeV1 {
    /// A native result carrying no consequential effect identity.
    ReadOnlyResult {
        /// Edge whose native result is represented.
        #[schemars(schema_with = "composition_id_schema")]
        edge_id: String,
        /// Invocation attempt claimed to have produced this native result.
        invocation_identity: InvocationIdentityV1,
        /// Contract defining the native result record.
        #[schemars(schema_with = "composition_id_schema")]
        outcome_contract_id: String,
    },
    /// A consequential native outcome with an independently typed effect identity.
    EffectfulOutcome {
        /// Edge whose native outcome is represented.
        #[schemars(schema_with = "composition_id_schema")]
        edge_id: String,
        /// Invocation attempt claimed to have produced this native outcome.
        invocation_identity: InvocationIdentityV1,
        /// Domain-separated native-effect identity.
        native_effect_identity: NativeEffectIdentityV1,
        /// Contract defining the native outcome record.
        #[schemars(schema_with = "composition_id_schema")]
        outcome_contract_id: String,
    },
}

impl NativeOutcomeV1 {
    /// Return the edge associated with this outcome.
    #[must_use]
    pub fn edge_id(&self) -> &str {
        match self {
            Self::ReadOnlyResult { edge_id, .. } | Self::EffectfulOutcome { edge_id, .. } => {
                edge_id
            }
        }
    }

    /// Return the exact invocation associated with this outcome.
    #[must_use]
    pub fn invocation_identity(&self) -> &InvocationIdentityV1 {
        match self {
            Self::ReadOnlyResult {
                invocation_identity,
                ..
            }
            | Self::EffectfulOutcome {
                invocation_identity,
                ..
            } => invocation_identity,
        }
    }

    /// Return the native-effect identity only for an effectful outcome.
    #[must_use]
    pub fn native_effect_identity(&self) -> Option<&NativeEffectIdentityV1> {
        match self {
            Self::ReadOnlyResult { .. } => None,
            Self::EffectfulOutcome {
                native_effect_identity,
                ..
            } => Some(native_effect_identity),
        }
    }
}

/// Consequence required at one explicit destination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DestinationConsequenceV1 {
    /// Exact selected participant that receives the consequence.
    pub destination: ParticipantRefV1,
    /// Contract defining the destination consequence.
    #[schemars(schema_with = "composition_id_schema")]
    pub consequence_contract_id: String,
}

/// Reconciliation obligation bound to one exact source frontier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReconciliationRequirementV1 {
    /// Contract defining the reconciliation obligation.
    #[schemars(schema_with = "composition_id_schema")]
    pub obligation_contract_id: String,
    /// Exact frontier against which reconciliation must occur.
    #[schemars(schema_with = "composition_id_schema")]
    pub frontier_id: String,
}

/// Exact compiled-content and source-content identities for one selected path.
///
/// Neither member is a publication, resident, Scope, arena-slot, or provider-index
/// generation. Later compilation stages may resolve these content identities to
/// their owning records without renaming them as lifecycle counters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CompiledContentCoordinateV1 {
    /// Domain-separated identity of the compiled content.
    pub compiled_content_identity: CompiledContentIdentityV1,
    /// Domain-separated identity of the exact source content.
    pub source_content_identity: SourceContentIdentityV1,
}

/// One explicitly selected sparse experience path.
///
/// The path is an allowlist of exact contracts and identities. It does not
/// materialize the cross-product of participants, providers, or edges.
///
/// In the law below, each `Q_i` is the time, cost, resource, and effect budget
/// of selected work-delegation edge `i`; `Q_used` is their componentwise sum.
/// `E_i` is that edge's admitted effect-class ceiling, so `E_admitted` is their
/// set intersection and can only narrow authority.
/// Addition does not make path traversal commutative: the edge order must still
/// form one connected causal path.
#[doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/composition-path-laws.display.mathml"
))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExperiencePathV1 {
    /// Stable experience-path record identity.
    #[schemars(schema_with = "composition_id_schema")]
    pub id: String,
    /// Domain-separated identity of the selected responsibility path.
    pub identity: ResponsibilityPathIdentityV1,
    /// Receipt contract proving the path's explicit origin.
    #[schemars(schema_with = "composition_id_schema")]
    pub origin_receipt_contract_id: String,
    /// Entry contract from which traversal begins.
    #[schemars(schema_with = "composition_id_schema")]
    pub entry_contract_id: String,
    /// Ordered work-delegation edges traversed by the path.
    #[schemars(
        length(max = 256),
        inner(length(min = 1, max = 256), regex(pattern = r"^[A-Za-z0-9._/-]+$")),
        extend("uniqueItems" = true)
    )]
    pub work_delegation_edge_ids: Vec<String>,
    /// Exact responsibility contracts used by the path.
    #[schemars(
        length(max = 256),
        inner(length(min = 1, max = 256), regex(pattern = r"^[A-Za-z0-9._/-]+$")),
        extend("uniqueItems" = true)
    )]
    pub responsibility_contract_ids: Vec<String>,
    /// Exact provider bindings used by the path.
    #[schemars(
        length(max = 256),
        inner(length(min = 1, max = 256), regex(pattern = r"^[A-Za-z0-9._/-]+$")),
        extend("uniqueItems" = true)
    )]
    pub provider_binding_ids: Vec<String>,
    /// Capabilities admitted for this path.
    #[schemars(
        length(max = 256),
        inner(length(min = 1, max = 256), regex(pattern = r"^[A-Za-z0-9._/-]+$")),
        extend("uniqueItems" = true)
    )]
    pub capability_ceiling: Vec<String>,
    /// Maximum effect classes admitted for this path.
    #[schemars(length(max = 6), extend("uniqueItems" = true))]
    pub effect_ceiling: Vec<EffectClass>,
    /// Ordered causal edge invocations.
    ///
    /// Each invocation source must already be reachable from the entry, work
    /// spine, or an earlier invocation. Reordering can therefore make an
    /// otherwise valid invocation chain disconnected and inadmissible.
    #[schemars(length(max = 256))]
    pub invocations: Vec<PathInvocationV1>,
    /// Explicit native-owner transitions and required receipts.
    #[schemars(length(max = 256))]
    pub native_owner_transitions: Vec<NativeOwnerTransitionV1>,
    /// Native results and effectful outcomes, with effect identity only when applicable.
    #[schemars(length(max = 256))]
    pub native_outcomes: Vec<NativeOutcomeV1>,
    /// Destination consequences required after traversal.
    #[schemars(length(max = 256))]
    pub destination_consequences: Vec<DestinationConsequenceV1>,
    /// Reconciliation obligations and their exact frontiers.
    #[schemars(length(max = 256))]
    pub reconciliation: Vec<ReconciliationRequirementV1>,
    /// Compiled-content and source-content identities, never lifecycle generations.
    pub compiled_content: CompiledContentCoordinateV1,
    /// Assurance subject governed by the path evidence.
    #[schemars(schema_with = "composition_id_schema")]
    pub assurance_subject_id: String,
    /// Evidence carrier retained for later admission or settlement.
    #[schemars(schema_with = "composition_id_schema")]
    pub evidence_carrier_id: String,
    /// Retention policy governing path evidence.
    #[schemars(schema_with = "composition_id_schema")]
    pub retention_policy_id: String,
    /// Privacy policy governing path evidence.
    #[schemars(schema_with = "composition_id_schema")]
    pub privacy_policy_id: String,
    /// Termination contract governing the path.
    #[schemars(schema_with = "composition_id_schema")]
    pub termination_contract_id: String,
    /// Settlement contract governing the path.
    #[schemars(schema_with = "composition_id_schema")]
    pub settlement_contract_id: String,
    /// Continuity contract governing obligations after traversal.
    #[schemars(schema_with = "composition_id_schema")]
    pub continuity_contract_id: String,
    /// Finite aggregate budget for the path.
    pub budget: PathBudgetV1,
}

/// Unsealed selected-path body used to derive [`ResponsibilityPathIdentityV1`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExperiencePathPreimageV1 {
    /// Stable experience-path record identity.
    pub id: String,
    /// Receipt contract proving the path's explicit origin.
    pub origin_receipt_contract_id: String,
    /// Entry contract from which traversal begins.
    pub entry_contract_id: String,
    /// Ordered work-delegation edges traversed by the path.
    pub work_delegation_edge_ids: Vec<String>,
    /// Exact responsibility contracts used by the path.
    pub responsibility_contract_ids: Vec<String>,
    /// Exact provider bindings used by the path.
    pub provider_binding_ids: Vec<String>,
    /// Capabilities admitted for this path.
    pub capability_ceiling: Vec<String>,
    /// Maximum effect classes admitted for this path.
    pub effect_ceiling: Vec<EffectClass>,
    /// Ordered causal edge invocations.
    pub invocations: Vec<PathInvocationV1>,
    /// Explicit native-owner transitions and required receipts.
    pub native_owner_transitions: Vec<NativeOwnerTransitionV1>,
    /// Native results and effectful outcomes.
    pub native_outcomes: Vec<NativeOutcomeV1>,
    /// Destination consequences required after traversal.
    pub destination_consequences: Vec<DestinationConsequenceV1>,
    /// Reconciliation obligations and their exact frontiers.
    pub reconciliation: Vec<ReconciliationRequirementV1>,
    /// Compiled-content and source-content identities.
    pub compiled_content: CompiledContentCoordinateV1,
    /// Assurance subject governed by the path evidence.
    pub assurance_subject_id: String,
    /// Evidence carrier retained for later admission or settlement.
    pub evidence_carrier_id: String,
    /// Retention policy governing path evidence.
    pub retention_policy_id: String,
    /// Privacy policy governing path evidence.
    pub privacy_policy_id: String,
    /// Termination contract governing the path.
    pub termination_contract_id: String,
    /// Settlement contract governing the path.
    pub settlement_contract_id: String,
    /// Continuity contract governing obligations after traversal.
    pub continuity_contract_id: String,
    /// Finite aggregate budget for the path.
    pub budget: PathBudgetV1,
}

impl ExperiencePathV1 {
    /// Seal a complete selected path with its responsibility-path identity.
    ///
    /// # Errors
    ///
    /// Returns [`crate::canon::CanonError`] when the body cannot satisfy the
    /// canonical JSON contract.
    pub fn seal(preimage: ExperiencePathPreimageV1) -> Result<Self, crate::canon::CanonError> {
        let identity = ResponsibilityPathIdentityV1 {
            responsibility_path_sha256: hash_composition_identity(
                b"PALE_RESPONSIBILITY_PATH_IDENTITY_V1\0",
                &preimage,
            )?,
        };
        let ExperiencePathPreimageV1 {
            id,
            origin_receipt_contract_id,
            entry_contract_id,
            work_delegation_edge_ids,
            responsibility_contract_ids,
            provider_binding_ids,
            capability_ceiling,
            effect_ceiling,
            invocations,
            native_owner_transitions,
            native_outcomes,
            destination_consequences,
            reconciliation,
            compiled_content,
            assurance_subject_id,
            evidence_carrier_id,
            retention_policy_id,
            privacy_policy_id,
            termination_contract_id,
            settlement_contract_id,
            continuity_contract_id,
            budget,
        } = preimage;
        Ok(Self {
            id,
            identity,
            origin_receipt_contract_id,
            entry_contract_id,
            work_delegation_edge_ids,
            responsibility_contract_ids,
            provider_binding_ids,
            capability_ceiling,
            effect_ceiling,
            invocations,
            native_owner_transitions,
            native_outcomes,
            destination_consequences,
            reconciliation,
            compiled_content,
            assurance_subject_id,
            evidence_carrier_id,
            retention_policy_id,
            privacy_policy_id,
            termination_contract_id,
            settlement_contract_id,
            continuity_contract_id,
            budget,
        })
    }

    /// Clone the complete unhashed selected-path body for verification or amendment.
    #[must_use]
    pub fn to_preimage(&self) -> ExperiencePathPreimageV1 {
        ExperiencePathPreimageV1 {
            id: self.id.clone(),
            origin_receipt_contract_id: self.origin_receipt_contract_id.clone(),
            entry_contract_id: self.entry_contract_id.clone(),
            work_delegation_edge_ids: self.work_delegation_edge_ids.clone(),
            responsibility_contract_ids: self.responsibility_contract_ids.clone(),
            provider_binding_ids: self.provider_binding_ids.clone(),
            capability_ceiling: self.capability_ceiling.clone(),
            effect_ceiling: self.effect_ceiling.clone(),
            invocations: self.invocations.clone(),
            native_owner_transitions: self.native_owner_transitions.clone(),
            native_outcomes: self.native_outcomes.clone(),
            destination_consequences: self.destination_consequences.clone(),
            reconciliation: self.reconciliation.clone(),
            compiled_content: self.compiled_content.clone(),
            assurance_subject_id: self.assurance_subject_id.clone(),
            evidence_carrier_id: self.evidence_carrier_id.clone(),
            retention_policy_id: self.retention_policy_id.clone(),
            privacy_policy_id: self.privacy_policy_id.clone(),
            termination_contract_id: self.termination_contract_id.clone(),
            settlement_contract_id: self.settlement_contract_id.clone(),
            continuity_contract_id: self.continuity_contract_id.clone(),
            budget: self.budget.clone(),
        }
    }

    /// Recompute the domain-separated identity of the selected-path body.
    ///
    /// # Errors
    ///
    /// Returns [`crate::canon::CanonError`] when the body cannot satisfy the
    /// canonical JSON contract.
    pub fn recompute_identity(
        &self,
    ) -> Result<ResponsibilityPathIdentityV1, crate::canon::CanonError> {
        Ok(ResponsibilityPathIdentityV1 {
            responsibility_path_sha256: hash_composition_identity(
                b"PALE_RESPONSIBILITY_PATH_IDENTITY_V1\0",
                &self.to_preimage(),
            )?,
        })
    }
}

/// One named graph view in the derived plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CompositionGraphViewV1 {
    /// Semantic domain represented by this view.
    pub kind: CompositionGraphKindV1,
    /// Deterministically ordered participants in the domain.
    #[schemars(length(max = 1_024), extend("uniqueItems" = true))]
    pub nodes: Vec<ParticipantRefV1>,
    /// Deterministically ordered domain-specific edge identities.
    #[schemars(
        length(max = 4_096),
        inner(length(min = 3, max = 1_024)),
        extend("uniqueItems" = true)
    )]
    pub edges: Vec<String>,
}

/// One typed causal step explaining an explicitly selected experience path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "reason", rename_all = "snake_case", deny_unknown_fields)]
pub enum CompositionReasonStepV1 {
    /// Owner-issued receipt that selected the path.
    OriginReceipt {
        /// Exact receipt contract named by the source path.
        #[schemars(schema_with = "composition_id_schema")]
        contract_id: String,
    },
    /// Entry contract from which the selected path begins.
    EntryContract {
        /// Exact entry-contract identity.
        #[schemars(schema_with = "composition_id_schema")]
        contract_id: String,
    },
    /// Responsibility contract required by the selected path.
    ResponsibilityContract {
        /// Exact responsibility-contract identity.
        #[schemars(schema_with = "composition_id_schema")]
        contract_id: String,
    },
    /// Provider binding explicitly required by the selected path.
    ProviderBinding {
        /// Exact provider-binding identity.
        #[schemars(schema_with = "composition_id_schema")]
        binding_id: String,
    },
    /// Ordered delegation edge traversed by the selected path.
    DelegationEdge {
        /// Exact work-delegation edge identity.
        #[schemars(schema_with = "composition_id_schema")]
        edge_id: String,
    },
    /// Terminal contract that bounds path completion.
    TerminationContract {
        /// Exact termination-contract identity.
        #[schemars(schema_with = "composition_id_schema")]
        contract_id: String,
    },
    /// Settlement contract required after traversal.
    SettlementContract {
        /// Exact settlement-contract identity.
        #[schemars(schema_with = "composition_id_schema")]
        contract_id: String,
    },
    /// Continuity contract that survives path completion.
    ContinuityContract {
        /// Exact continuity-contract identity.
        #[schemars(schema_with = "composition_id_schema")]
        contract_id: String,
    },
}

impl CompositionReasonStepV1 {
    /// Borrow the identity carried by this causal step.
    #[must_use]
    pub fn identifier(&self) -> &str {
        match self {
            Self::OriginReceipt { contract_id }
            | Self::EntryContract { contract_id }
            | Self::ResponsibilityContract { contract_id }
            | Self::TerminationContract { contract_id }
            | Self::SettlementContract { contract_id }
            | Self::ContinuityContract { contract_id } => contract_id,
            Self::ProviderBinding { binding_id } => binding_id,
            Self::DelegationEdge { edge_id } => edge_id,
        }
    }
}

/// Structural causal explanation associated with one selected sparse path.
///
/// This public wire record is constructible and deserializable. Its shape alone
/// proves no compiler admission or source provenance, and passing it to
/// [`ExperienceCompositionPlanV1::seal`] only binds the supplied bytes. Proof
/// provenance is established by the compiler or admission layer that returns
/// the record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CompositionReasonPathV1 {
    /// Selected experience path being explained.
    #[schemars(schema_with = "composition_id_schema")]
    pub path_id: String,
    /// Exact causal steps in deterministic compiler order.
    #[schemars(length(min = 3, max = 1_024))]
    pub reasons: Vec<CompositionReasonStepV1>,
}

/// Explicit neutral composition declarations shared by source and derived plan.
///
/// This is the single structural owner of the eleven declaration families.
/// It contains no inferred graph view, reason path, readiness claim, or native
/// occurrence. The compiler consumes it and retains the same admitted values in
/// [`ExperienceCompositionPlanV1::declarations`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExperienceCompositionDeclarationsV1 {
    /// Responsibility contracts available to selected paths.
    #[schemars(length(max = 1_024))]
    pub responsibility_contracts: Vec<ResponsibilityContractV1>,
    /// Participant and provider declarations.
    #[schemars(length(max = 1_024))]
    pub participant_provider_manifests: Vec<ParticipantProviderManifestV1>,
    /// Exact provider bindings; declarations alone never select a provider.
    #[schemars(length(max = 1_024))]
    pub provider_bindings: Vec<ProviderBindingV1>,
    /// Explicit user, controller, or native entry contracts.
    #[schemars(length(max = 1_024))]
    pub entry_contracts: Vec<EntryContractV1>,
    /// Directional request translations.
    #[schemars(length(max = 1_024))]
    pub request_translations: Vec<RequestTranslationV1>,
    /// Directional result translations.
    #[schemars(length(max = 1_024))]
    pub result_translations: Vec<ResultTranslationV1>,
    /// Terminal postures owned by their declared participants.
    #[schemars(length(max = 1_024))]
    pub termination_contracts: Vec<TerminationContractV1>,
    /// Evidence and settlement dispositions.
    #[schemars(length(max = 1_024))]
    pub settlement_contracts: Vec<SettlementContractV1>,
    /// Continuity obligations that remain distinct from execution state.
    #[schemars(length(max = 1_024))]
    pub continuity_contracts: Vec<ContinuityContractV1>,
    /// Typed composition edges whose variants fix graph semantics.
    #[schemars(length(max = 4_096))]
    pub edge_contracts: Vec<CompositionEdgeV1>,
    /// Sparse experience paths explicitly selected for compilation.
    #[schemars(length(max = 256))]
    pub selected_experience_paths: Vec<ExperiencePathV1>,
}

impl ExperienceCompositionDeclarationsV1 {
    /// Construct the explicit empty declaration set selected by an empty root inventory.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            responsibility_contracts: Vec::new(),
            participant_provider_manifests: Vec::new(),
            provider_bindings: Vec::new(),
            entry_contracts: Vec::new(),
            request_translations: Vec::new(),
            result_translations: Vec::new(),
            termination_contracts: Vec::new(),
            settlement_contracts: Vec::new(),
            continuity_contracts: Vec::new(),
            edge_contracts: Vec::new(),
            selected_experience_paths: Vec::new(),
        }
    }
}

/// One explicitly enumerated workspace source for neutral composition.
///
/// An absent root-manifest entry means an explicitly empty declaration set. A
/// present entry carries the complete source set; the compiler neither scans
/// for additional files nor invents declarations from responsibility or
/// organizational topology.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceCompositionSourceV1 {
    /// Stable initial-v1 wire identity.
    #[schemars(
        with = "String",
        extend("const" = WORKSPACE_COMPOSITION_SOURCE_V1_SCHEMA)
    )]
    pub schema: SchemaId,
    /// Initial public wire version.
    #[schemars(extend("const" = WORKSPACE_COMPOSITION_SOURCE_V1_SCHEMA_VERSION))]
    pub schema_version: u32,
    /// Complete explicit neutral declaration inventory.
    pub declarations: ExperienceCompositionDeclarationsV1,
}

/// Compiler-owned derived v1 plan. Presence means only that declared neutral
/// composition was modeled and carries a self-digest claim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExperienceCompositionPlanV1 {
    /// Exact record schema identity.
    #[schemars(
        with = "String",
        extend("const" = EXPERIENCE_COMPOSITION_PLAN_V1_SCHEMA)
    )]
    pub schema: SchemaId,
    /// Raw digest of the record schema bytes.
    pub schema_sha256: SchemaDigest,
    /// Initial public wire major; all pre-release iterations remain v1.
    #[schemars(extend("const" = EXPERIENCE_COMPOSITION_PLAN_V1_SCHEMA_VERSION))]
    pub schema_version: u32,
    /// Exact source declarations retained after validation and normalization.
    pub declarations: ExperienceCompositionDeclarationsV1,
    /// Provider-compatibility domain projection.
    #[schemars(schema_with = "provider_compatibility_graph_schema")]
    pub provider_compatibility_graph: CompositionGraphViewV1,
    /// Work-delegation domain projection.
    #[schemars(schema_with = "delegation_graph_schema")]
    pub delegation_graph: CompositionGraphViewV1,
    /// UI-embedding domain projection.
    #[schemars(schema_with = "ui_embedding_graph_schema")]
    pub ui_embedding_graph: CompositionGraphViewV1,
    /// Plugin-dependency domain projection.
    #[schemars(schema_with = "plugin_dependency_graph_schema")]
    pub plugin_dependency_graph: CompositionGraphViewV1,
    /// Native-application-call domain projection.
    #[schemars(schema_with = "native_application_graph_schema")]
    pub native_application_graph: CompositionGraphViewV1,
    /// Terminal-leaf domain projection.
    #[schemars(schema_with = "terminal_leaf_graph_schema")]
    pub terminal_leaf_graph: CompositionGraphViewV1,
    /// Evidence-settlement domain projection.
    #[schemars(schema_with = "evidence_settlement_graph_schema")]
    pub evidence_settlement_graph: CompositionGraphViewV1,
    /// Deterministic explanations for every selected path.
    #[schemars(length(max = 256))]
    pub reason_paths: Vec<CompositionReasonPathV1>,
    /// Canonical identity of the complete composition-plan body.
    pub plan_digest: ExperienceCompositionPlanDigest,
}

/// Unsealed construction input for [`ExperienceCompositionPlanV1`].
///
/// This non-deserializable value exists only while the compiler assembles the
/// complete composition body. [`ExperienceCompositionPlanV1::seal`] is the
/// sole transition into the portable, digest-bearing record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExperienceCompositionPlanPreimageV1 {
    /// Exact record schema identity.
    pub schema: SchemaId,
    /// Raw digest of the record schema bytes.
    pub schema_sha256: SchemaDigest,
    /// Initial public wire major.
    pub schema_version: u32,
    /// Exact source declarations retained after validation and normalization.
    pub declarations: ExperienceCompositionDeclarationsV1,
    /// Provider-compatibility domain projection.
    pub provider_compatibility_graph: CompositionGraphViewV1,
    /// Work-delegation domain projection.
    pub delegation_graph: CompositionGraphViewV1,
    /// UI-embedding domain projection.
    pub ui_embedding_graph: CompositionGraphViewV1,
    /// Plugin-dependency domain projection.
    pub plugin_dependency_graph: CompositionGraphViewV1,
    /// Native-application-call domain projection.
    pub native_application_graph: CompositionGraphViewV1,
    /// Terminal-leaf domain projection.
    pub terminal_leaf_graph: CompositionGraphViewV1,
    /// Evidence-settlement domain projection.
    pub evidence_settlement_graph: CompositionGraphViewV1,
    /// Deterministic explanations for every selected path.
    pub reason_paths: Vec<CompositionReasonPathV1>,
}

impl ExperienceCompositionPlanV1 {
    /// Compute and attach the canonical identity of a composition-plan body.
    ///
    /// This hashes the supplied model; it does not prove source provenance,
    /// semantic admission, provider readiness, activation, or native effects.
    ///
    /// # Errors
    ///
    /// Returns [`crate::canon::CanonError`] when the preimage cannot satisfy
    /// the canonical JSON contract.
    pub fn seal(
        preimage: ExperienceCompositionPlanPreimageV1,
    ) -> Result<Self, crate::canon::CanonError> {
        let plan_digest = ExperienceCompositionPlanDigest::hash_canonical(&preimage)?;
        let ExperienceCompositionPlanPreimageV1 {
            schema,
            schema_sha256,
            schema_version,
            declarations,
            provider_compatibility_graph,
            delegation_graph,
            ui_embedding_graph,
            plugin_dependency_graph,
            native_application_graph,
            terminal_leaf_graph,
            evidence_settlement_graph,
            reason_paths,
        } = preimage;
        Ok(Self {
            schema,
            schema_sha256,
            schema_version,
            declarations,
            provider_compatibility_graph,
            delegation_graph,
            ui_embedding_graph,
            plugin_dependency_graph,
            native_application_graph,
            terminal_leaf_graph,
            evidence_settlement_graph,
            reason_paths,
            plan_digest,
        })
    }

    /// Clone the unhashed plan body for explicit amendment and resealing.
    #[must_use]
    pub fn to_preimage(&self) -> ExperienceCompositionPlanPreimageV1 {
        ExperienceCompositionPlanPreimageV1 {
            schema: self.schema.clone(),
            schema_sha256: self.schema_sha256.clone(),
            schema_version: self.schema_version,
            declarations: self.declarations.clone(),
            provider_compatibility_graph: self.provider_compatibility_graph.clone(),
            delegation_graph: self.delegation_graph.clone(),
            ui_embedding_graph: self.ui_embedding_graph.clone(),
            plugin_dependency_graph: self.plugin_dependency_graph.clone(),
            native_application_graph: self.native_application_graph.clone(),
            terminal_leaf_graph: self.terminal_leaf_graph.clone(),
            evidence_settlement_graph: self.evidence_settlement_graph.clone(),
            reason_paths: self.reason_paths.clone(),
        }
    }

    /// Recompute the canonical identity of this plan's unhashed body.
    ///
    /// # Errors
    ///
    /// Returns [`crate::canon::CanonError`] when the body cannot satisfy the
    /// canonical JSON contract.
    pub fn recompute_digest(
        &self,
    ) -> Result<ExperienceCompositionPlanDigest, crate::canon::CanonError> {
        ExperienceCompositionPlanDigest::hash_canonical(&self.to_preimage())
    }

    /// Compute the identity of the normalized provider-binding closure.
    ///
    /// For an admitted plan, provider-binding identities are unique. This
    /// projection orders bindings by that identity before hashing, making the
    /// closure independent of source declaration order without weakening the
    /// complete plan identity in [`Self::plan_digest`]. It identifies declared
    /// bindings only: the digest proves no provider availability, activation,
    /// invocation, native effect, or owner acceptance.
    ///
    /// # Errors
    ///
    /// Returns [`crate::canon::CanonError`] if the binding set cannot satisfy
    /// the canonical JSON contract.
    pub fn provider_binding_closure_digest(
        &self,
    ) -> Result<ProviderBindingClosureDigest, crate::canon::CanonError> {
        let mut bindings = self
            .declarations
            .provider_bindings
            .iter()
            .collect::<Vec<_>>();
        bindings.sort_by(|left, right| left.id.cmp(&right.id));
        ProviderBindingClosureDigest::hash_canonical(&(
            "pale.provider-binding-closure.v1",
            bindings,
        ))
    }
}
