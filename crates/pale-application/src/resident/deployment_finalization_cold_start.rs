//! Classify finalized deployments from admitted cold-start evidence.
//!
//! Current trust closure, the historical principal-control chain, authenticated
//! administration records, and independent anchors remain separate proofs. This
//! module joins them for transaction one, then asks the selected deployment-store
//! reader whether transaction-two material is absent. It does not admit deployment
//! genesis, change an anchor, open a path, select current authority, or make a
//! deployment ready.

use pale_core::records::{
    ControlStateCheckpointV1, InvalidResidentRecordV1, PrincipalTrustCheckpointV1, SchemaDigest,
};
use pale_ports::{
    AntiRollbackAnchor, AntiRollbackAnchorError, AntiRollbackAnchorObservation,
    AntiRollbackAnchorPort, ControlDomainAntiRollbackAnchor, ControlStoreError,
    DeploymentControlStoreBinding, DeploymentControlStoreGenesisReadLimits,
    DeploymentControlStoreGenesisSnapshot, PreparedAntiRollbackAnchorTransition,
    PrincipalTrustMinimumAnchor,
};

use super::cold_start::{
    ColdStartAnchorState, ColdStartContinuation, ColdStartFacts, ColdStartTrustState,
};
use super::deployment_finalization::recovery::{
    recover_historical_deployment_finalization, HistoricalDeploymentFinalizationRecoveryBasis,
    HistoricalDeploymentFinalizationRecoveryError, HistoricalLocalAdminAuthorityResolver,
    HistoricalLocalAdminCommandSignatureVerifier, HistoricalLocalAdminReceiptSignatureVerifier,
    RecoveredHistoricalDeploymentFinalization,
};
use super::principal_control_checkpoint_recovery::{
    RecoveredPrincipalControlCheckpoint, RecoveredPrincipalControlCheckpointChain,
};
use super::principal_trust_frontier::RecoveredCurrentPrincipalTrustFrontier;
use super::principal_trust_recovery::RecoveredSignedPrincipalTrustCheckpoint;
use super::record_projection;
use super::{
    AdmittedDeploymentFinalization, AdmittedPrincipalTrustCheckpointTransition,
    DeploymentFinalizationCandidate, DeploymentFinalizationError,
};

/// Read transaction-two physical evidence from the caller-selected deployment store.
pub(in crate::resident) trait DeploymentGenesisEvidenceResolver {
    /// Read one exact deployment binding without discovering or opening a path here.
    fn read_deployment_genesis(
        &mut self,
        binding: &DeploymentControlStoreBinding,
        limits: DeploymentControlStoreGenesisReadLimits,
    ) -> Result<Option<DeploymentControlStoreGenesisSnapshot>, ControlStoreError>;
}

impl<F> DeploymentGenesisEvidenceResolver for F
where
    F: FnMut(
        &DeploymentControlStoreBinding,
        DeploymentControlStoreGenesisReadLimits,
    ) -> Result<Option<DeploymentControlStoreGenesisSnapshot>, ControlStoreError>,
{
    fn read_deployment_genesis(
        &mut self,
        binding: &DeploymentControlStoreBinding,
        limits: DeploymentControlStoreGenesisReadLimits,
    ) -> Result<Option<DeploymentControlStoreGenesisSnapshot>, ControlStoreError> {
        self(binding, limits)
    }
}

/// Transaction-one condition established for one deployment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::resident) enum DeploymentFinalizationColdStartPhase {
    /// The database commit is authenticated but one or both exact anchors lag it.
    FinalizationAnchorsPending,
    /// Transaction-one anchors are exact and no transaction-two material exists.
    FinalizedWithoutGenesis,
}

/// Admitted transaction-one outcome and its bounded cold-start classification.
pub(in crate::resident) struct RecoveredDeploymentFinalizationColdStart {
    binding: DeploymentControlStoreBinding,
    finalization: RecoveredHistoricalDeploymentFinalization,
    phase: DeploymentFinalizationColdStartPhase,
}

impl RecoveredDeploymentFinalizationColdStart {
    /// Borrow the exact deployment-store binding derived from finalized records.
    #[must_use]
    pub(in crate::resident) const fn binding(&self) -> &DeploymentControlStoreBinding {
        &self.binding
    }

    /// Borrow the authenticated historical command, receipt, and physical closure.
    #[must_use]
    pub(in crate::resident) const fn finalization(
        &self,
    ) -> &RecoveredHistoricalDeploymentFinalization {
        &self.finalization
    }

    /// Return the transaction-one state established for this deployment.
    #[must_use]
    pub(in crate::resident) const fn phase(&self) -> DeploymentFinalizationColdStartPhase {
        self.phase
    }

    /// Project only the admitted facts consumed by the pure cold-start classifier.
    #[must_use]
    pub(in crate::resident) const fn cold_start_facts(&self) -> ColdStartFacts {
        cold_start_facts_for_phase(self.phase)
    }
}

/// Why finalized-deployment evidence cannot be classified after restart.
#[derive(Debug, thiserror::Error)]
pub(in crate::resident) enum DeploymentFinalizationColdStartError {
    /// Trust and principal-control evidence do not belong to one store incarnation.
    #[error("deployment finalization evidence does not select one principal store")]
    EvidenceAssociation,
    /// Trust closure retained unlike counts of physical and semantic finalizations.
    #[error("principal trust frontier has incomplete deployment finalization context")]
    FinalizationSet,
    /// A finalization does not select its exact trust transition.
    #[error("deployment finalization does not match its principal trust transition")]
    TrustTransition,
    /// No unique adjacent principal-control checkpoint pair matches the finalization.
    #[error("deployment finalization does not match one principal control transition")]
    ControlTransition,
    /// Existing finalization admission rejected the joined records and head sets.
    #[error(transparent)]
    Finalization(#[from] DeploymentFinalizationError),
    /// Historical command or receipt authentication rejected the retained closure.
    #[error(transparent)]
    HistoricalFinalization(#[from] HistoricalDeploymentFinalizationRecoveryError),
    /// A retained semantic record cannot be projected to its immutable coordinate.
    #[error("deployment finalization record coordinate is invalid")]
    RecordCoordinate,
    /// The independent trust or control anchor cannot be read.
    #[error("deployment finalization anchor cannot be observed")]
    Anchor(#[source] AntiRollbackAnchorError),
    /// An independent anchor is absent, behind, conflicting, or ahead of admitted history.
    #[error("deployment finalization anchor disagrees with admitted checkpoint history")]
    AnchorEvidence,
    /// The selected deployment store cannot return one exact physical answer.
    #[error("deployment control-store genesis cannot be read exactly")]
    GenesisRead(#[source] ControlStoreError),
    /// Transaction-two material exists but this tranche does not admit it.
    #[error("deployment control-store genesis requires separate semantic admission")]
    GenesisPresent,
    /// Two finalized deployments map to the same immutable binding coordinate.
    #[error("deployment finalization set repeats a deployment binding")]
    DuplicateDeployment,
    /// A generic signed-checkpoint relation failed while reconstructing the finalization.
    #[error(transparent)]
    Record(#[from] InvalidResidentRecordV1),
}

/// Join accepted transaction-one evidence and classify each deployment independently.
///
/// Results are ordered by deployment-binding coordinate, not by store row order.
/// A newer anchor is accepted only when its complete value equals a later checkpoint
/// in the already-admitted chain. A predecessor anchor is pending only after the
/// authenticated receipt has been readmitted against this exact transition.
#[allow(clippy::too_many_arguments)]
pub(in crate::resident) fn recover_deployment_finalization_cold_start(
    trust: &RecoveredCurrentPrincipalTrustFrontier,
    control: &RecoveredPrincipalControlCheckpointChain,
    command_schema_sha256: &SchemaDigest,
    receipt_schema_sha256: &SchemaDigest,
    genesis_limits: DeploymentControlStoreGenesisReadLimits,
    authority_resolver: &mut dyn HistoricalLocalAdminAuthorityResolver,
    command_signature_verifier: &mut dyn HistoricalLocalAdminCommandSignatureVerifier,
    receipt_signature_verifier: &mut dyn HistoricalLocalAdminReceiptSignatureVerifier,
    anchors: &mut dyn AntiRollbackAnchorPort,
    genesis: &mut dyn DeploymentGenesisEvidenceResolver,
) -> Result<Box<[RecoveredDeploymentFinalizationColdStart]>, DeploymentFinalizationColdStartError> {
    let Some(finalizations) = trust.finalizations() else {
        return Ok(Box::new([]));
    };
    if control.evidence().binding() != finalizations.binding() {
        return Err(DeploymentFinalizationColdStartError::EvidenceAssociation);
    }
    let physical = finalizations.finalizations();
    let contexts = trust.finalization_contexts();
    if physical.len() != contexts.len() {
        return Err(DeploymentFinalizationColdStartError::FinalizationSet);
    }

    let mut recovered = Vec::with_capacity(physical.len());
    for (snapshot, context) in physical.iter().zip(contexts) {
        let predecessor = trust
            .checkpoints()
            .get(context.predecessor_checkpoint_index())
            .ok_or(DeploymentFinalizationColdStartError::TrustTransition)?;
        let successor = trust
            .checkpoints()
            .get(context.successor_checkpoint_index())
            .ok_or(DeploymentFinalizationColdStartError::TrustTransition)?;
        require_trust_pair(snapshot, predecessor, successor)?;
        let trust_transition = AdmittedPrincipalTrustCheckpointTransition::new(
            predecessor.coordinate().record_ref(),
            predecessor.checkpoint(),
            successor.coordinate().record_ref(),
            successor.checkpoint(),
        )?;
        let records = context.records();
        let finalization =
            AdmittedDeploymentFinalization::admit(DeploymentFinalizationCandidate {
                binding: records.binding(),
                checkpoint_signer: records.checkpoint_signer(),
                runtime_authority: records.runtime_authority(),
                trust: trust_transition,
                checkpoint_heads: context.predecessor_checkpoint_heads(),
                runtime_heads: context.predecessor_runtime_heads(),
            })?;
        let (predecessor_control, successor_control, successor_control_index) =
            require_control_pair(snapshot, control.checkpoints())?;
        recovered.push(recover_one_finalization(
            snapshot.clone(),
            &finalization,
            (
                predecessor.coordinate().record_ref(),
                predecessor.checkpoint(),
            ),
            (successor.coordinate().record_ref(), successor.checkpoint()),
            (
                predecessor_control.coordinate().record_ref(),
                predecessor_control.checkpoint(),
            ),
            (
                successor_control.coordinate().record_ref(),
                successor_control.checkpoint(),
            ),
            later_trust_anchors(trust.checkpoints(), context.successor_checkpoint_index()),
            later_control_anchors(control.checkpoints(), successor_control_index),
            command_schema_sha256,
            receipt_schema_sha256,
            genesis_limits,
            authority_resolver,
            command_signature_verifier,
            receipt_signature_verifier,
            anchors,
            genesis,
        )?);
    }
    order_deployments(&mut recovered, |value| value.binding.deployment_binding())?;
    Ok(recovered.into_boxed_slice())
}

#[allow(clippy::too_many_arguments)]
fn recover_one_finalization(
    snapshot: pale_ports::DeploymentFinalizationSnapshot,
    finalization: &AdmittedDeploymentFinalization<'_>,
    predecessor_trust: (
        &pale_core::records::ResidentRecordRefV1,
        &PrincipalTrustCheckpointV1,
    ),
    successor_trust: (
        &pale_core::records::ResidentRecordRefV1,
        &PrincipalTrustCheckpointV1,
    ),
    predecessor_control: (
        &pale_core::records::ResidentRecordRefV1,
        &ControlStateCheckpointV1,
    ),
    successor_control: (
        &pale_core::records::ResidentRecordRefV1,
        &ControlStateCheckpointV1,
    ),
    later_trust: impl Iterator<Item = AntiRollbackAnchor>,
    later_control: impl Iterator<Item = AntiRollbackAnchor>,
    command_schema_sha256: &SchemaDigest,
    receipt_schema_sha256: &SchemaDigest,
    genesis_limits: DeploymentControlStoreGenesisReadLimits,
    authority_resolver: &mut dyn HistoricalLocalAdminAuthorityResolver,
    command_signature_verifier: &mut dyn HistoricalLocalAdminCommandSignatureVerifier,
    receipt_signature_verifier: &mut dyn HistoricalLocalAdminReceiptSignatureVerifier,
    anchors: &mut dyn AntiRollbackAnchorPort,
    genesis: &mut dyn DeploymentGenesisEvidenceResolver,
) -> Result<RecoveredDeploymentFinalizationColdStart, DeploymentFinalizationColdStartError> {
    let historical = recover_historical_deployment_finalization(
        snapshot,
        HistoricalDeploymentFinalizationRecoveryBasis {
            finalization,
            predecessor_trust,
            successor_trust,
            predecessor_control,
            successor_control,
            command_schema_sha256,
            receipt_schema_sha256,
        },
        authority_resolver,
        command_signature_verifier,
        receipt_signature_verifier,
    )?;
    let trust_pending = anchor_is_pending(
        anchors,
        trust_transition_anchors_from_records(predecessor_trust, successor_trust)?,
        later_trust,
    )?;
    let control_pending = anchor_is_pending(
        anchors,
        control_transition_anchors_from_records(predecessor_control, successor_control)?,
        later_control,
    )?;
    let binding = deployment_store_binding_from_finalization(finalization)?;
    require_absent_genesis(genesis.read_deployment_genesis(&binding, genesis_limits))?;
    let phase = if trust_pending || control_pending {
        DeploymentFinalizationColdStartPhase::FinalizationAnchorsPending
    } else {
        DeploymentFinalizationColdStartPhase::FinalizedWithoutGenesis
    };
    Ok(RecoveredDeploymentFinalizationColdStart {
        binding,
        finalization: historical,
        phase,
    })
}

fn order_deployments<T>(
    values: &mut [T],
    binding: impl Fn(&T) -> &pale_ports::ControlRecordCoordinate,
) -> Result<(), DeploymentFinalizationColdStartError> {
    values.sort_by(|left, right| binding(left).cmp(binding(right)));
    if values
        .windows(2)
        .any(|pair| binding(&pair[0]) == binding(&pair[1]))
    {
        Err(DeploymentFinalizationColdStartError::DuplicateDeployment)
    } else {
        Ok(())
    }
}

const fn cold_start_facts_for_phase(phase: DeploymentFinalizationColdStartPhase) -> ColdStartFacts {
    match phase {
        DeploymentFinalizationColdStartPhase::FinalizationAnchorsPending => ColdStartFacts::new(
            ColdStartTrustState::Admitted,
            ColdStartAnchorState::ReceiptNamedAdvancesPending,
            ColdStartContinuation::None,
        ),
        DeploymentFinalizationColdStartPhase::FinalizedWithoutGenesis => ColdStartFacts::new(
            ColdStartTrustState::Admitted,
            ColdStartAnchorState::Agrees,
            ColdStartContinuation::InactiveDeploymentGenesis,
        ),
    }
}

fn require_absent_genesis<T>(
    result: Result<Option<T>, ControlStoreError>,
) -> Result<(), DeploymentFinalizationColdStartError> {
    match result {
        Ok(None) => Ok(()),
        Ok(Some(_)) => Err(DeploymentFinalizationColdStartError::GenesisPresent),
        Err(error) => Err(DeploymentFinalizationColdStartError::GenesisRead(error)),
    }
}

fn require_trust_pair(
    snapshot: &pale_ports::DeploymentFinalizationSnapshot,
    predecessor: &RecoveredSignedPrincipalTrustCheckpoint,
    successor: &RecoveredSignedPrincipalTrustCheckpoint,
) -> Result<(), DeploymentFinalizationColdStartError> {
    if snapshot.predecessor_trust().coordinate() != predecessor.coordinate()
        || snapshot.successor_trust().coordinate() != successor.coordinate()
    {
        Err(DeploymentFinalizationColdStartError::TrustTransition)
    } else {
        Ok(())
    }
}

fn require_control_pair<'a>(
    snapshot: &pale_ports::DeploymentFinalizationSnapshot,
    chain: &'a [RecoveredPrincipalControlCheckpoint],
) -> Result<
    (
        &'a RecoveredPrincipalControlCheckpoint,
        &'a RecoveredPrincipalControlCheckpoint,
        usize,
    ),
    DeploymentFinalizationColdStartError,
> {
    let mut matches = chain.windows(2).enumerate().filter(|(_, pair)| {
        pair[0].coordinate() == snapshot.predecessor_control().coordinate()
            && pair[1].coordinate() == snapshot.successor_control().coordinate()
    });
    let Some((index, pair)) = matches.next() else {
        return Err(DeploymentFinalizationColdStartError::ControlTransition);
    };
    if matches.next().is_some() {
        return Err(DeploymentFinalizationColdStartError::ControlTransition);
    }
    Ok((&pair[0], &pair[1], index + 1))
}

fn trust_transition_anchors_from_records(
    predecessor: (
        &pale_core::records::ResidentRecordRefV1,
        &PrincipalTrustCheckpointV1,
    ),
    successor: (
        &pale_core::records::ResidentRecordRefV1,
        &PrincipalTrustCheckpointV1,
    ),
) -> Result<PreparedAntiRollbackAnchorTransition, DeploymentFinalizationColdStartError> {
    PreparedAntiRollbackAnchorTransition::successor(
        PrincipalTrustMinimumAnchor::new(
            predecessor.1.principal_id().clone(),
            *predecessor.1.checkpoint_epoch(),
            record_projection::coordinate(
                predecessor.0,
                predecessor.1.canonical_trust_checkpoint_sha256().as_str(),
            )
            .map_err(|_| DeploymentFinalizationColdStartError::RecordCoordinate)?,
        )
        .into(),
        PrincipalTrustMinimumAnchor::new(
            successor.1.principal_id().clone(),
            *successor.1.checkpoint_epoch(),
            record_projection::coordinate(
                successor.0,
                successor.1.canonical_trust_checkpoint_sha256().as_str(),
            )
            .map_err(|_| DeploymentFinalizationColdStartError::RecordCoordinate)?,
        )
        .into(),
    )
    .map_err(|_| DeploymentFinalizationColdStartError::TrustTransition)
}

fn control_transition_anchors_from_records(
    predecessor: (
        &pale_core::records::ResidentRecordRefV1,
        &ControlStateCheckpointV1,
    ),
    successor: (
        &pale_core::records::ResidentRecordRefV1,
        &ControlStateCheckpointV1,
    ),
) -> Result<PreparedAntiRollbackAnchorTransition, DeploymentFinalizationColdStartError> {
    let anchor = |value: (
        &pale_core::records::ResidentRecordRefV1,
        &ControlStateCheckpointV1,
    )| {
        Ok::<_, DeploymentFinalizationColdStartError>(AntiRollbackAnchor::from(
            ControlDomainAntiRollbackAnchor::new(
                value.1.control_store_id().clone(),
                value.1.control_incarnation_id().clone(),
                value.1.checkpoint_domain_id().clone(),
                *value.1.domain_epoch(),
                *value.1.domain_sequence(),
                record_projection::coordinate(
                    value.0,
                    value.1.canonical_control_checkpoint_sha256().as_str(),
                )
                .map_err(|_| DeploymentFinalizationColdStartError::RecordCoordinate)?,
            ),
        ))
    };
    PreparedAntiRollbackAnchorTransition::successor(anchor(predecessor)?, anchor(successor)?)
        .map_err(|_| DeploymentFinalizationColdStartError::ControlTransition)
}

fn trust_anchor(checkpoint: &RecoveredSignedPrincipalTrustCheckpoint) -> AntiRollbackAnchor {
    PrincipalTrustMinimumAnchor::new(
        checkpoint.checkpoint().principal_id().clone(),
        *checkpoint.checkpoint().checkpoint_epoch(),
        checkpoint.coordinate().clone(),
    )
    .into()
}

fn control_anchor(checkpoint: &RecoveredPrincipalControlCheckpoint) -> AntiRollbackAnchor {
    let value = checkpoint.checkpoint();
    ControlDomainAntiRollbackAnchor::new(
        value.control_store_id().clone(),
        value.control_incarnation_id().clone(),
        value.checkpoint_domain_id().clone(),
        *value.domain_epoch(),
        *value.domain_sequence(),
        checkpoint.coordinate().clone(),
    )
    .into()
}

fn later_trust_anchors(
    chain: &[RecoveredSignedPrincipalTrustCheckpoint],
    successor_index: usize,
) -> impl Iterator<Item = AntiRollbackAnchor> + '_ {
    chain.iter().skip(successor_index + 1).map(trust_anchor)
}

fn later_control_anchors(
    chain: &[RecoveredPrincipalControlCheckpoint],
    successor_index: usize,
) -> impl Iterator<Item = AntiRollbackAnchor> + '_ {
    chain.iter().skip(successor_index + 1).map(control_anchor)
}

fn anchor_is_pending(
    port: &mut dyn AntiRollbackAnchorPort,
    transition: PreparedAntiRollbackAnchorTransition,
    later: impl Iterator<Item = AntiRollbackAnchor>,
) -> Result<bool, DeploymentFinalizationColdStartError> {
    let observed = port
        .read(transition.key())
        .map_err(DeploymentFinalizationColdStartError::Anchor)?;
    match transition.classify_observed(observed.as_ref()) {
        AntiRollbackAnchorObservation::ExactPredecessor => Ok(true),
        AntiRollbackAnchorObservation::ExactSuccessor => Ok(false),
        AntiRollbackAnchorObservation::Ahead { observed }
            if later.into_iter().any(|candidate| candidate == observed) =>
        {
            Ok(false)
        }
        AntiRollbackAnchorObservation::Missing
        | AntiRollbackAnchorObservation::Behind { .. }
        | AntiRollbackAnchorObservation::Ahead { .. }
        | AntiRollbackAnchorObservation::Conflicting { .. } => {
            Err(DeploymentFinalizationColdStartError::AnchorEvidence)
        }
    }
}

fn deployment_store_binding_from_finalization(
    finalization: &AdmittedDeploymentFinalization<'_>,
) -> Result<DeploymentControlStoreBinding, DeploymentFinalizationColdStartError> {
    let records = finalization.records();
    Ok(DeploymentControlStoreBinding::new(
        records.runtime_authority.1.control_store_id().clone(),
        records.runtime_authority.1.control_incarnation_id().clone(),
        records.runtime_authority.1.principal_id().clone(),
        records.binding.1.deployment_terms().workspace_id().clone(),
        record_projection::coordinate(
            records.binding.0,
            records.binding.1.canonical_binding_sha256().as_str(),
        )
        .map_err(|_| DeploymentFinalizationColdStartError::RecordCoordinate)?,
        record_projection::coordinate(
            records.checkpoint_signer.0,
            records
                .checkpoint_signer
                .1
                .canonical_checkpoint_signing_authority_sha256()
                .as_str(),
        )
        .map_err(|_| DeploymentFinalizationColdStartError::RecordCoordinate)?,
        record_projection::coordinate(
            records.runtime_authority.0,
            records
                .runtime_authority
                .1
                .canonical_deployment_runtime_authority_sha256()
                .as_str(),
        )
        .map_err(|_| DeploymentFinalizationColdStartError::RecordCoordinate)?,
    ))
}

#[cfg(test)]
#[path = "../../tests/internal/resident/deployment_finalization_cold_start.rs"]
mod tests;
