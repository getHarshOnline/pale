//! Bounded physical evidence needed before resident admission can begin.
//!
//! These snapshots retain store bindings, hard-frontier heads, and exact
//! opaque records from one native read transaction. They do not decode signed
//! records, establish trust, compare anti-rollback anchors, or make a resident
//! ready.

use pale_core::records::{
    ArtifactDigest, ControlTransactionSequenceV1, DeploymentControlStoreGenesisIdV1,
};

use crate::{
    AuthorityStoreCoordinate, ControlCheckpointHead, ControlRecord, ControlStoreError,
    DeploymentControlStoreBinding, DeploymentControlStoreGenesisRecords,
    DeploymentFinalizationRecords, DeploymentFinalizationReplayIdentity, OperationAccumulatorBatch,
    PrincipalResidentWriterHead, PrincipalTrustCheckpointHead, PrincipalTrustStoreBinding,
};

/// Hard copy limits for one principal's physical finalization set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeploymentFinalizationSetReadLimits {
    maximum_finalizations: usize,
    maximum_record_bytes: usize,
    maximum_mutations: usize,
}

impl DeploymentFinalizationSetReadLimits {
    /// Set finalization-count, aggregate record-body, and aggregate mutation ceilings.
    #[must_use]
    pub const fn new(
        maximum_finalizations: usize,
        maximum_record_bytes: usize,
        maximum_mutations: usize,
    ) -> Self {
        Self {
            maximum_finalizations,
            maximum_record_bytes,
            maximum_mutations,
        }
    }

    /// Return the maximum number of copied finalization closures.
    #[must_use]
    pub const fn maximum_finalizations(self) -> usize {
        self.maximum_finalizations
    }

    /// Return the maximum combined bytes copied from immutable records.
    #[must_use]
    pub const fn maximum_record_bytes(self) -> usize {
        self.maximum_record_bytes
    }

    /// Return the maximum combined accumulator rows copied from all closures.
    #[must_use]
    pub const fn maximum_mutations(self) -> usize {
        self.maximum_mutations
    }
}

/// Physical evidence retained by one deployment-finalization transaction.
///
/// The snapshot preserves the exact closure needed for later record and
/// signature admission. It does not decode those records, establish either
/// anti-rollback anchor, or make a deployment usable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeploymentFinalizationSnapshot {
    binding: PrincipalTrustStoreBinding,
    replay: DeploymentFinalizationReplayIdentity,
    admin_authority_record: ControlRecord,
    records: DeploymentFinalizationRecords,
    predecessor_trust: PrincipalTrustCheckpointHead,
    predecessor_trust_record: ControlRecord,
    successor_trust: PrincipalTrustCheckpointHead,
    predecessor_control: ControlCheckpointHead,
    predecessor_control_record: ControlRecord,
    successor_control: ControlCheckpointHead,
    accumulator: OperationAccumulatorBatch,
    committed_transaction: ControlTransactionSequenceV1,
    candidate_identity_sha256: ArtifactDigest,
}

impl DeploymentFinalizationSnapshot {
    /// Bind one stored close row to every record, head and tree mutation it names.
    ///
    /// # Errors
    ///
    /// Rejects substituted authority, command, predecessor or successor records,
    /// a non-successor frontier, or an accumulator outside this transaction.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        binding: PrincipalTrustStoreBinding,
        replay: DeploymentFinalizationReplayIdentity,
        admin_authority_record: ControlRecord,
        records: DeploymentFinalizationRecords,
        predecessor_trust: PrincipalTrustCheckpointHead,
        predecessor_trust_record: ControlRecord,
        successor_trust: PrincipalTrustCheckpointHead,
        predecessor_control: ControlCheckpointHead,
        predecessor_control_record: ControlRecord,
        successor_control: ControlCheckpointHead,
        accumulator: OperationAccumulatorBatch,
        committed_transaction: ControlTransactionSequenceV1,
        candidate_identity_sha256: ArtifactDigest,
    ) -> Result<Self, DeploymentFinalizationSnapshotError> {
        if admin_authority_record.coordinate() != replay.admin_authority()
            || records.command().coordinate() != replay.command()
        {
            return Err(DeploymentFinalizationSnapshotError::ReplayRecords);
        }
        if binding.principal_id() != predecessor_trust.principal_id()
            || binding.principal_id() != successor_trust.principal_id()
            || !successor_trust.is_exact_successor_of(&predecessor_trust)
            || predecessor_trust_record.coordinate() != predecessor_trust.coordinate()
            || records.trust_checkpoint().coordinate() != successor_trust.coordinate()
        {
            return Err(DeploymentFinalizationSnapshotError::TrustFrontier);
        }
        if !successor_control.is_exact_successor_of(&predecessor_control)
            || predecessor_control_record.coordinate() != predecessor_control.coordinate()
            || records.control_checkpoint().coordinate() != successor_control.coordinate()
        {
            return Err(DeploymentFinalizationSnapshotError::ControlFrontier);
        }
        let accumulator_binding = accumulator.binding();
        if accumulator_binding.store().store_id() != binding.trust_store_id()
            || accumulator_binding.store().incarnation_id() != binding.trust_store_incarnation_id()
            || accumulator_binding.successor() != &successor_control
            || accumulator_binding.transaction().successor() != committed_transaction
            || !matches!(
                accumulator_binding.phase(),
                crate::AccumulatorCheckpointPhase::Successor(predecessor)
                    if predecessor == &predecessor_control
            )
        {
            return Err(DeploymentFinalizationSnapshotError::Accumulator);
        }
        Ok(Self {
            binding,
            replay,
            admin_authority_record,
            records,
            predecessor_trust,
            predecessor_trust_record,
            successor_trust,
            predecessor_control,
            predecessor_control_record,
            successor_control,
            accumulator,
            committed_transaction,
            candidate_identity_sha256,
        })
    }

    /// Borrow the principal and exact trust-store incarnation.
    #[must_use]
    pub const fn binding(&self) -> &PrincipalTrustStoreBinding {
        &self.binding
    }

    /// Borrow the authority-scoped administration replay identity.
    #[must_use]
    pub const fn replay(&self) -> &DeploymentFinalizationReplayIdentity {
        &self.replay
    }

    /// Borrow the immutable administrator-authority record named by the replay key.
    #[must_use]
    pub const fn admin_authority_record(&self) -> &ControlRecord {
        &self.admin_authority_record
    }

    /// Borrow the seven records committed by finalization.
    #[must_use]
    pub const fn records(&self) -> &DeploymentFinalizationRecords {
        &self.records
    }

    /// Borrow the trust head present before this finalization.
    #[must_use]
    pub const fn predecessor_trust(&self) -> &PrincipalTrustCheckpointHead {
        &self.predecessor_trust
    }

    /// Borrow the immutable record for the predecessor trust head.
    #[must_use]
    pub const fn predecessor_trust_record(&self) -> &ControlRecord {
        &self.predecessor_trust_record
    }

    /// Borrow the trust head committed by this finalization.
    #[must_use]
    pub const fn successor_trust(&self) -> &PrincipalTrustCheckpointHead {
        &self.successor_trust
    }

    /// Borrow the principal control head present before this finalization.
    #[must_use]
    pub const fn predecessor_control(&self) -> &ControlCheckpointHead {
        &self.predecessor_control
    }

    /// Borrow the immutable record for the predecessor principal control head.
    #[must_use]
    pub const fn predecessor_control_record(&self) -> &ControlRecord {
        &self.predecessor_control_record
    }

    /// Borrow the principal control head committed by this finalization.
    #[must_use]
    pub const fn successor_control(&self) -> &ControlCheckpointHead {
        &self.successor_control
    }

    /// Borrow the exact physical accumulator changes committed by this finalization.
    #[must_use]
    pub const fn accumulator(&self) -> &OperationAccumulatorBatch {
        &self.accumulator
    }

    /// Return the principal-store transaction that retained this closure.
    #[must_use]
    pub const fn committed_transaction(&self) -> ControlTransactionSequenceV1 {
        self.committed_transaction
    }

    /// Borrow the stored identity of the complete finalization candidate.
    #[must_use]
    pub const fn candidate_identity_sha256(&self) -> &ArtifactDigest {
        &self.candidate_identity_sha256
    }
}

/// One physical finalization snapshot does not close exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DeploymentFinalizationSnapshotError {
    /// The replay identity does not name its retained authority or command record.
    #[error("deployment finalization replay records do not match")]
    ReplayRecords,
    /// The trust heads or their retained records do not form one succession.
    #[error("deployment finalization trust frontier does not match")]
    TrustFrontier,
    /// The principal control heads or their records do not form one succession.
    #[error("deployment finalization control frontier does not match")]
    ControlFrontier,
    /// The accumulator belongs to another store, transaction or checkpoint pair.
    #[error("deployment finalization accumulator does not match")]
    Accumulator,
}

/// Complete physical finalization closures observed in one principal-store snapshot.
///
/// Entries are ordered by their distinct committed transactions. This value
/// preserves opaque evidence only. It does not select a current deployment,
/// decode a signed record, establish an anchor, or make a resident ready.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeploymentFinalizationSetSnapshot {
    binding: PrincipalTrustStoreBinding,
    observed_through: ControlTransactionSequenceV1,
    finalizations: Box<[DeploymentFinalizationSnapshot]>,
}

impl DeploymentFinalizationSetSnapshot {
    /// Bind a non-empty, transaction-ordered set to one principal-store snapshot.
    ///
    /// # Errors
    ///
    /// Rejects another store binding, an empty set, a future transaction, or
    /// equal or reversed transaction ownership.
    pub fn new(
        binding: PrincipalTrustStoreBinding,
        observed_through: ControlTransactionSequenceV1,
        finalizations: Box<[DeploymentFinalizationSnapshot]>,
    ) -> Result<Self, DeploymentFinalizationSetSnapshotError> {
        if finalizations.is_empty() {
            return Err(DeploymentFinalizationSetSnapshotError::Empty);
        }
        let mut previous = None;
        for finalization in &finalizations {
            if finalization.binding() != &binding {
                return Err(DeploymentFinalizationSetSnapshotError::StoreBinding);
            }
            let transaction = finalization.committed_transaction();
            if transaction > observed_through {
                return Err(DeploymentFinalizationSetSnapshotError::FutureTransaction);
            }
            if previous.is_some_and(|prior| transaction <= prior) {
                return Err(DeploymentFinalizationSetSnapshotError::TransactionOrder);
            }
            previous = Some(transaction);
        }
        Ok(Self {
            binding,
            observed_through,
            finalizations,
        })
    }

    /// Borrow the principal and exact trust-store incarnation.
    #[must_use]
    pub const fn binding(&self) -> &PrincipalTrustStoreBinding {
        &self.binding
    }

    /// Return the store transaction observed by the native read snapshot.
    #[must_use]
    pub const fn observed_through(&self) -> ControlTransactionSequenceV1 {
        self.observed_through
    }

    /// Borrow every copied closure in committed transaction order.
    #[must_use]
    pub const fn finalizations(&self) -> &[DeploymentFinalizationSnapshot] {
        &self.finalizations
    }
}

/// A physical finalization set is not coherent around one store snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DeploymentFinalizationSetSnapshotError {
    /// No finalization closure was supplied; absence is represented by `None`.
    #[error("deployment finalization set is empty")]
    Empty,
    /// One closure belongs to another principal-store binding.
    #[error("deployment finalization set contains another store binding")]
    StoreBinding,
    /// One closure claims commitment after the observed store transaction.
    #[error("deployment finalization set contains a future transaction")]
    FutureTransaction,
    /// Committed transactions are equal or not strictly increasing.
    #[error("deployment finalization set is not in strict transaction order")]
    TransactionOrder,
}

/// Read-only physical discovery of a principal's deployment finalizations.
pub trait DeploymentFinalizationSetReadPort {
    /// Read every complete historical closure in one native read snapshot.
    ///
    /// `None` means no finalization close or finalization-specific material is
    /// retained. The result does not admit a record, signer, anchor or runtime,
    /// and its order does not select a current deployment.
    ///
    /// # Errors
    ///
    /// Returns a control-store error for partial, surplus, conflicting or
    /// wrongly bound evidence, an exceeded copy limit, or a native read failure.
    fn read_finalization_set(
        &mut self,
        binding: &PrincipalTrustStoreBinding,
        limits: DeploymentFinalizationSetReadLimits,
    ) -> Result<Option<DeploymentFinalizationSetSnapshot>, ControlStoreError>;
}

/// Current resident-writer head and both immutable records that close it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrincipalResidentWriterSnapshot {
    binding: PrincipalTrustStoreBinding,
    head: PrincipalResidentWriterHead,
    writer_record: ControlRecord,
    checkpoint_record: ControlRecord,
}

impl PrincipalResidentWriterSnapshot {
    /// Bind one physical writer head to its exact writer and checkpoint bytes.
    ///
    /// # Errors
    ///
    /// Rejects a different principal or either record-coordinate substitution.
    pub fn new(
        binding: PrincipalTrustStoreBinding,
        head: PrincipalResidentWriterHead,
        writer_record: ControlRecord,
        checkpoint_record: ControlRecord,
    ) -> Result<Self, PrincipalResidentWriterSnapshotError> {
        if binding.principal_id() != head.principal_id() {
            return Err(PrincipalResidentWriterSnapshotError::PrincipalBinding);
        }
        if writer_record.coordinate() != head.coordinate() {
            return Err(PrincipalResidentWriterSnapshotError::WriterRecord);
        }
        if checkpoint_record.coordinate() != head.checkpoint().coordinate() {
            return Err(PrincipalResidentWriterSnapshotError::CheckpointRecord);
        }
        Ok(Self {
            binding,
            head,
            writer_record,
            checkpoint_record,
        })
    }

    /// Borrow the exact trust-store incarnation and principal binding.
    #[must_use]
    pub const fn binding(&self) -> &PrincipalTrustStoreBinding {
        &self.binding
    }

    /// Borrow the current physical resident-writer head.
    #[must_use]
    pub const fn head(&self) -> &PrincipalResidentWriterHead {
        &self.head
    }

    /// Borrow the opaque writer-fence record named by the head.
    #[must_use]
    pub const fn writer_record(&self) -> &ControlRecord {
        &self.writer_record
    }

    /// Borrow the opaque control-checkpoint record named by the head.
    #[must_use]
    pub const fn checkpoint_record(&self) -> &ControlRecord {
        &self.checkpoint_record
    }
}

/// A physical writer snapshot contradicts its supplied coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PrincipalResidentWriterSnapshotError {
    /// The writer head belongs to another principal.
    #[error("resident-writer head does not match the principal binding")]
    PrincipalBinding,
    /// The writer record does not reproduce the writer-head coordinate.
    #[error("resident-writer record does not match its head")]
    WriterRecord,
    /// The checkpoint record does not reproduce the associated checkpoint head.
    #[error("resident-writer checkpoint record does not match its head")]
    CheckpointRecord,
}

/// Read-only physical discovery of the current principal resident writer.
pub trait PrincipalResidentWriterReadPort {
    /// Read the current writer and both closing records in one native snapshot.
    ///
    /// `maximum_record_bytes` bounds the combined copied record bodies. `None`
    /// means no writer lineage has started. It does not mean the principal store
    /// is absent or trusted. The returned evidence grants no lock, takeover,
    /// liveness, signing, or runtime authority.
    ///
    /// # Errors
    ///
    /// Returns a control-store error for a wrong store binding, partial or
    /// conflicting closure, exceeded byte limit, or native read failure.
    fn read_current_writer(
        &mut self,
        binding: &PrincipalTrustStoreBinding,
        maximum_record_bytes: usize,
    ) -> Result<Option<PrincipalResidentWriterSnapshot>, ControlStoreError>;
}

/// Copy limits for one physical resident-writer record-set snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrincipalResidentWriterRecordSetReadLimits {
    maximum_writer_records: usize,
    maximum_checkpoint_records: usize,
    maximum_record_bytes: usize,
}

impl PrincipalResidentWriterRecordSetReadLimits {
    /// Set separate row ceilings and one aggregate record-body ceiling.
    #[must_use]
    pub const fn new(
        maximum_writer_records: usize,
        maximum_checkpoint_records: usize,
        maximum_record_bytes: usize,
    ) -> Self {
        Self {
            maximum_writer_records,
            maximum_checkpoint_records,
            maximum_record_bytes,
        }
    }

    /// Return the maximum number of copied writer-family records.
    #[must_use]
    pub const fn maximum_writer_records(self) -> usize {
        self.maximum_writer_records
    }

    /// Return the maximum number of copied checkpoint records.
    #[must_use]
    pub const fn maximum_checkpoint_records(self) -> usize {
        self.maximum_checkpoint_records
    }

    /// Return the maximum combined body bytes copied from all records.
    #[must_use]
    pub const fn maximum_record_bytes(self) -> usize {
        self.maximum_record_bytes
    }
}

/// One opaque record and the store transaction that inserted it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrincipalResidentWriterStoredRecord {
    inserted_in: ControlTransactionSequenceV1,
    record: ControlRecord,
}

impl PrincipalResidentWriterStoredRecord {
    /// Retain one immutable record with its positive insertion stamp.
    #[must_use]
    pub const fn new(inserted_in: ControlTransactionSequenceV1, record: ControlRecord) -> Self {
        Self {
            inserted_in,
            record,
        }
    }

    /// Return the store transaction that inserted this record.
    #[must_use]
    pub const fn inserted_in(&self) -> ControlTransactionSequenceV1 {
        self.inserted_in
    }

    /// Borrow the exact opaque record.
    #[must_use]
    pub const fn record(&self) -> &ControlRecord {
        &self.record
    }
}

/// Physical resident-writer records copied from one principal-store snapshot.
///
/// The writer-row set contains records labelled with the fence, terminal
/// receipt, or takeover receipt schema ID. The checkpoint-row set contains
/// records labelled with the control checkpoint schema ID and inserted in the
/// same store transactions. These labels and bytes are unadmitted. This value
/// does not establish their meaning, lineage, signatures, liveness, takeover,
/// resident authority, or readiness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrincipalResidentWriterRecordSetSnapshot {
    binding: PrincipalTrustStoreBinding,
    head: PrincipalResidentWriterHead,
    observed_through: ControlTransactionSequenceV1,
    current_writer_index: usize,
    current_checkpoint_index: usize,
    writer_records: Box<[PrincipalResidentWriterStoredRecord]>,
    checkpoint_records: Box<[PrincipalResidentWriterStoredRecord]>,
}

impl PrincipalResidentWriterRecordSetSnapshot {
    /// Check one physical record set against its selected current head.
    ///
    /// # Errors
    ///
    /// Rejects another principal, empty or wrongly typed sets, repeated or
    /// out-of-order coordinates, future insertion stamps, a checkpoint without
    /// a writer transaction, or invalid current writer and checkpoint records.
    pub fn new(
        binding: PrincipalTrustStoreBinding,
        head: PrincipalResidentWriterHead,
        observed_through: ControlTransactionSequenceV1,
        writer_records: Box<[PrincipalResidentWriterStoredRecord]>,
        checkpoint_records: Box<[PrincipalResidentWriterStoredRecord]>,
    ) -> Result<Self, PrincipalResidentWriterRecordSetSnapshotError> {
        if binding.principal_id() != head.principal_id() {
            return Err(PrincipalResidentWriterRecordSetSnapshotError::PrincipalBinding);
        }
        if head.checkpoint().committed_transaction_sequence() > observed_through {
            return Err(PrincipalResidentWriterRecordSetSnapshotError::FutureRecord);
        }
        if writer_records.is_empty() || checkpoint_records.is_empty() {
            return Err(PrincipalResidentWriterRecordSetSnapshotError::Empty);
        }
        let writer_transactions = writer_records
            .iter()
            .map(PrincipalResidentWriterStoredRecord::inserted_in)
            .collect::<std::collections::BTreeSet<_>>();
        let mut coordinates = std::collections::BTreeSet::new();
        let mut current_writer_index = None;
        for (index, retained) in writer_records.iter().enumerate() {
            if retained.inserted_in() > observed_through {
                return Err(PrincipalResidentWriterRecordSetSnapshotError::FutureRecord);
            }
            let record = retained.record();
            if !matches!(
                record.schema_id().as_str(),
                pale_core::records::PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SCHEMA
                    | pale_core::records::PRINCIPAL_RESIDENT_WRITER_TERMINAL_RECEIPT_V1_SCHEMA
                    | pale_core::records::PRINCIPAL_RESIDENT_WRITER_TAKEOVER_RECEIPT_V1_SCHEMA
            ) {
                return Err(PrincipalResidentWriterRecordSetSnapshotError::RecordFamily);
            }
            if !coordinates.insert(record.coordinate().clone()) {
                return Err(PrincipalResidentWriterRecordSetSnapshotError::RepeatedCoordinate);
            }
            if record.coordinate() == head.coordinate()
                && current_writer_index.replace(index).is_some()
            {
                return Err(PrincipalResidentWriterRecordSetSnapshotError::CurrentRecord);
            }
        }
        let mut current_checkpoint_index = None;
        let mut checkpoint_transactions = std::collections::BTreeSet::new();
        for (index, retained) in checkpoint_records.iter().enumerate() {
            if retained.inserted_in() > observed_through {
                return Err(PrincipalResidentWriterRecordSetSnapshotError::FutureRecord);
            }
            if !writer_transactions.contains(&retained.inserted_in()) {
                return Err(PrincipalResidentWriterRecordSetSnapshotError::CheckpointTransaction);
            }
            checkpoint_transactions.insert(retained.inserted_in());
            let record = retained.record();
            if record.schema_id().as_str() != pale_core::records::CONTROL_STATE_CHECKPOINT_V1_SCHEMA
            {
                return Err(PrincipalResidentWriterRecordSetSnapshotError::RecordFamily);
            }
            if !coordinates.insert(record.coordinate().clone()) {
                return Err(PrincipalResidentWriterRecordSetSnapshotError::RepeatedCoordinate);
            }
            if record.coordinate() == head.checkpoint().coordinate()
                && current_checkpoint_index.replace(index).is_some()
            {
                return Err(PrincipalResidentWriterRecordSetSnapshotError::CurrentRecord);
            }
        }
        let current_writer_index = current_writer_index
            .ok_or(PrincipalResidentWriterRecordSetSnapshotError::CurrentRecord)?;
        let current_checkpoint_index = current_checkpoint_index
            .ok_or(PrincipalResidentWriterRecordSetSnapshotError::CurrentRecord)?;
        if !records_follow_store_order(&writer_records)
            || !records_follow_store_order(&checkpoint_records)
        {
            return Err(PrincipalResidentWriterRecordSetSnapshotError::RecordOrder);
        }
        if writer_records[current_writer_index]
            .record()
            .schema_id()
            .as_str()
            != pale_core::records::PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SCHEMA
            || writer_records[current_writer_index].inserted_in()
                != checkpoint_records[current_checkpoint_index].inserted_in()
            || head.checkpoint().committed_transaction_sequence()
                != checkpoint_records[current_checkpoint_index].inserted_in()
        {
            return Err(PrincipalResidentWriterRecordSetSnapshotError::CurrentRecord);
        }
        if !writer_transactions.is_subset(&checkpoint_transactions) {
            return Err(PrincipalResidentWriterRecordSetSnapshotError::CheckpointTransaction);
        }
        Ok(Self {
            binding,
            head,
            observed_through,
            current_writer_index,
            current_checkpoint_index,
            writer_records,
            checkpoint_records,
        })
    }

    /// Borrow the exact trust-store incarnation and principal binding.
    #[must_use]
    pub const fn binding(&self) -> &PrincipalTrustStoreBinding {
        &self.binding
    }

    /// Borrow the selected physical current writer head.
    #[must_use]
    pub const fn head(&self) -> &PrincipalResidentWriterHead {
        &self.head
    }

    /// Return the exact store transaction observed by the read.
    #[must_use]
    pub const fn observed_through(&self) -> ControlTransactionSequenceV1 {
        self.observed_through
    }

    /// Borrow writer-family records in transaction and reference order.
    #[must_use]
    pub const fn writer_records(&self) -> &[PrincipalResidentWriterStoredRecord] {
        &self.writer_records
    }

    /// Borrow checkpoints inserted with the copied writer records.
    #[must_use]
    pub const fn checkpoint_records(&self) -> &[PrincipalResidentWriterStoredRecord] {
        &self.checkpoint_records
    }

    /// Borrow the row whose coordinate matches the current writer head.
    #[must_use]
    pub fn current_writer_record(&self) -> &PrincipalResidentWriterStoredRecord {
        &self.writer_records[self.current_writer_index]
    }

    /// Borrow the row whose coordinate matches the current checkpoint head.
    #[must_use]
    pub fn current_checkpoint_record(&self) -> &PrincipalResidentWriterStoredRecord {
        &self.checkpoint_records[self.current_checkpoint_index]
    }
}

/// A physical writer record set is inconsistent with its current head.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PrincipalResidentWriterRecordSetSnapshotError {
    /// The writer head and store binding name different principals.
    #[error("resident-writer record set does not match the principal binding")]
    PrincipalBinding,
    /// At least one required physical set is empty.
    #[error("resident-writer record set is empty")]
    Empty,
    /// A row belongs to another record family.
    #[error("resident-writer record set contains another record family")]
    RecordFamily,
    /// Two rows repeat one immutable coordinate.
    #[error("resident-writer record set repeats a record coordinate")]
    RepeatedCoordinate,
    /// Rows do not follow store transaction and record-coordinate order.
    #[error("resident-writer record set is not in store order")]
    RecordOrder,
    /// A row or selected checkpoint head claims a future store transaction.
    #[error("resident-writer record set contains a future transaction claim")]
    FutureRecord,
    /// A copied checkpoint was not inserted with a copied writer record.
    #[error("resident-writer checkpoint has no matching writer transaction")]
    CheckpointTransaction,
    /// A current record is missing, repeated, wrongly typed, or from another transaction.
    #[error("resident-writer record set does not contain each current record exactly once")]
    CurrentRecord,
}

fn records_follow_store_order(records: &[PrincipalResidentWriterStoredRecord]) -> bool {
    records.windows(2).all(|pair| {
        (pair[0].inserted_in(), pair[0].record().coordinate())
            < (pair[1].inserted_in(), pair[1].record().coordinate())
    })
}

/// Read the complete bounded physical record set for one resident writer.
pub trait PrincipalResidentWriterRecordSetReadPort {
    /// Copy schema-labelled writer rows and their co-inserted checkpoint rows.
    ///
    /// `None` means no current writer head and no writer material exist. The
    /// returned bytes remain unadmitted and grant no lock, liveness, takeover,
    /// signing, mutation, resident authority, or readiness.
    ///
    /// # Errors
    ///
    /// Returns a control-store error for inconsistent physical evidence, an
    /// exceeded count or byte bound, or a native read failure.
    fn read_writer_record_set(
        &mut self,
        binding: &PrincipalTrustStoreBinding,
        limits: PrincipalResidentWriterRecordSetReadLimits,
    ) -> Result<Option<PrincipalResidentWriterRecordSetSnapshot>, ControlStoreError>;
}

/// Copy limits for one deployment control store's physical genesis closure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeploymentControlStoreGenesisReadLimits {
    maximum_checkpoints: usize,
    maximum_record_bytes: usize,
    maximum_mutations: usize,
}

impl DeploymentControlStoreGenesisReadLimits {
    /// Set checkpoint, aggregate record-body, and aggregate mutation ceilings.
    #[must_use]
    pub const fn new(
        maximum_checkpoints: usize,
        maximum_record_bytes: usize,
        maximum_mutations: usize,
    ) -> Self {
        Self {
            maximum_checkpoints,
            maximum_record_bytes,
            maximum_mutations,
        }
    }

    /// Return the maximum number of copied initial checkpoint closures.
    #[must_use]
    pub const fn maximum_checkpoints(self) -> usize {
        self.maximum_checkpoints
    }

    /// Return the maximum combined bytes copied from immutable records.
    #[must_use]
    pub const fn maximum_record_bytes(self) -> usize {
        self.maximum_record_bytes
    }

    /// Return the maximum combined physical mutations copied from all batches.
    #[must_use]
    pub const fn maximum_mutations(self) -> usize {
        self.maximum_mutations
    }
}

/// Deployment-store genesis and the immutable physical evidence that closes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeploymentControlStoreGenesisSnapshot {
    binding: DeploymentControlStoreBinding,
    genesis_id: DeploymentControlStoreGenesisIdV1,
    finalization_trust_store: AuthorityStoreCoordinate,
    committed_transaction: ControlTransactionSequenceV1,
    candidate_identity_sha256: ArtifactDigest,
    records: DeploymentControlStoreGenesisRecords,
    checkpoint_heads: Vec<ControlCheckpointHead>,
    accumulators: Vec<OperationAccumulatorBatch>,
}

impl DeploymentControlStoreGenesisSnapshot {
    /// Bind one genesis singleton to its receipt, checkpoints, and initial accumulators.
    ///
    /// This value preserves physical closure only. It does not admit signatures,
    /// trust, anchors, deployment selection, or runtime readiness.
    ///
    /// # Errors
    ///
    /// Rejects a missing, substituted, or differently committed checkpoint or
    /// accumulator batch.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        binding: DeploymentControlStoreBinding,
        genesis_id: DeploymentControlStoreGenesisIdV1,
        finalization_trust_store: AuthorityStoreCoordinate,
        committed_transaction: ControlTransactionSequenceV1,
        candidate_identity_sha256: ArtifactDigest,
        records: DeploymentControlStoreGenesisRecords,
        checkpoint_heads: Vec<ControlCheckpointHead>,
        accumulators: Vec<OperationAccumulatorBatch>,
    ) -> Result<Self, DeploymentControlStoreGenesisSnapshotError> {
        if finalization_trust_store == *binding.store() {
            return Err(DeploymentControlStoreGenesisSnapshotError::StoreSeparation);
        }
        if checkpoint_heads.len() != records.checkpoints().len()
            || checkpoint_heads.len() != accumulators.len()
            || checkpoint_heads
                .windows(2)
                .any(|pair| pair[0].checkpoint_domain_id() >= pair[1].checkpoint_domain_id())
        {
            return Err(DeploymentControlStoreGenesisSnapshotError::CheckpointSet);
        }
        for ((head, record), accumulator) in checkpoint_heads
            .iter()
            .zip(records.checkpoints())
            .zip(&accumulators)
        {
            if head.coordinate() != record.coordinate()
                || head.domain_epoch().get() != 1
                || head.domain_sequence().get() != 1
                || head.committed_transaction_sequence() != committed_transaction
            {
                return Err(DeploymentControlStoreGenesisSnapshotError::CheckpointSet);
            }
            if accumulator.binding().store() != binding.store()
                || accumulator.binding().successor() != head
                || !matches!(
                    accumulator.binding().phase(),
                    crate::AccumulatorCheckpointPhase::Initial
                )
            {
                return Err(DeploymentControlStoreGenesisSnapshotError::AccumulatorSet);
            }
        }
        Ok(Self {
            binding,
            genesis_id,
            finalization_trust_store,
            committed_transaction,
            candidate_identity_sha256,
            records,
            checkpoint_heads,
            accumulators,
        })
    }

    /// Borrow the exact deployment-store and finalized authority coordinates.
    #[must_use]
    pub const fn binding(&self) -> &DeploymentControlStoreBinding {
        &self.binding
    }

    /// Borrow the stable genesis transaction identity.
    #[must_use]
    pub const fn genesis_id(&self) -> &DeploymentControlStoreGenesisIdV1 {
        &self.genesis_id
    }

    /// Borrow the principal trust-store incarnation that finalized deployment.
    #[must_use]
    pub const fn finalization_trust_store(&self) -> &AuthorityStoreCoordinate {
        &self.finalization_trust_store
    }

    /// Return the deployment-store transaction that committed genesis.
    #[must_use]
    pub const fn committed_transaction(&self) -> ControlTransactionSequenceV1 {
        self.committed_transaction
    }

    /// Borrow the identity of the candidate whose genesis was retained.
    #[must_use]
    pub const fn candidate_identity_sha256(&self) -> &ArtifactDigest {
        &self.candidate_identity_sha256
    }

    /// Borrow the initial checkpoint records and genesis receipt.
    #[must_use]
    pub const fn records(&self) -> &DeploymentControlStoreGenesisRecords {
        &self.records
    }

    /// Borrow the domain-ordered initial checkpoint heads.
    #[must_use]
    pub fn checkpoint_heads(&self) -> &[ControlCheckpointHead] {
        &self.checkpoint_heads
    }

    /// Borrow the one-for-one initial accumulator batches retained at genesis.
    #[must_use]
    pub fn accumulators(&self) -> &[OperationAccumulatorBatch] {
        &self.accumulators
    }
}

/// A deployment genesis snapshot does not reproduce one exact closure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DeploymentControlStoreGenesisSnapshotError {
    /// Genesis reuses the principal trust-store coordinate as its deployment store.
    #[error("deployment genesis stores are not physically separate")]
    StoreSeparation,
    /// The checkpoint heads and immutable checkpoint records do not match.
    #[error("deployment genesis checkpoints do not form one exact closure")]
    CheckpointSet,
    /// The accumulator batches do not close over the same store and checkpoints.
    #[error("deployment genesis accumulators do not form one exact closure")]
    AccumulatorSet,
}

/// Read-only physical discovery of deployment control-store genesis.
pub trait DeploymentControlStoreGenesisReadPort {
    /// Read the complete physical genesis closure in one native snapshot.
    ///
    /// The selected limits bound copied checkpoints, record bytes, and mutations.
    /// `None` means no genesis material is retained. Presence does not establish
    /// signature, trust, anchor, or Pale Runtime assembly.
    ///
    /// # Errors
    ///
    /// Returns a control-store error for a wrong store binding, partial or
    /// conflicting genesis, an exceeded checkpoint, aggregate record-byte, or
    /// mutation limit, or native read failure.
    fn read_genesis(
        &mut self,
        binding: &DeploymentControlStoreBinding,
        limits: DeploymentControlStoreGenesisReadLimits,
    ) -> Result<Option<DeploymentControlStoreGenesisSnapshot>, ControlStoreError>;
}
