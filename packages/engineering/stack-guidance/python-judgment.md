# Write Python with explicit inputs and cleanup

Follow the repository's formatter, import and lint rules. Prefer immutable data where practical, including tuples and frozen dataclasses, and avoid mutable default arguments. Type public interfaces and structured data, then use the project's type checker to check that callers fit those interfaces.

## Keep data and resources understandable

Use protocols and structural behaviour where a concrete base class adds no useful contract. Keep functions small and composable. Dataclasses suit clear records, comprehensions suit readable transformations over a known collection, and generators help iterate without retaining the whole input. Use context managers for files, locks, connections and resources that need cleanup on every path.

Raise specific exceptions with useful context and catch them where recovery is meaningful. Avoid bare catches, silent failure and surprising work at import time. Inspect deserialization, subprocess arguments, import effects and dependency injection where dynamic values enter the system.

## Test the public behaviour

Use parameter sets for important input classes and narrowly scoped setup for resources. Replace clocks, networks, filesystems and other outside systems where the code calls them. Keep the behaviour under test real; a test that mocks all internal cooperation may only confirm its stubs.

Distinguish fast isolated checks from those that need an environment, including asynchronous cleanup and failure cases. Use coverage to locate meaningful gaps. In the reply, identify which kinds of check ran and any environment-dependent behaviour left untested.

## Parse an external record

Validate an external JSON value's fields before returning a typed record. Open the source file with a context manager. Test a valid record and a missing field, and distinguish a parse error from failure to access the file.

## Before handing off

- [ ] Is external data validated before typed code relies on it?
- [ ] Do cleanup and exceptions have a clear owner?
- [ ] Were the actual caller and a meaningful failure case exercised?
