+++
description = "When consumers interpret a contract, intermediaries rewrite records or an export takes part of a source, to check that meaning and effects survive."
uses = [{ scope = "local", resource_id = "contracts" }]

[guide]
exposure = "internal"
+++

# Keep meaning and effects through translation

Use these checks when consumers interpret a contract, intermediaries rewrite records or an export selects part of a source. Keep repeatable checks in the owning tests and report results and unresolved mappings in the review.

## Choose the check

- [Verify the meaning preserved across an edge](../meaning.md#check).
- [Verify what an intermediary preserves when it writes](../record-writes.md#check).
- [Check what an export includes and omits](../export.md#check).

## Worked example

An intermediary reads a field it does not expose and writes the record back. Compare source and result to see whether the field, precision and update behaviour survive. If an export supports only a subset, name it and verify excluded data stays out.
