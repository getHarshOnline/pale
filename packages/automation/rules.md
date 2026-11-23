+++
description = "Check the rule applied, scheduled runs and replacement behaviour."

uses = [{ scope = "local", resource_id = "workflow" }]

[guide]
+++

# Check the applied rule, schedule and caller contract

Check a policy change, scheduled run or replacement implementation against what the owning system actually applies.

## Record the rule that governed a transition

Keep the requested or assigned policy separate from the exact revision and rule applied. Check the applicable condition and authority against the accepted prior state and effective time. Record the selected source and revision, exact rule or override, and enough permitted evidence to reconstruct the decision. If policy selects a calendar or clock rule, retain that selection for the affected interval. Keep policy content or an immutable reference only as its retention permission allows.

Use a fallback or override when the declared policy or responsible owner’s decision permits it, and carry that permission forward. Record why it ran, which rule governed and which requested path was unavailable. If the caller requires the requested policy and it cannot be established, refuse the transition. A successful state change, assignment label or current default cannot establish which rule was used historically.

Commit the applied-rule evidence with the accepted state and transition. Identify downstream notifications separately. Test a changed assignment, missing variant, old decision racing a new policy or state revision, and failure after the state write but before history commits. Read accepted state and history together. Preserve earlier observations with ambiguous attribution and leave their applied rule unresolved. When interpreting them in an experiment, use `pale.evaluation` resource `runs`, then its “Identify which release, method and run support the claim” guide, especially “Separate assigned policy from observed execution”, before crediting an outcome.

<a id="qualify-scheduled-occurrences-and-recovery"></a>

## Check scheduled runs and missed-run recovery

Identify the schedule owner, exact revision and rule for when work is due. Give each due occurrence a durable identity, with separate attempt identities. Define how an edit or cancellation affects work already due, accepted or running. Record the clock, calendar and time zone used to determine due work. Define what happens at skipped or repeated local times.

Treat wake-ups as hints. Determine due work from durable schedule and occurrence records so missed or duplicate wake-ups cannot lose or repeat acceptance. Test competing workers claiming the first attempt and a worker stopping between claim and dispatch. Before retry or takeover, reconcile the original effect identity and use the destination's actual duplicate protection.

Specify whether missed occurrences run individually, combine into a limited set, are skipped with a recorded reason, or remain held for review. Set the backlog limit and recovery window. A combined replacement must identify the occurrences it covers. Read the occurrence and its outcome independently of scheduler or queue status.

Test delayed and repeated wake-ups, downtime beyond the backlog limit, an edit or cancellation racing due work, and an interrupted worker. For local-time schedules, include missing and repeated instants. Inspect accepted occurrences, attempt counts and applied effects separately. Schedule definitions, cancellation authority and durable workers belong to the adopting project.

<a id="keep-the-caller-contract-stable-across-implementations"></a>

## Check that a replacement preserves caller behaviour

Agree what inputs, outputs, effects, identities and errors mean to the caller. Compare old and replacement implementations on the same selected inputs, including retries, uncertain outcomes, replay and adverse responses. Distinguish authored, generated, installed and active revisions. A different runtime may replace the implementation when it satisfies this contract and the required operating checks.

Keep deployment environment separate from execution mode. For replay, fix the method, input evidence and clock where time affects the result. Record intended consequences in an isolated destination. Replay establishes behaviour on those inputs; test real delivery and state changes separately through the permitted system route.

## Worked example

A schedule and its rules change while an occurrence is queued. Identify the occurrence and rule actually applied. Run the same request through the replacement implementation and compare results, failures and recovery. Keep each occurrence tied to its own time and policy.

## Before handing off

- [ ] Is the exact applied rule retained with the committed transition?
- [ ] Were missed occurrences, schedule changes and duplicate dispatch checked?
- [ ] Does a replacement preserve caller results, effects and failure behaviour?
