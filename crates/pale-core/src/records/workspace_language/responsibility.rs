//! Attributable responsibility derivation and sparse projection records.
//!
//! These records begin after an owner has constituted a responsibility. They
//! preserve explicit answers and omissions; they neither create an
//! organization nor establish native authority, effect, acceptance, or
//! settlement.

use super::*;

/// Registry identity of the initial responsibility-derivation schema.
pub const RESPONSIBILITY_DERIVATION_V1_SCHEMA_ID: &str = "responsibility-derivation.v1";
/// Wire identity carried by every initial responsibility-derivation record.
pub const RESPONSIBILITY_DERIVATION_V1_SCHEMA: &str = "pale/responsibility-derivation.v1";
/// Wire version of [`ResponsibilityDerivationV1`].
pub const RESPONSIBILITY_DERIVATION_V1_SCHEMA_VERSION: u32 = 1;

portable_id!(
    #[doc = "Stable identity of one responsibility derivation."]
    ResponsibilityDerivationId,
    "responsibility derivation id"
);
portable_id!(
    #[doc = "Stable verb identity in a bounded responsibility statement."]
    ResponsibilityVerbId,
    "responsibility verb id"
);
portable_id!(
    #[doc = "Stable object identity in a bounded responsibility statement."]
    ResponsibilityObjectId,
    "responsibility object id"
);
portable_id!(
    #[doc = "Stable outcome identity in a bounded responsibility statement."]
    ResponsibilityOutcomeId,
    "responsibility outcome id"
);
portable_id!(
    #[doc = "Stable reason code explaining an explicit omission."]
    ResponsibilityOmissionReasonId,
    "responsibility omission reason id"
);
portable_id!(
    #[doc = "Stable native-projection identity for one responsibility."]
    ResponsibilityProjectionId,
    "responsibility projection id"
);
portable_id!(
    #[doc = "Stable Genealogy-link identity for responsibility references."]
    ResponsibilityGenealogyLinkId,
    "responsibility genealogy link id"
);
portable_id!(
    #[doc = "Stable occurrence reference retained by Genealogy."]
    ResponsibilityOccurrenceId,
    "responsibility occurrence id"
);

/// Maximum explicit structural-object selections in one derivation.
pub const MAX_RESPONSIBILITY_STRUCTURAL_SELECTIONS: usize = 11;
/// Maximum derivations in one pure lowering request.
pub const MAX_RESPONSIBILITY_DERIVATIONS: usize = 1_024;
/// Maximum sparse native projections in one derivation.
pub const MAX_RESPONSIBILITY_NATIVE_PROJECTIONS: usize = 1_024;
/// Maximum many-to-many Genealogy links in one derivation.
pub const MAX_RESPONSIBILITY_GENEALOGY_LINKS: usize = 4_096;
/// Maximum residence decisions in one derivation.
pub const MAX_RESPONSIBILITY_RESIDENCES: usize = 1_024;
/// Maximum members in a bounded nested responsibility set.
pub const MAX_RESPONSIBILITY_NESTED_MEMBERS: usize = 1_024;
/// Maximum lawful-silence classes in one answer.
pub const MAX_RESPONSIBILITY_SILENCE_CLASSES: usize = 3;
/// Candidate-only identifier tokens denied from responsibility and organization records.
pub const CANDIDATE_ONLY_WORKSPACE_LANGUAGE_IDENTIFIER_TOKENS_V1: [&str; 3] =
    ["concept", "contribution", "mirror"];

/// Return the candidate-only token carried by a machine identifier, if any.
///
/// Tokens are delimited by non-ASCII-alphanumeric bytes and compared without
/// ASCII case. This function is the single inventory consumed by responsibility
/// derivation, organizational translation, and contract-bundle identity checks.
/// Prose is outside its scope.
#[must_use]
pub fn candidate_only_workspace_language_identifier_token_v1(
    identifier: &str,
) -> Option<&'static str> {
    identifier
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .find_map(|token| {
            CANDIDATE_ONLY_WORKSPACE_LANGUAGE_IDENTIFIER_TOKENS_V1
                .iter()
                .copied()
                .find(|candidate| token.eq_ignore_ascii_case(candidate))
        })
}

/// Source and owner attribution attached to one answer or omission.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct ResponsibilityAttributionV1 {
    /// Exact source coordinate from which the answer was derived.
    pub source_id: WorkspaceDerivationSourceId,
    /// Owner attributable for the answer at authoring time.
    pub owner: WorkspaceResourceId,
    /// Exact evidence or authority reference retained without copying its body.
    pub evidence_ref: WorkspaceResourceId,
}

/// One supplied answer or explicit attributable omission.
///
/// The two variants make silence unrepresentable. `Omitted` is not `unknown`:
/// the author has positively stated why the answer is absent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "disposition", rename_all = "snake_case", deny_unknown_fields)]
#[schemars(rename = "AttributableAnswerFor{T}")]
pub enum AttributableAnswerV1<T> {
    /// The answer is supplied with its attribution.
    Supplied {
        /// Typed answer.
        value: T,
        /// Attribution for the answer.
        attribution: ResponsibilityAttributionV1,
    },
    /// The answer is intentionally absent for an attributable reason.
    Omitted {
        /// Stable omission reason code.
        reason: ResponsibilityOmissionReasonId,
        /// Attribution for the omission decision.
        attribution: ResponsibilityAttributionV1,
    },
}

impl<T> AttributableAnswerV1<T> {
    /// Report whether this answer is an explicit omission.
    #[must_use]
    pub const fn is_omitted(&self) -> bool {
        matches!(self, Self::Omitted { .. })
    }

    /// Borrow the attribution shared by supplied and omitted answers.
    #[must_use]
    pub fn attribution(&self) -> &ResponsibilityAttributionV1 {
        match self {
            Self::Supplied { attribution, .. } | Self::Omitted { attribution, .. } => attribution,
        }
    }
}

/// Bounded verb/object/outcome statement for one performed responsibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BoundedResponsibilityV1 {
    /// Stable performed-action identity.
    pub verb: ResponsibilityVerbId,
    /// Stable object or subject identity.
    pub object: ResponsibilityObjectId,
    /// Stable intended-outcome identity.
    pub outcome: ResponsibilityOutcomeId,
}

wire_enum!(
    /// Authority class of a responsibility's source or native truth owner.
    ResponsibilityOwnerClassV1 {
        /// Principal-owned truth.
        Principal => "principal",
        /// Relationship- or contract-owned truth.
        RelationshipContract => "relationship_contract",
        /// Native-application-owned truth.
        NativeApplication => "native_application",
        /// Independently governed module-owned truth.
        Module => "module"
    }
);

/// Exact source/native owner answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResponsibilityNativeOwnerV1 {
    /// Logical owner reference.
    pub owner: WorkspaceResourceId,
    /// Closed authority class of that owner.
    pub owner_class: ResponsibilityOwnerClassV1,
    /// Resolver that can establish current owner truth.
    pub authority_resolver: WorkspaceResourceId,
}

/// Explicit trigger for one responsibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ResponsibilityTriggerV1 {
    /// A declared recurring cadence triggers evaluation.
    RecurringCadence {
        /// Cadence definition reference.
        cadence: WorkspaceResourceId,
    },
    /// An owner-authored occurrence triggers evaluation.
    OwnerEvent {
        /// Owner-event contract reference.
        event: WorkspaceResourceId,
    },
    /// A native-owner occurrence triggers evaluation.
    NativeEvent {
        /// Native-event contract reference.
        event: WorkspaceResourceId,
    },
    /// A bounded attributable request triggers evaluation.
    BoundedRequest {
        /// Request contract reference.
        request_contract: WorkspaceResourceId,
    },
}

/// Placement of an already-constituted responsibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ResponsibilityPlacementV1 {
    /// The duty and its acceptance stay within one logical scope.
    VerticalLocal {
        /// Exact local scope.
        scope: WorkspaceResourceId,
    },
    /// One provider scope serves one consumer scope directionally.
    DirectionalService {
        /// Exact contract, including its provider and consumer scopes.
        service_contract: DirectionalServiceContractRefV1,
    },
}

wire_enum!(
    /// Closed structural questions asked of every responsibility.
    StructuralObjectKindV1 {
        /// Independently constituted logical organization.
        LogicalOrganization => "logical_organization",
        /// Stable role surviving occupant replacement.
        StableRole => "stable_role",
        /// Current occupant or provider binding.
        OccupantProvider => "occupant_provider",
        /// Directional service contract.
        DirectionalServiceContract => "directional_service_contract",
        /// Accountable-work native projection.
        AccountableWorkProjection => "accountable_work_projection",
        /// Communication native projection.
        CommunicationProjection => "communication_projection",
        /// Relationship or commercial native projection.
        RelationshipProjection => "relationship_projection",
        /// Replayable native application.
        NativeApplication => "native_application",
        /// Transferable procedure package.
        Package => "package",
        /// Owner-attested normative pack.
        NormativePack => "normative_pack",
        /// Exact typed reference with no copied body.
        ReferenceOnly => "reference_only"
    }
);

impl StructuralObjectKindV1 {
    /// Return the exhaustive structural-question inventory in wire order.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::LogicalOrganization,
            Self::StableRole,
            Self::OccupantProvider,
            Self::DirectionalServiceContract,
            Self::AccountableWorkProjection,
            Self::CommunicationProjection,
            Self::RelationshipProjection,
            Self::NativeApplication,
            Self::Package,
            Self::NormativePack,
            Self::ReferenceOnly,
        ]
    }
}

/// Explicit yes-or-no selection for one candidate structural object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResponsibilityStructuralSelectionV1 {
    /// Candidate structural class.
    pub kind: StructuralObjectKindV1,
    /// Selected resource or explicit attributable omission.
    pub selection: AttributableAnswerV1<WorkspaceResourceId>,
}

/// Actor and bounded authority responsible for framing a request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResponsibilityRequestFramerV1 {
    /// Framing role or native actor reference.
    pub actor: WorkspaceResourceId,
    /// Owner of the framing authority.
    pub owner: WorkspaceResourceId,
    /// Exact authority scope; no title or topology substitutes for it.
    pub authority_scope: WorkspaceAuthorityScopeId,
}

/// Common evidence coordinate wrapped by non-substitutable lifecycle types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResponsibilityEvidenceRequirementV1 {
    /// Owner that can establish the fact.
    pub owner: WorkspaceResourceId,
    /// Evidence contract or read-back reference.
    pub evidence_ref: WorkspaceResourceId,
}

macro_rules! distinct_responsibility_requirement {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
        #[serde(deny_unknown_fields)]
        pub struct $name {
            /// Exact owner and evidence coordinate for this lifecycle fact.
            pub requirement: ResponsibilityEvidenceRequirementV1,
        }
    };
}

distinct_responsibility_requirement!(
    /// Provider-local completion requirement; not provider acceptance.
    ProviderCompletionRequirementV1
);
distinct_responsibility_requirement!(
    /// Provider-local acceptance requirement; not completion or consumer acceptance.
    ProviderAcceptanceRequirementV1
);
distinct_responsibility_requirement!(
    /// Consumer-local consequence acceptance requirement.
    ConsumerAcceptanceRequirementV1
);
distinct_responsibility_requirement!(
    /// Native effect and owner read-back requirement.
    NativeReadBackRequirementV1
);
distinct_responsibility_requirement!(
    /// Pale obligation-settlement classification requirement.
    PaleSettlementRequirementV1
);

wire_enum!(
    /// Surface on which a responsibility may lawfully remain silent.
    ResponsibilitySilenceClassV1 {
        /// No communication consequence is required.
        Communication => "communication",
        /// No accountable-work consequence is required.
        AccountableWork => "accountable_work",
        /// No relationship-state consequence is required.
        RelationshipState => "relationship_state"
    }
);

/// Exact silence boundary; unlisted surfaces are not silently suppressed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResponsibilitySilenceBoundaryV1 {
    /// Canonical set of surfaces permitted to remain silent.
    #[schemars(length(max = 3), extend("uniqueItems" = true))]
    pub silent_surfaces: Vec<ResponsibilitySilenceClassV1>,
}

/// Exact module surface publishing a responsibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResponsibilityPublisherModuleV1 {
    /// Publishing module.
    pub module_id: WorkspaceModuleId,
    /// Exact immutable release.
    pub release_id: WorkspaceReleaseId,
    /// Named export containing the usable surface.
    pub export_id: WorkspaceExportId,
    /// Semantic owner of the published responsibility.
    pub semantic_owner: WorkspaceResourceId,
}

/// Exact consumer adoption of a published responsibility surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResponsibilityAdopterModuleV1 {
    /// Adopting module.
    pub module_id: WorkspaceModuleId,
    /// Imported publisher module.
    pub imported_module_id: WorkspaceModuleId,
    /// Exact imported release.
    pub imported_release_id: WorkspaceReleaseId,
    /// Exact imported export.
    pub imported_export_id: WorkspaceExportId,
    /// Importer-local namespace.
    pub local_namespace: WorkspaceNamespaceId,
    /// Owner attributable for adoption.
    pub adoption_owner: WorkspaceResourceId,
}

/// Policy and resource ceilings governing one derived responsibility.
///
/// [`Self::meet`] intersects policy axes and resource sets. It is commutative,
/// associative, and idempotent because both constituent meets have those laws.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResponsibilityCeilingsV1 {
    /// Authority, disclosure, controller, rights, retention, effect, and model-use ceiling.
    pub policy: WorkspacePolicyCeilingV1,
    /// Canonical set of logical resources admitted by the responsibility.
    #[schemars(length(max = 1024), extend("uniqueItems" = true))]
    pub resources: Vec<WorkspaceResourceId>,
}

impl ResponsibilityCeilingsV1 {
    /// Return the greatest ceiling admitted by both inputs.
    #[must_use]
    pub fn meet(&self, other: &Self) -> Self {
        let left = self
            .resources
            .iter()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        let right = other
            .resources
            .iter()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        Self {
            policy: self.policy.meet(&other.policy),
            resources: left.intersection(&right).cloned().collect(),
        }
    }
}

/// Current mechanism and authoritative writer retained until cutover proof.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResponsibilityIncumbentV1 {
    /// Current mechanism or artifact reference.
    pub mechanism: WorkspaceResourceId,
    /// Current authoritative writer.
    pub writer: WorkspaceResourceId,
    /// Evidence retained for later assimilation or retirement proof.
    pub retained_evidence: WorkspaceResourceId,
}

/// Operation-scoped evidence required before a responsibility is ready.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResponsibilityReadinessV1 {
    /// Exact operation contract.
    pub operation_contract: WorkspaceResourceId,
    /// Deterministic replay proof.
    pub replay_proof: WorkspaceResourceId,
    /// Failure-boundary proof.
    pub failure_proof: WorkspaceResourceId,
    /// Recovery proof.
    pub recovery_proof: WorkspaceResourceId,
    /// Rollback proof.
    pub rollback_proof: WorkspaceResourceId,
    /// Native read-back proof.
    pub native_read_back_proof: WorkspaceResourceId,
}

wire_enum!(
    /// Safe failure disposition for one responsibility.
    ResponsibilitySafeFailureV1 {
        /// Deny the consequence.
        Deny => "deny",
        /// Wait for a named dependency or owner.
        Wait => "wait",
        /// Continue in a declared reduced mode.
        Degrade => "degrade",
        /// Preserve the native or incumbent path.
        RemainNative => "remain_native",
        /// Invoke an owner-approved compensating path.
        Compensate => "compensate",
        /// Escalate to the attributable owner.
        OwnerEscalation => "owner_escalation"
    }
);

/// Standalone owner-native path that survives optional Pale composition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResponsibilityDirectPathV1 {
    /// Exact path contract reference.
    pub path: WorkspaceResourceId,
    /// Native owner of the path.
    pub native_owner: WorkspaceResourceId,
    /// Conditions under which the path remains available.
    pub availability_policy: WorkspaceResourceId,
}

wire_enum!(
    /// Lifecycle asserted for a responsibility derivation.
    ResponsibilityDerivationLifecycleV1 {
        /// Current owner truth has been verified.
        VerifiedCurrent => "verified_current",
        /// Accepted future semantic target.
        AcceptedTarget => "accepted_target",
        /// Principal-owner desired configuration.
        OwnerDesiredConfiguration => "owner_desired_configuration",
        /// Candidate design without accepted ABI status.
        Candidate => "candidate",
        /// Implementation remains open.
        ImplementationOpen => "implementation_open",
        /// Private population remains open.
        ConfigurationOpen => "configuration_open",
        /// Evidence must close before use.
        EvidenceGated => "evidence_gated",
        /// Historical evidence only.
        Historical => "historical",
        /// Rejected proposition retained as genealogy.
        Rejected => "rejected"
    }
);

wire_enum!(
    /// Explicit effective state of a fact or native projection.
    ///
    /// The variants are intentionally non-substitutable. In particular,
    /// `absent`, `denied`, and `unknown` are not a shared false value.
    EffectiveFactStateV1 {
        /// The fact is selected and effective.
        Present => "present",
        /// The fact is explicitly absent.
        Absent => "absent",
        /// Access or use is explicitly denied.
        Denied => "denied",
        /// The current fact cannot be established.
        Unknown => "unknown",
        /// The last observed fact is outside its freshness boundary.
        Stale => "stale",
        /// A successor fact has replaced this fact.
        Superseded => "superseded",
        /// The issuing owner revoked this fact.
        Revoked => "revoked"
    }
);

/// Conflict returned when two explicit effective states cannot be met.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("effective states `{left}` and `{right}` are incomparable")]
pub struct EffectiveFactStateConflictV1 {
    /// Left state.
    pub left: EffectiveFactStateV1,
    /// Right state.
    pub right: EffectiveFactStateV1,
}

impl EffectiveFactStateV1 {
    /// Meet two observations without collapsing unlike negative states.
    ///
    /// Equal states are idempotent. `unknown` is the information bottom and
    /// meets every state as `unknown`. All other unequal pairs are conflicts.
    ///
    /// # Errors
    ///
    /// Returns [`EffectiveFactStateConflictV1`] for incomparable explicit
    /// states such as `absent` and `denied`.
    pub const fn try_meet(self, other: Self) -> Result<Self, EffectiveFactStateConflictV1> {
        if self as u8 == other as u8 {
            Ok(self)
        } else if matches!(self, Self::Unknown) || matches!(other, Self::Unknown) {
            Ok(Self::Unknown)
        } else {
            Err(EffectiveFactStateConflictV1 {
                left: self,
                right: other,
            })
        }
    }
}

wire_enum!(
    /// Provider-neutral native projection class.
    ResponsibilityProjectionClassV1 {
        /// Accountable work and local acceptance.
        AccountableWork => "accountable_work",
        /// Conversation, membership, or bounded coordination.
        Communication => "communication",
        /// Relationship or commercial context.
        Relationship => "relationship",
        /// Native application request, result, effect, or read-back.
        NativeApplication => "native_application",
        /// Derived navigation or inspection view.
        DerivedView => "derived_view",
        /// Bounded derived context.
        Context => "context",
        /// Causal reference and occurrence lineage.
        Genealogy => "genealogy"
    }
);

/// One optional provider-neutral native projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResponsibilityNativeProjectionV1 {
    /// Stable projection identity.
    pub projection_id: ResponsibilityProjectionId,
    /// Projection class without a deployment-specific branch.
    pub class: ResponsibilityProjectionClassV1,
    /// Native owner retaining record and effect truth.
    pub native_owner: WorkspaceResourceId,
    /// Zero-to-many exact native references; none is primary.
    #[schemars(length(max = 1024), extend("uniqueItems" = true))]
    pub native_refs: Vec<WorkspaceResourceId>,
    /// Independently preserved effective state.
    pub state: EffectiveFactStateV1,
}

/// Many-to-many causal link among responsibilities, projections, and occurrences.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResponsibilityGenealogyLinkV1 {
    /// Stable link identity.
    pub link_id: ResponsibilityGenealogyLinkId,
    /// One-to-many participating responsibility derivations.
    #[schemars(length(min = 1, max = 1024), extend("uniqueItems" = true))]
    pub responsibilities: Vec<ResponsibilityDerivationId>,
    /// Zero-to-many native projections referenced by the link.
    #[schemars(length(max = 1024), extend("uniqueItems" = true))]
    pub projections: Vec<ResponsibilityProjectionId>,
    /// One-to-many attributable occurrences; bodies remain owner-native.
    #[schemars(length(min = 1, max = 1024), extend("uniqueItems" = true))]
    pub occurrences: Vec<ResponsibilityOccurrenceId>,
}

wire_enum!(
    /// Portable residence class retained by a derivation receipt.
    ResponsibilityResidenceClassV1 {
        /// Reusable method or obligations.
        Package => "package",
        /// Owner-attested normative content.
        NormativePack => "normative_pack",
        /// Replayable native truth or effect.
        NativeApplication => "native_application",
        /// Principal-private selection or binding.
        Configuration => "configuration",
        /// Exact reference to another owner's truth.
        Reference => "reference"
    }
);

/// One reconstructible residence decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResponsibilityResidenceV1 {
    /// Semantic residence class.
    pub class: ResponsibilityResidenceClassV1,
    /// Exact selected resource or release reference.
    pub resource: WorkspaceResourceId,
    /// Owner of the referenced content or record.
    pub owner: WorkspaceResourceId,
}

/// Complete initial-v1 responsibility-derivation source record.
///
/// The seventeen named fields correspond one-for-one to the attributable
/// derivation questions. Additional lifecycle, completion, settlement,
/// residence, projection, and Genealogy fields preserve required independent
/// seams without overloading any answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResponsibilityDerivationV1 {
    /// Stable wire schema identity.
    #[schemars(extend("const" = RESPONSIBILITY_DERIVATION_V1_SCHEMA))]
    pub schema: String,
    /// Initial wire version.
    #[schemars(extend("const" = RESPONSIBILITY_DERIVATION_V1_SCHEMA_VERSION))]
    pub schema_version: u32,
    /// Stable derivation identity.
    pub derivation_id: ResponsibilityDerivationId,
    /// Lifecycle asserted by the accountable author.
    pub lifecycle: ResponsibilityDerivationLifecycleV1,
    /// Source provenance for the complete record.
    pub provenance: ResponsibilityAttributionV1,
    /// Question 1: bounded responsibility.
    pub responsibility: AttributableAnswerV1<BoundedResponsibilityV1>,
    /// Question 2: source or native truth owner.
    pub native_owner: AttributableAnswerV1<ResponsibilityNativeOwnerV1>,
    /// Question 3: trigger.
    pub trigger: AttributableAnswerV1<ResponsibilityTriggerV1>,
    /// Question 4: vertical-local or directional-service placement.
    pub placement: AttributableAnswerV1<ResponsibilityPlacementV1>,
    /// Question 5: exhaustive structural selections.
    #[schemars(length(min = 11, max = 11))]
    pub structural_objects: Vec<ResponsibilityStructuralSelectionV1>,
    /// Question 6: request framer.
    pub request_framer: AttributableAnswerV1<ResponsibilityRequestFramerV1>,
    /// Provider completion, kept separate from question 7 acceptance.
    pub provider_completion: AttributableAnswerV1<ProviderCompletionRequirementV1>,
    /// Question 7: provider-local acceptance.
    pub provider_acceptance: AttributableAnswerV1<ProviderAcceptanceRequirementV1>,
    /// Question 8: consumer-local acceptance.
    pub consumer_acceptance: AttributableAnswerV1<ConsumerAcceptanceRequirementV1>,
    /// Question 9: native effect and read-back.
    pub native_result: AttributableAnswerV1<NativeReadBackRequirementV1>,
    /// Pale settlement, kept separate from every owner-native state.
    pub pale_settlement: AttributableAnswerV1<PaleSettlementRequirementV1>,
    /// Question 10: lawful silence.
    pub lawful_silence: AttributableAnswerV1<ResponsibilitySilenceBoundaryV1>,
    /// Question 11: publishing module surface.
    pub publisher_module: AttributableAnswerV1<ResponsibilityPublisherModuleV1>,
    /// Question 12: importing or adopting module surface.
    pub adopter_module: AttributableAnswerV1<ResponsibilityAdopterModuleV1>,
    /// Question 13: independent policy and resource ceilings.
    pub ceilings: AttributableAnswerV1<ResponsibilityCeilingsV1>,
    /// Question 14: incumbent mechanism and writer.
    pub incumbent: AttributableAnswerV1<ResponsibilityIncumbentV1>,
    /// Question 15: readiness evidence.
    pub readiness: AttributableAnswerV1<ResponsibilityReadinessV1>,
    /// Question 16: safe failure state.
    pub safe_failure: AttributableAnswerV1<ResponsibilitySafeFailureV1>,
    /// Question 17: surviving direct path.
    pub direct_path: AttributableAnswerV1<ResponsibilityDirectPathV1>,
    /// Reconstructible Package, Pack, App, configuration, and reference residence.
    #[schemars(length(min = 1, max = 1024), extend("uniqueItems" = true))]
    pub residence: Vec<ResponsibilityResidenceV1>,
    /// Explicit zero-to-many native projections.
    #[schemars(length(max = 1024))]
    pub native_projections: Vec<ResponsibilityNativeProjectionV1>,
    /// Explicit many-to-many Genealogy links.
    #[schemars(length(max = 4096))]
    pub genealogy_links: Vec<ResponsibilityGenealogyLinkV1>,
}

/// Canonical responsibility-derivation set accepted by the pure lowerer.
///
/// The set may carry cross-record many-to-many Genealogy references. It is a
/// pre-wiring semantic artifact, not an admitted universe or runtime store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResponsibilityDerivationIrV1 {
    /// Canonically ordered, identity-unique derivations.
    #[schemars(length(max = 1024))]
    pub derivations: Vec<ResponsibilityDerivationV1>,
}
