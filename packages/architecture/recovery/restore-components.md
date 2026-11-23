# Restore the intended backup into the intended target

Use these checks to identify backups, restore destinations and independently recovered components. Keep repeatable cases in the recovery tests. Report observed state and unresolved activation decisions in the recovery review.

## Bind the backup slot and the actual restore target

Identify the backup's owner, source, revision, recovery attempt and expected content. Give independent attempts different storage slots, or protect a shared slot with an exclusive claim enforced by its store. Reconcile held or abandoned claims before reuse. A later successful copy at the same path does not prove an earlier backup survives.

Specify whether restoring changes a link entry, the object it points to or a regular file. Inspect the written path, stored link and resolved target separately. Establish the target's owner and permitted root before opening it. Matching content at a path does not prove the intended object was restored. Refuse unexpected links or aliases. If linked restoration is permitted, keep the actual target fixed through the write and preserve the link unless replacement is separately authorised.

Before writing, check the claimed slot, content digest, destination's expected state and isolation from other work. Recheck after handoff or restart. After writing, inspect destination bytes, link entry and the previous state. Remove a backup only after verified recovery and under its retention or disposal rules. A failed check leaves required backups and unresolved records intact.

Test a regular restore, linked path, overlapping captures, changed destination and failure before final inspection. Verify which backup each attempt consumed, where it wrote, which other targets stayed untouched and which recovery records remain.

## Check a checkpoint with independently restored components

Identify the parent checkpoint, ordered component list, exact child checkpoints and compatible profiles, plus the parent's required store and replay position. State what recovery promises and which components that promise requires. Shared interfaces or checkpoint formats alone do not establish compatible state, providers or environments.

Obtain actual restore results for each component. Record its verified checkpoint and profile, kind of recovery achieved and required post-restore checks in the existing recovery results. A digest identifies inputs. An equivalence claim needs a metric, tolerances and observed comparison. Starting clean establishes fresh state with its recorded origin, not recovery of old state. Missing or failed children remain unresolved even if others pass. After child checks, test the combined system at the required store and replay position.

Accept only combinations the owner's policy permits. Exact recovery requires every required child and the composition to satisfy that claim. A degraded mode needs separate permission and explicit child gaps. Old reservations, grants and leases must satisfy current permissions or be approved again. Recheck position and authority when admitting restored state to service. Keep a partially restored candidate out of use until the accepted outcome permits it.

Test exact recovery of all components, a permitted degraded mix, an unsupported mix, a missing child or changed checkpoint, a profile change before activation and an incorrect parent replay position. Inspect the accepted aggregate and child results separately. Owners define recovery classes, allowed combinations, checks and permission to activate.

## Check strict decoding and failed store writes

Identify store and domain schemas, expected revision, adapter and read/write contract. Test the full round trip through the normal adapter. Account for every relevant record, typed value, reference and explicit absence. Compare accepted membership and content with committed source. Show rejected or quarantined inputs to the permitted owner. A decode failure must not become an absent record or a falsely successful partial graph.

Test malformed values and references to missing or wrong-type targets. Validate a reference within its domain and scope before deriving facts from it. An ID alone proves neither existence, a valid relationship nor satisfaction of a policy condition. Inspect storage constraints and effective settings on the connection actually used. If storage does not enforce a domain relationship, test the adapter's check before invalid state can be accepted.

Inject failure between dependent writes, including removal and replacement, using the real save path and a fault after its first mutation. Through a fresh connection, inspect committed records, references and revision after failure and reopening. Confirm no success record or derived current result was published. A successful control must commit the intended replacement, advance its revision and return the corresponding outcome record. Report adapter behaviour, storage enforcement and owner permission separately. Leave failed inputs and correction records with their owners.

## Worked example

Restore database and journal into a fresh target. Verify each input and restored component before combining them. Fail one store write and check that another component's success does not mark the entire checkpoint usable. Inspect data and replay position independently.
