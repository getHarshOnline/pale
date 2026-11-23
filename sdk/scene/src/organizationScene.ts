import { EffectiveProfileCompilation, ObservationProfileAssociation,
  ObservationProfileAssociationResult, ObservationProfileAssociationResultKind,
  ObservationSnapshot, OrganizationRelationLocality, RecipientQualifiedOrganizationEntry,
  RecipientQualifiedWorkspaceComparisonInspection, RecipientQualifiedWorkspaceInspection,
  RecipientQualifiedWorkspacePairSummary, RecipientQualifiedWorkspaceSummary,
  type EffectiveProfileCoordinates, type ObservationCursor, type ObservationProfileError,
  type OrganizationEvidenceRequirement } from "@rtepl/pale";
import { GeometryPlan, GeometryExecutionInput, type GeometryAlgorithm, type GeometryResult,
  type GeometryRegionEscalation, type PlacedPort, type PlacedRoute } from "./geometry";
import { ElkExecutor, type LayoutResult } from "./elkExecutor";
import { ContactExecutor, type ContactResult } from "./contactExecutor";
import type { ContactAnalysis, ContactLimits } from "./contactAnalysis";
import { OrganizationIntake } from "./organizationIntake";
import { captureOrganizationSceneBudget, captureOrganizationSceneInspection,
  type OrganizationSceneCoverage } from "./organizationIntake";
import { OrganizationTopology, type EndpointAppearance, type OrganizationGeometry,
  type OrganizationReuse, type OrganizationReuseChoices, type OrganizationReuseEvidence,
  type OrganizationReuseStep, type OrganizationRouteBinding } from "./organizationTopology";
import { chargeEncodedSceneOutput, chargeRetainedSceneBytes, chargeSceneWork, retainedSceneValueBytes,
  type SceneBudget } from "./sceneBudget";

/**
 * Exact semantic identity retained by the recipient-qualified inspection.
 * These values do not establish resident generation or native observation.
 */
export interface OrganizationSceneSource {
  readonly kind: "recipient-qualified-inspection";
  readonly workspaceId: string;
  readonly principalUniverseDigest: string;
  readonly semanticIrDigest: string;
  readonly admissionEvidenceDigest: string;
}
/**
 * Meaning of the organization scene's spatial projection.
 *
 * Proximity comes only from the selected presentation hierarchy, dimensions,
 * ports, routes and fixed layout options. It does not establish semantic
 * similarity, authority, causality, disclosure, freshness or native readiness.
 */
export interface OrganizationStructuralLens {
  readonly kind: "organization-structural";
  readonly subject: "scene-appearance";
  readonly grouping: "explicit-presentation-selection";
  readonly proximity: "layout-constraints-only";
  readonly routes: "selected-sdk-witnesses";
}
const organizationStructuralLens: OrganizationStructuralLens = Object.freeze({
  kind: "organization-structural",
  subject: "scene-appearance",
  grouping: "explicit-presentation-selection",
  proximity: "layout-constraints-only",
  routes: "selected-sdk-witnesses",
});
/** How this generation was produced and, when applicable, paired with its predecessor. */
export type OrganizationSceneCorrespondence =
  | { readonly kind: "full" }
  | { readonly kind: "reused"; readonly priorSource: OrganizationSceneSource;
      readonly comparison: RecipientQualifiedWorkspaceComparisonInspection;
      readonly evidence: readonly OrganizationReuseEvidence[] }
  | { readonly kind: "regional"; readonly priorSource: OrganizationSceneSource;
      readonly comparison: RecipientQualifiedWorkspaceComparisonInspection;
      readonly evidence: readonly OrganizationReuseEvidence[];
      readonly regionId: string };
/** Actual work for this publication, distinct from the complete target constraints. */
export type OrganizationSceneExecution =
  | { readonly kind: "full"; readonly input: GeometryExecutionInput; readonly geometry: GeometryResult }
  | { readonly kind: "reused" }
  | { readonly kind: "regional"; readonly input: GeometryExecutionInput; readonly geometry: GeometryResult };
/**
 * Depiction of a declared native endpoint, not a runtime control gate.
 * The explicit relation's SDK-owned locality establishes the declaration; its
 * state, interval and source evidence remain with the exact binding's witness.
 * Neither this placement nor a declaration in Unknown state proves native
 * existence, an executable provider binding, health or permission to cross.
 */
export interface OrganizationDeclaredGate {
  readonly binding: OrganizationRouteBinding;
  readonly end: "source" | "target";
  readonly appearance: EndpointAppearance;
  /** Exact accepted port. Its bounds are relative to port.nodeId, not the route frame. */
  readonly port: PlacedPort;
  /** Exact accepted route, retaining its independently checked container frame. */
  readonly route: PlacedRoute;
}
/** One routed directional service boundary. It does not prove provider readiness. */
export interface OrganizationServiceBoundary {
  readonly kind: "service-boundary";
  readonly binding: OrganizationRouteBinding;
  readonly route: PlacedRoute;
  readonly source: { readonly appearance: EndpointAppearance; readonly port: PlacedPort };
  readonly target: { readonly appearance: EndpointAppearance; readonly port: PlacedPort };
}
/** One requirement already carried by a routed directional service. */
export interface OrganizationAcceptanceFrontier {
  readonly kind: "acceptance-frontier";
  readonly frontier: "provider-completion" | "provider-acceptance" | "consumer-acceptance"
    | "native-read-back" | "pale-settlement";
  readonly binding: OrganizationRouteBinding;
  readonly route: PlacedRoute;
  readonly requirement: Readonly<OrganizationEvidenceRequirement>;
}
/** Existing routed boundary evidence. It does not authorize a crossing. */
export type OrganizationBoundaryAssociation =
  | ({ readonly kind: "native-boundary" } & OrganizationDeclaredGate)
  | OrganizationServiceBoundary
  | OrganizationAcceptanceFrontier;
/** One routed witness in back-to-front presentation order. */
export interface OrganizationOrderedRoute {
  /**
   * Zero-based occlusion depth. Zero is the furthest route from the viewer.
   * Depth carries no causal, authority, priority, execution or evidence meaning.
   */
  readonly depth: number;
  readonly binding: OrganizationRouteBinding;
  readonly route: PlacedRoute;
}
const routeOrderPermit = Symbol("issued organization route order");
let issueRouteOrder: (scene: OrganizationScene,
  routes: readonly OrganizationOrderedRoute[], encodedOutputBytes: number) => OrganizationRouteOrder;
let retainedRouteOrder: (order: OrganizationRouteOrder) => {
  readonly scene: OrganizationScene; readonly routes: readonly OrganizationOrderedRoute[];
};

/**
 * Exact route layering for one scene. Later routes are drawn above earlier routes.
 * The order controls visual composition only.
 */
export class OrganizationRouteOrder {
  readonly #scene: OrganizationScene;
  readonly #routes: readonly OrganizationOrderedRoute[];
  readonly #encodedOutputBytes: number;
  private constructor(token: symbol, scene: OrganizationScene,
    routes: readonly OrganizationOrderedRoute[], encodedOutputBytes: number) {
    if (token !== routeOrderPermit) throw new TypeError("Foreign organization route order");
    this.#scene = scene; this.#routes = routes; this.#encodedOutputBytes = encodedOutputBytes;
    Object.freeze(this);
  }
  static {
    issueRouteOrder = (scene, routes, encodedOutputBytes) =>
      new OrganizationRouteOrder(routeOrderPermit, scene, routes, encodedOutputBytes);
    retainedRouteOrder = order => ({ scene: order.#scene, routes: order.#routes });
  }
  /** Routes are listed from the visual background toward the foreground. */
  get layering(): "back-to-front" { return "back-to-front"; }
  /** Complete back-to-front order. Each item retains its semantic route witness. */
  get routes(): readonly OrganizationOrderedRoute[] { return this.#routes; }
  /** Bytes in the renderer-neutral depth projection charged to the shared budget. */
  get encodedOutputBytes(): number { return this.#encodedOutputBytes; }
  /** Test exact scene custody. Equal geometry or route IDs are insufficient. */
  belongsTo(scene: OrganizationScene): boolean { return this.#scene === scene; }
}
type Context = { readonly owner: RecipientQualifiedWorkspaceInspection;
  readonly source: OrganizationSceneSource;
  readonly budget: SceneBudget };
type Selection = Context & { readonly topology: OrganizationTopology; readonly assembly: OrganizationGeometry;
  readonly execution: GeometryExecutionInput };
type SceneState = Selection & { readonly geometry: GeometryResult;
  readonly declaredGates: readonly OrganizationDeclaredGate[];
  readonly boundaries: readonly OrganizationBoundaryAssociation[];
  readonly performed: OrganizationSceneExecution;
  readonly correspondence: OrganizationSceneCorrespondence; readonly encodedOutputBytes: number;
  readonly priorOwner?: RecipientQualifiedWorkspaceInspection };
const permit = Symbol("issued organization scene");
let issue: (state: SceneState) => OrganizationScene;
let retained: (scene: OrganizationScene) => Selection & { readonly geometry: GeometryResult };
const generationPermit = Symbol("issued organization scene generation");
let issueGeneration: (scene: OrganizationScene, state: SceneState) => OrganizationSceneGeneration;
const associationPermit = Symbol("issued effective-profile scene association");
let issueAssociation: (scene: OrganizationScene, snapshot: ObservationSnapshot,
  association: ObservationProfileAssociation) => EffectiveProfileSceneAssociation;

/** Exact identity and interpretation bounds of one accepted scene result. */
export class OrganizationSceneGeneration {
  readonly #scene: OrganizationScene;
  readonly #state: SceneState;
  private constructor(token: symbol, scene: OrganizationScene, state: SceneState) {
    if (token !== generationPermit) throw new TypeError("Foreign organization scene generation");
    this.#scene = scene; this.#state = state; Object.freeze(this);
  }
  static {
    issueGeneration = (scene, state) => new OrganizationSceneGeneration(generationPermit, scene, state);
  }
  /** Semantic coordinates retained by the recipient-qualified inspection. */
  get source(): OrganizationSceneSource { return this.#state.source; }
  /** Declared meaning of spatial proximity in this generation. */
  get lens(): OrganizationStructuralLens { return organizationStructuralLens; }
  /** Enumeration and selection coverage captured before layout. */
  get coverage(): OrganizationSceneCoverage { return this.#state.topology.coverage; }
  /** Exact plan-issued target constraints for this generation. */
  get constraints(): GeometryExecutionInput { return this.#state.execution; }
  /** Accepted renderer-neutral geometry in scene layout units. */
  get geometry(): GeometryResult { return this.#state.geometry; }
  /** Cross-generation evidence used to publish this generation. */
  get correspondence(): OrganizationSceneCorrespondence { return this.#state.correspondence; }
  /** Exact encoded bytes charged to the scene's aggregate budget. */
  get encodedOutputBytes(): number { return this.#state.encodedOutputBytes; }
  /** Test exact scene custody. Equal source coordinates are insufficient. */
  belongsTo(scene: OrganizationScene): boolean { return this.#scene === scene; }
}

/** One accepted presentation with exact source and semantic route witnesses. */
export class OrganizationScene {
  readonly #state: SceneState;
  readonly #generation: OrganizationSceneGeneration;
  private constructor(token: symbol, state: SceneState) {
    if (token !== permit) throw new OrganizationSceneError("foreign-scene");
    this.#state = state; this.#generation = issueGeneration(this, state); Object.freeze(this);
  }
  static {
    issue = state => new OrganizationScene(permit, state);
    // Do not retain a prior scene's prior history through repeated updates.
    retained = scene => ({ owner: scene.#state.owner, source: scene.#state.source,
      budget: scene.#state.budget,
      topology: scene.#state.topology, assembly: scene.#state.assembly, execution: scene.#state.execution,
      geometry: scene.#state.geometry });
  }
  /** Semantic coordinates retained by the recipient-qualified inspection. */
  get source(): OrganizationSceneSource { return this.#state.source; }
  /** Immutable identity and interpretation bounds for this accepted result. */
  get generation(): OrganizationSceneGeneration { return this.#generation; }
  /** Test exact admitted inspection custody; matching source digests are insufficient. */
  belongsTo(owner: RecipientQualifiedWorkspaceInspection): boolean {
    return this.#state.owner === owner;
  }
  /** Declares how this scene may be interpreted; it grants no semantic fact. */
  get lens(): OrganizationStructuralLens { return organizationStructuralLens; }
  /** Pinned layout engine identity and fixed root options. */
  get algorithm(): GeometryAlgorithm { return this.#state.execution.algorithm; }
  /** Complete target constraints, including geometry retained outside a local region. */
  get constraints(): GeometryExecutionInput { return this.#state.execution; }
  /** Work actually performed for this publication. */
  get execution(): OrganizationSceneExecution { return this.#state.performed; }
  /** Retained semantic witnesses and presentation selection. */
  get topology(): OrganizationTopology { return this.#state.topology; }
  /** Current topology coverage. Layout and prior scenes cannot upgrade it. */
  get coverage(): OrganizationSceneCoverage { return this.#state.topology.coverage; }
  /** Plan and semantic route bindings selected before layout. */
  get assembly(): OrganizationGeometry { return this.#state.assembly; }
  /** Accepted renderer-neutral geometry in scene layout units. */
  get geometry(): GeometryResult { return this.#state.geometry; }
  /** Selected routed native endpoints only; an empty result does not prove absence. */
  get declaredGates(): readonly OrganizationDeclaredGate[] { return this.#state.declaredGates; }
  /** Routed boundary and frontier evidence selected from existing SDK witnesses. */
  get boundaries(): readonly OrganizationBoundaryAssociation[] { return this.#state.boundaries; }
  /** Cross-generation evidence used to publish this scene. */
  get correspondence(): OrganizationSceneCorrespondence { return this.#state.correspondence; }
  /**
   * Derive the stable default route order from the retained route-binding order.
   *
   * The result controls occlusion and composition only. It does not establish
   * causal order, authority, priority, execution order or evidence strength.
   */
  canonicalRouteOrder(): OrganizationRouteOrder {
    return this.#orderRoutes(this.#state.assembly.routes.map(binding => binding.id));
  }
  /**
   * Apply an explicit presentation override to the complete route order.
   *
   * The override controls occlusion and composition only. It does not create a
   * relationship or boundary crossing and carries no causal, authority,
   * priority, execution or evidence meaning.
   * Work and retention are linear in the scene's already bounded route count.
   */
  orderRoutes(routeIds: readonly string[]): OrganizationRouteOrder {
    return this.#orderRoutes(routeIds);
  }
  #orderRoutes(routeIds: readonly string[]): OrganizationRouteOrder {
    if (!Array.isArray(routeIds) || routeIds.length !== this.#state.geometry.routes.length) {
      throw new OrganizationSceneError("invalid-route-order");
    }
    const routes = new Map(this.#state.geometry.routes.map(route => [route.id, route]));
    const bindings = new Map(this.#state.assembly.routes.map(binding => [binding.id, binding]));
    const ordered: OrganizationOrderedRoute[] = [];
    const count = routeIds.length;
    chargeSceneWork(this.#state.budget, 1 + count * 2);
    for (let index = 0; index < count; index++) {
      if (!Object.hasOwn(routeIds, index)) throw new OrganizationSceneError("invalid-route-order");
      const id = routeIds[index], route = typeof id === "string" ? routes.get(id) : undefined;
      const binding = typeof id === "string" ? bindings.get(id) : undefined;
      if (!route || !binding) throw new OrganizationSceneError("invalid-route-order");
      ordered.push(Object.freeze({ depth: index, binding, route }));
      routes.delete(id); bindings.delete(id);
    }
    if (routes.size !== 0 || bindings.size !== 0) throw new OrganizationSceneError("invalid-route-order");
    const projection = ordered.map(item => ({ depth: item.depth,
      bindingId: item.binding.id, routeId: item.route.id }));
    chargeRetainedSceneBytes(this.#state.budget, retainedSceneValueBytes(projection));
    const encodedOutputBytes = chargeEncodedSceneOutput(this.#state.budget, projection);
    return issueRouteOrder(this, Object.freeze(ordered), encodedOutputBytes);
  }
}

const retainedSceneGeneration = Object.getOwnPropertyDescriptor(
  OrganizationScene.prototype, "generation")!.get!;

/** One profile observation associated with one exact scene and snapshot. */
export class EffectiveProfileSceneAssociation {
  readonly #scene: OrganizationScene;
  readonly #generation: OrganizationSceneGeneration;
  readonly #snapshot: ObservationSnapshot;
  readonly #association: ObservationProfileAssociation;
  private constructor(token: symbol, scene: OrganizationScene, snapshot: ObservationSnapshot,
    association: ObservationProfileAssociation) {
    if (token !== associationPermit) throw new TypeError("Foreign effective-profile scene association");
    this.#scene = scene;
    this.#generation = retainedSceneGeneration.call(scene) as OrganizationSceneGeneration;
    this.#snapshot = snapshot;
    this.#association = association;
    Object.freeze(this);
  }
  static {
    issueAssociation = (scene, snapshot, association) =>
      new EffectiveProfileSceneAssociation(associationPermit, scene, snapshot, association);
  }
  /** Return the exact scene supplied to the association. */
  get scene(): OrganizationScene { return this.#scene; }
  /** Return the exact accepted scene generation. */
  get sceneGeneration(): OrganizationSceneGeneration { return this.#generation; }
  /** Return the exact snapshot supplied to the association. */
  get snapshot(): ObservationSnapshot { return this.#snapshot; }
  /** Return all coordinates of the associated effective profile. */
  get profileCoordinates(): EffectiveProfileCoordinates {
    return associationCoordinates.call(this.#association) as EffectiveProfileCoordinates;
  }
  /** Return the cursor of the exact retained observation snapshot. */
  get observationCursor(): ObservationCursor {
    return associationCursor.call(this.#association) as ObservationCursor;
  }
  /** Test exact scene and snapshot custody. Equal coordinates are insufficient. */
  belongsTo(scene: OrganizationScene, snapshot: ObservationSnapshot): boolean {
    return this.#scene === scene && this.#snapshot === snapshot
      && associationBelongsTo.call(this.#association, snapshot) as boolean;
  }
}

/** Result of associating an effective-profile observation with a retained scene. */
export type EffectiveProfileSceneAssociationResult =
  | { readonly kind: "associated"; readonly association: EffectiveProfileSceneAssociation }
  | { readonly kind: "refused"; readonly error: ObservationProfileError };

const associateProfile = RecipientQualifiedWorkspaceInspection.prototype.associateProfileObservation;
const associationResultKind = Object.getOwnPropertyDescriptor(
  ObservationProfileAssociationResult.prototype, "kind")!.get!;
const resultAssociation = Object.getOwnPropertyDescriptor(
  ObservationProfileAssociationResult.prototype, "association")!.get!;
const associationError = Object.getOwnPropertyDescriptor(
  ObservationProfileAssociationResult.prototype, "error")!.get!;
const associationCoordinates = Object.getOwnPropertyDescriptor(
  ObservationProfileAssociation.prototype, "coordinates")!.get!;
const associationCursor = Object.getOwnPropertyDescriptor(
  ObservationProfileAssociation.prototype, "cursor")!.get!;
const associationBelongsTo = ObservationProfileAssociation.prototype.belongsTo;

/**
 * Associate an effective profile and observation with a recipient-qualified scene.
 *
 * This fixed-work check performs no scene query, layout, or relayout. It adds
 * no authority, freshness, readiness, or effect meaning to either handle.
 */
export function associateEffectiveProfileObservation(scene: OrganizationScene,
  snapshot: ObservationSnapshot, profile: EffectiveProfileCompilation): EffectiveProfileSceneAssociationResult {
  const current = retained(scene);
  const result = associateProfile.call(current.owner, snapshot, profile) as ObservationProfileAssociationResult;
  if (associationResultKind.call(result) === ObservationProfileAssociationResultKind.Refused) {
    return Object.freeze({ kind: "refused", error: associationError.call(result) as ObservationProfileError });
  }
  const association = resultAssociation.call(result) as ObservationProfileAssociation | null;
  if (association === null) throw new TypeError("Associated result has no association");
  return Object.freeze({ kind: "associated", association: issueAssociation(scene, snapshot, association) });
}

/** Closed custody, lifecycle and accepted-result failures for scene publication. */
export class OrganizationSceneError extends Error {
  constructor(readonly kind: "closed" | "unselected" | "foreign-assembly" | "foreign-scene" | "foreign-result"
    | "foreign-route-order" | "invalid-route-order" | "comparison-mismatch") {
    super(`Organization scene rejected: ${kind}`);
    this.name = "OrganizationSceneError";
  }
}
/** Full or regional layout result. Completion always carries an accepted scene. */
export type OrganizationSceneLayout = Exclude<LayoutResult, { readonly kind: "completed" }>
  | { readonly kind: "completed"; readonly scene: OrganizationScene } | GeometryRegionEscalation;
/** Optional estimated annotation; the scene and its semantic witnesses stay intact. */
export type OrganizationSceneContacts = Exclude<ContactResult, { readonly kind: "completed" }>
  | { readonly kind: "completed"; readonly scene: OrganizationScene; readonly analysis: ContactAnalysis;
      /** Bytes in the renderer-neutral annotation projection charged to the shared budget. */
      readonly encodedOutputBytes: number };
/** One matching, reuse, regional-layout, escalation or terminal lifecycle step. */
export type OrganizationSceneReuseStep = Exclude<OrganizationReuseStep, { readonly kind: "reused" | "region-ready" }>
  | { readonly kind: "reused"; readonly scene: OrganizationScene }
  | { readonly kind: "region-ready"; readonly regionId: string;
      /** Lay out the prepared region. Cancellation or replacement prevents scene publication. */
      layout(executor: ElkExecutor, signal?: AbortSignal): Promise<OrganizationSceneLayout> };
/** Stepwise reuse preparation. Closing releases retained owners and prevents publication. */
export interface OrganizationSceneReuse {
  /** Advance at most one bounded matching step. Terminal results are stable. */
  step(): OrganizationSceneReuseStep;
  /** Release retained owners. Repeated close calls are harmless. */
  close(): void;
}
const retainInspection = OrganizationIntake.prototype.retainInspection;
const ownsGeometry = OrganizationTopology.prototype.ownsGeometry;
const hasAccepted = GeometryPlan.prototype.hasAccepted;
const executionInput = GeometryPlan.prototype.executionInput;
const executionBelongsTo = GeometryExecutionInput.prototype.belongsTo;
const prepareReuse = OrganizationTopology.prototype.prepareReuse;
const prepareRegion = OrganizationTopology.prototype.prepareRegion;
const execute = ElkExecutor.prototype.run;
const contactInput = GeometryPlan.prototype.contactInput;
const analyze = ContactExecutor.prototype.run;
const topologyEntry = OrganizationTopology.prototype.entry;
const entryRow = Object.getOwnPropertyDescriptor(
  RecipientQualifiedOrganizationEntry.prototype, "row")!.get!;
const inspectionSummary = RecipientQualifiedWorkspaceInspection.prototype.summary;
const sourceWorkspaceId = Object.getOwnPropertyDescriptor(
  RecipientQualifiedWorkspaceSummary.prototype, "workspaceId")!.get!;
const sourcePrincipalUniverseDigest = Object.getOwnPropertyDescriptor(
  RecipientQualifiedWorkspaceSummary.prototype, "principalUniverseDigest")!.get!;
const sourceSemanticIrDigest = Object.getOwnPropertyDescriptor(
  RecipientQualifiedWorkspaceSummary.prototype, "semanticIrDigest")!.get!;
const sourceAdmissionEvidenceDigest = Object.getOwnPropertyDescriptor(
  RecipientQualifiedWorkspaceSummary.prototype, "admissionEvidenceDigest")!.get!;
const comparesExactInspections =
  RecipientQualifiedWorkspaceComparisonInspection.prototype.comparesExactInspections;
const comparisonSummaries = RecipientQualifiedWorkspaceComparisonInspection.prototype.summaries;
const priorSummary = Object.getOwnPropertyDescriptor(
  RecipientQualifiedWorkspacePairSummary.prototype, "prior")!.get!;
const currentSummary = Object.getOwnPropertyDescriptor(
  RecipientQualifiedWorkspacePairSummary.prototype, "current")!.get!;

function contactOutputProjection(analysis: ContactAnalysis): unknown {
  const branch = (value: ContactAnalysis["contacts"][number]["first"]) => ({
    routeId: value.segment.route.id,
    segment: value.segment.index,
    backendTime: value.backendTime,
    parameter: value.parameter,
    parameterKind: value.parameterKind,
    point: value.point,
    reportedPoint: value.reportedPoint,
  });
  const contact = (value: ContactAnalysis["contacts"][number]) => ({
    first: branch(value.first), second: branch(value.second),
    separation: value.separation, above: value.above,
  });
  return {
    model: analysis.model,
    coverage: analysis.coverage,
    certifiedErrorBound: analysis.certifiedErrorBound,
    visitedPairs: analysis.visitedPairs,
    contacts: analysis.contacts.map(contact),
    overlaps: analysis.overlaps.map(value => ({ start: contact(value.start), end: contact(value.end) })),
  };
}

/**
 * Join existing scene owners, without a queue or new layout/inspection budget.
 * select() replaces one retained selection; an older completion cannot publish.
 * The executor owns worker cancellation and release. The compiler rechecks
 * cancellation and selection after release before it publishes a result.
 * Closing drops this compiler's selection. Issued scenes retain their qualified
 * inspection owners, not a recursive scene history.
 */
export class OrganizationSceneCompiler {
  #context: (Context & { readonly intake: OrganizationIntake }) | undefined;
  #selection: Selection | undefined;
  readonly #budget: SceneBudget;
  /** Retain one open intake and its shared aggregate budget. */
  constructor(intake: OrganizationIntake) {
    const owner = retainInspection.call(intake);
    const summary = inspectionSummary.call(owner) as RecipientQualifiedWorkspaceSummary;
    const source = Object.freeze({ kind: "recipient-qualified-inspection" as const,
      workspaceId: sourceWorkspaceId.call(summary) as string,
      principalUniverseDigest: sourcePrincipalUniverseDigest.call(summary) as string,
      semanticIrDigest: sourceSemanticIrDigest.call(summary) as string,
      admissionEvidenceDigest: sourceAdmissionEvidenceDigest.call(summary) as string });
    this.#budget = captureOrganizationSceneBudget(intake);
    this.#context = { owner: captureOrganizationSceneInspection(intake), source,
      intake, budget: this.#budget };
  }
  /** Replace the current topology and exact assembly. Older completions become stale. */
  select(topology: OrganizationTopology, assembly: OrganizationGeometry): void {
    const context = this.#context;
    if (!context) throw new OrganizationSceneError("closed");
    if (!ownsGeometry.call(topology, context.intake, assembly)) throw new OrganizationSceneError("foreign-assembly");
    const execution = executionInput.call(assembly.plan);
    if (!executionBelongsTo.call(execution, assembly.plan)) throw new OrganizationSceneError("foreign-assembly");
    chargeSceneWork(this.#budget, 1 + assembly.routes.length + assembly.bundles.length);
    this.#selection = { owner: context.owner, source: context.source,
      budget: context.budget,
      topology, assembly, execution };
  }
  /** Run a full layout and publish only if the selected assembly is still current. */
  async layout(executor: ElkExecutor, signal?: AbortSignal): Promise<OrganizationSceneLayout> {
    const selection = this.#selected();
    chargeSceneWork(this.#budget, 1);
    const result = await execute.call(executor, selection.execution, signal);
    if (result.kind === "failed" && result.reason === "termination") return result;
    if (this.#selection !== selection) return { kind: "stopped", reason: "superseded" };
    if (result.kind !== "completed") return result;
    if (signal?.aborted) return { kind: "stopped", reason: "cancelled" };
    return { kind: "completed", scene: this.#publish(selection, result.geometry, Object.freeze({ kind: "full" }),
      Object.freeze({ kind: "full", input: selection.execution, geometry: result.geometry })) };
  }
  /**
   * Estimate route contacts for one scene selected by this compiler.
   * A pre-cancelled valid request stops before constructing contact segments.
   */
  async contacts(scene: OrganizationScene, executor: ContactExecutor, routeOrder: OrganizationRouteOrder,
    limits: ContactLimits, signal?: AbortSignal): Promise<OrganizationSceneContacts> {
    const selection = this.#selected(), before = retained(scene);
    if (before.execution !== selection.execution) throw new OrganizationSceneError("foreign-scene");
    // Custody is checked first. A pre-cancelled valid request must not construct
    // the potentially large segment, frame and depth projection.
    if (signal?.aborted) return { kind: "stopped", reason: "cancelled" };
    const ordered = retainedRouteOrder(routeOrder);
    if (ordered.scene !== scene) throw new OrganizationSceneError("foreign-route-order");
    chargeSceneWork(this.#budget, 1 + ordered.routes.length);
    const input = contactInput.call(selection.assembly.plan, before.geometry,
      ordered.routes.map(item => item.route.id), limits);
    const result = await analyze.call(executor, input, signal);
    if (result.kind === "failed" && result.reason === "termination") return result;
    if (this.#selection !== selection) return { kind: "stopped", reason: "superseded" };
    if (result.kind !== "completed") return result;
    if (signal?.aborted) return { kind: "stopped", reason: "cancelled" };
    const projection = contactOutputProjection(result.analysis);
    chargeRetainedSceneBytes(this.#budget, retainedSceneValueBytes(projection));
    const encodedOutputBytes = chargeEncodedSceneOutput(this.#budget, projection);
    return Object.freeze({ kind: "completed", scene, analysis: result.analysis, encodedOutputBytes });
  }
  /** Prepare exact whole-scene reuse without starting a worker. */
  prepareReuse(prior: OrganizationScene,
    comparison: RecipientQualifiedWorkspaceComparisonInspection,
    choices: OrganizationReuseChoices): OrganizationSceneReuse {
    const current = this.#selected(), before = retained(prior);
    this.#matchComparison(before, current, comparison);
    const preparation = prepareReuse.call(current.topology, before.topology, before.assembly,
      current.assembly, before.geometry, comparison, choices);
    return this.#wrap(current, before, comparison, preparation);
  }
  /** Prepare bounded regional reuse from exact predecessor and current root appearances. */
  prepareRegion(prior: OrganizationScene,
    comparison: RecipientQualifiedWorkspaceComparisonInspection,
    roots: { readonly prior: EndpointAppearance; readonly current: EndpointAppearance },
    choices: OrganizationReuseChoices): OrganizationSceneReuse {
    const current = this.#selected(), before = retained(prior);
    this.#matchComparison(before, current, comparison);
    const preparation = prepareRegion.call(current.topology, before.topology, before.assembly,
      current.assembly, before.geometry, comparison, roots, choices);
    return this.#wrap(current, before, comparison, preparation);
  }
  /** Release this compiler's intake and selection. Issued scenes remain valid. */
  close(): void { this.#selection = undefined; this.#context = undefined; }

  #selected(): Selection {
    if (!this.#context) throw new OrganizationSceneError("closed");
    if (!this.#selection) throw new OrganizationSceneError("unselected");
    return this.#selection;
  }
  #publish(selection: Selection, geometry: GeometryResult, correspondence: OrganizationSceneCorrespondence,
    performed: OrganizationSceneExecution,
    priorOwner?: RecipientQualifiedWorkspaceInspection): OrganizationScene {
    if (!hasAccepted.call(selection.assembly.plan, geometry)) throw new OrganizationSceneError("foreign-result");
    // Exact plan acceptance already proves route/port membership. Index only this
    // bounded result: O(P + R) work/storage, with at most two associations per route.
    // Shared ports retain each relation and selected end; they do not merge meaning.
    const ports = new Map(geometry.ports.map(port => [port.id, port]));
    const routes = new Map(geometry.routes.map(route => [route.id, route]));
    const declaredGates: OrganizationDeclaredGate[] = [];
    const boundaries: OrganizationBoundaryAssociation[] = [];
    for (const binding of selection.assembly.routes) {
      const route = routes.get(binding.id)!;
      if (binding.witness.kind === "relation"
        && binding.witness.locality === OrganizationRelationLocality.NativeBoundary) {
        for (const end of ["source", "target"] as const) {
          const appearance = binding.selection[end];
          if (appearance.endpoint.kind !== "native-resource") continue;
          const port = ports.get(end === "source" ? route.sourcePort : route.targetPort)!;
          const gate = Object.freeze({ binding, end, appearance, port, route });
          declaredGates.push(gate); boundaries.push(Object.freeze({ kind: "native-boundary", ...gate }));
        }
      } else if (binding.witness.kind === "service") {
        const entry = topologyEntry.call(selection.topology,
          binding.witness.occurrence) as RecipientQualifiedOrganizationEntry;
        const row = entryRow.call(entry);
        if (row.kind !== "service") throw new OrganizationSceneError("foreign-result");
        boundaries.push(Object.freeze({ kind: "service-boundary", binding, route,
          source: Object.freeze({ appearance: binding.selection.source, port: ports.get(route.sourcePort)! }),
          target: Object.freeze({ appearance: binding.selection.target, port: ports.get(route.targetPort)! }) }));
        const frontiers = [
          ["provider-completion", row.providerCompletion], ["provider-acceptance", row.providerAcceptance],
          ["consumer-acceptance", row.consumerAcceptance], ["native-read-back", row.nativeReadBack],
          ["pale-settlement", row.paleSettlement],
        ] as const;
        for (const [frontier, requirement] of frontiers) {
          boundaries.push(Object.freeze({ kind: "acceptance-frontier", frontier, binding, route,
            requirement: Object.freeze({ owner: requirement.owner, evidenceRef: requirement.evidenceRef }) }));
        }
      }
    }
    const publication = Object.freeze({ source: selection.source, lens: organizationStructuralLens,
      coverage: Object.freeze({ enumeration: selection.topology.coverage.enumeration,
        selection: selection.topology.coverage.selection }),
      correspondence: correspondence.kind === "full" ? { kind: correspondence.kind }
        : { kind: correspondence.kind, priorSource: correspondence.priorSource,
          ...(correspondence.kind === "regional" ? { regionId: correspondence.regionId } : {}) },
      algorithm: selection.execution.algorithm, geometry,
      boundaries: boundaries.map(item => item.kind === "acceptance-frontier"
        ? { kind: item.kind, frontier: item.frontier, routeId: item.route.id, requirement: item.requirement }
        : item.kind === "service-boundary"
          ? { kind: item.kind, routeId: item.route.id, sourcePort: item.source.port.id, targetPort: item.target.port.id }
          : { kind: item.kind, routeId: item.route.id, end: item.end, portId: item.port.id }),
    });
    chargeSceneWork(this.#budget, 1 + geometry.nodes.length + geometry.ports.length
      + geometry.routes.length + geometry.bundles.length + boundaries.length);
    chargeRetainedSceneBytes(this.#budget, retainedSceneValueBytes(publication));
    const encodedOutputBytes = chargeEncodedSceneOutput(this.#budget, publication);
    return issue({ ...selection, geometry, declaredGates: Object.freeze(declaredGates),
      boundaries: Object.freeze(boundaries), correspondence, performed, encodedOutputBytes,
      ...(priorOwner ? { priorOwner } : {}) });
  }
  #wrap(current: Selection, before: Context,
    comparison: RecipientQualifiedWorkspaceComparisonInspection,
    preparation: OrganizationReuse): OrganizationSceneReuse {
    let closed = false;
    let terminal: OrganizationSceneReuseStep | undefined;
    const valid = (): boolean => !closed && this.#selection === current;
    return Object.freeze({
      step: (): OrganizationSceneReuseStep => {
        if (!valid()) { preparation.close(); return { kind: "closed" }; }
        if (terminal) return terminal;
        const result = preparation.step();
        if (result.kind === "reused") {
          const correspondence = Object.freeze({ kind: "reused" as const, priorSource: before.source, comparison, evidence: result.evidence });
          terminal = Object.freeze({ kind: "reused", scene: this.#publish(current, result.geometry, correspondence,
            Object.freeze({ kind: "reused" }), before.owner) });
          return terminal;
        }
        if (result.kind === "region-ready") {
          const localInput = executionInput.call(result.plan);
          const correspondence = Object.freeze({ kind: "regional" as const, priorSource: before.source,
            comparison, evidence: result.evidence, regionId: result.regionId });
          terminal = Object.freeze({ kind: "region-ready", regionId: result.regionId,
            layout: async (executor: ElkExecutor, signal?: AbortSignal): Promise<OrganizationSceneLayout> => {
              if (!valid()) return { kind: "stopped", reason: "superseded" };
              const local = await execute.call(executor, localInput, signal);
              if (local.kind === "failed" && local.reason === "termination") return local;
              if (!valid()) return { kind: "stopped", reason: "superseded" };
              if (local.kind !== "completed") return local;
              if (signal?.aborted) return { kind: "stopped", reason: "cancelled" };
              const composed = result.complete(local.geometry);
              if (composed.kind === "region-escalation") return composed;
              return { kind: "completed", scene: this.#publish(current, composed.geometry, correspondence,
                Object.freeze({ kind: "regional", input: localInput, geometry: local.geometry }), before.owner) };
            } });
          return terminal;
        }
        return result;
      },
      close: (): void => { closed = true; terminal = undefined; preparation.close(); },
    });
  }

  #matchComparison(prior: Context, current: Context,
    comparison: RecipientQualifiedWorkspaceComparisonInspection): void {
    if (!(comparesExactInspections.call(comparison, prior.owner, current.owner) as boolean)) {
      throw new OrganizationSceneError("comparison-mismatch");
    }
    const pair = comparisonSummaries.call(comparison) as RecipientQualifiedWorkspacePairSummary;
    const before = priorSummary.call(pair) as RecipientQualifiedWorkspaceSummary;
    const after = currentSummary.call(pair) as RecipientQualifiedWorkspaceSummary;
    const matches = (summary: RecipientQualifiedWorkspaceSummary,
      source: OrganizationSceneSource): boolean => sourceWorkspaceId.call(summary) === source.workspaceId
        && sourcePrincipalUniverseDigest.call(summary) === source.principalUniverseDigest
        && sourceSemanticIrDigest.call(summary) === source.semanticIrDigest
        && sourceAdmissionEvidenceDigest.call(summary) === source.admissionEvidenceDigest;
    if (!matches(before, prior.source) || !matches(after, current.source)) {
      throw new OrganizationSceneError("comparison-mismatch");
    }
  }
}
