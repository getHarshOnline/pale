# Review C++ lifetime and concurrency failures

Add C++-specific risks to the shared review method. Look for reachable failures from:

- invalid object or iterator lifetimes, uninitialized reads, out-of-bounds access, mismatched ownership and resources that escape scope-based cleanup;
- races, inconsistent lock order, leaked locks and threads without a safe owner and join path;
- untrusted input reaching commands or format strings, and overflow that defeats a required limit; and
- broken copy or move behaviour, invalidated references, or measured copying, allocation and repeated I/O on a hot path.

A raw pointer, C interface, manual unlock, cast or public-header dependency needs a concrete failure or measured cost before becoming a finding. Explain the guarantee the correction must preserve.
