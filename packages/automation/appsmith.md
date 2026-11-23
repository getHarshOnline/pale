+++
description = "Check an Appsmith application, its saved revision and the operator journey."
see_also = [{ scope = "local", resource_id = "human-review" }]
official_sources = [{ label = "appsmith native documentation", url = "https://docs.appsmith.com/advanced-concepts/version-control-with-git" }]

[guide]

[stack]
+++

# Check an Appsmith application and its operator journey

Compare generated definitions with the deployed revision: object bodies, argument defaults, variable and widget bindings, and datasource references. Keep credentials server-side. Test reads and writes separately. Identify the request and current selection so a late response from an earlier scope is ignored. Check that queue refresh preserves selection and that Save and Discard show the correct revision and where it came from. Exercise the operator's actual journey in the target environment as well as checking generated files.

Use the selected export-comparison operation with a complete unpublished export to compare layouts, action configuration, collections and datasources. A saved-import and reload receipt supports the narrower check of source identity and page lint. Check interaction and the active revision separately.

When regeneration must preserve a native design, select its exact layout and the geometry and presentation fields to carry forward. Match widgets by unique name, type and parent; resolve identity changes before copying fields. Generated source continues to own data bindings and behaviour. Exclude widget structure, generated binding metadata and whole custom-widget documents from the fields being copied. For a custom widget, inspect the retained presentation and generated script separately. Use the native layout library through the consuming builder, compare the resulting export with its baseline, and test the operator journey.

## Bind review permission to the values being reviewed

Declare the fields a reviewer may correct and lock the others in the screen. Enforce the same allowed fields at the server; a disabled widget cannot prevent a crafted request. Preserve each value's source and location and distinguish the reported editor from an authenticated approver. Saving a correction records an edit, while approving a calculation or business action follows its own authority checks.

Bind approval to the relevant values, formula or policy, source revision and candidate revision. Changing any approved input invalidates that approval for the new candidate. Retain the old decision with the old values and require the appropriate new decision before a dependent action. Compare values canonically when checking an input fingerprint so a harmless reordering of object keys does not masquerade as a value change.

For example, a reviewer changes an amount after a calculation was approved. The updated screen must show that a new decision is needed, and the server must refuse a request that reuses the old approval for the changed amount. Test changing a locked field directly, racing an edit with approval, and receiving a late response for the earlier selection. Read back the saved revision and any resulting action separately.

## Verify layout preparation before updating the generated tree

Prepare layout and binding changes for the whole page before updating its generated tree. Test a later widget failing after earlier changes were prepared, for example because binding metadata is malformed or a custom-widget document is unusable. On failure, compare the entire tree with its original baseline. On success, check the carried fields, generated behaviour and export separately. Saving or deploying that export is a separate action in Appsmith.

Derive function argument metadata from the authored signatures and the selected page collection name. Preserve literal defaults and leave binding expressions as source for the runtime. Compare each generated action's metadata with its function source as well as checking the collection body. Define how unsupported parameter syntax is handled before generating it.

Start each generated collection from the entrypoints selected for that page and follow references through the authored function library. Resolve the reachable functions again after installing a generated provider or read adapter. The builder defines reference rules, provider bindings and emission order. This establishes reachability under those rules; use the resulting action and collection inventory and a separate application check to establish runtime behaviour.

## Worked example

An operator changes a value, then Save fails. Open the unpublished revision with a test datasource and exercise success and failure. Check that the draft survives and the next action is understandable. Compare the exported application and its dependency mapping before reviewing deployment.

## Before handing off

- [ ] Check the application revision and its configured dependencies.
- [ ] Walk the operator’s action and error recovery in the actual application.
- [ ] Distinguish saved, published and observed behavior.
