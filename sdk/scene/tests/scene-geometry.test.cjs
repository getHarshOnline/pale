const assert = require("node:assert/strict");
const { test } = require("node:test");
const { createHash } = require("node:crypto");
const { readFileSync } = require("node:fs");
const { GeometryPlan, GeometryError } = require("@rtepl/pale-scene");
const enginePath = require("node:module").createRequire(require.resolve("@rtepl/pale-scene")).resolve("elkjs/lib/elk.bundled.js");
assert.equal(createHash("sha256").update(readFileSync(enginePath)).digest("hex"),
  "1222e44f953ce7746af23801e723708f8e6f436b8b377a6a5fc7552f34a307b3");
const ELK = require(enginePath);
const limits = { nodes: 20, ports: 40, relations: 30, points: 2000, depth: 8, coordinateMagnitude: 1e6 };
const node = (id, parentId) => ({ id, ...(parentId === undefined ? {} : { parentId }), width: 40, height: 30,
  ports: [{ id: `${id}_west`, side: "WEST" }, { id: `${id}_east`, side: "EAST" }] });
const relation = (id, source, target) => ({ id, sourcePort: `${source}_east`, targetPort: `${target}_west` });
const input = () => ({ nodes: [{ id: "group", width: 40, height: 30, ports: [] }, node("a", "group"), node("b", "group"), node("c")],
  relations: [relation("local", "a", "b"), relation("cross", "b", "c"), relation("parallel", "b", "c"), relation("cycle", "c", "a")] });
const layout = plan => new ELK({ algorithms: ["layered"] }).layout(plan.elkInput());
const rejects = (fn, kind) => assert.throws(fn, error => error instanceof GeometryError && error.kind === kind);

test("geometry limits read each declared getter once before using the captured values", () => {
  const reads = {}, supplied = {};
  for (const [key, value] of Object.entries(limits)) Object.defineProperty(supplied, key, {
    enumerable: true, get() {
      reads[key] = (reads[key] ?? 0) + 1;
      assert.equal(reads[key], 1, `${key} must not be read again`);
      return value;
    },
  });
  const plan = new GeometryPlan(input(), supplied);
  assert.equal(plan.elkInput().edges.length, input().relations.length);
  assert.deepEqual(reads, Object.fromEntries(Object.keys(limits).map(key => [key, 1])));
});

test("geometry limits ignore unrelated enumerable getters", () => {
  const supplied = { ...limits };
  Object.defineProperty(supplied, "unrelated", { enumerable: true,
    get() { assert.fail("undeclared limit fields must not be read"); } });
  assert.equal(new GeometryPlan(input(), supplied).elkInput().edges.length, input().relations.length);
});

test("an invalid first limit value cannot be replaced by a valid second getter result", () => {
  for (const key of Object.keys(limits)) {
    let reads = 0;
    const supplied = { ...limits };
    Object.defineProperty(supplied, key, { enumerable: true, get() {
      reads++;
      return reads === 1 ? 0 : limits[key];
    } });
    rejects(() => new GeometryPlan(input(), supplied), "bounds");
    assert.equal(reads, 1, key);
  }
});

test("later caller mutation cannot tighten or weaken retained geometry bounds", async () => {
  const supplied = { ...limits }, plan = new GeometryPlan(input(), supplied);
  const output = await layout(plan);
  for (const key of Object.keys(supplied)) supplied[key] = 0;
  assert.equal(plan.accept(output).nodes.length, input().nodes.length);
  supplied.coordinateMagnitude = limits.coordinateMagnitude * 2;
  const excessive = structuredClone(output);
  excessive.width = limits.coordinateMagnitude + 1;
  rejects(() => plan.accept(excessive), "bounds");
});

const presentationIds = graph => [...graph.nodes.flatMap(node => [node.id, ...node.ports.map(port => port.id)]),
  ...graph.relations.map(route => route.id), ...(graph.bundles ?? []).map(bundle => bundle.id)];
const renamePresentation = graph => {
  const pairs = presentationIds(graph).map((priorId, index) => ({ priorId, currentId: `new${index}` }));
  const ids = new Map(pairs.map(pair => [pair.priorId, pair.currentId]));
  const id = value => ids.get(value);
  return { pairs, graph: {
    nodes: graph.nodes.map(node => ({ ...node, id: id(node.id), ...(node.parentId ? { parentId: id(node.parentId) } : {}),
      ports: node.ports.map(port => ({ ...port, id: id(port.id) })) })),
    relations: graph.relations.map(route => ({ id: id(route.id), sourcePort: id(route.sourcePort), targetPort: id(route.targetPort) })),
    bundles: (graph.bundles ?? []).map(bundle => ({ ...bundle, id: id(bundle.id), routeIds: bundle.routeIds.map(id) })),
  } };
};

test("accepted geometry is deeply immutable and only its accepting plan can supply reuse", async () => {
  const graph = bundleInput(), prior = new GeometryPlan(graph, limits);
  const accepted = prior.accept(await layout(prior)), current = new GeometryPlan(graph, limits);
  const pairs = presentationIds(graph).map(id => ({ priorId: id, currentId: id }));
  const immutable = value => {
    if (value !== null && typeof value === "object") {
      assert(Object.isFrozen(value));
      for (const child of Object.values(value)) immutable(child);
    }
  };
  immutable(accepted);
  assert.equal(Reflect.set(accepted.nodes[0].bounds, "width", 1), false);
  assert.equal(Reflect.set(accepted.routes[0].points[0], "x", 0), false);
  assert.throws(() => accepted.bundles[0].routeIds.push("invented"));
  rejects(() => current.reuseFrom(prior, structuredClone(accepted), pairs), "input");
  rejects(() => current.reuseFrom(current, accepted, pairs), "input");
  assert.throws(() => current.reuseFrom(Object.create(GeometryPlan.prototype), accepted, pairs));
  // Public shadow properties cannot change the genuinely private constraint owner.
  prior.nodes = []; prior.limits = { nodes: 0 }; prior.retain = value => value;
  const copy = current.reuseFrom(prior, accepted, pairs);
  assert.equal(copy.kind, "reused"); immutable(copy.geometry);
});

test("exact explicit renaming preserves nested geometry without another engine invocation", async () => {
  const graph = input(), prior = new GeometryPlan(graph, limits), accepted = prior.accept(await layout(prior));
  const renamed = renamePresentation(graph), current = new GeometryPlan(renamed.graph, limits);
  const result = current.reuseFrom(prior, accepted, renamed.pairs);
  assert.equal(result.kind, "reused");
  const ids = new Map(renamed.pairs.map(pair => [pair.priorId, pair.currentId]));
  for (const [index, node] of result.geometry.nodes.entries()) {
    assert.equal(node.id, ids.get(accepted.nodes[index].id));
    assert.equal(node.parentId, ids.get(accepted.nodes[index].parentId) ?? "$scene");
    assert.strictEqual(node.bounds, accepted.nodes[index].bounds);
  }
  for (const [index, port] of result.geometry.ports.entries()) {
    assert.equal(port.id, ids.get(accepted.ports[index].id));
    assert.equal(port.nodeId, ids.get(accepted.ports[index].nodeId));
    assert.strictEqual(port.bounds, accepted.ports[index].bounds);
  }
  for (const [index, route] of result.geometry.routes.entries()) {
    const old = accepted.routes[index];
    for (const key of ["id", "sourcePort", "targetPort"]) assert.equal(route[key], ids.get(old[key]));
    assert.equal(route.containerId, ids.get(old.containerId) ?? "$scene");
    assert.strictEqual(route.points, old.points);
  }
  assert.strictEqual(result.geometry.root, accepted.root);
  const next = renamePresentation(renamed.graph), final = new GeometryPlan(next.graph, limits);
  assert.equal(final.reuseFrom(current, result.geometry, next.pairs).kind, "reused");
});

test("reuse rejects incomplete, duplicate, foreign and cross-category occurrence mappings", async () => {
  const graph = input(), prior = new GeometryPlan(graph, limits), accepted = prior.accept(await layout(prior));
  const renamed = renamePresentation(graph), current = new GeometryPlan(renamed.graph, limits);
  for (const mutate of [
    pairs => pairs.pop(), pairs => pairs.push({ priorId: "$scene", currentId: "$scene" }),
    pairs => { pairs[1].priorId = pairs[0].priorId; },
    pairs => { pairs[1].currentId = pairs[0].currentId; },
    pairs => { pairs[0].priorId = "$scene"; }, pairs => { pairs[0].currentId = "foreign"; },
    pairs => { [pairs[1].currentId, pairs[2].currentId] = [pairs[2].currentId, pairs[1].currentId]; },
    pairs => { pairs[0] = null; },
  ]) {
    const pairs = structuredClone(renamed.pairs); mutate(pairs);
    rejects(() => current.reuseFrom(prior, accepted, pairs), "input");
  }
  assert.equal(current.reuseFrom(prior, accepted, renamed.pairs).kind, "reused");
});

test("changed ordered structural constraints require layout rather than freezing stale geometry", async () => {
  const graph = input(), prior = new GeometryPlan(graph, limits), accepted = prior.accept(await layout(prior));
  const pairs = presentationIds(graph).map(id => ({ priorId: id, currentId: id }));
  for (const mutate of [
    graph => { graph.nodes[1].width += 1; }, graph => { graph.nodes[1].height += 1; },
    graph => { delete graph.nodes[1].parentId; },
    graph => { graph.nodes[1].ports[0].side = "NORTH"; },
    graph => { graph.nodes[2].ports.push(graph.nodes[1].ports.pop()); },
    graph => { graph.nodes[1].ports.reverse(); },
    graph => { [graph.nodes[1], graph.nodes[2]] = [graph.nodes[2], graph.nodes[1]]; },
    graph => { graph.relations.reverse(); },
    graph => { graph.relations[0].targetPort = "c_west"; },
  ]) {
    const changed = structuredClone(graph); mutate(changed);
    assert.deepEqual(new GeometryPlan(changed, limits).reuseFrom(prior, accepted, pairs),
      { kind: "layout-required", reason: "constraints" });
  }
  const removed = structuredClone(graph); removed.relations.pop();
  assert.equal(new GeometryPlan(removed, limits).reuseFrom(prior, accepted, []).kind, "layout-required");
  for (const key of Object.keys(limits)) {
    assert.deepEqual(new GeometryPlan(graph, { ...limits, [key]: limits[key] + 1 }).reuseFrom(prior, accepted, pairs),
      { kind: "layout-required", reason: "limits" });
  }
});

test("bundle choices participate in exact reuse even though they do not change ELK input", async () => {
  const graph = bundleInput(), prior = new GeometryPlan(graph, limits), accepted = prior.accept(await layout(prior));
  const pairs = presentationIds(graph).map(id => ({ priorId: id, currentId: id }));
  for (const mutate of [graph => graph.bundles.reverse(), graph => graph.bundles[0].routeIds.reverse()]) {
    const changed = structuredClone(graph); mutate(changed);
    assert.deepEqual(new GeometryPlan(changed, limits).reuseFrom(prior, accepted, pairs),
      { kind: "layout-required", reason: "constraints" });
  }
  // Switching one sole bundle's end remains a valid selection, but is not equal.
  graph.bundles.pop();
  const sole = new GeometryPlan(graph, limits), old = sole.accept(await layout(sole));
  const solePairs = presentationIds(graph).map(id => ({ priorId: id, currentId: id }));
  graph.bundles[0].end = "target";
  assert.equal(new GeometryPlan(graph, limits).reuseFrom(sole, old, solePairs).kind, "layout-required");
});

test("shared, partitioned and unshared corridors keep every original range when relabeled", async () => {
  const seen = new Set();
  for (const count of [2, 4]) {
    const graph = { nodes: Array.from({ length: count }, (_, index) => node(`n${index}`)),
      relations: Array.from({ length: count - 1 }, (_, index) => relation(`r${index}`, `n${index}`, `n${count - 1}`)) };
    graph.bundles = [{ id: "fan", end: "target", routeIds: graph.relations.map(route => route.id) }];
    // A two-route parallel pair covers the shared/unshared cases.
    if (count === 2) { graph.relations.push({ ...graph.relations[0], id: "parallel" }); graph.bundles[0].routeIds.push("parallel"); }
    const prior = new GeometryPlan(graph, limits), output = await layout(prior);
    const variants = [output];
    if (count === 2) {
      const shared = structuredClone(output), points = threeSegments(shared);
      for (const edge of shared.edges) setChain(edge.sections[0], structuredClone(points));
      variants.push(shared);
      const distinct = structuredClone(shared); distinct.edges[1].sections[0].bendPoints[0].y += 1;
      distinct.edges[1].sections[0].bendPoints.at(-1).y += 1;
      assert.notDeepEqual(distinct.edges[0].sections[0].bendPoints, distinct.edges[1].sections[0].bendPoints);
      variants.push(distinct);
    }
    const renamed = renamePresentation(graph), current = new GeometryPlan(renamed.graph, limits);
    const ids = new Map(renamed.pairs.map(pair => [pair.priorId, pair.currentId]));
    const relabel = value => typeof value === "string" ? (ids.get(value) ?? value)
      : Array.isArray(value) ? value.map(relabel)
      : value !== null && typeof value === "object" ? Object.fromEntries(Object.entries(value).map(([key, child]) => [key, relabel(child)])) : value;
    for (const variant of variants) {
      const accepted = prior.accept(variant), reused = current.reuseFrom(prior, accepted, renamed.pairs);
      assert.equal(reused.kind, "reused"); seen.add(accepted.bundles[0].kind);
      assert.deepEqual(reused.geometry, relabel(accepted));
    }
  }
  assert.deepEqual([...seen].sort(), ["partitioned", "shared", "unshared"]);
});

test("empty accepted geometry can be explicitly reused without claiming a complete semantic world", async () => {
  const empty = { nodes: [], relations: [] }, prior = new GeometryPlan(empty, limits);
  const accepted = prior.accept(await layout(prior)), current = new GeometryPlan(empty, limits);
  assert.deepEqual(current.reuseFrom(prior, accepted, []), { kind: "reused", geometry: accepted });
});

function regionInput(nested = false) {
  const graph = { nodes: [], relations: [] };
  if (nested) graph.nodes.push({ id: "world", width: 40, height: 40, ports: [] });
  for (const region of ["a", "b"]) {
    graph.nodes.push({ id: region, ...(nested ? { parentId: "world" } : {}), width: 40, height: 40,
      ports: [{ id: `${region}_gate`, side: region === "a" ? "EAST" : "WEST" }] });
    for (let i = 0; i < 3; i++) graph.nodes.push({ ...node(`${region}${i}`, region), width: 80,
      height: region === "a" && i === 0 ? 160 : 40 });
    for (let i = 0; i < 2; i++) graph.relations.push(relation(`${region}_r${i}`, `${region}${i}`, `${region}${i + 1}`));
  }
  graph.relations.push({ id: "bridge", sourcePort: "a_gate", targetPort: "b_gate" });
  return graph;
}
function boundaryPairs(plan, region, rename = value => value) {
  const boundary = plan.regionBoundary(region); assert.equal(boundary.kind, "region-boundary");
  return boundary.retainedIds.map(priorId => ({ priorId, currentId: rename(priorId) }));
}

test("one rooted update retains outside geometry in both frames and matches fresh exact membership", async () => {
  for (const nested of [false, true]) {
    const graph = regionInput(nested), prior = new GeometryPlan(graph, limits), accepted = prior.accept(await layout(prior));
    const changed = structuredClone(graph); changed.nodes.find(node => node.id === "a1").height = 100;
    const current = new GeometryPlan(changed, limits), ready = current.prepareRegion(prior, accepted, "a", "a", boundaryPairs(prior, "a"));
    assert.equal(ready.kind, "region-ready");
    const input = ready.plan.elkInput(), region = input.children[0];
    assert.equal(region.layoutOptions["elk.portConstraints"], "FIXED_POS");
    const local = ready.plan.accept(await layout(ready.plan)), result = ready.complete(local);
    assert.equal(result.kind, "region-composed");
    assert(Object.isFrozen(ready) && Object.isFrozen(result.geometry));
    for (const old of accepted.nodes.filter(node => !["a0", "a1", "a2"].includes(node.id))) {
      assert.deepEqual(result.geometry.nodes.find(node => node.id === old.id), old);
    }
    for (const old of accepted.routes.filter(route => !route.id.startsWith("a_r"))) {
      assert.deepEqual(result.geometry.routes.find(route => route.id === old.id), old);
    }
    const fresh = current.accept(await layout(current));
    const membership = geometry => ({ nodes: geometry.nodes.map(node => [node.id, node.parentId]),
      ports: geometry.ports.map(port => [port.id, port.nodeId]), routes: geometry.routes.map(route => [route.id, route.sourcePort, route.targetPort]) });
    assert.deepEqual(membership(result.geometry), membership(fresh));
    assert.equal(current.reuseFrom(current, result.geometry, presentationIds(changed).map(id => ({ priorId: id, currentId: id }))).kind, "reused");
  }
});

test("local scene-frame routes are rebased into the selected root rather than mistaken for full-scene coordinates", async () => {
  const graph = regionInput(true), prior = new GeometryPlan(graph, limits), original = await layout(prior);
  const bridge = original.edges.find(edge => edge.id === "bridge"), world = original.children[0];
  assert.equal(bridge.container, "world"); bridge.container = "$scene";
  for (const point of [bridge.sections[0].startPoint, ...(bridge.sections[0].bendPoints ?? []), bridge.sections[0].endPoint]) {
    point.x += world.x; point.y += world.y;
  }
  const accepted = prior.accept(original);
  const current = new GeometryPlan(graph, limits), ready = current.prepareRegion(prior, accepted, "a", "a", boundaryPairs(prior, "a"));
  assert.equal(ready.kind, "region-ready");
  const output = await layout(ready.plan), origin = output.children[0];
  for (const edge of output.edges) {
    assert.equal(edge.container, "a"); edge.container = "$scene";
    const section = edge.sections[0];
    for (const point of [section.startPoint, ...(section.bendPoints ?? []), section.endPoint]) { point.x += origin.x; point.y += origin.y; }
  }
  const local = ready.plan.accept(output), result = ready.complete(local);
  assert.equal(result.kind, "region-composed");
  assert(result.geometry.routes.filter(route => route.id.startsWith("a_r")).every(route => route.containerId === "a"));
  assert.deepEqual(result.geometry.routes, accepted.routes);
  assert.equal(result.geometry.routes.find(route => route.id === "bridge").containerId, "$scene");
});

test("added and removed interior identities do not require fabricated prior pairs", async () => {
  const graph = regionInput(), prior = new GeometryPlan(graph, limits), accepted = prior.accept(await layout(prior));
  const changed = structuredClone(graph), replacement = changed.nodes.find(node => node.id === "a1");
  replacement.id = "replacement"; replacement.ports = [{ id: "replacement_west", side: "WEST" }, { id: "replacement_east", side: "EAST" }];
  changed.relations.find(route => route.id === "a_r0").targetPort = "replacement_west";
  changed.relations.find(route => route.id === "a_r1").sourcePort = "replacement_east";
  const current = new GeometryPlan(changed, limits), ready = current.prepareRegion(prior, accepted, "a", "a", boundaryPairs(prior, "a"));
  assert.equal(ready.kind, "region-ready");
  const result = ready.complete(ready.plan.accept(await layout(ready.plan))); assert.equal(result.kind, "region-composed");
  assert(!result.geometry.nodes.some(node => node.id === "a1"));
  assert(result.geometry.nodes.some(node => node.id === "replacement"));
  assert.equal(result.geometry.routes.find(route => route.id === "a_r0").targetPort, "replacement_west");
});

test("a changed gate or envelope requires ancestor layout without changing the prior result", async () => {
  const graph = regionInput(), prior = new GeometryPlan(graph, limits), accepted = prior.accept(await layout(prior));
  const baseline = JSON.stringify(accepted);
  const current = new GeometryPlan(graph, limits), ready = current.prepareRegion(prior, accepted, "a", "a", boundaryPairs(prior, "a"));
  const output = await layout(ready.plan); output.children[0].ports[0].y += 1;
  assert.deepEqual(ready.complete(ready.plan.accept(output)), { kind: "region-escalation", reason: "ports", regionId: "$scene" });
  const larger = structuredClone(graph); larger.nodes.find(node => node.id === "a1").height = 240;
  const enlarged = new GeometryPlan(larger, limits).prepareRegion(prior, accepted, "a", "a", boundaryPairs(prior, "a"));
  assert.equal(enlarged.kind, "region-ready");
  assert.equal(enlarged.complete(enlarged.plan.accept(await layout(enlarged.plan))).reason, "envelope");
  assert.equal(JSON.stringify(accepted), baseline);
});

test("direct cross-region routes and mixed bundles escalate instead of cutting geometry", () => {
  const direct = regionInput(true); direct.relations[4] = relation("bridge", "a2", "b0");
  const crossed = new GeometryPlan(direct, limits).regionBoundary("a");
  assert.deepEqual(crossed, { kind: "region-escalation", reason: "crossing", regionId: "world" });
  const bundled = regionInput();
  bundled.relations.push({ id: "inner_gate", sourcePort: "a_gate", targetPort: "a0_west" });
  bundled.bundles = [{ id: "mixed", end: "source", routeIds: ["bridge", "inner_gate"] }];
  assert.deepEqual(new GeometryPlan(bundled, limits).regionBoundary("a"),
    { kind: "region-escalation", reason: "bundle", regionId: "$scene" });
});

test("accepted endpoint contact alone cannot retain a curve whose control hull enters the region", async () => {
  const graph = regionInput(), prior = new GeometryPlan(graph, limits), output = await layout(prior);
  const region = output.children.find(node => node.id === "a"), section = output.edges.find(edge => edge.id === "bridge").sections[0];
  assert(section.bendPoints.length > 0);
  section.bendPoints[0] = { x: region.x + region.width / 2, y: region.y + region.height / 2 };
  const accepted = prior.accept(output), current = new GeometryPlan(graph, limits);
  assert.deepEqual(current.prepareRegion(prior, accepted, "a", "a", boundaryPairs(prior, "a")),
    { kind: "region-escalation", reason: "confinement", regionId: "$scene" });
});

test("regional custody rejects forged plans/results and does not dispatch through shadowed acceptance", async () => {
  const graph = regionInput(), prior = new GeometryPlan(graph, limits), accepted = prior.accept(await layout(prior));
  const current = new GeometryPlan(graph, limits), pairs = boundaryPairs(prior, "a");
  rejects(() => current.prepareRegion(prior, structuredClone(accepted), "a", "a", pairs), "input");
  rejects(() => current.prepareRegion(prior, accepted, "a", "a", pairs.slice(1)), "input");
  const ready = current.prepareRegion(prior, accepted, "a", "a", pairs), raw = await layout(ready.plan);
  rejects(() => ready.complete(structuredClone(ready.plan.accept(raw))), "input");
  current.accept = () => assert.fail("composition must use the private shared validator");
  assert.equal(ready.complete(ready.plan.accept(raw)).kind, "region-composed");
});

test("regional composition checks the combined point budget and retains the accepted predecessor", async () => {
  const graph = regionInput(), probe = new GeometryPlan(graph, limits), output = await layout(probe);
  const initial = probe.accept(output), points = initial.routes.reduce((sum, route) => sum + route.points.length, 0);
  const exact = { ...limits, points }, prior = new GeometryPlan(graph, exact), accepted = prior.accept(output);
  const current = new GeometryPlan(graph, exact), ready = current.prepareRegion(prior, accepted, "a", "a", boundaryPairs(prior, "a"));
  assert.equal(ready.kind, "region-ready");
  const local = await layout(ready.plan), section = local.edges[0].sections[0];
  // ELK may return a straight edge. Encode that same line as one cubic first.
  if ((section.bendPoints ?? []).length === 0) section.bendPoints = [{ ...section.startPoint }, { ...section.endPoint }];
  // Add one zero-length cubic after the original end without changing contacts.
  section.bendPoints.push({ ...section.endPoint }, { ...section.endPoint }, { ...section.endPoint });
  const localAccepted = ready.plan.accept(local);
  assert(localAccepted.routes.reduce((sum, route) => sum + route.points.length, 0) <= points);
  assert.deepEqual(ready.complete(localAccepted), { kind: "region-escalation", reason: "limits", regionId: "$scene" });
  assert.deepEqual(prior.accept(output), accepted);
});

test("changed outside constraints and limits cannot be hidden by a region selection", async () => {
  const graph = regionInput(), prior = new GeometryPlan(graph, limits), accepted = prior.accept(await layout(prior));
  const changed = structuredClone(graph); changed.nodes.find(node => node.id === "b0").width += 1;
  assert.deepEqual(new GeometryPlan(changed, limits).prepareRegion(prior, accepted, "a", "a", boundaryPairs(prior, "a")),
    { kind: "region-escalation", reason: "constraints", regionId: "$scene" });
  assert.deepEqual(new GeometryPlan(graph, { ...limits, points: limits.points + 1 })
    .prepareRegion(prior, accepted, "a", "a", boundaryPairs(prior, "a")),
    { kind: "region-escalation", reason: "limits", regionId: "$scene" });
  for (const dimension of ["width", "height"]) {
    const larger = structuredClone(graph);
    larger.nodes.find(node => node.id === "a")[dimension] = accepted.nodes.find(node => node.id === "a").bounds[dimension] + 1;
    assert.deepEqual(new GeometryPlan(larger, limits).prepareRegion(prior, accepted, "a", "a", boundaryPairs(prior, "a")),
      { kind: "region-escalation", reason: "envelope", regionId: "$scene" });
  }
});

test("real compound layout honors declared minimum dimensions instead of shrinking to its contents", async () => {
  const graph = { nodes: [{ id: "group", width: 400, height: 200, ports: [] }, node("a", "group"), node("b")],
    relations: [relation("bridge", "a", "b")] };
  const plan = new GeometryPlan(graph, limits), result = plan.accept(await layout(plan));
  const group = result.nodes.find(node => node.id === "group");
  assert(group.bounds.width >= 400); assert(group.bounds.height >= 200);
  const original = await layout(plan), tooSmall = structuredClone(original);
  tooSmall.children[0].height = 199;
  rejects(() => plan.accept(tooSmall), "output");
});

test("nested frames, parallel edges and a cycle retain exact endpoints in input order", async () => {
  const plan = new GeometryPlan(input(), limits);
  const output = await layout(plan);
  const accepted = plan.accept(output);
  assert.deepEqual(accepted.nodes.map(node => node.id), ["group", "a", "b", "c"]);
  assert.deepEqual(accepted.routes.map(({ id, sourcePort, targetPort }) => ({ id, sourcePort, targetPort })), input().relations);
  assert.equal(accepted.routes[0].containerId, "group");
  assert(accepted.routes.some(route => route.kind === "cubic-chain"));
  assert(accepted.routes.some(route => route.kind === "line"));
  for (const route of accepted.routes) {
    if (route.kind === "cubic-chain") assert.equal((route.points.length - 1) % 3, 0);
    else assert.equal(route.points.length, 2);
  }
  assert.deepEqual(plan.accept(await layout(plan)), accepted);
  output.children.reverse(); output.edges.reverse();
  assert.deepEqual(plan.accept(output), accepted);
});

test("empty geometry is a valid scene, not fabricated semantic completeness", async () => {
  const plan = new GeometryPlan({ nodes: [], relations: [] }, limits);
  const result = plan.accept(await layout(plan));
  assert.deepEqual(result.nodes, []); assert.deepEqual(result.ports, []); assert.deepEqual(result.routes, []);
});

test("request ownership survives caller mutation and copies sent to the engine", async () => {
  const original = input(), plan = new GeometryPlan(original, limits);
  original.nodes[1].id = "different"; original.relations.length = 0;
  const copy = plan.elkInput(); copy.children.length = 0;
  const result = plan.accept(await layout(plan));
  assert.equal(result.nodes[1].id, "a"); assert.equal(result.routes.length, 4);
});

test("input rejects duplicate IDs, missing parents, unknown endpoints and depth overflow", () => {
  let graph = input(); graph.nodes[1].id = "group"; rejects(() => new GeometryPlan(graph, limits), "input");
  graph = input(); graph.nodes[1].parentId = "absent"; rejects(() => new GeometryPlan(graph, limits), "input");
  graph = input(); graph.relations[0].sourcePort = "absent"; rejects(() => new GeometryPlan(graph, limits), "input");
  rejects(() => new GeometryPlan(input(), { ...limits, depth: 1 }), "bounds");
});

test("bounded counts reject before allocating a returned scene", async () => {
  rejects(() => new GeometryPlan(input(), { ...limits, nodes: 3 }), "bounds");
  rejects(() => new GeometryPlan(input(), { ...limits, ports: 5 }), "bounds");
  rejects(() => new GeometryPlan(input(), { ...limits, relations: 3 }), "bounds");
  const output = await layout(new GeometryPlan(input(), limits));
  rejects(() => new GeometryPlan(input(), { ...limits, points: 2 }).accept(output), "bounds");
  rejects(() => new GeometryPlan(input(), { ...limits, points: Infinity }), "bounds");
});

test("no missing, extra or substituted node, port or relation is accepted", async () => {
  const plan = new GeometryPlan(input(), limits), output = await layout(plan);
  for (const mutate of [
    graph => graph.children.pop(),
    graph => { graph.children[0].id = "substituted"; },
    graph => graph.children[0].children[0].ports.pop(),
    graph => { graph.children[0].children[0].ports[0].id = "substituted"; },
    graph => graph.edges.pop(),
    graph => { graph.edges[0].id = graph.edges[1].id; },
    graph => { graph.edges[0].sources[0] = graph.edges[1].sources[0]; },
  ]) {
    const corrupt = structuredClone(output); mutate(corrupt);
    rejects(() => plan.accept(corrupt), "output");
  }
});

test("moving a node into another parent is not accepted under its old occurrence ID", async () => {
  const plan = new GeometryPlan(input(), limits), output = await layout(plan);
  output.children.push(output.children[0].children.pop());
  rejects(() => plan.accept(output), "output");
});

test("nonfinite and excessive coordinates reject even on unused control points", async () => {
  const plan = new GeometryPlan(input(), limits), output = await layout(plan);
  for (const value of [NaN, Infinity, -Infinity, 1e9]) {
    const corrupt = structuredClone(output);
    const section = corrupt.edges.flatMap(edge => edge.sections).find(section => section.bendPoints?.length);
    assert(section, "real ELK output must include a curved route");
    section.bendPoints[0].x = value;
    rejects(() => plan.accept(corrupt), "bounds");
  }
});

test("a malformed cubic chain is not interpreted as arbitrary polylines", async () => {
  const plan = new GeometryPlan(input(), limits), output = await layout(plan);
  const section = output.edges.flatMap(edge => edge.sections).find(section => section.bendPoints?.length);
  assert(section, "real ELK output must include a curved route");
  section.bendPoints.pop();
  rejects(() => plan.accept(output), "output");
});

test("wrong frame and endpoint displacement cannot pass relation identity checks", async () => {
  const plan = new GeometryPlan(input(), limits), output = await layout(plan);
  for (const mutate of [
    graph => { graph.edges[0].container = "$scene"; },
    graph => { graph.edges[1].container = "a"; },
    graph => { graph.edges[0].sections[0].startPoint.x += 1000; },
    graph => { graph.edges[0].sections[0].outgoingShape = "c_west"; },
  ]) {
    const corrupt = structuredClone(output); mutate(corrupt);
    rejects(() => plan.accept(corrupt), "output");
  }
});

test("unknown engine fields do not become a semantic metadata channel", async () => {
  const plan = new GeometryPlan(input(), limits), output = await layout(plan);
  const clean = plan.accept(output);
  output.authority = "pretend"; output.children[0].semanticId = "pretend";
  output.edges[0].permission = "pretend";
  assert.deepEqual(plan.accept(output), clean);
});

test("ports retain their requested side even when a forged route still touches them", async () => {
  const plan = new GeometryPlan(input(), limits), output = await layout(plan);
  const port = output.children[0].children[0].ports.find(port => port.id === "a_east");
  port.y += 1000;
  output.edges[0].sections[0].startPoint.y += 1000;
  rejects(() => plan.accept(output), "output");
});

test("objects cannot shrink below requested dimensions or escape group bounds", async () => {
  const plan = new GeometryPlan(input(), limits), output = await layout(plan);
  const undersized = structuredClone(output); undersized.children[0].children[0].width = 1;
  rejects(() => plan.accept(undersized), "output");
  const escaped = structuredClone(output); escaped.children[0].children[0].x += 1000;
  rejects(() => plan.accept(escaped), "output");
});

test("north/south ports and self-loop curves use the same exact occurrence joins", async () => {
  const graph = { nodes: [{ id: "a", width: 40, height: 30, ports: [
    { id: "n", side: "NORTH" }, { id: "s", side: "SOUTH" },
  ] }], relations: [{ id: "loop", sourcePort: "n", targetPort: "s" }] };
  const plan = new GeometryPlan(graph, limits);
  const result = plan.accept(await layout(plan));
  assert.equal(result.routes[0].sourcePort, "n"); assert.equal(result.routes[0].targetPort, "s");
  assert.equal(result.routes[0].kind, "cubic-chain");
  assert.deepEqual(result.nodes.map(node => node.id), ["a"]);
});

test("unused ports cannot cross the absolute coordinate ceiling", async () => {
  const graph = { nodes: [node("a")], relations: [] };
  const plan = new GeometryPlan(graph, limits), output = await layout(plan);
  output.width = limits.coordinateMagnitude;
  output.children[0].x = limits.coordinateMagnitude - output.children[0].width;
  rejects(() => plan.accept(output), "bounds");
});

test("equivalent ancestor-frame rebasing preserves geometry without recomputing containment", async () => {
  const plan = new GeometryPlan(input(), limits), output = await layout(plan);
  const original = plan.accept(output).routes[0];
  const group = output.children.find(node => node.id === "group");
  const edge = output.edges.find(edge => edge.id === "local");
  edge.container = "$scene";
  const section = edge.sections[0];
  for (const point of [section.startPoint, ...(section.bendPoints ?? []), section.endPoint]) {
    point.x += group.x; point.y += group.y;
  }
  const rebased = plan.accept(output).routes[0];
  assert.deepEqual(rebased.points, original.points.map(point => ({ x: point.x + group.x, y: point.y + group.y })));
  assert.equal(rebased.id, original.id); assert.equal(rebased.sourcePort, original.sourcePort);
  assert.equal(rebased.targetPort, original.targetPort); assert.equal(rebased.kind, original.kind);
});

const bundleInput = () => ({ nodes: [node("a"), node("b")],
  relations: [relation("first", "a", "b"), relation("second", "a", "b")],
  bundles: [{ id: "outbound", end: "source", routeIds: ["first", "second"] },
    { id: "inbound", end: "target", routeIds: ["second", "first"] }] });
const setChain = (section, points) => {
  section.startPoint = points[0]; section.endPoint = points.at(-1);
  section.bendPoints = points.slice(1, -1);
};
// Controlled whole-segment differences over real accepted endpoints and frames.
const threeSegments = output => {
  const { startPoint: s, endPoint: e } = output.edges[0].sections[0];
  return [s, { x: s.x + 1, y: s.y }, { x: s.x + 2, y: s.y }, { x: s.x + 3, y: s.y },
    { x: s.x + 4, y: s.y }, { x: e.x - 4, y: e.y }, { x: e.x - 3, y: e.y },
    { x: e.x - 2, y: e.y }, { x: e.x - 1, y: e.y }, e];
};

test("bundles preserve exact maximal whole cubic prefixes and suffixes, original routes and ranges", async () => {
  const graph = bundleInput(), plan = new GeometryPlan(graph, limits), output = await layout(plan);
  const first = threeSegments(output), second = structuredClone(first); second[4].y += 1;
  setChain(output.edges[0].sections[0], first); setChain(output.edges[1].sections[0], second);
  const result = plan.accept(output);
  assert.deepEqual(result.routes.map(route => route.id), ["first", "second"]);
  assert.deepEqual(result.routes.map(route => route.points), [first, second]);
  for (const bundle of result.bundles) {
    assert.equal(bundle.kind, "shared"); assert.equal(bundle.routeKind, "cubic-chain");
    assert.equal(bundle.containerId, "$scene");
    const start = bundle.end === "source" ? 0 : 2;
    assert.deepEqual(bundle.members, bundle.routeIds.map(routeId => ({ routeId, firstSegment: start, endSegmentExclusive: start + 1 })));
    assert.deepEqual(bundle.points, first.slice(start * 3, (start + 1) * 3 + 1));
    assert.equal(bundle.portId, bundle.end === "source" ? "a_east" : "b_west");
  }
  setChain(output.edges[1].sections[0], structuredClone(first));
  const equal = plan.accept(output);
  for (const bundle of equal.bundles) {
    assert.equal(bundle.kind, "shared"); assert.deepEqual(bundle.points, first);
    assert(bundle.members.every(member => member.firstSegment === 0 && member.endSegmentExclusive === 3));
  }
});

test("shared points or near-equal control points do not fabricate a shared segment", async () => {
  const plan = new GeometryPlan(bundleInput(), limits), output = await layout(plan);
  const first = threeSegments(output), second = structuredClone(first);
  second[1].y += 1e-9; second[8].y += 1e-9;
  setChain(output.edges[0].sections[0], first); setChain(output.edges[1].sections[0], second);
  const result = plan.accept(output);
  assert(result.bundles.every(bundle => bundle.kind === "unshared" && bundle.reason === "no-shared-segment"));
  assert.equal(result.routes.length, 2);
});

test("an exact point-only cubic prefix or suffix does not become a shared corridor", async () => {
  const plan = new GeometryPlan(bundleInput(), limits), original = await layout(plan);
  const { startPoint: s, endPoint: e } = original.edges[0].sections[0];
  for (const end of ["source", "target"]) {
    const output = structuredClone(original);
    const first = end === "source"
      ? [s, s, s, s, { x: s.x + 4, y: s.y }, { x: e.x - 4, y: e.y }, e]
      : [s, { x: s.x + 4, y: s.y }, { x: e.x - 4, y: e.y }, e, e, e, e];
    const second = structuredClone(first); second[end === "source" ? 4 : 1].y += 1;
    setChain(output.edges[0].sections[0], first); setChain(output.edges[1].sections[0], second);
    const result = plan.accept(output), bundle = result.bundles.find(bundle => bundle.end === end);
    assert.equal(bundle.kind, "unshared"); assert.equal(bundle.reason, "no-shared-segment");
    assert.deepEqual(result.routes.map(route => route.points), [first, second]);
  }
});

test("zero segments remain in an exact shared run that also has nonzero extent", async () => {
  const plan = new GeometryPlan(bundleInput(), limits), output = await layout(plan);
  const { startPoint: s, endPoint: e } = output.edges[0].sections[0];
  const points = [s, s, s, s, { x: s.x + 4, y: s.y }, { x: e.x - 4, y: e.y }, e, e, e, e];
  for (const edge of output.edges) setChain(edge.sections[0], structuredClone(points));
  const result = plan.accept(output);
  for (const bundle of result.bundles) {
    assert.equal(bundle.kind, "shared"); assert.deepEqual(bundle.points, points);
    assert(bundle.members.every(member => member.firstSegment === 0 && member.endSegmentExclusive === 3));
  }
});

test("whole straight segments can share; mixed straight and cubic chains stay distinct", async () => {
  const plan = new GeometryPlan(bundleInput(), limits), output = await layout(plan), points = threeSegments(output);
  for (const edge of output.edges) setChain(edge.sections[0], [points[0], points.at(-1)]);
  const straight = plan.accept(output);
  assert(straight.bundles.every(bundle => bundle.kind === "shared" && bundle.routeKind === "line"
    && bundle.members.every(member => member.firstSegment === 0 && member.endSegmentExclusive === 1)));
  setChain(output.edges[1].sections[0], points);
  assert(plan.accept(output).bundles.every(bundle => bundle.kind === "unshared" && bundle.reason === "different-kinds"));
});

test("different valid ancestor frames report incompatibility without fuzzy rebasing", async () => {
  const graph = bundleInput(); graph.nodes = [{ id: "group", width: 40, height: 30, ports: [] }, node("a", "group"), node("b", "group")];
  const plan = new GeometryPlan(graph, limits), output = await layout(plan);
  const points = threeSegments(output);
  for (const edge of output.edges) setChain(edge.sections[0], structuredClone(points));
  const group = output.children[0], edge = output.edges[1]; edge.container = "$scene";
  const section = edge.sections[0];
  for (const point of [section.startPoint, ...section.bendPoints, section.endPoint]) { point.x += group.x; point.y += group.y; }
  const result = plan.accept(output);
  assert.deepEqual(result.routes.map(route => route.containerId), ["group", "$scene"]);
  assert(result.bundles.every(bundle => bundle.kind === "unshared" && bundle.reason === "different-frames"));
});

test("bundle input and accepted result mutation cannot change the owned request", async () => {
  const graph = bundleInput(), plan = new GeometryPlan(graph, limits);
  const plain = new GeometryPlan({ nodes: graph.nodes, relations: graph.relations }, limits);
  assert.deepEqual(plan.elkInput(), plain.elkInput(), "bundle selection does not reroute ELK");
  graph.bundles[0].routeIds.reverse(); graph.bundles[0].end = "target"; graph.bundles.length = 0;
  const output = await layout(plan), expected = plan.accept(output), prior = plan.accept(output);
  assert.equal(Reflect.set(prior.bundles[0].routeIds, "length", 0), false);
  assert.equal(Reflect.set(prior.bundles, "length", 0), false);
  assert.deepEqual(plan.accept(output), expected);
  assert.deepEqual(expected.bundles[0].routeIds, ["first", "second"]);
  assert.equal(expected.bundles[0].end, "source");
});

test("bundle identity, selected end, membership and exact common port are validated", () => {
  for (const mutate of [
    graph => { graph.bundles[0].id = "first"; },
    graph => { graph.bundles[1].id = "outbound"; },
    graph => { graph.bundles[0].end = "middle"; },
    graph => { graph.bundles[0].routeIds[1] = "unknown"; },
    graph => { graph.bundles[0].routeIds[1] = "first"; },
    graph => { graph.bundles[1].end = "source"; },
    graph => { graph.relations[1].sourcePort = "a_west"; },
    graph => { graph.relations[1].targetPort = "b_east"; },
    graph => { graph.bundles[0].routeIds = []; },
    graph => { graph.bundles[0].routeIds = ["first"]; },
    graph => { graph.bundles[0].routeIds = { length: 2 }; },
  ]) {
    const graph = bundleInput(); mutate(graph); rejects(() => new GeometryPlan(graph, limits), "input");
  }
});

test("derived bundle bounds reject before selected members or node geometry are read", () => {
  const forbidden = () => { throw new Error("must not traverse over-budget selection"); };
  let graph = bundleInput();
  graph.bundles = new Proxy(new Array(3), { get(target, key, receiver) { if (key === "0") forbidden(); return Reflect.get(target, key, receiver); } });
  rejects(() => new GeometryPlan(graph, limits), "bounds");
  graph = bundleInput();
  const inaccessible = new Proxy(["first", "second", "first"], { get(target, key, receiver) { if (key === "0") forbidden(); return Reflect.get(target, key, receiver); } });
  graph.bundles[0].routeIds = inaccessible;
  graph.bundles[1].routeIds = inaccessible;
  Object.defineProperty(graph.nodes[0], "id", { get: forbidden });
  rejects(() => new GeometryPlan(graph, limits), "bounds");
});

test("malformed engine geometry never publishes a bundle and a valid retry still works", async () => {
  const plan = new GeometryPlan(bundleInput(), limits), output = await layout(plan), clean = plan.accept(output);
  const corrupt = structuredClone(output); corrupt.edges[0].sections[0].endPoint.x += 10000;
  rejects(() => plan.accept(corrupt), "output");
  assert.deepEqual(plan.accept(output), clean);
});

test("real pinned ELK fan-in exposes an exact terminal cubic bundle without replacing routes", async () => {
  const graph = { nodes: [node("a"), node("b"), node("c"), node("d"), node("sink")],
    relations: ["a", "b", "c", "d"].map(id => relation(`route_${id}`, id, "sink")),
    bundles: [{ id: "arrival", end: "target", routeIds: ["a", "b", "c", "d"].map(id => `route_${id}`) }] };
  const plan = new GeometryPlan(graph, limits), result = plan.accept(await layout(plan));
  assert.equal(result.routes.length, 4);
  const selection = result.bundles[0]; assert.equal(selection.kind, "partitioned");
  const bundle = selection.corridors.find(corridor => corridor.members.length === 4); assert(bundle);
  assert.equal(bundle.routeKind, "cubic-chain");
  assert.equal(selection.portId, "sink_west"); assert(bundle.points.length >= 4);
  assert(selection.corridors.some(corridor => corridor.members.length < 4), "longer subgroup runs remain visible");
  for (const member of bundle.members) {
    const route = result.routes.find(route => route.id === member.routeId);
    assert.equal(member.endSegmentExclusive, (route.points.length - 1) / 3);
    assert.deepEqual(route.points.slice(member.firstSegment * 3, member.endSegmentExclusive * 3 + 1), bundle.points);
  }
  assert.deepEqual(plan.accept(await layout(plan)), result);
  const { ElkExecutor } = require("@rtepl/pale-scene");
  const executor = new ElkExecutor({ requestBytes: 1e6, resultBytes: 1e6, deadlineMs: 10000,
    totalRequestBytes: 10e6, totalResultBytes: 10e6, totalActiveMs: 60000, oldHeapMb: 128, youngHeapMb: 32 });
  try {
    const first = await executor.run(plan), repeated = await executor.run(plan);
    assert.equal(first.kind, "completed"); assert.equal(repeated.kind, "completed");
    assert.deepEqual(first.geometry, result); assert.deepEqual(repeated.geometry, result);
  } finally { await executor.close(); }
});

test("partial bundles retain maximal runs at membership changes and exact directed ranges", async () => {
  const graph = bundleInput();
  graph.relations.push(relation("third", "a", "b"));
  for (const bundle of graph.bundles) bundle.routeIds.push("third");
  const plan = new GeometryPlan(graph, limits), output = await layout(plan);
  const points = threeSegments(output), changed = structuredClone(points);
  // Three routes share their first segment; only two continue through the middle.
  changed[4].y += 1;
  setChain(output.edges[0].sections[0], points);
  setChain(output.edges[1].sections[0], points);
  setChain(output.edges[2].sections[0], changed);
  const result = plan.accept(output);
  assert.deepEqual(result.routes.map(route => route.points), [points, points, changed]);
  for (const bundle of result.bundles) {
    assert.equal(bundle.kind, "partitioned");
    assert.deepEqual(bundle.unsharedRouteIds, []);
    assert.equal(bundle.corridors.length, 2);
    const common = bundle.corridors.find(corridor => corridor.members.length === 3);
    const pair = bundle.corridors.find(corridor => corridor.members.length === 2);
    assert(common && pair);
    assert.deepEqual(new Set(pair.members.map(member => member.routeId)), new Set(["first", "second"]));
    for (const corridor of bundle.corridors) {
      assert.equal(corridor.containerId, "$scene");
      assert.equal(corridor.routeKind, "cubic-chain");
      for (const member of corridor.members) {
        const route = result.routes.find(route => route.id === member.routeId);
        assert.deepEqual(route.points.slice(member.firstSegment * 3, member.endSegmentExclusive * 3 + 1), corridor.points);
      }
    }
    const [start, end] = bundle.end === "source" ? [0, 1] : [2, 3];
    assert(common.members.every(member => member.firstSegment === start && member.endSegmentExclusive === end));
  }
});

test("a distinct strand does not hide the shared subset or acquire its corridor", async () => {
  const graph = bundleInput();
  graph.relations.push(relation("third", "a", "b"));
  for (const bundle of graph.bundles) bundle.routeIds.push("third");
  const plan = new GeometryPlan(graph, limits), output = await layout(plan);
  const points = threeSegments(output), changed = structuredClone(points);
  changed[1].y += 1e-9; changed[8].y += 1e-9;
  for (let index = 0; index < 3; index++) setChain(output.edges[index].sections[0], index === 2 ? changed : points);
  const result = plan.accept(output);
  for (const bundle of result.bundles) {
    assert.equal(bundle.kind, "partitioned");
    assert.deepEqual(bundle.unsharedRouteIds, ["third"]);
    assert.equal(bundle.corridors.length, 1);
    assert.deepEqual(bundle.corridors[0].points, points);
    assert.equal(bundle.corridors[0].members.length, 2);
  }
  const expected = structuredClone(result);
  result.bundles[0].corridors[0].points[0].x += 10;
  result.bundles[0].unsharedRouteIds.length = 0;
  assert.deepEqual(plan.accept(output), expected, "a previous result cannot retarget a later acceptance");
});

test("real high-degree fan-in keeps subset corridors, original strands and repeatable geometry", async () => {
  const count = 32;
  const graph = { nodes: Array.from({ length: count }, (_, index) => node(`n${index}`)),
    relations: Array.from({ length: count - 1 }, (_, index) => relation(`r${index}`, `n${index}`, `n${count - 1}`)),
    bundles: [{ id: "arrival", end: "target", routeIds: Array.from({ length: count - 1 }, (_, index) => `r${index}`) }] };
  const plan = new GeometryPlan(graph, { ...limits, nodes: 64, ports: 128, relations: 64, points: 8192 });
  const result = plan.accept(await layout(plan));
  assert.equal(result.routes.length, count - 1);
  const bundle = result.bundles[0];
  assert.equal(bundle.kind, "partitioned");
  assert(bundle.corridors.some(corridor => corridor.members.length > 2));
  assert(bundle.unsharedRouteIds.length > 0);
  const covered = new Set(bundle.unsharedRouteIds);
  for (const corridor of bundle.corridors) for (const member of corridor.members) {
    covered.add(member.routeId);
    const route = result.routes.find(route => route.id === member.routeId);
    const stride = route.kind === "line" ? 1 : 3;
    assert.deepEqual(route.points.slice(member.firstSegment * stride, member.endSegmentExclusive * stride + 1), corridor.points);
  }
  assert.deepEqual(covered, new Set(graph.relations.map(route => route.id)));
  assert.deepEqual(plan.accept(await layout(plan)), result);
});

test("two shared subgroups in different valid frames remain separate corridors", async () => {
  const graph = bundleInput();
  graph.nodes = [{ id: "group", width: 40, height: 30, ports: [] }, node("a", "group"), node("b", "group")];
  graph.relations.push(relation("third", "a", "b"), relation("fourth", "a", "b"));
  for (const bundle of graph.bundles) bundle.routeIds.push("third", "fourth");
  const plan = new GeometryPlan(graph, limits), output = await layout(plan);
  const points = threeSegments(output), group = output.children[0];
  for (let index = 0; index < output.edges.length; index++) {
    const edge = output.edges[index];
    setChain(edge.sections[0], structuredClone(points));
    if (index >= 2) {
      edge.container = "$scene";
      const section = edge.sections[0];
      for (const point of [section.startPoint, ...section.bendPoints, section.endPoint]) {
        point.x += group.x; point.y += group.y;
      }
    }
  }
  const result = plan.accept(output);
  for (const bundle of result.bundles) {
    assert.equal(bundle.kind, "partitioned");
    assert.deepEqual(bundle.unsharedRouteIds, []);
    assert.deepEqual(new Set(bundle.corridors.map(corridor => corridor.containerId)), new Set(["group", "$scene"]));
    assert.equal(bundle.corridors.length, 2);
    for (const corridor of bundle.corridors) {
      assert.equal(corridor.members.length, 2);
      for (const member of corridor.members) {
        const route = result.routes.find(route => route.id === member.routeId);
        assert.equal(route.containerId, corridor.containerId);
        assert.deepEqual(route.points, corridor.points);
      }
    }
  }
});

test("a zero-only shared stem does not hide a later nonzero subgroup", async () => {
  const graph = bundleInput();
  graph.relations.push(relation("third", "a", "b"));
  for (const bundle of graph.bundles) bundle.routeIds.push("third");
  const plan = new GeometryPlan(graph, limits), original = await layout(plan);
  const { startPoint: s, endPoint: e } = original.edges[0].sections[0];
  for (const end of ["source", "target"]) {
    const output = structuredClone(original);
    const points = end === "source"
      ? [s, s, s, s, { x: s.x + 4, y: s.y }, { x: e.x - 4, y: e.y }, e]
      : [s, { x: s.x + 4, y: s.y }, { x: e.x - 4, y: e.y }, e, e, e, e];
    const different = structuredClone(points); different[end === "source" ? 4 : 1].y += 1;
    output.edges.forEach((edge, index) => setChain(edge.sections[0], index === 2 ? different : points));
    const result = plan.accept(output), bundle = result.bundles.find(bundle => bundle.end === end);
    assert.equal(bundle.kind, "partitioned");
    assert.deepEqual(bundle.unsharedRouteIds, ["third"]);
    assert.equal(bundle.corridors.length, 1);
    const corridor = bundle.corridors[0];
    assert.equal(corridor.members.length, 2);
    const first = end === "source" ? 1 : 0;
    assert(corridor.members.every(member => member.firstSegment === first && member.endSegmentExclusive === first + 1));
    assert.deepEqual(corridor.points, points.slice(first * 3, first * 3 + 4));
    assert.deepEqual(result.routes[2].points, different);
  }
});
