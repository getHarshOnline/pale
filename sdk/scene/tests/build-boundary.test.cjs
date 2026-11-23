const assert = require("node:assert/strict");
const { test } = require("node:test");
const { appendFile, copyFile, mkdir, mkdtemp, readFile, readdir, rm, symlink, writeFile } = require("node:fs/promises");
const { tmpdir } = require("node:os");
const { join } = require("node:path");
const { spawnSync } = require("node:child_process");
const { pathToFileURL } = require("node:url");

async function workspace(t) {
  const target = await mkdtemp(join(tmpdir(), "pale-scene-build-boundary-"));
  t.after(() => rm(target, { recursive: true, force: true }));
  for (const directory of ["src", "tools"]) await mkdir(join(target, directory));
  const source = join(__dirname, "../src");
  for (const name of await readdir(source)) await copyFile(join(source, name), join(target, "src", name));
  await copyFile(join(__dirname, "../tools/build.mjs"), join(target, "tools/build.mjs"));
  await copyFile(join(__dirname, "../tools/declaration-names.mjs"), join(target, "tools/declaration-names.mjs"));
  await copyFile(join(__dirname, "../package.json"), join(target, "package.json"));
  await copyFile(join(__dirname, "../tsconfig.json"), join(target, "tsconfig.json"));
  return target;
}

test("build rejects stale output before invoking TypeScript without deleting it", async t => {
  const target = await workspace(t);
  await mkdir(join(target, "dist"));
  await writeFile(join(target, "dist/stale.js"), "preserve this stale output\n");
  const result = spawnSync(process.execPath, [join(target, "tools/build.mjs")], { encoding: "utf8", timeout: 10000 });
  assert.ifError(result.error);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Unexpected build output: stale\.js/);
  assert.equal(await readFile(join(target, "dist/stale.js"), "utf8"), "preserve this stale output\n");
  assert.deepEqual(await readdir(join(target, "dist")), ["stale.js"]);
});

test("build rejects a symlink output directory before compilation", async t => {
  const target = await workspace(t);
  await mkdir(join(target, "other"));
  await symlink(join(target, "other"), join(target, "dist"));
  const result = spawnSync(process.execPath, [join(target, "tools/build.mjs")], { encoding: "utf8", timeout: 10000 });
  assert.ifError(result.error);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Build destination must be an ordinary directory/);
  assert.deepEqual(await readdir(join(target, "other")), []);
});

test("build rejects numeric declaration collision counters", async t => {
  const target = await workspace(t);
  await symlink(join(__dirname, "../node_modules"), join(target, "node_modules"), "dir");
  await appendFile(
    join(target, "src/index.ts"),
    "\nexport interface ProfileInspectionContext {}\nexport interface ProfileInspectionContext2 {}\n",
  );
  const result = spawnSync(process.execPath, [join(target, "tools/build.mjs")], {
    encoding: "utf8",
    timeout: 30_000,
  });
  assert.ifError(result.error);
  assert.notEqual(result.status, 0);
  assert.match(
    `${result.stdout}\n${result.stderr}`,
    /numeric collision counter: ProfileInspectionContext2/,
  );
});

test("build rejects undocumented public declarations", async t => {
  const target = await workspace(t);
  await symlink(join(__dirname, "../node_modules"), join(target, "node_modules"), "dir");
  await appendFile(join(target, "src/index.ts"), "\nexport interface UndocumentedPublicType {}\n");
  const result = spawnSync(process.execPath, [join(target, "tools/build.mjs")], {
    encoding: "utf8",
    timeout: 30_000,
  });
  assert.ifError(result.error);
  assert.notEqual(result.status, 0);
  assert.match(
    `${result.stdout}\n${result.stderr}`,
    /index\.d\.ts: exported declaration needs TSDoc/,
  );
});

test("declaration guard preserves meaningful numeric names", async () => {
  const guard = await import(pathToFileURL(join(__dirname, "../tools/declaration-names.mjs")));
  assert.doesNotThrow(() => guard.assertMeaningfulDeclarationNames(
    "export interface Sha256DigestV1 {}\nexport interface Http2PolicyV1 {}\n",
    "semantic-number-fixture.d.ts",
  ));
  assert.throws(() => guard.assertMeaningfulDeclarationNames(
    "export interface AttributableAnswerV1 {}\nexport interface AttributableAnswerV114 {}\n",
    "version-counter-fixture.d.ts",
  ), /meaningless collision name AttributableAnswerV114/);
});

test("generated declarations omit package-internal transport helpers", async () => {
  const declarationFiles = (await readdir(join(__dirname, "../dist")))
    .filter(name => name.endsWith(".d.ts"));
  const declarations = (await Promise.all(declarationFiles.map(name =>
    readFile(join(__dirname, "../dist", name), "utf8")))).join("\n");
  for (const internal of [
    /\belkInput\s*\(/,
    /\bworkerInput\s*\(/,
    /\bexecutionInput\s*\(/,
    /\bhasAccepted\s*\(/,
    /export declare const paperConfiguration\b/,
    /export interface ContactRequest\b/,
    /export interface ContactResponse\b/,
  ]) assert.doesNotMatch(declarations, internal);
});
