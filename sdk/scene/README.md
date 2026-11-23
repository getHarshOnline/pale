# Pale Scene

`@rtepl/pale-scene` compiles Pale inspection results into renderer-neutral geometry for Node.js. It owns layout only: Pale keeps semantic identity, relationships, authority, and evidence; a client owns rendering, transient attention, live observation joins, and instrumentation. This package provides no editor, renderer, desktop host, or browser build.

The output contains nodes, ports, routes, declared gates, and optional route-contact annotations. Stable structure stays visible across generations. Activity, consultation, handoffs, evidence movement, and reconciliation remain temporary client-side emphasis over that structure; Scene does not turn every causal fact into a permanent edge or infer live activity from geometry.

Choose the entry point that matches the question:

- `@rtepl/pale-scene` validates geometry, runs ELK off the main thread, and can estimate contacts between accepted routes.
- `@rtepl/pale-scene/organization` reads recipient-qualified organization results, selects a topology, and compiles a scene.
- `@rtepl/pale-scene/change` reads one recipient-qualified comparison for Source and Change presentations.

Geometry-only use does not load `@rtepl/pale`. The package requires Node.js 24.16 or later. You can build and run the geometry example below without a resident.

## Build the local package

From `sdk/scene`, with Node.js 24.16 or later and its pinned build dependencies available, run:

```sh
npm run build
```

The build produces `dist/`. Package that output and the two runtime dependencies from this checkout into an existing artifact directory:

```sh
npm pack --ignore-scripts --pack-destination /absolute/path/scene-artifacts .
npm pack --ignore-scripts --pack-destination /absolute/path/scene-artifacts ./node_modules/elkjs ./node_modules/paper
```

This creates the local Scene archive, `elkjs-0.12.0.tgz` and `paper-0.12.18.tgz`. Keep those exact dependency versions: the workers load their pinned files. Nothing is published to a registry.

Create a new empty consumer directory. In it, save `package.json` containing `{"private":true,"type":"module"}`, then install all three local archives:

```sh
npm install --offline --ignore-scripts --no-audit --no-fund \
  /absolute/path/scene-artifacts/rtepl-pale-scene-0.0.1.tgz \
  /absolute/path/scene-artifacts/elkjs-0.12.0.tgz \
  /absolute/path/scene-artifacts/paper-0.12.18.tgz
```

The geometry entry needs no `@rtepl/pale` peer. In this consumer, save the example below as `layout-example.mjs` and run `node layout-example.mjs`. It uses the installed package's worker and requires no resident handle.

## Lay out two connected nodes

This example runs ELK in a worker and prints the accepted node and route counts. The coordinates use scene layout units, not device pixels.

```javascript
import { ElkExecutor, GeometryPlan } from "@rtepl/pale-scene";

async function main() {
  const plan = new GeometryPlan({
    nodes: [
      { id: "source", width: 40, height: 30, ports: [{ id: "source-out", side: "EAST" }] },
      { id: "target", width: 40, height: 30, ports: [{ id: "target-in", side: "WEST" }] },
    ],
    relations: [{ id: "flow", sourcePort: "source-out", targetPort: "target-in" }],
  }, { nodes: 2, ports: 2, relations: 1, points: 32, depth: 1, coordinateMagnitude: 10_000 });
  const executor = new ElkExecutor({
    requestBytes: 100_000, resultBytes: 100_000, deadlineMs: 10_000,
    totalRequestBytes: 100_000, totalResultBytes: 100_000, totalActiveMs: 10_000,
    oldHeapMb: 128, youngHeapMb: 32,
  });

  try {
    const result = await executor.run(plan);
    if (result.kind !== "completed") {
      throw new Error(`Layout ${result.kind}: ${result.reason}`);
    }
    console.log(JSON.stringify({ nodes: result.geometry.nodes.length,
      routes: result.geometry.routes.length }));
  } finally {
    await executor.close();
  }
}

void main();
```

Expected output:

```json
{"nodes":2,"routes":1}
```

The non-completed result includes a closed `kind` and `reason`. The `finally` block releases the worker after success or failure.

## Integrate organization and change views

> [!IMPORTANT]
>
> Organization and change intake require opaque recipient-qualified handles issued by Pale's application authority. PaleJS can consume these handles but does not yet acquire them from a resident, so live intake is unavailable. Scene cannot create the handles from source documents, portable compilations or public coordinates. Use the same native SDK installation for the handle and Scene; use the geometry entry point for runnable integrations today.

The [JavaScript SDK](../js/README.md#retained-organization-inspection) describes these inspection types. The [Rust SDK](../rust/README.md#choose-portable-native-or-resident-operations) explains the resident attachment.

### Start a recipient-qualified organization intake

Pass a `RecipientQualifiedWorkspaceInspection` to `OrganizationIntake`, along with the per-query and aggregate limits. This function shows that typed boundary:

```ts
import type { RecipientQualifiedWorkspaceInspection } from "@rtepl/pale";
import { OrganizationIntake } from "@rtepl/pale-scene/organization";

function openSceneIntake(inspection: RecipientQualifiedWorkspaceInspection): OrganizationIntake {
  return new OrganizationIntake(
    inspection,
    { inspected: 64, returned: 64, copiedStringBytes: 1_048_576 },
    {
      queries: 256,
      recordVisits: 16_384,
      sourceVisits: 16_384,
      records: 4_096,
      occurrences: 8_192,
      sourceOccurrences: 8_192,
      copiedStringBytes: 16_777_216,
    },
  );
}
```

Every page and continuation returns to that object's current authority and disclosure check. Closing the intake releases Scene's retained references.

### Inspect a workspace change

`WorkspaceChangeIntake` retains one recipient-qualified SDK comparison. Subject, change-root, and current-impact pages advance independently. Point comparison, contributing-root, direct-dependency, and full source/proof reads accept only occurrences issued by that intake. A full read selects the exact prior or current side and preserves the native SDK page. Continue through the opaque value returned beside that page; it binds the exact intake, occurrence, and side before another native read can start. The comparison owner rechecks authority and disclosure for every call, including each continuation. Appearance IDs are local to the intake and establish neither semantic identity nor authority.

### Compile an organization scene

`OrganizationSceneCompiler` binds one topology selection to its exact admitted inspection. It can produce a full scene, reuse an accepted layout, or complete one bounded region. A completed `OrganizationScene` keeps the semantic coordinates, actual plan options, route witnesses, qualified inspection owner, and prior correspondence used to produce it.

Reuse and regional layout first require the comparison to retain the same qualified predecessor and current inspection owners. Matching public digests are insufficient. Scene then reads the admitted pair summaries and checks that their coordinates still match the two scene sources.

Each topology captures its enumeration and selection coverage when it is created. Coverage says whether enumeration was incomplete, stopped by an exact resource limit, or exhausted, and whether the topology selected exactly the enumerated occurrences. Endpoint queries and repeated appearances do not expand enumeration coverage. Full, reused, and regional scenes retain the current topology's coverage; layout cannot improve it or inherit it from a prior scene.

The structural lens states what spatial proximity means: layout constraints, not semantic similarity or authority. Nearby objects are not necessarily related. A coordinate does not establish causality, disclosure, freshness, or native readiness.

Route witnesses distinguish explicit relations, derived relations, and directional services. Service routes join the selected provider and consumer scopes. Their contract identity and requirements remain in the retained SDK entry. Explicit relations retain the locality reported by the SDK.

`scene.canonicalRouteOrder()` derives a stable back-to-front order from the retained route-binding order. Use `scene.orderRoutes(ids)` only when a client needs an explicit presentation override. Each item keeps its zero-based depth and semantic route witness beside its placed route. Retained and encoded output is charged to the scene's shared budget.

Depth controls occlusion and composition only. It establishes no causal order, priority, authority, execution order, evidence strength, or boundary crossing. Contact analysis consumes the same scene-bound order instead of reconstructing it from route IDs.

`scene.declaredGates` associates a routed native endpoint with its relation witness, appearance, port, and route. A declaration in an unknown state remains unknown. An unrouted declaration has no placement. These associations belong to one scene generation; they are not cross-generation gate identities and do not prove that a native provider exists.

`scene.boundaries` also retains routed service boundaries and the five independent service requirements already present in the SDK witness. These associations do not create a gate, authorize a crossing, or prove completion. `scene.generation` is the opaque handle for the accepted source, structural lens, coverage, constraints, correspondence, and encoded publication.

Use the intake `endpoint()` and `incidentRelations()` queries when you need a selected neighborhood instead of full workspace enumeration. These queries share intake budgets but retain their own SDK pages and source occurrences. They do not consume the enumeration cursor. Incident queries include explicit and already-derived relations only; they do not imply a complete neighborhood. Each bounded intake read accepts an optional `AbortSignal`. A cancelled signal stops the next read before it starts; it does not interrupt a native call already in progress.

Replacing a selection rejects an older completion. Reuse performs no layout. Regional completion keeps its local input and result. The existing executor owns worker cancellation and release. The compiler rechecks cancellation and selection before it publishes a result. A source descriptor is not recipient or resident authority. The retained inspection owner remains the authority for later semantic and observation queries.

## Run layout and contact analysis

Supply bounded geometry to `GeometryPlan`, then call `ElkExecutor.run`. A successful result contains accepted geometry. A failed run leaves its semantic input usable. Await `close()` to finish worker termination.

For optional crossing and depth annotations, call `plan.contactInput(geometry, routeOrder, limits)` with accepted geometry and a complete route order. Then call `ContactExecutor.run`. Later routes are drawn above earlier routes. `OrganizationSceneCompiler.contacts` also retains the selected scene and rejects stale publication.

Contact analysis runs Paper 0.12.18 core in a fresh confined worker. Its results are floating-point estimates in the root coordinate frame. Empty output does not prove that no contact exists. Extreme scales, cusps, retraced curves, and constant curves can produce incomplete results. A contact does not create a semantic relationship or change route geometry. Accepted annotation output is charged to the same aggregate Scene budget before it is returned.

The worker loads only pinned core files and used built-ins. It rejects an ambient DOM module before loading it. This confinement does not make the Node worker API browser-compatible.

## Each stage has its own resource limits

Pass one `SceneBudget` to the organization intake when a complete scene compilation needs a shared ceiling. The budget follows that intake through topology, layout, and accepted output. It charges work, Scene-owned retained bytes, encoded output, and a deadline. Completed work is not refunded after a later failure. Retention counts remain separate because a count and a byte charge answer different questions. The deadline stops between bounded operations; it cannot interrupt a synchronous native call already in progress.

Scene-owned byte charges cover copied strings, local projection values, and accepted renderer-neutral output. They do not estimate native SDK graphs, worker heaps, caller copies, or process RSS. If the caller omits a budget, Scene uses its conservative default aggregate work, retention, output, and deadline ceilings. Supplying a budget narrows or replaces those defaults explicitly.

Each executor limits request bytes, response bytes, active time, and lifetime totals. Only one worker runs at a time. Active-time charges end after worker termination because replacement work remains blocked until then. They do not include retained caller results, native buffers, idle time, or total process memory. Query counts, copied strings, geometry cells, encoded messages, and worker heap settings measure different resources.

Scene occurrences and coordinates are presentation-local. They do not prove semantic identity, permission, execution activity, or a relationship at a route crossing. Raw portable compilations and comparisons are not Scene input.

## Build and test

Build the package:

```sh
npm run build
```

Choose a check by the behavior you need to verify:

| Command | Runnable checks | Blocked checks |
| --- | --- | --- |
| `npm test` | Geometry, workers, contacts, public errors, build boundaries and performance-report checks. | Does not include the separate recipient suite. |
| `npm run test:recipient` | None until the recipient-handle requirement [above](#integrate-organization-and-change-views) is met. | Organization and change execution; no test-only authority substitutes for the required handles. |
| `npm run test:package -- /path/to/pale-sdk.tgz /path/to/organization-fixture` | Installed geometry, packaged workers and compilation of the recipient-qualified type boundary. | Organization execution is reported as blocked. |
| `npm run qualify:performance -- --output /path/to/scene-qualification.json` | Structural geometry performance, with optional contact measurements. | The recipient-qualified organization profile is reported as blocked. |

The recipient suite runs separately from `npm test`.

The package test installs the built Scene package and the native SDK archive into a disposable offline consumer. The SDK archive establishes dependency identity. The organization fixture is retained and hashed, not converted into an inspection handle. The archive and fixture must come from the same SDK qualification. The test does not build Rust at runtime or read another source checkout.

Run the separate geometry performance qualification when you need representative machine evidence:

```sh
npm run qualify:performance -- \
  --output /path/to/scene-qualification.json
```

The structural profile covers 8, 256, and 2,048 nodes. It enforces input, retained-output, wire-byte, worker-time, and caller-delay ceilings. It also requires byte-identical full, reused, and regional output with the pinned ELK algorithm. Warmups are checked but excluded from per-run summaries.

The caller-delay ceiling qualifies observed scheduling for that representative run. It is not a portable scheduling guarantee or a runtime interrupt bound.

The report records wall time and whole-process RSS. RSS includes setup, warmups, workers, and backend allocations, so it remains measured evidence, not a portable memory limit.

Contact analysis is optional because it has no certified completeness or numeric-error bound. Add `--include-contacts` to measure it separately. That measurement does not qualify contacts as part of the structural profile.
