//! Test-only construction of an already-proved provider dispatch.

use super::*;

pub(in crate::resident) fn recovered_provider_dispatch(
    dispatch: Arc<PreparedRecoveredOperationDispatch>,
    request: CommonRequestEnvelopeV1,
    provider_request: OwnedProviderInvocationRequest,
    admission: crate::provider_invocation::OwnedProviderInvocationAdmission,
) -> PreparedRecoveredProviderDispatch {
    PreparedRecoveredProviderDispatch {
        dispatch,
        request,
        provider_request,
        admission,
    }
}
