//! Behavioral timing-custody and report tests; no elapsed-time thresholds.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::rc::Rc;

use super::resident_kernel_measurement::{
    parallelism_field, retained_observation, Measurement, Sampling, TimingScope,
};

struct DropWitness {
    events: Rc<RefCell<Vec<&'static str>>>,
    label: &'static str,
}

impl Drop for DropWitness {
    fn drop(&mut self) {
        self.events.borrow_mut().push(self.label);
    }
}

fn witness(events: &Rc<RefCell<Vec<&'static str>>>, label: &'static str) -> DropWitness {
    DropWitness {
        events: Rc::clone(events),
        label,
    }
}

fn columns(measurement: Measurement) -> BTreeMap<String, String> {
    let row = measurement.tsv();
    let header = Measurement::TSV_HEADER.split('\t').collect::<Vec<_>>();
    let values = row.split('\t').collect::<Vec<_>>();
    assert_eq!(header.len(), values.len());
    header
        .into_iter()
        .zip(values)
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect()
}

#[test]
fn retained_observation_keeps_prepared_owner_and_result_alive_after_elapsed_capture() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let (_, state, output) = retained_observation(witness(&events, "state"), |state| {
        assert_eq!(state.label, "state");
        events.borrow_mut().push("operation");
        witness(&events, "output")
    });
    assert_eq!(*events.borrow(), ["operation"]);
    drop(output);
    assert_eq!(*events.borrow(), ["operation", "output"]);
    drop(state);
    assert_eq!(*events.borrow(), ["operation", "output", "state"]);
}

#[test]
fn complete_call_drops_each_result_before_the_next_call() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut calls = 0;
    let measured = Measurement::sample(Sampling::Smoke, || {
        assert_eq!(events.borrow().len(), calls);
        calls += 1;
        witness(&events, "output")
    });
    assert_eq!(calls, 6, "one warmup and five observations");
    assert_eq!(events.borrow().len(), calls);
    assert_eq!(columns(measured)["timing_scope"], "call_and_result_drop");
}

#[test]
fn consumed_input_destruction_remains_inside_the_operation() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let measured = Measurement::sample_prepared(
        Sampling::Smoke,
        || witness(&events, "input"),
        |input| {
            events.borrow_mut().push("operation");
            drop(input);
            assert_eq!(events.borrow().last(), Some(&"input"));
            witness(&events, "output")
        },
    );
    assert_eq!(*events.borrow(), ["operation", "input", "output"].repeat(6));
    assert_eq!(
        columns(measured)["timing_scope"],
        "consumed_state_returned_result_retained"
    );
}

#[test]
fn retained_sampling_borrows_each_independent_prepared_state() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut preparations = 0;
    let measured = Measurement::sample_retained(
        Sampling::Smoke,
        || {
            preparations += 1;
            witness(&events, "state")
        },
        |state| {
            assert_eq!(state.label, "state");
            witness(&events, "output")
        },
    );
    assert_eq!(preparations, 6);
    assert_eq!(
        events
            .borrow()
            .iter()
            .filter(|&&event| event == "state")
            .count(),
        6
    );
    assert_eq!(
        events
            .borrow()
            .iter()
            .filter(|&&event| event == "output")
            .count(),
        6
    );
    let row = columns(measured);
    assert_eq!(row["batch_size"], "1");
    assert_eq!(row["timing_scope"], "prepared_state_and_result_retained");
}

#[test]
fn batch_average_statistics_keep_nearest_rank_and_smoke_evidence_distinct() {
    let smoke = columns(Measurement::summarize(
        vec![5, 1, 4, 2, 3],
        8,
        TimingScope::CompleteCall,
    ));
    assert_eq!(smoke["samples"], "5");
    assert_eq!(smoke["batch_size"], "8");
    assert_eq!(smoke["batch_mean_p50_ns"], "3");
    assert_eq!(smoke["batch_mean_p95_ns"], "5");
    assert_eq!(smoke["batch_mean_p99_ns"], "");
    assert_eq!(smoke["batch_mean_max_ns"], "5");
    assert_eq!(smoke["benchmark_calls_per_second"], "333333333");

    let full = columns(Measurement::summarize(
        (1..=201).rev().collect(),
        64,
        TimingScope::CompleteCall,
    ));
    assert_eq!(full["batch_mean_p50_ns"], "101");
    assert_eq!(full["batch_mean_p95_ns"], "191");
    assert_eq!(full["batch_mean_p99_ns"], "199");
    assert_eq!(full["batch_mean_max_ns"], "201");
    assert_eq!(full["benchmark_calls_per_second"], "9900990");
}

#[test]
fn full_prepared_schedule_still_collects_201_individual_observations() {
    let row = columns(Measurement::sample_prepared(
        Sampling::Characterization,
        || 7,
        |value| value + 1,
    ));
    assert_eq!(row["samples"], "201");
    assert_eq!(row["batch_size"], "1");
    assert!(!row["batch_mean_p99_ns"].is_empty());
}

#[test]
fn unavailable_parallelism_is_not_a_fabricated_worker_count() {
    assert_eq!(parallelism_field(Ok(NonZeroUsize::new(16).unwrap())), "16");
    assert_eq!(
        parallelism_field(Err(std::io::Error::other("unavailable"))),
        ""
    );
}
