//! Direct `SQLite` characterization, isolated from the unadmitted adapter path.

use std::path::Path;
use std::time::{Duration, Instant};

use pale_core::sha256_hex;
use pale_ports::AuthorityByteClass;
use rusqlite::{params, Connection, OpenFlags, TransactionBehavior};

use super::input::{BenchmarkCase, CheckpointMode, Scale, SqlitePragmaCandidate, WorkloadClass};
use super::workload::{
    deterministic_payload, file_size, file_sizes, private_tempdir, CaseMeasurements,
    CheckpointStatus, CrashWriterRequest,
};
use crate::frame as production_frame;
use crate::{verify_embedded_storage_sources, ControlStoreKind};

/// Run one direct-SQLite append/checkpoint sample in an isolated database.
pub(crate) fn run_case_sample(case: &BenchmarkCase) -> CaseMeasurements {
    let directory = private_tempdir();
    let path = directory.path().join("control.sqlite3");
    let candidate = candidate(case);
    let mut connection = open_benchmark_database(&path, candidate);
    let payload = deterministic_payload(case.payload_bytes);
    let checkpoint_payload = deterministic_payload(case.checkpoint_bytes);
    create_workload_schema(&connection);
    let mut sample = CaseMeasurements::default();
    append_records(
        &mut connection,
        case,
        candidate,
        &payload,
        &checkpoint_payload,
        true,
        &mut sample,
    );
    sample.before_checkpoint.push(file_sizes(&path));
    sample
        .checkpoint_projection_commit
        .push(store_checkpoint_projection(
            &mut connection,
            case.records,
            &checkpoint_payload,
        ));
    let (elapsed, status) = run_wal_checkpoint(&connection, candidate.checkpoint_mode);
    sample.wal_checkpoint.push(elapsed);
    sample.checkpoint_statuses.push(status);
    sample.after_checkpoint.push(file_sizes(&path));
    close_connection(connection);
    sample
}

/// Create committed direct-SQLite state, then terminate without destructors.
pub(crate) fn run_crash_writer(request: &CrashWriterRequest) -> ! {
    let candidate = request
        .pragma_candidate
        .as_ref()
        .expect("SQLite characterization child has a pragma candidate");
    let path = request.root.join("control.sqlite3");
    let mut connection = open_benchmark_database(&path, candidate);
    let checkpoint_payload = deterministic_payload(request.checkpoint_bytes);
    create_workload_schema(&connection);
    let case = BenchmarkCase {
        pragma_candidate: Some(candidate.clone()),
        workload_class: WorkloadClass::SqliteCharacterization,
        scale: Scale::ActualInput,
        records: request.records,
        payload_bytes: request.payload_bytes,
        checkpoint_bytes: request.checkpoint_bytes,
        limits: request.limits,
    };
    let payload = deterministic_payload(request.payload_bytes);
    let mut discarded_timings = CaseMeasurements::default();
    append_records(
        &mut connection,
        &case,
        candidate,
        &payload,
        &checkpoint_payload,
        true,
        &mut discarded_timings,
    );
    std::hint::black_box(&connection);
    std::process::exit(0)
}

/// Reopen after process loss and validate one bounded direct-SQLite view.
pub(crate) fn run_recovery_sample(case: &BenchmarkCase, root: &Path) -> CaseMeasurements {
    let path = root.join("control.sqlite3");
    let mut sample = CaseMeasurements::default();
    sample.recovery_tail.push(file_size(&wal_path(&path)));
    let started = Instant::now();
    let connection = Connection::open_with_flags(
        &path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .expect("reopen process-loss SQLite characterization database");
    let checkpoint_frontier = newest_valid_checkpoint(&connection, case);
    validate_bounded_tail(&connection, case, checkpoint_frontier);
    sample.recovery.push(started.elapsed());
    close_connection(connection);
    sample
}

/// Open and configure one isolated database from a closed pragma candidate.
fn open_benchmark_database(path: &Path, candidate: &SqlitePragmaCandidate) -> Connection {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .expect("open isolated SQLite characterization database");
    connection
        .pragma_update(None, "foreign_keys", true)
        .expect("enable foreign keys");
    connection
        .pragma_update(None, "trusted_schema", false)
        .expect("disable trusted schema");
    let journal_mode: String = connection
        .query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))
        .expect("select WAL mode");
    assert_eq!(journal_mode.to_ascii_lowercase(), "wal");
    connection
        .pragma_update(None, "synchronous", candidate.synchronous.pragma_value())
        .expect("apply SQLite synchronous candidate");
    connection
        .pragma_update(None, "fullfsync", candidate.fullfsync)
        .expect("apply SQLite fullfsync candidate");
    connection
        .pragma_update(None, "checkpoint_fullfsync", candidate.checkpoint_fullfsync)
        .expect("apply SQLite checkpoint_fullfsync candidate");
    connection
        .pragma_update(
            None,
            "wal_autocheckpoint",
            candidate.wal_autocheckpoint_frames,
        )
        .expect("apply SQLite WAL autocheckpoint candidate");
    assert_observed_profile(&connection, candidate);
    connection
}

/// Prove `SQLite` observed every requested candidate before recording timings.
fn assert_observed_profile(connection: &Connection, candidate: &SqlitePragmaCandidate) {
    assert_eq!(
        pragma_i64(connection, "synchronous"),
        candidate.synchronous.pragma_value()
    );
    assert_eq!(
        pragma_i64(connection, "fullfsync"),
        i64::from(candidate.fullfsync)
    );
    assert_eq!(
        pragma_i64(connection, "checkpoint_fullfsync"),
        i64::from(candidate.checkpoint_fullfsync)
    );
    assert_eq!(
        pragma_i64(connection, "wal_autocheckpoint"),
        i64::from(candidate.wal_autocheckpoint_frames)
    );
}

/// Query one integer-valued pragma from a fixed internal name.
pub(crate) fn pragma_i64(connection: &Connection, name: &str) -> i64 {
    let statement = format!("PRAGMA {name}");
    connection
        .query_row(&statement, [], |row| row.get(0))
        .unwrap_or_else(|error| panic!("query {statement}: {error}"))
}

/// Create the exact verified deployment schema without claiming adapter use.
fn create_workload_schema(connection: &Connection) {
    let sources = verify_embedded_storage_sources().expect("verify production storage source");
    connection
        .execute_batch(
            sources
                .store(ControlStoreKind::DeploymentControl)
                .migration_head()
                .sql(),
        )
        .expect("apply verified deployment-control migration");
}

/// Commit deterministic frames in bounded direct-SQLite durability groups.
fn append_records(
    connection: &mut Connection,
    case: &BenchmarkCase,
    candidate: &SqlitePragmaCandidate,
    payload: &[u8],
    checkpoint_payload: &[u8],
    enable_checkpoints: bool,
    measurements: &mut CaseMeasurements,
) {
    let physical_frame = production_frame::encode(payload, AuthorityByteClass::JournalEntry)
        .expect("encode production journal frame");
    let group_records = u64::try_from(candidate.group_records.get())
        .expect("group size fits the u64 journal domain");
    let mut next_sequence = 1_u64;
    let mut group_ordinal = 0_u64;
    while next_sequence <= case.records {
        group_ordinal += 1;
        let last_sequence = case
            .records
            .min(next_sequence.saturating_add(group_records - 1));
        let checksum_started = Instant::now();
        for _ in next_sequence..=last_sequence {
            std::hint::black_box(sha256_hex(std::hint::black_box(payload)));
        }
        let checksum_elapsed = checksum_started.elapsed().as_nanos();
        let group_count = u128::from(last_sequence - next_sequence + 1);
        let per_entry = u64::try_from(checksum_elapsed / group_count).unwrap_or(u64::MAX);
        measurements
            .checksum_reference
            .push(Duration::from_nanos(per_entry));

        let group_started = Instant::now();
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .expect("begin SQLite characterization journal transaction");
        let append_started = Instant::now();
        let mut statement = transaction
            .prepare(
                "INSERT INTO pale_deployment_authority_journal \
                 (journal_sequence, physical_frame) VALUES (?1, ?2)",
            )
            .expect("prepare direct SQLite journal append");
        for sequence in next_sequence..=last_sequence {
            statement
                .execute(params![
                    sqlite_integer(sequence, "journal sequence"),
                    &physical_frame
                ])
                .expect("append direct SQLite journal frame");
        }
        drop(statement);
        transaction
            .execute(
                "UPDATE pale_deployment_authority_head \
                 SET journal_sequence = ?1 WHERE singleton = 1",
                [sqlite_integer(last_sequence, "journal head")],
            )
            .expect("advance direct SQLite journal head");
        measurements.append.push(append_started.elapsed());
        let commit_started = Instant::now();
        transaction
            .commit()
            .expect("commit direct SQLite durability group");
        measurements.commit.push(commit_started.elapsed());
        measurements.group_release.push(group_started.elapsed());
        next_sequence = last_sequence + 1;

        let cadence = u64::from(candidate.checkpoint_every_groups.get());
        if enable_checkpoints
            && next_sequence <= case.records
            && group_ordinal.is_multiple_of(cadence)
        {
            measurements
                .checkpoint_projection_commit
                .push(store_checkpoint_projection(
                    connection,
                    last_sequence,
                    checkpoint_payload,
                ));
            let (elapsed, status) = run_wal_checkpoint(connection, candidate.checkpoint_mode);
            measurements.wal_checkpoint.push(elapsed);
            measurements.checkpoint_statuses.push(status);
        }
    }
}

/// Commit one checkpoint projection without including WAL checkpoint time.
fn store_checkpoint_projection(
    connection: &mut Connection,
    through_sequence: u64,
    checkpoint_payload: &[u8],
) -> Duration {
    let started = Instant::now();
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .expect("begin direct SQLite checkpoint projection transaction");
    let physical_frame =
        production_frame::encode(checkpoint_payload, AuthorityByteClass::RecoveryCheckpoint)
            .expect("encode production checkpoint frame");
    transaction
        .execute(
            "INSERT INTO pale_deployment_recovery_checkpoint \
             (through_journal_sequence, physical_frame) VALUES (?1, ?2)",
            params![
                sqlite_integer(through_sequence, "checkpoint frontier"),
                physical_frame
            ],
        )
        .expect("store direct SQLite checkpoint projection");
    transaction
        .commit()
        .expect("commit direct SQLite checkpoint projection");
    started.elapsed()
}

/// Execute and time only one explicit WAL checkpoint operation.
fn run_wal_checkpoint(
    connection: &Connection,
    mode: CheckpointMode,
) -> (Duration, CheckpointStatus) {
    let started = Instant::now();
    let statement = format!("PRAGMA wal_checkpoint({})", mode.as_str());
    let status = connection
        .query_row(&statement, [], |row| {
            Ok(CheckpointStatus {
                busy: row.get(0)?,
                log_frames: row.get(1)?,
                checkpointed_frames: row.get(2)?,
            })
        })
        .unwrap_or_else(|error| panic!("execute {statement}: {error}"));
    (started.elapsed(), status)
}

/// Verify every checkpoint frame and return the newest bounded frontier.
fn newest_valid_checkpoint(connection: &Connection, case: &BenchmarkCase) -> u64 {
    let limit = sqlite_integer(
        u64::try_from(case.limits.maximum_checkpoint_candidates())
            .expect("checkpoint-candidate limit fits u64"),
        "maximum checkpoint candidates",
    );
    let mut statement = connection
        .prepare(
            "SELECT through_journal_sequence, physical_frame \
             FROM pale_deployment_recovery_checkpoint \
             ORDER BY through_journal_sequence DESC LIMIT ?1",
        )
        .expect("prepare bounded direct SQLite checkpoint read");
    let candidates = statement
        .query_map([limit], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?))
        })
        .expect("read bounded direct SQLite checkpoint candidates")
        .collect::<Result<Vec<_>, _>>()
        .expect("decode bounded direct SQLite checkpoint candidates");
    for (sequence, frame) in &candidates {
        assert!(*sequence > 0, "checkpoint frontier is positive");
        production_frame::decode(
            frame,
            case.limits.maximum_checkpoint_bytes(),
            AuthorityByteClass::RecoveryCheckpoint,
        )
        .expect("decode bounded production checkpoint frame");
    }
    candidates.first().map_or(0, |candidate| {
        u64::try_from(candidate.0).expect("checkpoint frontier is positive")
    })
}

/// Verify the exact ascending tail and every frame within its hard bound.
fn validate_bounded_tail(connection: &Connection, case: &BenchmarkCase, after: u64) {
    let read_limit = case
        .limits
        .maximum_tail_entries()
        .checked_add(1)
        .expect("tail-entry probe limit overflow");
    let after = sqlite_integer(after, "checkpoint frontier");
    let read_limit = sqlite_integer(
        u64::try_from(read_limit).expect("tail-entry probe limit fits u64"),
        "maximum tail entries probe",
    );
    let mut statement = connection
        .prepare(
            "SELECT journal_sequence, physical_frame \
             FROM pale_deployment_authority_journal \
             WHERE journal_sequence > ?1 ORDER BY journal_sequence LIMIT ?2",
        )
        .expect("prepare bounded direct SQLite recovery-tail read");
    let rows = statement
        .query_map(params![after, read_limit], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?))
        })
        .expect("read bounded direct SQLite recovery tail")
        .collect::<Result<Vec<_>, _>>()
        .expect("decode bounded direct SQLite recovery tail");
    assert!(
        rows.len() <= case.limits.maximum_tail_entries(),
        "recovery tail exceeds maximum_tail_entries"
    );
    let mut expected = u64::try_from(after)
        .expect("checkpoint frontier is nonnegative")
        .checked_add(1)
        .expect("tail sequence overflow");
    for (sequence, frame) in rows {
        let sequence = u64::try_from(sequence).expect("journal sequence is positive");
        assert_eq!(sequence, expected, "recovery tail has a sequence gap");
        production_frame::decode(
            &frame,
            case.limits.maximum_entry_bytes(),
            AuthorityByteClass::JournalEntry,
        )
        .expect("decode bounded production journal frame");
        expected = expected.checked_add(1).expect("tail sequence overflow");
    }
    assert_eq!(
        expected.saturating_sub(1),
        case.records,
        "bounded checkpoint plus tail does not reach the committed head"
    );
}

/// Borrow the required direct-SQLite candidate from one resolved case.
fn candidate(case: &BenchmarkCase) -> &SqlitePragmaCandidate {
    case.pragma_candidate
        .as_ref()
        .expect("SQLite characterization case has a pragma candidate")
}

/// Convert one bounded unsigned identity to `SQLite`'s signed integer domain.
fn sqlite_integer(value: u64, label: &str) -> i64 {
    i64::try_from(value).unwrap_or_else(|_| panic!("{label} exceeds SQLite INTEGER"))
}

/// Construct the `SQLite` WAL sidecar path.
fn wal_path(path: &Path) -> std::path::PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push("-wal");
    value.into()
}

/// Close a connection and surface a pending `SQLite` close failure.
fn close_connection(connection: Connection) {
    connection
        .close()
        .unwrap_or_else(|(_, error)| panic!("close SQLite characterization database: {error}"));
}
