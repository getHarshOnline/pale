import { sharedRouteCorridors } from "./routeBundles";
import { acceptContacts, contactLimits, pairCount, ContactError,
  type ContactAnalysis, type ContactLimits, type ContactRequest, type ContactSegment } from "./contactAnalysis";

/** Coordinates in scene layout units, not device pixels; the containing value names the frame. */
export interface Point { readonly x: number; readonly y: number }
/** Top-left position and nonnegative dimensions, in the same frame and layout units. */
export interface Rectangle extends Point { readonly width: number; readonly height: number }
/** A two-by-two-unit port constrained to the selected side. */
export interface PortInput {
  readonly id: string;
  readonly side: "NORTH" | "EAST" | "SOUTH" | "WEST";
}
/** One presentation node. Width and height use scene layout units. */
export interface NodeInput {
  readonly id: string;
  /** Parent precedes this node. Absence selects the scene root. */
  readonly parentId?: string;
  /** Minimum dimensions; layout may enlarge a node to fit its contents. */
  readonly width: number;
  readonly height: number;
  readonly ports: readonly PortInput[];
}
/** One directed route between two ports in the same geometry request. */
export interface RelationInput {
  readonly id: string;
  readonly sourcePort: string;
  readonly targetPort: string;
}
/** Explicit presentation selection, not inferred semantic grouping or a routing instruction. */
export interface BundleInput {
  readonly id: string;
  readonly end: "source" | "target";
  /** At least two distinct original routes with the same port at the selected end. */
  readonly routeIds: readonly string[];
}
/**
 * One scene's presentation occurrences, not semantic subjects.
 * Node, port, relation and bundle IDs share a unique namespace: 1–32 ASCII
 * letters, digits, underscores or hyphens. Relations name exact port IDs.
 */
export interface GeometryInput {
  readonly nodes: readonly NodeInput[];
  readonly relations: readonly RelationInput[];
  readonly bundles?: readonly BundleInput[];
}
/** Positive safe-integer cell/depth ceilings, not byte, memory or execution-time bounds. */
export interface GeometryLimits {
  readonly nodes: number;
  /** Total ports across all nodes. */
  readonly ports: number;
  readonly relations: number;
  /** Total route points, including endpoints and cubic controls, across the accepted scene. */
  readonly points: number;
  /** Maximum node nesting; the scene root is depth zero. */
  readonly depth: number;
  /** Finite absolute scalar ceiling, at least 2, for dimensions and local/root-translated coordinates. */
  readonly coordinateMagnitude: number;
}
/** One placed node whose bounds use its parent node's coordinate frame. */
export interface PlacedNode {
  readonly id: string;
  readonly parentId: string;
  /** Bounds relative to the parent node. */
  readonly bounds: Rectangle;
}
/** One placed port whose bounds use its owning node's coordinate frame. */
export interface PlacedPort {
  readonly id: string;
  readonly nodeId: string;
  /** Bounds relative to the node. */
  readonly bounds: Rectangle;
}
/** One placed route whose points use its declared container coordinate frame. */
export interface PlacedRoute extends RelationInput {
  /** Checked ancestor coordinate frame, not semantic containment or a minimal-container claim. */
  readonly containerId: string;
  /** A cubic chain has one start followed by control, control, end triples. */
  readonly kind: "line" | "cubic-chain";
  /** All points are relative to containerId, including the first and last. */
  readonly points: readonly Point[];
}
/** Frozen geometry accepted by a particular plan; copying its fields does not copy that association. */
export interface GeometryResult {
  /** Scene-root bounds at (0, 0). Parent/container ID "$scene" denotes this frame. */
  readonly root: Rectangle;
  readonly nodes: readonly PlacedNode[];
  readonly ports: readonly PlacedPort[];
  readonly routes: readonly PlacedRoute[];
  readonly bundles: readonly PlacedBundle[];
}

/** Exact fixed root options used by the adapter; per-node constraints remain on the plan. */
export const elkConfiguration = Object.freeze({
  engine: "elkjs" as const, version: "0.12.0" as const,
  rootOptions: Object.freeze({
    "elk.algorithm": "layered", "elk.direction": "RIGHT", "elk.hierarchyHandling": "INCLUDE_CHILDREN",
    "elk.edgeRouting": "SPLINES", "elk.layered.edgeRouting.splines.mode": "CONSERVATIVE",
    "elk.randomSeed": "1", "elk.json.shapeCoords": "PARENT", "elk.json.edgeCoords": "CONTAINER",
  }),
});
/** Pinned layout engine identity and fixed root options. */
export type GeometryAlgorithm = typeof elkConfiguration;

const executionPermit = Symbol("geometry execution input");
let issueExecution: (plan: GeometryPlan, input: () => ObjectValue,
  accept: (value: unknown) => GeometryResult) => GeometryExecutionInput;
/** Immutable plan-issued execution view; object-shaped copies cannot supply acceptance. */
export class GeometryExecutionInput {
  readonly #plan: GeometryPlan;
  readonly #input: () => ObjectValue;
  readonly #accept: (value: unknown) => GeometryResult;
  private constructor(token: symbol, plan: GeometryPlan, input: () => ObjectValue,
    accept: (value: unknown) => GeometryResult) {
    if (token !== executionPermit) throw new GeometryError("input");
    this.#plan = plan; this.#input = input; this.#accept = accept; Object.freeze(this);
  }
  static { issueExecution = (plan, input, accept) => new GeometryExecutionInput(executionPermit, plan, input, accept); }
  /** Pinned algorithm used by the issuing plan. */
  get algorithm(): GeometryAlgorithm { return elkConfiguration; }
  /** Test exact plan custody. Equal request values do not substitute for this association. */
  belongsTo(plan: GeometryPlan): boolean { return this.#plan === plan; }
  /** @internal Build the renderer-neutral ELK request owned by the issuing plan. */
  elkInput(): ObjectValue { return this.#input(); }
  /** @internal Validate untrusted ELK output and bind accepted geometry to the issuing plan. */
  accept(value: unknown): GeometryResult { return this.#accept(value); }
}

const contactPermit = Symbol("accepted geometry contact input");
let issueContact: (geometry: GeometryResult, segments: readonly ContactSegment[], depth: readonly PlacedRoute[],
  limits: ContactLimits) => GeometryContactInput;
/** Exact accepted geometry custody, independent of the lifetime of any worker. */
export class GeometryContactInput {
  readonly #geometry: GeometryResult;
  readonly #segments: readonly ContactSegment[];
  readonly #depth: readonly PlacedRoute[];
  readonly #limits: ContactLimits;
  private constructor(token: symbol, geometry: GeometryResult, segments: readonly ContactSegment[],
    depth: readonly PlacedRoute[], limits: ContactLimits) {
    if (token !== contactPermit) throw new ContactError("input");
    this.#geometry = geometry; this.#segments = segments; this.#depth = depth; this.#limits = limits; Object.freeze(this);
  }
  static { issueContact = (geometry, segments, depth, limits) => new GeometryContactInput(contactPermit, geometry, segments, depth, limits); }
  /** Exact geometry retained by this analysis request. */
  get geometry(): GeometryResult { return this.#geometry; }
  /** Root-frame route segments selected for analysis. */
  get segments(): readonly ContactSegment[] { return this.#segments; }
  /** Complete explicit route permutation, usable even if numerical analysis fails. */
  get depth(): readonly PlacedRoute[] { return this.#depth; }
  /** Captured numerical and count limits. */
  get limits(): ContactLimits { return this.#limits; }
  /** @internal Build the plain worker request without semantic or SDK-owned values. */
  workerInput(): ContactRequest {
    return { segments: this.#segments.map(segment => ({ kind: segment.route.kind, controls: segment.rootControls })), limits: this.#limits };
  }
  /** @internal
   * Check worker output against the retained segment and depth associations.
   * This validates transport, not numerical completeness.
   * @throws ContactError
   * For malformed output, invalid counts or excessive separation.
   */
  accept(value: unknown): ContactAnalysis {
    return acceptContacts(this, this.#geometry, this.#segments, this.#depth, this.#limits, value);
  }
}

/** Explicit visual-occurrence pairing. It does not establish semantic correspondence. */
export interface GeometryOccurrencePair {
  readonly priorId: string;
  readonly currentId: string;
}
/** Exact reuse or an explicit reason why a fresh layout is required. */
export type GeometryReuseResult =
  | { readonly kind: "reused"; readonly geometry: GeometryResult }
  | { readonly kind: "layout-required"; readonly reason: "constraints" | "limits" };

/** A presentation region that must be laid out again; never a semantic scope. */
export interface GeometryRegionEscalation {
  readonly kind: "region-escalation";
  readonly regionId: string;
  readonly reason: "constraints" | "limits" | "crossing" | "bundle" | "envelope" | "ports" | "confinement";
}
/** Completed regional composition or a request for a larger layout region. */
export type GeometryRegionResult = GeometryRegionEscalation
  | { readonly kind: "region-composed"; readonly regionId: string; readonly geometry: GeometryResult };
/** A bounded region plan whose accepted result can complete the whole scene. */
export interface GeometryRegionReady {
  readonly kind: "region-ready";
  readonly regionId: string;
  readonly plan: GeometryPlan;
  /** Only an actual result accepted by this local plan can complete the composition. */
  complete(result: GeometryResult): GeometryRegionResult;
}
/** Ready regional work or an explicit escalation. */
export type GeometryRegionPreparation = GeometryRegionEscalation | GeometryRegionReady;
/** IDs that must retain exact cross-generation pairing for one region. */
export type GeometryRegionBoundary = GeometryRegionEscalation
  | { readonly kind: "region-boundary"; readonly retainedIds: readonly string[] };
type GeometryKind = "node" | "port" | "route" | "bundle";
type RegionPartition = {
  readonly inside: ReadonlySet<string>;
  readonly localRoutes: ReadonlySet<string>;
  readonly localBundles: ReadonlySet<string>;
  readonly retained: ReadonlyMap<string, GeometryKind>;
};

/** Whole route segments contributed by one route to a shared corridor. */
export interface BundleMemberRange {
  readonly routeId: string;
  /** Half-open whole-segment range in the original directed route, not point indices. */
  readonly firstSegment: number;
  readonly endSegmentExclusive: number;
}
/** A maximal exact run with unchanged membership; different runs may share route IDs. */
export interface SharedRouteCorridor {
  /** Coordinate frame shared by every corridor point. */
  readonly containerId: string;
  readonly routeKind: PlacedRoute["kind"];
  readonly points: readonly Point[];
  readonly members: readonly BundleMemberRange[];
}
/** Accepted shared-route geometry, or an explicit reason sharing was unavailable. */
export type PlacedBundle = (BundleInput & { readonly portId: string }) & (
  | ({ readonly kind: "shared" } & SharedRouteCorridor)
  | { readonly kind: "partitioned"; readonly corridors: readonly SharedRouteCorridor[];
      /** Selected routes that share no nonzero segment with another member. */
      readonly unsharedRouteIds: readonly string[] }
  | { readonly kind: "unshared"; readonly reason: "different-frames" | "different-kinds" | "no-shared-segment" }
);

/** Layout failure is a presentation result, never a compiler diagnostic. */
export class GeometryError extends Error {
  constructor(readonly kind: "input" | "bounds" | "output") {
    super(`Scene geometry rejected: ${kind}`);
    this.name = "GeometryError";
  }
}

const rootId = "$scene";
const portSize = 2;
type ObjectValue = Record<string, unknown>;
function object(value: unknown): ObjectValue {
  if (value === null || typeof value !== "object" || Array.isArray(value)) throw new GeometryError("output");
  return value as ObjectValue;
}
function array(value: unknown, maximum: number): readonly unknown[] {
  if (!Array.isArray(value)) throw new GeometryError("output");
  if (value.length > maximum) throw new GeometryError("bounds");
  return value;
}
function same(actual: unknown, expected: unknown): void {
  if (actual !== expected) throw new GeometryError("output");
}
function inputArray(value: unknown): void {
  if (!Array.isArray(value)) throw new GeometryError("input");
}

/**
 * Own the geometry request before starting ELK, then check its untrusted result.
 * This boundary does not admit semantic input, run a worker, or bound worker memory.
 */
export class GeometryPlan {
  // Weak membership proves local acceptance without retaining a result history.
  readonly #accepted = new WeakSet<GeometryResult>();
  readonly #nodes: NodeInput[] = [];
  readonly #relations: RelationInput[] = [];
  readonly #parents = new Map<string, string>();
  readonly #portOwners = new Map<string, { nodeId: string; side: PortInput["side"] }>();
  readonly #nodeById = new Map<string, NodeInput>();
  readonly #relationById = new Map<string, RelationInput>();
  readonly #bundles: (BundleInput & { readonly portId: string })[] = [];
  readonly #limits: GeometryLimits;
  readonly #fixedPorts = new Map<string, Rectangle>();
  #fixedRoot: { readonly id: string; readonly width: number; readonly height: number } | undefined;

  /** @internal Only this exact plan's accepted results can be associated with a scene. */
  hasAccepted(result: GeometryResult): boolean { return this.#accepted.has(result); }

  /** @internal Capture this owner's implementation, independently of public method overrides. */
  executionInput(): GeometryExecutionInput {
    return issueExecution(this, () => this.#elkInput(), value => this.#accept(value));
  }

  /**
   * Prepare presentation-only contacts over this exact accepted geometry.
   * Segment/pair ceilings precede segment allocation. Frame-map work retains the
   * existing geometry bounds. Floating frame translation is not an error bound.
   * routeOrder must name every accepted route exactly once; later entries are
   * visually above earlier ones. The original route objects remain retained.
   * @throws ContactError
   * For foreign geometry, an invalid permutation or contact limits.
   * @throws GeometryError
   * If root-frame translation exceeds this plan's coordinate ceiling.
   */
  contactInput(geometry: GeometryResult, routeOrder: readonly string[], supplied: ContactLimits): GeometryContactInput {
    if (!this.#accepted.has(geometry)) throw new ContactError("input");
    const limits = contactLimits(supplied);
    if (!Array.isArray(routeOrder) || routeOrder.length !== geometry.routes.length) throw new ContactError("input");
    let count = 0;
    for (const route of geometry.routes) {
      const length = (route.points.length - 1) / (route.kind === "line" ? 1 : 3);
      if (length > limits.segments - count) throw new ContactError("bounds");
      count += length;
    }
    pairCount(count, limits.pairs);
    const byId = new Map(geometry.routes.map(route => [route.id, route])), depth: PlacedRoute[] = [];
    for (let index = 0; index < geometry.routes.length; index++) {
      const route = byId.get(routeOrder[index]!);
      if (!route) throw new ContactError("input");
      depth.push(route); byId.delete(route.id);
    }
    if (byId.size !== 0) throw new ContactError("input");
    const frames = this.#frames(geometry), segments: ContactSegment[] = [];
    for (const route of geometry.routes) {
      const stride = route.kind === "line" ? 1 : 3, frame = frames.get(route.containerId)!;
      for (let start = 0; start + stride < route.points.length; start += stride) {
        const rootControls: Point[] = [];
        for (let offset = 0; offset <= stride; offset++) rootControls.push(Object.freeze(this.#translate(route.points[start + offset]!, frame)));
        segments.push(Object.freeze({ route, index: start / stride, rootControls: Object.freeze(rootControls) }));
      }
    }
    return issueContact(geometry, Object.freeze(segments), Object.freeze(depth), limits);
  }

  /**
   * Copy the declared request and limits; later caller mutation does not update the plan.
   * No layout is run here.
   * @throws GeometryError
   * For invalid IDs, relationships or values exceeding the limits.
   */
  constructor(input: GeometryInput, limits: GeometryLimits) {
    limits = { nodes: limits.nodes, ports: limits.ports, relations: limits.relations,
      points: limits.points, depth: limits.depth, coordinateMagnitude: limits.coordinateMagnitude };
    this.#limits = limits;
    for (const count of [limits.nodes, limits.ports, limits.relations, limits.points, limits.depth]) {
      if (!Number.isSafeInteger(count) || count < 1) throw new GeometryError("bounds");
    }
    if (!Number.isFinite(limits.coordinateMagnitude) || limits.coordinateMagnitude < portSize) throw new GeometryError("bounds");
    if (input.nodes.length > limits.nodes || input.relations.length > limits.relations) throw new GeometryError("bounds");
    const bundles = input.bundles ?? [];
    // With at least two members and one use per route/end, there can be at most R
    // selections and 2R memberships. Check counts before reading members or copying.
    inputArray(bundles);
    if (bundles.length > input.relations.length) throw new GeometryError("bounds");
    let remainingMemberships = input.relations.length * 2;
    for (const bundle of bundles) {
      inputArray(bundle.routeIds);
      const count = bundle.routeIds.length;
      if (count < 2) throw new GeometryError("input");
      if (count > remainingMemberships) throw new GeometryError("bounds");
      remainingMemberships -= count;
    }
    const ids = new Set([rootId]);
    const depths = new Map([[rootId, 0]]);
    const claim = (id: string): void => {
      if (typeof id !== "string" || !/^[a-zA-Z0-9_-]{1,32}$/.test(id) || ids.has(id)) throw new GeometryError("input");
      ids.add(id);
    };
    for (const inputNode of input.nodes) {
      claim(inputNode.id);
      const parentId = inputNode.parentId ?? rootId;
      const parentDepth = depths.get(parentId);
      if (parentDepth === undefined) throw new GeometryError("input");
      if (parentDepth + 1 > limits.depth) throw new GeometryError("bounds");
      this.dimension(inputNode.width); this.dimension(inputNode.height);
      if (inputNode.ports.length > limits.ports - this.#portOwners.size) throw new GeometryError("bounds");
      const ports = inputNode.ports.map(port => {
        claim(port.id);
        if (!["NORTH", "EAST", "SOUTH", "WEST"].includes(port.side)) throw new GeometryError("input");
        this.#portOwners.set(port.id, { nodeId: inputNode.id, side: port.side });
        return { id: port.id, side: port.side };
      });
      const node: NodeInput = { id: inputNode.id, ...(inputNode.parentId === undefined ? {} : { parentId }),
        width: inputNode.width, height: inputNode.height, ports };
      this.#nodes.push(node); this.#nodeById.set(node.id, node);
      this.#parents.set(node.id, parentId); depths.set(node.id, parentDepth + 1);
    }
    for (const relation of input.relations) {
      claim(relation.id);
      if (!this.#portOwners.has(relation.sourcePort) || !this.#portOwners.has(relation.targetPort)) throw new GeometryError("input");
      const owned = { id: relation.id, sourcePort: relation.sourcePort, targetPort: relation.targetPort };
      this.#relations.push(owned); this.#relationById.set(owned.id, owned);
    }
    const used = { source: new Set<string>(), target: new Set<string>() };
    for (const bundle of bundles) {
      claim(bundle.id);
      if (bundle.end !== "source" && bundle.end !== "target") throw new GeometryError("input");
      let portId: string | undefined;
      const routeIds: string[] = [];
      for (const id of bundle.routeIds) {
        const route = this.#relationById.get(id);
        if (!route || used[bundle.end].has(id)) throw new GeometryError("input");
        const port = bundle.end === "source" ? route.sourcePort : route.targetPort;
        if (portId !== undefined && portId !== port) throw new GeometryError("input");
        portId = port; used[bundle.end].add(id); routeIds.push(id);
      }
      if (portId === undefined) throw new GeometryError("input");
      this.#bundles.push({ id: bundle.id, end: bundle.end, routeIds, portId });
    }
  }

  /** @internal Build a fresh geometry request for the pinned ELK adapter. */
  elkInput(): ObjectValue {
    return this.#elkInput();
  }

  #elkInput(): ObjectValue {
    const root: ObjectValue = { id: rootId, children: [], edges: [], layoutOptions: { ...elkConfiguration.rootOptions } };
    const nodes = new Map<string, ObjectValue>([[rootId, root]]);
    for (const input of this.#nodes) {
      const node: ObjectValue = { id: input.id, width: input.width, height: input.height, children: [],
        layoutOptions: { "elk.portConstraints": "FIXED_SIDE", "elk.nodeSize.constraints": "MINIMUM_SIZE",
          "elk.nodeSize.minimum": `(${input.width},${input.height})` },
        ports: input.ports.map(port => ({ id: port.id, width: portSize, height: portSize,
          layoutOptions: { "elk.port.side": port.side } })) };
      if (this.#fixedRoot?.id === input.id) {
        node.width = this.#fixedRoot.width; node.height = this.#fixedRoot.height;
        (node.layoutOptions as ObjectValue)["elk.portConstraints"] = "FIXED_POS";
        (node.layoutOptions as ObjectValue)["elk.nodeSize.minimum"] = `(${this.#fixedRoot.width},${this.#fixedRoot.height})`;
        for (const port of node.ports as ObjectValue[]) {
          const bounds = this.#fixedPorts.get(port.id as string)!;
          port.x = bounds.x; port.y = bounds.y;
        }
      }
      const parent = nodes.get(input.parentId ?? rootId);
      if (!parent) throw new GeometryError("input");
      (parent.children as ObjectValue[]).push(node); nodes.set(input.id, node);
    }
    root.edges = this.#relations.map(relation => ({ id: relation.id,
      sources: [relation.sourcePort], targets: [relation.targetPort] }));
    return root;
  }

  /** @internal
   * Validate an ELK-shaped result and freeze its exact association with this plan.
   * This is geometry acceptance, not evidence of semantic admission or worker execution.
   * @throws GeometryError
   * For missing, substituted, malformed or out-of-bounds geometry.
   */
  accept(value: unknown): GeometryResult {
    return this.#accept(value);
  }

  #accept(value: unknown): GeometryResult {
    const root = object(value); same(root.id, rootId);
    const nodes = new Map<string, PlacedNode>();
    const ports = new Map<string, PlacedPort>();
    const frames = new Map<string, Point>([[rootId, { x: 0, y: 0 }]]);
    const pending: { value: ObjectValue; id: string }[] = [{ value: root, id: rootId }];
    const edges: unknown[] = [];
    const rootBounds = this.rectangle(root);
    same(rootBounds.x, 0); same(rootBounds.y, 0);
    while (pending.length !== 0) {
      const current = pending.pop();
      if (!current) throw new GeometryError("output");
      const frame = frames.get(current.id);
      if (!frame) throw new GeometryError("output");
      for (const childValue of array(current.value.children ?? [], this.#nodes.length - nodes.size)) {
        const child = object(childValue);
        if (typeof child.id !== "string" || nodes.has(child.id) || this.#parents.get(child.id) !== current.id) throw new GeometryError("output");
        const bounds = this.rectangle(child);
        const expected = this.#nodeById.get(child.id);
        if (!expected || bounds.width < expected.width || bounds.height < expected.height) throw new GeometryError("output");
        nodes.set(child.id, { id: child.id, parentId: current.id, bounds });
        frames.set(child.id, this.#translate(bounds, frame));
        pending.push({ value: child, id: child.id });
      }
      for (const portValue of array(current.value.ports ?? [], this.#portOwners.size - ports.size)) {
        const port = object(portValue);
        const expected = typeof port.id === "string" ? this.#portOwners.get(port.id) : undefined;
        if (typeof port.id !== "string" || ports.has(port.id) || expected?.nodeId !== current.id) throw new GeometryError("output");
        const bounds = this.rectangle(port);
        same(bounds.width, portSize); same(bounds.height, portSize);
        const node = nodes.get(current.id);
        if (!node) throw new GeometryError("output");
        this.portSide(bounds, expected.side, node.bounds);
        // Unused ports need the same global bounds check as connected ones.
        const origin = this.#translate(bounds, frame);
        const corner = this.#translate({ x: bounds.x + bounds.width, y: bounds.y + bounds.height }, frame);
        if (origin.x < -1e-7 || origin.y < -1e-7 || corner.x > rootBounds.width + 1e-7
          || corner.y > rootBounds.height + 1e-7) throw new GeometryError("output");
        ports.set(port.id, { id: port.id, nodeId: current.id, bounds });
      }
      // Edges were supplied at the root. Their geometric container may be nested.
      const found = array(current.value.edges ?? [], this.#relations.length - edges.length);
      if (current.id !== rootId && found.length !== 0) throw new GeometryError("output");
      for (const edge of found) edges.push(edge);
    }
    same(nodes.size, this.#nodes.length); same(ports.size, this.#portOwners.size);
    same(edges.length, this.#relations.length);
    for (const node of nodes.values()) {
      const parent = node.parentId === rootId ? rootBounds : nodes.get(node.parentId)?.bounds;
      if (!parent || node.bounds.x < 0 || node.bounds.y < 0
        || node.bounds.x + node.bounds.width > parent.width + 1e-7
        || node.bounds.y + node.bounds.height > parent.height + 1e-7) throw new GeometryError("output");
    }
    const routes = new Map<string, PlacedRoute>();
    let pointCount = 0;
    for (const edgeValue of edges) {
      const edge = object(edgeValue);
      const expected = typeof edge.id === "string" ? this.#relationById.get(edge.id) : undefined;
      if (!expected || routes.has(expected.id)) throw new GeometryError("output");
      const sources = array(edge.sources, 1), targets = array(edge.targets, 1);
      same(sources.length, 1); same(targets.length, 1);
      same(sources[0], expected.sourcePort); same(targets[0], expected.targetPort);
      if (typeof edge.container !== "string" || !frames.has(edge.container)) throw new GeometryError("output");
      const frame = frames.get(edge.container);
      if (!frame) throw new GeometryError("output");
      this.ancestor(edge.container, this.#portOwners.get(expected.sourcePort)?.nodeId);
      this.ancestor(edge.container, this.#portOwners.get(expected.targetPort)?.nodeId);
      const sections = array(edge.sections, 1); same(sections.length, 1);
      const section = object(sections[0]);
      same(section.incomingShape, expected.sourcePort); same(section.outgoingShape, expected.targetPort);
      same(array(section.incomingSections ?? [], 0).length, 0);
      same(array(section.outgoingSections ?? [], 0).length, 0);
      const bends = array(section.bendPoints ?? [], Math.max(0, this.#limits.points - pointCount - 2));
      pointCount += bends.length + 2;
      if (pointCount > this.#limits.points) throw new GeometryError("bounds");
      // ELK 0.12 converts its internal NUB spline to Bézier control points and
      // omits the two endpoints from bendPoints. An unrouted straight edge has none.
      if (bends.length !== 0 && (bends.length + 1) % 3 !== 0) throw new GeometryError("output");
      const start = this.point(section.startPoint), end = this.point(section.endPoint);
      this.contact(this.#translate(start, frame), ports.get(expected.sourcePort), frames);
      this.contact(this.#translate(end, frame), ports.get(expected.targetPort), frames);
      const points = [start];
      for (const point of bends) points.push(this.point(point));
      points.push(end);
      for (const point of points) this.#translate(point, frame);
      routes.set(expected.id, { ...expected, containerId: edge.container,
        kind: bends.length === 0 ? "line" : "cubic-chain", points });
    }
    return this.#retain({ root: rootBounds,
      nodes: this.#nodes.map(node => nodes.get(node.id)!),
      ports: this.#nodes.flatMap(node => node.ports.map(port => ports.get(port.id)!)),
      routes: this.#relations.map(relation => routes.get(relation.id)!),
      bundles: this.#bundles.map(bundle => this.placeBundle(bundle, routes)) });
  }

  /**
   * Preserve accepted geometry when every ordered presentation constraint agrees.
   * Pair every node, port, route and bundle explicitly; IDs and array positions do
   * not establish cross-generation semantic identity. The caller must obtain that
   * evidence separately. Changed constraints require layout for the whole scene.
   * This does not claim that a fresh ELK run with renamed IDs yields equal geometry.
   *
   * Only an immutable result accepted by the supplied prior plan can be reused.
   * A copied or forged result and an incomplete, duplicate or cross-kind mapping
   * are input errors. Different limits conservatively require fresh validation.
   * No worker is started. Frozen points and rectangles are shared, not copied;
   * occurrence wrappers and mappings require linear work in the selected scene.
   */
  reuseFrom(prior: GeometryPlan, accepted: GeometryResult,
    pairs: readonly GeometryOccurrencePair[]): GeometryReuseResult {
    if (!prior.#accepted.has(accepted)) throw new GeometryError("input");
    for (const key of ["nodes", "ports", "relations", "points", "depth", "coordinateMagnitude"] as const) {
      if (prior.#limits[key] !== this.#limits[key]) return { kind: "layout-required", reason: "limits" };
    }
    if (prior.#nodes.length !== this.#nodes.length || prior.#portOwners.size !== this.#portOwners.size
      || prior.#relations.length !== this.#relations.length || prior.#bundles.length !== this.#bundles.length) {
      return { kind: "layout-required", reason: "constraints" };
    }
    const before = prior.#kinds(), current = this.#kinds();
    const mapping = this.#mapping(before, current, pairs);
    const id = (value: string): string => value === rootId ? rootId : mapping.get(value)!;
    if (!this.#constraints(prior, before, current, mapping)) return { kind: "layout-required", reason: "constraints" };
    const corridor = (value: SharedRouteCorridor): SharedRouteCorridor => ({
      containerId: id(value.containerId), routeKind: value.routeKind, points: value.points,
      members: value.members.map(member => ({ ...member, routeId: id(member.routeId) })),
    });
    const geometry = this.#retain({ root: accepted.root,
      nodes: accepted.nodes.map(node => ({ ...node, id: id(node.id), parentId: id(node.parentId) })),
      ports: accepted.ports.map(port => ({ ...port, id: id(port.id), nodeId: id(port.nodeId) })),
      routes: accepted.routes.map(route => ({ ...route, id: id(route.id), containerId: id(route.containerId),
        sourcePort: id(route.sourcePort), targetPort: id(route.targetPort) })),
      bundles: accepted.bundles.map(bundle => {
        const selection = { id: id(bundle.id), end: bundle.end, routeIds: bundle.routeIds.map(id), portId: id(bundle.portId) };
        switch (bundle.kind) {
          case "shared": return { ...selection, kind: "shared", ...corridor(bundle) };
          case "partitioned": return { ...selection, kind: "partitioned", corridors: bundle.corridors.map(corridor),
            unsharedRouteIds: bundle.unsharedRouteIds.map(id) };
          case "unshared": return { ...selection, kind: "unshared", reason: bundle.reason };
        }
      }),
    });
    return { kind: "reused", geometry };
  }

  #kinds(): Map<string, GeometryKind> {
    const result = new Map<string, GeometryKind>();
    for (const node of this.#nodes) result.set(node.id, "node");
    for (const id of this.#portOwners.keys()) result.set(id, "port");
    for (const route of this.#relations) result.set(route.id, "route");
    for (const bundle of this.#bundles) result.set(bundle.id, "bundle");
    return result;
  }

  #mapping(before: ReadonlyMap<string, GeometryKind>, current: ReadonlyMap<string, GeometryKind>,
    pairs: readonly GeometryOccurrencePair[]): Map<string, string> {
    inputArray(pairs);
    const count = pairs.length;
    if (count !== before.size || count !== current.size) throw new GeometryError("input");
    const mapping = new Map<string, string>(), used = new Set<string>();
    for (let index = 0; index < count; index++) {
      const pair = pairs[index];
      if (pair === null || typeof pair !== "object") throw new GeometryError("input");
      const left = pair.priorId, right = pair.currentId;
      if (!before.has(left) || before.get(left) !== current.get(right) || mapping.has(left) || used.has(right)) {
        throw new GeometryError("input");
      }
      mapping.set(left, right); used.add(right);
    }
    return mapping;
  }

  /** One ordered constraint law for both whole-scene and outside-region retention. */
  #constraints(prior: GeometryPlan, before: ReadonlyMap<string, GeometryKind>, current: ReadonlyMap<string, GeometryKind>,
    mapping: ReadonlyMap<string, string>, variableRoot?: string): boolean {
    const id = (value: string): string | undefined => value === rootId ? rootId : mapping.get(value);
    const nodes = this.#nodes.filter(node => current.has(node.id));
    const previousNodes = prior.#nodes.filter(node => before.has(node.id));
    if (nodes.length !== previousNodes.length || !previousNodes.every((node, index) => {
      const next = nodes[index]!;
      return id(node.id) === next.id && id(node.parentId ?? rootId) === (next.parentId ?? rootId)
        && (node.id === variableRoot || (node.width === next.width && node.height === next.height))
        && node.ports.length === next.ports.length
        && node.ports.every((port, position) => id(port.id) === next.ports[position]!.id && port.side === next.ports[position]!.side);
    })) return false;
    const routes = this.#relations.filter(route => current.has(route.id));
    const previousRoutes = prior.#relations.filter(route => before.has(route.id));
    if (routes.length !== previousRoutes.length || !previousRoutes.every((route, index) => {
      const next = routes[index]!;
      return id(route.id) === next.id && id(route.sourcePort) === next.sourcePort && id(route.targetPort) === next.targetPort;
    })) return false;
    const bundles = this.#bundles.filter(bundle => current.has(bundle.id));
    const previousBundles = prior.#bundles.filter(bundle => before.has(bundle.id));
    return bundles.length === previousBundles.length && previousBundles.every((bundle, index) => {
      const next = bundles[index]!;
      return id(bundle.id) === next.id && bundle.end === next.end && id(bundle.portId) === next.portId
        && bundle.routeIds.length === next.routeIds.length
        && bundle.routeIds.every((route, position) => id(route) === next.routeIds[position]);
    });
  }

  /**
   * Find the occurrence IDs that regional preparation must retain, including the
   * region root and its ports. This does not lay out or compare scenes.
   * Routes from any strict descendant to outside the region, and mixed bundles,
   * request a larger region. Use the returned IDs to prepare pairs for prepareRegion.
   * @throws GeometryError
   * If region does not name a node in this plan.
   */
  regionBoundary(region: string): GeometryRegionBoundary {
    if (!this.#nodeById.has(region)) throw new GeometryError("input");
    const partition = this.#partition(region);
    return "kind" in partition ? partition
      : Object.freeze({ kind: "region-boundary", retainedIds: Object.freeze([...partition.retained.keys()]) });
  }

  /**
   * Prepare a bounded local request using already selected outside correspondences.
   * Retained whole-segment control hulls must stay outside the expanded region.
   * This sufficient separation check may reject a curve that would not collide.
   * A local result must preserve the exact envelope and gate rectangles. Completion
   * copies a transient whole-scene input and runs the existing acceptance check.
   * It visits the complete selected scene; it is not a constant-time geometry splice.
   * Pairs cover the region root, its ports and every retained outside occurrence.
   * An escalation asks the caller for a larger layout; it starts no replacement work.
   * @throws GeometryError
   * For foreign accepted geometry, unknown roots or invalid pairs.
   */
  prepareRegion(prior: GeometryPlan, accepted: GeometryResult, priorRoot: string, currentRoot: string,
    pairs: readonly GeometryOccurrencePair[]): GeometryRegionPreparation {
    if (!prior.#accepted.has(accepted) || !prior.#nodeById.has(priorRoot) || !this.#nodeById.has(currentRoot)) {
      throw new GeometryError("input");
    }
    const escalation = (reason: GeometryRegionEscalation["reason"], regionId = rootId): GeometryRegionEscalation =>
      Object.freeze({ kind: "region-escalation", reason, regionId });
    for (const key of ["nodes", "ports", "relations", "points", "depth", "coordinateMagnitude"] as const) {
      if (prior.#limits[key] !== this.#limits[key]) return escalation("limits");
    }
    const current = this.#partition(currentRoot);
    if ("kind" in current) return current;
    const before = prior.#partition(priorRoot);
    if ("kind" in before) return escalation(before.reason);
    if (before.retained.size !== current.retained.size) return escalation("constraints");
    const mapping = this.#mapping(before.retained, current.retained, pairs);
    if (mapping.get(priorRoot) !== currentRoot) throw new GeometryError("input");
    if (!this.#constraints(prior, before.retained, current.retained, mapping, priorRoot)) return escalation("constraints");
    const oldNodes = new Map(accepted.nodes.map(node => [node.id, node]));
    const oldPorts = new Map(accepted.ports.map(port => [port.id, port]));
    const root = oldNodes.get(priorRoot)!;
    const minimum = this.#nodeById.get(currentRoot)!;
    if (minimum.width > root.bounds.width || minimum.height > root.bounds.height) {
      return escalation("envelope", this.#parents.get(currentRoot));
    }
    const oldFrames = prior.#frames(accepted);
    const boundary = { ...root.bounds, ...oldFrames.get(priorRoot)! };
    // Boundary ports extend by two units. Outside routes may touch this outer
    // rectangle, but cannot cross its interior when region contents change.
    const expanded = { x: boundary.x - portSize, y: boundary.y - portSize,
      width: boundary.width + portSize * 2, height: boundary.height + portSize * 2 };
    for (const node of accepted.nodes) {
      if (before.inside.has(node.id) || prior.#commonAncestor(node.id, priorRoot) === node.id) continue;
      const bounds = { ...node.bounds, ...oldFrames.get(node.id)! };
      if (!separated(bounds, expanded)) return escalation("confinement", this.#parents.get(currentRoot));
    }
    for (const route of accepted.routes) {
      if (!before.localRoutes.has(route.id) && !this.#routeOutside(route, oldFrames, expanded)) {
        return escalation("confinement", this.#parents.get(currentRoot));
      }
    }
    const local = new GeometryPlan({
      nodes: this.#nodes.filter(node => current.inside.has(node.id)).map(node => node.id === currentRoot
        ? { id: node.id, width: node.width, height: node.height, ports: node.ports } : node),
      relations: this.#relations.filter(route => current.localRoutes.has(route.id)),
      bundles: this.#bundles.filter(bundle => current.localBundles.has(bundle.id)),
    }, this.#limits);
    const reverse = new Map([...mapping].map(([left, right]) => [right, left]));
    local.#fixedRoot = { id: currentRoot, width: root.bounds.width, height: root.bounds.height };
    for (const port of this.#nodeById.get(currentRoot)!.ports) {
      local.#fixedPorts.set(port.id, oldPorts.get(reverse.get(port.id)!)!.bounds);
    }
    return Object.freeze({ kind: "region-ready", regionId: currentRoot, plan: local,
      complete: (result: GeometryResult): GeometryRegionResult => {
        if (!local.#accepted.has(result)) throw new GeometryError("input");
        const localNodes = new Map(result.nodes.map(node => [node.id, node]));
        const localPorts = new Map(result.ports.map(port => [port.id, port]));
        const changedRoot = localNodes.get(currentRoot)!;
        if (changedRoot.bounds.width !== root.bounds.width || changedRoot.bounds.height !== root.bounds.height) {
          return escalation("envelope", this.#parents.get(currentRoot));
        }
        for (const [id, expected] of local.#fixedPorts) {
          const actual = localPorts.get(id)!.bounds;
          if (!sameRectangle(expected, actual)) return escalation("ports", this.#parents.get(currentRoot));
        }
        const localFrames = local.#frames(result), origin = localFrames.get(currentRoot)!;
        const localEnvelope = { x: origin.x - portSize, y: origin.y - portSize,
          width: changedRoot.bounds.width + portSize * 2, height: changedRoot.bounds.height + portSize * 2 };
        for (const route of result.routes) for (const point of route.points) {
          const global = this.#translate(point, localFrames.get(route.containerId)!);
          if (global.x < localEnvelope.x || global.y < localEnvelope.y
            || global.x > localEnvelope.x + localEnvelope.width || global.y > localEnvelope.y + localEnvelope.height) {
            return escalation("confinement", this.#parents.get(currentRoot));
          }
        }
        let remainingPoints = this.#limits.points;
        for (const route of accepted.routes) if (!before.localRoutes.has(route.id)) {
          if (route.points.length > remainingPoints) return escalation("limits");
          remainingPoints -= route.points.length;
        }
        for (const route of result.routes) {
          if (route.points.length > remainingPoints) return escalation("limits");
          remainingPoints -= route.points.length;
        }
        const id = (value: string): string => value === rootId ? rootId : mapping.get(value)!;
        const retainedRoutes = new Map(accepted.routes.filter(route => !before.localRoutes.has(route.id))
          .map(route => [id(route.id), { ...route, id: id(route.id), sourcePort: id(route.sourcePort),
            targetPort: id(route.targetPort), containerId: id(route.containerId) }]));
        const changedRoutes = new Map(result.routes.map(route => [route.id, route.containerId === rootId
          ? { ...route, containerId: currentRoot,
            points: route.points.map(point => this.#translate(point, { x: -origin.x, y: -origin.y })) } : route]));
        const nodes = this.#nodes.map(node => {
          if (node.id === currentRoot) return { id: node.id, parentId: node.parentId ?? rootId, bounds: root.bounds };
          if (current.inside.has(node.id)) return localNodes.get(node.id)!;
          const old = oldNodes.get(reverse.get(node.id)!)!;
          return { id: node.id, parentId: id(old.parentId), bounds: old.bounds };
        });
        const ports = this.#nodes.flatMap(node => node.ports.map(port => {
          if (current.inside.has(node.id)) return localPorts.get(port.id)!;
          const old = oldPorts.get(reverse.get(port.id)!)!;
          return { id: port.id, nodeId: node.id, bounds: old.bounds };
        }));
        const routes = this.#relations.map(route => changedRoutes.get(route.id) ?? retainedRoutes.get(route.id)!);
        // Transient normalization only: the existing accept path rechecks every
        // current membership, frame, port and route. No raw ELK result is retained.
        const geometry = this.#accept(normalizedInput(accepted.root, nodes, ports, routes));
        return Object.freeze({ kind: "region-composed", regionId: currentRoot, geometry });
      } });
  }

  #partition(region: string): RegionPartition | GeometryRegionEscalation {
    const inside = new Set([region]);
    for (const node of this.#nodes) if (inside.has(node.parentId ?? rootId)) inside.add(node.id);
    const strict = (node: string): boolean => node !== region && inside.has(node);
    const localRoutes = new Set<string>();
    for (const route of this.#relations) {
      const source = this.#portOwners.get(route.sourcePort)!.nodeId, target = this.#portOwners.get(route.targetPort)!.nodeId;
      if (strict(source) || strict(target)) {
        if (!inside.has(source) || !inside.has(target)) return Object.freeze({ kind: "region-escalation",
          reason: "crossing", regionId: this.#commonAncestor(region, inside.has(source) ? target : source) });
        localRoutes.add(route.id);
      }
    }
    const localBundles = new Set<string>();
    for (const bundle of this.#bundles) {
      const localCount = bundle.routeIds.filter(id => localRoutes.has(id)).length;
      if (localCount !== 0 && localCount !== bundle.routeIds.length) return Object.freeze({ kind: "region-escalation",
        reason: "bundle", regionId: this.#parents.get(region)! });
      if (localCount !== 0) localBundles.add(bundle.id);
    }
    const retained = this.#kinds();
    for (const node of this.#nodes) if (strict(node.id)) {
      retained.delete(node.id); for (const port of node.ports) retained.delete(port.id);
    }
    for (const id of localRoutes) retained.delete(id);
    for (const id of localBundles) retained.delete(id);
    return { inside, localRoutes, localBundles, retained };
  }

  #commonAncestor(left: string, right: string): string {
    const ancestors = new Set<string>([rootId]);
    let node: string | undefined = left;
    while (node !== undefined) { ancestors.add(node); node = this.#parents.get(node); }
    node = right;
    while (node !== undefined && !ancestors.has(node)) node = this.#parents.get(node);
    return node ?? rootId;
  }

  #frames(result: GeometryResult): Map<string, Point> {
    const frames = new Map<string, Point>([[rootId, { x: 0, y: 0 }]]);
    for (const node of result.nodes) frames.set(node.id, this.#translate(node.bounds, frames.get(node.parentId)!));
    return frames;
  }

  #routeOutside(route: PlacedRoute, frames: ReadonlyMap<string, Point>, bounds: Rectangle): boolean {
    const frame = frames.get(route.containerId)!;
    const stride = route.kind === "line" ? 1 : 3;
    for (let start = 0; start + stride < route.points.length; start += stride) {
      let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
      for (let offset = 0; offset <= stride; offset++) {
        const point = this.#translate(route.points[start + offset]!, frame);
        minX = Math.min(minX, point.x); maxX = Math.max(maxX, point.x);
        minY = Math.min(minY, point.y); maxY = Math.max(maxY, point.y);
      }
      if (!separated({ x: minX, y: minY, width: maxX - minX, height: maxY - minY }, bounds)) return false;
    }
    return true;
  }

  #retain(geometry: GeometryResult): GeometryResult {
    freezeGeometry(geometry);
    this.#accepted.add(geometry);
    return geometry;
  }

  /** Called only with routes checked in this accept call, before any result escapes. */
  private placeBundle(bundle: BundleInput & { readonly portId: string }, routes: ReadonlyMap<string, PlacedRoute>): PlacedBundle {
    const base = routes.get(bundle.routeIds[0]!);
    if (!base) throw new GeometryError("output");
    // Copy IDs on every acceptance: mutating a prior result cannot alter the plan.
    const selection = { id: bundle.id, end: bundle.end, routeIds: [...bundle.routeIds], portId: bundle.portId };
    const members = bundle.routeIds.map(id => routes.get(id)!);
    const corridors = sharedRouteCorridors(members, bundle.end);
    if (corridors.length === 1 && corridors[0]!.members.length === members.length) {
      return { ...selection, kind: "shared", ...corridors[0]! };
    }
    if (corridors.length !== 0) {
      const shared = new Set(corridors.flatMap(corridor => corridor.members.map(member => member.routeId)));
      return { ...selection, kind: "partitioned", corridors,
        unsharedRouteIds: bundle.routeIds.filter(id => !shared.has(id)) };
    }
    const reason = members.some(route => route.containerId !== base.containerId) ? "different-frames"
      : members.some(route => route.kind !== base.kind) ? "different-kinds" : "no-shared-segment";
    return { ...selection, kind: "unshared", reason };
  }

  #scalar(value: unknown): number {
    if (typeof value !== "number" || !Number.isFinite(value) || Math.abs(value) > this.#limits.coordinateMagnitude) throw new GeometryError("bounds");
    return value;
  }
  private dimension(value: unknown): number {
    const dimension = this.#scalar(value);
    if (dimension < 0) throw new GeometryError("bounds");
    return dimension;
  }
  private point(value: unknown): Point {
    const point = object(value);
    return { x: this.#scalar(point.x), y: this.#scalar(point.y) };
  }
  private rectangle(value: ObjectValue): Rectangle {
    const rectangle = { ...this.point(value), width: this.dimension(value.width), height: this.dimension(value.height) };
    this.#scalar(rectangle.x + rectangle.width); this.#scalar(rectangle.y + rectangle.height);
    return rectangle;
  }
  #translate(point: Point, frame: Point): Point {
    return { x: this.#scalar(point.x + frame.x), y: this.#scalar(point.y + frame.y) };
  }
  private ancestor(container: string, endpoint: string | undefined): void {
    let current = endpoint;
    while (current !== undefined && current !== container) current = this.#parents.get(current);
    if (current !== container) throw new GeometryError("output");
  }
  private portSide(port: Rectangle, side: PortInput["side"], node: Rectangle): void {
    const tolerance = 1e-7;
    const vertical = side === "WEST" || side === "EAST";
    const offset = vertical ? port.x : port.y;
    const expected = side === "WEST" || side === "NORTH" ? -portSize : vertical ? node.width : node.height;
    const along = vertical ? port.y : port.x;
    const maximum = vertical ? node.height : node.width;
    if (Math.abs(offset - expected) > tolerance || along < -tolerance || along + portSize > maximum + tolerance) throw new GeometryError("output");
  }
  private contact(point: Point, port: PlacedPort | undefined, frames: ReadonlyMap<string, Point>): void {
    const frame = port && frames.get(port.nodeId);
    if (!port || !frame) throw new GeometryError("output");
    const origin = this.#translate(port.bounds, frame);
    const x = point.x - origin.x, y = point.y - origin.y, tolerance = 1e-7;
    if (x < -tolerance || x > port.bounds.width + tolerance || y < -tolerance || y > port.bounds.height + tolerance
      || Math.min(Math.abs(x), Math.abs(y), Math.abs(x - port.bounds.width), Math.abs(y - port.bounds.height)) > tolerance) throw new GeometryError("output");
  }
}

function sameRectangle(left: Rectangle, right: Rectangle): boolean {
  return left.x === right.x && left.y === right.y && left.width === right.width && left.height === right.height;
}
function separated(left: Rectangle, right: Rectangle): boolean {
  return left.x + left.width <= right.x || left.x >= right.x + right.width
    || left.y + left.height <= right.y || left.y >= right.y + right.height;
}

/** Adapt already bounded normalized values to the single existing acceptance path. */
function normalizedInput(bounds: Rectangle, nodes: readonly PlacedNode[], ports: readonly PlacedPort[],
  routes: readonly PlacedRoute[]): ObjectValue {
  const root: ObjectValue = { id: rootId, ...bounds, children: [], edges: [] };
  const objects = new Map<string, ObjectValue>([[rootId, root]]);
  for (const node of nodes) {
    const value: ObjectValue = { id: node.id, ...node.bounds, children: [], ports: [] };
    objects.set(node.id, value);
    (objects.get(node.parentId)!.children as ObjectValue[]).push(value);
  }
  for (const port of ports) (objects.get(port.nodeId)!.ports as ObjectValue[]).push({ id: port.id, ...port.bounds });
  root.edges = routes.map(route => ({ id: route.id, sources: [route.sourcePort], targets: [route.targetPort],
    container: route.containerId, sections: [{ incomingShape: route.sourcePort, outgoingShape: route.targetPort,
      startPoint: route.points[0], endPoint: route.points[route.points.length - 1], bendPoints: route.points.slice(1, -1) }] }));
  return root;
}

/** Only call on newly constructed, bounded geometry DTOs, never caller or SDK objects. */
function freezeGeometry(value: object): void {
  for (const member of Object.values(value)) {
    if (member !== null && typeof member === "object" && !Object.isFrozen(member)) freezeGeometry(member);
  }
  Object.freeze(value);
}
