+++
description = "Decide whether an agent's work should repeat, how to measure progress and when it must stop."

[guide]

[task]
outputs = ["In the review: the recommended action, supporting evidence and conditions for resuming."]
+++

# Decide whether to repeat an agent's work

Use a loop when repeated observation and action can reach a defined objective more reliably than one attended operation. This guide helps choose how work repeats and what controls it needs. Starting, scheduling, storing and recovering a process are jobs for the actual runtime.

## Establish why another iteration helps

Require an observable target state, a permitted evidence source, a defined set of actions, limits on their effects and a reason another iteration can use new evidence. Reject repetition whose only completion test is open-ended taste, whose progress is self-reported activity, or whose next attempt repeats the same unsupported hypothesis.

Choose the simplest form that fits:

- An ordered pipeline when each step produces the next input.
- Continued work over the same fixed context when the runtime supports it.
- Limited parallel work for independent reads or isolated writes.
- Another attempt triggered by new evidence from a check or observation.
- A dependency graph when outcomes have real prerequisites.
- One cleanup pass when measured effort or clutter justifies it.

Check worker availability, sessions, queues, durable state and dispatch in the actual system before relying on them.

## Define one iteration and when it ends

A cycle observes the state, compares it with the completion condition, chooses one permitted action, asks the owning system to perform it, reconciles the result, then stops or forms the next attempt. Give attempts, time, cost, rate, concurrency, stored state and effects separate budgets.

Test whether the loop moves toward completion and whether its progress measure still represents the goal. Check shared mistakes between generation and judgment, duplicate work, safe replay and containment of effects. Define a no-progress rule, stale-work detection, who can intervene, a safe final state and where recovery can resume. Stop on an exhausted budget, lost permission, an uncertain action that cannot safely be repeated, or repeated failure from the same cause.

Verify a claimed wait through the current job, process or session handle and the system that observes it. An old record or timeout cannot establish whether work is still running. Compare progress with the objective and actual state, allowing for a verified wait. Reconsider retries against the unresolved cause, side effects and scope. Reconcile actions with unknown outcomes before treating them as safe to repeat.

For a proposed loop, recommend allowing, redesigning or rejecting it. For a running loop, recommend continuing, intervening, stopping or leaving the decision unresolved. Return the review with its form, state changes, progress evidence, budgets, stopping and intervention rules, fallback and evidence needed to resume. Use execution records to establish whether a loop exists or ran.

## Before handing off

- [ ] Can the chosen signal distinguish useful progress?
- [ ] Have repeated failure, budget exhaustion and interruption been exercised?
- [ ] Can the loop stop without losing required state?
