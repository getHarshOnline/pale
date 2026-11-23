//! Exercise the real frozen-prefix host sweep with dispatch forbidden.

use super::super::super::operation_recovery_sweep::{
    ResidentProcessOperationRecoveryDeferred, ResidentProcessOperationRecoveryTurn,
    ResidentProcessRootProviderRecoveryOwner, ResidentProcessRootProviderResolution,
};
use super::super::super::restart::{
    ResidentProcessRecoveredAttempt, ResidentProcessRecoveredLocalCancellation,
    ResidentProcessRecoveredOperationResolution, ResidentProcessRecoveredPreparedAttempt,
    ResidentProcessRecoveredProviderResolution,
};
use super::*;

struct NoConsultTasks {
    empty_polls: Option<usize>,
}

impl crate::resident::task_owner::ResidentOperationTaskExecutor<(), ()> for NoConsultTasks {
    fn try_accept(
        &mut self,
        _: crate::resident::task_owner::ResidentOperationTask<()>,
    ) -> Result<(), crate::resident::task_owner::ResidentOperationTask<()>> {
        panic!("expiry repair cannot dispatch consultation work")
    }

    fn try_completion(
        &mut self,
    ) -> Option<crate::resident::task_owner::ResidentOperationTaskCompletion<(), ()>> {
        let polls = self
            .empty_polls
            .as_mut()
            .expect("an unopened consultation has no work to poll");
        *polls += 1;
        None
    }
}

struct NoDispatch;

macro_rules! refuse {
    ($name:ident, $phase:ty) => {
        fn $name<'store>(
            &mut self,
            _: &mut ResidentProcessHost<'store, ()>,
            _: Box<$phase>,
            _: ResidentDeploymentOperationAccess<'_>,
        ) -> Result<(), String> {
            panic!("cold expiry recovery cannot dispatch")
        }
    };
}

impl ResidentProcessRootProviderRecoveryOwner<()> for NoDispatch {
    type Output = ();
    type Error = String;
    refuse!(
        complete_local_cancellation,
        ResidentProcessRecoveredLocalCancellation
    );
    refuse!(
        recover_provider_resolved,
        ResidentProcessRecoveredProviderResolution
    );
    refuse!(
        recover_provider_prepared,
        ResidentProcessRecoveredPreparedAttempt
    );
    refuse!(recover_provider_dispatched, ResidentProcessRecoveredAttempt);
    refuse!(
        recover_native_prepared,
        ResidentProcessRecoveredPreparedAttempt
    );
    refuse!(recover_native_dispatched, ResidentProcessRecoveredAttempt);

    fn recover_operation_resolution<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, ()>,
        _: Box<ResidentProcessRecoveredOperationResolution>,
        _: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<ResidentProcessRootProviderResolution<()>, String> {
        panic!("cold expiry recovery cannot resolve an implementation")
    }
}

impl RestartFixture<'_> {
    /// Exercise the Consult sweep before any provider sweep.
    /// An unopened Consult still refuses continuation after its domain is repaired.
    pub fn cold_consult_sweep_with_depth(
        &mut self,
        bounded: (
            LocalOperationRecoveryContext<'_>,
            AccumulatorPreparationLimits,
        ),
        maximum_parent_depth: usize,
        custody: &mut dyn OperationCustodyPort,
        sessions: &dyn SessionLeaseCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
        admission: OperationControlRecoveryAdmission<'_>,
    ) -> Result<OperationClassV1, String> {
        self.cold_consult_sweep_with_history(
            bounded,
            maximum_parent_depth,
            custody,
            sessions,
            anchor,
            admission,
            None,
            None,
        )
    }

    /// Exercise cold collection admission separately from current permission.
    #[allow(clippy::too_many_arguments)]
    pub fn cold_consult_sweep_with_history(
        &mut self,
        bounded: (
            LocalOperationRecoveryContext<'_>,
            AccumulatorPreparationLimits,
        ),
        maximum_parent_depth: usize,
        custody: &mut dyn OperationCustodyPort,
        sessions: &dyn SessionLeaseCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
        admission: OperationControlRecoveryAdmission<'_>,
        historical: Option<&mut crate::resident::HistoricalConsultationAdmission<'_>>,
        expected_stop: Option<(
            &crate::resident::RecoveredConsultationCollection,
            UnixTimeV1,
        )>,
    ) -> Result<OperationClassV1, String> {
        use super::super::super::consult_recovery::{
            ResidentProcessConsultRecoveryError, ResidentProcessConsultRecoveryOtherOwner,
            ResidentProcessConsultRecoveryTurn,
        };
        let (context, limits) = bounded;
        let restart = restart::ResidentProcessOperationRestartInput {
            committed_observation_limits: Some(limits),
            operation: context,
            maximum_parent_depth,
            current_head: restart::ResidentProcessRestartReadLimits {
                observation: context.observation_limits,
                checkpoints: context.checkpoint_limits,
                membership: context.membership_limits,
            },
            maximum_selection_bytes: context.maximum_record_bytes,
            a2a_task: None,
            a2a_cancel_task: None,
        };
        let mut executor = NoConsultTasks {
            empty_polls: expected_stop.map(|_| 0),
        };
        let turn = self.process.recover_one_root_consult_start_with_retention(
            &self.selector,
            ResidentDeploymentOperationAccess {
                store: context.store,
                custody,
                sessions,
                anchor,
            },
            restart,
            admission,
            context.maximum_record_bytes,
            &mut (),
            crate::resident::ConsultationCollectionLimits::new(1, 65_536, 65_536).unwrap(),
            crate::resident::process_host::ResidentConsultationTaskLimits::new(
                std::num::NonZeroUsize::new(1).unwrap(),
                std::num::NonZeroUsize::new(1).unwrap(),
            ),
            expected_stop.map_or(UnixTimeV1::new(70).unwrap(), |(_, at)| at),
            &|_: &OperationObservationV1| Ok(context.store.clone()),
            |collection| {
                let (expected, _) =
                    expected_stop.expect("an unopened consultation has no collection to admit");
                assert_eq!(collection, expected.collection());
                Ok(())
            },
            historical,
            |_| Ok(()),
            &mut executor,
        );
        if expected_stop.is_some()
            && !matches!(
                &turn,
                Ok(ResidentProcessConsultRecoveryTurn::StoppedEstablished(_))
            )
        {
            return Err("retained Consult start must recover StoppedEstablished".into());
        }
        let class = match turn {
            Ok(ResidentProcessConsultRecoveryTurn::StoppedEstablished(stopped)) => {
                let (expected, _) = expected_stop.expect("only the complete-start fixture can recover a stop");
                let (selector, parent, stop) = stopped.into_parts();
                assert_eq!(selector, self.selector);
                assert_eq!(&parent, expected.collection().parent());
                assert_eq!(Some(&stop), expected.collection().stopped());
                assert_eq!(executor.empty_polls, Some(1));
                OperationClassV1::Consult
            }
            Ok(ResidentProcessConsultRecoveryTurn::Pending(pending))
                if matches!(*pending, super::super::super::consult_recovery::ResidentProcessConsultRecoveryPending::RecoveryOnly(_)) => OperationClassV1::Consult,
            Ok(ResidentProcessConsultRecoveryTurn::SkippedOtherOwner(
                ResidentProcessConsultRecoveryOtherOwner::Invoke,
            )) => OperationClassV1::Invoke,
            Err(ResidentProcessConsultRecoveryError::Restart(
                restart::ResidentProcessOperationRestartError::State,
            )) => OperationClassV1::Consult,
            Err(ResidentProcessConsultRecoveryError::PendingObservation(error)) => {
                return Err(format!("{error:?}"))
            }
            Err(ResidentProcessConsultRecoveryError::Restart(error)) => {
                return Err(format!("{error:?}"))
            }
            Err(ResidentProcessConsultRecoveryError::Discovery(error)) => {
                return Err(format!("discovery: {error:?}"))
            }
            Err(ResidentProcessConsultRecoveryError::Source(error)) => {
                return Err(format!("source: {error:?}"))
            }
            Err(ResidentProcessConsultRecoveryError::Start(
                super::super::super::consult_start::ResidentProcessConsultationResumeError::Selection(error),
            )) => return Err(format!("consult start selection: {error:?}")),
            _ => panic!("expiry repair must leave the original semantic owner responsible"),
        };
        assert_eq!(self.process.operations.outstanding_task_reservations(), 0);
        Ok(class)
    }

    /// Discover and repair through the actual host sweep, then require recovery-only classification.
    pub fn cold_sweep(
        &mut self,
        bounded: (
            LocalOperationRecoveryContext<'_>,
            AccumulatorPreparationLimits,
        ),
        custody: &mut dyn OperationCustodyPort,
        sessions: &dyn SessionLeaseCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
        admission: OperationControlRecoveryAdmission<'_>,
    ) -> Result<(), String> {
        self.cold_sweep_with_depth(bounded, 0, custody, sessions, anchor, admission)
            .map(|class| assert_eq!(class, OperationClassV1::Invoke))
    }

    /// Exercise descendant repair and report recovery-only or defer its Consult root.
    pub fn cold_sweep_with_depth(
        &mut self,
        bounded: (
            LocalOperationRecoveryContext<'_>,
            AccumulatorPreparationLimits,
        ),
        maximum_parent_depth: usize,
        custody: &mut dyn OperationCustodyPort,
        sessions: &dyn SessionLeaseCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
        admission: OperationControlRecoveryAdmission<'_>,
    ) -> Result<OperationClassV1, String> {
        let (context, limits) = bounded;
        let restart = restart::ResidentProcessOperationRestartInput {
            committed_observation_limits: Some(limits),
            operation: context,
            maximum_parent_depth,
            current_head: restart::ResidentProcessRestartReadLimits {
                observation: context.observation_limits,
                checkpoints: context.checkpoint_limits,
                membership: context.membership_limits,
            },
            maximum_selection_bytes: context.maximum_record_bytes,
            a2a_task: None,
            a2a_cancel_task: None,
        };
        let turn = self
            .process
            .recover_one_root_provider_start(
                &self.selector,
                ResidentDeploymentOperationAccess {
                    store: context.store,
                    custody,
                    sessions,
                    anchor,
                },
                restart,
                admission,
                context.maximum_record_bytes,
                &mut NoDispatch,
            )
            .map_err(|error| format!("{error:?}"))?;
        let class = match turn {
            ResidentProcessOperationRecoveryTurn::ReportedRecoveryOnly(start) => {
                assert_eq!(
                    start
                        .control
                        .operation()
                        .operation()
                        .handle()
                        .1
                        .operation_class(),
                    &OperationClassV1::Invoke,
                );
                OperationClassV1::Invoke
            }
            ResidentProcessOperationRecoveryTurn::DeferredUnsupported {
                owner: ResidentProcessOperationRecoveryDeferred::Consult,
                disposition: Some(disposition),
            } => {
                let restart::ResidentProcessOperationRestartDisposition::OperationResolution(value) =
                    *disposition
                else {
                    panic!("the Consult root must retain its original resolving phase");
                };
                assert_eq!(
                    value.phase,
                    restart::ResidentProcessOperationResolutionPhase::Resolving
                );
                let handle = value.start.control.operation().operation().handle().1;
                assert_eq!(handle.operation_class(), &OperationClassV1::Consult);
                assert!(handle.parent_operation().is_none());
                OperationClassV1::Consult
            }
            _ => panic!("expiry recovery must report recovery-only or defer Consult"),
        };
        assert_eq!(self.process.operations.outstanding_task_reservations(), 0);
        Ok(class)
    }
}
