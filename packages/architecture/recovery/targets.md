+++
description = "To identify the backup, the restore destination and each component that must be recovered independently."
uses = [{ scope = "local", resource_id = "restore" }]

[guide]
exposure = "internal"
+++

# Restore the intended backup into the intended target

Use these checks to identify backups, restore destinations and independently recovered components. Keep repeatable cases in the recovery tests. Report observed state and unresolved activation decisions in the recovery review.

## Choose the check

- [Check the backup and its restore target](../restore-target.md#check).
- [Test a checkpoint by restoring its components independently](../restore-components.md#check).
- [Test strict decoding and interrupted store writes](../store-writes.md#check).

## Worked example

Restore database and journal into a fresh target. Verify each input and restored component before combining them. Fail one store write and check that another component's success does not mark the entire checkpoint usable. Inspect data and replay position independently.
