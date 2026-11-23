import * as vscode from "vscode";

import { sameContext } from "./profilePresentation";
import type {
  ConfigurationImprovementOverview,
  ConfigurationImprovementPreview,
  ConfigurationImprovementPreviewCoordinate,
  ConfigurationImprovementRequest,
  ConfigurationImprovementResponse,
  ConfigurationImprovementSubject,
  ConfigurationImprovementSubjectDetails,
  AdmittedEffectiveProfileResponse,
  EffectiveProfileRequest,
  EffectiveProfileResponse,
  ExpectedProfileCoordinate,
  ProfileInspectionContext,
} from "./protocol";
import {
  expectedProfileCoordinate,
  sameConfigurationImprovementPreviewCoordinate,
  sameExpectedProfileCoordinate,
} from "./protocol";

/** Native transport calls used by the effect-free review controller. */
export interface ConfigurationImprovementTransport {
  effectiveProfile(
    request: EffectiveProfileRequest,
    token: vscode.CancellationToken,
  ): Thenable<EffectiveProfileResponse>;
  improve(
    request: ConfigurationImprovementRequest,
    token: vscode.CancellationToken,
  ): Thenable<ConfigurationImprovementResponse>;
}

/** Native, read-only destination for a compiler-owned review report. */
export interface ConfigurationImprovementPresentation extends vscode.Disposable {
  /** Replace the prior report and bring the exact current preview into view. */
  show(report: string): void;
}

const SILENT_PRESENTATION: ConfigurationImprovementPresentation = {
  show: () => undefined,
  dispose: () => undefined,
};

type ReviewState =
  | { readonly kind: "inactive" }
  | {
      readonly kind: "explained";
      readonly context: ProfileInspectionContext;
      readonly baseline: ExpectedProfileCoordinate;
    }
  | {
      readonly kind: "previewed";
      readonly context: ProfileInspectionContext;
      readonly coordinate: ConfigurationImprovementPreviewCoordinate;
      readonly preview: ConfigurationImprovementPreview;
    }
  | { readonly kind: "terminal"; readonly disposition: string };

/** Commands that are valid for the currently selected profile context. */
export interface ConfigurationImprovementAvailability {
  readonly canBegin: boolean;
  readonly canEvaluate: boolean;
  readonly canInspect: boolean;
  readonly canFinish: boolean;
}

/** Folder-local presentation state over the LSP-retained API typestate. */
export class ConfigurationImprovementReview implements vscode.Disposable {
  private state: ReviewState = { kind: "inactive" };
  private active: vscode.CancellationTokenSource | undefined;

  constructor(
    private readonly transport: ConfigurationImprovementTransport,
    private readonly presentation: ConfigurationImprovementPresentation = SILENT_PRESENTATION,
  ) {}

  /** Report legal review transitions for one exact profile selection. */
  availability(
    context: ProfileInspectionContext | undefined,
  ): ConfigurationImprovementAvailability {
    if (context === undefined) {
      return { canBegin: false, canEvaluate: false, canInspect: false, canFinish: false };
    }
    const ownsContext =
      (this.state.kind === "explained" || this.state.kind === "previewed") &&
      sameContext(this.state.context, context);
    return {
      canBegin:
        this.state.kind === "inactive" ||
        this.state.kind === "terminal" ||
        !ownsContext,
      canEvaluate: this.state.kind === "explained" && ownsContext,
      canInspect: this.state.kind === "previewed" && ownsContext,
      canFinish: this.state.kind === "previewed" && ownsContext,
    };
  }

  /** Begin from the exact current admitted profile selected in the Explorer. */
  async begin(context: ProfileInspectionContext): Promise<void> {
    if (!this.availability(context).canBegin) {
      throw new Error("finish the current Pale configuration review before starting another");
    }
    const profile = await this.withCancellation((token) =>
      this.transport.effectiveProfile(
        {
          context,
          section: "intended-use-case",
          offset: 0,
          limit: 1,
        },
        token,
      ),
    );
    if (!isCurrentProfile(profile)) {
      throw new Error("the current editor state has no admitted profile to review");
    }
    const expectedProfile = expectedProfileCoordinate(profile);
    const response = await this.withCancellation((token) =>
      this.transport.improve(
        { action: "begin", context, expectedProfile },
        token,
      ),
    );
    if (
      response.state !== "explained" ||
      !sameContext(response.context, context) ||
      !sameExpectedProfileCoordinate(response.baseline, expectedProfile)
    ) {
      throw new Error("language server returned an unlike improvement baseline");
    }
    this.state = { kind: "explained", context: { ...context }, baseline: response.baseline };
    await vscode.window.showInformationMessage(
      "Pale configuration review started. Edit the configuration, then evaluate the current editor state.",
    );
  }

  /** Compile, diagnose, and preview the exact current editor snapshot. */
  async evaluate(): Promise<void> {
    if (this.state.kind !== "explained") {
      throw new Error("start a Pale configuration review before evaluating changes");
    }
    const { context, baseline } = this.state;
    const response = await this.withCancellation((token) =>
      this.transport.improve(
        { action: "evaluate", context, expectedBaseline: baseline },
        token,
      ),
    );
    if (
      response.state !== "previewed" ||
      !sameContext(response.context, context) ||
      !sameExpectedProfileCoordinate(response.coordinate.baseline, baseline)
    ) {
      throw new Error("language server returned an unlike configuration preview");
    }
    this.state = {
      kind: "previewed",
      context,
      coordinate: response.coordinate,
      preview: response.preview,
    };
    await vscode.window.showInformationMessage(
      `Pale preview: ${response.preview.changeRootCount} change roots; ${response.preview.affectedCount} affected and ${response.preview.unaffectedCount} unaffected subjects.`,
    );
  }

  /** Show bounded compiler evidence for the retained preview without changing source. */
  async inspect(): Promise<void> {
    if (this.state.kind !== "previewed") {
      throw new Error("evaluate a Pale configuration candidate before inspecting its evidence");
    }
    const { context, coordinate, preview } = this.state;
    const overview = await this.withCancellation((token) =>
      this.transport.improve(
        {
          action: "inspect",
          context,
          expectedPreview: coordinate,
          detail: {
            kind: "overview",
            rootsOffset: 0,
            impactsOffset: 0,
            limit: REVIEW_PAGE_LIMIT,
          },
        },
        token,
      ),
    );
    if (
      overview.state !== "details" ||
      !sameContext(overview.context, context) ||
      !sameConfigurationImprovementPreviewCoordinate(overview.coordinate, coordinate)
    ) {
      throw new Error("language server returned details for an unlike configuration preview");
    }

    const details: ConfigurationImprovementSubjectDetails[] = [];
    const affected = overview.impacts.entries.filter(({ state }) => state === "affected");
    for (const { subject } of affected.slice(0, REVIEW_SUBJECT_LIMIT)) {
      const response = await this.inspectSubject(context, coordinate, subject);
      details.push(response);
    }
    this.presentation.show(configurationImprovementReport(preview, overview, details));
  }

  /** Consume the preview into one explicit non-effectful decision. */
  async finish(
    disposition: "accept" | "reject" | "cancel",
  ): Promise<void> {
    if (this.state.kind !== "previewed") {
      throw new Error("evaluate a Pale configuration candidate before finishing its review");
    }
    const { context, coordinate } = this.state;
    const response = await this.withCancellation((token) =>
      this.transport.improve(
        { action: "finish", context, expectedPreview: coordinate, disposition },
        token,
      ),
    );
    if (
      response.state !== "terminal" ||
      !sameContext(response.context, context) ||
      !sameConfigurationImprovementPreviewCoordinate(response.coordinate, coordinate)
    ) {
      throw new Error("language server returned an unlike terminal review product");
    }
    this.state = { kind: "terminal", disposition: response.disposition };
    await vscode.window.showInformationMessage(
      response.disposition === "accepted"
        ? "Pale preview accepted for a separate authority and write gate; no source was changed."
        : `Pale configuration review ended as ${response.disposition}; no source was changed.`,
    );
  }

  dispose(): void {
    this.active?.cancel();
    this.active?.dispose();
    this.active = undefined;
    this.presentation.dispose();
  }

  private async inspectSubject(
    context: ProfileInspectionContext,
    coordinate: ConfigurationImprovementPreviewCoordinate,
    subject: ConfigurationImprovementSubject,
  ): Promise<ConfigurationImprovementSubjectDetails> {
    const response = await this.withCancellation((token) =>
      this.transport.improve(
        {
          action: "inspect",
          context,
          expectedPreview: coordinate,
          detail: {
            kind: "subject",
            subject,
            inputLimit: REVIEW_SOURCE_LIMIT,
            maximumInspected: REVIEW_SOURCE_SCAN_LIMIT,
            maximumReturned: REVIEW_SOURCE_LIMIT,
          },
        },
        token,
      ),
    );
    if (
      response.state !== "subject-details" ||
      !sameContext(response.context, context) ||
      !sameConfigurationImprovementPreviewCoordinate(response.coordinate, coordinate)
    ) {
      throw new Error("language server returned evidence for an unlike configuration preview");
    }
    return response;
  }

  private async withCancellation<T>(
    operation: (token: vscode.CancellationToken) => Thenable<T>,
  ): Promise<T> {
    this.active?.cancel();
    this.active?.dispose();
    const cancellation = new vscode.CancellationTokenSource();
    this.active = cancellation;
    try {
      const result = await operation(cancellation.token);
      if (cancellation.token.isCancellationRequested) {
        throw new vscode.CancellationError();
      }
      return result;
    } finally {
      if (this.active === cancellation) {
        this.active = undefined;
      }
      cancellation.dispose();
    }
  }
}

const REVIEW_PAGE_LIMIT = 64;
const REVIEW_SUBJECT_LIMIT = 8;
const REVIEW_SOURCE_LIMIT = 16;
const REVIEW_SOURCE_SCAN_LIMIT = 64;

/** Format already-typed evidence without deriving new review meaning. */
export function configurationImprovementReport(
  preview: ConfigurationImprovementPreview,
  overview: ConfigurationImprovementOverview,
  details: ReadonlyArray<ConfigurationImprovementSubjectDetails>,
): string {
  const lines = [
    "Pale configuration review",
    "",
    `Project: ${preview.projectId}`,
    `Execution workspace: ${preview.executionWorkspaceId}`,
    `Candidate editor generation: ${overview.coordinate.candidateEditor.generation}`,
    `Candidate document revision: ${overview.coordinate.candidateEditor.documentRevision}`,
    `Derivation receipt: ${preview.derivationReceiptDigest}`,
    "",
    "Compiler-proved transitions",
    `- Selected topology: ${formatCoordinateChange(preview.selectedTopologyChange)}`,
    `- Effective profile: ${formatCoordinateChange(preview.profileChange)}`,
    `- Private instance: ${formatCoordinateChange(preview.instanceChange)}`,
    `- Package meaning: ${formatCoordinateChange(preview.packageSemanticIrChange)}`,
    `- Composition plan: ${formatCoordinateChange(preview.compositionPlanChange)}`,
    "",
    `Change roots (${preview.changeRootCount})`,
  ];
  for (const root of overview.changeRoots.entries) {
    lines.push(
      `- ${formatSubject(root.subject)}: ${root.change.kind}; root ${shortDigest(root.rootDigest)}`,
    );
  }
  if (overview.changeRoots.entries.length === 0) {
    lines.push("- None");
  }
  if (overview.changeRoots.nextOffset !== null) {
    lines.push(`- More roots remain after offset ${overview.changeRoots.nextOffset}.`);
  }

  lines.push("", `Affected subjects (${preview.affectedCount})`);
  const affected = overview.impacts.entries.filter(({ state }) => state === "affected");
  for (const impact of affected) {
    lines.push(
      `- ${formatSubject(impact.subject)} (${impact.contributingRootCount} contributing roots)`,
    );
  }
  if (affected.length === 0) {
    lines.push("- None in this bounded page");
  }
  if (overview.impacts.nextOffset !== null) {
    lines.push(`- More impact rows remain after offset ${overview.impacts.nextOffset}.`);
  }

  for (const detail of details) {
    lines.push("", `Why ${formatSubject(detail.explanation.subject)}`);
    if (detail.explanation.causal === undefined) {
      lines.push(`- Comparison state: ${detail.explanation.state}`);
    } else {
      lines.push(
        `- Root: ${formatSubject(detail.explanation.causal.root.subject)} (${detail.explanation.causal.root.change.kind})`,
      );
      for (const step of detail.explanation.causal.steps) {
        lines.push(`- ${step.rule}: ${formatSubject(step.subject)}`);
      }
    }
    lines.push("- Candidate source occurrences:");
    for (const occurrence of detail.source.occurrences) {
      lines.push(`  - ${occurrence.role}: ${formatSourceLocation(occurrence.location)}`);
    }
    if (detail.source.occurrences.length === 0) {
      lines.push("  - None in this bounded scan");
    }
    if (!detail.source.sourcesComplete) {
      lines.push(
        `  - Source scan is incomplete after inspecting ${detail.source.inspected} candidates.`,
      );
    }
    if (detail.source.recordAnchor !== undefined) {
      lines.push(`- Owning record: ${formatSourceLocation(detail.source.recordAnchor)}`);
    }
  }
  if (affected.length > details.length) {
    lines.push(
      "",
      `The report shows causal and source detail for the first ${details.length} affected subjects in this page.`,
    );
  }
  lines.push("", "This report does not apply or authorize a configuration write.");
  return `${lines.join("\n")}\n`;
}

function formatCoordinateChange(
  change: ConfigurationImprovementPreview["profileChange"],
): string {
  return change.state === "unchanged"
    ? `unchanged (${shortDigest(change.digest)})`
    : `${shortDigest(change.prior_digest)} → ${shortDigest(change.candidate_digest)}`;
}

function formatSubject(subject: ConfigurationImprovementSubject): string {
  switch (subject.kind) {
    case "source":
      return `source ${subject.sourceId}`;
    case "component":
      return `component ${subject.componentId}`;
    case "module":
      return `module ${subject.moduleId}`;
    case "declaration":
      return `declaration ${subject.ownerModuleId}/${subject.namespaceId}/${subject.declarationId}`;
    case "import":
      return `import ${subject.importerModuleId} → ${subject.upstreamModuleId}@${subject.upstreamReleaseId}/${subject.exportId}`;
    case "extension":
      return `extension ${subject.ownerModuleId}/${subject.extensionNamespaceId} → ${subject.targetNamespaceId}/${subject.targetDeclarationId}`;
    case "policy":
      return `policy ${subject.ownerModuleId}/${subject.namespaceId}/${subject.declarationId}`;
    case "responsibility-closure":
      return "responsibility closure";
    case "organizational-translation-closure":
      return "organizational translation closure";
    case "experience-composition":
      return "experience composition";
    case "resource":
      return `resource ${subject.ownerModuleId}/${subject.resourceId}`;
    case "resolution-lock":
      return "resolution lock";
    case "universe-lock":
      return "universe lock";
    case "workspace":
      return `workspace ${subject.workspaceId}`;
  }
}

function formatSourceLocation(
  location: ConfigurationImprovementSubjectDetails["source"]["occurrences"][number]["location"],
): string {
  const path = location.path.map((segment) =>
    segment.kind === "field" ? `.${segment.value}` : `[${segment.value}]`
  ).join("");
  return `${location.sourceLabel}${path}`;
}

function shortDigest(digest: string): string {
  return digest.slice(0, 12);
}

function isCurrentProfile(
  response: EffectiveProfileResponse,
): response is AdmittedEffectiveProfileResponse & { readonly state: { readonly kind: "current" } } {
  return response.state.kind === "current";
}
