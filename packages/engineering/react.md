+++
description = 'Review React behavior when React is a dependency or is selected in project configuration.'
uses = [{ scope = "local", resource_id = "web" }]
official_sources = [{ label = 'React reference', url = 'https://react.dev/reference/react' }]

[guide]

[role]
extends = [{ scope = "local", resource_id = "review" }]
applies_to = [{ scope = "local", resource_id = "react" }]

[stack]
extends = [{ scope = "local", resource_id = "web" }]
role_uses = [{ scope = "local", resource_id = "react" }]
+++

# Give React state effects and async work clear owners

Apply this guidance when React is selected, after the common and browser methods. Establish the exact framework, renderer, router and server-runtime behaviour from the repository and current official sources.

## Keep rendering and state understandable

Components should remain pure for equivalent inputs, with responsibilities that follow ownership and how the code changes. Call hooks unconditionally and include every captured reactive dependency. Use effects to synchronize outside systems, with cleanup that tolerates the selected runtime's lifecycle. Put event-driven changes in event handlers.

Keep state at its lowest shared owner and derive values instead of storing duplicate versions. Use stable keys for the intended identity. Introduce an external store when state has a shared lifecycle that needs it. Define loading, fallback, recovery and cancellation for asynchronous work and failures.

## Understand identity before changing memoization

Use memoization for a needed stable identity or a measured cost. An empty dependency list alone does not make it redundant: inspect captured reactive values, callback-ref attachment, effect dependencies and consumers of identity before removing it.

A function or object created during render has a new identity on each render. Moving it to module scope changes its lifetime and sharing. Keep dependencies complete instead of hiding changing inputs behind an empty list. A memo cache is an optimization; use state or a ref when correctness requires a value to persist.

## Preserve browser behaviour and server boundaries

Keep semantic HTML, accessible names and relationships, focus, keyboard paths and coherent form states. Validate or sanitize untrusted HTML and URLs for the place receiving them. Where server and client rendering coexist, keep confidential data out of client-visible output and ensure equivalent state produces compatible initial output.

Trace user-visible latency through request waterfalls, shipped code, server work, scheduling and rendering. Measure the dominant cost before splitting or deferring work. Component extraction and memoization need a traced behavioural problem or measured cost.

## Test what the user can observe

Exercise rendered behaviour through names, roles and interactions a person can perceive; test pure transformations directly. Control network responses where the code calls them and observe delayed success, failure and cancellation. Include keyboard, focus and accessible states in the same checks. Keep assertions independent of component internals, and use a real browser when correctness depends on its engine.

For example, keep the active filter with the component that owns the result list and derive visible results during render. If a lookup is asynchronous, ignore obsolete responses after the query changes. Exercise rapid edits and a failed lookup through the actual field.

## Review React lifecycle identity and interaction failures

Apply this role when React is selected. Add these risks to the shared review method:

- conditional or invalid hook calls, stale dependencies, missing cleanup and effects that derive state but leave it stale, leak resources or race;
- mutated state or unstable identities and keys that break rendering, side effects during render, and server/client components or actions that expose confidential data or trust unchecked input;
- unsanitized raw HTML, unchecked user URLs, private values in client bundles and long-lived script-readable credentials without an explicit assessment of that security tradeoff; and
- broken keyboard, focus or accessible-name behaviour, Suspense or error handling that fails observably, and measured rendering regressions.

Recommend memoization, dependency changes or component extraction only for traced behaviour or a measured cost.
