# Rebuild archive containers and shared storage

Use these checks for archives, ordered streams and records sharing physical storage. Keep recovery cases in the owning tests and report which content, container and metadata claims they establish in the recovery review.

## Rebuild a serialized container in its original order

Identify the original container's record, expected digest and byte count, saved carrier and selected members. Fix member order and the rule resolving each selector, such as a numbered entry in a hash-pinned archive. State whether order was captured originally or recovered later. Preserve original keys and their mappings separately from paths opened by the recovery tool. Refuse missing, duplicate or ambiguous selections; legitimate repeated members in the original sequence still retain their separate positions.

Fix the serializer and byte-affecting options: encoding, escaping, indentation, separators, newline conversion and final newline. Identical members or parsed values can produce different bytes. Replay original order rather than current filesystem traversal. Containers with different membership or serialisation need distinct records.

Rehearse in isolation and compare digest and byte count with the original. If only a checksum is needed and reading is permitted, stream output into the digest calculation. Keep any required inspection copy where its owner permits. Test changed order, serializer options and missing members. A semantic match with a different digest leaves exact recovery unproved. The owner decides retention, use and disclosure.

## Verify reconstruction of an exact archive container

State what must be recovered before substituting a container. A member, member inventory, uncompressed archive and distributed compressed container each have their own identity. Exact-container recovery requires matching the complete output digest. Check external filesystem metadata separately when an owner or reader needs it.

Identify original length and digest, saved template or packing inputs, and the immutable source and stored identity of every replacement member. Match paths using the archive's rules and refuse ambiguity. Preserve spelling, member order, headers, padding, empty directories and metadata companion files where they affect bytes. Preserve separately versioned members that lie outside the larger snapshot. Equal length or similar paths cannot establish member identity.

Use the format, compressor and framing required by the reconstruction recipe. Run with the original container unavailable, and verify exact bytes before publishing a new output path. Separately inspect required modes, timestamps and extended attributes. Keep original metadata observations distinct from the reconstructed file and any later move.

Test valid recovery, a different revision of equal length, duplicate or missing members and packaging changes that leave extracted files identical. Confirm refusal publishes no output. Report member recovery separately from complete-container equivalence. Storage responsibility, backup independence and retirement remain separate owner decisions.

## Preserve each record when storage is shared

Equal bytes may share storage without becoming the same record. For every occurrence preserve its record identity, original locator, revision, digest, length, custody, retention or hold, access scope and captured metadata. Include file kind, mode and timestamp precision when needed. Distinguish metadata restoration from metadata merely preserved as an observation. Separate captures with equal bytes remain separate events and records; equality alone does not establish independent observation.

Share a body only where all occurrences are permitted and retain their access, availability and retention rules. Resolve permission through the requested occurrence before opening or revealing a body or alias. A digest proves content identity, not access permission, an independent replica or a retention decision. Keep sealed originals and required copies with their authorised holders.

Reconstruct each occurrence in an isolated target and compare bytes and required metadata with its own record. Test equal bytes at two locators with different metadata, changed bytes under a familiar locator, forbidden lookup and loss of the shared body. Removing one occurrence must deny that route while preserving separately required retention. If holds, erasure or recovery promises cannot be honoured independently, keep the storage paths separate. Only the owner can retire a redundant physical copy after all affected recovery routes pass.

## Worked example

Rebuild an archive with repeated members and significant ordering. Compare the full container bytes and inspect each member in context. When two historical records refer to one stored object, preserve both records and their separate reasons for retention.
