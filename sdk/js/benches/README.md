# Native FFI characterization

Use this harness to measure five portable operations shared by the language
SDKs: bundle identity, canonical diagnostic admission,
contract validation, portable compilation, and one retained impact-page query.
It uses the vendored cross-leaf fixtures and proves their exact identities,
diagnostic bytes, compilation digests, and page result before timing.

Run the five-observation smoke check against the existing release binding:

```sh
npm run bench:smoke
```

A 201-observation characterization requires caller-assigned coordinates:

```sh
PALE_FFI_BENCH_SOURCE_COORDINATE='main@full-commit-id' \
PALE_FFI_BENCH_RUN_COORDINATE='local-run-id' \
PALE_FFI_BENCH_HARDWARE='machine and CPU identity' \
node benches/native_ffi.mjs ./index.js ./tests/fixtures/cross-leaf.v1
```

The harness emits a checksum-sealed TSV report to standard output and stores no
result in the repository. Latencies are absolute calls through the Node binding;
they are not a subtraction from a Rust baseline. Byte columns count only
caller-visible payload values that must cross or be observed at the boundary.
They are conservative copy lower bounds, not allocator traffic, ABI framing,
resident memory, or hardware movement. Results prove only local leaf latency and
fixture equivalence; they do not prove equal leaf eligibility, production
workload performance, release readiness, resident admission, or native effects.
