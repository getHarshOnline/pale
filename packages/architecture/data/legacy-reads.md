# Separate legacy reads from successor writes

## Keep legacy reads separate from successor writes

For a read adapter serving old records, identify the permitted sources, precedence, lookup identity and source information returned. Check that reading does not backfill, mutate or merge originals. An available neighbouring collection is not an authorised fallback. Keep a found record with empty content distinct from not found, invalid input, unavailable source and failed interpretation.

Preserve stored identifiers separately from display labels and submitted queries. An alias can lose information or refer to several records. Apply the owner's explicit alias, leading-zero and numeric-normalisation rules. Numeric conversion must not invent digits or merge opaque identities. Resolve ambiguity, or return it explicitly, before claiming a unique match. Return the canonical source identity and where the record came from.

A continuation creating a new assessment has its own identity and an explicit link to the earlier record; it does not revise the old assessment retroactively. If the new writer must support positional readers, map using actual headers or schema and test additions without shifting existing fields. Store identifiers without loss under the agreed type. Inspect the original, new record and returned relationship separately. Matching IDs alone establish neither parentage nor access permission.

Test duplicate aliases, an exact stored ID, a lossy query value, an unavailable permitted source and an unrelated fallback collection. Verify read-only behaviour, precedence and empty-content handling. Rehearse the new write against the old shape and confirm field positions and originals retain their meaning. Keep cases in the adapter's tests and report the inspected records and gaps in the review. A controlled fixture tests its logic; it does not prove live access or a deployed migration.

## Worked example

Keep an old consumer able to read its frozen input while the new writer produces the new schema. Validate both with their versioned fixtures. Read back the new state after the switch, then verify that retiring the old write route does not remove the read path still in use.
