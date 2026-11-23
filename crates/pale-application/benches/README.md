# Immutable Package-text characterization

This non-default `pale-application` benchmark extends the resident-kernel
measurement authority to the private immutable Package-text representation. It
loads the current Markdown tree without changing Package bytes, then exercises
six deliberately paired rather than Cartesian cases across the actual, 10x,
and 100x corpus sizes, Package reuse fan-out, repository count, heading
transformation, and whole-file or managed-block materialization boundaries.

Repository count and fan-out remain separate physical dimensions. Every
repository owns one recipe and one contiguous boundary output; fan-out repeats
the same admitted-looking source closure within that repository output. The
benchmark never collapses repositories and consumers into one repetition count.

The eager reference and lazy recipe interpreters must produce identical bytes
and digests before a case is timed. Reported `explicit_text_copy_lower_bound`
counts only source/output text copies visible in the measured Rust mechanics;
hash reads, allocator internals, operating-system copies, and source-fixture
construction remain outside that lower bound. Allocation evidence is
current-thread Rust allocator evidence. RSS is the process-lifetime high-water
mark and is not an operation-local delta.

`cold` and `warm` qualify only the recipe's memoized materialization state;
they do not claim cold CPU, filesystem, page-cache, or allocator state. Source
capture, fixture construction, reference outputs, and the byte-equivalence
gate remain outside every timed and allocator-counted operation. Those retained
fixtures can contribute to the explicitly process-lifetime RSS high-water mark.
Elapsed time ends after the operation result is constructed or observed and
before that result is dropped; allocator evidence separately includes result
destruction inside its measured closure.

Allocation prerequisites are prepared outside the measurement interval. An
operation that consumes and drops such owned input may therefore report a
negative signed retained count or byte balance. Relative peaks are not absolute
live-heap measurements; see the owning [`AllocationEvidence`](../src/packages/text_benchmark.rs).
Timing disables counter bookkeeping, but a `bench-memory` binary still uses the
reported system-allocator wrapper, not the plain feature-disabled allocator.

Input traversal is bounded to depth 16 and 16,384 filesystem members; at most
4,096 Markdown documents and 2 MiB of actual UTF-8 text are retained. Every
expanded fixture and complete repository-output set is capped at 256 MiB, and
the recipes for one case at 2,000,000 total parts, before the corresponding
large allocation. Symlinks and non-UTF-8 paths fail closed. These are
harness-safety ceilings, not Package, compiler, or release limits.

With `Q` denoting the exact output byte count, the measured lower bound is `Q`
for eager flattening, cold recipe materialization, and a final owned boundary
copy. It is zero for store sealing, recipe construction, warm memoized access,
and digest reads. Mutation amplification is undefined here because the immutable
workload has zero logically changed bytes.

The current Package tree is real local source, but the benchmark does not run
Package admission or bind a selected workspace closure. Consequently even the
actual-size case keeps `owner_qualified_actual=false`. Deterministic 10x and
100x replication is synthetic characterization. No row selects storage,
materialization, cache, admission, acceptance, release, or runtime policy.

Run a compile-and-execute smoke validation with allocation and RSS evidence:

```text
PALE_PACKAGE_TEXT_BENCH_SMOKE=1 \
cargo bench --offline --locked -p pale-application \
  --bench package_text --features bench-memory
```

Smoke mode uses five observations, emits `evidence_class=smoke_validation`, and
omits p99. A characterization run requires explicit source, run, and hardware
coordinates and uses 201 observations so p99 is meaningful:

```text
PALE_BENCH_RUN=1 \
PALE_PACKAGE_TEXT_BENCH_SOURCE_COORDINATE='sha256:exact-source-inventory-digest' \
PALE_PACKAGE_TEXT_BENCH_RUN_COORDINATE='local-run-id' \
PALE_PACKAGE_TEXT_BENCH_HARDWARE='machine and CPU identity' \
PALE_PACKAGE_TEXT_BENCH_KERNEL='kernel version' \
PALE_PACKAGE_TEXT_BENCH_PGO='none or exact PGO profile' \
PALE_PACKAGE_TEXT_BENCH_CPU_TARGET='exact target-cpu setting' \
cargo bench --offline --locked -p pale-application \
  --bench package_text --features bench-memory
```

The source coordinate must bind current content, including uncommitted changes;
HEAD alone does not identify a dirty-tree measurement.

When neither mode is selected, the executable exits without reading Package
source or running a measurement. This keeps ordinary all-target Cargo test
discovery from fabricating characterization evidence.

The benchmark writes a checksum-sealed TSV report to standard output and does
not bless or retain results in the repository. It measures immutable Package
text only. Mutable LSP editing, complete compiler/render/materialization paths,
filesystem staging, and native-owner payload resolution require their own
owner-native measurement seams and must not be inferred from these rows.
