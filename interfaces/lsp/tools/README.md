# Private LSP wire generation

This development tool generates TypeScript payload types and structural checks
from the Rust types used by Pale's private editor protocol. It does not run in
the LSP server or change the advertised protocol.

The exporter produces two schema catalogs. `Requests` uses deserialization
rules; `Responses` uses serialization rules. Each catalog shares definitions
through Schemars. Its properties name existing payload aliases, not fields of
an aggregate message. **Never send a catalog as an LSP request or response.**

The generator compiles each catalog once. A payload type is an indexed alias,
such as `Requests["CompilationSummaryRequest"]`. Making catalog keys required
preserves nullable payload types without adding an unintended `undefined`.
Ajv resolves each check from the corresponding catalog property. No script
merges schema definitions, rewrites references, or combines wire directions.

Runtime checks are emitted separately as `requests.mjs` and `responses.mjs`,
with matching declaration files. A receiving client imports `responses.mjs`;
it does not load request or initialization checks. No all-directions runtime
barrel is generated. Payload type aliases and the method map remain type and
development artifacts, not additional runtime policy.

From the repository root, export the schemas with:

```sh
cargo run --offline --locked --manifest-path interfaces/lsp/Cargo.toml \
  --features protocol-codegen --example export_protocol
```

Save that output as a JSON file. From this directory, after making the pinned
development dependencies available, generate into a new empty directory:

```sh
node generate.mjs /path/to/private-schemas.json /path/to/generated
node generate.mjs --check /path/to/private-schemas.json /path/to/generated
node ../tests/protocol_generation.mjs /path/to/private-schemas.json /path/to/worker-values.json
```

Generation rejects external references, unqualified formats, unsupported schema
keywords and nonempty output directories. `--check` compares the complete output
inventory and bytes without rewriting them. Code is readable and not minified.
The generated checks have no Ajv runtime dependency.
They include small bundled helpers from Ajv and ajv-formats. Each runtime module
carries the exact pinned upstream license texts in a preserved comment, so no
adjacent notice file is needed. The generator rejects an unqualified bundled
dependency. Its receipt records the bundled packages and source-license hashes.
The owning test also checks notice retention in a minified downstream bundle.
The test runner invokes the actual generator against the supplied Rust artifact
and hostile copies. It also checks actual worker values and the existing local
Rust-produced profile/improvement fixture. It retains its logs in a temporary
directory, including proof that a rejected nonempty destination was not changed.
The test checks outer-field closure for all three improvement response variants
and the fixture advertisement. For the latter, it restores only the omitted
self-digest from the fixture bytes, following the existing owner/client rule;
this is not a fresh server advertisement or a protocol-admission check.

Produce `worker-values.json` with the existing owning Rust test, from the LSP
repository root:

```sh
PALE_EXPORT_PRIVATE_WIRE=/path/to/worker-values.json cargo test --offline --locked \
  --features protocol-codegen \
  analysis::tests::worker_retains_one_admitted_baseline_comparison_for_typed_queries -- --exact
```

These values prove selected structural exchanges, not exhaustive variant
coverage, stdio delivery or client semantic validation. Keep the independent
client tests for malformed values and request-relative rules.

The same report labels six coordinate boundary cases as serializer-only.
They exercise actual Rust output at zero, above JavaScript's exact integer
range, and the u64 maximum; they do not claim worker admission at those values.
The `uint64-decimal` output format checks canonical spelling and range after
bounding text length. Request coordinate parsing remains a separate contract.

`generation.json` records the exact input artifact hash, generator and lock
hashes, and output hashes. Descriptions are part of the input bytes. A changed
description can change this artifact identity without changing admission law.
This is not a normalized semantic digest or a value added to runtime messages.

Structural checks do not replace the existing client checks for exact keys,
identifier grammar, current/stale evidence, request-relative coordinates, page
limits or valid state/result combinations. Open schema properties have type
`unknown`, not `any`. Passing a structural check does not admit configuration,
establish authority or authorize a write.

The pinned Draft 2020-12 settings have no transforms. An owning test checks that
assumption because extracting transformed definitions would not also transform
inline root schemas. A library upgrade must preserve or explicitly revise this
generation path and rerun actual Rust-wire and hostile client cases.

## Selected CommonJS responses

A CommonJS client can request the response roots it consumes without importing
request validators or the full response catalog:

```sh
node tools/generate.mjs --responses-cjs EffectiveProfileResponse,ConfigurationImprovementResponse schema.json selected-responses
node tools/generate.mjs --check --responses-cjs EffectiveProfileResponse,ConfigurationImprovementResponse schema.json selected-responses
```

This mode emits one TypeScript file per selected root, `responses.cjs`,
`responses.d.cts` and `generation.json`. The manifest identifies the sorted
selection and format.
The validator uses the same schema, library settings, bundled-package checks
and preserved license texts as the default generator. It does not define a new
wire method or semantic policy. The default output inventory is unchanged.

The owning `tests/protocol_generation.mjs` runner exercises selection failures,
actual existing response values, deterministic replay, check mode and notice
preservation. The receiving client owns byte-checked local copying and its
remaining request-relative and producer-value checks; it does not run this
tool through a sibling-private dependency during its ordinary build.
