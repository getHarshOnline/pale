+++
description = "Propose a shared design system from an existing product or accepted direction."

[task]
uses = [{ scope = "local", resource_id = "system" }, { scope = "local", resource_id = "art-direction" }, { scope = "local", resource_id = "access" }, { scope = "local", resource_id = "creative-work" }]
roles = [{ scope = "local", resource_id = "access-review" }]
outputs = ["Reviewable reference, token roles, component behavior, integration choices and checks in the requested specification."]
+++

# Propose a shared design system

Decide whether the task extracts a system from an existing product or develops one from an explicit new direction. Establish one source for tokens and map existing components to it. Preserve stable product conventions unless the requested change reaches them.

Return one coherent proposal in the requested reference or specification: design roles, token model, component behavior, supported variation and a preview plan. Examples should use those definitions rather than quietly inventing parallel ones. Name the representative states and observations needed for the owner to judge adoption. This operation prepares the proposal; installation, component edits and rendered previews belong to the implementation task.

When independent critique is needed, give the reviewer the original request, audience, retained directions, references and their roles, existing product meaning and proposed token and component decisions. Explain the question the independent review must answer and what an actual preview must still establish. Keep values and implementation in their owning sources. A proposed system remains a proposal until accepted.

## Before handing off

- [ ] Do the roles and states follow from the requested design and real tasks?
- [ ] Does the proposal show where shared components allow meaningful variation?
- [ ] Are the decisions still needed before adoption clear to their owner?
