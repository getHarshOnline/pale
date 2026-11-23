# Review a snapshot import

Identify the producer, receiving store, contract, ownership scope and source generation being imported. Use the snapshot method to check coverage, permission to remove records and the state readers can actually see in the receiving store.

In the review reply, distinguish cases the responsible systems exercised from requirements that have yet to be tested. Report replacements, unresolved removals and recovery gaps. Replacing a derived cache and changing an owner's original record are different actions; acceptance of one does not approve the other.

## Before handing off

- [ ] Check source coverage and the proposed replacements or removals.
- [ ] Inspect the imported state separately from activation.
- [ ] Report the untested parts of recovery or replacement.
