//! Shared trust succession and operation-specific preservation rules.
//! Core checks the predecessor relation. This module limits which frontier
//! roots an application operation may change; it does not admit signatures,
//! discover current heads or commit a store.

use pale_core::records::{
    InvalidResidentRecordV1, PrincipalTrustCheckpointV1, ResidentRecordRefV1,
};

/// Exact admitted principal-trust checkpoint predecessor and successor.
#[derive(Debug, Clone, Copy)]
pub struct AdmittedPrincipalTrustCheckpointTransition<'a> {
    pub(super) predecessor_ref: &'a ResidentRecordRefV1,
    pub(super) predecessor: &'a PrincipalTrustCheckpointV1,
    pub(super) successor_ref: &'a ResidentRecordRefV1,
    pub(super) successor: &'a PrincipalTrustCheckpointV1,
}

impl<'a> AdmittedPrincipalTrustCheckpointTransition<'a> {
    /// Bind an exact generic trust-checkpoint successor pair.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] unless `successor` is the exact
    /// operation-independent successor of `predecessor` at `predecessor_ref`.
    pub fn new(
        predecessor_ref: &'a ResidentRecordRefV1,
        predecessor: &'a PrincipalTrustCheckpointV1,
        successor_ref: &'a ResidentRecordRefV1,
        successor: &'a PrincipalTrustCheckpointV1,
    ) -> Result<Self, InvalidResidentRecordV1> {
        successor.verify_successor_of(predecessor_ref, predecessor)?;
        Ok(Self {
            predecessor_ref,
            predecessor,
            successor_ref,
            successor,
        })
    }

    /// Compare every unrelated frontier field and the unchanged principal signer.
    /// The operation still has to derive and compare the permitted new roots.
    pub(super) fn preserves_other_heads(&self, change: TrustHeadChange) -> bool {
        let prior = self.predecessor;
        let next = self.successor;
        let selected_roots = match change {
            TrustHeadChange::ProducerIncarnations => {
                next.checkpoint_signing_authority_heads_root_sha256()
                    == prior.checkpoint_signing_authority_heads_root_sha256()
                    && next.deployment_runtime_authority_heads_root_sha256()
                        == prior.deployment_runtime_authority_heads_root_sha256()
            }
            TrustHeadChange::DeploymentSigners => {
                next.producer_incarnation_heads_root_sha256()
                    == prior.producer_incarnation_heads_root_sha256()
            }
        };
        selected_roots
            && next.issuer_principal_id() == prior.issuer_principal_id()
            && next.issuer_key_id() == prior.issuer_key_id()
            && next.issuer_key_epoch() == prior.issuer_key_epoch()
            && next.principal_record_ref() == prior.principal_record_ref()
            && next.principal_record_sha256() == prior.principal_record_sha256()
            && next.recovery_policy_ref() == prior.recovery_policy_ref()
            && next.recovery_policy_sha256() == prior.recovery_policy_sha256()
            && next.local_admin_authority_ref() == prior.local_admin_authority_ref()
            && next.local_admin_authority_sha256() == prior.local_admin_authority_sha256()
            && next.producer_incarnation_authority_ref()
                == prior.producer_incarnation_authority_ref()
            && next.producer_incarnation_authority_sha256()
                == prior.producer_incarnation_authority_sha256()
            && next.session_floor_epoch() == prior.session_floor_epoch()
            && next.owner_record_signing_authority_heads_root_sha256()
                == prior.owner_record_signing_authority_heads_root_sha256()
            && next.owner_signer_publication_frontier_heads_root_sha256()
                == prior.owner_signer_publication_frontier_heads_root_sha256()
            && next.deployment_acceptance_revocation_heads_root_sha256()
                == prior.deployment_acceptance_revocation_heads_root_sha256()
            && next.federation_space_home_heads_root_sha256()
                == prior.federation_space_home_heads_root_sha256()
            && next.node_delegation_heads_root_sha256() == prior.node_delegation_heads_root_sha256()
            && next.node_delegation_revocation_heads_root_sha256()
                == prior.node_delegation_revocation_heads_root_sha256()
    }
}

/// The two currently implemented application changes to trust-head roots.
pub(super) enum TrustHeadChange {
    /// Allocate or retire one producer without changing deployment signers.
    ProducerIncarnations,
    /// Add one deployment's signer pair without changing producer custody.
    DeploymentSigners,
}
