import assert from "node:assert/strict";
import { isUtf8 } from "node:buffer";
import { execFileSync } from "node:child_process";
import { mkdtemp, readFile, readdir, rm } from "node:fs/promises";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { SCHEMA_FAMILIES } from "../dist/index.js";
import { assertMeaningfulDeclarationNames } from "../tools/declaration-names.mjs";

test("the built type inventory exactly projects the current schema registry", async () => {
  const files = await readdir(new URL("../dist/generated/types/", import.meta.url));
  const expected = SCHEMA_FAMILIES.flatMap((family) => [
    `${family}.d.ts`,
    `${family}.js`,
  ]).sort();

  assert.deepEqual(files.sort(), expected);
  assert.equal(
    files.some((file) => file.endsWith(".map")),
    false,
    "the package must not ship maps that point to omitted generated sources",
  );
});

test("generated declarations use semantic names instead of collision counters", async () => {
  const directory = new URL("../dist/generated/types/", import.meta.url);
  const declarations = (await readdir(directory)).filter((file) => file.endsWith(".d.ts"));
  for (const declaration of declarations) {
    const source = await readFile(new URL(declaration, directory), "utf8");
    assert.doesNotThrow(() => assertMeaningfulDeclarationNames(source, declaration), declaration);
  }
  assert.throws(
    () => assertMeaningfulDeclarationNames(
      "export interface ProfileInspectionContext {}\nexport interface ProfileInspectionContext2 {}\n",
      "counter-fixture.d.ts",
    ),
    /numeric collision counter: ProfileInspectionContext2/,
  );
  assert.throws(
    () => assertMeaningfulDeclarationNames(
      "export interface SigningAuthorityBindingV1 {}\nexport interface SigningAuthorityBindingV11 {}\n",
      "version-counter-fixture.d.ts",
    ),
    /meaningless collision name SigningAuthorityBindingV11/,
  );
  assert.throws(
    () => assertMeaningfulDeclarationNames(
      "export interface TypedDigestV1 {}\nexport interface TypedDigestV1TypedDigestCanonicalizationV1 {}\n",
      "path-name-fixture.d.ts",
    ),
    /repeats a version segment: TypedDigestV1TypedDigestCanonicalizationV1/,
  );
  assert.throws(
    () => assertMeaningfulDeclarationNames(
      "export interface ProviderBindingV1 {}\nexport interface ProviderBindingV1ProviderCoordinates {}\n",
      "owner-name-fixture.d.ts",
    ),
    /repeats its owning declaration: ProviderBindingV1ProviderCoordinates/,
  );
  assert.doesNotThrow(() => assertMeaningfulDeclarationNames(
    "export interface Sha256DigestV1 {}\nexport interface Http2PolicyV1 {}\n",
    "semantic-number-fixture.d.ts",
  ));

  for (const [file, names] of Object.entries({
    "adapter-failure.v1.d.ts": ["SigningAuthorityBindingModeConstraintV1"],
    "capability-result.v1.d.ts": [
      "CapabilitySuccessPayloadLocationV1",
      "SigningAuthorityBindingModeConstraintV1",
    ],
    "common-request-envelope.v1.d.ts": ["SigningAuthorityBindingModeConstraintV1"],
    "workspace-derivation-receipt.v1.d.ts": [
      "WorkspaceDerivationImpactStateV1",
      "WorkspaceDerivationRootChangeV1",
    ],
  })) {
    const source = await readFile(new URL(file, directory), "utf8");
    for (const name of names) {
      assert.match(source, new RegExp(`export type ${name}\\b`), `${file}: missing ${name}`);
    }
    assert.doesNotMatch(source, /export type [A-Za-z0-9]+VariantV1\b/);
  }
});

test("complete helper notices survive compilation and the packed distribution", async () => {
  const require = createRequire(import.meta.url);
  const root = fileURLToPath(new URL("../", import.meta.url));
  const source = await readFile(path.join(root, "src/generated/validators-standalone.js"));
  const compiled = await readFile(path.join(root, "dist/generated/validators-standalone.js"));
  for (const name of ["ajv", "ajv-formats", "fast-deep-equal"]) {
    const packageRoot = path.dirname(require.resolve(`${name}/package.json`));
    const metadata = JSON.parse(await readFile(path.join(packageRoot, "package.json"), "utf8"));
    const license = await readFile(path.join(packageRoot, "LICENSE"));
    assert(isUtf8(license), `${name}: upstream notice is not lossless UTF-8 text`);
    const notice = Buffer.concat([
      Buffer.from(`/*! Bundled helper: ${name}@${metadata.version}\n\n`),
      license,
      Buffer.from("\n*/"),
    ]);
    assert(source.includes(notice), `${name}: generated validator omitted the complete notice`);
    assert(compiled.includes(notice), `${name}: compilation did not preserve the complete notice`);
  }

  // Use the same offline npm pack path as the SDK consumer test. Read the
  // archive member, not only npm's inventory, to check the distributed bytes.
  const temporary = await mkdtemp(path.join(tmpdir(), "pale-contract-notices-"));
  try {
    const packed = JSON.parse(execFileSync("npm", [
      "pack", "--offline", "--json", "--ignore-scripts", "--pack-destination", temporary,
    ], { cwd: root, encoding: "utf8", timeout: 60_000 }));
    assert.equal(packed.length, 1);
    assert.equal(path.basename(packed[0].filename), packed[0].filename);
    const member = execFileSync("tar", [
      "-xOf", path.join(temporary, packed[0].filename), "package/dist/generated/validators-standalone.js",
    ], { maxBuffer: compiled.length + 1, timeout: 60_000 });
    assert.deepEqual(member, compiled, "npm pack changed or omitted the checked validator bytes");
  } finally {
    await rm(temporary, { recursive: true, force: true });
  }
});
