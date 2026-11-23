const assert = require("node:assert/strict");
const { test, mock } = require("node:test");
const { join } = require("node:path");
const threads = require("node:worker_threads");
const { performance } = require("node:perf_hooks");
const { GeometryPlan, ElkExecutor } = require("@rtepl/pale-scene");
const limits = { requestBytes: 1e6, resultBytes: 1e6, deadlineMs: 10000,
  totalRequestBytes: 10e6, totalResultBytes: 10e6, totalActiveMs: 60000,
  oldHeapMb: 128, youngHeapMb: 32 };
const graphLimits = { nodes: 300, ports: 600, relations: 600, points: 50000, depth: 8, coordinateMagnitude: 1e6 };
const plan = (count = 2) => new GeometryPlan({ nodes: Array.from({ length: count }, (_, index) => ({
  id: `n${index}`, width: 40, height: 30, ports: [{ id: `p${index}`, side: "EAST" }, { id: `q${index}`, side: "WEST" }],
})), relations: Array.from({ length: count - 1 }, (_, index) => ({ id: `r${index}`, sourcePort: `p${index}`, targetPort: `q${index + 1}` })) }, graphLimits);
function fixture(mode) {
  const Original = threads.Worker;
  return mock.method(threads, "Worker", class extends Original {
    constructor(_path, options) { super(join(__dirname, "fixtures/layout-worker.cjs"), { ...options, workerData: { ...options.workerData, fixture: mode } }); }
  });
}

test("real pinned ELK runs off-thread and returns only accepted geometry", async () => {
  const executor = new ElkExecutor(limits);
  try {
    const result = await executor.run(plan());
    assert.equal(result.kind, "completed");
    assert.deepEqual(result.geometry.nodes.map(node => node.id), ["n0", "n1"]);
    assert.equal(result.geometry.routes[0].sourcePort, "p0");
    assert.equal(executor.status, "idle");
    const repeated = await executor.run(plan());
    assert.equal(repeated.kind, "completed");
    assert.deepEqual(repeated.geometry, result.geometry);
  } finally { await executor.close(); }
});

test("abort before starting and input byte rejection create no worker", async () => {
  const interception = mock.method(threads, "Worker", () => { throw new Error("Must not construct"); });
  try {
    const executor = new ElkExecutor(limits), abort = new AbortController(); abort.abort();
    assert.deepEqual(await executor.run(plan(), abort.signal), { kind: "stopped", reason: "cancelled" });
    const duringPreparation = new AbortController(), prepared = plan(), encode = prepared.elkInput.bind(prepared);
    prepared.elkInput = () => { duringPreparation.abort(); return encode(); };
    assert.deepEqual(await executor.run(prepared, duringPreparation.signal), { kind: "stopped", reason: "cancelled" });
    assert(executor.charges.requestBytes > 0);
    assert.deepEqual(await new ElkExecutor({ ...limits, requestBytes: 1 }).run(plan()), { kind: "failed", reason: "request-bytes" });
    assert.equal(interception.mock.callCount(), 0);
    await executor.close();
  } finally { interception.mock.restore(); }
});

test("an unencodable public geometry request fails before worker construction", async () => {
  const interception = mock.method(threads, "Worker", () => { throw new Error("Must not construct"); });
  const executor = new ElkExecutor(limits), input = plan();
  try {
    input.elkInput = () => undefined;
    assert.deepEqual(await executor.run(input), { kind: "failed", reason: "geometry" });
    assert.equal(interception.mock.callCount(), 0);
    assert.equal(executor.status, "idle");
    assert.equal(executor.charges.requestBytes, 0);
  } finally { await executor.close(); interception.mock.restore(); }
});

test("synchronous preparation owns lifecycle custody before a worker exists", async () => {
  const Original = threads.Worker;
  const interception = mock.method(threads, "Worker", class extends Original {});
  const executor = new ElkExecutor(limits), input = plan(), encode = input.elkInput.bind(input);
  let nested, status;
  input.elkInput = () => {
    status = executor.status;
    nested = executor.run(plan());
    return encode();
  };
  try {
    assert.equal((await executor.run(input)).kind, "completed");
    assert.equal(status, "running");
    assert.deepEqual(await nested, { kind: "unavailable", reason: "busy" });
    assert.equal(interception.mock.callCount(), 1);
  } finally { await executor.close(); interception.mock.restore(); }
});

test("supersede and close during synchronous preparation prevent worker construction", async () => {
  const interception = mock.method(threads, "Worker", () => { throw new Error("Must not construct"); });
  try {
    const superseded = new ElkExecutor(limits), supersededInput = plan(), encodeSuperseded = supersededInput.elkInput.bind(supersededInput);
    let supersession;
    supersededInput.elkInput = () => { supersession = superseded.supersede(); return encodeSuperseded(); };
    assert.deepEqual(await superseded.run(supersededInput), { kind: "stopped", reason: "superseded" });
    await supersession;
    assert.equal(superseded.status, "idle");
    await superseded.close();

    const closed = new ElkExecutor(limits), closedInput = plan(), encodeClosed = closedInput.elkInput.bind(closedInput);
    let closure;
    closedInput.elkInput = () => { closure = closed.close(); return encodeClosed(); };
    assert.deepEqual(await closed.run(closedInput), { kind: "stopped", reason: "shutdown" });
    await closure;
    assert.equal(closed.status, "closed");
    assert.deepEqual(await closed.run(plan()), { kind: "unavailable", reason: "closed" });
    assert.equal(interception.mock.callCount(), 0);
  } finally { interception.mock.restore(); }
});

test("busy rejects without a queue; active abort terminates the actual worker", async () => {
  const Original = threads.Worker, abort = new AbortController();
  let exitCode, online;
  const entered = new Promise(resolve => { online = resolve; });
  const interception = mock.method(threads, "Worker", class extends Original {
    constructor(...args) { super(...args); this.on("online", online); this.on("exit", code => { exitCode = code; }); }
  });
  const executor = new ElkExecutor(limits);
  try {
    const pending = executor.run(plan(256), abort.signal);
    assert.deepEqual(await executor.run(plan()), { kind: "unavailable", reason: "busy" });
    await entered; abort.abort();
    assert.equal(executor.status, "stopping");
    assert.deepEqual(await pending, { kind: "stopped", reason: "cancelled" });
    assert.equal(typeof exitCode, "number"); assert.equal(executor.status, "idle");
  } finally { await executor.close(); interception.mock.restore(); }
});

test("deadline stops a real worker and a later call can run", async () => {
  const executor = new ElkExecutor({ ...limits, deadlineMs: 1 });
  assert.deepEqual(await executor.run(plan(256)), { kind: "stopped", reason: "deadline" });
  assert.equal(executor.status, "idle"); await executor.close();
});

test("actual worker enforces result bytes before sending geometry", async () => {
  const executor = new ElkExecutor({ ...limits, resultBytes: 16 });
  assert.deepEqual(await executor.run(plan()), { kind: "failed", reason: "result-bytes" });
  assert.equal(executor.status, "idle"); await executor.close();
});

test("engine errors, malformed geometry and oversized worker messages are distinct failures", async () => {
  for (const [mode, reason] of [["error", "worker"], ["malformed", "geometry"], ["oversized", "result-bytes"]]) {
    const interception = fixture(mode), executor = new ElkExecutor({ ...limits, resultBytes: 64 });
    try { assert.deepEqual(await executor.run(plan()), { kind: "failed", reason }); }
    finally { await executor.close(); interception.mock.restore(); }
  }
});

test("supersession invalidates a completed message until worker termination is acknowledged", async () => {
  const Original = threads.Worker;
  let release, stopping, terminated = false;
  const gate = new Promise(resolve => { release = resolve; });
  const requested = new Promise(resolve => { stopping = resolve; });
  const interception = mock.method(threads, "Worker", class extends Original {
    constructor(...args) {
      super(...args);
      const terminate = this.terminate.bind(this);
      this.terminate = async () => { stopping(); const exit = await terminate(); terminated = true; await gate; return exit; };
    }
  });
  const executor = new ElkExecutor(limits);
  try {
    let delivered = false;
    const pending = executor.run(plan()).then(result => { delivered = true; return result; });
    await requested;
    const superseded = executor.supersede();
    assert.equal(executor.status, "stopping"); assert.equal(delivered, false);
    assert.deepEqual(await executor.run(plan()), { kind: "unavailable", reason: "busy" });
    release(); await superseded;
    assert.equal(terminated, true);
    assert.deepEqual(await pending, { kind: "stopped", reason: "superseded" });
  } finally { release(); await executor.close(); interception.mock.restore(); }
});

test("late worker messages cannot replace a cancellation result", async () => {
  const Original = threads.Worker;
  const interception = mock.method(threads, "Worker", class extends Original {
    constructor(...args) {
      super(...args);
      const terminate = this.terminate.bind(this);
      this.terminate = async () => { this.emit("message", "{}"); return terminate(); };
    }
  });
  const executor = new ElkExecutor(limits), abort = new AbortController();
  try {
    const pending = executor.run(plan(), abort.signal); abort.abort();
    assert.deepEqual(await pending, { kind: "stopped", reason: "cancelled" });
  } finally { await executor.close(); interception.mock.restore(); }
});

test("shutdown awaits exit, rejects future work, and is repeatable", async () => {
  const interception = fixture("wait"), executor = new ElkExecutor(limits);
  try {
    const pending = executor.run(plan());
    await executor.close(); await executor.close();
    assert.deepEqual(await pending, { kind: "stopped", reason: "shutdown" });
    assert.equal(executor.status, "closed");
    assert.deepEqual(await executor.run(plan()), { kind: "unavailable", reason: "closed" });
  } finally { interception.mock.restore(); }
});

test("failed termination acknowledgement never claims idle or permits a new worker", async () => {
  const Original = threads.Worker;
  let exited = false;
  const interception = mock.method(threads, "Worker", class extends Original {
    constructor(...args) {
      super(...args);
      const terminate = this.terminate.bind(this);
      this.terminate = async () => { await terminate(); exited = true; throw new Error("Controlled acknowledgement failure"); };
    }
  });
  const executor = new ElkExecutor(limits);
  try {
    assert.deepEqual(await executor.run(plan()), { kind: "failed", reason: "termination" });
    assert.equal(exited, true); assert.equal(executor.status, "stopping");
    assert.deepEqual(await executor.run(plan()), { kind: "unavailable", reason: "busy" });
    await assert.rejects(executor.close(), /termination failed/);
  } finally { interception.mock.restore(); }
});

test("invalid limits never reach a worker", () => {
  for (const value of [0, -1, Infinity, NaN, 1.5]) assert.throws(() => new ElkExecutor({ ...limits, resultBytes: value }), RangeError);
  assert.throws(() => new ElkExecutor({ ...limits, deadlineMs: 2_147_483_648 }), RangeError);
  for (const key of Object.keys(limits)) {
    const missing = { ...limits }; delete missing[key];
    assert.throws(() => new ElkExecutor(missing), RangeError);
    assert.throws(() => new ElkExecutor({ ...limits, [key]: undefined }), RangeError);
  }
});

test("executor captures each declared limit once and ignores unrelated getters", async () => {
  const calls = {}, supplied = {};
  for (const [key, value] of Object.entries(limits)) {
    calls[key] = 0;
    Object.defineProperty(supplied, key, { enumerable: true, get() {
      calls[key]++;
      return calls[key] === 1 ? value : NaN;
    } });
  }
  Object.defineProperty(supplied, "unrelated", { enumerable: true, get() {
    throw new Error("An unrelated property is not an executor input");
  } });
  const executor = new ElkExecutor(supplied);
  try {
    assert.equal((await executor.run(plan())).kind, "completed");
    assert.deepEqual(calls, Object.fromEntries(Object.keys(limits).map(key => [key, 1])));
  } finally { await executor.close(); }
});

test("executor rejects an invalid first limit even if a later read would be valid", () => {
  for (const key of Object.keys(limits)) {
    for (const invalid of [0, -1, NaN, Infinity, 1.5,
      ...(key === "deadlineMs" ? [2_147_483_648] : [])]) {
      let calls = 0;
      const supplied = { ...limits };
      Object.defineProperty(supplied, key, { enumerable: true, get() {
        return ++calls === 1 ? invalid : limits[key];
      } });
      assert.throws(() => new ElkExecutor(supplied), RangeError);
      assert.equal(calls, 1);
    }
  }
});

test("later caller mutation neither loosens nor tightens captured executor limits", async () => {
  const suppliedSmall = { ...limits, requestBytes: 1 }, suppliedLarge = { ...limits };
  const small = new ElkExecutor(suppliedSmall), large = new ElkExecutor(suppliedLarge);
  Object.assign(suppliedSmall, limits);
  Object.assign(suppliedLarge, { requestBytes: 1, resultBytes: 1, deadlineMs: 1, oldHeapMb: 1, youngHeapMb: 1 });
  try {
    assert.deepEqual(await small.run(plan()), { kind: "failed", reason: "request-bytes" });
    assert.equal((await large.run(plan())).kind, "completed");
  } finally { await small.close(); await large.close(); }
});

test("cumulative request bytes stop another worker before construction", async () => {
  const input = plan(), inputBytes = Buffer.byteLength(JSON.stringify(input.elkInput()), "utf8");
  const Original = threads.Worker;
  const interception = mock.method(threads, "Worker", class extends Original {});
  const executor = new ElkExecutor({ ...limits, totalRequestBytes: inputBytes });
  try {
    assert.equal((await executor.run(input)).kind, "completed");
    assert.deepEqual(await executor.run(input), { kind: "failed", reason: "request-bytes" });
    assert.equal(interception.mock.callCount(), 1);
    assert.equal(executor.charges.requestBytes, inputBytes);
    assert.equal(Object.isFrozen(executor.charges), true);
  } finally { await executor.close(); interception.mock.restore(); }
});

test("cumulative result bytes cap the next worker without retaining output", async () => {
  const input = plan(), probe = new ElkExecutor(limits);
  let resultBytes;
  try {
    assert.equal((await probe.run(input)).kind, "completed");
    resultBytes = probe.charges.resultBytes;
    assert(resultBytes > 0);
  } finally { await probe.close(); }
  const Original = threads.Worker, admitted = [];
  const interception = mock.method(threads, "Worker", class extends Original {
    constructor(path, options) { admitted.push(options.workerData.resultBytes); super(path, options); }
  });
  const executor = new ElkExecutor({ ...limits, totalResultBytes: resultBytes });
  try {
    assert.equal((await executor.run(input)).kind, "completed");
    assert.deepEqual(await executor.run(input), { kind: "failed", reason: "result-bytes" });
    assert.deepEqual(admitted, [resultBytes]);
    assert.equal(executor.charges.resultBytes, resultBytes);
  } finally { await executor.close(); interception.mock.restore(); }
});

test("a rejected response still consumes the cumulative result-byte budget", async () => {
  const malformedBytes = Buffer.byteLength("{}", "utf8");
  const interception = fixture("malformed");
  const executor = new ElkExecutor({ ...limits, resultBytes: malformedBytes, totalResultBytes: malformedBytes });
  try {
    assert.deepEqual(await executor.run(plan()), { kind: "failed", reason: "geometry" });
    assert.equal(executor.charges.resultBytes, malformedBytes);
    assert.deepEqual(await executor.run(plan()), { kind: "failed", reason: "result-bytes" });
    assert.equal(interception.mock.callCount(), 1);
  } finally { await executor.close(); interception.mock.restore(); }
});

test("cumulative active time includes preparation and rejects later work", async () => {
  let elapsed = 0;
  const clock = mock.method(performance, "now", () => elapsed);
  const input = plan(), encode = input.elkInput.bind(input);
  const preparation = mock.method(input, "elkInput", () => { elapsed = 5; return encode(); });
  const worker = mock.method(threads, "Worker", () => { throw new Error("Must not construct"); });
  const executor = new ElkExecutor({ ...limits, deadlineMs: 100, totalActiveMs: 5 });
  try {
    assert.deepEqual(await executor.run(input), { kind: "stopped", reason: "deadline" });
    assert.deepEqual(executor.charges, { requestBytes: Buffer.byteLength(JSON.stringify(input.elkInput()), "utf8"),
      resultBytes: 0, activeMs: 5 });
    assert.deepEqual(await executor.run(input), { kind: "stopped", reason: "deadline" });
    assert.equal(worker.mock.callCount(), 0);
    assert.equal(preparation.mock.callCount(), 2);
  } finally { await executor.close(); worker.mock.restore(); preparation.mock.restore(); clock.mock.restore(); }
});

test("cumulative active time includes worker termination before admitting later work", async () => {
  let elapsed = 0;
  const clock = mock.method(performance, "now", () => elapsed);
  const Original = threads.Worker;
  const interception = mock.method(threads, "Worker", class extends Original {
    constructor(...args) {
      super(...args);
      const terminate = this.terminate.bind(this);
      this.terminate = async () => { const exit = await terminate(); elapsed = 1000; return exit; };
    }
  });
  const executor = new ElkExecutor({ ...limits, deadlineMs: 10000, totalActiveMs: 1000 });
  try {
    assert.equal((await executor.run(plan())).kind, "completed");
    assert.equal(executor.charges.activeMs, 1000);
    assert.deepEqual(await executor.run(plan()), { kind: "stopped", reason: "deadline" });
    assert.equal(interception.mock.callCount(), 1);
  } finally { await executor.close(); interception.mock.restore(); clock.mock.restore(); }
});

test("overdue request preparation stops before creating a worker", async () => {
  let elapsed = 0;
  const clock = mock.method(performance, "now", () => elapsed);
  const input = plan(), encode = input.elkInput.bind(input);
  const preparation = mock.method(input, "elkInput", () => { elapsed = limits.deadlineMs; return encode(); });
  const worker = mock.method(threads, "Worker", () => { throw new Error("Must not construct"); });
  const executor = new ElkExecutor(limits);
  try {
    assert.deepEqual(await executor.run(input), { kind: "stopped", reason: "deadline" });
    assert.equal(worker.mock.callCount(), 0);
    assert.equal(executor.status, "idle");
  } finally { await executor.close(); worker.mock.restore(); preparation.mock.restore(); clock.mock.restore(); }
});

test("overdue result checking cannot publish geometry before a delayed timer fires", async () => {
  let elapsed = 0;
  const clock = mock.method(performance, "now", () => elapsed);
  const input = plan(), accept = input.accept.bind(input);
  const checking = mock.method(input, "accept", value => { const result = accept(value); elapsed = limits.deadlineMs; return result; });
  const executor = new ElkExecutor(limits);
  try {
    assert.deepEqual(await executor.run(input), { kind: "stopped", reason: "deadline" });
    assert.equal(checking.mock.callCount(), 1);
    assert.equal(executor.status, "idle");
  } finally { await executor.close(); checking.mock.restore(); clock.mock.restore(); }
});
