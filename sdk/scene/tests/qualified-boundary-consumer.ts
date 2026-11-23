import type {
  PortableWorkspaceCompilation,
  RecipientQualifiedWorkspaceComparisonInspection,
  RecipientQualifiedWorkspaceInspection,
  WorkspaceComparison,
} from "@rtepl/pale";
import { OrganizationIntake } from "@rtepl/pale-scene/organization";
import { WorkspaceChangeIntake } from "@rtepl/pale-scene/change";

declare const inspection: RecipientQualifiedWorkspaceInspection;
declare const priorInspection: RecipientQualifiedWorkspaceInspection;
declare const currentInspection: RecipientQualifiedWorkspaceInspection;
declare const comparisonInspection: RecipientQualifiedWorkspaceComparisonInspection;
declare const rawWorkspace: PortableWorkspaceCompilation;
declare const rawComparison: WorkspaceComparison;

const organizationQuery = { inspected: 8, returned: 8, copiedStringBytes: 65_536 };
const organizationLimits = {
  queries: 16,
  recordVisits: 128,
  sourceVisits: 128,
  records: 64,
  occurrences: 64,
  sourceOccurrences: 64,
  copiedStringBytes: 1_048_576,
};
const changeQuery = {
  subjects: 8,
  roots: 8,
  impacts: 8,
  contributingRoots: 8,
  dependencies: 8,
  sourceCandidates: 8,
  sourceOccurrences: 8,
};
const changeLimits = {
  queries: 16,
  subjectVisits: 128,
  rootVisits: 128,
  proofVisits: 128,
  sourceVisits: 128,
  subjects: 64,
  roots: 64,
  impacts: 64,
  proofSteps: 128,
  sourceOccurrences: 128,
  copiedStringBytes: 1_048_576,
};

void (() => {
  const exactCustody: boolean = comparisonInspection.comparesExactInspections(
    priorInspection,
    currentInspection,
  );
  void exactCustody;
  new OrganizationIntake(inspection, organizationQuery, organizationLimits).close();
  new WorkspaceChangeIntake(comparisonInspection, changeQuery, changeLimits).close();

  // @ts-expect-error Raw compilations are not recipient-qualified inspection owners.
  comparisonInspection.comparesExactInspections(rawWorkspace, inspection);
  // @ts-expect-error Raw compilations do not carry recipient admission.
  new OrganizationIntake(rawWorkspace, organizationQuery, organizationLimits);
  // @ts-expect-error Raw comparisons do not carry recipient admission.
  new WorkspaceChangeIntake(rawComparison, changeQuery, changeLimits);
});
