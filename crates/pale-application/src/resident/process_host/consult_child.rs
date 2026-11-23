//! Process-local ownership shared by every Consult child handler.
//!
//! This module selects one deployment and preserves the exact Consult roster
//! lineage. Handler-specific modules own dispatch and execution.

use super::*;
use crate::resident::process_host::restart::{
    ResidentProcessOperationRestartInput, ResidentProcessRecoveredOperationStart,
};
use crate::resident::ConsultationChildDispatchBatch;

/// Disjoint process-host custody for one task-owned Consult child.
pub(super) struct ResidentProcessConsultChildDriver<'host, 'store> {
    pub(super) application: &'host ResidentApplication<'store>,
    pub(super) operations: &'host mut ResidentOperationHost,
    pub(super) selector: ResidentDeploymentAuthoritySelector,
    pub(super) store: AuthorityStoreCoordinate,
}

impl<WriterGuard> ResidentProcessHost<'_, WriterGuard> {
    /// Borrow the selected application separately from its Consult task owner.
    pub(super) fn consultation_child_driver(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
    ) -> Result<ResidentProcessConsultChildDriver<'_, '_>, ResidentProcessHostSelectionError> {
        let index = self.deployment_authority_index(selector)?;
        let Self {
            operations,
            deployments,
            ..
        } = self;
        Ok(ResidentProcessConsultChildDriver {
            application: deployments[index].application.as_ref(),
            operations,
            selector: selector.clone(),
            store: deployments[index].store().clone(),
        })
    }
}

impl ResidentProcessConsultChildDriver<'_, '_> {
    /// Recover one roster-bound child through the selected application.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn classify_child_operation<'operation>(
        &self,
        association: ResidentOperationControlAssociation<'_>,
        access: ResidentDeploymentOperationAccess<'_>,
        input: ResidentProcessOperationRestartInput<'operation>,
        admission: &mut OperationControlRecoveryAdmission<'_>,
        parent: &ResidentProcessRecoveredOperationStart,
        current: &RecoveredConsultationCollection,
        planned: &ConsultationChildDispatchBatch,
        candidate: &ConsultationChildDispatchCandidate,
        selected_dispatch: Option<&ImmutableRecordReferenceV1>,
        observed_at: UnixTimeV1,
    ) -> Result<ResidentProcessOperationRestartDisposition, ResidentProcessOperationRestartError>
    {
        ResidentProcessHost::<()>::classify_consultation_child_operation_in(
            self.application,
            &self.selector,
            association,
            access,
            input,
            admission,
            parent,
            current,
            planned,
            candidate,
            selected_dispatch,
            observed_at,
        )
    }

    /// Bind a ready child completion to this selected deployment.
    pub(super) fn bind_completion<Work, Output>(
        &self,
        ready: ResidentConsultationCompletionReady<Work, Output>,
    ) -> Result<
        ResidentProcessConsultationCompletionReady<Work, Output>,
        Box<ResidentProcessConsultationCompletionBindingFailure<Work, Output>>,
    > {
        if ready.store() == &self.store {
            Ok(ResidentProcessConsultationCompletionReady {
                selector: self.selector.clone(),
                ready: Box::new(ready),
            })
        } else {
            Err(Box::new(
                ResidentProcessConsultationCompletionBindingFailure {
                    selector: self.selector.clone(),
                    ready: Box::new(ready),
                    reason: ResidentProcessHostSelectionError::RecoveredCollectionStoreAssociation,
                },
            ))
        }
    }

    /// Admit one recovered terminal child into its exact Consult slot.
    pub(super) fn admit_child_terminal<Admit>(
        &self,
        parent: &ResidentProcessRecoveredOperationStart,
        current: &RecoveredConsultationCollection,
        child: &ResidentProcessRecoveredOperationStart,
        recorded_at: UnixTimeV1,
        limits: ConsultationCollectionLimits,
        admit: Admit,
    ) -> Result<ConsultationCollection, ResidentProcessConsultationChildTerminalError>
    where
        Admit: FnOnce(
            &super::super::consultation::ConsultationRosterSlot,
            &ImmutableRecordReferenceV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let Some(lineage) = child.lineage.consult_child() else {
            return Err(ResidentProcessHostSelectionError::ConsultationChildAssociation.into());
        };
        let parent_operation = parent.control.operation().operation();
        let child_operation = child.control.operation().operation();
        let (parent_ref, parent_handle) = parent_operation.handle();
        let (child_ref, child_handle) = child_operation.handle();
        if parent.selector != child.selector
            || parent.generation_digest != child.generation_digest
            || parent.control.operation().store() != current.store()
            || child.control.operation().store() != current.store()
            || current.collection().parent().record_ref != *parent_ref
            || current.collection().parent().record_sha256.as_str()
                != parent_handle.canonical_operation_handle_sha256().as_str()
            || lineage.child().record_ref != *child_ref
            || lineage.child().record_sha256.as_str()
                != child_handle.canonical_operation_handle_sha256().as_str()
            || !child.lineage.admits(child_ref, child_handle)
            || parent_handle.operation_class() != &OperationClassV1::Consult
        {
            return Err(ResidentProcessHostSelectionError::ConsultationChildAssociation.into());
        }
        current
            .admit_recovered_child_terminal(
                lineage.slot_id(),
                child.control.observation(),
                recorded_at,
                limits,
                admit,
            )
            .map_err(Into::into)
    }

    /// Join one exact task completion into its Consult parent.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn join_terminal<
        'operation,
        Authority,
        Work,
        Output,
        AdmitEvidence,
        AdmitCollection,
        Rotation,
        ObservationSign,
        CheckpointSign,
    >(
        &mut self,
        association: ResidentOperationControlAssociation<'_>,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        ready: ResidentProcessConsultationCompletionReady<Work, Output>,
        input: ConsultationChildJoinInput<'operation>,
        checkpoint: ConsultationSuccessorCommitInput<'_>,
        resolve_store: &dyn super::super::OperationObservationAuthorityStoreResolver,
        admit_evidence: AdmitEvidence,
        admit_collection: AdmitCollection,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<
        CompletedResidentConsultationTerminal<Work, Output>,
        ResidentProcessConsultationTerminalJoinFailure<Work, Output>,
    >
    where
        AdmitEvidence: FnOnce(
            &super::super::consultation::ConsultationRosterSlot,
            &ImmutableRecordReferenceV1,
        ) -> Result<(), InvalidResidentRecordV1>,
        AdmitCollection: FnOnce(&ConsultationCollection) -> Result<(), InvalidResidentRecordV1>,
        Rotation: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let ResidentProcessConsultationCompletionReady { selector, ready } = ready;
        let selection =
            if !selector.matches_signing_authority(association.current_request_authority) {
                Err(ResidentProcessHostSelectionError::CurrentRequestAuthority)
            } else if !selector.matches_signing_authority(association.historical_handle_authority) {
                Err(ResidentProcessHostSelectionError::HistoricalHandleAuthority)
            } else if association.recovery_store != &self.store {
                Err(ResidentProcessHostSelectionError::RecoveryStoreAssociation)
            } else if !checkpoint_authority_matches(
                &selector,
                &self.store,
                association.checkpoint_authority,
            ) {
                Err(ResidentProcessHostSelectionError::CheckpointAuthorityAssociation)
            } else if access.store != &self.store {
                Err(ResidentProcessHostSelectionError::AccessStoreAssociation)
            } else if ready.store() != &self.store {
                Err(ResidentProcessHostSelectionError::RecoveredCollectionStoreAssociation)
            } else {
                validate_consultation_terminal_associations(
                    &selector,
                    &self.store,
                    &association,
                    &input,
                    &checkpoint,
                )
            };
        if let Err(reason) = selection {
            return Err(ResidentProcessConsultationTerminalJoinFailure::Selection {
                ready: Box::new(ResidentProcessConsultationCompletionReady { selector, ready }),
                reason,
            });
        }
        let ResidentDeploymentOperationAccess {
            custody, anchor, ..
        } = access;
        self.operations
            .with_operation_service(authority, custody, anchor, |service| {
                service.join_consultation_terminal(
                    *ready,
                    input,
                    checkpoint,
                    resolve_store,
                    admit_evidence,
                    admit_collection,
                    admit_rotation,
                    sign_observation,
                    sign_checkpoint,
                )
            })
            .map_err(|failure| map_process_consultation_terminal_failure(selector, failure))
    }
}
