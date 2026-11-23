+++
description = "Use it to produce the variants a surface actually needs and to inspect the delivered bytes after resizing, encoding and integration."
uses = [{ scope = "local", resource_id = "assets" }]
see_also = [{ scope = "local", resource_id = "presentation" }]

[guide]
exposure = "internal"
+++

# Check the required asset variants where they will appear

## Deliver the necessary variants and inspect their bytes

Produce the resolutions, crops, backgrounds, modes or formats the consuming surface actually needs. Avoid an automatic matrix of unused variants. Keep masters and derivatives identifiable, with their relevant input and revision relationships. Documentary originals and original evidence remain unchanged; a retouched illustration must not quietly replace them.

Inspect the delivered asset after resizing, encoding, metadata changes and integration. Check alpha edges against its real background, color and tone after conversion, meaningful detail after compression, and loading cost in the target application. For an asset that communicates information, use [Preserve meaning across presentations](../presentation.md); the application also supplies the appropriate accessible equivalent.

Keep source/run provenance with its owner. Exports may embed private prompts, graphs, paths, account names or source identifiers. Inspect and remove fields outside the destination's permitted disclosure, preserving internal originals and any required attribution. Reinspect the resulting derivative. Generated, visually checked, owner accepted, exported and published are different states. For qualification of a production route and export claims, select `image-workflow` from the Design manifest.

When delivery needs several treatments, list the actual uses by crop, grade and form. Check the crop against subject, reading order and useful movement. A different narrow treatment can be a reframe or new composition. Design the dark grade from the dark palette and light. If the image depends on a movement, give reduced motion a composed still that carries the relationship. Keep separately supplied photography and recorded footage as masters. Store the repeatable export step with the project so a new derivative can be made without modifying the originals.

## Worked example

An illustration serves a wide article and a narrow card. Crop and encode each for its destination, keeping the defining subject and useful detail. Open the files, then read them in the page at real size. Check alpha edges on both grounds, compare color after conversion, and inspect the metadata for private paths or prompts. Document the derivative and keep any required attribution with it.

| Use | What to make and check |
| --- | --- |
| Wide article in light mode | A full illustration with its source and caption in the page. Verify both at reading size. |
| Narrow card in light mode | Reframe the subject and keep the caption legible. Check that the crop preserves the relationship. |
| Narrow card in dark mode | Use the same crop and grade from the dark ground and light. Check transparent edges against that ground. |
