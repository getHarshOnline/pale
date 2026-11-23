+++
description = "After understanding the change and its guarantees, check what is necessary, reuse suitable code and platform facilities, and choose a complete solution that people can maintain."

[policy]
+++

# Choose the simplest complete solution

Choose a solution after understanding the outcome, its cause and the guarantees it must keep. Prefer the solution that adds the fewest concepts, owners and moving parts while doing the whole job. When two solutions work, choose the one people can maintain, even if it changes more lines or files.

## Start with what is needed and what already works

Ask four questions:

1. Can removing unnecessary behaviour satisfy the request? Is a proposed requirement unsupported, so it should be declined rather than implemented?
2. Does the component that owns this behaviour already provide a mechanism with the right meaning?
3. Can the rule live where its state is owned, without a synchronized copy or translation layer?
4. If new code is needed, what is the smallest coherent component whose lifecycle, failures and tests remain understandable?

Check existing code, the standard library, platform facilities and maintained dependencies before building another mechanism. Reuse them when their meaning, version, permissions and failure behaviour fit. If reuse needs wrappers, compensating state and exceptions throughout the code, a focused local implementation may be easier to maintain. Familiarity alone is no reason to rebuild a maintained capability already available within the project's scope.

## Compare the costs that will remain

Consider whether each candidate fulfils the whole behaviour, how many concepts and dependencies it adds, what it costs to operate, who owns its data, how compatibility and recovery work, what must be tested, and what removing it later would require. Prefer one direct representation of state and a rule enforced at its owning interface over parallel state or conventions every caller must remember. Once replacement is complete and removal is authorised, retire the obsolete implementation instead of maintaining two meanings.

Add extensibility for a real need: avoid frameworks for one known case, configuration where nothing varies, performance machinery without a demonstrated need, and abstractions based only on similar-looking syntax. Security, accessibility, recovery, migration, documentation and changes required by users are part of the solution and its cost.

Explain the chosen design and material trade-offs in the existing plan or reply. Compare alternatives when there is a real choice; a focused repair using an established mechanism needs only the reason it fits. If the solution deliberately supports a limited case, state that limit and what future requirement would justify replacing it. Invented alternatives and extra reporting files do not help make that decision.
