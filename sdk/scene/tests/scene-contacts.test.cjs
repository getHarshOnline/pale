"use strict";
const assert = require("node:assert/strict");
const { test, mock } = require("node:test");
const threads = require("node:worker_threads");
const { join } = require("node:path");
const { mkdtempSync, existsSync, rmSync } = require("node:fs");
const { tmpdir } = require("node:os");
const { GeometryPlan, GeometryContactInput, ElkExecutor, ContactExecutor } = require("@rtepl/pale-scene");
const workerLimits = { requestBytes: 1e6, resultBytes: 1e6, deadlineMs: 10000,
  totalRequestBytes: 10e6, totalResultBytes: 10e6, totalActiveMs: 60000,
  oldHeapMb: 128, youngHeapMb: 32 };
const limits = { segments: 100, pairs: 5050, contacts: 500, maximumSeparation: 1e-5, maximumControlDrift: 1e-8 };
const geometryLimits = { nodes: 20, ports: 40, relations: 20, points: 1000, depth: 4, coordinateMagnitude: 10000 };
const loop = [[0, 0], [11, 4], [-2, 4], [9, 0]];
const line = [[0, 2.25], [9, 2.25]];

// Controlled curves through the real geometry acceptance owner, not ELK output.
// Separate tests below execute actual ELK and verify nested frame joins.
function curves(shapes = [loop, line, line]) {
  const nodes = [], relations = [], children = [], edges = [];
  for (const [index, controls] of shapes.entries()) {
    const points = controls.map(([x, y]) => ({ x: x + 20, y: y + 20 }));
    for (const [end, point] of [["a", points[0]], ["b", points.at(-1)]]) {
      const id = `n${index}${end}`, port = `p${index}${end}`;
      nodes.push({ id, width: 2, height: 2, ports: [{ id: port, side: "EAST" }] });
      children.push({ id, x: point.x - 2, y: point.y - 1, width: 2, height: 2,
        ports: [{ id: port, x: 2, y: 0, width: 2, height: 2 }] });
    }
    const id = `r${index}`, sourcePort = `p${index}a`, targetPort = `p${index}b`;
    relations.push({ id, sourcePort, targetPort });
    edges.push({ id, sources: [sourcePort], targets: [targetPort], container: "$scene",
      sections: [{ id: `s${index}`, incomingShape: sourcePort, outgoingShape: targetPort,
        startPoint: points[0], endPoint: points.at(-1), bendPoints: points.slice(1, -1) }] });
  }
  const graph = { nodes, relations }, plan = new GeometryPlan(graph, geometryLimits);
  const geometry = plan.accept({ id: "$scene", x: 0, y: 0, width: 100, height: 100, children, edges });
  return { graph, plan, geometry, input: plan.contactInput(geometry, relations.map(r => r.id), limits) };
}
function executor(t, values = workerLimits) { const result = new ContactExecutor(values); t.after(() => result.close()); return result; }
function closeTo(actual, expected) { assert(Math.abs(actual - expected) <= 1e-5, `${actual} versus ${expected}`); }

test("contact analysis preserves repeated visits, parallel witnesses, overlap ranges and explicit depth", async t => {
  const { plan, geometry, input } = curves(), worker = executor(t), before = JSON.stringify(geometry);
  const result = await worker.run(input); assert.equal(result.kind, "completed");
  const a = result.analysis; assert.equal(a.input, input); assert.equal(a.geometry, geometry);
  assert.equal(a.model.version, "0.12.18"); assert.equal(a.coverage, "not-established"); assert.equal(a.certifiedErrorBound, null);
  assert.equal(a.visitedPairs, 6); assert.equal(a.overlaps.length, 1);
  const visits = a.contacts.filter(c => c.first.segment.route.id === "r0" && c.second.segment.route.id === "r1");
  visits.sort((a, b) => a.first.parameter - b.first.parameter);
  assert.equal(visits.length, 2); closeTo(visits[0].first.parameter, .25); closeTo(visits[1].first.parameter, .75);
  for (const v of visits) { closeTo(v.second.parameter, .5); assert.equal(v.above, "second"); }
  assert.equal(a.contacts.filter(c => c.second.segment.route.id === "r2").length, 2);
  for (const c of a.contacts) for (const b of [c.first, c.second]) {
    assert.equal(b.segment, input.segments.find(s => s.route === b.segment.route && s.index === b.segment.index));
    assert(geometry.routes.includes(b.segment.route));
  }
  const reversed = await worker.run(plan.contactInput(geometry, ["r2", "r1", "r0"], limits));
  assert.equal(reversed.kind, "completed");
  assert(reversed.analysis.contacts.filter(c => c.first.segment.route !== c.second.segment.route).every(c => c.above === "first"));
  assert.deepEqual(a.contacts.map(c => [c.first.parameter, c.second.parameter]), reversed.analysis.contacts.map(c => [c.first.parameter, c.second.parameter]));
  assert.equal(JSON.stringify(geometry), before); assert(plan.hasAccepted(geometry)); assert(Object.isFrozen(a.contacts[0].first));
});

test("actual ELK nested routes use existing root-frame translations and exact local witnesses", async t => {
  const node = (id, parentId) => ({ id, ...(parentId ? { parentId } : {}), width: 40, height: 30,
    ports: [{ id: `${id}_in`, side: "WEST" }, { id: `${id}_out`, side: "EAST" }] });
  const graph = { nodes: [{ id: "group", width: 40, height: 30, ports: [] }, node("a", "group"), node("b", "group"), node("c")],
    relations: [["local", "a", "b"], ["cross", "b", "c"], ["parallel", "b", "c"]].map(([id, a, b]) => ({ id, sourcePort: `${a}_out`, targetPort: `${b}_in` })) };
  const plan = new GeometryPlan(graph, geometryLimits), elk = new ElkExecutor(workerLimits); t.after(() => elk.close());
  const layout = await elk.run(plan); assert.equal(layout.kind, "completed");
  const geometry = layout.geometry, input = plan.contactInput(geometry, ["parallel", "cross", "local"], limits);
  const local = geometry.routes.find(r => r.id === "local"), group = geometry.nodes.find(n => n.id === "group");
  assert.equal(local.containerId, "group");
  for (const s of input.segments.filter(s => s.route === local)) {
    for (let i = 0; i < s.rootControls.length; i++) {
      const p = local.points[s.index * (local.kind === "line" ? 1 : 3) + i];
      assert.deepEqual(s.rootControls[i], { x: p.x + group.bounds.x, y: p.y + group.bounds.y });
    }
  }
  const answer = await executor(t).run(input); assert.equal(answer.kind, "completed");
  assert.equal(answer.analysis.geometry, geometry); assert.equal(answer.analysis.visitedPairs, input.segments.length * (input.segments.length + 1) / 2);
  assert(plan.hasAccepted(geometry)); assert.equal(input.depth[0], geometry.routes[2]);
});

test("contact input rejects foreign/copied geometry and forged views without accepting overrides", async t => {
  const { plan, graph, geometry, input } = curves(), foreign = new GeometryPlan(graph, geometryLimits);
  assert.throws(() => foreign.contactInput(geometry, ["r0", "r1", "r2"], limits), { reason: "input" });
  assert.throws(() => plan.contactInput(structuredClone(geometry), ["r0", "r1", "r2"], limits), { reason: "input" });
  assert.throws(() => new GeometryContactInput(Symbol(), geometry, [], [], limits), { reason: "input" });
  const fake = Object.create(GeometryContactInput.prototype); fake.workerInput = () => input.workerInput();
  assert.deepEqual(await executor(t).run(fake), { kind: "failed", reason: "analysis" });
  assert.equal(Reflect.set(input, "geometry", {}), false); assert.equal(input.geometry, geometry);
});

test("contact limits snapshot declared getters once, ignore extras and retain their first values", () => {
  const { plan, geometry } = curves(), supplied = {}, reads = {};
  for (const [key, value] of Object.entries(limits)) Object.defineProperty(supplied, key, { get() { assert(!reads[key]); reads[key] = 1; return value; } });
  Object.defineProperty(supplied, "extra", { enumerable: true, get() { assert.fail("extra getter"); } });
  const input = plan.contactInput(geometry, ["r0", "r1", "r2"], supplied); assert.deepEqual(input.limits, limits);
  const mutable = { ...limits }, retained = plan.contactInput(geometry, ["r0", "r1", "r2"], mutable); mutable.contacts = 0;
  assert.equal(retained.limits.contacts, limits.contacts);
  for (const key of Object.keys(limits)) {
    let count = 0; const bad = { ...limits }; Object.defineProperty(bad, key, { get() { return ++count === 1 ? 0 : limits[key]; } });
    assert.throws(() => plan.contactInput(geometry, ["r0", "r1", "r2"], bad), { reason: "bounds" }); assert.equal(count, 1);
  }
});

test("depth is a complete snapshotted permutation with no inferred default order", () => {
  const { plan, geometry } = curves();
  for (const order of [[], ["r0", "r1"], ["r0", "r1", "r1"], ["r0", "r1", "foreign"], ["r0", "r1", "r2", "r2"]]) {
    assert.throws(() => plan.contactInput(geometry, order, limits), { reason: "input" });
  }
  const order = ["r2", "r0", "r1"], input = plan.contactInput(geometry, order, limits); order.reverse();
  assert.deepEqual(input.depth.map(r => r.id), ["r2", "r0", "r1"]);
  const iterable = ["r0", "r1", "r2"];
  iterable[Symbol.iterator] = () => { assert.fail("Caller iterator must not run"); };
  assert.equal(plan.contactInput(geometry, iterable, limits).depth.length, 3);
  const shortened = ["r0", "r1", "r2"];
  Object.defineProperty(shortened, 0, { get() { shortened.length = 1; return "r0"; } });
  assert.throws(() => plan.contactInput(geometry, shortened, limits), { reason: "input" });
});

test("segment and pair ceilings reject before segment-copy work without weakening geometry", () => {
  const { plan, geometry } = curves();
  for (const budget of [{ ...limits, segments: 2 }, { ...limits, pairs: 5 }]) {
    assert.throws(() => plan.contactInput(geometry, ["r0", "r1", "r2"], budget), { reason: "bounds" });
  }
  const input = plan.contactInput(geometry, ["r0", "r1", "r2"], { ...limits, segments: 3, pairs: 6 });
  assert.equal(input.segments.length, 3); assert(plan.hasAccepted(geometry));
  const original = input.segments.map(s => s.rootControls);
  plan.translate = () => ({ x: 0, y: 0 }); plan.scalar = () => 0;
  assert.deepEqual(plan.contactInput(geometry, ["r0", "r1", "r2"], limits).segments.map(s => s.rootControls), original);
});

test("worker response association rejects missing visits, foreign indices and substituted overlap endpoints", () => {
  const { input } = curves(), branch = { time: .5, parameter: .5, point: { x: 1, y: 1 }, reportedPoint: { x: 1, y: 1 } };
  const row = { firstIndex: 0, secondIndex: 1, first: branch, second: branch };
  assert.throws(() => input.accept({ visits: 5, contacts: [], overlaps: [] }), { reason: "output" });
  assert.throws(() => input.accept({ visits: 6, contacts: [{ ...row, secondIndex: 3 }], overlaps: [] }), { reason: "output" });
  assert.throws(() => input.accept({ visits: 6, contacts: [], overlaps: [{ start: row, end: { ...row, secondIndex: 2 } }] }), { reason: "output" });
  assert.throws(() => input.accept({ visits: 6, contacts: new Array(1), overlaps: [] }), { reason: "output" });
  assert.throws(() => input.accept({ visits: 6, contacts: [], overlaps: new Array(1) }), { reason: "output" });
  const equal = input.accept({ visits: 6, contacts: [{ ...row, secondIndex: 0 }], overlaps: [] });
  assert.equal(equal.contacts[0].above, "indistinguishable"); assert.equal(equal.coverage, "not-established");
});

test("actual contact-output exhaustion is atomic and leaves original geometry and depth usable", async t => {
  const { plan, geometry } = curves(), before = JSON.stringify(geometry);
  const input = plan.contactInput(geometry, ["r0", "r1", "r2"], { ...limits, contacts: 1 });
  assert.deepEqual(await executor(t).run(input), { kind: "failed", reason: "bounds" });
  assert.equal(JSON.stringify(geometry), before); assert(plan.hasAccepted(geometry)); assert.equal(input.depth.length, 3);
});

test("real contact worker preserves lazy refusal order and releases without changing geometry", async () => {
  const { input, plan, geometry } = curves(), before = JSON.stringify(geometry), Original = threads.Worker;
  const invalid = { kind: "line", controls: [{ x: null, y: 0 }, { x: 1, y: 1 }] };
  const requests = [
    ["bounds", request => { request.segments.push(invalid); request.limits.contacts = 1; }],
    ["numeric", request => { request.segments.unshift(invalid); }],
    ["bounds", request => { request.segments.unshift(invalid); request.limits.pairs = 1; }],
  ];
  for (const [reason, alter] of requests) {
    // Change only the serialized request crossing the existing worker boundary.
    // The actual worker, Paper loader, numerical checks and executor still run.
    const interception = mock.method(threads, "Worker", class extends Original {
      constructor(entry, options) {
        const request = JSON.parse(options.workerData.input); alter(request);
        super(entry, { ...options, workerData: { ...options.workerData, input: JSON.stringify(request) } });
      }
    });
    const worker = new ContactExecutor(workerLimits);
    try {
      assert.deepEqual(await worker.run(input), { kind: "failed", reason });
      assert.equal(worker.status, "idle"); assert.equal(JSON.stringify(geometry), before);
      assert.equal(input.geometry, geometry); assert.equal(input.depth.length, 3); assert(plan.hasAccepted(geometry));
    } finally { await worker.close(); interception.mock.restore(); }
  }
  const worker = new ContactExecutor(workerLimits);
  try {
    const result = await worker.run(input); assert.equal(result.kind, "completed");
    assert.equal(result.analysis.input, input); assert.equal(result.analysis.geometry, geometry);
    assert.equal(result.analysis.visitedPairs, 6); assert.equal(result.analysis.overlaps.length, 1);
  } finally { await worker.close(); }
});

test("contact request/result byte failures retain the accepted geometry", async t => {
  const { input, plan, geometry } = curves();
  assert.deepEqual(await executor(t, { ...workerLimits, requestBytes: 1 }).run(input), { kind: "failed", reason: "request-bytes" });
  assert.deepEqual(await executor(t, { ...workerLimits, resultBytes: 1 }).run(input), { kind: "failed", reason: "result-bytes" });
  assert(plan.hasAccepted(geometry));
});

test("contact busy, abort, close and later reuse wait for actual worker release", async t => {
  const { input } = curves(), worker = executor(t), abort = new AbortController();
  const pending = worker.run(input, abort.signal);
  assert.deepEqual(await worker.run(input), { kind: "unavailable", reason: "busy" }); abort.abort();
  assert.deepEqual(await pending, { kind: "stopped", reason: "cancelled" }); assert.equal(worker.status, "idle");
  assert.equal((await worker.run(input)).kind, "completed");
  const final = worker.run(input); await worker.close();
  assert.deepEqual(await final, { kind: "stopped", reason: "shutdown" });
  assert.deepEqual(await worker.run(input), { kind: "unavailable", reason: "closed" });
});

test("contact deadline returns only after worker release without revoking geometry", async t => {
  const { input, plan, geometry } = curves(), worker = executor(t, { ...workerLimits, deadlineMs: 1 });
  assert.deepEqual(await worker.run(input), { kind: "stopped", reason: "deadline" });
  assert.equal(worker.status, "idle"); assert(plan.hasAccepted(geometry));
});

test("contact supersession invalidates completion waiting for termination acknowledgement", async () => {
  const { input } = curves(), Original = threads.Worker; let release, reached;
  const gate = new Promise(resolve => { release = resolve; }), requested = new Promise(resolve => { reached = resolve; });
  const interception = mock.method(threads, "Worker", class extends Original {
    constructor(...args) { super(...args); const terminate = this.terminate.bind(this);
      this.terminate = async () => { reached(); const code = await terminate(); await gate; return code; }; }
  });
  const worker = new ContactExecutor(workerLimits);
  try {
    const pending = worker.run(input); await requested;
    const stopped = worker.supersede(); assert.equal(worker.status, "stopping"); release(); await stopped;
    assert.deepEqual(await pending, { kind: "stopped", reason: "superseded" }); assert.equal(worker.status, "idle");
  } finally { release(); await worker.close(); interception.mock.restore(); }
});

test("malformed contact output fails without mutating accepted geometry", async () => {
  const { input, plan, geometry } = curves(), Original = threads.Worker;
  const interception = mock.method(threads, "Worker", class extends Original {
    constructor(_path, options) { super(join(__dirname, "fixtures/layout-worker.cjs"), { ...options, workerData: { ...options.workerData, fixture: "malformed" } }); }
  });
  const worker = new ContactExecutor(workerLimits);
  try { assert.deepEqual(await worker.run(input), { kind: "failed", reason: "analysis" }); assert(plan.hasAccepted(geometry)); }
  finally { await worker.close(); interception.mock.restore(); }
});

test("known silent cusp omission remains an estimated empty result, never absence proof", async t => {
  const { input, geometry } = curves([[[0, 0], [1, 1], [0, 1], [1, 0]]]);
  const result = await executor(t).run(input); assert.equal(result.kind, "completed");
  assert.equal(result.analysis.contacts.length, 0); assert.equal(result.analysis.coverage, "not-established");
  assert.equal(result.analysis.certifiedErrorBound, null); assert.equal(result.analysis.geometry, geometry);
});

test("contact loader checks actual synchronous hook source before any substituted body executes", async t => {
  const temporary = mkdtempSync(join(tmpdir(), "pale-contact-source-")), marker = join(temporary, "executed");
  t.after(() => rmSync(temporary, { recursive: true, force: true }));
  const { input } = curves(), Original = threads.Worker;
  for (const format of [undefined, "module"]) {
    const interception = mock.method(threads, "Worker", class extends Original {
      constructor(entry, options) { super(join(__dirname, "fixtures/contact-source-worker.cjs"),
        { ...options, workerData: { ...options.workerData, entry, marker, format } }); }
    });
    const worker = new ContactExecutor(workerLimits);
    try { assert.deepEqual(await worker.run(input), { kind: "failed", reason: "load" }); assert.equal(existsSync(marker), false); }
    finally { await worker.close(); interception.mock.restore(); }
  }
});
