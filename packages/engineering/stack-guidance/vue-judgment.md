# Keep Vue reactivity and component ownership clear

Apply this guidance when Vue is selected, after the common and browser methods. Establish the exact Composition API, compiler, router, store and server-rendering behaviour from the repository and current official sources.

Choose `ref`, reactive objects and computed state according to ownership and identity. Derive values instead of copying them, and check that destructuring preserves the tracking the code needs. Use watchers for effects, with cleanup and cancellation of obsolete asynchronous work. Keep them within the component or effect scope that owns their lifetime.

Make props, emits, slots and model bindings explicit component contracts. A reusable composable should own a coherent piece of stateful behaviour. Keep local state local, and give shared domain state one accountable store. Navigation guards express transition rules; server-side permission checks still govern the action.

Use normal template escaping. Validate raw HTML, dynamic URLs, styles and event construction for the place receiving them. Server rendering must keep mutable module state from leaking between requests and produce initial output compatible with the client.

Check reactivity, lifecycle cleanup, component identity, asynchronous ordering, hydration and accessibility through actual behaviour. Measure rendering costs before prescribing an optimization. A store, composable or memoization needs a real ownership or behaviour problem to solve.

## Change a shared filter

Give the parent ownership of the applied filter and let the child emit a requested change. Use a computed value for the filtered rows. Change the parent's value and the child's input separately, then check the list and keyboard flow.

## Before handing off

- [ ] Do props, events and shared state have clear contracts and owners?
- [ ] Were actual template and async updates observed?
- [ ] Was the component exercised through its real input and failure path?
