import * as vscode from "vscode";

import type {
  BaselineDocument,
  WorkspaceSourceConfiguration,
} from "./protocol";

/** One confined source identity prepared before filesystem observation starts. */
export interface ResolvedWorkspaceSource {
  readonly configuration: WorkspaceSourceConfiguration;
  readonly uri: vscode.Uri;
  readonly watchPattern: string;
}

/** Exact configured source set for one resource-scoped workspace session. */
export interface ResolvedWorkspaceSources {
  readonly sources: readonly ResolvedWorkspaceSource[];
  readonly languageByUri: ReadonlyMap<string, WorkspaceSourceConfiguration["languageId"]>;
}

/** Resolve, confine, and deduplicate the complete configured source set. */
export function resolveConfiguredSources(
  root: vscode.Uri,
  sources: readonly WorkspaceSourceConfiguration[],
): ResolvedWorkspaceSources {
  const seen = new Set<string>();
  const resolved: ResolvedWorkspaceSource[] = [];
  const languageByUri = new Map<
    string,
    WorkspaceSourceConfiguration["languageId"]
  >();
  for (const source of sources) {
    const uri = resolveWorkspaceSourceUri(root, source.path);
    const identity = uri.toString();
    if (seen.has(identity)) {
      throw new Error(`Pale source is configured more than once: ${source.path}`);
    }
    seen.add(identity);
    languageByUri.set(identity, source.languageId);
    const rootPath = root.path.endsWith("/") ? root.path : `${root.path}/`;
    resolved.push({
      configuration: source,
      uri,
      watchPattern: escapeGlobPattern(uri.path.slice(rootPath.length)),
    });
  }
  return { sources: resolved, languageByUri };
}

/** Read exact prepared files for the initialization baseline. */
export async function captureConfiguredSources(
  resolved: ResolvedWorkspaceSources,
): Promise<BaselineDocument[]> {
  const documents: BaselineDocument[] = [];
  // Initialization must carry exact bytes, but reading them concurrently would
  // amplify a rejected configuration before the server can apply its canonical
  // role-specific limits. Sequential capture bounds in-flight native reads to one.
  for (const source of resolved.sources) {
    const bytes = await vscode.workspace.fs.readFile(source.uri);
    const text = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
    documents.push({
      uri: source.uri.toString(),
      languageId: source.configuration.languageId,
      role: source.configuration.role,
      text,
    });
  }
  return documents;
}

/** Set the configured editor language when a source document opens. */
export async function applyConfiguredLanguage(
  document: vscode.TextDocument,
  languageByUri: ReadonlyMap<string, WorkspaceSourceConfiguration["languageId"]>,
): Promise<void> {
  const languageId = languageByUri.get(document.uri.toString());
  if (languageId !== undefined && document.languageId !== languageId) {
    await vscode.languages.setTextDocumentLanguage(document, languageId);
  }
}

/**
 * Resolve a configured source and reject URI paths outside the workspace folder.
 * This checks the URI only; it does not resolve filesystem symlinks.
 */
function resolveWorkspaceSourceUri(root: vscode.Uri, sourcePath: string): vscode.Uri {
  if (sourcePath.trim().length === 0) {
    throw new Error("Pale source path must not be empty");
  }
  const candidate = vscode.Uri.joinPath(root, sourcePath);
  const rootPath = root.path.endsWith("/") ? root.path : `${root.path}/`;
  if (
    candidate.scheme !== root.scheme ||
    candidate.authority !== root.authority ||
    !candidate.path.startsWith(rootPath) ||
    candidate.query.length !== 0 ||
    candidate.fragment.length !== 0
  ) {
    throw new Error(`Pale source escapes the workspace folder: ${sourcePath}`);
  }
  return candidate;
}

/** Quote VS Code glob metacharacters so a path keeps its literal meaning. */
export function escapeGlobPattern(path: string): string {
  return path.replace(/[\[\]{}*?]/gu, (character) => `[${character}]`);
}
