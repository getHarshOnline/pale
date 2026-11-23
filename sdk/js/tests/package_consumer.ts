import {
  Pale, ObservationFilter, ObservationSubjectKind, ObservationProfileAssociationResultKind,
  ObservationProfileResultKind,
  ObservationCancellation, type ObservationCursor, type ObservationFact,
  type ObservationPageLimits,
  type ObservationLens, type ObservationLensNode,
  ObservationProfileMapping, type ObservationProfilePage, type ObservationProfileContinuation,
  LocalControlStorageInspectionRequest, ConfigurationImprovementPhase,
  ConfigurationImprovementDisposition, EffectiveProfileSection,
  EffectiveProfileNativeRealization,
  type EffectiveProfileExplanation, type EffectiveProfileExplanationEvidence,
  AuthoringInstancePathSegment, AuthoringCorrection, AuthoringCorrectionKind,
  type AuthoringIdentity, type AuthoringLocationSegment,
  type AuthoringOccurrencePage, type AuthoringOccurrencePageCursor, type AuthoringOccurrenceRole,
  type EffectiveAuthoringPage, type EffectiveAuthoringContinuation,
  PortableWorkspaceSourceState, PortableWorkspaceSubjectMembership,
  type PortableWorkspaceSourceInspection, type PortableWorkspaceSubjectOccurrences,
  type PortableWorkspaceSubjectInspection, type PortableWorkspaceSubjectInspectionCursor,
  WorkspaceContributingRootsState, type WorkspaceContributingRootsPage, type WorkspaceContributingRootsCursor,
  type PortableWorkspaceDependentPage, type PortableWorkspaceDependentCursor,
  type PortableWorkspaceDependencyPage, type PortableWorkspaceDependencyCursor,
  type WorkspaceComparisonSubjectPage, type WorkspaceComparisonSubjectCursor,
  type WorkspaceChangeRoot, WorkspaceChangeRootPage, type WorkspaceComparisonSummary, WorkspaceImpactPage,
  type WorkspaceCausalStep,
  type AuthoringOccurrence, type AuthoringSourceLocation,
  WorkspaceValueComparison, type WorkspaceSubjectComparison, type WorkspaceSubjectValues,
  type EffectiveProfileEntry, type EffectiveProfileEntryDetail, type WorkspaceSubjectValue, type WorkspaceSubject,
  type EffectiveProfileParticipant, type EffectiveProfileResponsibility,
  type EffectiveProfileSelection, type EffectiveProfileDeclaredSelection, type EffectiveProfileDeclaredSelector,
  type RequirementDisposition, type EffectiveProfileProviderDisposition,
  type EffectiveProfileFinding, type EffectiveProfilePathReason,
  type PortableWorkspaceRequest, type ConfigurationImprovementCandidateRequest,
  type ConfigurationImprovementPreview, type ConfigurationImprovementExplanation,
  type OneShotCompilation,
  MAX_AUTHORING_OCCURRENCE_PAGE_COPIED_STRING_BYTES,
} from "@rtepl/pale";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { join } from "node:path";

// Membership narrows the complete payload; absent sides are not optional facts
// on every variant, and missing values cannot masquerade as equal outputs.
function comparedValue(value: WorkspaceSubjectValues): string {
  switch (value.membership) {
    case "absent": {
      // @ts-expect-error An absent subject has no current output.
      void value.current;
      return value.membership;
    }
    case "prior-only": {
      // @ts-expect-error One-sided presence has no equality decision.
      void value.comparison;
      return String(value.prior.kind);
    }
    case "current-only": {
      // @ts-expect-error The predecessor is not present in this variant.
      void value.prior;
      return String(value.current.kind);
    }
    case "shared": {
      const equality: WorkspaceValueComparison = value.comparison;
      return `${value.prior.kind}:${value.current.kind}:${equality}`;
    }
    default: { const exhaustive: never = value; return exhaustive; }
  }
}

// @ts-expect-error Shared membership requires both typed outputs and equality.
const incompleteSubjectValues: WorkspaceSubjectValues = { membership: "shared" };
// @ts-expect-error An absent subject cannot carry a value-comparison result.
const absentWithEquality: WorkspaceSubjectValues = { membership: "absent", comparison: WorkspaceValueComparison.Equal };
void incompleteSubjectValues; void absentWithEquality;

// TypeScript does not have typed throws. Narrow the Error and its members explicitly.
function correctionSources(error: unknown): string[] {
  if (!(error instanceof Error) || !("authoringCorrections" in error)
    || !Array.isArray(error.authoringCorrections)) return [];
  return error.authoringCorrections.map((value: unknown) => {
    if (!(value instanceof AuthoringCorrection)) throw new Error("not a native correction");
    const kind: AuthoringCorrectionKind = value.kind;
    const subject: AuthoringIdentity = value.proofSubject;
    const path: AuthoringLocationSegment[] = value.path;
    // @ts-expect-error The compiler did not produce a generic edit-authority flag.
    void value.writeAuthority;
    // @ts-expect-error Exact source text is not a numeric coordinate.
    const invalid: number = value.expectedSource;
    void invalid;
    return `${kind}:${value.sourceLabel}:${value.expectedSource}:${value.replacementSource}:${subject.value}:${path.length}:${value.diagnosticIdentity}`;
  });
}
void correctionSources(new Error("no compiler evidence"));

function noCorrectionConstructor(): void {
  // @ts-expect-error Only an SDK-produced native correction has this nominal identity.
  new AuthoringCorrection();
}
void noCorrectionConstructor;

// These reads compile only when each literal tag narrows to required fields.
function entryCoordinate(detail: EffectiveProfileEntryDetail): string {
  switch (detail.kind) {
    case "declared-selection": return declaredSelectionCoordinate(detail.selection);
    case "responsibility": return detail.contractId + subjectCoordinate(detail.subject)
      + participantCoordinate(detail.participant) + detail.responsibilities.map(responsibilityCoordinate).join(",");
    case "selection": return selectionCoordinate(detail.selection);
    case "capability-narrowing": return detail.permittedCapabilities.join(",");
    case "governing-ceiling": return detail.scope + detail.subjectResource;
    case "repository": {
      // @ts-expect-error A repository payload has no operation coordinate.
      void detail.operationId;
      return detail.repositoryId + detail.materializationOrder;
    }
    case "provider-repository": return detail.providerBindingId + detail.nativeRepositoryCoordinate;
    case "execution-path": {
      const nullable = detail.terminalProviderBinding;
      // @ts-expect-error The intrinsic null must not be erased by the union.
      const missingNull: NonNullable<typeof nullable> = nullable;
      void missingNull;
      return detail.selectionId + (nullable === null ? "" : nullable.providerBindingIdentity);
    }
    case "operation": return detail.repositoryId + detail.operationId;
    case "capability": return detail.capabilityId + detail.requirementId;
    case "machine-realization": return detail.machineId;
    case "finding": return String(detail.finding.kind);
    default: {
      const exhaustive: never = detail;
      return exhaustive;
    }
  }
}

function declaredSelectorCoordinate(value: EffectiveProfileDeclaredSelector): string {
  switch (value.kind) {
    case "exact": {
      // @ts-expect-error An exact selector has no Package wildcard coordinate.
      void value.packageId;
      return value.resourceId;
    }
    case "all-from-package": return value.packageId;
    default: { const exhaustive: never = value; return exhaustive; }
  }
}

function declaredSelectionCoordinate(value: EffectiveProfileDeclaredSelection): string {
  switch (value.kind) {
    case "resource": return value.resourceKind + declaredSelectorCoordinate(value.selector);
    case "mode-default": {
      // @ts-expect-error A mode default has no selector.
      void value.selector;
      return value.axis + value.resourceId;
    }
    default: { const exhaustive: never = value; return exhaustive; }
  }
}

// @ts-expect-error An exact selector requires its resource coordinate.
const incompleteDeclaredSelector: EffectiveProfileDeclaredSelector = { kind: "exact" };
// @ts-expect-error A resource declaration requires its typed selector.
const incompleteDeclaredSelection: EffectiveProfileDeclaredSelection = { kind: "resource", resourceKind: 0 };
void incompleteDeclaredSelector; void incompleteDeclaredSelection;

function selectionCoordinate(value: EffectiveProfileSelection): string {
  switch (value.kind) {
    case "package": return value.packageId + value.version + value.contentDigest;
    case "normative-pack": return value.packId + value.attestorId + value.version + value.contentDigest;
    case "semantic-resource": return value.repositoryId + value.resourceKind + value.resourceId;
    case "run-policy": {
      // @ts-expect-error Another selection's field is not an optional property.
      void value.packageId;
      return value.runPolicyId;
    }
    case "session": {
      const count: number = value.activationRootCount;
      return value.executionWorkspaceId + count + value.additionalRootCount + value.activationPathCount;
    }
    case "lifecycle": return value.lifecycleId + value.eventId + value.actionId;
    case "authority-root": return value.authorityRootId;
    case "disclosure-ceiling": return value.authorityCoordinate;
    default: {
      const exhaustive: never = value;
      return exhaustive;
    }
  }
}

function explainedSelection(value: EffectiveProfileExplanation): string {
  const proof = value.evidence;
  switch (proof.kind) {
    case "declared-selection": return declaredSelectionCoordinate(proof.selection);
    case "responsibility": return proof.contractId + proof.executionPathId + proof.reasonSteps.map(pathReasonCoordinate).join(",");
    case "effective-selection": return selectionCoordinate(proof.selection);
    case "execution-path": return proof.selectionId + proof.reasonSteps.map(pathReasonCoordinate).join(",");
    case "operation-classification": return String(proof.exposure) + proof.nativeRealization;
    case "capability-classification": return requirementCoordinate(proof.requirementDisposition) + providerCoordinate(proof.providerDisposition) + proof.nativeRealization;
    case "finding-classification": return findingCoordinate(proof.finding);
    case "machine-realization": return value.handle + proof.realization;
    case "configuration": {
      // @ts-expect-error Configuration has no optional selection field.
      void proof.selection;
      return value.handle;
    }
    default: { const exhaustive: never = proof; return exhaustive; }
  }
}

function requirementCoordinate(value: RequirementDisposition): string {
  switch (value.kind) {
    case "satisfied": case "provider-binding-absent": return value.kind;
    case "narrowed": case "blocked": return String(value.reason);
    default: { const exhaustive: never = value; return exhaustive; }
  }
}
function providerCoordinate(value: EffectiveProfileProviderDisposition): string {
  switch (value.kind) {
    case "bound": return String(value.provider);
    case "missing-binding": case "excluded-by-narrowing": case "effect-mismatch": case "recipe-contract-mismatch": {
      // @ts-expect-error Only a bound provider has the required provider field.
      void value.provider;
      return value.kind;
    }
    default: { const exhaustive: never = value; return exhaustive; }
  }
}
function findingCoordinate(value: EffectiveProfileFinding): string {
  switch (value.kind) {
    case "native-realization-unknown": return value.machineId;
    case "operation-unbound": return value.repositoryId + value.operationId;
    case "requirement": return value.repositoryId + value.operationId + value.requirementId + value.capabilityId + requirementCoordinate(value.disposition);
    default: { const exhaustive: never = value; return exhaustive; }
  }
}
function pathReasonCoordinate(value: EffectiveProfilePathReason): string {
  switch (value.kind) {
    case "origin-receipt": case "entry-contract": case "responsibility-contract": case "termination-contract": case "settlement-contract": case "continuity-contract": return value.contractId;
    case "provider-binding": return value.bindingId;
    case "delegation-edge": {
      // @ts-expect-error An edge reason does not have a contract coordinate.
      void value.contractId;
      return value.edgeId;
    }
    default: { const exhaustive: never = value; return exhaustive; }
  }
}
// @ts-expect-error Narrowing requires its exact reason.
const missingRequirementReason: RequirementDisposition = { kind: "narrowed" };
// @ts-expect-error A bound disposition requires its provider family.
const missingProvider: EffectiveProfileProviderDisposition = { kind: "bound" };
// @ts-expect-error Native-unknown findings require the machine coordinate.
const missingFinding: EffectiveProfileFinding = { kind: "native-realization-unknown" };
// @ts-expect-error An origin reason requires its contract coordinate.
const missingPathContract: EffectiveProfilePathReason = { kind: "origin-receipt" };
// @ts-expect-error Responsibility evidence requires the complete proof payload.
const missingEvidence: EffectiveProfileExplanationEvidence = { kind: "responsibility" };
void missingRequirementReason; void missingProvider; void missingFinding; void missingPathContract; void missingEvidence;

// @ts-expect-error Required release fields cannot be omitted.
const incompleteSelection: EffectiveProfileSelection = { kind: "package", packageId: "package" };
void incompleteSelection;

function subjectCoordinate(subject: WorkspaceSubjectValue): string {
  switch (subject.kind) {
    case "source": return subject.sourceId;
    case "component": return subject.componentId;
    case "module": return subject.moduleId;
    case "declaration":
    case "policy": return subject.ownerModuleId + subject.namespaceId + subject.declarationId;
    case "import": return subject.importerModuleId + subject.upstreamModuleId + subject.upstreamReleaseId
      + subject.exportId + subject.sourceNamespaceId + subject.localNamespaceId;
    case "extension": return subject.ownerModuleId + subject.extensionNamespaceId
      + subject.targetNamespaceId + subject.targetDeclarationId;
    case "resource": return subject.ownerModuleId + subject.resourceId;
    case "workspace": return subject.workspaceId;
    case "responsibility-closure":
    case "organizational-translation-closure":
    case "experience-composition":
    case "resolution-lock":
    case "universe-lock": {
      // @ts-expect-error A closure/lock selector has no fabricated workspace ID.
      void subject.workspaceId;
      return subject.kind;
    }
    default: {
      const exhaustive: never = subject;
      return exhaustive;
    }
  }
}

function participantCoordinate(value: EffectiveProfileParticipant): string {
  switch (value.kind) {
    case "interface": {
      // @ts-expect-error Other participant identities are not nullable axes.
      void value.applicationId;
      return value.interfaceId;
    }
    case "application": return value.applicationId;
    case "engine": return value.engineId;
    case "capability": return value.capabilityId;
    case "terminal-leaf": return value.terminalLeafId;
    case "plugin": return value.pluginId;
    case "skill": return value.skillId;
    default: {
      const exhaustive: never = value;
      return exhaustive;
    }
  }
}

function responsibilityCoordinate(value: EffectiveProfileResponsibility): string {
  switch (value.kind) {
    case "extension": return value.namespaceId + value.contractId + value.schemaId + value.schemaSha256;
    case "communication":
    case "organizational-work":
    case "crm-business-state":
    case "engineering-execution":
    case "context-memory":
    case "terminal-harness-execution":
    case "native-event-publication":
    case "native-effect-read-back": {
      // @ts-expect-error Built-in responsibilities have no extension schema pin.
      void value.schemaSha256;
      return value.kind;
    }
    default: {
      const exhaustive: never = value;
      return exhaustive;
    }
  }
}

// @ts-expect-error Required payload fields cannot be omitted.
const incompleteEntry: EffectiveProfileEntryDetail = { kind: "repository" };
// @ts-expect-error Compound selector coordinates remain required.
const incompleteSubject: WorkspaceSubjectValue = { kind: "import", importerModuleId: "module" };
void incompleteEntry; void incompleteSubject;
// @ts-expect-error A participant requires its own exact identity.
const incompleteParticipant: EffectiveProfileParticipant = { kind: "interface" };
// @ts-expect-error Extension schema identity cannot be left out.
const incompleteResponsibility: EffectiveProfileResponsibility = { kind: "extension", namespaceId: "namespace", contractId: "contract", schemaId: "schema" };
void incompleteParticipant; void incompleteResponsibility;

const api = new Pale();
const localPreview: Promise<ConfigurationImprovementPreview> = api.previewLocalConfigurationImprovement({
  baselineWorkspaceManifest: "relative-baseline", candidateWorkspaceManifest: "relative-candidate",
  projectId: "engine", executionWorkspaceId: "default", machineId: "machine",
});
let localPreviewRejected = false;
try {
  await localPreview;
} catch (error: unknown) {
  localPreviewRejected = error instanceof Error && error.message.length > 0;
}
if (!localPreviewRejected) throw new Error("packed local preview must reject invalid locators");
const schema = api.contractSchema("diagnostic.v1");
// The generated boundary must carry Node's real Buffer type, not any or bytes
// reconstructed by a second TypeScript owner.
const firstByte: number = schema.bytes.readUInt8(0);
// @ts-expect-error A native Buffer is not a string.
const notAString: string = schema.bytes;
void notAString;
if (firstByte !== 123) throw new Error("native schema bytes must start with an object");

const request = LocalControlStorageInspectionRequest.principal(
  "relative-root", "principal-inspection", "store-inspection", "incarnation-inspection",
);
let rejected = false;
try {
  api.inspectLocalControlStore(request);
} catch (error: unknown) {
  rejected = error instanceof Error && error.message.length > 0;
}
if (!rejected) throw new Error("invalid root must remain rejected through the packed addon");

const fixtures = process.argv[2];
if (!fixtures) throw new Error("the API-owned fixture root is required");
const oneShot = JSON.parse(readFileSync(join(fixtures, "one-shot-journey.json"), "utf8"));
const vector = JSON.parse(readFileSync(join(fixtures, "improvement-journey.json"), "utf8"));
const source = join(fixtures, "one-shot");
const workspace: PortableWorkspaceRequest = {
  rootManifest: "pale-workspace.toml",
  documents: ["composition.toml", "pale-workspace.toml", "workspace-resolution.lock.toml", "workspace-universe.lock.toml"].map((label) => ({
    label, bytes: readFileSync(join(source, "workspace", label)),
  })),
};
const document = (label: string, name: string) => ({
  label, bytes: readFileSync(join(source, "private", name)),
});
const captured: ConfigurationImprovementCandidateRequest = {
  projectId: oneShot.selectors.project_id,
  executionWorkspaceId: oneShot.selectors.execution_workspace_id,
  machineId: oneShot.selectors.machine_id,
  privateInputs: {
    capabilities: document("capabilities.toml", "capabilities.toml"),
    machineBinding: document("machines/machine.toml", "machine-binding.toml"),
    distributionLock: document("distribution-lock.toml", "distribution-lock.toml"),
    lifecycleGuidance: [],
  },
};
const oneShotPromise: Promise<OneShotCompilation> = api.compileOneShot({
  ...captured, workspace,
  packageDocuments: ["package.toml", "role.md", "roles.toml"].map((name) => ({
    label: `engine/${name}`, bytes: readFileSync(join(source, "packages/engine", name)),
  })),
});
if (!(oneShotPromise instanceof Promise)) throw new Error("one-shot compilation must return a Promise");
const oneShotCompilation = await oneShotPromise;
// @ts-expect-error The resolved opaque handle does not itself implement Promise.
const missingPromise: Promise<OneShotCompilation> = oneShotCompilation;
void missingPromise;
if (api.oneShotSummary(oneShotCompilation).oneShotCompilationDigest !== oneShot.summary.one_shot_compilation_digest) {
  throw new Error("asynchronous compilation identity drift");
}
const baseline = api.compilePortableWorkspace(workspace);
const equalPair = api.comparePortableWorkspaces(baseline, baseline);
const dependencySubject = equalPair.impacts(0, 256).entries
  .map(entry => entry.subject())
  .find(subject => baseline.directDependencies(subject, 256).dependencies().length > 0);
if (dependencySubject === undefined) throw new Error("compiled workspace has no direct dependency to inspect");
const accountedInputs: PortableWorkspaceDependencyPage = baseline.directDependencies(dependencySubject, 256);
const accountedInputRows: WorkspaceCausalStep[] = accountedInputs.dependencies();
const accountedInputBytes: Buffer = accountedInputs.canonicalBytes;
const accountedInputWire = JSON.parse(accountedInputBytes.toString());
if (accountedInputRows.length === 0 || accountedInputs.copiedStringBytes <= 0
  || accountedInputs.copiedStringBytes !== accountedInputWire.copiedStringBytes) {
  throw new Error("direct dependency byte accounting lost its SDK-owned value");
}
const repeatedAccountedInputs: PortableWorkspaceDependencyPage = baseline.directDependencies(dependencySubject, 256);
if (repeatedAccountedInputs.copiedStringBytes !== accountedInputs.copiedStringBytes
  || !repeatedAccountedInputs.canonicalBytes.equals(accountedInputBytes)) {
  throw new Error("direct dependency accounting was not stable across the packed boundary");
}
const impactPage: WorkspaceImpactPage = equalPair.impacts(0, 1);
const impactBytes: Buffer = impactPage.canonicalBytes;
const impactNextOffset: number | undefined = impactPage.nextOffset;
const repeatedImpactPage: WorkspaceImpactPage = equalPair.impacts(0, 1);
if (!repeatedImpactPage.canonicalBytes.equals(impactBytes)
  || createHash("sha256").update(repeatedImpactPage.canonicalBytes).digest("hex")
    !== createHash("sha256").update(impactBytes).digest("hex")) {
  throw new Error("impact page lost exact canonical bytes across the packed boundary");
}
const impactWire = JSON.parse(impactBytes.toString());
if (impactWire.nextOffset !== impactNextOffset) throw new Error("impact continuation coordinate drift");
const mutableImpactBytes = impactPage.canonicalBytes;
mutableImpactBytes.fill(0);
if (!impactPage.canonicalBytes.equals(impactBytes)) throw new Error("impact bytes leaked mutable native state");
const detachedImpactEntries = impactPage.entries;
detachedImpactEntries.length = 0;
if (impactPage.entries.length !== 1) throw new Error("impact entries leaked mutable native state");
let sparseImpactRejected = false;
try {
  equalPair.impacts({ offset: 0 } as unknown as number, 1);
} catch (error: unknown) {
  sparseImpactRejected = error instanceof Error && "code" in error && error.code === "NumberExpected";
}
if (!sparseImpactRejected) throw new Error("sparse impact coordinates crossed the packed ABI");
function noImpactPageConstructor(): void {
  // @ts-expect-error Impact pages are compiler-produced output handles.
  new WorkspaceImpactPage();
}
void noImpactPageConstructor;
const firstImpact = impactPage.entries[0];
if (firstImpact === undefined) throw new Error("compiled comparison has no subjects");
if (impactWire.entries[0]?.subject?.sourceId !== firstImpact.subject().sourceId) {
  throw new Error("impact typed row and sealed bytes diverged");
}
const subjectComparison: WorkspaceSubjectComparison = equalPair.compareSubject(firstImpact.subject());
const comparisonSubject: WorkspaceSubject = subjectComparison.subject();
const comparisonBytes: Buffer = subjectComparison.canonicalBytes;
if (subjectComparison.receiptDigest !== equalPair.summary().receiptDigest
  || subjectComparison.values.membership !== "shared"
  || subjectComparison.values.comparison !== WorkspaceValueComparison.Equal
  || JSON.parse(comparisonBytes.toString()).receiptDigest !== subjectComparison.receiptDigest) {
  throw new Error("subject result lost its parent receipt or typed equality");
}
comparedValue(subjectComparison.values);
equalPair.compareSubject(comparisonSubject);
const profile = api.compileEffectiveProfile(baseline, {
  ...captured,
  packageDocuments: ["package.toml", "role.md", "roles.toml"].map((name) => ({
    label: `engine/${name}`, bytes: readFileSync(join(source, "packages/engine", name)),
  })),
});
const observed = api.observeEffectiveProfile(profile);
const profileAssociation = observed.associateProfile(profile, baseline);
if (profileAssociation.kind !== ObservationProfileAssociationResultKind.Associated
  || profileAssociation.association === null
  || !profileAssociation.association.belongsTo(observed)
  || profileAssociation.association.coordinates.workspaceId !== profile.workspaceId) {
  throw new Error("exact profile association was not retained");
}
const lensResult = observed.visualLens(
  new ObservationFilter(),
  {
    maximumInspected: 4,
    maximumNodes: 4,
    maximumCopiedStringBytes: 0xffffffff,
    maximumCausalPathLookups: 4,
    maximumCausalSteps: 256,
  },
  false,
  ObservationCancellation.Active,
);
if (lensResult.lens === null) throw new Error("active lens request was cancelled");
const lens: ObservationLens = lensResult.lens;
const lensCursor: ObservationCursor = lens.cursor;
const lensNodes: ObservationLensNode[] = lens.nodes;
const lensFacts: ObservationFact[] = lensNodes.flatMap((node) => node.facts);
const lensWork: number[] = [
  lens.inspectedSubjects, lens.copiedStringBytes, lens.causalPathLookups,
  lens.causalSteps, lens.snapshotOmittedCount,
];
if (lensFacts.length === 0) throw new Error("lens lost retained facts");
void lensCursor.revision;
void lensWork;
const observedMachine = observed.page(
  new ObservationFilter(),
  { maximumInspected: 256, maximumReturned: 256, maximumCopiedStringBytes: 0xffffffff } satisfies ObservationPageLimits,
  ObservationCancellation.Active,
).page?.facts
  .map((fact) => fact.subject).find((subject) => subject?.kind === ObservationSubjectKind.Machine);
if (observedMachine === undefined || observedMachine === null) throw new Error("configured machine is missing");
const association = observed.inspectProfile(profile, baseline, observedMachine, 1, 1);
if (association.kind !== ObservationProfileResultKind.Ready || association.page === null) {
  throw new Error("retained profile association refused");
}
const associatedPage: ObservationProfilePage = association.page;
const associationToken: ObservationProfileContinuation | null = associatedPage.continuation;
if (associatedPage.mapping !== ObservationProfileMapping.PrimaryRows) throw new Error("machine mapping missing");
if (associationToken !== null) {
  observed.inspectProfile(profile, baseline, observedMachine, 1, 1, associationToken);
}
const inspection = api.inspectConfigurationImprovement(baseline);
const explanation: ConfigurationImprovementExplanation = api.explainConfigurationImprovement(inspection, profile);
if (explanation.phase !== ConfigurationImprovementPhase.Explained) throw new Error("wrong phase");
const candidate = api.compilePortableWorkspace({
  ...workspace,
  documents: workspace.documents.map(({ label, bytes }) => ({
    label, bytes: Buffer.from(bytes.toString().replace('access = "mutate"', `access = "${vector.candidate_repository_access}"`)),
  })),
});
const prepared = api.prepareConfigurationImprovementPreview(explanation, candidate, {
  ...captured,
  privateInputs: {
    ...captured.privateInputs,
    machineBinding: {
      ...captured.privateInputs.machineBinding,
      bytes: Buffer.from(captured.privateInputs.machineBinding.bytes.toString().replace("/captured/repository", vector.candidate_machine_root)),
    },
  },
});
const preview: ConfigurationImprovementPreview = api.commitConfigurationImprovementPreview(explanation, prepared);
const summary = preview.summary();
if (summary.candidateInstanceDigest !== vector.summary.candidate_instance_digest) throw new Error("candidate identity drift");
if (preview.comparisonSummary().affectedCount !== vector.comparison.affected_count) throw new Error("comparison drift");
const changeRoots: WorkspaceChangeRootPage = preview.changeRoots(0, 1);
const changeRootBytes: Buffer = changeRoots.canonicalBytes;
const changeRootNextOffset: number | undefined = changeRoots.nextOffset;
if (changeRoots.entries.length === 0 || JSON.parse(changeRootBytes.toString()).entries.length === 0) {
  throw new Error("change-root page lost its entries or canonical bytes");
}
changeRoots.canonicalBytes.fill(0);
if (!changeRoots.canonicalBytes.equals(changeRootBytes)) {
  throw new Error("change-root bytes leaked mutable native state");
}
const detachedChangeRoots = changeRoots.entries;
detachedChangeRoots.length = 0;
if (changeRoots.entries.length === 0 || changeRoots.nextOffset !== changeRootNextOffset
  || !changeRoots.canonicalBytes.equals(changeRootBytes)) {
  throw new Error("change-root entries leaked mutable page state");
}
function noChangeRootPageConstructor(): void {
  // @ts-expect-error Change-root pages are compiler-produced output handles.
  new WorkspaceChangeRootPage();
}
void noChangeRootPageConstructor;
const page = preview.candidateProfile().inspect(EffectiveProfileSection.IntendedUseCase, 0, 1);
if (page.entryCount !== 1) throw new Error("retained profile query failed");
const machinePage = preview.candidateProfile().inspect(EffectiveProfileSection.MachineRealization, 0, 1);
const machineEntry: EffectiveProfileEntry | undefined = machinePage.entries[0];
if (machineEntry === undefined) throw new Error("machine page is empty");
const machineDetail = machineEntry.detail;
entryCoordinate(machineDetail);
if (machineDetail.kind !== "machine-realization" || machineDetail.realization !== EffectiveProfileNativeRealization.Unknown) throw new Error("typed machine row drift");
const requiredMachineId: string = machineDetail.machineId;
// @ts-expect-error Another variant's field does not exist on this detail.
void machineDetail.repositoryId;
// @ts-expect-error The entry has one detail discriminant, not an outer alias.
void machineEntry.kind;
void requiredMachineId;
const machineHandle: string = machineEntry.explanationHandle;
if (machineHandle !== JSON.parse(machinePage.canonicalBytes.toString()).entries[0].explanationHandle) throw new Error("canonical row correlation failed");
const machineExplanation: EffectiveProfileExplanation | null = preview.candidateProfile().explain(machineHandle);
if (machineExplanation === null || machineExplanation.handle !== machineHandle) throw new Error("retained explanation correlation failed");
const evidence: EffectiveProfileExplanationEvidence = machineExplanation.evidence;
if (!explainedSelection(machineExplanation)) throw new Error("empty explanation coordinate");
if (evidence.kind !== "machine-realization" || evidence.realization !== EffectiveProfileNativeRealization.Unknown) throw new Error("native realization evidence drift");
// @ts-expect-error Machine evidence has no reason-list axis.
void evidence.reasonSteps;
// @ts-expect-error The kind belongs to the complete evidence, not a parallel alias.
void machineExplanation.evidenceKind;
const profileIdentity = preview.candidateProfile().navigateAuthoring("pale-workspace.toml", [
  AuthoringInstancePathSegment.field("workspace"), AuthoringInstancePathSegment.field("desired_topology"),
  AuthoringInstancePathSegment.field("projects"), AuthoringInstancePathSegment.index(0),
  AuthoringInstancePathSegment.field("run_policy"),
]).identity();
if (profileIdentity === null) throw new Error("profile identity failed to resolve");
const joined = preview.candidateProfile().inspectEffectiveAuthoring(profileIdentity);
if (!joined.entries.some((row) => {
  const detail = row.detail;
  return detail.kind === "selection" && detail.selection.kind === "run-policy" && detail.selection.runPolicyId === "standard";
})) throw new Error("typed authoring join lost run policy");
// @ts-expect-error Complete entries replace the redundant handles-only projection.
void joined.explanationHandles;
const retainedProfile = preview.candidateProfile();
const occurrencePage: AuthoringOccurrencePage = retainedProfile.authoringOccurrences(profileIdentity, 1, 1);
const occurrenceCursor: AuthoringOccurrencePageCursor | null = occurrencePage.continuation();
const complete: boolean = occurrencePage.isComplete;
const inspected: number = occurrencePage.inspected;
const occurrenceCopiedBytes: number = occurrencePage.copiedStringBytes;
if (!Number.isSafeInteger(MAX_AUTHORING_OCCURRENCE_PAGE_COPIED_STRING_BYTES)
  || occurrenceCopiedBytes > MAX_AUTHORING_OCCURRENCE_PAGE_COPIED_STRING_BYTES) {
  throw new Error("occurrence copied-string accounting is outside its native ceiling");
}
for (const occurrence of occurrencePage.occurrences()) {
  const role: AuthoringOccurrenceRole = occurrence.role;
  const source: string = occurrence.location.sourceLabel;
  const sourcePath: AuthoringLocationSegment[] = occurrence.location.path;
  void [role, source, sourcePath];
}
if (occurrenceCursor !== null) retainedProfile.authoringOccurrences(profileIdentity, 1, 1, occurrenceCursor);
const associationPage: EffectiveAuthoringPage = retainedProfile.inspectEffectiveAuthoringPage(profileIdentity, 1);
const associationCursor: EffectiveAuthoringContinuation | null = associationPage.continuation();
if (associationCursor !== null) retainedProfile.inspectEffectiveAuthoringPage(profileIdentity, 1, associationCursor);
const privateInstance: string = associationPage.coordinates.instanceDigest;
const associatedRows: EffectiveProfileEntry[] = associationPage.entries;
const portableIdentity = api.navigatePortableWorkspaceAuthoring(baseline, "pale-workspace.toml", []).identity();
if (portableIdentity === null) throw new Error("portable source identity failed to resolve");
const portableOccurrences: AuthoringOccurrencePage = api.portableWorkspaceAuthoringOccurrences(baseline, portableIdentity, 1, 1);
const sourceInspection: PortableWorkspaceSourceInspection = api.portableWorkspaceSourceSubject(baseline, "pale-workspace.toml", []);
const sourceLocation: AuthoringSourceLocation = sourceInspection.location;
const sourceOccurrence: AuthoringOccurrence | null = sourceInspection.occurrence();
const sourceSubject: WorkspaceSubject | null = sourceInspection.subject();
if (sourceInspection.state !== PortableWorkspaceSourceState.Selected || sourceSubject === null
  || sourceOccurrence === null || !sourceInspection.belongsTo(baseline)) throw new Error("source inspection lost custody or meaning");
const sourcePage: PortableWorkspaceSubjectOccurrences = api.portableWorkspaceSubjectOccurrences(baseline, sourceSubject, 1, 1);
const sourceMatches: AuthoringOccurrencePage = sourcePage.page();
if (sourcePage.membership !== PortableWorkspaceSubjectMembership.Selected) throw new Error("source membership was erased");
const sourceCursor: AuthoringOccurrencePageCursor | null = sourceMatches.continuation();
if (sourceCursor !== null) api.portableWorkspaceSubjectOccurrences(baseline, sourceSubject, 1, 1, sourceCursor);
const subjectInspection: PortableWorkspaceSubjectInspection = baseline.inspectSubject(sourceSubject, 1, 1, 1);
const inspectionCursor: PortableWorkspaceSubjectInspectionCursor | null = subjectInspection.continuation();
const inspectionRoot: WorkspaceCausalStep | null = subjectInspection.root();
const inspectionInputs: WorkspaceCausalStep[] = subjectInspection.dependencies();
const inspectionOccurrences: AuthoringOccurrence[] = subjectInspection.occurrences();
const inspectionAnchor: AuthoringSourceLocation | null = subjectInspection.recordAnchor;
const inspectionBytes: Buffer = subjectInspection.canonicalBytes;
const inspectionInputsComplete: boolean = subjectInspection.inputsComplete;
const inspectionSourcesComplete: boolean = subjectInspection.sourcesComplete;
const inspectionComplete: boolean = subjectInspection.isComplete;
const inspectionInspected: number = subjectInspection.inspected;
const inspectionDependencyBytes: number = subjectInspection.dependencyCopiedStringBytes;
const inspectionSourceBytes: number = subjectInspection.sourceCopiedStringBytes;
const inspectionHeaders: string[] = [subjectInspection.workspaceId, subjectInspection.principalUniverseDigest,
  subjectInspection.semanticIrDigest, subjectInspection.admissionEvidenceDigest];
const inspectionSubject: WorkspaceSubject = subjectInspection.subject();
if (!subjectInspection.belongsTo(baseline)
  || inspectionDependencyBytes < 0 || inspectionSourceBytes < 0) {
  throw new Error("composed inspection lost custody or copied-string accounting");
}
if (inspectionCursor !== null) baseline.inspectSubject(inspectionSubject, 1, 1, 1, inspectionCursor);
if (inspectionRoot !== null) { inspectionRoot.subject(); inspectionRoot.fact(); inspectionRoot.provenance(); }
const candidateInspection: PortableWorkspaceSubjectInspection = preview.inspectCandidateSubject(sourceSubject, 1, 1, 1);
const candidateInspectionCursor: PortableWorkspaceSubjectInspectionCursor | null = candidateInspection.continuation();
if (candidateInspection.subject().kind !== sourceSubject.kind || candidateInspection.belongsTo(baseline)) {
  throw new Error("review candidate inspection lost its candidate owner or subject");
}
if (candidateInspectionCursor !== null) {
  preview.inspectCandidateSubject(candidateInspection.subject(), 1, 1, 1, candidateInspectionCursor);
}
if (inspectionCursor !== null) {
  let baselineCursorRejected = false;
  try {
    preview.inspectCandidateSubject(sourceSubject, 1, 1, 1, inspectionCursor);
  } catch {
    baselineCursorRejected = true;
  }
  if (!baselineCursorRejected) throw new Error("review candidate inspection accepted a baseline cursor");
}
const contributing: WorkspaceContributingRootsPage = equalPair.contributingRoots(sourceSubject, 1);
const contributionState: WorkspaceContributingRootsState = contributing.state;
const contributionSummary: WorkspaceComparisonSummary = contributing.comparison();
const contributionRoots: WorkspaceChangeRoot[] = contributing.roots();
const contributionCursor: WorkspaceContributingRootsCursor | null = contributing.continuation();
const contributionBytes: Buffer = contributing.canonicalBytes;
if (contributionState !== WorkspaceContributingRootsState.Unaffected || !contributing.isComplete
  || !contributing.belongsTo(equalPair) || contributionRoots.length !== 0
  || contributionSummary.receiptDigest !== equalPair.receiptDigest) throw new Error("contribution page lost membership or custody");
if (contributionCursor !== null) equalPair.contributingRoots(contributing.subject(), 1, contributionCursor);
const dependents: PortableWorkspaceDependentPage = baseline.directDependents(sourceSubject, 1, 1);
const dependentRoot: WorkspaceCausalStep | null = dependents.root();
const dependentRows: WorkspaceCausalStep[] = dependents.dependents();
const dependentCursor: PortableWorkspaceDependentCursor | null = dependents.continuation();
const dependentCounts: number[] = [dependents.inspected, dependents.inputDigestComparisons];
const dependentBytes: Buffer = dependents.canonicalBytes;
const dependentComplete: boolean = dependents.isComplete;
const dependentHeaders: string[] = [dependents.workspaceId, dependents.principalUniverseDigest,
  dependents.semanticIrDigest, dependents.admissionEvidenceDigest];
if (!dependents.belongsTo(baseline) || dependentRoot === null) throw new Error("dependent page lost selected root custody");
dependentRoot.fact(); dependentRows.map(step => [step.rule, step.provenance(), step.fact(), step.subject()]);
if (dependentCursor !== null) baseline.directDependents(dependents.subject(), 1, 1, dependentCursor);
const directInputs: PortableWorkspaceDependencyPage = baseline.directDependencies(sourceSubject, 1);
const directInputRoot: WorkspaceCausalStep | null = directInputs.root();
const directInputRows: WorkspaceCausalStep[] = directInputs.dependencies();
const directInputCursor: PortableWorkspaceDependencyCursor | null = directInputs.continuation();
const directInputBytes: Buffer = directInputs.canonicalBytes;
const directInputCopiedBytes: number = directInputs.copiedStringBytes;
const directInputCoordinates: string[] = [directInputs.principalUniverseDigest, directInputs.semanticIrDigest];
if (directInputRoot === null || !directInputs.isComplete || directInputRows.length !== 0) throw new Error("source direct inputs lost root or completion");
if (directInputCopiedBytes !== JSON.parse(directInputBytes.toString()).copiedStringBytes) {
  throw new Error("dependency byte accounting drift");
}
if (directInputCursor !== null) baseline.directDependencies(directInputs.subject(), 1, directInputCursor);
const discovered: WorkspaceComparisonSubjectPage = equalPair.subjects(1);
const discoveredSubjects: WorkspaceSubject[] = discovered.subjects();
const discoveredSummary: WorkspaceComparisonSummary = discovered.comparison();
const discoveredCursor: WorkspaceComparisonSubjectCursor | null = discovered.continuation();
const discoveredCounts: number[] = [discovered.inspected, discovered.keyComparisons];
const discoveredIdentityBytes: number = discovered.subjectIdentityBytes;
const discoveredBytes: Buffer = discovered.canonicalBytes;
const discoveredComplete: boolean = discovered.isComplete;
if (!discovered.belongsTo(equalPair) || discoveredSummary.receiptDigest !== equalPair.receiptDigest) throw new Error("subject discovery lost comparison custody");
if (discoveredIdentityBytes <= 0
  || discoveredIdentityBytes !== JSON.parse(discoveredBytes.toString()).subjectIdentityBytes) {
  throw new Error("subject identity byte accounting drift");
}
if (discoveredCursor !== null) equalPair.subjects(1, discoveredCursor);
discoveredSubjects.map(subject => equalPair.compareSubject(subject));
if (false) {
  // @ts-expect-error A detached terminal dependency result has no owner predicate.
  directInputs.belongsTo(baseline);
}
// @ts-expect-error The dependency projection does not add admission coordinates.
const inventedDependencyCoordinate: string = directInputs.admissionEvidenceDigest;
// @ts-expect-error Canonical answer bytes do not reconstruct dependency custody.
const reconstructedInput: PortableWorkspaceDependencyCursor = directInputBytes;
// @ts-expect-error Subject discovery has its own opaque continuation.
const reconstructedDiscovery: WorkspaceComparisonSubjectCursor = discoveredBytes;
void [directInputCoordinates, directInputCopiedBytes, discoveredCounts, discoveredIdentityBytes,
  discoveredComplete, inventedDependencyCoordinate,
  reconstructedInput, reconstructedDiscovery];
// @ts-expect-error Canonical answer bytes do not reconstruct a contributing-root cursor.
const reconstructedContribution: WorkspaceContributingRootsCursor = contributionBytes;
// @ts-expect-error Canonical answer bytes do not reconstruct a direct-dependent cursor.
const reconstructedDependent: PortableWorkspaceDependentCursor = dependentBytes;
// @ts-expect-error Dependent rows are typed compiler facts, not generic JSON strings.
const untypedDependents: string[] = dependentRows;
void [dependentCounts, dependentComplete, dependentHeaders, reconstructedContribution, reconstructedDependent, untypedDependents];
// @ts-expect-error Canonical bytes do not reconstruct an opaque continuation.
const reconstructedInspection: PortableWorkspaceSubjectInspectionCursor = inspectionBytes;
// @ts-expect-error Composed input rows remain typed compiler facts, not generic JSON.
const untypedInspectionInputs: string[] = inspectionInputs;
void [inspectionOccurrences, inspectionAnchor, inspectionInputsComplete, inspectionSourcesComplete,
  inspectionComplete, inspectionInspected, inspectionHeaders, reconstructedInspection, untypedInspectionInputs];
const unknownSource = api.portableWorkspaceSourceSubject(baseline, "not-captured.toml", []);
if (unknownSource.state !== PortableWorkspaceSourceState.NoOccurrence || unknownSource.subject() !== null
  || unknownSource.occurrence() !== null) throw new Error("missing occurrence acquired a subject");
// @ts-expect-error A copied source location is not an opaque compilation handle.
const fabricatedInspection: PortableWorkspaceSourceInspection = sourceLocation;
void fabricatedInspection;
void [complete, inspected, privateInstance, associatedRows, portableOccurrences];
const intendedEntry = page.entries[0];
if (intendedEntry === undefined) throw new Error("intended-use page is empty");
const intendedDetail = intendedEntry.detail;
entryCoordinate(intendedDetail);
if (intendedDetail.kind === "responsibility" || intendedDetail.kind === "execution-path") {
  const copiedSubject: WorkspaceSubjectValue = intendedDetail.subject;
  subjectCoordinate(copiedSubject);
  // @ts-expect-error Copied selectors are not admitted comparison handles.
  const admittedSubject: WorkspaceSubject = copiedSubject;
  void admittedSubject;
}
const disposition: ConfigurationImprovementDisposition = preview.accept().disposition;
if (disposition !== ConfigurationImprovementDisposition.Accepted) throw new Error("wrong terminal");
// @ts-expect-error Stage handles are distinct, not a generic state dispatcher.
const wrongPhase: ConfigurationImprovementPreview = explanation;
void wrongPhase;
console.log(JSON.stringify({
  schema: schema.registryId,
  manifest: api.contractBundleIdentity().schemaManifestSha256,
  nativeBufferTyped: true,
  asynchronousCompilation: true,
  localPreviewRejected,
  invalidRootRejected: rejected,
  improvementCandidate: summary.candidateInstanceDigest,
  improvementAccepted: true,
}, null, 2));
