//! Authenticate submission of the exact constructed finalization.
//! Candidate admission and checkpoint construction keep their existing owners.
//! This join retains the command for atomic custody; it does not commit it.

use pale_core::records::*;

use crate::resident::ConstructedDeploymentFinalization;

use super::projection;

/// An authenticated command joined to its candidate records and prepared checkpoint.
///
/// Authentication permits submission, not deployment activation. The caller must
/// still resolve current anchored trust, enforce replay and compare the retained
/// heads inside atomic custody. Dropping this value changes neither store.
#[must_use = "submission still requires atomic custody and independent anchors"]
pub struct DeploymentFinalizationSubmission<'a> {
    command: LocalAdminCommandV1,
    construction: ConstructedDeploymentFinalization<'a>,
}

impl<'a> DeploymentFinalizationSubmission<'a> {
    /// Authenticate a new command that names exactly this finalization.
    ///
    /// The existing core admission checks the delegated administrator, permission,
    /// time and signature. This application join checks ordered candidates and all
    /// four affected heads. Head order is immaterial. An optional head epoch must
    /// agree with that head's owner; it never substitutes for its record digest.
    ///
    /// The supplied trust must be the construction's predecessor. Its freshness
    /// and independent anchor remain the caller's responsibility. Historical
    /// replay must recover the original disposition, not use this new-submission
    /// path. Candidate signatures are not replaced by the administrator's key.
    ///
    /// # Errors
    /// Rejects failed authentication, unlike trust, expired candidate authority,
    /// future checkpoints, different candidates or missing, extra and unlike heads.
    /// Failure consumes the construction without writing either store.
    pub fn admit<F>(
        construction: ConstructedDeploymentFinalization<'a>,
        wire: LocalAdminCommandWireV1,
        context: &LocalAdminCommandAdmissionContextV1<'_>,
        verify_signature: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnOnce(
            &Ed25519PublicKeyV1,
            &[u8],
            &LocalAdminCommandPreimageV1,
            &LocalAdminCommandDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let records = construction.finalization().records();
        if context.trust_checkpoint.canonical_trust_checkpoint_sha256()
            != records
                .trust
                .predecessor
                .canonical_trust_checkpoint_sha256()
            || context.now < *construction.checkpoint().1.issued_at()
            || context.now >= *records.binding.1.expires_at()
            || context.now >= *records.checkpoint_signer.1.expires_at()
            || context.now >= *records.runtime_authority.1.expires_at()
        {
            return Err(reject(
                "the construction's trust predecessor and current candidate validity",
            ));
        }
        let command = LocalAdminCommandV1::admit_authenticated_wire(
            wire,
            context,
            |key, domain, value, digest, signature| {
                projection::verify_command_selection(
                    construction.finalization(),
                    construction.checkpoint(),
                    value,
                )
                .map_err(|mismatch| reject(mismatch.invariant()))?;
                verify_signature(key, domain, value, digest, signature)
            },
        )?;
        Ok(Self {
            command,
            construction,
        })
    }

    /// Borrow the authenticated command, including its exact replay identity.
    #[must_use]
    pub const fn command(&self) -> &LocalAdminCommandV1 {
        &self.command
    }

    /// Borrow the candidate records, signed checkpoint and physical tree batch.
    pub const fn construction(&self) -> &ConstructedDeploymentFinalization<'a> {
        &self.construction
    }
}

fn reject(invariant: &'static str) -> InvalidResidentRecordV1 {
    InvalidResidentRecordV1::ContextualAdmission {
        record: "deployment finalization submission",
        invariant,
    }
}
