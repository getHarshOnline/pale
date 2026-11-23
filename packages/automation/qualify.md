# Check a workflow change before it goes live

Identify the workflow's entry conditions, candidate version, owner, environment, mode and inputs in its existing review. Say the most it may do, such as sending a message, writing to a database or charging a card. Mark which transitions are deterministic, use a model, need a person or change an outside system. If a prerequisite or permitted effect is unclear, hold the dependent step and continue independent checks.

Follow every branch, child workflow and error route. Give each model step the smallest decision it needs to make, the required output shape, when to defer and which declared review or hold route applies. Refer to a person when that workflow requires one. Validate deterministically and look up the actual record before allowing an effect; a model's answer alone is not that lookup.

Run an ordinary case through review and inspect the result in the system that owns it, using an environment where those effects are permitted. Then try a stale decision, a duplicate callback, an ambiguous delivery and a replay child attempting a live effect. When no provider is connected, test a simulation of these cases and say which real paths remain untested.

In the workflow review or reply, show the affected systems and effects, the candidate and deployed versions, the result of each case, remaining gaps and how to recover safely. Give replayable cases to a separately chosen evaluator only with an allowed input snapshot and live effects disabled. Say separately what was approved and what actually changed. Finish when the requested cases answer whether this change is ready, or identify the specific missing check or decision. The usual mistake is to try only the ordinary case.

## Keep an unassessed check visible in a partial run

Declare which sources and earlier results each check needs. If extraction of one source fails, retain successful extractions and continue independent checks. Mark a dependent check **Not assessed**, with the missing prerequisite and failure reason. Do not turn it into Pass, Not applicable, a negative business answer or an omitted row. Distinguish a source that could not be read from a readable source that lacks required evidence.

Report processing completion, assessed findings, evidence coverage and permission to proceed separately. A completed check can find a mismatch or missing evidence. A partially processed run can contain useful completed checks without satisfying the requirements for the overall action. Apply the owner's rule for that action and show which requirement remains unresolved.

For example, a run has two originals and three checks: one needs the first original, one needs the second, and one needs both. If the second extraction fails, the first check can finish; the other two remain Not assessed and the run is partial. Recovery preserves the first result and follows the frozen run's retry rules. A changed source or method needs the system's reassessment procedure and its link to the earlier run.

## Before handing off

- [ ] Do the results cover the ordinary, failure, retry and replay paths affected by this change?
- [ ] Are the observed effects within the workflow's permissions, with a usable recovery route?
- [ ] Can the owner see what passed, what was simulated and what still needs a decision or check?
