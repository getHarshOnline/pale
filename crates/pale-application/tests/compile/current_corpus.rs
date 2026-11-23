//! Current authored Package journeys, independent of migration snapshots or binaries.
//!
//! These are compiler/render observations. They invoke no native provider and
//! establish neither live host admission nor agent-task or publication quality.

use super::*;

#[path = "source_guidance.rs"]
mod source_guidance;

#[path = "authored_paths.rs"]
mod authored_paths;

fn replace_page_body(path: &Path, body: &str) {
    let source = std::fs::read_to_string(path).unwrap();
    let header = source.split("+++").nth(1).unwrap();
    std::fs::write(path, format!("+++{header}+++\n\n{body}")).unwrap();
}

fn source_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn copy_directory(source: &Path, destination: &Path) {
    std::fs::create_dir_all(destination).unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        assert!(!entry.file_type().unwrap().is_symlink());
        let target = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_directory(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn fixture(operations: &[(&str, &str)]) -> SyntheticWorkspace {
    let fixture = SyntheticWorkspace::new(false);
    let distribution = fixture.root.path().join("distribution/packages");
    std::fs::remove_dir_all(distribution.join("engine")).unwrap();
    let mut seeds = vec![toml::Value::String("pale.engineering".into())];
    for entry in std::fs::read_dir(source_root().join("packages")).unwrap() {
        let entry = entry.unwrap();
        let manifest = entry.path().join("package.toml");
        if manifest.is_file() {
            copy_directory(&entry.path(), &distribution.join(entry.file_name()));
        }
    }
    seeds.extend(
        operations
            .iter()
            .map(|(package, _)| toml::Value::String((*package).into())),
    );
    seeds.sort_by_key(|value| value.as_str().unwrap().to_owned());
    seeds.dedup();
    let mut manifest: toml::Value =
        toml::from_str(&std::fs::read_to_string(&fixture.manifest).unwrap()).unwrap();
    let topology = &mut manifest["workspace"]["desired_topology"];
    topology["projects"][0]["package_seeds"] = toml::Value::Array(seeds);
    topology["execution_paths"] = toml::from_str::<toml::Value>(
        "paths = [{ id = 'direct-claude', experience_path_id = 'direct-claude-code' }, { id = 'direct-codex', experience_path_id = 'direct-codex' }]")
        .unwrap()["paths"].clone();
    topology["execution_workspaces"][0]["execution_path_ids"] =
        toml::Value::Array(vec!["direct-claude".into(), "direct-codex".into()]);
    topology["project_repositories"][0]["role"]["package_id"] = "pale.engineering".into();
    topology["project_repositories"][0]["role"]["resource_id"] = "change".into();
    topology["project_repositories"][0]["families"] = toml::Value::Array(
        [
            ("pale.engineering", "change/choice"),
            ("pale.engineering", "change/constraints"),
            ("pale.engineering", "change/impact"),
            ("pale.engineering", "change/limits"),
            ("pale.engineering", "review"),
            ("pale.engineering", "testing"),
            ("pale.governance", "authority"),
            ("pale.governance", "execution"),
        ]
        .iter()
        .map(|(package, resource)| {
            toml::from_str::<toml::Value>(&format!(
                "package_id = {package:?}\nresource_id = {resource:?}\n"
            ))
            .unwrap()
        })
        .collect(),
    );
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
    let refs = operations
        .iter()
        .map(|(package, resource)| {
            toml::from_str::<toml::Value>(&format!(
                "package_id = {package:?}\nresource_id = {resource:?}\n"
            ))
            .unwrap()
        })
        .collect::<Vec<_>>();
    capabilities.as_table_mut().unwrap().insert(
        "semantic".into(),
        toml::from_str::<toml::Value>("operations = []").unwrap(),
    );
    capabilities["semantic"]["operations"] = toml::Value::Array(
        refs.iter()
            .map(|reference| {
                let mut selection = toml::map::Map::new();
                selection.insert("selection".into(), "exact".into());
                selection.insert("resource".into(), reference.clone());
                toml::Value::Table(selection)
            })
            .collect(),
    );
    let bindings = toml::Value::Array(
        refs.into_iter()
            .map(|reference| {
                let mut row = toml::from_str::<toml::Value>(
                    "[binding.selection]\nkind = 'model_or_user'\naliases = []\n",
                )
                .unwrap();
                row.as_table_mut()
                    .unwrap()
                    .insert("operation".into(), reference);
                row
            })
            .collect(),
    );
    capabilities
        .as_table_mut()
        .unwrap()
        .insert("operation_bindings".into(), bindings);
    std::fs::write(path, toml::to_string(&capabilities).unwrap()).unwrap();
    fixture.refresh_distribution_lock();
    fixture
}

fn body(relative: &str) -> String {
    let text = std::fs::read_to_string(source_root().join("packages").join(relative)).unwrap();
    let text = if text.starts_with("+++\n") {
        text.split_once("\n+++\n")
            .unwrap()
            .1
            .split("\n## Before handing off")
            .next()
            .unwrap()
            .trim_start()
    } else {
        &text
    };
    let (_, prose) = text.split_once('\n').unwrap();
    normalize(prose.trim())
}

fn common_workflow_rules() -> String {
    let text =
        std::fs::read_to_string(source_root().join("packages/automation/workflow.md")).unwrap();
    normalize(
        text.split_once("\n+++\n")
            .unwrap()
            .1
            .trim_start()
            .split_once('\n')
            .unwrap()
            .1
            .split("\n## ")
            .next()
            .unwrap()
            .trim(),
    )
}

fn normalize(text: &str) -> String {
    text.lines()
        .map(|line| line.trim_start_matches('#').trim_start())
        .collect::<Vec<_>>()
        .join("\n")
}

fn operation<'a>(compilation: &'a Compilation, host: &str, slug: &str) -> &'a str {
    let path = format!(".{host}/skills/pale-op-{slug}/SKILL.md");
    compilation.repositories()[0]
        .render()
        .surfaces
        .iter()
        .find(|surface| surface.rel_path == path)
        .unwrap_or_else(|| panic!("missing operation {path}"))
        .content()
}

fn assert_local_links_resolve(compilation: &Compilation) -> usize {
    use comrak::{nodes::NodeValue, parse_document, Arena, Options};
    let surfaces = &compilation.repositories()[0].render().surfaces;
    let by_path = surfaces
        .iter()
        .map(|surface| (surface.rel_path.as_str(), surface))
        .collect::<BTreeMap<_, _>>();
    let mut checked = 0;
    for surface in surfaces.iter().filter(|surface| {
        matches!(
            surface.kind.as_str(),
            "operation-skill"
                | "doctrine-skill"
                | "specialist-agent"
                | "policy-skill"
                | "stack-skill"
                | "injection"
        )
    }) {
        let instructions;
        let content = if surface.rel_path.ends_with(".toml") {
            let value: toml::Value = toml::from_str(surface.content()).unwrap();
            instructions = value["developer_instructions"].as_str().unwrap().to_owned();
            instructions.as_str()
        } else {
            surface.content()
        };
        let arena = Arena::new();
        let document = parse_document(&arena, content, &Options::default());
        for node in document.descendants() {
            let data = node.data();
            let NodeValue::Link(link) = &data.value else {
                continue;
            };
            let href = link.url.as_str();
            if href.contains(':') {
                continue;
            }
            let (relative, fragment) = href.split_once('#').unwrap_or((href, ""));
            let path = if relative.is_empty() {
                PathBuf::from(&surface.rel_path)
            } else {
                Path::new(&surface.rel_path)
                    .parent()
                    .unwrap()
                    .join(relative)
            };
            let mut parts = Vec::new();
            for component in path.components() {
                match component {
                    std::path::Component::Normal(value) => parts.push(value.to_str().unwrap()),
                    std::path::Component::ParentDir => {
                        assert!(parts.pop().is_some());
                    }
                    std::path::Component::CurDir => {}
                    _ => panic!("unconfined generated link {href}"),
                }
            }
            let target = parts.join("/");
            assert!(
                by_path.contains_key(target.as_str()),
                "{} links to missing projected target {href} ({target})",
                surface.rel_path
            );
            if !fragment.is_empty() {
                let target_instructions;
                let target_source = if target.ends_with(".toml") {
                    let record: toml::Value =
                        toml::from_str(by_path[target.as_str()].content()).unwrap();
                    target_instructions = record["developer_instructions"]
                        .as_str()
                        .unwrap()
                        .to_owned();
                    target_instructions.as_str()
                } else {
                    by_path[target.as_str()].content()
                };
                let target_arena = Arena::new();
                let target_document =
                    parse_document(&target_arena, target_source, &Options::default());
                let mut anchors = std::collections::BTreeSet::new();
                for heading in target_document.descendants() {
                    let heading_data = heading.data();
                    let html = match &heading_data.value {
                        NodeValue::HtmlInline(html) => Some(html.as_str()),
                        NodeValue::HtmlBlock(html) if html.block_type == 7 => {
                            Some(html.literal.as_str())
                        }
                        _ => None,
                    };
                    if let Some(html) = html {
                        for line in html.lines() {
                            if let Some(anchor) = line.trim().strip_prefix("<a id=\"") {
                                if let Some((id, tail)) = anchor.split_once('"') {
                                    if matches!(tail, ">" | "></a>") {
                                        anchors.insert(id.to_owned());
                                    }
                                }
                            }
                        }
                    }
                    if !matches!(heading_data.value, NodeValue::Heading(_)) {
                        continue;
                    }
                    let mut text = String::new();
                    for child in heading.descendants() {
                        match &child.data().value {
                            NodeValue::Text(value) => text.push_str(value),
                            NodeValue::Code(value) => text.push_str(&value.literal),
                            _ => {}
                        }
                    }
                    let slug = text
                        .to_lowercase()
                        .chars()
                        .filter(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | ' '))
                        .map(|c| if c == ' ' { '-' } else { c })
                        .collect::<String>();
                    anchors.insert(slug);
                }
                assert!(
                    anchors.contains(fragment),
                    "missing projected fragment {href} from {}",
                    surface.rel_path
                );
            }
            checked += 1;
        }
    }
    checked
}

#[test]
fn representation_review_composes_numerical_and_protected_comparison_once() {
    let fixture = fixture(&[("pale.model-work", "representations")]);
    let compilation = fixture.compile().unwrap();
    compilation.package_semantic_ir().verify().unwrap();
    for host in ["codex", "claude"] {
        let text = compilation.repositories()[0]
            .render()
            .surfaces
            .iter()
            .filter(|surface| surface.rel_path.starts_with(&format!(".{host}/")))
            .map(|surface| normalize(surface.content()))
            .collect::<Vec<_>>()
            .join("\n");
        for source in ["evaluation/numbers.md", "evaluation/compare.md"] {
            assert_eq!(text.matches(&body(source)).count(), 1, "{host}: {source}");
        }
        for heading in [
            "Establish which observations are paired",
            "Check what similarity preserves",
            "Compare reconstruction at a stated sparsity",
            "Check features beyond reconstruction",
        ] {
            assert_eq!(text.matches(heading).count(), 1, "{host}: {heading}");
        }
        assert!(!text.contains("Fix the workload and system being compared"));
    }
}

#[test]
fn interface_implementation_composes_required_evidence_without_optional_craft() {
    let fixture = fixture(&[("pale.design", "build-interface")]);
    let compilation = fixture.compile().unwrap();
    for host in ["codex", "claude"] {
        let text = normalize(operation(&compilation, host, "pale-design-build-interface"));
        let surfaces = &compilation.repositories()[0].render().surfaces;
        for (package, id) in [
            ("design", "system"),
            ("design", "art-direction"),
            ("design", "inspect"),
            ("design", "access"),
            ("design", "access/web"),
            ("engineering", "execution"),
        ] {
            let trace = format!("doctrine:pale.{package}:{id}");
            let guide = surfaces
                .iter()
                .find(|surface| {
                    surface.rel_path.starts_with(&format!(".{host}/"))
                        && surface.kind == "doctrine-skill"
                        && surface.referenced_policy_ids.contains(&trace)
                })
                .expect("required guide is available");
            let authored = body(&format!("{package}/{id}.md"));
            let opening = authored.split("\n\n").next().unwrap();
            assert!(
                normalize(guide.content()).contains(opening),
                "{host}: {package}/{id}"
            );
            assert!(
                !text.contains(opening),
                "required guidance stays in its own page"
            );
        }
        for id in ["system", "art-direction", "inspect"] {
            assert!(text.contains(&format!("../pale-doctrine-pale-design-{id}/SKILL.md")));
        }
        assert!(!text.contains(&body("design/animate.md")));
        assert!(!text.contains(&body("design/independent.md")));
        assert!(!text.contains(&body("design/responsive.md")));
        assert!(!text.contains(&body("design/colour.md")));
        assert!(!text.contains(&body("design/type.md")));
        assert!(!text.contains(&body("design/controls.md")));
        assert!(!text.contains(&body("design/layout.md")));
        assert!(!text.contains(&body("design/surfaces.md")));
        assert!(!text.contains(&body("design/figures.md")));
        assert!(!text.contains(&body("design/variations.md")));
        assert!(!text.contains(&body("design/presentation.md")));
    }
    assert_local_links_resolve(&compilation);
}

#[test]
fn visual_craft_selection_composes_accessibility_once_without_an_implementation_operation() {
    for (resource, browser_prerequisite) in [
        ("responsive", true),
        ("colour", true),
        ("type", true),
        ("controls", true),
        ("layout", false),
        ("surfaces", false),
        ("figures", true),
    ] {
        let fixture = fixture(&[("pale.design", "build-interface")]);
        let path = fixture.root.path().join("workspace/capabilities.toml");
        let mut capabilities: toml::Value =
            toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        capabilities["semantic"]["operations"] = toml::Value::Array(Vec::new());
        capabilities["operation_bindings"] = toml::Value::Array(Vec::new());
        capabilities["semantic"].as_table_mut().unwrap().insert(
            "doctrine".into(),
            toml::from_str::<toml::Value>(&format!(
                "items = [{{ selection = 'exact', resource = {{ package_id = 'pale.design', resource_id = '{resource}' }} }}]",
            )).unwrap()["items"].clone(),
        );
        std::fs::write(path, toml::to_string(&capabilities).unwrap()).unwrap();
        fixture.refresh_distribution_lock();
        let compilation = fixture.compile().unwrap();
        for host in ["codex", "claude"] {
            let path = format!(".{host}/skills/pale-doctrine-pale-design-{resource}/SKILL.md");
            let text = normalize(
                compilation.repositories()[0]
                    .render()
                    .surfaces
                    .iter()
                    .find(|surface| surface.rel_path == path)
                    .expect("selected visual craft guidance")
                    .content(),
            );
            let entry = body(&format!("design/{resource}.md"));
            // The host renderer rewrites links in the task table to host skill paths.
            let opening = entry.split("\n\n").next().unwrap();
            assert_eq!(text.matches(opening).count(), 1, "{host}: {resource}");
            let surfaces = &compilation.repositories()[0].render().surfaces;
            for (id, required) in [("access", true), ("access/web", browser_prerequisite)] {
                let trace = format!("doctrine:pale.design:{id}");
                let guides = surfaces
                    .iter()
                    .filter(|surface| {
                        surface.rel_path.starts_with(&format!(".{host}/"))
                            && surface.kind == "doctrine-skill"
                            && surface.referenced_policy_ids.contains(&trace)
                    })
                    .collect::<Vec<_>>();
                assert_eq!(
                    guides.len(),
                    usize::from(required),
                    "{host}: {resource} needs {id}"
                );
                if required {
                    let authored = body(&format!("design/{id}.md"));
                    let opening = authored.split("\n\n").next().unwrap();
                    assert!(normalize(guides[0].content()).contains(opening));
                    assert!(!text.contains(opening));
                }
            }
            assert!(!compilation.repositories()[0]
                .render()
                .surfaces
                .iter()
                .any(|surface| surface.rel_path
                    == format!(".{host}/skills/pale-op-pale-design-build-interface/SKILL.md")));
        }
        assert_local_links_resolve(&compilation);
    }
}

#[test]
fn selected_craft_judgment_does_not_start_an_interface_or_review_workflow() {
    for (resource, sources) in [
        ("generate", vec!["design/assets.md", "design/generate.md"]),
        ("films", vec!["design/films.md"]),
        ("editing", vec!["design/editing.md"]),
        ("web", vec!["design/web.md"]),
    ] {
        let fixture = fixture(&[("pale.design", "build-interface")]);
        let path = fixture.root.path().join("workspace/capabilities.toml");
        let mut capabilities: toml::Value =
            toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        capabilities["semantic"]["operations"] = toml::Value::Array(Vec::new());
        capabilities["operation_bindings"] = toml::Value::Array(Vec::new());
        capabilities["semantic"].as_table_mut().unwrap().insert(
        "doctrine".into(),
        toml::from_str::<toml::Value>(
            &format!("items = [{{ selection = 'exact', resource = {{ package_id = 'pale.design', resource_id = '{resource}' }} }}]"),
        ).unwrap()["items"].clone(),
    );
        std::fs::write(path, toml::to_string(&capabilities).unwrap()).unwrap();
        fixture.refresh_distribution_lock();
        let compilation = fixture.compile().unwrap();
        for host in ["codex", "claude"] {
            let rendered = compilation.repositories()[0].render();
            let path = format!(".{host}/skills/pale-doctrine-pale-design-{resource}/SKILL.md");
            let text = normalize(
                rendered
                    .surfaces
                    .iter()
                    .find(|s| s.rel_path == path)
                    .unwrap()
                    .content(),
            );
            for source in &sources {
                let id = source
                    .strip_prefix("design/")
                    .unwrap()
                    .strip_suffix(".md")
                    .unwrap()
                    .replace('/', "-");
                let path = format!(".{host}/skills/pale-doctrine-pale-design-{id}/SKILL.md");
                let guidance = normalize(
                    rendered
                        .surfaces
                        .iter()
                        .find(|surface| surface.rel_path == path)
                        .expect("selected method and its required reading")
                        .content(),
                );
                for paragraph in body(source)
                    .split("\n\n")
                    .filter(|p| !p.contains("](") && *p != "Before handing off")
                {
                    assert_eq!(
                        guidance.matches(paragraph).count(),
                        1,
                        "{host}: {source}: {paragraph}"
                    );
                }
            }
            for source in [
                "design/access.md",
                "design/image-workflow.md",
                "design/independent.md",
            ] {
                assert!(
                    !text.contains(&body(source)),
                    "unselected procedure: {source}"
                );
            }
            assert!(!rendered
                .surfaces
                .iter()
                .any(|s| s.rel_path.starts_with(&format!(".{host}/skills/pale-op-"))));
        }
        assert_local_links_resolve(&compilation);
    }
}

#[test]
fn workflow_keeps_optional_topics_on_demand_and_links_reachable() {
    let fixture = fixture(&[("pale.automation", "check")]);
    let compilation = fixture.compile().unwrap();
    compilation.package_semantic_ir().verify().unwrap();
    for host in ["codex", "claude"] {
        let text = normalize(operation(&compilation, host, "pale-automation-check"));
        assert!(!text.contains(&common_workflow_rules()));
        assert!(!text.contains(&body("automation/callbacks.md")));
        let surfaces = &compilation.repositories()[0].render().surfaces;
        assert_eq!(
            surfaces
                .iter()
                .filter(|surface| {
                    surface.rel_path.starts_with(&format!(".{host}/"))
                        && normalize(surface.content()).contains(&common_workflow_rules())
                })
                .count(),
            1
        );
    }
    assert_local_links_resolve(&compilation);
}

#[test]
fn internal_operation_and_startup_links_use_the_actual_provider_file_origin() {
    let fixture = fixture(&[("pale.automation", "check")]);
    let package = fixture.root.path().join("distribution/packages/automation");
    let manifest_path = package.join("package.toml");
    let mut manifest: toml::Value =
        toml::from_str(&std::fs::read_to_string(&manifest_path).unwrap()).unwrap();
    manifest["package"]["sources"]
        .as_array_mut()
        .unwrap()
        .extend([
            toml::from_str("path = 'startup-links.md'\nrole = 'body'").unwrap(),
            toml::from_str("path = 'startup-fixture.toml'\nrole = 'declaration'").unwrap(),
        ]);
    manifest["package"]["resources"].as_array_mut().unwrap().push(
        toml::from_str("id = 'startup-links'\nkind = 'doctrine'\ndefinition_source = 'startup-fixture.toml'\nexposure = 'exported'").unwrap());
    std::fs::write(manifest_path, toml::to_string(&manifest).unwrap()).unwrap();
    let declaration = package.join("startup-fixture.toml");
    std::fs::write(declaration, "[doctrine]\nschema_version = 1\n[[doctrine.entry]]\nid = 'startup-links'\ndescription = 'Fixture startup navigation'\ndoctrine = 'startup-links.md'\nplacement = 'startup_policy'\nrequires = [{ scope = 'local', resource_id = 'delivery' }]\n").unwrap();
    std::fs::write(
        package.join("startup-links.md"),
        "# Startup navigation\n\n[Delivery](delivery.md#deliver-from-a-committed-occurrence)\n",
    )
    .unwrap();
    let procedure = package.join("check.md");
    let text = std::fs::read_to_string(&procedure).unwrap();
    std::fs::write(procedure, format!("{text}\n[Effects](workflow.md)\n")).unwrap();
    let path = fixture.root.path().join("workspace/capabilities.toml");
    let mut capabilities: toml::Value =
        toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    capabilities["operation_bindings"][0]["binding"]["selection"] =
        toml::from_str("kind = 'internal'").unwrap();
    capabilities["semantic"].as_table_mut().unwrap().insert("doctrine".into(), toml::from_str::<toml::Value>(
        "items = [{ selection = 'exact', resource = { package_id = 'pale.automation', resource_id = 'startup-links' } }]").unwrap()["items"].clone());
    std::fs::write(path, toml::to_string(&capabilities).unwrap()).unwrap();
    fixture.refresh_distribution_lock();
    let compilation = fixture.compile().unwrap();
    let surfaces = &compilation.repositories()[0].render().surfaces;
    assert_eq!(
        surfaces
            .iter()
            .filter(|surface| surface.rel_path.contains("/internal-operations/")
                && surface.content().contains("[Effects]"))
            .count(),
        2
    );
    assert_eq!(
        surfaces
            .iter()
            .filter(
                |surface| surface.kind == "injection" && surface.content().contains("[Delivery]")
            )
            .count(),
        2
    );
    assert!(assert_local_links_resolve(&compilation) > 0);
}

fn select_resource(fixture: &SyntheticWorkspace, package: &str, field: &str, id: &str) {
    let path = fixture.root.path().join("workspace/capabilities.toml");
    let mut value: toml::Value = toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    value["semantic"]["operations"] = toml::Value::Array(Vec::new());
    value["operation_bindings"] = toml::Value::Array(Vec::new());
    value["semantic"].as_table_mut().unwrap().insert(
        field.into(),
        toml::from_str::<toml::Value>(&format!(
            "items = [{{ selection = 'exact', resource = {{ package_id = '{package}', resource_id = '{id}' }} }}]"
        )).unwrap()["items"].clone(),
    );
    std::fs::write(path, toml::to_string(&value).unwrap()).unwrap();
}

fn add_doctrine(fixture: &SyntheticWorkspace, package: &str, id: &str) {
    let path = fixture.root.path().join("workspace/capabilities.toml");
    let mut value: toml::Value = toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let item = toml::from_str::<toml::Value>(&format!(
        "selection = 'exact'\nresource = {{ package_id = '{package}', resource_id = '{id}' }}"
    ))
    .unwrap();
    value["semantic"]
        .as_table_mut()
        .unwrap()
        .entry("doctrine")
        .or_insert_with(|| toml::Value::Array(Vec::new()))
        .as_array_mut()
        .unwrap()
        .push(item);
    std::fs::write(path, toml::to_string(&value).unwrap()).unwrap();
}

#[test]
fn internal_standalone_doctrine_can_be_a_local_link_but_not_a_project_choice() {
    let fixture = fixture(&[("pale.design", "build-interface")]);
    let path = fixture
        .root
        .path()
        .join("distribution/packages/design/package.toml");
    let mut manifest: toml::Value =
        toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let resource = manifest["package"]["resources"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["id"].as_str() == Some("animate"))
        .unwrap();
    resource["exposure"] = "internal".into();
    let motion_path = fixture
        .root
        .path()
        .join("distribution/packages/design/animate.md");
    let motion = std::fs::read_to_string(&motion_path).unwrap();
    std::fs::write(
        motion_path,
        motion.replacen("[guide]", "[guide]\nexposure = \"internal\"", 1),
    )
    .unwrap();
    std::fs::write(path, toml::to_string(&manifest).unwrap()).unwrap();
    select_resource(&fixture, "pale.design", "doctrine", "inspect");
    let body_path = fixture
        .root
        .path()
        .join("distribution/packages/design/inspect.md");
    let body = std::fs::read_to_string(&body_path).unwrap();
    std::fs::write(body_path, format!("{body}\n[Motion](animate.md)\n")).unwrap();
    fixture.refresh_distribution_lock();
    let compilation = fixture.compile().unwrap();
    for host in ["codex", "claude"] {
        let skill = format!(".{host}/skills/pale-doctrine-pale-design-animate/SKILL.md");
        assert!(!compilation.repositories()[0]
            .render()
            .surfaces
            .iter()
            .any(|surface| surface.rel_path == skill));
        assert!(compilation.repositories()[0]
            .render()
            .surfaces
            .iter()
            .any(|surface| surface.content().contains("`pale.design`, `animate.md`")));
    }
    assert_local_links_resolve(&compilation);

    select_resource(&fixture, "pale.design", "doctrine", "animate");
    assert!(
        fixture.compile().is_err(),
        "the project cannot choose an internal ID directly"
    );
}

#[test]
fn guide_prints_header_prerequisites_and_related_topics_before_the_body() {
    for selected_related in [false, true] {
        let fixture = fixture(&[("pale.design", "build-interface")]);
        let path = fixture
            .root
            .path()
            .join("distribution/packages/design/inspect.md");
        replace_page_body(
            &path,
            "# Rendered interface checks\n\nInspect the example in the browser.\n",
        );
        select_resource(&fixture, "pale.design", "doctrine", "inspect");
        if selected_related {
            add_doctrine(&fixture, "pale.design", "animate");
        }
        fixture.refresh_distribution_lock();
        let compilation = fixture.compile().unwrap();
        for host in ["codex", "claude"] {
            let path = format!(".{host}/skills/pale-doctrine-pale-design-inspect/SKILL.md");
            let text = compilation.repositories()[0]
                .render()
                .surfaces
                .iter()
                .find(|surface| surface.rel_path == path)
                .unwrap()
                .content();
            let (header, method) = text
                .split_once("Inspect the example in the browser.")
                .unwrap();
            let required = header
                .split("## Read first\n\n")
                .nth(1)
                .and_then(|section| section.split("## Related reading").next())
                .unwrap();
            let related = header.split("## Related reading\n\n").nth(1).unwrap();
            assert!(required.contains("pale-doctrine-pale-design-access-web"));
            assert!(related.contains("`pale.design`, `animate.md`"));
            let motion_path = format!(".{host}/skills/pale-doctrine-pale-design-animate/SKILL.md");
            assert_eq!(
                compilation.repositories()[0]
                    .render()
                    .surfaces
                    .iter()
                    .any(|surface| surface.rel_path == motion_path),
                selected_related
            );
            if selected_related {
                assert!(related.contains("[Design motion that helps people use the interface](../pale-doctrine-pale-design-animate/SKILL.md)"));
            } else {
                assert!(related.contains(
                    "Design motion that helps people use the interface, `pale.design`, `animate.md`"
                ));
                assert!(!related.contains("pale-doctrine-pale-design-animate"));
            }
            assert!(!method.contains("## Read first"));
            assert!(!method.contains("## Related reading"));
            assert!(!text.contains(&body("design/animate.md")));
        }
        assert_local_links_resolve(&compilation);
    }
}
#[test]
fn motion_entry_and_stack_overlays_keep_their_own_work() {
    let shared_motion_opening = body("design/animate.md")
        .split("\n\n")
        .next()
        .unwrap()
        .to_owned();
    for (field, resource, guide, snippet) in [
        ("doctrine", "animate", "animate", None),
        (
            "stacks",
            "animate/animejs/guide",
            "animate/animejs/guide",
            Some("createTimeline"),
        ),
        (
            "stacks",
            "animate/motion/guide",
            "animate/motion/guide",
            Some("animate()"),
        ),
        (
            "stacks",
            "animate/scroll/guide",
            "animate/scroll/guide",
            Some("Intersection Observer"),
        ),
    ] {
        let fixture = fixture(&[("pale.design", "build-interface")]);
        select_resource(&fixture, "pale.design", field, resource);
        let compilation = fixture.compile().unwrap();
        for host in ["codex", "claude"] {
            let surfaces = &compilation.repositories()[0].render().surfaces;
            let kind = if field == "stacks" {
                "stack"
            } else {
                "doctrine"
            };
            let trace = format!("{kind}:pale.design:{guide}");
            let text = surfaces
                .iter()
                .find(|surface| {
                    surface.rel_path.starts_with(&format!(".{host}/"))
                        && surface.kind == format!("{kind}-skill")
                        && surface.referenced_policy_ids.contains(&trace)
                })
                .expect("motion entry or library guide is available")
                .content();
            if let Some(snippet) = snippet {
                assert!(text.contains(snippet));
                assert!(!text.contains(&shared_motion_opening));
                assert!(text.contains("../pale-doctrine-pale-design-animate/SKILL.md"));
                let required = surfaces
                    .iter()
                    .find(|surface| {
                        surface.rel_path
                            == format!(".{host}/skills/pale-doctrine-pale-design-animate/SKILL.md")
                    })
                    .unwrap();
                assert!(normalize(required.content()).contains(&shared_motion_opening));
            } else {
                assert!(normalize(text).contains(&shared_motion_opening));
                assert!(!text.contains("createTimeline()"));
                assert!(!text.contains("Worked example: reverse a drawer"));
            }
            assert!(!surfaces
                .iter()
                .any(|surface| surface.kind == "operation-skill"));
        }
        assert_local_links_resolve(&compilation);
    }
}

#[test]
fn recovery_composes_required_input_once_and_preserves_real_links() {
    let fixture = fixture(&[
        ("pale.architecture", "restore"),
        ("pale.architecture", "contracts"),
        ("pale.architecture", "propose-design"),
    ]);
    let compilation = fixture.compile().unwrap();
    for host in ["codex", "claude"] {
        let text = normalize(operation(&compilation, host, "pale-architecture-restore"));
        assert!(!text.contains(&body("architecture/verified-input.md")));
        assert!(text.contains("../pale-doctrine-pale-architecture-verified-input/SKILL.md"));
        let required = compilation.repositories()[0]
            .render()
            .surfaces
            .iter()
            .filter(|surface| surface.rel_path.starts_with(&format!(".{host}/")))
            .map(|surface| normalize(surface.content()))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(
            required
                .matches(&body("architecture/verified-input.md"))
                .count(),
            1
        );
        assert!(!text.contains(&body("architecture/shared-storage.md")));
        assert!(!compilation.repositories()[0]
            .render()
            .surfaces
            .iter()
            .any(|surface| surface
                .rel_path
                .starts_with(&format!(".{host}/skills/pale-doctrine-pale-automation-"))));
    }
    assert_local_links_resolve(&compilation);
}

#[test]
fn prose_links_cannot_select_undeclared_or_missing_resources() {
    let href = "delivery.md#deliver-from-a-committed-occurrence";
    for (replacement, rank) in [
        ("missing.md", "1"),
        ("../../outside.md", "0"),
        ("missing.md?private=sensitive-marker", "0"),
        ("delivery.md#missing-heading", "5"),
        ("#missing-heading", "5"),
    ] {
        let fixture = fixture(&[("pale.automation", "check")]);
        let path = fixture
            .root
            .path()
            .join("distribution/packages/automation/workflow.md");
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::write(path, text.replacen(href, replacement, 1)).unwrap();
        fixture.refresh_distribution_lock();
        let error = fixture
            .compile()
            .expect_err("invalid navigation cannot reach rendering");
        assert_eq!(
            error.failure_disposition(),
            ApplicationFailureDisposition::InvalidInput
        );
        assert!(!error.to_string().contains("sensitive-marker"));
        assert_link_diagnostic(error, replacement, rank, "workflow");
    }
    let fixture = fixture(&[("pale.automation", "check")]);
    let package = fixture.root.path().join("distribution/packages/automation");
    let path = package.join("package.toml");
    let mut manifest: toml::Value =
        toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    manifest["package"]["sources"]
        .as_array_mut()
        .unwrap()
        .extend([
            toml::from_str("path = 'legacy-navigation.md'\nrole = 'body'").unwrap(),
            toml::from_str("path = 'legacy-navigation.toml'\nrole = 'declaration'").unwrap(),
        ]);
    manifest["package"]["resources"].as_array_mut().unwrap().push(
        toml::from_str("id = 'legacy-navigation'\nkind = 'doctrine'\ndefinition_source = 'legacy-navigation.toml'\nexposure = 'exported'").unwrap());
    std::fs::write(path, toml::to_string(&manifest).unwrap()).unwrap();
    std::fs::write(package.join("legacy-navigation.toml"), "[doctrine]\nschema_version = 1\n[[doctrine.entry]]\nid = 'legacy-navigation'\ndescription = 'Legacy edge check'\ndoctrine = 'legacy-navigation.md'\nplacement = 'standalone_judgment'\n").unwrap();
    std::fs::write(
        package.join("legacy-navigation.md"),
        format!("# Legacy navigation\n\n[Delivery]({href})\n"),
    )
    .unwrap();
    fixture.refresh_distribution_lock();
    let error = fixture
        .compile()
        .expect_err("prose cannot invent a missing typed relationship");
    assert_link_diagnostic(error, href, "2", "legacy-navigation");
}

fn assert_link_diagnostic(error: CompileError, href: &str, rank: &str, resource: &str) {
    use pale_application::packages::{PackageDiagnosticCode, PackageError};
    let CompileError::PackageLoad(error) = error else {
        panic!("{error}")
    };
    let PackageError::Diagnostics(diagnostics) = *error else {
        panic!("{error}")
    };
    let destination = ArtifactDigest::hash_bytes(href.as_bytes());
    let resource = format!("pale.automation/{resource}");
    assert!(diagnostics.entries().iter().any(|entry| entry.code()
        == PackageDiagnosticCode::BodyCustody
        && entry.coordinates()
            == [
                "markdown_link",
                resource.as_str(),
                destination.as_str(),
                rank
            ]));
}

#[test]
fn editorial_review_offers_specialists_without_loading_their_procedures() {
    let fixture = fixture(&[("pale.publication", "edit")]);
    let compilation = fixture.compile().unwrap();
    for host in ["codex", "claude"] {
        let text = normalize(operation(&compilation, host, "pale-publication-edit"));
        for paragraph in body("publication/edit.md")
            .split("\n\n")
            .filter(|p| !p.contains("](") && *p != "Before handing off")
        {
            assert_eq!(text.matches(paragraph).count(), 1, "{host}: {paragraph}");
        }
        for source in [
            "publication/reveals.md",
            "publication/reviewed-media.md",
            "publication/documentation.md",
            "research/source-expression.md",
            "research/source-rights.md",
            "evaluation/review.md",
        ] {
            assert!(
                !text.contains(&body(source)),
                "unselected review procedure: {source}"
            );
        }
        assert!(text.contains("`pale.publication`"));
        let rendered = compilation.repositories()[0].render();
        for resource in ["documentation", "reveals", "reviewed-media"] {
            assert!(!rendered.surfaces.iter().any(|s| s.rel_path
                == format!(".{host}/skills/pale-doctrine-pale-publication-{resource}/SKILL.md")));
        }
    }
    assert_local_links_resolve(&compilation);
}

#[test]
fn publication_requires_native_evidence_without_inventing_a_renderer() {
    let fixture = fixture(&[("pale.publication", "local-build")]);
    let compilation = fixture.compile().unwrap();
    assert!(compilation.executable_identity_digest().is_none());
    for host in ["codex", "claude"] {
        let text = normalize(operation(
            &compilation,
            host,
            "pale-publication-local-build",
        ));
        assert!(!text.contains(&body("engineering/execution.md")));
        assert!(text.contains("../pale-doctrine-pale-engineering-execution/SKILL.md"));
        let required = compilation.repositories()[0]
            .render()
            .surfaces
            .iter()
            .filter(|surface| surface.rel_path.starts_with(&format!(".{host}/")))
            .map(|surface| normalize(surface.content()))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(
            required.matches(&body("engineering/execution.md")).count(),
            1
        );
        assert_eq!(
            required.matches(&body("publication/references.md")).count(),
            1
        );
        assert!(text.contains("Status: blocked; a required tool is unavailable"));
        assert!(text.contains("publication-local-render"));
        assert!(text.contains("Match a preview to its built files"));
        assert!(!text.contains("A watched page can load part of an earlier"));
        assert!(text.contains("`pale.publication`, `build/preview.md`"));
        let preview_path =
            format!(".{host}/skills/pale-doctrine-pale-publication-build-preview/SKILL.md");
        assert!(!compilation.repositories()[0]
            .render()
            .surfaces
            .iter()
            .any(|surface| surface.rel_path == preview_path));
    }
    assert_local_links_resolve(&compilation);
}

#[test]
fn content_adaptation_composes_meaning_once_without_a_browser_duty() {
    let fixture = fixture(&[("pale.publication", "adapt-media")]);
    let compilation = fixture.compile().unwrap();
    for host in ["codex", "claude"] {
        let text = normalize(operation(
            &compilation,
            host,
            "pale-publication-adapt-media",
        ));
        assert_eq!(text.matches(&body("publication/adapt-media.md")).count(), 1);
        let authored = body("design/presentation.md");
        // The task table links to optional guides beside the operation.
        let required = authored.split("\n\n|").next().unwrap();
        assert!(!text.contains(required));
        assert!(text.contains("../pale-doctrine-pale-design-presentation/SKILL.md"));
        let guidance = compilation.repositories()[0]
            .render()
            .surfaces
            .iter()
            .filter(|surface| surface.rel_path.starts_with(&format!(".{host}/")))
            .map(|surface| normalize(surface.content()))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(guidance.matches(required).count(), 1);
        assert!(!guidance.contains(&body("design/access.md")));
        assert!(!text.contains(&body("design/access/web.md")));
        assert!(!text.contains(&body("design/inspect.md")));
    }
    assert_local_links_resolve(&compilation);
}

#[test]
fn ordinary_audit_and_browser_journeys_offer_variant_review_without_loading_it() {
    let authored = body("design/variations.md");
    let comparison = authored
        .split("\nUse [source-preserving presentation]")
        .next()
        .unwrap()
        .trim();
    for selection in ["audit-design", "browser-task"] {
        let fixture = fixture(&[("pale.design", selection)]);
        let compilation = fixture.compile().unwrap();
        for host in ["codex", "claude"] {
            let text = normalize(operation(
                &compilation,
                host,
                &format!("pale-design-{selection}"),
            ));
            assert!(!text.contains(comparison), "{host}: {selection}");
            let path = format!(".{host}/skills/pale-doctrine-pale-design-variations/SKILL.md");
            assert!(!compilation.repositories()[0]
                .render()
                .surfaces
                .iter()
                .any(|surface| surface.rel_path == path));
        }
        assert_local_links_resolve(&compilation);
    }
}

#[test]
fn delegation_composes_required_methods_without_a_native_dispatch_claim() {
    let fixture = fixture(&[("pale.agent-work", "delegate")]);
    let compilation = fixture.compile().unwrap();
    assert!(compilation.executable_identity_digest().is_none());
    for host in ["codex", "claude"] {
        let text = normalize(operation(&compilation, host, "pale-agent-work-delegate"));
        assert_eq!(text.matches(&body("agent-work/delegate.md")).count(), 1);
        assert!(!text.contains(&body("engineering/handoff.md")));
        assert!(text.contains("../pale-doctrine-pale-engineering-handoff/SKILL.md"));
        let all = compilation.repositories()[0]
            .render()
            .surfaces
            .iter()
            .filter(|surface| surface.rel_path.starts_with(&format!(".{host}/")))
            .map(|surface| normalize(surface.content()))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(all.matches(&body("engineering/handoff.md")).count(), 1);
    }
}

#[test]
fn langfuse_tooling_preserves_optional_lineage_and_starts_no_workflow() {
    for (field, resource) in [("doctrine", "langfuse-tools"), ("stacks", "langfuse")] {
        let fixture = fixture(&[("pale.evaluation", "compare-runs")]);
        let path = fixture.root.path().join("workspace/capabilities.toml");
        let mut capabilities: toml::Value =
            toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        capabilities["semantic"]["operations"] = toml::Value::Array(Vec::new());
        capabilities["operation_bindings"] = toml::Value::Array(Vec::new());
        capabilities["semantic"].as_table_mut().unwrap().insert(
            field.into(),
            toml::from_str::<toml::Value>(&format!(
                "items = [{{ selection = 'exact', resource = {{ package_id = 'pale.evaluation', resource_id = '{resource}' }} }}]"
            )).unwrap()["items"].clone(),
        );
        std::fs::write(path, toml::to_string(&capabilities).unwrap()).unwrap();
        fixture.refresh_distribution_lock();
        let compilation = fixture.compile().unwrap();
        assert!(compilation.executable_identity_digest().is_none());
        let surfaces = &compilation.repositories()[0].render().surfaces;
        assert!(!surfaces.iter().any(|s| s.kind == "operation-skill"));
        for host in ["codex", "claude"] {
            let path =
                format!(".{host}/skills/pale-doctrine-pale-evaluation-langfuse-tools/SKILL.md");
            let matches = surfaces
                .iter()
                .filter(|s| s.rel_path == path)
                .collect::<Vec<_>>();
            assert_eq!(matches.len(), 1, "{host}: {field}");
            let text = matches[0].content();
            assert!(text.contains("@langfuse/cli@1.3.1 api schema --json"));
            assert!(text.contains("f0bcb13088b490c04b3f4870bace17f6d57b199c"));
            assert!(!normalize(text).contains(&body("evaluation/langfuse.md")));
            let practice = surfaces.iter().find(|s| {
                s.rel_path == format!(".{host}/skills/pale-stack-pale-evaluation-langfuse/SKILL.md")
            });
            assert_eq!(practice.is_some(), field == "stacks");
            if let Some(practice) = practice {
                let text = normalize(practice.content());
                for paragraph in body("evaluation/langfuse.md")
                    .split("\n\n")
                    .filter(|paragraph| !paragraph.contains("]("))
                {
                    assert_eq!(text.matches(paragraph).count(), 1, "{host}: {paragraph}");
                }
                assert!(text.contains("../pale-doctrine-pale-evaluation-langfuse-tools/SKILL.md"));
            }
            if field == "doctrine" {
                assert!(text.contains("`pale.evaluation`, `langfuse.md`"));
            }
            assert!(!surfaces
                .iter()
                .filter(|s| s.kind == "injection")
                .any(|s| s.content().contains("@langfuse/cli@1.3.1")));
        }
        assert_local_links_resolve(&compilation);
    }
}

#[test]
fn family_reading_links_open_related_guides_without_loading_their_methods() {
    for (family, operation, entry, guide, example) in [
        (
            "engineering",
            "testing/repository",
            "change",
            "error-handling",
            "A room reservation times out",
        ),
        (
            "evaluation",
            "compare-runs",
            "notebook",
            "jupyter",
            "jupyter nbconvert --execute",
        ),
        (
            "research",
            "reuse",
            "source-expression",
            "source-rights",
            "Abstract component example",
        ),
        (
            "agent-work",
            "harness",
            "agents",
            "regression",
            "A review agent used a generated summary",
        ),
    ] {
        let package = format!("pale.{family}");
        let fixture = fixture(&[(&package, operation)]);
        select_resource(&fixture, &package, "doctrine", entry);
        let compilation = fixture.compile().unwrap();
        let surfaces = &compilation.repositories()[0].render().surfaces;
        assert!(!surfaces
            .iter()
            .any(|surface| surface.kind == "operation-skill"));
        for host in ["codex", "claude"] {
            let skill = |id| {
                let path = format!(".{host}/skills/pale-doctrine-pale-{family}-{id}/SKILL.md");
                surfaces
                    .iter()
                    .find(|surface| surface.rel_path == path)
                    .unwrap_or_else(|| panic!("missing {path}"))
                    .content()
            };
            let target = format!("../pale-doctrine-pale-{family}-{guide}/SKILL.md");
            assert!(!skill(entry).contains(&target), "{host}: {entry}");
            assert!(!skill(entry).contains(example), "{host}: {entry}");
            assert!(
                skill(entry).contains(&format!("`{package}`")),
                "{host}: {entry}"
            );
            assert!(!surfaces.iter().any(|surface| surface.rel_path
                == format!(".{host}/skills/pale-doctrine-pale-{family}-{guide}/SKILL.md")));
        }
        assert_local_links_resolve(&compilation);
        add_doctrine(&fixture, &package, guide);
        let selected = fixture.compile().unwrap();
        for host in ["codex", "claude"] {
            let surfaces = &selected.repositories()[0].render().surfaces;
            let skill = |id| {
                surfaces
                    .iter()
                    .find(|s| {
                        s.rel_path
                            == format!(".{host}/skills/pale-doctrine-pale-{family}-{id}/SKILL.md")
                    })
                    .unwrap()
                    .content()
            };
            assert!(
                skill(entry).contains(&format!("../pale-doctrine-pale-{family}-{guide}/SKILL.md"))
            );
            assert!(skill(guide).contains(example));
        }
        assert!(assert_local_links_resolve(&selected) > 0);
    }
}
