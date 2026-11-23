//! Checks deterministic cycle rejection across individual and aggregate composition graphs.

use super::{Adjacency, CompositionGraphs};
use pale_core::records::{CompositionGraphKindV1, ParticipantRefV1};

fn application(id: usize) -> ParticipantRefV1 {
    ParticipantRefV1::Application {
        application_id: format!("application-{id}"),
    }
}

#[test]
fn iterative_cycle_check_handles_deep_acyclic_chains_without_stack_growth() {
    let mut adjacency = Adjacency::default();
    for id in 0..20_000 {
        assert!(adjacency.insert(format!("edge-{id}"), application(id), application(id + 1)));
    }
    assert!(!adjacency.has_cycle());
}

#[test]
fn aggregate_view_detects_cycles_crossing_operational_graph_kinds() {
    let mut graphs = CompositionGraphs::default();
    graphs.insert(
        CompositionGraphKindV1::WorkDelegation,
        "delegate".into(),
        application(0),
        application(1),
    );
    graphs.insert(
        CompositionGraphKindV1::NativeApplicationCall,
        "invoke".into(),
        application(1),
        application(0),
    );
    assert!(graphs
        .reject_operational_cycles()
        .is_err_and(|error| error.to_string().contains("runtime/effect graph is cyclic")));
}
