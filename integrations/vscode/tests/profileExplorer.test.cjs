const assert = require("node:assert/strict");
const Module = require("node:module");
const test = require("node:test");

const crossLeafFixture = require(
  "./fixtures/cross-leaf.v1/contract-diagnostic.json",
);

let diagnostics = [];
const informationMessages = [];
const errorMessages = [];

class EventEmitter {
  constructor() {
    this.event = () => ({ dispose() {} });
  }
  fire() {}
  dispose() {}
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
    this.isTrusted = false;
  }
  appendText(value) {
    this.value += value;
  }
}

const vscode = {
  EventEmitter,
  CancellationTokenSource,
  TreeItem,
  ThemeIcon,
  MarkdownString,
  TreeItemCollapsibleState: { None: 0, Collapsed: 1 },
  languages: { getDiagnostics: () => diagnostics },
  window: {
    showInformationMessage: async (message) => informationMessages.push(message),
    showErrorMessage: async (message) => errorMessages.push(message),
  },
};

const originalLoad = Module._load;
Module._load = function load(request, parent, isMain) {
  return request === "vscode" ? vscode : originalLoad.call(this, request, parent, isMain);
};
const { ProfileExplorer } = require("../dist/profileExplorer.js");
Module._load = originalLoad;

const context = { projectId: "demo", executionWorkspaceId: "default" };
const coordinates = {
  workspaceId: "workspace",
  projectId: "demo",
  executionWorkspaceId: "default",
  machineId: "machine",
  profileDigest: "a".repeat(64),
  instanceDigest: "f".repeat(64),
  selectedTopologyDigest: "1".repeat(64),
  workspaceSemanticIrDigest: "b".repeat(64),
  workspaceAdmissionEvidenceDigest: "c".repeat(64),
  packageSemanticIrDigest: "d".repeat(64),
  compositionPlanDigest: "e".repeat(64),
};
const editor = { generation: "9", documentRevision: "14" };

function admitted(state, section, entries, nextOffset = null, diagnosticIdentity, offset = 0) {
  return {
    state,
    editor,
    context,
    coordinates,
    page: { section, offset, entries, nextOffset },
    ...(diagnosticIdentity === undefined ? {} : { diagnosticIdentity }),
  };
}

test("native TreeDataProvider renders typed distinctions and coordinate-fenced pagination", async () => {
  const requests = [];
  const operation = {
    explanationHandle: "1".repeat(64),
    kind: "operation",
    repositoryId: "engine",
    operationId: "example:operations/review",
    exposure: "capability-bearing",
    nativeRealization: "unknown",
  };
  const capability = {
    explanationHandle: "2".repeat(64),
    kind: "capability",
    capabilityId: "inspect",
    repositoryId: "engine",
    operationId: "example:operations/review",
    requirementId: "local-inspection",
    effectClass: "read",
    requirementDisposition: { kind: "provider-binding-absent" },
    providerDisposition: { kind: "missing-binding" },
    nativeRealization: "unknown",
  };
  const providerRepository = {
    explanationHandle: "4".repeat(64),
    kind: "provider-repository",
    repositoryId: "engine",
    providerBindingId: "provider-binding-a",
    nativeRepositoryCoordinate: "owner://rig/repository",
    nativeRealization: "unknown",
  };
  const finding = {
    explanationHandle: "3".repeat(64),
    kind: "finding",
    finding: {
      kind: "requirement",
      repositoryId: "engine",
      operationId: "example:operations/review",
      requirementId: "local-inspection",
      capabilityId: "inspect",
      disposition: { kind: "provider-binding-absent" },
    },
  };
  const explorer = new ProfileExplorer({
    effectiveProfile: async (request) => {
      requests.push(request);
      return request.offset === 0
        ? admitted(
            { kind: "current" },
            request.section,
            [operation, capability, providerRepository],
            3,
          )
        : admitted(
            { kind: "current" },
            request.section,
            [finding],
            null,
            undefined,
            request.offset,
          );
    },
    explain: async () => {
      throw new Error("not exercised");
    },
  });
  explorer.setInspectionContext(context);

  const first = await explorer.getChildren({
    kind: "section",
    section: "capabilities-and-operations",
    label: "Capabilities and operations",
  });
  assert.equal(first[0].kind, "message");
  assert.equal(first[1].kind, "entry");
  assert.equal(first[4].kind, "more");
  const operationItem = explorer.getTreeItem(first[1]);
  assert.equal(operationItem.id, `profile:entry:${operation.explanationHandle}`);
  assert.equal(operationItem.description, "exposure: capability-bearing · native: unknown");
  assert.match(operationItem.tooltip.value, /Exposure: capability-bearing/);
  assert.equal(operationItem.command.arguments[0].handle, operation.explanationHandle);

  const capabilityItem = explorer.getTreeItem(first[2]);
  assert.equal(capabilityItem.id, `profile:entry:${capability.explanationHandle}`);
  assert.match(capabilityItem.description, /requirement: provider-binding-absent/);
  assert.match(capabilityItem.description, /provider: missing-binding/);
  const providerItem = explorer.getTreeItem(first[3]);
  assert.equal(providerItem.label, "Provider projection engine");
  assert.equal(providerItem.description, "binding: provider-binding-a · native: unknown");
  assert.match(providerItem.tooltip.value, /Opaque native coordinate: owner:\/\/rig\/repository/);
  const continuation = await explorer.getChildren(first[4]);
  assert.equal(continuation[0].entry.finding.disposition.kind, "provider-binding-absent");
  assert.equal(requests[1].expectedCoordinate.profileDigest, coordinates.profileDigest);
  assert.equal(requests[1].expectedCoordinate.instanceDigest, coordinates.instanceDigest);
  explorer.dispose();
});

test("stale and not-admitted rows correlate diagnostics only by opaque identity", async () => {
  const identity = "f".repeat(64);
  const diagnostic = {
    ...JSON.parse(crossLeafFixture.canonical_diagnostic),
    notes: ["no workspace meaning was admitted"],
    help: ["correct the rejected declaration"],
    facts: [{ key: "project", value: "demo" }],
  };
  diagnostics = [
    [
      "file:///workspace/current.toml",
      [
        { source: "pale", message: "wrong", data: { identity: "0".repeat(64) } },
        {
          source: "pale",
          message: diagnostic.summary,
          code: diagnostic.code,
          data: { identity, diagnostic },
        },
      ],
    ],
  ];
  let response = admitted(
    {
      kind: "stale",
      admittedEditor: { generation: "7", documentRevision: "12" },
      admittedProfileDigest: coordinates.profileDigest,
      summary: "duplicate prose does not identify diagnostics",
    },
    "effective-configuration",
    [],
    null,
    identity,
  );
  const explorer = new ProfileExplorer({
    effectiveProfile: async () => response,
    explain: async () => {
      throw new Error("not exercised");
    },
  });
  explorer.setInspectionContext(context);
  const section = { kind: "section", section: "effective-configuration", label: "Effective" };
  const stale = await explorer.getChildren(section);
  assert.match(stale[0].label, /^Out of date:/);
  assert.match(stale[0].tooltip.value, new RegExp(diagnostic.summary));
  assert.match(stale[0].tooltip.value, new RegExp(`Stage: ${diagnostic.stage}`));
  assert.match(stale[0].tooltip.value, new RegExp(`Disposition: ${diagnostic.disposition}`));
  assert.match(stale[0].tooltip.value, new RegExp(`Retry: ${diagnostic.retry}`));
  assert.match(stale[0].tooltip.value, new RegExp(`Effect: ${diagnostic.effect}`));
  assert.match(stale[0].tooltip.value, /Note: no workspace meaning was admitted/);
  assert.match(stale[0].tooltip.value, /Help: correct the rejected declaration/);
  assert.match(stale[0].tooltip.value, /Fact project: demo/);

  response = {
    state: { kind: "not-admitted", summary: "no retained profile" },
    editor,
    context,
    diagnosticIdentity: identity,
  };
  const notAdmitted = await explorer.getChildren(section);
  assert.match(notAdmitted[0].label, /^No usable profile:/);
  assert.match(notAdmitted[0].tooltip.value, new RegExp(diagnostic.summary));
  explorer.dispose();
});

test("request failures present the canonical diagnostic payload when supplied", async () => {
  const diagnostic = {
    ...JSON.parse(crossLeafFixture.canonical_diagnostic),
    notes: ["the current editor input was not admitted"],
    help: ["correct the source and retry"],
    facts: [{ key: "workspace", value: "workspace" }],
  };
  const requestError = { data: diagnostic };
  const explorer = new ProfileExplorer({
    effectiveProfile: async () => {
      throw requestError;
    },
    explain: async () => {
      throw requestError;
    },
  });
  explorer.setInspectionContext(context);
  const children = await explorer.getChildren({
    kind: "section",
    section: "effective-configuration",
    label: "Effective",
  });
  assert.equal(children[0].label, diagnostic.summary);
  for (const expected of [
    `Code: ${diagnostic.code}`,
    `Stage: ${diagnostic.stage}`,
    `Disposition: ${diagnostic.disposition}`,
    `Retry: ${diagnostic.retry}`,
    `Effect: ${diagnostic.effect}`,
    `Note: ${diagnostic.notes[0]}`,
    `Help: ${diagnostic.help[0]}`,
    "Fact workspace: workspace",
  ]) {
    assert.match(children[0].tooltip.value, new RegExp(expected));
  }

  await explorer.explain({
    context,
    handle: "7".repeat(64),
    expectedCoordinate: { ...editor, ...coordinates },
  });
  assert.match(errorMessages.at(-1), new RegExp(diagnostic.summary));
  assert.match(errorMessages.at(-1), new RegExp(`Retry: ${diagnostic.retry}`));
  assert.match(errorMessages.at(-1), /Help: correct the source and retry/);
  explorer.dispose();
});

test("context replacement cancels the pending page and explanation uses its opaque handle", async () => {
  let resolvePage;
  let pendingToken;
  let explanationRequest;
  const explorer = new ProfileExplorer({
    effectiveProfile: (_request, token) => {
      pendingToken = token;
      return new Promise((resolve) => {
        resolvePage = resolve;
      });
    },
    explain: async (request) => {
      explanationRequest = request;
      return {
        editor,
        context,
        coordinates,
        explanation: {
          handle: request.handle,
          evidence: {
            kind: "operation-classification",
            exposure: "unbound",
            nativeRealization: "unknown",
          },
        },
      };
    },
  });
  explorer.setInspectionContext(context);
  const pending = explorer.getChildren({
    kind: "section",
    section: "effective-configuration",
    label: "Effective",
  });
  explorer.setInspectionContext({ projectId: "other", executionWorkspaceId: "default" });
  assert.equal(pendingToken.isCancellationRequested, true);
  resolvePage(admitted({ kind: "current" }, "effective-configuration", []));
  assert.deepEqual(await pending, []);

  explorer.setInspectionContext(context);
  const request = {
    context,
    handle: "4".repeat(64),
    expectedCoordinate: { ...editor, ...coordinates },
  };
  await explorer.explain(request);
  assert.equal(explanationRequest.handle, request.handle);
  assert.match(informationMessages.at(-1), /exposure unbound; native realization unknown/);
  explorer.dispose();
});

test("context replacement suppresses a non-cooperative pending explanation", async () => {
  let resolveExplanation;
  let explanationToken;
  const before = informationMessages.length;
  const explorer = new ProfileExplorer({
    effectiveProfile: async () => admitted({ kind: "current" }, "effective-configuration", []),
    explain: (_request, token) => {
      explanationToken = token;
      return new Promise((resolve) => {
        resolveExplanation = resolve;
      });
    },
  });
  explorer.setInspectionContext(context);
  const request = {
    context,
    handle: "8".repeat(64),
    expectedCoordinate: { ...editor, ...coordinates },
  };
  const pending = explorer.explain(request);
  explorer.setInspectionContext({ projectId: "other", executionWorkspaceId: "default" });
  assert.equal(explanationToken.isCancellationRequested, true);
  resolveExplanation({
    editor,
    context,
    coordinates,
    explanation: {
      handle: request.handle,
      evidence: { kind: "configuration" },
    },
  });
  await pending;
  assert.equal(informationMessages.length, before);
  explorer.dispose();
});

test("explanations require every semantic coordinate", async () => {
  let responseCoordinates = coordinates;
  const before = informationMessages.length;
  const explorer = new ProfileExplorer({
    effectiveProfile: async () => admitted({ kind: "current" }, "effective-configuration", []),
    explain: async (request) => ({
      editor,
      context,
      coordinates: responseCoordinates,
      explanation: {
        handle: request.handle,
        evidence: { kind: "configuration" },
      },
    }),
  });
  explorer.setInspectionContext(context);
  const request = {
    context,
    handle: "9".repeat(64),
    expectedCoordinate: { ...editor, ...coordinates },
  };
  for (const field of [
    "profileDigest",
    "instanceDigest",
    "selectedTopologyDigest",
    "workspaceSemanticIrDigest",
    "workspaceAdmissionEvidenceDigest",
    "packageSemanticIrDigest",
    "compositionPlanDigest",
  ]) {
    responseCoordinates = { ...coordinates, [field]: "0".repeat(64) };
    await explorer.explain(request);
  }
  assert.equal(informationMessages.length, before);
  explorer.dispose();
});
