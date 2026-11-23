//! Admission-safe binding of machine-local effects to a repository-surface plan.

use std::path::Path;

use pale_core::records::{
    AgentSurfacePlanV1, MachineLocalEffectKind, MachineLocalEffectV1,
    MaterializationPlanPreimageV1, MaterializationPlanV1, AGENT_SURFACE_PLAN_V1_SCHEMA,
    MATERIALIZATION_PLAN_V1_SCHEMA_VERSION,
};

use crate::plan::{validate, PlanError};

/// Complete materialization plan with its canonical bytes computed once.
///
/// The record remains ordinary deterministic planning output. Canonical bytes
/// are retained for saved-plan export and cross-surface parity; neither form is
/// admission or effect authority.
pub struct BoundMaterializationPlan {
    record: MaterializationPlanV1,
    canonical_bytes: Vec<u8>,
}

impl BoundMaterializationPlan {
    /// Borrow the complete typed materialization plan.
    #[must_use]
    pub const fn record(&self) -> &MaterializationPlanV1 {
        &self.record
    }

    /// Borrow exact canonical JCS bytes of [`Self::record`].
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    /// Consume the wrapper and retain only its typed record.
    #[must_use]
    pub fn into_record(self) -> MaterializationPlanV1 {
        self.record
    }
}

impl std::ops::Deref for BoundMaterializationPlan {
    type Target = MaterializationPlanV1;

    fn deref(&self) -> &Self::Target {
        self.record()
    }
}

/// Redact target and machine-local content while retaining stable counts.
impl std::fmt::Debug for BoundMaterializationPlan {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("BoundMaterializationPlan")
            .field("repository_id", &self.record.surface_plan.repository_id)
            .field(
                "plan_kind",
                &self.record.surface_plan.transition.plan_kind(),
            )
            .field(
                "operation_count",
                &self.record.surface_plan.operations.len(),
            )
            .field(
                "machine_local_effect_count",
                &self.record.machine_local_effects.len(),
            )
            .field("canonical_byte_len", &self.canonical_bytes.len())
            .finish()
    }
}

/// Bind a verified repository-surface plan to every machine-local effect required
/// by the same target transaction.
///
/// # Errors
///
/// Rejects a forged or unsupported nested surface plan, an unsafe or inconsistent
/// machine-local effect, duplicate effect identity, or schema/canonicalization drift.
pub fn bind_machine_local_effects(
    surface_plan: AgentSurfacePlanV1,
    mut machine_local_effects: Vec<MachineLocalEffectV1>,
) -> Result<BoundMaterializationPlan, PlanError> {
    verify_surface_plan(&surface_plan)?;
    validate_and_sort_machine_local_effects(&mut machine_local_effects)?;
    let schema_identity = pale_contracts::record_schema_identity("materialization-plan.v1")?;
    let plan = MaterializationPlanV1::seal(MaterializationPlanPreimageV1 {
        schema: schema_identity.schema,
        schema_sha256: schema_identity.schema_sha256,
        schema_version: MATERIALIZATION_PLAN_V1_SCHEMA_VERSION,
        schema_manifest_sha256: surface_plan.schema_manifest_sha256.clone(),
        schema_set_sha256: surface_plan.schema_set_sha256.clone(),
        one_shot_compilation_digest: surface_plan.one_shot_compilation_digest.clone(),
        surface_plan,
        machine_local_effects,
    })?;
    validate(&plan, "materialization-plan.v1")?;
    let canonical_bytes = pale_core::canon::to_canonical_string(&plan)?.into_bytes();
    Ok(BoundMaterializationPlan {
        record: plan,
        canonical_bytes,
    })
}

/// Validate and canonically order one complete machine-local effect set.
pub(super) fn validate_and_sort_machine_local_effects(
    machine_local_effects: &mut [MachineLocalEffectV1],
) -> Result<(), PlanError> {
    machine_local_effects
        .sort_by(|left, right| machine_effect_key(left).cmp(&machine_effect_key(right)));
    let mut prior_key = None;
    for effect in machine_local_effects.iter() {
        validate_machine_local_effect(effect)?;
        let key = machine_effect_key(effect);
        if prior_key.as_ref() == Some(&key) {
            return Err(PlanError::InvalidSuppliedPlan(format!(
                "duplicate machine-local effect `{}` at `{}`",
                effect.kind, effect.rel_path
            )));
        }
        prior_key = Some(key);
    }
    Ok(())
}

/// Re-admit the nested portable plan before binding machine-local effects to it.
fn verify_surface_plan(plan: &AgentSurfacePlanV1) -> Result<(), PlanError> {
    let carried_identity = pale_core::records::RecordSchemaIdentity {
        schema: plan.schema.clone(),
        schema_sha256: plan.schema_sha256.clone(),
    };
    if plan.schema.as_str() != AGENT_SURFACE_PLAN_V1_SCHEMA
        || pale_contracts::admit_record_schema_identity("agent-surface-plan.v1", &carried_identity)
            .is_err()
        || plan.schema_manifest_sha256
            != pale_contracts::schema_registry_identity()?.schema_manifest_sha256
        || plan.schema_set_sha256 != pale_contracts::schema_registry_identity()?.schema_set_sha256
    {
        return Err(PlanError::InvalidSuppliedPlan(
            "nested surface plan has an unsupported schema identity".into(),
        ));
    }
    if plan.recompute_digest()? != plan.plan_digest {
        return Err(PlanError::InvalidSuppliedPlan(
            "nested surface plan has a forged self-digest".into(),
        ));
    }
    validate(plan, "agent-surface-plan.v1")
}

/// Return the canonical ordering and uniqueness key for a machine-local effect.
fn machine_effect_key(effect: &MachineLocalEffectV1) -> (MachineLocalEffectKind, &str, &str) {
    (
        effect.kind,
        effect.root.canonical_path.as_str(),
        effect.rel_path.as_str(),
    )
}

/// Validate one effect's absolute root, confined target, and kind-specific path.
fn validate_machine_local_effect(effect: &MachineLocalEffectV1) -> Result<(), PlanError> {
    if !Path::new(&effect.root.canonical_path).is_absolute() {
        return Err(PlanError::InvalidSuppliedPlan(
            "machine-local root path must be absolute".into(),
        ));
    }
    if pale_core::paths::ConfinedRelativePath::parse(&effect.rel_path).is_none() {
        return Err(PlanError::InvalidSuppliedPlan(format!(
            "machine-local effect path is not confined: {}",
            effect.rel_path
        )));
    }
    if effect.kind == MachineLocalEffectKind::GitExcludeProtection
        && effect.rel_path != "info/exclude"
    {
        return Err(PlanError::InvalidSuppliedPlan(
            "Git exclude protection must target `info/exclude` beneath the common directory".into(),
        ));
    }
    validate_parent_directories(effect)
}

/// Require retained parent-directory evidence to be the exact target prefix chain.
fn validate_parent_directories(effect: &MachineLocalEffectV1) -> Result<(), PlanError> {
    let target =
        pale_core::paths::ConfinedRelativePath::parse(&effect.rel_path).ok_or_else(|| {
            PlanError::InvalidSuppliedPlan("machine-local effect path is not confined".into())
        })?;
    let components = target.components().collect::<Vec<_>>();
    let mut expected = Vec::new();
    let mut prefix = String::new();
    for component in components.iter().take(components.len().saturating_sub(1)) {
        if !prefix.is_empty() {
            prefix.push('/');
        }
        prefix.push_str(component);
        expected.push(prefix.clone());
    }
    let actual = effect
        .retained_directories
        .iter()
        .map(|directory| directory.rel_path.as_str())
        .chain(
            effect
                .transition
                .create_directories()
                .iter()
                .map(String::as_str),
        );
    if !actual.eq(expected.iter().map(String::as_str)) {
        return Err(PlanError::InvalidSuppliedPlan(
            "machine-local retained and created directories must exactly partition the target ancestors in shallowest-first order"
                .into(),
        ));
    }
    Ok(())
}
