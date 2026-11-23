+++
description = "Verify the published schemas and dependencies a consumer pins, including excluded profiles."
see_also = [{ scope = "local", resource_id = "contracts/identities" }]

[guide]
+++

# Check the contract version before integration

<a id="check"></a>

Identify the publishing owner, immutable manifest and schema versions and digests. Use the manifest to verify membership, ownership, transitive references and required publication fixtures. A field sketch or locally generated replacement cannot substitute for a publication the owner has not supplied.

For each consuming task, select the schema subset and every transitive definition its validator reads. Identify each dependency's publishing owner and revision. Validate the verified bytes in the real consumer. Keep the publisher's manifest, consumer selection and runtime binding distinguishable.

Test resolution and validation in a clean environment with the service absent. Include a wrong owner, altered bytes and missing or substituted transitive definitions. Report the pinned version and conformance results separately from deployment readiness, authentication, live execution and effects. Those later claims need observations from their responsible systems.

### Prove the selected profile without a deferred branch

For a staged release, identify the published profile, entry points and included and excluded branches. A field optional within this profile differs from a field belonging only to another profile. Excluded behaviour must not enter through a shared envelope, default, optional field or legacy ID. Its permissions, publication and validation remain separate requirements.

Run the selected path with deferred schemas, bindings and services absent. Test a valid selected record, a complete sibling record, a selected record with one sibling-only field and a partial or mixed binding. Keep each negative input valid until the intended rejection point. Recompute fixture signatures or hashes so an integrity failure cannot masquerade as profile rejection. When the producer can emit the selected record, inspect its actual bytes as well as the consumer's decision.

Observe request acceptance, saved state and diagnostic or scoring output separately where provided. A refused branch must contribute no branch-specific state or result evidence. Passing the selected path leaves the deferred branch untested. The owners define record families, exclusions, permitted diagnostics and later publication requirements.

## Before handing off

- [ ] Confirm which contract version the consumer actually loads.
- [ ] Check the required path with the declared providers present and missing.
- [ ] Keep an unimplemented required branch open.
