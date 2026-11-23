+++
description = "Use it to prepare a 3D model for the renderer it will actually run in, starting from the destination."
uses = [{ scope = "local", resource_id = "assets" }]
see_also = [{ scope = "local", resource_id = "figures/budget-a-scene" }]

[guide]
exposure = "internal"
+++

# Prepare a model for its actual renderer

Choose the destination first. A production scene may keep cameras, lights, rigs and procedural geometry even when its delivery is a still or encoded film. A live figure needs a reason to deliver geometry and a measured budget. Compare what a made image or pose set would preserve before making that choice. See [Choose and budget a drawing layer](../figures/budget-a-scene.md).

1. Block out in the target renderer's units. Set the origin, orientation and coordinate system for placement and rotation.
2. Model the form at the scale where it will be seen. Preserve editable construction in the master and clean the export mesh. Check the silhouette and topology where shading or deformation exposes them.
3. Unwrap mapped meshes. Keep UV shells separate where you bake light. Build materials the destination can reproduce. Test transparency and color interpretation instead of assuming the authoring viewport matches.
4. Bake static light, occlusion or fine detail when that preserves the image. Keep moving light, deformation and view-dependent response live where the viewer needs them.
5. Export a delivery copy. Name the meshes and tracks the caller uses and avoid relying on export order. Omit unused lights, cameras, shape keys, tracks and orphaned data from the delivery copy. Keep them in the production master if they are used there.

For a web model, a binary glTF container can carry the geometry and textures together. Check that your target supports its extensions. Use compression only when its decoder will reach the viewer and the decode cost fits. Size textures for the screen pixels they cover. Share channels where appropriate and add only the distance-based detail levels the scene actually uses. Give the figure a composed still for reduced motion, missing script and failed graphics. Validate the exported geometry, origin, color, light, bounds, animation and interaction in the target renderer. Measure its memory and frame cost there.

Keep inputs, licences and repeatable export settings with the editable source. Identify a reconstruction as a model even if it depicts a real object. It cannot replace a photograph or measurement where the viewer needs an original record.

## Worked example

A delivery shows the same pump in an encoded clip and an interactive exploded view. Keep the film rig and its animated light in the master. Bake the interactive base's ambient shading. Export the named parts and only the expansion track in one binary container. Check orientation and scale after loading it in the page, then test the same track with the fallback still and keyboard controls. If the phone cannot draw the busy frame on time, reduce the delivery meshes and texture sizes and measure again. Leave the master editable.

- [glTF 2.0 specification](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html)
- [Blender glTF import and export](https://docs.blender.org/manual/en/4.2/addons/import_export/scene_gltf2.html)
- [Khronos texture container](https://www.khronos.org/ktx/)
- [glTF Transform functions](https://gltf-transform.dev/functions)
