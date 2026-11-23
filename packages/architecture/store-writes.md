+++
description = "Test full record decoding, reference checks and failure between dependent store writes."
uses = [{ scope = "local", resource_id = "verified-input" }]
see_also = [{ scope = "local", resource_id = "recovery/targets" }]

[guide]
+++

# Test strict decoding and interrupted store writes

<a id="check"></a>

Identify store and domain schemas, expected revision, adapter and read/write contract. Test the full round trip through the normal adapter. Account for every relevant record, typed value, reference and explicit absence. Compare accepted membership and content with committed source. Show rejected or quarantined inputs to the permitted owner. A decode failure must not become an absent record or a falsely successful partial graph.

Test malformed values and references to missing or wrong-type targets. Validate a reference within its domain and scope before deriving facts from it. An ID alone proves neither existence, a valid relationship nor satisfaction of a policy condition. Inspect storage constraints and effective settings on the connection actually used. If storage does not enforce a domain relationship, test the adapter's check before invalid state can be accepted.

Inject failure between dependent writes, including removal and replacement, using the real save path and a fault after its first mutation. Through a fresh connection, inspect committed records, references and revision after failure and reopening. Confirm no success record or derived current result was published. A successful control must commit the intended replacement, advance its revision and return the corresponding outcome record. Report adapter behaviour, storage enforcement and owner permission separately. Leave failed inputs and correction records with their owners.

## Before handing off

- [ ] Rehearse strict decoding of a damaged or unsupported record.
- [ ] Interrupt a store write and read back what stayed visible.
- [ ] Keep the last good state available after refusal or interruption.
