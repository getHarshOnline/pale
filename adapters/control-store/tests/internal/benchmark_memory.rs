//! Laws for benchmark-only memory scopes and static copy lower bounds.

use pale_ports::AuthorityRecoveryReadLimits;

use super::super::input::{BenchmarkCase, Scale, WorkloadClass};
use super::super::workload::CaseMeasurements;
use super::{allocation_scope, explicit_copy_lower_bound, measure_case, rss_source_unit};

fn case(workload_class: WorkloadClass) -> BenchmarkCase {
    BenchmarkCase {
        pragma_candidate: None,
        workload_class,
        scale: Scale::ActualInput,
        records: 10,
        payload_bytes: 100,
        checkpoint_bytes: 1_000,
        limits: AuthorityRecoveryReadLimits::new(100, 1_000, 1, 3).unwrap(),
    }
}

#[test]
fn explicit_copy_lower_bound_keeps_adapter_and_native_sqlite_scopes_distinct() {
    assert_eq!(
        explicit_copy_lower_bound(&case(WorkloadClass::ExactAdapterMechanics)),
        4 * 10 * 100 + 5 * 1_000 + 3 * 100
    );
    assert_eq!(
        explicit_copy_lower_bound(&case(WorkloadClass::SqliteCharacterization)),
        2 * 100
    );
}

#[test]
fn instrumentation_names_its_platform_and_thread_scopes() {
    if cfg!(feature = "bench-memory") {
        assert!(allocation_scope().contains("current_thread"));
    } else {
        assert_eq!(allocation_scope(), "unavailable_feature_disabled");
    }
    assert!(matches!(
        rss_source_unit(),
        "bytes" | "kibibytes" | "unavailable"
    ));
}

#[test]
fn whole_case_runs_once_and_preserves_its_retained_results() {
    let mut invocations = 0;
    let observed = measure_case(&case(WorkloadClass::ExactAdapterMechanics), || {
        invocations += 1;
        CaseMeasurements {
            independent_runs: 2,
            recovery_tail: vec![11, 13],
            ..CaseMeasurements::default()
        }
    });
    assert_eq!(invocations, 1);
    assert_eq!(observed.independent_runs, 2);
    assert_eq!(observed.recovery_tail, [11, 13]);
    #[cfg(feature = "bench-memory")]
    {
        let allocations = observed.memory.allocations.unwrap();
        assert_eq!(allocations.total_count, 1);
        assert_eq!(allocations.total_bytes, 2 * size_of::<u64>() as u64);
        assert_eq!(allocations.retained_count, 1);
        assert_eq!(allocations.retained_bytes, 2 * size_of::<u64>() as i64);
    }
    #[cfg(not(feature = "bench-memory"))]
    {
        assert!(observed.memory.allocations.is_none());
        assert!(observed.memory.rss.is_none());
    }
}

#[cfg(feature = "bench-memory")]
#[test]
fn complete_case_peaks_are_relative_to_zero_not_the_prepared_heap() {
    let input = vec![0_u8; 4096];
    let observed = measure_case(&case(WorkloadClass::ExactAdapterMechanics), || {
        drop(std::hint::black_box(input));
        drop(std::hint::black_box(vec![0_u8; 1024]));
        CaseMeasurements::default()
    });
    let allocations = observed.memory.allocations.unwrap();
    assert_eq!(allocations.total_bytes, 1024);
    assert_eq!(allocations.peak_relative_count, 0);
    assert_eq!(allocations.peak_relative_bytes, 0);
    assert_eq!(allocations.retained_count, -1);
    assert_eq!(allocations.retained_bytes, -4096);
}

#[cfg(all(
    feature = "bench-memory",
    any(target_os = "macos", target_os = "linux")
))]
#[test]
fn rss_units_and_invalid_observations_are_not_guessed() {
    assert_eq!(super::rss_bytes(-1), None);
    assert_eq!(super::rss_bytes(0), Some(0));
    assert_eq!(
        super::rss_bytes(2),
        Some(if cfg!(target_os = "macos") { 2 } else { 2048 })
    );
    #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
    assert_eq!(super::rss_bytes(std::ffi::c_long::MAX), None);
}
