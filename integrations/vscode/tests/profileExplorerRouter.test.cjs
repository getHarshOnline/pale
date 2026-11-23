const assert = require("node:assert/strict");
const Module = require("node:module");
const test = require("node:test");

class Disposable {
  constructor(dispose = () => undefined) {
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

const selectedFolders = [];
const vscode = {
  EventEmitter,
  TreeItem: class TreeItem {},
  ThemeIcon: class ThemeIcon {},
  TreeItemCollapsibleState: { None: 0, Collapsed: 1 },
  window: {
    showQuickPick: async (items) => {
      selectedFolders.push(items.map(({ label }) => label));
      return items[0];
    },
  },
};

const originalLoad = Module._load;
Module._load = function load(request, parent, isMain) {
  if (request === "vscode") return vscode;
  if (request === "./workspaceSession") {
    return { sessionKey: (workspaceFolder) => workspaceFolder.uri.toString() };
  }
  return originalLoad.call(this, request, parent, isMain);
};
const { ProfileExplorerRouter } = require("../dist/profileExplorerRouter.js");
Module._load = originalLoad;

const NONE = {
  canBegin: false,
  canEvaluate: false,
  canInspect: false,
  canFinish: false,
};

function folder(name, index) {
  return {
    name,
    index,
    uri: { toString: () => `file:///work/${name}` },
  };
}

function session(folder, availability) {
  const changes = new EventEmitter();
  return {
    key: folder.uri.toString(),
    folder,
    explorer: {
      inspectionContext: () => ({
        projectId: `project-${folder.name}`,
        executionWorkspaceId: "default",
      }),
      onDidChangeTreeData: changes.event,
      refresh: () => undefined,
    },
    review: { availability: () => availability.value },
  };
}

test("review command routing remains phase-correct and workspace-local", async () => {
  selectedFolders.length = 0;
  const alphaFolder = folder("alpha", 0);
  const betaFolder = folder("beta", 1);
  const alphaAvailability = {
    value: { ...NONE, canEvaluate: true },
  };
  const betaAvailability = {
    value: { ...NONE, canBegin: true },
  };
  const alpha = session(alphaFolder, alphaAvailability);
  const beta = session(betaFolder, betaAvailability);
  const router = new ProfileExplorerRouter();
  router.add(alpha);
  router.add(beta);

  assert.deepEqual(router.configurationImprovementAvailability(), {
    canBegin: true,
    canEvaluate: true,
    canInspect: false,
    canFinish: false,
  });
  assert.equal(await router.chooseConfigurationImprovementSession("evaluate"), alpha);
  assert.equal(await router.chooseConfigurationImprovementSession("begin"), beta);

  router.prefer(alphaFolder);
  assert.deepEqual(router.configurationImprovementAvailability(), alphaAvailability.value);
  assert.equal(await router.chooseConfigurationImprovementSession("evaluate"), alpha);
  assert.equal(await router.chooseConfigurationImprovementSession("begin"), undefined);

  alphaAvailability.value = { ...NONE, canInspect: true, canFinish: true };
  assert.deepEqual(router.configurationImprovementAvailability(), alphaAvailability.value);
  assert.equal(await router.chooseConfigurationImprovementSession("inspect"), alpha);
  assert.equal(await router.chooseConfigurationImprovementSession("finish"), alpha);
  assert.equal(await router.chooseConfigurationImprovementSession("evaluate"), undefined);

  router.prefer(betaFolder);
  assert.deepEqual(router.configurationImprovementAvailability(), betaAvailability.value);
  assert.equal(await router.chooseConfigurationImprovementSession("begin"), beta);
  assert.equal(await router.chooseConfigurationImprovementSession("inspect"), undefined);

  betaAvailability.value = { ...NONE, canBegin: true };
  alphaAvailability.value = { ...NONE, canBegin: true };
  router.prefer(undefined);
  assert.equal(await router.chooseConfigurationImprovementSession("begin"), alpha);
  assert.deepEqual(selectedFolders.at(-1), ["alpha", "beta"]);

  router.dispose();
});
