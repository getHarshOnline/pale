# Keep meaning and effects through translation

Use these checks when consumers interpret a contract, intermediaries rewrite records or an export selects part of a source. Keep repeatable checks in the owning tests and report results and unresolved mappings in the review.

## Verify the meaning preserved across an edge

State the claim that must remain true when the consumer reads the producer's output. Trace the source definition and accepted profile through the produced artifact, consumer field or symbol and actual interpretation. Include identity, units, domain limits, order, assumptions and failures where they change the claim.

For each consumer, say whether it uses the meaning directly, translates it under an explicit mapping or has no supported route. A translation needs its own version, limits and tests. Mechanical compatibility does not prove the translated claim. Conversely, a connection promising only mechanical transport needs only the checks it promises.

Test a valid case and one that still compiles or decodes but violates the intended mapping. Observe the consumer's result or refusal. For a formal claim, distinguish statement, assumptions, proof or counterexample, and correspondence with implementation. After correcting a profile, revisit dependent mappings while preserving earlier artifacts and conclusions. Carry earlier evidence forward only where the affected interpretation is shown to agree.

Report mechanical and semantic results at the same pinned versions. An unresolved mapping leaves the affected claim open for its owner.

### Check a change in the selected semantic profile

Preserve the historical profile, source, contracts, evaluator and result schema behind each judgment. Identify the proposed profile and dependencies the actual consumer resolves separately. List downstream uses by claim, including type-only, wire, numerical and formal uses. A consumer can use types from a pinned historical profile without adopting its successor or that successor's claims.

Recheck interpretations affected by changed assumptions, domains, units, error rules or relationships. Classify each result as portable under a proved mapping, needing recheck, not portable, incomparable or unresolved. Keep the old judgment and profile readable. A compatible build, unchanged descriptor or equal result on an ordinary input cannot upgrade a historical judgment.

For a proposed translation, state direction, input class, preserved obligations, excluded cases and version. Exercise old and proposed profiles with the same consumer and an input that distinguishes them at a limit or exception. Include unchanged consumer code resolving a different source and a mapping that compiles while changing meaning. Report resolved dependencies, observed behaviour and required formal or domain evidence separately.

Recommend a pinned version, verified adapter, deliberate migration, historical use or hold for each consumer. Test a mismatched profile and inspect affected readers. Owners decide cutover order, acceptance and which historical results may inform new decisions.

## Verify what an intermediary preserves when it writes

Identify the versions each route reads and writes. Exercise the complete read/write entry point, including nested decoders. Compare emitted field presence, numbers and versioned meanings with the accepted input and intended edit. Successful decoding can be followed by a write that drops or alters a field. Testing an isolated field does not establish the enclosing decoder's behaviour.

Use saved old and new records with a well-formed new field inside a nested variant. Distinguish refusal, read-only acceptance, lossless writing and lossy writing. For sealed records, verify the seal on the bytes actually emitted. For unsealed authored records, compare intended values and run the producer or compiler to check resulting behaviour. Shape validation alone is insufficient.

A relay permitted to forward an opaque record can preserve its bytes for validation by the consumer's owner. Test byte identity along that route. A typed edit needs compatible readers and writers or an explicit reviewed translation. Any replacement seal must cover that permitted output. Report transport, structural validity, compatibility and permission to write separately.

## Check a bounded export

Identify the owner, caller, purpose, source and fixed revision, requested scope and optional members. Check permission and source membership independently. Build the export from permitted members at that revision. A matching label, current head or store-wide default does not establish membership. Select members before calculating counts, applying limits or pagination, generating summaries or assembling archives. Keep data and metadata within the same permitted scope.

Specify which requests may be narrowed and which must be refused. An explicit empty selection stays empty. Refuse unknown revisions and unsupported required members. Report permitted optional omissions without revealing private names, counts or rejection details. Define ordering, tie-breaks, limits and the scope and revision a cursor identifies. In the owner's export record, keep effective scope, output identity, allowed omissions, retention and invalidation rules.

Test overlapping identifiers across two sources and members outside the caller's permission. Include wrong caller or purpose, unknown source, stale revision or cursor, empty scope, unsupported flag and changes outside selected scope. Check data, counts, errors and attachments against the permitted view. Repeat on the same fixed source to check deterministic order and limits. For staged delivery, test revoked permission or invalidated source between staging and inspection under the owner's rules.

Report source and output identities, effective scope, refusals exercised and gaps. The owner defines memberships, selectors, export records, visibility and delivery. Evidence that an export succeeded grants no additional downstream use.

## Worked example

An intermediary reads a field it does not expose and writes the record back. Compare source and result to see whether the field, precision and update behaviour survive. If an export supports only a subset, name it and verify excluded data stays out.
