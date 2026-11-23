import assert from "node:assert/strict";
import path from "node:path";
import { createRequire } from "node:module";
import ts from "typescript";

// Executed beside an installed packed package, never against checkout types.
const consumer = path.join(process.cwd(), "nominal-consumer.ts");
const preamble = `import { Pale, OneShotCompilation, CapturedNativeActionRequest,
  ResidentHookAdmission, ConfigurationImprovementProposal,
  ConfigurationImprovementPhase, OneShotCompilationRequest,
  AuthoringOccurrencePageCursor, AuthoringOccurrencePage,
  EffectiveAuthoringContinuation, EffectiveAuthoringPage,
  ObservationSnapshot, ObservationSubject, ObservationProfileAssociation,
  ObservationProfileAssociationResult, ObservationProfileContinuation, ObservationProfilePage, ObservationProfileResult,
  WorkspaceComparison, WorkspaceSubjectComparison, WorkspaceSubject,
  PortableWorkspaceSourceInspection, PortableWorkspaceSubjectOccurrences,
  PortableWorkspaceSubjectInspection, PortableWorkspaceSubjectInspectionCursor,
  WorkspaceContributingRootsPage, WorkspaceContributingRootsCursor,
  PortableWorkspaceDependentPage, PortableWorkspaceDependentCursor,
  PortableWorkspaceDependencyPage, PortableWorkspaceDependencyCursor,
  WorkspaceComparisonSubjectPage, WorkspaceComparisonSubjectCursor,
  WorkspaceImpactPage,
  RecipientQualifiedWorkspaceInspection, RecipientQualifiedWorkspaceComparisonInspection,
  AuthoringIdentity, PortableWorkspaceCompilation, EffectiveProfileCompilation } from '@rtepl/pale';
declare const admitted: OneShotCompilation;
declare const action: CapturedNativeActionRequest;
declare const admission: ResidentHookAdmission;
declare const input: OneShotCompilationRequest;
declare const identity: AuthoringIdentity;
declare const workspace: PortableWorkspaceCompilation;
declare const profile: EffectiveProfileCompilation;
declare const observation: ObservationSnapshot;
declare const observedSubject: ObservationSubject;
declare const profileCursor: ObservationProfileContinuation;
declare const occurrenceCursor: AuthoringOccurrencePageCursor;
declare const associationCursor: EffectiveAuthoringContinuation;
declare const inspectionCursor: PortableWorkspaceSubjectInspectionCursor;
declare const contributionCursor: WorkspaceContributingRootsCursor;
declare const dependentCursor: PortableWorkspaceDependentCursor;
declare const dependencyCursor: PortableWorkspaceDependencyCursor;
declare const discoveryCursor: WorkspaceComparisonSubjectCursor;
declare const comparison: WorkspaceComparison;
declare const subject: WorkspaceSubject;
const api = new Pale();
api.oneShotSummary(admitted);
const pending: Promise<OneShotCompilation> = api.compileOneShot(input);
async function use() { api.oneShotSummary(await pending); }
api.portableWorkspaceAuthoringOccurrences(workspace, identity, 1, 1, occurrenceCursor);
profile.authoringOccurrences(identity, 1, 1, occurrenceCursor);
profile.inspectEffectiveAuthoringPage(identity, 1, associationCursor);
const subjectComparison: WorkspaceSubjectComparison = comparison.compareSubject(subject);
comparison.compareSubject(subjectComparison.subject());
const sourceInspection: PortableWorkspaceSourceInspection = api.portableWorkspaceSourceSubject(workspace, 'source.toml', []);
sourceInspection.belongsTo(workspace);
const sourceMatches: PortableWorkspaceSubjectOccurrences = api.portableWorkspaceSubjectOccurrences(workspace, subject, 1, 1, occurrenceCursor);
sourceMatches.page().occurrences();
const inspected: PortableWorkspaceSubjectInspection = workspace.inspectSubject(subject, 1, 1, 1, inspectionCursor);
inspected.belongsTo(workspace);
inspected.root()?.fact();
inspected.dependencies().map(step => step.subject());
const nextInspection: PortableWorkspaceSubjectInspectionCursor | null = inspected.continuation();
const contribution: WorkspaceContributingRootsPage = comparison.contributingRoots(subject, 1, contributionCursor);
contribution.belongsTo(comparison); contribution.comparison().receiptDigest;
contribution.roots().map(root => root.currentFact());
const nextContribution: WorkspaceContributingRootsCursor | null = contribution.continuation();
const dependent: PortableWorkspaceDependentPage = workspace.directDependents(subject, 1, 1, dependentCursor);
dependent.belongsTo(workspace); dependent.root()?.fact();
dependent.dependents().map(step => step.provenance());
const nextDependent: PortableWorkspaceDependentCursor | null = dependent.continuation();
const dependency: PortableWorkspaceDependencyPage = workspace.directDependencies(subject, 1, dependencyCursor);
dependency.root()?.fact(); dependency.dependencies().map(step => step.provenance());
const nextDependency: PortableWorkspaceDependencyCursor | null = dependency.continuation();
const discovery: WorkspaceComparisonSubjectPage = comparison.subjects(1, discoveryCursor);
discovery.belongsTo(comparison); discovery.subjects().map(subject => comparison.compareSubject(subject));
const nextDiscovery: WorkspaceComparisonSubjectCursor | null = discovery.continuation();
const impact: WorkspaceImpactPage = comparison.impacts(0, 1);
const impactBytes: Buffer = impact.canonicalBytes;
const impactNext: number | undefined = impact.nextOffset;
impact.entries.map(entry => entry.subject());
`;
const cases = [
  "const forgedObservationCursor = new ObservationProfileContinuation();",
  "const forgedProfileAssociation = new ObservationProfileAssociation();",
  "const forgedProfileAssociationResult = new ObservationProfileAssociationResult();",
  "const forgedObservationPage = new ObservationProfilePage();",
  "const forgedObservationResult = new ObservationProfileResult();",
  "const objectObservationCursor: ObservationProfileContinuation = {};",
  "const swappedObservationCursor: ObservationProfileContinuation = associationCursor;",
  "observation.inspectProfile(profile, workspace, observedSubject, 1, 1, associationCursor);",
  "profile.inspectEffectiveAuthoringPage(identity, 1, profileCursor);",
  "const objectCompilation: OneShotCompilation = {};",
  "const objectAction: CapturedNativeActionRequest = {};",
  "const objectAdmission: ResidentHookAdmission = {};",
  "const swappedOne: OneShotCompilation = action;",
  "const swappedTwo: CapturedNativeActionRequest = admission;",
  "const swappedThree: ResidentHookAdmission = admitted;",
  "const premature: OneShotCompilation = pending;",
  "const forgedConstructor = new OneShotCompilation();",
  "const forgedShape: ConfigurationImprovementProposal = {phase: ConfigurationImprovementPhase.Proposed};",
  "const forgedOccurrence = new AuthoringOccurrencePageCursor();",
  "const forgedAssociation = new EffectiveAuthoringContinuation();",
  "const forgedOccurrencePage = new AuthoringOccurrencePage();",
  "const forgedAssociationPage = new EffectiveAuthoringPage();",
  "const objectCursor: AuthoringOccurrencePageCursor = {};",
  "const swappedCursor: EffectiveAuthoringContinuation = occurrenceCursor;",
  "profile.authoringOccurrences(identity, 1, 1, associationCursor);",
  "profile.inspectEffectiveAuthoringPage(identity, 1, occurrenceCursor);",
  "api.portableWorkspaceAuthoringOccurrences(workspace, identity, '1', 1);",
  "profile.authoringOccurrences(identity, 1, '1');",
  "profile.inspectEffectiveAuthoringPage(identity, '1');",
  "const forgedSubjectComparison = new WorkspaceSubjectComparison();",
  "const objectSubjectComparison: WorkspaceSubjectComparison = {};",
  "comparison.compareSubject({});",
  "comparison.compareSubject(identity);",
  "const forgedSourceInspection = new PortableWorkspaceSourceInspection();",
  "const forgedSourceMatches = new PortableWorkspaceSubjectOccurrences();",
  "const objectSourceInspection: PortableWorkspaceSourceInspection = {};",
  "const objectSourceMatches: PortableWorkspaceSubjectOccurrences = {};",
  "api.portableWorkspaceSourceSubject(workspace, 'source.toml', [{}]);",
  "api.portableWorkspaceSubjectOccurrences(workspace, identity, 1, 1);",
  "api.portableWorkspaceSubjectOccurrences(workspace, subject, '1', 1);",
  "api.portableWorkspaceSubjectOccurrences(workspace, subject, 1, '1');",
  "api.portableWorkspaceSubjectOccurrences(workspace, subject, 1, 1, associationCursor);",
  "sourceInspection.belongsTo(profile);",
  "const forgedInspection = new PortableWorkspaceSubjectInspection();",
  "const forgedInspectionCursor = new PortableWorkspaceSubjectInspectionCursor();",
  "const objectInspection: PortableWorkspaceSubjectInspection = {};",
  "const objectInspectionCursor: PortableWorkspaceSubjectInspectionCursor = {};",
  "const swappedInspection: PortableWorkspaceSubjectInspectionCursor = occurrenceCursor;",
  "workspace.inspectSubject(subject, 1, 1, 1, occurrenceCursor);",
  "workspace.inspectSubject(subject, 1, 1, 1, associationCursor);",
  "api.portableWorkspaceSubjectOccurrences(workspace, subject, 1, 1, inspectionCursor);",
  "workspace.inspectSubject(identity, 1, 1, 1);",
  "workspace.inspectSubject(subject, '1', 1, 1);",
  "workspace.inspectSubject(subject, 1, '1', 1);",
  "workspace.inspectSubject(subject, 1, 1, '1');",
  "inspected.belongsTo(profile);",
  "const forgedContributionPage = new WorkspaceContributingRootsPage();",
  "const forgedContributionCursor = new WorkspaceContributingRootsCursor();",
  "const forgedDependentPage = new PortableWorkspaceDependentPage();",
  "const forgedDependentCursor = new PortableWorkspaceDependentCursor();",
  "const objectContribution: WorkspaceContributingRootsPage = {};",
  "const objectContributionCursor: WorkspaceContributingRootsCursor = {};",
  "const objectDependent: PortableWorkspaceDependentPage = {};",
  "const objectDependentCursor: PortableWorkspaceDependentCursor = {};",
  "const swappedDependentCursor: PortableWorkspaceDependentCursor = contributionCursor;",
  "const swappedContributionCursor: WorkspaceContributingRootsCursor = dependentCursor;",
  "comparison.contributingRoots(identity, 1);",
  "workspace.directDependents(identity, 1, 1);",
  "comparison.contributingRoots(subject, '1');",
  "workspace.directDependents(subject, '1', 1);",
  "workspace.directDependents(subject, 1, '1');",
  "comparison.contributingRoots(subject, 1, dependentCursor);",
  "workspace.directDependents(subject, 1, 1, contributionCursor);",
  "workspace.directDependents(subject, 1, 1, inspectionCursor);",
  "contribution.belongsTo(workspace);",
  "dependent.belongsTo(comparison);",
  "const forgedDependencyPage = new PortableWorkspaceDependencyPage();",
  "const forgedDependencyCursor = new PortableWorkspaceDependencyCursor();",
  "const forgedDiscoveryPage = new WorkspaceComparisonSubjectPage();",
  "const forgedDiscoveryCursor = new WorkspaceComparisonSubjectCursor();",
  "const objectDependencyPage: PortableWorkspaceDependencyPage = {};",
  "const objectDependencyCursor: PortableWorkspaceDependencyCursor = {};",
  "const objectDiscoveryPage: WorkspaceComparisonSubjectPage = {};",
  "const objectDiscoveryCursor: WorkspaceComparisonSubjectCursor = {};",
  "const forgedImpactPage = new WorkspaceImpactPage();",
  "const objectImpactPage: WorkspaceImpactPage = {};",
  "workspace.directDependencies(identity, 1);",
  "workspace.directDependencies(subject, '1');",
  "comparison.subjects('1');",
  "workspace.directDependencies(subject, 1, dependentCursor);",
  "workspace.directDependencies(subject, 1, discoveryCursor);",
  "comparison.subjects(1, dependencyCursor);",
  "comparison.subjects(1, contributionCursor);",
  "dependency.belongsTo(workspace);",
  "const inventedWorkspace: string = dependency.workspaceId;",
  "const inventedAdmission: string = dependency.admissionEvidenceDigest;",
  "discovery.belongsTo(workspace);",
  "const decodedDependency: PortableWorkspaceDependencyCursor = dependency.canonicalBytes;",
  "const decodedDiscovery: WorkspaceComparisonSubjectCursor = discovery.canonicalBytes;",
  "const forgedQualifiedWorkspace = new RecipientQualifiedWorkspaceInspection();",
  "const forgedQualifiedComparison = new RecipientQualifiedWorkspaceComparisonInspection();",
  "const objectQualifiedWorkspace: RecipientQualifiedWorkspaceInspection = {};",
  "const objectQualifiedComparison: RecipientQualifiedWorkspaceComparisonInspection = {};",
];
function check(text) {
  const options = {
    strict: true, noEmit: true, skipLibCheck: false, types: [],
    target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.NodeNext,
    moduleResolution: ts.ModuleResolutionKind.NodeNext,
  };
  const host = ts.createCompilerHost(options);
  const read = host.getSourceFile.bind(host);
  host.getSourceFile = (file, version, ...args) => file === consumer
    ? ts.createSourceFile(file, text, version, true) : read(file, version, ...args);
  return ts.getPreEmitDiagnostics(ts.createProgram([consumer], options, host));
}
assert.deepEqual(check(preamble), []);
const rejected = check(preamble + cases.join("\n"));
assert.equal(rejected.length, cases.length);
const firstLine = preamble.split("\n").length - 1;
for (const [index, diagnostic] of rejected.entries()) {
  assert.equal(diagnostic.file?.fileName, consumer);
  assert.equal(diagnostic.file.getLineAndCharacterOfPosition(diagnostic.start).line, firstLine + index);
}
const binding = createRequire(import.meta.url)("@rtepl/pale");
for (const Handle of [binding.OneShotCompilation, binding.CapturedNativeActionRequest, binding.ResidentHookAdmission,
  binding.AuthoringOccurrencePageCursor, binding.EffectiveAuthoringContinuation,
  binding.AuthoringOccurrencePage, binding.EffectiveAuthoringPage, binding.WorkspaceSubjectComparison,
  binding.PortableWorkspaceSourceInspection, binding.PortableWorkspaceSubjectOccurrences,
  binding.PortableWorkspaceSubjectInspection, binding.PortableWorkspaceSubjectInspectionCursor,
  binding.WorkspaceContributingRootsPage, binding.WorkspaceContributingRootsCursor,
  binding.PortableWorkspaceDependentPage, binding.PortableWorkspaceDependentCursor,
  binding.PortableWorkspaceDependencyPage, binding.PortableWorkspaceDependencyCursor,
  binding.WorkspaceComparisonSubjectPage, binding.WorkspaceComparisonSubjectCursor,
  binding.WorkspaceImpactPage,
  binding.RecipientQualifiedWorkspaceInspection,
  binding.RecipientQualifiedWorkspaceComparisonInspection,
  binding.ObservationProfileAssociation, binding.ObservationProfileAssociationResult,
  binding.ObservationProfileContinuation, binding.ObservationProfilePage, binding.ObservationProfileResult]) {
  assert.throws(() => new Handle());
}
assert.throws(() => new binding.Pale().oneShotSummary({}));
console.log(JSON.stringify({ rejectedMisuses: cases.length, admittedAndAwaitedHandlesAccepted: true, nativeIdentityUnchanged: true }, null, 2));
