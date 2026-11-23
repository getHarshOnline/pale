//! Bounded analysis scheduling for editor generations.

use std::time::{Duration, Instant};

/// Quiet interval required before the latest editor generation is compiled.
pub(super) const ANALYSIS_DEBOUNCE: Duration = Duration::from_millis(150);

/// Monotone debounce and stale-result coordinate.
#[derive(Debug, Default)]
pub(super) struct AnalysisClock {
    pub(super) generation: u64,
    due: Option<Instant>,
}

impl AnalysisClock {
    /// Supersede prior scheduled work and return the new generation.
    ///
    /// # Errors
    ///
    /// Returns [`AnalysisClockError::GenerationExhausted`] at the terminal
    /// process-local generation.
    pub(super) fn schedule(&mut self, now: Instant) -> Result<u64, AnalysisClockError> {
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or(AnalysisClockError::GenerationExhausted)?;
        self.due = Some(now + ANALYSIS_DEBOUNCE);
        Ok(self.generation)
    }

    /// Claim due work only for the current generation.
    pub(super) fn claim(&mut self, generation: u64, now: Instant) -> bool {
        if generation != self.generation || self.due.is_none_or(|due| now < due) {
            return false;
        }
        self.due = None;
        true
    }
}

/// Failure to advance a process-local analysis coordinate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub(super) enum AnalysisClockError {
    /// The generation has no representable successor.
    #[error("analysis generation is exhausted")]
    GenerationExhausted,
}

#[cfg(test)]
#[path = "../tests/internal/scheduling.rs"]
mod tests;
