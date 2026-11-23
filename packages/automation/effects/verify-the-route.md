# Check callbacks and destination identity

Check who sent a callback, what an acknowledgement establishes and whether refreshed authentication preserves the request.

<a id="admit-callbacks-at-the-effect-boundary"></a>

## Verify callbacks before applying changes

Identify the receiving route, contract version, sender credential scope, exact signed material, delivery identity, and any timestamp or destination checked by the verifier. Preserve the required body representation and headers before parsing or transforming them. Verify on the actual receiving path before processing. Missing verification material or a failed check must stop protected work. An alternate sender, direct call or fallback needs its own accepted authentication and scope checks on that path. Retain only safe identity and outcome evidence.

An authenticated notification establishes delivery from the verified source under the stated contract. Check its subject, tenant, current state and version independently. If it can trigger a business transition, accept a separately authenticated command under the same authority, version and state guards as the interactive route. Evaluate claims and instructions inside the payload under their own source's authority.

Deduplicate transport replay using the accepted delivery identity and retain its result within the defined limits. If acknowledgement stops upstream retries, acknowledge only the acceptance stage promised by the contract. When durable acceptance is promised, verify that the notification or equivalent recovery record survives a crash before later processing. On retry, read the delivery and any business command separately.

Test a genuine fixture, altered signed material, missing or invalid credentials, an unauthorised alternate route, a signed delivery with a wrong or stale subject, and replay before and after acknowledgement. Include restart after durable acceptance. Inspect verification, retained receipt, command acceptance and applied change separately. The owner defines signatures, replay window, data permissions and commands.

<a id="verify-the-acknowledged-identity-at-the-destination"></a>

## Check which write the destination acknowledged

State what each positive response certifies. A destination may acknowledge exact redelivery, or a write that lost to another write under its ordering rules, without committing a new record. Reading a shared address may return a different current write. Where the protocol exposes them, match the original immutable identity and expected revision or payload digest against an authorised destination read. Record that read's scope and time. To establish whether an effect ever committed, use destination history or a durable receipt.

Distinguish an exact match, a different record at the same address, and an inconclusive exact lookup. Absence is conclusive only within established visibility, retention and completeness. After a lost reply, retain and reconcile the original identity before retrying. A permitted retry reuses the original request identity.

Test a committed write, exact redelivery, an acknowledged write that loses to another identity and incomplete read-back. Check the durable outcome separately from the response and current record at the address.

## Preserve the request while refreshing transport authentication

Keep the business request and any signed or content-addressed payload separate from the transport attempt and its authentication. Retain submitted bytes when the provider's replay rules require them. Fresh authentication must cover those same bytes, destination and effect under current permission. Refreshing a session, nonce or envelope signature must not create a new publication, replace the effect identity or widen the target.

Interrupt delivery after the request may have landed, including after response headers but before a complete reply. Where exact replay is supported, compare retained payload and effect identity across attempts and verify authentication for each attempt. Inspect destination receipt and effect count separately. Include changed payload under the same identity and a fresh identity presented as a retry. If exact replay is unsupported or permission changed, preserve the unresolved outcome and reconcile with the owner before any new effect. A loopback transport test can establish byte preservation while leaving destination commit and permission unobserved.

## Worked example

Receive the same callback twice and compare verification and business identifiers. Resolve an acknowledgement to its actual destination and read the result. Refresh expired authentication without changing the request bytes or effect, then check that permission still covers the attempt.
