const assert = require("node:assert/strict");
const Module = require("node:module");
const path = require("node:path");
const test = require("node:test");

class Uri {
  constructor(pathname) {
    this.scheme = "file";
    this.authority = "";
    this.path = pathname;
    this.query = "";
    this.fragment = "";
  }

  toString() {
    return `file://${this.path}`;
  }

  static joinPath(root, source) {
    return new Uri(path.posix.resolve(root.path, source));
  }
}

test("configured sources retain exact bytes without opening every document", async () => {
  const bytes = new Map([
    ["/workspace/module.toml", Buffer.from("[module]\n")],
    ["/workspace/guide.md", Buffer.from("# Guide\n")],
    ["/workspace/review.md", Buffer.from("+++\r\ndescription = 'Review λ.'\r\n+++\r\n# Review\r\n")],
  ]);
  let opened = 0;
  const vscode = {
    Uri,
    workspace: {
      fs: {
        readFile: async (uri) => bytes.get(uri.path),
      },
      openTextDocument: async () => {
        opened += 1;
      },
    },
  };
  const originalLoad = Module._load;
  Module._load = function load(request, parent, isMain) {
    return request === "vscode" ? vscode : originalLoad.call(this, request, parent, isMain);
  };
  const { captureConfiguredSources, resolveConfiguredSources } = require("../dist/sourceCapture.js");
  Module._load = originalLoad;

  const resolved = resolveConfiguredSources(new Uri("/workspace"), [
    {
      path: "module.toml",
      languageId: "pale-toml",
      role: "workspace-module",
    },
    { path: "guide.md", languageId: "pale", role: "package-text" },
    { path: "review.md", languageId: "pale-page", role: "package-page" },
  ]);
  const documents = await captureConfiguredSources(resolved);

  assert.equal(opened, 0);
  assert.deepEqual(
    documents.map(({ languageId, role, text }) => ({ languageId, role, text })),
    [
      { languageId: "pale-toml", role: "workspace-module", text: "[module]\n" },
      { languageId: "pale", role: "package-text", text: "# Guide\n" },
      { languageId: "pale-page", role: "package-page", text: "+++\r\ndescription = 'Review λ.'\r\n+++\r\n# Review\r\n" },
    ],
  );
  assert.deepEqual([...resolved.languageByUri], [
    ["file:///workspace/module.toml", "pale-toml"],
    ["file:///workspace/guide.md", "pale"],
    ["file:///workspace/review.md", "pale-page"],
  ]);
});

test("an opened configured document receives its explicit lexical identity", async () => {
  const assignments = [];
  const vscode = {
    Uri,
    languages: {
      setTextDocumentLanguage: async (document, languageId) => {
        assignments.push(languageId);
        document.languageId = languageId;
        return document;
      },
    },
  };
  const originalLoad = Module._load;
  Module._load = function load(request, parent, isMain) {
    return request === "vscode" ? vscode : originalLoad.call(this, request, parent, isMain);
  };
  delete require.cache[require.resolve("../dist/sourceCapture.js")];
  const { applyConfiguredLanguage } = require("../dist/sourceCapture.js");
  Module._load = originalLoad;

  const document = {
    uri: new Uri("/workspace/module.toml"),
    languageId: "toml",
  };
  const languages = new Map([[document.uri.toString(), "pale-toml"]]);
  await applyConfiguredLanguage(document, languages);
  await applyConfiguredLanguage(document, languages);
  await applyConfiguredLanguage(
    { uri: new Uri("/workspace/other.toml"), languageId: "toml" },
    languages,
  );

  assert.deepEqual(assignments, ["pale-toml"]);
});

test("configured source capture rejects duplicate and escaping paths", async () => {
  const vscode = {
    Uri,
    workspace: {
      fs: { readFile: async () => Buffer.from("value = 1\n") },
    },
  };
  const originalLoad = Module._load;
  Module._load = function load(request, parent, isMain) {
    return request === "vscode" ? vscode : originalLoad.call(this, request, parent, isMain);
  };
  delete require.cache[require.resolve("../dist/sourceCapture.js")];
  const { resolveConfiguredSources } = require("../dist/sourceCapture.js");
  Module._load = originalLoad;
  const source = {
    path: "module.toml",
    languageId: "pale-toml",
    role: "workspace-module",
  };

  assert.throws(
    () => resolveConfiguredSources(new Uri("/workspace"), [source, source]),
    /configured more than once/,
  );
  assert.throws(
    () => resolveConfiguredSources(new Uri("/workspace"), [{ ...source, path: "../escape.toml" }]),
    /escapes the workspace folder/,
  );
  assert.equal(
    resolveConfiguredSources(new Uri("/workspace"), [
      { ...source, path: "config/[draft]*?.toml" },
    ]).sources[0].watchPattern,
    "config/[[]draft[]][*][?].toml",
  );
});
