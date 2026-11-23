import { readFile, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";

const toolRoot = path.dirname(fileURLToPath(import.meta.url));
const repositoryRoot = path.resolve(toolRoot, "../..");
const templatePath = path.join(toolRoot, "templates/generated-declaration-names.mjs");
const targets = [
  "interfaces/lsp/tools/declaration-names.mjs",
  "sdk/js/tools/declaration-names.mjs",
  "sdk/js/contracts/tools/declaration-names.mjs",
  "sdk/scene/tools/declaration-names.mjs",
];
const header = [
  "// Generated from tools/repository-quality/templates/generated-declaration-names.mjs.",
  "// Regenerate with tools/repository-quality/sync-generated-declaration-name-checks.mjs.",
  "",
  "",
].join("\n");

const expected = `${header}${await readFile(templatePath, "utf8")}`;
const checking = process.argv.slice(2).includes("--check");

for (const relativePath of targets) {
  const target = path.join(repositoryRoot, relativePath);
  if (checking) {
    const actual = await readFile(target, "utf8");
    if (actual !== expected) {
      throw new Error(`${relativePath} is not synchronized with ${path.relative(repositoryRoot, templatePath)}`);
    }
  } else {
    await writeFile(target, expected, "utf8");
  }
}

console.log(
  checking
    ? `verified ${targets.length} standalone declaration-name checks`
    : `updated ${targets.length} standalone declaration-name checks`,
);
