# Check who owns a running process

## Check process ownership before startup or cleanup

Identify executable, launch settings, instance, supervisor and actual process identity. Inspect startup prerequisites and the stop procedure before launching. A busy port, process name or numeric PID identifies a possible conflict, not permission to kill it. Before signalling, require a live process handle or equivalent identity tied to the owning service. Leave missing, stale or ambiguous identities alone. Recheck if a delayed action could hit a replacement process.

Distinguish readiness, cancellation, natural exit, reaping a direct child and cleaning up descendants. Specify the cooperative stop period and permitted escalation. Inspect exit and cleanup independently. Preserve the original exit result and report remaining descendants after partial or failed cleanup. Before reusing an endpoint or writable state, test repeated stop and restart against the same instance's identity.

Test a port occupied by another service, stale or reused process identity and a child exiting while descendants remain. Inspect intended signal targets and actual effects separately. Use inert signal stubs to check target selection and isolated processes you own to test cleanup. Report platform, execution-environment and supervisor limits in the launcher review, with repeatable cases in its tests. The owner supplies the launcher, permission and cleanup mechanism.

## Worked example

A replacement finds its port occupied. Verify the supervisor's live process handle before signalling. If the process belongs to another service, stop the replacement attempt and report the conflict. For the intended target, observe child exit and surviving descendants separately.
