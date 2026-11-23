//! Source-guidance journeys over actual Packages, without synthetic providers.

use pale_application::{compile::EffectiveProfileCompilation, render::SourceGuidanceHost};

use super::*;

#[cfg(unix)]
#[path = "guidance_plan.rs"]
mod guidance_plan;

#[path = "package_page_guidance.rs"]
mod package_pages;

fn remove_runtime_paths(fixture: &SyntheticWorkspace) {
    let mut manifest: toml::Value =
        toml::from_str(&std::fs::read_to_string(&fixture.manifest).unwrap()).unwrap();
    let topology = &mut manifest["workspace"]["desired_topology"];
    topology["execution_paths"] = toml::Value::Array(Vec::new());
    topology["execution_workspaces"][0]["execution_path_ids"] = toml::Value::Array(Vec::new());
    std::fs::write(&fixture.manifest, toml::to_string(&manifest).unwrap()).unwrap();
    let path = fixture.root.path().join("workspace/composition.toml");
    let mut composition: pale_core::records::WorkspaceCompositionSourceV1 =
        toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    composition.declarations = pale_core::records::ExperienceCompositionDeclarationsV1::empty();
    std::fs::write(path, toml::to_string(&composition).unwrap()).unwrap();
}

fn effective(fixture: &SyntheticWorkspace) -> EffectiveProfileCompilation {
    fn collect(root: &Path, directory: &Path, documents: &mut Vec<CapturedPackageDocument>) {
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                collect(root, &path, documents);
            } else {
                documents.push(
                    CapturedPackageDocument::new(
                        path.strip_prefix(root)
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .to_owned(),
                        std::fs::read(path).unwrap(),
                    )
                    .unwrap(),
                );
            }
        }
    }
    let root = fixture.root.path().join("distribution/packages");
    let mut documents = Vec::new();
    collect(&root, &root, &mut documents);
    let workspace = fixture.root.path().join("workspace");
    let private = CapturedPrivateWorkspaceInputs::new(
        private_document(
            "capabilities.toml",
            read_bytes(workspace.join("capabilities.toml")),
        ),
        private_document(
            "machines/machine.toml",
            read_bytes(&fixture.machine_binding),
        ),
        private_document(
            "distribution-lock.toml",
            read_bytes(workspace.join("distribution-lock.toml")),
        ),
        Vec::new(),
    )
    .unwrap();
    compile_captured_effective_profile_inputs(
        CapturedOneShotCompilationInputs::new(
            "demo".into(),
            "default".into(),
            "machine".into(),
            private,
            CapturedPackageSource::new(documents).unwrap(),
        ),
        &captured_workspace_universe(fixture),
    )
    .unwrap()
}

fn assert_links_resolve(projection: &pale_application::render::SourceGuidanceProjection) {
    assert!(
        check_links_resolve(projection) > 10,
        "exercise actual role, family and doctrine navigation"
    );
}

fn check_links_resolve(projection: &pale_application::render::SourceGuidanceProjection) -> usize {
    use comrak::{nodes::NodeValue, parse_document, Arena, Options};
    let files = projection
        .files()
        .iter()
        .map(|file| (file.rel_path(), file.content()))
        .chain(std::iter::once((
            projection.entrypoint_path(),
            projection.entrypoint_contribution(),
        )))
        .collect::<BTreeMap<_, _>>();
    let mut checked = 0;
    for (origin, content) in &files {
        if !origin.ends_with(".md") {
            continue;
        }
        let arena = Arena::new();
        let document = parse_document(&arena, content, &Options::default());
        for node in document.descendants() {
            let data = node.data();
            let NodeValue::Link(link) = &data.value else {
                continue;
            };
            if link.url.starts_with('#') || link.url.contains(':') {
                continue;
            }
            let path = Path::new(origin)
                .parent()
                .unwrap()
                .join(link.url.split('#').next().unwrap());
            let mut parts = Vec::new();
            for component in path.components() {
                match component {
                    std::path::Component::Normal(part) => parts.push(part.to_str().unwrap()),
                    std::path::Component::ParentDir => {
                        assert!(parts.pop().is_some());
                    }
                    std::path::Component::CurDir => {}
                    _ => panic!("unconfined guidance reference"),
                }
            }
            let target = parts.join("/");
            assert!(
                files.contains_key(target.as_str()),
                "{origin}: missing {target}"
            );
            checked += 1;
        }
    }
    checked
}

fn langfuse_fixture() -> SyntheticWorkspace {
    let fixture = fixture(&[("pale.evaluation", "compare-runs")]);
    remove_runtime_paths(&fixture);
    let path = fixture.root.path().join("workspace/capabilities.toml");
    let mut capabilities: toml::Value =
        toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    capabilities["semantic"]["operations"] = toml::Value::Array(Vec::new());
    capabilities["operation_bindings"] = toml::Value::Array(Vec::new());
    let doctrine = toml::from_str::<toml::Value>(
        "items = [{ selection = 'exact', resource = { package_id = 'pale.evaluation', resource_id = 'langfuse-tools' } }]"
    ).unwrap()["items"].clone();
    capabilities["semantic"]
        .as_table_mut()
        .unwrap()
        .insert("doctrine".into(), doctrine);
    std::fs::write(&path, toml::to_string(&capabilities).unwrap()).unwrap();
    fixture.refresh_distribution_lock();
    fixture
}

#[test]
fn current_packages_project_without_providers_and_retain_optional_references() {
    let fixture = langfuse_fixture();
    let compilation = effective(&fixture);
    assert!(compilation.profile().execution_paths.is_empty());
    assert!(compilation
        .experience_composition_plan()
        .declarations
        .provider_bindings
        .is_empty());
    assert!(compilation.instance().native_harnesses.is_empty());
    // Runtime rendering on these exact source declarations remains closed.
    let runtime_error = fixture
        .compile()
        .expect_err("runtime rendering requires an execution path");
    assert_eq!(
        runtime_error.failure_disposition(),
        ApplicationFailureDisposition::InvalidInput
    );
    assert!(
        matches!(
            &runtime_error,
            CompileError::MissingExecutionPaths { execution_workspace }
                if execution_workspace.as_str() == "default"
        ),
        "expected missing execution paths for the default workspace, got {runtime_error:?}"
    );
    let repository = LogicalRepositoryIdV1::parse("engine").unwrap();
    let mut identities = Vec::new();
    for (host, directory, entry) in [
        (SourceGuidanceHost::Codex, ".codex", "AGENTS.md"),
        (SourceGuidanceHost::ClaudeCode, ".claude", "AGENTS.md"),
    ] {
        let projection = compilation.source_guidance(&repository, host).unwrap();
        assert_eq!(projection.entrypoint_path(), entry);
        assert!(!projection
            .files()
            .iter()
            .any(|file| file.rel_path() == entry));
        let role = projection
            .files()
            .iter()
            .find(|file| {
                file.rel_path()
                    .ends_with("pale-role-pale-engineering-change.md")
            })
            .unwrap();
        assert_eq!(
            normalize(role.content())
                .matches(
                    body("engineering/change.md")
                        .split("| If you are doing this |")
                        .next()
                        .unwrap()
                        .trim_end()
                )
                .count(),
            1
        );
        for source in ["error-handling.md", "response-time.md", "security.md"] {
            assert!(role
                .content()
                .contains(&format!("`pale.engineering`, `{source}`")));
        }
        let tooling_path =
            format!("{directory}/skills/pale-doctrine-pale-evaluation-langfuse-tools/SKILL.md");
        let tooling = projection
            .files()
            .iter()
            .find(|file| file.rel_path() == tooling_path)
            .unwrap();
        assert!(tooling
            .content()
            .contains("@langfuse/cli@1.3.1 api schema --json"));
        assert!(!normalize(tooling.content()).contains(&body("evaluation/langfuse.md")));
        assert_links_resolve(&projection);
        for file in projection.files() {
            assert_eq!(
                file.sha256(),
                &ArtifactDigest::hash_bytes(file.content().as_bytes())
            );
            assert!(!file
                .content()
                .contains(fixture.root.path().to_str().unwrap()));
        }
        assert_eq!(
            &projection,
            &compilation.source_guidance(&repository, host).unwrap()
        );
        identities.push(projection.digest().clone());
    }
    assert_ne!(identities[0], identities[1]);
    let before = compilation
        .source_guidance(&repository, SourceGuidanceHost::Codex)
        .unwrap();
    std::fs::remove_dir_all(fixture.root.path().join("distribution")).unwrap();
    std::fs::remove_dir_all(fixture.root.path().join("workspace")).unwrap();
    assert_eq!(
        before,
        compilation
            .source_guidance(&repository, SourceGuidanceHost::Codex)
            .unwrap()
    );
    assert!(compilation
        .source_guidance(
            &LogicalRepositoryIdV1::parse("unselected").unwrap(),
            SourceGuidanceHost::Codex
        )
        .is_err());
}

#[test]
fn change_role_installs_shared_guide_without_selecting_a_test_operation() {
    let fixture = fixture(&[]);
    remove_runtime_paths(&fixture);
    let compilation = effective(&fixture);
    let repository = LogicalRepositoryIdV1::parse("engine").unwrap();
    for (host, directory) in [
        (SourceGuidanceHost::Codex, ".codex"),
        (SourceGuidanceHost::ClaudeCode, ".claude"),
    ] {
        let projection = compilation.source_guidance(&repository, host).unwrap();
        let common_path =
            format!("{directory}/skills/pale-doctrine-pale-engineering-change/SKILL.md");
        let common = projection
            .files()
            .iter()
            .find(|file| file.rel_path() == common_path)
            .expect("the coding role requires the shared method");
        assert!(common.content().contains("Give each file one job"));
        assert!(common
            .content()
            .contains("Comments say what the code cannot"));
        assert!(!projection.files().iter().any(|file| {
            file.rel_path()
                .contains("pale-doctrine-pale-engineering-security/")
        }));
        assert_links_resolve(&projection);
    }
}

#[test]
fn declaration_only_related_reading_survives_source_guidance_without_source_files() {
    for selected_related in [false, true] {
        let fixture = fixture(&[("pale.design", "build-interface")]);
        remove_runtime_paths(&fixture);
        replace_page_body(
            &fixture
                .root
                .path()
                .join("distribution/packages/design/inspect.md"),
            "# Rendered interface checks\n\nInspect the example in the browser.\n",
        );
        select_resource(&fixture, "pale.design", "doctrine", "inspect");
        if selected_related {
            add_doctrine(&fixture, "pale.design", "animate");
        }
        fixture.refresh_distribution_lock();
        let compilation = effective(&fixture);
        std::fs::remove_dir_all(fixture.root.path().join("distribution")).unwrap();
        let repository = LogicalRepositoryIdV1::parse("engine").unwrap();
        for (host, directory) in [
            (SourceGuidanceHost::Codex, ".codex"),
            (SourceGuidanceHost::ClaudeCode, ".claude"),
        ] {
            let projection = compilation.source_guidance(&repository, host).unwrap();
            let path = format!("{directory}/skills/pale-doctrine-pale-design-inspect/SKILL.md");
            let text = projection
                .files()
                .iter()
                .find(|file| file.rel_path() == path)
                .unwrap()
                .content();
            let related = text.split("## Related reading\n\n").nth(1).unwrap();
            assert!(related.contains("`pale.design`, `animate.md`"));
            assert!(related.contains("Design motion that helps people use the interface"));
            assert_eq!(
                related.contains("../pale-doctrine-pale-design-animate/SKILL.md"),
                selected_related
            );
            let motion_path =
                format!("{directory}/skills/pale-doctrine-pale-design-animate/SKILL.md");
            assert_eq!(
                projection
                    .files()
                    .iter()
                    .any(|file| file.rel_path() == motion_path),
                selected_related
            );
            assert!(!text.contains(&body("design/animate.md")));
            assert_links_resolve(&projection);
        }
    }
}

#[test]
fn selected_operations_are_not_silently_downgraded_to_guidance() {
    let fixture = fixture(&[("pale.evaluation", "compare-runs")]);
    remove_runtime_paths(&fixture);
    let compilation = effective(&fixture);
    assert!(!compilation.repository_semantic_plans()[0]
        .operations
        .is_empty());
    assert!(compilation
        .source_guidance(
            &LogicalRepositoryIdV1::parse("engine").unwrap(),
            SourceGuidanceHost::Codex
        )
        .is_err());
}

#[test]
fn review_operation_and_role_share_one_method_on_both_hosts() {
    let fixture = fixture(&[("pale.engineering", "review")]);
    let compilation = fixture.compile().unwrap();
    for host in ["codex", "claude"] {
        let text = normalize(operation(&compilation, host, "pale-engineering-review"));
        assert_eq!(text.matches(&body("engineering/review.md")).count(), 1);
        assert!(!text.contains(&body("engineering/security.md")));
    }
    assert_local_links_resolve(&compilation);
}

#[test]
fn inherited_role_doctrine_is_composed_once_and_read_ceiling_remains_visible() {
    let fixture = fixture(&[]);
    remove_runtime_paths(&fixture);
    let mut manifest: toml::Value =
        toml::from_str(&std::fs::read_to_string(&fixture.manifest).unwrap()).unwrap();
    manifest["workspace"]["desired_topology"]["project_repositories"][0]["role"]["resource_id"] =
        "security".into();
    manifest["workspace"]["desired_topology"]["execution_workspace_repositories"][0]["access"] =
        "read".into();
    std::fs::write(&fixture.manifest, toml::to_string(&manifest).unwrap()).unwrap();
    let compilation = effective(&fixture);
    for host in [SourceGuidanceHost::ClaudeCode, SourceGuidanceHost::Codex] {
        let projection = compilation
            .source_guidance(&LogicalRepositoryIdV1::parse("engine").unwrap(), host)
            .unwrap();
        let role = projection
            .files()
            .iter()
            .find(|file| {
                file.rel_path()
                    .ends_with("pale-role-pale-engineering-security.md")
            })
            .unwrap();
        for relative in ["engineering/review.md", "engineering/security.md"] {
            assert_eq!(
                normalize(role.content()).matches(&body(relative)).count(),
                1,
                "{relative}"
            );
        }
        let index = projection
            .files()
            .iter()
            .find(|file| file.rel_path().ends_with("index.md"))
            .unwrap();
        assert!(index
            .content()
            .contains("Configured access limit: read only"));
        assert_links_resolve(&projection);
    }
}

#[test]
fn selected_lifecycle_delivery_is_not_recast_as_an_instruction_link() {
    let fixture = fixture(&[]);
    remove_runtime_paths(&fixture);
    let path = fixture.root.path().join("workspace/capabilities.toml");
    let capabilities = std::fs::read_to_string(&path).unwrap();
    assert!(capabilities.contains("session_inject = false"));
    std::fs::write(
        path,
        capabilities.replace("session_inject = false", "session_inject = true"),
    )
    .unwrap();
    let compilation = effective(&fixture);
    assert!(compilation.profile().run_policy.session_inject);
    assert!(compilation
        .source_guidance(
            &LogicalRepositoryIdV1::parse("engine").unwrap(),
            SourceGuidanceHost::ClaudeCode
        )
        .is_err());
}

#[test]
fn explicitly_selected_startup_judgment_keeps_its_body_and_optional_navigation() {
    let fixture = langfuse_fixture();
    // The current Packages have no startup-policy resource. Add an explicitly
    // authored disposable one, leaving real Langfuse placement and links intact.
    let package = fixture.root.path().join("distribution/packages/evaluation");
    let path = package.join("package.toml");
    let mut manifest: toml::Value =
        toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    manifest["package"]["sources"]
        .as_array_mut()
        .unwrap()
        .push(toml::from_str("path = 'qualification-startup.md'\nrole = 'body'").unwrap());
    manifest["package"]["sources"].as_array_mut().unwrap().push(
        toml::from_str("path = 'qualification-declarations.toml'\nrole = 'declaration'").unwrap(),
    );
    manifest["package"]["resources"].as_array_mut().unwrap().push(
        toml::from_str("id = 'qualification-startup'\nkind = 'doctrine'\ndefinition_source = 'qualification-declarations.toml'\nexposure = 'exported'").unwrap());
    std::fs::write(path, toml::to_string(&manifest).unwrap()).unwrap();
    let path = package.join("qualification-declarations.toml");
    let mut declarations = "[doctrine]\nschema_version = 1\n".to_owned();
    declarations.push_str("\n[[doctrine.entry]]\nid = 'qualification-startup'\ndescription = 'Fixture required judgment'\ndoctrine = 'qualification-startup.md'\nplacement = 'startup_policy'\nsee_also = [{ scope = 'local', resource_id = 'langfuse' }]\n");
    std::fs::write(path, declarations).unwrap();
    std::fs::write(package.join("qualification-startup.md"),
        "# Required project judgment\n\nUse the selected project context.\n\n[Trace lineage](langfuse.md)\n").unwrap();
    let path = fixture.root.path().join("workspace/capabilities.toml");
    let mut capabilities: toml::Value =
        toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    capabilities["semantic"]["doctrine"].as_array_mut().unwrap().push(
        toml::from_str("selection = 'exact'\nresource = { package_id = 'pale.evaluation', resource_id = 'qualification-startup' }").unwrap());
    std::fs::write(path, toml::to_string(&capabilities).unwrap()).unwrap();
    fixture.refresh_distribution_lock();
    let compilation = effective(&fixture);
    assert_eq!(
        compilation.repository_semantic_plans()[0]
            .startup_policies
            .len(),
        1
    );
    for host in [SourceGuidanceHost::Codex, SourceGuidanceHost::ClaudeCode] {
        let projection = compilation
            .source_guidance(&LogicalRepositoryIdV1::parse("engine").unwrap(), host)
            .unwrap();
        let index = projection
            .files()
            .iter()
            .find(|file| file.rel_path().ends_with("index.md"))
            .unwrap();
        assert_eq!(
            index
                .content()
                .matches("Use the selected project context.")
                .count(),
            1
        );
        assert!(!index
            .content()
            .contains("@langfuse/cli@1.3.1 api schema --json"));
        assert!(projection
            .files()
            .iter()
            .any(|file| file.rel_path().contains("langfuse-tools/SKILL.md")));
        assert!(!normalize(index.content()).contains(&body("evaluation/langfuse.md")));
        assert_links_resolve(&projection);
    }
}

#[test]
fn color_and_animejs_stacks_install_only_their_required_guidance_on_both_hosts() {
    let fixture = fixture(&[("pale.design", "build-interface")]);
    remove_runtime_paths(&fixture);
    select_resource(&fixture, "pale.design", "stacks", "colour/css");
    let path = fixture.root.path().join("workspace/capabilities.toml");
    let mut value: toml::Value = toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    value["semantic"]["stacks"].as_array_mut().unwrap().push(toml::from_str(
        "selection = 'exact'\nresource = { package_id = 'pale.design', resource_id = 'animate/animejs/guide' }"
    ).unwrap());
    std::fs::write(path, toml::to_string(&value).unwrap()).unwrap();
    let compilation = effective(&fixture);
    for host in [SourceGuidanceHost::Codex, SourceGuidanceHost::ClaudeCode] {
        let projection = compilation
            .source_guidance(&LogicalRepositoryIdV1::parse("engine").unwrap(), host)
            .unwrap();
        let design = projection
            .files()
            .iter()
            .filter(|file| file.rel_path().contains("-pale-design-"))
            .collect::<Vec<_>>();
        for required in ["colour/css", "animate/animejs/guide"] {
            assert!(design.iter().any(|file| file.rel_path().contains(&format!(
                "/pale-stack-pale-design-{}-",
                required.replace('/', "-")
            ))));
        }
        for unchosen in [
            "films",
            "image-workflow",
            "animate/motion/guide",
            "animate/scroll/guide",
        ] {
            assert!(!design.iter().any(|file| file
                .rel_path()
                .contains(&format!("-pale-design-{}", unchosen.replace('/', "-")))));
        }
        assert!(design
            .iter()
            .any(|file| file.content().contains("`pale.design`, `")));
        eprintln!(
            "{host:?}: {} Design skills for colour/css + animate/animejs",
            design.len()
        );
        assert_links_resolve(&projection);
    }
}
