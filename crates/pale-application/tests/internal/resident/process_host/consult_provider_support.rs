//! Test-only projection of a retained provider loss phase.

use super::*;

pub(in crate::resident) fn project_cold_loss_offered_for_test<'input, 'operation, 'call>(
    selector: &ResidentDeploymentAuthoritySelector,
    turn: ResidentProcessConsultProviderColdLossTurn<'input, 'operation, 'call>,
    pending: Box<crate::resident::consultation::ResidentConsultationTaskLossOffer>,
    reason: crate::resident::task_reservation::ResidentTaskReservationTransitionFailure,
) -> Box<ResidentProcessConsultProviderColdLossFailure<'input, 'operation, 'call>> {
    let ResidentProcessConsultProviderColdLossTurn {
        attempt,
        access,
        task_limits: _,
        current,
        candidate: _,
        history,
        input,
        evidence,
        recovery,
        recording,
    } = turn;
    cold_loss_local_failure(
        Box::new(ResidentProcessConsultProviderColdLossRecording {
            selector: selector.clone(),
            access,
            current,
            history,
            recovered_dispatch: attempt.start.control.observation(),
            input,
            evidence,
            recovery,
            recording,
        }),
        ResidentConsultationTaskLossCommitFailure::Offered { pending, reason },
    )
}
