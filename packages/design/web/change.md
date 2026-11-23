+++
description = "Use it to find the producer, contract or boundary that should own a web change before editing."
uses = [{ scope = "local", resource_id = "web" }]

[guide]
exposure = "internal"
+++

# Find the source that should own a web change

## Keep each decision with its owner

Trace the semantic model, component or template contract, token source, local composition and delivery configuration separately. A shared rule belongs at its real producer; identity variation belongs at the supported variation boundary. Do not make shared code infer a consumer from its name, hostname or path when an explicit semantic input is available. A new mechanism can require a contract extension rather than another data flag.

Use documented blocks, imports and lifecycle methods from the pinned version. A wrapper can bind tokens, inputs, a still, cleanup or a stable interface; rebuilding the library's own timeline, loader or observer creates another implementation to maintain. Similar fragments need not share a reason, and two that look alike can represent different decisions. Keep them separate until the reason for sharing is clear. Extract shared meaning and behavior, not a shape that immediately needs unrelated switches. A single consumer can still justify a lifecycle, test or adapter boundary. Keep intentionally different decisions with their own rationale.

## Decide which implementation seam the change belongs to

| Decision | Owning seam |
| --- | --- |
| Shared color, size, timing or control state | Design roles and the shared control that consumes them |
| A layout threshold | The layout source and the query or build variable that actually uses it |
| A reader-facing field, count or relationship | The content model and the template that presents it |
| An enhancement on one figure | The figure's mount, state and cleanup contract |
| Story beats, holds or selected media | The editable sequence and its inputs |
| An identity's type, light or materials | The project's identity and its token or theme implementation |

Keep the style from depending on a script having run. Give behavior its own selectors or event hooks so changing a class for layout cannot break a control. Keep a reader-relevant state available in the rendered surface. A timer or animation handle can stay private to the module. When modules must agree, expose a specific state or event rather than letting one inspect the other's internal object.

Keep story beats, holds and frame ranges in the editable sequence so a timing change does not require rewriting its runtime. Name code after the actual system concept or library primitive; a private synonym makes the next reader learn the same mechanism twice. Remove old selectors, files and tests within the authorized change once the verified replacement has reached their consumers. A compatibility path can remain where a real consumer still needs it.

Trace the next plausible change through this structure. A contract change may correctly touch a template, stylesheet and module; the problem is duplicated ownership or needless coupling, not the number of files. If one value must be corrected in two places, find its producer. If a shared component branches on a site name, give the difference a meaningful input or extend the contract. If a wrapper recreates the library's own feature, use its documented primitive. These checks explain why a smaller-looking abstraction can still make the next change harder.

## Worked example

A new badge is missing on one template. Find whether the input record lacks the value, the template has the wrong branch or the style conceals it. Correct the producer responsible for the failure. Leave the display of other records to their existing component. Rebuild and inspect the page that was affected.

## Sources

- [Sandi Metz, The Wrong Abstraction](https://sandimetz.com/blog/2016/1/20/the-wrong-abstraction): why similar shapes with different reasons can cost less than an abstraction filled with switches.
- [Martin Fowler, Yagni](https://martinfowler.com/bliki/Yagni.html): the cost of speculative capability, distinct from maintaining useful structure.
- [MDN JavaScript modules](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Modules): explicit imports and exports at module boundaries.
- [MDN cascade layers](https://developer.mozilla.org/en-US/docs/Web/CSS/@layer): deliberate ordering of shared and local styles instead of escalating specificity.
