+++
description = "Use it to compare the authored workflow, generated export, imported revision and active revision, and to test worker and webhook routing and queue recovery."
see_also = [{ scope = "local", resource_id = "workflow" }]
official_sources = [{ label = "n8n native documentation", url = "https://docs.n8n.io/deploy/host-n8n/configure-n8n/scaling/enable-queue-mode/" }]

[guide]

[stack]
+++

# Check an n8n workflow before deployment and after failure

For a node-based engine such as n8n, compare the authored workflow, generated export, imported revision and active revision separately. Test worker and webhook routing, credential references, retries and queue recovery on the intended deployment. Include every child route and error workflow. Queue capacity and a successful execution do not establish that the destination applied the change. Use the selected version's documentation for deployment details.

For generated JavaScript Code nodes, lint each body in the function context the workflow supplies. When collecting its helper functions, use a parser to identify references that the body does not define itself. The shared local Code checker uses the consuming project's locked formatter and linter. That project supplies the export selection, formatting options and runtime-global overrides. Include only helpers reachable from the body or explicitly selected roots, visit each once, and preserve the project's chosen output order and serialization. Reject an unknown explicit root. These static checks do not run the workflow or establish that the target provides every allowed global.

## Check the selected node's actual capability and credentials

Prefer the installed service nodes for operations they support. A custom node needs a concrete capability gap that explains why the existing node cannot do the job. For one bounded model answer, use the supported single-response operation or a plain HTTP request; do not add conversation state or a chat-model workflow when the task does not need it.

Inspect the installed node type and version, supported operation, accepted parameters and returned output before choosing it for a step. Check the actual input and output in a permitted test execution. A provider supporting an API feature does not establish that this node exposes it, and a successful import does not establish that its settings took effect. If another node or an explicit HTTP request supplies the needed operation, verify its authentication, response parsing and error routes separately.

Resolve credentials by the target instance's exact credential ID and type, then verify the intended account and environment through an allowed connection check. A display name can be duplicated or changed. Keep secret values in the connection store. Check each child workflow and regenerated or imported node; a credential reference from another instance needs an explicit mapping.

When the workflow owns the attempt budget, switch off automatic node-level retries and account for retries in any underlying client. For example, a controller permitting two attempts cannot allow each attempt to trigger two hidden requests: that path could make four provider calls. Verify actual request counts in a failure case, including the fallback branch, and check the active workflow's settings after import.

## Verify the Code batch before updating the workflow

A JavaScript check requires a string body on every selected Code node. An omitted language selector may use the supported export format's JavaScript default. If another language is selected, a leftover JavaScript field supplies no evidence for it: leave that node's check open and use its matching checker. Analyse the body inside an anonymous wrapper so a checker-owned name cannot hide an undefined reference or cause a required helper to be omitted.

Prepare the formatted bodies and lint the whole batch before replacing any workflow body. If formatting or linting fails, keep the original workflow intact for diagnosis and retry. Findings must retain the workflow and node names and line numbers within the Code body. The adapter that talks to n8n writes the accepted export and checks the imported and active revisions separately.

Use the selected export-comparison operation to compare retained artifacts mechanically. If its provider is blocked, that comparison remains open.

## Worked example

A webhook starts a change and its response is lost. Run a disposable workflow copy with its provider mapped to a test target. Retry the same business request, inspect the execution and read the provider's recorded result, then compare exports with the maintained verifier. The execution count and the number of applied changes answer different questions.

## Before handing off

- [ ] Check the workflow revision, execution settings and dependencies.
- [ ] Exercise the change in a suitable environment with controlled effects.
- [ ] Confirm the observed result and how a failed run is recovered.
