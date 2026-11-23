# Review Rust ownership errors and concurrent work

Apply the inherited Rust guidance to the changed code, callers and relevant configured check results supplied by the requesting task. Investigate a failed check as part of reviewing the code.

Look for reachable failures from:

- panics on recoverable or untrusted input, and `unsafe` code whose required guarantee is absent or broken; a locally justified `unwrap` or `expect` is not itself a finding;
- discarded `must_use` results, lost error context, wildcard matches that hide new variants and contradictory states the types should reject;
- ownership clones or coupled lifetimes that obscure responsibility or materially harm a measured path or API contract;
- blocking work in asynchronous code, unlimited channels or tasks, inconsistent lock order and synchronization that fails under cancellation or concurrent work; and
- untrusted data reaching queries, commands, paths or deserialization, plus measured repeated allocation or N+1 I/O.

A stylistic lifetime change, clone removal, lint suppression or use of `unsafe` needs a broken guarantee, reachable failure or measurement before becoming a finding. Explain that consequence in the review.
