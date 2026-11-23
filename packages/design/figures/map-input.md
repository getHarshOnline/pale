+++
description = "Use it to design the still and unavailable-capability result of a live region, reserve its footprint and map input to its response."
uses = [{ scope = "local", resource_id = "figures" }]
see_also = [{ scope = "local", resource_id = "animate/retarget-controls" }]

[guide]
exposure = "internal"
+++

# Keep a meaningful still and map input to the live region

## Compose the fallback and the input response

Design the initial and unavailable-capability result deliberately. For a publication offering progressive enhancement, a composed still and useful semantic content can retain the region's job without script or GPU. Reserve the appropriate footprint so mounting does not unexpectedly move the reader's place. A blank canvas, endless spinner or arbitrary frame paused midway is not a finished alternative. An application requiring computation must expose its honest unavailable state rather than promise a static equivalent of the whole task. Preserve factual meaning as the carrier changes.

Choose relevant dark, reduced-motion, narrow-layout and capability conditions from the supported contract. A composed representative still can preserve a field's structure; an arbitrary frozen intermediate frame may leave it unreadable. An in-place state change can preserve a discrete response. Design the dark ground, key, emission and large-area luminance together so the region belongs beside the dark page instead of glaring against it. A light image inverted or filtered at runtime is not automatically a valid intended dark grade. Preserve authored state assets where they are required; another procedural route may generate its supported forms correctly. No mandatory four-cell asset matrix applies to every project.

| Full-motion treatment | A reduced form to compose and check |
| --- | --- |
| Travel, scale or camera movement | An immediate state change or suitable opacity change without the travel. |
| Evolving field | A representative still of the same field, with its structure intact. |
| Subdivision, convergence or enumeration | Geometry that carries the completed relationship or numbered sequence. |
| A relationship explained through flow | The same relation in geometry, labels and any supporting color. |
| State-bearing light | The same state shown in place, without a distracting pulse. |

Translate the meaning rather than merely slowing parallax or freezing whatever frame was visible. Read the preference at mount and follow later changes through the project's actual explicit and system preference routes.

Separate a position-bound picture from a reaction played toward a rest. The same settled input should produce the same bound picture; a discrete input should not accidentally leave a clip suspended between its intended states. An intentionally living field can have its own selected slow clock, suspended when hidden and reduced or stilled under reduced motion. Hold that clock fixed when comparing its input response. Use [Reverse or retarget a control in flight](../animate/retarget-controls.md) for timing and interruption rather than deriving a clock from whichever library was easiest to load. An authored film keeps its selected timeline.

Read pointer input relative to the actual region and update that geometry when its size or position changes. Match discrete authored poses with suitable zones and stable boundaries; preserve continuous response when that is the selected behavior. Leaving, cancellation and loss of focus need their intended return state. A coarse pointer lacks hover and a touch pointer supplies position while in contact; do not assume phone width tells you the input capability. Preserve keyboard and alternative paths for any meaningful interaction.

Where a stable procedural composition is required, use an explicit repeatable seed (for example, a stable page identifier or path when persistence under that identity is intended) and compare the fallback, first enhanced frame and repeated captures under the same inputs. Reproducible inputs do not guarantee pixel-identical output across GPUs, fonts or rendering versions. Time-varying art can deliberately vary, but its selected behavior is not permission to fabricate data.

## Write the object's resting states and the transitions between them

An object may be resting, raised, chosen or open. An actor can have authored poses and expressions. A field can change its orientation or density as the reader moves. Choose what answers and what remains visible at rest. Give each end a composed view the person can stop on. Use input-bound motion while the reader controls position. Use a played change when a discrete action should resolve to a stable state.

| From | Input | To | Behavior |
| --- | --- | --- | --- |
| Resting figure | Pointer enters a detail region or keyboard focus reaches its caption link | Detail emphasized | Track position when it adds information. Focus takes the same stable state. |
| Detail emphasized | Reader chooses Open | Detail open | Preserve the chosen state while a short transition plays. |
| Detail emphasized | Reader leaves or dismisses | Rest | Release to the original still. A new action can interrupt release. |

For the actual implementation, keep one row per transition with its starting state, input, ending state, bound or played reaction, timing owner and asset or frame range. An interface transition uses its design role; authored scene choreography retains its selected timing. Put those choices with the editable assets or state data so retiming does not hide in a handler. Mechanism calls and frame arithmetic belong in the implementation. When a project has a registered behavior vocabulary, extend its owning contract before delivery instead of forcing a new mechanism into a misleading old name.

Keep links and controls in the usable document even if the drawing presents them as objects. Normalize input in the region's own coordinates and check edges, leaving and reentering. A coarse pointer needs a clear action rather than a hidden hover zone. For a region that moves with device or scroll input, recheck position after resize, clipping and loss of focus.

## Avoid unstable input at the edge of a state

Normalize positions against the region after each relevant resize. When a change from one zone to another would flicker at a shared edge, let the current state hold a small range around the edge and check it from both directions. Check the same position approached at several speeds. A discrete press can commit an authored state once. Scrolling back across the region must not re-trigger that action. Under reduced motion, make the same state or relation readable with the appropriate rest or immediate end.

For a simple axis-aligned region, take `getBoundingClientRect()` and map `clientX` and `clientY` from its bounds to a clamped range such as −1 to 1 on each axis; guard empty dimensions. A transformed or perspective scene can need its renderer's inverse coordinate mapping rather than the enclosing rectangle. Capture the input, then draw at most once per display frame. Stable zones may have hysteresis at their borders; that deliberate zone memory is different from a continuous response that should settle to the same picture at the same input. Do not add an unselected blink, breath or look-around to fill a resting state.

Use source fields for the objects a drawing presents. An invented height, order or scale can look like a measurement. If the region stages a collection, keep each object's real link, identity and relevant details in reading and focus order. A touch user needs an explicit path to any reveal. If the live graphic is unavailable, the still and surrounding document should carry the useful state and links.

## Worked example

A plate turns toward the pointer and shows a field evolving inside. Convert pointer coordinates using the plate’s current rectangle, update it on resize and leave a meaningful resting view on pointer exit. Map touch while it is in contact and give focus an equivalent way to inspect the meaningful state. The field clock can pause while the view follows input. On a device without the graphics feature, show a composed still at the same footprint and keep the labels. Use the same seed for comparing the initial states where the design depends on a stable field.

## Sources

- [Pointer Events](https://www.w3.org/TR/pointerevents3/): pointer types, capture and cancellation; touch has position while in contact.
- [MDN `getBoundingClientRect()`](https://developer.mozilla.org/en-US/docs/Web/API/Element/getBoundingClientRect): viewport-relative bounds and their relation to scrolling.
- [The Book of Shaders: Random](https://thebookofshaders.com/10/): repeatable pseudo-random values from fixed inputs.
- [Animation from Interactions](https://www.w3.org/WAI/WCAG22/Understanding/animation-from-interactions.html): removing nonessential interaction motion without losing function or information.
