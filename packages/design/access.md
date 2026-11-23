+++
description = "Make the user task perceivable, operable and understandable across supported inputs and assistive technology, using the applicable accessibility criteria."
uses = [{ scope = "local", resource_id = "design" }]
official_sources = [{ label = "W3C Web Content Accessibility Guidelines 2", url = "https://www.w3.org/WAI/standards-guidelines/wcag/", jurisdiction = "International web standard; verify applicable local law separately", checked = "2026-08-04" }]

[guide]
+++

# Make the task accessible

Accessibility means that people with different sensory, motor, cognitive and assistive-technology needs can perceive the information, understand the choices, operate the controls and confirm the result. Start with that task and its loading, empty, disabled, error and cancellation states. Access takes precedence over visual identity and taste; keep the intended character in a form people can use.

Establish which platform, assistive technologies, standard and version the project supports. Read the relevant current criteria before interpreting a threshold or making a conformance claim. Product policy and legal applicability come from their responsible owners.

## Seven rules to apply to the task

| Rule | Look for | What to do |
| --- | --- | --- |
| Make every action operable through the supported inputs | A pointer-only action, a missing focus indicator, an accidental trap or focus lost on close | Provide keyboard and alternative-input routes, logical order, visible focus and a useful return from the action. |
| Measure the rendered result | A contrast, focus or target-size claim based only on tokens or markup | Measure the element on the actual surface and in each affected presentation state. |
| Keep requirements and project targets distinct | A rounded-up ratio or a stricter design target presented as the standard's minimum | Compare unrounded measurements with the applicable criterion. Report the criterion and project target separately. |
| Give meaning more than one carrier | A state conveyed only by colour, sound, hover, movement or position | Add equivalent text, shape, structure or an accessible announcement so the same decision remains possible. |
| Prefer the platform's built-in behaviour | An announced role with missing input, focus or state behaviour | Use a native element that fits. For a custom control, implement its complete interaction and accessibility behaviour. |
| Keep one modal interaction at a time | Stacked veils, competing focus containers or a dialog that lets focus escape | Hand work between tools inside one shell, keep the background inactive and return focus when it closes. |
| Check the task beyond an automated scan | A scanner result being treated as proof that someone can finish | Walk the task with supported inputs and assistive technology, then test zoom or reflow and relevant presentation changes. Report skipped checks as not run. |

## Work from meaning to implementation

First say what each element means and what it does. Then decide how its information and state are perceived, how input and focus work, and how the platform exposes those choices to assistive technology. A label cannot repair an otherwise unusable control.

Trace entry, progress, errors, recovery and the final result. Opening, closing, validation, route changes and asynchronous updates each need a deliberate focus or announcement outcome. Let people identify the field that failed, understand how to correct it and keep their valid work. Test whether the task still makes sense without colour, sound, fine pointer control or animation. Where the consequences justify it, include representative users as well as technical checks.

## Explain what the check found

In the review or reply, identify the task and state tested, the platform and versions, what failed, the criterion or project requirement it affects, and the smallest useful repair. Put a repeatable regression in the project's maintained checks. Assign content, interaction, implementation and product-policy changes to the part that owns them.

Say which states or assistive technologies you could not test and what would make that check possible. A scan can establish that its checked rules passed; keyboard traversal can establish the observed focus path; a screen-reader session can establish what was announced. Keep the remaining limits beside the result they qualify. Formal conformance decisions remain with the people responsible for them.

## Before handing off

- [ ] Can a reader identify the task, its relevant states and the requirements used?
- [ ] Can people perceive, operate and recover through the paths actually tested?
- [ ] Does the reply distinguish observed results, failures and checks still needed?
