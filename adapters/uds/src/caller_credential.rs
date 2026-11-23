//! Explicit file custody for a caller's separately provisioned transport key.

use std::{path::Path, time::Instant};

use ed25519_dalek::{Signer as _, SigningKey};
use pale_adapter_fs::{capture_private_configuration, ReadLimit};
use pale_ports::{LocalConnectionSignerPort, LocalConnectionSigningError};
use zeroize::Zeroizing;

use crate::authentication::REQUEST_DOMAIN;

/// The selected credential could not be captured or did not match its public pin.
#[derive(Debug, thiserror::Error)]
#[error("the selected caller connection credential is unavailable or mismatched")]
pub struct FileCallerCredentialError;

/// An explicit, owner-private file containing a raw 32-byte Ed25519 seed.
///
/// This optional custody implementation creates no credential and selects no
/// default path. It signs caller connection proofs only, for the pinned resident;
/// it cannot sign Pale records or resident proofs. POSIX ownership and mode
/// checks do not exclude other processes with the same UID or protect backups.
/// Selecting this implementation is not qualification of a deployment's custody.
pub struct FileCallerCredential {
    key: SigningKey,
    resident: [u8; 32],
}

impl FileCallerCredential {
    /// Capture the selected seed through no-follow, owner-private file checks.
    ///
    /// The file must be regular, single-link, exactly 32 bytes and mode `0600`
    /// beneath an owner-private immediate parent. The key must match `caller`.
    /// The successfully captured seed and signing key are zeroized on drop;
    /// this is not a claim about filesystem caches, swap or copies in backups.
    ///
    /// # Errors
    /// Refuses failed protection checks, a changed or invalid file, or a wrong key.
    pub fn load(
        path: &Path,
        caller: [u8; 32],
        resident: [u8; 32],
    ) -> Result<Self, FileCallerCredentialError> {
        let bytes = Zeroizing::new(
            capture_private_configuration(path, ReadLimit::new(32))
                .map_err(|_| FileCallerCredentialError)?,
        );
        let seed =
            <&[u8; 32]>::try_from(bytes.as_slice()).map_err(|_| FileCallerCredentialError)?;
        let key = SigningKey::from_bytes(seed);
        if key.verifying_key().to_bytes() != caller {
            return Err(FileCallerCredentialError);
        }
        Ok(Self { key, resident })
    }
}

impl LocalConnectionSignerPort for FileCallerCredential {
    fn public_key(&self) -> [u8; 32] {
        self.key.verifying_key().to_bytes()
    }

    fn sign(
        &mut self,
        message: &[u8],
        deadline: Instant,
    ) -> Result<[u8; 64], LocalConnectionSigningError> {
        let body = message
            .strip_prefix(REQUEST_DOMAIN)
            .filter(|body| body.len() == 160)
            .ok_or(LocalConnectionSigningError)?;
        if Instant::now() >= deadline
            || body[..32] != self.public_key()
            || body[32..64] != self.resident
        {
            return Err(LocalConnectionSigningError);
        }
        let signature = self.key.sign(message).to_bytes();
        if Instant::now() >= deadline {
            return Err(LocalConnectionSigningError);
        }
        Ok(signature)
    }
}

impl std::fmt::Debug for FileCallerCredential {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("FileCallerCredential(<private transport credential>)")
    }
}
