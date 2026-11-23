+++
description = "Write down a lasting design decision on the repository’s decisions page: the question, what pushed on it at the time, the real alternatives, the evidence, why this one, what it costs and which earlier decision it replaces."

[guide]

[task]
uses = [{ scope = "external", resource = { package_id = "pale.engineering", resource_id = "handoff" } }]
outputs = ["In the repository’s decision home: the proposed choice, evidence, alternatives and any accepted trade-offs.", "Affected claims and owners, required checks and recovery conditions."]
+++

# Keep the reasons for an architecture decision

Record a structural choice when its reasoning will still matter after the people implementing it have moved on.

## Record threshold

Use the repository's decision home when a choice establishes a lasting component responsibility, dependency direction, public contract, data model, operational property or hard-to-reverse technology commitment. Routine edits and choices already required by an accepted rule do not need another architecture record.

## Required evidence

Explain the question, why it arose, the options actually considered, what was known at the time, the chosen trade-off and consequences future work must respect. State uncertainty instead of rewriting the past with confidence gained later. Link the source, tests or other artifacts that establish the context and result.

## Lifecycle

A decision describes what was knowable at a particular time and revision. Later learning can supersede it without erasing that history. A successor names the earlier record, explains what changed and states the new consequences. Keep the history readable while making clear which decision governs current work.

## Bind the disposition to the reviewed candidate

Keep the precise candidate, predecessor, affected scope, supporting observations, decision-maker and applicable policy with the decision record. Verify that an acceptance or refusal refers to those materials and comes from someone permitted to decide for that scope. Preserve conditions, expiry and deferred parts. Conditional acceptance covers only what it says. A changed candidate or premise, or expired decision-making authority, needs a fresh decision before dependent work proceeds.

Check how the contract identifies the reviewed material when showing and reading it back. If the contract defines canonical equivalence, equivalent renderings may identify the same material. If a signature covers exact bytes, preserve and verify those bytes. In either case, reject substitution of the candidate, predecessor, owner or scope even when the displayed conclusion looks identical. Keep the original material and decision when a successor is recorded.

Test a complete decision path, then changed input, wrong scope or decision-maker, a stale predecessor and conditional acceptance. Read the saved decision and verify that no unapproved effect occurred. Matching the reviewed material, authenticating a decision, holding its records and releasing the result each require their own evidence. Put reusable cases in the owning tests and their observed results in the review.

## Trace the effects of a decision

Identify the proposal or accepted revision, its owner, changed claims and preconditions. Before acceptance, find dependent owners and the checks the change would require. Trace each changed claim to consumers, including derived views and artifacts. A shared identifier or link alone does not prove that a consumer relies on the claim.

For each affected output, keep its input revision, owner and resulting action in that output's existing change or build record. Leave it current when its relevant dependency still holds. Otherwise recheck, invalidate or regenerate it according to its owner's rules. Keep unresolved dependencies explicit, and preserve earlier observations, decisions and the inputs on which older outputs relied.

Track acceptance, application to the owning system's state and verification of dependent outputs separately. A view update can fail after a decision is saved. Inspect the owning record and each required view independently. Every new effect needs its own permission and observed result. If recovery changes state, record its decision and preconditions with a link to the original attempt, preserving both outcomes. Rejection or deferral never permits applying the proposal.

## Output and boundary

Return a record ready for the repository's decision home, with affected claims and owners and any follow-up checks or recovery. Stop when a future maintainer can distinguish a deliberate choice from an accident. The repository decides where the record lives, how it is identified, who accepts it and how it is saved.

Identify who may decide, the observation date and conditions under which the proposed choice applies. Prepare it in the repository’s existing decision home and label it as proposed until its owner accepts it. Keep the reasons for rejecting or deferring alternatives and the evidence that would reopen the choice.

## Before handing off

- [ ] Record the question, choice and the reasons for it.
- [ ] Keep rejected alternatives and resulting constraints understandable.
- [ ] Name the condition that would reopen the decision.
