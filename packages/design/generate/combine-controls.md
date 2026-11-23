+++
description = "Use it to choose structural maps such as edges, depth and pose for the structure they carry without freezing the subject."
uses = [{ scope = "local", resource_id = "generate" }]

[guide]
exposure = "internal"
+++

# Combine structural controls without freezing the subject

## Use structural guidance without freezing the wrong thing

Choose a map for the structure it carries. Hard edges can retain unwanted lettering and texture; softer contours can leave material free. Depth carries spatial ordering, pose carries a skeleton, and straight lines can carry architectural perspective. These controls do not independently prove subject identity or geometric accuracy. A map from an estimator contains estimates; an authored scene can supply more deliberate structure.

Retain a needed map as a source asset, including its origin and meaningful detector settings or edits. Remove details the new composition must be free to change. Check dimensions, proportions, alignment and the selected node's resize/crop behavior before trusting an overlay: the overlay can itself hide a mismatch. Recompose for a changed aspect when necessary. Verify that the base model, control, detector, map encoding and loader are compatible.

Test a control before multiplying a set. Start from the selected model's documented example, then tune its strength and, where the implementation supports them, its start and release. Keep the source of those starting values with the run. A traced, flattened result may be overconstrained; drifting structure may be weak, misaligned or incompatible. Compare a lower strength or earlier release for the first symptom, and inspect the map and model pairing before raising strength for the second. A full-range control can be appropriate when that model's example and the observed result support it. Some patches expose strength without a time range. Multiple controls can complement or conflict; inspect their individual effects and their combined result. Set the intended map type explicitly when the union-control interface supports that choice. Record the settings that actually ran.

Aligned pose, depth and subject references can constrain different qualities of the same subject. They do not have to occupy separate regions simply because they overlap. Use regional masks when independent subjects or conflicting instructions need separation, then inspect joins and the combined frame. Each additional control costs execution time and adds another input to reproduce; remove it when it holds no stated invariant.

## Worked example

Hold the camera and main perspective with depth. Add a local mask around the object that changes. Normalize dimensions and check depth direction against the runner’s expectations. If the source edges recreate an old texture inside the mask, remove that edge constraint and compare a new pair at the same size and seed. The geometry that matters stays stable while the material moves.
