import { isUtf8 } from "node:buffer";
import { createHash } from "node:crypto";
import { mkdir, readFile, readdir, realpath, rm, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";
import Ajv2020 from "ajv/dist/2020.js";
import standaloneCode from "ajv/dist/standalone/index.js";
import addFormats from "ajv-formats";
import { build } from "esbuild";
import { compile } from "json-schema-to-typescript";
import { _ } from "ajv/dist/compile/codegen/index.js";
import { UINT_FORMATS } from "./uint-formats.mjs";
import { assertMeaningfulDeclarationNames } from "./declaration-names.mjs";

const here = path.dirname(fileURLToPath(import.meta.url));
const arguments_ = process.argv.slice(2);
const check = arguments_.includes("--check");
const bundleOption = arguments_.indexOf("--bundle");
if (bundleOption === -1 || bundleOption + 1 >= arguments_.length) {
  throw new Error("usage: generate.mjs [--check] --bundle <agent-surface.v1-directory>");
}
const unknownArguments = arguments_.filter(
  (argument, index) => argument !== "--check" && argument !== "--bundle" && index !== bundleOption + 1,
);
if (unknownArguments.length !== 0) {
  throw new Error(`unknown arguments: ${unknownArguments.join(", ")}`);
}
const bundle = path.resolve(process.cwd(), arguments_[bundleOption + 1]);
const registryPath = path.join(bundle, "agent-surface-schemas.v1.json");
const generated = path.resolve(here, "../src/generated");
const packageRoot = path.resolve(here, "..");
const generatorPath = fileURLToPath(import.meta.url);
const declarationNamesPath = path.join(here, "declaration-names.mjs");
const [registryBytes, packageBytes, lockBytes, generatorBytes, declarationNamesBytes] = await Promise.all([
  readFile(registryPath),
  readFile(path.join(packageRoot, "package.json")),
  readFile(path.join(packageRoot, "package-lock.json")),
  readFile(generatorPath),
  readFile(declarationNamesPath),
]);
const registry = JSON.parse(registryBytes);
const packageMetadata = JSON.parse(packageBytes);
const families = Object.keys(registry.schemas).sort();
const schemas = new Map();

for (const family of families) {
  const descriptor = registry.schemas[family];
  const bytes = await readFile(path.join(bundle, descriptor.path));
  const digest = createHash("sha256").update(bytes).digest("hex");
  if (digest !== descriptor.schema_sha256) throw new Error(`${family}: registry digest is stale`);
  const schema = JSON.parse(bytes);
  if (schema.$schema !== "https://json-schema.org/draft/2020-12/schema") throw new Error(`${family}: not draft 2020-12`);
  schemas.set(family, schema);
}

const pascalPart = (part) => /^[A-Za-z]\d+[A-Za-z]$/.test(part)
  ? part.toUpperCase()
  : part[0].toUpperCase() + part.slice(1);
const pascal = (family) => family
  .split(/[^A-Za-z0-9]+/)
  .filter(Boolean)
  .map(pascalPart)
  .join("");
const identifier = (family) => `validate_${family.replace(/[^A-Za-z0-9_$]/g, "_")}`;
const insertBeforeVersion = (name, qualifier) => {
  const match = name.match(/^(.*?)(V\d+)$/);
  return match ? `${match[1]}${qualifier}${match[2]}` : `${name}${qualifier}`;
};
const rootTypeName = (family, schema) => {
  const conventional = pascal(family);
  return Object.hasOwn(schema.$defs ?? {}, conventional)
    ? insertBeforeVersion(conventional, "Document")
    : conventional;
};
const assertUniqueNames = (label, entries) => {
  const owners = new Map();
  for (const [family, name] of entries) {
    const prior = owners.get(name);
    if (prior !== undefined) {
      throw new Error(`${label} collision: ${JSON.stringify(prior)} and ${JSON.stringify(family)} both project as ${JSON.stringify(name)}`);
    }
    owners.set(name, family);
  }
};

// Some Rust records flatten a named state or constraint into shared fields plus
// `oneOf`. The TypeScript compiler needs a name for that inline conjunction.
// Keep those projection-only names explicit so they describe the constraint
// instead of exposing a generic `Variant` helper or a traversal-order counter.
const sharedOneOfProjectionNameEntries = [
  ["CapabilitySuccessV1", "CapabilitySuccessPayloadLocationV1"],
  ["SigningAuthorityBindingV1", "SigningAuthorityBindingModeConstraintV1"],
  ["WorkspaceDerivationImpactV1", "WorkspaceDerivationImpactStateV1"],
  ["WorkspaceDerivationRootV1", "WorkspaceDerivationRootChangeV1"],
];
assertUniqueNames("shared oneOf projection owner", sharedOneOfProjectionNameEntries
  .map(([owner, name]) => [name, owner]));
const sharedOneOfProjectionNames = new Map(sharedOneOfProjectionNameEntries);
assertUniqueNames("shared oneOf projection name", sharedOneOfProjectionNames);
const usedSharedOneOfProjectionNames = new Set();

const schemaReference = /^https:\/\/www\.rawthoughts\.in\/pale\/schemas\/([^/#]+)\.json(?:#\/\$defs\/([^/]+))?$/;
const stripVersion = (name) => name.replace(/V\d+$/, "");
const definitionQualifier = (family, name) => {
  const owner = stripVersion(rootTypes.get(family));
  return name.startsWith(owner)
    ? insertBeforeVersion(name, "Definition")
    : `${owner}${name}`;
};

// Build one local definition universe for each generated declaration file.
// External references often repeat the same small identity definition. Keeping
// each referenced schema as a separate document makes json-schema-to-typescript
// invent traversal-path names or numeric counters for that one meaning. Merge
// structurally identical definitions once and rewrite every reference to that semantic
// name. Only genuinely different definitions with the same name receive an
// owner qualifier.
const flattenReferences = (family) => {
  const source = structuredClone(schemas.get(family));
  const definitions = {};
  const fingerprints = new Map();
  const inProgress = new Set();

  const importDefinition = (ownerFamily, name) => {
    const ownerSchema = schemas.get(ownerFamily);
    const definition = ownerSchema?.$defs?.[name];
    if (definition === undefined) {
      throw new Error(`${ownerFamily}: missing referenced definition ${name}`);
    }
    return mergeDefinition(ownerFamily, name, definition);
  };

  const importRoot = (ownerFamily) => {
    const ownerSchema = schemas.get(ownerFamily);
    if (ownerSchema === undefined) throw new Error(`unregistered schema reference: ${ownerFamily}`);
    const name = rootTypes.get(ownerFamily);
    const definition = structuredClone(ownerSchema);
    delete definition.$defs;
    delete definition.$id;
    delete definition.$schema;
    delete definition.title;
    return mergeDefinition(ownerFamily, name, definition);
  };

  const rewrite = (value, ownerFamily) => {
    if (Array.isArray(value)) return value.map((item) => rewrite(item, ownerFamily));
    if (typeof value !== "object" || value === null) return value;
    const rewritten = {};
    for (const [key, item] of Object.entries(value)) {
      if (key !== "$ref" || typeof item !== "string") {
        rewritten[key] = rewrite(item, ownerFamily);
        continue;
      }
      if (item.startsWith("#/$defs/")) {
        const name = item.slice("#/$defs/".length);
        rewritten.$ref = `#/$defs/${importDefinition(ownerFamily, name)}`;
        continue;
      }
      const match = item.match(schemaReference);
      if (match === null) throw new Error(`${ownerFamily}: unsupported schema reference ${item}`);
      const [, referencedFamily, referencedDefinition] = match;
      rewritten.$ref = `#/$defs/${referencedDefinition === undefined
        ? importRoot(referencedFamily)
        : importDefinition(referencedFamily, referencedDefinition)}`;
    }
    return rewritten;
  };

  function mergeDefinition(ownerFamily, preferredName, definition) {
    const fingerprint = JSON.stringify(definition);
    let name = preferredName;
    const existing = fingerprints.get(name);
    if (existing !== undefined && existing !== fingerprint) {
      name = definitionQualifier(ownerFamily, preferredName);
    }
    while (fingerprints.has(name) && fingerprints.get(name) !== fingerprint) {
      name = definitionQualifier(ownerFamily, name);
    }
    if (fingerprints.has(name)) return name;
    fingerprints.set(name, fingerprint);
    definitions[name] = {};
    if (!inProgress.has(name)) {
      inProgress.add(name);
      definitions[name] = rewrite(structuredClone(definition), ownerFamily);
      inProgress.delete(name);
    }
    return name;
  }

  delete source.$defs;
  const flattened = rewrite(source, family);
  flattened.$defs = definitions;
  return flattened;
};

// json-schema-to-typescript otherwise derives some names from the traversal
// path and resolves collisions by appending digits. Give every definition its
// semantic schema key before the projection reaches that library. Definitions
// with the same name are byte-equivalent throughout this bundle, so an owner
// prefix would add noise without disambiguating meaning. Do not add a title to
// ordinary definitions: the compiler uses titles for inline refinements too,
// which would turn a constrained property into a numbered sibling alias.
//
// A definition that combines shared fields with oneOf branches must have one
// explicit semantic projection name above.
//
// These are TypeScript projection annotations. They do not modify the
// canonical schema bytes or validation behavior.
const prepareDefinitions = (schema) => {
  for (const [name, definition] of Object.entries(schema.$defs ?? {})) {
    if (typeof definition !== "object" || definition === null || Array.isArray(definition)) continue;
    const projectedName = name;
    if (Array.isArray(definition.oneOf) && definition.oneOf.length > 0) {
      const hasSharedShape = definition.properties !== undefined
        || definition.required !== undefined
        || definition.type !== undefined
        || definition.additionalProperties !== undefined
        || definition.unevaluatedProperties !== undefined;
      if (hasSharedShape) {
        const constraintName = sharedOneOfProjectionNames.get(projectedName);
        if (constraintName === undefined) {
          throw new Error(`${projectedName}: shared oneOf requires a semantic TypeScript projection name`);
        }
        if (Object.hasOwn(schema.$defs, constraintName)) {
          throw new Error(`${projectedName}: projection name collides with ${constraintName}`);
        }
        usedSharedOneOfProjectionNames.add(projectedName);
        const variant = {
          title: constraintName,
          oneOf: definition.oneOf,
        };
        delete definition.oneOf;
        definition.allOf = [...(definition.allOf ?? []), variant];
      }
    }
  }
  return schema;
};
const rootTypes = new Map(families.map((family) => [family, rootTypeName(family, schemas.get(family))]));
assertUniqueNames("contract root type", rootTypes);
assertUniqueNames("validator identifier", families.map((family) => [family, identifier(family)]));
const outputs = new Map();

for (const family of families) {
  const schema = prepareDefinitions(flattenReferences(family));
  const typeName = rootTypes.get(family);
  schema.title = typeName;
  const declaration = await compile(schema, typeName, {
    bannerComment: "/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */",
    format: true,
    unknownAny: false,
  });
  assertMeaningfulDeclarationNames(declaration, `${family}.ts`);
  outputs.set(`types/${family}.ts`, declaration);
}
const staleSharedOneOfProjectionNames = [...sharedOneOfProjectionNames.keys()]
  .filter((name) => !usedSharedOneOfProjectionNames.has(name));
if (staleSharedOneOfProjectionNames.length !== 0) {
  throw new Error(`unused shared oneOf projection names: ${staleSharedOneOfProjectionNames.join(", ")}`);
}

outputs.set("contracts.ts", `${families.map((family) => `export type { ${rootTypes.get(family)} } from "./types/${family}.js";`).join("\n")}\n`);
outputs.set("contract-map.ts", `/* Generated family-to-contract type map. Do not edit. */
${families.map((family) => `import type { ${rootTypes.get(family)} } from "./types/${family}.js";`).join("\n")}
export interface ContractByFamily {
${families.map((family) => `  ${JSON.stringify(family)}: ${rootTypes.get(family)};`).join("\n")}
}
`);

const authored = families.filter((family) => registry.schemas[family].lifecycle.role === "authored_input");
const identities = Object.fromEntries(families.map((family) => [family, registry.schemas[family].schema_sha256]));
outputs.set("registry.ts", `/* Generated from the canonical Pale initial-v1 registry. Do not edit. */
export const SCHEMA_REGISTRY_ID = ${JSON.stringify(registry.registry)} as const;
export const SCHEMA_MANIFEST_SHA256 = ${JSON.stringify(registry.schema_manifest_sha256)} as const;
export const SCHEMA_SET_SHA256 = ${JSON.stringify(registry.schema_set_sha256)} as const;
export const SCHEMA_FAMILIES = ${JSON.stringify(families, null, 2)} as const;
export const AUTHORED_INPUT_FAMILIES = ${JSON.stringify(authored, null, 2)} as const;
export type AuthoredInputFamily = typeof AUTHORED_INPUT_FAMILIES[number];
export const CONTRACT_IDENTITIES = ${JSON.stringify(identities, null, 2)} as const;
`);

const standaloneFormats = _`({...require("ajv-formats/dist/formats").fullFormats,...require("./tools/uint-formats.mjs").UINT_FORMATS})`;
const ajv = new Ajv2020({
  allErrors: true,
  code: { esm: true, formats: standaloneFormats, source: true },
  strict: false,
  validateFormats: true,
});
addFormats(ajv);
for (const [name, format] of Object.entries(UINT_FORMATS)) ajv.addFormat(name, format);
for (const schema of schemas.values()) ajv.addSchema(schema);
const validatorNames = Object.fromEntries(families.map((family) => [identifier(family), schemas.get(family).$id]));
// These packages contribute executable helpers to the standalone validator.
// Carry their complete notices in the file that consumers copy and bundle.
const require = createRequire(import.meta.url);
const lock = JSON.parse(lockBytes);
const helperNames = ["ajv", "ajv-formats", "fast-deep-equal"];
const helpers = await Promise.all(helperNames.map(async (name) => {
  const root = await realpath(path.dirname(require.resolve(`${name}/package.json`)));
  const metadata = JSON.parse(await readFile(path.join(root, "package.json"), "utf8"));
  const pinned = lock.packages[`node_modules/${name}`];
  if (!pinned || metadata.name !== name || metadata.version !== pinned.version || pinned.license !== "MIT" || metadata.license !== "MIT") {
    throw new Error(`${name}: bundled helper does not match the pinned MIT package`);
  }
  const licenseBytes = await readFile(path.join(root, "LICENSE"));
  if (!isUtf8(licenseBytes)) throw new Error(`${name}: license is not lossless UTF-8 text`);
  const license = licenseBytes.toString("utf8");
  if (licenseBytes.length === 0 || license.includes("*/")) {
    throw new Error(`${name}: complete license cannot be preserved in a block comment`);
  }
  return { name, root, version: metadata.version, license };
}));
const helperNotices = helpers.map(({ name, version, license }) =>
  `/*! Bundled helper: ${name}@${version}\n\n${license}\n*/`).join("\n");
const bundledValidators = await build({
  absWorkingDir: packageRoot,
  banner: { js: helperNotices },
  bundle: true,
  format: "esm",
  legalComments: "inline",
  metafile: true,
  minify: true,
  platform: "neutral",
  stdin: { contents: standaloneCode(ajv, validatorNames), loader: "js", resolveDir: path.resolve(here, "..") },
  write: false,
});
const localHelper = await realpath(path.join(here, "uint-formats.mjs"));
const bundledPackages = new Set();
for (const input of Object.keys(bundledValidators.metafile.inputs)) {
  if (input === "<stdin>") continue;
  const source = await realpath(path.resolve(packageRoot, input));
  if (source === localHelper) continue;
  const helperPackage = helpers.find(({ root }) =>
    source.startsWith(`${root}${path.sep}`) && !path.relative(root, source).split(path.sep).includes("node_modules"));
  if (!helperPackage) throw new Error(`bundled helper is outside the reviewed packages: ${input}`);
  bundledPackages.add(helperPackage.name);
}
if (helperNames.some((name) => !bundledPackages.has(name))) {
  throw new Error("bundled helper packages changed; review the notice inventory");
}
outputs.set("validators-standalone.js", bundledValidators.outputFiles[0].text);
outputs.set("validators.ts", `/* Generated standalone-validator inventory. Do not edit. */
import * as standalone from "./validators-standalone.js";
import type { ValidateFunction } from "./validator-types.js";
const validators = {
${families.map((family) => `  ${JSON.stringify(family)}: standalone.${identifier(family)} as ValidateFunction,`).join("\n")}
} as const;
export default validators;
`);

const outputRows = [...outputs]
  .map(([relative, content]) => ({
    path: relative,
    sha256: createHash("sha256").update(content).digest("hex"),
    size: Buffer.byteLength(content),
  }))
  .sort((left, right) => left.path.localeCompare(right.path));
const outputSetPreimage = outputRows
  .map(({ path: relative, sha256, size }) => `${relative}\0${sha256}\0${size}\n`)
  .join("");
outputs.set("source-coordinate.json", `${JSON.stringify({
  schema: "pale.typescript-contract-source-coordinate.v1",
  version: 1,
  source: {
    registry_sha256: createHash("sha256").update(registryBytes).digest("hex"),
    schema_manifest_sha256: registry.schema_manifest_sha256,
    schema_set_sha256: registry.schema_set_sha256,
    schema_count: families.length,
  },
  generator: {
    package: packageMetadata.name,
    version: packageMetadata.version,
    implementation_sha256: createHash("sha256")
      .update("generate.mjs\0").update(generatorBytes)
      .update("declaration-names.mjs\0").update(declarationNamesBytes)
      .digest("hex"),
    dependency_lock_sha256: createHash("sha256").update(lockBytes).digest("hex"),
    options: {
      schema_draft: "2020-12",
      types: { format: true, unknown_any: false },
      validation: {
        all_errors: true,
        minified: true,
        platform: "neutral",
        strict: false,
        validate_formats: true,
      },
    },
  },
  output_set: {
    aggregate_sha256: createHash("sha256").update(outputSetPreimage).digest("hex"),
    member_count: outputRows.length,
    members: outputRows,
  },
}, null, 2)}\n`);

if (check) {
  const expected = [...outputs.keys()].sort();
  const observed = (await readdir(generated, { recursive: true })).filter((entry) => !entry.endsWith("validator-types.ts")).sort();
  const files = observed.filter((entry) => !entry.endsWith("types"));
  if (JSON.stringify(files) !== JSON.stringify(expected)) throw new Error("generated file inventory is stale");
  for (const [relative, content] of outputs) {
    if ((await readFile(path.join(generated, relative), "utf8")) !== content) throw new Error(`${relative} is stale`);
  }
  console.log(`fresh: ${families.length} schemas at ${registry.schema_set_sha256}`);
} else {
  await rm(path.join(generated, "types"), { recursive: true, force: true });
  for (const [relative, content] of outputs) {
    const target = path.join(generated, relative);
    await mkdir(path.dirname(target), { recursive: true });
    await writeFile(target, content);
  }
  console.log(`generated: ${families.length} schemas at ${registry.schema_set_sha256}`);
}
