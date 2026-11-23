//! Assembled resident behavior over the exact unadmitted `SQLite` adapter.
//!
//! This benchmark combines real application compilation and contextual
//! generation admission with the crate-private deployment-store candidate. Its
//! workspace, runtime probe, materialization observations, and session population
//! are deterministic fixtures. Rows are therefore local characterization only:
//! they do not select a storage profile, resident capacity, acceptance budget,
//! release, native process, or effect policy.

use std::collections::BTreeSet;
use std::hint::black_box;
use std::io::{self, Write};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use pale_application::resident::{
    RecoveryReferenceKnowledge, ResidentApplication, ResidentColdOpenInput,
    ResidentColdOpenOutcome, ResidentCommitReceipt, ResidentGenerationInputSource,
    ResidentReconstructionParameters, ResidentWakeDisposition,
};
use pale_core::records::{JournalSequenceV1, SessionAdmissionLeaseV1, WakeEpochV1};
use pale_core::sha256_hex;
use pale_kernel::WakeConsumerCursor;
use pale_ports::AuthorityRecoveryReadLimits;

use super::adapter_workload::{initialize_deployment_store, AdapterFixture};
use super::statistics::Distribution;
use super::workload::{file_sizes, private_tempdir, FileSizes};
use crate::authority::DeploymentControlSqliteStore;

use crate::resident_fixture;

use resident_fixture::shared_resident::{
    admit_and_activate, live_workspace, reconstruction_input, resolver_for_active,
    session_lease_for_active, ActiveResidentFixture, FixedInputSource, RecoveryResolver,
};

/// Full-run independent observation floor required for p99.
const FULL_SAMPLES: usize = 201;
/// Compile-and-execute smoke observations; never characterization evidence.
const SMOKE_SAMPLES: usize = 5;
/// Exact operations emitted for every selected synthetic population.
const OPERATIONS_PER_SCALE: usize = 4;
/// Largest session population accepted before any workload allocation.
const MAXIMUM_SESSION_POPULATION: usize = 100_000;
/// Maximum UTF-8 bytes accepted in one caller-supplied evidence coordinate.
const MAX_COORDINATE_BYTES: usize = 512;
/// Terminal marker binding every preceding TSV byte.
const COMPLETION_MARKER: &str = "#pale_resident_application_report_complete_v1";

/// Closed assembled operations with non-interchangeable timing scopes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Operation {
    Pin,
    Drain,
    ReconstructCheckpointTail,
    ReconstructCheckpointHead,
}

impl Operation {
    const ALL: [Self; OPERATIONS_PER_SCALE] = [
        Self::Pin,
        Self::Drain,
        Self::ReconstructCheckpointTail,
        Self::ReconstructCheckpointHead,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::Pin => "pin_session",
            Self::Drain => "drain_session",
            Self::ReconstructCheckpointTail => "cold_reconstruct_checkpoint_then_pin_tail",
            Self::ReconstructCheckpointHead => "cold_reconstruct_checkpoint_including_pins",
        }
    }

    const fn timing_scope(self) -> &'static str {
        match self {
            Self::Pin | Self::Drain => "prepared_resident_and_lease_retained_result_retained",
            Self::ReconstructCheckpointTail | Self::ReconstructCheckpointHead => {
                "opened_store_and_owned_inputs_reconstruction_result_retained"
            }
        }
    }
}

/// Deterministic benchmark scale, not an owner-qualified production universe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Scale {
    label: &'static str,
    multiplier: usize,
}

const FULL_SCALES: [Scale; 3] = [
    Scale {
        label: "synthetic_1x",
        multiplier: 1,
    },
    Scale {
        label: "synthetic_10x",
        multiplier: 10,
    },
    Scale {
        label: "synthetic_100x",
        multiplier: 100,
    },
];
const SMOKE_SCALES: [Scale; 1] = [Scale {
    label: "smoke_synthetic_1x",
    multiplier: 1,
}];

/// Explicit bounded physical and resident reconstruction inputs.
#[derive(Debug, Clone, Copy)]
struct Bounds {
    maximum_entry_bytes: usize,
    maximum_checkpoint_bytes: usize,
    maximum_checkpoint_candidates: usize,
    maximum_tail_entries: usize,
    maximum_total_transitions: usize,
}

impl Bounds {
    fn recovery_limits(self) -> AuthorityRecoveryReadLimits {
        AuthorityRecoveryReadLimits::new(
            self.maximum_entry_bytes,
            self.maximum_checkpoint_bytes,
            self.maximum_checkpoint_candidates,
            self.maximum_tail_entries,
        )
        .expect("validated resident benchmark recovery limits")
    }
}

/// One explicit invocation after all workload arithmetic is preflighted.
struct Configuration {
    smoke: bool,
    samples: usize,
    baseline_sessions: usize,
    scales: &'static [Scale],
    bounds: Bounds,
    expected_rows: usize,
    projected_total_transitions: usize,
    source_coordinate: String,
    run_coordinate: String,
    hardware: String,
}

struct EvidenceCoordinates {
    source: String,
    run: String,
    hardware: String,
}

impl Configuration {
    /// Resolve an explicit benchmark mode, or skip ordinary target discovery.
    fn from_environment() -> Option<Self> {
        let smoke = flag("PALE_RESIDENT_BENCH_SMOKE");
        let characterize = flag("PALE_BENCH_RUN");
        assert!(
            !(smoke && characterize),
            "select either resident smoke validation or full characterization"
        );
        if !smoke && !characterize {
            return None;
        }
        let samples = if smoke { SMOKE_SAMPLES } else { FULL_SAMPLES };
        let scales: &'static [Scale] = if smoke { &SMOKE_SCALES } else { &FULL_SCALES };
        let baseline_sessions = bounded_number("PALE_RESIDENT_BENCH_BASELINE_SESSIONS", smoke, 1);
        let bounds = Bounds {
            maximum_entry_bytes: bounded_number(
                "PALE_RESIDENT_BENCH_MAXIMUM_ENTRY_BYTES",
                smoke,
                1 << 20,
            ),
            maximum_checkpoint_bytes: bounded_number(
                "PALE_RESIDENT_BENCH_MAXIMUM_CHECKPOINT_BYTES",
                smoke,
                1 << 24,
            ),
            maximum_checkpoint_candidates: bounded_number(
                "PALE_RESIDENT_BENCH_MAXIMUM_CHECKPOINT_CANDIDATES",
                smoke,
                8,
            ),
            maximum_tail_entries: bounded_number(
                "PALE_RESIDENT_BENCH_MAXIMUM_TAIL_ENTRIES",
                smoke,
                128,
            ),
            maximum_total_transitions: bounded_number(
                "PALE_RESIDENT_BENCH_MAXIMUM_TOTAL_TRANSITIONS",
                smoke,
                256,
            ),
        };
        Some(Self::preflight(
            smoke,
            samples,
            baseline_sessions,
            scales,
            bounds,
            EvidenceCoordinates {
                source: coordinate(
                    "PALE_RESIDENT_BENCH_SOURCE_COORDINATE",
                    smoke,
                    "smoke-unsealed-worktree",
                ),
                run: coordinate(
                    "PALE_RESIDENT_BENCH_RUN_COORDINATE",
                    smoke,
                    "smoke-validation",
                ),
                hardware: coordinate(
                    "PALE_RESIDENT_BENCH_HARDWARE",
                    smoke,
                    "unspecified-smoke-host",
                ),
            },
        ))
    }

    const fn evidence_class(&self) -> &'static str {
        if self.smoke {
            "smoke_validation"
        } else {
            "local_synthetic_fixture_characterization"
        }
    }

    fn preflight(
        smoke: bool,
        samples: usize,
        baseline_sessions: usize,
        scales: &'static [Scale],
        bounds: Bounds,
        coordinates: EvidenceCoordinates,
    ) -> Self {
        assert!(samples > 0, "independent sample count must be positive");
        assert!(
            baseline_sessions > 0,
            "baseline session count must be positive"
        );
        assert!(
            !scales.is_empty(),
            "at least one resident scale is required"
        );
        assert!(
            bounds.maximum_total_transitions > 0,
            "transition budget must be positive"
        );
        let expected_rows = scales
            .len()
            .checked_mul(OPERATIONS_PER_SCALE)
            .expect("resident benchmark row count overflow");
        let maximum_sessions = scaled_sessions(
            baseline_sessions,
            scales.last().expect("nonempty resident scales").multiplier,
        );
        assert!(
            maximum_sessions <= MAXIMUM_SESSION_POPULATION,
            "largest resident session population exceeds harness ceiling"
        );
        let _ = maximum_sessions
            .checked_add(4)
            .expect("resident and wake capacities overflow");
        assert!(
            bounds.maximum_tail_entries >= maximum_sessions,
            "tail-entry bound must hold the largest checkpoint-before-pins tail"
        );
        let projected_total_transitions = projected_transitions(baseline_sessions, scales, samples);
        assert!(
            projected_total_transitions <= bounds.maximum_total_transitions,
            "resident benchmark transition projection exceeds caller budget"
        );
        let _ = bounds.recovery_limits();
        Self {
            smoke,
            samples,
            baseline_sessions,
            scales,
            bounds,
            expected_rows,
            projected_total_transitions,
            source_coordinate: coordinates.source,
            run_coordinate: coordinates.run,
            hardware: coordinates.hardware,
        }
    }
}

/// One observation after behavioral assertions have accepted the result.
struct Sample {
    elapsed: Duration,
    sizes: FileSizes,
    journal_head: u64,
    retained_sessions: usize,
}

#[derive(Default)]
struct Measurements {
    elapsed: Vec<Duration>,
    main_bytes: Vec<u64>,
    wal_bytes: Vec<u64>,
    shm_bytes: Vec<u64>,
    journal_heads: Vec<u64>,
    retained_sessions: Vec<u64>,
}

impl Measurements {
    fn push(&mut self, sample: Sample) {
        self.elapsed.push(sample.elapsed);
        self.main_bytes.push(sample.sizes.main);
        self.wal_bytes.push(sample.sizes.wal);
        self.shm_bytes.push(sample.sizes.shm);
        self.journal_heads.push(sample.journal_head);
        self.retained_sessions.push(
            u64::try_from(sample.retained_sessions).expect("session count fits report domain"),
        );
    }
}

/// Run the explicit assembled benchmark and emit only a complete sealed report.
pub(crate) fn run() -> ExitCode {
    let Some(configuration) = Configuration::from_environment() else {
        eprintln!(
            "resident benchmark skipped; set PALE_BENCH_RUN=1 or PALE_RESIDENT_BENCH_SMOKE=1"
        );
        return ExitCode::SUCCESS;
    };
    let (workspace, probe) = live_workspace();
    let mut rows = Vec::new();
    rows.try_reserve_exact(configuration.expected_rows)
        .expect("validated resident report row count can be retained");
    for scale in configuration.scales {
        let sessions = scaled_sessions(configuration.baseline_sessions, scale.multiplier);
        for operation in Operation::ALL {
            let mut measurements = Measurements::default();
            for _ in 0..configuration.samples {
                measurements.push(run_sample(
                    operation,
                    sessions,
                    &configuration,
                    &workspace,
                    &probe,
                ));
            }
            rows.push(render_row(
                rows.len() + 1,
                operation,
                *scale,
                sessions,
                &configuration,
                &measurements,
            ));
        }
    }
    assert_eq!(rows.len(), configuration.expected_rows);
    let mut payload = report_header().to_owned();
    payload.push('\n');
    for row in rows {
        payload.push_str(&row);
        payload.push('\n');
    }
    let digest = sha256_hex(payload.as_bytes());
    let complete = format!(
        "{payload}{COMPLETION_MARKER}\t{}\t{}\t{digest}\n",
        configuration.expected_rows, configuration.projected_total_transitions
    );
    let mut stdout = io::stdout().lock();
    stdout
        .write_all(complete.as_bytes())
        .and_then(|()| stdout.flush())
        .expect("write complete assembled resident benchmark report");
    ExitCode::SUCCESS
}

fn run_sample(
    operation: Operation,
    sessions: usize,
    configuration: &Configuration,
    workspace: &resident_fixture::shared_resident::SyntheticWorkspace,
    probe: &resident_fixture::shared_resident::RuntimeProbe,
) -> Sample {
    match operation {
        Operation::Pin => run_pin_sample(sessions, configuration, workspace, probe),
        Operation::Drain => run_drain_sample(sessions, configuration, workspace, probe),
        Operation::ReconstructCheckpointTail => {
            run_reconstruction_sample(sessions, configuration, workspace, probe, false)
        }
        Operation::ReconstructCheckpointHead => {
            run_reconstruction_sample(sessions, configuration, workspace, probe, true)
        }
    }
}

fn run_pin_sample(
    sessions: usize,
    configuration: &Configuration,
    workspace: &resident_fixture::shared_resident::SyntheticWorkspace,
    probe: &resident_fixture::shared_resident::RuntimeProbe,
) -> Sample {
    with_active_resident(
        sessions,
        configuration,
        workspace,
        probe,
        |store, fixture, resident| {
            for index in 0..sessions.saturating_sub(1) {
                let lease = lease(fixture, index);
                let _ = resident
                    .pin_session(&lease)
                    .expect("prepare resident pin population");
            }
            let lease = lease(fixture, sessions - 1);
            let prior_head = resident
                .journal_head()
                .expect("active resident has a durable head");
            let prior_wake = resident.wake_cursor();
            let started = Instant::now();
            let receipt = black_box(
                resident
                    .pin_session(&lease)
                    .expect("timed session pin succeeds"),
            );
            let elapsed = started.elapsed();
            black_box(&receipt);
            let expected_head = next_sequence(prior_head);
            assert_commit_publication(resident, &receipt, prior_wake, expected_head);
            let pins = resident
                .session_pins()
                .expect("pin projection remains coherent");
            assert_eq!(pins.len(), sessions);
            let selected = pins
                .get(lease.session_admission_id())
                .expect("selected session entered the pin projection");
            assert_eq!(
                &selected.lease_digest,
                lease.session_admission_lease_digest()
            );
            assert_eq!(
                &selected.generation,
                fixture.generation.compiled_generation_digest()
            );
            let retaining = resident
                .sessions_retaining_generation(fixture.generation.compiled_generation_digest())
                .expect("reverse pin projection remains coherent");
            assert_eq!(retaining.len(), sessions);
            assert!(retaining.contains(lease.session_admission_id()));
            Sample {
                elapsed,
                sizes: file_sizes(&store.location.path()),
                journal_head: expected_head.get(),
                retained_sessions: pins.len(),
            }
        },
    )
}

fn run_drain_sample(
    sessions: usize,
    configuration: &Configuration,
    workspace: &resident_fixture::shared_resident::SyntheticWorkspace,
    probe: &resident_fixture::shared_resident::RuntimeProbe,
) -> Sample {
    with_active_resident(
        sessions,
        configuration,
        workspace,
        probe,
        |store, fixture, resident| {
            let mut leases = Vec::new();
            leases
                .try_reserve_exact(sessions)
                .expect("validated drain population can be retained");
            for index in 0..sessions {
                let lease = lease(fixture, index);
                let _ = resident
                    .pin_session(&lease)
                    .expect("prepare resident drain population");
                leases.push(lease);
            }
            let selected = &leases[sessions / 2];
            let prior_head = resident
                .journal_head()
                .expect("active resident has a durable head");
            let prior_wake = resident.wake_cursor();
            let started = Instant::now();
            let receipt = black_box(
                resident
                    .drain_session(
                        selected.session_admission_id(),
                        selected.session_admission_lease_digest(),
                    )
                    .expect("timed session drain succeeds"),
            );
            let elapsed = started.elapsed();
            black_box(&receipt);
            let expected_head = next_sequence(prior_head);
            assert_commit_publication(resident, &receipt, prior_wake, expected_head);
            let pins = resident
                .session_pins()
                .expect("drain projection remains coherent");
            assert_eq!(pins.len(), sessions - 1);
            assert!(!pins.contains_key(selected.session_admission_id()));
            let retaining = resident
                .sessions_retaining_generation(fixture.generation.compiled_generation_digest())
                .expect("reverse drain projection remains coherent");
            assert_eq!(retaining.len(), sessions - 1);
            assert!(!retaining.contains(selected.session_admission_id()));
            Sample {
                elapsed,
                sizes: file_sizes(&store.location.path()),
                journal_head: expected_head.get(),
                retained_sessions: pins.len(),
            }
        },
    )
}

fn assert_commit_publication(
    resident: &ResidentApplication<'_>,
    receipt: &ResidentCommitReceipt,
    prior_wake: WakeConsumerCursor,
    expected_head: JournalSequenceV1,
) {
    assert_eq!(receipt.journal_sequence(), expected_head);
    assert_eq!(resident.journal_head(), Some(expected_head));
    let ResidentWakeDisposition::Published(token) = receipt.wake() else {
        panic!("benchmark fixture has capacity for exact wake publication");
    };
    assert_eq!(token.journal_sequence(), expected_head);
    let batch = resident
        .wake_after(prior_wake)
        .expect("prior wake cursor remains within the prepared ring");
    assert_eq!(batch.len(), 1);
    assert_eq!(batch.iter().copied().next(), Some(token));
    assert_eq!(batch.advanced_cursor(), token.consumer_cursor());
}

fn next_sequence(prior: JournalSequenceV1) -> JournalSequenceV1 {
    JournalSequenceV1::new(
        prior
            .get()
            .checked_add(1)
            .expect("resident journal sequence overflow"),
    )
    .expect("successor resident journal sequence is positive")
}

fn run_reconstruction_sample(
    sessions: usize,
    configuration: &Configuration,
    workspace: &resident_fixture::shared_resident::SyntheticWorkspace,
    probe: &resident_fixture::shared_resident::RuntimeProbe,
    checkpoint_includes_pins: bool,
) -> Sample {
    let directory = private_tempdir();
    let store_fixture = initialize_deployment_store(directory.path());
    let (active_fixture, leases, expected_head) = {
        let store = open_store(&store_fixture);
        let mut source = FixedInputSource::default();
        let resolver = RecoveryResolver::default();
        let mut resident = open_resident(
            &store,
            &store_fixture,
            &resolver,
            &mut source,
            configuration,
            sessions,
            1,
        );
        let active_fixture = admit_and_activate(&mut resident, workspace, probe)
            .expect("real compiler and contextual generation admission succeed");
        if !checkpoint_includes_pins {
            resident
                .checkpoint()
                .expect("checkpoint active generation before pin tail");
        }
        let mut leases = Vec::new();
        leases
            .try_reserve_exact(sessions)
            .expect("validated reconstruction population can be retained");
        for index in 0..sessions {
            let lease = lease(&active_fixture, index);
            let _ = resident
                .pin_session(&lease)
                .expect("prepare recoverable session population");
            leases.push(lease);
        }
        if checkpoint_includes_pins {
            resident
                .checkpoint()
                .expect("checkpoint includes every session pin");
        }
        let expected_head = resident
            .journal_head()
            .expect("prepared recovery store has a durable head");
        (active_fixture, leases, expected_head)
    };

    let store = open_store(&store_fixture);
    let resolver = resolver_for_active(&active_fixture);
    let expected_generation = active_fixture
        .generation
        .compiled_generation_digest()
        .clone();
    let mut source = FixedInputSource {
        inputs: vec![reconstruction_input(workspace, probe, &active_fixture)],
        ..FixedInputSource::default()
    };
    let input = ResidentColdOpenInput::new(
        store_fixture.coordinate.clone(),
        configuration.bounds.recovery_limits(),
        configuration.bounds.maximum_entry_bytes,
        RecoveryReferenceKnowledge::Known(BTreeSet::new()),
        &resolver,
        &mut source,
        reconstruction_parameters(sessions, 2),
    );
    let started = Instant::now();
    let opened = black_box(
        ResidentApplication::cold_open(
            &store,
            &store,
            &mut &store,
            configuration.bounds.maximum_entry_bytes,
            input,
        )
        .expect("timed resident reconstruction succeeds"),
    );
    let elapsed = started.elapsed();
    black_box(&opened);
    let ResidentColdOpenOutcome::Ready(resident) = opened else {
        panic!("valid assembled fixture cannot reconstruct as degraded");
    };
    assert_eq!(source.calls, 1);
    assert_eq!(
        source.requested,
        Some(BTreeSet::from([expected_generation.clone()]))
    );
    let pins = resident.session_pins().expect("rebuilt pins are coherent");
    assert_eq!(pins.len(), sessions);
    for lease in &leases {
        let pin = pins
            .get(lease.session_admission_id())
            .expect("every exact retained lease reconstructs");
        assert_eq!(&pin.lease_digest, lease.session_admission_lease_digest());
        assert_eq!(&pin.generation, &expected_generation);
    }
    let retaining = resident
        .sessions_retaining_generation(&expected_generation)
        .expect("rebuilt reverse pins are coherent");
    assert_eq!(retaining.len(), sessions);
    assert!(leases
        .iter()
        .all(|lease| retaining.contains(lease.session_admission_id())));
    let wake = resident.wake_cursor();
    assert_eq!(wake.epoch(), WakeEpochV1::new(2));
    assert_eq!(wake.journal_sequence(), Some(expected_head));
    assert!(resident
        .wake_after(wake)
        .expect("reconstructed wake frontier is readable")
        .is_empty());
    assert_eq!(resident.journal_head(), Some(expected_head));
    Sample {
        elapsed,
        sizes: file_sizes(&store_fixture.location.path()),
        journal_head: expected_head.get(),
        retained_sessions: sessions,
    }
}

fn with_active_resident<T>(
    sessions: usize,
    configuration: &Configuration,
    workspace: &resident_fixture::shared_resident::SyntheticWorkspace,
    probe: &resident_fixture::shared_resident::RuntimeProbe,
    operation: impl FnOnce(&AdapterFixture, &ActiveResidentFixture, &mut ResidentApplication<'_>) -> T,
) -> T {
    let directory = private_tempdir();
    let store_fixture = initialize_deployment_store(directory.path());
    let store = open_store(&store_fixture);
    let resolver = RecoveryResolver::default();
    let mut source = FixedInputSource::default();
    let mut resident = open_resident(
        &store,
        &store_fixture,
        &resolver,
        &mut source,
        configuration,
        sessions,
        1,
    );
    let fixture = admit_and_activate(&mut resident, workspace, probe)
        .expect("real compiler and contextual generation admission succeed");
    operation(&store_fixture, &fixture, &mut resident)
}

fn open_store(fixture: &AdapterFixture) -> DeploymentControlSqliteStore {
    DeploymentControlSqliteStore::open_unadmitted_candidate(
        &fixture.location,
        fixture.coordinate.clone(),
    )
    .expect("open exact unadmitted deployment adapter")
}

fn open_resident<'store>(
    store: &'store DeploymentControlSqliteStore,
    fixture: &AdapterFixture,
    resolver: &dyn pale_application::resident::GenerationLifecycleRecoveryResolver,
    source: &mut dyn ResidentGenerationInputSource,
    configuration: &Configuration,
    sessions: usize,
    epoch: u64,
) -> ResidentApplication<'store> {
    let mut event_custody = store;
    let opened = ResidentApplication::cold_open(
        store,
        store,
        &mut event_custody,
        configuration.bounds.maximum_entry_bytes,
        ResidentColdOpenInput::new(
            fixture.coordinate.clone(),
            configuration.bounds.recovery_limits(),
            configuration.bounds.maximum_entry_bytes,
            RecoveryReferenceKnowledge::Known(BTreeSet::new()),
            resolver,
            source,
            reconstruction_parameters(sessions, epoch),
        ),
    )
    .expect("resident opens over exact SQLite candidate");
    match opened {
        ResidentColdOpenOutcome::Ready(resident) => *resident,
        ResidentColdOpenOutcome::Degraded(_) => panic!("valid benchmark store is not degraded"),
    }
}

fn reconstruction_parameters(sessions: usize, epoch: u64) -> ResidentReconstructionParameters {
    let capacity = sessions.checked_add(1).expect("resident capacity overflow");
    let wake_capacity = sessions.checked_add(4).expect("wake capacity overflow");
    ResidentReconstructionParameters::new(
        resident_fixture::shared_resident::synthetic_set_policy(),
        capacity,
        wake_capacity,
        WakeEpochV1::new(epoch),
    )
}

fn lease(fixture: &ActiveResidentFixture, index: usize) -> SessionAdmissionLeaseV1 {
    session_lease_for_active(
        fixture,
        &format!("resident-benchmark-session-{index:08}"),
        u64::try_from(index)
            .expect("session index fits lease budget")
            .checked_add(1)
            .expect("session lease budget overflow"),
    )
}

fn report_header() -> &'static str {
    "evidence_class\tworkload_class\tfixture_class\toperation\ttiming_scope\tscale\tcase_ordinal\tsession_population\tresident_session_capacity\twake_ring_capacity\tindependent_samples\telapsed_p50_ns\telapsed_p95_ns\telapsed_p99_ns\telapsed_max_ns\tjournal_head_p50\tretained_sessions_p50\tmain_p50_bytes\twal_p50_bytes\tshm_p50_bytes\tmaximum_entry_bytes\tmaximum_checkpoint_bytes\tmaximum_checkpoint_candidates\tmaximum_tail_entries\tprojected_total_transitions\tadapter_path\tsemantic_generation_admission\towner_qualified_actual\tprofile_selected\tstorage_admission_complete\tacceptance_budget_selected\trelease_evidence\tallocation_scope\trss_scope\tnative_effect_scope\tsource_coordinate\trun_coordinate\thardware\tos\tarch"
}

fn render_row(
    ordinal: usize,
    operation: Operation,
    scale: Scale,
    sessions: usize,
    configuration: &Configuration,
    measurements: &Measurements,
) -> String {
    assert_eq!(measurements.elapsed.len(), configuration.samples);
    let elapsed = Distribution::durations(&measurements.elapsed, configuration.samples).unwrap();
    let head = Distribution::numbers(
        measurements.journal_heads.iter().copied(),
        configuration.samples,
    )
    .unwrap();
    let retained = Distribution::numbers(
        measurements.retained_sessions.iter().copied(),
        configuration.samples,
    )
    .unwrap();
    let main = Distribution::numbers(
        measurements.main_bytes.iter().copied(),
        configuration.samples,
    )
    .unwrap();
    let wal = Distribution::numbers(
        measurements.wal_bytes.iter().copied(),
        configuration.samples,
    )
    .unwrap();
    let shm = Distribution::numbers(
        measurements.shm_bytes.iter().copied(),
        configuration.samples,
    )
    .unwrap();
    let capacity = sessions.checked_add(1).unwrap();
    let wake_capacity = sessions.checked_add(4).unwrap();
    let p99 = elapsed
        .p99
        .map_or_else(String::new, |value| value.to_string());
    let fields = vec![
        configuration.evidence_class().to_owned(),
        "assembled_resident_unadmitted_sqlite_v1".to_owned(),
        "real_compiler_and_contextual_admission_synthetic_runtime_materialization_sessions"
            .to_owned(),
        operation.label().to_owned(),
        operation.timing_scope().to_owned(),
        scale.label.to_owned(),
        ordinal.to_string(),
        sessions.to_string(),
        capacity.to_string(),
        wake_capacity.to_string(),
        configuration.samples.to_string(),
        elapsed.p50.to_string(),
        elapsed.p95.to_string(),
        p99,
        elapsed.max.to_string(),
        head.p50.to_string(),
        retained.p50.to_string(),
        main.p50.to_string(),
        wal.p50.to_string(),
        shm.p50.to_string(),
        configuration.bounds.maximum_entry_bytes.to_string(),
        configuration.bounds.maximum_checkpoint_bytes.to_string(),
        configuration
            .bounds
            .maximum_checkpoint_candidates
            .to_string(),
        configuration.bounds.maximum_tail_entries.to_string(),
        configuration.projected_total_transitions.to_string(),
        "true".to_owned(),
        "true".to_owned(),
        "false".to_owned(),
        "false".to_owned(),
        "false".to_owned(),
        "false".to_owned(),
        "false".to_owned(),
        "not_measured_by_this_initial_lane".to_owned(),
        "not_measured_by_this_initial_lane".to_owned(),
        "none_fixture_only".to_owned(),
        configuration.source_coordinate.clone(),
        configuration.run_coordinate.clone(),
        configuration.hardware.clone(),
        std::env::consts::OS.to_owned(),
        std::env::consts::ARCH.to_owned(),
    ];
    assert_eq!(fields.len(), report_header().split('\t').count());
    fields.join("\t")
}

fn scaled_sessions(baseline: usize, multiplier: usize) -> usize {
    baseline
        .checked_mul(multiplier)
        .expect("resident benchmark session scale overflow")
}

fn projected_transitions(baseline_sessions: usize, scales: &[Scale], samples: usize) -> usize {
    let transitions_per_sample = scales
        .iter()
        .try_fold(0usize, |total, scale| {
            let sessions = scaled_sessions(baseline_sessions, scale.multiplier);
            // Each row creates a three-transition active generation. Pin performs
            // N pins; drain performs N pins plus one drain; both recovery rows
            // perform N pins. Checkpoints do not advance semantic authority.
            let scale_transitions = sessions
                .checked_mul(4)?
                .checked_add(OPERATIONS_PER_SCALE * 3 + 1)?;
            total.checked_add(scale_transitions)
        })
        .expect("resident benchmark transition projection overflow");
    transitions_per_sample
        .checked_mul(samples)
        .expect("resident benchmark total transition projection overflow")
}

fn flag(name: &str) -> bool {
    match std::env::var_os(name) {
        None => false,
        Some(value) if value == "1" => true,
        Some(_) => panic!("{name} must be 1 when present"),
    }
}

fn bounded_number(name: &str, smoke: bool, smoke_default: usize) -> usize {
    match std::env::var(name) {
        Ok(value) => value
            .parse::<usize>()
            .unwrap_or_else(|_| panic!("{name} must be a positive base-10 integer")),
        Err(std::env::VarError::NotPresent) if smoke => smoke_default,
        Err(std::env::VarError::NotPresent) => panic!("{name} is required"),
        Err(std::env::VarError::NotUnicode(_)) => panic!("{name} must be UTF-8"),
    }
}

fn coordinate(name: &str, smoke: bool, smoke_default: &str) -> String {
    let value = match std::env::var(name) {
        Ok(value) => value,
        Err(std::env::VarError::NotPresent) if smoke => smoke_default.to_owned(),
        Err(std::env::VarError::NotPresent) => panic!("{name} is required"),
        Err(std::env::VarError::NotUnicode(_)) => panic!("{name} must be UTF-8"),
    };
    assert!(
        !value.is_empty() && value.len() <= MAX_COORDINATE_BYTES,
        "{name} must contain 1..={MAX_COORDINATE_BYTES} UTF-8 bytes"
    );
    assert!(
        !value
            .bytes()
            .any(|byte| matches!(byte, b'\t' | b'\n' | b'\r')),
        "{name} cannot contain TSV control bytes"
    );
    value
}

#[cfg(test)]
#[path = "../../tests/internal/benchmark_resident_application.rs"]
mod tests;
