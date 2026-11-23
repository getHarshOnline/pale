+++
description = "Check that a restore really works: from a known backup, to a known checkpoint, on the real system."
uses = [{ scope = "local", resource_id = "verified-input" }]
see_also = [{ scope = "local", resource_id = "recovery/targets" }, { scope = "local", resource_id = "recovery/history" }, { scope = "local", resource_id = "recovery/containers" }, { scope = "local", resource_id = "recovery/acceptance" }]

[guide]

[task]
outputs = ["In the recovery plan: backup and restored-state checks, responsible operators and permitted scope.", "Restart cases, refusal conditions and remaining activation requirements."]
+++

# Plan and check a restore

Identify the owner, backup, actual restore target, tool, recovery unit and every input the restore will read. Use a fresh isolated target. Inspect schema, data, checkpoint and journal tail separately. A successful restore establishes recoverability; the owner separately decides whether that state may return to service.

Follow the verified-input method to ensure the restore consumes the bytes that were checked. Select other tests for the actual backup, history, store and cutover. Confirm that a failed input or stage leaves the previous valid state available. Return the rehearsal plan or findings in the task's plan or review reply, stating what was read back and what remains untested. The owner decides activation, responsibility for restored records and retirement of old copies.

## Choose the check

| If you are doing this | Read this |
| --- | --- |
| Check which bytes the restore actually opens. | [Consume the bytes you verified](verified-input.md) |
| Identify a backup, target and restored components. | [Restore the intended backup into the intended target](recovery/targets.md) |
| <a id="verify-recovered-references-from-their-original-base"></a>Find the earlier record and what it actually linked to. | [Recover historical records and their references](historical-links.md#check) |
| Recover an archive, ordered stream or several records stored together. | [Rebuild archive containers and shared storage](recovery/containers.md) |
| Verify a committed frontier, owner permissions or retirement. | [Check the commit and the next recovery path](recovery/acceptance.md) |

For a rehearsal plan, describe the interruption or restore scenario and the readiness decision it should inform. Name who will run the checks and who can accept the recovered state for use. Leave the plan open if nobody has that authority. Planning a check does not establish that it ran or passed.

## Before handing off

- [ ] Verify the inputs and use a fresh restore target.
- [ ] Read back the required components before judging the restore.
- [ ] Name the remaining activation or retirement decision.
