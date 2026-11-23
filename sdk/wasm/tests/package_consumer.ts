import {
  Pale, AuthoringCorrection, AuthoringCorrectionKind, AuthoringSourceLocation,
  AuthoringIdentity, AuthoringInstancePathSegment, AuthoringOccurrence,
  AuthoringOccurrenceRole, AuthoringOccurrencePage, AuthoringOccurrencePageCursor,
  PortableWorkspaceDocument, PortableWorkspaceRequest,
  WorkspaceSubjectComparison, WorkspaceSubjectMembership, WorkspaceValueComparison,
  WorkspaceChangeRootPage, WorkspaceExplanation, WorkspaceImpactPage,
  WorkspaceFact, WorkspaceFactKind,
  AuthoringValueKind, StandaloneAuthoringRole,
  SCHEMA_MANIFEST_SHA256, SCHEMA_REGISTRY_ID, SCHEMA_SET_SHA256,
} from "@rtepl/pale-wasm";
import {
  validate, type ContractByFamily,
  SCHEMA_MANIFEST_SHA256 as contractManifest,
  SCHEMA_REGISTRY_ID as contractRegistry,
  SCHEMA_SET_SHA256 as contractSet,
} from "@rtepl/pale-contracts";
import fixture from "./contract-diagnostic.json" with { type: "json" };
import portable from "./portable-input.json" with { type: "json" };
import * as bindings from "@rtepl/pale-wasm";
import { verifyConsumedArguments } from "./consumed_arguments.mjs";

verifyConsumedArguments(bindings, portable, fixture);

function rejectConsumedArgumentShapes(): void {
  // @ts-expect-error Compilation requires a generated request handle.
  api.compilePortableWorkspace({});
  // @ts-expect-error Diagnostic projection has a different handle type.
  api.projectDiagnostic(workspace);
  // @ts-expect-error Documents are handles, not source strings.
  new PortableWorkspaceRequest(portable.rootManifest, ["source"]);
  // @ts-expect-error Grammar paths require generated path segments.
  api.inspectStandaloneAuthoring(StandaloneAuthoringRole.WorkspaceManifest, ["workspace"]);
  // @ts-expect-error Instance paths also require generated segments.
  workspace.navigateAuthoring(portable.rootManifest, ["workspace"]);
}

function assert(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(message);
}

const api = new Pale();
for (const invalid of [-1, 0.5, Number.NaN, Number.POSITIVE_INFINITY, 2 ** 32]) {
  let rejected = false;
  try { AuthoringInstancePathSegment.index(invalid); } catch { rejected = true; }
  assert(rejected, "the packed package rejects a lossy authoring instance-path index");
}
const maximumInstanceIndex = AuthoringInstancePathSegment.index(2 ** 32 - 1);
assert(maximumInstanceIndex.indexValue === 4294967295n,
  "the packed package preserves the largest admitted authoring instance-path index");
maximumInstanceIndex.free();
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
  let rejected = false;
  try { causeProjection.setCauses(["must not replace the retained cause"], invalid); }
  catch { rejected = true; }
  assert(rejected, "the packed package rejects a lossy omitted-cause count");
}
const causeDiagnostic = JSON.parse(new TextDecoder().decode(api.projectDiagnostic(causeProjection)));
assert(Array.isArray(causeDiagnostic.causes) && causeDiagnostic.causes.length === 1
  && causeDiagnostic.causes[0] === "retained cause"
  && causeDiagnostic.truncation?.omitted_causes === 1,
"rejected omitted-cause counts cannot replace admitted diagnostic causes");
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
assert(maximumOmittedDiagnostic.truncation?.omitted_causes === 4294967295,
  "the packed package preserves the largest admitted omitted-cause count");
const workspace = api.compilePortableWorkspace(new PortableWorkspaceRequest(
  portable.rootManifest,
  portable.documents.map(document => new PortableWorkspaceDocument(document.label, new Uint8Array(document.bytes))),
));
const navigation = workspace.navigateAuthoring(portable.rootManifest, [
  AuthoringInstancePathSegment.field("workspace"), AuthoringInstancePathSegment.field("workspace_id"),
]);
const authoringIdentity: AuthoringIdentity | undefined = navigation.identity();
assert(authoringIdentity !== undefined, "the root workspace identity is compiler-produced");
const copiedStringBytesGetter = Object.getOwnPropertyDescriptor(
  AuthoringOccurrencePage.prototype, "copiedStringBytes",
)?.get;
assert(typeof copiedStringBytesGetter === "function",
  "the packed page exposes its copied-string counter as a native getter");
for (const fabricated of [{}, Object.create(AuthoringOccurrencePage.prototype)]) {
  let rejected = false;
  try { copiedStringBytesGetter.call(fabricated); } catch { rejected = true; }
  assert(rejected, "the copied-string counter rejects fabricated native page wrappers");
}
let cursor: AuthoringOccurrencePageCursor | undefined;
let matched = 0;
let pages = 0;
do {
  const page: AuthoringOccurrencePage = workspace.authoringOccurrences(authoringIdentity, 1, 1, cursor);
  cursor = undefined; // The call consumed that handle.
  try {
    const inspected: number = page.inspected;
    const copiedStringBytes: number = page.copiedStringBytes;
    const complete: boolean = page.isComplete;
    assert(inspected <= 1 && Number.isInteger(copiedStringBytes) && copiedStringBytes >= 0,
      "the compiler reports its scan and copied-string bounds");
    const occurrences: AuthoringOccurrence[] = page.occurrences();
    const expectedCopiedStringBytes = occurrences.reduce((pageBytes, occurrence) => {
      const location = occurrence.location();
      const segments = location.path();
      const pathBytes = segments.reduce((sum, segment) => sum
        + (segment.kind() === bindings.AuthoringInstancePathSegmentKind.Field
          ? new TextEncoder().encode(segment.fieldValue!).length : 0), 0);
      for (const segment of segments) segment.free();
      const bytes = new TextEncoder().encode(location.sourceLabel).length + pathBytes;
      location.free();
      return pageBytes + bytes;
    }, 0);
    assert(copiedStringBytes === expectedCopiedStringBytes,
      "the packed package preserves exact occurrence string accounting");
    for (const occurrence of occurrences) {
      const location: AuthoringSourceLocation = occurrence.location();
      const role: AuthoringOccurrenceRole = occurrence.role();
      assert(location.sourceLabel.length > 0 && Object.values(AuthoringOccurrenceRole).includes(role),
        "the packed package preserves typed locations and compiler roles");
      location.free(); occurrence.free(); matched += 1;
    }
    cursor = page.continuation();
    assert((cursor === undefined) === complete, "only proven exhaustion ends the scan");
    assert(++pages < 2048, "the fixture must terminate");
  } finally { page.free(); }
} while (cursor !== undefined);
assert(matched > 1, "the complete scan returns multiple actual source occurrences");
function rejectMalformedOccurrenceRequests(): void {
  // @ts-expect-error Page limits are numbers, not coercible strings.
  workspace.authoringOccurrences(authoringIdentity!, "1", 1);
  // @ts-expect-error Cursors are opaque handles, not text.
  workspace.authoringOccurrences(authoringIdentity!, 1, 1, "cursor");
  // @ts-expect-error Only omission, not null, starts a first page.
  workspace.authoringOccurrences(authoringIdentity!, 1, 1, null);
  // @ts-expect-error Occurrence pages are produced by the compiler.
  new AuthoringOccurrencePage();
  // @ts-expect-error Cursors are produced by a page.
  new AuthoringOccurrencePageCursor();
  // @ts-expect-error Occurrences are produced by the compiler.
  new AuthoringOccurrence();
}
void rejectMalformedOccurrenceRequests;
const comparison = api.comparePortableWorkspaces(workspace, workspace);
assert(comparison.priorPrincipalUniverseDigest === workspace.principalUniverseDigest
  && comparison.currentPrincipalUniverseDigest === workspace.principalUniverseDigest,
"comparison retains both exact universe identities");
assert(comparison.priorSemanticIrDigest === workspace.semanticIrDigest
  && comparison.currentSemanticIrDigest === workspace.semanticIrDigest,
"comparison retains both exact semantic identities");
for (const invalid of [-1, 0.5, Number.NaN, Number.POSITIVE_INFINITY, 2 ** 32]) {
  for (const page of ["changeRoots", "impacts"] as const) {
    let rejected = false;
    try { comparison[page](invalid, 1); } catch { rejected = true; }
    assert(rejected, `${page} rejects a lossy offset before Rust integer conversion`);
  }
}
for (const invalid of [-1, 0.5, Number.NaN, Number.POSITIVE_INFINITY, 65536]) {
  for (const page of ["changeRoots", "impacts"] as const) {
    let rejected = false;
    try { comparison[page](0, invalid); } catch { rejected = true; }
    assert(rejected, `${page} rejects a lossy limit before Rust integer conversion`);
  }
}
const changeRootPage: WorkspaceChangeRootPage = comparison.changeRoots(0, 1);
const changeRootPageBytes: Uint8Array = changeRootPage.canonicalBytes();
const changeRootDocument: unknown = JSON.parse(new TextDecoder().decode(changeRootPageBytes));
assert(typeof changeRootDocument === "object" && changeRootDocument !== null
  && "entries" in changeRootDocument && Array.isArray(changeRootDocument.entries)
  && changeRootDocument.entries.length === 0
  && "nextOffset" in changeRootDocument && changeRootDocument.nextOffset === null,
"the packed change-root page preserves the SDK-owned canonical payload");
const changeRootDigest = new Uint8Array(await crypto.subtle.digest(
  "SHA-256", new Uint8Array(changeRootPageBytes).buffer,
));
const repeatedChangeRootPage = comparison.changeRoots(0, 1);
const repeatedChangeRootBytes = repeatedChangeRootPage.canonicalBytes();
const repeatedChangeRootDigest = new Uint8Array(await crypto.subtle.digest(
  "SHA-256", new Uint8Array(repeatedChangeRootBytes).buffer,
));
assert(changeRootDigest.length === 32
  && changeRootDigest.every((byte, index) => byte === repeatedChangeRootDigest[index])
  && changeRootPageBytes.every((byte, index) => byte === repeatedChangeRootBytes[index]),
"the retained comparison and change-root coordinates preserve the canonical payload digest");
repeatedChangeRootPage.free();
const retainedChangeRootBytes = changeRootPage.canonicalBytes();
changeRootPageBytes.fill(0);
assert(changeRootPage.canonicalBytes().every(
  (byte, index) => byte === retainedChangeRootBytes[index],
), "mutating returned change-root bytes cannot rewrite the retained canonical payload");
const comparisonPage = comparison.impacts(0, 1);
const comparisonPageBytes: Uint8Array = comparisonPage.canonicalBytes();
const comparisonPageDocument: unknown = JSON.parse(new TextDecoder().decode(comparisonPageBytes));
assert(typeof comparisonPageDocument === "object" && comparisonPageDocument !== null
  && "entries" in comparisonPageDocument && Array.isArray(comparisonPageDocument.entries)
  && comparisonPageDocument.entries.length === 1,
"the packed impact page preserves the SDK-owned canonical payload");
const comparisonPageDigest = new Uint8Array(await crypto.subtle.digest(
  "SHA-256", new Uint8Array(comparisonPageBytes).buffer,
));
const repeatedComparisonPage = comparison.impacts(0, 1);
const repeatedComparisonPageBytes = repeatedComparisonPage.canonicalBytes();
const repeatedComparisonPageDigest = new Uint8Array(
  await crypto.subtle.digest("SHA-256", new Uint8Array(repeatedComparisonPageBytes).buffer),
);
assert(comparisonPageDigest.length === 32
  && comparisonPageDigest.every((byte, index) => byte === repeatedComparisonPageDigest[index])
  && comparisonPageBytes.every((byte, index) => byte === repeatedComparisonPageBytes[index]),
"the retained comparison and page coordinates preserve the canonical payload digest");
repeatedComparisonPage.free();
const retainedComparisonPageBytes = comparisonPage.canonicalBytes();
comparisonPageBytes.fill(0);
assert(comparisonPage.canonicalBytes().every(
  (byte, index) => byte === retainedComparisonPageBytes[index],
), "mutating returned bytes cannot rewrite the retained canonical payload");
const comparisonEntries = comparisonPage.intoEntries();
const comparisonImpact = comparisonEntries[0];
assert(comparisonImpact !== undefined, "the actual compilation contains a subject");
const comparisonSubject = comparisonImpact.subject();
const explanation: WorkspaceExplanation = comparison.explain(comparisonSubject);
assert(explanation.state() === bindings.WorkspaceExplanationState.Unaffected
  && explanation.root() === undefined && explanation.frontier() === undefined
  && explanation.steps().length === 0,
"the typed explanation retains the unchanged subject outcome");
const explanationBytes: Uint8Array = explanation.canonicalBytes();
const explanationDocument: unknown = JSON.parse(new TextDecoder().decode(explanationBytes));
assert(typeof explanationDocument === "object" && explanationDocument !== null
  && "state" in explanationDocument && explanationDocument.state === "unaffected"
  && "causal" in explanationDocument && explanationDocument.causal === null
  && "subject" in explanationDocument,
"the packed explanation preserves the SDK-owned canonical payload");
const explanationDigest = new Uint8Array(await crypto.subtle.digest(
  "SHA-256", new Uint8Array(explanationBytes).buffer,
));
const repeatedExplanationSubject = explanation.subject();
const repeatedExplanation = comparison.explain(repeatedExplanationSubject);
repeatedExplanationSubject.free();
const repeatedExplanationBytes = repeatedExplanation.canonicalBytes();
const repeatedExplanationDigest = new Uint8Array(await crypto.subtle.digest(
  "SHA-256", new Uint8Array(repeatedExplanationBytes).buffer,
));
assert(explanationDigest.length === 32
  && explanationDigest.every((byte, index) => byte === repeatedExplanationDigest[index])
  && explanationBytes.every((byte, index) => byte === repeatedExplanationBytes[index]),
"the retained comparison and exact subject preserve the canonical explanation digest");
const retainedExplanationBytes = explanation.canonicalBytes();
explanationBytes.fill(0);
assert(explanation.canonicalBytes().every(
  (byte, index) => byte === retainedExplanationBytes[index],
), "mutating returned explanation bytes cannot rewrite the retained canonical payload");
const subjectResult: WorkspaceSubjectComparison = comparison.compareSubject(comparisonSubject);
const membership: WorkspaceSubjectMembership = subjectResult.membership();
const equality: WorkspaceValueComparison | undefined = subjectResult.comparison();
assert(membership === WorkspaceSubjectMembership.Shared && equality === WorkspaceValueComparison.Equal,
  "an unchanged subject is shared and equal");
const priorFact: WorkspaceFact | undefined = subjectResult.priorFact();
const currentFact: WorkspaceFact | undefined = subjectResult.currentFact();
assert(priorFact !== undefined && currentFact !== undefined, "shared membership carries both facts");
assert(priorFact.kind() === WorkspaceFactKind.SourceContent && currentFact.kind() === WorkspaceFactKind.SourceContent,
  "the fixture's first impact retains source-content facts");
assert(priorFact.sourceDigest === currentFact.sourceDigest && priorFact.sourcePath === currentFact.sourcePath,
  "equal facts retain their payloads");
const subjectBytes: Uint8Array = subjectResult.canonicalBytes();
assert(subjectResult.receiptDigest === comparison.receiptDigest, "result joins the exact parent receipt");
const copiedComparisonSubject = subjectResult.subject();
const repeatedSubjectResult = comparison.compareSubject(copiedComparisonSubject);
const repeatedSubjectBytes = repeatedSubjectResult.canonicalBytes();
assert(subjectBytes.length === repeatedSubjectBytes.length
  && subjectBytes.every((byte, index) => repeatedSubjectBytes[index] === byte),
  "the copied opaque subject resolves to the same canonical answer");
function rejectMalformedSubjectComparison(): void {
  // @ts-expect-error Results are made by the retained comparison.
  new WorkspaceSubjectComparison();
  // @ts-expect-error Subjects are opaque native values, not JSON descriptions.
  comparison.compareSubject({ kind: "source", sourceId: "source" });
  // @ts-expect-error Impact pages are output-only handles, not sparse transport objects.
  new WorkspaceImpactPage();
  // @ts-expect-error Change-root pages are output-only handles, not sparse transport objects.
  new WorkspaceChangeRootPage();
  // @ts-expect-error Explanations are output-only handles, not sparse transport objects.
  new WorkspaceExplanation();
  // @ts-expect-error Explanation subjects are opaque native values, not sparse transport objects.
  comparison.explain({ kind: "source", sourceId: "source" });
  // @ts-expect-error Change-root offsets are exact integers, not sparse transport objects.
  comparison.changeRoots({ offset: 0 }, 1);
  // @ts-expect-error Impact offsets are exact integers, not sparse transport objects.
  comparison.impacts({ offset: 0 }, 1);
  // @ts-expect-error Membership is an enum, not an untyped string.
  const invalidMembership: string = subjectResult.membership();
  // @ts-expect-error Compiler facts cannot be constructed by a caller.
  new WorkspaceFact();
  void invalidMembership;
}
void rejectMalformedSubjectComparison;
comparison.free();
assert(changeRootPage.canonicalBytes().every(
  (byte, index) => byte === retainedChangeRootBytes[index],
), "the exact change-root bytes survive their parent comparison");
assert(changeRootPage.intoEntries().length === 0, "the terminal typed page survives its parent comparison");
assert(explanation.canonicalBytes().every(
  (byte, index) => byte === retainedExplanationBytes[index],
), "the exact explanation bytes survive their parent comparison");
assert(subjectResult.canonicalBytes().length === subjectBytes.length, "the result survives its parent");
subjectResult.free();
explanation.free(); repeatedExplanation.free();
assert(currentFact.sourceDigest === priorFact.sourceDigest, "copied facts survive the result");
priorFact.free(); currentFact.free(); repeatedSubjectResult.free();
// Both supplied subject handles were consumed by compareSubject.
for (const entry of comparisonEntries) entry.free();
authoringIdentity.free(); navigation.free(); workspace.free();
const grammar = api.inspectStandaloneAuthoring(StandaloneAuthoringRole.WorkspaceManifest, []);
const rootNode = grammar.node();
assert(rootNode !== undefined, "the workspace schema has a root node");
try {
  const kinds: AuthoringValueKind[] = rootNode.valueKinds();
  assert(Array.isArray(kinds) && kinds.includes(AuthoringValueKind.Table), "root kind is a table");
  // @ts-expect-error Typed grammar kinds must not degrade to any[].
  const wrongNodeKinds: string[] = rootNode.valueKinds();
  void wrongNodeKinds;
  for (const field of rootNode.fields()) {
    try {
      const fieldKinds: AuthoringValueKind[] = field.valueKinds();
      assert(Array.isArray(fieldKinds), "field kinds are an ordinary array");
      assert(fieldKinds.every(kind => Object.values(AuthoringValueKind).includes(kind)),
        "field kinds belong to the compiler-owned enum");
      // @ts-expect-error Field kinds retain their enum element type.
      const wrongFieldKinds: string[] = field.valueKinds();
      void wrongFieldKinds;
    } finally { field.free(); }
  }
} finally {
  rootNode.free();
  grammar.free();
}
try {
  api.contractSchema("not-a-selected-schema");
  throw new Error("invalid schema must fail");
} catch (error: unknown) {
  assert(error instanceof Error, "an API failure is a native Error");
  assert("authoringCorrections" in error && Array.isArray(error.authoringCorrections),
    "compiler corrections are supplied separately from diagnostic bytes");
  for (const member of error.authoringCorrections) {
    const correction: unknown = member;
    assert(correction instanceof AuthoringCorrection, "each correction is a native typed value");
    const kind: AuthoringCorrectionKind = correction.kind();
    const expected: string = correction.expectedSource;
    const replacement: string = correction.replacementSource;
    const identity: string = correction.diagnosticIdentity;
    // @ts-expect-error The expected source is text, not a numeric coordinate.
    const invalid: number = correction.expectedSource;
    void invalid;
    assert(kind === AuthoringCorrectionKind.NarrowImportVisibility && expected !== replacement && identity.length > 0,
      "correction fields retain their types after runtime narrowing");
    correction.free();
  }
  assert("authoringLocation" in error, "location absence is explicit");
  if (error.authoringLocation !== undefined) {
    assert(error.authoringLocation instanceof AuthoringSourceLocation, "location is a native typed value");
    error.authoringLocation.free();
  }
}
function cannotConstructCompilerEvidence(): void {
  // @ts-expect-error Corrections are produced by the compiler, not public constructors.
  new AuthoringCorrection();
}
void cannotConstructCompilerEvidence;
const identity = api.contractBundleIdentity();
try {
  assert(SCHEMA_REGISTRY_ID === contractRegistry, "registry export must retain its owner");
  assert(SCHEMA_MANIFEST_SHA256 === contractManifest, "manifest export must retain its owner");
  assert(SCHEMA_SET_SHA256 === contractSet, "set export must retain its owner");
  assert(identity.schemaManifestSha256 === contractManifest, "embedded and package manifests differ");
  assert(identity.schemaSetSha256 === contractSet, "embedded and package contract sets differ");

  const unknownDiagnostic: unknown = JSON.parse(fixture.canonical_diagnostic);
  const checked = validate("diagnostic.v1", unknownDiagnostic);
  assert(checked.valid, "the canonical contract owner must admit the fixture");
  const diagnostic: ContractByFamily["diagnostic.v1"] = checked.value;
  assert(!validate("diagnostic.v1", { ...diagnostic, unexpected: true }).valid,
    "unknown members must remain rejected by the canonical validator");
  const bytes = new TextEncoder().encode(fixture.canonical_diagnostic);
  assert(new TextDecoder().decode(api.admitDiagnostic(bytes)) === fixture.canonical_diagnostic,
    "the WASM façade must preserve exact admitted diagnostic bytes");
  const document = api.validateContractDocument("diagnostic.v1", bytes);
  try {
    assert(new TextDecoder().decode(document.canonicalBytes) === fixture.canonical_diagnostic,
      "structural admission must preserve the same canonical instance");
  } finally {
    document.free();
  }
  console.log(JSON.stringify({ registry: contractRegistry, manifest: contractManifest, set: contractSet }, null, 2));
} finally {
  identity.free();
  api.free();
}
