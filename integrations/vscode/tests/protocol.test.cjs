const assert = require("node:assert/strict");
const test = require("node:test");
const { createHash } = require("node:crypto");
const { readFileSync } = require("node:fs");
const { join } = require("node:path");

const {
  decodeAnalysisPublication,
  decodeConfigurationImprovementResponse,
  decodeEffectiveProfileExplanationResponse,
  decodeEffectiveProfileResponse,
  decodeProfileContextsResponse,
  decodeServerProtocolAdvertisement,
  DOCUMENT_ROLES,
  PALE_LSP_CANCELLATION,
  PALE_LSP_METHODS,
  PALE_LSP_NOTIFICATIONS,
  PALE_LSP_PACKAGE_VERSION,
  PALE_LSP_PROTOCOL_ID,
  PALE_LSP_PROTOCOL_INVENTORY_SHA256,
  PALE_LSP_WIRE_FIXTURE_SHA256,
} = require("../dist/protocol.js");

test("analysis publications are closed and preserve exact editor coordinates", () => {
  const publication = {
    editor: { generation: "9", documentRevision: "14" },
    admission: "admitted",
  };
  assert.deepEqual(decodeAnalysisPublication(publication), publication);
  for (const malformed of [
    { ...publication, admission: "future" },
    { ...publication, unrecognizedField: true },
    { ...publication, editor: { generation: "09", documentRevision: "14" } },
  ]) {
    assert.throws(() => decodeAnalysisPublication(malformed));
  }
});

const serverAdvertisement = () => ({
  unrelatedExperimentalCapability: true,
  paleWorkspace: {
    protocolId: PALE_LSP_PROTOCOL_ID,
    packageVersion: PALE_LSP_PACKAGE_VERSION,
    inventorySha256: PALE_LSP_PROTOCOL_INVENTORY_SHA256,
    wireFixtureSha256: PALE_LSP_WIRE_FIXTURE_SHA256,
    methods: [...PALE_LSP_METHODS],
    notifications: [...PALE_LSP_NOTIFICATIONS],
    cancellation: PALE_LSP_CANCELLATION,
    documentRoles: [...DOCUMENT_ROLES],
  },
});

test("server advertisement rejects unlike inventories or attested exchanges", () => {
  decodeServerProtocolAdvertisement(serverAdvertisement());

  for (const mutate of [
    (value) => value.paleWorkspace.methods.reverse(),
    (value) => value.paleWorkspace.documentRoles.pop(),
    (value) => { value.paleWorkspace.inventorySha256 = "0".repeat(64); },
    (value) => { value.paleWorkspace.wireFixtureSha256 = "0".repeat(64); },
    (value) => { value.paleWorkspace.extra = true; },
  ]) {
    const value = serverAdvertisement();
    mutate(value);
    assert.throws(() => decodeServerProtocolAdvertisement(value));
  }
});

test("the client embeds the digest of its exact Rust-produced exchange fixture", () => {
  const bytes = readFileSync(join(__dirname, "fixtures/cross-leaf.v1/effective-profile-wire.json"));
  const digest = createHash("sha256").update(bytes).digest("hex");
  assert.equal(PALE_LSP_WIRE_FIXTURE_SHA256, digest);
});

const rustWireFixture = require("./fixtures/cross-leaf.v1/effective-profile-wire.json");

test("the client admits the actual Rust-owned protocol advertisement", () => {
  const advertisement = structuredClone(rustWireFixture.protocolAdvertisementWithoutWireDigest);
  const bytes = readFileSync(join(__dirname, "fixtures/cross-leaf.v1/effective-profile-wire.json"));
  assert.equal(Object.hasOwn(advertisement.paleWorkspace, "wireFixtureSha256"), false);
  advertisement.paleWorkspace.wireFixtureSha256 = createHash("sha256").update(bytes).digest("hex");
  decodeServerProtocolAdvertisement(advertisement);
});

const context = { projectId: "project", executionWorkspaceId: "workspace" };
const editor = { generation: "9", documentRevision: "14" };

test("profile contexts retain exact pairs and reject malformed enumerations", () => {
  const response = {
    editor,
    contexts: [
      { projectId: "alpha", executionWorkspaceId: "local" },
      { projectId: "alpha", executionWorkspaceId: "remote" },
    ],
  };
  assert.deepEqual(decodeProfileContextsResponse(response), response);
  assert.throws(() =>
    decodeProfileContextsResponse({
      editor,
      contexts: [response.contexts[0], response.contexts[0]],
    }),
  );
  assert.throws(() =>
    decodeProfileContextsResponse({ ...response, unknown: true }),
  );
});
const coordinates = {
  workspaceId: "principal-workspace",
  projectId: context.projectId,
  executionWorkspaceId: context.executionWorkspaceId,
  machineId: "machine",
  profileDigest: "a".repeat(64),
  instanceDigest: "b".repeat(64),
  selectedTopologyDigest: "c".repeat(64),
  workspaceSemanticIrDigest: "d".repeat(64),
  workspaceAdmissionEvidenceDigest: "e".repeat(64),
  packageSemanticIrDigest: "f".repeat(64),
  compositionPlanDigest: "0".repeat(64),
};
const expectedCoordinate = {
  ...editor,
  profileDigest: coordinates.profileDigest,
  instanceDigest: coordinates.instanceDigest,
  selectedTopologyDigest: coordinates.selectedTopologyDigest,
  workspaceSemanticIrDigest: coordinates.workspaceSemanticIrDigest,
  workspaceAdmissionEvidenceDigest: coordinates.workspaceAdmissionEvidenceDigest,
  packageSemanticIrDigest: coordinates.packageSemanticIrDigest,
  compositionPlanDigest: coordinates.compositionPlanDigest,
};
const request = {
  context,
  section: "effective-configuration",
  offset: 0,
  limit: 256,
};
const reasonSteps = [{ kind: "origin-receipt", contractId: "origin" }];
const subject = { kind: "experience-composition" };
const entries = [
  {
    explanationHandle: "0".repeat(64),
    kind: "declared-selection",
    selection: {
      kind: "resource",
      resourceKind: "operation",
      selector: { kind: "exact", resourceId: "package:operations/review" },
    },
  },
  {
    explanationHandle: "1".repeat(64),
    kind: "responsibility",
    contractId: "responsibility",
    executionPathId: "path",
    participant: { kind: "terminal-leaf", terminalLeafId: "leaf" },
    responsibilities: [
      { kind: "engineering-execution" },
      {
        kind: "extension",
        namespaceId: "namespace",
        contractId: "contract",
        schemaId: "schema",
        schemaSha256: "1".repeat(64),
      },
    ],
    authorityCeiling: ["read", "execute"],
    reasonSteps,
    subject,
  },
  {
    explanationHandle: "2".repeat(64),
    kind: "selection",
    selection: {
      kind: "session",
      executionWorkspaceId: context.executionWorkspaceId,
      activationRootCount: 1,
      additionalRootCount: 2,
      activationPathCount: 3,
    },
  },
  {
    explanationHandle: "3".repeat(64),
    kind: "capability-narrowing",
    permittedCapabilities: ["inspect"],
    forbiddenCapabilities: ["publish"],
    permittedEffects: ["read"],
    forbiddenEffects: ["publish"],
  },
  {
    explanationHandle: "4".repeat(64),
    kind: "governing-ceiling",
    scope: "operation",
    subjectResource: "package:operations/review",
    contributors: ["policy-a"],
    policies: ["package:policies/review"],
    permittedCapabilities: ["inspect"],
    forbiddenCapabilities: [],
    permittedEffects: ["read"],
    forbiddenEffects: [],
  },
  {
    explanationHandle: "5".repeat(64),
    kind: "repository",
    repositoryId: "repository",
    materializationOrder: 0,
    accessCeiling: "read",
  },
  {
    explanationHandle: "6".repeat(64),
    kind: "provider-repository",
    repositoryId: "repository",
    providerBindingId: "provider-binding",
    nativeRepositoryCoordinate: "owner://repository",
    nativeRealization: "unknown",
  },
  {
    explanationHandle: "7".repeat(64),
    kind: "execution-path",
    selectionId: "selection",
    experiencePathId: "experience-path",
    responsibilityPathIdentity: "2".repeat(64),
    responsibilityContractIds: ["responsibility"],
    workDelegationEdgeIds: ["delegation"],
    providerBindingIds: ["provider-binding"],
    capabilityCeiling: ["inspect"],
    effectCeiling: ["read"],
    terminalProviderBinding: {
      providerBindingId: "provider-binding",
      providerBindingIdentity: "3".repeat(64),
    },
    nativeRealization: "unknown",
    reasonSteps,
    subject,
  },
  {
    explanationHandle: "8".repeat(64),
    kind: "operation",
    repositoryId: "repository",
    operationId: "package:operations/review",
    exposure: "capability-bearing",
    nativeRealization: "unknown",
  },
  {
    explanationHandle: "9".repeat(64),
    kind: "capability",
    capabilityId: "inspect",
    repositoryId: "repository",
    operationId: "package:operations/review",
    requirementId: "local-inspection",
    effectClass: "read",
    requirementDisposition: { kind: "narrowed", reason: "effect-not-permitted" },
    providerDisposition: { kind: "bound", provider: "native-rust" },
    nativeRealization: "unknown",
  },
  {
    explanationHandle: "a".repeat(64),
    kind: "machine-realization",
    machineId: "machine",
    realization: "unknown",
  },
  {
    explanationHandle: "b".repeat(64),
    kind: "finding",
    finding: {
      kind: "requirement",
      repositoryId: "repository",
      operationId: "package:operations/review",
      requirementId: "local-inspection",
      capabilityId: "inspect",
      disposition: { kind: "provider-binding-absent" },
    },
  },
];

test("TypeScript decodes the exact Rust-produced profile wire products", () => {
  assert.deepEqual(
    decodeEffectiveProfileResponse(
      rustWireFixture.currentProfileResponse,
      rustWireFixture.profileRequest,
    ),
    rustWireFixture.currentProfileResponse,
  );
  assert.deepEqual(
    decodeEffectiveProfileResponse(
      rustWireFixture.staleProfileResponse,
      rustWireFixture.profileRequest,
    ),
    rustWireFixture.staleProfileResponse,
  );
  assert.deepEqual(
    decodeEffectiveProfileResponse(
      rustWireFixture.notAdmittedProfileResponse,
      rustWireFixture.notAdmittedProfileRequest,
    ),
    rustWireFixture.notAdmittedProfileResponse,
  );
  assert.deepEqual(
    decodeEffectiveProfileExplanationResponse(
      rustWireFixture.explanationResponse,
      rustWireFixture.explanationRequest,
    ),
    rustWireFixture.explanationResponse,
  );

  const improvement = rustWireFixture.improvement;
  assert.deepEqual(
    decodeConfigurationImprovementResponse(improvement.explainedResponse, improvement.beginRequest),
    improvement.explainedResponse,
  );
  assert.deepEqual(
    decodeConfigurationImprovementResponse(improvement.previewedResponse, improvement.evaluateRequest),
    improvement.previewedResponse,
  );
  assert.deepEqual(
    decodeConfigurationImprovementResponse(improvement.terminalResponse, improvement.finishRequest),
    improvement.terminalResponse,
  );
});

function admitted(state = { kind: "current" }, pageEntries = entries) {
  return {
    state,
    editor,
    context,
    coordinates,
    page: {
      section: request.section,
      offset: request.offset,
      entries: pageEntries,
      nextOffset: null,
    },
  };
}

test("profile decoder admits the complete closed entry algebra", () => {
  const response = admitted();
  assert.deepEqual(decodeEffectiveProfileResponse(response, request), response);

  const terminalProvider = response.page.entries.find(({ kind }) => kind === "execution-path");
  assert.equal(terminalProvider.terminalProviderBinding.providerBindingId, "provider-binding");
});

test("profile decoder preserves current stale and not-admitted custody", () => {
  const stale = admitted({
    kind: "stale",
    admittedEditor: { generation: "7", documentRevision: "12" },
    admittedProfileDigest: coordinates.profileDigest,
    summary: "current input was rejected",
  });
  stale.diagnosticIdentity = "c".repeat(64);
  assert.deepEqual(decodeEffectiveProfileResponse(stale, request), stale);

  const notAdmitted = {
    state: { kind: "not-admitted", summary: "no admitted profile" },
    editor,
    context,
  };
  assert.deepEqual(decodeEffectiveProfileResponse(notAdmitted, request), notAdmitted);
});

test("profile decoder rejects union, field, bound, and coordinate drift", () => {
  assert.throws(
    () => decodeEffectiveProfileResponse({ ...admitted(), nativeEffect: true }, request),
    /unknown or missing fields/,
  );
  assert.throws(
    () => decodeEffectiveProfileResponse({ ...admitted(), context: { ...context, projectId: "other" } }, request),
    /inspection context/,
  );
  assert.throws(
    () => decodeEffectiveProfileResponse(admitted({ kind: "future" }), request),
    /unknown effective-profile state/,
  );
  assert.throws(
    () => decodeEffectiveProfileResponse(admitted(undefined, [
      { ...entries[5], materializationOrder: 0x1_0000_0000 },
    ]), request),
    /unsigned 32-bit integer/,
  );
  assert.throws(
    () => decodeEffectiveProfileResponse(admitted(undefined, Array.from(
      { length: 257 },
      (_, index) => ({ ...entries[0], explanationHandle: index.toString(16).padStart(64, "0") }),
    )), request),
    /requested page window/,
  );
  assert.throws(
    () => decodeEffectiveProfileResponse(admitted(undefined, [
      { ...entries[7], terminalProviderBinding: undefined, terminalHarness: "codex" },
    ]), request),
    /unknown or missing fields/,
  );
  assert.throws(
    () => decodeEffectiveProfileResponse(admitted(), {
      ...request,
      expectedCoordinate: { ...expectedCoordinate, instanceDigest: "9".repeat(64) },
    }),
    /continuation coordinate/,
  );
});

test("profile decoder enforces normalized windows and exact page custody", () => {
  const sixtyFour = Array.from({ length: 64 }, (_, index) => ({
    ...entries[0],
    explanationHandle: index.toString(16).padStart(64, "0"),
  }));
  const defaultWindow = admitted(undefined, sixtyFour);
  defaultWindow.page.nextOffset = 64;
  assert.deepEqual(
    decodeEffectiveProfileResponse(defaultWindow, { ...request, limit: 0 }),
    defaultWindow,
  );

  const twoHundredFiftySix = Array.from({ length: 256 }, (_, index) => ({
    ...entries[0],
    explanationHandle: index.toString(16).padStart(64, "0"),
  }));
  const cappedWindow = admitted(undefined, twoHundredFiftySix);
  cappedWindow.page.nextOffset = 256;
  assert.deepEqual(
    decodeEffectiveProfileResponse(cappedWindow, { ...request, limit: 257 }),
    cappedWindow,
  );

  const underfilled = admitted(undefined, [entries[0]]);
  underfilled.page.nextOffset = 1;
  assert.throws(
    () => decodeEffectiveProfileResponse(underfilled, { ...request, limit: 2 }),
    /non-contiguous page coordinate/,
  );
  const emptyNonterminal = admitted(undefined, []);
  emptyNonterminal.page.nextOffset = 1;
  assert.throws(
    () => decodeEffectiveProfileResponse(emptyNonterminal, { ...request, limit: 1 }),
    /non-contiguous page coordinate/,
  );
  assert.throws(
    () => decodeEffectiveProfileResponse(admitted(undefined, entries.slice(0, 2)), {
      ...request,
      limit: 1,
    }),
    /requested page window/,
  );
  assert.throws(
    () => decodeEffectiveProfileResponse(admitted(), { ...request, offset: 1 }),
    /prior coordinate/,
  );
  assert.throws(
    () => decodeEffectiveProfileResponse(admitted(), {
      ...request,
      offset: 1,
      expectedCoordinate,
    }),
    /offset does not match/,
  );
});

test("profile decoder preserves stale and digest domains exactly", () => {
  const currentWithDiagnostic = admitted();
  currentWithDiagnostic.diagnosticIdentity = "c".repeat(64);
  assert.throws(
    () => decodeEffectiveProfileResponse(currentWithDiagnostic, request),
    /unknown or missing fields/,
  );

  for (const admittedEditor of [
    editor,
    { generation: "10", documentRevision: "14" },
    { generation: "9", documentRevision: "15" },
  ]) {
    assert.throws(
      () => decodeEffectiveProfileResponse(admitted({
        kind: "stale",
        admittedEditor,
        admittedProfileDigest: coordinates.profileDigest,
        summary: "rejected",
      }), request),
      /predecessor editor product/,
    );
  }

  assert.throws(
    () => decodeEffectiveProfileResponse({
      ...admitted(),
      coordinates: { ...coordinates, profileDigest: "not-a-digest" },
    }, request),
    /64 lowercase hexadecimal/,
  );
  const maximum = {
    state: { kind: "not-admitted", summary: "none" },
    editor: {
      generation: "18446744073709551615",
      documentRevision: "18446744073709551615",
    },
    context,
  };
  assert.deepEqual(decodeEffectiveProfileResponse(maximum, request), maximum);
  assert.throws(
    () => decodeEffectiveProfileResponse({
      ...maximum,
      editor: { ...maximum.editor, generation: "18446744073709551616" },
    }, request),
    /canonical decimal/,
  );
  assert.throws(
    () => decodeEffectiveProfileResponse({
      ...maximum,
      editor: { ...maximum.editor, generation: "9".repeat(1_000) },
    }, request),
    /canonical decimal/,
  );
});

test("profile decoder distinguishes terminal provider absence from malformed binding", () => {
  const nativeOnly = admitted(undefined, [
    { ...entries[7], terminalProviderBinding: null },
  ]);
  assert.equal(
    decodeEffectiveProfileResponse(nativeOnly, request).page.entries[0]
      .terminalProviderBinding,
    null,
  );
  for (const terminalProviderBinding of [
    undefined,
    {},
    { providerBindingId: "provider", providerBindingIdentity: "wrong" },
    // This existing DTO fixture checks wire shape, not provider admission.
    { ...entries[7].terminalProviderBinding, unrecognizedField: true },
  ]) {
    const candidate = { ...entries[7] };
    if (terminalProviderBinding === undefined) {
      delete candidate.terminalProviderBinding;
    } else {
      candidate.terminalProviderBinding = terminalProviderBinding;
    }
    assert.throws(
      () => decodeEffectiveProfileResponse(admitted(undefined, [candidate]), request),
      /unknown or missing fields|64 lowercase hexadecimal/,
    );
  }
});

test("explanation decoder admits every evidence variant and rejects cross-wiring", () => {
  const handle = "d".repeat(64);
  const explanationRequest = { context, handle, expectedCoordinate };
  const evidence = [
    { kind: "declared-selection", selection: entries[0].selection },
    { kind: "responsibility", contractId: "responsibility", executionPathId: "path", reasonSteps },
    { kind: "effective-selection", selection: entries[2].selection },
    { kind: "execution-path", selectionId: "selection", reasonSteps },
    { kind: "operation-classification", exposure: "semantic-only", nativeRealization: "unknown" },
    {
      kind: "capability-classification",
      requirementDisposition: { kind: "satisfied" },
      providerDisposition: { kind: "missing-binding" },
      nativeRealization: "unknown",
    },
    { kind: "finding-classification", finding: entries[11].finding },
    { kind: "machine-realization", realization: "unknown" },
    { kind: "configuration" },
  ];

  for (const member of evidence) {
    const response = {
      editor,
      context,
      coordinates,
      explanation: { handle, evidence: member },
    };
    assert.deepEqual(
      decodeEffectiveProfileExplanationResponse(response, explanationRequest),
      response,
    );
  }

  assert.throws(
    () => decodeEffectiveProfileExplanationResponse({
      editor,
      context,
      coordinates,
      explanation: { handle: "e".repeat(64), evidence: { kind: "configuration" } },
    }, explanationRequest),
    /handle does not match/,
  );
  assert.throws(
    () => decodeEffectiveProfileExplanationResponse({
      editor: { generation: "09", documentRevision: "14" },
      context,
      coordinates,
      explanation: { handle, evidence: { kind: "configuration" } },
    }, explanationRequest),
    /canonical decimal/,
  );
});
