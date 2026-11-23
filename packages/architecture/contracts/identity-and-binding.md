# Keep contract identity and permissions intact

Use these checks when changing names, locations, pinned contract versions or identity bindings. Keep durable cases with the consuming system's tests. In the review reply, identify the versions and actors checked, observed results and permissions or behaviours still unproved.

## Check identity and authority across a binding

Identify what authenticated, the identity the external system proved and the local actor record used by the request. A role and its occupant, provider account and local principal, or instance and active deployment are different things. A label, shared process, machine or valid signature does not establish a local assignment or permission.

Trace each required mapping to its issuer, source and target namespaces, supporting record, revision, validity period and revocation state. Check permission for the operation, resource, purpose, scope and effect separately. Require the intended binding and current permission before protected reads or actions. Apply the contract's explicit meaning for an absent scope field. Keep permission records, credential references and secret values separate. Only the system responsible for secrets resolves them into a process permitted to receive them.

Test a valid mapping, same-label identities, an external identity without a local mapping and a valid mapping used for the wrong action or scope. Substitute a binding, revision or deployment while keeping a plausible result record. Reassign a role or revoke a binding between selection and use. Confirm past actions retain their original attribution while current permissions do not silently follow a changed actor, credential or host. Check permission to read and permission to act separately.

Inspect the current actor, resolved binding, permission decision and actual effect independently. A denied case must perform no protected read or action and reveal only permitted diagnostics. Identity namespaces, succession, secret resolution and permission rules remain with their owning systems.

## Verify identity across generated names and locators

Identify the canonical ID and its owner's namespace separately from labels, filenames, URLs or generated invocation names. Use the producer's declared mapping or returned address. Consumers must resolve it to the intended identity and verify the referenced records, rather than reconstructing an address from display text.

Test in the destination's namespace. Choose distinct inputs that could collide after separator folding, case or Unicode normalisation, escaping or shortening. Test each alone, then together. Require distinct destinations or refusal before a conflicting write. Inspect lookup and stored results to confirm the intended record. After a location changes, test the promised historical reading route and collision handling with existing records. The owners determine compatibility periods, permissions and destination mechanics.

## Check an owner contract pin before integration

Identify the publishing owner, immutable manifest and schema versions and digests. Use the manifest to verify membership, ownership, transitive references and required publication fixtures. A field sketch or locally generated replacement cannot substitute for a publication the owner has not supplied.

For each consuming task, select the schema subset and every transitive definition its validator reads. Identify each dependency's publishing owner and revision. Validate the verified bytes in the real consumer. Keep the publisher's manifest, consumer selection and runtime binding distinguishable.

Test resolution and validation in a clean environment with the service absent. Include a wrong owner, altered bytes and missing or substituted transitive definitions. Report the pinned version and conformance results separately from deployment readiness, authentication, live execution and effects. Those later claims need observations from their responsible systems.

### Prove the selected profile without a deferred branch

For a staged release, identify the published profile, entry points and included and excluded branches. A field optional within this profile differs from a field belonging only to another profile. Excluded behaviour must not enter through a shared envelope, default, optional field or legacy ID. Its permissions, publication and validation remain separate requirements.

Run the selected path with deferred schemas, bindings and services absent. Test a valid selected record, a complete sibling record, a selected record with one sibling-only field and a partial or mixed binding. Keep each negative input valid until the intended rejection point. Recompute fixture signatures or hashes so an integrity failure cannot masquerade as profile rejection. When the producer can emit the selected record, inspect its actual bytes as well as the consumer's decision.

Observe request acceptance, saved state and diagnostic or scoring output separately where provided. A refused branch must contribute no branch-specific state or result evidence. Passing the selected path leaves the deferred branch untested. The owners define record families, exclusions, permitted diagnostics and later publication requirements.

## Separate terminology from compatibility identifiers

Read the owner's current definition before changing explanatory language. Distinguish prose, examples, compatibility IDs, configuration values and saved fields. Map each serialised ID to its producer revision, profile and actual behaviour. A token can survive a changed implementation. Decoding or identical serialisation alone cannot reveal which behaviour created an old record. Preserve its meaning at the recorded version; leave the mapping unresolved if that version is unknown.

Use current explanations while retaining wire IDs required by the compatibility contract. If an ID suggests a stronger mathematical or operational guarantee, require evidence for that guarantee. Give distinct canonical identities to different contracts that once shared a name, and refuse ambiguous reads. Keep historical quotations and terms attached to their original records.

A temporary alias needs an owner-approved mapping to the same contract, specified readers and versions, a canonical write form and a removal condition supported by observed usage. Test the permitted legacy read, canonical write, ambiguous name and retired alias. Include a well-formed record whose familiar token means something different in its producer's profile. Run a distinguishing input through the actual implementations and inspect results alongside parser and writer checks. An alias must not merge different algorithms or promises.

## Worked example

Two generated names normalise to one file path. Verify the underlying IDs, refuse the collision and regenerate from an agreed contract version. A readable label can change without changing the compatibility ID or a binding's permissions.
