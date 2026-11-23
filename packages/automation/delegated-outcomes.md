# Check delegated work across independently owned systems

Identify the originating owner and record, accepted handoff, exact execution attempt and owner of every later decision. Draw the required order as dependencies between records. Specify which receipts are needed before dispatch, which event establishes the child's result, and which later commands require their own permission checks.

Before dispatch, check the selected handoff's prerequisites through their owning systems: identity, scope, revision and expiry. Test missing and substituted acceptance receipts. A variant with fewer inputs needs its own authority and every receipt required for that variant. Verify that a refused assisted launch creates no child effect.

<a id="qualify-a-directional-service-without-merging-owner-decisions"></a>

## Check each side of a service exchange

Identify the provider, consumer, direction, service-contract revision and permissions for the request, disclosed evidence and returned result. Record the originating offer, destination's acceptance of the request, provider completion, provider acceptance, consumer acceptance, actual outcome and settlement where the chosen contract requires them. Identify each record's owner, revision and dependencies. Provider completion establishes its own result; a delivery acknowledgement establishes receipt within its stated scope. Each later decision needs its own owner's evidence and authority.

Test accepted and refused offers, reading both owners before and after each decision. Receiving an authenticated message or recognising its subject must not cause the destination to accept an unsolicited local command. Provider state must reflect only its authorised actions. Substitute the consumer, reverse the direction and use an earlier contract revision; verify neither side gains an unapproved effect. Replay the exact offer or result to test duplicate protection. A different direction, scope or attempt requires separate acceptance.

Where settlement requires both sides, read their results and acceptances independently. Withhold one acceptance and verify that overall completion remains open. If one side may finish independently, preserve its completed result. Check current revisions before retry, dispute, compensation or reopening. Report unresolved delivery, acceptance and settlement separately. The owners define exchange identities, receipt formats and permission to act.

<a id="qualify-a-provisional-record-bound-once-to-a-handoff"></a>

## Open a provisional record and bind one handoff

Before opening a provisional record, identify its issuing system and namespace, execution identity, reservation and the generation guard that excludes superseded attempts. A parent work item, delivery receipt or short run ID cannot identify the issuer by itself. Specify which facts belong on the provisional record and which arrive with the accepted handoff. A lookup must not create provisional state without a separate request to open it.

Commit the opening, request identity and original receipt together. Make competing openings for the same native identity contend for that one record. Before binding a handoff, verify the exact predecessor revision and digest and current permission. Bind the selected handoff and evidence once, committing the binding and receipt together. Later updates preserve these facts and add only permitted evidence.

Test two issuers using the same run ID, competing openings and a binding with the right revision but wrong digest. Reject changed bytes under a reused request identity. Replay the exact opening and binding after the record advances; verify the original outcomes return without creating new state. Offer a final result to a provisional record and to one bound to another handoff. Inspect each current revision, receipt, dependent effect count and accepted final evidence separately. The owner defines record types, signatures, retention and acceptance rules.

## Reconcile the owner attempt with its execution artifacts

Before resuming, replacing or cleaning up an interrupted attempt, read its committed launch, current generation guard and final outcome from the execution owner. Separately inspect its worker, checkout, spool or output and the exact branch, checkpoint or artifact receipt the owner expects. Distinguish an unavailable read from confirmed absence. A process, familiar path, reusable holder name or plausible output cannot establish the current attempt.

| What the owner and artifact reads show | Required check |
| --- | --- |
| Attempt and artifacts match | Reconcile process, checkpoint, pending effects and final state before resuming. Verify the original attempt resumes without repeating committed effects. |
| Attempt exists and an artifact is absent | Restore the same accepted identity and checkpoint through a checked owner route, or keep the attempt pending or isolated. Verify restored inputs before dispatch. |
| Artifact exists and the attempt is absent, stale or mismatched | Preserve the artifact under its owner's control. Verify it supplies neither a new claim nor a final result. Establish its origin, checkpoint and dependent effects before separately authorised cleanup. |
| A required read is unavailable or ambiguous | Hold affected effects and report the missing owner observation. Preserve independent duties and evidence. |

Test both asymmetric losses, a mismatched generation at a reused location, and interruption after launch but before its observation is recorded. Reconcile that launch before starting another. Inspect claim, artifact, checkpoint, owner outcome and effect count separately. Cleanup, replacement and acceptance each need their own owner decision.

<a id="qualify-replacement-against-the-exact-predecessor-attempt"></a>

## Replace an attempt without weakening its successor

Identify the predecessor's owner, attempt and generation guard, expected revision, and the transition that ends its right to start new effects. Read the committed outcome before permitting replacement. Check the resulting revision and any work or artifacts still held. Verify required signatures against the selected owner authority. Elapsed time, an expired reservation or a lost observer cannot establish the owner's decision.

Inspect the actual write that releases or updates the claim. A condition using only a holder name can clear a later claim when the name is reused. Require the exact attempt or generation in that conditional write, including when the host can choose among adapters or transports. Follow the selected route from caller to write: an earlier claim check does not establish that the later release is protected.

Test delayed predecessor actions after takeover by a different holder and by the same holder with a new attempt. After each refusal, inspect the successor's claim, revision and effect count. Withhold, substitute and replay the predecessor outcome. If the owner permits the old attempt to finish previously accepted completion, cancellation or reconciliation work, require its exact earlier permission and record the result under that attempt. Verify that successor authority and control of its artifacts remain intact.

<a id="qualify-the-child-result-and-later-owner-decisions"></a>

## Verify the child result before later decisions

Read the executor's committed result through an authorised owner route. Match its reference and verified digest to the originating publication, accepted handoff and exact attempt. A transport observation can connect these records without proving that the executor committed the result. A missing or unavailable owner read leaves dependent decisions open. Different bytes under the expected reference invalidate the evidence. Retain a valid predecessor result under its own attempt, even when the successor names the same parent revision.

Keep transport observation, episode closure and parent decisions as separately owned records. An optional final-result reference must match the exact attempt and handoff. The parent owner accepts its own command and checks the expected parent revision at commit. If the parent changed, preserve the child evidence and require a fresh decision. A child's final event cannot approve its parent by itself.

Test a valid transport observation with the executor's record withheld, then restore that exact record as the control. Substitute a digest at the same reference, a genuine result from another publication and a predecessor result for a replacement using the same parent revision. Advance the parent between validation and commit. Inspect the parent revision and resulting decision or emitted occurrence after each case. A failed evidence or revision check must cause no parent transition.

Where owners may update their final states independently, test both arrival orders. Read each owner after the first commit and after both, checking that each transition changes only its own record. Delay one update without inventing success or adding a prerequisite. If a separately authorised expiry may race with it, test expiry before the final event. Preserve the owner's final state and assess any permitted late evidence under that policy.

After a lost reply, repeat the exact result or command and verify its original outcome and effect count. Changed bytes under the same identity are a conflict.

Return the dependency order, exact owner records, intermediate and final observations, and remaining uncertainty. The owning systems define receipt formats, acceptance decisions, state transitions and execution.

## Before handing off

- [ ] Check who can dispatch the work and what the receiving owner sees.
- [ ] Observe each downstream result from the source that performs it.
- [ ] Report partial completion and any unresolved decision separately.
