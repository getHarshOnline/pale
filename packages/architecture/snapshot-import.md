# Check snapshot coverage, removals and activation

## Establish what the source actually covered

Identify the source owner, namespace, permitted visibility, record identities, generation, predecessor and manifest. Compare declared coverage with the source query and captured results. Fully transferring a partial view does not make it complete. Missing pages, changed permissions or filters and source outages can hide records that still exist. Ordered changes and complete record sets need different handling.

## Verify freshness at the consumer

State required coverage and freshness for each scope, partition and consumer task. Distinguish the latest business event, the source position acquisition actually covered, completion time of each stage and the input revision currently readable by the consumer. An old event may be the latest record in a fully covered quiet period. A recent event can appear while earlier coverage is missing. A successful stage or callback proves only its own work.

Trace scope and source position through extraction, copying, transformation and serving. Associate checkpoints with specific upstream and output versions. Inspect the revision consumers can see and its validity period. The highest partition checkpoint does not establish freshness for lagging partitions. A no-change result supports freshness only if acquisition established its window and scope. Keep missing, incomplete, stale and unknown observations distinct from a verified empty result.

Test fresh acquisition from a quiet source, recent rows with incomplete coverage, successful transformation of an old input, a missing partition and a consumer still serving the predecessor. Inspect the upstream checkpoint and visible consumer state independently. In the review, identify affected scopes and the owner's required hold or permitted degraded route. A successful rerun does not change what an earlier run established.

## Require authority for removal

Preserve omitted records unless the governing contract permits their removal. Distinguish an explicit owner instruction from deletion inferred from absence. The latter requires an active complete-set policy for that source, scope and generation. Check comparable coverage, first and current absence, grace conditions, reconciliation and superseding record or policy revisions. Recheck current policy when activating the import so a stale signer or revoked policy cannot commit it.

The authorised producer or reconciler creates removal records identifying their targets and supporting observations. Include them in the same verified candidate manifest as upserts. The receiver checks and applies that set; it cannot invent extra removals during activation. Cache eviction, removing a derived view and erasing the original record or copies each need their own permission and proof.

## Verify the prior and next visible generations

Stage the candidate without exposing it. Check the expected predecessor and permission together when activating it. Inspect the visible generation, retained records and removals separately from the import acknowledgement. Test exact replay and confirm rejection preserves the predecessor. After interruption or restore, recheck current permissions and verify readers see neither a partial candidate nor a revived removed record.

Test omission with retention, explicit removal and policy-derived removal separately. Challenge inferred removal with incomplete capture, narrowed visibility, stale permission, insufficient grace, reappearance, altered evidence and competing candidates for one predecessor. Keep these cases in the import tests and report observed state and unresolved removals in the review. The owning systems define comparison, trust and transaction rules and supply their own records of received and applied data.
