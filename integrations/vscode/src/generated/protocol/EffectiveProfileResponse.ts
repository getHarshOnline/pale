/* Generated from Pale LSP's private Rust wire types. Do not edit. */

/**
 * Opaque content identity of one semantically admitted diagnostic.
 *
 * The API derives this domain-separated identity from the exact canonical
 * `diagnostic.v1` bytes. Outer leaves may correlate the same diagnostic but
 * cannot construct or reinterpret its semantic classification.
 */
export type DiagnosticIdentity = string;
/**
 * One explainable row in a deterministic effective-profile page.
 */
export type EffectiveProfileEntry = EffectiveProfileEntryDetail & {
  /**
   * Opaque API-owned explanation identity for this exact profile row.
   */
  explanationHandle: EffectiveProfileExplanationHandle;
  [k: string]: unknown;
};
export type EffectiveProfileEntryDetail =
  | {
      kind: "declared-selection";
      /**
       * Exact declared selection retained by application inspection.
       */
      selection: EffectiveProfileDeclaredSelection;
      [k: string]: unknown;
    }
  | {
      /**
       * Maximum effect classes admitted for this contract.
       */
      authorityCeiling: EffectiveProfileEffectClass[];
      /**
       * Exact responsibility-contract identity.
       */
      contractId: string;
      /**
       * Execution-path selection carrying this contract.
       */
      executionPathId: string;
      kind: "responsibility";
      /**
       * Participant bounded by the contract.
       */
      participant: EffectiveProfileParticipant;
      /**
       * Ordered application-owned reasons for the selected carrying path.
       */
      reasonSteps: EffectiveProfilePathReason[];
      /**
       * Typed responsibilities assigned by the semantic owner.
       */
      responsibilities: EffectiveProfileResponsibility[];
      /**
       * Existing causal-proof selector.
       */
      subject: PortableWorkspaceSubjectSelector;
      [k: string]: unknown;
    }
  | {
      kind: "selection";
      /**
       * Closed structured selection coordinate.
       */
      selection: EffectiveProfileSelection;
      [k: string]: unknown;
    }
  | {
      /**
       * Capabilities forbidden by private configuration.
       */
      forbiddenCapabilities: string[];
      /**
       * Effects forbidden by private configuration.
       */
      forbiddenEffects: EffectiveProfileEffectClass[];
      kind: "capability-narrowing";
      /**
       * Capabilities retained by private configuration.
       */
      permittedCapabilities: string[];
      /**
       * Effects retained by private configuration.
       */
      permittedEffects: EffectiveProfileEffectClass[];
      [k: string]: unknown;
    }
  | {
      /**
       * Canonical semantic contributors.
       */
      contributors: string[];
      /**
       * Capabilities forbidden by any contributor.
       */
      forbiddenCapabilities: string[];
      /**
       * Effects forbidden by any contributor.
       */
      forbiddenEffects: EffectiveProfileEffectClass[];
      kind: "governing-ceiling";
      /**
       * Capabilities permitted by every contributor.
       */
      permittedCapabilities: string[];
      /**
       * Effects permitted by every contributor.
       */
      permittedEffects: EffectiveProfileEffectClass[];
      /**
       * Exact Package policy resources whose bodies were met.
       */
      policies: string[];
      /**
       * Semantic scope of the result.
       */
      scope: string;
      /**
       * Governed Package resource.
       */
      subjectResource: string;
      [k: string]: unknown;
    }
  | {
      /**
       * Maximum access admitted by the execution-workspace edge.
       */
      accessCeiling: EffectiveProfileRepositoryAccess;
      kind: "repository";
      /**
       * Zero-based effective materialization order.
       */
      materializationOrder: number;
      /**
       * Logical repository identity.
       */
      repositoryId: string;
      [k: string]: unknown;
    }
  | {
      kind: "provider-repository";
      /**
       * Owner-native availability remains separately qualified.
       */
      nativeRealization: EffectiveProfileNativeRealization;
      /**
       * Opaque provider-owned repository coordinate.
       */
      nativeRepositoryCoordinate: string;
      /**
       * Neutral provider-binding identity selected by an experience path.
       */
      providerBindingId: string;
      /**
       * Canonical logical repository selected by the execution workspace.
       */
      repositoryId: string;
      [k: string]: unknown;
    }
  | {
      /**
       * Portable capability ceiling.
       */
      capabilityCeiling: string[];
      /**
       * Portable effect ceiling.
       */
      effectCeiling: EffectiveProfileEffectClass[];
      /**
       * Selected experience-path identity.
       */
      experiencePathId: string;
      kind: "execution-path";
      /**
       * Native evidence remains independently qualified.
       */
      nativeRealization: EffectiveProfileNativeRealization;
      /**
       * Explicit provider-binding declarations required by the path.
       */
      providerBindingIds: string[];
      /**
       * Ordered causal selectors retained by the composition proof.
       */
      reasonSteps: EffectiveProfilePathReason[];
      /**
       * Exact responsibility contracts consumed by the path.
       */
      responsibilityContractIds: string[];
      /**
       * Domain-separated responsibility-path identity.
       */
      responsibilityPathIdentity: string;
      /**
       * Stable execution-path selection identity.
       */
      selectionId: string;
      /**
       * Existing composition-proof selector.
       */
      subject: PortableWorkspaceSubjectSelector;
      /**
       * Exact terminal provider binding derived from the path contract.
       *
       * `None` is reserved for a native-only path with no terminal leaf.
       */
      terminalProviderBinding: EffectiveProfileProviderBindingRef | null;
      /**
       * Ordered neutral delegation edges traversed by the path.
       */
      workDelegationEdgeIds: string[];
      [k: string]: unknown;
    }
  | {
      /**
       * Exact three-state operation exposure.
       */
      exposure: EffectiveProfileOperationExposure;
      kind: "operation";
      /**
       * Native availability remains separately qualified.
       */
      nativeRealization: EffectiveProfileNativeRealization;
      /**
       * Owner-qualified Package operation identity.
       */
      operationId: string;
      /**
       * Logical repository containing this occurrence.
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
       * Portable effect class.
       */
      effectClass: EffectiveProfileEffectClass;
      kind: "capability";
      /**
       * Native availability remains separately qualified.
       */
      nativeRealization: EffectiveProfileNativeRealization;
      /**
       * Owner-qualified operation identity.
       */
      operationId: string;
      /**
       * Private provider selection, independently typed.
       */
      providerDisposition: EffectiveProfileProviderDisposition;
      /**
       * Logical repository containing the occurrence.
       */
      repositoryId: string;
      /**
       * Portable configuration disposition.
       */
      requirementDisposition: EffectiveProfileRequirementDisposition;
      /**
       * Requirement identity local to the operation.
       */
      requirementId: string;
      [k: string]: unknown;
    }
  | {
      kind: "machine-realization";
      /**
       * Selected private machine-binding identity.
       */
      machineId: string;
      /**
       * Current owner-native evidence ceiling.
       */
      realization: EffectiveProfileNativeRealization;
      [k: string]: unknown;
    }
  | {
      /**
       * Closed structured finding without presentation prose.
       */
      finding: EffectiveProfileFinding;
      kind: "finding";
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
 * Portable effect ceiling retained without native-effect authority.
 */
export type EffectiveProfileEffectClass = "compute" | "read" | "execute" | "write" | "network" | "publish";
/**
 * Participant kind owning one selected responsibility contract.
 */
export type EffectiveProfileParticipant =
  | {
      /**
       * Stable interface identity.
       */
      interfaceId: string;
      kind: "interface";
      [k: string]: unknown;
    }
  | {
      /**
       * Stable application identity.
       */
      applicationId: string;
      kind: "application";
      [k: string]: unknown;
    }
  | {
      /**
       * Stable identity carried by the `engine` wire variant.
       */
      engineId: string;
      kind: "engine";
      [k: string]: unknown;
    }
  | {
      /**
       * Stable capability identity.
       */
      capabilityId: string;
      kind: "capability";
      [k: string]: unknown;
    }
  | {
      kind: "terminal-leaf";
      /**
       * Stable terminal-leaf identity.
       */
      terminalLeafId: string;
      [k: string]: unknown;
    }
  | {
      kind: "plugin";
      /**
       * Stable plugin identity.
       */
      pluginId: string;
      [k: string]: unknown;
    }
  | {
      kind: "skill";
      /**
       * Stable skill identity.
       */
      skillId: string;
      [k: string]: unknown;
    };
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
 * One typed responsibility assigned by the semantic owner.
 */
export type EffectiveProfileResponsibility =
  | {
      kind: "communication";
      [k: string]: unknown;
    }
  | {
      kind: "organizational-work";
      [k: string]: unknown;
    }
  | {
      kind: "crm-business-state";
      [k: string]: unknown;
    }
  | {
      kind: "engineering-execution";
      [k: string]: unknown;
    }
  | {
      kind: "context-memory";
      [k: string]: unknown;
    }
  | {
      kind: "terminal-harness-execution";
      [k: string]: unknown;
    }
  | {
      kind: "native-event-publication";
      [k: string]: unknown;
    }
  | {
      kind: "native-effect-read-back";
      [k: string]: unknown;
    }
  | {
      /**
       * Contract identity within that namespace.
       */
      contractId: string;
      kind: "extension";
      /**
       * Namespace owning the extension vocabulary.
       */
      namespaceId: string;
      /**
       * Exact admitted schema member.
       */
      schemaId: string;
      /**
       * SHA-256 of the exact schema bytes.
       */
      schemaSha256: string;
      [k: string]: unknown;
    };
/**
 * Owned, closed selector for one portable causal subject.
 *
 * Compound coordinates keep each typed identity in a distinct field; no
 * delimiter convention is part of this façade. Convert a selector with
 * [`PortableWorkspaceSubject::try_from`]. Successful conversion validates
 * every identity against the Pale Workspace grammar.
 */
export type PortableWorkspaceSubjectSelector =
  | {
      kind: "source";
      /**
       * Receipt-local source identity.
       */
      sourceId: string;
      [k: string]: unknown;
    }
  | {
      /**
       * Component identity.
       */
      componentId: string;
      kind: "component";
      [k: string]: unknown;
    }
  | {
      kind: "module";
      /**
       * Module identity.
       */
      moduleId: string;
      [k: string]: unknown;
    }
  | {
      /**
       * Declaration identity local to the namespace.
       */
      declarationId: string;
      kind: "declaration";
      /**
       * Namespace containing the declaration.
       */
      namespaceId: string;
      /**
       * Canonical owner module.
       */
      ownerModuleId: string;
      [k: string]: unknown;
    }
  | {
      /**
       * Named upstream export.
       */
      exportId: string;
      /**
       * Module declaring the import.
       */
      importerModuleId: string;
      kind: "import";
      /**
       * Namespace bound in the importing module.
       */
      localNamespaceId: string;
      /**
       * Namespace selected from the upstream export.
       */
      sourceNamespaceId: string;
      /**
       * Selected upstream module.
       */
      upstreamModuleId: string;
      /**
       * Selected upstream release.
       */
      upstreamReleaseId: string;
      [k: string]: unknown;
    }
  | {
      /**
       * Namespace that owns the extension rule.
       */
      extensionNamespaceId: string;
      kind: "extension";
      /**
       * Module that owns the extension.
       */
      ownerModuleId: string;
      /**
       * Identity of the extended declaration.
       */
      targetDeclarationId: string;
      /**
       * Namespace of the extended declaration.
       */
      targetNamespaceId: string;
      [k: string]: unknown;
    }
  | {
      /**
       * Declaration identity local to the namespace.
       */
      declarationId: string;
      kind: "policy";
      /**
       * Namespace containing the declaration.
       */
      namespaceId: string;
      /**
       * Canonical owner module.
       */
      ownerModuleId: string;
      [k: string]: unknown;
    }
  | {
      kind: "responsibility-closure";
      [k: string]: unknown;
    }
  | {
      kind: "organizational-translation-closure";
      [k: string]: unknown;
    }
  | {
      kind: "experience-composition";
      [k: string]: unknown;
    }
  | {
      kind: "resource";
      /**
       * Module selecting the resource.
       */
      ownerModuleId: string;
      /**
       * Workspace-local resource identity.
       */
      resourceId: string;
      [k: string]: unknown;
    }
  | {
      kind: "resolution-lock";
      [k: string]: unknown;
    }
  | {
      kind: "universe-lock";
      [k: string]: unknown;
    }
  | {
      kind: "workspace";
      /**
       * Principal-relative workspace identity.
       */
      workspaceId: string;
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
 * Maximum repository access admitted by the execution-workspace edge.
 */
export type EffectiveProfileRepositoryAccess = "read" | "mutate";
/**
 * Owner-native realization evidence available to this inspection.
 */
export type EffectiveProfileNativeRealization = "unknown";
/**
 * Exact three-state operation exposure contract.
 */
export type EffectiveProfileOperationExposure = "unbound" | "semantic-only" | "capability-bearing";
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
 * The five deliberately shallow Profile Explorer sections.
 */
export type EffectiveProfileSection =
  | "intended-use-case"
  | "effective-configuration"
  | "capabilities-and-operations"
  | "machine-realization"
  | "gaps-and-explanations";
/**
 * Admission relationship between current editor input and the shown profile.
 */
export type EffectiveProfileState =
  | {
      kind: "current";
      [k: string]: unknown;
    }
  | {
      /**
       * Editor coordinate that admitted the displayed predecessor.
       */
      admittedEditor: EditorCoordinateOutput;
      /**
       * API-owned identity of the displayed predecessor.
       */
      admittedProfileDigest: string;
      kind: "stale";
      /**
       * Plain owner summary for the rejected current input.
       */
      summary: string;
      [k: string]: unknown;
    }
  | {
      kind: "not-admitted";
      /**
       * Plain owner summary for the absent current product.
       */
      summary: string;
      [k: string]: unknown;
    };

/**
 * Typed LSP envelope around one API-owned effective-profile page.
 */
export interface EffectiveProfileResponse {
  /**
   * Ephemeral selection against which the response was admitted.
   */
  context: ProfileInspectionContext;
  /**
   * Absent only when no admitted profile exists for the selected context.
   */
  coordinates?: EffectiveProfileCoordinates;
  /**
   * Exact API diagnostic also published through standard LSP, when present.
   */
  diagnosticIdentity?: DiagnosticIdentity;
  /**
   * Exact current editor request coordinate.
   */
  editor: EditorCoordinateOutput;
  /**
   * Absent only when no admitted profile exists for the selected context.
   */
  page?: EffectiveProfilePage;
  /**
   * Truthful relationship between editor input and displayed evidence.
   */
  state: EffectiveProfileState;
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
 * One deterministic page from a retained effective-profile projection.
 */
export interface EffectiveProfilePage {
  /**
   * Complete entries in the requested bounded window.
   */
  entries: EffectiveProfileEntry[];
  /**
   * Next zero-based offset, or `None` for the terminal page.
   */
  nextOffset: number | null;
  /**
   * Exact zero-based offset requested for this page.
   *
   * This remains the requested coordinate even when it lies beyond the
   * terminal entry. Language transports use it to reject cross-page reply
   * substitution without interpreting profile content.
   */
  offset: number;
  /**
   * Section represented by this page.
   */
  section: EffectiveProfileSection;
}
/**
 * Exact opaque provider reference selected at a path's terminal leaf.
 */
export interface EffectiveProfileProviderBindingRef {
  /**
   * Stable provider-binding locator.
   */
  providerBindingId: string;
  /**
   * Domain-separated identity of the complete provider-binding body.
   */
  providerBindingIdentity: string;
}
