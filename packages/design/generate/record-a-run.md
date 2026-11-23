+++
description = "Use it to read the execution that actually produced an output, including the submitted graph and its prompt identity."
uses = [{ scope = "local", resource_id = "generate" }]

[guide]
exposure = "internal"
+++

# Read the run that actually produced the asset

## Observe the execution that produced the output

Use the host, established library, skill or tool interface. Do not build another wrapper merely to put a Package in the path. For ComfyUI, distinguish the editable canvas from the executable API graph, retain the submitted graph, and reconcile its prompt identity with history. The saved canvas may already contain the next seed. A service answering, a graph validating and a model-backed job completing establish different capabilities; a small model-free job can isolate transport and output acquisition before investigating model loading.

Observe the selected interpreter, engine and extension versions, device, precision and relevant model/input references to the extent the route exposes them. Storage precision, loaded precision and compute precision are different. A candidate file's hash or a model name in the graph does not establish what the loader executed: alternate paths, caches and indirect loaders can intervene. Mark unavailable evidence as unknown. A seed with different kernels, libraries, devices or hidden provider changes is not an exact reproduction guarantee.

Keep enough evidence to connect the actual request, inputs, run identity, outputs and review. Retain failed and uncertain attempts when they matter to diagnosis or retry safety. A timeout or disconnected client does not prove cancellation; reconcile the job before resubmitting. Fetch original delivered files rather than substituting screenshots or re-encoded copies. Preserve originals before any derivative processing. Reuse existing records; this guidance requires no new universal run schema.

Installation, dependency and server management remain native. Add a model, extension or background service only for an actual capability need, within existing authorization. Check the selected source revision, compatibility and applicable terms; detector weights can differ from the control model. Readiness checks should not silently install or download dependencies. Follow the project's selected division of agent operation and human decisions, and capture what actually ran. Keep provider access conditions in the applicable overlay, including any conflict with that selected route; a service restriction is not a universal owner policy. Stop at an account challenge or bot check and hand it to the person rather than bypassing it.

## Native references

- [Diffusers image-to-image](https://huggingface.co/docs/diffusers/en/using-diffusers/img2img) explains strength and pipeline behavior; use the selected version's API.
- [Diffusers ControlNet](https://huggingface.co/docs/diffusers/en/using-diffusers/controlnet) explains compatible controls, scale and combined inputs.
- [ComfyUI server routes](https://docs.comfy.org/development/comfyui-server/comms_routes) owns submission, queue, history and output interfaces.
- [Midjourney terms](https://docs.midjourney.com/hc/en-us/articles/32083055291277-Terms-of-Service) describes that service's access conditions.

## Record the change that produced each kept result

Write the record as the run happens, using the existing production record. A provider may stop exposing a seed, request or intermediate result later. Keep these fields when the selected route exposes them:

| Part | What connects the result to its cause |
| --- | --- |
| Purpose | Asset, shot or variant, and whether this is a study, style test, keyframe, edit, motion run or enlargement. |
| Request | Proposed source and revision, exact submitted text and parameters, any provider rewriting, and owner changes. |
| Inputs | Reference files and roles, hashes where available, masks, maps, preprocessors, control strengths and ranges. |
| Execution | Interface, job identity, model and adapter identities, graph or command, sampler, schedule, seed, dimensions and edit strength. |
| Environment | Runner and node versions, hardware, libraries and relevant precision, distinguishing declared settings from observed execution. |
| Outcome | Original output, run state, checks, selection or rejection reason, and whether a rerun was actually compared. |
| Human decisions | What a person selected, arranged or changed, and any acceptance decision with its scope. |

Keep the proposed request and actual execution separate when the owner changes the prompt or settings. Those changes can be legitimate exploration; record them without rewriting the proposal to pretend it produced the result alone. Preserve original run records and append later observations through the project's existing record surface. If a field is unavailable or was lost, say so. An asset that cannot be repeated can still be useful, but the next editor needs to know before planning a regeneration.

For a local graph, keep its submitted form, engine and extension versions, model and input identities, sampling and precision choices, and the observed result. For an edit, record the original, mask or region and which pixels should be unchanged. For a variant, record the parent and changed constraint. For film, record first or last frame inputs, motion controls and the timeline or shot that uses the result. Separate the generation from the later resize, transcode, crop or composite.

For motion, include duration, frame rate and the full-sequence check, with reverse playback only where the destination needs it. An authored grade or recomposition has its own checked master; delivery size, format and compression conversions point back to their source and record the transformation. Finish a body on a stable provider/model version where the route allows it. If that changes, identify the assets on either side and retest continuity before extending the body.

Keep source references, sensitive prompts, private URLs and provider responses with their owners. An output file can still contain metadata from those inputs after it has been edited. Inspect the actual derivative before sending it. Keep required attribution with the delivered form and keep the unmodified source and fuller record internally. Record which terms and permissions apply to the input, model and destination where they matter.

Use project-relative references or the workspace's source roles where possible, so machine paths and account-bearing filenames do not leak into public credits. Where required by the destination, attach supported provenance metadata to the final derivative after transformations and verify it survived delivery. Embedded metadata is a useful companion to the source record, but pipelines can strip it. Record the applicable model, adapter and input terms and any owner decision they require; a generation log records human and machine contributions without itself settling legal authorship or permission.

## Worked example

An image tool reports that a graph was queued. Follow its job ID to the result, confirm the model and graph used and open the output. If history is missing or the provider fell back to another model, keep that run separate from the planned comparison. Record the conditions that matter for rerunning without copying prompts or private paths into the deliverable.
