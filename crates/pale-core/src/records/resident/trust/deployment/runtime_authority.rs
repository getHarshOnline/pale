//! Root-delegated online authority for one admitted deployment runtime.
//!
//! An authority is scoped to one immutable deployment binding and one control-
//! store incarnation. Succession preserves that scope and advances the
//! authority epoch exactly once. The root-authorized successor may replace the
//! permitted record-schema set; the predecessor allowlist is not ambient or
//! hereditary authority.
//!
//! Admission here is a pure contract morphism. It verifies record structure,
//! immutable scope, lineage, and the caller-resolved principal-key signature;
//! it does not resolve a trust checkpoint, mutate a store, activate a runtime,
//! issue a session, or cause an effect.

use super::*;
use serde::Serialize;

/// Public schema root for deployment-runtime-authority wires.
pub const DEPLOYMENT_RUNTIME_AUTHORITY_V1_SCHEMA_ID: &str = "deployment-runtime-authority.v1";
/// Wire schema carried by every deployment runtime authority.
pub const DEPLOYMENT_RUNTIME_AUTHORITY_V1_SCHEMA: &str = "pale/deployment-runtime-authority.v1";
/// Signature domain for [`DeploymentRuntimeAuthorityV1`].
pub const DEPLOYMENT_RUNTIME_AUTHORITY_V1_SIGNATURE_DOMAIN: &[u8] =
    b"PALE_DEPLOYMENT_RUNTIME_AUTHORITY_V1\0";

resident_signed_record!(
    /// Principal-root-delegated signer for Pale-native runtime records in one deployment.
    DeploymentRuntimeAuthorityV1,
    DeploymentRuntimeAuthorityWireV1,
    DeploymentRuntimeAuthorityPreimageV1,
    canonical_deployment_runtime_authority_sha256: DeploymentRuntimeAuthorityDigest,
    DEPLOYMENT_RUNTIME_AUTHORITY_V1_SCHEMA,
    "deployment runtime authority",
    {
        /// Stable identity shared by this authority lineage.
        runtime_authority_id: DeploymentRuntimeAuthorityIdV1,
        /// Principal that owns the admitted deployment and control store.
        principal_id: ResidentPrincipalIdV1,
        /// Exact immutable deployment binding covered by this authority.
        deployment_binding_ref: ResidentRecordRefV1,
        /// Canonical digest of the referenced deployment binding.
        deployment_binding_sha256: DeploymentBindingDigest,
        /// Exact logical deployment control store covered by this authority.
        control_store_id: ControlStoreIdV1,
        /// Exact control-store incarnation covered by this authority.
        control_incarnation_id: ControlIncarnationIdV1,
        /// Stable identity of the delegated online signing key.
        subject_key_id: ResidentKeyIdV1,
        /// Positive epoch of the delegated online signing key.
        subject_key_epoch: SigningKeyEpochV1,
        /// Exact Ed25519 public key delegated for runtime-record signatures.
        subject_public_key: Ed25519PublicKeyV1,
        /// Non-empty canonical set of record schemas this key may sign.
        permitted_record_schema_ids: Vec<SchemaId>,
        /// Positive succession epoch of this authority lineage.
        authority_epoch: DeploymentRuntimeAuthorityEpochV1,
        /// Exact predecessor authority, absent only at epoch one.
        previous_authority_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::previous_authority_ref`].
        previous_authority_sha256: Option<DeploymentRuntimeAuthorityDigest>,
        /// Active or permanently retired authority state.
        state: PrincipalAuthorityStateV1,
        /// Unix issue time in whole seconds.
        issued_at: UnixTimeV1,
        /// Unix expiry time for newly signed runtime records.
        expires_at: UnixTimeV1,
        /// Principal whose active accepted root key issued this authority.
        issuer_principal_id: ResidentPrincipalIdV1,
        /// Active accepted root-key identity.
        issuer_key_id: ResidentKeyIdV1,
        /// Positive accepted root-key epoch.
        issuer_key_epoch: PrincipalKeyEpochV1
    },
    validate_deployment_runtime_authority_shape,
    wire_transform = deployment_runtime_authority_wire_schema
);

/// Owner-resolved immutable scope for runtime-authority admission.
///
/// The caller supplies an already admitted binding and the selected deployment
/// store coordinate. Constructing this witness performs no store lookup,
/// liveness decision, trust-checkpoint resolution, or activation.
#[derive(Clone, Copy)]
pub struct DeploymentRuntimeAuthorityAdmissionContextV1<'a> {
    /// Exact selected runtime-authority schema digest.
    pub expected_schema_sha256: &'a SchemaDigest,
    /// Immutable reference assigned to the admitted deployment binding.
    pub deployment_binding_ref: &'a ResidentRecordRefV1,
    /// Admitted binding that supplies the exact principal and binding digest.
    pub deployment_binding: &'a DeploymentBindingV1,
    /// Logical deployment control store selected for this authority.
    pub control_store_id: &'a ControlStoreIdV1,
    /// Exact deployment control-store incarnation.
    pub control_incarnation_id: &'a ControlIncarnationIdV1,
    /// Owner-resolved active principal root key admitting this record.
    pub signer: PrincipalKeyAdmissionContextV1<'a>,
}

/// Exact active runtime head committed by the principal trust checkpoint.
///
/// This derived array member is not a new signed record or admission proof.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeploymentRuntimeAuthorityHeadV1 {
    /// Stable authority lineage.
    pub runtime_authority_id: DeploymentRuntimeAuthorityIdV1,
    /// Exact immutable deployment binding digest.
    pub deployment_binding_sha256: DeploymentBindingDigest,
    /// Logical deployment control store.
    pub control_store_id: ControlStoreIdV1,
    /// Exact deployment-store incarnation.
    pub control_incarnation_id: ControlIncarnationIdV1,
    /// Immutable authority record reference.
    pub authority_ref: ResidentRecordRefV1,
    /// Exact authority record digest.
    pub authority_sha256: DeploymentRuntimeAuthorityDigest,
    /// Positive authority succession epoch.
    pub authority_epoch: DeploymentRuntimeAuthorityEpochV1,
}

/// Hash a supplied active runtime-head array in runtime-lineage order.
///
/// The empty array is the bootstrap root. Completeness and contextual admission
/// of the named records remain the caller's obligations.
///
/// # Errors
/// Rejects excessive heads, duplicate or unordered lineage IDs, or failed
/// canonical encoding.
pub fn deployment_runtime_authority_heads_root_sha256(
    heads: &[DeploymentRuntimeAuthorityHeadV1],
) -> Result<DeploymentRuntimeAuthorityHeadsRootDigest, InvalidResidentRecordV1> {
    bounded_len("deployment runtime authority heads", heads.len())?;
    if !heads
        .windows(2)
        .all(|pair| pair[0].runtime_authority_id < pair[1].runtime_authority_id)
    {
        return Err(InvalidResidentRecordV1::NonCanonical {
            relation: "deployment runtime authority heads",
        });
    }
    Ok(DeploymentRuntimeAuthorityHeadsRootDigest::hash_canonical(
        heads,
    )?)
}

impl DeploymentRuntimeAuthorityV1 {
    /// Project this active record into the canonical runtime-head array member.
    ///
    /// # Errors
    /// Rejects an invalid record or retired authority. This does not establish
    /// that the record is current in the principal trust store.
    pub fn active_head(
        &self,
        authority_ref: ResidentRecordRefV1,
    ) -> Result<DeploymentRuntimeAuthorityHeadV1, InvalidResidentRecordV1> {
        self.verify()?;
        if self.state() != &PrincipalAuthorityStateV1::Active {
            return Err(contextual("deployment runtime authority", "active head"));
        }
        Ok(DeploymentRuntimeAuthorityHeadV1 {
            runtime_authority_id: self.runtime_authority_id().clone(),
            deployment_binding_sha256: self.deployment_binding_sha256().clone(),
            control_store_id: self.control_store_id().clone(),
            control_incarnation_id: self.control_incarnation_id().clone(),
            authority_ref,
            authority_sha256: self.canonical_deployment_runtime_authority_sha256().clone(),
            authority_epoch: *self.authority_epoch(),
        })
    }

    /// Admit the active epoch-one authority for one deployment-store scope.
    ///
    /// `verify_signature` must authenticate
    /// [`DEPLOYMENT_RUNTIME_AUTHORITY_V1_SIGNATURE_DOMAIN`] followed by the
    /// canonical authority digest with the exact principal key carried by
    /// `context.signer`. `resolve_record_schema` must establish that every
    /// permitted schema is a current Pale-native runtime-record contract;
    /// a name or namespace prefix is not proof of ownership. This function
    /// neither persists the authority nor advances a principal-trust checkpoint.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] for schema, binding, principal,
    /// store, schema ownership, epoch-one, lifecycle-state, root-key, digest,
    /// or signature failure.
    pub fn admit_initial_wire<R, F>(
        wire: DeploymentRuntimeAuthorityWireV1,
        context: &DeploymentRuntimeAuthorityAdmissionContextV1<'_>,
        resolve_record_schema: R,
        verify_signature: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        R: FnMut(&SchemaId) -> Result<(), InvalidResidentRecordV1>,
        F: FnOnce(
            &[u8],
            &DeploymentRuntimeAuthorityPreimageV1,
            &DeploymentRuntimeAuthorityDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        Self::admit_scoped_wire(
            wire,
            context,
            resolve_record_schema,
            |preimage, digest, signature| {
                if preimage.authority_epoch.get() != 1
                    || preimage.state != PrincipalAuthorityStateV1::Active
                    || preimage.previous_authority_ref.is_some()
                {
                    return Err(contextual(
                        "deployment runtime authority",
                        "active epoch-one authority without predecessor",
                    ));
                }
                verify_signature(
                    DEPLOYMENT_RUNTIME_AUTHORITY_V1_SIGNATURE_DOMAIN,
                    preimage,
                    digest,
                    signature,
                )
            },
        )
    }

    /// Admit the exact next authority in one deployment-store lineage.
    ///
    /// The successor preserves authority identity, principal, deployment
    /// binding, control store, and store incarnation, then advances the epoch
    /// exactly once and binds the predecessor ref/digest. A principal-root-
    /// signed successor may replace the subject key, lifecycle state, validity
    /// interval, and permitted record-schema set. `resolve_record_schema`
    /// re-establishes Pale ownership for every member of that replacement.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] for schema, binding, principal,
    /// store, schema ownership, predecessor, next-epoch, root-key, digest, or
    /// signature failure.
    pub fn admit_successor_wire<R, F>(
        wire: DeploymentRuntimeAuthorityWireV1,
        context: &DeploymentRuntimeAuthorityAdmissionContextV1<'_>,
        predecessor_ref: &ResidentRecordRefV1,
        predecessor: &Self,
        resolve_record_schema: R,
        verify_signature: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        R: FnMut(&SchemaId) -> Result<(), InvalidResidentRecordV1>,
        F: FnOnce(
            &[u8],
            &DeploymentRuntimeAuthorityPreimageV1,
            &DeploymentRuntimeAuthorityDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        predecessor.verify()?;
        Self::admit_scoped_wire(
            wire,
            context,
            resolve_record_schema,
            |preimage, digest, signature| {
                let expected_epoch = predecessor
                    .authority_epoch()
                    .checked_successor()
                    .ok_or_else(|| {
                        contextual(
                            "deployment runtime authority",
                            "non-exhausted successor epoch",
                        )
                    })?;
                if predecessor.state() != &PrincipalAuthorityStateV1::Active
                    || preimage.runtime_authority_id != *predecessor.runtime_authority_id()
                    || preimage.principal_id != *predecessor.principal_id()
                    || preimage.deployment_binding_ref != *predecessor.deployment_binding_ref()
                    || preimage.deployment_binding_sha256
                        != *predecessor.deployment_binding_sha256()
                    || preimage.control_store_id != *predecessor.control_store_id()
                    || preimage.control_incarnation_id != *predecessor.control_incarnation_id()
                    || preimage.authority_epoch != expected_epoch
                    || preimage.previous_authority_ref.as_ref() != Some(predecessor_ref)
                    || preimage.previous_authority_sha256.as_ref()
                        != Some(predecessor.canonical_deployment_runtime_authority_sha256())
                {
                    return Err(contextual(
                        "deployment runtime authority",
                        "exact active predecessor, preserved scope, and next epoch",
                    ));
                }
                verify_signature(
                    DEPLOYMENT_RUNTIME_AUTHORITY_V1_SIGNATURE_DOMAIN,
                    preimage,
                    digest,
                    signature,
                )
            },
        )
    }

    fn admit_scoped_wire<R, F>(
        wire: DeploymentRuntimeAuthorityWireV1,
        context: &DeploymentRuntimeAuthorityAdmissionContextV1<'_>,
        mut resolve_record_schema: R,
        admit: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        R: FnMut(&SchemaId) -> Result<(), InvalidResidentRecordV1>,
        F: FnOnce(
            &DeploymentRuntimeAuthorityPreimageV1,
            &DeploymentRuntimeAuthorityDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        context.deployment_binding.verify()?;
        Self::admit_wire(wire, |preimage, digest, signature| {
            validate_principal_signed_coordinates(
                "deployment runtime authority",
                context.expected_schema_sha256,
                PrincipalSignedCoordinatesV1 {
                    schema_sha256: &preimage.schema_sha256,
                    subject_principal_id: &preimage.principal_id,
                    issuer_principal_id: &preimage.issuer_principal_id,
                    issuer_key_id: &preimage.issuer_key_id,
                    issuer_key_epoch: preimage.issuer_key_epoch,
                },
                &context.signer,
            )?;
            if &preimage.deployment_binding_ref != context.deployment_binding_ref
                || preimage.deployment_binding_sha256
                    != *context.deployment_binding.canonical_binding_sha256()
                || preimage.principal_id
                    != *context
                        .deployment_binding
                        .deployment_terms()
                        .principal_scope()
                || &preimage.control_store_id != context.control_store_id
                || &preimage.control_incarnation_id != context.control_incarnation_id
            {
                return Err(contextual(
                    "deployment runtime authority",
                    "exact deployment binding, principal, store, and incarnation",
                ));
            }
            for schema_id in &preimage.permitted_record_schema_ids {
                resolve_record_schema(schema_id)?;
            }
            admit(preimage, digest, signature)
        })
    }
}

fn deployment_runtime_authority_wire_schema(schema: &mut schemars::Schema) {
    project_resident_set_property_v1(schema, "permitted_record_schema_ids", Some(1));
    require_all_or_none_properties_v1(
        schema,
        &["previous_authority_ref", "previous_authority_sha256"],
    );
}

fn validate_deployment_runtime_authority_shape(
    value: &DeploymentRuntimeAuthorityPreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    validate_issuer_shape(
        "deployment runtime authority",
        &value.principal_id,
        &value.issuer_principal_id,
    )?;
    strictly_sorted(
        "deployment runtime authority permitted record schemas",
        &value.permitted_record_schema_ids,
    )?;
    let valid_lineage = matches!(
        (
            value.authority_epoch.get(),
            value.previous_authority_ref.as_ref(),
            value.previous_authority_sha256.as_ref(),
        ),
        (1, None, None) | (2.., Some(_), Some(_))
    );
    if value.permitted_record_schema_ids.is_empty() || !valid_lineage {
        Err(contextual(
            "deployment runtime authority",
            "non-empty schema scope and epoch lineage",
        ))
    } else {
        Ok(())
    }
}
