+++
description = "Check human-in-the-loop (HITL) decisions, usable reviews and the result actually applied."

[guide]
+++

# Check operator review and permission to act

Human-in-the-loop (HITL) review keeps a recommendation, its review, permission to act, the applied change and the observed result distinct. Give the reviewer the original evidence and exact proposed change. Record who decides, their scope, the intent, candidate revision, expiry and allowed action. Authenticate a response independently of its display name or email address. Model confidence can prompt a referral to a person; it cannot grant that person's authority.

Test accept, reject, revise, expire, cancel, duplicate and late responses. A changed candidate needs a new decision. Use the owning system's revision and duplicate-request rules when reviews or applications race. Show whether review is complete, action is approved, an attempt was made, the change committed or its result was verified. Explain why a proposal is held and what evidence would let it proceed.

Operational approvals and evaluation annotations may share delivery and recovery machinery. Give them separate response formats and destinations. Redelivery preserves response identity; a correction gets a new revision linked to the original. Verify that replaying an annotation cannot resume an operational workflow.

<a id="qualify-operator-queue-selection-and-action"></a>

## Check which work appears in the operator's queue

Identify whose queue it is, what makes a row eligible and which policy orders the rows. Keep assigned work, requests raised by the viewer, missing input they owe and available work separate where they answer different questions. Apply visibility rules before ranking, counting or pagination. Use the owning policy for status, due time, priority, tie-breaks and freshness, and show why an item appears or needs attention. A display preference cannot change the source system's priority or permission rules.

Identify the viewer, query and snapshot or continuation. Distinguish a complete empty result from unfinished pagination, an unreadable relationship, stale data and an unavailable source. Show partial results and the permitted recovery without exposing restricted records through counts or messages. When order changes between pages, test the source's continuation rules for omissions and duplicates.

Opening a row differs from claiming work, submitting a judgment and applying a change. Immediately before the action, recheck the actor, case, revision, assignment and prerequisite evidence, even if the queue entry was eligible. Show holds and reservations explicitly. Test competing operators, expiry and reassignment, and explain the permitted next action after refusal or a stale selection.

Include equal priority, competing ordering signals, incomplete and genuinely empty reads, a restricted or ineligible record, a stale selection and two operators attempting the same guarded action. Inspect queue membership, order, coverage and business changes separately. The owning system determines queue policy, claims, judgment authority and effects.

## Reconcile callback identity and destination

Give the transport event, review response and applied change separate identities. Match a response to its review instance, question, candidate revision, reviewer authority and owner-selected destination. Use the transport event to check delivery and replay. Resolve the destination from the accepted review contract, rather than a route or display identity proposed by the callback.

Check authority, expiry and candidate revision before accepting a new response. Deduplicate retries even if the provider gives them new delivery IDs. If distinct responses race for one transition, accept only the transition permitted by the owner's concurrency rule. Reusing an identity with different content is a conflict. A correction gets a new response revision linked to its predecessor and follows the receiver's correction process.

Where the system supports it, record acceptance and the resulting delivery obligation in one commit. After an uncertain acknowledgement, read both response and effect state before retrying. An evaluation annotation has its own assessment reference and receiver contract. Its acknowledgement cannot authorise a business continuation, even when both routes share a sender.

Test duplicate deliveries, conflicting responses, a changed revision, a misrouted annotation and an acknowledgement lost after commit. Read each destination separately. Report the accepted response, observed effect or unresolved gap, and permitted next action.

<a id="admit-a-scoped-override-or-handover"></a>

## Check an exception, correction or handover

Preserve the original recommendation, evidence, finding or assignment. State whether the new decision is a correction, authorised exception, replacement baseline or accepted handover. A correction records what was wrong. An exception applies to a named condition for a limited case or period. Replacing a baseline needs authority over that source revision. Responsibility transfers only when the receiving owner accepts it.

Identify the exact change, original revision, actor, reason, evidence, permitted scope and expiry. Recheck current authority, case state and requirements that cannot be waived before accepting or applying it. Approval for one requirement cannot waive another. A changed target requires a decision against its new revision. Retain denied, stale and conflicting submissions without rewriting the source.

Keep the accepted decision distinct from the observed change. Read the owning system before claiming a change occurred. An exact duplicate can return its recorded outcome. If someone intervenes in an experiment, preserve the original assignment and proposal alongside what was applied. The evaluation owner decides how these observations support a claim.

Test a valid exception, a requirement that still blocks action, an expired or out-of-scope decision, a changed revision, changed content under a duplicate identity, and an unaccepted handover. Read the unchanged original, accepted review, actual effect and current responsibility separately.

## Escalate a coverage failure without transferring the work

Identify the open duty, required coverage, response window and current evidence of available responders. A named role or roster entry does not establish availability. Distinguish routine delay, missing or stale availability evidence, and a verified breach of the owner's threshold. Preserve the original duty and evidence when opening a handoff or escalation.

Before publishing an alert, check the current escalation rule, destination, disclosure permission and expiry. Keep one identity for that escalation. Send the minimum permitted alert, with a protected reference where allowed. Check access to the underlying evidence again when the reference is opened. Leave an absent, ambiguous, stale or revoked route pending for its owner. An alternate destination needs its own declared route and current permission.

Record publication, acknowledgement, explicit acceptance of responsibility, response and resolution separately. A timer or opened alert gives no authority for a further action. A transfer uses current assignment and actor checks in the owning system. A late acknowledgement can remain historical evidence after an offer expires; apply current rules to any new action.

Test quiet delay, a verified breach, an unavailable rostered responder, a missing or revoked route, a duplicate alert and a protected link denied after publication. Inspect the same open duty and, where permitted, one alert, separating publication from acknowledgement. Include late acknowledgement and separately accepted responsibility. The owner defines thresholds, eligible roles, actions and the evidence for them.

## Distinguish a usable human wait from a broken review

Human review asks an authorised person for a decision the workflow cannot make alone. Establish the case and reason, sufficient context, an accessible form, an authorised assignee, a working response route and the transition that consumes the decision. A pending label proves none of these. A usable request within its agreed response window is a legitimate wait; a missing form, failed notification or broken callback needs technical repair.

Match the local review, external form and response identities before creating a replacement. Record the decision, actor, time and changed inputs. When a review is overdue, use the owner's deadline and escalation route, with a named next owner and an actionable request. Repeated alerts do not transfer responsibility. After a response, verify that the workflow consumed it and resumed or reached an explicit disposition. Manual takeover leaves the remaining work with an owner who has accepted it.

For example, a review is marked approved but the requested submission is absent. Check the accepted response and actual provider outcome before retrying the submission or reopening the review. If the provider outcome is unknown, reconcile that attempt first. A resolved form or a row disappearing from a dashboard does not establish completion.

Route each exception by what it needs. A known data correction, deterministic refusal or transient transport repair can have an existing system procedure. Refer a case to a person when it needs that person's judgment or authority. Keep operational approval separate from annotation that evaluates an earlier answer.

## Before handing off

- [ ] Show the item, proposed action and current state to the operator.
- [ ] Keep review, permission to act and applied outcome distinct.
- [ ] Read back the result before reporting it as complete.
