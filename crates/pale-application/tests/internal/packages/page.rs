//! Page headers and bodies retain their separate original source coordinates.

use super::*;

#[test]
fn folder_reading_chooses_the_nearest_authored_topic_and_never_a_sibling() {
    let pages = ["motion", "motion/timing", "motion/feedback", "other"]
        .into_iter()
        .map(|path| PackageResourceIdV1::parse(path).unwrap())
        .collect::<BTreeSet<_>>();
    for (page, expected) in [
        ("motion/timing/duration", Some("motion/timing")),
        ("motion/patterns/feedback", Some("motion")),
        ("motion/feedback", Some("motion")),
        ("motion", None),
        ("unrelated/start", None),
    ] {
        let page = PackageResourceIdV1::parse(page).unwrap();
        assert_eq!(
            parent_topic(&page, &pages).map(PackageResourceIdV1::as_str),
            expected
        );
    }
}

#[test]
fn typed_page_reader_checks_the_header_and_keeps_the_exact_body() {
    let source = concat!(
        "+++\r\n",
        "description = 'Review a change.'\r\n",
        "[guide]\r\nexposure = 'internal'\r\n",
        "[task]\r\noutputs = ['Findings in the reply.']\r\n",
        "[role]\r\n[policy]\r\n[stack]\r\n",
        "+++\r\n",
        "\r\n# Review λ\r\n\r\nKeep the original body.\r\n",
    );
    let page = read_page(source, Path::new("review.md")).unwrap();
    assert!(page.header.is_guide());
    assert_eq!(
        page.header.guide.unwrap().exposure,
        Some(pale_core::records::PackageResourceExposureV1::Internal)
    );
    assert_eq!(
        page.header.task.unwrap().outputs,
        ["Findings in the reply."]
    );
    assert!(page.header.role.is_some());
    assert!(page.header.policy.is_some());
    assert!(page.header.stack.is_some());
    assert_eq!(
        &source[page.parts.markdown],
        "\r\n# Review λ\r\n\r\nKeep the original body.\r\n"
    );
}

#[test]
fn page_reader_rejects_legacy_fields_invalid_shapes_and_empty_descriptions() {
    for declaration in [
        "description = ''",
        "description = 'Review.'\nid = 'other'",
        "description = 'Review.'\nplacement = 'standalone_judgment'",
        "description = 'Review.'\nlens = true",
        "description = 'Review.'\n[task]\nbody = 'other.md'",
        "description = 'Review.'\nuses = [{ scope = 'local', resource_id = '../escape' }]",
    ] {
        let source = format!("+++\n{declaration}\n+++\n# Review\n");
        assert!(
            matches!(
                read_page(&source, Path::new("review.md")),
                Err(PackageError::SourceSchema { .. })
            ),
            "{declaration}"
        );
    }
}

#[test]
fn page_toml_errors_do_not_retain_the_source_body_or_header_values() {
    let source =
        "+++\ndescription = 'private-header-value'\nbroken = \n+++\n# Private body value\n";
    let error = read_page(source, Path::new("review.md")).err().unwrap();
    assert!(matches!(error, PackageError::TomlSyntax { .. }));
    let message = error.to_string();
    assert!(!message.contains("private-header-value"));
    assert!(!message.contains("Private body value"));
}

#[test]
fn syntax_error_spans_point_into_the_original_page() {
    for newline in ["\n", "\r\n"] {
        let source = format!(
            "+++{newline}description = 'Review λ.'{newline}broken = ?{newline}+++{newline}# Review{newline}"
        );
        let PackageError::TomlSyntax { source: error, .. } =
            read_page(&source, Path::new("review.md")).err().unwrap()
        else {
            panic!("invalid TOML must retain its syntax error");
        };
        let span = error.span().expect("invalid value has a source span");
        assert_eq!(span.start, source.find('?').unwrap());
    }
}

#[test]
fn page_ranges_preserve_line_endings_unicode_and_later_delimiters() {
    for newline in ["\n", "\r\n"] {
        let declaration =
            format!("description = 'Review λ.'{newline}[guide]{newline}[task]{newline}");
        let markdown =
            format!("{newline}# Review λ{newline}{newline}Keep the method.{newline}+++{newline}");
        let source = format!("+++{newline}{declaration}+++{newline}{markdown}");
        let parts = split_page(&source).unwrap();
        assert_eq!(&source[parts.declaration.clone()], declaration);
        assert_eq!(&source[parts.markdown.clone()], markdown);
        let page: pale_core::records::PackagePageV1 =
            toml::from_str(&source[parts.declaration]).unwrap();
        assert!(page.is_guide());
        assert!(page.task.is_some());
    }
}

#[test]
fn declaration_delimiters_are_required_and_cannot_be_found_inside_the_body() {
    for source in [
        "",
        "# Guide\n\n+++\ndescription = 'Late.'\n+++\n",
        " +++\n",
        "+++\r",
    ] {
        assert_eq!(split_page(source), Err(PageHeaderError::MissingOpening));
    }
    assert_eq!(
        split_page("+++\ndescription = 'Unclosed.'\n"),
        Err(PageHeaderError::MissingClosing)
    );
    for source in ["+++\n+++", "+++\n+++\n\n \t"] {
        assert_eq!(split_page(source), Err(PageHeaderError::EmptyBody));
    }
}

#[test]
fn header_limit_does_not_truncate_or_limit_the_markdown_body() {
    let declaration = " ".repeat(MAX_HEADER_BYTES - 1) + "\n";
    let body = "a".repeat(MAX_HEADER_BYTES + 1);
    let source = format!("+++\n{declaration}+++\n{body}");
    let parts = split_page(&source).unwrap();
    assert_eq!(parts.declaration.len(), MAX_HEADER_BYTES);
    assert_eq!(&source[parts.markdown], body);
    let source = format!("+++\n {declaration}+++\n{body}");
    assert_eq!(split_page(&source), Err(PageHeaderError::TooLarge));
}

#[test]
fn unfinished_header_supports_editor_help_without_admitting_a_page() {
    let source = "+++\r\ndescription = ";
    assert_eq!(header_range(source), Some(5..source.len()));
    assert_eq!(split_page(source), Err(PageHeaderError::MissingClosing));
    assert_eq!(header_range("# Text\n+++\n"), None);
    assert_eq!(
        header_range(&format!("+++\n{}", "x".repeat(MAX_HEADER_BYTES + 1))),
        None
    );
}
