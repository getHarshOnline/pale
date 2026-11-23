// Copyright © 2026 RawThoughts Enterprises Private Limited. AGPL-3.0-or-later.
import assert from "node:assert/strict";
import { lstat, readFile, readdir } from "node:fs/promises";
import { createRequire } from "node:module";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const root = new URL("../", import.meta.url);
const modules = ["index", "organization", "geometry", "routeBundles", "elkExecutor",
  "elkWorker", "sceneWorker", "contactAnalysis", "contactExecutor", "paperWorker",
  "organizationIntake", "organizationTopology", "organizationScene", "sceneBudget", "change", "changeIntake"];
const expected = modules.flatMap(name => [`${name}.js`, `${name}.d.ts`]).sort();
assert.deepEqual((await readdir(new URL("src/", root))).sort(), modules.map(name => `${name}.ts`).sort());
async function inventory() {
  let entries;
  try {
    assert((await lstat(new URL("dist", root))).isDirectory(), "Build destination must be an ordinary directory");
    entries = await readdir(new URL("dist/", root));
  }
  catch (error) { if (error.code === "ENOENT") return []; throw error; }
  for (const name of entries) {
    assert(expected.includes(name), `Unexpected build output: ${name}`);
    assert((await lstat(new URL(`dist/${name}`, root))).isFile(), `Non-file build output: ${name}`);
  }
  return entries.sort();
}
await inventory();
const require = createRequire(new URL("package.json", root));
const result = spawnSync(process.execPath, [require.resolve("typescript/bin/tsc"), "-p", "tsconfig.json"], {
  cwd: fileURLToPath(root), stdio: "inherit",
});
if (result.error) throw result.error;
assert.equal(result.signal, null, "TypeScript compilation terminated");
assert.equal(result.status, 0, "TypeScript compilation failed");
assert.deepEqual(await inventory(), expected);
const { assertMeaningfulDeclarationNames } = await import("./declaration-names.mjs");
const { default: ts } = await import("typescript");
for (const file of expected.filter((name) => name.endsWith(".d.ts"))) {
  const text = await readFile(new URL(`dist/${file}`, root), "utf8");
  assertMeaningfulDeclarationNames(text, file);
  const source = ts.createSourceFile(file, text, ts.ScriptTarget.Latest, true);
  for (const statement of source.statements) {
    const declaration = ts.isClassDeclaration(statement)
      || ts.isEnumDeclaration(statement)
      || ts.isFunctionDeclaration(statement)
      || ts.isInterfaceDeclaration(statement)
      || ts.isTypeAliasDeclaration(statement)
      || ts.isVariableStatement(statement);
    const exported = statement.modifiers?.some((modifier) => modifier.kind === ts.SyntaxKind.ExportKeyword);
    if (!declaration || !exported) continue;
    const leading = source.text.slice(statement.getFullStart(), statement.getStart(source));
    assert.match(leading, /\/\*\*[\s\S]*?\*\//, `${file}: exported declaration needs TSDoc`);
  }
}
