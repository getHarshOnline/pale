/* Generated from Pale LSP's private Rust wire types. Do not edit. */

/**
 * Cancellation proof ceiling advertised by this process leaf.
 */
export type RequestCancellation = "result-suppression";

/**
 * Exact private protocol vocabulary hashed before advertisement.
 */
export interface PaleWorkspaceCapabilities {
  cancellation: RequestCancellation;
  documentRoles: string[];
  inventorySha256: string;
  methods: string[];
  /**
   * @minItems 1
   * @maxItems 1
   */
  notifications: [string];
  packageVersion: string;
  protocolId: string;
  wireFixtureSha256: string;
}
