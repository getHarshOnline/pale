import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { copyFile, mkdtemp, readFile, rename, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));
const manifest = JSON.parse(await readFile(path.join(root, "package.json"), "utf8"));
const temporary = await mkdtemp(path.join(tmpdir(), "pale-js-native-consumer-"));
const npm = (args, cwd) => execFileSync("npm", [...args, "--offline"], {
  cwd, encoding: "utf8", maxBuffer: 16 * 1024 * 1024,
  stdio: ["ignore", "pipe", "inherit"],
});

try {
  // The cached registry supplies pinned dependencies; no source-checkout
  // symlink, parent typeRoots, global shim or lifecycle script is admitted.
  const [packed] = JSON.parse(npm([
    "pack", "--json", "--ignore-scripts", "--pack-destination", temporary,
  ], root));
  assert(packed);
  const archive = path.join(temporary, packed.filename);
  await writeFile(path.join(temporary, "package.json"), JSON.stringify({
    name: "pale-js-native-package-consumer", private: true, type: "module",
  }, null, 2) + "\n");
  npm([
    "install", "--ignore-scripts", "--no-audit", "--no-fund", "--no-package-lock",
    archive, `typescript@${manifest.devDependencies.typescript}`,
  ], temporary);
  await copyFile(new URL("package_consumer.ts", import.meta.url), path.join(temporary, "consumer.ts"));
  await copyFile(new URL("organization_consumer.ts", import.meta.url), path.join(temporary, "organization-consumer.ts"));
  await writeFile(path.join(temporary, "tsconfig.json"), JSON.stringify({
    compilerOptions: {
      strict: true, noUncheckedIndexedAccess: true, exactOptionalPropertyTypes: true,
      noEmitOnError: true, skipLibCheck: false, types: [], target: "ES2022",
      module: "NodeNext", moduleResolution: "NodeNext", outDir: "compiled",
    },
    files: ["consumer.ts", "organization-consumer.ts"],
  }, null, 2) + "\n");
  const compiler = path.join(temporary, "node_modules/typescript/bin/tsc");
  const check = () => spawnSync(process.execPath, [compiler, "--project", "tsconfig.json"], {
    cwd: temporary, encoding: "utf8",
  });
  const accepted = check();
  assert.ifError(accepted.error);
  assert.equal(accepted.status, 0, accepted.stdout + accepted.stderr);
  const output = execFileSync(process.execPath, [
    path.join(temporary, "compiled/consumer.js"), path.join(root, "tests/fixtures/cross-leaf.v1"),
  ], {
    cwd: temporary, encoding: "utf8",
  });
  const organizationTypes = execFileSync(process.execPath, [path.join(temporary, "compiled/organization-consumer.js")], { cwd: temporary, encoding: "utf8" });
  await copyFile(new URL("organization.mjs", import.meta.url), path.join(temporary, "organization.mjs"));
  const organization = execFileSync(process.execPath, [
    "--expose-gc", path.join(temporary, "organization.mjs"), "--module", "@rtepl/pale", path.join(root, "tests/fixtures/organization"),
  ], { cwd: temporary, encoding: "utf8" });
  await copyFile(new URL("nominal_consumer.mjs", import.meta.url), path.join(temporary, "nominal-consumer.mjs"));
  const nominal = execFileSync(process.execPath, [path.join(temporary, "nominal-consumer.mjs")], {
    cwd: temporary, encoding: "utf8",
  });

  // Removing the declared type dependency must break the checked boundary;
  // an accidental any/global declaration must not keep this consumer green.
  const nodeTypes = path.join(temporary, "node_modules/@types/node");
  const installedTypes = JSON.parse(await readFile(path.join(nodeTypes, "package.json"), "utf8"));
  assert.equal(installedTypes.version, manifest.dependencies["@types/node"]);
  await rename(nodeTypes, path.join(temporary, "withheld-node-types"));
  const rejected = check();
  assert.ifError(rejected.error);
  assert.notEqual(rejected.status, 0, "missing Node declarations must fail strict checking");
  assert.match(rejected.stdout + rejected.stderr, /Cannot find type definition file for 'node'/);
  console.log(JSON.stringify({
    consumer: JSON.parse(output),
    organization: JSON.parse(organization),
    organizationTypes: JSON.parse(organizationTypes),
    nominal: JSON.parse(nominal),
    missingTypeDependencyRejected: true,
    packageSha256: createHash("sha256").update(await readFile(archive)).digest("hex"),
    node: process.version,
    nodeTypes: installedTypes.version,
    typescript: manifest.devDependencies.typescript,
  }, null, 2));
} finally {
  await rm(temporary, { recursive: true, force: true });
}
