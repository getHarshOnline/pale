# Identify the source a check actually used

Identify owner, repository, task, target, observation time and permitted changes. State whether the check concerns a committed revision, working checkout, artifact or restored candidate. For a clean revision, record commit and tree in the existing build, test or recovery results. Keep dirty working state distinct.

For a working checkout, use the repository's file inventory and state the inclusion rule. Associate each path with file kind, executable mode, bytes or link target, and relevant tracked deletions. Sorted content hashes alone lose the path-to-content relationship. If staging matters, record the index separately, including conflicts and unresolved entries. Identify relevant untracked inputs and omissions. Do not sweep ignored output, caches or secrets into a capture. If excluded input affects the command, use a permitted reference or narrow the claim.

Identify dependencies and locks the command actually resolves. Preserve symlink text, then verify its resolved target within the intended layout. Identify relevant sibling checkouts, generated sources, submodules and outside inputs independently. Stop at missing or unpermitted targets. Hashing a link or root checkout does not identify those inputs.

Record command, working directory, target, toolchain and relevant environment controls. Compare in disposable copies when commands write, preserving baseline and candidate inputs independently. Report first failure, exit status, output and appropriate inspection of resulting state. If a recipe tolerates a failed step, report it separately. An artifact digest, restored index, successful build and behavioural test establish different things. Captures and permission to use them remain with their owner.

## Verify the provider a consumer actually resolves

Distinguish the consumer declaration and lock from the provider resolved for its features and target. Inspect resolved manifest or artifact, location and source identity. For local paths, identify the checkout or permitted files, including relevant working edits. An unchanged path, package version and lock can still resolve to different bytes.

When comparing providers, hold consumer, configuration and command fixed. Resolve each in a separate disposable layout and verify identity before testing. Use the same consumer probe and a case distinguishing the claimed behaviour. Report connection setup, tests reached and observed behaviour separately. Preserve historical and successor source identities with their own owner decisions and publication states.

### Separate declared, selected and exercised dependency edges

State the relationship and claim being examined. Source inheritance, manifest declarations, build resolution and runtime behaviour are different observations. Preserve commented references as context. Dependencies may be optional, development-only or target-specific; a lock can include branches the selected command never uses.

Inspect the resolved graph for the target, features and dependency kinds in question. Check effective provider features, including defaults enabled through other consumers. Distinguish compilation from resolution. Identify suite, test count and provider behaviour reached by assertions. A zero-test success may establish compilation at that revision; it proves no assertion. Exercise the consumer's real entry point or leave runtime behaviour unverified.

Use an unselected declared dependency, a feature or target selecting it and a build with no relevant tests as controls. If integration requires a live route, observe it and a missing or incompatible provider. Report declaration, resolution, compilation and consumer behaviour separately for each relevant connection.

### Bind local dependency inputs beyond the lock

Identify the command's actual workspace, manifest and governing lock. Distinguish entries tied to immutable sources or artifact digests from those resolved locally. Record where copied or regenerated locks came from. A locked run can confirm a graph without pinning local source bytes.

For each relevant local dependency, inspect the resolver's actual manifest and physical source path, including transitive dependencies and workspace, feature and target choices. Identify consumed files, permitted working edits, generated inputs and required absences. Separate declaration-only listings, resolved graphs, compilation and executed tests. With Cargo, inspect the resolved graph using the applicable lock. Package name and version, or metadata produced with resolution disabled, cannot establish which source the consumer used.

Test a different sibling tree at the same path under the same manifest and lock. Require the source check to detect it even if outputs or tests agree. Include missing or shadowed local inputs where possible. If release promises standalone installation, test distributed source with sibling checkouts absent. The owner defines source pins and release requirements.

## Reconstruct layered worktree recovery

Specify required recovery before choosing a capture. List history and refs, index state when staging matters, and working paths, including untracked records and deletions. A commit or bundle may preserve history while omitting later working bytes. A patch may preserve edits without their base, untracked files or staged version. State what each component reconstructs and excludes.

For base-plus-delta recovery, identify the exact base and apply verified components in recorded order. Compare resulting paths, modes and links with the intended combined capture. Check deletions so base files do not survive when they should be absent. Keep original archive bytes and their record separate from the restore.

Restore into a fresh isolated root with no fallback to the original checkout. Inspect history and required refs, then index and working files separately. Staged bytes may intentionally differ from working bytes. Confirm the restored object store contains objects referenced by the index; a matching index digest alone cannot prove that. Preserve a captured temporary lock as evidence and determine whether it prevents replay before changing the disposable copy. Recheck index and worktree afterward. Keep untracked originals with their owner; current guidance may cite them and state the rule without copying their contents.

Run the target command only once restored inputs match. Inspect sibling dependencies through their own selected sources and pins. Report recovery completeness separately from command behaviour, naming excluded runtime state and unresolved inputs.

### Restore state carried by a stash or another transient ref

Inventory required refs and exact objects, including in-scope stashes and reflog-only entries. Check which objects exported refs make reachable. Branch history can contain a stash's base without the stash itself. Recover from the saved Git directory or export the selected object through an explicit ref in a disposable copy. Verify imported identity and connectivity without the original checkout, remotes or object store.

For a stash, identify base, index tree, tracked working tree and optional untracked tree. Restore at that base in a clean isolated worktree using the index-aware Git route. Compare staged and unstaged bytes, modes, deletions and untracked membership separately. Confirm empty layers stay empty. Resolving a ref or successfully applying a stash does not prove all layers match.

Test a branch-only capture containing the base but omitting the stash, then restore and apply the captured stash ref. Include staged bytes differing from working bytes and an untracked file when both are promised. Preserve original history and records. Recovery restores the stash's original context; it grants no permission to apply it to a current live branch.

### Verify pinned tree and ref closure separately

State whether recovery promises a source tree, history reachable from selected refs or a wider ref inventory. Nested repositories each have their own object store and scope. Inspect shallow boundaries, promisor settings and alternates before claiming complete history.

Compare the pinned tree with its required blobs, paths and modes. Separately walk every commit, tree, tag and blob required by the promised refs. Verify objects are readable from the saved store with remotes, lazy retrieval and outside object stores unavailable. A ref hash, checkout or integrity check that tolerates promised objects does not prove all history is present.

Test a saved tree with missing objects elsewhere in claimed history, and a complete independent store as a control. Report verified tree and refs, missing objects and intentional history limits. Keep exact-tree recovery usable for that scope while marking incomplete history recovery unproved.

## Verify a relocated or restored checkout

Separate original source identity from current location. Before replay, inspect actual worktree root and metadata paths. For Git, check both worktree-specific and shared common directories. A copied linked worktree may still use the original index or object store. Confirm those locations and sibling inputs belong to the intended isolated restore.

Compare required refs, index and working files before commands can refresh the index or generate output. A similarly named directory that builds does not resolve a missing checkout or dependency. Record the verified new location separately from the original capture and limit conclusions to commands and sources actually checked.
