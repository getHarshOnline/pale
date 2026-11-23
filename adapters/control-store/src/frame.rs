//! Candidate physical framing for opaque authority-store payloads.

use pale_core::sha256_hex;
use pale_ports::{AuthorityByteClass, DurableAuthorityStoreError, DurableAuthorityStoreErrorKind};

/// Eight-byte prefix that prevents arbitrary bytes from being decoded as a frame.
const FRAME_MAGIC: &[u8; 8] = b"PALEAF01";
/// Encoding version carried by every candidate frame.
const FRAME_VERSION: u8 = 1;
/// Stable evidence identity of the exercised candidate frame encoding.
///
/// This name identifies benchmarked mechanics only. It is not an admitted
/// [`crate::AuthorityJournalProfileV1::frame_encoding`] selection.
#[cfg(feature = "bench")]
pub(crate) const AUTHORITY_FRAME_ENCODING_CANDIDATE_ID: &str = "pale-authority-frame.v1";
/// Stable evidence identity of the exercised corruption-checksum candidate.
///
/// SHA-256 here detects accidental byte corruption; it is not a signature,
/// message-authentication code, or selected storage-profile coordinate.
#[cfg(feature = "bench")]
pub(crate) const AUTHORITY_CORRUPTION_CHECKSUM_CANDIDATE_ID: &str = "sha256-lowercase-hex";
/// Encoded byte width of the lowercase hexadecimal SHA-256 checksum.
pub(crate) const AUTHORITY_CORRUPTION_CHECKSUM_ENCODED_BYTES: usize = 64;
/// Magic, version, big-endian payload length, and checksum prefix width.
pub(crate) const FRAME_HEADER_BYTES: usize =
    FRAME_MAGIC.len() + 1 + size_of::<u64>() + AUTHORITY_CORRUPTION_CHECKSUM_ENCODED_BYTES;

/// Frame one non-empty opaque application payload without interpreting it.
pub(crate) fn encode(
    payload: &[u8],
    class: AuthorityByteClass,
) -> Result<Vec<u8>, DurableAuthorityStoreError> {
    let payload_length = u64::try_from(payload.len())
        .map_err(|_| DurableAuthorityStoreErrorKind::BoundExceeded { class })?;
    let frame_length = FRAME_HEADER_BYTES
        .checked_add(payload.len())
        .ok_or(DurableAuthorityStoreErrorKind::BoundExceeded { class })?;
    let mut frame = Vec::with_capacity(frame_length);
    frame.extend_from_slice(FRAME_MAGIC);
    frame.push(FRAME_VERSION);
    frame.extend_from_slice(&payload_length.to_be_bytes());
    frame.extend_from_slice(sha256_hex(payload).as_bytes());
    frame.extend_from_slice(payload);
    Ok(frame)
}

/// Verify and borrow one complete candidate frame under an explicit payload bound.
pub(crate) fn decode(
    frame: &[u8],
    maximum_payload_bytes: usize,
    class: AuthorityByteClass,
) -> Result<&[u8], DurableAuthorityStoreError> {
    if frame.len() < FRAME_HEADER_BYTES
        || &frame[..FRAME_MAGIC.len()] != FRAME_MAGIC
        || frame[FRAME_MAGIC.len()] != FRAME_VERSION
    {
        return Err(DurableAuthorityStoreErrorKind::Corrupt(class).into());
    }

    let length_start = FRAME_MAGIC.len() + 1;
    let length_end = length_start + size_of::<u64>();
    let length_bytes: [u8; size_of::<u64>()] = frame[length_start..length_end]
        .try_into()
        .map_err(|_| DurableAuthorityStoreErrorKind::Corrupt(class))?;
    let payload_length = u64::from_be_bytes(length_bytes);
    let payload_length = usize::try_from(payload_length)
        .map_err(|_| DurableAuthorityStoreErrorKind::BoundExceeded { class })?;
    if payload_length > maximum_payload_bytes {
        return Err(DurableAuthorityStoreErrorKind::BoundExceeded { class }.into());
    }
    let expected_length = FRAME_HEADER_BYTES
        .checked_add(payload_length)
        .ok_or(DurableAuthorityStoreErrorKind::BoundExceeded { class })?;
    if frame.len() != expected_length {
        return Err(DurableAuthorityStoreErrorKind::Corrupt(class).into());
    }

    let checksum = &frame[length_end..FRAME_HEADER_BYTES];
    let payload = &frame[FRAME_HEADER_BYTES..];
    if checksum != sha256_hex(payload).as_bytes() {
        return Err(DurableAuthorityStoreErrorKind::Corrupt(class).into());
    }
    Ok(payload)
}

#[cfg(test)]
#[path = "../tests/internal/frame.rs"]
mod tests;
