# Working on Pale

Pale compiles workspace declarations and reusable Packages so command-line tools, agents, editors and SDKs can use the same rules. It coordinates independently owned applications while each keeps its own data, permissions and records. The Rust application defines the shared behaviour; interfaces and integrations make it available to their callers. Resident operation is incomplete, so check whether a claim concerns configuration, a running service or an observed external action.

## Find the part that owns the change

Read the [README](README.md), [contribution rules](CONTRIBUTING.md) and [source layout](docs/source-layout.md). Follow the shared working contract reached through the machine's global agent entry. The [architecture guide](docs/architecture.md) explains how the components depend on each other: engine behaviour belongs in `crates/`, I/O in `adapters/`, host connections in `integrations/` and reusable methods in `packages/`.

For guidance changes, read the [Package library](packages/README.md) and [authoring guide](docs/package-authoring.md). For workspace or client work, use the [manual](docs/index.md) and [interface guide](docs/interfaces.md). Pale's language server helps author Pale Workspaces. Use repository search and language tools for Rust; before using CodeGraph, confirm its index belongs to this repository.

Use the [shared writing method](packages/publication/clear-writing.md) for comments, command help, errors, commits and release notes. Keep the purpose, conditions and useful examples in the form their reader needs.

## Build and check the changed component

From the repository root:

```sh
cargo build --workspace --locked
cargo test --workspace --locked
cargo fmt --all -- --check
```

These cover the engine and adapters. SDKs, interfaces, integrations and `tools/repository-quality` have separate commands in their READMEs. Follow the [source layout](docs/source-layout.md) for those checks, Clippy and Rustdoc with warnings denied, and contract generation. Change schema source records and regenerate their output.

Test where the behaviour is defined, including failures and uncertain outcomes. Keep output deterministic and limits on accepted inputs explicit. Follow the contribution rules for API documentation, notices and formats that have never been released.

## Shared repository rules

<!-- BEGIN shared repository rules -->

- Review a source file above 1,000 lines and split it by concern before 1,500.
- Keep Rust tests under `tests/`; keep Go's `_test.go` files beside their package.
- Finish a goal with the codebase the same size or smaller unless a requested feature requires growth. Preserve the behaviour, tests, cases and examples.
- Every engine must build from a fresh clone on Linux and macOS.
- Keep generated API reference and build output out of Git; change their source or generator.
- Write comments and API documentation for the code's reader. A crate or module comment first says what it is for in the system, what it builds on and what builds on it, then its purpose, inputs, results and useful edge cases, with the purpose written problem-first in the present tense: the problem it solves and what solves it today. One plain line after that opening says what is not built yet, as the problem now in front of it, and a negative appears only where a reader would expect the opposite: a limit stated as a property of what exists stays ("it runs on loopback with no caller authentication"), while a negative measured against the finished design is a hedge. Keep them self-contained, with links inside the repository.
- Push each repository as one commit whose whole message is `init`, with Harsh as author and committer under the account the folder uses and no `Co-Authored-By` or `Claude-Session` trailers; later commits stay short and plain.
- Describe the present design and research. The repository neither links nor ships lineage: earlier papers, predecessor versions, history tags and the commits behind them.
- Public repository files, including generated text, link within the repository or to public addresses their readers can open.
- A README is the repository's front door: it carries the product account for the technical reader who arrives there, opening with the product's plain name and the problem it solves, in the words its published pages use.
- Name every operation, command, tool, type, field and variable so a reader without its author's context can tell what it does, what it enables and when to use it; docstrings say the same. Keep full-field record schemas for the engine's consistency, named in plain words like everything else, and author each surface's schemas separately, one per intent, in its caller's terms: SDKs and APIs, the command line, the language server and the agent protocols (MCP, A2A, ACP) are different surfaces with different callers, and the agent protocols compose intents over the operations the others expose.
- Write each Markdown paragraph on one source line. Run the repository's formatter with `proseWrap: never` on changed Markdown.

<!-- END shared repository rules -->
