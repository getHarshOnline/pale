//! Type-gated materialization-plan admission.
//!
//! [`admit_plan`] checks a saved plan against a caller-supplied rebuilt plan and
//! returns the only plan type accepted by filesystem mutation APIs.
//! [`AdmittedPlan`] is neither publicly constructible nor deserializable. This
//! module performs no filesystem I/O: the caller is responsible for rebuilding
//! from the current target, and the effect adapter reobserves before mutation.

use crate::canon;
use crate::records::{
    AgentSurfacePlanV1, MaterializationPlanV1, PlanDigest, AGENT_SURFACE_PLAN_V1_SCHEMA,
    MATERIALIZATION_PLAN_V1_SCHEMA, MATERIALIZATION_PLAN_V1_SCHEMA_VERSION,
};

/// Closed operation class shared by planning and mutation admission.
pub use crate::records::PlanKind;

/// Errors from plan admission (verification).
#[derive(Debug, thiserror::Error)]
pub enum AdmissionError {
    /// Canonical serialization failed while recomputing identity.
    #[error("canonicalization: {0}")]
    Canon(#[from] canon::CanonError),
    /// Saved content no longer equals the caller-supplied rebuilt plan.
    #[error("stale plan: the saved plan does not equal the caller-supplied rebuilt plan")]
    Stale,
    /// The claimed digest does not identify the saved plan's own content.
    #[error("forged plan: the saved plan's own digest does not match its content")]
    Forged,
    /// The caller attempted to admit a plan for a different operation class.
    #[error("plan kind mismatch: saved `{saved}`, expected `{expected}")]
    KindMismatch {
        /// Operation class encoded by the saved plan.
        saved: PlanKind,
        /// Operation class requested by the mutation entry point.
        expected: PlanKind,
    },
    /// A persisted plan uses another wire identity or major.
    #[error("unsupported plan schema identity or version")]
    UnsupportedSchema,
    /// The nested repository-surface plan's self-digest does not match its body.
    #[error("forged surface plan: its digest does not match its canonical content")]
    ForgedSurfacePlan,
}

/// A verified plan admitted for one operation class.
///
/// The payload is private and this type has no public constructor or
/// [`serde::Deserialize`] implementation. This value is immutable verification
/// evidence, not a linear one-use execution authorization: effect adapters must
/// still revalidate their observed target state, and execution/settlement policy
/// must independently prevent an unintended repeated effect.
#[must_use = "an admitted plan must be presented to the matching mutation boundary"]
pub struct AdmittedPlan {
    plan: MaterializationPlanV1,
    digest: PlanDigest,
}

impl AdmittedPlan {
    /// The verified surface plan carried by this reusable admission evidence.
    pub fn plan(&self) -> &AgentSurfacePlanV1 {
        &self.plan.surface_plan
    }
    /// The complete verified v1 plan carrying planned surface and machine-local effects.
    pub fn record(&self) -> &MaterializationPlanV1 {
        &self.plan
    }
    /// Digest of the complete verified v1 plan.
    pub fn digest(&self) -> &PlanDigest {
        &self.digest
    }
    /// The operation class this plan was admitted for.
    pub fn kind(&self) -> PlanKind {
        self.plan.surface_plan.transition.plan_kind()
    }
}

/// Redact the verified plan payload while retaining its operation class.
impl std::fmt::Debug for AdmittedPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdmittedPlan")
            .field("kind", &self.plan.surface_plan.transition.plan_kind())
            .field("plan", &"<redacted>")
            .finish_non_exhaustive()
    }
}

/// Admit a saved plan only when its class and self-digest are valid and its
/// canonical content equals a caller-supplied rebuilt plan.
///
/// The returned [`AdmittedPlan`] carries the recomputed plan, not the
/// caller-supplied saved allocation. This function performs no filesystem I/O;
/// callers must derive `rebuilt` from current observations. The result proves
/// only the checked identities, relationships, class, and equality. Effect
/// adapters still own target revalidation and repeated-effect prevention.
///
/// # Errors
///
/// Returns [`AdmissionError::UnsupportedSchema`] for another wire contract;
/// [`AdmissionError::ForgedSurfacePlan`] for a bad nested digest;
/// [`AdmissionError::Forged`] for a bad envelope digest;
/// [`AdmissionError::KindMismatch`] for another operation class;
/// [`AdmissionError::Stale`] when saved and rebuilt content differ; or
/// [`AdmissionError::Canon`] when a digest preimage cannot be represented.
pub fn admit_plan(
    saved: &MaterializationPlanV1,
    rebuilt: &MaterializationPlanV1,
    expected_kind: PlanKind,
) -> Result<AdmittedPlan, AdmissionError> {
    verify_envelope(saved)?;
    verify_envelope(rebuilt)?;
    let saved_kind = saved.surface_plan.transition.plan_kind();
    if saved_kind != expected_kind {
        return Err(AdmissionError::KindMismatch {
            saved: saved_kind,
            expected: expected_kind,
        });
    }
    let saved_recomputed = saved.recompute_digest()?;
    if saved_recomputed != saved.plan_digest {
        return Err(AdmissionError::Forged);
    }
    let rebuilt_recomputed = rebuilt.recompute_digest()?;
    if rebuilt_recomputed != rebuilt.plan_digest {
        return Err(AdmissionError::Forged);
    }
    if saved_recomputed != rebuilt_recomputed || saved.to_preimage() != rebuilt.to_preimage() {
        return Err(AdmissionError::Stale);
    }
    Ok(AdmittedPlan {
        plan: rebuilt.clone(),
        digest: rebuilt_recomputed,
    })
}

fn verify_envelope(plan: &MaterializationPlanV1) -> Result<(), AdmissionError> {
    if plan.schema.as_str() != MATERIALIZATION_PLAN_V1_SCHEMA
        || plan.schema_version != MATERIALIZATION_PLAN_V1_SCHEMA_VERSION
        || plan.surface_plan.schema.as_str() != AGENT_SURFACE_PLAN_V1_SCHEMA
        || plan.schema_manifest_sha256 != plan.surface_plan.schema_manifest_sha256
        || plan.schema_set_sha256 != plan.surface_plan.schema_set_sha256
        || plan.one_shot_compilation_digest != plan.surface_plan.one_shot_compilation_digest
    {
        return Err(AdmissionError::UnsupportedSchema);
    }
    if plan.surface_plan.recompute_digest()? != plan.surface_plan.plan_digest {
        return Err(AdmissionError::ForgedSurfacePlan);
    }
    Ok(())
}
