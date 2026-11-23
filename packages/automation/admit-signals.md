+++
description = "Decide whether an incoming signal asks for a business action, and give admitted work an owner, a deadline and an escalation."
uses = [{ scope = "local", resource_id = "workflow" }]
see_also = [{ scope = "local", resource_id = "human-review" }]

[guide]
+++

# Admit a signal as work only when it asks for a business action

A signal is anything that arrives and might matter: an email, an alert, a status change, a webhook. Admit it as owned work only when it asks for a business action, such as a decision, a change, a reply or a payment. Record which action it asks for, who is entitled to ask for it and why this system, rather than a person's own attention, has to carry it.

A signal that only informs stays where people read it: a dashboard, a digest or a log. Admitting it would turn the work queue into one more unread inbox, and the items that need action would sink among the ones that do not. If a person or a rule later shows that an informational signal needs action, promote it then and record the promotion.

## Give admitted work an owner, a deadline and an escalation

An admitted item needs all three before it is shown as work.

- **Owner.** A named person or role who can take the action. An assignment is not ownership until the owner accepts it, so an item nobody has accepted is unowned work with an escalation of its own.
- **Deadline.** Set from the consequence of the action arriving late, such as a payment run or a customer's wait, and not from the time the signal arrived.
- **Escalation.** Who is told when the deadline passes without acceptance or action, by which route and with what disclosure. Escalate once per item, and leave the original owner's duty in place.

Keep one item per action. A repeat delivery, a follow-up or a related signal attaches to the item that already covers its action instead of opening another. Close an item when the action is done and its result has been read back from the system that owns it, or when the signal stops mattering, with the reason recorded. An item that is neither done nor closed stays visibly open.

For example, a supplier's message says an invoice date has moved. If nobody has to do anything about it, it appears on the dashboard and nowhere else. If it asks the accounts owner to approve the new date, it becomes one item for that owner, due before the payment run, escalating to their lead if the run is close and nobody has accepted it.

## Check the boundary in both directions

Send an informational signal and confirm it creates no queue entry. Send an action-bearing signal and confirm it creates exactly one item with an owner, a deadline and an escalation. Deliver the same signal twice, deliver a follow-up after the item is closed, let a deadline pass and let an owner decline. Read the queue after each case: the right items appear once, none age silently, and a promoted signal shows why it was promoted. Queue ordering and operator review belong to [human review](human-review.md).

## Before handing off

- [ ] Does each admitted item name the business action it asks for?
- [ ] Do owner, deadline and escalation exist before the item counts as work?
- [ ] Do informational signals stay out of the queue, and repeats attach to one item?
