# Pale CLI

Use the `pale` command to check workspace configuration, inspect generated plans, or run a selected built-in computation. The commands below describe the interface directly and assume its declared Rust dependencies are available. For a task walkthrough instead, start with [Inspect your workspace](https://rawthoughts.in/pale/inspect-your-workspace/) or [Add coding instructions to a repository](https://rawthoughts.in/pale/materialize-repository-guidance/). The [architecture](../../docs/architecture.md) explains why compilation, review and application are separate operations.

With Rust 1.94 or later and the dependencies available, run these commands from this directory to build the CLI and discover its commands and action inputs:

```sh
cargo build --locked
cargo run --quiet --locked -- --help
cargo run --quiet --locked -- action --help
cargo run --quiet --locked -- action list
cargo run --quiet --locked -- action describe pale.compute.path-inventory
```

`action list` reads the compiled catalog and shows each effect-free built-in computation in stable identity order. `action describe <id>` shows the exact registry contract and payload schema; it does not maintain a second CLI-specific action definition. Add the global `--result-format json` option before `action` for machine-readable output. The existing `action <id>` form runs the selected computation from standard input or a confined input file.

Workspace commands require the workspace manifest, project, execution workspace, and machine selections shown in help. These identities are not interchangeable. `action` can run without a workspace. `plan` saves one plan file per repository in stable compiled order, but does not apply those plans. Its `--out` path must be an absolute directory without `..`; Pale creates the directory when it is absent. `apply` and `remove` are separate operations that require saved plans to match freshly rebuilt plans before changing targets.

## Diagnose a local installation

`pale doctor` checks the selected workspace, its lock and bindings, installed repository files and explicitly selected build directories. It reports missing declarations and unavailable runtime observations separately from failed checks, without repair or cleanup. See [Diagnose a local installation](../../docs/doctor.md) for selectors, build-size units, limits and exit status. A complete report does not establish resident readiness.

## Check and read Package sources

Use the compiler's Package reader without selecting a workspace:

```sh
cargo run --quiet --locked -- packages check --root ../..
cargo run --quiet --locked -- packages show pale.design:animate --root ../..
cargo run --quiet --locked -- packages check --write --root ../..
```

`--root` names the distribution folder containing `packages/`. `check` reads every family, checks declarations and reading links, and reports stale indexes. `show` presents a body's declared required and optional reading before its authored method. Omit the resource to inspect the complete catalog as JSON. Add the global `--result-format json` option for a machine-readable body or check result.

Add `--caller-root /absolute/notes --caller-root /absolute/repositories` to check incoming Markdown links and qualified resource IDs from those trees. `--exclude-caller Records` skips that directory name at any depth; longer exclusions name a subtree relative to each root. A broken caller prevents index writes. The [authoring guide](../../docs/package-authoring.md) explains capture limits, default exclusions and which references are checked.

`check --write` regenerates only family `INDEX.md` files. It checks sources and all index preconditions before writing, replaces each file atomically and reads the result again. It preserves authored method bodies. The update is not a transaction across files: if a write fails, inspect the reported outcome before retrying. A successful source check neither selects the guidance for a project nor activates runtime permissions.

## Review and install Package guidance before activation

`guidance inspect` projects selected Package judgment through the source-only API, before a runtime execution path exists:

```sh
cargo run --quiet --locked -- --workspace /absolute/workspace/pale-workspace.toml \
  --project repository --execution-workspace local-guidance --machine local \
  guidance inspect --repository source --host codex
```

Select `--host claude-code` for Claude's layout. The human result lists proposed files and a separate contribution to the project instruction file. Add the global `--result-format json` option to inspect the complete SDK-owned file bodies, paths and hashes. Inspection reads inputs and writes nothing; it does not replace local project facts, observe installed-host readiness or grant tool invocation. Operations, recipes, modes and lifecycle delivery/gates require the runtime route and are refused here.

Both hosts contribute to one `AGENTS.md`; their native skills and references retain separate paths. Each owns only its marked entry contribution, so removal preserves the other host and surrounding project text. Pale leaves independent `CLAUDE.md` instructions alone. For Claude's native loading setting and migration from an earlier Pale-owned `CLAUDE.md` contribution, follow the [guidance starter](../../docs/examples/guidance-workspace/README.md#load-the-shared-entry-in-claude-code).

With the same workspace selections, `guidance context --repository source` prints private repository scope and the session's declared source reading order. Each entry shows its configured machine authority root and relative reference separately; each reading root needs an explicit `read-only` authority-access declaration. Use permitted native tools to distinguish a directory root from an exact-file root. Relative references resolve beneath a directory, or from a bound file's parent while remaining confined to that exact file. The command writes no files and does not read the referenced documents. Keep its output within the project's disclosure scope: it contains private paths and is separate from the portable Package files and `AGENTS.md` contribution. No host option is needed. Without a reading order for this repository's session entry, the output says so; paths alone establish neither ownership nor acceptance of a duty. JSON output contains the same text as a string in the ordinary result envelope.

`guidance plan --repository source --host codex` compares the selected methods with the checkout. It lists files to add, update, keep or remove and shows a text diff with surrounding context. `--intent apply` is the default; use `--intent remove` to review removal of that host's owned files and exact entry contribution. The plan binds source selection, native root, file/parent identities, prior bytes, modes and write order. Conflicting unowned or edited targets are refused. JSON includes the observed UTF-8 text, exact proposed bytes, hashes and modes, including preserved local instructions; keep it within the project's disclosure scope.

After displaying the complete proposal, the CLI saves only its identity in `.pale-source-guidance-review-codex.json` at the target root, with mode `0600`. Claude Code uses `.pale-source-guidance-review-claude-code.json`. Each host's latest successful plan replaces its earlier review, including a change of intent. Instruction files remain untouched. This file remembers what was shown; it does not establish that a person approved it. After reviewing the output, apply with the same workspace selections, repository and host:

```sh
cargo run --quiet --locked -- --workspace /absolute/workspace/pale-workspace.toml \
  --project repository --execution-workspace local-guidance --machine local \
  guidance apply --repository source --host codex
```

For Claude Code, use `--host claude-code` in both commands. For removal, use `--intent remove` in both. Each apply rebuilds the source profile and observes the target again; changed inputs are refused before target writes. After a successful install, run another plan before applying again: it should show `keep`, and applying that unchanged proposal writes no guidance files. If the remembered review is missing, run plan first. Automation may still pass `--expect-plan DIGEST` explicitly; it uses the same checks. Neither saved JSON nor a retained SDK plan bypasses them. Successful publication confirms file read-back, commit and journal cleanup. It does not establish that a coding host used the guidance, or activate runtime tool admission.

A pending interruption journal blocks normal writes. Review recovery using `guidance recovery plan --repository source --host codex --intent restore-before` and execute that exact decision using `guidance recovery apply` with the same selectors, intent and `--expect-recovery REVIEWED_SHA256`. Both commands also accept `--intent finalize-published`: that choice retains a complete observed after-state and finishes cleanup; it does not fill missing writes. A restore keeps independent edits and conditionally removes only recorded empty directories. Conflicting state is refused for owner adjudication. A matching uncheckpointed after-image establishes data, not who wrote it. JSON recovery review includes full proposed bytes and evidence; `null` means no pending journal was observed, not that the repository or host is ready.

The plan includes a managed block in the root `.gitignore`, excluding `.pale-source-guidance.lock`, `.pale-source-guidance-recovery.json` and both review files. The block is applied with the guidance and shown in its diff; existing rules and file permissions are preserved. This works in ordinary repositories and linked worktrees without modifying Git's administrative directories. An edited or malformed managed block stops planning so user additions cannot be silently overwritten. The exclusions remain after guidance removal because the cooperating lock and latest review remain. Planning before the first apply can leave a review file visible to Git; it contains only a format identifier and digest, not instruction text.

The temporary recovery journal contains private before-images and stays when recovery is needed; retain it until reconciled. Diagnostics distinguish writes that never started, confirmed restoration and uncertain outcomes, with coordination effects reported separately. An uncertain outcome requires native inspection before another decision, rather than a blind retry.

The [repository-guidance starter](https://rawthoughts.in/pale/materialize-repository-guidance/) supplies the source declarations and setup steps. `distribution-info --lockfile` prints the native inspector's exact TOML lock to stdout, without consuming an existing lock. Review that proposed lock before replacing a workspace pin. With `--result-format json`, the result is the same TOML document as a JSON string in the ordinary envelope. No lock-writing authority is implied.

Runtime `plan` and `apply` retain their separate admission requirements; a source-guidance result is not their input.

## Review a configuration change

Keep the current and candidate manifests in separate local source roots. Use the same project, execution workspace and machine selection for both:

```sh
cargo run --quiet --locked -- --workspace /absolute/current/pale-workspace.toml \
  --project demo --execution-workspace default --machine local \
  review-configuration --candidate-workspace /absolute/candidate/pale-workspace.toml \
  preview > review.json
```

The result is an indented JSON summary. Pale reads Package sources from the baseline only; the candidate uses that same Package meaning. These disk reads do not include unsaved editor buffers.

Inspect a bounded page of affected and unaffected facts using that same summary:

```sh
cargo run --quiet --locked -- --workspace /absolute/current/pale-workspace.toml \
  --project demo --execution-workspace default --machine local \
  review-configuration --candidate-workspace /absolute/candidate/pale-workspace.toml \
  impacts --limit 10 < review.json
```

Use the returned `nextOffset` as `--offset` for the next page; `null` means the end. Every invocation reconstructs the review and checks the complete original summary before returning a page. Changed compiled review coordinates are rejected, including private-instance changes. Zero or omitted `--limit` uses the SDK default, and the SDK caps larger limits. These checks do not establish unchanged source bytes, an editor revision, or native readiness. `Affected` describes dependency reachability, not necessarily a changed target output.

After inspecting the preview, repeat the command with `accept < review.json` instead of `preview > review.json`. Pale recaptures and evaluates the inputs, then compares the complete summary before returning the selected decision. `reject`, `cancel` and `stale` use the same input. Use `stale` only when you have separately established that the relevant source or conversation changed.

None of these review decisions writes configuration. A changed summary is rejected: inspect a new preview before deciding again. A separately authorized write must still check its source bytes immediately before applying the change.

## Use an attached resident

The ordinary `pale` executable exposes no resident operation commands. Its `hook` command can connect using an explicitly selected protected configuration, described below; it discovers no endpoint. A trusted embedder can provision a `ResidentAttachment` with its authenticated carrier source and response-admission policy, then call `run_with_resident_attachment`. The [Rust SDK reference instructions](../../sdk/rust/README.md#build-the-method-reference) lead to `ResidentAttachment`, `AuthenticatedResidentCarrierSource` and `ResidentResponseAdmission`, the types the embedder must implement or supply. An observation attachment enables only `resident observe`; an operation attachment enables the complete command tree below. The ordinary executable neither advertises nor accepts these commands:

```text
pale resident observe ...
pale resident invoke start|advance ...
pale resident consult start|advance ...
pale resident delegate start|advance ...
pale resident cancel ...
```

Each command reads one already signed common request from `--signed-request`. It never creates a signature or opens resident storage. Start results return a durable handle before dispatch. Pending and uncertain results are not terminal; observe or reconcile them before another attempt. When command output contains signed records, it exposes them as exact canonical JSON strings, with their immutable storage references reported separately.

An embedder can supply either a hook-only or operation attachment for `pale hook`. An observation attachment cannot deliver hooks. The ordinary executable reads one native event from standard input and accepts an explicit connection configuration:

```sh
pale hook session-start --workspace WORKSPACE --repository REPOSITORY \
  --provider-binding BINDING --provider-binding-identity DIGEST \
  --resident-hook-config /absolute/private/hook.json < native-event.json
```

The closed JSON document requires `schema_version: 1`, absolute paths in `endpoint_configuration` and `caller_credential`, trusted `caller_public_key` and `resident_public_key` pins as arrays of 32 bytes, and `timeout_ms` between 1 and 30000. The [endpoint configuration](../../adapters/uds/src/configuration.rs) names the socket. The credential file contains a separately provisioned raw 32-byte Ed25519 seed matching the caller pin. Configuration and credential files must be regular, single-link, mode `0600`, with owner-private immediate parent directories and no symlink traversal. Loading creates no keys or directories and changes no permissions. These filesystem checks do not protect against other processes with the same user ID, inspect ACLs or protect backups; file custody must be selected and qualified for the deployment. Never put a private key in repository hook configuration.

The connection authenticates the pinned resident before sending the native event, preserves its exact bytes and static route, and sends at most once within the selected deadline. The config option is a local connection choice; it is not added to the ten routing arguments sent to the resident. Missing or unsafe configuration and failed authentication block delivery. A refusal preserves the resident's reason. A lost or malformed reply leaves the outcome uncertain and requires reconciliation.

> [!IMPORTANT]
>
> **The file configuration supplies a connection, not permission to run a tool.** It has no independent response authority, so the ordinary executable continues to block native work with exit status 2. It supplies no resident operation commands, signed-record access or protection against a host skipping its hook.

An embedder can instead supply the SDK's `provision_admitted_hooks` attachment with independent response admission. After the resident saves preparation and dispatch and rechecks current authority, and the caller admits the exact response, the embedded hook command writes its native standard error and standard output and returns its selected exit status. It adds no result envelope. Production authority, deployment integration for interrupted-response recovery and proof through actual agent hooks remain unfinished.

## Results and failures

`pale compiled-identity` reports the SDK version, interface protocol, and build target without reading a workspace or verifying installed bytes. It is an executable-handshake response, not a resident health check.

Add `--result-format json` to a supported command to receive exactly one JSON object on standard output:

```json
{
  "completion": "complete",
  "result": {}
}
```

`result` is either the exact structured SDK or dispatcher value, or a typed CLI transport projection when the command combines several owner results. `completion` is `complete`, `partial`, or `non_terminal`. It describes whether that result is complete, not whether its facts are favorable. A conclusive failed-no-effect materialization result is complete; a bounded page with a continuation is partial; a configuration preview awaiting a decision is non-terminal.

This format is available for doctor diagnosis, compiled identity, control-store inspection, Package distribution, source-guidance inspection/planning/publication/recovery, configuration review, built-in action discovery and execution, compilation summary/validation, materialization results, resident operations supplied by an embedder, saving plan files, and surface-status reporting. An independently admitted repository-hook response keeps the native host's output format and exit status instead of using this envelope.

The `plan` result describes the selected plan kind (`apply`, `adopt`, or `remove`) and the plan files saved in stable repository order. The `publications` field lists those completed saves with their paths and operation counts. If saving stops, `stopped_repository_id` names that repository and `diagnostic` explains the failure. Every listed file was durably saved before the result was returned.

A conclusive stop produces `partial`. An unknown write outcome produces `non_terminal`: the stopped repository's plan file may already exist, even though its save was not confirmed. Inspect and reconcile that exact file in the output directory before trying to save it again. In both cases, standard output keeps the result envelope while standard error reports the failure through the selected error format.

Read the command's result, not just its exit status. For example, a completed control-store inspection can report a missing store; it does not create one or prove that a resident is running. A successful computation likewise does not prove provider availability, installation, or permission to apply a plan.

Post-parse failures go to standard error. Use `--error-format json` for an indented object containing `diagnostic`, `diagnosticIdentity`, `authoringCorrections`, and `authoringLocation`. The nested diagnostic keeps its original canonical contract; its identity is supplied by the SDK. Corrections preserve the compiler's exact source precondition, replacement, field/index path and proof subject. Missing evidence is an empty correction array and a null location, not permission to guess an edit. Human output shows the same available source evidence with escaped values.

This evidence is returned to the caller within the request's workspace and disclosure limits. Match the current source and review the change before writing; reporting a suggested edit neither applies it nor proves current editor contents. If the SDK cannot initialize, JSON output contains only `severity`, `code` and `summary`. SDK diagnostic and source-correction fields are unavailable. Command syntax errors and help retain Clap's own output. The [exit-status contract](src/lib.rs) distinguishes invalid input, blocked work, operational failure, and uncertain effects. Correct rejected input before retrying; reconcile an uncertain effect before attempting it again.

## Shared command implementation

The CLI calls `pale-sdk` for application behavior and owns only command-line parsing, presentation, and process exit status. Its only internal Pale dependency is `pale-sdk`; it does not implement another compiler or diagnostic policy.

The reusable command dispatcher is also consumed by the separate `pale-mcp` interface. MCP disables the default `native-local` feature and therefore receives neither local adapter composition nor the `pale` executable. MCP protocol behavior never becomes a hidden CLI subcommand.

Building or using the CLI does not activate or install a Pale resident.
