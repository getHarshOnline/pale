+++
description = "Write and review C and C++ code."
uses = [{ scope = "local", resource_id = "change" }]
official_sources = [{ label = "C++ Core Guidelines", url = "https://isocpp.github.io/CppCoreGuidelines/CppCoreGuidelines" }]

[guide]

[role]
extends = [{ scope = "local", resource_id = "review" }]
applies_to = [{ scope = "local", resource_id = "cpp" }]

[stack]
role_uses = [{ scope = "local", resource_id = "cpp" }]
+++

# Keep C++ ownership and lifetimes explicit

Apply the common coding method after identifying the repository's C++ standard, compiler, build, warnings and analysis settings.

Prefer values and containers whose lifetime follows their scope. Use smart pointers where ownership really transfers or is shared. For raw pointers and references, establish lifetime and whether absence is permitted. Make copy, move and destruction follow the resource's actual ownership, including what happens when an exception interrupts the operation.

Inspect narrowing, overflow, bounds, aliasing, initialization, object lifetime, concurrency and calls across foreign interfaces for reachable undefined behaviour. Give locks an owner and consistent order, and waits an explicit predicate. Asynchronous or detached work must finish before the borrowed state it uses is destroyed.

State the requirements of generic code and check whether it makes valid implementations easier to substitute. Keep ordinary control flow visible. Choose exceptions or error values consistently at each public interface, and keep cleanup from failing.

Exercise copy, move and failure paths through the actual callers. Use focused behaviour tests, the actual build and the sanitizers or static checks that address the risk. Investigate intermittent lifetime or concurrency failures. A passing sanitizer run covers what it exercised, so explain any remaining paths in the reply along with the required guarantee, reachable failure and proposed correction.

## Move a file handle

Put a file handle in a move-only RAII type so destruction releases it. Move that value into the consumer and exercise an early error return. Check that the original no longer owns the handle and that destruction closes it exactly once.

## When reviewing as a specialist

Add C++-specific risks to the shared review method. Look for reachable failures from:

- invalid object or iterator lifetimes, uninitialized reads, out-of-bounds access, mismatched ownership and resources that escape scope-based cleanup;
- races, inconsistent lock order, leaked locks and threads without a safe owner and join path;
- untrusted input reaching commands or format strings, and overflow that defeats a required limit; and
- broken copy or move behaviour, invalidated references, or measured copying, allocation and repeated I/O on a hot path.

A raw pointer, C interface, manual unlock, cast or public-header dependency needs a concrete failure or measured cost before becoming a finding. Explain the guarantee the correction must preserve.
