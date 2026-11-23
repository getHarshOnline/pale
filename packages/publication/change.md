+++
description = "Find whether a publication change belongs in its source, workspace, theme or engine, then check the affected output."
see_also = [{ scope = "local", resource_id = "write" }, { scope = "local", resource_id = "local-build" }, { scope = "external", resource = { package_id = "pale.engineering", resource_id = "change" } }, { scope = "external", resource = { package_id = "pale.design", resource_id = "inspect" } }]

[guide]
+++

# Find the owner of a publication change

Start from the requested result and the publication's actual source, configuration, theme and engine. Read their instructions and the affected files. Content carries the facts and writing; the workspace chooses their use; the theme supplies arrangement, layout and style; the engine supplies reusable publishing behaviour. Resolve which part produces the problem before editing it.

## Match the change to its owner

| What needs to change | Where it belongs | What establishes the result |
| --- | --- | --- |
| Meaning, claims, examples, relationships, authorship or an authored figure | The content's owning source. Product-native developer documentation stays with that product. | Read the source support and intended meaning, validate the document and inspect its actual presentation. Check identity, canonical links, discovery and exclusion in visible and machine-readable output where affected. |
| Source selection, publication and chapter composition, routes, theme selection, tool bindings or destinations | The publication workspace. Values and paths come from its owners. | Parse the declarations and resolve their sources, pins and providers. Check the affected publication first, then the complete configured set for route and output collisions, unintended exclusions, isolation and shared-policy effects. |
| Templates, styles, components, interactions, token use or rendering a supported page family | The theme or its shared base, following the world's brief and design system. | Render the affected families and states through the actual engine. Inspect desktop and phone, light and dark where supported, keyboard and focus, reduced motion and reading without scripts. Check shared consumers and compare with the approved design. |
| Parsing, validation, semantic compilation, CLI behaviour, output contracts, deterministic builds or release mechanics | The publishing engine. | Exercise the changed production path and its failure cases with native tests. Compare human and machine output where affected; test deterministic output, and interruption, retry and read-back where persistence or delivery changed. |

A route missing from a site can have several causes. If the workspace omitted the source, fix its selection. If the engine failed to produce a declared route, fix compilation. If the route exists but the navigation control does not expose it, trace the theme's input and rendering. If the link names the wrong subject, correct the authored source. Editing the generated HTML would leave each cause intact.

## Carry the work across the boundaries

For writing, use [composition](write.md) to establish the reader, source support and voice. Before drafting, establish the object's owner, native home, page purpose, lifecycle, canonical location, relationships and expected reader destination. Keep these facts in the existing brief or owning source; a second dossier is unnecessary when that account is adequate. Before drafting, also list the claims to be made, the source for each and what the sources do not allow the page to infer. Distinguish the subject of a page, a contextual mention, collection membership, editorial relatedness and provenance. The publication's current schema supplies their spelling and allowed combinations.

Two treatments of the same subject are two objects when their claim, audience or owner differs, and an identical object has one home that other pages link to. Never copy a project's own documentation into a publication's source; the product keeps it and the publication links to it. Make a correction or withdrawal at the owning source first, then in each page that projects it.

For code or configuration, use Engineering's change method from the related reading, including its before-and-after caller trace and CodeGraph checks where available. Preserve shared behaviour in its existing owner. A missing engine capability needs an engine change; a workspace flag or one-off theme workaround cannot establish it. In the same way, a concept the shared theme lacks goes back to the theme's owner: a new palette, page family or component fork added to make one fixture pass hides the gap for every other consumer. Keep reusable templates free of one publication's private facts, domains and accounts. Resolve current commands, schemas and tools from the owning repository rather than copying them into this guide.

When another owner is needed, name the required input or behaviour and the consumer that needs it in the existing task or reply. Continue independent work. A new binding, compatible render or successful build establishes only that part of the path; the publisher still controls release.

## Check the assembled publication

Use [local-build](local-build.md) to identify the inputs and output being checked, and Design's rendered inspection method from the related reading for browser claims. Follow changed assets, scripts, fonts and links through the real build to what the reader loads. Compare the requested publication set before and after the change so a successful build cannot hide an omitted site or page. A preview must refer to the inspected build.

Rerun the original failing case, then check the affected shared consumers. Preserve the selected design and the source's qualifications across the result. Report source checks, builds, browser observations and missing checks separately; none alone proves all the others or grants publication approval.

## Before handing off

- [ ] Does each change live with the part that owns its meaning or behaviour?
- [ ] Does the assembled result preserve the requested content, publication set and approved design?
- [ ] Does the reply distinguish verified results, unrun checks and any remaining owner action?
