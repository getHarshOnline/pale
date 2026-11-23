//! Private custody for the admission basis paired with one session lease.
//!
//! The signed lease remains the public authority record. This private body
//! preserves the exact grant, checkpoint, and route evidence needed to recheck
//! the same admission after restart. Decoding never makes a session current.

use pale_core::{canon, records::*};
use pale_ports::{ControlRecordBytes, ControlRecordError, SessionLeaseBody};
use serde::{Deserialize, Serialize};

use super::{AdmittedAmbientSession, AmbientSessionBasis, RetainedAmbientSession};

const FORMAT: &str = "pale-session-admission-basis.v1";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Body {
    format: String,
    session_lease_digest: SessionAdmissionLeaseDigest,
    ambient_basis: AmbientSessionBasis,
}

pub(in crate::resident) fn encode(
    session: &AdmittedAmbientSession,
    maximum_bytes: usize,
) -> Result<ControlRecordBytes, SessionAdmissionBasisCustodyError> {
    let body = Body {
        format: FORMAT.to_owned(),
        session_lease_digest: session.lease().session_admission_lease_digest().clone(),
        ambient_basis: session.basis.clone(),
    };
    let canonical = canon::to_canonical_string(&body)?;
    if canonical.len() > maximum_bytes {
        return Err(SessionAdmissionBasisCustodyError::BoundExceeded);
    }
    ControlRecordBytes::new(canonical.into_bytes()).map_err(Into::into)
}

pub(in crate::resident) fn decode(
    retained: &SessionLeaseBody,
    lease: &SessionAdmissionLeaseV1,
    maximum_bytes: usize,
) -> Result<RetainedAmbientSession, SessionAdmissionBasisCustodyError> {
    let bytes = retained
        .canonical_private_admission_basis_jcs()
        .ok_or(SessionAdmissionBasisCustodyError::Missing)?;
    if retained
        .canonical_wire_jcs()
        .as_slice()
        .len()
        .checked_add(bytes.as_slice().len())
        .is_none_or(|total| total > maximum_bytes)
    {
        return Err(SessionAdmissionBasisCustodyError::BoundExceeded);
    }
    let digest = retained
        .private_admission_basis_sha256()
        .ok_or(SessionAdmissionBasisCustodyError::Missing)?;
    if digest != &ArtifactDigest::hash_bytes(bytes.as_slice()) {
        return Err(SessionAdmissionBasisCustodyError::DigestMismatch);
    }
    let body: Body = serde_json::from_value(canon::parse_strict(bytes.as_slice())?)?;
    if body.format != FORMAT
        || body.session_lease_digest != *lease.session_admission_lease_digest()
        || canon::to_canonical_string(&body)?.as_bytes() != bytes.as_slice()
    {
        return Err(SessionAdmissionBasisCustodyError::Association);
    }
    Ok(RetainedAmbientSession::from_protected(
        lease.clone(),
        body.ambient_basis,
    ))
}

/// Private session-admission basis could not be retained or recovered exactly.
#[derive(Debug, thiserror::Error)]
pub(in crate::resident) enum SessionAdmissionBasisCustodyError {
    /// No basis was retained with this lease body.
    #[error("retained session lease has no private admission basis")]
    Missing,
    /// The caller-supplied basis ceiling was exceeded.
    #[error("private session-admission basis exceeds its byte ceiling")]
    BoundExceeded,
    /// Retained basis bytes do not match their physical digest.
    #[error("private session-admission basis digest does not match its bytes")]
    DigestMismatch,
    /// The format or lease association was substituted.
    #[error("private session-admission basis is associated with another lease")]
    Association,
    /// Canonical JSON encoding or admission failed.
    #[error(transparent)]
    Canonical(#[from] canon::CanonError),
    /// The canonical private body could not be decoded.
    #[error(transparent)]
    Decode(#[from] serde_json::Error),
    /// The physical byte wrapper rejected an empty body.
    #[error(transparent)]
    Record(#[from] ControlRecordError),
}
