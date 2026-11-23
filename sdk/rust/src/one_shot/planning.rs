//! Effect-free planning over one opaque admitted one-shot compilation.

use std::error::Error;

use pale_application::plan::{
    BoundMaterializationPlan, MaterializationIntent, MaterializationPayloads, PlanError, PlanKind,
    RepositoryMaterializationInput, SavedPlanAdmissionError,
};
use pale_core::records::{
    AgentSurfacePlanV1, ArtifactDigest, BaselineEntry, BaselineStateV1, DiagnosticDispositionV1,
    DiagnosticEffectCertaintyV1, DiagnosticRetryV1, DiagnosticStageV1, LogicalRepositoryIdV1,
    MachineLocalEffectV1, MaterializationContractIdentity, MaterializationStateV1, SurfaceAction,
    UnixFileMode,
};
#[cfg(feature = "native-local")]
use pale_ports::{AdmittedMaterializationEffect, MaterializationPayload};

use super::OneShotCompilation;
use crate::error::{ApiError, FailureSpecification};
use crate::PaleApi;

/// Maximum prior-state bytes admitted by the pure planning boundary.
const MAX_PRIOR_STATE_BYTES: usize = 16 * 1024 * 1024;
/// Maximum claimed target observations accepted before plan construction.
const MAX_BASELINE_ENTRIES: usize = 8_192;
/// Maximum canonical bytes admitted for one machine-local effect observation.
const MAX_MACHINE_LOCAL_EFFECT_BYTES: usize = 4 * 1024 * 1024;

/// Canonical, bounded machine-local observation supplied for plan binding.
///
/// This API-owned value prevents host leaves from depending on the ports crate
/// or reconstructing [`MachineLocalEffectV1`] field by field. Construction is
/// inert; [`PaleApi::bind_one_shot_machine_effects`] privately admits the
/// bytes and delegates effect-set semantics to the application planning owner.
pub struct MachineLocalEffectCandidate {
    canonical_bytes: Vec<u8>,
}

impl std::fmt::Debug for MachineLocalEffectCandidate {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MachineLocalEffectCandidate")
            .field("canonical_byte_len", &self.canonical_bytes.len())
            .finish()
    }
}

impl MachineLocalEffectCandidate {
    /// Capture one claimed canonical machine-local effect record.
    ///
    /// Parsing and semantic admission are deliberately deferred to
    /// [`PaleApi::bind_one_shot_machine_effects`] so every rejection crosses
    /// the structured diagnostic boundary.
    #[must_use]
    pub fn new(canonical_bytes: Vec<u8>) -> Self {
        Self { canonical_bytes }
    }

    fn into_record(self) -> Result<MachineLocalEffectV1, PlanningInputError> {
        if self.canonical_bytes.len() > MAX_MACHINE_LOCAL_EFFECT_BYTES {
            return Err(PlanningInputError::MachineEffectTooLarge);
        }
        let value = pale_core::canon::parse_strict(&self.canonical_bytes)
            .map_err(PlanningInputError::MachineEffectSyntax)?;
        let canonical = pale_core::canon::to_canonical_string(&value)
            .map_err(PlanningInputError::MachineEffectSyntax)?;
        if canonical.as_bytes() != self.canonical_bytes {
            return Err(PlanningInputError::MachineEffectNotCanonical);
        }
        serde_json::from_value(value).map_err(PlanningInputError::MachineEffectDecode)
    }
}

/// Closed effect-free repository-plan operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OneShotPlanKind {
    /// Reconcile current desired surfaces with prior ownership when present.
    Apply,
    /// Explicitly adopt matching pre-existing generated targets on first ownership.
    Adopt,
    /// Remove only surfaces proven by prior ownership state.
    Remove,
}

/// Claimed pre-mutation observation for one repository-relative target.
///
/// The API does not perform the observation and does not upgrade this value to
/// native evidence. A later effect adapter must reobserve the target before any
/// mutation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OneShotBaselineObservation {
    /// No regular file was observed.
    Absent,
    /// A regular file was observed with these exact claimed properties.
    Present {
        /// SHA-256 of the complete observed file bytes.
        sha256: String,
        /// Claimed Unix permission bits, limited to `0o7777`.
        unix_mode: u32,
    },
}

/// One owned claimed baseline observation bound to a confined logical target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OneShotBaselineEntry {
    relative_path: String,
    observation: OneShotBaselineObservation,
}

impl OneShotBaselineEntry {
    /// Construct one claimed baseline entry.
    ///
    /// Path, digest, mode, uniqueness, and exact target-set admission are
    /// deferred to [`PaleApi::plan_one_shot`] so all failures use the same
    /// structured diagnostic boundary.
    #[must_use]
    pub fn new(relative_path: String, observation: OneShotBaselineObservation) -> Self {
        Self {
            relative_path,
            observation,
        }
    }
}

/// Prior Pale ownership state supplied to pure planning.
///
/// `Owned` bytes must be the exact canonical `materialization-state.v1` record,
/// including a correct self-digest. The state is a claim used to construct a
/// candidate plan; this boundary neither reads durable lineage nor admits the
/// candidate for mutation.
pub enum OneShotPriorMaterialization {
    /// No prior Pale ownership record exists.
    Unowned,
    /// Exact canonical bytes of the claimed prior ownership record.
    Owned {
        /// Complete canonical JCS encoding of `materialization-state.v1`.
        canonical_state: Vec<u8>,
    },
}

impl std::fmt::Debug for OneShotPriorMaterialization {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unowned => formatter.write_str("OneShotPriorMaterialization::Unowned"),
            Self::Owned { canonical_state } => formatter
                .debug_struct("OneShotPriorMaterialization::Owned")
                .field("canonical_state_bytes", &canonical_state.len())
                .finish(),
        }
    }
}

/// One confined logical target that must be observed before pure planning.
///
/// The value is relative to the selected logical repository. It is neither a
/// host path nor proof that a file exists. Mapping a repository to a host root,
/// observing this target, and revalidating it immediately before mutation all
/// remain external responsibilities.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(transparent)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
pub struct OneShotObservationTarget {
    relative_path: String,
}

impl OneShotObservationTarget {
    /// Return the confined repository-relative target label.
    #[must_use]
    pub fn relative_path(&self) -> &str {
        &self.relative_path
    }
}

/// Complete ordered observation-target result for one planning intent.
///
/// The typed entries support Rust-side host observation. [`Self::canonical_bytes`]
/// is the single binding projection used by CLI-derived transports and language
/// leaves; it proves no observation and carries no host-path capability.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "wire-schema", schemars(deny_unknown_fields))]
pub struct OneShotObservationTargets {
    #[serde(rename = "targets")]
    entries: Vec<OneShotObservationTarget>,
    #[serde(skip)]
    #[cfg_attr(feature = "wire-schema", schemars(skip))]
    canonical_bytes: Vec<u8>,
}

impl OneShotObservationTargets {
    fn new(entries: Vec<OneShotObservationTarget>) -> Self {
        let mut targets = Self {
            entries,
            canonical_bytes: Vec::new(),
        };
        targets.canonical_bytes = pale_core::canon::to_canonical_string(&targets)
            .expect("API-owned observation targets are canonical JSON")
            .into_bytes();
        targets
    }

    /// Borrow the canonically ordered confined target labels.
    #[must_use]
    pub fn entries(&self) -> &[OneShotObservationTarget] {
        &self.entries
    }

    /// Borrow the RFC 8785 binding projection of [`Self::entries`].
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }
}

impl std::ops::Deref for OneShotObservationTargets {
    type Target = [OneShotObservationTarget];

    fn deref(&self) -> &Self::Target {
        self.entries()
    }
}

impl IntoIterator for OneShotObservationTargets {
    type Item = OneShotObservationTarget;
    type IntoIter = std::vec::IntoIter<OneShotObservationTarget>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}

/// Owned input for one effect-free repository plan candidate.
pub struct OneShotPlanningRequest {
    repository_id: String,
    kind: OneShotPlanKind,
    prior: OneShotPriorMaterialization,
    baseline: Vec<OneShotBaselineEntry>,
}

impl std::fmt::Debug for OneShotPlanningRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OneShotPlanningRequest")
            .field("repository_id_byte_len", &self.repository_id.len())
            .field("kind", &self.kind)
            .field("prior", &self.prior)
            .field("baseline_count", &self.baseline.len())
            .finish()
    }
}

impl OneShotPlanningRequest {
    /// Assemble a pure planning request from caller-captured claims.
    #[must_use]
    pub fn new(
        repository_id: String,
        kind: OneShotPlanKind,
        prior: OneShotPriorMaterialization,
        baseline: Vec<OneShotBaselineEntry>,
    ) -> Self {
        Self {
            repository_id,
            kind,
            prior,
            baseline,
        }
    }
}

/// Canonical non-admitted `agent-surface-plan.v1` candidate and flat counts.
///
/// The bytes are the existing application-owned record, not a façade-specific wire
/// format. This value proves deterministic plan construction only. It contains
/// no machine-local ignore effects, saved-plan admission, transaction authority,
/// native observation, activation, or effect evidence.
pub struct OneShotPlanCandidate {
    kind: OneShotPlanKind,
    repository_id: String,
    plan_digest: String,
    one_shot_compilation_digest: String,
    baseline_count: u32,
    operation_count: u32,
    create_count: u32,
    adopt_count: u32,
    update_count: u32,
    unchanged_count: u32,
    delete_count: u32,
    splice_count: u32,
    deferred_effect_count: u32,
    canonical_bytes: Vec<u8>,
    local_only_targets: Vec<String>,
    surface_plan: AgentSurfacePlanV1,
    payloads: MaterializationPayloads,
    contract_identity: MaterializationContractIdentity,
}

#[cfg(feature = "wire-schema")]
impl schemars::JsonSchema for OneShotPlanCandidate {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        <AgentSurfacePlanV1 as schemars::JsonSchema>::schema_name()
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        <AgentSurfacePlanV1 as schemars::JsonSchema>::schema_id()
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        <AgentSurfacePlanV1 as schemars::JsonSchema>::json_schema(generator)
    }
}

impl std::fmt::Debug for OneShotPlanCandidate {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OneShotPlanCandidate")
            .field("kind", &self.kind)
            .field("baseline_count", &self.baseline_count)
            .field("operation_count", &self.operation_count)
            .field("deferred_effect_count", &self.deferred_effect_count)
            .field("canonical_byte_len", &self.canonical_bytes.len())
            .finish_non_exhaustive()
    }
}

impl OneShotPlanCandidate {
    /// Requested planning operation.
    #[must_use]
    pub const fn kind(&self) -> OneShotPlanKind {
        self.kind
    }

    /// Logical repository selected from the one-shot compilation.
    #[must_use]
    pub fn repository_id(&self) -> &str {
        &self.repository_id
    }

    /// Canonical identity of the complete candidate body.
    #[must_use]
    pub fn plan_digest(&self) -> &str {
        &self.plan_digest
    }

    /// One-shot output identity that the candidate plans from.
    ///
    /// This is not a resident generation coordinate.
    #[must_use]
    pub fn one_shot_compilation_digest(&self) -> &str {
        &self.one_shot_compilation_digest
    }

    /// Number of caller-supplied target observations retained in the candidate.
    #[must_use]
    pub const fn baseline_count(&self) -> u32 {
        self.baseline_count
    }

    /// Number of planned repository-surface transitions.
    #[must_use]
    pub const fn operation_count(&self) -> u32 {
        self.operation_count
    }

    /// Number of create transitions.
    #[must_use]
    pub const fn create_count(&self) -> u32 {
        self.create_count
    }

    /// Number of explicit adoption transitions.
    #[must_use]
    pub const fn adopt_count(&self) -> u32 {
        self.adopt_count
    }

    /// Number of update transitions.
    #[must_use]
    pub const fn update_count(&self) -> u32 {
        self.update_count
    }

    /// Number of unchanged transitions.
    #[must_use]
    pub const fn unchanged_count(&self) -> u32 {
        self.unchanged_count
    }

    /// Number of whole-file deletion transitions.
    #[must_use]
    pub const fn delete_count(&self) -> u32 {
        self.delete_count
    }

    /// Number of managed-block splice transitions.
    #[must_use]
    pub const fn splice_count(&self) -> u32 {
        self.splice_count
    }

    /// Number of declared host effects intentionally excluded from the plan transaction.
    #[must_use]
    pub const fn deferred_effect_count(&self) -> u32 {
        self.deferred_effect_count
    }

    /// Exact canonical JCS bytes of the application-owned plan candidate.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    /// Confined targets whose generated form is deliberately machine-local.
    ///
    /// The host uses this exact application-owned set to compose local ignore
    /// observations. The labels are repository-relative and grant no path or
    /// mutation authority.
    #[must_use]
    pub fn local_only_targets(&self) -> &[String] {
        &self.local_only_targets
    }

    fn project(
        kind: OneShotPlanKind,
        plan: pale_application::plan::RepositoryMaterializationPlan,
        contract_identity: MaterializationContractIdentity,
    ) -> Result<Self, PlanProjectionError> {
        let (surface_plan, payloads) = plan.into_parts();
        let counts = ActionCounts::from_plan(&surface_plan)?;
        let canonical_bytes = pale_core::canon::to_canonical_string(&surface_plan)
            .map_err(PlanProjectionError::Canonicalization)?
            .into_bytes();
        let local_only_targets =
            pale_application::plan::local_only_materialization_targets(&surface_plan.operations);
        Ok(Self {
            kind,
            repository_id: surface_plan.repository_id.to_string(),
            plan_digest: surface_plan.plan_digest.as_str().to_string(),
            one_shot_compilation_digest: surface_plan
                .one_shot_compilation_digest
                .as_str()
                .to_string(),
            baseline_count: count("plan target baseline", surface_plan.target_baseline.len())?,
            operation_count: count("plan operations", surface_plan.operations.len())?,
            create_count: counts.create,
            adopt_count: counts.adopt,
            update_count: counts.update,
            unchanged_count: counts.unchanged,
            delete_count: counts.delete,
            splice_count: counts.splice,
            deferred_effect_count: count(
                "plan deferred effects",
                surface_plan.deferred_effects.len(),
            )?,
            canonical_bytes,
            local_only_targets,
            surface_plan,
            payloads,
            contract_identity,
        })
    }
}

/// Complete deterministic plan after machine-local host observations are bound.
///
/// This is still a candidate: only [`PaleApi::admit_one_shot_saved_plan`]
/// compares caller-captured saved bytes and mints effect authority.
pub struct OneShotBoundPlan {
    kind: OneShotPlanKind,
    repository_id: String,
    plan: BoundMaterializationPlan,
    payloads: MaterializationPayloads,
    contract_identity: MaterializationContractIdentity,
}

impl std::fmt::Debug for OneShotBoundPlan {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OneShotBoundPlan")
            .field("kind", &self.kind)
            .field("repository_id", &self.repository_id)
            .field("canonical_byte_len", &self.plan.canonical_bytes().len())
            .finish_non_exhaustive()
    }
}

impl OneShotBoundPlan {
    /// Requested operation class.
    #[must_use]
    pub const fn kind(&self) -> OneShotPlanKind {
        self.kind
    }

    /// Selected logical repository identity.
    #[must_use]
    pub fn repository_id(&self) -> &str {
        &self.repository_id
    }

    /// Exact canonical complete materialization-plan bytes.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        self.plan.canonical_bytes()
    }
}

/// Saved-plan equality proof and, in native-local builds, its private adapter capability.
///
/// The public value exposes identity only. Native-local composition inside this
/// crate may retain and consume the private capability without leaves observing
/// plan records, rendered payloads, contract identities, or machine-local paths.
pub struct OneShotAdmittedPlan {
    kind: OneShotPlanKind,
    repository_id: String,
    plan_digest: String,
    #[cfg(feature = "native-local")]
    effect: AdmittedMaterializationEffect,
}

impl std::fmt::Debug for OneShotAdmittedPlan {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OneShotAdmittedPlan")
            .field("kind", &self.kind)
            .field("repository_id", &self.repository_id)
            .field("plan_digest", &self.plan_digest)
            .finish_non_exhaustive()
    }
}

impl OneShotAdmittedPlan {
    /// Admitted operation class.
    #[must_use]
    pub const fn kind(&self) -> OneShotPlanKind {
        self.kind
    }

    /// Selected logical repository identity.
    #[must_use]
    pub fn repository_id(&self) -> &str {
        &self.repository_id
    }

    /// Digest of the admitted complete materialization plan.
    #[must_use]
    pub fn plan_digest(&self) -> &str {
        &self.plan_digest
    }

    /// Borrow the path-redacting capability for crate-private native-local composition.
    #[must_use]
    #[cfg(feature = "native-local")]
    pub(crate) const fn effect_capability(&self) -> &AdmittedMaterializationEffect {
        &self.effect
    }
}

impl PaleApi {
    /// Derive the exact target set that must be observed for one planning intent.
    ///
    /// Apply and adoption require the union of prior-owned and currently
    /// desired filesystem targets. Removal requires prior-owned targets only.
    /// The application planning owner derives and validates that set from the
    /// same algebra used by [`Self::plan_one_shot`]; the façade does not
    /// reconstruct union or removal rules.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when the repository selector is malformed or absent,
    /// the prior-state claim is non-canonical or incoherent, or the admitted
    /// compilation contradicts a planning invariant. No target is observed by
    /// this call.
    pub fn one_shot_observation_targets(
        &self,
        compilation: &OneShotCompilation,
        repository_id: String,
        kind: OneShotPlanKind,
        prior: &OneShotPriorMaterialization,
    ) -> Result<OneShotObservationTargets, ApiError> {
        let repository_id = LogicalRepositoryIdV1::parse(repository_id)
            .map_err(PlanningInputError::RepositoryId)
            .map_err(|error| planning_input_failure(&self.diagnostic_identity, error))?;
        let prior = admit_prior_state(prior)
            .map_err(|error| planning_input_failure(&self.diagnostic_identity, error))?;
        let input = RepositoryMaterializationInput::for_repository(
            &compilation.compilation,
            &repository_id,
        )
        .map_err(|error| planning_engine_failure(&self.diagnostic_identity, error))?;
        let plan_kind = match kind {
            OneShotPlanKind::Apply | OneShotPlanKind::Adopt => PlanKind::Apply,
            OneShotPlanKind::Remove => PlanKind::Remove,
        };
        input
            .required_observation_targets(plan_kind, prior.as_ref())
            .map_err(|error| planning_engine_failure(&self.diagnostic_identity, error))
            .map(|targets| {
                OneShotObservationTargets::new(
                    targets
                        .into_iter()
                        .map(|relative_path| OneShotObservationTarget { relative_path })
                        .collect(),
                )
            })
    }

    /// Build one deterministic repository-surface plan without observing or mutating a host.
    ///
    /// The opaque [`OneShotCompilation`] supplies every compiler identity and
    /// desired surface. The request supplies only claimed prior ownership and
    /// baseline observations. Apply, adoption, and removal use the application
    /// planning SSOT; this façade does not duplicate transition semantics.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when claimed bytes are non-canonical or violate the
    /// materialization-state contract, a baseline is invalid or incomplete, the
    /// repository selector is malformed or absent, its sealed
    /// execution-workspace membership is read-only, or deterministic plan
    /// construction contradicts a Pale application invariant. A read-only membership
    /// returns a blocked diagnostic whose effect certainty is not applicable.
    /// No error can imply a native effect because none is attempted.
    pub fn plan_one_shot(
        &self,
        compilation: &OneShotCompilation,
        request: OneShotPlanningRequest,
    ) -> Result<OneShotPlanCandidate, ApiError> {
        let OneShotPlanningRequest {
            repository_id,
            kind,
            prior,
            baseline,
        } = request;
        let repository_id = LogicalRepositoryIdV1::parse(repository_id)
            .map_err(PlanningInputError::RepositoryId)
            .map_err(|error| planning_input_failure(&self.diagnostic_identity, error))?;
        let prior = admit_prior_state(&prior)
            .map_err(|error| planning_input_failure(&self.diagnostic_identity, error))?;
        let baseline = admit_baseline(baseline)
            .map_err(|error| planning_input_failure(&self.diagnostic_identity, error))?;
        let intent = match kind {
            OneShotPlanKind::Apply => MaterializationIntent::Apply,
            OneShotPlanKind::Adopt => MaterializationIntent::AdoptUnowned,
            OneShotPlanKind::Remove => MaterializationIntent::Remove,
        };
        let input = RepositoryMaterializationInput::for_repository(
            &compilation.compilation,
            &repository_id,
        )
        .map_err(|error| planning_engine_failure(&self.diagnostic_identity, error))?;
        let plan = input
            .build(intent, prior.as_ref(), baseline)
            .map_err(|error| planning_engine_failure(&self.diagnostic_identity, error))?;
        let contract_identity =
            pale_application::plan::materialization_contract_identity(&compilation.compilation)
                .map_err(PlanError::Schema)
                .map_err(|error| planning_engine_failure(&self.diagnostic_identity, error))?;
        OneShotPlanCandidate::project(kind, plan, contract_identity)
            .map_err(|error| planning_projection_failure(&self.diagnostic_identity, error))
    }

    /// Bind host-composed machine-local observations into one complete plan.
    ///
    /// Each candidate is opaque at this boundary: the API consumes its record
    /// once and delegates validation, canonical ordering, and complete-plan
    /// sealing to the application owner.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when a supplied observation is malformed,
    /// duplicated, or contradicts the selected effect kind. No saved-plan
    /// authority or native effect is created.
    pub fn bind_one_shot_machine_effects(
        &self,
        candidate: OneShotPlanCandidate,
        effects: Vec<MachineLocalEffectCandidate>,
    ) -> Result<OneShotBoundPlan, ApiError> {
        let OneShotPlanCandidate {
            kind,
            repository_id,
            surface_plan,
            payloads,
            contract_identity,
            ..
        } = candidate;
        let effects = effects
            .into_iter()
            .map(MachineLocalEffectCandidate::into_record)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| planning_input_failure(&self.diagnostic_identity, error))?;
        let plan = pale_application::plan::bind_machine_local_effects(surface_plan, effects)
            .map_err(|error| planning_engine_failure(&self.diagnostic_identity, error))?;
        Ok(OneShotBoundPlan {
            kind,
            repository_id,
            plan,
            payloads,
            contract_identity,
        })
    }

    /// Admit complete saved-plan bytes against one freshly rebuilt bound plan.
    ///
    /// Exact semantic equality, plan class, nested identities, and self-digests
    /// are checked once by the application gate. Success returns an opaque
    /// adapter capability; it performs no filesystem mutation.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when saved bytes are oversized, malformed, forged,
    /// stale, wrong-class, or otherwise unequal to the rebuilt plan.
    pub fn admit_one_shot_saved_plan(
        &self,
        rebuilt: OneShotBoundPlan,
        saved_bytes: &[u8],
    ) -> Result<OneShotAdmittedPlan, ApiError> {
        let OneShotBoundPlan {
            kind,
            repository_id,
            plan,
            payloads,
            contract_identity,
        } = rebuilt;
        let expected = match kind {
            OneShotPlanKind::Apply | OneShotPlanKind::Adopt => PlanKind::Apply,
            OneShotPlanKind::Remove => PlanKind::Remove,
        };
        let admitted = pale_application::plan::admit_saved_materialization_plan(
            saved_bytes,
            plan.record(),
            expected,
        )
        .map_err(|error| saved_plan_failure(&self.diagnostic_identity, error))?;
        let plan_digest = admitted.digest().as_str().to_owned();
        #[cfg(feature = "native-local")]
        let payloads = payloads
            .into_iter()
            .map(|(path, payload)| {
                (
                    path,
                    MaterializationPayload::from_shared(payload.into_shared()),
                )
            })
            .collect();
        #[cfg(feature = "native-local")]
        let effect = AdmittedMaterializationEffect::from_api_admission(
            admitted,
            payloads,
            contract_identity,
        );
        #[cfg(not(feature = "native-local"))]
        let _ = (admitted, payloads, contract_identity);
        Ok(OneShotAdmittedPlan {
            kind,
            repository_id,
            plan_digest,
            #[cfg(feature = "native-local")]
            effect,
        })
    }
}

fn admit_prior_state(
    prior: &OneShotPriorMaterialization,
) -> Result<Option<MaterializationStateV1>, PlanningInputError> {
    let OneShotPriorMaterialization::Owned { canonical_state } = prior else {
        return Ok(None);
    };
    if canonical_state.len() > MAX_PRIOR_STATE_BYTES {
        return Err(PlanningInputError::PriorStateTooLarge);
    }
    let value = pale_core::canon::parse_strict(canonical_state)
        .map_err(PlanningInputError::PriorStateSyntax)?;
    pale_contracts::validate_value(&value, "materialization-state.v1")
        .map_err(|source| PlanningInputError::PriorStateContract(Box::new(source)))?;
    let canonical = pale_core::canon::to_canonical_string(&value)
        .map_err(PlanningInputError::PriorStateSyntax)?;
    if canonical.as_bytes() != canonical_state {
        return Err(PlanningInputError::PriorStateNotCanonical);
    }
    let state: MaterializationStateV1 =
        serde_json::from_value(value).map_err(PlanningInputError::PriorStateDecode)?;
    let expected = state
        .recompute_digest()
        .map_err(PlanningInputError::PriorStateSyntax)?;
    if expected != state.state_digest {
        return Err(PlanningInputError::PriorStateDigestMismatch);
    }
    Ok(Some(state))
}

fn admit_baseline(
    baseline: Vec<OneShotBaselineEntry>,
) -> Result<Vec<BaselineEntry>, PlanningInputError> {
    if baseline.len() > MAX_BASELINE_ENTRIES {
        return Err(PlanningInputError::BaselineLimitExceeded);
    }
    baseline
        .into_iter()
        .map(|entry| {
            let observation = match entry.observation {
                OneShotBaselineObservation::Absent => BaselineStateV1::Absent {},
                OneShotBaselineObservation::Present { sha256, unix_mode } => {
                    BaselineStateV1::Present {
                        sha256: ArtifactDigest::parse(sha256)
                            .map_err(PlanningInputError::BaselineDigest)?,
                        mode: UnixFileMode::try_from(unix_mode)
                            .map_err(PlanningInputError::BaselineMode)?,
                    }
                }
            };
            Ok(BaselineEntry {
                rel_path: entry.relative_path,
                observation,
            })
        })
        .collect()
}

#[derive(Default)]
struct ActionCounts {
    create: u32,
    adopt: u32,
    update: u32,
    unchanged: u32,
    delete: u32,
    splice: u32,
}

impl ActionCounts {
    fn from_plan(plan: &AgentSurfacePlanV1) -> Result<Self, PlanProjectionError> {
        let mut counts = Self::default();
        for operation in &plan.operations {
            let count = match operation.transition.action() {
                SurfaceAction::Create => &mut counts.create,
                SurfaceAction::Adopt => &mut counts.adopt,
                SurfaceAction::Update => &mut counts.update,
                SurfaceAction::Unchanged => &mut counts.unchanged,
                SurfaceAction::Delete => &mut counts.delete,
                SurfaceAction::Splice => &mut counts.splice,
            };
            *count = count
                .checked_add(1)
                .ok_or(PlanProjectionError::CountOverflow {
                    coordinate: "plan action partition",
                })?;
        }
        Ok(counts)
    }
}

#[derive(Debug, thiserror::Error)]
enum PlanningInputError {
    #[error("repository selector is not a canonical logical repository identity")]
    RepositoryId(#[source] pale_core::records::InvalidWorkspaceLanguageId),
    #[error("prior materialization state exceeds the API byte bound")]
    PriorStateTooLarge,
    #[error("prior materialization state is not strict canonical JSON")]
    PriorStateSyntax(#[source] pale_core::canon::CanonError),
    #[error("prior materialization state violates its selected contract")]
    PriorStateContract(#[source] Box<pale_contracts::SchemaError>),
    #[error("prior materialization state is not in canonical byte form")]
    PriorStateNotCanonical,
    #[error("prior materialization state cannot decode into its typed record")]
    PriorStateDecode(#[source] serde_json::Error),
    #[error("prior materialization state has an incorrect self-digest")]
    PriorStateDigestMismatch,
    #[error("target baseline exceeds the API entry bound")]
    BaselineLimitExceeded,
    #[error("target baseline contains an invalid content digest")]
    BaselineDigest(#[source] pale_core::records::InvalidDigest),
    #[error("target baseline contains an invalid Unix file mode")]
    BaselineMode(#[source] pale_core::records::InvalidUnixFileMode),
    #[error("machine-local effect exceeds the API byte bound")]
    MachineEffectTooLarge,
    #[error("machine-local effect is not strict canonical JSON")]
    MachineEffectSyntax(#[source] pale_core::canon::CanonError),
    #[error("machine-local effect is not in canonical byte form")]
    MachineEffectNotCanonical,
    #[error("machine-local effect cannot decode into its typed record")]
    MachineEffectDecode(#[source] serde_json::Error),
}

#[derive(Debug, thiserror::Error)]
enum PlanProjectionError {
    #[error("{coordinate} count exceeds the portable API bound")]
    CountOverflow { coordinate: &'static str },
    #[error("the sealed plan cannot be encoded as canonical JSON")]
    Canonicalization(#[source] pale_core::canon::CanonError),
}

fn count(coordinate: &'static str, value: usize) -> Result<u32, PlanProjectionError> {
    u32::try_from(value).map_err(|_| PlanProjectionError::CountOverflow { coordinate })
}

fn planning_input_failure(
    identity: &pale_core::records::RecordSchemaIdentity,
    error: PlanningInputError,
) -> ApiError {
    planning_failure(
        identity,
        error,
        "one_shot_planning_input_rejected",
        DiagnosticDispositionV1::InvalidInput,
        "one-shot planning input was rejected",
        DiagnosticRetryV1::AfterCorrection,
    )
}

fn planning_engine_failure(
    identity: &pale_core::records::RecordSchemaIdentity,
    error: PlanError,
) -> ApiError {
    let application_disposition = error.failure_disposition();
    let (disposition, retry) = crate::error::application_diagnostic_policy(application_disposition);
    let (code, summary) = match (&error, application_disposition) {
        (PlanError::RepositoryMutationDenied { .. }, _) => (
            "repository_mutation_not_permitted",
            "the selected execution workspace admits read-only repository access",
        ),
        (PlanError::OwnershipTransition(_), _) => (
            "repository_ownership_transition_blocked",
            "existing ownership must be removed before its mode can change",
        ),
        (_, pale_application::ApplicationFailureDisposition::InvalidInput) => (
            "one_shot_plan_rejected",
            "one-shot plan claims are inconsistent with the selected compilation",
        ),
        (_, pale_application::ApplicationFailureDisposition::Blocked) => (
            "one_shot_plan_blocked",
            "one-shot planning is blocked by current admitted state",
        ),
        (_, pale_application::ApplicationFailureDisposition::Operational) => (
            "one_shot_plan_operational_failure",
            "one-shot planning could not complete its host-independent mechanics",
        ),
        (_, pale_application::ApplicationFailureDisposition::Internal) => (
            "one_shot_plan_failed",
            "one-shot planning contradicted a Pale application invariant",
        ),
    };
    planning_failure(identity, error, code, disposition, summary, retry)
}

fn saved_plan_failure(
    identity: &pale_core::records::RecordSchemaIdentity,
    error: SavedPlanAdmissionError,
) -> ApiError {
    let (disposition, retry) = crate::error::application_diagnostic_policy(error.disposition());
    planning_failure(
        identity,
        error,
        "saved_materialization_plan_rejected",
        disposition,
        "saved materialization plan was rejected",
        retry,
    )
}

fn planning_projection_failure(
    identity: &pale_core::records::RecordSchemaIdentity,
    error: PlanProjectionError,
) -> ApiError {
    planning_failure(
        identity,
        error,
        "one_shot_plan_projection_failed",
        DiagnosticDispositionV1::InternalFailure,
        "one-shot plan projection contradicted a Pale application invariant",
        DiagnosticRetryV1::NotIndicated,
    )
}

fn planning_failure<E>(
    identity: &pale_core::records::RecordSchemaIdentity,
    error: E,
    code: &'static str,
    disposition: DiagnosticDispositionV1,
    summary: &'static str,
    retry: DiagnosticRetryV1,
) -> ApiError
where
    E: Error + Send + Sync + 'static,
{
    ApiError::new_with_opaque_source(
        identity,
        error,
        FailureSpecification {
            code,
            stage: DiagnosticStageV1::Planning,
            disposition,
            summary,
            subject: None,
            schema_coordinate: None,
            location: None,
            retry,
            effect: DiagnosticEffectCertaintyV1::NotApplicable,
            request: None,
            notes: Vec::new(),
            help: Vec::new(),
            facts: Vec::new(),
        },
    )
}
