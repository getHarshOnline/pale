# SDK characterization

## Local materialization

Use this opt-in benchmark to measure six local `pale-sdk` stages. It uses the
same cross-leaf one-shot
fixture as the behavioral suite, expands only the declared repository graph,
and runs all native effects inside disposable confined roots.

The deliberately paired cases contain 1, 10, and 100 repositories. They are
not a Cartesian product. For each case the harness measures six real façade
stages: local compilation, all repository plans, durable plan publication,
aggregate saved-plan admission, repository-scoped settlement, and a fresh
compile-to-settlement pipeline.

Before timing, every case must prove:

- repeated compilation preserves the one-shot and per-repository render
  coordinates;
- repeated planning produces byte-identical canonical plans;
- plan counts preserve the renderer-to-planner partition exposed by the public
  summary;
- repeated publication preserves the exact pretty-printed file bytes;
- aggregate admission preserves every repository and plan identity;
- settlement produces exact run-result read-back and clean repository status;
- recompilation and post-settlement planning remain deterministic.

Fixture construction, current-executable hashing, prerequisite compilation,
and prerequisite planning occur outside the corresponding timing interval.
The fresh-pipeline row starts after its disposable source tree exists and ends
when the settled result is available; it therefore measures Pale application,
SDK, and filesystem work, not synthetic fixture generation or temporary-tree
cleanup.

`visible_repository_bytes` is a post-settlement lower bound over regular files
outside `.git`. It is not allocator traffic, block-device traffic, write
amplification, resident memory, or a claim about filesystem durability
hardware. The benchmark selects no acceptance budget, optimization, storage,
installation, activation, release, or resident policy.

Run a five-observation compile-and-execute smoke validation. The fast smoke
gate exercises all six stages for the one-repository case; the 10- and
100-repository cases belong only to the full characterization:

```sh
PALE_LOCAL_MATERIALIZATION_BENCH_SMOKE=1 \
cargo bench --offline --locked -p pale-sdk \
  --bench local_materialization --features bench-local
```

A characterization run uses 201 observations to report an empirical p99, not
a statistical tail guarantee, and requires exact source, run, and hardware coordinates:

```sh
PALE_BENCH_RUN=1 \
PALE_LOCAL_MATERIALIZATION_BENCH_SOURCE_COORDINATE='main@full-commit-id' \
PALE_LOCAL_MATERIALIZATION_BENCH_RUN_COORDINATE='local-run-id' \
PALE_LOCAL_MATERIALIZATION_BENCH_HARDWARE='machine and CPU identity' \
PALE_LOCAL_MATERIALIZATION_BENCH_KERNEL='kernel version' \
PALE_LOCAL_MATERIALIZATION_BENCH_PGO='none or exact PGO profile' \
PALE_LOCAL_MATERIALIZATION_BENCH_CPU_TARGET='exact target-cpu setting' \
cargo bench --offline --locked -p pale-sdk \
  --bench local_materialization --features bench-local
```

The harness writes a checksum-sealed TSV report to standard output and retains
no result in the repository. Rows prove local characterization only; they do
not prove production workload latency, release performance, native-owner
settlement, or resident-service readiness.

## Retained subject inspection

The ignored library tests compare application primitives, application composition,
SDK primitives and SDK composition on the same actual empty, unadopted and wide
portable compiler fixtures. The integration tests and these measurements share
`tests/support/portable_workspace_inputs.rs`; no second fixture generator is used.
Every page's ordered facts, source roles/paths, inspected counts and independent
completion are compared before measurement. Application rows consume lazy input
lookups without collecting them. These are warmed, same-process retained-input
queries, not compilation or authenticated disclosure measurements.

From the SDK root, run separate optimized binaries (use two build jobs):

```sh
PALE_INSPECTION_BENCH_SMOKE=1 cargo test --offline --locked --release --no-default-features --lib portable_workspace::inspection_performance::inspection_latency -- --ignored --exact --nocapture --test-threads=1
PALE_INSPECTION_BENCH_SMOKE=1 cargo test --offline --locked --release --no-default-features --features bench-memory --lib portable_workspace::inspection_performance::inspection_allocations -- --ignored --exact --nocapture --test-threads=1
```

Both require explicit `PALE_INSPECTION_BENCH_` environment values for `SOURCE`,
`HARDWARE`, `KERNEL`, `TOOLCHAIN`, `CPU_TARGET`, `PGO` and `RUN`. The report includes
the executable SHA-256, ordered fixture-byte digest, full subject, raw limits,
per-page/journey counts and raw observations as pretty JSON records delimited by
`INSPECTION_BENCH_BEGIN` and `INSPECTION_BENCH_END`. Each record is written after
measurement; results are not accumulated across cases.
Retain and checksum the complete output alongside the exact source and resolved
feature graph. Five-observation smoke reports no p99. Omitting the smoke variable
selects 201 observations and an empirical p99, not a statistically reliable tail
guarantee. No performance threshold or improvement is asserted.

Single-call latency measures the query harness, not a naked method invocation:
it excludes returned-result destruction but includes outgoing cursor clones and
the harness Position returned alongside the owner's result. Incoming positions
are prepared outside measurement. Whole journeys include continuation transfer,
counter accumulation and intermediate destruction, retaining only final counts.
The application primitive pair performs an extra root-only lookup at its consumed
input tail. The SDK primitive pair does not repeat that root after input completion
and has no composition header, anchor or terminal-owner projection. Counts retain
these ownership and encoded-byte differences; semantic parity is not equal work.

The optional, pinned `allocation-counter` dependency is referenced only by the
ignored allocation test. Its global allocator instruments the allocation build;
the latency feature graph must exclude it. This is not a promise about arbitrary
all-features consumers. Allocation rows measure current-thread traffic, relative
peaks and net retained output, followed separately by create-and-drop cleanup.
They are not RSS, copy volume, graph size or aggregate memory bounds. Whole-journey
peaks are measured directly, never sums of page peaks. Panics invalidate a run.

Prepared-answer projection, prepared-Value canonicalization and prepared-byte
strict parsing are separate attribution rows. The existing canonicalizer itself
creates a temporary validation Value. Their medians or peaks must not be subtracted
to claim an isolated DOM cost. Fixture generation, compilation, parity projections,
reporting and provenance hashing are outside the measured intervals.
Attribution rows omit traversal counts. `preparedCanonicalBytes` describes the
prepared answer's canonical representation, not in-memory input size;
`encodedOutputBytes` is non-null only for encoded outputs, never a zero-size claim
about a parsed Value or mapped fact.

## Direct consumers of a compiler fact

The `direct_dependents` child of the same library-test owner measures the
application and SDK `direct_dependents` queries. A dependent here is a direct
consumer: it names the selected root as an input under its own derivation rule.
This is one hop, not transitive impact or a resident-operation prerequisite.

It reuses the existing minimal and 300-responsibility captured-input helpers.
The minimal workspace manifest source has four direct consumers: responsibility
lowering, organizational translation, experience composition and semantic
sealing. Other cases cover selected workspace facts with no consumers under
default and capped limits, one sparse responsibility-source consumer among many
candidates, and an absent subject. Ordinary parity tests must establish these
fixture shapes; 300 input documents do not imply 300 positive consumers. No
additional fixtures, public API or dependency is added.

Before measurement, the reference enumerates the existing forward relation and
compares every returned typed fact, rule, provenance and canonical row. It also
checks all four compilation coordinates, subject/root, ten projection keys,
actual per-page search comparisons, and complete ordered journeys without skips.
The full reference is dropped before measurement. Only the first, next, terminal
and first-positive incoming positions are retained, at most four per case.

From the SDK root, use the same explicit provenance environment values described
above and separate optimized binaries, with at most two build jobs:

```sh
PALE_INSPECTION_BENCH_SMOKE=1 cargo test --offline --locked --jobs 2 --release --no-default-features --lib portable_workspace::inspection_performance::direct_dependents::direct_dependent_latency -- --ignored --exact --nocapture --test-threads=1
PALE_INSPECTION_BENCH_SMOKE=1 cargo test --offline --locked --jobs 2 --release --no-default-features --features bench-memory --lib portable_workspace::inspection_performance::direct_dependents::direct_dependent_allocations -- --ignored --exact --nocapture --test-threads=1
```

The existing pretty-record reporter and separate timing/allocation measurement
helpers are unchanged. Application rows consume borrowed dependents without
collecting them; SDK rows retain owned typed values and canonical bytes. The
query harness returns an outgoing cursor clone and Position alongside each page.
Stage selection is outside single-call measurement. Whole journeys include
continuation transfers, counts and page destruction and retain only final counts.
This is not naked application-method latency or equal output ownership.

Reports distinguish inspected candidates, returned consumers and actual input
digest comparisons. The latter depend on the standard-library search algorithm,
exclude root lookup work, and are not semantic identities or time guarantees.
Application `canonicalBytes` is null because it has no encoded answer; SDK bytes
include repeated headers and root facts, not all retained memory. Allocation
peaks are measured directly, never summed or inferred by subtracting medians.
Fixtures, compilation, the full forward reference, parity checks and reporting
remain outside measurement. Five samples qualify a smoke run, with no p99,
speed-up or general workload claim; a 201-sample campaign is a separate decision.
