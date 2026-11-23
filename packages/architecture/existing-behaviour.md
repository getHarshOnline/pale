+++
description = "Test inherited behaviour and new capabilities separately so neither hides the other’s failures."
see_also = [{ scope = "local", resource_id = "contracts/profiles" }]

[guide]
+++

# Test existing behaviour and the added capability separately

<a id="check"></a>

Identify the accepted upstream candidate, inherited contract and suite, adapted implementation, consumer and versioned compatibility mappings. Run the inherited suite against that adaptation. If translating a test, account for every changed assertion and observation point. Use a known-bad case to show the mapping still detects violation of the original requirement. An omitted test remains a gap.

For the added capability, specify new behaviour, a control lacking it, what would disprove the claim and conditions under which it should hold. Declare cases, thresholds, repetitions and uncertainty before running where relevant. Keep suite identities, inputs, environment, commands and raw observations in the existing test results. The inherited suite proves only what it exercises; a new suite cannot compensate for inherited regressions.

Report each suite separately, including unchanged or unsupported claims, tied to the tested versions. Reject evidence from another candidate or an aggregate pass hiding failed or missing requirements. The owner's acceptance, attestation and release decisions remain separate from test results.

## Before handing off

- [ ] Name the checks the new implementation inherits.
- [ ] Exercise the existing behavior and the added capability separately.
- [ ] Check that the new path still enforces the old promises.
