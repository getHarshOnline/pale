+++
description = "Review a PostgreSQL change using supplied queries, migrations, plans and test results."

[role]
uses = [{ scope = "local", resource_id = "migrate" }]
applies_to = [{ scope = "local", resource_id = "postgres" }]
+++

# Review a database change

Review the supplied migrations, queries, execution plans and statistics using the PostgreSQL guidance. Work from the material the requesting task provides. Do not open a live database on the review's behalf or infer performance from how a query looks.

## High-signal failures

- A migration's order, locks, backfill, constraints or rollback prevent a safe rollout alongside the supported application versions.
- A reachable query permits injection, crosses tenants, bypasses a row policy or uses wider privileges than its task requires.
- A transaction exposes partial state, holds locks during external work, acquires locks in an inconsistent order or allows concurrent queue or idempotency operations to race.
- An execution plan shows scans, joins or sorts the workload cannot afford; repeated calls demonstrate N+1 work; or pagination cost grows beyond the workload's requirement.
- Types, keys, constraints or deletion rules permit invalid records or orphaned references.

An index or naming convention is a lead to investigate, not a defect by itself. Report demonstrated injection or missing tenant isolation as CRITICAL. Judge other rollout and performance findings by the users, records and operations actually affected. Put each finding, its supporting query or test and any missing evidence in the review reply.

## Before handing off

- [ ] Does each finding follow from a supplied query, migration, plan or test?
- [ ] Were access, transactions and stored relationships checked separately?
- [ ] Are missing database observations left with the owner who can obtain them?
