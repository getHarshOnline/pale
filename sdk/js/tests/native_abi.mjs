import { exerciseObservationProfile } from "./observation_profile.mjs";
import { exerciseObservationConformance } from "./observation_conformance.mjs";
import { exerciseOrganization } from "./organization.mjs";
import assert from "node:assert/strict";
import fs from "node:fs";
import crypto from "node:crypto";
import { createRequire } from "node:module";
import os from "node:os";
import path from "node:path";
import { exerciseConfigurationImprovement, exerciseLocalConfigurationImprovement } from "./improvement.mjs";
import { exerciseOneShotCompilation } from "./one_shot_async.mjs";
import { assertIntendedUseExplanation, assertProfileEntry, assertProfilePage } from "./effective_profile.mjs";
import { exerciseAuthoringErrors } from "./authoring_errors.mjs";
import { exerciseAuthoringPages } from "./authoring_pages.mjs";
import { exerciseSubjectComparison } from "./subject_comparison.mjs";
import { exerciseSourceSubjects, exerciseSubjectInspection } from "./source_subjects.mjs";

function writeFixture(root, relativePath, bytes) {
  const target = path.join(root, relativePath);
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.writeFileSync(target, bytes);
}

function createLocalWorkspace(fixtureRoot, compiledIdentity) {
  const root = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), "pale-js-native-abi-")));
  const oneShotRoot = path.join(fixtureRoot, "one-shot");
  for (const name of [
    "composition.toml",
    "pale-workspace.toml",
    "workspace-resolution.lock.toml",
    "workspace-universe.lock.toml",
  ]) {
    writeFixture(root, name, fs.readFileSync(path.join(oneShotRoot, "workspace", name)));
  }
  for (const name of ["capabilities.toml", "distribution-lock.toml"]) {
    writeFixture(root, name, fs.readFileSync(path.join(oneShotRoot, "private", name)));
  }
  for (const name of ["package.toml", "role.md", "roles.toml"]) {
    writeFixture(
      root,
      path.join("distribution", "packages", "engine", name),
      fs.readFileSync(path.join(oneShotRoot, "packages", "engine", name)),
    );
  }

  const checkout = path.join(root, "checkout");
  const authority = path.join(root, "authority");
  const lineage = path.join(root, "lineage");
  for (const directory of [checkout, authority, lineage]) {
    fs.mkdirSync(directory, { recursive: true });
  }
  const executable = fs.realpathSync(process.execPath);
  const executableDigest = crypto
    .createHash("sha256")
    .update(fs.readFileSync(executable))
    .digest("hex");
  const tomlString = (value) => JSON.stringify(value);
  const machine = `[machine]
id = "machine"
schema_version = 1
distribution_root = ${tomlString(path.join(root, "distribution"))}

[[repository_checkouts]]
repository_id = "engine"
checkout_root = ${tomlString(checkout)}

[authority_roots]
authority = ${tomlString(authority)}

[materialization_lineage]
root = ${tomlString(lineage)}
max_route_records_per_collection = 4096

[surface_executable]
exec = ${tomlString(executable)}
digest = ${tomlString(executableDigest)}
version = ${tomlString(compiledIdentity.version)}
protocol = ${tomlString(compiledIdentity.protocol)}
`;
  writeFixture(root, path.join("machines", "machine.toml"), machine);
  writeFixture(checkout, path.join(".git", "info", "exclude"), "# user-owned exclusion\n");
  return {
    root,
    manifest: path.join(root, "pale-workspace.toml"),
    checkout,
    plans: path.join(root, "plans"),
    evidence: path.join(root, "evidence"),
  };
}

const [modulePath, fixtureRoot] = process.argv.slice(2);
const bindings = createRequire(import.meta.url)(path.resolve(modulePath));
const { Pale } = bindings;
const semanticNodeExports = new Set(["ObservationLensNode"]);
assert.deepEqual(
  Object.keys(bindings).filter(
    (name) =>
      name.startsWith("Node") ||
      name.startsWith("Api") ||
      (name.endsWith("Node") && !semanticNodeExports.has(name)),
  ),
  [],
);
const fixture = JSON.parse(fs.readFileSync(`${fixtureRoot}/bundle-identity.json`, "utf8"));
const contract = JSON.parse(fs.readFileSync(`${fixtureRoot}/contract-diagnostic.json`, "utf8"));
const identity = new Pale().contractBundleIdentity();

assert.equal(identity.memberCount, fixture.member_count);
assert.equal(identity.schemaManifestSha256, fixture.schema_manifest_sha256);
assert.equal(identity.schemaSetSha256, fixture.schema_set_sha256);

const api = new Pale();
const organizationResult = exerciseOrganization(bindings, api, path.resolve(fixtureRoot, "../organization"));
exerciseAuthoringErrors(bindings, api);
const inspectionRoot = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), "pale-js-native-control-")));
try {
  fs.chmodSync(inspectionRoot, 0o700);
  const request = bindings.LocalControlStorageInspectionRequest.principal(
    inspectionRoot, "principal-inspection", "store-inspection", "incarnation-inspection",
  );
  const deployment = bindings.LocalControlStorageInspectionRequest.deployment(
    inspectionRoot, "workspace-inspection", "deployment-binding-inspection", "0".repeat(64),
    "store-inspection", "incarnation-inspection",
  );
  for (const [input, expectedKind] of [
    [request, bindings.LocalControlStoreKind.PrincipalTrust],
    [deployment, bindings.LocalControlStoreKind.DeploymentControl],
  ]) {
    const result = api.inspectLocalControlStore(input);
    assert.equal(result.kind, expectedKind);
    assert.equal(result.state, bindings.LocalControlStorageStateKind.Missing);
    assert.equal(result.unresolved, null);
    assert.equal(result.rejection, null);
    assert.deepEqual(Object.keys(input), []);
    assert(!JSON.stringify(result).includes(inspectionRoot));
    assert.deepEqual(fs.readdirSync(inspectionRoot), []);
  }
  for (const input of [
    bindings.LocalControlStorageInspectionRequest.principal(
      "relative/private-root", "principal-inspection", "store-inspection", "incarnation-inspection",
    ),
    bindings.LocalControlStorageInspectionRequest.principal(
      inspectionRoot, "", "store-inspection", "incarnation-inspection",
    ),
    bindings.LocalControlStorageInspectionRequest.deployment(
      inspectionRoot, "workspace-inspection", "deployment-binding-inspection", "not-a-digest",
      "store-inspection", "incarnation-inspection",
    ),
  ]) {
    assert.throws(() => api.inspectLocalControlStore(input), (error) => {
      assert.equal(error.diagnostic.code, "local_control_storage_inspection_failed");
      assert.equal(error.diagnostic.disposition, bindings.DiagnosticDisposition.InvalidInput);
      assert.equal(error.diagnostic.effect, bindings.DiagnosticEffectCertainty.NotApplicable);
      assert.equal(error.diagnostic.retry, bindings.DiagnosticRetry.AfterCorrection);
      assert.equal(error.message, error.diagnostic.summary);
      assert(!Buffer.from(error.diagnostic.canonicalBytes).toString().includes(inspectionRoot));
      return true;
    });
    assert.deepEqual(fs.readdirSync(inspectionRoot), []);
  }
  // A real unsafe descendant must remain untouched and use canonical policy.
  const unsafeDescendant = path.join(inspectionRoot, "pale");
  fs.mkdirSync(unsafeDescendant, { mode: 0o755 });
  fs.chmodSync(unsafeDescendant, 0o755);
  assert.throws(() => api.inspectLocalControlStore(request), (error) => {
    assert.equal(error.diagnostic.disposition, bindings.DiagnosticDisposition.Blocked);
    assert.equal(error.diagnostic.effect, bindings.DiagnosticEffectCertainty.NotApplicable);
    assert(!Buffer.from(error.diagnostic.canonicalBytes).toString().includes(inspectionRoot));
    return true;
  });
  assert.deepEqual(fs.readdirSync(unsafeDescendant), []);
  assert.equal(fs.statSync(unsafeDescendant).mode & 0o777, 0o755);
} finally {
  fs.rmSync(inspectionRoot, { recursive: true, force: true });
}
assert.throws(
  () => api.contractSchema("not-a-selected-schema"),
  (error) => {
    assert.equal(error.message, error.diagnostic.summary);
    assert.equal(error.diagnostic.code, "contract_rejected");
    assert.equal(
      JSON.parse(Buffer.from(error.diagnostic.canonicalBytes).toString()).code,
      "contract_rejected",
    );
    return true;
  },
);
const schema = api.contractSchema(contract.schema.registry_id);
assert.equal(schema.registryId, contract.schema.registry_id);
assert.equal(schema.schemaUri, contract.schema.schema_uri);
assert.equal(schema.wireSchemaId, contract.schema.wire_schema_id);
assert.equal(schema.schemaSha256, contract.schema.schema_sha256);
assert.equal(schema.bytes.length, contract.schema.byte_length);

const projection = {
  code: contract.projection.code,
  stage: bindings.DiagnosticStage.InputAdmission,
  disposition: bindings.DiagnosticDisposition.InvalidInput,
  summary: contract.projection.summary,
  retry: bindings.DiagnosticRetry.AfterCorrection,
  effect: bindings.DiagnosticEffectCertainty.NotApplicable,
  causes: [],
  omittedCauses: 0,
  help: [],
};
const diagnostic = api.projectDiagnostic(projection);
assert.equal(Buffer.from(diagnostic).toString(), contract.canonical_diagnostic);
const outcomeUnknown = api.projectDiagnostic({
  ...projection,
  code: contract.outcome_unknown.code,
  stage: bindings.DiagnosticStage.Materialization,
  disposition: bindings.DiagnosticDisposition.EffectOutcomeUnknown,
  summary: contract.outcome_unknown.summary,
  retry: bindings.DiagnosticRetry.ReconcileFirst,
  effect: bindings.DiagnosticEffectCertainty.OutcomeUnknown,
});
assert.equal(
  Buffer.from(outcomeUnknown).toString(),
  contract.outcome_unknown.canonical_diagnostic,
);
const recoveryDiagnostic = JSON.parse(Buffer.from(api.projectDiagnostic({
  ...projection,
  code: "resident_recovery_required",
  stage: bindings.DiagnosticStage.ResidentRecovery,
  disposition: bindings.DiagnosticDisposition.Blocked,
  summary: "resident recovery evidence must be reconciled",
  retry: bindings.DiagnosticRetry.ReconcileFirst,
})).toString());
assert.equal(recoveryDiagnostic.stage, "resident_recovery");
assert.equal(recoveryDiagnostic.retry, "reconcile_first");
assert.equal(recoveryDiagnostic.effect, "not_applicable");
assert.equal(
  Buffer.from(api.admitDiagnostic(Buffer.from(contract.canonical_diagnostic))).toString(),
  contract.canonical_diagnostic,
);
const validated = api.validateContractDocument(
  contract.schema.registry_id,
  Buffer.from(contract.canonical_diagnostic),
);
assert.equal(validated.schemaId, contract.schema.registry_id);
assert.equal(validated.schemaSha256, contract.schema.schema_sha256);
assert.equal(Buffer.from(validated.canonicalBytes).toString(), contract.canonical_diagnostic);

const portable = JSON.parse(fs.readFileSync(`${fixtureRoot}/portable-journey.json`, "utf8"));
const documents = portable.document_labels.map((label) => ({
  label,
  bytes: fs.readFileSync(`${fixtureRoot}/portable/${label}`),
}));
const request = () => ({ rootManifest: portable.root_manifest, documents });
const subjectComparisonCapabilities = exerciseSubjectComparison(bindings, api, request);
const sourceSubjectsCapability = exerciseSourceSubjects(bindings, api, request);
const subjectInspectionCapability = exerciseSubjectInspection(bindings, api, request);
const workspaceLimits = api.authoringIngressLimits(bindings.AuthoringIngressClass.Workspace);
assert.equal(workspaceLimits.maxDocuments, portable.workspace_authoring_limits.max_documents);
assert.equal(typeof workspaceLimits.maxDocumentBytes, "bigint");
assert.equal(
  workspaceLimits.maxDocumentBytes,
  BigInt(portable.workspace_authoring_limits.max_document_bytes),
);
assert.equal(typeof workspaceLimits.maxTotalBytes, "bigint");
assert.equal(
  workspaceLimits.maxTotalBytes,
  BigInt(portable.workspace_authoring_limits.max_total_bytes),
);
const rootDocument = documents.find(({ label }) => label === portable.root_manifest);
api.validateStandaloneAuthoring(
  bindings.StandaloneAuthoringRole.WorkspaceManifest,
  rootDocument.label,
  rootDocument.bytes,
);
const standaloneInspection = api.inspectStandaloneAuthoring(
  bindings.StandaloneAuthoringRole.WorkspaceManifest,
  [],
);
assert.equal(
  standaloneInspection.state,
  bindings.AuthoringInspectionState.Available,
);
assert(standaloneInspection.node.fields.length > 0);
assert.throws(
  () => api.inspectStandaloneAuthoring(
    bindings.StandaloneAuthoringRole.WorkspaceManifest,
    Array.from(
      { length: 65 },
      (_, index) => bindings.AuthoringPathSegment.field(`over-bound-${index}`),
    ),
  ),
  (error) => {
    assert.ok(error instanceof Error);
    assert.equal(error.message, error.diagnostic.summary);
    assert.equal(error.diagnostic.code, "authoring_path_too_deep");
    assert.equal(error.diagnostic.stage, bindings.DiagnosticStage.InputAdmission);
    assert.equal(
      error.diagnostic.disposition,
      bindings.DiagnosticDisposition.InvalidInput,
    );
    assert.equal(error.diagnostic.retry, bindings.DiagnosticRetry.AfterCorrection);
    assert.equal(
      error.diagnostic.effect,
      bindings.DiagnosticEffectCertainty.NotApplicable,
    );
    assert.deepEqual(
      JSON.parse(Buffer.from(error.diagnostic.canonicalBytes).toString()),
      {
        code: "authoring_path_too_deep",
        disposition: "invalid_input",
        effect: "not_applicable",
        facts: [
          { key: "unit", value: "segments" },
          { key: "observed", value: "65" },
          { key: "maximum", value: "64" },
        ],
        help: ["Reduce the authoring path to the stated maximum, then retry."],
        retry: "after_correction",
        schema: "pale/diagnostic.v1",
        schema_sha256: contract.schema.schema_sha256,
        schema_version: 1,
        severity: "error",
        stage: "input_admission",
        summary: "authoring path exceeds the editor-query bound",
        truncation: {
          omitted_causes: 1,
          omitted_facts: 0,
          omitted_help: 0,
          omitted_notes: 0,
          shortened_fields: 0,
        },
      },
    );
    return true;
  },
);
const prior = api.compilePortableWorkspace(request());
const current = api.compilePortableWorkspace(request());
const portableInspection = api.inspectPortableWorkspaceAuthoring(
  prior,
  portable.root_manifest,
  [],
);
assert.equal(portableInspection.state, bindings.AuthoringInspectionState.Available);
const field = (name) => bindings.AuthoringInstancePathSegment.field(name);
const portableNavigation = api.navigatePortableWorkspaceAuthoring(
  prior,
  portable.root_manifest,
  [field("workspace"), field("workspace_id")],
);
const portableIdentity = portableNavigation.identity();
assert.equal(portableIdentity.value, portable.compilation.workspace_id);
assert.equal(portableNavigation.definition().sourceLabel, portable.root_manifest);
const compilationFields = {
  workspace_id: prior.workspaceId,
  principal_universe_digest: prior.principalUniverseDigest,
  semantic_ir_digest: prior.semanticIrDigest,
  admission_evidence_digest: prior.admissionEvidenceDigest,
  source_document_count: prior.sourceDocumentCount,
  derivation_source_count: prior.derivationSourceCount,
  derivation_node_count: prior.derivationNodeCount,
  admission_evidence_count: prior.admissionEvidenceCount,
  adopted_release_count: prior.adoptedReleaseCount,
  package_release_count: prior.packageReleaseCount,
  normative_pack_release_count: prior.normativePackReleaseCount,
};
assert.deepEqual(compilationFields, portable.compilation);
const comparison = api.comparePortableWorkspaces(prior, current);
assert.deepEqual(
  {
    prior_principal_universe_digest: comparison.priorPrincipalUniverseDigest,
    current_principal_universe_digest: comparison.currentPrincipalUniverseDigest,
    prior_semantic_ir_digest: comparison.priorSemanticIrDigest,
    current_semantic_ir_digest: comparison.currentSemanticIrDigest,
    receipt_digest: comparison.receiptDigest,
    change_root_count: comparison.changeRootCount,
    affected_count: comparison.affectedCount,
    unaffected_count: comparison.unaffectedCount,
  },
  portable.comparison,
);
const roots = comparison.changeRoots(0, 65535);
assert.equal(roots.entries.length, portable.change_roots.entry_count);
assert.equal(roots.nextOffset ?? null, portable.change_roots.next_offset);
const impactPage = comparison.impacts(0, 1);
assert.equal(impactPage.nextOffset, portable.first_impact.next_offset);
const impactBytes = impactPage.canonicalBytes;
assert(Buffer.isBuffer(impactBytes));
assert.deepEqual(JSON.parse(impactBytes), {
  entries: [{
    subject: {
      kind: portable.first_impact.kind,
      sourceId: portable.first_impact.source_id,
    },
    state: portable.first_impact.state,
    contributingRootCount: portable.first_impact.contributing_root_count,
  }],
  nextOffset: portable.first_impact.next_offset,
});
const repeatedImpactPage = comparison.impacts(0, 1);
assert.deepEqual(repeatedImpactPage.canonicalBytes, impactBytes);
assert.equal(
  crypto.createHash("sha256").update(repeatedImpactPage.canonicalBytes).digest("hex"),
  crypto.createHash("sha256").update(impactBytes).digest("hex"),
);
impactPage.canonicalBytes.fill(0);
assert.deepEqual(impactPage.canonicalBytes, impactBytes);
const detachedImpacts = impactPage.entries;
detachedImpacts.length = 0;
assert.equal(impactPage.entries.length, 1);
assert.throws(() => new bindings.WorkspaceImpactPage(), /no `constructor`/);
assert.throws(
  () => comparison.impacts({ offset: 0 }, 1),
  (error) => error instanceof Error && error.code === "NumberExpected",
);
const impact = impactPage.entries[0];
const subject = impact.subject();
assert.equal(subject.kind, bindings.WorkspaceSubjectKind.Source);
assert.equal(subject.sourceId, portable.first_impact.source_id);
for (const field of ["workspaceId", "componentId", "moduleId", "ownerModuleId", "declarationNamespaceId", "declarationId", "importerModuleId", "upstreamModuleId", "upstreamReleaseId", "exportId", "sourceNamespaceId", "localNamespaceId", "extensionNamespaceId", "resourceId"]) assert.equal(subject[field], null);
assert.equal(impact.state, bindings.WorkspaceImpactState.Unaffected);
assert.equal(impact.contributingRootCount, 0);
const explanation = comparison.explain(subject);
assert.equal(explanation.state, bindings.WorkspaceExplanationState.Unaffected);
assert.equal(explanation.frontier, null);
assert.equal(explanation.root(), null);
assert.deepEqual(explanation.steps(), []);

const action = JSON.parse(fs.readFileSync(`${fixtureRoot}/native-action-journey.json`, "utf8"));
const actionDescriptors = api.nativeActionCatalog();
assert.deepEqual(actionDescriptors.map((descriptor) => descriptor.actionId), action.descriptor_ids);
assert.deepEqual(actionDescriptors.map((descriptor) => [descriptor.actionId, descriptor.description, crypto.createHash("sha256").update(descriptor.contractBytes).digest("hex"), descriptor.contractBytes.length, crypto.createHash("sha256").update(descriptor.payloadSchemaBytes).digest("hex"), descriptor.payloadSchemaBytes.length]), action.descriptor_projections);
const taggedExecution = api.invokeNativeAction(action.action_id, Buffer.from(action.tagged_input));
const payloadExecution = api.invokeNativeActionPayload(action.action_id, Buffer.from(action.payload_input));
assert.deepEqual(Buffer.from(taggedExecution), Buffer.from(payloadExecution));
assert.equal(payloadExecution.length, action.execution_length);
assert.equal(crypto.createHash("sha256").update(payloadExecution).digest("hex"), action.execution_sha256);

const oneShot = JSON.parse(fs.readFileSync(`${fixtureRoot}/one-shot-journey.json`, "utf8"));
const oneShotRoot = `${fixtureRoot}/one-shot`;
const privateDocument = (label, name) => ({
  label,
  bytes: fs.readFileSync(`${oneShotRoot}/private/${name}`),
});
const effectiveWorkspaceRequest = {
  rootManifest: "pale-workspace.toml",
  documents: ["composition.toml", "pale-workspace.toml", "workspace-resolution.lock.toml", "workspace-universe.lock.toml"].map((label) => ({ label, bytes: fs.readFileSync(`${oneShotRoot}/workspace/${label}`) })),
};
const effectiveWorkspace = api.compilePortableWorkspace(effectiveWorkspaceRequest);
const effectiveRequest = {
  projectId: oneShot.selectors.project_id,
  executionWorkspaceId: oneShot.selectors.execution_workspace_id,
  machineId: oneShot.selectors.machine_id,
  privateInputs: {
    capabilities: privateDocument("capabilities.toml", "capabilities.toml"),
    machineBinding: privateDocument("machines/machine.toml", "machine-binding.toml"),
    distributionLock: privateDocument("distribution-lock.toml", "distribution-lock.toml"),
    lifecycleGuidance: [],
  },
  packageDocuments: ["package.toml", "role.md", "roles.toml"].map((name) => ({ label: `engine/${name}`, bytes: fs.readFileSync(`${oneShotRoot}/packages/engine/${name}`) })),
};
const effective = api.compileEffectiveProfile(effectiveWorkspace, effectiveRequest);

// Reuse the canonical SDK's private semantic-selection fixture route. Only the
// request bytes change; the checked-in Package and workspace corpus stay exact.
for (const declaredRole of [
  '{ selection = "exact", resource = { package_id = "example.engine", resource_id = "engine" } }',
  '{ selection = "all_from_package", package_id = "example.engine" }',
]) {
  const declaredProfile = api.compileEffectiveProfile(effectiveWorkspace, {
    ...effectiveRequest,
    privateInputs: {
      ...effectiveRequest.privateInputs,
      capabilities: {
        label: "capabilities.toml",
        bytes: Buffer.concat([
          Buffer.from(`[semantic]\nroles = [${declaredRole}]\n\n`),
          effectiveRequest.privateInputs.capabilities.bytes,
        ]),
      },
    },
  });
  const page = declaredProfile.inspect(bindings.EffectiveProfileSection.IntendedUseCase, 0, 256);
  const canonical = JSON.parse(Buffer.from(page.canonicalBytes).toString());
  const index = canonical.entries.findIndex((row) => row.kind === "declared-selection");
  assert(index >= 0, "compiled declared intent must produce a real row");
  const row = canonical.entries[index];
  console.log(JSON.stringify({ declaredSelectionWitness: row.selection }));
  const entry = page.entries[index];
  const selection = entry.detail.selection;
  assert.equal(selection.kind, "resource", "declared selection must use literal discrimination");
  assertProfilePage(bindings, page);
  assertProfileEntry(bindings, entry, row);
  const explanation = declaredProfile.explain(entry.explanationHandle);
  assert.equal(explanation.handle, entry.explanationHandle);
  assert.equal(explanation.evidence.kind, "declared-selection");
  assert.deepEqual(explanation.evidence.selection, selection);
  const copied = selection.selector;
  const key = copied.kind === "exact" ? "resourceId" : "packageId";
  const original = copied[key];
  copied[key] = "changed copied declaration";
  assert.equal(page.entries[index].detail.selection.selector[key], original);
  assert.equal(declaredProfile.explain(entry.explanationHandle).evidence.selection.selector[key], original);
}
const effectiveCoordinates = effective.coordinates();
const improvementCapability = exerciseConfigurationImprovement(bindings, fixtureRoot, api, effectiveWorkspace, effective);
assert.equal(effectiveCoordinates.projectId, oneShot.selectors.project_id);
assert.equal(effectiveCoordinates.executionWorkspaceId, oneShot.selectors.execution_workspace_id);
assert.equal(effectiveCoordinates.profileDigest, effective.profileDigest);
assert.match(effectiveCoordinates.selectedTopologyDigest, /^[0-9a-f]{64}$/);
const effectivePage = effective.inspect(bindings.EffectiveProfileSection.IntendedUseCase, 0, 1);
const effectivePageValue = assertProfilePage(bindings, effectivePage);
assert.equal(effectivePageValue.section, "intended-use-case");
assert.equal(effectivePage.offset, effectivePageValue.offset);
assert.equal(effectivePage.entryCount, effectivePageValue.entries.length);
assert.equal(effectivePage.nextOffset ?? null, effectivePageValue.nextOffset ?? null);
assertIntendedUseExplanation(bindings, effective, effectivePageValue.entries[0]);
const realizationPage = JSON.parse(Buffer.from(effective.inspect(
  bindings.EffectiveProfileSection.MachineRealization, 0, 1,
).canonicalBytes));
const realizationRow = realizationPage.entries[0];
const realizationExplanation = effective.explain(realizationRow.explanationHandle);
assert.equal(realizationExplanation.handle, realizationRow.explanationHandle);
assert.equal(realizationExplanation.evidence.kind, "machine-realization");
assert.equal(realizationRow.realization, "unknown");
assert.equal(realizationExplanation.evidence.realization, bindings.EffectiveProfileNativeRealization.Unknown);
assert.deepEqual(Object.keys(realizationExplanation.evidence).sort(), ["kind", "realization"]);
assert.equal(effective.explain("0".repeat(64)), null);
assert.throws(() => effective.explain("not-a-handle"));
const effectiveAuthoringInspection = effective.inspectAuthoring(
  "pale-workspace.toml",
  [],
);
assert.equal(
  effectiveAuthoringInspection.state,
  bindings.AuthoringInspectionState.Available,
);
const effectiveNavigation = effective.navigateAuthoring(
  "pale-workspace.toml",
  [
    field("workspace"),
    field("desired_topology"),
    field("projects"),
    bindings.AuthoringInstancePathSegment.index(0),
    field("run_policy"),
  ],
);
const effectiveIdentity = effectiveNavigation.identity();
assert.equal(effectiveIdentity.kind, bindings.AuthoringIdentityKind.RunPolicy);
const effectiveJoin = effective.inspectEffectiveAuthoring(effectiveIdentity);
assert.equal(effectiveJoin.coordinates.profileDigest, effective.profileDigest);
assert.equal(
  effectiveJoin.coordinates.selectedTopologyDigest,
  effectiveCoordinates.selectedTopologyDigest,
);
assert(Array.isArray(effectiveJoin.entries), "authoring join must expose complete typed entries");
assert.equal(Object.hasOwn(effectiveJoin, "explanationHandles"), false);
assert(effectiveJoin.entries.length > 0);
assert.deepEqual(effectiveJoin.coordinates, effectiveCoordinates);
assert.equal(effectiveJoin.truncated, false);
const profileRows = new Map();
const effectiveProfileEntryKinds = new Set();
const effectiveProfileParticipantKinds = new Set();
const effectiveProfileResponsibilityKinds = new Set();
const effectiveProfileSelectionKinds = new Set();
let retainedPathRow;
let retainedResponsibilityRow;
for (const section of [bindings.EffectiveProfileSection.IntendedUseCase,
  bindings.EffectiveProfileSection.EffectiveConfiguration,
  bindings.EffectiveProfileSection.CapabilitiesAndOperations,
  bindings.EffectiveProfileSection.MachineRealization,
  bindings.EffectiveProfileSection.GapsAndExplanations]) {
  let offset = 0;
  const handles = [];
  do {
    const page = effective.inspect(section, offset, 1);
    const canonical = assertProfilePage(bindings, page);
    for (const [index, row] of canonical.entries.entries()) {
      assert.equal(profileRows.has(row.explanationHandle), false);
      profileRows.set(row.explanationHandle, row);
      if (row.kind === "execution-path") retainedPathRow = page.entries[index];
      if (row.kind === "selection") {
        effectiveProfileSelectionKinds.add(row.selection.kind);
        const selected = page.entries[index].detail.selection;
        assert.deepEqual(effective.explain(row.explanationHandle).evidence.selection, selected);
        if (selected.kind === "run-policy") {
          selected.runPolicyId = "changed copied selection";
          assert.equal(page.entries[index].detail.selection.runPolicyId, row.selection.runPolicyId);
        }
      }
      if (row.kind === "responsibility") {
        retainedResponsibilityRow = page.entries[index];
        effectiveProfileParticipantKinds.add(row.participant.kind);
        row.responsibilities.forEach((value) => effectiveProfileResponsibilityKinds.add(value.kind));
      }
      effectiveProfileEntryKinds.add(row.kind);
      handles.push(row.explanationHandle);
      assert.equal(effective.explain(page.entries[index].explanationHandle).handle, row.explanationHandle);
    }
    offset = page.nextOffset ?? null;
  } while (offset !== null);
  const maximum = effective.inspect(section, 0, 65535);
  assertProfilePage(bindings, maximum);
  assert(maximum.entries.length <= 256);
  assert.deepEqual(maximum.entries.map((row) => row.explanationHandle), handles);
  const defaultPage = effective.inspect(section, 0, 0);
  assertProfilePage(bindings, defaultPage);
  assert.deepEqual(Buffer.from(defaultPage.canonicalBytes), Buffer.from(effective.inspect(section, 0, 64).canonicalBytes));
  const beyond = effective.inspect(section, 0xffffffff, 1);
  assertProfilePage(bindings, beyond);
  assert.deepEqual(beyond.entries, []);
  assert.equal(beyond.offset, 0xffffffff);
  assert.equal(beyond.nextOffset ?? null, null);
}
assert.deepEqual([...effectiveProfileSelectionKinds].sort(), ["authority-root", "package", "run-policy", "semantic-resource"]);
const authoringPageCapabilities = exerciseAuthoringPages(bindings, api, {
  workspace: prior,
  rebuiltWorkspace: current,
  portableNavigation,
  effective,
  rebuiltEffective: api.compileEffectiveProfile(effectiveWorkspace, effectiveRequest),
  effectiveWorkspace,
  effectiveIdentity,
  profileRows,
});
for (const row of effectiveJoin.entries) {
  assertProfileEntry(bindings, row, profileRows.get(row.explanationHandle));
  const detail = row.detail;
  assert.equal(detail.kind, "selection");
  assert.equal(detail.selection.kind, "run-policy");
  assert.equal(Object.hasOwn(bindings, "EffectiveProfileSelectionKind"), false);
  assert.equal(Object.hasOwn(bindings, "EffectiveProfileSelection"), false);
  assert.equal(detail.selection.runPolicyId, "standard");
}
assert.equal(Object.hasOwn(bindings, "EffectiveProfileEntryKind"), false);
assert.equal(Object.hasOwn(bindings, "EffectiveProfileParticipantKind"), false);
assert.equal(Object.hasOwn(bindings, "EffectiveProfileResponsibilityKind"), false);
const repositoryIdentity = effective.navigateAuthoring("pale-workspace.toml", [
  field("workspace"), field("desired_topology"), field("repositories"),
  bindings.AuthoringInstancePathSegment.index(0), field("id"),
]).identity();
assert.equal(repositoryIdentity.kind, bindings.AuthoringIdentityKind.Repository);
const repositoryJoin = effective.inspectEffectiveAuthoring(repositoryIdentity);
assert(repositoryJoin.entries.length > 1);
assert.deepEqual(repositoryJoin.coordinates, effectiveCoordinates);
const repositoryHandles = repositoryJoin.entries.map((entry) => entry.explanationHandle);
const allHandles = [...profileRows.keys()];
assert.deepEqual(repositoryHandles.map((handle) => allHandles.indexOf(handle)),
  repositoryHandles.map((handle) => allHandles.indexOf(handle)).sort((a, b) => a - b));
repositoryJoin.entries.forEach((row) => assertProfileEntry(bindings, row, profileRows.get(row.explanationHandle)));
const copiedRow = effectivePage.entries[0];
effectivePage.entries.length = 0;
globalThis.gc();
assertProfileEntry(bindings, copiedRow, effectivePageValue.entries[0]);
const pathRow = retainedPathRow;
assert(pathRow, "fixture must emit its selected execution path");
const pathDetail = pathRow.detail;
assert.equal(pathDetail.kind, "execution-path");
assert.equal(Object.hasOwn(pathDetail, "terminalProviderBinding"), true);
assert.equal(pathDetail.terminalProviderBinding === null, profileRows.get(pathRow.explanationHandle).terminalProviderBinding === null);
const originalEffects = [...pathDetail.effectCeiling];
pathDetail.effectCeiling.push(bindings.EffectiveProfileEffectClass.Publish);
pathDetail.subject.kind = "changed-copy";
assert.deepEqual(pathRow.detail.effectCeiling, originalEffects);
assert.equal(pathRow.detail.subject.kind, "experience-composition");
assert(retainedResponsibilityRow, "fixture must emit its responsibility contract");
const responsibilityCopy = retainedResponsibilityRow.detail;
const participantField = Object.keys(responsibilityCopy.participant).find((key) => key !== "kind");
assert(participantField);
responsibilityCopy.participant[participantField] = "changed-copy";
responsibilityCopy.responsibilities.push({ kind: "changed-copy" });
assertProfileEntry(bindings, retainedResponsibilityRow, profileRows.get(retainedResponsibilityRow.explanationHandle));
const observationProfileCapability = exerciseObservationProfile(
  bindings, api, effectiveWorkspace, effective,
  api.compileEffectiveProfile(effectiveWorkspace, effectiveRequest),
  api.compilePortableWorkspace(effectiveWorkspaceRequest),
  api.compileEffectiveProfile(effectiveWorkspace, {
    ...effectiveRequest,
    privateInputs: {
      ...effectiveRequest.privateInputs,
      capabilities: {
        ...effectiveRequest.privateInputs.capabilities,
        bytes: Buffer.concat([
          Buffer.from('[semantic]\nroles = [{ selection = "all_from_package", package_id = "example.engine" }]\n\n'),
          effectiveRequest.privateInputs.capabilities.bytes,
        ]),
      },
    },
  }),
  profileRows,
);
const observationSnapshot = api.observeEffectiveProfile(effective);
assert(observationSnapshot.factCount > 0);
assert.equal(observationSnapshot.omittedFactCount, 0);
assert.equal(
  observationSnapshot.cursor.generation.profileDigest,
  effective.profileDigest,
);
assert.equal(
  observationSnapshot.cursor.generation.instanceDigest,
  effective.instanceDigest,
);
assert.equal(
  observationSnapshot.cursor.generation.compositionPlanDigest,
  effective.compositionPlanDigest,
);
const observationFilter = new bindings.ObservationFilter();
const observationPageResult = observationSnapshot.page(
  observationFilter,
  {
    maximumInspected: 2,
    maximumReturned: 2,
    maximumCopiedStringBytes: 2 ** 32 - 1,
  },
  bindings.ObservationCancellation.Active,
);
assert.equal(
  observationPageResult.kind,
  bindings.ObservationPageResultKind.Ready,
);
assert.equal(observationPageResult.page.facts.length, 2);
assert.equal(observationPageResult.page.snapshotOmittedCount, 0);
assert.notEqual(observationPageResult.page.continuation, null);
assert.notEqual(observationPageResult.page.continuation, undefined);
const mismatchedObservationPage = observationSnapshot.page(
  observationFilter,
  {
    maximumInspected: 2,
    maximumReturned: 3,
    maximumCopiedStringBytes: 2 ** 32 - 1,
  },
  bindings.ObservationCancellation.Active,
  observationPageResult.page.continuation,
);
assert.equal(
  mismatchedObservationPage.kind,
  bindings.ObservationPageResultKind.Resynchronize,
);
assert.notEqual(mismatchedObservationPage.resynchronization, null);
assert.notEqual(mismatchedObservationPage.resynchronization, undefined);
assert.equal(
  mismatchedObservationPage.resynchronization.reason,
  bindings.ObservationResynchronizationReason.ContinuationMismatch,
);
// This fixture declares no operations or capability requirements. Its actual
// observation facts must not acquire configured outcomes from another axis.
for (const fact of observationPageResult.page.facts) {
  assert.notEqual(fact.category, bindings.ObservationCategory.OperationExposure);
  assert.notEqual(fact.category, bindings.ObservationCategory.CapabilityRequirementDisposition);
  assert.equal(fact.operationExposure, null);
  assert.equal(fact.requirementDisposition, null);
}
const observationSubject = observationPageResult.page.facts
  .map((fact) => fact.subject)
  .find((subject) => subject !== undefined);
assert.notEqual(observationSubject, undefined);
const causal = observationSnapshot.causalPath(observationSubject);
assert.notEqual(causal.kind, bindings.ObservationCausalResultKind.Resynchronize);
const caughtUp = observationSnapshot.deltaFrom(
  observationSnapshot,
  1,
  bindings.ObservationCancellation.Active,
);
assert.equal(caughtUp.kind, bindings.ObservationDeltaOutcomeKind.CaughtUp);
const cancelledLens = observationSnapshot.visualLens(
  observationFilter,
  {
    maximumInspected: 4,
    maximumNodes: 4,
    maximumCopiedStringBytes: 2 ** 32 - 1,
    maximumCausalPathLookups: 4,
    maximumCausalSteps: 256,
  },
  true,
  bindings.ObservationCancellation.Cancelled,
);
assert.equal(
  cancelledLens.kind,
  bindings.ObservationLensDispositionKind.Cancelled,
);
const readyLens = observationSnapshot.visualLens(
  observationFilter,
  {
    maximumInspected: 4,
    maximumNodes: 4,
    maximumCopiedStringBytes: 2 ** 32 - 1,
    maximumCausalPathLookups: 4,
    maximumCausalSteps: 256,
  },
  false,
  bindings.ObservationCancellation.Active,
);
assert.equal(readyLens.kind, bindings.ObservationLensDispositionKind.Ready);
assert(readyLens.lens.nodes.length <= 4);
assert(readyLens.lens.nodes.every(node => node.facts.length > 0));
assert(readyLens.lens.cursor);
assert.equal(readyLens.lens.snapshotOmittedCount, 0);
exerciseObservationConformance(bindings, observationSnapshot, fixtureRoot);
const compilation = await exerciseOneShotCompilation(bindings, api, {
  projectId: oneShot.selectors.project_id,
  executionWorkspaceId: oneShot.selectors.execution_workspace_id,
  machineId: oneShot.selectors.machine_id,
  privateInputs: {
    capabilities: privateDocument("capabilities.toml", "capabilities.toml"),
    machineBinding: privateDocument("machines/machine.toml", "machine-binding.toml"),
    distributionLock: privateDocument("distribution-lock.toml", "distribution-lock.toml"),
    lifecycleGuidance: [],
  },
  packageDocuments: ["package.toml", "role.md", "roles.toml"].map((name) => ({ label: `engine/${name}`, bytes: fs.readFileSync(`${oneShotRoot}/packages/engine/${name}`) })),
  workspace: {
    rootManifest: "pale-workspace.toml",
    documents: ["composition.toml", "pale-workspace.toml", "workspace-resolution.lock.toml", "workspace-universe.lock.toml"].map((label) => ({ label, bytes: fs.readFileSync(`${oneShotRoot}/workspace/${label}`) })),
  },
});
const summary = api.oneShotSummary(compilation);
assert.deepEqual({
  project_id: summary.projectId,
  execution_workspace_id: summary.executionWorkspaceId,
  profile_digest: summary.profileDigest,
  instance_digest: summary.instanceDigest,
  one_shot_compilation_digest: summary.oneShotCompilationDigest,
  schema_manifest_sha256: summary.schemaManifestSha256,
  schema_set_sha256: summary.schemaSetSha256,
  repository_ids: summary.repositoryIds,
  canonical_sha256: crypto.createHash("sha256").update(summary.canonicalBytes).digest("hex"),
  canonical_length: summary.canonicalBytes.length,
}, oneShot.summary);
const priorOwnership = bindings.OneShotPriorMaterialization.unowned();
const observation = new bindings.OneShotObservationRequest(oneShot.selectors.repository_id, bindings.OneShotPlanKind.Apply, priorOwnership);
const targets = api.oneShotObservationTargets(compilation, observation);
assert.deepEqual(targets, oneShot.observation_targets);
const baseline = targets.map((target) => bindings.BaselineEntry.absent(target));
const planning = new bindings.OneShotPlanningRequest(oneShot.selectors.repository_id, bindings.OneShotPlanKind.Apply, priorOwnership, baseline);
const plan = api.planOneShot(compilation, planning);
assert.deepEqual({
  repository_id: plan.repositoryId,
  plan_digest: plan.planDigest,
  one_shot_compilation_digest: plan.oneShotCompilationDigest,
  operation_count: plan.operationCount,
  deferred_effect_count: plan.deferredEffectCount,
  canonical_sha256: crypto.createHash("sha256").update(plan.canonicalBytes).digest("hex"),
  canonical_length: plan.canonicalBytes.length,
}, oneShot.plan);
const boundPlan = api.bindOneShotMachineEffects(plan, []);
assert.equal(boundPlan.repositoryId, oneShot.selectors.repository_id);
const boundPlanValue = JSON.parse(Buffer.from(boundPlan.canonicalBytes).toString());
assert.equal(boundPlanValue.schema, "pale.materialization-plan.v1");
assert.equal(boundPlanValue.surface_plan.plan_digest, plan.planDigest);
assert.deepEqual(boundPlanValue.machine_local_effects, []);
const admittedPlan = api.admitOneShotSavedPlan(boundPlan, boundPlan.canonicalBytes);
assert.equal(admittedPlan.repositoryId, oneShot.selectors.repository_id);
assert.equal(admittedPlan.planDigest, boundPlanValue.plan_digest);
assert.throws(
  () => api.bindOneShotMachineEffects(plan, []),
  /already been consumed/,
);
assert.throws(
  () => api.admitOneShotSavedPlan(boundPlan, boundPlan.canonicalBytes),
  /already been consumed/,
);

const repositoryAdmission = JSON.parse(fs.readFileSync(`${fixtureRoot}/repository-admission.json`, "utf8"));
const admitted = api.admitRepositoryNativeActions(
  fs.readFileSync(`${fixtureRoot}/repository-catalog.json`),
  repositoryAdmission.profile_digest,
  repositoryAdmission.instance_digest,
  repositoryAdmission.repository_id,
);
assert.equal(admitted.profileDigest, repositoryAdmission.profile_digest);
assert.equal(admitted.instanceDigest, repositoryAdmission.instance_digest);
assert.equal(admitted.repositoryId, repositoryAdmission.repository_id);
assert.deepEqual(admitted.descriptors().map((descriptor) => descriptor.actionId), action.descriptor_ids);

const packageInspection = api.inspectPackageDistribution(
  ["package.toml", "role.md", "roles.toml"].map((name) => ({
    label: `engine/${name}`,
    bytes: fs.readFileSync(`${oneShotRoot}/packages/engine/${name}`),
  })),
);
assert.deepEqual(
  packageInspection.packages.map(({ id, version }) => [id, version]),
  [["example.engine", "1.0.0"]],
);
assert.deepEqual(
  JSON.parse(Buffer.from(packageInspection.canonicalBytes).toString()).packages.map(
    ({ id, version }) => [id, version],
  ),
  [["example.engine", "1.0.0"]],
);

const residentAdmission = api.admitResidentHook({
  event: "session-start",
  workspace: "workspace-a",
  repository: "repository-a",
  providerBinding: "provider-binding-claude-code",
  providerBindingIdentity: "ec03a4e4096f39aa0a8c2e3fe9b69705b0ff620963137a0558224b9826a4f3b6",
  payload: Buffer.from('{"hook_event_name":"SessionStart","session_id":"native-session"}'),
});
assert.throws(
  () => api.residentHookTransportUnavailable(residentAdmission),
  (error) => {
    assert.equal(error.diagnostic.code, "resident_hook_transport_unavailable");
    assert.equal(
      error.diagnostic.effect,
      bindings.DiagnosticEffectCertainty.NotStarted,
    );
    return true;
  },
);

const compiledIdentity = api.compiledInterfaceIdentity();
assert.equal(compiledIdentity.version, "0.0.1");
assert.equal(
  JSON.parse(Buffer.from(compiledIdentity.canonicalBytes).toString()).protocol,
  compiledIdentity.protocol,
);

const local = createLocalWorkspace(fixtureRoot, compiledIdentity);
const localImprovementCapability = await exerciseLocalConfigurationImprovement(
  bindings, fixtureRoot, api, createLocalWorkspace, compiledIdentity,
);
try {
  const localSelection = {
    workspaceManifest: local.manifest,
    projectId: oneShot.selectors.project_id,
    executionWorkspaceId: oneShot.selectors.execution_workspace_id,
    machineId: oneShot.selectors.machine_id,
  };
  const localCompilation = api.compileLocalOneShot(localSelection);
  const localSummary = localCompilation.summary();
  assert.equal(localSummary.projectId, oneShot.selectors.project_id);
  assert.equal(localSummary.executionWorkspaceId, oneShot.selectors.execution_workspace_id);
  assert.deepEqual(localSummary.repositoryIds, [oneShot.selectors.repository_id]);

  const localPackages = api.inspectLocalPackageDistribution(localSelection);
  assert.deepEqual(
    localPackages.packages.map(({ id, version }) => [id, version]),
    [["example.engine", "1.0.0"]],
  );

  writeFixture(local.root, path.join("input", "action.json"), action.tagged_input);
  const capturedAction = api.captureLocalNativeActionRequest({
    actionId: action.action_id,
    root: local.root,
    relativePath: "input/action.json",
  });
  assert.deepEqual(
    Buffer.from(api.invokeCapturedNativeAction(capturedAction)),
    Buffer.from(taggedExecution),
  );

  const localStatus = api.inspectLocalRepositoryStatus(
    localCompilation,
    oneShot.selectors.repository_id,
  );
  assert.equal(localStatus.repositoryId, oneShot.selectors.repository_id);
  assert.equal(localStatus.isClean, false);
  assert(localStatus.drift.length > 0);

  const localPlan = api.planLocalOneShot(localCompilation, {
    repositoryId: oneShot.selectors.repository_id,
    kind: bindings.OneShotPlanKind.Apply,
  });
  assert.equal(localPlan.repositoryId, oneShot.selectors.repository_id);
  assert(localPlan.counts().create > 0);
  const publication = api.publishLocalOneShotPlan(localPlan, local.plans);
  assert.equal(publication.repositoryId, oneShot.selectors.repository_id);
  const publishedPlan = fs.readFileSync(path.join(local.plans, publication.relativeTarget));
  assert(publishedPlan.includes(Buffer.from("\n  ")));
  assert(publishedPlan.subarray(-1).equals(Buffer.from("\n")));

  const admittedRun = api.admitLocalMaterializationRun(localCompilation, {
    kind: bindings.OneShotPlanKind.Apply,
    savedPlanDirectory: local.plans,
    evidenceDirectory: local.evidence,
  });
  assert.deepEqual(
    admittedRun.targets().map(({ repositoryId }) => repositoryId),
    [oneShot.selectors.repository_id],
  );
  assert.throws(
    () => api.inspectLocalMaterializationReconciliation(admittedRun),
    (error) => {
      assert.equal(error.diagnostic.code, "local_materialization_phase_rejected");
      assert.equal(
        error.diagnostic.effect,
        bindings.DiagnosticEffectCertainty.NotApplicable,
      );
      return true;
    },
  );

  const verifiedIdentity = api.verifyCurrentExecutableBinding(localCompilation);
  assert.deepEqual(
    Buffer.from(verifiedIdentity.canonicalBytes),
    Buffer.from(compiledIdentity.canonicalBytes),
  );
  const settled = api.settleLocalMaterializationRun(admittedRun);
  assert.equal(settled.outcome, bindings.LocalMaterializationRunOutcome.Succeeded);
  assert.equal(settled.succeededTargetCount, 1);
  assert(settled.targets[0].changedSurfaceCount > 0);
  assert(fs.readFileSync(path.join(local.evidence, "run-result.apply.json")).includes(Buffer.from("\n  ")));
  assert.equal(
    api.inspectLocalRepositoryStatus(localCompilation, oneShot.selectors.repository_id).isClean,
    true,
  );
} finally {
  fs.rmSync(local.root, { recursive: true, force: true });
}

const executed = [
  "workspace.portable-organization-inspection",
  sourceSubjectsCapability,
  subjectInspectionCapability,
  ...subjectComparisonCapabilities,
  ...authoringPageCapabilities,
  localImprovementCapability,
  fixture.capability_id,
  ...contract.capability_ids,
  "package-distribution.inspection",
  "authoring.ingress-limits",
  "authoring.standalone-validation",
  "authoring.standalone-inspection",
  portable.capability_ids[0],
  "workspace.portable-authoring-inspection",
  "workspace.portable-authoring-navigation",
  ...portable.capability_ids.slice(1),
  "workspace.effective-profile-compilation",
  "workspace.effective-profile-inspection",
  "workspace.effective-profile-explanation",
  "workspace.effective-profile-authoring-inspection",
  "workspace.effective-profile-authoring-navigation",
  "workspace.effective-profile-authoring-join",
  improvementCapability,
  "observation.snapshot",
  "observation.page",
  observationProfileCapability,
  "observation.delta",
  "observation.causal-path",
  "observation.visual-lens",
  ...action.capability_ids,
  repositoryAdmission.capability_id,
  "resident-hook.admission",
  ...oneShot.capability_ids,
  "one-shot.machine-effect-binding",
  "one-shot.saved-plan-admission",
  "local.one-shot-compilation",
  "local.package-distribution-inspection",
  "local.native-action-input-capture",
  "local.repository-status",
  "local.one-shot-planning",
  "local.one-shot-plan-publication",
  "local.materialization-run-admission",
  "local.materialization-run-settlement",
  "local.materialization-reconciliation-inspection",
  "local.compiled-interface-identity",
  "local.current-executable-binding-verification",
  "local.control-storage-inspection",
];
const witnessed = new Set(executed);
assert.deepEqual([...witnessed].filter((id) => !api.capabilityIds.includes(id)), []);
assert.deepEqual(api.capabilityIds.filter((id) => !witnessed.has(id)), []);
console.log(JSON.stringify({ executed, organizationResult,
  effectiveProfileEntryKinds: [...effectiveProfileEntryKinds],
  effectiveProfileParticipantKinds: [...effectiveProfileParticipantKinds],
  effectiveProfileResponsibilityKinds: [...effectiveProfileResponsibilityKinds],
}, null, 2));
