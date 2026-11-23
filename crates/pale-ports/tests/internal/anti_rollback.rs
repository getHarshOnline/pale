//! Contract tests for independently retained anti-rollback anchors.

use std::error::Error as _;

use super::*;
use crate::ControlRecordDigest;
use pale_core::records::{
    ControlCheckpointDomainIdV1, ControlDomainEpochV1, ControlDomainSequenceV1,
    ControlIncarnationIdV1, ControlStoreIdV1, PrincipalTrustCheckpointEpochV1,
    ResidentPrincipalIdV1, ResidentRecordRefV1,
};

fn coordinate(reference: &str, byte: char) -> ControlRecordCoordinate {
    ControlRecordCoordinate::new(
        ResidentRecordRefV1::parse(reference).expect("record reference"),
        ControlRecordDigest::parse(std::iter::repeat_n(byte, 64).collect::<String>())
            .expect("record digest"),
    )
}

fn principal(principal: &str, epoch: u64, reference: &str, byte: char) -> AntiRollbackAnchor {
    PrincipalTrustMinimumAnchor::new(
        ResidentPrincipalIdV1::parse(principal).expect("principal"),
        PrincipalTrustCheckpointEpochV1::new(epoch).expect("trust epoch"),
        coordinate(reference, byte),
    )
    .into()
}

fn domain(
    store: &str,
    incarnation: &str,
    domain: &str,
    epoch: u64,
    sequence: u64,
    reference: &str,
    byte: char,
) -> AntiRollbackAnchor {
    ControlDomainAntiRollbackAnchor::new(
        ControlStoreIdV1::parse(store).expect("store"),
        ControlIncarnationIdV1::parse(incarnation).expect("incarnation"),
        ControlCheckpointDomainIdV1::parse(domain).expect("domain"),
        ControlDomainEpochV1::new(epoch).expect("domain epoch"),
        ControlDomainSequenceV1::new(sequence).expect("domain sequence"),
        coordinate(reference, byte),
    )
    .into()
}

#[test]
fn initial_and_successor_transitions_preserve_each_typed_lineage() {
    let principal_1 = principal("principal-1", 1, "trust-1", '1');
    assert!(PreparedAntiRollbackAnchorTransition::initial(principal_1.clone()).is_ok());
    assert_eq!(
        PreparedAntiRollbackAnchorTransition::initial(principal("principal-1", 2, "trust-2", '2')),
        Err(AntiRollbackAnchorTransitionError::InitialPosition)
    );
    assert!(PreparedAntiRollbackAnchorTransition::successor(
        principal_1.clone(),
        principal("principal-1", 2, "trust-2", '2')
    )
    .is_ok());
    assert_eq!(
        PreparedAntiRollbackAnchorTransition::successor(
            principal_1.clone(),
            principal("principal-1", 3, "trust-3", '3')
        ),
        Err(AntiRollbackAnchorTransitionError::Successor)
    );
    assert_eq!(
        PreparedAntiRollbackAnchorTransition::successor(
            principal_1,
            principal("principal-2", 2, "trust-2", '2')
        ),
        Err(AntiRollbackAnchorTransitionError::Key)
    );

    let domain_1 = domain("store-1", "inc-1", "apply:a", 4, 1, "domain-1", '4');
    assert!(PreparedAntiRollbackAnchorTransition::initial(domain_1.clone()).is_ok());
    assert!(PreparedAntiRollbackAnchorTransition::successor(
        domain_1.clone(),
        domain("store-1", "inc-1", "apply:a", 4, 2, "domain-2", '5')
    )
    .is_ok());
    assert_eq!(
        PreparedAntiRollbackAnchorTransition::successor(
            domain_1,
            domain("store-1", "inc-1", "apply:a", 5, 2, "domain-2", '5')
        ),
        Err(AntiRollbackAnchorTransitionError::Successor)
    );
}

#[test]
fn classification_is_exact_and_never_uses_position_as_replay_proof() {
    let expected = principal("principal-1", 2, "trust-2", '2');
    let next = principal("principal-1", 3, "trust-3", '3');
    let transition =
        PreparedAntiRollbackAnchorTransition::successor(expected.clone(), next.clone())
            .expect("transition");

    assert_eq!(
        transition.classify_observed(None),
        AntiRollbackAnchorObservation::Missing
    );
    assert_eq!(
        transition.classify_observed(Some(&expected)),
        AntiRollbackAnchorObservation::ExactPredecessor
    );
    assert_eq!(
        transition.classify_observed(Some(&next)),
        AntiRollbackAnchorObservation::ExactSuccessor
    );

    let behind = principal("principal-1", 1, "trust-1", '1');
    assert_eq!(
        transition.classify_observed(Some(&behind)),
        AntiRollbackAnchorObservation::Behind {
            observed: behind.clone()
        }
    );
    let ahead = principal("principal-1", 4, "trust-4", '4');
    assert_eq!(
        transition.classify_observed(Some(&ahead)),
        AntiRollbackAnchorObservation::Ahead {
            observed: ahead.clone()
        }
    );
    let same_position_different_bytes = principal("principal-1", 3, "other-trust-3", '9');
    assert_eq!(
        transition.classify_observed(Some(&same_position_different_bytes)),
        AntiRollbackAnchorObservation::Conflicting {
            observed: same_position_different_bytes
        }
    );
    let other_key = principal("principal-2", 3, "trust-3", '3');
    assert_eq!(
        transition.classify_observed(Some(&other_key)),
        AntiRollbackAnchorObservation::Conflicting {
            observed: other_key
        }
    );
}

#[test]
fn control_observation_rejects_cross_epoch_ordering_and_keys_sort_deterministically() {
    let expected = domain("store-1", "inc-1", "apply:a", 2, 3, "domain-3", '3');
    let next = domain("store-1", "inc-1", "apply:a", 2, 4, "domain-4", '4');
    let transition =
        PreparedAntiRollbackAnchorTransition::successor(expected, next).expect("transition");
    let other_epoch = domain("store-1", "inc-1", "apply:a", 3, 5, "domain-5", '5');
    assert_eq!(
        transition.classify_observed(Some(&other_epoch)),
        AntiRollbackAnchorObservation::Conflicting {
            observed: other_epoch
        }
    );

    let mut keys = [
        domain("store-2", "inc-1", "apply:a", 1, 1, "d-2", '2').key(),
        principal("principal-1", 1, "p-1", '1').key(),
        domain("store-1", "inc-1", "apply:z", 1, 1, "d-z", '3').key(),
        domain("store-1", "inc-1", "apply:a", 1, 1, "d-a", '4').key(),
    ];
    keys.sort();
    assert!(matches!(
        &keys[0],
        AntiRollbackAnchorKey::PrincipalTrustMinimum(principal)
            if principal.as_str() == "principal-1"
    ));
    assert!(matches!(
        &keys[1],
        AntiRollbackAnchorKey::ControlDomain { checkpoint_domain_id, .. }
            if checkpoint_domain_id.as_str() == "apply:a"
    ));
    assert!(matches!(
        &keys[2],
        AntiRollbackAnchorKey::ControlDomain { checkpoint_domain_id, .. }
            if checkpoint_domain_id.as_str() == "apply:z"
    ));
}

#[derive(Default)]
struct FakeAnchor {
    value: Option<AntiRollbackAnchor>,
}

impl AntiRollbackAnchorPort for FakeAnchor {
    fn read(
        &mut self,
        key: &crate::AntiRollbackAnchorKey,
    ) -> Result<Option<crate::AntiRollbackAnchor>, AntiRollbackAnchorError> {
        Ok(self
            .value
            .as_ref()
            .filter(|value| &value.key() == key)
            .cloned())
    }

    fn compare_and_advance(
        &mut self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError> {
        let observation = transition.classify_observed(self.value.as_ref());
        match (&observation, transition.expected()) {
            (AntiRollbackAnchorObservation::Missing, None)
            | (AntiRollbackAnchorObservation::ExactPredecessor, _) => {
                self.value = Some(transition.next().clone());
                Ok(AntiRollbackAnchorAdvanceDisposition::Advanced)
            }
            (AntiRollbackAnchorObservation::ExactSuccessor, _) => {
                Ok(AntiRollbackAnchorAdvanceDisposition::ExactReplay)
            }
            _ => Ok(AntiRollbackAnchorAdvanceDisposition::NotAdvanced(
                observation,
            )),
        }
    }

    fn observe(
        &mut self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError> {
        Ok(transition.classify_observed(self.value.as_ref()))
    }
}

#[test]
fn object_safe_port_advances_initial_and_replays_exact_successor() {
    fn as_dyn(value: &mut FakeAnchor) -> &mut dyn AntiRollbackAnchorPort {
        value
    }

    let transition =
        PreparedAntiRollbackAnchorTransition::initial(principal("principal-1", 1, "trust-1", '1'))
            .expect("initial transition");
    let mut adapter = FakeAnchor::default();
    let port = as_dyn(&mut adapter);
    assert_eq!(
        port.compare_and_advance(&transition).expect("advance"),
        AntiRollbackAnchorAdvanceDisposition::Advanced
    );
    assert_eq!(
        port.compare_and_advance(&transition).expect("replay"),
        AntiRollbackAnchorAdvanceDisposition::ExactReplay
    );
}

#[test]
fn read_anchor_has_no_transition_or_initial_position_prerequisite() {
    let retained = principal("principal-1", 7, "trust-7", '7');
    let mut port = FakeAnchor {
        value: Some(retained.clone()),
    };
    assert_eq!(
        AntiRollbackAnchorPort::read(&mut port, &retained.key()).unwrap(),
        Some(retained.clone())
    );
    assert_eq!(port.value, Some(retained));
    assert_eq!(
        AntiRollbackAnchorPort::read(
            &mut port,
            &AntiRollbackAnchorKey::PrincipalTrustMinimum(
                ResidentPrincipalIdV1::parse("another").unwrap()
            )
        )
        .unwrap(),
        None
    );
}

#[test]
fn external_failures_keep_recovery_class_and_native_source() {
    let unavailable = AntiRollbackAnchorError::new(AntiRollbackAnchorErrorKind::Unavailable);
    assert_eq!(unavailable.kind(), AntiRollbackAnchorErrorKind::Unavailable);
    assert!(unavailable.source().is_none());

    let uncertain = AntiRollbackAnchorError::with_source(
        AntiRollbackAnchorErrorKind::OutcomeUnknown,
        std::io::Error::from(std::io::ErrorKind::Interrupted),
    );
    assert_eq!(
        uncertain.kind(),
        AntiRollbackAnchorErrorKind::OutcomeUnknown
    );
    assert!(uncertain.source().is_some());
}
