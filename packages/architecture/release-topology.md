# Choose how to release, observe and recover

Plan how an immutable candidate reaches its operating environment, how exposure grows, what operators observe and how they stop or recover. This plan does not establish readiness or perform deployment.

Identify the release, environments, users and traffic segments, dependencies, stateful components, compatibility window, tolerated failures, operator and decision-maker. Show which versions may coexist and which transitions must happen in order or cannot be undone.

Choose the rollout to fit the risk. Rolling replacement reduces duplicate infrastructure but requires old and new versions to work together. Blue–green keeps a complete environment to return to, but data reconciliation and traffic switching still need testing. A canary limits exposure only if assignment, comparisons, monitoring and abort thresholds are trustworthy. Feature controls can narrow behaviour; they cannot undo schema changes or external effects already performed.

Set checks for build identity, artifact integrity, configuration, migration, health, traffic entry, observation and completion. Liveness means a process responds. Readiness must exercise the dependencies and state required for the intended workload. Monitoring needs both thresholds and a person or system responsible for responding.

In the release plan, give each stage its entry requirements, maximum exposure and duration, success and abort signals, reconciliation and next owner. Choose rollback or forward repair before crossing an irreversible point. Explain what happens to in-flight work and partially written state.

| If you are doing this | Read this |
| --- | --- |
| Check startup, readiness, cleanup or shutdown. | [Check who owns a running process](release/process-ownership.md) |
| Test independent profiles, provider loss or whether a consumer adopted the release. | [Check standalone and integrated operation](release/integrated-profiles.md) |
| <a id="read-back-the-exact-release"></a>Check a deployed artifact, a forward rollback or a persisted result. | [Read back the release and check the recovery path](release/observe-and-recover.md#read-back-the-exact-release) |
| Check measurements or commands at the device. | [Check field-device paths in the environment they use](release/field-devices.md) |

## Before handing off

- [ ] Name the release, exposure, compatibility window and allowed duration.
- [ ] Check readiness, deployed state and the reason to stop or widen exposure.
- [ ] Check the return or forward repair path before an irreversible change.
