import { Buffer } from "node:buffer";
import { parentPort, workerData } from "node:worker_threads";
import { elkConfiguration } from "./geometry";

/** This entry receives only the geometry JSON prepared by GeometryPlan. */
async function layout(): Promise<void> {
  if (!parentPort) throw new Error("Layout requires a worker thread");
  const request: unknown = workerData;
  if (!request || typeof request !== "object" || !("input" in request) || typeof request.input !== "string"
    || !("resultBytes" in request) || typeof request.resultBytes !== "number"
    || !Number.isSafeInteger(request.resultBytes) || request.resultBytes < 1) throw new Error("Invalid layout message");
  // Packaging pins the dependency; this check rejects a different installed engine.
  const version: unknown = require("elkjs/package.json").version;
  if (version !== elkConfiguration.version) throw new Error("Unsupported layout engine");
  const Elk: unknown = require("elkjs/lib/elk.bundled.js");
  if (typeof Elk !== "function") throw new Error("Invalid layout engine");
  const engine: unknown = Reflect.construct(Elk, [{ algorithms: [elkConfiguration.rootOptions["elk.algorithm"]] }]);
  if (!engine || typeof engine !== "object" || !("layout" in engine) || typeof engine.layout !== "function") throw new Error("Invalid layout engine");
  const result: unknown = await Reflect.apply(engine.layout, engine, [JSON.parse(request.input)]);
  const encoded = JSON.stringify(result);
  parentPort.postMessage(Buffer.byteLength(encoded, "utf8") > request.resultBytes ? "result-bytes" : encoded);
}

void layout().catch(() => { throw new Error("Layout worker failed"); });
