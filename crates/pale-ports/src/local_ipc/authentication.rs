//! Signing supplied by the owner of a local connection credential.

use std::time::Instant;

/// A selected local connection signer could not produce its signature.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("local connection signing is unavailable")]
pub struct LocalConnectionSigningError;

/// Sign transport proofs with an independently selected Ed25519 credential.
///
/// The key identifies a connection peer, not a Pale principal or permission.
/// Deployment composition supplies its custody and the peers that trust it.
/// This port creates no key and must not substitute a record-authority signer.
pub trait LocalConnectionSignerPort {
    /// Return the public key corresponding to this connection credential.
    fn public_key(&self) -> [u8; 32];

    /// Sign the complete domain-separated local connection proof by `deadline`.
    ///
    /// # Errors
    /// Returns an error when the selected credential cannot sign. The caller
    /// must stop the exchange; another credential is not an automatic fallback.
    fn sign(
        &mut self,
        message: &[u8],
        deadline: Instant,
    ) -> Result<[u8; 64], LocalConnectionSigningError>;
}
