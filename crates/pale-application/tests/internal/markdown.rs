//! Markdown adapter proofs for structural recognition and source-byte surgery.

use std::sync::Arc;

use pale_core::records::{PackageIdV1, PackageResourceIdV1, PackageResourceRefV1};

use super::{project, MarkdownProjection};
use crate::packages::{seal_texts, TextRecipeBuilder};

#[test]
fn paragraph_formatting_preserves_structured_content_and_source_lines() {
    let text = "+++\ndescription = 'fixture'\n[guide]\n+++\n\n# Method\n\nA wrapped\nparagraph.\n\n- Wrapped list\n  continuation.\n\n> Wrapped quote\n> continuation.\n\n| First | Second |\n| --- | --- |\n| One | Two |\n\n```text\nWrapped code\nstays here.\n```\n\n    Indented code\n    stays here.\n\nA deliberate break.  \nThen another line.\n\nA `multiline\ncode span` stays intact.\n";
    for input in [text.to_owned(), text.replace('\n', "\r\n")] {
        assert_eq!(super::wrapped_paragraphs(&input), vec![8, 11, 14]);
    }
    assert!(super::wrapped_paragraphs(
        "# Formula\n\n$$\na = b + c,\\qquad\nd = e + f.\n$$\n\nA paragraph with $x+y$.\n"
    )
    .is_empty());
    assert!(super::wrapped_paragraphs("---\ntitle: Example\ndescription: |\n  Wrapped metadata\n  stays here.\n---\n\n# Title\n\nOne line.\n").is_empty());
}

#[test]
fn initial_indented_examples_keep_their_syntax_and_source_coordinates() {
    for prefix in ["", "\r\n \t\r\n"] {
        for indent in ["    ", "\t"] {
            let input = format!("{prefix}{indent}[example](missing.md) `example.policy:missing`\r\n\r\n[live](actual.md)\r\n");
            let projection = project(&input);
            assert!(projection.inline_code().is_empty());
            assert_eq!(projection.links().len(), 1);
            let live = &projection.links()[0];
            assert_eq!(live.destination, "actual.md");
            assert_eq!(live.line, prefix.lines().count() + 3);
            assert_eq!(&input[live.range.clone().unwrap()], "actual.md");
            assert!(input[projection.body_range()].starts_with(indent));
        }
    }
    let code_title = project("    # Example\n");
    assert!(!code_title.has_fragment("example"));
    assert!(code_title.title("    # Example\n").is_none());
    assert_eq!(split_title("   # Root\n\nBody\n"), Some(("Root", "Body")));
    assert_eq!(
        split_title("# Root\n\n    [example](missing.md)\n"),
        Some(("Root", "    [example](missing.md)"))
    );
}

#[test]
fn closing_questions_leave_examples_and_later_host_instructions_intact() {
    let questions = "## Before handing off\r\n\r\n- [ ] Is α correct?\r\n- [ ] Did β run?\r\n- [ ] What remains?";
    let text = format!("```md\r\n{questions}\r\n```\r\n\r\n{questions}\r\n\r\nHost permission remains in force.\r\n");
    let spans = super::closing_questions(&text);
    assert_eq!(spans.len(), 1);
    assert_eq!(&text[spans[0].range.clone()], questions);
    assert_eq!(spans[0].count, 3);
    assert!(text[spans[0].range.end..].contains("Host permission remains in force."));
    assert_eq!(
        super::closing_questions("## Before handing off\n\n- [ ] Only one\n")[0].count,
        1
    );
}

#[test]
fn heading_fragments_follow_rendered_text_and_duplicate_order() {
    let input = "# Root\n\n## Read *this* `API`\n\n## Read *this* `API`\n\nCafé & tea\n----------\n\n```md\n## Missing\n```\n";
    let projection = project(input);
    for fragment in [
        "",
        "root",
        "read-this-api",
        "read-this-api-1",
        "café--tea",
        "caf%C3%A9--tea",
    ] {
        assert!(projection.has_fragment(fragment), "{fragment}");
    }
    for fragment in ["missing", "read-this-api-2", "Root", "caf%C3", "%GG", "%"] {
        assert!(!projection.has_fragment(fragment), "{fragment}");
    }
}

#[test]
fn explicit_anchors_are_targets_but_examples_and_comments_are_not() {
    let input = "# Root\n\n<a id=\"section\"></a>\n\nSee <a id=\"inline\"></a> this.\n\n`<a id=\"code\"></a>`\n\n<!--\n<a id=\"comment\"></a>\n-->\n\n```html\n<a id=\"example\"></a>\n```\n";
    let projection = project(input);
    assert!(projection.has_fragment("section"));
    assert!(projection.has_fragment("inline"));
    for fragment in ["code", "comment", "example"] {
        assert!(!projection.has_fragment(fragment), "{fragment}");
    }
}

#[test]
fn resource_destination_spans_follow_real_links_not_code_examples() {
    let input = "# Root\r\n\r\n> λ [read](<nested/child.md#check> \"title\") and `[code](missing.md)`.\r\n\r\n```md\r\n[example](missing.md)\r\n```\r\n";
    let projection = project(input);
    assert_eq!(projection.links().len(), 1);
    let link = &projection.links()[0];
    assert_eq!(link.destination, "nested/child.md#check");
    assert_eq!(&input[link.range.clone().unwrap()], "nested/child.md#check");

    let escaped = "[nested](child\\(copy\\).md)";
    let link = project(escaped).links()[0].clone();
    assert_eq!(link.destination, "child(copy).md");
    assert_eq!(&escaped[link.range.unwrap()], "child\\(copy\\).md");
    assert!(project("[read][named]\n\n[named]: child.md").links()[0]
        .range
        .is_none());
    for source in [
        "[**read**](child.md \"title ](wrong.md)\")",
        "[label \\](literal](child.md)",
        "[](child.md)",
    ] {
        let projection = project(source);
        let link = &projection.links()[0];
        assert_eq!(link.destination, "child.md", "{source}");
        assert_eq!(&source[link.range.clone().unwrap()], "child.md", "{source}");
    }
}

#[test]
fn rewritten_destinations_preserve_url_meaning_in_bare_and_angle_syntax() {
    for source in [
        "[read](child.md#part\\))",
        "[read](child.md#a&amp;amp;)",
        "[read](<child.md#a&lt;b&gt;>)",
        "[read](<child.md#a b>)",
        "[read](child.md#λ)",
    ] {
        let projection = project(source);
        let link = &projection.links()[0];
        let range = link.range.clone().unwrap();
        let mut rewritten = source.to_owned();
        rewritten.replace_range(range, &super::escape_link_destination(&link.destination));
        assert_eq!(
            project(&rewritten).links()[0].destination,
            link.destination,
            "{source}"
        );
    }
}

#[test]
fn linked_recipe_preserves_source_and_shifts_only_admitted_spans() {
    let input = "# Root\n\n## Choice\n\nλ [**read**](<child.md#check> \"More detail\") and `[code](child.md)`.\n";
    let owner = PackageResourceRefV1 {
        package_id: PackageIdV1::parse("markdown-test").unwrap(),
        resource_id: PackageResourceIdV1::parse("body").unwrap(),
    };
    let (_, text) = seal_texts([(owner.clone(), Arc::<str>::from(input))])
        .pop()
        .unwrap();
    let projection = project(input);
    let link = crate::packages::ResourceTextLink {
        range: projection.links()[0].link_range.clone().unwrap(),
        destination_range: {
            let destination = projection.links()[0].range.clone().unwrap();
            let start = projection.links()[0].link_range.as_ref().unwrap().start;
            destination.start - start..destination.end - start
        },
        reference: crate::packages::ResourceTextReference {
            title: "Child".into(),
            path: "child.md".into(),
            target: owner,
            fragment: Some("check".into()),
        },
    };
    let mut recipe = TextRecipeBuilder::default();
    projection.append_linked_recipe(&text, projection.body_range(), 1, &[link], &mut recipe);
    let recipe = recipe.finish();
    let original = recipe.materialize();
    assert_eq!(
        original.as_str(),
        "### Choice\n\nλ [**read**](<child.md#check> \"More detail\") and `[code](child.md)`."
    );
    let projected = recipe
        .materialize_links::<std::convert::Infallible>(|_, fragment| {
            Ok(Some(format!("../selected/SKILL.md#{}", fragment.unwrap())))
        })
        .unwrap();
    assert_eq!(
        projected.as_str(),
        "### Choice\n\nλ [**read**](<../selected/SKILL.md#check> \"More detail\") and `[code](child.md)`."
    );
    let unselected = recipe
        .materialize_links::<std::convert::Infallible>(|_, _| Ok(None))
        .unwrap();
    assert_eq!(
        unselected.as_str(),
        "### Choice\n\nλ Child, `markdown-test`, `child.md#check` and `[code](child.md)`."
    );
    assert_eq!(recipe.materialize(), original);
}

impl MarkdownProjection {
    /// Demote admitted ATX markers without reparsing or normalizing other bytes.
    pub(crate) fn shift_headings(&self, markdown: &str, levels: usize) -> String {
        if levels == 0 || markdown.is_empty() {
            return markdown.to_string();
        }
        let mut shifted = markdown.to_string();
        for heading in self.atx_headings.iter().rev() {
            let new_width = heading.width.saturating_add(levels).min(6);
            if heading.width != new_width {
                shifted.replace_range(
                    heading.start..heading.start + heading.width,
                    &"#".repeat(new_width),
                );
            }
        }
        shifted
    }
}

fn shift_headings(markdown: &str, levels: usize) -> String {
    project(markdown).shift_headings(markdown, levels)
}

fn split_title(markdown: &str) -> Option<(&str, &str)> {
    let projection = project(markdown);
    projection
        .title(markdown)
        .map(|title| (title, &markdown[projection.body_range()]))
}

fn shifted_body(markdown: &str, levels: usize) -> String {
    let owner = PackageResourceRefV1 {
        package_id: PackageIdV1::parse("markdown-test").unwrap(),
        resource_id: PackageResourceIdV1::parse("body").unwrap(),
    };
    let (_, text) = seal_texts([(owner, Arc::<str>::from(markdown))])
        .pop()
        .unwrap();
    let projection = project(markdown);
    let mut recipe = TextRecipeBuilder::default();
    projection.append_shifted_recipe(&text, levels, &mut recipe);
    Arc::unwrap_or_clone(recipe.finish().materialize())
}

#[test]
fn heading_shift_uses_commonmark_fence_structure_and_preserves_line_endings() {
    let input =
        "# Root\r\n\r\n````md\r\n# literal\r\n```\r\n## still literal\r\n````\r\n### Tail\r\n";

    assert_eq!(
        shift_headings(input, 2),
        "### Root\r\n\r\n````md\r\n# literal\r\n```\r\n## still literal\r\n````\r\n##### Tail\r\n"
    );
}

#[test]
fn heading_shift_changes_only_recognized_atx_openers() {
    let input = concat!(
        "####### not-a-heading\n",
        "    # indented code\n",
        "###heading\n",
        "Setext title\n",
        "============\n",
        "<!-- # html -->\n",
        "# heading ###\n",
    );

    assert_eq!(
        shift_headings(input, 1),
        concat!(
            "####### not-a-heading\n",
            "    # indented code\n",
            "###heading\n",
            "Setext title\n",
            "============\n",
            "<!-- # html -->\n",
            "## heading ###\n",
        )
    );
    assert_eq!(shift_headings("# bounded", usize::MAX), "###### bounded");
}

#[test]
fn heading_shift_handles_nested_and_unicode_source_positions() {
    let input = concat!(
        "> # Quoted\n",
        "\n",
        "- ## Listed\n",
        "\n",
        "> λ before\n",
        ">\n",
        "> ### Unicode sibling\n",
    );
    assert_eq!(
        shift_headings(input, 1),
        concat!(
            "> ## Quoted\n",
            "\n",
            "- ### Listed\n",
            "\n",
            "> λ before\n",
            ">\n",
            "> #### Unicode sibling\n",
        )
    );
}

#[test]
fn heading_shift_preserves_unclosed_fences_and_every_unselected_byte() {
    let input = "prefix\tvalue\n\n# Visible\n\n~~~rust\n## literal\nλ = '#';\n";
    let shifted = shift_headings(input, 1);
    assert_eq!(
        shifted,
        "prefix\tvalue\n\n## Visible\n\n~~~rust\n## literal\nλ = '#';\n"
    );
    assert_eq!(shift_headings(&shifted, 0), shifted);
}

#[test]
fn heading_shift_tracks_lone_carriage_returns_and_mixed_endings() {
    let input = "# One\r## Two\r\n### Three\n#### Four\r";
    assert_eq!(
        shift_headings(input, 1),
        "## One\r### Two\r\n#### Three\n##### Four\r"
    );
}

#[test]
fn heading_shift_is_deterministic_and_leaves_html_and_setext_bytes_untouched() {
    let input = concat!(
        "<section>\r\n",
        "# html-looking text\r\n",
        "</section>\r\n",
        "\r\n",
        "Setext heading\n",
        "--------------\n",
        "\n",
        "λ prefix\r",
        "## Authored\r",
    );
    let expected = concat!(
        "<section>\r\n",
        "# html-looking text\r\n",
        "</section>\r\n",
        "\r\n",
        "Setext heading\n",
        "--------------\n",
        "\n",
        "λ prefix\r",
        "### Authored\r",
    );

    let first = shift_headings(input, 1);
    assert_eq!(first, expected);
    assert_eq!(shift_headings(input, 1), first);
}

#[test]
fn title_split_requires_a_leading_top_level_atx_h1() {
    assert_eq!(split_title("# Root\n\nBody"), Some(("Root", "Body")));
    assert_eq!(
        split_title("\n\n# Root ###\r\n\r\nBody\n"),
        Some(("Root", "Body"))
    );
    assert_eq!(split_title("## Not root\nBody"), None);
    assert_eq!(split_title("Root\n====\nBody"), None);
    assert_eq!(split_title("> # Nested\nBody"), None);
    assert_eq!(split_title("#\tTab-separated\nBody"), None);
    assert_eq!(split_title("# Root\r\rBody"), Some(("Root", "Body")));
    assert_eq!(split_title("# Root\r\n\rBody"), Some(("Root", "Body")));
}

#[test]
fn referenced_body_recipe_is_observationally_equal_to_the_owned_projection() {
    let cases = [
        (
            "# Root\n\n## Procedure\n\nExact bytes.\n",
            2,
            "#### Procedure\n\nExact bytes.",
        ),
        (
            "# Root\r\n\r\n> ## Quoted λ\r\n\r\n```md\r\n### literal\r\n```\r\n",
            1,
            "> ### Quoted λ\r\n\r\n```md\r\n### literal\r\n```",
        ),
        (
            "## No title\n\n###### Saturated\n",
            usize::MAX,
            "###### No title\n\n###### Saturated",
        ),
        ("# Empty body\n", 1, ""),
    ];
    for (source, levels, expected) in cases {
        assert_eq!(shifted_body(source, levels), expected);
    }
}

#[path = "markdown/representation.rs"]
mod representation;
