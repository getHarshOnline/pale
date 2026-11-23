# Trace workflow identity, effects and replay

Draw the workflow from the request to the observed result. At each step name the owner, input, output, allowed effect and safe failure. Keep the business object, request, execution, provider attempt and effect IDs separate. Give the effect one idempotency key across retries, with its expected revision and a rejection rule for a different payload.

Save the intent before the effect can happen. On a lost response, reconcile using the original key and the provider’s read-back. Distinguish absent, committed and unknown outcomes. Isolate unknown outcomes when the provider cannot safely deduplicate or reconcile them. Set a retry limit. Say where exhausted jobs go, who handles them, and which system records the business action as complete. An outbox or queue moves work. Check what the destination commits and whether it deduplicates retries.

Choose live, shadow or replay at entry and pass that mode to every child. List writes, labels, callbacks, notifications and provider routes. Replay uses a fixed input set, separate identities and providers that refuse live effects. Test a missing or conflicting mode in a child. Give the work queue, human review and telemetry separate state and recovery paths. A telemetry outage must not cause a business action to repeat.

## Choose the check

| If you are doing this | Read this |
| --- | --- |
| <a id="deliver-from-a-committed-occurrence"></a>Check an occurrence, producer ordering, delivery or reconnect. | [Deliver committed occurrences and recover history](effects/deliver-and-reconnect.md#deliver-from-a-committed-occurrence) |
| <a id="qualify-a-separately-authorized-delivery"></a>Check a delivery that needs separate permission. | [Check a separately permitted delivery](effects/deliver-and-reconnect.md#qualify-a-separately-authorized-delivery) |
| Check queue expiry, terminal results, prerequisites or reporting. | [Commit results once and retry from durable state](effects/commit-and-retry.md) |
| Check a callback, acknowledgement or refreshed authentication. | [Check callbacks and destination identity](effects/verify-the-route.md) |
| <a id="keep-the-caller-contract-stable-across-implementations"></a>Check a policy change, schedule or replacement implementation. | [Check the applied rule, schedule and caller contract](effects/rules-and-schedules.md#keep-the-caller-contract-stable-across-implementations) |

## Before handing off

- [ ] Keep business, attempt and effect identities distinct.
- [ ] Check read-back, retry and cancellation under the original effect identity.
- [ ] Run replay with separate inputs and no live effects.
- [ ] Name the owner and next safe action for unknown outcomes.
