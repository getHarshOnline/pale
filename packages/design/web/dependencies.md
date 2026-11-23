+++
description = "Use it to trace which sources, exports, routes and modules the build actually ships and loads."
uses = [{ scope = "local", resource_id = "web" }]
see_also = [{ scope = "local", resource_id = "web/navigation" }]

[guide]
exposure = "internal"
+++

# Load the dependencies and assets the page uses

## Deliver the chosen dependencies and resources

Distinguish a source file, declared export, selected resource, emitted route, loaded module and executed behavior. Trace required input and import closure under the build's actual path bases. A file need not ship because it sits in an assets folder. A supported browser feature can still fail the build parser. Inspect emitted URLs and referenced files, including CSS and font references.

Preserve shared module identity where required. A script tag beside an import can change execution or duplicate an independently bundled copy; inspect the actual resolution. Supporting renderers, controllers and passes can form one accountable stack. Several package names alone do not establish redundant engines. Follow [Use a view transition without losing navigation](navigation.md) when a dependency takes over links or introduces prefetching.

## Check the whole cost of the capability

Take the lightest sufficient route and explain why a lighter one does not serve:

1. The platform's existing element, attribute, CSS feature or API.
2. A small local implementation when the behavior is simple and its edge cases are understood.
3. A focused, pinned library for a job with substantial correctness work.
4. A larger library when its full model is the required capability, such as a scene graph or search index.

Do not rebuild focus management, locale formatting, color conversion, text shaping or decoding merely to remove a package. Compare total code, correctness and maintenance. A second dependency costs more when it duplicates a clock, renderer or state owner; distinct supporting roles can remain justified.

| Question | Evidence from the intended page |
| --- | --- |
| What does adoption replace or add? | The capability needed and the old path it supersedes, or the new capability when nothing is replaced. |
| What ships? | Measured bundled bytes, transitive modules, decoders, shaders, fonts and other assets; not an advertised minimum. |
| Can those files load? | Supported module format, emitted MIME types, path bases and import identity; offline or intended-network behavior where promised. |
| What can it contact? | Actual requests, including telemetry and deferred asset downloads, reconciled with delivery and consent policy. |
| Which version and rights apply? | Exact version or content pin, licence and required notices, including any wrapping or vendoring step. |
| How does it fit the page? | Compatible runtime, supported browsers, names and focus in the DOM, input behavior and cleanup. |
| What remains without it? | The promised task or honest unavailable state when script, a font or the graphics capability fails. |

Read a selected component's source before adapting it. Keep useful layout, material and choreography that carry its character; reconcile unselected demo styling with the destination's content, tokens and access. A different framework needs an explicit compatibility or adaptation route. Adopting one surface does not approve the dependency for every surface. Try removing a questionable dependency in an isolated change: keep it only if a required result or justified implementation benefit would be lost, and preserve consumer behavior when removing the superseded path.

## Worked example

A small interaction works in a component preview and fails on the real page. Follow its import from the emitted HTML to the resolved URL. Check whether the browser received a missing module or the wrong media type. Resolve the agreed dependency once, mount it at the intended element, then verify the original navigation and the same page without the enhancement.

## Sources

- [MDN progressive enhancement](https://developer.mozilla.org/en-US/docs/Glossary/Progressive_Enhancement): build from a usable supported base.
- [MDN JavaScript modules](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Modules): module identity, loading, imports and MIME requirements.
- [Performance budgets](https://web.dev/articles/performance-budgets-101): define what a budget counts before comparing a dependency with it.
