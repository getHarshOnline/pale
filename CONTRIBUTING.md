# Contributing to Pale

Contributors will propose changes to this repository after accepting the contributor agreement below, and each change is held to the engineering standard that follows. Pale is local and unreleased, and it is not accepting outside contributions: the agreement and its acceptance mechanism need legal review before public contribution intake opens.

## Contribution rights

Contributors retain copyright in their original contributions. Before RTEPL accepts a contribution, the contributor must accept the separate [Contributor License Agreement](CLA.md). That agreement gives RawThoughts Enterprises Private Limited permanent project-use, distribution, sublicensing, and relicensing rights, including the right to offer the combined project under commercial terms without later renegotiation or approval.

A Developer Certificate of Origin is not a substitute for this agreement. Accepted contributions follow the outbound licence of their owning source home:

- original Engine, adapter, interface, integration, and Package work uses `AGPL-3.0-or-later`;
- canonical contracts, generated native types, and genuinely thin language translation glue use `Apache-2.0`;
- inherited or third-party material retains its applicable licence.

The Apache interoperability boundary must not absorb compiler, admission, diagnostic, graph, runtime, reconciliation, or effect semantics. Such behavior belongs behind `pale-sdk` in the AGPL implementation.

## Engineering standard

A contribution must preserve the repository's existing ownership boundaries and prove behavior at the narrowest responsible layer. In particular:

- keep semantic meaning single-owned and reusable across projections;
- use closed, typed states instead of string or Boolean substitutes;
- preserve deterministic output, bounded inputs, and explicit effect certainty;
- keep tests, fixtures, and benchmarks outside production source modules;
- document invariants, failure semantics, and non-obvious use with native language documentation;
- avoid compatibility layers for formats that have never been released;
- preserve third-party provenance and applicable notices.

Architecture tests are appropriate when the dependency boundary is itself the behavioral invariant. Tests that only assert a particular helper call or source spelling are not sufficient evidence.

For the technical starting point, read [Pale architecture](docs/architecture.md) and [Source layout](docs/source-layout.md). The latter gives the dependency boundaries, contract generator and component-specific build conventions.

## Attribution

Harsh Joshi is the author and originator. RawThoughts Enterprises Private Limited owns the product, distribution, collective codebase, and commercial licensing authority. Contributors are acknowledged for accepted work; a patch does not automatically change the canonical author entry in `CITATION.cff`.

Use only project-owned or properly licensed names, imagery and other material. Do not imply endorsement by another project or rights holder.
