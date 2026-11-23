# Plan and check a restore

Identify the owner, backup, actual restore target, tool, recovery unit and every input the restore will read. Use a fresh isolated target. Inspect schema, data, checkpoint and journal tail separately. A successful restore establishes recoverability; the owner separately decides whether that state may return to service.

Follow the verified-input method to ensure the restore consumes the bytes that were checked. Select other tests for the actual backup, history, store and cutover. Confirm that a failed input or stage leaves the previous valid state available. Return the rehearsal plan or findings in the task's plan or review reply, stating what was read back and what remains untested. The owner decides activation, responsibility for restored records and retirement of old copies.

## Choose the check

| If you are doing this | Read this |
| --- | --- |
| Check which bytes the restore actually opens. | [Consume the bytes you verified](recovery-verified-input.md) |
| Identify a backup, target and restored components. | [Restore the intended backup into the intended target](recovery/restore-components.md) |
| <a id="verify-recovered-references-from-their-original-base"></a>Find the earlier record and what it actually linked to. | [Recover historical records and their references](recovery/restore-history.md#verify-recovered-references-from-their-original-base) |
| Recover an archive, ordered stream or several records stored together. | [Rebuild archive containers and shared storage](recovery/rebuild-containers.md) |
| Verify a committed frontier, owner permissions or retirement. | [Check the commit and the next recovery path](recovery/advance-or-retire.md) |

## Before handing off

- [ ] Check the bytes consumed and the target that received them.
- [ ] Read back the restored data and the required checks.
- [ ] Name any state or cutover decision the responsible owner still needs to make.
