const assert = require("node:assert/strict");
const { createHash } = require("node:crypto");
const { cpSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } = require("node:fs");
const { spawnSync } = require("node:child_process");
const { tmpdir } = require("node:os");
const path = require("node:path");
const test = require("node:test");

test("the client ships only its selected, byte-qualified response decoders", () => {
  const root = path.resolve(__dirname, "..");
  const source = path.join(root, "src/generated/protocol");
  const built = path.join(root, "dist/generated/protocol");
  const manifest = JSON.parse(readFileSync(path.join(source, "generation.json")));
  const roots = [
    "AnalysisPublication",
    "ConfigurationImprovementResponse",
    "EffectiveProfileExplanationResponse",
    "EffectiveProfileResponse",
    "PaleWorkspaceCapabilities",
    "ProfileContextsResponse",
  ];
  assert.deepEqual(manifest.selection, { direction: "Responses", roots, format: "commonjs" });
  assert.deepEqual(manifest.outputs.map(member => member.path).sort(),
    [...roots.map(root => `${root}.ts`), "responses.cjs", "responses.d.cts"].sort());
  for (const member of manifest.outputs) {
    const bytes = readFileSync(path.join(source, member.path));
    assert.equal(createHash("sha256").update(bytes).digest("hex"), member.sha256, member.path);
  }
  for (const file of ["responses.cjs", "responses.d.cts", "generation.json"]) {
    assert(readFileSync(path.join(source, file)).equals(readFileSync(path.join(built, file))), file);
  }
  assert.deepEqual(readdirSync(built).sort(), [
    ...roots.flatMap(root => [`${root}.js`, `${root}.d.ts`, `${root}.d.ts.map`]),
    "responses.cjs",
    "responses.d.cts",
    "generation.json",
  ].sort());
  const decoder = require(path.join(built, "responses.cjs"));
  assert.deepEqual(Object.keys(decoder), roots.map(root => `validate${root}`));
  const fixture = require("./fixtures/cross-leaf.v1/effective-profile-wire.json");
  for (const [name, value] of Object.entries(fixture.improvement)) {
    if (name.endsWith("Response")) {
      assert.equal(decoder.validateConfigurationImprovementResponse(value), true, name);
      assert.equal(decoder.validateConfigurationImprovementResponse({ ...value, unrecognizedField: true }), false, name);
    }
  }
  for (const [validator, value] of [
    ["validateAnalysisPublication", {
      editor: { generation: "1", documentRevision: "2" }, admission: "admitted",
    }],
    ["validateEffectiveProfileResponse", fixture.currentProfileResponse],
    ["validateEffectiveProfileExplanationResponse", fixture.explanationResponse],
    ["validateProfileContextsResponse", {
      editor: { generation: "1", documentRevision: "2" },
      contexts: [{ projectId: "project", executionWorkspaceId: "workspace" }],
    }],
  ]) {
    assert.equal(decoder[validator](value), true, validator);
    assert.equal(decoder[validator]({ ...value, unrecognizedField: true }), false, validator);
  }
  const advertisement = structuredClone(fixture.protocolAdvertisementWithoutWireDigest.paleWorkspace);
  advertisement.wireFixtureSha256 = createHash("sha256")
    .update(readFileSync(path.join(__dirname, "fixtures/cross-leaf.v1/effective-profile-wire.json")))
    .digest("hex");
  assert.equal(decoder.validatePaleWorkspaceCapabilities(advertisement), true);
  assert.equal(decoder.validatePaleWorkspaceCapabilities({ ...advertisement, unknown: true }), false);

  for (const [validator, original, path] of [
    ["validateEffectiveProfileResponse", fixture.currentProfileResponse, ["coordinates"]],
    ["validateEffectiveProfileExplanationResponse", fixture.explanationResponse, ["coordinates"]],
    ["validateConfigurationImprovementResponse", fixture.improvement.explainedResponse, ["baseline"]],
  ]) {
    const withoutPrivateInstance = structuredClone(original);
    const coordinate = path.reduce((value, key) => value[key], withoutPrivateInstance);
    delete coordinate.instanceDigest;
    assert.equal(decoder[validator](withoutPrivateInstance), false, `${validator} requires instanceDigest`);
  }
});

test("the build rejects stale generated request output before compiling", (t) => {
  const root = path.resolve(__dirname, "..");
  const temporary = mkdtempSync(path.join(tmpdir(), "pale-vscode-stale-output-"));
  t.after(() => rmSync(temporary, { recursive: true, force: true }));
  // Copy real local inputs. No compiler dependency is needed because this
  // boundary must reject the unrecognized output before invoking TypeScript.
  for (const file of ["package.json", "tools/build.mjs", "src/generated/protocol"]) {
    mkdirSync(path.dirname(path.join(temporary, file)), { recursive: true });
    cpSync(path.join(root, file), path.join(temporary, file), { recursive: true });
  }
  const stale = path.join(temporary, "dist/generated/protocol/requests.mjs");
  mkdirSync(path.dirname(stale), { recursive: true });
  writeFileSync(stale, "// stale output must not ship\n", { flag: "wx" });
  const result = spawnSync(process.execPath, [path.join(temporary, "tools/build.mjs")], {
    cwd: temporary, encoding: "utf8", timeout: 60_000,
  });
  assert.equal(result.error, undefined);
  assert.equal(result.signal, null);
  assert.equal(result.status, 1);
  assert.match(result.stderr, /unexpected generated build output inventory/);
  assert.equal(readFileSync(stale, "utf8"), "// stale output must not ship\n");
});
