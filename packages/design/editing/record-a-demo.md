+++
description = "Discover, rehearse and record a real browser workflow"
uses = [{ scope = "local", resource_id = "editing" }]
see_also = [{ scope = "local", resource_id = "editing/form-a-cut" }, { scope = "local", resource_id = "editing/check-delivery" }]

[guide]
exposure = "internal"
+++

# Record a real interface demonstration

A demonstration shows an actual action and its result. Decide which capability the viewer should understand, then discover the interface, rehearse the route and record it. Keep the capture script with the product's existing editable film sources.

## Find the route before scripting it

Inspect the actual controls, labels, input types, required fields and conditional states. Check option labels and values rather than assuming a nonempty value is a valid choice. For editable tables, connect each field to its column. Use this observation to write the interaction script.

Rehearse the complete sequence without recording. Fail on a missing, ambiguous or unusable control; correct the locator and repeat. Observe each result as well as each action. A script that continues after a missed click has not demonstrated the workflow.

## Capture the action and leave time to see it

Use a repeatable starting state and data approved for the intended audience. Keep account access outside the recording and check overlays, notifications and later frames for private content. Recording a workflow does not authorize a new payment, message or destructive action.

Give the viewer context, the action and a readable result. Move a visible cursor before activation, show meaningful typing, and leave holds where people need to read. Reinstall editorial cursor or caption overlays after navigation; make them ignore pointer input. Treat pauses as editorial timing, not a fixed test for application readiness.

With Playwright, prefer role and label locators tied to the inspected interface. Its actionability checks wait for a usable target; assert the visible consequence separately. Choose the viewport and video size explicitly so the recording does not unexpectedly scale a readable interface down. In the library API, recording belongs to the browser context:

```javascript
const context = await browser.newContext({
  viewport: { width: 1280, height: 720 },
  recordVideo: { dir: outputDirectory, size: { width: 1280, height: 720 } },
});
```

Those dimensions are an example, not a delivery requirement. Await `context.close()` to finish the video before saving or inspecting it. Each popup page has its own recording; capture and join it deliberately if the demonstrated route uses one. Watch the saved file through the action, result and end. A completed script alone does not establish a usable demonstration.

## Worked example

A filter narrows a report from twelve rows to four. Rehearse selecting the real option and assert the four resulting rows. In the recording, keep the filter and result count readable together, then hold on a row that explains the change. If the count never updates, repair the application or narrow the claim; adding an editorial “four results” caption would fabricate the missing response.

## Sources

- [ECC's UI demonstration method](https://github.com/affaan-m/ECC/blob/591ab5cbd3f2f65860ea91c226e410b1502c8e2e/skills/ui-demo/SKILL.md): discovery, rehearsal, visible input, navigation overlays and capture failures. Its timing and resolution defaults are examples; choose them for the actual film.
- [Playwright locators](https://playwright.dev/docs/locators) and [actionability](https://playwright.dev/docs/actionability): finding the intended control and waiting for it to accept input.
- [Playwright videos](https://playwright.dev/docs/videos): context recording, dimensions, finalization and per-page output.
