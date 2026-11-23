//! Anchor sequencing after producer database custody, with controlled port outcomes.

use super::*;
use crate::resident::{AnchorAdvanceEvidence, AnchorAdvanceFailureReason};
use pale_ports::{
    AntiRollbackAnchor, AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError,
    AntiRollbackAnchorErrorKind, AntiRollbackAnchorKey, AntiRollbackAnchorObservation,
    AntiRollbackAnchorPort, ControlDomainAntiRollbackAnchor, PreparedAntiRollbackAnchorTransition,
    PrincipalTrustMinimumAnchor,
};
use std::collections::VecDeque;

#[derive(Clone, Copy)]
enum Step {
    Advance,
    Replay,
    Unavailable,
    Refuse(Observed),
    Unknown(Observed),
    UnknownUnreadable,
}

#[derive(Clone, Copy)]
enum Observed {
    Missing,
    Predecessor,
    Successor,
    Conflict,
    Later,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Call {
    Compare,
    Observe,
}

struct Anchors {
    steps: VecDeque<Step>,
    calls: Vec<PreparedAntiRollbackAnchorTransition>,
    observation: Option<Result<Observed, AntiRollbackAnchorErrorKind>>,
    observations: usize,
    continuation_observations: VecDeque<Result<Observed, AntiRollbackAnchorErrorKind>>,
    observed_transitions: Vec<PreparedAntiRollbackAnchorTransition>,
    order: Vec<Call>,
}

impl Anchors {
    fn new(steps: impl IntoIterator<Item = Step>) -> Self {
        Self {
            steps: steps.into_iter().collect(),
            calls: Vec::new(),
            observation: None,
            observations: 0,
            continuation_observations: VecDeque::new(),
            observed_transitions: Vec::new(),
            order: Vec::new(),
        }
    }

    fn with_observations(
        mut self,
        values: impl IntoIterator<Item = Result<Observed, AntiRollbackAnchorErrorKind>>,
    ) -> Self {
        self.continuation_observations = values.into_iter().collect();
        self
    }
}

fn observe(
    transition: &PreparedAntiRollbackAnchorTransition,
    observed: Observed,
) -> AntiRollbackAnchorObservation {
    let value = match observed {
        Observed::Missing => None,
        Observed::Predecessor => transition.expected().cloned(),
        Observed::Successor => Some(transition.next().clone()),
        Observed::Conflict | Observed::Later => {
            let later = matches!(observed, Observed::Later);
            let coordinate = coordinate("other-anchor", 'f');
            Some(match transition.next() {
                AntiRollbackAnchor::PrincipalTrustMinimum(next) => {
                    PrincipalTrustMinimumAnchor::new(
                        next.principal_id().clone(),
                        if later {
                            next.checkpoint_epoch()
                                .checked_successor()
                                .expect("later epoch")
                        } else {
                            next.checkpoint_epoch()
                        },
                        coordinate,
                    )
                    .into()
                }
                AntiRollbackAnchor::ControlDomain(next) => ControlDomainAntiRollbackAnchor::new(
                    next.control_store_id().clone(),
                    next.control_incarnation_id().clone(),
                    next.checkpoint_domain_id().clone(),
                    next.domain_epoch(),
                    if later {
                        next.domain_sequence()
                            .checked_successor()
                            .expect("later sequence")
                    } else {
                        next.domain_sequence()
                    },
                    coordinate,
                )
                .into(),
            })
        }
    };
    transition.classify_observed(value.as_ref())
}

impl AntiRollbackAnchorPort for Anchors {
    fn read(
        &mut self,
        _: &AntiRollbackAnchorKey,
    ) -> Result<Option<AntiRollbackAnchor>, AntiRollbackAnchorError> {
        panic!("completion does not perform unbound discovery")
    }

    fn compare_and_advance(
        &mut self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError> {
        assert!(
            self.observation.is_none(),
            "unknown outcome must be observed first"
        );
        self.calls.push(transition.clone());
        self.order.push(Call::Compare);
        match self.steps.pop_front().expect("no implicit retry") {
            Step::Advance => Ok(AntiRollbackAnchorAdvanceDisposition::Advanced),
            Step::Replay => Ok(AntiRollbackAnchorAdvanceDisposition::ExactReplay),
            Step::Unavailable => Err(AntiRollbackAnchorError::new(
                AntiRollbackAnchorErrorKind::Unavailable,
            )),
            Step::Refuse(value) => Ok(AntiRollbackAnchorAdvanceDisposition::NotAdvanced(observe(
                transition, value,
            ))),
            Step::Unknown(value) => {
                self.observation = Some(Ok(value));
                Err(AntiRollbackAnchorError::new(
                    AntiRollbackAnchorErrorKind::OutcomeUnknown,
                ))
            }
            Step::UnknownUnreadable => {
                self.observation = Some(Err(AntiRollbackAnchorErrorKind::Unavailable));
                Err(AntiRollbackAnchorError::new(
                    AntiRollbackAnchorErrorKind::OutcomeUnknown,
                ))
            }
        }
    }

    fn observe(
        &mut self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError> {
        if self.observation.is_some() {
            assert_eq!(self.calls.last(), Some(transition));
        }
        self.observations += 1;
        self.observed_transitions.push(transition.clone());
        self.order.push(Call::Observe);
        self.observation
            .take()
            .or_else(|| self.continuation_observations.pop_front())
            .expect("exactly the selected ambiguity or continuation observation")
            .map(|value| observe(transition, value))
            .map_err(AntiRollbackAnchorError::new)
    }
}

fn committed() -> DatabaseCommittedPrincipalProducerIncarnationAnchorsPending {
    prepared()
        .commit(&mut FakeStore {
            commit: Some(Ok(PrincipalProducerIncarnationCommitDisposition::Committed)),
            observation: None,
            observation_calls: 0,
        })
        .expect("database leg precedes anchors")
}

fn anchor_coordinate(anchor: &AntiRollbackAnchor) -> &ControlRecordCoordinate {
    match anchor {
        AntiRollbackAnchor::PrincipalTrustMinimum(value) => value.coordinate(),
        AntiRollbackAnchor::ControlDomain(value) => value.coordinate(),
    }
}

#[test]
fn both_anchors_derive_only_from_the_retained_candidate() {
    for (step, evidence) in [
        (Step::Advance, AnchorAdvanceEvidence::Advanced),
        (Step::Replay, AnchorAdvanceEvidence::ExactReplay),
        (
            Step::Unknown(Observed::Successor),
            AnchorAdvanceEvidence::RecoveredExactAfterAmbiguousAdvance,
        ),
    ] {
        let pending = committed();
        let candidate = pending.candidate().clone();
        let mut anchors = Anchors::new([step, step]);
        let completed = pending.advance_anchors(&mut anchors).expect("both anchors");
        assert_eq!(completed.candidate(), &candidate);
        assert_eq!(completed.control_store(), &producer_control_store());
        assert_ne!(
            completed.control_store().store_id(),
            candidate.binding().trust_store_id()
        );
        assert_eq!(
            completed.phase(),
            PrincipalProducerIncarnationCustodyPhase::Completed
        );
        assert_eq!(
            completed.database_evidence(),
            PrincipalProducerIncarnationCommitEvidence::New
        );
        assert_eq!(completed.trust_anchor_evidence(), evidence);
        assert_eq!(completed.control_anchor_evidence(), evidence);
        assert_eq!(anchors.calls.len(), 2);
        let trust = &anchors.calls[0];
        assert!(matches!(
            trust.key(),
            AntiRollbackAnchorKey::PrincipalTrustMinimum(_)
        ));
        assert_eq!(
            anchor_coordinate(trust.expected().expect("trust predecessor")),
            candidate.expected_trust().coordinate()
        );
        assert_eq!(
            anchor_coordinate(trust.next()),
            candidate.next_trust().coordinate()
        );
        for (anchor, head) in [
            (
                trust.expected().expect("predecessor"),
                candidate.expected_trust(),
            ),
            (trust.next(), candidate.next_trust()),
        ] {
            let AntiRollbackAnchor::PrincipalTrustMinimum(value) = anchor else {
                panic!("trust value")
            };
            assert_eq!(value.principal_id(), head.principal_id());
            assert_eq!(value.checkpoint_epoch(), head.checkpoint_epoch());
        }
        let control = &anchors.calls[1];
        assert!(matches!(
            control.key(),
            AntiRollbackAnchorKey::ControlDomain { .. }
        ));
        assert_eq!(
            anchor_coordinate(control.expected().expect("control predecessor")),
            candidate.expected_checkpoint().coordinate()
        );
        assert_eq!(
            anchor_coordinate(control.next()),
            candidate.next_checkpoint().coordinate()
        );
        for (anchor, head) in [
            (
                control.expected().expect("predecessor"),
                candidate.expected_checkpoint(),
            ),
            (control.next(), candidate.next_checkpoint()),
        ] {
            let AntiRollbackAnchor::ControlDomain(value) = anchor else {
                panic!("control value")
            };
            assert_eq!(
                value.control_store_id(),
                producer_control_store().store_id()
            );
            assert_eq!(
                value.control_incarnation_id(),
                producer_control_store().incarnation_id()
            );
            assert_eq!(value.checkpoint_domain_id(), head.checkpoint_domain_id());
            assert_eq!(value.domain_epoch(), head.domain_epoch());
            assert_eq!(value.domain_sequence(), head.domain_sequence());
        }
        assert_eq!(
            anchors.observations,
            if matches!(step, Step::Unknown(_)) {
                2
            } else {
                0
            }
        );
        assert!(anchors.steps.is_empty());
    }
}

#[test]
fn every_anchor_failure_preserves_only_established_progress() {
    let mut expected = Anchors::new([Step::Advance, Step::Advance]);
    committed()
        .advance_anchors(&mut expected)
        .expect("reference transitions");
    for step in [
        Step::Unavailable,
        Step::Refuse(Observed::Missing),
        Step::Refuse(Observed::Predecessor),
        Step::Refuse(Observed::Conflict),
        Step::Refuse(Observed::Later),
        Step::Unknown(Observed::Missing),
        Step::Unknown(Observed::Predecessor),
        Step::Unknown(Observed::Conflict),
        Step::Unknown(Observed::Later),
        Step::UnknownUnreadable,
    ] {
        for position in 0..2 {
            let pending = committed();
            let candidate = pending.candidate().clone();
            let mut steps = vec![Step::Advance; position];
            steps.push(step);
            let mut anchors = Anchors::new(steps);
            let failure = pending
                .advance_anchors(&mut anchors)
                .expect_err("must remain pending");
            assert_eq!(anchors.calls.len(), position + 1);
            assert_eq!(
                anchors.observations,
                usize::from(matches!(step, Step::Unknown(_) | Step::UnknownUnreadable))
            );
            assert!(anchors.observation.is_none());
            let reason = match failure.reason() {
                PrincipalProducerIncarnationAnchorFailureReason::Advance { anchor, reason } => {
                    assert_eq!(anchor, anchors.calls[position].key());
                    reason
                }
                other => panic!("unexpected transition refusal: {other}"),
            };
            match (step, reason) {
                (Step::Unavailable, AnchorAdvanceFailureReason::Port(error)) => {
                    assert_eq!(error.kind(), AntiRollbackAnchorErrorKind::Unavailable)
                }
                (Step::Refuse(value), AnchorAdvanceFailureReason::NotAdvanced(observation)) => {
                    assert_eq!(**observation, observe(&anchors.calls[position], value))
                }
                (
                    Step::Unknown(value),
                    AnchorAdvanceFailureReason::AmbiguousNotExact {
                        uncertain,
                        observation,
                    },
                ) => {
                    assert_eq!(
                        uncertain.kind(),
                        AntiRollbackAnchorErrorKind::OutcomeUnknown
                    );
                    assert_eq!(**observation, observe(&anchors.calls[position], value));
                }
                (
                    Step::UnknownUnreadable,
                    AnchorAdvanceFailureReason::AmbiguousObservationFailed { uncertain, observe },
                ) => {
                    assert_eq!(
                        uncertain.kind(),
                        AntiRollbackAnchorErrorKind::OutcomeUnknown
                    );
                    assert_eq!(observe.kind(), AntiRollbackAnchorErrorKind::Unavailable);
                }
                _ => panic!("failure lost its original evidence"),
            }
            assert_eq!(failure.pending().candidate(), &candidate);
            assert_eq!(
                failure.pending().trust_anchor_evidence(),
                if position == 1 {
                    Some(AnchorAdvanceEvidence::Advanced)
                } else {
                    None
                }
            );
            if let Step::Refuse(
                value @ (Observed::Missing | Observed::Conflict | Observed::Later),
            )
            | Step::Unknown(
                value @ (Observed::Missing | Observed::Conflict | Observed::Later),
            ) = step
            {
                let mut resumed = Anchors::new([]).with_observations([Ok(value)]);
                let again = failure
                    .into_pending()
                    .advance_anchors(&mut resumed)
                    .expect_err("same conflict cannot become a replay");
                assert!(resumed.calls.is_empty());
                assert_eq!(resumed.order, [Call::Observe]);
                assert_eq!(
                    resumed.observed_transitions.as_slice(),
                    std::slice::from_ref(&expected.calls[position])
                );
                assert_eq!(again.pending().candidate(), &candidate);
                assert_eq!(
                    again.pending().trust_anchor_evidence(),
                    if position == 1 {
                        Some(AnchorAdvanceEvidence::Advanced)
                    } else {
                        None
                    }
                );
                continue;
            }
            let mut resumed = Anchors::new(vec![Step::Advance; 2 - position])
                .with_observations([Ok(Observed::Predecessor)]);
            let completed = failure
                .into_pending()
                .advance_anchors(&mut resumed)
                .expect("explicit resume");
            assert_eq!(resumed.calls, expected.calls[position..]);
            assert_eq!(resumed.order[0], Call::Observe);
            assert_eq!(
                resumed.observed_transitions.as_slice(),
                std::slice::from_ref(&expected.calls[position])
            );
            assert_eq!(completed.candidate(), &candidate);
            assert_eq!(
                completed.trust_anchor_evidence(),
                AnchorAdvanceEvidence::Advanced
            );
            assert_eq!(
                completed.control_anchor_evidence(),
                AnchorAdvanceEvidence::Advanced
            );
        }
    }
}

#[test]
fn repeated_uncertainty_observes_before_retry_and_preserves_its_evidence_kind() {
    for position in 0..2 {
        let mut steps = vec![Step::Advance; position];
        steps.push(Step::UnknownUnreadable);
        let mut first = Anchors::new(steps);
        let failure = committed()
            .advance_anchors(&mut first)
            .expect_err("unresolved advance");
        let transition = first.calls[position].clone();
        let candidate = failure.pending().candidate().clone();

        let mut unreadable =
            Anchors::new([]).with_observations([Err(AntiRollbackAnchorErrorKind::Unavailable)]);
        let failure = failure
            .into_pending()
            .advance_anchors(&mut unreadable)
            .expect_err("unreadable continuation cannot compare");
        assert_eq!(unreadable.order, [Call::Observe]);
        assert_eq!(
            unreadable.observed_transitions.as_slice(),
            std::slice::from_ref(&transition)
        );
        assert!(
            matches!(failure.reason(), PrincipalProducerIncarnationAnchorFailureReason::Observation { error, .. } if error.kind() == AntiRollbackAnchorErrorKind::Unavailable)
        );

        let mut retry =
            Anchors::new([Step::UnknownUnreadable]).with_observations([Ok(Observed::Predecessor)]);
        let failure = failure
            .into_pending()
            .advance_anchors(&mut retry)
            .expect_err("second advance also uncertain");
        assert_eq!(retry.order, [Call::Observe, Call::Compare, Call::Observe]);
        assert_eq!(retry.calls.as_slice(), std::slice::from_ref(&transition));
        assert_eq!(
            retry.observed_transitions,
            [transition.clone(), transition.clone()]
        );

        let mut recovered = Anchors::new(vec![Step::Advance; 1 - position])
            .with_observations([Ok(Observed::Successor)]);
        let completed = failure
            .into_pending()
            .advance_anchors(&mut recovered)
            .expect("exact successor resolves uncertainty without another comparison");
        assert_eq!(recovered.observed_transitions, [transition]);
        assert_eq!(recovered.order[0], Call::Observe);
        assert_eq!(recovered.calls.len(), 1 - position);
        assert_eq!(completed.candidate(), &candidate);
        assert_eq!(
            completed.trust_anchor_evidence(),
            if position == 0 {
                AnchorAdvanceEvidence::RecoveredExactAfterAmbiguousAdvance
            } else {
                AnchorAdvanceEvidence::Advanced
            }
        );
        assert_eq!(
            completed.control_anchor_evidence(),
            if position == 1 {
                AnchorAdvanceEvidence::RecoveredExactAfterAmbiguousAdvance
            } else {
                AnchorAdvanceEvidence::Advanced
            }
        );
    }
}
