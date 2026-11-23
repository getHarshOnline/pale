//! Shared workload dispatch, measurements, deterministic inputs, and file observations.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use pale_ports::AuthorityRecoveryReadLimits;

use super::adapter_workload;
use super::input::{
    required_coordinate, required_number, BenchmarkCase, SqlitePragmaCandidate, WorkloadClass,
};
use super::memory::MemoryEvidence;
use super::sqlite_characterization;

/// Fixed deterministic seed for benchmark payload construction.
const PAYLOAD_SEED: u64 = 0x4a41_5256_4953_0009;

/// Aggregated observations for one workload/scale pair.
#[derive(Debug, Default)]
pub(crate) struct CaseMeasurements {
    /// Benchmark-only memory and explicit-copy observations.
    pub(crate) memory: MemoryEvidence,
    /// Independently isolated regular-plus-process-loss runs.
    pub(crate) independent_runs: usize,
    /// SHA-256 reference hash-and-hex latency per entry.
    pub(crate) checksum_reference: Vec<Duration>,
    /// Pooled append operation latency.
    pub(crate) append: Vec<Duration>,
    /// Direct-SQLite transaction-commit return latency.
    pub(crate) commit: Vec<Duration>,
    /// Direct-SQLite time from transaction begin through commit return.
    pub(crate) group_release: Vec<Duration>,
    /// Checkpoint-projection port-call or transaction-commit latency.
    pub(crate) checkpoint_projection_commit: Vec<Duration>,
    /// Explicit `PRAGMA wal_checkpoint` latency, direct-SQLite lane only.
    pub(crate) wal_checkpoint: Vec<Duration>,
    /// Process-loss reopen plus bounded checkpoint-and-tail validation latency.
    pub(crate) recovery: Vec<Duration>,
    /// Physical sizes captured immediately before final explicit checkpoint.
    pub(crate) before_checkpoint: Vec<FileSizes>,
    /// Physical sizes captured after final explicit checkpoint.
    pub(crate) after_checkpoint: Vec<FileSizes>,
    /// WAL sizes left by process-loss children before recovery open.
    pub(crate) recovery_tail: Vec<u64>,
    /// Every direct-SQLite checkpoint busy/log/completed tuple.
    pub(crate) checkpoint_statuses: Vec<CheckpointStatus>,
}

impl CaseMeasurements {
    /// Merge the regular and recovery halves of one independently isolated run.
    pub(crate) fn record_independent_run(&mut self, regular: Self, recovery: Self) {
        assert_eq!(
            recovery.recovery.len(),
            1,
            "one recovery observation closes each independent run"
        );
        self.independent_runs = self
            .independent_runs
            .checked_add(1)
            .expect("independent run count overflow");
        self.absorb(regular);
        self.absorb(recovery);
    }

    /// Pool operation observations without changing the independent-run count.
    fn absorb(&mut self, sample: Self) {
        self.append.extend(sample.append);
        self.checksum_reference.extend(sample.checksum_reference);
        self.commit.extend(sample.commit);
        self.group_release.extend(sample.group_release);
        self.checkpoint_projection_commit
            .extend(sample.checkpoint_projection_commit);
        self.wal_checkpoint.extend(sample.wal_checkpoint);
        self.recovery.extend(sample.recovery);
        self.before_checkpoint.extend(sample.before_checkpoint);
        self.after_checkpoint.extend(sample.after_checkpoint);
        self.recovery_tail.extend(sample.recovery_tail);
        self.checkpoint_statuses.extend(sample.checkpoint_statuses);
    }
}

/// Crash-child input reconstructed from explicit environment members.
#[derive(Debug)]
pub(crate) struct CrashWriterRequest {
    /// Isolated root owned by this child invocation.
    pub(crate) root: PathBuf,
    /// Direct-SQLite candidate, absent for exact adapter mechanics.
    pub(crate) pragma_candidate: Option<SqlitePragmaCandidate>,
    /// Explicit unadmitted-adapter or direct-SQLite class.
    pub(crate) workload_class: WorkloadClass,
    /// Journal records committed before process loss.
    pub(crate) records: u64,
    /// Bytes per deterministic payload.
    pub(crate) payload_bytes: usize,
    /// Bytes per deterministic checkpoint projection.
    pub(crate) checkpoint_bytes: usize,
    /// Existing port-owned recovery bounds.
    pub(crate) limits: AuthorityRecoveryReadLimits,
}

/// Decode one crash-writer request supplied by the parent benchmark process.
pub(crate) fn crash_writer_request() -> CrashWriterRequest {
    let workload_class = WorkloadClass::parse(&required_coordinate(
        "PALE_SQLITE_BENCH_CHILD_WORKLOAD_CLASS",
    ))
    .unwrap_or_else(|error| panic!("{error}"));
    let pragma_candidate = match workload_class {
        WorkloadClass::ExactAdapterMechanics => None,
        WorkloadClass::SqliteCharacterization => Some(
            SqlitePragmaCandidate::parse(&required_coordinate("PALE_SQLITE_BENCH_CHILD_PROFILE"))
                .unwrap_or_else(|error| panic!("{error}")),
        ),
    };
    CrashWriterRequest {
        root: PathBuf::from(required_coordinate("PALE_SQLITE_BENCH_CHILD_ROOT")),
        pragma_candidate,
        workload_class,
        records: required_number("PALE_SQLITE_BENCH_CHILD_RECORDS"),
        payload_bytes: required_number("PALE_SQLITE_BENCH_CHILD_PAYLOAD_BYTES"),
        checkpoint_bytes: required_number("PALE_SQLITE_BENCH_CHILD_CHECKPOINT_BYTES"),
        limits: AuthorityRecoveryReadLimits::new(
            required_number("PALE_SQLITE_BENCH_CHILD_MAXIMUM_ENTRY_BYTES"),
            required_number("PALE_SQLITE_BENCH_CHILD_MAXIMUM_CHECKPOINT_BYTES"),
            required_number("PALE_SQLITE_BENCH_CHILD_MAXIMUM_CHECKPOINT_CANDIDATES"),
            required_number("PALE_SQLITE_BENCH_CHILD_MAXIMUM_TAIL_ENTRIES"),
        )
        .expect("crash-writer recovery limits are nonzero"),
    }
}

/// Run one regular sample through the selected, explicitly classified lane.
pub(crate) fn run_case_sample(case: &BenchmarkCase) -> CaseMeasurements {
    match case.workload_class {
        WorkloadClass::ExactAdapterMechanics => adapter_workload::run_case_sample(case),
        WorkloadClass::SqliteCharacterization => sqlite_characterization::run_case_sample(case),
    }
}

/// Create committed state and terminate without running connection destructors.
pub(crate) fn run_crash_writer(request: &CrashWriterRequest) -> ! {
    match request.workload_class {
        WorkloadClass::ExactAdapterMechanics => adapter_workload::run_crash_writer(request),
        WorkloadClass::SqliteCharacterization => sqlite_characterization::run_crash_writer(request),
    }
}

/// Reopen the process-loss store and validate a bounded checkpoint-plus-tail view.
pub(crate) fn run_recovery_sample(case: &BenchmarkCase, root: &Path) -> CaseMeasurements {
    match case.workload_class {
        WorkloadClass::ExactAdapterMechanics => adapter_workload::run_recovery_sample(case, root),
        WorkloadClass::SqliteCharacterization => {
            sqlite_characterization::run_recovery_sample(case, root)
        }
    }
}

/// `SQLite` checkpoint counters from one explicit direct-SQLite checkpoint.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CheckpointStatus {
    /// Nonzero when a connection prevented the requested checkpoint work.
    pub(crate) busy: i64,
    /// WAL frames observed by `SQLite`.
    pub(crate) log_frames: i64,
    /// WAL frames checkpointed by `SQLite`.
    pub(crate) checkpointed_frames: i64,
}

/// Main/WAL/shared-memory byte coordinates at one boundary.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FileSizes {
    /// Main database bytes.
    pub(crate) main: u64,
    /// Write-ahead-log bytes.
    pub(crate) wal: u64,
    /// Shared-memory index bytes.
    pub(crate) shm: u64,
}

/// Read all `SQLite` file sizes without treating them as durability proof.
pub(crate) fn file_sizes(path: &Path) -> FileSizes {
    FileSizes {
        main: file_size(path),
        wal: file_size(&wal_path(path)),
        shm: file_size(&sidecar_path(path, "-shm")),
    }
}

/// Return zero for an absent sidecar and fail for every other metadata error.
pub(crate) fn file_size(path: &Path) -> u64 {
    match fs::metadata(path) {
        Ok(metadata) => metadata.len(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => 0,
        Err(error) => panic!("inspect benchmark file {}: {error}", path.display()),
    }
}

/// Construct the `SQLite` WAL sidecar path by its documented filename suffix.
fn wal_path(path: &Path) -> PathBuf {
    sidecar_path(path, "-wal")
}

/// Construct a `SQLite` sidecar path without interpreting caller text as SQL.
fn sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
}

/// Build reproducible, non-compressing frame bytes from one fixed seed.
pub(crate) fn deterministic_payload(bytes: usize) -> Vec<u8> {
    let mut state = PAYLOAD_SEED;
    (0..bytes)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state.to_le_bytes()[0]
        })
        .collect()
}

/// Create an owner-private temporary root without retaining a symlinked prefix.
pub(crate) fn private_tempdir() -> tempfile::TempDir {
    let system_temporary = std::fs::canonicalize(std::env::temp_dir())
        .expect("canonicalize system temporary directory");
    let directory = tempfile::tempdir_in(system_temporary)
        .expect("create isolated benchmark directory under canonical temporary root");
    set_private_permissions(directory.path());
    directory
}

/// Apply the store root's required owner-only permission boundary.
pub(crate) fn set_private_permissions(root: &Path) {
    std::fs::set_permissions(root, std::fs::Permissions::from_mode(0o700))
        .expect("apply private benchmark root permissions");
}
