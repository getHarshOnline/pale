# Pale MCP server

Use `pale-mcp` when an agent in a coding session needs to know what a workspace configuration means before anyone edits it: what applies to this repository, what a proposed change would affect and why, and which files Pale would generate. Each call answers from the documents and observations the agent supplies, and whether the change goes ahead stays with the workspace's owner. It also provides smaller tools, such as path normalization and schema validation. Calls use the shared command implementation exported by `pale-cli`, not parsed terminal output or a second compiler.

## Connect and discover

With Rust 1.94 or later and the dependencies available, build from this directory in the local checkout:

```sh
cargo build --locked
```

For local development, configure an MCP client to launch `cargo` with `run --quiet --locked` from this directory. This avoids assuming which target directory the enclosing checkout selected. A packaged client instead launches the built `pale-mcp` executable with no arguments. This is a stdio process, not an HTTP server or an interactive shell. The client sends one JSON-RPC request per line and reads protocol responses from standard output; process failures go to standard error.

Pale accepts the `initialize` exchange used by Claude and Codex and self-contained MCP `2026-07-28` requests. Initialization supports `2025-06-18`, `2025-11-25` and `2026-07-28`. The server echoes a supported requested revision; for another revision it proposes `2026-07-28`, which the client must support before continuing.

### Clients that initialize a connection

Send `initialize`, read its response, then send `notifications/initialized`. Subsequent requests use the negotiated revision and can omit protocol metadata. Send each message below as a separate line, waiting for the preceding request's response:

```json
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"example-client","version":"1"}}}
{"jsonrpc":"2.0","method":"notifications/initialized"}
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"compute_path_inventory","arguments":{"style":"posix_relative","paths":["src/./lib.rs"]}}}
```

The first response identifies `pale-mcp`, its negotiated revision and capabilities. The notification receives no response. The tool list supplies names and input schemas. The final call returns the normalized path `src/lib.rs`; the path is input data, not a file opened by the server. Match replies by `id`. Initialization lasts for this process connection and keeps no compilation or review session. A second `initialize` on the same connection is rejected.

### Clients that send self-contained requests

For MCP `2026-07-28` without initialization, every request carries these two metadata fields:

```json
{
  "_meta": {
    "io.modelcontextprotocol/protocolVersion": "2026-07-28",
    "io.modelcontextprotocol/clientCapabilities": {}
  }
}
```

Send these three requests as separate lines, keeping the metadata in each:

```json
{"jsonrpc":"2.0","id":1,"method":"server/discover","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}}}}
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}}}}
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}},"name":"compute_path_inventory","arguments":{"style":"posix_relative","paths":["src/./lib.rs"]}}}
```

Discovery identifies the server and supported revisions; the tool call produces the same result as the initialized example. A self-contained request does not initialize later requests. An initialized client may also send complete `2026-07-28` metadata for an individual request; malformed or unsupported explicit metadata is rejected in either mode. `resources/list` and `resources/templates/list` discover the available contract resources.

## Results and limits

### Review workspace configuration

For a real workspace, use `compile_portable_workspace` with the complete [workspace example](../../docs/workspace.md#try-a-complete-portable-workspace) or your application's captured sources. The discovered schema specifies the root label and each labelled document's text content. The result summarizes the compilation; it does not keep an MCP compilation session. Send the sources again when a later tool needs to reconstruct that compilation.

For configuration review, call `review_configuration` with `action: "preview"`, a complete captured `baseline`, and a `candidate` containing workspace and private sources only. The candidate uses the baseline's Package; it cannot supply another Package tree. The discovered input schema defines these roles. The result contains the SDK-owned `summary` object and the first bounded impact page. Return that complete `summary` object when you finish the review. Use the profile, explanation, and impact tools for further detail. To inspect one review subject, send `action: "inspect"` with the same sources and summary, the subject from an impact entry, and the three advertised bounds. Pale checks the summary before returning the compiler's causal explanation and bounded candidate-source evidence. An incomplete source scan is reported as incomplete. `sourcePageState` is `complete` only after both evidence sections are exhausted. `retained_review_required` means that more evidence exists, but MCP keeps no review handle and cannot accept the SDK's owner-bound cursor. Use a retained SDK or editor review to continue the exact scan.

For a profile entry's specific reason, pass its `explanationHandle` and the same captured compilation sources to `explain_effective_profile_entry`. The result's `explanation` is the SDK-owned evidence, or `null` when the handle does not belong to the freshly compiled coordinates, including the private instance. This is distinct from portable workspace change/impact explanations and does not establish current editor contents or native readiness.

To finish, send `action: "finish"` with the complete sources again, the complete preview `summary`, and an explicit `accepted`, `rejected`, `cancelled`, or `stale` disposition. Pale reconstructs the review and checks the complete summary before returning a terminal result; it retains no MCP review session. Malformed or mismatched summaries fail with the SDK's diagnostic. Matching summaries prove the compiled review facts, not unchanged raw source, an editor generation, or human authorization. Acceptance does not write files; `cancelled` is a review disposition, not interruption of an in-flight computation.

### Discover result schemas

`tools/list` supplies output schemas for compilation, comparison, review, effective profiles, plans and the registered native actions. Keep these schemas with the tool you call rather than reconstructing response shapes from prose. Today `tools/list` returns 23 tools in about 824,000 characters of definitions, about 767,000 of them output schemas, and the input schemas describe 55 of their 390 properties; an agent that reads the whole list spends that much context before its first call. Contract validation is the exception: its `schemaId` selects the successful document shape, so the descriptor does not advertise one generic-object output schema. The SDK's generated contracts own the actual shapes.

### Handle a rejected call

An API-rejected tool call returns a short `code: summary` text block and a structured tool error containing `diagnostic`, `diagnosticIdentity`, `authoringCorrections`, and `authoringLocation`. The nested diagnostic keeps its canonical contract. The short text helps a caller route the failure without repeating the complete diagnostic. Structured content remains the exact detailed source. Corrections preserve the SDK's exact source precondition, replacement, typed path and proof subject; missing evidence is an empty array and a null location.

Source evidence is returned only to the requesting caller, within that request's workspace and disclosure limits. It does not authorize a write, establish current editor contents or broadcast configuration to other agents. Match the current source and review the proposed change before writing. Invalid protocol metadata or tool arguments can instead produce a JSON-RPC error. Inspect that distinction rather than treating every response as a successful tool result. If the server rejects the protocol revision, use a client that supports the selected revision; retrying the same incompatible request will not help.

Malformed tool-argument errors include a bounded decoder class and field path. They do not echo rejected values. An unknown field name is omitted while its containing object remains visible, and `path_truncated` reports when the path cannot fit the transport ceiling.

### Embed a resident connection

The standalone `pale-mcp` process uses the request-local, effect-free command set. Embedders can instead provision a resident attachment with their own authenticated carrier source and response-admission policy, then start the server with `run_with_resident_attachment`. An observation attachment adds only Observe; an operation attachment adds the other resident operation tools. Pale's resident owns their durable state; MCP does not add another task store, replay requests, or treat a client disconnect as cancellation. Observe an uncertain operation before you decide whether another action is safe. When a resident result contains signed records, it exposes them as exact canonical JSON strings, with their immutable storage references reported separately.

The standalone interface offers no durable task, progress, cancellation, resumption, or agent-session control. Input framing and concurrent requests are bounded in both modes; an overloaded process rejects a request rather than queueing work indefinitely.

Only journeys selected for conversational agent use are exposed. MCP transport availability never widens Pale authority, proves resident readiness, or proves a native effect.

For a command-line walkthrough of the same configuration question, see [Inspect your workspace](https://rawthoughts.in/pale/inspect-your-workspace/). For an embedded client that must retain handles and continue source scans, use the [SDK manuals](../../docs/sdks.md).
