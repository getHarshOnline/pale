# Check consent evidence and withdrawal

Use this method after another assessment establishes that consent matters to a specific operation. Check whether the evidence supports usable consent for that exact use and what withdrawal changes. This method does not select consent as the basis for authority, write notices, collect consent, manage a consent platform, erase data or attest legal validity.

## Bind the permission coordinates

Identify exactly what a consent record covers:

- the person consenting and any representative capacity;
- the actor requesting and relying on permission;
- the operation, purpose and decision enabled;
- the data or content covered, at useful field or category precision;
- collection method, transformations, recipients, destinations, public surfaces and material onward uses;
- the notice, explanation, interface and choices presented, with their versions;
- the affirmative act and capture channel;
- capture time, effective period, expiry or review trigger;
- source, record owner and evidence of integrity; and
- the method and destination for withdrawal.

Check each dimension independently. Permission to publish does not permit analytics. Permission for one site, audience, field set, recipient, campaign, model or period does not extend to another. Acceptance of terms, acknowledgement of a privacy notice, account creation, prior participation, relationship status, employment, site ownership, silence, continued browsing and preselected controls do not establish consent by themselves.

If an interface requests several permissions, preserve separate choices and records. A bundled “agree to everything” event cannot later establish permissions the person could not choose independently.

## Evaluate the evidence, not the label

Test each dimension separately under the applicable Pack of rules:

### Attribution and capacity

Can the affirmative act be attributed to the affected person or a representative whose authority is established? Could the person make that decision at the time? If age, disability, guardianship, employment, platform role or another relationship affects capacity or voluntariness, identify the relevant evidence without inventing it.

### Choice

Was refusal a real option? Check for unrelated detriment, service conditional on unnecessary processing, power imbalance, repeated pressure, deceptive design, preselection and take-it-or-leave-it bundles. A clickable reject control does not establish meaningful choice if the surrounding flow defeats it.

### Specificity

Could the person distinguish the purposes, operations, data categories, recipient classes and public or private destinations that materially affect the decision? Broad labels such as “improve services,” “partners” and “marketing” do not establish permission for an undisclosed operation.

### Information

Did the person receive understandable information about who would act, what would happen, why, which data was involved, material recipients or transfers, relevant consequences, duration and withdrawal? Identify the information actually presented. A later policy version or a link whose contents were not preserved cannot substitute for it.

### Affirmative expression

Does the record show an unambiguous action connected to the exact choices? Inactivity, defaults, inferred preference, unrelated clicks, hidden controls and events emitted before the choice do not satisfy this test. If capture cannot be reliably connected to the permission's scope, leave the state unverified.

### Currency

Have purpose, operation, controller or relying actor, data scope, recipient, interface, policy or surrounding expectations changed materially? Consent does not become permanent because no expiry field was stored. A material change needs a new decision or another independently established basis for authority. Relabelling the old record cannot supply it.

Failure on any required dimension makes consent unusable for that operation. Do not average strengths or replace underlying evidence with a provider's “consented” flag.

## Resolve a typed consent state

Return one of these evidence states for the exact permission scope:

- **absent** — no candidate record of affirmative permission exists;
- **captured but unverified** — a record exists, but attribution, scope, presented information, integrity or another validity condition remains unresolved;
- **usable** — every required condition is supported for the operation and period;
- **scope mismatch** — valid evidence may exist for other purposes, data, actors, destinations or uses;
- **expired** — the specified period or review condition has ended;
- **superseded** — a later decision or material scope change replaces the record;
- **withdrawn** — attributable withdrawal applies to this scope from its effective time;
- **disputed** — the person, record owner or evidence raises a material conflict; or
- **unavailable** — the consent owner cannot supply the record or authoritative state.

This state informs another operation's assessment. It does not instruct anyone to collect, publish, retain, delete or make contact.

## Treat withdrawal as a forward control with a lifecycle

Identify the person, permission scope, attributable withdrawal act, receipt time, effective time and responsible owner. Make withdrawal no harder than the relevant grant path permits. Do not require an unrelated account, explanation or new disclosure merely to express withdrawal.

Withdrawal normally ends future reliance on that permission. It neither rewrites history nor proves erasure of every stored copy. Resolve these questions separately:

1. When must new covered operations stop starting?
2. Which operations already in progress can be cancelled, isolated or allowed to finish under a separately established rule?
3. Which downstream recipients need a withdrawal or restriction signal, and who proves delivery and application of that change?
4. Which active data now lacks a supported basis for retention or use?
5. Which correction, restriction, deletion, archival, dispute or legal-hold process applies independently?
6. Which logs, backups, exports, caches, public copies or derived artifacts remain, under what restrictions and expiry?

Later permission can make future operations eligible only within its new scope and effective time. It cannot retroactively validate operations during a gap, cancel a withdrawal receipt or erase evidence that earlier permission ended.

## Reconcile withdrawal across owners

If a relying system keeps its own view of permission or permits downstream use, identify each affected owner and operation. Fix the permission revision, affected scope, withdrawal's effective time and authoritative revision recording it. Distinguish the owner's record, each delivery attempt, recipient acknowledgement and inspection of recipient state. An acknowledgement establishes only the stage it records. Confirm reconciliation only when the recipient's observed state covers the exact permission, scope and effective revision. Show missing, stale and unknown recipients.

For systems that restore earlier state, verify that a stale view cannot reopen withdrawn use. Before permitting use, compare restored state with current authority or an independently retained record of applicable revocations. If current authority cannot be established, hold the affected use. Preserve earlier use receipts as history. Proof that access stopped differs from proof that copies or derivatives were erased. A new grant needs its own scope and authority before future use may proceed.

## Fail closed in composition

When a personal-data assessment requires consent, only a **usable** state matching every required dimension supports that branch. Any other state means stop, redesign more narrowly or request specific missing evidence from the consent owner. Switching to another basis after consent fails requires independent applicability research establishing that basis for the same operation.

If consent is not the applicable basis, do not add this method as a decorative assurance step. Identify the actual basis and keep separate opt-out, objection, preference, publication permission, confidentiality and relationship controls with their owners.

## Produce the consent dossier

Include in the assessment result:

- exact permission scope and the operation requesting it;
- record and owner references;
- attribution and representative-capacity evidence;
- versions of the information and choices presented;
- evidence of the affirmative act and capture time;
- results for choice, specificity, information, expression and currency;
- resolved state, confidence, conflicts and missing conditions;
- withdrawal path, receipt, effective time, propagation duties and reconciliation evidence where applicable;
- separate questions about retention, restriction, deletion, holds and remaining copies; and
- the event that could make an unusable state eligible for reassessment.

Keep changeable facts about controllers, jurisdiction, policy, age thresholds, required wording, retention, providers and effective periods in the exact Pack or owning system. This Package defines the method, not those values.

## Stop conditions

Stop if the operation is unspecified, the affirmative act cannot be attributed to the person or representative, presented information cannot be reconstructed, permissions were bundled without independent choice, record integrity or time is unknown, use exceeds the scope, withdrawal or dispute cannot be reconciled, or another owner must decide capacity, validity, retention, deletion or a hold.

Do not infer consent to avoid an unresolved result. An evidenced absent, unverified, mismatched, expired, withdrawn, disputed or unavailable state completes the assessment.

## Before handing off

- [ ] Check the person, act, purpose and terms of the permission.
- [ ] Trace withdrawal to affected uses and stored copies.
- [ ] Report the actual result and any continuing obligation.
