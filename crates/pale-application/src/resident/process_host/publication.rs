//! Bounded Event publication turns for one selected deployment.

use pale_core::records::PrincipalResidentWriterFenceStateV1;
use pale_ports::{
    AuthorityStoreCoordinate, ControlTransactionPort, EventCustodyPort, OperationCustodyPort,
};

use super::{ResidentDeploymentAuthoritySelector, ResidentProcessHost};
use crate::resident::event_custody::{
    sweep_local_operation_publications, A2aTaskPublicationRecoveryLimits,
    A2aTaskPublicationRecoveryRequest, A2aTaskPublicationSourceRecoveryError,
    LocalOperationPublicationCurrent, LocalOperationPublicationIssuance,
    LocalOperationPublicationReadmission, LocalOperationPublicationRecoveryLimits,
    LocalOperationPublicationRecoveryRequest, LocalOperationPublicationSourceRecoveryError,
    LocalOperationPublicationSweepAttempt, LocalOperationPublicationSweepError,
    LocalOperationPublicationSweepLimits, LocalOperationPublicationSweepProgress,
    RecoveredA2aTaskPublicationSource,
};
use crate::resident::RecoveredOperationControl;

/// Store-bound operation sources and independent Event custody for one turn.
pub(super) struct ResidentDeploymentPublicationAccess<'a, EventPort> {
    pub(super) source_store: &'a AuthorityStoreCoordinate,
    pub(super) source_custody: &'a mut dyn OperationCustodyPort,
    pub(super) event_custody: &'a mut EventPort,
}

/// Recovery, signing, and work ceilings selected for one bounded turn.
pub(super) struct ResidentProcessPublicationTurn<'a> {
    pub(super) sweep_limits: LocalOperationPublicationSweepLimits,
    pub(super) local_recovery_limits: LocalOperationPublicationRecoveryLimits,
    pub(super) a2a_recovery_limits: A2aTaskPublicationRecoveryLimits,
    pub(super) current: LocalOperationPublicationCurrent<'a>,
    pub(super) issuance: LocalOperationPublicationIssuance<'a>,
    pub(super) readmission: LocalOperationPublicationReadmission<'a>,
}

impl<'store, WriterGuard> ResidentProcessHost<'store, WriterGuard> {
    /// Advance at most one publication stage for one selected deployment.
    ///
    /// Operation and A2A observations remain authoritative in operation custody.
    /// An unavailable Event store leaves the source pending and cannot change an
    /// operation result. A completed scan starts a fresh frozen prefix on the
    /// next turn so later observations can be found. A budget-limited scan keeps
    /// its exact cursor and counters; the caller must not mistake it for proof
    /// that the frozen prefix is complete.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn run_event_publication_turn<EventPort, RecoverLocal, RecoverA2a>(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentPublicationAccess<'_, EventPort>,
        mut turn: ResidentProcessPublicationTurn<'_>,
        recover_local: &mut RecoverLocal,
        recover_a2a: &mut RecoverA2a,
    ) -> Result<LocalOperationPublicationSweepProgress, ResidentProcessPublicationTurnError>
    where
        EventPort: EventCustodyPort + ControlTransactionPort,
        RecoverLocal: FnMut(
            &LocalOperationPublicationRecoveryRequest<'_>,
        ) -> Result<
            RecoveredOperationControl,
            LocalOperationPublicationSourceRecoveryError,
        >,
        RecoverA2a: FnMut(
            &A2aTaskPublicationRecoveryRequest<'_>,
        ) -> Result<
            RecoveredA2aTaskPublicationSource,
            A2aTaskPublicationSourceRecoveryError,
        >,
    {
        let index = self
            .deployment_authority_index(selector)
            .map_err(ResidentProcessPublicationTurnError::Selection)?;
        if self.deployments[index].store() != access.source_store {
            return Err(ResidentProcessPublicationTurnError::Selection(
                super::ResidentProcessHostSelectionError::AccessStoreAssociation,
            ));
        }
        if turn.current.producer.origin_principal_id() != &selector.principal_id
            || turn.current.writer_fence.principal_id() != &selector.principal_id
            || turn.current.writer_fence.state() != &PrincipalResidentWriterFenceStateV1::Active
            || !selector.matches_signing_authority(
                turn.current.signing_contexts.occurrence.signing_authority,
            )
            || !selector
                .matches_signing_authority(turn.current.signing_contexts.edge.signing_authority)
            || !selector
                .matches_signing_authority(turn.current.signing_contexts.frontier.signing_authority)
        {
            return Err(ResidentProcessPublicationTurnError::Selection(
                super::ResidentProcessHostSelectionError::PublicationCurrentAuthority,
            ));
        }

        let Self {
            publication_attempts,
            deployments,
            ..
        } = self;
        let application = deployments[index].application.as_mut();
        let progress = sweep_local_operation_publications(
            &mut publication_attempts[index],
            turn.sweep_limits,
            turn.local_recovery_limits,
            turn.a2a_recovery_limits,
            access.source_custody,
            application,
            access.event_custody,
            recover_local,
            recover_a2a,
            turn.current,
            &mut turn.issuance,
            &mut turn.readmission,
        )
        .map_err(ResidentProcessPublicationTurnError::Sweep)?;

        if matches!(
            progress,
            LocalOperationPublicationSweepProgress::SupportedLocalPrefixComplete { .. }
        ) {
            publication_attempts[index] =
                LocalOperationPublicationSweepAttempt::new(application.store().clone());
        }
        Ok(progress)
    }
}

/// One selected publication turn could not make safe progress.
#[derive(Debug, thiserror::Error)]
pub(super) enum ResidentProcessPublicationTurnError {
    /// The selector, source store, or current publication authority did not match.
    #[error(transparent)]
    Selection(super::ResidentProcessHostSelectionError),
    /// Source recovery or Event custody could not establish the next stage.
    #[error(transparent)]
    Sweep(LocalOperationPublicationSweepError),
}
