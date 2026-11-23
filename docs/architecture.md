# Pale architecture

Every coding tool asks for permissions and context at each step, and the same rules end up repeated in every session. Pale is the meta harness through which Weave, Scope, Counsel, RawFish and Gazette act, a harness over the tools people already use; RawFish and Gazette run alone, and Pale may coordinate them. It starts from a description of a person's world: the people, projects, applications and permissions their work depends on. The compiler turns that description into a checked version the tools can use. Today the local tools answer what applies to a repository, what a proposed change would affect and which guidance files it would generate. The path from a Claude Code or Codex hook to the resident, where the hook asks what may happen and what context the agent needs, is the problem in front; references to each owner's records carry changes between people and agents. The same coordination model supports people working without agents. For that experience, start with [Introduction to Pale](https://rawthoughts.in/pale/understand-your-world/).

The compiler and repository-guidance path work locally. The ordinary hook path supports an explicit hook-only connection that currently refuses native work. Successful admission through an authenticated resident and enforcement by the actual host remain unproved. An editor can already explain why a project receives particular guidance; carrying a live operation to its application also needs the route, permissions and provider checks described below.

> [!IMPORTANT]
>
> **Use matching builds.** The local tools compile and inspect workspaces, review changes and install repository guidance. A complete resident deployment and PaleUniverse are still in development. Use the API and contracts from the same build while a stable release is pending.

## A shared engine, a personal workspace, a running deployment

| Object | What it supplies |
| --- | --- |
| Distribution | Reusable engine, contracts, SDKs, interfaces and Packages |
| Workspace | A person's declarations about their projects, relationships and tools |
| Deployment | The environment, machine bindings and running state |

A workspace may describe a household, company and client engagement together. Each has its own people, records and permissions. A connected application controls access to its records and decides what changes it accepts. Pale's ability to read or call a path gives it no authority over that path: native applications, a person's private workspace and a Pack's distribution each keep their own owner. A request that crosses them is split into separate effects, and each effect Pale has no authority for goes to its owner. Workspace declarations and machine bindings let people share the same engine while keeping their own paths and policies. Another person can describe a different world with the same engine.

## Compile the description before using it

The compiler checks the source documents and resolves their relationships. The right call depends on the answer the caller needs.

| Compilation | Input | What the caller can inspect |
| --- | --- | --- |
| Portable workspace | Logical labels and complete document bytes | `PortableWorkspaceCompilation` for authoring, inspection and comparison. The caller supplies bytes, so the compiler needs no filesystem access. |
| Effective profile | A compiled workspace, a project and execution workspace, and the required Package and private inputs | The guidance and configuration that apply to that responsibility |
| One-shot | The complete runtime configuration and machine bindings | A handle for the rendered repository output in memory. Plan repository changes in a separate call. |

A repository ID identifies a repository in the workspace. Machine bindings tell Pale which checkout or executable to use on this machine. For a missing document, check the compilation inputs. When the configuration compiles but an operation cannot run, check the executable and the provider.

The [portable example](workspace.md#try-a-complete-portable-workspace) contains one project and one execution workspace. The [SDK guides](sdks.md) show how to compile those documents in an embedded application.

## Compose guidance without turning prose into permission

Packages supply reusable roles, procedures and policies. The workspace names what a responsibility needs, and the compiler resolves the prerequisites. Repositories can use the same method from one maintained source.

For example, change a repository's role from code work to security review. Its generated role and policy references can change while its own instructions continue to supply build commands and project facts. Inspect the proposed guidance first, then compare it with the repository's files in a file plan. The [guidance starter](https://rawthoughts.in/pale/materialize-repository-guidance/) runs this workflow without a runtime provider.

An agent uses the Markdown to make judgments. An operation gets permission from Pale's checks of the route, contract, capability bindings and caller authorization. The provider must satisfy that operation's contract. A provider name in a configuration file only identifies what to bind.

[Package authoring](package-authoring.md) explains how workspaces choose and combine resources. [Package source bundles](package-source-bundles.md) explains how to distribute them.

## Compare configuration before applying a file plan {#compare-configuration-then-apply-an-exact-plan}

A review compares the current compilation with the proposed one. It identifies what changed and which subjects depend on that input. A dependent subject may still have the same resulting value. Keep the compilation and comparison handles for follow-up queries. A display summary or hash cannot reconstruct either handle.

To write guidance after a review, create a file plan against the repository's current files. The CLI shows the differences and remembers that proposal for `guidance apply`; SDK callers can also pass its digest explicitly. Pale checks the inputs and files again, preserves project instructions and refuses conflicting files or changes to files it manages. If a write is interrupted, use recovery to determine what happened before trying again.

Runtime materialization makes its own file plan and checks write permissions separately. To take over a file Pale does not manage, request adoption explicitly. Removing an adopted whole file later deletes it without restoring an earlier backup. See [workspace planning](workspace.md#check-plan-and-apply) for both kinds of plan, or [Add coding instructions to a repository](https://rawthoughts.in/pale/materialize-repository-guidance/) for the installation walkthrough.

## Carry operations across interruptions

Delegated work can outlive its caller. If the caller loses a response, another application may still be acting. The resident gives the operation a durable handle so the caller can find it again.

| Operation | What it does |
| --- | --- |
| Start | Return an operation handle before dispatch |
| Advance | Perform one limited resident turn |
| Observe | Read the current signed records |
| Cancel | Request a stop. Observe the operation afterward to learn whether it stopped. |

If a send succeeds and the response is lost, a second send may duplicate the action. Pale must check the application that sent it before deciding what to do next. Pale's journal tracks its own state transitions. The connected application keeps the record of what actually happened there. Beside the journal Pale keeps genealogy, the history of context: append-only edge records that show where a piece of work came from, what it relied on and what followed, while each record stays with its owner. The resident components keep these edges as tested records. Scope's write-back of a pointer to the context it gave each agent is the work in front; once it completes the chain, that chain is what Reckon, Causality's evaluation laboratory, learns from.

The embedding application supplies `ResidentAttachment`. Its carrier authenticates the resident. Its response checks verify signatures, changes to who may act and the proofs needed to reject a rollback. Use the [Rust SDK](../sdk/rust/README.md#choose-portable-native-or-resident-operations) or the [A2A interface](../interfaces/a2a/README.md) to attach explicitly. The ordinary CLI can load an explicit [hook-only connection](../interfaces/cli/README.md), which still blocks native work and exposes no operation records. Ordinary CLI and MCP startup install no resident service and provide no resident operation attachment.

## Keep one implementation behind the interfaces

`pale-application` owns compilation, comparison, planning and operation behavior. `pale-sdk` exposes it to clients. The language bindings wrap that API. The CLI calls it directly; MCP reuses the CLI's dispatcher. LSP, ACP and A2A use the SDK for their editor, coding-session and delegation tasks.

Adapters implement I/O behind ports. Integrations translate between the host and Pale's API and keep using the application that owns each record.

Rust wire records define the shared contracts. The schema and language types are generated from them. Runtime calls also check the relationships in a request and whether the caller may act. Read a failure's diagnostic fields directly. If the compiler can propose a source correction, verify that its source precondition still matches before writing it.

## Find the responsible layer when something goes wrong

| Symptom | Where to look |
| --- | --- |
| A document or reference is rejected | The diagnostic's source location and the workspace's declared documents |
| A project receives unexpected guidance | Its effective profile, Package choices and comparison explanation |
| A saved plan no longer matches | Changed inputs or repository file baselines |
| Configuration compiles but an operation cannot run | The host capability, machine binding and provider readiness |
| A resident response is uncertain | Use the operation handle and check the record in the application that owns the result. |
| Scene geometry works but live intake cannot start | Check whether Pale issued the organization or comparison handle for the intended recipient. |

The [interface manuals](interfaces.md) and [SDK manuals](sdks.md) define inputs, results and handle lifetimes. [Scene](../sdk/scene/README.md) distinguishes geometry from live workspace intake.

## Extend the layer that owns the behavior

Add shared behavior in `pale-application` and expose it through `pale-sdk`. Change a wire record at its Rust definition, then regenerate the schema and language types. Put host presentation in an interface, I/O in an adapter and reusable agent procedure in a Package. Keep the core's types neutral: one application's meanings belong in its adapter or integration, and adding them to a shared kernel enum to make an adapter convenient puts that application's semantics in every user of the kernel. Integrations call the supported API.

For a new procedure, follow [Package authoring](package-authoring.md). For an attached delegation, use the [A2A call contract](../interfaces/a2a/README.md#construct-a-call-in-your-application). [Source layout](source-layout.md) maps the implementation, and [Contributing](../CONTRIBUTING.md) explains contribution requirements.
