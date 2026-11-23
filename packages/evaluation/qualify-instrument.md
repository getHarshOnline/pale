# Test an evaluation method before relying on it

Name the decision the result could change, the evidence available for it, and the exact scorer, rubric and case-set versions. Distinguish known-good, known-bad and unknown or unavailable cases. Set their expected results before looking at candidate scores. Use deterministic checks for mechanical claims. For judgment, use examples that distinguish adjacent grades and independent calibration when the decision requires it.

Run the project's method in its approved comparison environment when available. Confirm that a control reaches the check intended to reject it: an earlier unrelated failure does not demonstrate that check. Report assigned and scored case counts separately. Keep cases used to tune the rubric out of the holdout set. Changes to cases or method require a new comparison, and changes to any fixed input require a new result.

Return whether the method is qualified, qualified with limits or unqualified, with the control observations and remaining limits. A qualified method can support later comparisons within that scope. It does not decide release or policy.

## Test an independent checker for a generated candidate

Before generation, fix the property being checked, what it means, valid inputs, acceptable outputs, threats the check must withstand and the response to failure. Identify the checker, dependencies, fixtures, evaluation budget and components that must be trusted. Keep an independent checker outside the generator's permission to write or approve. A generated specification or test can propose a requirement, but it needs separate acceptance before it can judge the output. Keep the property and checker unchanged across repair attempts.

Choose a check that can establish the exact claim. Syntax validation, a successful build, execution within limits, a counterexample and a proof under stated assumptions establish different things. State the checked input domain and untested behaviour. Test a correct candidate, a buildable candidate that violates the property, a weakened check, a checker failure and an out-of-scope case. Independently inspect the protected checker and its actual result. A skipped check or exhausted budget leaves the required property unresolved.

For authorised execution, test the disposable runner before running the candidate. Apply the owner's limits on credentials, network, mounts, writable paths, child processes, runtime and resources. Verify that the candidate cannot alter the checker or read protected controls. Retain the runner's structured receipt identifying the executed bytes, check, outcome and verified cleanup. If the required execution isolation is unavailable, use only permitted static evidence. Keep permission to execute, acceptance of the checked property and permission to release separate.

## Before handing off

- [ ] Run the scorer on cases it should separate.
- [ ] Check controls, disagreements and relevant thresholds.
- [ ] Report which claim the instrument can support.
