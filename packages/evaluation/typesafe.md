+++
description = "Check how the selected provider returns decisions and confidence."
see_also = [{ scope = "local", resource_id = "decision" }]
official_sources = [{ label = "typesafe-decision native documentation", url = "https://docs.typesafe.ai/confidence" }]

[guide]


[stack]

+++

# Check how a provider returns decisions

Read the output contract for the exact provider version and predicate or choice operation. Check candidate identifiers, response order and schema before using confidence measures. Compare reported confidence with observed correctness in relevant groups, including when the number or wording of options changes. Treat the provider's concentration score as one feature to study. Application policy determines eligible decisions and accepted thresholds.

## Read each confidence measure with its answer

TypeSafe's [confidence reference](https://docs.typesafe.ai/confidence#how-confidence-is-calculated) defines different summaries for Noul, Choice and Score. Noul returns the probability of yes without a separate confidence field. A local $|2p-1|$ summary discards the direction. For Choice with $n>1$, the normalized concentration is $(p_{\max}-1/n)/(1-1/n)$. With a fixed option set this is a transform of top probability. It adds no independent evidence.

For Score, let $i$ range over the $n>1$ ordered level indices and $m$ be a most probable level. TypeSafe computes

$$
D_0=\frac1n\sum_i\left|i-\frac{n-1}{2}\right|,\qquad
c=\max\left(0,1-\frac{\sum_i p_i|i-m|}{D_0}\right).
$$

This formula uses positions in the ordered levels. It does not assume that moving between adjacent levels has equal consequences in the application. Retain the full distribution and inspect the probability of the level or option that policy actually requires. Check calibration against observed correctness for the exact question and provider.

## Worked example

A provider returns a score but omits the decision required by the caller. Check the typed result and exercise the uncertain or rejected route. Keep the score available for diagnosis without treating it as an accepted decision.

## Before handing off

- [ ] Check the provider’s inputs and possible outcomes.
- [ ] Exercise a missing, uncertain or rejected value.
- [ ] Confirm how the caller handles deferral and success.
