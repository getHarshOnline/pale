//! Pure command and receipt projections for deployment finalization.
//!
//! New submission and historical recovery use these same ordered candidates,
//! affected heads, committed records, and head transitions. The projections do
//! not authenticate a record, read a store, commit a transaction, or grant
//! current authority.

use pale_core::records::*;

use crate::resident::AdmittedDeploymentFinalization;

pub(super) enum CommandSelectionMismatch {
    Candidates,
    Heads,
}

impl CommandSelectionMismatch {
    pub(super) const fn invariant(&self) -> &'static str {
        match self {
            Self::Candidates => "the exact operation-ordered finalization candidates",
            Self::Heads => "exact predecessor heads and absent initial signer heads",
        }
    }
}

pub(super) fn verify_command_selection(
    finalization: &AdmittedDeploymentFinalization<'_>,
    checkpoint: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    command: &LocalAdminCommandPreimageV1,
) -> Result<(), CommandSelectionMismatch> {
    let records = finalization.records();
    let candidates = [
        (
            DEPLOYMENT_BINDING_V1_SCHEMA,
            records.binding.0,
            records.binding.1.canonical_binding_sha256().as_str(),
        ),
        (
            CHECKPOINT_SIGNING_AUTHORITY_V1_SCHEMA,
            records.checkpoint_signer.0,
            records
                .checkpoint_signer
                .1
                .canonical_checkpoint_signing_authority_sha256()
                .as_str(),
        ),
        (
            DEPLOYMENT_RUNTIME_AUTHORITY_V1_SCHEMA,
            records.runtime_authority.0,
            records
                .runtime_authority
                .1
                .canonical_deployment_runtime_authority_sha256()
                .as_str(),
        ),
        (
            PRINCIPAL_TRUST_CHECKPOINT_V1_SCHEMA,
            records.trust.successor_ref,
            records
                .trust
                .successor
                .canonical_trust_checkpoint_sha256()
                .as_str(),
        ),
    ];
    if command.operation != LocalAdminOperationV1::FinalizeDeployment
        || command.candidate_records.len() != candidates.len()
        || command.candidate_records.iter().zip(candidates).any(
            |(actual, (schema, reference, digest))| {
                actual.candidate_record_schema_id.as_str() != schema
                    || &actual.candidate_record_ref != reference
                    || actual.candidate_record_sha256.as_str() != digest
            },
        )
    {
        return Err(CommandSelectionMismatch::Candidates);
    }
    let trust = records.trust.predecessor;
    let signer = records.checkpoint_signer.1;
    let runtime = records.runtime_authority.1;
    let checkpoint = checkpoint.1;
    let heads = [
        (
            LocalAdminHeadSelectorV1::PrincipalTrustCheckpoint {
                principal_id: trust.principal_id().clone(),
                trust_store_id: trust.trust_store_id().clone(),
                trust_store_incarnation_id: trust.trust_store_incarnation_id().clone(),
            },
            Some(records.trust.predecessor_ref),
            Some(trust.canonical_trust_checkpoint_sha256().as_str()),
            Some(trust.checkpoint_epoch().get()),
        ),
        (
            LocalAdminHeadSelectorV1::CheckpointSigningAuthority {
                principal_id: signer.principal_id().clone(),
                checkpoint_signing_authority_id: signer.checkpoint_signing_authority_id().clone(),
                control_store_id: signer.control_store_id().clone(),
                control_incarnation_id: signer.control_incarnation_id().clone(),
                scope: signer.scope().clone(),
            },
            None,
            None,
            None,
        ),
        (
            LocalAdminHeadSelectorV1::DeploymentRuntimeAuthority {
                principal_id: runtime.principal_id().clone(),
                runtime_authority_id: runtime.runtime_authority_id().clone(),
                deployment_binding_ref: runtime.deployment_binding_ref().clone(),
                deployment_binding_sha256: runtime.deployment_binding_sha256().clone(),
                control_store_id: runtime.control_store_id().clone(),
                control_incarnation_id: runtime.control_incarnation_id().clone(),
            },
            None,
            None,
            None,
        ),
        (
            LocalAdminHeadSelectorV1::ControlCheckpoint {
                control_store_id: checkpoint.control_store_id().clone(),
                control_incarnation_id: checkpoint.control_incarnation_id().clone(),
                checkpoint_domain_id: checkpoint.checkpoint_domain_id().clone(),
            },
            checkpoint.previous_control_checkpoint_ref().as_ref(),
            checkpoint
                .previous_control_checkpoint_sha256()
                .as_ref()
                .map(|digest| digest.as_str()),
            Some(checkpoint.domain_epoch().get()),
        ),
    ];
    if command.expected_heads.len() != heads.len()
        || heads
            .into_iter()
            .any(|(selector, reference, digest, epoch)| {
                !command.expected_heads.iter().any(|actual| {
                    actual.head_kind == selector
                        && actual.expected_head_ref.as_ref() == reference
                        && actual
                            .expected_head_sha256
                            .as_ref()
                            .map(|digest| digest.as_str())
                            == digest
                        && actual
                            .expected_head_epoch
                            .is_none_or(|actual| Some(actual.get()) == epoch)
                })
            })
    {
        return Err(CommandSelectionMismatch::Heads);
    }
    Ok(())
}

pub(in crate::resident) struct FinalizationReceiptProjection {
    signing_authority: SigningAuthorityBindingV1,
    committed_records: Vec<LocalAdminCommittedRecordV1>,
    head_transitions: Vec<LocalAdminHeadTransitionV1>,
}

impl FinalizationReceiptProjection {
    pub(in crate::resident) fn new(
        finalization: &AdmittedDeploymentFinalization<'_>,
        checkpoint: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
        command_ref: &ResidentRecordRefV1,
        command: &LocalAdminCommandV1,
    ) -> Result<Self, InvalidResidentRecordV1> {
        let selected = finalization.records();
        let runtime = selected.runtime_authority.1;
        let committed_records = vec![
            committed(
                command_ref,
                command.canonical_admin_command_sha256().as_str(),
            )?,
            committed(
                selected.binding.0,
                selected.binding.1.canonical_binding_sha256().as_str(),
            )?,
            committed(
                selected.checkpoint_signer.0,
                selected
                    .checkpoint_signer
                    .1
                    .canonical_checkpoint_signing_authority_sha256()
                    .as_str(),
            )?,
            committed(
                selected.runtime_authority.0,
                runtime
                    .canonical_deployment_runtime_authority_sha256()
                    .as_str(),
            )?,
            committed(
                selected.trust.successor_ref,
                selected
                    .trust
                    .successor
                    .canonical_trust_checkpoint_sha256()
                    .as_str(),
            )?,
            committed(
                checkpoint.0,
                checkpoint.1.canonical_control_checkpoint_sha256().as_str(),
            )?,
        ];
        let head_transitions = vec![
            LocalAdminHeadTransitionV1 {
                head_kind: LocalAdminHeadSelectorV1::PrincipalTrustCheckpoint {
                    principal_id: selected.trust.predecessor.principal_id().clone(),
                    trust_store_id: selected.trust.predecessor.trust_store_id().clone(),
                    trust_store_incarnation_id: selected
                        .trust
                        .predecessor
                        .trust_store_incarnation_id()
                        .clone(),
                },
                previous_head_ref: Some(selected.trust.predecessor_ref.clone()),
                previous_head_sha256: Some(artifact_digest(
                    selected
                        .trust
                        .predecessor
                        .canonical_trust_checkpoint_sha256()
                        .as_str(),
                )?),
                new_head_ref: Some(selected.trust.successor_ref.clone()),
                new_head_sha256: Some(artifact_digest(
                    selected
                        .trust
                        .successor
                        .canonical_trust_checkpoint_sha256()
                        .as_str(),
                )?),
            },
            LocalAdminHeadTransitionV1 {
                head_kind: LocalAdminHeadSelectorV1::CheckpointSigningAuthority {
                    principal_id: selected.checkpoint_signer.1.principal_id().clone(),
                    checkpoint_signing_authority_id: selected
                        .checkpoint_signer
                        .1
                        .checkpoint_signing_authority_id()
                        .clone(),
                    control_store_id: selected.checkpoint_signer.1.control_store_id().clone(),
                    control_incarnation_id: selected
                        .checkpoint_signer
                        .1
                        .control_incarnation_id()
                        .clone(),
                    scope: selected.checkpoint_signer.1.scope().clone(),
                },
                previous_head_ref: None,
                previous_head_sha256: None,
                new_head_ref: Some(selected.checkpoint_signer.0.clone()),
                new_head_sha256: Some(artifact_digest(
                    selected
                        .checkpoint_signer
                        .1
                        .canonical_checkpoint_signing_authority_sha256()
                        .as_str(),
                )?),
            },
            LocalAdminHeadTransitionV1 {
                head_kind: LocalAdminHeadSelectorV1::DeploymentRuntimeAuthority {
                    principal_id: runtime.principal_id().clone(),
                    runtime_authority_id: runtime.runtime_authority_id().clone(),
                    deployment_binding_ref: runtime.deployment_binding_ref().clone(),
                    deployment_binding_sha256: runtime.deployment_binding_sha256().clone(),
                    control_store_id: runtime.control_store_id().clone(),
                    control_incarnation_id: runtime.control_incarnation_id().clone(),
                },
                previous_head_ref: None,
                previous_head_sha256: None,
                new_head_ref: Some(selected.runtime_authority.0.clone()),
                new_head_sha256: Some(artifact_digest(
                    runtime
                        .canonical_deployment_runtime_authority_sha256()
                        .as_str(),
                )?),
            },
            LocalAdminHeadTransitionV1 {
                head_kind: LocalAdminHeadSelectorV1::ControlCheckpoint {
                    control_store_id: checkpoint.1.control_store_id().clone(),
                    control_incarnation_id: checkpoint.1.control_incarnation_id().clone(),
                    checkpoint_domain_id: checkpoint.1.checkpoint_domain_id().clone(),
                },
                previous_head_ref: checkpoint.1.previous_control_checkpoint_ref().clone(),
                previous_head_sha256: checkpoint
                    .1
                    .previous_control_checkpoint_sha256()
                    .as_ref()
                    .map(|digest| artifact_digest(digest.as_str()))
                    .transpose()?,
                new_head_ref: Some(checkpoint.0.clone()),
                new_head_sha256: Some(artifact_digest(
                    checkpoint.1.canonical_control_checkpoint_sha256().as_str(),
                )?),
            },
        ];
        let signing_authority = SigningAuthorityBindingV1::integrated_runtime(
            runtime.principal_id().clone(),
            runtime.subject_key_id().clone(),
            *runtime.subject_key_epoch(),
            IntegratedRuntimeAuthorityV1 {
                deployment_runtime_authority_ref: selected.runtime_authority.0.clone(),
                deployment_runtime_authority_sha256: artifact_digest(
                    runtime
                        .canonical_deployment_runtime_authority_sha256()
                        .as_str(),
                )?,
                deployment_binding_ref: selected.binding.0.clone(),
                deployment_binding_sha256: artifact_digest(
                    selected.binding.1.canonical_binding_sha256().as_str(),
                )?,
                checked_principal_trust_checkpoint_ref: selected.trust.successor_ref.clone(),
                checked_principal_trust_checkpoint_sha256: artifact_digest(
                    selected
                        .trust
                        .successor
                        .canonical_trust_checkpoint_sha256()
                        .as_str(),
                )?,
            },
        )?;
        Ok(Self {
            signing_authority,
            committed_records,
            head_transitions,
        })
    }

    pub(in crate::resident) const fn signing_authority(&self) -> &SigningAuthorityBindingV1 {
        &self.signing_authority
    }

    /// Borrow the exact transaction-owned record list.
    pub(in crate::resident) fn committed_records(&self) -> &[LocalAdminCommittedRecordV1] {
        &self.committed_records
    }

    /// Borrow the exact affected-head transitions.
    pub(in crate::resident) fn head_transitions(&self) -> &[LocalAdminHeadTransitionV1] {
        &self.head_transitions
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn admission_context<'a>(
        &'a self,
        finalization: &'a AdmittedDeploymentFinalization<'_>,
        command_ref: &'a ResidentRecordRefV1,
        command: &'a LocalAdminCommandV1,
        receipt_id: &'a LocalAdminReceiptIdV1,
        receipt_schema_sha256: &'a SchemaDigest,
        committed_at: &'a UnixTimeV1,
        recorded_at: &'a UnixTimeV1,
    ) -> LocalAdminReceiptAdmissionContextV1<'a> {
        LocalAdminReceiptAdmissionContextV1 {
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: receipt_schema_sha256,
                signing_authority: self.signing_authority(),
            },
            admin_command_ref: command_ref,
            admin_command: command,
            runtime_authority: finalization.records().runtime_authority.1,
            expected_admin_receipt_id: receipt_id,
            expected_disposition: LocalAdminDispositionV1::Committed,
            expected_committed_records: &self.committed_records,
            expected_head_transitions: &self.head_transitions,
            expected_failure_code: None,
            expected_committed_at: Some(committed_at),
            expected_recorded_at: recorded_at,
        }
    }
}

fn committed(
    reference: &ResidentRecordRefV1,
    digest: &str,
) -> Result<LocalAdminCommittedRecordV1, InvalidResidentRecordV1> {
    Ok(LocalAdminCommittedRecordV1 {
        committed_record_ref: reference.clone(),
        committed_record_sha256: artifact_digest(digest)?,
    })
}

fn artifact_digest(value: &str) -> Result<ArtifactDigest, InvalidResidentRecordV1> {
    ArtifactDigest::parse(value).map_err(|_| reject("valid exact finalization coordinates"))
}

fn reject(invariant: &'static str) -> InvalidResidentRecordV1 {
    InvalidResidentRecordV1::ContextualAdmission {
        record: "deployment finalization",
        invariant,
    }
}
