//! Exact immutable Package and normative-Pack release coordinates.
//!
//! A release coordinate identifies owner-published bytes. Selection places that
//! coordinate in a bounded closure; it does not prove occurrence, admission,
//! activation, use, native consequence, or settlement. Package and normative
//! Pack coordinates deliberately remain non-substitutable because a Package
//! owns reusable procedure while a Pack attests exact normative content.

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize};

use crate::records::{NormativePackDigest, PackageDigest, PackageIdV1};

/// Maximum UTF-8 byte length of an initial-v1 release label.
///
/// Release labels are opaque owner-published coordinates, not implicitly
/// semantic-version ranges. The bound applies equally to Package and Pack
/// releases so parsing cost is independent of the publishing owner.
pub const MAX_RELEASE_LABEL_BYTES: usize = 255;

/// Maximum number of exact Package releases in one selected closure.
///
/// The value matches the initial-v1 1,024-declaration compilation scale. The
/// separate [`MAX_SELECTED_NORMATIVE_PACK_RELEASES`] bound prevents either
/// release class from consuming the other's admission budget.
pub const MAX_SELECTED_PACKAGE_RELEASES: usize = 1_024;

/// Maximum number of exact normative Pack releases in one selected closure.
pub const MAX_SELECTED_NORMATIVE_PACK_RELEASES: usize = 1_024;

/// A release label violates the initial-v1 lexical grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("release label must be a nonempty printable ASCII token of at most 255 bytes")]
pub struct InvalidReleaseLabelV1;

/// A normative Pack or attestor identity violates the initial-v1 grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("identity must be lowercase Package-style text of at most 255 bytes")]
pub struct InvalidNormativePackIdV1;

fn release_label_is_valid(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_RELEASE_LABEL_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b'+'))
}

fn owner_identifier_is_valid(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_RELEASE_LABEL_BYTES
        && value.split('.').all(|segment| {
            let mut bytes = segment.bytes();
            bytes.next().is_some_and(|byte| byte.is_ascii_lowercase())
                && bytes.all(|byte| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || matches!(byte, b'-' | b'_')
                })
        })
}

/// Owner-published Package release label.
///
/// This is intentionally not a version range. Dependency and distribution
/// records name one exact label, and [`PackageReleaseV1::content_digest`]
/// supplies immutable byte identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, schemars::JsonSchema)]
#[serde(transparent)]
#[schemars(transparent)]
pub struct PackageVersionV1(
    #[schemars(
        length(min = 1, max = 255),
        regex(pattern = r"^[A-Za-z0-9][A-Za-z0-9._+-]*$")
    )]
    String,
);

impl PackageVersionV1 {
    /// Validate and own one exact Package release label.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidReleaseLabelV1`] when `value` is empty, too long, or
    /// contains characters outside the initial-v1 release-token grammar.
    pub fn parse(value: impl Into<String>) -> Result<Self, InvalidReleaseLabelV1> {
        let value = value.into();
        release_label_is_valid(&value)
            .then_some(Self(value))
            .ok_or(InvalidReleaseLabelV1)
    }

    /// Borrow the validated wire value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for PackageVersionV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Display for PackageVersionV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One exact immutable Package release.
///
/// This is the sole owner-neutral Package release coordinate used by locks,
/// selected closures, plans, receipts, and generations. A manifest declares
/// [`Self::id`] and [`Self::version`] before its own digest can be computed;
/// admission joins those declarations to this complete coordinate.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct PackageReleaseV1 {
    /// Canonical Package identity.
    pub id: PackageIdV1,
    /// Exact owner-published release label.
    pub version: PackageVersionV1,
    /// Digest of the complete Package manifest and owned source inventory.
    pub content_digest: PackageDigest,
}

/// Canonical identity of one normative Pack, independent of release and use.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, schemars::JsonSchema)]
#[serde(transparent)]
#[schemars(transparent)]
pub struct NormativePackIdV1(
    #[schemars(
        length(min = 1, max = 255),
        regex(pattern = r"^[a-z][a-z0-9_-]*(?:\.[a-z][a-z0-9_-]*)*$")
    )]
    String,
);

impl NormativePackIdV1 {
    /// Validate and own a normative Pack identity.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidNormativePackIdV1`] when `value` violates the bounded
    /// lowercase initial-v1 Package-style identity grammar.
    pub fn parse(value: impl Into<String>) -> Result<Self, InvalidNormativePackIdV1> {
        let value = value.into();
        owner_identifier_is_valid(&value)
            .then_some(Self(value))
            .ok_or(InvalidNormativePackIdV1)
    }

    /// Borrow the validated wire value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for NormativePackIdV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Display for NormativePackIdV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Owner or attestor coordinate responsible for normative Pack bytes.
///
/// This identity is attributable custody, not a principal, controller,
/// publisher permission, or native-effect grant.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, schemars::JsonSchema)]
#[serde(transparent)]
#[schemars(transparent)]
pub struct NormativePackAttestorV1(
    #[schemars(
        length(min = 1, max = 255),
        regex(pattern = r"^[a-z][a-z0-9_-]*(?:\.[a-z][a-z0-9_-]*)*$")
    )]
    String,
);

impl NormativePackAttestorV1 {
    /// Validate and own one Pack-attestor coordinate.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidNormativePackIdV1`] under the same bounded grammar as
    /// [`NormativePackIdV1::parse`].
    pub fn parse(value: impl Into<String>) -> Result<Self, InvalidNormativePackIdV1> {
        let value = value.into();
        owner_identifier_is_valid(&value)
            .then_some(Self(value))
            .ok_or(InvalidNormativePackIdV1)
    }

    /// Borrow the validated wire value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for NormativePackAttestorV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Display for NormativePackAttestorV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Owner-published normative Pack release label.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, schemars::JsonSchema)]
#[serde(transparent)]
#[schemars(transparent)]
pub struct NormativePackVersionV1(
    #[schemars(
        length(min = 1, max = 255),
        regex(pattern = r"^[A-Za-z0-9][A-Za-z0-9._+-]*$")
    )]
    String,
);

impl NormativePackVersionV1 {
    /// Validate and own one exact normative Pack release label.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidReleaseLabelV1`] under the same lexical conditions as
    /// [`PackageVersionV1::parse`].
    pub fn parse(value: impl Into<String>) -> Result<Self, InvalidReleaseLabelV1> {
        let value = value.into();
        release_label_is_valid(&value)
            .then_some(Self(value))
            .ok_or(InvalidReleaseLabelV1)
    }

    /// Borrow the validated wire value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for NormativePackVersionV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Display for NormativePackVersionV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One exact immutable normative Pack release and its attestor.
///
/// The coordinate carries no Pack body and cannot prove that a workspace has
/// selected, admitted, activated, or used the release.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct NormativePackReleaseV1 {
    /// Canonical Pack identity.
    pub id: NormativePackIdV1,
    /// Exact owner or attestor of these normative bytes.
    pub attestor: NormativePackAttestorV1,
    /// Exact owner-published release label.
    pub version: NormativePackVersionV1,
    /// Digest of the attested immutable Pack bytes.
    pub content_digest: NormativePackDigest,
}

/// A selected release closure is unbounded, duplicated, or non-canonical.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ReleaseClosureErrorV1 {
    /// The Package closure exceeds its initial-v1 bound.
    #[error("Package closure contains {actual} releases; maximum is {maximum}")]
    TooManyPackages {
        /// Observed Package-release count.
        actual: usize,
        /// Initial-v1 Package-release ceiling.
        maximum: usize,
    },
    /// The normative Pack closure exceeds its independent initial-v1 bound.
    #[error("normative Pack closure contains {actual} releases; maximum is {maximum}")]
    TooManyNormativePacks {
        /// Observed Pack-release count.
        actual: usize,
        /// Initial-v1 Pack-release ceiling.
        maximum: usize,
    },
    /// Two non-identical exact releases claim one Package identity.
    #[error("conflicting exact releases for Package {package_id}")]
    ConflictingPackageRelease {
        /// Package owner whose release coordinates disagree.
        package_id: PackageIdV1,
        /// First exact coordinate encountered.
        existing: Box<PackageReleaseV1>,
        /// Non-identical coordinate for the same Package owner.
        conflicting: Box<PackageReleaseV1>,
    },
    /// Two non-identical exact releases claim one attestor/Pack identity.
    #[error("conflicting exact releases for normative Pack {attestor}/{pack_id}")]
    ConflictingNormativePackRelease {
        /// Attestor in the semantic owner key.
        attestor: NormativePackAttestorV1,
        /// Pack identity in the semantic owner key.
        pack_id: NormativePackIdV1,
        /// First exact coordinate encountered.
        existing: Box<NormativePackReleaseV1>,
        /// Non-identical coordinate for the same Pack owner.
        conflicting: Box<NormativePackReleaseV1>,
    },
    /// Package releases are not strictly ordered by Package identity.
    #[error("Package closure is not strictly ordered at {package_id}")]
    NonCanonicalPackageOrder {
        /// First Package identity that is duplicated or out of order.
        package_id: PackageIdV1,
    },
    /// Pack releases are not strictly ordered by `(attestor, id)`.
    #[error("normative Pack closure is not strictly ordered at {attestor}/{pack_id}")]
    NonCanonicalNormativePackOrder {
        /// Attestor of the duplicated or out-of-order Pack.
        attestor: NormativePackAttestorV1,
        /// Pack identity at the invalid frontier.
        pack_id: NormativePackIdV1,
    },
}

/// Canonical, independently bounded Package and normative-Pack closures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalReleaseClosuresV1 {
    /// Exact Package releases, strictly sorted by Package identity.
    pub packages: Vec<PackageReleaseV1>,
    /// Exact Pack releases, strictly sorted by `(attestor, Pack id)`.
    pub normative_packs: Vec<NormativePackReleaseV1>,
}

/// Coalesce identical occurrence releases into canonical selected closures.
///
/// Input bounds are checked before coalescing, so a duplicate flood cannot use
/// semantic idempotence to bypass initial-v1 work limits. Identical exact
/// coordinates coalesce; non-identical coordinates for one semantic owner are
/// a conflict rather than a last-writer-wins choice.
///
/// # Errors
///
/// Returns [`ReleaseClosureErrorV1`] when either supplied inventory exceeds its
/// independent bound or one semantic owner has conflicting exact coordinates.
pub fn canonicalize_release_closures(
    packages: Vec<PackageReleaseV1>,
    normative_packs: Vec<NormativePackReleaseV1>,
) -> Result<CanonicalReleaseClosuresV1, ReleaseClosureErrorV1> {
    validate_release_bounds(packages.len(), normative_packs.len())?;

    let mut canonical_packages = BTreeMap::<PackageIdV1, PackageReleaseV1>::new();
    for release in packages {
        match canonical_packages.get(&release.id) {
            Some(existing) if existing == &release => {}
            Some(existing) => {
                return Err(ReleaseClosureErrorV1::ConflictingPackageRelease {
                    package_id: release.id.clone(),
                    existing: Box::new(existing.clone()),
                    conflicting: Box::new(release),
                });
            }
            None => {
                canonical_packages.insert(release.id.clone(), release);
            }
        }
    }

    let mut canonical_packs =
        BTreeMap::<(NormativePackAttestorV1, NormativePackIdV1), NormativePackReleaseV1>::new();
    for release in normative_packs {
        let key = (release.attestor.clone(), release.id.clone());
        match canonical_packs.get(&key) {
            Some(existing) if existing == &release => {}
            Some(existing) => {
                return Err(ReleaseClosureErrorV1::ConflictingNormativePackRelease {
                    attestor: release.attestor.clone(),
                    pack_id: release.id.clone(),
                    existing: Box::new(existing.clone()),
                    conflicting: Box::new(release),
                });
            }
            None => {
                canonical_packs.insert(key, release);
            }
        }
    }

    let canonical = CanonicalReleaseClosuresV1 {
        packages: canonical_packages.into_values().collect(),
        normative_packs: canonical_packs.into_values().collect(),
    };
    validate_release_closures(&canonical.packages, &canonical.normative_packs)?;
    Ok(canonical)
}

/// Validate the two independently typed selected-release closures.
///
/// Canonical order is strict by semantic owner identity, so one closure cannot
/// contain two releases for the same Package or for the same attestor/Pack
/// pair. Sorting by digest would be deterministic but dishonest because it
/// would permit conflicting exact releases of one semantic owner.
///
/// # Errors
///
/// Returns [`ReleaseClosureErrorV1`] for an exceeded bound, duplicate owner
/// identity, or non-canonical order.
pub fn validate_release_closures(
    packages: &[PackageReleaseV1],
    normative_packs: &[NormativePackReleaseV1],
) -> Result<(), ReleaseClosureErrorV1> {
    validate_release_bounds(packages.len(), normative_packs.len())?;

    for pair in packages.windows(2) {
        if package_owner(&pair[0]) >= package_owner(&pair[1]) {
            return Err(ReleaseClosureErrorV1::NonCanonicalPackageOrder {
                package_id: pair[1].id.clone(),
            });
        }
    }
    for pair in normative_packs.windows(2) {
        if normative_pack_owner(&pair[0]) >= normative_pack_owner(&pair[1]) {
            return Err(ReleaseClosureErrorV1::NonCanonicalNormativePackOrder {
                attestor: pair[1].attestor.clone(),
                pack_id: pair[1].id.clone(),
            });
        }
    }
    Ok(())
}

fn validate_release_bounds(
    package_count: usize,
    normative_pack_count: usize,
) -> Result<(), ReleaseClosureErrorV1> {
    if package_count > MAX_SELECTED_PACKAGE_RELEASES {
        return Err(ReleaseClosureErrorV1::TooManyPackages {
            actual: package_count,
            maximum: MAX_SELECTED_PACKAGE_RELEASES,
        });
    }
    if normative_pack_count > MAX_SELECTED_NORMATIVE_PACK_RELEASES {
        return Err(ReleaseClosureErrorV1::TooManyNormativePacks {
            actual: normative_pack_count,
            maximum: MAX_SELECTED_NORMATIVE_PACK_RELEASES,
        });
    }
    Ok(())
}

fn package_owner(release: &PackageReleaseV1) -> &PackageIdV1 {
    &release.id
}

fn normative_pack_owner(
    release: &NormativePackReleaseV1,
) -> (&NormativePackAttestorV1, &NormativePackIdV1) {
    (&release.attestor, &release.id)
}

#[cfg(test)]
#[path = "../../tests/internal/records/release.rs"]
mod tests;
