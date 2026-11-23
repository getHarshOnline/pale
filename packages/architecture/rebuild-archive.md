+++
description = "Compare recovered members, archive bytes and distributed containers as separate identities."
uses = [{ scope = "local", resource_id = "historical-records" }]
see_also = [{ scope = "local", resource_id = "recovery/containers" }]

[guide]
+++

# Verify a rebuilt archive container

<a id="check"></a>

State what must be recovered before substituting a container. A member, member inventory, uncompressed archive and distributed compressed container each have their own identity. Exact-container recovery requires matching the complete output digest. Check external filesystem metadata separately when an owner or reader needs it.

Identify original length and digest, saved template or packing inputs, and the immutable source and stored identity of every replacement member. Match paths using the archive's rules and refuse ambiguity. Preserve spelling, member order, headers, padding, empty directories and metadata companion files where they affect bytes. Preserve separately versioned members that lie outside the larger snapshot. Equal length or similar paths cannot establish member identity.

Use the format, compressor and framing required by the reconstruction recipe. Run with the original container unavailable, and verify exact bytes before publishing a new output path. Separately inspect required modes, timestamps and extended attributes. Keep original metadata observations distinct from the reconstructed file and any later move.

Test valid recovery, a different revision of equal length, duplicate or missing members and packaging changes that leave extracted files identical. Confirm refusal publishes no output. Report member recovery separately from complete-container equivalence. Storage responsibility, backup independence and retirement remain separate owner decisions.

## Before handing off

- [ ] Identify the original archive and the data needed to rebuild it.
- [ ] Compare member order, metadata and bytes as required by the format.
- [ ] Check the rebuilt archive itself before using it for recovery.
