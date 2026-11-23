//! Domain-neutral SHA-256 primitives.
//!
//! Persisted contracts should normally use a domain-specific digest from
//! [`crate::records`] instead of carrying the untyped [`String`] returned here.

use std::io::{self, Read};

use sha2::{Digest, Sha256};

/// Return the lowercase hexadecimal SHA-256 of `bytes`.
///
/// Persistent record fields wrap this representation in the digest type owned
/// by their boundary.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Return the lowercase hexadecimal SHA-256 of a byte stream.
///
/// The fixed-size buffer lets callers hash a large artifact without first
/// loading it into memory. Typed executable identities expose this primitive
/// through
/// [`ExecutableDigest::hash_reader`](crate::records::ExecutableDigest::hash_reader).
///
/// # Errors
///
/// Returns the first read error reported by the supplied stream.
pub fn sha256_reader(mut reader: impl Read) -> io::Result<String> {
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Whether `value` is exactly 64 lowercase hexadecimal characters.
///
/// This checks representation only; it does not establish the preimage or
/// digest domain. Prefer a typed digest parser at trust boundaries.
#[must_use]
pub fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
