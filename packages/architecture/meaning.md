+++
description = "Test whether consumers preserve meaning across translations and changed contract profiles."
see_also = [{ scope = "local", resource_id = "contracts/translation" }]

[guide]
+++

# Verify the meaning preserved across an edge

<a id="check"></a>

State the claim that must remain true when the consumer reads the producer's output. Trace the source definition and accepted profile through the produced artifact, consumer field or symbol and actual interpretation. Include identity, units, domain limits, order, assumptions and failures where they change the claim.

For each consumer, say whether it uses the meaning directly, translates it under an explicit mapping or has no supported route. A translation needs its own version, limits and tests. Mechanical compatibility does not prove the translated claim. Conversely, a connection promising only mechanical transport needs only the checks it promises.

Test a valid case and one that still compiles or decodes but violates the intended mapping. Observe the consumer's result or refusal. For a formal claim, distinguish statement, assumptions, proof or counterexample, and correspondence with implementation. After correcting a profile, revisit dependent mappings while preserving earlier artifacts and conclusions. Carry earlier evidence forward only where the affected interpretation is shown to agree.

Report mechanical and semantic results at the same pinned versions. An unresolved mapping leaves the affected claim open for its owner.

### Check a change in the selected semantic profile

Preserve the historical profile, source, contracts, evaluator and result schema behind each judgment. Identify the proposed profile and dependencies the actual consumer resolves separately. List downstream uses by claim, including type-only, wire, numerical and formal uses. A consumer can use types from a pinned historical profile without adopting its successor or that successor's claims.

Recheck interpretations affected by changed assumptions, domains, units, error rules or relationships. Classify each result as portable under a proved mapping, needing recheck, not portable, incomparable or unresolved. Keep the old judgment and profile readable. A compatible build, unchanged descriptor or equal result on an ordinary input cannot upgrade a historical judgment.

For a proposed translation, state direction, input class, preserved obligations, excluded cases and version. Exercise old and proposed profiles with the same consumer and an input that distinguishes them at a limit or exception. Include unchanged consumer code resolving a different source and a mapping that compiles while changing meaning. Report resolved dependencies, observed behaviour and required formal or domain evidence separately.

Recommend a pinned version, verified adapter, deliberate migration, historical use or hold for each consumer. Test a mismatched profile and inspect affected readers. Owners decide cutover order, acceptance and which historical results may inform new decisions.

## Before handing off

- [ ] Name the producer’s claim and the consumer that uses it.
- [ ] Run a valid input and a decodable input with the wrong meaning.
- [ ] Keep the affected claim open if the mapping is unproved.
