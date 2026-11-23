+++
description = "Compare architecture choices and recommend responsibilities and change order."

[task]
uses = [{ scope = "local", resource_id = "contracts" }, { scope = "local", resource_id = "components" }]
roles = [{ scope = "local", resource_id = "architecture-review" }]
outputs = ["In the design review or reply: current and proposed responsibilities, alternatives and trade-offs.", "Unresolved assumptions, affected owners and acceptance tests."]
+++

# Propose an architecture change

Start at the real entry points and trace callers, stored state, contracts, deployment and recovery. Compare the viable alternatives against the same user-visible result. Include keeping the present design or deferring the choice when either still meets the need. Separate mandatory safety and acceptance rules from trade-offs in quality, cost and complexity.

For each option, explain its dependencies, who may invoke it, how it fails and recovers, and what observation would rule it out. Configuring an option, installing a provider or passing a fixture does not prove that the consuming system has permission or is ready to use it. Leave unverified prerequisites explicit. Preserve a usable fallback and say when to reconsider the choice.

Give the independent architect the intended result, source revisions, alternatives, accepted constraints and unanswered questions. Return the recommendation and proposed order of changes in the task's design review or reply.

## Before handing off

- [ ] Name the decision and the actors or systems it affects.
- [ ] Compare real alternatives under the stated constraints.
- [ ] State the change and the observations that would support it.
