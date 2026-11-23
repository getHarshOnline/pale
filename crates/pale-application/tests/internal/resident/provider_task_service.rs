//! Test-only construction for the provider task owner.

use super::*;
use pale_ports::AuthorityStoreCoordinate;

impl ResidentProviderTaskService {
    pub(in crate::resident) fn new(
        limits: ResidentOperationTaskLimits,
    ) -> Result<Self, ResidentOperationTaskOwnerBuildError> {
        Self::new_in(ResidentOperationTaskOwnerInstance::new(), limits)
    }

    pub(in crate::resident) fn test_reservations(&self) -> ResidentAggregateTaskReservations {
        ResidentAggregateTaskReservations::new(
            self.owner.owner_instance().clone(),
            crate::resident::task_reservation::ResidentAggregateTaskLimits::new(
                std::num::NonZeroUsize::new(1).expect("positive provider test aggregate limit"),
            ),
        )
        .expect("provider test aggregate ledger")
    }

    pub(in crate::resident) fn continue_custody_with_finalization_mismatch_for_test<Authority>(
        &mut self,
        coordinator: &mut ResidentLocalOperationCoordinator<'_, Authority>,
        custody: ResidentProviderRecordingCustody,
        reservations: &mut ResidentAggregateTaskReservations,
        substituted_store: AuthorityStoreCoordinate,
    ) -> Result<ResidentProviderTaskPoll<'static>, ResidentProviderRecordingCustodyRejection> {
        if let Err(reason) = custody.pending.require(reservations) {
            return Err(ResidentProviderRecordingCustodyRejection {
                custody: Box::new(custody),
                reason,
            });
        }
        let ResidentProviderRecordingCustody { custody, pending } = custody;
        let finalization = match coordinator.continue_provider_recording_custody(custody) {
            Ok(finalization) => finalization.substitute_expected_store_for_test(substituted_store),
            Err(custody) => {
                return Ok(ResidentProviderTaskPoll::Custody(Box::new(
                    ResidentProviderRecordingCustody { custody, pending },
                )));
            }
        };
        Ok(Self::finish_finalization_inner(
            finalization,
            pending,
            reservations,
        ))
    }

    pub(in crate::resident) fn restore_finalization_for_test(
        &mut self,
        finalization: ResidentProviderRecordingFinalization,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> Result<ResidentProviderTaskPoll<'static>, ResidentProviderRecordingFinalizationRejection>
    {
        if let Err(reason) = finalization.pending.require(reservations) {
            return Err(ResidentProviderRecordingFinalizationRejection {
                finalization: Box::new(finalization),
                reason,
            });
        }
        let ResidentProviderRecordingFinalization {
            finalization,
            pending,
        } = finalization;
        Ok(Self::finish_finalization_inner(
            finalization.restore_expected_store_for_test(),
            pending,
            reservations,
        ))
    }
}
