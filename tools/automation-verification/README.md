# Local automation export verification

Compare two explicitly selected local artifacts. The verifier reads local JSON
and emits a result with both input digests and a proof scope. It performs no
import, activation, deployment, browser session, or network request.

- `node verify.mjs n8n expected.json observed.json` checks workflow graph,
  node parameters and behavior flags including execute-once, retry count
  and delay when retries are enabled, credential reference IDs by kind,
  selected workflow settings, and pinned-data absence. Both inputs need a
  node array and a connection object, even when empty. Each node needs
  a nonblank type and a parameter object. Connection sources and
  destinations must be named nodes. An output's slots may contain
  `null` for a disconnected slot or arrays of edges, each with a
  destination input type and nonnegative integer index. Settings and pinned
  data may be omitted. If supplied, they must be objects. Added, missing and
  changed credential bindings count as drift. Label-only changes preserve
  the same binding. A malformed reference leaves the comparison open. The library accepts a caller-owned
  normalization function for known differences in a pinned native export.
- `node verify.mjs appsmith-saved expected.json observed.json unpublished-draft`
  checks the selected application's source digest, retained import and reload
  evidence, allowed deployment state, and lint evidence for every page in a
  nonempty planned inventory. This mode accepts a planned page without layout
  evidence and covers only the supplied receipt.

- `node verify.mjs appsmith-export expected.json observed.json` compares
  complete retained exports for unpublished layout and page routing,
  action configuration and argument metadata, JS collection bodies and
  variables, native plugin identity, and datasource drift. It compares
  retained deletion markers on pages, actions and collections.
  When supplied, the unpublished application page list contributes ordered
  membership, default-page and visibility checks. One-sided omission is
  reported in the page differences. If neither export includes this list,
  application navigation remains open. Each side needs a nonempty unpublished page inventory with layouts and DSL.
  Empty action, collection or datasource inventories can be supplied explicitly.
  Action entries need a configuration object. Collection entries need a body
  string and a variable array, which can be explicitly empty. Missing or
  malformed required evidence is refused. Results report identities and
  difference categories rather than raw values.

A completed comparison writes the v1 JSON result on standard output.
Exit zero means match and exit one means drift. Exit two reports an invalid
invocation or unusable evidence as a bounded JSON diagnostic on standard
error. Exit three reports an unexpected verifier failure there. Failure
diagnostics do not supply findings. Record the exit and output channels
separately. A match is evidence about those inputs. Establish the observed revision and
freshness with the native owner. Use a full native export or an appropriate
native read-back for claims about action metadata, datasources, and widgets.
Use a browser for interaction claims. Consumers keep their own artifact
selection, normalizations, and project acceptance rules.

## Statically check n8n Code nodes

Use [the Code checker module](n8n-code.mjs) from a build or verification
adapter. Create a checker with the consuming project's pinned ESLint
`Linter`, recommended rule map and Prettier `format` function. Pass that
project's format options to `formatCodeNodes(workflow, options)` before
writing an export. It updates the supplied workflow in place after every
formatted Code body passes lint. A formatter error or lint failure leaves
the supplied workflow intact. Use `checkWorkflowCode(workflow)` for a read-only
lint pass and `freeCodeReferences(body)` when choosing helper declarations to embed.

The checker requires a string JavaScript body on each Code node. It accepts
an omitted language or explicit `javaScript` and refuses other languages,
even when an old JavaScript body is retained. It checks the body inside
an anonymous async function so the wrapper cannot supply a helper name.
Its default globals
cover common n8n expressions and runtime names. A caller can pass
`globalNames` for its selected runtime. This establishes static coverage
only. The adapter still selects files, tool versions, styles and any
runtime qualification.

Use `resolveCodeHelpers(names, roots, referenceSource)` when assembling a
transitive helper closure. Supply the names the build can embed, the
explicit roots, and a function that returns each selected helper's source
or `undefined` for a terminal value. The returned names are in discovery
order. A consumer can emit them in its own library order. Serialize and
format declarations at the owner boundary.

## Carry forward a selected Appsmith layout

Import [the Appsmith layout module](appsmith-layout.mjs) from the native
builder. Call `preserveNativeLayout(generated, native, { geometry, presentation })`
with the selected page DSL roots and explicit field lists. The module checks
widget names, types and parent relationships, then prepares every widget
change before updating the generated tree in place. A reconciliation
error during preparation leaves that tree intact. It refreshes dynamic
binding entries for selected presentation
fields. For custom widgets with a source document on both sides, it carries
over native HTML and CSS and preserves generated JavaScript. The builder
selects the baseline and writes the resulting export. The selected field
lists must exclude widget names, types, children, dynamic binding,
property and trigger lists, and whole custom widget documents. These
fields are rejected before the generated tree changes.

## Derive Appsmith action arguments

Import [the argument module](appsmith-arguments.mjs) and call
`appsmithFunctionArguments(fn, collectionName)` with an authored function or
its source and the native JS collection identifier. It accepts parenthesized
simple named parameters and the supported default syntax. A complete single-
or double-quoted default becomes a string value. Other supported defaults
become Appsmith binding source. Direct `this.member` and `this[key]`
references use the supplied collection name. Quoted text and member
properties of other objects keep their spelling. Binding expressions never
run in this helper. Destructuring, rest parameters, interpolated templates,
and slash syntax in defaults require explicit metadata.

## Select reachable build dependencies

Use [the shared traversal](dependency-closure.mjs) as
`dependencyClosure(roots, dependencies)`, where the callback returns the
ordered direct edges for a selected name. The returned set visits each
name once. Discovery follows root and edge order by default. Set
`{ depthFirst: true }` as the third argument when the builder emits
depth-first discovery order. The n8n checker supplies lexical references.
An Appsmith builder supplies its page function references and can rerun the
closure after binding generated functions. The consumer chooses the roots,
reference interpretation, unknown-name policy and output order.
