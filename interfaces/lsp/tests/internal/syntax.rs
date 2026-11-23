//! Tolerant TOML structure and byte-path tests.

use super::*;

fn field(value: &str) -> AuthoringPathSegment {
    AuthoringPathSegment::Field(value.to_owned())
}

fn instance_field(value: &str) -> AuthoringInstancePathSegment {
    AuthoringInstancePathSegment::Field(value.to_owned())
}

#[test]
fn incomplete_values_retain_schema_paths_without_semantic_admission() {
    let text = "[workspace]\nvisibility = \"protected\"\nmissing =\n";
    let syntax = AuthoringSyntax::parse(text);
    let visibility = syntax
        .facts()
        .iter()
        .find(|fact| {
            fact.kind == SyntaxFactKind::Value
                && text.get(fact.range.start..fact.range.end) == Some("\"protected\"")
        })
        .expect("the tolerant parser retains the scalar");
    assert_eq!(
        visibility.path,
        vec![field("workspace"), field("visibility")]
    );

    let missing = syntax.position(text.len()).unwrap();
    assert_eq!(
        missing.fact.expect("incomplete value slot").path,
        vec![field("workspace"), field("missing")]
    );
}

#[test]
fn array_tables_and_array_values_have_distinct_item_paths() {
    let text = concat!(
        "[[workspace.projects]]\n",
        "id = \"demo\"\n",
        "tags = [\"one\", \"two\"]\n",
    );
    let syntax = AuthoringSyntax::parse(text);
    let table = syntax
        .facts()
        .iter()
        .find(|fact| fact.kind == SyntaxFactKind::ArrayTable)
        .expect("array-table header");
    assert_eq!(
        table.path,
        vec![
            field("workspace"),
            field("projects"),
            AuthoringPathSegment::ArrayItem
        ]
    );
    let tag_values = syntax
        .facts()
        .iter()
        .filter(|fact| {
            fact.kind == SyntaxFactKind::Value
                && matches!(
                    text.get(fact.range.start..fact.range.end),
                    Some("\"one\"") | Some("\"two\"")
                )
        })
        .collect::<Vec<_>>();
    assert_eq!(tag_values.len(), 2);
    for fact in tag_values {
        assert_eq!(
            fact.path,
            vec![
                field("workspace"),
                field("projects"),
                AuthoringPathSegment::ArrayItem,
                field("tags"),
                AuthoringPathSegment::ArrayItem,
            ]
        );
    }
}

#[test]
fn repeated_tables_and_scalar_arrays_retain_distinct_instance_paths() {
    let text = concat!(
        "[[workspace.projects]]\n",
        "id = \"one\"\n",
        "tags = [\"a\", \"b\"]\n",
        "[[workspace.projects]]\n",
        "id = \"two\"\n",
    );
    let syntax = AuthoringSyntax::parse(text);
    let values = syntax
        .facts()
        .iter()
        .filter(|fact| fact.kind == SyntaxFactKind::Value)
        .map(|fact| {
            (
                text.get(fact.range.start..fact.range.end)
                    .expect("fact range")
                    .to_owned(),
                fact.instance_path.clone(),
            )
        })
        .collect::<Vec<_>>();
    assert!(values.contains(&(
        "\"one\"".into(),
        vec![
            instance_field("workspace"),
            instance_field("projects"),
            AuthoringInstancePathSegment::Index(0),
            instance_field("id"),
        ],
    )));
    assert!(values.contains(&(
        "\"b\"".into(),
        vec![
            instance_field("workspace"),
            instance_field("projects"),
            AuthoringInstancePathSegment::Index(0),
            instance_field("tags"),
            AuthoringInstancePathSegment::Index(1),
        ],
    )));
    assert!(values.contains(&(
        "\"two\"".into(),
        vec![
            instance_field("workspace"),
            instance_field("projects"),
            AuthoringInstancePathSegment::Index(1),
            instance_field("id"),
        ],
    )));
}

#[test]
fn repeated_array_table_prefixes_are_reflected_in_nested_paths() {
    let text = "[[fruits]]\nname = \"apple\"\n[[fruits.varieties]]\nname = \"red\"\n";
    let syntax = AuthoringSyntax::parse(text);
    let paths = syntax
        .facts()
        .iter()
        .filter(|fact| fact.kind == SyntaxFactKind::ArrayTable)
        .map(|fact| fact.path.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        paths,
        vec![
            vec![field("fruits"), AuthoringPathSegment::ArrayItem],
            vec![
                field("fruits"),
                AuthoringPathSegment::ArrayItem,
                field("varieties"),
                AuthoringPathSegment::ArrayItem,
            ],
        ]
    );
}

#[test]
fn key_completion_context_tracks_only_fields_in_the_active_table() {
    let text = "root = true\n[workspace]\nname = \"demo\"\n\n";
    let syntax = AuthoringSyntax::parse(text);
    let root = syntax.position(0).unwrap();
    assert!(root.existing_fields.contains("root"));
    let workspace = syntax.position(text.len()).unwrap();
    assert_eq!(workspace.object_path, [field("workspace")]);
    assert!(workspace.existing_fields.contains("name"));
    assert!(!workspace.existing_fields.contains("root"));
}

#[test]
fn quoted_dotted_keys_remain_one_decoded_field() {
    let text = "[\"a.b\"]\n\"c.d\" = true\n";
    let syntax = AuthoringSyntax::parse(text);
    let value = syntax
        .facts()
        .iter()
        .find(|fact| fact.kind == SyntaxFactKind::Value)
        .expect("quoted-key value");
    assert_eq!(value.path, vec![field("a.b"), field("c.d")]);
    assert_eq!(
        value.instance_path,
        vec![instance_field("a.b"), instance_field("c.d")]
    );
}

#[test]
fn bom_prefixed_empty_quoted_key_is_distinct_from_recovery() {
    let text = "\u{feff}\"\" = true\r\n";
    let syntax = AuthoringSyntax::parse(text);

    let key = syntax
        .facts()
        .iter()
        .find(|fact| fact.kind == SyntaxFactKind::Key)
        .expect("empty quoted key remains an authored key");
    assert_eq!(text.get(key.range.start..key.range.end), Some("\"\""));
    assert_eq!(key.range.start, '\u{feff}'.len_utf8());
    assert_eq!(key.path, vec![field("")]);
    assert_eq!(key.instance_path, vec![instance_field("")]);

    let value = syntax
        .facts()
        .iter()
        .find(|fact| fact.kind == SyntaxFactKind::Value)
        .expect("empty quoted key owns its scalar value");
    assert_eq!(value.path, vec![field("")]);
    assert_eq!(value.instance_path, vec![instance_field("")]);

    let recovered = AuthoringSyntax::parse("= true\r\n");
    assert!(recovered.facts().iter().all(|fact| {
        fact.path != [field("")]
            && fact.instance_path != [instance_field("")]
            && fact.kind != SyntaxFactKind::Value
    }));
}

#[test]
fn incomplete_headers_do_not_rebase_following_recovery_facts() {
    let text = "[workspace]\nknown = true\n[incomplete\nafter = false\n";
    let syntax = AuthoringSyntax::parse(text);
    let after = syntax
        .facts()
        .iter()
        .find(|fact| {
            fact.kind == SyntaxFactKind::Value
                && text.get(fact.range.start..fact.range.end) == Some("false")
        })
        .expect("recovered value");
    assert_eq!(after.path, vec![field("workspace"), field("after")]);
}

#[test]
fn nested_array_tables_restart_child_indices_under_each_parent() {
    let text = concat!(
        "[[fruits]]\n",
        "[[fruits.varieties]]\n",
        "name = \"red\"\n",
        "[[fruits]]\n",
        "[[fruits.varieties]]\n",
        "name = \"green\"\n",
    );
    let syntax = AuthoringSyntax::parse(text);
    let values = syntax
        .facts()
        .iter()
        .filter(|fact| fact.kind == SyntaxFactKind::Value)
        .map(|fact| fact.instance_path.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        values,
        vec![
            vec![
                instance_field("fruits"),
                AuthoringInstancePathSegment::Index(0),
                instance_field("varieties"),
                AuthoringInstancePathSegment::Index(0),
                instance_field("name"),
            ],
            vec![
                instance_field("fruits"),
                AuthoringInstancePathSegment::Index(1),
                instance_field("varieties"),
                AuthoringInstancePathSegment::Index(0),
                instance_field("name"),
            ],
        ]
    );
}

#[test]
fn nested_containers_retain_every_schema_and_instance_coordinate() {
    let text = "items = [{ visibility = \"protected\" }]\nmatrix = [[1, 2], [3]]\n";
    let syntax = AuthoringSyntax::parse(text);
    let visibility = syntax
        .facts()
        .iter()
        .find(|fact| {
            fact.kind == SyntaxFactKind::Value
                && text.get(fact.range.start..fact.range.end) == Some("\"protected\"")
        })
        .expect("nested inline-table value");
    assert_eq!(
        visibility.path,
        vec![
            field("items"),
            AuthoringPathSegment::ArrayItem,
            field("visibility"),
        ]
    );
    assert_eq!(
        visibility.instance_path,
        vec![
            instance_field("items"),
            AuthoringInstancePathSegment::Index(0),
            instance_field("visibility"),
        ]
    );

    let matrix_values = syntax
        .facts()
        .iter()
        .filter(|fact| {
            fact.kind == SyntaxFactKind::Value
                && matches!(
                    text.get(fact.range.start..fact.range.end),
                    Some("1" | "2" | "3")
                )
        })
        .map(|fact| fact.instance_path.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        matrix_values,
        vec![
            vec![
                instance_field("matrix"),
                AuthoringInstancePathSegment::Index(0),
                AuthoringInstancePathSegment::Index(0),
            ],
            vec![
                instance_field("matrix"),
                AuthoringInstancePathSegment::Index(0),
                AuthoringInstancePathSegment::Index(1),
            ],
            vec![
                instance_field("matrix"),
                AuthoringInstancePathSegment::Index(1),
                AuthoringInstancePathSegment::Index(0),
            ],
        ]
    );
}

#[test]
fn parser_recovery_never_fabricates_exact_key_paths() {
    for text in ["a. = true\n", "\"bad\\q\" = true\n"] {
        let syntax = AuthoringSyntax::parse(text);
        assert!(
            syntax
                .facts()
                .iter()
                .all(|fact| fact.kind != SyntaxFactKind::Value),
            "invalid key produced an exact value fact for {text:?}"
        );
    }

    let recovered = AuthoringSyntax::parse("[a.]\nvalue = true\n");
    assert!(recovered
        .facts()
        .iter()
        .all(|fact| { fact.instance_path != vec![instance_field("a"), instance_field("value")] }));
    assert!(recovered.facts().iter().any(|fact| {
        fact.kind == SyntaxFactKind::Value && fact.instance_path == vec![instance_field("value")]
    }));
}

#[test]
fn syntax_fact_ceiling_marks_the_projection_incomplete_without_unbounded_retention() {
    let text = (0..MAX_AUTHORING_SYNTAX_FACTS)
        .map(|index| format!("key_{index} = {index}\n"))
        .collect::<String>();
    let syntax = AuthoringSyntax::parse(&text);

    assert!(!syntax.is_complete());
    assert!(syntax.facts().len() <= MAX_AUTHORING_SYNTAX_FACTS);
}

#[test]
fn incomplete_slots_end_before_later_recovery_regions() {
    let text = "missing =\n[incomplete\n";
    let syntax = AuthoringSyntax::parse(text);
    let slot = syntax
        .facts()
        .iter()
        .find(|fact| fact.kind == SyntaxFactKind::ValueSlot)
        .expect("missing-value slot");
    assert_eq!(slot.path, vec![field("missing")]);
    assert!(slot.range.end <= text.find("[incomplete").expect("header"));
    assert!(syntax.position(text.len()).unwrap().fact.is_none());
}

#[test]
fn missing_inline_value_never_relabels_the_following_key_as_its_value() {
    let text = "x = { a = , b = true }\n";
    let syntax = AuthoringSyntax::parse(text);
    assert!(
        syntax.facts().iter().any(|fact| {
            fact.kind == SyntaxFactKind::ValueSlot && fact.path == [field("x"), field("a")]
        }),
        "facts: {:?}",
        syntax.facts()
    );
    assert!(syntax.facts().iter().all(|fact| {
        fact.kind != SyntaxFactKind::Value
            || text.get(fact.range.start..fact.range.end) != Some("b")
    }));
}

#[test]
fn recursion_and_path_depth_share_the_public_authoring_ceiling() {
    let text = format!(
        "value = {}0{}\n",
        "[".repeat(AUTHORING_PATH_SEGMENT_LIMIT + 2),
        "]".repeat(AUTHORING_PATH_SEGMENT_LIMIT + 2)
    );
    let syntax = AuthoringSyntax::parse(&text);
    assert!(syntax.facts().iter().all(|fact| {
        fact.path.len() <= AUTHORING_PATH_SEGMENT_LIMIT
            && fact.instance_path.len() <= AUTHORING_PATH_SEGMENT_LIMIT
    }));
}

#[test]
fn header_syntax_preserves_original_offsets_and_excludes_markdown() {
    for newline in ["\n", "\r\n"] {
        let text = format!("+++{newline}description = 'λ'{newline}[guide]{newline}exposure = 'internal'{newline}+++{newline}[role]{newline}exposure = 'exported'{newline}");
        let region = pale_sdk::StandaloneAuthoringRole::PackagePage
            .declaration_range(&text)
            .unwrap();
        let syntax = AuthoringSyntax::parse_region(&text, region.clone()).unwrap();
        let fact = syntax
            .facts()
            .iter()
            .find(|fact| text.get(fact.range.start..fact.range.end) == Some("'internal'"))
            .unwrap();
        assert_eq!(fact.path, vec![field("guide"), field("exposure")]);
        assert!(syntax
            .facts()
            .iter()
            .all(|fact| fact.range.start >= region.start && fact.range.end <= region.end));
        assert!(syntax.position(0).is_none());
        assert!(syntax.position(region.end).is_none());
        assert!(syntax.position(text.find("exported").unwrap()).is_none());
        assert!(syntax.position(text.len()).is_none());
    }
}

#[test]
fn an_empty_closed_header_excludes_its_closing_delimiter() {
    let text = "+++\n+++\n# Body\n";
    let range = pale_sdk::StandaloneAuthoringRole::PackagePage
        .declaration_range(text)
        .unwrap();
    assert_eq!(range, 4..4);
    assert!(AuthoringSyntax::parse_region(text, range)
        .unwrap()
        .position(4)
        .is_none());
    let unfinished = "+++\n";
    let range = pale_sdk::StandaloneAuthoringRole::PackagePage
        .declaration_range(unfinished)
        .unwrap();
    assert!(AuthoringSyntax::parse_region(unfinished, range)
        .unwrap()
        .position(4)
        .is_some());
}
