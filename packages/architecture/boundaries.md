# Keep application rules independent of infrastructure

Use ports and adapters when application rules must outlast a delivery channel, storage engine, framework or outside provider. The component that owns a rule should continue to own it when those technologies change.

## Boundary test

Identify the decisions that express the application's purpose. They form its core. Define each outside interaction as a port describing what that core needs. An adapter translates a particular technology or protocol into the port without making the core depend on its types, lifecycle or failure assumptions.

Incoming adapters turn requests and events into use-case inputs. Outgoing adapters provide services such as storage or messaging. Dependencies point towards the rules: outer code can know the core contract; the core need not know which adapter is installed. A repository's dependency check can supply the `dependency-direction` check for these imports. Its result establishes the dependency paths it inspects, not whether every business rule is in the right place.

## Preserve policy across an atomic state change

A replacement adapter must preserve decisions, not just method signatures. Keep business rules with their owner and specify storage freshness, concurrency and atomicity in the port. A rule hidden in one adapter will not automatically survive its replacement.

When a decision depends on state being changed, evaluate it within the protected read-and-commit operation or recheck the expected revision before committing. A decision function supplied by the rule's owner can run inside that operation without the adapter importing the use-case implementation. Use that design when the existing transaction contract needs it. Preserve refusal and rollback behaviour, and derive the returned result from what actually committed.

If storage can rerun the decision, define its retry behaviour and keep the function free of effects a retry could duplicate. Trigger external work through a separately authorised committed intent, or the owning system's equivalent mechanism. A local commit does not prove that external work completed.

Run the same accepted and refused transitions through each replacement adapter. Include a change between read and write, an aborted commit, a repeated decision and a lost response. Inspect the decision result and stored state independently. Controlled ports can test use-case behaviour; the real adapter still needs tests of transactions and retries.

## Verification

Test use cases through controlled implementations of the ports, then test each adapter against its actual contract. Review transactions, error translation, time, identity and idempotency. These are common ways that a storage or delivery technology's assumptions enter application rules unnoticed. Keep durable cases in the repository's tests and describe results and gaps in the change review.

## Prove store isolation

For separately owned stores, identify each service's runtime identity, endpoint or file root, writer fencing token, permitted operations and recovery path. Name the enforcement layer. Server grants, filesystem permissions and application checks protect different routes. Different database names or paths alone provide no access control.

Using the intended runtime identities, test permitted owner operations and denied cross-store reads and writes. Include a wrong credential or path as a negative control, then inspect both stores after the denied write. Check that backup and restore preserve isolation and that an outage does not grant another service access. A disposable test establishes only the identities and enforcement layer it exercises.

## Prove retrieval isolation at the selection boundary

Define the permitted population and the required isolation before choosing an index or cache. Separate three claims:

- **Output safety:** the caller receives no ineligible record, snippet or identifier.
- **Traversal and ranking isolation:** changing ineligible records cannot alter which eligible candidates are found, ranked or selected.
- **Observable noninterference:** those changes stay within specified limits for timing, counts, caches, errors and resource signals visible to the caller or operator being considered.

Trace what every search branch, graph traversal, reranker and shared budget can see. Apply the correct partition and eligibility filter before candidate limits and ranking. Filtering only the final response can hide ineligible results while those records still consume candidate slots or influence traversal. Share an index only for a group whose owner permits that arrangement and whose tested mechanism supports the claimed isolation. Check cache keys and invalidation against permissions, scope and revision, including stale or revoked grants.

Fix a permitted query and population. Add, remove or change only ineligible records, including a high-scoring decoy and enough hidden records to reach a limit. Compare against an exact search of the eligible set. Inspect reached candidates, final order and allowed observations separately. Use post-filtering as a negative control. Test cold and warm reads, revocation and a missing partition. For timing and resource claims, state limits and repeat measurements. An exact oracle establishes expected retrieval for that input; it cannot prove physical isolation or another route's timing.

Tie each conclusion to the implementation and identity tested. If a required level is unproved, refuse the query or use a degraded route the owner permits whose isolation is separately verified. The owner decides partitioning, eligibility, observable information and fallback policy.

## Check declared read budgets against execution

Separate a read plan's estimate, limits enforced by the adapter and work actually observed. A declared relationship fanout is an assumption unless the source or execution system enforces it. Account for nested multiplication. Distinguish returned rows, fetched rows, bytes, physical requests, sequential dependency rounds and concurrent work. Pagination and lookahead may fetch more than the caller receives. One application request need not mean one database statement or a fixed amount of server work.

State whether each limit applies to a page, branch, request or whole invocation. Before executing a dynamically produced continuation, check it against both its local allowance and the invocation's remaining budget. Giving every new branch a fresh ceiling does not limit the total. Validate limit inputs and keep runtime accounting separate from a static plan that does not yet contain future continuations. Concurrent branches share the whole invocation's allowance according to its scheduling rules.

Test underestimated fanout, a continuation declaring too little work, repeated dynamic dependencies and individually valid branches whose total exceeds the allowance. Include a permitted path and a static plan refused before execution. Ask equivalent questions through independent, dependent and deduplicated routes. Equal results do not imply equal requests, latency or permission to execute. Inspect actual dispatch and partial work after refusal or failure. Label estimates separately from measured trace fields.

Show incomplete or truncated coverage to consumers. The owner chooses ceilings, consistency, permissions and failure handling. Introduce a plan abstraction only when it provides a needed capability; an ordinary limited read need not acquire a new query language or interpreter.

## Output and stop condition

In the design review or reply, describe component responsibilities, ports, adapters and any misplaced rule or dependency to repair. Stop splitting when each component protects a distinct responsibility and another abstraction would merely rename calls. This method does not require framework scaffolding or a wrapper around every dependency.

## Before handing off

- [ ] Keep decisions and ports with the domain that owns them.
- [ ] Check dependencies and transaction scope at the changed edge.
- [ ] Exercise the domain with a replaceable adapter.
