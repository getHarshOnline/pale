# Make failures useful to the caller

Design errors so the caller can tell what happened, what can be retried and whether state is safe. Keep the specific failure identifiable inside its owning component. Translate it when the receiving layer can act on a more useful category.

Distinguish invalid input, absence, conflict, denial, timeout, an unavailable dependency, a partial change, a broken internal guarantee and an unknown outcome. Preserve the information needed for recovery instead of turning one of these into success or an undifferentiated string.

Handle an error at the lowest layer that owns a valid response. Otherwise add the context the next layer needs and pass it on. Retry only a known transient operation that is safe to repeat, with an attempt limit and a check of current state. Contain dependency failure through a reduced service only when it still meets the declared outcome; tell the caller about omitted work.

## Keep cancellation effective

Treat cancellation separately from a failed optional step. Follow the signal through helpers, fallback, dispatch and late results. A helper may catch cancellation and return a usable value; its caller must still stop cancelled work. Check remote completion and billing with the provider separately from cancelling a local request or socket.

Before a write can partly succeed, decide how it will complete atomically, compensate, roll back or reconcile. When an outside action may have succeeded but its result is unavailable, retain the unknown state and check with the owning service before repeating it.

In the design or review reply, explain the failure categories, where they are translated, retry and containment rules, recovery from partial state, safe user messages and when escalation is needed. Use the project's language guidance for the error types and handling code, and include what the checks actually observed.

## A booking request times out

A room reservation times out after the request may have reached the booking service. Keep its request ID and tell the caller that the outcome is unknown. Check that ID with the service before retrying; if it created the booking, show that booking. If the caller cancelled, stop local follow-up work and check the booking separately.

## Before handing off

- [ ] Can the caller distinguish the cause from the action needed to recover?
- [ ] Are retries and partial writes handled explicitly?
- [ ] Were the relevant failures exercised and the resulting state checked?
