+++
description = "Choose generation controls, diagnose image or footage defects and compare outputs with the requests and runs that produced them."
uses = [{ scope = "local", resource_id = "assets" }]
see_also = [{ scope = "local", resource_id = "generate/choose-what-to-hold" }, { scope = "local", resource_id = "generate/combine-controls" }, { scope = "local", resource_id = "generate/plan-a-round" }, { scope = "local", resource_id = "generate/record-a-run" }, { scope = "local", resource_id = "generate/comfyui" }, { scope = "local", resource_id = "generate/controlnet" }, { scope = "local", resource_id = "generate/midjourney" }, { scope = "local", resource_id = "generate/read-a-diffusion-graph" }, { scope = "local", resource_id = "generate/write-a-request" }]

[guide]
+++

# Choose and diagnose a generation method

Use this for an image or footage generation, controlled edit or comparison. Start with the accepted references and the intended change. Decide what the output must keep and which control could preserve it. Use [Make an asset that belongs in its destination](assets.md) for the resulting asset.

| If you are doing this | Read this |
| --- | --- |
| Decide whether to use a prompt, mask, reference or structural control | [Choose what to hold while generating an edit](generate/choose-what-to-hold.md) |
| Prepare a map or combine controls | [Combine structural controls without freezing the subject](generate/combine-controls.md) |
| Plan a round or diagnose repeated misses | [Compare a generation hypothesis](generate/plan-a-round.md) |
| Check the request, tool result and provider limits | [Read the run that actually produced the asset](generate/record-a-run.md) |
| Run a local graph, prepare a map or use Midjourney | [Run and compare a ComfyUI graph](generate/comfyui.md), [Prepare and check a ControlNet map](generate/controlnet.md) or [Prepare a Midjourney request and inspect the result](generate/midjourney.md) |
| Change sampling, masking or adapters in a local diffusion run | [Read and compare a diffusion graph](generate/read-a-diffusion-graph.md) |
| Translate an approved treatment into a prompt or graph | [Write one generation request from its sources](generate/write-a-request.md) |

## Before handing off

- [ ] Does the result make the required change while preserving the specified details?
- [ ] Do controls and inputs match the actual model and production route?
- [ ] Do the compared outputs answer the same question?
- [ ] Does the delivery claim match the tool’s actual result?
