# Check reference answers and fix dataset membership

## Bind the case, assessment and question

Identify the exact case and input version, observation or attempt, review question and scoring definition. Keep the original output, observed annotation, proposed correction, adjudication and accepted reference answer separately identifiable. The same annotation delivered through another screen, file or API is still one judgment. A correction needs a new revision linked to its predecessor. Keep conflicts visible until the accountable reviewer settles the exact question.

## Record admission for the declared use

Identify who accepted the answer, its accepted revision, the policy, permitted purpose and evidence of source-use rights. Completed review or successful import cannot replace missing acceptance or broaden permitted use. A label valid for one question or population cannot fill a different reference set. Keep unreviewed, deferred, conflicting, denied and withdrawn items outside the accepted set, retaining their status. Recheck eligibility when consuming data whose rights can expire or be withdrawn.

## Establish review coverage before scoring

Connect each response to the exact candidate, review question and eligible decision population. Use the review record to establish which decisions were actually reviewed. Distinguish explicit approval and rejection from ambiguous feedback, blank responses, missing fields and absent review records. Keep reported or observed downstream actions separate from the human response. A decision may take effect while its review status remains unknown.

Retain the original response and context separately from a model's proposed interpretation and the adjudicated label. Approval establishes the reviewer's judgment on the named question. Correctness, optimality and approval of a later action each require evidence for that particular claim.

Report eligible cases, verified review coverage and decisive responses. When reporting acceptance among decisive responses, show that denominator. If coverage is incomplete or unknown, limit conclusions to observed responses. Samples containing only comments or exceptions can overrepresent objections.

## Check a change to annotation meaning

Identify both guideline revisions, including question, annotation unit, allowed answers, required evidence, ambiguous cases and review rules. Classify each change as equivalent wording, changed meaning or evidence requirements, or unresolved. Check claimed equivalence on paired cases near each changed boundary. Identical labels and reviewer agreement alone do not establish equivalence.

Hold source cases and candidate outputs fixed while comparing rules. Use independently adjudicated examples covering decisive results, ambiguous or missing evidence and cases handled differently by the old rule. Report paired transitions and denominators, including unreviewed and unmapped cases. A different measured quality under new rules does not establish changed candidate behaviour.

Keep every original answer and guideline revision. An accepted equivalence mapping may support a separately identified derived comparison. Changed meaning or evidence requirements need fresh review under the new guideline before a reference can be accepted for it. If re-review is unavailable, report the coverage gap. Keep old comparisons reproducible and tie new membership and downstream claims to the new revision.

## Freeze the comparison frontier

Fix item identities and revisions, input and reference digests, membership and partition assignments. Identify dataset and schema revisions separately. Group related cases by source, subject or derivation where information could leak between partitions. Hold out whole groups when the claim requires independent generalization. Include prior tuning, threshold selection and prompt examples in that check. A renamed item, duplicate capture or corrected version may still belong to the same group.

Keep a comparison's fixed inputs unchanged when a reference is later corrected. Identify the successor version and reopen affected conclusions only with its exact inputs and membership. An unavailable or ineligible item leaves a coverage gap; it cannot silently be replaced. Return case and label histories, statuses, split and eligibility evidence, fixed membership and unresolved items. The owning system accepts, creates or writes the dataset and authorises its use.

## Verify use at the consumer boundary

Before using an external reference, verify its exact version and digest, current status and permission for the consumer, purpose, data scope, destination, model use and time. Proposal, curator acceptance and creation of a dataset version are different steps. A use grant establishes eligibility; a use receipt identifies the version actually consumed by that run. When both owners record a handoff, match the source's use receipt with the consumer's fixed input and run identity.

Leave a required reference unresolved when current eligibility or evidence of use is missing. Substitute only if the comparison owner permits changed inputs, making that change explicit. Preserve earlier use receipts. Apply withdrawal, replacement and expiry to subsequent use, and send affected derived evidence to its owner for handling under the applicable policy.

Test a matching use, permission for another purpose or destination, an expired or withdrawn revision, and a receipt for another version or run. Report permission and actual use separately from evaluation results.

### Recheck retained reference use and result delivery

Distinguish inspecting a historical receipt, reconstructing its recorded inputs and consuming those bytes in a new run. Each action requires permission for what it reads, processes or discloses. A signature, unchanged digest or earlier use receipt can establish an earlier event without making a retained copy eligible today. A retry may recover the same recorded outcome where policy permits. Delivering protected bytes again still needs permission for that delivery.

Before new consumption, check the retained revision against the current grant, status and revocations from the authorities that own them. A restored snapshot, cached grant or local receipt cannot override their current decision. Hold dependent use if current authority cannot be established. Any permitted substitute needs separately accepted inputs and its own receipt.

For work in progress, establish how an effective permission change reaches the run and where it must be rechecked. Before exposing the final result, verify the selected recipient, result and current permission. Resolve a race with withdrawal using the owning system's ordering or conditional acceptance rule. Distinguish processing already observed, evidence allowed to remain, withheld delivery and the handling of downstream results. The owners decide whether affected work must stop, be isolated or be otherwise disposed of.

Test a historical receipt whose retained copy is now ineligible, a lost delivery acknowledgement followed by withdrawal, and permission that changes between acceptance and delivery. Include an authorised current path and an independently eligible replacement. Verify that refusal exposes no protected result, old receipts remain historical and the new attempt cannot inherit an earlier run's authority.
