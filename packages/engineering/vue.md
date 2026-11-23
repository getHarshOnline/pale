+++
description = 'Review Vue behavior on the web platform. Add TypeScript guidance when the project separately selects TypeScript.'
uses = [{ scope = "local", resource_id = "web" }]
official_sources = [{ label = 'Vue guide', url = 'https://vuejs.org/guide/introduction.html' }]

[guide]

[role]
extends = [{ scope = "local", resource_id = "review" }]
applies_to = [{ scope = "local", resource_id = "vue" }]

[stack]
extends = [{ scope = "local", resource_id = "web" }]
role_uses = [{ scope = "local", resource_id = "vue" }]
+++

# Keep Vue reactivity and component ownership clear

Apply this guidance when Vue is selected, after the common and browser methods. Establish the exact Composition API, compiler, router, store and server-rendering behaviour from the repository and current official sources.

Choose `ref`, reactive objects and computed state according to ownership and identity. Derive values instead of copying them, and check that destructuring preserves the tracking the code needs. Use watchers for effects, with cleanup and cancellation of obsolete asynchronous work. Keep them within the component or effect scope that owns their lifetime.

Make props, emits, slots and model bindings explicit component contracts. A reusable composable should own a coherent piece of stateful behaviour. Keep local state local, and give shared domain state one accountable store. Navigation guards express transition rules; server-side permission checks still govern the action.

Use normal template escaping. Validate raw HTML, dynamic URLs, styles and event construction for the place receiving them. Server rendering must keep mutable module state from leaking between requests and produce initial output compatible with the client.

Check reactivity, lifecycle cleanup, component identity, asynchronous ordering, hydration and accessibility through actual behaviour. Measure rendering costs before prescribing an optimization. A store, composable or memoization needs a real ownership or behaviour problem to solve.

## Change a shared filter

Give the parent ownership of the applied filter and let the child emit a requested change. Use a computed value for the filtered rows. Change the parent's value and the child's input separately, then check the list and keyboard flow. Exercise the component through its real input and failure path.

## Review Vue reactivity lifecycle and server rendering

Apply this role when Vue is selected. Add these risks to the shared review method:

- ref or prop destructuring, watcher sources or reactive-object replacement that lose tracking or leave state stale under the selected Vue version;
- composable effects outside their instance lifecycle, missing cleanup and shared module state leaking across components or server requests;
- mismatched props, emits, slots, store or router-guard contracts that cause runtime failure or a gap in permission checks;
- unsanitized `v-html` or user URLs, exposed confidential runtime configuration and server routes without effective input validation or permission checks; and
- server/client hydration mismatch, unstable list identity, broken focus or semantics and measured rendering cost.

Recommend memoization or reactivity changes only for a concrete failure or measured cost. Ask the requesting task for another review role when needed; do not delegate from this role.
