//! Pure fail-closed classification of unresolved control-storage profiles.

use pale_core::canon::JSON_SAFE_INTEGER_MAX;
use serde::Deserialize;

/// Adapter-private classification of one decoded profile frontier.
///
/// The current source contract admits only [`Self::Unresolved`]. A complete
/// measurement claim can be a structurally bounded candidate, but no manifest
/// field is an owner-admission receipt and therefore cannot produce a selected
/// state in this initial-v1 adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProfileAdmissionV1 {
    /// Exact unresolved source frontier admitted for structural initialization.
    Unresolved,
    /// Complete bounded claim that still requires a later owner admission.
    CandidateRequiresOwnerAdmission,
    /// Contradictory, incomplete, or mechanically invalid claim.
    Invalid,
}

/// Closed manifest spellings for `SQLite` transaction synchronization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SqliteSynchronousV1 {
    Normal,
    Full,
    Extra,
}

impl SqliteSynchronousV1 {
    /// Parse one lowercase manifest spelling without accepting `SQLite` aliases.
    const fn parse(value: &str) -> Option<Self> {
        match value.as_bytes() {
            b"normal" => Some(Self::Normal),
            b"full" => Some(Self::Full),
            b"extra" => Some(Self::Extra),
            _ => None,
        }
    }
}

/// Closed manifest spellings for explicit `SQLite` WAL checkpoint operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SqliteCheckpointModeV1 {
    Passive,
    Full,
    Restart,
    Truncate,
}

impl SqliteCheckpointModeV1 {
    /// Parse one lowercase manifest spelling without accepting `SQLite` aliases.
    const fn parse(value: &str) -> Option<Self> {
        match value.as_bytes() {
            b"passive" => Some(Self::Passive),
            b"full" => Some(Self::Full),
            b"restart" => Some(Self::Restart),
            b"truncate" => Some(Self::Truncate),
            _ => None,
        }
    }
}

/// Closed lifecycle state for one unresolved physical profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MeasurementStatusV1 {
    /// At least one evidence or owner-policy coordinate remains unresolved.
    RequiredBeforeRuntimeAdmission,
    /// Manifest claim that every coordinate is populated; not owner admission.
    Selected,
}

/// Storage coordinate that must be resolved before Pale Runtime assembly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ControlStorageProfileRequirement {
    /// Exact sorted `SQLite` compile-option fingerprint.
    SqliteCompileOptions,
    /// Transaction synchronization strength and barrier interpretation.
    SqliteSynchronous,
    /// Apple full-filesystem synchronization behavior.
    SqliteFullFsync,
    /// Apple checkpoint full-filesystem synchronization behavior.
    SqliteCheckpointFullFsync,
    /// WAL checkpoint mode selected from measured recovery and starvation evidence.
    SqliteCheckpointMode,
    /// Automatic WAL checkpoint threshold in frames.
    SqliteWalAutocheckpointFrames,
    /// Durability grouping and maximum release delay.
    SqliteDurabilityGrouping,
    /// Exact physical encoding of one length-framed journal or checkpoint payload.
    AuthorityFrameEncoding,
    /// Corruption-checksum algorithm used by the physical frame.
    AuthorityCorruptionChecksum,
    /// Fixed checksum width required by the selected algorithm.
    AuthorityCorruptionChecksumBytes,
    /// Store-admitted hard byte ceiling for one authority-journal entry.
    AuthorityMaximumEntryBytes,
    /// Store-admitted hard byte ceiling for one recovery checkpoint.
    AuthorityMaximumCheckpointBytes,
    /// Maximum newest-first checkpoint candidates returned by one recovery read.
    AuthorityMaximumCheckpointCandidates,
    /// Maximum contiguous journal-tail entries returned by one recovery read.
    AuthorityMaximumTailEntries,
    /// Recovery-checkpoint cadence selected from append and recovery evidence.
    AuthorityCheckpointCadence,
    /// Journal/checkpoint retention policy selected from bounded recovery evidence.
    AuthorityRetentionPolicy,
    /// Prefix-compaction policy that preserves the selected recovery invariant.
    AuthorityCompactionPolicy,
    /// Verified backup and inactive-restore policy for forward migration and recovery.
    BackupRestorePolicy,
}

impl ControlStorageProfileRequirement {
    /// Complete stable requirement universe in report and admission order.
    pub const ALL: [Self; 18] = [
        Self::SqliteCompileOptions,
        Self::SqliteSynchronous,
        Self::SqliteFullFsync,
        Self::SqliteCheckpointFullFsync,
        Self::SqliteCheckpointMode,
        Self::SqliteWalAutocheckpointFrames,
        Self::SqliteDurabilityGrouping,
        Self::AuthorityFrameEncoding,
        Self::AuthorityCorruptionChecksum,
        Self::AuthorityCorruptionChecksumBytes,
        Self::AuthorityMaximumEntryBytes,
        Self::AuthorityMaximumCheckpointBytes,
        Self::AuthorityMaximumCheckpointCandidates,
        Self::AuthorityMaximumTailEntries,
        Self::AuthorityCheckpointCadence,
        Self::AuthorityRetentionPolicy,
        Self::AuthorityCompactionPolicy,
        Self::BackupRestorePolicy,
    ];

    /// Stable evidence-report spelling of this runtime-admission blocker.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SqliteCompileOptions => "sqlite_compile_options",
            Self::SqliteSynchronous => "sqlite_synchronous",
            Self::SqliteFullFsync => "sqlite_fullfsync",
            Self::SqliteCheckpointFullFsync => "sqlite_checkpoint_fullfsync",
            Self::SqliteCheckpointMode => "sqlite_checkpoint_mode",
            Self::SqliteWalAutocheckpointFrames => "sqlite_wal_autocheckpoint_frames",
            Self::SqliteDurabilityGrouping => "sqlite_durability_grouping",
            Self::AuthorityFrameEncoding => "authority_frame_encoding",
            Self::AuthorityCorruptionChecksum => "authority_corruption_checksum",
            Self::AuthorityCorruptionChecksumBytes => "authority_corruption_checksum_bytes",
            Self::AuthorityMaximumEntryBytes => "authority_maximum_entry_bytes",
            Self::AuthorityMaximumCheckpointBytes => "authority_maximum_checkpoint_bytes",
            Self::AuthorityMaximumCheckpointCandidates => "authority_maximum_checkpoint_candidates",
            Self::AuthorityMaximumTailEntries => "authority_maximum_tail_entries",
            Self::AuthorityCheckpointCadence => "authority_checkpoint_cadence",
            Self::AuthorityRetentionPolicy => "authority_retention_policy",
            Self::AuthorityCompactionPolicy => "authority_compaction_policy",
            Self::BackupRestorePolicy => "backup_restore_policy",
        }
    }
}

/// Closed backup/restore frontier decoded from the storage manifest.
///
/// [`Self::NoneInitialV1`] truthfully records that no verified backup and
/// inactive-restore procedure has been selected. It is valid source state for
/// structural initialization but always blocks Pale Runtime assembly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupRestorePolicyV1 {
    /// Initial local frontier with no admitted backup or restore procedure.
    NoneInitialV1,
}

impl BackupRestorePolicyV1 {
    /// Whether this policy permits Pale Runtime assembly.
    ///
    /// No admitted backup/restore branch exists in the current initial-v1
    /// source, so this always returns `false`.
    #[must_use]
    pub const fn is_selected(self) -> bool {
        false
    }

    /// Return the lifecycle obligation retained by the unresolved policy.
    #[must_use]
    pub fn unresolved_requirements(self) -> Vec<ControlStorageProfileRequirement> {
        vec![ControlStorageProfileRequirement::BackupRestorePolicy]
    }
}

/// Storage-manifest projection of the initial-v1 `SQLite` profile.
///
/// [`Self::journal_mode`] is exactly `wal`; [`Self::synchronous`] admits only
/// `normal`, `full`, or `extra`, and [`Self::checkpoint_mode`] only `passive`,
/// `full`, `restart`, or `truncate`. A compile-options digest is exactly 64
/// lowercase hexadecimal SHA-256 characters. Numeric candidates are positive
/// integers no greater than 9,007,199,254,740,991 and must also fit both `SQLite`
/// `INTEGER` and the current platform's `usize`.
///
/// Every optional member is deliberately `None` in the admitted source
/// manifest until the required durability and performance study freezes one
/// owner-selected value. `fullfsync` and `checkpoint_fullfsync` describe Apple
/// filesystem-barrier behavior; their presence carries no portability claim.
/// Structural initialization may proceed at the exact unresolved frontier;
/// populated candidate fields do not admit normal runtime operation.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SqliteProfileV1 {
    /// Selected durable-journal mode. The only admitted spelling is `wal`.
    pub journal_mode: String,
    /// `SQLite` synchronous policy, pending durability measurement.
    pub synchronous: Option<String>,
    /// Apple full-filesystem synchronization switch, pending measurement.
    pub fullfsync: Option<bool>,
    /// Apple checkpoint synchronization switch, pending measurement.
    pub checkpoint_fullfsync: Option<bool>,
    /// SHA-256 of the exact sorted `SQLite` compile-option set.
    pub compile_options_sha256: Option<String>,
    /// Explicit WAL checkpoint mode, pending recovery measurement.
    pub checkpoint_mode: Option<String>,
    /// Automatic WAL checkpoint threshold, pending workload measurement.
    pub wal_autocheckpoint_frames: Option<u64>,
    /// Group-commit profile, pending associativity and latency proof.
    pub durability_grouping: Option<String>,
    /// Foreign-key enforcement required on every adapter connection.
    pub foreign_keys: bool,
    /// Trusted-schema mode required off on every adapter connection.
    pub trusted_schema: bool,
    /// Closed status preventing unresolved settings from implying admission.
    measurement_status: MeasurementStatusV1,
}

impl SqliteProfileV1 {
    /// Return every `SQLite` member that still blocks Pale Runtime assembly.
    #[must_use]
    pub fn unresolved_requirements(&self) -> Vec<ControlStorageProfileRequirement> {
        let mut unresolved = Vec::new();
        if self.compile_options_sha256.is_none() {
            unresolved.push(ControlStorageProfileRequirement::SqliteCompileOptions);
        }
        if self.synchronous.is_none() {
            unresolved.push(ControlStorageProfileRequirement::SqliteSynchronous);
        }
        if self.fullfsync.is_none() {
            unresolved.push(ControlStorageProfileRequirement::SqliteFullFsync);
        }
        if self.checkpoint_fullfsync.is_none() {
            unresolved.push(ControlStorageProfileRequirement::SqliteCheckpointFullFsync);
        }
        if self.checkpoint_mode.is_none() {
            unresolved.push(ControlStorageProfileRequirement::SqliteCheckpointMode);
        }
        if self.wal_autocheckpoint_frames.is_none() {
            unresolved.push(ControlStorageProfileRequirement::SqliteWalAutocheckpointFrames);
        }
        if self.durability_grouping.is_none() {
            unresolved.push(ControlStorageProfileRequirement::SqliteDurabilityGrouping);
        }
        unresolved
    }

    /// Whether an owner-admitted `SQLite` profile is available.
    ///
    /// This initial-v1 source has no owner-admitted profile, so even a complete
    /// mechanically valid `selected` claim remains an unadmitted candidate and
    /// returns `false`.
    #[must_use]
    pub fn is_selected(&self) -> bool {
        false
    }

    /// Whether this is the exact unresolved source frontier admitted today.
    ///
    /// A partial profile, an invalid candidate, or a populated `selected` claim
    /// is not coherent source-manifest state in this adapter.
    pub(crate) fn has_coherent_frontier(&self) -> bool {
        self.admission() == ProfileAdmissionV1::Unresolved
    }

    /// Classify the decoded fields once without mistaking presence for admission.
    fn admission(&self) -> ProfileAdmissionV1 {
        let fixed_profile_is_valid =
            self.journal_mode == "wal" && self.foreign_keys && !self.trusted_schema;
        let all_measurement_fields_absent = self.synchronous.is_none()
            && self.fullfsync.is_none()
            && self.checkpoint_fullfsync.is_none()
            && self.compile_options_sha256.is_none()
            && self.checkpoint_mode.is_none()
            && self.wal_autocheckpoint_frames.is_none()
            && self.durability_grouping.is_none();

        match self.measurement_status {
            MeasurementStatusV1::RequiredBeforeRuntimeAdmission => {
                if fixed_profile_is_valid && all_measurement_fields_absent {
                    ProfileAdmissionV1::Unresolved
                } else {
                    ProfileAdmissionV1::Invalid
                }
            }
            MeasurementStatusV1::Selected => {
                let candidate_is_valid = fixed_profile_is_valid
                    && self
                        .synchronous
                        .as_deref()
                        .is_some_and(|value| SqliteSynchronousV1::parse(value).is_some())
                    && self.fullfsync.is_some()
                    && self.checkpoint_fullfsync.is_some()
                    && self
                        .compile_options_sha256
                        .as_deref()
                        .is_some_and(is_lowercase_sha256)
                    && self
                        .checkpoint_mode
                        .as_deref()
                        .is_some_and(|value| SqliteCheckpointModeV1::parse(value).is_some())
                    && self
                        .wal_autocheckpoint_frames
                        .is_some_and(is_portable_sqlite_integer)
                    && self
                        .durability_grouping
                        .as_deref()
                        .is_some_and(|value| !value.is_empty());
                if candidate_is_valid {
                    ProfileAdmissionV1::CandidateRequiresOwnerAdmission
                } else {
                    ProfileAdmissionV1::Invalid
                }
            }
        }
    }
}

/// Storage-manifest projection of the physical authority-journal profile.
///
/// This profile owns corruption detection, physical framing, recovery-read
/// ceilings, and checkpoint maintenance. It does not authenticate a writer or
/// application payload; signatures and semantic re-admission remain separate
/// boundaries. Numeric candidates are positive, no greater than the portable
/// JCS safe-integer ceiling 9,007,199,254,740,991, representable as `SQLite`
/// `INTEGER`, and representable as `usize` on the compiling platform. A checksum
/// width must also fit both candidate payload ceilings.
///
/// Frame, checksum, cadence, retention, and compaction identifiers currently
/// have no selected spelling or algorithm/width pairing. Their nonempty
/// presence can describe only an unadmitted candidate; it cannot authenticate
/// selection. Every optional member therefore stays `None` in the admitted
/// source manifest until named-hardware measurement and owner admission select
/// one coherent profile.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorityJournalProfileV1 {
    /// Exact versioned length-frame encoding identifier.
    pub frame_encoding: Option<String>,
    /// Corruption-checksum algorithm identifier; never a signature or MAC claim.
    pub corruption_checksum: Option<String>,
    /// Fixed checksum output width in bytes.
    pub corruption_checksum_bytes: Option<u32>,
    /// Hard byte ceiling for one application-owned journal entry.
    pub maximum_entry_bytes: Option<u64>,
    /// Hard byte ceiling for one handle-free recovery checkpoint.
    pub maximum_checkpoint_bytes: Option<u64>,
    /// Maximum checkpoint candidates read newest-first during one recovery.
    pub maximum_checkpoint_candidates: Option<u64>,
    /// Maximum contiguous journal entries read after a recovery frontier.
    pub maximum_tail_entries: Option<u64>,
    /// Exact measured checkpoint-cadence policy identifier.
    pub checkpoint_cadence: Option<String>,
    /// Exact measured journal/checkpoint retention-policy identifier.
    pub retention_policy: Option<String>,
    /// Exact measured prefix-compaction policy identifier.
    pub compaction_policy: Option<String>,
    /// Closed status preventing unresolved mechanics from implying admission.
    measurement_status: MeasurementStatusV1,
}

impl AuthorityJournalProfileV1 {
    /// Return every physical journal member that still blocks Pale Runtime assembly.
    #[must_use]
    pub fn unresolved_requirements(&self) -> Vec<ControlStorageProfileRequirement> {
        let mut unresolved = Vec::new();
        if self.frame_encoding.is_none() {
            unresolved.push(ControlStorageProfileRequirement::AuthorityFrameEncoding);
        }
        if self.corruption_checksum.is_none() {
            unresolved.push(ControlStorageProfileRequirement::AuthorityCorruptionChecksum);
        }
        if self.corruption_checksum_bytes.is_none() {
            unresolved.push(ControlStorageProfileRequirement::AuthorityCorruptionChecksumBytes);
        }
        if self.maximum_entry_bytes.is_none() {
            unresolved.push(ControlStorageProfileRequirement::AuthorityMaximumEntryBytes);
        }
        if self.maximum_checkpoint_bytes.is_none() {
            unresolved.push(ControlStorageProfileRequirement::AuthorityMaximumCheckpointBytes);
        }
        if self.maximum_checkpoint_candidates.is_none() {
            unresolved.push(ControlStorageProfileRequirement::AuthorityMaximumCheckpointCandidates);
        }
        if self.maximum_tail_entries.is_none() {
            unresolved.push(ControlStorageProfileRequirement::AuthorityMaximumTailEntries);
        }
        if self.checkpoint_cadence.is_none() {
            unresolved.push(ControlStorageProfileRequirement::AuthorityCheckpointCadence);
        }
        if self.retention_policy.is_none() {
            unresolved.push(ControlStorageProfileRequirement::AuthorityRetentionPolicy);
        }
        if self.compaction_policy.is_none() {
            unresolved.push(ControlStorageProfileRequirement::AuthorityCompactionPolicy);
        }
        unresolved
    }

    /// Whether an owner-admitted authority-journal profile is available.
    ///
    /// This initial-v1 source has no owner-admitted frame, checksum, or policy
    /// identity. Complete candidate fields therefore still return `false`.
    #[must_use]
    pub fn is_selected(&self) -> bool {
        false
    }

    /// Whether this is the exact all-unresolved source frontier admitted today.
    ///
    /// A partial profile, an invalid candidate, or a populated `selected` claim
    /// is not coherent source-manifest state in this adapter.
    pub(crate) fn has_coherent_frontier(&self) -> bool {
        self.admission() == ProfileAdmissionV1::Unresolved
    }

    /// Classify the decoded fields once without inventing policy spellings.
    fn admission(&self) -> ProfileAdmissionV1 {
        let all_measurement_fields_absent = self.frame_encoding.is_none()
            && self.corruption_checksum.is_none()
            && self.corruption_checksum_bytes.is_none()
            && self.maximum_entry_bytes.is_none()
            && self.maximum_checkpoint_bytes.is_none()
            && self.maximum_checkpoint_candidates.is_none()
            && self.maximum_tail_entries.is_none()
            && self.checkpoint_cadence.is_none()
            && self.retention_policy.is_none()
            && self.compaction_policy.is_none();

        match self.measurement_status {
            MeasurementStatusV1::RequiredBeforeRuntimeAdmission => {
                if all_measurement_fields_absent {
                    ProfileAdmissionV1::Unresolved
                } else {
                    ProfileAdmissionV1::Invalid
                }
            }
            MeasurementStatusV1::Selected => {
                let strings_are_nonempty = [
                    self.frame_encoding.as_deref(),
                    self.corruption_checksum.as_deref(),
                    self.checkpoint_cadence.as_deref(),
                    self.retention_policy.as_deref(),
                    self.compaction_policy.as_deref(),
                ]
                .into_iter()
                .all(|value| value.is_some_and(|value| !value.is_empty()));
                let numeric_bounds = [
                    self.corruption_checksum_bytes.map(u64::from),
                    self.maximum_entry_bytes,
                    self.maximum_checkpoint_bytes,
                    self.maximum_checkpoint_candidates,
                    self.maximum_tail_entries,
                ];
                let numeric_bounds_are_valid = numeric_bounds
                    .into_iter()
                    .all(|value| value.is_some_and(is_portable_sqlite_integer));
                let checksum_fits_payload_bounds = self
                    .corruption_checksum_bytes
                    .zip(self.maximum_entry_bytes)
                    .zip(self.maximum_checkpoint_bytes)
                    .is_some_and(
                        |((checksum_bytes, maximum_entry_bytes), maximum_checkpoint_bytes)| {
                            let checksum_bytes = u64::from(checksum_bytes);
                            checksum_bytes <= maximum_entry_bytes
                                && checksum_bytes <= maximum_checkpoint_bytes
                        },
                    );

                if strings_are_nonempty && numeric_bounds_are_valid && checksum_fits_payload_bounds
                {
                    ProfileAdmissionV1::CandidateRequiresOwnerAdmission
                } else {
                    ProfileAdmissionV1::Invalid
                }
            }
        }
    }
}

/// Whether `value` is one exact lowercase hexadecimal SHA-256 spelling.
fn is_lowercase_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .as_bytes()
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
}

/// Whether a manifest integer survives JCS, `SQLite`, and platform conversion.
fn is_portable_sqlite_integer(value: u64) -> bool {
    value != 0
        && value <= JSON_SAFE_INTEGER_MAX
        && i64::try_from(value).is_ok()
        && usize::try_from(value).is_ok()
}

#[cfg(test)]
#[path = "../tests/internal/profile.rs"]
mod tests;
