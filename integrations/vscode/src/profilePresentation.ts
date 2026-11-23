import type {
  ApiDiagnosticFact,
  ApiDiagnosticProjection,
  EffectiveProfileEntry,
  EffectiveProfileExplanationEvidence,
  EffectiveProfileFinding,
  EffectiveProfileSection,
  EffectiveProfileState,
  EditorCoordinate,
  ProfileInspectionContext,
} from "./protocol";

/** Bounded LSP-owned capacity data carried outside the canonical diagnostic model. */
export interface LspCapacityProjection {
  readonly kind: string;
  readonly unit: string;
  readonly scope?: "workspace" | "private" | "Package";
  readonly maximum?: string;
  readonly observed?: string;
  readonly observedMinimum?: string;
  readonly recovery: string;
}

const CAPACITY_KIND_UNITS: Readonly<Record<string, string>> = {
  "query-queue": "requests",
  "direct-references": "locations",
  "reference-locations": "locations",
  "reference-pages": "pages",
  "syntax-facts": "facts",
  "standard-response": "members",
  "document-bytes": "bytes",
  "document-count": "documents",
  "total-bytes": "bytes",
};

/** Native tree roots in their stable, deliberately shallow display order. */
export const PROFILE_SECTIONS: ReadonlyArray<{
  section: EffectiveProfileSection;
  label: string;
}> = [
  { section: "intended-use-case", label: "Intended use case" },
  { section: "effective-configuration", label: "Effective configuration" },
  {
    section: "capabilities-and-operations",
    label: "Capabilities and operations",
  },
  { section: "machine-realization", label: "Machine realization" },
  { section: "gaps-and-explanations", label: "Gaps and explanations" },
];

/** Compare two ephemeral selection contexts without interpreting their identities. */
export function sameContext(
  left: ProfileInspectionContext | undefined,
  right: ProfileInspectionContext | undefined,
): boolean {
  return (
    left !== undefined &&
    right !== undefined &&
    left.projectId === right.projectId &&
    left.executionWorkspaceId === right.executionWorkspaceId
  );
}

/** Present one server-owned admission state without upgrading its evidence ceiling. */
export function profileStateLabel(
  state: EffectiveProfileState,
  current: EditorCoordinate,
): string {
  switch (state.kind) {
    case "current":
      return `Current editor generation ${current.generation}`;
    case "stale":
      return `Out of date: ${state.summary}; showing the last usable generation ${state.admittedEditor.generation}; current generation is ${current.generation}`;
    case "not-admitted":
      return `No usable profile: ${state.summary}`;
  }
}

/** Select only a native icon name; semantic labels and dispositions remain server-owned. */
export function entryIcon(entry: EffectiveProfileEntry): string {
  switch (entry.kind) {
    case "declared-selection":
      return "symbol-key";
    case "responsibility":
      return "target";
    case "selection":
      return "settings-gear";
    case "capability-narrowing":
      return "filter";
    case "governing-ceiling":
      return "shield";
    case "repository":
      return "repo";
    case "provider-repository":
      return "plug";
    case "execution-path":
      return "type-hierarchy-sub";
    case "operation":
      return "tools";
    case "capability":
      return "key";
    case "machine-realization":
      return "server";
    case "finding": {
      const finding = entry.finding;
      return finding.kind === "requirement" &&
        finding.disposition.kind === "satisfied"
        ? "pass"
        : "warning";
    }
  }
}

/** Read only the opaque API-owned correlation identity from LSP diagnostic data. */
export function opaqueDiagnosticIdentity(value: unknown): string | undefined {
  if (typeof value !== "object" || value === null) {
    return undefined;
  }
  const candidate = value as Readonly<Record<string, unknown>>;
  return typeof candidate.identity === "string"
    ? candidate.identity
    : undefined;
}

/**
 * Project the presentation fields from one already-admitted LSP diagnostic.
 *
 * This is structural transport handling, not diagnostic admission. In
 * particular, it does not constrain the API-owned policy vocabulary or decide
 * whether any combination of stage, disposition, retry, and effect is valid.
 */
export function apiDiagnosticProjection(
  value: unknown,
): ApiDiagnosticProjection | undefined {
  const candidate = record(value);
  if (candidate === undefined) {
    return undefined;
  }
  const code = stringField(candidate, "code");
  const stage = stringField(candidate, "stage");
  const disposition = stringField(candidate, "disposition");
  const retry = stringField(candidate, "retry");
  const effect = stringField(candidate, "effect");
  const summary = stringField(candidate, "summary");
  if (
    code === undefined ||
    stage === undefined ||
    disposition === undefined ||
    retry === undefined ||
    effect === undefined ||
    summary === undefined
  ) {
    return undefined;
  }
  const notes = optionalStringList(candidate.notes);
  const help = optionalStringList(candidate.help);
  const facts = optionalFacts(candidate.facts);
  if (notes === null || help === null || facts === null) {
    return undefined;
  }
  return {
    code,
    stage,
    disposition,
    retry,
    effect,
    summary,
    ...(notes === undefined ? {} : { notes }),
    ...(help === undefined ? {} : { help }),
    ...(facts === undefined ? {} : { facts }),
  };
}

/** Resolve the validated diagnostic presentation paired with one opaque LSP identity. */
export function apiDiagnosticFromLspData(
  value: unknown,
): ApiDiagnosticProjection | undefined {
  const envelope = record(value);
  return envelope === undefined
    ? undefined
    : apiDiagnosticProjection(envelope.diagnostic);
}

/** Resolve an API diagnostic carried directly as an LSP request-error payload. */
export function apiDiagnosticFromRequestError(
  value: unknown,
): ApiDiagnosticProjection | undefined {
  const error = record(value);
  return error === undefined ? undefined : apiDiagnosticProjection(error.data);
}

/** Decode only the closed, presentation-safe capacity data emitted by `pale-lsp`. */
export function lspCapacityFromRequestError(
  value: unknown,
): LspCapacityProjection | undefined {
  const error = record(value);
  const candidate = error === undefined ? undefined : record(error.data);
  if (candidate === undefined || !onlyCapacityKeys(candidate)) {
    return undefined;
  }
  const kind = stringField(candidate, "kind");
  const unit = stringField(candidate, "unit");
  const recovery = boundedPlainText(candidate.recovery);
  if (kind === undefined || unit === undefined || recovery === undefined) {
    return undefined;
  }
  const expectedUnit = CAPACITY_KIND_UNITS[kind];
  if (expectedUnit === undefined || unit !== expectedUnit) {
    return undefined;
  }
  const scope = optionalScope(candidate.scope);
  const maximum = optionalU64(candidate.maximum);
  const observed = optionalU64(candidate.observed);
  const observedMinimum = optionalU64(candidate.observedMinimum);
  if (
    scope === null ||
    maximum === null ||
    observed === null ||
    observedMinimum === null ||
    (observed !== undefined && observedMinimum !== undefined) ||
    ((observed !== undefined || observedMinimum !== undefined) &&
      maximum === undefined) ||
    !capacityShapeIsValid(kind, scope, maximum, observed, observedMinimum)
  ) {
    return undefined;
  }
  return {
    kind,
    unit,
    recovery,
    ...(scope === undefined ? {} : { scope }),
    ...(maximum === undefined ? {} : { maximum }),
    ...(observed === undefined ? {} : { observed }),
    ...(observedMinimum === undefined ? {} : { observedMinimum }),
  };
}

/** Render one structurally admitted capacity failure without trusting error prose. */
export function lspCapacityLines(capacity: LspCapacityProjection): string[] {
  const scope = capacity.scope === undefined ? "" : ` for ${capacity.scope}`;
  const demand =
    capacity.observed !== undefined
      ? `Observed: ${capacity.observed} ${capacity.unit}`
      : capacity.observedMinimum !== undefined
        ? `Observed: at least ${capacity.observedMinimum} ${capacity.unit}`
        : undefined;
  return [
    `Pale reached its ${capacity.kind.replaceAll("-", " ")} limit${scope}.`,
    ...(capacity.maximum === undefined
      ? []
      : [`Maximum: ${capacity.maximum} ${capacity.unit}`]),
    ...(demand === undefined ? [] : [demand]),
    `Recovery: ${capacity.recovery}`,
  ];
}

/** Render API-owned fields mechanically for an untrusted native text surface. */
export function apiDiagnosticLines(
  diagnostic: ApiDiagnosticProjection,
): string[] {
  return [
    diagnostic.summary,
    `Code: ${diagnostic.code}`,
    `Stage: ${diagnostic.stage}`,
    `Disposition: ${diagnostic.disposition}`,
    `Retry: ${diagnostic.retry}`,
    `Effect: ${diagnostic.effect}`,
    ...(diagnostic.notes ?? []).map((note) => `Note: ${note}`),
    ...(diagnostic.help ?? []).map((help) => `Help: ${help}`),
    ...(diagnostic.facts ?? []).map(
      (fact) => `Fact ${fact.key}: ${fact.value}`,
    ),
  ];
}

function record(value: unknown): Readonly<Record<string, unknown>> | undefined {
  return typeof value === "object" && value !== null
    ? (value as Readonly<Record<string, unknown>>)
    : undefined;
}

function stringField(
  value: Readonly<Record<string, unknown>>,
  field: string,
): string | undefined {
  const candidate = value[field];
  return typeof candidate === "string" ? candidate : undefined;
}

function optionalStringList(
  value: unknown,
): ReadonlyArray<string> | undefined | null {
  if (value === undefined) {
    return undefined;
  }
  return Array.isArray(value) &&
    value.every((entry) => typeof entry === "string")
    ? [...value]
    : null;
}

function optionalFacts(
  candidate: unknown,
): ReadonlyArray<ApiDiagnosticFact> | undefined | null {
  if (candidate === undefined) {
    return undefined;
  }
  if (!Array.isArray(candidate)) {
    return null;
  }
  const facts: ApiDiagnosticFact[] = [];
  for (const entry of candidate) {
    const fact = record(entry);
    if (fact === undefined) {
      return null;
    }
    const key = stringField(fact, "key");
    const factValue = stringField(fact, "value");
    if (key === undefined || factValue === undefined) {
      return null;
    }
    facts.push({ key, value: factValue });
  }
  return facts;
}

function onlyCapacityKeys(
  candidate: Readonly<Record<string, unknown>>,
): boolean {
  const allowed = new Set([
    "kind",
    "unit",
    "scope",
    "maximum",
    "observed",
    "observedMinimum",
    "recovery",
  ]);
  return Object.keys(candidate).every((key) => allowed.has(key));
}

function capacityShapeIsValid(
  kind: string,
  scope: LspCapacityProjection["scope"] | undefined,
  maximum: string | undefined,
  observed: string | undefined,
  observedMinimum: string | undefined,
): boolean {
  if (
    kind === "document-bytes" ||
    kind === "document-count" ||
    kind === "total-bytes"
  ) {
    return (
      scope !== undefined &&
      maximum !== undefined &&
      observed !== undefined &&
      observedMinimum === undefined
    );
  }
  if (scope !== undefined || observed !== undefined) {
    return false;
  }
  if (
    kind === "query-queue" ||
    kind === "syntax-facts" ||
    kind === "standard-response"
  ) {
    return maximum !== undefined && observedMinimum !== undefined;
  }
  if (kind === "reference-pages") {
    return maximum !== undefined && observedMinimum === undefined;
  }
  return maximum === undefined && observedMinimum === undefined;
}

function optionalScope(
  value: unknown,
): LspCapacityProjection["scope"] | undefined | null {
  if (value === undefined) {
    return undefined;
  }
  return value === "workspace" || value === "private" || value === "Package"
    ? value
    : null;
}

function optionalU64(value: unknown): string | undefined | null {
  if (value === undefined) {
    return undefined;
  }
  if (
    typeof value !== "string" ||
    value.length === 0 ||
    value.length > 20 ||
    !/^(0|[1-9][0-9]*)$/u.test(value) ||
    BigInt(value) > 0xffff_ffff_ffff_ffffn
  ) {
    return null;
  }
  return value;
}

function boundedPlainText(value: unknown): string | undefined {
  return typeof value === "string" &&
    value.length >= 1 &&
    value.length <= 512 &&
    value === value.trim() &&
    !/[\u0000-\u001f\u007f-\u009f\u202a-\u202e\u2066-\u2069]/u.test(value)
    ? value
    : undefined;
}

/** Typed server-owned disposition fields shown directly by the native tree. */
export function entryDescription(
  entry: EffectiveProfileEntry,
): string | undefined {
  switch (entry.kind) {
    case "declared-selection":
      return entry.selection.kind === "resource"
        ? `declared ${entry.selection.resourceKind} · ${entry.selection.selector.kind}`
        : `declared ${entry.selection.axis} default`;
    case "operation":
      return `exposure: ${entry.exposure} · native: ${entry.nativeRealization}`;
    case "capability":
      return `requirement: ${dispositionName(entry.requirementDisposition)} · provider: ${dispositionName(entry.providerDisposition)} · native: ${entry.nativeRealization}`;
    case "machine-realization":
      return `realization: ${entry.realization}`;
    case "finding":
      return `finding: ${findingDisposition(entry.finding)}`;
    case "repository":
      return `access: ${entry.accessCeiling}`;
    case "provider-repository":
      return `binding: ${entry.providerBindingId} · native: ${entry.nativeRealization}`;
    case "execution-path":
      return `terminal provider: ${entry.terminalProviderBinding?.providerBindingId ?? "not applicable (native-only path)"} · native: ${entry.nativeRealization}`;
    case "responsibility":
    case "selection":
    case "capability-narrowing":
    case "governing-ceiling":
      return undefined;
  }
}

/** Complete typed rows for an untrusted native tooltip. */
export function entryTooltipLines(entry: EffectiveProfileEntry): string[] {
  switch (entry.kind) {
    case "declared-selection":
      return entry.selection.kind === "resource"
        ? [
            `Declared resource kind: ${entry.selection.resourceKind}`,
            `Declared selector: ${entry.selection.selector.kind}`,
          ]
        : [
            `Declared mode-default axis: ${entry.selection.axis}`,
            `Declared resource: ${entry.selection.resourceId}`,
          ];
    case "operation":
      return [
        `Repository: ${entry.repositoryId}`,
        `Exposure: ${entry.exposure}`,
        `Native realization: ${entry.nativeRealization}`,
      ];
    case "capability":
      return [
        `Repository: ${entry.repositoryId}`,
        `Operation: ${entry.operationId}`,
        `Requirement: ${entry.requirementId}`,
        `Requirement disposition: ${dispositionName(entry.requirementDisposition)}`,
        `Provider disposition: ${dispositionName(entry.providerDisposition)}`,
        `Native realization: ${entry.nativeRealization}`,
      ];
    case "machine-realization":
      return [
        `Machine: ${entry.machineId}`,
        `Realization: ${entry.realization}`,
      ];
    case "finding":
      return [
        `Finding kind: ${entry.finding.kind}`,
        `Finding disposition: ${findingDisposition(entry.finding)}`,
      ];
    case "execution-path":
      return [
        `Experience path: ${entry.experiencePathId}`,
        entry.terminalProviderBinding === null
          ? "Terminal provider binding: not applicable (native-only path)"
          : `Terminal provider binding: ${entry.terminalProviderBinding.providerBindingId}`,
        ...(entry.terminalProviderBinding === null
          ? []
          : [
              `Terminal provider identity: ${entry.terminalProviderBinding.providerBindingIdentity}`,
            ]),
        `Native realization: ${entry.nativeRealization}`,
        `Composition reason steps: ${entry.reasonSteps.length}`,
      ];
    case "responsibility":
      return [
        `Execution path: ${entry.executionPathId}`,
        `Composition reason steps: ${entry.reasonSteps.length}`,
      ];
    case "repository":
      return [
        `Access ceiling: ${entry.accessCeiling}`,
        `Materialization order: ${entry.materializationOrder}`,
      ];
    case "provider-repository":
      return [
        `Repository: ${entry.repositoryId}`,
        `Provider binding: ${entry.providerBindingId}`,
        `Opaque native coordinate: ${entry.nativeRepositoryCoordinate}`,
        `Native realization: ${entry.nativeRealization}`,
      ];
    case "selection":
    case "capability-narrowing":
    case "governing-ceiling":
      return [];
  }
}

/** Compact native-message rendering of API-owned explanation evidence. */
export function explanationLabel(
  evidence: EffectiveProfileExplanationEvidence,
): string {
  switch (evidence.kind) {
    case "declared-selection":
      return "declared selector retained before effective closure expansion";
    case "responsibility":
      return `${evidence.reasonSteps.length} retained composition reason step(s)`;
    case "effective-selection":
      return `effective ${evidence.selection.kind} selection retained by compilation`;
    case "execution-path":
      return `${evidence.reasonSteps.length} retained composition reason step(s)`;
    case "operation-classification":
      return `exposure ${evidence.exposure}; native realization ${evidence.nativeRealization}`;
    case "capability-classification":
      return `requirement ${dispositionName(evidence.requirementDisposition)}; provider ${dispositionName(evidence.providerDisposition)}; native realization ${evidence.nativeRealization}`;
    case "finding-classification":
      return `finding ${findingDisposition(evidence.finding)}`;
    case "machine-realization":
      return `native realization ${evidence.realization}`;
    case "configuration":
      return "exact retained configuration evidence";
  }
}

function dispositionName(disposition: {
  kind: string;
  reason?: string;
  provider?: string;
}): string {
  return disposition.reason ?? disposition.provider ?? disposition.kind;
}

function findingDisposition(finding: EffectiveProfileFinding): string {
  return finding.kind === "requirement"
    ? dispositionName(finding.disposition)
    : finding.kind;
}

/** Mechanical epoch used only to suppress replies from a superseded context. */
export class ContextEpoch {
  private value = 0;

  /** Supersede every outstanding request and return the new epoch. */
  advance(): number {
    this.value += 1;
    return this.value;
  }

  /** Capture the current request epoch. */
  current(): number {
    return this.value;
  }

  /** Whether a response still belongs to the current ephemeral context. */
  owns(candidate: number): boolean {
    return candidate === this.value;
  }
}

/** Minimal cancellation surface shared by VS Code request tokens and tests. */
export interface CancellableRequest {
  cancel(): void;
}

/** Mechanical owner of in-flight requests for one ephemeral inspection context. */
export class ActiveRequestSet {
  private readonly requests = new Set<CancellableRequest>();

  add(request: CancellableRequest): void {
    this.requests.add(request);
  }

  delete(request: CancellableRequest): void {
    this.requests.delete(request);
  }

  /** Cancel and forget every request owned by the superseded context. */
  cancelAll(): void {
    for (const request of this.requests) {
      request.cancel();
    }
    this.requests.clear();
  }
}
