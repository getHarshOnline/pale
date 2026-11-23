# Make decision notebooks reproducible

Before analysis, state the question, what one observation represents, time and eligibility boundaries, outcome definition and comparison. Use only data available at decision time. Keep related groups or time periods apart when random row splits would leak information. Fit preprocessing, cutoffs and calibration on training data or a separate validation split. Keep the final holdout untouched until the comparison is fixed.

Compute a simple baseline alongside the candidates. Report case counts, coverage, deferrals, reliability, uncertainty and important groups where the method fails. To check probability calibration, compare mean predicted probability with the observed positive rate in each bin. Show counts and uncertainty, especially for sparse bins. Explain when a strong score mainly reflects the mix of outcomes. Distinguish prediction under observed decisions, an estimate of what would have happened under another decision, and a measured intervention. State assumptions and unobserved alternatives. In a causal investigation, establish the symptom, cause and proposed recovery separately.

Put a concise question and interpretation around each computed table or chart, with readable units and labels. Run the notebook from a clean kernel, top to bottom, with exact inputs and an identified environment. Retain the executed notebook and keep any shareable derivative separate from restricted outputs. If execution cannot be reproduced or the holdout was reused, report the failure and narrow the claim.

## Make the reasoning inspectable

For each substantive section, state the question, compute the result from the declared inputs, show an accessible table or figure where useful, then explain the consequence and its limits. Derive counts and claims from the run. Check that exported figures retain their units, labels and source context. Tie historical results to their original runs and preserve original notebooks and outputs separately from reruns.

## Choose a guide

| If you are doing this | Read this |
| --- | --- |
| Run or export a Jupyter notebook | [Notebook execution and artifact checks](jupyter-practice.md) |
| Check that two runs used comparable inputs | [Evaluation lineage and comparability](lineage.md) |

## Worked example

A notebook compares ways to predict a second visit. Put each person's records in one split and fix the evaluation window before fitting. Show the overall visit rate beside candidate scores. For each probability bin, compute its count, mean prediction and observed visit rate from the same run. Show uncertainty for sparse bins. If a clean run fails because the input file is missing, report that failure and identify the saved output as an earlier result.

## Before handing off

- [ ] Run from clean state using the same inputs and split.
- [ ] Check for leakage, order dependence and missing artifacts.
- [ ] Report what can be reproduced from the delivered notebook.
