+++
description = "When you change a name, location, pinned contract version or identity binding, to check that identity and permissions survive."
uses = [{ scope = "local", resource_id = "contracts" }]

[guide]
exposure = "internal"
+++

# Keep contract identity and permissions intact

Use these checks when changing names, locations, pinned contract versions or identity bindings. Keep durable cases with the consuming system's tests. In the review reply, identify the versions and actors checked, observed results and permissions or behaviours still unproved.

## Choose the check

- [Check identity and permissions across a binding](../identity.md#check).
- [Verify identity across generated names and locators](../names.md#check).
- [Check the contract version before integration](../contract-version.md#check).
- [Separate terminology from compatibility identifiers](../terminology.md#check).

## Worked example

Two generated names normalise to one file path. Verify the underlying IDs, refuse the collision and regenerate from an agreed contract version. A readable label can change without changing the compatibility ID or a binding's permissions.
