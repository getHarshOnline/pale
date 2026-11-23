+++
description = "Find the original version after its old path or URL has moved or been reused."
uses = [{ scope = "local", resource_id = "historical-records" }]
see_also = [{ scope = "local", resource_id = "recovery/history" }]

[guide]
+++

# Resolve historical bytes after a locator is reused

<a id="check"></a>

Preserve the original outcome record and locator. Use an owner-approved recovery mapping from that record, artifact role and expected digest to a saved file, object revision or archive member. Several records may share a locator yet require different bytes. Follow the route as a reader would and verify the original digest. Check every referenced artifact needed by the claim, including nested references in scope.

Test two historical versions at the same former locator, plus a moved or missing saved copy. Supply the current view and the other record's version as substitution controls. A reachable path or plausible title establishes navigation, not historical identity. If the required artifact cannot be found among permitted holdings, report the missing proof and preserve the original record for follow-up.

## Before handing off

- [ ] Resolve the locator from the historical record.
- [ ] Test a locator that now returns a different revision.
- [ ] Verify the original bytes before accepting the recovered record.
