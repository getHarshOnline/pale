//! Timing and row semantics shared by the kernel harness and its behavioral tests.

use std::hint::black_box;
use std::num::NonZeroUsize;
use std::time::{Duration, Instant};

/// Minimum elapsed batch interval sought when calibrating short complete calls.
const CALIBRATION_TARGET: Duration = Duration::from_millis(2);
/// Maximum calls per batch, bounding calibration even below its target interval.
const MAX_BATCH: usize = 65_536;

/// Existing smoke and characterization schedules; neither selects a budget.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Sampling {
    Smoke,
    Characterization,
}

impl Sampling {
    const fn samples(self) -> usize {
        match self {
            Self::Smoke => 5,
            Self::Characterization => 201,
        }
    }
}

/// Destruction included in the elapsed interval, distinct from preparation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TimingScope {
    /// Repeated complete calls, including destruction of each returned value.
    CompleteCall,
    /// Owned input is consumed inside timing; returned output is dropped later.
    ConsumedState,
    /// Prepared mutable state and returned output both outlive elapsed capture.
    RetainedState,
}

impl TimingScope {
    const fn label(self) -> &'static str {
        match self {
            Self::CompleteCall => "call_and_result_drop",
            Self::ConsumedState => "consumed_state_returned_result_retained",
            Self::RetainedState => "prepared_state_and_result_retained",
        }
    }
}

/// Nearest-rank distributions of integer batch-average elapsed nanoseconds.
///
/// A sample divides one elapsed batch by its call count, flooring the result
/// to an integer and then to at least one nanosecond. Its percentiles and maximum
/// are not individual-call tail latencies. Reciprocal mean throughput counts
/// complete benchmark calls, not the number of seed sets inside one graph call.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Measurement {
    samples: usize,
    batch_size: usize,
    timing_scope: TimingScope,
    p50_ns: u128,
    p95_ns: u128,
    p99_ns: Option<u128>,
    max_ns: u128,
    calls_per_second: u128,
}

impl Measurement {
    pub(crate) const TSV_HEADER: &'static str = "samples\tbatch_size\ttiming_scope\tbatch_mean_p50_ns\tbatch_mean_p95_ns\tbatch_mean_p99_ns\tbatch_mean_max_ns\tbenchmark_calls_per_second";

    /// Time complete repeated calls, including each result's destruction.
    pub(crate) fn sample<T>(sampling: Sampling, mut operation: impl FnMut() -> T) -> Self {
        black_box(operation());
        let batch_size = match sampling {
            Sampling::Smoke => 1,
            Sampling::Characterization => calibrated_batch(&mut operation),
        };
        let mut observations = Vec::with_capacity(sampling.samples());
        for _ in 0..sampling.samples() {
            let start = Instant::now();
            for _ in 0..batch_size {
                black_box(operation());
            }
            observations.push((start.elapsed().as_nanos() / batch_size as u128).max(1));
        }
        Self::summarize(observations, batch_size, TimingScope::CompleteCall)
    }

    /// Prepare outside timing and consume inside; only returned output is retained.
    pub(crate) fn sample_prepared<S, T>(
        sampling: Sampling,
        mut prepare: impl FnMut() -> S,
        mut operation: impl FnMut(S) -> T,
    ) -> Self {
        black_box(operation(prepare()));
        let mut observations = Vec::with_capacity(sampling.samples());
        for _ in 0..sampling.samples() {
            let state = prepare();
            let start = Instant::now();
            let output = black_box(operation(state));
            let elapsed = start.elapsed();
            black_box(&output);
            observations.push(elapsed.as_nanos().max(1));
        }
        Self::summarize(observations, 1, TimingScope::ConsumedState)
    }

    /// Borrow prepared state for one call, retaining it and the output past timing.
    pub(crate) fn sample_retained<S, T>(
        sampling: Sampling,
        mut prepare: impl FnMut() -> S,
        mut operation: impl FnMut(&mut S) -> T,
    ) -> Self {
        black_box(operation(&mut prepare()));
        let mut observations = Vec::with_capacity(sampling.samples());
        for _ in 0..sampling.samples() {
            let (elapsed, state, output) = retained_observation(prepare(), &mut operation);
            black_box((&state, &output));
            observations.push(elapsed.as_nanos().max(1));
        }
        Self::summarize(observations, 1, TimingScope::RetainedState)
    }

    pub(crate) fn summarize(
        mut observations: Vec<u128>,
        batch_size: usize,
        timing_scope: TimingScope,
    ) -> Self {
        let sum = observations.iter().sum::<u128>();
        let samples = observations.len();
        observations.sort_unstable();
        let mean = (sum / samples as u128).max(1);
        Self {
            samples,
            batch_size,
            timing_scope,
            p50_ns: nearest_rank(&observations, 50),
            p95_ns: nearest_rank(&observations, 95),
            p99_ns: (samples >= 101).then(|| nearest_rank(&observations, 99)),
            max_ns: *observations.last().expect("measurement has samples"),
            calls_per_second: 1_000_000_000 / mean,
        }
    }

    pub(crate) fn tsv(self) -> String {
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            self.samples,
            self.batch_size,
            self.timing_scope.label(),
            self.p50_ns,
            self.p95_ns,
            self.p99_ns
                .map(|value| value.to_string())
                .unwrap_or_default(),
            self.max_ns,
            self.calls_per_second,
        )
    }
}

/// Return both owners after elapsed capture, excluding their destruction.
pub(crate) fn retained_observation<S, T>(
    mut state: S,
    operation: impl FnOnce(&mut S) -> T,
) -> (Duration, S, T) {
    let start = Instant::now();
    let output = black_box(operation(&mut state));
    let elapsed = start.elapsed();
    (elapsed, state, output)
}

fn calibrated_batch<T>(operation: &mut impl FnMut() -> T) -> usize {
    let mut batch = 1;
    loop {
        let start = Instant::now();
        for _ in 0..batch {
            black_box(operation());
        }
        if start.elapsed() >= CALIBRATION_TARGET || batch == MAX_BATCH {
            return batch;
        }
        batch = (batch * 2).min(MAX_BATCH);
    }
}

fn nearest_rank(sorted: &[u128], percentile: usize) -> u128 {
    let rank = (percentile * sorted.len()).div_ceil(100).saturating_sub(1);
    sorted[rank]
}

/// Observed scheduler availability is neither an actual worker count nor zero
/// when the host cannot report it. Native backend worker counts are unobserved.
pub(crate) fn parallelism_field(observed: std::io::Result<NonZeroUsize>) -> String {
    observed.map(|count| count.to_string()).unwrap_or_default()
}
