//! Pure resident-supervisor state and reconciliation algebra.
//!
//! This module owns decisions, not effects. [`SupervisorPlan::admit_claimed_snapshot`]
//! structurally checks one complete set of process claims;
//! [`SupervisorSnapshot::reconcile`] then computes a deterministic pending
//! intent set. For desired state `d`, structurally admitted plan `p`, claimed
//! observation `o`, and explicit isolation policy `iota`, the pure phase is
//! rendered below. The same law uses `b` for bounded restart delay, `n` for the
//! positive restart count, `I` for initial delay, `M` for its ceiling, and `S`
//! for the bounded service set.

#![doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/supervisor-plan.display.mathml"
))]
//!
//! `prepare` validates and compares those four finite inputs; `pending` retains
//! the desired state and the ordered intents still requiring durable authority
//! and native proof. The arrow is a deterministic in-memory transformation, not
//! process execution or evidence that any intent has been applied.
//!
//! It opens no process, socket, database, service manager, or stop-sentinel
//! path. The pending value exposes no process directive until resident authority
//! persists `d`; separately selected ports must then establish every missing
//! native proof before interpretation. Process
//! claims remain distinct from compiled-generation activation,
//! producer-incarnation authority, and full operation readiness.
use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::num::{NonZeroU32, NonZeroU64};

use pale_core::records::{
    ArtifactDigest, CompiledGenerationDigest, ExecutableDigest, ResidentProcessInstanceIdV1,
    ServiceDefinition, ServiceDefinitionInput, ServiceLaunch,
};

use crate::service_topology::{normalize_owned_service_definitions, ServiceTopologyError};

/// Canonical identity of one service within an admitted supervisor plan.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SupervisorServiceId(String);

impl SupervisorServiceId {
    /// Borrow the shared lowercase service identity.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Construct only from a name admitted by the shared topology owner.
    fn from_admitted(value: String) -> Self {
        Self(value)
    }
}

impl std::fmt::Display for SupervisorServiceId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl Borrow<str> for SupervisorServiceId {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

/// Nonzero generation coordinate reported for one supervised child lineage.
///
/// This value supports checked arithmetic only. Constructing it does not prove
/// allocation order, durable custody, or monotonicity within a real lineage.
/// The lifecycle authority must establish those facts independently of this
/// numeric claim before authorizing a launcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProcessGeneration(NonZeroU64);

impl ProcessGeneration {
    /// Construct a nonzero child-process generation claim.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidProcessGeneration`] for zero.
    pub const fn new(value: u64) -> Result<Self, InvalidProcessGeneration> {
        match NonZeroU64::new(value) {
            Some(value) => Ok(Self(value)),
            None => Err(InvalidProcessGeneration::Zero),
        }
    }

    /// Return the numeric generation.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }

    /// Compute the next numeric coordinate without claiming its allocation.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidProcessGeneration::Exhausted`] at `u64::MAX`.
    pub const fn checked_successor(self) -> Result<Self, InvalidProcessGeneration> {
        match self.get().checked_add(1) {
            Some(value) => Self::new(value),
            None => Err(InvalidProcessGeneration::Exhausted),
        }
    }
}

/// Failure to construct or advance a child-process generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvalidProcessGeneration {
    /// Zero is reserved for the absence of a process lineage.
    #[error("process generation must be nonzero")]
    Zero,
    /// The current generation has no representable successor.
    #[error("process generation is exhausted")]
    Exhausted,
}

/// Explicit bounded restart and crash-loop policy for one service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RestartPolicy {
    maximum_consecutive_failures: NonZeroU32,
    initial_backoff_ms: NonZeroU64,
    maximum_backoff_ms: NonZeroU64,
}

impl RestartPolicy {
    /// Construct one policy with a nondecreasing delay ceiling.
    ///
    /// The exponential delay law is rendered on [`SupervisorPlan`]. Reaching
    /// `maximum_consecutive_failures` enters the crash-loop state instead of
    /// scheduling another restart.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidRestartPolicy`] when any input is zero or the maximum
    /// delay is smaller than the initial delay.
    pub const fn new(
        maximum_consecutive_failures: u32,
        initial_backoff_ms: u64,
        maximum_backoff_ms: u64,
    ) -> Result<Self, InvalidRestartPolicy> {
        let Some(maximum_consecutive_failures) = NonZeroU32::new(maximum_consecutive_failures)
        else {
            return Err(InvalidRestartPolicy::ZeroFailures);
        };
        let Some(initial_backoff_ms) = NonZeroU64::new(initial_backoff_ms) else {
            return Err(InvalidRestartPolicy::ZeroInitialDelay);
        };
        let Some(maximum_backoff_ms) = NonZeroU64::new(maximum_backoff_ms) else {
            return Err(InvalidRestartPolicy::ZeroMaximumDelay);
        };
        if maximum_backoff_ms.get() < initial_backoff_ms.get() {
            return Err(InvalidRestartPolicy::DescendingDelay);
        }
        Ok(Self {
            maximum_consecutive_failures,
            initial_backoff_ms,
            maximum_backoff_ms,
        })
    }

    /// Return the consecutive-failure count that closes automatic restart.
    #[must_use]
    pub const fn maximum_consecutive_failures(self) -> NonZeroU32 {
        self.maximum_consecutive_failures
    }

    /// Compute the bounded delay after one or more failures.
    fn delay_after(self, failures: NonZeroU32) -> NonZeroU64 {
        let exponent = failures.get().saturating_sub(1);
        let multiplier = 1_u64.checked_shl(exponent).unwrap_or(u64::MAX);
        let delay = self
            .initial_backoff_ms
            .get()
            .saturating_mul(multiplier)
            .min(self.maximum_backoff_ms.get());
        NonZeroU64::new(delay).unwrap_or(self.initial_backoff_ms)
    }
}

/// Invalid restart-policy shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvalidRestartPolicy {
    /// Automatic restart must have a positive failure ceiling.
    #[error("maximum consecutive failures must be positive")]
    ZeroFailures,
    /// The first retry delay must be positive.
    #[error("initial restart backoff must be positive")]
    ZeroInitialDelay,
    /// The bounded retry-delay ceiling must be positive.
    #[error("maximum restart backoff must be positive")]
    ZeroMaximumDelay,
    /// A delay ceiling cannot be smaller than the initial delay.
    #[error("maximum restart backoff must be at least the initial backoff")]
    DescendingDelay,
}

/// One provider-neutral service plus expected executable/configuration identity.
pub struct SupervisorServiceDeclaration {
    definition: ServiceDefinition,
    executable_digest: ExecutableDigest,
    configuration_digest: ArtifactDigest,
    restart_policy: RestartPolicy,
}

impl SupervisorServiceDeclaration {
    /// Bind one service declaration to exact executable/configuration identity.
    #[must_use]
    pub fn new(
        definition: ServiceDefinition,
        executable_digest: ExecutableDigest,
        configuration_digest: ArtifactDigest,
        restart_policy: RestartPolicy,
    ) -> Self {
        Self {
            definition,
            executable_digest,
            configuration_digest,
            restart_policy,
        }
    }
}

/// Admitted immutable service owned by one [`SupervisorPlan`].
pub struct SupervisedService {
    id: SupervisorServiceId,
    kind: String,
    precedence: u32,
    launch: ServiceLaunch,
    ports: Vec<u16>,
    dependencies: Vec<SupervisorServiceId>,
    dependency_indices: Vec<usize>,
    executable_digest: ExecutableDigest,
    configuration_digest: ArtifactDigest,
    restart_policy: RestartPolicy,
}

impl SupervisedService {
    /// Return the typed service identity.
    #[must_use]
    pub const fn id(&self) -> &SupervisorServiceId {
        &self.id
    }

    /// Return the caller-declared service class.
    #[must_use]
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// Return the canonical dependency-order coordinate.
    #[must_use]
    pub const fn precedence(&self) -> u32 {
        self.precedence
    }

    /// Borrow the literal launch declaration; this module never executes it.
    #[must_use]
    pub const fn launch(&self) -> &ServiceLaunch {
        &self.launch
    }

    /// Borrow the sorted globally unique port set.
    #[must_use]
    pub fn ports(&self) -> &[u16] {
        &self.ports
    }

    /// Borrow the complete transitive dependency closure in startup order.
    #[must_use]
    pub fn dependencies(&self) -> &[SupervisorServiceId] {
        &self.dependencies
    }

    /// Borrow the exact expected executable-byte identity.
    #[must_use]
    pub const fn executable_digest(&self) -> &ExecutableDigest {
        &self.executable_digest
    }

    /// Borrow the exact expected service-configuration identity.
    #[must_use]
    pub const fn configuration_digest(&self) -> &ArtifactDigest {
        &self.configuration_digest
    }

    /// Return the explicit bounded restart policy.
    #[must_use]
    pub const fn restart_policy(&self) -> RestartPolicy {
        self.restart_policy
    }
}

/// Structurally admitted service topology and immutable content coordinates.
///
/// Admission here proves only provider-neutral topology and content identity.
/// It does not select a runtime profile, allocate process lineage, establish
/// liveness, or prove full operation readiness.
pub struct SupervisorPlan {
    compiled_generation: CompiledGenerationDigest,
    services: Vec<SupervisedService>,
    service_index: BTreeMap<SupervisorServiceId, usize>,
}

impl SupervisorPlan {
    /// Admit one complete supervisor plan without starting or observing a process.
    ///
    /// Service topology is delegated to
    /// [`crate::service_topology::normalize_service_definitions`], so the native
    /// normalization action and
    /// resident supervisor cannot disagree about names, ports, dependency edges,
    /// or canonical order. It also computes each service's finite transitive
    /// dependency closure once; no reconciliation pass recomputes graph
    /// reachability.
    ///
    /// The complete service universe is bounded by
    /// [`pale_core::records::SERVICE_DEFINITION_MAX_SERVICES_V1`]. Closure
    /// construction and retained-memory bounds are rendered on
    /// [`SupervisorPlan`]. Boolean membership avoids a tree allocation for
    /// every closure row.
    ///
    /// # Errors
    ///
    /// Returns [`SupervisorPlanError`] for invalid topology or an internal
    /// contradiction between normalized topology and process coordinates.
    pub fn admit(
        compiled_generation: CompiledGenerationDigest,
        declarations: Vec<SupervisorServiceDeclaration>,
    ) -> Result<Self, SupervisorPlanError> {
        let mut definitions = Vec::with_capacity(declarations.len());
        let mut process_coordinates = BTreeMap::new();
        for declaration in declarations {
            process_coordinates.insert(
                declaration.definition.name.clone(),
                (
                    declaration.executable_digest,
                    declaration.configuration_digest,
                    declaration.restart_policy,
                ),
            );
            definitions.push(declaration.definition);
        }
        let topology = normalize_owned_service_definitions(ServiceDefinitionInput {
            services: definitions,
        })?;

        let mut services = topology
            .services
            .into_iter()
            .map(|definition| {
                let id = SupervisorServiceId::from_admitted(definition.name);
                let (executable_digest, configuration_digest, restart_policy) = process_coordinates
                    .remove(id.as_str())
                    .ok_or_else(|| SupervisorPlanError::MissingProcessCoordinates {
                        service: id.clone(),
                    })?;
                Ok(SupervisedService {
                    id,
                    kind: definition.kind,
                    precedence: definition.precedence,
                    launch: definition.launch,
                    ports: definition.ports,
                    dependencies: definition
                        .depends_on
                        .into_iter()
                        .map(SupervisorServiceId::from_admitted)
                        .collect(),
                    dependency_indices: Vec::new(),
                    executable_digest,
                    configuration_digest,
                    restart_policy,
                })
            })
            .collect::<Result<Vec<_>, SupervisorPlanError>>()?;
        let service_index = services
            .iter()
            .enumerate()
            .map(|(index, service)| (service.id.clone(), index))
            .collect::<BTreeMap<_, _>>();
        for index in 0..services.len() {
            let direct_dependencies = std::mem::take(&mut services[index].dependencies);
            let mut dependency_membership = vec![false; services.len()];
            for dependency in direct_dependencies {
                let dependency_index = *service_index.get(&dependency).ok_or_else(|| {
                    SupervisorPlanError::MissingDependencyAfterAdmission {
                        service: services[index].id.clone(),
                        dependency: dependency.clone(),
                    }
                })?;
                dependency_membership[dependency_index] = true;
                for predecessor_index in &services[dependency_index].dependency_indices {
                    dependency_membership[*predecessor_index] = true;
                }
            }
            let dependency_indices = dependency_membership
                .into_iter()
                .enumerate()
                .filter_map(|(dependency_index, retained)| retained.then_some(dependency_index))
                .collect::<Vec<_>>();
            services[index].dependencies = dependency_indices
                .iter()
                .map(|dependency_index| services[*dependency_index].id.clone())
                .collect();
            services[index].dependency_indices = dependency_indices;
        }

        Ok(Self {
            compiled_generation,
            services,
            service_index,
        })
    }

    /// Borrow the compiled generation that fences every expected child.
    #[must_use]
    pub const fn compiled_generation(&self) -> &CompiledGenerationDigest {
        &self.compiled_generation
    }

    /// Iterate services in canonical startup order.
    pub fn services(&self) -> impl ExactSizeIterator<Item = &SupervisedService> {
        self.services.iter()
    }

    /// Build one structurally expected fence claim from plan-owned content.
    ///
    /// The caller supplies the process-instance and generation coordinates.
    /// This method binds those claims to the plan's service and byte identities;
    /// it does not prove launcher allocation, durable custody, or liveness.
    ///
    /// # Errors
    ///
    /// Returns [`SupervisorPlanError::UnknownService`] if `service` does not
    /// belong to this plan.
    pub fn bind_expected_claim(
        &self,
        service: &SupervisorServiceId,
        process_instance_id: ResidentProcessInstanceIdV1,
        process_generation: ProcessGeneration,
    ) -> Result<ProcessFenceClaim, SupervisorPlanError> {
        let admitted = self.service(service)?;
        Ok(ProcessFenceClaim {
            service: service.clone(),
            process_instance_id,
            process_generation,
            compiled_generation: self.compiled_generation.clone(),
            executable_digest: admitted.executable_digest.clone(),
            configuration_digest: admitted.configuration_digest.clone(),
        })
    }

    /// Structurally admit one complete claimed snapshot against this plan.
    ///
    /// Expected fences must cover every service exactly once and must reproduce
    /// the plan's generation, executable, and configuration identities. Observed
    /// children may differ: that difference is retained for mismatch
    /// classification rather than rejected at the boundary. Process-instance
    /// identities may occur more than once only within the same service slot;
    /// cross-service reuse is rejected before reconciliation.
    ///
    /// The admission bound rendered on [`SupervisorPlan`] applies over the
    /// bounded service universe. Aligned vectors avoid a keyed lookup for each
    /// service during later reconciliation.
    ///
    /// # Errors
    ///
    /// Returns [`SupervisorObservationError`] for missing, extra, substituted,
    /// or key-mismatched expected state.
    pub fn admit_claimed_snapshot(
        &self,
        mut expected_claims: BTreeMap<SupervisorServiceId, ProcessFenceClaim>,
        mut observations: BTreeMap<SupervisorServiceId, ProcessObservationClaim>,
    ) -> Result<SupervisorSnapshot<'_>, SupervisorObservationError> {
        if !self.contains_exact_service_set(&expected_claims) {
            return Err(SupervisorObservationError::ExpectedClaimSetMismatch);
        }
        if !self.contains_exact_service_set(&observations) {
            return Err(SupervisorObservationError::ObservationSetMismatch);
        }
        for (service, fence) in &expected_claims {
            let Some(index) = self.service_index.get(service) else {
                return Err(SupervisorObservationError::ExpectedClaimSetMismatch);
            };
            let admitted = &self.services[*index];
            if fence.service != *service
                || fence.compiled_generation != self.compiled_generation
                || fence.executable_digest != admitted.executable_digest
                || fence.configuration_digest != admitted.configuration_digest
            {
                return Err(SupervisorObservationError::ExpectedClaimMismatch {
                    service: service.clone(),
                });
            }
        }
        let mut process_owners = BTreeMap::new();
        for (service, fence) in &expected_claims {
            admit_process_owner(&mut process_owners, service, fence.process_instance_id())?;
        }
        for (service, observation) in &observations {
            let process_instance_id = match observation {
                ProcessObservationClaim::Absent(_) => None,
                ProcessObservationClaim::FencedRunning { fence, .. } => {
                    Some(fence.process_instance_id())
                }
                ProcessObservationClaim::UnidentifiedRunning {
                    process_instance_id,
                } => Some(process_instance_id),
            };
            if let Some(process_instance_id) = process_instance_id {
                admit_process_owner(&mut process_owners, service, process_instance_id)?;
            }
        }
        let mut services = Vec::with_capacity(self.services.len());
        for service in &self.services {
            let expected = expected_claims
                .remove(&service.id)
                .ok_or(SupervisorObservationError::ExpectedClaimSetMismatch)?;
            let observed = observations
                .remove(&service.id)
                .ok_or(SupervisorObservationError::ObservationSetMismatch)?;
            services.push(ServiceSnapshot { expected, observed });
        }
        Ok(SupervisorSnapshot {
            plan: self,
            services,
        })
    }

    /// Resolve one service by typed identity.
    fn service(
        &self,
        service: &SupervisorServiceId,
    ) -> Result<&SupervisedService, SupervisorPlanError> {
        self.service_index
            .get(service)
            .map(|index| &self.services[*index])
            .ok_or_else(|| SupervisorPlanError::UnknownService {
                service: service.clone(),
            })
    }

    /// Whether a keyed input covers the admitted service universe exactly.
    fn contains_exact_service_set<T>(&self, values: &BTreeMap<SupervisorServiceId, T>) -> bool {
        values.len() == self.service_index.len()
            && values
                .keys()
                .all(|service| self.service_index.contains_key(service))
    }
}

/// Failure to admit a supervisor plan or resolve one of its services.
#[derive(Debug, thiserror::Error)]
pub enum SupervisorPlanError {
    /// Shared service topology is structurally invalid.
    #[error(transparent)]
    Topology(#[from] ServiceTopologyError),
    /// Normalized topology lost the process coordinates for one service.
    #[error("service `{service}` has no process coordinates after topology admission")]
    MissingProcessCoordinates {
        /// Service whose coordinates are absent.
        service: SupervisorServiceId,
    },
    /// Normalized topology lost one previously admitted dependency edge.
    #[error("service `{service}` lost dependency `{dependency}` after topology admission")]
    MissingDependencyAfterAdmission {
        /// Dependent service.
        service: SupervisorServiceId,
        /// Missing dependency identity.
        dependency: SupervisorServiceId,
    },
    /// A typed service identity does not belong to this plan.
    #[error("service `{service}` is not present in the supervisor plan")]
    UnknownService {
        /// Unknown typed identity.
        service: SupervisorServiceId,
    },
}

/// Raw generation-fenced process identity supplied to the pure reducer.
///
/// Equality proves only that two claims carry the same coordinates. It does not
/// prove launcher allocation, process liveness, probe execution, or custody in a
/// selected control store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessFenceClaim {
    service: SupervisorServiceId,
    process_instance_id: ResidentProcessInstanceIdV1,
    process_generation: ProcessGeneration,
    compiled_generation: CompiledGenerationDigest,
    executable_digest: ExecutableDigest,
    configuration_digest: ArtifactDigest,
}

impl ProcessFenceClaim {
    /// Construct one raw process-fence claim.
    ///
    /// The constructor performs no native observation and grants no authority.
    #[must_use]
    pub fn new(
        service: SupervisorServiceId,
        process_instance_id: ResidentProcessInstanceIdV1,
        process_generation: ProcessGeneration,
        compiled_generation: CompiledGenerationDigest,
        executable_digest: ExecutableDigest,
        configuration_digest: ArtifactDigest,
    ) -> Self {
        Self {
            service,
            process_instance_id,
            process_generation,
            compiled_generation,
            executable_digest,
            configuration_digest,
        }
    }

    /// Return the service lineage fenced by this coordinate.
    #[must_use]
    pub const fn service(&self) -> &SupervisorServiceId {
        &self.service
    }

    /// Borrow the claimed process-instance identity.
    #[must_use]
    pub const fn process_instance_id(&self) -> &ResidentProcessInstanceIdV1 {
        &self.process_instance_id
    }

    /// Return the claimed service-process generation coordinate.
    #[must_use]
    pub const fn process_generation(&self) -> ProcessGeneration {
        self.process_generation
    }

    /// Borrow the claimed compiled-generation coordinate.
    #[must_use]
    pub const fn compiled_generation(&self) -> &CompiledGenerationDigest {
        &self.compiled_generation
    }
}

/// Raw readiness-probe claim attached to one process-fence claim.
///
/// This enum records input to the pure dependency reducer. Its variants do not
/// prove that a selected native observer ran or that full operation readiness
/// has been established.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceProbeClaim {
    /// The caller reports a healthy service prerequisite.
    Healthy,
    /// The caller reports an unhealthy service prerequisite.
    Unhealthy(ServiceProbeUnhealthyReason),
    /// The caller cannot classify this service prerequisite.
    Unknown,
}

/// Closed reason reported for an unhealthy service-prerequisite claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceProbeUnhealthyReason {
    /// Startup has not completed its declared readiness transition.
    Starting,
    /// The caller reports an explicit unhealthy probe result.
    ProbeFailed,
}

/// Progress of one absent child after zero or more consecutive failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbsentProcessState {
    /// No retry delay or crash-loop state blocks a start.
    Startable,
    /// The bounded delay after these failures has not elapsed.
    BackoffPending {
        /// Positive consecutive-failure count.
        consecutive_failures: NonZeroU32,
    },
    /// The bounded delay elapsed; the failure count remains for crash-loop law.
    BackoffElapsed {
        /// Positive consecutive-failure count.
        consecutive_failures: NonZeroU32,
    },
}

/// Raw process observation supplied to the pure reconciliation phase.
///
/// Construction does not authenticate the source or establish native custody.
/// Before a computed intent can cross an effect boundary, the caller must join
/// these claims to independently verified native observations and lifecycle
/// authority. Structural admission by [`SupervisorPlan::admit_claimed_snapshot`]
/// does not supply either proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessObservationClaim {
    /// No child currently occupies the service's process slot.
    Absent(AbsentProcessState),
    /// The caller reports a child with a complete fence and probe claim.
    FencedRunning {
        /// Complete claimed process fence.
        fence: ProcessFenceClaim,
        /// Probe claim bound structurally to `fence`.
        probe: ServiceProbeClaim,
    },
    /// The caller reports a process without a complete fence.
    UnidentifiedRunning {
        /// Process-instance identity available to quarantine or termination policy.
        process_instance_id: ResidentProcessInstanceIdV1,
    },
}

/// Restart disposition derived from one absent-child state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartDisposition {
    /// The service may start immediately if dependencies are ready.
    Startable,
    /// The service must wait for the exact bounded retry delay.
    Backoff {
        /// Delay selected by [`RestartPolicy`].
        delay_ms: NonZeroU64,
    },
    /// Automatic restart is closed pending an explicit operator transition.
    CrashLoop {
        /// Failure count that reached or exceeded the policy ceiling.
        consecutive_failures: NonZeroU32,
    },
}

/// Apply one service's restart law without reading a clock or sleeping.
#[must_use]
pub fn classify_restart(policy: RestartPolicy, state: AbsentProcessState) -> RestartDisposition {
    match state {
        AbsentProcessState::Startable => RestartDisposition::Startable,
        AbsentProcessState::BackoffPending {
            consecutive_failures,
        }
        | AbsentProcessState::BackoffElapsed {
            consecutive_failures,
        } if consecutive_failures >= policy.maximum_consecutive_failures => {
            RestartDisposition::CrashLoop {
                consecutive_failures,
            }
        }
        AbsentProcessState::BackoffPending {
            consecutive_failures,
        } => RestartDisposition::Backoff {
            delay_ms: policy.delay_after(consecutive_failures),
        },
        AbsentProcessState::BackoffElapsed { .. } => RestartDisposition::Startable,
    }
}

/// Desired process state for the complete admitted supervisor plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesiredProcessState {
    /// Every admitted service is desired in canonical dependency order.
    Running,
    /// Every admitted service is desired stopped in reverse dependency order.
    Stopped,
}

impl DesiredProcessState {
    /// Apply one idempotent lifecycle command.
    #[must_use]
    pub const fn transition(self, command: SupervisorCommand) -> DesiredStateTransition {
        let next = match command {
            SupervisorCommand::Up => Self::Running,
            SupervisorCommand::Down => Self::Stopped,
        };
        if matches!(
            (self, next),
            (Self::Running, Self::Running) | (Self::Stopped, Self::Stopped)
        ) {
            DesiredStateTransition::Unchanged(next)
        } else {
            DesiredStateTransition::Changed {
                previous: self,
                next,
            }
        }
    }

    /// Derive the required local [`StopSentinelIntent`].
    #[must_use]
    pub const fn stop_sentinel(self) -> StopSentinelIntent {
        match self {
            Self::Running => StopSentinelIntent::EnsureAbsent,
            Self::Stopped => StopSentinelIntent::EnsurePresent,
        }
    }
}

/// Thin lifecycle command accepted by the pure desired-state reducer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupervisorCommand {
    /// Desire the complete plan running and clear the stop sentinel.
    Up,
    /// Desire the complete plan stopped and retain the stop sentinel.
    Down,
}

/// Result of applying an idempotent desired-state command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesiredStateTransition {
    /// The command changed desired state.
    Changed {
        /// State before the command.
        previous: DesiredProcessState,
        /// State after the command.
        next: DesiredProcessState,
    },
    /// The requested state was already desired.
    Unchanged(DesiredProcessState),
}

/// Filesystem intent for the service-manager stop sentinel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopSentinelIntent {
    /// Running state requires no sentinel.
    EnsureAbsent,
    /// Stopped state requires a sentinel before supervisor exit.
    EnsurePresent,
}

/// Policy-selected treatment for a process claim not structurally retainable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessIsolationAction {
    /// Fence the child from admission and retain it for operator inspection.
    Quarantine,
    /// Fence the child and request bounded process termination.
    Terminate,
}

/// Explicit policy for mismatching fenced and unidentified process claims.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessIsolationPolicy {
    fenced_mismatch: ProcessIsolationAction,
    unidentified: ProcessIsolationAction,
}

impl ProcessIsolationPolicy {
    /// Select mismatching and unidentified claim dispositions without a default.
    #[must_use]
    pub const fn new(
        fenced_mismatch: ProcessIsolationAction,
        unidentified: ProcessIsolationAction,
    ) -> Self {
        Self {
            fenced_mismatch,
            unidentified,
        }
    }
}

/// Why one process claim cannot match its structurally expected claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessClaimMismatch {
    /// A complete claimed fence differs from the expected claim.
    FencedMismatch,
    /// A process claim omits its complete fence.
    Unidentified,
}

/// Exact claimed identity carried into an isolation intent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IsolatedProcessClaim {
    /// Complete but mismatching fence claim.
    Fenced(ProcessFenceClaim),
    /// Process instance without a complete fence claim.
    Unidentified(ResidentProcessInstanceIdV1),
}

/// Typed reason one process prerequisite blocks a dependent service start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServicePrerequisiteFailureReason {
    /// No child occupies the service slot.
    ProcessAbsent,
    /// A process claim exists without a complete fence claim.
    ProcessIdentityUnproven,
    /// The claimed fence differs from the structurally expected claim.
    FenceMismatch,
    /// Probe state for the matching claim is unknown.
    ProbeUnknown,
    /// Probe state for the matching claim is explicitly unhealthy.
    ProbeUnhealthy(ServiceProbeUnhealthyReason),
}

/// One service-specific readiness failure with its exact typed owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServicePrerequisiteFailure {
    service: SupervisorServiceId,
    reason: ServicePrerequisiteFailureReason,
}

impl ServicePrerequisiteFailure {
    /// Return the service whose prerequisite claim failed.
    #[must_use]
    pub const fn service(&self) -> &SupervisorServiceId {
        &self.service
    }

    /// Return the exact failure class.
    #[must_use]
    pub const fn reason(&self) -> ServicePrerequisiteFailureReason {
        self.reason
    }
}

/// One ordered effect-free service directive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceDirective {
    service: SupervisorServiceId,
    action: ServiceDirectiveAction,
}

impl ServiceDirective {
    /// Return the service owning this directive.
    #[must_use]
    pub const fn service(&self) -> &SupervisorServiceId {
        &self.service
    }

    /// Borrow the exact requested action or hold state.
    #[must_use]
    pub const fn action(&self) -> &ServiceDirectiveAction {
        &self.action
    }
}

/// Closed action/hold state produced for one service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceDirectiveAction {
    /// Request a start under this structurally expected claim.
    StartExpected(ProcessFenceClaim),
    /// Retain a process whose raw claim exactly matches the expected claim.
    RetainMatching(ProcessFenceClaim),
    /// Dependency or retry state currently blocks a start.
    Hold(ServiceStartHold),
    /// Request a graceful stop for the exactly matching claim.
    StopMatching(ProcessFenceClaim),
    /// The desired-stopped service already has no process.
    AlreadyStopped,
    /// Fence a mismatching or unidentified claim under the explicit policy.
    Isolate {
        /// Why structural process-claim matching failed.
        mismatch: ProcessClaimMismatch,
        /// Complete fenced or partial claimed identity.
        child: IsolatedProcessClaim,
        /// Policy-selected effect intent; no effect occurs here.
        action: ProcessIsolationAction,
    },
}

/// Typed reason an absent service cannot start in the current reconciliation pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceStartHold {
    /// One or more transitive dependencies lack a satisfied fence/probe claim.
    Dependencies(Vec<ServicePrerequisiteFailure>),
    /// The retry delay selected by [`RestartPolicy`] has not elapsed.
    RestartBackoff {
        /// Exact bounded delay.
        delay_ms: NonZeroU64,
    },
    /// The service reached its bounded consecutive-failure ceiling.
    CrashLoop {
        /// Failure count closing automatic restart.
        consecutive_failures: NonZeroU32,
    },
}

/// One ordered post-persistence phase in a supervisor reconciliation.
///
/// Values are exposed only through [`SupervisorReconciliation`], after the
/// desired-state journal barrier has succeeded or proved the same state already
/// current. The stop sentinel therefore precedes every process directive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SupervisorReconciliationStep {
    /// Apply the local [`StopSentinelIntent`] after durable desired state.
    ApplyStopSentinel(StopSentinelIntent),
    /// Apply one process intent after the sentinel phase.
    Service(ServiceDirective),
}

/// Pure reconciliation withheld behind the durable desired-state barrier.
///
/// The public observations are the plan's [`Self::compiled_generation`] and
/// [`Self::desired_process_state`], which a resident authority owner must check
/// and commit. Sentinel and process directives remain
/// inaccessible until
/// [`ResidentApplication::commit_supervisor_reconciliation`](crate::resident::ResidentApplication::commit_supervisor_reconciliation)
/// returns a committed [`SupervisorReconciliation`].
#[derive(Debug)]
pub struct PendingSupervisorReconciliation {
    compiled_generation: CompiledGenerationDigest,
    desired_process_state: DesiredProcessState,
    steps: Vec<SupervisorReconciliationStep>,
}

impl PendingSupervisorReconciliation {
    /// Borrow the plan's originating compilation, not process-launch authority.
    #[must_use]
    pub const fn compiled_generation(&self) -> &CompiledGenerationDigest {
        &self.compiled_generation
    }

    /// Return the state that must cross the durable authority boundary first.
    #[must_use]
    pub const fn desired_process_state(&self) -> DesiredProcessState {
        self.desired_process_state
    }

    /// Release process intents only to the resident authority coordinator.
    pub(crate) fn into_committed(self) -> SupervisorReconciliation {
        SupervisorReconciliation {
            compiled_generation: self.compiled_generation,
            desired_process_state: self.desired_process_state,
            steps: self.steps,
        }
    }
}

/// Complete ordered reconciliation after desired-state persistence.
///
/// The enforced effect order is rendered on [`SupervisorPlan`]. Its semicolon
/// is a durability barrier, not list concatenation: no process intent is
/// observable through this type before desired-state persistence succeeds.
/// The retained origin is a compiled-content identity, not an activation fence
/// or permission to launch a child after later resident state changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupervisorReconciliation {
    compiled_generation: CompiledGenerationDigest,
    desired_process_state: DesiredProcessState,
    steps: Vec<SupervisorReconciliationStep>,
}

impl SupervisorReconciliation {
    /// Borrow the exact plan origin retained through desired-state persistence.
    #[must_use]
    pub const fn compiled_generation(&self) -> &CompiledGenerationDigest {
        &self.compiled_generation
    }

    /// Return the desired state proven durable before these steps were exposed.
    #[must_use]
    pub const fn desired_process_state(&self) -> DesiredProcessState {
        self.desired_process_state
    }

    /// Borrow the only post-persistence application order.
    #[must_use]
    pub fn steps(&self) -> &[SupervisorReconciliationStep] {
        &self.steps
    }
}

/// One service state aligned with [`SupervisorPlan::services`].
struct ServiceSnapshot {
    expected: ProcessFenceClaim,
    observed: ProcessObservationClaim,
}

/// One complete plan-bound set of structurally checked process claims.
pub struct SupervisorSnapshot<'plan> {
    plan: &'plan SupervisorPlan,
    services: Vec<ServiceSnapshot>,
}

impl SupervisorSnapshot<'_> {
    /// Prepare every service in deterministic startup or reverse-shutdown order.
    ///
    /// Dependency checks use the transitive topology closure. A matching raw
    /// fence claim may be retained, but coordinate equality establishes no
    /// native verification. Mismatching and unidentified process
    /// claims are never retained. All variants are effect-free intents withheld
    /// behind the desired-state persistence barrier. The worst-case pass bound
    /// is rendered on [`SupervisorPlan`]; the initial-v1 service bound keeps
    /// that work finite and deterministic.
    #[must_use]
    pub fn reconcile(
        &self,
        desired: DesiredProcessState,
        isolation_policy: ProcessIsolationPolicy,
    ) -> PendingSupervisorReconciliation {
        let mut steps = Vec::with_capacity(self.plan.services.len() + 1);
        steps.push(SupervisorReconciliationStep::ApplyStopSentinel(
            desired.stop_sentinel(),
        ));
        let mut append_service = |index: usize| {
            let service = &self.plan.services[index];
            steps.push(SupervisorReconciliationStep::Service(ServiceDirective {
                service: service.id.clone(),
                action: self.directive(index, desired, isolation_policy),
            }));
        };
        match desired {
            DesiredProcessState::Running => {
                (0..self.plan.services.len()).for_each(&mut append_service);
            }
            DesiredProcessState::Stopped => {
                (0..self.plan.services.len())
                    .rev()
                    .for_each(&mut append_service);
            }
        }
        PendingSupervisorReconciliation {
            compiled_generation: self.plan.compiled_generation.clone(),
            desired_process_state: desired,
            steps,
        }
    }

    /// Classify one service under the desired state without causing an effect.
    fn directive(
        &self,
        service_index: usize,
        desired: DesiredProcessState,
        policy: ProcessIsolationPolicy,
    ) -> ServiceDirectiveAction {
        let service = &self.plan.services[service_index];
        let snapshot = &self.services[service_index];
        let expected = &snapshot.expected;
        match (desired, &snapshot.observed) {
            (DesiredProcessState::Running, ProcessObservationClaim::Absent(state)) => {
                match classify_restart(service.restart_policy, *state) {
                    RestartDisposition::Backoff { delay_ms } => {
                        ServiceDirectiveAction::Hold(ServiceStartHold::RestartBackoff { delay_ms })
                    }
                    RestartDisposition::CrashLoop {
                        consecutive_failures,
                    } => ServiceDirectiveAction::Hold(ServiceStartHold::CrashLoop {
                        consecutive_failures,
                    }),
                    RestartDisposition::Startable => {
                        let failures = self.prerequisite_failures(&service.dependency_indices);
                        if failures.is_empty() {
                            ServiceDirectiveAction::StartExpected(expected.clone())
                        } else {
                            ServiceDirectiveAction::Hold(ServiceStartHold::Dependencies(failures))
                        }
                    }
                }
            }
            (
                DesiredProcessState::Running,
                ProcessObservationClaim::FencedRunning { fence, .. },
            ) if fence == expected => ServiceDirectiveAction::RetainMatching(fence.clone()),
            (
                DesiredProcessState::Stopped,
                ProcessObservationClaim::FencedRunning { fence, .. },
            ) if fence == expected => ServiceDirectiveAction::StopMatching(fence.clone()),
            (_, ProcessObservationClaim::FencedRunning { fence, .. }) => {
                ServiceDirectiveAction::Isolate {
                    mismatch: ProcessClaimMismatch::FencedMismatch,
                    child: IsolatedProcessClaim::Fenced(fence.clone()),
                    action: policy.fenced_mismatch,
                }
            }
            (
                _,
                ProcessObservationClaim::UnidentifiedRunning {
                    process_instance_id,
                },
            ) => ServiceDirectiveAction::Isolate {
                mismatch: ProcessClaimMismatch::Unidentified,
                child: IsolatedProcessClaim::Unidentified(process_instance_id.clone()),
                action: policy.unidentified,
            },
            (DesiredProcessState::Stopped, ProcessObservationClaim::Absent(_)) => {
                ServiceDirectiveAction::AlreadyStopped
            }
        }
    }

    /// Accumulate failures over one exact transitive prerequisite closure.
    fn prerequisite_failures(&self, required_indices: &[usize]) -> Vec<ServicePrerequisiteFailure> {
        required_indices
            .iter()
            .filter_map(|index| {
                let service = &self.plan.services[*index];
                let snapshot = &self.services[*index];
                let reason = match &snapshot.observed {
                    ProcessObservationClaim::Absent(_) => {
                        Some(ServicePrerequisiteFailureReason::ProcessAbsent)
                    }
                    ProcessObservationClaim::UnidentifiedRunning { .. } => {
                        Some(ServicePrerequisiteFailureReason::ProcessIdentityUnproven)
                    }
                    ProcessObservationClaim::FencedRunning { fence, .. }
                        if fence != &snapshot.expected =>
                    {
                        Some(ServicePrerequisiteFailureReason::FenceMismatch)
                    }
                    ProcessObservationClaim::FencedRunning {
                        probe: ServiceProbeClaim::Unknown,
                        ..
                    } => Some(ServicePrerequisiteFailureReason::ProbeUnknown),
                    ProcessObservationClaim::FencedRunning {
                        probe: ServiceProbeClaim::Unhealthy(reason),
                        ..
                    } => Some(ServicePrerequisiteFailureReason::ProbeUnhealthy(*reason)),
                    ProcessObservationClaim::FencedRunning {
                        probe: ServiceProbeClaim::Healthy,
                        ..
                    } => None,
                };
                reason.map(|reason| ServicePrerequisiteFailure {
                    service: service.id.clone(),
                    reason,
                })
            })
            .collect()
    }
}

/// Failure to structurally admit a complete plan-bound process-claim set.
#[derive(Debug, thiserror::Error)]
pub enum SupervisorObservationError {
    /// Expected claims do not cover exactly the plan service set.
    #[error("expected-claim service set does not equal the supervisor plan")]
    ExpectedClaimSetMismatch,
    /// Runtime claims do not cover exactly the plan service set.
    #[error("observation service set does not equal the supervisor plan")]
    ObservationSetMismatch,
    /// An expected claim substitutes a plan-owned immutable coordinate.
    #[error("expected claim for `{service}` does not match its supervisor plan")]
    ExpectedClaimMismatch {
        /// Service whose expected claim substituted plan-owned content.
        service: SupervisorServiceId,
    },
    /// One process-instance identity appears in two distinct service slots.
    #[error(
        "process instance `{process_instance_id:?}` is claimed by both `{first_service}` and `{second_service}`"
    )]
    CrossServiceProcessAlias {
        /// Aliased process-instance identity.
        process_instance_id: ResidentProcessInstanceIdV1,
        /// First service slot claiming the identity.
        first_service: SupervisorServiceId,
        /// Distinct later service slot claiming the identity.
        second_service: SupervisorServiceId,
    },
}

/// Admit one process identity into the snapshot's single-service ownership map.
fn admit_process_owner(
    process_owners: &mut BTreeMap<ResidentProcessInstanceIdV1, SupervisorServiceId>,
    service: &SupervisorServiceId,
    process_instance_id: &ResidentProcessInstanceIdV1,
) -> Result<(), SupervisorObservationError> {
    if let Some(first_service) = process_owners.get(process_instance_id) {
        if first_service != service {
            return Err(SupervisorObservationError::CrossServiceProcessAlias {
                process_instance_id: process_instance_id.clone(),
                first_service: first_service.clone(),
                second_service: service.clone(),
            });
        }
        return Ok(());
    }
    process_owners.insert(process_instance_id.clone(), service.clone());
    Ok(())
}

#[cfg(test)]
#[path = "../../tests/internal/resident/process_reconciliation.rs"]
mod tests;
