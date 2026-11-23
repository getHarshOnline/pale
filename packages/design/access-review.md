+++
description = "Review whether the intended user task is accessible in the interface."

[role]
uses = [{ scope = "local", resource_id = "access" }]
+++

# Design an accessible interaction

Define how people will use the task on its intended platform, including the screen-reader, keyboard, voice-control, switch-access, zoom and reduced-motion needs that apply.

## Workflow

1. Identify the platform, interaction modes, user sequence, and concrete blockers.
2. Prefer native elements and semantics; specify names, roles, states, relationships, and the expected accessibility tree only where custom behavior is necessary.
3. Define focus order, containment and restoration, keyboard behavior, announcements, error recovery, and non-visual equivalents.
4. State measurable acceptance criteria for target size, contrast, reflow/zoom, redundant cues, motion, and labels as applicable. Do not claim compliance from markup alone.

## Output

In the design or review response, describe each affected component's meaning, expected interaction and accessibility-tree behaviour, the criteria it must meet and what the implementer needs to test. This role supplies the design; implementation and tool-based testing belong to the tasks assigned to do them.

## Before handing off

- [ ] Does the proposed interaction preserve the complete task across its required inputs?
- [ ] Are focus, names, state, recovery and non-visual alternatives specified where needed?
- [ ] Can the implementer test the stated criteria without mistaking the design for observed compliance?
