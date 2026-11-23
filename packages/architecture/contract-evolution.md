# Evolve a shared contract with its consumers

Use a shared contract when independent producers and consumers must agree on identity, meaning, ordering, failure or state changes. Identify who owns those rules, participating systems, supported representations, coexistence period and validation points. Distinguish the accepted source from generated clients, adapters, examples and running instances.

Test actual consumer tolerance. Adding an optional field can change a signature, canonical serialisation, exhaustive match, strict validator or size limit. Check defaults, absent and unknown values, ordering, duplicates, precision and errors. A rename needs translation that affected readers actually use. Where coexistence is promised, test old readers with new output and new readers with saved inputs. Report invalid input, unsupported versions and state mismatches separately.

For a breaking change, identify old and new versions, which producers emit them, which consumers read them and how negotiation or routing works. Establish remaining old-version use, retirement conditions and rollback or forward repair. Change the shared definition first and test generated output through its consumer. Put these comparisons and remaining checks in the contract review; rollout still needs its own verification and decision.

## Choose the check

| If you are doing this | Read this |
| --- | --- |
| Check names, locators, version pins or binding decisions. | [Keep contract identity and permissions intact](contracts/identity-and-binding.md) |
| Check resolution, generation or which validator checks ran. | [Check the definition and validation used by a consumer](contracts/definitions-and-validation.md) |
| Check semantic compatibility, intermediary writes or exports. | [Keep meaning and effects through translation](contracts/meaning-and-writes.md) |
| <a id="separate-tuning-controls-from-contract-changes"></a>Check protocol, inherited behavior or a tuning change. | [Compare actual behavior across contract profiles](contracts/profile-behavior.md#separate-tuning-controls-from-contract-changes) |

## Before handing off

- [ ] Name the source of the contract and the actual consumers it affects.
- [ ] Check old and new readers and writers at the relevant seam.
- [ ] Keep the coexistence, rollout and fallback decisions explicit.
