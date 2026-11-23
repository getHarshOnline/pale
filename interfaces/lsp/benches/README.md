# LSP editor-pipeline characterization

Use this harness to measure the `pale-lsp` editor boundary. It uses the
same private `DocumentStore`, Ropey representation, mutation counters and
analysis debounce clock as the server. It does not expose those mechanics as a
public API or create a second parser, compiler, worker, cache or policy owner.

## Running the harness

Run the full characterization in an optimized build with an exact source
coordinate, caller-assigned run coordinate and concise host description. The
source coordinate must bind the measured file contents, including uncommitted
changes; a commit identifier alone is insufficient for a dirty worktree:

```sh
PALE_BENCH_RUN=1 \
PALE_BENCH_HARDWARE='machine and CPU identity' \
PALE_BENCH_SOURCE_COORDINATE='sha256:exact-source-inventory-digest' \
PALE_BENCH_RUN_COORDINATE='local-run-id' \
cargo bench --offline --locked --features bench --bench editor_pipeline
```

For compile-and-execute validation only:

```sh
PALE_BENCH_SMOKE=1 \
cargo bench --offline --locked --features bench --bench editor_pipeline
```

Smoke mode records five observations per case and is explicitly ineligible as
characterization evidence; its `p99_ns` field is empty. A full run records 201
observations per case.
Results are streamed as TSV to standard output; the harness never creates or
blesses a result file in the repository.

When neither mode is selected, the benchmark executable exits without running.
This keeps ordinary all-target test discovery from fabricating a benchmark run.

## Corpus and cases

The corpus reuses the three portable workspace fixtures owned by the LSP test
suite. The root manifest is padded only with bounded 80-byte TOML comment lines
to exact 4 KiB and 512 KiB source sizes. This keeps source size distinct from a
pathological unbounded-line-width case. The two sizes form a deliberate
small/large pair; the harness does not construct a Cartesian fixture matrix.

For each selected size the harness measures:

- structurally shared snapshot capture;
- the exact contiguous flattening required at the parser boundary;
- one same-width incremental edit inside the comment; and
- one same-size full-document replacement.

One additional case measures an eight-generation superseding debounce burst
and due claim through the same clock used by the server. A preflight asserts
that only the latest generation can be claimed and that the quiet interval is
not shortened.

Every text case records logical bytes touched and the exact lower bound exposed
by the LSP boundary for copied replacement or flattening bytes. The corpus
digest covers fixture bytes, edit results, selected sizes and burst width.

## Evidence ceiling

The TSV payload carries exact source, corpus and run coordinates, host/toolchain
metadata, five or 201 observations, nearest-rank percentiles and an incremental
SHA-256 terminal seal. A successful seal proves only local payload integrity.

With the optional `bench-memory` feature, the same cases additionally perform
one untimed allocation sample using `allocation-counter` 0.8.1 and read the
process-lifetime RSS high-water mark through `nix` 0.31.3. Replace `--features
bench` with `--features bench-memory` in either command above. No additional
corpus, worker or default-server allocator is introduced.

The [measurement owner](../src/benchmark.rs) defines setup/destruction,
current-thread exclusions, relative allocation peaks and supported RSS units.
These are not absolute live-heap or per-operation RSS measurements. Timings
disable counter bookkeeping but still use its allocator wrapper; compare a
separate `bench` run for the ordinary system-allocator baseline. Unavailable
memory fields are empty, never fabricated zeroes. Logical-byte and explicit-
copy counters remain separate; zero logical change makes mutation
amplification undefined.

All cases execute on one calling thread (`worker_threads=1`). The separate
`available_parallelism` column reports host availability, not workers used.

This is synthetic LSP transport characterization. It can reveal source-size
scaling, the incremental/full-replacement delta, flattening cost or scheduling
bookkeeping regressions. Without `bench-memory`, allocation and RSS evidence
remain unavailable. It does not measure parser/compiler latency, operating-system transport,
interactive end-to-end latency or extension-host behavior. It cannot select a
debounce interval, acceptance budget, cache, index, parallelism strategy,
release, installation, activation or native effect.
