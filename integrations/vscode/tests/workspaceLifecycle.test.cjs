const assert = require("node:assert/strict");
const Module = require("node:module");
const test = require("node:test");
const { PassThrough } = require("node:stream");
const jsonrpc = require("vscode-jsonrpc/node");

const protocol = require("../dist/protocol.js");

class Disposable {
  constructor(dispose = () => {}) {
    this.dispose = dispose;
  }
}

class EventEmitter {
  constructor() {
    this.listeners = new Set();
    this.event = (listener) => {
      this.listeners.add(listener);
      return new Disposable(() => this.listeners.delete(listener));
    };
  }

  fire(value) {
    for (const listener of this.listeners) {
      listener(value);
    }
  }

  dispose() {
    this.listeners.clear();
  }
}

class Uri {
  constructor(pathname) {
    this.scheme = "file";
    this.authority = "";
    this.path = pathname;
    this.fsPath = pathname;
    this.query = "";
    this.fragment = "";
  }

  toString() {
    return `file://${this.path}`;
  }

  static joinPath(root, child) {
    return new Uri(`${root.path}/${child}`);
  }
}

class RelativePattern {
  constructor(base, pattern) {
    this.base = base;
    this.pattern = pattern;
  }
}

class FileSystemWatcher {
  constructor(pattern) {
    this.pattern = pattern;
    this.created = new EventEmitter();
    this.changed = new EventEmitter();
    this.deleted = new EventEmitter();
    this.onDidCreate = this.created.event;
    this.onDidChange = this.changed.event;
    this.onDidDelete = this.deleted.event;
  }

  dispose() {
    this.created.dispose();
    this.changed.dispose();
    this.deleted.dispose();
  }
}

class TreeItem {
  constructor(label, collapsibleState) {
    this.label = label;
    this.collapsibleState = collapsibleState;
  }
}

class ThemeIcon {
  constructor(id) {
    this.id = id;
  }
}

class MarkdownString {
  constructor() {
    this.value = "";
  }

  appendText(text) {
    this.value += text;
  }
}

class CancellationTokenSource {
  constructor() {
    this.token = { isCancellationRequested: false };
  }

  cancel() {
    this.token.isCancellationRequested = true;
  }

  dispose() {}
}

const workspaceFoldersChanged = new EventEmitter();
const configurationChanged = new EventEmitter();
const documentOpened = new EventEmitter();
const activeEditorChanged = new EventEmitter();
const commands = new Map();
const commandContexts = new Map();
const errors = [];
const watchers = [];
const outputChannels = [];
const fileBytes = new Map();
let treeProvider;

const folderA = { name: "alpha", index: 0, uri: new Uri("/work/alpha") };
const folderB = { name: "beta", index: 1, uri: new Uri("/work/beta") };
const folderC = { name: "gamma", index: 1, uri: new Uri("/work/gamma") };
const folderWithGlobSyntax = {
  name: "bracketed",
  index: 0,
  uri: new Uri("/work/team[one]"),
};
const settings = new Map([
  [folderA.uri.toString(), { path: "alpha-lsp", manifest: "alpha.toml" }],
  [folderB.uri.toString(), { path: "beta-lsp", manifest: "beta.toml" }],
  [folderC.uri.toString(), { path: "gamma-lsp", manifest: "gamma.toml" }],
  [folderWithGlobSyntax.uri.toString(), { path: "bracketed-lsp", manifest: "bracketed.toml" }],
]);

const vscode = {
  EventEmitter,
  Uri,
  RelativePattern,
  CancellationTokenSource,
  TreeItem,
  ThemeIcon,
  MarkdownString,
  TreeItemCollapsibleState: { None: 0, Collapsed: 1 },
  workspace: {
    workspaceFolders: [folderA, folderB],
    textDocuments: [],
    getConfiguration: (_section, uri) => ({
      get: (key, fallback) => {
        const setting = settings.get(uri.toString());
        if (key === "server.path") return setting?.path ?? fallback;
        if (key === "workspace.rootManifest") return setting?.manifest ?? fallback;
        if (key === "workspace.sources") return setting?.sources ?? [];
        return fallback;
      },
    }),
    getWorkspaceFolder: (uri) =>
      vscode.workspace.workspaceFolders.find((folder) => uri.path.startsWith(`${folder.uri.path}/`)),
    onDidChangeWorkspaceFolders: workspaceFoldersChanged.event,
    onDidChangeConfiguration: configurationChanged.event,
    onDidOpenTextDocument: documentOpened.event,
    createFileSystemWatcher: (pattern) => {
      const watcher = new FileSystemWatcher(pattern);
      watchers.push(watcher);
      return watcher;
    },
    fs: { readFile: async (uri) => fileBytes.get(uri.toString()) ?? Buffer.from("") },
  },
  window: {
    activeTextEditor: { document: { uri: new Uri("/work/beta/current.toml") } },
    registerTreeDataProvider: (_view, provider) => {
      treeProvider = provider;
      return new Disposable();
    },
    onDidChangeActiveTextEditor: activeEditorChanged.event,
    showQuickPick: async (items) => items[0],
    showInputBox: async () => {
      throw new Error("profile selection must not ask the user to type an identity");
    },
    showErrorMessage: async (message) => errors.push(message),
    showInformationMessage: async () => undefined,
    createOutputChannel: (name) => {
      const channel = {
        name,
        value: "",
        visible: false,
        disposed: false,
        replace(value) { this.value = value; },
        show() { this.visible = true; },
        dispose() { this.disposed = true; },
      };
      outputChannels.push(channel);
      return channel;
    },
  },
  commands: {
    registerCommand: (name, handler) => {
      commands.set(name, handler);
      return new Disposable(() => commands.delete(name));
    },
    executeCommand: async (name, key, value) => {
      assert.equal(name, "setContext");
      commandContexts.set(key, value);
    },
  },
  languages: {
    getDiagnostics: () => [],
    setTextDocumentLanguage: async (document) => document,
  },
};

class LanguageClient {
  static instances = [];

  constructor(id, name, serverOptions, clientOptions) {
    this.id = id;
    this.name = name;
    this.serverOptions = serverOptions;
    this.clientOptions = clientOptions;
    this.notifications = new Map();
    this.sentNotifications = [];
    this.sentRequests = [];
    this.stopped = false;
    this.initializeResult = {
      capabilities: {
        experimental: {
          paleWorkspace: {
            protocolId: protocol.PALE_LSP_PROTOCOL_ID,
            packageVersion: protocol.PALE_LSP_PACKAGE_VERSION,
            inventorySha256: protocol.PALE_LSP_PROTOCOL_INVENTORY_SHA256,
            wireFixtureSha256: protocol.PALE_LSP_WIRE_FIXTURE_SHA256,
            methods: protocol.PALE_LSP_METHODS,
            notifications: protocol.PALE_LSP_NOTIFICATIONS,
            cancellation: protocol.PALE_LSP_CANCELLATION,
            documentRoles: protocol.DOCUMENT_ROLES,
          },
        },
      },
    };
    LanguageClient.instances.push(this);
  }

  async start() {}

  async stop() {
    this.stopped = true;
  }

  onNotification(method, handler) {
    this.notifications.set(method, handler);
    return new Disposable(() => this.notifications.delete(method));
  }

  sendRequest(request) {
    const method = typeof request === "string" ? request : request.method;
    this.sentRequests.push(method);
    if (method === protocol.PROFILE_CONTEXTS_METHOD) {
      return Promise.resolve({
        editor: { generation: "1", documentRevision: "0" },
        contexts: [{
          projectId: `project-${this.clientOptions.workspaceFolder.name}`,
          executionWorkspaceId: "default",
        }],
      });
    }
    throw new Error(`unexpected request: ${method}`);
  }

  async sendNotification(method, params) {
    this.sentNotifications.push({ method, params });
  }
}

const originalLoad = Module._load;
Module._load = function load(request, parent, isMain) {
  if (request === "vscode") return vscode;
  if (request === "vscode-languageclient/node") return { LanguageClient, RequestType0: jsonrpc.RequestType0 };
  return originalLoad.call(this, request, parent, isMain);
};
const extension = require("../dist/extension.js");
const { startWorkspaceSession, stopWorkspaceSession } = require("../dist/workspaceSession.js");
Module._load = originalLoad;

const settle = () => new Promise((resolve) => setImmediate(resolve));

test("workspace sessions remain folder-scoped across add remove and resource restart", async () => {
  const context = { subscriptions: [] };
  await extension.activate(context);

  assert.equal(errors.length, 0);
  assert.deepEqual(Object.fromEntries(commandContexts), {
    "pale.configurationReview.canBegin": false,
    "pale.configurationReview.canEvaluate": false,
    "pale.configurationReview.canInspect": false,
    "pale.configurationReview.canFinish": false,
  });
  assert.equal(LanguageClient.instances.length, 2);
  const [alpha, beta] = LanguageClient.instances;
  assert.equal(alpha.serverOptions.command, "alpha-lsp");
  assert.equal(beta.serverOptions.command, "beta-lsp");
  assert.equal(alpha.clientOptions.workspaceFolder, folderA);
  assert.equal(beta.clientOptions.workspaceFolder, folderB);
  assert.equal(alpha.clientOptions.initializationOptions.rootManifest, "alpha.toml");
  assert.equal(beta.clientOptions.initializationOptions.rootManifest, "beta.toml");
  assert.deepEqual(alpha.clientOptions.documentSelector, [
    { language: "pale", scheme: "file", pattern: "/work/alpha/**/*" },
    { language: "pale-toml", scheme: "file", pattern: "/work/alpha/**/*" },
    { language: "pale-page", scheme: "file", pattern: "/work/alpha/**/*" },
  ]);

  const roots = await treeProvider.getChildren();
  assert.deepEqual(
    roots.map((node) => [node.kind, node.sessionKey, node.label]),
    [
      ["workspace", folderA.uri.toString(), "alpha"],
      ["workspace", folderB.uri.toString(), "beta"],
    ],
  );
  assert.equal(
    treeProvider.getTreeItem(roots[0]).id,
    `pale:workspace:${folderA.uri.toString()}`,
  );
  const alphaSections = await treeProvider.getChildren(roots[0]);
  const betaSections = await treeProvider.getChildren(roots[1]);
  assert.notEqual(
    treeProvider.getTreeItem(alphaSections[0]).id,
    treeProvider.getTreeItem(betaSections[0]).id,
  );

  let publicationRefreshes = 0;
  const refreshSubscription = treeProvider.onDidChangeTreeData(() => {
    publicationRefreshes += 1;
  });
  alpha.notifications.get("pale/workspace/analysisPublished")({
    editor: { generation: "1", documentRevision: "1" },
    admission: "admitted",
  });
  assert.equal(publicationRefreshes, 1);
  assert.throws(() => alpha.notifications.get("pale/workspace/analysisPublished")({
    editor: { generation: "1", documentRevision: "2" },
    admission: "future",
  }), /invalid analysis publication/);
  assert.equal(publicationRefreshes, 1);
  refreshSubscription.dispose();

  settings.set(folderB.uri.toString(), { path: "beta-lsp-next", manifest: "beta.toml" });
  configurationChanged.fire({
    affectsConfiguration: (key, uri) =>
      key === "pale.server.path" && uri.toString() === folderB.uri.toString(),
  });
  await settle();
  await settle();
  assert.equal(LanguageClient.instances.length, 3);
  assert.equal(beta.stopped, true);
  assert.equal(LanguageClient.instances[2].serverOptions.command, "beta-lsp-next");

  vscode.workspace.workspaceFolders = [folderB, folderC];
  workspaceFoldersChanged.fire({ removed: [folderA], added: [folderC] });
  await settle();
  await settle();
  assert.equal(alpha.stopped, true);
  assert.equal(LanguageClient.instances.length, 4);
  assert.equal(LanguageClient.instances[3].serverOptions.command, "gamma-lsp");

  await extension.deactivate();
  assert.equal(LanguageClient.instances[2].stopped, true);
  assert.equal(LanguageClient.instances[3].stopped, true);
  assert.equal(documentOpened.listeners.size, 0);
  assert.equal(errors.length, 0);
});

test("document selectors bind the workspace folder without interpreting its path as glob syntax", async () => {
  const session = await startWorkspaceSession(folderWithGlobSyntax);
  try {
    assert.deepEqual(session.client.clientOptions.documentSelector, [
      { language: "pale", scheme: "file", pattern: "/work/team[[]one[]]/**/*" },
      { language: "pale-toml", scheme: "file", pattern: "/work/team[[]one[]]/**/*" },
      { language: "pale-page", scheme: "file", pattern: "/work/team[[]one[]]/**/*" },
    ]);
  } finally {
    await stopWorkspaceSession(session);
  }
});

test("profile selection offers exact folder-local compiler contexts", async () => {
  vscode.workspace.workspaceFolders = [folderA];
  LanguageClient.instances.length = 0;
  try {
    await extension.activate({ subscriptions: [] });
    await commands.get("pale.profile.selectContext")();
    assert.deepEqual(LanguageClient.instances[0].sentRequests, [
      protocol.PROFILE_CONTEXTS_METHOD,
      protocol.PROFILE_CONTEXTS_METHOD,
    ]);
  } finally {
    await extension.deactivate();
  }
});

test("configured source events stay exact, folder-local, and generation fenced", async () => {
  vscode.workspace.workspaceFolders = [folderA, folderB];
  LanguageClient.instances.length = 0;
  watchers.length = 0;
  settings.set(folderA.uri.toString(), {
    path: "alpha-lsp",
    manifest: "alpha.toml",
    sources: [
      { path: "config/[alpha].toml", languageId: "pale-toml", role: "workspace-module" },
    ],
  });
  settings.set(folderB.uri.toString(), {
    path: "beta-lsp",
    manifest: "beta.toml",
    sources: [
      { path: "config/beta.toml", languageId: "pale-toml", role: "workspace-module" },
    ],
  });
  const alphaUri = new Uri("/work/alpha/config/[alpha].toml");
  const betaUri = new Uri("/work/beta/config/beta.toml");
  fileBytes.set(alphaUri.toString(), Buffer.from("name = 'alpha'\n"));
  fileBytes.set(betaUri.toString(), Buffer.from("name = 'beta'\n"));

  try {
    await extension.activate({ subscriptions: [] });
    assert.equal(
      LanguageClient.instances.length,
      2,
      JSON.stringify({ errors: errors.slice(-2), watcherCount: watchers.length }),
    );
    assert.deepEqual(
      watchers.map((watcher) => [watcher.pattern.base.name, watcher.pattern.pattern]),
      [
        ["alpha", "config/[[]alpha[]].toml"],
        ["beta", "config/beta.toml"],
      ],
    );

    watchers[0].changed.fire(betaUri);
    watchers[0].deleted.fire(alphaUri);
    watchers[0].created.fire(alphaUri);
    await settle();
    await settle();

    assert.deepEqual(LanguageClient.instances[0].sentNotifications, [
      {
        method: "workspace/didChangeWatchedFiles",
        params: { changes: [{ uri: alphaUri.toString(), type: 1 }] },
      },
    ]);
    assert.deepEqual(LanguageClient.instances[1].sentNotifications, []);
  } finally {
    await extension.deactivate();
    settings.set(folderA.uri.toString(), { path: "alpha-lsp", manifest: "alpha.toml" });
    settings.set(folderB.uri.toString(), { path: "beta-lsp", manifest: "beta.toml" });
    fileBytes.clear();
  }
});

test("client rejects unlike protocol evidence before exposing a workspace session", async () => {
  vscode.workspace.workspaceFolders = [folderA];
  const originalStart = LanguageClient.prototype.start;
  try {
    for (const [field, value] of [
      ["packageVersion", "0.0.2"],
      ["inventorySha256", "0".repeat(64)],
      ["wireFixtureSha256", "0".repeat(64)],
    ]) {
      LanguageClient.instances.length = 0;
      LanguageClient.prototype.start = async function startWithUnlikeEvidence() {
        this.initializeResult.capabilities.experimental.paleWorkspace[field] = value;
      };
      await extension.activate({ subscriptions: [] });
      assert.equal(LanguageClient.instances.length, 1);
      assert.equal(LanguageClient.instances[0].stopped, true);
      assert.equal(LanguageClient.instances[0].notifications.size, 0);
      assert.match(errors.at(-1), /exact 0\.0\.1 authoring protocol/);
      await extension.deactivate();
    }
  } finally {
    LanguageClient.prototype.start = originalStart;
  }
});

test("failed startup releases folder subscriptions and attempts transport cleanup", async () => {
  vscode.workspace.workspaceFolders = [folderA];
  const originalStart = LanguageClient.prototype.start;
  LanguageClient.instances.length = 0;
  LanguageClient.prototype.start = async function failStartup() {
    throw new Error("startup failed");
  };
  try {
    await extension.activate({ subscriptions: [] });
    assert.equal(LanguageClient.instances.length, 1);
    assert.equal(documentOpened.listeners.size, 0);
    assert.equal(LanguageClient.instances[0].stopped, true);
    assert.deepEqual(await treeProvider.getChildren(), [{ kind: "empty" }]);
  } finally {
    LanguageClient.prototype.start = originalStart;
    await extension.deactivate();
  }
});

test("shutdown attempts every folder even when one transport rejects cleanup", async () => {
  vscode.workspace.workspaceFolders = [folderA, folderB];
  LanguageClient.instances.length = 0;
  const originalStop = LanguageClient.prototype.stop;
  const failedStop = new Error("alpha cleanup failed");
  const attempted = [];
  LanguageClient.prototype.stop = async function stopWithOneFailure() {
    attempted.push(this.clientOptions.workspaceFolder.name);
    if (this.clientOptions.workspaceFolder === folderA) throw failedStop;
    await originalStop.call(this);
  };
  try {
    await extension.activate({ subscriptions: [] });
    await assert.rejects(extension.deactivate(), (error) => {
      assert.deepEqual(attempted, ["alpha", "beta"]);
      assert.ok(error instanceof AggregateError);
      assert.deepEqual(error.errors, [failedStop]);
      return true;
    });
    assert.deepEqual(attempted, ["alpha", "beta"]);
    assert.equal(LanguageClient.instances[1].stopped, true);
    assert.equal(documentOpened.listeners.size, 0);
    assert.equal(workspaceFoldersChanged.listeners.size, 0);
    assert.equal(configurationChanged.listeners.size, 0);
    assert.equal(commands.size, 0);
    assert.deepEqual(await treeProvider.getChildren(), [{ kind: "empty" }]);
  } finally {
    LanguageClient.prototype.stop = originalStop;
    await extension.deactivate();
  }
});

test("startup preserves both initialization and uncertain cleanup failures", async () => {
  const originalStart = LanguageClient.prototype.start;
  const originalStop = LanguageClient.prototype.stop;
  const startupFailure = new Error("initialization failed");
  const cleanupFailure = new Error("stop outcome unknown");
  LanguageClient.prototype.start = async () => {
    throw startupFailure;
  };
  LanguageClient.prototype.stop = async () => {
    throw cleanupFailure;
  };
  try {
    await assert.rejects(startWorkspaceSession(folderA), (error) => {
      assert.ok(error instanceof AggregateError);
      assert.deepEqual(error.errors, [startupFailure, cleanupFailure]);
      return true;
    });
    assert.equal(documentOpened.listeners.size, 0);
  } finally {
    LanguageClient.prototype.start = originalStart;
    LanguageClient.prototype.stop = originalStop;
  }
});


test("profile context enumeration omits parameters on the real JSON-RPC transport", async () => {
  const toServer = new PassThrough();
  const fromServer = new PassThrough();
  const frames = [];
  toServer.on("data", (chunk) => frames.push(Buffer.from(chunk)));
  const client = jsonrpc.createMessageConnection(
    new jsonrpc.StreamMessageReader(fromServer),
    new jsonrpc.StreamMessageWriter(toServer),
  );
  const server = jsonrpc.createMessageConnection(
    new jsonrpc.StreamMessageReader(toServer),
    new jsonrpc.StreamMessageWriter(fromServer),
  );
  const expected = {
    editor: { generation: "1", documentRevision: "0" },
    contexts: [{ projectId: "notes", executionWorkspaceId: "local" }],
  };
  server.onRequest(protocol.PROFILE_CONTEXTS_METHOD, () => expected);
  server.listen();
  client.listen();
  const session = await startWorkspaceSession(folderA);
  session.client.sendRequest = (...args) => client.sendRequest(...args);
  try {
    assert.deepEqual(await session.profileContexts(), expected);
    const wire = Buffer.concat(frames).toString("utf8");
    const message = JSON.parse(wire.slice(wire.indexOf("\r\n\r\n") + 4));
    assert.equal(message.method, protocol.PROFILE_CONTEXTS_METHOD);
    assert.equal(Object.hasOwn(message, "params"), false,
      "The native method has no parameters; explicit null becomes [null] in this transport");
  } finally {
    await stopWorkspaceSession(session);
    client.dispose();
    server.dispose();
    toServer.destroy();
    fromServer.destroy();
  }
});
