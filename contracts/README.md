# Pale portable contracts

This directory is the canonical source home for generated, portable Pale
contract artifacts and versioned cross-language conformance data.

The human-reviewable JSON files are deterministic pretty JSON. RFC 8785/JCS is
used only where a contract explicitly defines canonical bytes for identity or
wire interchange; it is not stored as a second compact copy.

The schemas and generated contract definitions in this directory are licensed
under [Apache-2.0](LICENSE). Pale's compiler, semantic admission, diagnostics,
graph, resident, reconciliation, and effect behavior remain in the
AGPL-3.0-or-later implementation and are not reimplemented here.

The current artifacts are local initial-v1 engineering outputs. Their presence
does not establish publication, release, installation, resident admission, or a
native effect.
