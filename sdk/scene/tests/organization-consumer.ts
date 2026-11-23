import { Pale, OrganizationRelationLocality, type EffectiveProfileCompilation,
  type EffectiveProfileCoordinates, type ObservationCursor, type ObservationSnapshot,
  type PortableWorkspaceCompilation,
  type OrganizationCursor, type OrganizationNode } from "@rtepl/pale";
import { EffectiveProfileSceneAssociation, OrganizationIntake, OrganizationRouteOrder,
  OrganizationSceneGeneration,
  OrganizationSceneCoverage,
  OrganizationTopology, OrganizationSceneCompiler,
  associateEffectiveProfileObservation,
  type OrganizationScene, type OrganizationRouteWitness, type SceneOccurrence,
  type OrganizationDeclaredGate, type OrganizationOrderedRoute, type OrganizationQueryStep,
  type OrganizationBoundaryAssociation,
  type OrganizationIntakeCharges, type OrganizationIntakeResource, type OrganizationResourceStoppedStep,
  type OrganizationStoppedStep, type OrganizationStep,
  type OrganizationStructuralLens, type OrganizationEnumerationCoverage,
  type OrganizationSelectionCoverage, type EffectiveProfileSceneAssociationResult } from "@rtepl/pale-scene/organization";
import { ElkExecutor, SceneBudget, type SceneBudgetCharges } from "@rtepl/pale-scene";
import { readFileSync } from "node:fs";
import { join } from "node:path";

function cancellationAtCompiler(): AbortSignal {
  const base = new AbortController().signal;
  let reads = 0;
  return new Proxy(base, { get(target, property) {
    if (property === "aborted") return ++reads >= 4;
    const value: unknown = Reflect.get(target, property, target);
    return typeof value === "function" ? value.bind(target) : value;
  } });
}

function associationTypes(scene: OrganizationScene, snapshot: ObservationSnapshot,
  profile: EffectiveProfileCompilation): void {
  const result: EffectiveProfileSceneAssociationResult =
    associateEffectiveProfileObservation(scene, snapshot, profile);
  if (result.kind === "associated") {
    const coordinates: EffectiveProfileCoordinates = result.association.profileCoordinates;
    const cursor: ObservationCursor = result.association.observationCursor;
    result.association.belongsTo(scene, snapshot);
    void coordinates; void cursor;
    // @ts-expect-error Association result branches are read-only.
    result.kind = "refused";
  }
  void (() => {
    // @ts-expect-error Scene associations can be issued only by the package.
    new EffectiveProfileSceneAssociation();
  });
}
void associationTypes;

async function main(): Promise<void> {
  const fixture = process.argv[2];
  if (!fixture) throw new Error("Supply the SDK organization fixture");
  const files = ["pale-workspace.toml", "organization.toml", "responsibility.toml", "resolution.lock.toml", "universe.lock.toml"];
  const owner: PortableWorkspaceCompilation = new Pale().compilePortableWorkspace({ rootManifest: files[0]!,
    documents: files.map(label => ({ label, bytes: readFileSync(join(fixture, label)) })) });
  const sceneBudget = new SceneBudget({ workUnits: 1_000_000, retainedSceneBytes: 128 * 1024 * 1024,
    encodedOutputBytes: 32 * 1024 * 1024, deadlineMs: 60_000 });
  const intake = new OrganizationIntake(owner, { inspected: 2, returned: 2, copiedStringBytes: 65536 },
    { queries: 2000, recordVisits: 5000, sourceVisits: 5000, records: 1000,
      occurrences: 2000, sourceOccurrences: 5000, copiedStringBytes: 32 * 1024 * 1024 }, sceneBudget);
  const executor = new ElkExecutor({ requestBytes: 1000000, resultBytes: 1000000,
    deadlineMs: 10000, totalRequestBytes: 2000000, totalResultBytes: 2000000,
    totalActiveMs: 20000, oldHeapMb: 128, youngHeapMb: 32 });
  try {
    const initialCharges: OrganizationIntakeCharges = intake.charges;
    const resource: OrganizationIntakeResource = "sourceOccurrences";
    if (initialCharges[resource] !== 0) throw new Error("New intake already charged source occurrences");
    void (() => {
      // @ts-expect-error Charges are read-only observations.
      initialCharges.queries = 1;
      // @ts-expect-error Resource names form a closed union.
      const invalidResource: OrganizationIntakeResource = "memory";
      void invalidResource;
    });
    const stoppedController = new AbortController(); stoppedController.abort();
    const stopped: OrganizationStep = intake.next(stoppedController.signal);
    if (stopped.kind !== "stopped" || stopped.reason !== "cancelled") {
      throw new Error("Cancelled intake did not stop before inspection");
    }
    const typedStop: OrganizationStoppedStep = stopped;
    if (typedStop.reason !== "cancelled") throw new Error("Typed cancellation reason changed");
    const occurrences: SceneOccurrence[] = [];
    let exhausted = false;
    for (let count = 0; count < 2000; count++) {
      const step = intake.next();
      if (step.kind === "enumeration-exhausted") { exhausted = true; break; }
      if (step.kind !== "page") throw new Error(`Inspection did not progress: ${step.kind}`);
      occurrences.push(...step.occurrences);
    }
    if (!exhausted) throw new Error("Fixture enumeration did not exhaust within its budget");
    const families = new Set(occurrences.map(item => intake.entry(item).row.kind));
    if (families.size !== 18) throw new Error("Fixture family coverage changed");
    const selectedEndpoint: OrganizationNode = { kind: "role", role: { namespace: "alpha", roleId: "accountable-a" } };
    const endpointStep: OrganizationQueryStep = intake.endpoint(selectedEndpoint);
    if (endpointStep.kind !== "page") throw new Error("Endpoint query did not return its SDK page");
    const cursor: OrganizationCursor | null = endpointStep.page.continuation();
    if (cursor !== null && intake.endpoint(selectedEndpoint, cursor).kind !== "page") throw new Error("Endpoint cursor did not continue");
    if (!endpointStep.occurrences.every(item => item.inclusion === "endpoint-query")) throw new Error("Endpoint inclusion reason lost");
    const stoppedEndpoint: OrganizationQueryStep = intake.endpoint(selectedEndpoint, undefined, stoppedController.signal);
    if (stoppedEndpoint.kind !== "stopped") throw new Error("Cancelled endpoint query did not stop");
    const repeatLimited = new OrganizationIntake(owner,
      { inspected: 2, returned: 1, copiedStringBytes: 65536 },
      { queries: 2, recordVisits: 4, sourceVisits: 1, records: 1,
        occurrences: 1, sourceOccurrences: 1, copiedStringBytes: 2 * 65536 });
    try {
      const first = repeatLimited.endpoint(selectedEndpoint);
      if (first.kind !== "page" || !first.occurrences[0]) throw new Error("Repeat fixture endpoint missing");
      const repeated = repeatLimited.repeat(first.occurrences[0]);
      if (!("kind" in repeated) || repeated.kind !== "resource-stop") {
        throw new Error("Repeat exceeded its occurrence limit without stopping");
      }
      const repeatStop: OrganizationResourceStoppedStep<"occurrences"> = repeated;
      const repeatResource: OrganizationIntakeResource = repeatStop.resource;
      if (repeatResource !== "occurrences") throw new Error("Repeat reported the wrong limiting resource");
      void (() => {
        // @ts-expect-error Repeat can stop only on its occurrence reservation.
        const unrelated: "records" = repeatStop.resource;
        void unrelated;
      });
    } finally { repeatLimited.close(); }
    const incidentStep: OrganizationQueryStep = intake.incidentRelations(selectedEndpoint);
    if (incidentStep.kind !== "page" || incidentStep.page.endpointLookup !== null
      || !incidentStep.occurrences.every(item => item.inclusion === "incident-relation-query")) {
      throw new Error("Incident query was confused with endpoint lookup");
    }
    const serviceStep: OrganizationQueryStep = intake.incidentServices({ namespace: "alpha", scopeId: "scope" });
    if (serviceStep.kind !== "page" || !serviceStep.occurrences.every(item => item.inclusion === "incident-service-query")) {
      throw new Error("Service query lost its independent inclusion reason");
    }
    const serviceCursor: OrganizationCursor | null = serviceStep.page.continuation();
    if (serviceCursor !== null && intake.incidentServices({ namespace: "alpha", scopeId: "scope" }, serviceCursor).kind !== "page") {
      throw new Error("Service continuation failed");
    }
    void (() => {
      // @ts-expect-error A textual label is not a typed endpoint.
      intake.endpoint("alpha");
      // @ts-expect-error A role endpoint cannot impersonate a qualified service scope.
      intake.incidentServices(selectedEndpoint);
      // @ts-expect-error A service continuation is an SDK-owned cursor, not a DTO.
      intake.incidentServices({ namespace: "alpha", scopeId: "scope" }, {});
      // @ts-expect-error Cursor custody cannot be supplied by a position-shaped object.
      intake.incidentRelations(selectedEndpoint, {});
      // @ts-expect-error Returned occurrence arrays are read-only.
      endpointStep.occurrences.push(endpointStep.occurrences[0]!);
      // @ts-expect-error Inclusion reasons are immutable.
      endpointStep.occurrences[0]!.inclusion = "enumerated-record";
    });
    const topology = new OrganizationTopology(intake, occurrences,
      { records: 1000, appearances: 2000, routeAppearances: 3000, identityCodeUnits: 1000000 });
    const capturedCoverage: OrganizationSceneCoverage = topology.coverage;
    const enumerationCoverage: OrganizationEnumerationCoverage = capturedCoverage.enumeration;
    const selectionCoverage: OrganizationSelectionCoverage = capturedCoverage.selection;
    if (enumerationCoverage.kind !== "exhausted"
      || selectionCoverage.kind !== "all-enumerated-occurrences") {
      throw new Error("Complete fixture coverage was not retained");
    }
    void (() => {
      // @ts-expect-error Coverage is issued only from one exact intake frontier.
      new OrganizationSceneCoverage();
      // @ts-expect-error Enumeration coverage is immutable.
      enumerationCoverage.kind = "incomplete";
      // @ts-expect-error Selection coverage is immutable.
      selectionCoverage.kind = "subset";
    });
    // This is an explicit fixture choice, not a production ID rematcher.
    const relation = topology.relations.find(item => {
      const row = topology.entry(item).row;
      return row.kind === "relation" && row.relationId === "relation.collaboration";
    });
    if (!relation) throw new Error("Fixture collaboration relation missing");
    const endpoints = topology.endpoints(relation);
    if (endpoints.from.kind !== "included" || endpoints.to.kind !== "included") throw new Error("Fixture endpoints unavailable");
    const source = endpoints.from.appearances[0], target = endpoints.to.appearances[0];
    if (!source || !target) throw new Error("Fixture endpoint appearance missing");
    void (() => {
      // @ts-expect-error The scene-owned endpoint discriminant is frozen.
      source.endpoint.kind = "role";
      if (source.endpoint.kind === "role") {
        // @ts-expect-error Qualified scene reference fields are frozen too.
        source.endpoint.role.roleId = "retargeted";
        // @ts-expect-error A qualified reference cannot be replaced.
        source.endpoint.role = { namespace: "alpha", roleId: "retargeted" };
      }
      const selected = topology.endpoints(relation).from;
      if (selected.endpoint.kind === "role") {
        // @ts-expect-error Both included and not-included endpoint results are frozen copies.
        selected.endpoint.role.namespace = "retargeted";
      }
      // The input SDK DTO remains mutable and keeps ordinary union narrowing.
      if (selectedEndpoint.kind === "role") selectedEndpoint.role.roleId = "caller-owned-input";
    });
    const service = topology.relations.find(item => {
      const row = topology.entry(item).row;
      return row.kind === "service" && row.contract.namespace === "alpha";
    });
    if (!service) throw new Error("Fixture directional service missing");
    const serviceEndpoints = topology.endpoints(service);
    if (serviceEndpoints.from.kind !== "included" || serviceEndpoints.to.kind !== "included") throw new Error("Service scopes unavailable");
    const provider = serviceEndpoints.from.appearances[0], consumer = serviceEndpoints.to.appearances[0];
    if (!provider || !consumer) throw new Error("Service scope appearance missing");
    void (() => {
      if (provider.endpoint.kind === "scope") {
        // @ts-expect-error Directional service appearances use the same frozen reference type.
        provider.endpoint.scope.scopeId = "retargeted";
      }
    });
    const assembly = topology.geometry([source, target, provider, consumer].map(appearance => ({ appearance, width: 80, height: 40 })),
      [{ relation, source, target }, { relation: service, source: provider, target: consumer }],
      { nodes: 4, ports: 4, relations: 2, points: 1000, depth: 1, coordinateMagnitude: 10000 });
    const compiler = new OrganizationSceneCompiler(intake);
    compiler.select(topology, assembly);
    const cancelled = await compiler.layout(executor, cancellationAtCompiler());
    if (cancelled.kind !== "stopped" || cancelled.reason !== "cancelled") {
      throw new Error("Late cancellation published a scene");
    }
    const result = await compiler.layout(executor);
    compiler.close();
    if (result.kind !== "completed") throw new Error("Scene did not complete");
    const scene: OrganizationScene = result.scene;
    const generation: OrganizationSceneGeneration = scene.generation;
    const budgetCharges: SceneBudgetCharges = sceneBudget.charges;
    if (!generation.belongsTo(scene) || generation.source !== scene.source
      || generation.constraints !== scene.constraints || generation.coverage !== scene.coverage
      || generation.geometry !== scene.geometry
      || generation.correspondence !== scene.correspondence || generation.encodedOutputBytes === null
      || generation.encodedOutputBytes <= 0
      || budgetCharges.encodedOutputBytes !== generation.encodedOutputBytes) {
      throw new Error("Scene generation lost its exact accepted inputs");
    }
    const boundaries: readonly OrganizationBoundaryAssociation[] = scene.boundaries;
    void (() => {
      // @ts-expect-error Generations are issued only with an accepted scene.
      new OrganizationSceneGeneration();
      // @ts-expect-error Boundary associations are immutable.
      boundaries.push(boundaries[0]!);
      // @ts-expect-error Budget charges are observations, not mutable counters.
      budgetCharges.workUnits = 0;
    });
    if (scene.coverage !== capturedCoverage) throw new Error("Layout replaced topology coverage");
    const lens: OrganizationStructuralLens = scene.lens;
    if (lens.kind !== "organization-structural" || lens.subject !== "scene-appearance"
      || lens.grouping !== "explicit-presentation-selection" || lens.proximity !== "layout-constraints-only"
      || lens.routes !== "selected-sdk-witnesses") throw new Error("Scene lens qualification changed");
    void (() => {
      // @ts-expect-error Lens meaning is immutable and cannot be redefined by a renderer.
      lens.proximity = "semantic-similarity";
    });
    const gates: readonly OrganizationDeclaredGate[] = scene.declaredGates;
    if (gates.length !== 0) throw new Error("Internal and service routes acquired native gates");
    void (() => {
      // @ts-expect-error Declared gate associations are retained, not mutable selections.
      gates.push(gates[0]!);
    });
    const routeOrder: OrganizationRouteOrder = scene.orderRoutes(scene.geometry.routes.map(route => route.id));
    const ordered: readonly OrganizationOrderedRoute[] = routeOrder.routes;
    if (!routeOrder.belongsTo(scene) || routeOrder.layering !== "back-to-front"
      || ordered.length !== scene.geometry.routes.length
      || ordered.some(item => item.binding.id !== item.route.id)) throw new Error("Route order lost scene custody");
    void (() => {
      // @ts-expect-error Route order is issued only by an exact scene.
      new OrganizationRouteOrder();
      // @ts-expect-error Ordered routes cannot be replaced by a renderer.
      ordered.push(ordered[0]!);
    });
    if (scene.geometry.routes[0]?.id !== scene.assembly.routes[0]?.id) throw new Error("Scene route did not preserve its binding");
    const relationWitness = scene.assembly.routes[0]!.witness;
    if (relationWitness.kind !== "relation" || relationWitness.locality !== OrganizationRelationLocality.Internal) {
      throw new Error("Explicit relation lost its SDK-owned locality");
    }
    const serviceRoute = scene.assembly.routes[1];
    if (!serviceRoute || scene.geometry.routes[1]?.id !== serviceRoute.id) throw new Error("Service route binding lost");
    const witness: OrganizationRouteWitness = serviceRoute.witness;
    if (witness.kind !== "service" || witness.occurrence !== service) throw new Error("Service was reclassified or substituted");
    // @ts-expect-error A service does not carry the explicit-relation locality law.
    void witness.locality;
    const serviceEntry = topology.entry(witness.occurrence), serviceRow = serviceEntry.row;
    if (!serviceEntry.handle.belongsTo(owner) || serviceRow.kind !== "service") throw new Error("Service source custody lost");
    const sourceStep = intake.sources(witness.occurrence);
    if (sourceStep.kind !== "page" || !sourceStep.page.record.sameRecord(serviceEntry.handle)
      || !sourceStep.page.record.belongsTo(owner) || sourceStep.mapping.kind !== "named") {
      throw new Error("Service source page lost its exact record association");
    }
    if (serviceRow.contract.providerScope.namespace !== "alpha" || serviceRow.contract.consumerScope.namespace !== "beta"
      || new Set([serviceRow.providerCompletion, serviceRow.providerAcceptance, serviceRow.consumerAcceptance,
        serviceRow.nativeReadBack, serviceRow.paleSettlement].map(value => value.evidenceRef)).size !== 5) {
      throw new Error("Service direction or independent requirements lost");
    }
    if (scene.source.semanticIrDigest !== owner.semanticIrDigest || scene.algorithm.version !== "0.12.0"
      || scene.correspondence.kind !== "full") throw new Error("Scene source or algorithm association was lost");
    if (!topology.entry(relation).handle.belongsTo(owner)) throw new Error("Record lost its compilation");
    console.log(JSON.stringify({ organization: true, families: families.size, routes: scene.geometry.routes.length,
      coverage: { enumeration: scene.coverage.enumeration.kind, selection: scene.coverage.selection.kind } }));
  } finally { intake.close(); await executor.close(); }
}
void main();
