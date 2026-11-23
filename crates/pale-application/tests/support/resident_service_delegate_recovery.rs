//! Test-only access to retained Delegate completion phases.

use super::*;

pub(in crate::resident::process_host) fn retain_completed<'service, Inputs>(
    adapter: &mut ResidentServiceDelegateRecoveryAdapter<'service, Inputs>,
    selector: crate::resident::process_host::ResidentDeploymentAuthoritySelector,
    locator: ResidentOperationFrameLocator,
    completed: ResidentProcessA2aDelegateCompletion,
) -> Result<operation_request_host::ResidentOperationRequestOutcome, ResidentOperationFrameFailure>
{
    adapter.retain(
        selector,
        locator,
        ResidentServiceRetainedDelegatePhase::Launch(ResidentProcessA2aDelegateLaunch::Completed(
            completed,
        )),
    )
}

pub(in crate::resident::process_host) fn take_retained_completed<'service, Inputs>(
    adapter: &mut ResidentServiceDelegateRecoveryAdapter<'service, Inputs>,
) -> Option<ResidentProcessA2aDelegateCompletion> {
    if adapter.retained.len() != 1 {
        return None;
    }
    let mut retained = adapter.retained.pop()?;
    match retained.phase.take()? {
        ResidentServiceRetainedDelegatePhase::Launch(
            ResidentProcessA2aDelegateLaunch::Completed(completed),
        ) => Some(completed),
        phase => {
            retained.phase = Some(phase);
            adapter.retained.push(retained);
            None
        }
    }
}
