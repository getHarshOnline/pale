# Workspace language and generation contracts

A person describes their world once in the workspace language, and every Pale tool reads that one description. This maintainer specification sets out the full language and the cross-owner apply and activation protocol. The implementation compiles local workspace and agent-guidance inputs; it does not implement the complete language or ordinary resident activation. Imperative statements below specify the target contract unless their paragraph identifies an implemented component. The [workspace manual](../workspace.md) owns commands available to a user today, and the [agent-surface specification](agent-surfaces.md) covers the implemented repository-file slice and unfinished host integration.

## Current module-admission limit

The [local source loader](../../crates/pale-application/src/workspace/source.rs) reads declared TOML module files. [Module sealing](../../crates/pale-core/src/records/workspace_language/module.rs) checks structure and canonical content digests, not publisher signatures. The linker checks matching release, digest and publisher coordinates. [Universe admission](../../crates/pale-application/src/workspace/universe/admission.rs) requires a published release and publisher evidence for every adopted module, plus separate evidence for protected delivery.

The production compile path supplies no external proofs; only test code can construct them. The local workspace and guidance route can therefore compile when it needs no adopted-module or protected-delivery proof, while those imports remain blocked. Existing tests distinguish an empty adopter that needs no external proof, a published module missing proof, and candidate or revoked releases even with fixture proof. This gap does not create an unsigned-module exemption, establish production signing or activate a runtime.

## A workspace is a graph of modules

A module may stand for a principal, a household (only the consequences the household genuinely shares; personal facts stay in personal modules), an organisation, a legal entity, a relationship, a vertical, a brand, a service contract, an application binding, a resource, a policy, a machine or a deployment. Every authored scope first becomes a content-addressed source component for provenance and impact analysis, but a separate module is justified only by independent authority, privacy, reuse, adoption or evolution, never by folder layout. The root carries whatever display name its owner chooses, and folders inside it are navigation from that owner's point of view: they create no identity, membership, controller status or access.

Each declaration has a single home, reached by imports, protected surfaces and typed overlays. Copying a fact between folders would create two competing authorities, so reuse is exact import plus typed extension or narrowing, and it never inherits someone's identity, membership, authority or private facts. An appearance in Weave or Buzz alone does not create a Pale principal root or writer; these require explicit constitution. Shared truth between two workspaces is contract-owned assertions, authenticated acceptances and authorised projections, never a global shared database: each workspace keeps its interior and its journal.

The root manifest names the workspace ID, the registries of principals, organisations, controllers, relationships, projects, execution workspaces and repositories, the three relationship roots between them, the machine and provider binding roots, processing roles, environments, deployments, capabilities, policies and the distribution lock, plus its display name, root composition identity, confidentiality-policy root, the allowed module and import source roots and the generated-output roots. Nothing is discovered by convention, guessing or ambient search. A typical layout is a projection of the module graph:

```text
pale-workspace.toml
distribution/release.lock
principals/  organizations/  controllers/  relationships/
projects/  execution-workspaces/  repositories/
processing-roles/  environments/  deployments/
capabilities/{catalog,policies,fixtures}
packages/{custom,bindings}  packs/bindings  apps/bindings  corpora/bindings
contracts/{dependencies,generated}
schedules/  secrets/references.yaml  policies/
provider-adapters/{sources,fixtures,bindings,tests}
plugins/{custom,bindings,fixtures}
gastown/desired
bindings/{machines,providers}
```

Names change only through the schema, and ownership classes never collapse into one folder; a module-oriented layout may compile to the same manifests.

Modules are written directly in TOML or produced by confined builders in Rust, Python or TypeScript. There is no custom syntax. A builder runs only as a declared build step over declared immutable inputs with declared capabilities, cannot inspect files, environment, network or credentials beyond them, holds no signing key and cannot publish; its output is the trust boundary. Both routes emit the same canonical candidate manifest, JSON in RFC 8785 form with a versioned schema, and signing by an authorised publisher is a separate act. Runtime contracts use Protobuf, never the portable manifests. The [interface guide](../interfaces.md) describes the language server for authoring.

## Identities stay separate and are joined only explicitly

A principal is a person with their own root, lock, writer and activation. An organisation is an optional, externally constituted, versioned and attributable coordination scope for bounded acts, and its absence is valid. A controller is the legal, personal or business authority over data and effects. A relationship is an accepted, bounded relationship under a controller. A node is a runtime endpoint delegated by a principal, a caller is a human, agent, schedule or service acting under a role, and a worker is local to an executor. Beside these sit stable roles, occupants, native memberships and accounts, communication-actor handles, software agents, responsibility providers and their instances, model and runtime profiles, native harnesses and their sessions, native-presence attestations, deployment bindings and activation generations.

They are joined only by explicit, versioned, directional mappings. Replacing a provider leaves the stable role and native history unchanged, and matching display names or email domains never imply equality. Relationship-mediated action is directional: it names the represented principal, the representative, the act, the resource, the purpose, the time, the source authority and its revocation.

A deployment binds the workspace, principal scope, controller scope, optional relationship scope, environment, deployment ID, distribution release, processing role (controller, processor, joint or independent controller, or not applicable for personal use), confidentiality policy, data classification (which may only narrow) and permitted purposes. An environment carries controller-neutral ceilings on network hosts, data classes, effect risk, resources, retention and optional adapters, and never selects a controller, relationship, credential or runtime identity. A multi-scope workspace never compiles into one multi-controller mutable runtime, and organisation, controller or relationship is never inferred from a repository, directory, email domain, company name, workspace, community or label.

## Releases, visibility and three locks

A module is published as a signed release. The publisher has a stable authority descriptor, anchored in a recovery root, that delegates to rotating release keys; a Git author or repository owner alone is not enduring authority. A release can be fetched from a local path, an exact Git commit and subpath, a sealed archive, a registry or a peer mirror, but these are only carriers: the import binds the stable module ID, the exact release ID, the content digest, the authority descriptor, the export surface and the rights, effective-date and revocation coordinates. A branch, path, file name or URL never identifies a release, and a Git path or sparse checkout is not a confidentiality boundary; a genuinely private owner publishes a separately access-controlled signed surface.

Each module exposes `public`, `protected` or `private` surfaces, with an explicit `prevent` to stop onward propagation. Source visibility, source delivery, consumer display, propagation, republication, resolution mode and release, adoption and failure policy are independent axes. Authorised declaration surfaces, and the imports behind them, propagate transitively until `prevent`, `private`, a rights or controller ceiling or a downstream narrowing stops them. `protected` lets the author either grant permitted source or expose a signed, revision-bound resolver that answers on the owner's side without revealing private source. Consumers may narrow disclosure, effect, model-use, retention, rights or authority ceilings but never widen them. Authority, membership, controller status, resource grants, rights, model-use permission and native truth are never transitive, even when the declarations that mention them are. Resolving an upstream release through the import closure does not make the importer its publisher. `prevent` stops propagation from that point on; it does not revoke an importer's already admitted use or erase lawful history.

Successor policy is set on each import edge. The safe default is manual adoption; an exact class of change may be preauthorised, a publisher may require a stop, revocation or handshake, and an importer may narrow but never weaken the publisher's ceiling. A new release only makes affected importers aware that a dependency may be stale, and reverse-dependency evidence marks only the affected region stale. Fetching, adopting, compiling and activating the successor remain four separate local decisions, and a rejected candidate never damages the active generation.

Three locks are kept apart, and no generic "lock" may stand in for them:

- the **distribution lock** pins the exact Pale distribution release and its dependencies;
- the **universe lock** records the exact accepted closure of modules and imports in this workspace;
- the **activation fence** is the runtime's monotonic ordering of which generation is live, so a stale deployment can never adopt.

The principal root also chooses how rebuilds happen: eagerly, bound to relevance (which needs an authorised surface fingerprint and a derivation witness, with the source author choosing whether the comparison runs on the owner's side), or lazily on first use.

## Types, policies and cycles

The kernel keeps a minimal structural meta-ontology and consumes a pinned standard-ontology module; modules add their own namespaced types, traits and attributes, and two things with the same display name never merge. Customising a definition is typed: `extend` may add declared fields or traits and `narrow` may tighten policy, while weakening a restriction or changing identity or authority needs an explicit bridge or a new definition. When policies meet, ceilings intersect, obligations accumulate and conflicting exclusive authorities fail compilation. "Most restrictive wins" alone would lose obligations, which is why it is not the rule.

Cycles are allowed only where they are safe. A finite, monotone, typed strongly connected set of declarations (reciprocal relationships, for instance) compiles to a deterministic fixed point; negation through the cycle, native effects, arbitrary callbacks, non-monotone policy, unbounded generation or a missing convergence proof fails. This is one of three kinds of cycle, never confused: declarative cycles resolved at compile time, runtime call or effect recursion, which is bounded and rejected when cyclic, and the long human loops in which people revisit work, which are simply new occurrences over time. A static contradiction fails compilation; a "pending owner" state exists only at a declared live approval frontier, never as an escape from malformed configuration.

## The compiler runs in fixed stages

```text
TOML or confined builder output
  -> canonical candidate module manifests
  -> validation and authorised publisher signing
  -> source-module syntax tree
  -> exact release, import, export and authority resolution
  -> linked syntax tree
  -> namespaced extension, policy and monotone-cycle resolution
  -> platform-neutral normalised semantic IR
  -> principal-specific packed generation plus native and deployment plans
```

Every fact admitted into a generation, and every consequence Pale plans, carries a derivation receipt tracing it from source span, module, release, import and policy through the IR, together with proof of what it affects and of the regions it deliberately leaves untouched. A final allow or deny, or a path through a graph, is not an explanation. Another owner's facts, and Scope's semantic provenance, are referenced rather than reconstructed. Only managed bindings propose native operations; observed and native-only bindings never write.

The generation's content identity binds the distribution release, the workspace and profile, the machine instance and runtime, the package closure, normative packs, model occupant and application bindings, catalogues and contracts, the repository, participants, delegations and leaf, rights, the materialised closure, and the descriptor-manifest and schema-set digests. Small indexes (lifecycle, run policy, operation lookup, admission, path and mode selection) are built eagerly; immutable bodies load at most once per generation. The [architecture guide](../architecture.md) describes the engine layers.

## Validate, plan, apply and status

The operator sees four commands:

- **validate** is read-only: parsing, schemas, ownership, references, paths, secret patterns, cross-scope rules, source pins and policy;
- **plan** produces a deterministic, reviewable plan of desired against observed state, with exact input hashes, operations, authorities, risks, rollback and a plan hash (its ID is the SHA-256 of the canonical plan without the ID);
- **apply** requires that exact, unexpired plan hash and the applicable approval, then runs one durable apply through the owners' ports and writes a receipt per resource;
- **status** compares declared, compiled, deployed and observed state and reports drift or unknowns without inventing meaning.

Each command re-observes its sources. Every managed resource is identified by a seven-part key, which is also the unit of concurrency:

```text
(workspace, principal scope, controller scope, relationship scope or none,
 environment, owner, desired id)
```

Plans touching the same keys run one after another; unrelated scopes, environments or resources may run in parallel only when their owner adapters declare no shared runtime resource. Plan, authorisation, run, intent and receipt all carry the same route binding and contract set, so a changed route generation needs a new plan, and a stale or differently hashed plan is rejected. In code, `pale` takes the root `pale-workspace.toml` and has `validate`, `compile`, `plan`, `apply`, `surface-status` (drift of the generated agent surfaces only), `summary` and a read-only review of a candidate workspace.

The [schema tests](../../crates/pale-contract-codegen/tests/generator.rs) check the generated bundle against typed producers and selected admission rules. The [CLI mutation tests](../../interfaces/cli/tests/mutation.rs) check that incomplete apply plan sets or altered removal plans leave both fixture repositories untouched. They exercise the agent-surface slice; full generation and cross-owner apply remain separate implementation work.

## Apply never loses track of an effect

Apply is transactional. One apply authorisation covers the complete plan only: its authorised operations and conflict set must equal those derived from the plan, and consuming it atomically creates one apply run together with a lease on every key the run will touch. Presenting the same authorisation again resolves the existing run; reusing its nonce with a different plan fails.

For each operation the journal first commits a dispatch intent, binding the authorisation, deployment, key, input, expected owner generation, lease generation and fence, destination, route generation, contract sets and a preallocated reconciliation case. Committing the intent does not permit the call. Immediately before invoking the owner, the resident writer moves the journal entry from committed to dispatched by compare-and-swap, together with the next control-state checkpoint and anchor advance, and only then do bytes leave. Each operation closes with an owner receipt, independent proof that nothing was dispatched, or an ambiguous result that opens the reserved case. Logs can never satisfy any of these.

If the process dies after dispatch and before a receipt, the operation is ambiguous. Recovery opens the reconciliation case and, in the same step, a non-expiring custody claim on the full resource key: the lease may lapse or pass to another worker, but no future plan can acquire a lease or dispatch against that key until the case reaches a terminal result. Recovery is state-driven: an operation still at intent may be dispatched; a dispatched one with no receipt needs an independent read before any retry; an owner that proves atomic idempotency may return its original receipt; a case reconciled as not applied may be redispatched under the unchanged plan, scope, lease, fence and precondition; one reconciled as confirmed advances without redispatch. A final apply receipt is written only when every operation is terminal, in plan order and each exactly once.

Owner idempotency keys are derived from the stable logical effect (owner, schema, resource, precondition and input), never from the plan or operation record, so equivalent effects in two plans get the same key and an intentional later effect binds its changed precondition or input. A lease renewal keeps the fence; releasing and reacquiring issues a new lease generation and fence token, and an old fence cannot dispatch, acknowledge or commit. A signed stop-dispatch control is one-way, a consumed run may resume after its authorisation expires only through an admitted operator capability and within its maximum run time, and neither resume nor abandon can add operations, change input, replace leases or authorise rollback. The reconciliation-custody records exist in the code; the apply authorisation, run, lease set and journal records are still design.

**Omission never deletes.** Renaming, orphaning, retiring, adopting and destructive deletion are separate planned actions, so taking something out of the workspace never removes a native object. A managed-resource reference records its scopes, environment, owner, desired ID, observing node and route generation, workspace commit, desired-configuration hash, an opaque runtime-only native reference, the observed native version, adoption proof, generation, lifecycle state and last plan and receipt. The same rule holds for plugins, where omission is not uninstall, and for the Weave instances a workspace may declare, where absent configuration never implies deletion.

## Projects, execution workspaces and repositories

One canonical desired graph keeps four identities independent. A principal workspace is the configuration root. A project is a durable body of work. An execution workspace is a bounded scope in which chosen work, tools and repositories are made available for one purpose. A repository is one canonical source identity, independent of where it is checked out. Project-to-repository, project-to-execution-workspace and execution-workspace-to-repository relations are each many-to-many, and each exists only as a typed edge with a role, scope, provenance and lifecycle. A repository is declared once and referenced wherever it is used; a matching name, a folder, a checkout, a provider container or current work never creates an edge.

Two kinds of projection bind that graph to the real world without becoming it. A machine-checkout projection binds a repository to one machine, an exact revision and a local checkout or worktree path, so moving a checkout changes only the projection. A provider-native projection binds a desired identity to one opaque identity inside a provider, with its readiness and read-back, and never lets that provider's mutable state flow back into the workspace. A Gas Town rig is such a projection of exactly one repository. The workspace schema in code carries this graph and its three membership relations.

Gas Town's desired configuration names one selected provider instance, the canonical repository and, after the owner publishes and reads it back, the opaque rig projection, plus executor profiles with exact provider, adapter, sandbox, approval, filesystem, network, tool and capability ceilings, Package references, generation and fence policies and expected hashes. It never nests Gas Town under a project or organisation, and it never holds bead status, assignees, claims, worktrees, convoys, SQL credentials or an agent-selectable bypass profile. Safe executor profiles are the default; a relaxed profile is a distinct, versioned, explicitly authorised resource named in the handoff and preserved in the outcome, and an upstream flag that skips permissions authorises nothing.

## Secrets, credentials and plugins

A workspace holds secrets only by reference. A secret reference names a logical ID, its owner, the exact scope, the provider or account class, whether it reads or causes effects, the capabilities it serves, the deployment, the purpose and the rotation policy, and its value location is a logical owner, never a path. At apply time a runtime credential binding, kept in the control store, maps that reference and revision to an opaque runtime credential ID for one deployment incarnation, adapter, capability subset and generation; the ID never flows back into Git and the binding can never widen the reference. Creating, rotating, rebinding or revoking a binding is a compare-and-swap on its expected generation and leaves a transition receipt. A restored generation, reuse across controllers or relationships, a read credential used for an effect, or a lookup by provider or account name all fail. Tokens, cookies, client secrets, private keys, refresh tokens and recovery material never enter Git.

A capability record declares its semantic name and schema, native contract owner, tool class, allowed callers and roles, scopes, data class and risk, implementation candidates, timeout, retry, checkpoint, idempotency, effect, ambiguity and retention policies, generated surfaces, fixtures and negative and deletion tests, deployment and kill state. Its decision, implementation, policy, admission and enablement states are independent: a capability can be verified in code yet blocked by policy, or allowed yet absent.

A plugin is an executable extension loaded by a named host, not a skill, a capability or a shortcut to authority. Its source record states the owner, exact source and package hashes, host product and version range, capabilities and extension points, permission and network ceilings, forbidden consequences, secret classes, licence, notices and bill of materials, fixtures and its install and uninstall protocol. A binding ties that source to one scope, host instance, set of caller roles, tool allowlist, configuration hash and secret references. Install, upgrade, disable and uninstall are separate planned operations with journalled intent, read-back and receipts, and uninstall removes only the extension and its generated descriptors, never the host's own state. A plugin cannot mint admission, widen a skill, bypass an owner or turn host runtime IDs into Git source. Credential and plugin records are design only.

## What the materialiser may touch

A native application's own standalone configuration stays first-class and never depends on Pale. The materialiser writes only Pale-owned files, and only into destinations the workspace declares and their owners approve: the agent's front door, skills, roles, operation and tool references, the selected path bindings, and one repository-local Pale hook contribution. A repository's own settings and hook files stay the repository's. It never touches `~/.claude`, `~/.codex`, `~/.gt` or any `settings.local.json`, and it never patches a native plan, a repository's source code, a signal lane's accepted state, a site's authored content or a business decision. Operational source data is read only by its native owner. Runtime IDs and credentials are bound at apply time outside Git. The [agent-surface specification](agent-surfaces.md) describes hook and tool-server rendering and merging.

Generated repository artifacts are classified one by one. Portable doctrine, skills, indexes and contracts may be committable; machine-bound activation and context, locks, receipts and runtime state are local only. A mixed `.pale/` tree is never blanket-committed or blanket-ignored, and apply grants no Git authorisation.

## Inside the running generation

Once a generation is live, four resident machines coordinate it, each keeping its own authority:

1. **Configuration and activation**: publication, candidate fetch, local adoption, compilation, activation.
2. **Occurrence reconciliation**, with two lawful ways in. A native owner's transaction publishes a secret-free outbox entry that Pale admits into its journal and genealogy; or a wake asks Scope to open a case, a source-owned helper reads the source with its own credentials, and Scope admits the resulting secret-free projection before Pale journals it.
3. **Authority and work**: request, policy, then denial, live resolution by the owner, or exact native work and acceptance.
4. **Native effect and settlement**: a durable obligation, the native commit or ambiguity, read-back, receipts and typed settlement.

Adaptive learning is a fifth, separately governed lifecycle that none of these may shortcut.

Every typed relation declares how changes travel along it: **eager** when the receiver needs the fact before acting, **lazy** when a protected interface can safely resolve it later, **notification-only** when the receiver should decide whether to fetch, **prevented**, or **owner ruling** when a new disclosure or authority decision is needed. What the receiver has adopted is a different frontier from what the publisher has released and what the transport has delivered. A bare "something changed" is not enough where a decision depends on why, so the envelope carries the smallest provenance path that explains relevance without leaking the protected body. Agent attention is a budget: memory and precomputation are spent to avoid repeated digging, but actors are not flooded with updates they cannot act on.

A request that reaches an unresolved owner frontier ends with a pending-owner answer and a durable resolution obligation; a later authenticated ruling is revalidated against the generation, authority, scope, evidence and expiry and produces a new allow, deny, superseded or still-pending consequence. The original invocation does not wait, no fallback up a hierarchy is inferred, a missing dependency is never fallback authority, and every denial, failure, expiry and recovery leaves a disclosure-safe record.

Context crosses five independent frontiers, and passing one never implies the next: visibility of the workspace module, admission of the projection by Scope, visibility of the context bundle, permission to show it to a model, and authority to cause a native effect. Within what is already disclosable, context is ordered without a model first (structural relevance, then the current responsibility, then freshness, then explicit pins), Scope may rank candidates inside that set, and Counsel is asked only about genuine residual ties and may abstain. Its absence or a timeout can never widen the set or change an otherwise deterministic answer.

A relationship contract lives in its owner-native home with its parties and representation authorities, the provider's offer and the consumer's adoption, signatories, revision and effective interval, supersession and termination, disclosure, retention, rights and model-use ceilings, obligations and acceptance and settlement frontiers; a compiled contract set is never the contract's source. Source code visible through a protected module reaches a consumer's CodeGraph index only through an independent grant of bytes, an interface index or an owner-side resolver.

## Going live is one staged universe activation

Responsibility-level proof is required, but the whole chosen universe is proved off the live path and then switched once, rather than migrated one responsibility at a time or run in long parallel shadow:

```text
INCUMBENT_PATHS_ACCOUNTED_FOR
  -> COMPLETE_UNIVERSE_DECLARED_OFF_PATH
  -> ALL_INCLUDED_BINDINGS_PROVED_OR_EXPLICITLY_EXCLUDED
  -> OFF_PATH_REFERENCE_AND_FAILURE_JOURNEYS_PASS
  -> OWNER_OPENS_MAINTENANCE_WINDOW
  -> INCUMBENT_WRITERS_FENCED
  -> FINAL_DELTA_CAPTURED_AND_RECONCILED
  -> FINAL_LOCK_AND_GENERATION_VERIFIED
  -> GENERATION_ATOMICALLY_ACTIVATED
  -> NATIVE_READBACK_AND_QUEUE_REVALIDATION
  -> POST_ACTIVATION_OBSERVATION
  -> INCLUDED_RESPONSIBILITY_WRITER_RETIREMENT_PROVED
  -> INCLUDED_BOOTSTRAP_WRITERS_RETIRED
  -> GLOBAL_BOOTSTRAP_RETIRED_ONLY_IF_NO_EXCLUDED_BOOTSTRAP_REMAINS
```

Before fencing, the off-path candidate can be abandoned without touching an incumbent writer. Once fencing begins, abort follows the approved recovery plan even if activation has yet to happen. Read back each affected writer and the active generation. Reconcile the final delta and any ambiguous native effect before restoring a writer under current authority. If a writer's state is uncertain, keep its dependent path closed. After activation, use [forward rollback](../../packages/architecture/release.md) against the exact active predecessor and restore the earlier configuration under a new activation identity and fence. Preserve committed history.

A responsibility that is unready either blocks the generation or is explicitly excluded before the final lock, where the chosen profile permits exclusion. The inventory records a decision for every responsibility. An excluded responsibility keeps its current writer and direct path. Each displaced writer retires after read-back proves it has stopped writing. The bootstrap retires as a whole once no remaining path depends on it. Activation is per principal, so a joint proof still activates each person's universe separately.

The principal authorises activation by naming the generation, included and excluded responsibilities, incumbent and successor writers, final-delta frontier, fence, observation windows, rollback boundary and non-goals. Incumbent writers remain until authorised retirement or a proved successor; a separate owner-directed change remains possible. A clean adopter follows the same authority-first order from an empty workspace: declare owners before copying data, validate and compile, review the plan, authorise apply, obtain native trust and read-back, run reference and failure journeys, and prove rollback, uninstall and absence of another person's defaults.

## Everything fails closed at its own layer

| Layer | Fails closed on |
| --- | --- |
| Paths and sources | An absolute path in portable configuration, a path outside declared roots, a floating source pin |
| Desired graph | An undeclared repository or relation, a name-inferred edge, a duplicate identity |
| Scope | An inferred controller or relationship, a multi-controller mutable runtime |
| Secrets and data | A secret value or runtime ID in Git, a read credential used for an effect |
| Capabilities, skills and plugins | An undeclared caller, host, operation or credential; a widened ceiling |
| Apply | A stale or changed plan, a missing lease, an ambiguous dispatch treated as success, omission treated as deletion |
| Runtime sync | An opaque ID flowing into Git, implicit reverse sync from production |
| Events | A disabled event domain, a broker replacing the local journal |

Regenerating the workspace reproduces its hashes; losing the workspace destroys no native or runtime data; a fresh machine recovers each runtime from its own complete recovery unit; research, pilot and production share no database volume or encryption key.
