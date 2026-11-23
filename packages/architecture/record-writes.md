+++
description = "Check whether a complete read/write path preserves fields, values and seals."
see_also = [{ scope = "local", resource_id = "contracts/translation" }]

[guide]
+++

# Verify what an intermediary preserves when it writes

<a id="check"></a>

Identify the versions each route reads and writes. Exercise the complete read/write entry point, including nested decoders. Compare emitted field presence, numbers and versioned meanings with the accepted input and intended edit. Successful decoding can be followed by a write that drops or alters a field. Testing an isolated field does not establish the enclosing decoder's behaviour.

Use saved old and new records with a well-formed new field inside a nested variant. Distinguish refusal, read-only acceptance, lossless writing and lossy writing. For sealed records, verify the seal on the bytes actually emitted. For unsealed authored records, compare intended values and run the producer or compiler to check resulting behaviour. Shape validation alone is insufficient.

A relay permitted to forward an opaque record can preserve its bytes for validation by the consumer's owner. Test byte identity along that route. A typed edit needs compatible readers and writers or an explicit reviewed translation. Any replacement seal must cover that permitted output. Report transport, structural validity, compatibility and permission to write separately.

## Before handing off

- [ ] Run the complete reader and writer with a nested new field.
- [ ] Read back field values and version in what the writer emitted.
- [ ] Verify the seal on those bytes when the record is sealed.
