# Agent Work

Package ID: `pale.agent-work` · [Manifest](package.toml)

Write instructions, plan and delegate work, manage context, diagnose runs and evaluate agents and their host tools.

A workspace selects entries from this family for the duties its people and agents perform. To read it directly, open the page for your task and follow its reading links.

## Guides

- [Check recurring behaviour after an AI-assisted change](<regression.md>) · `regression` · `doctrine`
  Check recurring behaviour after an AI-assisted change, allowing harmless variation while testing state, tool actions, cost and latency.
- [Decide when an agent helps](<agents.md>) · `agents` · `doctrine`
  Decide when an agent helps, how to divide its work, which tools it needs and how to check the result.
  Read as needed: [Design the tools and controls around an agent](<harness.md>), [Plan work that can continue across sessions](<plan.md>), [Check recurring behaviour after an AI-assisted change](<regression.md>).
- [Decide whether to repeat an agent's work](<loops.md>) · `loops` · `doctrine`
  Decide whether an agent's work should repeat, how to measure progress and when it must stop.
- [Decide whether work can run unattended](<unattended.md>) · `unattended` · `doctrine`
  Decide whether work can run unattended by checking its objective, tools, permissions, budgets, stopping conditions and recovery.
- [Design a temporary workflow for one task](<temporary-workflow.md>) · `temporary-workflow` · `doctrine`
  Decide whether repeated steps need a temporary workflow, define its limits and finish or propose reuse.
- [Design an agent's character](<persona.md>) · `persona` · `doctrine`
  Design an agent's character from its relationship with users, voice, limits and responses under pressure.
- [Design the tools and controls around an agent](<harness.md>) · `harness` · `doctrine`
  Design what an agent can observe and do, with tools that enforce permissions, limits and recovery.
- [Find a reusable lesson in completed work](<learning.md>) · `learning` · `doctrine`
  Find a reusable lesson in completed work and propose its home, scope and checks before adoption.
- [Find where an agent run went wrong](<diagnose.md>) · `diagnose` · `doctrine`
  Trace an agent run from request to result, find its first unsupported step and propose recovery within the available evidence and permissions.
- [Find why instructions were missed or context became costly](<context.md>) · `context` · `doctrine`
  Find why an instruction was missed or context became costly, keeping required sources and completion criteria reachable.
- [Keep required context visible](<context-inputs.md>) · `context-inputs` · `doctrine`
  Check that required context reaches each recipient with current permission, including delayed handoffs and permitted reduced tasks.
- [Operate an agent over time](<operate.md>) · `operate` · `doctrine`
  Operate an agent over time with clear ownership, permissions, observations, change checks and incident recovery.
- [Plan work that can continue across sessions](<plan.md>) · `plan` · `doctrine`
  Plan work that can continue across sessions, with ordered inputs, permitted changes, owners, checks and restart points.
- [Review your own result before handing it back](<self-review.md>) · `self-review` · `doctrine`
  Review your own result for correctness, completeness, clarity and unsupported claims before handing it back.
- [Split work and brief the people doing it](<delegate.md>) · `delegate` · `doctrine`
  Split work into useful parts, brief each worker with the request and approvals, and check the combined result.
- [Write an instruction an agent can follow](<instructions.md>) · `instructions` · `doctrine`
  Write a self-contained instruction with the outcome, sources, permissions and a clear finish; return the improved draft.

## Tasks

- [Check how an agent session resumes compacts and closes](<sessions.md>) · `sessions` · `operation`
  Check how an agent session resumes, compacts and closes through its actual host.
  Read first: [Keep required context visible](<context-inputs.md>).
- [Decide whether to repeat an agent's work](<loops.md>) · `loops` · `operation`
  Decide whether an agent's work should repeat, how to measure progress and when it must stop.
- [Design a temporary workflow for one task](<temporary-workflow.md>) · `temporary-workflow` · `operation`
  Decide whether repeated steps need a temporary workflow, define its limits and finish or propose reuse.
- [Design an agent's character](<persona.md>) · `persona` · `operation`
  Design an agent's character from its relationship with users, voice, limits and responses under pressure.
- [Design the tools and controls around an agent](<harness.md>) · `harness` · `operation`
  Design what an agent can observe and do, with tools that enforce permissions, limits and recovery.
  Read first: [Decide whether work can run unattended](<unattended.md>), [Keep required context visible](<context-inputs.md>).
- [Find a reusable lesson in completed work](<learning.md>) · `learning` · `operation`
  Find a reusable lesson in completed work and propose its home, scope and checks before adoption.
  Read first: [Review your own result before handing it back](<self-review.md>).
- [Find where an agent run went wrong](<diagnose.md>) · `diagnose` · `operation`
  Trace an agent run from request to result, find its first unsupported step and propose recovery within the available evidence and permissions.
- [Find why instructions were missed or context became costly](<context.md>) · `context` · `operation`
  Find why an instruction was missed or context became costly, keeping required sources and completion criteria reachable.
  Read first: [Keep required context visible](<context-inputs.md>).
- [Plan work that can continue across sessions](<plan.md>) · `plan` · `operation`
  Plan work that can continue across sessions, with ordered inputs, permitted changes, owners, checks and restart points.
  Read first: [Give the next worker enough context to continue](<../engineering/handoff.md>).
- [Review a search for better instructions](<instruction-search.md>) · `instruction-search` · `operation`
  Review the inputs, effects, independent evaluation and adoption of a proposed instruction.
  Read first: [Check recurring behaviour after an AI-assisted change](<regression.md>).
- [Review a workspace configuration change](<workspace.md>) · `workspace` · `operation`
  Review a workspace configuration change and check its actual host behaviour.
  Read first: [Operate an agent over time](<operate.md>).
- [Split work and brief the people doing it](<delegate.md>) · `delegate` · `operation`
  Split work into useful parts, brief each worker with the request and approvals, and check the combined result.
  Read first: [Give the next worker enough context to continue](<../engineering/handoff.md>).
- [Write an instruction an agent can follow](<instructions.md>) · `instructions` · `operation`
  Write a self-contained instruction with the outcome, sources, permissions and a clear finish; return the improved draft.
