import { GeometryPlan, ElkExecutor, ContactExecutor, type GeometryResult, type ContactAnalysis } from "@rtepl/pale-scene";

async function main(): Promise<void> {
  const plan = new GeometryPlan({ nodes: [
    { id: "a", width: 40, height: 30, ports: [{ id: "out", side: "EAST" }] },
    { id: "b", width: 40, height: 30, ports: [{ id: "in", side: "WEST" }] },
  ], relations: [{ id: "edge", sourcePort: "out", targetPort: "in" }] },
  { nodes: 2, ports: 2, relations: 1, points: 100, depth: 1, coordinateMagnitude: 10000 });
  const executor = new ElkExecutor({ requestBytes: 100000, resultBytes: 100000,
    deadlineMs: 10000, totalRequestBytes: 200000, totalResultBytes: 200000,
    totalActiveMs: 20000, oldHeapMb: 128, youngHeapMb: 32 });
  const contacts = new ContactExecutor({ requestBytes: 100000, resultBytes: 100000,
    deadlineMs: 10000, totalRequestBytes: 200000, totalResultBytes: 200000,
    totalActiveMs: 20000, oldHeapMb: 128, youngHeapMb: 32 });
  try {
    const result = await executor.run(plan);
    if (result.kind !== "completed") throw new Error(`Layout did not complete: ${result.kind}`);
    const geometry: GeometryResult = result.geometry;
    if (geometry.routes.length !== 1 || geometry.routes[0]?.sourcePort !== "out"
      || geometry.routes[0]?.targetPort !== "in") throw new Error("Route identity was lost");
    if (Object.keys(require.cache).some(path => path.endsWith(".node"))) throw new Error("Geometry loaded a native addon");
    const input = plan.contactInput(geometry, ["edge"], { segments: 100, pairs: 5050, contacts: 500,
      maximumSeparation: 1e-5, maximumControlDrift: 1e-8 });
    const annotated = await contacts.run(input);
    if (process.argv[2] === "ambient-dom") {
      if (annotated.kind !== "failed" || annotated.reason !== "load") throw new Error("Ambient DOM was not confined");
    } else {
      if (annotated.kind !== "completed") throw new Error(`Contacts failed: ${annotated.kind}`);
      const analysis: ContactAnalysis = annotated.analysis;
      if (analysis.geometry !== geometry || analysis.input !== input || analysis.coverage !== "not-established"
        || analysis.certifiedErrorBound !== null) throw new Error("Contact custody/model was lost");
    }
    const layoutCharges = executor.charges, contactCharges = contacts.charges;
    if (!Object.isFrozen(layoutCharges) || !Object.isFrozen(contactCharges)
      || layoutCharges.requestBytes < 1 || layoutCharges.resultBytes < 1 || layoutCharges.activeMs < 0
      || contactCharges.requestBytes < 1 || contactCharges.activeMs < 0) {
      throw new Error("Executor aggregate charges were not retained at the public entry");
    }
    void (() => {
      // @ts-expect-error Aggregate charges are observations, not a replenishable caller budget.
      layoutCharges.requestBytes = 0;
    });
    console.log(JSON.stringify({ geometryOnly: true, routes: geometry.routes.length, nativeAddonLoaded: false,
      contacts: annotated.kind, aggregateCharges: { layout: layoutCharges, contacts: contactCharges },
      ambientDomConfined: process.argv[2] === "ambient-dom" }));
  } finally { await contacts.close(); await executor.close(); }
}
void main();
