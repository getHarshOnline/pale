//! Synthetic characterization harness for resident-kernel mechanics.
//!
//! `PALE_BENCH_RUN=1` selects a full run with at least 201 observations per
//! case and exact evidence coordinates.
//! `PALE_BENCH_SMOKE=1` selects a tiny compile-and-execute validation path;
//! smoke output is explicitly ineligible for characterization or policy choice
//! and leaves `batch_mean_p99_ns` empty.
//! `PALE_BENCH_SCOPE=graph` selects only the graph fixtures while retaining
//! the same generator, measurements, row contract, and evidence seal.

use std::cell::RefCell;
use std::fmt::Write as _;
use std::hint::black_box;
use std::io::{BufWriter, Write as _};
use std::process::Command;

use pale_core::records::{JournalSequenceV1, RingCursorV1, WakeEpochV1};
#[cfg(all(feature = "graphblas", not(target_family = "wasm")))]
use pale_kernel::BatchReachabilityBackend;
use pale_kernel::{
    AdaptiveNodeSet, GenerationArena, RuntimeArena, ScratchArena, WakeReadError, WakeTokenRing,
};
use petgraph::algo::{kosaraju_scc, toposort};
use sha2::{Digest, Sha256};

#[path = "../tests/support/resident_kernel_benchmark.rs"]
mod resident_kernel_benchmark;

#[path = "../tests/support/resident_kernel_measurement.rs"]
mod resident_kernel_measurement;

use resident_kernel_measurement::{parallelism_field, Measurement, Sampling};

use resident_kernel_benchmark::{
    characterization_set_cases, expected_benchmark_rows, graph_edges, paired_set_members,
    reference_closure, synthetic_runtime_bodies, synthetic_set_policy, Cardinality, GraphFixture,
    GraphPattern, SetFixture, SetPattern, SyntheticPackedGraph, CHARACTERIZATION_SET_FIXTURE_COUNT,
    SYNTHETIC_SEED,
};

/// Maximum caller-supplied metadata field retained in one streamed TSV row.
const MAX_TSV_FIELD_BYTES: usize = 512;

fn main() {
    let Some(configuration) = Configuration::from_environment() else {
        eprintln!("resident-kernel benchmark skipped; set PALE_BENCH_RUN=1 or PALE_BENCH_SMOKE=1");
        return;
    };
    let reporter = Reporter::new(configuration);
    reporter.print_header();
    match configuration.scope {
        BenchmarkScope::All => {
            benchmark_sets(&reporter);
            benchmark_graphs(&reporter);
            benchmark_arenas(&reporter);
            benchmark_wake_rings(&reporter);
        }
        BenchmarkScope::Graph => benchmark_graphs(&reporter),
    }
    reporter.finish();
}

#[derive(Debug, Clone, Copy)]
struct Configuration {
    /// Whether the ineligible compile-and-execute corpus is selected.
    smoke: bool,
    /// Closed fixture-family selection for this run.
    scope: BenchmarkScope,
}

/// Bounded fixture-family selection within the single canonical harness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BenchmarkScope {
    /// Adaptive sets, graphs, arenas, and process-local wake rings.
    All,
    /// Packed-graph mechanics only.
    Graph,
}

impl Configuration {
    /// Resolve one explicit run mode, or skip ordinary Cargo target discovery.
    fn from_environment() -> Option<Self> {
        let smoke = benchmark_flag("PALE_BENCH_SMOKE");
        let characterize = benchmark_flag("PALE_BENCH_RUN");
        assert!(
            !(smoke && characterize),
            "select either smoke validation or full characterization"
        );
        if !smoke && !characterize {
            return None;
        }
        let scope = match std::env::var("PALE_BENCH_SCOPE").as_deref() {
            Err(std::env::VarError::NotPresent) => BenchmarkScope::All,
            Ok("graph") => BenchmarkScope::Graph,
            Ok(value) => panic!("unsupported PALE_BENCH_SCOPE {value:?}; expected graph"),
            Err(std::env::VarError::NotUnicode(_)) => {
                panic!("PALE_BENCH_SCOPE must be valid Unicode")
            }
        };
        Some(Self { smoke, scope })
    }

    const fn sampling(self) -> Sampling {
        if self.smoke {
            Sampling::Smoke
        } else {
            Sampling::Characterization
        }
    }

    const fn evidence_class(self) -> &'static str {
        if self.smoke {
            "smoke_validation"
        } else {
            "synthetic_characterization"
        }
    }

    const fn expected_rows(self) -> usize {
        expected_benchmark_rows(
            self.smoke,
            cfg!(all(feature = "graphblas", not(target_family = "wasm"))),
            matches!(self.scope, BenchmarkScope::Graph),
        )
    }

    const fn corpus_class(self) -> &'static str {
        match self.scope {
            BenchmarkScope::All => "deterministic_synthetic_v1",
            BenchmarkScope::Graph => "deterministic_synthetic_graph_v1",
        }
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

struct Reporter {
    configuration: Configuration,
    source_coordinate: String,
    corpus_sha256: String,
    run_coordinate: String,
    hardware: String,
    os: &'static str,
    architecture: &'static str,
    toolchain: String,
    rustc_host: String,
    debug_assertions: bool,
    pgo: String,
    available_parallelism: String,
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
        let rustc_host = rustc
            .lines()
            .find_map(|line| line.strip_prefix("host: "))
            .unwrap_or("")
            .to_owned();
        let hardware = std::env::var("PALE_BENCH_HARDWARE").unwrap_or_else(|_| {
            if configuration.smoke {
                "unspecified-smoke-host".to_owned()
            } else {
                panic!("set PALE_BENCH_HARDWARE for a full synthetic characterization run")
            }
        });
        let pgo = std::env::var("PALE_BENCH_PGO").unwrap_or_else(|_| "none".to_owned());
        Self {
            configuration,
            source_coordinate: benchmark_coordinate(
                "PALE_BENCH_SOURCE_COORDINATE",
                configuration,
                "unsealed-smoke-source",
            ),
            corpus_sha256: selected_corpus_sha256(configuration),
            run_coordinate: benchmark_coordinate(
                "PALE_BENCH_RUN_COORDINATE",
                configuration,
                "unsealed-smoke-run",
            ),
            hardware: tsv_value(&hardware),
            os: std::env::consts::OS,
            architecture: std::env::consts::ARCH,
            toolchain: tsv_value(&toolchain),
            rustc_host: tsv_value(&rustc_host),
            debug_assertions: cfg!(debug_assertions),
            pgo: tsv_value(&pgo),
            available_parallelism: parallelism_field(std::thread::available_parallelism()),
            output: RefCell::new(EvidenceOutput::new()),
        }
    }

    fn print_header(&self) {
        self.output.borrow_mut().write_header(&format!(
            "source_coordinate\tcorpus_sha256\trun_coordinate\tevidence_class\tcorpus_class\towner_qualified_actual\tpolicy_selected\tacceptance_budget_selected\trelease_evidence\tfamily\toperation\tsize\tcardinality_or_edges\tpattern\trequested_overlap_permyriad\teffective_overlap\t{}\tseed\thardware\tos\tarch\tobserved_rustc_version\tdebug_assertions\trust_allocator\tpgo_label\tobserved_rustc_host\tavailable_parallelism\trust_driver_threads\tnative_worker_threads",
            Measurement::TSV_HEADER,
        ));
    }

    fn report(&self, case: &CaseDescriptor, measurement: Measurement) {
        let row = format!(
            "{source}\t{corpus}\t{run}\t{evidence}\t{corpus_class}\tfalse\tfalse\tfalse\tfalse\t{family}\t{operation}\t{size}\t{edges}\t{pattern}\t{requested_overlap}\t{effective_overlap}\t{measurement}\t{seed}\t{hardware}\t{os}\t{architecture}\t{toolchain}\t{debug_assertions}\tsystem\t{pgo}\t{rustc_host}\t{available_parallelism}\t1\t",
            source = self.source_coordinate,
            corpus = self.corpus_sha256,
            run = self.run_coordinate,
            evidence = self.configuration.evidence_class(),
            corpus_class = self.configuration.corpus_class(),
            family = case.family,
            operation = case.operation,
            size = case.size,
            edges = case.cardinality_or_edges,
            pattern = case.pattern,
            requested_overlap = case.requested_overlap,
            effective_overlap = case.effective_overlap,
            measurement = measurement.tsv(),
            seed = SYNTHETIC_SEED,
            hardware = self.hardware,
            os = self.os,
            architecture = self.architecture,
            toolchain = self.toolchain,
            debug_assertions = self.debug_assertions,
            pgo = self.pgo,
            rustc_host = self.rustc_host,
            available_parallelism = self.available_parallelism,
        );
        self.output.borrow_mut().write_row(&row);
    }

    fn measure<T>(&self, operation: impl FnMut() -> T) -> Measurement {
        Measurement::sample(self.configuration.sampling(), operation)
    }

    fn measure_prepared<S, T>(
        &self,
        prepare: impl FnMut() -> S,
        operation: impl FnMut(S) -> T,
    ) -> Measurement {
        Measurement::sample_prepared(self.configuration.sampling(), prepare, operation)
    }

    fn measure_retained<S, T>(
        &self,
        prepare: impl FnMut() -> S,
        operation: impl FnMut(&mut S) -> T,
    ) -> Measurement {
        Measurement::sample_retained(self.configuration.sampling(), prepare, operation)
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

/// Bounded streaming custody for one benchmark payload.
///
/// Only the current row and the standard library's bounded output buffer are
/// retained. The SHA-256 state is updated incrementally, so evidence sealing
/// never stores the complete run in memory.
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
        self.writer
            .write_all(line.as_bytes())
            .and_then(|()| self.writer.write_all(b"\n"))
            .expect("benchmark TSV must be writable");
    }

    fn finish(
        mut self,
        expected_rows: usize,
        source_coordinate: &str,
        corpus_sha256: &str,
        run_coordinate: &str,
    ) {
        assert_eq!(
            self.rows, expected_rows,
            "benchmark output must contain its exact feature-aware row count"
        );
        let payload_sha256 = hex_digest(self.payload_digest.finalize());
        writeln!(
            self.writer,
            "# pale-benchmark-seal\trows={}\texpected_rows={}\tpayload_sha256={}\tsource_coordinate={}\tcorpus_sha256={}\trun_coordinate={}",
            self.rows,
            expected_rows,
            payload_sha256,
            source_coordinate,
            corpus_sha256,
            run_coordinate,
        )
        .and_then(|()| self.writer.flush())
        .expect("sealed benchmark TSV must flush");
    }
}

struct CaseDescriptor {
    family: &'static str,
    operation: &'static str,
    size: u32,
    cardinality_or_edges: usize,
    pattern: String,
    requested_overlap: String,
    effective_overlap: String,
}

impl CaseDescriptor {
    fn set(
        operation: &'static str,
        universe: u32,
        cardinality: usize,
        pattern: String,
        requested_overlap: u16,
        effective_overlap: u32,
    ) -> Self {
        Self {
            family: "adaptive_node_set",
            operation,
            size: universe,
            cardinality_or_edges: cardinality,
            pattern,
            requested_overlap: requested_overlap.to_string(),
            effective_overlap: effective_overlap.to_string(),
        }
    }

    fn graph(operation: &'static str, fixture: &GraphFixture, pattern: &'static str) -> Self {
        Self {
            family: "packed_graph",
            operation,
            size: u32::try_from(fixture.graph.node_count()).expect("graph size fits u32"),
            cardinality_or_edges: fixture.graph.edge_count(),
            pattern: pattern.to_owned(),
            requested_overlap: "na".to_owned(),
            effective_overlap: "na".to_owned(),
        }
    }

    fn arena(operation: &'static str, size: u32) -> Self {
        Self {
            family: "typed_arena",
            operation,
            size,
            cardinality_or_edges: size as usize,
            pattern: "linear-bodies".to_owned(),
            requested_overlap: "na".to_owned(),
            effective_overlap: "na".to_owned(),
        }
    }

    fn wake_ring(operation: &'static str, capacity: u32, pattern: &'static str) -> Self {
        Self {
            family: "wake_token_ring",
            operation,
            size: capacity,
            cardinality_or_edges: capacity as usize,
            pattern: pattern.to_owned(),
            requested_overlap: "na".to_owned(),
            effective_overlap: "na".to_owned(),
        }
    }
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
            panic!("set {variable} for a full synthetic characterization run")
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
    coordinate
}

fn selected_set_cases(configuration: Configuration) -> Vec<(u32, Cardinality, SetPattern, u16)> {
    if configuration.smoke {
        vec![(256, Cardinality::Absolute(32), SetPattern::Clustered, 0)]
    } else {
        let cases = characterization_set_cases().collect::<Vec<_>>();
        assert_eq!(cases.len(), CHARACTERIZATION_SET_FIXTURE_COUNT);
        cases
    }
}

const fn selected_graph_sizes(configuration: Configuration) -> &'static [u32] {
    if configuration.smoke {
        &[256]
    } else {
        &[256, 4_096, 65_536]
    }
}

const fn selected_graph_patterns(configuration: Configuration) -> &'static [GraphPattern] {
    if configuration.smoke {
        &[GraphPattern::Parallel]
    } else {
        &[
            GraphPattern::Chain,
            GraphPattern::Layered,
            GraphPattern::Fanout,
            GraphPattern::Disjoint,
            GraphPattern::SccBand,
            GraphPattern::Parallel,
        ]
    }
}

const fn selected_arena_sizes(configuration: Configuration) -> &'static [u32] {
    if configuration.smoke {
        &[64]
    } else {
        &[64, 4_096, 65_536]
    }
}

const fn selected_wake_ring_capacities(configuration: Configuration) -> &'static [u32] {
    if configuration.smoke {
        &[64]
    } else {
        &[64, 4_096, 65_536]
    }
}

fn selected_corpus_sha256(configuration: Configuration) -> String {
    let mut digest = Sha256::new();
    digest.update(b"pale-resident-kernel-synthetic-v1\0");
    digest.update([u8::from(configuration.smoke)]);
    digest.update(configuration.corpus_class().as_bytes());
    digest.update([0]);
    digest.update(SYNTHETIC_SEED.to_be_bytes());

    if configuration.scope == BenchmarkScope::All {
        for (universe, cardinality, pattern, overlap) in selected_set_cases(configuration) {
            digest.update(b"set\0");
            digest.update(universe.to_be_bytes());
            match cardinality {
                Cardinality::Absolute(value) => {
                    digest.update(b"absolute\0");
                    digest.update(value.to_be_bytes());
                }
                Cardinality::Permyriad(value) => {
                    digest.update(b"permyriad\0");
                    digest.update(value.to_be_bytes());
                }
            }
            digest.update(set_pattern_label(pattern).as_bytes());
            digest.update([0]);
            digest.update(overlap.to_be_bytes());
            let members =
                paired_set_members(universe, cardinality.resolve(universe), pattern, overlap);
            digest.update(members.effective_overlap.to_be_bytes());
            digest_indices(&mut digest, &members.left);
            digest_indices(&mut digest, &members.right);
        }
    }

    for &size in selected_graph_sizes(configuration) {
        for &pattern in selected_graph_patterns(configuration) {
            digest.update(b"graph\0");
            digest.update(size.to_be_bytes());
            digest.update(graph_pattern_label(pattern).as_bytes());
            digest.update([0]);
            let edges = graph_edges(size, pattern);
            digest.update(
                u64::try_from(edges.len())
                    .expect("synthetic edge count fits u64")
                    .to_be_bytes(),
            );
            for edge in edges {
                digest.update(edge.id.to_be_bytes());
                digest.update(edge.source.to_be_bytes());
                digest.update(edge.target.to_be_bytes());
            }
        }
    }

    if configuration.scope == BenchmarkScope::All {
        for &size in selected_arena_sizes(configuration) {
            digest.update(b"arena\0");
            digest.update(size.to_be_bytes());
            for value in 0..size {
                digest.update(value.to_be_bytes());
            }
            for body in synthetic_runtime_bodies(size) {
                digest.update(body.identity.to_be_bytes());
                digest.update(body.generation_affinity.to_be_bytes());
                digest.update(body.revision.to_be_bytes());
                digest.update(body.deadline.to_be_bytes());
                for membership in body.memberships {
                    digest.update(membership.to_be_bytes());
                }
            }
        }

        for &capacity in selected_wake_ring_capacities(configuration) {
            digest.update(b"wake-ring\0");
            digest.update(capacity.to_be_bytes());
            digest.update(capacity.saturating_add(1).to_be_bytes());
        }
    }

    hex_digest(digest.finalize())
}

fn digest_indices(digest: &mut Sha256, indices: &[u32]) {
    digest.update(
        u64::try_from(indices.len())
            .expect("synthetic membership count fits u64")
            .to_be_bytes(),
    );
    for &index in indices {
        digest.update(index.to_be_bytes());
    }
}

fn hex_digest(bytes: impl AsRef<[u8]>) -> String {
    let bytes = bytes.as_ref();
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut encoded, "{byte:02x}").expect("writing to a String is infallible");
    }
    encoded
}

fn benchmark_sets(reporter: &Reporter) {
    for (universe, cardinality, pattern, overlap) in selected_set_cases(reporter.configuration) {
        let fixture = SetFixture::new(
            universe,
            cardinality,
            pattern,
            overlap,
            synthetic_set_policy(),
        );
        benchmark_set_fixture(reporter, &fixture, cardinality, pattern);
    }
}

fn benchmark_set_fixture(
    reporter: &Reporter,
    fixture: &SetFixture,
    cardinality: Cardinality,
    pattern: SetPattern,
) {
    let left_handles = fixture.handles(&fixture.members.left);
    let right_handles = fixture.handles(&fixture.members.right);
    let left = fixture.graph.node_set(left_handles.clone()).unwrap();
    let right = fixture.graph.node_set(right_handles.clone()).unwrap();
    assert_set_preflight(fixture, &left, &right);
    let descriptor = |operation| {
        CaseDescriptor::set(
            operation,
            u32::try_from(fixture.graph.node_count()).expect("set universe fits u32"),
            left.len(),
            format!(
                "{}:{}",
                set_pattern_label(pattern),
                cardinality_label(cardinality)
            ),
            fixture.members.requested_overlap_permyriad,
            fixture.members.effective_overlap,
        )
    };

    report_prepared_set_construction(reporter, fixture, &left_handles, &descriptor);
    let needle = *left_handles.first().expect("benchmark set is nonempty");
    reporter.report(
        &descriptor("contains"),
        reporter.measure(|| left.contains(needle).unwrap()),
    );
    reporter.report(&descriptor("len"), reporter.measure(|| left.len()));
    reporter.report(
        &descriptor("iteration"),
        reporter.measure(|| left.iter().count()),
    );
    reporter.report(
        &descriptor("union"),
        reporter.measure(|| left.union(&right).unwrap()),
    );
    reporter.report(
        &descriptor("intersection"),
        reporter.measure(|| left.intersection(&right).unwrap()),
    );
    reporter.report(
        &descriptor("difference"),
        reporter.measure(|| left.difference(&right).unwrap()),
    );
    reporter.report(
        &descriptor("subset"),
        reporter.measure(|| left.is_subset(&right).unwrap()),
    );
    reporter.report(
        &descriptor("disjoint"),
        reporter.measure(|| left.is_disjoint(&right).unwrap()),
    );
    reporter.report(
        &descriptor("union_difference_intersection_chain"),
        reporter.measure(|| {
            left.union(&right)
                .unwrap()
                .difference(&right)
                .unwrap()
                .intersection(&left)
                .unwrap()
        }),
    );
}

fn report_prepared_set_construction(
    reporter: &Reporter,
    fixture: &SetFixture,
    handles: &[pale_kernel::NodeHandle],
    descriptor: &impl Fn(&'static str) -> CaseDescriptor,
) {
    reporter.report(
        &descriptor("construction_from_validated_handles"),
        reporter.measure_prepared(
            || handles.to_vec(),
            |input| fixture.graph.node_set(input).unwrap(),
        ),
    );
}

fn assert_set_preflight(fixture: &SetFixture, left: &AdaptiveNodeSet, right: &AdaptiveNodeSet) {
    assert_eq!(left.len(), fixture.members.left.len());
    assert_eq!(right.len(), fixture.members.right.len());
    assert_eq!(
        left.intersection(right).unwrap().len(),
        fixture.members.effective_overlap as usize
    );
    assert_eq!(
        left.union(right).unwrap().len(),
        left.len() + right.len() - fixture.members.effective_overlap as usize
    );
}

fn benchmark_graphs(reporter: &Reporter) {
    for &size in selected_graph_sizes(reporter.configuration) {
        for &pattern in selected_graph_patterns(reporter.configuration) {
            let fixture = GraphFixture::new(size, pattern, synthetic_set_policy());
            benchmark_graph_fixture(reporter, &fixture, pattern);
        }
    }
}

fn benchmark_graph_fixture(reporter: &Reporter, fixture: &GraphFixture, pattern: GraphPattern) {
    assert_graph_preflight(fixture);
    let descriptor =
        |operation| CaseDescriptor::graph(operation, fixture, graph_pattern_label(pattern));
    report_prepared_graph_build(reporter, fixture, &descriptor);
    let lookup = u32::try_from(fixture.graph.node_count() / 2).expect("lookup index fits u32");
    reporter.report(
        &descriptor("binary_lookup"),
        reporter.measure(|| fixture.graph.find(&lookup)),
    );
    let origin = fixture.origin();
    reporter.report(
        &descriptor("outgoing_adjacency_iteration"),
        reporter.measure(|| {
            fixture
                .graph
                .edges_directed(origin, pale_kernel::Direction::Outgoing)
                .unwrap()
                .count()
        }),
    );
    reporter.report(
        &descriptor("forward_reachability_with_proof"),
        reporter.measure(|| fixture.graph.reachability(origin).unwrap()),
    );
    reporter.report(
        &descriptor("reverse_impact_with_unaffected_complement"),
        reporter.measure(|| {
            fixture
                .graph
                .reverse_impact([fixture.impact_seed()])
                .unwrap()
        }),
    );
    report_precomputed_graph_queries(reporter, fixture, &descriptor);
    #[cfg(all(feature = "graphblas", not(target_family = "wasm")))]
    report_graphblas_queries(reporter, fixture, &descriptor);
}

#[cfg(all(feature = "graphblas", not(target_family = "wasm")))]
fn report_graphblas_queries(
    reporter: &Reporter,
    fixture: &GraphFixture,
    descriptor: &impl Fn(&'static str) -> CaseDescriptor,
) {
    let mut prepared = fixture.graph.batch_reachability();
    let mut canonical = fixture.graph.canonical_batch_reachability();
    assert_eq!(
        prepared.backend(),
        BatchReachabilityBackend::GraphblasCpu,
        "GraphBLAS characterization cannot silently measure the CPU fallback"
    );
    reporter.report(
        &descriptor("graphblas_index_build"),
        reporter.measure(|| {
            let index = fixture.graph.batch_reachability();
            assert_eq!(
                index.backend(),
                BatchReachabilityBackend::GraphblasCpu,
                "every timed GraphBLAS construction must fail closed on fallback"
            );
            index
        }),
    );

    let batch_sizes = if reporter.configuration.smoke {
        &[8][..]
    } else {
        &[1, 8, 64][..]
    };
    for &batch_size in batch_sizes {
        let node_count = u32::try_from(fixture.graph.node_count()).expect("node count fits u32");
        let seeds = (0..batch_size)
            .map(|index| {
                let node = u32::try_from(
                    (u64::try_from(index).expect("batch index fits u64") * u64::from(node_count))
                        / u64::try_from(batch_size).expect("batch size fits u64"),
                )
                .expect("distributed seed remains in the graph");
                vec![fixture.graph.node_at(node).expect("benchmark seed exists")]
            })
            .collect::<Vec<_>>();
        let (forward_label, reverse_label, cpu_forward_label, cpu_reverse_label) = match batch_size
        {
            1 => (
                "graphblas_batch_forward_1_set",
                "graphblas_batch_reverse_1_set",
                "canonical_cpu_batch_forward_1_set",
                "canonical_cpu_batch_reverse_1_set",
            ),
            8 => (
                "graphblas_batch_forward_8_sets",
                "graphblas_batch_reverse_8_sets",
                "canonical_cpu_batch_forward_8_sets",
                "canonical_cpu_batch_reverse_8_sets",
            ),
            64 => (
                "graphblas_batch_forward_64_sets",
                "graphblas_batch_reverse_64_sets",
                "canonical_cpu_batch_forward_64_sets",
                "canonical_cpu_batch_reverse_64_sets",
            ),
            _ => unreachable!("benchmark batch sizes are a closed fixture"),
        };
        let accelerated_forward = prepared
            .query(
                pale_kernel::Direction::Outgoing,
                seeds.iter().map(|seed| seed.iter().copied()),
            )
            .unwrap();
        let canonical_forward = canonical
            .query(
                pale_kernel::Direction::Outgoing,
                seeds.iter().map(|seed| seed.iter().copied()),
            )
            .unwrap();
        let accelerated_reverse = prepared
            .query(
                pale_kernel::Direction::Incoming,
                seeds.iter().map(|seed| seed.iter().copied()),
            )
            .unwrap();
        let canonical_reverse = canonical
            .query(
                pale_kernel::Direction::Incoming,
                seeds.iter().map(|seed| seed.iter().copied()),
            )
            .unwrap();
        assert_eq!(accelerated_forward, canonical_forward);
        assert_eq!(accelerated_reverse, canonical_reverse);
        assert_eq!(prepared.backend(), BatchReachabilityBackend::GraphblasCpu);
        reporter.report(
            &descriptor(forward_label),
            reporter.measure(|| {
                prepared
                    .query(
                        pale_kernel::Direction::Outgoing,
                        seeds.iter().map(|seed| seed.iter().copied()),
                    )
                    .unwrap()
            }),
        );
        reporter.report(
            &descriptor(reverse_label),
            reporter.measure(|| {
                prepared
                    .query(
                        pale_kernel::Direction::Incoming,
                        seeds.iter().map(|seed| seed.iter().copied()),
                    )
                    .unwrap()
            }),
        );
        reporter.report(
            &descriptor(cpu_forward_label),
            reporter.measure(|| {
                canonical
                    .query(
                        pale_kernel::Direction::Outgoing,
                        seeds.iter().map(|seed| seed.iter().copied()),
                    )
                    .unwrap()
            }),
        );
        reporter.report(
            &descriptor(cpu_reverse_label),
            reporter.measure(|| {
                canonical
                    .query(
                        pale_kernel::Direction::Incoming,
                        seeds.iter().map(|seed| seed.iter().copied()),
                    )
                    .unwrap()
            }),
        );
        assert_eq!(prepared.backend(), BatchReachabilityBackend::GraphblasCpu);
    }
}

fn report_prepared_graph_build(
    reporter: &Reporter,
    fixture: &GraphFixture,
    descriptor: &impl Fn(&'static str) -> CaseDescriptor,
) {
    let nodes = (0..u32::try_from(fixture.graph.node_count()).expect("node count fits u32"))
        .rev()
        .map(|key| pale_kernel::GraphNode { key, payload: key })
        .collect::<Vec<_>>();
    let edges = fixture
        .edges
        .iter()
        .rev()
        .map(|edge| pale_kernel::GraphEdge {
            id: edge.id,
            source: edge.source,
            target: edge.target,
            payload: resident_kernel_benchmark::EdgeKind::Dependency,
            reason: edge.id,
        })
        .collect::<Vec<_>>();
    reporter.report(
        &descriptor("canonical_build_from_owned_declarations"),
        reporter.measure_prepared(
            || (nodes.clone(), edges.clone()),
            |(nodes, edges)| {
                SyntheticPackedGraph::build(nodes, edges, synthetic_set_policy()).unwrap()
            },
        ),
    );
}

fn report_precomputed_graph_queries(
    reporter: &Reporter,
    fixture: &GraphFixture,
    descriptor: &impl Fn(&'static str) -> CaseDescriptor,
) {
    let forward = fixture.graph.reachability(fixture.origin()).unwrap();
    let impact = fixture
        .graph
        .reverse_impact([fixture.impact_seed()])
        .unwrap();
    let path_target = forward
        .reached()
        .iter()
        .last()
        .expect("origin is always reached");
    reporter.report(
        &descriptor("reachability_index_build"),
        reporter.measure(|| fixture.graph.reachability_index()),
    );
    reporter.report(
        &descriptor("point_path_reference_full_closure"),
        reporter.measure_prepared(
            || (),
            |()| {
                fixture
                    .graph
                    .reachability(fixture.origin())
                    .unwrap()
                    .path_to(path_target)
                    .unwrap()
            },
        ),
    );
    reporter.report(
        &descriptor("point_path_cold"),
        reporter.measure_retained(
            || fixture.graph.reachability_index(),
            |index| index.path(fixture.origin(), path_target).unwrap(),
        ),
    );
    reporter.report(
        &descriptor("point_path_warm_exact_cache"),
        reporter.measure_retained(
            || {
                let mut index = fixture.graph.reachability_index();
                let path = index.path(fixture.origin(), path_target).unwrap();
                black_box(path);
                index
            },
            |index| index.path(fixture.origin(), path_target).unwrap(),
        ),
    );
    reporter.report(
        &descriptor("point_path_topological_negative"),
        reporter.measure_retained(
            || fixture.graph.reachability_index(),
            |index| index.path(fixture.impact_seed(), fixture.origin()).unwrap(),
        ),
    );
    reporter.report(
        &descriptor("unaffected_len"),
        reporter.measure(|| impact.unaffected().len()),
    );
    reporter.report(
        &descriptor("reason_path_reconstruction"),
        reporter.measure(|| forward.path_to(path_target).unwrap()),
    );
    reporter.report(
        &descriptor("petgraph_topological_sort"),
        reporter.measure(|| toposort(&fixture.graph.petgraph_view(), None)),
    );
    reporter.report(
        &descriptor("petgraph_scc"),
        reporter.measure(|| kosaraju_scc(&fixture.graph.petgraph_view())),
    );
    let node_count = u32::try_from(fixture.graph.node_count()).expect("node count fits u32");
    reporter.report(
        &descriptor("naive_reference_reverse_closure"),
        reporter.measure(|| {
            reference_closure(
                node_count,
                &fixture.edges,
                &[node_count - 1],
                pale_kernel::Direction::Incoming,
            )
        }),
    );
}

fn assert_graph_preflight(fixture: &GraphFixture) {
    let node_count = u32::try_from(fixture.graph.node_count()).expect("node count fits u32");
    assert_eq!(fixture.graph.edge_count(), fixture.edges.len());
    let expected = reference_closure(
        node_count,
        &fixture.edges,
        &[0],
        pale_kernel::Direction::Outgoing,
    );
    let actual = fixture.graph.reachability(fixture.origin()).unwrap();
    assert_eq!(
        actual
            .reached()
            .iter()
            .map(pale_kernel::NodeHandle::index)
            .collect::<Vec<_>>(),
        expected.reached.iter().copied().collect::<Vec<_>>()
    );
    let target = *expected
        .reached
        .last()
        .expect("the reference closure includes its origin");
    let actual_path = actual
        .path_to(fixture.graph.node_at(target).unwrap())
        .unwrap()
        .map(|path| {
            path.edges
                .into_iter()
                .map(|edge| *fixture.graph.edge_payload(edge).unwrap().1)
                .collect::<Vec<_>>()
        });
    assert_eq!(actual_path, expected.edge_path_to(target));
}

fn benchmark_arenas(reporter: &Reporter) {
    for &size in selected_arena_sizes(reporter.configuration) {
        let fixture = GraphFixture::new(size, GraphPattern::Chain, synthetic_set_policy());
        benchmark_arena_fixture(reporter, &fixture, size);
    }
}

fn benchmark_arena_fixture(reporter: &Reporter, fixture: &GraphFixture, size: u32) {
    let values = (0..size).collect::<Vec<_>>();
    let generation = GenerationArena::seal_for_graph(&fixture.graph, values.clone()).unwrap();
    let handle = generation.handle((size / 2) as usize).unwrap();
    reporter.report(
        &CaseDescriptor::arena("generation_seal_from_owned_bodies", size),
        reporter.measure_prepared(
            || values.clone(),
            |values| GenerationArena::seal_for_graph(&fixture.graph, values).unwrap(),
        ),
    );
    reporter.report(
        &CaseDescriptor::arena("generation_get", size),
        reporter.measure(|| generation.get(handle).unwrap()),
    );
    reporter.report(
        &CaseDescriptor::arena("generation_drop", size),
        reporter.measure_prepared(
            || GenerationArena::seal_for_graph(&fixture.graph, values.clone()).unwrap(),
            drop,
        ),
    );
    report_runtime_arena(reporter, size);
    report_scratch_arena(reporter, size);
}

fn report_runtime_arena(reporter: &Reporter, size: u32) {
    let bodies = synthetic_runtime_bodies(size);
    reporter.report(
        &CaseDescriptor::arena("runtime_fill_to_capacity", size),
        reporter.measure_prepared(
            || (RuntimeArena::new(size as usize).unwrap(), bodies.clone()),
            |(mut runtime, bodies)| {
                for body in bodies.iter().copied() {
                    black_box(runtime.insert(body).unwrap());
                }
                (runtime, bodies)
            },
        ),
    );

    let populated = || {
        let mut runtime = RuntimeArena::new(size as usize).unwrap();
        let handles = bodies
            .iter()
            .copied()
            .map(|body| runtime.insert(body).unwrap())
            .collect::<Vec<_>>();
        (runtime, handles)
    };
    let (mut runtime, handles) = populated();
    let mut handle = handles[(size / 2) as usize];
    reporter.report(
        &CaseDescriptor::arena("runtime_get_populated", size),
        reporter.measure(|| runtime.get(handle).unwrap()),
    );
    reporter.report(
        &CaseDescriptor::arena("runtime_remove_and_reuse_populated", size),
        reporter.measure(|| {
            let mut body = runtime.remove(handle).unwrap();
            body.advance();
            handle = runtime.insert(body).unwrap();
            handle
        }),
    );
    reporter.report(
        &CaseDescriptor::arena("runtime_drop_populated", size),
        reporter.measure_prepared(populated, |(runtime, handles)| {
            drop(runtime);
            handles
        }),
    );
}

fn report_scratch_arena(reporter: &Reporter, size: u32) {
    reporter.report(
        &CaseDescriptor::arena("scratch_reset", size),
        reporter.measure_prepared(
            || {
                let mut scratch = ScratchArena::new(size as usize).unwrap();
                for value in 0..size {
                    black_box(scratch.insert(value).unwrap());
                }
                scratch
            },
            |mut scratch| {
                scratch.reset().unwrap();
                scratch
            },
        ),
    );
}

fn benchmark_wake_rings(reporter: &Reporter) {
    for &capacity in selected_wake_ring_capacities(reporter.configuration) {
        benchmark_wake_ring_fixture(reporter, capacity);
    }
}

fn benchmark_wake_ring_fixture(reporter: &Reporter, capacity: u32) {
    let capacity_usize = capacity as usize;
    reporter.report(
        &CaseDescriptor::wake_ring(
            "construction_empty_explicit_frontiers",
            capacity,
            "single-writer",
        ),
        reporter.measure(|| wake_ring(capacity_usize)),
    );

    let mut saturated = wake_ring(capacity_usize);
    for sequence in 1..=u64::from(capacity) {
        publish_wake(&mut saturated, sequence);
    }
    let mut next_sequence = u64::from(capacity) + 1;
    reporter.report(
        &CaseDescriptor::wake_ring(
            "saturated_publication_overwrite",
            capacity,
            "single-writer:saturated",
        ),
        reporter.measure(|| {
            let token = publish_wake(&mut saturated, next_sequence);
            next_sequence += 1;
            token
        }),
    );

    let caught_up = saturated.current_cursor();
    reporter.report(
        &CaseDescriptor::wake_ring("caught_up_read", capacity, "read-only-consumer"),
        reporter.measure(|| saturated.read_after(caught_up).unwrap().len()),
    );

    let retained = retained_wake_ring(capacity_usize);
    let origin = retained.starting_cursor();
    reporter.report(
        &CaseDescriptor::wake_ring(
            "retained_suffix_iteration",
            capacity,
            "read-only-consumer:retained",
        ),
        reporter.measure(|| {
            retained
                .read_after(origin)
                .unwrap()
                .iter()
                .map(|token| token.journal_sequence().get())
                .fold(0_u64, u64::wrapping_add)
        }),
    );
    reporter.report(
        &CaseDescriptor::wake_ring(
            "two_independent_consumer_reads",
            capacity,
            "read-only-consumers:independent",
        ),
        reporter.measure(|| {
            let left = retained.read_after(origin).unwrap().len();
            let right = retained.read_after(origin).unwrap().len();
            (left, right)
        }),
    );

    let mut overflowed = retained_wake_ring(capacity_usize);
    publish_wake(&mut overflowed, u64::from(capacity) + 1);
    let overflow_origin = overflowed.starting_cursor();
    assert!(matches!(
        overflowed.read_after(overflow_origin),
        Err(WakeReadError::DurableReplayRequired { .. })
    ));
    reporter.report(
        &CaseDescriptor::wake_ring(
            "overflow_durable_replay_instruction",
            capacity,
            "read-only-consumer:overflow",
        ),
        reporter.measure(|| overflowed.read_after(overflow_origin)),
    );
}

fn wake_ring(capacity: usize) -> WakeTokenRing {
    WakeTokenRing::new(capacity, WakeEpochV1::new(1), RingCursorV1::new(0), None)
        .expect("synthetic wake-ring capacity is positive and bounded")
}

fn retained_wake_ring(capacity: usize) -> WakeTokenRing {
    let mut ring = wake_ring(capacity);
    for sequence in 1..=u64::try_from(capacity).expect("synthetic capacity fits u64") {
        publish_wake(&mut ring, sequence);
    }
    ring
}

fn publish_wake(ring: &mut WakeTokenRing, sequence: u64) -> pale_kernel::WakeToken {
    ring.publish_after_durable_append(
        JournalSequenceV1::new(sequence).expect("synthetic journal coordinate is positive"),
    )
    .expect("synthetic publication is adjacent and within its coordinate domain")
}

fn tsv_value(value: &str) -> String {
    assert!(
        value.len() <= MAX_TSV_FIELD_BYTES,
        "benchmark metadata fields must not exceed {MAX_TSV_FIELD_BYTES} bytes"
    );
    value
        .chars()
        .map(|character| {
            if matches!(character, '\t' | '\n' | '\r') {
                ' '
            } else {
                character
            }
        })
        .collect()
}

const fn set_pattern_label(pattern: SetPattern) -> &'static str {
    match pattern {
        SetPattern::Clustered => "clustered",
        SetPattern::Strided => "strided",
        SetPattern::FixedScatter => "fixed-scatter",
    }
}

const fn cardinality_label(cardinality: Cardinality) -> &'static str {
    match cardinality {
        Cardinality::Absolute(1) => "one",
        Cardinality::Absolute(4) => "four",
        Cardinality::Absolute(32) => "thirty-two",
        Cardinality::Absolute(_) => "absolute",
        Cardinality::Permyriad(10) => "0.1-percent",
        Cardinality::Permyriad(100) => "1-percent",
        Cardinality::Permyriad(1_000) => "10-percent",
        Cardinality::Permyriad(5_000) => "50-percent",
        Cardinality::Permyriad(9_000) => "90-percent",
        Cardinality::Permyriad(_) => "density",
    }
}

const fn graph_pattern_label(pattern: GraphPattern) -> &'static str {
    match pattern {
        GraphPattern::Chain => "chain",
        GraphPattern::Layered => "layered",
        GraphPattern::Fanout => "fanout",
        GraphPattern::Disjoint => "disjoint",
        GraphPattern::SccBand => "scc-band",
        GraphPattern::Parallel => "parallel",
    }
}
