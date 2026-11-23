//! Exact external-action body contract carried by a common request.

use serde::{Deserialize, Serialize};

use super::*;
use crate::records::*;

/// Public schema root for the unsigned action-envelope wire body.
pub const ACTION_ENVELOPE_V1_SCHEMA_ID: &str = "action-envelope.v1";
/// Schema identity selected by a common request carrying an action body.
pub const ACTION_ENVELOPE_V1_SCHEMA: &str = "pale/action-envelope.v1";

/// Immutable action-payload reference with its exact owner-retained byte digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ImmutableActionPayloadRefV1 {
    /// Immutable payload reference resolved through its native owner.
    pub payload_ref: ResidentRecordRefV1,
    /// SHA-256 of the exact referenced payload bytes.
    pub payload_ref_sha256: ArtifactDigest,
}

/// Candidate action body validated into [`ActionEnvelopeV1`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ActionEnvelopePreimageV1 {
    /// Stable logical action identity.
    pub action_id: ActionIdV1,
    /// Native system that owns the target effect.
    pub target_system: ActionTargetSystemV1,
    /// Exact target identity interpreted by that system.
    pub target_identity: ActionTargetIdentityV1,
    /// Schema identity defining the conflict-resource key.
    pub conflict_resource_key_schema_id: SchemaId,
    /// Digest of the conflict-resource-key schema bytes.
    pub conflict_resource_key_schema_sha256: SchemaDigest,
    /// Canonical typed/scoped conflict-resource key.
    pub conflict_resource_key: serde_json::Value,
    /// Semantic operation requested from the target owner.
    pub operation: ActionOperationV1,
    /// Business idempotency key fixed across exact logical replay.
    pub business_idempotency_key: BusinessIdempotencyKeyV1,
    /// Optional exact target precondition.
    pub expected_precondition_digest: Option<TypedDigestV1>,
    /// Schema identity defining the action payload.
    pub payload_schema_id: SchemaId,
    /// Digest of the action-payload schema bytes.
    pub payload_schema_sha256: SchemaDigest,
    /// Digest of the selected canonical or immutable payload bytes.
    pub payload_hash: ArtifactDigest,
    /// Bounded canonical JSON payload, mutually exclusive with the immutable reference.
    pub inline_payload: Option<serde_json::Value>,
    /// Owner-retained immutable payload, mutually exclusive with inline JSON.
    pub immutable_payload_ref: Option<ImmutableActionPayloadRefV1>,
    /// Immutable policy governing target-effect admission.
    pub effect_policy: ResidentRecordRefV1,
    /// Immutable policy governing ambiguous-outcome reconciliation.
    pub reconciliation_policy: ResidentRecordRefV1,
}

/// Untrusted structural wire shape for [`ActionEnvelopeV1`].
///
/// Parsing establishes only field shape. Call [`ActionEnvelopeV1::admit_wire`]
/// with an owner-native immutable-payload resolver before semantic use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ActionEnvelopeWireV1 {
    /// Stable logical action identity.
    action_id: ActionIdV1,
    /// Native system that owns the target effect.
    target_system: ActionTargetSystemV1,
    /// Exact target identity interpreted by that system.
    target_identity: ActionTargetIdentityV1,
    /// Schema identity defining the conflict-resource key.
    conflict_resource_key_schema_id: SchemaId,
    /// Digest of the conflict-resource-key schema bytes.
    conflict_resource_key_schema_sha256: SchemaDigest,
    /// Canonical typed/scoped conflict-resource key.
    conflict_resource_key: serde_json::Value,
    /// Semantic operation requested from the target owner.
    operation: ActionOperationV1,
    /// Business idempotency key fixed across exact logical replay.
    business_idempotency_key: BusinessIdempotencyKeyV1,
    /// Optional exact target precondition.
    expected_precondition_digest: Option<TypedDigestWireV1>,
    /// Schema identity defining the action payload.
    payload_schema_id: SchemaId,
    /// Digest of the action-payload schema bytes.
    payload_schema_sha256: SchemaDigest,
    /// Digest of the selected canonical or immutable payload bytes.
    payload_hash: ArtifactDigest,
    /// Bounded canonical JSON payload.
    inline_payload: Option<serde_json::Value>,
    /// Owner-retained immutable payload reference.
    immutable_payload_ref: Option<ImmutableActionPayloadRefV1>,
    /// Immutable policy governing target-effect admission.
    effect_policy: ResidentRecordRefV1,
    /// Immutable policy governing ambiguous-outcome reconciliation.
    reconciliation_policy: ResidentRecordRefV1,
}

/// Semantically admitted exact external-action body.
///
/// The caller signature and common request axes live only in
/// [`CommonRequestEnvelopeV1`]; this type does not duplicate them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ActionEnvelopeV1 {
    /// Stable logical action identity.
    action_id: ActionIdV1,
    /// Native system that owns the target effect.
    target_system: ActionTargetSystemV1,
    /// Exact target identity interpreted by that system.
    target_identity: ActionTargetIdentityV1,
    /// Schema identity defining the conflict-resource key.
    conflict_resource_key_schema_id: SchemaId,
    /// Digest of the conflict-resource-key schema bytes.
    conflict_resource_key_schema_sha256: SchemaDigest,
    /// Canonical typed/scoped conflict-resource key.
    conflict_resource_key: serde_json::Value,
    /// Semantic operation requested from the target owner.
    operation: ActionOperationV1,
    /// Business idempotency key fixed across exact logical replay.
    business_idempotency_key: BusinessIdempotencyKeyV1,
    /// Optional exact target precondition.
    expected_precondition_digest: Option<TypedDigestV1>,
    /// Schema identity defining the action payload.
    payload_schema_id: SchemaId,
    /// Digest of the action-payload schema bytes.
    payload_schema_sha256: SchemaDigest,
    /// Digest of the selected canonical or immutable payload bytes.
    payload_hash: ArtifactDigest,
    /// Bounded canonical JSON payload.
    inline_payload: Option<serde_json::Value>,
    /// Owner-retained immutable payload reference.
    immutable_payload_ref: Option<ImmutableActionPayloadRefV1>,
    /// Immutable policy governing target-effect admission.
    effect_policy: ResidentRecordRefV1,
    /// Immutable policy governing ambiguous-outcome reconciliation.
    reconciliation_policy: ResidentRecordRefV1,
}

impl ActionEnvelopeV1 {
    /// Admit an untrusted action body with bounded immutable-payload resolution.
    ///
    /// The resolver is called only for the immutable representation and must
    /// return the exact owner-retained bytes for the supplied reference.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] when JSON bounds, typed digests,
    /// representation exclusivity, payload resolution, or payload hashes fail.
    pub fn admit_wire<F>(
        wire: ActionEnvelopeWireV1,
        mut resolve_immutable: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnMut(&ResidentRecordRefV1) -> Option<Vec<u8>>,
    {
        let expected_precondition_digest = wire
            .expected_precondition_digest
            .map(|digest| TypedDigestV1::admit_wire(digest, &mut resolve_immutable))
            .transpose()?;
        let preimage = ActionEnvelopePreimageV1 {
            action_id: wire.action_id,
            target_system: wire.target_system,
            target_identity: wire.target_identity,
            conflict_resource_key_schema_id: wire.conflict_resource_key_schema_id,
            conflict_resource_key_schema_sha256: wire.conflict_resource_key_schema_sha256,
            conflict_resource_key: wire.conflict_resource_key,
            operation: wire.operation,
            business_idempotency_key: wire.business_idempotency_key,
            expected_precondition_digest,
            payload_schema_id: wire.payload_schema_id,
            payload_schema_sha256: wire.payload_schema_sha256,
            payload_hash: wire.payload_hash,
            inline_payload: wire.inline_payload,
            immutable_payload_ref: wire.immutable_payload_ref,
            effect_policy: wire.effect_policy,
            reconciliation_policy: wire.reconciliation_policy,
        };
        Self::admit(preimage, resolve_immutable)
    }

    /// Validate a trusted producer preimage with the same payload admission law.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] under the same conditions as
    /// [`Self::admit_wire`].
    pub fn admit<F>(
        preimage: ActionEnvelopePreimageV1,
        resolve_immutable: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnMut(&ResidentRecordRefV1) -> Option<Vec<u8>>,
    {
        validate_bounded_json(
            "action conflict-resource key",
            &preimage.conflict_resource_key,
        )?;
        if let Some(expected) = &preimage.expected_precondition_digest {
            validate_typed_digest(expected)?;
        }
        validate_action_payload(&preimage, resolve_immutable)?;
        let ActionEnvelopePreimageV1 {
            action_id,
            target_system,
            target_identity,
            conflict_resource_key_schema_id,
            conflict_resource_key_schema_sha256,
            conflict_resource_key,
            operation,
            business_idempotency_key,
            expected_precondition_digest,
            payload_schema_id,
            payload_schema_sha256,
            payload_hash,
            inline_payload,
            immutable_payload_ref,
            effect_policy,
            reconciliation_policy,
        } = preimage;
        Ok(Self {
            action_id,
            target_system,
            target_identity,
            conflict_resource_key_schema_id,
            conflict_resource_key_schema_sha256,
            conflict_resource_key,
            operation,
            business_idempotency_key,
            expected_precondition_digest,
            payload_schema_id,
            payload_schema_sha256,
            payload_hash,
            inline_payload,
            immutable_payload_ref,
            effect_policy,
            reconciliation_policy,
        })
    }

    /// Compute the exact digest placed in the common request body.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] if canonical JSON encoding fails.
    pub fn canonical_action_body_sha256(
        &self,
    ) -> Result<ActionEnvelopeDigest, InvalidResidentRecordV1> {
        Ok(ActionEnvelopeDigest::hash_canonical(self)?)
    }

    /// Borrow the stable logical action identity.
    #[must_use]
    pub const fn action_id(&self) -> &ActionIdV1 {
        &self.action_id
    }

    /// Borrow the native system that owns the requested effect.
    #[must_use]
    pub const fn target_system(&self) -> &ActionTargetSystemV1 {
        &self.target_system
    }

    /// Borrow the exact owner-native target identity.
    #[must_use]
    pub const fn target_identity(&self) -> &ActionTargetIdentityV1 {
        &self.target_identity
    }

    /// Borrow the semantic operation requested from the target owner.
    #[must_use]
    pub const fn operation(&self) -> &ActionOperationV1 {
        &self.operation
    }

    /// Borrow the business idempotency key.
    #[must_use]
    pub const fn business_idempotency_key(&self) -> &BusinessIdempotencyKeyV1 {
        &self.business_idempotency_key
    }

    /// Borrow the schema identity of the protected conflict key.
    #[must_use]
    pub const fn conflict_resource_key_schema_id(&self) -> &SchemaId {
        &self.conflict_resource_key_schema_id
    }
    /// Borrow the selected protected-key schema digest.
    #[must_use]
    pub const fn conflict_resource_key_schema_sha256(&self) -> &SchemaDigest {
        &self.conflict_resource_key_schema_sha256
    }
    /// Borrow the canonical protected conflict key.
    #[must_use]
    pub const fn conflict_resource_key(&self) -> &serde_json::Value {
        &self.conflict_resource_key
    }
    /// Borrow the selected effect policy.
    #[must_use]
    pub const fn effect_policy(&self) -> &ResidentRecordRefV1 {
        &self.effect_policy
    }
    /// Borrow the selected reconciliation policy.
    #[must_use]
    pub const fn reconciliation_policy(&self) -> &ResidentRecordRefV1 {
        &self.reconciliation_policy
    }
}

fn validate_action_payload<F>(
    value: &ActionEnvelopePreimageV1,
    mut resolve_immutable: F,
) -> Result<(), InvalidResidentRecordV1>
where
    F: FnMut(&ResidentRecordRefV1) -> Option<Vec<u8>>,
{
    match (&value.inline_payload, &value.immutable_payload_ref) {
        (Some(inline), None) => {
            let canonical = validate_bounded_json("action inline payload", inline)?;
            if ArtifactDigest::hash_bytes(canonical.as_bytes()) == value.payload_hash {
                Ok(())
            } else {
                Err(InvalidResidentRecordV1::DigestMismatch {
                    family: "action inline payload",
                })
            }
        }
        (None, Some(immutable)) => {
            let bytes = resolve_immutable(&immutable.payload_ref).ok_or(
                InvalidResidentRecordV1::ContextualAdmission {
                    record: "action envelope",
                    invariant: "immutable payload resolution",
                },
            )?;
            validate_bounded_content_bytes("action immutable payload bytes", &bytes)?;
            let observed = ArtifactDigest::hash_bytes(&bytes);
            if observed == immutable.payload_ref_sha256 && observed == value.payload_hash {
                Ok(())
            } else {
                Err(InvalidResidentRecordV1::DigestMismatch {
                    family: "action immutable payload",
                })
            }
        }
        _ => Err(InvalidResidentRecordV1::EffectState {
            record: "action envelope",
            state: "exactly one payload representation",
        }),
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/records/resident/action.rs"]
mod tests;
