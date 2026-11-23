"use strict";
const assert = require("node:assert/strict");
const { test, mock } = require("node:test");
const threads = require("node:worker_threads");
const { readFileSync } = require("node:fs");
const { join } = require("node:path");
const { Pale, WorkspaceValueComparison,
  OrganizationRelationLocality, OrganizationFactState } = require("@rtepl/pale");
const { GeometryPlan, GeometryExecutionInput, ElkExecutor, ContactExecutor } = require("@rtepl/pale-scene");
const { OrganizationIntake, OrganizationTopology, OrganizationRouteOrder,
  OrganizationScene, OrganizationSceneCompiler } = require("@rtepl/pale-scene/organization");

const fixture = process.env.PALE_SCENE_TEST_ORGANIZATION_FIXTURE;
assert(fixture, "Supply the existing compiler-backed organization fixture");
const files = ["pale-workspace.toml", "organization.toml", "responsibility.toml", "resolution.lock.toml", "universe.lock.toml"];
const geometryLimits = { nodes: 100, ports: 100, relations: 100, points: 100000, depth: 16, coordinateMagnitude: 1e6 };
const organizationIntake = (owner, selectedQuery, selectedLimits, budget) =>
  new OrganizationIntake(owner, selectedQuery, selectedLimits, budget);
const retainedComparison = comparison => comparison;
function executor(t) {
  const value = new ElkExecutor({ requestBytes: 1e6, resultBytes: 1e6, deadlineMs: 10000,
    totalRequestBytes: 10e6, totalResultBytes: 10e6, totalActiveMs: 60000, oldHeapMb: 128, youngHeapMb: 32 });
  t.after(() => value.close()); return value;
}
function selection(t, mutate = source => source, regional = false, relationId = "relation.collaboration",
  selectSubset = false) {
  const owner = new Pale().compilePortableWorkspace({ rootManifest: files[0], documents: files.map(label => ({ label,
    bytes: label === "organization.toml" ? Buffer.from(mutate(readFileSync(join(fixture, label), "utf8"))) : readFileSync(join(fixture, label)) })) });
  const intake = organizationIntake(owner, { inspected: 2, returned: 2, copiedStringBytes: 65536 },
    { queries: 2000, recordVisits: 5000, sourceVisits: 5000, records: 1000, occurrences: 2000,
      sourceOccurrences: 5000, copiedStringBytes: 32 * 1024 * 1024 });
  t.after(() => intake.close());
  const occurrences = []; let complete = false;
  for (let step = 0; step < 2000; step++) {
    const page = intake.next();
    if (page.kind === "enumeration-exhausted") { complete = true; break; }
    assert.equal(page.kind, "page"); occurrences.push(...page.occurrences);
  }
  assert(complete);
  const completeTopology = new OrganizationTopology(intake, occurrences,
    { records: 1000, appearances: 2000, routeAppearances: 3000, identityCodeUnits: 1000000 });
  const completeRelation = completeTopology.relations.find(
    item => completeTopology.entry(item).row.relationId === relationId);
  assert(completeRelation);
  const completeEndpoints = completeTopology.endpoints(completeRelation);
  assert.equal(completeEndpoints.from.kind, "included"); assert.equal(completeEndpoints.to.kind, "included");
  const selectedOccurrences = [completeRelation, completeEndpoints.from.appearances[0].occurrence,
    completeEndpoints.to.appearances[0].occurrence];
  if (regional) {
    for (const namespace of ["alpha", "beta"]) {
      const scope = completeTopology.appearances.find(item => item.endpoint.kind === "scope"
        && item.endpoint.scope.namespace === namespace);
      assert(scope); selectedOccurrences.push(scope.occurrence);
    }
  }
  const topology = selectSubset
    ? new OrganizationTopology(intake, [...new Set(selectedOccurrences)],
      { records: 1000, appearances: 2000, routeAppearances: 3000, identityCodeUnits: 1000000 })
    : completeTopology;
  const relation = topology.relations.find(item => topology.entry(item).row.relationId === relationId);
  assert(relation);
  const endpoints = topology.endpoints(relation);
  assert.equal(endpoints.from.kind, "included"); assert.equal(endpoints.to.kind, "included");
  const source = endpoints.from.appearances[0], target = endpoints.to.appearances[0];
  const root = topology.appearances.find(item => item.endpoint.kind === "scope" && item.endpoint.scope.namespace === "alpha");
  const outside = topology.appearances.find(item => item.endpoint.kind === "scope" && item.endpoint.scope.namespace === "beta");
  const placements = regional ? [
    { appearance: root, width: 80, height: 40 },
    { appearance: source, parent: root, width: 80, height: 160 },
    { appearance: target, parent: root, width: 80, height: 40 },
    { appearance: outside, width: 80, height: 40 },
  ] : [source, target].map(appearance => ({ appearance, width: 80, height: 40 }));
  if (regional) assert(root && outside);
  const assembly = topology.geometry(placements, [{ relation, source, target }], geometryLimits);
  const compiler = new OrganizationSceneCompiler(intake); compiler.select(topology, assembly);
  t.after(() => compiler.close());
  return { owner, inspection: intake.retainCompilation(), intake, topology, assembly, compiler,
    relation, source, target, root, outside };
}
async function full(t, selected) {
  const result = await selected.compiler.layout(executor(t));
  assert.equal(result.kind, "completed"); return result.scene;
}
const contactLimits = { segments: 100, pairs: 5050, contacts: 500, maximumSeparation: 1e-5, maximumControlDrift: 1e-8 };
function contactExecutor(t) {
  const worker = new ContactExecutor({ requestBytes: 1e6, resultBytes: 1e6, deadlineMs: 10000,
    totalRequestBytes: 10e6, totalResultBytes: 10e6, totalActiveMs: 60000, oldHeapMb: 128, youngHeapMb: 32 });
  t.after(() => worker.close()); return worker;
}
async function contacts(t, selected, scene) {
  const order = scene.orderRoutes(scene.geometry.routes.map(r => r.id));
  const result = await selected.compiler.contacts(scene, contactExecutor(t), order, contactLimits);
  assert.equal(result.kind, "completed"); assert.equal(result.scene, scene); assert.equal(result.analysis.geometry, scene.geometry);
  assert.equal(result.analysis.coverage, "not-established");
  for (const segment of result.analysis.input.segments) assert(scene.geometry.routes.includes(segment.route));
  return result;
}
function choices(before, after, regional = false) {
  const pair = (left, right) => left.map((prior, index) => ({ prior, current: right[index] }));
  const prior = regional ? [before.root, before.outside] : [before.source, before.target];
  const current = regional ? [after.root, after.outside] : [after.source, after.target];
  return { records: pair([...prior.map(item => item.occurrence), ...(regional ? [] : [before.relation])],
    [...current.map(item => item.occurrence), ...(regional ? [] : [after.relation])]),
    appearances: pair(prior, current), routes: regional ? [] : pair(before.assembly.routes, after.assembly.routes), bundles: [] };
}
function finish(preparation, priorIntake) {
  for (let index = 0; index < 100; index++) {
    const charges = priorIntake.charges.queries, result = preparation.step();
    assert(priorIntake.charges.queries - charges <= 1);
    if (result.kind !== "searching" && result.kind !== "record-matched") return result;
  }
  assert.fail("Bounded comparison fixture did not finish");
}

// Exposes cancellation only after the executor's final check. This models an
// abort delivered after worker release but before the compiler continuation.
function cancellationAtCompiler(read) {
  const base = new AbortController().signal;
  let count = 0;
  return new Proxy(base, { get(target, property) {
    if (property === "aborted") return ++count >= read;
    const value = Reflect.get(target, property, target);
    return typeof value === "function" ? value.bind(target) : value;
  } });
}

function nativeDeclarations(source) {
  for (const id of ["gate-front", "gate-loop", "gate-parallel"]) source += `
[[relations]]
relation_id = "relation.${id}"
class = "native_presence"
issuer = "issuer.${id}"
state = "unknown"
[relations.from]
${id === "gate-front" ? 'kind = "scope"\n[relations.from.scope]\nnamespace = "alpha"\nscope_id = "scope"'
    : 'kind = "native_resource"\nresource = "external.same"'}
[relations.to]
kind = "native_resource"
resource = "external.same"
[relations.interval]
start = 1
end_exclusive = 10
`;
  return source;
}
function routed(topology, id) {
  const relation = topology.relations.find(item => topology.entry(item).row.relationId === id); assert(relation);
  const endpoints = topology.endpoints(relation);
  assert.equal(endpoints.from.kind, "included"); assert.equal(endpoints.to.kind, "included");
  return { relation, source: endpoints.from.appearances[0], target: endpoints.to.appearances[0] };
}
function assertGate(scene, gate) {
  assert(scene.assembly.routes.includes(gate.binding));
  assert.equal(gate.binding.witness.kind, "relation");
  assert.equal(gate.binding.witness.locality, OrganizationRelationLocality.NativeBoundary);
  assert.equal(gate.appearance, gate.binding.selection[gate.end]);
  assert.equal(gate.appearance.endpoint.kind, "native-resource");
  assert.equal(gate.route, scene.geometry.routes.find(route => route.id === gate.binding.id));
  assert.equal(gate.port, scene.geometry.ports.find(port => port.id === gate.route[`${gate.end}Port`]));
  assert.equal(gate.port.nodeId, gate.appearance.id);
  assert.equal(scene.topology.entry(gate.binding.witness.occurrence).row.state, OrganizationFactState.Unknown);
  assert(Object.isFrozen(gate) && Object.isFrozen(gate.port) && Object.isFrozen(gate.route));
}

test("declared gates retain both native ends, parallel witnesses and repeated appearances through shared ports", async t => {
  const selected = selection(t, nativeDeclarations), { topology } = selected;
  const loop = routed(topology, "relation.gate-loop"), parallel = routed(topology, "relation.gate-parallel");
  const appearances = topology.appearances.filter(item => item.occurrence === loop.relation);
  assert.equal(appearances.length, 2); assert.deepEqual(appearances[0].endpoint, appearances[1].endpoint);
  const first = { relation: loop.relation, source: appearances[0], target: appearances[1] };
  const routes = [first, first, { ...first, relation: parallel.relation },
    { relation: loop.relation, source: appearances[1], target: appearances[0] }];
  const assembly = topology.geometry(appearances.map(appearance => ({ appearance, width: 80, height: 40 })), routes,
    geometryLimits, [{ end: "source", routeIndices: [0, 1, 2] }, { end: "target", routeIndices: [0, 1, 2] }]);
  selected.compiler.select(topology, assembly);
  // A copied row cannot change the locality already retained by an issued route.
  topology.entry(loop.relation).row.locality = OrganizationRelationLocality.Internal;
  const scene = await full(t, selected), gates = scene.declaredGates;
  assert.equal(gates.length, 8); gates.forEach(gate => assertGate(scene, gate));
  assert.deepEqual(gates.map(gate => gate.end), ["source", "target", "source", "target", "source", "target", "source", "target"]);
  assert.equal(gates[0].port, gates[2].port); assert.equal(gates[2].port, gates[4].port);
  assert.equal(gates[1].port, gates[3].port); assert.equal(gates[3].port, gates[5].port);
  assert.notEqual(gates[0].binding, gates[2].binding);
  assert.notEqual(gates[2].binding.witness.occurrence, gates[4].binding.witness.occurrence);
  assert.notEqual(gates[0].appearance, gates[6].appearance);
  assert(Object.isFrozen(gates)); assert.throws(() => gates.push(gates[0]), TypeError);
  assert.throws(() => { gates[0].end = "target"; }, TypeError);
  assert.throws(() => Object.getOwnPropertyDescriptor(OrganizationScene.prototype, "declaredGates").get.call({}), TypeError);
  assert.throws(() => new OrganizationScene(Symbol(), { ...scene, geometry: { ...scene.geometry }, declaredGates: gates }), { kind: "foreign-scene" });
  selected.compiler.close(); selected.intake.close();
  assert.equal(scene.declaredGates, gates);
  assert(gates.every(gate => scene.topology.entry(gate.binding.witness.occurrence).handle.belongsTo(selected.owner)));
});

test("internal relations, services, derived routes and unrouted native declarations do not acquire gates", async t => {
  const selected = selection(t, nativeDeclarations), { topology } = selected;
  const first = await full(t, selected); assert.deepEqual(first.declaredGates, []);
  assert(first.assembly.unrouted.some(item => topology.entry(item).row.relationId === "relation.gate-front"));
  const routes = [{ relation: selected.relation, source: selected.source, target: selected.target }];
  for (const kind of ["service", "derived-relation"]) {
    const relation = topology.relations.find(item => {
      if (topology.entry(item).row.kind !== kind) return false;
      const endpoints = topology.endpoints(item); return endpoints.from.kind === "included" && endpoints.to.kind === "included";
    });
    assert(relation); const endpoints = topology.endpoints(relation);
    routes.push({ relation, source: endpoints.from.appearances[0], target: endpoints.to.appearances[0] });
  }
  const appearances = [...new Set(routes.flatMap(route => [route.source, route.target]))];
  const assembly = topology.geometry(appearances.map(appearance => ({ appearance, width: 80, height: 40 })), routes, geometryLimits);
  selected.compiler.select(topology, assembly);
  const scene = await full(t, selected); assert.equal(scene.geometry.routes.length, 3);
  assert.deepEqual(scene.declaredGates, []);
  assert.equal(new Set(scene.topology.records.map(item => topology.entry(item).row.kind)).size, 18);
});

test("reused gates bind current declarations and accepted geometry rather than prior associations", async t => {
  const before = selection(t, nativeDeclarations, false, "relation.gate-front");
  const after = selection(t, source => nativeDeclarations(source).replace('issuer = "issuer.gate-front"',
    'issuer = "issuer.gate-updated"'), false, "relation.gate-front");
  const prior = await full(t, before), compare = retainedComparison(new Pale().comparePortableWorkspaces(before.owner, after.owner));
  const pairs = choices(before, after);
  // The external appearance is witnessed by the same relation already paired for its route.
  pairs.records = pairs.records.filter((pair, index, all) => all.findIndex(other => other.prior === pair.prior) === index);
  const result = finish(after.compiler.prepareReuse(prior, compare, pairs), before.intake);
  assert.equal(result.kind, "reused"); const scene = result.scene;
  assert.equal(scene.declaredGates.length, 1); assert.equal(prior.declaredGates.length, 1);
  const gate = scene.declaredGates[0]; assertGate(scene, gate);
  assert.equal(gate.end, "target"); assert.notEqual(gate, prior.declaredGates[0]);
  assert.notEqual(gate.binding, prior.declaredGates[0].binding); assert.notEqual(gate.port, prior.declaredGates[0].port);
  assert.equal(gate.binding, after.assembly.routes[0]);
  assert(scene.topology.entry(gate.binding.witness.occurrence).handle.belongsTo(after.owner));
  assert(scene.correspondence.evidence.some(item => item.correspondence.state.values === WorkspaceValueComparison.Changed));
  after.compiler.close(); before.compiler.close(); assert.equal(scene.declaredGates[0], gate);
});

test("regional gates use the complete current scene frame and retain outside declaration custody", async t => {
  const before = selection(t, nativeDeclarations, true), after = selection(t, nativeDeclarations, true);
  function selectRegion(selected) {
    const outer = routed(selected.topology, "relation.gate-front"); assert.equal(outer.source, selected.root);
    const placements = [{ appearance: selected.root, width: 80, height: 40 },
      { appearance: selected.source, parent: selected.root, width: 80, height: 160 },
      { appearance: selected.target, parent: selected.root, width: 80, height: 40 },
      { appearance: selected.outside, width: 80, height: 40 },
      { appearance: outer.target, width: 80, height: 40 }];
    selected.assembly = selected.topology.geometry(placements,
      [{ relation: selected.relation, source: selected.source, target: selected.target }, outer], geometryLimits);
    selected.compiler.select(selected.topology, selected.assembly); return outer;
  }
  const oldOuter = selectRegion(before), newOuter = selectRegion(after), prior = await full(t, before);
  const pairs = choices(before, after, true);
  pairs.records.push({ prior: oldOuter.relation, current: newOuter.relation });
  pairs.appearances.push({ prior: oldOuter.target, current: newOuter.target });
  pairs.routes.push({ prior: before.assembly.routes[1], current: after.assembly.routes[1] });
  const ready = finish(after.compiler.prepareRegion(prior, retainedComparison(new Pale().comparePortableWorkspaces(before.owner, after.owner)),
    { prior: before.root, current: after.root }, pairs), before.intake);
  assert.equal(ready.kind, "region-ready"); const result = await ready.layout(executor(t));
  assert.equal(result.kind, "completed"); const scene = result.scene;
  assert.equal(scene.correspondence.kind, "regional"); assert.equal(scene.declaredGates.length, 1);
  const gate = scene.declaredGates[0]; assertGate(scene, gate);
  assert.equal(gate.appearance, newOuter.target); assert.equal(gate.binding, after.assembly.routes[1]);
  assert(!scene.execution.geometry.ports.includes(gate.port)); assert(!scene.execution.geometry.routes.includes(gate.route));
  assert.equal(scene.execution.geometry.nodes.some(node => node.id === gate.port.nodeId), false);
  assert.deepEqual(gate.port.bounds, prior.declaredGates[0].port.bounds);
  assert.notEqual(gate, prior.declaredGates[0]);
  assert(scene.topology.entry(gate.binding.witness.occurrence).handle.belongsTo(after.owner));
});

test("issued execution view uses the exact original plan and options despite public method overrides", async t => {
  const selected = selection(t), plan = selected.assembly.plan, view = plan.executionInput();
  const options = plan.elkInput().layoutOptions;
  assert.deepEqual(view.algorithm.rootOptions, options);
  assert.equal(view.algorithm.version, "0.12.0"); assert(view.belongsTo(plan));
  plan.elkInput = () => { throw Error("overridden input"); };
  plan.accept = () => { throw Error("overridden acceptance"); };
  assert.throws(() => { view.algorithm.rootOptions["elk.algorithm"] = "other"; }, TypeError);
  const result = await executor(t).run(view);
  assert.equal(result.kind, "completed"); assert(plan.hasAccepted(result.geometry));
  assert(!plan.hasAccepted({ ...result.geometry }));
  assert.throws(() => new GeometryExecutionInput(Symbol(), plan, () => ({}), () => ({})));
  assert.deepEqual(await executor(t).run(Object.create(GeometryExecutionInput.prototype)), { kind: "failed", reason: "geometry" });
});

test("full scene retains exact supplied source, all families and route witnesses after caller close", async t => {
  const selected = selection(t), scene = await full(t, selected);
  const equalBytes = selection(t);
  assert.equal(scene.belongsTo(selected.inspection), true);
  assert.equal(scene.belongsTo(equalBytes.inspection), false);
  assert.equal(scene.source.workspaceId, selected.owner.workspaceId);
  assert.equal(scene.source.semanticIrDigest, selected.owner.semanticIrDigest);
  assert.equal(scene.source.principalUniverseDigest, selected.owner.principalUniverseDigest);
  assert.equal(scene.source.admissionEvidenceDigest, selected.owner.admissionEvidenceDigest);
  assert.equal(scene.source.kind, "supplied-input");
  assert.deepEqual(scene.lens, { kind: "organization-structural", subject: "scene-appearance",
    grouping: "explicit-presentation-selection", proximity: "layout-constraints-only",
    routes: "selected-sdk-witnesses" });
  assert(Object.isFrozen(scene.lens));
  assert.throws(() => { scene.lens.proximity = "semantic-similarity"; }, TypeError);
  assert.equal(scene.correspondence.kind, "full");
  assert.equal(scene.execution.kind, "full");
  assert.equal(scene.execution.input, scene.constraints);
  assert.equal(scene.execution.geometry, scene.geometry);
  assert(scene.constraints.belongsTo(selected.assembly.plan));
  assert.equal(scene.assembly, selected.assembly); assert.equal(scene.topology, selected.topology);
  assert.equal(scene.coverage, selected.topology.coverage);
  assert.deepEqual(scene.coverage, selected.topology.coverage);
  assert.deepEqual(scene.coverage.enumeration, { kind: "exhausted" });
  assert.deepEqual(scene.coverage.selection, { kind: "all-enumerated-occurrences" });
  assert.equal(scene.geometry.routes[0].id, scene.assembly.routes[0].id);
  assert.equal(scene.assembly.routes[0].selection.relation, selected.relation);
  const annotated = await contacts(t, selected, scene);
  selected.compiler.close(); selected.intake.close();
  assert.equal(annotated.scene.source, scene.source); assert.equal(annotated.analysis.geometry, scene.geometry);
  assert(scene.topology.entry(selected.relation).handle.belongsTo(selected.owner));
  assert.equal(new Set(scene.topology.records.map(item => scene.topology.entry(item).row.kind)).size, 18);
  assert(Object.isFrozen(scene) && Object.isFrozen(scene.source));
});

test("scene source reads the retained native compilation instead of shadowed JavaScript properties", async t => {
  const selected = selection(t);
  const expected = {
    workspaceId: selected.owner.workspaceId,
    principalUniverseDigest: selected.owner.principalUniverseDigest,
    semanticIrDigest: selected.owner.semanticIrDigest,
    admissionEvidenceDigest: selected.owner.admissionEvidenceDigest,
  };
  for (const property of Object.keys(expected)) {
    Object.defineProperty(selected.owner, property, { configurable: true, value: `shadowed-${property}` });
  }
  assert.equal(selected.owner.semanticIrDigest, "shadowed-semanticIrDigest");
  const compiler = new OrganizationSceneCompiler(selected.intake);
  compiler.select(selected.topology, selected.assembly);
  t.after(() => compiler.close());
  const result = await compiler.layout(executor(t));
  assert.equal(result.kind, "completed");
  assert.deepEqual(result.scene.source, { kind: "supplied-input", ...expected });
  assert.equal(result.scene.belongsTo(selected.inspection), true);
});

test("full, reused and regional scenes use one qualified structural lens", async t => {
  const before = selection(t), after = selection(t, undefined, false, "relation.collaboration", true);
  const prior = await full(t, before), comparison = retainedComparison(new Pale().comparePortableWorkspaces(before.owner, after.owner));
  const reused = finish(after.compiler.prepareReuse(prior, comparison, choices(before, after)), before.intake);
  assert.equal(reused.kind, "reused"); assert.equal(reused.scene.lens, prior.lens);
  assert.equal(reused.scene.coverage, after.topology.coverage);
  assert.notEqual(reused.scene.coverage, prior.coverage);
  assert.deepEqual(prior.coverage.selection, { kind: "all-enumerated-occurrences" });
  assert.deepEqual(reused.scene.coverage.selection, { kind: "subset" });
  const regionalBefore = selection(t, undefined, true);
  const regionalAfter = selection(t, undefined, true, "relation.collaboration", true);
  const regionalPrior = await full(t, regionalBefore);
  const regional = finish(
    regionalAfter.compiler.prepareRegion(regionalPrior,
      retainedComparison(new Pale().comparePortableWorkspaces(regionalBefore.owner, regionalAfter.owner)),
      { prior: regionalBefore.root, current: regionalAfter.root }, choices(regionalBefore, regionalAfter, true)),
    regionalBefore.intake,
  );
  assert.equal(regional.kind, "region-ready");
  const completed = await regional.layout(executor(t)); assert.equal(completed.kind, "completed");
  assert.equal(completed.scene.lens, regionalPrior.lens);
  assert.equal(completed.scene.lens.proximity, "layout-constraints-only");
  assert.equal(completed.scene.coverage, regionalAfter.topology.coverage);
  assert.notEqual(completed.scene.coverage, regionalPrior.coverage);
  assert.deepEqual(regionalPrior.coverage.selection, { kind: "all-enumerated-occurrences" });
  assert.deepEqual(completed.scene.coverage.selection, { kind: "subset" });
});

test("route order is complete, scene-bound and retains exact route witnesses", async t => {
  const selected = selection(t), scene = await full(t, selected), ids = scene.geometry.routes.map(route => route.id);
  const order = scene.orderRoutes(ids);
  ids[0] = "foreign";
  assert(order.belongsTo(scene));
  assert.equal(order.layering, "back-to-front");
  assert(Object.isFrozen(order) && Object.isFrozen(order.routes) && Object.isFrozen(order.routes[0]));
  assert.deepEqual(order.routes.map(item => item.route.id), scene.geometry.routes.map(route => route.id));
  assert.deepEqual(order.routes.map(item => item.depth), order.routes.map((_, index) => index));
  assert(order.encodedOutputBytes > 0);
  for (const item of order.routes) {
    assert.equal(item.binding, scene.assembly.routes.find(binding => binding.id === item.route.id));
    assert(scene.geometry.routes.includes(item.route));
  }
  assert.throws(() => scene.orderRoutes([]), { kind: "invalid-route-order" });
  assert.throws(() => scene.orderRoutes(["foreign"]), { kind: "invalid-route-order" });
  assert.throws(() => scene.orderRoutes(new Array(ids.length)), { kind: "invalid-route-order" });
  assert.throws(() => new OrganizationRouteOrder(Symbol(), scene, []), TypeError);
  const equalBytes = selection(t), other = await full(t, equalBytes);
  assert.equal(order.belongsTo(other), false);
  selected.compiler.close(); selected.intake.close();
  assert(order.belongsTo(scene)); assert.equal(order.routes[0].binding, scene.assembly.routes[0]);
});

test("selection rejects equal-byte foreign owners and copied assemblies without replacing valid state", async t => {
  const first = selection(t), second = selection(t);
  assert.equal(first.owner.semanticIrDigest, second.owner.semanticIrDigest);
  assert.throws(() => first.compiler.select(second.topology, second.assembly), { kind: "foreign-assembly" });
  assert.throws(() => first.compiler.select(first.topology, { ...first.assembly }), { kind: "foreign-assembly" });
  const bare = new OrganizationSceneCompiler(first.intake); t.after(() => bare.close());
  await assert.rejects(bare.layout(executor(t)), { kind: "unselected" });
  assert.throws(() => new OrganizationScene(Symbol(), {}), { kind: "foreign-scene" });
  const scene = await full(t, first); assert.equal(scene.assembly, first.assembly);
  assert.throws(() => first.compiler.prepareReuse(Object.create(OrganizationScene.prototype), {}, {}), TypeError);
});

test("replacement and cancellation cannot publish an older full layout and preserve executor lifecycle", async t => {
  const selected = selection(t), worker = executor(t);
  const pending = selected.compiler.layout(worker);
  assert.equal(worker.status, "running");
  selected.compiler.select(selected.topology, selected.assembly);
  assert.deepEqual(await selected.compiler.layout(worker), { kind: "unavailable", reason: "busy" });
  assert.deepEqual(await pending, { kind: "stopped", reason: "superseded" });
  assert.equal(worker.status, "idle");
  const abort = new AbortController(), cancelled = selected.compiler.layout(worker, abort.signal); abort.abort();
  assert.deepEqual(await cancelled, { kind: "stopped", reason: "cancelled" });
  assert.equal(worker.status, "idle");
  assert.equal((await selected.compiler.layout(worker)).kind, "completed");
  const Original = threads.Worker;
  let exited = false;
  const interception = mock.method(threads, "Worker", class extends Original {
    constructor(...args) {
      super(...args);
      const terminate = this.terminate.bind(this);
      this.terminate = async () => { await terminate(); exited = true; throw Error("Controlled acknowledgement failure"); };
    }
  });
  const failed = new ElkExecutor({ requestBytes: 1e6, resultBytes: 1e6, deadlineMs: 10000,
    totalRequestBytes: 10e6, totalResultBytes: 10e6, totalActiveMs: 60000, oldHeapMb: 128, youngHeapMb: 32 });
  try {
    const stale = selected.compiler.layout(failed); selected.compiler.select(selected.topology, selected.assembly);
    assert.deepEqual(await stale, { kind: "failed", reason: "termination" });
    assert(exited); assert.equal(failed.status, "stopping");
    await assert.rejects(failed.close(), /termination failed/);
  } finally { interception.mock.restore(); }
});

test("late cancellation cannot publish full, contact or regional results", async t => {
  const selected = selection(t), worker = executor(t);
  assert.deepEqual(await selected.compiler.layout(worker, cancellationAtCompiler(4)),
    { kind: "stopped", reason: "cancelled" });
  const scene = await full(t, selected), order = scene.orderRoutes(scene.geometry.routes.map(route => route.id));
  assert.deepEqual(await selected.compiler.contacts(scene, contactExecutor(t), order, contactLimits,
    cancellationAtCompiler(5)), { kind: "stopped", reason: "cancelled" });

  const before = selection(t, undefined, true), after = selection(t, undefined, true), prior = await full(t, before);
  const ready = finish(after.compiler.prepareRegion(prior, retainedComparison(new Pale().comparePortableWorkspaces(before.owner, after.owner)),
    { prior: before.root, current: after.root }, choices(before, after, true)), before.intake);
  assert.equal(ready.kind, "region-ready");
  assert.deepEqual(await ready.layout(executor(t), cancellationAtCompiler(4)),
    { kind: "stopped", reason: "cancelled" });
});

test("reused scene keeps SDK Changed evidence and independent current and prior sources without relayout", async t => {
  const before = selection(t), after = selection(t, source => source.replace('resources = ["resource.role-a"]', 'resources = ["resource.changed"]'));
  const prior = await full(t, before), comparison = retainedComparison(new Pale().comparePortableWorkspaces(before.owner, after.owner));
  const result = finish(after.compiler.prepareReuse(prior, comparison, choices(before, after)), before.intake);
  assert.equal(result.kind, "reused");
  const scene = result.scene, evidence = scene.correspondence.evidence;
  assert.equal(scene.correspondence.kind, "reused"); assert.equal(scene.correspondence.comparison, comparison);
  assert.deepEqual(scene.execution, { kind: "reused" });
  assert(scene.constraints.belongsTo(after.assembly.plan));
  assert.equal(scene.correspondence.priorSource, prior.source);
  assert(evidence.some(item => item.correspondence.state.values === WorkspaceValueComparison.Changed));
  assert(evidence.every(item => item.correspondence.selected.belongsTo(before.owner)
    && item.correspondence.state.other.handle.belongsTo(after.owner)));
  await contacts(t, after, scene);
  before.intake.close(); after.intake.close();
  before.compiler.close(); after.compiler.close();
  assert.equal(scene.correspondence.comparison.receiptDigest, comparison.receiptDigest);
  assert(evidence.every(item => item.correspondence.belongsTo(comparison)));
  assert(scene.topology.entry(after.relation).handle.belongsTo(after.owner));
  assert.equal(scene.geometry.routes[0].id, after.assembly.routes[0].id);
});

test("reuse keeps the SDK pair check and closed preparation does no further inspection", async t => {
  const before = selection(t), after = selection(t), third = selection(t), prior = await full(t, before);
  const wrong = retainedComparison(new Pale().comparePortableWorkspaces(third.owner, after.owner));
  assert.equal(wrong.compares(before.inspection, after.inspection), false);
  assert.equal(wrong.compares(third.inspection, after.inspection), true);
  assert.throws(() => after.compiler.prepareReuse(prior, wrong, { records: [], appearances: [], routes: [], bundles: [] }),
    { kind: "comparison-mismatch" });
  assert.throws(() => after.compiler.prepareRegion(prior, wrong,
    { prior: before.source, current: after.source }, { records: [], appearances: [], routes: [], bundles: [] }),
    { kind: "comparison-mismatch" });
  const charges = before.intake.charges.queries;
  const valid = after.compiler.prepareReuse(prior, retainedComparison(new Pale().comparePortableWorkspaces(before.owner, after.owner)), choices(before, after));
  after.compiler.select(after.topology, after.assembly);
  assert.deepEqual(valid.step(), { kind: "closed" }); assert.equal(before.intake.charges.queries, charges);
});

test("regional scene retains current source and exact outside correspondence through local layout", async t => {
  const before = selection(t, undefined, true), after = selection(t, undefined, true), prior = await full(t, before);
  const comparison = retainedComparison(new Pale().comparePortableWorkspaces(before.owner, after.owner));
  const ready = finish(after.compiler.prepareRegion(prior, comparison, { prior: before.root, current: after.root },
    choices(before, after, true)), before.intake);
  assert.equal(ready.kind, "region-ready");
  after.assembly.plan.accept = () => { throw Error("overridden outer acceptance"); };
  const result = await ready.layout(executor(t)); assert.equal(result.kind, "completed");
  assert.equal(result.scene.correspondence.kind, "regional");
  assert.equal(result.scene.execution.kind, "regional");
  assert(result.scene.constraints.belongsTo(after.assembly.plan));
  assert.equal(result.scene.execution.input.belongsTo(after.assembly.plan), false);
  assert.notEqual(result.scene.execution.geometry, result.scene.geometry);
  assert.equal(result.scene.execution.geometry.nodes.some(node => node.id === after.outside.id), false);
  assert(result.scene.geometry.nodes.some(node => node.id === after.outside.id));
  assert.equal(result.scene.correspondence.regionId, after.root.id);
  assert.equal(result.scene.correspondence.evidence.length, 2);
  assert.equal(result.scene.correspondence.priorSource, prior.source);
  assert.equal(result.scene.source.semanticIrDigest, after.owner.semanticIrDigest);
  assert.deepEqual(result.scene.geometry.nodes.find(node => node.id === after.outside.id).bounds,
    prior.geometry.nodes.find(node => node.id === before.outside.id).bounds);
  assert(result.scene.topology.entry(after.relation).handle.belongsTo(after.owner));
  await contacts(t, after, result.scene);
});

test("scene contacts fence exact scene custody, replacement, cancellation and close without losing prior answers", async t => {
  const selected = selection(t), scene = await full(t, selected), foreign = selection(t), other = await full(t, foreign);
  const worker = contactExecutor(t), order = scene.orderRoutes(scene.geometry.routes.map(r => r.id));
  await assert.rejects(selected.compiler.contacts(other, worker, order, contactLimits), { kind: "foreign-scene" });
  const otherOrder = other.orderRoutes(other.geometry.routes.map(route => route.id));
  await assert.rejects(selected.compiler.contacts(scene, worker, otherOrder, contactLimits), { kind: "foreign-route-order" });
  const preCancelled = new AbortController(); preCancelled.abort();
  const unreadOrder = Object.create(OrganizationRouteOrder.prototype);
  const unreadLimits = new Proxy(contactLimits, { get() { throw Error("cancelled contact limits were read"); } });
  assert.deepEqual(await selected.compiler.contacts(scene, worker, unreadOrder, unreadLimits, preCancelled.signal),
    { kind: "stopped", reason: "cancelled" });
  assert.deepEqual(worker.charges, { requestBytes: 0, resultBytes: 0, activeMs: 0 });
  const abort = new AbortController(), cancelled = selected.compiler.contacts(scene, worker, order, contactLimits, abort.signal); abort.abort();
  assert.deepEqual(await cancelled, { kind: "stopped", reason: "cancelled" });
  const pending = selected.compiler.contacts(scene, worker, order, contactLimits);
  selected.compiler.select(selected.topology, selected.assembly);
  assert.deepEqual(await pending, { kind: "stopped", reason: "superseded" });
  assert.equal(scene.topology.entry(selected.relation).handle.belongsTo(selected.owner), true);
  const current = await full(t, selected), currentOrder = current.orderRoutes(current.geometry.routes.map(route => route.id));
  const closing = selected.compiler.contacts(current, worker, currentOrder, contactLimits);
  selected.compiler.close(); assert.deepEqual(await closing, { kind: "stopped", reason: "superseded" });
  assert.equal(current.geometry.routes[0].id, selected.assembly.routes[0].id);
});

test("regional cancellation and stale replacement preserve typed stops and prevent scene publication", async t => {
  const before = selection(t, undefined, true), after = selection(t, undefined, true), prior = await full(t, before);
  const preparation = after.compiler.prepareRegion(prior, retainedComparison(new Pale().comparePortableWorkspaces(before.owner, after.owner)),
    { prior: before.root, current: after.root }, choices(before, after, true));
  const ready = finish(preparation, before.intake); assert.equal(ready.kind, "region-ready");
  const abort = new AbortController(); abort.abort();
  assert.deepEqual(await ready.layout(executor(t), abort.signal), { kind: "stopped", reason: "cancelled" });
  const pending = ready.layout(executor(t)); after.compiler.select(after.topology, after.assembly);
  assert.deepEqual(await pending, { kind: "stopped", reason: "superseded" });
  preparation.close(); assert.deepEqual(preparation.step(), { kind: "closed" });
  assert.deepEqual(await ready.layout(executor(t)), { kind: "stopped", reason: "superseded" });
  after.compiler.close(); assert.throws(() => after.compiler.select(after.topology, after.assembly), { kind: "closed" });
});
