# Check field-device paths in the environment they use

## Check field-device paths separately

Identify device, firmware, site assignment, gateway and consumer revision. Observe reachability, event capture, buffering, delivery and committed use separately. A recent heartbeat proves only its own route. Test evidence or media delivery when the workflow needs it, and check human handoffs with their owners.

Test a reachable device with a failed event or media route, an offline period followed by replay and a device restart. Associate events with the original device instance and assignment. Preserve capture and receipt times, reject effects based on stale assignments and inspect consumers for duplicate or missing commits. Report degraded capabilities and recovery limits for each route before expanding the rollout.

### Verify field measurements and command consequences

Specify the physical observation behind a software claim. Identify instrument, installation, current assignment, capture time, unit, calibration or tolerance evidence and every transformation into the consumed value. Working transport and decoding do not prove a valid measurement. Distinguish unavailable, invalid, stale, inferred and defaulted values; use zero only when an observation supports it. Before combining readings, compare measurement windows, units and overlapping coverage.

For a permitted command, identify requested change and target. Observe eligibility, control mode, dispatch, acknowledgement or feedback and actual consequence separately. Choose a verification signal whose source, timing and limits distinguish the requested change from stale telemetry, manual operation or unrelated activity. If feedback and observation disagree, reconcile the command and current state with the owner before retrying. Distinguish physical effect, its attribution and any claimed operational benefit.

Test valid and stale readings, reachable devices with invalid or missing measurements, replay after reassignment and inferred values incorrectly shown as measured. For command routes, test acknowledgement without effect and lost or negative feedback despite an observed change. Use simulation or a controlled setup until site owners permit a specific field test. State the expected-answer source and its limits. Owners supply device tolerances, operating limits, interlocks, manual permissions and recovery rules.

Put results in the field-test or release review: topology, stage requirements, coexistence assumptions, exposure and abort limits, observation plan, recovery decision and unresolved facts. Readiness assessment separately decides whether those observations justify deploying this arrangement.

## Worked example

A device reports a changed setting. Compare the request, acknowledgement and actual device state. Observe the measurement on the correct channel and in the correct unit. Exercise delayed and duplicate requests against a safe target, keeping measurement and command results separate.
