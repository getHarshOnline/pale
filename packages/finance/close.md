+++
description = "Close one owner's books for a period from statements and the accountant's books, and prepare the differences for review."

[task]
uses = [{ scope = "external", resource = { package_id = "pale.assurance", resource_id = "register" } }, { scope = "external", resource = { package_id = "pale.evaluation", resource_id = "workbook" } }]
capability_policy = { scope = "local", resource_id = "run-policy" }
capability_requirements = [{ id = "check", description = "Check the declaration, captured snapshot and selected Packs the question names before the close.", capability = "calculation-check", recipe = { scope = "local", resource_id = "check-run" }, effect_class = "read", policy = { kind = "required_block" }, failure_posture = "fail_closed" }, { id = "close", description = "Reconcile the period's opening position, movements and closing position for one owner and return its run record.", capability = "calculation-close", recipe = { scope = "local", resource_id = "close-run" }, effect_class = "execute", policy = { kind = "required_block" }, failure_posture = "fail_closed" }]
outputs = ["In the run folder: the period's opening position, accepted movements, closing position and the differences left open, each with who can settle it.", "The files the owner's accountant reviews, exported under the workspace's layout, and the run's outcome."]
+++

# Close an owner's books for a period

Close one owner at a time. Reconcile the opening position plus the period's movements to the closing position, account by account, against the statements and the accountant's books the workspace selects as sources; the register method says how to match movements and what to leave open. A transfer between the owner's own accounts is neither income nor spending. A view across several owners is a separate question that reads each close under its own grant and removes the transfers between them; it is never a merged set of books.

The close returns accepted movements, unmatched parts, possible duplicates, unresolved classifications and the questions for the accountant, each with the owner who can settle it. Export the review files under the layout the workspace supplies and check the workbook as bytes and as recalculated values. Sending the files for review is not acceptance, a calculated duty is not a filing, and the accountant's answer comes back as a review of the close, not as a change to the books.

## Before handing off

- [ ] Does the close reconcile one owner's opening position, movements and closing position against its selected sources?
- [ ] Are internal transfers removed, and is every open difference left visible with who can settle it?
- [ ] Are the review files exported and checked, with sending, acceptance and filing kept as separate acts?
