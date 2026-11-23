+++
description = "Check numerical results against independent references, boundary cases and justified tolerances."

[guide]

+++

# Check numerical results against independent references

To claim numerical agreement, fix the governing formula or algorithm, input domain, units, arithmetic and order of reduction where it matters. Identify the profile, implementation, entry point and reference revision. Decide which fields must match exactly or in order and which permit numerical error. Choose and justify absolute, relative or scale-aware error bounds for each quantity. Test the acceptance boundary, including values near zero where relative error alone is insufficient. Keep implementation tolerance separate from measurement uncertainty and decision margins.

## Bind the quantity before comparing values

State what each value measures: subject and coverage, period, units, scale and measurement basis. For a rate, ratio or weighted result, identify numerator and denominator, the counted unit and exclusions. For money, state the currency and whether fees or taxes are included. Identical labels or numbers can describe different quantities. Give blank or unavailable inputs their own result.

Compare values only after these properties match or a permitted conversion makes them comparable. Identify the conversion's source, direction, inputs and effective period. Preserve source values and material intermediate values. Changing scale changes representation. Converting gross to net, a unit rate to a total or one currency to another needs the governing inputs. Aggregate rates with their required weights and a reconciled denominator. Leave values incomparable when conversions are missing or matches unsupported.

Apply tolerance only after establishing comparability. Test an equivalent scale change, equal digits with incompatible bases, a missing conversion input and a weighted aggregate that differs from an unweighted average. Inspect the original value, normalized value and comparison result separately.

## Reconcile period flows with carried balances

For a model that carries balances, identify the opening state, period boundaries and when each movement is recognised and settled. Keep gross obligations, deductions, net recipient payments, remittances and cash receipts distinct. Derive subtotals independently from component rows. Money remaining after expenses differs from expenses, even if a formula or label gives them the same name.

Reconcile opening cash plus actual receipts minus cash payments to closing cash for each period and the full interval. Carry receivables and liabilities across boundaries instead of forcing them to zero. For a withholding flow whose net recipient amounts are paid in-period with no other adjustments, closing withholding liability equals opening liability plus deductions minus remittances; cash paid for that flow equals gross obligations plus opening liability minus closing liability. State these conditions. Credits, refunds, prepayments and other movements need their own supported terms. Use the owner's actual settlement rules and dates; the deduction date alone does not say when remittance leaves cash.

Test a remainder mislabelled as an expense, an opening liability, a final-period deduction remitted later and receipts moved between periods without changing the full-interval total. Include gross and net amounts incorrectly counted as separate payments. Check intermediate states as well as the final sum and retain unexplained differences. Reconciliation alone does not establish accounting or legal treatment, forecast validity, funding feasibility or permission to pay.

## Establish the independent reference

Derive reference values independently of the implementation being tested. Use hand calculations, an independent derivation or a sufficiently precise reference with a documented conversion to the target representation. Keep its source, assumptions and review result. A snapshot produced by the candidate can detect changes but cannot be its only reference. If implementations share code or fixtures, disclose that dependency before treating agreement as corroboration.

Choose cases that distinguish the intended algorithm from a plausible wrong one. Test relevant boundaries such as clamping, the first change in sample-size behaviour, signed zero, non-finite values, overflow and underflow. Check rejection, empty inputs and required output properties separately from numerical distance. For claims about state updates, also check exact event identity, replay and the resulting state in the owning system.

Report the first divergence, observed and reference values, permitted tolerance and exact fields that matched. Agreement covers only the selected profile, cases and entry points. Formula validity, causal meaning and usefulness on a workload need their own evidence.

## Check conditional and piecewise formulas

Identify the conditioning event, initial state, time origin and ordered input path before comparing formulas. Distinguish a result conditional on a stated path from an unconditional result or forecast. Check that initial mass and every state update preserve required totals. Account for unresolved mass at the horizon. Report an unreachable quantile as unresolved; claims beyond the tested horizon need separate evidence.

Use an independently derived case that exposes a wrong origin, reset or ordering. Test a zero-length interval and composition over two intervals when the model supports it. If elapsed state or a changing input path matters, carry that state through the split. Compare with the simpler constant-input model where it is a true limiting case. Use unequal path segments or noncommuting updates to expose accidental reordering. Independently verify conservation and boundary values.

For a guarded formula, establish each term's domain before evaluating it. An inactive branch may contain an undefined expression; zero weight does not make it safe to evaluate. Test a valid zero, missing or incomparable input, both sides of each boundary and the boundary itself. If terms reuse the same signal or funding source, trace its contribution to the output and use a case that exposes double counting. Distinguish accepted parameters and thresholds, inferred values, synthetic controls and observed outcomes.

## Keep monetary arithmetic exact until the required rounding step

Parse source decimal amounts into whole-number units at a declared scale or an exact fraction with integer numerator and denominator. Keep percentages and division exact as well; converting a binary floating-point result afterwards does not recover the source decimal. Carry currency, scale, formula version and the owner's rounding rule with the inputs. Use an integer representation that covers the permitted range and reject overflow or an invalid denominator. An arbitrary-precision integer can remove a fixed-width limit without settling the formula's meaning.

Preserve exact intermediate amounts. Round at the steps the governing rule requires, distinguishing line-item rounding from final-total or display rounding. For an illustrative two-decimal display, three exact amounts of one-third sum to 1.00; rounding each first to 0.33 produces 0.99. Neither policy may silently replace the other. Record any allocation of a remainder under its accepted rule.

Test zero amounts, negative values where permitted, half-way rounding, large values and a zero denominator. A zero rate remains zero; a missing rate stays missing. Bind reviewed values and source citations to the accepted formula and input revision. Changed inputs require the appropriate review again. Exact calculation cannot establish that an extracted value is correct, choose between disputed commercial terms or grant permission to pay.

## Before handing off

- [ ] Are the quantities, inputs, formulas and permitted tolerances comparable?
- [ ] Do independent references and boundary cases distinguish the intended calculation from plausible errors?
- [ ] Is the first divergence or supported scope clear without claiming more than the checks establish?
