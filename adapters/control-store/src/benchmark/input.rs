//! Explicit inputs, parsing, resource projection, and preflight rejection.

use std::collections::BTreeSet;
use std::num::{NonZeroU32, NonZeroUsize};
use std::str::FromStr;

use pale_core::sha256_hex;
use pale_ports::AuthorityRecoveryReadLimits;

use crate::verify_embedded_storage_sources;

/// Full characterization uses the resident-kernel sample floor plus margin.
const CHARACTERIZATION_SAMPLES: usize = 201;
/// Smoke validation proves compilation and mechanics only.
const SMOKE_SAMPLES: usize = 1;
/// Maximum caller-supplied base count before the mandatory 100x expansion.
const MAX_BASE_RECORDS: u64 = 100_000;
/// Maximum single deterministic payload accepted by the harness.
const MAX_PAYLOAD_BYTES: usize = 1_048_576;
/// Maximum distinct direct-`SQLite` candidates retained by one invocation.
const MAX_PRAGMA_CANDIDATES: usize = 64;

/// Workload implementation selected for one explicitly classified run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkloadClass {
    /// Exact unadmitted deployment adapter exercised through its neutral port.
    ExactAdapterMechanics,
    /// Direct `SQLite` instrument, isolated from adapter and profile admission.
    SqliteCharacterization,
}

impl WorkloadClass {
    /// Parse one exact workload-class coordinate.
    pub(crate) fn parse(value: &str) -> Result<Self, String> {
        match value {
            "exact_adapter_mechanics_v1" => Ok(Self::ExactAdapterMechanics),
            "sqlite_characterization" => Ok(Self::SqliteCharacterization),
            _ => Err(
                "workload class must be exact_adapter_mechanics_v1 or sqlite_characterization"
                    .to_owned(),
            ),
        }
    }

    /// Stable report spelling.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::ExactAdapterMechanics => "exact_adapter_mechanics_v1",
            Self::SqliteCharacterization => "sqlite_characterization",
        }
    }
}

/// Evidence classification retained in every emitted row.
#[derive(Debug, Clone, Copy)]
pub(crate) enum EvidenceClass {
    /// Tiny compile-and-execute gate, ineligible for performance decisions.
    SmokeValidation,
    /// Exact adapter mechanics at the explicitly unadmitted profile frontier.
    UnadmittedAdapterMechanics,
    /// Direct `SQLite` characterization with no adapter or profile authority.
    SqliteCharacterization,
}

impl EvidenceClass {
    /// Stable report spelling.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::SmokeValidation => "smoke_validation",
            Self::UnadmittedAdapterMechanics => "unadmitted_adapter_mechanics",
            Self::SqliteCharacterization => "sqlite_characterization",
        }
    }
}

/// `SQLite` synchronization candidate admitted by the harness parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SynchronousMode {
    /// `SQLite` `NORMAL`; comparative candidate, not a durability claim.
    Normal,
    /// `SQLite` `FULL`.
    Full,
    /// `SQLite` `EXTRA`.
    Extra,
}

impl SynchronousMode {
    /// Parse the exact benchmark input spelling.
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "NORMAL" => Ok(Self::Normal),
            "FULL" => Ok(Self::Full),
            "EXTRA" => Ok(Self::Extra),
            _ => Err("synchronous must be NORMAL, FULL, or EXTRA".to_owned()),
        }
    }

    /// Numeric `SQLite` pragma value.
    pub(crate) const fn pragma_value(self) -> i64 {
        match self {
            Self::Normal => 1,
            Self::Full => 2,
            Self::Extra => 3,
        }
    }

    /// Stable report spelling.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "NORMAL",
            Self::Full => "FULL",
            Self::Extra => "EXTRA",
        }
    }
}

/// Explicit `SQLite` WAL checkpoint candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CheckpointMode {
    /// Non-blocking checkpoint that may leave a busy tail.
    Passive,
    /// Complete checkpoint without resetting WAL readers.
    Full,
    /// Complete checkpoint that resets the WAL for the next writer when possible.
    Restart,
    /// Complete checkpoint that also truncates the WAL when possible.
    Truncate,
}

impl CheckpointMode {
    /// Parse the exact benchmark input spelling.
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "PASSIVE" => Ok(Self::Passive),
            "FULL" => Ok(Self::Full),
            "RESTART" => Ok(Self::Restart),
            "TRUNCATE" => Ok(Self::Truncate),
            _ => Err("checkpoint mode must be PASSIVE, FULL, RESTART, or TRUNCATE".to_owned()),
        }
    }

    /// Safe fixed SQL spelling used to construct a pragma statement.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Passive => "PASSIVE",
            Self::Full => "FULL",
            Self::Restart => "RESTART",
            Self::Truncate => "TRUNCATE",
        }
    }
}

/// One bounded `SQLite` pragma and grouping-size candidate.
///
/// The candidate does not specify a maximum durability-group release delay,
/// arrival/concurrency policy, retention, compaction, or backup behavior.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SqlitePragmaCandidate {
    /// Transaction synchronization candidate.
    pub(crate) synchronous: SynchronousMode,
    /// Apple full-filesystem synchronization candidate.
    pub(crate) fullfsync: bool,
    /// Apple checkpoint full-filesystem synchronization candidate.
    pub(crate) checkpoint_fullfsync: bool,
    /// Explicit checkpoint operation.
    pub(crate) checkpoint_mode: CheckpointMode,
    /// `SQLite` automatic-checkpoint threshold in WAL frames.
    pub(crate) wal_autocheckpoint_frames: u32,
    /// Records committed by one durability group.
    pub(crate) group_records: NonZeroUsize,
    /// Durability groups between explicit checkpoints.
    pub(crate) checkpoint_every_groups: NonZeroU32,
}

impl SqlitePragmaCandidate {
    /// Parse `SYNC/fullfsync/checkpoint_fullfsync/MODE/frames/group/cadence`.
    pub(crate) fn parse(value: &str) -> Result<Self, String> {
        let fields = value.split('/').collect::<Vec<_>>();
        let [synchronous, fullfsync, checkpoint_fullfsync, checkpoint_mode, frames, group, cadence] =
            fields.as_slice()
        else {
            return Err(
                "profile must contain seven slash-separated fields: SYNC/fullfsync/checkpoint_fullfsync/MODE/frames/group/cadence"
                    .to_owned(),
            );
        };
        Ok(Self {
            synchronous: SynchronousMode::parse(synchronous)?,
            fullfsync: parse_switch(fullfsync)?,
            checkpoint_fullfsync: parse_switch(checkpoint_fullfsync)?,
            checkpoint_mode: CheckpointMode::parse(checkpoint_mode)?,
            wal_autocheckpoint_frames: parse_number(frames, "WAL autocheckpoint frames")?,
            group_records: NonZeroUsize::new(parse_number(group, "group records")?)
                .ok_or_else(|| "group records must be greater than zero".to_owned())?,
            checkpoint_every_groups: NonZeroU32::new(parse_number(
                cadence,
                "checkpoint cadence groups",
            )?)
            .ok_or_else(|| "checkpoint cadence groups must be greater than zero".to_owned())?,
        })
    }

    /// Stable round-trippable environment and report encoding.
    pub(crate) fn encode(&self) -> String {
        format!(
            "{}/{}/{}/{}/{}/{}/{}",
            self.synchronous.as_str(),
            switch(self.fullfsync),
            switch(self.checkpoint_fullfsync),
            self.checkpoint_mode.as_str(),
            self.wal_autocheckpoint_frames,
            self.group_records,
            self.checkpoint_every_groups,
        )
    }
}

/// Mandatory scale classes derived from one explicit base input.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Scale {
    /// Caller-supplied base count; it is not automatically owner-qualified.
    ActualInput,
    /// Deterministic tenfold expansion.
    TenX,
    /// Deterministic hundredfold expansion.
    HundredX,
}

impl Scale {
    /// Checked multiplier for this scale.
    const fn multiplier(self) -> u64 {
        match self {
            Self::ActualInput => 1,
            Self::TenX => 10,
            Self::HundredX => 100,
        }
    }

    /// Stable report spelling.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::ActualInput => "actual_input",
            Self::TenX => "10x",
            Self::HundredX => "100x",
        }
    }
}

/// Exact externally supplied coordinates required for retained full-run evidence.
#[derive(Debug, Clone, Default)]
pub(crate) struct EvidenceCoordinates {
    /// Exact source/tree identity supplied by the operator.
    pub(crate) source_id: String,
    /// Expected SHA-256 of the executed benchmark binary.
    pub(crate) binary_sha256: String,
    /// Expected canonical storage-manifest SHA-256.
    pub(crate) storage_manifest_sha256: String,
    /// Filesystem identity and relevant mount options.
    pub(crate) filesystem: String,
    /// Storage-device identity and firmware/transport coordinate.
    pub(crate) storage: String,
    /// Exact external-power, battery, and cache envelope.
    pub(crate) power_envelope: String,
    /// Unique operator-assigned run identity.
    pub(crate) run_id: String,
}

impl EvidenceCoordinates {
    /// Require every full-run evidence coordinate without deriving or inventing it.
    fn from_environment() -> Self {
        let coordinates = Self {
            source_id: required_coordinate("PALE_SQLITE_BENCH_SOURCE_ID"),
            binary_sha256: required_coordinate("PALE_SQLITE_BENCH_BINARY_SHA256"),
            storage_manifest_sha256: required_coordinate(
                "PALE_SQLITE_BENCH_STORAGE_MANIFEST_SHA256",
            ),
            filesystem: required_coordinate("PALE_SQLITE_BENCH_FILESYSTEM"),
            storage: required_coordinate("PALE_SQLITE_BENCH_STORAGE"),
            power_envelope: required_coordinate("PALE_SQLITE_BENCH_POWER_ENVELOPE"),
            run_id: required_coordinate("PALE_SQLITE_BENCH_RUN_ID"),
        };
        require_sha256(&coordinates.binary_sha256, "benchmark binary SHA-256");
        require_sha256(
            &coordinates.storage_manifest_sha256,
            "storage manifest SHA-256",
        );
        let observed_manifest = verify_embedded_storage_sources()
            .expect("verify embedded storage source before benchmarking")
            .manifest_sha256()
            .to_owned();
        assert_eq!(
            coordinates.storage_manifest_sha256, observed_manifest,
            "supplied storage manifest SHA-256 does not match the embedded manifest"
        );
        coordinates
    }

    /// Whether all required retained-evidence coordinates are present.
    pub(crate) fn is_complete(&self) -> bool {
        [
            &self.source_id,
            &self.binary_sha256,
            &self.storage_manifest_sha256,
            &self.filesystem,
            &self.storage,
            &self.power_envelope,
            &self.run_id,
        ]
        .into_iter()
        .all(|value| !value.is_empty())
    }
}

/// Explicit operator ceiling checked against the complete run before any workload starts.
#[derive(Debug, Clone, Copy)]
pub(crate) struct OperatorResourceBudget {
    /// Maximum independent profile/scale runs authorized for this invocation.
    pub(crate) maximum_independent_runs: u128,
    /// Maximum pooled benchmark operations authorized for this invocation.
    pub(crate) maximum_operations: u128,
    /// Maximum deterministic journal/checkpoint payload bytes processed.
    pub(crate) maximum_payload_bytes: u128,
}

impl OperatorResourceBudget {
    /// Require a nonzero, explicitly supplied full-run budget.
    fn from_environment() -> Self {
        Self {
            maximum_independent_runs: required_positive_number(
                "PALE_SQLITE_BENCH_BUDGET_MAX_INDEPENDENT_RUNS",
            ),
            maximum_operations: required_positive_number("PALE_SQLITE_BENCH_BUDGET_MAX_OPERATIONS"),
            maximum_payload_bytes: required_positive_number(
                "PALE_SQLITE_BENCH_BUDGET_MAX_PAYLOAD_BYTES",
            ),
        }
    }
}

/// Conservative run-wide projection compared with the explicit operator budget.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ResourceProjection {
    /// Independent regular-plus-recovery runs.
    pub(crate) independent_runs: u128,
    /// Pooled journal writes, checkpoint commits, WAL checkpoints, and recovery reads.
    pub(crate) operations: u128,
    /// Journal/checkpoint payload bytes processed, excluding unknown `SQLite` overhead.
    pub(crate) payload_bytes: u128,
}

impl ResourceProjection {
    fn checked_add(self, other: Self) -> Result<Self, String> {
        Ok(Self {
            independent_runs: self
                .independent_runs
                .checked_add(other.independent_runs)
                .ok_or_else(|| "projected independent runs overflow u128".to_owned())?,
            operations: self
                .operations
                .checked_add(other.operations)
                .ok_or_else(|| "projected operations overflow u128".to_owned())?,
            payload_bytes: self
                .payload_bytes
                .checked_add(other.payload_bytes)
                .ok_or_else(|| "projected payload bytes overflow u128".to_owned())?,
        })
    }
}

/// Explicit benchmark inputs and their proof ceiling.
#[derive(Debug)]
pub(crate) struct BenchmarkConfiguration {
    /// Smoke or characterization evidence class.
    pub(crate) evidence_class: EvidenceClass,
    /// Exact unadmitted adapter mechanics or isolated direct `SQLite` characterization.
    pub(crate) workload_class: WorkloadClass,
    /// Caller-supplied base journal record count.
    actual_records: u64,
    /// Deterministic frame payload bytes per record.
    payload_bytes: usize,
    /// Bytes in each handle-free recovery-checkpoint projection.
    checkpoint_bytes: usize,
    /// Existing port-owned recovery read bounds supplied as benchmark inputs.
    limits: AuthorityRecoveryReadLimits,
    /// Independent runs per profile and scale.
    pub(crate) independent_runs: usize,
    /// Fully specified direct-SQLite pragma candidates; empty for the adapter lane.
    pub(crate) pragma_candidates: Vec<SqlitePragmaCandidate>,
    /// Caller-supplied workload label, never treated as semantic authority.
    pub(crate) workload_label: String,
    /// Explicit named hardware coordinate.
    pub(crate) hardware: String,
    /// Required full-run source, binary, environment, and run coordinates.
    pub(crate) coordinates: EvidenceCoordinates,
    /// Explicit operator execution ceiling, distinct from an acceptance budget.
    pub(crate) operator_budget: OperatorResourceBudget,
    /// Whether the operator supplied the execution ceiling for retained evidence.
    pub(crate) operator_budget_supplied: bool,
    /// Preflighted projection for the complete invocation.
    pub(crate) projection: ResourceProjection,
    /// SHA-256 of the canonical workload class and sorted candidate set.
    pub(crate) candidate_set_sha256: String,
    /// Exact number of data rows required before the report can be sealed.
    pub(crate) expected_rows: usize,
}

impl BenchmarkConfiguration {
    /// Read one explicit run mode, or skip ordinary Cargo target discovery.
    pub(crate) fn from_environment() -> Option<Self> {
        let smoke = benchmark_flag("PALE_SQLITE_BENCH_SMOKE");
        let characterize = benchmark_flag("PALE_BENCH_RUN");
        assert!(
            !(smoke && characterize),
            "select either smoke validation or full characterization"
        );
        if !smoke && !characterize {
            return None;
        }
        if smoke {
            return Some(Self::smoke_validation());
        }

        let workload_class =
            WorkloadClass::parse(&required_coordinate("PALE_SQLITE_BENCH_WORKLOAD_CLASS"))
                .unwrap_or_else(|error| panic!("{error}"));
        let actual_records = required_number("PALE_SQLITE_BENCH_ACTUAL_RECORDS");
        assert!(
            (1..=MAX_BASE_RECORDS).contains(&actual_records),
            "base records must be in 1..={MAX_BASE_RECORDS}"
        );
        let payload_bytes = required_number("PALE_SQLITE_BENCH_PAYLOAD_BYTES");
        assert!(
            (1..=MAX_PAYLOAD_BYTES).contains(&payload_bytes),
            "payload bytes must be in 1..={MAX_PAYLOAD_BYTES}"
        );
        let checkpoint_bytes = required_number("PALE_SQLITE_BENCH_CHECKPOINT_BYTES");
        assert!(
            (1..=MAX_PAYLOAD_BYTES).contains(&checkpoint_bytes),
            "checkpoint bytes must be in 1..={MAX_PAYLOAD_BYTES}"
        );
        let limits = AuthorityRecoveryReadLimits::new(
            required_number("PALE_SQLITE_BENCH_MAXIMUM_ENTRY_BYTES"),
            required_number("PALE_SQLITE_BENCH_MAXIMUM_CHECKPOINT_BYTES"),
            required_number("PALE_SQLITE_BENCH_MAXIMUM_CHECKPOINT_CANDIDATES"),
            required_number("PALE_SQLITE_BENCH_MAXIMUM_TAIL_ENTRIES"),
        )
        .expect("all recovery read limits must be greater than zero");
        assert!(
            payload_bytes <= limits.maximum_entry_bytes(),
            "payload bytes exceed maximum_entry_bytes"
        );
        assert!(
            checkpoint_bytes <= limits.maximum_checkpoint_bytes(),
            "checkpoint bytes exceed maximum_checkpoint_bytes"
        );
        let pragma_candidates = match workload_class {
            WorkloadClass::ExactAdapterMechanics => {
                assert!(
                    std::env::var_os("PALE_SQLITE_BENCH_PROFILES").is_none(),
                    "PALE_SQLITE_BENCH_PROFILES must be unset for exact adapter mechanics"
                );
                Vec::new()
            }
            WorkloadClass::SqliteCharacterization => {
                parse_pragma_candidates(&required_coordinate("PALE_SQLITE_BENCH_PROFILES"))
                    .unwrap_or_else(|error| panic!("{error}"))
            }
        };
        let candidate_set_sha256 = candidate_set_sha256(workload_class, &pragma_candidates);
        let expected_rows = expected_row_count(workload_class, pragma_candidates.len());

        let mut configuration = Self {
            evidence_class: match workload_class {
                WorkloadClass::ExactAdapterMechanics => EvidenceClass::UnadmittedAdapterMechanics,
                WorkloadClass::SqliteCharacterization => EvidenceClass::SqliteCharacterization,
            },
            workload_class,
            actual_records,
            payload_bytes,
            checkpoint_bytes,
            limits,
            independent_runs: CHARACTERIZATION_SAMPLES,
            pragma_candidates,
            workload_label: required_coordinate("PALE_SQLITE_BENCH_WORKLOAD_LABEL"),
            hardware: required_coordinate("PALE_BENCH_HARDWARE"),
            coordinates: EvidenceCoordinates::from_environment(),
            operator_budget: OperatorResourceBudget::from_environment(),
            operator_budget_supplied: true,
            projection: ResourceProjection::default(),
            candidate_set_sha256,
            expected_rows,
        };
        configuration.finish_preflight();
        Some(configuration)
    }

    /// Construct the fixed mechanics-only smoke invocation used by the binary.
    pub(crate) fn smoke_validation() -> Self {
        let workload_class = WorkloadClass::ExactAdapterMechanics;
        let mut configuration = Self {
            evidence_class: EvidenceClass::SmokeValidation,
            workload_class,
            actual_records: 4,
            payload_bytes: 64,
            checkpoint_bytes: 128,
            limits: AuthorityRecoveryReadLimits::new(1_024, 4_096, 2, 1_024)
                .expect("smoke recovery limits are nonzero"),
            independent_runs: SMOKE_SAMPLES,
            pragma_candidates: Vec::new(),
            workload_label: "smoke_only".to_owned(),
            hardware: String::new(),
            coordinates: EvidenceCoordinates::default(),
            operator_budget: OperatorResourceBudget {
                maximum_independent_runs: 3,
                maximum_operations: 2_000,
                maximum_payload_bytes: 100_000,
            },
            operator_budget_supplied: false,
            projection: ResourceProjection::default(),
            candidate_set_sha256: candidate_set_sha256(workload_class, &[]),
            expected_rows: expected_row_count(workload_class, 0),
        };
        configuration.finish_preflight();
        configuration
    }

    /// Resolve every candidate and scale without manufacturing adapter profile values.
    pub(crate) fn cases(&self) -> Vec<BenchmarkCase> {
        let candidates = match self.workload_class {
            WorkloadClass::ExactAdapterMechanics => vec![None],
            WorkloadClass::SqliteCharacterization => {
                self.pragma_candidates.iter().cloned().map(Some).collect()
            }
        };
        let mut cases = Vec::with_capacity(self.expected_rows);
        for candidate in candidates {
            for scale in [Scale::ActualInput, Scale::TenX, Scale::HundredX] {
                cases.push(BenchmarkCase::new(self, candidate.clone(), scale));
            }
        }
        assert_eq!(
            cases.len(),
            self.expected_rows,
            "resolved benchmark cases match the sealed row count"
        );
        cases
    }

    /// Project the entire invocation and reject it before any database is created.
    fn finish_preflight(&mut self) {
        self.projection = project_resources(
            self.actual_records,
            self.payload_bytes,
            self.checkpoint_bytes,
            self.workload_class,
            &self.pragma_candidates,
            self.independent_runs,
        )
        .unwrap_or_else(|error| panic!("benchmark resource projection failed: {error}"));
        preflight_resources(self.projection, self.operator_budget).unwrap_or_else(|error| {
            panic!("benchmark operator budget rejected projection: {error}")
        });
    }
}

/// Read one exact opt-in flag without treating an unknown value as selection.
fn benchmark_flag(name: &str) -> bool {
    match std::env::var_os(name) {
        None => false,
        Some(value) if value == "1" => true,
        Some(_) => panic!("{name} must be 1 when present"),
    }
}

/// Fully resolved workload and scale case.
#[derive(Debug, Clone)]
pub(crate) struct BenchmarkCase {
    /// Direct-SQLite pragma candidate; absent for exact adapter mechanics.
    pub(crate) pragma_candidate: Option<SqlitePragmaCandidate>,
    /// Workload implementation used by this case.
    pub(crate) workload_class: WorkloadClass,
    /// Scale label.
    pub(crate) scale: Scale,
    /// Exact record count after checked expansion.
    pub(crate) records: u64,
    /// Exact bytes in every deterministic frame payload.
    pub(crate) payload_bytes: usize,
    /// Exact bytes in every deterministic checkpoint projection.
    pub(crate) checkpoint_bytes: usize,
    /// Existing port-owned recovery read bounds retained in evidence.
    pub(crate) limits: AuthorityRecoveryReadLimits,
}

impl BenchmarkCase {
    /// Bind one optional characterization candidate to one checked scale.
    pub(crate) fn new(
        configuration: &BenchmarkConfiguration,
        pragma_candidate: Option<SqlitePragmaCandidate>,
        scale: Scale,
    ) -> Self {
        let records = configuration
            .actual_records
            .checked_mul(scale.multiplier())
            .expect("scaled record count overflow");
        Self {
            pragma_candidate,
            workload_class: configuration.workload_class,
            scale,
            records,
            payload_bytes: configuration.payload_bytes,
            checkpoint_bytes: configuration.checkpoint_bytes,
            limits: configuration.limits,
        }
    }
}

/// Parse, bound, deduplicate, and canonically order direct-`SQLite` candidates.
pub(crate) fn parse_pragma_candidates(value: &str) -> Result<Vec<SqlitePragmaCandidate>, String> {
    let parsed = value
        .split(';')
        .map(SqlitePragmaCandidate::parse)
        .collect::<Result<Vec<_>, _>>()?;
    if parsed.is_empty() {
        return Err("at least one pragma candidate is required".to_owned());
    }
    if parsed.len() > MAX_PRAGMA_CANDIDATES {
        return Err(format!(
            "pragma candidate count {} exceeds harness ceiling {MAX_PRAGMA_CANDIDATES}",
            parsed.len()
        ));
    }
    let canonical = parsed.iter().cloned().collect::<BTreeSet<_>>();
    if canonical.len() != parsed.len() {
        return Err("duplicate pragma candidates are not permitted".to_owned());
    }
    Ok(canonical.into_iter().collect())
}

/// Hash the exact workload class and canonically ordered candidate set.
pub(super) fn candidate_set_sha256(
    workload_class: WorkloadClass,
    candidates: &[SqlitePragmaCandidate],
) -> String {
    let mut preimage = format!("workload_class={}\n", workload_class.as_str());
    for candidate in candidates {
        preimage.push_str("candidate=");
        preimage.push_str(&candidate.encode());
        preimage.push('\n');
    }
    sha256_hex(preimage.as_bytes())
}

/// Derive the exact candidate-by-scale data-row count.
const fn expected_row_count(workload_class: WorkloadClass, candidate_count: usize) -> usize {
    let variants = match workload_class {
        WorkloadClass::ExactAdapterMechanics => 1,
        WorkloadClass::SqliteCharacterization => candidate_count,
    };
    variants * 3
}

/// Project the complete invocation across workload variants, scales, and independent runs.
pub(crate) fn project_resources(
    actual_records: u64,
    payload_bytes: usize,
    checkpoint_bytes: usize,
    workload_class: WorkloadClass,
    pragma_candidates: &[SqlitePragmaCandidate],
    independent_runs: usize,
) -> Result<ResourceProjection, String> {
    let mut total = ResourceProjection::default();
    let variants = match workload_class {
        WorkloadClass::ExactAdapterMechanics => vec![None],
        WorkloadClass::SqliteCharacterization => {
            pragma_candidates.iter().map(Some).collect::<Vec<_>>()
        }
    };
    for candidate in variants {
        for scale in [Scale::ActualInput, Scale::TenX, Scale::HundredX] {
            let records = u128::from(actual_records)
                .checked_mul(u128::from(scale.multiplier()))
                .ok_or_else(|| "scaled record count overflow".to_owned())?;
            let runs = independent_runs as u128;
            let journal_writes = records
                .checked_mul(2)
                .and_then(|value| value.checked_mul(runs))
                .ok_or_else(|| "projected journal writes overflow".to_owned())?;
            let checkpoint_writes_per_run = match workload_class {
                WorkloadClass::ExactAdapterMechanics => 2,
                WorkloadClass::SqliteCharacterization => {
                    let candidate = candidate.expect("characterization variant has a candidate");
                    let groups = records.div_ceil(candidate.group_records.get() as u128);
                    groups
                        .saturating_sub(1)
                        .checked_div(u128::from(candidate.checkpoint_every_groups.get()))
                        .and_then(|value| value.checked_mul(2))
                        .and_then(|value| value.checked_add(1))
                        .ok_or_else(|| "projected checkpoint writes overflow".to_owned())?
                }
            };
            let checkpoint_writes = checkpoint_writes_per_run
                .checked_mul(runs)
                .ok_or_else(|| "projected checkpoint writes overflow".to_owned())?;
            let recovery_reads = match workload_class {
                WorkloadClass::ExactAdapterMechanics => runs
                    .checked_mul(2)
                    .ok_or_else(|| "projected recovery reads overflow".to_owned())?,
                WorkloadClass::SqliteCharacterization => runs,
            };
            let wal_checkpoints = match workload_class {
                WorkloadClass::ExactAdapterMechanics => 0,
                WorkloadClass::SqliteCharacterization => checkpoint_writes,
            };
            let operations = journal_writes
                .checked_add(checkpoint_writes)
                .and_then(|value| value.checked_add(wal_checkpoints))
                .and_then(|value| value.checked_add(recovery_reads))
                .ok_or_else(|| "projected operations overflow".to_owned())?;
            let journal_payload = journal_writes
                .checked_mul(payload_bytes as u128)
                .ok_or_else(|| "projected journal payload bytes overflow".to_owned())?;
            let checkpoint_payload = checkpoint_writes
                .checked_mul(checkpoint_bytes as u128)
                .ok_or_else(|| "projected checkpoint payload bytes overflow".to_owned())?;
            total = total.checked_add(ResourceProjection {
                independent_runs: runs,
                operations,
                payload_bytes: journal_payload
                    .checked_add(checkpoint_payload)
                    .ok_or_else(|| "projected payload bytes overflow".to_owned())?,
            })?;
        }
    }
    Ok(total)
}

/// Reject a projection that exceeds any explicit operator execution ceiling.
pub(crate) fn preflight_resources(
    projection: ResourceProjection,
    budget: OperatorResourceBudget,
) -> Result<(), String> {
    if projection.independent_runs > budget.maximum_independent_runs {
        return Err(format!(
            "projected independent runs {} exceed budget {}",
            projection.independent_runs, budget.maximum_independent_runs
        ));
    }
    if projection.operations > budget.maximum_operations {
        return Err(format!(
            "projected operations {} exceed budget {}",
            projection.operations, budget.maximum_operations
        ));
    }
    if projection.payload_bytes > budget.maximum_payload_bytes {
        return Err(format!(
            "projected payload bytes {} exceed budget {}",
            projection.payload_bytes, budget.maximum_payload_bytes
        ));
    }
    Ok(())
}

/// Parse one numeric value with a field-specific failure.
fn parse_number<T>(value: &str, label: &str) -> Result<T, String>
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    value
        .parse::<T>()
        .map_err(|error| format!("parse {label}: {error}"))
}

/// Parse exact `on`/`off` benchmark switches.
fn parse_switch(value: &str) -> Result<bool, String> {
    match value {
        "on" => Ok(true),
        "off" => Ok(false),
        _ => Err("switch must be on or off".to_owned()),
    }
}

/// Stable switch spelling.
pub(crate) const fn switch(value: bool) -> &'static str {
    if value {
        "on"
    } else {
        "off"
    }
}

/// Require one nonempty, control-free evidence coordinate.
pub(crate) fn required_coordinate(name: &str) -> String {
    let value = std::env::var(name).unwrap_or_else(|_| panic!("set {name}"));
    validate_coordinate(name, &value).unwrap_or_else(|error| panic!("{error}"));
    value
}

/// Validate a coordinate without rewriting its exact supplied value.
fn validate_coordinate(name: &str, value: &str) -> Result<(), String> {
    if value.is_empty() || value.len() > 1_024 || value.chars().any(char::is_control) {
        return Err(format!(
            "{name} must be a nonempty, control-free value of at most 1024 bytes"
        ));
    }
    Ok(())
}

/// Parse one required numeric environment value.
pub(crate) fn required_number<T>(name: &str) -> T
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    parse_number(&required_coordinate(name), name).unwrap_or_else(|error| panic!("{error}"))
}

/// Parse one strictly positive u128 budget member.
fn required_positive_number(name: &str) -> u128 {
    let value = required_number(name);
    assert!(value > 0, "{name} must be greater than zero");
    value
}

/// Require lowercase hexadecimal SHA-256 spelling.
fn require_sha256(value: &str, label: &str) {
    assert!(
        value.len() == 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
        "{label} must be exactly 64 lowercase hexadecimal bytes"
    );
}
