import * as vscode from "vscode";

import type { ProfileInspectionContext } from "./protocol";
import {
  ProfileExplorerRouter,
  type RoutedExplanation,
  type RoutedProfileNode,
} from "./profileExplorerRouter";
import {
  sessionFailureMessage,
  sessionKey,
  startWorkspaceSession,
  stopWorkspaceSession,
  type WorkspaceSession,
} from "./workspaceSession";

const PROFILE_VIEW_ID = "pale.profile";
const REVIEW_CONTEXT = {
  canBegin: "pale.configurationReview.canBegin",
  canEvaluate: "pale.configurationReview.canEvaluate",
  canInspect: "pale.configurationReview.canInspect",
  canFinish: "pale.configurationReview.canFinish",
} as const;
const SESSION_CONFIGURATION = [
  "pale.server.path",
  "pale.workspace.rootManifest",
  "pale.workspace.machineId",
  "pale.workspace.sources",
] as const;

let runtime: ExtensionRuntime | undefined;

/** Start one independently configured language-server session per workspace folder. */
export async function activate(extensionContext: vscode.ExtensionContext): Promise<void> {
  const predecessor = runtime;
  if (predecessor !== undefined) {
    await predecessor.shutdown();
  }
  const next = new ExtensionRuntime(extensionContext);
  runtime = next;
  await next.start();
}

/** Stop every process transport started by this extension. */
export async function deactivate(): Promise<void> {
  const active = runtime;
  runtime = undefined;
  if (active !== undefined) {
    await active.shutdown();
  }
}

/** Serialized owner for workspace-folder process and presentation lifecycles. */
class ExtensionRuntime implements vscode.Disposable {
  private readonly sessions = new Map<string, WorkspaceSession>();
  private readonly router = new ProfileExplorerRouter();
  private readonly subscriptions: vscode.Disposable[] = [];
  private transition: Promise<void> = Promise.resolve();
  private shutdownPromise: Promise<void> | undefined;
  private stopping = false;

  constructor(private readonly extensionContext: vscode.ExtensionContext) {}

  async start(): Promise<void> {
    this.subscriptions.push(
      vscode.window.registerTreeDataProvider(PROFILE_VIEW_ID, this.router),
      vscode.commands.registerCommand("pale.profile.refresh", () =>
        this.router.refreshPreferred(),
      ),
      vscode.commands.registerCommand("pale.profile.selectContext", async () => {
        const session = await this.router.chooseSession();
        if (session === undefined) {
          return;
        }
        const selected = await selectInspectionContext(
          session,
          session.explorer.inspectionContext(),
        );
        if (selected !== undefined) {
          session.explorer.setInspectionContext(selected);
          await this.updateConfigurationImprovementAvailability();
        }
      }),
      vscode.commands.registerCommand(
        "pale.profile.explain",
        (target: RoutedProfileNode | RoutedExplanation | undefined) =>
          this.router.explain(target),
      ),
      vscode.commands.registerCommand("pale.improvement.begin", () =>
        this.serialize(() => this.beginConfigurationImprovement()),
      ),
      vscode.commands.registerCommand("pale.improvement.evaluate", () =>
        this.serialize(() => this.evaluateConfigurationImprovement()),
      ),
      vscode.commands.registerCommand("pale.improvement.inspect", () =>
        this.serialize(() => this.inspectConfigurationImprovement()),
      ),
      vscode.commands.registerCommand("pale.improvement.finish", () =>
        this.serialize(() => this.finishConfigurationImprovement()),
      ),
      vscode.workspace.onDidChangeWorkspaceFolders((event) => {
        void this.serialize(async () => {
          for (const folder of event.removed) {
            await this.removeSession(sessionKey(folder));
          }
          for (const folder of event.added) {
            await this.addSession(folder);
          }
          await this.preferActiveEditor();
        });
      }),
      vscode.workspace.onDidChangeConfiguration((event) => {
        void this.serialize(async () => {
          for (const folder of vscode.workspace.workspaceFolders ?? []) {
            if (
              SESSION_CONFIGURATION.some((setting) =>
                event.affectsConfiguration(setting, folder.uri),
              )
            ) {
              await this.restartSession(folder);
            }
          }
          await this.preferActiveEditor();
        });
      }),
      vscode.window.onDidChangeActiveTextEditor(() => {
        void this.serialize(() => this.preferActiveEditor());
      }),
    );
    this.extensionContext.subscriptions.push(this);
    await this.serialize(async () => {
      for (const folder of vscode.workspace.workspaceFolders ?? []) {
        await this.addSession(folder);
      }
      await this.preferActiveEditor();
    });
  }

  dispose(): void {
    void this.shutdown().catch((error: unknown) => {
      void vscode.window.showErrorMessage(sessionFailureMessage(error));
    });
  }

  /** Attempt every folder's cleanup, then report any unproven transport stops. */
  shutdown(): Promise<void> {
    if (this.shutdownPromise === undefined) {
      this.stopping = true;
      this.shutdownPromise = this.transition.then(async () => {
        for (const subscription of this.subscriptions.splice(0)) {
          subscription.dispose();
        }
        const failures: unknown[] = [];
        for (const key of [...this.sessions.keys()]) {
          try {
            await this.removeSession(key);
          } catch (error: unknown) {
            failures.push(error);
          }
        }
        this.router.dispose();
        if (failures.length !== 0) {
          throw new AggregateError(failures, "Language-server shutdown was incomplete");
        }
      });
    }
    return this.shutdownPromise;
  }

  private serialize(operation: () => Promise<void>): Promise<void> {
    if (this.stopping) {
      return Promise.resolve();
    }
    this.transition = this.transition.then(operation).catch((error: unknown) => {
      void vscode.window.showErrorMessage(sessionFailureMessage(error));
    });
    return this.transition;
  }

  private async addSession(folder: vscode.WorkspaceFolder): Promise<void> {
    const key = sessionKey(folder);
    if (this.stopping || this.sessions.has(key)) {
      return;
    }
    try {
      const session = await startWorkspaceSession(folder);
      if (this.stopping) {
        await stopWorkspaceSession(session);
        return;
      }
      this.sessions.set(key, session);
      this.router.add(session);
    } catch (error: unknown) {
      await vscode.window.showErrorMessage(
        `Pale could not start for ${folder.name}: ${sessionFailureMessage(error)}`,
      );
    }
  }

  private async removeSession(key: string): Promise<void> {
    const session = this.sessions.get(key);
    if (session === undefined) {
      return;
    }
    this.sessions.delete(key);
    this.router.remove(key);
    await stopWorkspaceSession(session);
  }

  private async restartSession(folder: vscode.WorkspaceFolder): Promise<void> {
    await this.removeSession(sessionKey(folder));
    if (
      (vscode.workspace.workspaceFolders ?? []).some(
        (candidate) => sessionKey(candidate) === sessionKey(folder),
      )
    ) {
      await this.addSession(folder);
    }
  }

  private async preferActiveEditor(): Promise<void> {
    const editor = vscode.window.activeTextEditor;
    this.router.prefer(
      editor === undefined
        ? undefined
        : vscode.workspace.getWorkspaceFolder(editor.document.uri),
    );
    await this.updateConfigurationImprovementAvailability();
  }

  private async beginConfigurationImprovement(): Promise<void> {
    const session = await this.router.chooseConfigurationImprovementSession("begin");
    const context = session?.explorer.inspectionContext();
    if (session === undefined || context === undefined) {
      await vscode.window.showInformationMessage(
        "Select a Pale profile context before starting a configuration review.",
      );
      return;
    }
    await session.review.begin(context);
    await this.updateConfigurationImprovementAvailability();
  }

  private async evaluateConfigurationImprovement(): Promise<void> {
    const session = await this.router.chooseConfigurationImprovementSession("evaluate");
    if (session === undefined) {
      return;
    }
    await session.review.evaluate();
    await this.updateConfigurationImprovementAvailability();
  }

  private async finishConfigurationImprovement(): Promise<void> {
    const session = await this.router.chooseConfigurationImprovementSession("finish");
    if (session === undefined) {
      return;
    }
    const selection = await vscode.window.showQuickPick(
      [
        { label: "Accept preview", disposition: "accept" as const },
        { label: "Reject preview", disposition: "reject" as const },
        { label: "Cancel review", disposition: "cancel" as const },
      ],
      {
        title: "Finish Pale configuration review",
        placeHolder: "This records review state only; no source is changed",
        ignoreFocusOut: true,
      },
    );
    if (selection !== undefined) {
      await session.review.finish(selection.disposition);
      await this.updateConfigurationImprovementAvailability();
    }
  }

  private async inspectConfigurationImprovement(): Promise<void> {
    const session = await this.router.chooseConfigurationImprovementSession("inspect");
    if (session !== undefined) {
      await session.review.inspect();
    }
  }

  private async updateConfigurationImprovementAvailability(): Promise<void> {
    const availability = this.router.configurationImprovementAvailability();
    await Promise.all([
      vscode.commands.executeCommand("setContext", REVIEW_CONTEXT.canBegin, availability.canBegin),
      vscode.commands.executeCommand(
        "setContext",
        REVIEW_CONTEXT.canEvaluate,
        availability.canEvaluate,
      ),
      vscode.commands.executeCommand(
        "setContext",
        REVIEW_CONTEXT.canInspect,
        availability.canInspect,
      ),
      vscode.commands.executeCommand("setContext", REVIEW_CONTEXT.canFinish, availability.canFinish),
    ]);
  }
}

async function selectInspectionContext(
  session: WorkspaceSession,
  current: ProfileInspectionContext | undefined,
): Promise<ProfileInspectionContext | undefined> {
  const initial = await session.profileContexts();
  if (initial.contexts.length === 0) {
    await vscode.window.showInformationMessage(
      "This workspace has no project and execution-workspace pair to inspect.",
    );
    return undefined;
  }
  const projects = [...new Set(initial.contexts.map(({ projectId }) => projectId))];
  const project = await vscode.window.showQuickPick(
    projects.map((projectId) => ({
      label: projectId,
      projectId,
      ...(current?.projectId === projectId ? { description: "Current project" } : {}),
    })),
    {
      title: "Pale Profile Explorer",
      placeHolder: "Select a project from the admitted workspace topology",
      ignoreFocusOut: true,
    },
  );
  if (project === undefined) {
    return undefined;
  }
  const executionWorkspace = await vscode.window.showQuickPick(
    initial.contexts
      .filter(({ projectId }) => projectId === project.projectId)
      .map(({ executionWorkspaceId }) => ({
        label: executionWorkspaceId,
        executionWorkspaceId,
        ...(current?.projectId === project.projectId &&
          current.executionWorkspaceId === executionWorkspaceId
          ? { description: "Current execution workspace" }
          : {}),
      })),
    {
      title: "Pale Profile Explorer",
      placeHolder: `Select an execution workspace for ${project.projectId}`,
      ignoreFocusOut: true,
    },
  );
  if (executionWorkspace === undefined) {
    return undefined;
  }
  const selected = {
    projectId: project.projectId,
    executionWorkspaceId: executionWorkspace.executionWorkspaceId,
  };
  const currentEnumeration = await session.profileContexts();
  if (
    currentEnumeration.editor.generation !== initial.editor.generation ||
    currentEnumeration.editor.documentRevision !== initial.editor.documentRevision ||
    !currentEnumeration.contexts.some(
      ({ projectId, executionWorkspaceId }) =>
        projectId === selected.projectId &&
        executionWorkspaceId === selected.executionWorkspaceId,
    )
  ) {
    await vscode.window.showInformationMessage(
      "The workspace changed while you selected a profile context. Select it again.",
    );
    return undefined;
  }
  return selected;
}
