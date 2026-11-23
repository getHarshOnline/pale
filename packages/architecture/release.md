+++
description = "Plan release stages, exposure, observation, compatibility and recovery before widening use."
see_also = [{ scope = "local", resource_id = "release/processes" }, { scope = "local", resource_id = "release/integrations" }, { scope = "local", resource_id = "release/read-back" }, { scope = "local", resource_id = "release/devices" }]

[guide]

[task]
uses = [{ scope = "local", resource_id = "release-materials" }]
outputs = ["In the release plan: stages, checkpoints and responsible operators.", "Readiness, inspection and recovery criteria."]
+++

# Choose how to release, observe and recover

Plan how an immutable candidate reaches its operating environment, how exposure grows, what operators observe and how they stop or recover. This plan does not establish readiness or perform deployment.

Identify the release, environments, users and traffic segments, dependencies, stateful components, compatibility window, tolerated failures, operator and decision-maker. Show which versions may coexist and which transitions must happen in order or cannot be undone.

Choose the rollout to fit the risk. Rolling replacement reduces duplicate infrastructure but requires old and new versions to work together. Blue–green keeps a complete environment to return to, but data reconciliation and traffic switching still need testing. A canary limits exposure only if assignment, comparisons, monitoring and abort thresholds are trustworthy. Feature controls can narrow behaviour; they cannot undo schema changes or external effects already performed.

Set checks for build identity, artifact integrity, configuration, migration, health, traffic entry, observation and completion. Liveness means a process responds. Readiness must exercise the dependencies and state required for the intended workload. Monitoring needs both thresholds and a person or system responsible for responding.

Follow the candidate through the pipeline: reviewed source, integrated revision, built artifact and deployed instance. A merge may create a different revision from the one reviewed; check that difference and the build used for deployment. Before dispatch, confirm that the target's starting state and the approval still match the proposed action. A queued deployment is pending until its owner reports completion and the served result is inspected. Keep observing within the agreed task; use the recovery plan if a required check fails instead of widening exposure because the submission succeeded.

In the release plan, give each stage its entry requirements, maximum exposure and duration, success and abort signals, reconciliation and next owner. Choose rollback or forward repair before crossing an irreversible point. Explain what happens to in-flight work and partially written state.

| If you are doing this | Read this |
| --- | --- |
| Check startup, readiness, cleanup or shutdown. | [Check who owns a running process](release/processes.md) |
| Test independent profiles, provider loss or whether a consumer adopted the release. | [Check standalone and integrated operation](release/integrations.md) |
| <a id="read-back-the-exact-release"></a>Check a deployed artifact, a forward rollback or a persisted result. | [Read back the release and check the recovery path](release/read-back.md#read-back-the-exact-release) |
| Check measurements or commands at the device. | [Check field-device paths in the environment they use](release/devices.md) |

Use the version-control method when this rollout requires repository operations. Return the plan for its owner to review. The responsible system and operator still perform and verify release and recovery; the plan supplies no approval to do so.

## Before handing off

- [ ] Name the artifact, exposure and compatibility window.
- [ ] Set readiness, stop and expansion conditions.
- [ ] Check the return or forward repair path.
