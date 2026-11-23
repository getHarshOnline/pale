# PaleWasm

`@rtepl/pale-wasm` is the WebAssembly SDK. Use it from JavaScript or TypeScript to inspect contracts, validate documents, and compile supplied workspace sources. It runs the portable operations of the canonical Rust SDK in WebAssembly.

This is the portable, effect-free subset of `pale-sdk`, not the full native API. It cannot read repository files, start processes, contact the resident or providers, or install, activate, or apply changes. Callers supply source bytes; successful compilation does not prove that a machine or provider is ready.

## Build the local package

Use Rust 1.94 or later, `wasm-pack`, and Node.js 22 or later. From `sdk/wasm` in this checkout, run:

```sh
npm run build
```

The build produces `dist/` in wasm-bindgen's **bundler** format. It may download Rust or WebAssembly build dependencies. A consumer also needs the matching local [`@rtepl/pale-contracts` package](../js/contracts/README.md#validate-a-document). The Contracts and package checks section below describes its ownership and freshness checks. These private `0.0.1` packages are not a promise of registry availability. Building them does not activate Pale.

## Prepare a local consumer

After building both packages, pack them into an existing output directory. Run each `npm pack` from its own package directory and retain the filenames it prints:

```sh
npm pack --ignore-scripts --pack-destination /absolute/path/artifacts
```

In a new empty consumer directory, create `package.json` with `{"private":true,"type":"module"}`. Install the two exact local archives:

```sh
npm install --ignore-scripts --no-audit --no-fund \
  /absolute/path/artifacts/rtepl-pale-contracts-0.0.1.tgz \
  /absolute/path/artifacts/rtepl-pale-wasm-0.0.1.tgz
```

Both archives must come from the same contract bundle. Installing both explicitly avoids asking a registry for this private dependency. Use the Node-hosted example below to check that consumer; a browser application needs a bundler that supports WebAssembly-module imports and its own integration checks.

## First use

Use an ESM consumer with the locally built `@rtepl/pale-wasm` package and its matching `@rtepl/pale-contracts` dependency available. The host must support the bundler distribution's WebAssembly-module imports. It does not expose a separate `init()` step.

Save this as `example.mjs` in that consumer. It prints the selected diagnostic schema's identity, then demonstrates a rejected schema lookup:

```js
import { Pale } from "@rtepl/pale-wasm";

const api = new Pale();
try {
  const schema = api.contractSchema("diagnostic.v1");
  try {
    console.log(
      JSON.stringify(
        {
          schema: schema.registryId,
          sha256: schema.schemaSha256,
        },
        null,
        2,
      ),
    );
  } finally {
    schema.free();
  }

  try {
    api.contractSchema("not-a-selected-schema");
  } catch (error) {
    if (!(error instanceof Error) || !error.diagnostic) throw error;
    console.error(
      JSON.stringify(
        {
          code: error.diagnostic.code,
          message: error.message,
        },
        null,
        2,
      ),
    );
  }
} finally {
  api.free();
}
```

For the Node-hosted check, use Node 22 or later:

```sh
node --experimental-wasm-modules example.mjs
```

Expect `schema: "diagnostic.v1"` and its build-specific hash on standard output, then error code `contract_rejected` on standard error. This is an intentional input failure; correct the requested registry identity rather than retrying it unchanged. Ordinary API failures carry `diagnostic` and exact canonical JSON in `diagnosticJson`. Do not assume every thrown value has those properties: initialization can throw a string, and host/WebAssembly failures are separate. If the host cannot construct an error or attach diagnostic data, that host failure is returned instead of an incomplete Pale diagnostic.

A rejected workspace can also supply `authoringCorrections` and an `authoringLocation`. Corrections contain the exact expected text, replacement, source path, and compiler evidence for a proposed edit. The array is empty when no correction is proved; the location is `undefined` when unavailable. These values do not change diagnostic bytes or apply an edit. Check each value's native class before using it, and release its WebAssembly handle when finished.

Methods run synchronously. Release returned WebAssembly handles with `free()` when finished, and do not reuse handles after freeing or consuming them. The generated [API reference](https://docs.rawthoughts.in/pale/reference/wasm/gazette-page-index) describes method arguments, results, and available getters. For example, `compilePortableWorkspace` consumes its request; do not reuse or free that request afterward. `api.capabilityIdsJson()` lists the operations selected for this build; absence is not permission to recreate them in the host.

For a browser or extension, verify the example through your actual bundler and run compilation in a worker so it does not block the UI. A desktop editor already using the native LSP does not need a second WASM compiler for the same work.

## Compile a workspace from memory

Supply the root manifest's logical relative label and the complete source set from your editor or loader. Each document has a `label` and `Uint8Array` bytes; include the root and the module and lock documents it selects.

With the local package available in the ESM consumer used above, save this as `compile-example.mjs`. It reads the [complete workspace example](../../docs/workspace.md#try-a-complete-portable-workspace) from a source checkout path supplied on the command line:

```javascript
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import {
  Pale,
  PortableWorkspaceDocument,
  PortableWorkspaceRequest,
} from "@rtepl/pale-wasm";

if (!process.argv[2]) throw new Error("Pass the Pale source checkout path");
const folder = pathToFileURL(
  resolve(process.argv[2], "docs/examples/workspace") + "/",
);
const documents = [
  "pale-workspace.toml",
  "resolution.lock.toml",
  "universe.lock.toml",
].map(
  (label) =>
    new PortableWorkspaceDocument(label, readFileSync(new URL(label, folder))),
);
const api = new Pale();
try {
  const request = new PortableWorkspaceRequest(
    "pale-workspace.toml",
    documents,
  );
  const workspace = api.compilePortableWorkspace(request);
  try {
    console.log(
      `${workspace.workspaceId}: ${workspace.sourceDocumentCount} documents`,
    );
  } finally {
    workspace.free();
  }
} finally {
  api.free();
}
```

Run `node --experimental-wasm-modules compile-example.mjs /absolute/path/to/pale`. Expect `notes-workspace: 3 documents`. The request consumes its documents and compilation consumes the request; do not free or reuse those consumed wrappers. The example releases the returned compilation after reading it. Keep it alive longer if you need authoring queries or a comparison with a second compilation. For browser use, replace the Node loader with fetched files or editor buffers.

Portable compilation does not load machine bindings, Package implementations or a resident. The [workspace contract](../../docs/workspace.md) explains the complete selection; [Inspect your workspace](https://rawthoughts.in/pale/inspect-your-workspace/) shows the command-line workflow for an existing configured workspace.

## Contracts and package checks

The JavaScript entry point consumes `@rtepl/pale-contracts/registry` and re-exports `SCHEMA_REGISTRY_ID`, `SCHEMA_MANIFEST_SHA256`, and `SCHEMA_SET_SHA256`. `api.contractBundleIdentity()` reports the bundle embedded in WASM; the package consumer test checks that its manifest and set match the contract package. Neither entry point generates another contract corpus. Import `@rtepl/pale-contracts` directly when you need its standalone schemas, validators, or pure builders; the WASM entry point does not load that validator bundle.

Cargo tests check Rust-side translation laws; `npm test` also exercises the built WebAssembly boundary and the shared conformance fixtures.

The package-consumer gate needs an explicitly selected packed `@rtepl/pale-contracts` artifact and an installed TypeScript compiler:

```sh
npm test -- --contracts /absolute/path/rtepl-pale-contracts-0.0.1.tgz \
  --typescript /absolute/path/typescript/bin/tsc
```

`npm run test:package --` accepts the same arguments for the focused gate. It builds the bundler distribution, packs this package, and installs only those two local artifacts into an isolated temporary consumer, offline and without install scripts. Strict TypeScript compilation and an actual package-name import check the canonical diagnostic and embedded/contract-package identity join, including rejection of a mismatched set coordinate.
