+++
description = "Plan schema and data changes around running readers, writers, backfills and recovery."
see_also = [{ scope = "local", resource_id = "data/history" }, { scope = "local", resource_id = "data/old-readers" }]

[guide]

[task]
uses = [{ scope = "local", resource_id = "restore" }]
outputs = ["In the migration plan: scope, starting state and expected result.", "Rehearsal, refusal, rollback and reconciliation conditions."]
+++

# Plan changes to stored data

A migration changes how durable state is interpreted. Plan compatibility and recovery across schema, data, application versions, readers, writers, operators and backups. Reviewing the DDL alone cannot establish those properties.

Identify current and target meanings, data volume and sensitivity, active reader and writer versions, traffic, availability needs, database capabilities, the backup's saved state and who may approve production changes. Find constraints, indexes, generated values, triggers, replicas, exports, jobs and downstream consumers that depend on the old form.

Choose the transition for the versions that must coexist. Expanding the schema, migrating data, switching callers and removing the old form can support overlap, but each step has risks: dual writes can diverge, backfills can race live writes and defaults can conceal unmigrated records. State which copy governs each phase, how repeated work is deduplicated, where it resumes, how differences are reconciled and what permits the next step.

Assess locks, transaction duration, index construction, write amplification, replication lag, query plans, storage growth, batch sizes and failure at each step. A down migration may not recover lost information. Choose rollback, forward repair, restore or compensation according to what can be recovered and which versions are still running.

In the migration plan, explain the meaning that changes, supported reader/writer combinations, phases and state owner per phase, lock and other risks, reconciliation, recovery, acceptance checks, abort conditions and when the old form can retire. The owning database and operator still execute the migration.

Identify the migration file revision separately from the resulting schema and data. Verify all three through their owning tools. Rehearse on an isolated copy of the actual predecessor and retain before-and-after observations in the existing migration test results. A matching header does not prove tables, indexes, triggers or views are unchanged; inspect actual state. Data can change while the schema remains identical. Neither check substitutes for the other. Recover failed or interrupted runs through the database's own procedure.

| If you need to do this | Read this |
| --- | --- |
| Resolve migration history before modifying it | [Read the guide](data/history.md) |
| <a id="keep-legacy-reads-separate-from-successor-writes"></a>Separate legacy reads from successor writes | [Read the guide](data/old-readers.md#keep-legacy-reads-separate-from-successor-writes) |

Return the commands or tests needed for the disposable rehearsal, together with the point at which rollback is possible and where reconciliation or forward repair is required. Preparing this plan does not apply the migration.

## Before handing off

- [ ] Name the old and new state and which consumers can coexist.
- [ ] Check the backfill, validation and recovery points.
- [ ] Leave any production cutover decision explicit.
