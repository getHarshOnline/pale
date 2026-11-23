# Find where an agent run went wrong

Use an observed case to review an agent system or diagnose a run that stalled, looped, misused a tool, lost context, claimed unsupported confidence or left an action's outcome unknown. Treat the model's explanation as a hypothesis and trace what happened from request to result.

## Establish one case

In the diagnosis, identify the objective, permissions, input and configuration versions, Package and Pack inputs, route, model, context, tools, budgets, events, adapter results, output transformations, rendered result, saved state and evidence of external actions. Mark each as observed, reconstructed, inferred or unavailable. Without a repeatable case, report an architectural risk rather than claiming its cause is established.

Trace request interpretation, context choice, reasoning, tool choice, input preparation, execution, result interpretation, answer composition, rendering, continuity and reconciliation of external effects. These are questions for diagnosis; a system need not use this exact component layout. At each handoff, ask whether information was lost, stale material overrode the controlling source, untrusted content became an instruction, a schema expanded a permitted action, failure looked like success, private data crossed its allowed scope or the display diverged from saved state.

## Find the first unsupported step

Locate the earliest step whose observed output no longer supports the expected next state. In the diagnosis, show the reported symptom, how it appeared and the last step checked as correct. The first visible error may expose an earlier fault. Separate the immediate mechanism from its originating cause and give the support or remaining uncertainty for each.

Distinguish missing data, denial, timeout, malformed input, provider failure, stale evidence, policy rejection, partial effects and unknown outcomes. Form a small set of competing explanations, say what each predicts and use the least consequential check that distinguishes them.

If one missing observation would separate the remaining causes, request it with the case identifier, its owner and what each possible result would establish. Evidence outside the permitted scope, data never retained and an observation nobody collected need different recovery steps; say which is missing.

Prefer replay of fixed inputs, contract inspection or changing one variable. Changing the prompt, model, tool and interface together obscures which correction mattered. An absent receipt is no reason to repeat an irreversible action.

## Match recovery to what is known

When local records and an external effect may differ, read the provider's record for that attempt before retrying. Missing local history leaves the provider outcome open.

Choose a response that fits the failure: a limited retry for a known temporary failure on an idempotent path; corrected input or scope on a sound route; an equivalent, permitted replacement for a failed provider; rollback or reconciliation for contained partial state; or an owner decision when permission, privacy or the effect itself remains uncertain.

Return the case, trace, first divergence, competing explanations, checks, best-supported cause and confidence, smallest correction or recovery scope, validation still needed and remaining unknowns. Distinguish immediate containment, a lasting fix, recovery of affected cases, prevention, observability and the check that could disprove the fix. Name who may authorise each consequential action.

Stop the affected diagnosis or recovery when evidence cannot separate causes, required logs or identities are missing, inputs keep changing, replay could repeat harm or recovery exceeds permission. State what would allow it to continue. Describe the repaired case's result within the scope actually checked.
