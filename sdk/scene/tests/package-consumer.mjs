// Exercise the packed public entries, without source-checkout imports.
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { copyFile, mkdir, mkdtemp, readFile, readdir, rm, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { execFileSync, spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

assert.equal(process.argv.length, 4, "Supply the exact native SDK archive and its organization fixture directory");
const sdkArchive = resolve(process.argv[2]), fixture = resolve(process.argv[3]);
const root = fileURLToPath(new URL("../", import.meta.url));
const manifest = JSON.parse(await readFile(join(root, "package.json")));
const temporary = await mkdtemp(join(tmpdir(), "pale-scene-consumer-"));
const sha = bytes => createHash("sha256").update(bytes).digest("hex");
const npmCommands = [];
const npm = (args, cwd) => {
  const result = spawnSync("npm", [...args, "--offline"], { cwd, encoding: "utf8",
    detached: true, timeout: 120000, killSignal: "SIGKILL", maxBuffer: 8 * 1024 * 1024,
    stdio: ["ignore", "pipe", "pipe"] });
  // npm runs without lifecycle scripts. Still stop its exact owned group if
  // timeout/failure leaves descendants; never inspect or signal unrelated work.
  const cleanup = { group: result.pid ?? null, beforeAlive: false, afterAlive: false, killed: false, error: null };
  const alive = () => {
    try { process.kill(-result.pid, 0); return true; }
    catch (error) { if (error.code !== "ESRCH") throw error; return false; }
  };
  try { if (result.pid > 1) {
    cleanup.beforeAlive = alive();
    if (cleanup.beforeAlive) {
      try { process.kill(-result.pid, "SIGKILL"); cleanup.killed = true; }
      catch (error) { if (error.code !== "ESRCH") throw error; }
      for (let count = 0; count < 40 && alive(); count++) Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 50);
    }
    cleanup.afterAlive = alive();
  } } catch (error) { cleanup.error = error.stack; }
  const command = { args, cwd, status: result.status, signal: result.signal,
    error: result.error?.message ?? null, cleanup };
  npmCommands.push(command);
  if (result.error || result.status !== 0 || cleanup.error || cleanup.afterAlive || cleanup.beforeAlive) {
    console.error(JSON.stringify(command, null, 2));
  }
  if (result.stderr) process.stderr.write(result.stderr);
  assert.ifError(result.error);
  assert.equal(result.signal, null, "npm terminated");
  assert.equal(result.status, 0, "npm failed");
  assert.equal(cleanup.error, null, "npm process cleanup inspection failed");
  assert.equal(cleanup.afterAlive, false, "npm's process group remains active");
  assert.equal(cleanup.beforeAlive, false, "npm's process group outlived the command");
  return result.stdout;
};
const node = (args, options = {}) => execFileSync(process.execPath, args, {
  cwd: temporary, encoding: "utf8", timeout: 60000, maxBuffer: 8 * 1024 * 1024, ...options,
});
let passed = false;
try {
  const [packed] = JSON.parse(npm(["pack", "--json", "--ignore-scripts", "--pack-destination", temporary], root));
  assert(packed);
  const localArchive = (name, expectedVersion, modules = join(root, "node_modules")) => {
    const source = join(modules, ...name.split("/"));
    const [dependency] = JSON.parse(npm(["pack", "--json", "--ignore-scripts", "--pack-destination", temporary], source));
    assert.equal(dependency.name, name); assert.equal(dependency.version, expectedVersion);
    return join(temporary, dependency.filename);
  };
  // Package qualification is self-contained. It must not depend on registry
  // metadata being present in a particular developer's npm cache.
  const typescriptArchive = localArchive("typescript", manifest.devDependencies.typescript);
  const nodeTypesArchive = localArchive("@types/node", manifest.devDependencies["@types/node"]);
  const undiciArchive = localArchive("undici-types", manifest.devDependencies["undici-types"]);
  const elkArchive = localArchive("elkjs", manifest.dependencies.elkjs);
  const paperArchive = localArchive("paper", manifest.dependencies.paper);
  const sdkModules = resolve(root, "../js/node_modules");
  const sdkSourceManifest = JSON.parse(await readFile(resolve(root, "../js/package.json")));
  const sdkUndiciManifest = JSON.parse(await readFile(join(sdkModules, "undici-types/package.json")));
  const sdkNodeTypes = localArchive("@types/node", sdkSourceManifest.dependencies["@types/node"], sdkModules);
  const sdkUndici = localArchive("undici-types", sdkUndiciManifest.version, sdkModules);
  const modules = ["index", "organization", "geometry", "routeBundles", "elkExecutor", "elkWorker", "sceneWorker",
    "contactAnalysis", "contactExecutor", "paperWorker", "organizationIntake", "organizationTopology", "organizationScene",
    "sceneBudget", "change", "changeIntake"];
  const expected = ["package.json", "LICENSE", "README.md", "tsconfig.json", "tools/build.mjs", "tools/declaration-names.mjs", "tools/qualify-performance.mjs", "third-party/elkjs/LICENSE.md", "third-party/paper/LICENSE.txt",
    ...modules.flatMap(name => [`src/${name}.ts`, `dist/${name}.js`, `dist/${name}.d.ts`])].sort();
  assert.deepEqual(packed.files.map(item => item.path).sort(), expected);
  const archive = join(temporary, packed.filename);
  const geometryRoot = join(temporary, "geometry-only");
  await mkdir(geometryRoot);
  await writeFile(join(geometryRoot, "package.json"), JSON.stringify({ name: "pale-scene-geometry-consumer", private: true, type: "commonjs" }, null, 2) + "\n");
  npm(["install", "--ignore-scripts", "--no-audit", "--no-fund", "--no-package-lock", archive,
    typescriptArchive, nodeTypesArchive, undiciArchive, elkArchive, paperArchive], geometryRoot);
  const geometryRequire = createRequire(join(geometryRoot, "package.json"));
  const packedSceneRoot = dirname(dirname(geometryRequire.resolve("@rtepl/pale-scene")));
  for (const name of ["LICENSE", "third-party/elkjs/LICENSE.md", "third-party/paper/LICENSE.txt"]) {
    assert((await readFile(join(packedSceneRoot, name))).equals(await readFile(join(root, name))),
      `Packed ${name} must retain its source bytes`);
  }
  const packedManifest = JSON.parse(await readFile(join(packedSceneRoot, "package.json")));
  assert.equal(packedManifest.repository, manifest.repository);
  assert.equal(packedManifest.private, true);
  assert.throws(() => geometryRequire.resolve("@rtepl/pale"), { code: "MODULE_NOT_FOUND" });
  await copyFile(join(root, "tests/geometry-consumer.ts"), join(geometryRoot, "geometry-consumer.ts"));
  await copyFile(join(root, "tests/first-use-consumer.ts"), join(geometryRoot, "first-use-consumer.ts"));
  const compilerOptions = { strict: true, noUncheckedIndexedAccess: true, exactOptionalPropertyTypes: true,
    noImplicitReturns: true, noFallthroughCasesInSwitch: true, allowUnreachableCode: false, allowUnusedLabels: false,
    noEmitOnError: true, skipLibCheck: false, target: "ES2022", module: "NodeNext", moduleResolution: "NodeNext", outDir: "compiled" };
  await writeFile(join(geometryRoot, "tsconfig.json"), JSON.stringify({ compilerOptions,
    files: ["geometry-consumer.ts", "first-use-consumer.ts"] }, null, 2) + "\n");
  node([geometryRequire.resolve("typescript/bin/tsc"), "-p", "tsconfig.json"], { cwd: geometryRoot });
  const firstUse = JSON.parse(node([join(geometryRoot, "compiled/first-use-consumer.js")], { cwd: geometryRoot }));
  assert.deepEqual(firstUse, { nodes: 2, routes: 1 });
  const geometry = JSON.parse(node([join(geometryRoot, "compiled/geometry-consumer.js")], { cwd: geometryRoot }));
  assert.equal(geometry.contacts, "completed");
  assert(geometry.aggregateCharges.layout.requestBytes > 0);
  assert(geometry.aggregateCharges.layout.resultBytes > 0);
  assert(geometry.aggregateCharges.contacts.requestBytes > 0);
  const ambient = join(geometryRoot, "node_modules/jsdom"), ambientMarker = join(geometryRoot, "dom-loaded");
  await mkdir(ambient);
  await writeFile(join(ambient, "package.json"), JSON.stringify({ name: "jsdom", version: "0.0.0", main: "index.js" }, null, 2) + "\n");
  await writeFile(join(ambient, "index.js"), `require("node:fs").writeFileSync(${JSON.stringify(ambientMarker)}, "loaded");\nmodule.exports = {};\n`);
  const confined = JSON.parse(node([join(geometryRoot, "compiled/geometry-consumer.js"), "ambient-dom"], { cwd: geometryRoot }));
  assert.equal(confined.ambientDomConfined, true); assert.equal(confined.contacts, "failed");
  assert.equal(confined.aggregateCharges.contacts.resultBytes, 0);
  await assert.rejects(readFile(ambientMarker), { code: "ENOENT" });
  const sdkBytes = await readFile(sdkArchive);
  await writeFile(join(temporary, "sdk.tgz"), sdkBytes);
  await writeFile(join(temporary, "package.json"), JSON.stringify({ name: "pale-scene-headless-consumer", private: true, type: "commonjs" }, null, 2) + "\n");
  npm(["install", "--ignore-scripts", "--no-audit", "--no-fund", "--no-package-lock",
    archive, join(temporary, "sdk.tgz"), typescriptArchive, sdkNodeTypes, sdkUndici, elkArchive, paperArchive], temporary);
  const consumerRequire = createRequire(join(temporary, "package.json"));
  const sceneEntry = consumerRequire.resolve("@rtepl/pale-scene");
  const sceneRequire = createRequire(sceneEntry);
  assert.throws(() => consumerRequire.resolve("@rtepl/pale-scene/dist/elkWorker.js"), { code: "ERR_PACKAGE_PATH_NOT_EXPORTED" });
  assert.equal(sceneRequire.resolve("@rtepl/pale"), consumerRequire.resolve("@rtepl/pale"), "Use one native SDK installation");
  const sdkRoot = dirname(consumerRequire.resolve("@rtepl/pale"));
  const sdkManifest = JSON.parse(await readFile(join(sdkRoot, "package.json")));
  assert.equal(sdkManifest.version, manifest.peerDependencies["@rtepl/pale"]);
  const peerArtifacts = {};
  const addons = (await readdir(sdkRoot)).filter(name => name.endsWith(".node"));
  assert.equal(addons.length, 1, "Supply the selected host's native SDK archive");
  for (const name of ["index.js", "index.d.ts", ...addons]) {
    peerArtifacts[name] = sha(await readFile(join(sdkRoot, name)));
  }
  const elkRoot = dirname(sceneRequire.resolve("elkjs/package.json"));
  assert.equal(JSON.parse(await readFile(join(elkRoot, "package.json"))).version, "0.12.0");
  assert.equal(sha(await readFile(join(elkRoot, "lib/elk.bundled.js"))), "1222e44f953ce7746af23801e723708f8e6f436b8b377a6a5fc7552f34a307b3");
  assert((await readFile(join(elkRoot, "LICENSE.md"))).equals(await readFile(join(root, "third-party/elkjs/LICENSE.md"))));
  const paperRoot = dirname(sceneRequire.resolve("paper/package.json"));
  assert.equal(JSON.parse(await readFile(join(paperRoot, "package.json"))).version, "0.12.18");
  assert((await readFile(join(paperRoot, "LICENSE.txt"))).equals(await readFile(join(root, "third-party/paper/LICENSE.txt"))));
  const fixtureInputs = {};
  for (const name of ["pale-workspace.toml", "organization.toml", "responsibility.toml", "resolution.lock.toml", "universe.lock.toml"]) {
    const bytes = await readFile(join(fixture, name));
    fixtureInputs[name] = sha(bytes);
  }
  for (const name of ["geometry-consumer.ts", "qualified-boundary-consumer.ts"]) {
    await copyFile(join(root, "tests", name), join(temporary, name));
  }
  await writeFile(join(temporary, "tsconfig.json"), JSON.stringify({ compilerOptions,
    files: ["geometry-consumer.ts", "qualified-boundary-consumer.ts"] }, null, 2) + "\n");
  node([consumerRequire.resolve("typescript/bin/tsc"), "-p", "tsconfig.json"]);
  const testNames = ["public-errors.test.cjs", "scene-geometry.test.cjs",
    "scene-worker.test.cjs", "scene-contacts.test.cjs"];
  await mkdir(join(temporary, "tests/fixtures"), { recursive: true });
  for (const name of testNames) await copyFile(join(root, "tests", name), join(temporary, "tests", name));
  await copyFile(join(root, "tests/fixtures/layout-worker.cjs"), join(temporary, "tests/fixtures/layout-worker.cjs"));
  await copyFile(join(root, "tests/fixtures/contact-source-worker.cjs"), join(temporary, "tests/fixtures/contact-source-worker.cjs"));
  const behavior = node(["--test", "--test-isolation=none", "--test-concurrency=1", ...testNames.map(name => join("tests", name))],
    { env: process.env });
  const tests = behavior.split("\n").filter(line => line.startsWith("✔ "));
  assert.equal(tests.length, 94);
  for (const [label, count] of Object.entries({ tests: 94, pass: 94, fail: 0, cancelled: 0, skipped: 0, todo: 0 })) {
    assert(behavior.includes(`ℹ ${label} ${count}\n`), label);
  }
  console.log(JSON.stringify({ firstUse, geometry, confined,
    recipientQualifiedOrganization: {
      status: "blocked",
      dependency: "PaleJS resident acquisition of recipient-qualified inspection handles",
      reason: "A source fixture cannot grant inspection authority.",
    },
    packageSha256: sha(await readFile(archive)), sdkArchiveSha256: sha(sdkBytes), peerArtifacts,
    fixtureInputs, packageFiles: expected, tests, npmCommands }, null, 2));
  passed = true;
} finally {
  if (passed) await rm(temporary, { recursive: true, force: true });
  else console.error(`Failed packed consumer retained at ${temporary}`);
}
