+++
description = "For protocol adapters, inherited behaviour or operator settings that make one contract behave differently by profile."
uses = [{ scope = "local", resource_id = "contracts" }]

[guide]
exposure = "internal"
+++

# Compare actual behavior across contract profiles

Use these checks for protocol adapters, inherited behaviour and operator settings. Keep reproducible cases in the relevant tests. Report tested versions, inputs, actual results and missing coverage in the review reply.

## Choose the check

- [Compare protocols against the same operation](../protocols.md#check).
- [Test existing behaviour and the added capability separately](../existing-behaviour.md#check).
- [Separate tuning controls from contract changes](../settings.md#check).

## Worked example

Two transports advertise the same operation. Compare accepted inputs, denied cases and downstream results. Check inherited behaviour and the new transport's added capability separately. Change a retry timeout as a tuning exercise and observe its effect separately from any changed operation contract.
