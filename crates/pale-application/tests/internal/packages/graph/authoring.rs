//! Package definitions and references keep their authored editor coordinates.

use super::*;
use crate::workspace::AuthoringInstancePathSegment::{Field, Index};
use pale_core::paths::OwnedConfinedRelativePath;

fn resource_id(value: &str) -> pale_core::records::PackageResourceIdV1 {
    pale_core::records::PackageResourceIdV1::parse(value).expect("valid resource identity")
}

fn package_id(value: &str) -> PackageIdV1 {
    PackageIdV1::parse(value).expect("valid Package identity")
}

fn local(value: &str) -> PackageResourceLinkV1 {
    PackageResourceLinkV1::Local {
        resource_id: resource_id(value),
    }
}

#[test]
fn catalog_resources_keep_their_substantive_definition_and_reference_paths() {
    let external = PackageResourceRefV1 {
        package_id: package_id("dependency"),
        resource_id: resource_id("doctrine"),
    };
    let resource = ParsedResource::Role(SpecialistRole {
        id: resource_id("reviewer"),
        description: "Review changes.".into(),
        doctrine: OwnedConfinedRelativePath::parse("reviewer.md").expect("confined body"),
        extends: vec![local("base-reviewer")],
        inherits_lenses: Vec::new(),
        doctrine_refs: vec![PackageResourceLinkV1::External {
            resource: external.clone(),
        }],
        applies_to: Vec::new(),
    });

    let source = package_authoring_source(&resource, 2, "package/roles.toml".into());
    assert_eq!(
        source.definition_path,
        vec![
            Field("roles".into()),
            Field("role".into()),
            Index(2),
            Field("id".into()),
        ]
    );

    let references =
        package_resource_references(&resource, &package_id("owner"), &source.content_path);
    assert_eq!(references.len(), 2);
    assert_eq!(
        references[0].expected,
        Some(PackageResourceKindV1::RoleArchetype)
    );
    assert_eq!(
        references[1].expected,
        Some(PackageResourceKindV1::Doctrine)
    );
    assert_eq!(
        references[0].target,
        PackageResourceRefV1 {
            package_id: package_id("owner"),
            resource_id: resource_id("base-reviewer"),
        }
    );
    assert_eq!(
        references[0].path,
        vec![
            Field("roles".into()),
            Field("role".into()),
            Index(2),
            Field("extends".into()),
            Index(0),
            Field("resource_id".into()),
        ]
    );
    assert_eq!(references[1].target, external);
    assert_eq!(
        references[1].path,
        vec![
            Field("roles".into()),
            Field("role".into()),
            Index(2),
            Field("doctrine_refs".into()),
            Index(0),
            Field("resource".into()),
            Field("resource_id".into()),
        ]
    );
}

#[test]
fn singleton_manifest_identities_define_the_document_not_an_invented_scalar() {
    let governing = ParsedResource::GoverningOrder {
        id: resource_id("governing"),
        record: GoverningOrderV1 {
            schema_version: 1,
            constraints: Vec::new(),
            obligations: Vec::new(),
            exclusive_authority: Vec::new(),
            narrowing: Vec::new(),
            order: Vec::new(),
        },
    };
    let governing_source = package_authoring_source(&governing, 0, "package/governing.toml".into());
    assert!(governing_source.definition_path.is_empty());
    assert_eq!(
        governing_source.content_path,
        vec![Field("governing_order".into())]
    );

    let policy = ParsedResource::CapabilityPolicy {
        id: resource_id("policy"),
        record: CapabilityPolicy {
            permitted_capabilities: vec![pale_core::records::CapabilityIdV1::parse(
                "repository.read",
            )
            .expect("valid capability")],
            forbidden_capabilities: Vec::new(),
            permitted_effects: Vec::new(),
            forbidden_effects: Vec::new(),
        },
    };
    let policy_source = package_authoring_source(&policy, 0, "package/policy.toml".into());
    assert!(policy_source.definition_path.is_empty());
    assert!(policy_source.content_path.is_empty());
    assert_eq!(
        package_capability_references(&policy, &policy_source.content_path)[0].1,
        vec![Field("permitted_capabilities".into()), Index(0)]
    );
}

#[test]
fn operation_recipe_and_capability_occurrences_follow_the_authored_policy_shape() {
    let fallback_recipe = PackageResourceRefV1 {
        package_id: package_id("dependency"),
        resource_id: resource_id("fallback-recipe"),
    };
    let operation = ParsedResource::Operation(OnDemandOperation {
        id: resource_id("review"),
        description: "Review changes.".into(),
        doctrine: OwnedConfinedRelativePath::parse("review.md").expect("confined body"),
        extends: Vec::new(),
        doctrine_refs: Vec::new(),
        roles: Vec::new(),
        outputs: Vec::new(),
        capability_requirements: vec![OperationCapabilityRequirement {
            id: CapabilityRequirementIdV1::parse("repository").expect("valid requirement"),
            description: "Read the repository.".into(),
            capability: pale_core::records::CapabilityIdV1::parse("repository.read")
                .expect("valid capability"),
            recipe: Some(local("primary-recipe")),
            effect_class: EffectClass::Read,
            policy: CapabilityRequirementPolicy::Optional {
                fallback: CapabilityFallback {
                    guidance: "Continue from admitted context.".into(),
                    recipe: Some(PackageResourceLinkV1::External {
                        resource: fallback_recipe.clone(),
                    }),
                },
            },
            failure_posture: FailurePosture::FailOpen,
        }],
        capability_policy: Some(local("review-policy")),
    });
    let source = package_authoring_source(&operation, 0, "package/operations.toml".into());
    let references =
        package_resource_references(&operation, &package_id("owner"), &source.content_path);

    assert_eq!(references.len(), 3);
    assert_eq!(
        references
            .iter()
            .map(|reference| reference.expected)
            .collect::<Vec<_>>(),
        vec![
            Some(PackageResourceKindV1::ToolRecipe),
            Some(PackageResourceKindV1::ToolRecipe),
            Some(PackageResourceKindV1::CapabilityPolicy)
        ]
    );
    assert_eq!(
        references[0].path,
        vec![
            Field("operations".into()),
            Field("operation".into()),
            Index(0),
            Field("capability_requirements".into()),
            Index(0),
            Field("recipe".into()),
            Field("resource_id".into()),
        ]
    );
    assert_eq!(references[1].target, fallback_recipe);
    assert_eq!(
        references[1].path,
        vec![
            Field("operations".into()),
            Field("operation".into()),
            Index(0),
            Field("capability_requirements".into()),
            Index(0),
            Field("policy".into()),
            Field("fallback".into()),
            Field("recipe".into()),
            Field("resource".into()),
            Field("resource_id".into()),
        ]
    );
    assert_eq!(
        references[2].path,
        vec![
            Field("operations".into()),
            Field("operation".into()),
            Index(0),
            Field("capability_policy".into()),
            Field("resource_id".into()),
        ]
    );
    assert_eq!(
        package_capability_references(&operation, &source.content_path)[0].1,
        vec![
            Field("operations".into()),
            Field("operation".into()),
            Index(0),
            Field("capability_requirements".into()),
            Index(0),
            Field("capability".into()),
        ]
    );
}
