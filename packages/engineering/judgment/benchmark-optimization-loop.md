# Compare a performance change fairly

Before changing the implementation, fix the workload, data, environment and build versions, warm-up, repetitions, metrics, correctness requirements and target. Keep those conditions with the results in the project's benchmark record so the comparison can be repeated and its sources of noise understood.

Measure the original distribution and profile the resource or path responsible for the cost. Form one causal hypothesis and change one material variable. Run the same workload again. Inspect variation and outliers, including startup, cache state, contention, transfers, memory and work shifted to another component.

Keep the change when the improvement exceeds measurement noise, preserves correctness and resource limits, and matters to the stated user or operating goal. Check that an apparent speedup has not changed the workload, hidden work, increased slow-tail risk or moved cost outside the measurement.

In the reply or existing benchmark report, give the baseline, hypothesis, changed version, comparable measurements, correctness checks, shifted resources and uncertainty. Recommend keeping, reverting or trying another experiment. Stop when the target is met, further gains flatten, measurements become unstable or the next change cannot be isolated.
