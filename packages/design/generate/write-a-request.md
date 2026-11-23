+++
description = "Use it to write one generation request from the agreed treatment and the shot's record."
uses = [{ scope = "local", resource_id = "generate" }]
see_also = [{ scope = "local", resource_id = "generate/record-a-run" }]

[guide]
exposure = "internal"
+++

# Write one generation request from its sources

Read the treatment the project has agreed to. Keep its recurring medium, materials, light and subjects stable across the set. A shot changes the subject action, camera, framing or state called for by its shot record. The treatment can set how an image and words relate. The shot says where the words read in this particular frame, or leaves the field unused when words sit elsewhere.

Carry the agreed shared treatment and standing exclusions unchanged into each request in the body. Fill the shot from its record, not from memory of the beat. A deliberate treatment revision changes the shared source and names which later requests use it; quietly rephrasing it for each shot introduces another variable. Model, sampler, speed and quality settings belong to the run, while the treatment describes the intended images independently of the tool that makes them.

| Part | Draw it from |
| --- | --- |
| Shared treatment | The agreed appearance, references and standing exclusions. Keep one copy for the body of work. |
| Shot or edit | Its action, scene, point of view, reading plane and anything that must match adjacent frames. Add only shot-specific exclusions. |
| Run settings | Aspect or dimensions, candidate count, model, controls and weights, sampler and seed where available. |

Translate these parts into the tool's dialect. A hosted tool may accept descriptive text and reference images. A diffusion runner can expose positive and negative conditioning, and a graph can give each control its own node. Use exclusions as requirements. Express them through a dedicated negative input when it works, positive wording when it does not, or explicit change and preservation instructions for an editing model.

Use the project's existing request format to refer to shared text and references. Record the selected revision or hashes and the actual resolved inputs. Where the native entrypoint supports source verification, check declared local files before use; a mismatch needs the intended source or an explicit request update. Mark URL references or inputs whose bytes were not inspected as unverified. A working URL or a matching file does not establish permission to use or upload the material. Preserve the proposed request when an owner changes it during a manual run, and [record what actually ran](record-a-run.md).

Use material, medium, light, composition and the selected tradition as style instructions. Apply the project's restrictions on living artists', studios' or brands' names explicitly: when those names are excluded as style prompts, describe the wanted qualities instead. Keep real likenesses, private source material and documentary claims within the project's permissions and purpose. A generated illustration cannot establish that a product behaved as depicted or that a measurement was observed. Keep the page's actual words in the page; a shot may separately call for intentional lettering within its depicted scene.

Write a request a person can picture. Put the subject, its state and action, framing, light and material in a clear order, adjusted to the tool's documented dialect. Name a quality precisely once instead of burying the subject under repeated adjectives. Word order and weighting differ by model; parentheses copied from another runner can be literal text. If a word meant to describe a quality keeps generating an unwanted object—a gallery wall, a paint swatch or a sheet of torn paper—describe the quality directly and compare the result. Save any alternatives or prompt rewriting that the interface resolves before submission; the seed does not necessarily control that expansion.

## Worked example

A plate set uses etched steel, a raking light and approved pump photographs. The next image shows the impeller half removed. Keep the treatment and reference roles, then describe that action, the same camera position and an unoccupied area where the caption sits. Keep the caption in the page. In a graph run, hold dimensions and record the structure input and its strength. If a hosted tool cannot constrain the exact part positions, compare the output with the reference and revise the route before sending more shots.
