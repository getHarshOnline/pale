//! Exhaustive classification of what one benchmark lane proves for each profile gate.

use crate::ControlStorageProfileRequirement;

use super::input::WorkloadClass;

/// Proof ceiling of one requirement in one emitted benchmark row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProfileEvidenceDisposition {
    /// The lane emits performance or recovery measurements for this candidate.
    MeasuredCandidate,
    /// The lane measures only part of the requirement's complete policy.
    PartiallyMeasuredCandidate,
    /// The lane records or exercises the coordinate without selection-grade proof.
    ObservedCandidate,
    /// The lane does not exercise this requirement.
    NotExercised,
    /// Selection requires an explicit owner policy and admission decision.
    OwnerPolicyAdmissionRequired,
    /// Selection requires Pale conformance evidence followed by owner admission.
    EngineConformanceAndOwnerAdmissionRequired,
}

impl ProfileEvidenceDisposition {
    /// Stable report spelling.
    const fn as_str(self) -> &'static str {
        match self {
            Self::MeasuredCandidate => "measured_candidate",
            Self::PartiallyMeasuredCandidate => "partially_measured_candidate",
            Self::ObservedCandidate => "observed_candidate",
            Self::NotExercised => "not_exercised",
            Self::OwnerPolicyAdmissionRequired => "owner_policy_admission_required",
            Self::EngineConformanceAndOwnerAdmissionRequired => {
                "engine_conformance_and_owner_admission_required"
            }
        }
    }
}

/// Classify one admission requirement under the exact exercised workload lane.
///
/// This exhaustive match is the benchmark's semantic coverage owner: adding a
/// new runtime-admission requirement cannot compile until its evidence ceiling
/// is classified here.
pub(crate) const fn disposition(
    requirement: ControlStorageProfileRequirement,
    workload_class: WorkloadClass,
) -> ProfileEvidenceDisposition {
    use ControlStorageProfileRequirement as Requirement;
    use ProfileEvidenceDisposition as Disposition;

    match requirement {
        Requirement::SqliteCompileOptions => Disposition::ObservedCandidate,
        Requirement::SqliteSynchronous
        | Requirement::SqliteCheckpointMode
        | Requirement::SqliteWalAutocheckpointFrames => match workload_class {
            WorkloadClass::ExactAdapterMechanics => Disposition::NotExercised,
            WorkloadClass::SqliteCharacterization => Disposition::MeasuredCandidate,
        },
        Requirement::SqliteDurabilityGrouping => match workload_class {
            WorkloadClass::ExactAdapterMechanics => Disposition::NotExercised,
            WorkloadClass::SqliteCharacterization => Disposition::PartiallyMeasuredCandidate,
        },
        Requirement::SqliteFullFsync | Requirement::SqliteCheckpointFullFsync => {
            match workload_class {
                WorkloadClass::ExactAdapterMechanics => Disposition::NotExercised,
                WorkloadClass::SqliteCharacterization => Disposition::ObservedCandidate,
            }
        }
        Requirement::AuthorityFrameEncoding
        | Requirement::AuthorityCorruptionChecksum
        | Requirement::AuthorityCorruptionChecksumBytes
        | Requirement::AuthorityMaximumEntryBytes
        | Requirement::AuthorityMaximumCheckpointBytes
        | Requirement::AuthorityMaximumCheckpointCandidates
        | Requirement::AuthorityMaximumTailEntries => Disposition::ObservedCandidate,
        Requirement::AuthorityCheckpointCadence => match workload_class {
            WorkloadClass::ExactAdapterMechanics => Disposition::NotExercised,
            WorkloadClass::SqliteCharacterization => Disposition::MeasuredCandidate,
        },
        Requirement::AuthorityRetentionPolicy | Requirement::AuthorityCompactionPolicy => {
            Disposition::OwnerPolicyAdmissionRequired
        }
        Requirement::BackupRestorePolicy => Disposition::EngineConformanceAndOwnerAdmissionRequired,
    }
}

/// Render every admission requirement exactly once in stable requirement order.
pub(crate) fn render_coverage(workload_class: WorkloadClass) -> String {
    ControlStorageProfileRequirement::ALL
        .into_iter()
        .map(|requirement| {
            format!(
                "{}={}",
                requirement.as_str(),
                disposition(requirement, workload_class).as_str()
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}
