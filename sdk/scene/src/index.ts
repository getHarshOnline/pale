/** Headless geometry for Node.js. This entry does not load the native Pale SDK. */
export { GeometryPlan, GeometryError, GeometryExecutionInput, GeometryContactInput } from "./geometry";
export type { Point, Rectangle, PortInput, NodeInput, RelationInput, BundleInput,
  GeometryInput, GeometryLimits, PlacedNode, PlacedPort, PlacedRoute, GeometryResult,
  GeometryOccurrencePair, GeometryReuseResult, BundleMemberRange, SharedRouteCorridor,
  PlacedBundle, GeometryRegionEscalation, GeometryRegionResult, GeometryRegionReady,
  GeometryRegionPreparation, GeometryRegionBoundary, GeometryAlgorithm } from "./geometry";
export { ElkExecutor } from "./elkExecutor";
export type { LayoutLimits, LayoutResult } from "./elkExecutor";
export { ContactError } from "./contactAnalysis";
export type { ContactLimits, ContactSegment, ContactBranch, ContactDepth, EstimatedContact,
  EstimatedOverlap, ContactAnalysis, ContactRejection } from "./contactAnalysis";
export { ContactExecutor } from "./contactExecutor";
export type { ContactResult } from "./contactExecutor";
export { SceneBudget, SceneBudgetError } from "./sceneBudget";
export type { SceneBudgetLimits, SceneBudgetCharges, SceneBudgetResource } from "./sceneBudget";
