// Copyright © 2026 RawThoughts Enterprises Private Limited. AGPL-3.0-or-later.
import assert from "node:assert/strict";
import { Buffer } from "node:buffer";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { performance } from "node:perf_hooks";
import { fileURLToPath } from "node:url";

const require = createRequire(import.meta.url);
const { GeometryPlan, GeometryError, ElkExecutor, ContactExecutor } = require("../dist/index.js");
const manifest = require("../package.json");
const structuralSizes = Object.freeze([
  Object.freeze({ label: "small", nodes: 8 }),
  Object.freeze({ label: "medium", nodes: 256 }),
  Object.freeze({ label: "large", nodes: 2048 }),
]);
const optionalContactSizes = Object.freeze([
  Object.freeze({ label: "small", nodes: 8 }),
  Object.freeze({ label: "medium", nodes: 24 }),
  Object.freeze({ label: "large", nodes: 48 }),
]);
const structuralModes = Object.freeze(["full", "reused", "regional"]);
const allModes = Object.freeze([...structuralModes, "contacts"]);
const profile = Object.freeze({
  name: "representative-structural-v1",
  input: Object.freeze({ nodes: 2048, ports: 4094, relations: 2045, points: 65_440, depth: 4,
    coordinateMagnitude: 10_000_000 }),
  retainedOutputBytes: 2 * 1024 * 1024,
  worker: Object.freeze({
    requestBytes: 2 * 1024 * 1024,
    resultBytes: 4 * 1024 * 1024,
    activeMs: 5000,
    oldHeapMb: 256,
    youngHeapMb: 64,
  }),
  maximumEventLoopDelayMs: 100,
});

function integer(value, label, allowZero = false) {
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed) || parsed < (allowZero ? 0 : 1)) {
    throw new Error(`${label} must be ${allowZero ? "a nonnegative" : "a positive"} safe integer`);
  }
  return parsed;
}

function argumentsOf(values) {
  const result = { samples: 5, warmups: 1, output: undefined, child: undefined,
    organizationFixture: undefined, includeContacts: false };
  for (let index = 0; index < values.length; index++) {
    const value = values[index];
    if (value === "--samples") result.samples = integer(values[++index], "samples");
    else if (value === "--warmups") result.warmups = integer(values[++index], "warmups", true);
    else if (value === "--output") {
      const output = values[++index];
      if (typeof output !== "string" || output.length === 0) throw new Error("output must be a nonempty path");
      result.output = output;
    }
    else if (value === "--case") result.child = JSON.parse(values[++index]);
    else if (value === "--include-contacts") result.includeContacts = true;
    else if (value === "--organization-fixture") {
      const fixture = values[++index];
      if (typeof fixture !== "string" || fixture.length === 0) {
        throw new Error("organization fixture must be a nonempty path");
      }
      result.organizationFixture = fixture;
    }
    else throw new Error(`Unknown argument: ${value}`);
  }
  if (result.output !== undefined && (typeof result.output !== "string" || result.output.length === 0)) {
    throw new Error("output must be a nonempty path");
  }
  if (result.organizationFixture !== undefined
    && (typeof result.organizationFixture !== "string" || result.organizationFixture.length === 0)) {
    throw new Error("organization fixture must be a nonempty path");
  }
  return result;
}

function sceneNode(id, parentId) {
  return { id, ...(parentId === undefined ? {} : { parentId }), width: 52, height: 32,
    ports: [{ id: `${id}_w`, side: "WEST" }, { id: `${id}_e`, side: "EAST" }] };
}

function sceneRelation(id, source, target) {
  return { id, sourcePort: `${source}_e`, targetPort: `${target}_w` };
}

function fixture(nodeCount) {
  assert(nodeCount >= 8 && nodeCount % 2 === 0);
  const perRegion = (nodeCount - 2) / 2;
  const graph = { nodes: [
    { id: "left", width: 64, height: 48, ports: [{ id: "left_gate", side: "EAST" }] },
    { id: "right", width: 64, height: 48, ports: [{ id: "right_gate", side: "WEST" }] },
  ], relations: [] };
  for (const [prefix, parent] of [["l", "left"], ["r", "right"]]) {
    for (let index = 0; index < perRegion; index++) graph.nodes.push(sceneNode(`${prefix}${index}`, parent));
    for (let index = 0; index < perRegion - 1; index++) {
      graph.relations.push(sceneRelation(`${prefix}e${index}`, `${prefix}${index}`, `${prefix}${index + 1}`));
    }
  }
  graph.relations.push({ id: "bridge", sourcePort: "left_gate", targetPort: "right_gate" });
  const ports = graph.nodes.reduce((sum, node) => sum + node.ports.length, 0);
  return { graph, limits: { nodes: graph.nodes.length, ports, relations: graph.relations.length,
    points: Math.max(256, graph.relations.length * 32), depth: 4, coordinateMagnitude: 10_000_000 } };
}

function presentationIds(graph) {
  return [...graph.nodes.flatMap(node => [node.id, ...node.ports.map(port => port.id)]),
    ...graph.relations.map(relation => relation.id), ...(graph.bundles ?? []).map(bundle => bundle.id)];
}

function workerLimits(runs) {
  return { requestBytes: profile.worker.requestBytes, resultBytes: profile.worker.resultBytes,
    deadlineMs: profile.worker.activeMs, totalRequestBytes: runs * profile.worker.requestBytes,
    totalResultBytes: runs * profile.worker.resultBytes, totalActiveMs: runs * profile.worker.activeMs,
    oldHeapMb: profile.worker.oldHeapMb, youngHeapMb: profile.worker.youngHeapMb };
}

function structuralInput(limits) {
  const input = { nodes: limits.nodes, ports: limits.ports, relations: limits.relations,
    points: limits.points, depth: limits.depth, coordinateMagnitude: limits.coordinateMagnitude };
  for (const [resource, value] of Object.entries(input)) {
    assert(value <= profile.input[resource], `structural ${resource} exceeds the declared profile`);
  }
  return input;
}

function percentile(values, fraction) {
  assert(values.length > 0);
  const ordered = [...values].sort((left, right) => left - right);
  return ordered[Math.max(0, Math.ceil(ordered.length * fraction) - 1)];
}

const rounded = value => Math.round(value * 1000) / 1000;
function summary(values) {
  return Object.freeze({ minimum: rounded(Math.min(...values)), p50: rounded(percentile(values, 0.5)),
    p95: rounded(percentile(values, 0.95)), maximum: rounded(Math.max(...values)) });
}

function loopMonitor() {
  const intervalMs = 2, delays = [];
  let expected = performance.now() + intervalMs;
  let handle = setTimeout(function tick() {
    const now = performance.now(); delays.push(Math.max(0, now - expected)); expected = now + intervalMs;
    handle = setTimeout(tick, intervalMs);
  }, intervalMs);
  return () => {
    clearTimeout(handle);
    const samples = delays.length;
    // The action can finish in a microtask before the overdue timer observes a
    // synchronous acceptance/composition tail. Account for that final interval
    // without presenting it as another timer tick.
    delays.push(Math.max(0, performance.now() - expected));
    return { samples, maximumDelayMs: Math.max(...delays) };
  };
}

async function timed(action, retain) {
  const stop = loopMonitor(), started = performance.now();
  let responsiveness;
  try {
    const value = await action();
    const output = retain(value);
    const encoded = JSON.stringify(output);
    assert.equal(typeof encoded, "string", "retained output must encode to JSON");
    const elapsedMs = performance.now() - started;
    responsiveness = stop();
    return { value, output, encoded, encodedBytes: Buffer.byteLength(encoded, "utf8"), elapsedMs, responsiveness };
  } finally {
    if (responsiveness === undefined) stop();
  }
}

async function full(graph, limits, worker, report) {
  const plan = new GeometryPlan(graph, limits), executor = new ElkExecutor(worker);
  const measured = await timed(async () => {
    try { return await executor.run(plan); }
    finally { await executor.close(); }
  }, value => {
    assert.equal(value.kind, "completed"); return value.geometry;
  });
  assert.equal(executor.status, "closed", "layout worker must be released before publication");
  report(measured, executor.charges);
  return { plan, geometry: measured.output };
}

function contactProjection(analysis) {
  const branch = value => [value.segment.route.id, value.segment.index, value.backendTime, value.parameter,
    value.point.x, value.point.y, value.reportedPoint.x, value.reportedPoint.y];
  const contact = value => [branch(value.first), branch(value.second), value.separation, value.above];
  return { visitedPairs: analysis.visitedPairs, contacts: analysis.contacts.map(contact),
    overlaps: analysis.overlaps.map(value => [contact(value.start), contact(value.end)]) };
}

function contactLimits(geometry) {
  const segments = geometry.routes.reduce((sum, route) => sum + (route.kind === "line" ? 1 : (route.points.length - 1) / 3), 0);
  const pairs = segments * (segments + 1) / 2;
  return { segments, pairs, contacts: Math.max(1, pairs * 2), maximumSeparation: 1e-5, maximumControlDrift: 1e-8 };
}

async function runCase(spec) {
  const selected = (spec.mode === "contacts" ? optionalContactSizes : structuralSizes)
    .find(value => value.label === spec.size);
  assert(selected && allModes.includes(spec.mode));
  const certified = spec.mode !== "contacts";
  const samples = integer(spec.samples, "samples"), warmups = integer(spec.warmups, "warmups", true);
  const { graph, limits } = fixture(selected.nodes), runs = samples + warmups;
  const input = certified ? structuralInput(limits) : limits;
  let base;
  if (spec.mode !== "full") {
    let ignored;
    base = await full(graph, limits, workerLimits(1), value => { ignored = value; });
    assert(ignored.value.kind === "completed");
  }
  const declaredWorkerLimits = workerLimits(1);
  const durations = [], delays = [], ticks = [], retainedBytes = [], requestBytes = [], resultBytes = [], activeMs = [];
  let expected;
  const observe = (measured, include,
    charges = { requestBytes: 0, resultBytes: 0, activeMs: 0 }) => {
    const encoded = measured.encoded;
    if (expected === undefined) expected = encoded; else assert.equal(encoded, expected, "identical input must retain identical output");
    assert(charges.requestBytes <= declaredWorkerLimits.requestBytes);
    assert(charges.resultBytes <= declaredWorkerLimits.resultBytes);
    assert(charges.activeMs <= declaredWorkerLimits.totalActiveMs);
    if (certified) {
      assert(measured.encodedBytes <= profile.retainedOutputBytes,
        "retained structural output exceeds the supported representative ceiling");
      assert(measured.responsiveness.maximumDelayMs <= profile.maximumEventLoopDelayMs,
        "caller-thread delay exceeds the supported representative ceiling");
      if (spec.mode !== "reused") {
        assert(measured.responsiveness.samples > 0,
          "off-thread structural work must let the event loop advance");
      }
    }
    if (!include) return;
    durations.push(measured.elapsedMs); delays.push(measured.responsiveness.maximumDelayMs);
    ticks.push(measured.responsiveness.samples); retainedBytes.push(measured.encodedBytes);
    requestBytes.push(charges.requestBytes); resultBytes.push(charges.resultBytes); activeMs.push(charges.activeMs);
  };
  for (let iteration = 0; iteration < runs; iteration++) {
    const include = iteration >= warmups;
    const capture = (measured, charges) => observe(measured, include, charges);
    if (spec.mode === "full") await full(graph, limits, workerLimits(1), capture);
    else if (spec.mode === "reused") {
      const current = new GeometryPlan(graph, limits), pairs = presentationIds(graph).map(id => ({ priorId: id, currentId: id }));
      const measured = await timed(async () => current.reuseFrom(base.plan, base.geometry, pairs), value => {
        assert.equal(value.kind, "reused"); return value.geometry;
      });
      capture(measured);
    }
    else if (spec.mode === "regional") {
      const current = new GeometryPlan(graph, limits), boundary = base.plan.regionBoundary("left");
      assert.equal(boundary.kind, "region-boundary");
      const ready = current.prepareRegion(base.plan, base.geometry, "left", "left",
        boundary.retainedIds.map(id => ({ priorId: id, currentId: id })));
      assert.equal(ready.kind, "region-ready");
      const executor = new ElkExecutor(workerLimits(1));
      const measured = await timed(async () => {
        try {
          const local = await executor.run(ready.plan);
          assert.equal(local.kind, "completed"); return ready.complete(local.geometry);
        } finally { await executor.close(); }
      }, value => {
        assert.equal(value.kind, "region-composed"); return value.geometry;
      });
      assert.equal(executor.status, "closed", "regional worker must be released before publication");
      capture(measured, executor.charges);
    }
    else {
      const input = base.plan.contactInput(base.geometry, base.geometry.routes.map(route => route.id), contactLimits(base.geometry));
      const executor = new ContactExecutor(workerLimits(1));
      const measured = await timed(async () => {
        try { return await executor.run(input); }
        finally { await executor.close(); }
      }, value => {
        assert.equal(value.kind, "completed"); return contactProjection(value.analysis);
      });
      assert.equal(executor.status, "closed", "contact worker must be released before publication");
      capture(measured, executor.charges);
    }
  }
  return { size: selected.label, mode: spec.mode, fixture: input, samples, warmups,
    qualification: certified ? { status: "passed", profile: profile.name }
      : { status: "uncertified", reason: "contact estimates have no certified completeness or numeric-error bound" },
    determinism: { comparedRuns: runs, byteEqual: true,
      outputSha256: createHash("sha256").update(expected).digest("hex") }, wallTimeMs: summary(durations),
    eventLoopDelayMs: summary(delays), eventLoopTicks: { minimum: Math.min(...ticks), maximum: Math.max(...ticks) },
    observations: { wallTimeMs: durations.map(rounded), eventLoopDelayMs: delays.map(rounded), eventLoopTicks: ticks },
    retainedOutputBytes: { minimum: Math.min(...retainedBytes), maximum: Math.max(...retainedBytes) },
    workerCharges: { requestBytes: { minimum: Math.min(...requestBytes), maximum: Math.max(...requestBytes) },
      resultBytes: { minimum: Math.min(...resultBytes), maximum: Math.max(...resultBytes) },
      activeMs: summary(activeMs) },
    process: { scope: "case process including fixture setup, warmups and measured samples",
      maximumResidentSetKiB: process.resourceUsage().maxRSS, finalResidentSetBytes: process.memoryUsage.rss() } };
}

async function boundEvidence() {
  const { graph, limits } = fixture(8);
  assert.throws(() => new GeometryPlan(graph, { ...limits, nodes: graph.nodes.length - 1 }),
    error => error instanceof GeometryError && error.kind === "bounds");
  const plan = new GeometryPlan(graph, limits);
  const request = new ElkExecutor({ ...workerLimits(1), requestBytes: 1 });
  const response = new ElkExecutor({ ...workerLimits(1), resultBytes: 1 });
  try {
    assert.deepEqual(await request.run(plan), { kind: "failed", reason: "request-bytes" });
    assert.deepEqual(await response.run(plan), { kind: "failed", reason: "result-bytes" });
  } finally { await request.close(); await response.close(); }
  const accepted = await full(graph, limits, workerLimits(1), () => {});
  assert.throws(() => accepted.plan.contactInput(accepted.geometry, accepted.geometry.routes.map(route => route.id),
    { ...contactLimits(accepted.geometry), segments: 1 }), error => error?.reason === "bounds");
  return { geometryCount: "rejected-before-worker", requestBytes: "failed:request-bytes",
    resultBytes: "failed:result-bytes", contactSegments: "rejected-before-worker" };
}

function structuralEquivalence(cases) {
  return structuralSizes.map(size => {
    const selected = cases.filter(value => value.size === size.label);
    assert.deepEqual(selected.map(value => value.mode), structuralModes);
    const digests = new Set(selected.map(value => value.determinism.outputSha256));
    assert.equal(digests.size, 1,
      `${size.label} full, reused and regional results must retain identical bytes`);
    return { size: size.label, modes: structuralModes, byteEqual: true,
      outputSha256: selected[0].determinism.outputSha256 };
  });
}

function runChild(file, spec) {
  const child = spawnSync(process.execPath, [file, "--case", JSON.stringify(spec)], { encoding: "utf8",
    maxBuffer: 16 * 1024 * 1024, env: { ...process.env, NODE_NO_WARNINGS: "1" } });
  if (child.error) throw child.error;
  if (child.status !== 0) throw new Error(`Qualification ${spec.size}/${spec.mode} failed:\n${child.stderr}`);
  return JSON.parse(child.stdout);
}

async function main() {
  const options = argumentsOf(process.argv.slice(2));
  if (options.organizationFixture) {
    throw new Error("organization qualification requires a recipient-qualified PaleJS acquisition route; a source fixture cannot grant inspection authority");
  }
  if (options.child) {
    process.stdout.write(`${JSON.stringify(await runCase(options.child))}\n`); return;
  }
  const file = fileURLToPath(import.meta.url), results = [];
  for (const size of structuralSizes) for (const mode of structuralModes) {
    results.push(runChild(file, { size: size.label, mode, samples: options.samples, warmups: options.warmups }));
  }
  const contactCases = options.includeContacts ? optionalContactSizes.map(size => runChild(file,
    { size: size.label, mode: "contacts", samples: options.samples, warmups: options.warmups })) : [];
  const report = { reportKind: "performance-qualification", package: manifest.name, packageVersion: manifest.version,
    runtime: { node: process.version, platform: process.platform, architecture: process.arch,
      elk: require("elkjs/package.json").version, paper: require("paper/package.json").version },
    profile: { ...profile,
      algorithms: { elk: require("elkjs/package.json").version,
        rootOptions: new GeometryPlan(fixture(8).graph, fixture(8).limits).executionInput().algorithm.rootOptions },
      measuredOnly: ["wall time", "total process RSS", "backend temporary allocation within total process RSS"] },
    configuration: { samples: options.samples, warmups: options.warmups, structuralSizes,
      optionalContactSizes,
      interpretation: "Structural byte and active-time ceilings are enforced. The caller-delay threshold qualifies observed scheduling on this representative run; it is not runtime admission control. Wall time, total process RSS, and backend temporary allocation remain measured evidence." },
    cases: results, structuralEquivalence: structuralEquivalence(results),
    optionalContactAnalysis: options.includeContacts
      ? { status: "measured", certification: "uncertified", cases: contactCases,
        reason: "Contact estimates have no certified completeness or numeric-error bound." }
      : { status: "not-run", certification: "uncertified",
        reason: "Pass --include-contacts to measure optional contact estimates separately." },
    boundEvidence: await boundEvidence(),
    recipientQualifiedOrganization: {
      status: "blocked",
      dependency: "PaleJS resident acquisition of recipient-qualified inspection handles",
      reason: "A source fixture cannot grant inspection authority.",
    } };
  const encoded = `${JSON.stringify(report, null, 2)}\n`;
  if (options.output) await writeFile(options.output, encoded);
  process.stdout.write(encoded);
}

await main();
