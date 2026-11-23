# Compare actual behavior across contract profiles

Use these checks for protocol adapters, inherited behaviour and operator settings. Keep reproducible cases in the relevant tests. Report tested versions, inputs, actual results and missing coverage in the review reply.

## Check each protocol projection against the same operation

Identify who defines the operation and which version is under review. For each protocol and deployment profile, distinguish what the wire format can express, what the profile enables, what a particular caller can discover and what they may invoke. Discovery and invocation need separate tests. Responses must respect visibility rules. A restricted internal trace may distinguish refusal causes the caller is not allowed to infer.

For operations with several routes, compare normalised requests and outcomes at the component that owns the operation's meaning. Include identity, permissions, input, revision, meaningful errors, uncertainty and reconciliation. Keep protocol-specific acknowledgements, artifacts and cancellation separate. If an adapter cannot preserve a required field or outcome, exclude that operation or define a distinct operation with its own approved contract.

Run the same case through two enabled routes against the same expected result at the owning system. Test unsupported, disabled, concealed and unauthorised cases separately where applicable. Include an unknown outcome and a route that would lose that distinction. Compare caller-visible observations with the owner's actual effects and stored results. Report routes, profiles and outcomes covered; parity proves only the cases and systems exercised.

## Check inherited behavior and the added capability separately

Identify the accepted upstream candidate, inherited contract and suite, adapted implementation, consumer and versioned compatibility mappings. Run the inherited suite against that adaptation. If translating a test, account for every changed assertion and observation point. Use a known-bad case to show the mapping still detects violation of the original requirement. An omitted test remains a gap.

For the added capability, specify new behaviour, a control lacking it, what would disprove the claim and conditions under which it should hold. Declare cases, thresholds, repetitions and uncertainty before running where relevant. Keep suite identities, inputs, environment, commands and raw observations in the existing test results. The inherited suite proves only what it exercises; a new suite cannot compensate for inherited regressions.

Report each suite separately, including unchanged or unsupported claims, tied to the tested versions. Reject evidence from another candidate or an aggregate pass hiding failed or missing requirements. The owner's acceptance, attestation and release decisions remain separate from test results.

## Separate tuning controls from contract changes

Classify a setting by the meanings and effects it changes, not whether it is numeric or stored in an environment variable. The owner defines the permitted tuning range. Changes to input/output shapes, required instructions, acceptance or permissions still need a contract decision. Document parsing, allowed values, omitted versus explicit zero, defaults, invalid-input handling and effective configuration revision.

Trace when the consumer reads a value: build time, startup, per request or explicit refresh. Reading the process environment per request does not prove an upstream configuration source can update that process. Test the real refresh or recreation route and inspect the effective value. Rebuild browser-visible constants when their input is fixed at build time.

For deadlines, distinguish total work time, idle time, transport limits and how long the client waits. If the server must return a structured timeout before the caller gives up, test that ordering with delivery margin and intervening proxy limits. Include a long but active response and a stalled one. Simply setting the client timer higher than the server's does not reconcile different timer meanings. Keep configured limits, observed cancellation and remote outcomes separate.

## Worked example

Two transports advertise the same operation. Compare accepted inputs, denied cases and downstream results. Check inherited behaviour and the new transport's added capability separately. Change a retry timeout as a tuning exercise and observe its effect separately from any changed operation contract.
