//! Admission and eager custody of one immutable resident compilation.
//!
//! A resident generation is the deterministic join where `C` is one verified
//! live [`Compilation`] and `A` contains exactly one
//! owner-admitted applied materialization per compiled repository, and `S`
//! contains the corresponding exact source frontiers. Admission consumes `C`
//! into one generation-fenced packed dependency graph and aligned immutable
//! body arena. The graph owns live adjacency; the arena owns complete bodies.

mod diagnostic;
mod evidence;
mod impact;

use pale_core::records::{
    AdmittedLifecycleBehaviorV1, CompiledGenerationDigest, CompiledGenerationPreimageV1,
    CompiledGenerationV1, CompositionGraphViewV1, CompositionReasonPathV1, ExperiencePathV1,
    LifecycleBehaviorIdV1, LogicalRepositoryIdV1, MaterializationLineageV1, MaterializationStateV1,
    ModeAxis, PackageResourceRefV1, ProviderBindingV1, RepositorySourceFrontierV1,
    WorkspaceDerivationNodeV1, WorkspaceDerivationReceiptV1, WorkspaceDerivationSourceV1,
    COMPILED_GENERATION_V1_SCHEMA_VERSION,
};
use pale_kernel::AdaptiveNodeSetPolicy;

use crate::compile::{Compilation, CompiledRepo};
use crate::plan::AdmittedSuccessReceipt;
use crate::semantic::{ResolvedOperation, ResolvedRunMode};

use super::definition::ResidentDefinitionStore;
pub(super) use diagnostic::{resident_definition_coordinate, resident_evidence_coordinate};
pub use diagnostic::{
    CompiledGenerationAdmissionError, ResidentCompositionDefinitionKind,
    ResidentDefinitionCoordinate, ResidentDefinitionFailureCoordinate, ResidentDefinitionInvariant,
    ResidentImpactQueryError, ResidentRelationshipEvidenceCoordinate,
    ResidentWorkspaceCausalCoordinate,
};
use evidence::{
    materialization_closure, validate_applied_materialization,
    validate_owner_materialization_records, validate_repository_coverage,
};
pub use impact::{
    GenerationInputChange, GenerationSuccessorImpact, PackageResourceResidentCoordinate,
    RemovedPackageResourceBridge, RemovedRepositoryBridge, RemovedWorkspaceRootBridge,
    RepositoryResidentCoordinate, ResidentImpactReasonPath, ResidentImpactReasonStep,
};

/// Domain tag for the canonical set of verified applied-materialization proofs.
const MATERIALIZATION_CLOSURE_SCHEMA: &str = "pale.compiled-generation-materializations.v1";

/// One exact coordinate class checked while joining applied evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppliedMaterializationCoordinate {
    /// Receipt fields inherited from the one-shot compilation.
    ReceiptCompilation,
    /// Receipt Package, Pack, and semantic identities.
    ReceiptSemantics,
    /// The successful receipt's post-state and post-lineage transition.
    ReceiptTransition,
    /// The persisted ownership state's own schema and digest.
    StateIdentity,
    /// State fields inherited from the one-shot compilation.
    StateCompilation,
    /// Exact intended non-deferred filesystem surface set.
    StateSurfaceClosure,
    /// The persisted lineage record's own schema and digest.
    LineageIdentity,
    /// Lineage fields inherited from the one-shot compilation.
    LineageCompilation,
    /// Lineage binding to the admitted receipt, state, and successful transition.
    LineageReceiptBinding,
    /// Exact repository identity carried by the source frontier.
    SourceFrontier,
    /// Canonical closure of all verified repository materializations.
    MaterializationClosure,
}

/// Exact owner-admitted evidence for one applied repository projection.
pub struct AppliedRepositoryMaterialization {
    receipt: AdmittedSuccessReceipt,
    state: MaterializationStateV1,
    lineage: MaterializationLineageV1,
    source_frontier: RepositorySourceFrontierV1,
}

impl AppliedRepositoryMaterialization {
    /// Admit one owner-observed applied repository record set.
    ///
    /// The machine-local owner remains responsible for observing the canonical
    /// target root and exact source frontier. This boundary verifies the complete
    /// record-local join: schema and self-digests, successful receipt transition,
    /// repository identity, compilation coordinates, lineage binding, and source
    /// frontier identity. [`AdmittedCompiledGeneration`] later compares the same
    /// proof with the selected one-shot [`Compilation`] and desired surface set.
    ///
    /// # Errors
    ///
    /// Returns [`CompiledGenerationAdmissionError`] when any supplied owner record
    /// is invalid, names a removal transition, or disagrees with another member.
    pub fn admit(
        receipt: AdmittedSuccessReceipt,
        state: MaterializationStateV1,
        lineage: MaterializationLineageV1,
        source_frontier: RepositorySourceFrontierV1,
    ) -> Result<Self, CompiledGenerationAdmissionError> {
        let admitted = Self {
            receipt,
            state,
            lineage,
            source_frontier,
        };
        validate_owner_materialization_records(&admitted)?;
        Ok(admitted)
    }

    /// Borrow the owner-admitted successful transaction receipt.
    pub fn receipt(&self) -> &AdmittedSuccessReceipt {
        &self.receipt
    }

    /// Borrow the exact applied ownership state.
    #[must_use]
    pub fn state(&self) -> &MaterializationStateV1 {
        &self.state
    }

    /// Borrow the exact applied ownership lineage.
    #[must_use]
    pub fn lineage(&self) -> &MaterializationLineageV1 {
        &self.lineage
    }

    /// Borrow the independently observed source frontier.
    #[must_use]
    pub fn source_frontier(&self) -> &RepositorySourceFrontierV1 {
        &self.source_frontier
    }
}

/// One immutable core generation plus its eager definition and proof bodies.
///
/// The core [`CompiledGenerationV1`] is the canonical public identity. This
/// aggregate retains the bodies needed to serve resident lookups without
/// reopening Package source, a target filesystem, or a mutable owner store.
/// The one-shot [`Compilation`] wrapper is consumed and dropped after the sole
/// packed graph and aligned body arena have been admitted.
pub struct AdmittedCompiledGeneration {
    record: CompiledGenerationV1,
    store: ResidentDefinitionStore,
    hook_policies: super::hook::prepared::PreparedHookPolicies,
    derivation_receipt: Option<WorkspaceDerivationReceiptV1>,
    successor_impact: Option<GenerationSuccessorImpact>,
    activation_compatibility: GenerationActivationCompatibility,
}

/// Opaque activation-compatibility evidence for one admitted generation.
///
/// A genesis proof is valid only while no generation is active. A successor
/// proof binds the exact prior generation, current generation, and the admitted
/// workspace derivation receipt. Recovered content carries a closed non-proof
/// state and cannot authorize ordinary activation. No constructor accepts
/// caller-supplied digests.
pub struct GenerationActivationCompatibility {
    current_generation: CompiledGenerationDigest,
    lineage: GenerationActivationLineage,
}

/// Closed provenance states behind one activation-compatibility decision.
enum GenerationActivationLineage {
    /// Ordinary admission proved that no predecessor was active.
    Genesis,
    /// Successor admission reproduced an exact prior/current workspace derivation.
    Successor {
        /// Exact active predecessor required by the proof.
        prior_generation: CompiledGenerationDigest,
    },
    /// Recovery rebuilt immutable content without claiming an absent predecessor proof.
    RecoveredContentOnly,
}

impl GenerationActivationCompatibility {
    /// Match this proof to the exact active predecessor and advertised candidate.
    pub(crate) fn admits(
        &self,
        prior_generation: Option<&CompiledGenerationDigest>,
        current_generation: &CompiledGenerationDigest,
    ) -> bool {
        if &self.current_generation != current_generation {
            return false;
        }
        match &self.lineage {
            GenerationActivationLineage::Genesis => prior_generation.is_none(),
            GenerationActivationLineage::Successor {
                prior_generation: expected,
            } => prior_generation == Some(expected),
            GenerationActivationLineage::RecoveredContentOnly => false,
        }
    }
}

impl AdmittedCompiledGeneration {
    /// Consume one live compilation and its complete applied-materialization proof set.
    ///
    /// Evidence order must equal [`Compilation::repositories`]. The canonical
    /// materialization hash and public source-frontier set are independently
    /// sorted by repository identity, so profile materialization order is not
    /// confused with set canonicalization.
    ///
    /// # Errors
    ///
    /// Returns [`CompiledGenerationAdmissionError`] when the compilation is not
    /// live and self-consistent, repository coverage differs, any receipt/state/
    /// lineage coordinate disagrees, the applied state differs from the exact
    /// compiled non-deferred filesystem surfaces, the resident graph/body join
    /// is incomplete or ambiguous, or the core generation contract is rejected.
    pub(crate) fn admit(
        compilation: Compilation,
        materializations: Vec<AppliedRepositoryMaterialization>,
        set_policy: AdaptiveNodeSetPolicy,
    ) -> Result<Self, CompiledGenerationAdmissionError> {
        Self::admit_inner(
            compilation,
            materializations,
            set_policy,
            GenerationAdmissionContext::Genesis,
        )
    }

    /// Rebuild exact immutable content without fabricating activation lineage.
    ///
    /// Recovery may retain a generation after its derivation predecessor is
    /// lawfully reclaimed. This path performs the same compilation, public
    /// record, graph, arena, and index admission as [`Self::admit`], but its
    /// [`GenerationActivationCompatibility`] always rejects ordinary activation.
    /// A later activation must derive a fresh successor proof or use the
    /// lifecycle's separately guarded rollback transition.
    ///
    /// # Errors
    ///
    /// Returns [`CompiledGenerationAdmissionError`] under the same exact
    /// content and resident-definition laws as [`Self::admit`].
    pub(crate) fn admit_recovered(
        compilation: Compilation,
        materializations: Vec<AppliedRepositoryMaterialization>,
        set_policy: AdaptiveNodeSetPolicy,
    ) -> Result<Self, CompiledGenerationAdmissionError> {
        Self::admit_inner(
            compilation,
            materializations,
            set_policy,
            GenerationAdmissionContext::RecoveredContent,
        )
    }

    /// Consume a successor compilation under an exact prior/current derivation proof.
    ///
    /// The receipt is derived inside this boundary from both admitted workspace
    /// universes. The successor always builds a fresh graph and arena, then
    /// derives its affected/unaffected partition through that graph's reverse
    /// dependency closure. Callers cannot inject a merely self-consistent
    /// transition receipt or preserve predecessor handles.
    ///
    /// # Errors
    ///
    /// Returns [`CompiledGenerationAdmissionError`] for any ordinary generation
    /// failure or when the predecessor/current derivation cannot be reproduced.
    pub(crate) fn admit_successor(
        prior: &Self,
        compilation: Compilation,
        materializations: Vec<AppliedRepositoryMaterialization>,
        set_policy: AdaptiveNodeSetPolicy,
    ) -> Result<Self, CompiledGenerationAdmissionError> {
        let receipt = compilation.workspace_derivation_receipt(prior.store.derivation())?;
        Self::admit_inner(
            compilation,
            materializations,
            set_policy,
            GenerationAdmissionContext::Successor(Box::new(SuccessorAdmission {
                prior_generation: prior.record.compiled_generation_digest().clone(),
                prior_store: &prior.store,
                receipt,
            })),
        )
    }

    fn admit_inner(
        compilation: Compilation,
        materializations: Vec<AppliedRepositoryMaterialization>,
        set_policy: AdaptiveNodeSetPolicy,
        context: GenerationAdmissionContext<'_>,
    ) -> Result<Self, CompiledGenerationAdmissionError> {
        compilation.verify_admitted_compilation()?;
        let executable_identity_digest = compilation
            .executable_identity_digest()
            .cloned()
            .ok_or(CompiledGenerationAdmissionError::MissingExecutableIdentity)?;
        validate_repository_coverage(&compilation, &materializations)?;

        for (repository, materialization) in
            compilation.repositories().iter().zip(&materializations)
        {
            validate_applied_materialization(&compilation, repository, materialization)?;
        }

        let hook_policies = super::hook::prepared::PreparedHookPolicies::prepare(&compilation)?;
        let materialization_closure_digest = materialization_closure(&materializations)?;
        let mut repository_source_frontiers = materializations
            .iter()
            .map(|materialization| materialization.source_frontier.clone())
            .collect::<Vec<_>>();
        repository_source_frontiers
            .sort_by(|left, right| left.repository_id.cmp(&right.repository_id));

        let schema_identity = pale_contracts::record_schema_identity("compiled-generation.v1")?;
        let schema_registry = compilation.schema_registry_identity();
        let preimage = CompiledGenerationPreimageV1 {
            schema: schema_identity.schema,
            schema_sha256: schema_identity.schema_sha256,
            schema_version: COMPILED_GENERATION_V1_SCHEMA_VERSION,
            distribution_lock_digest: compilation.distribution_lock_digest().clone(),
            profile_digest: compilation.profile().profile_digest.clone(),
            instance_digest: compilation.instance().instance_digest.clone(),
            package_semantic_ir_digest: compilation.package_semantic_ir().digest().clone(),
            profile_package_release_closure: compilation.profile().package_closure.clone(),
            profile_normative_pack_release_closure: compilation
                .profile()
                .normative_pack_closure
                .clone(),
            schema_manifest_digest: schema_registry.schema_manifest_sha256.clone(),
            schema_set_digest: schema_registry.schema_set_sha256.clone(),
            workspace_semantic_ir_digest: compilation.workspace_semantic_ir_digest().clone(),
            workspace_admission_evidence_digest: compilation
                .workspace_admission_evidence_digest()
                .clone(),
            principal_admitted_universe_digest: compilation
                .principal_admitted_universe_digest()
                .clone(),
            workspace_module_release_closure: compilation
                .workspace_module_release_closure()
                .to_vec(),
            workspace_package_release_closure: compilation
                .workspace_package_release_closure()
                .to_vec(),
            workspace_normative_pack_release_closure: compilation
                .workspace_normative_pack_release_closure()
                .to_vec(),
            workspace_resolution_lock_digest: compilation
                .workspace_resolution_lock_digest()
                .clone(),
            workspace_universe_lock_digest: compilation.workspace_universe_lock_digest().clone(),
            responsibility_derivation_closure_digest: compilation
                .responsibility_derivation_closure_digest()
                .clone(),
            organizational_translation_closure_digest: compilation
                .organizational_translation_closure_digest()
                .clone(),
            experience_composition_source_digest: compilation
                .experience_composition_source_digest()
                .clone(),
            provider_binding_closure_digest: compilation.provider_binding_closure_digest().clone(),
            experience_composition_plan_digest: compilation
                .experience_composition_plan()
                .plan_digest
                .clone(),
            execution_paths: compilation.profile().execution_paths.clone(),
            materialization_closure_digest,
            machine_overlay_digest: compilation.machine_overlay_digest().clone(),
            executable_identity_digest,
            compilation_input_digest: compilation.compilation_input_digest().clone(),
            one_shot_compilation_digest: compilation.one_shot_compilation_digest().clone(),
            repository_source_frontiers,
        };
        let record = CompiledGenerationV1::admit_owner_preimage(preimage)?;
        let store = ResidentDefinitionStore::build(
            record.compiled_generation_digest().clone(),
            compilation.into_resident_parts(),
            materializations,
            set_policy,
        )
        .map_err(|source| CompiledGenerationAdmissionError::from_definition_store(&source))?;
        if store.owner() != record.compiled_generation_digest() {
            return Err(CompiledGenerationAdmissionError::ResidentDefinition {
                invariant: ResidentDefinitionInvariant::OwnerDigest,
                coordinate: None,
            });
        }
        let successor_impact = match &context {
            GenerationAdmissionContext::Successor(successor) => store
                .successor_impact(successor.prior_store, &successor.receipt)
                .map(Some)
                .map_err(|source| {
                    CompiledGenerationAdmissionError::from_definition_store(&source)
                })?,
            GenerationAdmissionContext::Genesis | GenerationAdmissionContext::RecoveredContent => {
                None
            }
        };

        let (derivation_receipt, activation_compatibility) = match context {
            GenerationAdmissionContext::Successor(successor) => {
                let compatibility = GenerationActivationCompatibility {
                    current_generation: record.compiled_generation_digest().clone(),
                    lineage: GenerationActivationLineage::Successor {
                        prior_generation: successor.prior_generation,
                    },
                };
                (Some(successor.receipt), compatibility)
            }
            GenerationAdmissionContext::Genesis => (
                None,
                GenerationActivationCompatibility {
                    current_generation: record.compiled_generation_digest().clone(),
                    lineage: GenerationActivationLineage::Genesis,
                },
            ),
            GenerationAdmissionContext::RecoveredContent => (
                None,
                GenerationActivationCompatibility {
                    current_generation: record.compiled_generation_digest().clone(),
                    lineage: GenerationActivationLineage::RecoveredContentOnly,
                },
            ),
        };

        Ok(Self {
            record,
            store,
            hook_policies,
            derivation_receipt,
            successor_impact,
            activation_compatibility,
        })
    }

    /// Borrow the canonical public compiled-generation record.
    #[must_use]
    pub fn record(&self) -> &CompiledGenerationV1 {
        &self.record
    }

    /// Borrow the exact workspace derivation receipt retained by a successor.
    ///
    /// Genesis and recovery-only content admission return [`None`]. Recovery
    /// does not claim that a lawfully reclaimed predecessor remains available.
    #[must_use]
    pub fn derivation_receipt(&self) -> Option<&WorkspaceDerivationReceiptV1> {
        self.derivation_receipt.as_ref()
    }

    /// Borrow the opaque compatibility evidence consumed by generation activation.
    #[must_use]
    pub fn activation_compatibility(&self) -> &GenerationActivationCompatibility {
        &self.activation_compatibility
    }

    /// Borrow the exact typed inputs that seeded successor invalidation.
    ///
    /// Genesis and recovery-only content admission return [`None`].
    #[must_use]
    pub fn successor_input_changes(&self) -> Option<&[GenerationInputChange]> {
        self.successor_impact
            .as_ref()
            .map(|impact| impact.input_changes.as_slice())
    }

    /// Borrow the complete semantic invalidation proof for this successor.
    #[must_use]
    pub fn successor_impact(&self) -> Option<&GenerationSuccessorImpact> {
        self.successor_impact.as_ref()
    }

    /// Reconstruct one canonical successor-impact reason on demand.
    ///
    /// Returns [`None`] for genesis or recovery-only content admission, unknown
    /// coordinates, or definitions outside the affected closure. The compact
    /// resident proof is retained once; this call allocates only the requested
    /// path rather than eagerly expanding every affected witness.
    ///
    /// # Errors
    ///
    /// Returns an error if the admitted graph/proof generation fence or one
    /// semantic coordinate no longer agrees with the resident store.
    pub fn successor_reason_path(
        &self,
        target: &ResidentDefinitionCoordinate,
    ) -> Result<Option<ResidentImpactReasonPath>, ResidentImpactQueryError> {
        let Some(impact) = &self.successor_impact else {
            return Ok(None);
        };
        self.store
            .impact_reason_path(&impact.closure, target)
            .map_err(|source| ResidentImpactQueryError::from_definition_store(&source))
    }

    /// Borrow exact source coordinates retained by the admitted workspace proof.
    #[must_use]
    pub fn workspace_derivation_sources(&self) -> &[WorkspaceDerivationSourceV1] {
        self.store.workspace_derivation_sources()
    }

    /// Borrow canonical causal nodes retained as reviewable derivation proof.
    #[must_use]
    pub fn workspace_derivation_nodes(&self) -> &[WorkspaceDerivationNodeV1] {
        self.store.workspace_derivation_nodes()
    }

    /// Borrow canonical reason paths retained by the admitted composition plan.
    ///
    /// [`Some`] with an empty slice means that the retained plan selected no
    /// paths. [`None`] means the corresponding plan is absent or its retained
    /// digest does not match; this accessor never reconstructs reasons from
    /// resident adjacency or raw declarations.
    #[must_use]
    pub fn composition_reason_paths(&self) -> Option<&[CompositionReasonPathV1]> {
        self.store
            .composition_reason_paths(self.record.experience_composition_plan_digest())
    }

    /// Borrow non-authoritative composition graph views retained for review.
    #[must_use]
    pub fn composition_graph_views(&self) -> Option<&[CompositionGraphViewV1]> {
        self.store
            .composition_graph_views(self.record.experience_composition_plan_digest())
    }

    /// Look up one retained repository projection by its stable identity.
    #[must_use]
    pub fn repository(&self, repository_id: &LogicalRepositoryIdV1) -> Option<&CompiledRepo> {
        self.store.repository(repository_id)
    }

    pub(super) fn hook_path(
        &self,
        repository: &LogicalRepositoryIdV1,
        selection: &pale_core::records::ExecutionPathSelectionIdV1,
    ) -> Option<&super::hook::prepared::PreparedHookPath> {
        self.hook_policies.path(repository, selection)
    }

    /// Look up the applied proof retained for one compiled repository.
    #[must_use]
    pub fn materialization(
        &self,
        repository_id: &LogicalRepositoryIdV1,
    ) -> Option<&AppliedRepositoryMaterialization> {
        self.store.materialization(repository_id)
    }

    /// Look up one retained provider binding by its exact stable record ID.
    ///
    /// The returned reference borrows the body already owned by this
    /// generation's resident definition arena. It neither clones provider
    /// coordinates nor establishes current provider availability.
    #[must_use]
    pub fn provider_binding(&self, binding_id: &str) -> Option<&ProviderBindingV1> {
        self.store.provider_binding(binding_id)
    }

    /// Look up one retained experience path by its exact stable record ID.
    ///
    /// The returned reference borrows the admitted path body; it does not
    /// reconstruct the path from adjacency or imply that its provider is live.
    #[must_use]
    pub fn experience_path(&self, path_id: &str) -> Option<&ExperiencePathV1> {
        self.store.experience_path(path_id)
    }

    /// Look up one owner-qualified semantic operation in one repository.
    #[must_use]
    pub fn operation(
        &self,
        repository_id: &LogicalRepositoryIdV1,
        operation_id: &PackageResourceRefV1,
    ) -> Option<&ResolvedOperation> {
        self.store.operation(repository_id, operation_id)
    }

    /// Look up one private lifecycle behavior by its stable profile identity.
    #[must_use]
    pub fn lifecycle_behavior(
        &self,
        behavior_id: &LifecycleBehaviorIdV1,
    ) -> Option<&AdmittedLifecycleBehaviorV1> {
        self.store.lifecycle_behavior(behavior_id)
    }

    /// Look up one owner-qualified run mode in one repository.
    #[must_use]
    pub fn run_mode(
        &self,
        repository_id: &LogicalRepositoryIdV1,
        mode_id: &PackageResourceRefV1,
    ) -> Option<&ResolvedRunMode> {
        self.store.run_mode(repository_id, mode_id)
    }

    /// Borrow the effective default for one orthogonal mode axis in one repository.
    #[must_use]
    pub fn mode_default(
        &self,
        repository_id: &LogicalRepositoryIdV1,
        axis: ModeAxis,
    ) -> Option<&PackageResourceRefV1> {
        self.store.mode_default(repository_id, axis)
    }
}

struct SuccessorAdmission<'a> {
    prior_generation: CompiledGenerationDigest,
    prior_store: &'a ResidentDefinitionStore,
    receipt: WorkspaceDerivationReceiptV1,
}

/// Admission context selecting only proofs reproduced by the owning builder.
enum GenerationAdmissionContext<'a> {
    /// Ordinary first-generation admission.
    Genesis,
    /// Ordinary successor admission with an exact prior store.
    Successor(Box<SuccessorAdmission<'a>>),
    /// Recovery-only content admission with no activation-lineage claim.
    RecoveredContent,
}

#[cfg(test)]
#[path = "../../tests/internal/resident/generation.rs"]
pub(super) mod tests;
