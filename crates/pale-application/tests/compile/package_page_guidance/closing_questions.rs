//! Closing conditions follow the selected method through both host layouts.

use super::*;

pub(super) const QUESTIONS: &str = "\n## Before handing off\n\n- [ ] Does the change satisfy the request?\n- [ ] Did the relevant check run?\n- [ ] Is the remaining work clear?\n";

pub(super) fn add_questions(fixture: &SyntheticWorkspace, page: &str, questions: &str) {
    let path = fixture
        .root
        .path()
        .join("distribution/packages/engine")
        .join(page);
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(path, format!("{text}{questions}")).unwrap();
    fixture.refresh_distribution_lock();
}

pub(super) fn assert_finished(text: &str) {
    assert_eq!(text.matches("## Before handing off").count(), 1);
    assert_eq!(text.matches("- [ ] ").count(), 3);
    assert!(text
        .trim_end()
        .ends_with("- [ ] Is the remaining work clear?"));
}

#[test]
fn invalid_own_questions_cannot_fall_back_to_a_valid_parent() {
    let fixture = page_fixture();
    add_questions(&fixture, "work.md", QUESTIONS);
    add_questions(
        &fixture,
        "work/review.md",
        "\n## Before handing off\n\n- [ ] Only one?\n",
    );
    let profile = effective(&fixture);
    let result = profile.source_guidance(
        &LogicalRepositoryIdV1::parse("engine").unwrap(),
        SourceGuidanceHost::Codex,
    );
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("three or four unchecked closing questions"));
}

#[test]
fn different_required_questions_need_an_authored_choice() {
    let fixture = page_fixture();
    add_questions(&fixture, "work.md", QUESTIONS);
    let package = fixture.root.path().join("distribution/packages/engine");
    let manifest_path = package.join("package.toml");
    let mut manifest: toml::Value =
        toml::from_str(&std::fs::read_to_string(&manifest_path).unwrap()).unwrap();
    for (field, value) in [
        ("sources", "path = 'other.md'\nrole = 'declaration'"),
        ("resources", "id = 'other'\nkind = 'doctrine'\ndefinition_source = 'other.md'\nexposure = 'exported'"),
    ] {
        manifest["package"][field].as_array_mut().unwrap().push(toml::from_str(value).unwrap());
    }
    std::fs::write(&manifest_path, toml::to_string(&manifest).unwrap()).unwrap();
    std::fs::write(
        package.join("other.md"),
        format!(
            "+++\ndescription = 'Other required guidance.'\n+++\n\n# Other method\n\n{}",
            QUESTIONS.replace("remaining work", "remaining risk")
        ),
    )
    .unwrap();
    let page = package.join("work/review.md");
    let text = std::fs::read_to_string(&page).unwrap().replace(
        "description = 'Review the work.'",
        "description = 'Review the work.'\nuses = [{ scope = 'local', resource_id = 'other' }]",
    );
    std::fs::write(page, text).unwrap();
    fixture.refresh_distribution_lock();
    let profile = effective(&fixture);
    let repository = LogicalRepositoryIdV1::parse("engine").unwrap();
    assert!(profile
        .source_guidance(&repository, SourceGuidanceHost::Codex)
        .unwrap_err()
        .to_string()
        .contains("one authored closing checklist"));
    add_questions(&fixture, "work/review.md", QUESTIONS);
    let profile = effective(&fixture);
    let projection = profile
        .source_guidance(&repository, SourceGuidanceHost::Codex)
        .unwrap();
    for child in projection
        .files()
        .iter()
        .filter(|file| file.content().contains("SHARED REVIEW METHOD"))
    {
        assert_finished(child.content());
    }
}

#[test]
fn authored_questions_follow_host_instructions_in_both_layouts() {
    let fixture = page_fixture();
    add_questions(&fixture, "work/review.md", QUESTIONS);
    let profile = effective(&fixture);
    for host in [SourceGuidanceHost::Codex, SourceGuidanceHost::ClaudeCode] {
        let projection = profile
            .source_guidance(&LogicalRepositoryIdV1::parse("engine").unwrap(), host)
            .unwrap();
        assert_links_resolve(&projection);
        let children = projection
            .files()
            .iter()
            .filter(|file| file.content().contains("SHARED REVIEW METHOD"))
            .collect::<Vec<_>>();
        assert_eq!(children.len(), 4);
        for child in children {
            assert_finished(child.content());
            assert!(child.content().contains("https://example.com/method"));
            if child.rel_path().contains("/roles/") {
                assert!(child
                    .content()
                    .contains("use tools under their configured permissions"));
            }
        }
    }
}

#[test]
fn required_topic_supplies_questions_without_copying_its_method() {
    let fixture = page_fixture();
    add_questions(&fixture, "work.md", QUESTIONS);
    let profile = effective(&fixture);
    for host in [SourceGuidanceHost::Codex, SourceGuidanceHost::ClaudeCode] {
        let projection = profile
            .source_guidance(&LogicalRepositoryIdV1::parse("engine").unwrap(), host)
            .unwrap();
        for child in projection
            .files()
            .iter()
            .filter(|file| file.content().contains("SHARED REVIEW METHOD"))
        {
            assert_finished(child.content());
            assert!(!child.content().contains("PARENT METHOD APPEARS ONLY HERE"));
        }
    }
}

#[test]
fn optional_reading_cannot_supply_closing_conditions() {
    let fixture = mixed_fixture();
    add_questions(&fixture, "optional.md", QUESTIONS);
    let profile = effective(&fixture);
    let projection = profile
        .source_guidance(
            &LogicalRepositoryIdV1::parse("engine").unwrap(),
            SourceGuidanceHost::Codex,
        )
        .unwrap();
    assert!(!projection
        .files()
        .iter()
        .any(|file| file.content().contains("Is the remaining work clear?")));
}
