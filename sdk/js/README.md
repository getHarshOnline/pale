# PaleJS

`@rtepl/pale` is the JavaScript/TypeScript SDK for Node.js. Use it to compile workspace configuration, validate a contract document, or explain a proposed change. It calls the canonical Rust SDK, `pale-sdk`. `api.capabilityIds` lists the operations available in this host. Other SDKs and interfaces can support a different set of operations; shared operations keep the same contract identities, meaning, diagnostics, and effect classifications.

## Build the local addon

Build this package with Node.js 22 or later, Rust 1.94 or later, and the npm dependencies pinned by `package-lock.json`. From `sdk/js` in this checkout, with those dependencies available, run:

```sh
npm run build
```

The build creates the native addon, its `index.js` loader and TypeScript declarations. The [JavaScript API reference](https://docs.rawthoughts.in/pale/reference/js/gazette-page-index) describes the public types and operations. Cargo may download dependencies. The examples import `./index.js` from this build; there is no registry installation step or resident startup.

## Compile a workspace from memory

Use `compilePortableWorkspace` when your application already has the source buffers, for example in an editor. Supply a root label and a complete array of `{ label, bytes }` documents. Each label is relative to the logical workspace; each `bytes` value is a Node `Buffer`. Include the root manifest and the module and lock documents it selects.

After building the addon, save this as `compile-example.mjs` in `sdk/js` and run `node compile-example.mjs`. It loads the [complete workspace example](../../docs/workspace.md#try-a-complete-portable-workspace):

```javascript
import bindings from "./index.js";
import { readFileSync } from "node:fs";

const folder = new URL("../../docs/examples/workspace/", import.meta.url);
const documents = [
  "pale-workspace.toml",
  "resolution.lock.toml",
  "universe.lock.toml",
].map((label) => ({ label, bytes: readFileSync(new URL(label, folder)) }));
const api = new bindings.Pale();
const workspace = api.compilePortableWorkspace({
  rootManifest: "pale-workspace.toml",
  documents,
});
console.log(
  `${workspace.workspaceId}: ${workspace.sourceDocumentCount} documents`,
);
```

Expect `notes-workspace: 3 documents`. The returned compilation supports authoring queries and comparison with a separately compiled candidate. In an editor, replace the loader with its buffers. A missing document or invalid workspace produces the API diagnostic rather than a smaller successful result.

For the declared project, execution-setting and machine selections, use the [workspace contract](../../docs/workspace.md). For ordinary command-line use, follow [Inspect your workspace](https://rawthoughts.in/pale/inspect-your-workspace/).

## Inspect a contract

A contract schema describes the fields a document can contain. This example looks up the schema for a Pale diagnostic, then handles an unavailable schema. It reads the addon's embedded data; it does not write files or start a resident service.

After building the addon, save this JavaScript as `example.mjs` in this directory, then run `node example.mjs`:

```javascript
import bindings from "./index.js";

const api = new bindings.Pale();
const schema = api.contractSchema("diagnostic.v1");
console.log(
  JSON.stringify(
    {
      contract: schema.registryId,
      schemaSha256: schema.schemaSha256,
    },
    null,
    2,
  ),
);

try {
  api.contractSchema("not-a-contract.v1");
} catch (error) {
  if (!(error instanceof Error)) throw error;
  console.error(error.diagnostic?.summary ?? error.message);
}
```

The first call returns the schema identity and a SHA-256 digest of its exact bytes. The second reports that the requested contract is unavailable. API failures provide structured details in `error.diagnostic`; use its fields rather than parsing the message. Initialization or host errors can instead be ordinary `Error` values, which is why the example keeps a message fallback.

To check a document, pass its `Buffer` to `api.validateContractDocument(schemaId, bytes)`. Successful structural validation checks the document's shape, not its authority to run an operation. The [API reference](https://docs.rawthoughts.in/pale/reference/js/gazette-page-index) describes the requests and results for this and the other available operations.

## Compile and review safely

Compilation returns an opaque handle: a native object that your program passes back to the API instead of recreating its internal state. Generated TypeScript types reject plain objects and handles of the wrong class. A type assertion can bypass that check, so the native binding also checks class identity at runtime.

`await api.compileOneShot(request)` copies the captured input before returning its promise. A Node-API worker validates the input and renders the outputs; the promise returns a compilation handle or rejects with the API diagnostic. `await api.previewLocalConfigurationImprovement(request)` also runs on the worker pool. It captures two absolute workspace manifests under one common project, execution workspace and machine, then returns the existing review preview. Only the baseline supplies Package source. Capture is not an atomic filesystem snapshot or a view of unsaved editor buffers. Other operations are synchronous and can block an editor's event loop. This worker route adds neither an abort operation nor permission to cause native effects.

Configuration review advances through API stages. Advancing a consuming stage uses up its handle; your program cannot reuse that stage afterward. The separate prepare/commit route lets you retry failed preparation with the same explanation. Commit requires that exact explanation object, not another with an equal digest. Preview queries reuse the saved comparison and profile handles. Accept, reject, cancel and stale describe review outcomes: none writes configuration or grants permission to cause a native effect.

`api.compiledInterfaceIdentity()` reports compiled SDK version, interface and target claims. `api.verifyCurrentExecutableBinding(compilation)` checks the current Node executable against the captured workspace binding. It does not check the loaded addon's bytes. Neither call observes a resident or establishes process health. Local settlement checks this same binding before making changes. A workspace bound to a separate Pale CLI executable cannot be settled directly from Node.

## Inspect organizations {#retained-organization-inspection}

Use this branch when your application needs to browse declared organizations and relationships. For compilation or candidate review, continue with [Compile and review safely](#compile-and-review-safely) instead. The [generated API reference](https://docs.rawthoughts.in/pale/reference/js/gazette-page-index) defines each request, returned record and continuation type; search for `inspectOrganization` to find its complete signature.

### Read endpoints and records

Use `inspectOrganizationEndpoint` to find an exact typed endpoint and `inspectOrganizationIncidentRelations` to read its incoming and outgoing relations. Both query the retained compilation. A found record does not prove effective state or native readiness; an empty page with a continuation does not prove absence.

Use `PortableWorkspaceCompilation.inspectOrganization` to browse organization and responsibility records, then use each returned handle to read its detailed fields and source mapping. Communication, service, projection and genealogy members can be read independently. Every record variant carries its typed payload; u64 intervals and retention use `bigint`, not lossy JavaScript numbers. Source mappings distinguish unmapped records from named empty, incomplete and terminal pages.

### Continue a query {#continue-a-bounded-query}

Record limits are explicit integers from 1 through 256; copy budgets are integers from 1 through the SDK-owned 4 MiB ceiling. No zero default or 257 clamp applies. Changing the owner, record, nested parent, section or original limits rejects a continuation. Errors preserve SDK diagnostics; rejected queries publish no partial page and do not consume the original cursor. Limits count SDK query visits and copied UTF-8 payload, not total native/JavaScript memory, repeated getter copies, source text, encoded bytes or retained compilation size. All handles are factory-only; copied values and digests cannot recreate them. Mutating a JavaScript copy never changes the retained SDK observation.

This is inspection of caller-supplied admitted inputs, not recipient disclosure, resident authentication or native effect. It adds no geometry or transport schema. PaleJS also exposes opaque recipient-qualified inspection result types for trusted application hosts. JavaScript cannot construct them, and the current resident attachment does not yet return them. Do not substitute a portable compilation or public digest for that missing authority-bearing acquisition step.

### Find the same record across a change

`WorkspaceComparison.correspondOrganizationRecord` searches the other retained compilation using the selected record's canonical key. Select `Prior` or `Current` explicitly; a searching cursor is not absence. A match returns the opposite-side entry and the SDK's complete value comparison, including nested fields omitted from shallow rows. `sameRecord` identifies the exact retained owner and record, including independently paged handles, but never equates equal-valued generations. The original pair, selected handle, direction and raw limits bind continuation; copy-budget failure leaves that cursor usable. Neither query authorizes disclosure.

## Verify the package

The package manifest owns the napi-rs build-tool pin. The [contracts directory](contracts/README.md) owns the generated Apache-licensed `@rtepl/pale-contracts` TypeScript package; neither package duplicates Rust compiler, admission, diagnostic or graph policy.

Run `npm test` to rebuild and exercise all eligible operations through the real addon using the shared API test cases. It also checks deterministic declaration generation and installs the packed addon into an isolated temporary consumer. That consumer checks strict TypeScript, calls the addon, rejects forged handles, and verifies that removing the declared Node types breaks type checking. These checks use cached npm dependencies offline; they do not publish the package or install it into a user's workspace. Cargo tests alone do not test this complete JavaScript-to-native call boundary.

The organization checks exercise generated declarations, the native ABI and the same inspection behavior through the installed packed package. Their fixtures are exported from the application and checked against its source bytes; the fixture README describes regeneration. These tests exercise supplied-input inspection and do not grant recipient access to a resident.

The declaration build uses the pinned TypeScript parser to preserve native class identity and hide constructors for factory-created handles. It adds no runtime fields. Plain request/result records remain structural types, and byte fields use Node's `Buffer` through the pinned `@types/node` dependency.
