# Diagnose a local installation

`pale doctor` reports what it can establish from the compiled executable, an explicitly selected workspace and the machine's declared bindings. It keeps failed checks alongside independent results, with an explanation of what to fix. It changes no workspace files, installs nothing and performs no recovery or cleanup.

Build the [CLI](getting-started.md#build-the-command-line-tool), then run:

```sh
pale --workspace /absolute/workspace/pale-workspace.toml \
  --project demo --execution-workspace default --machine local doctor
```

The four selectors use the ordinary workspace compiler. Without the complete selection, doctor still checks the compiled platform and embedded contracts and reports dependent checks as unavailable. Reports can contain selected private paths and source facts; keep them within the workspace's disclosure scope. A workspace failure retains the compiler's diagnostic code, causes, notes and safe facts; it does not suppress separately requested build measurements.

## Read the checks

Each check is `passed`, `failed`, `unavailable` or `not_selected`. A pass establishes only the named observation. `unavailable` means required declarations or an implemented inspection path are missing; `not_selected` means the compiled instance does not select that optional state root or any agent execution path. Neither a successful compilation nor a matching generated file establishes resident readiness or permission to work.

The current command checks the compiled platform; admission of the embedded contract registry; workspace and distribution-lock compilation; selected repository directories and authority directories or exact files; declared reading-order containment; the bound Pale executable and native harness identities; selected state roots; and installed repository files against their admitted projection. Contract admission uses the existing registry checks, including raw schema and descriptor identities and offline reference closure. It does not regenerate schemas or repair a lock. Executable checks use the existing bounded probes of the exact declared binaries.

Exact toolchain pins, an immutable executable release with component references and notices, the core/MONAD component-selection constraint, and owner-native compatibility packets have no complete declaration or inspection path in this command. They remain unavailable. The command also lacks an authenticated resident diagnosis and actual host callback qualification: a socket, process or matching hook file cannot substitute for recovery, generation, session or enforcement evidence. Gas Town continues to own its Beads and Dolt inspection. The [trust and recovery design](maintainers/trust-and-recovery.md) describes the remaining deployment work.

Add `--result-format json` before `doctor` for the ordinary CLI envelope. Its `result` contains `checks_passed`, `checks` and `builds`. `completion: "complete"` means the report finished, including unfavorable or unavailable observations. Exit status is `0` only when all selected checks pass; failed or unavailable checks produce `1`. The current missing deployment paths therefore keep a full diagnosis from returning success. CLI parsing and SDK-initialization failures retain their ordinary diagnostic behavior.

## Measure build directories

Select up to 32 absolute physical directories. Doctor does not discover or clean build trees:

```sh
pale --result-format json doctor \
  --build-root /absolute/repository/target \
  --build-root /absolute/other-repository/dist \
  --build-limit 10737418240
```

The optional ceiling is in logical regular-file bytes per tree; equality passes. Every selected root has a result, including empty, missing and incomplete trees. A result includes `logical_bytes`, `file_count`, `skipped_symlinks`, `complete` and `errors`. Hard links count once per directory entry and sparse files count their logical lengths. These numbers are not allocated, exclusive or reclaimable space. Do not add overlapping roots together.

Roots reached through symlinks are refused and links inside a tree are skipped. Confined traversal admits at most one million entries and 128 levels per tree. A traversal failure leaves the total absent; later file-measurement failures retain an explicitly partial total and up to 32 errors. Concurrent builds can change a tree during the scan, so the report is not a snapshot. Review errors before using a total, and obtain the owner's deletion decision separately.

The transitional vault tool uses this same native measurement. From the repository root, select the built executable explicitly:

```sh
python3 tools/vault.py --root /absolute/knowledge \
  --pale-executable /absolute/path/to/pale \
  --build-root /absolute/repository/target --json builds
```

Its `check` command accepts the same executable and build options. The Python consumer never scans build files or builds Pale itself; an absent, incompatible or timed-out executable becomes an incomplete measurement. Other unavailable doctor checks do not discard a successfully completed build result. The vault tool retains its own existing finding format for read errors and size ceilings.

## Keep build reuse separate from cleanup

Keep each working copy's Cargo target directory separate, as described in the [source layout](source-layout.md#contracts-are-generated-from-rust). Reusing a target can let Cargo accept another copy's stale output when source timestamps are older than its fingerprints. Disabling incremental compilation limits growth but does not fix that failure.

Compiler-result caching is a different mechanism. The bounded check below exercises an explicitly selected `sccache` with Rust 1.94.0, separate targets and old timestamps. It requires correct output from different source copies, a real cache hit for an unchanged dependency at one shared path, and invalidation when that dependency's `include_str!` input changes:

```sh
python3 tools/check_compiler_cache.py \
  --sccache /absolute/path/to/sccache \
  --toolchain 1.94.0 --work-root /absolute/disposable-checks
```

The check uses one private local server and a 128 MiB cache beneath its own temporary directory. It stops that server and removes only its fixture when finished. It neither changes existing targets nor installs a wrapper or global setting. Copies have equal package names and versions, but distinct path-source identities; the unchanged dependency retains one source path. A passing result qualifies those tracked Rust inputs only.

The [sccache Rust limitations](https://github.com/mozilla/sccache/blob/v0.19.0/docs/Rust.md) include untracked files read by procedural macros and uncached crates that invoke the linker. Its [local cache](https://github.com/mozilla/sccache/blob/v0.19.0/docs/Local.md) requires one server per cache directory. Keep those limits in any deployment qualification; this check does not justify sharing Cargo targets, enabling arbitrary source generators or removing existing build material.
