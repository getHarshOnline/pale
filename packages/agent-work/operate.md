+++
description = "Operate an agent over time with clear ownership, permissions, observations, change checks and incident recovery."

[guide]
+++

# Operate an agent over time

An agent running over time needs ordinary production controls, plus controls for probabilistic decisions and delegated actions. Use this guide to assess how a particular workload will be operated. The owning system performs startup, pausing, deployment and supervision.

## Establish what runs and who is responsible

In the operating plan, identify the objective, artifact revision, model and configuration, allowed capabilities, data classification, effect limits, owners, service expectations and what can be rolled back together. Keep the Package's guidance, executable release and changing environment distinct. The operator must be able to identify what is running and whose permission it uses.

Grant only the capabilities and data access needed for the task. Supply secrets at execution time for the smallest necessary purpose; keep them out of prompts, logs and artifacts. Limit time, attempts, spending, rate, concurrency, output volume and stored state. External or hard-to-reverse effects need stronger evidence, safe retry or compensation, and a way to intervene.

## Observe decisions and actions

Collect enough structured records to reconstruct input identity, chosen route, consequential decisions, attempted tool actions and effects, validation, final status, cost, latency and recovery. Minimise sensitive data and control access. Distinguish successful outcomes, retries, partial effects, operator interventions, policy denials and unknown results. More activity does not establish greater reliability.

Give each watched job one observer and a working route back to the task that will act on its result. Prefer the host's event or wait facility; use bounded polling only when needed, respecting the provider's shared rate limits. Process existing failures before waiting for new events. After interruption, reconcile the same job and its last observed result before starting another observer. A timeout or lost notification leaves the outcome unknown.

Continue until the agreed result is observed or a specific dependency needs another owner. Report meaningful changes, failures and decisions rather than repeated waiting messages. When handing off, stop or transfer observation and put the live job reference, unresolved findings and next action in the existing task. Ending the observer does not cancel the underlying job; inspect that job before claiming cancellation or completion.

## Change and recover safely

Treat changes to prompts, Packages, Packs, models, tools, permissions and policies as versioned changes in behaviour. Test them against stable cases and failure scenarios, compare with the current release, obtain approval for the actual change, limit initial exposure and keep a tested return path.

During an incident, contain new effects, record the precise state reached, preserve evidence and locate the failure: judgment, data, tool connection, provider or owning system. Make the smallest reversible correction, check it in isolation and resume under the owner's permission. Preserve failed traces so the record remains honest.

Return the operating plan or review with identities, controls, budgets, required observations, change checks, incident procedure, remaining risks and owners. Hold affected work when the running setup cannot be reproduced, permissions exceed need, effects lack result records, intervention is impossible, private data lacks an accountable owner, rollback is untested or evidence cannot distinguish success from an unknown outcome.

## Before handing off

- [ ] Is it clear who operates, changes and recovers the agent?
- [ ] Do permissions, logs and runtime settings remain with their owners?
- [ ] Have retry limits and an incident recovery path been checked?
