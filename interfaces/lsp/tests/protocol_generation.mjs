// Development-only regression runner. Exercise the real generator process.
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { createRequire } from "node:module";
import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { checkRecursiveTypeSharing, checkTypeSharing } from "./protocol_type_sharing.mjs";
import { assertMeaningfulDeclarationNames } from "../tools/declaration-names.mjs";

const here = path.dirname(fileURLToPath(import.meta.url));
const tools = path.resolve(here, "../tools");
if (process.argv.length !== 4) {
  throw new Error("usage: node tests/protocol_generation.mjs <actual-rust-schema-artifact.json> <actual-worker-report.json>");
}
const source = JSON.parse(readFileSync(path.resolve(process.argv[2]), "utf8"));
const workerReport = JSON.parse(readFileSync(path.resolve(process.argv[3]), "utf8"));
const profileFixtureBytes = readFileSync(path.join(here, "fixtures/cross-leaf.v1/effective-profile-wire.json"));
const profileFixture = JSON.parse(profileFixtureBytes);
const home = mkdtempSync(path.join(os.tmpdir(), "pale-lsp-generation-tests-"));
const observations = [];

function invoke(name, args, succeeds, expectedFailure) {
  const result = spawnSync(process.execPath, [path.join(tools, "generate.mjs"), ...args], {
    cwd: tools,
    encoding: "utf8",
    timeout: 60_000,
  });
  writeFileSync(path.join(home, `${name}.stdout`), result.stdout, { flag: "wx" });
  writeFileSync(path.join(home, `${name}.stderr`), result.stderr, { flag: "wx" });
  observations.push({ name, status: result.status, signal: result.signal });
  assert.equal(result.error, undefined, `${name}: generator could not complete`);
  assert.equal(result.signal, null, `${name}: generator was terminated`);
  assert(Number.isInteger(result.status), `${name}: generator has no exit status`);
  assert.equal(result.status === 0, succeeds, `${name}: ${result.stderr}`);
  if (!succeeds) assert.match(result.stderr, expectedFailure, `${name}: wrong rejection class`);
}

// Every hostile case is a mutation of actual owner output, not a copied parser.
const firstRequest = Object.keys(source.schemas.Requests.properties).sort()[0];
for (const [name, mutate, expectedFailure] of [
  ["external-reference", (value) => {
    value.schemas.Requests.properties[firstRequest] = { $ref: "https://invalid.example/private-schema.json" };
  }, /non-local schema reference/],
  ["unknown-keyword", (value) => {
    value.schemas.Requests.properties[firstRequest].unqualifiedKeyword = true;
  }, /strict mode: unknown keyword/],
  ["unknown-format", (value) => {
    value.schemas.Requests.properties[firstRequest] = { type: "string", format: "unqualified-format" };
  }, /unqualified schema format/],
  ["unqualified-bundled-helper", (value) => {
    // Object equality earns Ajv's deep-equality helper through the real compiler.
    value.schemas.Requests.properties[firstRequest] = {
      type: "object", const: { fixture: "bundled-package-boundary" },
    };
  }, /unqualified bundled helper input: .*fast-deep-equal/],
]) {
  const value = structuredClone(source);
  mutate(value);
  const input = path.join(home, `${name}.json`);
  writeFileSync(input, `${JSON.stringify(value, null, 2)}\n`, { flag: "wx" });
  invoke(name, [input, path.join(home, `${name}-output`)], false, expectedFailure);
}

const input = path.join(home, "actual.json");
const output = path.join(home, "generated");
writeFileSync(input, `${JSON.stringify(source, null, 2)}\n`, { flag: "wx" });
invoke("generate", [input, output], true);
for (const file of readdirSync(output).filter(file => /\.(?:ts|d\.mts|d\.cts)$/.test(file))) {
  assert.doesNotThrow(
    () => assertMeaningfulDeclarationNames(readFileSync(path.join(output, file), "utf8"), file),
    file,
  );
}
assert.throws(
  () => assertMeaningfulDeclarationNames(
    "export interface ProfileInspectionContext {}\nexport interface ProfileInspectionContext2 {}\n",
    "counter-fixture.ts",
  ),
  /numeric collision counter: ProfileInspectionContext2/,
);
assert.throws(
  () => assertMeaningfulDeclarationNames(
    "export interface AttributableAnswerV1 {}\nexport interface AttributableAnswerV114 {}\n",
    "version-counter-fixture.ts",
  ),
  /meaningless collision name AttributableAnswerV114/,
);
assert.doesNotThrow(() => assertMeaningfulDeclarationNames(
  "export interface Sha256DigestV1 {}\nexport interface Http2PolicyV1 {}\n",
  "semantic-number-fixture.ts",
));
const requireTool = createRequire(path.join(tools, "package.json"));
const generation = JSON.parse(readFileSync(path.join(output, "generation.json"), "utf8"));
const expectedLicenses = {};
for (const name of ["ajv", "ajv-formats"]) {
  const manifest = requireTool.resolve(`${name}/package.json`);
  const metadata = JSON.parse(readFileSync(manifest, "utf8"));
  const bytes = readFileSync(path.join(path.dirname(manifest), "LICENSE"));
  expectedLicenses[name] = {
    version: metadata.version,
    licenseSha256: createHash("sha256").update(bytes).digest("hex"),
  };
  for (const file of ["requests.mjs", "responses.mjs"]) {
    assert(readFileSync(path.join(output, file)).includes(bytes), `${file}: missing exact ${name} license`);
  }
}
assert.deepEqual(generation.bundledLicenses, expectedLicenses);
assert.deepEqual(generation.bundledPackages, {
  "requests.mjs": ["ajv", "ajv-formats"],
  "responses.mjs": ["ajv", "ajv-formats"],
});
// A downstream bundle must retain the full license text, even when minified.
const { build } = await import(pathToFileURL(requireTool.resolve("esbuild")));
for (const file of ["requests.mjs", "responses.mjs"]) {
  const bundled = await build({ entryPoints: [path.join(output, file)], bundle: true,
    format: "esm", platform: "neutral", minify: true, legalComments: "inline", write: false });
  for (const name of Object.keys(expectedLicenses)) {
    const manifest = requireTool.resolve(`${name}/package.json`);
    const bytes = readFileSync(path.join(path.dirname(manifest), "LICENSE"));
    assert(Buffer.from(bundled.outputFiles[0].contents).includes(bytes), `${file}: bundling lost ${name} notice`);
  }
}
const requests = await import(pathToFileURL(path.join(output, "requests.mjs")));
const responses = await import(pathToFileURL(path.join(output, "responses.mjs")));
const wireObservations = [];
const serializerObservations = [];
function validateWire(schema, value, origin, observations = wireObservations) {
  const inRequests = Object.hasOwn(source.schemas.Requests.properties, schema);
  const inResponses = Object.hasOwn(source.schemas.Responses.properties, schema);
  assert.notEqual(inRequests, inResponses, `${schema}: expected one owning wire direction`);
  const validate = (inRequests ? requests : responses)[`validate${schema}`];
  assert.equal(typeof validate, "function", `${schema}: missing generated check`);
  assert.equal(validate(value), true, `${origin}: ${schema} rejected an actual Rust value`);
  observations.push({ schema, origin });
}

// The existing Rust report distinguishes real worker results from explicit
// serializer-only boundaries. Neither category is generated from a schema.
assert(Array.isArray(workerReport) && workerReport.length > 0, "expected actual worker values");
const coordinateLocations = {
  EffectiveProfileResponse: "editor",
  ConfigurationImprovementResponse: "baseline",
};
const serializerCases = [];
for (const [index, value] of workerReport.entries()) {
  if (value.origin === undefined) {
    validateWire(value.schema, value.value, `retained-worker case ${index}`);
  } else {
    assert.equal(value.origin, "serializer-only-u64-coordinate");
    assert(Object.hasOwn(coordinateLocations, value.schema));
    validateWire(value.schema, value.value, `Rust serializer-only case ${index}`, serializerObservations);
    const coordinate = value.value[coordinateLocations[value.schema]];
    assert.equal(coordinate.generation, coordinate.documentRevision);
    serializerCases.push(value);
  }
}
for (const schema of Object.keys(coordinateLocations)) {
  assert.deepEqual(serializerCases.filter(value => value.schema === schema)
    .map(value => value.value[coordinateLocations[schema]].generation),
  ["0", "9007199254740993", "18446744073709551615"]);
}

// Mutate actual serialized boundary products. Counts are output grammar only;
// these values do not assert a worker admitted an editor at u64::MAX.
const decimalObservations = [];
for (const schema of Object.keys(coordinateLocations)) {
  const original = serializerCases.find(value => value.schema === schema).value;
  for (const field of ["generation", "documentRevision"]) {
    for (const invalid of [
      "18446744073709551616", "9".repeat(1_000_000), "", "01", "+1", "-1", "1.0", "1e0",
      "1\n", "1\r", "1\u2028", "1\u2029", "1\r\n", "1 ", "1\t", " 1", "١", 1, null,
    ]) {
      const value = structuredClone(original);
      value[coordinateLocations[schema]][field] = invalid;
      assert.equal(responses[`validate${schema}`](value), false, `${schema}.${field}: noncanonical counter`);
      decimalObservations.push({ schema, field, inputType: typeof invalid,
        input: typeof invalid === "string" && invalid.length > 100 ? "million-digit input" : invalid });
    }
  }
}
for (const [key, schema] of [
  ["profileRequest", "EffectiveProfileRequest"],
  ["currentProfileResponse", "EffectiveProfileResponse"],
  ["staleProfileResponse", "EffectiveProfileResponse"],
  ["notAdmittedProfileRequest", "EffectiveProfileRequest"],
  ["notAdmittedProfileResponse", "EffectiveProfileResponse"],
  ["explanationRequest", "EffectiveProfileExplanationRequest"],
  ["explanationResponse", "EffectiveProfileExplanationResponse"],
]) validateWire(schema, profileFixture[key], `Rust fixture: ${key}`);
for (const [key, value] of Object.entries(profileFixture.improvement)) {
  validateWire(key.endsWith("Request") ? "ConfigurationImprovementRequest" : "ConfigurationImprovementResponse", value, `Rust improvement fixture: ${key}`);
}

// The existing fixture omits its own digest. Restore only that field using the
// same byte-hash rule tested by the Rust advertisement owner and the client.
// This is fixture reconstruction, not a newly emitted server advertisement.
const advertisement = structuredClone(profileFixture.protocolAdvertisementWithoutWireDigest.paleWorkspace);
assert.equal(Object.hasOwn(advertisement, "wireFixtureSha256"), false);
advertisement.wireFixtureSha256 = createHash("sha256").update(profileFixtureBytes).digest("hex");
const envelopeCases = [
  ["PaleWorkspaceCapabilities", advertisement, "Rust advertisement fixture with its byte digest restored"],
  ...Object.entries(profileFixture.improvement).filter(([key]) => key.endsWith("Response"))
    .map(([key, value]) => ["ConfigurationImprovementResponse", value, `Rust improvement fixture: ${key}`]),
];
assert.deepEqual(envelopeCases.filter(([schema]) => schema === "ConfigurationImprovementResponse")
  .map(([, value]) => value.state).sort(), ["explained", "previewed", "terminal"]);
const closedEnvelopeObservations = [];
for (const [schema, original, origin] of envelopeCases) {
  const validate = responses[`validate${schema}`];
  assert.equal(validate(original), true, `${origin}: legitimate envelope was rejected`);
  assert.equal(Object.hasOwn(original, "unrecognizedField"), false);
  const mutated = structuredClone(original);
  mutated.unrecognizedField = true;
  assert.equal(validate(mutated), false, `${origin}: unexpected outer field was accepted`);
  closedEnvelopeObservations.push({ schema, origin, ...(original.state ? { state: original.state } : {}) });
}

// Mutate actual serialized products, not schema-derived examples. These nine
// nonflattened record shapes occur in the existing Rust-produced wire fixture.
const closedRecordObservations = [];
for (const [record, schema, original, location] of [
  ["EffectiveProfileResponse", "EffectiveProfileResponse", profileFixture.currentProfileResponse, []],
  ["EffectiveProfileCoordinates", "EffectiveProfileResponse", profileFixture.currentProfileResponse, ["coordinates"]],
  ["EditorCoordinateOutput", "EffectiveProfileResponse", profileFixture.currentProfileResponse, ["editor"]],
  ["EffectiveProfilePage", "EffectiveProfileResponse", profileFixture.currentProfileResponse, ["page"]],
  ["EffectiveProfileExplanationResponse", "EffectiveProfileExplanationResponse", profileFixture.explanationResponse, []],
  ["EffectiveProfileExplanation", "EffectiveProfileExplanationResponse", profileFixture.explanationResponse, ["explanation"]],
  ["ExpectedProfileCoordinateOutput", "ConfigurationImprovementResponse", profileFixture.improvement.explainedResponse, ["baseline"]],
  ["ImprovementPreviewCoordinate", "ConfigurationImprovementResponse", profileFixture.improvement.previewedResponse, ["coordinate"]],
  ["ConfigurationImprovementPreviewProjection", "ConfigurationImprovementResponse", profileFixture.improvement.previewedResponse, ["preview"]],
]) {
  const mutated = structuredClone(original);
  const target = location.reduce((value, member) => value[member], mutated);
  assert(target !== null && typeof target === "object" && !Array.isArray(target), record);
  assert(!Object.hasOwn(target, "unrecognizedField"), record);
  target.unrecognizedField = true;
  assert.equal(responses[`validate${schema}`](mutated), false, `${record}: unexpected field was accepted`);
  closedRecordObservations.push({ record, schema, location });
}

const original = new Map(readdirSync(output).map((file) => [file, readFileSync(path.join(output, file))]));
const selectedRoots = [
  "AnalysisPublication",
  "ConfigurationImprovementResponse",
  "EffectiveProfileExplanationResponse",
  "EffectiveProfileResponse",
  "PaleWorkspaceCapabilities",
  "ProfileContextsResponse",
];
const selectedRootsArgument = selectedRoots.join(",");
const selectedOutput = path.join(home, "selected-review");
const selectedReplay = path.join(home, "selected-review-replay");
invoke("selected-response", ["--responses-cjs", selectedRootsArgument, input, selectedOutput], true);
invoke("selected-response-replay", ["--responses-cjs", [...selectedRoots].reverse().join(","), input, selectedReplay], true);
const typeSharingObservations = checkTypeSharing(requireTool, output, selectedOutput);

// Generation-only fixture: equal fields are not shared identity. A recursive
// root must reuse itself rather than a copied declaration with a suffix.
const sharingSource = structuredClone(source);
sharingSource.schemas.Responses.properties.ReferenceSharingProbe = { $ref: "#/$defs/ReferenceSharingProbe" };
sharingSource.schemas.Responses.required.push("ReferenceSharingProbe");
const sharingDefinitions = sharingSource.schemas.Responses.$defs;
sharingDefinitions.FirstContext = structuredClone(sharingDefinitions.ProfileInspectionContext);
sharingDefinitions.SecondContext = structuredClone(sharingDefinitions.ProfileInspectionContext);
sharingDefinitions.ReferenceSharingProbe = {
  type: "object",
  additionalProperties: false,
  required: ["next", "first", "second"],
  properties: {
    next: { anyOf: [{ $ref: "#/$defs/ReferenceSharingProbe" }, { type: "null" }] },
    first: { $ref: "#/$defs/FirstContext", description: "First semantic owner." },
    second: { $ref: "#/$defs/SecondContext", description: "Second semantic owner." },
  },
};
const sharingInput = path.join(home, "reference-sharing.json");
const sharingOutput = path.join(home, "reference-sharing");
writeFileSync(sharingInput, `${JSON.stringify(sharingSource, null, 2)}\n`, { flag: "wx" });
invoke("recursive-type-sharing", ["--responses-cjs", "ReferenceSharingProbe", sharingInput, sharingOutput], true);
checkRecursiveTypeSharing(requireTool, sharingOutput);
const sharingValidator = requireTool(path.join(sharingOutput, "responses.cjs")).validateReferenceSharingProbe;
const context = profileFixture.improvement.explainedResponse.context;
const sharingValue = { next: null, first: context, second: context };
assert.equal(sharingValidator({ ...sharingValue, next: sharingValue }), true);
assert.equal(sharingValidator({ ...sharingValue, next: { ...sharingValue, first: {} } }), false);
const selectedFiles = [...selectedRoots.map(root => `${root}.ts`), "responses.cjs", "responses.d.cts", "generation.json"];
assert.deepEqual(readdirSync(selectedOutput).sort(), [...selectedFiles].sort());
assert.deepEqual(readdirSync(selectedReplay).sort(), [...selectedFiles].sort());
for (const file of selectedFiles) {
  assert(readFileSync(path.join(selectedOutput, file)).equals(readFileSync(path.join(selectedReplay, file))), file);
}
const selectedManifest = JSON.parse(readFileSync(path.join(selectedOutput, "generation.json")));
assert.deepEqual(selectedManifest.selection, { direction: "Responses", roots: selectedRoots, format: "commonjs" });
assert.deepEqual(selectedManifest.bundledLicenses, expectedLicenses);
assert.deepEqual(selectedManifest.bundledPackages, { "responses.cjs": ["ajv", "ajv-formats"] });
const selected = requireTool(path.join(selectedOutput, "responses.cjs"));
assert.deepEqual(Object.keys(selected), selectedRoots.map(root => `validate${root}`));
const selectedObservations = [];
for (const [key, value] of Object.entries(profileFixture.improvement)) {
  if (!key.endsWith("Response")) continue;
  assert.equal(selected.validateConfigurationImprovementResponse(value), true, key);
  assert.equal(selected.validateConfigurationImprovementResponse({ ...value, unrecognizedField: true }), false, key);
  selectedObservations.push({ origin: `Rust improvement fixture: ${key}`, state: value.state });
}
for (const row of serializerCases.filter(row => row.schema === "ConfigurationImprovementResponse")) {
  assert.equal(selected.validateConfigurationImprovementResponse(row.value), true);
  selectedObservations.push({ origin: row.origin, generation: row.value.baseline.generation });
}
const selectedBundle = await build({ entryPoints: [path.join(selectedOutput, "responses.cjs")], bundle: true,
  format: "cjs", platform: "node", minify: true, legalComments: "inline", write: false });
for (const name of Object.keys(expectedLicenses)) {
  const manifest = requireTool.resolve(`${name}/package.json`);
  const bytes = readFileSync(path.join(path.dirname(manifest), "LICENSE"));
  assert(readFileSync(path.join(selectedOutput, "responses.cjs")).includes(bytes));
  assert(Buffer.from(selectedBundle.outputFiles[0].contents).includes(bytes));
}
invoke("selected-missing-alias", ["--responses-cjs"], false, /requires response aliases/);
invoke("selected-empty-alias", ["--responses-cjs", "EffectiveProfileResponse,,ProfileContextsResponse", input,
  path.join(home, "invalid-empty")], false, /requires nonempty comma-separated response aliases/);
for (const [label, root] of [["unknown", "UnknownResponse"], ["request", firstRequest]]) {
  invoke(`selected-${label}-alias`, ["--responses-cjs", root, input, path.join(home, `invalid-${label}`)], false,
    /not an exported response alias/);
}
invoke("selected-nonempty", ["--responses-cjs", selectedRootsArgument, input, selectedOutput], false, /output directory must be empty/);
for (const file of selectedFiles) {
  assert(readFileSync(path.join(selectedOutput, file)).equals(readFileSync(path.join(selectedReplay, file))), file);
}
invoke("selected-check", ["--check", "--responses-cjs", selectedRootsArgument, input, selectedOutput], true);
writeFileSync(path.join(selectedOutput, "unexpected.txt"), "not a generated member\n", { flag: "wx" });
invoke("selected-check-extra", ["--check", "--responses-cjs", selectedRootsArgument, input, selectedOutput], false,
  /generated output inventory differs/);

invoke("nonempty-output", [input, output], false, /output directory must be empty/);
for (const [file, bytes] of original) assert(bytes.equals(readFileSync(path.join(output, file))), file);
assert.deepEqual(readdirSync(output).sort(), [...original.keys()].sort());
invoke("check", ["--check", input, output], true);
writeFileSync(path.join(output, "unexpected.txt"), "not a generated member\n", { flag: "wx" });
invoke("check-extra-file", ["--check", input, output], false, /generated output inventory differs/);
writeFileSync(path.join(home, "observations.json"), `${JSON.stringify(observations, null, 2)}\n`, { flag: "wx" });
writeFileSync(path.join(home, "wire-observations.json"), `${JSON.stringify(wireObservations, null, 2)}\n`, { flag: "wx" });
writeFileSync(path.join(home, "serializer-observations.json"), `${JSON.stringify(serializerObservations, null, 2)}\n`, { flag: "wx" });
writeFileSync(path.join(home, "decimal-observations.json"), `${JSON.stringify(decimalObservations, null, 2)}\n`, { flag: "wx" });
writeFileSync(path.join(home, "closed-record-observations.json"), `${JSON.stringify(closedRecordObservations, null, 2)}\n`, { flag: "wx" });
writeFileSync(path.join(home, "closed-envelope-observations.json"), `${JSON.stringify(closedEnvelopeObservations, null, 2)}\n`, { flag: "wx" });
writeFileSync(path.join(home, "selected-observations.json"), `${JSON.stringify(selectedObservations, null, 2)}\n`, { flag: "wx" });
console.log(JSON.stringify({ evidenceDirectory: home, observations, wireObservations,
  serializerObservations, decimalObservations, closedRecordObservations, closedEnvelopeObservations,
  selectedObservations, typeSharingObservations }, null, 2));
