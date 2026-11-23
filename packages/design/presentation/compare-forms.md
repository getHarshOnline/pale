+++
description = "Use it to check what the reader receives in each affected form, with expected facts sourced independently of the template that renders them."
uses = [{ scope = "local", resource_id = "presentation" }]

[guide]
exposure = "internal"
+++

# Check what the reader receives in each form

## Verify what the recipient receives

Choose the affected forms and supported conditions from the change and its acceptance criteria. Retain valid existing evidence for untouched consumers. Source the expected facts independently from the selected payload or owning account, rather than constructing expectations with the same template that renders them. Normalize only the transformations the source permits.

Inspect the actual output at the level of the claim. Source markup can establish that text was emitted; visible browser output can establish whether a stylesheet hid it; accessibility inspection and real input have their own coverage. Compare machine-readable values and structured relations with the visible claims where both are delivered. Hidden canonical text or `textContent` alone cannot prove that a reader saw a qualification. An accessible name is not a substitute for the whole visual or interaction judgment.

Use `innerText` from a rendered container when the question is what text its layout presents, and inspect accessible names separately. Do not query an arbitrary hidden node and treat its returned string as visibility evidence: for an element that is not being rendered, `innerText` can return its text content. Nor does a matching string prove sufficient contrast, absence of clipping or that an overlay leaves the text readable. Inspect the rendered view and the relevant input path alongside extraction.

Check that no presentation introduces a new figure, strengthens status, loses a unit, changes a destination or attaches a qualification to the wrong object. Where emphasis could imply a comparison or alter comprehension, inspect the neutral and intended treatment. Test missing-field records and the capability conditions the product actually supports. No unrelated full matrix is required for a bounded source correction.

Establish that a decisive comparison can detect its named fault: an appropriate known-bad case or controlled altered output can expose a circular oracle. Reuse that qualification while its inputs and check remain valid. Do not relabel an ordinary passing test unrun merely because every scenario was not mutation tested. Record the boundary actually established and any missing observation. Correct shared anatomy or transformation once, follow its actual consumers and retain owner acceptance separately from structural and rendered evidence.

## Verify the same information in forms that can legitimately differ

Choose a fixture whose fact, unit, source, scope and status matter and are known independently of the render. Emit it as the forms used by the change, such as a row, a preview, an inline reference and a full artifact. Compare the amount of context each form can omit without changing the claim. Verify that two adjacent values still read as separate measurements when their grid stacks. Check a missing date, an unknown unit and a closed or superseded status instead of only complete records.

Inspect both the reading surface and the structured output. A date known to the month should not acquire a day in metadata. A value known to one decimal place should not gain implied precision elsewhere. Compare the ordinary and accent-muted presentation when accent could alter the apparent relation. Plant one wrong unit or source in a fixture to see whether the comparison would catch it. Read the rendered relationship as well as matching its values.

Include two independent figures and their relation in the fixture, so the check can catch a ratio invented by layout as well as a changed number. Compare both directions: every required supplied field is present or appropriately linked, and every rendered figure, sample size, date and source has an authorized origin. An injected sample size tests a different failure from a missing unit. Record the failing field, form, identity, presentation state and context when those dimensions affect the result. A successful negative check qualifies that assertion; it does not prove every possible rendering fault detectable.

## Worked example

Copy an item’s figure and source from the wide and narrow render. Compare the actual units and the qualification beside the number. Open the target from both routes, then run the script-off view. If a qualifier only appears in a hover overlay, move it to the form that supplies the claim and repeat the check.

## Sources

- [MDN `innerText`](https://developer.mozilla.org/en-US/docs/Web/API/HTMLElement/innerText): rendered text extraction, its distinction from `textContent` and the hidden-element exception.
- [W3C JSON-LD 1.1](https://www.w3.org/TR/json-ld11/): identifiers and typed relations when comparing a structured representation with its source and visible form.
