//! Exact common-request, authority-evidence, and effect-control record floor.

use serde::{Deserialize, Serialize};

use super::*;
use crate::records::*;

mod request_signing;
pub use request_signing::COMMON_REQUEST_ENVELOPE_V1_SIGNATURE_DOMAIN;

/// Public schema root for caller-signed common-request wires.
pub const COMMON_REQUEST_ENVELOPE_V1_SCHEMA_ID: &str = "common-request-envelope.v1";
/// Wire schema carried by every caller-signed common request.
pub const COMMON_REQUEST_ENVELOPE_V1_SCHEMA: &str = "pale/common-request-envelope.v1";
/// Public schema root for typed-digest structural wires.
pub const TYPED_DIGEST_V1_SCHEMA_ID: &str = "typed-digest.v1";
/// Public schema root for effect-conflict claim wires.
pub const EFFECT_CONFLICT_CLAIM_V1_SCHEMA_ID: &str = "effect-conflict-claim.v1";
/// Wire schema carried by every effect-conflict claim.
pub const EFFECT_CONFLICT_CLAIM_V1_SCHEMA: &str = "pale/effect-conflict-claim.v1";
/// Public schema root for effect-admission wires.
pub const EFFECT_ADMISSION_V1_SCHEMA_ID: &str = "effect-admission.v1";
/// Wire schema carried by every effect-admission decision.
pub const EFFECT_ADMISSION_V1_SCHEMA: &str = "pale/effect-admission.v1";
/// Public schema root for durable effect-dispatch intent wires.
pub const EFFECT_DISPATCH_INTENT_V1_SCHEMA_ID: &str = "effect-dispatch-intent.v1";
/// Wire schema carried by every effect-dispatch intent.
pub const EFFECT_DISPATCH_INTENT_V1_SCHEMA: &str = "pale/effect-dispatch-intent.v1";
/// Public schema root for effect-receipt wires.
pub const EFFECT_RECEIPT_V1_SCHEMA_ID: &str = "effect-receipt.v1";
/// Wire schema carried by every effect receipt.
pub const EFFECT_RECEIPT_V1_SCHEMA: &str = "pale/effect-receipt.v1";

/// Maximum canonical base64 length for [`MAX_RESIDENT_CONTENT_BYTES`] raw bytes.
const MAX_INLINE_BASE64_BYTES: usize = MAX_RESIDENT_CONTENT_BYTES.div_ceil(3) * 4;

/// The sole generic immutable authority pointer shape.
///
/// The hash covers the exact immutable bytes at [`Self::authority_ref`]. This
/// pair points at authority evidence; it does not acquire a second schema,
/// signature, digest, or authority layer merely by being embedded in another
/// contract.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct AuthorityEvidenceV1 {
    /// Immutable authority-record reference.
    authority_ref: ResidentRecordRefV1,
    /// SHA-256 of the exact immutable authority-record bytes.
    authority_sha256: ArtifactDigest,
}

impl AuthorityEvidenceV1 {
    /// Bind one immutable authority reference to its exact bytes.
    #[must_use]
    pub const fn new(authority_ref: ResidentRecordRefV1, authority_sha256: ArtifactDigest) -> Self {
        Self {
            authority_ref,
            authority_sha256,
        }
    }

    /// Borrow the immutable authority-record reference.
    #[must_use]
    pub const fn authority_ref(&self) -> &ResidentRecordRefV1 {
        &self.authority_ref
    }

    /// Borrow the digest of the referenced immutable bytes.
    #[must_use]
    pub const fn authority_sha256(&self) -> &ArtifactDigest {
        &self.authority_sha256
    }
}

/// Identity, role, surface, and signer coordinate of one request caller.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequestCallerV1 {
    /// Caller identity admitted by the selected authority regime.
    pub identity: RequestCallerIdV1,
    /// Caller role proven by authority rather than self-asserted.
    pub role: RequestCallerRoleV1,
    /// Native or projected surface that originated the request.
    pub surface: RequestCallerSurfaceV1,
    /// Key identity used for the caller signature.
    pub signer_key_id: ResidentKeyIdV1,
    /// Nonzero signer-key epoch checked during admission.
    pub signer_key_epoch: SigningKeyEpochV1,
}

/// Closed authority regime selected independently of transport or deployment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SigningAuthorityModeV1 {
    /// Pale's admitted deployment runtime signs.
    IntegratedRuntime,
    /// An integrated native owner signs under its published authority.
    IntegratedOwner,
    /// A standalone native owner signs under independently published authority.
    StandaloneOwnerNative,
}

/// Authority details present only for [`SigningAuthorityModeV1::IntegratedRuntime`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IntegratedRuntimeAuthorityV1 {
    /// Immutable deployment-runtime authority reference.
    pub deployment_runtime_authority_ref: ResidentRecordRefV1,
    /// Digest of the deployment-runtime authority bytes.
    pub deployment_runtime_authority_sha256: ArtifactDigest,
    /// Immutable deployment-binding reference.
    pub deployment_binding_ref: ResidentRecordRefV1,
    /// Digest of the deployment-binding bytes.
    pub deployment_binding_sha256: ArtifactDigest,
    /// Principal-trust checkpoint used for admission.
    pub checked_principal_trust_checkpoint_ref: ResidentRecordRefV1,
    /// Digest of the checked principal-trust checkpoint.
    pub checked_principal_trust_checkpoint_sha256: ArtifactDigest,
}

/// Authority details present only for [`SigningAuthorityModeV1::IntegratedOwner`].
#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, schemars::JsonSchema)]
#[schemars(transform = integrated_owner_authority_schema_v1)]
#[serde(deny_unknown_fields)]
pub struct IntegratedOwnerAuthorityV1 {
    /// Native owner component identity.
    pub owner_component_id: OwnerComponentIdV1,
    /// Owner-published record-signing authority reference.
    pub owner_record_signing_authority_ref: ResidentRecordRefV1,
    /// Digest of the owner signing-authority bytes.
    pub owner_record_signing_authority_sha256: ArtifactDigest,
    /// Owner-published contract manifest.
    pub owner_contract_manifest_ref: ResidentRecordRefV1,
    /// Digest of the owner contract-manifest bytes.
    pub owner_contract_manifest_sha256: ArtifactDigest,
    /// Positive owner-controlled signer-record sequence.
    pub owner_signer_record_sequence: OwnerSignerRecordSequenceV1,
    /// Optional digest of the predecessor owner record.
    pub previous_owner_record_sha256: Option<ArtifactDigest>,
    /// Principal-trust checkpoint used for admission.
    pub checked_principal_trust_checkpoint_ref: ResidentRecordRefV1,
    /// Digest of the checked principal-trust checkpoint.
    pub checked_principal_trust_checkpoint_sha256: ArtifactDigest,
}

impl_resident_wire_deserialize!(IntegratedOwnerAuthorityV1 {
    owner_component_id: OwnerComponentIdV1,
    owner_record_signing_authority_ref: ResidentRecordRefV1,
    owner_record_signing_authority_sha256: ArtifactDigest,
    owner_contract_manifest_ref: ResidentRecordRefV1,
    owner_contract_manifest_sha256: ArtifactDigest,
    owner_signer_record_sequence: OwnerSignerRecordSequenceV1,
    previous_owner_record_sha256: Option<ArtifactDigest>,
    checked_principal_trust_checkpoint_ref: ResidentRecordRefV1,
    checked_principal_trust_checkpoint_sha256: ArtifactDigest,
});

/// Authority details present only for [`SigningAuthorityModeV1::StandaloneOwnerNative`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StandaloneOwnerNativeAuthorityV1 {
    /// Native owner component identity.
    pub owner_component_id: OwnerComponentIdV1,
    /// Exact independently published owner-authority schema.
    pub owner_authority_schema_id: SchemaId,
    /// Digest of the owner-authority schema bytes.
    pub owner_authority_schema_sha256: SchemaDigest,
    /// Immutable owner-authority record reference.
    pub owner_authority_ref: ResidentRecordRefV1,
    /// Digest of the owner-authority record bytes.
    pub owner_authority_sha256: ArtifactDigest,
    /// Owner-published contract manifest.
    pub owner_contract_manifest_ref: ResidentRecordRefV1,
    /// Digest of the owner contract-manifest bytes.
    pub owner_contract_manifest_sha256: ArtifactDigest,
    /// Owner-native revocation frontier checked for admission.
    pub checked_owner_revocation_frontier_ref: ResidentRecordRefV1,
    /// Digest of the checked owner-native revocation frontier.
    pub checked_owner_revocation_frontier_sha256: ArtifactDigest,
}

/// Exact authority binding for one caller signature.
#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, schemars::JsonSchema)]
#[schemars(transform = signing_authority_binding_schema_v1)]
#[serde(deny_unknown_fields)]
pub struct SigningAuthorityBindingV1 {
    authority_mode: SigningAuthorityModeV1,
    principal_id: ResidentPrincipalIdV1,
    signer_key_id: ResidentKeyIdV1,
    signer_key_epoch: SigningKeyEpochV1,
    integrated_runtime: Option<IntegratedRuntimeAuthorityV1>,
    integrated_owner: Option<IntegratedOwnerAuthorityV1>,
    standalone_owner_native: Option<StandaloneOwnerNativeAuthorityV1>,
}

impl SigningAuthorityBindingV1 {
    /// Bind an integrated-runtime signer to its deployment and checked trust frontier.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] when the supplied identities or
    /// authority detail violates the common binding invariants.
    pub fn integrated_runtime(
        principal_id: ResidentPrincipalIdV1,
        signer_key_id: ResidentKeyIdV1,
        signer_key_epoch: SigningKeyEpochV1,
        authority: IntegratedRuntimeAuthorityV1,
    ) -> Result<Self, InvalidResidentRecordV1> {
        Self::new(
            principal_id,
            signer_key_id,
            signer_key_epoch,
            SigningAuthorityModeV1::IntegratedRuntime,
            Some(authority),
            None,
            None,
        )
    }

    /// Bind an integrated native-owner signer to its published owner authority.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] for invalid common or owner authority fields.
    pub fn integrated_owner(
        principal_id: ResidentPrincipalIdV1,
        signer_key_id: ResidentKeyIdV1,
        signer_key_epoch: SigningKeyEpochV1,
        authority: IntegratedOwnerAuthorityV1,
    ) -> Result<Self, InvalidResidentRecordV1> {
        Self::new(
            principal_id,
            signer_key_id,
            signer_key_epoch,
            SigningAuthorityModeV1::IntegratedOwner,
            None,
            Some(authority),
            None,
        )
    }

    /// Bind a standalone native-owner signer to its independently published authority.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] for invalid common or owner authority fields.
    pub fn standalone_owner_native(
        principal_id: ResidentPrincipalIdV1,
        signer_key_id: ResidentKeyIdV1,
        signer_key_epoch: SigningKeyEpochV1,
        authority: StandaloneOwnerNativeAuthorityV1,
    ) -> Result<Self, InvalidResidentRecordV1> {
        Self::new(
            principal_id,
            signer_key_id,
            signer_key_epoch,
            SigningAuthorityModeV1::StandaloneOwnerNative,
            None,
            None,
            Some(authority),
        )
    }

    fn new(
        principal_id: ResidentPrincipalIdV1,
        signer_key_id: ResidentKeyIdV1,
        signer_key_epoch: SigningKeyEpochV1,
        authority_mode: SigningAuthorityModeV1,
        integrated_runtime: Option<IntegratedRuntimeAuthorityV1>,
        integrated_owner: Option<IntegratedOwnerAuthorityV1>,
        standalone_owner_native: Option<StandaloneOwnerNativeAuthorityV1>,
    ) -> Result<Self, InvalidResidentRecordV1> {
        let value = Self {
            authority_mode,
            principal_id,
            signer_key_id,
            signer_key_epoch,
            integrated_runtime,
            integrated_owner,
            standalone_owner_native,
        };
        value.verify()?;
        Ok(value)
    }

    /// Return the selected authority regime.
    #[must_use]
    pub const fn authority_mode(&self) -> SigningAuthorityModeV1 {
        self.authority_mode
    }

    /// Borrow the principal whose authority admits this signer.
    #[must_use]
    pub const fn principal_id(&self) -> &ResidentPrincipalIdV1 {
        &self.principal_id
    }

    /// Borrow the exact signer-key identity.
    #[must_use]
    pub fn signer_key_id(&self) -> &str {
        self.signer_key_id.as_str()
    }

    /// Return the nonzero signer-key epoch.
    #[must_use]
    pub const fn signer_key_epoch(&self) -> &SigningKeyEpochV1 {
        &self.signer_key_epoch
    }

    /// Borrow the integrated-runtime authority when that regime is selected.
    #[must_use]
    pub const fn integrated_runtime_authority(&self) -> Option<&IntegratedRuntimeAuthorityV1> {
        self.integrated_runtime.as_ref()
    }

    /// Borrow the integrated-owner authority when that regime is selected.
    #[must_use]
    pub const fn integrated_owner_authority(&self) -> Option<&IntegratedOwnerAuthorityV1> {
        self.integrated_owner.as_ref()
    }

    /// Borrow the standalone-owner authority when that regime is selected.
    #[must_use]
    pub const fn standalone_owner_native_authority(
        &self,
    ) -> Option<&StandaloneOwnerNativeAuthorityV1> {
        self.standalone_owner_native.as_ref()
    }

    pub(crate) fn verify(&self) -> Result<(), InvalidResidentRecordV1> {
        let exact_variant = match self.authority_mode {
            SigningAuthorityModeV1::IntegratedRuntime => {
                self.integrated_runtime.is_some()
                    && self.integrated_owner.is_none()
                    && self.standalone_owner_native.is_none()
            }
            SigningAuthorityModeV1::IntegratedOwner => {
                self.integrated_runtime.is_none()
                    && self.integrated_owner.is_some()
                    && self.standalone_owner_native.is_none()
            }
            SigningAuthorityModeV1::StandaloneOwnerNative => {
                self.integrated_runtime.is_none()
                    && self.integrated_owner.is_none()
                    && self.standalone_owner_native.is_some()
            }
        };
        if let Some(authority) = &self.integrated_owner {
            validate_bounded_text(
                "integrated owner component",
                authority.owner_component_id.as_str(),
            )?;
        }
        if let Some(authority) = &self.standalone_owner_native {
            validate_bounded_text(
                "standalone owner component",
                authority.owner_component_id.as_str(),
            )?;
        }
        if !exact_variant || self.signer_key_id.as_str().is_empty() {
            Err(InvalidResidentRecordV1::NonCanonical {
                relation: "signing authority binding",
            })
        } else {
            Ok(())
        }
    }
}

impl<'de> Deserialize<'de> for SigningAuthorityBindingV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            authority_mode: SigningAuthorityModeV1,
            principal_id: ResidentPrincipalIdV1,
            signer_key_id: ResidentKeyIdV1,
            signer_key_epoch: SigningKeyEpochV1,
            integrated_runtime: Option<IntegratedRuntimeAuthorityV1>,
            integrated_owner: Option<IntegratedOwnerAuthorityV1>,
            standalone_owner_native: Option<StandaloneOwnerNativeAuthorityV1>,
        }
        let untrusted = serde_json::Value::deserialize(deserializer)?;
        reject_explicit_resident_null_v1(&untrusted).map_err(serde::de::Error::custom)?;
        let wire = serde_json::from_value::<Wire>(untrusted).map_err(serde::de::Error::custom)?;
        let value = Self {
            authority_mode: wire.authority_mode,
            principal_id: wire.principal_id,
            signer_key_id: wire.signer_key_id,
            signer_key_epoch: wire.signer_key_epoch,
            integrated_runtime: wire.integrated_runtime,
            integrated_owner: wire.integrated_owner,
            standalone_owner_native: wire.standalone_owner_native,
        };
        value.verify().map_err(serde::de::Error::custom)?;
        Ok(value)
    }
}

/// Exact admitted local or federated route coordinate.
#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, schemars::JsonSchema)]
#[schemars(transform = common_request_route_schema_v1)]
#[serde(deny_unknown_fields)]
pub struct CommonRequestRouteV1 {
    /// Principal that originated the request.
    pub origin_principal_id: ResidentPrincipalIdV1,
    /// Origin node under that principal.
    pub origin_node_id: ResidentNodeIdV1,
    /// Principal owning the destination.
    pub destination_principal_id: ResidentPrincipalIdV1,
    /// Optional exact destination node.
    pub destination_node_id: Option<ResidentNodeIdV1>,
    /// Stable admitted route identity.
    pub route_id: ResidentRouteIdV1,
    /// Positive route generation.
    pub route_generation: RouteGenerationV1,
    /// Selected transport profile kind.
    pub transport: BoundedResidentTextV1,
    /// Exact route-binding schema identity.
    pub route_binding_schema_id: SchemaId,
    /// Digest of the exact selected route-binding schema bytes.
    pub route_binding_schema_sha256: SchemaDigest,
    /// Immutable route-binding reference.
    pub route_binding_ref: ResidentRecordRefV1,
    /// Digest of the route-binding bytes.
    pub route_binding_sha256: ArtifactDigest,
    /// Immutable selected contract-set reference.
    pub contract_set_ref: ResidentRecordRefV1,
    /// Digest of the selected contract-set bytes.
    pub contract_set_sha256: ArtifactDigest,
}

impl CommonRequestRouteV1 {
    /// Whether this route carries the exact route and contract-set binding.
    ///
    /// This compares immutable coordinates only. It does not establish route
    /// freshness, provider selection, authority, or transport availability.
    #[must_use]
    pub fn matches_request_binding(&self, binding: &RequestBindingV1) -> bool {
        self.route_binding_schema_id == *binding.route_binding_schema_id()
            && self.route_binding_schema_sha256 == *binding.route_binding_schema_sha256()
            && self.route_binding_ref == *binding.route_binding_ref()
            && self.route_binding_sha256 == *binding.route_binding_sha256()
            && self.route_generation == *binding.route_generation()
            && self.contract_set_ref == *binding.contract_set_ref()
            && self.contract_set_sha256 == *binding.contract_set_sha256()
    }
}

impl_resident_wire_deserialize!(CommonRequestRouteV1 {
    origin_principal_id: ResidentPrincipalIdV1,
    origin_node_id: ResidentNodeIdV1,
    destination_principal_id: ResidentPrincipalIdV1,
    destination_node_id: Option<ResidentNodeIdV1>,
    route_id: ResidentRouteIdV1,
    route_generation: RouteGenerationV1,
    transport: BoundedResidentTextV1,
    route_binding_schema_id: SchemaId,
    route_binding_schema_sha256: SchemaDigest,
    route_binding_ref: ResidentRecordRefV1,
    route_binding_sha256: ArtifactDigest,
    contract_set_ref: ResidentRecordRefV1,
    contract_set_sha256: ArtifactDigest,
});

/// Immutable admission lineage copied into every result, receipt, and safe failure.
#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, schemars::JsonSchema)]
#[schemars(transform = request_binding_schema_v1)]
#[serde(deny_unknown_fields)]
pub struct RequestBindingV1 {
    request_id: ResidentRequestIdV1,
    correlation_id: ResidentCorrelationIdV1,
    capability_id: CapabilityIdV1,
    canonical_request_sha256: CommonRequestEnvelopeDigest,
    session_grant_id: SessionGrantIdV1,
    session_grant_sha256: ArtifactDigest,
    deployment_binding_ref: ResidentRecordRefV1,
    deployment_binding_sha256: ArtifactDigest,
    route_binding_schema_id: SchemaId,
    route_binding_schema_sha256: SchemaDigest,
    route_binding_ref: ResidentRecordRefV1,
    route_binding_sha256: ArtifactDigest,
    route_generation: RouteGenerationV1,
    contract_set_ref: ResidentRecordRefV1,
    contract_set_sha256: ArtifactDigest,
    execution_owner_fence_ref: Option<ResidentRecordRefV1>,
    execution_owner_fence_sha256: Option<ExecutionOwnerFenceDigest>,
    federation_request_admission_ref: Option<ResidentRecordRefV1>,
    federation_request_admission_sha256: Option<ArtifactDigest>,
}

/// Candidate fields validated into an immutable [`RequestBindingV1`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestBindingPreimageV1 {
    /// Stable logical request identity.
    pub request_id: ResidentRequestIdV1,
    /// End-to-end request correlation identity.
    pub correlation_id: ResidentCorrelationIdV1,
    /// Capability admitted for the request.
    pub capability_id: CapabilityIdV1,
    /// Digest of the exact canonical common request.
    pub canonical_request_sha256: CommonRequestEnvelopeDigest,
    /// Session grant selected during admission.
    pub session_grant_id: SessionGrantIdV1,
    /// Digest of the immutable session-grant bytes.
    pub session_grant_sha256: ArtifactDigest,
    /// Deployment binding selected by session admission.
    pub deployment_binding_ref: ResidentRecordRefV1,
    /// Digest of the immutable deployment-binding bytes.
    pub deployment_binding_sha256: ArtifactDigest,
    /// Exact route-binding schema identity.
    pub route_binding_schema_id: SchemaId,
    /// Digest of the selected route-binding schema bytes.
    pub route_binding_schema_sha256: SchemaDigest,
    /// Immutable route-binding reference.
    pub route_binding_ref: ResidentRecordRefV1,
    /// Digest of the immutable route-binding bytes.
    pub route_binding_sha256: ArtifactDigest,
    /// Positive selected route generation.
    pub route_generation: RouteGenerationV1,
    /// Immutable selected contract-set reference.
    pub contract_set_ref: ResidentRecordRefV1,
    /// Digest of the selected contract-set bytes.
    pub contract_set_sha256: ArtifactDigest,
    /// Optional exact organized-execution fence reference.
    pub execution_owner_fence_ref: Option<ResidentRecordRefV1>,
    /// Optional digest paired with the execution-fence reference.
    pub execution_owner_fence_sha256: Option<ExecutionOwnerFenceDigest>,
    /// Optional federation admission reference.
    pub federation_request_admission_ref: Option<ResidentRecordRefV1>,
    /// Optional digest paired with the federation admission reference.
    pub federation_request_admission_sha256: Option<ArtifactDigest>,
}

impl RequestBindingV1 {
    /// Validate the immutable request-admission lineage.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] for empty identities, zero route
    /// generation, or an incomplete federation-admission ref/hash pair.
    pub fn new(preimage: RequestBindingPreimageV1) -> Result<Self, InvalidResidentRecordV1> {
        let RequestBindingPreimageV1 {
            request_id,
            correlation_id,
            capability_id,
            canonical_request_sha256,
            session_grant_id,
            session_grant_sha256,
            deployment_binding_ref,
            deployment_binding_sha256,
            route_binding_schema_id,
            route_binding_schema_sha256,
            route_binding_ref,
            route_binding_sha256,
            route_generation,
            contract_set_ref,
            contract_set_sha256,
            execution_owner_fence_ref,
            execution_owner_fence_sha256,
            federation_request_admission_ref,
            federation_request_admission_sha256,
        } = preimage;
        let value = Self {
            request_id,
            correlation_id,
            capability_id,
            canonical_request_sha256,
            session_grant_id,
            session_grant_sha256,
            deployment_binding_ref,
            deployment_binding_sha256,
            route_binding_schema_id,
            route_binding_schema_sha256,
            route_binding_ref,
            route_binding_sha256,
            route_generation,
            contract_set_ref,
            contract_set_sha256,
            execution_owner_fence_ref,
            execution_owner_fence_sha256,
            federation_request_admission_ref,
            federation_request_admission_sha256,
        };
        value.verify()?;
        Ok(value)
    }

    /// Borrow the admitted request identity.
    #[must_use]
    pub const fn request_id(&self) -> &ResidentRequestIdV1 {
        &self.request_id
    }
    /// Borrow the end-to-end correlation identity.
    #[must_use]
    pub fn correlation_id(&self) -> &str {
        self.correlation_id.as_str()
    }
    /// Borrow the admitted capability identity.
    #[must_use]
    pub const fn capability_id(&self) -> &CapabilityIdV1 {
        &self.capability_id
    }
    /// Borrow the canonical common-request digest.
    #[must_use]
    pub const fn canonical_request_sha256(&self) -> &CommonRequestEnvelopeDigest {
        &self.canonical_request_sha256
    }
    /// Borrow the selected session-grant identity.
    #[must_use]
    pub fn session_grant_id(&self) -> &str {
        self.session_grant_id.as_str()
    }
    /// Borrow the immutable session-grant byte digest.
    #[must_use]
    pub const fn session_grant_sha256(&self) -> &ArtifactDigest {
        &self.session_grant_sha256
    }
    /// Borrow the selected deployment-binding reference.
    #[must_use]
    pub const fn deployment_binding_ref(&self) -> &ResidentRecordRefV1 {
        &self.deployment_binding_ref
    }
    /// Borrow the selected deployment-binding byte digest.
    #[must_use]
    pub const fn deployment_binding_sha256(&self) -> &ArtifactDigest {
        &self.deployment_binding_sha256
    }
    /// Borrow the exact route-binding schema identity.
    #[must_use]
    pub const fn route_binding_schema_id(&self) -> &SchemaId {
        &self.route_binding_schema_id
    }
    /// Borrow the exact route-binding schema digest.
    #[must_use]
    pub const fn route_binding_schema_sha256(&self) -> &SchemaDigest {
        &self.route_binding_schema_sha256
    }
    /// Borrow the immutable route-binding reference.
    #[must_use]
    pub const fn route_binding_ref(&self) -> &ResidentRecordRefV1 {
        &self.route_binding_ref
    }
    /// Borrow the immutable route-binding byte digest.
    #[must_use]
    pub const fn route_binding_sha256(&self) -> &ArtifactDigest {
        &self.route_binding_sha256
    }
    /// Return the positive selected route generation.
    #[must_use]
    pub const fn route_generation(&self) -> &RouteGenerationV1 {
        &self.route_generation
    }
    /// Borrow the immutable selected contract-set reference.
    #[must_use]
    pub const fn contract_set_ref(&self) -> &ResidentRecordRefV1 {
        &self.contract_set_ref
    }
    /// Borrow the digest of the selected contract-set bytes.
    #[must_use]
    pub const fn contract_set_sha256(&self) -> &ArtifactDigest {
        &self.contract_set_sha256
    }
    /// Borrow the optional organized-execution fence coordinate.
    #[must_use]
    pub fn execution_owner_fence(
        &self,
    ) -> Option<(&ResidentRecordRefV1, &ExecutionOwnerFenceDigest)> {
        self.execution_owner_fence_ref
            .as_ref()
            .zip(self.execution_owner_fence_sha256.as_ref())
    }
    /// Borrow the optional owner-supplied federation-admission coordinate.
    ///
    /// The paired reference and digest preserve structural lineage only. This
    /// accessor neither resolves nor verifies membership, grant, or revocation
    /// state, and the coordinate does not enable federation by its presence.
    #[must_use]
    pub fn federation_request_admission(&self) -> Option<(&ResidentRecordRefV1, &ArtifactDigest)> {
        self.federation_request_admission_ref
            .as_ref()
            .zip(self.federation_request_admission_sha256.as_ref())
    }

    pub(crate) fn verify(&self) -> Result<(), InvalidResidentRecordV1> {
        validate_bounded_text("request binding correlation", self.correlation_id.as_str())?;
        validate_bounded_text(
            "request binding session grant",
            self.session_grant_id.as_str(),
        )?;
        if self.correlation_id.as_str().is_empty()
            || self.session_grant_id.as_str().is_empty()
            || self.execution_owner_fence_ref.is_some()
                != self.execution_owner_fence_sha256.is_some()
            || self.federation_request_admission_ref.is_some()
                != self.federation_request_admission_sha256.is_some()
        {
            Err(InvalidResidentRecordV1::NonCanonical {
                relation: "request binding",
            })
        } else {
            Ok(())
        }
    }
}

impl<'de> Deserialize<'de> for RequestBindingV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            request_id: ResidentRequestIdV1,
            correlation_id: ResidentCorrelationIdV1,
            capability_id: CapabilityIdV1,
            canonical_request_sha256: CommonRequestEnvelopeDigest,
            session_grant_id: SessionGrantIdV1,
            session_grant_sha256: ArtifactDigest,
            deployment_binding_ref: ResidentRecordRefV1,
            deployment_binding_sha256: ArtifactDigest,
            route_binding_schema_id: SchemaId,
            route_binding_schema_sha256: SchemaDigest,
            route_binding_ref: ResidentRecordRefV1,
            route_binding_sha256: ArtifactDigest,
            route_generation: RouteGenerationV1,
            contract_set_ref: ResidentRecordRefV1,
            contract_set_sha256: ArtifactDigest,
            execution_owner_fence_ref: Option<ResidentRecordRefV1>,
            execution_owner_fence_sha256: Option<ExecutionOwnerFenceDigest>,
            federation_request_admission_ref: Option<ResidentRecordRefV1>,
            federation_request_admission_sha256: Option<ArtifactDigest>,
        }
        let untrusted = serde_json::Value::deserialize(deserializer)?;
        reject_explicit_resident_null_v1(&untrusted).map_err(serde::de::Error::custom)?;
        let wire = serde_json::from_value::<Wire>(untrusted).map_err(serde::de::Error::custom)?;
        Self::new(RequestBindingPreimageV1 {
            request_id: wire.request_id,
            correlation_id: wire.correlation_id,
            capability_id: wire.capability_id,
            canonical_request_sha256: wire.canonical_request_sha256,
            session_grant_id: wire.session_grant_id,
            session_grant_sha256: wire.session_grant_sha256,
            deployment_binding_ref: wire.deployment_binding_ref,
            deployment_binding_sha256: wire.deployment_binding_sha256,
            route_binding_schema_id: wire.route_binding_schema_id,
            route_binding_schema_sha256: wire.route_binding_schema_sha256,
            route_binding_ref: wire.route_binding_ref,
            route_binding_sha256: wire.route_binding_sha256,
            route_generation: wire.route_generation,
            contract_set_ref: wire.contract_set_ref,
            contract_set_sha256: wire.contract_set_sha256,
            execution_owner_fence_ref: wire.execution_owner_fence_ref,
            execution_owner_fence_sha256: wire.execution_owner_fence_sha256,
            federation_request_admission_ref: wire.federation_request_admission_ref,
            federation_request_admission_sha256: wire.federation_request_admission_sha256,
        })
        .map_err(serde::de::Error::custom)
    }
}

fn integrated_owner_authority_schema_v1(schema: &mut schemars::Schema) {
    forbid_explicit_null_properties_v1(schema);
}

fn common_request_route_schema_v1(schema: &mut schemars::Schema) {
    forbid_explicit_null_properties_v1(schema);
}

fn common_request_session_admission_schema_v1(schema: &mut schemars::Schema) {
    forbid_explicit_null_properties_v1(schema);
    require_all_or_none_properties_v1(
        schema,
        &["checked_revocation_ref", "checked_revocation_sha256"],
    );
}

fn common_request_scope_schema_v1(schema: &mut schemars::Schema) {
    forbid_explicit_null_properties_v1(schema);
}

fn signing_authority_binding_schema_v1(schema: &mut schemars::Schema) {
    forbid_explicit_null_properties_v1(schema);
    schema.ensure_object().insert(
        "oneOf".into(),
        serde_json::json!([
            {
                "properties": { "authority_mode": { "const": "integrated_runtime" } },
                "required": ["integrated_runtime"],
                "not": { "anyOf": [
                    { "required": ["integrated_owner"] },
                    { "required": ["standalone_owner_native"] }
                ] }
            },
            {
                "properties": { "authority_mode": { "const": "integrated_owner" } },
                "required": ["integrated_owner"],
                "not": { "anyOf": [
                    { "required": ["integrated_runtime"] },
                    { "required": ["standalone_owner_native"] }
                ] }
            },
            {
                "properties": { "authority_mode": { "const": "standalone_owner_native" } },
                "required": ["standalone_owner_native"],
                "not": { "anyOf": [
                    { "required": ["integrated_runtime"] },
                    { "required": ["integrated_owner"] }
                ] }
            }
        ]),
    );
}

fn request_binding_schema_v1(schema: &mut schemars::Schema) {
    forbid_explicit_null_properties_v1(schema);
    require_all_or_none_properties_v1(
        schema,
        &["execution_owner_fence_ref", "execution_owner_fence_sha256"],
    );
    require_all_or_none_properties_v1(
        schema,
        &[
            "federation_request_admission_ref",
            "federation_request_admission_sha256",
        ],
    );
}

/// Require complete equality of an immutable request-admission lineage.
pub(crate) fn validate_exact_request_binding(
    observed: &RequestBindingV1,
    expected: &RequestBindingV1,
    record: &'static str,
) -> Result<(), InvalidResidentRecordV1> {
    observed.verify()?;
    expected.verify()?;
    if observed == expected {
        Ok(())
    } else {
        Err(InvalidResidentRecordV1::ContextualAdmission {
            record,
            invariant: "complete immutable request binding",
        })
    }
}

/// Exact ambient-session lease, grant, and revocation frontier checked for a request.
#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, schemars::JsonSchema)]
#[schemars(transform = common_request_session_admission_schema_v1)]
#[serde(deny_unknown_fields)]
pub struct CommonRequestSessionAdmissionV1 {
    /// Exact ambient-session admission selected for this request.
    pub session_admission_id: SessionAdmissionIdV1,
    /// Digest of the exact admitted ambient-session lease bytes.
    pub session_admission_lease_digest: SessionAdmissionLeaseDigest,
    /// Exact session-grant identity.
    pub session_grant_id: SessionGrantIdV1,
    /// Digest of the immutable session-grant bytes.
    pub session_grant_sha256: ArtifactDigest,
    /// Deployment binding admitted for the session.
    pub deployment_binding_ref: ResidentRecordRefV1,
    /// Digest of the deployment-binding bytes.
    pub deployment_binding_sha256: ArtifactDigest,
    /// Session floor epoch checked against rollback.
    pub session_floor_epoch: PrincipalSessionFloorEpochV1,
    /// Principal that issued the session grant.
    pub grant_issuer_principal_id: ResidentPrincipalIdV1,
    /// Key that signed the session grant.
    pub grant_issuer_key_id: ResidentKeyIdV1,
    /// Nonzero grant-issuer key epoch.
    pub grant_issuer_key_epoch: PrincipalKeyEpochV1,
    /// Revocation epoch checked for this admission.
    pub checked_revocation_epoch: PrincipalRevocationEpochV1,
    /// Optional exact revocation record reference.
    pub checked_revocation_ref: Option<ResidentRecordRefV1>,
    /// Optional digest paired with the revocation reference.
    pub checked_revocation_sha256: Option<ArtifactDigest>,
    /// Immutable revocation-checkpoint reference.
    pub checked_revocation_checkpoint_ref: ResidentRecordRefV1,
    /// Digest of the revocation-checkpoint bytes.
    pub checked_revocation_checkpoint_sha256: ArtifactDigest,
    /// Time at which the revocation frontier was checked.
    pub checked_at: UnixTimeV1,
}

impl_resident_wire_deserialize!(CommonRequestSessionAdmissionV1 {
    session_admission_id: SessionAdmissionIdV1,
    session_admission_lease_digest: SessionAdmissionLeaseDigest,
    session_grant_id: SessionGrantIdV1,
    session_grant_sha256: ArtifactDigest,
    deployment_binding_ref: ResidentRecordRefV1,
    deployment_binding_sha256: ArtifactDigest,
    session_floor_epoch: PrincipalSessionFloorEpochV1,
    grant_issuer_principal_id: ResidentPrincipalIdV1,
    grant_issuer_key_id: ResidentKeyIdV1,
    grant_issuer_key_epoch: PrincipalKeyEpochV1,
    checked_revocation_epoch: PrincipalRevocationEpochV1,
    checked_revocation_ref: Option<ResidentRecordRefV1>,
    checked_revocation_sha256: Option<ArtifactDigest>,
    checked_revocation_checkpoint_ref: ResidentRecordRefV1,
    checked_revocation_checkpoint_sha256: ArtifactDigest,
    checked_at: UnixTimeV1,
});

/// Optional neutral execution-owner coordinate bound into a common request.
///
/// This projection repeats the owner coordinates needed to admit the request
/// against the referenced [`ExecutionOwnerFenceV1`]. It neither copies the
/// owner's native authorization body nor turns the request into owner truth.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CommonRequestExecutionOwnerFenceV1 {
    /// Immutable execution-fence reference.
    pub execution_fence_ref: ResidentRecordRefV1,
    /// Digest of the exact execution-fence bytes.
    pub execution_fence_sha256: ExecutionOwnerFenceDigest,
    /// Exact organized-execution attempt identity.
    pub execution_attempt_id: ExecutionAttemptIdV1,
    /// Neutral path selected by the canonical workspace graph.
    pub execution_path_id: ExecutionPathSelectionIdV1,
    /// Native system that owns the selected attempt.
    pub execution_owner_system_id: ExecutionOwnerSystemIdV1,
    /// Selected participant within the owner system.
    pub execution_owner_participant_id: ExecutionOwnerParticipantIdV1,
    /// Exact schema of the owner-native authorization.
    pub owner_authorization_schema_id: SchemaId,
    /// Digest of the exact owner-authorization schema bytes.
    pub owner_authorization_schema_sha256: SchemaDigest,
    /// Immutable owner-authorization reference.
    pub owner_authorization_ref: ResidentRecordRefV1,
    /// Digest of the exact owner-authorization bytes.
    pub owner_authorization_sha256: ArtifactDigest,
    /// Selected execution-owner generation.
    pub owner_generation: ExecutionOwnerGenerationV1,
    /// Selected execution control epoch.
    pub control_epoch: ExecutionControlEpochV1,
}

impl CommonRequestExecutionOwnerFenceV1 {
    /// Project the exact admission coordinates of an admitted execution fence.
    ///
    /// The projection copies identity and digest coordinates only. It neither
    /// copies owner-native authorization bytes nor makes the referenced owner
    /// record true. Deployment and scope remain in their existing common-request
    /// fields and are checked by [`CommonRequestEnvelopeV1::admit_contextual_wire`].
    #[must_use]
    pub fn from_admitted(
        execution_fence_ref: &ResidentRecordRefV1,
        execution_fence: &ExecutionOwnerFenceV1,
    ) -> Self {
        Self {
            execution_fence_ref: execution_fence_ref.clone(),
            execution_fence_sha256: execution_fence.canonical_execution_fence_sha256().clone(),
            execution_attempt_id: execution_fence.execution_attempt_id().clone(),
            execution_path_id: execution_fence.execution_path_id().clone(),
            execution_owner_system_id: execution_fence.execution_owner_system_id().clone(),
            execution_owner_participant_id: execution_fence
                .execution_owner_participant_id()
                .clone(),
            owner_authorization_schema_id: execution_fence.owner_authorization_schema_id().clone(),
            owner_authorization_schema_sha256: execution_fence
                .owner_authorization_schema_sha256()
                .clone(),
            owner_authorization_ref: execution_fence.owner_authorization_ref().clone(),
            owner_authorization_sha256: execution_fence.owner_authorization_sha256().clone(),
            owner_generation: *execution_fence.owner_generation(),
            control_epoch: *execution_fence.control_epoch(),
        }
    }
}

/// Authority-specific revocation proof in a federated claim.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct FederationRevocationProofRefV1 {
    /// Authority principal whose revocation proof is checked.
    pub authority_principal_id: ResidentPrincipalIdV1,
    /// Immutable revocation-proof reference.
    pub proof_ref: ResidentRecordRefV1,
    /// Digest of the revocation-proof bytes.
    pub proof_sha256: ArtifactDigest,
}

/// Owner-supplied structural coordinates for an optional federation claim.
///
/// Construction and contextual equality preserve the selected space,
/// membership, grant, epoch, and revocation-proof coordinates. They do not
/// establish current membership, admit the grant, verify revocation state, or
/// enable federation; those decisions remain with the owning admission path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CommonRequestFederationClaimV1 {
    /// Federation space identity.
    pub space_id: FederationSpaceIdV1,
    /// Selected governance epoch for the space.
    pub space_governance_epoch: FederationGovernanceEpochV1,
    /// Exact origin membership identity.
    pub membership_id: FederationMembershipIdV1,
    /// Selected membership-state epoch.
    pub membership_state_epoch: FederationMembershipEpochV1,
    /// Digest of the immutable membership bytes.
    pub membership_sha256: ArtifactDigest,
    /// Exact resource-owner grant identity.
    pub grant_id: FederationGrantIdV1,
    /// Digest of the immutable grant bytes.
    pub grant_sha256: ArtifactDigest,
    /// Canonically ordered authority-specific revocation proofs.
    pub revocation_proofs: Vec<FederationRevocationProofRefV1>,
}

/// Exact scope admitted for one common request.
#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, schemars::JsonSchema)]
#[schemars(transform = common_request_scope_schema_v1)]
#[serde(deny_unknown_fields)]
pub struct CommonRequestScopeV1 {
    /// Principal boundary of the requested operation.
    pub principal_scope: ResidentPrincipalIdV1,
    /// Controller boundary of the requested operation.
    pub controller_scope: ResidentControllerIdV1,
    /// Optional relationship boundary.
    pub relationship_scope: Option<RelationshipScopeV1>,
    /// Admitted execution environment.
    pub environment: BoundedResidentTextV1,
    /// Data classification governing the request.
    pub data_classification: BoundedResidentTextV1,
    /// Exact declared purpose.
    pub purpose: BoundedResidentTextV1,
    /// Ordered immutable authority evidence required by policy.
    pub authority_evidence: Vec<AuthorityEvidenceV1>,
}

impl_resident_wire_deserialize!(CommonRequestScopeV1 {
    principal_scope: ResidentPrincipalIdV1,
    controller_scope: ResidentControllerIdV1,
    relationship_scope: Option<RelationshipScopeV1>,
    environment: BoundedResidentTextV1,
    data_classification: BoundedResidentTextV1,
    purpose: BoundedResidentTextV1,
    authority_evidence: Vec<AuthorityEvidenceV1>,
});

/// Bounded request payload with exact schema and content identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CommonRequestBodyV1 {
    /// Exact body schema identity.
    pub schema_id: SchemaId,
    /// Digest of the exact selected body schema bytes.
    pub schema_sha256: SchemaDigest,
    /// Digest of the RFC-8785 canonical payload.
    pub payload_sha256: ArtifactDigest,
    /// Bounded request payload admitted by the body schema.
    pub payload: serde_json::Value,
}

/// Optional immutable expected-state precondition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExpectedPreconditionV1 {
    /// Exact expected-state schema identity.
    pub state_schema_id: SchemaId,
    /// Digest of the exact expected-state schema bytes.
    pub state_schema_sha256: SchemaDigest,
    /// Digest of the immutable expected state.
    pub expected_state_sha256: ArtifactDigest,
}

/// Exact existing operation addressed by a caller-signed request.
///
/// This coordinate binds an advance or child-start request to the operation
/// named by its outer transport frame. It is not generic authority evidence,
/// and it does not replace the operation-specific request body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OperationTargetReferenceV1 {
    /// Immutable reference of the signed operation-handle record.
    pub operation_handle_ref: ResidentRecordRefV1,
    /// Canonical digest declared by that operation handle.
    pub operation_handle_sha256: OperationHandleDigest,
}

/// Exact unsigned canonical preimage of a [`CommonRequestEnvelopeV1`].
/// Parsing it supplies claims only; the signing and admission owners must
/// independently resolve the caller's authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommonRequestEnvelopePreimageV1 {
    /// Exact common-envelope schema identity.
    pub envelope_schema_id: SchemaId,
    /// Digest of the exact selected common-envelope schema bytes.
    pub envelope_schema_sha256: SchemaDigest,
    /// Stable logical request identity.
    pub request_id: ResidentRequestIdV1,
    /// End-to-end correlation identity.
    pub correlation_id: ResidentCorrelationIdV1,
    /// Workspace whose compiled contract admits the request.
    pub workspace_id: ResidentWorkspaceIdV1,
    /// Capability requested by the caller.
    pub capability_id: CapabilityIdV1,
    /// Exact compiled Package operation requested by the caller.
    pub operation: PackageResourceRefV1,
    /// Caller and signing-key coordinate.
    pub caller: RequestCallerV1,
    /// Exact authority regime governing the caller signature.
    pub signing_authority_binding: SigningAuthorityBindingV1,
    /// Exact admitted route and contract-set coordinate.
    pub route: CommonRequestRouteV1,
    /// Exact ambient-session lease, grant, and checked revocation frontier.
    pub session_admission: CommonRequestSessionAdmissionV1,
    /// Existing operation addressed by an advance or child-start request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_target: Option<OperationTargetReferenceV1>,
    /// Optional execution-owner fence for organized execution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_owner_fence: Option<CommonRequestExecutionOwnerFenceV1>,
    /// Optional structural [`CommonRequestFederationClaimV1`] coordinates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub federation_claim: Option<CommonRequestFederationClaimV1>,
    /// Exact principal, controller, relationship, and purpose scope.
    pub scope: CommonRequestScopeV1,
    /// Closed policy-owned risk classification token.
    pub risk_class: BoundedResidentTextV1,
    /// Canonically ordered bounded resource constraints.
    pub resource_constraints: Vec<BoundedResidentTextV1>,
    /// Optional immutable expected-state precondition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_precondition: Option<ExpectedPreconditionV1>,
    /// Stable semantic idempotency key.
    pub idempotency_key: RequestIdempotencyKeyV1,
    /// Request issue time in Unix seconds.
    pub issued_at: UnixTimeV1,
    /// Hard request expiry in Unix seconds.
    pub expires_at: UnixTimeV1,
    /// Optional compiled-generation coordinate required by this capability.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation: Option<CompiledGenerationDigest>,
    /// Optional activation-fence coordinate required by this capability.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fence_token: Option<ActivationFenceDigest>,
    /// Bounded typed request body.
    pub body: CommonRequestBodyV1,
}

/// Exact caller-signed common request envelope used across trust boundaries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, schemars::JsonSchema)]
#[schemars(transform = close_resident_schema_identity_v1::<CommonRequestEnvelopeV1>)]
#[serde(deny_unknown_fields)]
pub struct CommonRequestEnvelopeV1 {
    #[schemars(with = "String", extend("const" = COMMON_REQUEST_ENVELOPE_V1_SCHEMA))]
    envelope_schema_id: SchemaId,
    envelope_schema_sha256: SchemaDigest,
    request_id: ResidentRequestIdV1,
    correlation_id: ResidentCorrelationIdV1,
    workspace_id: ResidentWorkspaceIdV1,
    capability_id: CapabilityIdV1,
    operation: PackageResourceRefV1,
    caller: RequestCallerV1,
    signing_authority_binding: SigningAuthorityBindingV1,
    route: CommonRequestRouteV1,
    session_admission: CommonRequestSessionAdmissionV1,
    #[serde(skip_serializing_if = "Option::is_none")]
    operation_target: Option<OperationTargetReferenceV1>,
    #[serde(skip_serializing_if = "Option::is_none")]
    execution_owner_fence: Option<CommonRequestExecutionOwnerFenceV1>,
    #[serde(skip_serializing_if = "Option::is_none")]
    federation_claim: Option<CommonRequestFederationClaimV1>,
    scope: CommonRequestScopeV1,
    risk_class: BoundedResidentTextV1,
    resource_constraints: Vec<BoundedResidentTextV1>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expected_precondition: Option<ExpectedPreconditionV1>,
    idempotency_key: RequestIdempotencyKeyV1,
    issued_at: UnixTimeV1,
    expires_at: UnixTimeV1,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation: Option<CompiledGenerationDigest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fence_token: Option<ActivationFenceDigest>,
    body: CommonRequestBodyV1,
    canonical_request_sha256: CommonRequestEnvelopeDigest,
    caller_signature: CanonicalSignatureV1,
}

/// Untrusted structural wire shape for [`CommonRequestEnvelopeV1`].
///
/// Deserialization establishes only field shape. Call
/// [`CommonRequestEnvelopeV1::admit_contextual_wire`] with the complete
/// owner-resolved admission context before treating the request as admitted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(transform = close_resident_schema_identity_v1::<CommonRequestEnvelopeWireV1>)]
#[serde(deny_unknown_fields)]
pub struct CommonRequestEnvelopeWireV1 {
    envelope_schema_id: SchemaId,
    envelope_schema_sha256: SchemaDigest,
    request_id: ResidentRequestIdV1,
    correlation_id: ResidentCorrelationIdV1,
    workspace_id: ResidentWorkspaceIdV1,
    capability_id: CapabilityIdV1,
    operation: PackageResourceRefV1,
    caller: RequestCallerV1,
    signing_authority_binding: SigningAuthorityBindingV1,
    route: CommonRequestRouteV1,
    session_admission: CommonRequestSessionAdmissionV1,
    #[serde(skip_serializing_if = "Option::is_none")]
    operation_target: Option<OperationTargetReferenceV1>,
    #[serde(skip_serializing_if = "Option::is_none")]
    execution_owner_fence: Option<CommonRequestExecutionOwnerFenceV1>,
    #[serde(skip_serializing_if = "Option::is_none")]
    federation_claim: Option<CommonRequestFederationClaimV1>,
    scope: CommonRequestScopeV1,
    risk_class: BoundedResidentTextV1,
    resource_constraints: Vec<BoundedResidentTextV1>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expected_precondition: Option<ExpectedPreconditionV1>,
    idempotency_key: RequestIdempotencyKeyV1,
    issued_at: UnixTimeV1,
    expires_at: UnixTimeV1,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation: Option<CompiledGenerationDigest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fence_token: Option<ActivationFenceDigest>,
    body: CommonRequestBodyV1,
    canonical_request_sha256: CommonRequestEnvelopeDigest,
    caller_signature: CanonicalSignatureV1,
}

impl CommonRequestEnvelopeWireV1 {
    /// Copy the unsigned fields claimed by this untrusted wire value.
    ///
    /// The result is a selector input only. It has not passed contextual
    /// admission, signature verification, replay admission, or policy. A
    /// caller must still pass this wire value to
    /// [`CommonRequestEnvelopeV1::admit_contextual_wire`] before using any
    /// claimed field as authority.
    #[must_use]
    pub fn untrusted_preimage(&self) -> CommonRequestEnvelopePreimageV1 {
        CommonRequestEnvelopePreimageV1 {
            envelope_schema_id: self.envelope_schema_id.clone(),
            envelope_schema_sha256: self.envelope_schema_sha256.clone(),
            request_id: self.request_id.clone(),
            correlation_id: self.correlation_id.clone(),
            workspace_id: self.workspace_id.clone(),
            capability_id: self.capability_id.clone(),
            operation: self.operation.clone(),
            caller: self.caller.clone(),
            signing_authority_binding: self.signing_authority_binding.clone(),
            route: self.route.clone(),
            session_admission: self.session_admission.clone(),
            operation_target: self.operation_target.clone(),
            execution_owner_fence: self.execution_owner_fence.clone(),
            federation_claim: self.federation_claim.clone(),
            scope: self.scope.clone(),
            risk_class: self.risk_class.clone(),
            resource_constraints: self.resource_constraints.clone(),
            expected_precondition: self.expected_precondition.clone(),
            idempotency_key: self.idempotency_key.clone(),
            issued_at: self.issued_at,
            expires_at: self.expires_at,
            generation: self.generation.clone(),
            fence_token: self.fence_token.clone(),
            body: self.body.clone(),
        }
    }
}

impl ResidentSchemaIdentityV1 for CommonRequestEnvelopeV1 {
    const SCHEMA_FIELD: &'static str = "envelope_schema_id";
    const SCHEMA: &'static str = COMMON_REQUEST_ENVELOPE_V1_SCHEMA;
    const VERSION: Option<(&'static str, u32)> = None;
}

impl ResidentSchemaIdentityV1 for CommonRequestEnvelopeWireV1 {
    const SCHEMA_FIELD: &'static str = "envelope_schema_id";
    const SCHEMA: &'static str = COMMON_REQUEST_ENVELOPE_V1_SCHEMA;
    const VERSION: Option<(&'static str, u32)> = None;
}

impl CommonRequestEnvelopePreimageV1 {
    fn validate(&self) -> Result<(), InvalidResidentRecordV1> {
        if self.envelope_schema_id.as_str() != COMMON_REQUEST_ENVELOPE_V1_SCHEMA {
            return Err(InvalidResidentRecordV1::SchemaCoordinate {
                family: "common request envelope",
            });
        }
        self.signing_authority_binding.verify()?;
        if self.signing_authority_binding.signer_key_id != self.caller.signer_key_id
            || self.signing_authority_binding.signer_key_epoch != self.caller.signer_key_epoch
            || self.route.origin_principal_id != self.signing_authority_binding.principal_id
        {
            return Err(InvalidResidentRecordV1::NonCanonical {
                relation: "request caller authority binding",
            });
        }
        if let Some(runtime) = &self.signing_authority_binding.integrated_runtime {
            if runtime.deployment_binding_ref != self.session_admission.deployment_binding_ref
                || runtime.deployment_binding_sha256
                    != self.session_admission.deployment_binding_sha256
            {
                return Err(InvalidResidentRecordV1::NonCanonical {
                    relation: "runtime session deployment binding",
                });
            }
        }
        for (kind, text) in [
            ("request correlation", self.correlation_id.as_str()),
            ("request workspace", self.workspace_id.as_str()),
            ("caller identity", self.caller.identity.as_str()),
            ("caller role", self.caller.role.as_str()),
            ("caller surface", self.caller.surface.as_str()),
            ("route identity", self.route.route_id.as_str()),
            ("route transport", self.route.transport.as_str()),
            (
                "session grant",
                self.session_admission.session_grant_id.as_str(),
            ),
            ("request risk class", self.risk_class.as_str()),
            ("request idempotency key", self.idempotency_key.as_str()),
            ("request environment", self.scope.environment.as_str()),
            (
                "data classification",
                self.scope.data_classification.as_str(),
            ),
            ("request purpose", self.scope.purpose.as_str()),
        ] {
            validate_bounded_text(kind, text)?;
        }
        validate_common_request_nested_text(self)?;
        bounded_len(
            "request resource constraints",
            self.resource_constraints.len(),
        )?;
        strictly_sorted("request resource constraints", &self.resource_constraints)?;
        if self.issued_at >= self.expires_at
            || self.session_admission.checked_revocation_ref.is_some()
                != self.session_admission.checked_revocation_sha256.is_some()
        {
            return Err(InvalidResidentRecordV1::InvalidCoordinate {
                coordinate: "common request lifetime/authority",
            });
        }
        strictly_sorted("request authority evidence", &self.scope.authority_evidence)?;
        if let Some(claim) = &self.federation_claim {
            strictly_sorted("federation revocation proofs", &claim.revocation_proofs)?;
        }
        let payload = validate_bounded_json("request payload", &self.body.payload)?;
        if ArtifactDigest::hash_bytes(payload.as_bytes()) != self.body.payload_sha256 {
            return Err(InvalidResidentRecordV1::DigestMismatch {
                family: "common request body",
            });
        }
        Ok(())
    }
}

/// Admitted execution-fence evidence used to validate one common request.
///
/// The immutable reference and admitted record stay paired so request
/// admission can derive the transport projection from one semantic owner.
#[derive(Clone, Copy)]
pub struct CommonRequestExecutionOwnerFenceAdmissionV1<'a> {
    execution_fence_ref: &'a ResidentRecordRefV1,
    execution_fence: &'a ExecutionOwnerFenceV1,
}

impl<'a> CommonRequestExecutionOwnerFenceAdmissionV1<'a> {
    /// Bind one immutable reference to the admitted fence it identifies.
    ///
    /// The caller must already have resolved this pair to the current admitted
    /// lineage head; construction performs no store read or head comparison.
    #[must_use]
    pub const fn new(
        execution_fence_ref: &'a ResidentRecordRefV1,
        execution_fence: &'a ExecutionOwnerFenceV1,
    ) -> Self {
        Self {
            execution_fence_ref,
            execution_fence,
        }
    }

    /// Project the exact owner-fence coordinates carried by a common request.
    #[must_use]
    pub fn project(self) -> CommonRequestExecutionOwnerFenceV1 {
        CommonRequestExecutionOwnerFenceV1::from_admitted(
            self.execution_fence_ref,
            self.execution_fence,
        )
    }

    fn matches_request(self, request: &CommonRequestEnvelopePreimageV1) -> bool {
        let fence = self.execution_fence;
        fence.state() == &ExecutionOwnerFenceStateV1::Active
            && request.session_admission.deployment_binding_ref == *fence.deployment_binding_ref()
            && request.session_admission.deployment_binding_sha256.as_str()
                == fence.deployment_binding_sha256().as_str()
            && request.scope.principal_scope == *fence.principal_scope()
            && request.scope.controller_scope == *fence.controller_scope()
            && request.scope.relationship_scope.as_ref() == fence.relationship_scope().as_ref()
            && request.issued_at >= *fence.issued_at()
            && fence
                .expires_at()
                .is_none_or(|expires_at| request.expires_at <= expires_at)
    }
}

/// Owner-resolved evidence used to admit one [`CommonRequestEnvelopeV1`].
///
/// Every field here is authority-, policy-, or frontier-owned rather than
/// caller-self-attested. Admission compares the complete coordinate before the
/// caller signature is accepted.
///
/// An optional execution fence is supplied as an admitted fence plus its
/// immutable reference, never as a second caller-maintained projection.
pub struct CommonRequestAdmissionContextV1<'a> {
    /// Digest of the exact selected common-request schema bytes.
    pub expected_schema_sha256: &'a SchemaDigest,
    /// Identity of the exact selected request-body schema.
    pub expected_body_schema_id: &'a SchemaId,
    /// Digest of the exact selected request-body schema bytes.
    pub expected_body_schema_sha256: &'a SchemaDigest,
    /// Caller coordinate resolved by the selected authority.
    pub caller: &'a RequestCallerV1,
    /// Exact signing authority resolved for the caller key.
    pub signing_authority: &'a SigningAuthorityBindingV1,
    /// Exact route and contract-set coordinate resolved by its owner.
    pub route: &'a CommonRequestRouteV1,
    /// Exact ambient-session lease, grant, deployment, and revocation frontier.
    pub session_admission: &'a CommonRequestSessionAdmissionV1,
    /// Optional existing operation selected by the frame owner.
    pub operation_target: Option<&'a OperationTargetReferenceV1>,
    /// Optional organized-execution fence selected by its owner.
    pub execution_owner_fence: Option<CommonRequestExecutionOwnerFenceAdmissionV1<'a>>,
    /// Optional owner-supplied [`CommonRequestFederationClaimV1`] coordinates.
    pub federation_claim: Option<&'a CommonRequestFederationClaimV1>,
    /// Exact workspace for which policy admitted the request.
    pub workspace_id: &'a ResidentWorkspaceIdV1,
    /// Exact capability selected by policy.
    pub capability_id: &'a CapabilityIdV1,
    /// Exact compiled Package operation selected by policy.
    pub operation: &'a PackageResourceRefV1,
    /// Exact principal, controller, relationship, purpose, and authority scope.
    pub scope: &'a CommonRequestScopeV1,
    /// Exact policy-owned risk classification.
    pub risk_class: &'a BoundedResidentTextV1,
    /// Complete canonical resource-bound set selected by policy.
    pub resource_constraints: &'a [BoundedResidentTextV1],
    /// Optional owner-resolved expected-state precondition.
    pub expected_precondition: Option<&'a ExpectedPreconditionV1>,
    /// Exact semantic idempotency coordinate.
    pub idempotency_key: &'a RequestIdempotencyKeyV1,
    /// Earliest issue time accepted by the owner.
    pub issued_at: UnixTimeV1,
    /// Hard expiry selected by the owner.
    pub expires_at: UnixTimeV1,
    /// Optional compiled generation selected for this request.
    pub generation: Option<&'a CompiledGenerationDigest>,
    /// Optional activation fence paired with the selected generation.
    pub fence_token: Option<&'a ActivationFenceDigest>,
}

fn validate_common_request_context(
    value: &CommonRequestEnvelopePreimageV1,
    context: &CommonRequestAdmissionContextV1<'_>,
) -> Result<(), InvalidResidentRecordV1> {
    let expected_execution_owner_fence = context
        .execution_owner_fence
        .map(CommonRequestExecutionOwnerFenceAdmissionV1::project);
    let execution_owner_fence_is_coherent = context
        .execution_owner_fence
        .is_none_or(|fence| fence.matches_request(value));
    let exact = value.envelope_schema_sha256 == *context.expected_schema_sha256
        && value.body.schema_id == *context.expected_body_schema_id
        && value.body.schema_sha256 == *context.expected_body_schema_sha256
        && value.caller == *context.caller
        && value.signing_authority_binding == *context.signing_authority
        && value.route == *context.route
        && value.session_admission == *context.session_admission
        && value.operation_target.as_ref() == context.operation_target
        && value.execution_owner_fence.as_ref() == expected_execution_owner_fence.as_ref()
        && execution_owner_fence_is_coherent
        && value.federation_claim.as_ref() == context.federation_claim
        && value.workspace_id == *context.workspace_id
        && value.capability_id == *context.capability_id
        && value.operation == *context.operation
        && value.scope == *context.scope
        && value.risk_class == *context.risk_class
        && value.resource_constraints.as_slice() == context.resource_constraints
        && value.expected_precondition.as_ref() == context.expected_precondition
        && value.idempotency_key == *context.idempotency_key
        && value.issued_at == context.issued_at
        && value.expires_at == context.expires_at
        && value.generation.as_ref() == context.generation
        && value.fence_token.as_ref() == context.fence_token;
    if exact {
        Ok(())
    } else {
        Err(InvalidResidentRecordV1::ContextualAdmission {
            record: "common request envelope",
            invariant: "complete owner-resolved admission coordinate",
        })
    }
}

fn validate_common_request_nested_text(
    value: &CommonRequestEnvelopePreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    let mut fields = vec![
        (
            "signer key",
            value.signing_authority_binding.signer_key_id.as_str(),
        ),
        ("route origin node", value.route.origin_node_id.as_str()),
        ("route identity", value.route.route_id.as_str()),
        ("route transport", value.route.transport.as_str()),
        (
            "grant issuer key",
            value.session_admission.grant_issuer_key_id.as_str(),
        ),
        ("request environment", value.scope.environment.as_str()),
        (
            "data classification",
            value.scope.data_classification.as_str(),
        ),
        ("request purpose", value.scope.purpose.as_str()),
    ];
    if let Some(node) = value.route.destination_node_id.as_ref() {
        fields.push(("route destination node", node.as_str()));
    }
    if let Some(relationship) = value.scope.relationship_scope.as_ref() {
        fields.push(("relationship scope", relationship.as_str()));
    }
    if let Some(fence) = &value.execution_owner_fence {
        fields.extend([
            ("execution attempt", fence.execution_attempt_id.as_str()),
            ("execution path", fence.execution_path_id.as_str()),
            (
                "execution owner system",
                fence.execution_owner_system_id.as_str(),
            ),
            (
                "execution owner participant",
                fence.execution_owner_participant_id.as_str(),
            ),
        ]);
    }
    if let Some(claim) = &value.federation_claim {
        fields.extend([
            ("federation space", claim.space_id.as_str()),
            ("federation membership", claim.membership_id.as_str()),
            ("federation grant", claim.grant_id.as_str()),
        ]);
        bounded_len(
            "federation revocation proofs",
            claim.revocation_proofs.len(),
        )?;
    }
    for constraint in &value.resource_constraints {
        validate_bounded_text("request resource constraint", constraint.as_str())?;
    }
    for (relation, text) in fields {
        validate_bounded_text(relation, text)?;
    }
    Ok(())
}

fn validate_bounded_text(kind: &'static str, value: &str) -> Result<(), InvalidResidentRecordV1> {
    if value.is_empty()
        || value.len() > MAX_RESIDENT_ID_BYTES
        || value.chars().any(char::is_control)
    {
        Err(InvalidResidentRecordV1::InvalidIdentity { kind })
    } else {
        Ok(())
    }
}

impl CommonRequestEnvelopeV1 {
    /// Derive the complete binding of an already admitted non-federated request.
    ///
    /// This copies signed coordinates, including the optional execution fence;
    /// it does not re-admit or renew the request. Federation requires its own
    /// downstream admission record and cannot be represented by this local path.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] if a federation claim or a
    /// cross-principal route requires federation admission.
    pub fn local_request_binding(&self) -> Result<RequestBindingV1, InvalidResidentRecordV1> {
        if self.federation_claim.is_some()
            || self.route.origin_principal_id != self.route.destination_principal_id
        {
            return Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "local request binding",
                invariant: "federated request requires its admission owner",
            });
        }
        RequestBindingV1::new(RequestBindingPreimageV1 {
            request_id: self.request_id.clone(),
            correlation_id: self.correlation_id.clone(),
            capability_id: self.capability_id.clone(),
            canonical_request_sha256: self.canonical_request_sha256.clone(),
            session_grant_id: self.session_admission.session_grant_id.clone(),
            session_grant_sha256: self.session_admission.session_grant_sha256.clone(),
            deployment_binding_ref: self.session_admission.deployment_binding_ref.clone(),
            deployment_binding_sha256: self.session_admission.deployment_binding_sha256.clone(),
            route_binding_schema_id: self.route.route_binding_schema_id.clone(),
            route_binding_schema_sha256: self.route.route_binding_schema_sha256.clone(),
            route_binding_ref: self.route.route_binding_ref.clone(),
            route_binding_sha256: self.route.route_binding_sha256.clone(),
            route_generation: self.route.route_generation,
            contract_set_ref: self.route.contract_set_ref.clone(),
            contract_set_sha256: self.route.contract_set_sha256.clone(),
            execution_owner_fence_ref: self
                .execution_owner_fence
                .as_ref()
                .map(|fence| fence.execution_fence_ref.clone()),
            execution_owner_fence_sha256: self
                .execution_owner_fence
                .as_ref()
                .map(|fence| fence.execution_fence_sha256.clone()),
            federation_request_admission_ref: None,
            federation_request_admission_sha256: None,
        })
    }

    /// Validate and seal the caller-signature preimage.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] when any nested authority, route,
    /// lifetime, bound, canonical-order, or payload-digest law fails.
    pub(crate) fn seal(
        preimage: CommonRequestEnvelopePreimageV1,
        caller_signature: CanonicalSignatureV1,
    ) -> Result<Self, InvalidResidentRecordV1> {
        preimage.validate()?;
        let canonical_request_sha256 = CommonRequestEnvelopeDigest::hash_canonical(&preimage)?;
        let CommonRequestEnvelopePreimageV1 {
            envelope_schema_id,
            envelope_schema_sha256,
            request_id,
            correlation_id,
            workspace_id,
            capability_id,
            operation,
            caller,
            signing_authority_binding,
            route,
            session_admission,
            operation_target,
            execution_owner_fence,
            federation_claim,
            scope,
            risk_class,
            resource_constraints,
            expected_precondition,
            idempotency_key,
            issued_at,
            expires_at,
            generation,
            fence_token,
            body,
        } = preimage;
        Ok(Self {
            envelope_schema_id,
            envelope_schema_sha256,
            request_id,
            correlation_id,
            workspace_id,
            capability_id,
            operation,
            caller,
            signing_authority_binding,
            route,
            session_admission,
            operation_target,
            execution_owner_fence,
            federation_claim,
            scope,
            risk_class,
            resource_constraints,
            expected_precondition,
            idempotency_key,
            issued_at,
            expires_at,
            generation,
            fence_token,
            body,
            canonical_request_sha256,
            caller_signature,
        })
    }

    /// Admit an untrusted request wire against resolved owner context.
    ///
    /// Admission compares the complete authority, route, session, scope, policy,
    /// lifetime, and body coordinate before invoking caller-signature
    /// verification. A selected execution fence must be active, match the
    /// request's deployment and scopes, and contain the request lifetime.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] when nested structure, canonical
    /// request identity, owner-resolved context, execution-fence containment,
    /// or caller-signature verification fails.
    pub fn admit_contextual_wire<F>(
        wire: CommonRequestEnvelopeWireV1,
        context: &CommonRequestAdmissionContextV1<'_>,
        verify_signature: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnOnce(
            &CommonRequestEnvelopePreimageV1,
            &CommonRequestEnvelopeDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let claimed = wire.canonical_request_sha256;
        let caller_signature = wire.caller_signature;
        let preimage = CommonRequestEnvelopePreimageV1 {
            envelope_schema_id: wire.envelope_schema_id,
            envelope_schema_sha256: wire.envelope_schema_sha256,
            request_id: wire.request_id,
            correlation_id: wire.correlation_id,
            workspace_id: wire.workspace_id,
            capability_id: wire.capability_id,
            operation: wire.operation,
            caller: wire.caller,
            signing_authority_binding: wire.signing_authority_binding,
            route: wire.route,
            session_admission: wire.session_admission,
            operation_target: wire.operation_target,
            execution_owner_fence: wire.execution_owner_fence,
            federation_claim: wire.federation_claim,
            scope: wire.scope,
            risk_class: wire.risk_class,
            resource_constraints: wire.resource_constraints,
            expected_precondition: wire.expected_precondition,
            idempotency_key: wire.idempotency_key,
            issued_at: wire.issued_at,
            expires_at: wire.expires_at,
            generation: wire.generation,
            fence_token: wire.fence_token,
            body: wire.body,
        };
        let value = Self::seal(preimage, caller_signature)?;
        if value.canonical_request_sha256 != claimed {
            return Err(InvalidResidentRecordV1::DigestMismatch {
                family: "common request envelope",
            });
        }
        context.signing_authority.verify()?;
        value.verify()?;
        let preimage = value.to_preimage();
        validate_common_request_context(&preimage, context)?;
        verify_signature(
            &preimage,
            &value.canonical_request_sha256,
            &value.caller_signature,
        )?;
        Ok(value)
    }

    /// Revalidate structure and the exact caller-signature preimage digest.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] when structure or the canonical
    /// request digest no longer verifies.
    pub fn verify(&self) -> Result<(), InvalidResidentRecordV1> {
        let preimage = self.to_preimage();
        preimage.validate()?;
        if CommonRequestEnvelopeDigest::hash_canonical(&preimage)? == self.canonical_request_sha256
        {
            Ok(())
        } else {
            Err(InvalidResidentRecordV1::DigestMismatch {
                family: "common request envelope",
            })
        }
    }

    /// Verify that this common request carries exactly one admitted action body.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] unless the body schema, canonical
    /// payload, and action digest match and the outer generic precondition is
    /// absent as required by the effect-specific contract.
    pub fn verify_action_body(
        &self,
        action: &ActionEnvelopeV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        self.verify()?;
        let payload =
            serde_json::to_value(action).map_err(crate::canon::CanonError::Canonicalize)?;
        let digest = action.canonical_action_body_sha256()?;
        if self.body.schema_id.as_str() == ACTION_ENVELOPE_V1_SCHEMA
            && self.expected_precondition.is_none()
            && self.body.payload == payload
            && self.body.payload_sha256.as_str() == digest.as_str()
        {
            Ok(())
        } else {
            Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "common request action body",
                invariant: "exact action payload and effect-specific precondition ownership",
            })
        }
    }

    /// Verify that this common request carries the exact admitted unsigned
    /// reconciliation body supplied by the caller.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] unless the body schema, canonical
    /// payload, and reconciliation-request digest match exactly.
    pub fn verify_reconciliation_request_body(
        &self,
        request: &ReconciliationRequestV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        self.verify()?;
        let payload =
            serde_json::to_value(request).map_err(crate::canon::CanonError::Canonicalize)?;
        if self.body.schema_id.as_str() == RECONCILIATION_REQUEST_V1_SCHEMA
            && self.body.payload == payload
            && self.body.payload_sha256.as_str()
                == request.canonical_reconciliation_request_sha256()?.as_str()
        {
            Ok(())
        } else {
            Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "common request reconciliation body",
                invariant: "exact admitted reconciliation request body",
            })
        }
    }

    /// Verify that this common request carries one exact signed operation handle.
    ///
    /// This validates the request body only. It does not recover the operation,
    /// establish current inspection or cancellation authority, or renew any
    /// lease represented by the handle.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] unless the body schema, schema digest,
    /// canonical payload, and payload digest all name the supplied handle.
    pub fn verify_operation_handle_body(
        &self,
        handle: &OperationHandleV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        self.verify()?;
        handle.verify()?;
        let payload =
            serde_json::to_value(handle).map_err(crate::canon::CanonError::Canonicalize)?;
        let payload_jcs = crate::canon::to_canonical_string(&payload)?;
        let payload_sha256 = ArtifactDigest::hash_bytes(payload_jcs.as_bytes());
        if self.body.schema_id.as_str() == OPERATION_HANDLE_V1_SCHEMA
            && self.body.schema_sha256 == *handle.schema_sha256()
            && self.body.payload == payload
            && self.body.payload_sha256 == payload_sha256
        {
            Ok(())
        } else {
            Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "common request operation-handle body",
                invariant: "exact signed operation-handle payload",
            })
        }
    }

    /// Reconstruct the immutable unsigned preimage for inspection or signature verification.
    #[must_use]
    pub fn to_preimage(&self) -> CommonRequestEnvelopePreimageV1 {
        CommonRequestEnvelopePreimageV1 {
            envelope_schema_id: self.envelope_schema_id.clone(),
            envelope_schema_sha256: self.envelope_schema_sha256.clone(),
            request_id: self.request_id.clone(),
            correlation_id: self.correlation_id.clone(),
            workspace_id: self.workspace_id.clone(),
            capability_id: self.capability_id.clone(),
            operation: self.operation.clone(),
            caller: self.caller.clone(),
            signing_authority_binding: self.signing_authority_binding.clone(),
            route: self.route.clone(),
            session_admission: self.session_admission.clone(),
            operation_target: self.operation_target.clone(),
            execution_owner_fence: self.execution_owner_fence.clone(),
            federation_claim: self.federation_claim.clone(),
            scope: self.scope.clone(),
            risk_class: self.risk_class.clone(),
            resource_constraints: self.resource_constraints.clone(),
            expected_precondition: self.expected_precondition.clone(),
            idempotency_key: self.idempotency_key.clone(),
            issued_at: self.issued_at,
            expires_at: self.expires_at,
            generation: self.generation.clone(),
            fence_token: self.fence_token.clone(),
            body: self.body.clone(),
        }
    }

    /// Borrow the schema identity of the signed request envelope, not its body.
    #[must_use]
    pub const fn envelope_schema_id(&self) -> &SchemaId {
        &self.envelope_schema_id
    }

    /// Borrow the schema digest of the signed request envelope, not its body.
    #[must_use]
    pub const fn envelope_schema_sha256(&self) -> &SchemaDigest {
        &self.envelope_schema_sha256
    }

    /// Borrow the canonical caller-signature preimage digest.
    #[must_use]
    pub const fn canonical_request_sha256(&self) -> &CommonRequestEnvelopeDigest {
        &self.canonical_request_sha256
    }

    /// Borrow the stable request identity.
    #[must_use]
    pub const fn request_id(&self) -> &ResidentRequestIdV1 {
        &self.request_id
    }

    /// Borrow the exact capability bound into the signed request.
    #[must_use]
    pub const fn capability_id(&self) -> &CapabilityIdV1 {
        &self.capability_id
    }

    /// Borrow the exact compiled Package operation bound into the signed request.
    #[must_use]
    pub const fn operation(&self) -> &PackageResourceRefV1 {
        &self.operation
    }

    /// Borrow the admitted caller identity and signing coordinate.
    #[must_use]
    pub const fn caller(&self) -> &RequestCallerV1 {
        &self.caller
    }

    /// Borrow the admitted request workspace identity; this is not an execution-workspace cast.
    #[must_use]
    pub const fn workspace_id(&self) -> &ResidentWorkspaceIdV1 {
        &self.workspace_id
    }

    /// Borrow the exact route coordinates already bound into the caller signature.
    #[must_use]
    pub const fn route(&self) -> &CommonRequestRouteV1 {
        &self.route
    }

    /// Borrow the exact request-side ambient lease, grant, and revocation coordinates.
    #[must_use]
    pub const fn session_admission(&self) -> &CommonRequestSessionAdmissionV1 {
        &self.session_admission
    }

    /// Borrow the existing operation addressed by this request, when present.
    #[must_use]
    pub const fn operation_target(&self) -> Option<&OperationTargetReferenceV1> {
        self.operation_target.as_ref()
    }

    /// Borrow the exact admitted request scope.
    #[must_use]
    pub const fn scope(&self) -> &CommonRequestScopeV1 {
        &self.scope
    }

    /// Borrow the exact typed body covered by the caller signature.
    #[must_use]
    pub const fn body(&self) -> &CommonRequestBodyV1 {
        &self.body
    }

    /// Borrow the policy-owned risk classification.
    #[must_use]
    pub const fn risk_class(&self) -> &BoundedResidentTextV1 {
        &self.risk_class
    }
    /// Return the caller-signed issue time.
    #[must_use]
    pub const fn issued_at(&self) -> UnixTimeV1 {
        self.issued_at
    }
    /// Return the caller-signed expiry time.
    #[must_use]
    pub const fn expires_at(&self) -> UnixTimeV1 {
        self.expires_at
    }

    /// Borrow the compiled generation required by this request, when selected.
    #[must_use]
    pub const fn generation(&self) -> Option<&CompiledGenerationDigest> {
        self.generation.as_ref()
    }

    /// Borrow the activation fence required by this request, when selected.
    #[must_use]
    pub const fn fence_token(&self) -> Option<&ActivationFenceDigest> {
        self.fence_token.as_ref()
    }

    /// Borrow the opaque caller signature.
    #[must_use]
    pub const fn caller_signature(&self) -> &CanonicalSignatureV1 {
        &self.caller_signature
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/records/resident/effect.rs"]
mod tests;

#[cfg(test)]
#[path = "../../../tests/internal/records/resident/effect/contextual.rs"]
mod contextual_tests;

/// Exactly one content carrier covered by a [`TypedDigestV1`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TypedDigestContentBindingV1 {
    /// Bounded JSON content canonicalized with RFC 8785.
    InlineJson {
        /// Bounded JSON value hashed after RFC 8785 canonicalization.
        inline_json: serde_json::Value,
    },
    /// Bounded raw bytes represented with canonical base64.
    InlineBytesBase64 {
        /// Canonical base64 representation of bounded raw bytes.
        inline_bytes_base64: String,
    },
    /// Immutable content retained by its native owner.
    ImmutableContent {
        /// Immutable content reference.
        content_ref: ResidentRecordRefV1,
        /// Digest of the referenced immutable bytes.
        content_ref_sha256: ArtifactDigest,
    },
}

/// Typed canonicalization and binding of state or provider evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TypedDigestCanonicalizationV1 {
    /// RFC 8785 JSON Canonicalization Scheme.
    Rfc8785Json,
    /// Exact raw bytes without textual reinterpretation.
    RawBytes,
}

/// Digest algorithm selected by the exact initial-v1 typed-digest contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TypedDigestAlgorithmV1 {
    /// SHA-256 over the selected canonical representation.
    Sha256,
}

/// Typed canonicalization and binding of state or provider evidence.
///
/// The rendered law below defines semantic identity as the complete coordinate
/// of kind, schema and its hash, canonicalizer, algorithm, and
/// canonical-byte hash. The terminal hash alone is insufficient. In the
/// rendered coordinate, `k` is kind, `sigma` schema identity, `c` canonicalizer,
/// `a` is the hash algorithm, and `hash` is SHA-256 over the indicated bytes:
#[doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/typed-digest-coordinate.display.mathml"
))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TypedDigestV1 {
    /// Domain-specific content classification.
    kind: BoundedResidentTextV1,
    /// Exact content schema identity.
    content_schema_id: SchemaId,
    /// Digest of the selected content schema bytes.
    content_schema_sha256: SchemaDigest,
    /// Closed canonicalization algorithm token.
    canonicalization: TypedDigestCanonicalizationV1,
    /// Closed digest algorithm token.
    digest_algorithm: TypedDigestAlgorithmV1,
    /// Digest of the canonical bound content.
    digest: ArtifactDigest,
    /// Exactly one inline or immutable content binding.
    content_binding: TypedDigestContentBindingV1,
}

/// Untrusted structural wire shape for [`TypedDigestV1`].
///
/// Immutable content is not admitted by deserialization. Call
/// [`TypedDigestV1::admit_wire`] with an owner-native byte resolver.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TypedDigestWireV1 {
    kind: BoundedResidentTextV1,
    content_schema_id: SchemaId,
    content_schema_sha256: SchemaDigest,
    canonicalization: TypedDigestCanonicalizationV1,
    digest_algorithm: TypedDigestAlgorithmV1,
    digest: ArtifactDigest,
    content_binding: TypedDigestContentBindingV1,
}

/// Candidate typed digest admitted only after immutable-byte resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TypedDigestPreimageV1 {
    /// Domain-specific content classification.
    pub kind: BoundedResidentTextV1,
    /// Exact content-schema identity.
    pub content_schema_id: SchemaId,
    /// Digest of the selected content-schema bytes.
    pub content_schema_sha256: SchemaDigest,
    /// Canonicalization applied before hashing.
    pub canonicalization: TypedDigestCanonicalizationV1,
    /// Closed digest algorithm token.
    pub digest_algorithm: TypedDigestAlgorithmV1,
    /// Digest of the canonical bound content.
    pub digest: ArtifactDigest,
    /// Exactly one inline or immutable content binding.
    pub content_binding: TypedDigestContentBindingV1,
}

impl TypedDigestV1 {
    /// Admit one typed content digest with immutable-byte resolution.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] when representation, bounds,
    /// canonical encoding, immutable resolution, or digest validation fails.
    pub fn admit<F>(
        preimage: TypedDigestPreimageV1,
        mut resolve_immutable: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnMut(&ResidentRecordRefV1) -> Option<Vec<u8>>,
    {
        let TypedDigestPreimageV1 {
            kind,
            content_schema_id,
            content_schema_sha256,
            canonicalization,
            digest_algorithm,
            digest,
            content_binding,
        } = preimage;
        let value = Self {
            kind,
            content_schema_id,
            content_schema_sha256,
            canonicalization,
            digest_algorithm,
            digest,
            content_binding,
        };
        validate_typed_digest_with_resolver(&value, &mut resolve_immutable)?;
        Ok(value)
    }

    /// Admit an untrusted structural wire digest with exact byte resolution.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] under the same conditions as
    /// [`Self::admit`].
    pub fn admit_wire<F>(
        wire: TypedDigestWireV1,
        resolve_immutable: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnMut(&ResidentRecordRefV1) -> Option<Vec<u8>>,
    {
        Self::admit(
            TypedDigestPreimageV1 {
                kind: wire.kind,
                content_schema_id: wire.content_schema_id,
                content_schema_sha256: wire.content_schema_sha256,
                canonicalization: wire.canonicalization,
                digest_algorithm: wire.digest_algorithm,
                digest: wire.digest,
                content_binding: wire.content_binding,
            },
            resolve_immutable,
        )
    }

    /// Borrow the domain-specific content classification.
    #[must_use]
    pub fn kind(&self) -> &str {
        self.kind.as_str()
    }
    /// Borrow the exact content-schema identity.
    #[must_use]
    pub const fn content_schema_id(&self) -> &SchemaId {
        &self.content_schema_id
    }
    /// Borrow the digest of the selected content-schema bytes.
    #[must_use]
    pub const fn content_schema_sha256(&self) -> &SchemaDigest {
        &self.content_schema_sha256
    }
    /// Return the canonicalization applied before [`Self::digest`] is computed.
    #[must_use]
    pub const fn canonicalization(&self) -> TypedDigestCanonicalizationV1 {
        self.canonicalization
    }
    /// Return the digest algorithm applied to canonical content.
    #[must_use]
    pub const fn digest_algorithm(&self) -> TypedDigestAlgorithmV1 {
        self.digest_algorithm
    }
    /// Borrow the canonical-content digest, distinct from an immutable reference's byte hash.
    #[must_use]
    pub const fn digest(&self) -> &ArtifactDigest {
        &self.digest
    }
    /// Borrow the exact inline or immutable content binding.
    #[must_use]
    pub const fn content_binding(&self) -> &TypedDigestContentBindingV1 {
        &self.content_binding
    }
}

pub(crate) fn validate_typed_digest(value: &TypedDigestV1) -> Result<(), InvalidResidentRecordV1> {
    validate_typed_digest_shape(value)
}

fn validate_typed_digest_shape(value: &TypedDigestV1) -> Result<(), InvalidResidentRecordV1> {
    validate_bounded_text("typed digest kind", value.kind.as_str())?;
    let representation_matches = matches!(
        (&value.canonicalization, &value.content_binding),
        (
            TypedDigestCanonicalizationV1::Rfc8785Json,
            TypedDigestContentBindingV1::InlineJson { .. }
        ) | (
            TypedDigestCanonicalizationV1::RawBytes,
            TypedDigestContentBindingV1::InlineBytesBase64 { .. }
        ) | (_, TypedDigestContentBindingV1::ImmutableContent { .. })
    );
    if !representation_matches {
        return Err(InvalidResidentRecordV1::NonCanonical {
            relation: "typed digest content binding",
        });
    }
    match &value.content_binding {
        TypedDigestContentBindingV1::InlineJson { inline_json } => {
            let canonical = crate::canon::to_canonical_string(inline_json)?;
            validate_bounded_content_bytes("typed digest inline JSON bytes", canonical.as_bytes())?;
            if ArtifactDigest::hash_bytes(canonical.as_bytes()) != value.digest {
                return Err(InvalidResidentRecordV1::DigestMismatch {
                    family: "typed digest inline JSON",
                });
            }
        }
        TypedDigestContentBindingV1::InlineBytesBase64 {
            inline_bytes_base64,
        } => {
            if inline_bytes_base64.is_empty() || inline_bytes_base64.len() > MAX_INLINE_BASE64_BYTES
            {
                return Err(InvalidResidentRecordV1::BoundExceeded {
                    relation: "typed digest inline base64 bytes",
                    observed: inline_bytes_base64.len(),
                    maximum: MAX_INLINE_BASE64_BYTES,
                });
            }
            let decoded = decode_canonical_base64(inline_bytes_base64)?;
            if ArtifactDigest::hash_bytes(&decoded) != value.digest {
                return Err(InvalidResidentRecordV1::DigestMismatch {
                    family: "typed digest inline bytes",
                });
            }
        }
        TypedDigestContentBindingV1::ImmutableContent { .. } => {}
    }
    Ok(())
}

fn validate_typed_digest_with_resolver<F>(
    value: &TypedDigestV1,
    resolve: &mut F,
) -> Result<(), InvalidResidentRecordV1>
where
    F: FnMut(&ResidentRecordRefV1) -> Option<Vec<u8>> + ?Sized,
{
    validate_typed_digest_shape(value)?;
    if let TypedDigestContentBindingV1::ImmutableContent {
        content_ref,
        content_ref_sha256,
    } = &value.content_binding
    {
        let bytes = resolve(content_ref).ok_or(InvalidResidentRecordV1::NonCanonical {
            relation: "immutable content resolution",
        })?;
        if ArtifactDigest::hash_bytes(&bytes) != *content_ref_sha256 {
            return Err(InvalidResidentRecordV1::DigestMismatch {
                family: "immutable content reference",
            });
        }
        let canonical = match value.canonicalization {
            TypedDigestCanonicalizationV1::RawBytes => bytes,
            TypedDigestCanonicalizationV1::Rfc8785Json => {
                let parsed = crate::canon::parse_strict(&bytes).map_err(|_| {
                    InvalidResidentRecordV1::NonCanonical {
                        relation: "immutable JSON content",
                    }
                })?;
                crate::canon::to_canonical_string(&parsed)?.into_bytes()
            }
        };
        if ArtifactDigest::hash_bytes(&canonical) != value.digest {
            return Err(InvalidResidentRecordV1::DigestMismatch {
                family: "typed immutable content",
            });
        }
    }
    Ok(())
}

fn decode_canonical_base64(value: &str) -> Result<Vec<u8>, InvalidResidentRecordV1> {
    if !value.len().is_multiple_of(4) {
        return Err(InvalidResidentRecordV1::NonCanonical {
            relation: "inline base64",
        });
    }
    let bytes = value.as_bytes();
    let padding = bytes.iter().rev().take_while(|byte| **byte == b'=').count();
    if padding > 2 || bytes[..bytes.len().saturating_sub(padding)].contains(&b'=') {
        return Err(InvalidResidentRecordV1::NonCanonical {
            relation: "inline base64",
        });
    }
    let sextet = |byte: u8| -> Option<u8> {
        match byte {
            b'A'..=b'Z' => Some(byte - b'A'),
            b'a'..=b'z' => Some(byte - b'a' + 26),
            b'0'..=b'9' => Some(byte - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    };
    let mut decoded = Vec::with_capacity(value.len() / 4 * 3 - padding);
    for (index, chunk) in bytes.chunks_exact(4).enumerate() {
        let last = index + 1 == bytes.len() / 4;
        let a = sextet(chunk[0]);
        let b = sextet(chunk[1]);
        let c = if chunk[2] == b'=' {
            None
        } else {
            sextet(chunk[2])
        };
        let d = if chunk[3] == b'=' {
            None
        } else {
            sextet(chunk[3])
        };
        let (Some(a), Some(b)) = (a, b) else {
            return Err(InvalidResidentRecordV1::NonCanonical {
                relation: "inline base64",
            });
        };
        if (!last && (c.is_none() || d.is_none())) || (c.is_none() && d.is_some()) {
            return Err(InvalidResidentRecordV1::NonCanonical {
                relation: "inline base64",
            });
        }
        decoded.push((a << 2) | (b >> 4));
        if let Some(c) = c {
            decoded.push((b << 4) | (c >> 2));
            if let Some(d) = d {
                decoded.push((c << 6) | d);
            } else if c & 0b11 != 0 {
                return Err(InvalidResidentRecordV1::NonCanonical {
                    relation: "inline base64",
                });
            }
        } else if b & 0b1111 != 0 {
            return Err(InvalidResidentRecordV1::NonCanonical {
                relation: "inline base64",
            });
        }
    }
    Ok(decoded)
}

/// Closed initial-v1 effect-admission decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EffectAdmissionDecisionV1 {
    /// The exact action is admitted until its bound expiry.
    Admitted,
}

/// Lifecycle state of one immutable conflict-resource claim generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EffectConflictClaimStateV1 {
    /// Exclusively blocks the resource for the admitted action.
    Active,
    /// Releases the resource after a terminal no-reconciliation outcome.
    Released,
    /// Transfers exclusive blocking responsibility to reconciliation custody.
    Transferred,
}

/// Terminal reason attached to a released or transferred conflict claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EffectConflictTerminalReasonV1 {
    /// Native evidence confirmed the intended effect.
    Confirmed,
    /// Independent evidence proved the effect was not applied.
    NotApplied,
    /// Separately admitted cancellation completed before dispatch.
    CancelledBeforeDispatch,
    /// Exclusive blocking responsibility moved to reconciliation custody.
    ReconciliationCustody,
}

resident_signed_record!(
    /// Append-only exclusive claim over one exact conflict-resource key.
    EffectConflictClaimV1, EffectConflictClaimWireV1, EffectConflictClaimPreimageV1,
    canonical_effect_conflict_claim_sha256: EffectConflictClaimDigest,
    EFFECT_CONFLICT_CLAIM_V1_SCHEMA, "effect conflict claim",
    {
        /// Stable identity shared by the complete claim succession chain.
        conflict_claim_id: EffectConflictClaimIdV1,
        /// Positive generation advanced by every release or transfer successor.
        claim_generation: EffectConflictClaimGenerationV1,
        /// Exact prior claim reference, absent only for generation one.
        previous_claim_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::previous_claim_ref`].
        previous_claim_sha256: Option<EffectConflictClaimDigest>,
        /// Schema identity defining the protected conflict-resource key.
        conflict_resource_key_schema_id: SchemaId,
        /// Digest of the protected-key schema bytes.
        conflict_resource_key_schema_sha256: SchemaDigest,
        /// Canonical typed/scoped resource key protected by this claim.
        conflict_resource_key: serde_json::Value,
        /// Principal scope within which the resource is exclusive.
        principal_scope: ResidentPrincipalIdV1,
        /// Controller scope within which the resource is exclusive.
        controller_scope: ResidentControllerIdV1,
        /// Optional relationship boundary carried from the request.
        relationship_scope: Option<RelationshipScopeV1>,
        /// Caller-signed request that owns this claim.
        request_id: ResidentRequestIdV1,
        /// Digest of the exact caller-signed common request.
        canonical_request_sha256: CommonRequestEnvelopeDigest,
        /// Logical action protected by this claim.
        action_id: ActionIdV1,
        /// Digest of the exact action body.
        action_body_sha256: ActionEnvelopeDigest,
        /// Business idempotency key fixed by the action body.
        business_idempotency_key: BusinessIdempotencyKeyV1,
        /// Lifecycle state of this immutable generation.
        claim_state: EffectConflictClaimStateV1,
        /// Principal that holds the active claim or issued the terminal transition.
        holder_principal_id: ResidentPrincipalIdV1,
        /// Terminal reason, absent only while active.
        terminal_reason: Option<EffectConflictTerminalReasonV1>,
        /// Effect receipt supporting confirmed or not-applied release.
        effect_receipt_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::effect_receipt_ref`].
        effect_receipt_sha256: Option<EffectReceiptDigest>,
        /// Separately admitted cancellation authority.
        cancellation_authority_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::cancellation_authority_ref`].
        cancellation_authority_sha256: Option<ArtifactDigest>,
        /// Exclusive reconciliation custody receiving a transfer.
        reconciliation_custody_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::reconciliation_custody_ref`].
        reconciliation_custody_sha256: Option<ReconciliationCustodyDigest>,
        /// Issue time of this immutable claim generation.
        issued_at: UnixTimeV1,
        /// Transfer or release time, absent while active.
        transferred_or_released_at: Option<UnixTimeV1>,
        /// Exact deployment-runtime authority that signed this claim generation.
        deployment_runtime_authority_ref: ResidentRecordRefV1,
        /// Digest of the exact deployment-runtime-authority bytes.
        deployment_runtime_authority_sha256: ArtifactDigest,
        /// Principal under whose authority this claim was issued.
        issuer_principal_id: ResidentPrincipalIdV1,
        /// Key identity used for the claim signature.
        issuer_key_id: ResidentKeyIdV1,
        /// Positive issuer-key epoch checked for the claim.
        issuer_key_epoch: SigningKeyEpochV1
    },
    |value: &EffectConflictClaimPreimageV1| {
        validate_bounded_json("effect conflict claim resource key", &value.conflict_resource_key)?;
        if value.previous_claim_ref.is_some() != value.previous_claim_sha256.is_some()
            || (value.claim_generation.get() == 1) != value.previous_claim_ref.is_none()
            || value.effect_receipt_ref.is_some() != value.effect_receipt_sha256.is_some()
            || value.cancellation_authority_ref.is_some()
                != value.cancellation_authority_sha256.is_some()
            || value.reconciliation_custody_ref.is_some()
                != value.reconciliation_custody_sha256.is_some()
            || value
                .transferred_or_released_at
                .is_some_and(|terminal| terminal < value.issued_at)
        {
            return Err(InvalidResidentRecordV1::EffectState {
                record: "effect conflict claim",
                state: "lineage",
            });
        }
        let active = value.terminal_reason.is_none()
            && value.effect_receipt_ref.is_none()
            && value.cancellation_authority_ref.is_none()
            && value.reconciliation_custody_ref.is_none()
            && value.transferred_or_released_at.is_none();
        let receipt_release = matches!(
            value.terminal_reason,
            Some(
                EffectConflictTerminalReasonV1::Confirmed
                    | EffectConflictTerminalReasonV1::NotApplied
            )
        ) && value.effect_receipt_ref.is_some()
            && value.cancellation_authority_ref.is_none()
            && value.reconciliation_custody_ref.is_none()
            && value.transferred_or_released_at.is_some();
        let cancellation_release = value.terminal_reason
            == Some(EffectConflictTerminalReasonV1::CancelledBeforeDispatch)
            && value.effect_receipt_ref.is_none()
            && value.cancellation_authority_ref.is_some()
            && value.reconciliation_custody_ref.is_none()
            && value.transferred_or_released_at.is_some();
        let custody_transfer = value.terminal_reason
            == Some(EffectConflictTerminalReasonV1::ReconciliationCustody)
            && value.effect_receipt_ref.is_none()
            && value.cancellation_authority_ref.is_none()
            && value.reconciliation_custody_ref.is_some()
            && value.transferred_or_released_at.is_some();
        match value.claim_state {
            EffectConflictClaimStateV1::Active if active => Ok(()),
            EffectConflictClaimStateV1::Released if receipt_release || cancellation_release => Ok(()),
            EffectConflictClaimStateV1::Transferred if custody_transfer => Ok(()),
            _ => Err(InvalidResidentRecordV1::EffectState {
                record: "effect conflict claim",
                state: "active/released/transferred",
            }),
        }
    }
);

/// Resolved request, action, and active claim for effect admission.
pub struct EffectAdmissionContextV1<'a> {
    /// Exact caller-signed request.
    pub request: &'a CommonRequestEnvelopeV1,
    /// Exact admitted action body.
    pub action: &'a ActionEnvelopeV1,
    /// Reference resolving `claim`.
    pub claim_ref: &'a ResidentRecordRefV1,
    /// Exact active conflict claim.
    pub claim: &'a EffectConflictClaimV1,
    /// Exact schema and signer coordinates.
    pub signed: SignedResidentAdmissionContextV1<'a>,
}

/// Resolved parent records for initial conflict-claim admission.
pub struct EffectConflictClaimAdmissionContextV1<'a> {
    /// Exact caller-signed request.
    pub request: &'a CommonRequestEnvelopeV1,
    /// Exact admitted unsigned action body.
    pub action: &'a ActionEnvelopeV1,
    /// Exact schema and signer coordinates.
    pub signed: SignedResidentAdmissionContextV1<'a>,
}

/// Exact active predecessor and signing coordinate shared by a claim transition.
#[derive(Clone, Copy)]
pub struct EffectConflictClaimPredecessorV1<'a> {
    /// Reference resolving `predecessor`.
    pub predecessor_ref: &'a ResidentRecordRefV1,
    /// Exact active predecessor claim.
    pub predecessor: &'a EffectConflictClaimV1,
    /// Exact schema and signer coordinates.
    pub signed: SignedResidentAdmissionContextV1<'a>,
}

/// Closed evidence context authorizing one terminal conflict-claim successor.
pub enum EffectConflictClaimSuccessorContextV1<'a> {
    /// Release supported by an exact terminal effect receipt.
    ReceiptRelease {
        /// Shared exact predecessor lineage.
        predecessor: EffectConflictClaimPredecessorV1<'a>,
        /// Reference resolving `receipt`.
        receipt_ref: &'a ResidentRecordRefV1,
        /// Exact effect receipt supporting release.
        receipt: &'a EffectReceiptV1,
        /// Policy applied to the receipt's reconciliation decision.
        policy_requires_reconciliation: bool,
    },
    /// Release supported by separately resolved cancellation authority.
    CancellationRelease {
        /// Shared exact predecessor lineage.
        predecessor: EffectConflictClaimPredecessorV1<'a>,
        /// Exact immutable cancellation-authority reference.
        cancellation_authority_ref: &'a ResidentRecordRefV1,
        /// Digest of the resolved cancellation-authority bytes.
        cancellation_authority_sha256: &'a ArtifactDigest,
    },
    /// Transfer supported by the exact ambiguity case and custody generation.
    CustodyTransfer {
        /// Shared exact predecessor lineage.
        predecessor: EffectConflictClaimPredecessorV1<'a>,
        /// Reference resolving `case`.
        case_ref: &'a ResidentRecordRefV1,
        /// Exact ambiguity case protected by custody.
        case: &'a ReconciliationCaseV1,
        /// Reference resolving `custody`.
        custody_ref: &'a ResidentRecordRefV1,
        /// Exact receiving custody generation.
        custody: &'a ReconciliationCustodyV1,
    },
}

impl<'a> EffectConflictClaimSuccessorContextV1<'a> {
    fn predecessor(&self) -> EffectConflictClaimPredecessorV1<'a> {
        match self {
            Self::ReceiptRelease { predecessor, .. }
            | Self::CancellationRelease { predecessor, .. }
            | Self::CustodyTransfer { predecessor, .. } => *predecessor,
        }
    }
}

impl EffectConflictClaimV1 {
    /// Admit the initial active claim against the exact caller request, action,
    /// schema, and signing authority.
    ///
    /// # Errors
    ///
    /// Returns an error when structural validation, request/action binding,
    /// schema or signing-authority validation, or signature verification fails.
    pub fn admit_initial_wire<F>(
        wire: EffectConflictClaimWireV1,
        context: &EffectConflictClaimAdmissionContextV1<'_>,
        verify_signature: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnOnce(
            &EffectConflictClaimPreimageV1,
            &EffectConflictClaimDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        Self::admit_wire(wire, |preimage, digest, signature| {
            validate_signed_coordinates(
                "effect conflict claim",
                &preimage.schema_sha256,
                context.signed.expected_schema_sha256,
                &preimage.issuer_principal_id,
                &preimage.issuer_key_id,
                preimage.issuer_key_epoch,
                context.signed.signing_authority,
            )?;
            validate_integrated_runtime_authority_coordinates(
                "effect conflict claim",
                &preimage.deployment_runtime_authority_ref,
                &preimage.deployment_runtime_authority_sha256,
                context.signed.signing_authority,
            )?;
            context.request.verify_action_body(context.action)?;
            let action_digest = context.action.canonical_action_body_sha256()?;
            if preimage.claim_generation.get() != 1
                || preimage.claim_state != EffectConflictClaimStateV1::Active
                || preimage.request_id != *context.request.request_id()
                || preimage.canonical_request_sha256 != *context.request.canonical_request_sha256()
                || preimage.action_id != *context.action.action_id()
                || preimage.action_body_sha256 != action_digest
                || preimage.business_idempotency_key != *context.action.business_idempotency_key()
                || preimage.conflict_resource_key_schema_id
                    != *context.action.conflict_resource_key_schema_id()
                || preimage.conflict_resource_key_schema_sha256
                    != *context.action.conflict_resource_key_schema_sha256()
                || preimage.conflict_resource_key != *context.action.conflict_resource_key()
            {
                return Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "effect conflict claim",
                    invariant: "exact initial request action and active claim lineage",
                });
            }
            verify_signature(preimage, digest, signature)
        })
    }

    /// Admit an exact terminal successor under one closed evidence route.
    ///
    /// Succession requires the exact predecessor reference/digest pair before
    /// the selected evidence route is evaluated. For prior and next claim
    /// generations `g_prior` and `g_next`, the exact successor law is:
    #[doc = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/docs/math/effect-conflict-claim-successor.inline.mathml"
    ))]
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] when signature admission fails, the
    /// generation or predecessor reference is not exact, immutable key/request/
    /// action lineage changes, or issue/terminal time regresses.
    pub fn admit_successor_wire<F>(
        wire: EffectConflictClaimWireV1,
        context: &EffectConflictClaimSuccessorContextV1<'_>,
        verify_signature: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnOnce(
            &EffectConflictClaimPreimageV1,
            &EffectConflictClaimDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let predecessor_context = context.predecessor();
        Self::admit_wire(wire, |preimage, digest, signature| {
            let reject = |invariant| InvalidResidentRecordV1::ContextualAdmission {
                record: "effect conflict claim successor",
                invariant,
            };
            validate_signed_coordinates(
                "effect conflict claim",
                &preimage.schema_sha256,
                predecessor_context.signed.expected_schema_sha256,
                &preimage.issuer_principal_id,
                &preimage.issuer_key_id,
                preimage.issuer_key_epoch,
                predecessor_context.signed.signing_authority,
            )?;
            validate_integrated_runtime_authority_coordinates(
                "effect conflict claim",
                &preimage.deployment_runtime_authority_ref,
                &preimage.deployment_runtime_authority_sha256,
                predecessor_context.signed.signing_authority,
            )?;
            let predecessor = predecessor_context.predecessor;
            if predecessor.claim_state() != &EffectConflictClaimStateV1::Active
                || preimage.claim_state == EffectConflictClaimStateV1::Active
            {
                return Err(reject("active predecessor and terminal successor"));
            }
            if predecessor
                .claim_generation()
                .checked_successor()
                .is_none_or(|next| preimage.claim_generation != next)
            {
                return Err(reject("exact next generation"));
            }
            if preimage.previous_claim_ref.as_ref() != Some(predecessor_context.predecessor_ref)
                || preimage.previous_claim_sha256.as_ref()
                    != Some(predecessor.canonical_effect_conflict_claim_sha256())
            {
                return Err(reject("exact predecessor reference"));
            }
            if preimage.conflict_claim_id != *predecessor.conflict_claim_id()
                || preimage.conflict_resource_key_schema_id
                    != *predecessor.conflict_resource_key_schema_id()
                || preimage.conflict_resource_key_schema_sha256
                    != *predecessor.conflict_resource_key_schema_sha256()
                || preimage.conflict_resource_key != *predecessor.conflict_resource_key()
                || preimage.principal_scope != *predecessor.principal_scope()
                || preimage.controller_scope != *predecessor.controller_scope()
                || preimage.relationship_scope.as_ref() != predecessor.relationship_scope().as_ref()
                || preimage.request_id != *predecessor.request_id()
                || preimage.canonical_request_sha256 != *predecessor.canonical_request_sha256()
                || preimage.action_id != *predecessor.action_id()
                || preimage.action_body_sha256 != *predecessor.action_body_sha256()
                || preimage.business_idempotency_key != *predecessor.business_idempotency_key()
                || preimage.deployment_runtime_authority_ref
                    != *predecessor.deployment_runtime_authority_ref()
                || preimage.deployment_runtime_authority_sha256
                    != *predecessor.deployment_runtime_authority_sha256()
            {
                return Err(reject("immutable key request and action lineage"));
            }
            if preimage.issued_at < *predecessor.issued_at()
                || predecessor
                    .transferred_or_released_at()
                    .is_some_and(|prior| {
                        preimage
                            .transferred_or_released_at
                            .is_none_or(|current| current < prior)
                    })
            {
                return Err(reject("monotone claim time"));
            }
            validate_conflict_claim_transition(preimage, context)?;
            verify_signature(preimage, digest, signature)
        })
    }
}

fn validate_conflict_claim_transition(
    value: &EffectConflictClaimPreimageV1,
    context: &EffectConflictClaimSuccessorContextV1<'_>,
) -> Result<(), InvalidResidentRecordV1> {
    let reject = |invariant| InvalidResidentRecordV1::ContextualAdmission {
        record: "effect conflict claim successor",
        invariant,
    };
    let predecessor = context.predecessor();
    match context {
        EffectConflictClaimSuccessorContextV1::ReceiptRelease {
            receipt_ref,
            receipt,
            policy_requires_reconciliation,
            ..
        } => {
            receipt.verify_reconciliation_policy(*policy_requires_reconciliation)?;
            let reason = match receipt.observed_effect() {
                ObservedEffectV1::Confirmed => EffectConflictTerminalReasonV1::Confirmed,
                ObservedEffectV1::NotApplied => EffectConflictTerminalReasonV1::NotApplied,
                ObservedEffectV1::Unknown | ObservedEffectV1::Partial => {
                    return Err(reject("terminal receipt outcome"));
                }
            };
            let exact = value.claim_state == EffectConflictClaimStateV1::Released
                && value.terminal_reason == Some(reason)
                && value.effect_receipt_ref.as_ref() == Some(*receipt_ref)
                && value.effect_receipt_sha256.as_ref() == Some(receipt.canonical_receipt_sha256())
                && !*receipt.reconciliation_required()
                && receipt.request_binding().request_id() == predecessor.predecessor.request_id()
                && receipt.request_binding().canonical_request_sha256()
                    == predecessor.predecessor.canonical_request_sha256()
                && receipt.action_id() == predecessor.predecessor.action_id()
                && receipt.action_body_sha256() == predecessor.predecessor.action_body_sha256()
                && receipt.conflict_claim_ref() == predecessor.predecessor_ref
                && receipt.conflict_claim_sha256()
                    == predecessor
                        .predecessor
                        .canonical_effect_conflict_claim_sha256()
                && value.holder_principal_id == *predecessor.predecessor.holder_principal_id();
            if !exact {
                return Err(reject("exact terminal receipt evidence"));
            }
        }
        EffectConflictClaimSuccessorContextV1::CancellationRelease {
            cancellation_authority_ref,
            cancellation_authority_sha256,
            ..
        } => {
            let exact = value.claim_state == EffectConflictClaimStateV1::Released
                && value.terminal_reason
                    == Some(EffectConflictTerminalReasonV1::CancelledBeforeDispatch)
                && value.cancellation_authority_ref.as_ref() == Some(*cancellation_authority_ref)
                && value.cancellation_authority_sha256.as_ref()
                    == Some(*cancellation_authority_sha256)
                && value.holder_principal_id == *predecessor.predecessor.holder_principal_id();
            if !exact {
                return Err(reject("exact cancellation authority evidence"));
            }
        }
        EffectConflictClaimSuccessorContextV1::CustodyTransfer {
            case_ref,
            case,
            custody_ref,
            custody,
            ..
        } => {
            let exact = value.claim_state == EffectConflictClaimStateV1::Transferred
                && value.terminal_reason
                    == Some(EffectConflictTerminalReasonV1::ReconciliationCustody)
                && value.reconciliation_custody_ref.as_ref() == Some(*custody_ref)
                && value.reconciliation_custody_sha256.as_ref()
                    == Some(custody.canonical_custody_sha256())
                && custody.case_ref() == *case_ref
                && custody.case_sha256() == case.canonical_case_sha256()
                && custody.custody_id() == case.reserved_custody_id()
                && custody.custody_state() != &ReconciliationCustodyStateV1::Closed
                && custody.conflict_resource_key_schema_id()
                    == predecessor.predecessor.conflict_resource_key_schema_id()
                && custody.conflict_resource_key_schema_sha256()
                    == predecessor
                        .predecessor
                        .conflict_resource_key_schema_sha256()
                && custody.conflict_resource_key()
                    == predecessor.predecessor.conflict_resource_key()
                && case.action_id() == predecessor.predecessor.action_id()
                && case.conflict_resource_key_schema_id()
                    == predecessor.predecessor.conflict_resource_key_schema_id()
                && case.conflict_resource_key_schema_sha256()
                    == predecessor
                        .predecessor
                        .conflict_resource_key_schema_sha256()
                && case.conflict_resource_key() == predecessor.predecessor.conflict_resource_key()
                && value.holder_principal_id == *custody.holder_principal_id();
            if !exact {
                return Err(reject("exact reconciliation case and custody evidence"));
            }
        }
    }
    Ok(())
}

resident_signed_record!(
    /// Pale effect-admission decision bound to active claim and exact policy.
    EffectAdmissionV1, EffectAdmissionWireV1, EffectAdmissionPreimageV1,
    canonical_effect_admission_sha256: EffectAdmissionDigest,
    EFFECT_ADMISSION_V1_SCHEMA, "effect admission",
    {
        /// Stable identity of this effect-admission decision.
        effect_admission_id: EffectAdmissionIdV1,
        /// Logical request whose caller signature was admitted.
        request_id: ResidentRequestIdV1,
        /// Digest of the exact caller-signed common request.
        canonical_request_sha256: CommonRequestEnvelopeDigest,
        /// Stable logical action admitted for dispatch.
        action_id: ActionIdV1,
        /// Digest of the exact effect-specific action body.
        action_body_sha256: ActionEnvelopeDigest,
        /// Schema identity defining the conflict-resource key.
        conflict_resource_key_schema_id: SchemaId,
        /// Digest of the selected conflict-resource-key schema bytes.
        conflict_resource_key_schema_sha256: SchemaDigest,
        /// Canonical typed/scoped key protected before admission returns.
        conflict_resource_key: serde_json::Value,
        /// Durable active conflict claim acquired for this action.
        conflict_claim_ref: ResidentRecordRefV1,
        /// Digest of the exact conflict-claim bytes.
        conflict_claim_sha256: EffectConflictClaimDigest,
        /// Canonically ordered authority evidence checked for admission.
        authority_evidence: Vec<AuthorityEvidenceV1>,
        /// Policy-owned risk classification admitted for this action.
        risk_class: BoundedResidentTextV1,
        /// Immutable effect policy applied by the admission authority.
        effect_policy: ResidentRecordRefV1,
        /// Immutable reconciliation policy governing ambiguous outcomes.
        reconciliation_policy: ResidentRecordRefV1,
        /// Closed initial-v1 admission outcome.
        decision: EffectAdmissionDecisionV1,
        /// Admission issue time in Unix seconds.
        issued_at: UnixTimeV1,
        /// Hard admission expiry in Unix seconds.
        expires_at: UnixTimeV1,
        /// Exact deployment-runtime authority that signed this admission.
        deployment_runtime_authority_ref: ResidentRecordRefV1,
        /// Digest of the exact deployment-runtime-authority bytes.
        deployment_runtime_authority_sha256: ArtifactDigest,
        /// Principal under whose authority admission was issued.
        issuer_principal_id: ResidentPrincipalIdV1,
        /// Key identity used for the admission signature.
        issuer_key_id: ResidentKeyIdV1,
        /// Positive issuer-key epoch checked during admission.
        issuer_key_epoch: SigningKeyEpochV1
    },
    |value: &EffectAdmissionPreimageV1| {
        strictly_sorted("effect admission authority evidence", &value.authority_evidence)?;
        validate_bounded_json("effect conflict-resource key bytes", &value.conflict_resource_key)?;
        validate_bounded_text("effect admission risk class", value.risk_class.as_str())?;
        if value.decision != EffectAdmissionDecisionV1::Admitted
            || value.issued_at >= value.expires_at {
            Err(InvalidResidentRecordV1::EffectState { record: "effect admission", state: "admitted" })
        } else { Ok(()) }
    }
);

impl EffectAdmissionV1 {
    /// Admit an effect decision against the exact caller request, action body,
    /// active conflict claim, schema, and signing authority.
    ///
    /// # Errors
    ///
    /// Returns an error when structural validation, contextual lineage,
    /// schema or signing-authority validation, or signature verification fails.
    pub fn admit_contextual_wire<F>(
        wire: EffectAdmissionWireV1,
        context: &EffectAdmissionContextV1<'_>,
        verify_signature: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnOnce(
            &EffectAdmissionPreimageV1,
            &EffectAdmissionDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        Self::admit_wire(wire, |preimage, digest, signature| {
            validate_signed_coordinates(
                "effect admission",
                &preimage.schema_sha256,
                context.signed.expected_schema_sha256,
                &preimage.issuer_principal_id,
                &preimage.issuer_key_id,
                preimage.issuer_key_epoch,
                context.signed.signing_authority,
            )?;
            validate_integrated_runtime_authority_coordinates(
                "effect admission",
                &preimage.deployment_runtime_authority_ref,
                &preimage.deployment_runtime_authority_sha256,
                context.signed.signing_authority,
            )?;
            context.request.verify_action_body(context.action)?;
            let action_digest = context.action.canonical_action_body_sha256()?;
            let exact = preimage.request_id == *context.request.request_id()
                && preimage.canonical_request_sha256 == *context.request.canonical_request_sha256()
                && preimage.action_id == *context.action.action_id()
                && preimage.action_body_sha256 == action_digest
                && preimage.conflict_claim_ref == *context.claim_ref
                && preimage.conflict_claim_sha256
                    == *context.claim.canonical_effect_conflict_claim_sha256()
                && context.claim.claim_state() == &EffectConflictClaimStateV1::Active
                && context.claim.request_id() == context.request.request_id()
                && context.claim.canonical_request_sha256()
                    == context.request.canonical_request_sha256()
                && context.claim.action_id() == context.action.action_id()
                && context.claim.action_body_sha256() == &action_digest
                && preimage.conflict_resource_key_schema_id
                    == *context.action.conflict_resource_key_schema_id()
                && preimage.conflict_resource_key_schema_sha256
                    == *context.action.conflict_resource_key_schema_sha256()
                && preimage.conflict_resource_key == *context.action.conflict_resource_key()
                && preimage.effect_policy == *context.action.effect_policy()
                && preimage.reconciliation_policy == *context.action.reconciliation_policy()
                && preimage.risk_class == *context.request.risk_class()
                && preimage.issued_at >= context.request.issued_at()
                && preimage.expires_at <= context.request.expires_at();
            let exact = exact
                && preimage.deployment_runtime_authority_ref
                    == *context.claim.deployment_runtime_authority_ref()
                && preimage.deployment_runtime_authority_sha256
                    == *context.claim.deployment_runtime_authority_sha256();
            if !exact {
                return Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "effect admission",
                    invariant: "exact request action claim and policy lineage",
                });
            }
            verify_signature(preimage, digest, signature)
        })
    }
}

resident_signed_context_record!(
    /// Durable intent that must commit before any effect bytes leave Pale.
    EffectDispatchIntentV1, EffectDispatchIntentWireV1, EffectDispatchIntentPreimageV1,
    canonical_dispatch_intent_sha256: EffectDispatchIntentDigest,
    EFFECT_DISPATCH_INTENT_V1_SCHEMA, "effect dispatch intent",
    {
        /// Stable identity of this durable pre-dispatch intent.
        effect_dispatch_intent_id: EffectDispatchIntentIdV1,
        /// Exact effect attempt authorized by this intent.
        attempt_id: CapabilityAttemptIdV1,
        /// Immutable caller-request reference.
        request_ref: ResidentRecordRefV1,
        /// Digest of the exact caller-signed request bytes.
        canonical_request_sha256: CommonRequestEnvelopeDigest,
        /// Stable logical action being attempted.
        action_id: ActionIdV1,
        /// Digest of the exact effect-specific action body.
        action_body_sha256: ActionEnvelopeDigest,
        /// Immutable effect-admission reference.
        effect_admission_ref: ResidentRecordRefV1,
        /// Digest of the exact effect-admission bytes.
        effect_admission_sha256: EffectAdmissionDigest,
        /// Active conflict claim that remains held through dispatch.
        conflict_claim_ref: ResidentRecordRefV1,
        /// Digest of the exact conflict-claim bytes.
        conflict_claim_sha256: EffectConflictClaimDigest,
        /// Immutable selected contract-set reference.
        contract_set_ref: ResidentRecordRefV1,
        /// Digest of the selected contract-set bytes.
        contract_set_sha256: ArtifactDigest,
        /// Exact schema identity of the admitted route binding.
        route_binding_schema_id: SchemaId,
        /// Digest of the exact route-binding schema bytes.
        route_binding_schema_sha256: SchemaDigest,
        /// Immutable route-binding reference.
        route_binding_ref: ResidentRecordRefV1,
        /// Digest of the admitted route-binding bytes.
        route_binding_sha256: ArtifactDigest,
        /// Positive route generation fenced for dispatch.
        route_generation: RouteGenerationV1,
        /// Optional organized-execution fence reference.
        execution_owner_fence_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::execution_owner_fence_ref`].
        execution_owner_fence_sha256: Option<ExecutionOwnerFenceDigest>,
        /// Optional bounded digest of the provider request representation.
        provider_request_digest: Option<TypedDigestV1>,
        /// Preallocated ambiguity-case identity available before dispatch.
        reserved_reconciliation_case_id: ReconciliationCaseIdV1,
        /// Preallocated custody identity paired with the reserved case.
        reserved_reconciliation_custody_id: ReconciliationCustodyIdV1,
        /// Provider-neutral implementation class selected for the attempt.
        implementation_kind: BoundedResidentTextV1,
        /// Exact selected implementation version.
        implementation_version: BoundedResidentTextV1,
        /// Time at which dispatch was durably intended.
        intended_at: UnixTimeV1,
        /// Exact deployment-runtime authority that signed this dispatch intent.
        deployment_runtime_authority_ref: ResidentRecordRefV1,
        /// Digest of the exact deployment-runtime-authority bytes.
        deployment_runtime_authority_sha256: ArtifactDigest,
        /// Principal under whose authority the intent was issued.
        issuer_principal_id: ResidentPrincipalIdV1,
        /// Key identity used for the intent signature.
        issuer_key_id: ResidentKeyIdV1,
        /// Positive issuer-key epoch checked before intent commit.
        issuer_key_epoch: SigningKeyEpochV1
    },
    wire {
        effect_dispatch_intent_id: EffectDispatchIntentIdV1,
        attempt_id: CapabilityAttemptIdV1,
        request_ref: ResidentRecordRefV1,
        canonical_request_sha256: CommonRequestEnvelopeDigest,
        action_id: ActionIdV1,
        action_body_sha256: ActionEnvelopeDigest,
        effect_admission_ref: ResidentRecordRefV1,
        effect_admission_sha256: EffectAdmissionDigest,
        conflict_claim_ref: ResidentRecordRefV1,
        conflict_claim_sha256: EffectConflictClaimDigest,
        contract_set_ref: ResidentRecordRefV1,
        contract_set_sha256: ArtifactDigest,
        route_binding_schema_id: SchemaId,
        route_binding_schema_sha256: SchemaDigest,
        route_binding_ref: ResidentRecordRefV1,
        route_binding_sha256: ArtifactDigest,
        route_generation: RouteGenerationV1,
        execution_owner_fence_ref: Option<ResidentRecordRefV1>,
        execution_owner_fence_sha256: Option<ExecutionOwnerFenceDigest>,
        provider_request_digest: Option<TypedDigestWireV1>,
        reserved_reconciliation_case_id: ReconciliationCaseIdV1,
        reserved_reconciliation_custody_id: ReconciliationCustodyIdV1,
        implementation_kind: BoundedResidentTextV1,
        implementation_version: BoundedResidentTextV1,
        intended_at: UnixTimeV1,
        deployment_runtime_authority_ref: ResidentRecordRefV1,
        deployment_runtime_authority_sha256: ArtifactDigest,
        issuer_principal_id: ResidentPrincipalIdV1,
        issuer_key_id: ResidentKeyIdV1,
        issuer_key_epoch: SigningKeyEpochV1
    },
    |value: &EffectDispatchIntentPreimageV1| {
        validate_bounded_text(
            "effect implementation kind",
            value.implementation_kind.as_str(),
        )?;
        validate_bounded_text(
            "effect implementation version",
            value.implementation_version.as_str(),
        )?;
        if let Some(provider_request) = &value.provider_request_digest {
            validate_typed_digest(provider_request)?;
        }
        if value.execution_owner_fence_ref.is_some() != value.execution_owner_fence_sha256.is_some()
        { Err(InvalidResidentRecordV1::EffectState { record: "effect dispatch intent", state: "binding" }) }
        else { Ok(()) }
    }
);

/// Resolved lineage for dispatch-intent admission.
pub struct EffectDispatchIntentAdmissionContextV1<'a> {
    /// Reference resolving `request`.
    pub request_ref: &'a ResidentRecordRefV1,
    /// Exact caller-signed request.
    pub request: &'a CommonRequestEnvelopeV1,
    /// Exact admitted action body.
    pub action: &'a ActionEnvelopeV1,
    /// Reference resolving `admission`.
    pub admission_ref: &'a ResidentRecordRefV1,
    /// Exact effect admission.
    pub admission: &'a EffectAdmissionV1,
    /// Reference resolving `claim`.
    pub claim_ref: &'a ResidentRecordRefV1,
    /// Exact active conflict claim.
    pub claim: &'a EffectConflictClaimV1,
    /// Exact schema and signer coordinates.
    pub signed: SignedResidentAdmissionContextV1<'a>,
}

impl EffectDispatchIntentV1 {
    /// Admit a durable dispatch intent against its exact request, action,
    /// admission, active claim, typed provider request, and signing authority.
    ///
    /// # Errors
    ///
    /// Returns an error when typed bytes cannot be resolved, structural or
    /// contextual validation fails, or the signature is not valid.
    pub fn admit_contextual_wire<R, S>(
        wire: EffectDispatchIntentWireV1,
        context: &EffectDispatchIntentAdmissionContextV1<'_>,
        mut resolve_immutable: R,
        verify_signature: S,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        R: FnMut(&ResidentRecordRefV1) -> Option<Vec<u8>>,
        S: FnOnce(
            &EffectDispatchIntentPreimageV1,
            &EffectDispatchIntentDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let provider_request_digest = wire
            .provider_request_digest
            .map(|value| TypedDigestV1::admit_wire(value, &mut resolve_immutable))
            .transpose()?;
        let preimage = EffectDispatchIntentPreimageV1 {
            schema_id: wire.schema_id,
            schema_sha256: wire.schema_sha256,
            effect_dispatch_intent_id: wire.effect_dispatch_intent_id,
            attempt_id: wire.attempt_id,
            request_ref: wire.request_ref,
            canonical_request_sha256: wire.canonical_request_sha256,
            action_id: wire.action_id,
            action_body_sha256: wire.action_body_sha256,
            effect_admission_ref: wire.effect_admission_ref,
            effect_admission_sha256: wire.effect_admission_sha256,
            conflict_claim_ref: wire.conflict_claim_ref,
            conflict_claim_sha256: wire.conflict_claim_sha256,
            contract_set_ref: wire.contract_set_ref,
            contract_set_sha256: wire.contract_set_sha256,
            route_binding_schema_id: wire.route_binding_schema_id,
            route_binding_schema_sha256: wire.route_binding_schema_sha256,
            route_binding_ref: wire.route_binding_ref,
            route_binding_sha256: wire.route_binding_sha256,
            route_generation: wire.route_generation,
            execution_owner_fence_ref: wire.execution_owner_fence_ref,
            execution_owner_fence_sha256: wire.execution_owner_fence_sha256,
            provider_request_digest,
            reserved_reconciliation_case_id: wire.reserved_reconciliation_case_id,
            reserved_reconciliation_custody_id: wire.reserved_reconciliation_custody_id,
            implementation_kind: wire.implementation_kind,
            implementation_version: wire.implementation_version,
            intended_at: wire.intended_at,
            deployment_runtime_authority_ref: wire.deployment_runtime_authority_ref,
            deployment_runtime_authority_sha256: wire.deployment_runtime_authority_sha256,
            issuer_principal_id: wire.issuer_principal_id,
            issuer_key_id: wire.issuer_key_id,
            issuer_key_epoch: wire.issuer_key_epoch,
        };
        Self::admit_parts(
            preimage,
            wire.canonical_dispatch_intent_sha256,
            wire.signature,
            |preimage, digest, signature| {
                validate_signed_coordinates(
                    "effect dispatch intent",
                    &preimage.schema_sha256,
                    context.signed.expected_schema_sha256,
                    &preimage.issuer_principal_id,
                    &preimage.issuer_key_id,
                    preimage.issuer_key_epoch,
                    context.signed.signing_authority,
                )?;
                validate_integrated_runtime_authority_coordinates(
                    "effect dispatch intent",
                    &preimage.deployment_runtime_authority_ref,
                    &preimage.deployment_runtime_authority_sha256,
                    context.signed.signing_authority,
                )?;
                context.request.verify_action_body(context.action)?;
                let action_digest = context.action.canonical_action_body_sha256()?;
                let expected_execution_fence = context.request.execution_owner_fence.as_ref();
                let exact = preimage.request_ref == *context.request_ref
                    && preimage.canonical_request_sha256
                        == *context.request.canonical_request_sha256()
                    && preimage.action_id == *context.action.action_id()
                    && preimage.action_body_sha256 == action_digest
                    && preimage.effect_admission_ref == *context.admission_ref
                    && preimage.effect_admission_sha256
                        == *context.admission.canonical_effect_admission_sha256()
                    && context.admission.request_id() == context.request.request_id()
                    && context.admission.action_id() == context.action.action_id()
                    && preimage.conflict_claim_ref == *context.claim_ref
                    && preimage.conflict_claim_sha256
                        == *context.claim.canonical_effect_conflict_claim_sha256()
                    && context.claim.claim_state() == &EffectConflictClaimStateV1::Active
                    && context.admission.conflict_claim_ref() == context.claim_ref
                    && context.admission.conflict_claim_sha256()
                        == context.claim.canonical_effect_conflict_claim_sha256()
                    && preimage.contract_set_ref == context.request.route.contract_set_ref
                    && preimage.contract_set_sha256 == context.request.route.contract_set_sha256
                    && preimage.route_binding_schema_id
                        == context.request.route.route_binding_schema_id
                    && preimage.route_binding_schema_sha256
                        == context.request.route.route_binding_schema_sha256
                    && preimage.route_binding_ref == context.request.route.route_binding_ref
                    && preimage.route_binding_sha256 == context.request.route.route_binding_sha256
                    && preimage.route_generation == context.request.route.route_generation
                    && preimage.execution_owner_fence_ref.as_ref()
                        == expected_execution_fence.map(|fence| &fence.execution_fence_ref)
                    && preimage.execution_owner_fence_sha256.as_ref()
                        == expected_execution_fence.map(|fence| &fence.execution_fence_sha256)
                    && preimage.deployment_runtime_authority_ref
                        == *context.admission.deployment_runtime_authority_ref()
                    && preimage.deployment_runtime_authority_sha256
                        == *context.admission.deployment_runtime_authority_sha256()
                    && preimage.deployment_runtime_authority_ref
                        == *context.claim.deployment_runtime_authority_ref()
                    && preimage.deployment_runtime_authority_sha256
                        == *context.claim.deployment_runtime_authority_sha256();
                if !exact {
                    return Err(InvalidResidentRecordV1::ContextualAdmission {
                        record: "effect dispatch intent",
                        invariant: "exact request action admission and active claim lineage",
                    });
                }
                verify_signature(preimage, digest, signature)
            },
        )
    }
}

/// Native effect observation reported by an owner after dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ObservedEffectV1 {
    /// Native observation confirms the intended postcondition.
    Confirmed,
    /// Independent bounded evidence proves no effect was applied.
    NotApplied,
    /// Observation cannot prove application or absence.
    Unknown,
    /// Some but not all intended effects are observed.
    Partial,
}

resident_signed_context_record!(
    /// Signed native effect receipt; provider return alone cannot confirm effect.
    EffectReceiptV1, EffectReceiptWireV1, EffectReceiptPreimageV1,
    canonical_receipt_sha256: EffectReceiptDigest,
    EFFECT_RECEIPT_V1_SCHEMA, "effect receipt",
    {
        /// Immutable common-request admission lineage copied into the receipt.
        request_binding: RequestBindingV1,
        /// Stable logical action whose native outcome was observed.
        action_id: ActionIdV1,
        /// Exact effect attempt reported by this receipt.
        attempt_id: CapabilityAttemptIdV1,
        /// Durable intent that preceded all effect bytes.
        effect_dispatch_intent_ref: ResidentRecordRefV1,
        /// Digest of the exact dispatch-intent bytes.
        effect_dispatch_intent_sha256: EffectDispatchIntentDigest,
        /// Principal scope in which the effect was attempted.
        principal_scope: ResidentPrincipalIdV1,
        /// Controller scope in which the effect was attempted.
        controller_scope: ResidentControllerIdV1,
        /// Optional relationship boundary carried from admission.
        relationship_scope: Option<RelationshipScopeV1>,
        /// Provider-neutral transport outcome, distinct from native effect state.
        transport_status: BoundedResidentTextV1,
        /// Native effect state supported by bounded evidence.
        observed_effect: ObservedEffectV1,
        /// Optional owner-native reference actually returned by the effect boundary.
        external_reference: Option<ResidentRecordRefV1>,
        /// Typed observed postcondition required for confirmed and partial effects.
        observed_postcondition_digest: Option<TypedDigestV1>,
        /// Independent bounded evidence proving the effect was absent.
        absence_evidence_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::absence_evidence_ref`].
        absence_evidence_sha256: Option<ArtifactDigest>,
        /// Digest of the immutable action body authorized for this attempt.
        action_body_sha256: ActionEnvelopeDigest,
        /// Immutable effect-admission reference.
        effect_admission_ref: ResidentRecordRefV1,
        /// Digest of the exact effect-admission bytes.
        effect_admission_sha256: EffectAdmissionDigest,
        /// Conflict claim held while the attempt executed.
        conflict_claim_ref: ResidentRecordRefV1,
        /// Digest of the exact conflict-claim bytes.
        conflict_claim_sha256: EffectConflictClaimDigest,
        /// Optional typed digest of provider request bytes or an admitted projection.
        provider_request_digest: Option<TypedDigestV1>,
        /// Optional typed digest of response bytes actually received.
        response_digest: Option<TypedDigestV1>,
        /// Provider-neutral implementation class that executed the attempt.
        implementation_kind: BoundedResidentTextV1,
        /// Exact implementation version that executed the attempt.
        implementation_version: BoundedResidentTextV1,
        /// Whether policy and observed state require a durable ambiguity case.
        reconciliation_required: bool,
        /// Preallocated case identity, present exactly when reconciliation is required.
        reconciliation_case_id: Option<ReconciliationCaseIdV1>,
        /// Native attempt start time in Unix seconds.
        started_at: UnixTimeV1,
        /// Native attempt completion-observation time in Unix seconds.
        completed_at: UnixTimeV1,
        /// Exact deployment-runtime authority under which this record was issued.
        deployment_runtime_authority_ref: ResidentRecordRefV1,
        /// Digest of the exact deployment-runtime authority bytes.
        deployment_runtime_authority_sha256: ArtifactDigest,
        /// Principal under whose authority the receipt was issued.
        issuer_principal_id: ResidentPrincipalIdV1,
        /// Key identity used for the receipt signature.
        issuer_key_id: ResidentKeyIdV1,
        /// Positive issuer-key epoch checked for the receipt.
        issuer_key_epoch: SigningKeyEpochV1
    },
    wire {
        request_binding: RequestBindingV1,
        action_id: ActionIdV1,
        attempt_id: CapabilityAttemptIdV1,
        effect_dispatch_intent_ref: ResidentRecordRefV1,
        effect_dispatch_intent_sha256: EffectDispatchIntentDigest,
        principal_scope: ResidentPrincipalIdV1,
        controller_scope: ResidentControllerIdV1,
        relationship_scope: Option<RelationshipScopeV1>,
        transport_status: BoundedResidentTextV1,
        observed_effect: ObservedEffectV1,
        external_reference: Option<ResidentRecordRefV1>,
        observed_postcondition_digest: Option<TypedDigestWireV1>,
        absence_evidence_ref: Option<ResidentRecordRefV1>,
        absence_evidence_sha256: Option<ArtifactDigest>,
        action_body_sha256: ActionEnvelopeDigest,
        effect_admission_ref: ResidentRecordRefV1,
        effect_admission_sha256: EffectAdmissionDigest,
        conflict_claim_ref: ResidentRecordRefV1,
        conflict_claim_sha256: EffectConflictClaimDigest,
        provider_request_digest: Option<TypedDigestWireV1>,
        response_digest: Option<TypedDigestWireV1>,
        implementation_kind: BoundedResidentTextV1,
        implementation_version: BoundedResidentTextV1,
        reconciliation_required: bool,
        reconciliation_case_id: Option<ReconciliationCaseIdV1>,
        started_at: UnixTimeV1,
        completed_at: UnixTimeV1,
        deployment_runtime_authority_ref: ResidentRecordRefV1,
        deployment_runtime_authority_sha256: ArtifactDigest,
        issuer_principal_id: ResidentPrincipalIdV1,
        issuer_key_id: ResidentKeyIdV1,
        issuer_key_epoch: SigningKeyEpochV1
    },
    |value: &EffectReceiptPreimageV1| {
        value.request_binding.verify()?;
        validate_bounded_text("effect transport status", value.transport_status.as_str())?;
        validate_bounded_text(
            "effect implementation kind",
            value.implementation_kind.as_str(),
        )?;
        validate_bounded_text(
            "effect implementation version",
            value.implementation_version.as_str(),
        )?;
        for digest in [
            value.observed_postcondition_digest.as_ref(),
            value.provider_request_digest.as_ref(),
            value.response_digest.as_ref(),
        ].into_iter().flatten() {
            validate_typed_digest(digest)?;
        }
        if value.started_at > value.completed_at
            || value.reconciliation_required != value.reconciliation_case_id.is_some()
            || value.absence_evidence_ref.is_some() != value.absence_evidence_sha256.is_some()
        { return Err(InvalidResidentRecordV1::EffectState { record: "effect receipt", state: "timeline/evidence" }); }
        let state_ok = match value.observed_effect {
            ObservedEffectV1::Confirmed => {
                value.observed_postcondition_digest.is_some()
                    && value.absence_evidence_ref.is_none()
            }
            ObservedEffectV1::NotApplied => {
                value.external_reference.is_none()
                    && value.observed_postcondition_digest.is_none()
                    && value.absence_evidence_ref.is_some()
            }
            ObservedEffectV1::Unknown => {
                value.external_reference.is_none()
                    && value.observed_postcondition_digest.is_none()
                    && value.response_digest.is_none()
                    && value.absence_evidence_ref.is_none()
                    && value.reconciliation_required
            }
            ObservedEffectV1::Partial => {
                value.observed_postcondition_digest.is_some()
                    && value.absence_evidence_ref.is_none()
                    && value.reconciliation_required
            }
        };
        if state_ok { Ok(()) } else { Err(InvalidResidentRecordV1::EffectState { record: "effect receipt", state: "observed effect" }) }
    }
);

/// Resolved lineage and policy for effect-receipt admission.
pub struct EffectReceiptAdmissionContextV1<'a> {
    /// Exact caller-signed request.
    pub request: &'a CommonRequestEnvelopeV1,
    /// Complete owner-resolved request binding expected in the receipt.
    pub request_binding: &'a RequestBindingV1,
    /// Exact admitted action body.
    pub action: &'a ActionEnvelopeV1,
    /// Reference resolving `dispatch`.
    pub dispatch_ref: &'a ResidentRecordRefV1,
    /// Exact dispatch intent.
    pub dispatch: &'a EffectDispatchIntentV1,
    /// Reference resolving `admission`.
    pub admission_ref: &'a ResidentRecordRefV1,
    /// Exact effect admission.
    pub admission: &'a EffectAdmissionV1,
    /// Reference resolving `claim`.
    pub claim_ref: &'a ResidentRecordRefV1,
    /// Exact conflict claim held through dispatch.
    pub claim: &'a EffectConflictClaimV1,
    /// Whether admitted policy requires reconciliation for a definite outcome.
    pub policy_requires_reconciliation: bool,
    /// Exact schema and signer coordinates.
    pub signed: SignedResidentAdmissionContextV1<'a>,
}

impl EffectReceiptV1 {
    /// Admit an effect receipt against the exact dispatch, request, action,
    /// effect admission, conflict claim, typed evidence, and signing authority.
    ///
    /// # Errors
    ///
    /// Returns an error when typed bytes cannot be resolved, structural or
    /// contextual validation fails, or the signature is not valid.
    pub fn admit_contextual_wire<R, S>(
        wire: EffectReceiptWireV1,
        context: &EffectReceiptAdmissionContextV1<'_>,
        mut resolve_immutable: R,
        verify_signature: S,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        R: FnMut(&ResidentRecordRefV1) -> Option<Vec<u8>>,
        S: FnOnce(
            &EffectReceiptPreimageV1,
            &EffectReceiptDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let observed_postcondition_digest = wire
            .observed_postcondition_digest
            .map(|value| TypedDigestV1::admit_wire(value, &mut resolve_immutable))
            .transpose()?;
        let provider_request_digest = wire
            .provider_request_digest
            .map(|value| TypedDigestV1::admit_wire(value, &mut resolve_immutable))
            .transpose()?;
        let response_digest = wire
            .response_digest
            .map(|value| TypedDigestV1::admit_wire(value, &mut resolve_immutable))
            .transpose()?;
        let preimage = EffectReceiptPreimageV1 {
            schema_id: wire.schema_id,
            schema_sha256: wire.schema_sha256,
            request_binding: wire.request_binding,
            action_id: wire.action_id,
            attempt_id: wire.attempt_id,
            effect_dispatch_intent_ref: wire.effect_dispatch_intent_ref,
            effect_dispatch_intent_sha256: wire.effect_dispatch_intent_sha256,
            principal_scope: wire.principal_scope,
            controller_scope: wire.controller_scope,
            relationship_scope: wire.relationship_scope,
            transport_status: wire.transport_status,
            observed_effect: wire.observed_effect,
            external_reference: wire.external_reference,
            observed_postcondition_digest,
            absence_evidence_ref: wire.absence_evidence_ref,
            absence_evidence_sha256: wire.absence_evidence_sha256,
            action_body_sha256: wire.action_body_sha256,
            effect_admission_ref: wire.effect_admission_ref,
            effect_admission_sha256: wire.effect_admission_sha256,
            conflict_claim_ref: wire.conflict_claim_ref,
            conflict_claim_sha256: wire.conflict_claim_sha256,
            provider_request_digest,
            response_digest,
            implementation_kind: wire.implementation_kind,
            implementation_version: wire.implementation_version,
            reconciliation_required: wire.reconciliation_required,
            reconciliation_case_id: wire.reconciliation_case_id,
            started_at: wire.started_at,
            completed_at: wire.completed_at,
            deployment_runtime_authority_ref: wire.deployment_runtime_authority_ref,
            deployment_runtime_authority_sha256: wire.deployment_runtime_authority_sha256,
            issuer_principal_id: wire.issuer_principal_id,
            issuer_key_id: wire.issuer_key_id,
            issuer_key_epoch: wire.issuer_key_epoch,
        };
        let value = Self::admit_parts(
            preimage,
            wire.canonical_receipt_sha256,
            wire.signature,
            |preimage, digest, signature| {
                validate_signed_coordinates(
                    "effect receipt",
                    &preimage.schema_sha256,
                    context.signed.expected_schema_sha256,
                    &preimage.issuer_principal_id,
                    &preimage.issuer_key_id,
                    preimage.issuer_key_epoch,
                    context.signed.signing_authority,
                )?;
                validate_integrated_runtime_authority_coordinates(
                    "effect receipt",
                    &preimage.deployment_runtime_authority_ref,
                    &preimage.deployment_runtime_authority_sha256,
                    context.signed.signing_authority,
                )?;
                context.request.verify_action_body(context.action)?;
                validate_exact_request_binding(
                    &preimage.request_binding,
                    context.request_binding,
                    "effect receipt",
                )?;
                let action_digest = context.action.canonical_action_body_sha256()?;
                let expected_case = preimage
                    .reconciliation_required
                    .then_some(context.dispatch.reserved_reconciliation_case_id());
                let exact = preimage.request_binding.request_id() == context.request.request_id()
                    && preimage.request_binding.canonical_request_sha256()
                        == context.request.canonical_request_sha256()
                    && preimage.action_id == *context.action.action_id()
                    && preimage.action_body_sha256 == action_digest
                    && preimage.effect_dispatch_intent_ref == *context.dispatch_ref
                    && preimage.effect_dispatch_intent_sha256
                        == *context.dispatch.canonical_dispatch_intent_sha256()
                    && context.dispatch.canonical_request_sha256()
                        == context.request.canonical_request_sha256()
                    && preimage.attempt_id == *context.dispatch.attempt_id()
                    && preimage.effect_admission_ref == *context.admission_ref
                    && preimage.effect_admission_sha256
                        == *context.admission.canonical_effect_admission_sha256()
                    && preimage.conflict_claim_ref == *context.claim_ref
                    && preimage.conflict_claim_sha256
                        == *context.claim.canonical_effect_conflict_claim_sha256()
                    && context.dispatch.effect_admission_ref() == context.admission_ref
                    && context.dispatch.conflict_claim_ref() == context.claim_ref
                    && preimage.principal_scope == context.request.scope.principal_scope
                    && preimage.controller_scope == context.request.scope.controller_scope
                    && preimage.relationship_scope == context.request.scope.relationship_scope
                    && preimage.implementation_kind == *context.dispatch.implementation_kind()
                    && preimage.implementation_version
                        == *context.dispatch.implementation_version()
                    && preimage.provider_request_digest.as_ref()
                        == context.dispatch.provider_request_digest().as_ref()
                    && preimage.reconciliation_case_id.as_ref() == expected_case;
                if !exact {
                    return Err(InvalidResidentRecordV1::ContextualAdmission {
                        record: "effect receipt",
                        invariant: "exact request action dispatch admission and claim lineage",
                    });
                }
                verify_signature(preimage, digest, signature)
            },
        )?;
        value.verify_reconciliation_policy(context.policy_requires_reconciliation)?;
        Ok(value)
    }

    /// Verify the receipt's reconciliation decision against admitted policy.
    ///
    /// [`ObservedEffectV1::Unknown`] and [`ObservedEffectV1::Partial`] always
    /// enter reconciliation. Confirmed and not-applied observations follow the
    /// selected policy instead; the structural record cannot infer that policy
    /// from its own bytes.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] when the recorded reconciliation
    /// decision contradicts the admitted policy and observed-effect state.
    pub fn verify_reconciliation_policy(
        &self,
        policy_requires_reconciliation: bool,
    ) -> Result<(), InvalidResidentRecordV1> {
        let required = match self.observed_effect {
            ObservedEffectV1::Unknown | ObservedEffectV1::Partial => true,
            ObservedEffectV1::Confirmed | ObservedEffectV1::NotApplied => {
                policy_requires_reconciliation
            }
        };
        if self.reconciliation_required == required {
            Ok(())
        } else {
            Err(InvalidResidentRecordV1::EffectState {
                record: "effect receipt",
                state: "reconciliation policy",
            })
        }
    }
}
