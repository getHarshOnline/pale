+++
description = "Use it to audit a shared design through representative journeys and complete states rather than attractive screenshots."
uses = [{ scope = "local", resource_id = "system" }]

[guide]
exposure = "internal"
+++

# Audit a shared design through complete states

## Audit through complete states

Inspect representative journeys rather than attractive screenshots. Compare:

- hierarchy, density, spacing, alignment, and reading rhythm;
- semantic token use and component state contracts;
- default, focus, active, loading, empty, error, success, and disabled states;
- responsive reflow, content stress, input modes, and accessibility;
- feedback timing, target confidence, interruption, and recovery; and
- decorative burden that does not communicate state or identity.

Classify each divergence as a justified extension, migration debt, or unexplained inconsistency. Locate it in the artifact and state. A screenshot cannot establish focus, motion, latency, or state-transition behavior.

For system formation, put the role-based token model, component behavior, permitted variation and proposed adoption scope in the requested specification. For an audit, return the located differences, the shared behavior or meaning they break and a repair order based on user consequence and recurrence. Use the existing review or reply. Token files, component implementation, migration and visual approval remain with their product owners.

## Worked example

Run the same select control in two contexts. Press, cancel, open with the keyboard, enter a slow or failing state, then retry. The first instance loses focus when it closes. Compare actual component versions and active overrides. If a deliberate local variation changes material, check that it still uses the shared behavior.
