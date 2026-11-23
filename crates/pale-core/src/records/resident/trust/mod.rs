//! Signed principal-bootstrap, resident-writer, and producer-incarnation trust contracts.
//!
//! [`principal`] and [`authority`] establish the signed trust roots consumed by
//! [`bootstrap`]. [`writer`] defines the principal writer-fence transition
//! algebra, while [`producer`] defines allocation and retirement of producer
//! incarnations. These modules perform pure contract admission: they do not
//! acquire OS locks, decide liveness, mutate a trust store, generate randomness,
//! commit a journal, activate a runtime, or cause an external effect.
//!
//! # Signature verification contract
//!
//! Every verification callback authenticates the signature-domain bytes
//! `D_signature` followed by canonical unsigned-preimage digest `H_canonical`:

#![doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/signature-preimage.inline.mathml"
))]
//!
//! Separate callback arguments do not permit authenticating only one component
//! or substituting a transport challenge. Signer provenance changes by stage:
//!
//! - During bootstrap, [`PRINCIPAL_RECORD_V1_SIGNATURE_DOMAIN`] and
//!   [`PRINCIPAL_RECOVERY_POLICY_V1_SIGNATURE_DOMAIN`] use the candidate root
//!   key whose mutually bound pair is being admitted. A key-transition
//!   principal successor instead uses the transition's new key; a policy and
//!   same-key principal successor use the active accepted principal key.
//! - [`PRINCIPAL_KEY_TRANSITION_OLD_V1_SIGNATURE_DOMAIN`] and
//!   [`PRINCIPAL_KEY_TRANSITION_NEW_V1_SIGNATURE_DOMAIN`] use the exact
//!   predecessor and proposed successor keys, respectively. Recovery omits the
//!   old-key signature and retains the required new-key signature.
//! - [`PRINCIPAL_RECOVERY_PROOF_V1_SIGNATURE_DOMAIN`] uses exactly the
//!   threshold of distinct eligible keys resolved from the predecessor's
//!   pinned [`PrincipalRecoveryPolicyV1`].
//! - [`LOCAL_ADMIN_AUTHORITY_V1_SIGNATURE_DOMAIN`],
//!   [`PRODUCER_INCARNATION_AUTHORITY_V1_SIGNATURE_DOMAIN`],
//!   [`CHECKPOINT_SIGNING_AUTHORITY_V1_SIGNATURE_DOMAIN`],
//!   [`PRINCIPAL_TRUST_CHECKPOINT_V1_SIGNATURE_DOMAIN`], and
//!   [`PRINCIPAL_BOOTSTRAP_RECEIPT_V1_SIGNATURE_DOMAIN`] use that admitted
//!   candidate root. Initial hard-domain checkpoints separately use the
//!   bootstrap-scoped key under
//!   [`crate::records::CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN`].
//! - [`PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SIGNATURE_DOMAIN`],
//!   [`PRINCIPAL_RESIDENT_WRITER_TERMINAL_RECEIPT_V1_SIGNATURE_DOMAIN`],
//!   and [`PRINCIPAL_RESIDENT_WRITER_TAKEOVER_RECEIPT_V1_SIGNATURE_DOMAIN`]
//!   use the exact owner-resolved active principal-key coordinate supplied to
//!   admission.
//! - [`PRODUCER_INCARNATION_V1_SIGNATURE_DOMAIN`] and
//!   [`PRODUCER_INCARNATION_RETIREMENT_V1_SIGNATURE_DOMAIN`] use the exact
//!   public key carried by the owner-resolved active
//!   [`ProducerIncarnationAuthorityV1`].
//! - [`DEPLOYMENT_BINDING_PROPOSAL_V1_SIGNATURE_DOMAIN`],
//!   [`DEPLOYMENT_BINDING_V1_SIGNATURE_DOMAIN`], and
//!   [`DEPLOYMENT_RUNTIME_AUTHORITY_V1_SIGNATURE_DOMAIN`] use exact active
//!   principal-key coordinates resolved from the accepted trust frontier.
//! - [`DEPLOYMENT_ACCEPTANCE_V1_SIGNATURE_DOMAIN`] and
//!   [`DEPLOYMENT_ACCEPTANCE_REVOCATION_V1_SIGNATURE_DOMAIN`] remain signed by
//!   their external controller/relationship semantic owner. Their admission
//!   callbacks must prove the applicable authority class; Pale neither
//!   invents nor substitutes that authority.
mod administration;
mod authority;
mod bootstrap;
mod deployment;
mod principal;
mod producer;
mod writer;

pub use administration::*;
pub use authority::*;
pub use bootstrap::*;
pub use deployment::*;
pub use principal::*;
pub use producer::*;
pub use writer::*;

use super::*;
use crate::records::{InvalidResidentRecordV1, ResidentPrincipalIdV1, SchemaDigest};

/// Owner-resolved active principal-key coordinate used by signed trust admission.
///
/// The caller must resolve this coordinate from the accepted principal
/// checkpoint before admission.
#[derive(Clone, Copy)]
pub struct PrincipalKeyAdmissionContextV1<'a> {
    principal_id: &'a ResidentPrincipalIdV1,
    key_id: &'a ResidentKeyIdV1,
    key_epoch: PrincipalKeyEpochV1,
}

impl<'a> PrincipalKeyAdmissionContextV1<'a> {
    /// Bind the exact active principal key selected by the admitting owner.
    pub const fn new(
        principal_id: &'a ResidentPrincipalIdV1,
        key_id: &'a ResidentKeyIdV1,
        key_epoch: PrincipalKeyEpochV1,
    ) -> Self {
        Self {
            principal_id,
            key_id,
            key_epoch,
        }
    }
}

pub(super) struct PrincipalSignedCoordinatesV1<'a> {
    pub schema_sha256: &'a SchemaDigest,
    pub subject_principal_id: &'a ResidentPrincipalIdV1,
    pub issuer_principal_id: &'a ResidentPrincipalIdV1,
    pub issuer_key_id: &'a ResidentKeyIdV1,
    pub issuer_key_epoch: PrincipalKeyEpochV1,
}

pub(super) fn validate_principal_signed_coordinates(
    family: &'static str,
    expected_schema_sha256: &SchemaDigest,
    coordinates: PrincipalSignedCoordinatesV1<'_>,
    signer: &PrincipalKeyAdmissionContextV1<'_>,
) -> Result<(), InvalidResidentRecordV1> {
    if coordinates.schema_sha256 != expected_schema_sha256
        || coordinates.subject_principal_id != signer.principal_id
        || coordinates.issuer_principal_id != signer.principal_id
        || coordinates.issuer_key_id != signer.key_id
        || coordinates.issuer_key_epoch != signer.key_epoch
    {
        Err(contextual(family, "exact schema and active principal key"))
    } else {
        Ok(())
    }
}

pub(super) fn validate_issuer_shape(
    family: &'static str,
    subject_principal_id: &ResidentPrincipalIdV1,
    issuer_principal_id: &ResidentPrincipalIdV1,
) -> Result<(), InvalidResidentRecordV1> {
    if subject_principal_id != issuer_principal_id {
        Err(contextual(family, "same-principal issuer"))
    } else {
        Ok(())
    }
}

pub(super) fn contextual(record: &'static str, invariant: &'static str) -> InvalidResidentRecordV1 {
    InvalidResidentRecordV1::ContextualAdmission { record, invariant }
}

#[cfg(test)]
#[path = "../../../../tests/internal/records/resident/trust/mod.rs"]
mod tests;
