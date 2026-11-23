+++
description = "Choose what to check in the build and its output."
see_also = [{ scope = "local", resource_id = "build/actions" }, { scope = "local", resource_id = "build/withheld" }, { scope = "local", resource_id = "build/stages" }, { scope = "local", resource_id = "build/documentation" }, { scope = "local", resource_id = "build/references" }, { scope = "local", resource_id = "build/resources" }, { scope = "local", resource_id = "build/compare" }, { scope = "local", resource_id = "build/preview" }]

[guide]
exposure = "internal"

[task]
uses = [{ scope = "external", resource = { package_id = "pale.engineering", resource_id = "execution" } }, { scope = "local", resource_id = "references" }]
capability_policy = { scope = "local", resource_id = "build-policy" }
capability_requirements = [{ id = "render", description = "Render the selected publication locally and return exact input and output identities.", capability = "publication-local-render", recipe = { scope = "local", resource_id = "render-local" }, effect_class = "execute", policy = { kind = "required_block" }, failure_posture = "fail_closed" }]
outputs = ["In the build result: publication, input versions, output identity and renderer result record.", "Checks performed, observations and unresolved evidence or decisions."]
+++

# Choose the checks for a local publication build

Name the request, destination, authored source, asset revisions, configuration, theme and renderer. Resolve the configuration before interpreting the preview. Separate who owns the source, who chooses placement and where the reader can reach the result.

Run the supported local route on the candidate you will inspect. When that route cannot run, report the blocked check. Record the input inventory, the publications built and the output each result refers to. Read the actual output and rerun the previously failing route against it. If two clients disagree, compare their requests, tool registration, executable, workspace and sessions before trying again.

When a single publication is expected, require a complete result for that one publication. In a multi-publication result, check the requested member against its own inputs and output. A passing sibling cannot supply a missing result. Use the local-render result check for the status, input inventory and output digest. Open and inspect the artifact separately. The renderer still owns its stage reports. Report the compile and read-back findings separately. Browser behaviour needs a browser check; the publisher decides release, including rights, destination and approval.

| If you need to check this | Read this |
| --- | --- |
| Check actions in their intended page slots | [Check actions in their intended page slots](build/actions.md) |
| Check material that must stay out of the output | [Check material that must stay out of the output](build/withheld.md) |
| Check every build stage and cached input | [Check every build stage and cached input](build/stages.md) |
| Check assembled documentation and generated reference | [Check assembled documentation and generated reference](build/documentation.md) |
| Check the bibliography in the rendered document | [Check the bibliography in the rendered document](build/references.md) |
| Check resources through the actual build and renderer | [Check resources through the actual build and renderer](build/resources.md) |
| Compare artifacts by the claim they need to support | [Compare artifacts by the claim they need to support](build/compare.md) |
| Check that the browser opened the intended preview | [Match a preview to its built files](build/preview.md) |

## Before handing off

- [ ] Confirm the result is for the intended publication, inputs and destination.
- [ ] Read the actual output and rerun the case that failed.
- [ ] Name incomplete stages, missing checks and any release decision still open.
