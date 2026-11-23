//! Retained composition for provider and A2A Delegate recovery owners.
//!
//! Cold recovery first classifies one exact Delegate phase. This owner then
//! forwards generic provider phases to the provider owner and A2A phases to the
//! A2A owner. It does not choose a route, allocate evidence, or call a carrier.
//! Those inputs remain with the injected phase owners, which must use the
//! existing process-host journeys and preserve every linear continuation.

use super::delegate_recovery_sweep::ResidentProcessDelegateRecoveryOwner;
use super::operation_recovery_sweep::{
    ResidentProcessRootProviderRecoveryOwner, ResidentProcessRootProviderResolution,
};
use super::restart::{
    ResidentProcessRecoveredA2aCancelTaskRequest, ResidentProcessRecoveredA2aTask,
    ResidentProcessRecoveredAttempt, ResidentProcessRecoveredLocalCancellation,
    ResidentProcessRecoveredOperationResolution, ResidentProcessRecoveredPreparedAttempt,
    ResidentProcessRecoveredProviderResolution,
};
use super::{ResidentDeploymentOperationAccess, ResidentProcessHost};

/// A2A-specific recovery phases retained by the resident composition root.
///
/// Each successful call must durably close the supplied phase or retain its
/// exact successor. The port receives no method that can repeat `SendMessage`.
pub(super) trait ResidentA2aDelegateRecoveryPort<WriterGuard> {
    /// Result shared with the resident's generic provider recovery owner.
    type Output;
    /// Failure that preserves any unfinished linear continuation.
    type Error;

    /// Continue one point-addressed Delegate from admitted resolution facts.
    ///
    /// The concrete port may consume an already-admitted exact A2A route and a
    /// one-shot send owner. This interface does not supply a route selector or
    /// a replayable carrier.
    fn recover_delegate_resolution<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<ResidentProcessRecoveredOperationResolution>,
        access: ResidentDeploymentOperationAccess<'_>,
        input: super::program::ResidentProgramDelegateResolutionInput<'_, '_>,
    ) -> Result<Self::Output, Self::Error>;

    /// Continue a retained send whose durable evidence identifies no remote Task.
    ///
    /// This phase can reconcile uncertainty or preserve terminal no-task
    /// evidence. It cannot send the message again.
    fn recover_pre_task<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<ResidentProcessRecoveredAttempt>,
        access: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error>;

    /// Continue observation of one exact identified remote Task.
    fn recover_task<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<ResidentProcessRecoveredA2aTask>,
        access: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error>;

    /// Continue read-back for one retained remote cancellation request.
    fn recover_cancel_task<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<ResidentProcessRecoveredA2aCancelTaskRequest>,
        access: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error>;
}

/// One retained owner that keeps generic provider and A2A recovery separate.
///
/// The generic owner remains responsible for implementation resolution and
/// non-A2A provider/native phases. The A2A owner receives only a phase already
/// classified from durable evidence. This prevents a route adapter from
/// becoming the owner of provider selection or generic operation recovery.
pub(super) struct ResidentA2aDelegateRecoveryOwner<Provider, A2a> {
    provider: Provider,
    a2a: A2a,
}

impl<Provider, A2a> ResidentA2aDelegateRecoveryOwner<Provider, A2a> {
    /// Retain the two existing phase owners without selecting a route.
    pub(super) const fn new(provider: Provider, a2a: A2a) -> Self {
        Self { provider, a2a }
    }
}

#[cfg(test)]
#[path = "../../../tests/support/a2a_delegate_recovery_owner.rs"]
pub(in crate::resident::process_host) mod a2a_recovery_owner_test_support;

impl<WriterGuard, Provider, A2a> ResidentProcessRootProviderRecoveryOwner<WriterGuard>
    for ResidentA2aDelegateRecoveryOwner<Provider, A2a>
where
    Provider: ResidentProcessRootProviderRecoveryOwner<WriterGuard>,
{
    type Output = Provider::Output;
    type Error = Provider::Error;

    fn complete_local_cancellation<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<ResidentProcessRecoveredLocalCancellation>,
        access: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        self.provider
            .complete_local_cancellation(host, recovered, access)
    }

    fn recover_operation_resolution<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<ResidentProcessRecoveredOperationResolution>,
        access: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<ResidentProcessRootProviderResolution<Self::Output>, Self::Error> {
        self.provider
            .recover_operation_resolution(host, recovered, access)
    }

    fn recover_provider_resolved<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<ResidentProcessRecoveredProviderResolution>,
        access: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        self.provider
            .recover_provider_resolved(host, recovered, access)
    }

    fn recover_provider_prepared<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<ResidentProcessRecoveredPreparedAttempt>,
        access: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        self.provider
            .recover_provider_prepared(host, recovered, access)
    }

    fn recover_provider_dispatched<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<ResidentProcessRecoveredAttempt>,
        access: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        self.provider
            .recover_provider_dispatched(host, recovered, access)
    }

    fn recover_native_prepared<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<ResidentProcessRecoveredPreparedAttempt>,
        access: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        self.provider
            .recover_native_prepared(host, recovered, access)
    }

    fn recover_native_dispatched<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<ResidentProcessRecoveredAttempt>,
        access: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        self.provider
            .recover_native_dispatched(host, recovered, access)
    }
}

impl<WriterGuard, Provider, A2a> ResidentProcessDelegateRecoveryOwner<WriterGuard>
    for ResidentA2aDelegateRecoveryOwner<Provider, A2a>
where
    Provider: ResidentProcessRootProviderRecoveryOwner<WriterGuard>,
    A2a: ResidentA2aDelegateRecoveryPort<
        WriterGuard,
        Output = Provider::Output,
        Error = Provider::Error,
    >,
{
    fn recover_delegate_operation_resolution<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<ResidentProcessRecoveredOperationResolution>,
        access: ResidentDeploymentOperationAccess<'_>,
        input: super::program::ResidentProgramDelegateResolutionInput<'_, '_>,
    ) -> Result<Self::Output, Self::Error> {
        self.a2a
            .recover_delegate_resolution(host, recovered, access, input)
    }

    fn recover_a2a_pre_task<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<ResidentProcessRecoveredAttempt>,
        access: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        self.a2a.recover_pre_task(host, recovered, access)
    }

    fn recover_a2a_task<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<ResidentProcessRecoveredA2aTask>,
        access: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        self.a2a.recover_task(host, recovered, access)
    }

    fn recover_a2a_cancel_task<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<ResidentProcessRecoveredA2aCancelTaskRequest>,
        access: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        self.a2a.recover_cancel_task(host, recovered, access)
    }
}
