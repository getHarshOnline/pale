# Rustdoc mathematical contracts

This development-only tool translates the layout-sensitive TeX laws owned by
individual crates into static MathML that Rustdoc includes verbatim. KaTeX is
an authoring-time parser and translator only: generated documentation loads no
JavaScript, stylesheet, font, CDN, or other network resource.

The only toolchain prerequisite is Node.js 20 or newer. The script checks that
floor directly; no package-manager install, `package.json`, lockfile, or
`node_modules` tree participates in this bounded translator.

Each source ends in `.display.tex` or `.inline.tex`; its generated peer replaces
only `.tex` with `.mathml`. Rust source includes the generated file from the
owning crate. Default validation discovers every crate-local `docs/math` root
across the repository's core, adapter, SDK, interface, integration,
and tool layout. Every actual equation or algebraic law has one crate-local TeX
owner, regardless of length; other documentation explains or links to that
owner instead of restating it. Ordinary code, configuration, command, and
pipeline examples remain code. Isolated facts that do not clarify a law stay
as precise prose rather than earning decorative notation.

Regenerate after changing a formula:

```sh
node tools/rustdoc-math/render.mjs --write
```

The validation route is:

```sh
node tools/rustdoc-math/render.mjs --check
node --test tools/rustdoc-math/tests/render.test.mjs
cargo test --manifest-path tools/repository-quality/Cargo.toml --test source_organization rustdoc_math_ --locked
node tools/rustdoc-math/render.mjs --check-rustdoc
RUSTDOCFLAGS="-D warnings" cargo test --workspace --doc
```

`--check-rustdoc` builds the root workspace, the benchmark-only control-storage
law, and every direct standalone Cargo source home with strict private-item
Rustdoc, then verifies the complete rendered tree. It reuses `CARGO_TARGET_DIR`
when that variable is set; otherwise it uses the repository's `target`
directory. This keeps Cargo's cache portable and prevents interrupted checks
from stranding complete build trees in the system temporary directory. It uses
only each source home's selected features; the root workspace's optional
GraphBLAS backend remains outside this documentation gate.

The vendored module and licence are hash-checked against `vendor.json`; that
metadata also records the exact source-archive URL, SHA-256, and registry SRI.
The Rust integration gate structurally parses ordinary and macro-owned doc
attributes, enforcing one source owner per generated law without treating
comments or string literals as evidence. Upgrades require an
exact release-coordinate review, a complete generated-fragment byte diff, and
regeneration of the repository SBOM and notices. This gate proves local
source-level Rustdoc maintainability only; it is not crates.io, docs.rs, manual,
publication, or release evidence.

The rendered-tree check requires every MathML root to match a governed fragment
and every governed fragment to appear. Rustdoc may render the same owning item
at a public re-export and its private module when private-item documentation is
enabled; those mechanically repeated projections are permitted and do not
create a second formula owner. Literal delimiter-like text inside rendered code
declarations and code blocks is likewise not interpreted as prose mathematics.
