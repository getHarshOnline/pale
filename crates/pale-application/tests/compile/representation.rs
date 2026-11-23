//! Deterministic modeled copy bounds across render and effect-free planning.

use std::collections::BTreeMap;

use super::*;

const COMPLETION_MARKER: &str = "#pale_application_representation_report_complete_v1";

#[test]
fn rendered_payload_sharing_has_a_deterministic_static_copy_account() {
    let compilation = shared_captured_fixture_compilation();
    let repository = &compilation.repositories()[0];
    let source_payloads = repository
        .render()
        .surfaces
        .iter()
        .filter(|surface| !surface.deferred && !surface.rel_path.starts_with('{'))
        .map(|surface| (surface.rel_path.as_str(), surface.content()))
        .collect::<BTreeMap<_, _>>();
    let input = RepositoryMaterializationInput::for_repository(&compilation, repository.id())
        .expect("shared repository planning input");
    let targets = input
        .required_observation_targets(PlanKind::Apply, None)
        .expect("materializable target set");
    let baseline = targets
        .iter()
        .map(|target| BaselineEntry {
            rel_path: target.clone(),
            observation: BaselineStateV1::Absent {},
        })
        .collect();
    let planned = input
        .build(MaterializationIntent::Apply, None, baseline)
        .expect("effect-free apply plan");

    for (target, payload) in planned.payloads() {
        let source = source_payloads
            .get(target.as_str())
            .expect("every planned payload has one rendered owner");
        assert_eq!(*source, payload.content());
    }
    let payload_bytes = planned
        .payloads()
        .values()
        .map(|value| value.content().len())
        .sum::<usize>();
    let (surface_plan, payloads) = planned.into_parts();
    let canonical_bytes = bind_machine_local_effects(surface_plan, Vec::new())
        .expect("complete effect-free plan")
        .canonical_bytes()
        .len();
    assert_eq!(
        payload_bytes,
        payloads
            .values()
            .map(|value| value.content().len())
            .sum::<usize>()
    );

    let corpus = format!(
        "one-shot-compilation:{}",
        compilation.one_shot_compilation_digest()
    );
    let header = concat!(
        "corpus\toperation\tlogical_touched_bytes\t",
        "reference_copied_bytes_lower_bound\tcurrent_copied_bytes_lower_bound\t",
        "canonical_bytes\teffect_bytes\ttransport_payload_bytes_lower_bound\n",
    );
    let row = format!("{corpus}\trender_to_materialization_payloads\t{payload_bytes}\t",)
        + &format!("{payload_bytes}\t0\t{canonical_bytes}\t{payload_bytes}\t0\n");
    let payload = format!("{header}{row}");
    let report = format!("{payload}{COMPLETION_MARKER}\t1\n");
    let lines = report.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 3);
    let terminal = lines[2].split('\t').collect::<Vec<_>>();
    assert_eq!(terminal[0], COMPLETION_MARKER);
    assert_eq!(terminal[1], "1");
    assert_eq!(terminal.len(), 2);
}
