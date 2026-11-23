# Architecture

Package ID: `pale.architecture` · [Manifest](package.toml)

Keep rules with their owners across interfaces, data transformations, storage, contracts, releases and recovery.

A workspace selects entries from this family for the duties its people and agents perform. To read it directly, open the page for your task and follow its reading links.

## Guides

- [Check a record transformation](<transform.md>) · `transform` · `doctrine`
  Check record meaning, keys, incremental coverage and every write or delivery a pipeline performs.
- [Check a release's contents and source records](<release-materials.md>) · `release-materials` · `doctrine`
  Compare release contents with source records, licence notices and required source delivery.
- [Check commands that may be retried](<retry.md>) · `retry` · `doctrine`
  Test command retries, saved outcomes, concurrent writes and lease takeover without duplicating effects.
- [Check deletion and its dependent copies](<erase.md>) · `erase` · `doctrine`
  Check deletion across copies, dependent views and backups without reviving removed data.
- [Check identity and permissions across a binding](<identity.md>) · `identity` · `doctrine`
  Check identity mappings, current permissions, role reassignment and revocation.
  Read as needed: [Keep contract identity and permissions intact](<contracts/identities.md>).
- [Check recovered historical records](<historical-records.md>) · `historical-records` · `doctrine`
  Recover the original historical bytes and check rights to keep and use them separately.
  Read first: [Consume the exact bytes you verified](<verified-input.md>).
  Read as needed: [Verify recovered references from their original base](<historical-links.md>), [Resolve historical bytes after a locator is reused](<historical-locations.md>), [Rebuild a serialized container in its original order](<ordered-records.md>), [Verify a rebuilt archive container](<rebuild-archive.md>), [Recover historical records and their references](<recovery/history.md>), [Verify where restored metadata came from](<restored-metadata.md>), [Prove the replacement route before retiring a recovery copy](<retire-copy.md>), [Preserve each record when storage is shared](<shared-storage.md>).
- [Check snapshot coverage, removals and activation](<import.md>) · `import` · `doctrine`
  Check snapshot coverage, permitted removals and the generation consumers actually see.
- [Check the backup and its restore target](<restore-target.md>) · `restore-target` · `doctrine`
  Check backup reservations, link targets and the actual destination before and after restoration.
  Read first: [Consume the exact bytes you verified](<verified-input.md>).
  Read as needed: [Restore the intended backup into the intended target](<recovery/targets.md>).
- [Check the contract version before integration](<contract-version.md>) · `contract-version` · `doctrine`
  Verify the published schemas and dependencies a consumer pins, including excluded profiles.
  Read as needed: [Keep contract identity and permissions intact](<contracts/identities.md>).
- [Check Vite inputs and browser output](<vite-build.md>) · `vite-build` · `doctrine`
  Check Vite build inputs, plugins, browser-visible settings and production output.
- [Check what an export includes and omits](<export.md>) · `export` · `doctrine`
  Export only permitted members and metadata, with checked revisions, limits and delivery.
  Read as needed: [Keep meaning and effects through translation](<contracts/translation.md>).
- [Check which shared definition the consumer loads](<loaded-definitions.md>) · `loaded-definitions` · `doctrine`
  Identify which definitions the consumer loads, even when different inputs produce equal output.
  Read as needed: [Check the definition and validation used by a consumer](<contracts/definitions.md>).
- [Check who may use the recovered state](<restored-permissions.md>) · `restored-permissions` · `doctrine`
  Prevent restored data from reviving permissions or state older than the independent recovery minimum.
  Read first: [Consume the exact bytes you verified](<verified-input.md>).
  Read as needed: [Check the commit and the next recovery path](<recovery/acceptance.md>).
- [Choose a version-control workflow](<git.md>) · `git` · `doctrine`
  Choose a Git workflow, reassess older changes and follow pull requests through checks and integration.
  Read as needed: [Check fork lineage and what a recipient can reach](<repo/delivery.md>), [Verify what the staged change contains](<repo/staging.md>), [Choose a Git strategy from the actual repository state](<repo/strategy.md>).
- [Choose how to release, observe and recover](<release.md>) · `release` · `doctrine`
  Plan release stages, exposure, observation, compatibility and recovery before widening use.
  Read as needed: [Check field-device paths in the environment they use](<release/devices.md>), [Check standalone and integrated operation](<release/integrations.md>), [Check who owns a running process](<release/processes.md>), [Read back the release and check the recovery path](<release/read-back.md>).
- [Choose Redis structures for the state they hold](<redis-storage.md>) · `redis-storage` · `doctrine`
  Choose and test Redis caches, leases, counters and event streams for their required guarantees.
- [Compare protocols against the same operation](<protocols.md>) · `protocols` · `doctrine`
  Compare discovery, permissions and actual outcomes for one operation across protocols.
  Read as needed: [Compare actual behavior across contract profiles](<contracts/profiles.md>).
- [Consume the exact bytes you verified](<verified-input.md>) · `verified-input` · `doctrine`
  Ensure the reader consumes the bytes it verified, even when paths or nested references change.
- [Design an HTTP interface](<http.md>) · `http` · `doctrine`
  Design HTTP resources, requests, pagination, errors, access controls and compatible changes.
  Read as needed: [Evolve a shared contract with its consumers](<contracts.md>), [Verify the meaning preserved across an edge](<meaning.md>).
- [Evolve a shared contract with its consumers](<contracts.md>) · `contracts` · `doctrine`
  Change a shared interface while checking old and new producers, consumers and migration paths.
  Read as needed: [Check the definition and validation used by a consumer](<contracts/definitions.md>), [Keep contract identity and permissions intact](<contracts/identities.md>), [Compare actual behavior across contract profiles](<contracts/profiles.md>), [Keep meaning and effects through translation](<contracts/translation.md>), [Separate tuning controls from contract changes](<settings.md>).
- [Identify the source a check actually used](<source-state.md>) · `source-state` · `doctrine`
  Identify the source, staged files and local dependencies a build, review or restore actually used.
- [Keep application rules independent of infrastructure](<components.md>) · `components` · `doctrine`
  Keep application rules independent of storage, frameworks and delivery adapters.
- [Keep the reasons for an architecture decision](<decisions.md>) · `decisions` · `doctrine`
  Write down a lasting design decision on the repository’s decisions page: the question, what pushed on it at the time, the real alternatives, the evidence, why this one, what it costs and which earlier decision it replaces.
- [Plan and check a restore](<restore.md>) · `restore` · `doctrine`
  Check that a restore really works: from a known backup, to a known checkpoint, on the real system.
  Read first: [Consume the exact bytes you verified](<verified-input.md>).
  Read as needed: [Verify recovered references from their original base](<historical-links.md>), [Check the commit and the next recovery path](<recovery/acceptance.md>), [Rebuild archive containers and shared storage](<recovery/containers.md>), [Recover historical records and their references](<recovery/history.md>), [Restore the intended backup into the intended target](<recovery/targets.md>).
- [Plan changes to stored data](<migrate.md>) · `migrate` · `doctrine`
  Plan schema and data changes around running readers, writers, backfills and recovery.
  Read as needed: [Resolve migration history before modifying it](<data/history.md>), [Separate legacy reads from successor writes](<data/old-readers.md>).
- [Preserve each record when storage is shared](<shared-storage.md>) · `shared-storage` · `doctrine`
  Share identical bytes while preserving each record’s access, metadata and retention rules.
  Read first: [Check recovered historical records](<historical-records.md>).
  Read as needed: [Rebuild archive containers and shared storage](<recovery/containers.md>).
- [Prove the replacement route before retiring a recovery copy](<retire-copy.md>) · `retire-copy` · `doctrine`
  Before an owner deletes a copy, make sure everything in it can still be recovered: the live version, its history and a copy that does not depend on this machine.
  Read first: [Check recovered historical records](<historical-records.md>).
  Read as needed: [Check the commit and the next recovery path](<recovery/acceptance.md>), [Retire a route only after its replacement passes its own gate](<retire-route.md>).
- [Rebuild a serialized container in its original order](<ordered-records.md>) · `ordered-records` · `doctrine`
  Rebuild a container from its original ordered members and serializer settings.
  Read first: [Check recovered historical records](<historical-records.md>).
  Read as needed: [Rebuild archive containers and shared storage](<recovery/containers.md>).
- [Resolve historical bytes after a locator is reused](<historical-locations.md>) · `historical-locations` · `doctrine`
  Find the original version after its old path or URL has moved or been reused.
  Read first: [Check recovered historical records](<historical-records.md>).
  Read as needed: [Recover historical records and their references](<recovery/history.md>).
- [Retire a route only after its replacement passes its own gate](<retire-route.md>) · `retire-route` · `doctrine`
  Retire an old route only after the domain that replaces it passes its own gate, and account for every route before calling the retirement complete.
  Read as needed: [Prove the replacement route before retiring a recovery copy](<retire-copy.md>).
- [Review dbt models, hooks and tests](<dbt-models.md>) · `dbt-models` · `doctrine`
  Review a dbt model’s actual target, hooks, incremental logic and tests.
- [Review PostgreSQL storage and queries](<postgres.md>) · `postgres` · `doctrine`
  Review PostgreSQL queries, constraints, transactions, access controls and migration behaviour.
- [Separate terminology from compatibility identifiers](<terminology.md>) · `terminology` · `doctrine`
  Change explanatory names while preserving historical meaning and required wire identifiers.
  Read as needed: [Keep contract identity and permissions intact](<contracts/identities.md>).
- [Separate tuning controls from contract changes](<settings.md>) · `settings` · `doctrine`
  Distinguish permitted tuning from contract changes and test refresh and deadline behaviour.
  Read as needed: [Compare actual behavior across contract profiles](<contracts/profiles.md>).
- [Test a checkpoint by restoring its components independently](<restore-components.md>) · `restore-components` · `doctrine`
  Restore components independently, then check whether their combined checkpoint can return to service.
  Read first: [Consume the exact bytes you verified](<verified-input.md>).
  Read as needed: [Restore the intended backup into the intended target](<recovery/targets.md>).
- [Test existing behaviour and the added capability separately](<existing-behaviour.md>) · `existing-behaviour` · `doctrine`
  Test inherited behaviour and new capabilities separately so neither hides the other’s failures.
  Read as needed: [Compare actual behavior across contract profiles](<contracts/profiles.md>).
- [Test strict decoding and interrupted store writes](<store-writes.md>) · `store-writes` · `doctrine`
  Test full record decoding, reference checks and failure between dependent store writes.
  Read first: [Consume the exact bytes you verified](<verified-input.md>).
  Read as needed: [Restore the intended backup into the intended target](<recovery/targets.md>).
- [Understand a codebase for the task at hand](<codebase.md>) · `codebase` · `doctrine`
  Trace a task through real code and verify the source, index, entry points and checks used.
- [Verify a rebuilt archive container](<rebuild-archive.md>) · `rebuild-archive` · `doctrine`
  Compare recovered members, archive bytes and distributed containers as separate identities.
  Read first: [Check recovered historical records](<historical-records.md>).
  Read as needed: [Rebuild archive containers and shared storage](<recovery/containers.md>).
- [Verify generated contracts at the consumer boundary](<generated-definitions.md>) · `generated-definitions` · `doctrine`
  Check generated contracts against their source and through the consumer that loads them.
  Read first: [Verify which validation checks ran](<validation.md>).
  Read as needed: [Check the definition and validation used by a consumer](<contracts/definitions.md>).
- [Verify identity across generated names and locators](<names.md>) · `names` · `doctrine`
  Check that generated names and addresses resolve to the intended records without collisions.
  Read as needed: [Keep contract identity and permissions intact](<contracts/identities.md>).
- [Verify recovered references from their original base](<historical-links.md>) · `historical-links` · `doctrine`
  Resolve recovered links and fragments from the original document’s base and version.
  Read first: [Check recovered historical records](<historical-records.md>).
  Read as needed: [Recover historical records and their references](<recovery/history.md>).
- [Verify the exact commit before advancing recovery](<recovery-commit.md>) · `recovery-commit` · `doctrine`
  Verify what actually committed and reconcile lost replies before advancing recovery.
  Read first: [Consume the exact bytes you verified](<verified-input.md>).
  Read as needed: [Check the commit and the next recovery path](<recovery/acceptance.md>).
- [Verify the meaning preserved across an edge](<meaning.md>) · `meaning` · `doctrine`
  Test whether consumers preserve meaning across translations and changed contract profiles.
  Read as needed: [Keep meaning and effects through translation](<contracts/translation.md>).
- [Verify what an intermediary preserves when it writes](<record-writes.md>) · `record-writes` · `doctrine`
  Check whether a complete read/write path preserves fields, values and seals.
  Read as needed: [Keep meaning and effects through translation](<contracts/translation.md>).
- [Verify where restored metadata came from](<restored-metadata.md>) · `restored-metadata` · `doctrine`
  Check which restored metadata was captured originally and which the restore introduced.
  Read first: [Check recovered historical records](<historical-records.md>).
  Read as needed: [Recover historical records and their references](<recovery/history.md>).
- [Verify which validation checks ran](<validation.md>) · `validation` · `doctrine`
  Compare required checks with assertions actually executed and explain changed coverage.
  Read as needed: [Check the definition and validation used by a consumer](<contracts/definitions.md>).

## Tasks

- [Check a record transformation](<transform.md>) · `transform` · `operation`
  Check record meaning, keys, incremental coverage and every write or delivery a pipeline performs.
  Read first: [Check what an analysis counts](<../evaluation/rows.md>).
- [Check a release's contents and source records](<release-materials.md>) · `release-materials` · `operation`
  Compare release contents with source records, licence notices and required source delivery.
- [Check deletion and its dependent copies](<erase.md>) · `erase` · `operation`
  Check deletion across copies, dependent views and backups without reviving removed data.
  Read first: [Plan and check a restore](<restore.md>).
- [Check snapshot coverage, removals and activation](<import.md>) · `import` · `operation`
  Check snapshot coverage, permitted removals and the generation consumers actually see.
- [Choose how to release, observe and recover](<release.md>) · `release` · `operation`
  Plan release stages, exposure, observation, compatibility and recovery before widening use.
  Read first: [Check a release's contents and source records](<release-materials.md>).
  Read as needed: [Check field-device paths in the environment they use](<release/devices.md>), [Check standalone and integrated operation](<release/integrations.md>), [Check who owns a running process](<release/processes.md>), [Read back the release and check the recovery path](<release/read-back.md>).
- [Evolve a shared contract with its consumers](<contracts.md>) · `contracts` · `operation`
  Change a shared interface while checking old and new producers, consumers and migration paths.
  Read first: [Check commands that may be retried](<retry.md>).
  Read as needed: [Check the definition and validation used by a consumer](<contracts/definitions.md>), [Keep contract identity and permissions intact](<contracts/identities.md>), [Compare actual behavior across contract profiles](<contracts/profiles.md>), [Keep meaning and effects through translation](<contracts/translation.md>), [Separate tuning controls from contract changes](<settings.md>).
- [Keep the reasons for an architecture decision](<decisions.md>) · `decisions` · `operation`
  Write down a lasting design decision on the repository’s decisions page: the question, what pushed on it at the time, the real alternatives, the evidence, why this one, what it costs and which earlier decision it replaces.
  Read first: [Give the next worker enough context to continue](<../engineering/handoff.md>).
- [Plan and check a restore](<restore.md>) · `restore` · `operation`
  Check that a restore really works: from a known backup, to a known checkpoint, on the real system.
  Read first: [Consume the exact bytes you verified](<verified-input.md>).
  Read as needed: [Verify recovered references from their original base](<historical-links.md>), [Check the commit and the next recovery path](<recovery/acceptance.md>), [Rebuild archive containers and shared storage](<recovery/containers.md>), [Recover historical records and their references](<recovery/history.md>), [Restore the intended backup into the intended target](<recovery/targets.md>).
- [Plan changes to stored data](<migrate.md>) · `migrate` · `operation`
  Plan schema and data changes around running readers, writers, backfills and recovery.
  Read first: [Plan and check a restore](<restore.md>).
  Read as needed: [Resolve migration history before modifying it](<data/history.md>), [Separate legacy reads from successor writes](<data/old-readers.md>).
- [Propose an architecture change](<propose-design.md>) · `propose-design` · `operation`
  Compare architecture choices and recommend responsibilities and change order.
  Read first: [Evolve a shared contract with its consumers](<contracts.md>), [Keep application rules independent of infrastructure](<components.md>).
  Read as needed: [Review an architecture independently](<architecture-review.md>).
- [Review a fork and its consumer](<fork-review.md>) · `fork-review` · `operation`
  Check a fork’s source, refs, transport and the version its consumer actually uses.
  Read first: [Choose a version-control workflow](<git.md>).
- [Review a live status view](<diagnostics.md>) · `diagnostics` · `operation`
  Check that a live status view respects permissions, preserves meaning and shows recovery accurately.
  Read first: [Verify the meaning preserved across an edge](<meaning.md>).
- [Review a PostgreSQL change](<review-postgres.md>) · `review-postgres` · `operation`
  Review PostgreSQL queries, migrations and access changes using actual database observations.
  Read first: [Review PostgreSQL storage and queries](<postgres.md>).
  Read as needed: [Review a database change](<database-review.md>).
- [Review dbt models, hooks and tests](<dbt-models.md>) · `dbt-models` · `operation`
  Review a dbt model’s actual target, hooks, incremental logic and tests.
  Read first: [Check a record transformation](<transform.md>), [Check what an analysis counts](<../evaluation/rows.md>).
- [Understand a codebase for the task at hand](<codebase.md>) · `codebase` · `operation`
  Trace a task through real code and verify the source, index, entry points and checks used.
  Read first: [Identify the source a check actually used](<source-state.md>), [Give the next worker enough context to continue](<../engineering/handoff.md>).

## Roles

- [Review a database change](<database-review.md>) · `database-review` · `role`
  Review a PostgreSQL change using supplied queries, migrations, plans and test results.
  Read first: [Plan changes to stored data](<migrate.md>).
  Read as needed: [Review PostgreSQL storage and queries](<postgres.md>).
- [Review an architecture independently](<architecture-review.md>) · `architecture-review` · `role`
  Independently review an architecture proposal, alternatives, risks and required checks.
  Read first: [Evolve a shared contract with its consumers](<contracts.md>), [Keep application rules independent of infrastructure](<components.md>).

## Stacks and tools

- [Check Vite inputs and browser output](<vite-build.md>) · `vite-build` · `stack`
  Check Vite build inputs, plugins, browser-visible settings and production output.
  Read as needed: [Check Vite inputs and browser output](<vite-build.md>), [Build web interfaces around browser behaviour](<../engineering/web.md>).
- [Choose Redis structures for the state they hold](<redis-storage.md>) · `redis-storage` · `stack`
  Choose and test Redis caches, leases, counters and event streams for their required guarantees.
  Read as needed: [Choose Redis structures for the state they hold](<redis-storage.md>).
- [Review dbt models, hooks and tests](<dbt-models.md>) · `dbt-models` · `stack`
  Review a dbt model’s actual target, hooks, incremental logic and tests.
  Read as needed: [Review dbt models, hooks and tests](<dbt-models.md>).
- [Review PostgreSQL storage and queries](<postgres.md>) · `postgres` · `stack`
  Review PostgreSQL queries, constraints, transactions, access controls and migration behaviour.
  Read as needed: [Review PostgreSQL storage and queries](<postgres.md>).
