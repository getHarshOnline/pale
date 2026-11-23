# Check fork lineage and what a recipient can reach

## Check fork lineage and transport

Identify canonical upstream, local mirror, maintained branch and consumer pin separately. For each, record ref name, object ID, observation source and time in the review. Cached tracking refs describe earlier local observations. If freshness matters, fetch the precise source ref with permission and compare its returned object with the last accepted revision and mirror. Verify ancestry before advancing a fast-forward-only mirror. A moved tag or divergent mirror requires a decision. Equal tips can still serve different roles.

For a fetch-only upstream, inspect effective fetch and all push destinations after includes and URL rewrites. Check branch tracking and default push destination separately. A default does not block an explicit push elsewhere. Confirm the upstream push guard refuses and no alternate push URL bypasses it. Test using inert configuration checks or disposable remotes, never a real attempted upstream push.

Separate observing source, advancing the mirror, integration review, updating the maintained branch and changing a consumer pin. Review old and proposed revisions, changes, rights, maintainer, consumer tests and rollback revision. Distinguish upstream identity from claims that a local mirror or build is equivalent. State whether the consumer uses stock source, a verified equivalent mirror or reviewed modifications. Holding, inspecting or showing material grants no permission to consume or publish it.

## Inspect repository disclosure by revision and destination

Identify the proposed destination, revision and refs whose reachable history may transfer. Inspect working files, ignore rules, index, selected tree and older reachable objects separately. Ignore rules can match tracked files. A clean tip can still expose an earlier version through a parent or another selected ref. Local tracking refs establish only their last observation; claims about current remote exposure require a permitted observation of that remote.

Start with paths, modes, object IDs and reachability. Read content only within the permitted inspection and report only what the decision needs. Git stores a symlink's text and mode; opening its target is unnecessary to establish tree membership. Separately check whether the exporter follows links or includes submodules, extra refs or working files. Inspecting an artifact does not inspect history transferred with it.

Test an ignored-but-tracked path, a clean candidate with an affected ancestor, a ref excluded from export and a link to an out-of-scope target. Inspect exported refs and objects in a disposable destination. Report scope and unresolved copies in the disclosure review. Those observations grant no permission to publish, rewrite history, untrack material or delete saved evidence.

### Compare sent objects with the recipient’s reachable history

Identify transport, advertised refs, prerequisite commits and the recipient baseline available for inspection. Inspect a tree archive through emitted members. For Git transfer, distinguish newly sent objects from everything reachable using the transfer plus the recipient's existing object store. Omitting an ancestor from an incremental transfer neither revokes access to it nor removes descendants' dependency on it.

Rehearse the actual transfer into an empty destination and a disposable one containing the permitted prerequisite. Inspect received refs, prerequisite handling, reachable objects and affected paths at their historical trees. Include an object absent from the candidate tree and deliberately withheld from transfer, an incremental transfer sending no new copy of that object and an unselected ref or tag. Compare observations with transport results and package contents. If the actual recipient baseline is unknown, leave total exposure unresolved.

Report archive contents, newly sent objects, received refs, resulting reachable history and residual destination copies separately. The owner decides disclosure, retention and any later withdrawal or history change.

## Worked example

Before sending a subset to another remote, check ancestry and the commits or objects the recipient will be able to reach. A clean tip alone does not reveal the parent history being delivered.
