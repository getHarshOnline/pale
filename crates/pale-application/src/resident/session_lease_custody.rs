//! Canonical retention and historical re-admission of session-lease bodies.
//!
//! Lifecycle state names a lease only by digest. This owner retains the exact
//! wire before that digest can enter the journal, then re-admits recovered bytes
//! against the historical generation, profile, active head, and fence selected
//! by the recovered lease itself. It does not reissue a lease or establish
//! present ambient authority, freshness, native readiness, or continuation.

use pale_core::canon;
use pale_core::records::{
    SessionAdmissionLeaseContextV1, SessionAdmissionLeaseDigest, SessionAdmissionLeaseV1,
    SessionAdmissionLeaseWireV1, SignedResidentAdmissionContextV1,
    SESSION_ADMISSION_LEASE_V1_SCHEMA_ID,
};
use pale_ports::{
    AuthorityStoreCoordinate, ControlRecordBytes, ControlStoreErrorKind, SessionLeaseBody,
    SessionLeaseBodyRead, SessionLeaseBodyRetention, SessionLeaseCustodyPort,
};

use super::custody_phase::{
    commit_with_exact_readback, DatabaseCommitDisposition, DatabaseCommitEvidence,
    DatabaseCommitFailureReason, DatabaseReadBackDisposition,
};
use super::lifecycle::{GenerationLifecycleError, GenerationLifecycleRecoveryResolver};
use super::session_admission::{
    self, AdmittedAmbientSession, RetainedAmbientSession, SessionAdmissionBasisCustodyError,
};

/// Exact physical custody parameters shared by checkpoint and tail replay.
pub(super) struct SessionLeaseRecoveryContext<'a> {
    port: &'a dyn SessionLeaseCustodyPort,
    store: AuthorityStoreCoordinate,
    maximum_body_bytes: usize,
}

impl<'a> SessionLeaseRecoveryContext<'a> {
    /// Bind recovery to one store incarnation and explicit hard byte ceiling.
    #[must_use]
    pub(super) const fn new(
        port: &'a dyn SessionLeaseCustodyPort,
        store: AuthorityStoreCoordinate,
        maximum_body_bytes: usize,
    ) -> Self {
        Self {
            port,
            store,
            maximum_body_bytes,
        }
    }

    /// Recover and re-admit one digest through the historical owner context.
    pub(super) fn recover(
        &self,
        digest: &SessionAdmissionLeaseDigest,
        resolver: &dyn GenerationLifecycleRecoveryResolver,
    ) -> Result<SessionAdmissionLeaseV1, GenerationLifecycleError> {
        recover_session_lease_body(
            self.port,
            &self.store,
            digest,
            self.maximum_body_bytes,
            resolver,
        )
    }
}

/// Prepare and retain one admitted lease body before its digest is journaled.
pub(super) fn retain_session_lease_body(
    port: &dyn SessionLeaseCustodyPort,
    store: &AuthorityStoreCoordinate,
    lease: &SessionAdmissionLeaseV1,
    maximum_body_bytes: usize,
) -> Result<DatabaseCommitEvidence, DatabaseCommitFailureReason> {
    let canonical = canon::to_canonical_string(&SessionAdmissionLeaseWireV1::from(lease)).map_err(
        |source| {
            DatabaseCommitFailureReason::Store(pale_ports::ControlStoreError::with_source(
                ControlStoreErrorKind::RecoveryRequired,
                source,
            ))
        },
    )?;
    if canonical.len() > maximum_body_bytes {
        return Err(DatabaseCommitFailureReason::Store(
            pale_ports::ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired),
        ));
    }
    let bytes = ControlRecordBytes::new(canonical.into_bytes()).map_err(|source| {
        DatabaseCommitFailureReason::Store(pale_ports::ControlStoreError::with_source(
            ControlStoreErrorKind::RecoveryRequired,
            source,
        ))
    })?;
    let body = SessionLeaseBody::new(
        store.clone(),
        lease.session_admission_lease_digest().clone(),
        lease.schema_id().clone(),
        lease.schema_sha256().clone(),
        bytes,
    );
    let mut custody = port;
    commit_with_exact_readback(
        &mut custody,
        &body,
        |custody, candidate| {
            custody
                .compare_and_retain(candidate)
                .map(|outcome| match outcome {
                    SessionLeaseBodyRetention::Retained => DatabaseCommitDisposition::Committed,
                    SessionLeaseBodyRetention::ExactReplay => {
                        DatabaseCommitDisposition::ExactReplay
                    }
                })
        },
        |custody, candidate| {
            let request = SessionLeaseBodyRead::new(
                candidate.store().clone(),
                candidate.digest().clone(),
                maximum_body_bytes,
            );
            custody.read(&request).map(|observed| match observed {
                None => DatabaseReadBackDisposition::Predecessor,
                Some(observed) if observed == *candidate => {
                    DatabaseReadBackDisposition::ExactSuccessor
                }
                Some(_) => DatabaseReadBackDisposition::ConflictingOrPartial,
            })
        },
    )
}

/// Retain one admitted lease and its private revalidation basis as one body.
pub(super) fn retain_admitted_session_body(
    port: &dyn SessionLeaseCustodyPort,
    store: &AuthorityStoreCoordinate,
    session: &AdmittedAmbientSession,
    maximum_body_bytes: usize,
) -> Result<DatabaseCommitEvidence, DatabaseCommitFailureReason> {
    let lease = session.lease();
    let canonical = canon::to_canonical_string(&SessionAdmissionLeaseWireV1::from(lease)).map_err(
        |source| {
            DatabaseCommitFailureReason::Store(pale_ports::ControlStoreError::with_source(
                ControlStoreErrorKind::RecoveryRequired,
                source,
            ))
        },
    )?;
    if canonical.len() > maximum_body_bytes {
        return Err(DatabaseCommitFailureReason::Store(
            pale_ports::ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired),
        ));
    }
    let bytes = ControlRecordBytes::new(canonical.into_bytes()).map_err(|source| {
        DatabaseCommitFailureReason::Store(pale_ports::ControlStoreError::with_source(
            ControlStoreErrorKind::RecoveryRequired,
            source,
        ))
    })?;
    let basis = session_admission::encode_admission_basis(session, maximum_body_bytes).map_err(
        |source| {
            DatabaseCommitFailureReason::Store(pale_ports::ControlStoreError::with_source(
                ControlStoreErrorKind::RecoveryRequired,
                source,
            ))
        },
    )?;
    if bytes
        .as_slice()
        .len()
        .checked_add(basis.as_slice().len())
        .is_none_or(|total| total > maximum_body_bytes)
    {
        return Err(DatabaseCommitFailureReason::Store(
            pale_ports::ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired),
        ));
    }
    let body = SessionLeaseBody::new(
        store.clone(),
        lease.session_admission_lease_digest().clone(),
        lease.schema_id().clone(),
        lease.schema_sha256().clone(),
        bytes,
    )
    .with_private_admission_basis(basis);
    let mut custody = port;
    commit_with_exact_readback(
        &mut custody,
        &body,
        |custody, candidate| {
            custody
                .compare_and_retain(candidate)
                .map(|outcome| match outcome {
                    SessionLeaseBodyRetention::Retained => DatabaseCommitDisposition::Committed,
                    SessionLeaseBodyRetention::ExactReplay => {
                        DatabaseCommitDisposition::ExactReplay
                    }
                })
        },
        |custody, candidate| {
            let request = SessionLeaseBodyRead::new(
                candidate.store().clone(),
                candidate.digest().clone(),
                maximum_body_bytes,
            );
            custody.read(&request).map(|observed| match observed {
                None => DatabaseReadBackDisposition::Predecessor,
                Some(observed) if observed == *candidate => {
                    DatabaseReadBackDisposition::ExactSuccessor
                }
                Some(_) => DatabaseReadBackDisposition::ConflictingOrPartial,
            })
        },
    )
}

/// Recover the exact private basis paired with one historically admitted lease.
pub(super) fn recover_retained_session(
    port: &dyn SessionLeaseCustodyPort,
    store: &AuthorityStoreCoordinate,
    digest: &SessionAdmissionLeaseDigest,
    maximum_body_bytes: usize,
    resolver: &dyn GenerationLifecycleRecoveryResolver,
) -> Result<RetainedAmbientSession, RetainedSessionRecoveryError> {
    let lease = recover_session_lease_body(port, store, digest, maximum_body_bytes, resolver)?;
    let request = SessionLeaseBodyRead::new(store.clone(), digest.clone(), maximum_body_bytes);
    let body = port
        .read(&request)?
        .ok_or(RetainedSessionRecoveryError::MissingBody)?;
    if body.store() != store || body.digest() != digest {
        return Err(RetainedSessionRecoveryError::BodySubstitution);
    }
    session_admission::decode_admission_basis(&body, &lease, maximum_body_bytes).map_err(Into::into)
}

/// Exact ambient-session recovery failed before current revalidation.
#[derive(Debug, thiserror::Error)]
pub(in crate::resident) enum RetainedSessionRecoveryError {
    /// Historical lease recovery or admission failed.
    #[error(transparent)]
    Historical(#[from] GenerationLifecycleError),
    /// Physical custody could not read the paired body.
    #[error(transparent)]
    Store(#[from] pale_ports::ControlStoreError),
    /// The lease body disappeared between exact historical reads.
    #[error("retained session lease body is missing")]
    MissingBody,
    /// Physical custody returned a body from another store or digest.
    #[error("retained session lease body coordinate was substituted")]
    BodySubstitution,
    /// The private admission basis was absent, malformed, or misbound.
    #[error(transparent)]
    Basis(#[from] SessionAdmissionBasisCustodyError),
}

/// Recover and contextually re-admit one exact historical lease body.
pub(super) fn recover_session_lease_body(
    port: &dyn SessionLeaseCustodyPort,
    store: &AuthorityStoreCoordinate,
    digest: &SessionAdmissionLeaseDigest,
    maximum_body_bytes: usize,
    resolver: &dyn GenerationLifecycleRecoveryResolver,
) -> Result<SessionAdmissionLeaseV1, GenerationLifecycleError> {
    let request = SessionLeaseBodyRead::new(store.clone(), digest.clone(), maximum_body_bytes);
    let body = port
        .read(&request)
        .map_err(GenerationLifecycleError::SessionLeaseCustody)?
        .ok_or(GenerationLifecycleError::MissingRecoveryContext(
            "retained session lease body",
        ))?;
    if body.store() != store || body.digest() != digest {
        return Err(GenerationLifecycleError::RecoveryContextSubstitution(
            "retained session lease body",
        ));
    }
    if body.canonical_wire_jcs().as_slice().len() > maximum_body_bytes {
        return Err(GenerationLifecycleError::SessionLeaseBoundExceeded);
    }
    let expected = pale_contracts::record_schema_identity(SESSION_ADMISSION_LEASE_V1_SCHEMA_ID)
        .map_err(GenerationLifecycleError::SessionLeaseContract)?;
    if body.schema_id() != &expected.schema || body.schema_sha256() != &expected.schema_sha256 {
        return Err(GenerationLifecycleError::RecoveryContextSubstitution(
            "retained session lease schema",
        ));
    }
    let value = canon::parse_strict(body.canonical_wire_jcs().as_slice())
        .map_err(GenerationLifecycleError::SessionLeaseCanonical)?;
    let wire: SessionAdmissionLeaseWireV1 =
        serde_json::from_value(value).map_err(GenerationLifecycleError::SessionLeaseDecode)?;
    let canonical = canon::to_canonical_string(&wire)
        .map_err(GenerationLifecycleError::SessionLeaseCanonical)?;
    if canonical.as_bytes() != body.canonical_wire_jcs().as_slice() {
        return Err(GenerationLifecycleError::SessionLeaseNonCanonical);
    }
    if wire.session_admission_lease_digest() != digest
        || wire.schema_id() != body.schema_id()
        || wire.schema_sha256() != body.schema_sha256()
    {
        return Err(GenerationLifecycleError::RecoveryContextSubstitution(
            "retained session lease body",
        ));
    }
    let active_head = resolver
        .active_head(wire.active_generation_head_digest())
        .ok_or(GenerationLifecycleError::MissingRecoveryContext(
            "session active generation head",
        ))?;
    let compiled_generation = resolver
        .compiled_generation(wire.compiled_generation_digest())
        .ok_or(GenerationLifecycleError::MissingRecoveryContext(
            "session compiled generation",
        ))?;
    let effective_profile = resolver.effective_profile(wire.profile_digest()).ok_or(
        GenerationLifecycleError::MissingRecoveryContext("session effective profile"),
    )?;
    let activation_fence = resolver
        .activation_fence(wire.activation_fence_digest())
        .ok_or(GenerationLifecycleError::MissingRecoveryContext(
            "session activation fence",
        ))?;
    let signing_authority = resolver
        .session_signing_authority(
            wire.deployment_runtime_authority_ref(),
            wire.deployment_runtime_authority_sha256(),
            wire.issuer_principal_id(),
            wire.issuer_key_id(),
            *wire.issuer_key_epoch(),
        )
        .ok_or(GenerationLifecycleError::MissingRecoveryContext(
            "session deployment runtime authority",
        ))?;
    let admitted = SessionAdmissionLeaseV1::admit_contextual_wire(
        wire,
        &SessionAdmissionLeaseContextV1 {
            active_head,
            compiled_generation,
            effective_profile,
            activation_fence,
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &expected.schema_sha256,
                signing_authority: &signing_authority,
            },
        },
        |domain, preimage, observed_digest, signature| {
            resolver.verify_session_admission_lease_signature(
                &signing_authority,
                domain,
                preimage,
                observed_digest,
                signature,
            )
        },
    )
    .map_err(GenerationLifecycleError::SessionLeaseAdmission)?;
    if admitted.session_admission_lease_digest() != digest {
        return Err(GenerationLifecycleError::RecoveryContextSubstitution(
            "retained session lease body",
        ));
    }
    Ok(admitted)
}
