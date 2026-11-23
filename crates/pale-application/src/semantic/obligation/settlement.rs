//! Structural evidence-claim settlement without scores or cross-owner substitution.
//!
//! Settlement deterministically classifies admitted claim state. It does not

//! authenticate referenced records, owner policy, or native read-back.

use std::collections::BTreeSet;

use pale_core::records::{
    EffectClass, ImmutableRecordReferenceV1, UnixTimeV1, WorkspaceResourceId,
};

use super::{
    AdmittedEvidenceCoordinates, CoverageGap, EvidenceCoordinateState, EvidenceCoordinates,
    FreshnessCoordinate, FreshnessState, IndependenceRelation, ObligationSubject,
    ProviderEvidenceCoordinate,
};

/// Exact target whose obligation is being settled.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SettlementTarget {
    /// Provider-local request/result delivery only.
    ProviderDelivery,
    /// Conclusion supported by the four evidence dimensions.
    EvidenceConclusion,
    /// Native effect requiring owner-native verification.
    NativeEffect {
        /// Maximum effect admitted for the target.
        effect: EffectClass,
        /// Native owner that alone can verify the consequence.
        owner: WorkspaceResourceId,
    },
    /// Explicit owner acceptance frontier.
    OwnerAcceptance {
        /// Owner of the acceptance decision.
        owner: WorkspaceResourceId,
    },
    /// Owner-controlled merge, publication, release, or canon frontier.
    QualitativeFrontier {
        /// Owner of the qualitative decision.
        owner: WorkspaceResourceId,
        /// Exact frontier kind.
        frontier: QualitativeFrontier,
    },
}

/// Qualitative frontiers at which open-ended judgment is lawful.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QualitativeFrontier {
    /// Owner-controlled merge decision.
    Merge,
    /// Owner-controlled publication decision.
    Publication,
    /// Owner-controlled release decision.
    Release,
    /// Owner-controlled canonicalization decision.
    Canon,
}

/// Structurally admitted native-verification claim, separate from provider delivery.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NativeVerification {
    /// This target has no native consequence to verify.
    NotApplicable,
    /// The claim names exact owner-native evidence said to verify the consequence.
    Verified {
        /// Native owner that produced the verification.
        owner: WorkspaceResourceId,
        /// Exact owner-native verification evidence.
        evidence: ImmutableRecordReferenceV1,
    },
    /// The claim names exact owner-native evidence said to deny the consequence.
    Denied {
        /// Native owner that denied the consequence.
        owner: WorkspaceResourceId,
        /// Exact owner-native denial evidence.
        evidence: ImmutableRecordReferenceV1,
    },
    /// The claim names exact retained evidence said to establish native failure.
    Failed {
        /// Native owner that observed failure.
        owner: WorkspaceResourceId,
        /// Exact owner-native failure evidence.
        evidence: ImmutableRecordReferenceV1,
    },
    /// The claim records evidence that a consequential effect remains uncertain.
    EffectUnknown {
        /// Native owner whose effect remains uncertain.
        owner: WorkspaceResourceId,
        /// Exact uncertainty evidence.
        evidence: ImmutableRecordReferenceV1,
    },
    /// The claim records that required native verification has not arrived.
    Pending {
        /// Native owner from which verification is pending.
        owner: WorkspaceResourceId,
    },
}

/// Structurally admitted owner-policy state for acceptance or qualitative settlement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OwnerPolicyRequirement {
    /// No owner judgment is required for this target.
    NotRequired,
    /// Required owner frontier and its independent evidence state.
    Required {
        /// Owner that controls the frontier.
        owner: WorkspaceResourceId,
        /// Exact owner-policy evidence, or its explicit absence/failure state.
        evidence: EvidenceCoordinateState<ImmutableRecordReferenceV1>,
    },
}

/// Complete typed input to one obligation settlement decision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceSettlementInput {
    /// Exact Package obligation subject.
    pub obligation: ObligationSubject,
    /// Target and effect whose state is classified.
    pub target: SettlementTarget,
    /// Exact compiled generation.
    pub generation: pale_core::records::CompiledGenerationDigest,
    /// Selected provider/evidence coordinate, when one exists.
    pub provider_evidence: ProviderEvidenceCoordinate,
    /// Four non-substitutable evidence dimensions.
    pub evidence: AdmittedEvidenceCoordinates,
    /// Independently current or stale freshness layers.
    pub freshness: FreshnessCoordinate,
    /// Exact contradiction evidence.
    pub contradictions: BTreeSet<ImmutableRecordReferenceV1>,
    /// Owner-native verification state.
    pub native_verification: NativeVerification,
    /// Owner-policy frontier state.
    pub owner_policy: OwnerPolicyRequirement,
    /// Optional expiry; time passing may invalidate, never establish freshness.
    pub expires_at: Option<UnixTimeV1>,
    /// Exact invalidation evidence, when already invalidated.
    pub invalidation: Option<ImmutableRecordReferenceV1>,
}

/// Causal settlement state; variants are not truthy/falsy aliases.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceSettlementState {
    /// Every supplied constitutive claim is structurally classified as current.
    Satisfied,
    /// Admitted evidence exists but is no longer current.
    Stale,
    /// Exact evidence is contradictory.
    Conflicted,
    /// A required owner-controlled judgment has not arrived.
    PendingOwner,
    /// Owner policy or native verification explicitly denied the target.
    Denied,
    /// Required evaluation or native execution failed.
    Failed,
    /// A native effect cannot be proved present or absent.
    EffectUnknown,
    /// The selected provider cannot express a required evidence dimension.
    Unsupported,
    /// A required carrier or coordinate is unavailable.
    Unavailable,
    /// Owner policy explicitly omitted a required dimension.
    Omitted,
    /// Evidence was cut off before complete coverage.
    Truncated,
    /// Evidence is isolated pending adjudication.
    Quarantined,
    /// No causal evidence establishes another state.
    Unknown,
    /// The settlement's explicit validity interval ended.
    Expired,
}

/// Required next action paired with a non-terminal settlement state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResidualAction {
    /// No residual action remains for a satisfied target.
    None,
    /// Obtain the missing owner-controlled decision.
    RequestOwnerDecision,
    /// Refresh stale or expired evidence at its owning layer.
    RefreshEvidence,
    /// Reconcile a native effect whose outcome is uncertain.
    ReconcileNativeEffect,
    /// Resolve contradictory or quarantined evidence.
    AdjudicateEvidence,
    /// Supply a missing evidence carrier or provider capability.
    SupplyEvidenceCarrier,
    /// Retry the failed evaluation or native operation under existing policy.
    Retry,
    /// Stop because owner policy denied the target.
    StopDenied,
}

/// Deterministic settlement classification and its residual action.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SettlementDecision {
    /// Causal settlement state.
    pub state: EvidenceSettlementState,
    /// Next action that follows from that state.
    pub residual_action: ResidualAction,
}

/// Classify one exact obligation claim without scores or cross-owner substitution.
///
/// Provider delivery can settle only [`SettlementTarget::ProviderDelivery`].
/// Native effects require both coverage of their exact [`EffectClass`] and
/// [`NativeVerification::Verified`]; qualitative targets require current
/// evidence from their declared owner.
/// This function does not authenticate those references or prove native truth.
/// In the law below, `o` is the obligation, `t` its target, `L(t)` the target's
/// required freshness layers, `F` freshness, and `D` the evidence dimensions:
/// repeatability `R`, independence `I`, grounding `G`, and coverage `C`.
#[doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/evidence-settlement-precedence.display.mathml"
))]
#[must_use]
pub fn settle_obligation(
    input: &EvidenceSettlementInput,
    checked_at: UnixTimeV1,
) -> SettlementDecision {
    if !input.contradictions.is_empty() {
        return decision(EvidenceSettlementState::Conflicted);
    }
    if input.invalidation.is_some() {
        return decision(EvidenceSettlementState::Stale);
    }
    if input.expires_at.is_some_and(|expiry| checked_at >= expiry) {
        return decision(EvidenceSettlementState::Expired);
    }
    let required_freshness = required_freshness_layers(&input.target);
    if required_freshness
        .iter()
        .any(|layer| matches!(input.freshness.state(*layer), FreshnessState::Stale { .. }))
    {
        return decision(EvidenceSettlementState::Stale);
    }
    if required_freshness.iter().any(|layer| {
        matches!(
            input.freshness.state(*layer),
            FreshnessState::Unknown | FreshnessState::NotApplicable
        )
    }) {
        return decision(EvidenceSettlementState::Unknown);
    }
    let evidence = input.evidence.coordinates();
    if let Some(state) = evidence_failure(evidence) {
        return decision(state);
    }
    if let EvidenceCoordinateState::Current(grounding) = &evidence.grounding.0 {
        if !grounding.contradictions.is_empty() {
            return decision(EvidenceSettlementState::Conflicted);
        }
    }
    if let EvidenceCoordinateState::Current(independence) = &evidence.independence.0 {
        if matches!(independence.relation, IndependenceRelation::Coupled { .. }) {
            return decision(EvidenceSettlementState::Conflicted);
        }
    }
    if let EvidenceCoordinateState::Current(coverage) = &evidence.coverage.0 {
        if let Some(state) = coverage_gap_state(&coverage.gaps) {
            return decision(state);
        }
        if let SettlementTarget::NativeEffect { effect, .. } = &input.target {
            if !coverage.effects.contains(effect) {
                return decision(EvidenceSettlementState::Unknown);
            }
        }
    }
    let target_owner = match &input.target {
        SettlementTarget::NativeEffect { owner, .. }
        | SettlementTarget::OwnerAcceptance { owner }
        | SettlementTarget::QualitativeFrontier { owner, .. } => Some(owner),
        SettlementTarget::ProviderDelivery | SettlementTarget::EvidenceConclusion => None,
    };
    match &input.owner_policy {
        OwnerPolicyRequirement::NotRequired => {}
        OwnerPolicyRequirement::Required { owner, evidence } if target_owner != Some(owner) => {
            return decision(EvidenceSettlementState::Conflicted);
        }
        OwnerPolicyRequirement::Required { evidence, .. } => match evidence {
            EvidenceCoordinateState::Current(_) => {}
            EvidenceCoordinateState::Stale { .. } => {
                return decision(EvidenceSettlementState::Stale)
            }
            EvidenceCoordinateState::Failed { .. } => {
                return decision(EvidenceSettlementState::Failed)
            }
            EvidenceCoordinateState::Quarantined { .. } => {
                return decision(EvidenceSettlementState::Quarantined)
            }
            EvidenceCoordinateState::Unsupported
            | EvidenceCoordinateState::Unavailable
            | EvidenceCoordinateState::Omitted
            | EvidenceCoordinateState::Truncated { .. }
            | EvidenceCoordinateState::Unknown => {
                return decision(EvidenceSettlementState::PendingOwner)
            }
        },
    }
    match (
        &input.target,
        &input.provider_evidence,
        &input.native_verification,
    ) {
        (
            SettlementTarget::ProviderDelivery,
            ProviderEvidenceCoordinate::Bound { .. },
            NativeVerification::NotApplicable,
        ) => decision(EvidenceSettlementState::Satisfied),
        (SettlementTarget::EvidenceConclusion, _, NativeVerification::NotApplicable) => {
            decision(EvidenceSettlementState::Satisfied)
        }
        (
            SettlementTarget::NativeEffect { owner, .. },
            _,
            NativeVerification::Verified {
                owner: verifying_owner,
                ..
            },
        ) if owner == verifying_owner => decision(EvidenceSettlementState::Satisfied),
        (
            SettlementTarget::OwnerAcceptance { .. } | SettlementTarget::QualitativeFrontier { .. },
            _,
            NativeVerification::NotApplicable,
        ) if matches!(input.owner_policy, OwnerPolicyRequirement::Required { .. }) => {
            decision(EvidenceSettlementState::Satisfied)
        }
        (
            SettlementTarget::NativeEffect { owner, .. },
            _,
            NativeVerification::Denied {
                owner: verifying_owner,
                ..
            },
        ) if owner == verifying_owner => decision(EvidenceSettlementState::Denied),
        (
            SettlementTarget::NativeEffect { owner, .. },
            _,
            NativeVerification::Failed {
                owner: verifying_owner,
                ..
            },
        ) if owner == verifying_owner => decision(EvidenceSettlementState::Failed),
        (
            SettlementTarget::NativeEffect { owner, .. },
            _,
            NativeVerification::EffectUnknown {
                owner: verifying_owner,
                ..
            },
        ) if owner == verifying_owner => decision(EvidenceSettlementState::EffectUnknown),
        (
            SettlementTarget::NativeEffect { owner, .. },
            _,
            NativeVerification::Pending {
                owner: verifying_owner,
            },
        ) if owner == verifying_owner => decision(EvidenceSettlementState::EffectUnknown),
        (
            _,
            _,
            NativeVerification::Pending { .. }
            | NativeVerification::Verified { .. }
            | NativeVerification::Denied { .. }
            | NativeVerification::Failed { .. }
            | NativeVerification::EffectUnknown { .. },
        )
        | (
            SettlementTarget::NativeEffect { .. }
            | SettlementTarget::OwnerAcceptance { .. }
            | SettlementTarget::QualitativeFrontier { .. },
            _,
            NativeVerification::NotApplicable,
        ) => decision(EvidenceSettlementState::Unknown),
        (
            SettlementTarget::ProviderDelivery,
            ProviderEvidenceCoordinate::Missing | ProviderEvidenceCoordinate::NotApplicable,
            NativeVerification::NotApplicable,
        ) => decision(EvidenceSettlementState::Unknown),
    }
}

fn required_freshness_layers(target: &SettlementTarget) -> &'static [super::FreshnessLayer] {
    use super::FreshnessLayer::{
        CompiledGeneration, ContextProjectionDependencies, NativeOwnerRevision, ProviderIndex,
    };

    match target {
        SettlementTarget::ProviderDelivery => &[ProviderIndex, CompiledGeneration],
        SettlementTarget::EvidenceConclusion => {
            &[CompiledGeneration, ContextProjectionDependencies]
        }
        SettlementTarget::NativeEffect { .. }
        | SettlementTarget::OwnerAcceptance { .. }
        | SettlementTarget::QualitativeFrontier { .. } => &[
            CompiledGeneration,
            ContextProjectionDependencies,
            NativeOwnerRevision,
        ],
    }
}

fn coverage_gap_state(gaps: &BTreeSet<CoverageGap>) -> Option<EvidenceSettlementState> {
    gaps.iter()
        .map(|gap| match gap {
            CoverageGap::Omitted(_) => EvidenceSettlementState::Omitted,
            CoverageGap::Unsupported(_) => EvidenceSettlementState::Unsupported,
            CoverageGap::Truncated { .. } => EvidenceSettlementState::Truncated,
            CoverageGap::BlindSpot(_) => EvidenceSettlementState::Unknown,
        })
        .max_by_key(|state| evidence_precedence(*state))
}

fn evidence_failure(coordinates: &EvidenceCoordinates) -> Option<EvidenceSettlementState> {
    [
        evidence_state(&coordinates.repeatability.0),
        evidence_state(&coordinates.independence.0),
        evidence_state(&coordinates.grounding.0),
        evidence_state(&coordinates.coverage.0),
    ]
    .into_iter()
    .flatten()
    .max_by_key(|state| evidence_precedence(*state))
}

fn evidence_state<T>(state: &EvidenceCoordinateState<T>) -> Option<EvidenceSettlementState> {
    match state {
        EvidenceCoordinateState::Current(_) => None,
        EvidenceCoordinateState::Stale { .. } => Some(EvidenceSettlementState::Stale),
        EvidenceCoordinateState::Unsupported => Some(EvidenceSettlementState::Unsupported),
        EvidenceCoordinateState::Unavailable => Some(EvidenceSettlementState::Unavailable),
        EvidenceCoordinateState::Omitted => Some(EvidenceSettlementState::Omitted),
        EvidenceCoordinateState::Truncated { .. } => Some(EvidenceSettlementState::Truncated),
        EvidenceCoordinateState::Failed { .. } => Some(EvidenceSettlementState::Failed),
        EvidenceCoordinateState::Quarantined { .. } => Some(EvidenceSettlementState::Quarantined),
        EvidenceCoordinateState::Unknown => Some(EvidenceSettlementState::Unknown),
    }
}

const fn evidence_precedence(state: EvidenceSettlementState) -> u8 {
    match state {
        EvidenceSettlementState::Quarantined => 8,
        EvidenceSettlementState::Failed => 7,
        EvidenceSettlementState::Stale => 6,
        EvidenceSettlementState::Truncated => 5,
        EvidenceSettlementState::Unavailable => 4,
        EvidenceSettlementState::Unsupported => 3,
        EvidenceSettlementState::Omitted => 2,
        EvidenceSettlementState::Unknown => 1,
        EvidenceSettlementState::Satisfied
        | EvidenceSettlementState::Conflicted
        | EvidenceSettlementState::PendingOwner
        | EvidenceSettlementState::Denied
        | EvidenceSettlementState::EffectUnknown
        | EvidenceSettlementState::Expired => 0,
    }
}

const fn decision(state: EvidenceSettlementState) -> SettlementDecision {
    let residual_action = match state {
        EvidenceSettlementState::Satisfied => ResidualAction::None,
        EvidenceSettlementState::Stale | EvidenceSettlementState::Expired => {
            ResidualAction::RefreshEvidence
        }
        EvidenceSettlementState::Conflicted | EvidenceSettlementState::Quarantined => {
            ResidualAction::AdjudicateEvidence
        }
        EvidenceSettlementState::PendingOwner => ResidualAction::RequestOwnerDecision,
        EvidenceSettlementState::Denied => ResidualAction::StopDenied,
        EvidenceSettlementState::Failed => ResidualAction::Retry,
        EvidenceSettlementState::EffectUnknown => ResidualAction::ReconcileNativeEffect,
        EvidenceSettlementState::Unsupported
        | EvidenceSettlementState::Unavailable
        | EvidenceSettlementState::Omitted
        | EvidenceSettlementState::Truncated
        | EvidenceSettlementState::Unknown => ResidualAction::SupplyEvidenceCarrier,
    };
    SettlementDecision {
        state,
        residual_action,
    }
}
