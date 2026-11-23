+++
description = "Check which restored metadata was captured originally and which the restore introduced."
uses = [{ scope = "local", resource_id = "historical-records" }]
see_also = [{ scope = "local", resource_id = "recovery/history" }]

[guide]
+++

# Verify where restored metadata came from

<a id="check"></a>

Identify the original occurrence, carrier bytes, restore tool and options, destination filesystem and metadata needed by the claim. Inventory paths, file kinds, content and required metadata separately. For each field, say whether the carrier stores it and whether restoration matches, changes, omits or leaves it unverified. Inspect metadata on the restored filesystem. An archive listing, content digest or successful extraction does not establish filesystem equivalence.

Separate metadata captured at source from values introduced by packaging, transport, extraction or destination. Attribute a cause only with evidence, such as encoded metadata or a controlled comparison varying the restore route. Test identical content with missing or changed metadata, then a route preserving the required field. Never invent origin or security attributes to make a historical comparison pass.

Report the outcome for every required field and dependent use. Matching content may suffice for a content-only reader. If another reader needs metadata the carrier cannot recover, retain the original occurrence under its owner's rules. A verified custody record can preserve an observation when the owner accepts that form of evidence. Keep separately captured transcripts and review state when the carrier holds only source members.

## Before handing off

- [ ] Identify where each recovered metadata field came from.
- [ ] Keep original metadata separate from details the restore generated.
- [ ] Compare against the original record and record any gaps.
