"use strict";
const assert = require("node:assert/strict");
const { test } = require("node:test");
const { readFileSync } = require("node:fs");
const { dirname, join, resolve } = require("node:path");
const { Pale, ObservationProfileAssociation, ObservationProfileAssociationResult,
  ObservationProfileAssociationResultKind, ObservationProfileError,
  ObservationSnapshot } = require("@rtepl/pale");
const { ElkExecutor } = require("@rtepl/pale-scene");
const { EffectiveProfileSceneAssociation, OrganizationIntake, OrganizationTopology,
  OrganizationSceneCompiler, associateEffectiveProfileObservation } = require("@rtepl/pale-scene/organization");

const organizationFixture = process.env.PALE_SCENE_TEST_ORGANIZATION_FIXTURE;
assert(organizationFixture, "Supply the canonical organization fixture");
const oneShot = resolve(dirname(organizationFixture), "cross-leaf.v1/one-shot");
const workspaceFiles = ["composition.toml", "pale-workspace.toml",
  "workspace-resolution.lock.toml", "workspace-universe.lock.toml"];
const organizationFiles = ["organization.toml", "responsibility.toml"];
const geometryLimits = { nodes: 100, ports: 100, relations: 100, points: 100000, depth: 16,
  coordinateMagnitude: 1e6 };
const query = { inspected: 2, returned: 2, copiedStringBytes: 65536 };
const intakeLimits = { queries: 2000, recordVisits: 5000, sourceVisits: 5000, records: 1000,
  occurrences: 2000, sourceOccurrences: 5000, copiedStringBytes: 32 * 1024 * 1024 };
const organizationIntake = (owner, selectedQuery, selectedLimits, budget) =>
  new OrganizationIntake(owner, selectedQuery, selectedLimits, budget);
const retainedComparison = comparison => comparison;

function selected(t, regional = false) {
  const documents = workspaceFiles.map(label => ({ label, bytes: readFileSync(join(oneShot, "workspace", label)) }));
  const manifest = documents.find(document => document.label === "pale-workspace.toml");
  manifest.bytes = Buffer.from(manifest.bytes.toString()
    .replace("responsibility_derivations = []", 'responsibility_derivations = ["responsibility.toml"]')
    .replace("organizational_translations = []", 'organizational_translations = ["organization.toml"]'));
  for (const label of organizationFiles) documents.push({ label, bytes: readFileSync(join(organizationFixture, label)) });
  const api = new Pale();
  const owner = api.compilePortableWorkspace({ rootManifest: "pale-workspace.toml", documents });
  const privateDocument = (label, name) => ({ label, bytes: readFileSync(join(oneShot, "private", name)) });
  const profile = api.compileEffectiveProfile(owner, {
    projectId: "demo", executionWorkspaceId: "default", machineId: "machine",
    privateInputs: {
      capabilities: privateDocument("capabilities.toml", "capabilities.toml"),
      machineBinding: privateDocument("machines/machine.toml", "machine-binding.toml"),
      distributionLock: privateDocument("distribution-lock.toml", "distribution-lock.toml"),
      lifecycleGuidance: [],
    },
    packageDocuments: ["package.toml", "role.md", "roles.toml"].map(name => ({
      label: `engine/${name}`, bytes: readFileSync(join(oneShot, "packages/engine", name)),
    })),
  });
  const snapshot = api.observeEffectiveProfile(profile);
  const intake = organizationIntake(owner, query, intakeLimits); t.after(() => intake.close());
  const occurrences = [];
  for (let page = 0; page < intakeLimits.queries; page++) {
    const step = intake.next();
    if (step.kind === "enumeration-exhausted") break;
    assert.equal(step.kind, "page"); occurrences.push(...step.occurrences);
  }
  const topology = new OrganizationTopology(intake, occurrences,
    { records: 1000, appearances: 2000, routeAppearances: 3000, identityCodeUnits: 1000000 });
  const relation = topology.relations.find(item => topology.entry(item).row.relationId === "relation.collaboration");
  assert(relation); const endpoints = topology.endpoints(relation);
  assert.equal(endpoints.from.kind, "included"); assert.equal(endpoints.to.kind, "included");
  const source = endpoints.from.appearances[0], target = endpoints.to.appearances[0];
  const root = topology.appearances.find(item => item.endpoint.kind === "scope"
    && item.endpoint.scope.namespace === "alpha");
  const outside = topology.appearances.find(item => item.endpoint.kind === "scope"
    && item.endpoint.scope.namespace === "beta");
  const placements = regional
    ? [{ appearance: root, width: 80, height: 40 }, { appearance: source, parent: root, width: 80, height: 160 },
      { appearance: target, parent: root, width: 80, height: 40 }, { appearance: outside, width: 80, height: 40 }]
    : [source, target].map(appearance => ({ appearance, width: 80, height: 40 }));
  const assembly = topology.geometry(placements, [{ relation, source, target }], geometryLimits);
  const compiler = new OrganizationSceneCompiler(intake); compiler.select(topology, assembly); t.after(() => compiler.close());
  return { api, owner, profile, snapshot, intake, topology, assembly, compiler, relation, source, target, root, outside };
}

function executor(t) {
  const value = new ElkExecutor({ requestBytes: 1e6, resultBytes: 1e6, deadlineMs: 10000,
    totalRequestBytes: 10e6, totalResultBytes: 10e6, totalActiveMs: 60000, oldHeapMb: 128, youngHeapMb: 32 });
  t.after(() => value.close()); return value;
}
async function full(t, selection) {
  const result = await selection.compiler.layout(executor(t)); assert.equal(result.kind, "completed"); return result.scene;
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
  for (let count = 0; count < 100; count++) {
    const before = priorIntake.charges.queries, step = preparation.step();
    assert(priorIntake.charges.queries - before <= 1);
    if (step.kind !== "searching" && step.kind !== "record-matched") return step;
  }
  assert.fail("Bounded comparison did not finish");
}

test("effective-profile association retains exact scene, snapshot and all profile coordinates", async t => {
  const current = selected(t), scene = await full(t, current);
  const secondSnapshot = current.api.observeEffectiveProfile(current.profile);
  const charges = current.intake.charges, geometry = scene.geometry;
  const result = associateEffectiveProfileObservation(scene, current.snapshot, current.profile);
  assert.equal(result.kind, "associated"); const association = result.association;
  assert(association instanceof EffectiveProfileSceneAssociation);
  assert.equal(association.scene, scene); assert.equal(association.snapshot, current.snapshot);
  assert.equal(association.sceneGeneration, scene.generation);
  assert.equal(association.sceneGeneration.belongsTo(scene), true);
  assert.deepEqual(association.profileCoordinates, current.profile.coordinates());
  assert.deepEqual(Object.keys(association.profileCoordinates).sort(), ["compositionPlanDigest", "executionWorkspaceId",
    "instanceDigest", "machineId", "packageSemanticIrDigest", "profileDigest", "projectId",
    "selectedTopologyDigest", "workspaceAdmissionEvidenceDigest", "workspaceId", "workspaceSemanticIrDigest"]);
  assert.deepEqual(association.observationCursor, current.snapshot.cursor);
  assert.deepEqual(secondSnapshot.cursor, current.snapshot.cursor);
  assert.equal(association.belongsTo(scene, current.snapshot), true);
  assert.equal(association.belongsTo(scene, secondSnapshot), false);
  assert.deepEqual(current.intake.charges, charges);
  assert.equal(scene.geometry, geometry);
  assert.deepEqual(Object.keys(association), []);
  assert.throws(() => new EffectiveProfileSceneAssociation(), TypeError);
});

test("association uses native prototypes and refuses equal-byte foreign ownership", async t => {
  const current = selected(t), foreign = selected(t);
  const scene = await full(t, current), foreignScene = await full(t, foreign);
  assert.deepEqual(current.profile.coordinates(), foreign.profile.coordinates());
  const replacements = [
    [ObservationSnapshot.prototype, "associateProfile", { value: () => assert.fail("replacement was called") }],
    [ObservationProfileAssociationResult.prototype, "kind", { get: () => ObservationProfileAssociationResultKind.Refused }],
    [ObservationProfileAssociationResult.prototype, "association", { get: () => null }],
    [ObservationProfileAssociationResult.prototype, "error", { get: () => ObservationProfileError.NoProfileSource }],
    [ObservationProfileAssociation.prototype, "coordinates", { get: () => ({ workspaceId: "shadowed" }) }],
    [ObservationProfileAssociation.prototype, "cursor", { get: () => ({ kind: "shadowed" }) }],
    [ObservationProfileAssociation.prototype, "belongsTo", { value: () => false }],
  ];
  const originals = replacements.map(([target, name]) => [target, name,
    Object.getOwnPropertyDescriptor(target, name)]);
  for (const [target, name, descriptor] of replacements) {
    Object.defineProperty(target, name, { ...descriptor, configurable: true });
  }
  Object.defineProperty(current.profile, "coordinates", { value: { workspaceId: "shadowed" }, configurable: true });
  try {
    const result = associateEffectiveProfileObservation(scene, current.snapshot, current.profile);
    assert.equal(result.kind, "associated");
    assert.equal(result.association.profileCoordinates.workspaceId, "act-ws");
    assert.deepEqual(result.association.observationCursor, current.snapshot.cursor);
    assert.equal(result.association.belongsTo(scene, current.snapshot), true);
  } finally {
    for (const [target, name, descriptor] of originals) Object.defineProperty(target, name, descriptor);
  }
  const foreignProfile = associateEffectiveProfileObservation(scene, current.snapshot, foreign.profile);
  assert.deepEqual(foreignProfile, { kind: "refused", error: ObservationProfileError.SourceMismatch });
  const foreignOwner = associateEffectiveProfileObservation(foreignScene, current.snapshot, current.profile);
  assert.deepEqual(foreignOwner, { kind: "refused", error: ObservationProfileError.SourceMismatch });
});

test("reused and regional scenes associate only with their current owner", async t => {
  const before = selected(t), after = selected(t), prior = await full(t, before);
  const comparison = retainedComparison(new Pale().comparePortableWorkspaces(before.owner, after.owner));
  const reused = finish(after.compiler.prepareReuse(prior, comparison, choices(before, after)), before.intake);
  assert.equal(reused.kind, "reused");
  assert.equal(associateEffectiveProfileObservation(reused.scene, after.snapshot, after.profile).kind, "associated");
  assert.deepEqual(associateEffectiveProfileObservation(reused.scene, before.snapshot, before.profile),
    { kind: "refused", error: ObservationProfileError.SourceMismatch });

  const regionalBefore = selected(t, true), regionalAfter = selected(t, true), regionalPrior = await full(t, regionalBefore);
  const prepared = finish(regionalAfter.compiler.prepareRegion(regionalPrior,
    retainedComparison(new Pale().comparePortableWorkspaces(regionalBefore.owner, regionalAfter.owner)),
    { prior: regionalBefore.root, current: regionalAfter.root }, choices(regionalBefore, regionalAfter, true)),
  regionalBefore.intake);
  assert.equal(prepared.kind, "region-ready"); const completed = await prepared.layout(executor(t));
  assert.equal(completed.kind, "completed");
  assert.equal(associateEffectiveProfileObservation(completed.scene,
    regionalAfter.snapshot, regionalAfter.profile).kind, "associated");
  assert.deepEqual(associateEffectiveProfileObservation(completed.scene,
    regionalBefore.snapshot, regionalBefore.profile),
  { kind: "refused", error: ObservationProfileError.SourceMismatch });
});

test("refused association preserves the retained scene and intake charges", async t => {
  const current = selected(t), foreign = selected(t), scene = await full(t, current);
  const before = {
    source: scene.source, lens: scene.lens, constraints: scene.constraints,
    execution: scene.execution, topology: scene.topology, assembly: scene.assembly,
    geometry: scene.geometry, declaredGates: scene.declaredGates,
    correspondence: scene.correspondence, charges: current.intake.charges,
  };
  const result = associateEffectiveProfileObservation(scene, current.snapshot, foreign.profile);
  assert.deepEqual(result, { kind: "refused", error: ObservationProfileError.SourceMismatch });
  assert.equal(scene.source, before.source); assert.equal(scene.lens, before.lens);
  assert.equal(scene.constraints, before.constraints); assert.equal(scene.execution, before.execution);
  assert.equal(scene.topology, before.topology); assert.equal(scene.assembly, before.assembly);
  assert.equal(scene.geometry, before.geometry); assert.equal(scene.declaredGates, before.declaredGates);
  assert.equal(scene.correspondence, before.correspondence);
  assert.deepEqual(current.intake.charges, before.charges);
});
