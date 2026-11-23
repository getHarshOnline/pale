# Pale

Every coding tool asks for permissions and context at each step, and the same rules end up repeated in every session. Pale is the meta harness through which Weave, Scope, Counsel, RawFish and Gazette act, a harness over the tools people already use; RawFish and Gazette run alone, and Pale may coordinate them. It lets an integration answer which configuration applies to a responsibility and what a proposed edit would change. Supply the workspace documents and Pale resolves how people, projects, repositories and systems relate. Packages add reusable roles and procedures. The SDK, command line, editor and agent interfaces share one Rust implementation. These answers come from the local tools today; the path from a Claude Code or Codex hook to Pale's resident, where the hook asks what may happen and what context the agent needs, is the problem in front.

## Start with your integration

::::card-group
:::card
[Build the command-line tool](getting-started.md)

Build from source and run a computation on supplied data.
:::
:::card
[Embed the compiler](sdks.md)

Choose a language binding and compile a workspace from your own application.
:::
:::card
[Connect an editor or agent](interfaces.md)

Find the protocol that fits your host and check what the caller supplies.
:::
::::

To inspect a workspace or install coding instructions in a repository, start with the [product guides](https://rawthoughts.in/pale/).

## What can run locally {#understand-the-boundaries}

The local tools compile and inspect configuration, compare proposed changes and write shared coding instructions to repositories. See [Working with a workspace](workspace.md) for the inputs and comparisons, and [Package authoring](package-authoring.md) for how to compose guidance from a single source.

A complete resident deployment needs an authenticated connection, configured trust and a managed lifecycle. The compiled configuration describes the connections. The applications themselves control access and must be started separately. [Pale architecture](architecture.md) follows a change from compilation to a repository update.
