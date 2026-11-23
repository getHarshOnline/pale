+++
description = "Check ordinary page failures on the rendered result"
uses = [{ scope = "local", resource_id = "inspect" }]
see_also = [{ scope = "local", resource_id = "inspect/lenses" }]

[guide]
exposure = "internal"
+++

# Check the page before judging its treatment

Run the sweep on every rendered review pass, including the pass after a fix: a repair can introduce an ordinary defect or lose an intended transition. Mark a row passed only after inspecting the actual page or exercising the behavior it names. Source intent and a saved capture that nobody looked at do not establish a pass.

Keep the scope explicit. A new page or delivery review covers all applicable rows; a focused repair covers the affected page, its reached consumers, prior failures and plausible regressions. Preserve an explicitly requested full scope. Do not repeat unrelated checks without a changed input or unresolved concern. Record passed, failed, blocked or not-run outcomes in the existing review, with the route, condition and observed evidence. Link this page instead of copying the table into another manual.

| Check | Observe |
| --- | --- |
| Links and routes | Follow real destinations, including generated pages and relevant fragments. |
| Home and return | Use the site’s own navigation and Back; check the returned position and state. |
| Not-found route | Open a missing address and find a useful way onward. |
| Access across contexts | Reach required content and destinations with the supported narrow and wide layouts. |
| Document identity | Read the page title and emitted language, icon and relevant color-mode metadata. |
| Descriptions | Compare any description with the supplied source rather than invented template copy. |
| Heading structure | Inspect the page title and meaningful heading order. |
| Root overflow | Compare scroll and client widths and investigate unexpected sideways scrolling. |
| Clipped content | Read escaping and clipped text, including content hidden inside otherwise fitting boxes. |
| Navigation controls | Open, close and activate them with keyboard and actual touch where supported. |
| Hit targets | Compare effective hit regions and spacing with the applicable criterion and separate project target. |
| Visible viewport | Check full-height regions, browser chrome, device insets and the software keyboard. |
| Control action | Confirm that pointer and keyboard activation do what the label promises. |
| Press feedback | Observe feedback at contact before slow script or network work. |
| Focus | Traverse every affected surface; check visible focus, traps and restoration. |
| Waiting | Inspect the pending state, reserved space and movement when the result arrives. |
| Outcomes and recovery | Reach success and real failure states and use their recovery actions. |
| Service honesty | Compare claimed availability and completion with the actual provider or application result. |
| Missing content | Distinguish intentional omissions and labelled examples from accidental placeholders or filler. |
| Figures and status | Trace counts, dates, values and status to the source; preserve units and qualifications. |
| Current dates and interface labels | Check the owning content or runtime source, including localisation; a hard-coded year must not pretend to be current. |
| Audience wording | Keep necessary explanation and disclosure; move internal production commentary to its owning records. |
| Contact and help | Exercise supplied call, message, map and help links; omit details the source does not supply. |
| Legal and utility routes | Verify the pages required by the product’s actual collection and service arrangements. |
| Intake | Check who receives it, what happens next and what the provider actually confirms. |
| Image layout and first paint | Observe reserved image space and layout shifts; identify the actual largest-paint element and its loading priority. |
| Media and fonts | Measure delivered media against the project’s budget and inspect the face that actually rendered. |
| Motion and preferences | Check that interface feedback follows the action it answers. An intentionally living field keeps its own declared slow clock, suspends when hidden and reduces or stills under reduced motion. Exercise pause, explicit preference and system routes where offered; judge an authored film on its own timeline. |
| Designed renditions | Read supported dark, reduced and narrow treatments for intact meaning, hierarchy and accessible controls. |
| Capability loss and scrubbing | Use the promised scripts-off or graphics fallback; for a scrubbed sequence compare the same settled position from both directions. |

A clean sweep still leaves design judgment. Use [Judge the design through its working parts](lenses.md) to compare the whole result with the brief and references. After a repair, look again at the qualities that were meant to survive, including artwork, transitions and their reading holds. An intact layout can conceal a lost expressive treatment.

## Worked example

A fix to a narrow title changes its shared container. Inspect the title and the other consumer that uses that container, then follow navigation and play the transition through it. The title now fits, but an overflow rule clips the artwork during movement. Keep that finding open and repair the shared rule; the clean final still did not establish that the transition survived.
