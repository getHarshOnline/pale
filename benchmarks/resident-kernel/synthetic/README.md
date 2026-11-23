# Resident-kernel synthetic characterization

This directory documents the deterministic, generated corpus exercised by the
feature-gated `pale-kernel` benchmark. The harness writes tab-separated rows
to standard output; it does not create or bless a result file here.

## Evidence ceiling

The corpus contains generated set memberships, graph declarations,
deterministic multi-field arena bodies, and process-local wake coordinates. It contains no
owner-qualified Pale Package, workspace, repository, transport, or request
data. A full run therefore has `evidence_class` set to
`synthetic_characterization` and keeps all of these fields false:

- `owner_qualified_actual`
- `policy_selected`
- `acceptance_budget_selected`
- `release_evidence`

The measurements can reveal algorithmic scaling, representation cliffs, or a
regression worth investigating. They cannot choose an
`AdaptiveNodeSetPolicy`, establish an acceptance budget, represent resident
production traffic, or prove admission, activation, release, or native effect.

## Running the harness

Run the full characterization in an optimized build with a stable, concise host
description, exact source coordinate, and caller-assigned run coordinate:

The source coordinate must bind the current file inventory or snapshot digest;
HEAD alone is insufficient for an uncommitted worktree.

```text
PALE_BENCH_RUN=1 \
PALE_BENCH_HARDWARE='machine and CPU identity' \
PALE_BENCH_SOURCE_COORDINATE='source-inventory-sha256:exact-digest' \
PALE_BENCH_RUN_COORDINATE='local-run-id' \
cargo bench --offline --locked -p pale-kernel \
  --bench resident_kernel --features bench
```

An optional `PALE_BENCH_PGO` value records the selected build treatment. It
does not assert that the binary was built correctly for that treatment.

For compile-and-execute validation only:

```text
PALE_BENCH_SMOKE=1 cargo bench --offline --locked \
  -p pale-kernel --bench resident_kernel --features bench
```

Smoke mode uses five observations and emits
`evidence_class=smoke_validation`. Its output is never characterization
evidence and must not be retained as an accepted benchmark run.

The optional GraphBLAS slice reuses the same corpus and output schema:

```text
PALE_BENCH_RUN=1 \
PALE_BENCH_HARDWARE='machine and CPU identity' \
PALE_BENCH_SOURCE_COORDINATE='source-inventory-sha256:exact-digest' \
PALE_BENCH_RUN_COORDINATE='local-run-id' \
cargo bench --locked -p pale-kernel \
  --bench resident_kernel --features bench,graphblas
```

The canonical dependency and fallback rationale lives on
[`BatchReachability`](../../../crates/pale-kernel/src/graph.rs). The harness
fails if it cannot prepare that optional backend, so a CPU fallback cannot be
mislabeled as GraphBLAS evidence.

The first native build is not intrinsically offline: the pinned
`issundb-graphblas-sys` build script otherwise downloads this exact source:

```text
https://github.com/DrTimothyAldenDavis/GraphBLAS/archive/refs/tags/v10.3.1.tar.gz
sha256 a3c4de775f47d9b448d0f548234a6c321c45f9f6a54e32c9e3a41b28df55cd0a
```

For a network-free build, fetch and verify that archive beforehand, extract it,
and point `ISSUNDB_GRAPHBLAS_SRC` at the extracted `GraphBLAS-10.3.1` root that
contains `Include/GraphBLAS.h`:

```text
ISSUNDB_GRAPHBLAS_SRC='/absolute/path/GraphBLAS-10.3.1' \
PALE_BENCH_RUN=1 \
PALE_BENCH_HARDWARE='machine and CPU identity' \
PALE_BENCH_SOURCE_COORDINATE='source-inventory-sha256:exact-digest' \
PALE_BENCH_RUN_COORDINATE='local-run-id' \
cargo bench --offline --locked -p pale-kernel \
  --bench resident_kernel --features bench,graphblas
```

Cargo's `--offline` flag governs Cargo registry access; it does not by itself
prevent a dependency build script from using the network. The explicit local
source coordinate is therefore mandatory for a truthful clean offline build.

Every row repeats the bounded source, corpus, and run coordinates. The corpus
coordinate is the SHA-256 digest of the exact selected generated declarations,
including set members, graph edges, arena bodies, wake-ring capacities, seed,
and smoke/full choice. The default full run must emit exactly 3,192 data rows. A `GraphBLAS` run adds
126 accelerator rows and 108 matched canonical-CPU batch rows, for exactly
3,426. Smoke emits exactly 39 or 44 rows respectively. The harness fails closed
on a count mismatch.

Output is streamed through a bounded buffer while an incremental SHA-256 is
computed. A successful terminal line records the exact row count, expected row
count, three coordinates, and payload digest. The terminal line is not part of
the digested TSV payload. Its seal proves only local payload integrity; it is
not a repository, admission, activation, release, or publication seal.

## Deterministic corpus

The full harness uses fixed seed `0x4a41525649530001`. Its set slice is the
exact Cartesian product of the dimensions below: 288 fixtures and 2,880 timed
set rows. It covers:

- node-set universes of 256, 4,096, 65,536, and 262,144 nodes;
- absolute cardinalities 1, 4, and 32, plus 0.1%, 1%, 10%, 50%, and 90%;
- clustered, strided, and fixed-scatter membership shapes;
- requested overlaps of 0%, 50%, and 90%, crossed with every other set
  dimension;
- graphs of 256, 4,096, and 65,536 nodes with chain, layered, fanout,
  disjoint-chain, strongly connected band, and parallel-edge topologies; and
- generation, runtime, and scratch arenas of 64, 4,096, and 65,536 bodies.

When two equal-cardinality sets cannot realize the requested overlap because
their combined cardinality exceeds the universe, the generator applies the
mathematical minimum `max(0, 2|S| - |U|)`. Both requested and effective overlap
are emitted, so a dense case cannot masquerade as a physically impossible
disjoint pair.

The integration test in `crates/pale-kernel/tests/benchmark_corpus.rs` uses a
bounded representative subset. It checks set algebra against `BTreeSet`, graph
closure and canonical reason paths against an independent adjacency model, and
arena alignment/staleness laws. The shared generators and reference mechanics
live under `crates/pale-kernel/tests/support/`; no test body or test-only
helper is placed under `src/`.

## Timed operations and statistics

The set slice times validated construction, membership, cardinality, iteration,
union, intersection, difference, subset, disjointness, and an algebra chain.
The graph slice times canonical construction, lookup, zero-copy adjacency,
forward reachability, reverse impact with its unaffected complement, reason-path
reconstruction, `petgraph` topological/SCC views, and the independent reference
closure. When selected, the GraphBLAS slice separately times derived-matrix
construction and forward/reverse batches of 1, 8, and 64 independent seed sets,
with matched canonical-CPU batch rows for each case. The matrix is built once
per query index and reused across observations. The
arena slice times sealing, lookup, drop, runtime fill, populated lookup,
populated slot reuse, populated drop, and
scratch reset. The wake-ring slice times explicit-frontier construction,
steady-state publication at capacity, caught-up reads, retained-suffix
iteration, two independent reads, and the typed durable-replay instruction
after overwrite. The private set-complement primitive is covered through the
public reverse-impact operation rather than exposed solely for measurement.

The wake-ring rows characterize one existing body-free accelerator. They do
not measure writer contention: publication requires exclusive mutable custody
by construction. They also cannot measure machine-wide MCP admission,
single-flight compilation, fairness, hook deadlines, resident IPC, or request
cancellation because no assembled resident currently owns those mechanisms.
Adding simulated rows for them would create benchmark-shaped fiction rather
than evidence. The separate SQLite harness remains the owner for current
journal, checkpoint, process-loss, and bounded-recovery measurements.

A full run records 201 observations per case. The benchmark-owned
[`Measurement`](../../../crates/pale-kernel/tests/support/resident_kernel_measurement.rs)
defines batch-average statistics and the exact destruction scope carried by each
row. Repeated calls include result destruction; consuming construction/drop cases
retain returned output but may destroy consumed input during timing. Point-query
cases retain both the prepared index and result until after elapsed capture.
Thus cached and topologically rejected queries exclude index destruction.
The full-closure point-query reference likewise uses single-call observations
and retains the returned path; its intermediate closure destruction remains
part of the reference algorithm's cost.

The TSV records the running process's OS/architecture and debug-assertion setting,
the separately observed `rustc` version/host, and caller-supplied hardware/PGO
labels. Observing `rustc` does not attest the binary's compilation toolchain or
target. Scheduler availability is separate from the one Rust driver thread;
unavailable scheduler capacity and unobserved native worker counts remain empty.
GraphBLAS's internal OpenMP work is not measured as a worker count. A call over
several independent seed sets is not a concurrent-request throughput measurement.

The harness reports elapsed-time distributions only. It does not claim
allocation volume, peak resident memory, or mutation amplification until a
separate measured allocator/RSS instrument supplies those observations.
