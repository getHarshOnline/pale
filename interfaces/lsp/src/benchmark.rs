//! Synthetic characterization of the LSP-owned editor text pipeline.
//!
//! The harness emits checksum-sealed TSV to standard output. A full run keeps
//! 201 observations per case; `PALE_BENCH_SMOKE=1` selects five observations
//! that prove only that the harness compiles and executes. Smoke rows leave
//! `p99_ns` empty because five observations cannot support that percentile.
//! The separate `PALE_BENCH_RUN=1` opt-in keeps ordinary all-target test
//! discovery from performing or misclassifying a characterization run.

use std::cell::RefCell;
use std::fmt::Write as _;
use std::hint::black_box;
use std::io::{BufWriter, Write as _};
use std::process::Command;
use std::time::{Duration, Instant};

use lsp_types::{TextDocumentContentChangeEvent, Uri};
use sha2::{Digest, Sha256};

#[path = "../benches/support/editor_pipeline.rs"]
mod support;

use super::document::{BaselineDocument, DocumentRole, DocumentStore, TextMutationMetrics};
use super::scheduling::{AnalysisClock, ANALYSIS_DEBOUNCE};
use support::{
    corpus_sha256, selected_document_sizes, EditorFixture, DEBOUNCE_BURST, RESOLUTION_LOCK,
    UNIVERSE_LOCK,
};

/// Observations retained for every characterization case.
const CHARACTERIZATION_SAMPLES: usize = 201;
/// Compile-and-execute observations, ineligible as characterization evidence.
const SMOKE_SAMPLES: usize = 5;
/// Minimum elapsed interval used to calibrate repeatable read-only operations.
const CALIBRATION_TARGET: Duration = Duration::from_millis(2);
/// Maximum calibrated batch for a single observation.
const MAX_BATCH: usize = 65_536;
/// Maximum retained caller metadata field in one TSV row.
const MAX_TSV_FIELD_BYTES: usize = 512;
/// Every workload closure runs synchronously on the calling benchmark thread.
const WORKER_THREADS: usize = 1;
/// Expected smoke rows: four text operations at one size plus debounce.
const SMOKE_ROWS: usize = 5;
/// Expected characterization rows: four operations at two sizes plus debounce.
const CHARACTERIZATION_ROWS: usize = 9;
/// Completion marker separating payload rows from the terminal seal.
const COMPLETION_MARKER: &str = "#pale_lsp_editor_pipeline_complete_v1";

/// Run the opt-in editor-pipeline characterization against production modules.
///
/// Without an explicit benchmark environment selector, this returns after
/// reporting that ordinary Cargo target discovery skipped characterization.
pub fn run() {
    let Some(configuration) = Configuration::from_environment() else {
        eprintln!("editor-pipeline benchmark skipped; set PALE_BENCH_RUN=1 or PALE_BENCH_SMOKE=1");
        return;
    };
    let reporter = Reporter::new(configuration);
    reporter.print_header();
    for &document_bytes in selected_document_sizes(configuration.smoke) {
        benchmark_document(&reporter, document_bytes);
    }
    benchmark_debounce(&reporter);
    reporter.finish();
}

#[derive(Clone, Copy, Debug)]
struct Configuration {
    smoke: bool,
    samples: usize,
}

impl Configuration {
    fn from_environment() -> Option<Self> {
        let smoke = std::env::var_os("PALE_BENCH_SMOKE").is_some_and(|value| value == "1");
        let characterize = std::env::var_os("PALE_BENCH_RUN").is_some_and(|value| value == "1");
        assert!(
            !(smoke && characterize),
            "select either smoke validation or full characterization"
        );
        if !smoke && !characterize {
            return None;
        }
        Some(Self {
            smoke,
            samples: if smoke {
                SMOKE_SAMPLES
            } else {
                CHARACTERIZATION_SAMPLES
            },
        })
    }

    const fn evidence_class(self) -> &'static str {
        if self.smoke {
            "smoke_validation"
        } else {
            "synthetic_characterization"
        }
    }

    const fn expected_rows(self) -> usize {
        if self.smoke {
            SMOKE_ROWS
        } else {
            CHARACTERIZATION_ROWS
        }
    }
}

struct Reporter {
    configuration: Configuration,
    source_coordinate: String,
    corpus_sha256: String,
    run_coordinate: String,
    hardware: String,
    os: &'static str,
    architecture: &'static str,
    toolchain: String,
    target: String,
    profile: &'static str,
    available_parallelism: Option<usize>,
    output: RefCell<EvidenceOutput>,
}

impl Reporter {
    fn new(configuration: Configuration) -> Self {
        let rustc = Command::new("rustc")
            .arg("-vV")
            .output()
            .expect("rustc must be available to identify a benchmark run");
        assert!(rustc.status.success(), "rustc -vV must succeed");
        let rustc = String::from_utf8(rustc.stdout).expect("rustc version output must be UTF-8");
        let toolchain = rustc.lines().next().unwrap_or("unknown-rustc").to_owned();
        let target = rustc
            .lines()
            .find_map(|line| line.strip_prefix("host: "))
            .unwrap_or("unknown-target")
            .to_owned();
        let hardware = std::env::var("PALE_BENCH_HARDWARE").unwrap_or_else(|_| {
            if configuration.smoke {
                "unspecified-smoke-host".to_owned()
            } else {
                panic!("set PALE_BENCH_HARDWARE for a full characterization run")
            }
        });
        Self {
            configuration,
            source_coordinate: benchmark_coordinate(
                "PALE_BENCH_SOURCE_COORDINATE",
                configuration,
                "unsealed-smoke-source",
            ),
            corpus_sha256: corpus_sha256(configuration.smoke),
            run_coordinate: benchmark_coordinate(
                "PALE_BENCH_RUN_COORDINATE",
                configuration,
                "unsealed-smoke-run",
            ),
            hardware: tsv_value(&hardware),
            os: std::env::consts::OS,
            architecture: std::env::consts::ARCH,
            toolchain: tsv_value(&toolchain),
            target: tsv_value(&target),
            profile: if cfg!(debug_assertions) {
                "debug"
            } else {
                "release"
            },
            available_parallelism: std::thread::available_parallelism()
                .ok()
                .map(std::num::NonZeroUsize::get),
            output: RefCell::new(EvidenceOutput::new()),
        }
    }

    fn print_header(&self) {
        self.output.borrow_mut().write_header(
            "source_coordinate\tcorpus_sha256\trun_coordinate\tevidence_class\towner_qualified_actual\tpolicy_selected\tacceptance_budget_selected\trelease_evidence\tfamily\toperation\tdocument_bytes\tlogical_touched_bytes\tboundary_copied_bytes_lower_bound\tsamples\tbatch_size\tp50_ns\tp95_ns\tp99_ns\tmax_ns\tops_per_second\thardware\tos\tarch\ttoolchain\tprofile\tallocator\ttarget\tavailable_parallelism\tworker_threads\tallocation_scope\tallocation_samples\tallocation_total_count\tallocation_total_bytes\tallocation_peak_relative_count\tallocation_peak_relative_bytes\tallocation_retained_count\tallocation_retained_bytes\trss_scope\tprocess_rss_high_water_bytes\ttiming_allocation_counting",
        );
    }

    fn report(&self, case: CaseDescriptor, measurement: Measurement) {
        let mut row = format!(
            "{source}\t{corpus}\t{run}\t{evidence}\tfalse\tfalse\tfalse\tfalse\t{family}\t{operation}\t{document_bytes}\t{logical}\t{copied}\t{samples}\t{batch}\t{p50}\t{p95}\t{p99}\t{max}\t{throughput}\t{hardware}\t{os}\t{architecture}\t{toolchain}\t{profile}\t{allocator}\t{target}\t{available_parallelism}\t{worker_threads}",
            source = self.source_coordinate,
            corpus = self.corpus_sha256,
            run = self.run_coordinate,
            evidence = self.configuration.evidence_class(),
            family = case.family,
            operation = case.operation,
            document_bytes = case.document_bytes,
            logical = case.logical_touched_bytes,
            copied = case.boundary_copied_bytes_lower_bound,
            samples = measurement.samples,
            batch = measurement.batch_size,
            p50 = measurement.p50_ns,
            p95 = measurement.p95_ns,
            p99 = measurement
                .p99_ns
                .map(|value| value.to_string())
                .unwrap_or_default(),
            max = measurement.max_ns,
            throughput = measurement.ops_per_second,
            hardware = self.hardware,
            os = self.os,
            architecture = self.architecture,
            toolchain = self.toolchain,
            profile = self.profile,
            allocator = allocator_coordinate(),
            target = self.target,
            available_parallelism = self.available_parallelism.map(|count| count.to_string()).unwrap_or_default(),
            worker_threads = WORKER_THREADS,
        );
        measurement.memory.append_tsv(&mut row);
        self.output.borrow_mut().write_row(&row);
    }

    fn measure<T>(&self, operation: impl FnMut() -> T) -> Measurement {
        Measurement::sample(self.configuration, operation)
    }

    fn measure_prepared<S, T>(
        &self,
        prepare: impl FnMut() -> S,
        operation: impl FnMut(S) -> T,
    ) -> Measurement {
        Measurement::sample_prepared(self.configuration, prepare, operation)
    }

    fn finish(self) {
        let Self {
            configuration,
            source_coordinate,
            corpus_sha256,
            run_coordinate,
            output,
            ..
        } = self;
        output.into_inner().finish(
            configuration.expected_rows(),
            &source_coordinate,
            &corpus_sha256,
            &run_coordinate,
        );
    }
}

#[derive(Clone, Copy, Debug)]
struct CaseDescriptor {
    family: &'static str,
    operation: &'static str,
    document_bytes: usize,
    logical_touched_bytes: usize,
    boundary_copied_bytes_lower_bound: usize,
}

impl CaseDescriptor {
    const fn text(
        operation: &'static str,
        document_bytes: usize,
        logical_touched_bytes: usize,
        boundary_copied_bytes_lower_bound: usize,
    ) -> Self {
        Self {
            family: "editor_text",
            operation,
            document_bytes,
            logical_touched_bytes,
            boundary_copied_bytes_lower_bound,
        }
    }

    const fn debounce() -> Self {
        Self {
            family: "analysis_scheduling",
            operation: "supersede_and_claim_8_generations",
            document_bytes: 0,
            logical_touched_bytes: 0,
            boundary_copied_bytes_lower_bound: 0,
        }
    }
}

/// Untimed memory observations; neither field measures bytes copied or sent.
///
/// RSS includes retained fixtures and earlier cases in this process. It is not
/// a per-operation delta, steady-state sample, fragmentation or leak verdict.
#[derive(Clone, Copy, Debug, Default)]
struct MemoryEvidence {
    allocations: Option<AllocationEvidence>,
    process_rss_high_water_bytes: Option<u64>,
}

/// Current-thread Rust allocator observations, including result destruction.
///
/// Ropey's Rust storage is counted, but other threads and native heaps are not.
/// Peaks describe the measured balance from zero, not absolute live storage:
/// freeing prepared input before allocating output can suppress the peak.
#[derive(Clone, Copy, Debug)]
struct AllocationEvidence {
    total_count: u64,
    total_bytes: u64,
    /// High-water of the measured balance starting at zero, not absolute live allocations.
    peak_relative_count: u64,
    /// High-water of the measured byte balance, excluding pre-existing live storage.
    peak_relative_bytes: u64,
    retained_count: i64,
    retained_bytes: i64,
}

impl MemoryEvidence {
    fn append_tsv(self, row: &mut String) {
        match self.allocations {
            Some(evidence) => write!(
                row,
                "\tcurrent_thread_one_operation_rust_allocator_including_result_drop\t1\t{}\t{}\t{}\t{}\t{}\t{}",
                evidence.total_count,
                evidence.total_bytes,
                evidence.peak_relative_count,
                evidence.peak_relative_bytes,
                evidence.retained_count,
                evidence.retained_bytes,
            )
            .expect("writing benchmark evidence to a String is infallible"),
            None => row.push_str("\tunavailable_feature_disabled\t\t\t\t\t\t\t"),
        }
        if let Some(bytes) = self.process_rss_high_water_bytes {
            write!(row, "\tprocess_lifetime_high_water\t{bytes}")
                .expect("writing benchmark evidence to a String is infallible");
        } else {
            row.push_str("\tunavailable_feature_platform_or_observation\t");
        }
        row.push_str("\tdisabled");
    }
}

/// Prepare outside one counted invocation; include the result's destruction.
///
/// Fixture allocations, reporting, other threads and native heaps are excluded.
/// Dropping prepared input may make the signed retained balance negative.
fn measure_memory<S, T>(
    prepare: impl FnOnce() -> S,
    operation: impl FnOnce(S) -> T,
) -> MemoryEvidence {
    #[cfg(feature = "bench-memory")]
    let allocations = {
        let state = prepare();
        let observed = allocation_counter::measure(|| drop(black_box(operation(state))));
        Some(AllocationEvidence {
            total_count: observed.count_total,
            total_bytes: observed.bytes_total,
            peak_relative_count: observed.count_max,
            peak_relative_bytes: observed.bytes_max,
            retained_count: observed.count_current,
            retained_bytes: observed.bytes_current,
        })
    };
    #[cfg(not(feature = "bench-memory"))]
    let allocations = {
        let _ = (prepare, operation);
        None
    };
    MemoryEvidence {
        allocations,
        process_rss_high_water_bytes: resident_set_high_water(),
    }
}

/// Disable counter bookkeeping throughout warm-up, calibration and timing.
///
/// The opt-in binary still uses the instrument's system-allocator wrapper.
/// Comparing with an ordinary system-allocator build requires a separate run.
fn without_allocation_counting<T>(run: impl FnOnce() -> T) -> T {
    #[cfg(feature = "bench-memory")]
    {
        let mut result = None;
        allocation_counter::opt_out(|| result = Some(run()));
        result.expect("the timing closure returns one result")
    }
    #[cfg(not(feature = "bench-memory"))]
    run()
}

const fn allocator_coordinate() -> &'static str {
    if cfg!(feature = "bench-memory") {
        "system_with_allocation-counter-0.8.1"
    } else {
        "system"
    }
}

/// Process-lifetime RSS, normalized from macOS bytes or Linux kibibytes.
///
/// Other platforms remain unavailable until their native units are verified.
#[cfg(all(
    feature = "bench-memory",
    any(target_os = "macos", target_os = "linux")
))]
fn resident_set_high_water() -> Option<u64> {
    use nix::sys::resource::{getrusage, UsageWho};

    rss_bytes(getrusage(UsageWho::RUSAGE_SELF).ok()?.max_rss())
}

/// Reject unavailable native readings or unit conversions instead of inventing zero.
#[cfg(all(
    feature = "bench-memory",
    any(target_os = "macos", target_os = "linux")
))]
fn rss_bytes(native: std::ffi::c_long) -> Option<u64> {
    let bytes = u64::try_from(native).ok()?;
    if cfg!(target_os = "macos") {
        Some(bytes)
    } else {
        bytes.checked_mul(1024)
    }
}

#[cfg(not(all(
    feature = "bench-memory",
    any(target_os = "macos", target_os = "linux")
)))]
const fn resident_set_high_water() -> Option<u64> {
    None
}

#[derive(Clone, Copy, Debug)]
struct Measurement {
    samples: usize,
    batch_size: usize,
    p50_ns: u128,
    p95_ns: u128,
    p99_ns: Option<u128>,
    max_ns: u128,
    ops_per_second: u128,
    memory: MemoryEvidence,
}

impl Measurement {
    fn sample<T>(configuration: Configuration, mut operation: impl FnMut() -> T) -> Self {
        let mut measurement = without_allocation_counting(|| {
            black_box(operation());
            let batch_size = if configuration.smoke {
                1
            } else {
                calibrated_batch(&mut operation)
            };
            let mut observations = Vec::with_capacity(configuration.samples);
            for _ in 0..configuration.samples {
                let start = Instant::now();
                for _ in 0..batch_size {
                    black_box(operation());
                }
                observations.push((start.elapsed().as_nanos() / batch_size as u128).max(1));
            }
            Self::summarize(observations, batch_size)
        });
        measurement.memory = measure_memory(|| (), |()| operation());
        measurement
    }

    fn sample_prepared<S, T>(
        configuration: Configuration,
        mut prepare: impl FnMut() -> S,
        mut operation: impl FnMut(S) -> T,
    ) -> Self {
        let mut measurement = without_allocation_counting(|| {
            black_box(operation(prepare()));
            let mut observations = Vec::with_capacity(configuration.samples);
            for _ in 0..configuration.samples {
                let state = prepare();
                let start = Instant::now();
                let output = black_box(operation(state));
                let elapsed = start.elapsed();
                black_box(&output);
                observations.push(elapsed.as_nanos().max(1));
            }
            Self::summarize(observations, 1)
        });
        measurement.memory = measure_memory(prepare, operation);
        measurement
    }

    fn summarize(mut observations: Vec<u128>, batch_size: usize) -> Self {
        let sum = observations.iter().sum::<u128>();
        let samples = observations.len();
        observations.sort_unstable();
        let mean = (sum / samples as u128).max(1);
        Self {
            samples,
            batch_size,
            p50_ns: nearest_rank(&observations, 50),
            p95_ns: nearest_rank(&observations, 95),
            p99_ns: (samples >= 101).then(|| nearest_rank(&observations, 99)),
            max_ns: *observations.last().expect("measurement has samples"),
            ops_per_second: 1_000_000_000 / mean,
            memory: MemoryEvidence::default(),
        }
    }
}

struct EvidenceOutput {
    writer: BufWriter<std::io::Stdout>,
    payload_digest: Sha256,
    rows: usize,
}

impl EvidenceOutput {
    fn new() -> Self {
        Self {
            writer: BufWriter::new(std::io::stdout()),
            payload_digest: Sha256::new(),
            rows: 0,
        }
    }

    fn write_header(&mut self, header: &str) {
        self.write_payload_line(header);
    }

    fn write_row(&mut self, row: &str) {
        self.write_payload_line(row);
        self.rows += 1;
    }

    fn write_payload_line(&mut self, line: &str) {
        self.payload_digest.update(line.as_bytes());
        self.payload_digest.update(b"\n");
        writeln!(self.writer, "{line}").expect("benchmark output must remain writable");
    }

    fn finish(
        mut self,
        expected_rows: usize,
        source_coordinate: &str,
        corpus_sha256: &str,
        run_coordinate: &str,
    ) {
        assert_eq!(self.rows, expected_rows, "benchmark row count drifted");
        let payload_sha256 = encode_hex(self.payload_digest.finalize());
        writeln!(
            self.writer,
            "{COMPLETION_MARKER}\trows={}\texpected_rows={expected_rows}\tsource_coordinate={}\tcorpus_sha256={corpus_sha256}\trun_coordinate={}\tsha256={payload_sha256}",
            self.rows, source_coordinate, run_coordinate,
        )
        .expect("benchmark terminal seal must remain writable");
        self.writer
            .flush()
            .expect("benchmark terminal seal must be flushed");
    }
}

fn benchmark_document(reporter: &Reporter, document_bytes: usize) {
    let fixture = EditorFixture::new(document_bytes);
    let (incremental_metrics, replacement_metrics) = preflight_document(&fixture);

    let snapshot_store = opened_store(&fixture.initial);
    reporter.report(
        CaseDescriptor::text("snapshot_structural_share", document_bytes, 0, 0),
        reporter.measure(|| snapshot_store.snapshot()),
    );

    let snapshot = snapshot_store.snapshot();
    let root_document_uri = root_uri();
    let root_document = snapshot
        .document(&root_document_uri)
        .expect("the benchmark root remains in every snapshot");
    reporter.report(
        CaseDescriptor::text("flatten_parser_boundary", document_bytes, 0, document_bytes),
        reporter.measure(|| root_document.bytes()),
    );

    let incremental_range = fixture.incremental_range;
    reporter.report(
        CaseDescriptor::text(
            "incremental_atomic_edit",
            document_bytes,
            incremental_metrics.logical_bytes_touched,
            incremental_metrics.replacement_bytes_copied,
        ),
        reporter.measure_prepared(
            || {
                (
                    opened_store(&fixture.initial),
                    vec![TextDocumentContentChangeEvent {
                        range: Some(incremental_range),
                        range_length: Some(1),
                        text: "b".to_owned(),
                    }],
                )
            },
            |(mut store, changes)| {
                store
                    .change(&root_document_uri, 2, changes)
                    .expect("the deterministic incremental edit remains valid");
                store
            },
        ),
    );

    reporter.report(
        CaseDescriptor::text(
            "full_document_replacement",
            document_bytes,
            replacement_metrics.logical_bytes_touched,
            replacement_metrics.replacement_bytes_copied,
        ),
        reporter.measure_prepared(
            || {
                (
                    opened_store(&fixture.initial),
                    vec![TextDocumentContentChangeEvent {
                        range: None,
                        range_length: None,
                        text: fixture.full_replacement.clone(),
                    }],
                )
            },
            |(mut store, changes)| {
                store
                    .change(&root_document_uri, 2, changes)
                    .expect("the deterministic full replacement remains valid");
                store
            },
        ),
    );
}

fn preflight_document(fixture: &EditorFixture) -> (TextMutationMetrics, TextMutationMetrics) {
    let uri = root_uri();
    let mut incremental = opened_store(&fixture.initial);
    incremental
        .change(
            &uri,
            2,
            vec![TextDocumentContentChangeEvent {
                range: Some(fixture.incremental_range),
                range_length: Some(1),
                text: "b".to_owned(),
            }],
        )
        .expect("the deterministic incremental edit must remain valid");
    assert_eq!(
        incremental
            .snapshot()
            .document(&uri)
            .expect("the edited root remains present")
            .text(),
        fixture.incremental_result
    );

    let mut replacement = opened_store(&fixture.initial);
    replacement
        .change(
            &uri,
            2,
            vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: fixture.full_replacement.clone(),
            }],
        )
        .expect("the deterministic full replacement must remain valid");
    assert_eq!(
        replacement
            .snapshot()
            .document(&uri)
            .expect("the replaced root remains present")
            .text(),
        fixture.full_replacement
    );
    (
        incremental.last_change_metrics(),
        replacement.last_change_metrics(),
    )
}

fn benchmark_debounce(reporter: &Reporter) {
    assert_debounce_law();
    reporter.report(
        CaseDescriptor::debounce(),
        reporter.measure_prepared(AnalysisClock::default, |mut clock| {
            let start = Instant::now();
            let first = clock
                .schedule(start)
                .expect("the benchmark clock has a successor");
            let mut latest = first;
            for _ in 1..DEBOUNCE_BURST {
                latest = clock
                    .schedule(start)
                    .expect("the bounded benchmark burst has a successor");
            }
            let due = start + ANALYSIS_DEBOUNCE;
            (clock.claim(first, due), clock.claim(latest, due))
        }),
    );
}

fn assert_debounce_law() {
    let start = Instant::now();
    let mut clock = AnalysisClock::default();
    let first = clock.schedule(start).unwrap();
    let mut latest = first;
    for _ in 1..DEBOUNCE_BURST {
        latest = clock.schedule(start).unwrap();
    }
    assert!(!clock.claim(latest, start + ANALYSIS_DEBOUNCE - Duration::from_nanos(1)));
    assert!(!clock.claim(first, start + ANALYSIS_DEBOUNCE));
    assert!(clock.claim(latest, start + ANALYSIS_DEBOUNCE));
    assert!(!clock.claim(latest, start + ANALYSIS_DEBOUNCE));
}

fn opened_store(text: &str) -> DocumentStore {
    let mut store = DocumentStore::new(
        &workspace_uri(),
        vec![
            baseline(
                "file:///workspace/pale-workspace.toml",
                DocumentRole::WorkspaceManifest,
                text,
            ),
            baseline(
                "file:///workspace/resolution.lock.toml",
                DocumentRole::WorkspaceResolutionLock,
                RESOLUTION_LOCK,
            ),
            baseline(
                "file:///workspace/universe.lock.toml",
                DocumentRole::WorkspaceUniverseLock,
                UNIVERSE_LOCK,
            ),
        ],
    )
    .expect("the canonical benchmark fixture remains inside ingress limits");
    store
        .open(root_uri(), "pale-toml".to_owned(), 1, text.to_owned())
        .expect("the canonical benchmark root remains registered");
    store
}

fn baseline(uri: &str, role: DocumentRole, text: &str) -> BaselineDocument {
    BaselineDocument {
        uri: uri.parse().expect("benchmark URI is valid"),
        language_id: "pale-toml".to_owned(),
        role,
        text: text.to_owned(),
    }
}

fn workspace_uri() -> Uri {
    "file:///workspace".parse().expect("workspace URI is valid")
}

fn root_uri() -> Uri {
    "file:///workspace/pale-workspace.toml"
        .parse()
        .expect("root URI is valid")
}

fn calibrated_batch<T>(operation: &mut impl FnMut() -> T) -> usize {
    let mut batch = 1;
    loop {
        let start = Instant::now();
        for _ in 0..batch {
            black_box(operation());
        }
        if start.elapsed() >= CALIBRATION_TARGET || batch == MAX_BATCH {
            return batch;
        }
        batch = (batch * 2).min(MAX_BATCH);
    }
}

fn nearest_rank(sorted: &[u128], percentile: usize) -> u128 {
    let rank = (percentile * sorted.len()).div_ceil(100).saturating_sub(1);
    sorted[rank]
}

fn benchmark_coordinate(
    variable: &str,
    configuration: Configuration,
    smoke_default: &str,
) -> String {
    let coordinate = std::env::var(variable).unwrap_or_else(|_| {
        if configuration.smoke {
            smoke_default.to_owned()
        } else {
            panic!("set {variable} for a full characterization run")
        }
    });
    assert!(
        !coordinate.is_empty() && coordinate.len() <= 256,
        "{variable} must contain between 1 and 256 bytes"
    );
    assert!(
        !coordinate.chars().any(char::is_control),
        "{variable} must not contain control characters"
    );
    tsv_value(&coordinate)
}

fn tsv_value(value: &str) -> String {
    assert!(
        value.len() <= MAX_TSV_FIELD_BYTES,
        "benchmark metadata exceeds {MAX_TSV_FIELD_BYTES} bytes"
    );
    assert!(
        !value.chars().any(|character| character.is_control()),
        "benchmark metadata must not contain control characters"
    );
    value.to_owned()
}

fn encode_hex(bytes: impl AsRef<[u8]>) -> String {
    let bytes = bytes.as_ref();
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut output, "{byte:02x}").expect("writing to a String is infallible");
    }
    output
}

#[cfg(test)]
#[path = "../tests/internal/benchmark.rs"]
mod tests;
