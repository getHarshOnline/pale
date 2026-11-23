# Check the candidate an optimiser produces

## Freeze the optimisation problem

Fix the base state, profile and rule versions, objective and whether to minimise or maximise it, decision variables, locked values, domains, units and tolerances. Distinguish hard constraints from preferences used to rank results. A penalty can guide search, but each hard constraint needs independent verification before a result can be called feasible. Test boundaries and branch changes under the selected profile. State and justify the limits of any approximation.

## Verify the exact emitted candidate

Build each trial from an isolated copy of the accepted base. Retain the requested vector and inspect the values that actually reached the evaluator. Reevaluate the exact emitted candidate after rounding, clamping, repair or canonicalisation. Tie the recomputed objective, result of each constraint check and evaluation identity to that vector and the fixed problem. A cached baseline result or convergence flag supplies no evidence about the new candidate. Inspect the base after success, rejection and interruption. Applying a checked candidate remains a separate action for the owner.

## Limit the search claim to its proof

Return the best verified feasible candidate, stopping reason, remaining uncertainty and relevant binding constraints and sensitivity. Identify a fallback as a separate solver path with current permission, implementation and scope. An incomplete search without a feasible point leaves the question unresolved. Infeasibility or global optimality requires a valid certificate or complete coverage of a bounded domain. State local claims or bounds with the domain and evidence they actually cover.

Use independent reference values for a small problem. Test a baseline that meets the target while the proposed candidate fails, a solver that stops without a solution and rounding that changes the selected point. After replay, check whether evidence supports a stronger search claim. The owner supplies constraints, permitted solver paths and decisions.

## Verify feasibility within the funding envelope

Fix the target, baseline, unit, deadline, available resources and response to each permitted action. A target level differs from a target share of a changing total. Calculate the opening margin against the selected target. Keep available capacity, actual injections and accessible balances distinct. If funding an action changes another input, replay those linked terms and count each source once. Stop using a fixed-input shortcut when its conditions no longer hold.

Check the unchanged baseline first. If it already meets the target, show that zero action is feasible. If a gap remains, check the action's response and limits before dividing, searching or reporting utilisation. A zero denominator can make a ratio undefined while feasibility remains decidable. Check any proposed minimum against the exact constraints and justify why a smaller action fails. Test a point exactly on the feasible boundary and one just outside it. Future resources cannot repair a missed deadline unless the governing contract permits that timing.

For a relaxation over dated periods, establish that every feasible trajectory of the full problem must pass the relaxed test over the stated prefix. Explain what was omitted, pooled or bounded and why passing is necessary. Failure of a necessary condition supplies a certificate within that scope; passing leaves full feasibility open. State the first failed constraint, interval and tolerance. A feasible allocation found by search bounds the minimum only in the direction that the example proves. Search stopped without a feasible example supplies no certificate.

Keep diagnostic plans, slack and partial provisioning separate from evidence of feasibility. Preserve original targets and show remaining shortfalls. An accepted sensitivity band defines a different profile; recheck the same obligations and exact replay before using a feasible result from it. Test an already-met target, an action with no response, zero capacity, exact and inadequate funding, an early shortfall followed by later funding, and a relaxed test that passes while the full case fails.

## Before handing off

- [ ] Read the candidate the solver actually emitted.
- [ ] Check feasibility and constraints on that candidate.
- [ ] State the supported search or optimality claim separately.
