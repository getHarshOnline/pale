# Machine-backed materialization fixture

This complete workspace selects Claude Code and Codex execution paths for
project `act`. Its machine binding selects repository and authority roots, the
Pale executable, two native harnesses, and local evidence storage. Tests use it
for the real `plan` → `apply`/`remove` materialization and recovery boundaries.

The workspace declaration is defined once here as data. The CLI-local
loader at `tests/support/activation_fixture.rs` materializes it without reading
another crate's private test tree. Its current consumers are:

- CLI integration tests (`tests/cli.rs`); and
- mutation and recovery integration tests.

The files present here are static fixture data. The loader copies them verbatim,
then renders only the machine-specific placeholders in
`bindings/machines/m.toml.tmpl`. The neutral two-path `composition.toml` remains
unchanged.
The loader materializes the schema-current, two-Package distribution defined by
`tests/support/distribution.rs` and derives its exact lock through the public
distribution inspector. The materialization test therefore exercises
portable operation, lifecycle, and run-mode contracts without reading,
changing, or pretending to extend the accepted Package corpus.

| placeholder            | meaning                                                    |
|------------------------|------------------------------------------------------------|
| `@DISTRIBUTION_ROOT@`  | absolute path of the isolated Package test distribution    |
| `@TARGET@`             | absolute path of the `alpha` target repository             |
| `@AUTH@`               | absolute path of the `auth_a` read-only authority root     |
| `@EXEC@`               | absolute path of the selected `pale` executable            |
| `@DIGEST@`             | SHA-256 of the selected executable bytes                   |
| `@VERSION@`            | that binary's exact version (`CARGO_PKG_VERSION`)          |
| `@STATE_ROOT@`         | absolute machine-local root for ephemeral hook state       |
