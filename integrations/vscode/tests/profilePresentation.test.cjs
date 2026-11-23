const assert = require("node:assert/strict");
const test = require("node:test");

const {
  ActiveRequestSet,
  ContextEpoch,
  PROFILE_SECTIONS,
  apiDiagnosticFromLspData,
  apiDiagnosticFromRequestError,
  apiDiagnosticLines,
  entryDescription,
  entryIcon,
  entryTooltipLines,
  explanationLabel,
  lspCapacityFromRequestError,
  lspCapacityLines,
  opaqueDiagnosticIdentity,
  profileStateLabel,
  sameContext,
} = require("../dist/profilePresentation.js");
const { profileProductMatchesExpected } = require("../dist/protocol.js");

const crossLeafFixture = require("./fixtures/cross-leaf.v1/contract-diagnostic.json");

test("the explorer exposes only the five approved shallow sections", () => {
  assert.deepEqual(
    PROFILE_SECTIONS.map(({ section }) => section),
    [
      "intended-use-case",
      "effective-configuration",
      "capabilities-and-operations",
      "machine-realization",
      "gaps-and-explanations",
    ],
  );
});

test("context equality uses both non-substitutable logical coordinates", () => {
  const context = { projectId: "pale", executionWorkspaceId: "primary" };
  assert.equal(sameContext(context, { ...context }), true);
  assert.equal(
    sameContext(context, { projectId: "pale", executionWorkspaceId: "review" }),
    false,
  );
  assert.equal(sameContext(context, undefined), false);
  assert.equal(sameContext(undefined, undefined), false);
});

test("a context epoch rejects replies from every predecessor", () => {
  const epoch = new ContextEpoch();
  const predecessor = epoch.current();
  assert.equal(epoch.owns(predecessor), true);
  const current = epoch.advance();
  assert.equal(epoch.owns(predecessor), false);
  assert.equal(epoch.owns(current), true);
});

test("a context switch cancels every prior in-flight request", () => {
  const requests = new ActiveRequestSet();
  let cancellations = 0;
  requests.add({ cancel: () => (cancellations += 1) });
  requests.add({ cancel: () => (cancellations += 1) });
  requests.cancelAll();
  requests.cancelAll();
  assert.equal(cancellations, 2);
});

test("paged replies cannot cross editor or profile coordinates", () => {
  const response = {
    state: { kind: "current" },
    editor: { generation: "9", documentRevision: "14" },
    context: { projectId: "pale", executionWorkspaceId: "primary" },
    coordinates: {
      workspaceId: "workspace",
      projectId: "pale",
      executionWorkspaceId: "primary",
      machineId: "harsh-primary",
      profileDigest: "a".repeat(64),
      instanceDigest: "f".repeat(64),
      selectedTopologyDigest: "1".repeat(64),
      workspaceSemanticIrDigest: "d".repeat(64),
      workspaceAdmissionEvidenceDigest: "e".repeat(64),
      packageSemanticIrDigest: "b".repeat(64),
      compositionPlanDigest: "c".repeat(64),
    },
    page: {
      section: "effective-configuration",
      offset: 0,
      entries: [],
      nextOffset: null,
    },
  };
  assert.equal(
    profileProductMatchesExpected(response.editor, response.coordinates, {
      generation: "9",
      documentRevision: "14",
      profileDigest: "a".repeat(64),
      instanceDigest: "f".repeat(64),
      selectedTopologyDigest: "1".repeat(64),
      workspaceSemanticIrDigest: "d".repeat(64),
      workspaceAdmissionEvidenceDigest: "e".repeat(64),
      packageSemanticIrDigest: "b".repeat(64),
      compositionPlanDigest: "c".repeat(64),
    }),
    true,
  );
  assert.equal(
    profileProductMatchesExpected(response.editor, response.coordinates, {
      generation: "10",
      documentRevision: "14",
      profileDigest: "a".repeat(64),
      instanceDigest: "f".repeat(64),
      selectedTopologyDigest: "1".repeat(64),
      workspaceSemanticIrDigest: "d".repeat(64),
      workspaceAdmissionEvidenceDigest: "e".repeat(64),
      packageSemanticIrDigest: "b".repeat(64),
      compositionPlanDigest: "c".repeat(64),
    }),
    false,
  );
  const admittedCoordinate = {
    generation: "9",
    documentRevision: "14",
    profileDigest: "a".repeat(64),
    instanceDigest: "f".repeat(64),
    selectedTopologyDigest: "1".repeat(64),
    workspaceSemanticIrDigest: "d".repeat(64),
    workspaceAdmissionEvidenceDigest: "e".repeat(64),
    packageSemanticIrDigest: "b".repeat(64),
    compositionPlanDigest: "c".repeat(64),
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
    assert.equal(
      profileProductMatchesExpected(response.editor, response.coordinates, {
        ...admittedCoordinate,
        [field]: "0".repeat(64),
      }),
      false,
      `${field} must fence the continuation`,
    );
  }
});

test("profile states remain visibly current stale or not admitted", () => {
  const editor = { generation: "9", documentRevision: "14" };
  assert.equal(
    profileStateLabel({ kind: "current" }, editor),
    "Current editor generation 9",
  );
  assert.equal(
    profileStateLabel(
      {
        kind: "stale",
        admittedEditor: { generation: "7", documentRevision: "12" },
        admittedProfileDigest: "a".repeat(64),
        summary: "current input was rejected",
      },
      editor,
    ),
    "Out of date: current input was rejected; showing the last usable generation 7; current generation is 9",
  );
  assert.equal(
    profileStateLabel(
      { kind: "not-admitted", summary: "current input was rejected" },
      editor,
    ),
    "No usable profile: current input was rejected",
  );
});

test("icons are presentation-only and preserve finding distinctions", () => {
  assert.equal(
    entryIcon({
      kind: "finding",
      finding: {
        kind: "requirement",
        repositoryId: "repository",
        operationId: "operation",
        requirementId: "requirement",
        capabilityId: "capability",
        disposition: { kind: "satisfied" },
      },
    }),
    "pass",
  );
  assert.equal(
    entryIcon({
      kind: "finding",
      finding: {
        kind: "operation-unbound",
        repositoryId: "repository",
        operationId: "operation",
      },
    }),
    "warning",
  );
  assert.equal(
    entryIcon({
      kind: "machine-realization",
      machineId: "machine",
      realization: "unknown",
    }),
    "server",
  );
  assert.equal(
    entryIcon({
      kind: "repository",
      repositoryId: "repository",
      materializationOrder: 0,
      accessCeiling: "read",
    }),
    "repo",
  );
  const providerRepository = {
    kind: "provider-repository",
    repositoryId: "repository",
    providerBindingId: "provider-binding",
    nativeRepositoryCoordinate: "owner://rig/repository",
    nativeRealization: "unknown",
  };
  assert.equal(entryIcon(providerRepository), "plug");
  assert.equal(
    entryDescription(providerRepository),
    "binding: provider-binding · native: unknown",
  );
  assert.deepEqual(entryTooltipLines(providerRepository), [
    "Repository: repository",
    "Provider binding: provider-binding",
    "Opaque native coordinate: owner://rig/repository",
    "Native realization: unknown",
  ]);
  assert.equal(
    entryIcon({
      kind: "execution-path",
      selectionId: "selection",
      experiencePathId: "experience",
      responsibilityPathIdentity: "path",
      responsibilityContractIds: [],
      workDelegationEdgeIds: [],
      providerBindingIds: [],
      capabilityCeiling: [],
      effectCeiling: [],
      terminalProviderBinding: {
        providerBindingId: "provider-binding",
        providerBindingIdentity: "provider-binding-identity",
      },
      nativeRealization: "unknown",
      reasonSteps: [],
      subject: { kind: "experience-composition" },
    }),
    "type-hierarchy-sub",
  );
  assert.equal(
    entryIcon({
      kind: "capability-narrowing",
      permittedCapabilities: [],
      forbiddenCapabilities: [],
      permittedEffects: [],
      forbiddenEffects: [],
    }),
    "filter",
  );
  assert.equal(
    entryIcon({
      kind: "governing-ceiling",
      scope: "operation",
      subjectResource: "package:operations/review",
      contributors: [],
      policies: [],
      permittedCapabilities: [],
      forbiddenCapabilities: [],
      permittedEffects: [],
      forbiddenEffects: [],
    }),
    "shield",
  );
});

test("diagnostics are correlated only by the API-owned opaque identity", () => {
  const identity = "a".repeat(64);
  assert.equal(
    opaqueDiagnosticIdentity({
      identity,
      diagnostic: { arbitrary: "owner-owned" },
    }),
    identity,
  );
  assert.equal(
    opaqueDiagnosticIdentity({ diagnostic: { summary: "same prose" } }),
    undefined,
  );
});

test("diagnostic transport validates and copies only the presentation projection", () => {
  const identity = "b".repeat(64);
  const canonical = JSON.parse(
    crossLeafFixture.outcome_unknown.canonical_diagnostic,
  );

  const expected = {
    code: canonical.code,
    stage: canonical.stage,
    disposition: canonical.disposition,
    retry: canonical.retry,
    effect: canonical.effect,
    summary: canonical.summary,
  };
  assert.deepEqual(
    apiDiagnosticFromLspData({ identity, diagnostic: canonical }),
    expected,
  );
  assert.deepEqual(
    apiDiagnosticFromRequestError({ data: canonical }),
    expected,
  );
  assert.equal(
    opaqueDiagnosticIdentity({ identity, diagnostic: canonical }),
    identity,
  );
});

test("diagnostic projection rejects malformed rendered fields without trusting unrelated members", () => {
  const canonical = JSON.parse(crossLeafFixture.canonical_diagnostic);
  assert.equal(
    apiDiagnosticFromRequestError({
      data: { ...canonical, notes: ["valid", 7] },
    }),
    undefined,
  );
  assert.deepEqual(
    apiDiagnosticFromRequestError({
      data: { ...canonical, unowned: { policy: true } },
    }),
    {
      code: canonical.code,
      stage: canonical.stage,
      disposition: canonical.disposition,
      retry: canonical.retry,
      effect: canonical.effect,
      summary: canonical.summary,
    },
  );
});

test("diagnostic rendering exposes owner-supplied explanation without reclassifying it", () => {
  const canonical = JSON.parse(
    crossLeafFixture.outcome_unknown.canonical_diagnostic,
  );
  const projected = apiDiagnosticFromRequestError({
    data: {
      ...canonical,
      notes: ["the owner retained one exact uncertainty observation"],
      help: ["reconcile the retained evidence before retrying"],
      facts: [{ key: "attempt", value: "attempt-7" }],
    },
  });
  assert.notEqual(projected, undefined);
  assert.deepEqual(apiDiagnosticLines(projected), [
    canonical.summary,
    `Code: ${canonical.code}`,
    `Stage: ${canonical.stage}`,
    `Disposition: ${canonical.disposition}`,
    `Retry: ${canonical.retry}`,
    `Effect: ${canonical.effect}`,
    "Note: the owner retained one exact uncertainty observation",
    "Help: reconcile the retained evidence before retrying",
    "Fact attempt: attempt-7",
  ]);
});

test("capacity errors preserve decimal bounds and server-owned recovery without trusting error prose", () => {
  const projected = lspCapacityFromRequestError({
    message: "untrusted server prose",
    data: {
      kind: "reference-pages",
      unit: "pages",
      maximum: "18446744073709551615",
      recovery:
        "Restart this request from the current admitted editor generation.",
    },
  });
  assert.notEqual(projected, undefined);
  assert.deepEqual(lspCapacityLines(projected), [
    "Pale reached its reference pages limit.",
    "Maximum: 18446744073709551615 pages",
    "Recovery: Restart this request from the current admitted editor generation.",
  ]);

  assert.deepEqual(
    lspCapacityLines(
      lspCapacityFromRequestError({
        data: {
          kind: "document-bytes",
          unit: "bytes",
          scope: "workspace",
          maximum: "1024",
          observed: "1025",
          recovery:
            "Reduce this source document, then retry the editor change.",
        },
      }),
    ),
    [
      "Pale reached its document bytes limit for workspace.",
      "Maximum: 1024 bytes",
      "Observed: 1025 bytes",
      "Recovery: Reduce this source document, then retry the editor change.",
    ],
  );
});

test("capacity projection rejects unsafe recovery, integers, and extra fields", () => {
  const base = {
    kind: "query-queue",
    unit: "requests",
    maximum: "64",
    observedMinimum: "65",
    recovery:
      "Wait for one Pale request to finish, then retry against the current editor generation.",
  };
  assert.equal(
    lspCapacityFromRequestError({
      data: { ...base, recovery: "retry\nthen continue" },
    }),
    undefined,
  );
  assert.equal(
    lspCapacityFromRequestError({
      data: { ...base, recovery: "x".repeat(513) },
    }),
    undefined,
  );
  assert.equal(
    lspCapacityFromRequestError({ data: { ...base, maximum: 64 } }),
    undefined,
  );
  assert.equal(
    lspCapacityFromRequestError({
      data: { ...base, maximum: "18446744073709551616" },
    }),
    undefined,
  );
  assert.equal(
    lspCapacityFromRequestError({ data: { ...base, unowned: true } }),
    undefined,
  );
});

test("native presentation exposes independent operation capability provider and machine axes", () => {
  const operation = {
    explanationHandle: "a".repeat(64),
    kind: "operation",
    repositoryId: "engine",
    operationId: "example:operations/review",
    exposure: "semantic-only",
    nativeRealization: "unknown",
  };
  assert.equal(
    entryDescription(operation),
    "exposure: semantic-only · native: unknown",
  );
  assert.deepEqual(entryTooltipLines(operation), [
    "Repository: engine",
    "Exposure: semantic-only",
    "Native realization: unknown",
  ]);

  const capability = {
    explanationHandle: "b".repeat(64),
    kind: "capability",
    capabilityId: "inspect",
    repositoryId: "engine",
    operationId: "example:operations/review",
    requirementId: "local-inspection",
    effectClass: "read",
    requirementDisposition: {
      kind: "narrowed",
      reason: "capability-not-permitted",
    },
    providerDisposition: { kind: "bound", provider: "host-native" },
    nativeRealization: "unknown",
  };
  assert.equal(
    entryDescription(capability),
    "requirement: capability-not-permitted · provider: host-native · native: unknown",
  );
  assert.match(
    entryTooltipLines(capability).join("\n"),
    /Provider disposition: host-native/,
  );

  const finding = {
    explanationHandle: "c".repeat(64),
    kind: "finding",
    finding: {
      kind: "requirement",
      repositoryId: "engine",
      operationId: "example:operations/review",
      requirementId: "local-inspection",
      capabilityId: "inspect",
      disposition: { kind: "blocked", reason: "capability-forbidden" },
    },
  };
  assert.equal(entryDescription(finding), "finding: capability-forbidden");
  assert.equal(
    explanationLabel({
      kind: "capability-classification",
      requirementDisposition: capability.requirementDisposition,
      providerDisposition: capability.providerDisposition,
      nativeRealization: "unknown",
    }),
    "requirement capability-not-permitted; provider host-native; native realization unknown",
  );
});
