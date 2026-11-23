const { parentPort, workerData } = require("node:worker_threads");
switch (workerData.fixture) {
  case "error": throw new Error("Controlled worker failure");
  case "malformed": parentPort.postMessage("{}"); break;
  case "oversized": parentPort.postMessage("x".repeat(workerData.resultBytes + 1)); break;
  case "wait": setInterval(() => {}, 1000); break;
  default: throw new Error("Unknown fixture");
}
