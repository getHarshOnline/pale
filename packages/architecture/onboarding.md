# Understand a codebase for the task at hand

Give a new reader enough of the repository's structure and reasoning to begin their task and resume it later. Start with that task, the repository's instructions and current revision. Trace responsibilities, real entry points, data and control flow, outside dependencies, generated files and the project's build and test commands. Read current code and configuration before claiming a convention.

For a code tour, follow one execution or reasoning path in causal order. Point each important step to an actual file, symbol or location, explain its purpose and show what depends on it next. A file inventory alone does not explain how the program works.

Put the useful map in the requested handoff or reply: entry points, owning modules, contracts, conventions, commands, protected or generated files, open questions and a safe first change. Link the source and identify its revision so the next reader can spot stale observations. Stop when they can start the named task without inventing architecture or permission.

## Verify a derived code index against the selected source

Use an index to answer a specific structural question. Establish its provider, repository, worktree, completed generation and file coverage. Compare relevant indexed content with the actual source bytes. Account for excluded, omitted, failed and truncated files. A refresh acknowledgement, timestamp, status or matching file size does not prove which bytes were indexed.

Limit permitted reads before the indexer discovers files. Resolve the allowed file list and roots independently of its scan and ignore rules. Check whether links, generated inputs or untracked files expose additional material. When the original tree is broader than the permission, an isolated copy containing only permitted regular files can supply the input. Keep the source-to-copy correspondence with the index's input metadata, then verify indexed paths and coverage. Test an unpermitted local file and an in-tree link to an unpermitted target: both must be excluded before scanning and absent from results.

To check refresh behaviour, use a known matching baseline, a changed symbol and a removed symbol or file. Include a byte change that preserves size and modification time. Verify that new structure appears and obsolete results disappear. Recheck the source after the operation if concurrent edits could have invalidated the comparison. Report an unverified refresh as a gap for the index owner.

Confirm that a returned symbol, its location and displayed excerpt describe the same indexed generation or source snapshot. Some tools combine stale coordinates with excerpts read from the current file. Do not use that mixture as proof of current structure. A permitted direct source read can answer its own question; report it separately.

Treat graph edges as leads until the relevant caller, producer, contract and consumer are confirmed. Read required configuration, templates, generated files and other omitted inputs through their own entry points. Keep those observations distinct from index results. A missing hit says nothing about files outside verified coverage. If the task requires a complete current index, an unavailable index leaves that requirement unresolved. Its owner handles index administration.

### Verify the exact repository index before use

Identify the permitted canonical repository and worktree roots, actual index location and generation, provider and operation. Check which index the tool will open before a query, refresh or watch. A supplied path is only a request until the tool's resolver is verified. If it searches ancestors or defaults, use a confined entry point or isolated layout that cannot open another index.

Report a missing, invalid or inaccessible index before any operation can fall through to another one. Keep reading a generation, proposing a rebuild, publishing a generation and deleting one as separate actions. Direct source reading retains its own permission and limits; it does not repair the index or satisfy a requirement to use a verified index.

Test an indexed parent and unindexed child with distinguishable markers. Query the child and attempt a child refresh. Confirm that no parent results appear and the parent's index remains unchanged. Repeat with a verified child index and a missing root outside the parent. Inspect selected locations and generation changes where the tool exposes them. If selection or confinement cannot be verified, report that requirement as unresolved.

### Verify the exact query anchor and trace the contract

Translate the task into a precise source location: file and qualified symbol, configuration key or schema identifier. Match the returned definition's canonical worktree-relative path, symbol and scope, and verify its source in the indexed generation. Check whether the requested scope was honoured separately from exit status and result count. A suffix match, widened search or lone namesake after a failed file filter does not resolve the requested anchor. Test both an exact match and a missing anchor with one remaining namesake.

Trace the producer through exports or bindings, source descriptors, generated contracts, loaders or validation, and actual consumers. Read each repository at its own revision and connect them through the contract they share. Use a test-only graph edge to locate the production caller. Include configuration, templates, styles and SQL when they implement the behaviour. Confirm surprising edges in source before widening the search.

After changing the code, retrace the affected path for competing producers, duplicate definitions, misplaced responsibilities, stale paths or generated files, and affected consumers and rejection cases. An unchanged graph may be correct. Reading source, verifying generated output, exercising a loader and running a consumer test establish different things. Compare generated contracts with their source, then exercise the relevant consumer or rejection through its actual entry point.

### Recheck the required source frontier at use

Identify the source revision the question requires and whether it requires a verified index or permits direct source and test evidence. When using a result to decide, recheck permission and compare its verified revision with the required one. An earlier successful query or unchanged index timestamp supplies no fresh observation. A result for a pinned earlier revision can still answer that historical question if its use remains permitted.

If current-source evidence is required and the source has changed, hold an index-dependent action until a matching generation is verified. If a direct fallback is permitted, report its revision, source read or test and limitations; leave the current-index claim unresolved. A refresh or new observation needs its own recorded result in the tool's existing output or the review reply.

Test a valid query followed by an edit before the result is used. Compare a question about the saved revision, a current-source question allowing direct reading and one requiring a matching index. Confirm that fallback claims only what it observed and that a refreshed generation does not rewrite the earlier query's record.

## Preserve deliberate local conventions

Read the instructions and a representative part of the implementation before changing mature code. Follow the established naming, structure, formatting and idioms for that component. Propose a harmful or obsolete convention separately. Unrelated modernisation expands both the change and the checks needed to establish it.
