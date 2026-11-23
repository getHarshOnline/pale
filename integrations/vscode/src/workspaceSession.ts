import * as vscode from "vscode";
import {
  LanguageClient,
  RequestType0,
  type LanguageClientOptions,
  type ServerOptions,
} from "vscode-languageclient/node";

import { ProfileExplorer, type ProfileTransport } from "./profileExplorer";
import {
  CONFIGURATION_IMPROVEMENT_METHOD,
  ANALYSIS_PUBLISHED_NOTIFICATION,
  EFFECTIVE_PROFILE_METHOD,
  EFFECTIVE_PROFILE_EXPLAIN_METHOD,
  PROFILE_CONTEXTS_METHOD,
  decodeAnalysisPublication,
  decodeConfigurationImprovementResponse,
  decodeEffectiveProfileExplanationResponse,
  decodeEffectiveProfileResponse,
  decodeProfileContextsResponse,
  decodeServerProtocolAdvertisement,
  type ConfigurationImprovementRequest,
  type EffectiveProfileExplanationRequest,
  type EffectiveProfileRequest,
  type ProfileContextsResponse,
  type WorkspaceSourceConfiguration,
} from "./protocol";
import {
  ConfigurationImprovementReview,
  type ConfigurationImprovementPresentation,
  type ConfigurationImprovementTransport,
} from "./configurationImprovement";
import {
  applyConfiguredLanguage,
  captureConfiguredSources,
  escapeGlobPattern,
  resolveConfiguredSources,
  type ResolvedWorkspaceSource,
} from "./sourceCapture";

// The native profile-context method has no parameters. A string call with null
// is encoded as one positional argument by vscode-jsonrpc.
const PROFILE_CONTEXTS_REQUEST = new RequestType0<unknown, void>(PROFILE_CONTEXTS_METHOD);

const DID_CHANGE_WATCHED_FILES_METHOD = "workspace/didChangeWatchedFiles";
const FILE_CREATED = 1;
const FILE_CHANGED = 2;
const FILE_DELETED = 3;

/** One folder-local language transport and its presentation state. */
export interface WorkspaceSession {
  readonly key: string;
  readonly folder: vscode.WorkspaceFolder;
  readonly client: LanguageClient;
  readonly explorer: ProfileExplorer;
  readonly review: ConfigurationImprovementReview;
  /** Read exact valid profile contexts from this folder's current compilation. */
  readonly profileContexts: () => Promise<ProfileContextsResponse>;
  readonly analysisSubscription: vscode.Disposable;
  readonly languageSubscription: vscode.Disposable;
  readonly sourceSynchronization: vscode.Disposable;
}

/** Typed failure when the server does not match this client's private protocol. */
export class ServerProtocolError extends Error {
  constructor(readonly summary: string) {
    super();
    this.name = "ServerProtocolError";
  }
}

/** Start a folder-local transport; failed startup releases its subscriptions. */
export async function startWorkspaceSession(
  folder: vscode.WorkspaceFolder,
): Promise<WorkspaceSession> {
  const key = sessionKey(folder);
  const configuration = vscode.workspace.getConfiguration("pale", folder.uri);
  const command = configuration.get<string>("server.path", "pale-lsp");
  const rootManifest = configuration.get<string>(
    "workspace.rootManifest",
    "pale-workspace.toml",
  );
  const machineId = configuration.get<string>("workspace.machineId", "");
  const sources = configuration.get<WorkspaceSourceConfiguration[]>("workspace.sources", []);
  const resolved = resolveConfiguredSources(folder.uri, sources);
  const sourceSynchronization = new ConfiguredSourceSynchronization(folder, resolved.sources);
  let client: LanguageClient | undefined;
  let languageSubscription: vscode.Disposable | undefined;
  let review: ConfigurationImprovementReview | undefined;
  try {
    const documents = await captureConfiguredSources(resolved);
    for (const document of vscode.workspace.textDocuments) {
      await applyConfiguredLanguage(document, resolved.languageByUri);
    }
    const rootPath = folder.uri.path.replace(/\/$/u, "");
    const pattern = `${escapeGlobPattern(rootPath)}/**/*`;
    const serverOptions: ServerOptions = {
      command,
      options: { cwd: folder.uri.fsPath },
    };
    const clientOptions: LanguageClientOptions = {
      documentSelector: [
        { language: "pale", scheme: folder.uri.scheme, pattern },
        { language: "pale-toml", scheme: folder.uri.scheme, pattern },
        { language: "pale-page", scheme: folder.uri.scheme, pattern },
      ],
      workspaceFolder: folder,
      initializationOptions: {
        rootManifest,
        ...(machineId.length === 0 ? {} : { machineId }),
        documents,
      },
    };
    client = new LanguageClient(
      `pale-lsp:${key}`,
      `Pale Language Server (${folder.name})`,
      serverOptions,
      clientOptions,
    );
    languageSubscription = vscode.workspace.onDidOpenTextDocument((document) => {
      void applyConfiguredLanguage(document, resolved.languageByUri);
    });
    await client.start();
    verifyServer(client);
    sourceSynchronization.start(client);
    const transport: ProfileTransport & ConfigurationImprovementTransport = {
      effectiveProfile: (
        request: EffectiveProfileRequest,
        token: vscode.CancellationToken,
      ) =>
        client!
          .sendRequest<unknown>(EFFECTIVE_PROFILE_METHOD, request, token)
          .then((value) => decodeEffectiveProfileResponse(value, request)),
      explain: (
        request: EffectiveProfileExplanationRequest,
        token: vscode.CancellationToken,
      ) =>
        client!
          .sendRequest<unknown>(EFFECTIVE_PROFILE_EXPLAIN_METHOD, request, token)
          .then((value) => decodeEffectiveProfileExplanationResponse(value, request)),
      improve: async (
        request: ConfigurationImprovementRequest,
        token: vscode.CancellationToken,
      ) =>
        decodeConfigurationImprovementResponse(
          await client!.sendRequest<unknown>(
            CONFIGURATION_IMPROVEMENT_METHOD,
            request,
            token,
          ),
          request,
        ),
    };
    const explorer = new ProfileExplorer(transport, key);
    const channel = vscode.window.createOutputChannel(
      `Pale Configuration Review (${folder.name})`,
    );
    const presentation: ConfigurationImprovementPresentation = {
      show: (report) => {
        channel.replace(report);
        channel.show(true);
      },
      dispose: () => channel.dispose(),
    };
    review = new ConfigurationImprovementReview(transport, presentation);
    const analysisSubscription = client.onNotification(
      ANALYSIS_PUBLISHED_NOTIFICATION,
      (publication: unknown) => {
        decodeAnalysisPublication(publication);
        explorer.refresh();
      },
    );
    return {
      key,
      folder,
      client,
      explorer,
      review,
      profileContexts: () =>
        client!
          .sendRequest(PROFILE_CONTEXTS_REQUEST)
          .then(decodeProfileContextsResponse),
      analysisSubscription,
      languageSubscription,
      sourceSynchronization,
    };
  } catch (error: unknown) {
    review?.dispose();
    sourceSynchronization.dispose();
    languageSubscription?.dispose();
    if (client === undefined) {
      throw error;
    }
    try {
      await client.stop();
    } catch (cleanup: unknown) {
      throw new AggregateError([error, cleanup], "Language-server startup and cleanup failed");
    }
    throw error;
  }
}

/** Dispose one folder-local presentation and process transport. */
export async function stopWorkspaceSession(session: WorkspaceSession): Promise<void> {
  session.analysisSubscription.dispose();
  session.languageSubscription.dispose();
  session.sourceSynchronization.dispose();
  session.review.dispose();
  session.explorer.dispose();
  await session.client.stop();
}

interface PendingSourceEvent {
  readonly generation: number;
  readonly uri: string;
  readonly type: number;
}

/** Forward only the newest exact-file event without adding another timer or debounce. */
class ConfiguredSourceSynchronization implements vscode.Disposable {
  private readonly subscriptions: vscode.Disposable[] = [];
  private readonly pending = new Map<string, PendingSourceEvent>();
  private transition: Promise<void> = Promise.resolve();
  private client: LanguageClient | undefined;
  private generation = 0;
  private active = true;

  constructor(folder: vscode.WorkspaceFolder, sources: readonly ResolvedWorkspaceSource[]) {
    for (const source of sources) {
      const watcher = vscode.workspace.createFileSystemWatcher(
        new vscode.RelativePattern(folder, source.watchPattern),
      );
      const identity = source.uri.toString();
      this.subscriptions.push(
        watcher,
        watcher.onDidCreate((uri) => this.observe(identity, uri, FILE_CREATED)),
        watcher.onDidChange((uri) => this.observe(identity, uri, FILE_CHANGED)),
        watcher.onDidDelete((uri) => this.observe(identity, uri, FILE_DELETED)),
      );
    }
  }

  start(client: LanguageClient): void {
    this.client = client;
    for (const event of this.pending.values()) {
      this.forward(event);
    }
  }

  dispose(): void {
    this.active = false;
    this.pending.clear();
    for (const subscription of this.subscriptions.splice(0)) {
      subscription.dispose();
    }
  }

  private observe(identity: string, uri: vscode.Uri, type: number): void {
    if (!this.active || uri.toString() !== identity) {
      return;
    }
    this.generation += 1;
    const event = { generation: this.generation, uri: identity, type };
    this.pending.set(identity, event);
    if (this.client !== undefined) {
      this.forward(event);
    }
  }

  private forward(event: PendingSourceEvent): void {
    this.transition = this.transition
      .then(async () => {
        const latest = this.pending.get(event.uri);
        if (!this.active || latest?.generation !== event.generation || this.client === undefined) {
          return;
        }
        this.pending.delete(event.uri);
        await this.client.sendNotification(DID_CHANGE_WATCHED_FILES_METHOD, {
          changes: [{ uri: event.uri, type: event.type }],
        });
      })
      .catch((error: unknown) => {
        if (this.active) {
          void vscode.window.showErrorMessage(
            `Pale could not synchronize a configured source: ${sessionFailureMessage(error)}`,
          );
        }
      });
  }
}

/** Stable process-local map key for one workspace folder. */
export function sessionKey(folder: vscode.WorkspaceFolder): string {
  return folder.uri.toString();
}

/** Bound an untrusted process failure to a presentation-safe explanation. */
export function sessionFailureMessage(error: unknown): string {
  return error instanceof ServerProtocolError
    ? error.summary
    : "language-server process failed; inspect the Pale output for transport details";
}

function verifyServer(languageClient: LanguageClient): void {
  try {
    decodeServerProtocolAdvertisement(
      languageClient.initializeResult?.capabilities.experimental,
    );
  } catch {
    throw new ServerProtocolError(
      "pale-lsp does not advertise the exact 0.0.1 authoring protocol required by this client",
    );
  }
}
