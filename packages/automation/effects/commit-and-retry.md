# Commit results once and retry from durable state

Check queue expiry, final results, prerequisites and reporting against the application's saved records.

## Keep queue history separate from business history

Store the business command, committed result, progress and delivery attempts under a durable identity owned by the application. Reuse that identity across retries and resumed chunks. Queue job IDs identify executions. Their retained status may show acceptance or execution, but expiry or disappearance leaves earlier execution uncertain. A completed job alone cannot establish completion of the business obligation.

Test recovery after queue history expires. Reconcile the durable command and effect records before scheduling again. Verify competing workers cannot both claim the first attempt. The destination's duplicate protection limits repeated effects after dispatch; the application still owns concurrent claims and their recorded outcomes.

<a id="fence-terminal-results-and-cancellation"></a>

## Let completion and cancellation settle only once

Match each result submission to the owner, run, current generation and attempt, expected revision, sealed input set and output requirements. Validate the whole result and verify required artifacts at their retained identities before acceptance. Queue status, progress events and truncated responses supply observations. Only the owner's final commit closes the run. Establish caller acceptance and later effects separately.

Make final commit and effective cancellation compete through the same durable guard. Record a cancellation request separately from cancellation taking effect. If the final result wins, read it without reclassifying it. If cancellation or a successor generation wins, exclude late output from accepted history and prevent its downstream effects. Exact redelivery may return the stored outcome; different content under the same identity needs investigation.

Resolve uncertain completion from the durable run and retained output, including after queue eviction or a lost reply. An inconclusive read leaves it open. Test interrupted persistence, missing or truncated results, both orders of cancellation and completion, late output from a superseded attempt, exact replay and expired queue metadata. Inspect the single final decision, artifact identity and any separately accepted delivery or result acceptance.

<a id="recheck-queued-work-against-its-prerequisite-generation"></a>

## Recheck the exact prerequisite before queued work runs

Before dispatch, save the command identity with the dependent work's exact owner, partition, run, generation, attempt, prerequisite commit and input revision. A queue lookup or duplicate-request key using only a short run ID may retrieve another partition's job or an earlier generation.

Recheck the prerequisite when dequeuing and before committing protected output or an effect. Hold work if the prerequisite is missing, belongs to another partition or no longer covers the selected generation. A plausible completion marker at a fallback location cannot replace it. If re-preparation or a successor replaces the generation, settle the old job under its original identity and accept successor work separately. A retry may recover the old outcome; it cannot adopt new inputs or turn preparation into completion.

Test exact retry, the same short ID in two partitions, a re-prepared run whose old completion remains readable, and successor completion while the old worker is held. Change the generation after a worker starts but before it commits. Verify which prerequisite each worker read, how many generations reached the expensive step, and the resulting state and output receipts. Retain old results under their retention policy while excluding them from the successor's accepted evidence.

## Retry reporting from the committed result

For reporting that must survive a crash, commit the completed result and pending export together or derive the export obligation from the committed log. Retain the originating execution, attempt and observation identity. A retry sends the stored result under the same observation identity with a separate delivery attempt. It must neither regenerate the result nor invoke the business continuation.

Check what the destination's acknowledgement covers. Partial batch acknowledgement leaves the rest unresolved. After a lost acknowledgement, reconcile the same observation or use verified destination deduplication before retrying. Test an outage after source commit, partial acceptance and a lost response. Inspect the original result, export status and effect count separately.

## Share one allowance across retries, shadows and fallback

For a model job, distinguish availability failure from an answer that is invalid, refused or uncertain. Define which failures may cause another attempt and which require repair, review or stopping. In an example policy, transport failure, HTTP 429 or 5xx permits an outage attempt; bad credentials, bad requests, refusal, malformed output and ambiguous but valid answers do not. These categories are an example policy to configure, not permission to override the owning workflow's response.

Under a two-attempt ceiling, the second call can be a same-provider retry, a paired shadow or an alternate provider used for availability. They share that ceiling: a paired job has already spent its alternate attempt and cannot add a third call for fallback. A failed shadow does not displace a valid assigned answer. Disable or include automatic client and workflow-node retries so they cannot multiply the declared allowance. Count every attempted call, including failures, and stop before the next one exceeds the remaining task budget. Reserve the maximum calls for the remaining stages and branches before dispatch, subtracting only completed work whose actual attempts are known. The model-cost guide supplies the worked reservation example.

For an illustrative outage delay, let $a$ be the failed attempt number, $b=\min(60,2\cdot2^{a-1})$ seconds, $r$ the server's retry delay and $u\in[0,1)$ a reproducible jitter value for this call. Wait:

```math
\left\lceil\max(b,r)+bu\right\rceil \text{ seconds}.
```

With a first failure, a five-second server delay and $u=0.25$, the delay is six seconds. A policy that allows at most sixty seconds must hold if the computed delay exceeds that limit, rather than shorten the server's instruction. Resolve an HTTP-date retry header against the time it was observed. With only one additional attempt, the exponential rule supplies only its first step. Carry a rate-limit cooldown into later jobs where required; quota or spending exhaustion stops the work instead of starting another retry.

An outage response does not establish that a remote effect failed. Apply the durable-result and destination-reconciliation rules above before resubmitting any uncertain write. Record which permitted attempt supplied the answer, separately from the configured primary, shadow and fallback roles.

## Worked example

Cancellation races with a result and reporting times out. Read the final state and result from the owner. Retry that observation without rerunning the business work. Before applying a queued dependent item, check the prerequisite revision it actually requires.
