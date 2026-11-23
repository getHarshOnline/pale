//! Record expiry of an Invoke or Delegate that never crossed dispatch.
//!
//! Expiry is durable but not terminal. It prevents the retained pre-dispatch
//! operation from starting. Reconciliation must later select a terminal result.
//! This module samples one wall clock and prepares one exact diagnostic and
//! successor. It does not cancel work or call an effect implementation.

use crate::diagnostic::{AdmittedDiagnostic, DiagnosticAdmissionError, DiagnosticProjection};
use crate::resident::operation_dispatch::{signing, OperationAttemptError};
use crate::resident::{
    operation_dispatch::{
        commit_prepared_checkpoint, observation_checkpoint_preimage, OperationAppendFailure,
        OperationCheckpointInput,
    },
    prepare_operation_observation_append, CompletedOperationObservationCustody,
    DatabaseCommittedOperationObservationAnchorPending, IssuedLocalOperation,
    OperationObservationAuthorityStoreResolver, OperationObservationCustodyAnchorFailure,
    OperationObservationCustodyCommitFailure, OperationObservationCustodyProjectionError,
    OperationObservationTransition, PreparedOperationObservationCheckpoint,
    PreparedOperationObservationCustody, RecoveredOperationControl,
};
use pale_core::records::{
    ArtifactDigest, CanonicalSignatureV1, ControlCheckpointSigningAuthorityAdmissionContextV1,
    ControlStateCheckpointDigest, ControlStateCheckpointPreimageV1, ControlStateCheckpointV1,
    DiagnosticDispositionV1, DiagnosticEffectCertaintyV1, DiagnosticRetryV1, DiagnosticStageV1,
    InvalidResidentRecordV1, OperationActivePhaseV1, OperationClassV1, OperationObservationDigest,
    OperationObservationPreimageV1, OperationObservationStateV1, OperationObservationV1,
    ResidentRecordRefV1, SchemaDigest, SignedResidentAdmissionContextV1, UnixTimeV1,
};
use pale_ports::{
    AccumulatorPreparationLimits, AntiRollbackAnchorPort, OperationCustodyPort,
    OperationCustodyRecord, WallClockSourcePort,
};

mod recovery;
pub use recovery::recover_committed_pre_dispatch_expiry;

/// Fresh identities and authority for one observed pre-dispatch expiry.
pub(in crate::resident) struct PreDispatchOperationExpiryInput<'a> {
    /// Selected wall clock, sampled exactly once by the expiry owner.
    pub(in crate::resident) clock: &'a dyn WallClockSourcePort,
    /// Fresh immutable reference for the admitted diagnostic.
    pub(in crate::resident) diagnostic_ref: ResidentRecordRefV1,
    /// Fresh immutable reference for the successor observation.
    pub(in crate::resident) observation_ref: ResidentRecordRefV1,
    /// Current admitted runtime signer for the successor observation.
    pub(in crate::resident) signing: SignedResidentAdmissionContextV1<'a>,
    /// Maximum canonical bytes accepted for the diagnostic.
    pub(in crate::resident) maximum_diagnostic_bytes: usize,
}

/// Expiry preparation stopped before a signed successor existed.
#[derive(Debug, thiserror::Error)]
pub(in crate::resident) enum PreDispatchOperationExpiryError {
    /// The selected clock did not produce a portable time.
    #[error(transparent)]
    Clock(#[from] pale_ports::WallClockSourceError),
    /// The operation has not reached its handle expiry.
    #[error("operation handle has not expired")]
    NotExpired,
    /// The clock reading precedes retained operation evidence.
    #[error("expiry time precedes the retained operation history")]
    TimeOrder,
    /// The operation's evidence retention period has elapsed.
    #[error("operation expiry is outside its evidence retention period")]
    RetentionElapsed,
    /// The recovered operation, state or fresh identities do not agree.
    #[error("operation expiry does not match the recovered pre-dispatch operation")]
    Association,
    /// The diagnostic exceeded its explicit byte ceiling.
    #[error("operation expiry diagnostic exceeds its byte limit")]
    BoundExceeded,
    /// Diagnostic construction or admission failed.
    #[error(transparent)]
    Diagnostic(#[from] DiagnosticAdmissionError),
    /// Observation projection, admission or signing rejected the successor.
    #[error(transparent)]
    Observation(#[from] OperationAttemptError),
}

/// Signed expiry and its diagnostic awaiting one exact checkpoint.
#[must_use = "retain the expiry through checkpoint custody or discard it"]
pub(in crate::resident) struct PreparedPreDispatchOperationExpiry<'a> {
    operation: &'a IssuedLocalOperation,
    checkpoint: (
        ResidentRecordRefV1,
        pale_core::records::ControlStateCheckpointV1,
    ),
    predecessor: (ResidentRecordRefV1, OperationObservationV1),
    diagnostic: AdmittedDiagnostic,
    diagnostic_record: pale_ports::ControlRecord,
    observation_ref: ResidentRecordRefV1,
    observation: OperationObservationV1,
    observed_at: UnixTimeV1,
}

impl PreparedPreDispatchOperationExpiry<'_> {
    /// Borrow the issued operation whose execution authority expired.
    pub(in crate::resident) const fn operation(&self) -> &IssuedLocalOperation {
        self.operation
    }

    /// Borrow the admitted diagnostic before physical custody.
    pub(in crate::resident) const fn diagnostic(&self) -> &AdmittedDiagnostic {
        &self.diagnostic
    }

    /// Borrow the immutable reference reserved for the diagnostic record.
    pub(in crate::resident) fn diagnostic_ref(&self) -> &ResidentRecordRefV1 {
        self.diagnostic_record.coordinate().record_ref()
    }

    /// Borrow the signed nonterminal expiry observation.
    pub(in crate::resident) const fn observation(
        &self,
    ) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.observation_ref, &self.observation)
    }

    /// Borrow the authenticated checkpoint that precedes this append.
    pub(in crate::resident) const fn predecessor_checkpoint(
        &self,
    ) -> (
        &ResidentRecordRefV1,
        &pale_core::records::ControlStateCheckpointV1,
    ) {
        (&self.checkpoint.0, &self.checkpoint.1)
    }

    /// Return the single wall-clock reading used for observation and checkpoint time.
    pub(in crate::resident) const fn observed_at(&self) -> UnixTimeV1 {
        self.observed_at
    }

    /// Detach the exact signed expiry from its recovered-operation borrow.
    ///
    /// The returned value retains the existing operation identity, predecessor,
    /// diagnostic, and signed observation. It carries no implementation or
    /// transport and cannot start external work.
    pub(in crate::resident) fn into_owned_observation(
        self,
    ) -> crate::resident::operation_dispatch::PreparedOwnedOperationObservation {
        crate::resident::operation_dispatch::PreparedOwnedOperationObservation::from_expiry(
            self.operation,
            (&self.predecessor.0, &self.predecessor.1),
            self.observation_ref,
            self.observation,
            self.diagnostic_record,
        )
    }

    /// Compute the diagnostic and observation roots without signing a checkpoint.
    fn prepare_checkpoint_value<'a>(
        &'a self,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        limits: AccumulatorPreparationLimits,
        custody: &mut dyn OperationCustodyPort,
    ) -> Result<
        PreparedOperationObservationCheckpoint<'a>,
        OperationObservationCustodyProjectionError,
    > {
        prepare_operation_observation_append(
            OperationObservationTransition::new(
                self.operation.handle().0,
                self.operation.handle().1,
                &self.predecessor.0,
                &self.predecessor.1,
                &self.observation_ref,
                &self.observation,
            ),
            resolve_store,
        )?
        .with_associated_record(OperationCustodyRecord::Diagnostic(
            self.diagnostic_record.clone(),
        ))
        .prepare_checkpoint(
            self.operation.checkpoint_selection(),
            &self.checkpoint.0,
            &self.checkpoint.1,
            limits,
            custody,
        )
    }

    /// Compute the exact roots for the diagnostic and expiry successor.
    pub(in crate::resident) fn prepare_checkpoint<'a>(
        &'a self,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        limits: AccumulatorPreparationLimits,
        custody: &mut dyn OperationCustodyPort,
    ) -> Result<PreparedPreDispatchOperationExpiryCheckpoint<'a>, OperationAppendFailure> {
        Ok(PreparedPreDispatchOperationExpiryCheckpoint {
            expiry: self,
            prepared: self.prepare_checkpoint_value(resolve_store, limits, custody)?,
        })
    }
}

/// Exact prepared roots for one pre-dispatch expiry checkpoint.
#[must_use = "issue this exact checkpoint or discard the prepared expiry"]
pub(in crate::resident) struct PreparedPreDispatchOperationExpiryCheckpoint<'a> {
    expiry: &'a PreparedPreDispatchOperationExpiry<'a>,
    prepared: PreparedOperationObservationCheckpoint<'a>,
}

impl<'a> PreparedPreDispatchOperationExpiryCheckpoint<'a> {
    /// Borrow the signed expiry whose roots were prepared.
    pub(in crate::resident) const fn expiry(&self) -> &PreparedPreDispatchOperationExpiry<'a> {
        self.expiry
    }

    /// Sign one fixed checkpoint. Later attachment accepts no signer.
    pub(in crate::resident) fn issue<Sign>(
        self,
        identity: OperationCheckpointInput,
        schema: &SchemaDigest,
        authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        sign: Sign,
    ) -> Result<IssuedPreDispatchOperationExpiryCheckpoint<'a>, OperationAppendFailure>
    where
        Sign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let preimage = observation_checkpoint_preimage(
            &self.prepared,
            self.expiry.predecessor_checkpoint(),
            &identity,
            self.expiry.observed_at(),
            schema,
            authority,
        )?;
        let checkpoint = self
            .prepared
            .issue_checkpoint_value(preimage, schema, authority, sign)?;
        Ok(IssuedPreDispatchOperationExpiryCheckpoint {
            expiry: self.expiry,
            identity,
            checkpoint,
        })
    }
}

/// Checkpoint signed once for an expiry, before physical attachment.
#[must_use = "attach this exact checkpoint without signing another value"]
pub(in crate::resident) struct IssuedPreDispatchOperationExpiryCheckpoint<'a> {
    expiry: &'a PreparedPreDispatchOperationExpiry<'a>,
    identity: OperationCheckpointInput,
    checkpoint: ControlStateCheckpointV1,
}

impl<'a> IssuedPreDispatchOperationExpiryCheckpoint<'a> {
    /// Borrow the signed expiry paired with this checkpoint.
    pub(in crate::resident) const fn expiry(&self) -> &PreparedPreDispatchOperationExpiry<'a> {
        self.expiry
    }

    /// Borrow the exact already-signed checkpoint.
    pub(in crate::resident) fn checkpoint(
        &self,
    ) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        (&self.identity.reference, &self.checkpoint)
    }

    /// Rebuild deterministic roots, attach this checkpoint and enter custody.
    pub(in crate::resident) fn attach_and_commit(
        self,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        limits: AccumulatorPreparationLimits,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<CompletedPreDispatchOperationExpiry<'a>, PreDispatchOperationExpiryCustodyFailure<'a>>
    {
        let prepared = match self
            .expiry
            .prepare_checkpoint(resolve_store, limits, custody)
        {
            Ok(prepared) => prepared,
            Err(failure) => {
                return Err(PreDispatchOperationExpiryCustodyFailure::Issued {
                    continuation: Box::new(self),
                    failure: Box::new(failure),
                });
            }
        };
        let physical = match prepared
            .prepared
            .attach_issued_checkpoint(&self.identity.reference, &self.checkpoint)
        {
            Ok(physical) => physical,
            Err(failure) => {
                return Err(PreDispatchOperationExpiryCustodyFailure::Issued {
                    continuation: Box::new(self),
                    failure: Box::new(OperationAppendFailure::Projection(failure)),
                });
            }
        };
        commit_expiry(
            self.expiry,
            self.identity,
            self.checkpoint,
            physical,
            custody,
            anchor,
        )
    }
}

/// One expiry whose database outcome remains ambiguous.
#[must_use = "reconcile only this retained exact expiry candidate"]
pub(in crate::resident) struct PreDispatchOperationExpiryCommitFailure<'a> {
    expiry: &'a PreparedPreDispatchOperationExpiry<'a>,
    checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    failure: OperationObservationCustodyCommitFailure,
}

impl<'a> PreDispatchOperationExpiryCommitFailure<'a> {
    /// Borrow the signed expiry retained by this candidate.
    pub(in crate::resident) const fn expiry(&self) -> &PreparedPreDispatchOperationExpiry<'a> {
        self.expiry
    }

    /// Read back this candidate without issuing or committing another value.
    pub(in crate::resident) fn reconcile_exact(
        self,
        custody: &mut dyn OperationCustodyPort,
    ) -> Result<PreDispatchOperationExpiryAnchorPending<'a>, Box<Self>> {
        match self.failure.reconcile_exact(custody) {
            Ok(pending) => Ok(PreDispatchOperationExpiryAnchorPending {
                expiry: self.expiry,
                checkpoint: self.checkpoint,
                custody: ExpiryAnchorCustody::Pending(Box::new(pending)),
            }),
            Err(failure) => Err(Box::new(Self { failure, ..self })),
        }
    }
}

enum ExpiryAnchorCustody {
    Pending(Box<DatabaseCommittedOperationObservationAnchorPending>),
    Failed(OperationObservationCustodyAnchorFailure),
}

/// Exact database custody awaiting only its independent anchor.
#[must_use = "continue only this retained expiry anchor transition"]
pub(in crate::resident) struct PreDispatchOperationExpiryAnchorPending<'a> {
    expiry: &'a PreparedPreDispatchOperationExpiry<'a>,
    checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    custody: ExpiryAnchorCustody,
}

impl<'a> PreDispatchOperationExpiryAnchorPending<'a> {
    /// Borrow the signed expiry retained by the database.
    pub(in crate::resident) const fn expiry(&self) -> &PreparedPreDispatchOperationExpiry<'a> {
        self.expiry
    }

    /// Continue the exact retained independent-anchor transition.
    pub(in crate::resident) fn continue_anchor(
        self,
        anchor: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<CompletedPreDispatchOperationExpiry<'a>, Box<Self>> {
        let Self {
            expiry,
            checkpoint,
            custody,
        } = self;
        let completed = match custody {
            ExpiryAnchorCustody::Pending(pending) => pending.advance_anchor(anchor),
            ExpiryAnchorCustody::Failed(failure) => match failure.try_into_retryable() {
                Ok(pending) => pending.advance_anchor(anchor),
                Err(failure) => failure.continue_after_failed_observation(anchor),
            },
        };
        match completed {
            Ok(custody) => Ok(CompletedPreDispatchOperationExpiry {
                expiry,
                checkpoint,
                custody,
            }),
            Err(failure) => Err(Box::new(Self {
                expiry,
                checkpoint,
                custody: ExpiryAnchorCustody::Failed(failure),
            })),
        }
    }
}

/// Durable expiry custody. No effect owner was called.
pub(in crate::resident) struct CompletedPreDispatchOperationExpiry<'a> {
    expiry: &'a PreparedPreDispatchOperationExpiry<'a>,
    checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    custody: CompletedOperationObservationCustody,
}

impl<'a> CompletedPreDispatchOperationExpiry<'a> {
    /// Borrow the exact signed expiry.
    pub(in crate::resident) const fn expiry(&self) -> &PreparedPreDispatchOperationExpiry<'a> {
        self.expiry
    }

    /// Borrow the checkpoint that contains the expiry.
    pub(in crate::resident) fn checkpoint(
        &self,
    ) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        (&self.checkpoint.0, &self.checkpoint.1)
    }

    /// Borrow the completed database and anchor custody proof.
    pub(in crate::resident) const fn custody(&self) -> &CompletedOperationObservationCustody {
        &self.custody
    }
}

/// Exact custody phase retained when expiry storage stops.
pub(in crate::resident) enum PreDispatchOperationExpiryCustodyFailure<'a> {
    /// The checkpoint was signed once. This continuation accepts no signer.
    Issued {
        continuation: Box<IssuedPreDispatchOperationExpiryCheckpoint<'a>>,
        failure: Box<OperationAppendFailure>,
    },
    /// Commit ambiguity retains the exact physical candidate.
    Commit(Box<PreDispatchOperationExpiryCommitFailure<'a>>),
    /// Database custody is exact and only the anchor remains.
    Anchor(Box<PreDispatchOperationExpiryAnchorPending<'a>>),
}

fn commit_expiry<'a>(
    expiry: &'a PreparedPreDispatchOperationExpiry<'a>,
    identity: OperationCheckpointInput,
    checkpoint: ControlStateCheckpointV1,
    physical: PreparedOperationObservationCustody,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
) -> Result<CompletedPreDispatchOperationExpiry<'a>, PreDispatchOperationExpiryCustodyFailure<'a>> {
    let checkpoint = (identity.reference, checkpoint);
    match commit_prepared_checkpoint(physical, checkpoint, custody, anchor) {
        Ok((custody, checkpoint)) => Ok(CompletedPreDispatchOperationExpiry {
            expiry,
            checkpoint,
            custody,
        }),
        Err(OperationAppendFailure::Commit {
            checkpoint,
            failure,
        }) => Err(PreDispatchOperationExpiryCustodyFailure::Commit(Box::new(
            PreDispatchOperationExpiryCommitFailure {
                expiry,
                checkpoint: *checkpoint,
                failure,
            },
        ))),
        Err(OperationAppendFailure::Anchor {
            checkpoint,
            failure,
        }) => Err(PreDispatchOperationExpiryCustodyFailure::Anchor(Box::new(
            PreDispatchOperationExpiryAnchorPending {
                expiry,
                checkpoint: *checkpoint,
                custody: ExpiryAnchorCustody::Failed(failure),
            },
        ))),
        Err(OperationAppendFailure::Operation(_) | OperationAppendFailure::Projection(_)) => {
            unreachable!("an attached expiry checkpoint has no preparation phase")
        }
    }
}

/// Observe expiry of one exact admitted Invoke or Delegate before dispatch.
///
/// `lineage_has_parent_operation` records whether the process owner's admitted
/// lineage retains a parent coordinate. The value must agree with the handle;
/// this function does not infer lineage authority from the handle alone.
pub(in crate::resident) fn prepare_pre_dispatch_operation_expiry<'operation, Rotation, Sign>(
    recovered: &'operation RecoveredOperationControl,
    lineage_has_parent_operation: bool,
    input: PreDispatchOperationExpiryInput<'_>,
    admit_rotation: Rotation,
    sign: Sign,
) -> Result<PreparedPreDispatchOperationExpiry<'operation>, PreDispatchOperationExpiryError>
where
    Rotation: FnOnce(
        &ResidentRecordRefV1,
        &ArtifactDigest,
        &ResidentRecordRefV1,
        &ArtifactDigest,
    ) -> Result<(), InvalidResidentRecordV1>,
    Sign: FnOnce(
        &[u8],
        &OperationObservationPreimageV1,
        &OperationObservationDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
{
    let operation = recovered.operation().operation();
    let (handle_ref, handle) = operation.handle();
    let predecessor = recovered.observation().selected_head();
    let selected = recovered.observation().selected();
    let checkpoint = recovered.observation().current_checkpoint();
    if !matches!(
        handle.operation_class(),
        OperationClassV1::Invoke | OperationClassV1::Delegate
    ) || handle.parent_operation().is_some() != lineage_has_parent_operation
        || selected.0 != predecessor.0
        || selected.1.canonical_operation_observation_sha256()
            != predecessor.1.canonical_operation_observation_sha256()
        || !matches!(
            predecessor.1.state(),
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::Resolving
                    | OperationActivePhaseV1::ContextBound { .. }
                    | OperationActivePhaseV1::ProviderResolved { .. }
                    | OperationActivePhaseV1::DispatchPrepared { .. }
            }
        )
    {
        return Err(PreDispatchOperationExpiryError::Association);
    }
    for reference in [&input.diagnostic_ref, &input.observation_ref] {
        if reference == handle_ref
            || reference == handle.operation_lease_ref()
            || reference == operation.request_coordinate().record_ref()
            || reference == predecessor.0
            || Some(reference) == predecessor.1.predecessor_ref().as_ref()
            || reference == checkpoint.0
        {
            return Err(PreDispatchOperationExpiryError::Association);
        }
    }
    if input.diagnostic_ref == input.observation_ref || input.maximum_diagnostic_bytes == 0 {
        return Err(PreDispatchOperationExpiryError::Association);
    }

    let observed_at = input.clock.observe_unix_time()?;
    if observed_at < *handle.expires_at() {
        return Err(PreDispatchOperationExpiryError::NotExpired);
    }
    if observed_at < *predecessor.1.observed_at() || observed_at < *checkpoint.1.issued_at() {
        return Err(PreDispatchOperationExpiryError::TimeOrder);
    }
    if observed_at > *handle.retain_until() {
        return Err(PreDispatchOperationExpiryError::RetentionElapsed);
    }

    let identity = pale_contracts::record_schema_identity("diagnostic.v1")
        .map_err(DiagnosticAdmissionError::Registry)?;
    let diagnostic = DiagnosticProjection {
        code: "operation_expired_before_dispatch".to_string(),
        stage: DiagnosticStageV1::OperationAdmission,
        disposition: DiagnosticDispositionV1::Blocked,
        summary: "operation expired before dispatch".to_string(),
        subject: Some((
            "operation".to_string(),
            handle.operation_handle_id().as_str().to_string(),
        )),
        source: None,
        schema_reference: None,
        causes: Vec::new(),
        notes: Vec::new(),
        help: Vec::new(),
        facts: Vec::new(),
        retry: DiagnosticRetryV1::NotIndicated,
        effect: DiagnosticEffectCertaintyV1::NotStarted,
        request: Some((
            "operation_request".to_string(),
            recovered
                .operation()
                .request()
                .canonical_request_sha256()
                .as_str()
                .to_string(),
        )),
        generation: None,
        receipt: None,
        omitted_causes: 0,
    }
    .admit(&identity)?;
    if diagnostic.canonical_bytes().len() > input.maximum_diagnostic_bytes {
        return Err(PreDispatchOperationExpiryError::BoundExceeded);
    }
    let (diagnostic_digest, diagnostic_record) =
        crate::resident::record_projection::diagnostic_record(&input.diagnostic_ref, &diagnostic)
            .map_err(OperationObservationCustodyProjectionError::from)
            .map_err(OperationAttemptError::from)?;
    let observation = signing::issue_observation(
        &pale_core::records::OperationObservationAdmissionContextV1 {
            signed: input.signing,
            handle_ref,
            handle,
            predecessor: Some(predecessor),
        },
        OperationObservationStateV1::Expired {
            diagnostic: pale_core::records::ImmutableRecordReferenceV1 {
                record_ref: input.diagnostic_ref,
                record_sha256: diagnostic_digest,
            },
        },
        observed_at,
        admit_rotation,
        sign,
    )?;
    Ok(PreparedPreDispatchOperationExpiry {
        operation,
        checkpoint: (checkpoint.0.clone(), checkpoint.1.clone()),
        predecessor: (predecessor.0.clone(), predecessor.1.clone()),
        diagnostic,
        diagnostic_record,
        observation_ref: input.observation_ref,
        observation,
        observed_at,
    })
}
