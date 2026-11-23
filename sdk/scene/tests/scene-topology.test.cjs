const assert = require("node:assert/strict");
const { test } = require("node:test");
const { readFileSync } = require("node:fs");
const { join } = require("node:path");
const { Pale, WorkspaceValueComparison,
  OrganizationRelationLocality } = require("@rtepl/pale");
const { OrganizationIntake, OrganizationTopology, OrganizationSceneCompiler } = require("@rtepl/pale-scene/organization");
const { ElkExecutor, ContactExecutor } = require("@rtepl/pale-scene");

// Same admitted SDK fixture as the intake laws; no fabricated handles or DTOs.
const fixture = process.env.PALE_SCENE_TEST_ORGANIZATION_FIXTURE;
assert(fixture, "Supply the canonical organization fixture before running this native test");
const files = ["pale-workspace.toml", "organization.toml", "responsibility.toml", "resolution.lock.toml", "universe.lock.toml"];
const limits = { records: 1000, appearances: 2000, routeAppearances: 3000, identityCodeUnits: 1000000 };
const geometryLimits = { nodes: 2000, ports: 6000, relations: 3000, points: 100000, depth: 16, coordinateMagnitude: 1000000 };
const organizationIntake = (owner, selectedQuery, selectedLimits, budget) =>
  new OrganizationIntake(owner, selectedQuery, selectedLimits, budget);
const retainedComparison = comparison => comparison;
function setup(t, mutate = value => value) {
  const owner = new Pale().compilePortableWorkspace({ rootManifest: files[0], documents: files.map(label => ({ label,
    bytes: label === "organization.toml" ? Buffer.from(mutate(readFileSync(join(fixture, label), "utf8")))
      : readFileSync(join(fixture, label)) })) });
  const intake = organizationIntake(owner, { inspected: 2, returned: 2, copiedStringBytes: 65536 },
    { queries: 2000, recordVisits: 5000, sourceVisits: 5000, records: 1000, occurrences: 2000,
      sourceOccurrences: 5000, copiedStringBytes: 32 * 1024 * 1024 });
  t.after(() => intake.close());
  const occurrences = [];
  for (let calls = 0; calls < 2000; calls++) {
    const page = intake.next();
    if (page.kind === "enumeration-exhausted") return { owner, intake, occurrences };
    assert.equal(page.kind, "page");
    for (const occurrence of page.occurrences) occurrences.push(occurrence);
  }
  assert.fail("The bounded fixture did not exhaust");
}
const placements = appearances => appearances.map(appearance => ({ appearance, width: 80, height: 40 }));
function selectedRoute(topology, relation, sourceIndex = 0, targetIndex = 0) {
  const endpoints = topology.endpoints(relation);
  assert.equal(endpoints.from.kind, "included"); assert.equal(endpoints.to.kind, "included");
  const source = endpoints.from.appearances[sourceIndex], target = endpoints.to.appearances[targetIndex];
  assert(source && target);
  return { relation, source, target };
}
const findRelation = (topology, id) => {
  const found = topology.relations.find(occurrence => topology.entry(occurrence).row.relationId === id);
  assert(found); return found;
};
const findService = (topology, namespace) => {
  const found = topology.relations.find(occurrence => {
    const row = topology.entry(occurrence).row;
    return row.kind === "service" && row.contract.namespace === namespace;
  });
  assert(found); return found;
};
// Change only the first canonical service fixture. Nested tables stay attached.
function changeAlphaService(source, change) {
  const start = source.indexOf("[[services]]"), end = source.indexOf("[[services]]", start + 1);
  assert(start >= 0 && end > start);
  return source.slice(0, start) + change(source.slice(start, end)) + source.slice(end);
}
function serviceScene(data, reverse = false, namespace = "alpha") {
  const topology = new OrganizationTopology(data.intake, reverse ? [...data.occurrences].reverse() : data.occurrences, limits);
  const service = findService(topology, namespace), route = selectedRoute(topology, service);
  const appearances = [route.source, route.target];
  return { topology, appearances, records: [service, ...appearances.map(item => item.occurrence)],
    geometry: topology.geometry(placements(appearances), [route], geometryLimits) };
}

// Explicit fixture selections are test inputs, not a production identity matcher.
function reuseScene(data, { reverse = false, repeated = false, width = 80, relationId = "relation.collaboration", bundle = false } = {}) {
  const occurrences = reverse ? [...data.occurrences].reverse() : [...data.occurrences];
  if (repeated) {
    const role = data.occurrences.find(item => data.intake.entry(item).row.kind === "role"
      && data.intake.entry(item).row.role.roleId === "accountable-a");
    const extra = data.intake.repeat(role); assert.equal(extra.inclusion, "additional-occurrence"); occurrences.push(extra);
  }
  const topology = new OrganizationTopology(data.intake, occurrences, limits);
  const relation = findRelation(topology, relationId), first = selectedRoute(topology, relation);
  const routes = repeated ? [first, selectedRoute(topology, relation, 1)] : bundle ? [first, first] : [first];
  const appearances = [...new Set(routes.flatMap(route => [route.source, route.target]))];
  const records = [...new Set([...appearances.map(item => item.occurrence), relation])];
  const geometry = topology.geometry(appearances.map(appearance => ({ appearance, width, height: 40 })), routes,
    geometryLimits, bundle ? [{ end: "target", routeIndices: [0, 1] }] : []);
  return { topology, geometry, appearances, records };
}
function visualPairs(prior, current) {
  const pair = (left, right) => left.map((value, index) => ({ prior: value, current: right[index] }));
  return { records: pair(prior.records, current.records), appearances: pair(prior.appearances, current.appearances),
    routes: pair(prior.geometry.routes, current.geometry.routes), bundles: pair(prior.geometry.bundles, current.geometry.bundles) };
}
async function acceptedGeometry(t, scene) {
  const executor = new ElkExecutor({ requestBytes: 1e6, resultBytes: 1e6, deadlineMs: 10000,
    totalRequestBytes: 10e6, totalResultBytes: 10e6, totalActiveMs: 60000, oldHeapMb: 128, youngHeapMb: 32 });
  t.after(() => executor.close());
  const completed = await executor.run(scene.geometry.plan); assert.equal(completed.kind, "completed");
  return completed.geometry;
}
function finishReuse(preparation, intake) {
  for (let count = 0; count < 100; count++) {
    const before = intake.charges.queries, result = preparation.step();
    assert(intake.charges.queries - before <= 1, "one step must not automatically exhaust SDK pages");
    if (result.kind !== "searching" && result.kind !== "record-matched") return result;
  }
  assert.fail("The bounded correspondence fixture did not finish");
}

const regionFixture = source => `${source}
[[relations]]
relation_id = "relation.region-gate"
class = "native_presence"
issuer = "issuer.region-gate"
state = "unknown"
[relations.from]
kind = "scope"
[relations.from.scope]
namespace = "alpha"
scope_id = "scope"
[relations.to]
kind = "native_resource"
resource = "external.region-gate"
[relations.interval]
start = 1
end_exclusive = 10
`;
function regionScene(data, reverse = false) {
  const topology = new OrganizationTopology(data.intake, reverse ? [...data.occurrences].reverse() : data.occurrences, limits);
  const inner = selectedRoute(topology, findRelation(topology, "relation.collaboration"));
  const outer = selectedRoute(topology, findRelation(topology, "relation.region-gate"));
  const root = outer.source;
  const geometry = topology.geometry([
    { appearance: root, width: 80, height: 40 },
    { appearance: inner.source, parent: root, width: 80, height: 160 },
    { appearance: inner.target, parent: root, width: 80, height: 40 },
    { appearance: outer.target, width: 80, height: 40 },
  ], [inner, outer], geometryLimits);
  return { topology, geometry, root, inner, outer };
}
function regionPairs(prior, current) {
  return { records: [{ prior: prior.root.occurrence, current: current.root.occurrence },
    { prior: prior.outer.relation, current: current.outer.relation }],
    appearances: [{ prior: prior.root, current: current.root }, { prior: prior.outer.target, current: current.outer.target }],
    routes: [{ prior: prior.geometry.routes[1], current: current.geometry.routes[1] }], bundles: [] };
}

test("actual SDK region update replaces interior coverage while retaining proved outside geometry", async t => {
  const before = setup(t, regionFixture);
  const after = setup(t, source => {
    // Collaboration is local. Declare a new role in the same organization instead
    // of pointing this internal relation into beta's different organization.
    const role = source.indexOf('role_id = "accountable-b"'); assert(role >= 0);
    const start = source.lastIndexOf("[[roles]]", role), end = source.indexOf("[[roles]]", role);
    assert(start >= 0 && end > start);
    const replacement = source.slice(start, end).replace('role_id = "accountable-b"', 'role_id = "regional-replacement"');
    const changed = source.replace('[relations.to.role]\nnamespace = "alpha"\nrole_id = "accountable-b"',
      '[relations.to.role]\nnamespace = "alpha"\nrole_id = "regional-replacement"');
    assert.notEqual(changed, source);
    return regionFixture(`${changed}\n${replacement}`);
  });
  const prior = regionScene(before), current = regionScene(after, true), accepted = await acceptedGeometry(t, prior);
  assert.notDeepEqual(prior.inner.target.endpoint, current.inner.target.endpoint);
  const comparison = retainedComparison(new Pale().comparePortableWorkspaces(before.owner, after.owner));
  const preparation = current.topology.prepareRegion(prior.topology, prior.geometry, current.geometry, accepted,
    comparison, { prior: prior.root, current: current.root }, regionPairs(prior, current));
  const ready = finishReuse(preparation, before.intake); assert.equal(ready.kind, "region-ready");
  assert.equal(ready.evidence.length, 2);
  assert(ready.evidence.every(item => item.correspondence.state.other.handle.belongsTo(after.owner)));
  const executor = new ElkExecutor({ requestBytes: 1e6, resultBytes: 1e6, deadlineMs: 10000,
    totalRequestBytes: 10e6, totalResultBytes: 10e6, totalActiveMs: 60000, oldHeapMb: 128, youngHeapMb: 32 });
  t.after(() => executor.close());
  const local = await executor.run(ready.plan); assert.equal(local.kind, "completed");
  const result = ready.complete(local.geometry); assert.equal(result.kind, "region-composed");
  const oldOutside = accepted.nodes.find(node => node.id === prior.outer.target.id);
  assert.deepEqual(result.geometry.nodes.find(node => node.id === current.outer.target.id).bounds, oldOutside.bounds);
  const selected = current.topology.entry(current.inner.target.occurrence);
  assert(selected.handle.belongsTo(after.owner)); assert.equal(selected.row.role.roleId, "regional-replacement");
  assert.equal(new Set(current.topology.records.map(item => current.topology.entry(item).row.kind)).size, 18);
  assert(result.geometry.nodes.some(node => node.id === current.inner.target.id));
  const fresh = await executor.run(current.geometry.plan); assert.equal(fresh.kind, "completed");
  assert.deepEqual(result.geometry.routes.map(route => [route.id, route.sourcePort, route.targetPort]),
    fresh.geometry.routes.map(route => [route.id, route.sourcePort, route.targetPort]));
});

test("regional reuse requires outside/root proof and respects close before another SDK page", async t => {
  const before = setup(t, regionFixture), after = setup(t, regionFixture);
  const prior = regionScene(before), current = regionScene(after), accepted = await acceptedGeometry(t, prior);
  const comparison = retainedComparison(new Pale().comparePortableWorkspaces(before.owner, after.owner)), missing = regionPairs(prior, current);
  missing.records.pop();
  const incomplete = current.topology.prepareRegion(prior.topology, prior.geometry, current.geometry, accepted,
    comparison, { prior: prior.root, current: current.root }, missing);
  assert.equal(incomplete.step().reason, "incomplete-pairing");
  const preparation = current.topology.prepareRegion(prior.topology, prior.geometry, current.geometry, accepted,
    comparison, { prior: prior.root, current: current.root }, regionPairs(prior, current));
  const calls = before.intake.charges.queries; preparation.close(); assert.equal(preparation.step().kind, "closed");
  assert.equal(before.intake.charges.queries, calls);
  assert(current.topology.entry(current.root.occurrence).handle.belongsTo(after.owner));
});

test("a cancelled local worker leaves both semantic scenes and a retryable preparation result intact", async t => {
  const before = setup(t, regionFixture), after = setup(t, regionFixture), prior = regionScene(before), current = regionScene(after);
  const accepted = await acceptedGeometry(t, prior), comparison = retainedComparison(new Pale().comparePortableWorkspaces(before.owner, after.owner));
  const ready = finishReuse(current.topology.prepareRegion(prior.topology, prior.geometry, current.geometry, accepted,
    comparison, { prior: prior.root, current: current.root }, regionPairs(prior, current)), before.intake);
  assert.equal(ready.kind, "region-ready");
  const executor = new ElkExecutor({ requestBytes: 1e6, resultBytes: 1e6, deadlineMs: 10000,
    totalRequestBytes: 10e6, totalResultBytes: 10e6, totalActiveMs: 60000, oldHeapMb: 128, youngHeapMb: 32 });
  t.after(() => executor.close());
  const abort = new AbortController(); abort.abort();
  const stopped = await executor.run(ready.plan, abort.signal); assert.equal(stopped.kind, "stopped");
  assert(current.topology.entry(current.root.occurrence).handle.belongsTo(after.owner));
  const retried = await executor.run(ready.plan); assert.equal(retried.kind, "completed");
  assert.equal(ready.complete(retried.geometry).kind, "region-composed");
});

test("an interior route pair proves only its retained root port, not the changed external reference", async t => {
  const fixtureWithTwoGates = source => {
    const first = regionFixture(source), start = first.lastIndexOf("[[relations]]");
    return `${first}\n${first.slice(start).replaceAll("region-gate", "outside-gate")}`;
  };
  const before = setup(t, fixtureWithTwoGates), after = setup(t, source => fixtureWithTwoGates(source)
    .replace('resource = "external.region-gate"', 'resource = "external.changed-reference"'));
  const scene = data => {
    const topology = new OrganizationTopology(data.intake, data.occurrences, limits);
    const inner = selectedRoute(topology, findRelation(topology, "relation.region-gate"));
    const outer = selectedRoute(topology, findRelation(topology, "relation.outside-gate")), root = outer.source;
    const geometry = topology.geometry([{ appearance: root, width: 80, height: 200 },
      { appearance: inner.target, parent: root, width: 80, height: 40 },
      { appearance: outer.target, width: 80, height: 40 }], [inner, outer], geometryLimits);
    return { topology, geometry, root, inner, outer };
  };
  const prior = scene(before), current = scene(after), accepted = await acceptedGeometry(t, prior);
  const comparison = retainedComparison(new Pale().comparePortableWorkspaces(before.owner, after.owner)), choices = regionPairs(prior, current);
  const missing = current.topology.prepareRegion(prior.topology, prior.geometry, current.geometry, accepted,
    comparison, { prior: prior.root, current: current.root }, choices);
  assert.equal(missing.step().reason, "incomplete-pairing");
  choices.records.push({ prior: prior.inner.relation, current: current.inner.relation });
  choices.routes.push({ prior: prior.geometry.routes[0], current: current.geometry.routes[0] });
  const ready = finishReuse(current.topology.prepareRegion(prior.topology, prior.geometry, current.geometry, accepted,
    comparison, { prior: prior.root, current: current.root }, choices), before.intake);
  assert.equal(ready.kind, "region-ready");
  assert.equal(ready.evidence.find(item => item.prior === prior.inner.relation).correspondence.state.values, WorkspaceValueComparison.Changed);
  assert.equal(current.inner.target.witness, "external-reference");
  assert.equal(current.inner.target.endpoint.nativeResource, "external.changed-reference");
  const rootInput = ready.plan.elkInput().children[0];
  assert.equal(rootInput.layoutOptions["elk.portConstraints"], "FIXED_POS");
  const port = rootInput.ports.find(port => port.id === "p0s"), previous = accepted.ports.find(port => port.id === "p0s");
  assert.equal(port.x, previous.bounds.x); assert.equal(port.y, previous.bounds.y);
  assert(current.topology.entry(current.inner.relation).handle.belongsTo(after.owner));
});

test("real SDK correspondence reuses real ELK geometry with different local IDs and fresh source custody", async t => {
  const before = setup(t), after = setup(t, source => `\n# shifted source occurrence\n${source}`);
  const prior = reuseScene(before), current = reuseScene(after, { reverse: true });
  assert.notEqual(prior.appearances[0].id, current.appearances[0].id);
  const accepted = await acceptedGeometry(t, prior), comparison = retainedComparison(new Pale().comparePortableWorkspaces(before.owner, after.owner));
  const preparation = current.topology.prepareReuse(prior.topology, prior.geometry, current.geometry,
    accepted, comparison, visualPairs(prior, current));
  const result = finishReuse(preparation, before.intake); assert.equal(result.kind, "reused");
  assert.equal(result.evidence.length, prior.records.length);
  for (const item of result.evidence) {
    assert(item.correspondence.belongsTo(comparison));
    assert(item.correspondence.selected.belongsTo(before.owner));
    assert(item.correspondence.state.other.handle.belongsTo(after.owner));
    assert(!item.correspondence.selected.sameRecord(item.correspondence.state.other.handle));
  }
  assert.equal(preparation.step(), result); assert(Object.isFrozen(result) && Object.isFrozen(result.evidence));
  assert(Object.isFrozen(current.geometry) && Object.isFrozen(current.geometry.routes[0]));
  assert.equal(current.geometry.routes[0].witness.locality, OrganizationRelationLocality.Internal);
  assert.equal(current.geometry.routes[0].witness.locality,
    current.topology.entry(current.geometry.routes[0].witness.occurrence).row.locality);
  assert.equal(new Set(current.topology.records.map(item => current.topology.entry(item).row.kind)).size, 18);
  preparation.close(); assert.equal(preparation.step().kind, "closed");
  assert(current.topology.entry(current.records[0]).handle.belongsTo(after.owner));
});

test("topology and reuse keep SDK meaning when retained instances are shadowed", async t => {
  const before = setup(t), after = setup(t, source => `\n# shifted source occurrence\n${source}`);
  const prior = reuseScene(before), accepted = await acceptedGeometry(t, prior);
  const entryReplacement = t.mock.method(after.intake, "entry",
    () => assert.fail("Topology must read entries through its retained intake owner"));
  const current = reuseScene(after, { reverse: true });
  assert.equal(entryReplacement.mock.callCount(), 0);

  const comparison = retainedComparison(new Pale().comparePortableWorkspaces(before.owner, after.owner));
  const intakeReplacement = t.mock.method(before.intake, "correspond",
    () => assert.fail("Reuse must use the retained intake implementation"));
  const comparisonReplacement = t.mock.method(comparison, "correspondOrganizationRecord",
    () => assert.fail("Intake must use the SDK correspondence implementation"));
  const result = finishReuse(current.topology.prepareReuse(prior.topology, prior.geometry, current.geometry,
    accepted, comparison, visualPairs(prior, current)), before.intake);
  assert.equal(result.kind, "reused");
  assert.equal(intakeReplacement.mock.callCount(), 0);
  assert.equal(comparisonReplacement.mock.callCount(), 0);
  assert(result.evidence.every(item => item.correspondence.belongsTo(comparison)));
});

test("changed nested meaning may reuse constraints while equal meaning with different dimensions requires layout", async t => {
  const before = setup(t), after = setup(t, source => source.replace('resources = ["resource.role-a"]', 'resources = ["resource.changed"]'));
  const prior = reuseScene(before), current = reuseScene(after), accepted = await acceptedGeometry(t, prior);
  const comparison = retainedComparison(new Pale().comparePortableWorkspaces(before.owner, after.owner));
  const result = finishReuse(current.topology.prepareReuse(prior.topology, prior.geometry, current.geometry,
    accepted, comparison, visualPairs(prior, current)), before.intake);
  assert.equal(result.kind, "reused");
  assert(result.evidence.some(item => item.correspondence.state.values === WorkspaceValueComparison.Changed));
  const equal = setup(t), wider = reuseScene(equal, { width: 120 });
  const equalPair = retainedComparison(new Pale().comparePortableWorkspaces(before.owner, equal.owner));
  const changed = finishReuse(wider.topology.prepareReuse(prior.topology, prior.geometry, wider.geometry,
    accepted, equalPair, visualPairs(prior, wider)), before.intake);
  assert.deepEqual(changed, { kind: "layout-required", reason: "constraints" });
});

test("repeated semantic appearances and parallel route depictions keep explicit independent reuse choices", async t => {
  for (const options of [{ repeated: true }, { bundle: true }]) {
    const before = setup(t), after = setup(t), prior = reuseScene(before, options), current = reuseScene(after, { ...options, reverse: true });
    const accepted = await acceptedGeometry(t, prior), comparison = retainedComparison(new Pale().comparePortableWorkspaces(before.owner, after.owner));
    const result = finishReuse(current.topology.prepareReuse(prior.topology, prior.geometry, current.geometry,
      accepted, comparison, visualPairs(prior, current)), before.intake);
    assert.equal(result.kind, "reused"); assert.equal(current.geometry.routes.length, 2);
    assert.notEqual(current.geometry.routes[0], current.geometry.routes[1]);
    assert.equal(current.geometry.routes[0].selection.relation, current.geometry.routes[1].selection.relation);
    if (options.bundle) {
      assert(Object.isFrozen(current.geometry.bundles[0]) && Object.isFrozen(current.geometry.bundles[0].routes));
      assert.deepEqual(current.geometry.bundles[0].routes, current.geometry.routes);
    }
  }
});

test("forged assemblies, copied route tokens, duplicate and incomplete choices never grant geometry reuse", t => {
  const before = setup(t), after = setup(t), prior = reuseScene(before), current = reuseScene(after);
  const comparison = retainedComparison(new Pale().comparePortableWorkspaces(before.owner, after.owner));
  const invoke = (choices, first = prior.geometry) => current.topology.prepareReuse(prior.topology, first,
    current.geometry, {}, comparison, choices);
  assert.throws(() => invoke(visualPairs(prior, current), { ...prior.geometry }), error => error.kind === "foreign-assembly");
  const copied = visualPairs(prior, current); copied.routes[0] = { ...copied.routes[0], prior: { ...prior.geometry.routes[0] } };
  assert.throws(() => invoke(copied), error => error.kind === "invalid-pairing");
  const duplicate = visualPairs(prior, current); duplicate.records[1] = duplicate.records[0];
  assert.throws(() => invoke(duplicate), error => error.kind === "invalid-pairing");
  const incomplete = visualPairs(prior, current); incomplete.appearances.pop();
  const calls = before.intake.charges.queries;
  const result = invoke(incomplete).step(); assert.deepEqual(result, { kind: "layout-required", reason: "incomplete-pairing" });
  assert.equal(before.intake.charges.queries, calls); assert(Object.isFrozen(result));
  const missingRecord = visualPairs(prior, current); missingRecord.records.pop();
  assert.equal(invoke(missingRecord).step().reason, "incomplete-pairing");
});

test("same bytes from a third compilation are not the comparison's current selection", async t => {
  const before = setup(t), after = setup(t), unrelated = setup(t), prior = reuseScene(before), current = reuseScene(unrelated);
  const comparison = retainedComparison(new Pale().comparePortableWorkspaces(before.owner, after.owner)), accepted = await acceptedGeometry(t, prior);
  const preparation = current.topology.prepareReuse(prior.topology, prior.geometry, current.geometry,
    accepted, comparison, visualPairs(prior, current));
  const result = finishReuse(preparation, before.intake); assert.equal(result.kind, "not-included");
  assert(result.evidence.correspondence.state.other.handle.belongsTo(after.owner));
  assert(Object.isFrozen(result)); assert.equal(preparation.step(), result);
});

test("bounded correspondence keeps searching separate from final reuse and supports close without consuming more pages", t => {
  const before = setup(t), after = setup(t);
  const left = new OrganizationTopology(before.intake, before.occurrences, limits), right = new OrganizationTopology(after.intake, after.occurrences, limits);
  const choose = topology => topology.records.filter(item => topology.entry(item).row.kind === "derived-relation").at(-1);
  const prior = choose(left), current = choose(right); assert(prior && current);
  const first = left.geometry([], [], geometryLimits), second = right.geometry([], [], geometryLimits);
  const preparation = right.prepareReuse(left, first, second, {}, retainedComparison(new Pale().comparePortableWorkspaces(before.owner, after.owner)),
    { records: [{ prior, current }], appearances: [], routes: [], bundles: [] });
  const calls = before.intake.charges.queries;
  assert.equal(preparation.step().kind, "searching"); assert.equal(before.intake.charges.queries, calls + 1);
  preparation.close(); assert.equal(preparation.step().kind, "closed"); assert.equal(before.intake.charges.queries, calls + 1);
  assert(left.entry(prior).handle.belongsTo(before.owner));
});

test("exact binding interval removal returns unmatched instead of substituting the same occupant", t => {
  const before = setup(t), after = setup(t, source => source.replace('[occupant_bindings.interval]\nstart = 1', '[occupant_bindings.interval]\nstart = 2'));
  const left = new OrganizationTopology(before.intake, before.occurrences, limits), right = new OrganizationTopology(after.intake, after.occurrences, limits);
  const select = topology => topology.appearances.find(item => item.endpoint.kind === "occupant");
  const prior = select(left), current = select(right); assert(prior && current);
  const first = left.geometry(placements([prior]), [], geometryLimits), second = right.geometry(placements([current]), [], geometryLimits);
  const preparation = right.prepareReuse(left, first, second, {}, retainedComparison(new Pale().comparePortableWorkspaces(before.owner, after.owner)),
    { records: [{ prior: prior.occurrence, current: current.occurrence }], appearances: [{ prior, current }], routes: [], bundles: [] });
  const result = finishReuse(preparation, before.intake); assert.equal(result.kind, "unmatched");
  assert.equal(result.evidence.correspondence.state.kind, "unmatched");
});

test("pair bounds and snapshots are enforced before traversing extra input or overridable helpers", t => {
  const before = setup(t), after = setup(t), prior = reuseScene(before), current = reuseScene(after);
  const comparison = retainedComparison(new Pale().comparePortableWorkspaces(before.owner, after.owner));
  const choices = visualPairs(prior, current), excessive = new Array(prior.topology.records.length + 1);
  Object.defineProperty(excessive, 0, { get() { assert.fail("over-budget records must not be read"); } });
  assert.throws(() => current.topology.prepareReuse(prior.topology, prior.geometry, current.geometry, {}, comparison,
    { ...choices, records: excessive }), error => error.kind === "bounds");
  const invalidLength = new Proxy([], { get(target, key) { return key === "length" ? NaN : Reflect.get(target, key); } });
  assert.throws(() => current.topology.prepareReuse(prior.topology, prior.geometry, current.geometry, {}, comparison,
    { ...choices, records: invalidLength }), error => error.kind === "bounds");
  assert.throws(() => current.topology.geometry(invalidLength, [], geometryLimits), error => error.kind === "bounds");
  assert.throws(() => new OrganizationTopology(before.intake, invalidLength, limits), error => error.kind === "bounds");
  prior.topology.entry = current.topology.entry = () => assert.fail("private custody must not dispatch public helpers");
  const counts = {}; const snapshots = {};
  for (const key of ["records", "appearances", "routes", "bundles"]) Object.defineProperty(snapshots, key, {
    get() { counts[key] = (counts[key] ?? 0) + 1; assert.equal(counts[key], 1); return choices[key]; }
  });
  choices.appearances.pop();
  assert.equal(current.topology.prepareReuse(prior.topology, prior.geometry, current.geometry, {}, comparison, snapshots).step().reason, "incomplete-pairing");
  assert.deepEqual(counts, { records: 1, appearances: 1, routes: 1, bundles: 1 });
});

test("exhausted intake stops before dispatch and close does not revoke retained topology records", t => {
  const before = setup(t), after = setup(t);
  const budget = { queries: 1, recordVisits: 100, sourceVisits: 100, records: 100, occurrences: 100,
    sourceOccurrences: 100, copiedStringBytes: 1e6 };
  const limited = organizationIntake(before.owner, { inspected: 1, returned: 1, copiedStringBytes: 65536 }, budget);
  t.after(() => limited.close());
  const occurrence = limited.next().occurrences[0], left = new OrganizationTopology(limited, [occurrence], limits);
  const right = new OrganizationTopology(after.intake, after.occurrences, limits);
  const current = right.records.find(item => right.entry(item).row.kind === left.entry(occurrence).row.kind);
  const first = left.geometry([], [], geometryLimits), second = right.geometry([], [], geometryLimits);
  const preparation = right.prepareReuse(left, first, second, {}, retainedComparison(new Pale().comparePortableWorkspaces(before.owner, after.owner)),
    { records: [{ prior: occurrence, current }], appearances: [], routes: [], bundles: [] });
  const charged = limited.charges;
  for (let count = 0; count < 2; count++) assert.deepEqual(preparation.step(), { kind: "resource-stop", resource: "queries" });
  assert.deepEqual(limited.charges, charged);
  limited.close(); assert.equal(preparation.step().kind, "closed");
  assert(left.entry(occurrence).handle.belongsTo(before.owner));
});

test("reversed comparison custody fails without advancing preparation; accepted geometry cannot be copied", async t => {
  const before = setup(t), after = setup(t), prior = reuseScene(before), current = reuseScene(after);
  const accepted = await acceptedGeometry(t, prior), choices = visualPairs(prior, current);
  const reversed = retainedComparison(new Pale().comparePortableWorkspaces(after.owner, before.owner));
  const preparation = current.topology.prepareReuse(prior.topology, prior.geometry, current.geometry, accepted, reversed, choices);
  const calls = before.intake.charges.queries;
  for (let attempt = 1; attempt <= 2; attempt++) {
    assert.throws(() => preparation.step(), error => error.diagnostic.code === "workspace_organization_custody_mismatch");
    assert.equal(before.intake.charges.queries, calls + attempt);
  }
  preparation.close();
  const comparison = retainedComparison(new Pale().comparePortableWorkspaces(before.owner, after.owner));
  current.geometry.plan.reuseFrom = () => assert.fail("reuse must invoke the method captured before plan exposure");
  const forged = current.topology.prepareReuse(prior.topology, prior.geometry, current.geometry, { ...accepted }, comparison, choices);
  assert.throws(() => finishReuse(forged, before.intake), error => error.kind === "input");
  const genuine = current.topology.prepareReuse(prior.topology, prior.geometry, current.geometry, accepted, comparison, choices);
  assert.equal(finishReuse(genuine, before.intake).kind, "reused");
  assert(prior.topology.entry(prior.records[0]).handle.belongsTo(before.owner));
});

test("stable relation identity with a changed external reference cannot inherit endpoint placement", t => {
  const append = resource => source => `${source}
[[relations]]
relation_id = "relation.external-reuse"
class = "native_presence"
issuer = "issuer.external"
state = "unknown"
[relations.from]
kind = "role"
[relations.from.role]
namespace = "alpha"
role_id = "accountable-a"
[relations.to]
kind = "native_resource"
resource = "${resource}"
[relations.interval]
start = 1
end_exclusive = 10
`;
  const before = setup(t, append("native.first")), after = setup(t, append("native.second"));
  const prior = reuseScene(before, { relationId: "relation.external-reuse" });
  const current = reuseScene(after, { relationId: "relation.external-reuse" });
  const result = finishReuse(current.topology.prepareReuse(prior.topology, prior.geometry, current.geometry, {},
    retainedComparison(new Pale().comparePortableWorkspaces(before.owner, after.owner)), visualPairs(prior, current)), before.intake);
  assert.deepEqual(result, { kind: "layout-required", reason: "endpoints" });
  assert.equal(prior.appearances[1].witness, "external-reference");
  assert.equal(current.topology.entry(current.appearances[1].occurrence).row.kind, "relation");
});

test("an external self-loop keeps source and target witness roles separate even for equal endpoint fields", t => {
  const append = source => `${source}
[[relations]]
relation_id = "relation.external-loop"
class = "native_presence"
issuer = "issuer.external"
state = "unknown"
[relations.from]
kind = "native_resource"
resource = "native.same"
[relations.to]
kind = "native_resource"
resource = "native.same"
[relations.interval]
start = 1
end_exclusive = 10
`;
  const before = setup(t, append), after = setup(t, append);
  const left = new OrganizationTopology(before.intake, before.occurrences, limits), right = new OrganizationTopology(after.intake, after.occurrences, limits);
  const build = (topology, swap) => {
    const relation = findRelation(topology, "relation.external-loop");
    const appearances = topology.appearances.filter(item => item.occurrence === relation);
    assert.equal(appearances.length, 2); assert.deepEqual(appearances[0].endpoint, appearances[1].endpoint);
    if (swap) appearances.reverse();
    const geometry = topology.geometry(placements(appearances), [{ relation, source: appearances[0], target: appearances[1] }], geometryLimits);
    return { topology, geometry, appearances, records: [relation] };
  };
  const prior = build(left, false), current = build(right, true);
  const result = finishReuse(right.prepareReuse(left, prior.geometry, current.geometry, {},
    retainedComparison(new Pale().comparePortableWorkspaces(before.owner, after.owner)), visualPairs(prior, current)), before.intake);
  assert.deepEqual(result, { kind: "layout-required", reason: "endpoints" });
});

test("all eighteen families retain exact SDK entries without inventing geometry for non-node records", t => {
  const { owner, intake, occurrences } = setup(t);
  const topology = new OrganizationTopology(intake, occurrences, limits);
  assert.deepEqual(topology.records, occurrences);
  assert.equal(new Set(topology.records.map(occurrence => topology.entry(occurrence).row.kind)).size, 18);
  for (const occurrence of occurrences) {
    assert.equal(topology.entry(occurrence), intake.entry(occurrence));
    assert(topology.entry(occurrence).handle.belongsTo(owner));
  }
  for (const kind of ["service", "population-rule", "responsibility", "inspection", "native-projection"]) {
    const occurrence = occurrences.find(item => intake.entry(item).row.kind === kind); assert(occurrence);
    assert(!topology.appearances.some(item => item.occurrence === occurrence));
  }
  const geometry = topology.geometry([], [], geometryLimits);
  assert.equal(geometry.unplaced.length, topology.appearances.length);
  assert.deepEqual(geometry.unrouted, topology.relations);
  intake.close(); assert(topology.entry(occurrences[0]).handle.belongsTo(owner));
});

test("directional service routes retain exact contracts and five independent requirements", t => {
  const { owner, intake, occurrences } = setup(t), topology = new OrganizationTopology(intake, occurrences, limits);
  const services = ["alpha", "beta"].map(namespace => findService(topology, namespace));
  const routes = services.map(service => selectedRoute(topology, service));
  assert.equal(routes[0].source, routes[1].target); assert.equal(routes[0].target, routes[1].source);
  const geometry = topology.geometry(placements(topology.appearances), routes, geometryLimits);
  for (const [index, namespace] of ["alpha", "beta"].entries()) {
    const binding = geometry.routes[index], entry = topology.entry(binding.witness.occurrence), row = entry.row;
    assert.deepEqual(binding.witness, { kind: "service", occurrence: services[index] });
    assert(Object.isFrozen(binding.witness)); assert.equal(entry, intake.entry(services[index]));
    assert(entry.handle.belongsTo(owner)); assert.equal(row.contract.namespace, namespace);
    assert.equal(row.contract.contractId, "service.shared"); assert.equal(row.contractOwner, `owner.${namespace}`);
    assert.deepEqual(binding.selection.source.endpoint, { kind: "scope", scope: row.contract.providerScope });
    assert.deepEqual(binding.selection.target.endpoint, { kind: "scope", scope: row.contract.consumerScope });
    const requirements = [row.providerCompletion, row.providerAcceptance, row.consumerAcceptance, row.nativeReadBack, row.paleSettlement];
    assert.equal(new Set(requirements.map(value => value.evidenceRef)).size, 5);
    assert(requirements.every(value => value.owner === namespace));
    assert.deepEqual(row.interval, { start: 1n, endExclusive: 10n });
  }
  assert.throws(() => topology.geometry(placements(topology.appearances),
    [{ ...routes[0], source: routes[0].target, target: routes[0].source }], geometryLimits), error => error.kind === "endpoint-mismatch");
  const wrongNamespace = topology.appearances.find(item => item.endpoint.kind === "scope" && item.endpoint.scope.namespace === "gamma");
  assert(wrongNamespace); assert.equal(wrongNamespace.endpoint.scope.scopeId, routes[0].source.endpoint.scope.scopeId);
  assert.throws(() => topology.geometry(placements(topology.appearances), [{ ...routes[0], source: wrongNamespace }], geometryLimits),
    error => error.kind === "endpoint-mismatch");
  assert(!JSON.stringify(geometry.plan.elkInput()).includes("service.shared"));
  intake.close(); assert(topology.entry(geometry.routes[0].witness.occurrence).handle.belongsTo(owner));
});

test("a selected service without selected scope records keeps typed not-included endpoints", t => {
  const { intake, occurrences } = setup(t);
  const service = occurrences.find(item => intake.entry(item).row.kind === "service"); assert(service);
  const row = intake.entry(service).row, topology = new OrganizationTopology(intake, [service], limits);
  assert.deepEqual(topology.endpoints(service), {
    from: { kind: "not-included", endpoint: { kind: "scope", scope: row.contract.providerScope } },
    to: { kind: "not-included", endpoint: { kind: "scope", scope: row.contract.consumerScope } },
  });
  assert.equal(topology.appearances.length, 0);
  assert.deepEqual(topology.geometry([], [], geometryLimits).unrouted, [service]);
  assert.equal(topology.entry(service), intake.entry(service));
});

test("parallel services and repeated appearances keep independent witnesses through shared ports", t => {
  const data = setup(t, source => changeAlphaService(source, service => `${service}\n${service
    .replace('namespace = "alpha"', 'namespace = "gamma"').replace('contract_owner = "owner.alpha"', 'contract_owner = "owner.gamma"')}`));
  const original = new OrganizationTopology(data.intake, data.occurrences, limits);
  const service = findService(original, "alpha"), provider = selectedRoute(original, service).source.occurrence;
  const repeatedProvider = data.intake.repeat(provider), repeatedService = data.intake.repeat(service);
  const topology = new OrganizationTopology(data.intake, [...data.occurrences, repeatedProvider, repeatedService], limits);
  const parallel = findService(topology, "gamma");
  assert.equal(topology.entry(service).row.contract.contractId, topology.entry(parallel).row.contract.contractId);
  assert(!topology.entry(service).handle.sameRecord(topology.entry(parallel).handle));
  assert(topology.entry(service).handle.sameRecord(topology.entry(repeatedService).handle));
  const routes = [selectedRoute(topology, service), selectedRoute(topology, service, 1),
    selectedRoute(topology, repeatedService), selectedRoute(topology, parallel)];
  assert.notEqual(routes[0].source, routes[1].source);
  assert.equal(topology.endpoints(service).from.appearances.length, 2);
  const geometry = topology.geometry(placements(topology.appearances), routes, geometryLimits,
    [{ end: "target", routeIndices: [0, 1, 2, 3] }]);
  assert.equal(geometry.routes.length, 4); assert.equal(new Set(geometry.routes.map(route => route.id)).size, 4);
  assert.deepEqual(geometry.routes.map(route => route.witness.occurrence), [service, service, repeatedService, parallel]);
  assert(geometry.routes.every(route => route.witness.kind === "service"));
  assert.deepEqual(geometry.bundles[0].routes, geometry.routes);
  assert.deepEqual(geometry.plan.elkInput().edges.map(edge => edge.targets), [["b0p"], ["b0p"], ["b0p"], ["b0p"]]);
  assert.throws(() => topology.geometry(placements(topology.appearances), routes, geometryLimits,
    [{ end: "source", routeIndices: [0, 1] }]), error => error.kind === "invalid-bundle");
});

test("service endpoint snapshots cannot be retargeted by changing a copied SDK row", t => {
  const data = setup(t), topology = new OrganizationTopology(data.intake, data.occurrences, limits);
  const service = findService(topology, "alpha"), row = topology.entry(service).row;
  const before = topology.endpoints(service), route = selectedRoute(topology, service);
  for (const endpoint of [before.from.endpoint, before.to.endpoint, route.source.endpoint, route.target.endpoint]) {
    assert(Object.isFrozen(endpoint)); assert(Object.isFrozen(endpoint.scope));
    assert.equal(Reflect.set(endpoint, "kind", "role"), false);
    assert.equal(Reflect.set(endpoint.scope, "namespace", "retargeted"), false);
  }
  row.contract.providerScope.namespace = "gamma"; row.contract.consumerScope.namespace = "alpha";
  assert.deepEqual(topology.endpoints(service), before);
  const geometry = topology.geometry(placements(topology.appearances), [route], geometryLimits);
  assert.equal(geometry.routes[0].selection.source.endpoint.scope.namespace, "alpha");
  assert.equal(geometry.routes[0].selection.target.endpoint.scope.namespace, "beta");
  assert.equal(geometry.routes[0].witness.occurrence, service);
});

test("service correspondence reuses real geometry while retaining changed requirements and fresh source custody", async t => {
  const before = setup(t), after = setup(t, source => `\n# moved service source\n${source
    .replace('evidence_ref = "evidence.alpha.completion"', 'evidence_ref = "evidence.alpha.changed-completion"')}`);
  const prior = serviceScene(before), current = serviceScene(after, true), accepted = await acceptedGeometry(t, prior);
  assert.notEqual(prior.appearances[0].id, current.appearances[0].id);
  const comparison = retainedComparison(new Pale().comparePortableWorkspaces(before.owner, after.owner));
  const result = finishReuse(current.topology.prepareReuse(prior.topology, prior.geometry, current.geometry,
    accepted, comparison, visualPairs(prior, current)), before.intake);
  assert.equal(result.kind, "reused");
  const serviceProof = result.evidence.find(item => item.prior === prior.records[0]); assert(serviceProof);
  assert.equal(serviceProof.correspondence.state.values, WorkspaceValueComparison.Changed);
  assert(serviceProof.correspondence.selected.belongsTo(before.owner));
  assert(serviceProof.correspondence.state.other.handle.sameRecord(current.topology.entry(current.records[0]).handle));
  const currentEntry = current.topology.entry(current.geometry.routes[0].witness.occurrence);
  assert(currentEntry.handle.belongsTo(after.owner)); assert.equal(currentEntry.row.providerCompletion.evidenceRef, "evidence.alpha.changed-completion");
  assert.equal(prior.topology.entry(prior.records[0]).row.providerCompletion.evidenceRef, "evidence.alpha.completion");
  assert.equal(current.geometry.routes[0].witness.kind, "service");
});

test("changing either directional scope leaves the former full service key unmatched", async t => {
  // The responsibility fixture selects alpha's full contract key. Change beta's
  // independent service so the test does not leave that authored join dangling.
  const before = setup(t), prior = serviceScene(before, false, "beta"), accepted = await acceptedGeometry(t, prior);
  for (const [field, namespace] of [["provider_scope", "beta"], ["consumer_scope", "alpha"]]) {
    const after = setup(t, source => source.replace(
      `[services.${field}]\nnamespace = "${namespace}"`, `[services.${field}]\nnamespace = "gamma"`));
    const current = serviceScene(after, false, "beta"), comparison = retainedComparison(new Pale().comparePortableWorkspaces(before.owner, after.owner));
    assert.equal(prior.topology.entry(prior.records[0]).row.contract.contractId, current.topology.entry(current.records[0]).row.contract.contractId);
    const result = finishReuse(current.topology.prepareReuse(prior.topology, prior.geometry, current.geometry,
      accepted, comparison, visualPairs(prior, current)), before.intake);
    assert.equal(result.kind, "unmatched"); assert.equal(result.evidence.prior, prior.records[0]);
    assert.equal(result.evidence.correspondence.state.kind, "unmatched");
    assert(current.topology.entry(current.records[0]).handle.belongsTo(after.owner));
  }
});

test("an equal local service name cannot substitute another retained service during reuse", t => {
  const before = setup(t), after = setup(t), prior = serviceScene(before), current = serviceScene(after, false, "beta");
  const comparison = retainedComparison(new Pale().comparePortableWorkspaces(before.owner, after.owner));
  const result = finishReuse(current.topology.prepareReuse(prior.topology, prior.geometry, current.geometry,
    {}, comparison, visualPairs(prior, current)), before.intake);
  assert.equal(result.kind, "not-included"); assert.equal(result.evidence.prior, prior.records[0]);
  assert.equal(result.evidence.correspondence.state.other.row.contract.namespace, "alpha");
  assert.equal(current.topology.entry(current.records[0]).row.contract.namespace, "beta");
});

test("namespace-qualified endpoints reject a same-local-ID node in another namespace", t => {
  const { intake, occurrences } = setup(t), topology = new OrganizationTopology(intake, occurrences, limits);
  const relation = findRelation(topology, "relation.coordinates"), route = selectedRoute(topology, relation);
  const wrong = topology.appearances.find(item => item.endpoint.kind === "discipline-tree"
    && item.endpoint.tree.treeId === route.target.endpoint.tree.treeId
    && item.endpoint.tree.namespace !== route.target.endpoint.tree.namespace);
  assert(wrong);
  assert.throws(() => topology.geometry(placements(topology.appearances), [{ ...route, target: wrong }], geometryLimits),
    error => error.kind === "endpoint-mismatch");
  const geometry = topology.geometry(placements(topology.appearances), [route], geometryLimits);
  assert.equal(geometry.routes[0].selection.target, route.target);
  const input = geometry.plan.elkInput(), encoded = JSON.stringify(input);
  assert(!encoded.includes("alpha") && !encoded.includes("primary-tree") && !encoded.includes("relation.coordinates"));
  assert.equal(input.edges.length, 1);
});

test("repeated appearances require explicit routing and never cause a cross product", t => {
  const { intake, occurrences } = setup(t);
  const role = occurrences.find(item => intake.entry(item).row.kind === "role"
    && intake.entry(item).row.role.roleId === "accountable-a"); assert(role);
  const repeated = intake.repeat(role); assert.equal(repeated.inclusion, "additional-occurrence");
  const topology = new OrganizationTopology(intake, [...occurrences, repeated], limits);
  const relation = findRelation(topology, "relation.collaboration");
  assert.equal(topology.endpoints(relation).from.appearances.length, 2);
  const first = selectedRoute(topology, relation), second = selectedRoute(topology, relation, 1);
  assert.notEqual(first.source, second.source);
  assert.equal(topology.entry(first.source.occurrence), topology.entry(second.source.occurrence));
  const geometry = topology.geometry(placements(topology.appearances), [second], geometryLimits);
  assert.equal(geometry.routes.length, 1); assert.equal(geometry.routes[0].selection.source, second.source);
  assert.equal(geometry.plan.elkInput().edges.length, 1);
  const twice = topology.geometry(placements(topology.appearances), [first, second], geometryLimits);
  assert.equal(new Set(twice.routes.map(item => item.id)).size, 2);
  assert(twice.routes.every(item => item.selection.relation === relation));
});

test("parallel explicit relations and distinct derived intervals keep independent route custody", t => {
  const { intake, occurrences } = setup(t, source => {
    const begin = source.indexOf('[[relations]]\nrelation_id = "relation.collaboration"');
    const end = source.indexOf("[[relations]]", begin + 1); assert(begin >= 0 && end > begin);
    const parallel = source.slice(begin, end).replace("relation.collaboration", "relation.parallel");
    const bindingStart = source.indexOf("[[occupant_bindings]]"), bindingEnd = source.indexOf("[[provider_bindings]]");
    assert(bindingStart >= 0 && bindingEnd > bindingStart);
    const later = source.slice(bindingStart, bindingEnd).replace("start = 1", "start = 11").replace("end_exclusive = 10", "end_exclusive = 20");
    return `${source}\n${parallel}\n${later}`;
  });
  const topology = new OrganizationTopology(intake, occurrences, limits);
  const parallel = ["relation.collaboration", "relation.parallel"].map(id => selectedRoute(topology, findRelation(topology, id)));
  assert.equal(parallel[0].source, parallel[1].source); assert.equal(parallel[0].target, parallel[1].target);
  const derived = topology.relations.filter(item => topology.entry(item).row.kind === "derived-relation"
    && topology.entry(item).row.source.kind === "occupant-binding");
  assert.equal(derived.length, 2);
  assert.deepEqual(derived.map(item => topology.entry(item).row.interval.start).sort((a, b) => a < b ? -1 : 1), [1n, 11n]);
  const bindings = topology.appearances.filter(item => item.witness === "binding" && item.endpoint.kind === "occupant");
  assert.equal(bindings.length, 2);
  const routes = [...parallel, ...derived.map(item => selectedRoute(topology, item))];
  const geometry = topology.geometry(placements(topology.appearances), routes, geometryLimits);
  assert.equal(geometry.routes.length, 4); assert.equal(new Set(geometry.routes.map(item => item.id)).size, 4);
  assert.deepEqual(geometry.routes.map(item => item.selection.relation), routes.map(item => item.relation));
  assert.deepEqual(geometry.routes.map(item => item.witness.kind), ["relation", "relation", "derived-relation", "derived-relation"]);
  assert(geometry.routes.slice(0, 2).every(item => item.witness.locality === OrganizationRelationLocality.Internal));
  assert(geometry.routes.slice(2).every(item => !Object.hasOwn(item.witness, "locality")));
  assert.deepEqual(geometry.routes.slice(2).map(item => topology.entry(item.selection.relation).row.interval.start)
    .sort((a, b) => a < b ? -1 : 1), [1n, 11n]);
});

test("one leadership topic does not collapse four compiler-derived endpoint pairs", t => {
  const { intake, occurrences } = setup(t), topology = new OrganizationTopology(intake, occurrences, limits);
  const relations = topology.relations.filter(item => topology.entry(item).row.kind === "derived-relation"
    && topology.entry(item).row.source.kind === "leadership-participation");
  assert.equal(relations.length, 4);
  const geometry = topology.geometry(placements(topology.appearances), relations.map(item => selectedRoute(topology, item)), geometryLimits);
  assert.equal(geometry.routes.length, 4);
  assert.equal(new Set(geometry.routes.map(item => `${item.selection.source.id}/${item.selection.target.id}`)).size, 4);
});

test("partial selection leaves endpoints not included and preserves the unrouted relation", t => {
  const { intake, occurrences } = setup(t);
  const relation = occurrences.find(item => intake.entry(item).row.relationId === "relation.collaboration"); assert(relation);
  const topology = new OrganizationTopology(intake, [relation], limits), endpoints = topology.endpoints(relation);
  assert.equal(endpoints.from.kind, "not-included"); assert.equal(endpoints.to.kind, "not-included");
  assert.equal(topology.appearances.length, 0);
  assert.deepEqual(topology.geometry([], [], geometryLimits).unrouted, [relation]);
  assert.equal(topology.entry(relation), intake.entry(relation));
});

test("external endpoint appearance is only a relation reference, not a native record", t => {
  const { intake, occurrences } = setup(t, source => `${source}
[[relations]]
relation_id = "relation.external"
class = "native_presence"
issuer = "issuer.external"
state = "unknown"
[relations.from]
kind = "role"
[relations.from.role]
namespace = "alpha"
role_id = "accountable-a"
[relations.to]
kind = "native_resource"
resource = "external.unobserved"
[relations.interval]
start = 1
end_exclusive = 10
`);
  const topology = new OrganizationTopology(intake, occurrences, limits), relation = findRelation(topology, "relation.external");
  const route = selectedRoute(topology, relation);
  assert.equal(route.target.witness, "external-reference"); assert.equal(route.target.occurrence, relation);
  assert.deepEqual(route.target.endpoint, { kind: "native-resource", nativeResource: "external.unobserved" });
  assert.equal(topology.entry(route.target.occurrence).row.kind, "relation");
  const row = topology.entry(relation).row;
  assert.equal(row.locality, OrganizationRelationLocality.NativeBoundary);
  // The issued witness snapshots SDK locality. A subsequently changed copied DTO
  // cannot reinterpret that route, and repeated depictions retain their own IDs.
  row.locality = OrganizationRelationLocality.Internal;
  const geometry = topology.geometry(placements(topology.appearances), [route, route], geometryLimits);
  assert.equal(geometry.routes[0].selection.target, route.target);
  assert.notEqual(geometry.routes[0].id, geometry.routes[1].id);
  for (const binding of geometry.routes) {
    assert.deepEqual(binding.witness, { kind: "relation", occurrence: relation,
      locality: OrganizationRelationLocality.NativeBoundary });
    assert(Object.isFrozen(binding.witness));
    assert.equal(binding.selection.target.witness, "external-reference");
  }
  assert(!JSON.stringify(geometry.plan.elkInput()).includes("external.unobserved"));
});

test("limits, forged appearances and geometry failures leave SDK evidence intact", t => {
  const { owner, intake, occurrences } = setup(t);
  assert.throws(() => new OrganizationTopology(intake, occurrences, { ...limits, records: 1 }), error => error.kind === "bounds");
  assert.throws(() => new OrganizationTopology(intake, occurrences, { ...limits, identityCodeUnits: 1 }), error => error.kind === "bounds");
  assert.throws(() => new OrganizationTopology(intake, [occurrences[0], occurrences[0]], limits), error => error.kind === "duplicate-occurrence");
  const topology = new OrganizationTopology(intake, occurrences, limits), other = new OrganizationTopology(intake, occurrences, limits);
  assert.throws(() => topology.geometry(placements([other.appearances[0]]), [], geometryLimits), error => error.kind === "foreign-appearance");
  assert.throws(() => topology.geometry(placements([{ ...topology.appearances[0] }]), [], geometryLimits), error => error.kind === "foreign-appearance");
  assert.throws(() => topology.geometry(placements([topology.appearances[0], topology.appearances[0]]), [], geometryLimits), error => error.kind === "duplicate-placement");
  assert.throws(() => topology.geometry([{ appearance: topology.appearances[0], width: NaN, height: 40 }], [], geometryLimits));
  assert(topology.entry(occurrences[0]).handle.belongsTo(owner));
  const relation = findRelation(topology, "relation.collaboration"), route = selectedRoute(topology, relation);
  assert.throws(() => topology.geometry([], [route], geometryLimits), error => error.kind === "unplaced-endpoint");
  assert.equal(Reflect.set(route.source.endpoint.role, "roleId", "wrong"), false);
});

test("presentation parents are explicit while the role retains separate group and tree associations", t => {
  const { intake, occurrences } = setup(t);
  const role = occurrences.find(item => {
    const row = intake.entry(item).row;
    return row.kind === "role" && row.group !== null && row.disciplineTree !== null;
  }); assert(role);
  const again = intake.repeat(role); assert.equal(again.inclusion, "additional-occurrence");
  const topology = new OrganizationTopology(intake, [...occurrences, again], limits), row = topology.entry(role).row;
  const group = topology.appearances.find(item => item.endpoint.kind === "group"
    && item.endpoint.group.namespace === row.group.namespace && item.endpoint.group.groupId === row.group.groupId);
  const tree = topology.appearances.find(item => item.endpoint.kind === "discipline-tree"
    && item.endpoint.tree.namespace === row.disciplineTree.namespace && item.endpoint.tree.treeId === row.disciplineTree.treeId);
  const first = topology.appearances.find(item => item.occurrence === role);
  const second = topology.appearances.find(item => item.occurrence === again);
  assert(group && tree && first && second);
  const layout = topology.geometry([...placements([group, tree]),
    { appearance: first, parent: group, width: 80, height: 40 },
    { appearance: second, parent: tree, width: 80, height: 40 }], [], geometryLimits).plan.elkInput();
  assert.equal(layout.children[0].children[0].id, first.id);
  assert.equal(layout.children[1].children[0].id, second.id);
  assert.notDeepEqual(row.group, row.disciplineTree);
  assert.equal(topology.entry(role), topology.entry(again));
  assert.throws(() => topology.geometry([{ appearance: first, parent: group, width: 80, height: 40 }], [], geometryLimits),
    error => error.kind === "input");
});

test("geometry count budgets reject before accessing selected placements or routes", t => {
  const { owner, intake, occurrences } = setup(t), topology = new OrganizationTopology(intake, occurrences, limits);
  let accesses = 0;
  const placement = { get appearance() { accesses++; throw new Error("placement accessed"); } };
  const route = { get relation() { accesses++; throw new Error("route accessed"); } };
  const reject = (nodes, routes, budget) => assert.throws(() => topology.geometry(nodes, routes, budget),
    error => error.kind === "bounds");
  reject([placement, placement], [], { ...geometryLimits, nodes: 1 });
  reject([], [route, route], { ...geometryLimits, relations: 1 });
  reject([], [route], { ...geometryLimits, ports: 1 });
  for (const field of ["nodes", "relations", "ports"]) {
    const missing = { ...geometryLimits }; delete missing[field];
    reject([placement], [route], missing);
    for (const value of [0, -1, 1.5, NaN, Infinity, Number.MAX_SAFE_INTEGER + 1]) {
      reject([placement], [route], { ...geometryLimits, [field]: value });
    }
  }
  assert.equal(accesses, 0);
  assert(topology.entry(occurrences[0]).handle.belongsTo(owner));
});

test("compiler-backed relationships survive real layout, layout failure and repetition", async t => {
  const { owner, intake, occurrences } = setup(t);
  const topology = new OrganizationTopology(intake, occurrences, limits);
  // This test chooses one depiction per relation; the topology does not make that choice.
  const selections = topology.relations.map(relation => selectedRoute(topology, relation));
  assert(selections.length > 0);
  const assembled = topology.geometry(placements(topology.appearances), selections, geometryLimits);
  assert.equal(assembled.unplaced.length, 0);
  assert.equal(assembled.unrouted.length, 0);
  const budgets = { requestBytes: 1e6, resultBytes: 1e6, deadlineMs: 10000,
    totalRequestBytes: 10e6, totalResultBytes: 10e6, totalActiveMs: 60000,
    oldHeapMb: 128, youngHeapMb: 32 };
  const limited = new ElkExecutor({ ...budgets, resultBytes: 16 });
  try {
    assert.deepEqual(await limited.run(assembled.plan), { kind: "failed", reason: "result-bytes" });
    for (const selection of selections) {
      assert(topology.entry(selection.relation).handle.belongsTo(owner));
      assert.equal(topology.entry(selection.relation), intake.entry(selection.relation));
    }
  } finally { await limited.close(); }
  const executor = new ElkExecutor(budgets);
  try {
    const first = await executor.run(assembled.plan);
    assert.equal(first.kind, "completed");
    assert.deepEqual(first.geometry.nodes.map(node => node.id), topology.appearances.map(node => node.id));
    assert.deepEqual(first.geometry.routes.map(route => route.id), assembled.routes.map(route => route.id));
    for (const [index, route] of first.geometry.routes.entries()) {
      const binding = assembled.routes[index];
      assert.equal(binding.selection.relation, selections[index].relation);
      assert.equal(route.sourcePort, `p${index}s`);
      assert.equal(route.targetPort, `p${index}t`);
      assert(route.points.length >= 2);
    }
    const repeated = await executor.run(assembled.plan);
    assert.equal(repeated.kind, "completed");
    assert.deepEqual(repeated.geometry, first.geometry);
    for (const occurrence of topology.records) assert(topology.entry(occurrence).handle.belongsTo(owner));
  } finally { await executor.close(); }
});

test("explicit route bundles share only the selected port and retain independent semantic relations", t => {
  const { owner, intake, occurrences } = setup(t, source => {
    const begin = source.indexOf('[[relations]]\nrelation_id = "relation.collaboration"');
    const end = source.indexOf("[[relations]]", begin + 1); assert(begin >= 0 && end > begin);
    return `${source}\n${source.slice(begin, end).replace("relation.collaboration", "relation.parallel")}`;
  });
  const topology = new OrganizationTopology(intake, occurrences, limits);
  const routes = ["relation.collaboration", "relation.parallel"].map(id => selectedRoute(topology, findRelation(topology, id)));
  const choice = { end: "target", routeIndices: [0, 1] };
  const assembled = topology.geometry(placements(topology.appearances), routes, { ...geometryLimits, ports: 3 }, [choice]);
  const input = assembled.plan.elkInput();
  assert.equal(input.children.flatMap(node => node.ports).length, 3);
  assert.deepEqual(input.edges.map(edge => edge.targets), [["b0p"], ["b0p"]]);
  assert.deepEqual(input.edges.map(edge => edge.sources), [["p0s"], ["p1s"]]);
  assert.deepEqual(assembled.routes.map(route => route.selection.relation), routes.map(route => route.relation));
  assert.notEqual(assembled.routes[0].selection.relation, assembled.routes[1].selection.relation);
  for (const route of assembled.routes) assert(topology.entry(route.selection.relation).handle.belongsTo(owner));
  choice.end = "source"; choice.routeIndices.reverse(); routes[0].target = routes[0].source;
  assert.deepEqual(assembled.plan.elkInput(), input);
  assert(!JSON.stringify(input).includes("collaboration"));
});

test("bundle ports cannot collapse two visual appearances of the same semantic endpoint", t => {
  const { intake, occurrences } = setup(t);
  const target = occurrences.find(item => intake.entry(item).row.kind === "role"
    && intake.entry(item).row.role.roleId === "accountable-b"); assert(target);
  const repeated = intake.repeat(target); assert.equal(repeated.inclusion, "additional-occurrence");
  const topology = new OrganizationTopology(intake, [...occurrences, repeated], limits);
  const relation = findRelation(topology, "relation.collaboration");
  const routes = [selectedRoute(topology, relation), selectedRoute(topology, relation, 0, 1)];
  assert.deepEqual(routes[0].target.endpoint, routes[1].target.endpoint);
  assert.notEqual(routes[0].target, routes[1].target);
  assert.throws(() => topology.geometry(placements(topology.appearances), routes, geometryLimits,
    [{ end: "target", routeIndices: [0, 1] }]), error => error.kind === "invalid-bundle");
  const sharedSource = topology.geometry(placements(topology.appearances), routes, geometryLimits,
    [{ end: "source", routeIndices: [0, 1] }]);
  assert.deepEqual(sharedSource.plan.elkInput().edges.map(edge => edge.sources), [["b0p"], ["b0p"]]);
  assert.notEqual(sharedSource.routes[0].selection.target, sharedSource.routes[1].selection.target);
});

test("bundle selectors enforce count, index and per-end membership before touching routes", t => {
  const { intake, occurrences } = setup(t), topology = new OrganizationTopology(intake, occurrences, limits);
  let accesses = 0;
  const unread = { get relation() { accesses++; throw new Error("route accessed"); } };
  const reject = (bundles, kind = "invalid-bundle") => assert.throws(
    () => topology.geometry([], [unread, unread], geometryLimits, bundles), error => error.kind === kind);
  for (const routeIndices of [[], [0], [0, 0], [-1, 1], [0, 2], [0, 0.5], [0, NaN]]) {
    reject([{ end: "source", routeIndices }]);
  }
  reject([{ end: "neither", routeIndices: [0, 1] }]);
  reject([{ end: "source", routeIndices: [0, 1] }, { end: "source", routeIndices: [1, 0] }]);
  reject([{ get end() { accesses++; throw new Error("selection accessed"); } }, {}, {}], "bounds");
  reject([{ end: "target", routeIndices: new Proxy([0, 1, 2], {
    get: (target, property) => { if (property !== "length") { accesses++; throw new Error("members accessed"); } return target.length; },
  }) }], "bounds");
  assert.throws(() => topology.geometry([], [unread, unread], { ...geometryLimits, ports: 2 },
    [{ end: "target", routeIndices: [0, 1] }]), error => error.kind === "bounds");
  assert.equal(accesses, 0);
});

test("one route may share both ends without merging its separate depictions", t => {
  const { intake, occurrences } = setup(t), topology = new OrganizationTopology(intake, occurrences, limits);
  const route = selectedRoute(topology, findRelation(topology, "relation.collaboration"));
  const assembled = topology.geometry(placements(topology.appearances), [route, route], { ...geometryLimits, ports: 2 },
    [{ end: "source", routeIndices: [0, 1] }, { end: "target", routeIndices: [1, 0] }]);
  assert.deepEqual(assembled.plan.elkInput().edges, [
    { id: "r0", sources: ["b0p"], targets: ["b1p"] },
    { id: "r1", sources: ["b0p"], targets: ["b1p"] },
  ]);
  assert.equal(assembled.routes.length, 2);
  assert.equal(assembled.routes[0].selection.relation, assembled.routes[1].selection.relation);
});

test("actual SDK appearances retain their evidence through real bundled fan-in layout", async t => {
  const { owner, intake, occurrences } = setup(t);
  const source = occurrences.find(item => intake.entry(item).row.kind === "role"
    && intake.entry(item).row.role.roleId === "accountable-a"); assert(source);
  const repeats = Array.from({ length: 3 }, () => intake.repeat(source));
  assert(repeats.every(item => item.inclusion === "additional-occurrence"));
  const topology = new OrganizationTopology(intake, [...occurrences, ...repeats], limits);
  const relation = findRelation(topology, "relation.collaboration");
  const selections = [0, 1, 2, 3].map(index => selectedRoute(topology, relation, index));
  const selectedNodes = [...selections.map(route => route.source), selections[0].target];
  const assembled = topology.geometry(placements(selectedNodes), selections, { ...geometryLimits, ports: 5 },
    [{ end: "target", routeIndices: [0, 1, 2, 3] }]);
  assert.equal(assembled.plan.elkInput().children.flatMap(node => node.ports).length, 5);
  const executor = new ElkExecutor({ requestBytes: 1e6, resultBytes: 1e6, deadlineMs: 10000,
    totalRequestBytes: 10e6, totalResultBytes: 10e6, totalActiveMs: 60000, oldHeapMb: 128, youngHeapMb: 32 });
  const contacts = new ContactExecutor({ requestBytes: 1e6, resultBytes: 1e6, deadlineMs: 10000,
    totalRequestBytes: 10e6, totalResultBytes: 10e6, totalActiveMs: 60000, oldHeapMb: 128, youngHeapMb: 32 });
  const compiler = new OrganizationSceneCompiler(intake); compiler.select(topology, assembled);
  try {
    const first = await executor.run(assembled.plan); assert.equal(first.kind, "completed");
    const selection = first.geometry.bundles[0]; assert.equal(selection.kind, "partitioned");
    const bundle = selection.corridors.find(corridor => corridor.members.length === 4); assert(bundle);
    assert.equal(bundle.routeKind, "cubic-chain"); assert.equal(selection.portId, "b0p");
    assert.deepEqual(selection.routeIds, ["r0", "r1", "r2", "r3"]);
    for (const corridor of selection.corridors) for (const member of corridor.members) {
      const route = first.geometry.routes.find(item => item.id === member.routeId); assert(route);
      assert(assembled.routes.some(item => item.id === member.routeId));
      assert.deepEqual(route.points.slice(member.firstSegment * 3, member.endSegmentExclusive * 3 + 1), corridor.points);
    }
    for (const member of bundle.members) {
      const route = first.geometry.routes.find(item => item.id === member.routeId); assert(route);
      const binding = assembled.routes.find(item => item.id === member.routeId); assert(binding);
      assert.equal(binding.selection.relation, relation);
      assert.equal(member.endSegmentExclusive, (route.points.length - 1) / 3);
      assert.deepEqual(route.points.slice(member.firstSegment * 3, member.endSegmentExclusive * 3 + 1), bundle.points);
      assert(topology.entry(binding.selection.relation).handle.belongsTo(owner));
      assert.equal(topology.entry(binding.selection.source.occurrence), intake.entry(source));
    }
    assert.equal(new Set(assembled.routes.map(item => item.selection.source)).size, 4);
    assert.equal(first.geometry.routes.length, 4);
    const repeated = await compiler.layout(executor); assert.equal(repeated.kind, "completed");
    const scene = repeated.scene; assert.deepEqual(scene.geometry, first.geometry);
    const order = scene.orderRoutes(["r3", "r1", "r0", "r2"]);
    assert.deepEqual(order.routes.map(item => item.route.id), ["r3", "r1", "r0", "r2"]);
    assert(order.routes.every(item => item.binding === assembled.routes.find(binding => binding.id === item.route.id)));
    const beforeContacts = JSON.stringify(scene.geometry);
    // This actual ELK fan-in can yield a spurious near-endpoint Paper estimate.
    // Refusal retains the accepted scene; it is not repaired into an annotation.
    assert.deepEqual(await compiler.contacts(scene, contacts, order,
      { segments: 100, pairs: 5050, contacts: 500, maximumSeparation: 1e-5, maximumControlDrift: 1e-8 }),
      { kind: "failed", reason: "numeric" });
    assert.equal(contacts.status, "idle");
    compiler.close(); intake.close();
    assert.equal(JSON.stringify(scene.geometry), beforeContacts); assert(assembled.plan.hasAccepted(scene.geometry));
    assert.equal(scene.source.semanticIrDigest, owner.semanticIrDigest);
    assert.equal(scene.source.admissionEvidenceDigest, owner.admissionEvidenceDigest);
    assert.equal(scene.assembly, assembled); assert.equal(scene.topology, topology);
    for (const binding of scene.assembly.routes) {
      assert(scene.geometry.routes.some(route => route.id === binding.id));
      assert.equal(binding.selection.relation, relation);
      assert(scene.topology.entry(binding.selection.relation).handle.belongsTo(owner));
      for (const appearance of [binding.selection.source, binding.selection.target])
        assert(scene.topology.entry(appearance.occurrence).handle.belongsTo(owner));
    }
  } finally { compiler.close(); await contacts.close(); await executor.close(); }
});
