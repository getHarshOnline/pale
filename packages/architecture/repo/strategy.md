+++
description = "Use it before recommending Git commands, to identify the owner, worktree, branch, upstreams, protections and consumers and to keep the decisions separate."
uses = [{ scope = "local", resource_id = "git" }]

[guide]
exposure = "internal"
+++

# Choose a Git strategy from the actual repository state

## Resolve authority and state first

Identify the owner, worktree, branch, upstreams, branch protections, review requirements, release path, shared history and downstream consumers. Inspect status and the commit graph before recommending commands. A branch label does not prove its role; a clean worktree does not prove the base is current.

Keep five decisions separate:

- where new work begins;
- what belongs in one reviewable change;
- whether local commits may be rewritten;
- how reviewed work enters integration or release branches;
- which revision a consumer should pin.

Different owners and permissions may govern each.

## Choose the smallest fitting strategy

Prefer short-lived branches when frequent integration and automated checks support them. Long-lived or release branches need an actual support, batching, regulatory or deployment requirement. Make commits coherent enough to review and revert. Follow the repository's message convention; introduce no format solely because it is fashionable.

Rebase changes commit identities. It can clarify private, unpublished work when policy permits, but rewriting history others consume requires coordination. Merge preserves relationships between histories. Squash creates a new integration commit that intentionally compresses history. Choose by policy, consumer state, review obligations and rollback needs.

Before integration or publication, reread the diff, run required checks and verify destination and remote. Force updates, protected-branch changes, tags, releases, publication and consumer-pin changes each need permission for that action. The source-control tool performs it and supplies the result; this guide helps choose it.

Stop the affected action if ownership is unclear, status or graph cannot be inspected, unrelated work is mixed in, consumers of rewritten commits are unknown, checks are missing or permission does not cover the action. In the review reply, report observed state, chosen strategy, rejected alternatives, checks, residual risks and the fact or approval needed to continue.

## Reassess an older change before moving it forward

Recover the problem the branch was meant to solve, later corrections and unresolved reviews. Read changes to the base since divergence, including replacements elsewhere, changed interfaces, schemas and ownership. Decide whether the solution still belongs in the current design, needs reworking or has been superseded. Resolving textual conflicts alone cannot answer that question. An obsolete proposal gets a recommendation with reasons; closing it still follows its owner's permission.

Preserve unrelated edits and use an isolated worktree when adapting the branch would disturb them. Save the original commit reference in the existing review, compare the before-and-after series with `git range-diff` when useful, and exercise the retained behaviour. Keep original authorship distinct from the takeover's fixes. If rewriting a published branch is authorised, verify the remote head has not changed since the agreed starting point and use an explicit expected revision with `--force-with-lease`; a collaborator's new commit requires reconciliation.

## Prepare the pull request for its readers

Check for an existing request for this branch before creating another. State the problem, resulting behaviour, relevant checks and remaining limits in the repository's form. A concrete before-and-after example helps more than a command diary. Preserve author credit and remove private paths, secrets and unrelated context from public text and attachments. Draft status, publication and review comments follow the task's permission. Read back a submitted comment before resolving the discussion it addresses.

## Write a review comment that answers the finding

Read the finding and the surrounding discussion before replying. Lead with what changed or why the finding does not apply, then give the relevant code, check or observed behaviour. Distinguish an observation from your judgment. A reply such as “The retry now keeps the original request ID; the lost-response case passes” tells the reviewer what was resolved. Generic thanks or a list of unrelated edits does not.

Follow the repository's convention for agent attribution. Where it requires a model or harness name, use the identity actually reported by the host; do not guess a model, human author or email address. Keep that disclosure separate from the answer and preserve the original contributors' credit. A comment on someone's behalf is not a quotation from them.

Before an authorised post, preview its Markdown, links, quotations and attachments for readability and private information. Use the correct thread and avoid reposting an answer already present. Read back the resulting comment and its location; resolve the discussion only when the reply addresses it and resolution is within the task. Preparing a response does not itself authorise posting it.

## Follow checks and reviews through to the agreed result

Use one watcher for the pull request, with a named owner responsible for resuming work. Prefer the code host's event or wait facility; otherwise poll within its rate limits, backing off on transient failures. Watch the current base and head, required checks, open findings and human review state. Fetch every needed page of results: a partial comment window or a failed protection lookup leaves readiness unknown. Review requirements come from this repository, not a fixed list of bot names.

React to a failed check, new finding, changed base or completed queue entry. Distinguish a code defect from a service failure before retrying. Older unresolved findings remain relevant until addressed or shown to no longer apply; the latest push's time does not erase them. Keep edits to a review summary visible when they change its conclusion. Report meaningful changes instead of repeated waiting messages. A wait timeout prompts inspection of the same live run, not a duplicate watcher or a claim that it stopped.

Immediately before an authorised merge, inspect the current head and base again. Required checks must cover that candidate, required reviews must still apply, material findings must be resolved and mergeability must be known. A green status can arrive before all expected checks are registered; use the repository's required set and actual run state. A changed head or base needs the affected checks and review renewed. Observe a merge queue through completion or failure, then verify the resulting target revision. Queue acceptance, a watcher exit and a successful merge command alone do not prove integration.

For a task that must stop before merge, check auto-merge and queue membership at the start and before each branch update. If the task permits changing those settings, prevent an automatic merge and verify the result. Otherwise report the conflict before an update that could trigger it. At handoff, inspect the live state again and distinguish ready for the owner to merge, waiting on a required approval and blocked by a failed check. Report if someone else has already merged it. A preparation task must not silently become integration.

Stop or transfer the watcher when its assigned work ends, preserving only the current finding and resume information in the existing task or pull request. This workflow adds no separate progress ledger.

## Worked example

A published branch has two consumers. Add a correction commit instead of rewriting shared commits unless the owners agree on a coordinated move. Inspect branch and remote before choosing the integration command.
