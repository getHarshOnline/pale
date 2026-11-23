+++
description = "Use it to identify the reader's job and the destination and expected contents of each authored or generated output."
uses = [{ scope = "local", resource_id = "local-build" }]

[guide]
exposure = "internal"
+++

# Check assembled documentation and generated reference

## Identify the reader and intended output

Start with the reader's job: using a product, integrating or changing it, understanding a concept or looking up an exact detail. Installation and CLI use can be product tasks. Generated reference is a developer-documentation form. Use `documentation` from the Publication manifest for writing and continuity of the reader's task.

For each authored or generated output, identify its destination and expected contents. One source directory may contain several forms; its location alone neither chooses a destination nor permits another copy. In the build result, identify selected pages, companions and generators and what is excluded. Preserve source ownership and revision even when different outputs resolve links to the same source.

Use the documentation owner's supported generation and assembly tools. A configured site renderer covers only the output types it supports. Distinguish selected guidance from an unavailable execution tool.

## Inspect generated content and assembly

For authored pages, check output order, routes, companions and links. For generated reference, identify source and dependency versions, generator executable and version, configuration and execution limits. Inspect its result and artifact before assembly. Compare the destination's symbols, signatures, examples, attribution, navigation and links against that artifact. Identify missing generators, unsupported constructs and unchecked links at their actual scope.

Exercise the complete chosen set, a missing page or companion, and an invalid or absent generated artifact. If modules share a destination, test collisions and ensure failure of one module leaves the incomplete set visible. Inspect the corrected output tree and navigation from the actual assembly. Keep configured routes, local assembly, hosted rendering and publisher approval distinct. The owner supplies generator and formats; the publisher decides release.

## Give each reader the depth they came for

Follow the guide from its normal collection or journey and check outline, reference and return paths. Keep product tasks distinct from developer API or architecture reference. A documentation category may use the site's ordinary articles, collections and sequences. Check depth, breadcrumbs and navigation without turning an incidental implementation tool into the subject of a product page.

Check generated reference against the provider and version the guide uses. After a timeout, inspect for partial tables, stale results, omitted pages and blank reference sections before treating the page as usable. Keep a reachable source path and label unavailable sections when the publisher permits a partial result.

## Worked example

A developer manual needs two authored pages, a companion example and generated API reference. Declare their inputs, inspect generator output, assemble the destination, then follow its real navigation and links and read a generated signature. Remove one input in a test build and verify that the incomplete result is visible.
