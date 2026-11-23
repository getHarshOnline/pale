# Choose the checks for a local publication build

Name the request, destination, authored source, asset revisions, configuration, theme and renderer. Resolve the configuration before interpreting the preview. Separate who owns the source, who chooses placement and where the reader can reach the result.

Run the supported local route on the candidate you will inspect. Record the input inventory, the publications built and the output each result refers to. Read the actual output and rerun the previously failing route against it. If two clients disagree, compare their requests, tool registration, executable, workspace and sessions before trying again.

When a single publication is expected, require a complete result for that one publication. In a multi-publication result, check the requested member against its own inputs and output. A passing sibling cannot supply a missing result. Use the local-render result check for the status, input inventory and output digest. Open and inspect the artifact separately. The renderer still owns its stage reports.

| If you need to check this | Read this |
| --- | --- |
| Check actions in their intended page slots | [Check actions in their intended page slots](page-actions.md) |
| Check material that must stay out of the output | [Check material that must stay out of the output](withheld-output.md) |
| Check every build stage and cached input | [Check every build stage and cached input](build-stages.md) |
| Check assembled documentation and generated reference | [Check assembled documentation and generated reference](documentation-output.md) |
| Check the bibliography in the rendered document | [Check the bibliography in the rendered document](bibliography.md) |
| Check resources through the actual build and renderer | [Check resources through the actual build and renderer](resource-loads.md) |
| Compare artifacts by the claim they need to support | [Compare artifacts by the claim they need to support](compare-artifacts.md) |
| Check that the browser opened the intended preview | [Match a preview to its built files](match-preview.md) |

## Before handing off

- [ ] Confirm the result is for the intended publication, inputs and destination.
- [ ] Read the actual output and rerun the case that failed.
- [ ] Name incomplete stages, missing checks and any release decision still open.
