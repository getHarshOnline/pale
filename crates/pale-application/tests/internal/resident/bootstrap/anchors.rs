//! Bootstrap anchor ordering, partial completion and recovery behavior.

use super::*;
use crate::resident::AnchorAdvanceEvidence;
use pale_ports::{
    AntiRollbackAnchor, AntiRollbackAnchorAdvanceDisposition as Advance, AntiRollbackAnchorError,
    AntiRollbackAnchorErrorKind as ErrorKind, AntiRollbackAnchorKey,
    AntiRollbackAnchorObservation as Observed, AntiRollbackAnchorPort,
};
use std::collections::VecDeque;

enum Step {
    Advance(Result<Advance, ErrorKind>),
    Observe(Result<Observed, ErrorKind>),
}
struct Anchors {
    steps: VecDeque<Step>,
    calls: Vec<(bool, PreparedAntiRollbackAnchorTransition)>,
}
impl Anchors {
    fn new(steps: impl IntoIterator<Item = Step>) -> Self {
        Self {
            steps: steps.into_iter().collect(),
            calls: Vec::new(),
        }
    }
}
impl AntiRollbackAnchorPort for Anchors {
    fn read(
        &mut self,
        _: &AntiRollbackAnchorKey,
    ) -> Result<Option<AntiRollbackAnchor>, AntiRollbackAnchorError> {
        panic!("no unbound discovery")
    }
    fn compare_and_advance(
        &mut self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<Advance, AntiRollbackAnchorError> {
        self.calls.push((true, transition.clone()));
        let Some(Step::Advance(value)) = self.steps.pop_front() else {
            panic!("unexpected CAS or implicit retry")
        };
        value.map_err(AntiRollbackAnchorError::new)
    }
    fn observe(
        &mut self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<Observed, AntiRollbackAnchorError> {
        self.calls.push((false, transition.clone()));
        let Some(Step::Observe(value)) = self.steps.pop_front() else {
            panic!("unexpected observation")
        };
        value.map_err(AntiRollbackAnchorError::new)
    }
}
fn committed(
    disposition: PrincipalBootstrapCommitDisposition,
) -> DatabaseCommittedPrincipalBootstrapAnchorsPending {
    prepared()
        .commit(&mut FakeStore {
            commit: Some(Ok(disposition)),
            observation: None,
            observation_calls: 0,
        })
        .unwrap()
}

#[test]
fn initial_anchors_follow_database_commit_and_keep_control_identity() {
    let pending = committed(PrincipalBootstrapCommitDisposition::New);
    let candidate = pending.candidate().clone();
    let mut port = Anchors::new([
        Step::Advance(Ok(Advance::Advanced)),
        Step::Advance(Ok(Advance::Advanced)),
    ]);
    let complete = pending.advance_anchors(&mut port).unwrap();
    assert_eq!(complete.phase(), PrincipalBootstrapCustodyPhase::Complete);
    assert_eq!(
        complete.database_evidence(),
        PrincipalBootstrapCommitEvidence::New
    );
    assert_eq!(complete.candidate(), &candidate);
    assert_eq!(complete.anchors().len(), 2);
    assert!(matches!(
        complete.anchors()[0].0,
        AntiRollbackAnchor::PrincipalTrustMinimum(_)
    ));
    let AntiRollbackAnchor::ControlDomain(control) = &complete.anchors()[1].0 else {
        panic!("control anchor")
    };
    assert_eq!(control.control_store_id().as_str(), "principal-control-1");
    assert_ne!(
        control.control_store_id(),
        candidate.binding().trust_store_id()
    );
    assert!(port
        .calls
        .iter()
        .all(|(cas, transition)| *cas && transition.expected().is_none()));
    assert!(port.steps.is_empty());
}

#[test]
fn failure_on_later_domain_retains_prior_legs_without_repeating_them() {
    let mut pending = committed(PrincipalBootstrapCommitDisposition::New);
    let third = PreparedAntiRollbackAnchorTransition::initial(
        ControlDomainAntiRollbackAnchor::new(
            ControlStoreIdV1::parse("principal-control-1").unwrap(),
            ControlIncarnationIdV1::parse("control-incarnation-1").unwrap(),
            ControlCheckpointDomainIdV1::parse("second-domain").unwrap(),
            ControlDomainEpochV1::new(1).unwrap(),
            ControlDomainSequenceV1::new(1).unwrap(),
            coordinate("second-domain-checkpoint", '9'),
        )
        .into(),
    )
    .unwrap();
    pending.anchors.push((third.clone(), AnchorProgress::Ready));
    let mut port = Anchors::new([
        Step::Advance(Ok(Advance::Advanced)),
        Step::Advance(Ok(Advance::Advanced)),
        Step::Advance(Err(ErrorKind::Unavailable)),
        Step::Observe(Ok(Observed::Missing)),
        Step::Advance(Ok(Advance::Advanced)),
    ]);
    let failure = pending.advance_anchors(&mut port).unwrap_err();
    assert!(matches!(
        failure.reason(),
        PrincipalBootstrapAnchorFailureReason::Advance { .. }
    ));
    assert_eq!(
        failure.pending().phase(),
        PrincipalBootstrapCustodyPhase::DatabaseCommittedAnchorsPending
    );
    let complete = failure.into_pending().advance_anchors(&mut port).unwrap();
    assert_eq!(complete.anchors().len(), 3);
    assert_eq!(port.calls.len(), 5);
    for (_, called) in &port.calls[2..] {
        assert_eq!(called, &third);
    }
    assert!(port.steps.is_empty());
}

#[test]
fn uncertainty_is_observed_once_and_resumes_without_blind_retry() {
    let mut port = Anchors::new([
        Step::Advance(Err(ErrorKind::OutcomeUnknown)),
        Step::Observe(Err(ErrorKind::Unavailable)),
        Step::Observe(Ok(Observed::ExactSuccessor)),
        Step::Advance(Ok(Advance::Advanced)),
    ]);
    let failure = committed(PrincipalBootstrapCommitDisposition::New)
        .advance_anchors(&mut port)
        .unwrap_err();
    assert_eq!(port.calls.len(), 2);
    let complete = failure.into_pending().advance_anchors(&mut port).unwrap();
    assert_eq!(
        complete.anchors()[0].1,
        AnchorAdvanceEvidence::RecoveredExactAfterAmbiguousAdvance
    );
    assert_eq!(
        port.calls.iter().map(|(cas, _)| *cas).collect::<Vec<_>>(),
        [true, false, false, true]
    );
}

#[test]
fn replay_observes_each_anchor_and_only_initial_absence_permits_cas() {
    for disposition in [
        PrincipalBootstrapCommitDisposition::ExactReplay,
        PrincipalBootstrapCommitDisposition::New,
    ] {
        let mut pending = committed(disposition);
        if disposition == PrincipalBootstrapCommitDisposition::New {
            pending = prepared()
                .commit(&mut FakeStore {
                    commit: Some(Err(ControlStoreError::new(
                        ControlStoreErrorKind::OutcomeUnknown,
                    ))),
                    observation: Some(Ok(PrincipalBootstrapDatabaseObservation::Exact)),
                    observation_calls: 0,
                })
                .unwrap();
        }
        let mut port = Anchors::new([
            Step::Observe(Ok(Observed::ExactSuccessor)),
            Step::Observe(Ok(Observed::Missing)),
            Step::Advance(Ok(Advance::Advanced)),
        ]);
        let complete = pending.advance_anchors(&mut port).unwrap();
        assert_eq!(complete.anchors()[0].1, AnchorAdvanceEvidence::ExactReplay);
        assert_eq!(
            port.calls.iter().map(|(cas, _)| *cas).collect::<Vec<_>>(),
            [false, false, true]
        );
    }
}

#[test]
fn later_conflicting_and_unreadable_anchors_never_complete_or_advance() {
    let next = committed(PrincipalBootstrapCommitDisposition::New).anchors[0]
        .0
        .next()
        .clone();
    for observation in [
        Ok(Observed::Ahead {
            observed: next.clone(),
        }),
        Ok(Observed::Conflicting {
            observed: next.clone(),
        }),
        Err(ErrorKind::Unavailable),
    ] {
        let mut port = Anchors::new([Step::Observe(observation)]);
        assert!(committed(PrincipalBootstrapCommitDisposition::ExactReplay)
            .advance_anchors(&mut port)
            .is_err());
        assert_eq!(port.calls.len(), 1);
        assert!(!port.calls[0].0);
    }
}

#[test]
fn ambiguous_missing_does_not_retry_until_explicit_continuation() {
    let mut port = Anchors::new([
        Step::Advance(Err(ErrorKind::OutcomeUnknown)),
        Step::Observe(Ok(Observed::Missing)),
        Step::Observe(Ok(Observed::Missing)),
        Step::Advance(Ok(Advance::Advanced)),
        Step::Advance(Ok(Advance::ExactReplay)),
    ]);
    let failure = committed(PrincipalBootstrapCommitDisposition::New)
        .advance_anchors(&mut port)
        .unwrap_err();
    assert_eq!(port.calls.len(), 2);
    let completed = failure.into_pending().advance_anchors(&mut port).unwrap();
    assert_eq!(completed.anchors()[0].1, AnchorAdvanceEvidence::Advanced);
    assert_eq!(completed.anchors()[1].1, AnchorAdvanceEvidence::ExactReplay);
    assert!(port.steps.is_empty());
}
