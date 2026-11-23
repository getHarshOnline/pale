+++
description = "Change explanatory names while preserving historical meaning and required wire identifiers."
see_also = [{ scope = "local", resource_id = "contracts/identities" }]

[guide]
+++

# Separate terminology from compatibility identifiers

<a id="check"></a>

Read the owner's current definition before changing explanatory language. Distinguish prose, examples, compatibility IDs, configuration values and saved fields. Map each serialised ID to its producer revision, profile and actual behaviour. A token can survive a changed implementation. Decoding or identical serialisation alone cannot reveal which behaviour created an old record. Preserve its meaning at the recorded version; leave the mapping unresolved if that version is unknown.

Use current explanations while retaining wire IDs required by the compatibility contract. If an ID suggests a stronger mathematical or operational guarantee, require evidence for that guarantee. Give distinct canonical identities to different contracts that once shared a name, and refuse ambiguous reads. Keep historical quotations and terms attached to their original records.

A temporary alias needs an owner-approved mapping to the same contract, specified readers and versions, a canonical write form and a removal condition supported by observed usage. Test the permitted legacy read, canonical write, ambiguous name and retired alias. Include a well-formed record whose familiar token means something different in its producer's profile. Run a distinguishing input through the actual implementations and inspect results alongside parser and writer checks. An alias must not merge different algorithms or promises.

## Before handing off

- [ ] List which names readers see and which identifiers callers send.
- [ ] Check old identifiers at the parser and serialized output.
- [ ] Change aliases and protocol values under their own compatibility rules.
