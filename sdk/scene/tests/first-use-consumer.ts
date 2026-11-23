import { ElkExecutor, GeometryPlan } from "@rtepl/pale-scene";

async function main(): Promise<void> {
  const plan = new GeometryPlan({
    nodes: [
      { id: "source", width: 40, height: 30, ports: [{ id: "source-out", side: "EAST" }] },
      { id: "target", width: 40, height: 30, ports: [{ id: "target-in", side: "WEST" }] },
    ],
    relations: [{ id: "flow", sourcePort: "source-out", targetPort: "target-in" }],
  }, { nodes: 2, ports: 2, relations: 1, points: 32, depth: 1, coordinateMagnitude: 10_000 });
  const executor = new ElkExecutor({
    requestBytes: 100_000, resultBytes: 100_000, deadlineMs: 10_000,
    totalRequestBytes: 100_000, totalResultBytes: 100_000, totalActiveMs: 10_000,
    oldHeapMb: 128, youngHeapMb: 32,
  });

  try {
    const result = await executor.run(plan);
    if (result.kind !== "completed") {
      throw new Error(`Layout ${result.kind}: ${result.reason}`);
    }
    console.log(JSON.stringify({ nodes: result.geometry.nodes.length,
      routes: result.geometry.routes.length }));
  } finally {
    await executor.close();
  }
}

void main();
