+++
description = "Find why an instruction was missed or context became costly, keeping required sources and completion criteria reachable."

[guide]

[task]
uses = [{ scope = "local", resource_id = "context-inputs" }]
outputs = ["In the reply: why the instruction was missed and the smallest fix, or how much context the host loads and what could go, with what was not checked."]
+++

# Find why instructions were missed or context became costly

Start with the task and the decision the user needs. Diagnose a missed instruction from the route it took through the host. When the question is cost or redundant content, measure what was loaded before proposing cuts.

## Measure the context in use

Identify each component consuming context and its token cost. Find repeated or low-value material, then compare possible savings while keeping required sources, constraints and completion criteria reachable. Report actual free space when the host exposes it. Propose removals before carrying them out unless they are already authorised.

Keep the authored library, the selected and rendered files, and the model's observed input separate. In the audit, identify the profile, host, point in the turn, file versions and content actually loaded or injected. Count physical copies, effective bodies and repeated meanings separately. A skill on disk is available to read; establish whether it reached the model before counting it as used context.

Give measured bytes or tokens with the method, tokenizer or model, and scope. Use host usage or context observations for actual spend and free space. When unavailable, report file sizes or labelled estimates. For each proposed trim, keep the owning source and its scoped exceptions, and run a check that the consumer still reaches what it needs.

## Diagnose routing before trimming

Replay the task from its applicable instruction scopes and the descriptions the host actually supplied for finding skills. Trace the chosen skill, references opened, required sources missed, duplicate instructions and unnecessary intermediate files. Distinguish an overbroad trigger, a broken link, a missed source and an optional procedure treated as mandatory. Correct the description, link or instruction at the first cause the evidence supports. Use the smallest change that respects instruction priority and the owner’s limits, keeping required context, constraints and checks reachable.

Compare the original and corrected routes using the same task and completion criteria. If a trigger changes, also use a nearby task that should take another route. Compare references reached, avoidable work and the delivered result. For design tasks that include rendering, compare the rendered outcome with the same selected originals and states.

## Compare ways of finding the same context

Before the comparison, fix the questions, source revision, permitted disclosure, model and available tools, answer criteria and failure conditions. Use equivalent tasks with the same available evidence. In the comparison, show which sources and tools each variant reached, its coverage and failures, and the context that reached the model. A smaller answer or cheaper run must still satisfy the same task.

Measure setup and maintenance separately from each answer. Include compiling or indexing reusable context and refreshing it after an owner update. Distinguish cold and warm runs, cache reuse and repeated searches for the same information. Use observed tokens and elapsed time where available; label estimates and their scope. An amortised cost needs its setup cost, number of answers, refresh schedule and per-answer assumptions beside it.

Judge unsupported inferences, missing or stale facts, mistakes about who can decide and disclosure failures against the fixed criteria. Keep failures that prevent use separate from time or token savings. Report setup and per-task cost, quality and coverage together. Label an isolated baseline observation as such until there is a comparable experiment. Check permissions and actual actions separately from the context-cost result.

Finish a routing diagnosis when the cause and the corrected route are established. Finish a cost audit when the selected scope is measured and any proposed reduction preserves the task’s requirements. Return the correction or proposal, what the comparison showed and checks still missing in the task’s reply or review. A smaller input alone is not the result.

## Before handing off

- [ ] Does the correction explain the reported failure on the actual host and task?
- [ ] Can the worker still reach every source and instruction the task needs?
- [ ] Does the result distinguish what was measured from what still needs a real run?
