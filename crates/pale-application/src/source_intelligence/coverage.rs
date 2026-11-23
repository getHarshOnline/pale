//! Exact source-index coverage and edge-provenance admission.

use std::collections::{BTreeMap, BTreeSet};

use pale_core::records::ImmutableRecordReferenceV1;
use serde::Serialize;

/// Maximum subjects on one declared coverage axis.
pub const MAX_SOURCE_COVERAGE_SUBJECTS: usize = 65_536;

/// Independently reported source-index coverage dimension.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceCoverageDimension {
    /// Programming or declaration languages.
    Language,
    /// Source document classes.
    Document,
    /// Parser implementations or parser modes.
    Parser,
    /// Symbol classes.
    Symbol,
    /// Relationship/edge classes.
    EdgeClass,
}

impl SourceCoverageDimension {
    /// Complete required coverage-axis inventory.
    pub const ALL: [Self; 5] = [
        Self::Language,
        Self::Document,
        Self::Parser,
        Self::Symbol,
        Self::EdgeClass,
    ];
}

/// Why one declared coverage subject is not observed.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceCoverageGapKind {
    /// Selected implementation does not support the subject.
    Unsupported,
    /// Admitted indexing policy deliberately omitted the subject.
    Omitted,
    /// Bounded processing stopped before observing the subject.
    Truncated,
    /// Provider attempted the subject and failed.
    Failed,
}

/// Complete partition of one declared coverage denominator.
///
/// Let `D` be the declared denominator, `O` the observed subjects, and `G_k`
/// the subjects in gap class `k`:
#[doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/source-coverage-partition.display.mathml"
))]
/// No unsupported, omitted, failed, or truncated subject can disappear into an
/// aggregate percentage, and a reported percentage is never stored as truth.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SourceCoveragePartition {
    declared: BTreeSet<ImmutableRecordReferenceV1>,
    observed: BTreeSet<ImmutableRecordReferenceV1>,
    gaps: BTreeMap<ImmutableRecordReferenceV1, SourceCoverageGapKind>,
}

impl SourceCoveragePartition {
    /// Admit one exact declared/observed/gap partition.
    ///
    /// # Errors
    ///
    /// Returns [`SourceCoverageError`] when the denominator exceeds its bound,
    /// an observed subject is undeclared, a gap is undeclared, or observed and
    /// gap subjects do not form the complete disjoint declared set.
    pub fn admit(
        declared: BTreeSet<ImmutableRecordReferenceV1>,
        observed: BTreeSet<ImmutableRecordReferenceV1>,
        gaps: BTreeMap<ImmutableRecordReferenceV1, SourceCoverageGapKind>,
    ) -> Result<Self, SourceCoverageError> {
        if declared.len() > MAX_SOURCE_COVERAGE_SUBJECTS {
            return Err(SourceCoverageError::TooManySubjects(declared.len()));
        }
        if !observed.is_subset(&declared) {
            return Err(SourceCoverageError::ObservedOutsideDenominator);
        }
        if gaps.keys().any(|subject| !declared.contains(subject)) {
            return Err(SourceCoverageError::GapOutsideDenominator);
        }
        if gaps.keys().any(|subject| observed.contains(subject)) {
            return Err(SourceCoverageError::ObservedGapOverlap);
        }
        if observed.len() + gaps.len() != declared.len() {
            return Err(SourceCoverageError::IncompletePartition);
        }
        Ok(Self {
            declared,
            observed,
            gaps,
        })
    }

    /// Exact declared denominator.
    #[must_use]
    pub fn denominator(&self) -> usize {
        self.declared.len()
    }

    /// Borrow the exact observed subset.
    #[must_use]
    pub fn observed(&self) -> &BTreeSet<ImmutableRecordReferenceV1> {
        &self.observed
    }

    /// Borrow every exact non-observation and its independent disposition.
    #[must_use]
    pub fn gaps(&self) -> &BTreeMap<ImmutableRecordReferenceV1, SourceCoverageGapKind> {
        &self.gaps
    }

    /// Borrow the complete declared denominator.
    #[must_use]
    pub fn declared(&self) -> &BTreeSet<ImmutableRecordReferenceV1> {
        &self.declared
    }
}

/// Five-axis coverage report admitted without collapsing denominators.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SourceCoverageReport(BTreeMap<SourceCoverageDimension, SourceCoveragePartition>);

impl SourceCoverageReport {
    /// Admit exactly one partition for each required dimension.
    ///
    /// # Errors
    ///
    /// Returns [`SourceCoverageError::MissingOrExtraDimension`] unless the map
    /// contains exactly [`SourceCoverageDimension::ALL`].
    pub fn admit(
        axes: BTreeMap<SourceCoverageDimension, SourceCoveragePartition>,
    ) -> Result<Self, SourceCoverageError> {
        let expected = SourceCoverageDimension::ALL
            .into_iter()
            .collect::<BTreeSet<_>>();
        let observed = axes.keys().copied().collect::<BTreeSet<_>>();
        if observed != expected {
            return Err(SourceCoverageError::MissingOrExtraDimension);
        }
        Ok(Self(axes))
    }

    /// Borrow one independent coverage partition.
    ///
    /// # Panics
    ///
    /// Panics only if the private invariant established by [`Self::admit`] is
    /// violated and a required axis is missing.
    #[must_use]
    pub fn axis(&self, dimension: SourceCoverageDimension) -> &SourceCoveragePartition {
        self.0
            .get(&dimension)
            .expect("admission installed every required coverage dimension")
    }

    /// Iterate dimensions in their canonical order.
    pub fn axes(
        &self,
    ) -> impl Iterator<Item = (SourceCoverageDimension, &SourceCoveragePartition)> {
        SourceCoverageDimension::ALL
            .into_iter()
            .map(|dimension| (dimension, self.axis(dimension)))
    }
}

/// Provenance method for one returned structural edge.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceEdgeProvenanceKind {
    /// Direct parser observation.
    Parsed,
    /// Name/type/link resolution over parsed observations.
    Resolved,
    /// Provider synthesis from exact admitted inputs.
    Synthesized,
    /// Provider heuristic that must remain visibly non-exact.
    Heuristic,
}

/// Confidence state kept independent from [`SourceEdgeProvenanceKind`].
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case", tag = "state", content = "basis")]
pub enum SourceConfidence {
    /// Exact basis establishes the asserted structural relationship.
    Established(ImmutableRecordReferenceV1),
    /// Provider supplies a bounded estimate with an exact basis reference.
    Estimated(ImmutableRecordReferenceV1),
    /// Provider exposes no confidence conclusion.
    Unknown,
}

/// Exact provenance envelope for one returned relationship.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct SourceEdgeProvenance {
    /// Independent derivation method.
    pub kind: SourceEdgeProvenanceKind,
    /// Immutable evidence for the method.
    pub basis: ImmutableRecordReferenceV1,
    /// Separately typed confidence conclusion and basis.
    pub confidence: SourceConfidence,
}

/// Rejection while admitting source coverage.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SourceCoverageError {
    /// Declared denominator exceeded the finite bound.
    #[error("source coverage contains {0} subjects; maximum is 65536")]
    TooManySubjects(usize),
    /// Provider reported an observed subject outside its denominator.
    #[error("observed source-coverage subject is outside the declared denominator")]
    ObservedOutsideDenominator,
    /// Provider reported a gap outside its denominator.
    #[error("source-coverage gap is outside the declared denominator")]
    GapOutsideDenominator,
    /// One subject is simultaneously observed and unavailable.
    #[error("source-coverage subject is both observed and a gap")]
    ObservedGapOverlap,
    /// Declared subjects are missing from both output partitions.
    #[error("source coverage does not completely partition its declared denominator")]
    IncompletePartition,
    /// Report omitted or added an axis.
    #[error("source coverage must contain exactly language, document, parser, symbol, and edge-class axes")]
    MissingOrExtraDimension,
}
