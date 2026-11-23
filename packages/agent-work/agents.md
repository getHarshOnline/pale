+++
description = "Decide when an agent helps, how to divide its work, which tools it needs and how to check the result."
see_also = [{ scope = "local", resource_id = "harness" }, { scope = "local", resource_id = "regression" }, { scope = "local", resource_id = "plan" }]

[guide]
+++

# Decide when an agent helps

Use an agent when a task needs judgment across changing evidence or conditional actions. Start by comparing four ways to do the work: a direct deterministic operation, an expert working with assistance, an agent carrying out a defined task, and repeated or unattended autonomous work. Mechanical transformations belong in ordinary code; consequential decisions still belong to whoever has the right to make them.

Compare how much inputs vary, how completion can be observed, the cost of an error, required permissions, available tools, context needed, reversibility, independent verification and coordination effort. Choose an agent only when the outcome and stopping point are clear, the tools it needs actually work, success can be distinguished from a plausible answer, and recovery suits the possible effects.

Split the work where dependencies, context, risks or checks differ. A specialist should contribute a distinct judgment; a loop should gain evidence that can support progress. Keep mechanical rules in deterministic components and use model judgment where ambiguity remains. Match evaluation to the consequences and recurrence of the behaviour. Self-review is useful advice; important results need an independent check.

In the proposed plan or reply, name the chosen approach and why, the capabilities and checks it needs, how the work divides, decisions that need a person, risks and facts needed to resume. Use the invocation's actual model, provider, session, tools, observations and actions when assessing what ran.

## Choose a guide

| If you are doing this | Read this |
| --- | --- |
| Give an agent tools or control of a repeatable job | [Design the tools and controls around an agent](harness.md) |
| Check whether an instruction or tool change fixes a recurring failure | [Regression checks for AI-assisted systems](regression.md) |

For work that resumes across sessions or owners, use **Plan work that can continue across sessions** from this family’s index.

## Worked example

A request asks to verify that generated links resolve and explain one surprising destination. Run the repository's link check directly. Use judgment to follow the source of the surprising link and explain where its target comes from. If the target is another team's page, leave that team the source and proposed correction. The link check and the explanation have separate ways to verify the result.

## Before handing off

- [ ] Is the result clear, with a way to establish that it works?
- [ ] Does the division of work follow its dependencies, effects and checks?
- [ ] Has the actual result been checked before widening the workflow?
