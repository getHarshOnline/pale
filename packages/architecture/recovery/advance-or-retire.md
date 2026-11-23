# Check the commit and the next recovery path

Use these checks before accepting recovered state, advancing a recovery marker or retiring a copy. Keep tests and results in the owning recovery system and report remaining decisions in its review.

## Verify the exact commit before advancing recovery

Distinguish a successful transaction or staging step, an immutable commit and subsequent visibility or acceptance. Before committing, identify the original request or checkpoint, expected predecessor and exact staged changes. Protect that scope through commit with the system's isolation or fencing mechanism. A chosen namespace, table, directory or staging area may still include another writer's changes. Verify what the physical operation actually commits.

Read the immutable revision by identity. Check its parent, required content and excluded changes that might have slipped in. Inspect current head and working state separately when relevant. Later working edits do not alter earlier commit bytes. Do not expose the result or advance dependent checkpoints until required observations agree.

After a lost acknowledgement, look up the original request and candidate revision through the same recovery procedure. A matching commit permits only the recovery work still outstanding. A moved predecessor, wrong content or ambiguous history requires reconciliation. Repeat a commit only when observations establish that the original effect is absent and its guards and permissions still hold.

In an isolated rehearsal, interleave an independent write inside the physical staging unit and another outside it. Inspect the actual commit. Then lose its response and make a later working-state edit. Check that a matching committed result finishes recovery without another commit, and that a mismatch prevents acceptance.

## Check the retained authority frontier

Restoring old state can revive expired grants, consumed requests, deleted records or earlier policies. Keep the minimum permitted state outside the failure and restore domain it protects. Test whether backup, whole-machine restore or cloning can roll back that anchor together with the data. A co-restored copy cannot establish an independent minimum. Identify the exact permission state and checkpoint as well as its ordered position. Timestamps do not repair rollback.

Compare restored state with that independent minimum:

| Observation | Recovery disposition |
| --- | --- |
| Exact accepted state and preserved minimum agree | Continue the owning system's readiness checks. |
| State is ahead with the verified pending successor or permitted contiguous chain | Complete only the outstanding marker advance, preserving original identity and outcome record. |
| State is behind, equal coordinates identify different bytes, or a predecessor is missing | Refuse use and retain observations for authorised recovery. |
| A required anchor is absent or cannot be authenticated | Block dependent effects and follow the owner's recovery policy. Starting again needs separate permission and evidence. |

If the protocol advances an external marker after a database commit, interrupt on both sides of that step. Inspect committed state before resuming. Confirm that an uncertain response cannot cause another mutation, outcome record or repeated external effect. Test stale backup, equal coordinate with changed digest and clean restoration at the expected position. Report accepted and refused ranges, rehearsal results and the production decision still needed.

## Prove the replacement route before retiring a recovery copy

Identify the copy and every required use. Trace current consumers, recovery procedures and historical references separately through configuration, scripts, resolved links and retention rules. In the retirement review, list each dependency, the state it requires and its replacement route or unresolved need. Owners can repoint current instructions. Sealed records keep original references, with a permitted recovery route readers can follow from them.

Restore from the replacement into a clean isolated target while the candidate for retirement is unavailable. Compare paths, bytes, metadata, stored state and dependencies according to each use's promise. A clean checkout may restore history while omitting staged or working edits. A source archive may restore files while omitting a live database or journal. State what each component restores and exercise dependent readers or commands where behaviour matters. Missing coverage or an unresolved active dependency requires retaining the copy.

Check custody and independent-backup obligations separately from equivalent content. A second copy in the same failure domain may satisfy a read while failing backup requirements. Rehearse retirement and reverse restoration using a controlled copy. Check live routes, historical reads, paths, metadata and intervening edits. The custodian decides whether to retire the original and how that decision remains recoverable.

## Worked example

After restoration, inspect the committed revision and current owner's permitted uses. Rehearse recovery from the replacement. Retire the earlier copy only when required uses work from the new location and the owner agrees.
