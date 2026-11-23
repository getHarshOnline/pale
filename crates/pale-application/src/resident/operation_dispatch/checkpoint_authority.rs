//! Retain the exact checkpoint-signing selection across custody interruptions.

use super::*;

#[derive(Clone, PartialEq, Eq)]
pub(super) struct RetainedCheckpointAuthority {
    authority_ref: ResidentRecordRefV1,
    authority_sha256: CheckpointSigningAuthorityDigest,
    principal_id: ResidentPrincipalIdV1,
    control_store_id: ControlStoreIdV1,
    control_incarnation_id: ControlIncarnationIdV1,
    checkpoint_domain_id: ControlCheckpointDomainIdV1,
    checkpoint_profile_ref: ResidentRecordRefV1,
    checkpoint_profile_sha256: CheckpointProfileDigest,
    subject_key_id: ResidentKeyIdV1,
    subject_key_epoch: SigningKeyEpochV1,
}

impl RetainedCheckpointAuthority {
    pub(super) fn new(authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>) -> Self {
        Self {
            authority_ref: authority.authority_ref.clone(),
            authority_sha256: authority.authority_sha256.clone(),
            principal_id: authority.principal_id.clone(),
            control_store_id: authority.control_store_id.clone(),
            control_incarnation_id: authority.control_incarnation_id.clone(),
            checkpoint_domain_id: authority.checkpoint_domain_id.clone(),
            checkpoint_profile_ref: authority.checkpoint_profile_ref.clone(),
            checkpoint_profile_sha256: authority.checkpoint_profile_sha256.clone(),
            subject_key_id: authority.subject_key_id.clone(),
            subject_key_epoch: authority.subject_key_epoch,
        }
    }

    pub(super) fn context(&self) -> ControlCheckpointSigningAuthorityAdmissionContextV1<'_> {
        ControlCheckpointSigningAuthorityAdmissionContextV1 {
            authority_ref: &self.authority_ref,
            authority_sha256: &self.authority_sha256,
            principal_id: &self.principal_id,
            control_store_id: &self.control_store_id,
            control_incarnation_id: &self.control_incarnation_id,
            checkpoint_domain_id: &self.checkpoint_domain_id,
            checkpoint_profile_ref: &self.checkpoint_profile_ref,
            checkpoint_profile_sha256: &self.checkpoint_profile_sha256,
            subject_key_id: &self.subject_key_id,
            subject_key_epoch: self.subject_key_epoch,
        }
    }
}
