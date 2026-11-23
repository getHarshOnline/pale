+++
description = "Use it to save the graph that ran with its inputs and output and compare two graphs by what changed."
uses = [{ scope = "local", resource_id = "generate" }]

[guide]
exposure = "internal"
+++

# Read and compare a diffusion graph

Save the graph that ran with the inputs and output. The graph's node types, values and connections establish what the runner submitted. Editor state and embedded image metadata can differ or be stripped. Pin the runner, extension, model and adapter versions. Record file hashes where the runner names models by filename. Inline alternatives expanded before submission are another source of change. Save the actual expanded text or write it out before the comparison.

| Change | What to check |
| --- | --- |
| Model | Learned tendencies, resolution and compatible control models. Changing it calls for a new style comparison. |
| Sampler and schedule | How noise resolves and what kind of detail remains. Some samplers add noise at each step. |
| Steps and guidance | Extra steps can stop adding value. High guidance can burn contrast or detail, and low guidance can drift from the request. |
| Resolution | Frame proportions and a size the model handles. Check any implicit crop, stretch or resize. |
| Seed | Starting noise. Hold it when isolating one change and vary it when exploring. |
| Adapter stack | Order and weight. If weights are separate for the drawing model and text encoder, record both. Check each licence. |

A negative input changes a run only when the model and sampler use it as conditioning. A distilled model may skip that pass. Check the selected model's instructions. In that case, write the needed result in positive terms and use its supported controls. Combining conditionings by concatenation or averaging can change the result. Keep the combination with the graph.

For image-to-image, low denoising can preserve a frame while changing its surface. High denoising can remake it. Runners count steps and strength differently. Record the pair with the runner. A mask limits the proposed edit. Compare the decoded pixels outside it and composite the edit over the original when they must remain unchanged. Check padding and softness at the join. Outpainting invents new content. A refiner or generative enlargement can invent detail and tiled passes can shift texture or color between regions. Inspect the whole frame. Repair a flaw before enlarging it.

A seed cannot ensure a repeat across hardware, precision, library versions or decoding methods. Record the source of the initial noise. Where the tool supports it, a fixed CPU generator may give consistent starting noise across devices. The sampler can still draw further noise on the device. Tiled decoding can produce different pixels from whole-frame decoding. Derive a pixel-preserving variant from the approved file.

## Worked example

A generated plate has the right object, but a reflection is too hard. Hold model, sampler, seed, dimensions and adapter weights. Use a soft mask on the reflection with low strength, then check the edited area and its edge at full size. Keep the unaffected pixels from the approved plate if the decoded output shifted them. Compare a second run after a runner update before using it on the rest of the plates.

- [Diffusers image-to-image](https://huggingface.co/docs/diffusers/en/using-diffusers/img2img)
- [Diffusers inpainting](https://huggingface.co/docs/diffusers/en/using-diffusers/inpaint)
- [Diffusers reproducibility](https://huggingface.co/docs/diffusers/main/en/using-diffusers/reproducibility)
- [Diffusers adapters](https://huggingface.co/docs/diffusers/en/tutorials/using_peft_for_inference)
