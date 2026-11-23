# Review a PostgreSQL change

Identify the query, migration or access change, the role that will execute it, the data model and transaction. Inspect the available execution plan and database test results. Check permissions, locking and constraints separately. Where feasible, use an isolated role or policy that should fail as a negative control.

An execution plan supports conclusions about that query and data; it does not guarantee throughput. In the review reply, give findings, missing evidence and the database checks needed to resolve it, including inspection of the stored result. The database owner runs those checks and any writes.

## Before handing off

- [ ] Inspect the actual query, access scope and expected write.
- [ ] Check constraints, locks and the rollback or retry behavior.
- [ ] Read back the result on the intended database.
