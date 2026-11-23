# Check every build stage and cached input

## Check each required stage

List the build stages and child jobs required for this candidate, including generated figures, references and inventories. Associate each child with its full inputs and output path. Inspect its diagnostics and result. A later success or readable final artifact cannot establish that an earlier child succeeded.

Stage generated output separately. Accept a child artifact only after its result and required readback pass. Distinguish an earlier valid output from a failed successor. Before reusing an output on retry, verify that it meets the current complete inputs. Require every necessary child to pass before final rendering.

Test an early child failing before a later one succeeds, and a failed rebuild with older output already present. The overall build must fail, partial output must stay outside the verified set, and retry must still detect the unresolved child. Repair the input and check that the intended successor appears in the final artifact.

## Check inputs before reusing cached output

Identify all resolved inputs that can change the output: transitive files, resolution order, tool and recipe versions, configuration and relevant environment. Associate paths with content identity and include required members and absences. File times are hints. If complete inputs cannot be established, rebuild in the permitted environment and state that gap in the result.

For a cache hit, check that the saved build record covers current inputs and that output matches its inventory and digests. Separately recheck current permission and destination. Permitted reuse can provide bytes without a fresh render observation. A failed successor build cannot make an old output valid for that successor.

Test an unchanged hit, a transitive change retaining name and timestamp, a missing dependency and corrupt cached output. Ensure stale or corrupt content cannot reach final rendering, then independently read back the corrected output.

## Compile templates, inspect their data and render the same candidate

A template or theme can fail before rendering, while assembling an entry's data, or only in a particular component or branch. Check which templates and dependencies the application loaded. Render the affected entry through the publishing engine and inspect its data beside the generated page. A missing required field points to the data's producer; a present field used incorrectly points to rendering logic. A standalone template runner can isolate syntax or scope, but repeat the check through the application that supplies functions, input limits and output rules.

Use neutral fixtures for missing, null and present optional fields; empty, single-item and longer lists; and the actual family, relationship, lifecycle and capability cases served by the change. Require missing-input errors where a reusable component needs the field. Optional fields should reach a supported empty or unavailable state. A malformed template can fail during parsing even if unused, while an unknown function in a branch may fail only when that branch runs. Check compilation and rendered cases. After a shared-base change, exercise its real consumers and alternate entry points.

## Worked example

A figure stage reuses an old output after an indirect source changes. Compare each child's input inventory, recipe, tool version and output. Rebuild the affected child and final renderer. Then fail an earlier child before a later success and verify that the old figure cannot make the overall build pass.
