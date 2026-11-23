+++
description = "Use it to read the configured includes, execution order and applicable profile, and to compare the migration ledger with the actual database state before changing a migration."
uses = [{ scope = "local", resource_id = "migrate" }]

[guide]
exposure = "internal"
+++

# Resolve migration history before modifying it

## Resolve migration history before changing its inputs

Read configured includes, execution order and applicable profile or context. With permitted access, compare the migration ledger and actual database state. A branch name, filename or search with no matches does not prove a migration is unapplied or an object absent. Distinguish source bytes, the tool's checksum and its saved execution identity. Preserve applied history under the database's migration policy. A matching tool checksum need not mean identical bytes; an unmerged branch does not establish that editing a pending file is safe.

Separate skipping a migration already recorded, replaying its statements and recovering an interrupted run. Specify the starting and resulting state for each required case. Existence guards, ignored insert conflicts and repeatable execution do not prove that definitions, data or grants are correct. Inspect the actual result and follow the owner's reconciliation and recovery procedure. Never clear checksums, mark a migration executed or drop and recreate enforcement merely to make a check pass. Put observed ledger/schema differences and the proposed next migration in the migration review.

## Worked example

A deployed database has already recorded a migration under an existing name. Read the recorded migration and the actual schema. Prepare a successor that reaches the intended state. Rehearse it against both a fresh install and an existing predecessor.
