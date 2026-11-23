+++
description = "Understand a code change, fit it to its callers and check the whole result."
see_also = [
  { scope = "local", resource_id = "error-handling" },
  { scope = "local", resource_id = "response-time" },
  { scope = "local", resource_id = "security" },
]

[guide]

[role]

[policy]
+++

# Change code so it fits

Read the repository's `AGENTS.md` and `CONTRIBUTING.md`, then the code and its callers. Explain in three or four sentences what the code does, what calls it and why it has this shape. Read further when that explanation is still uncertain. Repository and language guidance supplies the tool, style and framework choices; this method helps decide how the parts fit and what would catch a meaningful failure.

For repair and hardening work, first establish a deployable path through the whole system, with its required safety checks and its configuration and documentation in agreement. Then harden that working path incrementally. Keep both outcomes in the task: a functioning system and the protections it requires.

## Work through the change in order

1. **What does it touch?** Trace the ordinary entry point through the change to its callers, stored data and outputs. Include callers a text search might miss: generated bindings, configuration, plugins and external users. Find the cause of a failure before choosing the edit.
2. **What comes in from outside?** Check input where it enters. Keep permissions, secrets and ownership with the component responsible for them; changing an adapter must not silently change who may act.
3. **What is the simplest design that does the whole job?** Compare the parts, types, paths and names a maintainer must understand. Reuse an existing responsibility when it fits, and separate a genuinely different one. Explain which requirement each new part serves. Preserve the required behaviour and useful checks; line count alone cannot establish a simpler design.
4. **What would fail if this were wrong?** Run that check through the normal entry point and keep a useful regression test. Choose a check that reaches the changed behaviour; a command that succeeds without running it leaves the question open.
5. **Would someone who did not write it accept it?** Review the final change against its callers and requirements. Resolve actionable findings and explain any material uncertainty in the reply.
6. **What is left for later?** Put necessary follow-up work on the product's existing work list, with why it can wait and what would make it urgent. A missing requirement or failing check needed for this task is unfinished work, not “later”.

## Find the owner and the real callers

Locate the definition that gives a concept its meaning, its implementation, generated forms and consumers. Duplicated enums, schema copies, canonicalizers and separate command-line and runtime validation paths drift even when their names agree. Generate necessary projections from their owner. One source of meaning can span cohesive files; it need not become one giant file or store.

When CodeGraph is available, anchor a query to the repository, file and symbol you have read. Confirm that a result is the intended definition, not a namesake, and check index scope and freshness. Trace representations, adapters, callers that make decisions and tests of those boundaries. A quiet query establishes no absence: check source, registrations, generated code and dynamic callers before removing anything. If the index is unavailable, use source search and say which relationships you could not establish.

After the change, inspect the same paths for a second definition or validation path, stale callers or docs, public symbols without consumers, test-only structure entering production, or a provider dependency entering the core. Finish when the changed relationships and checks are understood; do not expand this into unrelated cleanup.

## Give each file one job

Split a file when a second responsibility arrives. Group code by the concept it owns and its reason to change, rather than moving whole functions into arbitrary smaller files. A folder module can keep a thin root exposing the same public API while implementation lives in named children. Keep helpers private until a caller needs a wider contract, and avoid re-export chains that hide the definition. In Rust, give each module a `//!` description of its job. Use the same layout for comparable modules so a reader can predict where to look.

Prefer explicit inputs and outputs, immutable bindings and typed constructors that establish valid state. Keep necessary mutation local and I/O at the edges. Name meaningful constants, flatten avoidable nesting and use early returns where they clarify the path. Global registries, hidden singletons, ambient environment reads and service locators make a change reach further than its signature suggests.

Share code when real consumers share a rule, an external boundary needs a stable interface, or security, identity or lifecycle correctness needs one implementation. Similar syntax alone is not a reason: independent owners and lifecycles may need independent code. Avoid speculative traits, generic utility drawers, boolean mode switches and pass-through wrappers that add no validation, translation, policy, observability or useful replacement boundary.

## Keep boundaries small and explicit

Name an interface for the capability it supplies, with explicit input, result and error types. State its ordering, concurrency, timeout, cancellation, retry and recovery behaviour where they affect a caller. Make it small enough to test the same contract against each implementation.

An adapter converts between native and core forms at one boundary and contains transport, authentication and platform details. It preserves the native system's authority and result, writes through that system's interface, and can be removed without corrupting either side's state. Successful dispatch means the request was sent; confirm the resulting effect before claiming it happened. Keep provider and product choices outside the shared core.

Give recoverable errors distinct types when the caller must respond differently: absence, denial, stale data, conflict, timeout, cancellation, unsupported input, invalid input, unavailable service, an effect whose outcome is unknown, or terminal failure. Keep the cause available without leaking secrets. Logs and free-form strings are not a protocol. Separate independent states instead of collapsing them into one flag. For a durable transition, identify when the change commits and what recovery does; for a retry, establish idempotency and what happens if the first attempt may already have taken effect.

## A reusable component holds nobody's values

Take adopter paths, account names, limits, policy and credentials as inputs. Keep them out of reusable implementation and examples, including tests; use synthetic data and temporary directories. A fresh clone must build on every platform the project supports, without the author's home folder or another checkout. Ship required templates, schemas and other assets with the distribution, or embed them at build time; a compile-time checkout path is not a runtime asset location. Keep dependency versions and generated pins in their declared source rather than copying them into tests and docs.

For a solved problem, assess a maintained library before writing another implementation. Check correctness, standards support, security, maintenance, licence, platform behaviour, deterministic operation, API size and transitive cost. Installing a dependency and permitting network access at runtime are separate choices. Follow the repository's lockfile and release-notice conventions. Use version-matched official documentation when the source cannot establish an API claim.

## Comments say what the code cannot

Explain an invariant, non-obvious choice, limitation or reason a simpler-looking alternative is wrong. Make the comment understandable from the code it accompanies: task numbers, private plans and review-round labels cannot supply its meaning. Link related code with the language's documentation links and keep the essential reason here. Public API docs explain ownership, preconditions, effects, errors and recovery. Architecture docs explain relationships across parts; they should not copy every symbol or executable field. Check README commands against the actual entry points, regenerate generated docs, and correct obsolete diagrams or mark genuine history as history.

## Replace in three steps

Put all required behaviour and reasons in the new home, move its callers, then retire the old implementation when removal is authorised. Until the third step, name the remaining caller or reason the old form exists. Do not invent compatibility machinery for unreleased scaffolding. For a released name that must remain supported, make it one alias to the current implementation for the agreed compatibility period; do not maintain competing implementations. Move durable lessons into code, tests or the manual before retiring temporary review material, and keep lasting checks independent of temporary folders and upstream clones.

## Check the behaviour a user or attacker would notice

Arrange the input and state, act through the relevant entry point, then assert the observable result. A path validator should reject traversal and a symlink escape without changing the protected file. A failed update should leave the promised bytes and permissions intact. A stale or forged request should fail before any effect. For identity derived from input, change a meaningful input and check that the identity changes; a hard-coded digest alone can agree with the same mistake twice.

Use pure tests for local transformations, representation vectors for persisted or cross-language formats, shared contract tests for adapters, integration tests for process and storage boundaries, restart and failure injection for durable state, and read-back from the real system for claimed effects. Choose the levels the change needs. Exercise the same promised behaviour locally and in each supported environment; keep timing, order and fixtures deterministic where possible. A shared fixture is data kept once behind a shared loader; a one-off parser input can stay beside its test. An architecture test earns its place when the dependency rule itself protects required behaviour.

Improve assertions and failure cases before adding tests. Avoid tests that mirror implementation, assert trivia or pass despite the defect. Fix the code when a valid test fails; change the test only when its requirement or setup is wrong. A mechanical edit may need only the existing focused check. Follow the repository's formatter, lint and type checks, and report what ran and any remaining gap in the reply; none replaces examining the behaviour.

## Review the risks the change reaches

Give authentication, authorisation, external input, secrets, new endpoints, payments and sensitive data a security review. Validate at entry, encode output for its destination, and check for injection, access-control bypass, exposed credentials and partial writes. Follow the repository's vulnerability-handling policy. Also inspect shared mutation, swallowed errors, hidden work on a hot path, needless allocation or locking, and a changed dependency direction. A performance claim needs a comparable measurement.

| If you are doing this | Read this |
| --- | --- |
| Design a retry, fallback or response to partial state | [Failure semantics](error-handling.md) |
| Decide whether a deadline, queue or cache change helps | [Latency and freshness](response-time.md) |
| Check access to a protected path or artifact | [Security review](security.md) |

## Before handing off

- [ ] Does the whole requested behaviour work through its ordinary caller, with clear owners and no competing meaning?
- [ ] Would the checks catch a consequential mistake, and does the reply distinguish observed results from failures and unfinished work?
- [ ] Can another maintainer understand the responsibilities, reasons, supported environments and remaining obligations without this conversation?
