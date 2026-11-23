//! Header pages retain one reading route through compilation and both host layouts.

use super::*;

#[path = "package_page_guidance/closing_questions.rs"]
mod closing_questions;

#[path = "package_page_guidance/legacy_reading.rs"]
mod legacy_reading;

fn page_fixture() -> SyntheticWorkspace {
    let fixture = SyntheticWorkspace::new(false);
    let package = fixture.root.path().join("distribution/packages/engine");
    std::fs::remove_dir_all(&package).unwrap();
    std::fs::create_dir_all(package.join("work")).unwrap();
    let mut resources = vec![
        "{ id = 'work', kind = 'doctrine', definition_source = 'work.md', exposure = 'exported' }"
            .to_owned(),
    ];
    for kind in ["doctrine", "operation", "role", "policy_family", "stack"] {
        resources.push(format!("{{ id = 'work/review', kind = '{kind}', definition_source = 'work/review.md', exposure = 'exported' }}"));
    }
    std::fs::write(package.join("package.toml"), format!(
        "[package]\nid = 'example.engine'\nversion = '1.0.0'\nschema_version = 1\ntitle = 'Page fixture'\ndescription = 'Exercise source-guidance page uses'\ncompatible_compilers = ['pale-surface@1']\nsources = [{{ path = 'work.md', role = 'declaration' }}, {{ path = 'work/review.md', role = 'declaration' }}]\nresources = [{}]\n", resources.join(", "))).unwrap();
    std::fs::write(package.join("work.md"), "+++\ndescription = 'Understand the work.'\n+++\n\n# Understand the work\n\nPARENT METHOD APPEARS ONLY HERE.\n").unwrap();
    std::fs::write(package.join("work/review.md"), "+++\ndescription = 'Review the work.'\nofficial_sources = [{ label = 'Method reference', url = 'https://example.com/method' }]\n[guide]\n[task]\n[role]\n[policy]\n[stack]\n+++\n\n# Review the work\n\nSHARED REVIEW METHOD.\n").unwrap();
    let mut manifest: toml::Value =
        toml::from_str(&std::fs::read_to_string(&fixture.manifest).unwrap()).unwrap();
    let repository = &mut manifest["workspace"]["desired_topology"]["project_repositories"][0];
    repository["role"]["resource_id"] = "work/review".into();
    let reference = repository["role"].clone();
    repository["families"] = toml::Value::Array(vec![reference.clone()]);
    repository["stacks"] = toml::Value::Array(vec![reference]);
    std::fs::write(&fixture.manifest, toml::to_string(&manifest).unwrap()).unwrap();
    let capabilities = fixture.root.path().join("workspace/capabilities.toml");
    let mut text = std::fs::read_to_string(&capabilities).unwrap();
    text.push_str("\n[semantic]\ndoctrine = [{ selection = 'exact', resource = { package_id = 'example.engine', resource_id = 'work/review' } }]\n");
    std::fs::write(capabilities, text).unwrap();
    fixture.refresh_distribution_lock();
    fixture
}

#[test]
fn header_page_compiles_and_links_in_both_source_guidance_hosts() {
    assert_header_page_surfaces(&page_fixture());
}

#[test]
fn compact_manifest_compiles_the_same_page_uses_in_both_hosts() {
    let fixture = page_fixture();
    let path = fixture
        .root
        .path()
        .join("distribution/packages/engine/package.toml");
    let mut manifest: toml::Value =
        toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let package = manifest["package"].as_table_mut().unwrap();
    package.remove("sources");
    package.remove("resources");
    std::fs::write(path, toml::to_string(&manifest).unwrap()).unwrap();
    fixture.refresh_distribution_lock();
    assert_header_page_surfaces(&fixture);
}

fn assert_header_page_surfaces(fixture: &SyntheticWorkspace) {
    let profile = effective(fixture);
    for host in [SourceGuidanceHost::Codex, SourceGuidanceHost::ClaudeCode] {
        let projection = profile
            .source_guidance(&LogicalRepositoryIdV1::parse("engine").unwrap(), host)
            .unwrap();
        assert_links_resolve(&projection);
        let files = projection.files();
        let index = files
            .iter()
            .find(|file| file.rel_path().ends_with("/index.md"))
            .unwrap();
        assert!(index
            .content()
            .contains("Open every Read first link before applying a method."));
        assert!(!index
            .content()
            .contains("already includes its required guidance"));
        let parent = files
            .iter()
            .filter(|file| file.content().contains("PARENT METHOD APPEARS ONLY HERE"))
            .collect::<Vec<_>>();
        assert_eq!(parent.len(), 1);
        let children = files
            .iter()
            .filter(|file| file.content().contains("SHARED REVIEW METHOD"))
            .collect::<Vec<_>>();
        assert_eq!(
            children.len(),
            4,
            "guide, role reference, policy and stack each carry their selected use"
        );
        for child in children {
            assert_eq!(
                child.content().matches("## Read first").count(),
                1,
                "{}",
                child.rel_path()
            );
            assert!(!child.content().contains("PARENT METHOD APPEARS ONLY HERE"));
            assert!(!child.content().contains("[task]"));
            assert_eq!(
                child
                    .content()
                    .matches("https://example.com/method")
                    .count(),
                1
            );
        }
        assert!(
            !files
                .iter()
                .any(|file| file.rel_path().contains("pale-op-")),
            "declaring a task must not select it"
        );
    }
}

fn mixed_fixture() -> SyntheticWorkspace {
    let fixture = page_fixture();
    let package = fixture.root.path().join("distribution/packages/engine");
    let path = package.join("package.toml");
    let mut manifest: toml::Value =
        toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    for (name, role, text) in [
        ("legacy-guides.toml", "declaration", "[doctrine]\nschema_version = 1\n[[doctrine.entry]]\nid = 'legacy-guide'\ndescription = 'Legacy outer guide'\ndoctrine = 'legacy-guide.md'\nplacement = 'standalone_judgment'\nrequires = [{ scope = 'local', resource_id = 'work/review' }]\nsee_also = [{ scope = 'local', resource_id = 'optional' }]\n[[doctrine.entry]]\nid = 'legacy-startup'\ndescription = 'Legacy startup guidance'\ndoctrine = 'legacy-startup.md'\nplacement = 'startup_policy'\nrequires = [{ scope = 'local', resource_id = 'work/review' }]\n"),
        ("legacy-tasks.toml", "declaration", "[operations]\nschema_version = 1\n[[operations.archetype]]\nid = 'legacy-procedure'\ndescription = 'Legacy shared procedure'\ndoctrine = 'legacy-procedure.md'\ndoctrine_refs = [{ scope = 'local', resource_id = 'work/review' }]\n[[operations.operation]]\nid = 'legacy-task'\ndescription = 'Legacy task'\ndoctrine = 'legacy-task.md'\nextends = [{ scope = 'local', resource_id = 'legacy-procedure' }]\ndoctrine_refs = [{ scope = 'local', resource_id = 'work/review' }]\n"),
        ("legacy-guide.md", "body", "# Legacy outer guide\n\nOUTER GUIDE BODY.\n"),
        ("legacy-startup.md", "body", "# Legacy startup\n\nSTARTUP BODY.\n"),
        ("legacy-procedure.md", "body", "# Shared procedure\n\nARCHETYPE BODY.\n"),
        ("legacy-task.md", "body", "# Legacy task\n\nOUTER TASK BODY.\n"),
        ("optional.md", "declaration", "+++\ndescription = 'Optional reading.'\n+++\n\n# Optional evidence\n\nOPTIONAL BODY.\n"),
    ] {
        std::fs::write(package.join(name), text).unwrap();
        manifest["package"]["sources"].as_array_mut().unwrap().push(toml::from_str(&format!("path = '{name}'\nrole = '{role}'")).unwrap());
    }
    for (id, kind, definition) in [
        ("legacy-guide", "doctrine", "legacy-guides.toml"),
        ("legacy-startup", "doctrine", "legacy-guides.toml"),
        (
            "legacy-procedure",
            "operation_archetype",
            "legacy-tasks.toml",
        ),
        ("legacy-task", "operation", "legacy-tasks.toml"),
        ("optional", "doctrine", "optional.md"),
    ] {
        manifest["package"]["resources"].as_array_mut().unwrap().push(toml::from_str(&format!("id = '{id}'\nkind = '{kind}'\ndefinition_source = '{definition}'\nexposure = 'exported'")).unwrap());
    }
    std::fs::write(path, toml::to_string(&manifest).unwrap()).unwrap();
    let path = fixture.root.path().join("workspace/capabilities.toml");
    let mut capabilities: toml::Value =
        toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    for id in ["legacy-guide", "legacy-startup"] {
        capabilities["semantic"]["doctrine"].as_array_mut().unwrap().push(toml::from_str(&format!("selection = 'exact'\nresource = {{ package_id = 'example.engine', resource_id = '{id}' }}")).unwrap());
    }
    std::fs::write(path, toml::to_string(&capabilities).unwrap()).unwrap();
    fixture.refresh_distribution_lock();
    fixture
}

#[test]
fn legacy_guides_and_startup_policy_keep_included_page_reading() {
    let fixture = mixed_fixture();
    let profile = effective(&fixture);
    for host in [SourceGuidanceHost::Codex, SourceGuidanceHost::ClaudeCode] {
        let projection = profile
            .source_guidance(&LogicalRepositoryIdV1::parse("engine").unwrap(), host)
            .unwrap();
        assert_links_resolve(&projection);
        for marker in ["OUTER GUIDE BODY", "STARTUP BODY"] {
            let file = projection
                .files()
                .iter()
                .find(|file| file.content().contains(marker))
                .unwrap();
            assert_eq!(file.content().matches("## Read first").count(), 1);
            assert!(file.content().contains("[Understand the work]"));
            assert!(file.content().contains("SHARED REVIEW METHOD"));
            assert!(!file.content().contains("PARENT METHOD APPEARS ONLY HERE"));
        }
        let outer = projection
            .files()
            .iter()
            .find(|file| file.content().contains("OUTER GUIDE BODY"))
            .unwrap();
        assert!(outer.content().contains("Optional evidence"));
        assert!(!projection
            .files()
            .iter()
            .any(|file| file.content().contains("OPTIONAL BODY")));
    }
}

#[test]
fn legacy_task_archetype_keeps_included_page_reading() {
    let fixture = mixed_fixture();
    closing_questions::add_questions(&fixture, "legacy-task.md", closing_questions::QUESTIONS);
    closing_questions::add_questions(&fixture, "work/review.md", closing_questions::QUESTIONS);
    let path = fixture.root.path().join("workspace/capabilities.toml");
    let mut capabilities: toml::Value =
        toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    capabilities["semantic"].as_table_mut().unwrap().insert("operations".into(), toml::from_str::<toml::Value>("items = [{ selection = 'exact', resource = { package_id = 'example.engine', resource_id = 'legacy-task' } }]").unwrap()["items"].clone());
    capabilities.as_table_mut().unwrap().insert("operation_bindings".into(), toml::from_str::<toml::Value>("items = [{ operation = { package_id = 'example.engine', resource_id = 'legacy-task' }, binding = { selection = { kind = 'model_or_user', aliases = [] } } }]").unwrap()["items"].clone());
    std::fs::write(path, toml::to_string(&capabilities).unwrap()).unwrap();
    let compilation = fixture.compile().unwrap();
    assert!(super::super::assert_local_links_resolve(&compilation) > 0);
    let task = compilation.repositories()[0]
        .render()
        .surfaces
        .iter()
        .find(|surface| surface.kind == "operation-skill")
        .unwrap();
    assert!(task.content().contains("ARCHETYPE BODY"));
    assert!(task.content().contains("SHARED REVIEW METHOD"));
    assert_eq!(
        task.content().matches("Before handing off").count(),
        2,
        "legacy composition retains its authored bodies until that task is migrated"
    );
    assert!(task.content().contains("[Understand the work]"));
    assert_eq!(task.content().matches("## Read first").count(), 1);
    assert!(!task.content().contains("PARENT METHOD APPEARS ONLY HERE"));
    let injection = compilation.repositories()[0]
        .render()
        .surfaces
        .iter()
        .find(|surface| surface.kind == "injection")
        .unwrap();
    assert!(injection.content().contains("STARTUP BODY"));
    assert!(injection.content().contains("[Understand the work]"));
    assert!(!injection
        .content()
        .contains("PARENT METHOD APPEARS ONLY HERE"));
}

#[test]
fn included_role_keeps_its_required_reading_without_copying_the_guide() {
    let fixture = page_fixture();
    let package = fixture.root.path().join("distribution/packages/engine");
    let path = package.join("package.toml");
    let mut manifest: toml::Value =
        toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    manifest["package"]["sources"]
        .as_array_mut()
        .unwrap()
        .push(toml::from_str("path = 'evidence.md'\nrole = 'declaration'").unwrap());
    for row in [
        "id = 'work'\nkind = 'stack'\ndefinition_source = 'work.md'\nexposure = 'internal'",
        "id = 'work'\nkind = 'role'\ndefinition_source = 'work.md'\nexposure = 'internal'",
        "id = 'evidence'\nkind = 'doctrine'\ndefinition_source = 'evidence.md'\nexposure = 'exported'",
    ] {
        manifest["package"]["resources"].as_array_mut().unwrap().push(toml::from_str(row).unwrap());
    }
    std::fs::write(path, toml::to_string(&manifest).unwrap()).unwrap();
    std::fs::write(package.join("evidence.md"), "+++\ndescription = 'Check the evidence.'\n+++\n\n# Check evidence\n\nEVIDENCE GUIDE BODY.\n").unwrap();
    let parent = package.join("work.md");
    let text = std::fs::read_to_string(&parent).unwrap().replace("+++\n\n", "[guide]\n[stack]\nexposure = 'internal'\n[role]\nexposure = 'internal'\nuses = [{ scope = 'local', resource_id = 'evidence' }]\n+++\n\n");
    std::fs::write(parent, text).unwrap();
    let child = package.join("work/review.md");
    let text = std::fs::read_to_string(&child).unwrap().replace(
        "[role]\n",
        "[role]\nextends = [{ scope = 'local', resource_id = 'work' }]\n",
    );
    let text = text.replace(
        "[stack]\n",
        "[stack]\nextends = [{ scope = 'local', resource_id = 'work' }]\n",
    );
    std::fs::write(child, text).unwrap();
    fixture.refresh_distribution_lock();
    let profile = effective(&fixture);
    for host in [SourceGuidanceHost::Codex, SourceGuidanceHost::ClaudeCode] {
        let projection = profile
            .source_guidance(&LogicalRepositoryIdV1::parse("engine").unwrap(), host)
            .unwrap();
        assert_links_resolve(&projection);
        let child = projection
            .files()
            .iter()
            .find(|file| {
                file.rel_path().contains("/roles/")
                    && file.content().contains("SHARED REVIEW METHOD")
            })
            .unwrap();
        assert!(child.content().contains("[Check evidence]"));
        assert!(!child.content().contains("EVIDENCE GUIDE BODY"));
        assert_eq!(child.content().matches("## Read first").count(), 1);
        let stack = projection
            .files()
            .iter()
            .find(|file| {
                file.rel_path().contains("pale-stack-")
                    && file.content().contains("SHARED REVIEW METHOD")
            })
            .unwrap();
        assert!(stack.content().contains("## Parent stacks"));
    }
}

#[test]
fn selected_header_task_keeps_read_first_links_in_both_runtime_layouts() {
    let fixture = page_fixture();
    let parent = fixture
        .root
        .path()
        .join("distribution/packages/engine/work.md");
    let text = std::fs::read_to_string(&parent).unwrap().replace(
        "description = 'Understand the work.'",
        "description = 'Understand the work.'\nsee_also = [{ scope = 'local', resource_id = 'work/review' }]",
    );
    std::fs::write(parent, text).unwrap();
    closing_questions::add_questions(
        &fixture,
        "work.md",
        &closing_questions::QUESTIONS.replace(
            "Does the change satisfy the request?",
            "Does the [review method](work/review.md#review-the-work) satisfy the request?",
        ),
    );
    let mut manifest: toml::Value =
        toml::from_str(&std::fs::read_to_string(&fixture.manifest).unwrap()).unwrap();
    let topology = &mut manifest["workspace"]["desired_topology"];
    topology["execution_paths"] = toml::from_str::<toml::Value>("paths = [{ id = 'direct-claude', experience_path_id = 'direct-claude-code' }, { id = 'direct-codex', experience_path_id = 'direct-codex' }]").unwrap()["paths"].clone();
    topology["execution_workspaces"][0]["execution_path_ids"] =
        toml::Value::Array(vec!["direct-claude".into(), "direct-codex".into()]);
    std::fs::write(&fixture.manifest, toml::to_string(&manifest).unwrap()).unwrap();
    let composition = composition_source_fixtures::workspace_composition_source(&[
        "direct-claude-code",
        "direct-codex",
    ]);
    std::fs::write(
        fixture.root.path().join("workspace/composition.toml"),
        toml::to_string(&composition).unwrap(),
    )
    .unwrap();
    let path = fixture.root.path().join("workspace/capabilities.toml");
    let mut capabilities: toml::Value =
        toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    capabilities["semantic"].as_table_mut().unwrap().insert("operations".into(), toml::from_str::<toml::Value>("items = [{ selection = 'exact', resource = { package_id = 'example.engine', resource_id = 'work/review' } }]").unwrap()["items"].clone());
    capabilities.as_table_mut().unwrap().insert("operation_bindings".into(), toml::from_str::<toml::Value>("items = [{ operation = { package_id = 'example.engine', resource_id = 'work/review' }, binding = { selection = { kind = 'model_or_user', aliases = [] } } }]").unwrap()["items"].clone());
    std::fs::write(path, toml::to_string(&capabilities).unwrap()).unwrap();
    let compilation = fixture.compile().unwrap();
    assert!(super::super::assert_local_links_resolve(&compilation) > 0);
    let tasks = compilation.repositories()[0]
        .render()
        .surfaces
        .iter()
        .filter(|surface| surface.kind == "operation-skill")
        .collect::<Vec<_>>();
    assert_eq!(tasks.len(), 2);
    for task in tasks {
        closing_questions::assert_finished(task.content());
        assert!(task
            .content()
            .contains("[review method](../pale-doctrine-example-engine-work-review-"));
        assert_eq!(task.content().matches("## Read first").count(), 1);
        assert!(task.content().contains("SHARED REVIEW METHOD"));
        assert!(!task.content().contains("PARENT METHOD APPEARS ONLY HERE"));
        assert!(!task.content().contains("[task]"));
        assert_eq!(
            task.content().matches("https://example.com/method").count(),
            1
        );
    }
    let agents = compilation.repositories()[0]
        .render()
        .surfaces
        .iter()
        .filter(|surface| surface.kind == "specialist-agent")
        .collect::<Vec<_>>();
    assert_eq!(agents.len(), 2);
    for agent in agents {
        let text = if agent.content().starts_with("---") {
            agent.content().to_owned()
        } else {
            let record: toml::Value = toml::from_str(agent.content()).unwrap();
            record["developer_instructions"]
                .as_str()
                .unwrap()
                .to_owned()
        };
        closing_questions::assert_finished(&text);
        assert!(
            text.contains("[review method](../skills/pale-doctrine-example-engine-work-review-")
        );
        assert!(text.contains("configured permissions"));
    }
}

#[test]
fn stack_page_reading_preserves_generic_roles_without_repeating_the_shared_role_body() {
    for role_kind in [
        "shared",
        "header",
        "legacy",
        "startup_policy",
        "operation_procedure",
    ] {
        let fixture = page_fixture();
        let package = fixture.root.path().join("distribution/packages/engine");
        let page = package.join("work/review.md");
        let source = std::fs::read_to_string(&page).unwrap().replace(
            "[stack]\n",
            "[stack]\nrole_uses = [{ scope = 'local', resource_id = 'work/review' }]\n",
        );
        std::fs::write(page, source).unwrap();
        let manifest_path = package.join("package.toml");
        let mut manifest: toml::Value =
            toml::from_str(&std::fs::read_to_string(&manifest_path).unwrap()).unwrap();
        if role_kind != "shared" {
            let (body, source_role, declaration) = if role_kind == "header" {
                (
                    "+++\ndescription = 'Review any repository.'\n[role]\n+++\n\n# General review\n\nGENERIC ROLE BODY.\n",
                    "declaration",
                    "generic.md",
                )
            } else {
                std::fs::write(package.join("roles.toml"), "[roles]\nschema_version = 1\n[[roles.role]]\nid = 'generic'\ndescription = 'Review any repository.'\ndoctrine = 'generic.md'\n").unwrap();
                manifest["package"]["sources"]
                    .as_array_mut()
                    .unwrap()
                    .push(toml::from_str("path = 'roles.toml'\nrole = 'declaration'").unwrap());
                (
                    "# General review\n\nGENERIC ROLE BODY.\n",
                    "body",
                    "roles.toml",
                )
            };
            std::fs::write(package.join("generic.md"), body).unwrap();
            manifest["package"]["sources"].as_array_mut().unwrap().push(
                toml::from_str(&format!("path = 'generic.md'\nrole = '{source_role}'")).unwrap(),
            );
            manifest["package"]["resources"].as_array_mut().unwrap().push(
                toml::from_str(&format!("id = 'generic'\nkind = 'role'\ndefinition_source = '{declaration}'\nexposure = 'exported'")).unwrap(),
            );
        }
        if matches!(role_kind, "startup_policy" | "operation_procedure") {
            let page = package.join("work/review.md");
            let source = std::fs::read_to_string(&page).unwrap().replace(
                "role_uses = [{ scope = 'local', resource_id = 'work/review' }]",
                "role_uses = [{ scope = 'local', resource_id = 'legacy-stack-guide' }]",
            );
            std::fs::write(page, source).unwrap();
            std::fs::write(package.join("legacy-guides.toml"), format!("[doctrine]\nschema_version = 1\n[[doctrine.entry]]\nid = 'legacy-stack-guide'\ndescription = 'Legacy stack guidance'\ndoctrine = 'legacy-stack-guide.md'\nplacement = '{role_kind}'\nofficial_sources = [{{ label = 'Legacy reference', url = 'https://example.com/legacy-method' }}]\n")).unwrap();
            std::fs::write(
                package.join("legacy-stack-guide.md"),
                "# Legacy stack guidance\n\nNON-LINKABLE STACK GUIDE.\n",
            )
            .unwrap();
            for (path, role) in [
                ("legacy-guides.toml", "declaration"),
                ("legacy-stack-guide.md", "body"),
            ] {
                manifest["package"]["sources"]
                    .as_array_mut()
                    .unwrap()
                    .push(toml::from_str(&format!("path = '{path}'\nrole = '{role}'")).unwrap());
            }
            manifest["package"]["resources"].as_array_mut().unwrap().push(
                toml::from_str("id = 'legacy-stack-guide'\nkind = 'doctrine'\ndefinition_source = 'legacy-guides.toml'\nexposure = 'exported'").unwrap(),
            );
        }
        std::fs::write(manifest_path, toml::to_string(&manifest).unwrap()).unwrap();
        let mut workspace: toml::Value =
            toml::from_str(&std::fs::read_to_string(&fixture.manifest).unwrap()).unwrap();
        let repository = &mut workspace["workspace"]["desired_topology"]["project_repositories"][0];
        if role_kind != "shared" {
            repository["role"]["resource_id"] = "generic".into();
        }
        std::fs::write(&fixture.manifest, toml::to_string(&workspace).unwrap()).unwrap();
        let path = fixture.root.path().join("workspace/capabilities.toml");
        let mut capabilities: toml::Value =
            toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        capabilities["semantic"]["doctrine"] = toml::Value::Array(Vec::new());
        std::fs::write(path, toml::to_string(&capabilities).unwrap()).unwrap();
        fixture.refresh_distribution_lock();
        let profile = effective(&fixture);
        for host in [SourceGuidanceHost::Codex, SourceGuidanceHost::ClaudeCode] {
            let projection = profile
                .source_guidance(&LogicalRepositoryIdV1::parse("engine").unwrap(), host)
                .unwrap();
            assert!(
                check_links_resolve(&projection) > 0,
                "{role_kind}: no links checked"
            );
            let role = projection
                .files()
                .iter()
                .find(|file| file.rel_path().contains("/roles/"))
                .unwrap();
            assert_eq!(
                role.content().matches("SHARED REVIEW METHOD").count(),
                usize::from(role_kind == "shared"),
                "{role_kind}: {}",
                role.content()
            );
            if matches!(role_kind, "startup_policy" | "operation_procedure") {
                assert_eq!(
                    role.content().matches("NON-LINKABLE STACK GUIDE").count(),
                    1
                );
                assert!(!role.content().contains("## Read first"));
                assert!(role.content().contains("https://example.com/legacy-method"));
            } else {
                assert_eq!(role.content().matches("## Read first").count(), 1);
                assert!(role.content().contains("[Understand the work]"));
                assert_eq!(
                    role.content().contains("[Review the work]"),
                    role_kind != "shared"
                );
            }
            if role_kind != "shared" {
                assert_eq!(role.content().matches("GENERIC ROLE BODY").count(), 1);
            }
        }
    }
}
