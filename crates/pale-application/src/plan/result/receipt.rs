//! Construction and admission of exact successful materialization receipts.
//!
//! [`AdmittedSuccessReceipt`] is available only after plan identity, lineage,
//! operation results, and effect observations agree. It attests this bounded
//! machine-local filesystem transaction. It does not attest installation,
//! Pale Runtime activation, or an application-native or business effect.

use std::collections::BTreeMap;

use pale_core::admission::AdmittedPlan;
use pale_core::records::{
    AgentSurfacePlanV1, BaselineStateV1, MachineLocalEffectObservationV1,
    MachineLocalEffectTransitionV1, MachineLocalReceiptOperationV1, MaterializationLineageDigest,
    MaterializationReceiptPreimageV1, MaterializationReceiptTransitionV1, MaterializationReceiptV1,
    MaterializationStateTransitionV1, ReceiptDigest, ReceiptOperation,
    SurfaceCreationObservationV1, SurfaceOperationObservationV1, SurfaceOperationTransitionV1,
    SurfaceOwnership, UnchangedDispositionV1, MATERIALIZATION_RECEIPT_V1_SCHEMA_VERSION,
};

use super::run::RunTargetEvidence;
use crate::plan::{validate, PlanError};

/// Validate an unchanged observation without conflating owned-region and file digests.
///
/// [`SurfaceOwnership::WholeFile`] uses one digest domain: `owned_sha256`, the
/// baseline, and the adapter observation all identify the complete file. A
/// [`SurfaceOwnership::ManagedBlock`] transition instead identifies only the
/// owned block, while the baseline and [`SurfaceOperationObservationV1`] identify
/// the complete user-owned file. The filesystem adapter authenticates that block
/// before it can produce the observation, so receipt construction compares the
/// two whole-file observations here without duplicating adapter admission.
fn unchanged_observation_matches(
    ownership: SurfaceOwnership,
    owned_sha256: &pale_core::records::ArtifactDigest,
    baseline_sha256: &pale_core::records::ArtifactDigest,
    observed_sha256: &pale_core::records::ArtifactDigest,
    disposition: UnchangedDispositionV1,
    expected_disposition: UnchangedDispositionV1,
) -> bool {
    observed_sha256 == baseline_sha256
        && disposition == expected_disposition
        && (ownership == SurfaceOwnership::ManagedBlock || owned_sha256 == baseline_sha256)
}

/// Success receipt admitted from one exact materialization plan and its observed effects.
///
/// The record can be borrowed for durable local receipt persistence. Consuming
/// this proof is the only way to create a successful execution-workspace target.
#[must_use = "persist the admitted receipt before consuming it into execution-workspace evidence"]
pub struct AdmittedSuccessReceipt {
    record: MaterializationReceiptV1,
    receipt_digest: ReceiptDigest,
    canonical_bytes: Vec<u8>,
}

impl AdmittedSuccessReceipt {
    /// Immutable record to persist before consuming this proof into a run target.
    pub fn record(&self) -> &MaterializationReceiptV1 {
        &self.record
    }

    /// Exact JCS bytes of the admitted immutable receipt.
    ///
    /// These remain the semantic, digest, and wire identity. Human-readable
    /// `.json` persistence derives file bytes exactly once through
    /// [`pale_core::canon::pretty_json_document`] and uses that exact projection
    /// for publication and idempotence; adapters do not reserialize the record.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    /// Bind this admitted receipt into one successful execution-workspace target.
    pub fn into_run_target(self) -> RunTargetEvidence {
        RunTargetEvidence::succeeded(
            self.record.repository_id,
            self.receipt_digest,
            self.record.transition.plan_kind(),
            self.record.profile_digest,
            self.record.instance_digest,
            pale_core::records::SchemaRegistryIdentity {
                schema_manifest_sha256: self.record.schema_manifest_sha256,
                schema_set_sha256: self.record.schema_set_sha256,
            },
            self.record.one_shot_compilation_digest,
        )
    }
}

/// Exact machine-local lineage transition observed by a successful transaction.
///
/// The variants mirror the four legal ownership-state transitions. The receipt
/// builder additionally requires the selected variant to match the admitted
/// plan, so evidence from another transition class cannot be rebound.
#[derive(Debug)]
pub enum MaterializationLineageEvidence {
    /// First apply created both ownership state and its lineage anchor.
    Initialized {
        /// Digest of the newly persisted lineage record.
        post: MaterializationLineageDigest,
    },
    /// A later apply advanced an existing lineage anchor.
    Advanced {
        /// Digest of the lineage record admitted before mutation.
        prior: MaterializationLineageDigest,
        /// Digest of the lineage record persisted after mutation.
        post: MaterializationLineageDigest,
    },
    /// Removal retired an existing ownership state and lineage anchor.
    Removed {
        /// Digest of the lineage record admitted before removal.
        prior: MaterializationLineageDigest,
    },
    /// Removal observed no prior owned state and therefore no lineage anchor.
    NoOwnedState,
}

impl MaterializationLineageEvidence {
    /// Classify the exact lineage digests observed around one target mutation.
    ///
    /// This constructor establishes only the four-state lineage shape. Receipt
    /// admission later verifies that the shape matches the admitted plan's
    /// initialize, advance, remove, or remove-unowned transition.
    #[must_use]
    pub fn from_observation(
        prior: Option<&MaterializationLineageDigest>,
        post: Option<&MaterializationLineageDigest>,
    ) -> Self {
        match (prior, post) {
            (None, Some(post)) => Self::Initialized { post: post.clone() },
            (Some(prior), Some(post)) => Self::Advanced {
                prior: prior.clone(),
                post: post.clone(),
            },
            (Some(prior), None) => Self::Removed {
                prior: prior.clone(),
            },
            (None, None) => Self::NoOwnedState,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Self::Initialized { .. } => "initialized",
            Self::Advanced { .. } => "advanced",
            Self::Removed { .. } => "removed",
            Self::NoOwnedState => "no-owned-state",
        }
    }

    fn admit_for(
        self,
        plan: &AgentSurfacePlanV1,
    ) -> Result<MaterializationReceiptTransitionV1, PlanError> {
        let expected = match &plan.transition {
            MaterializationStateTransitionV1::Initialize { .. } => "initialized",
            MaterializationStateTransitionV1::Advance { .. } => "advanced",
            MaterializationStateTransitionV1::Remove { .. } => "removed",
            MaterializationStateTransitionV1::RemoveUnowned {} => "no-owned-state",
        };
        let actual = self.label();
        match (&plan.transition, self) {
            (
                MaterializationStateTransitionV1::Initialize { next_state },
                Self::Initialized { post },
            ) => Ok(MaterializationReceiptTransitionV1::Initialized {
                post_state_digest: next_state.state_digest.clone(),
                post_lineage_digest: post,
            }),
            (
                MaterializationStateTransitionV1::Advance { prior, next_state },
                Self::Advanced {
                    prior: prior_lineage_digest,
                    post: post_lineage_digest,
                },
            ) => Ok(MaterializationReceiptTransitionV1::Advanced {
                prior_state_digest: prior.state_digest.clone(),
                post_state_digest: next_state.state_digest.clone(),
                prior_lineage_digest,
                post_lineage_digest,
            }),
            (
                MaterializationStateTransitionV1::Remove { prior },
                Self::Removed {
                    prior: prior_lineage_digest,
                },
            ) => Ok(MaterializationReceiptTransitionV1::Removed {
                prior_state_digest: prior.state_digest.clone(),
                prior_lineage_digest,
            }),
            (MaterializationStateTransitionV1::RemoveUnowned {}, Self::NoOwnedState) => {
                Ok(MaterializationReceiptTransitionV1::NoOwnedState {})
            }
            _ => Err(PlanError::InvalidLineageTransition { expected, actual }),
        }
    }
}

/// Match every observed surface result to the admitted operation and content digest.
fn validate_success_operations(
    plan: &AgentSurfacePlanV1,
    observed: &[ReceiptOperation],
) -> Result<(), PlanError> {
    if observed.len() != plan.operations.len() {
        return Err(PlanError::InvalidReceipt(format!(
            "observed {} surface operations for {} planned effects",
            observed.len(),
            plan.operations.len()
        )));
    }
    let baseline = plan
        .target_baseline
        .iter()
        .map(|entry| (entry.rel_path.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    for (planned, actual) in plan.operations.iter().zip(observed) {
        if actual.rel_path != planned.rel_path || actual.mode != planned.mode {
            return Err(PlanError::InvalidReceipt(format!(
                "operation evidence for `{}` does not match its planned path and mode",
                planned.rel_path
            )));
        }
        let before = baseline.get(planned.rel_path.as_str()).ok_or_else(|| {
            PlanError::InvalidReceipt(format!(
                "operation `{}` has no admitted baseline",
                planned.rel_path
            ))
        })?;
        let valid = match (
            &planned.transition,
            &before.observation,
            &actual.observation,
        ) {
            (
                SurfaceOperationTransitionV1::Create {
                    ownership,
                    desired_sha256,
                },
                BaselineStateV1::Absent {},
                SurfaceOperationObservationV1::Create {
                    target: SurfaceCreationObservationV1::AbsentFile { post_sha256 },
                },
            ) => *ownership == SurfaceOwnership::ManagedBlock || post_sha256 == desired_sha256,
            (
                SurfaceOperationTransitionV1::Create {
                    ownership: SurfaceOwnership::ManagedBlock,
                    ..
                },
                BaselineStateV1::Present { sha256, .. },
                SurfaceOperationObservationV1::Create {
                    target:
                        SurfaceCreationObservationV1::ExistingFile {
                            pre_sha256,
                            post_sha256: _,
                        },
                },
            ) => pre_sha256 == sha256,
            (
                SurfaceOperationTransitionV1::Adopt {
                    ownership,
                    desired_sha256,
                },
                BaselineStateV1::Present { sha256, .. },
                SurfaceOperationObservationV1::Adopt {
                    pre_sha256,
                    post_sha256,
                },
            ) => {
                pre_sha256 == sha256
                    && (*ownership == SurfaceOwnership::ManagedBlock
                        || post_sha256 == desired_sha256)
            }
            (
                SurfaceOperationTransitionV1::Update {
                    ownership,
                    desired_sha256,
                },
                BaselineStateV1::Present { sha256, .. },
                SurfaceOperationObservationV1::Update {
                    pre_sha256,
                    post_sha256,
                },
            ) => {
                pre_sha256 == sha256
                    && (*ownership == SurfaceOwnership::ManagedBlock
                        || post_sha256 == desired_sha256)
            }
            (
                SurfaceOperationTransitionV1::Unchanged {
                    ownership,
                    owned_sha256,
                },
                BaselineStateV1::Present { sha256, mode },
                SurfaceOperationObservationV1::Unchanged {
                    sha256: observed_sha256,
                    disposition,
                },
            ) => {
                let expected = if *mode == planned.mode {
                    UnchangedDispositionV1::Verified
                } else {
                    UnchangedDispositionV1::ModeRepaired
                };
                unchanged_observation_matches(
                    *ownership,
                    owned_sha256,
                    sha256,
                    observed_sha256,
                    *disposition,
                    expected,
                )
            }
            (
                SurfaceOperationTransitionV1::Delete { prior_owned_sha256 },
                BaselineStateV1::Present { sha256, .. },
                SurfaceOperationObservationV1::Delete { pre_sha256 },
            ) => pre_sha256 == sha256 && prior_owned_sha256 == sha256,
            (
                SurfaceOperationTransitionV1::Splice { .. },
                BaselineStateV1::Present { sha256, .. },
                SurfaceOperationObservationV1::Splice { pre_sha256, .. },
            ) => pre_sha256 == sha256,
            _ => false,
        };
        if !valid {
            return Err(PlanError::InvalidReceipt(format!(
                "operation `{}` evidence is impossible for its planned transition and baseline",
                planned.rel_path
            )));
        }
    }
    Ok(())
}

/// Match machine-local observations to the plan's exact ordered effect set.
///
/// This verifies transaction evidence only; it does not perform or repeat an effect.
fn validate_machine_local_results(
    admitted_plan: &AdmittedPlan,
    observed: &[MachineLocalReceiptOperationV1],
) -> Result<(), PlanError> {
    let planned = &admitted_plan.record().machine_local_effects;
    if observed.len() != planned.len() {
        return Err(PlanError::InvalidReceipt(format!(
            "observed {} machine-local effects for {} planned effects",
            observed.len(),
            planned.len()
        )));
    }
    for (planned, actual) in planned.iter().zip(observed) {
        if actual.kind != planned.kind
            || actual.root != planned.root
            || actual.rel_path != planned.rel_path
            || actual.mode != planned.transition.mode()
        {
            return Err(PlanError::InvalidReceipt(format!(
                "machine-local evidence for `{}` differs from its admitted effect",
                planned.rel_path
            )));
        }
        let valid = match (&planned.transition, &actual.observation) {
            (
                MachineLocalEffectTransitionV1::Create {
                    post,
                    create_directories,
                },
                MachineLocalEffectObservationV1::Create {
                    post: observed,
                    created_directories,
                },
            ) => post.sha256 == observed.sha256 && create_directories == created_directories,
            (
                MachineLocalEffectTransitionV1::Update { prior, post },
                MachineLocalEffectObservationV1::Update {
                    pre_sha256,
                    post: observed,
                },
            )
            | (
                MachineLocalEffectTransitionV1::Splice { prior, post },
                MachineLocalEffectObservationV1::Splice {
                    pre_sha256,
                    post: observed,
                },
            ) => prior.sha256 == *pre_sha256 && post.sha256 == observed.sha256,
            (
                MachineLocalEffectTransitionV1::Unchanged { state },
                MachineLocalEffectObservationV1::Unchanged { state: observed },
            ) => state.sha256 == observed.sha256 && state.identity == observed.identity,
            (
                MachineLocalEffectTransitionV1::Delete { prior },
                MachineLocalEffectObservationV1::Delete { pre_sha256 },
            ) => prior.sha256 == *pre_sha256,
            _ => false,
        };
        if !valid {
            return Err(PlanError::InvalidReceipt(format!(
                "machine-local evidence for `{}` is impossible for its admitted transition",
                planned.rel_path
            )));
        }
    }
    Ok(())
}

/// Build the immutable success receipt for one admitted target transaction.
///
/// Failed targets have no receipt and remain explicit failed rows in the
/// execution-workspace run result. Requiring [`AdmittedPlan`] prevents an unverified or
/// stale plan from becoming receipt evidence.
///
/// # Errors
///
/// Returns [`PlanError`] when plan identity, operation evidence, machine-local
/// evidence, lineage, schema identity, or the resulting receipt is invalid.
pub fn build_success_receipt_with_lineage(
    admitted_plan: &AdmittedPlan,
    surface_operations: Vec<ReceiptOperation>,
    machine_local_effects: Vec<MachineLocalReceiptOperationV1>,
    lineage: MaterializationLineageEvidence,
) -> Result<AdmittedSuccessReceipt, PlanError> {
    let plan = admitted_plan.plan();
    validate_success_operations(plan, &surface_operations)?;
    validate_machine_local_results(admitted_plan, &machine_local_effects)?;
    let transition = lineage.admit_for(plan)?;
    let surface_plan_digest = plan.plan_digest.clone();
    let schema_identity = pale_contracts::record_schema_identity("receipt.v1")?;
    let receipt = MaterializationReceiptV1::seal(MaterializationReceiptPreimageV1 {
        schema: schema_identity.schema,
        schema_sha256: schema_identity.schema_sha256,
        schema_version: MATERIALIZATION_RECEIPT_V1_SCHEMA_VERSION,
        schema_manifest_sha256: plan.schema_manifest_sha256.clone(),
        schema_set_sha256: plan.schema_set_sha256.clone(),
        one_shot_compilation_digest: plan.one_shot_compilation_digest.clone(),
        repository_id: plan.repository_id.clone(),
        transition,
        profile_digest: plan.profile_digest.clone(),
        instance_digest: plan.instance_digest.clone(),
        manifest_digest: plan.manifest_digest.clone(),
        package_closure: plan.package_closure.clone(),
        normative_pack_closure: plan.normative_pack_closure.clone(),
        package_semantic_ir_digest: plan.package_semantic_ir_digest.clone(),
        surface_plan_digest,
        plan_digest: admitted_plan.digest().clone(),
        surface_operations,
        machine_local_effects,
        deferred_effects: plan.deferred_effects.clone(),
    })?;
    let receipt_digest = receipt.receipt_digest.clone();
    validate(&receipt, "receipt.v1")?;
    let canonical_bytes = pale_core::canon::to_canonical_string(&receipt)?.into_bytes();
    Ok(AdmittedSuccessReceipt {
        record: receipt,
        receipt_digest,
        canonical_bytes,
    })
}

#[cfg(test)]
#[path = "../../../tests/internal/plan/result/receipt.rs"]
mod tests;
