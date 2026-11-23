import { RecipientQualifiedOrganizationCorrespondence, RecipientQualifiedOrganizationEntry,
  RecipientQualifiedOrganizationPage, RecipientQualifiedOrganizationRecordHandle,
  RecipientQualifiedOrganizationSourcePage, RecipientQualifiedWorkspaceComparisonInspection,
  RecipientQualifiedWorkspaceInspection } from "@rtepl/pale";
import type { OrganizationNode, OrganizationScopeRef, OrganizationComparisonSide,
  RecipientQualifiedOrganizationCorrespondenceCursor, RecipientQualifiedOrganizationCursor,
  RecipientQualifiedOrganizationSourceCursor, RecipientQualifiedOrganizationSourceMapping } from "@rtepl/pale";
import { chargeRetainedSceneBytes, chargeSceneWork, defaultSceneBudget, retainedSceneValueBytes,
  type SceneBudget } from "./sceneBudget";

// Keep semantic reads on the native SDK implementations. Public JavaScript
// properties remain useful to callers, but shadowing them cannot replace Pale's
// query, custody or continuation checks inside this package.
const inspectOrganization = RecipientQualifiedWorkspaceInspection.prototype.inspectOrganization;
const inspectEndpoint = RecipientQualifiedWorkspaceInspection.prototype.inspectOrganizationEndpoint;
const inspectRelations = RecipientQualifiedWorkspaceInspection.prototype.inspectOrganizationIncidentRelations;
const inspectServices = RecipientQualifiedWorkspaceInspection.prototype.inspectOrganizationIncidentServices;
const inspectSources = RecipientQualifiedWorkspaceInspection.prototype.inspectOrganizationSources;
const pageEntries = RecipientQualifiedOrganizationPage.prototype.entries;
const pageContinuation = RecipientQualifiedOrganizationPage.prototype.continuation;
const pageInspected = Object.getOwnPropertyDescriptor(RecipientQualifiedOrganizationPage.prototype, "inspected")!.get!;
const pageCopiedStringBytes = Object.getOwnPropertyDescriptor(
  RecipientQualifiedOrganizationPage.prototype, "copiedStringBytes")!.get!;
const entryHandle = Object.getOwnPropertyDescriptor(
  RecipientQualifiedOrganizationEntry.prototype, "handle")!.get!;
const sameRecord = RecipientQualifiedOrganizationRecordHandle.prototype.sameRecord;
const sourceRecord = Object.getOwnPropertyDescriptor(
  RecipientQualifiedOrganizationSourcePage.prototype, "record")!.get!;
const sourceMapping = Object.getOwnPropertyDescriptor(
  RecipientQualifiedOrganizationSourcePage.prototype, "mapping")!.get!;
const sourceCopiedStringBytes = Object.getOwnPropertyDescriptor(
  RecipientQualifiedOrganizationSourcePage.prototype, "copiedStringBytes")!.get!;
const correspondRecord = RecipientQualifiedWorkspaceComparisonInspection.prototype.correspondOrganizationRecord;
const correspondenceSelectedSide = Object.getOwnPropertyDescriptor(
  RecipientQualifiedOrganizationCorrespondence.prototype, "selectedSide")!.get!;
const correspondenceSelected = Object.getOwnPropertyDescriptor(
  RecipientQualifiedOrganizationCorrespondence.prototype, "selected")!.get!;
const correspondenceInspected = Object.getOwnPropertyDescriptor(
  RecipientQualifiedOrganizationCorrespondence.prototype, "inspected")!.get!;
const correspondenceCopiedStringBytes = Object.getOwnPropertyDescriptor(
  RecipientQualifiedOrganizationCorrespondence.prototype, "copiedStringBytes")!.get!;

/** Fixed SDK query limits. A continuation must use these same values. */
export interface OrganizationQueryLimits {
  readonly inspected: number;
  readonly returned: number;
  readonly copiedStringBytes: number;
}
/** Main-side work and retention limits, separate from geometry and worker limits. */
export interface OrganizationIntakeLimits {
  readonly queries: number;
  readonly recordVisits: number;
  readonly sourceVisits: number;
  readonly records: number;
  readonly occurrences: number;
  readonly sourceOccurrences: number;
  /** SDK query string charges only; not repeated getter copies or total memory. */
  readonly copiedStringBytes: number;
}
/** A counter whose declared limit can stop an intake operation. */
export type OrganizationIntakeResource = "queries" | "recordVisits" | "sourceVisits" | "records"
  | "occurrences" | "sourceOccurrences" | "copiedStringBytes";
/** One intake reservation that exceeded the declared limit for its resource. */
export interface OrganizationResourceStoppedStep<
  Resource extends OrganizationIntakeResource = OrganizationIntakeResource,
> {
  readonly kind: "resource-stop";
  readonly resource: Resource;
}
/** Work and retained values charged to one intake. Failed calls can keep conservative charges. */
export interface OrganizationIntakeCharges {
  readonly queries: number;
  readonly recordVisits: number;
  readonly sourceVisits: number;
  readonly records: number;
  readonly occurrences: number;
  readonly sourceOccurrences: number;
  readonly copiedStringBytes: number;
}
/** Presentation-local reference to one retained SDK organization entry. */
export interface SceneOccurrence {
  /** Local to this intake. Never use this ID to match semantic records across scenes. */
  readonly id: string;
  readonly inclusion: "enumerated-record" | "endpoint-query" | "incident-relation-query" | "incident-service-query" | "additional-occurrence";
}
/** How far the intake's ordered organization enumeration had progressed. */
export type OrganizationEnumerationCoverage =
  | { readonly kind: "incomplete" }
  | { readonly kind: "resource-stopped"; readonly resource: OrganizationIntakeResource }
  | { readonly kind: "exhausted" };
/** Whether the topology selected exactly the occurrences returned by enumeration so far. */
export type OrganizationSelectionCoverage =
  | { readonly kind: "all-enumerated-occurrences" }
  | { readonly kind: "subset" };
const coveragePermit = Symbol("issued organization scene coverage");
let issueCoverage: (enumeration: OrganizationEnumerationCoverage,
  selection: OrganizationSelectionCoverage) => OrganizationSceneCoverage;
let captureCoverage: (intake: OrganizationIntake,
  occurrences: readonly SceneOccurrence[]) => OrganizationSceneCoverage;
let captureBudget: (intake: OrganizationIntake) => SceneBudget;
let captureInspection: (intake: OrganizationIntake) => RecipientQualifiedWorkspaceInspection;

/** Immutable coverage captured when an organization topology is created. */
export class OrganizationSceneCoverage {
  readonly #enumeration: OrganizationEnumerationCoverage;
  readonly #selection: OrganizationSelectionCoverage;
  private constructor(token: symbol, enumeration: OrganizationEnumerationCoverage,
    selection: OrganizationSelectionCoverage) {
    if (token !== coveragePermit) throw new TypeError("Foreign organization scene coverage");
    this.#enumeration = enumeration;
    this.#selection = selection;
    Object.freeze(this);
  }
  static {
    issueCoverage = (enumeration, selection) =>
      new OrganizationSceneCoverage(coveragePermit, enumeration, selection);
  }
  /** State of the independent ordered enumeration at topology construction. */
  get enumeration(): OrganizationEnumerationCoverage { return this.#enumeration; }
  /** Relationship between the selected occurrences and that enumeration frontier. */
  get selection(): OrganizationSelectionCoverage { return this.#selection; }
}
/** The caller cancelled before this intake started another bounded SDK call. */
export interface OrganizationStoppedStep {
  readonly kind: "stopped";
  readonly reason: "cancelled";
}
const cancelledStep: OrganizationStoppedStep = Object.freeze({ kind: "stopped", reason: "cancelled" });
/** One SDK page with retained appearances, or a closed/budget-stopped intake. */
export type OrganizationQueryStep =
  | { readonly kind: "page"; readonly page: RecipientQualifiedOrganizationPage;
    readonly occurrences: readonly SceneOccurrence[] }
  | OrganizationResourceStoppedStep
  | OrganizationStoppedStep
  | { readonly kind: "closed" };
/** One enumeration page, exhaustion, cancellation, resource stop or closed state. */
export type OrganizationStep = OrganizationQueryStep | { readonly kind: "enumeration-exhausted" };
/** Exact SDK source page and its copied mapping, or an explicit intake stop. */
export type SourceStep =
  | { readonly kind: "page"; readonly page: RecipientQualifiedOrganizationSourcePage;
    readonly mapping: RecipientQualifiedOrganizationSourceMapping }
  | OrganizationResourceStoppedStep
  | OrganizationStoppedStep
  | { readonly kind: "closed" };
/** One exact correspondence or an explicit intake stop. */
export type CorrespondenceStep =
  | { readonly kind: "result"; readonly result: RecipientQualifiedOrganizationCorrespondence }
  | OrganizationResourceStoppedStep
  | OrganizationStoppedStep
  | { readonly kind: "closed" };
type Position = { readonly kind: "initial" }
  | { readonly kind: "continuing"; readonly cursor: RecipientQualifiedOrganizationCursor }
  | { readonly kind: "exhausted" };
type State = { readonly kind: "open"; readonly owner: RecipientQualifiedWorkspaceInspection;
  position: Position }
  | { readonly kind: "closed" };

/** Closed lifecycle and custody failures for organization intake. */
export class OrganizationIntakeError extends Error {
  constructor(readonly kind: "closed" | "foreign-occurrence" | "owner-mismatch") {
    super(`Organization scene intake rejected: ${kind}`);
    this.name = "OrganizationIntakeError";
  }
}

/**
 * Retain SDK records on the main side and give their visual appearances local IDs.
 * Selected organization records remain their original SDK values and handles.
 * This class does not derive topology, infer semantic identity, or grant disclosure.
 *
 * One call advances at most one SDK page. Empty pages may continue. Exhaustion
 * means organization enumeration ended, not that every workspace fact was inspected.
 * Nested details stay on the existing SDK handles; they are not eagerly expanded.
 * Source occurrences retain the SDK's exact paths, roles and continuation.
 *
 * A query reserves its maximum visits and copied-string bytes before dispatch.
 * Success refunds unused capacity. A failed SDK query keeps that charge because
 * its actual work is unknown; the original continuation remains unchanged.
 * SDK diagnostics propagate unchanged. No source text or native handle is sent
 * to a layout worker. Previously returned SDK handles can outlive close().
 * Cancellation stops between bounded calls. It does not interrupt a native
 * SDK call that has already started.
 */
export class OrganizationIntake {
  #state: State;
  readonly #query: OrganizationQueryLimits;
  readonly #limits: OrganizationIntakeLimits;
  readonly #used: Record<OrganizationIntakeResource, number> = { queries: 0, recordVisits: 0, sourceVisits: 0,
    records: 0, occurrences: 0, sourceOccurrences: 0, copiedStringBytes: 0 };
  readonly #entries = new Map<SceneOccurrence, RecipientQualifiedOrganizationEntry>();
  readonly #enumerated: SceneOccurrence[] = [];
  #enumerationResourceStop: OrganizationIntakeResource | undefined;
  readonly #activeRetention = { records: 0, occurrences: 0 };
  readonly #budget: SceneBudget;
  readonly #inspection: RecipientQualifiedWorkspaceInspection;

  static {
    captureCoverage = (intake, occurrences) => intake.#captureCoverage(occurrences);
    captureBudget = intake => intake.#budget;
    captureInspection = intake => intake.#inspection;
  }

  /** Retain one recipient-qualified inspection with fixed query and aggregate limits. */
  constructor(owner: RecipientQualifiedWorkspaceInspection, query: OrganizationQueryLimits,
    limits: OrganizationIntakeLimits,
    budget?: SceneBudget) {
    if (!(owner instanceof RecipientQualifiedWorkspaceInspection)) {
      throw new TypeError("Organization intake requires a recipient-qualified inspection");
    }
    const querySnapshot = { inspected: query.inspected, returned: query.returned,
      copiedStringBytes: query.copiedStringBytes };
    const limitSnapshot = { queries: limits.queries, recordVisits: limits.recordVisits,
      sourceVisits: limits.sourceVisits, records: limits.records, occurrences: limits.occurrences,
      sourceOccurrences: limits.sourceOccurrences, copiedStringBytes: limits.copiedStringBytes };
    for (const value of [querySnapshot.inspected, querySnapshot.returned, querySnapshot.copiedStringBytes,
      limitSnapshot.queries, limitSnapshot.recordVisits, limitSnapshot.sourceVisits, limitSnapshot.records,
      limitSnapshot.occurrences, limitSnapshot.sourceOccurrences, limitSnapshot.copiedStringBytes]) {
      if (!Number.isSafeInteger(value) || value < 1) throw new RangeError("Scene intake limits must be positive safe integers");
    }
    // The SDK remains the owner of its narrower query ceilings and diagnostics.
    this.#query = querySnapshot; this.#limits = limitSnapshot; this.#budget = budget ?? defaultSceneBudget();
    this.#inspection = owner;
    this.#state = { kind: "open", owner, position: { kind: "initial" } };
  }

  /** Counts include conservative reservations retained after failed queries. */
  get charges(): OrganizationIntakeCharges { return { ...this.#used }; }

  /** @internal Retain the same opaque admission owner for another main-side Scene query. */
  retainInspection(): RecipientQualifiedWorkspaceInspection {
    if (this.#state.kind === "closed") throw new OrganizationIntakeError("closed");
    return this.#state.owner;
  }

  /** Advance one enumeration page. Cancellation is checked before the SDK call starts. */
  next(signal?: AbortSignal): OrganizationStep {
    if (this.#state.kind === "closed") return { kind: "closed" };
    if (this.#state.position.kind === "exhausted") return { kind: "enumeration-exhausted" };
    if (signal?.aborted) return cancelledStep;
    const state = this.#state;
    const step = this.#page("enumerated-record", owner => inspectOrganization.call(owner,
      this.#query.inspected, this.#query.returned, this.#query.copiedStringBytes,
      state.position.kind === "continuing" ? state.position.cursor : undefined));
    if (step.kind === "resource-stop") {
      this.#enumerationResourceStop = step.resource;
    } else if (step.kind === "page") {
      this.#enumerationResourceStop = undefined;
      this.#enumerated.push(...step.occurrences);
      const continuation = pageContinuation.call(step.page);
      state.position = continuation === null ? { kind: "exhausted" } : { kind: "continuing", cursor: continuation };
    }
    return step;
  }

  #captureCoverage(occurrences: readonly SceneOccurrence[]): OrganizationSceneCoverage {
    if (this.#state.kind === "closed") throw new OrganizationIntakeError("closed");
    const selected = new Set<SceneOccurrence>();
    for (const occurrence of occurrences) {
      if (!this.#entries.has(occurrence)) throw new OrganizationIntakeError("foreign-occurrence");
      selected.add(occurrence);
    }
    const allEnumerated = occurrences.length === this.#enumerated.length
      && selected.size === this.#enumerated.length
      && this.#enumerated.every(occurrence => selected.has(occurrence));
    const enumeration: OrganizationEnumerationCoverage = this.#state.position.kind === "exhausted"
      ? Object.freeze({ kind: "exhausted" })
      : this.#enumerationResourceStop === undefined
        ? Object.freeze({ kind: "incomplete" })
        : Object.freeze({ kind: "resource-stopped", resource: this.#enumerationResourceStop });
    const selection: OrganizationSelectionCoverage = Object.freeze({
      kind: allEnumerated ? "all-enumerated-occurrences" : "subset",
    });
    return issueCoverage(enumeration, selection);
  }

  /**
   * Read one exact endpoint page and retain its record or binding witnesses.
   * The SDK page preserves cumulative lookup, selected-input and completeness
   * evidence. ExternalReference does not invent a native record or prove readiness.
   * Pass the original SDK continuation explicitly; this never moves enumeration.
   * SDK selector, cursor, limit and copy failures propagate with their diagnostics;
   * failed calls retain the reserved work charge and publish no occurrences.
   * Closing during selector conversion prevents enrollment; nested calls share
   * the same available retention capacity.
   */
  endpoint(endpoint: OrganizationNode, after?: RecipientQualifiedOrganizationCursor | null,
    signal?: AbortSignal): OrganizationQueryStep {
    return this.#page("endpoint-query", owner => inspectEndpoint.call(owner, endpoint,
      this.#query.inspected, this.#query.returned, this.#query.copiedStringBytes, after), signal);
  }

  /**
   * Read one page of explicit and already-derived incident relations.
   * This is not a complete neighborhood: services, structural associations and
   * transitive closure are not selected by this SDK query. Empty incident output
   * does not establish endpoint absence. Direction and parallel records stay intact.
   * The caller holds the SDK continuation; enumeration remains independent.
   * Failure and reservation rules are the same as endpoint(), including no partial
   * enrollment on SDK failure. Neither query grants disclosure or execution rights.
   */
  incidentRelations(endpoint: OrganizationNode, after?: RecipientQualifiedOrganizationCursor | null,
    signal?: AbortSignal): OrganizationQueryStep {
    return this.#page("incident-relation-query", owner => inspectRelations.call(owner, endpoint,
      this.#query.inspected, this.#query.returned, this.#query.copiedStringBytes, after), signal);
  }

  /**
   * Read one service page at either end of an exact qualified scope.
   * Direction, full contract identity and independent requirements stay in the
   * SDK entry. Empty pages can continue and never prove scope absence or readiness.
   * The caller retains the SDK cursor; enumeration is unchanged. Reservations,
   * reentrant closure and diagnostic propagation follow endpoint().
   */
  incidentServices(scope: OrganizationScopeRef, after?: RecipientQualifiedOrganizationCursor | null,
    signal?: AbortSignal): OrganizationQueryStep {
    return this.#page("incident-service-query", owner => inspectServices.call(owner, scope,
      this.#query.inspected, this.#query.returned, this.#query.copiedStringBytes, after), signal);
  }

  #page(inclusion: Exclude<SceneOccurrence["inclusion"], "additional-occurrence">,
    read: (owner: RecipientQualifiedWorkspaceInspection) => RecipientQualifiedOrganizationPage,
    signal?: AbortSignal): OrganizationQueryStep {
    if (this.#state.kind === "closed") return { kind: "closed" };
    if (signal?.aborted) return cancelledStep;
    const state = this.#state, owner = state.owner;
    const shortage = this.#shortage({ queries: 1, recordVisits: this.#query.inspected,
      records: this.#query.returned, occurrences: this.#query.returned, copiedStringBytes: this.#query.copiedStringBytes });
    if (shortage) return { kind: "resource-stop", resource: shortage };
    chargeSceneWork(this.#budget, 1 + this.#query.inspected);
    this.#used.queries++; this.#used.recordVisits += this.#query.inspected;
    this.#used.copiedStringBytes += this.#query.copiedStringBytes;
    this.#activeRetention.records += this.#query.returned;
    this.#activeRetention.occurrences += this.#query.returned;
    try {
      const page = read(owner);
      if (this.#state !== state) return { kind: "closed" };
      const entries = pageEntries.call(page);
      const copiedStringBytes = pageCopiedStringBytes.call(page) as number;
      const localOccurrences = entries.map((_, index) => ({ id: `o${this.#used.occurrences + index}`, inclusion }));
      chargeRetainedSceneBytes(this.#budget, copiedStringBytes + retainedSceneValueBytes(localOccurrences));
      this.#used.recordVisits -= this.#query.inspected - pageInspected.call(page);
      this.#used.copiedStringBytes -= this.#query.copiedStringBytes - copiedStringBytes;
      this.#used.records += entries.length;
      const occurrences = Object.freeze(entries.map(entry => this.#include(entry, inclusion)));
      return { kind: "page", page, occurrences };
    } finally {
      this.#activeRetention.records -= this.#query.returned;
      this.#activeRetention.occurrences -= this.#query.returned;
    }
  }

  /** Resolve by this exact appearance token, never by its text ID or a title. */
  entry(occurrence: SceneOccurrence): RecipientQualifiedOrganizationEntry {
    return this.#entry(occurrence);
  }

  #entry(occurrence: SceneOccurrence): RecipientQualifiedOrganizationEntry {
    if (this.#state.kind === "closed") throw new OrganizationIntakeError("closed");
    const entry = this.#entries.get(occurrence);
    if (!entry) throw new OrganizationIntakeError("foreign-occurrence");
    return entry;
  }

  /** A second appearance shares the same native record; it creates no semantic subject. */
  repeat(occurrence: SceneOccurrence): SceneOccurrence | OrganizationResourceStoppedStep<"occurrences"> {
    const entry = this.#entry(occurrence);
    if (this.#shortage({ occurrences: 1 })) return { kind: "resource-stop", resource: "occurrences" };
    chargeSceneWork(this.#budget, 1);
    chargeRetainedSceneBytes(this.#budget,
      retainedSceneValueBytes({ id: `o${this.#used.occurrences}`, inclusion: "additional-occurrence" }));
    return this.#include(entry, "additional-occurrence");
  }

  /**
   * Read source occurrences from the selected record's exact SDK owner.
   * The returned page preserves the record association and copied-byte evidence.
   * Use the named continuation unchanged; unmapped does not mean source absent.
   */
  sources(occurrence: SceneOccurrence, after?: RecipientQualifiedOrganizationSourceCursor,
    signal?: AbortSignal): SourceStep {
    if (this.#state.kind === "closed") return { kind: "closed" };
    if (signal?.aborted) return cancelledStep;
    const entry = this.#entry(occurrence);
    const shortage = this.#shortage({ queries: 1, sourceVisits: this.#query.inspected,
      sourceOccurrences: this.#query.returned, copiedStringBytes: this.#query.copiedStringBytes });
    if (shortage) return { kind: "resource-stop", resource: shortage };
    chargeSceneWork(this.#budget, 1 + this.#query.inspected);
    this.#used.queries++; this.#used.sourceVisits += this.#query.inspected;
    this.#used.copiedStringBytes += this.#query.copiedStringBytes;
    const handle = entryHandle.call(entry);
    const page = inspectSources.call(this.#inspection, handle, this.#query.inspected,
      this.#query.returned, this.#query.copiedStringBytes, after);
    const retainedRecord = sourceRecord.call(page);
    if (!sameRecord.call(retainedRecord, handle)) throw new OrganizationIntakeError("owner-mismatch");
    const mapping = sourceMapping.call(page);
    const copiedStringBytes = sourceCopiedStringBytes.call(page) as number;
    chargeRetainedSceneBytes(this.#budget, copiedStringBytes);
    this.#used.sourceVisits -= this.#query.inspected - (mapping.kind === "named" ? mapping.inspected : 0);
    this.#used.copiedStringBytes -= this.#query.copiedStringBytes - copiedStringBytes;
    this.#used.sourceOccurrences += mapping.kind === "named" ? mapping.occurrences.length : 0;
    return { kind: "page", page, mapping };
  }

  /**
   * Inspect one correspondence page without enrolling the opposite-side entry.
   * The SDK owns pair, side, selected-record and cursor validation. Searching is
   * not absence; callers retain the exact returned cursor to continue explicitly.
   * Uses the existing query/record-visit/copy budget; failed calls keep their
   * conservative reservation and publish neither a result nor a new cursor.
   */
  correspond(comparison: RecipientQualifiedWorkspaceComparisonInspection,
    side: OrganizationComparisonSide, occurrence: SceneOccurrence,
    after?: RecipientQualifiedOrganizationCorrespondenceCursor,
    signal?: AbortSignal): CorrespondenceStep {
    if (this.#state.kind === "closed") return { kind: "closed" };
    if (signal?.aborted) return cancelledStep;
    const entry = this.#entry(occurrence);
    const shortage = this.#shortage({ queries: 1, recordVisits: this.#query.inspected,
      copiedStringBytes: this.#query.copiedStringBytes });
    if (shortage) return { kind: "resource-stop", resource: shortage };
    chargeSceneWork(this.#budget, 1 + this.#query.inspected);
    this.#used.queries++; this.#used.recordVisits += this.#query.inspected;
    this.#used.copiedStringBytes += this.#query.copiedStringBytes;
    const handle = entryHandle.call(entry);
    const result = correspondRecord.call(comparison, side, handle, this.#query.inspected,
      this.#query.copiedStringBytes, after);
    const selected = correspondenceSelected.call(result);
    if (correspondenceSelectedSide.call(result) !== side
      || !sameRecord.call(selected, handle)) throw new OrganizationIntakeError("owner-mismatch");
    const copiedStringBytes = correspondenceCopiedStringBytes.call(result) as number;
    chargeRetainedSceneBytes(this.#budget, copiedStringBytes);
    this.#used.recordVisits -= this.#query.inspected - correspondenceInspected.call(result);
    this.#used.copiedStringBytes -= this.#query.copiedStringBytes - copiedStringBytes;
    return { kind: "result", result };
  }

  /** Release this intake's references; independently returned native handles remain valid. */
  close(): void { this.#state = { kind: "closed" }; this.#entries.clear(); }

  #include(entry: RecipientQualifiedOrganizationEntry,
    inclusion: SceneOccurrence["inclusion"]): SceneOccurrence {
    const occurrence = Object.freeze({ id: `o${this.#used.occurrences}`, inclusion });
    this.#used.occurrences++;
    this.#entries.set(occurrence, entry);
    return occurrence;
  }

  #shortage(reservation: Partial<Record<OrganizationIntakeResource, number>>): OrganizationIntakeResource | undefined {
    for (const key of Object.keys(reservation) as OrganizationIntakeResource[]) {
      const active = key === "records" || key === "occurrences" ? this.#activeRetention[key] : 0;
      if (reservation[key]! > this.#limits[key] - this.#used[key] - active) return key;
    }
    return undefined;
  }
}

/** @internal Capture coverage only while constructing an organization topology. */
export function captureOrganizationSceneCoverage(intake: OrganizationIntake,
  occurrences: readonly SceneOccurrence[]): OrganizationSceneCoverage {
  return captureCoverage(intake, occurrences);
}

/** @internal Return the shared aggregate budget for this intake and its scene. */
export function captureOrganizationSceneBudget(intake: OrganizationIntake): SceneBudget {
  return captureBudget(intake);
}

/** @internal Return the opaque admission owner for an exact observation join. */
export function captureOrganizationSceneInspection(
  intake: OrganizationIntake,
): RecipientQualifiedWorkspaceInspection {
  return captureInspection(intake);
}
