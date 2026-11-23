+++
description = "Put unfinished work and accepted limits on the owning work list, with the responsible owner, checks and an observable reason to revisit them."

[policy]
+++

# Make unfinished work and accepted limits explicit

A smaller solution can be complete within an agreed range of use. Say what that range is and what would require a larger solution. A missing requirement, failed check, unknown safety condition or unresolved owner decision remains unfinished work.

Put each deferred task on the owning product's work list. Explain what remains, why it is outside the current change, what consequence is accepted meanwhile, the conditions under which the interim design works, and who will reconsider it. Give an observable reason to revisit it: load, number of users, expiry, changed evidence, incompatibility, an incident or a named decision. Include the check that establishes the interim design stays within its limit.

Distinguish three cases:

- **Optional follow-up:** the accepted outcome is complete; the later enhancement is optional.
- **Accepted limit:** the outcome was deliberately narrowed, and its users can see that limit.
- **Blocker:** the requested outcome remains incomplete until this task is resolved.

Choose the case from the agreement and evidence, not pressure to finish. A run record can report a decision and the current state, while the owning work list keeps responsibility for the task. Naming a future trigger does not make a system monitor or act on it automatically.

In the handoff, link to the task and explain the affected users, reason, accepted limit, responsible owner, trigger, evidence needed on return and interim checks. Remove it from current work when it is completed, replaced by an attributable decision or shown to be inapplicable. Keep the history with the decision or product record when it explains an earlier design.
