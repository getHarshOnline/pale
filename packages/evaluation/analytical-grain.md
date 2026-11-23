# Check what an analysis counts

Use this method when a report, query or evaluation adds up observations or combines results from different sources. The owner defines the measure, which records qualify, which time period it covers and what evidence it requires.

## Reconcile grain, joins and denominators

Define what one row represents and the key that identifies it in each output. State the eligible population, the event or state that counts as an outcome, the time basis and interval, and the units of values or weights. A case, message, action and latest status are different things to count. Give each measure its denominator and say what happens when the population is empty. Fix an as-of cutoff when later status changes or related records could otherwise change an earlier group.

Before adding values, check how many rows on each side can match a join key. Count unique eligible keys, unmatched keys and ambiguous matches separately. If related rows only establish eligibility, test whether they exist. If they contribute values, first aggregate them to the target key, or state how their values and weights are allocated. Check whether a filter removes the unmatched rows from an outer join. A distinct case count does not repair a sum inflated by repeated rows, or a case total repeated across overlapping groups.

Reconcile the numerator, denominator and values with the eligible keys before rounding or displaying them. Check that mutually exclusive groups cover the intended population without overlap; keep deliberately overlapping groups identified as such. Unavailable, unknown and pending outcomes must remain distinct from negative outcomes. Without a denominator, report the measure as undefined. A record's presence, status or highest stage establishes only what that field actually represents. A stronger claim needs the relevant event or the owner's receipt.

Test repeated child rows, two independent one-to-many joins, a missing related row, overlapping categories and an empty population. Test records at the population boundary and a later status change against the fixed comparison. Inspect the resulting key sets, denominators and representative values. Include a case where the displayed count stays the same while the sum changes incorrectly. Keep three conclusions separate: the calculation reconciles, the observations measure the intended thing, and the result supports a causal claim.

## Before handing off

- [ ] State what one row represents and which key should be unique.
- [ ] Check repeated matches, missing rows and the denominator.
- [ ] Reconcile the output with a small known set.
