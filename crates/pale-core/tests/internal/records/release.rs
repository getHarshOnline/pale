//! Release record contract tests.

use super::*;

fn package(id: &str, version: &str, digest: &str) -> PackageReleaseV1 {
    PackageReleaseV1 {
        id: PackageIdV1::parse(id).expect("synthetic Package id must be valid"),
        version: PackageVersionV1::parse(version).expect("synthetic Package version must be valid"),
        content_digest: PackageDigest::parse(digest)
            .expect("synthetic Package digest must be valid"),
    }
}

fn pack(id: &str, digest: &str) -> NormativePackReleaseV1 {
    NormativePackReleaseV1 {
        id: NormativePackIdV1::parse(id).expect("synthetic Pack id must be valid"),
        attestor: NormativePackAttestorV1::parse("example.owner")
            .expect("synthetic attestor must be valid"),
        version: NormativePackVersionV1::parse("1.0.0")
            .expect("synthetic Pack version must be valid"),
        content_digest: NormativePackDigest::parse(digest)
            .expect("synthetic Pack digest must be valid"),
    }
}

#[test]
fn canonicalization_is_order_independent_and_idempotent() {
    let a = package("example.a", "1.0.0", &"a".repeat(64));
    let b = package("example.b", "1.0.0", &"b".repeat(64));
    let pack = pack("example.policy", &"c".repeat(64));

    let first = canonicalize_release_closures(
        vec![b.clone(), a.clone(), a.clone()],
        vec![pack.clone(), pack.clone()],
    )
    .expect("identical coordinates must coalesce");
    let second = canonicalize_release_closures(vec![a.clone(), b.clone()], vec![pack.clone()])
        .expect("canonical coordinates must remain admitted");

    assert_eq!(first, second);
    assert_eq!(first.packages, vec![a, b]);
    assert_eq!(first.normative_packs, vec![pack]);
}

#[test]
fn canonicalization_rejects_conflicting_package_and_pack_owners() {
    let packages = vec![
        package("example.a", "1.0.0", &"a".repeat(64)),
        package("example.a", "2.0.0", &"b".repeat(64)),
    ];
    assert!(matches!(
        canonicalize_release_closures(packages, Vec::new()),
        Err(ReleaseClosureErrorV1::ConflictingPackageRelease { .. })
    ));

    let first = pack("example.policy", &"c".repeat(64));
    let mut conflicting = first.clone();
    conflicting.version =
        NormativePackVersionV1::parse("2.0.0").expect("synthetic Pack version must be valid");
    assert!(matches!(
        canonicalize_release_closures(Vec::new(), vec![first, conflicting]),
        Err(ReleaseClosureErrorV1::ConflictingNormativePackRelease { .. })
    ));
}

#[test]
fn canonicalization_checks_bounds_before_coalescing() {
    let duplicate = package("example.a", "1.0.0", &"a".repeat(64));
    let packages = vec![duplicate; MAX_SELECTED_PACKAGE_RELEASES + 1];

    assert!(matches!(
        canonicalize_release_closures(packages, Vec::new()),
        Err(ReleaseClosureErrorV1::TooManyPackages { .. })
    ));
}

#[test]
fn package_and_pack_coordinates_are_not_substitutable() {
    let package = serde_json::to_value(package("example.a", "1.0.0", &"a".repeat(64)))
        .expect("Package coordinate must serialize");
    let pack = serde_json::to_value(pack("example.policy", &"b".repeat(64)))
        .expect("Pack coordinate must serialize");

    assert!(package.get("attestor").is_none());
    assert_eq!(pack["attestor"], "example.owner");
}

#[test]
fn release_closures_reject_duplicate_semantic_owners() {
    let packages = vec![
        package("example.a", "1.0.0", &"a".repeat(64)),
        package("example.a", "2.0.0", &"b".repeat(64)),
    ];

    assert!(matches!(
        validate_release_closures(&packages, &[]),
        Err(ReleaseClosureErrorV1::NonCanonicalPackageOrder { .. })
    ));
}

#[test]
fn unselected_release_bytes_do_not_enter_a_selected_frontier() {
    let selected = vec![package("example.a", "1.0.0", &"a".repeat(64))];
    validate_release_closures(&selected, &[]).expect("selected closure must be valid");
    let first = crate::canon::digest(&selected).expect("selected closure must hash");

    let _unselected = package("example.b", "1.0.0", &"b".repeat(64));
    let second = crate::canon::digest(&selected).expect("selected closure must hash again");

    assert_eq!(first, second);
}
