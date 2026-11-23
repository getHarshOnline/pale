+++
description = "Use it to execute a notebook in a clean process in the approved environment and to check that the export carries the computed results."
see_also = [{ scope = "local", resource_id = "notebook" }]
official_sources = [{ label = "jupyter native documentation", url = "https://nbclient.readthedocs.io/en/latest/client.html" }]

[guide]


[stack]

+++

# Check notebook execution and exported results

Use the approved environment and kernel specification. Execute in a clean process with the intended working directory, timeout and input paths. Fail on unexpected errors and keep the source notebook separate from the executed copy. Record the kernel interpreter and dependency versions with the run. Verify that the report or export includes the computed results and may be shared under the source permissions. A saved cell output remains historical until reproducible execution connects it to the current method and data.

## Bind execution controls and retained method

State which parameters select the population and inputs, which change only the view, and which control saving or attaching a review. Keep the resolved selection with the result. Change parameters only at their declared assignment locations in a review copy. Changing computation, or replacing a literal control with executable code, changes the method.

Move reusable plotting boilerplate into helper files while keeping the notebook's analytical steps, inputs and computed results visible. Identify the method source, helpers and dependencies actually loaded by the kernel. Restart after helper changes and recheck the method and inputs before retaining a run. The current file on disk cannot establish what a long-lived kernel imported. Distinguish method, request, executed notebook and structured result. Reject scope or method mismatches and keep new runs separate from earlier evidence.

## Preserve method code when changing or masking controls

Before replacing or masking controls, declare their names, allowed assignment forms and expected occurrence counts. Verify the literal value and exact source span. A tool that replaces a whole line must require that line to contain only the declared assignment. Another statement on that line may calculate a result or perform an action; clearing the line could erase it from execution and from the method fingerprint. A tool that replaces exact source spans may instead preserve the other statement unchanged.

Use the same assignment boundary when preparing the notebook and verifying its method. Exclude only approved controls from source comparison. Keep executable expressions, undeclared statements and ordering in the method identity, or reject unsupported layouts. Run these checks before relying on a new result. Preserve earlier notebooks and receipts under their original method and verifier revisions.

Test ordinary literal substitution, a control sharing a line with a calculation, two controls on one line and a repeated control. Verify that the calculation remains both in execution and in the method identity, or that preparation and verification reject the layout. Check that valid control substitution preserves the expected method identity.

## Bind exported claims to the completed run

Before executing a review copy, clear saved outputs and execution counts. Identify the source notebook, resolved input versions, working directory and kernel environment. State which result cells must run and which cells, if any, the execution profile may skip. Unexpected errors or missing required results leave the run incomplete even if the original notebook still shows saved output.

Inspect the newly executed notebook. Verify that required results were produced in that run and that exported values, units and scope match them. Keep source, executed notebook and export separately identified. Test a failed input load with stale saved output and an export linked to the wrong run. Neither supports a new claim.

## Worked example

Run `jupyter nbconvert --execute --to notebook --output checked.ipynb analysis.ipynb` from the intended environment. Inspect the new notebook for the expected result and a failure in a relevant cell. Keep the source notebook separate from the executed output.

## Before handing off

- [ ] Run the notebook from clean state on its intended inputs.
- [ ] Read the resulting outputs and inspect failed cells.
- [ ] Keep claims tied to the executed notebook and its data.
