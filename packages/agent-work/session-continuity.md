# Check how an agent session resumes compacts and closes

Identify the host and version, repository, workspace, execution path, session and accepted generation. Map the host's actual events and what each can observe or enforce. Distinguish handler output, host response, session acceptance, permission for an operation and state read back from the continuity owner. Check what ran and reached the session beyond the event name or generated hook.

## Resume from the right checkpoint

Match the accepted session to one checkpoint permitted for the current owner, scope and lease. Check its generation, identity, freshness and revocation state before loading required context. If the match is unavailable or ambiguous, use the declared refusal or permitted reduced path. A newer checkpoint from another scope is not a substitute.

## Keep continuity through compaction

In the review, identify which host identity and checkpoint survive compaction. Carry forward what the current scope still needs. Keep the verified checkpoint, a proposed refresh and context actually seen by the recipient distinct. Include payload size. A stale refresh must leave a newer refresh requirement in force.

## Close without repeating completed work

Reconcile active work, unresolved effects and the final checkpoint with their owners. Check that duplicate or late events neither settle the same work twice nor reopen a closed lease. Keep recoverable failure evidence in the continuity owner's records and show what the next session may restore.

Exercise fresh, resumed, compacted and closed sessions on each selected host. Include changed identity, a missing delivery channel, a stale generation and an interrupted close. Check the host's enforcement separately from the adapter's response. Return the event map, observations, missing checks and the owner of each recovery decision. This review itself does not move session state.

## Before handing off

- [ ] Has the receiving session's actual context been checked?
- [ ] Has restore or close been exercised through the applicable host?
- [ ] Are missing context and state identified for the next worker?
