+++
description = "When the project generates with a local ComfyUI graph and needs to run it and compare results."
uses = [{ scope = "local", resource_id = "generate" }]
see_also = [{ scope = "external", resource = { package_id = "pale.model-work", resource_id = "artifact" } }, { scope = "external", resource = { package_id = "pale.model-work", resource_id = "hugging-face" } }, { scope = "external", resource = { package_id = "pale.research", resource_id = "source-rights" } }]
official_sources = [{ label = "Run and compare a ComfyUI graph", url = "https://docs.comfy.org/development/comfyui-server/comms_routes" }]

[guide]
exposure = "internal"

[stack]
+++

# Run and compare a ComfyUI graph

Use this overlay for a local ComfyUI graph. Choose the actual engine and graph, then validate node types, inputs, required weights and outputs. An API-format graph is different from a saved editor layout. Check node availability and model architecture against the installed engine. A graph that validates can still depend on a missing weight or return an unsuitable image.

The workspace's image engine supplies the local runner. Request files come from the bound workspace and uploaded inputs from the bound source library. A workflow name selects a bundled template; an explicit graph path is resolved from the calling directory. The ComfyUI runner also owns the video graphs that use the same loader, model table and result handling.

## Bind and inspect the actual engine

Keep the project workspace, source library and engine root in the machine binding, under `[projects.NAME]` and `[engines.comfyui]`, rather than writing one machine's paths into reusable graphs. Reuse a compatible install. Bind a separate environment deliberately when incompatible requirements need it, and record which environment each run uses. Use the engine's own extra model paths for a second model store instead of copying weights between folders.

Run through the selected environment's interpreter. Keep its dependencies isolated from system and user-wide Python, and keep runtime downloads in declared caches. A server helper should use the bound interpreter and put `HF_HOME` and `TORCH_HOME` inside the install. It should accept only a loopback HTTP endpoint, start without requiring the web canvas, and stop only a process it started and recognises. An idle engine can retain models in memory; stop the owned server when its work pauses, leaving unrelated processes alone.

Prefer an existing core node when it serves the job. A custom node runs with the engine's access, so inspect its source, licence, maintainer and dependency changes before installation. Pin the selected revision, install its requirements with that engine's interpreter, restart and run the model-free check before using it. Record the extension revision and local modifications for the node types in the submitted graph. A manager's registry selection is useful for discovery; the actual installed revision still needs recording. [ComfyUI's custom-node guide](https://docs.comfy.org/installation/install_custom_node) explains the supported installation routes.

Treat an engine, frontend, library or extension update as a deliberate change. Keep a body on a stable stack where possible; after an update, run the model-free check and compare one known graph with its retained output before extending the set. A working graph before the update does not establish its behaviour afterwards.

## Pin models and their permitted use

The engine's model manifest records each role's filename, model folder, source revision, size, hash, precision, terms and gated-access state. Alternatives have their own hashes and conditions. Use the manifest's distinct filenames so two generic upstream names cannot overwrite one another. Read the source file index at the pinned revision before a large transfer, then hash downloaded bytes. A matching local candidate still does not prove which file a native loader executed.

Use Model Work's artifact and Hub inspection guides, available through this page's optional reading, for source, revision and access evidence. Read a repackaged model's origin as well as the upload's own terms; conversion or a community copy does not erase upstream conditions. Record which terms apply to the intended model operation and which apply to its outputs.

Research's source-rights guide carries the model-licence worked case in the optional reading. Keep the project's selected licence policy in its workspace; the classes identify questions to check, not a universal choice of model. [Gated access](https://huggingface.co/docs/hub/en/models-gated) is an account action; a metadata read is not acceptance of its terms. Readiness checks report missing, mismatched or inaccessible files without downloading them. Carry existing download authorization forward; a new model or use outside that scope needs its own decision.

Storage, loaded and compute precision are different. An fp8 file may be converted while loading and require more memory than its download size suggests. A text encoder can use a different loader from the diffusion model. Check each on the actual backend, retain its hash and observed precision, and compare the result when changing it. The same seed with different precision or libraries need not produce the same pixels.

## Choose the graph for the job

The templates and `index.toml` under `image/comfyui/assets/workflows/` identify inputs, model roles, adjustable values and the source of each starting setting. Use the current files; the table describes their jobs, not their readiness on this machine.

| Template | Job and required input |
| --- | --- |
| `check-engine` | Upload, scale, save and fetch one image without loading a model |
| `refine-qwen-image` | Refine or enlarge an accepted source frame |
| `edit-qwen-image` | Change a frame by instruction, with an optional second reference |
| `map-edges` | Extract edges from an approved frame at the working size |
| `map-overlay` | Inspect a map over its frame after the separate dimension check |
| `control-qwen-image` | Generate from an edge or depth map using the compatible patch |
| `control-sdxl-union` | Generate from a map with its union type explicitly selected |
| `film-wan-first-frame` | Start motion from an approved still |
| `film-wan-control` | Condition motion on an appearance reference and an authored depth or edge video |
| `still-qwen-image` | Generate from words when the project selects local generation |

Templates that take words compose the world block, shot block and exclusions inside the graph. Fill them from the request file, with node titles identifying the inputs to change. A title must name one node; numeric node IDs can change when a graph is edited. Keep the approved treatment and the actual shot distinct. Copy a template into the project's own workflows before making a project-specific change; a useful shared graph can later be proposed for the template library.

For refinement, keep the accepted frame as the input. The current template starts at `sampler.denoise = 0.3`; increasing it changes more of the image. Around or above 0.5, check especially carefully whether the result is a new frame rather than a preserved one—this is a review cue, not a universal numeric boundary. Three seeds at one denoise can help distinguish a consistent change from variation. Increasing `working size.megapixels` invents new detail; inspect the enlargement as a generated result.

The edit template uses an instruction for what changes and a second reference where needed. Use `--drop 'reference 2'` when that optional node is unused. Its one-megapixel working size preserves proportions in steps of 16; a later refinement can reach the delivery size. The full-frame edit can alter pixels outside the requested detail. For a local correction, prepare a masked project graph and check the outside region rather than assuming the instruction protects it. Keep the mask in the final frame's coordinates and verify its editable polarity, dimensions and colour interpretation, and keep them stable across comparisons.

The Qwen templates use size steps of 16; their refine and edit passes size by total pixels. Control templates take the map's exact dimensions. Use the [ControlNet method](controlnet.md) before running the map, including size-before-overlay checks and recorded strength and range. Do not force every result through another model because of its family name; compare continuity against the approved treatment.

For Wan first-frame motion, the template resizes the input in steps of 32 before its latent node can centre-crop a mismatch. Length is `4n + 1` frames; 121 frames at 24 fps is about five seconds. Check the first and last frames against the source and inspect the whole sequence. In `film-wan-control`, the appearance reference does not lock the first frame: both reference and control video must match `control.width` and `control.height`, and the video must contain `control.length` frames. Set `film.fps` to its authored rate. The 512×288, 17-frame default is a bounded smoke check, not a production-quality choice.

A retained comparison on PyTorch 2.14.0 and ComfyUI `6634bc8288075fe6369dd35a0533491c4aa70c14` found progressive bands and blown-out colour with Wan and UniPC on MPS, consistent with [issue 16573](https://github.com/Comfy-Org/ComfyUI/issues/16573). Changing only `sampler.sampler_name` to `euler` removed the bands in that comparison. Keep this workaround scoped to the affected stack, retain the other settings when comparing, and inspect every frame. It is not proof of every MPS version or final quality, and a project sampler choice does not require modifying the engine.

Masked edits, first-and-last-frame motion and tiled enlargement beyond available memory may need a project graph rather than these templates. Check the required model and nodes; the first-and-last-frame route needs a different, larger film model than this manifest currently supplies. Export a canvas-built graph to API format, title its configurable nodes and check it before use.

## Follow a submission through to its output

Use the project’s existing runner to validate the API graph against the installed node schema, resolve the required models and inputs, submit once and retain the returned prompt ID. Compare the submitted graph with that ID’s terminal history and fetched outputs. A saved editor canvas may already show the next seed. A model filename identifies a requested candidate; caches and indirect loaders can require additional evidence of what executed.

Use `--request` for the recorded request. The runner fills its prompt blocks, then applies explicit command-line overrides; record those differences as the actual attempt. `--set TITLE.INPUT=VALUE` reads JSON numbers and booleans, or text otherwise, and `@PATH` reads text relative to the workspace. `--input TITLE=PATH` reads an image or video relative to the source library. Dropping a node is valid only when the remaining graph does not require its output.

`--check` is an offline structural check; `--object-info` adds checks against a saved node catalogue. `--dry-run` checks the actual project request against the running server without upload, submission or record writes. Neither executes custom-node validation or proves a model-backed result. The separate `comfy_check.py --workflow NAME --binding studio.local.toml --hash` checks a bundled template's model paths and pins and reports live validation when available; it does not include request overrides.

Before upload the runner reserves a unique journal under the workspace's `records/runs/`. It checks input bytes again, saves the attempted graph before submission, records the returned prompt ID, retains terminal history and fetches output bytes. The completed TOML keeps the graph, request, environment, model-candidate evidence, extension revisions, seeds and input/output hashes. Fill its remaining purpose, input-role and verdict fields before setting `complete = true`; the script cannot decide acceptance.

After a timeout or interruption, read the existing run record and engine history before submitting again. Keep unknown submission and partial output acquisition explicit. Fetch original outputs through the runner and keep their byte identities with the run. Read the failing node and message when a graph fails rather than summarizing it as an unexplained failed generation.

The `.journal.json` retains the last saved phase, including refusal, uncertain submission, native error, timeout and partial fetching. A killed client does not cancel the engine, and a disk failure can leave only an earlier phase. Reconcile what was submitted and fetched before retrying. Native history can disappear on restart; retain it while available. Output nodes in history do not identify every node that executed, and `candidate_sha256` is explicitly a local-file observation, not a loaded-model hash.

Masters land under `generated/comfyui/<date>/` in the source library, with hashes and references in the workspace record. The engine's `input/` and `output/` copies are scratch; verify the retained masters before any separately authorised cleanup. An uploaded file in a subfolder may not appear in a load node's top-level picker even though the server accepts it. Inspect the submitted path and native result rather than treating picker absence as a missing upload.

The project owns server startup, endpoint, interpreter, extensions and model storage. Use its configured local route and stop only a process that route owns. A readiness check should not quietly install weights or extensions. Inspect exported PNG metadata: save nodes can embed request text and workflow data. Keep that information with the internal original and remove private fields from a delivery derivative, then reopen the derivative.

[ComfyUI server routes](https://docs.comfy.org/development/comfyui-server/comms_routes) describes queue, history, inputs and outputs. [The API workflow format](https://docs.comfy.org/development/api-development/workflow-api-format) distinguishes executable graphs from editor layouts.

## Worked example: check the engine before using a model

```sh
python3 image/comfyui/scripts/comfy_run.py --check check-engine
python3 image/comfyui/scripts/comfy_run.py --project example --workflow check-engine \
  --input 'input image=check.png' --job readiness --binding studio.local.toml
```

Use a small test image in the project’s source library. Open the result, verify its dimensions and match it with the recorded graph and run. A readiness run of this graph checks image upload, queue, processing and return. Compare a real generation in a separate run with its model, controls, seed and output settings named.

Start a new graph branch when it lets you test a different hypothesis, preserving the earlier graph and comparison. A valid graph and a completed run still return to the asset's owner for selection and the appropriate still or motion review.
