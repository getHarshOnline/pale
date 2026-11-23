+++
description = "Assess whether a release is ready for its intended users, operating conditions and rollout."
uses = [{ scope = "local", resource_id = "review" }]

[guide]

[task]
uses = [{ scope = "external", resource = { package_id = "pale.research", resource_id = "evidence" } }]
outputs = ["In the release assessment: ship, ship with conditions, block or unresolved, with remaining risks, evidence and missing checks."]
+++

# Assess whether a release is ready for its intended use

Assess one release under stated operating conditions. A build result or code-quality score answers only part of that question. Use the research method when readiness depends on documentation, standards or external facts. The assessment is read-only; fixes, deployment, rollback and approval are separately authorised actions. Run a project check, scanner or browser inspection only through a tool permitted for that scope.

## State the release and the rollout being assessed

Identify the release unit and version, target environment, intended users and traffic, data sensitivity, change window, dependencies, responsible operator, decision-maker and acceptable impact. State the decision deadline and proposed rollout. Readiness for a small internal canary can differ from readiness for a broad public launch.

Turn readiness into claims with explicit acceptance conditions. Depending on the release, cover functionality, security and privacy, data migration and integrity, reliability and capacity, observability, dependency health, support ownership, release integrity, recovery and compliance. Match each claim's evidence to the release revision and operating scope. Explain which useful properties a local fixture or another revision establishes and which target conditions still need checking.

## Say what kind of evidence supports each claim

In the release assessment, distinguish:

- **Inspected:** configuration, code, documentation or an artifact was read.
- **Executed:** a named check ran against a stated version and environment.
- **Observed:** behavior was witnessed under stated conditions.
- **Read back:** saved or deployed state was retrieved again from the system that owns it.
- **Inferred:** a conclusion follows from stated evidence but was not directly observed.
- **Stale:** the version, environment or age of earlier evidence no longer supports the claim.
- **Missing:** required evidence was unavailable, outside the scope or never produced.

Check whether a control works in the target conditions. Verification asks whether implementation meets the specified requirement; validation asks whether the released system is fit for its intended use. In the assessment's evidence notes, give the commands, versions, environments, dates, results and limits needed to repeat the checks. Keep the result to what those checks establish.

## Assess the risks and the controls that address them

For each material failure, identify the affected asset or user, trigger, consequence, reasons for its estimated likelihood, ability to detect it, reversibility, recovery path and time, preventive control, detection method, response owner and remaining risk. Prioritise consequence, then evidence about likelihood, detection before harm, reversibility and recovery within the tolerated time. Use numbers when they have support; unexplained arithmetic adds no objectivity.

Give particular attention to irreversible loss, unsafe behavior, security exposure, corrupted state, regulatory breach and recovery beyond the allowed time. Challenge common false assurances: tests may omit deployment behavior; a scanner may be optional, misconfigured or pointed at the wrong artifact; a rollback script may fail after an irreversible migration; monitoring needs someone responsible for responding; and a third-party status page leaves this release's dependency behavior to check.

## Define what can proceed and under which conditions

Classify findings by their consequence for this rollout:

- A **blocking condition** must be resolved because no workable limit on the rollout makes the remaining risk acceptable.
- A **release condition** permits a narrower rollout with a named owner, check, deadline, operating limit, monitoring signal and abort threshold.
- A **follow-up** can wait only when deferring it leaves the readiness claims valid.

Match the rollout to the risk: a limited audience, staged exposure, canary, feature control, rate limit, shadow run or reversible migration may help. Define when to roll back or fix forward, the latest safe decision point, how to reconcile state after reversing, and who can initiate it. An irreversible change needs stronger prevention, rehearsal, recovery and authorisation.

## Give a recommendation its owner can act on

Return the exact release and operating scope, evidence, material risks, missing checks, blockers, conditions for a narrower rollout, and prioritised fixes with owners. Recommend one of:

- **Ship:** required conditions have supporting evidence and remaining risk is within tolerance.
- **Ship with conditions:** evidence supports a stated narrower rollout with owners and checks for each condition.
- **Block:** evidence establishes a condition that prevents this rollout. Keep other unresolved claims visible separately.
- **Unresolved:** missing, stale or inconclusive evidence prevents a recommendation. Name the observation needed and leave approval of the proposed rollout open.

Finish when every readiness claim and material risk has a clear result the decision-maker can use. Keep the owner's actual release decision separate from the recommendation. Dated results and release facts stay with their source system or a Pack pinned to the assessed version.

## Before handing off

- [ ] Does the assessment identify the exact release, operating conditions and proposed rollout?
- [ ] Do the evidence and recovery checks cover the risks that matter to that rollout?
- [ ] Are blockers, narrower release conditions and missing observations distinguished, with responsible owners?
- [ ] Is the recommendation supported by the checks, with the actual release decision left to its owner?
