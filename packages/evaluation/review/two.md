+++
description = "Reconcile two independent reviews while keeping disagreement visible."

[task]
roles = [{ scope = "local", resource_id = "review/independent" }]
outputs = ["Two completed independent reviews, or the exact missing coverage.", "Shared findings, individual findings, disagreements and unresolved questions."]
+++

# Reconcile two independent reviews

Give both reviewers the same exact artifact, decision criteria, available evidence and limits on what they may do. State the consequence of accepting a defective result. Each reviewer works without seeing the other's reasoning and returns a conclusion, evidence for each criterion, uncertainty and any condition that blocks acceptance. Keep their reviews separate until both are complete. The host or review owner must provide the actual isolation.

Reconcile four outcomes separately: findings supported by both reviewers, findings raised by only one, genuine disagreements, and questions neither could resolve. Settle each question using evidence. Two votes do not make a finding true, and a shared omission does not make a candidate pass.

Return both original reviews and the reconciled decision in the existing review result, with any specific unresolved question or repair request. If only one independent review exists, report that coverage and its limit. Rephrasing it does not produce a second review.

This operation reviews evidence. Another round can assess new evidence, but a review result supplies no permission to edit, commit, publish, deploy or take another consequential action.

## Before handing off

- [ ] Preserve each reviewer’s independent reasoning.
- [ ] Resolve the disputed claim using the relevant inputs.
- [ ] Keep material disagreement visible in the decision.
