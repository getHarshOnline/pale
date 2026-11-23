//! In-process reversible reconciliation from trusted ownership to an admitted state.
//!
//! All effects are classified before mutation and rechecked against a bounded
//! bytes-and-mode snapshot immediately before each effect. Recovery covers recorded
//! effects in the current process; it does not make the multi-file sequence
//! crash-atomic.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::path::Path;

use pale_core::admission::AdmittedPlan;
use pale_core::records::{
    AgentSurfacePlanV1, ArtifactDigest, PlanDigest, ReceiptOperation, SurfaceAction,
    SurfaceCreationObservationV1, SurfaceOperationObservationV1, UnchangedDispositionV1,
    UnixFileMode, MATERIALIZATION_STATE_PATH,
};

use crate::confine::{ConfinedLocation, ConfinedRoot};
use crate::errors::FsError;
use crate::file_transaction::{
    backup_of, recover, AppliedFileTransaction, AppliedFileTransactionObservation, Backup,
    FileSnapshot,
};
use crate::materialization_state::{lineage_relative, read_trusted_from_roots, verify_transition};
use pale_core::records::MaterializationContractIdentity;

mod record;
mod surface;

/// Byte-level effect selected after ownership and transition admission.
enum Mutation<'payload> {
    Unchanged,
    Write(Cow<'payload, [u8]>),
    Delete,
}

/// One fully classified leaf mutation awaiting precondition revalidation.
struct Staged<'payload> {
    location: ConfinedLocation,
    rel_path: String,
    action: SurfaceAction,
    mutation: Mutation<'payload>,
    pre_sha256: Option<ArtifactDigest>,
    pre_mode: Option<u32>,
    post_sha256: Option<ArtifactDigest>,
    mode: u32,
    record_receipt: bool,
}

/// The two forward effects whose failure boundary drives transaction recovery.
///
/// This port is private to reconciliation. Production has one filesystem
/// implementation; descendant unit tests supply deterministic faults without
/// widening the crate API.
trait MutationEffects {
    fn write(&self, location: &ConfinedLocation, bytes: &[u8], mode: u32) -> Result<(), FsError>;

    fn remove(&self, location: &ConfinedLocation) -> Result<(), FsError>;
}

/// Production implementation of the private reconciliation effect seam.
struct FilesystemEffects;

impl MutationEffects for FilesystemEffects {
    fn write(&self, location: &ConfinedLocation, bytes: &[u8], mode: u32) -> Result<(), FsError> {
        location.write_materialized_surface(bytes, mode)
    }

    fn remove(&self, location: &ConfinedLocation) -> Result<(), FsError> {
        if location.remove_file()? {
            Ok(())
        } else {
            Err(FsError::Collision(
                location.relative().to_string(),
                "owned file disappeared before removal".into(),
            ))
        }
    }
}

/// Execute one admitted state transition through the production filesystem effects.
pub(super) fn reconcile<T: AsRef<str>>(
    root: &Path,
    lineage_root: &Path,
    admitted: &AdmittedPlan,
    contents: &BTreeMap<String, T>,
    contract_identity: &MaterializationContractIdentity,
) -> Result<AppliedFileTransaction, FsError> {
    reconcile_with(
        &FilesystemEffects,
        root,
        lineage_root,
        admitted.plan(),
        admitted.digest(),
        contents,
        contract_identity,
    )
}

fn reconcile_with<T: AsRef<str>>(
    effects: &dyn MutationEffects,
    root: &Path,
    lineage_root: &Path,
    plan: &AgentSurfacePlanV1,
    admitted_plan_digest: &PlanDigest,
    contents: &BTreeMap<String, T>,
    contract_identity: &MaterializationContractIdentity,
) -> Result<AppliedFileTransaction, FsError> {
    let target = ConfinedRoot::open(root)?;
    let lineage = ConfinedRoot::open(lineage_root)?;
    verify_root_identities(&target, &lineage)?;
    let root_canon = target.canonical_path();
    let (apply_lock, coordination_effect) = crate::file_transaction::acquire_apply_lock(&target)
        .map_err(|failure| {
            let effect = failure.effect();
            failure.into_source().with_coordination_effect(effect)
        })?;
    let settlement = (|| -> Result<AppliedFileTransaction, FsError> {
        verify_root_identities(&target, &lineage)?;
        let actual_prior =
            read_trusted_from_roots(&target, &lineage, &plan.repository_id, contract_identity)?;
        let transition = verify_transition(
            plan,
            actual_prior.as_ref().map(|prior| &prior.state),
            contract_identity,
        )?;
        let prior_lineage_digest = actual_prior
            .as_ref()
            .map(|prior| prior.lineage.lineage_digest.clone());
        let prior_state_bytes = actual_prior
            .as_ref()
            .map(|prior| pale_core::canon::to_canonical_string(&prior.state))
            .transpose()
            .map_err(|source| {
                crate::errors::data_error(
                    "encode prior materialization state",
                    pale_core::records::MATERIALIZATION_STATE_PATH,
                    source,
                )
            })?;
        let lineage_record_path = crate::materialization_state::lineage_relative(root_canon);
        let prior_lineage_bytes = actual_prior
            .as_ref()
            .map(|prior| pale_core::canon::to_canonical_string(&prior.lineage))
            .transpose()
            .map_err(|source| {
                crate::errors::data_error(
                    "encode prior materialization lineage",
                    &lineage_record_path,
                    source,
                )
            })?;
        let next_lineage = record::next_lineage(
            plan,
            admitted_plan_digest,
            root_canon,
            &contract_identity.lineage_schema,
        )?;
        let post_lineage_digest = next_lineage.as_ref().map(|(_, digest)| digest.clone());

        let staged_capacity = plan.operations.len().checked_add(2).ok_or_else(|| {
            crate::errors::invariant(
                "reserve staged mutation",
                root,
                "operation count exceeds addressable memory",
            )
        })?;
        let mut staged = Vec::new();
        staged
            .try_reserve_exact(staged_capacity)
            .map_err(|source| crate::errors::data_error("reserve staged mutation", root, source))?;
        for operation in &plan.operations {
            staged.push(surface::stage(
                &target,
                operation,
                transition.prior.get(&operation.rel_path),
                transition.next.get(&operation.rel_path),
                transition.baseline.get(&operation.rel_path),
                contents,
            )?);
        }
        if let Some(state) = record::stage(
            target.location(MATERIALIZATION_STATE_PATH)?,
            MATERIALIZATION_STATE_PATH,
            prior_state_bytes.as_deref().map(str::as_bytes),
            transition.next_state_bytes,
            false,
        )? {
            staged.push(state);
        }
        if let Some(lineage) = record::stage(
            lineage.location(&lineage_relative(root_canon))?,
            "materialization-lineage",
            prior_lineage_bytes.as_deref().map(str::as_bytes),
            next_lineage.map(|(bytes, _)| bytes),
            false,
        )? {
            staged.push(lineage);
        }

        apply_staged(
            effects,
            &staged,
            StagedApplication {
                apply_lock,
                coordination_effect,
                prior_lineage_digest,
                post_lineage_digest,
                target_root: &target,
                lineage_root: &lineage,
            },
        )
    })();
    settlement.map_err(|source| source.with_coordination_effect(coordination_effect))
}

/// Transaction-scoped custody consumed by [`apply_staged`].
///
/// Grouping these inseparable coordinates prevents the effect loop from
/// acquiring a positional parameter list while preserving their nominal types.
struct StagedApplication<'root> {
    apply_lock: crate::lock::AdvisoryLock,
    coordination_effect: crate::lock::AdvisoryLockEffect,
    prior_lineage_digest: Option<pale_core::records::MaterializationLineageDigest>,
    post_lineage_digest: Option<pale_core::records::MaterializationLineageDigest>,
    target_root: &'root ConfinedRoot,
    lineage_root: &'root ConfinedRoot,
}

fn apply_staged(
    effects: &dyn MutationEffects,
    staged: &[Staged<'_>],
    application: StagedApplication<'_>,
) -> Result<AppliedFileTransaction, FsError> {
    let StagedApplication {
        apply_lock,
        coordination_effect,
        prior_lineage_digest,
        post_lineage_digest,
        target_root,
        lineage_root,
    } = application;
    let mut backups = Vec::new();
    backups.try_reserve_exact(staged.len()).map_err(|source| {
        crate::errors::data_error(
            "reserve transaction recovery records",
            target_root.canonical_path(),
            source,
        )
    })?;
    let mut created_dirs = Vec::new();
    let mut results = Vec::new();
    results.try_reserve_exact(staged.len()).map_err(|source| {
        crate::errors::data_error(
            "reserve transaction receipt operations",
            target_root.canonical_path(),
            source,
        )
    })?;
    let mutation = (|| -> Result<(), FsError> {
        for effect in staged {
            verify_root_identities(target_root, lineage_root)?;
            let snapshot = backup_of(&effect.location)?;
            verify_precondition(effect, snapshot.as_ref())?;
            match &effect.mutation {
                Mutation::Unchanged => {}
                Mutation::Write(bytes) => {
                    backups.push(Backup::write(
                        effect.location.clone(),
                        snapshot,
                        bytes,
                        effect.mode,
                    ));
                    effect.location.ensure_parent_tracked(&mut created_dirs)?;
                    effects.write(&effect.location, bytes, effect.mode)?;
                    let _published_identity = backups
                        .last_mut()
                        .ok_or_else(|| {
                            crate::errors::invariant(
                                "bind published write recovery identity",
                                effect.location.display_path(),
                                "recovery journal lost the staged write",
                            )
                        })?
                        .verify_published()?;
                    verify_root_identities(target_root, lineage_root)?;
                }
                Mutation::Delete => {
                    backups.push(Backup::delete(effect.location.clone(), snapshot));
                    effects.remove(&effect.location)?;
                    let _published_identity = backups
                        .last_mut()
                        .ok_or_else(|| {
                            crate::errors::invariant(
                                "bind published delete recovery identity",
                                effect.location.display_path(),
                                "recovery journal lost the staged deletion",
                            )
                        })?
                        .verify_published()?;
                    verify_root_identities(target_root, lineage_root)?;
                }
            }
            if effect.record_receipt {
                results.push(ReceiptOperation {
                    rel_path: effect.rel_path.clone(),
                    observation: receipt_observation(effect)?,
                    mode: UnixFileMode::new(effect.mode).map_err(|source| {
                        FsError::InvalidState(format!(
                            "staged operation `{}` carries an invalid mode: {source}",
                            effect.rel_path
                        ))
                    })?,
                });
            }
        }
        verify_root_identities(target_root, lineage_root)?;
        Ok(())
    })();

    if let Err(error) = mutation {
        if let Err(restore_error) = recover(&backups, &created_dirs) {
            return Err(FsError::RestoreFailed {
                source: Box::new(error),
                failures: restore_error,
            });
        }
        return Err(error);
    }
    Ok(AppliedFileTransaction::new(
        AppliedFileTransactionObservation {
            operations: results,
            prior_lineage_digest,
            post_lineage_digest,
        },
        coordination_effect,
        backups,
        created_dirs,
        apply_lock,
    ))
}

fn receipt_observation(effect: &Staged<'_>) -> Result<SurfaceOperationObservationV1, FsError> {
    let required = |digest: &Option<ArtifactDigest>, position: &str| {
        digest.clone().ok_or_else(|| {
            FsError::InvalidState(format!(
                "staged `{}` operation has no {position} content identity",
                effect.rel_path
            ))
        })
    };
    match effect.action {
        SurfaceAction::Create => {
            let post_sha256 = required(&effect.post_sha256, "post")?;
            let target = match &effect.pre_sha256 {
                Some(pre_sha256) => SurfaceCreationObservationV1::ExistingFile {
                    pre_sha256: pre_sha256.clone(),
                    post_sha256,
                },
                None => SurfaceCreationObservationV1::AbsentFile { post_sha256 },
            };
            Ok(SurfaceOperationObservationV1::Create { target })
        }
        SurfaceAction::Adopt => Ok(SurfaceOperationObservationV1::Adopt {
            pre_sha256: required(&effect.pre_sha256, "pre")?,
            post_sha256: required(&effect.post_sha256, "post")?,
        }),
        SurfaceAction::Update => Ok(SurfaceOperationObservationV1::Update {
            pre_sha256: required(&effect.pre_sha256, "pre")?,
            post_sha256: required(&effect.post_sha256, "post")?,
        }),
        SurfaceAction::Unchanged => {
            let pre = required(&effect.pre_sha256, "pre")?;
            let post = required(&effect.post_sha256, "post")?;
            if pre != post {
                return Err(FsError::InvalidState(format!(
                    "staged unchanged operation `{}` changed content identity",
                    effect.rel_path
                )));
            }
            let disposition = if effect.pre_mode == Some(effect.mode) {
                UnchangedDispositionV1::Verified
            } else {
                UnchangedDispositionV1::ModeRepaired
            };
            Ok(SurfaceOperationObservationV1::Unchanged {
                sha256: post,
                disposition,
            })
        }
        SurfaceAction::Delete => Ok(SurfaceOperationObservationV1::Delete {
            pre_sha256: required(&effect.pre_sha256, "pre")?,
        }),
        SurfaceAction::Splice => Ok(SurfaceOperationObservationV1::Splice {
            pre_sha256: required(&effect.pre_sha256, "pre")?,
            post_sha256: required(&effect.post_sha256, "post")?,
        }),
    }
}

#[cfg(all(test, unix))]
#[path = "../../tests/internal/reconcile.rs"]
mod tests;

fn verify_root_identities(
    target_root: &ConfinedRoot,
    lineage_root: &ConfinedRoot,
) -> Result<(), FsError> {
    target_root.verify_path_identity()?;
    lineage_root.verify_path_identity()
}

fn verify_precondition(
    effect: &Staged<'_>,
    snapshot: Option<&FileSnapshot>,
) -> Result<(), FsError> {
    let actual_sha256 = snapshot.map(|file| ArtifactDigest::hash_bytes(&file.bytes));
    let actual_mode = snapshot.map(|file| file.mode);
    if actual_sha256 != effect.pre_sha256 || actual_mode != effect.pre_mode {
        return Err(FsError::Collision(
            effect.rel_path.clone(),
            "path changed after the transition was staged".into(),
        ));
    }
    Ok(())
}
