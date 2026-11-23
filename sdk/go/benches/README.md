# Native FFI characterization

Use this command to measure the portable operations shared by the language
SDKs: bundle identity, canonical diagnostic admission,
contract validation, portable compilation, and one retained impact-page query.
It reuses the vendored cross-leaf fixtures and proves their exact identities,
diagnostic bytes, compilation digests, and page result before timing.

Run the five-observation smoke check against the local cgo binding:

```sh
go run ./benches -smoke
```

A 201-observation characterization requires caller-assigned coordinates:

```sh
PALE_FFI_BENCH_SOURCE_COORDINATE='main@full-commit-id' \
PALE_FFI_BENCH_RUN_COORDINATE='local-run-id' \
PALE_FFI_BENCH_HARDWARE='machine and CPU identity' \
go run ./benches
```

The command emits a checksum-sealed TSV report to standard output and stores no
result in the repository. Latencies are absolute calls through the Go/cgo
bridge, not a subtraction from a Rust baseline. Byte columns count only
caller-visible payload values that must cross or be observed at the boundary.
They are conservative copy lower bounds, not allocator traffic, ABI framing,
resident memory, or hardware movement. Results prove only local leaf latency and
fixture equivalence; they do not prove equal leaf eligibility, production
workload performance, release readiness, resident admission, or native effects.

Each case has three untimed warmups. A portable compilation is closed after
each warmup and after each sample's elapsed time is recorded; its disposal is
not part of the compile-call latency. The law-gate workspaces are closed once
their independently retained comparison has been checked, and the comparison
is closed after reporting. The benchmark does not retain a compilation per
sample until client shutdown.

## Go response allocations

Measure Go allocations for bundle lookup, schema lookup, and a rejected schema
lookup with the native library already built:

```sh
go test ./tests -run '^TestContractResponsesOwnTheirBytes$' \
  -bench '^BenchmarkContractResponse$' -benchmem -count=5
```

The benchmark includes each complete call and a byte-equivalence check, but
excludes client creation. Allocation counts cover Go memory, not Rust memory
or resident storage. The accompanying test checks that returned bytes remain
independent across calls and survive client closure. Use the same native
artifact for before-and-after comparisons of the Go response path.
