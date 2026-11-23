# SDKs

Embed Pale to compile workspace documents, look up the configuration that applies and compare proposed edits. The language bindings call `pale-sdk`. Choose a binding by the operations your host needs and how long it can keep the returned handles.

## Choose your host

| Your application | SDK and first example |
| --- | --- |
| Rust or a binding built on the Rust API | [Rust SDK](../sdk/rust/README.md#compile-workspace-documents) |
| Python | [PyPale](../sdk/python/README.md#compile-a-workspace-from-memory) |
| Node.js or TypeScript | [PaleJS](../sdk/js/README.md#compile-a-workspace-from-memory) |
| Go with cgo | [PaleGo](../sdk/go/README.md#compile-a-workspace-from-memory) |
| Browser, editor or plugin with WebAssembly | [PaleWasm](../sdk/wasm/README.md#compile-a-workspace-from-memory) |
| Types and JSON validation in TypeScript | [TypeScript contracts](../sdk/js/contracts/README.md) |

Each binding guide explains how to build its local package. Use the TypeScript contracts when you need types and JSON validation without an addon.

## Compile and compare a workspace

Start with the [three-document example](workspace.md#try-a-complete-portable-workspace), then follow the guide for your binding above. It describes a project called `notes` and its `local` execution workspace.

1. Pass the root declaration and both lock documents as labelled bytes. Your application supplies them from files or editor buffers; the portable compiler needs no access to the host filesystem.
2. Compile that source set and keep the returned handle. Read its summary and project contexts; the Rust example prints `notes-workspace: 3 documents` and `notes / local`.
3. Change the project title from `Shared notes` to `Team notes` in a second source set. Keep the IDs and lock documents, compile the candidate, and retain both handles.
4. Compare the handles and read the impact pages. A subject can depend on the edited document while retaining the same compiled value; inspect its before and after values to see what changed.

The [Rust comparison example](../sdk/rust/README.md#compare-an-edited-workspace) continues the compilation program through those queries. The [workspace guide](workspace.md#review-a-proposed-change) also explains local configuration review and how its inputs differ. Keep the handles for subsequent queries: a copied summary or cursor cannot restore them.

## Inspect a contract in your language

After building a binding, run its example to read the diagnostic schema embedded in that build. Save the Node.js example as `example.mjs` in `sdk/js` so `./index.js` resolves to the addon you built.

:::code-group

```rust Rust
use pale_sdk::PaleApi;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api = PaleApi::new()?;
    let schema = api.contract_schema("diagnostic.v1")?;
    println!("{}: {} bytes", schema.schema_uri(), schema.bytes().len());
    Ok(())
}
```

```python Python
from pypale import Pale

api = Pale()
schema = api.contract_schema("diagnostic.v1")
print(schema.schema_uri)
print(len(schema.bytes))
```

```javascript JavaScript
import bindings from "./index.js";

const api = new bindings.Pale();
const schema = api.contractSchema("diagnostic.v1");
console.log(schema.registryId, schema.schemaSha256);
```

:::

## Match the operation to the host

Use portable methods when your application already has the source bytes, such as in an editor. Filesystem and process calls require a binding that exposes the operation and the permissions that call checks. Resident calls also require an authenticated connection. WebAssembly provides the portable subset and preserves 64-bit values without rounding them to JavaScript numbers.

Check the binding's capability list for the method. To check whether a provider can serve the request, use its own status. The build guides produce packages for local use. Publishing them is a separate step.

## Lay out a visual view

[Pale Scene](../sdk/scene/README.md) uses ELK to lay out geometry for your renderer. To build Scene from source, make the `@rtepl/pale` TypeScript declarations from `sdk/js` available to the compiler. A project that uses only Scene’s geometry entry point can run without installing `@rtepl/pale`.

Live organization and change views need handles Pale issues for the intended recipient. PaleJS can consume those handles, but it cannot yet acquire them from the resident. Use the geometry entry point for an integration you can run today.
