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

## Worked example

A published branch has two consumers. Add a correction commit instead of rewriting shared commits unless the owners agree on a coordinated move. Inspect branch and remote before choosing the integration command.
