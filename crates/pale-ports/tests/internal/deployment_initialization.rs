//! Behavioral tests for deployment finalization and control-store genesis.

use super::*;
use crate::{
    AccumulatorCheckpointBinding, AccumulatorMutation, AccumulatorPosition, AntiRollbackAnchorKey,
    ControlRecordBytes, ControlRecordDigest, ControlTransactionPreparation, StateAccumulatorBatch,
    UnresolvedCustodyAccumulatorBatch,
};
use pale_core::records::{
    ArtifactDigest, CheckpointProfileDigest, ControlCheckpointDomainIdV1, ControlDomainEpochV1,
    ControlDomainSequenceV1, ControlTransactionSequenceV1, PrincipalTrustCheckpointEpochV1,
    ResidentRecordRefV1, SchemaDigest, SchemaId, MAX_RESIDENT_SET_MEMBERS,
};
use std::collections::VecDeque;

fn hash(character: char) -> String {
    character.to_string().repeat(64)
}

fn coordinate(reference: &str, character: char) -> ControlRecordCoordinate {
    ControlRecordCoordinate::new(
        ResidentRecordRefV1::parse(reference).unwrap(),
        ControlRecordDigest::parse(hash(character)).unwrap(),
    )
}

fn record(schema: &str, reference: &str, character: char) -> ControlRecord {
    ControlRecord::new(
        coordinate(reference, character),
        SchemaId::parse(schema).unwrap(),
        SchemaDigest::parse(hash('f')).unwrap(),
        ControlRecordBytes::new(format!(r#"{{"record":"{reference}"}}"#).into_bytes()).unwrap(),
    )
}

fn checkpoint_head(
    domain: &str,
    sequence: u64,
    transaction: u64,
    reference: &str,
    character: char,
) -> ControlCheckpointHead {
    ControlCheckpointHead::new(
        ControlCheckpointDomainIdV1::parse(domain).unwrap(),
        ControlDomainEpochV1::new(1).unwrap(),
        ControlDomainSequenceV1::new(sequence).unwrap(),
        ControlTransactionSequenceV1::new(transaction).unwrap(),
        coordinate(reference, character),
    )
}

fn accumulator(
    store: AuthorityStoreCoordinate,
    phase: AccumulatorCheckpointPhase,
    successor: ControlCheckpointHead,
    expected_transaction: u64,
    character: u8,
) -> OperationAccumulatorBatch {
    let transaction = ControlTransactionPreparation::new(
        store.clone(),
        ControlTransactionSequenceV1::new(expected_transaction).unwrap(),
    )
    .unwrap();
    let binding = AccumulatorCheckpointBinding::new(
        store,
        ResidentRecordRefV1::parse("checkpoint-profile").unwrap(),
        CheckpointProfileDigest::parse(hash('e')).unwrap(),
        phase,
        successor,
        transaction,
    )
    .unwrap();
    let zero = ArtifactDigest::parse(hash('0')).unwrap();
    let state = StateAccumulatorBatch::new(
        zero.clone(),
        ArtifactDigest::parse(hash(char::from(character))).unwrap(),
        vec![AccumulatorMutation::new(
            AccumulatorPosition::Leaf([character; 32]),
            None,
            Some(vec![character; 32]),
            32,
        )
        .unwrap()],
        1,
    )
    .unwrap();
    let unresolved =
        UnresolvedCustodyAccumulatorBatch::new(zero.clone(), zero, Vec::new(), 1).unwrap();
    OperationAccumulatorBatch::new(binding, state, unresolved).unwrap()
}

fn finalization_records() -> DeploymentFinalizationRecords {
    DeploymentFinalizationRecords::new(
        record(LOCAL_ADMIN_COMMAND_V1_SCHEMA, "admin-command", '1'),
        record(DEPLOYMENT_BINDING_V1_SCHEMA, "deployment-binding", '2'),
        record(
            CHECKPOINT_SIGNING_AUTHORITY_V1_SCHEMA,
            "deployment-checkpoint-signer",
            '3',
        ),
        record(
            DEPLOYMENT_RUNTIME_AUTHORITY_V1_SCHEMA,
            "deployment-runtime-authority",
            '4',
        ),
        record(PRINCIPAL_TRUST_CHECKPOINT_V1_SCHEMA, "trust-successor", '5'),
        record(
            CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
            "principal-control-successor",
            '6',
        ),
        record(LOCAL_ADMIN_RECEIPT_V1_SCHEMA, "admin-receipt", '7'),
    )
    .unwrap()
}

fn finalization() -> PreparedDeploymentFinalizationAtom {
    let principal = ResidentPrincipalIdV1::parse("principal-1").unwrap();
    let trust_store = AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse("trust-store-1").unwrap(),
        ControlIncarnationIdV1::parse("trust-incarnation-1").unwrap(),
    );
    let records = finalization_records();
    let predecessor_control =
        checkpoint_head("principal-head", 1, 1, "principal-control-predecessor", '8');
    let successor_control =
        checkpoint_head("principal-head", 2, 2, "principal-control-successor", '6');
    let predecessor_trust = PrincipalTrustCheckpointHead::new(
        principal.clone(),
        PrincipalTrustCheckpointEpochV1::new(1).unwrap(),
        coordinate("trust-predecessor", '9'),
    );
    let successor_trust = PrincipalTrustCheckpointHead::new(
        principal.clone(),
        PrincipalTrustCheckpointEpochV1::new(2).unwrap(),
        coordinate("trust-successor", '5'),
    );
    let replay = DeploymentFinalizationReplayIdentity::new(
        LocalAdminCommandIdV1::parse("finalization-command").unwrap(),
        LocalAdminIdempotencyKeyV1::parse("finalization-replay").unwrap(),
        coordinate("admin-authority", 'a'),
        records.command().coordinate().clone(),
        LocalAdminReceiptIdV1::parse("finalization-receipt").unwrap(),
    );
    let batch = accumulator(
        trust_store.clone(),
        AccumulatorCheckpointPhase::Successor(predecessor_control.clone()),
        successor_control.clone(),
        1,
        b'b',
    );
    PreparedDeploymentFinalizationAtom::new(
        PrincipalTrustStoreBinding::new(
            trust_store.store_id().clone(),
            trust_store.incarnation_id().clone(),
            principal,
        ),
        replay,
        records,
        predecessor_trust,
        successor_trust,
        predecessor_control,
        successor_control,
        batch,
    )
    .unwrap()
}

fn genesis(
    finalization: &AnchoredDeploymentFinalizationCapability,
) -> PreparedDeploymentControlStoreGenesisAtom {
    let store = AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse("deployment-store-1").unwrap(),
        ControlIncarnationIdV1::parse("deployment-incarnation-1").unwrap(),
    );
    let binding = DeploymentControlStoreBinding::new(
        store.store_id().clone(),
        store.incarnation_id().clone(),
        finalization.principal_id().clone(),
        WorkspaceId::parse("workspace-1").unwrap(),
        finalization.binding().clone(),
        finalization.checkpoint_signing_authority().clone(),
        finalization.runtime_authority().clone(),
    );
    let head = checkpoint_head("deployment-head", 1, 2, "deployment-control-genesis", 'c');
    let records = DeploymentControlStoreGenesisRecords::new(
        vec![record(
            CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
            "deployment-control-genesis",
            'c',
        )],
        record(
            DEPLOYMENT_CONTROL_STORE_GENESIS_RECEIPT_V1_SCHEMA,
            "deployment-genesis-receipt",
            'd',
        ),
    )
    .unwrap();
    let batch = accumulator(
        store,
        AccumulatorCheckpointPhase::Initial,
        head.clone(),
        1,
        b'c',
    );
    PreparedDeploymentControlStoreGenesisAtom::new(
        finalization,
        binding,
        DeploymentControlStoreGenesisIdV1::parse("deployment-genesis-1").unwrap(),
        records,
        vec![head],
        vec![batch],
    )
    .unwrap()
}

#[derive(Clone, Copy)]
enum FinalizationCommitMode {
    New,
    Replay,
    AmbiguousExact,
}

struct FinalizationStore {
    mode: FinalizationCommitMode,
    compares: usize,
    observations: usize,
}

impl FinalizationStore {
    const fn new(mode: FinalizationCommitMode) -> Self {
        Self {
            mode,
            compares: 0,
            observations: 0,
        }
    }
}

impl DeploymentFinalizationCustodyPort for FinalizationStore {
    fn compare_and_commit(
        &mut self,
        _: &PreparedDeploymentFinalizationAtom,
    ) -> Result<DeploymentFinalizationCommitDisposition, ControlStoreError> {
        self.compares += 1;
        match self.mode {
            FinalizationCommitMode::New => Ok(DeploymentFinalizationCommitDisposition::New),
            FinalizationCommitMode::Replay => {
                Ok(DeploymentFinalizationCommitDisposition::ExactReplay)
            }
            FinalizationCommitMode::AmbiguousExact => Err(ControlStoreError::new(
                ControlStoreErrorKind::OutcomeUnknown,
            )),
        }
    }

    fn observe_exact(
        &mut self,
        _: &PreparedDeploymentFinalizationAtom,
    ) -> Result<DeploymentFinalizationDatabaseObservation, ControlStoreError> {
        self.observations += 1;
        Ok(DeploymentFinalizationDatabaseObservation::Exact)
    }
}

enum AnchorCompareStep {
    Advanced,
    OutcomeUnknown,
    Unavailable,
}

enum AnchorObservationStep {
    Successor,
    Predecessor,
    Missing,
    MisclassifiedPredecessorAsConflict,
}

#[derive(Default)]
struct ScriptedAnchors {
    compares: Vec<AntiRollbackAnchorKey>,
    observations: Vec<AntiRollbackAnchorKey>,
    compare_steps: VecDeque<AnchorCompareStep>,
    observation_steps: VecDeque<AnchorObservationStep>,
}

impl ScriptedAnchors {
    fn new(
        compare_steps: impl IntoIterator<Item = AnchorCompareStep>,
        observation_steps: impl IntoIterator<Item = AnchorObservationStep>,
    ) -> Self {
        Self {
            compare_steps: compare_steps.into_iter().collect(),
            observation_steps: observation_steps.into_iter().collect(),
            ..Self::default()
        }
    }
}

impl AntiRollbackAnchorPort for ScriptedAnchors {
    fn read(
        &mut self,
        _: &AntiRollbackAnchorKey,
    ) -> Result<Option<AntiRollbackAnchor>, AntiRollbackAnchorError> {
        panic!("finalization anchor completion never performs an unclassified read")
    }

    fn compare_and_advance(
        &mut self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError> {
        self.compares.push(transition.key().clone());
        match self.compare_steps.pop_front().expect("scripted comparison") {
            AnchorCompareStep::Advanced => Ok(AntiRollbackAnchorAdvanceDisposition::Advanced),
            AnchorCompareStep::OutcomeUnknown => Err(AntiRollbackAnchorError::new(
                AntiRollbackAnchorErrorKind::OutcomeUnknown,
            )),
            AnchorCompareStep::Unavailable => Err(AntiRollbackAnchorError::new(
                AntiRollbackAnchorErrorKind::Unavailable,
            )),
        }
    }

    fn observe(
        &mut self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError> {
        self.observations.push(transition.key().clone());
        Ok(
            match self
                .observation_steps
                .pop_front()
                .expect("scripted observation")
            {
                AnchorObservationStep::Successor => AntiRollbackAnchorObservation::ExactSuccessor,
                AnchorObservationStep::Predecessor => {
                    AntiRollbackAnchorObservation::ExactPredecessor
                }
                AnchorObservationStep::Missing => AntiRollbackAnchorObservation::Missing,
                AnchorObservationStep::MisclassifiedPredecessorAsConflict => {
                    AntiRollbackAnchorObservation::Conflicting {
                        observed: transition.expected().expect("successor transition").clone(),
                    }
                }
            },
        )
    }
}

fn anchored_finalization() -> AnchoredDeploymentFinalizationCapability {
    let committed = finalization()
        .commit(&mut FinalizationStore::new(FinalizationCommitMode::New))
        .unwrap();
    committed
        .anchors_pending()
        .complete(&mut ScriptedAnchors::new(
            [AnchorCompareStep::Advanced, AnchorCompareStep::Advanced],
            [],
        ))
        .unwrap()
}

#[test]
fn finalization_and_genesis_are_distinct_ordered_atoms() {
    let prepared = finalization();
    let finalization = anchored_finalization();
    let genesis = genesis(&finalization);

    assert_ne!(
        prepared.accumulator().binding().store(),
        genesis.binding().store()
    );
    assert_eq!(
        genesis.finalization_trust_store(),
        finalization.trust_store()
    );
    assert_eq!(
        prepared.records().receipt().schema_id().as_str(),
        LOCAL_ADMIN_RECEIPT_V1_SCHEMA
    );
    assert_eq!(
        genesis.records().receipt().schema_id().as_str(),
        DEPLOYMENT_CONTROL_STORE_GENESIS_RECEIPT_V1_SCHEMA
    );
    assert!(genesis
        .records()
        .checkpoints()
        .iter()
        .all(|record| record.schema_id().as_str() == CONTROL_STATE_CHECKPOINT_V1_SCHEMA));
}

#[test]
fn finalization_rejects_a_genesis_receipt_in_transaction_one() {
    let error = DeploymentFinalizationRecords::new(
        record(LOCAL_ADMIN_COMMAND_V1_SCHEMA, "admin-command", '1'),
        record(DEPLOYMENT_BINDING_V1_SCHEMA, "deployment-binding", '2'),
        record(
            CHECKPOINT_SIGNING_AUTHORITY_V1_SCHEMA,
            "deployment-checkpoint-signer",
            '3',
        ),
        record(
            DEPLOYMENT_RUNTIME_AUTHORITY_V1_SCHEMA,
            "deployment-runtime-authority",
            '4',
        ),
        record(PRINCIPAL_TRUST_CHECKPOINT_V1_SCHEMA, "trust", '5'),
        record(CONTROL_STATE_CHECKPOINT_V1_SCHEMA, "control", '6'),
        record(
            DEPLOYMENT_CONTROL_STORE_GENESIS_RECEIPT_V1_SCHEMA,
            "wrong-receipt",
            '7',
        ),
    )
    .unwrap_err();
    assert_eq!(error, DeploymentFinalizationCandidateError::RecordFamily);
}

#[test]
fn genesis_rejects_an_administration_receipt_in_transaction_two() {
    let error = DeploymentControlStoreGenesisRecords::new(
        vec![record(CONTROL_STATE_CHECKPOINT_V1_SCHEMA, "control", '1')],
        record(LOCAL_ADMIN_RECEIPT_V1_SCHEMA, "wrong-receipt", '2'),
    )
    .unwrap_err();
    assert_eq!(
        error,
        DeploymentControlStoreGenesisCandidateError::RecordFamily
    );
}

#[test]
fn genesis_rejects_the_principal_trust_store_as_its_store() {
    let finalization = anchored_finalization();
    let binding = DeploymentControlStoreBinding::new(
        finalization.trust_store().store_id().clone(),
        finalization.trust_store().incarnation_id().clone(),
        finalization.principal_id().clone(),
        WorkspaceId::parse("workspace-1").unwrap(),
        finalization.binding().clone(),
        finalization.checkpoint_signing_authority().clone(),
        finalization.runtime_authority().clone(),
    );
    let head = checkpoint_head("deployment-head", 1, 2, "control", '1');
    let records = DeploymentControlStoreGenesisRecords::new(
        vec![record(CONTROL_STATE_CHECKPOINT_V1_SCHEMA, "control", '1')],
        record(
            DEPLOYMENT_CONTROL_STORE_GENESIS_RECEIPT_V1_SCHEMA,
            "genesis-receipt",
            '2',
        ),
    )
    .unwrap();
    let accumulator = accumulator(
        finalization.trust_store().clone(),
        AccumulatorCheckpointPhase::Initial,
        head.clone(),
        1,
        b'1',
    );
    let error = PreparedDeploymentControlStoreGenesisAtom::new(
        &finalization,
        binding,
        DeploymentControlStoreGenesisIdV1::parse("genesis-1").unwrap(),
        records,
        vec![head],
        vec![accumulator],
    )
    .unwrap_err();
    assert_eq!(
        error,
        DeploymentControlStoreGenesisCandidateError::StoreSeparation
    );
}

#[test]
fn replay_identity_is_part_of_exact_finalization_equality() {
    let first = finalization();
    let exact = first.clone();
    assert_eq!(first, exact);
    assert_eq!(
        first.replay().idempotency_key().as_str(),
        "finalization-replay"
    );
    assert_eq!(first.replay().receipt_id().as_str(), "finalization-receipt");
}

struct AmbiguousGenesisStore {
    compares: usize,
    observations: usize,
}

impl DeploymentControlStoreGenesisCustodyPort for AmbiguousGenesisStore {
    fn compare_and_commit(
        &mut self,
        _candidate: &PreparedDeploymentControlStoreGenesisAtom,
    ) -> Result<DeploymentControlStoreGenesisCommitDisposition, ControlStoreError> {
        self.compares += 1;
        Err(ControlStoreError::new(
            ControlStoreErrorKind::OutcomeUnknown,
        ))
    }

    fn observe_exact(
        &mut self,
        _candidate: &PreparedDeploymentControlStoreGenesisAtom,
    ) -> Result<DeploymentControlStoreGenesisDatabaseObservation, ControlStoreError> {
        self.observations += 1;
        Ok(DeploymentControlStoreGenesisDatabaseObservation::Exact)
    }
}

#[test]
fn ambiguous_finalization_is_read_back_once_without_retry() {
    let mut store = FinalizationStore::new(FinalizationCommitMode::AmbiguousExact);
    let committed = finalization().commit(&mut store).unwrap();
    assert_eq!(store.compares, 1);
    assert_eq!(store.observations, 1);
    assert_eq!(
        committed.evidence(),
        DeploymentFinalizationCommitEvidence::RecoveredExactAfterAmbiguousCommit
    );
    let mut anchors = ScriptedAnchors::new(
        [],
        [
            AnchorObservationStep::Successor,
            AnchorObservationStep::Successor,
        ],
    );
    let capability = committed.anchors_pending().complete(&mut anchors).unwrap();
    assert!(anchors.compares.is_empty());
    assert_eq!(anchors.observations.len(), 2);
    assert_eq!(
        capability.database_commit_evidence(),
        DeploymentFinalizationCommitEvidence::RecoveredExactAfterAmbiguousCommit
    );
    assert_eq!(
        capability.trust_anchor_evidence(),
        DeploymentFinalizationAnchorEvidence::ExactReplay
    );
}

#[test]
fn ambiguous_genesis_is_read_back_once_without_retry() {
    let finalization = anchored_finalization();
    let mut store = AmbiguousGenesisStore {
        compares: 0,
        observations: 0,
    };
    let committed = genesis(&finalization).commit(&mut store).unwrap();
    assert_eq!(store.compares, 1);
    assert_eq!(store.observations, 1);
    assert_eq!(
        committed.evidence(),
        DeploymentControlStoreGenesisCommitEvidence::RecoveredExactAfterAmbiguousCommit
    );
    assert_eq!(
        committed.candidate().finalization_trust_store(),
        finalization.trust_store()
    );
}

#[test]
fn second_anchor_failure_retains_first_anchor_and_mints_no_capability() {
    let committed = finalization()
        .commit(&mut FinalizationStore::new(FinalizationCommitMode::New))
        .unwrap();
    let mut anchors = ScriptedAnchors::new(
        [
            AnchorCompareStep::Advanced,
            AnchorCompareStep::Unavailable,
            AnchorCompareStep::Advanced,
        ],
        [AnchorObservationStep::Predecessor],
    );
    let failure = committed
        .anchors_pending()
        .complete(&mut anchors)
        .unwrap_err();
    assert_eq!(
        failure.owner(),
        DeploymentFinalizationAnchorOwner::PrincipalControlDomain
    );
    assert!(matches!(
        failure.reason(),
        DeploymentFinalizationAnchorFailureReason::Port(error)
            if error.kind() == AntiRollbackAnchorErrorKind::Unavailable
    ));
    let capability = failure.into_pending().complete(&mut anchors).unwrap();
    assert_eq!(anchors.compares.len(), 3);
    assert_ne!(anchors.compares[0], anchors.compares[1]);
    assert_eq!(anchors.compares[1], anchors.compares[2]);
    assert_eq!(anchors.observations, vec![anchors.compares[1].clone()]);
    assert_eq!(
        capability.trust_anchor_evidence(),
        DeploymentFinalizationAnchorEvidence::Advanced
    );
}

#[test]
fn replay_rejects_missing_or_mismatched_anchor_observations_without_comparing() {
    for observation in [
        AnchorObservationStep::Missing,
        AnchorObservationStep::MisclassifiedPredecessorAsConflict,
    ] {
        let committed = finalization()
            .commit(&mut FinalizationStore::new(FinalizationCommitMode::Replay))
            .unwrap();
        let mut anchors = ScriptedAnchors::new([], [observation]);
        let failure = committed
            .anchors_pending()
            .complete(&mut anchors)
            .unwrap_err();
        assert_eq!(
            failure.owner(),
            DeploymentFinalizationAnchorOwner::PrincipalTrustMinimum
        );
        assert!(matches!(
            failure.reason(),
            DeploymentFinalizationAnchorFailureReason::NotAdvanced(_)
        ));
        assert!(anchors.compares.is_empty());
        assert_eq!(anchors.observations.len(), 1);
    }
}

#[test]
fn exact_replay_observes_both_successors_before_minting_capability() {
    let committed = finalization()
        .commit(&mut FinalizationStore::new(FinalizationCommitMode::Replay))
        .unwrap();
    let mut anchors = ScriptedAnchors::new(
        [],
        [
            AnchorObservationStep::Successor,
            AnchorObservationStep::Successor,
        ],
    );
    let capability = committed.anchors_pending().complete(&mut anchors).unwrap();
    assert!(anchors.compares.is_empty());
    assert_eq!(anchors.observations.len(), 2);
    assert_eq!(
        capability.trust_anchor_evidence(),
        DeploymentFinalizationAnchorEvidence::ExactReplay
    );
    assert_eq!(
        capability.control_anchor_evidence(),
        DeploymentFinalizationAnchorEvidence::ExactReplay
    );
}

#[test]
fn ambiguous_anchor_requires_exact_successor_observation() {
    let committed = finalization()
        .commit(&mut FinalizationStore::new(FinalizationCommitMode::New))
        .unwrap();
    let mut anchors = ScriptedAnchors::new(
        [
            AnchorCompareStep::OutcomeUnknown,
            AnchorCompareStep::Advanced,
        ],
        [AnchorObservationStep::Successor],
    );
    let capability = committed.anchors_pending().complete(&mut anchors).unwrap();
    assert_eq!(anchors.compares.len(), 2);
    assert_eq!(anchors.observations.len(), 1);
    assert_eq!(
        capability.trust_anchor_evidence(),
        DeploymentFinalizationAnchorEvidence::RecoveredExactAfterAmbiguousAdvance
    );
    assert_eq!(
        capability.control_anchor_evidence(),
        DeploymentFinalizationAnchorEvidence::Advanced
    );
}

#[test]
fn genesis_records_reject_more_than_the_admitted_checkpoint_bound() {
    let checkpoint = record(CONTROL_STATE_CHECKPOINT_V1_SCHEMA, "control", '1');
    let error = DeploymentControlStoreGenesisRecords::new(
        vec![checkpoint; MAX_RESIDENT_SET_MEMBERS + 1],
        record(
            DEPLOYMENT_CONTROL_STORE_GENESIS_RECEIPT_V1_SCHEMA,
            "receipt",
            '2',
        ),
    )
    .unwrap_err();
    assert_eq!(
        error,
        DeploymentControlStoreGenesisCandidateError::CheckpointBound
    );
}
