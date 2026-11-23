//! Bounded, deterministic diagnostics for repository semantic projection.
//!
//! A diagnostic retains one typed [`SemanticCause`]. Its stage, code, subject,
//! target, and source coordinates are derived from that cause rather than stored
//! as parallel strings. Human, JSON, and protocol projections can therefore
//! share one cause without acquiring divergent classification rules.

use std::collections::BinaryHeap;

use pale_core::records::{
    CapabilityIdV1, LogicalRepositoryIdV1, ModeAxis, PackageResourceKindV1, PackageResourceRefV1,
    ProfileDigest,
};
use serde::Serialize;

/// Maximum number of independently actionable semantic diagnostics retained.
pub const MAX_SEMANTIC_DIAGNOSTICS: usize = 128;

/// Compiler phase that established one semantic contradiction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticStage {
    /// Integrity check over the already sealed Package graph.
    GraphIntegrity,
    /// Repository occurrence selection and private binding membership.
    OccurrenceSelection,
    /// Dependency closure and content composition.
    Composition,
    /// Private capability, provider, command, or mode-default join.
    PrivateJoin,
}

/// Stable machine-readable class of one semantic contradiction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticDiagnosticCode {
    /// Repository input is not the unique matching effective-profile member.
    RepositoryProfileMismatch,
    /// A selected semantic resource has no admitted content body.
    MissingBody,
    /// A dependency relation contains a directed cycle.
    CompositionCycle,
    /// A selected resource does not have the required semantic kind.
    WrongResourceKind,
    /// Private configuration repeats one operation binding.
    DuplicateOperationBinding,
    /// A private operation binding targets an unselected resource.
    OperationBindingOutsideSelection,
    /// A mode default does not select a usable mode on its axis.
    InvalidModeDefault,
    /// A private capability binding cannot be projected for one repository.
    InvalidCapabilityBinding,
}

/// Typed coordinate named as the subject or target of a diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum SemanticCoordinate {
    /// The complete selection-bounded Package graph.
    SealedPackageGraph,
    /// Effective project-profile identity.
    Profile(ProfileDigest),
    /// Repository occurrence identity.
    Repository(LogicalRepositoryIdV1),
    /// Owner-qualified Package semantic resource.
    Resource(PackageResourceRefV1),
    /// Provider-neutral capability identity.
    Capability(CapabilityIdV1),
    /// Orthogonal run-mode axis.
    ModeAxis(ModeAxis),
}

/// Typed source coordinate from which a diagnostic was established.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum SemanticDiagnosticSource {
    /// The selection-bounded, digest-verified Package graph.
    SealedPackageGraph,
    /// Private effective profile joined after Package graph sealing.
    EffectiveProfile,
    /// One repository occurrence in the effective profile.
    Repository(LogicalRepositoryIdV1),
    /// One admitted owner-qualified Package resource.
    PackageResource(PackageResourceRefV1),
}

/// Dependency algebra whose deterministic traversal failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticComposition {
    /// Doctrine inheritance and requirement closure.
    Doctrine,
    /// Doctrine required by an operation or role.
    RequiredDoctrine,
    /// Operation-archetype inheritance.
    OperationArchetype,
    /// Role-lens inheritance.
    RoleLens,
    /// Role-archetype inheritance.
    RoleArchetype,
    /// Same-axis run-mode inheritance.
    RunMode,
}

impl std::fmt::Display for SemanticComposition {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Doctrine => "doctrine",
            Self::RequiredDoctrine => "required doctrine",
            Self::OperationArchetype => "operation archetype",
            Self::RoleLens => "role lens",
            Self::RoleArchetype => "role archetype",
            Self::RunMode => "run mode",
        })
    }
}

/// Required resource shape at a consuming semantic boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(tag = "expectation", content = "kind", rename_all = "snake_case")]
pub enum SemanticResourceExpectation {
    /// Any resource retained in the selected closure.
    SelectedResource,
    /// One exact semantic kind.
    Kind(PackageResourceKindV1),
    /// A run mode on the same axis as the extending mode.
    SameAxisRunMode,
}

/// Exact reason one qualified mode cannot become an effective default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InvalidModeDefaultReason {
    /// The default is outside the repository's selected resource closure.
    OutsideSelectedClosure,
    /// The selected mode belongs to a different axis.
    WrongAxis,
    /// A session-only mode was selected as a persistent default.
    SessionOnly,
}

/// Exact reason a private capability binding cannot be projected.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum InvalidCapabilityBindingReason {
    /// One repository-supplied command violates the shell-free grammar.
    CommandOutsideGrammar {
        /// One-based position in the repository command inventory.
        command_index: usize,
    },
}

/// Single source of truth for one semantic diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(tag = "cause", rename_all = "snake_case")]
pub enum SemanticCause {
    /// Repository input is not the unique equal member of its effective profile.
    RepositoryProfileMismatch {
        /// Effective profile being projected.
        profile: ProfileDigest,
        /// Repository input that failed exact membership.
        repository: LogicalRepositoryIdV1,
    },
    /// A selected Package resource lacks its admitted Markdown body.
    MissingBody {
        /// Resource whose content custody is incomplete.
        resource: PackageResourceRefV1,
    },
    /// A finite dependency walk encountered an active node again.
    CompositionCycle {
        /// Semantic relation being closed.
        composition: SemanticComposition,
        /// Owner-qualified cycle witness, with the first node repeated last.
        path: Vec<PackageResourceRefV1>,
    },
    /// A consuming phase received a resource of another semantic kind.
    WrongResourceKind {
        /// Resource that failed the consuming phase's expectation.
        resource: PackageResourceRefV1,
        /// Shape required by the consuming phase.
        expected: SemanticResourceExpectation,
    },
    /// Private configuration repeats one owner-qualified operation binding.
    DuplicateOperationBinding {
        /// Operation with more than one binding.
        resource: PackageResourceRefV1,
    },
    /// A private binding targets an operation outside repository selection.
    OperationBindingOutsideSelection {
        /// Unselected bound operation.
        resource: PackageResourceRefV1,
    },
    /// A project or user default cannot select the requested run mode.
    InvalidModeDefault {
        /// Axis receiving the default.
        axis: ModeAxis,
        /// Owner-qualified selected mode.
        resource: PackageResourceRefV1,
        /// Typed rejection reason.
        reason: InvalidModeDefaultReason,
    },
    /// Repository command expansion produced invalid private mechanics.
    InvalidCapabilityBinding {
        /// Capability whose private binding was being projected.
        capability: CapabilityIdV1,
        /// Repository occurrence supplying the command inventory.
        repository: LogicalRepositoryIdV1,
        /// Typed rejection reason.
        reason: InvalidCapabilityBindingReason,
    },
}

impl SemanticCause {
    /// Compiler phase that owns this cause.
    #[must_use]
    pub const fn stage(&self) -> SemanticStage {
        match self {
            Self::RepositoryProfileMismatch { .. }
            | Self::DuplicateOperationBinding { .. }
            | Self::OperationBindingOutsideSelection { .. } => SemanticStage::OccurrenceSelection,
            Self::MissingBody { .. }
            | Self::CompositionCycle { .. }
            | Self::WrongResourceKind { .. } => SemanticStage::Composition,
            Self::InvalidModeDefault { .. } | Self::InvalidCapabilityBinding { .. } => {
                SemanticStage::PrivateJoin
            }
        }
    }

    /// Stable code derived from the cause variant.
    #[must_use]
    pub const fn code(&self) -> SemanticDiagnosticCode {
        match self {
            Self::RepositoryProfileMismatch { .. } => {
                SemanticDiagnosticCode::RepositoryProfileMismatch
            }
            Self::MissingBody { .. } => SemanticDiagnosticCode::MissingBody,
            Self::CompositionCycle { .. } => SemanticDiagnosticCode::CompositionCycle,
            Self::WrongResourceKind { .. } => SemanticDiagnosticCode::WrongResourceKind,
            Self::DuplicateOperationBinding { .. } => {
                SemanticDiagnosticCode::DuplicateOperationBinding
            }
            Self::OperationBindingOutsideSelection { .. } => {
                SemanticDiagnosticCode::OperationBindingOutsideSelection
            }
            Self::InvalidModeDefault { .. } => SemanticDiagnosticCode::InvalidModeDefault,
            Self::InvalidCapabilityBinding { .. } => {
                SemanticDiagnosticCode::InvalidCapabilityBinding
            }
        }
    }

    /// Primary typed coordinate affected by this cause.
    #[must_use]
    pub fn subject(&self) -> SemanticCoordinate {
        match self {
            Self::RepositoryProfileMismatch { repository, .. }
            | Self::InvalidCapabilityBinding { repository, .. } => {
                SemanticCoordinate::Repository(repository.clone())
            }
            Self::MissingBody { resource }
            | Self::WrongResourceKind { resource, .. }
            | Self::DuplicateOperationBinding { resource }
            | Self::OperationBindingOutsideSelection { resource } => {
                SemanticCoordinate::Resource(resource.clone())
            }
            Self::CompositionCycle { path, .. } => path.first().map_or_else(
                || SemanticCoordinate::SealedPackageGraph,
                |resource| SemanticCoordinate::Resource(resource.clone()),
            ),
            Self::InvalidModeDefault { axis, .. } => SemanticCoordinate::ModeAxis(*axis),
        }
    }

    /// Secondary typed coordinate, when the cause relates two owners.
    #[must_use]
    pub fn target(&self) -> Option<SemanticCoordinate> {
        match self {
            Self::RepositoryProfileMismatch { profile, .. } => {
                Some(SemanticCoordinate::Profile(profile.clone()))
            }
            Self::CompositionCycle { path, .. } => {
                path.get(1).cloned().map(SemanticCoordinate::Resource)
            }
            Self::InvalidModeDefault { resource, .. } => {
                Some(SemanticCoordinate::Resource(resource.clone()))
            }
            Self::InvalidCapabilityBinding { capability, .. } => {
                Some(SemanticCoordinate::Capability(capability.clone()))
            }
            Self::MissingBody { .. }
            | Self::WrongResourceKind { .. }
            | Self::DuplicateOperationBinding { .. }
            | Self::OperationBindingOutsideSelection { .. } => None,
        }
    }

    /// Typed authority surface from which the contradiction was established.
    #[must_use]
    pub fn source(&self) -> SemanticDiagnosticSource {
        match self {
            Self::RepositoryProfileMismatch { .. } | Self::DuplicateOperationBinding { .. } => {
                SemanticDiagnosticSource::EffectiveProfile
            }
            Self::OperationBindingOutsideSelection { .. } | Self::InvalidModeDefault { .. } => {
                SemanticDiagnosticSource::EffectiveProfile
            }
            Self::InvalidCapabilityBinding { .. } => match self {
                Self::InvalidCapabilityBinding { repository, .. } => {
                    SemanticDiagnosticSource::Repository(repository.clone())
                }
                _ => unreachable!(),
            },
            Self::MissingBody { resource } | Self::WrongResourceKind { resource, .. } => {
                SemanticDiagnosticSource::PackageResource(resource.clone())
            }
            Self::CompositionCycle { .. } => SemanticDiagnosticSource::SealedPackageGraph,
        }
    }
}

impl std::fmt::Display for SemanticCause {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RepositoryProfileMismatch {
                profile,
                repository,
            } => write!(
                formatter,
                "repository `{repository}` is not the unique exact member of profile `{profile}`"
            ),
            Self::MissingBody { resource } => {
                write!(
                    formatter,
                    "selected Package resource {resource:?} has no admitted body"
                )
            }
            Self::CompositionCycle { composition, path } => write!(
                formatter,
                "cycle in {composition} composition: {}",
                path.iter()
                    .map(super::resource_label)
                    .collect::<Vec<_>>()
                    .join(" -> ")
            ),
            Self::WrongResourceKind { resource, expected } => write!(
                formatter,
                "selected Package resource {resource:?} does not satisfy {expected:?}"
            ),
            Self::DuplicateOperationBinding { resource } => {
                write!(formatter, "private operation bindings repeat {resource:?}")
            }
            Self::OperationBindingOutsideSelection { resource } => write!(
                formatter,
                "private operation binding {resource:?} is outside repository selection"
            ),
            Self::InvalidModeDefault {
                axis,
                resource,
                reason,
            } => write!(
                formatter,
                "invalid {axis} mode default {resource:?}: {reason:?}"
            ),
            Self::InvalidCapabilityBinding {
                capability,
                repository,
                reason,
            } => write!(
                formatter,
                "invalid capability binding {capability} for repository `{repository}`: {reason:?}"
            ),
        }
    }
}

/// One typed, classifiable semantic diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct SemanticDiagnostic {
    cause: SemanticCause,
}

impl SemanticDiagnostic {
    /// Construct a diagnostic from its single typed cause.
    pub(crate) const fn new(cause: SemanticCause) -> Self {
        Self { cause }
    }

    /// Borrow the cause that owns every projected coordinate.
    #[must_use]
    pub const fn cause(&self) -> &SemanticCause {
        &self.cause
    }

    /// Compiler phase derived from [`Self::cause`].
    #[must_use]
    pub const fn stage(&self) -> SemanticStage {
        self.cause.stage()
    }

    /// Stable code derived from [`Self::cause`].
    #[must_use]
    pub const fn code(&self) -> SemanticDiagnosticCode {
        self.cause.code()
    }

    /// Primary typed coordinate derived from [`Self::cause`].
    #[must_use]
    pub fn subject(&self) -> SemanticCoordinate {
        self.cause.subject()
    }

    /// Secondary typed coordinate derived from [`Self::cause`].
    #[must_use]
    pub fn target(&self) -> Option<SemanticCoordinate> {
        self.cause.target()
    }

    /// Typed authority surface derived from [`Self::cause`].
    #[must_use]
    pub fn source(&self) -> SemanticDiagnosticSource {
        self.cause.source()
    }
}

impl std::fmt::Display for SemanticDiagnostic {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "[{:?}/{:?}] {}",
            self.stage(),
            self.code(),
            self.cause
        )
    }
}

/// Stable bounded set of independently actionable semantic diagnostics.
///
/// In the rendered merge law, `A` and `B` are already-bounded diagnostic
/// multisets with cardinality at most `N`, where `N` is
/// [`MAX_SEMANTIC_DIAGNOSTICS`]. Their multiset union is truncated to its
/// smallest canonical prefix.
#[doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/bounded-diagnostic-merge.display.mathml"
))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SemanticDiagnostics {
    entries: Vec<SemanticDiagnostic>,
    truncated: usize,
}

impl SemanticDiagnostics {
    /// Deterministically ordered retained diagnostics.
    #[must_use]
    pub fn entries(&self) -> &[SemanticDiagnostic] {
        &self.entries
    }

    /// Number of additional diagnostics omitted by the bounded accumulator.
    #[must_use]
    pub const fn truncated(&self) -> usize {
        self.truncated
    }

    /// Total contradictions observed before truncation.
    #[must_use]
    pub fn observed_len(&self) -> usize {
        self.entries.len() + self.truncated
    }
}

impl std::fmt::Display for SemanticDiagnostics {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{} semantic contradiction(s)",
            self.observed_len()
        )?;
        for diagnostic in &self.entries {
            write!(formatter, "; {diagnostic}")?;
        }
        if self.truncated > 0 {
            write!(
                formatter,
                "; {} additional diagnostic(s) truncated",
                self.truncated
            )?;
        }
        Ok(())
    }
}

impl std::error::Error for SemanticDiagnostics {}

/// Stable top-`N` accumulator independent of traversal and partitioning.
///
/// The merge law rendered on [`SemanticDiagnostics`] is associative and
/// commutative with an empty identity; `truncated` preserves the exact count
/// excluded from the retained prefix.
#[derive(Default)]
pub(crate) struct DiagnosticAccumulator {
    entries: BinaryHeap<SemanticDiagnostic>,
    truncated: usize,
}

impl DiagnosticAccumulator {
    /// Retain one diagnostic under [`MAX_SEMANTIC_DIAGNOSTICS`].
    pub(crate) fn push(&mut self, diagnostic: SemanticDiagnostic) {
        if self.entries.len() < MAX_SEMANTIC_DIAGNOSTICS {
            self.entries.push(diagnostic);
            return;
        }
        self.truncated += 1;
        if self
            .entries
            .peek()
            .is_some_and(|largest| diagnostic < *largest)
        {
            self.entries.pop();
            self.entries.push(diagnostic);
        }
    }

    /// Capture a result while retaining an independent failure for later review.
    pub(crate) fn capture<T>(&mut self, result: Result<T, SemanticDiagnostic>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(diagnostic) => {
                self.push(diagnostic);
                None
            }
        }
    }

    /// Merge an independently evaluated semantic-validation partition.
    pub(crate) fn merge(&mut self, other: Self) {
        for diagnostic in other.entries {
            self.push(diagnostic);
        }
        self.truncated += other.truncated;
    }

    /// Finish with the complete success value or deterministically sorted failures.
    pub(crate) fn finish<T>(self, value: T) -> Result<T, SemanticDiagnostics> {
        if self.entries.is_empty() {
            return Ok(value);
        }
        let mut entries = self.entries.into_vec();
        entries.sort();
        Err(SemanticDiagnostics {
            entries,
            truncated: self.truncated,
        })
    }
}

#[cfg(test)]
#[path = "../../tests/internal/semantic/diagnostic.rs"]
mod tests;
