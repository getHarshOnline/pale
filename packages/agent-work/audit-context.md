# Find why an instruction was missed or context grew

Start from the reported failure and the actual task. For an instruction-routing problem, replay the path through the instructions and discovery text the host exposes. Use “Diagnose routing before trimming” in the required context-budget guidance. For a context-cost question, list the chosen inputs in the existing review and compare them with what the host actually received where that is observable; measure only that scope.

Fix the cause with the smallest change that keeps the needed sources reachable, respects instruction priority and the owner's limits, and lets another reader find the evidence. Repeat the same task with the same completion criteria. A routing check can show which instructions and decisions were reached; implementation or rendered results still need their own run when the task requires it.

Return the correction, what the comparison showed and checks still missing in the task's reply or review. Propose any removal from live context before carrying it out unless that removal is already authorised. Finish when the cause and the corrected route are established; a smaller input alone is not the result.

## Before handing off

- [ ] Does the correction explain the reported failure on the actual host and task?
- [ ] Can the worker still reach every source and instruction the task needs?
- [ ] Does the result distinguish what was measured from what still needs a real run?
