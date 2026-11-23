//! Repository-local policy-family presentation-order tests.

use pale_core::records::{
    AdmittedScopedOrderRelationV1, PackageIdV1, PackageResourceIdV1, PackageResourceUseV1,
    ResolvedGoverningContributionV1,
};

use super::*;

fn resource(id: &str) -> PackageResourceRefV1 {
    PackageResourceRefV1 {
        package_id: PackageIdV1::parse("example.semantic").expect("valid Package id"),
        resource_id: PackageResourceIdV1::parse(id).expect("valid resource id"),
    }
}

fn family(id: &str) -> ResolvedPolicyFamily {
    ResolvedPolicyFamily {
        id: resource(id),
        title: id.to_string(),
        purpose: format!("purpose for {id}"),
        body: format!("# {id}\n").into(),
        body_sources: Vec::new(),
    }
}

fn empty_plan() -> RepositorySemanticPlan {
    RepositorySemanticPlan {
        page_reading: BTreeMap::new(),
        governing_order: ResolvedGoverningContributionV1::compose([], &BTreeMap::new()).unwrap(),
        operations: Vec::new(),
        roles: Vec::new(),
        doctrine: Vec::new(),
        startup_policies: Vec::new(),
        run_modes: Vec::new(),
        mode_defaults: BTreeMap::new(),
        mode_default_sources: BTreeMap::new(),
        stacks: Vec::new(),
        policy_families: Vec::new(),
        recipes: Vec::new(),
        repository_verifier_capabilities: Vec::new(),
    }
}

#[test]
fn repository_renderer_consumes_only_the_plan_local_governing_order() {
    let alpha = resource("alpha");
    let zeta = resource("zeta");
    for (kind, expected) in [
        (
            PackageResourceKindV1::PolicyFamily,
            vec![zeta.clone(), alpha.clone()],
        ),
        (
            PackageResourceKindV1::Doctrine,
            vec![alpha.clone(), zeta.clone()],
        ),
    ] {
        let governing_order = ResolvedGoverningContributionV1::compose(
            [ResolvedGoverningContributionV1 {
                constraints: Vec::new(),
                obligations: Vec::new(),
                exclusive_authority: Vec::new(),
                narrowing: Vec::new(),
                order: vec![AdmittedScopedOrderRelationV1 {
                    scope: PackageResourceUseV1::new(
                        resource("repository-scope"),
                        PackageResourceKindV1::Role,
                    ),
                    purpose: SemanticOrderPurposeV1::Presentation,
                    before: PackageResourceUseV1::new(zeta.clone(), kind),
                    after: PackageResourceUseV1::new(alpha.clone(), kind),
                    contributor: resource("governing"),
                }],
            }],
            &BTreeMap::new(),
        )
        .expect("acyclic local presentation order");
        let mut plan = empty_plan();
        plan.governing_order = governing_order;
        plan.policy_families = vec![family("alpha"), family("zeta")];

        let ordered = ordered_families(&plan).expect("local presentation order must render");
        assert_eq!(
            ordered
                .into_iter()
                .map(|entry| entry.family.id.clone())
                .collect::<Vec<_>>(),
            expected
        );
    }
}

#[test]
fn policy_skill_uses_the_page_title_once_and_keeps_its_body() {
    let mut policy = family("review");
    policy.title = "Old catalog title".to_string();
    policy.body = "# Review the finished change\n\nRead the changed code.\n\n## Check the result\n\nRun the relevant test.\n\n```markdown\n# A heading inside an example\n```\n".into();
    for host in [
        TerminalAdapterDescriptor::claude_code(),
        TerminalAdapterDescriptor::codex(),
    ] {
        let rendered = skill_content(&policy, "profile-digest", host, &empty_plan()).unwrap();
        assert_eq!(rendered.matches("# Review the finished change").count(), 1);
        assert!(!rendered.contains("Old catalog title"));
        assert_eq!(rendered.matches("purpose for review").count(), 1);
        assert!(rendered.contains("Package: `example.semantic`"));
        assert!(rendered.contains("## Check the result\n\nRun the relevant test."));
        assert!(rendered.contains("```markdown\n# A heading inside an example\n```"));
    }
}

#[test]
fn page_navigation_links_prerequisites_without_copying_them_into_each_use() {
    use crate::packages::{PackageResourceUse, ResourceTextReference};
    use crate::semantic::{
        ResolvedDoctrine, ResolvedDoctrineRecord, ResolvedPageReading, ResolvedStack,
    };
    use pale_core::records::{DoctrinePlacement, PackageResourceKindV1 as Kind};

    let mut plan = empty_plan();
    plan.doctrine.push(ResolvedDoctrine {
        record: ResolvedDoctrineRecord {
            id: resource("change"),
            description: "Understand the change".into(),
            placement: DoctrinePlacement::StandaloneJudgment,
            requires: Vec::new(),
            see_also: Vec::new(),
            official_sources: Vec::new(),
        },
        title: "Change [carefully]".into(),
        body: "UNIQUE PARENT BODY".into(),
        related_reading: Vec::new(),
        body_sources: Vec::new(),
    });
    let policy = family("change/review");
    plan.policy_families.push(family("optional"));
    for kind in [
        Kind::Doctrine,
        Kind::Operation,
        Kind::Role,
        Kind::PolicyFamily,
        Kind::Stack,
    ] {
        plan.page_reading.insert(
            PackageResourceUse::new(policy.id.clone(), kind),
            ResolvedPageReading {
                authored_page: true,
                official_sources: Vec::new(),
                required: vec![resource("change")],
                related: vec![ResourceTextReference {
                    target: resource("optional"),
                    title: "Optional".into(),
                    path: "optional.md".into(),
                    fragment: None,
                }],
            },
        );
        let navigation =
            super::super::body_links::page_navigation(&policy.id, kind, &plan, "../").unwrap();
        assert!(navigation.contains(
            "[Change \\[carefully\\]](../pale-doctrine-example-semantic-change/SKILL.md)"
        ));
        assert!(navigation.contains("[Optional](../pale-example-semantic-optional/SKILL.md)"));
        assert!(!navigation.contains("UNIQUE PARENT BODY"));
    }
    let rendered =
        skill_content(&policy, "digest", TerminalAdapterDescriptor::codex(), &plan).unwrap();
    assert_eq!(rendered.matches("## Read first").count(), 1);
    assert!(!rendered.contains("UNIQUE PARENT BODY"));
    for (origin, prefix) in [
        (
            ".pale/source-guidance/roles/reviewer.md",
            "../../../.agents/skills/",
        ),
        (
            ".pale/providers/local/internal-operations/review.md",
            "../../../../.agents/skills/",
        ),
    ] {
        let navigation = super::super::body_links::navigation_at(
            &policy.id,
            Kind::Role,
            &plan,
            origin,
            ".agents/skills",
        )
        .unwrap();
        assert!(navigation.contains(&format!(
            "({prefix}pale-doctrine-example-semantic-change/SKILL.md)"
        )));
    }
    let stack = ResolvedStack {
        id: policy.id.clone(),
        description: "Review stack".into(),
        extends: Vec::new(),
        doctrine: Vec::new(),
        role_doctrine: Vec::new(),
        official_sources: Vec::new(),
        body: Some("# Review methods\n\nSTACK BODY\n".into()),
    };
    let rendered = super::super::stack_content::stack_skill(&stack, &plan)
        .unwrap()
        .unwrap();
    assert_eq!(rendered.matches("# Review methods").count(), 1);
    assert!(rendered.contains("STACK BODY"));
    assert!(rendered.contains("## Read first"));
    assert!(!rendered.contains("UNIQUE PARENT BODY"));

    plan.doctrine.clear();
    assert!(skill_content(&policy, "digest", TerminalAdapterDescriptor::codex(), &plan).is_err());
}
