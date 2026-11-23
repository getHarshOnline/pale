//! Cold reconstruction of retained generation and session-pin projections.
//!
//! A complete authority recovery yields retained generation-digest set `G` and
//! durable journal frontier `j`. If `D` is the set of canonical generation
//! digests reproduced by ordinary admission of the owner inputs, reconstruction
//! requires exact custody equality:

#![doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/resident-reconstruction-closure.inline.mathml"
))]
//!
//! Every packed graph, immutable arena, derived index, graph handle, and wake
//! cursor is therefore new process-local projection state. The independently
//! journaled desired process state is retained unchanged; it neither changes
//! `G` nor supplies generation inputs. Generation content rebuilds independently
//! because lifecycle retention is not predecessor closure; recovery creates no
//! activation-lineage proof. A degraded recovery exposes only its validated
//! semantic prefix and never invokes the input source.
use std::collections::{BTreeMap, BTreeSet};

use pale_core::records::{CompiledGenerationDigest, RingCursorV1, WakeEpochV1};
use pale_kernel::{AdaptiveNodeSetPolicy, WakeRingBuildError, WakeTokenRing};

use crate::compile::Compilation;

use super::generation::{
    AdmittedCompiledGeneration, AppliedRepositoryMaterialization, CompiledGenerationAdmissionError,
};
use super::journal::{AuthorityRecoveryOutcome, DegradedAuthorityState, RecoveredAuthorityState};
use super::working_set::{ResidentSessionWorkingSet, ResidentSessionWorkingSetError};

/// Complete consuming inputs for rebuilding one retained compiled generation.
///
/// [`Compilation`] remains the sealed one-shot oracle. Neither this value nor
/// its owner boundary carries graph nodes, arena handles, index positions, or
/// wake cursors.
pub struct ResidentGenerationReconstructionInput {
    expected_generation: CompiledGenerationDigest,
    compilation: Compilation,
    materializations: Vec<AppliedRepositoryMaterialization>,
}

impl ResidentGenerationReconstructionInput {
    /// Bind one expected retained identity to its complete one-shot inputs.
    ///
    /// Construction makes no genesis or predecessor claim. During
    /// [`ResidentApplication::cold_open`](crate::resident::ResidentApplication::cold_open),
    /// recovery re-admits the immutable content and deliberately produces no
    /// ordinary activation proof.
    #[must_use]
    pub fn new(
        expected_generation: CompiledGenerationDigest,
        compilation: Compilation,
        materializations: Vec<AppliedRepositoryMaterialization>,
    ) -> Self {
        Self {
            expected_generation,
            compilation,
            materializations,
        }
    }
}

/// Typed failure at the immutable generation-input custody boundary.
#[derive(Debug, thiserror::Error)]
pub enum ResidentGenerationInputSourceError {
    /// The owner cannot currently provide the complete requested input set.
    #[error("retained compiled-generation inputs are unavailable")]
    Unavailable,
}

/// Owner boundary for exact retained generation reconstruction inputs.
///
/// The requested set is the complete digest frontier recovered from the
/// lifecycle. Implementations return owned inputs because generation admission
/// consumes the one-shot [`Compilation`]. Implementations may obtain these
/// bounded inputs from native or durable owner custody; this trait defines
/// neither that I/O nor its encoding.
pub trait ResidentGenerationInputSource {
    /// Load the complete owned input set for the exact retained digest frontier.
    ///
    /// The returned claims remain untrusted until
    /// [`ResidentApplication::cold_open`](crate::resident::ResidentApplication::cold_open)
    /// re-admits every generation and proves exact set equality.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentGenerationInputSourceError`] when owner custody cannot
    /// supply the complete requested frontier.
    fn load_generation_inputs(
        &mut self,
        required_generations: &BTreeSet<CompiledGenerationDigest>,
    ) -> Result<Vec<ResidentGenerationReconstructionInput>, ResidentGenerationInputSourceError>;
}

/// Explicit physical parameters for one cold resident rebuild.
///
/// The set representation, resident-session capacity, and wake-ring capacity remain
/// caller-admitted measurement outputs. `wake_epoch` is a caller-supplied
/// process-local epoch; its numeric value alone does not prove freshness across
/// process starts.
#[derive(Debug, Clone, Copy)]
pub struct ResidentReconstructionParameters {
    set_policy: AdaptiveNodeSetPolicy,
    resident_session_capacity: usize,
    wake_ring_capacity: usize,
    wake_epoch: WakeEpochV1,
}

impl ResidentReconstructionParameters {
    /// Retain explicit bounded projection parameters without guessing defaults.
    #[must_use]
    pub const fn new(
        set_policy: AdaptiveNodeSetPolicy,
        resident_session_capacity: usize,
        wake_ring_capacity: usize,
        wake_epoch: WakeEpochV1,
    ) -> Self {
        Self {
            set_policy,
            resident_session_capacity,
            wake_ring_capacity,
            wake_epoch,
        }
    }
}

/// Exact cold projection of the currently implemented resident authority state.
///
/// The map owns exactly one fresh [`AdmittedCompiledGeneration`] for every
/// lifecycle-retained digest. The wake ring starts empty at the recovered
/// durable frontier and a new process-local epoch. Exact admitted session
/// leases are rebuilt from the same recovered lifecycle into a fresh bounded
/// runtime arena; old process-local handles therefore cannot survive recovery.
pub(crate) struct ReconstructedResident {
    pub(super) authority: RecoveredAuthorityState,
    pub(super) generations: BTreeMap<CompiledGenerationDigest, AdmittedCompiledGeneration>,
    pub(super) sessions: ResidentSessionWorkingSet,
    pub(super) wake_ring: WakeTokenRing,
}

/// Closed result of attempting resident reconstruction after journal recovery.
pub(crate) enum ResidentColdRecoveryOutcome {
    /// Complete authority, immutable generations, and sessions rebuilt exactly.
    Complete(Box<ReconstructedResident>),
    /// Read-only validated prefix plus its unresolved-suffix quarantine boundary.
    Degraded(Box<DegradedAuthorityState>),
}

/// One complete resident rebuild violated generation input or projection custody.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ResidentReconstructionError {
    /// Owner custody could not supply the requested immutable generation inputs.
    #[error(transparent)]
    InputSource(#[from] ResidentGenerationInputSourceError),
    /// The source returned an identity outside the recovered retained set.
    #[error("generation input names an identity outside the retained set: {generation:?}")]
    UnexpectedGeneration {
        /// Unrequested claimed generation identity.
        generation: CompiledGenerationDigest,
    },
    /// The source returned more than one input for the same expected identity.
    #[error("generation input is duplicated: {generation:?}")]
    DuplicateGeneration {
        /// Duplicated claimed generation identity.
        generation: CompiledGenerationDigest,
    },
    /// No input was supplied for one lifecycle-retained identity.
    #[error("retained generation input is missing: {generation:?}")]
    MissingGeneration {
        /// Required generation identity absent from owner input.
        generation: CompiledGenerationDigest,
    },
    /// Ordinary generation admission rejected the exact owner input.
    #[error("retained generation admission failed for {generation:?}: {source}")]
    GenerationAdmission {
        /// Claimed generation identity being reconstructed.
        generation: CompiledGenerationDigest,
        /// Exact existing admission failure.
        #[source]
        source: Box<CompiledGenerationAdmissionError>,
    },
    /// Admitted one-shot inputs produced a different canonical generation identity.
    #[error("generation input substitution: expected {expected:?}, observed {observed:?}")]
    GenerationSubstitution {
        /// Retained identity requested from owner custody.
        expected: CompiledGenerationDigest,
        /// Identity reproduced by ordinary generation admission.
        observed: CompiledGenerationDigest,
    },
    /// The explicit wake-ring parameters could not build a bounded fresh ring.
    #[error(transparent)]
    WakeRing(#[from] WakeRingBuildError),
    /// The recovered resident-session relation could not rebuild within its bound.
    #[error(transparent)]
    ResidentSessions(#[from] ResidentSessionWorkingSetError),
}

/// Rebuild every resident projection only after complete authority recovery.
///
/// Degraded recovery is a closed read-only branch: its validated semantic
/// prefix and quarantine boundary are returned before the input source or any
/// generation builder is called. Complete recovery independently reconstructs
/// each immutable body, checks every resulting digest against the claimed
/// retained identity, and creates an empty wake ring at cursor zero and the
/// recovered durable frontier. Rebuilt content cannot authorize ordinary
/// activation until a fresh compatibility proof is derived against the
/// then-current active generation.
///
/// # Errors
///
/// Returns [`ResidentReconstructionError`] for unavailable, missing, extra,
/// duplicate or substituted generation input, ordinary generation admission
/// failure, an invalid wake-ring capacity, or a resident-session bound, allocation,
/// or integrity failure.
pub(crate) fn reconstruct_resident(
    recovery: AuthorityRecoveryOutcome,
    source: &mut dyn ResidentGenerationInputSource,
    parameters: ResidentReconstructionParameters,
) -> Result<ResidentColdRecoveryOutcome, ResidentReconstructionError> {
    match recovery {
        AuthorityRecoveryOutcome::Complete(authority) => {
            reconstruct_complete(*authority, source, parameters)
                .map(|resident| ResidentColdRecoveryOutcome::Complete(Box::new(resident)))
        }
        AuthorityRecoveryOutcome::Degraded(degraded) => {
            Ok(ResidentColdRecoveryOutcome::Degraded(degraded))
        }
    }
}

fn reconstruct_complete(
    authority: RecoveredAuthorityState,
    source: &mut dyn ResidentGenerationInputSource,
    parameters: ResidentReconstructionParameters,
) -> Result<ReconstructedResident, ResidentReconstructionError> {
    let required = authority.lifecycle().snapshot().retained_generations;
    let sessions = ResidentSessionWorkingSet::rebuild(
        authority.lifecycle(),
        parameters.resident_session_capacity,
    )?;
    let mut inputs = source.load_generation_inputs(&required)?;
    inputs.sort_by(|left, right| left.expected_generation.cmp(&right.expected_generation));

    let mut pending = BTreeMap::new();
    for input in inputs {
        if !required.contains(&input.expected_generation) {
            return Err(ResidentReconstructionError::UnexpectedGeneration {
                generation: input.expected_generation,
            });
        }
        let key = input.expected_generation.clone();
        if pending.insert(key.clone(), input).is_some() {
            return Err(ResidentReconstructionError::DuplicateGeneration { generation: key });
        }
    }
    if let Some(missing) = required
        .iter()
        .find_map(|digest| (!pending.contains_key(digest)).then(|| digest.clone()))
    {
        return Err(ResidentReconstructionError::MissingGeneration {
            generation: missing,
        });
    }

    let mut generations = BTreeMap::new();
    for (_, input) in pending {
        let admitted = AdmittedCompiledGeneration::admit_recovered(
            input.compilation,
            input.materializations,
            parameters.set_policy,
        )
        .map_err(|source| ResidentReconstructionError::GenerationAdmission {
            generation: input.expected_generation.clone(),
            source: Box::new(source),
        })?;
        let observed = admitted.record().compiled_generation_digest().clone();
        if observed != input.expected_generation {
            return Err(ResidentReconstructionError::GenerationSubstitution {
                expected: input.expected_generation,
                observed,
            });
        }
        generations.insert(observed, admitted);
    }

    let wake_ring = WakeTokenRing::new(
        parameters.wake_ring_capacity,
        parameters.wake_epoch,
        RingCursorV1::new(0),
        authority.journal_head(),
    )?;
    Ok(ReconstructedResident {
        authority,
        generations,
        sessions,
        wake_ring,
    })
}

#[cfg(test)]
#[path = "../../tests/internal/resident/reconstruction.rs"]
mod tests;
