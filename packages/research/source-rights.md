+++
description = "Check whether supplied permissions cover each proposed use of each source or component."
see_also = [{ scope = "local", resource_id = "source-expression" }]

[guide]
+++

# Check the evidence for a proposed source use

Use this guide to assess whether supplied evidence covers a particular use of a source, component or asset, with its conditions and gaps. Applicable law and uncertain licence interpretation belong with the qualified decision-maker. This review supplies no permission of its own and performs no proposed action.

## Identify the material, actor and action

For every component, identify the work or asset and version, known author or creator, evidence of its current rightsholder or authorised licensor, the licence or other permission relied on, the actor proposing to use it, audience and recipient, destination, territory, effective period, commercial or private context, embedded material with a separate origin, and each proposed act.

Distinguish access, internal evaluation, copying, quotation, adaptation, translation, compilation, incorporation into a model or workflow, training, evaluation use, derivative creation, distribution, network provision, publication and commercial use. Evidence and conditions may differ for each. A project's licence label leaves the review incomplete until the material and proposed act are identified.

## Trace the permission to its source

Check whether the grant covers this version and component; whether the named grantor controls the relevant act; whether transfers, contributor terms, third-party credits or embedded materials leave gaps; whether actor, destination, territory, time and use fit; and whether the evidence is current, complete, authentic and reproducible.

Public availability establishes access, while reuse needs its own basis. Origin and attribution identify the source but do not grant rights. The author may no longer hold the rights at issue. A repository or container licence may not cover its data, images, fonts, models, documentation, generated assets or other components. Storing software and other material together does not establish common terms. Independently expressed text may still use assets or separately governed material that need their own evidence.

## Reconcile transfers and conflicting records

When a claim depends on transfer, contribution, exclusion or later recordal, reconstruct it one asset and right at a time. In the review table, identify the asset and scheduled member, actor and capacity, earlier holder, claimed recipient, relationship, date, and effect or trigger. Cite the clause and schedule from the relevant instrument version. Leave private evidence with its custodian and carry only a permitted reference and the finding needed.

Distinguish what each record establishes: earlier authorship or registration, proposal, prepared instrument, execution, fulfilment of a condition, owner account, payment, filing request and completed recordal. A prior-work exclusion or disclosure limits the relationship in which it is incorporated; verify that incorporation and seek separate evidence for a transfer. A blank saved signature field leaves execution unverified in that copy while allowing that another counterpart may exist. Keep an owner-confirmed outcome distinct from formal steps whose documentation is still incomplete.

Compare conflicting clauses, schedules, parties and identifiers at the relationship affected. Give distinct assets and rights separate rows, including exclusions in one revision and rights contingent on a later event. Name missing counterparts, authority, acceptance, triggers or register evidence and their custodians. Recheck affected rows when new evidence arrives. Verify that a disclosure, invoice or filing receipt cannot settle an unrelated transfer or recordal.

## Check coverage for each act and component

Use one review-table row for each material component and proposed act, with its supporting permission, conditions, exclusions and unresolved questions. This keeps a broad grant for one component from being applied to another and distinguishes permission for private drafting from public distribution.

Where relevant under the supplied terms, check attribution and its required form; copyright, licence and warranty notices; source or corresponding-material availability; reciprocal or share-alike treatment; noncommercial limits; restrictions on distributing adaptations; modification markings and dates; audience, destination, field-of-use or territory limits; expiry, revocation, withdrawal or supersession; downstream offers, disclosures or access duties; and separate embedded-component terms.

These are questions to investigate, not universal obligations. Take each obligation from its owning Pack, licence, permission, contract, rightsholder evidence or qualified finding. A label, badge or customary practice cannot supply missing terms.

## Refer questions that need qualified judgment

For a statutory, contractual or policy exception, identify the jurisdiction, effective period, exact test, facts and qualified decision-maker through an accepted Pack or responsible authority. A slogan, percentage, word count, educational purpose or claim of transformation cannot establish the exception.

If competing licences, ambiguous scope, a transfer, collective work, model or data terms, patent or trademark questions, or another material issue needs qualified interpretation, state that need and hold the affected conclusion. The Package identifies the question and evidence; the qualified owner decides it.

## Report a result for each proposed use

Choose one result for each act and component:

- **Supported by supplied evidence:** the permission chain and applicable details cover it.
- **Conditionally supported, with named obligations:** coverage is established and every remaining condition can be checked before the act.
- **Unresolved, needing more evidence:** a critical fact or permission link is missing, stale, conflicting or incomplete.
- **Unsupported or denied, stop:** evidence excludes the act or required permission was refused.
- **Qualified determination needed:** the question exceeds this method's competence.

Use conditional support only when the grant itself is established. Uncertainty about coverage remains unresolved.

Return the material and versions, actor and acts, creator and rightsholder evidence, permission chain, result per row, obligations and exclusions, evidence owners, uncovered recipients or places or periods, sensitivity to expiry or changed terms, required owner or qualified decision, stopping and resuming conditions, and actions still without permission. The review table is the work product.

The Package owns this reusable evidence method. Exact law, jurisdiction, effective period, licence bytes, contracts, rightsholder evidence and qualified findings belong in their governing Packs. Actual publication, distribution, upload, acceptance, training, execution, network provision and action records remain with the owning applications.

### Abstract component example

A repository may state clear terms for code while its bundled font and example image have different notices. Evidence supporting code distribution does not settle the other rows. Missing grants leave incorporation or publication of those components unresolved even when the repository is public and its code licence is known.

Hold affected work when material or version is unknown, components are missing from the inventory, the act is vague, a capable grantor cannot be traced, actor or destination is uncovered, conditions conflict or qualified interpretation is needed. Resume with the missing evidence, narrower act, separately assessed component, owner ruling or qualified determination. A downstream workflow needing to continue does not supply permission.

If source influence on a draft is unclear, use the [expression assessment](source-expression.md).

## Worked example: model weights and generated assets

A local image workflow proposes a base model, a community repack and an identity adapter. Give each component and proposed act its own row: reading metadata, obtaining weights, running the model for the project, distributing weights and publishing an output can have different conditions. Read the model card and licence file at the selected revision, record when they were read, and follow a repack to its origin as well as its own upload terms. Two community uploads of the same model can carry different or incomplete claims.

| Terms to distinguish | What to establish for the proposed use |
| --- | --- |
| Permissive, such as Apache-2.0 | The applicable notices and other obligations at the pinned source |
| Responsible-use, such as OpenRAIL++-M | The permitted use and restrictions that must accompany the weights |
| Non-commercial or non-production model use | The licence's actual definitions and any separate grant; output rights and absence of payment do not settle model-use permission |
| Research-only adapter | Whether the work fits the permitted evaluation scope; a research run does not establish permission for a shipped asset |
| Gated access | The account owner's accepted terms and approved access, separately from the model's use rights |

Suppose the pinned terms allow commercial outputs but restrict running the model to non-production use. The output clause alone cannot clear a production run. If a research-only adapter covers evaluation, that finding supports only the named evaluation; check separately whether the proposed shipped asset is covered. Record any separate grant and the actor and use it covers. A gate establishes how access is obtained, not that every intended use is permitted.

These are classes for inspecting supplied terms, not legal conclusions from a label. A workspace may select permissive models by default, require a project's review of responsible-use restrictions, or exclude research-only components from shipped work. Those are that project's choices. Keep its actual policy and accepted terms with their owner, and carry each component's applicable conditions into the asset's provenance. A missing grant or conflicting condition leaves that component's use unresolved under the result categories above.

## Before handing off

- [ ] Was each intended act checked against that source's terms?
- [ ] Do obligations and unknowns travel with the material?
- [ ] Does clearance remain with the person responsible?
