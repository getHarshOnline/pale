//! Proof-carrying causal receipts for workspace semantic derivation.
//!
//! Receipts bind deterministic compiler provenance and impact classification;
//! they do not authorize runtime, provider, or native effects.

use super::*;

macro_rules! workspace_byte_digest {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(
            Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
            schemars::JsonSchema,
        )]
        #[serde(transparent)]
        pub struct $name(ArtifactDigest);

        impl $name {
            /// Validate a claimed lowercase SHA-256 representation.
            ///
            /// # Errors
            ///
            /// Returns [`crate::records::InvalidDigest`] unless `value` is exactly 64
            /// lowercase hexadecimal characters. Parsing does not observe the
            /// bytes or canonical record claimed by the digest.
            pub fn parse(value: impl Into<String>) -> Result<Self, super::super::InvalidDigest> {
                ArtifactDigest::parse(value).map(Self)
            }

            /// Compute this identity from the exact supplied bytes.
            #[must_use]
            pub fn hash_bytes(bytes: &[u8]) -> Self {
                Self(ArtifactDigest::hash_bytes(bytes))
            }

            /// Borrow the lowercase hexadecimal wire representation.
            #[must_use]
            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(self.as_str())
            }
        }
    };
}

macro_rules! workspace_canonical_digest {
    ($(#[$meta:meta])* $name:ident) => {
        workspace_byte_digest!($(#[$meta])* $name);

        impl $name {
            /// Compute this identity from a domain-bearing RFC 8785 preimage.
            ///
            /// # Errors
            ///
            /// Returns [`crate::canon::CanonError`] when `value` cannot satisfy
            /// the canonical record contract.
            pub fn hash_canonical<T: Serialize + ?Sized>(
                value: &T,
            ) -> Result<Self, crate::canon::CanonError> {
                let canonical = crate::canon::to_canonical_string(value)?;
                Ok(Self::hash_bytes(canonical.as_bytes()))
            }
        }
    };
}

workspace_byte_digest!(
    /// SHA-256 of the exact bounded source bytes consumed by workspace admission.
    WorkspaceSourceContentDigest
);
workspace_canonical_digest!(
    /// Content identity of one canonical causal derivation node.
    WorkspaceDerivationNodeDigest
);
workspace_canonical_digest!(
    /// Content identity of one derived fact whose native digest has another owner.
    WorkspaceDerivationValueDigest
);
workspace_canonical_digest!(
    /// Content identity of one canonical derivation change root.
    WorkspaceDerivationRootDigest
);

/// One bounded owner-relative source root retained by a derivation receipt.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceDerivationSourceV1 {
    /// Receipt-local identity referenced by causal origins.
    pub source_id: WorkspaceDerivationSourceId,
    /// Confined owner-relative label, never an absolute path or URL.
    pub source_path: OwnedConfinedRelativePath,
    /// Exact bytes admitted from this source.
    pub source_digest: WorkspaceSourceContentDigest,
    /// Exact admitted byte length.
    pub byte_len: u32,
}

/// Typed provenance owner for one current causal fact.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(tag = "origin", rename_all = "snake_case", deny_unknown_fields)]
pub enum WorkspaceDerivationOriginV1 {
    /// Fact admitted directly from a retained source root.
    Source {
        /// Source whose exact bytes contain the fact.
        source_id: WorkspaceDerivationSourceId,
    },
    /// Fact owned by one exact selected module release.
    ModuleRelease {
        /// Owning module.
        module_id: WorkspaceModuleId,
        /// Exact release label.
        release_id: WorkspaceReleaseId,
        /// Exact sealed release identity.
        module_digest: WorkspaceModuleDigest,
    },
    /// Fact derived from one exact selected import path.
    Import {
        /// Importing module.
        importer_module: WorkspaceModuleId,
        /// Exact upstream module.
        upstream_module: WorkspaceModuleId,
        /// Exact upstream release.
        upstream_release: WorkspaceReleaseId,
        /// Named upstream export.
        export_id: WorkspaceExportId,
        /// Namespace selected in that export.
        source_namespace: WorkspaceNamespaceId,
        /// Importer-local namespace.
        local_namespace: WorkspaceNamespaceId,
    },
    /// Fact derived from one exact workspace extension.
    Extension {
        /// Module that owns the extension.
        owner_module: WorkspaceModuleId,
        /// Namespace that owns the extension.
        namespace: WorkspaceNamespaceId,
        /// Declaration extended by the rule.
        target: WorkspaceDeclarationRefV1,
    },
    /// Aggregate produced by the deterministic workspace compiler.
    Compilation,
}

/// Closed rule vocabulary for the initial-v1 workspace derivation algebra.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceDerivationRuleV1 {
    /// Admit exact source bytes.
    SourceAdmission,
    /// Admit and seal one semantic source component.
    ComponentAdmission,
    /// Seal one exact module candidate.
    ModuleSealing,
    /// Resolve one declaration into its canonical owner coordinate.
    DeclarationLinking,
    /// Resolve one exact import surface.
    ImportLinking,
    /// Apply one typed extension or narrowing.
    ExtensionApplication,
    /// Meet ceilings and accumulate obligations for one declaration.
    PolicyComposition,
    /// Lower the responsibility sources explicitly selected by the root manifest.
    ResponsibilityLowering,
    /// Lower the optional explicit organizational translation.
    OrganizationalTranslation,
    /// Admit and normalize explicit neutral composition declarations.
    ExperienceComposition,
    /// Retain one exact logical or release-bound resource selection.
    ResourceSelection,
    /// Bind the exact resolution-lock frontier.
    ResolutionLocking,
    /// Bind the exact principal-adopted universe frontier.
    UniverseLocking,
    /// Seal the complete path-free semantic IR.
    SemanticIrSealing,
}

/// Canonical subject of one causal fact or impact classification.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(tag = "subject", rename_all = "snake_case", deny_unknown_fields)]
pub enum WorkspaceDerivationSubjectV1 {
    /// One retained source root.
    Source {
        /// Receipt-local source identity.
        source_id: WorkspaceDerivationSourceId,
    },
    /// One admitted semantic source component.
    Component {
        /// Component identity.
        component_id: WorkspaceComponentId,
    },
    /// One selected module release.
    Module {
        /// Module identity.
        module_id: WorkspaceModuleId,
    },
    /// One resolved semantic declaration.
    Declaration {
        /// Canonical owner module.
        owner_module: WorkspaceModuleId,
        /// Owner-local declaration reference.
        declaration: WorkspaceDeclarationRefV1,
    },
    /// One exact import path.
    Import {
        /// Importing module.
        importer_module: WorkspaceModuleId,
        /// Exact upstream module.
        upstream_module: WorkspaceModuleId,
        /// Exact upstream release.
        upstream_release: WorkspaceReleaseId,
        /// Named upstream export.
        export_id: WorkspaceExportId,
        /// Namespace selected in that export.
        source_namespace: WorkspaceNamespaceId,
        /// Importer-local namespace.
        local_namespace: WorkspaceNamespaceId,
    },
    /// One exact extension.
    Extension {
        /// Module owning the extension.
        owner_module: WorkspaceModuleId,
        /// Namespace owning the extension.
        namespace: WorkspaceNamespaceId,
        /// Declaration extended by the rule.
        target: WorkspaceDeclarationRefV1,
    },
    /// Effective policy for one resolved declaration.
    Policy {
        /// Canonical owner module.
        owner_module: WorkspaceModuleId,
        /// Owner-local declaration reference.
        declaration: WorkspaceDeclarationRefV1,
    },
    /// Canonically lowered manifest-selected responsibility set.
    ResponsibilityClosure,
    /// Complete optional organizational translation closure.
    OrganizationalTranslationClosure,
    /// Complete explicit neutral composition declaration closure.
    ExperienceComposition,
    /// One logical or release-bound resource selection.
    Resource {
        /// Selecting module.
        owner_module: WorkspaceModuleId,
        /// Workspace-local resource identity.
        resource_id: WorkspaceResourceId,
    },
    /// Exact resolution-lock record.
    ResolutionLock,
    /// Exact universe-lock record.
    UniverseLock,
    /// Complete workspace semantic IR.
    Workspace {
        /// Principal-relative workspace identity.
        workspace_id: WorkspaceId,
    },
}

/// Typed output carried by one causal node or predecessor change root.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(tag = "output", rename_all = "snake_case", deny_unknown_fields)]
pub enum WorkspaceDerivationOutputV1 {
    /// Exact admitted source bytes.
    SourceContent {
        /// Portable owner-relative source label.
        source_path: OwnedConfinedRelativePath,
        /// Exact byte identity.
        source_digest: WorkspaceSourceContentDigest,
        /// Exact admitted source length.
        byte_len: u32,
    },
    /// Canonical semantic component.
    Component {
        /// Exact component identity.
        component_digest: WorkspaceComponentDigest,
    },
    /// Canonical module candidate.
    Module {
        /// Exact module identity.
        module_digest: WorkspaceModuleDigest,
    },
    /// Canonically resolved declaration fact.
    Declaration {
        /// Domain-separated derived value identity.
        value_digest: WorkspaceDerivationValueDigest,
    },
    /// Canonically resolved import surface.
    ImportSurface {
        /// Existing import-surface identity owned by linking.
        surface_digest: WorkspaceImportSurfaceDigest,
    },
    /// Canonically applied extension fact.
    Extension {
        /// Domain-separated derived value identity.
        value_digest: WorkspaceDerivationValueDigest,
    },
    /// Canonically accumulated effective policy.
    EffectivePolicy {
        /// Domain-separated derived value identity.
        value_digest: WorkspaceDerivationValueDigest,
    },
    /// Canonically lowered responsibility set.
    ResponsibilityClosure {
        /// Domain-separated identity of the manifest-selected lowered set.
        closure_digest: ResponsibilityDerivationClosureDigest,
    },
    /// Canonically lowered optional organizational translation.
    OrganizationalTranslationClosure {
        /// Domain-separated identity of the complete optional closure.
        closure_digest: OrganizationalTranslationClosureDigest,
    },
    /// Explicit neutral composition declaration closure.
    ExperienceComposition {
        /// Domain-separated identity of the complete declaration set.
        source_digest: ExperienceCompositionSourceDigest,
    },
    /// Canonical selected-resource fact.
    Resource {
        /// Domain-separated derived value identity.
        value_digest: WorkspaceDerivationValueDigest,
    },
    /// Exact resolution-lock identity.
    ResolutionLock {
        /// Sealed lock identity.
        resolution_digest: WorkspaceResolutionLockDigest,
    },
    /// Exact universe-lock identity.
    UniverseLock {
        /// Sealed lock identity.
        universe_digest: WorkspaceUniverseLockDigest,
    },
    /// Exact workspace semantic-IR identity.
    SemanticIr {
        /// Sealed path-free semantic identity.
        workspace_semantic_ir_digest: WorkspaceSemanticIrDigest,
    },
}

/// One content-addressed fact in the finite workspace causal DAG.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceDerivationNodeV1 {
    /// Content address of this node's canonical preimage.
    pub node_digest: WorkspaceDerivationNodeDigest,
    /// Typed owner/provenance classification.
    pub origin: WorkspaceDerivationOriginV1,
    /// Closed deterministic rule applied at this node.
    pub rule: WorkspaceDerivationRuleV1,
    /// Exact semantic subject derived by the rule.
    pub subject: WorkspaceDerivationSubjectV1,
    /// Typed fact produced for the subject.
    pub output: WorkspaceDerivationOutputV1,
    /// Sorted unique direct causal predecessors.
    #[schemars(schema_with = "derivation_inputs_schema")]
    pub inputs: Vec<WorkspaceDerivationNodeDigest>,
}

impl WorkspaceDerivationNodeV1 {
    /// Seal one bounded causal fact from already content-addressed inputs.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidWorkspaceDerivationReceipt`] for an excessive or
    /// duplicate input set, an incompatible rule/subject/output triple, or a
    /// canonical encoding failure. Cross-node references are checked by the
    /// containing [`WorkspaceDerivationReceiptV1`].
    pub fn seal(
        origin: WorkspaceDerivationOriginV1,
        rule: WorkspaceDerivationRuleV1,
        subject: WorkspaceDerivationSubjectV1,
        output: WorkspaceDerivationOutputV1,
        mut inputs: Vec<WorkspaceDerivationNodeDigest>,
    ) -> Result<Self, InvalidWorkspaceDerivationReceipt> {
        bounded_derivation_len("node input", inputs.len(), derivation_input_limit(rule))?;
        inputs.sort();
        if !strictly_ordered(inputs.iter()) && !inputs.is_empty() {
            return Err(InvalidWorkspaceDerivationReceipt::NodeInputOrder);
        }
        validate_derivation_fact(&origin, rule, &subject, &output)?;
        let node_digest = derivation_node_digest(&origin, rule, &subject, &output, &inputs)
            .map_err(InvalidWorkspaceDerivationReceipt::Canonical)?;
        Ok(Self {
            node_digest,
            origin,
            rule,
            subject,
            output,
            inputs,
        })
    }

    fn recompute_digest(&self) -> Result<WorkspaceDerivationNodeDigest, crate::canon::CanonError> {
        derivation_node_digest(
            &self.origin,
            self.rule,
            &self.subject,
            &self.output,
            &self.inputs,
        )
    }
}

/// Closed transition proved by one change root.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(tag = "change", rename_all = "snake_case", deny_unknown_fields)]
pub enum WorkspaceDerivationRootChangeV1 {
    /// Subject has no predecessor fact in the compared frontier.
    Added {
        /// Exact current causal node.
        current_node: WorkspaceDerivationNodeDigest,
    },
    /// Subject exists in both frontiers with different typed output.
    Modified {
        /// Exact predecessor output.
        prior_output: WorkspaceDerivationOutputV1,
        /// Exact current causal node.
        current_node: WorkspaceDerivationNodeDigest,
    },
    /// Subject exists only in the predecessor frontier.
    Removed {
        /// Exact predecessor output.
        prior_output: WorkspaceDerivationOutputV1,
    },
}

/// One content-addressed root cause in an incremental derivation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceDerivationRootV1 {
    /// Content address of this root's canonical preimage.
    pub root_digest: WorkspaceDerivationRootDigest,
    /// Semantic fact that changed.
    pub subject: WorkspaceDerivationSubjectV1,
    /// Exact closed transition.
    #[serde(flatten)]
    pub change: WorkspaceDerivationRootChangeV1,
}

impl WorkspaceDerivationRootV1 {
    /// Seal one typed change root.
    ///
    /// # Errors
    ///
    /// Returns [`crate::canon::CanonError`] when the root preimage cannot be
    /// represented canonically. Node existence is checked by the receipt.
    pub fn seal(
        subject: WorkspaceDerivationSubjectV1,
        change: WorkspaceDerivationRootChangeV1,
    ) -> Result<Self, crate::canon::CanonError> {
        let root_digest = derivation_root_digest(&subject, &change)?;
        Ok(Self {
            root_digest,
            subject,
            change,
        })
    }
}

/// Closed impact disposition for one current causal subject.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "impact", rename_all = "snake_case", deny_unknown_fields)]
pub enum WorkspaceDerivationImpactStateV1 {
    /// Subject is reachable from one or more exact change roots.
    Affected {
        /// Sorted unique roots that justify reachability.
        #[schemars(schema_with = "derivation_root_refs_schema")]
        root_digests: Vec<WorkspaceDerivationRootDigest>,
    },
    /// Subject lies outside every root's bounded impact closure.
    Unaffected,
}

/// Complete affected/unaffected classification for one current causal subject.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceDerivationImpactV1 {
    /// Current causal subject classified exactly once.
    pub subject: WorkspaceDerivationSubjectV1,
    /// Root-backed impact disposition.
    #[serde(flatten)]
    pub state: WorkspaceDerivationImpactStateV1,
}

/// Unsealed causal-proof preimage consumed by [`WorkspaceDerivationReceiptV1::seal`].
///
/// Separating this aggregate from the sealed record makes the trust transition
/// explicit: callers supply facts, while `seal` canonicalizes and verifies the
/// complete bounded proof before assigning its receipt identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceDerivationReceiptPreimageV1 {
    /// Exact predecessor resolution lock.
    pub prior_resolution_digest: WorkspaceResolutionLockDigest,
    /// Exact current resolution lock.
    pub current_resolution_digest: WorkspaceResolutionLockDigest,
    /// Exact predecessor universe lock.
    pub prior_universe_digest: WorkspaceUniverseLockDigest,
    /// Exact current universe lock.
    pub current_universe_digest: WorkspaceUniverseLockDigest,
    /// Exact current semantic IR identity.
    pub workspace_semantic_ir_digest: WorkspaceSemanticIrDigest,
    /// Current source inventory.
    pub sources: Vec<WorkspaceDerivationSourceV1>,
    /// Current finite causal DAG.
    pub nodes: Vec<WorkspaceDerivationNodeV1>,
    /// Exact changed-fact roots.
    pub roots: Vec<WorkspaceDerivationRootV1>,
    /// Complete current impact partition.
    pub impacts: Vec<WorkspaceDerivationImpactV1>,
}

/// Self-identifying finite causal proof for one workspace derivation.
///
/// The node DAG records provenance and deterministic rules; roots record exact
/// predecessor/current differences; impacts form a total partition of current
/// subjects. [`Self::verify`] proves structural self-integrity and exact
/// reachability from added/modified roots through the current DAG. Removed-root
/// impact requires the predecessor dependency frontier, so the application must
/// compare this record with both verified IRs to establish that attribution as
/// well as overall semantic completeness and truth.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceDerivationReceiptV1 {
    /// Stable wire schema identity.
    #[schemars(extend("const" = "pale/workspace-derivation-receipt.v1"))]
    pub schema: String,
    /// Initial contract version.
    #[schemars(extend("const" = 1))]
    pub schema_version: u32,
    /// Exact predecessor resolution lock.
    pub prior_resolution_digest: WorkspaceResolutionLockDigest,
    /// Exact current resolution lock.
    pub current_resolution_digest: WorkspaceResolutionLockDigest,
    /// Exact predecessor universe lock.
    pub prior_universe_digest: WorkspaceUniverseLockDigest,
    /// Exact current universe lock.
    pub current_universe_digest: WorkspaceUniverseLockDigest,
    /// Exact current semantic IR identity.
    pub workspace_semantic_ir_digest: WorkspaceSemanticIrDigest,
    /// Canonical source inventory consumed by current derivation nodes.
    #[schemars(schema_with = "derivation_sources_schema")]
    pub sources: Vec<WorkspaceDerivationSourceV1>,
    /// Canonical finite causal DAG for current semantic facts.
    #[schemars(schema_with = "derivation_nodes_schema")]
    pub nodes: Vec<WorkspaceDerivationNodeV1>,
    /// Canonical set of changed predecessor/current facts.
    #[schemars(schema_with = "derivation_roots_schema")]
    pub roots: Vec<WorkspaceDerivationRootV1>,
    /// Total affected/unaffected partition of current causal subjects.
    #[schemars(schema_with = "derivation_impacts_schema")]
    pub impacts: Vec<WorkspaceDerivationImpactV1>,
    /// Domain-separated canonical receipt identity.
    pub receipt_digest: WorkspaceDerivationReceiptDigest,
}

impl WorkspaceDerivationReceiptV1 {
    /// Seal one caller-supplied causal proof without asserting semantic truth.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidWorkspaceDerivationReceipt`] when any aggregate, source,
    /// node, root, DAG, impact partition, or top-level coordinate violates the
    /// initial-v1 contract, or canonical encoding fails.
    pub fn seal(
        mut preimage: WorkspaceDerivationReceiptPreimageV1,
    ) -> Result<Self, InvalidWorkspaceDerivationReceipt> {
        let WorkspaceDerivationReceiptPreimageV1 {
            prior_resolution_digest,
            current_resolution_digest,
            prior_universe_digest,
            current_universe_digest,
            workspace_semantic_ir_digest,
            ref mut sources,
            ref mut nodes,
            ref mut roots,
            ref mut impacts,
        } = preimage;
        sources.sort_by(|left, right| left.source_id.cmp(&right.source_id));
        nodes.sort_by(|left, right| left.node_digest.cmp(&right.node_digest));
        roots.sort_by(|left, right| left.root_digest.cmp(&right.root_digest));
        impacts.sort_by(|left, right| left.subject.cmp(&right.subject));
        let mut receipt = Self {
            schema: "pale/workspace-derivation-receipt.v1".into(),
            schema_version: 1,
            prior_resolution_digest,
            current_resolution_digest,
            prior_universe_digest,
            current_universe_digest,
            workspace_semantic_ir_digest,
            sources: std::mem::take(sources),
            nodes: std::mem::take(nodes),
            roots: std::mem::take(roots),
            impacts: std::mem::take(impacts),
            receipt_digest: WorkspaceDerivationReceiptDigest::hash_canonical(&(
                "pale.workspace-derivation-receipt.v1.empty",
            ))
            .map_err(InvalidWorkspaceDerivationReceipt::Canonical)?,
        };
        receipt.validate_structure()?;
        receipt.receipt_digest = receipt
            .recompute_digest()
            .map_err(InvalidWorkspaceDerivationReceipt::Canonical)?;
        Ok(receipt)
    }

    /// Verify this receipt's identity and complete finite causal-proof grammar.
    ///
    /// This check proves only self-integrity. It cannot establish that the
    /// claimed sources or changes describe real workspace inputs; the application
    /// separately reconstructs and compares the receipt from two verified IRs.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidWorkspaceDerivationReceipt`] for any structural or
    /// content-addressing violation, or when the receipt digest differs.
    pub fn verify(&self) -> Result<(), InvalidWorkspaceDerivationReceipt> {
        if self.schema != "pale/workspace-derivation-receipt.v1" || self.schema_version != 1 {
            return Err(InvalidWorkspaceDerivationReceipt::SchemaCoordinate);
        }
        self.validate_structure()?;
        let observed = self
            .recompute_digest()
            .map_err(InvalidWorkspaceDerivationReceipt::Canonical)?;
        if observed != self.receipt_digest {
            return Err(InvalidWorkspaceDerivationReceipt::DigestMismatch);
        }
        Ok(())
    }

    fn validate_structure(&self) -> Result<(), InvalidWorkspaceDerivationReceipt> {
        validate_derivation_bounds(self)?;
        validate_derivation_order(self)?;
        let sources = self
            .sources
            .iter()
            .map(|source| (&source.source_id, source))
            .collect::<std::collections::BTreeMap<_, _>>();
        for source in &self.sources {
            if looks_like_uri_reference(source.source_path.as_str()) {
                return Err(InvalidWorkspaceDerivationReceipt::SourcePath(
                    source.source_id.to_string(),
                ));
            }
        }
        let nodes = self
            .nodes
            .iter()
            .map(|node| (&node.node_digest, node))
            .collect::<std::collections::BTreeMap<_, _>>();
        let mut subjects = std::collections::BTreeMap::new();
        let mut admitted_sources = std::collections::BTreeSet::new();
        for node in &self.nodes {
            bounded_derivation_len(
                "node input",
                node.inputs.len(),
                derivation_input_limit(node.rule),
            )?;
            if !node.inputs.is_empty() && !strictly_ordered(node.inputs.iter()) {
                return Err(InvalidWorkspaceDerivationReceipt::NodeInputOrder);
            }
            validate_derivation_fact(&node.origin, node.rule, &node.subject, &node.output)?;
            if let WorkspaceDerivationOriginV1::Source { source_id } = &node.origin {
                let source = sources.get(source_id).ok_or_else(|| {
                    InvalidWorkspaceDerivationReceipt::UnknownSource(source_id.to_string())
                })?;
                if let (
                    WorkspaceDerivationSubjectV1::Source {
                        source_id: subject_source,
                    },
                    WorkspaceDerivationOutputV1::SourceContent {
                        source_path,
                        source_digest,
                        byte_len,
                    },
                ) = (&node.subject, &node.output)
                {
                    if subject_source != source_id
                        || source_path != &source.source_path
                        || source_digest != &source.source_digest
                        || byte_len != &source.byte_len
                    {
                        return Err(InvalidWorkspaceDerivationReceipt::FactKind);
                    }
                    admitted_sources.insert(subject_source.clone());
                }
            }
            if node
                .recompute_digest()
                .map_err(InvalidWorkspaceDerivationReceipt::Canonical)?
                != node.node_digest
            {
                return Err(InvalidWorkspaceDerivationReceipt::NodeDigestMismatch);
            }
            if subjects.insert(&node.subject, node).is_some() {
                return Err(InvalidWorkspaceDerivationReceipt::DuplicateSubject);
            }
            for input in &node.inputs {
                if input == &node.node_digest || !nodes.contains_key(input) {
                    return Err(InvalidWorkspaceDerivationReceipt::UnknownNode(
                        input.to_string(),
                    ));
                }
            }
            validate_derivation_inputs(node, &nodes)?;
        }
        let retained_sources = sources.keys().map(|id| (*id).clone()).collect();
        validate_source_admission_bijection(&retained_sources, &admitted_sources)?;
        let causal = validate_derivation_dag(&self.nodes)?;
        validate_derivation_roots_and_impacts(self, &nodes, &subjects, &causal)?;
        validate_derivation_coordinates(self)?;
        Ok(())
    }

    fn recompute_digest(
        &self,
    ) -> Result<WorkspaceDerivationReceiptDigest, crate::canon::CanonError> {
        WorkspaceDerivationReceiptDigest::hash_canonical(&(
            "pale.workspace-derivation-receipt.v1",
            &self.prior_resolution_digest,
            &self.current_resolution_digest,
            &self.prior_universe_digest,
            &self.current_universe_digest,
            &self.workspace_semantic_ir_digest,
            &self.sources,
            &self.nodes,
            &self.roots,
            &self.impacts,
        ))
    }
}

fn validate_source_admission_bijection(
    retained: &std::collections::BTreeSet<WorkspaceDerivationSourceId>,
    admitted: &std::collections::BTreeSet<WorkspaceDerivationSourceId>,
) -> Result<(), InvalidWorkspaceDerivationReceipt> {
    if retained == admitted {
        Ok(())
    } else {
        Err(InvalidWorkspaceDerivationReceipt::SourceAdmissionBijection)
    }
}

fn validate_derivation_inputs(
    node: &WorkspaceDerivationNodeV1,
    nodes: &std::collections::BTreeMap<&WorkspaceDerivationNodeDigest, &WorkspaceDerivationNodeV1>,
) -> Result<(), InvalidWorkspaceDerivationReceipt> {
    let inputs = node
        .inputs
        .iter()
        .map(|digest| nodes[digest])
        .collect::<Vec<_>>();
    let classes = inputs.iter().map(|input| input.rule).collect::<Vec<_>>();
    let valid = match (&node.origin, node.rule, &node.subject) {
        (_, WorkspaceDerivationRuleV1::SourceAdmission, _) => inputs.is_empty(),
        (
            WorkspaceDerivationOriginV1::Source { source_id },
            WorkspaceDerivationRuleV1::ComponentAdmission,
            _,
        )
        | (
            WorkspaceDerivationOriginV1::Source { source_id },
            WorkspaceDerivationRuleV1::ResolutionLocking,
            _,
        ) => {
            inputs.len() == 1
                && matches!(
                    &inputs[0].subject,
                    WorkspaceDerivationSubjectV1::Source { source_id: input } if input == source_id
                )
        }
        (_, WorkspaceDerivationRuleV1::DeclarationLinking, WorkspaceDerivationSubjectV1::Declaration { .. }) => {
            inputs.len() == 1 && inputs[0].rule == WorkspaceDerivationRuleV1::ComponentAdmission
        }
        (_, WorkspaceDerivationRuleV1::ResourceSelection, WorkspaceDerivationSubjectV1::Resource { owner_module, .. }) => {
            inputs.len() == 1 && matches!(&inputs[0].subject, WorkspaceDerivationSubjectV1::Module { module_id } if module_id == owner_module)
        }
        (_, WorkspaceDerivationRuleV1::ModuleSealing, WorkspaceDerivationSubjectV1::Module { .. }) => {
            classes.iter().filter(|rule| **rule == WorkspaceDerivationRuleV1::SourceAdmission).count() == 1
                && classes.iter().all(|rule| matches!(rule, WorkspaceDerivationRuleV1::SourceAdmission | WorkspaceDerivationRuleV1::ComponentAdmission))
        }
        (WorkspaceDerivationOriginV1::Source { source_id }, WorkspaceDerivationRuleV1::UniverseLocking, _) => {
            inputs.len() == 2
                && inputs.iter().any(|input| matches!(&input.subject, WorkspaceDerivationSubjectV1::Source { source_id: input } if input == source_id))
                && inputs.iter().any(|input| input.rule == WorkspaceDerivationRuleV1::ResolutionLocking)
        }
        (_, WorkspaceDerivationRuleV1::ImportLinking, WorkspaceDerivationSubjectV1::Import { importer_module, upstream_module, .. }) => {
            inputs.iter().filter(|input| matches!(&input.subject, WorkspaceDerivationSubjectV1::Module { module_id } if module_id == importer_module)).count() == 1
                && inputs.iter().filter(|input| matches!(&input.subject, WorkspaceDerivationSubjectV1::Module { module_id } if module_id == upstream_module)).count() == 1
                && inputs.len() >= 2
                && inputs.iter().all(|input| matches!(input.rule, WorkspaceDerivationRuleV1::ModuleSealing | WorkspaceDerivationRuleV1::DeclarationLinking))
        }
        (_, WorkspaceDerivationRuleV1::ExtensionApplication, WorkspaceDerivationSubjectV1::Extension { owner_module, .. }) => {
            inputs.len() >= 2
                && inputs.iter().filter(|input| matches!(&input.subject, WorkspaceDerivationSubjectV1::Module { module_id } if module_id == owner_module)).count() == 1
                && inputs.iter().all(|input| matches!(input.rule, WorkspaceDerivationRuleV1::ModuleSealing | WorkspaceDerivationRuleV1::DeclarationLinking))
        }
        (_, WorkspaceDerivationRuleV1::PolicyComposition, _) => !inputs.is_empty() && inputs.iter().all(|input| matches!(input.rule, WorkspaceDerivationRuleV1::DeclarationLinking | WorkspaceDerivationRuleV1::ImportLinking | WorkspaceDerivationRuleV1::ExtensionApplication)),
        (WorkspaceDerivationOriginV1::Compilation, WorkspaceDerivationRuleV1::ResponsibilityLowering, WorkspaceDerivationSubjectV1::ResponsibilityClosure)
        | (WorkspaceDerivationOriginV1::Compilation, WorkspaceDerivationRuleV1::OrganizationalTranslation, WorkspaceDerivationSubjectV1::OrganizationalTranslationClosure)
        | (WorkspaceDerivationOriginV1::Compilation, WorkspaceDerivationRuleV1::ExperienceComposition, WorkspaceDerivationSubjectV1::ExperienceComposition) => {
            !inputs.is_empty()
                && inputs
                    .iter()
                    .all(|input| input.rule == WorkspaceDerivationRuleV1::SourceAdmission)
        }
        (_, WorkspaceDerivationRuleV1::SemanticIrSealing, _) => {
            classes.iter().filter(|rule| **rule == WorkspaceDerivationRuleV1::SourceAdmission).count() == 1
                && classes.iter().all(|rule| matches!(rule, WorkspaceDerivationRuleV1::SourceAdmission | WorkspaceDerivationRuleV1::ModuleSealing | WorkspaceDerivationRuleV1::PolicyComposition | WorkspaceDerivationRuleV1::ResponsibilityLowering | WorkspaceDerivationRuleV1::OrganizationalTranslation | WorkspaceDerivationRuleV1::ExperienceComposition | WorkspaceDerivationRuleV1::ImportLinking | WorkspaceDerivationRuleV1::ExtensionApplication | WorkspaceDerivationRuleV1::ResourceSelection))
        }
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(InvalidWorkspaceDerivationReceipt::InputGrammar)
    }
}

fn derivation_node_digest(
    origin: &WorkspaceDerivationOriginV1,
    rule: WorkspaceDerivationRuleV1,
    subject: &WorkspaceDerivationSubjectV1,
    output: &WorkspaceDerivationOutputV1,
    inputs: &[WorkspaceDerivationNodeDigest],
) -> Result<WorkspaceDerivationNodeDigest, crate::canon::CanonError> {
    WorkspaceDerivationNodeDigest::hash_canonical(&(
        "pale.workspace-derivation-node.v1",
        origin,
        rule,
        subject,
        output,
        inputs,
    ))
}

fn derivation_root_digest(
    subject: &WorkspaceDerivationSubjectV1,
    change: &WorkspaceDerivationRootChangeV1,
) -> Result<WorkspaceDerivationRootDigest, crate::canon::CanonError> {
    WorkspaceDerivationRootDigest::hash_canonical(&(
        "pale.workspace-derivation-root.v1",
        subject,
        change,
    ))
}

fn validate_derivation_fact(
    origin: &WorkspaceDerivationOriginV1,
    rule: WorkspaceDerivationRuleV1,
    subject: &WorkspaceDerivationSubjectV1,
    output: &WorkspaceDerivationOutputV1,
) -> Result<(), InvalidWorkspaceDerivationReceipt> {
    let compatible = match (origin, rule, subject, output) {
        (
            WorkspaceDerivationOriginV1::Source { source_id: origin },
            WorkspaceDerivationRuleV1::SourceAdmission,
            WorkspaceDerivationSubjectV1::Source { source_id: subject },
            WorkspaceDerivationOutputV1::SourceContent { .. },
        ) => origin == subject,
        (
            WorkspaceDerivationOriginV1::Source { .. },
            WorkspaceDerivationRuleV1::ComponentAdmission,
            WorkspaceDerivationSubjectV1::Component { .. },
            WorkspaceDerivationOutputV1::Component { .. },
        ) => true,
        (
            WorkspaceDerivationOriginV1::ModuleRelease {
                module_id: origin,
                module_digest: origin_digest,
                ..
            },
            WorkspaceDerivationRuleV1::ModuleSealing,
            WorkspaceDerivationSubjectV1::Module { module_id: subject },
            WorkspaceDerivationOutputV1::Module { module_digest },
        ) => origin == subject && origin_digest == module_digest,
        (
            WorkspaceDerivationOriginV1::ModuleRelease { module_id, .. },
            WorkspaceDerivationRuleV1::DeclarationLinking,
            WorkspaceDerivationSubjectV1::Declaration { owner_module, .. },
            WorkspaceDerivationOutputV1::Declaration { .. },
        )
        | (
            WorkspaceDerivationOriginV1::ModuleRelease { module_id, .. },
            WorkspaceDerivationRuleV1::ResourceSelection,
            WorkspaceDerivationSubjectV1::Resource { owner_module, .. },
            WorkspaceDerivationOutputV1::Resource { .. },
        ) => module_id == owner_module,
        (
            WorkspaceDerivationOriginV1::Import {
                importer_module: oi,
                upstream_module: ou,
                upstream_release: or,
                export_id: oe,
                source_namespace: os,
                local_namespace: ol,
            },
            WorkspaceDerivationRuleV1::ImportLinking,
            WorkspaceDerivationSubjectV1::Import {
                importer_module: si,
                upstream_module: su,
                upstream_release: sr,
                export_id: se,
                source_namespace: ss,
                local_namespace: sl,
            },
            WorkspaceDerivationOutputV1::ImportSurface { .. },
        ) => (oi, ou, or, oe, os, ol) == (si, su, sr, se, ss, sl),
        (
            WorkspaceDerivationOriginV1::Extension {
                owner_module: om,
                namespace: on,
                target: ot,
            },
            WorkspaceDerivationRuleV1::ExtensionApplication,
            WorkspaceDerivationSubjectV1::Extension {
                owner_module: sm,
                namespace: sn,
                target: st,
            },
            WorkspaceDerivationOutputV1::Extension { .. },
        ) => (om, on, ot) == (sm, sn, st),
        (
            WorkspaceDerivationOriginV1::Compilation,
            WorkspaceDerivationRuleV1::PolicyComposition,
            WorkspaceDerivationSubjectV1::Policy { .. },
            WorkspaceDerivationOutputV1::EffectivePolicy { .. },
        )
        | (
            WorkspaceDerivationOriginV1::Compilation,
            WorkspaceDerivationRuleV1::ResponsibilityLowering,
            WorkspaceDerivationSubjectV1::ResponsibilityClosure,
            WorkspaceDerivationOutputV1::ResponsibilityClosure { .. },
        )
        | (
            WorkspaceDerivationOriginV1::Compilation,
            WorkspaceDerivationRuleV1::OrganizationalTranslation,
            WorkspaceDerivationSubjectV1::OrganizationalTranslationClosure,
            WorkspaceDerivationOutputV1::OrganizationalTranslationClosure { .. },
        )
        | (
            WorkspaceDerivationOriginV1::Compilation,
            WorkspaceDerivationRuleV1::ExperienceComposition,
            WorkspaceDerivationSubjectV1::ExperienceComposition,
            WorkspaceDerivationOutputV1::ExperienceComposition { .. },
        )
        | (
            WorkspaceDerivationOriginV1::Compilation,
            WorkspaceDerivationRuleV1::SemanticIrSealing,
            WorkspaceDerivationSubjectV1::Workspace { .. },
            WorkspaceDerivationOutputV1::SemanticIr { .. },
        )
        | (
            WorkspaceDerivationOriginV1::Source { .. },
            WorkspaceDerivationRuleV1::ResolutionLocking,
            WorkspaceDerivationSubjectV1::ResolutionLock,
            WorkspaceDerivationOutputV1::ResolutionLock { .. },
        )
        | (
            WorkspaceDerivationOriginV1::Source { .. },
            WorkspaceDerivationRuleV1::UniverseLocking,
            WorkspaceDerivationSubjectV1::UniverseLock,
            WorkspaceDerivationOutputV1::UniverseLock { .. },
        ) => true,
        _ => false,
    };
    if compatible {
        Ok(())
    } else {
        Err(InvalidWorkspaceDerivationReceipt::FactKind)
    }
}

/// Maximum direct fan-in earned by each closed derivation rule.
const fn derivation_input_limit(rule: WorkspaceDerivationRuleV1) -> usize {
    match rule {
        WorkspaceDerivationRuleV1::SourceAdmission => 0,
        WorkspaceDerivationRuleV1::ComponentAdmission
        | WorkspaceDerivationRuleV1::DeclarationLinking
        | WorkspaceDerivationRuleV1::ResourceSelection
        | WorkspaceDerivationRuleV1::ResolutionLocking => 1,
        WorkspaceDerivationRuleV1::UniverseLocking => 2,
        WorkspaceDerivationRuleV1::ResponsibilityLowering => MAX_RESPONSIBILITY_DERIVATIONS + 1,
        WorkspaceDerivationRuleV1::OrganizationalTranslation => 2,
        WorkspaceDerivationRuleV1::ExperienceComposition => 2,
        WorkspaceDerivationRuleV1::ModuleSealing => MAX_WORKSPACE_COMPONENTS + 1,
        WorkspaceDerivationRuleV1::ImportLinking => MAX_WORKSPACE_DECLARATIONS + 2,
        WorkspaceDerivationRuleV1::ExtensionApplication => MAX_WORKSPACE_EXTENSION_TRAITS + 2,
        WorkspaceDerivationRuleV1::PolicyComposition => {
            MAX_WORKSPACE_DECLARATIONS + MAX_WORKSPACE_IMPORT_PATHS + MAX_WORKSPACE_EXTENSIONS
        }
        WorkspaceDerivationRuleV1::SemanticIrSealing => MAX_WORKSPACE_DERIVATION_INPUTS,
    }
}

fn validate_derivation_bounds(
    receipt: &WorkspaceDerivationReceiptV1,
) -> Result<(), InvalidWorkspaceDerivationReceipt> {
    for (kind, observed, limit) in [
        (
            "source",
            receipt.sources.len(),
            MAX_WORKSPACE_DERIVATION_SOURCES,
        ),
        ("node", receipt.nodes.len(), MAX_WORKSPACE_DERIVATION_NODES),
        ("root", receipt.roots.len(), MAX_WORKSPACE_DERIVATION_ROOTS),
        (
            "impact",
            receipt.impacts.len(),
            MAX_WORKSPACE_DERIVATION_IMPACTS,
        ),
    ] {
        bounded_derivation_len(kind, observed, limit)?;
    }
    Ok(())
}

fn bounded_derivation_len(
    kind: &'static str,
    observed: usize,
    limit: usize,
) -> Result<(), InvalidWorkspaceDerivationReceipt> {
    if observed > limit {
        Err(InvalidWorkspaceDerivationReceipt::Limit {
            kind,
            observed,
            limit,
        })
    } else {
        Ok(())
    }
}

fn validate_derivation_order(
    receipt: &WorkspaceDerivationReceiptV1,
) -> Result<(), InvalidWorkspaceDerivationReceipt> {
    if (!receipt.sources.is_empty()
        && !strictly_ordered(receipt.sources.iter().map(|source| &source.source_id)))
        || (!receipt.nodes.is_empty()
            && !strictly_ordered(receipt.nodes.iter().map(|node| &node.node_digest)))
        || (!receipt.roots.is_empty()
            && !strictly_ordered(receipt.roots.iter().map(|root| &root.root_digest)))
        || (!receipt.impacts.is_empty()
            && !strictly_ordered(receipt.impacts.iter().map(|impact| &impact.subject)))
    {
        return Err(InvalidWorkspaceDerivationReceipt::AggregateOrder);
    }
    Ok(())
}

struct WorkspaceCausalDagIndex {
    dependents: std::collections::BTreeMap<
        WorkspaceDerivationNodeDigest,
        Vec<WorkspaceDerivationNodeDigest>,
    >,
    topological_order: Vec<WorkspaceDerivationNodeDigest>,
}

fn validate_derivation_dag(
    nodes: &[WorkspaceDerivationNodeV1],
) -> Result<WorkspaceCausalDagIndex, InvalidWorkspaceDerivationReceipt> {
    let mut indegree = nodes
        .iter()
        .map(|node| (node.node_digest.clone(), node.inputs.len()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut dependents = nodes
        .iter()
        .map(|node| (node.node_digest.clone(), Vec::new()))
        .collect::<std::collections::BTreeMap<_, Vec<WorkspaceDerivationNodeDigest>>>();
    for node in nodes {
        for input in &node.inputs {
            dependents
                .get_mut(input)
                .expect("node references were checked before DAG traversal")
                .push(node.node_digest.clone());
        }
    }
    let mut ready = indegree
        .iter()
        .filter_map(|(node, degree)| (*degree == 0).then_some(node.clone()))
        .collect::<std::collections::BTreeSet<_>>();
    let mut topological_order = Vec::with_capacity(nodes.len());
    while let Some(node) = ready.iter().next().cloned() {
        ready.remove(&node);
        topological_order.push(node.clone());
        for dependent in &dependents[&node] {
            let degree = indegree
                .get_mut(dependent)
                .expect("dependent belongs to the admitted node set");
            *degree -= 1;
            if *degree == 0 {
                ready.insert(dependent.clone());
            }
        }
    }
    if topological_order.len() == nodes.len() {
        Ok(WorkspaceCausalDagIndex {
            dependents,
            topological_order,
        })
    } else {
        Err(InvalidWorkspaceDerivationReceipt::CausalCycle)
    }
}

fn validate_derivation_roots_and_impacts(
    receipt: &WorkspaceDerivationReceiptV1,
    nodes: &std::collections::BTreeMap<&WorkspaceDerivationNodeDigest, &WorkspaceDerivationNodeV1>,
    subjects: &std::collections::BTreeMap<
        &WorkspaceDerivationSubjectV1,
        &WorkspaceDerivationNodeV1,
    >,
    causal: &WorkspaceCausalDagIndex,
) -> Result<(), InvalidWorkspaceDerivationReceipt> {
    let mut root_subjects = std::collections::BTreeSet::new();
    let mut root_seeds = std::collections::BTreeMap::<
        WorkspaceDerivationNodeDigest,
        std::collections::BTreeSet<WorkspaceDerivationRootDigest>,
    >::new();
    let mut removed_root_ids = std::collections::BTreeSet::new();
    let root_ids = receipt
        .roots
        .iter()
        .map(|root| root.root_digest.clone())
        .collect::<std::collections::BTreeSet<_>>();
    for root in &receipt.roots {
        if !root_subjects.insert(&root.subject) {
            return Err(InvalidWorkspaceDerivationReceipt::DuplicateRootSubject);
        }
        if derivation_root_digest(&root.subject, &root.change)
            .map_err(InvalidWorkspaceDerivationReceipt::Canonical)?
            != root.root_digest
        {
            return Err(InvalidWorkspaceDerivationReceipt::RootDigestMismatch);
        }
        match &root.change {
            WorkspaceDerivationRootChangeV1::Added { current_node } => {
                let current = nodes.get(current_node).ok_or_else(|| {
                    InvalidWorkspaceDerivationReceipt::UnknownNode(current_node.to_string())
                })?;
                if current.subject != root.subject {
                    return Err(InvalidWorkspaceDerivationReceipt::RootSubjectMismatch);
                }
                root_seeds
                    .entry(current_node.clone())
                    .or_default()
                    .insert(root.root_digest.clone());
            }
            WorkspaceDerivationRootChangeV1::Modified {
                prior_output,
                current_node,
            } => {
                let current = nodes.get(current_node).ok_or_else(|| {
                    InvalidWorkspaceDerivationReceipt::UnknownNode(current_node.to_string())
                })?;
                if current.subject != root.subject
                    || &current.output == prior_output
                    || !derivation_subject_accepts_output(&root.subject, prior_output)
                {
                    return Err(InvalidWorkspaceDerivationReceipt::RootSubjectMismatch);
                }
                root_seeds
                    .entry(current_node.clone())
                    .or_default()
                    .insert(root.root_digest.clone());
            }
            WorkspaceDerivationRootChangeV1::Removed { prior_output } => {
                if subjects.contains_key(&root.subject)
                    || !derivation_subject_accepts_output(&root.subject, prior_output)
                {
                    return Err(InvalidWorkspaceDerivationReceipt::RootSubjectMismatch);
                }
                removed_root_ids.insert(root.root_digest.clone());
            }
        }
    }
    if receipt.impacts.len() != subjects.len()
        || receipt
            .impacts
            .iter()
            .map(|impact| &impact.subject)
            .ne(subjects.keys().copied())
    {
        return Err(InvalidWorkspaceDerivationReceipt::ImpactPartition);
    }
    let mut reachable_roots = receipt
        .nodes
        .iter()
        .map(|node| {
            (
                node.node_digest.clone(),
                root_seeds.remove(&node.node_digest).unwrap_or_default(),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    for node in &causal.topological_order {
        let roots = reachable_roots[node].clone();
        for dependent in &causal.dependents[node] {
            reachable_roots
                .get_mut(dependent)
                .expect("causal dependent belongs to the current node set")
                .extend(roots.iter().cloned());
        }
    }
    for impact in &receipt.impacts {
        let node = subjects[&impact.subject];
        let expected = &reachable_roots[&node.node_digest];
        match &impact.state {
            WorkspaceDerivationImpactStateV1::Affected { root_digests } => {
                if !strictly_ordered(root_digests.iter()) {
                    return Err(InvalidWorkspaceDerivationReceipt::ImpactPartition);
                }
                if root_digests.iter().any(|root| !root_ids.contains(root)) {
                    return Err(InvalidWorkspaceDerivationReceipt::UnknownRoot);
                }
                let claimed_current_roots = root_digests
                    .iter()
                    .filter(|root| !removed_root_ids.contains(*root));
                if claimed_current_roots.ne(expected.iter()) {
                    return Err(InvalidWorkspaceDerivationReceipt::ImpactPartition);
                }
            }
            WorkspaceDerivationImpactStateV1::Unaffected => {
                if !expected.is_empty() {
                    return Err(InvalidWorkspaceDerivationReceipt::ImpactPartition);
                }
            }
        }
    }
    Ok(())
}

fn derivation_subject_accepts_output(
    subject: &WorkspaceDerivationSubjectV1,
    output: &WorkspaceDerivationOutputV1,
) -> bool {
    matches!(
        (subject, output),
        (
            WorkspaceDerivationSubjectV1::Source { .. },
            WorkspaceDerivationOutputV1::SourceContent { .. }
        ) | (
            WorkspaceDerivationSubjectV1::Component { .. },
            WorkspaceDerivationOutputV1::Component { .. }
        ) | (
            WorkspaceDerivationSubjectV1::Module { .. },
            WorkspaceDerivationOutputV1::Module { .. }
        ) | (
            WorkspaceDerivationSubjectV1::Declaration { .. },
            WorkspaceDerivationOutputV1::Declaration { .. }
        ) | (
            WorkspaceDerivationSubjectV1::Import { .. },
            WorkspaceDerivationOutputV1::ImportSurface { .. }
        ) | (
            WorkspaceDerivationSubjectV1::Extension { .. },
            WorkspaceDerivationOutputV1::Extension { .. }
        ) | (
            WorkspaceDerivationSubjectV1::Policy { .. },
            WorkspaceDerivationOutputV1::EffectivePolicy { .. }
        ) | (
            WorkspaceDerivationSubjectV1::ResponsibilityClosure,
            WorkspaceDerivationOutputV1::ResponsibilityClosure { .. }
        ) | (
            WorkspaceDerivationSubjectV1::OrganizationalTranslationClosure,
            WorkspaceDerivationOutputV1::OrganizationalTranslationClosure { .. }
        ) | (
            WorkspaceDerivationSubjectV1::ExperienceComposition,
            WorkspaceDerivationOutputV1::ExperienceComposition { .. }
        ) | (
            WorkspaceDerivationSubjectV1::Resource { .. },
            WorkspaceDerivationOutputV1::Resource { .. }
        ) | (
            WorkspaceDerivationSubjectV1::ResolutionLock,
            WorkspaceDerivationOutputV1::ResolutionLock { .. }
        ) | (
            WorkspaceDerivationSubjectV1::UniverseLock,
            WorkspaceDerivationOutputV1::UniverseLock { .. }
        ) | (
            WorkspaceDerivationSubjectV1::Workspace { .. },
            WorkspaceDerivationOutputV1::SemanticIr { .. }
        )
    )
}

fn validate_derivation_coordinates(
    receipt: &WorkspaceDerivationReceiptV1,
) -> Result<(), InvalidWorkspaceDerivationReceipt> {
    let has_resolution = receipt.nodes.iter().any(|node| {
        matches!(
            &node.output,
            WorkspaceDerivationOutputV1::ResolutionLock { resolution_digest }
                if resolution_digest == &receipt.current_resolution_digest
        )
    });
    let has_universe = receipt.nodes.iter().any(|node| {
        matches!(
            &node.output,
            WorkspaceDerivationOutputV1::UniverseLock { universe_digest }
                if universe_digest == &receipt.current_universe_digest
        )
    });
    let has_semantic_ir = receipt.nodes.iter().any(|node| {
        matches!(
            &node.output,
            WorkspaceDerivationOutputV1::SemanticIr { workspace_semantic_ir_digest }
                if workspace_semantic_ir_digest == &receipt.workspace_semantic_ir_digest
        )
    });
    if has_resolution && has_universe && has_semantic_ir {
        Ok(())
    } else {
        Err(InvalidWorkspaceDerivationReceipt::CoordinateMismatch)
    }
}

fn looks_like_uri_reference(value: &str) -> bool {
    let Some((scheme, _)) = value.split_once(':') else {
        return false;
    };
    let mut chars = scheme.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
        && chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '-' | '.'))
}

pub(super) fn strictly_ordered<'a, T: Ord + 'a>(mut values: impl Iterator<Item = &'a T>) -> bool {
    let Some(mut prior) = values.next() else {
        return true;
    };
    for value in values {
        if prior >= value {
            return false;
        }
        prior = value;
    }
    true
}

pub(super) fn adjacent_duplicate<'a, T: Ord + 'a>(
    mut values: impl Iterator<Item = &'a T>,
) -> Option<&'a T> {
    let mut prior = values.next()?;
    for value in values {
        if prior == value {
            return Some(value);
        }
        prior = value;
    }
    None
}

/// A derivation receipt fails its own canonical self-integrity contract.
#[derive(Debug, thiserror::Error)]
pub enum InvalidWorkspaceDerivationReceipt {
    /// The schema identity or initial-v1 version differs.
    #[error("workspace derivation receipt has an invalid schema coordinate")]
    SchemaCoordinate,
    /// A source, node, root, or impact set is duplicated or noncanonical.
    #[error("workspace derivation aggregates are not strictly ordered and unique")]
    AggregateOrder,
    /// One node input set is duplicated or noncanonical.
    #[error("workspace derivation node inputs are not strictly ordered and unique")]
    NodeInputOrder,
    /// A receipt aggregate exceeds its initial-v1 ceiling.
    #[error("workspace derivation {kind} count {observed} exceeds {limit}")]
    Limit {
        /// Closed aggregate whose bound was exceeded.
        kind: &'static str,
        /// Observed element count.
        observed: usize,
        /// Maximum admitted element count.
        limit: usize,
    },
    /// A source label resembles a URI rather than an owner-relative path.
    #[error("workspace derivation source `{0}` uses a URL or URI-like path")]
    SourcePath(String),
    /// A source-origin node names no retained source record.
    #[error("workspace derivation node refers to unknown source `{0}`")]
    UnknownSource(String),
    /// Retained sources and source-admission facts are not a bijection.
    #[error("workspace derivation sources do not have exactly one matching admission fact")]
    SourceAdmissionBijection,
    /// A causal input or change root names no current node.
    #[error("workspace derivation refers to unknown node `{0}`")]
    UnknownNode(String),
    /// A node's rule, subject, and output do not form one legal typed fact.
    #[error("workspace derivation rule, subject, and output are incompatible")]
    FactKind,
    /// A rule consumes the wrong number or class of causal inputs.
    #[error("workspace derivation rule consumes incompatible causal inputs")]
    InputGrammar,
    /// A node's claimed digest differs from its canonical fact preimage.
    #[error("workspace derivation node digest does not match its canonical preimage")]
    NodeDigestMismatch,
    /// More than one current node claims the same semantic subject.
    #[error("workspace derivation repeats a current semantic subject")]
    DuplicateSubject,
    /// Causal input edges contain a cycle.
    #[error("workspace derivation node inputs are cyclic")]
    CausalCycle,
    /// More than one root claims the same changed semantic subject.
    #[error("workspace derivation repeats a root subject")]
    DuplicateRootSubject,
    /// A root's claimed digest differs from its canonical change preimage.
    #[error("workspace derivation root digest does not match its canonical preimage")]
    RootDigestMismatch,
    /// A current root node does not produce the root's subject or a removed subject still exists.
    #[error("workspace derivation root does not match its subject transition")]
    RootSubjectMismatch,
    /// An impact cites no known change root.
    #[error("workspace derivation impact refers to an unknown root")]
    UnknownRoot,
    /// Impacts do not classify every current subject exactly once with valid roots.
    #[error("workspace derivation impacts do not form a complete root-backed partition")]
    ImpactPartition,
    /// Current lock or semantic-IR nodes disagree with the receipt header.
    #[error("workspace derivation causal outputs disagree with receipt coordinates")]
    CoordinateMismatch,
    /// Canonical encoding failed.
    #[error("workspace derivation receipt canonicalization failed: {0}")]
    Canonical(#[source] crate::canon::CanonError),
    /// Claimed and recomputed receipt identities differ.
    #[error("workspace derivation receipt digest does not match its canonical preimage")]
    DigestMismatch,
}

#[cfg(test)]
#[path = "../../../tests/internal/records/workspace_language/derivation.rs"]
mod derivation_grammar_tests;
