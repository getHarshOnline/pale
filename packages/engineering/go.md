+++
description = "Write and review Go code using modules."
uses = [{ scope = "local", resource_id = "change" }]
official_sources = [{ label = "Go documentation", url = "https://go.dev/doc/" }]

[guide]

[role]
extends = [{ scope = "local", resource_id = "review" }]
applies_to = [{ scope = "local", resource_id = "go" }]

[stack]
role_uses = [{ scope = "local", resource_id = "go" }]
+++

# Write Go with clear interfaces and cancellation

Apply the common coding method and the repository's Go formatting, import and checking rules. Prefer plain control flow, useful zero values and composition. Give packages clear responsibilities and keep their dependencies free of cycles.

## Design for the caller

Let consumers define small interfaces, and return concrete types when that makes the API clearer. Use functional options when a constructor has real variation to support. Inject dependencies through interfaces that serve an actual need.

When adding error context, preserve the identity used by `errors.Is` and `errors.As`. Handle the error where a useful response is possible, and reserve panic for a broken guarantee from which the program cannot recover.

## Own concurrent work and its shutdown

Pass context through I/O calls with explicit deadlines and cancellation. Give every goroutine an owner and a shutdown path that completes within its allowed time. Establish who owns and closes each channel, including what happens to senders when a receiver leaves. Avoid shared state where practical; otherwise protect it with the smallest suitable synchronization scope. Use the project's race checks for changes where concurrent access matters, within the task's permission.

Test observable behaviour. Named table cases help when inputs fall into meaningful classes; shared setup should still leave the failing call easy to see. Keep complex expected output readable and owned by the test. Use generated inputs for parsers and other wide input spaces, benchmarks for a stated performance question, and coverage to find gaps worth investigating.

## Cancel a provider request

Have the consumer define its small reader interface with `context.Context`, and pass the request context to the provider. Cancel that request in a test. Check both the returned error and whether the provider's work stops.

## When reviewing as a specialist

Add Go-specific risks to the shared review method. Look for reachable failures from:

- discarded errors, lost error identity or context, and panics on recoverable paths;
- goroutine leaks, missing cancellation or coordination, channel deadlocks, inconsistent lock order and shared-state races;
- untrusted input reaching queries, processes, paths or TLS handling without the required restrictions;
- unlimited collection or channel growth, loop defers retaining resources, repeated network or database calls, and measured allocation costs on hot paths; and
- package state or an interface that makes the changed behaviour unsafe or prevents its intended substitution.

An explicit unlock, concrete type or test without a table needs a concrete consequence before becoming a finding. Ask the requesting task for the relevant repository-configured build, vet, race, static-analysis or vulnerability results.
