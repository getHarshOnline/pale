+++
description = "Check trace identities, dataset membership and score targets."
see_also = [{ scope = "local", resource_id = "runs" }, { scope = "local", resource_id = "langfuse-tools" }]
official_sources = [{ label = "Langfuse official agent skill", url = "https://github.com/langfuse/skills/tree/f0bcb13088b490c04b3f4870bace17f6d57b199c/skills/langfuse" }, { label = "Langfuse native CLI", url = "https://langfuse.com/docs/api-and-data-platform/features/cli" }, { label = "langfuse native documentation", url = "https://langfuse.com/docs/evaluation/experiments/datasets" }, { label = "langfuse code evaluator runtime", url = "https://langfuse.com/docs/evaluation/evaluation-methods/code-evaluators" }, { label = "langfuse model judge execution", url = "https://langfuse.com/docs/evaluation/evaluation-methods/llm-as-a-judge" }]

[guide]


[stack]
uses = [{ scope = "local", resource_id = "langfuse-tools" }]
+++

# Check trace identities, datasets and scores

For a Langfuse task, use the [official skill and native tool guidance](langfuse-tools.md) to find the selected API and workflow. This optional reference does not select Langfuse for another backend or start additional evaluation work.

For the selected trace and dataset backend, identify the exact trace and observation keys, dataset version and scoring configuration. Keep exports connected across delivery retries without creating another business attempt. Verify dataset membership and case identity before scoring. Separate deterministic identity and schema checks, model judgment and human adjudication. Use a known-bad control and independent review for consequential interpretive claims.

## Bind observation identities before the first export

Identify the receiving project and the scope within which it indexes identifiers. Carry trace context across services and assign one exporter to each observation. Preserve the mapping from original run and attempt to trace, observation and parent keys. Verify that distinct observations remain distinct at the destination, including repeated step or provider-attempt numbers in different runs or producers. A distinct trace ID alone may not suffice. Check the transport's ID format and the key actually used to retrieve the result.

Fix the observation map and exact export bytes before first delivery. If IDs depend on sorted position, fix membership as well as order: adding an earlier item could otherwise change what an old ID means. Record delivery attempts separately and replay only the retained export. A deliberate correction or backfill needs its own accepted mapping and history. Keep original score targets and supporting evidence inspectable.

Test two runs with repeated step names, reordered source rows, replay of the fixed export and a changed body offered under the same observation ID. Verify parent links, one exporter per observation and distinct retrieved records for both runs. Reconcile partial acknowledgements using original keys. A transport receipt establishes delivery; the originating system owns the result and any actions.

[OpenTelemetry trace context](https://opentelemetry.io/docs/specs/otel/trace/api/#spancontext) and [Langfuse distributed tracing](https://langfuse.com/docs/observability/features/trace-ids-and-distributed-tracing) define the native context and ID shapes. Confirm write and read semantics against the selected backend and configuration.

## Map scoring to the exact observation

Choose one score writer per check and run. Before claiming agreement, identify the trace or observation scored by each human or automated evaluator. Supply evidence the evaluator can actually read. Belonging to a trace does not establish that a judge can see sibling or child observations. State the score type and question; exclude evaluator output from the live selection that determines its own inputs. Distinguish captured annotations from corrected references accepted into a versioned dataset. Identify item selection and schema revision separately.

Before pairing human and automated scores, match the exact case and assessment, question and rubric revision, trace or observation target, and evidence visible to each evaluator. If one scores a trace and another an observation, require an explicit mapping and verify that they answer the same question using equivalent evidence. Preserve each score's original target and writer. Show missing, held and inapplicable results outside quality aggregates.

Test a matching pair, a score attached to the wrong target, missing required evidence and a repeated write of the same score. Verify that only eligible pairs enter agreement reporting and that a repeated write adds no vote.

## Choose where the evaluator runs

List the evidence, dependencies, provider route, execution limits and budget the check needs. Check that the selected backend and dispatcher can supply them. For hosted code checks, place required evidence in the permitted observation or experiment context. Langfuse code evaluators run without network egress. A missing prerequisite leaves the check unavailable or held under its rubric.

For hosted model judges, verify the worker's actual model connection, retry controls and spending limits separately from the application's production adapter. Use a separately authorised evaluator when the hosted route cannot meet evidence or provider requirements. An external evaluator writes to the same accepted case and question. Verify the target and designated writer before enabling rules on either route.

Test a complete case, one missing required evidence and a route unable to meet provider or budget policy. Inspect which checks ran and which scores were written. A trace exporter may retry delivery of an existing result without rerunning inference or business processing.

## Worked example

An evaluation looks worse after a prompt change. Find traces for the same dataset items, check prompt and evaluator versions, then inspect the failed items' actual observations. Compare like inputs before attributing the difference to the prompt.

## Before handing off

- [ ] Find the trace, dataset or run used for the claim.
- [ ] Check its inputs, version and linkage to the observed output.
- [ ] Keep unrelated or private project data out of the result.
