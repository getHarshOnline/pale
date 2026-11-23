//! Exact admission of repository source-frontier constituents.

use std::collections::{BTreeMap, BTreeSet};

use pale_core::{
    canon::{to_canonical_string, CanonError, JSON_SAFE_INTEGER_MAX},
    paths::OwnedConfinedRelativePath,
    records::{
        ArtifactDigest, DeploymentIdV1, ImmutableRecordReferenceV1, LogicalExecutionWorkspaceIdV1,
        LogicalProjectIdV1, LogicalRepositoryIdV1, ProviderIndexOwnerV1, RepositoryRevisionV1,
        RepositorySourceFrontierV1, ResidentControllerIdV1, ResidentPrincipalIdV1,
        ResourceContinuityMachineScopeIdV1,
    },
};
use serde::Serialize;

/// Maximum exact members admitted on one source-frontier axis.
pub const MAX_SOURCE_FRONTIER_MEMBERS: usize = 65_536;

/// Complete isolation key for one provider-owned source index.
///
/// Repository identity is not a worktree identity, and provider identity is
/// not a provider pin. The body-free references retain those independent
/// coordinates without copying native records into Pale.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SourceIndexScope {
    /// Principal allowed to consume the derived index.
    pub principal_id: ResidentPrincipalIdV1,
    /// Controller responsible for the selected provider deployment.
    pub controller_id: ResidentControllerIdV1,
    /// Exact deployment scope.
    pub deployment_id: DeploymentIdV1,
    /// Exact machine scope.
    pub machine_id: ResourceContinuityMachineScopeIdV1,
    /// Desired project whose operation requested evidence.
    pub project_id: LogicalProjectIdV1,
    /// Desired execution workspace; this is not the worktree coordinate.
    pub execution_workspace_id: LogicalExecutionWorkspaceIdV1,
    /// Desired repository identity.
    pub repository_id: LogicalRepositoryIdV1,
    /// Exact owner-native worktree record.
    pub worktree: ImmutableRecordReferenceV1,
    /// Source-index provider instance.
    pub provider_index_owner: ProviderIndexOwnerV1,
    /// Exact immutable provider build/configuration pin.
    pub provider_pin: ImmutableRecordReferenceV1,
}

impl SourceIndexScope {
    /// Compute the internal exact-byte identity used only for registry lookup.
    ///
    /// This digest is not a public contract identity, provider generation,
    /// authority proof, or native revision.
    pub(crate) fn lookup_digest(&self) -> Result<ArtifactDigest, CanonError> {
        Ok(ArtifactDigest::hash_bytes(
            to_canonical_string(self)?.as_bytes(),
        ))
    }
}

/// Exact state of one admitted tracked path.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackedSourceDisposition {
    /// Path was added at this frontier.
    Added,
    /// Existing path has different complete bytes.
    Modified,
    /// Path was removed and therefore carries no successor bytes.
    Deleted,
}

/// One tracked modification in the exact source frontier.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TrackedSourceMember {
    /// Repository-relative path.
    pub path: OwnedConfinedRelativePath,
    /// Exact modification class.
    pub disposition: TrackedSourceDisposition,
    /// Successor bytes for added or modified paths.
    pub content_sha256: Option<ArtifactDigest>,
    /// Successor byte length for added or modified paths.
    pub content_bytes: Option<u64>,
}

/// Exact bounded untracked or generated source member.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SourceByteMember {
    /// Repository-relative path.
    pub path: OwnedConfinedRelativePath,
    /// Digest of the complete admitted bytes.
    pub content_sha256: ArtifactDigest,
    /// Length of the complete admitted bytes.
    pub content_bytes: u64,
}

/// Exact submodule source coordinate.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SubmoduleFrontier {
    /// Repository-relative submodule root.
    pub path: OwnedConfinedRelativePath,
    /// Exact checked-out submodule revision.
    pub revision: RepositoryRevisionV1,
}

/// Whether vendor roots may enter the index.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VendorSourcePolicy {
    /// Vendor paths are excluded.
    Exclude,
    /// Only explicitly listed vendor roots may be included.
    IncludeDeclared,
}

/// Symlink handling while constituting a source frontier.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceSymlinkPolicy {
    /// Every symlink is rejected.
    Reject,
    /// A provider may resolve only beneath a retained, revalidated root.
    ConfinedNoFollow,
}

/// Concurrent source mutation behavior during one indexing attempt.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ConcurrentMutationPolicy {
    /// Fail the attempt on any detected mutation.
    Fail,
    /// Restart from a newly observed frontier, up to an exact finite bound.
    Retry {
        /// Maximum complete attempts, including the initial attempt.
        attempts: u16,
    },
}

/// Exact source acquisition and exclusion policy included in the frontier.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SourceFrontierPolicy {
    /// Vendor-source policy.
    pub vendor: VendorSourcePolicy,
    /// Explicit vendor roots when [`VendorSourcePolicy::IncludeDeclared`] is selected.
    pub vendor_roots: BTreeSet<OwnedConfinedRelativePath>,
    /// Symlink policy.
    pub symlinks: SourceSymlinkPolicy,
    /// Explicit excluded roots.
    pub excluded_roots: BTreeSet<OwnedConfinedRelativePath>,
    /// Concurrent-mutation behavior.
    pub concurrent_mutation: ConcurrentMutationPolicy,
}

/// Exact constituents claimed by one [`RepositorySourceFrontierV1`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceFrontierCandidate {
    /// Existing typed frontier coordinate.
    pub coordinate: RepositorySourceFrontierV1,
    /// Complete set of repository-relative source roots.
    pub source_roots: BTreeSet<OwnedConfinedRelativePath>,
    /// Exact tracked delta.
    pub tracked: Vec<TrackedSourceMember>,
    /// Exact untracked-source inventory.
    pub untracked: Vec<SourceByteMember>,
    /// Exact generated-source inventory.
    pub generated: Vec<SourceByteMember>,
    /// Exact submodule revisions.
    pub submodules: Vec<SubmoduleFrontier>,
    /// Acquisition and exclusion policy.
    pub policy: SourceFrontierPolicy,
}

/// Source frontier after its claimed digests and confinement have been reproved.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedSourceFrontier(SourceFrontierCandidate);

impl AdmittedSourceFrontier {
    /// Borrow the exact public frontier coordinate.
    #[must_use]
    pub fn coordinate(&self) -> &RepositorySourceFrontierV1 {
        &self.0.coordinate
    }

    /// Borrow the admitted complete constituents.
    #[must_use]
    pub fn candidate(&self) -> &SourceFrontierCandidate {
        &self.0
    }

    /// Decide whether one repository-relative path belongs to this frontier.
    ///
    /// Admission requires containment beneath a declared source root and
    /// rejects every path beneath an excluded root. This is the shared
    /// membership law for frontier members and provider graph locations.
    #[must_use]
    pub(super) fn admits_path(&self, path: &OwnedConfinedRelativePath) -> bool {
        path_is_admitted(&self.0, path)
    }
}

#[derive(Serialize)]
struct UntrackedPreimage<'a> {
    untracked: &'a [SourceByteMember],
    generated: &'a [SourceByteMember],
}

#[derive(Serialize)]
struct DependencyPreimage<'a> {
    source_roots: &'a BTreeSet<OwnedConfinedRelativePath>,
    submodules: &'a [SubmoduleFrontier],
    policy: &'a SourceFrontierPolicy,
}

/// Reprove the complete constituents of one exact source frontier.
///
/// The three digests already carried by [`RepositorySourceFrontierV1`] bind the
/// tracked delta, untracked/generated inventory, and dependency/policy frontier
/// respectively. Admitted members retain digests and lengths, not source bodies.
///
/// # Errors
///
/// Returns [`SourceFrontierError`] for an empty or oversized axis, malformed
/// byte/deletion state, a path outside all admitted roots, policy mismatch, or
/// any claimed digest that differs from the canonical constituents.
pub fn admit_source_frontier(
    candidate: SourceFrontierCandidate,
) -> Result<AdmittedSourceFrontier, SourceFrontierError> {
    validate_member_count("source roots", candidate.source_roots.len())?;
    if candidate.source_roots.is_empty() {
        return Err(SourceFrontierError::EmptySourceRoots);
    }
    for (axis, count) in [
        ("tracked members", candidate.tracked.len()),
        ("untracked members", candidate.untracked.len()),
        ("generated members", candidate.generated.len()),
        ("submodules", candidate.submodules.len()),
        ("vendor roots", candidate.policy.vendor_roots.len()),
        ("excluded roots", candidate.policy.excluded_roots.len()),
    ] {
        validate_member_count(axis, count)?;
    }
    validate_canonical_paths(&candidate)?;
    validate_policy(&candidate.policy)?;
    for member in &candidate.tracked {
        match (
            member.disposition,
            member.content_sha256.is_some(),
            member.content_bytes,
        ) {
            (TrackedSourceDisposition::Deleted, false, None) => {}
            (
                TrackedSourceDisposition::Added | TrackedSourceDisposition::Modified,
                true,
                Some(bytes),
            ) if bytes <= JSON_SAFE_INTEGER_MAX => {}
            _ => {
                return Err(SourceFrontierError::InvalidTrackedMember(
                    member.path.clone(),
                ))
            }
        }
    }
    for member in candidate.untracked.iter().chain(&candidate.generated) {
        if member.content_bytes > JSON_SAFE_INTEGER_MAX {
            return Err(SourceFrontierError::UnsafeByteLength(member.path.clone()));
        }
    }
    let tracked = digest_preimage(&candidate.tracked)?;
    let untracked = digest_preimage(&UntrackedPreimage {
        untracked: &candidate.untracked,
        generated: &candidate.generated,
    })?;
    let dependency = digest_preimage(&DependencyPreimage {
        source_roots: &candidate.source_roots,
        submodules: &candidate.submodules,
        policy: &candidate.policy,
    })?;
    for (axis, observed, claimed) in [
        (
            "tracked delta",
            tracked,
            &candidate.coordinate.tracked_delta_sha256,
        ),
        (
            "untracked inventory",
            untracked,
            &candidate.coordinate.untracked_inventory_sha256,
        ),
        (
            "dependency frontier",
            dependency,
            &candidate.coordinate.dependency_frontier_sha256,
        ),
    ] {
        if &observed != claimed {
            return Err(SourceFrontierError::DigestMismatch { axis });
        }
    }
    Ok(AdmittedSourceFrontier(candidate))
}

fn digest_preimage(value: &impl Serialize) -> Result<ArtifactDigest, SourceFrontierError> {
    Ok(ArtifactDigest::hash_bytes(
        to_canonical_string(value)
            .map_err(SourceFrontierError::Canonicalize)?
            .as_bytes(),
    ))
}

fn validate_member_count(axis: &'static str, count: usize) -> Result<(), SourceFrontierError> {
    if count <= MAX_SOURCE_FRONTIER_MEMBERS {
        Ok(())
    } else {
        Err(SourceFrontierError::TooManyMembers { axis, count })
    }
}

fn validate_canonical_paths(
    candidate: &SourceFrontierCandidate,
) -> Result<(), SourceFrontierError> {
    for paths in [
        candidate
            .tracked
            .iter()
            .map(|member| &member.path)
            .collect::<Vec<_>>(),
        candidate
            .untracked
            .iter()
            .map(|member| &member.path)
            .collect(),
        candidate
            .generated
            .iter()
            .map(|member| &member.path)
            .collect(),
        candidate
            .submodules
            .iter()
            .map(|member| &member.path)
            .collect(),
    ] {
        if !paths.windows(2).all(|pair| pair[0] < pair[1]) {
            return Err(SourceFrontierError::NonCanonicalMembers);
        }
        for path in paths {
            if !path_is_admitted(candidate, path) {
                return Err(SourceFrontierError::OutsideSourceRoots(path.clone()));
            }
        }
    }
    let mut owners = BTreeMap::<&OwnedConfinedRelativePath, &'static str>::new();
    for (axis, paths) in [
        (
            "tracked",
            candidate
                .tracked
                .iter()
                .map(|member| &member.path)
                .collect::<Vec<_>>(),
        ),
        (
            "untracked",
            candidate
                .untracked
                .iter()
                .map(|member| &member.path)
                .collect::<Vec<_>>(),
        ),
        (
            "generated",
            candidate
                .generated
                .iter()
                .map(|member| &member.path)
                .collect::<Vec<_>>(),
        ),
    ] {
        for path in paths {
            if owners.insert(path, axis).is_some() {
                return Err(SourceFrontierError::OverlappingSourceClass(path.clone()));
            }
        }
    }
    Ok(())
}

fn path_is_admitted(candidate: &SourceFrontierCandidate, path: &OwnedConfinedRelativePath) -> bool {
    candidate
        .source_roots
        .iter()
        .any(|root| path_is_within(path, root))
        && !candidate
            .policy
            .excluded_roots
            .iter()
            .any(|root| path_is_within(path, root))
}

fn path_is_within(path: &OwnedConfinedRelativePath, root: &OwnedConfinedRelativePath) -> bool {
    path.as_str() == root.as_str()
        || path
            .as_str()
            .strip_prefix(root.as_str())
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn validate_policy(policy: &SourceFrontierPolicy) -> Result<(), SourceFrontierError> {
    match (policy.vendor, policy.vendor_roots.is_empty()) {
        (VendorSourcePolicy::Exclude, true) | (VendorSourcePolicy::IncludeDeclared, false) => {}
        _ => return Err(SourceFrontierError::VendorPolicyMismatch),
    }
    if matches!(
        policy.concurrent_mutation,
        ConcurrentMutationPolicy::Retry { attempts: 0 }
    ) {
        return Err(SourceFrontierError::ZeroMutationAttempts);
    }
    Ok(())
}

/// Rejection while binding complete source constituents to a frontier coordinate.
#[derive(Debug, thiserror::Error)]
pub enum SourceFrontierError {
    /// At least one source root is required.
    #[error("source frontier contains no source root")]
    EmptySourceRoots,
    /// A bounded axis exceeded its maximum.
    #[error("source frontier {axis} contains {count} members; maximum is 65536")]
    TooManyMembers {
        /// Rejected axis.
        axis: &'static str,
        /// Observed member count.
        count: usize,
    },
    /// A vector axis is not strictly ordered and duplicate-free.
    #[error("source frontier members are not in canonical strict path order")]
    NonCanonicalMembers,
    /// One path is not beneath any admitted source root.
    #[error("source path `{0}` lies outside every admitted source root")]
    OutsideSourceRoots(OwnedConfinedRelativePath),
    /// One path appears in more than one source class.
    #[error("source path `{0}` appears in multiple source classes")]
    OverlappingSourceClass(OwnedConfinedRelativePath),
    /// Tracked deletion/bytes state is inconsistent.
    #[error("tracked source member `{0}` has inconsistent disposition and bytes")]
    InvalidTrackedMember(OwnedConfinedRelativePath),
    /// A source length is outside the portable exact-integer domain.
    #[error("source member `{0}` has a non-portable byte length")]
    UnsafeByteLength(OwnedConfinedRelativePath),
    /// Vendor roots disagree with the selected vendor policy.
    #[error("vendor roots disagree with the selected source-frontier policy")]
    VendorPolicyMismatch,
    /// Bounded retry must permit at least one complete attempt.
    #[error("concurrent-mutation retry policy permits zero attempts")]
    ZeroMutationAttempts,
    /// A claimed frontier digest does not cover its exact constituents.
    #[error("source frontier {axis} digest does not match its canonical constituents")]
    DigestMismatch {
        /// Mismatched digest axis.
        axis: &'static str,
    },
    /// Canonical projection failed.
    #[error("cannot canonicalize source-frontier constituents: {0}")]
    Canonicalize(#[source] CanonError),
}
