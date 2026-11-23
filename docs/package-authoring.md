# Select and author Package resources

Write a Package when several projects need the same role, procedure or policy. The Package owns that method; a workspace chooses where it applies. Start with an exported resource, follow its prerequisites and change its source rather than the generated host files. For ordinary repository generation, use [Add coding instructions to a repository](https://rawthoughts.in/pale/materialize-repository-guidance/).

## Shape a family around the work

Give each topic an entry that says when to use it, what decision it helps make, where it sits among the families and which kind of duty selects it, so the page knows its place. An index shows each entry's kind: a `doctrine` is a guide read before deciding, an `operation` a task with declared outputs, a `role` the instructions for one kind of actor, a `stack` the syntax of a library or tool and a `policy_family` a set of rules a project can apply. `tool_recipe`, `capability_policy` and `governing_order` are typed data, described under [Put declarations beside the method](#put-declarations-beside-the-method). A short table can point to the references needed for different tasks. Give each reference one job, with a snippet, before-and-after comparison or filled table when that makes the method usable. Split where a reader can use a part independently; merge explanations that always need to be read together. Keep the features, cases and reasons while removing repetition. Length alone decides neither.

Use [Write an instruction an agent can follow](../packages/agent-work/instructions.md) when drafting the method. Each step that checks or keeps something says where its result belongs and when the step is finished. A result usually goes into the changed artifact, a maintained test or the reply. When recording a decision is the task, name its home, such as the repository’s decisions page. Keep records that a product or later step actually consumes; avoid creating a report just to show the instruction was followed.

Close an instruction path once with three or four questions about the result. Reference leaves do not need another checklist, and the questions should not replay the steps. For header-authored pages, the renderer chooses the page’s closing questions or the single set supplied by required reading; optional reading supplies none. Preserve unique instructions from a former leaf checklist in the shared method before removing the repeated checklist.

Put library and tool syntax in a page with a `[stack]` use. A stack can share its body with a guide. Show commands or APIs for the supported version, how to read their output and a worked example. A Langfuse overlay, for instance, can find observations for a known trace ID and distinguish an empty result from fields the query never requested.

Declare required reading with `uses`, adding task-specific reading under `[task]`; use `see_also` for optional reading. Across Packages, use `scope = "external"`, declare the dependency and refer to an exported ID. A same-Package guide may be internal. Update references and consumers together when changing an exported name. Do not keep a second page merely to repeat the new page under an old name; any required compatibility belongs in the tool.

The current corpus declares methods and their relationships in page headers while listing sources and resources explicitly in each manifest. A page is the one home for its guidance: each manifest lists it once, and no second copy of it stays under an earlier name or declaration file. The compiler also accepts the compact inventory below; keep the manifest and source declarations in agreement. Let the renderer print prerequisites instead of copying its generated text into the body. For direct source reading, the family `INDEX.md` lists entries and prerequisites, and each entry links its task guides. Use `pale packages show pale.design:animate --root /absolute/distribution` to preview a body with its reading links. Run `pale packages check --root /absolute/distribution` to check declarations, body links, heading fragments and index freshness. Add `--write` to regenerate family indexes after those source checks pass. The compiler supplies the catalog, body preview and indexes through the same reader; these commands need no workspace selection.

To check documents that refer to the library, repeat `--caller-root` for each directory: `pale packages check --root /absolute/distribution --caller-root /absolute/notes --caller-root /absolute/repositories`. The check reads Markdown links into this distribution’s `packages/` directory and qualified resource IDs in inline code, such as `pale.design:animate`. It rejects missing files, missing heading or anchor targets, and unknown IDs; fenced examples and unqualified code words are not resource declarations. An invalid caller prevents `--write` from updating any index. Links into generated indexes are checked against their proposed replacement, so regeneration cannot silently remove a caller’s heading target. Diagnostics identify the caller and line without copying the linked URL or query.

Caller capture reads `.md` and `.mdx` files without following symlinks. It skips `.git`, `target`, `node_modules` and the distribution’s own Package tree, whose links are already checked. Use `--exclude-caller Records` to exclude that name at any depth, or `--exclude-caller archive/old` for a subtree relative to each caller root. Choose exclusions for the actual records and generated output; excluding a live caller removes it from the check. A capture error is a failed check. The command accepts at most 16 caller roots and 256 exclusions; each tree is limited to 250,000 traversed entries, 64 directory levels, 4 MiB per file and 128 MiB of retained text, with 64,000 documents and 256 MiB across the request. This explicit authoring scope does not configure a workspace or install a recurring check.

Write for the person doing the work: who acts, on what, with which result. When a technical word hides the action, explain the action. Keep a technical term when the reader needs its precise meaning. Dates and versions belong where they change the method; run logs, hashes and an account of writing the guide belong elsewhere. Use a full stop in place of an em dash.

Keep each prose paragraph on one source line. `pale packages check` reports wrapped paragraphs in live resource bodies and refuses index writes until they are formatted. It leaves code, maths, tables, front matter and deliberate Markdown hard breaks alone; documentation-only files are outside this check. The repository check, `python3 tools/package_navigation.py`, also runs Prettier with the repository configuration and `proseWrap: never` on the compiler's live-body list before checking or regenerating indexes. It needs Prettier on `PATH`; use `--formatter /absolute/path/to/prettier` to select another installed executable. A missing or failing formatter fails the check, including with `--write`. Both commands leave authored text unchanged. Run the formatter's `--write` only on the reported files you are editing; documentation files and generated indexes are outside the formatter check.

| Before | After |
| --- | --- |
| Verify the selected native projection. | Open the generated page and check where the button takes you. |
| Retain evidence of the interruption boundary. | Close the drawer while it is opening; say in the review whether it stopped where intended. |
| Check the exact dependency closure. | Install in an empty checkout and list the failed imports in your reply. |

A resource description names the job in the words someone would use to ask for it. Its `outputs` say what the person gets back and where.

For optional wording prompts, run `python3 tools/package_text.py --word-list /path/to/words.txt` from the repository root. The caller supplies a UTF-8 file with one word per line; a final `*` also matches its continuations, and blank lines are ignored. Keep that list with the workspace's writing choices. The tool has no built-in vocabulary. Without a list it reports page sizes only. Add `--baseline /path/to/earlier/checkout` to compare both versions with the same list. The report identifies whether a list was supplied and gives counts and sentences to reread; wording and size warnings never fail the command. Keep a word when its precise or ordinary meaning helps, and rewrite the action when the word obscures it. Reading the method decides whether it works.

## Review the method from its sources

Start with who will use the method, the decision or action it should improve and the sources behind it. Read those sources before changing the structure. Distinguish original reasoning, conventional advice, quotations and passages too close to a source. Changing synonyms or shuffling sentences leaves borrowed expression intact. Explain the idea in a structure and examples suited to this reader, and retain the attribution and licence notices the source needs. If its origin or permission is unresolved, identify that passage in the review and hold its publication until the question is settled.

Look for the judgment that makes the page useful: the conditions under which a rule applies, why it exists, the trade-off, observable criteria, failure cases and an example that exercises the reasoning. A heading followed by slogans needs that missing substance. Decide whether the existing page needs deeper explanation, a separate reusable part, a merge with its duplicate or a different home. Keep needed history when replacing a method; another owner's decision goes to that owner. Put findings and unresolved choices in the requested review, with enough detail to make the change, rather than creating a second catalogue of the corpus. End the review of a page with one decision about it: keep it, deepen it, split it, merge it with its duplicate, move it to its owner, rederive it from the principle in place of the source, add the attribution it needs, hold it back until its origin is settled, replace it with a successor, remove it, or hand its mechanics to another owner.

Watch for one rule repeated in several roles, Markdown restating a declaration field by field, tiny pages that only point at each other, or a large page mixing independently changing subjects. Keep one home for shared meaning and link its users. Explain relationships instead of duplicating their declarations. Keep machine paths, credentials, project status and a particular consumer's model choice in that consumer's configuration. Tool-specific syntax and supported versions belong in the relevant overlay. A method must leave another owner's effects and permissions with that owner.

After writing, check declarations, links, examples and contradictions with neighbouring methods. Read for what a capable person can now decide or do. Mechanical checks answer whether the Package can be accepted and rendered; use in a project and permission to publish still need the checks and decisions for those tasks.

## Put declarations beside the method

A compact Package omits both `sources` and `resources` from `[package]`. The compiler derives its inventory from the retained files; supplying just one inventory is invalid. Keep release metadata and dependencies in `package.toml`:

```toml
[package]
id = "example.review"
version = "1.0.0"
schema_version = 1
title = "Review methods"
description = "Review changes and their evidence."
compatible_compilers = ["pale-surface@1"]
```

Start an instruction page with a `+++` TOML header, followed by its Markdown body. For example, `review.md` can supply both a guide and a task from one body:

```markdown
+++
description = "Review a change against its intended behaviour."
[guide]
[task]
outputs = ["In the reply: actionable findings with evidence, or what was checked and what remains uncertain."]
+++

# Review a change

Read the intended behaviour, trace the changed caller and check the result against that intent.
```

The relative path without `.md` supplies the ID, here `review`. A header with no explicit use declares a guide. Adding `[task]`, `[role]`, `[policy]` or `[stack]` declares those uses; include `[guide]` when the same page must also be selectable as a guide. Each use retains its own kind and visibility even though it shares the body. Page `uses` declares required reading for every use; `task.uses` and `role.uses` add their own reading. `see_also` stays optional. References use the same local or external forms shown below. A child such as `review/security.md` also reads its nearest authored parent topic, which must declare a guide. The host renders these prerequisites as Read first links.

A page may finish with `## Before handing off` and three or four unchecked questions. Keep the questions in the method that owns them. Generated page uses put that set after host, tool and reference instructions. When a page has no set of its own, its required reading can supply one; optional reading cannot. If required methods supply different sets, write the questions that close this page’s task instead of relying on an arbitrary choice. Useful checks that belong inside the method stay in its prose. Check the generated instruction, including its final questions, rather than requiring a repeated checklist in every source leaf.

Keep data without an instruction body in typed `.toml` files. Their path without `.toml` supplies their ID, and `kind` selects `tool_recipe`, `capability_policy` or `governing_order`. For example, `read-only.toml` declares a capability ceiling:

```toml
kind = "capability_policy"
exposure = "internal"

[policy]
permitted_capabilities = ["repository.read"]
permitted_effects = ["read"]
```

The nested payload is respectively `recipe`, `policy` or `governing_order`; its existing rules still apply. Unknown kinds and fields are rejected. Plain Markdown without a header and other supporting files remain documentation. Every retained file contributes its exact bytes to the release identity, including documentation and unselected data. Nested Packages and competing declarations for one path-derived resource remain invalid.

Explicit inventories can also list header-authored pages and typed data alongside older declaration files. A typed data source must declare exactly one resource whose ID, kind and exposure match the file. Its top-level `kind` selects the same grammar for compilation and editor assistance. Files listed as documentation contribute to release identity without supplying resources. Use that role for material that travels with a Package, such as licence texts and attribution, and not as a second home for guidance that a declared page already carries.

The compiler, catalog and host renderers accept these pages. Editors register a page as `package-page` with language `pale-page`, and typed data as `package-data` with language `pale-toml`; [the language server](../interfaces/lsp/README.md#connect-an-editor) supplies header assistance without parsing the Markdown body as TOML. Header-authored pages declare guidance in every family; Engineering, Automation, Publication, Design and Finance also have typed data. Design's explicit inventory lists its declared pages and typed data, with its licence and attribution files as documentation. A compact Package derives membership from its sources and does not mix in a separate resource inventory.

## Select a resource for the actual task

A Package's `package.toml` identifies its release and exact dependencies. Membership comes either from its explicit inventories or from the compact source declarations above. Each resource has an ID, kind, definition source and exposure. A workspace can select exported resources; an internal archetype can support a concrete operation without becoming a workspace entrypoint.

Making a Package available through the project's `package_seeds` and exact distribution lock does not select every resource in it. The project's capabilities file selects operations, doctrine, roles, stacks and run modes by their appropriate semantic fields. Repository declarations separately select roles, stacks and policy families. A dependency on another Package makes its declared release available; it does not load all that Package's instructions.

For a project that already selects the exact Engineering release, this capabilities-file fragment selects its exported verification operation:

```toml
[semantic]
operations = [
  { selection = "exact", resource = { package_id = "pale.engineering", resource_id = "testing/repository" } },
]
```

The [Engineering manifest](../packages/engineering/package.toml) exports that ID. Its [task page](../packages/engineering/testing/repository.md) declares the operation and its dependencies. Read first links reach the [testing method](../packages/engineering/testing.md) and [execution method](../packages/engineering/execution.md). It also requires the `project-test` capability through the [repository verifier recipe](../packages/engineering/repository-verifier.toml). Selecting it does not supply the project's command or a working provider.

Operation exposure is a separate consumer choice. For example, append this binding to the same capabilities file to offer the selected operation only through explicit user selection:

```toml
[[operation_bindings]]
operation = { package_id = "pale.engineering", resource_id = "testing/repository" }
[operation_bindings.binding.selection]
kind = "user_only"
aliases = []
```

Without an operation binding, the selected operation remains unbound. This exposure choice still supplies no `project-test` provider, command authorization or execution evidence. Private bindings can narrow Package ceilings; they cannot rewrite the operation's requirements, fallbacks or evidence contract.

Choose by the requested decision. Engineering's `review` reviews an exact change and its evidence; `testing/repository` requires a native check. A task that only needs a judgment resource can select doctrine instead of an operation. For example, this alternative fragment selects Design's rendered interface judgment:

```toml
[semantic]
doctrine = [
  { selection = "exact", resource = { package_id = "pale.design", resource_id = "inspect" } },
]
```

These fragments belong in a complete workspace with its selected sources and run policy. Repository guidance can render selected methods, roles and policies without a runtime provider or execution path. It refuses selected runtime operations rather than turning them into plain guidance silently. Runtime rendering separately needs the selected terminal path, provider-binding record and local machine binding. Detecting an installed host does not create that provider record. See the [workspace contract](workspace.md) for configuration and repository planning.

Select more resources when the task needs them, rather than because descriptions share a word. An explicit `all_from_package` selector is broader: it selects every exported resource of that semantic field's kind from the named Package.

## Read composition as part of the contract

Typed references tell the compiler what belongs together. They also tell a human reading the sources which guidance must be read before applying a method.

| Header field | Meaning |
| --- | --- |
| Page `uses` | Make required guides available through Read first links for every use of the page |
| `[task]`, `[role]` or `[stack]` `uses` | Add required reading for that use |
| `extends` under a use | Include the declared parent of the same kind |
| `see_also` | Name optional reading without selecting or installing it |

The older separate declarations remain supported: operation `doctrine_refs` and guide `requires` compose their required bodies, while `see_also` keeps reading optional. Header-authored `uses` keeps those methods in their own linked pages.

Design’s [rendered inspection method](../packages/design/inspect.md) requires [browser access](../packages/design/access/web.md), which itself requires [accessible tasks](../packages/design/access.md). Each required method is available once through a Read first link even when several selected resources reach it. Motion, responsive recomposition and independent design review remain optional references from that entry. Their availability does not require performing all those tasks for every rendered inspection.

A local reference uses `{ scope = "local", resource_id = "..." }`. A cross-Package reference names the owner explicitly:

```toml
uses = [
  { scope = "external", resource = { package_id = "pale.engineering", resource_id = "execution" } },
]
```

The importing Package must declare the exact dependency and reference an exported resource of the expected kind. Renaming a folder, adding a prose link or placing a file beside another Package does not create that relationship.

## Keep governing relations attached to the selected use

Governing scopes and endpoints can name different kinds of resources. Add `kind` when a page declares several uses; omitting it is valid only for a target with one declared use. Admission preserves that kind in the resolved relation, including scope, ordering, narrowing and obligations. A capability ceiling's `policy` field already requires a capability-policy resource.

For example, a compact Package with a `change` role and two policy pages can declare this reasoning order in `order.toml`:

```toml
kind = "governing_order"

[governing_order]
schema_version = 1

[[governing_order.order]]
scope = { scope = "local", resource_id = "change", kind = "role" }
purpose = "reasoning"
before = { scope = "local", resource_id = "change/impact", kind = "policy_family" }
after = { scope = "local", resource_id = "review", kind = "policy_family" }
```

Selecting the `change` role activates this relation and its dependencies. Selecting only that page's guide leaves the role's order inactive. An external endpoint adds `kind` beside `resource`, and the chosen use must be exported by the declared dependency. Order and narrowing endpoints must have matching kinds. The compiler rejects an omitted kind on a shared page rather than choosing a use from the other endpoint.

## Author the source that owns the behavior

Extend an existing resource when the change belongs to its task. Add a resource when it has a distinct selection or ownership boundary. Keep project commands, service addresses, credentials, accepted visual references and local acceptance criteria with the consumer; they are inputs to a reusable method.

For a new operation in an existing Package:

1. Write the method in Markdown, keeping the reasoning and failure cases needed for the task. Compose an existing base through a typed reference instead of copying its body.
2. Declare the page’s task use, description, required reading and expected outputs in its header. Its path supplies the ID. Add a capability requirement only when it needs one. Its capability policy sets the permitted scope; the consumer supplies providers and may narrow it.
3. Let a compact manifest derive membership from the pages and typed data, or list them in an explicit inventory. An explicit inventory must agree with every declared use and exposure; supporting documentation supplies no resources.
4. Check the changed Package, then review each consumer's updated pin. Leave an existing pinned release unchanged. The compiler's release-content identity identifies the actual source; a version label or checkout revision alone does not.

Engineering’s [review page](../packages/engineering/review.md) declares a task, role and policy over one method, with the guide kept internal. Its task needs no native capability:

```toml
[task]
roles = [{ scope = "local", resource_id = "review" }]
outputs = ["In the reply: findings ordered by consequence, with the supporting code or check and any uncertainty.", "Checks run, what was unavailable, and the conclusion supported by this review."]
```

This is the task section inside that page’s header, not a standalone declaration file. The full header requires the change method through `uses`. The body owns the review method for all its uses, and generated host skills remain projections to inspect. Older families can still declare internal archetypes and exported concrete operations in separate tables; migrating them must preserve their composition and capability requirements.

Doctrine additionally declares its allowed placement. `standalone_judgment` can appear as an on-demand skill, `operation_procedure` belongs inside a selected operation, and `startup_policy` belongs in sealed startup context. Keep that distinction when adding references: a composed procedure does not thereby gain a standalone skill destination.

## Keep body links aligned with typed references

A local Markdown link in a resource body must reach a manifest-owned standalone doctrine body in the same Package, with an explicit typed relationship from the referring resource. Use an inline link such as `[method](method.md)`, optionally with a heading fragment. An operation uses `doctrine_refs` for supporting doctrine; a doctrine entry uses `requires` for prerequisites and `see_also` for optional references. A prose link alone never adds selection.

When the target is selected, the renderer rewrites the link destination to its host skill path, retaining the label, optional title and anchor. When the target is unchosen, it prints the target's title, Package and source path, including any anchor, instead of installing the page or emitting a broken link. The authored source and its digest stay unchanged. External URLs and code examples retain their authored text. Reference-style local links, paths outside the Package and targets that have no standalone skill are refused. Cross-Package composition uses typed references rather than a guessed relative checkout path. When supporting doctrine is composed only inside an operation, follow its typed declaration and body source; do not invent a projected standalone link for it.

## Check the changed boundary

From the source repository, the maintained checks are:

```sh
python3 tools/package_navigation.py
cargo test --locked --offline -p pale-application --test packages
cargo test --locked --offline -p pale-application --test compile current_corpus
```

The repository runner checks formatting, declarations, reading links and index freshness. The first Rust suite checks that each Package lists its own files and that the compiler accepts them. The second exercises current Package journeys on both hosts, including required composition, optional reading, link destinations and missing-provider errors. For a new resource or relationship, add a focused case when the existing journeys do not cover its behavior. Inspect the affected host output and the refusal cases relevant to the change.

Configuration and file review belong to the [workspace contract](workspace.md). Running the project’s own check is a separate step. The verifier recipe needs a real exit status, and a pass shows only what that check tests. Inspect the generated instructions as well as running the compiler checks. Actual host invocation, task quality, outside effects and owner acceptance each need their own observation when they are part of the task.

[Package source bundles](package-source-bundles.md) covers copying and checking selected source files for distribution. Extraction makes the source available. Choosing a resource for a project and checking that it works there are separate steps.
