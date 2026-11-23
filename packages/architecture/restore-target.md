+++
description = "Check backup reservations, link targets and the actual destination before and after restoration."
uses = [{ scope = "local", resource_id = "verified-input" }]
see_also = [{ scope = "local", resource_id = "recovery/targets" }]

[guide]
+++

# Check the backup and its restore target

<a id="check"></a>

Identify the backup's owner, source, revision, recovery attempt and expected content. Give independent attempts different storage slots, or protect a shared slot with an exclusive claim enforced by its store. Reconcile held or abandoned claims before reuse. A later successful copy at the same path does not prove an earlier backup survives.

Specify whether restoring changes a link entry, the object it points to or a regular file. Inspect the written path, stored link and resolved target separately. Establish the target's owner and permitted root before opening it. Matching content at a path does not prove the intended object was restored. Refuse unexpected links or aliases. If linked restoration is permitted, keep the actual target fixed through the write and preserve the link unless replacement is separately authorised.

Before writing, check the claimed slot, content digest, destination's expected state and isolation from other work. Recheck after handoff or restart. After writing, inspect destination bytes, link entry and the previous state. Remove a backup only after verified recovery and under its retention or disposal rules. A failed check leaves required backups and unresolved records intact.

Test a regular restore, linked path, overlapping captures, changed destination and failure before final inspection. Verify which backup each attempt consumed, where it wrote, which other targets stayed untouched and which recovery records remain.

## Before handing off

- [ ] Check the stored backup identity and the restore target separately.
- [ ] Restore into an isolated target and read back the result there.
- [ ] Refuse a different backup or target even when its label matches.
