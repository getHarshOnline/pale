import * as vscode from "vscode";

import {
  ActiveRequestSet,
  apiDiagnosticFromLspData,
  apiDiagnosticFromRequestError,
  apiDiagnosticLines,
  ContextEpoch,
  entryDescription,
  entryIcon,
  entryTooltipLines,
  explanationLabel,
  lspCapacityFromRequestError,
  lspCapacityLines,
  opaqueDiagnosticIdentity,
  PROFILE_SECTIONS,
  profileStateLabel,
  sameContext,
} from "./profilePresentation";
import {
  expectedProfileCoordinate,
  profileProductMatchesExpected,
} from "./protocol";
import type {
  AdmittedEffectiveProfileResponse,
  ApiDiagnosticProjection,
  EffectiveProfileEntry,
  EffectiveProfileExplanationRequest,
  EffectiveProfileExplanationResponse,
  EffectiveProfileFinding,
  EffectiveProfileRequest,
  EffectiveProfileResponse,
  EffectiveProfileSection,
  EffectiveProfileSelection,
  ExpectedProfileCoordinate,
  ProfileInspectionContext,
} from "./protocol";

const PAGE_SIZE = 64;

/** Transport-only calls supplied by the language client. */
export interface ProfileTransport {
  effectiveProfile(
    request: EffectiveProfileRequest,
    token: vscode.CancellationToken,
  ): Thenable<EffectiveProfileResponse>;
  explain(
    request: EffectiveProfileExplanationRequest,
    token: vscode.CancellationToken,
  ): Thenable<EffectiveProfileExplanationResponse>;
}

export type ProfileNode = SectionNode | EntryNode | MoreNode | MessageNode;

interface SectionNode {
  kind: "section";
  section: EffectiveProfileSection;
  label: string;
}

interface EntryNode {
  kind: "entry";
  entry: EffectiveProfileEntry;
  explanationRequest: EffectiveProfileExplanationRequest;
}

interface MoreNode {
  kind: "more";
  section: EffectiveProfileSection;
  offset: number;
  expectedCoordinate: ExpectedProfileCoordinate;
}

interface MessageNode {
  kind: "message";
  label: string;
  icon: string;
  command?: vscode.Command;
  tooltip?: vscode.MarkdownString;
}

/** Native, presentation-only tree over the retained LSP inspection handle. */
export class ProfileExplorer
  implements vscode.TreeDataProvider<ProfileNode>, vscode.Disposable
{
  private readonly changes = new vscode.EventEmitter<
    ProfileNode | undefined | null | void
  >();
  private readonly activeRequests = new ActiveRequestSet();
  private readonly contextEpoch = new ContextEpoch();
  private context: ProfileInspectionContext | undefined;

  readonly onDidChangeTreeData = this.changes.event;

  constructor(
    private readonly transport: ProfileTransport,
    private readonly identity = "profile",
  ) {}

  /** Current ephemeral inspection context; it is never persisted as configuration. */
  inspectionContext(): ProfileInspectionContext | undefined {
    return this.context === undefined ? undefined : { ...this.context };
  }

  /** Replace the ephemeral context and cancel every request against its predecessor. */
  setInspectionContext(context: ProfileInspectionContext): void {
    if (sameContext(this.context, context)) {
      return;
    }
    this.context = { ...context };
    this.refresh();
  }

  /** Ask VS Code to rebuild the visible native tree from the retained server handle. */
  refresh(): void {
    this.contextEpoch.advance();
    this.cancelActiveRequests();
    this.changes.fire();
  }

  getTreeItem(node: ProfileNode): vscode.TreeItem {
    switch (node.kind) {
      case "section": {
        const item = new vscode.TreeItem(
          node.label,
          vscode.TreeItemCollapsibleState.Collapsed,
        );
        item.id = `${this.identity}:section:${node.section}`;
        item.contextValue = "pale.profile.section";
        item.iconPath = new vscode.ThemeIcon("list-tree");
        return item;
      }
      case "entry": {
        const item = new vscode.TreeItem(
          entryLabel(node.entry),
          vscode.TreeItemCollapsibleState.None,
        );
        item.id = `${this.identity}:entry:${node.entry.explanationHandle}`;
        const description = entryDescription(node.entry);
        if (description !== undefined) {
          item.description = description;
        }
        item.tooltip = tooltip(node.entry);
        item.iconPath = new vscode.ThemeIcon(entryIcon(node.entry));
        item.contextValue = "pale.profile.explainable";
        item.command = {
          command: "pale.profile.explain",
          title: "Explain profile item",
          arguments: [node.explanationRequest],
        };
        return item;
      }
      case "more": {
        const item = new vscode.TreeItem(
          "More…",
          vscode.TreeItemCollapsibleState.Collapsed,
        );
        item.id = `${this.identity}:more:${node.section}:${node.offset}:${node.expectedCoordinate.instanceDigest}`;
        item.contextValue = "pale.profile.more";
        item.iconPath = new vscode.ThemeIcon("ellipsis");
        return item;
      }
      case "message": {
        const item = new vscode.TreeItem(
          node.label,
          vscode.TreeItemCollapsibleState.None,
        );
        item.id = `${this.identity}:message:${node.icon}:${node.label}`;
        item.iconPath = new vscode.ThemeIcon(node.icon);
        if (node.tooltip !== undefined) {
          item.tooltip = node.tooltip;
        }
        if (node.command !== undefined) {
          item.command = node.command;
        }
        item.contextValue = "pale.profile.message";
        return item;
      }
    }
  }

  async getChildren(node?: ProfileNode): Promise<ProfileNode[]> {
    if (node === undefined) {
      return PROFILE_SECTIONS.map(({ section, label }) => ({
        kind: "section",
        section,
        label,
      }));
    }
    if (node.kind === "section") {
      return this.loadPage(node.section, 0);
    }
    if (node.kind === "more") {
      return this.loadPage(node.section, node.offset, node.expectedCoordinate);
    }
    return [];
  }

  /** Delegate one opaque handle to the retained effective-profile explanation owner. */
  async explain(request: EffectiveProfileExplanationRequest): Promise<void> {
    const epoch = this.contextEpoch.current();
    const cancellation = new vscode.CancellationTokenSource();
    this.activeRequests.add(cancellation);
    try {
      const response = await this.transport.explain(
        request,
        cancellation.token,
      );
      if (
        cancellation.token.isCancellationRequested ||
        !this.contextEpoch.owns(epoch) ||
        !sameContext(this.context, request.context) ||
        response.explanation.handle !== request.handle ||
        !sameContext(request.context, response.context) ||
        !profileProductMatchesExpected(
          response.editor,
          response.coordinates,
          request.expectedCoordinate,
        )
      ) {
        return;
      }
      await vscode.window.showInformationMessage(
        `Pale: ${explanationLabel(response.explanation.evidence)}.`,
      );
    } catch (error: unknown) {
      if (!cancellation.token.isCancellationRequested) {
        const diagnostic = apiDiagnosticFromRequestError(error);
        const capacity = lspCapacityFromRequestError(error);
        await vscode.window.showErrorMessage(
          diagnostic !== undefined
            ? apiDiagnosticLines(diagnostic).join("\n")
            : capacity !== undefined
              ? lspCapacityLines(capacity).join("\n")
              : "Pale explanation transport failed",
        );
      }
    } finally {
      this.activeRequests.delete(cancellation);
      cancellation.dispose();
    }
  }

  dispose(): void {
    this.cancelActiveRequests();
    this.changes.dispose();
  }

  private async loadPage(
    section: EffectiveProfileSection,
    offset: number,
    expectedCoordinate?: ExpectedProfileCoordinate,
  ): Promise<ProfileNode[]> {
    const context = this.inspectionContext();
    if (context === undefined) {
      return [
        {
          kind: "message",
          label: "Select a project and execution workspace to inspect",
          icon: "info",
          command: {
            command: "pale.profile.selectContext",
            title: "Select profile context",
          },
        },
      ];
    }

    const epoch = this.contextEpoch.current();
    const cancellation = new vscode.CancellationTokenSource();
    this.activeRequests.add(cancellation);
    try {
      const response = await this.transport.effectiveProfile(
        {
          context,
          section,
          offset,
          limit: PAGE_SIZE,
          ...(expectedCoordinate === undefined ? {} : { expectedCoordinate }),
        },
        cancellation.token,
      );
      if (
        cancellation.token.isCancellationRequested ||
        !this.contextEpoch.owns(epoch) ||
        !sameContext(context, response.context)
      ) {
        return [];
      }
      if (!hasAdmittedProfile(response)) {
        return offset === 0
          ? [
              {
                kind: "message",
                label: profileStateLabel(response.state, response.editor),
                icon: "error",
                ...diagnosticTooltipProperty(response.diagnosticIdentity),
              },
            ]
          : [];
      }
      if (response.page.section !== section) {
        return [];
      }
      if (
        expectedCoordinate !== undefined &&
        !profileProductMatchesExpected(
          response.editor,
          response.coordinates,
          expectedCoordinate,
        )
      ) {
        return [
          {
            kind: "message",
            label: "Editor state changed; refresh this section",
            icon: "warning",
          },
        ];
      }

      const coordinate = expectedProfileCoordinate(response);
      const nodes: ProfileNode[] = response.page.entries.map((entry) => ({
        kind: "entry",
        entry,
        explanationRequest: {
          context: response.context,
          handle: entry.explanationHandle,
          expectedCoordinate: coordinate,
        },
      }));
      if (offset === 0) {
        nodes.unshift({
          kind: "message",
          label: profileStateLabel(response.state, response.editor),
          icon:
            response.state.kind === "current"
              ? "pass"
              : response.state.kind === "stale"
                ? "history"
                : "error",
          ...(response.state.kind === "current"
            ? {}
            : diagnosticTooltipProperty(response.diagnosticIdentity)),
        });
      }
      if (response.page.nextOffset !== null) {
        nodes.push({
          kind: "more",
          section,
          offset: response.page.nextOffset,
          expectedCoordinate: coordinate,
        });
      }
      return nodes;
    } catch (error: unknown) {
      if (
        cancellation.token.isCancellationRequested ||
        !this.contextEpoch.owns(epoch)
      ) {
        return [];
      }
      const diagnostic = apiDiagnosticFromRequestError(error);
      const capacity = lspCapacityFromRequestError(error);
      return [
        {
          kind: "message",
          label:
            diagnostic?.summary ??
            (capacity === undefined
              ? undefined
              : lspCapacityLines(capacity)[0]) ??
            "Profile inspection transport failed; inspect Pale diagnostics",
          icon: "error",
          ...(diagnostic !== undefined
            ? { tooltip: diagnosticTooltip(diagnostic) }
            : capacity === undefined
              ? {}
              : { tooltip: plainTooltip(lspCapacityLines(capacity)) }),
        },
      ];
    } finally {
      this.activeRequests.delete(cancellation);
      cancellation.dispose();
    }
  }

  private cancelActiveRequests(): void {
    this.activeRequests.cancelAll();
  }
}

function tooltip(entry: EffectiveProfileEntry): vscode.MarkdownString {
  const markdown = new vscode.MarkdownString(undefined, false);
  markdown.appendText(
    [entryLabel(entry), ...entryTooltipLines(entry)].join("\n"),
  );
  markdown.isTrusted = false;
  return markdown;
}

function entryLabel(entry: EffectiveProfileEntry): string {
  switch (entry.kind) {
    case "declared-selection":
      return declaredSelectionLabel(entry.selection);
    case "capability-narrowing":
      return "Capability narrowing";
    case "governing-ceiling":
      return entry.subjectResource;
    case "responsibility":
      return `Responsibility ${entry.contractId}`;
    case "selection":
      return selectionLabel(entry.selection);
    case "repository":
      return `Repository ${entry.repositoryId}`;
    case "provider-repository":
      return `Provider projection ${entry.repositoryId}`;
    case "execution-path":
      return `Execution path ${entry.selectionId}`;
    case "operation":
      return `Operation ${entry.operationId}`;
    case "capability":
      return `Capability ${entry.capabilityId}`;
    case "machine-realization":
      return `Machine ${entry.machineId}`;
    case "finding":
      return findingLabel(entry.finding);
  }
}

function declaredSelectionLabel(
  selection: import("./protocol").EffectiveProfileDeclaredSelection,
): string {
  if (selection.kind === "mode-default") {
    return `Declared ${selection.axis} default ${selection.resourceId}`;
  }
  const selector = selection.selector;
  return selector.kind === "exact"
    ? `Declared ${selection.resourceKind} ${selector.resourceId}`
    : `Declared all ${selection.resourceKind} from ${selector.packageId}`;
}

function selectionLabel(selection: EffectiveProfileSelection): string {
  switch (selection.kind) {
    case "package":
      return `Package ${selection.packageId} ${selection.version}`;
    case "normative-pack":
      return `Normative Pack ${selection.packId} ${selection.version}`;
    case "semantic-resource":
      return `${selection.resourceKind} ${selection.resourceId}`;
    case "run-policy":
      return `Run policy ${selection.runPolicyId}`;
    case "session":
      return `Session ${selection.executionWorkspaceId}`;
    case "lifecycle":
      return `Lifecycle ${selection.lifecycleId}`;
    case "authority-root":
      return `Authority root ${selection.authorityRootId}`;
    case "disclosure-ceiling":
      return `Disclosure ceiling ${selection.authorityCoordinate}`;
  }
}

function findingLabel(finding: EffectiveProfileFinding): string {
  switch (finding.kind) {
    case "native-realization-unknown":
      return `Native realization unknown for ${finding.machineId}`;
    case "operation-unbound":
      return `Operation unbound: ${finding.operationId}`;
    case "requirement":
      return `Requirement ${finding.requirementId}`;
  }
}

function correlatedDiagnosticTooltip(
  identity: string | undefined,
): vscode.MarkdownString | undefined {
  const diagnostic = diagnosticWithIdentity(identity);
  if (diagnostic === undefined) {
    return undefined;
  }
  return diagnosticTooltip(diagnostic);
}

function diagnosticTooltip(
  diagnostic: ApiDiagnosticProjection,
): vscode.MarkdownString {
  return plainTooltip(apiDiagnosticLines(diagnostic));
}

function plainTooltip(lines: ReadonlyArray<string>): vscode.MarkdownString {
  const markdown = new vscode.MarkdownString(undefined, false);
  markdown.appendText(lines.join("\n"));
  markdown.isTrusted = false;
  return markdown;
}

function diagnosticTooltipProperty(
  identity: string | undefined,
): Partial<Pick<MessageNode, "tooltip">> {
  const tooltip = correlatedDiagnosticTooltip(identity);
  return tooltip === undefined ? {} : { tooltip };
}

function diagnosticWithIdentity(
  identity: string | undefined,
): ApiDiagnosticProjection | undefined {
  if (identity === undefined) {
    return undefined;
  }
  for (const [, diagnostics] of vscode.languages.getDiagnostics()) {
    for (const diagnostic of diagnostics) {
      const data = (
        diagnostic as vscode.Diagnostic & { readonly data?: unknown }
      ).data;
      if (
        diagnostic.source === "pale" &&
        opaqueDiagnosticIdentity(data) === identity
      ) {
        return apiDiagnosticFromLspData(data);
      }
    }
  }
  return undefined;
}

function hasAdmittedProfile(
  response: EffectiveProfileResponse,
): response is AdmittedEffectiveProfileResponse {
  return response.state.kind !== "not-admitted";
}
