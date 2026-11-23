+++
description = "Review the inputs, effects, independent evaluation and adoption of a proposed instruction."

[task]
uses = [{ scope = "local", resource_id = "regression" }]
outputs = ["In the review: search inputs, observed writes and the staged candidate.", "Independent evaluation, adoption checks and unresolved conditions for the owner."]
+++

# Review a search for better instructions

Identify the instruction and current base, evidence the search may use and its permitted purposes, search method, budget and staging destination. Set aside evaluation cases the search cannot see. Name the effect limits and who may approve adoption. Keep the original draft, search-generated candidate, independent evaluation and active instruction separately identifiable.

## Check what the search can read and write

Inspect the actual entry point and its access. Require explicit evidence inputs and a staging area separate from the active target. Check missing or ineligible evidence. Account for all permitted output, including log content and retention. Inspect what a dry run writes; its name alone cannot establish that it leaves files untouched. Keep unrelated history, changes to active instructions or memory, and automatic adoption outside the search route.

### Review and dispatch the same inputs

Capture the task, target, current base, assignments of search and evaluation cases, and every instruction or auxiliary input the worker and backend can read. Review and permit use of those exact bytes. Resolve the target and dispatch from that same accepted snapshot. A reviewed flag or unchanged path cannot identify content read later. Keep omitted inputs unavailable to the worker, including automatically discovered project instructions.

Test replacement of the task between validation and dispatch, including replacement with a different reviewed revision. Substitute a link at each input path, and try an undeclared instruction file while its optional feature is disabled. Inspect what actually reaches the worker and backend. Require refusal before unapproved bytes are disclosed. Turning a feature off does not establish that surrounding inputs remain unread.

### Check saved evidence separately from worker completion

Identify every required evidence destination. Check the resolved directory and file where the system writes them. Test a redirected directory, a redirected log and an unavailable destination separately. Read back the saved records and their run and candidate identities. A worker result with missing required evidence leaves this check open. Reconcile partial writes before retrying and preserve unrelated outputs.

Read back the staged candidate and compare the active target with its baseline. Missing or rejected results must leave that target unchanged. Replay and validation can establish readiness for independent evaluation. Use held-out cases to measure transfer to other cases, regressions and the claimed gain over the current instruction. Keep uncertain effects and failed or inconclusive cases visible in the review.

## Check adoption and recovery for each target

Adoption is a separate action for the owner. Before writing, associate the evaluated candidate bytes with the target, current base, decision and permitted effects. Recheck the base at the point where the write is guarded. On disposable copies, try an intervening edit and a changed staged artifact. A stale base or changed candidate needs a new decision using the new evidence. Read back the accepted bytes through the owning system before calling them active.

For adoption across several targets, list the staged members, destination identities and recovery artifacts before writing. Check that the actual entry point consumes that manifest format and reports each target. Read back every requested target and its original bytes from the recovery record. Recovery identifiers must distinguish destinations even when files share a basename.

Exercise a missing later member after an earlier write, a recovery-name collision and a staged member the entry point can skip. Inspect the actual writes and recovery bytes after each result. For partial or uncertain outcomes, keep each target's result and reconcile it before retrying. An error can follow a completed write, and a success with no work can leave the entire staged set unprocessed.

Return the search, evaluation and adoption findings separately in the requested review. Running the search and adopting an instruction require their own execution steps and permissions.

## Before handing off

- [ ] Were the original and candidate compared under equivalent conditions?
- [ ] Were changed behaviour and effects inspected before preferring a draft?
- [ ] Does the adoption decision remain with its owner?
