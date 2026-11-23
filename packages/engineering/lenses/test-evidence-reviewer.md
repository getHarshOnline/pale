# Review whether the tests support the claim

Independently judge whether the supplied tests would fail if the claimed behaviour were wrong. Use [the shared engineering method](../judgment/common.md) to inspect the assertions, then trace each requirement to its test boundary, fixture state, relevant failure cases and executed result. Challenge tests that mirror implementation, pass for the wrong reason, depend on hidden order or omit a failure path required by the claim.

For a journey, inspect setup, state transitions, assertions, cleanup and failure output. Distinguish a product defect, a test defect, a harness failure and an uncertain environment so the finding points to the right repair. For test-first work, confirm that the initial failure represented the missing behaviour and that the final test would catch its return.

Return findings tied to the affected requirement, the missing checks and what the available results support. The parent operation owns test design, source edits, execution and any output files; this review does not create another set of them.
