+++
description = "Rebuild a container from its original ordered members and serializer settings."
uses = [{ scope = "local", resource_id = "historical-records" }]
see_also = [{ scope = "local", resource_id = "recovery/containers" }]

[guide]
+++

# Rebuild a serialized container in its original order

<a id="check"></a>

Identify the original container's record, expected digest and byte count, saved carrier and selected members. Fix member order and the rule resolving each selector, such as a numbered entry in a hash-pinned archive. State whether order was captured originally or recovered later. Preserve original keys and their mappings separately from paths opened by the recovery tool. Refuse missing, duplicate or ambiguous selections; legitimate repeated members in the original sequence still retain their separate positions.

Fix the serializer and byte-affecting options: encoding, escaping, indentation, separators, newline conversion and final newline. Identical members or parsed values can produce different bytes. Replay original order rather than current filesystem traversal. Containers with different membership or serialisation need distinct records.

Rehearse in isolation and compare digest and byte count with the original. If only a checksum is needed and reading is permitted, stream output into the digest calculation. Keep any required inspection copy where its owner permits. Test changed order, serializer options and missing members. A semantic match with a different digest leaves exact recovery unproved. The owner decides retention, use and disclosure.

## Before handing off

- [ ] Read the original member sequence and serialized form.
- [ ] Rebuild and verify in that order, including duplicates.
- [ ] Check the actual bytes when the reader relies on byte identity.
