//! Close the current principal trust frontier over implemented transitions.
//!
//! Historical checkpoint signatures, producer custody, and deployment
//! finalization custody arrive as separate admitted evidence. This module joins
//! those proofs, reproduces every retained finalization record, and accepts only
//! producer-head changes and initial deployment-signer insertions. It does not
//! select a current principal key, compare an independent anchor, recover a
//! writer, or make a resident ready.

use std::collections::BTreeSet;

use pale_core::records::{
    checkpoint_signing_authority_heads_root_sha256, deployment_runtime_authority_heads_root_sha256,
    producer_incarnation_heads_root_sha256, AdmittedPrincipalBootstrapV1,
    CheckpointSigningAuthorityHeadV1, CheckpointSigningAuthorityV1,
    DeploymentAcceptanceRevocationHeadsRootDigest, DeploymentBindingV1,
    DeploymentRuntimeAuthorityHeadV1, DeploymentRuntimeAuthorityV1,
    FederationSpaceHomeHeadsRootDigest, InvalidResidentRecordV1, NodeDelegationHeadsRootDigest,
    NodeDelegationRevocationHeadsRootDigest, OwnerRecordSigningAuthorityHeadsRootDigest,
    OwnerSignerPublicationFrontierHeadsRootDigest, PrincipalTrustCheckpointV1, ResidentRecordRefV1,
};
use pale_ports::{
    ControlRecord, ControlRecordCoordinate, DeploymentFinalizationSetSnapshot,
    DeploymentFinalizationSnapshot,
};

use super::deployment_finalization::{
    AdmittedDeploymentFinalization, DeploymentFinalizationCandidate, DeploymentFinalizationError,
};
use super::principal_trust_recovery::{
    RecoveredSignedPrincipalTrustCheckpoint, RecoveredSignedPrincipalTrustCheckpointChain,
};
use super::record_projection;
use super::trust_transition::{AdmittedPrincipalTrustCheckpointTransition, TrustHeadChange};
use super::RecoveredPrincipalProducerIncarnationFrontier;

/// Typed records admitted from one retained deployment-finalization closure.
///
/// The admitting owner resolves schemas, signatures, and record-specific
/// context. Frontier closure independently compares the result with the exact
/// stored bytes before it can affect a current head set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::resident) struct AdmittedHistoricalDeploymentFinalizationRecords {
    binding_ref: ResidentRecordRefV1,
    binding: DeploymentBindingV1,
    checkpoint_signer_ref: ResidentRecordRefV1,
    checkpoint_signer: CheckpointSigningAuthorityV1,
    runtime_authority_ref: ResidentRecordRefV1,
    runtime_authority: DeploymentRuntimeAuthorityV1,
}

impl AdmittedHistoricalDeploymentFinalizationRecords {
    /// Retain the three independently admitted semantic records.
    #[must_use]
    pub(in crate::resident) const fn new(
        binding_ref: ResidentRecordRefV1,
        binding: DeploymentBindingV1,
        checkpoint_signer_ref: ResidentRecordRefV1,
        checkpoint_signer: CheckpointSigningAuthorityV1,
        runtime_authority_ref: ResidentRecordRefV1,
        runtime_authority: DeploymentRuntimeAuthorityV1,
    ) -> Self {
        Self {
            binding_ref,
            binding,
            checkpoint_signer_ref,
            checkpoint_signer,
            runtime_authority_ref,
            runtime_authority,
        }
    }

    /// Borrow the admitted deployment binding and its immutable reference.
    #[must_use]
    pub(in crate::resident) const fn binding(
        &self,
    ) -> (&ResidentRecordRefV1, &DeploymentBindingV1) {
        (&self.binding_ref, &self.binding)
    }

    /// Borrow the admitted checkpoint signer and its immutable reference.
    #[must_use]
    pub(in crate::resident) const fn checkpoint_signer(
        &self,
    ) -> (&ResidentRecordRefV1, &CheckpointSigningAuthorityV1) {
        (&self.checkpoint_signer_ref, &self.checkpoint_signer)
    }

    /// Borrow the admitted runtime authority and its immutable reference.
    #[must_use]
    pub(in crate::resident) const fn runtime_authority(
        &self,
    ) -> (&ResidentRecordRefV1, &DeploymentRuntimeAuthorityV1) {
        (&self.runtime_authority_ref, &self.runtime_authority)
    }
}

/// Semantic finalization inputs retained at their exact trust transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::resident) struct RecoveredDeploymentFinalizationContext {
    records: AdmittedHistoricalDeploymentFinalizationRecords,
    predecessor_checkpoint_index: usize,
    successor_checkpoint_index: usize,
    predecessor_checkpoint_heads: Box<[CheckpointSigningAuthorityHeadV1]>,
    predecessor_runtime_heads: Box<[DeploymentRuntimeAuthorityHeadV1]>,
}

impl RecoveredDeploymentFinalizationContext {
    /// Borrow the three semantic records reproduced from physical finalization evidence.
    #[must_use]
    pub(in crate::resident) const fn records(
        &self,
    ) -> &AdmittedHistoricalDeploymentFinalizationRecords {
        &self.records
    }

    /// Return the predecessor trust checkpoint's index in the admitted chain.
    #[must_use]
    pub(in crate::resident) const fn predecessor_checkpoint_index(&self) -> usize {
        self.predecessor_checkpoint_index
    }

    /// Return the successor trust checkpoint's index in the admitted chain.
    #[must_use]
    pub(in crate::resident) const fn successor_checkpoint_index(&self) -> usize {
        self.successor_checkpoint_index
    }

    /// Borrow the checkpoint-signer heads that existed before finalization.
    #[must_use]
    pub(in crate::resident) const fn predecessor_checkpoint_heads(
        &self,
    ) -> &[CheckpointSigningAuthorityHeadV1] {
        &self.predecessor_checkpoint_heads
    }

    /// Borrow the runtime-authority heads that existed before finalization.
    #[must_use]
    pub(in crate::resident) const fn predecessor_runtime_heads(
        &self,
    ) -> &[DeploymentRuntimeAuthorityHeadV1] {
        &self.predecessor_runtime_heads
    }
}

/// Record-specific admission required by historical frontier closure.
pub(in crate::resident) trait HistoricalDeploymentFinalizationAdmission {
    /// Admit the exact binding and two authority records retained by one close.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when any stored record fails its own
    /// schema, signature, or owner-supplied contextual admission.
    fn admit_historical_deployment_finalization(
        &mut self,
        binding: &ControlRecord,
        checkpoint_signer: &ControlRecord,
        runtime_authority: &ControlRecord,
    ) -> Result<AdmittedHistoricalDeploymentFinalizationRecords, InvalidResidentRecordV1>;
}

impl<F> HistoricalDeploymentFinalizationAdmission for F
where
    F: FnMut(
        &ControlRecord,
        &ControlRecord,
        &ControlRecord,
    )
        -> Result<AdmittedHistoricalDeploymentFinalizationRecords, InvalidResidentRecordV1>,
{
    fn admit_historical_deployment_finalization(
        &mut self,
        binding: &ControlRecord,
        checkpoint_signer: &ControlRecord,
        runtime_authority: &ControlRecord,
    ) -> Result<AdmittedHistoricalDeploymentFinalizationRecords, InvalidResidentRecordV1> {
        self(binding, checkpoint_signer, runtime_authority)
    }
}

/// One checkpoint-signing authority whose exact source bytes were admitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::resident) struct RecoveredCheckpointSigningAuthority {
    record_ref: ResidentRecordRefV1,
    authority: CheckpointSigningAuthorityV1,
    head: CheckpointSigningAuthorityHeadV1,
}

impl RecoveredCheckpointSigningAuthority {
    /// Borrow the exact admitted authority reference.
    #[must_use]
    pub(in crate::resident) const fn record_ref(&self) -> &ResidentRecordRefV1 {
        &self.record_ref
    }

    /// Borrow the admitted authority record.
    #[must_use]
    pub(in crate::resident) const fn authority(&self) -> &CheckpointSigningAuthorityV1 {
        &self.authority
    }

    /// Borrow the canonical active-head projection.
    #[must_use]
    pub(in crate::resident) const fn head(&self) -> &CheckpointSigningAuthorityHeadV1 {
        &self.head
    }
}

/// One deployment runtime authority whose exact source bytes were admitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::resident) struct RecoveredDeploymentRuntimeAuthority {
    binding_ref: ResidentRecordRefV1,
    binding: DeploymentBindingV1,
    record_ref: ResidentRecordRefV1,
    authority: DeploymentRuntimeAuthorityV1,
    head: DeploymentRuntimeAuthorityHeadV1,
}

impl RecoveredDeploymentRuntimeAuthority {
    /// Borrow the deployment binding reference admitted with this authority.
    #[must_use]
    pub(in crate::resident) const fn binding_ref(&self) -> &ResidentRecordRefV1 {
        &self.binding_ref
    }

    /// Borrow the admitted deployment binding.
    #[must_use]
    pub(in crate::resident) const fn binding(&self) -> &DeploymentBindingV1 {
        &self.binding
    }

    /// Borrow the exact admitted runtime-authority reference.
    #[must_use]
    pub(in crate::resident) const fn record_ref(&self) -> &ResidentRecordRefV1 {
        &self.record_ref
    }

    /// Borrow the admitted runtime-authority record.
    #[must_use]
    pub(in crate::resident) const fn authority(&self) -> &DeploymentRuntimeAuthorityV1 {
        &self.authority
    }

    /// Borrow the canonical active-head projection.
    #[must_use]
    pub(in crate::resident) const fn head(&self) -> &DeploymentRuntimeAuthorityHeadV1 {
        &self.head
    }
}

/// Current frontier closed over the transition subset implemented by Pale.
///
/// The value retains the complete signed checkpoint chain, the producer
/// frontier, exact physical finalization evidence, and every admitted active
/// checkpoint/runtime authority. It proves neither current principal-key
/// selection nor any external anti-rollback minimum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::resident) struct RecoveredCurrentPrincipalTrustFrontier {
    checkpoints: RecoveredSignedPrincipalTrustCheckpointChain,
    producers: RecoveredPrincipalProducerIncarnationFrontier,
    finalizations: Option<DeploymentFinalizationSetSnapshot>,
    finalization_contexts: Box<[RecoveredDeploymentFinalizationContext]>,
    checkpoint_authorities: Box<[RecoveredCheckpointSigningAuthority]>,
    runtime_authorities: Box<[RecoveredDeploymentRuntimeAuthority]>,
}

impl RecoveredCurrentPrincipalTrustFrontier {
    #[cfg(feature = "interface-test-support")]
    pub(in crate::resident) fn for_interface_conformance(
        checkpoint_ref: ResidentRecordRefV1,
        checkpoint: PrincipalTrustCheckpointV1,
        binding_ref: ResidentRecordRefV1,
        binding: DeploymentBindingV1,
        runtime_authority_ref: ResidentRecordRefV1,
        runtime_authority: DeploymentRuntimeAuthorityV1,
    ) -> Result<Self, InvalidResidentRecordV1> {
        use super::principal_trust_recovery::RecoveredSignedPrincipalTrustCheckpointChain;
        use super::RecoveredPrincipalProducerIncarnationFrontier;
        use pale_ports::PrincipalTrustCheckpointHead;

        let checkpoints = RecoveredSignedPrincipalTrustCheckpointChain::for_interface_conformance(
            checkpoint_ref,
            checkpoint,
        )?;
        let current = checkpoints.current();
        let through = PrincipalTrustCheckpointHead::new(
            current.checkpoint().principal_id().clone(),
            *current.checkpoint().checkpoint_epoch(),
            current.coordinate().clone(),
        );
        let producers =
            RecoveredPrincipalProducerIncarnationFrontier::empty_for_interface_conformance(through);
        let head = runtime_authority.active_head(runtime_authority_ref.clone())?;
        Ok(Self {
            checkpoints,
            producers,
            finalizations: None,
            finalization_contexts: Box::new([]),
            checkpoint_authorities: Box::new([]),
            runtime_authorities: vec![RecoveredDeploymentRuntimeAuthority {
                binding_ref,
                binding,
                record_ref: runtime_authority_ref,
                authority: runtime_authority,
                head,
            }]
            .into_boxed_slice(),
        })
    }

    /// Borrow the final signed checkpoint.
    #[must_use]
    pub(in crate::resident) fn current_checkpoint(
        &self,
    ) -> &RecoveredSignedPrincipalTrustCheckpoint {
        self.checkpoints.current()
    }

    /// Borrow every admitted trust checkpoint in succession order.
    #[must_use]
    pub(in crate::resident) const fn checkpoints(
        &self,
    ) -> &[RecoveredSignedPrincipalTrustCheckpoint] {
        self.checkpoints.checkpoints()
    }

    /// Borrow the complete admitted producer frontier.
    #[must_use]
    pub(in crate::resident) const fn producers(
        &self,
    ) -> &RecoveredPrincipalProducerIncarnationFrontier {
        &self.producers
    }

    /// Borrow the exact physical deployment-finalization evidence, when present.
    #[must_use]
    pub(in crate::resident) const fn finalizations(
        &self,
    ) -> Option<&DeploymentFinalizationSetSnapshot> {
        self.finalizations.as_ref()
    }

    /// Borrow finalization inputs retained at their exact trust transitions.
    #[must_use]
    pub(in crate::resident) const fn finalization_contexts(
        &self,
    ) -> &[RecoveredDeploymentFinalizationContext] {
        &self.finalization_contexts
    }

    /// Borrow active checkpoint-signing authorities in canonical head order.
    #[must_use]
    pub(in crate::resident) const fn checkpoint_authorities(
        &self,
    ) -> &[RecoveredCheckpointSigningAuthority] {
        &self.checkpoint_authorities
    }

    /// Borrow active deployment runtime authorities in canonical head order.
    #[must_use]
    pub(in crate::resident) const fn runtime_authorities(
        &self,
    ) -> &[RecoveredDeploymentRuntimeAuthority] {
        &self.runtime_authorities
    }
}

/// Why admitted historical evidence does not close the current trust frontier.
#[derive(Debug, thiserror::Error)]
pub(in crate::resident) enum PrincipalTrustFrontierRecoveryError {
    /// An existing record or canonical-set law rejected supplied evidence.
    #[error(transparent)]
    Record(#[from] InvalidResidentRecordV1),
    /// The bootstrap, checkpoint chain, and producer evidence select unlike heads.
    #[error("principal trust recovery evidence does not select one frontier")]
    EvidenceAssociation,
    /// A successor changes an immutable principal or direct-authority coordinate.
    #[error("principal trust checkpoint changes a direct principal record or authority")]
    DirectTrustChange,
    /// A successor changes the principal signer coordinate.
    #[error("principal trust checkpoint changes the principal signer")]
    PrincipalSignerChange,
    /// A successor changes the admitted minimum session epoch.
    #[error("principal trust checkpoint changes the session floor")]
    SessionFloorChange,
    /// A currently unsupported frontier is not the canonical empty set.
    #[error("principal trust checkpoint uses an unsupported non-empty frontier")]
    UnsupportedFrontier,
    /// Current producer evidence does not reproduce the selected checkpoint root.
    #[error("principal producer frontier does not match the current trust checkpoint")]
    ProducerFrontier,
    /// Physical deployment-finalization evidence belongs to another trust store.
    #[error("deployment finalization evidence does not match the trust store")]
    FinalizationSetAssociation,
    /// A signer-root change has no matching retained finalization closure.
    #[error("principal trust checkpoint is missing its deployment finalization")]
    MissingFinalization,
    /// A retained finalization does not correspond to a signer-root change.
    #[error("deployment finalization evidence is surplus to the checkpoint chain")]
    SurplusFinalization,
    /// Finalization closures do not follow checkpoint succession order.
    #[error("deployment finalization evidence is not in checkpoint order")]
    FinalizationOrder,
    /// A retained finalization follows another predecessor or successor branch.
    #[error("deployment finalization evidence belongs to a different trust branch")]
    FinalizationFork,
    /// Typed finalization records do not reproduce their stored bytes.
    #[error("admitted deployment finalization records do not match storage")]
    FinalizationRecordMismatch,
    /// A retained finalization contains a successor authority, not an initial one.
    #[error("deployment finalization attempts an authority rotation")]
    AuthorityRotation,
    /// The existing exact finalization join rejected this transition.
    #[error(transparent)]
    Finalization(#[from] DeploymentFinalizationError),
    /// A checkpoint transition is not one of the two implemented changes.
    #[error("principal trust checkpoint transition is not supported during recovery")]
    UnsupportedTransition,
}

/// Close a current trust frontier over already admitted historical evidence.
///
/// Bootstrap supplies the initial checkpoint signer. Each retained deployment
/// finalization may add exactly one checkpoint signer and one runtime authority.
/// Every other successor may change only the producer-head root. The function
/// reproduces stored finalization bytes before using the existing application
/// joins and compares all resulting roots with the final checkpoint.
///
/// # Errors
///
/// Rejects unlike evidence, changed direct authority or signer coordinates,
/// unsupported non-empty roots, missing or surplus finalizations, finalization
/// forks or order changes, substituted record bytes, unsupported rotations, and
/// disagreement between the reconstructed head sets and the current checkpoint.
pub(in crate::resident) fn recover_current_principal_trust_frontier(
    bootstrap: &AdmittedPrincipalBootstrapV1,
    checkpoints: RecoveredSignedPrincipalTrustCheckpointChain,
    producers: RecoveredPrincipalProducerIncarnationFrontier,
    finalizations: Option<DeploymentFinalizationSetSnapshot>,
    admission: &mut dyn HistoricalDeploymentFinalizationAdmission,
) -> Result<RecoveredCurrentPrincipalTrustFrontier, PrincipalTrustFrontierRecoveryError> {
    let bootstrap_view = bootstrap.atom_view();
    let (bootstrap_ref, bootstrap_checkpoint) = bootstrap_view.trust_checkpoint_entry();
    let recovered = checkpoints.checkpoints();
    let Some(first) = recovered.first() else {
        return Err(PrincipalTrustFrontierRecoveryError::EvidenceAssociation);
    };
    if first.coordinate().record_ref() != bootstrap_ref
        || first.checkpoint() != bootstrap_checkpoint
        || producers.through() != checkpoints.evidence().through()
    {
        return Err(PrincipalTrustFrontierRecoveryError::EvidenceAssociation);
    }
    if finalizations
        .as_ref()
        .is_some_and(|set| set.binding() != checkpoints.evidence().binding())
    {
        return Err(PrincipalTrustFrontierRecoveryError::FinalizationSetAssociation);
    }

    let authorities = bootstrap_view.authorities();
    let bootstrap_signer = authorities.checkpoint_authority();
    let bootstrap_signer_ref = authorities.checkpoint_authority_ref().clone();
    let bootstrap_head = bootstrap_signer.active_head(bootstrap_signer_ref.clone())?;
    let mut checkpoint_heads = vec![bootstrap_head.clone()];
    let mut checkpoint_authorities = vec![RecoveredCheckpointSigningAuthority {
        record_ref: bootstrap_signer_ref,
        authority: bootstrap_signer.clone(),
        head: bootstrap_head,
    }];
    let mut runtime_heads = Vec::new();
    let mut runtime_authorities = Vec::new();
    let mut finalization_contexts = Vec::new();

    let finalization_slice = finalizations
        .as_ref()
        .map_or(&[][..], DeploymentFinalizationSetSnapshot::finalizations);
    require_distinct_finalization_successors(finalization_slice)?;
    let mut finalization_index = 0usize;

    for (predecessor_checkpoint_index, pair) in recovered.windows(2).enumerate() {
        let predecessor = &pair[0];
        let successor = &pair[1];
        require_stable_supported_fields(bootstrap_checkpoint, successor.checkpoint())?;
        let transition = AdmittedPrincipalTrustCheckpointTransition::new(
            predecessor.coordinate().record_ref(),
            predecessor.checkpoint(),
            successor.coordinate().record_ref(),
            successor.checkpoint(),
        )?;
        let signer_roots_changed = predecessor
            .checkpoint()
            .checkpoint_signing_authority_heads_root_sha256()
            != successor
                .checkpoint()
                .checkpoint_signing_authority_heads_root_sha256()
            || predecessor
                .checkpoint()
                .deployment_runtime_authority_heads_root_sha256()
                != successor
                    .checkpoint()
                    .deployment_runtime_authority_heads_root_sha256();

        if signer_roots_changed {
            let Some(physical) = finalization_slice.get(finalization_index) else {
                return Err(PrincipalTrustFrontierRecoveryError::MissingFinalization);
            };
            require_finalization_position(
                physical,
                predecessor,
                successor,
                finalization_slice,
                finalization_index,
            )?;
            require_exact_trust_records(
                checkpoints.evidence().checkpoint_records(),
                physical,
                predecessor,
                successor,
            )?;
            let records = admission.admit_historical_deployment_finalization(
                physical.records().binding(),
                physical.records().checkpoint_signing_authority(),
                physical.records().runtime_authority(),
            )?;
            require_exact_finalization_records(physical, &records)?;
            require_initial_finalization_authorities(&records)?;
            let admitted =
                AdmittedDeploymentFinalization::admit(DeploymentFinalizationCandidate {
                    binding: (&records.binding_ref, &records.binding),
                    checkpoint_signer: (&records.checkpoint_signer_ref, &records.checkpoint_signer),
                    runtime_authority: (&records.runtime_authority_ref, &records.runtime_authority),
                    trust: transition,
                    checkpoint_heads: &checkpoint_heads,
                    runtime_heads: &runtime_heads,
                })?;
            let checkpoint_head = admitted.checkpoint_signer_head().clone();
            let runtime_head = admitted.runtime_authority_head().clone();
            finalization_contexts.push(RecoveredDeploymentFinalizationContext {
                records: records.clone(),
                predecessor_checkpoint_index,
                successor_checkpoint_index: predecessor_checkpoint_index + 1,
                predecessor_checkpoint_heads: checkpoint_heads.clone().into_boxed_slice(),
                predecessor_runtime_heads: runtime_heads.clone().into_boxed_slice(),
            });
            insert_checkpoint_authority(
                &mut checkpoint_heads,
                &mut checkpoint_authorities,
                RecoveredCheckpointSigningAuthority {
                    record_ref: records.checkpoint_signer_ref,
                    authority: records.checkpoint_signer,
                    head: checkpoint_head,
                },
            )?;
            insert_runtime_authority(
                &mut runtime_heads,
                &mut runtime_authorities,
                RecoveredDeploymentRuntimeAuthority {
                    binding_ref: records.binding_ref,
                    binding: records.binding,
                    record_ref: records.runtime_authority_ref,
                    authority: records.runtime_authority,
                    head: runtime_head,
                },
            )?;
            finalization_index += 1;
        } else {
            if finalization_slice
                .get(finalization_index)
                .is_some_and(|physical| {
                    physical.successor_trust().coordinate() == successor.coordinate()
                        || physical.predecessor_trust().coordinate() == predecessor.coordinate()
                })
            {
                return Err(PrincipalTrustFrontierRecoveryError::SurplusFinalization);
            }
            if !transition.preserves_other_heads(TrustHeadChange::ProducerIncarnations)
                || predecessor
                    .checkpoint()
                    .producer_incarnation_heads_root_sha256()
                    == successor
                        .checkpoint()
                        .producer_incarnation_heads_root_sha256()
            {
                return Err(PrincipalTrustFrontierRecoveryError::UnsupportedTransition);
            }
        }
    }
    if finalization_index != finalization_slice.len() {
        return Err(PrincipalTrustFrontierRecoveryError::SurplusFinalization);
    }

    let current = checkpoints.current().checkpoint();
    require_stable_supported_fields(bootstrap_checkpoint, current)?;
    if producer_incarnation_heads_root_sha256(producers.heads())?
        != *current.producer_incarnation_heads_root_sha256()
    {
        return Err(PrincipalTrustFrontierRecoveryError::ProducerFrontier);
    }
    if checkpoint_signing_authority_heads_root_sha256(&checkpoint_heads)?
        != *current.checkpoint_signing_authority_heads_root_sha256()
        || deployment_runtime_authority_heads_root_sha256(&runtime_heads)?
            != *current.deployment_runtime_authority_heads_root_sha256()
    {
        return Err(PrincipalTrustFrontierRecoveryError::UnsupportedTransition);
    }

    Ok(RecoveredCurrentPrincipalTrustFrontier {
        checkpoints,
        producers,
        finalizations,
        finalization_contexts: finalization_contexts.into_boxed_slice(),
        checkpoint_authorities: checkpoint_authorities.into_boxed_slice(),
        runtime_authorities: runtime_authorities.into_boxed_slice(),
    })
}

fn require_initial_finalization_authorities(
    records: &AdmittedHistoricalDeploymentFinalizationRecords,
) -> Result<(), PrincipalTrustFrontierRecoveryError> {
    if records.checkpoint_signer.signer_epoch().get() != 1
        || records.checkpoint_signer.previous_authority_ref().is_some()
        || records.runtime_authority.authority_epoch().get() != 1
        || records.runtime_authority.previous_authority_ref().is_some()
    {
        Err(PrincipalTrustFrontierRecoveryError::AuthorityRotation)
    } else {
        Ok(())
    }
}

fn require_stable_supported_fields(
    initial: &PrincipalTrustCheckpointV1,
    candidate: &PrincipalTrustCheckpointV1,
) -> Result<(), PrincipalTrustFrontierRecoveryError> {
    if candidate.principal_record_ref() != initial.principal_record_ref()
        || candidate.principal_record_sha256() != initial.principal_record_sha256()
        || candidate.recovery_policy_ref() != initial.recovery_policy_ref()
        || candidate.recovery_policy_sha256() != initial.recovery_policy_sha256()
        || candidate.local_admin_authority_ref() != initial.local_admin_authority_ref()
        || candidate.local_admin_authority_sha256() != initial.local_admin_authority_sha256()
        || candidate.producer_incarnation_authority_ref()
            != initial.producer_incarnation_authority_ref()
        || candidate.producer_incarnation_authority_sha256()
            != initial.producer_incarnation_authority_sha256()
    {
        return Err(PrincipalTrustFrontierRecoveryError::DirectTrustChange);
    }
    if candidate.issuer_principal_id() != initial.issuer_principal_id()
        || candidate.issuer_key_id() != initial.issuer_key_id()
        || candidate.issuer_key_epoch() != initial.issuer_key_epoch()
    {
        return Err(PrincipalTrustFrontierRecoveryError::PrincipalSignerChange);
    }
    if candidate.session_floor_epoch() != initial.session_floor_epoch() {
        return Err(PrincipalTrustFrontierRecoveryError::SessionFloorChange);
    }
    let empty = [] as [(); 0];
    let owner_records = OwnerRecordSigningAuthorityHeadsRootDigest::hash_canonical(&empty)
        .map_err(InvalidResidentRecordV1::from)?;
    let owner_publication = OwnerSignerPublicationFrontierHeadsRootDigest::hash_canonical(&empty)
        .map_err(InvalidResidentRecordV1::from)?;
    let acceptance_revocations =
        DeploymentAcceptanceRevocationHeadsRootDigest::hash_canonical(&empty)
            .map_err(InvalidResidentRecordV1::from)?;
    let federation_homes = FederationSpaceHomeHeadsRootDigest::hash_canonical(&empty)
        .map_err(InvalidResidentRecordV1::from)?;
    let node_delegations = NodeDelegationHeadsRootDigest::hash_canonical(&empty)
        .map_err(InvalidResidentRecordV1::from)?;
    let node_delegation_revocations =
        NodeDelegationRevocationHeadsRootDigest::hash_canonical(&empty)
            .map_err(InvalidResidentRecordV1::from)?;
    if candidate.owner_record_signing_authority_heads_root_sha256() != &owner_records
        || candidate.owner_signer_publication_frontier_heads_root_sha256() != &owner_publication
        || candidate.deployment_acceptance_revocation_heads_root_sha256() != &acceptance_revocations
        || candidate.federation_space_home_heads_root_sha256() != &federation_homes
        || candidate.node_delegation_heads_root_sha256() != &node_delegations
        || candidate.node_delegation_revocation_heads_root_sha256() != &node_delegation_revocations
    {
        return Err(PrincipalTrustFrontierRecoveryError::UnsupportedFrontier);
    }
    Ok(())
}

fn require_distinct_finalization_successors(
    finalizations: &[DeploymentFinalizationSnapshot],
) -> Result<(), PrincipalTrustFrontierRecoveryError> {
    let mut seen = BTreeSet::new();
    if finalizations
        .iter()
        .any(|finalization| !seen.insert(finalization.successor_trust().coordinate()))
    {
        Err(PrincipalTrustFrontierRecoveryError::FinalizationOrder)
    } else {
        Ok(())
    }
}

fn require_finalization_position(
    physical: &DeploymentFinalizationSnapshot,
    predecessor: &RecoveredSignedPrincipalTrustCheckpoint,
    successor: &RecoveredSignedPrincipalTrustCheckpoint,
    finalizations: &[DeploymentFinalizationSnapshot],
    current_index: usize,
) -> Result<(), PrincipalTrustFrontierRecoveryError> {
    let successor_is_later = finalizations[current_index + 1..]
        .iter()
        .any(|candidate| candidate.successor_trust().coordinate() == successor.coordinate());
    if successor_is_later {
        return Err(PrincipalTrustFrontierRecoveryError::FinalizationOrder);
    }
    if physical.predecessor_trust().coordinate() != predecessor.coordinate()
        || physical.successor_trust().coordinate() != successor.coordinate()
        || physical.predecessor_trust().checkpoint_epoch()
            != *predecessor.checkpoint().checkpoint_epoch()
        || physical.successor_trust().checkpoint_epoch()
            != *successor.checkpoint().checkpoint_epoch()
    {
        return Err(PrincipalTrustFrontierRecoveryError::FinalizationFork);
    }
    Ok(())
}

fn require_exact_trust_records(
    checkpoint_records: &[pale_ports::PrincipalTrustCheckpointRecord],
    physical: &DeploymentFinalizationSnapshot,
    predecessor: &RecoveredSignedPrincipalTrustCheckpoint,
    successor: &RecoveredSignedPrincipalTrustCheckpoint,
) -> Result<(), PrincipalTrustFrontierRecoveryError> {
    let find = |coordinate: &ControlRecordCoordinate| {
        checkpoint_records
            .iter()
            .find(|record| record.record().coordinate() == coordinate)
            .map(pale_ports::PrincipalTrustCheckpointRecord::record)
    };
    if find(predecessor.coordinate()) != Some(physical.predecessor_trust_record())
        || find(successor.coordinate()) != Some(physical.records().trust_checkpoint())
    {
        Err(PrincipalTrustFrontierRecoveryError::FinalizationRecordMismatch)
    } else {
        Ok(())
    }
}

fn require_exact_finalization_records(
    physical: &DeploymentFinalizationSnapshot,
    records: &AdmittedHistoricalDeploymentFinalizationRecords,
) -> Result<(), PrincipalTrustFrontierRecoveryError> {
    require_exact_record(
        physical.records().binding(),
        &records.binding_ref,
        &records.binding,
    )?;
    require_exact_record(
        physical.records().checkpoint_signing_authority(),
        &records.checkpoint_signer_ref,
        &records.checkpoint_signer,
    )?;
    require_exact_record(
        physical.records().runtime_authority(),
        &records.runtime_authority_ref,
        &records.runtime_authority,
    )
}

fn require_exact_record<T>(
    physical: &ControlRecord,
    record_ref: &ResidentRecordRefV1,
    semantic: &T,
) -> Result<(), PrincipalTrustFrontierRecoveryError>
where
    T: serde::Serialize + record_projection::ResidentRecordSchema,
{
    let projected = record_projection::record(physical.coordinate().clone(), semantic)
        .map_err(|_| PrincipalTrustFrontierRecoveryError::FinalizationRecordMismatch)?;
    if physical.coordinate().record_ref() != record_ref || projected != *physical {
        Err(PrincipalTrustFrontierRecoveryError::FinalizationRecordMismatch)
    } else {
        Ok(())
    }
}

fn insert_checkpoint_authority(
    heads: &mut Vec<CheckpointSigningAuthorityHeadV1>,
    records: &mut Vec<RecoveredCheckpointSigningAuthority>,
    record: RecoveredCheckpointSigningAuthority,
) -> Result<(), PrincipalTrustFrontierRecoveryError> {
    let index = heads.binary_search_by(|head| {
        head.checkpoint_signing_authority_id
            .cmp(&record.head.checkpoint_signing_authority_id)
    });
    let Err(index) = index else {
        return Err(PrincipalTrustFrontierRecoveryError::UnsupportedTransition);
    };
    heads.insert(index, record.head.clone());
    records.insert(index, record);
    Ok(())
}

fn insert_runtime_authority(
    heads: &mut Vec<DeploymentRuntimeAuthorityHeadV1>,
    records: &mut Vec<RecoveredDeploymentRuntimeAuthority>,
    record: RecoveredDeploymentRuntimeAuthority,
) -> Result<(), PrincipalTrustFrontierRecoveryError> {
    let index = heads.binary_search_by(|head| {
        head.runtime_authority_id
            .cmp(&record.head.runtime_authority_id)
    });
    let Err(index) = index else {
        return Err(PrincipalTrustFrontierRecoveryError::UnsupportedTransition);
    };
    heads.insert(index, record.head.clone());
    records.insert(index, record);
    Ok(())
}

#[cfg(test)]
#[path = "../../tests/internal/resident/principal_trust_frontier.rs"]
pub(in crate::resident) mod tests;
