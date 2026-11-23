import { OrganizationComparisonSide, RecipientQualifiedComparisonSubjectPage,
  RecipientQualifiedContributingRootsPage, RecipientQualifiedDependencyPage,
  RecipientQualifiedSubjectInspection, RecipientQualifiedWorkspaceComparisonInspection,
  WorkspaceChangeRoot, WorkspaceChangeRootPage, WorkspaceImpact, WorkspaceImpactPage, WorkspaceSubject,
  WorkspaceSubjectComparison } from "@rtepl/pale";
import type { RecipientQualifiedComparisonSubjectCursor,
  RecipientQualifiedContributingRootsCursor, RecipientQualifiedDependencyCursor,
  RecipientQualifiedSubjectInspectionCursor } from "@rtepl/pale";
import { chargeRetainedSceneBytes, chargeSceneWork, defaultSceneBudget, type SceneBudget } from "./sceneBudget";

// Keep semantic calls on their native SDK owners. Later instance or prototype
// shadowing cannot replace the recipient-qualified behavior used by this package.
const comparisonSubjects = RecipientQualifiedWorkspaceComparisonInspection.prototype.subjects;
const comparisonRoots = RecipientQualifiedWorkspaceComparisonInspection.prototype.changeRoots;
const comparisonImpacts = RecipientQualifiedWorkspaceComparisonInspection.prototype.impacts;
const comparisonSubject = RecipientQualifiedWorkspaceComparisonInspection.prototype.compareSubject;
const comparisonContributingRoots = RecipientQualifiedWorkspaceComparisonInspection.prototype.contributingRoots;
const directDependencies = RecipientQualifiedWorkspaceComparisonInspection.prototype.directDependencies;
const inspectSubject = RecipientQualifiedWorkspaceComparisonInspection.prototype.inspectSubject;
const subjectPageSubjects = RecipientQualifiedComparisonSubjectPage.prototype.subjects;
const subjectPageContinuation = RecipientQualifiedComparisonSubjectPage.prototype.continuation;
const subjectPageInspected = Object.getOwnPropertyDescriptor(
  RecipientQualifiedComparisonSubjectPage.prototype, "inspected")!.get!;
const subjectPageIdentityBytes = Object.getOwnPropertyDescriptor(
  RecipientQualifiedComparisonSubjectPage.prototype, "subjectIdentityBytes")!.get!;
const rootSubject = WorkspaceChangeRoot.prototype.subject;
const rootPageCanonicalBytes = Object.getOwnPropertyDescriptor(
  WorkspaceChangeRootPage.prototype, "canonicalBytes")!.get!;
const impactSubject = WorkspaceImpact.prototype.subject;
const impactEntries = Object.getOwnPropertyDescriptor(WorkspaceImpactPage.prototype, "entries")!.get!;
const impactNextOffset = Object.getOwnPropertyDescriptor(WorkspaceImpactPage.prototype, "nextOffset")!.get!;
const impactPageCanonicalBytes = Object.getOwnPropertyDescriptor(
  WorkspaceImpactPage.prototype, "canonicalBytes")!.get!;
const pageRoots = RecipientQualifiedContributingRootsPage.prototype.roots;
const dependencySteps = RecipientQualifiedDependencyPage.prototype.dependencies;
const dependencyCopiedStringBytes = Object.getOwnPropertyDescriptor(
  RecipientQualifiedDependencyPage.prototype, "copiedStringBytes")!.get!;
const inspectionDependencies = RecipientQualifiedSubjectInspection.prototype.dependencies;
const inspectionOccurrences = RecipientQualifiedSubjectInspection.prototype.occurrences;
const inspectionContinuation = RecipientQualifiedSubjectInspection.prototype.continuation;
const inspectionInspected = Object.getOwnPropertyDescriptor(
  RecipientQualifiedSubjectInspection.prototype, "inspected")!.get!;
const inspectionDependencyBytes = Object.getOwnPropertyDescriptor(
  RecipientQualifiedSubjectInspection.prototype, "dependencyCopiedStringBytes")!.get!;
const inspectionSourceBytes = Object.getOwnPropertyDescriptor(
  RecipientQualifiedSubjectInspection.prototype, "sourceCopiedStringBytes")!.get!;

/** How an occurrence entered this intake. */
export type WorkspaceChangeInclusion = "subject-enumeration" | "change-root" | "impact-partition";

/** Raw page sizes passed to the matching native SDK calls. */
export interface WorkspaceChangeQueryLimits {
  readonly subjects: number;
  readonly roots: number;
  readonly impacts: number;
  readonly contributingRoots: number;
  /** Direct proof inputs returned by either proof query. */
  readonly dependencies: number;
  /** Source candidates examined by one full subject read. */
  readonly sourceCandidates: number;
  /** Named source occurrences returned by one full subject read. */
  readonly sourceOccurrences: number;
}

/** Aggregate work and retained-output limits for one intake. */
export interface WorkspaceChangeIntakeLimits {
  readonly queries: number;
  readonly subjectVisits: number;
  readonly rootVisits: number;
  readonly proofVisits: number;
  readonly sourceVisits: number;
  readonly subjects: number;
  readonly roots: number;
  readonly impacts: number;
  readonly proofSteps: number;
  readonly sourceOccurrences: number;
  /** UTF-8 bytes retained in local IDs and published SDK source/proof pages. */
  readonly copiedStringBytes: number;
}

/** One independently limited intake resource. */
export type WorkspaceChangeIntakeResource = keyof WorkspaceChangeIntakeLimits;

/** Aggregate charges, including reservations kept after native failures. */
export interface WorkspaceChangeIntakeCharges {
  readonly queries: number;
  readonly subjectVisits: number;
  readonly rootVisits: number;
  readonly proofVisits: number;
  readonly sourceVisits: number;
  readonly subjects: number;
  readonly roots: number;
  readonly impacts: number;
  readonly proofSteps: number;
  readonly sourceOccurrences: number;
  readonly copiedStringBytes: number;
}

/**
 * A step stopped at one aggregate limit.
 *
 * Row reservations stop before a native call. Full inspection can measure
 * copied bytes only after its pure native read; an oversized page is not
 * published, and the query and row reservations remain charged.
 */
export interface WorkspaceChangeResourceStoppedStep<
  Resource extends WorkspaceChangeIntakeResource = WorkspaceChangeIntakeResource,
> {
  readonly kind: "resource-stop";
  readonly resource: Resource;
}

/** The caller cancelled before the next synchronous SDK call started. */
export interface WorkspaceChangeStoppedStep {
  readonly kind: "stopped";
  readonly reason: "cancelled";
}

const stoppedStep: WorkspaceChangeStoppedStep = Object.freeze({ kind: "stopped", reason: "cancelled" });

interface OccurrenceState {
  readonly appearanceId: string;
  readonly subject: WorkspaceSubject;
  readonly inclusion: WorkspaceChangeInclusion;
}
const occurrenceStates = new WeakMap<WorkspaceChangeOccurrence, OccurrenceState>();

/** One intake-local appearance of an SDK-owned semantic subject. */
export class WorkspaceChangeOccurrence {
  private constructor() { throw new TypeError("Workspace change occurrences are issued by an intake"); }

  /** Local presentation identity. It has no meaning outside the issuing intake. */
  get appearanceId(): string { return occurrenceState(this).appearanceId; }

  /** The original SDK subject retained for this appearance. */
  get subject(): WorkspaceSubject { return occurrenceState(this).subject; }

  /** The bounded native page that introduced this appearance. */
  get inclusion(): WorkspaceChangeInclusion { return occurrenceState(this).inclusion; }
}

function occurrenceState(value: WorkspaceChangeOccurrence): OccurrenceState {
  const state = occurrenceStates.get(value);
  if (!state) throw new TypeError("Invalid workspace change occurrence");
  return state;
}

function issueOccurrence(state: OccurrenceState): WorkspaceChangeOccurrence {
  const occurrence = Object.create(WorkspaceChangeOccurrence.prototype) as WorkspaceChangeOccurrence;
  occurrenceStates.set(occurrence, state);
  return Object.freeze(occurrence);
}

type InspectionSide = "prior" | "current";
interface InspectionContinuationState {
  readonly intake: WorkspaceChangeIntake;
  readonly occurrence: WorkspaceChangeOccurrence;
  readonly side: InspectionSide;
  readonly native: RecipientQualifiedSubjectInspectionCursor;
}
const inspectionContinuationStates = new WeakMap<
  WorkspaceChangeInspectionContinuation, InspectionContinuationState>();

/** Opaque position for one intake, occurrence appearance, and comparison side. */
export class WorkspaceChangeInspectionContinuation {
  private readonly __sceneInspectionContinuation: void = undefined;

  private constructor() {
    void this.__sceneInspectionContinuation;
    throw new TypeError("Workspace change inspection continuations are issued by an intake");
  }
}

function issueInspectionContinuation(
  state: InspectionContinuationState,
): WorkspaceChangeInspectionContinuation {
  const continuation = Object.create(
    WorkspaceChangeInspectionContinuation.prototype) as WorkspaceChangeInspectionContinuation;
  inspectionContinuationStates.set(continuation, state);
  return Object.freeze(continuation) as WorkspaceChangeInspectionContinuation;
}

type PagePosition<Cursor> = { readonly kind: "initial" }
  | { readonly kind: "continuing"; readonly cursor: Cursor }
  | { readonly kind: "exhausted" };
type OffsetPosition = { readonly kind: "offset"; readonly value: number }
  | { readonly kind: "exhausted" };
interface OpenState {
  readonly kind: "open";
  readonly comparison: RecipientQualifiedWorkspaceComparisonInspection;
  subjects: PagePosition<RecipientQualifiedComparisonSubjectCursor>;
  roots: OffsetPosition;
  impacts: OffsetPosition;
}
type State = OpenState | { readonly kind: "closed" };

type IntakeStop = WorkspaceChangeResourceStoppedStep | WorkspaceChangeStoppedStep | { readonly kind: "closed" };
/** One page or terminal state from the independent subject enumeration. */
export type WorkspaceChangeSubjectStep =
  | { readonly kind: "page"; readonly page: RecipientQualifiedComparisonSubjectPage;
    readonly occurrences: readonly WorkspaceChangeOccurrence[] }
  | { readonly kind: "enumeration-exhausted" } | IntakeStop;
/** One page or terminal state from the independent change-root enumeration. */
export type WorkspaceChangeRootStep =
  | { readonly kind: "page"; readonly page: WorkspaceChangeRootPage;
    readonly occurrences: readonly WorkspaceChangeOccurrence[] }
  | { readonly kind: "enumeration-exhausted" } | IntakeStop;
/** One page or terminal state from the independent impact enumeration. */
export type WorkspaceChangeImpactStep =
  | { readonly kind: "page"; readonly page: WorkspaceImpactPage;
    readonly occurrences: readonly WorkspaceChangeOccurrence[] }
  | { readonly kind: "enumeration-exhausted" } | IntakeStop;
/** One exact subject comparison or an explicit intake stop. */
export type WorkspaceSubjectComparisonStep =
  | { readonly kind: "result"; readonly result: WorkspaceSubjectComparison } | IntakeStop;
/** One page of contributing roots or an explicit intake stop. */
export type WorkspaceContributingRootsStep =
  | { readonly kind: "page"; readonly page: RecipientQualifiedContributingRootsPage } | IntakeStop;
/** One page of direct proof inputs or an explicit intake stop. */
export type WorkspaceDependencyStep =
  | { readonly kind: "page"; readonly page: RecipientQualifiedDependencyPage } | IntakeStop;
/** One source-and-proof page with its exact continuation, or an explicit intake stop. */
export type WorkspaceSubjectInspectionStep =
  | { readonly kind: "page"; readonly page: RecipientQualifiedSubjectInspection;
    readonly continuation: WorkspaceChangeInspectionContinuation | null }
  | IntakeStop;

/** Closed custody failures. These do not describe resource exhaustion. */
export type WorkspaceChangeIntakeCustodyError =
  | { readonly kind: "foreign-occurrence" }
  | { readonly kind: "owner-mismatch" }
  | { readonly kind: "continuation-mismatch" };

/** A native result exceeded the maximum reserved for one typed resource. */
export interface WorkspaceChangeIntakeBoundsError {
  readonly kind: "bounds";
  readonly resource: WorkspaceChangeIntakeResource;
  readonly observed: number;
  readonly limit: number;
}

/** Structured reason for rejecting a workspace-change intake operation. */
export type WorkspaceChangeIntakeErrorDetails =
  | WorkspaceChangeIntakeCustodyError
  | WorkspaceChangeIntakeBoundsError;

/** A custody mismatch or a native result that exceeded its declared bound. */
export class WorkspaceChangeIntakeError extends Error {
  readonly details: Readonly<WorkspaceChangeIntakeErrorDetails>;

  constructor(details: WorkspaceChangeIntakeErrorDetails) {
    const captured = Object.freeze({ ...details });
    super(captured.kind === "bounds"
      ? `Workspace change intake exceeded ${captured.resource}: observed ${captured.observed}, limit ${captured.limit}`
      : `Workspace change intake rejected custody: ${captured.kind}`);
    this.name = "WorkspaceChangeIntakeError";
    this.details = captured;
  }

  /** Stable closed category. Branch on this value, not the message. */
  get kind(): WorkspaceChangeIntakeErrorDetails["kind"] { return this.details.kind; }

  /** Resource whose result exceeded its reservation, or no value for custody failures. */
  get resource(): WorkspaceChangeIntakeResource | undefined {
    return this.details.kind === "bounds" ? this.details.resource : undefined;
  }

  /** Native count that was rejected, or no value for custody failures. */
  get observed(): number | undefined {
    return this.details.kind === "bounds" ? this.details.observed : undefined;
  }

  /** Maximum count reserved for the operation, or no value for custody failures. */
  get limit(): number | undefined {
    return this.details.kind === "bounds" ? this.details.limit : undefined;
  }
}

function rejectBounds(resource: WorkspaceChangeIntakeResource, observed: number, limit: number): void {
  if (observed > limit) {
    throw new WorkspaceChangeIntakeError({ kind: "bounds", resource, observed, limit });
  }
}

/**
 * Retain one recipient-qualified workspace comparison for bounded Scene reads.
 * Native pages and handles pass through unchanged. Appearance IDs are local.
 * Each call returns to the comparison owner's authority check before reading.
 * A supplied SceneBudget also charges this intake's work and Scene-owned copies.
 */
export class WorkspaceChangeIntake {
  #state: State;
  readonly #query: WorkspaceChangeQueryLimits;
  readonly #limits: WorkspaceChangeIntakeLimits;
  readonly #used: Record<WorkspaceChangeIntakeResource, number> = {
    queries: 0, subjectVisits: 0, rootVisits: 0, proofVisits: 0, sourceVisits: 0,
    subjects: 0, roots: 0, impacts: 0, proofSteps: 0, sourceOccurrences: 0,
    copiedStringBytes: 0,
  };
  readonly #occurrences = new Set<WorkspaceChangeOccurrence>();
  readonly #budget: SceneBudget;
  #nextAppearance = 0;

  constructor(comparison: RecipientQualifiedWorkspaceComparisonInspection,
    query: WorkspaceChangeQueryLimits,
    limits: WorkspaceChangeIntakeLimits, budget?: SceneBudget) {
    if (!(comparison instanceof RecipientQualifiedWorkspaceComparisonInspection)) {
      throw new TypeError("Workspace change intake requires a recipient-qualified comparison");
    }
    const querySnapshot = { subjects: query.subjects, roots: query.roots, impacts: query.impacts,
      contributingRoots: query.contributingRoots, dependencies: query.dependencies,
      sourceCandidates: query.sourceCandidates, sourceOccurrences: query.sourceOccurrences };
    const limitSnapshot = { queries: limits.queries, subjectVisits: limits.subjectVisits,
      rootVisits: limits.rootVisits, proofVisits: limits.proofVisits,
      sourceVisits: limits.sourceVisits, subjects: limits.subjects, roots: limits.roots,
      impacts: limits.impacts, proofSteps: limits.proofSteps,
      sourceOccurrences: limits.sourceOccurrences, copiedStringBytes: limits.copiedStringBytes };
    for (const value of Object.values(querySnapshot)) {
      if (!Number.isSafeInteger(value) || value < 1 || value > 65535) {
        throw new RangeError("Workspace change query limits must be integers from 1 through 65535");
      }
    }
    for (const value of Object.values(limitSnapshot)) {
      if (!Number.isSafeInteger(value) || value < 1) {
        throw new RangeError("Workspace change intake limits must be positive safe integers");
      }
    }
    this.#query = querySnapshot;
    this.#limits = limitSnapshot;
    this.#budget = budget ?? defaultSceneBudget();
    this.#state = { kind: "open", comparison,
      subjects: { kind: "initial" }, roots: { kind: "offset", value: 0 },
      impacts: { kind: "offset", value: 0 } };
  }

  /** Current charges. Failed native calls retain their full reservation. */
  get charges(): WorkspaceChangeIntakeCharges { return { ...this.#used }; }

  /** Advance the independent comparison-subject enumeration by one page. */
  nextSubjects(signal?: AbortSignal): WorkspaceChangeSubjectStep {
    if (this.#state.kind === "closed") return { kind: "closed" };
    if (this.#state.subjects.kind === "exhausted") return { kind: "enumeration-exhausted" };
    if (signal?.aborted) return stoppedStep;
    const state = this.#state;
    const copiedStringBytes = this.#appearanceReservation(this.#query.subjects);
    // The ordered merge can read the page limit plus one look-ahead from each side.
    const reservation = { queries: 1, subjectVisits: this.#query.subjects * 2 + 2,
      subjects: this.#query.subjects, copiedStringBytes };
    const shortage = this.#reserve(reservation);
    if (shortage) return { kind: "resource-stop", resource: shortage };
    chargeSceneWork(this.#budget, 1 + reservation.subjectVisits);
    const page = comparisonSubjects.call(state.comparison, this.#query.subjects,
      state.subjects.kind === "continuing" ? state.subjects.cursor : undefined);
    const subjects = subjectPageSubjects.call(page);
    const inspected = subjectPageInspected.call(page) as number;
    const subjectIdentityBytes = subjectPageIdentityBytes.call(page) as number;
    rejectBounds("subjectVisits", inspected, reservation.subjectVisits);
    rejectBounds("subjects", subjects.length, reservation.subjects);
    const identityShortage = this.#reserve({ copiedStringBytes: subjectIdentityBytes });
    if (identityShortage) return { kind: "resource-stop", resource: identityShortage };
    const continuation = subjectPageContinuation.call(page);
    chargeRetainedSceneBytes(this.#budget,
      subjectIdentityBytes + this.#appearanceReservation(subjects.length));
    const occurrences = Object.freeze(subjects.map(subject => this.#include(subject, "subject-enumeration")));
    this.#refund({ subjectVisits: reservation.subjectVisits - inspected,
      subjects: reservation.subjects - subjects.length,
      copiedStringBytes: copiedStringBytes - byteLengthOfAppearances(occurrences) });
    state.subjects = continuation === null ? { kind: "exhausted" }
      : { kind: "continuing", cursor: continuation };
    return { kind: "page", page, occurrences };
  }

  /** Advance the independent change-root enumeration by one page. */
  nextRoots(signal?: AbortSignal): WorkspaceChangeRootStep {
    if (this.#state.kind === "closed") return { kind: "closed" };
    const position = this.#state.roots;
    if (position.kind === "exhausted") return { kind: "enumeration-exhausted" };
    if (signal?.aborted) return stoppedStep;
    const state = this.#state, offset = position.value;
    const copiedStringBytes = this.#appearanceReservation(this.#query.roots);
    const reservation = { queries: 1, rootVisits: this.#query.roots,
      roots: this.#query.roots, copiedStringBytes };
    const shortage = this.#reserve(reservation);
    if (shortage) return { kind: "resource-stop", resource: shortage };
    chargeSceneWork(this.#budget, 1 + reservation.rootVisits);
    const page = comparisonRoots.call(state.comparison, offset, this.#query.roots);
    const entries = page.entries;
    const pageBytes = (rootPageCanonicalBytes.call(page) as Buffer).byteLength;
    rejectBounds("roots", entries.length, reservation.roots);
    const byteShortage = this.#reserve({ copiedStringBytes: pageBytes });
    if (byteShortage) return { kind: "resource-stop", resource: byteShortage };
    const next = page.nextOffset;
    chargeRetainedSceneBytes(this.#budget,
      pageBytes + this.#appearanceReservation(entries.length));
    const occurrences = Object.freeze(entries.map(root => this.#include(rootSubject.call(root), "change-root")));
    this.#refund({ rootVisits: reservation.rootVisits - entries.length,
      roots: reservation.roots - entries.length,
      copiedStringBytes: copiedStringBytes - byteLengthOfAppearances(occurrences) });
    state.roots = next === undefined ? { kind: "exhausted" } : { kind: "offset", value: next };
    return { kind: "page", page, occurrences };
  }

  /** Advance the independent current-impact partition by one page. */
  nextImpacts(signal?: AbortSignal): WorkspaceChangeImpactStep {
    if (this.#state.kind === "closed") return { kind: "closed" };
    const position = this.#state.impacts;
    if (position.kind === "exhausted") return { kind: "enumeration-exhausted" };
    if (signal?.aborted) return stoppedStep;
    const state = this.#state, offset = position.value;
    const copiedStringBytes = this.#appearanceReservation(this.#query.impacts);
    const reservation = { queries: 1, subjectVisits: this.#query.impacts,
      impacts: this.#query.impacts, copiedStringBytes };
    const shortage = this.#reserve(reservation);
    if (shortage) return { kind: "resource-stop", resource: shortage };
    chargeSceneWork(this.#budget, 1 + reservation.subjectVisits);
    const page = comparisonImpacts.call(state.comparison, offset, this.#query.impacts);
    const entries = impactEntries.call(page) as WorkspaceImpact[];
    const pageBytes = (impactPageCanonicalBytes.call(page) as Buffer).byteLength;
    rejectBounds("impacts", entries.length, reservation.impacts);
    const byteShortage = this.#reserve({ copiedStringBytes: pageBytes });
    if (byteShortage) return { kind: "resource-stop", resource: byteShortage };
    const next = impactNextOffset.call(page) as number | undefined;
    chargeRetainedSceneBytes(this.#budget,
      pageBytes + this.#appearanceReservation(entries.length));
    const occurrences = Object.freeze(entries.map(impact => this.#include(impactSubject.call(impact), "impact-partition")));
    this.#refund({ subjectVisits: reservation.subjectVisits - entries.length,
      impacts: reservation.impacts - entries.length,
      copiedStringBytes: copiedStringBytes - byteLengthOfAppearances(occurrences) });
    state.impacts = next === undefined ? { kind: "exhausted" } : { kind: "offset", value: next };
    return { kind: "page", page, occurrences };
  }

  /** Compare the exact subject carried by an occurrence from this intake. */
  compare(occurrence: WorkspaceChangeOccurrence, signal?: AbortSignal): WorkspaceSubjectComparisonStep {
    if (this.#state.kind === "closed") return { kind: "closed" };
    if (signal?.aborted) return stoppedStep;
    const state = this.#state, subject = this.#subject(occurrence);
    const shortage = this.#reserve({ queries: 1, subjectVisits: 1 });
    if (shortage) return { kind: "resource-stop", resource: shortage };
    chargeSceneWork(this.#budget, 2);
    return { kind: "result", result: comparisonSubject.call(state.comparison, subject) };
  }

  /** Read contributing roots for an exact current subject occurrence. */
  contributingRoots(occurrence: WorkspaceChangeOccurrence,
    after?: RecipientQualifiedContributingRootsCursor | null,
    signal?: AbortSignal): WorkspaceContributingRootsStep {
    if (this.#state.kind === "closed") return { kind: "closed" };
    if (signal?.aborted) return stoppedStep;
    const state = this.#state, subject = this.#subject(occurrence);
    const reservation = { queries: 1, rootVisits: this.#query.contributingRoots,
      roots: this.#query.contributingRoots };
    const shortage = this.#reserve(reservation);
    if (shortage) return { kind: "resource-stop", resource: shortage };
    chargeSceneWork(this.#budget, 1 + reservation.rootVisits);
    const page = comparisonContributingRoots.call(state.comparison, subject,
      this.#query.contributingRoots, after);
    const roots = pageRoots.call(page);
    rejectBounds("roots", roots.length, reservation.roots);
    this.#refund({ rootVisits: reservation.rootVisits - roots.length,
      roots: reservation.roots - roots.length });
    return { kind: "page", page };
  }

  /** Read direct proof inputs from the selected side of the retained pair. */
  dependencies(side: "prior" | "current", occurrence: WorkspaceChangeOccurrence,
    after?: RecipientQualifiedDependencyCursor | null,
    signal?: AbortSignal): WorkspaceDependencyStep {
    if (this.#state.kind === "closed") return { kind: "closed" };
    if (signal?.aborted) return stoppedStep;
    if (side !== "prior" && side !== "current") {
      throw new RangeError("Workspace dependency side must be prior or current");
    }
    const state = this.#state, subject = this.#subject(occurrence);
    const reservation = { queries: 1, proofVisits: this.#query.dependencies,
      proofSteps: this.#query.dependencies };
    const shortage = this.#reserve(reservation);
    if (shortage) return { kind: "resource-stop", resource: shortage };
    chargeSceneWork(this.#budget, 1 + reservation.proofVisits);
    const page = directDependencies.call(state.comparison,
      side === "prior" ? OrganizationComparisonSide.Prior : OrganizationComparisonSide.Current,
      subject, this.#query.dependencies, after);
    const steps = dependencySteps.call(page);
    const copiedStringBytes = dependencyCopiedStringBytes.call(page) as number;
    rejectBounds("proofSteps", steps.length, reservation.proofSteps);
    const byteShortage = this.#reserve({ copiedStringBytes });
    if (byteShortage) return { kind: "resource-stop", resource: byteShortage };
    chargeRetainedSceneBytes(this.#budget, copiedStringBytes);
    this.#refund({ proofVisits: reservation.proofVisits - steps.length,
      proofSteps: reservation.proofSteps - steps.length });
    return { kind: "page", page };
  }

  /**
   * Read the selected side's exact source and direct-proof page.
   *
   * The page remains the native SDK projection. Resume with the continuation
   * returned beside it. That value is bound to this intake, the exact occurrence
   * appearance, and the selected side. The qualified owner rechecks disclosure
   * before every page, including every continuation.
   */
  inspect(side: InspectionSide, occurrence: WorkspaceChangeOccurrence,
    after?: WorkspaceChangeInspectionContinuation | null,
    signal?: AbortSignal): WorkspaceSubjectInspectionStep {
    if (this.#state.kind === "closed") return { kind: "closed" };
    if (signal?.aborted) return stoppedStep;
    if (side !== "prior" && side !== "current") {
      throw new RangeError("Workspace inspection side must be prior or current");
    }
    const state = this.#state, subject = this.#subject(occurrence);
    const nativeAfter = after == null ? undefined
      : this.#inspectionContinuation(side, occurrence, after);
    const reservation = { queries: 1, proofVisits: this.#query.dependencies,
      sourceVisits: this.#query.sourceCandidates, proofSteps: this.#query.dependencies,
      sourceOccurrences: this.#query.sourceOccurrences };
    const shortage = this.#reserve(reservation);
    if (shortage) return { kind: "resource-stop", resource: shortage };
    chargeSceneWork(this.#budget, 1 + reservation.proofVisits + reservation.sourceVisits);
    const page = inspectSubject.call(state.comparison,
      side === "prior" ? OrganizationComparisonSide.Prior : OrganizationComparisonSide.Current,
      subject, this.#query.dependencies, this.#query.sourceCandidates,
      this.#query.sourceOccurrences, nativeAfter);
    const dependencies = inspectionDependencies.call(page);
    const occurrences = inspectionOccurrences.call(page);
    const inspected = inspectionInspected.call(page) as number;
    rejectBounds("proofSteps", dependencies.length, reservation.proofSteps);
    rejectBounds("sourceVisits", inspected, reservation.sourceVisits);
    rejectBounds("sourceOccurrences", occurrences.length, reservation.sourceOccurrences);
    const copiedStringBytes = (inspectionDependencyBytes.call(page) as number)
      + (inspectionSourceBytes.call(page) as number);
    const byteShortage = this.#reserve({ copiedStringBytes });
    if (byteShortage) return { kind: "resource-stop", resource: byteShortage };
    chargeRetainedSceneBytes(this.#budget, copiedStringBytes);
    this.#refund({ proofVisits: reservation.proofVisits - dependencies.length,
      sourceVisits: reservation.sourceVisits - inspected,
      proofSteps: reservation.proofSteps - dependencies.length,
      sourceOccurrences: reservation.sourceOccurrences - occurrences.length });
    const nativeContinuation = inspectionContinuation.call(page);
    const continuation = nativeContinuation === null ? null : issueInspectionContinuation({
      intake: this, occurrence, side, native: nativeContinuation,
    });
    return { kind: "page", page, continuation };
  }

  /** Release retained owners and local membership. Returned SDK handles remain valid. */
  close(): void { this.#state = { kind: "closed" }; this.#occurrences.clear(); }

  #subject(occurrence: WorkspaceChangeOccurrence): WorkspaceSubject {
    if (!this.#occurrences.has(occurrence)) {
      throw new WorkspaceChangeIntakeError({ kind: "foreign-occurrence" });
    }
    return occurrenceState(occurrence).subject;
  }

  #inspectionContinuation(side: InspectionSide, occurrence: WorkspaceChangeOccurrence,
    continuation: WorkspaceChangeInspectionContinuation): RecipientQualifiedSubjectInspectionCursor {
    const state = inspectionContinuationStates.get(continuation);
    if (!state || state.intake !== this || state.occurrence !== occurrence || state.side !== side) {
      throw new WorkspaceChangeIntakeError({ kind: "continuation-mismatch" });
    }
    return state.native;
  }

  #include(subject: WorkspaceSubject, inclusion: WorkspaceChangeInclusion): WorkspaceChangeOccurrence {
    const occurrence = issueOccurrence({ appearanceId: `c${this.#nextAppearance++}`, subject, inclusion });
    this.#occurrences.add(occurrence);
    return occurrence;
  }

  #appearanceReservation(count: number): number {
    let bytes = 0;
    for (let index = 0; index < count; index++) bytes += Buffer.byteLength(`c${this.#nextAppearance + index}`);
    return bytes;
  }

  #reserve(reservation: Partial<Record<WorkspaceChangeIntakeResource, number>>): WorkspaceChangeIntakeResource | undefined {
    for (const resource of Object.keys(reservation) as WorkspaceChangeIntakeResource[]) {
      if (reservation[resource]! > this.#limits[resource] - this.#used[resource]) return resource;
    }
    for (const resource of Object.keys(reservation) as WorkspaceChangeIntakeResource[]) {
      this.#used[resource] += reservation[resource]!;
    }
    return undefined;
  }

  #refund(refund: Partial<Record<WorkspaceChangeIntakeResource, number>>): void {
    for (const resource of Object.keys(refund) as WorkspaceChangeIntakeResource[]) {
      this.#used[resource] -= refund[resource]!;
    }
  }
}

function byteLengthOfAppearances(occurrences: readonly WorkspaceChangeOccurrence[]): number {
  return occurrences.reduce((total, occurrence) =>
    total + Buffer.byteLength(occurrenceState(occurrence).appearanceId), 0);
}
