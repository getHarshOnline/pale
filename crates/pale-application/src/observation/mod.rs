//! Read-only snapshots of a compiled configuration and of the resident's state,
//! with deltas that bring a snapshot up to date.
//!
//! This module builds on `compile`, `resident` and `workspace`; `compile` and
//! `provider_invocation` build on it in return.
//!
//! Observation is a read projection, never a second authority store. The
//! builders in this module copy only typed, already-admitted facts from an
//! [`crate::compile::EffectiveProfileCompilation`] or
//! [`crate::resident::ResidentApplication`]. They do not read
//! provider bodies, reopen source, infer native readiness, or create a live
//! runtime dispatcher.
//!
//! [`ObservationSnapshot`] is generation- and revision-bound. A client may
//! apply an [`ObservationDelta`] only when both coordinates are compatible. In
//! the law below, `S_r` is the snapshot at revision `r` and `D_(r+1)` is its
//! proposed successor delta. The delta must name the same generation and the
//! exact revision step:

#![doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/observation-stream-laws.display.mathml"
))]
//!
//! A gap or generation change returns an explicit
//! [`ObservationDeltaOutcome::Resynchronize`]. Visualization lenses reuse the
//! same bounded facts and causal proof; their layout remains client-owned and
//! carries no semantic or native authority.
mod profile;
mod snapshot;
mod types;

pub use profile::{
    ObservationProfileError, ObservationProfileMapping, ObservationProfileQuery,
    ObservationProfileRows,
};
pub use snapshot::ObservationSnapshot;
pub use types::{
    ObservationBackpressureRecovery, ObservationCancellation, ObservationCategory,
    ObservationCausalPath, ObservationCausalSelector, ObservationCausalStep, ObservationChange,
    ObservationCursor, ObservationDelta, ObservationDeltaOutcome, ObservationDeltaRequest,
    ObservationFact, ObservationFilter, ObservationFreshness, ObservationFreshnessClass,
    ObservationGeneration, ObservationHealthState, ObservationLens, ObservationLensCursor,
    ObservationLensDisposition, ObservationLensNode, ObservationLensRequest,
    ObservationNativeState, ObservationPage, ObservationPageCursor, ObservationPageDisposition,
    ObservationPageRequest, ObservationProcessIntent, ObservationReadinessState,
    ObservationReconciliationState, ObservationResumeCoordinate, ObservationResynchronization,
    ObservationResynchronizationReason, ObservationRevision, ObservationStepCompletion,
    ObservationStepLimit, ObservationSubject, DEFAULT_OBSERVATION_CAUSAL_PATH_LOOKUPS,
    DEFAULT_OBSERVATION_CAUSAL_STEPS, DEFAULT_OBSERVATION_COPIED_STRING_BYTES,
    DEFAULT_OBSERVATION_INSPECTION_LIMIT, DEFAULT_OBSERVATION_PAGE_LIMIT,
    MAX_OBSERVATION_CAUSAL_PATH_LOOKUPS, MAX_OBSERVATION_CAUSAL_STEPS,
    MAX_OBSERVATION_COPIED_STRING_BYTES, MAX_OBSERVATION_DELTA_ITEMS, MAX_OBSERVATION_FACTS,
    MAX_OBSERVATION_INSPECTION_LIMIT, MAX_OBSERVATION_LENS_NODES, MAX_OBSERVATION_PAGE_LIMIT,
};
