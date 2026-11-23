+++
description = "Check that formal statements support written claims and distinguish proof from implementation evidence."

[task]
outputs = ["In the release review: claim-to-declaration mapping, proof checks and coverage results.", "Unresolved meaning, implementation and release claims, with the responsible owners."]
+++

# Check that a formal proof supports the written claim

## Identify claims and proof inputs

Fix the prose or specification, formal sources, dependency lock, checker, proof engine and options. In the release's existing claim map, identify every claim presented as proved, give it a stable location and map it to fully qualified declarations that prove it. Record definitions and assumptions using versions actually resolved by the proof engine. Mark open, deferred and intentionally informal claims. The text owner decides which claims the release includes.

## Run proof and coverage checks

Build the chosen sources with the pinned toolchain. Resolve and inspect declarations and their transitive axiom dependencies. Check them against project-approved assumptions; reject proof placeholders and unapproved assumptions. Where the owner requires matching coverage in both directions, trace claims to declarations and declarations back to claims. A file existing, a build passing and a declaration name resolving establish different facts.

Test the checker with a missing or stale declaration, an unmapped or excluded claim and an unapproved axiom. Run both the normal development check and actual release-coverage check. In the review, distinguish expected open obligations from unexpected failures. Passing a partial check leaves strict release requirements open. Preserve outstanding obligations, and keep checking the checker's regressions separate from reviewing the mathematics.

## Compare the meaning of each statement

Compare domains, quantifiers, hypotheses, definitions, units, conclusion and scope for each material prose claim and formal statement. Use a plausible same-name theorem with narrower hypotheses as a control. A proof over a finite domain cannot establish a more general one. A counterexample refutes the quantified claim it addresses. Interpretations, special cases and reformulations each need their own correspondence evidence.

If code or a wire schema claims to implement the result, separately identify its profile, implementation and consumers. Equivalence needs refinement evidence or verified extraction. Use explicit conformance fixtures and counterexamples for narrower behavioural claims. Keep the build, declaration and axiom checks, coverage result, statement review and implementation evidence distinct. Research and release owners decide what each artifact may claim or promote.

## Worked example

An article claims the result for every real input, but the declaration with the matching name assumes positive input. Record that hypothesis in the claim map and leave the wider claim open. If the release also claims that a numerical routine implements the result, inspect its input check and test an input outside the positive domain for the documented refusal. A successful proof build and observed routine behaviour answer different questions.

## Before handing off

- [ ] Does each prose claim match its formal statement and assumptions?
- [ ] Can implementation claims be traced to supporting code or runs?
- [ ] Are gaps between proof, description and execution visible?
