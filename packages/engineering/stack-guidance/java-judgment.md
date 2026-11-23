# Model Java states and dependencies explicitly

Apply the common coding method with the repository's pinned Java version, build, modules, formatter and static checks. Use framework-specific guidance for behaviour supplied by a framework, and check what that repository actually configures.

## Represent meaningful states in types

Prefer immutable values and final fields. Keep partially initialized objects from escaping. Use records for transparent data carriers, sealed hierarchies for genuinely closed variants, and domain types when absence, failure, identity or units matter.

Use `Optional` at return interfaces where it helps express absence. Design required fields, collection elements and persisted state explicitly. Avoid raw types and unexplained unchecked casts, which defer a type obligation until runtime.

## Make dependencies and concurrent work visible

Supply the dependencies an object needs through construction instead of fetching mutable global state. Keep I/O, time, randomness, environment access and outside clients behind small interfaces. Give packages and modules responsibilities that can be understood and tested independently. Use a builder when construction has real optionality or staged validation.

Use streams for clear finite transformations. Use loops when they make control flow, early exit, mutation, error handling or measured performance clearer. Choose concurrency deliberately rather than applying parallel streams everywhere. For asynchronous work, establish thread ownership, cancellation, timeouts, backpressure and where blocking may occur.

## Preserve failures the caller can act on

Distinguish invalid input, unavailable dependencies, conflicts, timeouts and internal defects when they need different responses. Keep the cause and useful context while protecting credentials and identifiable user information. Propagate failures instead of swallowing them or returning false success. Retry only when repetition is safe and the stop condition is known.

## Check behaviour through the right interface

Test through the smallest public interface that exercises the requested behaviour. Use parameterized or nested cases when they clarify the decision table. Substitute outside dependencies where appropriate while retaining the behaviour under test. Use a real disposable dependency when the claim concerns transactions, isolation, serialization, queries, messaging or another interaction whose actual semantics matter.

Use the repository's declared build and test commands, with separate checks for packaging, migration, startup or outside integration where needed. Inspect parameter binding, effective server-side permission checks, file paths, process execution, serialization limits, secrets and error translation. Verify framework defaults and annotations against the actual repository.

## Translate a request into domain data

Check an API request's required fields and units before making a domain record. Inject a repository interface through the constructor. Exercise invalid input and a failed write through the same service entry.

## Before handing off

- [ ] Are domain types and input mapping explicit?
- [ ] Does the change use the project's supported Java features and dependency style?
- [ ] Were the relevant integration behaviours actually checked?
