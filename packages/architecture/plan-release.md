# Plan a release

Identify what will be released, where it will run, the rollout stages, the changes permitted and the operator responsible. Describe the starting state, health and readiness checks, how to inspect the result, when to abort and who will recover the system.

Use the version-control method when this rollout requires repository operations. Return the plan for its owner to review, including compatibility, conditions for expanding exposure and a rollback or forward-repair path. The responsible system and operator still perform and verify release and recovery; the plan supplies no approval to do so.

## Before handing off

- [ ] Name the artifact, exposure and compatibility window.
- [ ] Set readiness, stop and expansion conditions.
- [ ] Check the return or forward repair path.
