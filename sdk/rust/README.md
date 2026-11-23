# Pale SDK for Rust

Use `pale-sdk` to compile a workspace in your application, inspect the configuration it describes and compare a proposed change. The same API supplies Pale's language bindings, command line and editor interfaces. Your application supplies the inputs; Pale returns validated results and handles for further queries.

Use Rust 1.94 or later and build from the Pale checkout. Its `.cargo/config.toml` resolves the matching local `0.0.1` dependencies; these packages are not a published registry release. The example runs Cargo from that root so you need no extra dependency patches.

## Compile workspace documents

An editor can compile the documents it already holds without making the SDK read files. Supply the root manifest's logical relative label and the complete labelled source set it selects, including its module and lock documents. The labels identify those supplied bytes; they are not paths the SDK opens on your machine.

Use the [complete workspace example](../../docs/workspace.md#try-a-complete-portable-workspace) for a first call. Create a fresh application directory outside the checkout, with a `src` subdirectory. Save this as its `Cargo.toml`, replacing the dependency path with the absolute path to this checkout's `sdk/rust` directory:

```toml
[package]
name = "pale-workspace-example"
version = "0.0.0"
edition = "2024"

[dependencies]
pale-sdk = { path = "/absolute/path/to/pale/sdk/rust" }
```

Save the following program as `src/main.rs` in that application directory:

```rust
use pale_sdk::{PaleApi, PortableWorkspaceDocument, PortableWorkspaceRequest};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let documents = ["pale-workspace.toml", "resolution.lock.toml", "universe.lock.toml"]
        .into_iter()
        .map(|label| {
            let bytes = std::fs::read(format!("docs/examples/workspace/{label}"))?;
            Ok(PortableWorkspaceDocument::new(label.to_owned(), bytes))
        })
        .collect::<Result<Vec<_>, std::io::Error>>()?;
    let api = PaleApi::new()?;
    let workspace = api.compile_portable_workspace(
        PortableWorkspaceRequest::new("pale-workspace.toml".to_owned(), documents),
    )?;

    println!("{}: {} documents", workspace.workspace_id(), workspace.source_document_count());
    for context in workspace.profile_contexts() {
        println!("{} / {}", context.project_id(), context.execution_workspace_id());
    }
    Ok(())
}
```

From the **Pale repository root**, run the application, replacing the manifest path with the one you just saved:

```sh
cargo run --manifest-path /absolute/path/to/pale-workspace-example/Cargo.toml
```

Keep that working directory: Cargo reads the checkout's `.cargo/config.toml` to resolve the SDK's matching internal dependencies, and the program reads `docs/examples/workspace/` relative to the same root. The first run may download Rust dependencies and creates a lockfile in your example application. No edit to Pale's Cargo manifests is needed.

Expect `notes-workspace: 3 documents` followed by `notes / local`. This program's loader reads the files; the SDK receives only labels and bytes. Replace the loader with your editor's buffers when embedding it. A missing document or invalid relationship returns an `ApiError` instead of a partly compiled world.

The returned handle keeps the validated workspace available. Its `summary()` provides displayable counts and identities; pass the handle itself, not a copied summary, to operations that need the compilation. To compare a proposed edit, compile the candidate documents separately and call `compare_portable_workspaces` with both handles. Comparison and impact queries retain their own paging limits.

Portable workspace compilation describes the declared world. Package guidance also needs the selected Package inputs and effective-profile compilation; runtime settings and operation contracts need complete one-shot compilation. The [workspace contract](../../docs/workspace.md) explains the project, execution setting and machine selections used for that work. For command-line use rather than embedding, continue with [Inspect your workspace](https://rawthoughts.in/pale/inspect-your-workspace/).

## Generate Package guidance {#project-package-guidance-before-runtime-activation}

Compile the portable workspace, then call `compile_effective_profile` with its project/execution selectors, complete private inputs and exact Package documents. The returned `EffectiveProfileCompilation` can render source guidance for an explicit repository and host:

```rust
use pale_sdk::{EffectiveProfileCompilation, SourceGuidanceHost, SourceGuidanceProjection};

fn inspect_guidance(
    profile: &EffectiveProfileCompilation,
    repository_id: &str,
) -> Result<SourceGuidanceProjection, pale_sdk::ApiError> {
    let guidance = profile.source_guidance(repository_id, SourceGuidanceHost::Codex)?;
    for file in guidance.files() {
        println!("{} {}", file.sha256(), file.rel_path());
    }
    Ok(guidance)
}
```

`ClaudeCode` selects Claude's native skills and references. Both hosts return `AGENTS.md` from `entrypoint_path()`. The result contains proposed skills, role references and a reading index. `entrypoint_contribution()` is a small proposed addition to that shared file, separate from complete files; it does not take ownership of existing project instructions. Required doctrine and inherited role lenses remain composed once, and optional subjects stay linked. Local project facts and native tool bindings retain their owners. Claude's [native loading setting](../../docs/examples/guidance-workspace/README.md#load-the-shared-entry-in-claude-code) is a separate host concern.

This is a source-only Rust API. It observes no host or checkout and writes no files. An adopter must separately review the exact destinations, preserve project instructions and qualify fresh file baselines before writing. The result cannot be used as a runtime saved plan. Selections containing operations, recipes, modes or lifecycle delivery/gates are refused rather than silently dropped; use the admitted runtime path for those duties. Source guidance does not make the ordinary CLI a resident client or grant native tool invocation.

With `native-local`, `compile_local_effective_profile` accepts the existing `LocalOneShotCompilationRequest` selectors and captures the required local source files through the bounded capture owner. It returns the same retained profile, stopping before runtime rendering. This adds filesystem reads, without observing a host's readiness or writing guidance to a repository.

`EffectiveProfileCompilation::repository_context(repository_id)` returns private context separately from `source_guidance`: selected scope, repository access and role, and the session's ordered reading references with their configured machine authority roots. Every reading root requires declared read-only access. Each binding and relative reference remains separate: determining whether the root is a directory or an exact file requires native observation. The method neither reads the referenced documents nor writes repository files, and its machine paths must stay within the project's disclosure scope. Missing reading order is reported explicitly. This renders existing session declarations; it does not infer ownership, accept duties or admit a host. The CLI exposes it as `guidance context` with the same workspace selectors.

For a local reconciliation proposal, call `plan_local_source_guidance` with `LocalSourceGuidancePlanRequest::new(selection, repository_id, host, intent)`. Here `selection` is the same `LocalOneShotCompilationRequest`, `host` is a `SourceGuidanceHost`, and `intent` is `LocalSourceGuidanceIntent::Apply` or `Remove`. The SDK captures the exact selected canonical checkout, private ownership record and required file/parent baselines through the filesystem adapter, observes those paths again and delegates reconciliation to the application planner. Missing runtime paths do not prevent source planning; runtime-dependent selections still prevent an apply proposal. Removal can plan retirement of existing source guidance after a runtime operation is selected.

`LocalSourceGuidancePlan::report()` returns the SDK-owned serializable inventory, baseline hashes/modes, observed UTF-8 text, exact proposed bytes, publication order and plan digest. Before/after entry text includes local facts: serialization is an explicit local review operation, not a public telemetry payload. The opaque handle's debug output omits target paths and bodies. Root and file identities stay private. The application's budget covers observed plus proposed payload bytes, not process memory or extra caller-owned report copies.

Planning writes nothing and creates no lock or ownership state. Its digest is content/baseline identity, not effect authority or a runtime saved plan. Pass the same selection and reviewed digest to `apply_local_source_guidance(LocalSourceGuidanceWriteRequest::new(plan_request, digest))`. The SDK rebuilds sources and native baselines, obtains application admission for that exact review, and invokes the confined filesystem publisher. Its owned result reports the confirmed plan, changed-file count and separate coordination effect. A pending recovery journal blocks normal writes. Publication failure distinguishes not-started, restored before-images and uncertain outcomes; uncertainty requires native inspection before retry.

For a user-facing plan/apply flow, display the report, then call `remember_local_source_guidance_plan(&plan)`. This separate save writes only the proposal identity in a mode-`0600` root file for its host; it does not establish user approval. A later `LocalSourceGuidanceWriteRequest::remembered(plan_request)` retrieves that identity and uses the same fresh-source and target checks. A new plan replaces that host's remembered review. After an apply changes files, review a new plan before another apply. The CLI follows this flow, including when its report is JSON.

For a pending journal, construct `LocalSourceGuidanceRecoveryRequest` from the same selection, repository and host, with an explicit `LocalSourceGuidanceRecoveryIntent::RestoreBefore` or `FinalizePublished`. `plan_local_source_guidance_recovery` validates logical ownership before exposing an optional local review. `None` means no pending journal was observed, not host readiness. Its report gives exact proposed bytes, evidence and conditional directory cleanup; serialization may disclose local before-images. `execute_local_source_guidance_recovery` takes `LocalSourceGuidanceRecoveryExecutionRequest::new(recovery_request, digest)` and rebuilds the observations before exact reviewed admission. Restore preserves independent edits; finalize requires a complete after-state and fills no missing work. An uncheckpointed matching after-image remains uncertain execution origin.

The filesystem adapter owns publication, before-image custody and recovery. The application plan includes a `.gitignore` block for the persistent root lock, private pending journal and remembered reviews, preserving other rules; its contents appear in the same review and transaction as the guidance. The block remains after removal while the lock and review files remain. Git's administrative directories are untouched, including in linked worktrees. See the [CLI guide](../../interfaces/cli/README.md) for exact paths and terminal use. These APIs are available through Rust and the CLI. Actual coding-host use remains separate; no MCP, language-binding operation or runtime invocation grant is implied.

## Compare an edited workspace

In the program above, insert this before `Ok(())`. It keeps the original compilation, changes the project's title in a second source batch and reads every bounded page of impacts:

```rust
let candidate_documents = ["pale-workspace.toml", "resolution.lock.toml", "universe.lock.toml"]
    .into_iter()
    .map(|label| {
        let text = std::fs::read_to_string(format!("docs/examples/workspace/{label}"))?;
        let text = if label == "pale-workspace.toml" {
            text.replace("Shared notes", "Team notes")
        } else { text };
        Ok(PortableWorkspaceDocument::new(label.to_owned(), text.into_bytes()))
    })
    .collect::<Result<Vec<_>, std::io::Error>>()?;
let candidate = api.compile_portable_workspace(PortableWorkspaceRequest::new(
    "pale-workspace.toml".to_owned(), candidate_documents,
))?;
let comparison = api.compare_portable_workspaces(&workspace, &candidate)?;
println!("{} source changes", comparison.change_root_count());
let mut offset = 0;
loop {
    let page = comparison.impacts(offset, 10);
    for impact in page.entries() {
        let values = comparison.compare_subject(impact.subject());
        if let pale_sdk::PortableWorkspaceSubjectValues::Shared { comparison: value, .. } = values.values() {
            println!("{:?}: {:?}", impact.state(), value);
        }
    }
    match page.next_offset() {
        Some(next) => offset = next,
        None => break,
    }
}
```

The comparison reports one source change without changing either file on disk. Its rows include `Affected: Equal`: an input changed, but the selected subject's compiled value did not. Use `compare_subject` for a subject's prior/current values and `explain` to trace why it is affected. Impact means a dependency on the changed input; it does not by itself mean that the subject's resulting value differs. The [local method reference](#build-the-method-reference) defines these methods on `PortableWorkspaceComparison`, including paging and retained-handle rules.

## Read contract data and handle errors

`PaleApi::new()` verifies the contracts embedded in the binary. You can retrieve a schema to inspect its fields or validate a document against its selected contract:

```rust
use pale_sdk::PaleApi;

fn print_diagnostic_schema() -> Result<(), Box<dyn std::error::Error>> {
    let api = PaleApi::new()?;
    let schema = api.contract_schema("diagnostic.v1")?;
    println!("{}: {} bytes", schema.schema_uri(), schema.bytes().len());
    Ok(())
}
```

API failures carry structured diagnostics; use `error.diagnostic()` for their fields rather than parsing the displayed message. API initialization can fail separately. A protocol adapter that needs only diagnostic conversion can use `DiagnosticProjector` without initializing the unrelated operation catalog.

## Choose portable, native or resident operations

Portable methods use supplied data. The optional `native-local` feature adds filesystem and process operations; each call still has its own inputs and effect permissions. Each language binding exposes its supported subset.

A resident is Pale's long-running local service. Resident operation methods require a caller-supplied `ResidentAttachment` with an authenticated carrier and response checks. Start returns a durable handle before dispatch; advance performs one bounded turn; Observe reads signed records; Cancel requests cancellation whose result must still be observed. The SDK does not discover or start that service.

Hook input admission and captured native observations are separate, effect-free steps. `admit_resident_hook` checks a static route and bounded native event; it supplies no active session or carrier. For an admitted original pre-tool event, `observe_codex_command_completion(&admission, started_bytes, completed_bytes)` checks captured Codex v2 command notifications. The codec checks the exact session, turn, tool-use identity, reported cwd, native command representation and lifecycle phases. It preserves terminal item status and an optional typed process exit independently. Missing exit remains missing, regardless of output text. The result retains identities rather than raw IDs, commands or output.

`deliver_resident_hook` consumes an admitted event and sends its exact native bytes and static route through a hook-only or operation attachment. Use `ResidentAttachment::provision_hooks` for blocking hook delivery without signed-record admission, operation commands or Observe access. With `native-local` on macOS and Linux, `load_hook_configuration` loads the explicitly selected protected configuration described in the [CLI guide](../../interfaces/cli/README.md). Observation-only attachments cannot send hooks. The complete frame is limited to 256 KiB, including its JSON envelope. Delivery uses the attachment's shared queue and deadline and never retries automatically.

An embedder with an independent response authority can use `ResidentAttachment::provision_admitted_hooks`. `deliver_resident_hook` then returns `Result<ResidentHookOutput, ApiError>`: the exact native standard output, standard error and exit status become available only after the signed dispatch records and response pass that authority's checks. The resident prepares the selected permission and context response, saves its preparation and dispatch with their checkpoints and independent anchor, then checks current request, session, permission and disclosure authority again before release. The caller verifies the native request and response digests and requires independent signature, authority and one-use progress admission; ordinary operation-observation admission cannot authorize a hook response.

> [!IMPORTANT]
>
> **Select and qualify response authority before permitting native work.** The protected file configuration and `provision_hooks` do not supply it, so they continue to block native work, including reads. Production response authority and deployment integration for interrupted-response recovery remain unfinished. A stopped response append, replay conflict, lost reply or malformed or late response requires reconciliation before another attempt; connecting successfully does not establish enforcement by the host.

This Rust-only codec was qualified against Codex 0.144.1 observations. The integration must independently qualify its actual binary/configuration and authenticate the carrier. The original hook command and native shell-wrapped command have separate identities; best-effort `commandActions` establishes no equivalence. Reading captured notifications neither registers an operation nor consumes an invocation. The first repository-verification duty still requires the admitted durable-operation mapping and one-shot consumption before its native result can settle that duty. No CLI, MCP or language-binding exposure is advertised for this observation helper.

The attachment does not resend a failed request. A separate later call can request a fresh connection. Returned signed records keep their exact canonical JSON, and Rust callers can borrow the original admitted bytes. Effect-free A2A handle inspection checks only the handle's structure, digest and operation class; live custody and signature checks need the resident attachment.

## Build the method reference

From this directory:

```sh
cargo doc --no-deps --features native-local --open
```

Cargo opens Rustdoc from its active target directory, normally `target/doc/pale_sdk/index.html` in this checkout. This is a local reference; the documentation site does not currently publish the complete Rust API. Open `PaleApi` for compilation, `PortableWorkspaceComparison` for source changes and impacts, and `ResidentAttachment` for the carrier and response-checking contracts.

The method reference contains the request types, handle transitions and detailed failure conditions for the operations above. Building it does not run a resident operation.
