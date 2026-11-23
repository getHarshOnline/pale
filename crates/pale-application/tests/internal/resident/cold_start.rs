//! Behavioral laws for whole-resident cold-start classification.

use super::*;

const TRUST_STATES: [ColdStartTrustState; 6] = [
    ColdStartTrustState::NoTrustHeads,
    ColdStartTrustState::Admitted,
    ColdStartTrustState::FreshIncarnationRecoveryAdmitted(ColdStartTrustLoss::Missing),
    ColdStartTrustState::FreshIncarnationRecoveryAdmitted(ColdStartTrustLoss::Corrupt),
    ColdStartTrustState::FreshIncarnationRecoveryAdmitted(ColdStartTrustLoss::RolledBack),
    ColdStartTrustState::Unavailable,
];

const ANCHOR_STATES: [ColdStartAnchorState; 5] = [
    ColdStartAnchorState::NotApplicable,
    ColdStartAnchorState::Agrees,
    ColdStartAnchorState::ReceiptNamedAdvancesPending,
    ColdStartAnchorState::TrustRecoveryAdvancesPending,
    ColdStartAnchorState::RecoveryFrontierAdmitted,
];

const CONTINUATIONS: [ColdStartContinuation; 4] = [
    ColdStartContinuation::None,
    ColdStartContinuation::InactiveDeploymentGenesis,
    ColdStartContinuation::InactiveRecoveredControlStore,
    ColdStartContinuation::Conflicting,
];

const INTEGRITY_FAILURES: [ColdStartIntegrityFailure; 8] = [
    ColdStartIntegrityFailure::RollbackDetected,
    ColdStartIntegrityFailure::AnchorAheadOfStore,
    ColdStartIntegrityFailure::CheckpointHashConflict,
    ColdStartIntegrityFailure::BrokenAccumulator,
    ColdStartIntegrityFailure::UnknownCheckpointProfile,
    ColdStartIntegrityFailure::UnknownSigner,
    ColdStartIntegrityFailure::MissingHardDomain,
    ColdStartIntegrityFailure::PartialEvidence,
];

const SAFE_CASES: [(ColdStartFacts, ColdStartDisposition); 9] = [
    (
        ColdStartFacts::new(
            ColdStartTrustState::NoTrustHeads,
            ColdStartAnchorState::NotApplicable,
            ColdStartContinuation::None,
        ),
        ColdStartDisposition::OfflineBootstrapEligible,
    ),
    (
        ColdStartFacts::new(
            ColdStartTrustState::Admitted,
            ColdStartAnchorState::ReceiptNamedAdvancesPending,
            ColdStartContinuation::None,
        ),
        ColdStartDisposition::CompleteReceiptNamedAnchorAdvancesOnly,
    ),
    (
        ColdStartFacts::new(
            ColdStartTrustState::Admitted,
            ColdStartAnchorState::Agrees,
            ColdStartContinuation::None,
        ),
        ColdStartDisposition::EligibleForLaterNormalAssembly,
    ),
    (
        ColdStartFacts::new(
            ColdStartTrustState::Admitted,
            ColdStartAnchorState::Agrees,
            ColdStartContinuation::InactiveDeploymentGenesis,
        ),
        ColdStartDisposition::ResumeInactiveDeploymentGenesisOnly,
    ),
    (
        ColdStartFacts::new(
            ColdStartTrustState::Admitted,
            ColdStartAnchorState::Agrees,
            ColdStartContinuation::InactiveRecoveredControlStore,
        ),
        ColdStartDisposition::ResumeInactiveRecoveredControlStoreOnly,
    ),
    (
        ColdStartFacts::new(
            ColdStartTrustState::Admitted,
            ColdStartAnchorState::TrustRecoveryAdvancesPending,
            ColdStartContinuation::None,
        ),
        ColdStartDisposition::ResumeTrustRecoveryAnchorAdvancesOnly,
    ),
    (
        ColdStartFacts::new(
            ColdStartTrustState::FreshIncarnationRecoveryAdmitted(ColdStartTrustLoss::Missing),
            ColdStartAnchorState::RecoveryFrontierAdmitted,
            ColdStartContinuation::None,
        ),
        ColdStartDisposition::RecoverTrustIntoFreshIncarnationsOffline,
    ),
    (
        ColdStartFacts::new(
            ColdStartTrustState::FreshIncarnationRecoveryAdmitted(ColdStartTrustLoss::Corrupt),
            ColdStartAnchorState::RecoveryFrontierAdmitted,
            ColdStartContinuation::None,
        ),
        ColdStartDisposition::RecoverTrustIntoFreshIncarnationsOffline,
    ),
    (
        ColdStartFacts::new(
            ColdStartTrustState::FreshIncarnationRecoveryAdmitted(ColdStartTrustLoss::RolledBack),
            ColdStartAnchorState::RecoveryFrontierAdmitted,
            ColdStartContinuation::None,
        ),
        ColdStartDisposition::RecoverTrustIntoFreshIncarnationsOffline,
    ),
];

#[test]
fn decision_table_admits_only_the_ratified_safe_states() {
    for trust in TRUST_STATES {
        for anchors in ANCHOR_STATES {
            for continuation in CONTINUATIONS {
                let actual = classify_cold_start(ColdStartFacts::new(trust, anchors, continuation));
                assert_eq!(
                    actual,
                    expected_disposition(trust, anchors, continuation),
                    "unexpected decision for {trust:?}, {anchors:?}, {continuation:?}"
                );
            }
        }
    }
}

#[test]
fn unsafe_anchor_evidence_fails_or_conflicts_for_every_fact() {
    for reason in INTEGRITY_FAILURES {
        for trust in TRUST_STATES {
            for continuation in CONTINUATIONS {
                let failure = if matches!(
                    trust,
                    ColdStartTrustState::FreshIncarnationRecoveryAdmitted(_)
                ) {
                    ColdStartFailure::ConflictingEvidence
                } else {
                    ColdStartFailure::Integrity(reason)
                };
                assert_eq!(
                    classify_cold_start(ColdStartFacts::new(
                        trust,
                        ColdStartAnchorState::Unsafe(reason),
                        continuation,
                    )),
                    ColdStartDisposition::FailClosedIntoOfflineOwnerRecovery(failure),
                );
            }
        }
    }
}

#[test]
fn empty_trust_with_any_other_work_fails_closed() {
    let hostile = [
        (ColdStartAnchorState::Agrees, ColdStartContinuation::None),
        (
            ColdStartAnchorState::ReceiptNamedAdvancesPending,
            ColdStartContinuation::None,
        ),
        (
            ColdStartAnchorState::NotApplicable,
            ColdStartContinuation::InactiveDeploymentGenesis,
        ),
        (
            ColdStartAnchorState::NotApplicable,
            ColdStartContinuation::InactiveRecoveredControlStore,
        ),
    ];

    for (anchors, continuation) in hostile {
        assert_eq!(
            classify_cold_start(ColdStartFacts::new(
                ColdStartTrustState::NoTrustHeads,
                anchors,
                continuation,
            )),
            ColdStartDisposition::FailClosedIntoOfflineOwnerRecovery(
                ColdStartFailure::ConflictingEvidence,
            ),
        );
    }
}

#[test]
fn anchor_completion_never_resumes_unrelated_work() {
    for anchors in [
        ColdStartAnchorState::ReceiptNamedAdvancesPending,
        ColdStartAnchorState::TrustRecoveryAdvancesPending,
    ] {
        for continuation in [
            ColdStartContinuation::InactiveDeploymentGenesis,
            ColdStartContinuation::InactiveRecoveredControlStore,
            ColdStartContinuation::Conflicting,
        ] {
            assert_eq!(
                classify_cold_start(ColdStartFacts::new(
                    ColdStartTrustState::Admitted,
                    anchors,
                    continuation,
                )),
                ColdStartDisposition::FailClosedIntoOfflineOwnerRecovery(
                    ColdStartFailure::ConflictingEvidence,
                ),
            );
        }
    }
}

#[test]
fn fresh_incarnation_recovery_requires_its_exact_frontier_and_no_continuation() {
    for loss in [
        ColdStartTrustLoss::Missing,
        ColdStartTrustLoss::Corrupt,
        ColdStartTrustLoss::RolledBack,
    ] {
        let trust = ColdStartTrustState::FreshIncarnationRecoveryAdmitted(loss);
        assert_eq!(
            classify_cold_start(ColdStartFacts::new(
                trust,
                ColdStartAnchorState::RecoveryFrontierAdmitted,
                ColdStartContinuation::None,
            )),
            ColdStartDisposition::RecoverTrustIntoFreshIncarnationsOffline,
        );

        for anchors in [
            ColdStartAnchorState::NotApplicable,
            ColdStartAnchorState::Agrees,
            ColdStartAnchorState::ReceiptNamedAdvancesPending,
            ColdStartAnchorState::TrustRecoveryAdvancesPending,
        ] {
            for continuation in CONTINUATIONS {
                assert_eq!(
                    classify_cold_start(ColdStartFacts::new(trust, anchors, continuation)),
                    ColdStartDisposition::FailClosedIntoOfflineOwnerRecovery(
                        ColdStartFailure::ConflictingEvidence,
                    ),
                );
            }
        }

        for continuation in [
            ColdStartContinuation::InactiveDeploymentGenesis,
            ColdStartContinuation::InactiveRecoveredControlStore,
            ColdStartContinuation::Conflicting,
        ] {
            assert_eq!(
                classify_cold_start(ColdStartFacts::new(
                    trust,
                    ColdStartAnchorState::RecoveryFrontierAdmitted,
                    continuation,
                )),
                ColdStartDisposition::FailClosedIntoOfflineOwnerRecovery(
                    ColdStartFailure::ConflictingEvidence,
                ),
            );
        }
    }
}

#[test]
fn rollback_without_recovery_authority_fails_as_unsafe() {
    assert_eq!(
        classify_cold_start(ColdStartFacts::new(
            ColdStartTrustState::Unavailable,
            ColdStartAnchorState::Unsafe(ColdStartIntegrityFailure::RollbackDetected),
            ColdStartContinuation::None,
        )),
        ColdStartDisposition::FailClosedIntoOfflineOwnerRecovery(ColdStartFailure::Integrity(
            ColdStartIntegrityFailure::RollbackDetected,
        )),
    );
}

#[test]
fn unavailable_trust_without_recovery_authority_fails_closed() {
    assert_eq!(
        classify_cold_start(ColdStartFacts::new(
            ColdStartTrustState::Unavailable,
            ColdStartAnchorState::NotApplicable,
            ColdStartContinuation::None,
        )),
        ColdStartDisposition::FailClosedIntoOfflineOwnerRecovery(
            ColdStartFailure::TrustRecoveryUnavailable,
        ),
    );
}

fn expected_disposition(
    trust: ColdStartTrustState,
    anchors: ColdStartAnchorState,
    continuation: ColdStartContinuation,
) -> ColdStartDisposition {
    let facts = ColdStartFacts::new(trust, anchors, continuation);
    if let Some((_, disposition)) = SAFE_CASES.iter().find(|(safe, _)| *safe == facts) {
        return *disposition;
    }
    let reason = if matches!(
        trust,
        ColdStartTrustState::FreshIncarnationRecoveryAdmitted(_)
    ) {
        ColdStartFailure::ConflictingEvidence
    } else if matches!(trust, ColdStartTrustState::Unavailable)
        && !matches!(continuation, ColdStartContinuation::Conflicting)
    {
        ColdStartFailure::TrustRecoveryUnavailable
    } else {
        ColdStartFailure::ConflictingEvidence
    };
    ColdStartDisposition::FailClosedIntoOfflineOwnerRecovery(reason)
}
