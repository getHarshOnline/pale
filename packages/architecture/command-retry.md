# Check commands that may be retried

Use this method when a command can change state. Define how its request key identifies the owner, permitted caller scope, operation and version. Include the target, resolved route parameters, normalised input and preconditions in the canonical request. State which fields select the stored request and which must match it. A transport attempt has its own identity, separate from the logical command.

Test reusing a key with changed input and with a different target. Neither case may return an outcome belonging to the wrong request or target. Put the cases in the owner's command tests and report what ran, what was observed and what remains unverified in the review.

## Resolve original replays and semantic duplicates before new writes

Check the current caller's permission to run the command or read its saved outcome before revealing that outcome. Then compare the original request identity. An identical completed request returns its original result even if that command, or a later one, advanced the state revision. Changed input under a reserved key conflicts. A pending or uncertain request is reconciled under its original identity and remains subject to its original visibility rules.

Some systems also recognise a new request that asks for an effect already committed. Define this effect identity separately from the request key: owner, target, scope, effect, versioned input and any precondition that changes its meaning. Compare the canonical inputs within that identity. If the contract permits deduplication, an identical committed effect can produce a distinct duplicate result for the new request, referring to the original outcome. Preserve the original effect and record. A pending match follows the original recovery and visibility rules; a conflicting use of a reserved effect identity is refused. Systems without this facility process a fresh request under their ordinary write rules.

Check fresh-write preconditions only when a new effect is needed. The expected revision and all required read predicates must still hold when it commits. Distinguish the business change, its saved outcome and separately permitted changes to a journal, receipt or writer token. A replay may leave the business state and outcome untouched while the storage system advances a writer reservation. Check each claim against that system's contract.

Test an original replay after a lost response and a later revision, changed and cross-target key reuse, current access to the saved result, concurrent attempts, stale preconditions for a new write and an old holder returning after takeover. Where effect deduplication exists, also test a fresh duplicate after a later revision, a pending original and an effect-key collision. Independently inspect original and duplicate outcome records, their references, the number of effects, business revisions and writer-token changes. Keep rejection, rollback, durable commit and unknown outcome distinct. Isolated fixtures test the protocol they exercise; the actual store and external destinations still need their own checks.

## Protect membership, predicates and absence at the write boundary

List every fact that makes a fresh command eligible: row revisions, relationships, membership, policy or permission versions, query predicates and records that must be absent. Identify who owns each fact, its partition and the state covered by the check. A new blocker or revoked grant can invalidate a command without changing its target row. Locking existing rows does not necessarily protect an absent row or a range of relationships.

Choose a guard that meets the owner's freshness and consistency requirements. Protect the checks and mutation together using the store's transaction and locking protocol, a conditional write or a shared dependency generation. A generation counter works only if every relevant insert, delete, retargeting, policy change and administrative writer participates. Reading a predicate before a scalar compare-and-swap, or checking a token outside the protected write, can leave a race. A transaction annotation, snapshot, timestamp filter or isolation-level name alone proves none of these guarantees.

Test a newly inserted blocker after the eligibility read, a changed or removed dependency and revoked permission where applicable. Include a writer that bypasses protection as a negative control, an unchanged valid case and the same subject label in another owner's partition. When the competing change wins under the contract, confirm that the stale command cannot write its business effect, history or outcome record. Inspect the competing change and original command result separately. The owner must cover every write route and choose conflict and retry behaviour; a synthetic fixture cannot establish an adapter or store profile it never exercised.

## Check each effect in a multi-step command

List effects in order, including uploads, reservations or disclosures before the final action. Identify each effect's owner, destination, permitted input and attempt identity. Resolve the final target and every prerequisite governing disclosure before the first irreversible or externally visible step. That preliminary step needs its own permission and completion evidence. An intent record or a later route that might succeed supplies neither.

Recheck mutable permissions, target identity and required state before each dependent step. Where supported, make the check and effect inseparable through a conditional write or reservation. If a later check fails, preserve earlier confirmed results and stop the dependent work. Report the first effect, later action and any cleanup separately. A lost acknowledgement remains uncertain until permitted inspection reconciles that same attempt. Resume only the remaining eligible step. Cleanup needs its own permission, retention check and result record.

Test a missing, substituted or revoked prerequisite before the first effect, and inspect the destination to confirm no effect occurred. Then let the first step succeed and deny or interrupt the next. Check the partial result and that replay does not repeat the first effect. Include lost acknowledgement, forbidden inspection or cleanup, and a fully permitted path. Inspect the actual destinations and outcome records for each step. The system owner determines ordering, guards, visibility and reconciliation.

## Check the fence for each lease attempt

Distinguish the authenticated holder, displayed or normalised assignee name, target, logical operation and lease attempt. Issue a new fencing token or generation on acquisition or takeover, including reacquisition by the same holder. Renewal, release and protected writes must carry the token captured by that attempt. Looking up the current token for an old request does not give that request permission to act.

Check target, holder, token and applicable expiry in the same protected mutation so takeover cannot race between checking and writing. Expiry may start reconciliation. Transfer the right to act only when the old holder can no longer commit at the destination, or the destination deduplicates that logical effect. A stale local marker or missing reply leaves the earlier effect unresolved. Returning an old outcome need not operate on the current lease.

Race two initial claimants. After expiry and takeover, replay the old attempt's renewal, release and write against successors held by a different holder, the same holder and an equivalent spelling of that holder. Confirm the old token cannot alter the successor, business effect or capacity accounting. Test a valid release or commit with the current token and inspect the lease, outcome and accounting independently. If an external destination cannot block or deduplicate a late effect, keep the work pending or quarantined until its owner reconciles it.

## Recover a derived view after the command commits

Read the command's durable result from the owning store. Check the derived view's availability and published revision separately. A missing, stale or failed view does not undo a confirmed command. Rebuild it from the verified original record or a snapshot with defined coverage; never repeat the business effect to repair the view. Reconcile an unknown command outcome under its original identity first. If its source is corrupt or unreadable, stop and use the owner's store-recovery procedure.

Identify each rebuild by source identity, committed revision and transformation version. Validate the output before publishing it. Prevent an older job from replacing a newer view when jobs finish out of order. Until caught up, show the actual revision, or a pending or unavailable result, as the owner's contract requires. Inspect the visible generation before declaring the repair finished.

Test a confirmed commit followed by failed publication, a lost command response, out-of-order rebuild completion and an unreadable source. Inspect command outcomes and effect counts separately from the view and its revision. Run a successful repair twice and confirm it converges without another business effect. The owner sets acceptable lag, visibility, storage and recovery rules.
