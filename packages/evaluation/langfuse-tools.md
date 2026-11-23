+++
description = "Use Langfuse's official skill and available CLI or MCP for the selected task, checking versions, permissions and the relevant workflow."
see_also = [{ scope = "local", resource_id = "langfuse" }]

[guide]

+++

# Use Langfuse's skill and native tools

Use this guide for a Langfuse task: inspecting data, adding application tracing, managing prompts or datasets, or running evaluators. Choose the workflow and references for that task. An evaluation request alone gives no reason to select Langfuse, install a skill, migrate a backend or start collecting feedback. Follow the project's acceptance criteria, data permissions, deployment, credential setup and chosen framework.

The [official Langfuse skill](https://github.com/langfuse/skills/tree/f0bcb13088b490c04b3f4870bace17f6d57b199c/skills/langfuse) explains how to perform these tasks. This Package brings its tool discovery and workflow selection into Pale without adopting its broad automatic trigger for unrelated AI engineering. A host can install the upstream skill in its chosen directory with `npx skills add langfuse/skills --skill langfuse`; confirm the destination and inspect the installed revision. Selecting this Package supplies guidance. It does not install that skill, create an account or connect an MCP server.

## Establish the actual native surface

Before implementation, read the selected deployment's documentation. Find the page through [the documentation index](https://langfuse.com/llms.txt), then use its Markdown view or the available web reader. Langfuse pages support a `.md` suffix. For self-hosted compatibility, use that deployment's API reference. Check the installed SDK, framework integration, CLI and API snapshot separately. Propose an upgrade only when needed for the task and when the consuming project can test it. A changelog tells you a change exists; it does not define the complete API.

Prefer the CLI when the host has shell access. Otherwise, use an actually connected [Langfuse MCP server](https://langfuse.com/docs/api-and-data-platform/features/mcp-server). Neither needs a new Pale wrapper. If the connection is missing, report it as unavailable; documentation about a server does not mean it is connected.

The [current CLI documentation](https://langfuse.com/docs/api-and-data-platform/features/cli) recommends `@langfuse/cli`; older skill examples use the former `langfuse-cli` package and `api __schema`. These examples use `@langfuse/cli` **1.3.1**, Node **24.22.2** and API snapshot **4.71.0**. Check the installed version and its help when a project uses a different pin. This CLI requires Node 20+.

```sh
npx --yes @langfuse/cli@1.3.1 --version
npx --yes @langfuse/cli@1.3.1 api help
npx --yes @langfuse/cli@1.3.1 api schema --json
npx --yes @langfuse/cli@1.3.1 api observations list --help
npx --yes @langfuse/cli@1.3.1 api prompts get --help
npx --yes @langfuse/cli@1.3.1 api dataset-items list --help
npx --yes @langfuse/cli@1.3.1 api scores list --help
```

An installed and checked `langfuse` executable accepts the same arguments. Use action help and the emitted schema to find names, required inputs, tagged body variants and response fields. Pass nested request data with `--body-json` or `--body-file`. A self-hosted version may need a different bundled snapshot: inspect `api versions list` and `--api-version` help. The CLI version does not establish the server version. Automatic version detection contacts the server, so it is not offline schema discovery.

Supply the selected project's `LANGFUSE_PUBLIC_KEY`, `LANGFUSE_SECRET_KEY` and explicit `LANGFUSE_BASE_URL` or `LANGFUSE_HOST` through the native secret/environment arrangement. Verify the intended region or self-hosted deployment; do not rely on a default host. In CLI 1.3.1, an explicitly selected `--env` file takes precedence over inherited variables. Resolve that precedence before a call so the wrong project cannot silently receive it. Never put keys into Package bodies, command arguments, chat, Git or retained output. Request secrets through the owner's native setup when missing, not by asking for their values in chat.

## Query the evidence the question needs

Use `observations list` for current observation/trace inspection, with the exact trace ID where available and bounded time/environment filters otherwise. Select fields deliberately: the default `core,basic` excludes usage, cost, prompt details and raw input/output. Request `usage` for cost and `trace_context` for trace tags/names; request `io` or expanded metadata only when permitted and needed. A missing unrequested field is not evidence that it was never captured.

Use the returned cursor for the next page of observations and scores. In the checked snapshot, dataset-item listing uses page/limit instead; historical dataset queries also require the dataset name and version. Check pagination for each action. `--all --max-items N` caps collection; report whether that cap or another failure left coverage incomplete. Distinguish empty results, HTTP refusal, delayed visibility and unsupported endpoints.

To derive per-trace cost from observations, request usage and group by exact trace identity, keeping the selected population and time window explicit. A top-N query does not provide a complete export. Scores identify their own subjects: request those fields and join by the actual trace, observation, session or experiment target rather than a display name. Use [trace and dataset lineage](langfuse.md) for export identities, partial acknowledgements, replay, score targets and evaluator placement.

`--json` provides the native response envelope; retain the process exit status alongside it. `--curl` previews a request without sending that request, but its output can contain authentication or private payloads: keep it out of shared logs and redact before retention. A preview proves neither server acceptance nor a write. After an uncertain mutation, read the exact native object before considering a retry; a lost response does not establish non-execution.

For CLI 1.3.1, exits distinguish internal failure (1), invalid usage/input (2), configuration/credentials (3), transport failure (4), HTTP failure (5), local file/contract failure (6), and invalid/inconsistent response (7). Zero covers successful local help/schema commands as well as successful API responses; it does not by itself establish a server interaction. The local qualification exercised discovery and missing-credential/invalid-input refusals, not an authenticated project, ingestion, evaluator or production outcome.

## Follow the workflow that matches the request

The references below come from the reviewed official skill revision. Read the relevant reference and current provider documentation before running its workflow. Carry existing project decisions forward and clarify only unresolved choices. Writes to the service, skill publication, evaluator activation and backend migration still require the owner's authority.

| Task | Official reference and useful boundary |
| --- | --- |
| Add or audit tracing | [Instrumentation](https://github.com/langfuse/skills/blob/f0bcb13088b490c04b3f4870bace17f6d57b199c/skills/langfuse/references/instrumentation.md): prefer an applicable framework integration, preserve observation types and parentage, and inspect an emitted trace |
| Construct an evaluation dataset | [Dataset construction](https://github.com/langfuse/skills/blob/f0bcb13088b490c04b3f4870bace17f6d57b199c/skills/langfuse/references/create-dataset.md): preserve case provenance, the decision being measured and held-out membership |
| Move an existing prompt | [Prompt migration](https://github.com/langfuse/skills/blob/f0bcb13088b490c04b3f4870bace17f6d57b199c/skills/langfuse/references/prompt-migration.md): preserve runtime variables, model/configuration and fallback behavior |
| Revise or debug a prompt | [Prompt engineering](https://github.com/langfuse/skills/blob/f0bcb13088b490c04b3f4870bace17f6d57b199c/skills/langfuse/references/prompt-engineering.md): compare on the relevant cases and retain prompt version/label; a mutable label alone is not a frozen experiment input |
| Set up evaluation | [Evaluation setup](https://github.com/langfuse/skills/blob/f0bcb13088b490c04b3f4870bace17f6d57b199c/skills/langfuse/references/setting-up-evals.md): distinguish online monitoring from offline experiments and define observable metrics that affect the actual decision |
| Capture feedback | [User feedback](https://github.com/langfuse/skills/blob/f0bcb13088b490c04b3f4870bace17f6d57b199c/skills/langfuse/references/user-feedback.md): apply only to a selected feedback task; behavioral signals are evidence to interpret, not ground truth by default |
| Calibrate a model judge | [Judge calibration](https://github.com/langfuse/skills/blob/f0bcb13088b490c04b3f4870bace17f6d57b199c/skills/langfuse/references/judge-calibration.md): check labeled cases and error trade-offs before enabling the judge; do not default every check to an LLM |
| Diagnose recurring failures | [Error analysis](https://github.com/langfuse/skills/blob/f0bcb13088b490c04b3f4870bace17f6d57b199c/skills/langfuse/references/error-analysis.md): inspect representative and failing cases before proposing new metrics or interventions |
| Gate a CI/CD change | [CI/CD experiments](https://github.com/langfuse/skills/blob/f0bcb13088b490c04b3f4870bace17f6d57b199c/skills/langfuse/references/ci-cd.md): bind the candidate, dataset, evaluator and meaningful regression threshold; gate acceptance separately from running the experiment |
| Optimize cost | [Cost optimization](https://github.com/langfuse/skills/blob/f0bcb13088b490c04b3f4870bace17f6d57b199c/skills/langfuse/references/cost-optimization.md): compare cost with quality and latency on matched work rather than optimizing spend alone |
| Migrate an existing Langfuse project | [V4 project migration](https://github.com/langfuse/skills/blob/f0bcb13088b490c04b3f4870bace17f6d57b199c/skills/langfuse/references/v4-project-migration.md): use only for an authorized migration with deployment-specific compatibility and recovery |

When adding tracing, load environment and configuration before initializing the SDK, following the integration's import and setup order. Check model name, usage, meaningful inputs and outputs, observation types, parentage and masking against the current [trace-quality guide](https://langfuse.com/docs/observability/best-practices). Add user, session or tenant identifiers only when needed and permitted. In an agent tree, represent each observed child execution once as an agent observation, preserving the hierarchy at every level. Do not claim to have traced unseen children. Flush short-lived producers, run the authorised path, fetch the exact emitted observations and inspect them. Successful compilation or an export acknowledgement does not establish readable traces with correct scope. Report inspected identities, visibility delays and missing coverage. If access is unavailable, say live inspection could not be completed.

For skills hosted by Langfuse, use the CLI's separate `skills` commands and current [skill-management documentation](https://langfuse.com/docs/skill-management/overview). This differs from installing Langfuse's own agent skill. Preview the exact files before `skills push`: it can include tracked files and eligible untracked files that are not ignored. Exclusions affect new versions and do not retract earlier exposure. Keep version and label resolution, manifest checks and installation scope intact. This Package does not publish a skill or transfer ownership of Pale Packages, project rules or host connections to Langfuse.

Adapted from Langfuse GmbH's MIT-licensed skill at the linked revision; the corpus NOTICE retains the upstream conditions. Native syntax was checked against the MIT-licensed `@langfuse/cli` 1.3.1 artifact, source revision `46d3bce82e695cdf1dc05c89f9f9822e4260d065`. Public provider references may change; recheck the selected deployment and tool pin when implementing a later task.

## Worked example

A teammate gives you a trace ID and asks which prompt ran. Use the installed Langfuse help and the official skill’s trace reference to find the read command for that route. Open the configured project and inspect that trace and its observations. Keep identifiers and the relevant prompt version in the result, with any missing data named.

## Before handing off

- [ ] Read the official task reference for the installed route.
- [ ] Use the configured project and the command or tool that fits.
- [ ] Read the result and record which trace or run it describes.
