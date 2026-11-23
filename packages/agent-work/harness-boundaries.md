# Design the tools and controls around an agent

An agent harness controls what a model can observe, decide and do. Use this guide to design or review those controls. The design must identify the provider, tools and runtime needed to enforce them; writing it does not install or start them.

## Begin with decisions and effects

In the design, list the decisions the agent must make, the evidence each needs, the actions it may request and the possible final outcomes. Give each capability a meaningful job. Make it narrow enough to control and verify, but broad enough that the model can complete a transaction without assembling fragile steps.

For each capability, specify typed inputs, output limits, the kind of effect, required permission, whether repeating it is safe, timeouts, error categories, handling of sensitive data and how to identify the result. Distinguish reading, drafting, local changes, publication and irreversible actions. Validate requests in the adapter that handles them. Instructions alone cannot make an unsafe interface safe.

## Choose guidance for the decision

Use the intended result and project instructions to choose the primary skill. Its description should say what decision it helps with, when it applies and which nearby task belongs elsewhere. Honour an explicitly requested skill within the governing constraints. Add another only when it helps with a distinct decision reached during the work. If none fits, follow the project's established route.

Read the skill's entry, requirements and expected result. Follow the sources that own unresolved decisions or changed dependencies; open optional examples, templates and tools when needed. A link helps reach the subject's owner. Keep project constraints and facts with that owner, and specialised procedure with the skill.

Match planning, intermediate artifacts and checking to the change and its consequences. Reuse existing mechanisms and carry out authorised work when the scope is clear. Report the result, relevant check and anything the available evidence leaves open.

## Return useful observations and recover from failures

Give the model enough information for its next decision: status, relevant evidence, the result's identity, uncertainty and permitted next actions. Distinguish an empty result from an unavailable service, denied access, invalid request, partial result or unknown outcome. Keep large payloads and raw logs with the service that owns them and return a reference when it can serve details on demand.

Define recovery for each error category. Retry known transient failures only when repeating the work is safe and the budget permits it. Correct wrong inputs or scope. Escalate uncertain effects, conflicting permissions, privacy risks and repeated lack of progress. Set limits for calls, elapsed time, spending, output, stored state and autonomous iterations.

## Check worker isolation and completion

Before launch, identify the worker, the runtime enforcing its restrictions, the policy and the host. For every allowed capability, list what the worker and its child processes can read, write, inherit, reach and invoke. Check effective credentials, environment, mounts, writable output paths, network and tool access. Use harmless synthetic data to attempt prohibited reads, writes, network calls and credential access through the real enforcement layer. A checkout, tool list or orchestration hook may organise work; claim isolation only for restrictions the runtime enforces and these probes exercise.

If a required isolation profile is missing or invalid, stop before the payload starts. Exercise failure of the worker and its enforcement mechanism, restart and cancellation. Give any approved alternate route its own identity, limits, checks and permission to run. Carry one deadline through the queue, setup, execution, tool calls and cleanup. Limit concurrency, CPU, memory, storage and result volume where relevant, separately from model usage. Check that exhausted resources or a stopped worker cannot leave unauthorised child processes running or interrupt unrelated work.

Before dispatch, associate the submitting identity, invocation and exact inputs with the job. For an artifact result, check identity, ownership and successful reading from durable storage before reporting success. A child process exiting successfully or an upload partly completing cannot establish that. Exercise missing input, wrong-owner or stale output, interrupted storage writes, replay and cleanup within the caller's permissions. In the check results, identify the runtime and route tested. Keep declared restrictions, platform records and observed effects distinct.

## Check the whole harness

Test schemas and adapters, then exercise representative tasks and failure paths: denied actions, malformed or adversarial content, stale state, an absent provider, partial success and a result record belonging to a different action. Check whether the available capabilities make permitted work easy and prevent prohibited work through actual controls.

Put the decisions and effects, capability definitions, returned observations, recovery rules, budgets, test cases and unresolved service dependencies in the design or requested review. Hold the affected work when permissions or ownership are unclear, a capability combines incompatible effects, success cannot be distinguished from an unknown outcome, or recovery needs a control the route cannot provide.

## Worked example

A draft-review tool can read a draft and append comments. Give it the draft identifier and a comment action limited to that draft. Keep sending and publication outside its permissions. Interrupt a comment request after submission and read back the comment ID before retrying. Drafting and review can remain usable while publishing is unavailable when their own requirements are still met.

## Before handing off

- [ ] Is it clear what the agent observes, decides and can do?
- [ ] Do the owning tools enforce permissions, budgets and recovery?
- [ ] Have an allowed action, a denied action and a failure been exercised?
