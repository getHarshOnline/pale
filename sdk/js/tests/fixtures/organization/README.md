# Organization source fixture

These five TOML files are exact localized copies of the already adopted Rust
SDK organization fixture. The application owns their compiler-backed builder
and explicit development export command at
`crates/pale-application/tests/internal/workspace/organization_inspection.rs`.
The source route and command are documented in the owning Rust SDK fixture
README. Qualification must compare all five exact emitted files to these
localized bytes; do not hand-edit the originals or import a sibling-private
fixture implementation into this package.

The fixtures contain local declarations with explicitly omitted publisher and
adopter modules. Successful compilation is not imported publisher admission,
recipient disclosure, native realization or runtime occurrence admission.
The behavioral test makes explicitly named in-memory source mutations and
recompiles them through PaleJS when asserting admitted behavior.

`npm test` runs `tests/organization.mjs` both through the actual native ABI and
an independently installed packed package. The strict package consumer also
checks `tests/organization_consumer.ts` against actual generated declarations.
No fixture or development-only runner is shipped as a public protocol.
