# Trust, storage and recovery design

Pale lets agents act in the world, so a restored backup or a second writer must never bring back a permission that was withdrawn. This maintainer specification describes the storage, writer fencing, trust records and recovery protocols that guard against it. Some contracts and adapters exist, while the complete resident, offline recovery commands and deployment qualification remain unfinished. Command examples in proposed procedures describe the interface the design proposes; they are not installation instructions or proof that the ordinary executable supports them.

The first deployment should use a lighter design for one machine. A separately administered etcd quorum, a Keychain signing helper and the offline trust ceremonies remain later proposals. Their existing adapters and local tests can inform that work without making them prerequisites for the first design. Hard effects remain disabled until the deployment can prove that restoring old state cannot restore permission to act.

The [source layout](../source-layout.md) identifies the implementation layers. The [resident host](../../interfaces/resident/README.md) describes current assembly. This specification keeps proposed guarantees separate from those entry points.

## An anchor must survive independently of a restored store

A hard-effect deployment compares its trust store and each consequential record domain with an independently retained minimum checkpoint. The anchor advances only after the database write is durable. It must support exact-predecessor compare-and-advance, durable read-back after an uncertain reply, and administration, credentials, backups and restore outside the trust store, control stores and signing-key restore domain. A local simulation or a cluster restored with the same machine cannot establish that independence.

The [anchor port](../../crates/pale-ports/src/anti_rollback.rs) defines transition and recovery checks; the [etcd adapter](../../adapters/etcd/README.md) implements that port. The [Darwin signing adapter](../../adapters/darwin-signing/README.md) describes the proposed helper. Qualifying it requires checking plaintext seed exposure in the helper, serialised Keychain access, bounded authorisation waits, exact dependency pins and features, advisories and any release-specific audit. No API guarantee or historical audit alone qualifies a deployment.

## Storage is chosen per class of data

Choose storage for the job: a cache, a crash-recovery log and a versioned owner ledger have different requirements even if one engine could hold all three.

For each data class, declare latency and tail budgets, visibility, tolerated loss, required history, its single writer and readers, growth and retention, platform and filesystem assumptions, and recovery. Before Pale relies on a storage component, its contract must also state how much spare capacity it needs and what happens when it slows down or fails.

Each person (a principal) has one trust store, and each deployment its own control store:

```text
<runtime-state-root>/pale/principals/<principal-id>/trust.sqlite3
<runtime-state-root>/pale/deployments/<workspace-id>/<deployment-binding-id>/control.sqlite3
```

The **trust store** holds the principal record, recovery policy and local administration; node delegations; the authority to create producer incarnations, which distinguish successive or independently restored writers, and the latest allocated incarnation records; and the active checkpoint signers.

A **control store** belongs to exactly one deployment. Deployments share no mutable control state and cannot issue conflicting trust checkpoints for it. The store retains that deployment's request, action and observation metadata and receipts, plus event references, subscriptions, deliveries, acknowledgements and cursors.

It also holds providers’ checkpoint state, updated only if its saved predecessor still matches (compare-and-swap); capability deployments and grants; session leases recording permission and limits, links matching calls to their results, revoked permissions and final outcomes; and references and hashes for the trust store. Federation spaces, invitations, memberships, grants, revocations and peer routes belong here, as do deployment lifecycle, generation, health, drift and recovery metadata.

Neither store holds another owner's tables or raw provider payloads: owner payloads stay in owners' stores, referenced by ID and hash, and no store holds a secret value. Both use SQLite in write-ahead logging (WAL) mode. In code they live in the [control-store adapter](../../adapters/control-store), with one initial migration each under `migrations/principal-trust` and `migrations/deployment-control`, pinned by the storage manifest [pale-control-storage.v1.json](../../contracts/manifests/pale-control-storage.v1.json). The manifest pins the ordered migration sets, schema identities and hashes, supported versions, required SQLite features and pragmas, backup profile and fixture roots. Each store records its kind, identity, incarnation, schema and migration head, so a deployment database can never be opened as a trust database.

Migrations run forward only: a verified backup, an immutable migration intent, each step applied once in order, invariants validated, and a migration receipt written before local inter-process communication (IPC) opens. A failure restores the backup into an inactive recovery location; there is never a down migration. An unknown, wrong-kind, corrupt or half-migrated store is quarantined until the owner runs an explicit recovery. The fixture set covers the empty initial store, a supported older store, the wrong kind, a newer store, a corrupt one, a partial journal and an invariant failure.

## One writer per person, with old writers blocked

One resident process is the only writer to the trust store and every control store it opens; the command line and adapters go through ports. Before opening a store the resident takes a principal-scoped operating-system lock and advances a durable writer fence, and holds both until every store is closed. The fence binds a generation, the process instance, the machine boot incarnation, the primary node, the trust-store incarnation, the deployment-store root and the session floor. A PID, a wall-clock time, a missing socket, SQLite's own locking or another process cannot displace it.

A clean stop publishes a terminal writer receipt. Taking over after an unclean stop is an authenticated ceremony that proves the predecessor is gone and names the exact store and anchor heads, retires its writer generation, raises the principal's session floor so every earlier session is invalid, invalidates the predecessor's routes and leases, quarantines its unfinished custody and issues a takeover receipt. If the predecessor's state is uncertain, the resident stays stopped. The fence and both receipts exist as contracts. Clean transitions have application and storage code. The operating-system lock exists in the control-store adapter but is not yet composed into resident startup. The code can check takeover records against their contract; it cannot yet carry out the takeover procedure.

## Proposed trust bootstrap for hard effects

The bootstrap and recovery procedures below are later design proposals. Existing contracts and isolated tests establish only their implemented parts. They do not select a deployment or make these command examples available in the ordinary CLI.

### Trust is created once, offline

With the resident stopped and the trust store empty, `pale trust bootstrap --offline` takes the principal key from the platform key store and, in one trust-store transaction, installs the principal record, recovery policy, the first local-administrator authority, the producer-incarnation authority, the checkpoint signer, a signed trust checkpoint, the first checkpoint of every hard domain and a bootstrap receipt binding the whole set. Only then does it advance the independent anchors, which is idempotently recoverable but not one physical transaction with the database. It starts no services, refuses to run once any trust head exists, and can resume only a complete committed set whose anchor advances are pending: "committed in the database, anchors pending" is the one recoverable partial state.

When Pale checks and accepts a deployment later, it creates that deployment’s control store, delegates it a store-bound checkpoint signer, advances the principal's signer-head root, and creates and anchors that store's first checkpoints before its IPC opens.

## Every later change is one signed command

Every administrative change is one signed local-administrator command carrying the exact ordered set of candidate records and the heads it expects, committed together with the new heads and an administration receipt, or not at all. The administrator's signature authorises submission only; each candidate is verified under its own issuer, so an administrator cannot sign a controller's acceptance of a deployment. An exact replay returns the existing receipt, and reusing the command's idempotency key with changed content is a conflict.

The commands cover principal-key rotation and recovery, the recovery policy, administrator and producer-incarnation authority, node delegation and its revocation, checkpoint-signer and deployment-runtime-authority rotation, owner record-signing authorities, producer incarnation allocation and retirement, local routes, the deployment proposal, acceptance, revocation and final binding, session grants and their revocation, execution fences and apply authorisations. The command line exposes them as `pale trust rotate-key`, `update-recovery-policy`, `delegate-node`, `revoke-node`, `rotate-checkpoint-signer` and the rest, as clients of the administration port. `recover-key` (the principal key is lost but the administrator survives) and `recover-key-and-admin` (both are lost) are different ceremonies and never fall back to each other.

The contract families for the administration command, the administration receipt and the deployment control-store genesis receipt are present in the code's contract bundle, with the bootstrap receipt, trust and control checkpoints, the writer fence and its receipts, and the principal recovery proof.

### Accepting a deployment takes three signed records

Pale accepts a deployment through three signed steps so that nobody accepts on someone else’s behalf. The compiler signs a binding proposal. The controller or relationship authority signs an acceptance of that exact proposal, with its own authority and evidence. Only then does a deployment binding reference both, under the same deployment terms digest, with accepted-at never before proposed-at and never after the proposal expires, and the binding never outliving its accepted validity. The deployment's contract set is sorted, duplicate-free and canonical. A revocation of the acceptance, signed by the same class of authority, blocks new grants and requests while keeping history. Finalising a deployment is two ordered, receipt-bound transactions: the binding, its checkpoint signer, its runtime authority and a trust checkpoint first; then the separate control store's genesis, its receipt and its anchors.

### Sessions, runtime records and owner signers

A capability session grant is short-lived and non-transferable. It binds the caller's key, the deployment binding, a capability allowlist, route constraints, scope ceilings, the current session floor, the trust checkpoint and the deployment's revocation epoch; its floor must equal the active floor, and an empty capability, route, purpose or authority set never means unbounded. The first grant co-commits an empty revocation checkpoint, and every revocation after that is append-only and anchored. The same peer Unix user is supplementary evidence at most.

Every independently signed Pale-native deployment-runtime evidence record carries a reference to that deployment's runtime authority; a checkpoint signer cannot issue one. Unsigned structural state and records signed by another defined owner keep their own authority rules. Owners keep their own private keys: an owner record-signing authority is signed by the principal key, needs a fresh proof of possession at its first epoch and on every rotation, numbers its records within each epoch, and publishes a contiguous frontier that is bound into the next trust checkpoint, so a record backdated under a retired key is rejected. Pale verifies owner-signed records and cannot mint them.

Every signed record follows one non-recursive digest rule: omit the record's own digest and signature, canonicalise the rest with RFC 8785, hash with SHA-256, insert the digest, and sign the domain separator followed by the digest with Ed25519.

## Each consequential change saves its checkpoint in the same transaction

For each group of consequential records, the hard frontier is the latest signed checkpoint that recovery must preserve. Every transaction that creates or changes authority, consumes a nonce or authorisation, records request or action idempotency, advances a route, fence or generation, reserves or dispatches an apply or effect, or takes, changes or releases responsibility for unfinished work or records commits the next signed control-state checkpoint in the same database transaction. Replayable state (delivery, acknowledgement, cursors, health) may batch in its own domains. Recovery replays a lost suffix from the durable source for that stream, and status and doctor show the lag.

Each checkpoint updates a verifiable summary of one group of related records, called a domain accumulator. It names its predecessor, raises the domain sequence by one and carries the state root (a hash summarising the records), their leaf count, a separate root for unfinished work still held, and a size-limited set of changed leaves. The control-store adapter uses a pinned sparse Merkle tree, so an update costs logarithmic time in the domain's size rather than rehashing the whole store.

Keep separate hard domains for at least principal trust, producer incarnations, deployment, routes and federation state, and each managed resource's apply, effect and reconciliation. Conflicting records share a domain; unrelated scopes never serialise behind one another.

Benchmarks still have to set the numeric budgets for writers, domains, leaves, mutation rates, anchor latency and replay lag. They must measure contention within one domain, parallel domains and logarithmic growth.

The trust checkpoint is the person’s signed minimum state that recovery must preserve. It covers the principal record, recovery policy, administrator and producer authorities, producer allocation heads, the session floor, every checkpoint-signer head, deployment runtime-authority heads, owner-signer heads, node delegations and their revocations. The session floor starts at one and rises once with every offline administrator or control-store recovery. Revoking a node delegation, signed by the principal key, permanently prevents that node from allowing new routes or requests, and works without federation.

## Time proves only what its source can support

Ordering and expiry need different evidence. Predecessor revisions order one history. A monotonic process clock measures duration only within that process; neither it nor a local journal sequence proves restart-safe expiry or ordering across independent systems.

Every lease, route-freshness check, issuance, retention and recovery path declares its physical time source and trust assumptions. Preserve backward wall-clock movement where the record format allows it. Otherwise report the anomaly and withhold any transition whose signed lower time bound cannot be established. Do not hide it by clamping the clock.

Unrelated pure work continues, and there is no inferred global signer-time frontier. These are qualification requirements, not evidence that the current runtime enforces them.

## Cold start classifies before it acts

When the resident starts it classifies what it finds and does only the one thing that classification allows:

| What startup finds | What Pale may do |
| --- | --- |
| No trust heads at all | Offline bootstrap may begin |
| Store ahead of its anchors by exactly the advances a committed receipt names | Complete those anchor advances, then start |
| Store and anchors agree | Start normally |
| A deployment Pale accepted whose initial records were never fully created | Finish only those initial records, keeping the deployment inactive |
| An inactive recovered control store with no trust-store receipt | Resume that recovery only |
| A trust-store recovery receipt whose anchors lag | Resume the anchor advances only |
| A missing, corrupt or rolled-back trust store, with a recovery capsule and threshold authorisation | Recover the trust store into fresh incarnations |
| An anchor ahead of the store, a checkpoint with the same sequence and a different hash, a broken accumulator, an unknown profile or signer, a missing domain or a partial set | Fail closed into offline owner recovery; bootstrap never reruns |

## Four offline recoveries, each one-use

Each recovery runs with the resident stopped, before IPC opens, consumes a one-use authorisation and advances the hard anti-rollback frontier. A recovery quorum authorises; it never impersonates the principal key, and it signs an operation-specific digest under its own domain separator.

1. **Lost administrator.** A root- or quorum-signed authorisation plus a replacement administrator signed by the active principal key. The session floor rises, so every earlier session dies, even without a revocation record.
2. **Lost principal key and administrator.** A principal recovery proof lets a new key sign a key transition, the successor principal record and a replacement administrator, binding the predecessor checkpoint, the next session floor, a nonce and an expiry, with its own receipt.
3. **Lost or damaged control store.** The logical store ID is kept and a new incarnation created through an ordered two-store protocol rather than a pretend transaction across two SQLite files: rebuild an inactive store and its genesis checkpoints; then, in the trust store, retire the old store-bound signer, install its replacement, raise the session floor, advance the signer-head root and write a successor trust checkpoint with a recovery receipt; then advance the anchors and permit startup. Any head that cannot be reconstructed exactly, and any unresolved custody, stays quarantined.
4. **Lost trust store.** A recovery capsule placed in advance, outside the trust store, control stores, machine backups and the active key's failure domains, binds the principal's identity, last accepted trust minimum, recovery-policy head, a reference to the root key's recovery material, the classes that can be reconstructed and a one-use namespace, and never holds a plaintext private key. With a threshold-signed authorisation it creates fresh trust and control incarnations. A copied backup is evidence input, never authority.

Only the principal recovery proof exists as a contract; the other recovery records and the capsule have no schema and no code yet.

## A second machine is a companion, not a writer

Each person has one primary resident writer. Another machine gets a signed, expiring node delegation and a local route binding tied to both machines' delegations and the trust checkpoint. The connection requires mutual node authentication, encryption, endpoint binding and replay protection. The companion never opens or replicates the primary's stores and retains only bounded node-local evidence.

While the primary is unreachable, the companion may read and queue intents in the relevant owner's store. It may not execute a new Pale command, allocate authority or a producer ID, dispatch an effect, accept work or create a trust frontier.

On reconnection, it rechecks the trust checkpoint, revocation head, route and generation, native frontiers and authority. It receives a selective owner digest and revalidates every queued intent against the restored frontier. Multiple writers, partition-tolerant execution and failover writers are later possibilities, not part of this design.

## The supervisor keeps the resident alive, not the components

The [current read-only doctor](../doctor.md) uses existing local inspections without a supervisor or administration-port connection. It reports missing deployment declarations and unimplemented resident diagnosis as unavailable and creates no roots. The supervisor, startup and recovery behavior below remains the target design.

The operating-system service manager, launchd or systemd, keeps one foreground `pale supervise` process alive. It never double-forks or keeps a PID file. The commands `up`, `down`, `status`, `doctor`, `validate`, `plan`, `apply` and `recover` are clients of its administration port.

The supervisor owns desired service state and dependency order; process generations and fences; readiness and health; restart backoff and crash-loop detection; and adoption of exactly matching children after restart. It also owns operating-system schedule files, drift and recovery evidence, and the stop sentinel that prevents a relaunch after `down`. It reports ready only after recovery, the active generation has been checked and accepted, and IPC is available.

Readiness belongs to one operation and generation. It binds the exact provider instance, capability or operation, native revision and configuration, actor, authority and effect ceiling, dependencies and proof frontier. A healthy process or one proved capability cannot make another operation, or the whole product, ready.

`pale up` compiles the deployment's dependency graph, sets desired state and waits for readiness gates in order. It names the failed gate and leaves ready services running unless the declaration requires atomic rollback. `pale down` stops in reverse order, first gracefully and then by process group after a timeout, and sets the stop sentinel.

`pale status` reports declared and desired state, process, readiness, generation and fence, drift, backlogs, partitions and the last recovery. Those observations imply no domain truth.

Project-local harness integrations, the resident supervisor, Gas Town's workers and each application keep separate lifecycles. Gas Town runs its own Dolt store, Scope owns its semantic state while Pale supervises its process, and Weave owns its records. An interface may request a supervisor action. The supervisor reports Scope availability, and the checks before launch apply the selected handoff’s receipt requirements. If Pale itself dies, launchd or systemd relaunches it and it adopts only exactly matching children.

`pale doctor` checks platform and toolchains; the distribution release, lock, hashes, notices and the agreement of the contract set (raw schema hashes, descriptor manifest, generated descriptor table, offline reference closure, schema-set digest); that the core profile never loads MONAD; the packet Gas Town publishes for itself and its Beads and Dolt internals; and the workspace and deployment. It rejects floating, `@latest`, wrong, dirty, moved or missing references, creates only declared runtime roots, delegates owner migrations and credential provisioning to owner adapters, and reports without improvising repairs. Build, doctor, install and activation fail closed on a duplicate schema ID, an unresolved or network reference, a mismatch, a wrong set digest or historical bytes under a current ID, and never regenerate the manifest at the trust boundary. Pale is the only component that writes operating-system schedule files; each component ships callable operations and retry, and the person's workspace declares the cadence.

The application exposes a [resident frame service](../../crates/pale-application/src/resident/frame_service.rs) for bounded requests and recovery turns. [Service assembly](../../crates/pale-application/src/resident/process_host/service_assembly.rs) joins the deployment’s held writer guard to the checked writer record, current trust checkpoint, opened stores and permissions for each operation. The process adapter supplies supervision, probe and termination mechanics behind `SupervisedProcessPort`. The deployable supervisor still has to assemble those owners, manage the listener and publish readiness. The [interface guide](../interfaces.md) describes the current client boundary.
