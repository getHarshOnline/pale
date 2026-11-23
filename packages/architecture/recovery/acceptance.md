+++
description = "Before accepting recovered state, moving a recovery marker or retiring a copy."
uses = [{ scope = "local", resource_id = "restore" }]

[guide]
exposure = "internal"
+++

# Check the commit and the next recovery path

Use these checks before accepting recovered state, advancing a recovery marker or retiring a copy. Keep tests and results in the owning recovery system and report remaining decisions in its review.

## Choose the check

- [Verify the exact commit before advancing recovery](../recovery-commit.md#check).
- [Check who may use the recovered state](../restored-permissions.md#check).
- [Prove the replacement route before retiring a recovery copy](../retire-copy.md#check).

## Worked example

After restoration, inspect the committed revision and current owner's permitted uses. Rehearse recovery from the replacement. Retire the earlier copy only when required uses work from the new location and the owner agrees.
