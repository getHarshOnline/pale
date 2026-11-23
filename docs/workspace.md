# Working with a workspace

To compile a Pale workspace, give the SDK the root `pale-workspace.toml` and the documents it names. An editor can pass the documents it already has. A local client that reads from disk can use the filesystem adapter. The resulting compilation describes people, projects, responsibilities and the systems they use.

Start with the portable example below to compile documents and compare an edit. For command-line work, follow [Inspect your workspace](https://rawthoughts.in/pale/inspect-your-workspace/) or [Add coding instructions to a repository](https://rawthoughts.in/pale/materialize-repository-guidance/). To extend the declarations, see [local module authoring](#author-local-modules-before-publication) and [project owners and repository duties](#name-a-projects-owner-and-repository-duties).

## Try a complete portable workspace

The [portable example](examples/workspace/pale-workspace.toml) describes a `notes` project with one `local` execution workspace:

| Logical label | Source |
| --- | --- |
| `pale-workspace.toml` | [Project and execution-workspace declarations](examples/workspace/pale-workspace.toml) |
| `resolution.lock.toml` | [Release choices](examples/workspace/resolution.lock.toml), empty in this example |
| `universe.lock.toml` | [The workspace's release lock](examples/workspace/universe.lock.toml) |

Pass all three documents to an SDK's portable compilation call, using the labels in the table. The [Rust example](../sdk/rust/README.md#compile-workspace-documents) prints `notes-workspace: 3 documents` followed by `notes / local`.

Keep the returned compilation handle. Next, change the project title from `Shared notes` to `Team notes` in a second copy and compile it with the same IDs and lock documents. [Compare the two handles](#review-a-proposed-change) to inspect what the edit affects. This example has no Package releases, repositories or execution paths, so it stops at inspection and comparison.

## Review a proposed change

An embedded application keeps both compilation handles and their comparison handle for further queries. The [Rust example](../sdk/rust/README.md#compare-an-edited-workspace) shows how to read pages of impacts, compare values and explain a subject.

The title edit produces one source change. Some impact rows read `Affected: Equal`: the subject depends on the changed document, but its compiled value stayed the same. Read the before and after values to distinguish a changed input from a changed result. Both compilations remain available, and comparison leaves the source files unchanged.

### Compare local configuration with the CLI

For configuration review, compare current and candidate workspaces with the same project, execution workspace and machine. Pale uses the baseline's captured Package definitions for both. It rejects a different distribution lock digest or a change to the project's starting set of Packages. For either change, validate each workspace against its own Package inputs and compare those results separately.

`review-configuration impacts` lists affected and unaffected subjects a page at a time. A subject is affected if it depends on a changed input, even when its value stays the same. `accept`, `reject`, `cancel` and `stale` return a decision without writing files.

Pass the complete preview summary to the CLI on standard input. It reconstructs the comparison before returning a page or a decision. If the summary no longer matches, generate a new preview. The [product review task](https://rawthoughts.in/pale/inspect-your-workspace/#compare-a-candidate-without-applying-it) shows the commands.

The [MCP interface](../interfaces/mcp/README.md#results-and-limits) rebuilds each review from the supplied sources. If a source scan needs a continuation tied to its original handle, keep that handle in the SDK or editor. A copied cursor or digest cannot restore it.

## Select what you are working on

Pale reads the documents named by the root. For a local CLI call, identify the workspace and the context to compile.

| CLI option | Meaning |
| --- | --- |
| `--workspace` | Absolute path to the root declaration |
| `--project` | ID of a project declared by the workspace |
| `--execution-workspace` | ID of an execution setting linked to that project |
| `--machine` | ID of the binding that resolves logical names to physical paths |

A project can resolve differently in another execution workspace or on another machine. To generate coding instructions, supply the settings that identify the Package methods and the target checkout. Runtime `summary`, `compile` and materialization also need the full runtime configuration.

To check whether a connected application is running, use that application's status. The compiled profile answers which configuration would apply.

## Check, plan and apply

Use the planning command for the files you want to update. Keep the plan with the command and configuration that produced it.

::::tabs

:::tab "Shared source guidance"

Use `guidance inspect` to see the role, policies and proposed files for a coding host. To compare that output with a checkout, run `guidance plan --repository source --host codex` (or `--host claude-code`). The plan lists files to add, update, keep or remove and shows their text differences. It leaves instructions unchanged and remembers only the proposal's identity in an owner-readable file at the repository root. Use `--result-format json` for exact before/after text, hashes and modes.

After reviewing the plan, run `guidance apply --repository source --host codex` with the same workspace selections and host. Pale retrieves that host's remembered proposal and recomputes it from current inputs before writing. Changed sources, files or permissions require another plan. It preserves project instructions and refuses conflicting unowned or edited managed files. The plan also includes a `.gitignore` block for Pale's lock, recovery and review files; other ignore rules remain unchanged. These commands write guidance before runtime tool activation.

The [product task](https://rawthoughts.in/pale/materialize-repository-guidance/) follows that path. The [CLI contract](../interfaces/cli/README.md#review-and-install-package-guidance-before-activation) describes ownership, private coordination files and recovery.

:::

:::tab "Runtime materialization"

Runtime `plan` saves one plan per repository. `apply` and `remove` both rebuild the plan and require it to match the saved plan before changing files. An apply plan can remove obsolete generated content. To remove all Pale-managed content, generate and use a removal plan.

Adopting a file Pale does not own requires `--adopt-unowned` at both planning and application. Whole files must match the state recorded in the plan. For managed blocks, Pale requires valid markers and preserves the surrounding text. Removing an adopted whole file deletes it without restoring a pre-adoption backup.

Use `surface-status` to compare current files with the runtime plan. If a file differs, check the cause and create a matching plan before writing. The [CLI reference](../interfaces/cli/README.md#results-and-failures) defines results and failures.

:::

::::

Runtime `apply` requires a runtime plan. Use the result of `guidance plan` with `guidance apply`.

## What Pale writes into a repository

Pale writes host instructions, roles, skills, references and bindings into repositories named by the workspace. Both coding hosts use one `AGENTS.md`. Source guidance adds a separately owned block for each host and leaves the rest of that file under the project's control; runtime rendering produces one shared entry for the selected paths. Runtime output can include hook settings and activation data for that machine. Pale writes to the named repositories and leaves global agent settings alone. The [guidance starter](examples/guidance-workspace/README.md#load-the-shared-entry-in-claude-code) explains how to load the shared entry alongside existing Claude instructions and migrate earlier Pale installations.

The repository owner decides what to commit. Keep private machine bindings, coordination files and the copies saved for interrupted-write recovery out of public output.

## Author local modules before publication

A workspace can name unpublished module sources with `module_sources = ["modules/studio.toml"]` inside `[workspace]`. Keep `module_manifests` and the release/adoption selections empty for this source-only path. The root still selects the component files in `source_components`; each module names their logical IDs instead of supplying component digests.

```toml
# modules/studio.toml
schema = "pale/workspace-module-source.v1"
schema_version = 1
module_id = "studio"
source_components = []
imports = []
exports = []
extensions = []

[[resources]]
kind = "organization"
resource_id = "company"
authority_resolver = "company-authority"
```

Run `pale --workspace /absolute/path/pale-workspace.toml check-source`, adding `--result-format json` for the complete report. No project, execution-workspace or machine selector is needed. The Rust SDK exposes the same operation as `PaleApi::inspect_local_workspace_source(LocalWorkspaceLocator)`, with the `native-local` feature.

This check validates module structure, computes selected component digests, resolves project owners and repository duty references, and uses the existing responsibility and organisational translation lowerers. Its report identifies the phase as `source_only`. A content digest identifies the lowered meaning; separate source evidence records the exact input bytes, so changing a comment changes the evidence without changing that meaning. Machine bindings, Package distributions and credentials are not read.

Source-only inspection currently accepts local modules without release imports or experience-composition files. It does not link arbitrary declaration namespaces or normalize runtime policy. Duties retain attributable omissions for publisher and adopter coordinates until those releases exist. Publication fields are rejected in a local module source. Preparing a published module later requires explicit publication identities and the normal release, signature and admission checks; this report cannot be used as an admitted workspace handle. Ordinary workspace compilation rejects a root that still selects local module sources.

<a id="name-a-projects-owner-and-repository-duties"></a>

## Name a project's owner and repository duties

A project may declare `owner` as a reference to a selected module resource. For example, `owner = { kind = "organization", module_id = "studio", resource_id = "company" }` names the organisation selected as `company` by the selected `studio` module. Use `kind = "principal"` for a person who owns a project in their own capacity. The module and resource must exist in the selected set (adopted releases for ordinary compilation, local sources for `check-source`), and the resource's kind must match. Leaving `owner` absent leaves ownership unspecified; a directory, author or repository user does not fill it in.

A project-to-repository membership may name existing duties with `responsibility_derivation_ids = ["verify-source"]`. The list is sorted, unique and limited to 1,024 IDs; each must resolve to a responsibility derivation selected by the root manifest. The referenced record retains the duty's inputs, provider, consumer, acceptance and other answers. A repository shared by two projects can therefore have different duties in each project without being declared twice.

In ordinary workspace compilation, these references participate in source navigation and the selected topology's identity. Source-only inspection retains them in its report and content digest; it does not produce that compilation's navigation index. They declare relationships; they do not supply signed module admission, appoint a provider, accept a result or widen repository access. The private `guidance context` output currently shows the selected repository and source reading order; it does not yet render these owner and duty records.
