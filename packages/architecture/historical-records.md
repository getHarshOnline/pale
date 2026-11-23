+++
description = "Recover the original historical bytes and check rights to keep and use them separately."
uses = [{ scope = "local", resource_id = "verified-input" }]
see_also = [{ scope = "local", resource_id = "rebuild-archive" }, { scope = "local", resource_id = "retire-copy" }, { scope = "local", resource_id = "historical-locations" }, { scope = "local", resource_id = "historical-links" }, { scope = "local", resource_id = "restored-metadata" }, { scope = "local", resource_id = "ordered-records" }, { scope = "local", resource_id = "shared-storage" }, { scope = "local", resource_id = "recovery/history" }]

[guide]
+++

# Check recovered historical records

<a id="check"></a>

If an outcome record depends on exact bytes, associate it with an immutable revision and digest under the owner's retention rules. A mutable view may point to the latest version. Test that the older record still resolves after the view changes, including a move of the storage location.

For each required artifact, read preserved bytes or rebuild from the exact saved inputs, configuration and tool revision. Compare the result with the original digest. Identify whether recovery used retained bytes or a verified rebuild, or produced different bytes or no result. If exact reproduction is impossible, retain the original while policy permits and a claim depends on it. Leave the claim unproved when required bytes cannot be recovered. A digest match establishes content identity; possession history, rights and permission to use restored state need separate checks.

## Before handing off

- [ ] Name the original record, its owner and what the recovery needs to show.
- [ ] Compare the recovered bytes and references with the original record.
- [ ] Keep questions open when a recovery claim cannot be checked.
