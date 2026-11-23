+++
description = "Compare saved n8n workflows, Appsmith unpublished exports or saved-import receipts with the configured verifier."

[task]
uses = [{ scope = "external", resource = { package_id = "pale.engineering", resource_id = "execution" } }]
outputs = ["Exact artifact digests, what the comparison covers and the differences found"]
capability_policy = { scope = "local", resource_id = "export-policy" }
capability_requirements = [{ id = "compare-retained-artifacts", description = "Compare the exact retained artifacts and report differences.", capability = "automation-export-verification", recipe = { scope = "local", resource_id = "verify-exports" }, effect_class = "execute", policy = { kind = "required_block" }, failure_posture = "fail_closed" }]
+++

# Compare saved workflow and application exports

Select the exact planned artifact and the capture retained from the system that owns it. Record the environment, revision, capture method and time. Keep the original artifacts and their identities. If moving between environments requires an intentional translation, retain that translation and the derived input identities alongside the originals.

Choose the mode supported by the available evidence.

| Mode | Required inputs | What the comparison can establish |
| --- | --- | --- |
| `n8n` | Planned and retained native workflow exports for the same workflow | Selected graph, parameters, credential references, behavior flags, settings and pinned-data checks. Normalize only as documented for that exact export format. |
| `appsmith-export` | Planned and retained complete unpublished application exports | Layouts, page routing, selected action configuration and bindings, collection bodies and variables, plugin identity, and datasource agreement. |
| `appsmith-saved` | Exact planned application source, saved import and reload receipt, and the owner's accepted deployment states | Import and reload for the expected source digest, plus lint evidence for each expected page. |

Appsmith export mode needs a nonempty unpublished page inventory and layout DSL for every page on both sides. Action, collection and datasource inventories must be explicit, though each may be empty. Every action needs a configuration object. Every collection needs a body string and a variable array, which may be empty. Missing or malformed required evidence leaves the comparison open even if both inputs omit it. Saved mode needs a nonempty planned page inventory and lint evidence for each page; it does not require layouts or execution payloads.

## Verify retained Appsmith action placement and binding

For full exports, compare page slugs and visibility, action and collection plugin identities, and deletion markers. For each action, compare the client-side execution flag, collection ID, qualified name and configuration. Preserve supplied values and distinguish omission from an explicit value. A saved-import receipt supports its narrower checks when these fields are unavailable. Test action execution and operator interaction through the application itself.

## Verify retained unpublished application navigation

Check the application's ordered page references separately from page definitions and layouts. Where supplied, resolve every `exportedApplication.pages` reference against the unpublished inventory according to the supported export format. If the capture uses a different identifier form, provide and retain an explicit ID mapping. Compare order, membership, default page and visibility. Reject duplicate, unresolved or malformed references and contradictory default selections.

A list present on only one side is a page difference. If both captures omit it, page definitions can still be compared, while application navigation remains unchecked. Published page lists and the live landing page need their own source-system observations and browser checks.

## Verify the retained n8n graph and behavior

Require explicit node inventories and connection maps on both sides; an explicitly empty graph is valid. Two missing maps cannot establish agreement. Every node needs a nonblank type and a parameter object, which may be empty. Resolve connection endpoints against that export's own inventory. Each output slot may be null or an edge array; each edge needs a destination input type and a nonnegative integer index. Preserve slot positions. Matching malformed nodes or connections still leave the comparison unresolved. Settings and pinned data may be omitted, but must be maps if supplied.

Compare credential-reference IDs by binding kind in both directions. Added, missing or changed bindings are differences. A changed display label with the same kind and reference ID preserves the binding. A malformed reference leaves the comparison open. Keep credential contents out of artifacts and findings.

Inspect execute-once and retained retry count and delay as node behaviour. Compare count and delay when both nodes enable retries. An omitted active setting differs from an explicit value in the retained evidence. Whether they behave alike depends on the pinned engine and its defaults.

### Compare the selected Code node body

For the core Code node, read the language selector and the body it selects. JavaScript uses `jsCode`; Python uses `pythonCode`. An omitted selector means JavaScript in the supported export shape. Keep legacy and native Python selectors distinct. The selected body must be present and a string on both sides. A stale body field for another language cannot satisfy this requirement. Preserve Python whitespace, including leading indentation. Compare other parameters separately under the caller's documented normalization. This checks exports; confirm interpreter availability and actual behaviour in the selected runner. See the [Core Code node declaration](https://github.com/n8n-io/n8n/blob/master/packages/nodes-base/nodes/Code/Code.node.ts).

### Compare retained workflow settings in both directions

Compare every supplied setting after documented normalization. An absent settings map and an explicit empty map both contain no settings. A key present only in the native export is a difference even when the plan deliberately omitted it. A default or environment translation needs evidence for the pinned version before it may be normalized. Report retained-value differences separately from the running workflow's observed behaviour.

<a id="bind-the-command-and-its-execution-evidence"></a>

## Identify the comparison command and its result

Use the configured local verifier when available. Inspect input digests, coverage, match status and findings. If comparing directly, use the same mode and read the same evidence from the owning system. Preserve the command's nonzero result. Missing input, an incomplete capture or tool failure leaves the affected comparison open.

Apply the selected guidance for recording native execution evidence. Identify the imported comparison module and exact artifact digests as well as the launcher and runtime. Preserve the complete report and exit status. A command name or contract version does not identify the comparison module's bytes.

For a batch, keep every selected artifact's identity and evidence state. Compare acquired captures; mark missing native captures unavailable and exclude them from difference findings. If a required member is unavailable, the aggregate stays incomplete and returns nonzero. An intentionally empty exported workflow is still a captured artifact with its own identity.

Check active revision, applied changes and browser interaction separately. Matching exports or a saved receipt do not establish these outcomes.

<a id="separate-comparison-drift-from-unusable-evidence"></a>

### Distinguish differences from an invalid comparison

A completed comparison writes its v1 result and both input digests to standard output. A match exits zero. A completed comparison finding differences exits one and still supplies its report. Inspect that report.

An invalid invocation or invalid evidence makes the shared command exit two. Read its bounded diagnostic on standard error: an unreadable artifact or invalid JSON is identified by input role, with both digests when both artifacts were read. The role may remain unresolved when validation concerns the relationship between the artifacts. An unexpected verifier failure exits three. Neither failure supplies a comparison report. Preserve its diagnostic and exit status and leave the comparison open. Missing findings do not mean a match, and a nonzero exit alone does not prove a retained difference.

<a id="verify-the-failure-carrier-for-the-selected-host"></a>

### Check that the host reports command failure

Run a match and a known difference through the same selected command. Match the model-visible tool to its native event name, session, invocation, working directory and exact command. Preserve the host's returned result separately from its native event class. Conversation status and response text cannot supply a missing execution outcome.

If the operation requires the host to block further work on failure, verify that the selected route distinguishes failed execution from success. When both native events have the same shape and no trusted status, leave that route unapproved for this purpose. A direct comparison may still establish the artifact result through its own route. Test any replacement route with the same negative case before relying on it to block work.

## Before handing off

- [ ] Identify the two exports or the import result being compared.
- [ ] Use the verifier that matches their kind and scope.
- [ ] Inspect material differences and name the runtime checks still needed.
