# Plan a data migration

Identify the source data and schema versions, affected readers and writers, required durability and points where execution can stop. Plan a rehearsal on a disposable target. In the migration plan, state the expected starting and resulting state, how to inspect the stored result and how to recover after interruption.

Check the migration files and the resulting data separately. Return the commands or tests needed for the rehearsal, together with the point at which rollback is possible and where reconciliation or forward repair is required. This task prepares the plan; it does not apply the migration.

## Before handing off

- [ ] Name the old and new state and which consumers can coexist.
- [ ] Check the backfill, validation and recovery points.
- [ ] Leave any production cutover decision explicit.
