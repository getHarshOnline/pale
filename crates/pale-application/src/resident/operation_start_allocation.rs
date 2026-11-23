//! Allocate the private identities needed by one durable operation start.
//!
//! The signed request keeps its caller-owned identity. This owner first checks
//! whether that exact request already started an operation. Only an absent
//! start permits fresh lease, handle, observation, and checkpoint allocation.
//! The allocated values remain preparation inputs; they grant no authority and
//! expose no operation before the existing commit and anchor path completes.

use std::collections::BTreeSet;

use pale_core::records::{
    CommonRequestEnvelopeV1, ControlCheckpointIdV1, InvalidResidentRecordV1, OperationHandleIdV1,
    OperationLeaseIdV1, ResidentRecordRefV1,
};
use pale_ports::{
    AuthorityStoreCoordinate, CryptographicRandomSourceError, CryptographicRandomSourcePort,
    OperationCustodyPort,
};

use super::operation_custody::{
    read_operation_start_for_request, OperationRecoveryError, OperationStartRecoveryEvidence,
};

const RANDOM_BYTES: usize = 32;
const MAXIMUM_RANDOM_DRAWS: usize = 16;

const LEASE_ID_DOMAIN: &[u8] = b"pale:operation-start:lease-id:v1\0";
const LEASE_REF_DOMAIN: &[u8] = b"pale:operation-start:lease-record-ref:v1\0";
const HANDLE_ID_DOMAIN: &[u8] = b"pale:operation-start:handle-id:v1\0";
const HANDLE_REF_DOMAIN: &[u8] = b"pale:operation-start:handle-record-ref:v1\0";
const OBSERVATION_REF_DOMAIN: &[u8] = b"pale:operation-start:observation-record-ref:v1\0";
const CHECKPOINT_ID_DOMAIN: &[u8] = b"pale:operation-start:checkpoint-id:v1\0";
const CHECKPOINT_REF_DOMAIN: &[u8] = b"pale:operation-start:checkpoint-record-ref:v1\0";

/// Result of checking one exact request before allocating start identities.
pub(super) enum OperationStartAllocation {
    /// The request already owns a durable start. No entropy was consumed.
    Existing(Box<OperationStartRecoveryEvidence>),
    /// No start exists and one fresh, single-use identity bundle was allocated.
    Fresh(Box<FreshOperationStartAllocation>),
}

/// Private identities for one new operation-start candidate.
///
/// This value contains no request ID, attempt ID, remote identity, authority,
/// signature, or durable-start proof. Its store coordinate scopes the raw
/// spellings to the exact deployment-control incarnation checked before
/// allocation.
pub(super) struct FreshOperationStartAllocation {
    store: AuthorityStoreCoordinate,
    request_ref: ResidentRecordRefV1,
    lease_id: OperationLeaseIdV1,
    lease_ref: ResidentRecordRefV1,
    handle_id: OperationHandleIdV1,
    handle_ref: ResidentRecordRefV1,
    observation_ref: ResidentRecordRefV1,
    checkpoint_id: ControlCheckpointIdV1,
    checkpoint_ref: ResidentRecordRefV1,
}

impl FreshOperationStartAllocation {
    pub(super) const fn store(&self) -> &AuthorityStoreCoordinate {
        &self.store
    }

    pub(super) const fn request_ref(&self) -> &ResidentRecordRefV1 {
        &self.request_ref
    }

    pub(super) const fn lease_id(&self) -> &OperationLeaseIdV1 {
        &self.lease_id
    }

    pub(super) const fn lease_ref(&self) -> &ResidentRecordRefV1 {
        &self.lease_ref
    }

    pub(super) const fn handle_id(&self) -> &OperationHandleIdV1 {
        &self.handle_id
    }

    pub(super) const fn handle_ref(&self) -> &ResidentRecordRefV1 {
        &self.handle_ref
    }

    pub(super) const fn observation_ref(&self) -> &ResidentRecordRefV1 {
        &self.observation_ref
    }

    pub(super) const fn checkpoint_id(&self) -> &ControlCheckpointIdV1 {
        &self.checkpoint_id
    }

    pub(super) const fn checkpoint_ref(&self) -> &ResidentRecordRefV1 {
        &self.checkpoint_ref
    }
}

/// Operation-start allocation stopped before a start candidate was built.
#[derive(Debug, thiserror::Error)]
pub(super) enum OperationStartAllocationError {
    /// Existing request custody could not be classified exactly.
    #[error("operation-start request lookup failed: {0}")]
    Recovery(#[from] OperationRecoveryError),
    /// The selected cryptographic random source failed.
    #[error(transparent)]
    Random(#[from] CryptographicRandomSourceError),
    /// The bounded source repeatedly returned a nonce already used in this bundle.
    #[error("operation-start identity allocation exhausted its random-draw ceiling")]
    RandomDrawsExhausted,
    /// A fixed private spelling no longer fits its owning initial-v1 identity type.
    #[error("operation-start identity representation is invalid: {0}")]
    Identity(#[from] InvalidResidentRecordV1),
}

/// Check one request and allocate a fresh identity bundle only when it is absent.
///
/// The exact request lookup always precedes the first random-source call.
/// Existing or conflicting request custody therefore cannot consume entropy or
/// create another candidate. Returned fresh values still require semantic
/// admission, lease retention, atomic start commitment, and independent anchor
/// advancement through their existing owners.
pub(super) fn begin_operation_start_allocation(
    custody: &mut dyn OperationCustodyPort,
    random: &dyn CryptographicRandomSourcePort,
    store: &AuthorityStoreCoordinate,
    request: &CommonRequestEnvelopeV1,
    maximum_record_bytes: usize,
) -> Result<OperationStartAllocation, OperationStartAllocationError> {
    if let Some(existing) =
        read_operation_start_for_request(custody, store, request, maximum_record_bytes)?
    {
        return Ok(OperationStartAllocation::Existing(Box::new(existing)));
    }

    allocate_fresh_operation_start(random, store, request)
        .map(Box::new)
        .map(OperationStartAllocation::Fresh)
}

fn allocate_fresh_operation_start(
    random: &dyn CryptographicRandomSourcePort,
    store: &AuthorityStoreCoordinate,
    request: &CommonRequestEnvelopeV1,
) -> Result<FreshOperationStartAllocation, OperationStartAllocationError> {
    let request_ref = ResidentRecordRefV1::parse(format!(
        "operation-request-record:v1:{}",
        request.canonical_request_sha256().as_str()
    ))?;
    let mut nonces = NoncePool::default();
    Ok(FreshOperationStartAllocation {
        store: store.clone(),
        request_ref,
        lease_id: OperationLeaseIdV1::parse(nonces.allocate(
            random,
            LEASE_ID_DOMAIN,
            "operation-lease:v1",
        )?)?,
        lease_ref: ResidentRecordRefV1::parse(nonces.allocate(
            random,
            LEASE_REF_DOMAIN,
            "operation-lease-record:v1",
        )?)?,
        handle_id: OperationHandleIdV1::parse(nonces.allocate(
            random,
            HANDLE_ID_DOMAIN,
            "operation-handle:v1",
        )?)?,
        handle_ref: ResidentRecordRefV1::parse(nonces.allocate(
            random,
            HANDLE_REF_DOMAIN,
            "operation-handle-record:v1",
        )?)?,
        observation_ref: ResidentRecordRefV1::parse(nonces.allocate(
            random,
            OBSERVATION_REF_DOMAIN,
            "operation-observation-record:v1",
        )?)?,
        checkpoint_id: ControlCheckpointIdV1::parse(nonces.allocate(
            random,
            CHECKPOINT_ID_DOMAIN,
            "operation-checkpoint:v1",
        )?)?,
        checkpoint_ref: ResidentRecordRefV1::parse(nonces.allocate(
            random,
            CHECKPOINT_REF_DOMAIN,
            "operation-checkpoint-record:v1",
        )?)?,
    })
}

#[derive(Default)]
struct NoncePool {
    seen: BTreeSet<[u8; RANDOM_BYTES]>,
    draws: usize,
}

impl NoncePool {
    fn allocate(
        &mut self,
        random: &dyn CryptographicRandomSourcePort,
        domain: &[u8],
        prefix: &str,
    ) -> Result<String, OperationStartAllocationError> {
        loop {
            if self.draws == MAXIMUM_RANDOM_DRAWS {
                return Err(OperationStartAllocationError::RandomDrawsExhausted);
            }
            self.draws += 1;
            let mut nonce = [0_u8; RANDOM_BYTES];
            random.fill_bytes(&mut nonce)?;
            if !self.seen.insert(nonce) {
                continue;
            }
            let mut preimage = Vec::with_capacity(domain.len() + nonce.len());
            preimage.extend_from_slice(domain);
            preimage.extend_from_slice(&nonce);
            return Ok(format!("{prefix}:{}", pale_core::sha256_hex(&preimage)));
        }
    }
}

#[cfg(test)]
#[path = "../../tests/internal/resident/operation_start_allocation.rs"]
pub(in crate::resident) mod tests;
