//! Choose the only safe next step from admitted cold-start facts.
//!
//! The caller has already admitted record structure, signatures, checkpoint
//! profiles, accumulator closure, store bindings, and independent-anchor
//! comparisons. This module only classifies those facts. It does not open a
//! store, advance an anchor, construct a process host or attachment, select a
//! signer or transport, establish liveness, or grant effect authority.

/// Trust-store condition established before whole-resident classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::resident) enum ColdStartTrustState {
    /// No trust head or partial trust material exists.
    NoTrustHeads,
    /// The trust lineage has been admitted through its selected current head.
    Admitted,
    /// Fresh-incarnation recovery has admitted the named trust loss.
    FreshIncarnationRecoveryAdmitted(ColdStartTrustLoss),
    /// Trust custody is unusable and lacks admitted fresh-incarnation recovery.
    Unavailable,
}

/// Trust loss admitted for fresh-incarnation recovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::resident) enum ColdStartTrustLoss {
    /// The required trust custody is absent.
    Missing,
    /// The required trust custody is corrupt.
    Corrupt,
    /// The supplied trust custody is behind an accepted frontier.
    RolledBack,
}

/// Relationship between admitted store heads and independent anchors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::resident) enum ColdStartAnchorState {
    /// No anchor comparison applies because no trust frontier exists.
    NotApplicable,
    /// Every required hard frontier agrees with its independent anchor.
    Agrees,
    /// The store leads only by advances named by one committed receipt.
    ReceiptNamedAdvancesPending,
    /// A trust-recovery receipt names the only pending anchor advances.
    TrustRecoveryAdvancesPending,
    /// The authorized fresh-incarnation recovery frontier has been admitted.
    RecoveryFrontierAdmitted,
    /// The admitted evidence requires offline owner recovery.
    Unsafe(ColdStartIntegrityFailure),
}

/// Inactive work that cold start may resume without assembling the resident.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::resident) enum ColdStartContinuation {
    /// No incomplete offline transaction was admitted.
    None,
    /// One admitted deployment has incomplete control-store genesis.
    InactiveDeploymentGenesis,
    /// One recovered control store awaits its trust-store receipt.
    InactiveRecoveredControlStore,
    /// More than one continuation claims exclusive recovery custody.
    Conflicting,
}

/// Admitted facts consumed by whole-resident cold-start classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::resident) struct ColdStartFacts {
    trust: ColdStartTrustState,
    anchors: ColdStartAnchorState,
    continuation: ColdStartContinuation,
}

impl ColdStartFacts {
    /// Combine the independently admitted trust, anchor, and continuation facts.
    #[must_use]
    pub(in crate::resident) const fn new(
        trust: ColdStartTrustState,
        anchors: ColdStartAnchorState,
        continuation: ColdStartContinuation,
    ) -> Self {
        Self {
            trust,
            anchors,
            continuation,
        }
    }
}

/// Anchor or hard-frontier evidence that makes startup unsafe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::resident) enum ColdStartIntegrityFailure {
    /// Store custody is rolled back from an accepted frontier.
    RollbackDetected,
    /// An independent anchor is ahead of the supplied store.
    AnchorAheadOfStore,
    /// One checkpoint sequence is bound to different hashes.
    CheckpointHashConflict,
    /// A hard-domain accumulator does not reproduce its checkpoint.
    BrokenAccumulator,
    /// The checkpoint profile is not admitted by this resident.
    UnknownCheckpointProfile,
    /// A required checkpoint signer is not admitted.
    UnknownSigner,
    /// A required hard domain is absent.
    MissingHardDomain,
    /// The supplied store, receipt, or frontier set is partial.
    PartialEvidence,
}

/// Why cold start must stop for offline owner recovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::resident) enum ColdStartFailure {
    /// Admitted facts claim incompatible startup states.
    ConflictingEvidence,
    /// Anchor or hard-frontier integrity evidence is unsafe.
    Integrity(ColdStartIntegrityFailure),
    /// Fresh-incarnation trust recovery lacks admitted authority.
    TrustRecoveryUnavailable,
}

/// The only next step permitted by admitted cold-start facts.
///
/// An eligible disposition permits a later owner to attempt that named step.
/// It does not perform the step or make the resident ready.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::resident) enum ColdStartDisposition {
    /// An offline bootstrap may begin.
    OfflineBootstrapEligible,
    /// Only the advances named by the committed receipt may complete.
    CompleteReceiptNamedAnchorAdvancesOnly,
    /// Later normal resident assembly may begin.
    EligibleForLaterNormalAssembly,
    /// Only the admitted inactive deployment genesis may resume.
    ResumeInactiveDeploymentGenesisOnly,
    /// Only the admitted inactive control-store recovery may resume.
    ResumeInactiveRecoveredControlStoreOnly,
    /// Only the trust-recovery receipt's pending anchor advances may complete.
    ResumeTrustRecoveryAnchorAdvancesOnly,
    /// Offline recovery may create fresh trust and control-store incarnations.
    RecoverTrustIntoFreshIncarnationsOffline,
    /// Startup must stop until the owner completes offline recovery.
    FailClosedIntoOfflineOwnerRecovery(ColdStartFailure),
}

/// Classify admitted cold-start facts without performing any effect.
#[must_use]
pub(in crate::resident) const fn classify_cold_start(
    facts: ColdStartFacts,
) -> ColdStartDisposition {
    if let ColdStartTrustState::FreshIncarnationRecoveryAdmitted(_) = facts.trust {
        return match (facts.anchors, facts.continuation) {
            (ColdStartAnchorState::RecoveryFrontierAdmitted, ColdStartContinuation::None) => {
                ColdStartDisposition::RecoverTrustIntoFreshIncarnationsOffline
            }
            _ => ColdStartDisposition::FailClosedIntoOfflineOwnerRecovery(
                ColdStartFailure::ConflictingEvidence,
            ),
        };
    }
    if let ColdStartAnchorState::Unsafe(reason) = facts.anchors {
        return ColdStartDisposition::FailClosedIntoOfflineOwnerRecovery(
            ColdStartFailure::Integrity(reason),
        );
    }
    if let ColdStartContinuation::Conflicting = facts.continuation {
        return ColdStartDisposition::FailClosedIntoOfflineOwnerRecovery(
            ColdStartFailure::ConflictingEvidence,
        );
    }

    match (facts.trust, facts.anchors, facts.continuation) {
        (
            ColdStartTrustState::NoTrustHeads,
            ColdStartAnchorState::NotApplicable,
            ColdStartContinuation::None,
        ) => ColdStartDisposition::OfflineBootstrapEligible,
        (
            ColdStartTrustState::Admitted,
            ColdStartAnchorState::ReceiptNamedAdvancesPending,
            ColdStartContinuation::None,
        ) => ColdStartDisposition::CompleteReceiptNamedAnchorAdvancesOnly,
        (
            ColdStartTrustState::Admitted,
            ColdStartAnchorState::Agrees,
            ColdStartContinuation::None,
        ) => ColdStartDisposition::EligibleForLaterNormalAssembly,
        (
            ColdStartTrustState::Admitted,
            ColdStartAnchorState::Agrees,
            ColdStartContinuation::InactiveDeploymentGenesis,
        ) => ColdStartDisposition::ResumeInactiveDeploymentGenesisOnly,
        (
            ColdStartTrustState::Admitted,
            ColdStartAnchorState::Agrees,
            ColdStartContinuation::InactiveRecoveredControlStore,
        ) => ColdStartDisposition::ResumeInactiveRecoveredControlStoreOnly,
        (
            ColdStartTrustState::Admitted,
            ColdStartAnchorState::TrustRecoveryAdvancesPending,
            ColdStartContinuation::None,
        ) => ColdStartDisposition::ResumeTrustRecoveryAnchorAdvancesOnly,
        (ColdStartTrustState::Unavailable, _, _) => {
            ColdStartDisposition::FailClosedIntoOfflineOwnerRecovery(
                ColdStartFailure::TrustRecoveryUnavailable,
            )
        }
        _ => ColdStartDisposition::FailClosedIntoOfflineOwnerRecovery(
            ColdStartFailure::ConflictingEvidence,
        ),
    }
}

#[cfg(test)]
#[path = "../../tests/internal/resident/cold_start.rs"]
mod tests;
