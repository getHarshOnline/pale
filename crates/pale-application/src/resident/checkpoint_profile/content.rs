//! Closed content and exact request association for operation checkpoint preparation.
//!
//! This is not a complete principal/deployment profile, authority delegation,
//! durable operation, or a choice of signing/anchor backend.

use pale_core::{
    canon,
    records::{
        CheckpointProfileDigest, CommonRequestEnvelopeV1, ControlCheckpointDomainIdV1,
        ControlDomainEpochV1, ControlStateCheckpointV1, InvalidResidentRecordV1, OperationHandleV1,
        RequestBindingV1, ResidentRecordRefV1,
    },
};
use pale_ports::{
    AccumulatorImplementation, AccumulatorInitialDomain, AccumulatorPreparationBasis,
    AuthorityStoreCoordinate,
};
use serde::Serialize;

use super::{
    CONTROL_SCHEMA_VERSION, HANDLE_VALUE_DOMAIN, INITIAL_DOMAIN_EPOCH, KEY_DOMAIN,
    REQUEST_VALUE_DOMAIN, SCOPE_DOMAIN, VALUE_DOMAIN,
};
use crate::resident::operation_custody::{operation_request_key, OperationStartRecoveryEvidence};

const CONTENT_LIMIT: usize = 4096;
const IMPLEMENTATION: AccumulatorImplementation =
    AccumulatorImplementation::SparseMerkleTree061Sha256PaleV1;

#[derive(Serialize)]
struct Content {
    content: &'static str,
    control_schema_version: u32,
    native_implementation: &'static str,
    conflict_domain: &'static str,
    scope_encoding: &'static str,
    key_encoding: &'static str,
    initial_domain_epoch: u64,
    request_value_encoding: &'static str,
    handle_value_encoding: &'static str,
    observation_value_encoding: &'static str,
    a2a_value_encoding: &'static str,
    session_basis_value_encoding: &'static str,
    session_basis_shape_sha256: pale_core::records::ArtifactDigest,
    supported_mutation: &'static str,
    unresolved_custody: &'static str,
    recovery_indexes: &'static str,
    present_zero: &'static str,
}

fn content() -> Result<Content, canon::CanonError> {
    Ok(Content {
        content: "pale/operation-checkpoint-profile.v1",
        control_schema_version: CONTROL_SCHEMA_VERSION,
        native_implementation: IMPLEMENTATION.as_str(),
        conflict_domain: "root-request-index-identity-inherited-by-child-v1",
        scope_encoding: SCOPE_DOMAIN,
        key_encoding: KEY_DOMAIN,
        initial_domain_epoch: INITIAL_DOMAIN_EPOCH,
        request_value_encoding: REQUEST_VALUE_DOMAIN,
        handle_value_encoding: HANDLE_VALUE_DOMAIN,
        observation_value_encoding: VALUE_DOMAIN,
        a2a_value_encoding: super::a2a::A2A_VALUE_DOMAIN,
        session_basis_value_encoding: super::session_basis::SESSION_BASIS_VALUE_DOMAIN,
        session_basis_shape_sha256:
            crate::resident::local_operation_start::session_basis::shape_digest()?,
        supported_mutation: "initial-root-and-inherited-child-request-handle-session-basis-observation-insertion-retained-observation-replacement-a2a-send-preparation-route-and-evidence-insertion-and-task-history-replacement",
        unresolved_custody: "initial-empty-then-unchanged",
        recovery_indexes: "derived-excluded-checkpoint-self-reference",
        present_zero: "reject-reserved-absence",
    })
}

/// Why immutable content or its exact operation association could not be admitted.
#[derive(Debug, thiserror::Error)]
pub enum OperationCheckpointProfileError {
    /// The bounded descriptor is malformed or selects another implementation.
    #[error("checkpoint profile content is unsupported or exceeds its byte limit")]
    Unsupported,
    /// The canonical descriptor does not have the committed digest.
    #[error("checkpoint profile content does not match its committed digest")]
    Digest,
    /// The exact request, store, profile or checkpoint association differs.
    #[error("checkpoint profile does not belong to this operation and checkpoint")]
    Association,
    /// Canonical byte validation failed.
    #[error(transparent)]
    Canonicalize(#[from] canon::CanonError),
    /// An admitted local request binding could not be projected.
    #[error(transparent)]
    Request(#[from] InvalidResidentRecordV1),
}

/// One source-supported immutable descriptor. This grants no signing authority.
#[derive(Debug, Clone)]
pub struct SupportedOperationCheckpointProfile {
    reference: ResidentRecordRefV1,
    digest: CheckpointProfileDigest,
}

impl SupportedOperationCheckpointProfile {
    pub(in crate::resident) const fn coordinates(
        &self,
    ) -> (&ResidentRecordRefV1, &CheckpointProfileDigest) {
        (&self.reference, &self.digest)
    }

    /// Render the single supported descriptor from its implementation owners.
    ///
    /// # Errors
    /// Returns an error if the source description cannot be canonically encoded.
    pub fn canonical_content() -> Result<String, canon::CanonError> {
        canon::to_canonical_string(&content()?)
    }

    /// Compute the descriptor's existing typed canonical identity.
    ///
    /// # Errors
    /// Returns an error if canonical encoding fails.
    pub fn content_digest() -> Result<CheckpointProfileDigest, canon::CanonError> {
        CheckpointProfileDigest::hash_canonical(&content()?)
    }

    /// Admit bounded bytes only when their digest and exact supported content agree.
    ///
    /// Reference resolution and signer authority are separate caller obligations.
    /// No external I/O, public schema registration or mutable profile lookup occurs.
    ///
    /// # Errors
    /// Rejects oversized, duplicate-key, unsupported or digest-mismatched content.
    pub fn admit(
        reference: ResidentRecordRefV1,
        expected: &CheckpointProfileDigest,
        bytes: &[u8],
    ) -> Result<Self, OperationCheckpointProfileError> {
        if bytes.is_empty() || bytes.len() > CONTENT_LIMIT {
            return Err(OperationCheckpointProfileError::Unsupported);
        }
        let value = canon::parse_strict(bytes)?;
        let digest = CheckpointProfileDigest::hash_canonical(&value)?;
        if &digest != expected {
            return Err(OperationCheckpointProfileError::Digest);
        }
        if canon::to_canonical_string(&value)? != Self::canonical_content()? {
            return Err(OperationCheckpointProfileError::Unsupported);
        }
        Ok(Self { reference, digest })
    }

    /// Derive a root request's stable conflict domain before resolving its signer.
    /// Child starts inherit their protected parent's domain instead; they do
    /// not call this function with a child request to establish another domain.
    /// Store/incarnation independently scope the result. This grants no authority.
    ///
    /// # Errors
    /// Rejects a request whose existing identity key cannot be encoded.
    pub fn conflict_domain(
        request: &CommonRequestEnvelopeV1,
    ) -> Result<ControlCheckpointDomainIdV1, OperationCheckpointProfileError> {
        let key = operation_request_key(request)
            .map_err(|_| OperationCheckpointProfileError::Association)?;
        ControlCheckpointDomainIdV1::parse(format!("operation:{}", key.as_str()))
            .map_err(Into::into)
    }

    pub(in crate::resident) fn select(
        &self,
        store: &AuthorityStoreCoordinate,
        request: &CommonRequestEnvelopeV1,
    ) -> Result<OperationCheckpointSelection, OperationCheckpointProfileError> {
        // Store/incarnation already scope this identifier at every checkpoint and
        // index boundary. Contents and freshly allocated handles are not inputs.
        let domain = Self::conflict_domain(request)?;
        Ok(OperationCheckpointSelection {
            profile: self.clone(),
            store: store.clone(),
            domain,
            request_binding: request.local_request_binding()?,
        })
    }

    /// Recover the original selection from exact stored evidence and re-admitted records.
    ///
    /// The supplied typed records must have passed their contextual admission.
    /// Exact bytes are checked against the existing recovery owner's evidence;
    /// neither the handle issuer nor a replacement request reconstructs the caller.
    /// This returns no current execution or signing authority.
    ///
    /// # Errors
    /// Rejects unlike original request, handle, checkpoint, store or profile.
    pub fn recover_selection(
        &self,
        evidence: &OperationStartRecoveryEvidence,
        request: &CommonRequestEnvelopeV1,
        handle: &OperationHandleV1,
        checkpoint: &ControlStateCheckpointV1,
    ) -> Result<OperationCheckpointSelection, OperationCheckpointProfileError> {
        for (stored, actual) in [
            (evidence.request(), canon::to_canonical_string(request)?),
            (evidence.handle(), canon::to_canonical_string(handle)?),
            (
                evidence.checkpoint(),
                canon::to_canonical_string(checkpoint)?,
            ),
        ] {
            if stored.canonical_record_jcs().as_slice() != actual.as_bytes() {
                return Err(OperationCheckpointProfileError::Association);
            }
        }
        let selected = self.select(evidence.store(), request)?;
        selected.verify(evidence.store(), handle, checkpoint)?;
        Ok(selected)
    }
}

/// Retained exact request/profile association, not a lease or issued handle.
///
/// Creation is confined to contextual start or exact original-record recovery.
#[derive(Clone)]
pub struct OperationCheckpointSelection {
    profile: SupportedOperationCheckpointProfile,
    store: AuthorityStoreCoordinate,
    domain: ControlCheckpointDomainIdV1,
    request_binding: RequestBindingV1,
}

impl std::fmt::Debug for OperationCheckpointSelection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OperationCheckpointSelection")
            .finish_non_exhaustive()
    }
}

impl OperationCheckpointSelection {
    /// Match the retained physical store before allowing a caller-directed read.
    pub(in crate::resident) fn matches_store(&self, store: &AuthorityStoreCoordinate) -> bool {
        &self.store == store
    }

    /// Children insert their own immutable identity in the parent's conflict domain.
    pub(in crate::resident) fn child(
        &self,
        profile: &SupportedOperationCheckpointProfile,
        request: &CommonRequestEnvelopeV1,
    ) -> Result<Self, OperationCheckpointProfileError> {
        if self.profile.reference != profile.reference || self.profile.digest != profile.digest {
            return Err(OperationCheckpointProfileError::Association);
        }
        Ok(Self {
            profile: self.profile.clone(),
            store: self.store.clone(),
            domain: self.domain.clone(),
            request_binding: request.local_request_binding()?,
        })
    }

    pub(in crate::resident) fn initial_basis(
        &self,
    ) -> Result<AccumulatorPreparationBasis, OperationCheckpointProfileError> {
        Ok(AccumulatorPreparationBasis::Initial(
            AccumulatorInitialDomain::new(
                self.store.clone(),
                self.profile.reference.clone(),
                self.profile.digest.clone(),
                self.domain.clone(),
                ControlDomainEpochV1::new(INITIAL_DOMAIN_EPOCH)?,
            ),
        ))
    }

    pub(in crate::resident) fn verify(
        &self,
        store: &AuthorityStoreCoordinate,
        handle: &OperationHandleV1,
        checkpoint: &ControlStateCheckpointV1,
    ) -> Result<(), OperationCheckpointProfileError> {
        if !self.matches_store(store)
            || handle.request_binding() != &self.request_binding
            || checkpoint.control_store_id() != store.store_id()
            || checkpoint.control_incarnation_id() != store.incarnation_id()
            || checkpoint.checkpoint_domain_id() != &self.domain
            || checkpoint.domain_epoch().get() != INITIAL_DOMAIN_EPOCH
            || checkpoint.checkpoint_profile_ref() != &self.profile.reference
            || checkpoint.checkpoint_profile_sha256() != &self.profile.digest
            || checkpoint.control_schema_version() != &CONTROL_SCHEMA_VERSION
        {
            return Err(OperationCheckpointProfileError::Association);
        }
        Ok(())
    }

    pub(in crate::resident) const fn implementation(&self) -> AccumulatorImplementation {
        IMPLEMENTATION
    }

    pub(in crate::resident) fn matches_authority(
        &self,
        authority: &pale_core::records::ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    ) -> bool {
        authority.control_store_id == self.store.store_id()
            && authority.control_incarnation_id == self.store.incarnation_id()
            && authority.checkpoint_domain_id == &self.domain
            && authority.checkpoint_profile_ref == &self.profile.reference
            && authority.checkpoint_profile_sha256 == &self.profile.digest
    }
}
