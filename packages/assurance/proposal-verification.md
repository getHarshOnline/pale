# Verify the evidence for proposed rules

Use this method to turn completed applicability research into a proposed change to a Pack of governing rules that its owner can review. Research must first establish the questions, scope and source chain. This method stops before professional attestation, acceptance or activation of the Pack, and execution of an evaluation.

## Inputs and proposal boundary

Require:

- applicability findings for the defined scope;
- exact source references or captured source bytes;
- the proposed assertions about governing rules;
- the intended consumer and permitted use;
- every known dependency on private facts; and
- the predecessor Pack, or an explicit statement that none exists.

Reject inputs consisting only of a summary, search result, remembered value, unexplained claim about current state or source collection without defined scope. Verification cannot supply missing applicability reasoning by making uncertain language sound confident.

The lifecycle is:

    mutable retrieval or research
    → evidence-backed candidate proposal
    → qualified review and attestation outside Package authority
    → immutable admitted Pack
    → separately bound native evaluation

Package guidance may structure and test the proposal. It cannot certify law, provide professional attestation, accept a candidate for the Pack owner, activate immutable bytes, authorise a calculation engine or permit an external action.

## Build an atomic assertion ledger

Within the proposal, separate assertions that can be accepted, rejected, corrected or replaced independently. For each, state:

- exact jurisdiction and kind of authority;
- affected subjects, roles, transactions and conditions for applicability;
- effective period, including commencement, transition and historical treatment;
- exact source identity and reproducible reference or captured-byte hash;
- its relationship to amendments, corrigenda and superseded rules;
- intended consumer and the question it may answer;
- private facts it requires but does not contain;
- conflicts, alternative interpretations and unresolved dependencies; and
- the predecessor assertion it continues, changes or withdraws.

Keep different kinds of claims separate. A classification under a rule is not a private fact. A value differs from its unit, a threshold from the rule that makes it relevant, and a rounding convention from a source-derived quantity. An exception does not establish that a subject meets its conditions. An available election is not necessarily effective.

Where relevant, make values, units, currencies, periods, rounding, thresholds, classifications, exceptions and elections independently reviewable. This requirement concerns meaning; it does not permit inventing a Pack schema, field name, serialization or runtime ABI.

## Verify the candidate’s meaning

### Claim-to-source traceability

Trace every material assertion to the exact source version reviewed and the role that source serves. Identify paraphrase, inference and professional interpretation instead of presenting them as quoted authority. A source may support only part of an assertion.

### Predecessor comparison

Compare each assertion and its applicability conditions with the predecessor. Identify additions, removals, changed meanings or periods, new exceptions, withdrawn elections and changed sources. A byte diff shows change but does not explain its significance for the rules.

If no predecessor exists, check that the proposal implies no unestablished continuity, historical coverage or prior acceptance.

### Cross-source contradiction review

Check whether sources disagree, serve different roles, govern different periods or apply to different subjects. Preserve unresolved conflicts in the candidate. Do not silently favour the newest, easiest to retrieve or most readable source.

### Units and effective periods

Check dimensional consistency, currency, scale, period boundaries, where rounding occurs, and relationships among event, assessment, reporting and commencement dates. Verify that examples do not combine a value from one period with authority from another.

### Boundary and negative cases

Construct examples covering:

- a positive case where all required conditions are established;
- a negative case where evidence refutes a decisive condition;
- cases immediately on both sides of a threshold, date, classification or scope boundary;
- a transition spanning a change in treatment;
- a historical replay using only the Pack version and facts available for that period; and
- missing evidence that must stop the dependent work.

Examples test meaning. They neither establish live facts nor replace an accepted deterministic implementation.

### Independent review

Require independent, qualified review when a mistaken interpretation could materially affect legal or financial matters, disclosure, safety or rights. Name the required acceptance or attestation and its owner. Package guidance cannot appoint that reviewer, claim they acted or substitute automated agreement for professional judgment.

## Rights, permitted use, and correction

Identify permitted evaluation, derivative, disclosure and publication uses to the extent required by the Pack owner. Access alone does not establish a right to copy, redistribute, transform, publish or use a source in deterministic evaluation. Preserve restrictions and unresolved rights questions for the authority accepting the Pack.

Define how a candidate may be corrected, withdrawn, rejected or replaced. Link a successor to the affected assertion and effective period. Withdrawal must preserve evidence needed to explain earlier evaluations; correction must not silently rewrite an immutable predecessor.

## Fail-closed verification behavior

Return “not ready to propose” if a material assertion lacks reproducible evidence, applicability conditions remain unresolved, the source chain is incomplete, units or periods conflict, a predecessor cannot be identified, rights are unclear for the intended use or the required independent review cannot be specified.

Return a candidate with explicit unresolved items only if its permitted use tolerates them and no consumer can mistake them for accepted truth. Do not fill gaps with a plausible rate, deadline, classification, formula, authority or interpretation.

## Fetch-to-propose law

Preserve these semantics exactly:

- mutable retrieval may propose;
- retrieval cannot attest;
- retrieval cannot enter deterministic evaluation;
- only exact admitted Pack bytes selected by the proper authority may be consumed later.

Freshness supplies evidence for review, not permission to change an evaluation.

## Abstract amendment example

An accepted Pack governs a calculation already running for Period A. A retrieval provider returns a new amendment that may change an input category from a future commencement date. The fetched bytes are current and authentic, but fall outside the selected Pack. Their relationship to transition provisions has not been resolved, and they lack independent attestation.

Retrieval may start a candidate proposal identifying the new source hash, intended future period, changed assertion, predecessor relationship, transition questions and boundary examples. The proposal cannot replace bytes selected for the running evaluation, change historical replay retroactively or be consumed by the evaluator. The amendment can affect a later evaluation only after the Pack owner completes qualified review, attests its intended meaning, accepts immutable successor bytes and a later invocation selects that exact successor.
