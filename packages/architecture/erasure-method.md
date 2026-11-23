# Check deletion and its dependent copies

## Bind the governed closure

Identify the target and source revision, the owner's decision, retention rules, holds and copies each owner controls. Trace every route that can reconstruct or identify the target, return it or use it to influence a result. Include durable history, replicas, journals, indexes, caches, learned features, managed artifacts, exports, outcome records, backups and encryption keys where relevant. Distinguish outside references and confirmations needed from other copy holders. Missing import records, failed fetches and routine eviction do not permit deletion of an owner's original record.

## Prove the right state for each path

Check immediate denial of access, invalidation of derived state, actual payload deletion, restricted residual retention and outside-holder confirmation separately. Tie the rule preventing further use to the relevant source or permission generation. Test stale readers, in-flight exports, queued work, retries and rebuilds, while confirming unrelated records and scopes still work. An invalidated cache may retain bytes under its own rules. Deleting one copy does not prove another is gone.

Inspect surviving metadata for what it can reconstruct or identify. A tombstone may preserve the permitted minimum history without the payload; a digest of predictable input can still disclose it. For cryptographic erasure, account for every usable key copy and plaintext residue, then check that recovery cannot restore either. State physical and provider limits on what was observed. If removing inputs makes replay impossible, say so.

## Rehearse partial failure and recovery

Use a disposable target with old snapshots and derived copies. Confirm that reads are denied while downstream cleanup remains pending. Inspect uncertain outcomes before retrying. Test denial after restart, rebuild and restore from a backup predating deletion. Apply the separately preserved current permissions before exposing restored state.

In the deletion review or reply, identify the target, next permitted action, observations for each copy, remaining holds or expiry conditions and confirmations still needed from other holders. If the checks establish only logical inaccessibility, say exactly that. Each owning system supplies its execution results and decides what they establish for its copy.
