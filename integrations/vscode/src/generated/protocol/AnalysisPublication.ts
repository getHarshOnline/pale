/* Generated from Pale LSP's private Rust wire types. Do not edit. */

/**
 * Admission disposition of one accepted analysis publication.
 */
export type AnalysisAdmission = "admitted" | "rejected";

/**
 * Transport-only notice that the retained analysis state changed.
 */
export interface AnalysisPublication {
  /**
   * Whether the canonical compiler admitted that editor state.
   */
  admission: AnalysisAdmission;
  /**
   * Exact editor state accepted by the server scheduler.
   */
  editor: EditorCoordinateOutput;
}
/**
 * Serialized editor counters, not JavaScript numbers.
 */
export interface EditorCoordinateOutput {
  documentRevision: string;
  generation: string;
}
