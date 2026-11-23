+++
description = "Retire an old route only after the domain that replaces it passes its own gate, and account for every route before calling the retirement complete."
see_also = [{ scope = "local", resource_id = "retire-copy" }]

[guide]
+++

# Retire a route only after its replacement passes its own gate

A route is any way a caller reaches a behaviour: an address, an endpoint, a queue, a command, a link, a scheduled job, a mailbox, or the place a copy of data is read from. Retire the old route when the domain that replaces it has passed its own acceptance gate, meaning the checks its owner applies to work that arrives that way, such as authentication, permissions, validation and recovery. The side being retired cannot declare the replacement ready, and a successful request through the new route establishes only that request.

## Account for the whole estate before claiming full retirement

List every route to the behaviour, not only the documented ones. Look in configuration, scheduled jobs, other repositories, saved links and bookmarks, partner or customer integrations, and the paths used only at month end, in recovery or in an audit. Compare the list with usage records covering a period long enough to include the rare paths. Give each route one of three outcomes: its replacement and that replacement's gate result, the owner's decision to drop it, or a named gap.

A route without an outcome blocks the word "retired". Say which routes are retired and which remain, with the owner of each remaining one. Routes that sit in someone else's domain, such as a partner's integration, need that owner's confirmation of the move and not only the absence of recent traffic.

## Move callers, then turn the route off

Run the replacement beside the old route and move callers across. Compare results on the same inputs, including failures, retries and the cases the old route handled quietly. Watch the old route's use fall to what the move predicts, and find every remaining caller. When it is off, make it refuse with a pointer to its replacement instead of failing silently. Keep the way back until the replacement has served one full cycle of its work, then delete the old route and its configuration. The same care applies to a copy of data, covered by [retiring a recovery copy](retire-copy.md).

For example, a service moves from one address to a new gateway. The gateway passes its own authentication and permission checks, the usage log shows a weekly job and a partner still calling the old address, and the partner confirms its move. Only then is the old address refused with a pointer and, a full cycle later, removed.

## Before handing off

- [ ] Has the replacement passed its owner's gate for the work arriving by this route?
- [ ] Does every route in the estate have a replacement, a drop decision or a named gap?
- [ ] Can the retirement be reversed until the replacement has served a full cycle?
