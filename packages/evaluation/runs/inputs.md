+++
description = "Fix the inputs available at decision time for a fair replay."
exposure = "internal"

[guide]

+++

# Freeze the decision-time inputs for a fair replay

## Freeze what was available at decision time

State the decision time with its time zone or exact instant. For each source value or mapping, distinguish the time it describes, the interval when it applies and when that exact revision became available to the route being assessed. Record observation or extraction time separately. A backfilled record or correction may apply to an earlier event while becoming available much later. Neither the event date nor a current extract establishes historical availability.

Reconstruct inputs from retained versions with evidence that they were available by the cutoff. An effective-dated mapping must both cover the event and have been available in that revision. Apply the owner's precedence and tie rules within those limits. Leave missing, overlapping or unverified mappings as gaps when those rules cannot resolve them. Reconstruction using current data is a separate exercise; state its limits with its results.

Keep decision inputs, later outcome evidence and retrospective corrections in separate manifests or partitions. Later observations may inform assessment at a declared outcome cutoff. They cannot enter retrieval, prompts, features or reference selection visible to the evaluated decision. Preserve independently supported observations with their applicable cutoff instead of dropping them from the case entirely.

Test a late-arriving row with an early event date, a backdated correction, an unverified historical mapping and a value exactly at the cutoff. Inspect reconstructed decision inputs separately from later outcome inputs. Keep cutoffs, source coverage, version references and excluded cases with the comparison.

## Resolve a reused protocol identifier

Treat a displayed identifier as a way to locate a protocol within its owner's source and revision. If sources reuse an identifier, identify each definition by its source and decision. Compare the question, population and split, configurations tested, baseline, metric and unit, thresholds, budgets and permitted next actions. A shared measure or dataset does not establish a shared comparison or decision.

Give each distinct protocol an unambiguous reference before accepting a run. Preserve the original labels and definitions. If definitions conflict or a threshold's unit is unclear, leave the comparison unresolved until the owner settles it. An approved rename changes navigation. Changed configurations, measurements or decision rules require a distinct revision or protocol with its own evidence. Keep a proposed mapping separate from historical results.

Test a reused identifier without its source, a result offered to the wrong protocol, and matching data or metrics with different baselines. A selected protocol and its exact run can proceed through their own checks. Carry a pass, numeric threshold or downstream decision across reused identifiers only after separately establishing that the definitions correspond.

## Admit an external run without changing its evidence class

Start with the experiment owner's fixed question, comparison and source-run states eligible for assessment. Identify the source owner, run and generation, original outcome receipt, fixed inputs and configuration, and permitted artifacts with their digests. Before opening evidence, verify permission for the evaluator, purpose, scope and destination. A catalog entry, orchestration event or downstream acceptance can identify a run but does not grant export or evaluation rights.

Preserve the kind of evidence, its sources and limits through acceptance and assessment. A citation supports its source's original claim. A simulated observation supports a claim about that run under its stated conditions. An inference retains its inputs and assumptions. A score or valid simulation alone establishes no real-world observation, operational outcome, acceptance or permission to act. Report missing and ineligible cases explicitly.

Open only permitted artifacts. Before using evidence, reject a substituted run or generation, mismatched receipt or artifact, and absent or incompatible permission. Return a separately identified assessment linked to the exact inputs and their limits. The source owner retains the original run; the authorised decision-maker decides whether to adopt a conclusion. Test a valid handoff, substituted receipt, withdrawn access and an attempt to treat one kind of evidence as a stronger kind.

## Worked example

A rerun includes a final case label that the first run could not see. Rebuild the decision-time snapshot, identify the exact protocol version used by each run and compare again. Keep third-party results labelled with the source that observed them.
