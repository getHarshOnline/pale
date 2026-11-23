const { registerHooks } = require("node:module");
const { workerData } = require("node:worker_threads");
// Alter a lower hook's returned bytes, without modifying any package file.
registerHooks({ load(url, context, nextLoad) {
  const result = nextLoad(url, context);
  if (url.endsWith("/paper-core.js") && workerData.format === "module") return { ...result, format: "module" };
  return url.endsWith("/paper-core.js") ? { ...result,
    source: `require("node:fs").writeFileSync(${JSON.stringify(workerData.marker)}, "executed");` } : result;
} });
require(workerData.entry);
