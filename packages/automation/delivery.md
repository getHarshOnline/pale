+++
description = "Use it when delivery depends on a source commit, to record the occurrence in that transaction and keep the source record, attempt, acknowledgement and effect distinct."

uses = [{ scope = "local", resource_id = "workflow" }]

[guide]
+++

# Deliver committed occurrences and recover history

Check delivery, producer order and recovery after a lost connection against records retained by the source and destination.

## Deliver from a committed occurrence

If delivery depends on a source commit, record the occurrence in that transaction or use a durable log proving the same committed position. Keep source record, occurrence, delivery attempt, durable acknowledgement and downstream effect distinct. A webhook or watcher may prompt a read; its transport acknowledgement proves only the stage its contract records.

Deduplicate using the complete occurrence identity scoped to its owner, including producer incarnation where the protocol uses one. Retain a digest to distinguish exact redelivery from changed bytes under the same identity. Exact redelivery keeps the earlier outcome; conflicting bytes must be isolated. Equal bytes under different valid identities are separate occurrences. A carrier's delivery ID identifies an attempt.

Where order matters, define the partition and sequence. Keep gaps visible until recovered from the authorised durable source. Advance the consumer checkpoint only when acceptance of the covered range is durable. Replaying an accepted occurrence must not repeat its downstream effect. Reconcile uncertain effects using their original identities.

Test rollback before source commit, interruption between commit and delivery, redelivery after lost acknowledgement, changed bytes under a reused occurrence identity, and recovery across a missing sequence. Read source record, durable destination state and checkpoint separately.

<a id="qualify-a-separately-authorized-delivery"></a>

## Check permission for a separate delivery

Distinguish source acceptance, source commit, delivery obligation, provider publication and recipient access. A source outage may leave an incoming command unverified. After a known commit, a lost reply leaves the result available for authorised read-back and cannot authorise a second source command. Where policy requires a reply, notification or projection, tie that obligation to the committed record and revision, a stable delivery identity and the agreed placement rules. Keep it unresolved when placement or transport is unavailable.

Before each outbound attempt, verify the exact audience, destination and required reply anchor, together with current permission to send and disclose each field. Resolve the anchor by its exact identity in the destination system. A lookalike room, thread or result is insufficient. Permission to decide, send and read back are separate; approval, refusal and transport receipts do not supply the other grants. An alternate audience or new thread needs its own permission under the applicable contract.

After a lost response, reconcile the exact attempted delivery under current read permission. Stop sending when send permission is revoked. If reads remain permitted, reconcile only the original attempt. With no readable receipt or incomplete coverage, keep publication uncertain and record revocation separately. Historical publication, current visibility and recipient acknowledgement are distinct facts.

Test a gap before source acceptance, outage after commit, ambiguous placement, substituted reply anchor, and lost acknowledgement followed by revocation with reads both allowed and denied. Inspect source state, delivery obligation, publication, access and downstream effects separately. A refused or unresolved delivery must preserve the source result without duplicating its effect or widening its audience.

<a id="qualify-ordering-across-producer-incarnations"></a>

## Check event order when a producer restarts or is replaced

Identify the source's ordering promise: contiguous sequence, monotonic order with permitted gaps, or opaque cursors. Identify the partition and authority establishing a producer incarnation and its successor. A counter decrease, restart, reconnect or new transport receipt cannot establish succession. Handle wraparound only as the source contract permits.

Within a known incarnation, distinguish exact redelivery, conflicting reuse, late arrival and an uncovered interval under the stated guarantee. Check each occurrence's digest. A higher sequence may be stored durably while a required predecessor remains unresolved. Keep the fully accepted contiguous position separate from the highest observed sequence. Advance an opaque cursor only with the source's authorised continuation and coverage evidence. Update the business view from the accepted position, not arrival order.

Before a successor produces new effects, verify its transition and starting position against source authority. Retain the predecessor's accepted position and outcome. Late old events cannot update the successor's view. If recovery supplies only a current snapshot, state which current facts it establishes and leave unavailable event history visible. An explicitly accepted new start may resume current work within its own limits without inventing missing occurrences.

Test reordering, a missing predecessor and recovery, exact and conflicting duplicates, an apparent reset without owner evidence, an accepted successor and late predecessor delivery. After restart, inspect history, checkpoint and current view separately. Compare permitted skips or opaque continuations where supported. The source owner defines ordering, succession, retention and permission to act.

<a id="prove-authorized-coverage-after-reconnect"></a>

## Check which history was recovered after reconnecting

Identify the source, producer incarnation, subscriber scope, filter revision, last accepted checkpoint and recovery interval. Record the earliest retained position, query limits, continuation rules and current read permission. Distinguish the latest observed event from the latest position whose required history is established. A completed subscription, empty batch or end marker may mean only that the server finished answering the request.

Establish coverage using evidence supported by the source contract, such as complete pages through a fixed high-water mark or a contiguous durable interval. For filtered feeds, use the guarantee for that exact authorised view: a gap in global numbering does not by itself identify a missing eligible record. Re-read a permitted overlap after reconnect and deduplicate by owner identity and digest. Events sharing a timestamp remain distinct. Test the source's tie-break or use a sufficiently inclusive read. If retention, query failure, response limits or changed permission leave coverage uncertain, retain the uncovered interval without exposing hidden records.

Test a completion marker with a required occurrence missing. Verify that the checkpoint stays at its last established position and no downstream command is inferred. Recover through an authorised owner read and redeliver the original occurrence across the overlap. If a mapping changed, keep an already accepted command's outcome tied to its original target and revision. An unaccepted event needs current owner checks. Inspect recovery coverage and downstream effect counts independently. A new snapshot establishes only its current-state scope; report unavailable history separately.

## Worked example

Drop the connection after a producer commits but before a consumer stores its checkpoint. Reconnect from the last known point, observe redelivery and check each permitted destination. Confirm that a successor producer cannot silently replace missing history.

## Before handing off

- [ ] Are source, occurrence, delivery and effect identities distinct and retained?
- [ ] Do retries, producer replacement and reconnect preserve the accepted order and known coverage?
- [ ] Were permission changes, missing history and uncertain destination outcomes checked?
