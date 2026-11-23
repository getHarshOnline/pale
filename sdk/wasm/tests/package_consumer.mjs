import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { copyFile, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

// Test only packed files: neither package resolves through the source checkout.
const { values } = parseArgs({
  options: { contracts: { type: "string" }, typescript: { type: "string" } },
});
assert(values.contracts && values.typescript,
  "pass --contracts <exact local tgz> --typescript <installed typescript/bin/tsc>");
assert(Number(process.versions.node.split(".")[0]) >= 22, "Node >=22 is required");
const contracts = path.resolve(values.contracts);
const compiler = path.resolve(values.typescript);
const root = fileURLToPath(new URL("../", import.meta.url));
const temporary = await mkdtemp(path.join(tmpdir(), "pale-wasm-consumer-"));
const digest = async (file) => createHash("sha256").update(await readFile(file)).digest("hex");
const npm = (args, cwd) => execFileSync("npm", [
  ...args, "--offline", "--cache", path.join(temporary, "npm-cache"),
], { cwd, encoding: "utf8", maxBuffer: 16 * 1024 * 1024, stdio: ["ignore", "pipe", "inherit"] });

try {
  const packed = JSON.parse(npm([
    "pack", "--json", "--ignore-scripts", "--pack-destination", temporary,
  ], root));
  assert.equal(packed.length, 1);
  const wasmPackage = path.join(temporary, packed[0].filename);
  await writeFile(path.join(temporary, "package.json"), JSON.stringify({
    name: "pale-wasm-package-consumer", private: true, type: "module",
  }, null, 2) + "\n");
  npm([
    "install", "--ignore-scripts", "--no-audit", "--no-fund", "--no-package-lock",
    wasmPackage, contracts,
  ], temporary);
  await copyFile(new URL("package_consumer.ts", import.meta.url), path.join(temporary, "consumer.ts"));
  await copyFile(new URL("consumed_arguments.mjs", import.meta.url), path.join(temporary, "consumed_arguments.mjs"));
  await copyFile(new URL("fixtures/cross-leaf.v1/contract-diagnostic.json", import.meta.url),
    path.join(temporary, "contract-diagnostic.json"));
  const portableRoot = new URL("fixtures/cross-leaf.v1/", import.meta.url);
  const portable = JSON.parse(await readFile(new URL("portable-journey.json", portableRoot), "utf8"));
  const documents = [];
  for (const label of portable.document_labels) {
    documents.push({ label, bytes: [...await readFile(new URL(`portable/${label}`, portableRoot))] });
  }
  await writeFile(path.join(temporary, "portable-input.json"), JSON.stringify({
    rootManifest: portable.root_manifest, documents,
  }, null, 2) + "\n");
  execFileSync(process.execPath, [
    compiler, "consumer.ts", "--allowJs", "--strict", "--noUncheckedIndexedAccess",
    "--exactOptionalPropertyTypes", "--noEmitOnError", "--module", "NodeNext",
    "--moduleResolution", "NodeNext", "--target", "ES2022", "--lib", "ESNext,DOM",
    "--resolveJsonModule", "--outDir", "compiled",
  ], { cwd: temporary, stdio: "inherit" });
  const run = () => spawnSync(process.execPath, [
    "--experimental-wasm-modules", path.join(temporary, "compiled/consumer.js"),
  ], { cwd: temporary, encoding: "utf8" });
  const accepted = run();
  assert.ifError(accepted.error);
  assert.equal(accepted.status, 0, accepted.stderr);

  // A version match alone must not let different sealed contract sets pass this gate.
  const registry = path.join(temporary, "node_modules/@rtepl/pale-contracts/dist/generated/registry.js");
  const original = await readFile(registry, "utf8");
  const mismatched = original.replace(/(SCHEMA_SET_SHA256 = ")[a-f0-9]{64}(";)/,
    `$1${"0".repeat(64)}$2`);
  assert.notEqual(mismatched, original, "the generated set coordinate must be present");
  await writeFile(registry, mismatched);
  const rejected = run();
  assert.ifError(rejected.error);
  assert.notEqual(rejected.status, 0, "a mismatched contract set must fail the consumer gate");
  assert.match(rejected.stderr, /embedded and package contract sets differ/);

  console.log(JSON.stringify({
    consumer: JSON.parse(accepted.stdout),
    mismatchedSetRejected: true,
    wasmPackageSha256: await digest(wasmPackage),
    contractsPackageSha256: await digest(contracts),
    node: process.version,
    typescript: execFileSync(process.execPath, [compiler, "--version"], { encoding: "utf8" }).trim(),
    evidence: "strict TypeScript and Node-hosted bundler-format package import; not browser qualification",
  }, null, 2));
} finally {
  await rm(temporary, { recursive: true, force: true });
}
