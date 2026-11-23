//! Test-only construction of the custody-complete Consult release boundary.

use super::*;

impl<Work, Output> ResidentConsultationJoinedTerminal<Work, Output> {
    /// Commit the exact parent join while deliberately retaining aggregate
    /// release for a separate behavioral retry.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn commit_without_aggregate_release_for_test<
        'operation,
        AdmitEvidence,
        AdmitCollection,
        Rotation,
        ObservationSign,
        CheckpointSign,
    >(
        tasks: &ResidentConsultationTaskOwner,
        reservations: &ResidentAggregateTaskReservations,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
        ready: ResidentConsultationCompletionReady<Work, Output>,
        input: ConsultationChildJoinInput<'operation>,
        checkpoint: ConsultationSuccessorCommitInput<'_>,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        admit_evidence: AdmitEvidence,
        admit_collection: AdmitCollection,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<Self, ResidentConsultationTerminalJoinFailure<Work, Output>>
    where
        AdmitEvidence: FnOnce(
            &super::super::ConsultationRosterSlot,
            &ImmutableRecordReferenceV1,
        ) -> Result<(), InvalidResidentRecordV1>,
        AdmitCollection:
            FnOnce(&super::super::ConsultationCollection) -> Result<(), InvalidResidentRecordV1>,
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
        let (current, poll) = ready.into_parts();
        if let Err(reason) = tasks.require_terminal_join(&poll, &current, &input, reservations) {
            return Err(ResidentConsultationTerminalJoinFailure::Preflight {
                ready: Box::new(ResidentConsultationCompletionReady::from_parts(
                    current, poll,
                )),
                reason,
            });
        }
        match ResidentConsultationCoordinator::new(custody, anchor).join_child(
            &current,
            input,
            checkpoint,
            resolve_store,
            admit_evidence,
            admit_collection,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        ) {
            Ok(collection) => Ok(Self { collection, poll }),
            Err(ConsultationChildJoinFailure::Admission(reason)) => {
                Err(ResidentConsultationTerminalJoinFailure::Admission {
                    ready: Box::new(ResidentConsultationCompletionReady::from_parts(
                        current, poll,
                    )),
                    reason,
                })
            }
            Err(ConsultationChildJoinFailure::Custody(failure)) => {
                Err(ResidentConsultationTerminalJoinFailure::Custody(Box::new(
                    ResidentConsultationTerminalCustody { failure, poll },
                )))
            }
        }
    }
}
