//! In-memory application frontier for outer-interface conformance.
//!
//! This feature-only module constructs admitted bootstrap, deployment,
//! generation, session, writer, and operation evidence, then recovers the
//! ordinary resident frame service. One callback runs Start and Advance. A
//! second callback observes the same operation after another cold recovery.
//!
//! Deterministic development signatures and memory-backed custody keep the
//! proof reproducible. The frontier proves application ownership, public
//! framing, and same-handle recovery. It does not prove production transport,
//! key custody, clocks, entropy, crash durability, trust-chain replay,
//! independent anti-rollback media, installation, activation, or deployment.

use pale_core::{canon, records::*};
use pale_ports::*;

use super::frame_service::ResidentFrameService;
use crate::resident::*;

mod evidence;
mod frontier;
mod memory;

#[path = "../../../tests/support/interface_consult_restart.rs"]
mod restart_test_support;
pub use restart_test_support::{
    exercise_consult_service_expiry_restart, exercise_consult_service_restart,
};

use evidence::*;
pub use frontier::owners::{
    exercise_consult_child_ambiguous_restart, exercise_consult_child_collection_anchor_restart,
    exercise_consult_child_prefix_service_restart, exercise_consult_child_service_restart,
    exercise_consult_child_suffix_service_restart,
};
use frontier::*;
use memory::*;

// Current-request admission must reconstruct this exact signed evidence time.
fn conformance_ingress_time() -> UnixTimeV1 {
    UnixTimeV1::new(61).expect("conformance ingress time")
}

/// Signed request factory for one in-memory operation conformance journey.
///
/// The factory carries the admitted Start request and its session activation
/// fence. It creates an Advance request after Start returns the real operation
/// locator and a separate targetless Observe request. It contains no predicted
/// operation identity or precomputed resident response.
#[derive(Clone, Debug)]
pub struct OperationConformanceRequestFactory {
    start_signed_common_request: String,
    activation_fence: ActivationFenceDigest,
}

impl OperationConformanceRequestFactory {
    /// Borrow the signed Start request.
    #[must_use]
    pub fn start_signed_common_request(&self) -> &str {
        &self.start_signed_common_request
    }

    /// Construct a signed Advance request for a locator returned by Start.
    ///
    /// # Errors
    /// Returns a construction error if the retained Start cannot be admitted
    /// or the derived request cannot be serialized canonically.
    pub fn advance_signed_common_request(
        &self,
        locator: &ResidentOperationFrameLocator,
    ) -> Result<String, OperationConformanceFrontierError> {
        self.advance_request(
            locator,
            "interface-conformance-advance",
            ConformanceRequestFreshness::Current,
        )
    }

    /// Construct a signed but expired Advance request for a negative case.
    ///
    /// # Errors
    /// Returns a construction error if the retained Start cannot be admitted
    /// or the expired request cannot be serialized canonically.
    pub fn expired_advance_signed_common_request(
        &self,
        locator: &ResidentOperationFrameLocator,
    ) -> Result<String, OperationConformanceFrontierError> {
        self.advance_request(
            locator,
            "interface-conformance-expired-advance",
            ConformanceRequestFreshness::Expired,
        )
    }

    /// Construct the signed read-only Observe request from the returned handle.
    ///
    /// The Observe frame carries the application-issued locator. The common
    /// request keeps the production Observe owner's targetless request shape
    /// and carries the exact signed handle as its `operation-handle.v1` body.
    ///
    /// # Errors
    /// Rejects a noncanonical handle, a digest unlike the locator, an invalid
    /// retained Start, or a request that cannot be serialized canonically.
    pub fn observe_signed_common_request(
        &self,
        locator: &ResidentOperationFrameLocator,
        operation_handle_canonical: &[u8],
    ) -> Result<String, OperationConformanceFrontierError> {
        let operation_handle: serde_json::Value =
            canon::parse_strict(operation_handle_canonical)
                .map_err(|_| OperationConformanceFrontierError::Construction)?;
        let canonical = canon::to_canonical_string(&operation_handle)
            .map_err(|_| OperationConformanceFrontierError::Construction)?;
        let handle_digest = operation_handle
            .get("canonical_operation_handle_sha256")
            .and_then(serde_json::Value::as_str)
            .ok_or(OperationConformanceFrontierError::Construction)?;
        if canonical.as_bytes() != operation_handle_canonical
            || handle_digest != locator.operation_handle_sha256()
        {
            return Err(OperationConformanceFrontierError::Construction);
        }
        let start = self.admitted_start_request()?;
        canon::to_canonical_string(&observe_request(
            &start,
            operation_handle,
            &self.activation_fence,
        ))
        .map_err(|_| OperationConformanceFrontierError::Construction)
    }

    fn advance_request(
        &self,
        locator: &ResidentOperationFrameLocator,
        suffix: &str,
        freshness: ConformanceRequestFreshness,
    ) -> Result<String, OperationConformanceFrontierError> {
        self.request_for_phase(
            Some(locator),
            ConformanceRequestPhase::Advance,
            suffix,
            freshness,
        )
    }

    fn request_for_phase(
        &self,
        locator: Option<&ResidentOperationFrameLocator>,
        phase: ConformanceRequestPhase,
        suffix: &str,
        freshness: ConformanceRequestFreshness,
    ) -> Result<String, OperationConformanceFrontierError> {
        let start = self.admitted_start_request()?;
        canon::to_canonical_string(&phase_request(&start, phase, locator, suffix, freshness))
            .map_err(|_| OperationConformanceFrontierError::Construction)
    }

    fn admitted_start_request(
        &self,
    ) -> Result<CommonRequestEnvelopeV1, OperationConformanceFrontierError> {
        let wire: CommonRequestEnvelopeWireV1 =
            serde_json::from_str(&self.start_signed_common_request)
                .map_err(|_| OperationConformanceFrontierError::Construction)?;
        let preimage = wire.untrusted_preimage();
        let context = CommonRequestAdmissionContextV1 {
            expected_schema_sha256: &preimage.envelope_schema_sha256,
            expected_body_schema_id: &preimage.body.schema_id,
            expected_body_schema_sha256: &preimage.body.schema_sha256,
            caller: &preimage.caller,
            signing_authority: &preimage.signing_authority_binding,
            route: &preimage.route,
            session_admission: &preimage.session_admission,
            operation_target: None,
            execution_owner_fence: None,
            federation_claim: None,
            workspace_id: &preimage.workspace_id,
            capability_id: &preimage.capability_id,
            operation: &preimage.operation,
            scope: &preimage.scope,
            risk_class: &preimage.risk_class,
            resource_constraints: &preimage.resource_constraints,
            expected_precondition: None,
            idempotency_key: &preimage.idempotency_key,
            issued_at: preimage.issued_at,
            expires_at: preimage.expires_at,
            generation: preimage.generation.as_ref(),
            fence_token: preimage.fence_token.as_ref(),
        };
        CommonRequestEnvelopeV1::admit_contextual_wire(wire, &context, |_, _, _| Ok(()))
            .map_err(|_| OperationConformanceFrontierError::Construction)
    }
}

/// Failure to construct or recover the in-memory conformance frontier.
#[derive(Debug, thiserror::Error)]
pub enum OperationConformanceFrontierError {
    /// Admitted deterministic evidence could not be constructed.
    #[error("interface-conformance evidence could not be constructed")]
    Construction,
    /// The ordinary resident service rejected the supplied frontier.
    #[error(transparent)]
    Service(#[from] ResidentServiceAssemblyError),
}

#[derive(Default)]
struct ConsultConformanceCounters {
    start_policy: std::sync::atomic::AtomicUsize,
    completion_policy: std::sync::atomic::AtomicUsize,
    initial_collection: std::sync::atomic::AtomicUsize,
    finite_turn: std::sync::atomic::AtomicUsize,
    historical_previous_collections: std::sync::atomic::AtomicUsize,
    historical_stopped_collections: std::sync::atomic::AtomicUsize,
    current_stopped_collections: std::sync::atomic::AtomicUsize,
    executor_polls: std::sync::atomic::AtomicUsize,
    child_stop_join_or_acceptance_attempts: std::sync::atomic::AtomicUsize,
    clock_reads: std::sync::atomic::AtomicUsize,
    identity_entropy_fills: std::sync::atomic::AtomicUsize,
    handle_signatures: std::sync::atomic::AtomicUsize,
}

/// Application activity retained for one Consult conformance journey.
///
/// These counters prove which bounded application owners ran. They expose no
/// private fixture record, operation identity, or production telemetry.
pub struct ConsultConformanceActivity {
    counters: std::sync::Arc<ConsultConformanceCounters>,
}

impl ConsultConformanceActivity {
    /// Return how many times the root Consult policy was selected.
    #[must_use]
    pub fn start_policy_calls(&self) -> usize {
        self.counters
            .start_policy
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Return how many times the retained completion policy was evaluated.
    #[must_use]
    pub fn completion_policy_calls(&self) -> usize {
        self.counters
            .completion_policy
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Return how many initial collection input sets were lent.
    #[must_use]
    pub fn initial_collection_calls(&self) -> usize {
        self.counters
            .initial_collection
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Return how many bounded continuation turns were lent.
    #[must_use]
    pub fn finite_turn_calls(&self) -> usize {
        self.counters
            .finite_turn
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Return how many non-blocking executor polls occurred.
    #[must_use]
    pub fn executor_polls(&self) -> usize {
        self.counters
            .executor_polls
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Return attempted child acceptance, stop, or join input requests.
    #[must_use]
    pub fn child_stop_join_or_acceptance_attempts(&self) -> usize {
        self.counters
            .child_stop_join_or_acceptance_attempts
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Return how many fresh physical-time observations the first service made.
    #[must_use]
    pub fn clock_reads(&self) -> usize {
        self.counters
            .clock_reads
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Return how many bounded entropy fills allocated operation identity.
    #[must_use]
    pub fn identity_entropy_fills(&self) -> usize {
        self.counters
            .identity_entropy_fills
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Return how many parent operation handles were signed.
    #[must_use]
    pub fn handle_signatures(&self) -> usize {
        self.counters
            .handle_signatures
            .load(std::sync::atomic::Ordering::SeqCst)
    }
}

/// Lend an in-memory Invoke frontier over one application-owned custody.
///
/// The first callback starts and advances one real operation, then returns its
/// application-issued locator. The function cold-recovers a second service and
/// lends it to the Observe callback with that exact locator. Both services use
/// the same operation custody and anti-rollback model. No service or private
/// fixture value can escape this function.
///
/// # Errors
///
/// Returns an error when evidence construction, cold recovery, or ordinary
/// resident-service assembly fails.
pub fn with_in_memory_invoke_conformance_frontier<Intermediate, Output>(
    use_start_and_advance: impl FnOnce(
        &mut ResidentFrameService<'_>,
        &OperationConformanceRequestFactory,
    ) -> (ResidentOperationFrameLocator, Intermediate),
    use_observe: impl FnOnce(
        &mut ResidentFrameService<'_>,
        &OperationConformanceRequestFactory,
        &ResidentOperationFrameLocator,
        Intermediate,
    ) -> Output,
) -> Result<Output, OperationConformanceFrontierError> {
    with_in_memory_invoke_conformance_frontier_for_peer(
        pale_ports::LocalIpcPeerCredentials::new(501, 20, Some(8_081)),
        use_start_and_advance,
        use_observe,
    )
}

/// Lend the same in-memory conformance service for one exact supplied native peer.
///
/// Socket-interface tests can supply their observed kernel peer rather than
/// relabeling native credentials as the older channel fixture's coordinates.
/// This changes only the test peer comparison; custody, signatures and anchors
/// remain synthetic and no production admission is established.
///
/// # Errors
///
/// Returns the existing conformance construction or recovery failure.
///
/// # Panics
/// Panics if deterministic fixture evidence violates its construction
/// invariants or either caller-supplied callback panics.
pub fn with_in_memory_invoke_conformance_frontier_for_peer<Intermediate, Output>(
    expected_peer: pale_ports::LocalIpcPeerCredentials,
    use_start_and_advance: impl FnOnce(
        &mut ResidentFrameService<'_>,
        &OperationConformanceRequestFactory,
    ) -> (ResidentOperationFrameLocator, Intermediate),
    use_observe: impl FnOnce(
        &mut ResidentFrameService<'_>,
        &OperationConformanceRequestFactory,
        &ResidentOperationFrameLocator,
        Intermediate,
    ) -> Output,
) -> Result<Output, OperationConformanceFrontierError> {
    let mut state = ConformanceResidentState::prepare();
    state.expected_peer = expected_peer;
    let mut custody = ConformanceOperationCustody::new(state.store.clone());
    let mut anchors = ConformanceAntiRollbackAnchors::default();
    let mut first_source = state.take_generation_source();
    let mut second_source = state.take_generation_source();
    let mut first_events = UnavailableEventCustody;
    let mut second_events = UnavailableEventCustody;

    let output = state.session.with_admitted_session(
        state.generation.record(),
        &state.profile,
        &state.lifecycle.active,
        &state.lifecycle.fence,
        &state.trust_ref,
        &state.trust,
        |session, current, lease| {
            let start = start_request(session, current, lease);
            let requests = OperationConformanceRequestFactory {
                start_signed_common_request: canon::to_canonical_string(&start)
                    .expect("admitted request canonicalizes"),
                activation_fence: session.lease().activation_fence_digest().clone(),
            };
            let evidence = current_evidence(current);
            let start_policy = invoke_policy(lease, &start);
            let advance = phase_request(
                &start,
                ConformanceRequestPhase::Advance,
                None,
                "interface-conformance-advance-policy",
                ConformanceRequestFreshness::Current,
            );
            let repository_id = session.lease().repository_id().clone();
            let observe_policy = observe_service_policy(
                &state.generation,
                repository_id.clone(),
                &start,
                session.lease().activation_fence_digest(),
            );
            let request_policies = ConformanceInvokeRequestPolicies::new(
                service_policy(&state.generation, repository_id.clone(), &start),
                service_policy(&state.generation, repository_id, &advance),
                observe_policy.clone(),
            );

            let (locator, intermediate) = with_cold_recovered_invoke_observe_service(
                &state,
                &mut first_source,
                &mut first_events,
                &mut custody,
                &mut anchors,
                evidence.clone(),
                request_policies.clone(),
                Some(start_policy),
                ConformanceCryptography::new(&state.store, &start, &state.session),
                ConformanceInvokeInputs::new(
                    &state.store,
                    &state.historical,
                    &state.session,
                    &start,
                ),
                UnavailableObserveInputs,
                |service| use_start_and_advance(service, &requests),
            )?;

            with_cold_recovered_observe_service(
                &state,
                &mut second_source,
                &mut second_events,
                &mut custody,
                &mut anchors,
                evidence,
                observe_policy,
                &start,
                &locator,
                recovery_limits(),
                |service| use_observe(service, &requests, &locator, intermediate),
            )
        },
    )?;
    Ok(output)
}

/// Lend an in-memory Consult frontier over one application-owned custody.
///
/// The first exact Consult Start can return unavailable while the application
/// advances its retained start and continuation phases. Resubmitting the same
/// signed Start bytes recovers the original durable Start records; it does not
/// repeat identity allocation or signing. A changed request cannot consume the
/// retained continuation. Advance then reports that continuation, and a second
/// service cold-recovers the same operation. This proves the public application
/// policy seam. It does not prove compiler-selected roster policy, child
/// dispatch, provider readiness, or fan-in.
///
/// # Errors
///
/// Returns an error when evidence construction, cold recovery, or ordinary
/// resident-service assembly fails.
///
/// # Panics
/// Panics if deterministic fixture evidence violates its construction
/// invariants or either caller-supplied callback panics.
pub fn with_in_memory_consult_conformance_frontier<Intermediate, Output>(
    use_start_and_advance: impl FnOnce(
        &mut ResidentFrameService<'_>,
        &OperationConformanceRequestFactory,
    ) -> (ResidentOperationFrameLocator, Intermediate),
    use_observe: impl FnOnce(
        &mut ResidentFrameService<'_>,
        &OperationConformanceRequestFactory,
        &ResidentOperationFrameLocator,
        &ConsultConformanceActivity,
        Intermediate,
    ) -> Output,
) -> Result<Output, OperationConformanceFrontierError> {
    let mut state = ConformanceResidentState::prepare();
    let mut custody = ConformanceOperationCustody::new(state.store.clone());
    let mut anchors = ConformanceAntiRollbackAnchors::default();
    let mut first_source = state.take_generation_source();
    let mut second_source = state.take_generation_source();
    let mut first_events = UnavailableEventCustody;
    let mut second_events = UnavailableEventCustody;
    let counters = std::sync::Arc::new(ConsultConformanceCounters::default());
    let activity = ConsultConformanceActivity {
        counters: std::sync::Arc::clone(&counters),
    };

    let output = state.session.with_admitted_session(
        state.generation.record(),
        &state.profile,
        &state.lifecycle.active,
        &state.lifecycle.fence,
        &state.trust_ref,
        &state.trust,
        |session, current, lease| {
            let start = start_request(session, current, lease);
            let requests = OperationConformanceRequestFactory {
                start_signed_common_request: canon::to_canonical_string(&start)
                    .expect("admitted request canonicalizes"),
                activation_fence: session.lease().activation_fence_digest().clone(),
            };
            let advance = phase_request(
                &start,
                ConformanceRequestPhase::Advance,
                None,
                "interface-conformance-consult-advance-policy",
                ConformanceRequestFreshness::Current,
            );
            let repository_id = session.lease().repository_id().clone();
            let observe_policy = observe_service_policy(
                &state.generation,
                repository_id.clone(),
                &start,
                session.lease().activation_fence_digest(),
            );
            let consult_request_policies = ConformanceConsultRequestPolicies::new(
                service_policy(&state.generation, repository_id.clone(), &start),
                service_policy(&state.generation, repository_id.clone(), &advance),
                observe_policy.clone(),
            );
            let evidence = current_evidence(current);
            let policy = consult_policy(lease, &start, std::sync::Arc::clone(&counters));

            let (locator, intermediate) = with_cold_recovered_consult_observe_service(
                &state,
                &mut first_source,
                &mut first_events,
                &mut custody,
                &mut anchors,
                evidence.clone(),
                consult_request_policies,
                policy,
                ConformanceCryptography::new_for_consult(
                    &state.store,
                    &start,
                    &state.session,
                    std::sync::Arc::clone(&counters),
                ),
                ConformanceInvokeInputs::new_for_consult(
                    &state.store,
                    &state.historical,
                    &state.session,
                    &start,
                ),
                ConformanceConsultInputs::new(
                    &state.store,
                    &state.session,
                    &start,
                    std::sync::Arc::clone(&counters),
                ),
                UnavailableObserveInputs,
                |service| use_start_and_advance(service, &requests),
            )?;

            with_cold_recovered_observe_service(
                &state,
                &mut second_source,
                &mut second_events,
                &mut custody,
                &mut anchors,
                evidence,
                observe_policy,
                &start,
                &locator,
                consult_recovery_limits(),
                |service| use_observe(service, &requests, &locator, &activity, intermediate),
            )
        },
    )?;
    Ok(output)
}

#[cfg(test)]
#[path = "../../../tests/internal/resident/process_host/interface_test_support.rs"]
mod tests;
