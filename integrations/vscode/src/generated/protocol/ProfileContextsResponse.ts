/* Generated from Pale LSP's private Rust wire types. Do not edit. */

/**
 * Exact profile-selection pairs from one retained editor compilation.
 */
export interface ProfileContextsResponse {
  /**
   * Canonically ordered valid project and execution-workspace pairs.
   */
  contexts: PortableWorkspaceProfileContext[];
  /**
   * Editor snapshot that supplied these pairs.
   */
  editor: EditorCoordinateOutput;
}
/**
 * One valid effective-profile context from an admitted workspace topology.
 *
 * The two identities remain separate because a project and an execution
 * workspace have different meanings. This value describes desired topology
 * only; it proves no machine readiness, activation, or native state.
 */
export interface PortableWorkspaceProfileContext {
  executionWorkspaceId: string;
  projectId: string;
}
/**
 * Serialized editor counters, not JavaScript numbers.
 */
export interface EditorCoordinateOutput {
  documentRevision: string;
  generation: string;
}
