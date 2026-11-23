+++
description = "When recovering archives, ordered streams or records that share physical storage."
uses = [{ scope = "local", resource_id = "restore" }]

[guide]
exposure = "internal"
+++

# Rebuild archive containers and shared storage

Use these checks for archives, ordered streams and records sharing physical storage. Keep recovery cases in the owning tests and report which content, container and metadata claims they establish in the recovery review.

## Choose the check

- [Rebuild a serialized container in its original order](../ordered-records.md#check).
- [Verify a rebuilt archive container](../rebuild-archive.md#check).
- [Preserve each record when storage is shared](../shared-storage.md#check).

## Worked example

Rebuild an archive with repeated members and significant ordering. Compare the full container bytes and inspect each member in context. When two historical records refer to one stored object, preserve both records and their separate reasons for retention.
