# Compare models with the same resource budget

## Define the claim and comparison

State the claim and decision it could change. Fix the cases, input and tool access, scoring rule, minimum required capability, baseline and alternatives before inspecting results. Choose a strong current method and a control that removes the mechanism being credited. A route that misses a critical capability requirement is ineligible even if it costs less.

Say which resource is held equal, which have hard limits and which are measured trade-offs. Equal elapsed time, compute, peak resident memory and spending answer different questions. If one match cannot settle the claim, compare across declared budgets and report where one route becomes preferable. Avoid substituting an unmeasured budget or arbitrary weighted sum. Separate training and tuning effort from per-case inference and explain any reuse or amortisation.

## Count the complete route

Measure every alternative over the same parts of the system. Include coordination, communication, state movement, retries, tools, evaluators, fallback and all attempted cases where they belong to the route. When parallelism changes their relationship, distinguish total work from the time taken by the longest dependent sequence. Track capacity, peak resources and dropped or invalid outputs alongside quality. A required resource that was not measured leaves the comparison unresolved.

Use paired cases and stated repetitions where feasible. Compare quality differences on eligible work, critical subgroups and the resource levels where the result changes. To attribute a gain to routing, memory or another mechanism, vary that mechanism against a matched control. Whole-system improvement leaves its cause open. In the comparison, identify deviations and limit claims when alternatives differ in exposure, workload, available resources or measurement.

Return the fixed comparison conditions, all attempted alternatives, which met capability requirements and resource limits, comparable differences and uncertainty, and the baseline to retain if the proposal is worse or unresolved. The owning system runs the experiment, and its owner decides adoption.

## Distinguish a direct contribution from choosing a route

Identify where a component can change selection, routing weights, timing or fallback, and where its value can enter the final computation. Define the observable and where it is read. Inspect the executed path, including selected identities, order, weights and downstream conditions relevant to the claim. An activation, intermediate result or available connection establishes only what was observed.

To test direct contribution, hold decisions and other inputs fixed, then vary or neutralise the value where it enters the output calculation. Use a permitted fixture that the system can actually reach, and state which choices were held constant. Fixing the chosen branch is insufficient if weights or fallback still change the result. Compare with a selection-only route, where the same signal chooses or weights a branch but contributes no direct output value. Include a sham and a sensitive positive control so rounded or saturated output cannot make a working connection appear absent.

Report the connection, the observed output difference under fixed choices, and whole-route quality and cost separately. A forced internal choice establishes that diagnostic under its tested conditions. A claim of improvement in the real route needs a matched end-to-end comparison. A claim about an external causal mechanism needs its own identification evidence. If route or readout cannot be held comparable, describe their coupling and limit attribution to the contrast actually tested.

## Worked example

Compare a direct classifier with a router that calls the same classifier or a larger model. Fix the cases, minimum quality on the hardest subgroup and maximum spending. Charge the router for its routing call and fallback attempts. Report paired quality, every attempted case and time to a usable result. To test the routing decision itself, use a control with the same available models and budget but a fixed assignment rule.
