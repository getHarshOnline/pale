+++
description = "Check that generated names and addresses resolve to the intended records without collisions."
see_also = [{ scope = "local", resource_id = "contracts/identities" }]

[guide]
+++

# Verify identity across generated names and locators

<a id="check"></a>

Identify the canonical ID and its owner's namespace separately from labels, filenames, URLs or generated invocation names. Use the producer's declared mapping or returned address. Consumers must resolve it to the intended identity and verify the referenced records, rather than reconstructing an address from display text.

Test in the destination's namespace. Choose distinct inputs that could collide after separator folding, case or Unicode normalisation, escaping or shortening. Test each alone, then together. Require distinct destinations or refusal before a conflicting write. Inspect lookup and stored results to confirm the intended record. After a location changes, test the promised historical reading route and collision handling with existing records. The owners determine compatibility periods, permissions and destination mechanics.

## Before handing off

- [ ] Resolve the original and generated names to their records.
- [ ] Test reused names and two inputs that generate the same locator.
- [ ] Check the resolved identity at the consumer.
