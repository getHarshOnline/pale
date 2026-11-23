// Development tooling only. No provider access or semantic admission occurs here.
import { createHash } from "node:crypto";
import { mkdir, readFile, readdir, realpath, writeFile } from "node:fs/promises";
import path from "node:path";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import Ajv2020 from "ajv/dist/2020.js";
import standaloneCode from "ajv/dist/standalone/index.js";
import addFormats from "ajv-formats";
import { _ } from "ajv/dist/compile/codegen/index.js";
import { build } from "esbuild";
import { compile } from "json-schema-to-typescript";
import { numberFormats } from "./number-formats.mjs";
import { assertMeaningfulDeclarationNames } from "./declaration-names.mjs";

const here = path.dirname(fileURLToPath(import.meta.url));
const args = process.argv.slice(2);
const check = args[0] === "--check";
if (check) args.shift();
let selectedResponses;
if (args[0] === "--responses-cjs") {
  args.shift();
  const selection = args.shift();
  if (selection === undefined) throw new Error("--responses-cjs requires response aliases");
  selectedResponses = [...new Set(selection.split(","))].sort();
  if (selectedResponses.length === 0 || selectedResponses.some((name) => name.length === 0)) {
    throw new Error("--responses-cjs requires nonempty comma-separated response aliases");
  }
}
if (args.length !== 2) throw new Error("usage: generate.mjs [--check] [--responses-cjs <response-aliases>] <schema-artifact.json> <output-directory>");
const [input, destination] = args.map((value) => path.resolve(value));
const schemaBytes = await readFile(input);
const source = JSON.parse(schemaBytes);
const bundles = Object.keys(source.schemas).sort();
if (JSON.stringify(bundles) !== JSON.stringify(["Requests", "Responses"])) {
  throw new Error("expected separate request and response schema catalogs");
}
const roots = new Map();
const hash = (bytes) => createHash("sha256").update(bytes).digest("hex");
const outputs = new Map();
const typeOptions = {
  bannerComment: "/* Generated from Pale LSP's private Rust wire types. Do not edit. */",
  format: true,
  unknownAny: true,
  $refOptions: { resolve: { file: false, http: false } },
};
const require = createRequire(import.meta.url);
const toolPackage = JSON.parse(await readFile(path.join(here, "package.json"), "utf8"));
const bundledLicenses = {};
const packageRoots = new Map();
let licenseBanner = "/*!\nBundled third-party helper licenses.\n";
for (const name of ["ajv", "ajv-formats"]) {
  const version = toolPackage.devDependencies?.[name];
  if (typeof version !== "string" || !/^[0-9]+\.[0-9]+\.[0-9]+$/.test(version)) {
    throw new Error(`bundled helper must have an exact release pin: ${name}`);
  }
  const manifest = require.resolve(`${name}/package.json`);
  const metadata = JSON.parse(await readFile(manifest, "utf8"));
  if (metadata.name !== name || metadata.version !== version) {
    throw new Error(`unqualified bundled helper version: ${name}`);
  }
  const root = await realpath(path.dirname(manifest));
  const license = await readFile(path.join(root, "LICENSE"));
  const text = license.toString("utf8");
  if (!Buffer.from(text).equals(license) || text.includes("*/")) {
    throw new Error(`bundled license cannot be preserved in a comment: ${name}`);
  }
  packageRoots.set(name, root);
  bundledLicenses[name] = { version, licenseSha256: hash(license) };
  licenseBanner += `\n--- ${name} ${version} ---\n${text}\n`;
}
licenseBanner += "*/";
const bundledPackages = {};

async function qualifiedBundlePackages(inputs) {
  const packages = new Set();
  for (const input of Object.keys(inputs)) {
    if (input === "<stdin>") continue;
    const absolute = await realpath(path.resolve(here, input));
    if (absolute === await realpath(path.join(here, "number-formats.mjs"))) continue;
    const dependency = [...packageRoots].find(([, root]) => {
      const relative = path.relative(root, absolute);
      return relative !== "" && relative !== ".." && !relative.startsWith(`..${path.sep}`)
        && !path.isAbsolute(relative) && !relative.split(path.sep).includes("node_modules");
    });
    if (!dependency) throw new Error(`unqualified bundled helper input: ${input}`);
    packages.add(dependency[0]);
  }
  return [...packages].sort();
}

// Each direction shares its own library-owned definitions. No resolver may
// fetch a remote reference or join the two serialization contracts.
function checkReferences(value) {
  if (value === null || typeof value !== "object") return;
  if ("$ref" in value && (typeof value.$ref !== "string" || !value.$ref.startsWith("#"))) {
    throw new Error(`non-local schema reference: ${value.$ref}`);
  }
  for (const child of Object.values(value)) checkReferences(child);
}

// json-schema-to-typescript dereferences a `$ref` with sibling annotations as
// a fresh schema object at every use. Express the same Draft 2020-12
// conjunction as an `allOf` for the type compiler so one definition keeps one
// declaration identity. This graph is type-generation input only; Ajv still
// compiles the exact exported wire schema below.
function typeCompilerSchema(schema) {
  const result = structuredClone(schema);
  for (const [name, definition] of Object.entries(result.$defs ?? {})) {
    if (definition === null || typeof definition !== "object" || Array.isArray(definition)
        || !Array.isArray(definition.oneOf) || definition.oneOf.length === 0) continue;
    const hasSharedShape = definition.properties !== undefined
      || definition.required !== undefined
      || definition.type !== undefined
      || definition.additionalProperties !== undefined
      || definition.unevaluatedProperties !== undefined;
    if (!hasSharedShape) continue;
    const detailName = `${name}Detail`;
    if (Object.hasOwn(result.$defs, detailName)) {
      throw new Error(`${name}: generated detail name collides with ${detailName}`);
    }
    // A flattened Rust tagged enum arrives as shared record fields plus oneOf.
    // Give that conjunction's variant portion its semantic role before the
    // type compiler invents a numeric sibling such as `EffectiveProfileEntry1`.
    const detail = { title: detailName, oneOf: definition.oneOf };
    delete definition.oneOf;
    definition.allOf = [...(definition.allOf ?? []), detail];
  }
  function preserveReferences(value) {
    if (value === null || typeof value !== "object") return;
    for (const child of Object.values(value)) preserveReferences(child);
    if (!Array.isArray(value) && Object.hasOwn(value, "$ref") && Object.keys(value).length > 1) {
      const reference = value.$ref;
      delete value.$ref;
      if (!Array.isArray(value.allOf)) value.allOf = [];
      value.allOf.push({ $ref: reference });
    }
  }
  preserveReferences(result);
  return result;
}

function selectedTypeCompilerSchema(schema, name) {
  const selection = schema.properties[name];
  const reference = selection.$ref;
  if (typeof reference !== "string" || !/^#\/\$defs\/[A-Za-z0-9_]+$/.test(reference)
      || Object.keys(selection).length !== 1) {
    return { title: name, $schema: schema.$schema, allOf: [selection], $defs: schema.$defs };
  }
  const definitionName = reference.slice("#/$defs/".length);
  if (!Object.hasOwn(schema.$defs, definitionName)) {
    throw new Error(`missing response definition: ${definitionName}`);
  }
  const definition = schema.$defs[definitionName];
  if (definition === null || typeof definition !== "object" || Array.isArray(definition)
      || Object.hasOwn(definition, "$ref")) {
    throw new Error("selected response requires an object definition without a root reference");
  }
  // Retain this exact object in its definition slot. Recursive references then
  // resolve back to the selected declaration instead of a numbered clone.
  const root = { ...definition, title: name, $schema: schema.$schema };
  root.$defs = { ...schema.$defs, [definitionName]: root };
  return root;
}

const typeSchemas = new Map();

for (const bundle of bundles) {
  const schema = source.schemas[bundle];
  checkReferences(schema);
  if (schema.$schema !== "https://json-schema.org/draft/2020-12/schema") {
    throw new Error(`${bundle}: expected Draft 2020-12`);
  }
  // Required catalog keys make indexed aliases precise, including nullable
  // roots. This catalog is never a request or response sent over the wire.
  const aliases = Object.keys(schema.properties).sort();
  if (JSON.stringify([...schema.required].sort()) !== JSON.stringify(aliases)) {
    throw new Error(`${bundle}: every exported alias must be a required catalog key`);
  }
  const typeSchema = typeCompilerSchema(schema);
  typeSchemas.set(bundle, typeSchema);
  if (selectedResponses === undefined) {
    outputs.set(`${bundle}.ts`, await compile({ ...typeSchema, title: bundle }, bundle, typeOptions));
  }
  for (const name of aliases) {
    if (!/^[A-Z][A-Za-z0-9]*$/.test(name) || bundles.includes(name) || roots.has(name)) {
      throw new Error(`invalid or repeated exported root alias: ${name}`);
    }
    roots.set(name, `${bundle}#/properties/${name}`);
    if (selectedResponses === undefined) outputs.set(`${name}.ts`, `/* Generated private payload alias. Do not edit. */
import type { ${bundle} } from "./${bundle}.js";
export type ${name} = ${bundle}[${JSON.stringify(name)}];
`);
  }
}
if (selectedResponses !== undefined) {
  const schema = source.schemas.Responses;
  for (const selectedResponse of selectedResponses) {
    if (!Object.hasOwn(schema.properties, selectedResponse)) {
      throw new Error(`not an exported response alias: ${selectedResponse}`);
    }
    outputs.set(`${selectedResponse}.ts`, await compile(
      selectedTypeCompilerSchema(typeSchemas.get("Responses"), selectedResponse),
      selectedResponse,
      typeOptions,
    ));
  }
}
const ajv = new Ajv2020({
  allErrors: true,
  messages: true,
  strict: false,
  strictSchema: true,
  inlineRefs: false,
  validateFormats: true,
  code: {
    esm: true,
    source: true,
    formats: _`({...require("ajv-formats/dist/formats").fullFormats,...require("./number-formats.mjs").numberFormats})`,
  },
});
addFormats(ajv);
for (const [name, format] of Object.entries(numberFormats)) ajv.addFormat(name, format);
function checkFormats(value) {
  if (value === null || typeof value !== "object") return;
  if ("format" in value && !(value.format in ajv.formats)) {
    throw new Error(`unqualified schema format: ${value.format}`);
  }
  for (const child of Object.values(value)) checkFormats(child);
}
for (const bundle of bundles) {
  checkFormats(source.schemas[bundle]);
  ajv.addSchema(source.schemas[bundle], bundle);
}
for (const bundle of bundles) {
  if (selectedResponses !== undefined && bundle !== "Responses") continue;
  const selectedNames = selectedResponses === undefined
    ? Object.keys(source.schemas[bundle].properties).sort() : selectedResponses;
  const validators = Object.fromEntries(selectedNames.map((name) => [`validate${name}`, roots.get(name)]));
  const bundled = await build({
    bundle: true,
    preserveSymlinks: true,
    absWorkingDir: here,
    format: selectedResponses === undefined ? "esm" : "cjs",
    minify: false,
    legalComments: "inline",
    banner: { js: licenseBanner },
    metafile: true,
    platform: "neutral",
    stdin: { contents: standaloneCode(ajv, validators), loader: "js", resolveDir: here },
    write: false,
  });
  // A receiving client imports only responses; it need not rely on removing
  // Ajv's top-level function metadata assignments to exclude request checks.
  const stem = bundle.toLowerCase();
  const extension = selectedResponses === undefined ? "mjs" : "cjs";
  const declaration = selectedResponses === undefined ? "d.mts" : "d.cts";
  bundledPackages[`${stem}.${extension}`] = await qualifiedBundlePackages(bundled.metafile.inputs);
  outputs.set(`${stem}.${extension}`, bundled.outputFiles[0].text);
  outputs.set(`${stem}.${declaration}`, `${selectedNames.map((name) => `import type { ${name} } from "./${name}.js";`).join("\n")}
${selectedNames.map((name) => `export function validate${name}(value: unknown): value is ${name};`).join("\n")}
`);
}
if (selectedResponses === undefined) outputs.set("protocol-map.ts", `/* Generated private method/root associations. Not admission policy. */
export const METHODS = ${JSON.stringify(source.methods, null, 2)} as const;
export const NOTIFICATIONS = ${JSON.stringify(source.notifications, null, 2)} as const;
export const INITIALIZATION = ${JSON.stringify(source.initialization)} as const;
export const ADVERTISEMENT = ${JSON.stringify(source.advertisement)} as const;
`);

for (const [file, content] of outputs) {
  if (!/\.(?:ts|d\.mts|d\.cts)$/.test(file)) continue;
  assertMeaningfulDeclarationNames(content, file);
}

const generatorFiles = ["declaration-names.mjs", "generate.mjs", "number-formats.mjs", "package.json", "package-lock.json"];
const generator = {};
for (const file of generatorFiles) generator[file] = hash(await readFile(path.join(here, file)));
const members = [...outputs].map(([file, content]) => ({ path: file, sha256: hash(content) }));
// This is the exact artifact identity, including descriptions. It is not an
// admission digest or a normalized semantic wire identity. No self-hash occurs.
outputs.set("generation.json", `${JSON.stringify({
  schemaArtifactSha256: hash(schemaBytes),
  ...(selectedResponses === undefined ? {} : {
    selection: { direction: "Responses", roots: selectedResponses, format: "commonjs" },
  }),
  generator,
  bundledLicenses,
  bundledPackages,
  outputs: members,
}, null, 2)}\n`);

if (check) {
  const observed = (await readdir(destination)).sort();
  const expected = [...outputs.keys()].sort();
  if (JSON.stringify(observed) !== JSON.stringify(expected)) throw new Error("generated output inventory differs");
  for (const [file, content] of outputs) {
    if (!Buffer.from(content).equals(await readFile(path.join(destination, file)))) {
      throw new Error(`generated output differs: ${file}`);
    }
  }
} else {
  await mkdir(destination, { recursive: true });
  if ((await readdir(destination)).length !== 0) throw new Error("output directory must be empty; use --check to compare");
  for (const [file, content] of outputs) await writeFile(path.join(destination, file), content);
}
