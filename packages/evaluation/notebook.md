+++
description = "Run a notebook from clean state and check for leaked information."
see_also = [{ scope = "local", resource_id = "jupyter" }, { scope = "local", resource_id = "runs" }]

[guide]

+++

# Make decision notebooks reproducible

Before analysis, state the question, what one observation represents, time and eligibility boundaries, outcome definition and comparison. Use only data available at decision time. Keep related groups or time periods apart when random row splits would leak information. Fit preprocessing, cutoffs and calibration on training data or a separate validation split. Keep the final holdout untouched until the comparison is fixed.

Compute a simple baseline alongside the candidates. Report case counts, coverage, deferrals, reliability, uncertainty and important groups where the method fails. To check probability calibration, compare mean predicted probability with the observed positive rate in each bin. Show counts and uncertainty, especially for sparse bins. Explain when a strong score mainly reflects the mix of outcomes. Distinguish prediction under observed decisions, an estimate of what would have happened under another decision, and a measured intervention. State assumptions and unobserved alternatives. In a causal investigation, establish the symptom, cause and proposed recovery separately.

Put a concise question and interpretation around each computed table or chart, with readable units and labels. Run the notebook from a clean kernel, top to bottom, with exact inputs and an identified environment. Retain the executed notebook and keep any shareable derivative separate from restricted outputs. If execution cannot be reproduced or the holdout was reused, report the failure and narrow the claim.

## Make the reasoning inspectable

For each substantive section, state the question, compute the result from the declared inputs, show an accessible table or figure where useful, then explain the consequence and its limits. Compute the analysis directly from the original exports or other declared source data; another report can be a comparison, but repeating its conclusions does not produce an independent analysis. Read the observations and their context, not just their summary labels. Derive counts and claims from the run. Check that exported figures retain their units, labels and source context. Tie historical results to their original runs and preserve original notebooks and outputs separately from reruns.

Keep the overall evaluation table when adding a closer view. Put the important findings near that overview, with detail available for investigating a poor result. For a forecast, show how far it missed the observed value, not only whether it passed. Give the error in meaningful units and, where comparisons across different scales need it, a normalised value with its denominator stated. Show the evaluated population and baseline so the reader can see which deviations matter and where an average hides them.

Make the decision visible in each relevant chart and table. A lightly shaded desirable region, a band around a target or an uncertainty envelope can show what the values mean; label which of those it is and the rule or calculation behind it. Mark consequential deviations against the baseline, and use conditional formatting with readable labels in tables. Preserve the underlying values and uncertainty. Apply these cues wherever they answer the reader's question, including expanded detail, without crowding the view or inventing a threshold just to colour it.

## Choose a guide

| If you are doing this | Read this |
| --- | --- |
| Run or export a Jupyter notebook | [Notebook execution and artifact checks](jupyter.md) |
| Check that two runs used comparable inputs | [Evaluation lineage and comparability](runs.md) |

## Worked example

A notebook compares ways to predict a second visit. Put each person's records in one split and fix the evaluation window before fitting. Show the overall visit rate beside candidate scores. For each probability bin, compute its count, mean prediction and observed visit rate from the same run. Show uncertainty for sparse bins. If a clean run fails because the input file is missing, report that failure and identify the saved output as an earlier result.

## Before handing off

- [ ] Run from clean state using the same inputs and split.
- [ ] Check for leakage, order dependence and missing artifacts.
- [ ] Report what can be reproduced from the delivered notebook.
