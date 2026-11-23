//! Compose the two physical deployment-initialization transactions.
//!
//! The principal trust store finalizes authority first. Its database commit and
//! two independent anchors must be exact before a separately signed genesis can
//! initialize the deployment control store. These phases reuse the existing
//! record, custody, read-back, and anchor owners; they do not select signers,
//! stores, or production anchor mechanisms.

use pale_core::{canon, records::*};
use pale_ports::{
    AccumulatorCheckpointPhase, AnchoredDeploymentFinalizationCapability, AntiRollbackAnchor,
    AntiRollbackAnchorError, AntiRollbackAnchorKey, AntiRollbackAnchorPort,
    AntiRollbackAnchorTransitionError, AuthorityStoreCoordinate, ControlCheckpointHead,
    ControlDomainAntiRollbackAnchor, ControlRecordError,
    DatabaseCommittedDeploymentControlStoreGenesis, DeploymentControlStoreBinding,
    DeploymentControlStoreGenesisCandidateError, DeploymentControlStoreGenesisCommitEvidence,
    DeploymentControlStoreGenesisCommitFailure, DeploymentControlStoreGenesisCustodyPort,
    DeploymentControlStoreGenesisRecords, DeploymentFinalizationAnchorFailure,
    DeploymentFinalizationAnchorOwner, DeploymentFinalizationAnchorsPending,
    DeploymentFinalizationCandidateError, DeploymentFinalizationCommitEvidence,
    DeploymentFinalizationCommitFailure, DeploymentFinalizationCustodyPort,
    DeploymentFinalizationRecords, DeploymentFinalizationReplayIdentity, OperationAccumulatorBatch,
    PreparedAntiRollbackAnchorTransition, PreparedDeploymentControlStoreGenesisAtom,
    PreparedDeploymentFinalizationAtom, PrincipalTrustCheckpointHead, PrincipalTrustStoreBinding,
};

use super::{
    anchor_progress::{AnchorCompletionFailure, AnchorProgress, ObservationReason},
    custody_phase::AnchorAdvanceFailureReason,
    deployment_finalization::projection::FinalizationReceiptProjection,
    record_projection::{self, ControlRecordProjectionError},
    AnchorAdvanceEvidence, DeploymentFinalizationSubmission,
};

/// Coordinates and times that bind one deployment-finalization receipt.
///
/// A trusted administration composition root supplies these values from its
/// retained command and allocation policy. Constructing this value grants no
/// signing authority and changes no store.
pub struct DeploymentFinalizationReceiptParameters {
    command_ref: ResidentRecordRefV1,
    receipt_ref: ResidentRecordRefV1,
    receipt_id: LocalAdminReceiptIdV1,
    schema_sha256: SchemaDigest,
    committed_at: UnixTimeV1,
    recorded_at: UnixTimeV1,
}

impl DeploymentFinalizationReceiptParameters {
    /// Retain the exact command, receipt, schema, and transaction times.
    #[must_use]
    pub const fn new(
        command_ref: ResidentRecordRefV1,
        receipt_ref: ResidentRecordRefV1,
        receipt_id: LocalAdminReceiptIdV1,
        schema_sha256: SchemaDigest,
        committed_at: UnixTimeV1,
        recorded_at: UnixTimeV1,
    ) -> Self {
        Self {
            command_ref,
            receipt_ref,
            receipt_id,
            schema_sha256,
            committed_at,
            recorded_at,
        }
    }
}

/// Authenticated command and signed receipt for the principal-store transaction.
///
/// The command is already admitted against the selected administrator and
/// finalization candidate. The receipt is issued or admitted against the exact
/// runtime authority. This value is still only input to atomic custody.
#[must_use = "prepare or retain the authenticated finalization transaction"]
pub struct SignedDeploymentFinalizationInput<'a> {
    submission: DeploymentFinalizationSubmission<'a>,
    command_ref: ResidentRecordRefV1,
    receipt_ref: ResidentRecordRefV1,
    receipt: LocalAdminReceiptV1,
}

impl<'a> SignedDeploymentFinalizationInput<'a> {
    /// Issue transaction one's receipt under its exact admitted runtime authority.
    ///
    /// # Errors
    ///
    /// Rejects unlike command, finalization, checkpoint, or receipt coordinates,
    /// invalid record material, and a rejected signing operation.
    pub fn issue<F>(
        submission: DeploymentFinalizationSubmission<'a>,
        parameters: DeploymentFinalizationReceiptParameters,
        sign: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnOnce(
            &[u8],
            &LocalAdminReceiptPreimageV1,
            &LocalAdminReceiptDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let DeploymentFinalizationReceiptParameters {
            command_ref,
            receipt_ref,
            receipt_id,
            schema_sha256,
            committed_at,
            recorded_at,
        } = parameters;
        let association = FinalizationReceiptProjection::new(
            submission.construction().finalization(),
            submission.construction().checkpoint(),
            &command_ref,
            submission.command(),
        )?;
        let selected = submission.construction().finalization().records();
        let runtime = selected.runtime_authority.1;
        let preimage = LocalAdminReceiptPreimageV1 {
            schema_id: SchemaId::parse(LOCAL_ADMIN_RECEIPT_V1_SCHEMA)
                .map_err(|_| invalid_finalization_receipt())?,
            schema_sha256: schema_sha256.clone(),
            admin_receipt_id: receipt_id.clone(),
            admin_command_ref: command_ref.clone(),
            admin_command_sha256: submission
                .command()
                .canonical_admin_command_sha256()
                .clone(),
            disposition: LocalAdminDispositionV1::Committed,
            committed_records: association.committed_records().to_vec(),
            head_transitions: association.head_transitions().to_vec(),
            failure_code: None,
            committed_at: Some(committed_at),
            recorded_at,
            deployment_runtime_authority_ref: selected.runtime_authority.0.clone(),
            deployment_runtime_authority_sha256: runtime
                .canonical_deployment_runtime_authority_sha256()
                .clone(),
            issuer_principal_id: runtime.principal_id().clone(),
            issuer_key_id: runtime.subject_key_id().clone(),
            issuer_key_epoch: *runtime.subject_key_epoch(),
        };
        let receipt = LocalAdminReceiptV1::issue_contextual(
            preimage,
            &association.admission_context(
                submission.construction().finalization(),
                &command_ref,
                submission.command(),
                &receipt_id,
                &schema_sha256,
                &committed_at,
                &recorded_at,
            ),
            sign,
        )?;
        Ok(Self {
            submission,
            command_ref,
            receipt_ref,
            receipt,
        })
    }

    /// Admit transaction one's receipt under the same exact command and result.
    ///
    /// # Errors
    ///
    /// Rejects malformed or unsigned wire material, an unlike command or result,
    /// and a signature that the selected runtime authority does not verify.
    pub fn admit<F>(
        submission: DeploymentFinalizationSubmission<'a>,
        parameters: DeploymentFinalizationReceiptParameters,
        wire: LocalAdminReceiptWireV1,
        verify_signature: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnOnce(
            &Ed25519PublicKeyV1,
            &[u8],
            &LocalAdminReceiptPreimageV1,
            &LocalAdminReceiptDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let DeploymentFinalizationReceiptParameters {
            command_ref,
            receipt_ref,
            receipt_id,
            schema_sha256,
            committed_at,
            recorded_at,
        } = parameters;
        let association = FinalizationReceiptProjection::new(
            submission.construction().finalization(),
            submission.construction().checkpoint(),
            &command_ref,
            submission.command(),
        )?;
        let receipt = LocalAdminReceiptV1::admit_contextual_wire(
            wire,
            &association.admission_context(
                submission.construction().finalization(),
                &command_ref,
                submission.command(),
                &receipt_id,
                &schema_sha256,
                &committed_at,
                &recorded_at,
            ),
            verify_signature,
        )?;
        Ok(Self {
            submission,
            command_ref,
            receipt_ref,
            receipt,
        })
    }

    /// Close transaction one without writing either store or any anchor.
    ///
    /// # Errors
    ///
    /// Returns the unchanged authenticated input when physical record projection,
    /// candidate closure, or the predecessor checkpoint association fails.
    pub fn prepare(
        self,
    ) -> Result<PreparedDeploymentFinalization, DeploymentFinalizationPreparationFailure<'a>> {
        match prepare_finalization(&self) {
            Ok(prepared) => Ok(prepared),
            Err(reason) => Err(DeploymentFinalizationPreparationFailure {
                input: Box::new(self),
                reason,
            }),
        }
    }
}

fn invalid_finalization_receipt() -> InvalidResidentRecordV1 {
    InvalidResidentRecordV1::ContextualAdmission {
        record: "deployment finalization receipt",
        invariant: "valid exact signed command, records, heads and runtime authority",
    }
}

/// Closed principal-store candidate and its required anchor transitions.
#[must_use = "commit or retain the prepared deployment finalization"]
pub struct PreparedDeploymentFinalization {
    atom: PreparedDeploymentFinalizationAtom,
}

impl PreparedDeploymentFinalization {
    /// Borrow the closed physical candidate without granting write authority.
    #[must_use]
    pub fn atom(&self) -> &PreparedDeploymentFinalizationAtom {
        &self.atom
    }

    /// Commit transaction one with its existing exact read-back policy.
    ///
    /// # Errors
    ///
    /// Returns the complete prepared transaction when custody is rejected or
    /// mandatory read-back cannot prove whether the exact atom committed.
    pub fn commit(
        self,
        store: &mut dyn DeploymentFinalizationCustodyPort,
    ) -> Result<
        DatabaseCommittedDeploymentFinalizationAnchorsPending,
        Box<DeploymentFinalizationCommitPhaseFailure>,
    > {
        match self.atom.commit(store) {
            Ok(committed) => Ok(DatabaseCommittedDeploymentFinalizationAnchorsPending {
                pending: committed.anchors_pending(),
            }),
            Err(failure) => Err(Box::new(DeploymentFinalizationCommitPhaseFailure {
                failure,
            })),
        }
    }
}

/// Exact principal-store custody whose independent anchors remain pending.
#[must_use = "complete or retain the pending finalization anchors"]
pub struct DatabaseCommittedDeploymentFinalizationAnchorsPending {
    pending: DeploymentFinalizationAnchorsPending,
}

impl DatabaseCommittedDeploymentFinalizationAnchorsPending {
    /// Complete both independent principal-store anchors in their required order.
    ///
    /// # Errors
    ///
    /// Returns retained progress when an anchor cannot be observed or advanced.
    pub fn complete(
        self,
        anchors: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<AnchoredDeploymentFinalization, DeploymentFinalizationAnchorPhaseFailure> {
        self.pending
            .complete(anchors)
            .map(|capability| AnchoredDeploymentFinalization { capability })
            .map_err(|failure| DeploymentFinalizationAnchorPhaseFailure { failure })
    }
}

/// Exact principal-store finalization with both independent anchors proved.
///
/// Only this phase can prepare the separate deployment-store genesis.
#[must_use = "prepare deployment genesis or retain the anchored finalization"]
pub struct AnchoredDeploymentFinalization {
    capability: AnchoredDeploymentFinalizationCapability,
}

impl AnchoredDeploymentFinalization {
    /// Return how the principal-store transaction obtained exact custody.
    #[must_use]
    pub const fn database_commit_evidence(&self) -> DeploymentFinalizationCommitEvidence {
        self.capability.database_commit_evidence()
    }

    /// Close the separately signed transaction-two candidate.
    ///
    /// # Errors
    ///
    /// Returns both unchanged inputs when the signed genesis, physical records,
    /// accumulator batches, or required anchor transitions do not agree.
    pub fn prepare_genesis(
        self,
        input: SignedDeploymentGenesisInput,
    ) -> Result<PreparedDeploymentGenesis, DeploymentGenesisPreparationFailure> {
        match prepare_genesis(&self.capability, &input) {
            Ok((atom, anchors)) => Ok(PreparedDeploymentGenesis {
                finalization: self,
                atom,
                anchors,
            }),
            Err(reason) => Err(DeploymentGenesisPreparationFailure {
                finalization: Box::new(self),
                input: Box::new(input),
                reason,
            }),
        }
    }
}

/// One admitted initial checkpoint and its matching accumulator batch.
pub struct SignedDeploymentGenesisCheckpoint {
    record_ref: ResidentRecordRefV1,
    checkpoint: ControlStateCheckpointV1,
    accumulator: OperationAccumulatorBatch,
}

impl SignedDeploymentGenesisCheckpoint {
    /// Join the checkpoint's immutable reference to its prepared accumulator.
    #[must_use]
    pub const fn new(
        record_ref: ResidentRecordRefV1,
        checkpoint: ControlStateCheckpointV1,
        accumulator: OperationAccumulatorBatch,
    ) -> Self {
        Self {
            record_ref,
            checkpoint,
            accumulator,
        }
    }
}

/// Separately admitted and signed material for deployment-store genesis.
///
/// The receipt and checkpoints must already be authenticated by their semantic
/// owners. The application checks their association again before physical
/// custody. This container never signs or writes them.
#[must_use = "prepare or retain the signed deployment genesis"]
pub struct SignedDeploymentGenesisInput {
    binding: DeploymentControlStoreBinding,
    genesis_id: DeploymentControlStoreGenesisIdV1,
    receipt_ref: ResidentRecordRefV1,
    receipt: DeploymentControlStoreGenesisReceiptV1,
    checkpoints: Vec<SignedDeploymentGenesisCheckpoint>,
}

impl SignedDeploymentGenesisInput {
    /// Retain the exact store binding, replay identity, receipt, and checkpoints.
    pub fn new(
        binding: DeploymentControlStoreBinding,
        genesis_id: DeploymentControlStoreGenesisIdV1,
        receipt_ref: ResidentRecordRefV1,
        receipt: DeploymentControlStoreGenesisReceiptV1,
        checkpoints: Vec<SignedDeploymentGenesisCheckpoint>,
    ) -> Self {
        Self {
            binding,
            genesis_id,
            receipt_ref,
            receipt,
            checkpoints,
        }
    }
}

/// Closed deployment-store genesis candidate and its required domain anchors.
#[must_use = "commit or retain the prepared deployment genesis"]
pub struct PreparedDeploymentGenesis {
    finalization: AnchoredDeploymentFinalization,
    atom: PreparedDeploymentControlStoreGenesisAtom,
    anchors: Vec<AnchorLeg>,
}

impl PreparedDeploymentGenesis {
    /// Borrow the closed physical candidate without granting write authority.
    #[must_use]
    pub fn atom(&self) -> &PreparedDeploymentControlStoreGenesisAtom {
        &self.atom
    }

    /// Commit transaction two in its distinct deployment control store.
    ///
    /// # Errors
    ///
    /// Returns the complete prepared genesis when custody is rejected or
    /// mandatory read-back cannot prove whether the exact atom committed.
    pub fn commit(
        self,
        store: &mut dyn DeploymentControlStoreGenesisCustodyPort,
    ) -> Result<DeploymentGenesisAnchorsPending, DeploymentGenesisCommitPhaseFailure> {
        let Self {
            finalization,
            atom,
            anchors,
        } = self;
        match atom.commit(store) {
            Ok(committed) => Ok(DeploymentGenesisAnchorsPending::new(
                finalization,
                committed,
                anchors,
            )),
            Err(failure) => Err(DeploymentGenesisCommitPhaseFailure {
                finalization: Box::new(finalization),
                failure,
                anchors,
            }),
        }
    }
}

/// Exact deployment-store custody whose domain anchors remain pending.
#[must_use = "complete or retain the pending deployment genesis anchors"]
pub struct DeploymentGenesisAnchorsPending {
    finalization: AnchoredDeploymentFinalization,
    committed: DatabaseCommittedDeploymentControlStoreGenesis,
    anchors: Vec<AnchorLeg>,
}

impl DeploymentGenesisAnchorsPending {
    fn new(
        finalization: AnchoredDeploymentFinalization,
        committed: DatabaseCommittedDeploymentControlStoreGenesis,
        mut anchors: Vec<AnchorLeg>,
    ) -> Self {
        let progress = progress_after_genesis_commit(committed.evidence());
        for anchor in &mut anchors {
            anchor.progress = progress;
        }
        Self {
            finalization,
            committed,
            anchors,
        }
    }

    /// Prove every transaction-two domain anchor in canonical checkpoint order.
    ///
    /// # Panics
    /// Panics if an internal anchor leg reports completion without retaining
    /// its evidence. External observation or advance failures return an error.
    ///
    /// # Errors
    ///
    /// Returns retained progress when a domain anchor cannot be observed or advanced.
    pub fn complete(
        mut self,
        port: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<CompletedDeploymentInitialization, DeploymentGenesisAnchorPhaseFailure> {
        for index in 0..self.anchors.len() {
            if let Err(reason) = self.anchors[index].complete(port) {
                let owner = self.anchors[index].transition.key().clone();
                let reason = match reason {
                    AnchorCompletionFailure::Observation(error) => {
                        DeploymentGenesisAnchorPhaseFailureReason::Observation {
                            anchor: owner.clone(),
                            error,
                        }
                    }
                    AnchorCompletionFailure::Advance(reason) => {
                        DeploymentGenesisAnchorPhaseFailureReason::Advance {
                            anchor: owner.clone(),
                            reason,
                        }
                    }
                };
                return Err(DeploymentGenesisAnchorPhaseFailure {
                    pending: Box::new(self),
                    reason: Box::new(reason),
                });
            }
        }
        let anchor_evidence = self
            .anchors
            .iter()
            .map(|anchor| {
                anchor
                    .evidence()
                    .expect("successful completion retains genesis-anchor evidence")
            })
            .collect();
        Ok(CompletedDeploymentInitialization {
            finalization: self.finalization,
            genesis: self.committed,
            genesis_anchor_evidence: anchor_evidence,
        })
    }
}

/// Exact custody evidence for both stores and every required anchor.
///
/// This proves deployment initialization under the supplied storage and anchor
/// ports. It does not activate a resident or prove process health.
pub struct CompletedDeploymentInitialization {
    finalization: AnchoredDeploymentFinalization,
    genesis: DatabaseCommittedDeploymentControlStoreGenesis,
    genesis_anchor_evidence: Vec<AnchorAdvanceEvidence>,
}

impl CompletedDeploymentInitialization {
    /// Borrow the anchored principal-store finalization.
    pub fn finalization(&self) -> &AnchoredDeploymentFinalization {
        &self.finalization
    }

    /// Borrow the exact deployment-store database custody.
    pub fn genesis(&self) -> &DatabaseCommittedDeploymentControlStoreGenesis {
        &self.genesis
    }

    /// Borrow domain-anchor evidence in canonical checkpoint order.
    #[must_use]
    pub fn genesis_anchor_evidence(&self) -> &[AnchorAdvanceEvidence] {
        &self.genesis_anchor_evidence
    }
}

struct AnchorLeg {
    transition: PreparedAntiRollbackAnchorTransition,
    progress: AnchorProgress,
}

impl AnchorLeg {
    const fn new(
        transition: PreparedAntiRollbackAnchorTransition,
        progress: AnchorProgress,
    ) -> Self {
        Self {
            transition,
            progress,
        }
    }

    fn complete(
        &mut self,
        port: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<AnchorAdvanceEvidence, AnchorCompletionFailure> {
        self.progress.complete(port, &self.transition)
    }

    fn evidence(&self) -> Option<AnchorAdvanceEvidence> {
        match self.progress {
            AnchorProgress::Established(evidence) => Some(evidence),
            AnchorProgress::Ready | AnchorProgress::ObserveFirst(_) => None,
        }
    }
}

fn prepare_finalization(
    input: &SignedDeploymentFinalizationInput<'_>,
) -> Result<PreparedDeploymentFinalization, DeploymentFinalizationPreparationFailureReason> {
    let command = input.submission.command();
    let construction = input.submission.construction();
    let selected = construction.finalization().records();
    let command_coordinate = record_projection::coordinate(
        &input.command_ref,
        command.canonical_admin_command_sha256().as_str(),
    )?;
    let records = DeploymentFinalizationRecords::new(
        record_projection::record(command_coordinate.clone(), command)?,
        record_projection::record(
            record_projection::coordinate(
                selected.binding.0,
                selected.binding.1.canonical_binding_sha256().as_str(),
            )?,
            selected.binding.1,
        )?,
        record_projection::record(
            record_projection::coordinate(
                selected.checkpoint_signer.0,
                selected
                    .checkpoint_signer
                    .1
                    .canonical_checkpoint_signing_authority_sha256()
                    .as_str(),
            )?,
            selected.checkpoint_signer.1,
        )?,
        record_projection::record(
            record_projection::coordinate(
                selected.runtime_authority.0,
                selected
                    .runtime_authority
                    .1
                    .canonical_deployment_runtime_authority_sha256()
                    .as_str(),
            )?,
            selected.runtime_authority.1,
        )?,
        record_projection::record(
            record_projection::coordinate(
                selected.trust.successor_ref,
                selected
                    .trust
                    .successor
                    .canonical_trust_checkpoint_sha256()
                    .as_str(),
            )?,
            selected.trust.successor,
        )?,
        record_projection::record(
            record_projection::coordinate(
                construction.checkpoint().0,
                construction
                    .checkpoint()
                    .1
                    .canonical_control_checkpoint_sha256()
                    .as_str(),
            )?,
            construction.checkpoint().1,
        )?,
        record_projection::record_with_schema(
            record_projection::coordinate(
                &input.receipt_ref,
                input.receipt.canonical_admin_receipt_sha256().as_str(),
            )?,
            input.receipt.schema_id().clone(),
            input.receipt.schema_sha256().clone(),
            &input.receipt,
        )?,
    )?;
    let predecessor_trust = trust_head(selected.trust.predecessor_ref, selected.trust.predecessor)?;
    let successor_trust = trust_head(selected.trust.successor_ref, selected.trust.successor)?;
    let predecessor_control = match construction.accumulator().binding().phase() {
        AccumulatorCheckpointPhase::Successor(predecessor) => predecessor.clone(),
        AccumulatorCheckpointPhase::Initial => {
            return Err(DeploymentFinalizationPreparationFailureReason::ControlPredecessor);
        }
    };
    let successor_control = control_head(construction.checkpoint().0, construction.checkpoint().1)?;
    let atom = PreparedDeploymentFinalizationAtom::new(
        PrincipalTrustStoreBinding::new(
            selected.trust.predecessor.trust_store_id().clone(),
            selected
                .trust
                .predecessor
                .trust_store_incarnation_id()
                .clone(),
            selected.trust.predecessor.principal_id().clone(),
        ),
        DeploymentFinalizationReplayIdentity::new(
            command.admin_command_id().clone(),
            command.idempotency_key().clone(),
            record_projection::coordinate(
                command.admin_authority_ref(),
                command.admin_authority_sha256().as_str(),
            )?,
            command_coordinate,
            input.receipt.admin_receipt_id().clone(),
        ),
        records,
        predecessor_trust.clone(),
        successor_trust.clone(),
        predecessor_control.clone(),
        successor_control.clone(),
        construction.accumulator().clone(),
    )?;
    Ok(PreparedDeploymentFinalization { atom })
}

fn prepare_genesis(
    finalization: &AnchoredDeploymentFinalizationCapability,
    input: &SignedDeploymentGenesisInput,
) -> Result<
    (PreparedDeploymentControlStoreGenesisAtom, Vec<AnchorLeg>),
    DeploymentGenesisPreparationFailureReason,
> {
    verify_genesis_association(finalization, input)?;
    let mut checkpoint_records = Vec::with_capacity(input.checkpoints.len());
    let mut checkpoint_heads = Vec::with_capacity(input.checkpoints.len());
    let mut accumulators = Vec::with_capacity(input.checkpoints.len());
    let mut anchors = Vec::with_capacity(input.checkpoints.len());
    for checkpoint in &input.checkpoints {
        let coordinate = record_projection::coordinate(
            &checkpoint.record_ref,
            checkpoint
                .checkpoint
                .canonical_control_checkpoint_sha256()
                .as_str(),
        )?;
        let head = ControlCheckpointHead::new(
            checkpoint.checkpoint.checkpoint_domain_id().clone(),
            *checkpoint.checkpoint.domain_epoch(),
            *checkpoint.checkpoint.domain_sequence(),
            *checkpoint.checkpoint.committed_transaction_sequence(),
            coordinate.clone(),
        );
        anchors.push(AnchorLeg::new(
            PreparedAntiRollbackAnchorTransition::initial(AntiRollbackAnchor::from(
                control_anchor(input.binding.store(), &head),
            ))?,
            AnchorProgress::Ready,
        ));
        checkpoint_records.push(record_projection::record(
            coordinate,
            &checkpoint.checkpoint,
        )?);
        checkpoint_heads.push(head);
        accumulators.push(checkpoint.accumulator.clone());
    }
    let receipt_coordinate = record_projection::coordinate(
        &input.receipt_ref,
        input
            .receipt
            .canonical_deployment_control_store_genesis_receipt_sha256()
            .as_str(),
    )?;
    let records = DeploymentControlStoreGenesisRecords::new(
        checkpoint_records,
        record_projection::record(receipt_coordinate, &input.receipt)?,
    )?;
    let atom = PreparedDeploymentControlStoreGenesisAtom::new(
        finalization,
        input.binding.clone(),
        input.genesis_id.clone(),
        records,
        checkpoint_heads,
        accumulators,
    )?;
    Ok((atom, anchors))
}

fn verify_genesis_association(
    finalization: &AnchoredDeploymentFinalizationCapability,
    input: &SignedDeploymentGenesisInput,
) -> Result<(), DeploymentGenesisPreparationFailureReason> {
    let receipt = &input.receipt;
    if receipt.genesis_id() != &input.genesis_id
        || receipt.principal_id() != input.binding.principal_id()
        || receipt.workspace_id() != input.binding.workspace_id()
        || receipt.control_store_id() != input.binding.store().store_id()
        || receipt.control_incarnation_id() != input.binding.store().incarnation_id()
        || receipt.deployment_binding_ref() != finalization.binding().record_ref()
        || receipt.deployment_binding_sha256().as_str()
            != finalization.binding().record_sha256().as_str()
        || receipt.checkpoint_signing_authority_ref()
            != finalization.checkpoint_signing_authority().record_ref()
        || receipt.checkpoint_signing_authority_sha256().as_str()
            != finalization
                .checkpoint_signing_authority()
                .record_sha256()
                .as_str()
        || receipt.deployment_runtime_authority_ref()
            != finalization.runtime_authority().record_ref()
        || receipt.deployment_runtime_authority_sha256().as_str()
            != finalization.runtime_authority().record_sha256().as_str()
        || receipt.initial_control_checkpoints().len() != input.checkpoints.len()
        || receipt
            .initial_control_checkpoints()
            .iter()
            .zip(&input.checkpoints)
            .any(|(member, checkpoint)| {
                member.control_checkpoint_ref != checkpoint.record_ref
                    || member.control_checkpoint_sha256
                        != *checkpoint.checkpoint.canonical_control_checkpoint_sha256()
                    || member.checkpoint_domain_id != *checkpoint.checkpoint.checkpoint_domain_id()
                    || member.domain_epoch != *checkpoint.checkpoint.domain_epoch()
                    || member.domain_sequence != *checkpoint.checkpoint.domain_sequence()
            })
    {
        return Err(DeploymentGenesisPreparationFailureReason::Association);
    }
    Ok(())
}

fn trust_head(
    reference: &ResidentRecordRefV1,
    checkpoint: &PrincipalTrustCheckpointV1,
) -> Result<PrincipalTrustCheckpointHead, ControlRecordProjectionError> {
    Ok(PrincipalTrustCheckpointHead::new(
        checkpoint.principal_id().clone(),
        *checkpoint.checkpoint_epoch(),
        record_projection::coordinate(
            reference,
            checkpoint.canonical_trust_checkpoint_sha256().as_str(),
        )?,
    ))
}

fn control_head(
    reference: &ResidentRecordRefV1,
    checkpoint: &ControlStateCheckpointV1,
) -> Result<ControlCheckpointHead, ControlRecordProjectionError> {
    Ok(ControlCheckpointHead::new(
        checkpoint.checkpoint_domain_id().clone(),
        *checkpoint.domain_epoch(),
        *checkpoint.domain_sequence(),
        *checkpoint.committed_transaction_sequence(),
        record_projection::coordinate(
            reference,
            checkpoint.canonical_control_checkpoint_sha256().as_str(),
        )?,
    ))
}

fn control_anchor(
    store: &AuthorityStoreCoordinate,
    head: &ControlCheckpointHead,
) -> ControlDomainAntiRollbackAnchor {
    ControlDomainAntiRollbackAnchor::new(
        store.store_id().clone(),
        store.incarnation_id().clone(),
        head.checkpoint_domain_id().clone(),
        head.domain_epoch(),
        head.domain_sequence(),
        head.coordinate().clone(),
    )
}

fn progress_after_genesis_commit(
    evidence: DeploymentControlStoreGenesisCommitEvidence,
) -> AnchorProgress {
    match evidence {
        DeploymentControlStoreGenesisCommitEvidence::New => AnchorProgress::Ready,
        DeploymentControlStoreGenesisCommitEvidence::ExactReplay
        | DeploymentControlStoreGenesisCommitEvidence::RecoveredExactAfterAmbiguousCommit => {
            AnchorProgress::ObserveFirst(ObservationReason::DatabaseReplay)
        }
    }
}

/// Preparation failure that retains the authenticated transaction unchanged.
pub struct DeploymentFinalizationPreparationFailure<'a> {
    input: Box<SignedDeploymentFinalizationInput<'a>>,
    reason: DeploymentFinalizationPreparationFailureReason,
}

impl std::fmt::Debug for DeploymentFinalizationPreparationFailure<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DeploymentFinalizationPreparationFailure")
            .field("reason", &self.reason)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Display for DeploymentFinalizationPreparationFailure<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "deployment finalization could not be prepared: {}",
            self.reason
        )
    }
}

impl std::error::Error for DeploymentFinalizationPreparationFailure<'_> {}

impl<'a> DeploymentFinalizationPreparationFailure<'a> {
    /// Borrow the exact projection or candidate rejection.
    #[must_use]
    pub const fn reason(&self) -> &DeploymentFinalizationPreparationFailureReason {
        &self.reason
    }

    /// Recover the authenticated transaction for diagnosis or corrected preparation.
    pub fn into_input(self) -> SignedDeploymentFinalizationInput<'a> {
        *self.input
    }
}

/// Why the authenticated finalization could not form a physical candidate.
#[derive(Debug, thiserror::Error)]
pub enum DeploymentFinalizationPreparationFailureReason {
    /// Canonical signed-record encoding failed.
    #[error(transparent)]
    Canonicalize(#[from] canon::CanonError),
    /// A physical record coordinate or canonical byte string was malformed.
    #[error(transparent)]
    Record(#[from] ControlRecordError),
    /// The projected transaction contradicted its physical closure.
    #[error(transparent)]
    Candidate(#[from] DeploymentFinalizationCandidateError),
    /// The new control checkpoint did not name its exact predecessor.
    #[error("deployment finalization accumulator has no predecessor control checkpoint")]
    ControlPredecessor,
}

impl From<ControlRecordProjectionError> for DeploymentFinalizationPreparationFailureReason {
    fn from(error: ControlRecordProjectionError) -> Self {
        match error {
            ControlRecordProjectionError::Canonicalize(source) => Self::Canonicalize(source),
            ControlRecordProjectionError::Record(source) => Self::Record(source),
        }
    }
}

/// Principal-store failure that retains the complete prepared transaction.
pub struct DeploymentFinalizationCommitPhaseFailure {
    failure: DeploymentFinalizationCommitFailure,
}

impl std::fmt::Debug for DeploymentFinalizationCommitPhaseFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DeploymentFinalizationCommitPhaseFailure")
            .field("reason", self.failure.reason())
            .finish_non_exhaustive()
    }
}

impl std::fmt::Display for DeploymentFinalizationCommitPhaseFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "deployment finalization did not commit: {}",
            self.failure.reason()
        )
    }
}

impl std::error::Error for DeploymentFinalizationCommitPhaseFailure {}

impl DeploymentFinalizationCommitPhaseFailure {
    /// Borrow the exact custody or read-back failure.
    #[must_use]
    pub fn reason(&self) -> &pale_ports::DeploymentFinalizationCommitFailureReason {
        self.failure.reason()
    }

    /// Recover the prepared transaction for explicit retry or recovery.
    pub fn into_prepared(self) -> PreparedDeploymentFinalization {
        PreparedDeploymentFinalization {
            atom: self.failure.into_candidate(),
        }
    }
}

/// Principal-store anchor failure that retains all completed progress.
pub struct DeploymentFinalizationAnchorPhaseFailure {
    failure: DeploymentFinalizationAnchorFailure,
}

impl std::fmt::Debug for DeploymentFinalizationAnchorPhaseFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DeploymentFinalizationAnchorPhaseFailure")
            .field("owner", &self.failure.owner())
            .field("reason", self.failure.reason())
            .finish_non_exhaustive()
    }
}

impl std::fmt::Display for DeploymentFinalizationAnchorPhaseFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "deployment finalization anchor {:?} remains incomplete: {}",
            self.failure.owner(),
            self.failure.reason()
        )
    }
}

impl std::error::Error for DeploymentFinalizationAnchorPhaseFailure {}

impl DeploymentFinalizationAnchorPhaseFailure {
    /// Return the anchor owner that remained incomplete.
    #[must_use]
    pub const fn owner(&self) -> DeploymentFinalizationAnchorOwner {
        self.failure.owner()
    }

    /// Borrow the failed anchor observation or advance.
    #[must_use]
    pub const fn reason(&self) -> &pale_ports::DeploymentFinalizationAnchorFailureReason {
        self.failure.reason()
    }

    /// Recover the retained anchor progress for explicit continuation.
    pub fn into_pending(self) -> DatabaseCommittedDeploymentFinalizationAnchorsPending {
        DatabaseCommittedDeploymentFinalizationAnchorsPending {
            pending: self.failure.into_pending(),
        }
    }
}

/// Genesis preparation failure that retains the anchored finalization and input.
pub struct DeploymentGenesisPreparationFailure {
    finalization: Box<AnchoredDeploymentFinalization>,
    input: Box<SignedDeploymentGenesisInput>,
    reason: DeploymentGenesisPreparationFailureReason,
}

impl std::fmt::Debug for DeploymentGenesisPreparationFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DeploymentGenesisPreparationFailure")
            .field("reason", &self.reason)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Display for DeploymentGenesisPreparationFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "deployment genesis could not be prepared: {}",
            self.reason
        )
    }
}

impl std::error::Error for DeploymentGenesisPreparationFailure {}

impl DeploymentGenesisPreparationFailure {
    /// Borrow the exact association, projection, candidate, or anchor rejection.
    #[must_use]
    pub const fn reason(&self) -> &DeploymentGenesisPreparationFailureReason {
        &self.reason
    }

    /// Recover both inputs for diagnosis or corrected preparation.
    pub fn into_parts(self) -> (AnchoredDeploymentFinalization, SignedDeploymentGenesisInput) {
        (*self.finalization, *self.input)
    }
}

/// Why the signed genesis could not form the separate physical transaction.
#[derive(Debug, thiserror::Error)]
pub enum DeploymentGenesisPreparationFailureReason {
    /// The receipt, store binding, or checkpoints name unlike finalization evidence.
    #[error("signed genesis does not match anchored finalization and its exact checkpoints")]
    Association,
    /// Canonical signed-record encoding failed.
    #[error(transparent)]
    Canonicalize(#[from] canon::CanonError),
    /// A physical record coordinate or canonical byte string was malformed.
    #[error(transparent)]
    Record(#[from] ControlRecordError),
    /// The projected transaction contradicted its physical closure.
    #[error(transparent)]
    Candidate(#[from] DeploymentControlStoreGenesisCandidateError),
    /// One required independent anchor transition was invalid.
    #[error(transparent)]
    Anchor(#[from] AntiRollbackAnchorTransitionError),
}

impl From<ControlRecordProjectionError> for DeploymentGenesisPreparationFailureReason {
    fn from(error: ControlRecordProjectionError) -> Self {
        match error {
            ControlRecordProjectionError::Canonicalize(source) => Self::Canonicalize(source),
            ControlRecordProjectionError::Record(source) => Self::Record(source),
        }
    }
}

/// Deployment-store failure that retains the complete prepared genesis.
pub struct DeploymentGenesisCommitPhaseFailure {
    finalization: Box<AnchoredDeploymentFinalization>,
    failure: DeploymentControlStoreGenesisCommitFailure,
    anchors: Vec<AnchorLeg>,
}

impl std::fmt::Debug for DeploymentGenesisCommitPhaseFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DeploymentGenesisCommitPhaseFailure")
            .field("reason", self.failure.reason())
            .finish_non_exhaustive()
    }
}

impl std::fmt::Display for DeploymentGenesisCommitPhaseFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "deployment genesis did not commit: {}",
            self.failure.reason()
        )
    }
}

impl std::error::Error for DeploymentGenesisCommitPhaseFailure {}

impl DeploymentGenesisCommitPhaseFailure {
    /// Borrow the exact custody or read-back failure.
    #[must_use]
    pub fn reason(&self) -> &pale_ports::DeploymentControlStoreGenesisCommitFailureReason {
        self.failure.reason()
    }

    /// Recover the prepared genesis for explicit retry or recovery.
    pub fn into_prepared(self) -> PreparedDeploymentGenesis {
        PreparedDeploymentGenesis {
            finalization: *self.finalization,
            atom: self.failure.into_candidate(),
            anchors: self.anchors,
        }
    }
}

/// Deployment-store anchor failure that retains all completed progress.
pub struct DeploymentGenesisAnchorPhaseFailure {
    pending: Box<DeploymentGenesisAnchorsPending>,
    reason: Box<DeploymentGenesisAnchorPhaseFailureReason>,
}

impl std::fmt::Debug for DeploymentGenesisAnchorPhaseFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DeploymentGenesisAnchorPhaseFailure")
            .field("reason", &self.reason)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Display for DeploymentGenesisAnchorPhaseFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "deployment genesis anchor remains incomplete: {}",
            self.reason
        )
    }
}

impl std::error::Error for DeploymentGenesisAnchorPhaseFailure {}

impl DeploymentGenesisAnchorPhaseFailure {
    /// Borrow the exact anchor key that remained incomplete.
    #[must_use]
    pub fn owner(&self) -> &AntiRollbackAnchorKey {
        self.reason.owner()
    }

    /// Borrow the failed anchor observation or advance.
    #[must_use]
    pub fn reason(&self) -> &DeploymentGenesisAnchorPhaseFailureReason {
        &self.reason
    }

    /// Recover the retained anchor progress for explicit continuation.
    pub fn into_pending(self) -> DeploymentGenesisAnchorsPending {
        *self.pending
    }
}

/// Why one deployment-store domain anchor remains unproved.
#[derive(Debug, thiserror::Error)]
pub enum DeploymentGenesisAnchorPhaseFailureReason {
    /// The required read before resumption failed; no new CAS was attempted.
    #[error("deployment genesis anchor {anchor:?} could not be observed: {error}")]
    Observation {
        /// Retained anchor key, not caller-supplied replacement coordinates.
        anchor: AntiRollbackAnchorKey,
        /// Failure returned by the external mechanism.
        #[source]
        error: AntiRollbackAnchorError,
    },
    /// The anchor did not establish the exact retained successor.
    #[error("deployment genesis anchor {anchor:?} did not complete: {reason}")]
    Advance {
        /// Retained anchor key.
        anchor: AntiRollbackAnchorKey,
        /// The comparison and any immediate ambiguity observation.
        #[source]
        reason: AnchorAdvanceFailureReason,
    },
}

impl DeploymentGenesisAnchorPhaseFailureReason {
    const fn owner(&self) -> &AntiRollbackAnchorKey {
        match self {
            Self::Observation { anchor, .. } | Self::Advance { anchor, .. } => anchor,
        }
    }
}

#[cfg(test)]
#[path = "../../tests/internal/resident/deployment_initialization.rs"]
mod tests;
