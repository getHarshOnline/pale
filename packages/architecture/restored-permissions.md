+++
description = "Prevent restored data from reviving permissions or state older than the independent recovery minimum."
uses = [{ scope = "local", resource_id = "verified-input" }]
see_also = [{ scope = "local", resource_id = "recovery/acceptance" }]

[guide]
+++

# Check who may use the recovered state

<a id="check"></a>

Restoring old state can revive expired grants, consumed requests, deleted records or earlier policies. Keep the minimum permitted state outside the failure and restore domain it protects. Test whether backup, whole-machine restore or cloning can roll back that anchor together with the data. A co-restored copy cannot establish an independent minimum. Identify the exact permission state and checkpoint as well as its ordered position. Timestamps do not repair rollback.

Compare restored state with that independent minimum:

| Observation | Recovery disposition |
| --- | --- |
| Exact accepted state and preserved minimum agree | Continue the owning system's readiness checks. |
| State is ahead with the verified pending successor or permitted contiguous chain | Complete only the outstanding marker advance, preserving original identity and outcome record. |
| State is behind, equal coordinates identify different bytes, or a predecessor is missing | Refuse use and retain observations for authorised recovery. |
| A required anchor is absent or cannot be authenticated | Block dependent effects and follow the owner's recovery policy. Starting again needs separate permission and evidence. |

If the protocol advances an external marker after a database commit, interrupt on both sides of that step. Inspect committed state before resuming. Confirm that an uncertain response cannot cause another mutation, outcome record or repeated external effect. Test stale backup, equal coordinate with changed digest and clean restoration at the expected position. Report accepted and refused ranges, rehearsal results and the production decision still needed.

## Before handing off

- [ ] Read the owner and permission state that recovery would keep.
- [ ] Test that older or superseded permissions cannot return.
- [ ] Check who can approve the recovered state before using it.
