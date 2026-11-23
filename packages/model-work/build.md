+++
description = "Build, evaluate, release and monitor a reproducible model with clear requirements for promotion and rollback."

[guide]

[task]
uses = [{ scope = "external", resource = { package_id = "pale.evaluation", resource_id = "runs" } }, { scope = "external", resource = { package_id = "pale.evaluation", resource_id = "compare" } }]
roles = [{ scope = "local", resource_id = "review" }]
outputs = ["In the review: findings for each stage, supporting evidence and blocked transitions.", "A recommendation for the owner, with fallback and recovery needs."]
+++

# Build, release and maintain a model reproducibly

Start from an accepted need for learning. At each stage, establish what permits or prevents moving forward: data, training, evaluation, artifact preparation, release, serving, monitoring and retirement. This guide helps judge those transitions; the actual platforms run jobs and manage environments.

## Define data and experiments

Specify the observation unit, target, prediction time, population, feature availability, data sources, permissions, quality checks, splits and leakage risks. Identify the code, configuration, dependencies, data revisions, seeds, relevant hardware and experiment. Another permitted run should be able to reconstruct the claimed result within a stated tolerance; stochastic output need not be bit-for-bit identical.

Begin training when the data requirements and baseline can disprove the proposed approach. Evaluate on held-out data that was not used to choose features, tune thresholds or select checkpoints. Declare primary metrics, critical subgroups, uncertainty method, cost and latency limits, and blocking regressions before examining results. Preserve failed trials and account for testing multiple alternatives.

## Prepare and judge a release candidate

Package the model with preprocessing, feature definitions, labels and schema identity, code/configuration/data history, numerical environment, interface requirements, evaluation evidence, limitations and fallback. The best checkpoint may differ from the latest. A model file alone leaves the candidate incomplete.

Keep the complete inference setup with the candidate identity. Changing a scaler, feature order, tokenizer, threshold or dependency makes replaying the same weights a different experiment. Before comparing scores, inspect what features were knowable at prediction time and where each split came from. A higher score obtained from later labels or a holdout used for tuning cannot support promotion. Keep the previous deployable setup and its recovery evidence separately pinned.

Promotion requires comparison with the current baseline, evidence for critical subgroups, calibration when decisions use probabilities, robustness and misuse cases proportionate to harm, serving compatibility, rollback, monitoring and acceptance by the named owner.

## Serve, monitor and decide what comes next

For serving, establish input validation, model identity, timeouts, capacity, privacy, fallback and limits on effects. Monitor data availability and drift, prediction distribution, delayed outcome quality, subgroup behaviour, latency, cost and business safeguards. Uptime alone cannot establish model health. Distinguish an alert, investigation, reduced traffic, rollback, a retraining proposal and retirement.

New data or feedback can support another candidate cycle. Changes to the active model or Package guidance need their own review and adoption. In the lifecycle review, return evidence for each transition, blocked requirements, candidate identity, deployment conditions, monitoring and rollback requirements, and the next owner decision.

An independent review identifies the first unsupported transition, its consequence and the observation that could settle it. Keep invalid controls, missing critical subgroups, uncertainty and recovery needs visible. The reviewer recommends; the accountable owner decides promotion.

## Before handing off

- [ ] Are the data, model and versions under review identifiable?
- [ ] Have evaluation, promotion and rollback requirements been checked?
- [ ] Is it clear what supports adoption and who decides?
