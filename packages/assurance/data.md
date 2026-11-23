+++
description = "Assess one personal-data operation, its purposes, data paths, controls and remaining copies."

[guide]
+++

# Assess one operation involving personal data

Use this method to assess one proposed handling of personal data against the authority and facts supplied. Assess an operation, such as collection, derivation, access, use, disclosure, transfer, retention, correction, restriction or deletion, rather than a product, database, policy or organisation in the abstract. Return findings for review. They are not legal advice, professional attestation, consent, a privacy notice, system configuration or permission to perform the operation.

## Define the operation before judging it

Describe the smallest operation that can independently be allowed or stopped. Identify:

- the action and the decision it serves;
- people whose data may be affected, including those represented indirectly or inferred from another person;
- data fields, derived attributes, sensitivity, detail and sources;
- the source, collection context and expectations created there;
- every actor and system able to determine, access, transform, receive or retain the data;
- the stated purpose and plausible secondary or onward purposes;
- trigger, frequency, duration and effective period;
- destinations, transfers, disclosures and public surfaces;
- the authority, policy, contract or other asserted basis; and
- missing, disputed, stale or privately held facts.

Split proposals when parts differ in purpose, affected population, authority, destination, retention or the decision to stop them. “Analytics,” “personalization,” “research” and “service improvement” are labels, not defined operations. A page view, cross-site identifier, contact-form submission and reuse of that submission for outreach need separate assessment even if one provider handles all four.

## Resolve roles from conduct

Establish who decides the purpose and essential means, who acts on documented instructions, who receives data for an independent purpose and who only transports or stores it. Use the applicable Pack's role definitions and evidence of the operation. A contract heading, vendor label, site owner, account owner or repository configuration alone cannot establish the role.

Preserve joint, delegated and changing roles rather than assigning one label across the whole lifecycle. If instructions, onward-use rights or decision control are unavailable, leave the role unresolved. Package reasoning can identify the missing condition, but cannot constitute an organisation or bind another actor.

## Trace the data lineage

Follow the operation from origin to final handling of the data. For each step, identify:

1. the input and how it was obtained or inferred;
2. transformations, combinations, scoring or enrichment;
3. who can access input and output, and for which purpose;
4. what leaves the current system and under whose authority;
5. durable, cached, logged, backed-up, indexed and model-derived copies created; and
6. the event ending use, access, retention or disclosure for each copy.

Pseudonymization, hashing, aggregation, encryption, removal from a primary table and provider marketing terms do not establish that personal-data obligations have ended. Check what can still be linked, singled out, restored, inferred or acted on in context. Preserve uncertainty where that evidence is missing.

## Scope reversible pseudonyms and check reconstruction at delivery

Define where a token may remain linkable. Limit its mapping to the minimum necessary scope, such as owner or authenticated caller, conversation or case, purpose and destination. If mappings use keys or epochs, state how rotation affects lookup and replay. Verify that equal values correlate only across permitted scopes. Protect the reconstruction map and access credentials separately from tokenized records. Identify when the map may be read, when it expires or is revoked and where deletion must propagate.

Reconstruction is a new disclosure decision. At the actual delivery adapter, resolve recipient and destination from trusted request context. Before restoring a value, verify the delivery purpose, recipient, token scope and current mapping permission together. Withhold the affected fragment until a streamed token is complete. For altered, unknown, ambiguous or out-of-scope tokens, withhold reconstruction and follow the approved refusal or review path. Guessing can substitute another person's data. Check retries, queued deliveries, callbacks and errors against current permission.

Test a permitted restoration and refused reconstruction for a different caller, case, purpose or destination; an expired or revoked map; and partial or altered tokens. Inspect actual delivery output and evidence emitted on both paths. Keep original values, lookup secrets and reconstructable maps out of diagnostics. Check remaining linkability through joins and combinations. Treat reuse in telemetry, evaluation and training as separate operations needing their own purposes and evidence.

## Test purpose and authority together

For each purpose, identify the exact basis proposed by the applicable Pack and the facts needed to rely on it. Check the implemented purpose as well as the one stated in writing or policy. Permission to collect does not automatically permit later use, disclosure, combination, training, targeting, publication or retention.

Ask:

- Is the purpose specific enough to constrain implementation and review?
- Do source context, notice, relationship and reasonable expectations support it?
- Is the operation necessary for that purpose, or merely convenient?
- Could a less intrusive design achieve the required outcome with acceptable loss?
- Is a new purpose compatible with the collection context, or does it need a separately supported basis?
- Do contractual restrictions, duties, rights, objections, age or capacity conditions, territorial limits or special-category rules change the result?
- Does the claimed basis require consent? If so, stop that branch until consent review returns a usable state.

Do not choose a basis for convenience or use consent to repair a misleading, excessive, prohibited or otherwise unsupported operation, including one outside the actor's role. A policy statement cannot create authority absent from the governing source or data subject.

## Minimize across the whole operation

Minimization means more than deleting fields at the end. Check independently:

- people included;
- fields and derived attributes;
- precision and identifiability;
- collection timing and frequency;
- observation duration;
- who has access and with what privileges;
- transformations and joins;
- destinations and onward recipients;
- public visibility;
- retention and remaining copies; and
- use after the original decision is complete.

For each retained element, explain which part of the purpose requires it and what would happen if it were omitted or made less precise. “Useful later,” custom, provider defaults and cheap storage do not establish necessity. Prefer designs that avoid collection, compute locally, aggregate before disclosure, shorten observation, separate identities, limit access or discard intermediate data when they preserve the required outcome.

## Evaluate the lifecycle and controls

Trace initiation, correction, restriction, applicable withdrawal or objection, completion of purpose, retention review, deletion and expiry of remaining copies. Identify the event changing each state and who must demonstrate that transition.

Check controls against the actual risks and data path: validation at system boundaries, least-privilege access, separation of environments and purposes, protection of secrets and transport, logging without secondary exposure, recipient restrictions, correction propagation, incident handling, and inspection or reconciliation after requested changes. A written control without a responsible owner, implementation evidence or defined failure behaviour remains an assumption.

Specify deletion scope. Distinguish primary records, replicas, caches, indexes, logs, exports, backups, derived profiles, trained artifacts and records retained under a separately established obligation or dispute hold. If immediate erasure is impossible, state restrictions on active use, remaining locations, expiry or review triggers and the authority for continued retention. Evidence about one copy cannot establish full deletion.

## Give a result for this operation

Return one state for the exact operation and scope assessed:

- **supportable**: every condition and control critical to the decision is established for this scope and period;
- **supportable with conditions**: specified controls or narrower scope are required before operation, with owners and verification evidence identified;
- **unsupported**: evidence shows the operation exceeds its authority, purpose, necessity, role or lifecycle limits;
- **unresolved**: critical facts, sources, roles, consent states or control evidence are missing or conflicting; or
- **out of scope**: the request is not an assessment of a personal-data operation or requires another owner's authority.

“Low risk,” “common practice” and “probably compliant” are not assessment results. Conditions must be testable. They cannot postpone the core authority question until after collection or disclosure.

## Return the assessment

Include in the assessment result:

- operation identity and versioned scope;
- affected population, data inventory, sources and sensitivity claims;
- actors and roles, including unresolved assignments;
- the basis for each purpose and applicability references;
- data paths through transformations, recipients, onward uses and remaining copies;
- minimization decisions and rejected alternatives;
- lifecycle events, controls, evidence and responsible owners;
- a separate consent-state reference where relevant;
- rights, objections, restrictions, holds and conflicting duties;
- result, conditions, uncertainty and events that permit reassessment; and
- exact actions that remain unauthorised.

Keep changeable facts about jurisdiction, controllers, processors, policies, effective periods, retention, providers and organisations in a Pack or owning system. The assessment may identify exact references; do not copy those values into timeless Package guidance.

## Stop conditions

Return unresolved or unsupported when:

- the operation cannot be separated into stable purposes and destinations;
- material questions about jurisdiction, period, role or authority remain uncertain;
- data sources or affected population cannot be established;
- required consent is absent, invalid, disputed or outside scope;
- recipient instructions, onward use, transfer conditions or remaining copies are unknown;
- necessity rests on an untested claim while a materially less intrusive design remains plausible;
- the action conflicts with a restriction, objection, withdrawal, hold or another owner's authority;
- controls cannot be verified before collection, disclosure or publication; or
- the request requires professional approval, Pack acceptance, configuration, deployment, publication, deletion, disclosure, notification or another action in the owning system.

Stopping does not authorise a reduced version of the operation.

## Before handing off

- [ ] Are the people, data, purposes, roles and actual flow established for this operation?
- [ ] Do authority, minimization and lifecycle controls cover each use and recipient?
- [ ] Are unresolved copies, control evidence and the owner who can act on them visible?
