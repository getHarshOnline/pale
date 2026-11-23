+++
description = "Identify which definitions the consumer loads, even when different inputs produce equal output."
see_also = [{ scope = "local", resource_id = "contracts/definitions" }]

[guide]
+++

# Check which shared definition the consumer loads

<a id="check"></a>

Identify the entry point and the complete ordered set of definitions it imports. Resolve each to its owner, revision and bytes. Files present in a checkout may differ from those loaded. Check transitive imports and namespaces, including competing providers and permitted overrides.

Load that set in a clean run of the actual tool. Exercise different uses of shared names within the same consumer. Compare behaviour, expansion or rendered output with independent expectations for each name. Separate provider tests or a successful build do not establish that combined names preserve their meanings.

Substitute a known conflicting definition. Require rejection, or a behavioural mismatch if the tool permits shadowing. Where overrides are supported, include a control that builds successfully using the wrong definition. Recheck affected consumer routes. Report build completion and resolved behaviour separately from whether the underlying claim is correct.

### Verify input provenance when outputs match

When an import can come from several locations, inspect ordered search roots and paths the tool actually resolved. Use dependency listings, read traces or another independent observation. Identify the source revision and input digest for each path, and retain command and tool version in existing build or test output. A dependency declaration, logical import name or output digest does not reveal which checkout supplied the definition.

Run with the intended input, then place an equivalent copy in another permitted search root. Compare output and resolved inputs separately. Equal output may establish equivalence at that output while the wrong source still violates the required pin. Refuse or report the input source as unresolved if it cannot be identified. The project defines accepted roots and revisions.

## Before handing off

- [ ] Check which definition the consumer finds in its real load order.
- [ ] Exercise a duplicate name or different source with the same output.
- [ ] Record the input resolved and what the consumer did with it.
