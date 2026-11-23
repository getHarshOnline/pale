//! Behavioral laws for editor-benchmark measurement boundaries.

use std::cell::Cell;

use super::*;

#[test]
fn source_visible_copy_counts_come_from_the_admitted_edit() {
    let fixture = EditorFixture::new(support::SMALL_DOCUMENT_BYTES);
    let (incremental, replacement) = preflight_document(&fixture);
    assert_eq!(incremental.logical_bytes_touched, 2);
    assert_eq!(incremental.replacement_bytes_copied, 1);
    assert_eq!(replacement.logical_bytes_touched, fixture.initial.len() * 2);
    assert_eq!(replacement.replacement_bytes_copied, fixture.initial.len());
}

#[test]
fn disabled_memory_sample_never_runs_the_workload() {
    let prepared = Cell::new(0);
    let invoked = Cell::new(0);
    let evidence = measure_memory(
        || prepared.set(prepared.get() + 1),
        |()| invoked.set(invoked.get() + 1),
    );
    let expected = usize::from(cfg!(feature = "bench-memory"));
    assert_eq!(prepared.get(), expected);
    assert_eq!(invoked.get(), expected);
    assert_eq!(
        evidence.allocations.is_some(),
        cfg!(feature = "bench-memory")
    );
}

#[test]
fn prepared_sampling_adds_one_untimed_invocation_only_when_enabled() {
    let prepared = Cell::new(0);
    let invoked = Cell::new(0);
    let measurement = Measurement::sample_prepared(
        Configuration {
            smoke: true,
            samples: SMOKE_SAMPLES,
        },
        || prepared.set(prepared.get() + 1),
        |()| invoked.set(invoked.get() + 1),
    );
    let expected = 1 + SMOKE_SAMPLES + usize::from(cfg!(feature = "bench-memory"));
    assert_eq!(prepared.get(), expected);
    assert_eq!(invoked.get(), expected);
    assert_eq!(measurement.samples, SMOKE_SAMPLES);
    assert_eq!(measurement.p99_ns, None);
}

#[test]
fn prepared_sampling_runs_only_on_the_calling_thread() {
    let owner = std::thread::current().id();
    Measurement::sample_prepared(
        Configuration {
            smoke: true,
            samples: SMOKE_SAMPLES,
        },
        || assert_eq!(std::thread::current().id(), owner),
        |()| assert_eq!(std::thread::current().id(), owner),
    );
}

#[test]
fn unavailable_measurements_are_empty_not_zero() {
    let mut row = String::new();
    MemoryEvidence::default().append_tsv(&mut row);
    let fields = row.split('\t').skip(1).collect::<Vec<_>>();
    assert_eq!(fields.len(), 11);
    assert_eq!(fields[0], "unavailable_feature_disabled");
    assert!(fields[1..8].iter().all(|field| field.is_empty()));
    assert_eq!(fields[8], "unavailable_feature_platform_or_observation");
    assert_eq!(fields[9], "");
    assert_eq!(fields[10], "disabled");
}

#[cfg(feature = "bench-memory")]
#[test]
fn allocation_sample_excludes_setup_and_includes_prepared_input_drop() {
    let evidence = measure_memory(
        || black_box(vec![1_u8; 4096]),
        |prepared| {
            let result = black_box(vec![2_u8; 32]);
            drop(black_box(prepared));
            result
        },
    );
    let allocations = evidence.allocations.unwrap();
    assert_eq!(allocations.total_count, 1);
    assert_eq!(allocations.total_bytes, 32);
    assert_eq!(allocations.retained_count, -1);
    assert_eq!(allocations.retained_bytes, -4096);
}

#[cfg(feature = "bench-memory")]
#[test]
fn prepared_drop_before_allocation_keeps_peak_explicitly_relative() {
    let evidence = measure_memory(
        || black_box(vec![1_u8; 4096]),
        |prepared| {
            drop(black_box(prepared));
            black_box(vec![2_u8; 32])
        },
    );
    let allocations = evidence.allocations.unwrap();
    assert_eq!(allocations.total_bytes, 32);
    assert_eq!(allocations.peak_relative_count, 0);
    assert_eq!(allocations.peak_relative_bytes, 0);
    assert_eq!(allocations.retained_bytes, -4096);
}

#[cfg(feature = "bench-memory")]
#[test]
fn timing_explicitly_excludes_allocation_bookkeeping() {
    let observed = allocation_counter::measure(|| {
        without_allocation_counting(|| drop(black_box(vec![0_u8; 4096])));
    });
    assert_eq!(observed.count_total, 0);
    assert_eq!(observed.bytes_total, 0);
    assert_eq!(observed.bytes_current, 0);
}

#[cfg(feature = "bench-memory")]
#[test]
fn real_document_flattening_is_measured_without_fixture_allocation() {
    let fixture = EditorFixture::new(support::SMALL_DOCUMENT_BYTES);
    let store = opened_store(&fixture.initial);
    let snapshot = store.snapshot();
    let uri = root_uri();
    let document = snapshot.document(&uri).unwrap();
    let evidence = measure_memory(|| (), |()| document.bytes());
    let allocations = evidence.allocations.unwrap();
    assert!(allocations.total_bytes >= fixture.initial.len() as u64);
    assert_eq!(allocations.retained_bytes, 0);
    assert_eq!(allocations.retained_count, 0);
}

#[cfg(all(
    feature = "bench-memory",
    any(target_os = "macos", target_os = "linux")
))]
#[test]
fn rss_conversion_preserves_units_and_unavailable_readings() {
    assert_eq!(rss_bytes(-1), None);
    assert_eq!(rss_bytes(0), Some(0));
    assert_eq!(
        rss_bytes(1),
        Some(if cfg!(target_os = "macos") { 1 } else { 1024 })
    );
    #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
    assert_eq!(rss_bytes(std::ffi::c_long::MAX), None);
}
