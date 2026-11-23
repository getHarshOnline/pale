+++
description = "Frame a money question so one run can answer it: whose books, which facts and rules, what is unknown and what counts as failing."

[task]
uses = [{ scope = "external", resource = { package_id = "pale.assurance", resource_id = "rules" } }, { scope = "external", resource = { package_id = "pale.research", resource_id = "evidence" } }]
outputs = ["A question the workspace can run: the book owner, the permitted sources and grants, the selected Packs, the observation date, the scenarios and the unknowns declared as swept.", "The facts that are missing, stale or in conflict, each with the owner who can settle it."]
+++

# Frame a money question

A money question asks whether an owner can meet its obligations and goals, at what monthly amount, with which choices still open and which fails first. Name the owner whose books it reads: a person, or an organisation acting through the seat that holds the question. A question over several owners reads each set of books under its own grant and records nothing of its own.

Write the question so a run can refuse it. It names the facts it needs and the page or statement that owns each one, the grant that permits reading it, the Pack of rule values selected by identity and hash for the period, the observation date and the scenarios it compares. An unknown enters only when the question declares it swept across a range; any other missing, stale or conflicting fact stops the run and is reported with its owner, never guessed. An owner's own choices, such as what counts as a reserve or when a loss counts as recovered, stay as guidance a reader interprets, not values the engine computes on.

A scenario is a declared intervention, a dated change to the books, never an observed fact, and the scenarios a question compares are its arms. Income enters two ways: swept like the amount set aside, or as dated income scenarios, each marked known, unknown or hypothetical with its source.

Research which rules apply before selecting a Pack, and judge what the sources support. Obligations come first; the flexible decisions sit inside the room they leave. Sweep the amount set aside rather than asserting one figure, and let a conflict between goals show instead of inventing income to cover it.

## Before handing off

- [ ] Does the question name its owner, every fact it needs with its source and grant, the Pack and the observation date?
- [ ] Are the unknowns declared as swept, with everything else known or reported as missing, stale or conflicting?
- [ ] Does it say, before any run, what it treats as failing and which choices it leaves open?
