# Consume the exact bytes you verified

Obtain the expected digest or immutable revision from the source authorised to define it. Read a snapshot within the allowed size and verify the bytes the consumer will use. Parse, inspect or copy that same snapshot. If it cannot fit in memory, use an immutable staging object or fixed version the owner permits, and ensure every consumed byte belongs to the verified object. Opening a mutable path, URL or archive alias again is a new read. Checking a path or its metadata does not establish which bytes will be consumed.

Verify referenced members independently when a record points to further evidence. Distinguish reading the original historical version from proving that it is current. When current state matters, check the owning system's committed position or version. A matching digest proves content identity; schema, meaning, scope and permission still need separate checks.

Test a permitted payload and a different well-formed payload. Replace the locator's target after verification but before consumption. The reader may use the verified historical snapshot or refuse the read; it must not show or act on replacement bytes under the earlier digest. Also test a fresh read with the old digest after replacement, plus missing and oversized payloads. Keep these cases in the consuming reader's tests and report which bytes it actually used.

## Before handing off

- [ ] Consume the verified bytes through the checked handle or snapshot.
- [ ] Recheck nested references from their real base.
- [ ] Stop the restore if the current input no longer matches.
