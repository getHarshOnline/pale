/** Typed transport projections owned by the Pale language server boundary. */

import type { DiagnosticV1 } from "@rtepl/pale-contracts" with { "resolution-mode": "import" };
import type {
  AnalysisPublication as GeneratedAnalysisPublication,
} from "./generated/protocol/AnalysisPublication.js";
import type {
  ConfigurationImprovementResponse as GeneratedConfigurationImprovementResponse,
  ConfigurationImprovementCoordinateChange,
  SubjectSelector as ImprovementSubjectSelector,
} from "./generated/protocol/ConfigurationImprovementResponse.js";
import type {
  EffectiveProfileExplanationResponse as GeneratedEffectiveProfileExplanationResponse,
} from "./generated/protocol/EffectiveProfileExplanationResponse.js";
import type {
  EffectiveProfileResponse as GeneratedEffectiveProfileResponse,
} from "./generated/protocol/EffectiveProfileResponse.js";
import type {
  ProfileContextsResponse as GeneratedProfileContextsResponse,
} from "./generated/protocol/ProfileContextsResponse.js";
import {
  validateAnalysisPublication,
  validateConfigurationImprovementResponse,
  validateEffectiveProfileExplanationResponse,
  validateEffectiveProfileResponse,
  validatePaleWorkspaceCapabilities,
  validateProfileContextsResponse,
} from "./generated/protocol/responses.cjs";

export const EFFECTIVE_PROFILE_METHOD = "pale/workspace/effectiveProfile";
export const PROFILE_CONTEXTS_METHOD = "pale/workspace/profileContexts";
export const EFFECTIVE_PROFILE_EXPLAIN_METHOD = "pale/workspace/effectiveProfileExplain";
export const CONFIGURATION_IMPROVEMENT_METHOD =
  "pale/workspace/configurationImprovement";
export const ANALYSIS_PUBLISHED_NOTIFICATION = "pale/workspace/analysisPublished";
export const PALE_LSP_PROTOCOL_ID = "getharshonline.pale-lsp.workspace";
export const PALE_LSP_PACKAGE_VERSION = "0.0.1";
export const PALE_LSP_PROTOCOL_INVENTORY_SHA256 =
  "fe47491f1260454669fb888235c09745ce4a229ee00856e3ac57a398fcd6f6df";
/** Exact Rust-produced exchange fixture attested by the cross-client tests, not all DTO shapes. */
export const PALE_LSP_WIRE_FIXTURE_SHA256 =
  "25955a504c6470e203a673761d4a911b743ad523adaa87f5961fb249faad6e5c";
export const PALE_LSP_METHODS = [
  "pale/workspace/compilationSummary",
  "pale/workspace/comparisonSummary",
  "pale/workspace/changeRoots",
  "pale/workspace/impacts",
  "pale/workspace/explain",
  PROFILE_CONTEXTS_METHOD,
  EFFECTIVE_PROFILE_METHOD,
  EFFECTIVE_PROFILE_EXPLAIN_METHOD,
  CONFIGURATION_IMPROVEMENT_METHOD,
] as const;
export const PALE_LSP_NOTIFICATIONS = [ANALYSIS_PUBLISHED_NOTIFICATION] as const;
export const PALE_LSP_CANCELLATION = "result-suppression" as const;
export const DOCUMENT_ROLES = [
  "workspace-manifest",
  "workspace-module",
  "workspace-language-primitives",
  "responsibility-derivation",
  "organizational-translation",
  "experience-composition",
  "workspace-resolution-lock",
  "workspace-universe-lock",
  "capabilities",
  "machine-binding",
  "distribution-lock",
  "lifecycle-guidance",
  "package-manifest",
  "package-doctrine",
  "package-operations",
  "package-roles",
  "package-stacks",
  "package-run-modes",
  "package-policy-family",
  "package-governing-order",
  "package-capability-policy",
  "package-tool-recipe",
  "package-page",
  "package-data",
  "package-text",
] as const;

/** Generated contract shape reused for one rendered diagnostic fact. */
export type ApiDiagnosticFact = Readonly<NonNullable<DiagnosticV1["facts"]>[number]>;

/**
 * Fields rendered by the editor from an already-admitted API diagnostic.
 *
 * The policy-bearing strings deliberately remain strings here. This client
 * neither defines their vocabulary nor validates relations between stage,
 * disposition, retry, and effect certainty; it only projects the values carried
 * by LSP.
 */
export interface ApiDiagnosticProjection {
  readonly code: string;
  readonly stage: string;
  readonly disposition: string;
  readonly retry: string;
  readonly effect: string;
  readonly summary: string;
  readonly notes?: ReadonlyArray<string>;
  readonly help?: ReadonlyArray<string>;
  readonly facts?: ReadonlyArray<ApiDiagnosticFact>;
}

/** Explicit, ephemeral profile-selection context chosen by the editor user. */
export interface ProfileInspectionContext {
  projectId: string;
  executionWorkspaceId: string;
}

/** Valid profile contexts from one exact editor compilation. */
export interface ProfileContextsResponse {
  readonly editor: EditorCoordinate;
  readonly contexts: ReadonlyArray<ProfileInspectionContext>;
}

/** One accepted server analysis result that can refresh the folder-local view. */
export type AnalysisPublication = GeneratedAnalysisPublication;

/** Decode an exact, bounded profile-context enumeration. */
export function decodeProfileContextsResponse(value: unknown): ProfileContextsResponse {
  const record = exactRecord(value, "profile-context response");
  exactKeys(record, ["editor", "contexts"], "profile-context response");
  const contexts = decodeArray(
    record.contexts,
    "profile contexts",
    decodeInspectionContext,
  );
  if (contexts.length > 4_096) {
    throw new Error("profile-context response exceeds the compiler input ceiling");
  }
  const identities = new Set(
    contexts.map(({ projectId, executionWorkspaceId }) =>
      JSON.stringify([projectId, executionWorkspaceId]),
    ),
  );
  if (identities.size !== contexts.length) {
    throw new Error("profile-context response contains a duplicate pair");
  }
  if (!validateProfileContextsResponse(value)) {
    throw new Error("invalid profile-context response");
  }
  const response = { editor: decodeEditorCoordinate(record.editor), contexts };
  assertGeneratedResponse<GeneratedProfileContextsResponse>(response);
  return response;
}

/** Admit one server-owned analysis notice before it changes editor presentation. */
export function decodeAnalysisPublication(value: unknown): AnalysisPublication {
  if (!validateAnalysisPublication(value)) {
    throw new Error("invalid analysis publication");
  }
  return {
    editor: decodeEditorCoordinate(value.editor),
    admission: enumValue(value.admission, ["admitted", "rejected"] as const, "analysis admission"),
  };
}

/** Exact editor snapshot coordinate represented by one server response. */
export interface EditorCoordinate {
  /** Decimal process-local generation, encoded as text to avoid JavaScript integer loss. */
  generation: string;
  /** Decimal document-store revision, encoded as text to avoid JavaScript integer loss. */
  documentRevision: string;
}

/** Semantic coordinates that prevent unlike compiler products from being conflated. */
export interface EffectiveProfileCoordinates {
  workspaceId: string;
  projectId: string;
  executionWorkspaceId: string;
  machineId: string;
  profileDigest: string;
  /** Admitted private machine-resolved instance; this is not native-readiness proof. */
  instanceDigest: string;
  selectedTopologyDigest: string;
  workspaceSemanticIrDigest: string;
  workspaceAdmissionEvidenceDigest: string;
  packageSemanticIrDigest: string;
  compositionPlanDigest: string;
}

/** Prior-page coordinate required to keep pagination on one admitted result. */
export interface ExpectedProfileCoordinate extends EditorCoordinate {
  profileDigest: string;
  instanceDigest: string;
  selectedTopologyDigest: string;
  workspaceSemanticIrDigest: string;
  workspaceAdmissionEvidenceDigest: string;
  packageSemanticIrDigest: string;
  compositionPlanDigest: string;
}

/** Compiler source families selected explicitly by workspace configuration. */
export type DocumentRole = (typeof DOCUMENT_ROLES)[number];

/** One role-explicit source selected for local LSP capture. */
export interface WorkspaceSourceConfiguration {
  path: string;
  languageId: "pale" | "pale-toml" | "pale-page";
  role: DocumentRole;
}

/** Exact initialization document sent without parsing or role inference. */
export interface BaselineDocument {
  uri: string;
  languageId: "pale" | "pale-toml" | "pale-page";
  role: DocumentRole;
  text: string;
}

/**
 * Reject unlike private inventories or attested example exchanges before session use.
 * These coordinates are not exhaustive shape reflection; every response still
 * crosses its strict decoder, including variants absent from the fixture.
 */
export function decodeServerProtocolAdvertisement(value: unknown): void {
  const experimental = exactRecord(value, "language-server experimental capabilities");
  const workspace = exactRecord(
    experimental.paleWorkspace,
    "Pale workspace protocol advertisement",
  );
  if (!validatePaleWorkspaceCapabilities(workspace)) {
    throw new Error("invalid Pale workspace protocol advertisement");
  }
  exactKeys(
    workspace,
    [
      "protocolId",
      "packageVersion",
      "inventorySha256",
      "wireFixtureSha256",
      "methods",
      "notifications",
      "cancellation",
      "documentRoles",
    ],
    "Pale workspace protocol advertisement",
  );
  if (
    requiredString(workspace, "protocolId") !== PALE_LSP_PROTOCOL_ID ||
    requiredString(workspace, "packageVersion") !== PALE_LSP_PACKAGE_VERSION ||
    sha256Hex(workspace.inventorySha256, "LSP protocol-inventory digest") !==
      PALE_LSP_PROTOCOL_INVENTORY_SHA256 ||
    sha256Hex(workspace.wireFixtureSha256, "LSP wire-fixture digest") !==
      PALE_LSP_WIRE_FIXTURE_SHA256 ||
    requiredString(workspace, "cancellation") !== PALE_LSP_CANCELLATION
  ) {
    throw new Error("language server advertises an incompatible Pale protocol identity");
  }
  assertExactInventory(
    decodeStringArray(workspace.methods, "LSP custom methods"),
    PALE_LSP_METHODS,
    "LSP custom methods",
  );
  assertExactInventory(
    decodeStringArray(workspace.notifications, "LSP notifications"),
    PALE_LSP_NOTIFICATIONS,
    "LSP notifications",
  );
  assertExactInventory(
    decodeStringArray(workspace.documentRoles, "LSP document roles"),
    DOCUMENT_ROLES,
    "LSP document roles",
  );
}

function assertExactInventory(
  actual: ReadonlyArray<string>,
  expected: ReadonlyArray<string>,
  label: string,
): void {
  if (
    actual.length !== expected.length ||
    actual.some((entry, index) => entry !== expected[index])
  ) {
    throw new Error(`${label} do not match the attested Pale protocol inventory`);
  }
}

/** The five intentionally shallow sections of the native Profile Explorer. */
export type EffectiveProfileSection = (typeof PROFILE_SECTIONS)[number];

/** One bounded request against the current retained compiler handle. */
export interface EffectiveProfileRequest {
  context: ProfileInspectionContext;
  section: EffectiveProfileSection;
  offset: number;
  limit: number;
  expectedCoordinate?: ExpectedProfileCoordinate;
}

/** The current editor input owns an admitted profile. */
export interface CurrentProfileState {
  kind: "current";
}

/** The current input is rejected while a prior admitted profile remains inspectable. */
export interface StaleProfileState {
  kind: "stale";
  admittedEditor: EditorCoordinate;
  admittedProfileDigest: string;
  summary: string;
}

/** No admitted profile may be shown for this inspection context. */
export interface NotAdmittedProfileState {
  kind: "not-admitted";
  summary: string;
}

export type EffectiveProfileState =
  | CurrentProfileState
  | StaleProfileState
  | NotAdmittedProfileState;

/** Existing workspace-proof selectors accepted by `pale/workspace/explain`. */
export type WorkspaceSubjectSelector =
  | { kind: "source"; sourceId: string }
  | { kind: "component"; componentId: string }
  | { kind: "module"; moduleId: string }
  | {
      kind: "declaration";
      ownerModuleId: string;
      namespaceId: string;
      declarationId: string;
    }
  | {
      kind: "import";
      importerModuleId: string;
      upstreamModuleId: string;
      upstreamReleaseId: string;
      exportId: string;
      sourceNamespaceId: string;
      localNamespaceId: string;
    }
  | {
      kind: "extension";
      ownerModuleId: string;
      extensionNamespaceId: string;
      targetNamespaceId: string;
      targetDeclarationId: string;
    }
  | {
      kind: "policy";
      ownerModuleId: string;
      namespaceId: string;
      declarationId: string;
    }
  | { kind: "responsibility-closure" }
  | { kind: "organizational-translation-closure" }
  | { kind: "experience-composition" }
  | { kind: "resource"; ownerModuleId: string; resourceId: string }
  | { kind: "resolution-lock" }
  | { kind: "universe-lock" }
  | { kind: "workspace"; workspaceId: string };

/** Stable display fields supplied by the semantic owner rather than reconstructed here. */
export type EffectClass = (typeof EFFECT_CLASSES)[number];
export type RepositoryAccess = (typeof REPOSITORY_ACCESS)[number];
export type NativeRealizationDisposition = (typeof NATIVE_REALIZATIONS)[number];
export type OperationExposure = (typeof OPERATION_EXPOSURES)[number];
export type ProviderKind = (typeof PROVIDER_KINDS)[number];
export type ResourceKind = (typeof RESOURCE_KINDS)[number];
export type ModeAxis = (typeof MODE_AXES)[number];

/** Opaque terminal provider selected by the compiler-owned path proof. */
export interface EffectiveProfileProviderBinding {
  providerBindingId: string;
  providerBindingIdentity: string;
}

export type EffectiveProfileDeclaredSelector =
  | { kind: "exact"; resourceId: string }
  | { kind: "all-from-package"; packageId: string };

export type EffectiveProfileDeclaredSelection =
  | {
      kind: "resource";
      resourceKind: ResourceKind;
      selector: EffectiveProfileDeclaredSelector;
    }
  | { kind: "mode-default"; axis: ModeAxis; resourceId: string };

/** One exact API-owned selection coordinate. */
export type EffectiveProfileSelection =
  | { kind: "package"; packageId: string; version: string; contentDigest: string }
  | {
      kind: "normative-pack";
      packId: string;
      attestorId: string;
      version: string;
      contentDigest: string;
    }
  | {
      kind: "semantic-resource";
      repositoryId: string;
      resourceKind: ResourceKind;
      resourceId: string;
    }
  | { kind: "run-policy"; runPolicyId: string }
  | {
      kind: "session";
      executionWorkspaceId: string;
      activationRootCount: number;
      additionalRootCount: number;
      activationPathCount: number;
    }
  | { kind: "lifecycle"; lifecycleId: string; eventId: string; actionId: string }
  | { kind: "authority-root"; authorityRootId: string }
  | { kind: "disclosure-ceiling"; authorityCoordinate: string };

export type EffectiveProfileParticipant =
  | { kind: "interface"; interfaceId: string }
  | { kind: "application"; applicationId: string }
  | { kind: "engine"; engineId: string }
  | { kind: "capability"; capabilityId: string }
  | { kind: "terminal-leaf"; terminalLeafId: string }
  | { kind: "plugin"; pluginId: string }
  | { kind: "skill"; skillId: string };

export type EffectiveProfileResponsibility =
  | { kind: "communication" }
  | { kind: "organizational-work" }
  | { kind: "crm-business-state" }
  | { kind: "engineering-execution" }
  | { kind: "context-memory" }
  | { kind: "terminal-harness-execution" }
  | { kind: "native-event-publication" }
  | { kind: "native-effect-read-back" }
  | {
      kind: "extension";
      namespaceId: string;
      contractId: string;
      schemaId: string;
      schemaSha256: string;
    };

export type NarrowingReason = (typeof NARROWING_REASONS)[number];
export type PolicyBlock = (typeof POLICY_BLOCKS)[number];
export type RequirementDisposition =
  | { kind: "satisfied" }
  | { kind: "provider-binding-absent" }
  | { kind: "narrowed"; reason: NarrowingReason }
  | { kind: "blocked"; reason: PolicyBlock };
export type ProviderDisposition =
  | { kind: "bound"; provider: ProviderKind }
  | { kind: "missing-binding" }
  | { kind: "excluded-by-narrowing" }
  | { kind: "effect-mismatch" }
  | { kind: "recipe-contract-mismatch" };

export type EffectiveProfileFinding =
  | { kind: "native-realization-unknown"; machineId: string }
  | { kind: "operation-unbound"; repositoryId: string; operationId: string }
  | {
      kind: "requirement";
      repositoryId: string;
      operationId: string;
      requirementId: string;
      capabilityId: string;
      disposition: RequirementDisposition;
    };

export type EffectiveProfilePathReason =
  | { kind: "origin-receipt"; contractId: string }
  | { kind: "entry-contract"; contractId: string }
  | { kind: "responsibility-contract"; contractId: string }
  | { kind: "provider-binding"; bindingId: string }
  | { kind: "delegation-edge"; edgeId: string }
  | { kind: "termination-contract"; contractId: string }
  | { kind: "settlement-contract"; contractId: string }
  | { kind: "continuity-contract"; contractId: string };

/** One structured row serialized directly by `pale-sdk`. */
export type EffectiveProfileEntry = { explanationHandle: string } & (
  | {
      kind: "declared-selection";
      selection: EffectiveProfileDeclaredSelection;
    }
  | {
      kind: "responsibility";
      contractId: string;
      executionPathId: string;
      participant: EffectiveProfileParticipant;
      responsibilities: EffectiveProfileResponsibility[];
      authorityCeiling: EffectClass[];
      reasonSteps: EffectiveProfilePathReason[];
      subject: WorkspaceSubjectSelector;
    }
  | { kind: "selection"; selection: EffectiveProfileSelection }
  | {
      kind: "capability-narrowing";
      permittedCapabilities: string[];
      forbiddenCapabilities: string[];
      permittedEffects: EffectClass[];
      forbiddenEffects: EffectClass[];
    }
  | {
      kind: "governing-ceiling";
      scope: string;
      subjectResource: string;
      contributors: string[];
      policies: string[];
      permittedCapabilities: string[];
      forbiddenCapabilities: string[];
      permittedEffects: EffectClass[];
      forbiddenEffects: EffectClass[];
    }
  | {
      kind: "repository";
      repositoryId: string;
      materializationOrder: number;
      accessCeiling: RepositoryAccess;
    }
  | {
      kind: "provider-repository";
      repositoryId: string;
      providerBindingId: string;
      nativeRepositoryCoordinate: string;
      nativeRealization: NativeRealizationDisposition;
    }
  | {
      kind: "execution-path";
      selectionId: string;
      experiencePathId: string;
      responsibilityPathIdentity: string;
      responsibilityContractIds: string[];
      workDelegationEdgeIds: string[];
      providerBindingIds: string[];
      capabilityCeiling: string[];
      effectCeiling: EffectClass[];
      /** `null` only for a native-only path with no terminal leaf. */
      terminalProviderBinding: EffectiveProfileProviderBinding | null;
      nativeRealization: NativeRealizationDisposition;
      reasonSteps: EffectiveProfilePathReason[];
      subject: WorkspaceSubjectSelector;
    }
  | {
      kind: "operation";
      repositoryId: string;
      operationId: string;
      exposure: OperationExposure;
      nativeRealization: NativeRealizationDisposition;
    }
  | {
      kind: "capability";
      capabilityId: string;
      repositoryId: string;
      operationId: string;
      requirementId: string;
      effectClass: EffectClass;
      requirementDisposition: RequirementDisposition;
      providerDisposition: ProviderDisposition;
      nativeRealization: NativeRealizationDisposition;
    }
  | {
      kind: "machine-realization";
      machineId: string;
      realization: NativeRealizationDisposition;
    }
  | { kind: "finding"; finding: EffectiveProfileFinding }
);

/** One deterministic, bounded page returned for a single explorer section. */
export interface EffectiveProfilePage {
  section: EffectiveProfileSection;
  offset: number;
  entries: EffectiveProfileEntry[];
  nextOffset: number | null;
}

interface EffectiveProfileResponseBase {
  editor: EditorCoordinate;
  context: ProfileInspectionContext;
  diagnosticIdentity?: string;
}

/** Current or explicitly stale evidence carrying a real admitted profile page. */
export type AdmittedEffectiveProfileResponse = EffectiveProfileResponseBase & {
  state: CurrentProfileState | StaleProfileState;
  coordinates: EffectiveProfileCoordinates;
  page: EffectiveProfilePage;
};

/** Rejected input for which no same-context admitted profile may be shown. */
export type NotAdmittedEffectiveProfileResponse = EffectiveProfileResponseBase & {
  state: NotAdmittedProfileState;
};

/** Profile result bound to exact editor and ephemeral inspection context. */
export type EffectiveProfileResponse =
  | AdmittedEffectiveProfileResponse
  | NotAdmittedEffectiveProfileResponse;

/** Exact coordinate-fenced request for one retained API-owned explanation. */
export interface EffectiveProfileExplanationRequest {
  context: ProfileInspectionContext;
  handle: string;
  expectedCoordinate: ExpectedProfileCoordinate;
}

export type EffectiveProfileExplanationEvidence =
  | { kind: "declared-selection"; selection: EffectiveProfileDeclaredSelection }
  | {
      kind: "responsibility";
      contractId: string;
      executionPathId: string;
      reasonSteps: EffectiveProfilePathReason[];
    }
  | { kind: "effective-selection"; selection: EffectiveProfileSelection }
  | { kind: "execution-path"; selectionId: string; reasonSteps: EffectiveProfilePathReason[] }
  | {
      kind: "operation-classification";
      exposure: OperationExposure;
      nativeRealization: NativeRealizationDisposition;
    }
  | {
      kind: "capability-classification";
      requirementDisposition: RequirementDisposition;
      providerDisposition: ProviderDisposition;
      nativeRealization: NativeRealizationDisposition;
    }
  | { kind: "finding-classification"; finding: EffectiveProfileFinding }
  | { kind: "machine-realization"; realization: NativeRealizationDisposition }
  | { kind: "configuration" };

/** API-owned bounded explanation returned from the sole retained compilation. */
export interface EffectiveProfileExplanationResponse {
  editor: EditorCoordinate;
  context: ProfileInspectionContext;
  coordinates: EffectiveProfileCoordinates;
  explanation: {
    handle: string;
    evidence: EffectiveProfileExplanationEvidence;
  };
}

/** Begin one review from the exact current profile/editor product. */
export interface BeginConfigurationImprovementRequest {
  action: "begin";
  context: ProfileInspectionContext;
  expectedProfile: ExpectedProfileCoordinate;
}

/** Evaluate the current editor snapshot against a retained review baseline. */
export interface EvaluateConfigurationImprovementRequest {
  action: "evaluate";
  context: ProfileInspectionContext;
  expectedBaseline: ExpectedProfileCoordinate;
}

/** Select one bounded view of an exact retained preview. */
export type ConfigurationImprovementDetailRequest =
  | {
      kind: "overview";
      rootsOffset: number;
      impactsOffset: number;
      limit: number;
    }
  | {
      kind: "subject";
      subject: ImprovementSubjectSelector;
      inputLimit: number;
      maximumInspected: number;
      maximumReturned: number;
    };

export type ConfigurationImprovementReviewDisposition = "accept" | "reject" | "cancel";

/** Exact preview product required before a terminal review decision. */
export type ConfigurationImprovementPreviewCoordinate =
  Extract<GeneratedConfigurationImprovementResponse, { state: "previewed" }>["coordinate"];

/** Inspect evidence retained by one exact preview without evaluating it again. */
export interface InspectConfigurationImprovementRequest {
  action: "inspect";
  context: ProfileInspectionContext;
  expectedPreview: ConfigurationImprovementPreviewCoordinate;
  detail: ConfigurationImprovementDetailRequest;
}

/** Consume one preview into an explicit non-effectful disposition. */
export interface FinishConfigurationImprovementRequest {
  action: "finish";
  context: ProfileInspectionContext;
  expectedPreview: ConfigurationImprovementPreviewCoordinate;
  disposition: ConfigurationImprovementReviewDisposition;
}

export type ConfigurationImprovementRequest =
  | BeginConfigurationImprovementRequest
  | EvaluateConfigurationImprovementRequest
  | InspectConfigurationImprovementRequest
  | FinishConfigurationImprovementRequest;

/** Bounded transport projection of the API-owned preview. */
export type ConfigurationImprovementPreview =
  Extract<GeneratedConfigurationImprovementResponse, { state: "previewed" }>["preview"];

export type ConfigurationImprovementResponse = GeneratedConfigurationImprovementResponse;
export type ConfigurationImprovementOverview = Extract<
  GeneratedConfigurationImprovementResponse,
  { state: "details" }
>;
export type ConfigurationImprovementSubjectDetails = Extract<
  GeneratedConfigurationImprovementResponse,
  { state: "subject-details" }
>;
export type ConfigurationImprovementSubject = ImprovementSubjectSelector;

/** Decode and fence one profile page against the complete originating request. */
export function decodeEffectiveProfileResponse(
  value: unknown,
  request: EffectiveProfileRequest,
): EffectiveProfileResponse {
  const requestOffset = u32(request.offset, "effective-profile request offset");
  const requestLimit = normalizedProfilePageLimit(request.limit);
  if (requestOffset > 0 && request.expectedCoordinate === undefined) {
    throw new Error("effective-profile continuation requires its prior coordinate");
  }
  const record = exactRecord(value, "effective-profile response");
  const state = decodeEffectiveProfileState(record.state);
  const editor = decodeEditorCoordinate(record.editor);
  const context = decodeInspectionContext(record.context);
  const diagnosticIdentity = optionalSha256Hex(record, "diagnosticIdentity");
  assertSameInspectionContext(context, request.context, "effective-profile response");

  if (state.kind === "not-admitted") {
    exactKeysAllowing(
      record,
      ["state", "editor", "context"],
      ["diagnosticIdentity"],
      "not-admitted effective-profile response",
    );
    const response: NotAdmittedEffectiveProfileResponse = {
      state,
      editor,
      context,
      ...(diagnosticIdentity === undefined ? {} : { diagnosticIdentity }),
    };
    if (!validateEffectiveProfileResponse(value)) {
      throw new Error("invalid effective-profile response");
    }
    assertGeneratedResponse<GeneratedEffectiveProfileResponse>(response);
    return response;
  }

  exactKeysAllowing(
    record,
    ["state", "editor", "context", "coordinates", "page"],
    state.kind === "stale" ? ["diagnosticIdentity"] : [],
    "admitted effective-profile response",
  );
  const coordinates = decodeEffectiveProfileCoordinates(record.coordinates);
  assertProfileProduct(context, coordinates, editor, state);
  if (
    request.expectedCoordinate !== undefined &&
    !profileProductMatchesExpected(editor, coordinates, request.expectedCoordinate)
  ) {
    throw new Error("effective-profile response does not match its continuation coordinate");
  }
  const page = decodeEffectiveProfilePage(record.page, requestLimit);
  if (page.section !== request.section) {
    throw new Error("effective-profile response section does not match its request");
  }
  if (page.offset !== requestOffset) {
    throw new Error("effective-profile response offset does not match its request");
  }
  if (
    page.nextOffset !== null &&
    (page.entries.length !== requestLimit ||
      page.nextOffset !== requestOffset + page.entries.length)
  ) {
    throw new Error("effective-profile response contains a non-contiguous page coordinate");
  }
  const response: AdmittedEffectiveProfileResponse = {
    state,
    editor,
    context,
    coordinates,
    page,
    ...(diagnosticIdentity === undefined ? {} : { diagnosticIdentity }),
  };
  if (!validateEffectiveProfileResponse(value)) {
    throw new Error("invalid effective-profile response");
  }
  assertGeneratedResponse<GeneratedEffectiveProfileResponse>(response);
  return response;
}

/** Decode one retained explanation only when every coordinate remains fenced. */
export function decodeEffectiveProfileExplanationResponse(
  value: unknown,
  request: EffectiveProfileExplanationRequest,
): EffectiveProfileExplanationResponse {
  const record = exactRecord(value, "effective-profile explanation response");
  exactKeys(record, ["editor", "context", "coordinates", "explanation"], "effective-profile explanation response");
  const editor = decodeEditorCoordinate(record.editor);
  const context = decodeInspectionContext(record.context);
  const coordinates = decodeEffectiveProfileCoordinates(record.coordinates);
  const explanation = decodeEffectiveProfileExplanation(record.explanation);
  assertSameInspectionContext(context, request.context, "effective-profile explanation response");
  assertProfileProduct(context, coordinates, editor);
  if (!profileProductMatchesExpected(editor, coordinates, request.expectedCoordinate)) {
    throw new Error("effective-profile explanation does not match its requested coordinate");
  }
  if (explanation.handle !== request.handle) {
    throw new Error("effective-profile explanation handle does not match its request");
  }
  if (!validateEffectiveProfileExplanationResponse(value)) {
    throw new Error("invalid effective-profile explanation response");
  }
  const response = { editor, context, coordinates, explanation };
  assertGeneratedResponse<GeneratedEffectiveProfileExplanationResponse>(response);
  return response;
}

/** Reject malformed server values before they enter editor presentation state. */
export function decodeConfigurationImprovementResponse(
  value: unknown,
  request: ConfigurationImprovementRequest,
): ConfigurationImprovementResponse {
  if (!validateConfigurationImprovementResponse(value)) {
    throw new Error("invalid configuration-improvement response");
  }
  const record = value;
  switch (record.state) {
    case "explained": {
      const response = {
        state: "explained",
        context: decodeInspectionContext(record.context),
        baseline: decodeExpectedProfileCoordinate(record.baseline),
      } as const;
      assertImprovementResponseMatchesRequest(response, request);
      return response;
    }
    case "previewed": {
      const response = checkedConfigurationImprovementProduct({
        state: "previewed",
        context: decodeInspectionContext(record.context),
        coordinate: decodeImprovementPreviewCoordinate(record.coordinate),
        preview: decodeImprovementPreview(record.preview),
      });
      assertImprovementResponseMatchesRequest(response, request);
      return response;
    }
    case "details": {
      const response = checkedConfigurationImprovementCoordinate({
        state: "details",
        context: decodeInspectionContext(record.context),
        coordinate: decodeImprovementPreviewCoordinate(record.coordinate),
        changeRoots: structuredClone(record.changeRoots),
        impacts: structuredClone(record.impacts),
      });
      assertImprovementResponseMatchesRequest(response, request);
      return response;
    }
    case "subject-details": {
      const response = checkedConfigurationImprovementCoordinate({
        state: "subject-details",
        context: decodeInspectionContext(record.context),
        coordinate: decodeImprovementPreviewCoordinate(record.coordinate),
        explanation: structuredClone(record.explanation),
        source: structuredClone(record.source),
      });
      assertImprovementResponseMatchesRequest(response, request);
      return response;
    }
    case "terminal": {
      const response = checkedConfigurationImprovementProduct({
        state: "terminal",
        context: decodeInspectionContext(record.context),
        disposition: record.disposition,
        coordinate: decodeImprovementPreviewCoordinate(record.coordinate),
        preview: decodeImprovementPreview(record.preview),
      });
      assertImprovementResponseMatchesRequest(response, request);
      return response;
    }
  }
}

function assertImprovementResponseMatchesRequest(
  response: ConfigurationImprovementResponse,
  request: ConfigurationImprovementRequest,
): void {
  assertSameInspectionContext(
    response.context,
    request.context,
    "configuration-improvement response",
  );
  if (
    request.action === "begin" &&
    response.state === "explained" &&
    sameExpectedProfileCoordinate(response.baseline, request.expectedProfile)
  ) {
    return;
  }
  if (
    request.action === "evaluate" &&
    response.state === "previewed" &&
    sameExpectedProfileCoordinate(response.coordinate.baseline, request.expectedBaseline)
  ) {
    return;
  }
  if (
    request.action === "inspect" &&
    (response.state === "details" || response.state === "subject-details") &&
    sameConfigurationImprovementPreviewCoordinate(
      response.coordinate,
      request.expectedPreview,
    )
  ) {
    if (
      (request.detail.kind === "overview" && response.state === "details") ||
      (request.detail.kind === "subject" &&
        response.state === "subject-details" &&
        sameImprovementSubject(response.explanation.subject, request.detail.subject) &&
        sameImprovementSubject(response.source.subject, request.detail.subject))
    ) {
      assertImprovementDetailBounds(response, request.detail);
      return;
    }
  }
  if (
    request.action === "finish" &&
    response.state === "terminal" &&
    sameConfigurationImprovementPreviewCoordinate(
      response.coordinate,
      request.expectedPreview,
    )
  ) {
    const expected = {
      accept: "accepted",
      reject: "rejected",
      cancel: "cancelled",
    } as const;
    if (response.disposition === expected[request.disposition] || response.disposition === "stale") {
      return;
    }
  }
  throw new Error("configuration-improvement response does not match its request");
}

/** Compare two complete profile-continuation coordinates without reinterpreting them. */
export function sameExpectedProfileCoordinate(
  left: ExpectedProfileCoordinate,
  right: ExpectedProfileCoordinate,
): boolean {
  return (
    left.generation === right.generation &&
    left.documentRevision === right.documentRevision &&
    left.profileDigest === right.profileDigest &&
    left.instanceDigest === right.instanceDigest &&
    left.selectedTopologyDigest === right.selectedTopologyDigest &&
    left.workspaceSemanticIrDigest === right.workspaceSemanticIrDigest &&
    left.workspaceAdmissionEvidenceDigest === right.workspaceAdmissionEvidenceDigest &&
    left.packageSemanticIrDigest === right.packageSemanticIrDigest &&
    left.compositionPlanDigest === right.compositionPlanDigest
  );
}

/** Compare two complete configuration-preview coordinates. */
export function sameConfigurationImprovementPreviewCoordinate(
  left: ConfigurationImprovementPreviewCoordinate,
  right: ConfigurationImprovementPreviewCoordinate,
): boolean {
  return (
    sameExpectedProfileCoordinate(left.baseline, right.baseline) &&
    left.candidateEditor.generation === right.candidateEditor.generation &&
    left.candidateEditor.documentRevision === right.candidateEditor.documentRevision &&
    left.candidateSelectedTopologyDigest === right.candidateSelectedTopologyDigest &&
    left.candidateProfileDigest === right.candidateProfileDigest &&
    left.candidateInstanceDigest === right.candidateInstanceDigest &&
    left.derivationReceiptDigest === right.derivationReceiptDigest
  );
}

function checkedConfigurationImprovementProduct<T extends ConfigurationImprovementResponse>(
  response: T,
): T {
  if (response.state === "explained") {
    return response;
  }
  if (response.state === "details" || response.state === "subject-details") {
    checkedConfigurationImprovementCoordinate(response);
    return response;
  }
  const { context, coordinate, preview } = response;
  if (
    preview.projectId !== context.projectId ||
    preview.executionWorkspaceId !== context.executionWorkspaceId ||
    preview.profileDigest !== coordinate.baseline.profileDigest ||
    preview.instanceDigest !== coordinate.baseline.instanceDigest ||
    preview.candidateProfileDigest !== coordinate.candidateProfileDigest ||
    preview.candidateInstanceDigest !== coordinate.candidateInstanceDigest ||
    preview.derivationReceiptDigest !== coordinate.derivationReceiptDigest
  ) {
    throw new Error("configuration-improvement response contains mismatched coordinates");
  }
  checkedConfigurationImprovementCoordinate(response);
  return response;
}

function checkedConfigurationImprovementCoordinate<T extends Exclude<
  ConfigurationImprovementResponse,
  { state: "explained" }
>>(response: T): T {
  const { coordinate } = response;
  if (
    BigInt(coordinate.candidateEditor.generation) <
      BigInt(coordinate.baseline.generation) ||
    BigInt(coordinate.candidateEditor.documentRevision) <
      BigInt(coordinate.baseline.documentRevision)
  ) {
    throw new Error(
      "configuration-improvement candidate must not precede its baseline editor product",
    );
  }
  return response;
}

function sameImprovementSubject(
  left: ImprovementSubjectSelector,
  right: ImprovementSubjectSelector,
): boolean {
  return improvementSubjectIdentity(left) === improvementSubjectIdentity(right);
}

function improvementSubjectIdentity(subject: ImprovementSubjectSelector): string {
  switch (subject.kind) {
    case "source":
      return JSON.stringify([subject.kind, subject.sourceId]);
    case "component":
      return JSON.stringify([subject.kind, subject.componentId]);
    case "module":
      return JSON.stringify([subject.kind, subject.moduleId]);
    case "declaration":
    case "policy":
      return JSON.stringify([
        subject.kind,
        subject.ownerModuleId,
        subject.namespaceId,
        subject.declarationId,
      ]);
    case "import":
      return JSON.stringify([
        subject.kind,
        subject.importerModuleId,
        subject.upstreamModuleId,
        subject.upstreamReleaseId,
        subject.exportId,
        subject.sourceNamespaceId,
        subject.localNamespaceId,
      ]);
    case "extension":
      return JSON.stringify([
        subject.kind,
        subject.ownerModuleId,
        subject.extensionNamespaceId,
        subject.targetNamespaceId,
        subject.targetDeclarationId,
      ]);
    case "resource":
      return JSON.stringify([subject.kind, subject.ownerModuleId, subject.resourceId]);
    case "workspace":
      return JSON.stringify([subject.kind, subject.workspaceId]);
    case "responsibility-closure":
    case "organizational-translation-closure":
    case "experience-composition":
    case "resolution-lock":
    case "universe-lock":
      return subject.kind;
  }
}

function assertImprovementDetailBounds(
  response: ConfigurationImprovementOverview | ConfigurationImprovementSubjectDetails,
  request: ConfigurationImprovementDetailRequest,
): void {
  if (request.kind === "overview" && response.state === "details") {
    const limit = normalizedImprovementDetailLimit(request.limit);
    assertReviewPage(
      response.changeRoots.entries.length,
      response.changeRoots.nextOffset,
      request.rootsOffset,
      limit,
      "change-root",
    );
    assertReviewPage(
      response.impacts.entries.length,
      response.impacts.nextOffset,
      request.impactsOffset,
      limit,
      "impact",
    );
    for (const root of response.changeRoots.entries) {
      sha256Hex(root.rootDigest, "change-root digest");
    }
    return;
  }
  if (request.kind === "subject" && response.state === "subject-details") {
    if (
      response.source.occurrences.length >
        normalizedImprovementDetailLimit(request.maximumReturned) ||
      response.source.inspected >
        normalizedImprovementDetailLimit(request.maximumInspected)
    ) {
      throw new Error("configuration-improvement source result exceeds its requested bounds");
    }
    if ((response.explanation.causal?.steps.length ?? 0) > 64) {
      throw new Error("configuration-improvement explanation exceeds its 64-step bound");
    }
    if (response.explanation.causal !== undefined) {
      sha256Hex(response.explanation.causal.root.rootDigest, "causal root digest");
      for (const step of response.explanation.causal.steps) {
        sha256Hex(step.nodeDigest, "causal node digest");
      }
    }
  }
}

function normalizedImprovementDetailLimit(requested: number): number {
  const limit = u16(requested, "configuration-improvement detail limit");
  return limit === 0 || limit > 64 ? 64 : limit;
}

function assertReviewPage(
  length: number,
  nextOffset: number | null,
  requestedOffset: number,
  limit: number,
  label: string,
): void {
  const offset = u32(requestedOffset, `${label} offset`);
  if (length > limit) {
    throw new Error(`${label} page exceeds its requested bound`);
  }
  if (
    nextOffset !== null &&
    (length !== limit || nextOffset !== offset + length)
  ) {
    throw new Error(`${label} page contains a non-contiguous continuation`);
  }
}

function decodeEffectiveProfileState(value: unknown): EffectiveProfileState {
  const record = exactRecord(value, "effective-profile state");
  switch (requiredString(record, "kind")) {
    case "current":
      exactKeys(record, ["kind"], "current effective-profile state");
      return { kind: "current" };
    case "stale":
      exactKeys(
        record,
        ["kind", "admittedEditor", "admittedProfileDigest", "summary"],
        "stale effective-profile state",
      );
      return {
        kind: "stale",
        admittedEditor: decodeEditorCoordinate(record.admittedEditor),
        admittedProfileDigest: sha256Hex(record.admittedProfileDigest, "admitted profile digest"),
        summary: requiredString(record, "summary"),
      };
    case "not-admitted":
      exactKeys(record, ["kind", "summary"], "not-admitted effective-profile state");
      return { kind: "not-admitted", summary: requiredString(record, "summary") };
    default:
      throw new Error("unknown effective-profile state");
  }
}

function decodeEffectiveProfileCoordinates(value: unknown): EffectiveProfileCoordinates {
  const record = exactRecord(value, "effective-profile coordinates");
  exactKeys(
    record,
    [
      "workspaceId",
      "projectId",
      "executionWorkspaceId",
      "machineId",
      "profileDigest",
      "instanceDigest",
      "selectedTopologyDigest",
      "workspaceSemanticIrDigest",
      "workspaceAdmissionEvidenceDigest",
      "packageSemanticIrDigest",
      "compositionPlanDigest",
    ],
    "effective-profile coordinates",
  );
  return {
    workspaceId: requiredString(record, "workspaceId"),
    projectId: requiredString(record, "projectId"),
    executionWorkspaceId: requiredString(record, "executionWorkspaceId"),
    machineId: requiredString(record, "machineId"),
    profileDigest: sha256Hex(record.profileDigest, "profile digest"),
    instanceDigest: sha256Hex(record.instanceDigest, "instance digest"),
    selectedTopologyDigest: sha256Hex(record.selectedTopologyDigest, "topology digest"),
    workspaceSemanticIrDigest: sha256Hex(
      record.workspaceSemanticIrDigest,
      "workspace semantic IR digest",
    ),
    workspaceAdmissionEvidenceDigest: sha256Hex(
      record.workspaceAdmissionEvidenceDigest,
      "workspace admission evidence digest",
    ),
    packageSemanticIrDigest: sha256Hex(
      record.packageSemanticIrDigest,
      "Package semantic IR digest",
    ),
    compositionPlanDigest: sha256Hex(record.compositionPlanDigest, "composition-plan digest"),
  };
}

function decodeEffectiveProfilePage(value: unknown, maximumEntries: number): EffectiveProfilePage {
  const record = exactRecord(value, "effective-profile page");
  exactKeys(
    record,
    ["section", "offset", "entries", "nextOffset"],
    "effective-profile page",
  );
  if (!Array.isArray(record.entries)) {
    throw new Error("effective-profile entries must be an array");
  }
  if (record.entries.length > maximumEntries) {
    throw new Error("effective-profile response exceeds its requested page window");
  }
  const entries = record.entries.map(decodeEffectiveProfileEntry);
  return {
    section: enumValue(record.section, PROFILE_SECTIONS, "effective-profile section"),
    offset: u32(record.offset, "effective-profile page offset"),
    entries,
    nextOffset: record.nextOffset === null
      ? null
      : u32(record.nextOffset, "effective-profile next offset"),
  };
}

function decodeEffectiveProfileEntry(value: unknown): EffectiveProfileEntry {
  const record = exactRecord(value, "effective-profile entry");
  const explanationHandle = sha256Hex(
    record.explanationHandle,
    "effective-profile explanation handle",
  );
  switch (requiredString(record, "kind")) {
    case "declared-selection":
      exactKeys(record, ["explanationHandle", "kind", "selection"], "declared-selection entry");
      return {
        explanationHandle,
        kind: "declared-selection",
        selection: decodeDeclaredSelection(record.selection),
      };
    case "responsibility":
      exactKeys(
        record,
        ["explanationHandle", "kind", "contractId", "executionPathId", "participant", "responsibilities", "authorityCeiling", "reasonSteps", "subject"],
        "responsibility entry",
      );
      return {
        explanationHandle,
        kind: "responsibility",
        contractId: requiredString(record, "contractId"),
        executionPathId: requiredString(record, "executionPathId"),
        participant: decodeParticipant(record.participant),
        responsibilities: decodeArray(record.responsibilities, "responsibilities", decodeResponsibility),
        authorityCeiling: decodeEnumArray(record.authorityCeiling, EFFECT_CLASSES, "authority ceiling"),
        reasonSteps: decodeArray(record.reasonSteps, "responsibility reasons", decodePathReason),
        subject: decodeSubject(record.subject),
      };
    case "selection":
      exactKeys(record, ["explanationHandle", "kind", "selection"], "selection entry");
      return { explanationHandle, kind: "selection", selection: decodeSelection(record.selection) };
    case "capability-narrowing":
      exactKeys(
        record,
        ["explanationHandle", "kind", "permittedCapabilities", "forbiddenCapabilities", "permittedEffects", "forbiddenEffects"],
        "capability-narrowing entry",
      );
      return {
        explanationHandle,
        kind: "capability-narrowing",
        permittedCapabilities: decodeStringArray(record.permittedCapabilities, "permitted capabilities"),
        forbiddenCapabilities: decodeStringArray(record.forbiddenCapabilities, "forbidden capabilities"),
        permittedEffects: decodeEnumArray(record.permittedEffects, EFFECT_CLASSES, "permitted effects"),
        forbiddenEffects: decodeEnumArray(record.forbiddenEffects, EFFECT_CLASSES, "forbidden effects"),
      };
    case "governing-ceiling":
      exactKeys(
        record,
        ["explanationHandle", "kind", "scope", "subjectResource", "contributors", "policies", "permittedCapabilities", "forbiddenCapabilities", "permittedEffects", "forbiddenEffects"],
        "governing-ceiling entry",
      );
      return {
        explanationHandle,
        kind: "governing-ceiling",
        scope: requiredString(record, "scope"),
        subjectResource: requiredString(record, "subjectResource"),
        contributors: decodeStringArray(record.contributors, "governing contributors"),
        policies: decodeStringArray(record.policies, "governing policies"),
        permittedCapabilities: decodeStringArray(record.permittedCapabilities, "permitted capabilities"),
        forbiddenCapabilities: decodeStringArray(record.forbiddenCapabilities, "forbidden capabilities"),
        permittedEffects: decodeEnumArray(record.permittedEffects, EFFECT_CLASSES, "permitted effects"),
        forbiddenEffects: decodeEnumArray(record.forbiddenEffects, EFFECT_CLASSES, "forbidden effects"),
      };
    case "repository":
      exactKeys(record, ["explanationHandle", "kind", "repositoryId", "materializationOrder", "accessCeiling"], "repository entry");
      return {
        explanationHandle,
        kind: "repository",
        repositoryId: requiredString(record, "repositoryId"),
        materializationOrder: u32(record.materializationOrder, "repository materialization order"),
        accessCeiling: enumValue(record.accessCeiling, REPOSITORY_ACCESS, "repository access ceiling"),
      };
    case "provider-repository":
      exactKeys(record, ["explanationHandle", "kind", "repositoryId", "providerBindingId", "nativeRepositoryCoordinate", "nativeRealization"], "provider-repository entry");
      return {
        explanationHandle,
        kind: "provider-repository",
        repositoryId: requiredString(record, "repositoryId"),
        providerBindingId: requiredString(record, "providerBindingId"),
        nativeRepositoryCoordinate: requiredString(record, "nativeRepositoryCoordinate"),
        nativeRealization: decodeNativeRealization(record.nativeRealization),
      };
    case "execution-path":
      exactKeys(
        record,
        ["explanationHandle", "kind", "selectionId", "experiencePathId", "responsibilityPathIdentity", "responsibilityContractIds", "workDelegationEdgeIds", "providerBindingIds", "capabilityCeiling", "effectCeiling", "terminalProviderBinding", "nativeRealization", "reasonSteps", "subject"],
        "execution-path entry",
      );
      return {
        explanationHandle,
        kind: "execution-path",
        selectionId: requiredString(record, "selectionId"),
        experiencePathId: requiredString(record, "experiencePathId"),
        responsibilityPathIdentity: sha256Hex(
          record.responsibilityPathIdentity,
          "responsibility-path identity",
        ),
        responsibilityContractIds: decodeStringArray(record.responsibilityContractIds, "responsibility contracts"),
        workDelegationEdgeIds: decodeStringArray(record.workDelegationEdgeIds, "work-delegation edges"),
        providerBindingIds: decodeStringArray(record.providerBindingIds, "provider bindings"),
        capabilityCeiling: decodeStringArray(record.capabilityCeiling, "capability ceiling"),
        effectCeiling: decodeEnumArray(record.effectCeiling, EFFECT_CLASSES, "effect ceiling"),
        terminalProviderBinding: decodeProviderBinding(record.terminalProviderBinding),
        nativeRealization: decodeNativeRealization(record.nativeRealization),
        reasonSteps: decodeArray(record.reasonSteps, "execution-path reasons", decodePathReason),
        subject: decodeSubject(record.subject),
      };
    case "operation":
      exactKeys(record, ["explanationHandle", "kind", "repositoryId", "operationId", "exposure", "nativeRealization"], "operation entry");
      return {
        explanationHandle,
        kind: "operation",
        repositoryId: requiredString(record, "repositoryId"),
        operationId: requiredString(record, "operationId"),
        exposure: enumValue(record.exposure, OPERATION_EXPOSURES, "operation exposure"),
        nativeRealization: decodeNativeRealization(record.nativeRealization),
      };
    case "capability":
      exactKeys(record, ["explanationHandle", "kind", "capabilityId", "repositoryId", "operationId", "requirementId", "effectClass", "requirementDisposition", "providerDisposition", "nativeRealization"], "capability entry");
      return {
        explanationHandle,
        kind: "capability",
        capabilityId: requiredString(record, "capabilityId"),
        repositoryId: requiredString(record, "repositoryId"),
        operationId: requiredString(record, "operationId"),
        requirementId: requiredString(record, "requirementId"),
        effectClass: enumValue(record.effectClass, EFFECT_CLASSES, "capability effect class"),
        requirementDisposition: decodeRequirementDisposition(record.requirementDisposition),
        providerDisposition: decodeProviderDisposition(record.providerDisposition),
        nativeRealization: decodeNativeRealization(record.nativeRealization),
      };
    case "machine-realization":
      exactKeys(record, ["explanationHandle", "kind", "machineId", "realization"], "machine-realization entry");
      return {
        explanationHandle,
        kind: "machine-realization",
        machineId: requiredString(record, "machineId"),
        realization: decodeNativeRealization(record.realization),
      };
    case "finding":
      exactKeys(record, ["explanationHandle", "kind", "finding"], "finding entry");
      return { explanationHandle, kind: "finding", finding: decodeFinding(record.finding) };
    default:
      throw new Error("unknown effective-profile entry kind");
  }
}

function decodeEffectiveProfileExplanation(value: unknown): EffectiveProfileExplanationResponse["explanation"] {
  const record = exactRecord(value, "effective-profile explanation");
  exactKeys(record, ["handle", "evidence"], "effective-profile explanation");
  return {
    handle: sha256Hex(record.handle, "effective-profile explanation handle"),
    evidence: decodeExplanationEvidence(record.evidence),
  };
}

function decodeExplanationEvidence(value: unknown): EffectiveProfileExplanationEvidence {
  const record = exactRecord(value, "effective-profile explanation evidence");
  switch (requiredString(record, "kind")) {
    case "declared-selection":
      exactKeys(record, ["kind", "selection"], "declared-selection explanation");
      return { kind: "declared-selection", selection: decodeDeclaredSelection(record.selection) };
    case "responsibility":
      exactKeys(record, ["kind", "contractId", "executionPathId", "reasonSteps"], "responsibility explanation");
      return {
        kind: "responsibility",
        contractId: requiredString(record, "contractId"),
        executionPathId: requiredString(record, "executionPathId"),
        reasonSteps: decodeArray(record.reasonSteps, "responsibility reasons", decodePathReason),
      };
    case "effective-selection":
      exactKeys(record, ["kind", "selection"], "effective-selection explanation");
      return { kind: "effective-selection", selection: decodeSelection(record.selection) };
    case "execution-path":
      exactKeys(record, ["kind", "selectionId", "reasonSteps"], "execution-path explanation");
      return {
        kind: "execution-path",
        selectionId: requiredString(record, "selectionId"),
        reasonSteps: decodeArray(record.reasonSteps, "execution-path reasons", decodePathReason),
      };
    case "operation-classification":
      exactKeys(record, ["kind", "exposure", "nativeRealization"], "operation-classification explanation");
      return {
        kind: "operation-classification",
        exposure: enumValue(record.exposure, OPERATION_EXPOSURES, "operation exposure"),
        nativeRealization: decodeNativeRealization(record.nativeRealization),
      };
    case "capability-classification":
      exactKeys(record, ["kind", "requirementDisposition", "providerDisposition", "nativeRealization"], "capability-classification explanation");
      return {
        kind: "capability-classification",
        requirementDisposition: decodeRequirementDisposition(record.requirementDisposition),
        providerDisposition: decodeProviderDisposition(record.providerDisposition),
        nativeRealization: decodeNativeRealization(record.nativeRealization),
      };
    case "finding-classification":
      exactKeys(record, ["kind", "finding"], "finding-classification explanation");
      return { kind: "finding-classification", finding: decodeFinding(record.finding) };
    case "machine-realization":
      exactKeys(record, ["kind", "realization"], "machine-realization explanation");
      return { kind: "machine-realization", realization: decodeNativeRealization(record.realization) };
    case "configuration":
      exactKeys(record, ["kind"], "configuration explanation");
      return { kind: "configuration" };
    default:
      throw new Error("unknown effective-profile explanation evidence kind");
  }
}

function decodeDeclaredSelection(value: unknown): EffectiveProfileDeclaredSelection {
  const record = exactRecord(value, "declared selection");
  switch (requiredString(record, "kind")) {
    case "resource":
      exactKeys(record, ["kind", "resourceKind", "selector"], "resource declared selection");
      return {
        kind: "resource",
        resourceKind: enumValue(record.resourceKind, RESOURCE_KINDS, "resource kind"),
        selector: decodeDeclaredSelector(record.selector),
      };
    case "mode-default":
      exactKeys(record, ["kind", "axis", "resourceId"], "mode-default declared selection");
      return {
        kind: "mode-default",
        axis: enumValue(record.axis, MODE_AXES, "mode axis"),
        resourceId: requiredString(record, "resourceId"),
      };
    default:
      throw new Error("unknown declared selection kind");
  }
}

function decodeDeclaredSelector(value: unknown): EffectiveProfileDeclaredSelector {
  const record = exactRecord(value, "declared selector");
  switch (requiredString(record, "kind")) {
    case "exact":
      exactKeys(record, ["kind", "resourceId"], "exact declared selector");
      return { kind: "exact", resourceId: requiredString(record, "resourceId") };
    case "all-from-package":
      exactKeys(record, ["kind", "packageId"], "Package-scoped declared selector");
      return { kind: "all-from-package", packageId: requiredString(record, "packageId") };
    default:
      throw new Error("unknown declared selector kind");
  }
}

function decodeSelection(value: unknown): EffectiveProfileSelection {
  const record = exactRecord(value, "effective selection");
  switch (requiredString(record, "kind")) {
    case "package":
      exactKeys(record, ["kind", "packageId", "version", "contentDigest"], "Package selection");
      return {
        kind: "package",
        packageId: requiredString(record, "packageId"),
        version: requiredString(record, "version"),
        contentDigest: sha256Hex(record.contentDigest, "Package content digest"),
      };
    case "normative-pack":
      exactKeys(record, ["kind", "packId", "attestorId", "version", "contentDigest"], "normative-Pack selection");
      return {
        kind: "normative-pack",
        packId: requiredString(record, "packId"),
        attestorId: requiredString(record, "attestorId"),
        version: requiredString(record, "version"),
        contentDigest: sha256Hex(record.contentDigest, "normative-Pack content digest"),
      };
    case "semantic-resource":
      exactKeys(record, ["kind", "repositoryId", "resourceKind", "resourceId"], "semantic-resource selection");
      return {
        kind: "semantic-resource",
        repositoryId: requiredString(record, "repositoryId"),
        resourceKind: enumValue(record.resourceKind, RESOURCE_KINDS, "resource kind"),
        resourceId: requiredString(record, "resourceId"),
      };
    case "run-policy":
      exactKeys(record, ["kind", "runPolicyId"], "run-policy selection");
      return { kind: "run-policy", runPolicyId: requiredString(record, "runPolicyId") };
    case "session":
      exactKeys(record, ["kind", "executionWorkspaceId", "activationRootCount", "additionalRootCount", "activationPathCount"], "session selection");
      return {
        kind: "session",
        executionWorkspaceId: requiredString(record, "executionWorkspaceId"),
        activationRootCount: u32(record.activationRootCount, "activation-root count"),
        additionalRootCount: u32(record.additionalRootCount, "additional-root count"),
        activationPathCount: u32(record.activationPathCount, "activation-path count"),
      };
    case "lifecycle":
      exactKeys(record, ["kind", "lifecycleId", "eventId", "actionId"], "lifecycle selection");
      return {
        kind: "lifecycle",
        lifecycleId: requiredString(record, "lifecycleId"),
        eventId: requiredString(record, "eventId"),
        actionId: requiredString(record, "actionId"),
      };
    case "authority-root":
      exactKeys(record, ["kind", "authorityRootId"], "authority-root selection");
      return { kind: "authority-root", authorityRootId: requiredString(record, "authorityRootId") };
    case "disclosure-ceiling":
      exactKeys(record, ["kind", "authorityCoordinate"], "disclosure-ceiling selection");
      return { kind: "disclosure-ceiling", authorityCoordinate: requiredString(record, "authorityCoordinate") };
    default:
      throw new Error("unknown effective selection kind");
  }
}

function decodeParticipant(value: unknown): EffectiveProfileParticipant {
  const record = exactRecord(value, "effective-profile participant");
  const kind = requiredString(record, "kind");
  const fieldByKind = {
    interface: "interfaceId",
    application: "applicationId",
    engine: "engineId",
    capability: "capabilityId",
    "terminal-leaf": "terminalLeafId",
    plugin: "pluginId",
    skill: "skillId",
  } as const;
  if (!(kind in fieldByKind)) {
    throw new Error("unknown effective-profile participant kind");
  }
  const typedKind = kind as keyof typeof fieldByKind;
  const field = fieldByKind[typedKind];
  exactKeys(record, ["kind", field], "effective-profile participant");
  const identity = requiredString(record, field);
  switch (typedKind) {
    case "interface": return { kind: typedKind, interfaceId: identity };
    case "application": return { kind: typedKind, applicationId: identity };
    case "engine": return { kind: typedKind, engineId: identity };
    case "capability": return { kind: typedKind, capabilityId: identity };
    case "terminal-leaf": return { kind: typedKind, terminalLeafId: identity };
    case "plugin": return { kind: typedKind, pluginId: identity };
    case "skill": return { kind: typedKind, skillId: identity };
  }
}

function decodeResponsibility(value: unknown): EffectiveProfileResponsibility {
  const record = exactRecord(value, "effective-profile responsibility");
  switch (requiredString(record, "kind")) {
    case "communication":
    case "organizational-work":
    case "crm-business-state":
    case "engineering-execution":
    case "context-memory":
    case "terminal-harness-execution":
    case "native-event-publication":
    case "native-effect-read-back": {
      const kind = enumValue(record.kind, SIMPLE_RESPONSIBILITIES, "responsibility kind");
      exactKeys(record, ["kind"], "effective-profile responsibility");
      return { kind };
    }
    case "extension":
      exactKeys(record, ["kind", "namespaceId", "contractId", "schemaId", "schemaSha256"], "extension responsibility");
      return {
        kind: "extension",
        namespaceId: requiredString(record, "namespaceId"),
        contractId: requiredString(record, "contractId"),
        schemaId: requiredString(record, "schemaId"),
        schemaSha256: sha256Hex(record.schemaSha256, "extension schema digest"),
      };
    default:
      throw new Error("unknown effective-profile responsibility kind");
  }
}

function decodeRequirementDisposition(value: unknown): RequirementDisposition {
  const record = exactRecord(value, "requirement disposition");
  switch (requiredString(record, "kind")) {
    case "satisfied":
      exactKeys(record, ["kind"], "satisfied requirement disposition");
      return { kind: "satisfied" };
    case "provider-binding-absent":
      exactKeys(record, ["kind"], "provider-binding-absent requirement disposition");
      return { kind: "provider-binding-absent" };
    case "narrowed":
      exactKeys(record, ["kind", "reason"], "narrowed requirement disposition");
      return {
        kind: "narrowed",
        reason: enumValue(record.reason, NARROWING_REASONS, "narrowing reason"),
      };
    case "blocked":
      exactKeys(record, ["kind", "reason"], "blocked requirement disposition");
      return {
        kind: "blocked",
        reason: enumValue(record.reason, POLICY_BLOCKS, "policy block"),
      };
    default:
      throw new Error("unknown requirement disposition kind");
  }
}

function decodeProviderDisposition(value: unknown): ProviderDisposition {
  const record = exactRecord(value, "provider disposition");
  switch (requiredString(record, "kind")) {
    case "bound":
      exactKeys(record, ["kind", "provider"], "bound provider disposition");
      return {
        kind: "bound",
        provider: enumValue(record.provider, PROVIDER_KINDS, "provider kind"),
      };
    case "missing-binding":
    case "excluded-by-narrowing":
    case "effect-mismatch":
    case "recipe-contract-mismatch": {
      const kind = enumValue(record.kind, SIMPLE_PROVIDER_DISPOSITIONS, "provider disposition");
      exactKeys(record, ["kind"], "provider disposition");
      return { kind };
    }
    default:
      throw new Error("unknown provider disposition kind");
  }
}

function decodeFinding(value: unknown): EffectiveProfileFinding {
  const record = exactRecord(value, "effective-profile finding");
  switch (requiredString(record, "kind")) {
    case "native-realization-unknown":
      exactKeys(record, ["kind", "machineId"], "native-realization finding");
      return { kind: "native-realization-unknown", machineId: requiredString(record, "machineId") };
    case "operation-unbound":
      exactKeys(record, ["kind", "repositoryId", "operationId"], "operation-unbound finding");
      return {
        kind: "operation-unbound",
        repositoryId: requiredString(record, "repositoryId"),
        operationId: requiredString(record, "operationId"),
      };
    case "requirement":
      exactKeys(record, ["kind", "repositoryId", "operationId", "requirementId", "capabilityId", "disposition"], "requirement finding");
      return {
        kind: "requirement",
        repositoryId: requiredString(record, "repositoryId"),
        operationId: requiredString(record, "operationId"),
        requirementId: requiredString(record, "requirementId"),
        capabilityId: requiredString(record, "capabilityId"),
        disposition: decodeRequirementDisposition(record.disposition),
      };
    default:
      throw new Error("unknown effective-profile finding kind");
  }
}

function decodePathReason(value: unknown): EffectiveProfilePathReason {
  const record = exactRecord(value, "effective-profile path reason");
  const kind = requiredString(record, "kind");
  switch (kind) {
    case "origin-receipt":
    case "entry-contract":
    case "responsibility-contract":
    case "termination-contract":
    case "settlement-contract":
    case "continuity-contract": {
      exactKeys(record, ["kind", "contractId"], "contract path reason");
      const contractId = requiredString(record, "contractId");
      return { kind, contractId };
    }
    case "provider-binding":
      exactKeys(record, ["kind", "bindingId"], "provider-binding path reason");
      return { kind, bindingId: requiredString(record, "bindingId") };
    case "delegation-edge":
      exactKeys(record, ["kind", "edgeId"], "delegation-edge path reason");
      return { kind, edgeId: requiredString(record, "edgeId") };
    default:
      throw new Error("unknown effective-profile path reason kind");
  }
}

function decodeSubject(value: unknown): WorkspaceSubjectSelector {
  const record = exactRecord(value, "workspace subject selector");
  const kind = requiredString(record, "kind");
  switch (kind) {
    case "source":
      exactKeys(record, ["kind", "sourceId"], "source subject");
      return { kind, sourceId: requiredString(record, "sourceId") };
    case "component":
      exactKeys(record, ["kind", "componentId"], "component subject");
      return { kind, componentId: requiredString(record, "componentId") };
    case "module":
      exactKeys(record, ["kind", "moduleId"], "module subject");
      return { kind, moduleId: requiredString(record, "moduleId") };
    case "declaration":
      exactKeys(record, ["kind", "ownerModuleId", "namespaceId", "declarationId"], "declaration subject");
      return {
        kind,
        ownerModuleId: requiredString(record, "ownerModuleId"),
        namespaceId: requiredString(record, "namespaceId"),
        declarationId: requiredString(record, "declarationId"),
      };
    case "import":
      exactKeys(record, ["kind", "importerModuleId", "upstreamModuleId", "upstreamReleaseId", "exportId", "sourceNamespaceId", "localNamespaceId"], "import subject");
      return {
        kind,
        importerModuleId: requiredString(record, "importerModuleId"),
        upstreamModuleId: requiredString(record, "upstreamModuleId"),
        upstreamReleaseId: requiredString(record, "upstreamReleaseId"),
        exportId: requiredString(record, "exportId"),
        sourceNamespaceId: requiredString(record, "sourceNamespaceId"),
        localNamespaceId: requiredString(record, "localNamespaceId"),
      };
    case "extension":
      exactKeys(record, ["kind", "ownerModuleId", "extensionNamespaceId", "targetNamespaceId", "targetDeclarationId"], "extension subject");
      return {
        kind,
        ownerModuleId: requiredString(record, "ownerModuleId"),
        extensionNamespaceId: requiredString(record, "extensionNamespaceId"),
        targetNamespaceId: requiredString(record, "targetNamespaceId"),
        targetDeclarationId: requiredString(record, "targetDeclarationId"),
      };
    case "policy":
      exactKeys(record, ["kind", "ownerModuleId", "namespaceId", "declarationId"], "policy subject");
      return {
        kind,
        ownerModuleId: requiredString(record, "ownerModuleId"),
        namespaceId: requiredString(record, "namespaceId"),
        declarationId: requiredString(record, "declarationId"),
      };
    case "resource":
      exactKeys(record, ["kind", "ownerModuleId", "resourceId"], "resource subject");
      return {
        kind,
        ownerModuleId: requiredString(record, "ownerModuleId"),
        resourceId: requiredString(record, "resourceId"),
      };
    case "workspace":
      exactKeys(record, ["kind", "workspaceId"], "workspace subject");
      return { kind, workspaceId: requiredString(record, "workspaceId") };
    case "responsibility-closure":
    case "organizational-translation-closure":
    case "experience-composition":
    case "resolution-lock":
    case "universe-lock":
      exactKeys(record, ["kind"], "singleton workspace subject");
      return { kind };
    default:
      throw new Error("unknown workspace subject selector kind");
  }
}

function decodeProviderBinding(value: unknown): EffectiveProfileProviderBinding | null {
  if (value === null) {
    return null;
  }
  const record = exactRecord(value, "terminal provider binding");
  exactKeys(record, ["providerBindingId", "providerBindingIdentity"], "terminal provider binding");
  return {
    providerBindingId: requiredString(record, "providerBindingId"),
    providerBindingIdentity: sha256Hex(
      record.providerBindingIdentity,
      "terminal provider-binding identity",
    ),
  };
}

function decodeNativeRealization(value: unknown): NativeRealizationDisposition {
  return enumValue(value, NATIVE_REALIZATIONS, "native realization");
}

const PROFILE_SECTIONS = [
  "intended-use-case",
  "effective-configuration",
  "capabilities-and-operations",
  "machine-realization",
  "gaps-and-explanations",
] as const;
const EFFECT_CLASSES = ["compute", "read", "execute", "write", "network", "publish"] as const;
const REPOSITORY_ACCESS = ["read", "mutate"] as const;
const NATIVE_REALIZATIONS = ["unknown"] as const;
const OPERATION_EXPOSURES = ["unbound", "semantic-only", "capability-bearing"] as const;
const PROVIDER_KINDS = ["host-native", "native-rust", "connector"] as const;
const RESOURCE_KINDS = [
  "operation",
  "role",
  "doctrine",
  "startup-policy",
  "run-mode",
  "stack",
  "policy-family",
  "recipe",
] as const;
const MODE_AXES = ["intensity", "task-stance"] as const;
const NARROWING_REASONS = [
  "capability-not-permitted",
  "effect-not-permitted",
  "capability-and-effect-not-permitted",
] as const;
const POLICY_BLOCKS = [
  "capability-forbidden",
  "effect-forbidden",
  "capability-and-effect-forbidden",
  "provider-effect-mismatch",
  "recipe-contract-mismatch",
] as const;
const SIMPLE_PROVIDER_DISPOSITIONS = [
  "missing-binding",
  "excluded-by-narrowing",
  "effect-mismatch",
  "recipe-contract-mismatch",
] as const;
const SIMPLE_RESPONSIBILITIES = [
  "communication",
  "organizational-work",
  "crm-business-state",
  "engineering-execution",
  "context-memory",
  "terminal-harness-execution",
  "native-event-publication",
  "native-effect-read-back",
] as const;

function assertSameInspectionContext(
  actual: ProfileInspectionContext,
  expected: ProfileInspectionContext,
  label: string,
): void {
  if (
    actual.projectId !== expected.projectId ||
    actual.executionWorkspaceId !== expected.executionWorkspaceId
  ) {
    throw new Error(`${label} does not match its inspection context`);
  }
}

function assertProfileProduct(
  context: ProfileInspectionContext,
  coordinates: EffectiveProfileCoordinates,
  editor: EditorCoordinate,
  state?: CurrentProfileState | StaleProfileState,
): void {
  assertSameInspectionContext(
    {
      projectId: coordinates.projectId,
      executionWorkspaceId: coordinates.executionWorkspaceId,
    },
    context,
    "effective-profile coordinates",
  );
  if (state?.kind === "stale") {
    if (state.admittedProfileDigest !== coordinates.profileDigest) {
      throw new Error("stale effective-profile state names a different admitted profile");
    }
    const admittedGeneration = BigInt(state.admittedEditor.generation);
    const admittedRevision = BigInt(state.admittedEditor.documentRevision);
    const currentGeneration = BigInt(editor.generation);
    const currentRevision = BigInt(editor.documentRevision);
    if (
      admittedGeneration > currentGeneration ||
      admittedRevision > currentRevision ||
      (admittedGeneration === currentGeneration && admittedRevision === currentRevision)
    ) {
      throw new Error("stale effective-profile state must name a predecessor editor product");
    }
  }
}

/** Construct the complete continuation coordinate for one admitted profile product. */
export function expectedProfileCoordinate(
  response: AdmittedEffectiveProfileResponse,
): ExpectedProfileCoordinate {
  return {
    generation: response.editor.generation,
    documentRevision: response.editor.documentRevision,
    profileDigest: response.coordinates.profileDigest,
    instanceDigest: response.coordinates.instanceDigest,
    selectedTopologyDigest: response.coordinates.selectedTopologyDigest,
    workspaceSemanticIrDigest: response.coordinates.workspaceSemanticIrDigest,
    workspaceAdmissionEvidenceDigest:
      response.coordinates.workspaceAdmissionEvidenceDigest,
    packageSemanticIrDigest: response.coordinates.packageSemanticIrDigest,
    compositionPlanDigest: response.coordinates.compositionPlanDigest,
  };
}

/** Fence one editor/profile product against a complete expected coordinate. */
export function profileProductMatchesExpected(
  editor: EditorCoordinate,
  coordinates: EffectiveProfileCoordinates,
  expected: ExpectedProfileCoordinate,
): boolean {
  return (
    editor.generation === expected.generation &&
    editor.documentRevision === expected.documentRevision &&
    coordinates.profileDigest === expected.profileDigest &&
    coordinates.instanceDigest === expected.instanceDigest &&
    coordinates.selectedTopologyDigest === expected.selectedTopologyDigest &&
    coordinates.workspaceSemanticIrDigest === expected.workspaceSemanticIrDigest &&
    coordinates.workspaceAdmissionEvidenceDigest === expected.workspaceAdmissionEvidenceDigest &&
    coordinates.packageSemanticIrDigest === expected.packageSemanticIrDigest &&
    coordinates.compositionPlanDigest === expected.compositionPlanDigest
  );
}

function decodeInspectionContext(value: unknown): ProfileInspectionContext {
  const record = exactRecord(value, "profile inspection context");
  exactKeys(record, ["projectId", "executionWorkspaceId"]);
  return {
    projectId: requiredString(record, "projectId"),
    executionWorkspaceId: requiredString(record, "executionWorkspaceId"),
  };
}

function decodeEditorCoordinate(value: unknown): EditorCoordinate {
  const record = exactRecord(value, "editor coordinate");
  exactKeys(record, ["generation", "documentRevision"]);
  return {
    generation: canonicalDecimal(record.generation),
    documentRevision: canonicalDecimal(record.documentRevision),
  };
}

function decodeExpectedProfileCoordinate(record: ExpectedProfileCoordinate): ExpectedProfileCoordinate {
  return {
    generation: record.generation,
    documentRevision: record.documentRevision,
    profileDigest: sha256Hex(record.profileDigest, "profile digest"),
    instanceDigest: sha256Hex(record.instanceDigest, "instance digest"),
    selectedTopologyDigest: sha256Hex(
      record.selectedTopologyDigest,
      "topology digest",
    ),
    workspaceSemanticIrDigest: sha256Hex(
      record.workspaceSemanticIrDigest,
      "workspace semantic IR digest",
    ),
    workspaceAdmissionEvidenceDigest: sha256Hex(
      record.workspaceAdmissionEvidenceDigest,
      "workspace admission evidence digest",
    ),
    packageSemanticIrDigest: sha256Hex(
      record.packageSemanticIrDigest,
      "Package semantic IR digest",
    ),
    compositionPlanDigest: sha256Hex(
      record.compositionPlanDigest,
      "composition-plan digest",
    ),
  };
}

function decodeImprovementPreviewCoordinate(
  record: ConfigurationImprovementPreviewCoordinate,
): ConfigurationImprovementPreviewCoordinate {
  return {
    baseline: decodeExpectedProfileCoordinate(record.baseline),
    candidateEditor: {
      generation: record.candidateEditor.generation,
      documentRevision: record.candidateEditor.documentRevision,
    },
    candidateSelectedTopologyDigest: sha256Hex(
      record.candidateSelectedTopologyDigest,
      "candidate topology digest",
    ),
    candidateProfileDigest: sha256Hex(
      record.candidateProfileDigest,
      "candidate profile digest",
    ),
    candidateInstanceDigest: sha256Hex(
      record.candidateInstanceDigest,
      "candidate instance digest",
    ),
    derivationReceiptDigest: sha256Hex(
      record.derivationReceiptDigest,
      "derivation-receipt digest",
    ),
  };
}

function decodeImprovementPreview(record: ConfigurationImprovementPreview): ConfigurationImprovementPreview {
  return {
    workspaceId: requiredString(record, "workspaceId"),
    projectId: requiredString(record, "projectId"),
    executionWorkspaceId: requiredString(record, "executionWorkspaceId"),
    baselineWorkspaceSemanticIrDigest: sha256Hex(
      record.baselineWorkspaceSemanticIrDigest,
      "baseline workspace semantic IR digest",
    ),
    baselineWorkspaceAdmissionEvidenceDigest: sha256Hex(
      record.baselineWorkspaceAdmissionEvidenceDigest,
      "baseline workspace admission evidence digest",
    ),
    profileDigest: sha256Hex(record.profileDigest, "profile digest"),
    instanceDigest: sha256Hex(record.instanceDigest, "instance digest"),
    candidateProfileDigest: sha256Hex(
      record.candidateProfileDigest,
      "candidate profile digest",
    ),
    candidateInstanceDigest: sha256Hex(
      record.candidateInstanceDigest,
      "candidate instance digest",
    ),
    candidateWorkspaceSemanticIrDigest: sha256Hex(
      record.candidateWorkspaceSemanticIrDigest,
      "candidate workspace semantic IR digest",
    ),
    candidateWorkspaceAdmissionEvidenceDigest: sha256Hex(
      record.candidateWorkspaceAdmissionEvidenceDigest,
      "candidate workspace admission evidence digest",
    ),
    derivationReceiptDigest: sha256Hex(
      record.derivationReceiptDigest,
      "derivation-receipt digest",
    ),
    changeRootCount: record.changeRootCount,
    affectedCount: record.affectedCount,
    unaffectedCount: record.unaffectedCount,
    selectedTopologyChange: decodeImprovementCoordinateChange(
      record.selectedTopologyChange,
      "selected topology",
    ),
    profileChange: decodeImprovementCoordinateChange(record.profileChange, "profile"),
    instanceChange: decodeImprovementCoordinateChange(record.instanceChange, "instance"),
    packageSemanticIrChange: decodeImprovementCoordinateChange(
      record.packageSemanticIrChange,
      "Package semantic IR",
    ),
    compositionPlanChange: decodeImprovementCoordinateChange(
      record.compositionPlanChange,
      "composition plan",
    ),
  };
}

function decodeImprovementCoordinateChange(
  change: ConfigurationImprovementCoordinateChange,
  label: string,
): ConfigurationImprovementCoordinateChange {
  if (change.state === "unchanged") {
    return { state: "unchanged", digest: sha256Hex(change.digest, `${label} digest`) };
  }
  return {
    state: "changed",
    prior_digest: sha256Hex(change.prior_digest, `prior ${label} digest`),
    candidate_digest: sha256Hex(change.candidate_digest, `candidate ${label} digest`),
  };
}

function exactRecord(value: unknown, label: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error(`${label} must be an object`);
  }
  return value as Record<string, unknown>;
}

// Generated nested records can retain permissive index signatures. Compare
// their named fields here; the semantic decoders close each shape at runtime.
type NamedWireKey<Key> = string extends Key
  ? never
  : number extends Key
    ? never
    : symbol extends Key
      ? never
      : Key;

type ExplicitWireShape<T> = T extends ReadonlyArray<infer Item>
  ? ExplicitWireShape<Item>[]
  : T extends object
    ? {
        [Key in keyof T as NamedWireKey<Key>]: ExplicitWireShape<T[Key]>;
      }
    : T;

function assertGeneratedResponse<T>(response: ExplicitWireShape<T>): void {
  void response;
}

function exactKeys(
  record: Record<string, unknown>,
  expected: ReadonlyArray<string>,
  label = "transport value",
): void {
  const actual = Object.keys(record).sort();
  const canonical = [...expected].sort();
  if (actual.length !== canonical.length || actual.some((key, index) => key !== canonical[index])) {
    throw new Error(`${label} contains unknown or missing fields`);
  }
}

function exactKeysAllowing(
  record: Record<string, unknown>,
  required: ReadonlyArray<string>,
  optional: ReadonlyArray<string>,
  label: string,
): void {
  const actual = Object.keys(record);
  if (
    required.some((key) => !Object.hasOwn(record, key)) ||
    actual.some((key) => !required.includes(key) && !optional.includes(key))
  ) {
    throw new Error(`${label} contains unknown or missing fields`);
  }
}

function requiredString<T extends object>(record: T, key: keyof T & string): string {
  const value = record[key];
  if (typeof value !== "string" || value.length === 0) {
    throw new Error(`transport field ${key} must be nonempty text`);
  }
  return value;
}

function optionalSha256Hex(
  record: Record<string, unknown>,
  key: string,
): string | undefined {
  return Object.hasOwn(record, key) ? sha256Hex(record[key], key) : undefined;
}

function sha256Hex(value: unknown, label: string): string {
  if (typeof value !== "string" || !/^[0-9a-f]{64}$/u.test(value)) {
    throw new Error(`${label} must be 64 lowercase hexadecimal characters`);
  }
  return value;
}

function enumValue<const T extends string>(
  value: unknown,
  allowed: ReadonlyArray<T>,
  label: string,
): T {
  if (typeof value !== "string" || !allowed.some((candidate) => candidate === value)) {
    throw new Error(`unknown ${label}`);
  }
  return value as T;
}

function decodeArray<T>(
  value: unknown,
  label: string,
  decode: (member: unknown) => T,
): T[] {
  if (!Array.isArray(value)) {
    throw new Error(`${label} must be an array`);
  }
  return value.map(decode);
}

function decodeStringArray(value: unknown, label: string): string[] {
  return decodeArray(value, label, (member) => {
    if (typeof member !== "string" || member.length === 0) {
      throw new Error(`${label} members must be nonempty text`);
    }
    return member;
  });
}

function decodeEnumArray<const T extends string>(
  value: unknown,
  allowed: ReadonlyArray<T>,
  label: string,
): T[] {
  return decodeArray(value, label, (member) => enumValue(member, allowed, label));
}

function u32(value: unknown, label: string): number {
  if (typeof value !== "number" || !Number.isInteger(value) || value < 0 || value > 0xffff_ffff) {
    throw new Error(`${label} must be an unsigned 32-bit integer`);
  }
  return value;
}

function u16(value: unknown, label: string): number {
  if (typeof value !== "number" || !Number.isInteger(value) || value < 0 || value > 0xffff) {
    throw new Error(`${label} must be an unsigned 16-bit integer`);
  }
  return value;
}

function normalizedProfilePageLimit(value: unknown): number {
  const requested = u16(value, "effective-profile request limit");
  return requested === 0 ? 64 : Math.min(requested, 256);
}

function canonicalDecimal(value: unknown): string {
  if (
    typeof value !== "string" ||
    value.length > 20 ||
    !/^(0|[1-9][0-9]*)$/u.test(value) ||
    BigInt(value) > 0xffff_ffff_ffff_ffffn
  ) {
    throw new Error("editor coordinate must be canonical decimal text");
  }
  return value;
}
