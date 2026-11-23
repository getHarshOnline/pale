+++
description = "Choose parsing or model interpretation from input regularity, error cost and how reliably output can be checked."
see_also = [{ scope = "local", resource_id = "sources" }]

[guide]
+++

# Choose how to extract structured information

Choose the least uncertain route that meets the required output schema and error tolerance. Inspect representative variation, kinds of ambiguity, volume, latency and cost limits, available validation, privacy and the consequences of a wrong field.

Use deterministic parsing for stable syntax with recognisable failures. Use model interpretation for named ambiguities where rules would be brittle or uneconomical and outputs can still be constrained and checked. A parser miss cannot establish that the model's answer is correct.

For each route, define accepted, rejected, low-confidence, conflicting and unparseable results. Validate structure and meaning. Keep source spans or references for consequential fields, and measure accuracy by input class so a reliable majority cannot hide a dangerous minority. Set an owner or human-review threshold where automated evidence cannot support the consequence.

Compare hybrid approaches by maintenance and failure cost alongside accuracy. In the design or review, return the chosen route, ambiguity classes, validation and confidence rules, evaluation cases, fallback, cost limits and stopping conditions. Actual parsers, models, credentials, execution and stored outputs remain with their owning systems.

## Before handing off

- [ ] Does the extraction approach fit input regularity and error cost?
- [ ] Were structure and uncertainty checked before using values?
- [ ] Was a difficult result compared with its original source?
