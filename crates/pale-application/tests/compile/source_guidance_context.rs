//! Private source context stays separate from portable repository guidance.

use super::*;

#[test]
fn private_context_preserves_exact_file_binding_without_guessing_a_target() {
    let fixture = fixture();
    let mut manifest: toml::Value =
        toml::from_str(&std::fs::read_to_string(&fixture.manifest).unwrap()).unwrap();
    let session: toml::Value = toml::from_str(
        "activation_roots = ['engine']\nauthority_access = [{root_id = 'authority', class = 'read-only'}]\nreading_order = [{root_id = 'authority', rel_path = 'guide.md'}]\n",
    )
    .unwrap();
    manifest["workspace"]["desired_topology"]["execution_workspaces"][0]
        .as_table_mut()
        .unwrap()
        .insert("session".into(), session);
    std::fs::write(&fixture.manifest, toml::to_string(&manifest).unwrap()).unwrap();
    let mut machine: toml::Value =
        toml::from_str(&std::fs::read_to_string(&fixture.machine_binding).unwrap()).unwrap();
    let bound_file =
        Path::new(machine["authority_roots"]["authority"].as_str().unwrap()).join("guide.md");
    assert!(!bound_file.exists());
    machine["authority_roots"]["authority"] = bound_file.to_str().unwrap().into();
    std::fs::write(&fixture.machine_binding, toml::to_string(&machine).unwrap()).unwrap();

    let context = effective(&fixture)
        .repository_context(&LogicalRepositoryIdV1::parse("engine").unwrap())
        .unwrap();
    assert!(context.contains(&format!(
        "Root \"authority\", binding {:?}, relative reference \"guide.md\".",
        bound_file.to_str().unwrap(),
    )));
    assert!(!context.contains("guide.md/guide.md"));
    assert!(context.contains("permits only the exact bound file"));
    assert!(!bound_file.exists());
}

#[test]
fn private_context_preserves_read_order_without_leaking_into_portable_guidance() {
    let fixture = fixture();
    let mut manifest: toml::Value =
        toml::from_str(&std::fs::read_to_string(&fixture.manifest).unwrap()).unwrap();
    let session: toml::Value = toml::from_str(
        "activation_roots = ['engine']\nauthority_access = [{ root_id = 'authority', class = 'read-only' }]\nreading_order = [{root_id = 'authority', rel_path = 'Owner.md'}, {root_id = 'authority', rel_path = 'Duty.md'}]\n",
    )
    .unwrap();
    manifest["workspace"]["desired_topology"]["execution_workspaces"][0]
        .as_table_mut()
        .unwrap()
        .insert("session".into(), session);
    std::fs::write(&fixture.manifest, toml::to_string(&manifest).unwrap()).unwrap();
    let compilation = effective(&fixture);
    let repository = LogicalRepositoryIdV1::parse("engine").unwrap();
    let context = compilation.repository_context(&repository).unwrap();
    let private_root = compilation.instance().authority_roots["authority"].clone();
    assert!(context.contains(&private_root));
    assert!(context.find("Owner.md").unwrap() < context.find("Duty.md").unwrap());
    assert!(context.contains("These sources are read only"));
    assert!(!Path::new(&private_root).join("Owner.md").exists());
    for host in [SourceGuidanceHost::Codex, SourceGuidanceHost::ClaudeCode] {
        let guidance = compilation.source_guidance(&repository, host).unwrap();
        assert!(!guidance.entrypoint_contribution().contains(&private_root));
        assert!(!guidance.entrypoint_contribution().contains("Owner.md"));
        assert!(guidance.files().iter().all(|file| {
            !file.content().contains(&private_root) && !file.content().contains("Owner.md")
        }));
    }
    assert!(compilation
        .repository_context(&LogicalRepositoryIdV1::parse("unselected").unwrap())
        .is_err());

    manifest["workspace"]["desired_topology"]["execution_workspaces"][0]["session"]
        ["authority_access"] = toml::Value::Array(Vec::new());
    std::fs::write(&fixture.manifest, toml::to_string(&manifest).unwrap()).unwrap();
    assert!(effective(&fixture)
        .repository_context(&repository)
        .unwrap_err()
        .to_string()
        .contains("no declared read-only access"));
}
