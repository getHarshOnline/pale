# Check trace coverage, sampling and review costs

## Separate coverage, sampling and spending

Keep every assigned case in the fixed comparison, including failures, abstentions, exclusions and incomplete pairs. An absent result has no score. Draw representative routine samples from a defined eligible population and label diagnostic samples that deliberately include extra failures. Retain the selection rule, denominator and uncertainty. Set spending and attempt caps for paid scoring, including retries. Retaining traces, selecting them for review, accepting them into a dataset and permitting scoring are separate decisions.

### Check which trace fields reach each destination

Define the trace purpose, permitted fields and data classifications, tenant or controller, producing attempt, each destination and viewer, and retention rule. Distinguish routine metadata, diagnostic captures and evaluation fixtures. Sampling or request failure gives no permission to send more data. Prefer correlation identifiers and approved measurements to raw prompts, responses, source rows and tool arguments. Assess whether a digest itself may disclose information or link records.

Use invented canary values in an isolated run. Place them in nested tool arguments, provider errors and other fields the selected paths can carry. Test a routine success, sampled failure and queued export retried after delivery fails. Inspect the serialized payload before export and the retained and viewer-facing records in scope. Check indexes, queues and fallback logs written along the path. Verify that excluded values cannot reappear through formatting, truncation, exception handling or later export. Keep enough permitted correlation to connect case, attempt and outcome without recovering excluded inputs.

Return the policy, paths, observed fields and destinations that remain unchecked. Distinguish collection, disclosure, retention and removal. A local fixture establishes only the transformation and checks it exercises. Hosted storage, tenant access and deletion each require evidence from the system that performs them.

### Verify trace parentage within the admitted scope

Identify the controller or account, trace or run, observation and parent using keys actually enforced at the origin and destination. Check whether a run identifier can recur under different accounts and whether several events describe one run. A stored parent value proposes a relationship; resolve it within the permitted namespace. Distinguish explicit roots, late arrivals, missing or ambiguous parents, cycles and references outside scope. A cross-trace link is separate from structural parentage.

Test a valid root and child, a late or missing parent, self-parentage, a longer cycle and the same identifier under another account. Compare the exact chains in the source system and evaluator input. An unresolved parent leaves the chain incomplete and blocks claims that depend on it. Check the caller's permissions separately from the selected account. A filtered response establishes membership under that filter; authorisation needs evidence from where access is enforced, including a test with a mismatched caller and account selector.

### Reserve review capacity before delivery

Keep the recovery journal complete within the agreed retention. Before making a repeatable selection, define the eligible population, stable sampling identity, policy revision, capacity scope and time window. Save the selection and reserve capacity before delivery using the owning system's concurrency rules. Retry from that saved decision and reservation, even after the window changes. Reconcile an uncertain delivery with the destination before retrying. An older trace without a saved selection needs an explicit backfill decision. Keep a selected trace's root and required attempts together.

Report eligible, sampled, reserved, delivered, completed and accepted cases separately. State whether failed and uncertain deliveries consume capacity. A cap can reduce actual coverage and favour earlier arrivals. Measure coverage by the groups and time windows relevant to the claim. Test a sampled case with and without remaining capacity, a retry after the window changes, conflicting history and a trace without a review reservation. Verify that the review cap leaves independently authorised trace export intact.

Use deterministic checks for identity, schema, arithmetic and known transitions. Give a judge assessing meaning one question and the evidence needed to answer it. Reference answers need independent review and explicit acceptance into the dataset. Keep disagreements visible until resolved by the responsible reviewer. An aggregate cannot erase a blocking safety failure.

## Worked example

Sample child traces from a rerun without assuming that the sample represents every attempt. Check parents, destinations and the fields received at each. Include failed and retried calls in cost. If the review budget is exhausted, show which items still need review.
