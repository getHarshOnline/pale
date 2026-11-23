import { OrganizationComparisonSide, RecipientQualifiedOrganizationCorrespondence,
  RecipientQualifiedOrganizationEntry, RecipientQualifiedOrganizationRecordHandle,
  RecipientQualifiedWorkspaceComparisonInspection,
  type OrganizationNode, type OrganizationRelationLocality, type OrganizationRow,
  type RecipientQualifiedOrganizationCorrespondenceCursor } from "@rtepl/pale";
import { GeometryPlan, type BundleInput, type GeometryLimits, type NodeInput, type RelationInput,
  type GeometryOccurrencePair, type GeometryResult, type GeometryReuseResult,
  type GeometryRegionPreparation, type GeometryRegionReady, type GeometryRegionEscalation } from "./geometry";
import { captureOrganizationSceneBudget, captureOrganizationSceneCoverage, OrganizationIntake,
  type OrganizationResourceStoppedStep, type OrganizationSceneCoverage, type SceneOccurrence } from "./organizationIntake";
import { chargeRetainedSceneBytes, chargeSceneWork, retainedSceneValueBytes,
  type SceneBudget } from "./sceneBudget";

// Topology may expose the retained SDK entries for inspection, but its own
// semantic reads stay on the original intake and SDK implementations. A caller
// cannot retarget later assembly or cross-generation pairing by shadowing a
// public method or getter on one of those retained objects.
const intakeEntry = OrganizationIntake.prototype.entry;
const intakeCorrespond = OrganizationIntake.prototype.correspond;
const intakeCoverage = captureOrganizationSceneCoverage;
const entryRow = Object.getOwnPropertyDescriptor(
  RecipientQualifiedOrganizationEntry.prototype, "row")!.get!;
const entryHandle = Object.getOwnPropertyDescriptor(
  RecipientQualifiedOrganizationEntry.prototype, "handle")!.get!;
const correspondenceSelectedSide = Object.getOwnPropertyDescriptor(
  RecipientQualifiedOrganizationCorrespondence.prototype, "selectedSide")!.get!;
const correspondenceSelected = Object.getOwnPropertyDescriptor(
  RecipientQualifiedOrganizationCorrespondence.prototype, "selected")!.get!;
const correspondenceState = Object.getOwnPropertyDescriptor(
  RecipientQualifiedOrganizationCorrespondence.prototype, "state")!.get!;
const sameRecord = RecipientQualifiedOrganizationRecordHandle.prototype.sameRecord;

/** Limits for this selected snapshot, not the compilation or total JavaScript memory. */
export interface OrganizationTopologyLimits {
  readonly records: number;
  readonly appearances: number;
  readonly routeAppearances: number;
  /** Sum of UTF-16 code units in indexed endpoint fields, including repeated keys. */
  readonly identityCodeUnits: number;
}

// Match snapshot(): the endpoint and its qualified reference are frozen;
// the underlying SDK grammar and independently returned DTOs stay unchanged.
type EndpointSnapshot<Node extends OrganizationNode = OrganizationNode> = Node extends OrganizationNode
  ? { readonly [Field in keyof Node]: Readonly<Node[Field]> } : never;

/** A local geometry choice backed by a record or an exact external reference. */
export interface EndpointAppearance {
  readonly id: string;
  /** Frozen scene copy, including the qualified reference within it. */
  readonly endpoint: EndpointSnapshot;
  readonly witness: "record" | "binding" | "external-reference";
  readonly occurrence: SceneOccurrence;
}
/** One explicit node placement. Dimensions use scene layout units. */
export interface OrganizationPlacement {
  readonly appearance: EndpointAppearance;
  /** Explicit presentation grouping. It does not assert semantic containment. */
  readonly parent?: EndpointAppearance;
  readonly width: number;
  readonly height: number;
}
/** One explicit depiction of a retained relation or service witness. */
export interface OrganizationRoutePlacement {
  /** Exact explicit relation, derived relation or directional service occurrence. */
  readonly relation: SceneOccurrence;
  readonly source: EndpointAppearance;
  readonly target: EndpointAppearance;
}
/** An explicit shared port at one end of selected visual routes, not a semantic group. */
export interface OrganizationBundlePlacement {
  readonly end: "source" | "target";
  /** Indices into this geometry call's route list. Distinct depictions remain distinct. */
  readonly routeIndices: readonly number[];
}
/** Plan-issued geometry request with its retained semantic route witnesses. */
export interface OrganizationGeometry {
  readonly plan: GeometryPlan;
  /** Main-side route custody; no SDK values enter the geometry request. */
  readonly routes: readonly OrganizationRouteBinding[];
  readonly bundles: readonly OrganizationBundleBinding[];
  readonly unplaced: readonly EndpointAppearance[];
  readonly unrouted: readonly SceneOccurrence[];
}
/**
 * Which existing SDK record supplies a route's meaning. The exact entry retains
 * its full identity, interval and requirements; this tag does not duplicate them.
 */
export type OrganizationRouteWitness =
  | { readonly kind: "relation"; readonly occurrence: SceneOccurrence;
      /** SDK-owned declared locality; no native existence or provider binding is implied. */
      readonly locality: OrganizationRelationLocality }
  | { readonly kind: "derived-relation"; readonly occurrence: SceneOccurrence }
  | { readonly kind: "service"; readonly occurrence: SceneOccurrence };
/** One issued route depiction, not the semantic relation or service identity. */
export interface OrganizationRouteBinding {
  readonly id: string;
  readonly witness: OrganizationRouteWitness;
  readonly selection: OrganizationRoutePlacement;
}
/** One issued explicit shared-port selection. */
export interface OrganizationBundleBinding {
  readonly id: string;
  readonly end: "source" | "target";
  readonly routes: readonly OrganizationRouteBinding[];
}
/** Explicit cross-generation pairs proposed for presentation reuse. */
export interface OrganizationReuseChoices {
  readonly records: readonly { readonly prior: SceneOccurrence; readonly current: SceneOccurrence }[];
  readonly appearances: readonly { readonly prior: EndpointAppearance; readonly current: EndpointAppearance }[];
  readonly routes: readonly { readonly prior: OrganizationRouteBinding; readonly current: OrganizationRouteBinding }[];
  readonly bundles: readonly { readonly prior: OrganizationBundleBinding; readonly current: OrganizationBundleBinding }[];
}
/** One SDK-proved correspondence retained for presentation reuse. */
export interface OrganizationReuseEvidence {
  readonly prior: SceneOccurrence;
  readonly current: SceneOccurrence;
  /** Actual SDK result; current and prior source handles remain independent. */
  readonly correspondence: RecipientQualifiedOrganizationCorrespondence;
}
/** One bounded matching, reuse, regional-layout or terminal step. */
export type OrganizationReuseStep =
  | { readonly kind: "searching" | "record-matched"; readonly prior: SceneOccurrence; readonly current: SceneOccurrence }
  | OrganizationResourceStoppedStep
  | { readonly kind: "unmatched" | "not-included"; readonly evidence: OrganizationReuseEvidence }
  | { readonly kind: "layout-required"; readonly reason: "incomplete-pairing" | "endpoints" | "constraints" | "limits" }
  | { readonly kind: "reused"; readonly geometry: GeometryResult; readonly evidence: readonly OrganizationReuseEvidence[] }
  | (GeometryRegionReady & { readonly evidence: readonly OrganizationReuseEvidence[] })
  | GeometryRegionEscalation
  | { readonly kind: "closed" };
/** One bounded SDK page per step; closing releases this preparation's retained owners. */
export interface OrganizationReuse {
  step(): OrganizationReuseStep;
  close(): void;
}
type Assembly = {
  readonly reuse: GeometryPlan["reuseFrom"];
  readonly regionBoundary: GeometryPlan["regionBoundary"];
  readonly prepareRegion: GeometryPlan["prepareRegion"];
  readonly nodes: ReadonlyMap<EndpointAppearance, string>;
  readonly routes: ReadonlyMap<OrganizationRouteBinding, { readonly id: string; readonly sourcePort: string; readonly targetPort: string }>;
  readonly bundles: ReadonlyMap<OrganizationBundleBinding, { readonly id: string; readonly portId: string }>;
};
type CapturedRelation = { readonly witness: OrganizationRouteWitness;
  readonly from: EndpointSnapshot; readonly to: EndpointSnapshot;
  readonly fromKey: string; readonly toKey: string };
type EndpointChoices =
  | { readonly kind: "not-included"; readonly endpoint: EndpointSnapshot }
  | { readonly kind: "included"; readonly endpoint: EndpointSnapshot; readonly appearances: readonly EndpointAppearance[] };

/** Closed selection, custody and assembly failures for organization geometry. */
export class OrganizationTopologyError extends Error {
  constructor(readonly kind: "bounds" | "duplicate-occurrence" | "foreign-occurrence" | "foreign-appearance"
    | "not-relation" | "duplicate-placement" | "endpoint-mismatch" | "unplaced-endpoint" | "invalid-bundle"
    | "foreign-assembly" | "invalid-pairing") {
    super(`Organization geometry rejected: ${kind}`);
    this.name = "OrganizationTopologyError";
  }
}

/**
 * Connect a bounded selection of compiler-projected organization records to geometry.
 * Every selected record retains its SDK entry, including records with no node.
 * The caller supplies occurrences from one intake; this class never fetches pages.
 * Missing endpoint appearances mean not included here, not absent from the workspace.
 *
 * Occupant/provider appearances are backed by binding witnesses, not invented
 * independent records. A witness explains why the endpoint is present in this
 * selection; its interval does not replace a routed relation's own interval.
 * Native-resource appearances retain a relation's external
 * reference; neither their placement nor their name proves native existence.
 * Explicit relations, derived relations and directional services remain separate,
 * including parallel routes. A service runs from its provider scope to its consumer
 * scope; its five requirements remain independent in the retained SDK entry.
 * No state filtering, relationship derivation or semantic parent selection occurs.
 *
 * The caller chooses each route's appearances explicitly. Repeated appearances do
 * not select a first match or expand into a cross product. IDs are snapshot-local.
 * Index keys compare the existing typed fields; they are not serialized identities,
 * hash domains, authority evidence or cross-generation correspondence.
 *
 * Bounds cover this object's retained references and assembly work. Independently
 * returned SDK handles, caller retention and repeated queries have separate costs.
 * Closing the intake does not revoke entries already retained in this snapshot.
 */
export class OrganizationTopology {
  readonly #records: readonly SceneOccurrence[];
  readonly #appearances: readonly EndpointAppearance[];
  readonly #relations: readonly SceneOccurrence[];
  readonly #intake: OrganizationIntake;
  readonly #entries = new Map<SceneOccurrence, RecipientQualifiedOrganizationEntry>();
  readonly #nodes = new Map<EndpointAppearance, string>();
  readonly #index = new Map<string, readonly EndpointAppearance[]>();
  readonly #edges = new Map<SceneOccurrence, CapturedRelation>();
  readonly #limits: OrganizationTopologyLimits;
  readonly #coverage: OrganizationSceneCoverage;
  readonly #budget: SceneBudget | undefined;
  #identityCodeUnits = 0;
  readonly #witnesses = new Map<EndpointAppearance, "record" | "binding" | "source" | "target">();
  readonly #assemblies = new WeakMap<OrganizationGeometry, Assembly>();

  /** Selected intake occurrences, including records with no visual node. */
  get records(): readonly SceneOccurrence[] { return this.#records; }
  /** Presentation-local endpoint appearances backed by selected occurrences. */
  get appearances(): readonly EndpointAppearance[] { return this.#appearances; }
  /** Enumeration and selection coverage captured before this topology did any semantic work. */
  get coverage(): OrganizationSceneCoverage { return this.#coverage; }
  /** Route-capable records: explicit relations, derived relations and services. */
  get relations(): readonly SceneOccurrence[] { return this.#relations; }

  /** @internal Test the exact intake and issued assembly, without interpreting their values. */
  ownsGeometry(intake: OrganizationIntake, geometry: OrganizationGeometry): boolean {
    return this.#intake === intake && this.#assemblies.has(geometry);
  }

  /** Capture one bounded selection from an open intake without fetching more pages. */
  constructor(intake: OrganizationIntake, occurrences: readonly SceneOccurrence[], limits: OrganizationTopologyLimits) {
    const selectedLimits = { records: limits.records, appearances: limits.appearances,
      routeAppearances: limits.routeAppearances, identityCodeUnits: limits.identityCodeUnits };
    for (const value of [selectedLimits.records, selectedLimits.appearances, selectedLimits.routeAppearances, selectedLimits.identityCodeUnits]) {
      if (!Number.isSafeInteger(value) || value < 1) throw new OrganizationTopologyError("bounds");
    }
    if (!Array.isArray(occurrences)) throw new OrganizationTopologyError("bounds");
    const occurrenceCount = occurrences.length;
    if (!Number.isSafeInteger(occurrenceCount) || occurrenceCount < 0
      || occurrenceCount > selectedLimits.records) throw new OrganizationTopologyError("bounds");
    this.#budget = captureOrganizationSceneBudget(intake);
    chargeSceneWork(this.#budget, 1 + occurrenceCount * 4);
    const selectedOccurrences: SceneOccurrence[] = [];
    for (let index = 0; index < occurrenceCount; index++) {
      // A missing own element must not borrow an occurrence from Array.prototype.
      if (!Object.hasOwn(occurrences, index)) throw new OrganizationTopologyError("bounds");
      selectedOccurrences.push(occurrences[index]!);
    }
    this.#limits = selectedLimits;
    this.#intake = intake;
    this.#coverage = intakeCoverage(intake, selectedOccurrences);
    const appearances: EndpointAppearance[] = [], relations: SceneOccurrence[] = [], records: SceneOccurrence[] = [];
    const index = new Map<string, EndpointAppearance[]>();
    const include = (endpoint: OrganizationNode, occurrence: SceneOccurrence,
      witness: "record" | "binding" | "source" | "target"): void => {
      if (appearances.length === this.#limits.appearances) throw new OrganizationTopologyError("bounds");
      const key = this.#captureKey(endpoint);
      const appearance = Object.freeze({ id: `n${appearances.length}`, endpoint: snapshot(endpoint),
        witness: witness === "source" || witness === "target" ? "external-reference" as const : witness, occurrence });
      this.#witnesses.set(appearance, witness);
      appearances.push(appearance); this.#nodes.set(appearance, key);
      const matches = index.get(key);
      if (matches) matches.push(appearance); else index.set(key, [appearance]);
    };
    for (let index = 0; index < occurrenceCount; index++) {
      const occurrence: SceneOccurrence = selectedOccurrences[index]!;
      if (this.#entries.has(occurrence)) throw new OrganizationTopologyError("duplicate-occurrence");
      const entry = intakeEntry.call(intake, occurrence), row = entryRow.call(entry);
      this.#entries.set(occurrence, entry);
      records.push(occurrence);
      const endpoint = recordEndpoint(row);
      if (endpoint) include(endpoint, occurrence,
        row.kind === "occupant-binding" || row.kind === "provider-binding" ? "binding" : "record");
      if (row.kind === "relation" || row.kind === "derived-relation" || row.kind === "service") {
        // Capture the small endpoint values; mutable client DTOs cannot retarget a later route.
        const from = snapshot(row.kind === "service" ? { kind: "scope", scope: row.contract.providerScope } : row.from);
        const to = snapshot(row.kind === "service" ? { kind: "scope", scope: row.contract.consumerScope } : row.to);
        const witness: OrganizationRouteWitness = row.kind === "relation"
          ? Object.freeze({ kind: row.kind, occurrence, locality: row.locality })
          : Object.freeze({ kind: row.kind, occurrence });
        this.#edges.set(occurrence, { witness, from, to,
          fromKey: this.#captureKey(from), toKey: this.#captureKey(to) });
        relations.push(occurrence);
        if (from.kind === "native-resource") include(from, occurrence, "source");
        if (to.kind === "native-resource") include(to, occurrence, "target");
      }
    }
    for (const [key, matches] of index) this.#index.set(key, Object.freeze(matches));
    this.#records = Object.freeze(records);
    this.#appearances = Object.freeze(appearances); this.#relations = Object.freeze(relations);
    chargeRetainedSceneBytes(this.#budget, retainedSceneValueBytes({
      records: this.#records,
      appearances: this.#appearances.map(item => ({ id: item.id, endpoint: item.endpoint,
        witness: item.witness, occurrence: item.occurrence })),
      relations: this.#relations,
    }));
  }

  /** Return the original retained entry, including non-geometric records and source handles. */
  entry(occurrence: SceneOccurrence): RecipientQualifiedOrganizationEntry {
    return this.#entry(occurrence);
  }

  #entry(occurrence: SceneOccurrence): RecipientQualifiedOrganizationEntry {
    const entry = this.#entries.get(occurrence);
    if (!entry) throw new OrganizationTopologyError("foreign-occurrence");
    return entry;
  }

  /** Enumerate choices already indexed in this selection; never scan or expand the SDK. */
  endpoints(relation: SceneOccurrence): { readonly from: EndpointChoices; readonly to: EndpointChoices } {
    chargeSceneWork(this.#budget, 1);
    const edge = this.#relation(relation);
    const choices = (endpoint: EndpointSnapshot, key: string): EndpointChoices => {
      const appearances = this.#index.get(key);
      return appearances ? { kind: "included", endpoint, appearances } : { kind: "not-included", endpoint };
    };
    return { from: choices(edge.from, edge.fromKey), to: choices(edge.to, edge.toKey) };
  }

  /**
   * Assemble explicit visual choices. Parents must precede children, as GeometryPlan requires.
   * Omissions stay visible. A geometry error does not discard the retained semantic answer.
   * Multiple routes may depict the same relation; each retains the exact selected endpoints.
   * Bundles share a port only at the same selected appearance, never by title or
   * semantic endpoint equality. ELK may still produce no shared whole-segment corridor.
   */
  geometry(placements: readonly OrganizationPlacement[], routes: readonly OrganizationRoutePlacement[],
    geometryLimits: GeometryLimits, bundles: readonly OrganizationBundlePlacement[] = []): OrganizationGeometry {
    geometryLimits = { nodes: geometryLimits.nodes, ports: geometryLimits.ports, relations: geometryLimits.relations,
      points: geometryLimits.points, depth: geometryLimits.depth, coordinateMagnitude: geometryLimits.coordinateMagnitude };
    const placementCount = placements.length, routeCount = routes.length, bundleCount = bundles.length;
    if (!Array.isArray(placements) || !Array.isArray(routes) || !Array.isArray(bundles)) {
      throw new OrganizationTopologyError("bounds");
    }
    for (const count of [placementCount, routeCount, bundleCount]) {
      if (!Number.isSafeInteger(count) || count < 0) throw new OrganizationTopologyError("bounds");
    }
    // Reject allocation-count shortages before reading any placement or route.
    // GeometryPlan remains the owner of dimensions, frames, IDs and geometry validity.
    for (const count of [geometryLimits.nodes, geometryLimits.relations, geometryLimits.ports]) {
      if (!Number.isSafeInteger(count) || count < 1) throw new OrganizationTopologyError("bounds");
    }
    if (placementCount > this.#appearances.length || placementCount > geometryLimits.nodes
      || routeCount > this.#limits.routeAppearances || routeCount > geometryLimits.relations
      || bundleCount > routeCount) {
      throw new OrganizationTopologyError("bounds");
    }
    chargeSceneWork(this.#budget, 1 + placementCount + routeCount + bundleCount);
    const shared = { source: new Map<number, string>(), target: new Map<number, string>() };
    const bundleInputs: BundleInput[] = [];
    const bundleSelections: { end: "source" | "target"; indices: number[]; portId: string }[] = [];
    let portCount = routeCount * 2, memberships = 0;
    // Inspect only bounded bundle selectors before reading any route or placement.
    for (let position = 0; position < bundleCount; position++) {
      const selection: OrganizationBundlePlacement = bundles[position]!;
      const end = selection.end, indices = selection.routeIndices;
      if (end !== "source" && end !== "target") throw new OrganizationTopologyError("invalid-bundle");
      const memberCount = indices.length;
      if (!Array.isArray(indices) || !Number.isSafeInteger(memberCount) || memberCount < 0) {
        throw new OrganizationTopologyError("invalid-bundle");
      }
      if (memberCount > routeCount || memberCount > routeCount * 2 - memberships) {
        throw new OrganizationTopologyError("bounds");
      }
      if (memberCount < 2) throw new OrganizationTopologyError("invalid-bundle");
      const id = `b${bundleInputs.length}`, portId = `${id}p`, routeIds: string[] = [], selectedIndices: number[] = [], ports = shared[end];
      for (let member = 0; member < memberCount; member++) {
        const index: number = indices[member]!;
        if (!Number.isSafeInteger(index) || index < 0 || index >= routeCount || ports.has(index)) {
          throw new OrganizationTopologyError("invalid-bundle");
        }
        ports.set(index, portId); routeIds.push(`r${index}`); selectedIndices.push(index);
      }
      memberships += memberCount; portCount -= memberCount - 1;
      bundleInputs.push({ id, end, routeIds });
      bundleSelections.push({ end, indices: selectedIndices, portId });
    }
    if (portCount > geometryLimits.ports) throw new OrganizationTopologyError("bounds");
    const nodes: NodeInput[] = [], edges: RelationInput[] = [];
    const placed = new Map<EndpointAppearance, { id: string; ports: { id: string; side: "EAST" | "WEST" }[] }>();
    for (let index = 0; index < placementCount; index++) {
      const choice: OrganizationPlacement = placements[index]!;
      const placement = { appearance: choice.appearance, parent: choice.parent, width: choice.width, height: choice.height };
      this.#nodeKey(placement.appearance);
      if (placed.has(placement.appearance)) throw new OrganizationTopologyError("duplicate-placement");
      if (placement.parent) this.#nodeKey(placement.parent);
      const ports: { id: string; side: "EAST" | "WEST" }[] = [];
      const node = { id: placement.appearance.id, ports };
      placed.set(placement.appearance, node);
      nodes.push({ ...node, ...(placement.parent ? { parentId: placement.parent.id } : {}),
        width: placement.width, height: placement.height });
    }
    const routed = new Set<SceneOccurrence>();
    const bindings: OrganizationRouteBinding[] = [];
    const routeMembership = new Map<OrganizationRouteBinding, { id: string; sourcePort: string; targetPort: string }>();
    const portAppearances = new Map<string, EndpointAppearance>();
    const attachPort = (id: string, appearance: EndpointAppearance, node: { ports: { id: string; side: "EAST" | "WEST" }[] },
      side: "EAST" | "WEST"): void => {
      const previous = portAppearances.get(id);
      if (previous && previous !== appearance) throw new OrganizationTopologyError("invalid-bundle");
      if (!previous) { portAppearances.set(id, appearance); node.ports.push({ id, side }); }
    };
    for (let index = 0; index < routeCount; index++) {
      const choice: OrganizationRoutePlacement = routes[index]!;
      const selection = { relation: choice.relation, source: choice.source, target: choice.target };
      const relation = this.#relation(selection.relation);
      if (this.#nodeKey(selection.source) !== relation.fromKey || this.#nodeKey(selection.target) !== relation.toKey) {
        throw new OrganizationTopologyError("endpoint-mismatch");
      }
      const source = placed.get(selection.source), target = placed.get(selection.target);
      if (!source || !target) throw new OrganizationTopologyError("unplaced-endpoint");
      const id = `r${edges.length}`, sourcePort = shared.source.get(edges.length) ?? `p${edges.length}s`,
        targetPort = shared.target.get(edges.length) ?? `p${edges.length}t`;
      attachPort(sourcePort, selection.source, source, "EAST"); attachPort(targetPort, selection.target, target, "WEST");
      edges.push({ id, sourcePort, targetPort }); routed.add(selection.relation);
      const binding = Object.freeze({ id, witness: relation.witness, selection: Object.freeze(selection) });
      bindings.push(binding); routeMembership.set(binding, { id, sourcePort, targetPort });
    }
    const bundleBindings: OrganizationBundleBinding[] = [];
    const bundleMembership = new Map<OrganizationBundleBinding, { id: string; portId: string }>();
    for (const [index, selection] of bundleSelections.entries()) {
      const binding = Object.freeze({ id: bundleInputs[index]!.id, end: selection.end,
        routes: Object.freeze(selection.indices.map(index => bindings[index]!)) });
      bundleBindings.push(binding); bundleMembership.set(binding, { id: binding.id, portId: selection.portId });
    }
    const plan = new GeometryPlan({ nodes, relations: edges, bundles: bundleInputs }, geometryLimits);
    const geometry = Object.freeze({ plan,
      routes: Object.freeze(bindings), bundles: Object.freeze(bundleBindings),
      unplaced: Object.freeze(this.#appearances.filter(node => !placed.has(node))),
      unrouted: Object.freeze(this.#relations.filter(relation => !routed.has(relation))) });
    chargeRetainedSceneBytes(this.#budget, retainedSceneValueBytes({ nodes, relations: edges, bundles: bundleInputs,
      routes: bindings.map(binding => ({ id: binding.id, witness: binding.witness,
        source: binding.selection.source.id, target: binding.selection.target.id })),
      unplaced: geometry.unplaced.map(item => item.id), unrouted: geometry.unrouted }));
    this.#assemblies.set(geometry, { reuse: plan.reuseFrom.bind(plan),
      regionBoundary: plan.regionBoundary.bind(plan), prepareRegion: plan.prepareRegion.bind(plan),
      nodes: new Map([...placed].map(([appearance, node]) => [appearance, node.id])),
      routes: routeMembership, bundles: bundleMembership });
    return geometry;
  }

  /**
   * Qualify explicit visual pairs before asking GeometryPlan to reuse a layout.
   * Each step examines at most one SDK page through the prior intake's budget.
   * No ID, label, list position or digest establishes semantic correspondence.
   * Missing visual pairs require layout; a matching record outside the proposed
   * current selection stays distinct from an unmatched SDK record.
   *
   * Proof is per selected pair, not whole-topology recipient admission. An empty
   * scene can reuse geometry without claiming membership in a comparison pair.
   */
  prepareReuse(prior: OrganizationTopology, priorGeometry: OrganizationGeometry,
    currentGeometry: OrganizationGeometry, accepted: GeometryResult,
    comparison: RecipientQualifiedWorkspaceComparisonInspection,
    choices: OrganizationReuseChoices): OrganizationReuse {
    return this.#prepare(prior, priorGeometry, currentGeometry, accepted, comparison, choices);
  }

  /**
   * Prove retained outside/root pairs, then prepare one explicitly selected region.
   * Added or removed interior appearances remain the current assembly's SDK values.
   * Optional interior route pairs can identify a retained boundary port; they do
   * not cause their old route geometry or old interior endpoints to be reused.
   */
  prepareRegion(prior: OrganizationTopology, priorGeometry: OrganizationGeometry,
    currentGeometry: OrganizationGeometry, accepted: GeometryResult,
    comparison: RecipientQualifiedWorkspaceComparisonInspection,
    roots: { readonly prior: EndpointAppearance; readonly current: EndpointAppearance },
    choices: OrganizationReuseChoices): OrganizationReuse {
    const selectedRoots = { prior: roots.prior, current: roots.current };
    return this.#prepare(prior, priorGeometry, currentGeometry, accepted, comparison, choices, selectedRoots);
  }

  #prepare(prior: OrganizationTopology, priorGeometry: OrganizationGeometry,
    currentGeometry: OrganizationGeometry, accepted: GeometryResult,
    comparison: RecipientQualifiedWorkspaceComparisonInspection,
    choices: OrganizationReuseChoices,
    roots?: { readonly prior: EndpointAppearance; readonly current: EndpointAppearance }): OrganizationReuse {
    const before = prior.#assemblies.get(priorGeometry), current = this.#assemblies.get(currentGeometry);
    if (!before || !current) throw new OrganizationTopologyError("foreign-assembly");
    let beforeRegion: ReadonlySet<string> | undefined, currentRegion: ReadonlySet<string> | undefined;
    let priorRoot: string | undefined, currentRoot: string | undefined;
    if (roots) {
      priorRoot = before.nodes.get(roots.prior); currentRoot = current.nodes.get(roots.current);
      if (priorRoot === undefined || currentRoot === undefined) throw new OrganizationTopologyError("foreign-appearance");
      const right = current.regionBoundary(currentRoot);
      if (right.kind === "region-escalation") return new ReusePreparation({ kind: "terminal", result: right });
      const left = before.regionBoundary(priorRoot);
      if (left.kind === "region-escalation") return new ReusePreparation({ kind: "terminal",
        result: Object.freeze({ ...left, regionId: "$scene" }) });
      beforeRegion = new Set(left.retainedIds); currentRegion = new Set(right.retainedIds);
    }
    const retainBefore = (id: string): boolean => beforeRegion === undefined || beforeRegion.has(id);
    const retainCurrent = (id: string): boolean => currentRegion === undefined || currentRegion.has(id);
    const records = choices.records, appearances = choices.appearances, routes = choices.routes, bundles = choices.bundles;
    for (const list of [records, appearances, routes, bundles]) {
      if (!Array.isArray(list)) throw new OrganizationTopologyError("invalid-pairing");
    }
    const recordCount = records.length, appearanceCount = appearances.length, routeCount = routes.length, bundleCount = bundles.length;
    for (const count of [recordCount, appearanceCount, routeCount, bundleCount]) {
      if (!Number.isSafeInteger(count) || count < 0) throw new OrganizationTopologyError("bounds");
    }
    if (recordCount > Math.min(prior.#records.length, this.#records.length)
      || appearanceCount > Math.min(before.nodes.size, current.nodes.size)
      || routeCount > Math.min(before.routes.size, current.routes.size)
      || bundleCount > Math.min(before.bundles.size, current.bundles.size)) {
      throw new OrganizationTopologyError("bounds");
    }
    chargeSceneWork(this.#budget, 1 + recordCount + appearanceCount + routeCount + bundleCount);
    const recordMap = new Map<SceneOccurrence, SceneOccurrence>(), usedRecords = new Set<SceneOccurrence>();
    const work: ReuseRecord[] = [];
    for (let index = 0; index < recordCount; index++) {
      const pair = records[index]!;
      const left = pair.prior, right = pair.current;
      const priorEntry = prior.#entry(left), currentEntry = this.#entry(right);
      if (recordMap.has(left) || usedRecords.has(right)) throw new OrganizationTopologyError("invalid-pairing");
      recordMap.set(left, right); usedRecords.add(right);
      work.push({ prior: left, current: right, priorEntry, currentEntry });
    }
    const appearanceMap = new Map<EndpointAppearance, EndpointAppearance>(), usedAppearances = new Set<EndpointAppearance>();
    const routeMap = new Map<OrganizationRouteBinding, OrganizationRouteBinding>(), usedRoutes = new Set<OrganizationRouteBinding>();
    const usedBundles = new Set<OrganizationBundleBinding>(), priorBundles = new Set<OrganizationBundleBinding>();
    const geometryMap = new Map<string, string>(), usedIds = new Map<string, string>();
    let complete = roots !== undefined || (appearanceCount === before.nodes.size && appearanceCount === current.nodes.size
      && routeCount === before.routes.size && routeCount === current.routes.size
      && bundleCount === before.bundles.size && bundleCount === current.bundles.size);
    let endpointsAgree = true, constraintsAgree = true;
    const pairId = (left: string, right: string): void => {
      const previous = geometryMap.get(left), reverse = usedIds.get(right);
      if ((previous !== undefined && previous !== right) || (reverse !== undefined && reverse !== left)) {
        constraintsAgree = false;
      } else { geometryMap.set(left, right); usedIds.set(right, left); }
    };
    const requireRecord = (left: SceneOccurrence, right: SceneOccurrence): void => {
      const mapped = recordMap.get(left);
      if (mapped === undefined) complete = false;
      else if (mapped !== right) throw new OrganizationTopologyError("invalid-pairing");
    };
    for (let index = 0; index < appearanceCount; index++) {
      const pair = appearances[index]!;
      const left = pair.prior, right = pair.current;
      const leftId = before.nodes.get(left), rightId = current.nodes.get(right);
      if (leftId === undefined || rightId === undefined) throw new OrganizationTopologyError("foreign-appearance");
      if (appearanceMap.has(left) || usedAppearances.has(right)) throw new OrganizationTopologyError("invalid-pairing");
      appearanceMap.set(left, right); usedAppearances.add(right);
      requireRecord(left.occurrence, right.occurrence);
      if (retainBefore(leftId) !== retainCurrent(rightId)) throw new OrganizationTopologyError("invalid-pairing");
      if (retainBefore(leftId)) {
        endpointsAgree &&= prior.#witnesses.get(left) === this.#witnesses.get(right)
          && prior.#nodeKey(left) === this.#nodeKey(right);
        pairId(leftId, rightId);
      }
    }
    for (let index = 0; index < routeCount; index++) {
      const pair = routes[index]!;
      const left = pair.prior, right = pair.current;
      const leftRoute = before.routes.get(left), rightRoute = current.routes.get(right);
      if (!leftRoute || !rightRoute) throw new OrganizationTopologyError("invalid-pairing");
      if (routeMap.has(left) || usedRoutes.has(right)) throw new OrganizationTopologyError("invalid-pairing");
      routeMap.set(left, right); usedRoutes.add(right);
      requireRecord(left.selection.relation, right.selection.relation);
      endpointsAgree &&= left.witness.kind === right.witness.kind;
      const retained = retainBefore(leftRoute.id);
      if (retained !== retainCurrent(rightRoute.id)) throw new OrganizationTopologyError("invalid-pairing");
      for (const end of ["source", "target"] as const) {
        const leftPort = end === "source" ? leftRoute.sourcePort : leftRoute.targetPort;
        const rightPort = end === "source" ? rightRoute.sourcePort : rightRoute.targetPort;
        if (!retained && !retainBefore(leftPort) && !retainCurrent(rightPort)) continue;
        const mapped = appearanceMap.get(left.selection[end]);
        if (mapped === undefined) complete = false;
        else if (mapped !== right.selection[end]) throw new OrganizationTopologyError("invalid-pairing");
        if (retainBefore(leftPort) !== retainCurrent(rightPort)) throw new OrganizationTopologyError("invalid-pairing");
        pairId(leftPort, rightPort);
      }
      if (retained) pairId(leftRoute.id, rightRoute.id);
    }
    for (let index = 0; index < bundleCount; index++) {
      const pair = bundles[index]!;
      const left = pair.prior, right = pair.current;
      const leftBundle = before.bundles.get(left), rightBundle = current.bundles.get(right);
      if (!leftBundle || !rightBundle || priorBundles.has(left) || usedBundles.has(right)) {
        throw new OrganizationTopologyError("invalid-pairing");
      }
      priorBundles.add(left); usedBundles.add(right);
      if (retainBefore(leftBundle.id) !== retainCurrent(rightBundle.id)) throw new OrganizationTopologyError("invalid-pairing");
      if (!retainBefore(leftBundle.id)) continue;
      constraintsAgree &&= left.end === right.end && left.routes.length === right.routes.length
        && left.routes.every((route, index) => routeMap.get(route) === right.routes[index]);
      pairId(leftBundle.id, rightBundle.id); pairId(leftBundle.portId, rightBundle.portId);
    }
    if (beforeRegion && currentRegion) {
      for (const id of beforeRegion) if (!geometryMap.has(id)) complete = false;
      for (const id of currentRegion) if (!usedIds.has(id)) complete = false;
      if (roots && appearanceMap.get(roots.prior) !== roots.current) complete = false;
    }
    if (!complete) return new ReusePreparation({ kind: "terminal",
      result: Object.freeze({ kind: "layout-required", reason: "incomplete-pairing" }) });
    const pairs: GeometryOccurrencePair[] = [...geometryMap].map(([priorId, currentId]) => ({ priorId, currentId }));
    return new ReusePreparation({ kind: "active", intake: prior.#intake, comparison, records: work,
      index: 0, cursor: undefined, evidence: [], finish: () => {
        if (!endpointsAgree) return { kind: "layout-required", reason: "endpoints" };
        if (!constraintsAgree) return { kind: "layout-required", reason: "constraints" };
        if (priorRoot !== undefined && currentRoot !== undefined) {
          return current.prepareRegion(priorGeometry.plan, accepted, priorRoot, currentRoot, pairs);
        }
        return current.reuse(priorGeometry.plan, accepted, pairs);
      } });
  }

  #relation(occurrence: SceneOccurrence): CapturedRelation {
    this.#entry(occurrence);
    const relation = this.#edges.get(occurrence);
    if (!relation) throw new OrganizationTopologyError("not-relation");
    return relation;
  }
  #nodeKey(appearance: EndpointAppearance): string {
    const key = this.#nodes.get(appearance);
    if (key === undefined) throw new OrganizationTopologyError("foreign-appearance");
    return key;
  }
  #captureKey(endpoint: OrganizationNode): string {
    const parts = endpointFields(endpoint);
    // Check before encoding. JSON escaping uses at most six characters per input code unit.
    for (const part of parts) {
      if (part.length > this.#limits.identityCodeUnits - this.#identityCodeUnits) throw new OrganizationTopologyError("bounds");
      this.#identityCodeUnits += part.length;
    }
    return JSON.stringify(parts);
  }
}

type ReuseRecord = { readonly prior: SceneOccurrence; readonly current: SceneOccurrence;
  readonly priorEntry: RecipientQualifiedOrganizationEntry;
  readonly currentEntry: RecipientQualifiedOrganizationEntry };
type ReuseState = { readonly kind: "active"; readonly intake: OrganizationIntake;
  readonly comparison: RecipientQualifiedWorkspaceComparisonInspection;
  readonly records: readonly ReuseRecord[];
  readonly evidence: OrganizationReuseEvidence[]; index: number;
  cursor: RecipientQualifiedOrganizationCorrespondenceCursor | undefined;
  readonly finish: () => GeometryReuseResult | GeometryRegionPreparation
    | { readonly kind: "layout-required"; readonly reason: "endpoints" } }
  | { readonly kind: "terminal"; readonly result: OrganizationReuseStep }
  | { readonly kind: "closed" };

/** Local continuation only. It is never an input token accepted by another owner. */
class ReusePreparation implements OrganizationReuse {
  #state: ReuseState;
  constructor(state: ReuseState) { this.#state = state; }

  step(): OrganizationReuseStep {
    const state = this.#state;
    if (state.kind === "closed") return { kind: "closed" };
    if (state.kind === "terminal") return state.result;
    if (state.index === state.records.length) return this.#finish(state);
    const record = state.records[state.index]!;
    // This internal continuation does not pass a cancellation signal, so the
    // intake cannot return its between-call stopped disposition here.
    const step = intakeCorrespond.call(state.intake, state.comparison, OrganizationComparisonSide.Prior,
      record.prior, state.cursor) as Exclude<ReturnType<typeof intakeCorrespond>, { readonly kind: "stopped" }>;
    if (step.kind === "closed") { this.close(); return step; }
    if (step.kind === "resource-stop") return step;
    const result = step.result;
    const selected = correspondenceSelected.call(result);
    const priorHandle = entryHandle.call(record.priorEntry);
    if (correspondenceSelectedSide.call(result) !== OrganizationComparisonSide.Prior
      || !sameRecord.call(selected, priorHandle)) throw new OrganizationTopologyError("invalid-pairing");
    const disposition = correspondenceState.call(result);
    if (disposition.kind === "searching") {
      state.cursor = disposition.cursor;
      return { kind: "searching", prior: record.prior, current: record.current };
    }
    const evidence = Object.freeze({ prior: record.prior, current: record.current, correspondence: result });
    const currentMatches = disposition.kind === "matched" && sameRecord.call(
      entryHandle.call(disposition.other), entryHandle.call(record.currentEntry));
    if (disposition.kind === "unmatched" || !currentMatches) {
      const terminal = Object.freeze({ kind: disposition.kind === "unmatched" ? "unmatched" as const : "not-included" as const, evidence });
      this.#state = { kind: "terminal", result: terminal };
      return terminal;
    }
    state.evidence.push(evidence); state.cursor = undefined; state.index++;
    if (state.index === state.records.length) return this.#finish(state);
    return { kind: "record-matched", prior: record.prior, current: record.current };
  }

  #finish(state: Extract<ReuseState, { kind: "active" }>): OrganizationReuseStep {
    const result = state.finish();
    const terminal = result.kind === "reused" || result.kind === "region-ready"
      ? Object.freeze({ ...result, evidence: Object.freeze([...state.evidence]) }) : Object.freeze(result);
    this.#state = { kind: "terminal", result: terminal };
    return terminal;
  }

  close(): void { this.#state = { kind: "closed" }; }
}

/** Mechanical equality of the existing endpoint grammar, not organization admission. */
function endpointFields(node: OrganizationNode): readonly string[] {
  switch (node.kind) {
    case "scope": return [node.kind, node.scope.namespace, node.scope.scopeId];
    case "organization": return [node.kind, node.organization.namespace, node.organization.organizationId];
    case "group": return [node.kind, node.group.namespace, node.group.groupId];
    case "discipline-tree": return [node.kind, node.tree.namespace, node.tree.treeId];
    case "role": return [node.kind, node.role.namespace, node.role.roleId];
    case "occupant": return [node.kind, node.occupant];
    case "provider": return [node.kind, node.provider];
    case "native-resource": return [node.kind, node.nativeResource];
    case "object": return [node.kind, node.object];
    default: { const exhaustive: never = node; return exhaustive; }
  }
}
function snapshot(node: OrganizationNode): EndpointSnapshot {
  switch (node.kind) {
    case "scope": return Object.freeze({ ...node, scope: Object.freeze({ ...node.scope }) });
    case "organization": return Object.freeze({ ...node, organization: Object.freeze({ ...node.organization }) });
    case "group": return Object.freeze({ ...node, group: Object.freeze({ ...node.group }) });
    case "discipline-tree": return Object.freeze({ ...node, tree: Object.freeze({ ...node.tree }) });
    case "role": return Object.freeze({ ...node, role: Object.freeze({ ...node.role }) });
    case "occupant": case "provider": case "native-resource": case "object": return Object.freeze({ ...node });
    default: { const exhaustive: never = node; return exhaustive; }
  }
}
function recordEndpoint(row: OrganizationRow): OrganizationNode | undefined {
  switch (row.kind) {
    case "scope": return { kind: "scope", scope: row.scope };
    case "organization": return { kind: "organization", organization: row.organization };
    case "group": return { kind: "group", group: row.group };
    case "discipline-tree": return { kind: "discipline-tree", tree: row.tree };
    case "role": return { kind: "role", role: row.role };
    case "object": return { kind: "object", object: row.objectId };
    case "occupant-binding": return { kind: "occupant", occupant: row.occupant };
    case "provider-binding": return { kind: "provider", provider: row.provider };
    case "communication-actor": case "leadership": case "relation": case "population-rule":
    case "group-lens": case "service": case "native-projection": case "inspection":
    case "derived-relation": case "responsibility": return undefined;
    default: { const exhaustive: never = row; return exhaustive; }
  }
}
