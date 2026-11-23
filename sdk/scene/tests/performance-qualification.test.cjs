"use strict";
const assert = require("node:assert/strict");
const { spawnSync } = require("node:child_process");
const { test } = require("node:test");
const { join } = require("node:path");

test("performance qualification covers each deterministic mode and declared refusal", () => {
  for (const option of ["--output", "--organization-fixture"]) {
    const missing = spawnSync(process.execPath, [join(__dirname, "../tools/qualify-performance.mjs"), option],
      { cwd: join(__dirname, ".."), encoding: "utf8", maxBuffer: 1024 * 1024 });
    assert.ifError(missing.error); assert.equal(missing.signal, null); assert.notEqual(missing.status, 0);
    assert.match(missing.stderr, /must be a nonempty path/);
  }
  const result = spawnSync(process.execPath,
    [join(__dirname, "../tools/qualify-performance.mjs"), "--samples", "1", "--warmups", "1"],
    { cwd: join(__dirname, ".."), encoding: "utf8", maxBuffer: 16 * 1024 * 1024 });
  assert.ifError(result.error); assert.equal(result.signal, null); assert.equal(result.status, 0, result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(report.reportKind, "performance-qualification");
  assert.equal(report.package, "@rtepl/pale-scene");
  assert.equal(report.profile.name, "representative-structural-v1");
  assert.deepEqual(report.profile.input,
    { nodes: 2048, ports: 4094, relations: 2045, points: 65440, depth: 4, coordinateMagnitude: 10000000 });
  assert.equal(report.profile.algorithms.elk, "0.12.0");
  assert.equal(report.profile.algorithms.rootOptions["elk.algorithm"], "layered");
  assert.deepEqual(report.configuration.structuralSizes,
    [{ label: "small", nodes: 8 }, { label: "medium", nodes: 256 }, { label: "large", nodes: 2048 }]);
  assert.equal(report.cases.length, 9);
  for (const size of ["small", "medium", "large"]) {
    assert.deepEqual(report.cases.filter(value => value.size === size).map(value => value.mode),
      ["full", "reused", "regional"]);
  }
  for (const value of report.cases) {
    assert.equal(value.samples, 1); assert.equal(value.warmups, 1);
    assert.equal(value.qualification.status, "passed");
    assert.equal(value.qualification.profile, report.profile.name);
    assert.equal(value.determinism.comparedRuns, 2); assert.equal(value.determinism.byteEqual, true);
    assert.match(value.determinism.outputSha256, /^[0-9a-f]{64}$/);
    assert(value.wallTimeMs.p50 >= 0); assert(value.eventLoopDelayMs.p95 >= 0);
    assert(value.eventLoopDelayMs.maximum <= report.profile.maximumEventLoopDelayMs);
    assert.equal(value.observations.wallTimeMs.length, 1);
    assert.equal(value.observations.eventLoopDelayMs.length, 1);
    assert.equal(value.observations.eventLoopTicks.length, 1);
    assert(value.retainedOutputBytes.minimum > 0);
    assert(value.retainedOutputBytes.maximum <= report.profile.retainedOutputBytes);
    assert(value.process.maximumResidentSetKiB > 0); assert(value.process.finalResidentSetBytes > 0);
    assert(value.workerCharges.requestBytes.maximum >= 0);
    assert(value.workerCharges.resultBytes.maximum >= 0);
    for (const [resource, limit] of Object.entries(report.profile.input)) {
      assert(value.fixture[resource] <= limit, `${value.size}/${value.mode} ${resource}`);
    }
    if (value.mode === "reused") {
      assert.deepEqual(value.workerCharges, {
        requestBytes: { minimum: 0, maximum: 0 }, resultBytes: { minimum: 0, maximum: 0 },
        activeMs: { minimum: 0, p50: 0, p95: 0, maximum: 0 },
      });
    } else {
      assert(value.workerCharges.requestBytes.minimum > 0);
      assert(value.workerCharges.resultBytes.minimum > 0);
      assert(value.eventLoopTicks.minimum > 0);
    }
  }
  assert.deepEqual(report.cases.find(value => value.size === "large" && value.mode === "full").fixture,
    report.profile.input);
  assert.deepEqual(report.structuralEquivalence.map(value =>
    ({ size: value.size, modes: value.modes, byteEqual: value.byteEqual })), [
    { size: "small", modes: ["full", "reused", "regional"], byteEqual: true },
    { size: "medium", modes: ["full", "reused", "regional"], byteEqual: true },
    { size: "large", modes: ["full", "reused", "regional"], byteEqual: true },
  ]);
  for (const evidence of report.structuralEquivalence) {
    const cases = report.cases.filter(value => value.size === evidence.size);
    assert(cases.every(value => value.determinism.outputSha256 === evidence.outputSha256));
  }
  assert.deepEqual(report.optionalContactAnalysis, {
    status: "not-run", certification: "uncertified",
    reason: "Pass --include-contacts to measure optional contact estimates separately.",
  });
  assert.deepEqual(report.boundEvidence, {
    geometryCount: "rejected-before-worker", requestBytes: "failed:request-bytes",
    resultBytes: "failed:result-bytes", contactSegments: "rejected-before-worker",
  });
  assert.deepEqual(report.recipientQualifiedOrganization, {
    status: "blocked",
    dependency: "PaleJS resident acquisition of recipient-qualified inspection handles",
    reason: "A source fixture cannot grant inspection authority.",
  });

  const deniedFixture = spawnSync(process.execPath,
    [join(__dirname, "../tools/qualify-performance.mjs"), "--organization-fixture", "fixture"],
    { cwd: join(__dirname, ".."), encoding: "utf8", maxBuffer: 1024 * 1024 });
  assert.ifError(deniedFixture.error); assert.equal(deniedFixture.signal, null);
  assert.notEqual(deniedFixture.status, 0);
  assert.match(deniedFixture.stderr, /source fixture cannot grant inspection authority/);

  const contacts = spawnSync(process.execPath,
    [join(__dirname, "../tools/qualify-performance.mjs"), "--case",
      JSON.stringify({ size: "small", mode: "contacts", samples: 1, warmups: 0 })],
    { cwd: join(__dirname, ".."), encoding: "utf8", maxBuffer: 4 * 1024 * 1024 });
  assert.ifError(contacts.error); assert.equal(contacts.signal, null); assert.equal(contacts.status, 0, contacts.stderr);
  const contact = JSON.parse(contacts.stdout);
  assert.equal(contact.mode, "contacts");
  assert.equal(contact.qualification.status, "uncertified");
  assert.match(contact.qualification.reason, /no certified completeness or numeric-error bound/);
});
