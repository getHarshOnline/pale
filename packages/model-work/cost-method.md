# Keep model-assisted work within its value and budget

Use this guide when model calls, long inputs, retries or repeated search can consume resources without improving the intended result. Cost is one requirement alongside quality and uncertainty. A cheaper model is useful only if it can do the work.

Start with the decision the result must support. In the comparison, state minimum quality, consequences of error, remaining uncertainty, evidence that would show improvement, and the owner's maximum time, money and calls. Choose a cheaper route when observed quality meets these requirements. A more capable route is justified when it can plausibly change the decision or resolve a named uncertainty.

Count the whole task: input and output volume, repeated system or retrieved context, candidate generation, evaluation calls, tool-mediated model calls, retries, fallbacks and loop branches. A low price per call can still cost more through repair, review, latency or failure. Compare routes on the same representative cases, including abstention or manual handling when automation is uneconomical.

Reuse requires stable input identity and current freshness, purpose and recipient permissions for that use. Keep private and task-specific material within its permitted route and scope. Cached prompts, responses, embeddings, rankings and retrieval results are disposable optimisations. Their presence does not make them source truth, permission, evidence or Package guidance. Invalidate or bypass them when inputs, policy, permissions, model behaviour or required freshness change.

Before a sequence of calls begins, set maximum calls, elapsed time and candidates, allowed retry categories, evaluation independence and a no-progress stop. Retry an observed transient failure only when another attempt can change the outcome. Deterministic rejection, invalid input, missing permission, repeated evaluator disagreement or exhaustion of the budget for resolving uncertainty requires stopping or reconsidering the work.

In the recommendation, give the chosen route, alternatives rejected, complete cost estimate, quality and risk evidence, cache and freshness conditions, fallback order, budgets, stopping triggers and remaining uncertainty. If no route is shown to meet the value requirements within budget, recommend manual handling, deferral or report unavailable.

The Package supplies the comparison method. Verify current prices, quotas, models, provider availability, cache contents, metering, billing and actions through their owning Pack, private configuration, engine or provider when used.

Keep planned limits, measured usage, invoice reconciliation and unknown or delayed charges distinct. Count every attempted provider call, including failures and retries, against the task budget. Identify who supplied rates and currency assumptions. Local elapsed time or a simulated meter does not establish billed usage. Stop before another attempt if its worst applicable cost or effect cannot fit the remaining limit.

## Check what a cache actually reuses

Identify the object reused: prompt-processing state, a generated answer or an action result record. Prefix reuse does not return an earlier answer. A key that prevents duplicate writes does not establish reuse of a model result. Check cache eligibility and actual inputs for the provider route, model revision, source revisions and permitted scope. Preserve instruction and source priority when arranging stable context; padding or reordering to reach an assumed cache threshold must not change the meaning.

Compare equivalent cold and warm work under the same quality criteria. State which provider counters were available, with their units and scope. Distinguish observed reuse from configuration intended to encourage it. An absent reuse counter is unknown, not measured zero. Prompt-token reuse, reduced transport, latency and billed savings each need their own evidence.

For explicitly retained context, include creation, storage duration, lookup, refresh and disposal in the cost and ownership model. Keep input, recipient, freshness and invalidation requirements effective across reuse. One opportunistic hit cannot establish persistence for the next request. Verify current provider limits, prices and result records. Choose an explicit cache when measured recurrence and coverage justify managing its lifecycle.

## Report cost for assigned work and accepted outcomes

Use the task's accounting records to connect assigned work, execution attempts, measured quantities and accepted outcomes. One case can need several attempts or end in abstention or manual handling. Allocate retries, failures and shared work once under a stated rule. Reconcile usage and acceptance records at their own keys before joining them. When analytical-grain guidance is selected, apply its join and denominator checks to the underlying table.

Report total known usage, missing coverage and cost per assigned unit. For cost per accepted outcome, use the same work population in the numerator and count only accepted outcomes in the denominator. Include the acceptance rate, manual and fallback costs, and unresolved charges. Keep a statistic for successful attempts alone separate. Zero accepted outcomes makes cost per accepted outcome undefined; incomplete metering gives only a partial numerator.

## Before handing off

- [ ] Do capability and spending fit the decision and its uncertainty?
- [ ] Are retries, long inputs and repeated paths included?
- [ ] Were cost and quality checked together on the intended work?
