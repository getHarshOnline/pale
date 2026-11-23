//! Private evidence persistence and repository settlement mechanics.

use std::collections::BTreeSet;
use std::time::Duration;

use pale_application::plan::{
    MaterializationLineageEvidence, MaterializationSettlement, ReceiptPublicationOutcome,
    RunTargetEvidence,
};
use pale_core::records::{MachineLocalReceiptOperationV1, PlanKind, ReceiptOperation};

use super::{
    AuxiliaryEffectObservation, LocalMaterializationError, LocalMaterializationReceiptObservation,
    PreparationFailure, RECEIPT_MODE,
};

/// Maximum directory entries inspected while reconstructing local evidence.
const MAX_EVIDENCE_DIRECTORY_ENTRIES: usize = 4_096;
/// Maximum bytes admitted from one local evidence record.
const MAX_EVIDENCE_RECORD_BYTES: usize = 8 * 1024 * 1024;
/// Repository-relative advisory-lock name for local evidence publication.
const EVIDENCE_LOCK_PATH: &str = ".pale-receipts.lock";
/// Maximum wait for exclusive local evidence-publication custody.
const EVIDENCE_LOCK_TIMEOUT: Duration = Duration::from_secs(2);

pub(super) struct MutationEvidenceRun {
    root: pale_adapter_fs::ConfinedRoot,
    _lock: pale_adapter_fs::AdvisoryLock,
    kind: PlanKind,
    receipt_paths: BTreeSet<String>,
}

#[derive(Debug)]
struct ReceiptPublicationFailure {
    kind: ReceiptPublicationFailureKind,
    error: Box<LocalMaterializationError>,
    proposed: Option<Vec<u8>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReceiptPublicationFailureKind {
    Absent,
    Collision,
    Uncertain,
}

pub(super) struct UnknownTargetFailure {
    pub(super) error: Box<LocalMaterializationError>,
    pub(super) expected_receipt: Option<Vec<u8>>,
}

pub(super) enum SettledTarget {
    Succeeded(RunTargetEvidence),
    FailedNoEffect(
        RunTargetEvidence,
        LocalMaterializationError,
        AuxiliaryEffectObservation,
    ),
}

impl MutationEvidenceRun {
    pub(super) fn begin<'a>(
        root: pale_adapter_fs::ConfinedRoot,
        kind: PlanKind,
        plans: impl IntoIterator<Item = &'a pale_core::admission::AdmittedPlan>,
        mut observation: AuxiliaryEffectObservation,
    ) -> Result<Self, PreparationFailure> {
        let mut receipt_paths = BTreeSet::new();
        for plan in plans {
            if plan.kind() != kind || !receipt_paths.insert(receipt_path(plan)) {
                return Err(PreparationFailure::new(
                    LocalMaterializationError::InternalInvariant(
                        "duplicate or wrong-class receipt reservation",
                    ),
                    observation,
                ));
            }
        }
        let (lock, lock_effect) = pale_adapter_fs::acquire_advisory_lock(
            &root,
            EVIDENCE_LOCK_PATH,
            EVIDENCE_LOCK_TIMEOUT,
            "materialization evidence",
        )
        .map_err(|failure| {
            let effect = match failure.effect() {
                pale_adapter_fs::AdvisoryLockEffect::Untouched => {
                    AuxiliaryEffectObservation::Untouched
                }
                pale_adapter_fs::AdvisoryLockEffect::Confirmed => {
                    AuxiliaryEffectObservation::Confirmed
                }
                pale_adapter_fs::AdvisoryLockEffect::OutcomeUnknown => {
                    AuxiliaryEffectObservation::OutcomeUnknown
                }
            };
            PreparationFailure::new(
                LocalMaterializationError::Filesystem {
                    operation: "acquire evidence lock",
                    source: failure.into_source(),
                },
                observation.combine(effect),
            )
        })?;
        observation = observation.combine(match lock_effect {
            pale_adapter_fs::AdvisoryLockEffect::Untouched => AuxiliaryEffectObservation::Untouched,
            pale_adapter_fs::AdvisoryLockEffect::Confirmed => AuxiliaryEffectObservation::Confirmed,
            pale_adapter_fs::AdvisoryLockEffect::OutcomeUnknown => {
                unreachable!("successful lock admission cannot have an unknown effect")
            }
        });
        let mut additional_entries = 0_usize;
        for relative in receipt_paths
            .iter()
            .map(String::as_str)
            .chain(std::iter::once(run_result_path(kind)))
        {
            let absent = root
                .location(relative)
                .and_then(|location| {
                    location
                        .inspect_file(pale_adapter_fs::ReadLimit::new(MAX_EVIDENCE_RECORD_BYTES))
                })
                .map_err(|source| LocalMaterializationError::Filesystem {
                    operation: "inspect evidence slot",
                    source,
                })
                .map_err(|error| PreparationFailure::new(error, observation))?
                .is_none();
            additional_entries = additional_entries
                .checked_add(usize::from(absent))
                .ok_or_else(|| {
                    PreparationFailure::new(
                        LocalMaterializationError::InternalInvariant(
                            "evidence reservation count overflow",
                        ),
                        observation,
                    )
                })?;
        }
        let observed = root
            .entry_count(pale_adapter_fs::DirectoryLimit::new(
                MAX_EVIDENCE_DIRECTORY_ENTRIES.saturating_add(1),
            ))
            .map_err(|source| LocalMaterializationError::Filesystem {
                operation: "inspect evidence directory capacity",
                source,
            })
            .map_err(|error| PreparationFailure::new(error, observation))?;
        if observed.saturating_add(additional_entries) > MAX_EVIDENCE_DIRECTORY_ENTRIES {
            return Err(PreparationFailure::new(
                LocalMaterializationError::EvidenceCapacity,
                observation,
            ));
        }
        let invalidation = root
            .location(run_result_path(kind))
            .and_then(|location| location.remove_file())
            .map_err(|source| LocalMaterializationError::Filesystem {
                operation: "invalidate prior run result",
                source,
            });
        match invalidation {
            Ok(true | false) => {}
            Err(error) => {
                let effect = match &error {
                    LocalMaterializationError::Filesystem { source, .. }
                        if source.effect_visibility()
                            == pale_adapter_fs::EffectVisibility::MayBeVisible =>
                    {
                        AuxiliaryEffectObservation::OutcomeUnknown
                    }
                    _ => AuxiliaryEffectObservation::Untouched,
                };
                return Err(PreparationFailure::new(error, observation.combine(effect)));
            }
        }
        Ok(Self {
            root,
            _lock: lock,
            kind,
            receipt_paths,
        })
    }

    fn persist_receipt(
        &self,
        admitted: &pale_core::admission::AdmittedPlan,
        operations: Vec<ReceiptOperation>,
        machine_local_effects: Vec<MachineLocalReceiptOperationV1>,
        lineage: MaterializationLineageEvidence,
    ) -> Result<RunTargetEvidence, ReceiptPublicationFailure> {
        let receipt = pale_application::plan::build_success_receipt_with_lineage(
            admitted,
            operations,
            machine_local_effects,
            lineage,
        )
        .map_err(LocalMaterializationError::Plan)
        .map_err(|error| ReceiptPublicationFailure {
            kind: ReceiptPublicationFailureKind::Absent,
            error: Box::new(error),
            proposed: None,
        })?;
        let receipt_file_bytes = pale_core::canon::pretty_json_document(receipt.canonical_bytes())
            .map_err(LocalMaterializationError::Canonical)
            .map_err(|error| ReceiptPublicationFailure {
                kind: ReceiptPublicationFailureKind::Absent,
                error: Box::new(error),
                proposed: None,
            })?;
        let relative = receipt_path(admitted);
        if !self.receipt_paths.contains(&relative) {
            return Err(ReceiptPublicationFailure {
                kind: ReceiptPublicationFailureKind::Absent,
                error: Box::new(LocalMaterializationError::InternalInvariant(
                    "receipt slot was not pre-admitted",
                )),
                proposed: Some(receipt_file_bytes),
            });
        }
        self.publish_immutable(&relative, &receipt_file_bytes)?;
        Ok(receipt.into_run_target())
    }

    fn publish_immutable(
        &self,
        relative: &str,
        bytes: &[u8],
    ) -> Result<(), ReceiptPublicationFailure> {
        if bytes.len() > MAX_EVIDENCE_RECORD_BYTES {
            return Err(ReceiptPublicationFailure {
                kind: ReceiptPublicationFailureKind::Absent,
                error: Box::new(LocalMaterializationError::InternalInvariant(
                    "receipt exceeds its byte ceiling",
                )),
                proposed: Some(bytes.to_vec()),
            });
        }
        let location = self
            .root
            .location(relative)
            .map_err(|source| LocalMaterializationError::Filesystem {
                operation: "resolve immutable receipt slot",
                source,
            })
            .map_err(|error| ReceiptPublicationFailure {
                kind: ReceiptPublicationFailureKind::Absent,
                error: Box::new(error),
                proposed: Some(bytes.to_vec()),
            })?;
        let prior = location
            .read_file(pale_adapter_fs::ReadLimit::new(MAX_EVIDENCE_RECORD_BYTES))
            .map_err(|source| LocalMaterializationError::Filesystem {
                operation: "read immutable receipt slot",
                source,
            })
            .map_err(|error| ReceiptPublicationFailure {
                kind: ReceiptPublicationFailureKind::Uncertain,
                error: Box::new(error),
                proposed: Some(bytes.to_vec()),
            })?;
        if let Some(existing) = prior {
            return exact_receipt_or_collision(&existing, bytes);
        }
        match location.write_atomic_noclobber(bytes, RECEIPT_MODE) {
            Ok(()) => Ok(()),
            Err(publication) => reconcile_failed_publication(&location, bytes, publication),
        }
    }

    pub(super) fn publish_run_summary(
        &self,
        bytes: &[u8],
    ) -> Result<(), LocalMaterializationError> {
        let file_bytes = pale_core::canon::pretty_json_document(bytes)
            .map_err(LocalMaterializationError::Canonical)?;
        if file_bytes.len() > MAX_EVIDENCE_RECORD_BYTES {
            return Err(LocalMaterializationError::InternalInvariant(
                "run result exceeds its byte ceiling",
            ));
        }
        self.root
            .location(run_result_path(self.kind))
            .and_then(|location| location.write_atomic(&file_bytes, RECEIPT_MODE))
            .map_err(|source| LocalMaterializationError::Filesystem {
                operation: "publish run result",
                source,
            })
    }

    pub(super) fn observe_receipt(
        &self,
        admitted: &pale_core::admission::AdmittedPlan,
        expected: &[u8],
    ) -> Result<LocalMaterializationReceiptObservation, LocalMaterializationError> {
        let relative = receipt_path(admitted);
        if !self.receipt_paths.contains(&relative) {
            return Err(LocalMaterializationError::InternalInvariant(
                "reconciliation receipt slot was not admitted",
            ));
        }
        let existing = self
            .root
            .location(&relative)
            .and_then(|location| {
                location.read_file(pale_adapter_fs::ReadLimit::new(MAX_EVIDENCE_RECORD_BYTES))
            })
            .map_err(|source| LocalMaterializationError::Filesystem {
                operation: "read reconciliation receipt",
                source,
            })?;
        Ok(match existing {
            None => LocalMaterializationReceiptObservation::Absent,
            Some(existing) if existing.bytes == expected && existing.mode == RECEIPT_MODE => {
                LocalMaterializationReceiptObservation::Exact
            }
            Some(_) => LocalMaterializationReceiptObservation::Different,
        })
    }
}

pub(super) fn settle_target(
    evidence: &MutationEvidenceRun,
    admitted: &pale_core::admission::AdmittedPlan,
    transaction: pale_adapter_fs::AppliedFileTransaction,
    local_effect: Option<pale_adapter_fs::AppliedMachineLocalFile>,
) -> Result<SettledTarget, UnknownTargetFailure> {
    let coordination_effect = transaction.coordination_effect().into();
    let observation = transaction.observation();
    let operations = observation.operations.clone();
    let lineage = MaterializationLineageEvidence::from_observation(
        observation.prior_lineage_digest.as_ref(),
        observation.post_lineage_digest.as_ref(),
    );
    let machine_local_effects = local_effect
        .as_ref()
        .map(|effect| vec![effect.observation().clone()])
        .unwrap_or_default();
    let publication =
        evidence.persist_receipt(admitted, operations, machine_local_effects, lineage);
    let observed = match &publication {
        Ok(_) => ReceiptPublicationOutcome::Published,
        Err(failure) => match failure.kind {
            ReceiptPublicationFailureKind::Absent => ReceiptPublicationOutcome::Absent,
            ReceiptPublicationFailureKind::Collision => ReceiptPublicationOutcome::Collision,
            ReceiptPublicationFailureKind::Uncertain => ReceiptPublicationOutcome::Uncertain,
        },
    };
    match pale_application::plan::decide_materialization_settlement(observed) {
        MaterializationSettlement::Commit => {
            if let Some(local_effect) = local_effect {
                local_effect.commit();
            }
            pale_adapter_fs::commit(transaction);
            let Ok(evidence) = publication else {
                unreachable!("only published receipt selects commit")
            };
            Ok(SettledTarget::Succeeded(evidence))
        }
        MaterializationSettlement::RollbackNoEffect => {
            let failure = match publication {
                Err(failure)
                    if matches!(
                        failure.kind,
                        ReceiptPublicationFailureKind::Absent
                            | ReceiptPublicationFailureKind::Collision
                    ) =>
                {
                    failure
                }
                Ok(_) | Err(_) => {
                    unreachable!("only absent or colliding publication selects rollback")
                }
            };
            restore_effects(transaction, local_effect).map_err(|error| UnknownTargetFailure {
                error: Box::new(error),
                expected_receipt: failure.proposed.clone(),
            })?;
            Ok(SettledTarget::FailedNoEffect(
                RunTargetEvidence::attest_failed_no_effect(admitted.plan().repository_id.clone()),
                *failure.error,
                coordination_effect,
            ))
        }
        MaterializationSettlement::RetainForReconciliation => {
            let failure = match publication {
                Err(failure) if failure.kind == ReceiptPublicationFailureKind::Uncertain => failure,
                Ok(_) | Err(_) => unreachable!("only uncertain publication retains post-state"),
            };
            if let Some(local_effect) = local_effect {
                local_effect.commit();
            }
            pale_adapter_fs::commit(transaction);
            Err(UnknownTargetFailure {
                error: failure.error,
                expected_receipt: failure.proposed,
            })
        }
    }
}

/// Restore admitted repository materialization after a machine-local failure.
/// Persistent apply-lock coordination state is outside this rollback set and is
/// retained separately by the caller's transaction observation.
pub(super) fn rollback_after_local_effect_failure(
    transaction: pale_adapter_fs::AppliedFileTransaction,
    local_error: pale_adapter_fs::FsError,
) -> Result<LocalMaterializationError, LocalMaterializationError> {
    let coordination_effect = transaction.coordination_effect();
    let local_unknown = matches!(local_error, pale_adapter_fs::FsError::RestoreFailed { .. });
    let surface_restore = pale_adapter_fs::rollback(transaction);
    if local_unknown || surface_restore.is_err() {
        return Err(LocalMaterializationError::Filesystem {
            operation: "restore repository after machine-local effect failure",
            source: surface_restore
                .err()
                .unwrap_or(local_error)
                .with_coordination_effect(coordination_effect),
        });
    }
    Ok(LocalMaterializationError::Filesystem {
        operation: "publish machine-local effect",
        source: local_error,
    })
}

/// Restore admitted repository and machine-local materialization state only.
/// Persistent coordination lock files and their parent namespace are not pruned.
fn restore_effects(
    transaction: pale_adapter_fs::AppliedFileTransaction,
    local_effect: Option<pale_adapter_fs::AppliedMachineLocalFile>,
) -> Result<(), LocalMaterializationError> {
    let coordination_effect = transaction.coordination_effect();
    let local_restore = local_effect.map(pale_adapter_fs::AppliedMachineLocalFile::rollback);
    let surface_restore = pale_adapter_fs::rollback(transaction);
    if let Some(Err(source)) = local_restore {
        return Err(LocalMaterializationError::Filesystem {
            operation: "restore machine-local effect",
            source: source.with_coordination_effect(coordination_effect),
        });
    }
    surface_restore.map_err(|source| LocalMaterializationError::Filesystem {
        operation: "restore repository effect",
        source: source.with_coordination_effect(coordination_effect),
    })
}

fn receipt_path(plan: &pale_core::admission::AdmittedPlan) -> String {
    format!(
        "{}.{}.{}.receipt.json",
        plan.plan().repository_id,
        plan.kind(),
        plan.digest()
    )
}

const fn run_result_path(kind: PlanKind) -> &'static str {
    match kind {
        PlanKind::Apply => "run-result.apply.json",
        PlanKind::Remove => "run-result.remove.json",
    }
}

fn exact_receipt_or_collision(
    existing: &pale_adapter_fs::ExistingFile,
    proposed: &[u8],
) -> Result<(), ReceiptPublicationFailure> {
    if existing.bytes == proposed && existing.mode == RECEIPT_MODE {
        Ok(())
    } else {
        Err(ReceiptPublicationFailure {
            kind: ReceiptPublicationFailureKind::Collision,
            error: Box::new(LocalMaterializationError::InternalInvariant(
                "immutable receipt differs from the admitted bytes or mode",
            )),
            proposed: Some(proposed.to_vec()),
        })
    }
}

fn reconcile_failed_publication(
    location: &pale_adapter_fs::ConfinedLocation,
    proposed: &[u8],
    publication: pale_adapter_fs::FsError,
) -> Result<(), ReceiptPublicationFailure> {
    let collision = matches!(publication, pale_adapter_fs::FsError::Collision(_, _));
    match location.read_file(pale_adapter_fs::ReadLimit::new(MAX_EVIDENCE_RECORD_BYTES)) {
        Ok(Some(existing)) if existing.bytes == proposed && existing.mode == RECEIPT_MODE => {
            if collision {
                Ok(())
            } else {
                Err(ReceiptPublicationFailure {
                    kind: ReceiptPublicationFailureKind::Uncertain,
                    error: Box::new(LocalMaterializationError::Filesystem {
                        operation: "durably publish immutable receipt",
                        source: publication,
                    }),
                    proposed: Some(proposed.to_vec()),
                })
            }
        }
        Ok(Some(existing)) => exact_receipt_or_collision(&existing, proposed),
        Ok(None) => Err(ReceiptPublicationFailure {
            kind: ReceiptPublicationFailureKind::Absent,
            error: Box::new(LocalMaterializationError::Filesystem {
                operation: "publish immutable receipt",
                source: publication,
            }),
            proposed: Some(proposed.to_vec()),
        }),
        Err(source) => Err(ReceiptPublicationFailure {
            kind: ReceiptPublicationFailureKind::Uncertain,
            error: Box::new(LocalMaterializationError::Filesystem {
                operation: "reconcile immutable receipt publication",
                source,
            }),
            proposed: Some(proposed.to_vec()),
        }),
    }
}

#[cfg(test)]
#[path = "../../../../tests/internal/local_materialization_settlement.rs"]
mod tests;
