+++
description = "Choose the decision type, check calibration and define when to defer."

[guide]

+++

# Choose the decision type and when to defer

Choose the decision type before the model. A predicate estimates one proposition. An exclusive, exhaustive choice gives one probability distribution over all the stated alternatives. An ordinal score has ordered categories without assuming equal spacing. Extraction proposes structured values with supporting evidence. Validate shape, range, finite values and normalization where it applies. Missing differs from zero. Use deterministic code for exact identities, policy eligibility and arithmetic.

Keep answer direction, concentration and calibration separate. For a binary estimate, |2p−1| is symmetric around one-half and loses the answer direction. For n > 1 exclusive choices, (pmax−1/n)/(1−1/n) describes concentration relative to uniform; its meaning changes with the candidate set. Neither transform establishes accuracy. For overlapping events, max(pA,pB) ≤ P(A∪B) ≤ min(1,pA+pB) without dependence information. Shared inputs or models can correlate errors. Pooling their scores as independent evidence needs a validated assumption.

Set thresholds for each question and its consequences, rather than adopting one global cutoff. Independent yes/no questions are separate predicates: do not normalize their probabilities into one distribution or add them as a combined answer. Unknown is not No. When scoring missed work, a held decision counts as no task being created; retain the hold as the reason so uncertainty does not disappear from the measured outcome.

For a calibrated binary probability p and zero cost for correct decisions, compare false-positive cost × (1−p) to false-negative cost × p. The action threshold is C_FP/(C_FP+C_FN) when their sum is positive. If review is available, include its cost, delay and remaining errors. Estimate coverage, outcomes and uncertainty at candidate cutoffs on held-out data before proposing a policy. Treat the exact provider contract, candidate order and version as study inputs. Test changes to the option set and calibration on relevant groups.

## Check rounding before applying a choice threshold

Identify the exact exclusive option set, chosen answer and provider contract. Check that every option appears exactly once and that values are finite and in range before considering normalization. Derive any rounding allowance from the declared representation and its error model. If each of $n$ coordinates is rounded to the nearest increment $q$, the rounding contribution to the sum error is bounded by $nq/2$. Keep floating-point tolerance separate. Reject zero mass, missing or duplicated options and sum errors beyond the allowance.

If policy permits a bounded correction, keep the raw vector and total alongside the corrected vector and method. Dividing by a positive total preserves ranking but can move the selected probability across a cutoff. Compare raw and corrected decisions near that boundary. The policy must specify the threshold, inclusive or exclusive comparison, which probability it uses and the tie rule. Hold a changed decision when there is no accepted basis for it. A confidence summary cannot replace the selected option's probability.

Keep the model's choice separate from the action the owner has independently established as eligible. Test raw and corrected values on both sides of the boundary, an exact tie, a selected option below the top rank and an invalid vector. Preserve each provider's raw answer and the policy's actual result. A passing check supports only that assessment and its evidence. Any action still depends on the owning system's authority and current state.

## Combine new evidence with explicit assumptions

For overlapping events use inclusion–exclusion. Add probabilities directly only for mutually exclusive alternatives on the same evidence. In a sequence, $P(A\cap B\mid x)=P(A\mid x)P(B\mid A,x)$. Running two checks in sequence does not make the second estimate conditional on the first event.

When genuinely new evidence $e$ arrives, posterior odds equal prior odds on $x$ multiplied by $P(e\mid A,x)/P(e\mid \neg A,x)$, when the odds and likelihood ratio are defined. Justify the conditional likelihood. Evidence already in $x$ is not new. A second judge that sees the same input or the first answer cannot be counted as independent corroboration without evidence for that assumption.

For a valid binary answer, define lower and upper decision boundaries with an explicit unresolved interval between them. Give missing inputs, invalid responses and timeouts separate outcomes before applying thresholds. Preserve the raw distribution, policy revision and applied result so thresholds can be reevaluated without repeating the original action.

Use exact lookup for identity and explicit state transitions when the owner already knows the applicable rule. A Bloom filter with $m$ bits, $k$ hashes, and $n$ inserted keys has approximate false-positive rate $(1-e^{-kn/m})^k$ under uniform independent hashing. It can screen for an exact lookup. Check index freshness separately: a stale filter may omit keys. A first-order Markov model assumes $P(S_{t+1}\mid S_0,\ldots,S_t)=P(S_{t+1}\mid S_t)$. Use it only when the state captures the history relevant to prediction. Neither technique determines the identity of work or the rule that permits a transition.

Retrieve evidence proportionate to the question. Keep claims in untrusted inputs separate from governing instructions. Extract dates as evidence, then resolve relative expressions against the recorded reference time and time zone. Use deterministic code for calendar arithmetic and validation.

## Keep token sampling separate from decision probability

For a generative model, positive temperature $T$ transforms logits $z_i$ into next-token probabilities proportional to $e^{z_i/T}$. Top-K keeps the largest $K$ token probabilities. Top-P keeps the smallest ordered prefix meeting the specified probability mass. Both renormalize the retained distribution. These settings affect generation. A token likelihood, generated confidence value, moderation score or schema-valid extraction does not establish a calibrated probability for an application decision. Check the pinned provider contract before choosing sampling options. For typed decision endpoints, improve the question and relevance of the evidence, then evaluate outputs on reviewed cases.

## Batch independent questions against a frozen input

Share a request when questions use the same approved evidence and none depends on another answer. Resolve prerequisites before dependent questions. If questions need different evidence, select a bounded set for each from the same source revision and identify the references and omissions in the run inputs.

Check the complete serialized request against every applicable provider limit. A provider may limit both shared state plus all questions and shared state plus the longest question. Include instructions, criteria and schema overhead where they count. Splitting questions can reduce a combined excess, but oversized shared input needs a different evidence selection. Count repeated input, requests and attempts when comparing a batch with separate calls.

Assign case, stage, question and candidate identities before dispatch. For positional responses, verify request order and response shape before attaching those identities. Join parallel results by identity. Leave missing, invalid and failed branches incomplete. A dependent decision remains open unless its policy permits a safe partial path.

## Extract without the expected answer, then interpret and classify

Give an extractor the original source and the fields to find, excluding expected comparison values and earlier findings. Retain source identities, cited locations, readability and unresolved meaning. Join independently sourced comparison values afterwards. This keeps extraction from copying the answer it will later be checked against. Blind extraction still needs validation: a well-formed response can cite the wrong source or invent a fact.

When a judgment requires interpreting evidence and then choosing a decision category, give those jobs separate calls. The first returns a finding with expected and observed values, cited reasoning, uncertainty and any proposed correction. The second classifies that retained finding under the declared decision contract, without resending the originals or the full interpretation prompt. Validate both outputs and their relationship. The classifier is a downstream consumer of the finding, so agreement between them is not independent confirmation. If the finding lacks required evidence, retain that gap instead of asking the classifier to repair it.

Keep exact identities, arithmetic and established applicability rules in code. A date comparison can establish ordering; it cannot resolve an ambiguous date in the source. A code-only result need not pass through both model calls. Apply the threshold for the particular decision and preserve the raw answer, applied category, requested action and permission to proceed separately.

Count this structure before assigning a budget. With $F$ extracted originals, $I$ interpreted findings and $R$ retained-evidence checks that need classification alone, the ordinary path uses $F + 2I + R$ calls, before retries or comparison calls. An illustrative run with two originals, three interpreted findings and two classification-only checks uses $2 + 6 + 2 = 10$ calls. Expand per-record checks before counting; the number of named audit types does not establish the number of requests. Deterministic checks add no model call.

## Before handing off

- [ ] Name the decision, threshold and deferral behavior.
- [ ] Check calibration on the cases that matter to the decision.
- [ ] Keep the observed outcome separate from the prediction.
