//! Proof-bearing custody for one Package-selected consultation stop.
//!
//! The bounded supervision turn selects a stop but performs no physical work.
//! This module consumes that owned phase through the existing collection stop
//! owner. Admission failure returns the selected phase. Physical uncertainty
//! returns the exact database or anchor continuation.

use pale_core::records::*;
use pale_ports::{AntiRollbackAnchorPort, OperationCustodyPort};

use super::{
    commit_recovered_consultation_stop, CompletedConsultationCollection,
    ConsultationCollectionCommitFailure, ConsultationCollectionError, ConsultationStopFailure,
    ConsultationStopInput, ConsultationSuccessorCommitInput, ResidentConsultationStopSelected,
};
use crate::resident::OperationObservationAuthorityStoreResolver;

/// Exact physical stop candidate retained after admission completed.
#[must_use = "continue the exact stop custody candidate"]
pub(in crate::resident) struct ResidentConsultationStopCustody {
    failure: ConsultationCollectionCommitFailure,
}

impl std::fmt::Debug for ResidentConsultationStopCustody {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ResidentConsultationStopCustody")
            .finish_non_exhaustive()
    }
}

/// Closed failure while committing one Package-selected stop.
#[must_use = "preserve the selected stop or exact custody continuation"]
#[allow(
    dead_code,
    reason = "private stop continuations are consumed by the assembled resident task owner"
)]
pub(in crate::resident) enum ResidentConsultationStopCommitFailure<'operation> {
    /// Semantic or signing admission rejected before physical custody began.
    Admission {
        /// Unchanged selected stop, including its recovered collection.
        selected: Box<ResidentConsultationStopSelected<ConsultationStopInput<'operation>>>,
        /// Exact application admission failure.
        reason: ConsultationCollectionError,
    },
    /// Database or independent-anchor custody remains incomplete.
    Custody(Box<ResidentConsultationStopCustody>),
}

impl std::fmt::Debug for ResidentConsultationStopCommitFailure<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::Admission { .. } => "Admission",
            Self::Custody(_) => "Custody",
        };
        formatter
            .debug_struct("ResidentConsultationStopCommitFailure")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

/// Commit one selected stop through its existing semantic and physical owners.
#[allow(clippy::too_many_arguments)]
pub(in crate::resident) fn commit_selected_consultation_stop<
    'operation,
    AdmitStop,
    AdmitCollection,
    Rotation,
    ObservationSign,
    CheckpointSign,
>(
    selected: ResidentConsultationStopSelected<ConsultationStopInput<'operation>>,
    checkpoint: ConsultationSuccessorCommitInput<'_>,
    resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    admit_stop: AdmitStop,
    admit_collection: AdmitCollection,
    admit_rotation: Rotation,
    sign_observation: ObservationSign,
    sign_checkpoint: CheckpointSign,
) -> Result<CompletedConsultationCollection, ResidentConsultationStopCommitFailure<'operation>>
where
    AdmitStop: FnOnce(
        &super::ConsultationCollection,
        &super::ConsultationRequirementSummary,
    ) -> Result<(), InvalidResidentRecordV1>,
    AdmitCollection: FnOnce(&super::ConsultationCollection) -> Result<(), InvalidResidentRecordV1>,
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
    let (current, input) = selected.into_parts();
    let retained_input = input.clone();
    match commit_recovered_consultation_stop(
        &current,
        input,
        checkpoint,
        resolve_store,
        custody,
        anchor,
        admit_stop,
        admit_collection,
        admit_rotation,
        sign_observation,
        sign_checkpoint,
    ) {
        Ok(completed) => Ok(completed),
        Err(ConsultationStopFailure::Admission(reason)) => {
            Err(ResidentConsultationStopCommitFailure::Admission {
                selected: Box::new(ResidentConsultationStopSelected::from_parts(
                    current,
                    retained_input,
                )),
                reason,
            })
        }
        Err(ConsultationStopFailure::Custody(failure)) => {
            Err(ResidentConsultationStopCommitFailure::Custody(Box::new(
                ResidentConsultationStopCustody { failure },
            )))
        }
    }
}

/// Continue only the retained database or anchor candidate for one stop.
pub(in crate::resident) fn continue_consultation_stop_custody(
    pending: ResidentConsultationStopCustody,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
) -> Result<CompletedConsultationCollection, ResidentConsultationStopCustody> {
    match pending.failure.continue_custody(custody, anchor) {
        Ok(completed) => Ok(completed),
        Err(failure) => Err(ResidentConsultationStopCustody { failure }),
    }
}
