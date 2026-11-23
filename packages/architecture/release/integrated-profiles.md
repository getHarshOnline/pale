# Check standalone and integrated operation

Keep repeatable cases in the relevant integration tests. In the release review, report each profile, version pair, observed outcome and remaining permission or adoption decision separately.

## Check standalone and integrated profiles separately

For every claimed operating profile, distinguish a published interface, configured and permitted provider binding, and observed end-to-end behaviour. Test standalone operation from a clean installation without optional providers and inspect required results. For each integration, identify both owners, contract and provider versions, permissions and the path from caller to actual effect. Test a missing or rejected binding and the consumer's promised refusal or degradation.

Say which capabilities each observation establishes. A generally optional provider can be mandatory for one profile. Its absence leaves that profile unverified while separately tested standalone behaviour remains established. Publishing a contract does not prove integration works. Never silently activate another route as fallback.

### Check independent invocation and source paths

Trace each supported direction separately: initiating owner, adapter and translation, receiving interface, and who permits the action, stores state and verifies results. Publishing an occurrence and sending a command in the reverse direction are separate contracts even on one transport. An acknowledgement can establish delivery within its scope; it cannot authorise a reverse call or prove another system's effect. Every extra hop needs its own permitted capability.

Consider caller mode, input origin and intended effect independently. Choosing a caller must not silently select a source, and permission to read a source does not permit downstream action. For each supported combination, identify selector, workspace or instance, owner, contract and source revision. Mark unsupported and unresolved combinations explicitly. Caller adapters and provider interfaces stay with their respective owners.

Inspect requested versus actual routes, source reads and effects separately. Revoke or remove one caller, source or dependency at a time and retest an unaffected path. Include an unavailable selected source with a plausible local value and a valid occurrence incorrectly offered as permission for a reverse command. Require refusal or the declared degraded path before protected reads or effects, without substitution or extra calls. Report results by combination. A synthetic matrix or local-only run leaves unexercised integrations open.

### Check a compatible pair across independent activations

Identify each owner, deployment, immutable release and activation. Compatibility results must name the version pair, contracts, required capabilities, scope and allowed degradation. Read release, compatibility and activation facts from their respective owners. A successful handshake establishes compatibility for that pair; each owner still decides and verifies its own activation.

Before dispatch, permit the request against one current, authorised pair. Keep that binding through completion and verify the response belongs to the same request and pair. Either owner may activate a successor during the request. Existing pinned work may finish only while permissions and required data remain valid; a new head cannot silently replace half the pair. If the pair is unavailable, refuse or use a separately tested fallback allowed by both policy and request.

Test both activation orders, reuse of an old handshake for a new pair, activation between acceptance and reply, and revocation or outage during a request. Include a reply from the wrong deployment or generation. Inspect each owner's active state, the pair used by each request, permitted completion on an old pair and continued availability of independently tested standalone operation after integration failure.

### Locate the scope of a required authority

For each required permission or authority service, identify the smallest deployment, transaction or optional role that depends on it. State issuer, validity, check point, protected effects and how to inspect the result. Starting or recovering a deployment, performing one transaction and enabling an optional role may depend on the same provider but impose different requirements. A reachable service or healthy sibling role proves none of them by itself.

Withhold or revoke one binding at a time. A deployment prerequisite must block readiness and protected work for that deployment. A transaction prerequisite must block its effect while an unrelated permitted transaction can still complete. A missing optional role must be shown as unavailable while unaffected standalone and sibling work continues. No other writer or capability inherits its powers automatically; a replacement needs its own permitted binding and tests.

Test permission loss before acceptance and during a request, following the owner's in-flight-work rules. Reconcile uncertain completion before replay. Inspect readiness, stored state, unaffected operations and unfinished work separately. Restore permission through its owner and confirm stale evidence cannot reopen an earlier blocked path. Report the blocked unit, preserved outcomes and observations needed for recovery.

### Exercise provider loss and check retained output eligibility

Test the provider-free profile from clean installation, restart and recovery, without the provider or derived state. Exercise required results through the consumer itself. Separately test the integrated profile and calls it requires. A request needing an unavailable call must remain blocked. Degradation requires permission from the current owner policy and request. Distinguish invocation failure, rejected output, refusal and permitted degradation.

Keeping an old output differs from permission to use it now. At use time, check provider or producing profile, source revision, purpose, permission and expiry under the consumer's rules. Test outage, profile retirement, changed source, expiry and revocation separately. Reopen a cache or restore a snapshot and verify old derived outputs still qualify before use. An old output can establish its origin; it cannot satisfy a required current invocation or restore lost rights.

Test the host-free baseline, integrated success, permitted degradation and refusal of a mandatory route. Inspect actual provider calls, selected outputs and historical records or stored state that transitions must preserve. Exclude ineligible outputs before they influence selection on degraded paths. Other services keep their own permissions and evidence. The consumer and source owners decide which outputs remain usable, require renewed acceptance or may only be retained.

#### Check loss of one role within a provider

For a provider supplying several roles, identify each role's binding, permitted effects, dependent consumers and supporting evidence version. Say which release or operation needs it. A healthy provider or sibling binding does not establish that this role is selected, ready or permitted.

Keep provider and siblings unchanged while disabling one binding or expiring its evidence. Test a dependent consumer and another using only a sibling. Require refusal or pending status before the affected effect, success through the unchanged sibling and continued standalone behaviour. Inspect actual route selection and ensure no sibling call substituted for the missing role. Separately test a provider-wide outage and restoration under a new binding.

Keep earlier outcome records tied to their original role and evidence, rechecking use eligibility. A new binding cannot adopt an old outcome merely by sharing the provider's identity. Inspect saved records and each owner's state after transitions. Role owners decide restoration, expiry, retention and required downstream evidence.

### Separate provider qualification from consumer adoption

A provider's test and acceptance record must identify its artifact, build inputs, consumer scope, deployment or use profile, tested contract, cases, limits, refusals, recovery and accepting authority. Preserve it by immutable identity. Consumer adoption needs a separate decision referring to that record, consumer contract, selected scope, allowed uses, conditions, expiry and supersession. Installation and activation still require their own observations.

Verify the test-record signer and consumer's current authority separately. Before use, compare artifact, consumer and profile with both records. Test a valid record without adoption, one for another consumer or profile, changed runtime bytes and expired or superseded adoption. Test retirement and a successor with both records pinned. If only one active head is allowed, race successors and verify the winning result while preserving predecessor evidence.

Report what tests establish, what the consumer adopted, what deployment serves and what remains unresolved. Exact schemas, signatures, transitions and actions belong to the participating systems.

## Worked example

A consumer uses calculation and artifact-serving roles from one provider. Test standalone operation, then the compatible pair. Disable one role and check dependent work and old outputs separately. Inspect the consumer's active binding before treating provider support as actual adoption.
