//! Package resource-link qualification tests.

use pale_core::paths::OwnedConfinedRelativePath;
use pale_core::records::{
    PackageDependencyV1, PackageIdV1, PackageResourceExposureV1, PackageResourceIdV1,
    PackageResourceKindV1, PackageResourceLinkV1, PackageResourceRefV1, PackageResourceV1,
    PackageVersionV1,
};

use super::{check_link_against, PackageError};
use crate::packages::{PackageResourceUse, ResourceLinkRelation, ResourceLinkViolation};

fn package(id: &str) -> PackageIdV1 {
    PackageIdV1::parse(id).expect("valid test Package id")
}

fn resource(package_id: &str, resource_id: &str) -> PackageResourceRefV1 {
    PackageResourceRefV1 {
        package_id: package(package_id),
        resource_id: PackageResourceIdV1::parse(resource_id).expect("valid test resource id"),
    }
}

fn declaration(
    id: &str,
    kind: PackageResourceKindV1,
    exposure: PackageResourceExposureV1,
) -> PackageResourceV1 {
    PackageResourceV1 {
        id: PackageResourceIdV1::parse(id).expect("valid test resource id"),
        kind,
        definition_source: OwnedConfinedRelativePath::parse("doctrine.toml")
            .expect("valid declaration path"),
        exposure,
    }
}

fn dependency(id: &str) -> PackageDependencyV1 {
    PackageDependencyV1 {
        id: package(id),
        version: PackageVersionV1::parse("1.0.0").expect("valid test version"),
    }
}

fn reason(error: PackageError) -> ResourceLinkViolation {
    let PackageError::InvalidResourceLink { reason, .. } = error else {
        panic!("qualification failure must retain its typed Link cause");
    };
    reason
}

#[test]
fn link_qualification_covers_the_complete_owner_export_and_kind_matrix() {
    let from = resource("example.owner", "consumer");
    let local = resource("example.owner", "local");
    let exported = resource("example.direct", "exported");
    let internal = resource("example.direct", "internal");
    let transitive = resource("example.transitive", "exported");
    let wrong_kind = resource("example.direct", "role");
    let declarations = [
        (
            local.clone(),
            declaration(
                "local",
                PackageResourceKindV1::Doctrine,
                PackageResourceExposureV1::Internal,
            ),
        ),
        (
            exported.clone(),
            declaration(
                "exported",
                PackageResourceKindV1::Doctrine,
                PackageResourceExposureV1::Exported,
            ),
        ),
        (
            internal.clone(),
            declaration(
                "internal",
                PackageResourceKindV1::Doctrine,
                PackageResourceExposureV1::Internal,
            ),
        ),
        (
            transitive.clone(),
            declaration(
                "exported",
                PackageResourceKindV1::Doctrine,
                PackageResourceExposureV1::Exported,
            ),
        ),
        (
            wrong_kind.clone(),
            declaration(
                "role",
                PackageResourceKindV1::Role,
                PackageResourceExposureV1::Exported,
            ),
        ),
    ]
    .into_iter()
    .map(|(reference, declaration)| {
        (
            PackageResourceUse::new(reference, declaration.kind),
            declaration,
        )
    })
    .collect();
    let dependencies = [dependency("example.direct")];
    let qualify = |link: PackageResourceLinkV1| {
        check_link_against(
            &link,
            &from,
            &dependencies,
            &declarations,
            Some(PackageResourceKindV1::Doctrine),
            ResourceLinkRelation::DoctrineExtends,
        )
    };

    assert_eq!(
        qualify(PackageResourceLinkV1::Local {
            resource_id: local.resource_id.clone(),
        })
        .unwrap(),
        local
    );
    assert_eq!(
        qualify(PackageResourceLinkV1::External {
            resource: exported.clone(),
        })
        .unwrap(),
        exported
    );
    assert_eq!(
        reason(
            qualify(PackageResourceLinkV1::External {
                resource: from.clone(),
            })
            .unwrap_err()
        ),
        ResourceLinkViolation::SelfExternal
    );
    assert_eq!(
        reason(
            qualify(PackageResourceLinkV1::External {
                resource: transitive,
            })
            .unwrap_err()
        ),
        ResourceLinkViolation::NotDirectDependency
    );
    assert_eq!(
        reason(qualify(PackageResourceLinkV1::External { resource: internal }).unwrap_err()),
        ResourceLinkViolation::InternalTarget
    );
    assert_eq!(
        reason(
            qualify(PackageResourceLinkV1::External {
                resource: wrong_kind,
            })
            .unwrap_err()
        ),
        ResourceLinkViolation::WrongKind
    );
    assert_eq!(
        reason(
            qualify(PackageResourceLinkV1::Local {
                resource_id: PackageResourceIdV1::parse("missing").expect("valid missing id"),
            })
            .unwrap_err()
        ),
        ResourceLinkViolation::MissingTarget
    );
}

#[test]
fn equal_local_ids_in_distinct_packages_never_alias() {
    let from = resource("example.owner", "consumer");
    let local = resource("example.owner", "shared");
    let external = resource("example.direct", "shared");
    let declarations = [
        (
            local.clone(),
            declaration(
                "shared",
                PackageResourceKindV1::Doctrine,
                PackageResourceExposureV1::Internal,
            ),
        ),
        (
            external,
            declaration(
                "shared",
                PackageResourceKindV1::Doctrine,
                PackageResourceExposureV1::Exported,
            ),
        ),
    ]
    .into_iter()
    .map(|(reference, declaration)| {
        (
            PackageResourceUse::new(reference, declaration.kind),
            declaration,
        )
    })
    .collect();
    let qualified = check_link_against(
        &PackageResourceLinkV1::Local {
            resource_id: local.resource_id.clone(),
        },
        &from,
        &[dependency("example.direct")],
        &declarations,
        Some(PackageResourceKindV1::Doctrine),
        ResourceLinkRelation::DoctrineExtends,
    )
    .unwrap();
    assert_eq!(qualified, local);
}

#[test]
fn shared_page_links_keep_the_requested_kind_and_its_visibility() {
    let from = resource("example.owner", "consumer");
    let target = resource("example.direct", "shared");
    let declarations = [
        declaration(
            "shared",
            PackageResourceKindV1::Doctrine,
            PackageResourceExposureV1::Internal,
        ),
        declaration(
            "shared",
            PackageResourceKindV1::Operation,
            PackageResourceExposureV1::Exported,
        ),
    ]
    .into_iter()
    .map(|declaration| {
        (
            PackageResourceUse::new(target.clone(), declaration.kind),
            declaration,
        )
    })
    .collect();
    let link = PackageResourceLinkV1::External {
        resource: target.clone(),
    };
    let dependencies = [dependency("example.direct")];
    let qualify = |kind| {
        check_link_against(
            &link,
            &from,
            &dependencies,
            &declarations,
            kind,
            ResourceLinkRelation::GoverningObligation,
        )
    };
    assert_eq!(
        qualify(Some(PackageResourceKindV1::Operation)).unwrap(),
        target
    );
    assert_eq!(
        reason(qualify(Some(PackageResourceKindV1::Doctrine)).unwrap_err()),
        ResourceLinkViolation::InternalTarget
    );
    assert_eq!(
        reason(qualify(Some(PackageResourceKindV1::Role)).unwrap_err()),
        ResourceLinkViolation::WrongKind
    );
    assert_eq!(
        reason(qualify(None).unwrap_err()),
        ResourceLinkViolation::AmbiguousTarget
    );
}
