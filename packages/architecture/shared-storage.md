+++
description = "Share identical bytes while preserving each record’s access, metadata and retention rules."
uses = [{ scope = "local", resource_id = "historical-records" }]
see_also = [{ scope = "local", resource_id = "recovery/containers" }]

[guide]
+++

# Preserve each record when storage is shared

<a id="check"></a>

Equal bytes may share storage without becoming the same record. For every occurrence preserve its record identity, original locator, revision, digest, length, custody, retention or hold, access scope and captured metadata. Include file kind, mode and timestamp precision when needed. Distinguish metadata restoration from metadata merely preserved as an observation. Separate captures with equal bytes remain separate events and records; equality alone does not establish independent observation.

Share a body only where all occurrences are permitted and retain their access, availability and retention rules. Resolve permission through the requested occurrence before opening or revealing a body or alias. A digest proves content identity, not access permission, an independent replica or a retention decision. Keep sealed originals and required copies with their authorised holders.

Reconstruct each occurrence in an isolated target and compare bytes and required metadata with its own record. Test equal bytes at two locators with different metadata, changed bytes under a familiar locator, forbidden lookup and loss of the shared body. Removing one occurrence must deny that route while preserving separately required retention. If holds, erasure or recovery promises cannot be honoured independently, keep the storage paths separate. Only the owner can retire a redundant physical copy after all affected recovery routes pass.

## Before handing off

- [ ] List the separate records and owners that use the same storage.
- [ ] Restore the requested record without changing what the other record can read.
- [ ] Check each owner’s access and removal rules separately.
