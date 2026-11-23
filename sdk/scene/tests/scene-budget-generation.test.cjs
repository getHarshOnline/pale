"use strict";
const assert = require("node:assert/strict");
const { test } = require("node:test");
const { readFileSync } = require("node:fs");
const { join } = require("node:path");
const { Pale } = require("@rtepl/pale");
const { ContactExecutor, ElkExecutor, SceneBudget, SceneBudgetError } = require("@rtepl/pale-scene");
const { OrganizationIntake, OrganizationSceneGeneration, OrganizationTopology,
  OrganizationSceneCompiler } = require("@rtepl/pale-scene/organization");

const fixture = process.env.PALE_SCENE_TEST_ORGANIZATION_FIXTURE;
assert(fixture, "Supply the canonical organization fixture");
const files = ["pale-workspace.toml", "organization.toml", "responsibility.toml",
  "resolution.lock.toml", "universe.lock.toml"];
const query = { inspected: 8, returned: 8, copiedStringBytes: 1024 * 1024 };
const intakeLimits = { queries: 1000, recordVisits: 8000, sourceVisits: 8000, records: 2000,
  occurrences: 4000, sourceOccurrences: 8000, copiedStringBytes: 64 * 1024 * 1024 };
const topologyLimits = { records: 2000, appearances: 4000, routeAppearances: 4000,
  identityCodeUnits: 2 * 1024 * 1024 };
const geometryLimits = { nodes: 4000, ports: 8000, relations: 4000, points: 100000,
  depth: 16, coordinateMagnitude: 1e6 };
const ample = () => new SceneBudget({ workUnits: 1_000_000, retainedSceneBytes: 128 * 1024 * 1024,
  encodedOutputBytes: 32 * 1024 * 1024, deadlineMs: 60_000 });
const organizationIntake = (owner, selectedQuery, selectedLimits, budget) =>
  new OrganizationIntake(owner, selectedQuery, selectedLimits, budget);

function compile(native = false) {
  return new Pale().compilePortableWorkspace({ rootManifest: files[0], documents: files.map(label => ({ label,
    bytes: label === "organization.toml" && native
      ? Buffer.from(`${readFileSync(join(fixture, label), "utf8")}
[[relations]]
relation_id = "relation.scene-native-boundary"
class = "native_presence"
issuer = "issuer.scene-boundary"
state = "unknown"
[relations.from]
kind = "scope"
[relations.from.scope]
namespace = "alpha"
scope_id = "scope"
[relations.to]
kind = "native_resource"
resource = "external.scene-boundary"
[relations.interval]
start = 1
end_exclusive = 10
`) : readFileSync(join(fixture, label)) })) });
}

function executor(t) {
  const value = new ElkExecutor({ requestBytes: 2e6, resultBytes: 2e6, deadlineMs: 10000,
    totalRequestBytes: 4e6, totalResultBytes: 4e6, totalActiveMs: 20000,
    oldHeapMb: 128, youngHeapMb: 32 });
  t.after(() => value.close()); return value;
}

function contactExecutor(t) {
  const value = new ContactExecutor({ requestBytes: 2e6, resultBytes: 2e6, deadlineMs: 10000,
    totalRequestBytes: 4e6, totalResultBytes: 4e6, totalActiveMs: 20000,
    oldHeapMb: 128, youngHeapMb: 32 });
  t.after(() => value.close()); return value;
}

function selected(t, budget, native = true) {
  const owner = compile(native), intake = organizationIntake(owner, query, intakeLimits, budget);
  t.after(() => intake.close());
  const occurrences = [];
  for (let count = 0; count < intakeLimits.queries; count++) {
    const step = intake.next();
    if (step.kind === "enumeration-exhausted") break;
    assert.equal(step.kind, "page"); occurrences.push(...step.occurrences);
  }
  const topology = new OrganizationTopology(intake, occurrences, topologyLimits);
  const relation = topology.relations.find(item => topology.entry(item).row.relationId === "relation.collaboration");
  const service = topology.relations.find(item => topology.entry(item).row.kind === "service");
  const boundary = native
    ? topology.relations.find(item => topology.entry(item).row.relationId === "relation.scene-native-boundary")
    : undefined;
  assert(relation && service && (!native || boundary));
  const selections = [relation, service, ...(boundary ? [boundary] : [])].map(item => {
    const endpoints = topology.endpoints(item);
    assert.equal(endpoints.from.kind, "included"); assert.equal(endpoints.to.kind, "included");
    return { relation: item, source: endpoints.from.appearances[0], target: endpoints.to.appearances[0] };
  });
  const appearances = [...new Set(selections.flatMap(item => [item.source, item.target]))];
  const assembly = topology.geometry(appearances.map(appearance => ({ appearance, width: 80, height: 40 })),
    selections, geometryLimits);
  const compiler = new OrganizationSceneCompiler(intake); compiler.select(topology, assembly);
  t.after(() => compiler.close()); return { owner, intake, topology, assembly, compiler };
}

test("one Scene budget spans intake, topology, layout, and accepted output", async t => {
  const budget = ample(), current = selected(t, budget);
  const result = await current.compiler.layout(executor(t));
  assert.equal(result.kind, "completed"); const scene = result.scene, generation = scene.generation;
  assert(generation instanceof OrganizationSceneGeneration);
  assert.equal(generation.belongsTo(scene), true);
  assert.equal(generation.source, scene.source); assert.equal(generation.lens, scene.lens);
  assert.equal(generation.coverage, scene.coverage); assert.equal(generation.constraints, scene.constraints);
  assert.equal(generation.geometry, scene.geometry);
  assert.equal(generation.correspondence, scene.correspondence);
  assert(generation.encodedOutputBytes > 0);
  assert.equal(generation.encodedOutputBytes, budget.charges.encodedOutputBytes);
  assert(budget.charges.workUnits > 0 && budget.charges.retainedSceneBytes > 0);
  const kinds = scene.boundaries.map(item => item.kind);
  assert.equal(kinds.filter(kind => kind === "native-boundary").length, 1);
  assert.equal(kinds.filter(kind => kind === "service-boundary").length, 1);
  assert.equal(kinds.filter(kind => kind === "acceptance-frontier").length, 5);
  assert.equal(scene.declaredGates.length, 1);
  assert.deepEqual(scene.boundaries.filter(item => item.kind === "acceptance-frontier").map(item => item.frontier),
    ["provider-completion", "provider-acceptance", "consumer-acceptance", "native-read-back", "pale-settlement"]);
  const beforeOrder = budget.charges;
  const order = scene.orderRoutes([...scene.geometry.routes].reverse().map(route => route.id));
  assert.deepEqual(order.routes.map(item => item.route.id), [...scene.geometry.routes].reverse().map(route => route.id));
  assert.deepEqual(order.routes.map(item => item.depth), order.routes.map((_, index) => index));
  assert(order.encodedOutputBytes > 0);
  assert(budget.charges.workUnits > beforeOrder.workUnits);
  assert(budget.charges.retainedSceneBytes > beforeOrder.retainedSceneBytes);
  assert(budget.charges.encodedOutputBytes > beforeOrder.encodedOutputBytes);
  const beforeContacts = budget.charges;
  const contacts = await current.compiler.contacts(scene, contactExecutor(t), order,
    { segments: 100, pairs: 10000, contacts: 10000,
      maximumSeparation: 1e-5, maximumControlDrift: 1e-8 });
  assert.equal(contacts.kind, "completed"); assert(contacts.encodedOutputBytes > 0);
  assert(budget.charges.retainedSceneBytes > beforeContacts.retainedSceneBytes);
  assert(budget.charges.encodedOutputBytes > beforeContacts.encodedOutputBytes);
  const foreign = selected(t, ample()), foreignResult = await foreign.compiler.layout(executor(t));
  assert.equal(foreignResult.kind, "completed");
  assert.deepEqual(foreignResult.scene.source, scene.source);
  assert.equal(generation.belongsTo(foreignResult.scene), false);
  assert(Object.isFrozen(scene.boundaries) && Object.isFrozen(generation));
  assert.throws(() => new OrganizationSceneGeneration(), TypeError);
});

test("aggregate budget keeps completed work charges on failure and checks deadlines before native work", async () => {
  const failedWork = ample();
  const invalidCopy = organizationIntake(compile(), { ...query, copiedStringBytes: 1 }, intakeLimits, failedWork);
  try {
    assert.throws(() => invalidCopy.next());
    assert.equal(failedWork.charges.workUnits, 1 + query.inspected);
  } finally { invalidCopy.close(); }

  const retained = new SceneBudget({ workUnits: 100, retainedSceneBytes: 1,
    encodedOutputBytes: 100, deadlineMs: 1000 });
  const retainedStop = organizationIntake(compile(), query, intakeLimits, retained);
  try {
    assert.throws(() => retainedStop.next(), { resource: "retainedSceneBytes" });
    assert.equal(retained.charges.workUnits, 1 + query.inspected);
    assert.equal(retained.charges.retainedSceneBytes, 0);
  } finally { retainedStop.close(); }

  const expired = new SceneBudget({ workUnits: 100, retainedSceneBytes: 100,
    encodedOutputBytes: 100, deadlineMs: 1 });
  await new Promise(resolve => setTimeout(resolve, 5));
  const late = organizationIntake(compile(), query, intakeLimits, expired);
  try {
    assert.throws(() => late.next(), { resource: "deadline" });
    assert.deepEqual(expired.charges, { workUnits: 0, retainedSceneBytes: 0, encodedOutputBytes: 0 });
  } finally { late.close(); }
  assert.throws(() => new SceneBudget({ workUnits: 0, retainedSceneBytes: 1,
    encodedOutputBytes: 1, deadlineMs: 1 }), SceneBudgetError);
});

test("an encoded-output stop publishes no generation and retains prior work", async t => {
  const budget = new SceneBudget({ workUnits: 1_000_000, retainedSceneBytes: 128 * 1024 * 1024,
    encodedOutputBytes: 1, deadlineMs: 60_000 });
  const current = selected(t, budget, false), before = budget.charges.workUnits;
  await assert.rejects(current.compiler.layout(executor(t)), { resource: "encodedOutputBytes" });
  assert(budget.charges.workUnits > before);
  assert.equal(budget.charges.encodedOutputBytes, 0);
});

test("route depth and contact annotations cannot escape the aggregate output ceiling", async t => {
  const probeBudget = ample(), probe = selected(t, probeBudget, false);
  const probeScene = (await probe.compiler.layout(executor(t))).scene;
  const sceneBytes = probeBudget.charges.encodedOutputBytes;
  const probeOrder = probeScene.orderRoutes(probeScene.geometry.routes.map(route => route.id));
  const orderBytes = probeBudget.charges.encodedOutputBytes;
  const probeContacts = await probe.compiler.contacts(probeScene, contactExecutor(t), probeOrder,
    { segments: 100, pairs: 10000, contacts: 10000,
      maximumSeparation: 1e-5, maximumControlDrift: 1e-8 });
  assert.equal(probeContacts.kind, "completed");
  assert(probeBudget.charges.encodedOutputBytes > orderBytes && orderBytes > sceneBytes);

  const depthBudget = new SceneBudget({ workUnits: 1_000_000, retainedSceneBytes: 128 * 1024 * 1024,
    encodedOutputBytes: sceneBytes, deadlineMs: 60_000 });
  const depth = selected(t, depthBudget, false);
  const depthScene = (await depth.compiler.layout(executor(t))).scene;
  assert.throws(() => depthScene.orderRoutes(depthScene.geometry.routes.map(route => route.id)),
    { resource: "encodedOutputBytes" });

  const contactBudget = new SceneBudget({ workUnits: 1_000_000, retainedSceneBytes: 128 * 1024 * 1024,
    encodedOutputBytes: orderBytes, deadlineMs: 60_000 });
  const annotated = selected(t, contactBudget, false);
  const annotatedScene = (await annotated.compiler.layout(executor(t))).scene;
  const annotatedOrder = annotatedScene.orderRoutes(annotatedScene.geometry.routes.map(route => route.id));
  await assert.rejects(annotated.compiler.contacts(annotatedScene, contactExecutor(t), annotatedOrder,
    { segments: 100, pairs: 10000, contacts: 10000,
      maximumSeparation: 1e-5, maximumControlDrift: 1e-8 }), { resource: "encodedOutputBytes" });
});
