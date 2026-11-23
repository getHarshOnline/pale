# Organization inspection source fixture

The five adjacent TOML inputs are localized output of the application's existing
organization/responsibility fixture owner. They are compiled through the public
portable compiler before export and again through the public SDK in these tests.
They are local declarations with explicitly omitted publisher/adopter modules;
they do not prove publisher import, recipient disclosure, native realization, or
runtime occurrence admission. Genealogy occurrences are body-free references.

To regenerate, create a fresh empty absolute output directory, then run from the
repository root with the selected toolchain:

```sh
PALE_ORGANIZATION_FIXTURE_DIR=/absolute/empty/output cargo test --manifest-path crates/pale-application/Cargo.toml --offline --locked --jobs 2 --lib -- --ignored --exact workspace::organization_inspection::tests::export_sdk_organization_fixture
```

This explicitly selected ignored test is a development export command, not an
ignored behavioral test counted as coverage. Its owner lives at
`crates/pale-application/tests/internal/workspace/organization_inspection.rs`.
Check the discovered test name and exactly one executed export. Compare the exact
five emitted files (`pale-workspace.toml`, `organization.toml`,
`responsibility.toml`, `resolution.lock.toml`, `universe.lock.toml`) with these
localized bytes before changing fixtures. Do not hand-edit the localized originals
or import sibling-private Rust fixture modules into SDK tests. Hostile test cases
make explicit in-memory source mutations and must pass actual compilation when
they claim admission.

The organization integration tests run with the normal SDK
`--test portable_workspace organization::` filter. Private ownership/copy-budget
tests run with `--lib portable_workspace::organization::tests::`. The application
detail tests run with `--lib workspace::organization_inspection::details::tests::`.
The exporter is separate from those behavioral counts.
