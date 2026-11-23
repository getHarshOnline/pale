import * as vscode from "vscode";

import type { ConfigurationImprovementAvailability } from "./configurationImprovement";
import type { ProfileNode } from "./profileExplorer";
import type { EffectiveProfileExplanationRequest } from "./protocol";
import type { WorkspaceSession } from "./workspaceSession";
import { sessionKey } from "./workspaceSession";

export type RoutedProfileNode =
  | { readonly kind: "workspace"; readonly sessionKey: string; readonly label: string }
  | { readonly kind: "profile"; readonly sessionKey: string; readonly node: ProfileNode }
  | { readonly kind: "empty" };

/** Exact folder-local explanation request carried by a Tree Item command. */
export interface RoutedExplanation {
  readonly sessionKey: string;
  readonly request: EffectiveProfileExplanationRequest;
}

export type ConfigurationImprovementCommand = "begin" | "evaluate" | "inspect" | "finish";

/** Multiplex one native Tree View over folder-local profile explorers. */
export class ProfileExplorerRouter
  implements vscode.TreeDataProvider<RoutedProfileNode>, vscode.Disposable
{
  private readonly changes = new vscode.EventEmitter<RoutedProfileNode | undefined | null | void>();
  private readonly sessions = new Map<string, WorkspaceSession>();
  private readonly subscriptions = new Map<string, vscode.Disposable>();
  private preferredSessionKey: string | undefined;

  readonly onDidChangeTreeData = this.changes.event;

  add(session: WorkspaceSession): void {
    this.remove(session.key);
    this.sessions.set(session.key, session);
    this.subscriptions.set(
      session.key,
      session.explorer.onDidChangeTreeData(() => this.changes.fire()),
    );
    this.changes.fire();
  }

  remove(key: string): void {
    this.subscriptions.get(key)?.dispose();
    this.subscriptions.delete(key);
    if (this.sessions.delete(key)) {
      if (this.preferredSessionKey === key) {
        this.preferredSessionKey = undefined;
      }
      this.changes.fire();
    }
  }

  prefer(folder: vscode.WorkspaceFolder | undefined): void {
    this.preferredSessionKey = folder === undefined ? undefined : sessionKey(folder);
  }

  refreshPreferred(): void {
    const preferred = this.preferredSession();
    if (preferred !== undefined) {
      preferred.explorer.refresh();
      return;
    }
    for (const session of this.orderedSessions()) {
      session.explorer.refresh();
    }
  }

  async chooseSession(): Promise<WorkspaceSession | undefined> {
    const preferred = this.preferredSession();
    if (preferred !== undefined) {
      return preferred;
    }
    const sessions = this.orderedSessions();
    if (sessions.length === 1) {
      return sessions[0];
    }
    if (sessions.length === 0) {
      return undefined;
    }
    const selected = await vscode.window.showQuickPick(
      sessions.map((session) => ({
        label: session.folder.name,
        description: session.folder.uri.toString(),
        session,
      })),
      {
        title: "Pale Profile Explorer",
        placeHolder: "Select the workspace folder to inspect",
        ignoreFocusOut: true,
      },
    );
    return selected?.session;
  }

  /** Report legal review commands for the preferred folder, or all selectable folders. */
  configurationImprovementAvailability(): ConfigurationImprovementAvailability {
    const preferred = this.preferredSession();
    const sessions = preferred === undefined ? this.orderedSessions() : [preferred];
    return sessions.reduce<ConfigurationImprovementAvailability>(
      (combined, session) => {
        const available = session.review.availability(session.explorer.inspectionContext());
        return {
          canBegin: combined.canBegin || available.canBegin,
          canEvaluate: combined.canEvaluate || available.canEvaluate,
          canInspect: combined.canInspect || available.canInspect,
          canFinish: combined.canFinish || available.canFinish,
        };
      },
      { canBegin: false, canEvaluate: false, canInspect: false, canFinish: false },
    );
  }

  /** Select only a folder for which the requested review transition is legal. */
  async chooseConfigurationImprovementSession(
    command: ConfigurationImprovementCommand,
  ): Promise<WorkspaceSession | undefined> {
    const allowed = (session: WorkspaceSession): boolean => {
      const available = session.review.availability(session.explorer.inspectionContext());
      switch (command) {
        case "begin":
          return available.canBegin;
        case "evaluate":
          return available.canEvaluate;
        case "inspect":
          return available.canInspect;
        case "finish":
          return available.canFinish;
      }
    };
    const preferred = this.preferredSession();
    if (preferred !== undefined) {
      return allowed(preferred) ? preferred : undefined;
    }
    const sessions = this.orderedSessions().filter(allowed);
    if (sessions.length === 1) {
      return sessions[0];
    }
    if (sessions.length === 0) {
      return undefined;
    }
    const selected = await vscode.window.showQuickPick(
      sessions.map((session) => ({
        label: session.folder.name,
        description: session.folder.uri.toString(),
        session,
      })),
      {
        title: "Pale configuration review",
        placeHolder: "Select a workspace folder",
        ignoreFocusOut: true,
      },
    );
    return selected?.session;
  }

  async explain(target: RoutedProfileNode | RoutedExplanation | undefined): Promise<void> {
    const routed = explanationTarget(target);
    if (routed !== undefined) {
      await this.sessions.get(routed.sessionKey)?.explorer.explain(routed.request);
    }
  }

  getTreeItem(node: RoutedProfileNode): vscode.TreeItem {
    if (node.kind === "empty") {
      const item = new vscode.TreeItem(
        "Open a workspace folder to inspect Pale configuration",
        vscode.TreeItemCollapsibleState.None,
      );
      item.id = "pale:workspace:none";
      item.iconPath = new vscode.ThemeIcon("info");
      return item;
    }
    if (node.kind === "workspace") {
      const item = new vscode.TreeItem(node.label, vscode.TreeItemCollapsibleState.Collapsed);
      item.id = `pale:workspace:${node.sessionKey}`;
      item.iconPath = new vscode.ThemeIcon("root-folder");
      return item;
    }
    const session = this.sessions.get(node.sessionKey);
    if (session === undefined) {
      const item = new vscode.TreeItem("Workspace session ended", vscode.TreeItemCollapsibleState.None);
      item.id = `pale:workspace:ended:${node.sessionKey}`;
      item.iconPath = new vscode.ThemeIcon("warning");
      return item;
    }
    const item = session.explorer.getTreeItem(node.node);
    if (node.node.kind === "entry") {
      item.command = {
        command: "pale.profile.explain",
        title: "Explain profile item",
        arguments: [
          {
            sessionKey: node.sessionKey,
            request: node.node.explanationRequest,
          } satisfies RoutedExplanation,
        ],
      };
    }
    return item;
  }

  async getChildren(node?: RoutedProfileNode): Promise<RoutedProfileNode[]> {
    if (node?.kind === "empty") {
      return [];
    }
    if (node?.kind === "profile") {
      return this.profileChildren(node.sessionKey, node.node);
    }
    if (node?.kind === "workspace") {
      return this.profileChildren(node.sessionKey);
    }
    const sessions = this.orderedSessions();
    if (sessions.length === 0) {
      return [{ kind: "empty" }];
    }
    if (sessions.length === 1) {
      const [session] = sessions;
      return session === undefined ? [] : this.profileChildren(session.key);
    }
    return sessions.map((session) => ({
      kind: "workspace",
      sessionKey: session.key,
      label: session.folder.name,
    }));
  }

  dispose(): void {
    for (const subscription of this.subscriptions.values()) {
      subscription.dispose();
    }
    this.subscriptions.clear();
    this.sessions.clear();
    this.changes.dispose();
  }

  private preferredSession(): WorkspaceSession | undefined {
    return this.preferredSessionKey === undefined
      ? undefined
      : this.sessions.get(this.preferredSessionKey);
  }

  private orderedSessions(): WorkspaceSession[] {
    return [...this.sessions.values()].sort(
      (left, right) =>
        left.folder.index - right.folder.index || left.key.localeCompare(right.key),
    );
  }

  private async profileChildren(
    key: string,
    node?: ProfileNode,
  ): Promise<RoutedProfileNode[]> {
    const session = this.sessions.get(key);
    if (session === undefined) {
      return [];
    }
    return (await session.explorer.getChildren(node)).map((child) => ({
      kind: "profile",
      sessionKey: key,
      node: child,
    }));
  }
}

function explanationTarget(
  target: RoutedProfileNode | RoutedExplanation | undefined,
): RoutedExplanation | undefined {
  if (target === undefined) {
    return undefined;
  }
  if ("request" in target) {
    return target;
  }
  if (target.kind === "profile" && target.node.kind === "entry") {
    return {
      sessionKey: target.sessionKey,
      request: target.node.explanationRequest,
    };
  }
  return undefined;
}
