+++
description = "Compare artifact bytes, content, appearance or interactions according to the claim being checked."
uses = [{ scope = "local", resource_id = "local-build" }]

[guide]
exposure = "internal"
+++

# Compare artifacts for the claim being checked

## Compare a rebuild with the saved original

Keep the original, exact rebuild inputs, renderer and options identifiable. Decide the required comparison before running it. Complete digests establish byte identity. Extracted-content comparison needs its extraction method and coverage stated. For visual equivalence, render both with the same settings and identify pages, viewports or states inspected. Check destinations, internal references, accessibility structures and interactions when the claim depends on them.

Vary the layers the check claims to cover independently. Metadata can change bytes without changing appearance; graphics can change while extracted text matches; a link target can change while its wording and appearance remain identical. In the result, distinguish these comparisons and uncovered layers. A visually matching rebuild retains its own identity and origin alongside the original.

## Find why two builds differ

Start each run in fresh output with the same declared inputs, versions and environment. Compare file inventory and bytes. Compare rendered model, content, assets and navigation where they can diverge independently. Trace differences to source, time, randomness, undeclared environment, iteration order or an incomplete cache key. When a template promises stable order, try the same unordered object with a different key order.

For an intentional change, compare the affected layer. Identical screenshots leave links, reading order and metadata unchecked. Different bytes can preserve those layers, but a byte-stability requirement still needs a byte comparison. Inspect machine-facing output such as sitemaps or search indexes when affected. Identify a candidate differing only in an approved layer and the conditions under which that finding holds.

## Worked example

Two exports look identical but a link leads to a different address. Compare rendered pages for appearance and extracted destinations for behaviour. A metadata edit can change the digest without changing text or appearance. Report each tested layer and keep both artifacts identifiable.
