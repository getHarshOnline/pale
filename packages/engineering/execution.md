+++
description = "Connect a claim to the actual command, inputs and structured result, including child tasks, restoration and cleanup."
+++

# Establish what actually ran

Connect the selected tool and its contract to the actual invocation. For a repository command, identify the checkout and revision or local changes, launcher, working directory, arguments and required environment. Check what the command may change and the permission for those effects.

A portable render shows the selected configuration. A machine check may resolve a tool binding before its command or prerequisites are ready. Check the actual launcher, runtime, dependencies and writable scratch location under the chosen execution rules. In the existing run result or reply, say where any failed attempt stopped.

For an invocation through an agent host, keep the tool name shown to the caller separate from the name in the native event. Match them through the native invocation identity, session, working directory and exact command or typed input. Changed arguments, extra shell actions and substitute tools belong to different observations. A claim that Pale admitted or settled the operation needs its operation records for this same invocation.

Read the host's structured result, native receipt or observed process status. Output text alone leaves success or failure unknown. Say which inputs and outputs the result covers. A setup failure leaves the unexecuted checks pending.

## Check shared-state work and recovery

Before concurrent work, identify what each task reads, writes and owns as scratch space. Independent tasks can overlap within the permitted capacity. Mutating one shared checkout, checking it and restoring it requires exclusive access for that whole sequence. Establish that access explicitly; a temporary workspace or job limit alone does not provide it.

Account for every started child before combining results. On cancellation, stop and join the work owned by this attempt using the system's recovery procedure. Retain each child's input identity, status and diagnostics in the run results. A successful sibling cannot hide a failure or missing result, and a quiet log does not establish that a process has stopped.

Read back restoration and cleanup separately from the assertions. Failed cleanup leaves validation incomplete. Clean only this attempt's resources and keep unrelated state intact. When checking an orchestrator's shared-state or recovery behaviour, exercise a failed child, interrupted work and failed restoration at the point being qualified. Reuse those results while the relevant rules remain unchanged. The native system supplies the commands, scheduling, capacity and recovery mechanics.

If Pale's route is unavailable, leave that route unexecuted. When the owner permits a direct native run, use the same candidate and conditions, then identify that route, result and artifacts in the run's own record or reply.

## Before handing off

- [ ] Does the reported result identify the actual command, inputs, source state and permitted effects?
- [ ] Did the required assertions run, with every child’s outcome and any missing result accounted for?
- [ ] Were restoration and cleanup checked separately, and are incomplete stages still visible?
