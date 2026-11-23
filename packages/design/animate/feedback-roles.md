+++
description = "Use it to give each interaction a reason to move, a response to interruption and a useful reduced form, from the project's shared timing and curve roles."
uses = [{ scope = "local", resource_id = "animate/productive-motion" }]
see_also = [{ scope = "local", resource_id = "animate/reduced-motion" }, { scope = "local", resource_id = "controls" }, { scope = "local", resource_id = "animate/expressive-motion" }, { scope = "local", resource_id = "animate/preserve-transform-origin" }]

[guide]
exposure = "internal"
+++

# Choose feedback roles and timing

Give each interaction a reason to move, a response to interruption and a useful reduced form. Keep the product’s actual state separate from the animation that draws it. Use the project’s shared timing and curve roles; the names below describe jobs rather than imposing a token vocabulary.

## 1. The role contract

| Role | Timing source | Trigger | What changes | Interruption and focus | Reduced form |
| --- | --- | --- | --- | --- | --- |
| Input acknowledgement | Short feedback role | Press, keyboard activation or fine-pointer hover | Emphasis, light or a press drawing inside a stable hit area | Release on pointer up; a repeat starts from the current drawing | Immediate visible state change |
| Focus | Immediate | Keyboard or programmatic focus | The focus indicator | Keeps up with key repeat and remains visible beside other states | Immediate |
| Preview | Intent delay where needed, then preview role | Deliberate hover, focus or explicit touch action | Preview anchored to its trigger; paragraph stays still | Leaving cancels a pending open; moving onto it preserves access | Appears and leaves without travel |
| Shell entrance | Entrance role for occasional activation; immediate at shortcut speed | A visible trigger or command shortcut | Shell and backdrop, with container before supporting content | Focus lands inside at once; closing during entrance reverses from the current state | Immediate or a brief in-place opacity change |
| Result update | Immediate content replacement; a movement role only for an indicator that can keep up | Query or active-result change | Results and active indicator | Latest eligible result wins; no animation queue or delayed typing | Immediate content and highlight |
| Dismissal | Exit role | Escape, close control or supported outside activation | Surface returns toward its origin | Focus returns to the invoker or intended successor at the start of exit | Immediate |
| Disclosure or ordered expansion | Expansion role; stagger only where order carries information | Disclosure or sequence-part activation | Content expands from its trigger; current part gains emphasis | Reversible; retain the reading anchor and usable controls | Open in place; a cascade appears together in its final order |
| Action result | Feedback role | Commit, error or completion | Text, form or light on the affected control or field | Never outlast the state it reports | Immediate state change |

Acknowledgement answers the input; result feedback reports what happened. They may share a timing role without becoming the same state. The project defines which feedback duration is its shortest useful one. A press may transform a visual child, but must keep the target and surrounding layout stable. The [Give expressive motion a purpose, “The material event contract”](expressive-motion.md#the-material-event-contract) uses the same trigger, drawing, interruption and reduced-form questions for its different purpose.

## 2. Rules

1. **Use a coherent timing system.** Read shared curves and role durations rather than tuning each control independently. One system may use one productive curve, a shorter exit duration and a symmetric movement curve for an already visible indicator. Another may have an accepted asymmetric pair. Follow the actual system and inspect the resulting continuity. Keep expressive character separate from the learned response of shared controls.
2. **Make the origin intelligible.** A popover or preview can grow from its trigger and return toward it; a centered dialog can stay centered. Use a deliberate starting scale rather than making a surface appear from a point by habit. [Build the origin and reduced form into the control, “3. CSS first”](preserve-transform-origin.md#3-css-first) and inspect it after scrolling or resizing.
3. **Allow use at arrival.** Focus, typing and scrolling must work when the interaction makes the destination available. Presentation settling must not defer them until animation end.
4. **Keep the page and target stable.** Opening an overlay should preserve the underlying scroll and layout. A hover or press must not move the target away or disturb incidental prose.
5. **Choose properties for the meaning.** Transform and opacity often suit movement and arrival; color, emphasis or light can show state. Prefer the less expensive construction that preserves the selected change, then profile it. A transform is not proof of compositor-only work.
6. **Make every press visible.** An actionable control answers pointer-down and keyboard activation promptly, independently of network or computation. Commit the action through its normal activation path so an abandoned press can cancel. The useful rule is that every interaction is implemented, precise and visible when pressed; timing alone cannot establish it.
7. **Match hover to capability.** Use hover effects where the pointer can hover. Preserve the same meaning at rest, on focus or through an explicit touch path.
8. **Keep destinations available.** A sequence must not delay navigation or hide the only route to content.

## Worked example

A pointer press on a navigation link changes the drawing while the person holds it. Dragging off and releasing can cancel. A keyboard user gets feedback too. Opening a drawer puts focus inside at once. Its entrance follows the shared timing role without delaying input or moving the trigger. Reverse it halfway through and check the same state with reduced motion.

Put the role decisions in the component’s existing specification and the observed result in its review or maintained interaction test.

## Sources

- [Carbon Design System: Motion](https://carbondesignsystem.com/elements/motion/overview/): the productive versus expressive split, productive motion for task moments and microinteractions, and duration and easing tokens by role.
- [Fluent 2: Motion](https://fluent2.microsoft.design/motion): motion as functional and consistent, scaled to element size and distance, communicating change and relationship without making people wait.
- [Apple HIG: Motion](https://developer.apple.com/design/human-interface-guidelines/motion): brief, purposeful feedback motion that never delays people, and restraint for frequent interactions.
