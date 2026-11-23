# Review PostgreSQL storage and queries

Apply this guidance alongside the persistence and migration methods when the project uses PostgreSQL. Judge schemas, queries and concurrency against the actual workload and server observations.

Use constraints and domain-appropriate types to exclude invalid state. Choose indexes for equality, range, ordering, selectivity and update cost. Partial, covering, expression and composite indexes need a measured access path to justify them. Sequential scanning can be appropriate when a query needs much of the table.

Review transactions through MVCC visibility, isolation, lock order, deadlocks and retries. Keep transactions small without splitting one required invariant across commits. Test row-level security with the actual role and query context before relying on it for access control.

Use execution plans and runtime statistics to distinguish estimation errors, skew, missing access paths, N+1 requests, lock waits and maintenance effects. In the database review, identify the violated rule, observations, proposed correction, write/storage costs, migration consequences and remaining checks. Read actual versions, extensions, settings and operations from the project.

## Resolve conflict identity and null semantics

Compare an insert's conflict action with the actual schema. A column or expression conflict target infers a suitable unique index, including its predicate and other inference requirements. `ON CONSTRAINT` names a constraint. An exclusion constraint cannot arbitrate `DO UPDATE`. Check immediate versus deferrable enforcement and arbiters supported by the server version.

Targetless `DO NOTHING` can skip conflicts against usable arbiters. It does not suppress foreign-key, check, not-null or arbitrary execution errors. Without an enforcing key it does not establish deduplication. A batch upsert must not try to update the same existing row more than once.

Decide from the domain whether missing key parts represent the same identity. Unique keys normally permit repeated combinations containing nulls. Require values, use `NULLS NOT DISTINCT` where supported (PostgreSQL 15 onward), or justify another schema design. Do not invent placeholder values merely to force matches. Find existing duplicates before tightening the key.

## Treat queue claims as a state transition

`SKIP LOCKED` lets competing workers pass rows they cannot immediately lock, so its view is deliberately incomplete. Do not use it to claim complete coverage of eligible records. Table locks may still wait. Identify selected rows, transaction, eligibility condition and durable claim before releasing row locks. A released lock alone does not reserve work while an external action runs. Trace completion, leases, retries and fencing through the real recovery procedure. Choose waiting, immediate failure or skipping according to the query's own requirements.

Test changed conflict/null rules against actual DDL and server version. When queue behaviour changes, use competing transactions and interrupted work. Report isolated database tests, migration rehearsals and deployed application behaviour separately. The database owner authorises execution and writes.

## Model a vocabulary from its owner

Identify who can add, rename or retire a value, what its identity means and whether consumers need labels, ordering or availability metadata. A lookup table with a foreign key can manage values and metadata as data. A check constraint can enforce a restricted predicate. A field appearing in a UI does not choose between them. Specify compatibility between code enums and stored identities, including unknown values and rollout order.

Claims that a join or planner cost makes a model unsuitable need measurement on the actual workload. Existing schemas change through their approved migration process. A convention for new columns does not require rewriting old tables.

## Plan enforcement and replay separately

For the PostgreSQL version examined here, foreign-key and check constraints support `NOT VALID`: defer checking existing rows while enforcing later writes. Validation must then scan and accept historical data before the constraint can be relied on for every row. Check each step's actual lock and commit behaviour. This is neither lock-free nor a recipe for all constraint kinds. Dropping and adding a constraint can change enforcement and dependencies; preserve the required rule throughout the transition.

`CREATE TABLE IF NOT EXISTS` and `CREATE INDEX IF NOT EXISTS` may accept an existing name with a different definition. Inspect actual objects, keys, predicates, constraints and privileges required by the change. A successful rerun or skipped statement does not prove the intended schema exists.

## Owning references

- [PostgreSQL 15 conflict actions and index inference](https://www.postgresql.org/docs/15/sql-insert.html#SQL-ON-CONFLICT)
- [PostgreSQL 15 uniqueness and null semantics](https://www.postgresql.org/docs/15/ddl-constraints.html#DDL-CONSTRAINTS-UNIQUE-CONSTRAINTS)
- [PostgreSQL 15 row-lock selection](https://www.postgresql.org/docs/15/sql-select.html#SQL-FOR-UPDATE-SHARE)
- [PostgreSQL 15 staged constraint installation and validation](https://www.postgresql.org/docs/15/sql-altertable.html)
- [PostgreSQL 15 existing-table handling](https://www.postgresql.org/docs/15/sql-createtable.html)
- [PostgreSQL 15 existing-index handling](https://www.postgresql.org/docs/15/sql-createindex.html)

These references qualify the examined semantics, including the version floor for `NULLS NOT DISTINCT`; the adopter resolves its actual server and extensions.

## Worked example

A worker claims one ready row with `SELECT … FOR UPDATE SKIP LOCKED` and updates its lease in the same transaction. Run two workers against the fixture and verify each claims a different row. Keep retries and expired leases separate from the lock itself.

## Before handing off

- [ ] Identify the transaction and access scope of the query.
- [ ] Check constraints, locking and the actual plan.
- [ ] Exercise failure and rollback where writes are affected.
