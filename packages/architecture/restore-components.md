+++
description = "Restore components independently, then check whether their combined checkpoint can return to service."
uses = [{ scope = "local", resource_id = "verified-input" }]
see_also = [{ scope = "local", resource_id = "recovery/targets" }]

[guide]
+++

# Test a checkpoint by restoring its components independently

<a id="check"></a>

Identify the parent checkpoint, ordered component list, exact child checkpoints and compatible profiles, plus the parent's required store and replay position. State what recovery promises and which components that promise requires. Shared interfaces or checkpoint formats alone do not establish compatible state, providers or environments.

Obtain actual restore results for each component. Record its verified checkpoint and profile, kind of recovery achieved and required post-restore checks in the existing recovery results. A digest identifies inputs. An equivalence claim needs a metric, tolerances and observed comparison. Starting clean establishes fresh state with its recorded origin, not recovery of old state. Missing or failed children remain unresolved even if others pass. After child checks, test the combined system at the required store and replay position.

Accept only combinations the owner's policy permits. Exact recovery requires every required child and the composition to satisfy that claim. A degraded mode needs separate permission and explicit child gaps. Old reservations, grants and leases must satisfy current permissions or be approved again. Recheck position and authority when admitting restored state to service. Keep a partially restored candidate out of use until the accepted outcome permits it.

Test exact recovery of all components, a permitted degraded mix, an unsupported mix, a missing child or changed checkpoint, a profile change before activation and an incorrect parent replay position. Inspect the accepted aggregate and child results separately. Owners define recovery classes, allowed combinations, checks and permission to activate.

## Before handing off

- [ ] Bind the checkpoint to each component and its stored version.
- [ ] Restore components independently and compare their identities and positions.
- [ ] Check whether the components can serve together after the restore.
