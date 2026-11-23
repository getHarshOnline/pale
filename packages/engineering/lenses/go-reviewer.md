# Review Go errors and concurrent work

Add Go-specific risks to the shared review method. Look for reachable failures from:

- discarded errors, lost error identity or context, and panics on recoverable paths;
- goroutine leaks, missing cancellation or coordination, channel deadlocks, inconsistent lock order and shared-state races;
- untrusted input reaching queries, processes, paths or TLS handling without the required restrictions;
- unlimited collection or channel growth, loop defers retaining resources, repeated network or database calls, and measured allocation costs on hot paths; and
- package state or an interface that makes the changed behaviour unsafe or prevents its intended substitution.

An explicit unlock, concrete type or test without a table needs a concrete consequence before becoming a finding. Ask the requesting task for the relevant repository-configured build, vet, race, static-analysis or vulnerability results.
