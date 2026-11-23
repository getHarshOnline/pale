import type { GeometryContactInput, GeometryResult, PlacedRoute, Point } from "./geometry";

/** @internal Pinned numerical backend identity retained in accepted analysis. */
export const paperConfiguration = Object.freeze({ engine: "paper-core" as const, version: "0.12.18" as const });
/**
 * Contact-query ceilings and acceptance thresholds, not certified approximation errors.
 * Counts are positive safe integers; thresholds are finite and positive.
 * These bounds do not cap the backend's temporary intersection allocation.
 */
export interface ContactLimits {
  /** Total directed line/cubic segments across the accepted routes. */
  readonly segments: number;
  /** Each distinct segment pair and each segment paired with itself is visited once. */
  readonly pairs: number;
  /** A contact costs one; an overlap interval costs two endpoints. */
  readonly contacts: number;
  /** Maximum Euclidean distance between evaluated contact branches, in root-frame layout units. */
  readonly maximumSeparation: number;
  /** Maximum per-coordinate drift when Paper reconstructs controls, in layout units. */
  readonly maximumControlDrift: number;
}
/** Original accepted route plus its segment occurrence; root controls are floating translations. */
export interface ContactSegment {
  readonly route: PlacedRoute;
  /** Zero-based directed segment position in the original route, not a point index. */
  readonly index: number;
  readonly rootControls: readonly Point[];
}
/** One evaluated branch of an estimated contact in the scene root frame. */
export interface ContactBranch {
  readonly segment: ContactSegment;
  /** Paper's unit-interval curve parameter, not elapsed time or arc length. */
  readonly backendTime: number;
  /** Unit-interval position within this segment: line fraction or cubic parameter, not whole-route distance. */
  readonly parameter: number;
  readonly parameterKind: "line-fraction-estimate" | "cubic-time-estimate";
  /** Evaluated and backend-reported positions in the scene root frame. */
  readonly point: Point;
  readonly reportedPoint: Point;
}
/** Later explicit route, then later directed segment/estimated parameter, is above. */
export type ContactDepth = "first" | "second" | "indistinguishable";
/** One estimated point contact between two directed route segments. */
export interface EstimatedContact {
  readonly first: ContactBranch;
  readonly second: ContactBranch;
  /** Evaluated branch distance in root-frame layout units; a residual, not an error bound. */
  readonly separation: number;
  readonly above: ContactDepth;
}
/** One estimated overlap interval between two directed route segments. */
export interface EstimatedOverlap {
  /** Endpoints ordered by the first branch; the second branch may run backward. */
  readonly start: EstimatedContact;
  readonly end: EstimatedContact;
}
/**
 * Estimated presentation annotations, never a replacement scene or proof of no contacts.
 * Completed analysis can miss contacts; coverage and a certified error bound remain
 * unavailable. No global parameter ordering of contacts is promised.
 */
export interface ContactAnalysis {
  readonly input: GeometryContactInput;
  readonly geometry: GeometryResult;
  /** Numerical backend used for these estimates. */
  readonly model: Readonly<{ readonly engine: "paper-core"; readonly version: "0.12.18" }>;
  readonly coverage: "not-established";
  readonly certifiedErrorBound: null;
  readonly visitedPairs: number;
  readonly contacts: readonly EstimatedContact[];
  readonly overlaps: readonly EstimatedOverlap[];
}
/**
 * Input: wrong geometry/order; bounds: query ceiling; output: malformed transport;
 * numeric: failed numerical checks; load: the pinned no-DOM backend could not load.
 * Executor results have their own failure envelope and preserve accepted geometry.
 */
export type ContactRejection = "input" | "bounds" | "output" | "numeric" | "load";
/** Route-contact input, limit, transport, numerical or backend-load failure. */
export class ContactError extends Error {
  constructor(readonly reason: ContactRejection) {
    super(`Route contact analysis rejected: ${reason}`);
    this.name = "ContactError";
  }
}

/** @internal Private worker request. Original route witnesses stay on the main thread. */
export interface ContactRequest {
  readonly segments: readonly { readonly kind: PlacedRoute["kind"]; readonly controls: readonly Point[] }[];
  readonly limits: ContactLimits;
}
/** @internal One worker-reported branch value. */
export interface BranchValue {
  readonly time: number;
  readonly parameter: number;
  readonly point: Point;
  readonly reportedPoint: Point;
}
/** @internal One worker-reported contact. */
export interface ContactValue {
  readonly firstIndex: number;
  readonly secondIndex: number;
  readonly first: BranchValue;
  readonly second: BranchValue;
}
/** @internal Complete worker response before main-side custody checks. */
export interface ContactResponse {
  readonly visits: number;
  readonly contacts: readonly ContactValue[];
  readonly overlaps: readonly { readonly start: ContactValue; readonly end: ContactValue }[];
}
/** @internal Capture and validate contact limits. */
export function contactLimits(supplied: ContactLimits): ContactLimits {
  const limits = { segments: supplied.segments, pairs: supplied.pairs, contacts: supplied.contacts,
    maximumSeparation: supplied.maximumSeparation, maximumControlDrift: supplied.maximumControlDrift };
  for (const count of [limits.segments, limits.pairs, limits.contacts]) {
    if (!Number.isSafeInteger(count) || count < 1) throw new ContactError("bounds");
  }
  for (const value of [limits.maximumSeparation, limits.maximumControlDrift]) {
    if (!Number.isFinite(value) || value <= 0) throw new ContactError("bounds");
  }
  return Object.freeze(limits);
}
/** @internal Compute the bounded segment-pair count. */
export function pairCount(segments: number, maximum: number): number {
  if (segments > Math.floor(Number.MAX_SAFE_INTEGER / (segments + 1))) throw new ContactError("bounds");
  const count = segments * (segments + 1) / 2;
  if (count > maximum) throw new ContactError("bounds");
  return count;
}
type ObjectValue = Record<string, unknown>;
function object(value: unknown): ObjectValue {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new ContactError("output");
  return value as ObjectValue;
}
function unit(value: unknown): number {
  if (typeof value !== "number" || !Number.isFinite(value) || value < 0 || value > 1) throw new ContactError("output");
  return value;
}
function point(value: unknown): Point {
  const p = object(value);
  if (typeof p.x !== "number" || typeof p.y !== "number" || !Number.isFinite(p.x) || !Number.isFinite(p.y)) {
    throw new ContactError("output");
  }
  return Object.freeze({ x: p.x, y: p.y });
}
/** @internal Validate transport association; this is not a second intersection solver. */
export function acceptContacts(input: GeometryContactInput, geometry: GeometryResult,
  segments: readonly ContactSegment[], depth: readonly PlacedRoute[], limits: ContactLimits,
  value: unknown): ContactAnalysis {
  const result = object(value), contactValues = result.contacts, overlapValues = result.overlaps;
  if (result.visits !== pairCount(segments.length, limits.pairs) || !Array.isArray(contactValues)
    || !Array.isArray(overlapValues)) throw new ContactError("output");
  const contactCount = contactValues.length, overlapCount = overlapValues.length;
  if (contactCount > limits.contacts
    || overlapCount > Math.floor((limits.contacts - contactCount) / 2)) throw new ContactError("output");
  const rank = new Map(depth.map((route, index) => [route, index]));
  const branch = (value: unknown, segment: ContactSegment): ContactBranch => {
    const b = object(value);
    return Object.freeze({ segment, backendTime: unit(b.time), parameter: unit(b.parameter),
      parameterKind: segment.route.kind === "line" ? "line-fraction-estimate" : "cubic-time-estimate",
      point: point(b.point), reportedPoint: point(b.reportedPoint) });
  };
  const contact = (value: unknown): EstimatedContact => {
    const c = object(value), a = c.firstIndex, b = c.secondIndex;
    if (typeof a !== "number" || typeof b !== "number" || !Number.isSafeInteger(a) || !Number.isSafeInteger(b)
      || a < 0 || b < a || b >= segments.length) throw new ContactError("output");
    const first = branch(c.first, segments[a]!), second = branch(c.second, segments[b]!);
    const separation = Math.hypot(first.point.x - second.point.x, first.point.y - second.point.y);
    if (!Number.isFinite(separation) || separation > limits.maximumSeparation) throw new ContactError("numeric");
    const delta = rank.get(first.segment.route)! - rank.get(second.segment.route)!
      || first.segment.index - second.segment.index || first.parameter - second.parameter;
    return Object.freeze({ first, second, separation,
      above: delta === 0 ? "indistinguishable" : delta > 0 ? "first" : "second" });
  };
  const contacts: EstimatedContact[] = [], overlaps: EstimatedOverlap[] = [];
  for (let index = 0; index < contactCount; index++) contacts.push(contact(contactValues[index]));
  for (let index = 0; index < overlapCount; index++) {
    const row = object(overlapValues[index]), start = contact(row.start), end = contact(row.end);
    if (start.first.segment !== end.first.segment || start.second.segment !== end.second.segment
      || start.first.parameter > end.first.parameter) throw new ContactError("output");
    overlaps.push(Object.freeze({ start, end }));
  }
  Object.freeze(contacts); Object.freeze(overlaps);
  return Object.freeze({ input, geometry, model: paperConfiguration, coverage: "not-established", certifiedErrorBound: null,
    visitedPairs: result.visits as number, contacts, overlaps });
}
