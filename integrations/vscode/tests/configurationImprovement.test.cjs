const assert = require("node:assert/strict");
const Module = require("node:module");
const test = require("node:test");

const {
  decodeConfigurationImprovementResponse,
} = require("../dist/protocol.js");

class CancellationError extends Error {}

class CancellationTokenSource {
  constructor() {
    this.token = { isCancellationRequested: false };
  }

  cancel() {
    this.token.isCancellationRequested = true;
  }

  dispose() {}
}

const informationMessages = [];
const vscode = {
  CancellationError,
  CancellationTokenSource,
  window: {
    showInformationMessage: async (message) => informationMessages.push(message),
  },
};
const originalLoad = Module._load;
Module._load = function load(request, parent, isMain) {
  if (request === "vscode") return vscode;
  return originalLoad.call(this, request, parent, isMain);
};
const {
  ConfigurationImprovementReview,
} = require("../dist/configurationImprovement.js");
Module._load = originalLoad;

const context = { projectId: "project", executionWorkspaceId: "workspace" };
const baseline = {
  generation: "9",
  documentRevision: "14",
  profileDigest: "a".repeat(64),
  instanceDigest: "b".repeat(64),
  selectedTopologyDigest: "c".repeat(64),
  workspaceSemanticIrDigest: "d".repeat(64),
  workspaceAdmissionEvidenceDigest: "e".repeat(64),
  packageSemanticIrDigest: "f".repeat(64),
  compositionPlanDigest: "0".repeat(64),
};
const coordinate = {
  baseline,
  candidateEditor: { generation: "10", documentRevision: "15" },
  candidateSelectedTopologyDigest: "1".repeat(64),
  candidateProfileDigest: "2".repeat(64),
  candidateInstanceDigest: "3".repeat(64),
  derivationReceiptDigest: "4".repeat(64),
};
const preview = {
  workspaceId: "principal-workspace",
  projectId: "project",
  executionWorkspaceId: "workspace",
  baselineWorkspaceSemanticIrDigest: baseline.workspaceSemanticIrDigest,
  baselineWorkspaceAdmissionEvidenceDigest: baseline.workspaceAdmissionEvidenceDigest,
  profileDigest: baseline.profileDigest,
  instanceDigest: baseline.instanceDigest,
  candidateProfileDigest: coordinate.candidateProfileDigest,
  candidateInstanceDigest: coordinate.candidateInstanceDigest,
  candidateWorkspaceSemanticIrDigest: "5".repeat(64),
  candidateWorkspaceAdmissionEvidenceDigest: "6".repeat(64),
  derivationReceiptDigest: coordinate.derivationReceiptDigest,
  changeRootCount: 1,
  affectedCount: 2,
  unaffectedCount: 3,
  selectedTopologyChange: {
    state: "changed",
    prior_digest: baseline.selectedTopologyDigest,
    candidate_digest: coordinate.candidateSelectedTopologyDigest,
  },
  profileChange: {
    state: "changed",
    prior_digest: baseline.profileDigest,
    candidate_digest: coordinate.candidateProfileDigest,
  },
  instanceChange: {
    state: "changed",
    prior_digest: baseline.instanceDigest,
    candidate_digest: coordinate.candidateInstanceDigest,
  },
  packageSemanticIrChange: {
    state: "unchanged",
    digest: baseline.packageSemanticIrDigest,
  },
  compositionPlanChange: {
    state: "unchanged",
    digest: baseline.compositionPlanDigest,
  },
};
const subject = { kind: "source", sourceId: "workspace/pale-workspace.toml" };
const priorFact = {
  kind: "source-content",
  sourcePath: "pale-workspace.toml",
  sourceDigest: "7".repeat(64),
  byteLen: 100,
};
const currentFact = { ...priorFact, sourceDigest: "8".repeat(64), byteLen: 104 };
const root = {
  subject,
  rootDigest: "9".repeat(64),
  change: { kind: "modified", prior: priorFact, current: currentFact },
};
const overview = {
  state: "details",
  context,
  coordinate,
  changeRoots: { entries: [root], nextOffset: null },
  impacts: {
    entries: [{ subject, state: "affected", contributingRootCount: 1 }],
    nextOffset: null,
  },
};
const subjectDetails = {
  state: "subject-details",
  context,
  coordinate,
  explanation: {
    subject,
    state: "affected",
    causal: {
      frontier: "current",
      root,
      steps: [{
        subject,
        nodeDigest: "a".repeat(64),
        fact: currentFact,
        provenance: { kind: "source", sourceId: subject.sourceId },
        rule: "source-admission",
      }],
    },
  },
  source: {
    subject,
    occurrences: [{
      location: {
        sourceLabel: "pale-workspace.toml",
        path: [{ kind: "field", value: "workspace" }],
      },
      role: "definition",
    }],
    inspected: 4,
    sourcesComplete: true,
  },
};

test("configuration review decoder admits every closed transport product", () => {
  const explained = { state: "explained", context, baseline };
  assert.deepEqual(
    decodeConfigurationImprovementResponse(explained, {
      action: "begin",
      context,
      expectedProfile: baseline,
    }),
    explained,
  );

  const previewed = { state: "previewed", context, coordinate, preview };
  assert.deepEqual(
    decodeConfigurationImprovementResponse(previewed, {
      action: "evaluate",
      context,
      expectedBaseline: baseline,
    }),
    previewed,
  );

  assert.deepEqual(
    decodeConfigurationImprovementResponse(overview, {
      action: "inspect",
      context,
      expectedPreview: coordinate,
      detail: { kind: "overview", rootsOffset: 0, impactsOffset: 0, limit: 64 },
    }),
    overview,
  );
  assert.deepEqual(
    decodeConfigurationImprovementResponse(subjectDetails, {
      action: "inspect",
      context,
      expectedPreview: coordinate,
      detail: {
        kind: "subject",
        subject,
        inputLimit: 16,
        maximumInspected: 64,
        maximumReturned: 16,
      },
    }),
    subjectDetails,
  );

  for (const [disposition, requested] of [
    ["accepted", "accept"],
    ["rejected", "reject"],
    ["cancelled", "cancel"],
    ["stale", "accept"],
  ]) {
    const terminal = { state: "terminal", context, disposition, coordinate, preview };
    assert.deepEqual(
      decodeConfigurationImprovementResponse(terminal, {
        action: "finish",
        context,
        expectedPreview: coordinate,
        disposition: requested,
      }),
      terminal,
    );
  }
});

test("configuration review decoder rejects coordinate and union drift", () => {
  assert.throws(
    () => decodeConfigurationImprovementResponse({
      state: "explained",
      context,
      baseline: { ...baseline, generation: "09" },
    }, { action: "begin", context, expectedProfile: baseline }),
    /invalid configuration-improvement response/,
  );
  assert.throws(
    () => decodeConfigurationImprovementResponse({
      state: "previewed",
      context,
      coordinate: { ...coordinate, candidateSelectedTopologyDigest: "" },
      preview,
    }, { action: "evaluate", context, expectedBaseline: baseline }),
    /64 lowercase hexadecimal/,
  );
  assert.throws(
    () => decodeConfigurationImprovementResponse({
      state: "terminal",
      context,
      disposition: "applied",
      coordinate,
      preview,
    }, { action: "finish", context, expectedPreview: coordinate, disposition: "accept" }),
    /invalid configuration-improvement response/,
  );
  assert.throws(
    () => decodeConfigurationImprovementResponse({
      state: "previewed",
      context,
      coordinate,
      preview: { ...preview, candidateProfileDigest: "5".repeat(64) },
    }, { action: "evaluate", context, expectedBaseline: baseline }),
    /mismatched coordinates/,
  );
  assert.throws(
    () => decodeConfigurationImprovementResponse({
      state: "terminal",
      context: { ...context, executionWorkspaceId: "another-workspace" },
      disposition: "accepted",
      coordinate,
      preview,
    }, { action: "finish", context, expectedPreview: coordinate, disposition: "accept" }),
    /mismatched coordinates/,
  );
  assert.throws(
    () => decodeConfigurationImprovementResponse({
      state: "explained",
      context,
      baseline,
      nativeEffect: true,
    }, { action: "begin", context, expectedProfile: baseline }),
    /invalid configuration-improvement response/,
  );

  assert.throws(
    () => decodeConfigurationImprovementResponse({
      state: "previewed",
      context,
      coordinate,
      preview,
    }, { action: "begin", context, expectedProfile: baseline }),
    /does not match its request/,
  );
  assert.throws(
    () => decodeConfigurationImprovementResponse({
      ...overview,
      changeRoots: { entries: [root, root], nextOffset: null },
    }, {
      action: "inspect",
      context,
      expectedPreview: coordinate,
      detail: { kind: "overview", rootsOffset: 0, impactsOffset: 0, limit: 1 },
    }),
    /requested bound/,
  );
  assert.throws(
    () => decodeConfigurationImprovementResponse(subjectDetails, {
      action: "inspect",
      context,
      expectedPreview: coordinate,
      detail: {
        kind: "subject",
        subject: { kind: "source", sourceId: "another-source" },
        inputLimit: 16,
        maximumInspected: 64,
        maximumReturned: 16,
      },
    }),
    /does not match its request/,
  );
  assert.throws(
    () => decodeConfigurationImprovementResponse({
      state: "terminal",
      context,
      disposition: "rejected",
      coordinate,
      preview,
    }, { action: "finish", context, expectedPreview: coordinate, disposition: "accept" }),
    /does not match its request/,
  );

  for (const candidateEditor of [
    { generation: "8", documentRevision: "15" },
    { generation: "10", documentRevision: "13" },
  ]) {
    const rolledBack = { ...coordinate, candidateEditor };
    assert.throws(
      () => decodeConfigurationImprovementResponse({
        state: "previewed",
        context,
        coordinate: rolledBack,
        preview,
      }, { action: "evaluate", context, expectedBaseline: baseline }),
      /must not precede its baseline/,
    );
  }

  assert.throws(
    () => decodeConfigurationImprovementResponse({
      state: "previewed",
      context,
      coordinate,
      preview: { ...preview, affectedCount: 0x1_0000_0000 },
    }, { action: "evaluate", context, expectedBaseline: baseline }),
    /invalid configuration-improvement response/,
  );
});

test("configuration review decoding returns independent nested copies", () => {
  const value = structuredClone({ state: "previewed", context, coordinate, preview });
  const original = structuredClone(value);
  const result = decodeConfigurationImprovementResponse(value, {
    action: "evaluate", context, expectedBaseline: baseline,
  });
  assert.deepEqual(result, value);
  assert.notStrictEqual(result, value);
  assert.notStrictEqual(result.context, value.context);
  assert.notStrictEqual(result.coordinate, value.coordinate);
  assert.notStrictEqual(result.coordinate.baseline, value.coordinate.baseline);
  assert.notStrictEqual(result.coordinate.candidateEditor, value.coordinate.candidateEditor);
  assert.notStrictEqual(result.preview, value.preview);
  result.context.projectId = "changed";
  result.coordinate.baseline.profileDigest = "changed";
  result.coordinate.candidateEditor.generation = "999";
  result.preview.affectedCount = 99;
  assert.deepEqual(value, original);
});

test("folder-local review projects one non-effectful begin evaluate finish journey", async () => {
  const requests = [];
  const reports = [];
  const review = new ConfigurationImprovementReview({
    effectiveProfile: async () => ({
      state: { kind: "current" },
      editor: { generation: "9", documentRevision: "14" },
      context,
      coordinates: {
        workspaceId: "principal-workspace",
        projectId: "project",
        executionWorkspaceId: "workspace",
        machineId: "machine",
        profileDigest: baseline.profileDigest,
        instanceDigest: baseline.instanceDigest,
        selectedTopologyDigest: baseline.selectedTopologyDigest,
        workspaceSemanticIrDigest: baseline.workspaceSemanticIrDigest,
        workspaceAdmissionEvidenceDigest: baseline.workspaceAdmissionEvidenceDigest,
        packageSemanticIrDigest: baseline.packageSemanticIrDigest,
        compositionPlanDigest: baseline.compositionPlanDigest,
      },
      page: { section: "intended-use-case", offset: 0, entries: [], nextOffset: null },
    }),
    improve: async (request) => {
      requests.push(request);
      if (request.action === "begin") {
        return { state: "explained", context, baseline };
      }
      if (request.action === "evaluate") {
        return { state: "previewed", context, coordinate, preview };
      }
      if (request.action === "inspect" && request.detail.kind === "overview") {
        return overview;
      }
      if (request.action === "inspect" && request.detail.kind === "subject") {
        return subjectDetails;
      }
      return {
        state: "terminal",
        context,
        disposition: "accepted",
        coordinate,
        preview,
      };
    },
  }, {
    show: (report) => reports.push(report),
    dispose: () => undefined,
  });

  assert.deepEqual(review.availability(undefined), {
    canBegin: false,
    canEvaluate: false,
    canInspect: false,
    canFinish: false,
  });
  assert.deepEqual(review.availability(context), {
    canBegin: true,
    canEvaluate: false,
    canInspect: false,
    canFinish: false,
  });
  await review.begin(context);
  assert.deepEqual(review.availability(context), {
    canBegin: false,
    canEvaluate: true,
    canInspect: false,
    canFinish: false,
  });
  assert.deepEqual(
    review.availability({ projectId: "another-project", executionWorkspaceId: "workspace" }),
    { canBegin: true, canEvaluate: false, canInspect: false, canFinish: false },
  );
  await assert.rejects(review.begin(context), /finish the current Pale configuration review/);
  await review.evaluate();
  assert.deepEqual(review.availability(context), {
    canBegin: false,
    canEvaluate: false,
    canInspect: true,
    canFinish: true,
  });
  await review.inspect();
  assert.match(reports.at(-1), /Candidate source occurrences/);
  assert.match(reports.at(-1), /pale-workspace\.toml\.workspace/);
  assert.match(reports.at(-1), /does not apply or authorize a configuration write/);
  await review.finish("accept");
  assert.deepEqual(review.availability(context), {
    canBegin: true,
    canEvaluate: false,
    canInspect: false,
    canFinish: false,
  });
  assert.deepEqual(requests.map(({ action }) => action), [
    "begin",
    "evaluate",
    "inspect",
    "inspect",
    "finish",
  ]);
  assert.equal(
    requests[0].expectedProfile.selectedTopologyDigest,
    baseline.selectedTopologyDigest,
  );
  assert.match(informationMessages.at(-1), /no source was changed/);
  review.dispose();
});

test("disposing a pending review suppresses its cancelled reply", async () => {
  let token;
  let resolveImprovement;
  const review = new ConfigurationImprovementReview({
    effectiveProfile: async () => ({
      state: { kind: "current" },
      editor: { generation: "9", documentRevision: "14" },
      context,
      coordinates: {
        workspaceId: "principal-workspace",
        projectId: "project",
        executionWorkspaceId: "workspace",
        machineId: "machine",
        profileDigest: baseline.profileDigest,
        instanceDigest: baseline.instanceDigest,
        selectedTopologyDigest: baseline.selectedTopologyDigest,
        workspaceSemanticIrDigest: baseline.workspaceSemanticIrDigest,
        workspaceAdmissionEvidenceDigest: baseline.workspaceAdmissionEvidenceDigest,
        packageSemanticIrDigest: baseline.packageSemanticIrDigest,
        compositionPlanDigest: baseline.compositionPlanDigest,
      },
      page: { section: "intended-use-case", offset: 0, entries: [], nextOffset: null },
    }),
    improve: (_request, cancellation) => {
      token = cancellation;
      return new Promise((resolve) => {
        resolveImprovement = resolve;
      });
    },
  });

  const pending = review.begin(context);
  await new Promise((resolve) => setImmediate(resolve));
  review.dispose();
  assert.equal(token.isCancellationRequested, true);
  resolveImprovement({ state: "explained", context, baseline });
  await assert.rejects(pending, CancellationError);
});
