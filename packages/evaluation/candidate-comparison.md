# Compare changes separately and together

Name the decision and its owner. Fix the current candidate, the components being changed and the inputs held constant. Before inspecting outcomes, record the eligible cases, primary measures, blocking criteria, uncertainty method, budget and stopping conditions in the study's protocol. Check that protected evidence is suitable and permitted for this comparison.

For one change, compare the new candidate with the current one while holding everything else fixed. For two changes, use matched cases in four configurations: neither change, each change alone and both together. Identify the exact component versions and settings for each. Keep cases, budgets, measurements and the rest of the environment comparable. Report the difference for each single change and for the joint change. When subtraction is meaningful for the measure, compare the joint difference with the sum of the two single differences, including uncertainty. Show missing or invalid configurations. Comparing only the original and joint configurations cannot attribute the difference to either component or their interaction.

Identify who or what generated candidates and rewards, who evaluated them, and any models, data, prompts or reviewers those steps share. If a generator or reward scorer supplies acceptance evidence, require a separately tested check using protected evidence. Renaming a scorer, calling it again on the same tuned cases or copying its judgment adds no independence. Keep generation scores, independent observations, unresolved criteria and the owner's decision distinct. A strong aggregate or joint score cannot settle a blocking failure or replace a required check.

## Bind what a simulation comparison can establish

Before running a set of simulations, state the quantity and unit being estimated. Identify the simulator, provider or model revision, scenario, origins of parameters, sampling and seed schedule, and execution and replay mode. Keep cached replays distinct from fresh draws. Say which runs share a scenario, seed, model or input, and account for that dependence or pairing in the comparison.

Separate three claims. Conditional variation describes reruns of the selected scenario and simulator. Sensitivity describes the observed response to specified input changes. An empirical claim needs observations from the stated population or environment, a justified sampling or validation design and an applicable uncertainty measure. More seeds can improve coverage of that simulator under those settings. Claims about a population, prediction or cause each need independent evidence for their particular target.

For sensitivity, fix the baseline, perturbations, method, budget for valid runs and observed coverage. Use actual runs to claim how the selected simulator responds. Label a surrogate's result as a separate estimate and check it against matched actual runs. Changing one factor at a time tests only those contrasts. Test matched joint changes when interaction matters to the decision. Report missing or failed configurations, the observed range and the uncertainty method, including its resampling unit and assumptions. Keep the owner's decision separate from the comparison.

## Match forecasts to eligible observations

Fix the issued forecast, subject and population, target quantity and unit, forecast origin and information cutoff, time basis, target window or deadline, and model, policy and input revisions. In historical replay, reconstruct only information available at each forecast origin, including reporting delays and later revisions. Give the benchmark the same information and target. Distinguish fitted residuals, issued forecasts and retrospective reconstructions.

Before scoring, specify the observation source, eligibility, interval endpoints and rule for selecting a revision. Distinguish when an event happened from when it was reported, captured or revised. Check both endpoints and what is counted. Equal-length intervals with different origins can describe different outcomes. State tolerances, whether time means civil or elapsed time, and how partial windows are handled. Show shifted, shorter or longer windows as diagnostics. Rescaling needs a separately defined and validated target or transformation.

Keep pending, censored, revised, conflicting and missing outcomes distinct from observed zero. Preserve valid negative net changes. For time-to-event and probability forecasts, define the observation horizon and censoring rule before treating an event as absent. Score the same eligible cases for every candidate and baseline. Report coverage by horizon and relevant group, with reasons for exclusions and dependence caused by repeated subjects or overlapping windows.

Choose a score suited to the target, forecast form and units. For point forecasts, show the sign convention for error and the absolute or squared errors. Percentage error needs a meaningful nonzero denominator. With no eligible cases, error is undefined. Test a matching interval, a shifted interval of equal length, a late input unavailable when the forecast was issued, a revised observation and a valid zero. Report forecast accuracy separately from the effect of a decision made using it.

For point scores, probabilistic scores and rolling-origin evaluation, see [Forecasting: Principles and Practice](https://otexts.com/fpp3/toolbox.html).

## Compare results when the evaluator changes

Keep each historical observation with its evaluator, rubric, metric, schema, cases and available evidence. Establish a separate comparison for a shared question. Match cases by identity and revision; fix the candidate outputs, independent references and scoring policies. Check whether units, criterion meanings, thresholds and rules for missing cases permit the comparison. The same score label or numeric range does not establish the same measure.

Hold the evaluator and rubric fixed to compare candidate outputs. Hold outputs fixed to compare evaluators against an independent reference. If both change and their interaction matters, score both candidates with both evaluators and compare within each fixed configuration. Compute a joint difference only on a justified common measure. Retain each evaluator's own scores and findings beside the comparison.

Test a changed rubric with the same score label, changed case coverage and an unavailable historical output. Keep the original denominator and disclose incomplete pairs. Limit a matched-subset result to that subset. If the common measure, coverage or replay inputs remain unresolved, retain the original findings and leave the broader comparison open.

## Verify what could influence the candidate

Fix the protected case set and decide which related cases must stay together to prevent leakage for this claim. Group repeated captures and derived variants by source, subject, time or other shared origin as needed. A new row identity, digest or seed does not make a related case independent. Keep the owner's membership and rights evidence with the declared split.

Before calling a score held out, compare that set with what the candidate, its selector and its evaluator actually saw. Inspect the inputs used in execution: training and tuning data, prompt examples, retrieval indexes and caches, rubric calibration, threshold and checkpoint selection, and results used to make further changes. Include earlier attempts whose feedback influenced the selected candidate. A declared split without this inspection leaves actual exposure unknown.

Test the leakage check with a disguised duplicate, a transformed descendant and a protected result reused for selection. If feedback influenced selection, describe that result as tuning evidence and use a separate protected set for an independent claim. If upstream exposure is unknown, limit the claim to the inspected pipeline and name the gap. In the comparison result, identify the split, overlaps, unresolved exposure paths and evidence needed for an independent comparison. The owner controls data permissions and acceptance for later use.

## Check an intervention claim

Distinguish a proxy signal, a measured difference between intervention and control, and an identified causal effect. State the intervention and control, assignment and analysis units, outcome, target population or environment, and the effect being estimated. Fix the task, data and split, artifact versions, policy, configuration, intervention timing, seed schedule, repeat count and budget. Preserve the signed difference and uncertainty before converting results into scores or credit.

Explain what allows the design to identify the effect, including relevant assumptions about confounding, assignment, measurement and interference. Show how the baseline and counterfactual were constructed, what stayed fixed and how leakage and contamination were checked. Exercise a sham or negative control and plausible rival explanations where the design permits. Name the uncertainty method and its independent or paired unit. Before interpreting multiple tests, declare the family of comparisons, error criterion and any correction. Show missing configurations and failed controls beside the estimate.

Test a bare difference after removing a component, a proxy, a significant result with an unresolved identification assumption, and a result whose sign changes under a plausible alternative specification. Label each with the strength of evidence it actually provides. Evidence for A to B and B to C leaves A to C unresolved. A closure or ranking calculation must show its assumptions and inferred edges separately from direct observations.

Return the measured differences, supported claim and scope, findings about controls and assumptions, uncertainty, limitations and exact evidence needed for a stronger claim. The study owner defines the protocol, acceptance thresholds, formal meaning and permitted downstream uses.
