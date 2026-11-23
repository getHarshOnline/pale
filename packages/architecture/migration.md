# Plan changes to stored data

A migration changes how durable state is interpreted. Plan compatibility and recovery across schema, data, application versions, readers, writers, operators and backups. Reviewing the DDL alone cannot establish those properties.

Identify current and target meanings, data volume and sensitivity, active reader and writer versions, traffic, availability needs, database capabilities, the backup's saved state and who may approve production changes. Find constraints, indexes, generated values, triggers, replicas, exports, jobs and downstream consumers that depend on the old form.

Choose the transition for the versions that must coexist. Expanding the schema, migrating data, switching callers and removing the old form can support overlap, but each step has risks: dual writes can diverge, backfills can race live writes and defaults can conceal unmigrated records. State which copy governs each phase, how repeated work is deduplicated, where it resumes, how differences are reconciled and what permits the next step.

Assess locks, transaction duration, index construction, write amplification, replication lag, query plans, storage growth, batch sizes and failure at each step. A down migration may not recover lost information. Choose rollback, forward repair, restore or compensation according to what can be recovered and which versions are still running.

In the migration plan, explain the meaning that changes, supported reader/writer combinations, phases and state owner per phase, lock and other risks, reconciliation, recovery, acceptance checks, abort conditions and when the old form can retire. The owning database and operator still execute the migration.

Identify the migration file revision separately from the resulting schema and data. Verify all three through their owning tools. Rehearse on an isolated copy of the actual predecessor and retain before-and-after observations in the existing migration test results. A matching header does not prove tables, indexes, triggers or views are unchanged; inspect actual state. Data can change while the schema remains identical. Neither check substitutes for the other. Recover failed or interrupted runs through the database's own procedure.

| If you need to do this | Read this |
| --- | --- |
| Resolve migration history before modifying it | [Read the guide](data/migration-history.md) |
| <a id="keep-legacy-reads-separate-from-successor-writes"></a>Separate legacy reads from successor writes | [Read the guide](data/legacy-reads.md#keep-legacy-reads-separate-from-successor-writes) |

## Before handing off

- [ ] List the old and new readers and writers that coexist.
- [ ] Check resumable backfill and recovery before the irreversible step.
- [ ] Read back data and compatibility after the change.
