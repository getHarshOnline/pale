//! Deterministic modeled text-custody lower bounds for the editor overlay.

use super::*;

/// Test-report marker; this checksum is evidence framing, not content identity.
const COMPLETION_MARKER: &str = "#pale_lsp_representation_report_complete_v1";
/// Golden digest of the independently witnessed deterministic row vector.
const EXPECTED_REPORT_DIGEST: &str = "067ea68ea67525d8";

#[derive(Debug, Clone, Copy)]
struct CopyRow {
    operation: &'static str,
    logical_touched_bytes: usize,
    reference_copied_bytes_lower_bound: usize,
    current_copied_bytes_lower_bound: usize,
    canonical_bytes: usize,
    effect_bytes: usize,
    transport_payload_bytes_lower_bound: usize,
}

impl CopyRow {
    fn render(self, corpus: &str) -> String {
        format!(
            "{corpus}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            self.operation,
            self.logical_touched_bytes,
            self.reference_copied_bytes_lower_bound,
            self.current_copied_bytes_lower_bound,
            self.canonical_bytes,
            self.effect_bytes,
            self.transport_payload_bytes_lower_bound,
        )
    }
}

fn fnv1a64(bytes: &[u8]) -> String {
    let digest = bytes
        .iter()
        .fold(0xcbf2_9ce4_8422_2325_u64, |digest, byte| {
            (digest ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
        });
    format!("{digest:016x}")
}

fn complete_report(corpus: &str, rows: &[CopyRow]) -> String {
    let mut payload = concat!(
        "corpus\toperation\tlogical_touched_bytes\t",
        "reference_copied_bytes_lower_bound\tcurrent_copied_bytes_lower_bound\t",
        "canonical_bytes\teffect_bytes\ttransport_payload_bytes_lower_bound\n",
    )
    .to_owned();
    for row in rows {
        payload.push_str(&row.render(corpus));
        payload.push('\n');
    }
    let digest = fnv1a64(payload.as_bytes());
    format!(
        "{payload}{COMPLETION_MARKER}\t{}\tfnv1a64\t{digest}\n",
        rows.len()
    )
}

#[test]
fn snapshot_and_change_copy_model_matches_owned_string_semantics_and_its_golden_vector() {
    let initial = "a".repeat(4_096);
    let corpus = format!("lsp-document-cow-v1:{}", fnv1a64(initial.as_bytes()));
    let document_uri = uri("file:///repo/pale-workspace.toml");
    let mut store = DocumentStore::new(
        &uri("file:///repo"),
        vec![baseline(
            "file:///repo/pale-workspace.toml",
            initial.as_str(),
        )],
    )
    .unwrap();
    store
        .open(
            document_uri.clone(),
            "pale-toml".to_owned(),
            1,
            initial.clone(),
        )
        .unwrap();

    let frozen = store.snapshot();
    assert_eq!(snapshot_bytes(&frozen), initial.len());

    let mut reference = initial.clone();
    let incremental = "β";
    reference.replace_range(2_048..2_052, incremental);
    store
        .change(
            &document_uri,
            2,
            vec![TextDocumentContentChangeEvent {
                range: Some(Range::new(Position::new(0, 2_048), Position::new(0, 2_052))),
                range_length: Some(4),
                text: incremental.to_owned(),
            }],
        )
        .unwrap();
    let incremental_metrics = store.last_change_metrics;
    assert_eq!(
        incremental_metrics,
        TextMutationMetrics {
            logical_bytes_touched: 4 + incremental.len(),
            replacement_bytes_copied: incremental.len(),
        }
    );
    assert_eq!(store.snapshot().documents()[0].text(), reference);
    assert_eq!(frozen.documents()[0].text(), initial);

    let before_replacement = reference.len();
    let replacement = "final\n".repeat(64);
    store
        .change(
            &document_uri,
            3,
            vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: replacement.clone(),
            }],
        )
        .unwrap();
    let replacement_metrics = store.last_change_metrics;
    reference = replacement.clone();
    assert_eq!(store.snapshot().documents()[0].text(), reference);

    let rows = [
        CopyRow {
            operation: "snapshot_text_share",
            logical_touched_bytes: 0,
            reference_copied_bytes_lower_bound: initial.len(),
            current_copied_bytes_lower_bound: 0,
            canonical_bytes: 0,
            effect_bytes: 0,
            transport_payload_bytes_lower_bound: 0,
        },
        CopyRow {
            operation: "incremental_atomic_edit",
            logical_touched_bytes: 4 + incremental.len(),
            reference_copied_bytes_lower_bound: initial.len(),
            current_copied_bytes_lower_bound: incremental_metrics.replacement_bytes_copied,
            canonical_bytes: 0,
            effect_bytes: 0,
            transport_payload_bytes_lower_bound: incremental.len(),
        },
        CopyRow {
            operation: "full_owned_replacement",
            logical_touched_bytes: before_replacement + replacement.len(),
            reference_copied_bytes_lower_bound: before_replacement,
            current_copied_bytes_lower_bound: replacement_metrics.replacement_bytes_copied,
            canonical_bytes: 0,
            effect_bytes: 0,
            transport_payload_bytes_lower_bound: replacement.len(),
        },
    ];
    let report = complete_report(&corpus, &rows);
    let lines = report.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), rows.len() + 2);
    let terminal = lines.last().unwrap().split('\t').collect::<Vec<_>>();
    assert_eq!(terminal[0], COMPLETION_MARKER);
    assert_eq!(terminal[1], rows.len().to_string());
    let mut payload = lines[..lines.len() - 1].join("\n");
    payload.push('\n');
    assert_eq!(terminal[2], "fnv1a64");
    assert_eq!(terminal[3], EXPECTED_REPORT_DIGEST);
}
