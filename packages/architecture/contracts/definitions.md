+++
description = "When a consumer imports definitions or generated contracts, to check which validation actually covers them."
uses = [{ scope = "local", resource_id = "contracts" }]
see_also = [{ scope = "local", resource_id = "validation" }]

[guide]
exposure = "internal"
+++

# Check the definition and validation used by a consumer

Use these checks for imported definitions, generated contracts and validation coverage. Keep repeatable cases in the consumer's tests and put the observed inputs, results and gaps in the review reply.

## Choose the check

- [Check which shared definition the consumer loads](../loaded-definitions.md#check).
- [Verify generated contracts at the consumer boundary](../generated-definitions.md#check).
- [Verify which validation checks ran](../validation.md#check).

## Worked example

Two profile builds produce equal output, but one resolved an older shared definition. Compare their actual input trees, load order and generator results. Remove a required validator from a test run and check that the result identifies the missing check even if code still emits.
