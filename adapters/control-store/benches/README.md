# SQLite benchmark harness

Use this opt-in harness to characterize the unresolved SQLite-WAL storage
profile. It has two separate workload classes:

- `exact_adapter_mechanics_v1` initializes the verified store sources, requires
  `ProfileUnselected`, opens `DeploymentControlSqliteStore` only through its
  crate-private unadmitted constructor, and performs journal/checkpoint/recovery
  operations through `DurableAuthorityStorePort`.
- `sqlite_characterization` applies explicit SQLite pragma candidates to a
  direct connection over the verified deployment schema and the unadmitted
  candidate physical frame. It is not the adapter path.

The exact-adapter lane reports `adapter_path=true`; direct SQLite
characterization reports `adapter_path=false`. Every row still reports
`profile_selected=false`, `storage_admission_complete=false`, and
`release_evidence=false`. The adapter-path fact identifies exercised mechanics,
not an admitted resident-service path. No row selects a profile, acceptance
threshold, checksum, retention or compaction policy, Pale Runtime assembly,
release, or durability claim.

Every row also carries the complete typed storage requirements and the
lane-specific ceiling for each requirement: measured candidate, partially
measured candidate, observed candidate, not exercised, owner-policy admission
required, or Pale-conformance-plus-owner-admission required. The exercised
`pale-authority-frame.v1` and `sha256-lowercase-hex` mechanics are identified
as candidates with their encoded width; their selection flags remain false.
Retention, compaction, backup/restore, power-loss, and owner-qualified workload
proof remain visibly unresolved.

Rows are bounded and buffered until every candidate × 1x/10x/100x case
succeeds. The harness then emits the header, canonically ordered rows, and a
terminal `#pale_control_sqlite_report_complete_v1` line binding the expected
row count, candidate-set digest, and exact preceding report payload. Output
without a reproducing terminal line is incomplete evidence.

## Validation mode

The smoke invocation is intentionally tiny and needs no retained-evidence
coordinates:

```sh
PALE_SQLITE_BENCH_SMOKE=1 \
  cargo bench -p pale-adapter-control-storage --bench control_sqlite --features bench
```

It exercises mechanics only and does not produce selection-grade evidence.

## Retained characterization inputs

A non-smoke run requires all workload, recovery-bound, resource-budget, and
evidence coordinates. Values are operator inputs; the harness does not invent
or normalize them.

```text
PALE_BENCH_RUN=1
PALE_SQLITE_BENCH_WORKLOAD_CLASS
PALE_SQLITE_BENCH_ACTUAL_RECORDS
PALE_SQLITE_BENCH_PAYLOAD_BYTES
PALE_SQLITE_BENCH_CHECKPOINT_BYTES
PALE_SQLITE_BENCH_MAXIMUM_ENTRY_BYTES
PALE_SQLITE_BENCH_MAXIMUM_CHECKPOINT_BYTES
PALE_SQLITE_BENCH_MAXIMUM_CHECKPOINT_CANDIDATES
PALE_SQLITE_BENCH_MAXIMUM_TAIL_ENTRIES
PALE_SQLITE_BENCH_WORKLOAD_LABEL
PALE_BENCH_HARDWARE
PALE_SQLITE_BENCH_SOURCE_ID
PALE_SQLITE_BENCH_BINARY_SHA256
PALE_SQLITE_BENCH_STORAGE_MANIFEST_SHA256
PALE_SQLITE_BENCH_FILESYSTEM
PALE_SQLITE_BENCH_STORAGE
PALE_SQLITE_BENCH_POWER_ENVELOPE
PALE_SQLITE_BENCH_RUN_ID
PALE_SQLITE_BENCH_BUDGET_MAX_INDEPENDENT_RUNS
PALE_SQLITE_BENCH_BUDGET_MAX_OPERATIONS
PALE_SQLITE_BENCH_BUDGET_MAX_PAYLOAD_BYTES
```

When neither mode is selected, the executable exits before opening a store or
running a measurement. This keeps ordinary all-target Cargo test discovery
from fabricating characterization evidence.

`PALE_SQLITE_BENCH_PROFILES` is required only for
`sqlite_characterization`; it must be unset for `exact_adapter_mechanics_v1`.
Each semicolon-separated candidate has this closed form:

```text
SYNC/fullfsync/checkpoint_fullfsync/MODE/wal_frames/group_records/checkpoint_groups
```

The runner expands the supplied base record count to 1x, 10x, and 100x and
uses 201 independently isolated runs per case. Before creating a database, it
projects the complete invocation and rejects any independent-run, operation, or
payload-byte total above the explicit operator budget.

The optional `bench-memory` feature adds two deliberately non-substitutable
memory scopes. `allocation-counter` observes Rust allocations made by the
parent's current thread across a complete case; it excludes SQLite's C heap and
the crash-writer child. `nix::sys::resource::getrusage` reports process-lifetime
RSS high-water marks for the parent and terminated children. Native RSS units
are retained in the report and normalized to bytes. These cumulative values
cannot be read as per-operation ownership or leak proof.

The [memory evidence owner](../src/benchmark/memory.rs) defines relative allocation
balances and unavailable RSS handling. Memory-enabled timing includes counter
bookkeeping across the complete case; compare it only with the reported allocator
and timing treatment visible. It is not an uninstrumented timing baseline.

The explicit-copy column is a static lower bound over source-visible payload
slice copies, reported per independent run. It excludes SQLite binding, VFS,
kernel, allocator, metadata, and string copies. The lower bound and allocator
bytes therefore answer different questions; neither is a substitute for the
existing physical database/WAL observations.

PGO evidence is not a caller label. [`tools/benchmark_pgo.py`](../../../tools/benchmark_pgo.py)
builds ordinary, instrumented, and profile-use binaries; rejects a mismatched
`llvm-profdata`; runs the same complete workload; and binds both comparison
reports and executable hashes in one receipt. The checked-in tool does not
select a performance threshold or turn a local comparison into release proof.

Pooled operation counts remain distinct from independent runs. A p99 field is
blank unless at least 101 independent runs support the row. Checkpoint
projection-commit time and explicit WAL-checkpoint time are separate columns;
all checkpoint status observations contribute to their distributions and
worst-case fields.

The process-loss child skips Rust destructors after committed work. Recovery
evidence therefore covers process loss, reopen, physical frame validation, and
bounded checkpoint-plus-tail reading. It is not power-loss or filesystem
durability proof.

## Assembled resident benchmark

`resident_application` is a separate report lane. It combines the exact
crate-private deployment SQLite candidate with `ResidentApplication`, the real
compiler, contextual generation admission, durable lifecycle transitions, the
session working-set projection, and cold reconstruction. It measures four
operations at each selected synthetic population:

- one pin after preparing `N - 1` retained sessions;
- one middle-session drain after preparing `N` retained sessions;
- cold reconstruction of `N` pins from a checkpoint before the pin tail; and
- cold reconstruction of `N` pins from a checkpoint that includes the pins.

Compiler input capture, generation activation, lease construction, population
setup, SQLite opening, and behavioral assertions are outside each timed
interval. Pin and drain timing starts immediately before the application call
and ends with its owned receipt retained. Reconstruction timing starts with an
already-open exact SQLite candidate and owned generation inputs, and ends with
the complete `ResidentColdOpenOutcome` retained. It therefore includes bounded
checkpoint/tail replay, ordinary generation re-admission, session working-set
rebuild, fresh wake-ring construction, and coordinator resume, but not store
pathname classification/open time.

Here, “cold reconstruction” means rebuilding resident state in the same
benchmark process with ordinary operating-system caches. It is not an OS-cold
startup or process-crash recovery measurement.

Rows report elapsed p50/p95/p99/max, exact session/capacity inputs, recovered
journal/session observations, and main/WAL/shared-memory sizes. This initial
lane does not instrument allocator volume or RSS; those fields explicitly say
`not_measured_by_this_initial_lane`. Physical byte sizes are observations, not
logical ownership, allocation, durability, or leak proof.
The five-observation smoke run leaves p99 blank; only a 201-observation full
row crosses the shared p99 evidence floor.

The compiler and contextual heads are real, but live resident probes,
materialization observations, and session populations are deterministic test
fixtures. Every row therefore keeps `owner_qualified_actual=false`,
`profile_selected=false`, `storage_admission_complete=false`,
`acceptance_budget_selected=false`, and `release_evidence=false`. The benchmark
starts no resident process and performs no native effect. Its `1x`, `10x`, and
`100x` labels scale a caller-supplied synthetic baseline; they are not the
measured or qualified production workload.

Compile and execute the five-observation smoke path:

```sh
PALE_RESIDENT_BENCH_SMOKE=1 \
cargo bench --offline --locked -p pale-adapter-control-storage \
  --bench resident_application --features resident-bench
```

Full characterization uses 201 independently prepared observations per row.
It requires explicit evidence coordinates and workload/read budgets; the
harness checks the complete transition projection and largest recovery tail
before creating a workload:

```text
PALE_BENCH_RUN=1
PALE_RESIDENT_BENCH_BASELINE_SESSIONS
PALE_RESIDENT_BENCH_MAXIMUM_ENTRY_BYTES
PALE_RESIDENT_BENCH_MAXIMUM_CHECKPOINT_BYTES
PALE_RESIDENT_BENCH_MAXIMUM_CHECKPOINT_CANDIDATES
PALE_RESIDENT_BENCH_MAXIMUM_TAIL_ENTRIES
PALE_RESIDENT_BENCH_MAXIMUM_TOTAL_TRANSITIONS
PALE_RESIDENT_BENCH_SOURCE_COORDINATE
PALE_RESIDENT_BENCH_RUN_COORDINATE
PALE_RESIDENT_BENCH_HARDWARE
```

The final line begins `#pale_resident_application_report_complete_v1` and binds
the expected row count, projected transition count, and SHA-256 of every
preceding TSV byte. An incomplete stream is not benchmark evidence.

Kernel synthetic mechanics, physical SQLite characterization, and this
assembled resident path have different preparation and timing boundaries.
Their latencies must not be added or presented as interchangeable measurements.
