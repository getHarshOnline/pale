import assert from "node:assert/strict";
import fs from "node:fs";
import crypto from "node:crypto";
import { createRequire } from "node:module";
import path from "node:path";
import { verifyDiagnosticTransport } from "./diagnostic_transport.mjs";
import { verifyAuthoringOccurrencePages } from "./authoring_pages.mjs";
import { verifySubjectComparison } from "./subject_comparison.mjs";
import { verifyConsumedArguments } from "./consumed_arguments.mjs";

const [moduleArgument, fixtureArgument] = process.argv.slice(2);
const modulePath = path.resolve(moduleArgument);
const fixtureRoot = path.resolve(fixtureArgument);
const commonJsModulePath = modulePath.replace(/\.js$/, ".cjs");
fs.copyFileSync(modulePath, commonJsModulePath);
process.on("exit", () => fs.rmSync(commonJsModulePath, { force: true }));
const bindings = createRequire(import.meta.url)(commonJsModulePath);
const { Pale } = bindings;
assert.deepEqual(
  Object.keys(bindings).filter((name) => name.startsWith("Wasm")),
  [],
);
const fixture = JSON.parse(fs.readFileSync(`${fixtureRoot}/bundle-identity.json`, "utf8"));
const contract = JSON.parse(fs.readFileSync(`${fixtureRoot}/contract-diagnostic.json`, "utf8"));
const identity = new Pale().contractBundleIdentity();

assert.equal(identity.memberCount, fixture.member_count);
assert.equal(identity.schemaManifestSha256, fixture.schema_manifest_sha256);
assert.equal(identity.schemaSetSha256, fixture.schema_set_sha256);

const api = new Pale();
verifyAuthoringErrors();
verifyDiagnosticTransport(api);

function exactPath(value) {
  return value.path().map((segment) => {
    try {
      return segment.kind() === bindings.AuthoringInstancePathSegmentKind.Field
        ? { field: segment.fieldValue } : { index: segment.indexValue };
    } finally {
      segment.free();
    }
  });
}

function verifyAuthoringErrors() {
  const sourceRoot = new URL("../../../interfaces/lsp/tests/fixtures/authoring/visibility-widening/", import.meta.url);
  const labels = fs.readdirSync(sourceRoot).sort();
  const failure = (invalidVersion = false) => {
    const client = new Pale();
    try {
      client.compilePortableWorkspace(new bindings.PortableWorkspaceRequest(
        "pale-workspace.toml", labels.map((label) => {
          let text = fs.readFileSync(new URL(label, sourceRoot), "utf8");
          if (invalidVersion && label === "pale-workspace.toml") {
            text = text.replace("schema_version = 1", "schema_version = 2");
          }
          return new bindings.PortableWorkspaceDocument(label, new TextEncoder().encode(text));
        }),
      ));
    } catch (error) {
      return error;
    } finally {
      client.free();
    }
    assert.fail("the compiler must reject the fixture");
  };
  let error = failure();
  assert.ok(error instanceof Error);
  assert.equal(error.diagnostic.code, "workspace_semantics_rejected");
  assert.equal(error.authoringCorrections.length, 1);
  assert.ok(error.authoringLocation instanceof bindings.AuthoringSourceLocation);
  const correction = error.authoringCorrections[0];
  const location = error.authoringLocation;
  assert.ok(correction instanceof bindings.AuthoringCorrection);
  assert.equal(error.message, error.diagnostic.summary);
  assert.deepEqual(JSON.parse(error.diagnosticJson), error.diagnostic);
  assert.equal(Object.hasOwn(error.diagnostic, "authoringCorrections"), false);
  assert.equal(Object.hasOwn(error.diagnostic, "authoringLocation"), false);
  error = undefined;
  // These owned projections remain usable after the client and error are released.
  try {
    assert.equal(correction.kind(), bindings.AuthoringCorrectionKind.NarrowImportVisibility);
    assert.equal(correction.sourceLabel, "consumer-module.toml");
    const expectedPath = [{ field: "module" }, { field: "imports" }, { index: 0n }, { field: "visibility_ceiling" }];
    assert.deepEqual(exactPath(correction), expectedPath);
    assert.equal(correction.expectedSource, '"public"');
    assert.equal(correction.replacementSource, '"protected"');
    assert.match(correction.diagnosticIdentity, /^[a-f0-9]{64}$/);
    const subject = correction.proofSubject();
    try {
      assert.equal(subject.kind(), bindings.AuthoringIdentityKind.Export);
      assert.equal(subject.value, "candidate::public-api");
      assert.equal(subject.authoredValue, "public-api");
    } finally { subject.free(); }
    assert.equal(location.sourceLabel, correction.sourceLabel);
    assert.deepEqual(exactPath(location), expectedPath);
  } finally {
    correction.free();
    location.free();
  }
  const located = failure(true);
  assert.equal(located.diagnostic.code, "workspace_contract_rejected");
  assert.deepEqual(located.authoringCorrections, []);
  try {
    assert.equal(located.authoringLocation.sourceLabel, "pale-workspace.toml");
    assert.deepEqual(exactPath(located.authoringLocation), [{ field: "workspace" }, { field: "schema_version" }]);
  } finally { located.authoringLocation.free(); }
}

assert.throws(
  () => api.contractSchema("not-a-selected-schema"),
  (error) => {
    assert.ok(error instanceof Error);
    assert.equal(error.message, error.diagnostic.summary);
    assert.equal(error.diagnostic.code, "contract_rejected");
    assert.deepEqual(error.diagnostic, JSON.parse(error.diagnosticJson));
    assert.deepEqual(error.authoringCorrections, []);
    assert.ok(Object.hasOwn(error, "authoringLocation"));
    assert.equal(error.authoringLocation, undefined);
    return true;
  },
);
const schema = api.contractSchema(contract.schema.registry_id);
assert.equal(schema.registryId, contract.schema.registry_id);
assert.equal(schema.schemaUri, contract.schema.schema_uri);
assert.equal(schema.wireSchemaId, contract.schema.wire_schema_id);
assert.equal(schema.schemaSha256, contract.schema.schema_sha256);
assert.equal(schema.bytes().length, contract.schema.byte_length);

const causeProjection = new bindings.DiagnosticProjection(
  "wasm_numeric_admission_test",
  bindings.DiagnosticStage.InputAdmission,
  bindings.DiagnosticDisposition.InvalidInput,
  "the diagnostic projection retains admitted causes",
  bindings.DiagnosticRetry.AfterCorrection,
  bindings.DiagnosticEffectCertainty.NotApplicable,
);
causeProjection.setCauses(["retained cause"], 1);
for (const invalid of [-1, 0.5, Number.NaN, Number.POSITIVE_INFINITY, 2 ** 32]) {
  assert.throws(
    () => causeProjection.setCauses(["must not replace the retained cause"], invalid),
    /diagnostic omitted-cause count must be an integer from 0 through 4294967295/,
  );
}
const causeDiagnostic = JSON.parse(new TextDecoder().decode(api.projectDiagnostic(causeProjection)));
assert.deepEqual(causeDiagnostic.causes, ["retained cause"]);
assert.equal(causeDiagnostic.truncation.omitted_causes, 1);
const maximumOmittedProjection = new bindings.DiagnosticProjection(
  "wasm_numeric_maximum_test",
  bindings.DiagnosticStage.InputAdmission,
  bindings.DiagnosticDisposition.InvalidInput,
  "the diagnostic projection preserves the maximum admitted omission count",
  bindings.DiagnosticRetry.AfterCorrection,
  bindings.DiagnosticEffectCertainty.NotApplicable,
);
maximumOmittedProjection.setCauses([], 2 ** 32 - 1);
const maximumOmittedDiagnostic = JSON.parse(
  new TextDecoder().decode(api.projectDiagnostic(maximumOmittedProjection)),
);
assert.equal(maximumOmittedDiagnostic.truncation.omitted_causes, 4294967295);

const projection = new bindings.DiagnosticProjection(
  contract.projection.code,
  bindings.DiagnosticStage.InputAdmission,
  bindings.DiagnosticDisposition.InvalidInput,
  contract.projection.summary,
  bindings.DiagnosticRetry.AfterCorrection,
  bindings.DiagnosticEffectCertainty.NotApplicable,
);
const diagnostic = new TextDecoder().decode(api.projectDiagnostic(projection));
assert.equal(diagnostic, contract.canonical_diagnostic);
const outcomeUnknown = new bindings.DiagnosticProjection(
  contract.outcome_unknown.code,
  bindings.DiagnosticStage.Materialization,
  bindings.DiagnosticDisposition.EffectOutcomeUnknown,
  contract.outcome_unknown.summary,
  bindings.DiagnosticRetry.ReconcileFirst,
  bindings.DiagnosticEffectCertainty.OutcomeUnknown,
);
assert.equal(
  new TextDecoder().decode(api.projectDiagnostic(outcomeUnknown)),
  contract.outcome_unknown.canonical_diagnostic,
);
const recoveryProjection = new bindings.DiagnosticProjection(
  "resident_recovery_required",
  bindings.DiagnosticStage.ResidentRecovery,
  bindings.DiagnosticDisposition.Blocked,
  "resident recovery evidence must be reconciled",
  bindings.DiagnosticRetry.ReconcileFirst,
  bindings.DiagnosticEffectCertainty.NotApplicable,
);
const recoveryDiagnostic = JSON.parse(new TextDecoder().decode(api.projectDiagnostic(recoveryProjection)));
assert.equal(recoveryDiagnostic.stage, "resident_recovery");
assert.equal(recoveryDiagnostic.retry, "reconcile_first");
assert.equal(recoveryDiagnostic.effect, "not_applicable");
assert.equal(
  new TextDecoder().decode(api.admitDiagnostic(new TextEncoder().encode(contract.canonical_diagnostic))),
  diagnostic,
);
const validated = api.validateContractDocument(
  contract.schema.registry_id,
  new TextEncoder().encode(contract.canonical_diagnostic),
);
assert.equal(validated.schemaId, contract.schema.registry_id);
assert.equal(validated.schemaSha256, contract.schema.schema_sha256);
assert.equal(new TextDecoder().decode(validated.canonicalBytes), diagnostic);

const portable = JSON.parse(fs.readFileSync(`${fixtureRoot}/portable-journey.json`, "utf8"));
const makeRequest = () => new bindings.PortableWorkspaceRequest(
  portable.root_manifest,
  portable.document_labels.map((label) => new bindings.PortableWorkspaceDocument(
    label,
    fs.readFileSync(`${fixtureRoot}/portable/${label}`),
  )),
);
const consumedRequest = makeRequest();
const releasedCompilation = api.compilePortableWorkspace(consumedRequest);
assert.throws(() => api.compilePortableWorkspace(consumedRequest));
releasedCompilation.free();
assert.throws(() => releasedCompilation.workspaceId);
const prior = api.compilePortableWorkspace(makeRequest());
const current = api.compilePortableWorkspace(makeRequest());
const workspaceLimits = api.authoringIngressLimits(bindings.AuthoringIngressClass.Workspace);
assert.equal(typeof workspaceLimits.maxDocuments, "bigint");
assert.equal(
  workspaceLimits.maxDocuments,
  BigInt(portable.workspace_authoring_limits.max_documents),
);
assert.equal(
  workspaceLimits.maxDocumentBytes,
  BigInt(portable.workspace_authoring_limits.max_document_bytes),
);
assert.equal(
  workspaceLimits.maxTotalBytes,
  BigInt(portable.workspace_authoring_limits.max_total_bytes),
);
const field = (value) => bindings.AuthoringPathSegment.field(value);
const instanceField = (value) => bindings.AuthoringInstancePathSegment.field(value);
for (const invalid of [-1, 0.5, Number.NaN, Number.POSITIVE_INFINITY, 2 ** 32]) {
  assert.throws(
    () => bindings.AuthoringInstancePathSegment.index(invalid),
    /authoring instance-path index must be an integer from 0 through 4294967295/,
  );
}
const maximumInstanceIndex = bindings.AuthoringInstancePathSegment.index(2 ** 32 - 1);
try {
  assert.equal(maximumInstanceIndex.indexValue, 4294967295n);
} finally {
  maximumInstanceIndex.free();
}
api.validateAuthoringDocument(
  bindings.StandaloneAuthoringRole.WorkspaceManifest,
  portable.root_manifest,
  fs.readFileSync(`${fixtureRoot}/portable/${portable.root_manifest}`),
);
const standaloneAuthoring = api.inspectStandaloneAuthoring(
  bindings.StandaloneAuthoringRole.WorkspaceManifest,
  [field("workspace"), field("desired_topology")],
);
assert.equal(standaloneAuthoring.state(), bindings.AuthoringInspectionState.Available);
assert.equal(standaloneAuthoring.schemaId, "pale-workspace.v1");
assert.ok(standaloneAuthoring.node().fields().length > 0);
assert.throws(
  () => api.inspectStandaloneAuthoring(
    bindings.StandaloneAuthoringRole.WorkspaceManifest,
    Array.from({ length: 65 }, (_, index) => field(`over-bound-${index}`)),
  ),
  (error) => {
    assert.ok(error instanceof Error);
    assert.equal(error.message, error.diagnostic.summary);
    assert.equal(error.diagnostic.code, "authoring_path_too_deep");
    assert.equal(error.diagnostic.stage, "input_admission");
    assert.equal(error.diagnostic.disposition, "invalid_input");
    assert.equal(error.diagnostic.retry, "after_correction");
    assert.equal(error.diagnostic.effect, "not_applicable");
    assert.deepEqual(error.diagnostic, JSON.parse(error.diagnosticJson));
    return true;
  },
);
const workspaceAuthoring = prior.inspectAuthoring(
  portable.root_manifest,
  [field("workspace"), field("desired_topology")],
);
assert.equal(workspaceAuthoring.state(), bindings.AuthoringInspectionState.Available);
assert.equal(workspaceAuthoring.schemaId, standaloneAuthoring.schemaId);
const navigation = prior.navigateAuthoring(
  portable.root_manifest,
  [instanceField("workspace"), instanceField("workspace_id")],
);
assert.equal(navigation.identity().kind(), bindings.AuthoringIdentityKind.Workspace);
assert.equal(navigation.identity().value, portable.compilation.workspace_id);
assert.equal(navigation.definition().sourceLabel, portable.root_manifest);
assert.ok(navigation.references().length > 0);
assert.equal(navigation.referencesTruncated, false);
assert.equal(navigation.alternativesTruncated, false);
assert.deepEqual(
  {
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
  },
  portable.compilation,
);
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
for (const invalid of [-1, 0.5, Number.NaN, Number.POSITIVE_INFINITY, 2 ** 32]) {
  assert.throws(
    () => comparison.changeRoots(invalid, 1),
    /change-root page offset must be an integer from 0 through 4294967295/,
  );
  assert.throws(
    () => comparison.impacts(invalid, 1),
    /impact page offset must be an integer from 0 through 4294967295/,
  );
}
for (const invalid of [-1, 0.5, Number.NaN, Number.POSITIVE_INFINITY, 65536]) {
  assert.throws(
    () => comparison.changeRoots(0, invalid),
    /change-root page limit must be an integer from 0 through 65535/,
  );
  assert.throws(
    () => comparison.impacts(0, invalid),
    /impact page limit must be an integer from 0 through 65535/,
  );
}
const roots = comparison.changeRoots(0, 65535);
assert.equal(roots.nextOffset ?? null, portable.change_roots.next_offset);
const rootBytes = roots.canonicalBytes();
assert.deepEqual(JSON.parse(Buffer.from(rootBytes)), {
  entries: [],
  nextOffset: portable.change_roots.next_offset,
});
const repeatedRootPage = comparison.changeRoots(0, 65535);
assert.deepEqual(repeatedRootPage.canonicalBytes(), rootBytes);
repeatedRootPage.free();
const retainedRootBytes = roots.canonicalBytes();
rootBytes.fill(0);
assert.deepEqual(roots.canonicalBytes(), retainedRootBytes);
assert.equal(roots.intoEntries().length, portable.change_roots.entry_count);
const impactPage = comparison.impacts(0, 1);
assert.equal(impactPage.nextOffset, portable.first_impact.next_offset);
const impactBytes = impactPage.canonicalBytes();
assert.deepEqual(JSON.parse(Buffer.from(impactBytes)), {
  entries: [{
    subject: { kind: portable.first_impact.kind, sourceId: portable.first_impact.source_id },
    state: portable.first_impact.state,
    contributingRootCount: portable.first_impact.contributing_root_count,
  }],
  nextOffset: portable.first_impact.next_offset,
});
const retainedImpactBytes = impactPage.canonicalBytes();
impactBytes.fill(0);
assert.deepEqual(impactPage.canonicalBytes(), retainedImpactBytes);
const impact = impactPage.intoEntries()[0];
const subject = impact.subject();
assert.equal(subject.kind(), bindings.WorkspaceSubjectKind.Source);
assert.equal(subject.sourceId, portable.first_impact.source_id);
for (const field of ["workspaceId", "componentId", "moduleId", "ownerModuleId", "declarationNamespaceId", "declarationId", "importerModuleId", "upstreamModuleId", "upstreamReleaseId", "exportId", "sourceNamespaceId", "localNamespaceId", "extensionNamespaceId", "resourceId"]) assert.equal(subject[field], undefined);
assert.equal(impact.state(), bindings.WorkspaceImpactState.Unaffected);
assert.equal(impact.contributingRootCount, 0);
const explanation = comparison.explain(subject);
assert.equal(explanation.state(), bindings.WorkspaceExplanationState.Unaffected);
assert.equal(explanation.frontier(), undefined);
assert.equal(explanation.root(), undefined);
assert.deepEqual(explanation.steps(), []);

const action = JSON.parse(fs.readFileSync(`${fixtureRoot}/native-action-journey.json`, "utf8"));
const actionDescriptors = api.nativeActionCatalog();
assert.deepEqual(actionDescriptors.map((descriptor) => descriptor.actionId), action.descriptor_ids);
assert.deepEqual(actionDescriptors.map((descriptor) => { const contractBytes = descriptor.contractBytes(); const payloadBytes = descriptor.payloadSchemaBytes(); return [descriptor.actionId, descriptor.description, crypto.createHash("sha256").update(contractBytes).digest("hex"), contractBytes.length, crypto.createHash("sha256").update(payloadBytes).digest("hex"), payloadBytes.length]; }), action.descriptor_projections);
const taggedExecution = api.invokeNativeAction(action.action_id, new TextEncoder().encode(action.tagged_input));
const payloadExecution = api.invokeNativeActionPayload(action.action_id, new TextEncoder().encode(action.payload_input));
assert.deepEqual(Buffer.from(taggedExecution), Buffer.from(payloadExecution));
assert.equal(payloadExecution.length, action.execution_length);
assert.equal(crypto.createHash("sha256").update(payloadExecution).digest("hex"), action.execution_sha256);

const standaloneAuthoringCapabilityIds = [
  "authoring.ingress-limits",
  "authoring.standalone-validation",
  "authoring.standalone-inspection",
];
const workspaceAuthoringCapabilityIds = [
  "workspace.portable-authoring-inspection",
  "workspace.portable-authoring-navigation",
  verifyAuthoringOccurrencePages(bindings, fixtureRoot),
];
const [portableCompilationCapabilityId, ...portableDerivationCapabilityIds] = portable.capability_ids;
verifyConsumedArguments(bindings, {
  rootManifest: portable.root_manifest,
  documents: portable.document_labels.map(label => ({ label,
    bytes: fs.readFileSync(`${fixtureRoot}/portable/${label}`) })),
}, contract);
const executed = [
  verifySubjectComparison(bindings, fixtureRoot),
  fixture.capability_id,
  ...contract.capability_ids,
  ...standaloneAuthoringCapabilityIds,
  portableCompilationCapabilityId,
  ...workspaceAuthoringCapabilityIds,
  ...portableDerivationCapabilityIds,
  ...action.capability_ids,
];
const capabilityIdsJson = api.capabilityIdsJson();
const capabilityIds = JSON.parse(capabilityIdsJson);
assert.deepEqual([...executed].sort(), [...capabilityIds].sort());
assert.equal(capabilityIdsJson, JSON.stringify(capabilityIds, null, 2));
console.log(JSON.stringify({ executed }, null, 2));
