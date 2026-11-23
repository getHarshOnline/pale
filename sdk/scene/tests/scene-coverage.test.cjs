"use strict";
const assert = require("node:assert/strict");
const { test } = require("node:test");
const { readFileSync } = require("node:fs");
const { join } = require("node:path");
const { Pale } = require("@rtepl/pale");
const { OrganizationIntake, OrganizationSceneCoverage,
  OrganizationTopology } = require("@rtepl/pale-scene/organization");

const fixture = process.env.PALE_SCENE_TEST_ORGANIZATION_FIXTURE;
assert(fixture, "Supply the existing compiler-backed organization fixture");
const files = ["pale-workspace.toml", "organization.toml", "responsibility.toml",
  "resolution.lock.toml", "universe.lock.toml"];
const query = { inspected: 2, returned: 2, copiedStringBytes: 65536 };
const limits = { queries: 2000, recordVisits: 5000, sourceVisits: 5000, records: 1000,
  occurrences: 2000, sourceOccurrences: 5000, copiedStringBytes: 32 * 1024 * 1024 };
const topologyLimits = { records: 1000, appearances: 2000, routeAppearances: 3000,
  identityCodeUnits: 1000000 };
const organizationIntake = (owner, selectedQuery, selectedLimits, budget) =>
  new OrganizationIntake(owner, selectedQuery, selectedLimits, budget);

function compilation() {
  return new Pale().compilePortableWorkspace({ rootManifest: files[0],
    documents: files.map(label => ({ label, bytes: readFileSync(join(fixture, label)) })) });
}

function intake(selectedQuery = query, selectedLimits = limits) {
  return organizationIntake(compilation(), selectedQuery, selectedLimits);
}

function exhaust(selected) {
  const occurrences = [];
  for (let page = 0; page < limits.queries; page++) {
    const step = selected.next();
    if (step.kind === "enumeration-exhausted") return occurrences;
    assert.equal(step.kind, "page");
    occurrences.push(...step.occurrences);
  }
  assert.fail("Organization fixture did not exhaust");
}

test("coverage distinguishes final-page exhaustion, an open frontier, and an exact resource stop", () => {
  const whole = intake({ inspected: 256, returned: 256, copiedStringBytes: 65536 });
  const open = intake();
  const stopped = intake(query, { ...limits, recordVisits: 1 });
  try {
    const final = whole.next();
    assert.equal(final.kind, "page");
    const complete = new OrganizationTopology(whole, final.occurrences, topologyLimits);
    assert.deepEqual(complete.coverage.enumeration, { kind: "exhausted" });
    assert.deepEqual(complete.coverage.selection, { kind: "all-enumerated-occurrences" });
    assert.deepEqual(whole.next(), { kind: "enumeration-exhausted" });

    const unfinished = new OrganizationTopology(open, [], topologyLimits);
    assert.deepEqual(unfinished.coverage.enumeration, { kind: "incomplete" });
    assert.deepEqual(unfinished.coverage.selection, { kind: "all-enumerated-occurrences" });

    assert.deepEqual(stopped.next(), { kind: "resource-stop", resource: "recordVisits" });
    const resourceStopped = new OrganizationTopology(stopped, [], topologyLimits);
    assert.deepEqual(resourceStopped.coverage.enumeration,
      { kind: "resource-stopped", resource: "recordVisits" });
    assert.deepEqual(resourceStopped.coverage.selection, { kind: "all-enumerated-occurrences" });
  } finally { whole.close(); open.close(); stopped.close(); }
});

test("coverage treats omitted, queried, and repeated occurrences as a selected subset", () => {
  const selected = intake();
  try {
    const first = selected.next();
    assert.equal(first.kind, "page");
    assert(first.occurrences.length > 1);
    const omitted = new OrganizationTopology(selected, first.occurrences.slice(0, 1), topologyLimits);
    assert.deepEqual(omitted.coverage.enumeration, { kind: "incomplete" });
    assert.deepEqual(omitted.coverage.selection, { kind: "subset" });

    const endpoint = selected.endpoint({ kind: "role", role: { namespace: "alpha", roleId: "accountable-a" } });
    assert.equal(endpoint.kind, "page");
    const queried = endpoint.occurrences[0];
    assert(queried);
    const repeated = selected.repeat(queried);
    assert.equal("kind" in repeated, false);
    const queryTopology = new OrganizationTopology(selected, [queried, repeated], topologyLimits);
    assert.deepEqual(queryTopology.coverage.enumeration, { kind: "incomplete" });
    assert.deepEqual(queryTopology.coverage.selection, { kind: "subset" });
  } finally { selected.close(); }
});

test("issued coverage is immutable and does not change when its intake advances", () => {
  const selected = intake();
  try {
    const first = selected.next();
    assert.equal(first.kind, "page");
    const before = new OrganizationTopology(selected, first.occurrences, topologyLimits);
    const retained = before.coverage;
    assert.deepEqual(retained.enumeration, { kind: "incomplete" });
    assert.deepEqual(retained.selection, { kind: "all-enumerated-occurrences" });

    const remaining = exhaust(selected);
    const current = new OrganizationTopology(selected, [...first.occurrences, ...remaining], topologyLimits);
    assert.deepEqual(current.coverage.enumeration, { kind: "exhausted" });
    assert.deepEqual(current.coverage.selection, { kind: "all-enumerated-occurrences" });
    assert.deepEqual(retained.enumeration, { kind: "incomplete" });
    assert(Object.isFrozen(retained) && Object.isFrozen(retained.enumeration)
      && Object.isFrozen(retained.selection));
    assert.throws(() => { retained.enumeration.kind = "exhausted"; }, TypeError);
    assert.throws(() => new OrganizationSceneCoverage(Symbol(), { kind: "exhausted" },
      { kind: "all-enumerated-occurrences" }), TypeError);
  } finally { selected.close(); }
});

test("a topology rejects occurrences issued by another intake", () => {
  const selected = intake(), foreign = intake();
  try {
    const localPage = selected.next(), foreignPage = foreign.next();
    assert.equal(localPage.kind, "page"); assert.equal(foreignPage.kind, "page");
    assert.throws(() => new OrganizationTopology(selected, [foreignPage.occurrences[0]], topologyLimits),
      { kind: "foreign-occurrence" });
    const local = new OrganizationTopology(selected, localPage.occurrences, topologyLimits);
    assert.deepEqual(local.coverage.selection, { kind: "all-enumerated-occurrences" });
  } finally { selected.close(); foreign.close(); }
});

test("coverage rejects sparse and prototype-supplied occurrence lists", () => {
  const selected = intake();
  try {
    const page = selected.next();
    assert.equal(page.kind, "page");
    const sparse = new Array(1);
    assert.throws(() => new OrganizationTopology(selected, sparse, topologyLimits), { kind: "bounds" });

    const inherited = new Array(1);
    const prototype = Object.create(Array.prototype);
    Object.defineProperty(prototype, 0, { value: page.occurrences[0], enumerable: true });
    Object.setPrototypeOf(inherited, prototype);
    assert.equal(inherited[0], page.occurrences[0]);
    assert.throws(() => new OrganizationTopology(selected, inherited, topologyLimits), { kind: "bounds" });
  } finally { selected.close(); }
});
