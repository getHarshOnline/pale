+++
description = "Change a shared interface while checking old and new producers, consumers and migration paths."
see_also = [{ scope = "local", resource_id = "contracts/identities" }, { scope = "local", resource_id = "contracts/definitions" }, { scope = "local", resource_id = "contracts/translation" }, { scope = "local", resource_id = "contracts/profiles" }]

[guide]

[task]
uses = [{ scope = "local", resource_id = "retry" }]
outputs = ["In the review reply: versions, callers and supported combinations.", "Located findings, negative tests and observed compatibility results."]
+++

# Evolve a shared contract with its consumers

Use a shared contract when independent producers and consumers must agree on identity, meaning, ordering, failure or state changes. Identify who owns those rules, participating systems, supported representations, coexistence period and validation points. Distinguish the accepted source from generated clients, adapters, examples and running instances.

Test actual consumer tolerance. Adding an optional field can change a signature, canonical serialisation, exhaustive match, strict validator or size limit. Check defaults, absent and unknown values, ordering, duplicates, precision and errors. A rename needs translation that affected readers actually use. Where coexistence is promised, test old readers with new output and new readers with saved inputs. Report invalid input, unsupported versions and state mismatches separately.

For a breaking change, identify old and new versions, which producers emit them, which consumers read them and how negotiation or routing works. Establish remaining old-version use, retirement conditions and rollback or forward repair. Change the shared definition first and test generated output through its consumer. Put these comparisons and remaining checks in the contract review; rollout still needs its own verification and decision.

## Choose the check

| If you are doing this | Read this |
| --- | --- |
| Check names, locators, version pins or binding decisions. | [Keep contract identity and permissions intact](contracts/identities.md) |
| Check resolution, generation or which validator checks ran. | [Check the definition and validation used by a consumer](contracts/definitions.md) |
| Check semantic compatibility, intermediary writes or exports. | [Keep meaning and effects through translation](contracts/translation.md) |
| <a id="separate-tuning-controls-from-contract-changes"></a>Check protocol, inherited behavior or a tuning change. | [Compare actual behavior across contract profiles](settings.md#check) |

Fix the accepted predecessor and proposed versions. Explain each finding through an actual caller, triggering input and consequence, and name the negative and compatibility tests required before release. State what ran, what failed and who owns the remaining checks. Use the HTTP guidance only when the interface uses HTTP.

## Before handing off

- [ ] Compare old and new producer and consumer behavior.
- [ ] Check compatibility, validation and migration at the actual interface.
- [ ] Name outstanding failures and who can resolve them.
