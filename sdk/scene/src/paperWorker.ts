import { Buffer } from "node:buffer";
import { createHash } from "node:crypto";
import { readFileSync, realpathSync } from "node:fs";
import { createRequire } from "node:module";
import * as modules from "node:module";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { parentPort, workerData } from "node:worker_threads";
import type paper = require("paper");
import { ContactError, contactLimits, pairCount, paperConfiguration,
  type BranchValue, type ContactRequest, type ContactResponse, type ContactValue } from "./contactAnalysis";
import type { Point } from "./geometry";

// Exact published core closure; no PaperScript, DOM or canvas implementation.
const coreFiles = Object.freeze({
  "paper-core.js": "9a914deb3bdb8cd07ee7ec199bfae9a2e9fbc1c93b185c4dbe83e99696e9470b",
  "node/self.js": "4be7c43d51d5c16379bc2e42c3f5c5e8b014af0554819a738e2338b81c4000da",
  "node/extend.js": "b09e2bfd406ee9c3085e2ece543eb5b9b484a5829a3ccc1cddc7c5f7f7e49f9e",
});
function check(value: unknown, reason: "numeric" | "bounds" | "load"): asserts value {
  if (!value) throw new ContactError(reason);
}
function finite(point: Point): boolean { return Number.isFinite(point.x) && Number.isFinite(point.y); }
type Core = typeof paper;
type Curve = InstanceType<Core["Curve"]>;
type Location = InstanceType<Core["CurveLocation"]>;
type Segment = ContactRequest["segments"][number];

/** Public Curve API only; parameter and residual estimates are not error bounds. */
function construct(core: Core, segment: Segment, limit: number): Curve {
  const p = segment.controls, line = segment.kind === "line", end = p[p.length - 1]!;
  check(p.length === (line ? 2 : 4) && p.every(finite), "numeric");
  const h1 = line ? { x: 0, y: 0 } : { x: p[1]!.x - p[0]!.x, y: p[1]!.y - p[0]!.y };
  const h2 = line ? { x: 0, y: 0 } : { x: p[2]!.x - end.x, y: p[2]!.y - end.y };
  check(finite(h1) && finite(h2), "numeric");
  const point = (v: Point): InstanceType<Core["Point"]> => new core.Point(v.x, v.y);
  const curve = new core.Curve(point(p[0]!), point(h1), point(h2), point(end));
  const expected = (line ? [p[0]!, p[0]!, end, end] : p).flatMap(v => [v.x, v.y]);
  const actual = curve.values;
  check(actual.length === 8 && actual.every(Number.isFinite), "numeric");
  check(actual.every((v, index) => Math.abs(v - expected[index]!) <= limit), "numeric");
  return curve;
}
function branch(location: Location, curve: Curve, segment: Segment): BranchValue {
  check(location.curve === curve, "numeric");
  const time = location.time, reported = location.point;
  check(Number.isFinite(time) && time >= 0 && time <= 1 && finite(reported), "numeric");
  const evaluated = curve.getPointAtTime(time); check(evaluated && finite(evaluated), "numeric");
  let parameter = time;
  if (segment.kind === "line") {
    const [a, b] = segment.controls, dx = b!.x - a!.x, dy = b!.y - a!.y;
    check(Number.isFinite(dx) && Number.isFinite(dy) && (dx !== 0 || dy !== 0), "numeric");
    parameter = Math.abs(dx) >= Math.abs(dy) ? (evaluated.x - a!.x) / dx : (evaluated.y - a!.y) / dy;
  }
  check(Number.isFinite(parameter) && parameter >= 0 && parameter <= 1, "numeric");
  return { time, parameter, point: { x: evaluated.x, y: evaluated.y }, reportedPoint: { x: reported.x, y: reported.y } };
}
function analyze(core: Core, input: ContactRequest): ContactResponse {
  const limits = contactLimits(input.limits), segments = input.segments;
  check(Array.isArray(segments) && segments.length <= limits.segments, "bounds");
  const visits = pairCount(segments.length, limits.pairs);
  // One curve per encountered segment, scoped to this analysis.
  // Stay lazy so later invalid controls cannot outrun an earlier pair refusal.
  const curves: (Curve | undefined)[] = new Array(segments.length);
  const curve = (index: number): Curve => curves[index] ??= construct(core, segments[index]!, limits.maximumControlDrift);
  const contacts: ContactValue[] = [], overlaps: { start: ContactValue; end: ContactValue }[] = [];
  let count = 0;
  for (let i = 0; i < segments.length; i++) for (let j = i; j < segments.length; j++) {
    const a = curve(i);
    const b = i === j ? a : curve(j);
    const locations = a.getIntersections(b);
    // The library constructs its result first; this ceiling bounds retained rows,
    // not that internal allocation. Worker heap/deadline limits remain independent.
    check(Array.isArray(locations) && locations.length <= limits.contacts - count, "bounds");
    count += locations.length;
    const ends: ContactValue[] = [];
    for (const location of locations) {
      const other = location.intersection;
      check(other && other.intersection === location, "numeric");
      const first = branch(location, a, segments[i]!), second = branch(other, b, segments[j]!);
      const separation = Math.hypot(first.point.x - second.point.x, first.point.y - second.point.y);
      check(Number.isFinite(separation) && separation <= limits.maximumSeparation, "numeric");
      check(location.hasOverlap() === other.hasOverlap(), "numeric");
      const contact = { firstIndex: i, secondIndex: j, first, second };
      if (location.hasOverlap()) ends.push(contact); else contacts.push(contact);
    }
    check(ends.length === 0 || ends.length === 2, "numeric");
    if (ends.length === 2) {
      ends.sort((a, b) => a.first.parameter - b.first.parameter);
      overlaps.push({ start: ends[0]!, end: ends[1]! });
    }
  }
  return { visits, contacts, overlaps };
}

function execute(): void {
  if (!parentPort) throw new Error("Contact analysis requires a worker thread");
  const request: unknown = workerData;
  if (!request || typeof request !== "object" || !("input" in request) || typeof request.input !== "string"
    || !("resultBytes" in request) || typeof request.resultBytes !== "number"
    || !Number.isSafeInteger(request.resultBytes) || request.resultBytes < 1) throw new Error("Invalid contact message");
  const require = createRequire(__filename);
  const root = dirname(require.resolve("paper/package.json"));
  check(JSON.parse(readFileSync(join(root, "package.json"), "utf8")).version === paperConfiguration.version, "load");
  const allowed = new Map(Object.entries(coreFiles).map(([file, hash]) => [realpathSync(join(root, "dist", file)), hash]));
  // Node 24's synchronous hooks are not declared by the package's pinned Node 20
  // type package. Check the actual runtime API instead of expanding that dependency.
  const register: unknown = Reflect.get(modules, "registerHooks"); check(typeof register === "function", "load");
  let blocked = false;
  const loaded = (valid: unknown): void => { if (!valid) { blocked = true; throw new ContactError("load"); } };
  const guard = (url: string): void => {
    if (url === "node:fs" || url === "node:path") return;
    if (url.startsWith("file:") && allowed.has(fileURLToPath(url))) return;
    blocked = true; throw new ContactError("load");
  };
  const hook: unknown = Reflect.apply(register, modules, [{
    resolve(specifier: string, context: unknown, next: (specifier: string, context: unknown) => { url: string }) {
      // A genuinely missing optional jsdom keeps upstream's ordinary fallback.
      // A resolved ambient module is rejected before load, even if Paper catches it.
      const result = next(specifier, context); guard(result.url); return result;
    },
    load(url: string, context: unknown, next: (url: string, context: unknown) => unknown) {
      guard(url);
      const result = next(url, context);
      if (!url.startsWith("file:")) return result;
      if (!result || typeof result !== "object") { loaded(false); throw new ContactError("load"); }
      loaded("format" in result && (result.format === "commonjs" || result.format === undefined));
      const raw = "source" in result ? result.source : null;
      let source: Buffer;
      if (typeof raw === "string") source = Buffer.from(raw);
      else if (ArrayBuffer.isView(raw)) source = Buffer.from(raw.buffer, raw.byteOffset, raw.byteLength);
      else if (raw instanceof ArrayBuffer) source = Buffer.from(raw);
      else { loaded(false); throw new ContactError("load"); }
      loaded(createHash("sha256").update(source).digest("hex") === allowed.get(fileURLToPath(url)));
      // Synchronous Node 24 load hooks expose the actual CommonJS source.
      // Return that same checked load result; no replacement loader or file re-read.
      return result;
    },
  }]);
  check(hook && typeof hook === "object" && "deregister" in hook && typeof hook.deregister === "function", "load");
  try {
    const core = require(join(root, "dist/paper-core.js")) as Core;
    check(!blocked && core.version === paperConfiguration.version && !core.PaperScript
      && Reflect.get(core, "document") === undefined, "load");
    const result = analyze(core, JSON.parse(request.input) as ContactRequest);
    check(!blocked, "load");
    const encoded = JSON.stringify(result);
    parentPort.postMessage(Buffer.byteLength(encoded, "utf8") > request.resultBytes ? "result-bytes" : encoded);
  } finally { Reflect.apply(hook.deregister, hook, []); }
}
try { execute(); }
catch (error) {
  if (error instanceof ContactError && ["load", "numeric", "bounds"].includes(error.reason)) parentPort?.postMessage(error.reason);
  else throw new Error("Contact worker failed");
}
