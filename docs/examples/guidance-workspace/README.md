# Repository-guidance workspace

Use this complete example to inspect or integrate Pale's source-guidance route. It chooses Engineering's `change` role and four Engineering/Governance policy families for one repository. It contains no operation, provider, lifecycle hook or runtime execution path.

For copying the starter, setting machine paths, inspecting guidance and applying a reviewed plan, follow [Add coding instructions to a repository](https://rawthoughts.in/pale/materialize-repository-guidance/). That product task owns the installation procedure.

## What the inputs describe

| Input | Meaning |
| --- | --- |
| `pale-workspace.toml` | Project `repository`, execution workspace `local-guidance` and repository `source` |
| `capabilities.toml` | Source-only method selection without callable runtime operations |
| `machines/local.toml` | Local Pale distribution and canonical target-repository paths |
| `distribution.lock.toml` | Package content identities produced by `distribution-info --lockfile` for the actual distribution |
| `resolution.lock.toml` and `universe.lock.toml` | Workspace resolution and universe identities |

The distribution lock is created after copying the example; it is not supplied as a universal pin. Review it when the distribution changes. Project facts, commands, credentials and acceptance criteria stay with the repository and its tools.

Both hosts use the target's `AGENTS.md`, preserving local text and each other's marked contributions. Skills and references use each host's native paths. Pale does not create `CLAUDE.md`; see [Load the shared entry in Claude Code](#load-the-shared-entry-in-claude-code) for how Claude Code reads the shared file.

## Embed the same workflow

The [Rust SDK](../../../sdk/rust/README.md#project-package-guidance-before-runtime-activation) compiles the effective profile and renders a repository/host projection. Its native source-guidance methods additionally plan against real file baselines, remember a displayed proposal, apply that exact review and reconcile interrupted writes. The [CLI](../../../interfaces/cli/README.md#review-and-install-package-guidance-before-activation) exposes those operations with a readable text diff and a digest-free apply.

Inspection does not write guidance. A plan does not authorize a write. Applying the reviewed plan confirms its files, while the coding host's use of that guidance needs an actual task in that host. A source-guidance result cannot be passed to runtime `apply`.

## Load the shared entry in Claude Code

Use Claude Code 2.1.285 or later with its built-in instruction-file plugin enabled. An existing project or ancestor `CLAUDE.md` can mask `AGENTS.md` under the default loading mode. To load both without replacing those instructions, select `claude-md-and-agents-md` under **Project instructions** in `/config`, or in user settings, managed settings or the invocation's `--settings` JSON:

```json
{
  "pluginConfigs": {
    "cc-plugin-agents-md@builtin": {
      "options": {
        "instructionFiles": "claude-md-and-agents-md"
      }
    }
  }
}
```

Project and local settings do not control this option. Check `/memory` in the actual host to confirm the shared file was loaded; the file's presence on disk does not show that. Claude's [instruction-file documentation](https://code.claude.com/docs/en/memory) describes the plugin. Pale does not change global settings or create a second repository entry file, and existing client and upstream instruction files keep their owners.

An earlier Pale ownership record may name `CLAUDE.md`. An update then refuses to install into the new entry. Review `guidance plan --host claude-code --intent remove` with the same workspace and repository selections, apply that removal, then plan and apply the new installation. Removal preserves text outside Pale's exact recorded contribution. Reconcile any pending journal through the [CLI recovery route](../../../interfaces/cli/README.md#review-and-install-package-guidance-before-activation) before proceeding.
