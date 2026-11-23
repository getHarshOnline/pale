# Check a calculation request and its authority

Use this preflight before asking a calculation engine to evaluate an accepted scenario. It checks the purpose, authority, exact accepted rules, private facts, implementation, numerical conventions, validation and limits on actions. It neither performs the calculation nor defines a provisional engine API.

## Resolve the evaluation request

Describe the exact calculation, scenario, projection, comparison or finding requested, its purpose and permitted use. Broad requests such as “check compliance” or “calculate everything” hide different inputs, authorities, periods and consequences.

Identify the requesting person or system and its authority. State the controller's scope and any narrower relationship, account, entity, resource, transaction or scenario limits. Permission to view one source or request one finding gives no authority over adjacent facts or actions.

## Bind the normative basis

Select the exact immutable Pack of law or other governing rules, identified by its identity and content hash. State the effective period it covers and the evidence connecting those rules to the scenario. A familiar name, “latest” label, mutable URL, cache entry or candidate proposal is not a Pack identity.

Confirm the Pack's attestation and validation for the intended use. If applicability depends on a private fact, identify the established result for that condition or stop. Keep private facts out of reusable Package guidance.

<a id="bind-finance-owned-facts-and-implementation"></a>

## Bind private facts and implementation

Identify the private source snapshot or reproducible reference supplied by the owning application. Its owner, permissions, capture time, scope and freshness must suit the request. Preserve conflicts among ledgers, statements, contracts, elections, classifications and other private records rather than choosing a convenient value.

Identify the implementation, model, code, configuration and dependencies that will perform the evaluation. A product name does not identify its implementation. A result cannot be reproduced or checked when the execution path is unknown.

State currency, unit, scale, event period, assessment or reporting period, rounding convention, scenario assumptions and comparison basis. Resolve conflicts before invocation. If a convention can change the finding, specify it rather than relying on an implementation default.

## Bind validation and effect ceiling

Identify validation evidence for the exact Pack, implementation, scenario class, units, periods and intended use. Distinguish inspection, tests, independent review, professional attestation and runtime execution; none establishes another.

Set explicit limits on permitted actions, such as analysis only, scenario comparison only or preparation of a finding for review. Exclude every action not separately authorised. Invocation itself grants no permission for disclosure, filing, payment, allocation, trading, signing, publication, advice or approval.

These requirements describe meaning. They do not prescribe field names, wire formats, service methods or an ABI.

## Preserve ownership

- The Package owns this reusable preflight method.
- The Pack owner controls the exact rules, jurisdiction, effective period, authority, source history and attestation.
- The calculation engine and its owning application retain private facts, deterministic calculation, scenario state, derived findings and execution receipts.
- External record and source owners retain authority over their records and sources.
- The caller holds only the authority established for the declared purpose and scope.

Combining inputs does not transfer one owner's responsibilities to another.

## Reject or stop

Do not invoke when:

- Pack identity, hash, attestation or effective period is absent or inconsistent;
- applicability depends on missing, stale, unauthorised or conflicting facts;
- the private snapshot is ambiguous, stale for the purpose, outside controller scope or not reproducible;
- the implementation, model, code or configuration path is unknown;
- currency, units, scale, period, rounding, assumptions or comparison basis conflict;
- relevant validation evidence is missing, expired or covers different inputs;
- the requested use exceeds the accepted purpose or limits on actions;
- a candidate proposal or mutable retrieval is offered instead of an accepted Pack; or
- required professional, signatory, regulator, fiduciary, committee or other external authority is absent.

Identify the missing or conflicting requirement and who can resolve it. A failed preflight cannot be bypassed with an improvised spreadsheet, remembered rule, approximate calculation or unsupported advice.

## Interpret the result narrowly

A calculation result is a derived finding supported by the relevant validation under the exact Pack, facts, implementation, conventions, scenario, purpose and authority selected for the invocation. Changing any of those may make the finding inapplicable.

The result does not inherently establish:

- accepted law or correct attestation of the Pack;
- professional, legal, tax, accounting, fiduciary or investment advice;
- a filing, return, disclosure, certification or signatory act;
- a payment instruction or proof of payment;
- an investment recommendation, trade or allocation approval; or
- a change to external state.

Package Markdown must not calculate tax, GST, penalties, filing dates, returns, trades or portfolio allocations. Exact arithmetic belongs to the accepted calculation implementation using the selected Pack and private snapshot.

## Unavailable and degraded outcomes

If the calculation engine is unavailable, report that and preserve the preflight evidence. If the required Pack has not been accepted, return proposal-only or not ready; research evidence cannot replace it. If the private snapshot is inaccessible, return unresolved without inferring its contents. If caller authority is missing, return denied without disclosing protected facts. If validation or attestation has expired or is disputed, stop and identify the required revalidation or owner decision.

Unavailable, unresolved, denied and proposal-only are valid completed outcomes. Do not substitute a plausible number whose authority or sources cannot be established.

## Abstract preflight example

A controller asks to compare two reporting scenarios for an unnamed resource during Period B. The request supplies an analysis-only purpose, authority over that resource, an accepted Pack identity and hash covering Period B, applicability findings, an authorised private snapshot, implementation release, units, rounding, scenario assumptions and validation evidence. Filing, payment, disclosure and advice are excluded.

The preflight finds that one scenario uses a classification present only in a fetched candidate amendment, outside the accepted Pack. It permits evaluation of the first scenario under the exact accepted inputs, marks the second proposal-only and stops it before calculation. The eventual finding is available for review; it is neither an instruction nor evidence of an external action.
