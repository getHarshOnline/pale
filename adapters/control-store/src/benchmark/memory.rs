//! Benchmark-only memory and explicit-copy evidence.

use super::input::{BenchmarkCase, WorkloadClass};
use super::workload::CaseMeasurements;

/// Memory evidence whose scopes deliberately do not masquerade as equivalent.
#[derive(Debug, Default)]
pub(crate) struct MemoryEvidence {
    /// Current-thread Rust allocations across the complete case.
    pub(crate) allocations: Option<AllocationEvidence>,
    /// Process-lifetime high-water observations after the complete case.
    pub(crate) rss: Option<RssEvidence>,
    /// Static lower bound for explicit payload-slice copies in one independent run.
    pub(crate) explicit_copy_bytes_per_run: u64,
}

/// Allocation-counter observations for the parent benchmark thread only.
///
/// Balances start at zero. Dropping pre-existing input may make them negative
/// and suppress later peaks; these are not absolute live-heap measurements.
#[derive(Debug, Clone, Copy)]
pub(crate) struct AllocationEvidence {
    /// Total allocation calls observed.
    pub(crate) total_count: u64,
    /// Total bytes allocated, including bytes subsequently freed.
    pub(crate) total_bytes: u64,
    /// Maximum nonnegative relative allocation-count balance.
    pub(crate) peak_relative_count: u64,
    /// Maximum nonnegative relative allocated-byte balance.
    pub(crate) peak_relative_bytes: u64,
    /// Net live allocation count when the case returned.
    pub(crate) retained_count: i64,
    /// Net live allocated bytes when the case returned.
    pub(crate) retained_bytes: i64,
}

/// Operating-system resident-set high-water observations.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RssEvidence {
    /// Current process lifetime high-water mark, normalized to bytes.
    pub(crate) process_bytes: u64,
    /// Terminated-child lifetime high-water mark, normalized to bytes.
    pub(crate) terminated_children_bytes: u64,
}

/// Run one complete case under opt-in benchmark instrumentation.
///
/// Includes timing collection, retained results, and parent-side child launch
/// mechanics. Counter bookkeeping remains enabled during timed operations.
/// Native `SQLite` heaps and child-thread/process allocations are not counted.
pub(crate) fn measure_case(
    case: &BenchmarkCase,
    run: impl FnOnce() -> CaseMeasurements,
) -> CaseMeasurements {
    #[cfg(feature = "bench-memory")]
    let (mut measurements, allocations) = {
        let mut result = None;
        let observed = allocation_counter::measure(|| result = Some(run()));
        let measurements = result.expect("measured benchmark case returns its observations");
        (
            measurements,
            Some(AllocationEvidence {
                total_count: observed.count_total,
                total_bytes: observed.bytes_total,
                peak_relative_count: observed.count_max,
                peak_relative_bytes: observed.bytes_max,
                retained_count: observed.count_current,
                retained_bytes: observed.bytes_current,
            }),
        )
    };

    #[cfg(not(feature = "bench-memory"))]
    let (mut measurements, allocations) = (run(), None);

    measurements.memory = MemoryEvidence {
        allocations,
        rss: resident_set_high_water(),
        explicit_copy_bytes_per_run: explicit_copy_lower_bound(case),
    };
    measurements
}

/// Return the exact scope used by the allocation-counter columns.
pub(crate) const fn allocation_scope() -> &'static str {
    if cfg!(feature = "bench-memory") {
        "parent_current_thread_complete_case_rust_allocator_only"
    } else {
        "unavailable_feature_disabled"
    }
}

/// Actual benchmark allocator; the wrapper is absent when instrumentation is disabled.
pub(crate) const fn allocator_coordinate() -> &'static str {
    if cfg!(feature = "bench-memory") {
        "system_with_allocation-counter-0.8.1"
    } else {
        "system_uninstrumented"
    }
}

/// Counter bookkeeping treatment inside the reported timing intervals.
pub(crate) const fn timing_allocation_counter() -> &'static str {
    if cfg!(feature = "bench-memory") {
        "enabled_complete_case"
    } else {
        "disabled"
    }
}

/// Return only verified native units; unavailable observations carry no unit claim.
pub(crate) const fn rss_source_unit() -> &'static str {
    if cfg!(target_os = "macos") {
        "bytes"
    } else if cfg!(target_os = "linux") {
        "kibibytes"
    } else {
        "unavailable"
    }
}

/// Read process-lifetime RSS through the safe `nix` wrapper when enabled.
#[cfg(all(
    feature = "bench-memory",
    any(target_os = "macos", target_os = "linux")
))]
fn resident_set_high_water() -> Option<RssEvidence> {
    use nix::sys::resource::{getrusage, UsageWho};

    Some(RssEvidence {
        process_bytes: rss_bytes(getrusage(UsageWho::RUSAGE_SELF).ok()?.max_rss())?,
        terminated_children_bytes: rss_bytes(getrusage(UsageWho::RUSAGE_CHILDREN).ok()?.max_rss())?,
    })
}

#[cfg(not(all(
    feature = "bench-memory",
    any(target_os = "macos", target_os = "linux")
)))]
const fn resident_set_high_water() -> Option<RssEvidence> {
    None
}

/// Normalize verified macOS bytes or Linux KiB; invalid observations are unavailable.
#[cfg(all(
    feature = "bench-memory",
    any(target_os = "macos", target_os = "linux")
))]
fn rss_bytes(value: std::ffi::c_long) -> Option<u64> {
    let value = u64::try_from(value).ok()?;
    if cfg!(target_os = "macos") {
        Some(value)
    } else {
        value.checked_mul(1024)
    }
}

/// Compute a conservative, source-auditable payload-copy lower bound.
///
/// The rendered law measures control-store-owned copies from journal payload,
/// checkpoint, record-count, and bounded recovery-tail inputs. Its configured
/// maximum tail-entry count is independent of the LSP text adapter's copy
/// accounting. Here `r` is the record count, `p` the payload bytes per record,
/// `c` the checkpoint bytes, `t` the retained recovery-tail count, and `T_max`
/// the configured tail-entry ceiling. Both `B` values are byte counts. The
/// exact source-auditable lower bounds are:
#[doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/explicit-copy-lower-bound.display.mathml"
))]
///
/// Regular and crash paths each copy every payload into a bounded value and a
/// physical frame; recovery copies one checkpoint and `t` tail payloads. Native
/// `SQLite` binding copies remain deliberately outside the direct-lane bound.
fn explicit_copy_lower_bound(case: &BenchmarkCase) -> u64 {
    let payload = u64::try_from(case.payload_bytes).expect("payload bytes fit u64");
    let checkpoint = u64::try_from(case.checkpoint_bytes).expect("checkpoint bytes fit u64");
    match case.workload_class {
        WorkloadClass::ExactAdapterMechanics => {
            let tail = case.records.saturating_sub(1).min(
                u64::try_from(case.limits.maximum_tail_entries())
                    .expect("tail-entry bound fits u64"),
            );
            case.records
                .checked_mul(payload)
                .and_then(|bytes| bytes.checked_mul(4))
                .and_then(|bytes| bytes.checked_add(checkpoint.checked_mul(5)?))
                .and_then(|bytes| bytes.checked_add(tail.checked_mul(payload)?))
                .expect("explicit-copy lower bound overflow")
        }
        WorkloadClass::SqliteCharacterization => payload
            .checked_mul(2)
            .expect("explicit-copy lower bound overflow"),
    }
}

#[cfg(test)]
#[path = "../../tests/internal/benchmark_memory.rs"]
mod tests;
