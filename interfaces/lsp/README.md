# Pale language server

Connect `pale-lsp` to an editor that needs diagnostics, completion, hover, navigation and configuration review for Pale Workspace files. The server uses Pale's compiler for both the explanation and any proposed source correction.

A host editor integration supplies configured sources, maintains one server per workspace folder, and presents profile inspection and configuration-review commands. The language server does not discover editor workspaces by itself.

## Connect an editor

The `pale-lsp` binary communicates over standard LSP stdio. The initializing client must capture the selected sources. Pointing the server at a folder does not make it discover or scan configuration files. After initialization, a standard `workspace/didChangeWatchedFiles` notification may refresh only an exact source URI already registered in that captured set.

The `initialize` request requires `rootUri` and `initializationOptions` with:

| Field | Caller responsibility |
| --- | --- |
| `rootManifest` | Supply the root manifest's logical label relative to `rootUri`. |
| `documents` | Supply the complete captured source set for the compiler request. Each document has `uri`, `languageId`, `role`, and complete `text`. |
| `machineId` | Optional at initialization, required for private effective-profile inspection. Selects a declared machine; it does not establish native readiness. |

Document URIs must remain lexically within the root and have distinct logical labels. Use `pale-toml` for configuration TOML and typed Package data (`package-data`), `pale-page` for a Package page with a `+++` TOML header (`package-page`), and `pale` for plain Package text, with the matching compiler source role. The closed role inventory is owned by [`DocumentRole`](src/document.rs), not inferred from filenames or prose. The [initialization tests](tests/internal/server.rs) show the exact wire shape; decoding that shape is not evidence that the supplied configuration compiles.

Package page completion, hover and navigation use only the header and keep the original source positions, including CRLF and Unicode text. Markdown and the delimiters receive no TOML assistance. An unfinished header can receive structural help within the compiler’s header-size limit; compilation still requires a closed header and a nonempty body. The complete source remains part of the Package release identity.

After initialization, send normal open/change/save/close notifications and watched-file events for the exact configured source set. A changed closed file replaces its saved bytes; a missing, unreadable, invalid, or over-limit source is removed from the effective snapshot and reported through the existing editor error channel. An open buffer remains authoritative until close. Text changes use UTF-16 positions and incremental synchronization. One analysis worker compiles the current snapshot. Incomplete syntax can still receive structural assistance, but cannot manufacture an admitted profile. Profile inspection keeps the selected project and execution workspace distinct from configuration authority and labels retained older results as stale. Query cancellation suppresses delivery; it does not interrupt an API operation already running. Capacity failures return their unit, the known ceiling, the minimum observed demand, and a safe recovery instruction in the LSP response-error data. A field is omitted when its exact value belongs to the SDK rather than this transport.

## Review configuration

The server calls the editor-safe `pale-sdk` façade. It is not a second TOML compiler or a language server for arbitrary programming languages in the repositories on which an agent works.

The private `pale/workspace/configuration-improvement` method carries one human-governed review over the compilation already retained for the exact editor generation and selected profile context. It advances through explained baseline, evaluated and diagnosed candidate, preview, and an explicit accepted, rejected, cancelled, or stale disposition. Invalid current buffers cannot be evaluated through last-known-good evidence, and accepting a preview records no source mutation or native effect; any write remains a separate authority gate.

While a preview is current, the same private method can return a bounded page of change roots and impact records. It can also explain one affected subject and return its exact candidate source occurrences. Every detail request repeats the editor, profile, private instance, and preview coordinates. A stale or cross-workspace request is rejected instead of being answered from another retained compilation.
