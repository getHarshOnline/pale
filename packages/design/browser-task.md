+++
description = "Exercise the requested browser task in its agreed test environment, including behavior, visual state, accessibility and privacy."

[task]
uses = [{ scope = "external", resource = { package_id = "pale.engineering", resource_id = "web" } }, { scope = "local", resource_id = "user-task" }, { scope = "local", resource_id = "inspect" }]
roles = [{ scope = "external", resource = { package_id = "pale.engineering", resource_id = "testing" } }]
outputs = ["Scenario results and relevant captures in the maintained test output or existing review.", "Product failures, environment limitations and unobserved outcomes stated in the reply."]
+++

# Check a user task in the browser

Choose the browser claim, permitted environment, starting state, user path and private data that must stay protected. Use the shared end-to-end, accessibility and relevant product guidance to select the checks that decide that claim. Keep intended and observed results distinct, and mark an unobserved step unknown.

Put scenario results and the conclusion in the existing test output, review or reply. Distinguish a product failure from limits of the environment or observation. This is a checking task: put product repairs on the product’s work list, keep persistent changes within the agreed scenario and leave release decisions with the owner.

Record the exact build, browser, viewport, input modality and presentation preferences. Check the user-visible result and focus, role or name when they carry the meaning. A browser accessibility tree is distinct from a screen-reader session. Simulated touch is distinct from physical touch. For an interaction that can regress silently, use a known-bad control or an exact prior failing observation. Keep a screenshot, state trace or native receipt tied to the checkpoint it actually establishes.

## Check touch input and the settled result

Confirm the actual CSS viewport, input mode and relevant presentation settings in the session that exercises the journey. Use a route that can dispatch touch or a physical device. Record which route delivered the input, where it landed and the state it produced. A narrow viewport or coarse-pointer setting alone supplies no touch input.

For a swipe or drag, record the intended axis, starting and ending offsets on the relevant scroll surfaces, and the item selected at rest. Check selection and measured alignment before a follow-up tap. A selection marker can change while the target is still moving. When interruption is part of the claim, exercise it during travel and keep that result separate from the settled tap.

If the claim includes a scripts-off or other capability fallback, verify the fallback setting and exercise its own controls. Read back the resulting state and history where navigation matters.

Check that the capture route preserves the observed viewport, input mode and presentation settings. If capture changes a property that affects the claim, withhold that image as evidence. A verified viewport capture can support the states it shows. Full-page coverage and input delivery each need their own observation.

## Verify readiness, activation and outcome separately

Name the intended control and action, the input modality, and the earliest state in which the user should be able to act. Record whether the control is readable or perceivable, which target would receive that input, and which state permits it. For pointer input inspect the actual hit target. For keyboard input inspect the actual focus and activation semantics. A visible label can be covered, disabled or bound to an earlier state.

Send the actual input once, then identify what received it and which request, navigation or local transition it produced. Record submission, acceptance or pending feedback separately from the observed result. When completion depends on a native receipt, match its identity and terminal state to the exact request. Keep the visible outcome and the native evidence in separate fields. A successful response, animation or status label alone cannot establish durable completion.

Exercise a readable but unavailable control, an intercepted input and a delayed or mismatched result when those failures affect the claim. Hold or release the relevant state deterministically and observe each checkpoint in order. A harness that waits or retries can move the action past the window under test. Record that behavior when it happens and qualify only the input and outcome actually observed.

### Verify delayed responses after the view changes

Bind each request to the owning record and to the view or request generation that may consume its result. Apply the same eligibility check to success, failure, clearing pending state and any follow-up request. A matching record identifier alone cannot distinguish an earlier visit to that same record. Transport cancellation can reduce work. Verify the receiving guard even when an old response can still arrive.

Hold a request open. Switch to another record and let that view settle. Deliver the older success and failure in separate cases. Repeat after leaving and returning to the same record, and compare with an eligible current response. When several requests can run within one visit, use the project's declared ordering policy and include an older request finishing after a newer one.

Read back the current selection, visible data and status, error and pending feedback, and the coordinates of any follow-up requests. Verify that an ineligible result cannot replace, clear or act through the current view. Retain an earlier result under its original identity when the contract calls for it. A mocked service can qualify client handling. Claims about server authority, cancellation or durable effects need native evidence.

## Verify captured inputs and staged completion

For an input that is reset or reused, follow its selected items through the immediate event, deferred update and visible retained state. Exercise single and multiple selection where supported, empty or cancelled selection, and re-selection under the product's declared deduplication policy. Read back item identities and counts rather than inferring success from an Add label. A synthetic deferred update can test whether selected inputs survive that delay. The actual app and browser are needed to establish the complete upload journey.

For a staged Apply, identify whether the product promises to commit all selected items together or permits a partial result. Compare the visible readiness, actual activation and committed result for complete, empty, missing and invalid assignments. Change or remove an item after readiness and include busy or late completion when relevant. Inspect the native handler's own checks separately from the button gate. A direct handler probe can reveal a bypass even when the browser blocks a click. Verify which items changed, what remained staged and whether correction is still possible. Keep those observations separate from service authorization and durable persistence. Use the shared web judgment for the implementation boundary; report any current limitation without broadening the verdict.

## Check a link and the state it restores

Identify which path, query and fragment fields can address a resource or select a view. Keep transient inputs and private payloads in their owning state. A link that locates a saved record still requires the recipient's normal access checks. Apply the route contract before loading or changing owner data. Do not treat incoming URL fields as authority or provenance.

Exercise both the link emitted by the interface and direct navigation to it. Compare opening it in the current tab, a fresh session, and a reload where those states matter. Test unknown keys, repeated keys, unsupported values, and combinations that are valid only in another view. Verify what is accepted, rejected or normalized under the project's contract. Include a saved-record link with a conflicting selector and a link without the local draft it would need.

Read back the final address, displayed selection and relevant request or state changes separately. Inspect copied links and any logs or referrers in scope for private values. A router or mocked host test can prove parsing and state selection within that fixture. Use the actual browser and native service when the claim depends on history, sharing across sessions, or access enforcement.

## Verify search discovery through to its destination

When the claim includes a search palette or navigation catalog, follow one intended entry from inclusion through query matching, presentation and activation. An entry in source can still be filtered out before indexing. Distinguish intentional exclusions from missing discovery. Use the owner's labels, synonyms and category policy; a successful exact-label query alone does not establish the promised vocabulary coverage.

Exercise the relevant keyboard and pointer paths. Read back the resolved resource, selected view and section after loading settles. Compare same-page activation, cross-page navigation and a copied or fresh-tab link when offered. Router-local state may activate a tab without surviving URL sharing or reload; verify that difference against the declared contract. Reuse the link checks above rather than treating a changed address as a reached destination.

For entries that perform an action, verify the currently registered handler, unavailable feedback and remount or cleanup behavior when relevant. Dispatch and durable completion remain separate claims. Use synthetic checks for pure index or registration rules, and the actual browser for focus, scrolling and navigation. Catalog contents, ranking, exclusions and native actions stay with the consuming product.

## Before handing off

- [ ] Did the real browser task reach its intended result at the relevant addresses and states?
- [ ] Do return, failure and retry paths work where required?
- [ ] Does the conclusion name the behaviors and platforms actually observed?
