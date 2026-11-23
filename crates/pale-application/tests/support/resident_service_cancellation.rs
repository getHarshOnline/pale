//! Cancellation input owner used when a test intentionally omits that route.

use super::*;

pub(in crate::resident::process_host) struct UnavailableResidentServiceCancellationInputs;

impl<'service> ResidentServiceCancellationInputPort<'service>
    for UnavailableResidentServiceCancellationInputs
{
    fn with_cancellation_inputs<UseInputs>(
        &mut self,
        _: &ResidentServiceDeploymentSelector,
        _: &CommonRequestEnvelopeV1,
        _: &ControlRecordCoordinate,
        _: UseInputs,
    ) -> Result<(), ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(ResidentServiceCancellationInputs<'input>),
    {
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    fn with_a2a_cancellation_outcome_inputs<Output, UseInputs>(
        self,
        _: &ResidentServiceDeploymentSelector,
        _: &ResidentRequestIdV1,
        _: &CommonRequestEnvelopeDigest,
        _: &ControlRecordCoordinate,
        outcome: Box<crate::resident::A2aSendOutcomeUnknown>,
        _: UseInputs,
    ) -> (
        Self,
        Result<
            Output,
            (
                ResidentOperationFrameFailure,
                Box<crate::resident::A2aSendOutcomeUnknown>,
            ),
        >,
    )
    where
        UseInputs: FnOnce(crate::resident::A2aUnknownOutcomeRetention<'service>) -> Output,
    {
        (
            self,
            Err((
                ResidentOperationFrameFailure::ConfigurationUnavailable,
                outcome,
            )),
        )
    }
}
