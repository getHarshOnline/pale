# Regenerating contract fixture coordinates

The negative fixtures and standalone source capsule have different proof owners.
Their development commands update derived coordinates, not test-case meaning or
evidence that a build passed. Both commands use the registry embedded in the
current build. Run them only against the intended canonical bundle successor.

These are ignored test-harness entry points. They are not ordinary passing tests,
runtime APIs, or release operations. They require a new absolute output directory
whose parent already exists. They never overwrite a file or directory. An I/O
failure can leave partial output for inspection; the output is not an atomic
publication mechanism.

## Negative fixture registry

From the repository root, using its pinned offline toolchain:

```sh
PALE_NEGATIVE_FIXTURE_OUTPUT=/absolute/new/negative-output \
cargo test --offline --locked -p pale-contracts --test negative_fixtures \
  regenerate_negative_fixture_coordinates -- --exact --ignored
```

The output contains `fixture-set.v1.json` as pretty JSON. Only its two
`active_registry` digest fields change. Case payloads, case digests, expected
failure coordinates, coverage declarations, and historical/semantic boundaries
remain unchanged. The command validates those existing cases through the same
tests used by the normal suite before writing. It does not require the old
registry coordinates to equal the new ones.

Review the output and adopt it at `tests/fixtures/negative/fixture-set.v1.json`.
Then rebuild and run the complete `negative_fixtures` target without `--ignored`.
A changed rejection or coverage law needs review by its owner; this generator
does not repair that law to make a test pass.

## Standalone source capsule

```sh
PALE_CAPSULE_COORDINATE_OUTPUT=/absolute/new/capsule-output \
cargo test --offline --locked -p pale-contracts --test standalone_bundle \
  regenerate_standalone_capsule_coordinates -- --exact --ignored
```

The command uses the existing confined, symlink-rejecting canonical input
inventory. It derives the schema count and registry identities from the built
contracts, hashes the retained capsule `Cargo.lock`, and hashes the rendered
inventory. It does not run Cargo inside the capsule or change its dependency
lock. The output contains:

- `source-inventory.v1.txt`: the actual sorted canonical input paths.
- `source-capsule.lock.json`: pretty JSON containing their expected identities.

Review and adopt both files together under `tests/fixtures/standalone/`. Then
run the full `standalone_bundle` target without `--ignored`. Its existing
`canonical_contract_sources_build_without_the_repository_parent` test copies
the closed source inventory outside the repository, resolves only the three
declared local crates, and builds the capsule with offline, locked dependencies.
The successful build, not the coordinate generator, establishes the stated
local source-independence evidence. Neither operation establishes publication,
installation, activation, compatibility, or release readiness.

If the capsule's existing dependency lock cannot build the new source, resolve
that dependency change separately and regenerate these coordinates from its
actual reviewed bytes. Do not fabricate a successful build receipt.
