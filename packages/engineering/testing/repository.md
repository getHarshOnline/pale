+++
description = "Run the project's check, confirm the required stages and assertions ran, and read the result the consumer receives."
uses = [{ scope = "local", resource_id = "execution" }]

[task]
outputs = ["In the reply: the claim, source version, command and observed result, with incomplete stages and omitted coverage."]
capability_policy = { scope = "local", resource_id = "verification-policy" }
capability_requirements = [{ id = "verify", description = "Run the exact repository-selected check.", capability = "project-test", recipe = { scope = "local", resource_id = "repository-verifier" }, effect_class = "execute", policy = { kind = "required_block" }, failure_posture = "fail_closed" }]
+++

# Run the repository check and read its result

Find the project's exact check through its configured repository verifier and identify the claim it tests. Use [the execution method](../execution.md) for the actual command and result. If the process stops before the assertions, leave the behaviour claim open.

In the reply, name the source version or local changes checked, the command, environment, process result and omitted coverage. Compare the process exit status with the stages and assertions required by the claim. The component that owns acceptance decides whether that evidence is sufficient.

## Check each entry point and how it reports failure

Map supported entry points, bypass options and alternate routes to the checks required before an artifact is produced or used. Check the actual bytes after transformations. For each required property, establish the observation that tests it, the allowed scope and its prerequisites. Keep syntax checks, property checks, compilation and execution distinct. A partial artifact must keep the status of what actually ran.

Exercise a rejected input, a skipped or unavailable check, an exception from the checker and a backend failure after verification. A failed or missing required check must prevent dependent stages from proceeding. Where a route deliberately omits verification, check that its output cannot satisfy an operation that requires it.

Read the stage outcomes, artifact identity and existence, structured result and top-level status as the actual consumer receives them. Include a control where the outer command exits successfully despite a failed required stage. Report the first incomplete or failed stage and any partial result. Source tracing can identify these paths; say which were actually run under the available permission.

## Before handing off

- [ ] Did the exact repository check reach every stage and assertion required by the claim?
- [ ] Does the consumer-visible result preserve failures, skipped work and partial artifacts even when the outer command succeeds?
- [ ] Does the reply name what ran, what it established and what remains unchecked?
