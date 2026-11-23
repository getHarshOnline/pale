import { Pale, PortableWorkspaceCompilation, PortableWorkspaceDependencyPage,
  PortableWorkspaceSubjectInspection, WorkspaceComparisonSubjectPage,
  WorkspaceContributingRootsPage, WorkspaceImpactPage } from "@rtepl/pale";
import { WorkspaceChangeInspectionContinuation, WorkspaceChangeIntake,
  WorkspaceChangeIntakeError, WorkspaceChangeOccurrence, WorkspaceChangeIntakeCharges,
  WorkspaceChangeIntakeErrorDetails, WorkspaceChangeIntakeResource,
  WorkspaceChangeSubjectStep, WorkspaceChangeRootStep, WorkspaceChangeImpactStep,
  WorkspaceSubjectComparisonStep, WorkspaceContributingRootsStep, WorkspaceDependencyStep,
  WorkspaceSubjectInspectionStep } from "@rtepl/pale-scene/change";
import { readFileSync } from "node:fs";
import { join } from "node:path";

function compile(api: Pale, fixture: string, changed: boolean): PortableWorkspaceCompilation {
  const files = ["pale-workspace.toml", "organization.toml", "responsibility.toml",
    "resolution.lock.toml", "universe.lock.toml"];
  return api.compilePortableWorkspace({ rootManifest: files[0]!, documents: files.map(label => {
    let bytes = readFileSync(join(fixture, label));
    if (changed && label === "responsibility.toml") {
      bytes = Buffer.from(bytes.toString().replaceAll("evidence.alpha", "evidence.current"));
    }
    return { label, bytes };
  }) });
}

function main(): void {
  const fixture = process.argv[2];
  if (!fixture) throw new Error("Supply the SDK organization fixture");
  const api = new Pale(), prior = compile(api, fixture, false), current = compile(api, fixture, true);
  const boundsDetails: WorkspaceChangeIntakeErrorDetails =
    { kind: "bounds", resource: "proofSteps", observed: 3, limit: 2 };
  const boundsError = new WorkspaceChangeIntakeError(boundsDetails);
  if (boundsError.kind !== "bounds" || boundsError.resource !== "proofSteps"
    || boundsError.observed !== 3 || boundsError.limit !== 2) {
    throw new Error("Workspace change bounds details were lost");
  }
  const comparison = api.comparePortableWorkspaces(prior, current);
  const comparisonReceiptDigest = comparison.receiptDigest;
  const intake = new WorkspaceChangeIntake(prior, current, comparison,
    { subjects: 2, roots: 2, impacts: 2, contributingRoots: 2, dependencies: 2,
      sourceCandidates: 2, sourceOccurrences: 2 },
    { queries: 100, subjectVisits: 1000, rootVisits: 1000, proofVisits: 1000,
      sourceVisits: 1000, subjects: 100, roots: 100, impacts: 100, proofSteps: 1000,
      sourceOccurrences: 1000, copiedStringBytes: 10000 });
  try {
    const resource: WorkspaceChangeIntakeResource = "proofSteps";
    const charges: WorkspaceChangeIntakeCharges = intake.charges;
    if (charges[resource] !== 0) throw new Error("New intake has proof charges");
    const subjects: WorkspaceChangeSubjectStep = intake.nextSubjects();
    const roots: WorkspaceChangeRootStep = intake.nextRoots();
    const impacts: WorkspaceChangeImpactStep = intake.nextImpacts();
    if (subjects.kind !== "page" || !(subjects.page instanceof WorkspaceComparisonSubjectPage)) {
      throw new Error("Subject page unavailable");
    }
    if (roots.kind !== "page" || roots.occurrences.some(item => item.inclusion !== "change-root")) {
      throw new Error("Change-root appearances unavailable");
    }
    if (impacts.kind !== "page" || !(impacts.page instanceof WorkspaceImpactPage)) {
      throw new Error("Impact page unavailable");
    }
    const occurrence: WorkspaceChangeOccurrence | undefined = subjects.occurrences[0];
    if (!occurrence || occurrence.inclusion !== "subject-enumeration") throw new Error("Subject occurrence unavailable");
    const point: WorkspaceSubjectComparisonStep = intake.compare(occurrence);
    const contributing: WorkspaceContributingRootsStep = intake.contributingRoots(occurrence);
    const dependencies: WorkspaceDependencyStep = intake.dependencies("current", occurrence);
    const inspected: WorkspaceSubjectInspectionStep = intake.inspect("current", occurrence);
    if (point.kind !== "result" || point.result.receiptDigest !== comparisonReceiptDigest) {
      throw new Error("Point comparison unavailable");
    }
    if (contributing.kind !== "page" || !(contributing.page instanceof WorkspaceContributingRootsPage)) {
      throw new Error("Contributing roots unavailable");
    }
    if (dependencies.kind !== "page" || !(dependencies.page instanceof PortableWorkspaceDependencyPage)) {
      throw new Error("Dependencies unavailable");
    }
    if (inspected.kind !== "page" || !(inspected.page instanceof PortableWorkspaceSubjectInspection)
      || !inspected.page.belongsTo(current)) {
      throw new Error("Source inspection unavailable");
    }
    const inspectionCursor: WorkspaceChangeInspectionContinuation | null = inspected.continuation;
    if (inspectionCursor !== null) {
      const continued: WorkspaceSubjectInspectionStep = intake.inspect("current", occurrence, inspectionCursor);
      if (continued.kind !== "page" || !continued.page.belongsTo(current)) {
        throw new Error("Source inspection continuation unavailable");
      }
    }
    void (() => {
      // @ts-expect-error An occurrence cannot be constructed outside its intake.
      new WorkspaceChangeOccurrence();
      // @ts-expect-error An inspection continuation cannot be constructed outside its intake.
      new WorkspaceChangeInspectionContinuation();
      // @ts-expect-error Dependency sides form a closed union.
      intake.dependencies("next", occurrence);
      // @ts-expect-error Inspection sides form a closed union.
      intake.inspect("next", occurrence);
      // @ts-expect-error A native SDK cursor cannot bypass Scene association.
      intake.inspect("current", occurrence, inspected.page.continuation());
      // @ts-expect-error A textual appearance ID is not an occurrence.
      intake.compare(occurrence.appearanceId);
      // @ts-expect-error Occurrence fields are read-only.
      occurrence.inclusion = "impact-partition";
    });
    console.log(JSON.stringify({ subjects: subjects.occurrences.length, roots: roots.occurrences.length,
      impacts: impacts.occurrences.length, point: point.kind, inspected: inspected.kind }));
  } finally { intake.close(); }
}
main();
