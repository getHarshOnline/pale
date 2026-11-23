+++
description = "Before an owner deletes a copy, make sure everything in it can still be recovered: the live version, its history and a copy that does not depend on this machine."
uses = [{ scope = "local", resource_id = "historical-records" }]
see_also = [{ scope = "local", resource_id = "recovery/acceptance" }, { scope = "local", resource_id = "retire-route" }]

[guide]
+++

# Prove the replacement route before retiring a recovery copy

<a id="check"></a>

Identify the copy and every required use. Trace current consumers, recovery procedures and historical references separately through configuration, scripts, resolved links and retention rules. In the retirement review, list each dependency, the state it requires and its replacement route or unresolved need. Owners can repoint current instructions. Sealed records keep original references, with a permitted recovery route readers can follow from them.

Restore from the replacement into a clean isolated target while the candidate for retirement is unavailable. Compare paths, bytes, metadata, stored state and dependencies according to each use's promise. A clean checkout may restore history while omitting staged or working edits. A source archive may restore files while omitting a live database or journal. State what each component restores and exercise dependent readers or commands where behaviour matters. Missing coverage or an unresolved active dependency requires retaining the copy.

Check custody and independent-backup obligations separately from equivalent content. A second copy in the same failure domain may satisfy a read while failing backup requirements. Rehearse retirement and reverse restoration using a controlled copy. Check live routes, historical reads, paths, metadata and intervening edits. The custodian decides whether to retire the original and how that decision remains recoverable.

## Before handing off

- [ ] Restore through the proposed replacement without relying on the old copy.
- [ ] Check the required records, references and recovery permissions.
- [ ] Confirm the owner approves retiring the old route.
