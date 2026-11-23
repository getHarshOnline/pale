//! Check each live header-authored entry in isolation through both host renderers.
//!
//! These are render checks, not observations of an agent using the instructions.

use super::*;

fn exact_selection(package: &str, id: &str) -> toml::Value {
    toml::from_str(&format!(
        "selection = 'exact'\n[resource]\npackage_id = '{package}'\nresource_id = '{id}'"
    ))
    .unwrap()
}

fn isolated_entry(package: &str, kind: &str, id: &str) -> SyntheticWorkspace {
    let operations = if kind == "operation" {
        vec![(package, id)]
    } else {
        Vec::new()
    };
    let fixture = fixture(&operations);
    let mut workspace: toml::Value =
        toml::from_str(&std::fs::read_to_string(&fixture.manifest).unwrap()).unwrap();
    if kind != "role" {
        role_package_fixture::write_role_package(
            &fixture.root.path().join("distribution"),
            "engine",
            "example.engine",
            "# Repository context\n\nNo additional guidance.\n",
        );
    }
    let mut seeds = vec![package.to_string()];
    if kind != "role" {
        seeds.push("example.engine".into());
    }
    seeds.sort();
    workspace["workspace"]["desired_topology"]["projects"][0]["package_seeds"] =
        toml::Value::Array(seeds.into_iter().map(toml::Value::String).collect());
    let membership = &mut workspace["workspace"]["desired_topology"]["project_repositories"][0];
    // A repository must have a role. Outside a role case, keep that structural
    // requirement free of unrelated dependencies that could hide missing links.
    membership["role"]["package_id"] = if kind == "role" {
        package
    } else {
        "example.engine"
    }
    .into();
    membership["role"]["resource_id"] = if kind == "role" { id } else { "engine" }.into();
    membership["stacks"] = toml::Value::Array(Vec::new());
    membership["families"] = toml::Value::Array(if kind == "policy_family" {
        vec![exact_selection(package, id)["resource"].clone()]
    } else {
        Vec::new()
    });
    std::fs::write(&fixture.manifest, toml::to_string(&workspace).unwrap()).unwrap();
    let path = fixture.root.path().join("workspace/capabilities.toml");
    let mut capabilities: toml::Value =
        toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    for (entry_kind, field) in [
        ("role", "roles"),
        ("stack", "stacks"),
        ("doctrine", "doctrine"),
    ] {
        capabilities["semantic"].as_table_mut().unwrap().insert(
            field.into(),
            toml::Value::Array(if kind == entry_kind {
                vec![exact_selection(package, id)]
            } else {
                Vec::new()
            }),
        );
    }
    std::fs::write(path, toml::to_string(&capabilities).unwrap()).unwrap();
    fixture.refresh_distribution_lock();
    fixture
}

fn check_independent_entries(directory: &str) -> (usize, usize) {
    let manifest: toml::Value = toml::from_str(
        &std::fs::read_to_string(
            source_root()
                .join("packages")
                .join(directory)
                .join("package.toml"),
        )
        .unwrap(),
    )
    .unwrap();
    let package = manifest["package"]["id"].as_str().unwrap();
    let resources = manifest["package"]["resources"].as_array().unwrap();
    let mut checked_links = 0;
    let mut checked_entries = 0;
    for (kind, surface_kind) in [
        ("operation", "operation-skill"),
        ("role", "specialist-agent"),
        ("policy_family", "policy-skill"),
        ("stack", "stack-skill"),
        ("doctrine", "doctrine-skill"),
    ] {
        let entries = resources
            .iter()
            .filter(|row| {
                row["kind"].as_str() == Some(kind) && row["exposure"].as_str() == Some("exported")
            })
            .collect::<Vec<_>>();
        for entry in entries {
            let id = entry["id"].as_str().unwrap();
            let fixture = isolated_entry(package, kind, id);
            let compilation = fixture
                .compile()
                .unwrap_or_else(|error| panic!("isolated {kind} {id}: {error:?}"));
            let resource_label = format!("{package}:{id}");
            let trace_label = if kind == "policy_family" {
                resource_label
            } else {
                format!("{kind}:{resource_label}")
            };
            let surfaces = &compilation.repositories()[0].render().surfaces;
            for host in ["codex", "claude"] {
                let prefix = format!(".{host}/");
                assert!(
                    surfaces.iter().any(|surface| {
                        surface.rel_path.starts_with(&prefix)
                            && surface.kind == surface_kind
                            && surface.referenced_policy_ids.contains(&trace_label)
                    }),
                    "isolated {kind} {id}: missing {host} entry"
                );
                let mut checked = 0;
                let mut shared_design_models = 0;
                for surface in surfaces.iter().filter(|surface| {
                    surface.rel_path.starts_with(&prefix)
                        && surface.rel_path.contains(directory)
                        && (surface.rel_path.ends_with(".md")
                            || surface.rel_path.ends_with(".toml"))
                }) {
                    let text = if surface.rel_path.ends_with(".toml") {
                        let record: toml::Value = toml::from_str(surface.content()).unwrap();
                        record["developer_instructions"]
                            .as_str()
                            .unwrap()
                            .to_owned()
                    } else {
                        surface.content().to_owned()
                    };
                    let context = format!("isolated {kind} {id}: {}", surface.rel_path);
                    let sections = text.split("## Before handing off").collect::<Vec<_>>();
                    if directory == "design"
                        && surface.kind == "doctrine-skill"
                        && surface
                            .referenced_policy_ids
                            .contains(&"doctrine:pale.design:design".to_string())
                    {
                        assert!(
                            text.contains("# Carry meaning from the source to the rendered design"),
                            "{context}"
                        );
                        assert_eq!(
                            sections.len(),
                            1,
                            "shared model has no method checklist: {context}"
                        );
                        shared_design_models += 1;
                        continue;
                    }
                    assert_eq!(sections.len(), 2, "{context}");
                    let questions = sections[1].trim().lines().collect::<Vec<_>>();
                    assert!((3..=4).contains(&questions.len()), "{context}");
                    assert!(
                        questions.iter().all(|line| line.starts_with("- [ ] ")),
                        "{context}"
                    );
                    checked += 1;
                }
                assert!(checked > 0, "isolated {kind} {id}: {host}");
                if directory == "design" {
                    assert_eq!(
                        shared_design_models, 1,
                        "isolated {kind} {id}: {host} needs the shared model"
                    );
                }
            }
            checked_links += assert_local_links_resolve(&compilation);
            checked_entries += 1;
        }
    }
    (checked_entries, checked_links)
}

#[test]
fn every_live_engineering_entry_renders_independently_with_resolving_links_and_closing() {
    let (entries, links) = check_independent_entries("engineering");
    assert_eq!(entries, 67);
    assert!(links > 20, "exercise links across the isolated entries");
}

#[test]
fn each_governance_policy_renders_independently_with_its_closing_questions() {
    let (entries, _) = check_independent_entries("governance");
    assert_eq!(entries, 2);
}

#[test]
fn each_product_method_renders_independently_in_its_declared_uses() {
    let (entries, links) = check_independent_entries("product");
    assert_eq!(entries, 6);
    assert!(
        links > 0,
        "task-specific Research prerequisites must resolve"
    );
}

#[test]
fn each_business_method_renders_independently_in_its_declared_uses() {
    let (entries, links) = check_independent_entries("business");
    assert_eq!(entries, 8);
    assert!(links > 8, "exercise shared review and task prerequisites");
}

#[test]
fn each_research_method_renders_independently_in_its_declared_uses() {
    let (entries, links) = check_independent_entries("research");
    assert_eq!(entries, 18);
    assert!(
        links > 12,
        "exercise research methods and optional source reading"
    );
}

#[test]
fn each_assurance_method_renders_independently_in_its_declared_uses() {
    let (entries, _) = check_independent_entries("assurance");
    assert_eq!(entries, 13);
    let fixture = isolated_entry("pale.assurance", "operation", "data/website");
    let compilation = fixture.compile().unwrap();
    let surfaces = &compilation.repositories()[0].render().surfaces;
    for host in ["codex", "claude"] {
        for (package, id, heading) in [
            (
                "pale.assurance",
                "data",
                "# Assess one operation involving personal data",
            ),
            (
                "pale.assurance",
                "rules",
                "# Research which rules apply to a proposed act",
            ),
            (
                "pale.research",
                "evidence",
                "# Judge what research evidence supports",
            ),
        ] {
            let trace = format!("doctrine:{package}:{id}");
            assert!(
                surfaces.iter().any(|surface| {
                    surface.rel_path.starts_with(&format!(".{host}/"))
                        && surface.kind == "doctrine-skill"
                        && surface.referenced_policy_ids.contains(&trace)
                        && surface.content().contains(heading)
                }),
                "{host}: website review needs readable {package}:{id}"
            );
        }
    }
}

#[test]
fn design_default_checks_reach_the_critic_without_loading_for_every_direction() {
    for (kind, id, needs_catalogue) in [
        ("doctrine", "art-direction", false),
        ("doctrine", "art-direction/recognize-defaults", true),
        ("role", "critic", true),
    ] {
        let fixture = isolated_entry("pale.design", kind, id);
        let compilation = fixture.compile().unwrap();
        let surfaces = &compilation.repositories()[0].render().surfaces;
        for host in ["codex", "claude"] {
            let has_catalogue = surfaces.iter().any(|surface| {
                surface.rel_path.starts_with(&format!(".{host}/"))
                    && surface
                        .content()
                        .contains("# Recognize unconsidered design choices")
                    && surface
                        .content()
                        .contains("## Look for clusters of defaults")
                    && surface
                        .content()
                        .contains("## Judge a cinematic treatment in time")
            });
            assert_eq!(has_catalogue, needs_catalogue, "{host}: {kind} {id}");
        }
        assert_local_links_resolve(&compilation);
    }
}

#[test]
fn each_model_work_method_renders_independently_in_its_declared_uses() {
    let (entries, _) = check_independent_entries("model-work");
    assert_eq!(entries, 17);
    let fixture = isolated_entry("pale.model-work", "stack", "hugging-face");
    let compilation = fixture.compile().unwrap();
    for host in ["codex", "claude"] {
        let surfaces = &compilation.repositories()[0].render().surfaces;
        let stack = surfaces
            .iter()
            .find(|surface| {
                surface.rel_path.starts_with(&format!(".{host}/"))
                    && surface.kind == "stack-skill"
                    && surface
                        .referenced_policy_ids
                        .contains(&"stack:pale.model-work:hugging-face".to_string())
            })
            .unwrap();
        assert!(stack
            .content()
            .contains("Resolve a branch or tag to a full commit hash"));
        assert!(stack
            .content()
            .contains("https://huggingface.co/docs/hub/api"));
    }
}

#[test]
fn each_evaluation_method_renders_independently_in_its_declared_uses() {
    let (entries, links) = check_independent_entries("evaluation");
    assert_eq!(entries, 28);
    assert!(links > 20, "exercise links across the isolated entries");
}

#[test]
fn each_automation_method_renders_independently_with_its_required_reading() {
    let (entries, links) = check_independent_entries("automation");
    assert_eq!(entries, 14);
    assert!(links > 20, "exercise links across the isolated entries");
}

#[test]
fn each_agent_work_method_renders_independently_in_its_declared_uses() {
    let (entries, links) = check_independent_entries("agent-work");
    assert_eq!(entries, 29);
    assert!(links > 0, "exercise links across the isolated entries");
}

#[test]
fn each_publication_method_renders_independently_in_its_declared_uses() {
    let (entries, links) = check_independent_entries("publication");
    assert_eq!(entries, 19);
    assert!(links > 0, "exercise links across the isolated entries");
}

#[test]
fn each_architecture_method_renders_independently_in_its_declared_uses() {
    let (entries, links) = check_independent_entries("architecture");
    assert_eq!(entries, 68);
    assert!(links > 0, "exercise links across the isolated entries");
}

#[test]
fn each_design_method_renders_independently_in_its_declared_uses() {
    let (entries, links) = check_independent_entries("design");
    assert_eq!(entries, 51);
    assert!(links > 0, "exercise Design reading links");
}
