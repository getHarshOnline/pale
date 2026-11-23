//! Initial operation leaves, native construction, signing and custody assembly.
//! No initialized domain or usable operation is fabricated during preparation.

use pale_core::records::*;
use pale_ports::{
    AccumulatorLeafChange, AccumulatorPreparationBasis, AccumulatorPreparationLimits,
    OperationAccumulatorCheckpoint, OperationAccumulatorPreparation, OperationCheckpointPhase,
    OperationCustodyCandidate, OperationCustodyCheckpoint, OperationCustodyPort,
    OperationCustodyTransitionKind, PreparedOperationAccumulator, StateLeafChanges,
    UnresolvedCustodyLeafChanges,
};

use super::{
    checkpoint_record, require, LocalOperationStartError, PendingLocalOperationStart,
    PreparedLocalOperationStart, StartBasis,
};
use crate::resident::operation_custody::{prepare_operation_start, PreparedOperationStart};

/// A native-verified root awaiting its exact initial or successor checkpoint signature.
/// This retains neither a database transaction nor a signer. It cannot issue a
/// locator; physical comparison, atomic custody and independent anchoring follow.
#[must_use = "prepared roots are not durable operation custody"]
pub struct PreparedLocalOperationCheckpoint {
    start: PreparedLocalOperationStart,
    changed_keys: Vec<ChangedControlLeafKeyV1>,
    accumulator: PreparedOperationAccumulator,
    leaf_count: ControlLeafCountV1,
}

/// Signed operation records retained when native checkpoint preparation stops.
#[must_use = "continue checkpoint preparation for these exact signed records"]
pub struct PreparedLocalOperationStartFailure {
    continuation: Box<PreparedLocalOperationStart>,
    failure: LocalOperationStartError,
}

impl std::fmt::Debug for PreparedLocalOperationStartFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PreparedLocalOperationStartFailure")
            .field("reason", &self.failure)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Display for PreparedLocalOperationStartFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.failure, formatter)
    }
}

impl std::error::Error for PreparedLocalOperationStartFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.failure)
    }
}

/// Prepared checkpoint state retained when validation or signing stops.
#[must_use = "continue checkpoint issuance for this exact signed operation start"]
pub(in crate::resident) struct PreparedLocalOperationCheckpointFailure {
    continuation: Box<PreparedLocalOperationCheckpoint>,
    failure: LocalOperationStartError,
}

impl PreparedLocalOperationCheckpointFailure {
    pub(in crate::resident) fn continue_issuance<Sign>(
        self,
        reference: &ResidentRecordRefV1,
        preimage: ControlStateCheckpointPreimageV1,
        schema: &SchemaDigest,
        authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        sign: Sign,
    ) -> Result<IssuedLocalOperationCheckpoint, Self>
    where
        Sign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        self.continuation
            .issue_checkpoint_retained(reference, preimage, schema, authority, sign)
    }
}

/// Exact signed checkpoint and signed start awaiting physical attachment.
#[must_use = "attach this exact checkpoint without signing another value"]
pub(in crate::resident) struct IssuedLocalOperationCheckpoint {
    start: PreparedLocalOperationStart,
    accumulator: Option<PreparedOperationAccumulator>,
    limits: AccumulatorPreparationLimits,
    reference: ResidentRecordRefV1,
    checkpoint: ControlStateCheckpointV1,
    kind: OperationCustodyTransitionKind,
    phase: OperationCheckpointPhase,
}

/// Attachment failure retaining the already signed checkpoint.
#[must_use = "continue attachment without signing another checkpoint"]
pub(in crate::resident) struct IssuedLocalOperationCheckpointFailure {
    continuation: Box<IssuedLocalOperationCheckpoint>,
    failure: LocalOperationStartError,
}

impl IssuedLocalOperationCheckpointFailure {
    pub(in crate::resident) fn continue_attachment(
        self,
        custody: &mut dyn OperationCustodyPort,
    ) -> Result<(PreparedOperationStart, PendingLocalOperationStart), Self> {
        self.continuation.continue_attachment(custody)
    }
}

impl PreparedLocalOperationStartFailure {
    /// Borrow the reason without discarding the signed start continuation.
    #[must_use]
    pub const fn reason(&self) -> &LocalOperationStartError {
        &self.failure
    }

    /// Retry native preparation for the same signed handle and observation.
    ///
    /// # Errors
    /// Returns the retained start if limits, domain state or native preparation
    /// still prevent checkpoint construction.
    pub fn continue_checkpoint(
        self,
        limits: AccumulatorPreparationLimits,
        custody: &mut dyn OperationCustodyPort,
    ) -> Result<PreparedLocalOperationCheckpoint, Self> {
        self.continuation.prepare_checkpoint(limits, custody)
    }
}

/// A checkpoint completion that stopped without losing its exact signed phase.
///
/// The retained phase determines whether retry may sign the prepared checkpoint
/// or must only rebuild its native attachment. Callers cannot inspect or replace
/// that phase.
#[must_use = "continue this exact checkpoint phase instead of signing or preparing again"]
pub struct LocalOperationCheckpointFailure {
    continuation: Box<LocalOperationCheckpointContinuation>,
}

impl std::fmt::Debug for LocalOperationCheckpointFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LocalOperationCheckpointFailure")
            .field("reason", self.reason())
            .finish_non_exhaustive()
    }
}

impl std::fmt::Display for LocalOperationCheckpointFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.reason(), formatter)
    }
}

impl std::error::Error for LocalOperationCheckpointFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.reason())
    }
}

enum LocalOperationCheckpointContinuation {
    Prepared(PreparedLocalOperationCheckpointFailure),
    Issued(IssuedLocalOperationCheckpointFailure),
}

impl LocalOperationCheckpointFailure {
    fn from_prepared(failure: PreparedLocalOperationCheckpointFailure) -> Self {
        Self {
            continuation: Box::new(LocalOperationCheckpointContinuation::Prepared(failure)),
        }
    }

    fn from_issued(failure: IssuedLocalOperationCheckpointFailure) -> Self {
        Self {
            continuation: Box::new(LocalOperationCheckpointContinuation::Issued(failure)),
        }
    }

    /// Borrow the reason without discarding the prepared or issued checkpoint.
    #[must_use]
    pub const fn reason(&self) -> &LocalOperationStartError {
        match &*self.continuation {
            LocalOperationCheckpointContinuation::Prepared(failure) => &failure.failure,
            LocalOperationCheckpointContinuation::Issued(failure) => &failure.failure,
        }
    }

    /// Continue the exact retained phase.
    ///
    /// A prepared phase may call `sign` once. An issued phase never calls it;
    /// it only reconstructs or validates the native attachment when required.
    ///
    /// # Errors
    /// Returns the retained phase when checkpoint validation, signing or native
    /// attachment fails. An issued checkpoint remains signed exactly once.
    #[allow(clippy::too_many_arguments)]
    pub fn continue_checkpoint<Sign>(
        self,
        reference: &ResidentRecordRefV1,
        preimage: ControlStateCheckpointPreimageV1,
        schema: &SchemaDigest,
        authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        custody: &mut dyn OperationCustodyPort,
        sign: Sign,
    ) -> Result<(PreparedOperationStart, PendingLocalOperationStart), Self>
    where
        Sign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let issued = match *self.continuation {
            LocalOperationCheckpointContinuation::Prepared(prepared) => prepared
                .continue_issuance(reference, preimage, schema, authority, sign)
                .map_err(Self::from_prepared)?,
            LocalOperationCheckpointContinuation::Issued(issued) => {
                return issued
                    .continue_attachment(custody)
                    .map_err(Self::from_issued);
            }
        };
        issued.attach_existing().map_err(Self::from_issued)
    }
}

impl PreparedLocalOperationStart {
    /// Prepare request, handle, session-basis and observation leaves against
    /// whole-domain absence or the retained parent head.
    ///
    /// The store must already have been admitted by its owner. Root absence
    /// is checked across every epoch/profile of this domain, not just epoch one.
    /// Children use the inherited domain and transaction captured before parent
    /// recovery. Native preparation must reject any intervening store mutation.
    /// No custody or anchor write occurs. An existing independent anchor still
    /// prevents initial commitment even if the local database has lost its head.
    ///
    /// # Errors
    /// Returns the same signed start when limits, domain state, native proof
    /// construction, or the returned preparation does not admit. Retry only
    /// through [`PreparedLocalOperationStartFailure::continue_checkpoint`].
    pub fn prepare_checkpoint(
        self,
        limits: AccumulatorPreparationLimits,
        custody: &mut dyn OperationCustodyPort,
    ) -> Result<PreparedLocalOperationCheckpoint, PreparedLocalOperationStartFailure> {
        let prepared = (|| {
            let mut changes = self.leaves.changes()?;
            let mut changed_keys: Vec<_> = changes.iter().map(|(key, _, _)| key.clone()).collect();
            changed_keys.sort();
            // Core checkpoint keys use their logical tuple order. Native proof
            // inputs instead use hashed-key order; neither is an enumeration rank.
            changes.sort_by(|left, right| left.1.cmp(&right.1));
            let leaves = changes
                .into_iter()
                .map(|(_, key, value)| AccumulatorLeafChange::new(key, None, Some(value)))
                .collect::<Result<Vec<_>, _>>()?;
            let (basis, transaction, previous_count) = match &self.basis {
                StartBasis::Root => (
                    self.checkpoint_selection.initial_basis()?,
                    custody
                        .prepare_transaction(&self.store)
                        .map_err(pale_ports::OperationAccumulatorPreparationError::from)?,
                    0,
                ),
                StartBasis::Child(parent) => {
                    let (reference, checkpoint) = &parent.checkpoint;
                    let (head, _) = checkpoint_record((reference, checkpoint))?;
                    let prior = OperationAccumulatorCheckpoint::new(
                        self.store.clone(),
                        checkpoint.checkpoint_profile_ref().clone(),
                        checkpoint.checkpoint_profile_sha256().clone(),
                        head,
                        checkpoint.state_root_sha256().clone(),
                        checkpoint.unresolved_custody_root_sha256().clone(),
                    );
                    // Never refresh the transaction captured before the protected parent read.
                    (
                        AccumulatorPreparationBasis::Successor(prior),
                        parent.transaction.clone(),
                        checkpoint.leaf_count().get(),
                    )
                }
            };
            let leaf_count = ControlLeafCountV1::new(
                previous_count
                    .checked_add(changed_keys.len() as u64)
                    .ok_or(LocalOperationStartError::Coordinate)?,
            )?;
            let request = OperationAccumulatorPreparation::new(
                basis,
                StateLeafChanges::new(leaves, limits.maximum_leaf_changes())?,
                UnresolvedCustodyLeafChanges::new(Vec::new(), limits.maximum_leaf_changes())?,
                limits,
                self.checkpoint_selection.implementation(),
                transaction,
            )?;
            let accumulator = custody.prepare_accumulator(&request)?;
            require(accumulator.request() == &request)?;
            Ok::<_, LocalOperationStartError>((changed_keys, accumulator, leaf_count))
        })();
        match prepared {
            Ok((changed_keys, accumulator, leaf_count)) => Ok(PreparedLocalOperationCheckpoint {
                start: self,
                changed_keys,
                accumulator,
                leaf_count,
            }),
            Err(failure) => Err(PreparedLocalOperationStartFailure {
                continuation: Box::new(self),
                failure,
            }),
        }
    }
}

impl PreparedLocalOperationCheckpoint {
    /// Existing domain head for a child, or absence for a root initialization.
    #[must_use]
    pub fn predecessor(&self) -> Option<(&ResidentRecordRefV1, &ControlStateCheckpointV1)> {
        match &self.start.basis {
            StartBasis::Root => None,
            StartBasis::Child(parent) => Some((&parent.checkpoint.0, &parent.checkpoint.1)),
        }
    }

    /// Exact retained count after the three new identity and observation leaves.
    #[must_use]
    pub const fn leaf_count(&self) -> ControlLeafCountV1 {
        self.leaf_count
    }

    /// Proposed store mutation to put in the checkpoint preimage.
    /// This is neither committed nor reserved; a later writer causes conflict.
    #[must_use]
    pub fn transaction_sequence(&self) -> ControlTransactionSequenceV1 {
        self.accumulator.request().transaction().successor()
    }

    /// Borrow the native-verified root for the exact checkpoint preimage.
    #[must_use]
    pub fn state_root(&self) -> &ArtifactDigest {
        self.accumulator.state_root()
    }

    /// Borrow the verified empty initial or unchanged inherited unresolved root.
    #[must_use]
    pub fn unresolved_root(&self) -> &ArtifactDigest {
        self.accumulator.unresolved_root()
    }

    /// Borrow the three newly inserted logical keys in the existing core canonical order.
    #[must_use]
    pub fn changed_keys(&self) -> &[ChangedControlLeafKeyV1] {
        &self.changed_keys
    }

    /// Sign the prepared initial or child-successor checkpoint and assemble custody.
    ///
    /// The existing core record owner checks schema, structure and the supplied
    /// current signing witness before calling `sign`. Reference allocation,
    /// signer resolution/key custody and time remain caller obligations. The
    /// retained native preparation supplies the proposed transaction fence;
    /// actual commitment must still compare it inside the mutation transaction.
    /// The native batch does not authenticate an active-at-issuance signer.
    /// This method never treats a label
    /// or a signature alone as construction proof: the roots come from retained
    /// native preparation of the same logical leaves and exact domain.
    /// The returned pending locator becomes usable only after the exact database
    /// candidate and its exact independent-anchor transition have both completed.
    ///
    /// # Errors
    /// Returns the exact prepared or issued checkpoint when validation, signing,
    /// or attachment stops. Retry only through
    /// [`LocalOperationCheckpointFailure::continue_checkpoint`].
    pub fn issue_checkpoint<Sign>(
        self,
        reference: &ResidentRecordRefV1,
        preimage: ControlStateCheckpointPreimageV1,
        schema: &SchemaDigest,
        authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        sign: Sign,
    ) -> Result<(PreparedOperationStart, PendingLocalOperationStart), LocalOperationCheckpointFailure>
    where
        Sign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        self.issue_checkpoint_retained(reference, preimage, schema, authority, sign)
            .map_err(LocalOperationCheckpointFailure::from_prepared)?
            .attach_existing()
            .map_err(LocalOperationCheckpointFailure::from_issued)
    }

    pub(in crate::resident) fn issue_checkpoint_retained<Sign>(
        self,
        reference: &ResidentRecordRefV1,
        preimage: ControlStateCheckpointPreimageV1,
        schema: &SchemaDigest,
        authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        sign: Sign,
    ) -> Result<IssuedLocalOperationCheckpoint, PreparedLocalOperationCheckpointFailure>
    where
        Sign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let basis = self.accumulator.basis();
        let result = (|| {
            let (sequence, previous_ref, previous_digest) = match &self.start.basis {
                StartBasis::Root => (1, None, None),
                StartBasis::Child(parent) => (
                    parent
                        .checkpoint
                        .1
                        .domain_sequence()
                        .get()
                        .checked_add(1)
                        .ok_or(LocalOperationStartError::Coordinate)?,
                    Some(&parent.checkpoint.0),
                    Some(parent.checkpoint.1.canonical_control_checkpoint_sha256()),
                ),
            };
            require(
                self.start.checkpoint_selection.matches_authority(authority)
                    && authority.principal_id == self.start.handle.issuer_principal_id()
                    && &preimage.control_store_id == basis.store().store_id()
                    && &preimage.control_incarnation_id == basis.store().incarnation_id()
                    && &preimage.checkpoint_domain_id == basis.domain()
                    && preimage.domain_epoch == basis.epoch()
                    && preimage.domain_sequence.get() == sequence
                    && preimage.previous_control_checkpoint_ref.as_ref() == previous_ref
                    && preimage.previous_control_checkpoint_sha256.as_ref() == previous_digest
                    && preimage.committed_transaction_sequence == self.transaction_sequence()
                    && &preimage.checkpoint_profile_ref == basis.profile_ref()
                    && &preimage.checkpoint_profile_sha256 == basis.profile_sha256()
                    && preimage.control_schema_version
                        == crate::resident::checkpoint_profile::CONTROL_SCHEMA_VERSION
                    && &preimage.state_root_sha256 == self.accumulator.state_root()
                    && &preimage.unresolved_custody_root_sha256
                        == self.accumulator.unresolved_root()
                    && preimage.changed_leaf_keys == self.changed_keys
                    && preimage.leaf_count == self.leaf_count,
            )?;
            let (checkpoint, kind, phase) = match &self.start.basis {
                StartBasis::Root => (
                    ControlStateCheckpointV1::issue_initial_contextual(
                        preimage, schema, authority, sign,
                    )?,
                    OperationCustodyTransitionKind::StartRoot,
                    OperationCheckpointPhase::Initial,
                ),
                StartBasis::Child(parent) => {
                    let (previous_ref, previous) = &parent.checkpoint;
                    // Project the immutable predecessor before the successor is
                    // signed. A projection failure must not discard a valid
                    // signature and make a later continuation sign again.
                    let (predecessor, record) = checkpoint_record((previous_ref, previous))?;
                    let checkpoint = ControlStateCheckpointV1::issue_successor_contextual(
                        preimage,
                        previous_ref,
                        previous,
                        schema,
                        authority,
                        sign,
                    )?;
                    (
                        checkpoint,
                        OperationCustodyTransitionKind::StartChild,
                        OperationCheckpointPhase::Successor {
                            predecessor,
                            predecessor_record: Box::new(record),
                        },
                    )
                }
            };
            Ok::<_, LocalOperationStartError>((checkpoint, kind, phase))
        })();
        match result {
            Ok((checkpoint, kind, phase)) => Ok(IssuedLocalOperationCheckpoint {
                limits: self.accumulator.request().limits(),
                start: self.start,
                accumulator: Some(self.accumulator),
                reference: reference.clone(),
                checkpoint,
                kind,
                phase,
            }),
            Err(failure) => Err(PreparedLocalOperationCheckpointFailure {
                continuation: Box::new(self),
                failure,
            }),
        }
    }
}

impl IssuedLocalOperationCheckpoint {
    pub(in crate::resident) fn attach_retained(
        self,
    ) -> Result<
        (PreparedOperationStart, PendingLocalOperationStart),
        IssuedLocalOperationCheckpointFailure,
    > {
        self.attach_existing()
    }

    fn attach_existing(
        mut self,
    ) -> Result<
        (PreparedOperationStart, PendingLocalOperationStart),
        IssuedLocalOperationCheckpointFailure,
    > {
        let result = (|| {
            self.start.checkpoint_selection.verify(
                &self.start.store,
                &self.start.handle,
                &self.checkpoint,
            )?;
            let (head, record) = checkpoint_record((&self.reference, &self.checkpoint))?;
            let accumulator = self
                .accumulator
                .take()
                .expect("issued checkpoint retains native preparation")
                .bind_successor(head.clone())?;
            let candidate = OperationCustodyCandidate::new_start(
                self.start.store.clone(),
                self.kind,
                self.start.records.clone(),
                self.start.indexes.clone(),
                OperationCustodyCheckpoint::new(self.phase.clone(), head, record)?,
                accumulator,
                self.start.session_basis.clone(),
            )?;
            let prepared = prepare_operation_start(candidate.clone())?;
            Ok::<_, LocalOperationStartError>((prepared, candidate))
        })();
        let (prepared, candidate) = match result {
            Ok(values) => values,
            Err(failure) => {
                return Err(IssuedLocalOperationCheckpointFailure {
                    continuation: Box::new(self),
                    failure,
                });
            }
        };
        let start = self.start;
        Ok((
            prepared,
            PendingLocalOperationStart {
                checkpoint_selection: start.checkpoint_selection,
                expected: candidate,
                handle_ref: start.handle_ref,
                handle: start.handle,
                observation_ref: start.observation_ref,
                observation: start.observation,
                retained_session: start.retained_session,
                operation_lease: start.operation_lease,
                publication_policy: start.publication_policy,
                consultation_start_intent: start.consultation_start_intent,
            },
        ))
    }

    fn continue_attachment(
        self,
        custody: &mut dyn OperationCustodyPort,
    ) -> Result<
        (PreparedOperationStart, PendingLocalOperationStart),
        IssuedLocalOperationCheckpointFailure,
    > {
        if self.accumulator.is_some() {
            return self.attach_existing();
        }
        let Self {
            start,
            limits,
            reference,
            checkpoint,
            kind,
            phase,
            ..
        } = self;
        let prepared = match start.prepare_checkpoint(limits, custody) {
            Ok(prepared) => prepared,
            Err(failure) => {
                return Err(IssuedLocalOperationCheckpointFailure {
                    continuation: Box::new(IssuedLocalOperationCheckpoint {
                        start: *failure.continuation,
                        accumulator: None,
                        limits,
                        reference,
                        checkpoint,
                        kind,
                        phase,
                    }),
                    failure: failure.failure,
                });
            }
        };
        IssuedLocalOperationCheckpoint {
            start: prepared.start,
            accumulator: Some(prepared.accumulator),
            limits,
            reference,
            checkpoint,
            kind,
            phase,
        }
        .attach_existing()
    }
}
