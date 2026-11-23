//! Exact applied-materialization closure and cross-record admission.

use serde::Serialize;

use pale_core::records::{
    LogicalRepositoryIdV1, MaterializationClosureDigest, MaterializationReceiptTransitionV1,
    MATERIALIZATION_LINEAGE_SCHEMA, MATERIALIZATION_LINEAGE_SCHEMA_VERSION,
};

use crate::compile::{Compilation, CompiledRepo};
use crate::plan::desired_materialization_state;

use super::{
    AppliedMaterializationCoordinate, AppliedRepositoryMaterialization,
    CompiledGenerationAdmissionError, MATERIALIZATION_CLOSURE_SCHEMA,
};

#[derive(Serialize)]
struct MaterializationClosurePreimage<'a> {
    schema: &'static str,
    repositories: Vec<MaterializationClosureMember<'a>>,
}

#[derive(Serialize)]
struct MaterializationClosureMember<'a> {
    repository_id: &'a LogicalRepositoryIdV1,
    receipt_digest: &'a pale_core::records::ReceiptDigest,
    state_digest: &'a pale_core::records::MaterializationStateDigest,
    lineage_digest: &'a pale_core::records::MaterializationLineageDigest,
}

pub(super) fn materialization_closure(
    materializations: &[AppliedRepositoryMaterialization],
) -> Result<MaterializationClosureDigest, CompiledGenerationAdmissionError> {
    let mut repositories = materializations
        .iter()
        .map(|materialization| MaterializationClosureMember {
            repository_id: &materialization.receipt.record().repository_id,
            receipt_digest: &materialization.receipt.record().receipt_digest,
            state_digest: &materialization.state.state_digest,
            lineage_digest: &materialization.lineage.lineage_digest,
        })
        .collect::<Vec<_>>();
    repositories.sort_by(|left, right| left.repository_id.cmp(right.repository_id));
    MaterializationClosureDigest::hash_canonical(&MaterializationClosurePreimage {
        schema: MATERIALIZATION_CLOSURE_SCHEMA,
        repositories,
    })
    .map_err(
        |source| CompiledGenerationAdmissionError::Canonicalization {
            coordinate: AppliedMaterializationCoordinate::MaterializationClosure,
            source: Box::new(source),
        },
    )
}

pub(super) fn validate_repository_coverage(
    compilation: &Compilation,
    materializations: &[AppliedRepositoryMaterialization],
) -> Result<(), CompiledGenerationAdmissionError> {
    if compilation.repositories().len() != materializations.len() {
        return Err(CompiledGenerationAdmissionError::RepositoryCoverage {
            compiled: compilation.repositories().len(),
            applied: materializations.len(),
        });
    }
    for (position, (repository, materialization)) in compilation
        .repositories()
        .iter()
        .zip(materializations)
        .enumerate()
    {
        let observed = &materialization.receipt.record().repository_id;
        if repository.id() != observed {
            return Err(CompiledGenerationAdmissionError::RepositoryOrder {
                position,
                compiled: repository.id().clone(),
                applied: observed.clone(),
            });
        }
    }
    Ok(())
}

/// Admit the record-local owner join before compilation-specific comparison.
pub(super) fn validate_owner_materialization_records(
    materialization: &AppliedRepositoryMaterialization,
) -> Result<(), CompiledGenerationAdmissionError> {
    let receipt = materialization.receipt.record();
    let repository_id = &receipt.repository_id;
    let (post_state_digest, post_lineage_digest) = match &receipt.transition {
        MaterializationReceiptTransitionV1::Initialized {
            post_state_digest,
            post_lineage_digest,
        }
        | MaterializationReceiptTransitionV1::Advanced {
            post_state_digest,
            post_lineage_digest,
            ..
        } => (post_state_digest, post_lineage_digest),
        MaterializationReceiptTransitionV1::Removed { .. }
        | MaterializationReceiptTransitionV1::NoOwnedState {} => {
            return Err(CompiledGenerationAdmissionError::RemovalReceipt {
                repository_id: repository_id.clone(),
            });
        }
    };

    pale_contracts::validate_record(&materialization.state, "materialization-state.v1").map_err(
        |source| CompiledGenerationAdmissionError::StateSchema {
            repository_id: repository_id.clone(),
            source: Box::new(source),
        },
    )?;
    if materialization.state.recompute_digest().map_err(|source| {
        CompiledGenerationAdmissionError::Canonicalization {
            coordinate: AppliedMaterializationCoordinate::StateIdentity,
            source: Box::new(source),
        }
    })? != materialization.state.state_digest
    {
        return mismatch(
            repository_id,
            AppliedMaterializationCoordinate::StateIdentity,
        );
    }
    if post_state_digest != &materialization.state.state_digest {
        return mismatch(
            repository_id,
            AppliedMaterializationCoordinate::ReceiptTransition,
        );
    }

    pale_contracts::validate_record(&materialization.lineage, "materialization-lineage.v1")
        .map_err(|source| CompiledGenerationAdmissionError::LineageSchema {
            repository_id: repository_id.clone(),
            source: Box::new(source),
        })?;
    if materialization.lineage.schema.as_str() != MATERIALIZATION_LINEAGE_SCHEMA
        || materialization.lineage.schema_version != MATERIALIZATION_LINEAGE_SCHEMA_VERSION
        || materialization
            .lineage
            .recompute_digest()
            .map_err(
                |source| CompiledGenerationAdmissionError::Canonicalization {
                    coordinate: AppliedMaterializationCoordinate::LineageIdentity,
                    source: Box::new(source),
                },
            )?
            != materialization.lineage.lineage_digest
    {
        return mismatch(
            repository_id,
            AppliedMaterializationCoordinate::LineageIdentity,
        );
    }
    if post_lineage_digest != &materialization.lineage.lineage_digest
        || materialization.lineage.state_digest != materialization.state.state_digest
        || materialization.lineage.admitted_plan_digest != receipt.plan_digest
    {
        return mismatch(
            repository_id,
            AppliedMaterializationCoordinate::LineageReceiptBinding,
        );
    }
    if &materialization.state.repository_id != repository_id
        || &materialization.lineage.repository_id != repository_id
        || materialization.state.schema_manifest_sha256 != receipt.schema_manifest_sha256
        || materialization.state.schema_set_sha256 != receipt.schema_set_sha256
        || materialization.state.one_shot_compilation_digest != receipt.one_shot_compilation_digest
        || materialization.state.profile_digest != receipt.profile_digest
        || materialization.state.instance_digest != receipt.instance_digest
        || materialization.state.manifest_digest != receipt.manifest_digest
    {
        return mismatch(
            repository_id,
            AppliedMaterializationCoordinate::StateCompilation,
        );
    }
    if materialization.lineage.schema_manifest_sha256 != receipt.schema_manifest_sha256
        || materialization.lineage.schema_set_sha256 != receipt.schema_set_sha256
        || materialization.lineage.one_shot_compilation_digest
            != receipt.one_shot_compilation_digest
        || materialization.lineage.profile_digest != receipt.profile_digest
        || materialization.lineage.instance_digest != receipt.instance_digest
        || materialization.lineage.manifest_digest != receipt.manifest_digest
    {
        return mismatch(
            repository_id,
            AppliedMaterializationCoordinate::LineageCompilation,
        );
    }
    if materialization.source_frontier.repository_id != *repository_id {
        return mismatch(
            repository_id,
            AppliedMaterializationCoordinate::SourceFrontier,
        );
    }
    Ok(())
}

/// Compare an owner-admitted materialization with one compiled repository.
///
/// [`AppliedRepositoryMaterialization::admit`](super::AppliedRepositoryMaterialization::admit)
/// already establishes the record-local receipt/state/lineage join. This phase
/// deliberately checks only compilation-relative coordinates and the exact
/// desired surface, so the same immutable records are not schema-validated and
/// rehashed a second time during generation admission.
pub(super) fn validate_applied_materialization(
    compilation: &Compilation,
    repository: &CompiledRepo,
    materialization: &AppliedRepositoryMaterialization,
) -> Result<(), CompiledGenerationAdmissionError> {
    let repository_id = repository.id();
    let receipt = materialization.receipt.record();
    let schema_registry = compilation.schema_registry_identity();
    if receipt.schema_manifest_sha256 != schema_registry.schema_manifest_sha256
        || receipt.schema_set_sha256 != schema_registry.schema_set_sha256
        || receipt.one_shot_compilation_digest != *compilation.one_shot_compilation_digest()
        || receipt.profile_digest != compilation.profile().profile_digest
        || receipt.instance_digest != compilation.instance().instance_digest
        || receipt.manifest_digest != repository.manifest().manifest_digest
    {
        return mismatch(
            repository_id,
            AppliedMaterializationCoordinate::ReceiptCompilation,
        );
    }
    if receipt.package_closure != compilation.profile().package_closure
        || receipt.normative_pack_closure != compilation.profile().normative_pack_closure
        || receipt.package_semantic_ir_digest != *compilation.package_semantic_ir().digest()
    {
        return mismatch(
            repository_id,
            AppliedMaterializationCoordinate::ReceiptSemantics,
        );
    }

    let expected_state = desired_materialization_state(compilation, repository_id)?;
    if materialization.state != expected_state {
        return mismatch(
            repository_id,
            AppliedMaterializationCoordinate::StateSurfaceClosure,
        );
    }

    Ok(())
}

fn mismatch<T>(
    repository_id: &LogicalRepositoryIdV1,
    coordinate: AppliedMaterializationCoordinate,
) -> Result<T, CompiledGenerationAdmissionError> {
    Err(CompiledGenerationAdmissionError::CoordinateMismatch {
        repository_id: repository_id.clone(),
        coordinate,
    })
}
