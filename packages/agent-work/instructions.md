+++
description = "Write a self-contained instruction with the outcome, sources, permissions and a clear finish; return the improved draft."

[guide]

[task]
outputs = ["In the reply: revised instruction, reasons for material changes, open assumptions and component choices checked."]
+++

# Write an instruction an agent can follow

Use this method to improve a draft request. Return the revised instruction and a short explanation of what you clarified; carrying out the request is a separate task.

## Find the task behind the draft

Read the original request and its corrections. Establish the outcome, who will do the work, the sources they need, constraints, permissions already given and the form of the result. Check that a named tool or component exists and can do the job before making the instruction depend on it.

Separate the desired result from the draft's chosen wording. Keep deliberate ambiguity and voice while repairing omissions that would change the result. Resolve missing sources, unclear ownership, hidden assumptions and conflicting constraints. Say what a satisfactory result will show. A formatting requirement helps present that result; it cannot substitute for it. Leave a capable executor room to reason instead of prescribing unnecessary intermediate steps.

## Carry the whole request, with its limits

Say what to do in the words someone would use to ask for it. Lead with one clear objective and include the context that changes the work. Carry the user's original request, material corrections, settled decisions and existing approvals together. Distinguish them from earlier proposals, observed results and unanswered claims. Name the instructions still in force and those superseded. Link the evidence that supports a result; an earlier assistant's summary helps locate it but does not replace it.

Write a rule with the limit that keeps it useful: preserve features, cases and examples while simplifying their expression; run the check that answers the question and stop when it does. A step that says to record, keep or verify something names where the result belongs and when the step is finished. Usually that is the changed page or code, a maintained test, or a sentence in the reply. Cut a step whose result has no purpose or destination. Run the useful check and give its result in a sentence; include output when a failure needs explanation. Records the system exists to keep belong to the product. Create a separate account of the agent’s work only when the task or a later consumer needs it.

Name a reusable method, role or tool by the job it performs and point to its instructions instead of copying them. State what the worker may change and what they must leave alone. Continue through work already authorised; ask only for a material choice or permission that is still missing, and keep independent work moving while it waits. Never turn an earlier approval into a fresh stop point or imply that an unresolved approval has arrived.

Keep the opening rules usable at a glance. A new rule sharpens or replaces one already there; put the fuller explanation below it.

## Finish with a usable handoff

Use representative checks to show how the instruction is found, where it leads and whether it causes unwanted work. Return the revised instruction, material reasons, open assumptions and component choices checked in the requested destination. Finish when a fresh executor can identify the outcome, sources, permissions, constraints and expected result without reconstructing the conversation. End the resulting instruction path once with three or four questions about its result, rather than repeating the steps on every supporting page. Improving an instruction gives no extra permission to perform the work it describes.

## Before handing off

- [ ] Does the draft keep the original task and constraints?
- [ ] Do representative checks cover how the instruction is found and unwanted work it might trigger?
- [ ] Does the reply give the draft and what the comparison established?
