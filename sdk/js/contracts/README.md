# Pale TypeScript contracts

Use `@rtepl/pale-contracts` for TypeScript declarations, standalone validators,
and builders for eligible authored inputs. It is generated from Pale's initial-v1 JSON Schema bundle. Use it to check JSON values without loading the native engine. Rust records and the contract registry define those shapes.

## Validate a document

Build this local private package from `sdk/js/contracts` with Node.js 22 or
later and its pinned npm dependencies available:

```sh
npm run build
```

Save this as `validate-example.mjs` in that directory. The sample is a
structurally valid diagnostic with this build's schema identity; it does not
claim that a real operation failed:

```javascript
import { CONTRACT_IDENTITIES, validate } from "./dist/index.js";

const diagnostic = {
  schema: "pale/diagnostic.v1",
  schema_sha256: CONTRACT_IDENTITIES["diagnostic.v1"],
  schema_version: 1,
  code: "example_missing_project",
  stage: "input_admission",
  severity: "error",
  disposition: "invalid_input",
  summary: "Choose a project before compiling its profile.",
  retry: "after_correction",
  effect: "not_applicable",
  truncation: {
    shortened_fields: 0, omitted_causes: 0, omitted_notes: 0,
    omitted_help: 0, omitted_facts: 0,
  },
};
const result = validate("diagnostic.v1", diagnostic);
if (result.valid) {
  console.log(`${result.value.code}: ${result.value.summary}`);
} else {
  console.error(result.errors);
  process.exitCode = 1;
}
```

Run `node validate-example.mjs`. Expect
`example_missing_project: Choose a project before compiling its profile.`
Replace the sample value with parsed JSON from your application when checking a
real response. Invalid documents report schema errors. Import by package name
in a consumer where this local package is installed; this is not a
published-registry install.

Type declarations describe values at compile time; `validate` checks unknown
runtime values. Neither establishes an operation's semantic admission or its
permission to run. For workspace compilation or API calls, use
[PaleJS](../README.md) or [PaleWasm](../../wasm/README.md).

## Build an authored candidate

`AUTHORED_INPUT_FAMILIES` lists the schema families accepted by
`buildAuthoredInput(family, value)`. The builder validates, copies and freezes
the JSON tree, then returns its canonical JSON alongside the typed value.
It rejects a family that is not an authored input. This is useful when your
application must preserve exactly what it will submit; it performs no I/O,
signing or execution. Submit the candidate through the responsible Pale API
for its semantic checks.

For example, prepare a capability declaration that selects no callable tools:

```javascript
import { buildAuthoredInput } from "./dist/index.js";

const source = { capability_bindings: {} };
const candidate = buildAuthoredInput("capabilities.v1", source);
console.log(candidate.canonicalJson);
console.log(Object.isFrozen(candidate.value));
```

This prints `{"capability_bindings":{}}` and `true`. It is a usable empty
capability document to extend, not an installed provider or a runnable operation.
Changing `source` later leaves the retained candidate unchanged. A consumer
that needs an actual tool must supply its real provider binding before passing
that declaration to Pale for semantic checks.

## Keep the generated contracts current

The generator writes declarations, Ajv 2020 validators, builder eligibility,
and manifest identities into `src/generated`. Its source coordinate records the
exact registry, generator, dependency lock, behavior options, and ordered
output set. This lets the package check freshness from exact inputs instead of
guessing from a version number.

Run `npm test` to compile the checked-in projection, replay the JCS vectors, and
exercise validation and builder confinement. Source freshness is a separate
composition-root check because this independent package does not assume a
parent checkout:

```sh
npm run check:fresh -- --bundle <canonical-agent-surface.v1-directory>
```
