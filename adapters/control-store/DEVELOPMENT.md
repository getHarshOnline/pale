# Storage-source regeneration

The migration SQL is authored source. Its exact byte digest and the separately derived resultant SQLite schema digest belong to the existing storage manifest. Do not hand-edit expected digests after changing initial-v1 DDL.

The owner-local development generator is an explicitly ignored test-harness entry point, not a behavioral passing test or runtime-readiness gate. It adds no runtime API, SDK operation, feature, or dependency. Run it from the repository root with the existing pinned offline toolchain, naming an absolute directory that does not yet exist:

```sh
PALE_STORAGE_GENERATOR_OUTPUT=/absolute/new/output-directory \
cargo test --offline --locked --jobs 2 -p pale-adapter-control-storage --lib \
  sources::tests::regeneration::regenerate_storage_manifest -- --exact --ignored --nocapture
```

It uses the manifest and both migration sources embedded by that build. It checks the original manifest pin and existing constant/profile policy, derives both schemas using the pinned bundled SQLite and the existing schema projection, updates only the four derived digest values, and runs the actual source verifier against the result before writing anything. Profile selections are not filled or normalized. A stale source pin or changed non-derived contract must be resolved by its owner; this tool cannot invent that approval.

The new directory contains exactly:

- `pale-control-storage.v1.json`: pretty JSON, with its identity derived from JCS.
- `storage_manifest_pin.rs`: the generated Rust pin for that same manifest value.

The tool never replaces live files or an existing output directory. A failed output write may leave a partial directory for inspection; only a successful invocation emits the complete pair. These development files carry no crash-safe publication guarantee.

Review and adopt both outputs together at `contracts/manifests/pale-control-storage.v1.json` and `adapters/control-store/src/storage_manifest_pin.rs`, using exact source guards. Rebuild and run the normal source-verification, store/profile and relevant custody tests against the same SQL. No manual digest substitutions or verifier bypasses are part of this workflow. Unresolved runtime profile requirements stay unresolved even when generation and source validation succeed.

The non-ignored tests under `sources::tests::regeneration` prove rendering, non-derived-value preservation, actual changed-schema derivation, verifier rejection and new-directory-only output behavior. Report their results separately from the explicitly invoked development generator.

## Existing stores after collection-journal separation

Collection entries now use a journal and head keyed by the exact operation handle. Resident authority recovery reads only the authority journal. Both remain in the deployment database, and a collection append commits atomically with its observation, checkpoint and indexes.

This changes the unreleased initial-v1 deployment schema and the shared storage-manifest identity. An existing pre-change trust or deployment database is rejected as `SourceIdentityMismatch`; the deployment authority adapter refuses the old deployment store, and initialization refuses an occupied destination. No automatic migration, payload relocation or version relabeling is implemented. The previous schema and manifest identities are preserved in the refusal fixture, which verifies that retained mixed-journal bytes and the database file remain unchanged.

Keep existing databases with their matching source build for inspection. Moving saved history to this layout requires an explicit migration and recovery implementation that validates the original authority and collection associations and preserves their checkpoints and independent anchors. Regenerating source pins does not perform that migration, and editing a database's identity fields cannot substitute for it.
