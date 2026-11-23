const assert = require("node:assert/strict");
const { execFileSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const test = require("node:test");
const { DOCUMENT_ROLES } = require("../dist/protocol.js");

const clientRoot = path.resolve(__dirname, "..");

test("the unpublished client has the repository version and license", () => {
  const manifest = JSON.parse(fs.readFileSync(path.join(clientRoot, "package.json"), "utf8"));
  assert.equal(manifest.name, "pale-vscode");
  assert.equal(manifest.version, "0.0.1");
  assert.equal(manifest.private, true);
  assert.equal(manifest.license, "AGPL-3.0-or-later");
  assert.deepEqual(Object.keys(manifest.dependencies), ["vscode-languageclient"]);
});

test("the npm packlist contains the complete compiled runtime and legal metadata", () => {
  const cache = fs.mkdtempSync(path.join(os.tmpdir(), "pale-vscode-pack-"));
  try {
    const packed = JSON.parse(
      execFileSync("npm", ["pack", "--dry-run", "--json", "--cache", cache], {
        cwd: clientRoot,
        encoding: "utf8",
      }),
    );
    assert.equal(packed.length, 1);
    const paths = new Set(packed[0].files.map((file) => file.path));
    const manifest = JSON.parse(
      fs.readFileSync(path.join(clientRoot, "package.json"), "utf8"),
    );
    assert.equal(paths.has(manifest.main.replace(/^\.\//u, "")), true);
    for (const entry of fs.readdirSync(path.join(clientRoot, "dist"))) {
      if (entry.endsWith(".js")) {
        assert.equal(paths.has(`dist/${entry}`), true);
      }
    }
    for (const required of [
      "package.json",
      "README.md",
      "LICENSE",
    ]) {
      assert.equal(paths.has(required), true);
    }
    assert.equal([...paths].some((entry) => entry.startsWith("src/")), false);
    assert.equal([...paths].some((entry) => entry.startsWith("tests/")), false);
  } finally {
    fs.rmSync(cache, { recursive: true, force: true });
  }
});

test("the independent client has no parent-checkout dependency or fixture reach", () => {
  const manifest = JSON.parse(fs.readFileSync(path.join(clientRoot, "package.json"), "utf8"));
  const dependencies = { ...manifest.dependencies, ...manifest.devDependencies };
  assert.equal(dependencies["@rtepl/pale-contracts"], "0.0.1");
  for (const coordinate of Object.values(dependencies)) {
    assert.equal(coordinate.startsWith("file:") && coordinate.includes(".."), false);
  }

  const packageLock = fs.readFileSync(path.join(clientRoot, "package-lock.json"), "utf8");
  assert.doesNotMatch(packageLock, /(?:bindings|clients|crates[/]pale-sdk)/);
  const tests = fs
    .readdirSync(path.join(clientRoot, "tests"))
    .filter((entry) => entry.endsWith(".cjs"))
    .map((entry) => fs.readFileSync(path.join(clientRoot, "tests", entry), "utf8"))
    .join("\n");
  for (const ownerRoot of [
    "api",
    "bindings",
    "clients",
    "crates",
    "extensions",
    "interfaces",
    "integrations",
  ]) {
    assert.equal(tests.includes("../../../" + ownerRoot + "/"), false);
  }
});

test("workspace capture is explicitly role and machine configured", () => {
  const manifest = JSON.parse(fs.readFileSync(path.join(clientRoot, "package.json"), "utf8"));
  assert.deepEqual(manifest.activationEvents, ["workspaceContains:pale-workspace.toml"]);
  assert.deepEqual(manifest.extensionKind, ["workspace"]);
  assert.deepEqual(manifest.capabilities, {
    untrustedWorkspaces: {
      supported: false,
      description:
        "Pale starts a configured language-server executable and reads explicitly selected workspace sources.",
    },
    virtualWorkspaces: {
      supported: false,
      description: "The native Pale language-server process requires a host filesystem workspace.",
    },
  });
  const properties = manifest.contributes.configuration.properties;
  assert.equal(properties["pale.server.path"].scope, "resource");
  assert.equal(properties["pale.server.path"].restricted, true);
  for (const key of [
    "pale.workspace.rootManifest",
    "pale.workspace.machineId",
    "pale.workspace.sources",
  ]) {
    assert.equal(properties[key].scope, "resource");
  }
  assert.equal(properties["pale.workspace.machineId"].type, "string");
  assert.deepEqual(properties["pale.workspace.sources"].items.required, [
    "path",
    "languageId",
    "role",
  ]);
  assert.deepEqual(
    properties["pale.workspace.sources"].items.properties.role.enum,
    DOCUMENT_ROLES,
  );
});

test("configuration review commands declare phase-specific enablement", () => {
  const manifest = JSON.parse(fs.readFileSync(path.join(clientRoot, "package.json"), "utf8"));
  const commands = new Map(
    manifest.contributes.commands.map((command) => [command.command, command]),
  );
  assert.equal(
    commands.get("pale.improvement.begin").enablement,
    "pale.configurationReview.canBegin",
  );
  assert.equal(
    commands.get("pale.improvement.evaluate").enablement,
    "pale.configurationReview.canEvaluate",
  );
  assert.equal(
    commands.get("pale.improvement.inspect").enablement,
    "pale.configurationReview.canInspect",
  );
  assert.equal(
    commands.get("pale.improvement.finish").enablement,
    "pale.configurationReview.canFinish",
  );
});
