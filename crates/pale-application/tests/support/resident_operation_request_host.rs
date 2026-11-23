//! Test-only composition of peer admission and one operation response.

use super::*;

pub(in crate::resident::process_host) fn answer<WriterGuard, Handle>(
    host: &mut ResidentOperationRequestHost<'_>,
    received: ReceivedResidentOperationFrame,
    resident: &mut ResidentProcessHost<'_, WriterGuard>,
    resolution: ResidentOperationRequestResolution<'_>,
    admit_peer: &mut PeerAdmission<'_, ()>,
    response_deadline: Instant,
    handle: Handle,
) -> Result<ResidentOperationFrameResponse, ResidentOperationRequestHostError>
where
    Handle: FnOnce(
        &mut ResidentProcessHost<'_, WriterGuard>,
        &CommonRequestEnvelopeV1,
    ) -> Result<ResidentOperationRequestOutcome, ResidentOperationFrameFailure>,
{
    let admitted = host.admit_peer(received, admit_peer)?;
    host.answer_admitted(admitted, resident, resolution, response_deadline, handle)
}
