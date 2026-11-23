+++
description = "Trace a journey taken by a person or an agent through competing writes, repeated requests, interrupted replies and permission checks to find races or lost changes."

[task]
roles = [{ scope = "local", resource_id = "review" }]
outputs = ["In the reply: the promise, relevant states and paths, concrete mismatches and what was read or run."]
+++

# Check a journey for races and lost changes

A journey is the path one actor takes to a result: a person using an interface, or an agent calling a tool or an API. Start with a promise that actor can observe and the evidence available for checking it. Trace the relevant states, who owns each one, permitted transitions and visible outcomes. Follow competing and interrupted paths alongside the ordinary journey. Look for ambiguous ownership, lost or repeated transitions, and outcomes the supplied code or observations cannot reach.

## Find which system commits the result

For each consequential transition, note in the working analysis who acts, the starting revision or state, the required guard, the request or occurrence identity, and the component that can commit the next state. Distinguish accepting a request, durably committing it, delivering a response, displaying a derived view and independently reading the result back. Determine which event the caller actually observed.

Where the code permits them, trace a competing write between validation and commit, duplicate delivery and a lost reply after a possible commit. Identify which condition must be checked again, how replaying the same request finds its original result, and how reuse of its identity with changed content is rejected. If the result is unknown, identify the read-back from its owner that can settle it before recommending a retry. A refreshed display needs that same connection to the committed result, and for an agent the equivalent is a tool result returned to it.

## Follow the permission check along every route

Start with the actual permission record required for the action. Establish its owner, resource, current revision or generation and purpose. Follow each route that can reach the action: browser or client, a tool or API call, direct service entry, queue, worker and background trigger. Check permission before each route exposes protected state, commits a change, queues work or passes data to an outside action. If permission can change while work is queued, check it again at execution.

Hold the request and other inputs fixed while comparing a current permitted record with three controls: a missing record, the same local identifier under another owner or scope, and a superseded generation. Observe the response, state change, queue submission and downstream call separately. A browser refusal, service credential and matching identifier establish different facts; follow each to the action it actually governs. If a worker or generator is intercepted for testing, say where the observation stops.

In the reply, give the promise, the states needed to explain it, supported and unsupported paths, and each mismatch with its source or observed result. Source reading can establish a possible path; a run establishes what happened in its tested environment. This audit ends with the diagnosis. Repairs and release follow their own task and permission.

For each finding, identify the earliest transition that breaks the promised result.
