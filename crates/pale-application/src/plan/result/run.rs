//! Execution-workspace outcome aggregation from typed target evidence.

use pale_core::records::{
    InstanceDigest, LogicalRepositoryIdV1, OneShotCompilationDigest, PlanKind, ProfileDigest,
    ReceiptDigest, RunResultPreimageV1, RunResultV1, RunTargetResult, SchemaRegistryIdentity,
    RUN_RESULT_V1_SCHEMA_VERSION,
};

use crate::compile::Compilation;
use crate::identity::ProjectIdentity;
use crate::plan::{validate, PlanError};

/// Admitted execution-workspace result with its exact JCS identity.
///
/// The application owns aggregation, validation, and canonicalization. Outer
/// persistence and presentation layers may inspect the JCS bytes. Human-readable
/// `.json` persistence derives file bytes exactly once through
/// [`pale_core::canon::pretty_json_document`]; it does not define a second
/// semantic, digest, or wire encoding.
pub struct AdmittedRunResult {
    record: RunResultV1,
    canonical_bytes: Vec<u8>,
}

impl AdmittedRunResult {
    /// Borrow the admitted typed run-result record.
    #[must_use]
    pub const fn record(&self) -> &RunResultV1 {
        &self.record
    }

    /// Exact JCS semantic, digest, and wire identity of the admitted run result.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    /// Aggregate effect outcome established by the admitted target evidence.
    #[must_use]
    pub fn outcome(&self) -> pale_core::records::Outcome {
        self.record.outcome()
    }

    /// Number of target rows admitted into the result.
    #[must_use]
    pub fn target_count(&self) -> usize {
        self.record.targets.len()
    }
}

/// Compilation coordinates carried only by receipt-backed successful targets.
struct SuccessBinding {
    plan_kind: PlanKind,
    profile_digest: ProfileDigest,
    instance_digest: InstanceDigest,
    schema_registry: SchemaRegistryIdentity,
    one_shot_compilation_digest: OneShotCompilationDigest,
}

/// One target outcome admitted for aggregation into an execution-workspace result.
#[must_use = "include target evidence in the execution-workspace run result"]
pub struct RunTargetEvidence {
    target: RunTargetResult,
    success_binding: Option<SuccessBinding>,
}

impl RunTargetEvidence {
    /// Bind a successful target only to its exact receipt and compilation coordinates.
    pub(super) fn succeeded(
        repository_id: LogicalRepositoryIdV1,
        receipt_digest: ReceiptDigest,
        plan_kind: PlanKind,
        profile_digest: ProfileDigest,
        instance_digest: InstanceDigest,
        schema_registry: SchemaRegistryIdentity,
        one_shot_compilation_digest: OneShotCompilationDigest,
    ) -> Self {
        Self {
            target: RunTargetResult::Succeeded {
                repository_id,
                receipt_digest,
            },
            success_binding: Some(SuccessBinding {
                plan_kind,
                profile_digest,
                instance_digest,
                schema_registry,
                one_shot_compilation_digest,
            }),
        }
    }

    /// Attest that a target transaction left no effect.
    ///
    /// This boundary validates only the target identity. The executor may call
    /// it only after its own transaction or recovery mechanism has established
    /// the no-effect outcome; this type does not recreate that external proof.
    ///
    pub fn attest_failed_no_effect(repository_id: LogicalRepositoryIdV1) -> Self {
        Self {
            target: RunTargetResult::FailedNoEffect { repository_id },
            success_binding: None,
        }
    }
}

/// Build one execution-workspace result from an ordered repository prefix.
///
/// Target rows must follow the profile's repository order. Success carries a
/// receipt; one final failed row may terminate the prefix without a receipt.
/// An all-success result must cover every repository in the profile.
///
/// # Errors
///
/// Returns [`PlanError`] when compilation identity, target order, receipt
/// binding, outcome aggregation, or the resulting run record is invalid.
pub fn build_run_result(
    compilation: &Compilation,
    plan_kind: PlanKind,
    evidence: Vec<RunTargetEvidence>,
) -> Result<AdmittedRunResult, PlanError> {
    compilation
        .verify_admitted_compilation()
        .map_err(PlanError::from)?;
    let profile = compilation.profile();
    let instance = compilation.instance();
    let identity = ProjectIdentity::verify(profile, instance).map_err(|source| {
        PlanError::InvalidIdentity {
            source: Box::new(source),
        }
    })?;
    if evidence.is_empty() {
        return Err(PlanError::InvalidRunResult(
            "an execution must report at least one repository".to_string(),
        ));
    }
    if evidence.len() > profile.repositories.len() {
        return Err(PlanError::InvalidRunResult(
            "target count exceeds the compiled repository set".to_string(),
        ));
    }
    let target_count = evidence.len();
    for (index, target_evidence) in evidence.iter().enumerate() {
        let target = &target_evidence.target;
        let expected = &profile.repositories[index].id;
        if target.repository_id() != expected {
            return Err(PlanError::InvalidRunResult(format!(
                "target {} is `{}`, expected `{expected}`",
                index,
                target.repository_id()
            )));
        }
        match (&target_evidence.success_binding, target) {
            (Some(binding), RunTargetResult::Succeeded { .. })
                if binding.plan_kind == plan_kind
                    && &binding.profile_digest == identity.profile()
                    && &binding.instance_digest == identity.instance()
                    && &binding.schema_registry == compilation.schema_registry_identity()
                    && &binding.one_shot_compilation_digest
                        == compilation.one_shot_compilation_digest() => {}
            (None, RunTargetResult::FailedNoEffect { .. }) if index + 1 == target_count => {}
            (Some(_), RunTargetResult::Succeeded { .. }) => {
                return Err(PlanError::InvalidRunResult(format!(
                    "successful target `{expected}` receipt belongs to another plan class or project identity"
                )));
            }
            (None, RunTargetResult::FailedNoEffect { .. }) => {
                return Err(PlanError::InvalidRunResult(format!(
                    "failed target `{expected}` must be the receipt-less final row"
                )));
            }
            _ => {
                return Err(PlanError::InvalidRunResult(format!(
                    "target `{expected}` has an impossible evidence and outcome combination"
                )));
            }
        }
    }
    let targets = evidence
        .into_iter()
        .map(|evidence| evidence.target)
        .collect::<Vec<_>>();
    if targets
        .iter()
        .all(|target| matches!(target, RunTargetResult::Succeeded { .. }))
        && targets.len() != profile.repositories.len()
    {
        return Err(PlanError::InvalidRunResult(
            "an all-success result must cover every compiled repository".to_string(),
        ));
    }
    let schema_identity = pale_contracts::record_schema_identity("run-result.v1")?;
    let result = RunResultV1::seal(RunResultPreimageV1 {
        schema: schema_identity.schema,
        schema_sha256: schema_identity.schema_sha256,
        schema_version: RUN_RESULT_V1_SCHEMA_VERSION,
        schema_manifest_sha256: compilation
            .schema_registry_identity()
            .schema_manifest_sha256
            .clone(),
        schema_set_sha256: compilation
            .schema_registry_identity()
            .schema_set_sha256
            .clone(),
        one_shot_compilation_digest: compilation.one_shot_compilation_digest().clone(),
        project_id: profile.project_id.clone(),
        execution_workspace_id: profile.execution_workspace_id.clone(),
        plan_kind,
        profile_digest: identity.profile().clone(),
        instance_digest: identity.instance().clone(),
        targets,
    })?;
    validate(&result, "run-result.v1")?;
    let canonical_bytes = pale_core::canon::to_canonical_string(&result)?.into_bytes();
    Ok(AdmittedRunResult {
        record: result,
        canonical_bytes,
    })
}
