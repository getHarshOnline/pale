+++
description = "Distinguish direct relation evidence, derived reachability and other properties that need separate proof."

[task]
uses = [{ scope = "local", resource_id = "runs" }]
outputs = ["Original assertions and edges supporting each derived result.", "Controls, evidence for claimed properties and remaining limits."]
+++

# Check what the evidence says about a relation

Before computing a relation, name it and the question it answers. Fix the subjects or vertices, direction, scope, available evidence and applicable time. Define the evidence required for a direct edge, observed response or other base assertion. For a derived path, identify the exact relation types, revisions, direction, permitted view and traversal limits. Keep original assertions separate from derived pairs.

For a path or closure, retain the edges that establish it and identify pairs added by composition or reflexivity. A path establishes the stated reachability claim. Direct relationships, intervention effects, structural claims and permissions need their own evidence. For an intervention, state what it sets, what else it may change, the preparation and the recorded output. Setting a variable and observing a response may still leave the direct mechanism unknown. If a relation must be transitive, state the assumptions and test or prove transitivity over the permitted domain. Taking a relation's closure creates a different relation with different evidence requirements.

Test a chain whose endpoints are reachable without a direct edge. For intervention claims, include detectable responses between adjacent subjects whose end-to-end response cancels under the permitted interventions. Compare with a separately justified configuration that satisfies the required property. Test absence only within the exact permitted view. Depth limits, missing history and inaccessible inputs may leave the question open. Do not traverse denied vertices or edges to explain a result, or treat an omitted result as proof of global absence.

## Check independent relations in one projection

Identify each relation's owner, identity, type, direction, participants, revision, validity and permitted view. State how the source type maps to the destination relation class. Independent relations may share endpoints, labels or layout. Require separate evidence of authority for each class and retain every independently supported edge. If a view derives a path, keep the supporting edges and derivation rule separate from direct relation records.

Test parallel relations between the same endpoints and the same label used under a different owner. Use an identically shaped graph under different authority as a control. Remove, deny or revise one relation at a time and verify that only dependent parts of the view change. Keep disclosure within the destination's permissions. Where the caller may see these distinctions, separate verified absence in the permitted view from missing evidence, denied coverage and a search stopped at its limit.

Inspect the selected view and original records. Check identity, multiplicity, direction, validity and unresolved mappings for each relation, as well as the conditions that govern accepting it or acting on it. Report collapsed rows or inferred authority as defects in the view. The source owner supplies relations and evidence; the view owner supplies mapping, visibility, presentation and inspection of the resulting view.

## Check source matches without merging identities

Identify source versions, record scope, original keys and placement identities on both sides of a proposed match. Preserve each key as written and state normalization or alias rules before joining. Similar labels, padding or names may suggest a case for review. An exact key match establishes only the relation that key defines in the selected namespace.

Report both populations, exact matches, unmatched keys on each side, ambiguous matches and how many rows match each key. Preserve one-to-many placements and rows without counterparts. Identical bytes under different source keys still have distinct placements and histories. A digest can establish byte equality without settling who owns, describes or may use those bytes.

Test a near match, a code reused in another namespace, an exact match with unresolved ownership and identical bytes under two keys. Inspect the original source locations and selected view. Separately verify subject identity, material claims and permitted uses where the consumer needs them. Keep unresolved matches visible to the source owner and retain original revisions.

Return the original evidence, edges supporting derived results, assumed properties and scope of each conclusion. The relevant owners retain control of the graph or model, inference rules, acceptance of evidence and permissions.

## Before handing off

- [ ] Verify the underlying edges and their sources.
- [ ] Separate derived reachability from other relation properties.
- [ ] Check the smallest case that can disprove the claim.
