//! Deterministic modeled copy lower bounds for body composition.

use super::*;

const COMPLETION_MARKER: &str = "#pale_markdown_representation_report_complete_v1";
/// Golden digest of the independently witnessed deterministic row vector.
const EXPECTED_REPORT_DIGEST: &str =
    "41afc68edb9ee988bcb07265e0a82466a2bc39531cf167b7b1bd5b26e3b98871";

struct Case {
    name: &'static str,
    source: &'static str,
    levels: usize,
    logical_touched_bytes: usize,
}

#[test]
fn direct_append_matches_the_owned_reference_and_its_static_copy_golden() {
    let cases = [
        Case {
            name: "lf_title_and_body",
            source: "# Root\n\n## Procedure\n\nExact bytes.\n",
            levels: 2,
            logical_touched_bytes: 6,
        },
        Case {
            name: "crlf_unicode_and_fence",
            source: "# Root\r\n\r\n> ## Quoted λ\r\n\r\n```md\r\n### literal\r\n```\r\n",
            levels: 1,
            logical_touched_bytes: 5,
        },
        Case {
            name: "untitled_saturation",
            source: "## No title\n\n###### Saturated\n",
            levels: usize::MAX,
            logical_touched_bytes: 8,
        },
    ];
    let corpus_bytes = cases
        .iter()
        .flat_map(|case| case.source.as_bytes())
        .copied()
        .collect::<Vec<_>>();
    let corpus = format!(
        "markdown-body-composition-v1:{}",
        pale_core::sha256_hex(&corpus_bytes)
    );
    let mut payload = concat!(
        "corpus\toperation\tlogical_touched_bytes\t",
        "reference_copied_bytes_lower_bound\tcurrent_copied_bytes_lower_bound\t",
        "canonical_bytes\teffect_bytes\ttransport_payload_bytes_lower_bound\n",
    )
    .to_owned();

    for case in &cases {
        let projection = project(case.source);
        let body = &case.source[projection.body_range()];
        let expected = project(body).shift_headings(body, case.levels);
        let actual = shifted_body(case.source, case.levels);
        assert_eq!(actual, expected);

        let reference_copy_floor = body.len() + expected.len();
        let current_copy_floor = actual.len();
        payload.push_str(&format!(
            "{corpus}\t{}\t{}\t{reference_copy_floor}\t{current_copy_floor}\t",
            case.name, case.logical_touched_bytes,
        ));
        payload.push_str(&format!("{}\t0\t0\n", actual.len()));
    }

    let digest = pale_core::sha256_hex(payload.as_bytes());
    let report = format!(
        "{payload}{COMPLETION_MARKER}\t{}\tsha256\t{digest}\n",
        cases.len()
    );
    let lines = report.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), cases.len() + 2);
    let terminal = lines.last().unwrap().split('\t').collect::<Vec<_>>();
    assert_eq!(terminal[0], COMPLETION_MARKER);
    assert_eq!(terminal[1], cases.len().to_string());
    assert_eq!(terminal[2], "sha256");
    assert_eq!(terminal[3], EXPECTED_REPORT_DIGEST);
}
