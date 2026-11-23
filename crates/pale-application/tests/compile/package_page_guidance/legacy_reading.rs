//! Header-page prerequisites remain readable across legacy guide placements.

use super::*;

#[test]
fn required_legacy_reading_gets_a_surface_without_changing_unlinked_placement() {
    for placement in ["operation_procedure", "startup_policy"] {
        for required in [false, true] {
            let fixture = page_fixture();
            let package = fixture.root.path().join("distribution/packages/engine");
            let manifest_path = package.join("package.toml");
            let mut manifest: toml::Value =
                toml::from_str(&std::fs::read_to_string(&manifest_path).unwrap()).unwrap();
            manifest["package"]["sources"][0]["role"] = "body".into();
            manifest["package"]["sources"]
                .as_array_mut()
                .unwrap()
                .push(toml::from_str("path = 'legacy.toml'\nrole = 'declaration'").unwrap());
            manifest["package"]["resources"][0]["definition_source"] = "legacy.toml".into();
            std::fs::write(manifest_path, toml::to_string(&manifest).unwrap()).unwrap();
            std::fs::write(package.join("legacy.toml"), format!(
                "[doctrine]\nschema_version = 1\n[[doctrine.entry]]\nid = 'work'\ndescription = 'Read the original method.'\ndoctrine = 'work.md'\nplacement = '{placement}'\n"
            )).unwrap();
            std::fs::write(
                package.join("work.md"),
                "# Original method\n\nLEGACY READING BODY.\n",
            )
            .unwrap();
            if required {
                let path = package.join("work/review.md");
                let text = std::fs::read_to_string(&path).unwrap().replace(
                    "[guide]\n",
                    "uses = [{ scope = 'local', resource_id = 'work' }]\n[guide]\n",
                );
                std::fs::write(path, text).unwrap();
            }
            let path = fixture.root.path().join("workspace/capabilities.toml");
            let mut capabilities: toml::Value =
                toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            capabilities["semantic"]["doctrine"].as_array_mut().unwrap().insert(
                0, toml::from_str("selection = 'exact'\nresource = { package_id = 'example.engine', resource_id = 'work' }").unwrap(),
            );
            std::fs::write(path, toml::to_string(&capabilities).unwrap()).unwrap();
            fixture.refresh_distribution_lock();
            let profile = effective(&fixture);
            for host in [SourceGuidanceHost::Codex, SourceGuidanceHost::ClaudeCode] {
                let projection = profile
                    .source_guidance(&LogicalRepositoryIdV1::parse("engine").unwrap(), host)
                    .unwrap();
                assert!(check_links_resolve(&projection) > 0);
                let guide = projection.files().iter().find(|file| {
                    file.rel_path()
                        .ends_with("/pale-doctrine-example-engine-work/SKILL.md")
                });
                assert_eq!(
                    guide.is_some(),
                    required,
                    "{placement}: required={required}"
                );
                if let Some(guide) = guide {
                    assert!(guide.content().contains("LEGACY READING BODY."));
                }
                let index = projection
                    .files()
                    .iter()
                    .find(|file| file.rel_path().ends_with("/index.md"))
                    .unwrap();
                assert_eq!(
                    index.content().contains("LEGACY READING BODY."),
                    placement == "startup_policy",
                    "startup behavior remains independent of linked reading"
                );
            }
        }
    }
}
