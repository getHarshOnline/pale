+++
description = "Check TypeScript types against runtime data and review JavaScript or TypeScript caller failures."
uses = [{ scope = "local", resource_id = "change" }]
official_sources = [{ label = "TypeScript documentation", url = "https://www.typescriptlang.org/docs/" }]

[guide]

[role]
extends = [{ scope = "local", resource_id = "review" }]
applies_to = [{ scope = "local", resource_id = "typescript" }]

[stack]
role_uses = [{ scope = "local", resource_id = "typescript" }]
+++

# Check TypeScript types against runtime data

Apply this guidance when TypeScript is selected. JavaScript alone, including a React or Vue project, does not establish that TypeScript is in use. Read the actual compiler options and identify where runtime data is validated before relying on a static type.

Give public interfaces precise types. Use discriminated unions for states whose cases must be handled explicitly, and narrow `unknown` before trusting it. An `any`, assertion, non-null claim or unchecked generic needs a checked condition that supplies the guarantee the type system no longer enforces.

Prefer immutable data flow where it makes ownership clearer. Give each promise an owner: await it or deliberately return it, with failures handled and parallel work chosen explicitly. Validate parsed values, environment variables, network responses, stored data and messages when they enter the program.

Inspect dynamic execution, object merging, URLs, HTML, queries, processes, paths and client bundles. Keep secrets and server-only values out of emitted JavaScript and public source maps.

Run the repository's configured type check and exercise the real caller and invalid input path wherever a type check is bypassed or external data enters. In the reply, say which runtime checks support the type claim and what remains unchecked.

## Validate a response variant

Treat a response body as `unknown`. Check its discriminant and fields before narrowing it to a success or error variant. Exercise a response with the right tag but a missing payload, so the caller cannot mistake unchecked data for a valid result.

## Review JavaScript and TypeScript runtime failures

Review the changed JavaScript or TypeScript code and configured lint or runtime results supplied by the requesting task. Apply TypeScript guidance and type-check results only when that stack is selected; React, Vue or JavaScript alone does not establish it.

Look for reachable failures from:

- in TypeScript, `any`, casts, non-null assertions or weakened compiler checks hiding a contract the caller needs; in either language, parsed or environment data reaching typed or trusted code without validation;
- promises with no owner or competing owners, unhandled rejection, `forEach(async ...)`, and sequencing that creates a race or avoidable measured latency;
- untrusted data reaching dynamic execution, HTML, URLs, queries, child processes, paths or object merging, and confidential values entering client bundles;
- synchronous I/O on request or event paths, mixed module systems that fail to load, and mutable module state unsafe under concurrent requests; and
- empty catches or thrown values that break the established error contract.

Style, broader type coverage and parallelization need concrete impact before becoming findings. Ask the requesting task for relevant repository-configured check results.
