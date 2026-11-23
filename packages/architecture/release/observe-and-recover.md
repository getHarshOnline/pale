# Read back the release and check the recovery path

## Read back the exact release

For each destination, keep the accepted starting state, intended release and target, dispatch attempt and independent inspection in the existing release record. Check platform build or activation status separately from the bytes or behaviour served to users. A matching target reference does not prove it was served. Inspect intended routes, workloads and users, explicitly reporting missing or unavailable members.

Before dispatch, define how to distinguish intended state, verified starting state, drift and uncertainty. A timeout or lost response leaves the effect uncertain. Inspect the original target and preserve that attempt before considering another action. Seeing the old state now does not erase earlier success or an unresolved attempt. Retry, compensation and rollback each need permission for the action.

Rehearse a lost response after a real fixture update, a reachable endpoint serving the old release and a partially inspected target set. Confirm inspection detects each case. Report who can resolve each state, what permits wider exposure and which checks remain unrun.

## Check forward rollback

Use forward rollback when the active generation must change while committed history, revocations and other security state continue forward. Identify the current generation, saved artifact proposed for reuse and current scope, permissions, policy, compatibility and dependency versions. A historical digest identifies the artifact; separately check whether it remains usable and permitted today. Revalidate or refuse it when permissions, dependencies, data or compatibility have changed.

Stage it away from the active path. Tie the recovery decision to the expected predecessor and activate under a new identity and fencing token through the owner's release transactions. Preserve the failed generation and its outcome records. Keep revocations and committed history current. State which in-flight work may finish on existing pins and which current permissions require refusal. Check independently owned consumers through their own activation and compatibility observations.

Test permitted reuse, changed predecessor, an artifact with invalidated permission or dependency and interrupted activation. Inspect active identity, served artifact, history, revocations and drained or refused work separately. After an ambiguous reply, reconcile the attempted successor before retrying. Each claimed effect needs an actual observation from its owning system.

## Check a persisted receipt at its commit boundary

Identify the attempt, intended outputs and destination of its completion record. Inspect stored outputs and digests at the point that record claims. If the owner copied or transformed an output, compare resulting bytes with the reviewed input. Checking input alone does not prove saved output. Confirm every named member is readable and matches before exposing a complete record under its final identity.

Test a concurrent reader during record publication, changed output and a collision at the final identity. A new record must become visible complete. A collision must preserve the competing record and its supporting material. If rollback is promised, inspect the restored state and ensure cleanup affects only the failed attempt. For uncertain commit status, retain the evidence and reconcile that attempt before retrying or removing residue.

Report visibility, any crash-durability claim and independent inspection separately in the release review. Transaction and storage mechanisms, recovery policy and release permission remain with their owners.

## Worked example

A canary fails after committing part of a change. Inspect deployed build and settings, stop expansion and rehearse compatible forward repair. Check the saved outcome after commit and confirm in-flight operations reconcile.
