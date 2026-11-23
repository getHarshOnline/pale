+++
description = 'Review Spring Statemachine where the project uses its state graph and dependencies. Select persistence guidance separately.'
uses = [{ scope = "local", resource_id = "spring" }]
official_sources = [{ label = 'Spring Statemachine reference; resolve the consuming version', url = 'https://docs.spring.io/spring-statemachine/docs/4.0.1/reference/' }]

[guide]

[stack]
extends = [{ scope = "local", resource_id = "spring" }]
role_uses = [{ scope = "local", resource_id = "spring-statemachine" }]
+++

# Check state-machine transitions completion and recovery

Apply this guidance when Spring Statemachine is selected, after Spring, Java and the common method. Identify the actual library, graph builder, machine factory, regions and dispatcher. Select persistence and workflow or effect methods separately when needed; an in-memory state machine supplies no durable workflow on its own.

## Inspect the graph that was built

Inspect actual states, edges, guards and bound actions. In the examined 4.0.1 external-transition DSL, assigning a second source, target and event on one mutable configurer replaces its edge instead of adding one. Check the intended edge set, overlapping triggers, guard results, priority, hierarchy and recovery routes in the built graph. A declaration-order result from one flat graph does not establish every conflict-resolution policy.

Distinguish transition actions, entry and exit work, and state-do work. An internal self-transition can run its transition action while skipping entry hooks; choose the kind that supplies the required re-entry behaviour. Trace chained events against machine state at dispatch, including deferred and queued work. Establish factory startup and context before accepting events. Check database commits and completed outside effects separately from entry into a target state.

## Distinguish event acceptance from completed work

Reactive event results report acceptance, denial or deferral by region. Account for the relevant results and their completion route. A result stream can finish before an asynchronous action, so establish the action's completion signal rather than assuming that blocking the stream waits for all work. Native persistence, business acceptance and downstream delivery have their own completion conditions.

Trace errors through the selected action or entry callback, reactive chain, listener, facade and transaction. In the exercised 4.0.1 fixture, a throwing transition action produced denial without exposing its cause on completion. Investigate the cause of a denial before calling it a business rejection. Check where each failure appears and whether a real transaction rolled back. Retain protected causal diagnostics without leaking payloads, and verify recovery and durable audit through their actual mechanisms.

## Restore the machine before repeating an effect

Bind machine identity, graph revision, the durable state source, pending event and required extended context. Restore from the owner's authoritative checkpoint with explicit fallback rules; a display status may omit needed state. Decide which entry or initial actions may run during stop, reset and start, and which transient values must be reconstructed. Exercise restoration and replay in a fresh instance.

Keep the aggregate's mutation guard, scheduler claim, outside attempt and accepted review decision distinct. Recheck state, revision and current authority where the protected transition or effect happens. Coordinate transactions with persistence and recovery, including work on other threads. Do not hold network work inside a lock merely to make the diagram appear atomic. The owners retain the business graph, review policy, repeat-safety rules, checkpoint schema and native tools.

Exercise changed edges, false guards, entry versus transition hooks, asynchronous completion, cancellation, failure diagnostics, restart and concurrent or stale work where affected. A controlled library fixture establishes the semantics it exercises; test the application's graph assembly, persistence and outcomes through the repository's permitted native paths.

## Restore a saved order

A guarded transition moves a saved order from `draft` to `submitted`. Exercise a denied guard and a failed action separately. Restore stored state into a fresh machine and check which effect the application actually committed before repeating the event.

## Versioned references

- [Spring Statemachine 4.0.1 reference](https://docs.spring.io/spring-statemachine/docs/4.0.1/reference/)
- [Spring Statemachine 4.0.1 event-result API](https://docs.spring.io/spring-statemachine/docs/4.0.1/api/org/springframework/statemachine/StateMachineEventResult.html)

These describe the examined version. Resolve the consuming project's actual library, configuration and execution path.
