+++
description = "Check approval, execution and filing claims separately against their original records."
see_also = [{ scope = "local", resource_id = "rules" }]

[task]
outputs = ["Evidence for each claim, party and period.", "Unmet requirements, responsible custodians and actions that must wait."]
+++

# Review governance evidence

Identify the exact act, legal parties and their capacities, instrument revision, effective period and record custodians. For each party, establish who could decide, sign or submit on its behalf at that time, with the scope, conditions and expiry of that authority. A shared director, adviser, signatory, repository or payment channel does not supply another party's approval. Keep unilateral authorisations identified as such.

For each required approval or disclosure, trace relevant interests to the act and effective period. Check the deciding body's composition, delegation, required quorum, participation restrictions, disclosures and recusals against the original record. Link a resolution to the act and version it covers. A material change in price, scope, capacity, counterparty or terms requires a separate check of whether renewed approval is needed. Resolve uncertain requirements through [the applicability method](rules.md) with the qualified owner. Keep the sufficiency of evidence separate from the professional ruling.

Assess approval, signing of counterparts, performance, payment, register entry, submission and final outcome as separate claims. Preserve chronology and identify the evidence needed for each. Read the actual signed counterpart, authenticated record or original attestation where required. An undated signature image or typed name has a narrower evidentiary role. For a service or related-party payment, connect approved scope and price basis to the agreement, delivery or usage, and actual invoice or payment. Payment alone does not establish delivery, authorisation, correct price or tax treatment.

For a filing, match the entity, event, reporting period, submission identity and submitted artifact with the portal or custodian record. Distinguish prepared, submitted, receipt or fee recorded, accepted, rejected, returned for correction and superseded where the system exposes those states. Deduplicate copies by filing identity and retain the link between a correction and its predecessor. Verify the currently accepted revision or state which record could not be inspected. A local gap or receipt does not establish current status. Check each required period independently for recurring filings. Use `pale.assurance:register` separately to reconstruct the resulting holder, officer or obligation balance.

In the review result, give each applicable claim, party and period its own evidence row, with custodian, exact source, supported scope and unresolved condition. Distinguish historical evidence, currently observed state and proposed next action. Hold each affected action when scope, execution, acceptance or professional judgment remains unresolved. Reviewing evidence grants no permission to approve, execute, file, amend, pay or publish.

## Worked example

A case file contains an approved draft, a signed copy with a changed amount and a submission confirmation. Record the amount shown by each document. Ask the owner whether the earlier decision covers the signed version. Check the filing record for the accepted version. Keep approval of the changed amount and acceptance of the filing as separate open questions until their respective records support them.

## Before handing off

- [ ] Were approval, execution and filing records read in their context and actual versions?
- [ ] Does each record support the claimed act, party, capacity and date?
- [ ] Are conflicts, missing authority and undecided outcomes kept visible with the owner who can settle them?
