+++
description = "Check how an agent session resumes, compacts and closes through its actual host."

[task]
uses = [{ scope = "local", resource_id = "context-inputs" }]
outputs = ["In the review: host events, what each can observe or enforce, and who owns each recovery decision.", "Observed continuity results, missing context or state, and unresolved checks."]
+++

# Check how an agent session resumes compacts and closes

Identify the host and version, repository, workspace, execution path, session and accepted generation. Map the host's actual events and what each can observe or enforce. Distinguish handler output, host response, session acceptance, permission for an operation and state read back from the continuity owner. Check what ran and reached the session beyond the event name or generated hook.

## Resume from the right checkpoint

Match the accepted session to one checkpoint permitted for the current owner, scope and lease. Check its generation, identity, freshness and revocation state before loading required context. If the match is unavailable or ambiguous, use the declared refusal or permitted reduced path. A newer checkpoint from another scope is not a substitute.

## Check temporary use of shared memory

If a host swaps workspace memory into shared or global state, identify the session that may change it and the exact original state to restore. Check ownership when acquiring and releasing the lock; a second session must neither claim an occupied lock nor remove the first session's lock. Keep each recovery attempt's backup separately identifiable so another start cannot overwrite the only original copy.

In a disposable host environment, exercise overlapping sessions, interrupted startup and close, and restoration through a linked path. Inspect the link entry, its target and restored bytes separately. Check that the correct workspace receives saved learning, unrelated state stays intact and recovery material remains available after a failed restore. A successful helper call does not prove the full session restores its original state; run through the actual entry point and shutdown path.

## Keep continuity through compaction

In the review, identify which host identity and checkpoint survive compaction. Carry forward what the current scope still needs. Keep the verified checkpoint, a proposed refresh and context actually seen by the recipient distinct. Include payload size. A stale refresh must leave a newer refresh requirement in force.

## Close without repeating completed work

Reconcile active work, unresolved effects and the final checkpoint with their owners. Check that duplicate or late events neither settle the same work twice nor reopen a closed lease. Keep recoverable failure evidence in the continuity owner's records and show what the next session may restore.

Exercise fresh, resumed, compacted and closed sessions on each selected host. Include changed identity, a missing delivery channel, a stale generation and an interrupted close. Check the host's enforcement separately from the adapter's response. Return the event map, observations, missing checks and the owner of each recovery decision. This review itself does not move session state.

## Before handing off

- [ ] Has the receiving session's actual context been checked?
- [ ] Has restore or close been exercised through the applicable host?
- [ ] Are missing context and state identified for the next worker?
