+++
description = "Check identity mappings, current permissions, role reassignment and revocation."
see_also = [{ scope = "local", resource_id = "contracts/identities" }]

[guide]
+++

# Check identity and permissions across a binding

<a id="check"></a>

Identify what authenticated, the identity the external system proved and the local actor record used by the request. A role and its occupant, provider account and local principal, or instance and active deployment are different things. A label, shared process, machine or valid signature does not establish a local assignment or permission.

Trace each required mapping to its issuer, source and target namespaces, supporting record, revision, validity period and revocation state. Check permission for the operation, resource, purpose, scope and effect separately. Require the intended binding and current permission before protected reads or actions. Apply the contract's explicit meaning for an absent scope field. Keep permission records, credential references and secret values separate. Only the system responsible for secrets resolves them into a process permitted to receive them.

Test a valid mapping, same-label identities, an external identity without a local mapping and a valid mapping used for the wrong action or scope. Substitute a binding, revision or deployment while keeping a plausible result record. Reassign a role or revoke a binding between selection and use. Confirm past actions retain their original attribution while current permissions do not silently follow a changed actor, credential or host. Check permission to read and permission to act separately.

Inspect the current actor, resolved binding, permission decision and actual effect independently. A denied case must perform no protected read or action and reveal only permitted diagnostics. Identity namespaces, succession, secret resolution and permission rules remain with their owning systems.

## Before handing off

- [ ] Trace the identity and permissions through the configured binding.
- [ ] Exercise a similar-looking identity with a different scope.
- [ ] Check who can act after rebinding or revocation.
