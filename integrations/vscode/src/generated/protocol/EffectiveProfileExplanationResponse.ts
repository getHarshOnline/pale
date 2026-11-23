/* Generated from Pale LSP's private Rust wire types. Do not edit. */

/**
 * Bounded compiler evidence retained for one explainable profile entry.
 */
export type EffectiveProfileExplanationEvidence =
  | {
      kind: "declared-selection";
      /**
       * Normalized declared selection copied from application inspection.
       */
      selection: EffectiveProfileDeclaredSelection;
      [k: string]: unknown;
    }
  | {
      /**
       * Exact responsibility contract.
       */
      contractId: string;
      /**
       * Selected path carrying the contract.
       */
      executionPathId: string;
      kind: "responsibility";
      /**
       * Ordered application-owned composition reasons.
       */
      reasonSteps: EffectiveProfilePathReason[];
      [k: string]: unknown;
    }
  | {
      kind: "effective-selection";
      /**
       * Exact selected configuration coordinate.
       */
      selection: EffectiveProfileSelection;
      [k: string]: unknown;
    }
  | {
      kind: "execution-path";
      /**
       * Ordered application-owned composition reasons.
       */
      reasonSteps: EffectiveProfilePathReason[];
      /**
       * Stable selected path identity.
       */
      selectionId: string;
      [k: string]: unknown;
    }
  | {
      /**
       * Exact exposure classification.
       */
      exposure: EffectiveProfileOperationExposure;
      kind: "operation-classification";
      /**
       * Independently qualified native evidence.
       */
      nativeRealization: EffectiveProfileNativeRealization;
      [k: string]: unknown;
    }
  | {
      kind: "capability-classification";
      /**
       * Independently qualified native evidence.
       */
      nativeRealization: EffectiveProfileNativeRealization;
      /**
       * Private provider selection result.
       */
      providerDisposition: EffectiveProfileProviderDisposition;
      /**
       * Portable configuration result.
       */
      requirementDisposition: EffectiveProfileRequirementDisposition;
      [k: string]: unknown;
    }
  | {
      /**
       * Typed finding without presentation prose.
       */
      finding: EffectiveProfileFinding;
      kind: "finding-classification";
      [k: string]: unknown;
    }
  | {
      kind: "machine-realization";
      /**
       * Independently qualified native evidence.
       */
      realization: EffectiveProfileNativeRealization;
      [k: string]: unknown;
    }
  | {
      kind: "configuration";
      [k: string]: unknown;
    };
/**
 * One normalized member of the admitted declared semantic selection.
 */
export type EffectiveProfileDeclaredSelection =
  | {
      kind: "resource";
      /**
       * Semantic family supplied by the enclosing declaration field.
       */
      resourceKind: EffectiveProfileResourceKind;
      /**
       * Exact or Package-scoped selector retained without closure expansion.
       */
      selector: EffectiveProfileDeclaredSelector;
      [k: string]: unknown;
    }
  | {
      /**
       * Orthogonal runtime-mode dimension being defaulted.
       */
      axis: EffectiveProfileModeAxis;
      kind: "mode-default";
      /**
       * Exact owner-qualified run-mode resource.
       */
      resourceId: string;
      [k: string]: unknown;
    };
/**
 * Closed Package semantic family of one selected resource.
 */
export type EffectiveProfileResourceKind =
  "operation" | "role" | "doctrine" | "startup-policy" | "run-mode" | "stack" | "policy-family" | "recipe";
/**
 * One normalized selector retained from declared private intent.
 *
 * This remains distinct from [`EffectiveProfileSelection::SemanticResource`],
 * which represents a member of the compiler-expanded effective closure.
 */
export type EffectiveProfileDeclaredSelector =
  | {
      kind: "exact";
      /**
       * Exact resource selected by the principal declaration.
       */
      resourceId: string;
      [k: string]: unknown;
    }
  | {
      kind: "all-from-package";
      /**
       * Package scope of the typed wildcard.
       */
      packageId: string;
      [k: string]: unknown;
    };
/**
 * Closed runtime-scoped mode axis used by a declared mode default.
 */
export type EffectiveProfileModeAxis = "intensity" | "task-stance";
/**
 * One typed causal selector retained by a selected execution path.
 */
export type EffectiveProfilePathReason =
  | {
      /**
       * Exact origin-receipt contract identity.
       */
      contractId: string;
      kind: "origin-receipt";
      [k: string]: unknown;
    }
  | {
      /**
       * Exact entry-contract identity.
       */
      contractId: string;
      kind: "entry-contract";
      [k: string]: unknown;
    }
  | {
      /**
       * Exact responsibility-contract identity.
       */
      contractId: string;
      kind: "responsibility-contract";
      [k: string]: unknown;
    }
  | {
      /**
       * Exact provider-binding identity.
       */
      bindingId: string;
      kind: "provider-binding";
      [k: string]: unknown;
    }
  | {
      /**
       * Exact neutral delegation-edge identity.
       */
      edgeId: string;
      kind: "delegation-edge";
      [k: string]: unknown;
    }
  | {
      /**
       * Exact termination-contract identity.
       */
      contractId: string;
      kind: "termination-contract";
      [k: string]: unknown;
    }
  | {
      /**
       * Exact settlement-contract identity.
       */
      contractId: string;
      kind: "settlement-contract";
      [k: string]: unknown;
    }
  | {
      /**
       * Exact continuity-contract identity.
       */
      contractId: string;
      kind: "continuity-contract";
      [k: string]: unknown;
    };
/**
 * One exact selected configuration coordinate.
 *
 * The closed variants avoid delimiter-composed identifiers and optional
 * prose fields. Every value therefore states the exact coordinate family it
 * carries, while the application remains the owner of why it was selected.
 */
export type EffectiveProfileSelection =
  | {
      /**
       * Digest of the exact Package release bytes.
       */
      contentDigest: string;
      kind: "package";
      /**
       * Package identity.
       */
      packageId: string;
      /**
       * Selected Package version.
       */
      version: string;
      [k: string]: unknown;
    }
  | {
      /**
       * Attestor owning the normative release statement.
       */
      attestorId: string;
      /**
       * Digest of the exact Pack release bytes.
       */
      contentDigest: string;
      kind: "normative-pack";
      /**
       * Normative Pack identity.
       */
      packId: string;
      /**
       * Selected Pack version.
       */
      version: string;
      [k: string]: unknown;
    }
  | {
      kind: "semantic-resource";
      /**
       * Logical repository receiving the resource.
       */
      repositoryId: string;
      /**
       * Owner-qualified Package resource identity.
       */
      resourceId: string;
      /**
       * Exact semantic resource family.
       */
      resourceKind: EffectiveProfileResourceKind;
      [k: string]: unknown;
    }
  | {
      kind: "run-policy";
      /**
       * Exact run-policy identity.
       */
      runPolicyId: string;
      [k: string]: unknown;
    }
  | {
      /**
       * Number of activation paths.
       */
      activationPathCount: number;
      /**
       * Number of activation roots.
       */
      activationRootCount: number;
      /**
       * Number of additional roots.
       */
      additionalRootCount: number;
      /**
       * Execution workspace owning the session selection.
       */
      executionWorkspaceId: string;
      kind: "session";
      [k: string]: unknown;
    }
  | {
      /**
       * Typed lifecycle action identity.
       */
      actionId: string;
      /**
       * Typed lifecycle event identity.
       */
      eventId: string;
      kind: "lifecycle";
      /**
       * Lifecycle behavior identity.
       */
      lifecycleId: string;
      [k: string]: unknown;
    }
  | {
      /**
       * Exact authority-root identity.
       */
      authorityRootId: string;
      kind: "authority-root";
      [k: string]: unknown;
    }
  | {
      /**
       * Exact forbidden-token authority coordinate.
       */
      authorityCoordinate: string;
      kind: "disclosure-ceiling";
      [k: string]: unknown;
    };
/**
 * Exact three-state operation exposure contract.
 */
export type EffectiveProfileOperationExposure = "unbound" | "semantic-only" | "capability-bearing";
/**
 * Owner-native realization evidence available to this inspection.
 */
export type EffectiveProfileNativeRealization = "unknown";
/**
 * Private provider-selection disposition, independent of native readiness.
 */
export type EffectiveProfileProviderDisposition =
  | {
      kind: "bound";
      /**
       * Selected closed provider kind.
       */
      provider: EffectiveProfileProviderKind;
      [k: string]: unknown;
    }
  | {
      kind: "missing-binding";
      [k: string]: unknown;
    }
  | {
      kind: "excluded-by-narrowing";
      [k: string]: unknown;
    }
  | {
      kind: "effect-mismatch";
      [k: string]: unknown;
    }
  | {
      kind: "recipe-contract-mismatch";
      [k: string]: unknown;
    };
/**
 * Closed provider kind selected by private configuration.
 */
export type EffectiveProfileProviderKind = "host-native" | "native-rust" | "connector";
/**
 * Portable disposition of one Package-owned capability requirement.
 *
 * This axis is independent of [`EffectiveProfileOperationExposure`],
 * [`EffectiveProfileProviderDisposition`], and
 * [`EffectiveProfileNativeRealization`]. In particular, an explicit
 * forbidden-set member is blocked, while absence from a positive permitted
 * set is narrowed.
 */
export type EffectiveProfileRequirementDisposition =
  | {
      kind: "satisfied";
      [k: string]: unknown;
    }
  | {
      kind: "provider-binding-absent";
      [k: string]: unknown;
    }
  | {
      kind: "narrowed";
      /**
       * Exact omitted dimension or dimensions.
       */
      reason: EffectiveProfileNarrowingReason;
      [k: string]: unknown;
    }
  | {
      kind: "blocked";
      /**
       * Exact forbidden or provider-contract dimension.
       */
      reason: EffectiveProfilePolicyBlock;
      [k: string]: unknown;
    };
/**
 * Exact positive-set relation that narrowed a requirement.
 */
export type EffectiveProfileNarrowingReason =
  "capability-not-permitted" | "effect-not-permitted" | "capability-and-effect-not-permitted";
/**
 * Exact policy dimension blocking a present provider.
 */
export type EffectiveProfilePolicyBlock =
  | "capability-forbidden"
  | "effect-forbidden"
  | "capability-and-effect-forbidden"
  | "provider-effect-mismatch"
  | "recipe-contract-mismatch";
/**
 * One structured, application-owned finding.
 */
export type EffectiveProfileFinding =
  | {
      kind: "native-realization-unknown";
      /**
       * Selected private machine-binding identity.
       */
      machineId: string;
      [k: string]: unknown;
    }
  | {
      kind: "operation-unbound";
      /**
       * Owner-qualified Package operation identity.
       */
      operationId: string;
      /**
       * Logical repository containing the operation occurrence.
       */
      repositoryId: string;
      [k: string]: unknown;
    }
  | {
      /**
       * Portable capability identity.
       */
      capabilityId: string;
      /**
       * Application-owned portable disposition.
       */
      disposition: EffectiveProfileRequirementDisposition;
      kind: "requirement";
      /**
       * Owner-qualified Package operation identity.
       */
      operationId: string;
      /**
       * Logical repository containing the requirement occurrence.
       */
      repositoryId: string;
      /**
       * Requirement identity local to the operation.
       */
      requirementId: string;
      [k: string]: unknown;
    };
/**
 * Opaque identity of one explanation retained by an effective-profile handle.
 *
 * The value is a domain-separated SHA-256 established by `pale-sdk`. Its
 * representation reveals neither compiler indices nor source coordinates.
 */
export type EffectiveProfileExplanationHandle = string;

/**
 * Typed LSP envelope around one retained API-owned profile explanation.
 */
export interface EffectiveProfileExplanationResponse {
  /**
   * Ephemeral selection under which the retained profile was admitted.
   */
  context: ProfileInspectionContext;
  /**
   * API/profile coordinates fenced by the request.
   */
  coordinates: EffectiveProfileCoordinates;
  /**
   * Exact current editor request coordinate.
   */
  editor: EditorCoordinateOutput;
  /**
   * Bounded API-owned explanation evidence.
   */
  explanation: EffectiveProfileExplanation;
}
/**
 * Explicit, ephemeral project selection supplied by the editor user.
 */
export interface ProfileInspectionContext {
  /**
   * Selected logical execution-workspace identity.
   */
  executionWorkspaceId: string;
  /**
   * Selected logical project identity.
   */
  projectId: string;
}
/**
 * Exact semantic coordinates shared by every page of one profile inspection.
 */
export interface EffectiveProfileCoordinates {
  /**
   * Identity of the neutral experience-composition proof.
   */
  compositionPlanDigest: string;
  /**
   * Selected logical execution workspace.
   */
  executionWorkspaceId: string;
  /**
   * Canonical private machine-resolved instance identity.
   *
   * This coordinate proves which admitted private projection was compiled;
   * it does not claim machine or native realization.
   */
  instanceDigest: string;
  /**
   * Selected private machine-binding identity.
   */
  machineId: string;
  /**
   * Identity of the admitted Package semantic closure.
   */
  packageSemanticIrDigest: string;
  /**
   * Canonical portable profile identity.
   */
  profileDigest: string;
  /**
   * Selected logical project.
   */
  projectId: string;
  /**
   * Identity of the exact project-relative desired topology selection.
   */
  selectedTopologyDigest: string;
  /**
   * Identity of the owner-proof set used for workspace admission.
   */
  workspaceAdmissionEvidenceDigest: string;
  /**
   * Principal workspace owning the desired graph.
   */
  workspaceId: string;
  /**
   * Identity of the admitted workspace semantic graph.
   */
  workspaceSemanticIrDigest: string;
}
/**
 * Serialized editor counters, not JavaScript numbers.
 */
export interface EditorCoordinateOutput {
  documentRevision: string;
  generation: string;
}
/**
 * One bounded explanation returned from the retained API projection.
 */
export interface EffectiveProfileExplanation {
  /**
   * Typed application-owned proof or classification evidence.
   */
  evidence: EffectiveProfileExplanationEvidence;
  /**
   * Opaque handle used for exact request/result correlation.
   */
  handle: EffectiveProfileExplanationHandle;
}
