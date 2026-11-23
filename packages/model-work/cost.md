+++
description = "Choose a model route that meets the task’s quality and budget, including context, retries, caches and repeated calls."
see_also = [{ scope = "external", resource = { package_id = "pale.evaluation", resource_id = "rows" } }, { scope = "external", resource = { package_id = "pale.evaluation", resource_id = "compare" } }]

[guide]

[task]
outputs = ["In the assessment: route, budget, comparable evidence, fallback, stopping conditions and missing usage measurements."]
+++

# Keep model-assisted work within its value and budget

Use this guide when model calls, long inputs, retries or repeated search can consume resources without improving the intended result. Cost is one requirement alongside quality and uncertainty. A cheaper model is useful only if it can do the work.

Start with the decision the result must support. In the comparison, state minimum quality, consequences of error, remaining uncertainty, evidence that would show improvement, and the owner's maximum time, money and calls. Choose a cheaper route when observed quality meets these requirements. A more capable route is justified when it can plausibly change the decision or resolve a named uncertainty.

Count the whole task: input and output volume, repeated system or retrieved context, candidate generation, evaluation calls, tool-mediated model calls, retries, fallbacks and loop branches. A low price per call can still cost more through repair, review, latency or failure. Compare routes on the same representative cases, including abstention or manual handling when automation is uneconomical.

Reuse requires stable input identity and current freshness, purpose and recipient permissions for that use. Keep private and task-specific material within its permitted route and scope. Cached prompts, responses, embeddings, rankings and retrieval results are disposable optimisations. Their presence does not make them source truth, permission, evidence or Package guidance. Invalidate or bypass them when inputs, policy, permissions, model behaviour or required freshness change.

Before a sequence of calls begins, set maximum calls, elapsed time and candidates, allowed retry categories, evaluation independence and a no-progress stop. Retry an observed transient failure only when another attempt can change the outcome. Deterministic rejection, invalid input, missing permission, repeated evaluator disagreement or exhaustion of the budget for resolving uncertainty requires stopping or reconsidering the work.

In the recommendation, give the chosen route, alternatives rejected, complete cost estimate, quality and risk evidence, cache and freshness conditions, fallback order, budgets, stopping triggers and remaining uncertainty. If no route is shown to meet the value requirements within budget, recommend manual handling, deferral or report unavailable.

The Package supplies the comparison method. Verify current prices, quotas, models, provider availability, cache contents, metering, billing and actions through their owning Pack, private configuration, engine or provider when used.

Keep planned limits, measured usage, invoice reconciliation and unknown or delayed charges distinct. Count every attempted provider call, including failures and retries, against the task budget. Identify who supplied rates and currency assumptions. Local elapsed time or a simulated meter does not establish billed usage. Stop before another attempt if its worst applicable cost or effect cannot fit the remaining limit.

## Budget for repeated context

Measure the input to each model call, not just the first prompt or the size of the source dataset. Within one invocation, let $S$ be the first call's input tokens, $K$ the number of model calls, and $D_j$ the tokens added after call $j$ that later calls read again. The additions include tool results, replayed assistant requests and message overhead. If the transcript only grows, with no trimming or compaction, call $k$ reads:

```math
I_k = S + \sum_{j=1}^{k-1} D_j.
```

Summing those inputs gives the invocation's input volume. An addition made after call $j$ appears in the remaining $K-j$ calls:

```math
B_{\mathrm{in}} = \sum_{k=1}^{K} I_k
= KS + \sum_{j=1}^{K-1}(K-j)D_j.
```

For an equal addition $D$ after each call, the sum and largest request are:

```math
B_{\mathrm{in}} = KS + D\frac{K(K-1)}{2},
\qquad I_{\mathrm{peak}} = S + (K-1)D.
```

Use that as an estimate when additions are only approximately equal. With $O_k$ output tokens from call $k$, total token volume is:

```math
B_{\mathrm{total}} = B_{\mathrm{in}} + \sum_{k=1}^{K} O_k.
```

An assistant's tool request legitimately counts as output when generated and as input when replayed in a later call. Do not add reasoning tokens separately if the reported output total already includes them. Compaction, retrieval changes and removed messages break the growing-transcript assumption; use the actual per-call inputs for those paths.

### Separate input size from call count

For fixed $K$ and additions bounded by $O(S)$, input volume is $\Theta(S)$. Doubling the starting input does not itself create quadratic consumption. If calls increase while each turn adds a constant $D$, the growth is $\Theta(KS + DK^2)$ as $K$ grows: repeated reading creates the quadratic term in call count. Similarly, fixed calls and rounds give $O(N)$ consumption for $N$ items only when input and tool results grow at most linearly in $N$, with bounded fields and text per item. An unrestricted history field can invalidate that bound.

These expressions count tokens. They do not predict transformer compute, latency or charges. Cache discounts can change the bill while the represented input volume stays the same; apply the actual provider's accounting separately.

For illustration, start with 12,000 input tokens and add 4,000 per turn. These are invented values, not a measured run:

| Calls | Largest input request | Input summed over all calls |
| ----- | --------------------- | --------------------------- |
| 1     | 12,000                | 12,000                      |
| 2     | 16,000                | 28,000                      |
| 3     | 20,000                | 48,000                      |
| 5     | 28,000                | 100,000                     |
| 10    | 48,000                | 300,000                     |

The three-call case reads 12,000, then 16,000, then 20,000 tokens. Its 48,000-token total is usage across calls; no request needs a 48,000-token context window.

### Include every agent and round

For $R$ executed rounds, sum each agent's actual calls rather than treating the largest context window as workflow usage:

```math
B_{\mathrm{workflow,in}} = \sum_{r=1}^{R}\sum_a\sum_{k=1}^{K_{r,a}} I_{r,a,k}.
```

Consider another illustrative round: a drafting agent reads 10,000, 12,000 and 18,000 input tokens, producing 100, 100 and 2,000 output tokens. A reviewer reads 12,000 and 16,000 input tokens, producing 100 and 500 output tokens. The five calls use 68,000 input and 2,800 output tokens, or 70,800 together; the largest input is 18,000. Three equal rounds would use 15 calls, 204,000 input and 8,400 output tokens, totalling 212,400. Multiply only when rounds really are comparable; feedback and new drafts often change their sizes.

A fresh invocation resets its transcript, not the usage already incurred. Repeating the earlier 12,000 → 16,000 → 20,000 sequence three times uses 144,000 input tokens while keeping the largest input at 20,000. This reset example is separate from the two-agent workflow.

### Remove unnecessary intermediate reads

In the drafting example, independent tool results add 2,000 and 6,000 tokens. Reading them in separate turns gives 10,000 → 12,000 → 18,000, totalling 40,000 input tokens. Obtaining both before the next model call gives 10,000 → 18,000, totalling 28,000. Reading the larger result first in a sequential path would total 44,000 instead.

Group requests when their arguments are independent and the same permitted evidence will reach the agent. The saving comes from avoiding an intermediate model call and its complete reread. A dependent request must wait for the result that determines its arguments. Keep necessary evidence and permission checks; reduce repeated reading by choosing the needed source sections and calls, not by discarding the information required for a sound answer.

## Check what a cache actually reuses

Identify the object reused: prompt-processing state, a generated answer or an action result record. Prefix reuse does not return an earlier answer. A key that prevents duplicate writes does not establish reuse of a model result. Check cache eligibility and actual inputs for the provider route, model revision, source revisions and permitted scope. Preserve instruction and source priority when arranging stable context; padding or reordering to reach an assumed cache threshold must not change the meaning.

Compare equivalent cold and warm work under the same quality criteria. State which provider counters were available, with their units and scope. Distinguish observed reuse from configuration intended to encourage it. An absent reuse counter is unknown, not measured zero. Prompt-token reuse, reduced transport, latency and billed savings each need their own evidence.

For explicitly retained context, include creation, storage duration, lookup, refresh and disposal in the cost and ownership model. Keep input, recipient, freshness and invalidation requirements effective across reuse. One opportunistic hit cannot establish persistence for the next request. Verify current provider limits, prices and result records. Choose an explicit cache when measured recurrence and coverage justify managing its lifecycle.

## Report cost for assigned work and accepted outcomes

Use the task's accounting records to connect assigned work, execution attempts, measured quantities and accepted outcomes. One case can need several attempts or end in abstention or manual handling. Allocate retries, failures and shared work once under a stated rule. Reconcile usage and acceptance records at their own keys before joining them. When analytical-grain guidance is selected, apply its join and denominator checks to the underlying table.

Report total known usage, missing coverage and cost per assigned unit. For cost per accepted outcome, use the same work population in the numerator and count only accepted outcomes in the denominator. Include the acceptance rate, manual and fallback costs, and unresolved charges. Keep a statistic for successful attempts alone separate. Zero accepted outcomes makes cost per accepted outcome undefined; incomplete metering gives only a partial numerator.

## Reserve calls for the branches that can still run

Draw the stages before promising a call budget. Count each decision, extraction, generation and evaluation job, then multiply by its permitted attempts. A shadow, a retry and a fallback all consume calls. Where they share one attempt allowance, count that shared maximum rather than multiplying independent retry settings. Check whether the client, workflow node and provider adapter can each retry; one job limit must cover their combined behaviour.

For an illustrative request-processing workflow, let each decision job allow at most $d$ provider calls and the extraction job allow at most $e$. One initial decision can stop work early; otherwise extraction and another decision identify requests, followed by two decision jobs for each of $T$ branches. Reserve:

```math
C_{max}=2d+e+2dT.
```

With one attempt per job, this is $3+2T$. With at most two attempts for decision jobs and one for extraction, it is $5+4T$. Pairing both kinds of job makes it $6+4T$. An early stop needs only the initial job's allowance, $d$. The formula belongs to this stage graph; derive another graph's bound from its own jobs rather than reusing the expression by name.

After the initial jobs finish, replace their worst-case allowance with their observed attempt count $U$: reserve $U+2dT$ for the completed and remaining work together. With $d=2$, two branches and three observed initial calls, the total reservation is eleven; five initial calls make it thirteen. Missing attempt evidence requires the original worst-case bound. Under an illustrative twelve-call cap, the first case fits and the second must hold before another call. Do not drop a required branch to make the estimate fit. A reservation is capacity, not usage; report the attempts that actually ran separately.

A call ceiling also needs token, time and money limits. A long-context fallback can cost more than several short calls. Include the maximum permitted request and response for each remaining attempt, and reserve any paid evaluation independently of the production answer. A result-cache hit may remove a model call only when reuse is valid and observed; prompt-prefix reuse still makes a call. Reconcile the final usage with the same task and attempt identities used to reserve it.

## Route by the job and evidence it needs

For a lookup, first use the authoritative index or deterministic query if it answers the question. A small model can help identify likely passages or extract a constrained field when that work is actually needed. Check source support, completeness and permitted output on representative cases before choosing it. Use a larger model for a named unresolved interpretation only when the additional call can improve the required outcome within budget. The router's own inference, verification and escalations belong in the comparison.

For example, compare a small extractor followed by deterministic validation and selective review with a larger model handling every case. Keep the same inputs, acceptance criteria and permitted data routes. Include cases with several requests, missing evidence and malformed output. Count rejected extractions, review time and escalated calls in the small route's total. An exact lookup needs no model merely to fit the routing design, and an incomplete answer needs its cause resolved rather than automatic escalation until some model agrees.

Choose the answer form as well as the model: a fixed decision head may suit a stated proposition, while an extractor must return the requested fields and source evidence. Availability fallback and quality escalation answer different problems. Declare which failures permit another provider, which answer can continue the task and the shared attempt ceiling. A shadow supports comparison; its existence does not make it an approved replacement answer. Recheck the complete routed path against an independent reference before treating its lower cost as an improvement.

## Before handing off

- [ ] Were the same tasks and conditions compared across routes?
- [ ] Does the comparison include context, retries, latency and failure costs?
- [ ] Is it clear which route fits and where its limits remain?
