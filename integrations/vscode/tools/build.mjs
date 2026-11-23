// Compile the client and copy its qualified private runtime artifact unchanged.
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdir, readFile, readdir, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const source = path.join(root, "src/generated/protocol");
const destination = path.join(root, "dist/generated/protocol");
const roots = [
  "AnalysisPublication",
  "ConfigurationImprovementResponse",
  "EffectiveProfileExplanationResponse",
  "EffectiveProfileResponse",
  "PaleWorkspaceCapabilities",
  "ProfileContextsResponse",
];
const expected = [...roots.map(root => `${root}.ts`), "responses.cjs", "responses.d.cts"];
const expectedBuilt = [
  ...roots.flatMap(root => [`${root}.js`, `${root}.d.ts`, `${root}.d.ts.map`]),
  "responses.cjs",
  "responses.d.cts",
  "generation.json",
];
const manifestBytes = await readFile(path.join(source, "generation.json"));
const manifest = JSON.parse(manifestBytes);
const qualified = new Map([["generation.json", manifestBytes]]);
assert.deepEqual(manifest.selection, {
  direction: "Responses", roots, format: "commonjs",
});
assert.deepEqual(manifest.outputs.map(member => member.path).sort(), [...expected].sort());
assert.deepEqual((await readdir(source)).sort(), [...expected, "generation.json"].sort());
for (const member of manifest.outputs) {
  const bytes = await readFile(path.join(source, member.path));
  assert.equal(createHash("sha256").update(bytes).digest("hex"), member.sha256, member.path);
  qualified.set(member.path, bytes);
}

// The package ships dist wholesale. Reject stale catalog/request artifacts
// before compilation; do not silently delete an unknown output.
let existingBuilt;
try { existingBuilt = await readdir(destination); }
catch (error) {
  if (error.code !== "ENOENT") throw error;
  existingBuilt = [];
}
assert(existingBuilt.every(file => expectedBuilt.includes(file)), "unexpected generated build output inventory");

const require = createRequire(path.join(root, "package.json"));
const result = spawnSync(process.execPath, [require.resolve("typescript/bin/tsc"), "-p", "tsconfig.json"], {
  cwd: root, stdio: "inherit",
});
if (result.error) throw result.error;
if (result.signal) throw new Error(`TypeScript compiler terminated: ${result.signal}`);
if (result.status !== 0) process.exit(result.status ?? 1);
for (const [file, bytes] of qualified) {
  assert((await readFile(path.join(source, file))).equals(bytes), `generated input changed during compilation: ${file}`);
}
await mkdir(destination, { recursive: true });
for (const file of ["responses.cjs", "responses.d.cts", "generation.json"]) {
  const bytes = qualified.get(file);
  await writeFile(path.join(destination, file), bytes);
  assert(bytes.equals(await readFile(path.join(destination, file))), file);
}
assert.deepEqual((await readdir(destination)).sort(), [...expectedBuilt].sort(), "generated build output inventory differs");
