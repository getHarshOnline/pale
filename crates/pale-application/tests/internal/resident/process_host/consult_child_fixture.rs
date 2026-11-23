//! Shared process-host fixtures for a real Consult child.

use std::collections::VecDeque;
use std::num::NonZeroUsize;

use pale_core::records::ControlStateCheckpointV1;
use pale_ports::*;

use crate::local_operation_start_fixture::{
    host_source_generation_input_copies, source_generation_input_copies_with, RecoveryParents,
};
use crate::resident::consultation::tests::{
    with_issued_invoke_child_named, with_issued_provider_invoke_child_named, IssuedTestChild,
    IssuedTestChildLeaseCustody,
};
use crate::resident::consultation::CompletedResidentConsultationTerminal;
use crate::resident::local_operation_start::IssuedLocalOperation;
use crate::resident::process_host::ResidentProcessConsultationTerminalJoinFailure;
use crate::resident::task_owner::{
    ResidentOperationTask, ResidentOperationTaskCompletion, ResidentOperationTaskExecutor,
};
use crate::resident::AdmittedCompiledGeneration;

/// Retain one accepted child task until the test supplies its exact completion.
pub(super) struct DeferredConsultChildExecutor<Work, Output> {
    accepted: Option<ResidentOperationTask<Work>>,
    completions: VecDeque<ResidentOperationTaskCompletion<Work, Output>>,
    next_acceptance: DeferredConsultChildAcceptance,
    acceptance_attempts: usize,
    completion_attempts: usize,
}

#[derive(Clone, Copy, Default, Eq, PartialEq)]
enum DeferredConsultChildAcceptance {
    #[default]
    Accept,
    Refuse,
}

impl<Work, Output> Default for DeferredConsultChildExecutor<Work, Output> {
    fn default() -> Self {
        Self {
            accepted: None,
            completions: VecDeque::new(),
            next_acceptance: DeferredConsultChildAcceptance::Accept,
            acceptance_attempts: 0,
            completion_attempts: 0,
        }
    }
}

impl<Work, Output> DeferredConsultChildExecutor<Work, Output> {
    pub(super) fn take_accepted(&mut self) -> ResidentOperationTask<Work> {
        self.accepted.take().expect("one accepted child task")
    }

    pub(super) fn complete(&mut self, task: ResidentOperationTask<Work>, output: Output) {
        self.completions
            .push_back(ResidentOperationTaskCompletion::Completed { task, output });
    }

    pub(super) fn lose(&mut self, task: ResidentOperationTask<Work>) {
        self.completions.push_back(task.into_lost());
    }

    pub(super) fn refuse_next(&mut self) {
        self.next_acceptance = DeferredConsultChildAcceptance::Refuse;
    }

    pub(super) const fn acceptance_attempts(&self) -> usize {
        self.acceptance_attempts
    }

    pub(super) const fn completion_attempts(&self) -> usize {
        self.completion_attempts
    }
}

impl<Work, Output> ResidentOperationTaskExecutor<Work, Output>
    for DeferredConsultChildExecutor<Work, Output>
{
    fn try_accept(
        &mut self,
        task: ResidentOperationTask<Work>,
    ) -> Result<(), ResidentOperationTask<Work>> {
        self.acceptance_attempts += 1;
        if self.next_acceptance == DeferredConsultChildAcceptance::Refuse {
            self.next_acceptance = DeferredConsultChildAcceptance::Accept;
            return Err(task);
        }
        if self.accepted.is_some() {
            return Err(task);
        }
        self.accepted = Some(task);
        Ok(())
    }

    fn try_completion(&mut self) -> Option<ResidentOperationTaskCompletion<Work, Output>> {
        self.completion_attempts += 1;
        self.completions.pop_front()
    }
}

/// Issue one real Invoke child under a Consult parent and retain a matching
/// generation for process-host recovery.
#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum ConsultChildFixtureKind {
    SelfContained,
    ProviderBacked,
}

pub(super) fn with_process_consult_invoke_child<R>(
    label: &str,
    kind: ConsultChildFixtureKind,
    run: impl FnOnce(
        &IssuedTestChild,
        &IssuedLocalOperation,
        OperationCustodyCandidate,
        ControlStateCheckpointV1,
        IssuedTestChildLeaseCustody,
        &AdmittedCompiledGeneration,
        RecoveryParents<'_>,
        AdmittedCompiledGeneration,
    ) -> R,
) -> R {
    with_process_consult_invoke_child_generations(
        label,
        kind,
        NonZeroUsize::new(1).expect("one retained host generation"),
        |child,
         parent,
         parent_candidate,
         parent_checkpoint,
         lease_custody,
         admitted_generation,
         parents,
         mut host_generations| {
            let host_generation = host_generations
                .pop()
                .expect("the wrapper requests one retained host generation");
            assert!(host_generations.is_empty());
            run(
                child,
                parent,
                parent_candidate,
                parent_checkpoint,
                lease_custody,
                admitted_generation,
                parents,
                host_generation,
            )
        },
    )
}

/// Issue one child with a bounded number of independently admitted, identical
/// generations for tests that create more than one cold process host.
#[allow(clippy::too_many_arguments)]
pub(super) fn with_process_consult_invoke_child_generations<R>(
    label: &str,
    kind: ConsultChildFixtureKind,
    host_count: NonZeroUsize,
    run: impl FnOnce(
        &IssuedTestChild,
        &IssuedLocalOperation,
        OperationCustodyCandidate,
        ControlStateCheckpointV1,
        IssuedTestChildLeaseCustody,
        &AdmittedCompiledGeneration,
        RecoveryParents<'_>,
        Vec<AdmittedCompiledGeneration>,
    ) -> R,
) -> R {
    let copy_count = host_count
        .get()
        .checked_add(1)
        .expect("bounded Consult-child generation count");
    let mut source_generations = match kind {
        ConsultChildFixtureKind::ProviderBacked => {
            host_source_generation_input_copies("consult-child", copy_count)
        }
        ConsultChildFixtureKind::SelfContained => {
            source_generation_input_copies_with(|_| {}, |_| {}, copy_count)
        }
    }
    .into_iter();
    let (compiled, materializations, profile) = source_generations
        .next()
        .expect("Consult-child fixture generation");
    let generation = AdmittedCompiledGeneration::admit(
        compiled,
        materializations,
        crate::resident_fixture::synthetic_set_policy(),
    )
    .expect("compiled Consult-child generation");
    let host_generations = source_generations
        .map(|(compiled, materializations, _)| {
            AdmittedCompiledGeneration::admit(
                compiled,
                materializations,
                crate::resident_fixture::synthetic_set_policy(),
            )
            .expect("retained Consult-child generation")
        })
        .collect::<Vec<_>>();
    assert_eq!(host_generations.len(), host_count.get());

    let run = move |child: &IssuedTestChild,
                    parent: &IssuedLocalOperation,
                    parent_candidate: OperationCustodyCandidate,
                    parent_checkpoint: ControlStateCheckpointV1,
                    lease_custody: IssuedTestChildLeaseCustody,
                    admitted_generation: &AdmittedCompiledGeneration,
                    parents: RecoveryParents<'_>| {
        run(
            child,
            parent,
            parent_candidate,
            parent_checkpoint,
            lease_custody,
            admitted_generation,
            parents,
            host_generations,
        )
    };
    match kind {
        ConsultChildFixtureKind::ProviderBacked => {
            with_issued_provider_invoke_child_named(&generation, &profile, label, run)
        }
        ConsultChildFixtureKind::SelfContained => {
            with_issued_invoke_child_named(&generation, &profile, label, run)
        }
    }
}

/// Require the exact terminal Consult-child phase and explain any retained
/// continuation by its owning phase.
pub(super) fn expect_consult_child_terminal_join<Work, Output>(
    joined: Result<
        CompletedResidentConsultationTerminal<Work, Output>,
        ResidentProcessConsultationTerminalJoinFailure<Work, Output>,
    >,
) -> CompletedResidentConsultationTerminal<Work, Output> {
    match joined {
        Ok(completed) => completed,
        Err(ResidentProcessConsultationTerminalJoinFailure::Selection { reason, .. }) => {
            panic!("terminal join selection: {reason:?}")
        }
        Err(ResidentProcessConsultationTerminalJoinFailure::Preflight { reason, .. }) => {
            panic!("terminal join preflight: {reason:?}")
        }
        Err(ResidentProcessConsultationTerminalJoinFailure::Admission { reason, .. }) => {
            panic!("terminal join admission: {reason:?}")
        }
        Err(ResidentProcessConsultationTerminalJoinFailure::Custody(_)) => {
            panic!("terminal join custody")
        }
        Err(ResidentProcessConsultationTerminalJoinFailure::CustodyPreflight { .. }) => {
            panic!("terminal join custody preflight")
        }
        Err(ResidentProcessConsultationTerminalJoinFailure::CustodySelection { .. }) => {
            panic!("terminal join custody selection")
        }
        Err(ResidentProcessConsultationTerminalJoinFailure::Release { .. }) => {
            panic!("terminal join release")
        }
        Err(ResidentProcessConsultationTerminalJoinFailure::ReleasePreflight { .. }) => {
            panic!("terminal join release preflight")
        }
        Err(ResidentProcessConsultationTerminalJoinFailure::ReleaseSelection { .. }) => {
            panic!("terminal join release selection")
        }
    }
}
