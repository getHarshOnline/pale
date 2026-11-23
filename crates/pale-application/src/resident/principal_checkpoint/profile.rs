//! Immutable content for the principal trust checkpoint's finite state model.

use pale_core::{canon, records::*};
use pale_ports::{AccumulatorImplementation, AuthorityStoreCoordinate};
use serde::Serialize;

use super::PrincipalCheckpointError;

const CONTENT_LIMIT: usize = 4096;
pub(super) const CONTROL_SCHEMA_VERSION: u32 = 1;
pub(super) const INITIAL_DOMAIN_EPOCH: u64 = 1;
pub(super) const SCOPE_DOMAIN: &str = "PALE_PRINCIPAL_TRUST_CHECKPOINT_SCOPE_V1";
pub(super) const VALUE_DOMAIN: &str = "PALE_PRINCIPAL_TRUST_CHECKPOINT_LEAF_V1";
pub(super) const IMPLEMENTATION: AccumulatorImplementation =
    AccumulatorImplementation::SparseMerkleTree061Sha256PaleV1;

#[derive(Serialize)]
struct Content {
    content: &'static str,
    control_schema_version: u32,
    initial_domain_epoch: u64,
    native_implementation: &'static str,
    conflict_domain: &'static str,
    scope_encoding: &'static str,
    key_encoding: &'static str,
    value_encoding: &'static str,
    record_classes: [(&'static str, &'static str, &'static str); 4],
    bootstrap: &'static str,
    unresolved_custody: &'static str,
    derived_exclusions: &'static str,
    present_zero: &'static str,
}

fn content() -> Content {
    Content {
        content: "pale/principal-trust-checkpoint-profile.v1",
        control_schema_version: CONTROL_SCHEMA_VERSION,
        initial_domain_epoch: INITIAL_DOMAIN_EPOCH,
        native_implementation: IMPLEMENTATION.as_str(),
        conflict_domain: "principal-trust",
        scope_encoding: SCOPE_DOMAIN,
        key_encoding: crate::resident::checkpoint_profile::KEY_DOMAIN,
        value_encoding: VALUE_DOMAIN,
        record_classes: [
            (
                "principal-trust-head",
                "principal_id",
                "principal_id,checkpoint_epoch,trust_checkpoint_ref,trust_checkpoint_sha256",
            ),
            (
                "checkpoint-signing-authority-head",
                "checkpoint_signing_authority_id",
                "active-CheckpointSigningAuthorityHeadV1",
            ),
            (
                "deployment-runtime-authority-head",
                "runtime_authority_id",
                "active-DeploymentRuntimeAuthorityHeadV1",
            ),
            (
                "deployment-finalization",
                "deployment_binding_id",
                "binding_ref,binding_sha256,checkpoint_signer_ref,checkpoint_signer_sha256,runtime_authority_ref,runtime_authority_sha256,trust_checkpoint_ref,trust_checkpoint_sha256",
            ),
        ],
        bootstrap: "initial-trust-head-and-one-principal-checkpoint-signer",
        unresolved_custody: "empty-for-bootstrap-and-finalization",
        derived_exclusions: "containing-checkpoint,bootstrap-receipt,transaction-stamps,physical-tree-nodes,containing-checkpoint-lookups",
        present_zero: "reject-reserved-absence",
    }
}

/// Source-supported principal checkpoint content, not a deployment selection.
///
/// The descriptor assigns direct leaves to the trust head, active signer and
/// runtime heads, and immutable finalization selections. The bootstrap producer
/// constructs only its initial trust and signer leaves. Finalization construction
/// consumes a separately admitted transition through
/// [`Self::prepare_finalization`]; the descriptor alone does not admit records.
/// Administrator, producer-allocation and writer-fence mutations are unsupported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportedPrincipalTrustCheckpointProfile {
    pub(super) reference: ResidentRecordRefV1,
    pub(super) digest: CheckpointProfileDigest,
    pub(super) domains: [ControlCheckpointDomainIdV1; 1],
}

impl SupportedPrincipalTrustCheckpointProfile {
    /// Test the exact immutable source coordinate retained by this profile.
    #[must_use]
    pub(in crate::resident) fn has_coordinate(
        &self,
        reference: &ResidentRecordRefV1,
        digest: &CheckpointProfileDigest,
    ) -> bool {
        &self.reference == reference && &self.digest == digest
    }

    /// Test whether this source-supported profile defines the control domain.
    #[must_use]
    pub(in crate::resident) fn supports_domain(
        &self,
        domain: &ControlCheckpointDomainIdV1,
    ) -> bool {
        self.domains.binary_search(domain).is_ok()
    }

    /// Test the profile coordinate, domain and control-schema law of a checkpoint.
    #[must_use]
    pub(in crate::resident) fn supports_checkpoint(
        &self,
        checkpoint: &ControlStateCheckpointV1,
    ) -> bool {
        self.has_coordinate(
            checkpoint.checkpoint_profile_ref(),
            checkpoint.checkpoint_profile_sha256(),
        ) && self.supports_domain(checkpoint.checkpoint_domain_id())
            && *checkpoint.control_schema_version() == CONTROL_SCHEMA_VERSION
    }

    /// Render the exact source-supported descriptor as canonical JSON.
    ///
    /// # Errors
    /// Returns an error if canonical encoding fails.
    pub fn canonical_content() -> Result<String, canon::CanonError> {
        canon::to_canonical_string(&content())
    }

    /// Compute the descriptor's canonical content identity.
    ///
    /// # Errors
    /// Returns an error if canonical encoding fails.
    pub fn content_digest() -> Result<CheckpointProfileDigest, canon::CanonError> {
        CheckpointProfileDigest::hash_canonical(&content())
    }

    /// Admit bounded content only when its identity and implementation agree.
    ///
    /// This does not resolve the reference or reinterpret an existing store.
    ///
    /// # Errors
    /// Rejects excessive bytes, duplicate keys, a different digest, or content
    /// unsupported by this implementation.
    pub fn admit(
        reference: ResidentRecordRefV1,
        expected: &CheckpointProfileDigest,
        bytes: &[u8],
    ) -> Result<Self, PrincipalCheckpointError> {
        if bytes.is_empty() || bytes.len() > CONTENT_LIMIT {
            return Err(PrincipalCheckpointError::UnsupportedProfile);
        }
        let value = canon::parse_strict(bytes)?;
        let digest = CheckpointProfileDigest::hash_canonical(&value)?;
        if &digest != expected {
            return Err(PrincipalCheckpointError::ProfileDigest);
        }
        if canon::to_canonical_string(&value)? != Self::canonical_content()? {
            return Err(PrincipalCheckpointError::UnsupportedProfile);
        }
        Ok(Self {
            reference,
            digest,
            domains: [ControlCheckpointDomainIdV1::parse("principal-trust")?],
        })
    }

    /// Bind this content to independently selected physical and control stores.
    ///
    /// The physical trust store owns the transaction; the principal control
    /// store scopes the signed checkpoint and logical leaf keys. They need not
    /// have the same identity. This value feeds existing core admission and
    /// grants neither permission to initialize a store nor signing authority.
    #[must_use]
    pub fn bootstrap_profile<'a>(
        &'a self,
        trust_store: &'a AuthorityStoreCoordinate,
        control_store: &'a AuthorityStoreCoordinate,
        control_schema: &'a SchemaDigest,
    ) -> PrincipalBootstrapCheckpointProfileV1<'a> {
        PrincipalBootstrapCheckpointProfileV1 {
            trust_store_id: trust_store.store_id(),
            trust_store_incarnation_id: trust_store.incarnation_id(),
            principal_control_store_id: control_store.store_id(),
            principal_control_incarnation_id: control_store.incarnation_id(),
            checkpoint_profile_ref: &self.reference,
            checkpoint_profile_sha256: &self.digest,
            required_checkpoint_domains: &self.domains,
            control_checkpoint_schema_sha256: control_schema,
            control_schema_version: CONTROL_SCHEMA_VERSION,
        }
    }
}
