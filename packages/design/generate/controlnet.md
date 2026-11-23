+++
description = "When the generator needs geometry or pose from a prepared control map."
uses = [{ scope = "local", resource_id = "generate" }]
official_sources = [{ label = "Prepare and check a ControlNet map", url = "https://huggingface.co/docs/diffusers/en/using-diffusers/controlnet" }]

[guide]
exposure = "internal"

[stack]
+++

# Prepare and check a ControlNet map

Use this overlay when the generator needs geometry or pose from a prepared map. Choose the map for the relation you need to keep. Edges can keep silhouettes while holding unwanted texture. Depth can keep foreground and background while losing a precise contour. Pose keeps joints while leaving room for shape and clothing changes.

Match the map and its model to the base model family and workflow. A weight trained for one family can fail in another. Read the preprocessor, output shape, supported scale and the steps where control applies. A combination of correct inputs can still overstate one constraint.

## Choose what the map holds and leaves free

| Map | Structure it carries | What can still change | Typical source |
| --- | --- | --- | --- |
| Hard edges | Outlines, including texture edges | Material, light and colour | Edge detector |
| Soft edges or line art | Main contours with less texture noise | Detail within shapes | Line extractor or drawing |
| Straight lines | Architectural and perspective lines | Surfaces and objects between them | Line-segment detector |
| Scribble | A rough compositional gesture | Most shape and detail | Hand sketch |
| Depth | Relative near and far space | Interior contours and surface detail | Depth estimator or a 3D depth pass |
| Surface normals | Surface orientation and form | Colour and material | Normal estimator or a 3D normal pass |
| Pose | Supported body, hand and face landmarks | Appearance, clothing and setting | Pose estimator or an authored skeleton |
| Segmentation | Regions and their semantic classes | The appearance within each region | Segmenter or correctly encoded painted regions |
| Tile | Coarse layout and colour | Fine detail, which the model redraws | The source frame |
| Inpaint | A region selected for change | The masked content | Frame and mask |

These are intended constraints, not exact-pixel guarantees. Inpainting still needs an outside-mask comparison or compositing when untouched pixels must remain identical. Tile redraws detail; treat its result as a new image to inspect, not a faithful enlargement of a flawed original. A pose map leaves clothing freer than an outline does. For a building, straight lines or depth avoid preserving unwanted signage and weathering. Choose the least structure that keeps the invariant; increasing strength cannot make a wrong map type carry the right one. The [ControlNet 1.1 reference](https://github.com/lllyasviel/ControlNet-v1-1-nightly) describes the expected inputs and limits of its individual types.

## Match the control's form to the model and loader

| Family | Forms and choices to distinguish |
| --- | --- |
| Qwen-Image | [DiffSynth patches](https://huggingface.co/Comfy-Org/Qwen-Image-DiffSynth-ControlNets) for Canny, depth or inpainting; a union LoRA for Canny, depth, pose, line art, soft edges and normals; and [InstantX ordinary control models](https://huggingface.co/InstantX/Qwen-Image-ControlNet-Union), whose union supports Canny, soft edges, depth and pose, with inpainting in a separate model. A patch, LoRA and ControlNet use different loaders. The [ComfyUI workflows](https://docs.comfy.org/tutorials/image/qwen/qwen-image) show those forms. |
| SDXL | [Xinsir's union model](https://huggingface.co/xinsir/controlnet-union-sdxl-1.0), including the promax file's twelve types and editing modes, or a suitable single-type model. [MistoLine](https://huggingface.co/TheMistoAI/MistoLine) accepts varied line-art styles. Read the exact [Control-LoRA terms](https://huggingface.co/stabilityai/control-lora) rather than inferring them from the base model. |
| SD 1.5 | ControlNet 1.1 lists eleven production-ready models and three experimental ones: shuffle, instruction edits and tile. Test the chosen output size and behaviour; those labels do not establish suitability for this project. |
| Flux | [Canny](https://huggingface.co/black-forest-labs/FLUX.1-Canny-dev) and depth variants have their own model-use conditions. Permission to use an output does not itself permit every use of the model that produced it. |
| Z-Image | The [Turbo union control patch](https://huggingface.co/alibaba-pai/Z-Image-Turbo-Fun-Controlnet-Union), whose card declares Apache-2.0, uses its compatible patch loader; it is not an SDXL control file. |
| Wan film | [Wan2.2 Fun 5B Control](https://huggingface.co/alibaba-pai/Wan2.2-Fun-5B-Control), whose card declares Apache-2.0, can combine an appearance reference with depth or edge video. An appearance reference does not lock the first frame; use and check the separate first-frame route when that is the requirement. |

Read the base model, control, adapter and detector terms at the selected revisions. For example, the Qwen DiffSynth patches and Xinsir union cards declare Apache-2.0, while MistoLine declares OpenRAIL++; a shared file format says nothing about those differences. Detector weights need their own check: [Depth Anything V2 Small](https://huggingface.co/depth-anything/Depth-Anything-V2-Small) declares Apache-2.0, while [V2 Large](https://huggingface.co/depth-anything/Depth-Anything-V2-Large) declares CC-BY-NC-4.0. Record model size and applicable permissions, including separately granted rights, before choosing a route for paid work. Unknown terms remain unknown, not permission.

A research checkout can explain training and detectors without being the selected runtime. Record the actual runtime, loaders and extensions. Add a preprocessor extension only for a needed map type, pin it and its dependencies, and account for detector downloads on first use. Core and extension capabilities vary by version; inspect the selected nodes rather than assuming every detector is installed.

## Author, retain and check the map

Make a map from an authorised reference, an approved frame, a storyboard, a compatible authored pose or a 3D depth, normal or outline pass. A public download does not establish rights for every transformation or likeness use. Keep the map as a reusable source asset with its path, hash and making record in the project's existing library; do not generate it inside a run and discard it.

Make it at the exact working width and height of the pass it controls, in the size step required by the selected model and node. A wide and a tall variant are different compositions: re-author rather than stretch one map. Record detector identity, resolution and thresholds. Remove unwanted lettering, logos, ornament and stray texture; keep a required text area clear only when the shot calls for it. Retain both the detector output and cleaned derivative, with the edit and its reason.

Check dimensions before looking at an overlay: a blend or resize node can silently crop a mismatched map and conceal the fault. Use the workspace image engine's map checker first. Give it the map, the target frame and the selected workflow's size step: a pass whose step is 32 needs 32, not a default of 16. The checker reads PNG, JPEG or WebP dimensions, requires positive sizes, checks divisibility and compares the exact frame size. It does not decode the full image or establish that its geometry is right. Inspect the overlay next, then run one image at the planned schedule before using the map across the set. Keep the original and cleaned maps alongside that result.

## Choose a starting schedule, then inspect its effect

Strength controls how strongly the map influences the run. Where the selected node supports start and end fractions, starting early can establish structure and releasing before the end can let surface detail settle. A patch that exposes only strength cannot accept a schedule invented for another loader.

| Trial | Strength | Start | End | How to use the value |
| --- | --- | --- | --- | --- |
| InstantX Qwen-Image union control | 1.0 | 0 | 1.0 | The maker’s Canny example uses conditioning scale 1.0; compare the intended map and material rather than treating it as universal. This full-range starting trial matches the apply node's 0–1 range in the [ComfyUI template](https://raw.githubusercontent.com/Comfy-Org/workflow_templates/main/templates/image_qwen_image_instantx_controlnet.json); check any linked overrides in the actual graph. |
| SDXL union through a node with all three controls | 0.8 | 0 | 0.8 | A worked starting trial that releases control before the end, not a model guarantee or provider default. |
| Qwen-Image patch exposing strength only | 1.0 | Unavailable | Unavailable | The [ComfyUI patch template](https://raw.githubusercontent.com/Comfy-Org/workflow_templates/main/templates/image_qwen_image_controlnet_patch.json) sets this strength on `QwenImageDiffsynthControlnet`; it exposes no start or end fractions. |

The [InstantX model card](https://huggingface.co/InstantX/Qwen-Image-ControlNet-Union) identifies its supported maps and example settings. [ComfyUI’s ControlNet guide](https://docs.comfy.org/tutorials/controlnet/controlnet) explains the model, preprocessor and apply workflow. Keep the selected version and actual settings with the submitted graph.

Compare a traced or flattened result with a lower strength or earlier release. Full-range control is valid when it preserves both the intended structure and material. Compare drifting structure with map alignment, detector compatibility and earlier or stronger control before blaming the seed. If combining maps, give each its own node, intended preserved feature and supported schedule. Set the union map type explicitly where the node offers it. Test each contribution and the complete stack; record a reason and check continuity when changing a schedule across a set.

Complementary maps may constrain the same subject. Use regional masks when separate subjects need separate controls; overlap alone is not a failure. For example, depth can establish the room early while pose holds a figure longer. Align, mask or remove a conflicting contribution and retest the whole stack. Similar strengths in one documented regional example are not a universal balancing rule. Keep every node and setting in the submitted graph rather than describing an unrecorded stack in prose.

## Worked example: hold camera depth and change surface texture

Use the same output width and height for the depth map and target. The same ratio alone leaves scale and alignment ambiguous. Inspect which side of the map means near. Run one trial at the original seed with depth alone. Add an edge map only if the silhouette drifts. When the second trial reproduces an unwanted edge within the edited area, shorten its influence or mask out that edge.

Read the map checker's own help for its installed checks and parameter names. Inspect the map and the output after running. A valid map can still encode the wrong geometry.
