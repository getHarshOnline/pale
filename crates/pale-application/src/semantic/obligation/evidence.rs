//! Structural admission for four independent evidence coordinates and freshness layers.
//!
//! This module checks finite shape, canonical ordering, and cross-field laws. It

//! does not authenticate referenced records or establish owner-native truth.

use std::collections::BTreeSet;

use pale_core::records::{EffectClass, ImmutableRecordReferenceV1, WorkspaceResourceId};

/// Maximum members admitted on one bounded evidence axis.
pub const MAX_EVIDENCE_AXIS_MEMBERS: usize = 4_096;

/// One coordinate's independently retained evidence state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EvidenceCoordinateState<T> {
    /// The caller classifies this structurally admitted coordinate as current.
    Current(T),
    /// Evidence exists but is outside its admitted freshness boundary.
    Stale {
        /// Last admitted coordinate.
        coordinate: T,
        /// Immutable invalidation evidence.
        invalidation: ImmutableRecordReferenceV1,
    },
    /// The selected provider cannot express this dimension.
    Unsupported,
    /// The selected evidence carrier is not available.
    Unavailable,
    /// Owner policy explicitly omitted this dimension.
    Omitted,
    /// A bound was reached before the complete dimension was observed.
    Truncated {
        /// Immutable evidence of the truncation frontier.
        frontier: ImmutableRecordReferenceV1,
    },
    /// Evaluation failed before a qualifying coordinate was established.
    Failed {
        /// Immutable failure evidence.
        evidence: ImmutableRecordReferenceV1,
    },
    /// Contradictory or malformed evidence was isolated for adjudication.
    Quarantined {
        /// Immutable quarantine evidence.
        evidence: ImmutableRecordReferenceV1,
    },
    /// No qualifying evidence establishes this dimension.
    Unknown,
}

/// Typed wrapper preventing the four evidence dimensions from being substituted.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceCoordinate<T>(pub EvidenceCoordinateState<T>);

/// Exact repeatability coordinate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepeatabilityCoordinate {
    /// Exact mechanism evidence.
    pub mechanism: ImmutableRecordReferenceV1,
    /// Exact mechanism-version evidence.
    pub version: ImmutableRecordReferenceV1,
    /// Exact runtime evidence.
    pub runtime: ImmutableRecordReferenceV1,
    /// Deterministic or stochastic repetition contract.
    pub method: RepeatabilityMethod,
}

/// Non-interchangeable deterministic and stochastic repetition evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RepeatabilityMethod {
    /// Repeated executions use no stochastic seed.
    Deterministic {
        /// Positive number of observed repetitions.
        repetitions: u32,
        /// Immutable stability evidence across repetitions.
        stability: ImmutableRecordReferenceV1,
    },
    /// Repeated executions bind exact stochastic inputs and variance evidence.
    Stochastic {
        /// Exact seed or seed-schedule evidence.
        seed: ImmutableRecordReferenceV1,
        /// Positive number of observed repetitions.
        repetitions: u32,
        /// Immutable variance evidence.
        variance: ImmutableRecordReferenceV1,
        /// Immutable stability evidence.
        stability: ImmutableRecordReferenceV1,
    },
}

/// Exact evaluator ancestry and coupling evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndependenceCoordinate {
    /// Candidate-producing execution.
    pub candidate: ImmutableRecordReferenceV1,
    /// Evaluating execution.
    pub evaluator: ImmutableRecordReferenceV1,
    /// Proven independent or explicitly coupled relation.
    pub relation: IndependenceRelation,
}

/// Couplings that prevent an evaluator from being called independent.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum IndependenceCoupling {
    /// Candidate and evaluator share model ancestry.
    Model,
    /// Candidate and evaluator share runtime ancestry.
    Runtime,
    /// Candidate and evaluator share data ancestry.
    Data,
    /// Candidate and evaluator share prompt ancestry.
    Prompt,
    /// Candidate and evaluator share an organizational control boundary.
    Organization,
    /// Leakage evidence connects candidate and evaluator inputs.
    Leakage,
}

/// Whether evaluator independence is established or explicitly qualified.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IndependenceRelation {
    /// Referenced evidence is classified as independent for the admitted scope.
    Independent {
        /// Immutable independence verification.
        verification: ImmutableRecordReferenceV1,
    },
    /// One or more admitted couplings qualify the evaluator relationship.
    Coupled {
        /// Non-empty canonical coupling set.
        couplings: BTreeSet<IndependenceCoupling>,
        /// Immutable relationship assessment.
        assessment: ImmutableRecordReferenceV1,
    },
}

/// Whether one grounding statement was observed or inferred.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum GroundingKind {
    /// The caller classifies the referenced evidence as a direct native observation.
    Observed,
    /// The statement was inferred from separately retained evidence.
    Inferred {
        /// Exact inference basis.
        basis: ImmutableRecordReferenceV1,
    },
}

/// One exact grounded statement without copying its native body.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct GroundingAssertion {
    /// Native or admitted evidence reference.
    pub evidence: ImmutableRecordReferenceV1,
    /// Observed-versus-inferred classification.
    pub kind: GroundingKind,
}

/// Native frontiers, assertions, contradictions, and limitations for grounding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GroundingCoordinate {
    /// Exact native source/runtime frontiers.
    pub frontiers: BTreeSet<ImmutableRecordReferenceV1>,
    /// Grounded statements.
    pub assertions: Vec<GroundingAssertion>,
    /// Contradicting evidence retained for settlement.
    pub contradictions: BTreeSet<ImmutableRecordReferenceV1>,
    /// Explicit evidence limitations.
    pub limitations: BTreeSet<WorkspaceResourceId>,
}

/// One typed coverage gap.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum CoverageGap {
    /// Intended subject was not observed.
    Omitted(WorkspaceResourceId),
    /// Intended subject is unsupported.
    Unsupported(WorkspaceResourceId),
    /// Intended subject was truncated at an exact frontier.
    Truncated {
        /// Intended subject.
        subject: WorkspaceResourceId,
        /// Exact truncation frontier.
        frontier: ImmutableRecordReferenceV1,
    },
    /// A known blind spot remains outside observed scope.
    BlindSpot(WorkspaceResourceId),
}

/// Intended and observed scope for one evidence claim.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoverageCoordinate {
    /// Canonical intended subject scope.
    pub intended: BTreeSet<WorkspaceResourceId>,
    /// Canonical observed subject scope.
    pub observed: BTreeSet<WorkspaceResourceId>,
    /// Explicit omissions, unsupported/truncated cases, and blind spots.
    pub gaps: BTreeSet<CoverageGap>,
    /// Negative cases actually exercised.
    pub negative_cases: BTreeSet<WorkspaceResourceId>,
    /// Effect classes included in the evidence boundary.
    pub effects: BTreeSet<EffectClass>,
}

/// Four independent evidence dimensions required by settlement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceCoordinates {
    /// Repeatability evidence.
    pub repeatability: EvidenceCoordinate<RepeatabilityCoordinate>,
    /// Evaluator-independence evidence.
    pub independence: EvidenceCoordinate<IndependenceCoordinate>,
    /// Native grounding evidence.
    pub grounding: EvidenceCoordinate<GroundingCoordinate>,
    /// Intended-versus-observed coverage evidence.
    pub coverage: EvidenceCoordinate<CoverageCoordinate>,
}

impl EvidenceCoordinates {
    /// Admit all four dimensions after checking their finite structural laws.
    ///
    /// # Errors
    ///
    /// Returns [`EvidenceCoordinateError`] when a dimension is malformed or
    /// exceeds [`MAX_EVIDENCE_AXIS_MEMBERS`].
    pub fn admit(self) -> Result<AdmittedEvidenceCoordinates, EvidenceCoordinateError> {
        validate_evidence_coordinates(&self)?;
        Ok(AdmittedEvidenceCoordinates(self))
    }
}

/// Four structurally admitted evidence dimensions.
///
/// Settlement accepts this state instead of raw coordinates, so malformed
/// repetitions, assertions, couplings, and coverage cannot enter its algebra.
/// Admission does not authenticate the referenced evidence or its owner.
/// In the law below, `Ret(s)` means that `s` is either
/// [`EvidenceCoordinateState::Current`] or [`EvidenceCoordinateState::Stale`].
/// For repeatability, `r` is the repetition count. For independence, `p` and
/// `q` are the candidate and evaluator, `ρ` is their relation, and `K` is its
/// coupling set. For grounding, `F`, `A`, and `X` are frontiers, assertions,
/// and contradictions. For coverage, `T`, `O`, `H`, `N`, and `E` are intended
/// scope, observed scope, the subject-indexed gap map, negative cases, and the
/// effect boundary; `dom(H)` is the set of subjects classified by `H`. `Q`
/// ranges over the bounded evidence collections `B`.
#[doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/evidence-coordinate-admission.display.mathml"
))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedEvidenceCoordinates(EvidenceCoordinates);

impl AdmittedEvidenceCoordinates {
    /// Borrow the admitted four-coordinate evidence.
    #[must_use]
    pub fn coordinates(&self) -> &EvidenceCoordinates {
        &self.0
    }
}

/// Freshness domain; unlike layers cannot refresh one another.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum FreshnessLayer {
    /// Provider-owned derived index or evidence frontier.
    ProviderIndex,
    /// Pale-owned compiled generation, journal, or admitted frontier.
    CompiledGeneration,
    /// Context projection and its exact dependency closure.
    ContextProjectionDependencies,
    /// Native-owner revision or read-back frontier.
    NativeOwnerRevision,
}

/// Freshness state of one exact layer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FreshnessState {
    /// The caller classifies this exact frontier as owner-verified and current.
    Current(ImmutableRecordReferenceV1),
    /// A later fact invalidated the retained frontier.
    Stale {
        /// Last admitted frontier.
        frontier: ImmutableRecordReferenceV1,
        /// Exact invalidation evidence.
        invalidation: ImmutableRecordReferenceV1,
    },
    /// No owner evidence establishes freshness.
    Unknown,
    /// The target does not consume this layer; this is not evidence of currency.
    NotApplicable,
}

/// Four non-substitutable freshness layers used by one settlement.
///
/// Every layer is present even when explicitly inapplicable. Omission therefore
/// cannot be misread as currency.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FreshnessCoordinate {
    /// Provider-owned derived index or evidence frontier.
    pub provider_index: FreshnessState,
    /// Compiled generation, journal, or admitted application frontier.
    pub compiled_generation: FreshnessState,
    /// Context projection and its exact dependency closure.
    pub context_projection_dependencies: FreshnessState,
    /// Native-owner revision or read-back frontier.
    pub native_owner_revision: FreshnessState,
}

impl FreshnessCoordinate {
    /// Borrow the state of one exact freshness layer.
    #[must_use]
    pub fn state(&self, layer: FreshnessLayer) -> &FreshnessState {
        match layer {
            FreshnessLayer::ProviderIndex => &self.provider_index,
            FreshnessLayer::CompiledGeneration => &self.compiled_generation,
            FreshnessLayer::ContextProjectionDependencies => &self.context_projection_dependencies,
            FreshnessLayer::NativeOwnerRevision => &self.native_owner_revision,
        }
    }
}

/// Provider/evidence binding retained separately from native verification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProviderEvidenceCoordinate {
    /// The target does not depend on provider-delivery evidence.
    NotApplicable,
    /// Exact provider/evidence record selected for this settlement.
    Bound {
        /// Exact selected provider coordinate.
        provider: WorkspaceResourceId,
        /// Exact provider-owned delivery evidence.
        evidence: ImmutableRecordReferenceV1,
    },
    /// Provider-delivery evidence is required but has not arrived.
    Missing,
}

/// Validate bounded, non-empty structural properties of evidence coordinates.
///
/// # Errors
///
/// Returns [`EvidenceCoordinateError`] for malformed repeatability or
/// independence, grounding without a canonical disjoint evidence set, coverage
/// without an exact observed-or-gap partition, or an evidence collection that
/// exceeds the initial-v1 bound.
pub fn validate_evidence_coordinates(
    coordinates: &EvidenceCoordinates,
) -> Result<(), EvidenceCoordinateError> {
    if let Some(repeatability) = retained_coordinate(&coordinates.repeatability.0) {
        let repetitions = match &repeatability.method {
            RepeatabilityMethod::Deterministic { repetitions, .. }
            | RepeatabilityMethod::Stochastic { repetitions, .. } => *repetitions,
        };
        if repetitions == 0 {
            return Err(EvidenceCoordinateError::ZeroRepetitions);
        }
    }
    if let Some(independence) = retained_coordinate(&coordinates.independence.0) {
        if matches!(
            &independence.relation,
            IndependenceRelation::Coupled { couplings, .. } if couplings.is_empty()
        ) {
            return Err(EvidenceCoordinateError::EmptyCouplingSet);
        }
        if independence.candidate == independence.evaluator
            && matches!(
                independence.relation,
                IndependenceRelation::Independent { .. }
            )
        {
            return Err(EvidenceCoordinateError::SelfEvaluatedIndependence);
        }
    }
    if let Some(grounding) = retained_coordinate(&coordinates.grounding.0) {
        for (axis, size) in [
            ("grounding frontiers", grounding.frontiers.len()),
            ("grounding assertions", grounding.assertions.len()),
            ("grounding contradictions", grounding.contradictions.len()),
            ("grounding limitations", grounding.limitations.len()),
        ] {
            bounded_axis(axis, size)?;
        }
        if grounding.frontiers.is_empty()
            || (grounding.assertions.is_empty() && grounding.contradictions.is_empty())
        {
            return Err(EvidenceCoordinateError::EmptyGroundingEvidence);
        }
        if !grounding
            .assertions
            .windows(2)
            .all(|pair| pair[0] < pair[1])
        {
            return Err(EvidenceCoordinateError::NonCanonicalGroundingAssertions);
        }
        if grounding
            .assertions
            .iter()
            .map(|assertion| &assertion.evidence)
            .collect::<BTreeSet<_>>()
            .len()
            != grounding.assertions.len()
            || grounding
                .assertions
                .iter()
                .any(|assertion| grounding.contradictions.contains(&assertion.evidence))
        {
            return Err(EvidenceCoordinateError::DuplicateGroundingEvidence);
        }
    }
    if let Some(coverage) = retained_coordinate(&coordinates.coverage.0) {
        for (axis, size) in [
            ("coverage intended scope", coverage.intended.len()),
            ("coverage observed scope", coverage.observed.len()),
            ("coverage gaps", coverage.gaps.len()),
            ("coverage negative cases", coverage.negative_cases.len()),
            ("coverage effects", coverage.effects.len()),
        ] {
            bounded_axis(axis, size)?;
        }
        if coverage.intended.is_empty() {
            return Err(EvidenceCoordinateError::EmptyCoverageIntent);
        }
        if coverage.effects.is_empty() {
            return Err(EvidenceCoordinateError::EmptyCoverageEffects);
        }
        if !coverage.observed.is_subset(&coverage.intended) {
            return Err(EvidenceCoordinateError::ObservedOutsideIntendedScope);
        }
        if coverage.gaps.iter().any(|gap| {
            let subject = match gap {
                CoverageGap::Omitted(subject)
                | CoverageGap::Unsupported(subject)
                | CoverageGap::BlindSpot(subject)
                | CoverageGap::Truncated { subject, .. } => subject,
            };
            !coverage.intended.contains(subject)
        }) {
            return Err(EvidenceCoordinateError::GapOutsideIntendedScope);
        }
        if !coverage.negative_cases.is_subset(&coverage.observed) {
            return Err(EvidenceCoordinateError::NegativeCaseUnobserved);
        }
        let gap_subjects = coverage
            .gaps
            .iter()
            .map(coverage_gap_subject)
            .collect::<BTreeSet<_>>();
        if gap_subjects.len() != coverage.gaps.len() {
            return Err(EvidenceCoordinateError::DuplicateCoverageGap);
        }
        if gap_subjects
            .iter()
            .any(|subject| coverage.observed.contains(*subject))
        {
            return Err(EvidenceCoordinateError::ObservedGapOverlap);
        }
        if coverage
            .observed
            .iter()
            .chain(gap_subjects)
            .cloned()
            .collect::<BTreeSet<_>>()
            != coverage.intended
        {
            return Err(EvidenceCoordinateError::IncompleteCoveragePartition);
        }
    }
    Ok(())
}

fn retained_coordinate<T>(state: &EvidenceCoordinateState<T>) -> Option<&T> {
    match state {
        EvidenceCoordinateState::Current(coordinate)
        | EvidenceCoordinateState::Stale { coordinate, .. } => Some(coordinate),
        EvidenceCoordinateState::Unsupported
        | EvidenceCoordinateState::Unavailable
        | EvidenceCoordinateState::Omitted
        | EvidenceCoordinateState::Truncated { .. }
        | EvidenceCoordinateState::Failed { .. }
        | EvidenceCoordinateState::Quarantined { .. }
        | EvidenceCoordinateState::Unknown => None,
    }
}

const fn coverage_gap_subject(gap: &CoverageGap) -> &WorkspaceResourceId {
    match gap {
        CoverageGap::Omitted(subject)
        | CoverageGap::Unsupported(subject)
        | CoverageGap::BlindSpot(subject)
        | CoverageGap::Truncated { subject, .. } => subject,
    }
}

fn bounded_axis(axis: &'static str, size: usize) -> Result<(), EvidenceCoordinateError> {
    if size > MAX_EVIDENCE_AXIS_MEMBERS {
        Err(EvidenceCoordinateError::AxisTooLarge { axis, size })
    } else {
        Ok(())
    }
}

/// Evidence coordinate violates a finite structural invariant.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum EvidenceCoordinateError {
    /// Repeatability evidence must contain at least one execution.
    #[error("repeatability requires at least one observed execution")]
    ZeroRepetitions,
    /// A coupled evaluator relationship must name at least one coupling.
    #[error("coupled evaluator relation must name at least one coupling")]
    EmptyCouplingSet,
    /// An execution cannot establish its own independence.
    #[error("candidate and evaluator are identical while independence is claimed")]
    SelfEvaluatedIndependence,
    /// Grounding must name a frontier and at least one assertion or contradiction.
    #[error("current or stale grounding evidence has no frontier or statement")]
    EmptyGroundingEvidence,
    /// One evidence record cannot ground unlike assertions or a contradiction.
    #[error("grounding assertions or contradictions reuse one evidence record")]
    DuplicateGroundingEvidence,
    /// Coverage needs a non-empty intended denominator.
    #[error("current or stale coverage has an empty intended scope")]
    EmptyCoverageIntent,
    /// Coverage must declare at least one included effect class.
    #[error("current or stale coverage has no effect class")]
    EmptyCoverageEffects,
    /// Observed scope cannot exceed the intended scope it evaluates.
    #[error("observed coverage contains a subject outside intended scope")]
    ObservedOutsideIntendedScope,
    /// A declared gap must belong to the intended scope.
    #[error("coverage gap names a subject outside intended scope")]
    GapOutsideIntendedScope,
    /// Negative cases are observations and must be present in observed scope.
    #[error("coverage negative case was not observed")]
    NegativeCaseUnobserved,
    /// One intended subject cannot carry multiple incompatible gap states.
    #[error("coverage contains multiple gaps for one intended subject")]
    DuplicateCoverageGap,
    /// One subject cannot be both observed and retained as a coverage gap.
    #[error("coverage subject is both observed and retained as a gap")]
    ObservedGapOverlap,
    /// Observed and explicitly gapped subjects must partition the full intent.
    #[error("coverage leaves an intended subject neither observed nor explicitly gapped")]
    IncompleteCoveragePartition,
    /// Grounding assertions must be strictly ordered and duplicate-free.
    #[error("grounding assertions are not in canonical strict order")]
    NonCanonicalGroundingAssertions,
    /// One evidence axis exceeds the initial-v1 bound.
    #[error("{axis} contains {size} members; maximum is 4096")]
    AxisTooLarge {
        /// Evidence axis.
        axis: &'static str,
        /// Observed member count.
        size: usize,
    },
}
