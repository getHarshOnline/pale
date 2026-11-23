//! Receipt laws for unchanged whole-file and managed-block surfaces.

use pale_core::records::{ArtifactDigest, SurfaceOwnership, UnchangedDispositionV1};

use super::unchanged_observation_matches;

fn digest(byte: u8) -> ArtifactDigest {
    ArtifactDigest::hash_bytes(&[byte])
}

#[test]
fn whole_file_unchanged_receipt_requires_one_shared_digest_domain() {
    let owned = digest(1);
    let different = digest(2);

    assert!(unchanged_observation_matches(
        SurfaceOwnership::WholeFile,
        &owned,
        &owned,
        &owned,
        UnchangedDispositionV1::Verified,
        UnchangedDispositionV1::Verified,
    ));
    assert!(!unchanged_observation_matches(
        SurfaceOwnership::WholeFile,
        &different,
        &owned,
        &owned,
        UnchangedDispositionV1::Verified,
        UnchangedDispositionV1::Verified,
    ));
}

#[test]
fn managed_block_unchanged_receipt_keeps_block_and_file_digests_distinct() {
    let owned_block = digest(1);
    let whole_file = digest(2);
    let changed_file = digest(3);

    assert!(unchanged_observation_matches(
        SurfaceOwnership::ManagedBlock,
        &owned_block,
        &whole_file,
        &whole_file,
        UnchangedDispositionV1::Verified,
        UnchangedDispositionV1::Verified,
    ));
    assert!(!unchanged_observation_matches(
        SurfaceOwnership::ManagedBlock,
        &owned_block,
        &whole_file,
        &changed_file,
        UnchangedDispositionV1::Verified,
        UnchangedDispositionV1::Verified,
    ));
}
