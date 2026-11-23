+++
description = "Compare discovery, permissions and actual outcomes for one operation across protocols."
see_also = [{ scope = "local", resource_id = "contracts/profiles" }]

[guide]
+++

# Compare protocols against the same operation

<a id="check"></a>

Identify who defines the operation and which version is under review. For each protocol and deployment profile, distinguish what the wire format can express, what the profile enables, what a particular caller can discover and what they may invoke. Discovery and invocation need separate tests. Responses must respect visibility rules. A restricted internal trace may distinguish refusal causes the caller is not allowed to infer.

For operations with several routes, compare normalised requests and outcomes at the component that owns the operation's meaning. Include identity, permissions, input, revision, meaningful errors, uncertainty and reconciliation. Keep protocol-specific acknowledgements, artifacts and cancellation separate. If an adapter cannot preserve a required field or outcome, exclude that operation or define a distinct operation with its own approved contract.

Run the same case through two enabled routes against the same expected result at the owning system. Test unsupported, disabled, concealed and unauthorised cases separately where applicable. Include an unknown outcome and a route that would lose that distinction. Compare caller-visible observations with the owner's actual effects and stored results. Report routes, profiles and outcomes covered; parity proves only the cases and systems exercised.

## Before handing off

- [ ] Name the shared operation and what each protocol promises.
- [ ] Compare valid requests and failures at each actual entrypoint.
- [ ] Check that translated failures preserve what callers need to know.
