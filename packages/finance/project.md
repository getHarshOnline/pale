+++
description = "Project an owner's books under a framed question, sweep the amount set aside and read what fails first."

[task]
uses = [{ scope = "external", resource = { package_id = "pale.assurance", resource_id = "calculation" } }, { scope = "external", resource = { package_id = "pale.evaluation", resource_id = "numbers" } }, { scope = "external", resource = { package_id = "pale.evaluation", resource_id = "optimisation" } }, { scope = "external", resource = { package_id = "pale.evaluation", resource_id = "notebook" } }, { scope = "external", resource = { package_id = "pale.evaluation", resource_id = "workbook" } }, { scope = "external", resource = { package_id = "pale.evaluation", resource_id = "runs" } }, { scope = "external", resource = { package_id = "pale.evaluation", resource_id = "compare" } }]
capability_policy = { scope = "local", resource_id = "run-policy" }
capability_requirements = [{ id = "check", description = "Check the declaration, captured snapshot and selected Packs the question names before any projection.", capability = "calculation-check", recipe = { scope = "local", resource_id = "check-run" }, effect_class = "read", policy = { kind = "required_block" }, failure_posture = "fail_closed" }, { id = "project", description = "Run the projection or sweep and return its run record.", capability = "calculation-project", recipe = { scope = "local", resource_id = "project-run" }, effect_class = "execute", policy = { kind = "required_block" }, failure_posture = "fail_closed" }]
outputs = ["In the run folder: the snapshot with each fact's state, the results, findings and run record, and the notebook or workbook the question asked for.", "The outcome: complete, or unavailable, unresolved, denied or conflicting, with the missing input, Pack, grant or conflict named."]
+++

# Project the books and read what fails first

Run the preflight first: the exact Pack, the snapshot captured once for this run, the implementation and the limits on what the result may be used for. The engine takes the snapshot, the typed declaration, the Pack hashes and the observation date, and returns results, findings and a run record naming the engine release, the declaration and snapshot digests, the Pack hashes, the seed schedule, the first failure and the stopping reason. It keeps no store and makes no decision; the workspace keeps the run folder, and a person corrects the owning page.

Read a sweep as hit or miss per goal across the amounts tried, with the trade-offs left visible; a single score hides the conflict the question exists to show. Check the candidate an optimiser produces against its constraints, and check numbers against independent references and boundary cases. Simulated return paths and sensitivity follow the comparison method, with seeds, replay and the resampling unit declared before the runs. The notebook shows inputs, method, assumptions and checks so that a fluent reading cannot hide a wrong calculation; a workbook export is checked as bytes and as recalculated values.

A result is a finding for review under the exact inputs selected, and changing any input can make it inapplicable. It files, pays and allocates nothing.

## Before handing off

- [ ] Did the preflight pass on the exact Pack, snapshot and implementation, with the run record kept in the run folder?
- [ ] Does the readout show what fails first and which choices stay open, without collapsing them into one score?
- [ ] Are the findings, the missing facts and the outcome stated separately, each with the owner who can act on it?
