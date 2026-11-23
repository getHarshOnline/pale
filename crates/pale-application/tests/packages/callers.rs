//! Incoming caller links share the compiler's target and Markdown interpretation.

use std::path::Path;

use pale_application::packages::{read_catalog, PackageCallerIssueKind};

use crate::common::{write_manifest, write_policy_package};

#[test]
fn formatting_checks_each_live_body_once_and_leaves_original_documents_alone() {
    let root = tempfile::tempdir().unwrap();
    let package = root.path().join("packages/example");
    std::fs::create_dir_all(&package).unwrap();
    std::fs::write(package.join("package.toml"), "[package]\nid = 'example.methods'\nversion = '1.0.0'\nschema_version = 1\ntitle = 'Methods'\ndescription = 'Fixture'\ncompatible_compilers = ['pale-surface@1']\n").unwrap();
    let source = "+++\ndescription = 'Check a change'\n[guide]\n[task]\noutputs = ['Review findings']\n+++\n# Entry\n\nWrapped\nparagraph.\n";
    std::fs::write(package.join("entry.md"), source).unwrap();
    std::fs::write(
        package.join("original.md"),
        "# Original\n\nKeep this\noriginal intact.\n",
    )
    .unwrap();
    let catalog = read_catalog(&pale_adapter_fs::LocalSourceFilesystem, root.path()).unwrap();
    let issues = catalog.formatting_issues();
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].source, Path::new("packages/example/entry.md"));
    assert_eq!(issues[0].line, 9);
    assert_eq!(
        std::fs::read_to_string(package.join("entry.md")).unwrap(),
        source
    );
}

#[test]
fn declared_markdown_bodies_keep_anchors_without_an_md_extension() {
    let root = tempfile::tempdir().unwrap();
    let package = write_manifest(root.path(), "example", "example.policy", "family.toml", "body.txt",
        "{ id = 'policy', kind = 'policy_family', definition_source = 'family.toml', exposure = 'exported' }");
    std::fs::write(
        package.join("family.toml"),
        "id = 'policy'\ntitle = 'Policy'\npurpose = 'Fixture'\nbody = 'body.txt'\n",
    )
    .unwrap();
    std::fs::write(package.join("body.txt"), "# Policy\n").unwrap();
    let catalog = read_catalog(&pale_adapter_fs::LocalSourceFilesystem, root.path()).unwrap();
    let report = catalog.check_caller(
        root.path(),
        &root.path().join("README.md"),
        "[policy](packages/example/body.txt#policy)",
    );
    assert_eq!(report.references, 1);
    assert!(report.issues.is_empty());
}

#[test]
fn caller_checks_links_ids_and_fragments_without_reading_live_targets() {
    let root = tempfile::tempdir().unwrap();
    write_policy_package(
        root.path(),
        "example",
        "example.policy",
        "# Policy\n\n<a id=\"topic:part\"></a>\n\n## Évidence\n",
    );
    let catalog = read_catalog(&pale_adapter_fs::LocalSourceFilesystem, root.path()).unwrap();
    let caller = root.path().join("notes/Caller.md");
    let markdown = concat!(
        "\n# Caller\n\n",
        "[valid](../packages/example/body.md#policy)\n",
        "[encoded](../packages/example/body.md#topic%3Apart)\n",
        "[reference][method]\n\n",
        "[method]: ../packages/example/body.md#%C3%A9vidence\n\n",
        "`example.policy:policy`\n",
        "[directory](../packages/example/)\n",
        "[missing](../packages/example/gone.md)\n",
        "[anchor](../packages/example/body.md#absent)\n",
        "`example.policy:renamed`\n",
        "`missing.family:method`\n",
        "[external](https://example.com/packages/example/gone.md)\n",
        "[elsewhere](../other/gone.md)\n",
        "`a-normal-code-word`\n\n",
        "`next.config.ts:30` `.gitignore:28` `worker.go:93` `schemas.rs:239-241`\n\n",
        "```md\n[example](../packages/example/gone.md)\n`example.policy:old`\n```\n",
    );
    std::fs::write(root.path().join("packages/example/body.md"), "# Changed\n").unwrap();
    let result = catalog.check_caller(root.path(), &caller, markdown);
    assert_eq!(result.documents, 1);
    assert_eq!(result.references, 9);
    assert_eq!(
        result
            .issues
            .iter()
            .map(|issue| (issue.line, issue.kind))
            .collect::<Vec<_>>(),
        [
            (12, PackageCallerIssueKind::MissingFile),
            (13, PackageCallerIssueKind::MissingAnchor),
            (14, PackageCallerIssueKind::UnknownResource),
            (15, PackageCallerIssueKind::UnknownResource),
        ]
    );
    assert!(result.issues.iter().all(|issue| issue.source == caller));
}

#[test]
fn caller_paths_handle_spaces_absolute_links_and_parent_segments() {
    let root = tempfile::Builder::new()
        .prefix("caller space ")
        .tempdir()
        .unwrap();
    write_policy_package(root.path(), "example", "example.policy", "# Policy\n");
    let catalog = read_catalog(&pale_adapter_fs::LocalSourceFilesystem, root.path()).unwrap();
    let target = root.path().join("packages/example/body.md");
    for destination in [
        target.display().to_string(),
        target.display().to_string().replace(' ', "%20"),
        format!("file://{}#policy", target.display()),
        "../packages/example/../example/body.md#policy".to_string(),
    ] {
        let result = catalog.check_caller(
            root.path(),
            &root.path().join("notes/a.md"),
            &format!("[method](<{destination}>)"),
        );
        assert_eq!(result.references, 1, "{destination}");
        assert!(
            result.issues.is_empty(),
            "{destination}: {:?}",
            result.issues
        );
    }
    let outside = catalog.check_caller(
        root.path(),
        Path::new("/elsewhere/caller.md"),
        "[local](packages/example/gone.md)",
    );
    assert_eq!(outside.references, 0);
}
