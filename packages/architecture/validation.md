+++
description = "Compare required checks with assertions actually executed and explain changed coverage."
see_also = [{ scope = "local", resource_id = "contracts/definitions" }]

[guide]
+++

# Verify which validation checks ran

<a id="check"></a>

Start with the owner's required checks for the artifact versions, entry point and mode. Each check needs eligible inputs, an assertion and an expected result. Inspect discovery, parsing, dispatch and outcome separately for each family. A listed file, schema or test name does not prove an assertion executed. If selection is empty, establish that the eligible scope really was empty.

Use an unchanged valid fixture and a well-formed single violation for each rule tested. Verify that the result identifies the intended check. Also test a malformed prerequisite, missing dispatch and early-stage failure. Mark later checks unobserved if execution never reached them. Rejection elsewhere does not prove the intended assertion caught its violation.

Compare required checks with actual outcomes before accepting the result. Distinguish passed, failed, blocked, unimplemented and intentionally excluded checks. If development mode permits open items, list what remains and compare the same input in strict mode. A passing subset supports only its covered claims. Strict acceptance needs an observed result for every required check under the approved policy. An approved exception changes requirements; it does not supply evidence that the waived check passed. Semantic review and release approval remain separate.

### Reconcile changes to the required check inventory

Identify the previously approved and proposed inventories separately from checker output. Compare stable check identities and meanings: scope, input selector, expected-answer source and whether failure blocks progress. Map renamed, narrowed, split and merged obligations explicitly. Changing the list can improve the verdict without improving the candidate.

For every changed or missing obligation, explain what happened and why in the inventory change review. Distinguish satisfied, superseded, deferred and retired requirements. Satisfaction needs its own proof or accepted review. Narrowing or retirement needs the owner's decision and a visible coverage change. Associate the successor policy and each decision with the artifact, mode and intended release. Preserve the last approved scope and historical results.

Test an approved inventory change and a negative control that silently drops or narrows a required row. Compare the checker's verdict with an independent inventory comparison. Passing an empty or altered list does not resolve the earlier obligation. Stop if the predecessor, mapping or decision cannot be verified. Report old and new scope, support for each change and the acceptance requirement still unresolved.

## Before handing off

- [ ] Reconcile required check IDs with outcomes from the actual run.
- [ ] Test a well-formed violation that the intended check must catch.
- [ ] Record checks the run skipped or never reached.
