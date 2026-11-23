//! Deterministic modeled copy bounds at the final filesystem staging boundary.

use std::borrow::Cow;

use super::*;
use crate::reconcile::Mutation;

/// Terminal marker proving the emitted representation report was not truncated.
const COMPLETION_MARKER: &str = "#pale_adapter_fs_representation_report_complete_v1";

struct CopyRow<'a> {
    operation: &'a str,
    logical_touched_bytes: usize,
    reference_copied_bytes_lower_bound: usize,
    current_copied_bytes_lower_bound: usize,
    canonical_bytes: usize,
    effect_bytes: usize,
}

impl CopyRow<'_> {
    fn append_to(&self, corpus: &str, payload: &mut String) {
        payload.push_str(&format!(
            "{corpus}\t{}\t{}\t{}\t{}\t{}\t{}\t0\n",
            self.operation,
            self.logical_touched_bytes,
            self.reference_copied_bytes_lower_bound,
            self.current_copied_bytes_lower_bound,
            self.canonical_bytes,
            self.effect_bytes,
        ));
    }
}

#[test]
fn staging_copy_rows_distinguish_borrowable_payloads_from_required_file_joins() {
    let tmp = tempfile::tempdir().unwrap();
    let (target, _) = roots(&tmp);
    let root = crate::confine::ConfinedRoot::open(&target).unwrap();

    let whole_path = "generated.txt";
    let whole_content = "complete generated surface\n";
    let whole_state = state(
        vec![surface(
            whole_path,
            "artifact",
            SurfaceOwnership::WholeFile,
            whole_content,
        )],
        'a',
    );
    let whole_plan = transition_plan(PlanKind::Apply, None, Some(whole_state.clone()));
    let whole_payloads = payload(whole_path, whole_content);
    let whole_staged = crate::reconcile::surface::stage(
        &root,
        &whole_plan.operations[0],
        None,
        Some(&whole_state.surfaces[0]),
        None,
        &whole_payloads,
    )
    .unwrap();
    match &whole_staged.mutation {
        Mutation::Write(Cow::Borrowed(bytes)) => {
            assert_eq!(*bytes, whole_content.as_bytes());
            assert_eq!(
                bytes.as_ptr(),
                whole_payloads[whole_path].as_bytes().as_ptr()
            );
        }
        _ => panic!("whole-file staging must borrow its admitted generated payload"),
    }

    let managed_path = "AGENTS.md";
    let existing = "user-owned prefix\n";
    let managed_block = format!(
        "{}\nPale projection\n{}",
        pale_core::MANAGED_BEGIN,
        pale_core::MANAGED_END
    );
    fs::write(target.join(managed_path), existing).unwrap();
    let managed_state = state(
        vec![surface(
            managed_path,
            "activation",
            SurfaceOwnership::ManagedBlock,
            &managed_block,
        )],
        'b',
    );
    let managed_plan = transition_plan(PlanKind::Apply, None, Some(managed_state.clone()));
    let managed_payloads = payload(managed_path, managed_block.clone());
    let managed_staged = crate::reconcile::surface::stage(
        &root,
        &managed_plan.operations[0],
        None,
        Some(&managed_state.surfaces[0]),
        None,
        &managed_payloads,
    )
    .unwrap();
    let joined = match &managed_staged.mutation {
        Mutation::Write(Cow::Owned(bytes)) => bytes,
        _ => panic!("managed-block staging must own the complete joined file"),
    };
    assert_eq!(joined, format!("{existing}{managed_block}").as_bytes());

    let whole_canonical = pale_core::canon::to_canonical_string(&whole_plan)
        .unwrap()
        .len();
    let managed_canonical = pale_core::canon::to_canonical_string(&managed_plan)
        .unwrap()
        .len();
    let corpus_bytes = [
        whole_content.as_bytes(),
        existing.as_bytes(),
        managed_block.as_bytes(),
    ]
    .concat();
    let corpus = format!(
        "filesystem-surface-staging-v1:{}",
        pale_core::sha256_hex(&corpus_bytes)
    );
    let rows = [
        CopyRow {
            operation: "whole_file_stage",
            logical_touched_bytes: whole_content.len(),
            reference_copied_bytes_lower_bound: whole_content.len(),
            current_copied_bytes_lower_bound: 0,
            canonical_bytes: whole_canonical,
            effect_bytes: whole_content.len(),
        },
        CopyRow {
            operation: "managed_block_join",
            logical_touched_bytes: managed_block.len(),
            reference_copied_bytes_lower_bound: joined.len(),
            current_copied_bytes_lower_bound: joined.len(),
            canonical_bytes: managed_canonical,
            effect_bytes: joined.len(),
        },
    ];
    let mut report_payload = concat!(
        "corpus\toperation\tlogical_touched_bytes\t",
        "reference_copied_bytes_lower_bound\tcurrent_copied_bytes_lower_bound\t",
        "canonical_bytes\teffect_bytes\ttransport_payload_bytes_lower_bound\n",
    )
    .to_owned();
    for row in &rows {
        row.append_to(&corpus, &mut report_payload);
    }
    let report = format!("{report_payload}{COMPLETION_MARKER}\t{}\n", rows.len());
    let lines = report.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), rows.len() + 2);
    let terminal = lines.last().unwrap().split('\t').collect::<Vec<_>>();
    assert_eq!(terminal[0], COMPLETION_MARKER);
    assert_eq!(terminal[1], rows.len().to_string());
    assert_eq!(terminal.len(), 2);
}
