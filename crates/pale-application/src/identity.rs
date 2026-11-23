//! Integrity-checked identities for a project and the machine it runs on, shared
//! by the steps that compile and render a workspace.
//!
//! `compile`, `plan`, `render` and `workspace` build on it; it builds on no other
//! module in this crate.

use pale_core::canon;
use pale_core::records::{
    EffectiveProjectProfileV1, InstanceDigest, ProfileDigest, ResolvedProjectInstanceV1,
};

/// Failure to establish the profile-to-instance identity chain.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ProjectIdentityError {
    #[error("cannot canonicalize {record}: {source}")]
    Canonicalize {
        record: &'static str,
        #[source]
        source: canon::CanonError,
    },
    #[error("{record} does not satisfy its canonical schema: {source}")]
    Schema {
        record: &'static str,
        #[source]
        source: pale_contracts::SchemaError,
    },
    #[error("profile digest does not match the canonical profile")]
    ProfileDigestMismatch,
    #[error("instance digest does not match the canonical resolved instance")]
    InstanceDigestMismatch,
    #[error("resolved instance does not bind the validated profile")]
    ProfileBinding,
}

/// A portable profile whose schema and canonical self-digest have both been verified.
#[derive(Clone, Copy)]
pub(crate) struct VerifiedProfile<'a> {
    digest: &'a ProfileDigest,
}

impl<'a> VerifiedProfile<'a> {
    /// Admit a profile only after schema and canonical self-digest verification.
    pub(crate) fn verify(
        profile: &'a EffectiveProjectProfileV1,
    ) -> Result<Self, ProjectIdentityError> {
        pale_contracts::validate_record(profile, "effective-profile.v1").map_err(|source| {
            ProjectIdentityError::Schema {
                record: "profile",
                source,
            }
        })?;
        let digest = &profile.profile_digest;
        let actual =
            profile
                .recompute_digest()
                .map_err(|source| ProjectIdentityError::Canonicalize {
                    record: "profile",
                    source,
                })?;
        if digest != &actual {
            return Err(ProjectIdentityError::ProfileDigestMismatch);
        }
        Ok(Self { digest })
    }

    /// Borrow the digest proven by this admission capability.
    pub(crate) fn digest(self) -> &'a ProfileDigest {
        self.digest
    }
}

/// Integrity proof bound to the exact immutable profile and machine instance.
///
/// Consumers borrow records from this capability rather than pairing a digest
/// proof with separately supplied records. Shared borrows prevent either owner
/// from changing while the proof is reused across repository projections.
/// A later admission boundary still calls [`Self::verify`] independently.
#[derive(Clone, Copy)]
pub(crate) struct ProjectIdentity<'a> {
    profile: &'a EffectiveProjectProfileV1,
    instance: &'a ResolvedProjectInstanceV1,
}

impl<'a> ProjectIdentity<'a> {
    /// Verify both self-digests and the instance-to-profile binding once.
    pub(crate) fn verify(
        profile: &'a EffectiveProjectProfileV1,
        instance: &'a ResolvedProjectInstanceV1,
    ) -> Result<Self, ProjectIdentityError> {
        let profile_digest = VerifiedProfile::verify(profile)?.digest();
        if &instance.profile_digest != profile_digest {
            return Err(ProjectIdentityError::ProfileBinding);
        }

        pale_contracts::validate_record(instance, "resolved-instance.v1").map_err(|source| {
            ProjectIdentityError::Schema {
                record: "resolved instance",
                source,
            }
        })?;

        let instance_digest = &instance.instance_digest;
        let expected_instance =
            instance
                .recompute_digest()
                .map_err(|source| ProjectIdentityError::Canonicalize {
                    record: "resolved instance",
                    source,
                })?;
        if instance_digest != &expected_instance {
            return Err(ProjectIdentityError::InstanceDigestMismatch);
        }

        Ok(Self { profile, instance })
    }

    /// Borrow the verified portable profile digest.
    pub(crate) fn profile(self) -> &'a ProfileDigest {
        &self.profile.profile_digest
    }

    /// Borrow the verified machine-instance digest.
    pub(crate) fn instance(self) -> &'a InstanceDigest {
        &self.instance.instance_digest
    }

    /// Borrow the exact portable record admitted by [`Self::verify`].
    pub(crate) fn profile_record(self) -> &'a EffectiveProjectProfileV1 {
        self.profile
    }

    /// Borrow the exact resolved record whose binding and self-digest were verified.
    pub(crate) fn instance_record(self) -> &'a ResolvedProjectInstanceV1 {
        self.instance
    }
}
