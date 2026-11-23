//! Untyped governing endpoints must stay unique before editor indexing.

use super::*;
use crate::packages::PackageReleaseIndex;
use pale_core::paths::OwnedConfinedRelativePath;
use pale_core::records::{
    DistributionLockFile, DistributionMeta, GoverningNarrowingV1, PackageResourceIdV1,
    SchemaManifestDigest, SchemaSetDigest, SemanticOrderPurposeV1,
};

fn resource(id: &str) -> PackageResourceRefV1 {
    PackageResourceRefV1 {
        package_id: PackageIdV1::parse("example.governing").unwrap(),
        resource_id: PackageResourceIdV1::parse(id).unwrap(),
    }
}

fn local(id: &str) -> GoverningResourceLinkV1 {
    GoverningResourceLinkV1::Local {
        kind: None,
        resource_id: resource(id).resource_id,
    }
}

fn declarations(
    kinds: &[PackageResourceKindV1],
) -> BTreeMap<PackageResourceUse, PackageResourceV1> {
    std::iter::once(("first", PackageResourceKindV1::Doctrine))
        .chain(kinds.iter().map(|kind| ("second", *kind)))
        .map(|(id, kind)| {
            (
                PackageResourceUse::new(resource(id), kind),
                PackageResourceV1 {
                    id: resource(id).resource_id,
                    kind,
                    definition_source: OwnedConfinedRelativePath::parse("source.toml").unwrap(),
                    exposure: PackageResourceExposureV1::Exported,
                },
            )
        })
        .collect()
}

fn with_owner(check: impl FnOnce(&RetainedPackageRelease)) {
    let root = tempfile::tempdir().unwrap();
    crate::package_fixtures::write_policy_package(
        root.path(),
        "governing",
        "example.governing",
        "# Policy\n",
    );
    let snapshot =
        DistributionSnapshot::capture(&pale_adapter_fs::LocalSourceFilesystem, root.path())
            .unwrap();
    let index = PackageReleaseIndex::capture(&snapshot).unwrap();
    let lock = DistributionLockFile {
        distribution: DistributionMeta {
            compiler: "pale-surface@1".into(),
            schema_manifest_sha256: SchemaManifestDigest::parse("0".repeat(64)).unwrap(),
            schema_set_sha256: SchemaSetDigest::parse("1".repeat(64)).unwrap(),
        },
        packages: index.releases().cloned().collect(),
    };
    let closure = index
        .select(&lock, &[resource("first").package_id])
        .unwrap();
    let (_, owner) = closure.packages().next().unwrap();
    check(owner);
}

fn check_endpoints(
    qualify: impl Fn(
        &RetainedPackageRelease,
        &BTreeMap<PackageResourceUse, PackageResourceV1>,
    ) -> Result<(), PackageError>,
) {
    with_owner(|owner| {
        for (kinds, expected) in [
            (
                vec![
                    PackageResourceKindV1::Doctrine,
                    PackageResourceKindV1::Operation,
                ],
                ResourceLinkViolation::AmbiguousTarget,
            ),
            (
                vec![PackageResourceKindV1::Operation],
                ResourceLinkViolation::WrongKind,
            ),
        ] {
            let error = qualify(owner, &declarations(&kinds)).expect_err(
                "an ambiguous or wrong-kind governing endpoint must fail before indexing",
            );
            assert!(
                matches!(error, PackageError::InvalidResourceLink { reason, .. } if reason == expected)
            );
        }
        qualify(owner, &declarations(&[PackageResourceKindV1::Doctrine])).unwrap();
    });
}

#[test]
fn narrowing_rejects_ambiguous_broader_and_preserves_equal_kind_requirement() {
    check_endpoints(|owner, declarations| {
        qualify_narrowing(
            &GoverningNarrowingV1 {
                scope: local("first"),
                narrower: local("first"),
                broader: local("second"),
            },
            &resource("order"),
            owner,
            declarations,
        )
        .map(|_| ())
    });
}

#[test]
fn order_rejects_ambiguous_after_and_preserves_equal_kind_requirement() {
    check_endpoints(|owner, declarations| {
        qualify_order(
            &ScopedOrderRelationV1 {
                scope: local("first"),
                purpose: SemanticOrderPurposeV1::Presentation,
                before: local("first"),
                after: local("second"),
            },
            &resource("order"),
            owner,
            declarations,
        )
        .map(|_| ())
    });
}

#[test]
fn explicit_governing_kinds_resolve_shared_page_uses_independently() {
    with_owner(|owner| {
        use PackageResourceKindV1 as Kind;
        let declarations = declarations(&[Kind::Doctrine, Kind::Role]);
        let typed = |kind| GoverningResourceLinkV1::Local {
            resource_id: resource("second").resource_id,
            kind: Some(kind),
        };
        let mut edge = ScopedOrderRelationV1 {
            scope: typed(Kind::Role),
            purpose: SemanticOrderPurposeV1::Reasoning,
            before: local("first"),
            after: typed(Kind::Doctrine),
        };
        let resolved = qualify_order(&edge, &resource("order"), owner, &declarations).unwrap();
        assert_eq!(
            resolved.scope,
            PackageResourceUse::new(resource("second"), Kind::Role)
        );
        assert_eq!(
            resolved.after,
            PackageResourceUse::new(resource("second"), Kind::Doctrine)
        );
        edge.after = typed(Kind::Stack);
        assert!(matches!(
            qualify_order(&edge, &resource("order"), owner, &declarations),
            Err(PackageError::InvalidResourceLink {
                reason: ResourceLinkViolation::WrongKind,
                ..
            })
        ));
        edge.after = typed(Kind::Role);
        assert!(matches!(
            qualify_order(&edge, &resource("order"), owner, &declarations),
            Err(PackageError::InvalidResourceLink {
                reason: ResourceLinkViolation::WrongKind,
                ..
            })
        ));
        edge.after = local("second");
        assert!(matches!(
            qualify_order(&edge, &resource("order"), owner, &declarations),
            Err(PackageError::InvalidResourceLink {
                reason: ResourceLinkViolation::AmbiguousTarget,
                ..
            })
        ));
    });
}
