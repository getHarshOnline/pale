//! Verify a materialization plan against prior and desired file ownership.

use std::collections::{BTreeMap, BTreeSet};

use pale_core::canon;
use pale_core::records::{
    AgentSurfacePlanV1, BaselineEntry, BaselineStateV1, MaterializationStateV1, SurfaceAction,
    SurfaceDigest, SurfaceOwnership, MATERIALIZATION_STATE_PATH,
};

use super::read::validate_state;
use crate::errors::{data_error, FsError};
use pale_core::records::MaterializationContractIdentity;

/// Exact ownership indexes and bytes admitted for one state transition.
pub(crate) struct VerifiedTransition {
    /// Prior owned surfaces keyed by relative path.
    pub(crate) prior: BTreeMap<String, SurfaceDigest>,
    /// Desired owned surfaces keyed by relative path.
    pub(crate) next: BTreeMap<String, SurfaceDigest>,
    /// Admitted pre-mutation target observations keyed by relative path.
    pub(crate) baseline: BTreeMap<String, BaselineEntry>,
    /// Canonical next-state bytes, absent only for full removal.
    pub(crate) next_state_bytes: Option<Vec<u8>>,
}

fn invalid(message: impl Into<String>) -> FsError {
    FsError::InvalidState(message.into())
}

fn expected_action(
    prior: Option<&SurfaceDigest>,
    next: Option<&SurfaceDigest>,
    baseline_exists: bool,
    planned: SurfaceAction,
) -> Result<SurfaceAction, FsError> {
    match (prior, next) {
        (None, Some(_next)) => {
            if planned == SurfaceAction::Adopt {
                if !baseline_exists {
                    return Err(invalid("adopt requires a pre-existing generated target"));
                }
                Ok(SurfaceAction::Adopt)
            } else {
                Ok(SurfaceAction::Create)
            }
        }
        (Some(previous), None) => Ok(match previous.ownership {
            SurfaceOwnership::ManagedBlock => SurfaceAction::Splice,
            SurfaceOwnership::WholeFile => SurfaceAction::Delete,
        }),
        (Some(previous), Some(next)) => {
            if previous.ownership != next.ownership {
                return Err(invalid(format!(
                    "ownership mode changed at {}",
                    previous.rel_path
                )));
            }
            Ok(if previous.sha256 == next.sha256 {
                SurfaceAction::Unchanged
            } else {
                SurfaceAction::Update
            })
        }
        (None, None) => Err(invalid("empty state transition")),
    }
}

/// Reconcile an admitted operation set with the exact prior and desired ownership maps.
pub(crate) fn verify_transition(
    plan: &AgentSurfacePlanV1,
    actual_prior: Option<&MaterializationStateV1>,
    contract_identity: &MaterializationContractIdentity,
) -> Result<VerifiedTransition, FsError> {
    if let Some(state) = actual_prior {
        validate_state(state, contract_identity)?;
        if plan
            .transition
            .prior()
            .map(|prior| &prior.one_shot_compilation_digest)
            != Some(&state.one_shot_compilation_digest)
        {
            return Err(invalid(
                "prior state compilation identity does not match the admitted plan",
            ));
        }
        if state.repository_id != plan.repository_id {
            return Err(invalid("prior state belongs to another repository"));
        }
    }
    let actual_digest = actual_prior.map(|state| state.state_digest.clone());
    if actual_digest.as_ref() != plan.transition.prior().map(|prior| &prior.state_digest) {
        return Err(FsError::Collision(
            MATERIALIZATION_STATE_PATH.into(),
            "target state does not match the admitted plan".into(),
        ));
    }

    let prior = actual_prior
        .map(|state| {
            state
                .surfaces
                .iter()
                .cloned()
                .map(|surface| (surface.rel_path.clone(), surface))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();
    let (next, next_state_bytes) = match plan.transition.next_state() {
        Some(state) => {
            validate_state(state, contract_identity)?;
            if state.one_shot_compilation_digest != contract_identity.one_shot_compilation_digest
                || state.one_shot_compilation_digest != plan.one_shot_compilation_digest
            {
                return Err(invalid(
                    "next state compilation identity does not match the admitted plan",
                ));
            }
            if state.repository_id != plan.repository_id
                || state.profile_digest != plan.profile_digest
                || state.instance_digest != plan.instance_digest
                || state.manifest_digest != plan.manifest_digest
            {
                return Err(invalid("next state identity does not match the plan"));
            }
            let bytes = canon::to_canonical_string(state)
                .map_err(|source| {
                    data_error(
                        "encode next materialization state",
                        MATERIALIZATION_STATE_PATH,
                        source,
                    )
                })?
                .into_bytes();
            let surfaces = state
                .surfaces
                .iter()
                .cloned()
                .map(|surface| (surface.rel_path.clone(), surface))
                .collect();
            (surfaces, Some(bytes))
        }
        None => (BTreeMap::new(), None),
    };

    let expected_paths = match plan.transition.next_state() {
        Some(_) => prior
            .keys()
            .chain(next.keys())
            .cloned()
            .collect::<BTreeSet<_>>(),
        None => prior.keys().cloned().collect(),
    };
    if plan.operations.len() != expected_paths.len() {
        return Err(invalid("operation set does not cover the state transition"));
    }
    let mut seen = BTreeSet::new();
    let mut baseline = BTreeMap::new();
    for entry in &plan.target_baseline {
        if baseline
            .insert(entry.rel_path.clone(), entry.clone())
            .is_some()
        {
            return Err(invalid(format!(
                "duplicate target baseline at {}",
                entry.rel_path
            )));
        }
    }
    for operation in &plan.operations {
        if !seen.insert(operation.rel_path.as_str())
            || !expected_paths.contains(&operation.rel_path)
        {
            return Err(invalid(
                "operation paths are duplicated or outside the transition",
            ));
        }
        let previous = prior.get(&operation.rel_path);
        let desired = next.get(&operation.rel_path);
        let baseline_exists = baseline
            .get(&operation.rel_path)
            .is_some_and(|entry| matches!(&entry.observation, BaselineStateV1::Present { .. }));
        let expected_action = expected_action(
            previous,
            desired,
            baseline_exists,
            operation.transition.action(),
        )?;
        let identity = desired.or(previous).ok_or_else(|| {
            invalid(format!(
                "transition path has no prior or desired identity at {}",
                operation.rel_path
            ))
        })?;
        if operation.transition.action() != expected_action
            || operation.kind != identity.kind
            || operation.transition.ownership() != identity.ownership
            || operation.transition.content_sha256() != &identity.sha256
            || operation.committable != identity.committable
        {
            return Err(invalid(format!(
                "operation does not match the state transition at {}",
                operation.rel_path
            )));
        }
    }

    Ok(VerifiedTransition {
        prior,
        next,
        baseline,
        next_state_bytes,
    })
}
