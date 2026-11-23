+++
description = "Verify what actually committed and reconcile lost replies before advancing recovery."
uses = [{ scope = "local", resource_id = "verified-input" }]
see_also = [{ scope = "local", resource_id = "recovery/acceptance" }]

[guide]
+++

# Verify the exact commit before advancing recovery

<a id="check"></a>

Distinguish a successful transaction or staging step, an immutable commit and subsequent visibility or acceptance. Before committing, identify the original request or checkpoint, expected predecessor and exact staged changes. Protect that scope through commit with the system's isolation or fencing mechanism. A chosen namespace, table, directory or staging area may still include another writer's changes. Verify what the physical operation actually commits.

Read the immutable revision by identity. Check its parent, required content and excluded changes that might have slipped in. Inspect current head and working state separately when relevant. Later working edits do not alter earlier commit bytes. Do not expose the result or advance dependent checkpoints until required observations agree.

After a lost acknowledgement, look up the original request and candidate revision through the same recovery procedure. A matching commit permits only the recovery work still outstanding. A moved predecessor, wrong content or ambiguous history requires reconciliation. Repeat a commit only when observations establish that the original effect is absent and its guards and permissions still hold.

In an isolated rehearsal, interleave an independent write inside the physical staging unit and another outside it. Inspect the actual commit. Then lose its response and make a later working-state edit. Check that a matching committed result finishes recovery without another commit, and that a mismatch prevents acceptance.

## Before handing off

- [ ] Read the committed state at the intended position.
- [ ] Check that an older, partial or uncommitted write cannot move it.
- [ ] Reconcile the commit and any interrupted update before proceeding.
