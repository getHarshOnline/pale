/* Generated from Pale LSP's private Rust wire types. Do not edit. */

/**
 * Closed transport projection of the retained improvement typestate.
 */
export type ConfigurationImprovementResponse =
  | {
      /**
       * Exact editor/profile product selected as baseline.
       */
      baseline: ExpectedProfileCoordinateOutput;
      /**
       * Ephemeral selection bound to the review.
       */
      context: ProfileInspectionContext;
      state: "explained";
    }
  | {
      /**
       * Ephemeral selection bound to the review.
       */
      context: ProfileInspectionContext;
      /**
       * Exact review product coordinate.
       */
      coordinate: ImprovementPreviewCoordinate;
      /**
       * API-owned bounded preview projection.
       */
      preview: ConfigurationImprovementPreviewProjection;
      state: "previewed";
    }
  | {
      /**
       * Canonical change-root page.
       */
      changeRoots: ChangeRootPage;
      /**
       * Ephemeral selection bound to the review.
       */
      context: ProfileInspectionContext;
      /**
       * Exact review product coordinate.
       */
      coordinate: ImprovementPreviewCoordinate;
      /**
       * Canonical current impact-partition page.
       */
      impacts: ImpactPage;
      state: "details";
    }
  | {
      /**
       * Ephemeral selection bound to the review.
       */
      context: ProfileInspectionContext;
      /**
       * Exact review product coordinate.
       */
      coordinate: ImprovementPreviewCoordinate;
      /**
       * Canonical comparison-owned causal result.
       */
      explanation: Explanation;
      /**
       * Exact candidate-owned source result.
       */
      source: ConfigurationImprovementSourceProjection;
      state: "subject-details";
    }
  | {
      /**
       * Ephemeral selection bound to the review.
       */
      context: ProfileInspectionContext;
      /**
       * Exact reviewed preview coordinate.
       */
      coordinate: ImprovementPreviewCoordinate;
      /**
       * Exact terminal disposition selected or proved stale.
       */
      disposition: ConfigurationImprovementDisposition;
      /**
       * API-owned bounded preview projection.
       */
      preview: ConfigurationImprovementPreviewProjection;
      state: "terminal";
    };
/**
 * Exact digest transition established by candidate diagnosis.
 */
export type ConfigurationImprovementCoordinateChange =
  | {
      /**
       * Shared canonical digest.
       */
      digest: string;
      state: "unchanged";
    }
  | {
      /**
       * Canonical digest produced by candidate compilation.
       */
      candidate_digest: string;
      /**
       * Canonical digest of the explained baseline.
       */
      prior_digest: string;
      state: "changed";
    };
/**
 * One exact source-change transition.
 */
export type Change =
  | {
      current: Fact;
      kind: "added";
      [k: string]: unknown;
    }
  | {
      current: Fact;
      kind: "modified";
      prior: Fact;
      [k: string]: unknown;
    }
  | {
      kind: "removed";
      prior: Fact;
      [k: string]: unknown;
    };
/**
 * API-owned typed fact projected without collapsing digest domains.
 */
export type Fact =
  | {
      byteLen: number;
      kind: "source-content";
      sourceDigest: string;
      sourcePath: string;
      [k: string]: unknown;
    }
  | {
      componentDigest: string;
      kind: "component";
      [k: string]: unknown;
    }
  | {
      kind: "module";
      moduleDigest: string;
      [k: string]: unknown;
    }
  | {
      kind: "declaration";
      valueDigest: string;
      [k: string]: unknown;
    }
  | {
      kind: "import-surface";
      surfaceDigest: string;
      [k: string]: unknown;
    }
  | {
      kind: "extension";
      valueDigest: string;
      [k: string]: unknown;
    }
  | {
      kind: "effective-policy";
      valueDigest: string;
      [k: string]: unknown;
    }
  | {
      closureDigest: string;
      kind: "responsibility-closure";
      [k: string]: unknown;
    }
  | {
      closureDigest: string;
      kind: "organizational-translation-closure";
      [k: string]: unknown;
    }
  | {
      kind: "experience-composition";
      sourceDigest: string;
      [k: string]: unknown;
    }
  | {
      kind: "resource";
      valueDigest: string;
      [k: string]: unknown;
    }
  | {
      kind: "resolution-lock";
      resolutionDigest: string;
      [k: string]: unknown;
    }
  | {
      kind: "universe-lock";
      universeDigest: string;
      [k: string]: unknown;
    }
  | {
      kind: "semantic-ir";
      workspaceSemanticIrDigest: string;
      [k: string]: unknown;
    };
/**
 * Closed transport spelling of [`PortableWorkspaceSubjectSelector`].
 */
export type SubjectSelector =
  | {
      kind: "source";
      sourceId: string;
    }
  | {
      componentId: string;
      kind: "component";
    }
  | {
      kind: "module";
      moduleId: string;
    }
  | {
      declarationId: string;
      kind: "declaration";
      namespaceId: string;
      ownerModuleId: string;
    }
  | {
      exportId: string;
      importerModuleId: string;
      kind: "import";
      localNamespaceId: string;
      sourceNamespaceId: string;
      upstreamModuleId: string;
      upstreamReleaseId: string;
    }
  | {
      extensionNamespaceId: string;
      kind: "extension";
      ownerModuleId: string;
      targetDeclarationId: string;
      targetNamespaceId: string;
    }
  | {
      declarationId: string;
      kind: "policy";
      namespaceId: string;
      ownerModuleId: string;
    }
  | {
      kind: "responsibility-closure";
    }
  | {
      kind: "organizational-translation-closure";
    }
  | {
      kind: "experience-composition";
    }
  | {
      kind: "resource";
      ownerModuleId: string;
      resourceId: string;
    }
  | {
      kind: "resolution-lock";
    }
  | {
      kind: "universe-lock";
    }
  | {
      kind: "workspace";
      workspaceId: string;
    };
/**
 * Closed reachability classification.
 */
export type ImpactState = "affected" | "unaffected";
/**
 * Proof DAG selected for one causal witness.
 */
export type ExplanationFrontier = "predecessor" | "current";
/**
 * Semantic owner/provenance classification for one proof fact.
 */
export type Provenance =
  | {
      kind: "source";
      sourceId: string;
      [k: string]: unknown;
    }
  | {
      kind: "module-release";
      moduleDigest: string;
      moduleId: string;
      releaseId: string;
      [k: string]: unknown;
    }
  | {
      exportId: string;
      importerModuleId: string;
      kind: "import";
      localNamespaceId: string;
      sourceNamespaceId: string;
      upstreamModuleId: string;
      upstreamReleaseId: string;
      [k: string]: unknown;
    }
  | {
      extensionNamespaceId: string;
      kind: "extension";
      ownerModuleId: string;
      targetDeclarationId: string;
      targetNamespaceId: string;
      [k: string]: unknown;
    }
  | {
      kind: "compilation";
      [k: string]: unknown;
    };
/**
 * Deterministic compiler rule that produced one causal step.
 */
export type DerivationRule =
  | "source-admission"
  | "component-admission"
  | "module-sealing"
  | "declaration-linking"
  | "import-linking"
  | "extension-application"
  | "policy-composition"
  | "responsibility-lowering"
  | "organizational-translation"
  | "experience-composition"
  | "resource-selection"
  | "resolution-locking"
  | "universe-locking"
  | "semantic-ir-sealing";
/**
 * Closed explanation state.
 */
export type ExplanationState = "not-present" | "unaffected" | "affected";
/**
 * One exact source path segment retained by candidate compilation.
 */
export type SourcePathSegment =
  | {
      kind: "field";
      value: string;
      [k: string]: unknown;
    }
  | {
      kind: "index";
      value: number;
      [k: string]: unknown;
    };
/**
 * Closed role of one compiler-proved source occurrence.
 */
export type SourceOccurrenceRole = "definition" | "reference" | "projection";
/**
 * Explicit terminal disposition after a complete preview.
 */
export type ConfigurationImprovementDisposition = "accepted" | "rejected" | "cancelled" | "stale";

/**
 * Borrowed serialized profile coordinate; editor counters are decimal strings.
 */
export interface ExpectedProfileCoordinateOutput {
  compositionPlanDigest: string;
  documentRevision: string;
  generation: string;
  instanceDigest: string;
  packageSemanticIrDigest: string;
  profileDigest: string;
  selectedTopologyDigest: string;
  workspaceAdmissionEvidenceDigest: string;
  workspaceSemanticIrDigest: string;
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
 * Exact preview product required before terminal disposition.
 */
export interface ImprovementPreviewCoordinate {
  /**
   * Baseline editor and compiler product.
   */
  baseline: ExpectedProfileCoordinateOutput;
  /**
   * Editor snapshot admitted as the candidate.
   */
  candidateEditor: EditorCoordinateOutput;
  /**
   * Candidate private-instance identity.
   */
  candidateInstanceDigest: string;
  /**
   * Candidate effective-profile identity.
   */
  candidateProfileDigest: string;
  /**
   * Candidate selected-topology identity.
   */
  candidateSelectedTopologyDigest: string;
  /**
   * Exact candidate derivation proof.
   */
  derivationReceiptDigest: string;
}
/**
 * Serialized editor counters, not JavaScript numbers.
 */
export interface EditorCoordinateOutput {
  documentRevision: string;
  generation: string;
}
/**
 * Bounded editor transport over the API-owned preview summary.
 */
export interface ConfigurationImprovementPreviewProjection {
  /**
   * Number of affected semantic subjects.
   */
  affectedCount: number;
  /**
   * Baseline workspace admission-evidence identity.
   */
  baselineWorkspaceAdmissionEvidenceDigest: string;
  /**
   * Baseline workspace semantic identity.
   */
  baselineWorkspaceSemanticIrDigest: string;
  /**
   * Candidate private-instance identity.
   */
  candidateInstanceDigest: string;
  /**
   * Candidate effective-profile identity.
   */
  candidateProfileDigest: string;
  /**
   * Candidate workspace admission-evidence identity.
   */
  candidateWorkspaceAdmissionEvidenceDigest: string;
  /**
   * Candidate workspace semantic identity.
   */
  candidateWorkspaceSemanticIrDigest: string;
  /**
   * Number of exact changed roots.
   */
  changeRootCount: number;
  /**
   * Exact composition-proof transition.
   */
  compositionPlanChange: ConfigurationImprovementCoordinateChange;
  /**
   * Exact derivation receipt.
   */
  derivationReceiptDigest: string;
  /**
   * Selected execution workspace.
   */
  executionWorkspaceId: string;
  /**
   * Exact private-instance transition.
   */
  instanceChange: ConfigurationImprovementCoordinateChange;
  /**
   * Baseline private-instance identity.
   */
  instanceDigest: string;
  /**
   * Exact selected Package semantic transition.
   */
  packageSemanticIrChange: ConfigurationImprovementCoordinateChange;
  /**
   * Exact effective-profile transition.
   */
  profileChange: ConfigurationImprovementCoordinateChange;
  /**
   * Baseline effective-profile identity.
   */
  profileDigest: string;
  /**
   * Selected project.
   */
  projectId: string;
  /**
   * Exact desired-topology transition.
   */
  selectedTopologyChange: ConfigurationImprovementCoordinateChange;
  /**
   * Number of unaffected semantic subjects.
   */
  unaffectedCount: number;
  /**
   * Principal workspace under review.
   */
  workspaceId: string;
}
/**
 * One bounded page of exact comparison roots.
 */
export interface ChangeRootPage {
  entries: ChangeRoot[];
  nextOffset: number | null;
  [k: string]: unknown;
}
/**
 * One exact comparison root.
 */
export interface ChangeRoot {
  change: Change;
  rootDigest: string;
  subject: SubjectSelector;
  [k: string]: unknown;
}
/**
 * One bounded page of the complete current impact partition.
 */
export interface ImpactPage {
  entries: Impact[];
  nextOffset: number | null;
  [k: string]: unknown;
}
/**
 * One current causal subject and its root-backed reachability state.
 */
export interface Impact {
  contributingRootCount: number;
  state: ImpactState;
  subject: SubjectSelector;
  [k: string]: unknown;
}
/**
 * Result of one typed causal explanation request.
 */
export interface Explanation {
  causal?: CausalExplanation;
  state: ExplanationState;
  subject: SubjectSelector;
  [k: string]: unknown;
}
/**
 * Deterministically selected causal witness.
 */
export interface CausalExplanation {
  frontier: ExplanationFrontier;
  root: ChangeRoot;
  steps: CausalStep[];
  [k: string]: unknown;
}
/**
 * One content-addressed fact on a causal witness.
 */
export interface CausalStep {
  fact: Fact;
  nodeDigest: string;
  provenance: Provenance;
  rule: DerivationRule;
  subject: SubjectSelector;
  [k: string]: unknown;
}
/**
 * Bounded candidate-source evidence for one review subject.
 */
export interface ConfigurationImprovementSourceProjection {
  inspected: number;
  occurrences: SourceOccurrence[];
  recordAnchor?: SourceLocation;
  sourcesComplete: boolean;
  subject: SubjectSelector;
}
/**
 * One exact candidate source occurrence.
 */
export interface SourceOccurrence {
  location: SourceLocation;
  role: SourceOccurrenceRole;
}
/**
 * One exact compiler-owned logical source location.
 */
export interface SourceLocation {
  path: SourcePathSegment[];
  sourceLabel: string;
}
