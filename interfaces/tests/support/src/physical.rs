//! Deterministic physical-port models for resident interface tests.

use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Mutex;

use pale_core::records::{
    CanonicalSignatureV1, ProducerEpochV1, ProducerIdV1, ResidentPrincipalIdV1, UnixTimeV1,
};
use pale_ports::{
    AntiRollbackAnchor, AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError,
    AntiRollbackAnchorKey, AntiRollbackAnchorObservation, AntiRollbackAnchorPort,
    ControlRecordCoordinate, ControlStoreError, ControlStoreErrorKind,
    CryptographicRandomSourceError, CryptographicRandomSourcePort,
    DeploymentFinalizationSetReadLimits, DeploymentFinalizationSetReadPort,
    DeploymentFinalizationSetSnapshot, PreparedAntiRollbackAnchorTransition,
    PrincipalProducerIncarnationHeadPage, PrincipalProducerIncarnationHeadPageBoundary,
    PrincipalProducerIncarnationHeadPageRead, PrincipalProducerIncarnationReadObservation,
    PrincipalProducerIncarnationReadPort, PrincipalResidentWriterReadPort,
    PrincipalResidentWriterSnapshot, PrincipalTrustCheckpointSetReadLimits,
    PrincipalTrustCheckpointSetReadPort, PrincipalTrustCheckpointSetSnapshot,
    PrincipalTrustSnapshot, PrincipalTrustStoreBinding, PrincipalWriterGuardPort,
    WallClockSourceError, WallClockSourcePort,
};

const TEST_SIGNATURE: &str =
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

fn recovery_error() -> ControlStoreError {
    ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired)
}

fn bound_error() -> ControlStoreError {
    ControlStoreError::new(ControlStoreErrorKind::BoundExceeded)
}

/// Return the fixed, syntactically valid signature used by conformance models.
///
/// This value proves deterministic byte flow only. It is not a cryptographic
/// signature and must never be used as production evidence.
#[must_use]
pub fn deterministic_signature() -> CanonicalSignatureV1 {
    CanonicalSignatureV1::parse(TEST_SIGNATURE).expect("fixed test signature is valid base64url")
}

/// Queue-backed physical clock with no relationship to wall time.
pub struct DeterministicClock(Mutex<VecDeque<UnixTimeV1>>);

impl DeterministicClock {
    /// Retain the exact sequence returned by later clock observations.
    #[must_use]
    pub fn new(values: impl IntoIterator<Item = UnixTimeV1>) -> Self {
        Self(Mutex::new(values.into_iter().collect()))
    }
}

impl WallClockSourcePort for DeterministicClock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        self.0
            .lock()
            .map_err(|_| {
                WallClockSourceError::from_source(std::io::Error::other("test clock lock failed"))
            })?
            .pop_front()
            .ok_or_else(|| {
                WallClockSourceError::from_source(std::io::Error::other("test clock is exhausted"))
            })
    }
}

/// Repeatable non-cryptographic byte source for conformance tests.
pub struct DeterministicEntropy(AtomicU8);

impl DeterministicEntropy {
    /// Begin with the supplied byte; each call uses the next byte value.
    #[must_use]
    pub const fn new(initial: u8) -> Self {
        Self(AtomicU8::new(initial))
    }
}

impl CryptographicRandomSourcePort for DeterministicEntropy {
    fn fill_bytes(&self, destination: &mut [u8]) -> Result<(), CryptographicRandomSourceError> {
        let byte = self.0.fetch_add(1, Ordering::SeqCst).wrapping_add(1);
        destination.fill(byte);
        Ok(())
    }
}

/// Held-principal model for a development-only writer guard.
pub struct TestWriterGuard(ResidentPrincipalIdV1);

impl TestWriterGuard {
    /// Bind the guard to one exact principal.
    #[must_use]
    pub const fn new(principal_id: ResidentPrincipalIdV1) -> Self {
        Self(principal_id)
    }
}

impl PrincipalWriterGuardPort for TestWriterGuard {
    fn principal_id(&self) -> &ResidentPrincipalIdV1 {
        &self.0
    }
}

/// One bounded, single-use resident-writer physical snapshot.
pub struct InMemoryResidentWriterReader {
    expected_binding: PrincipalTrustStoreBinding,
    minimum_byte_limit: usize,
    snapshot: Option<PrincipalResidentWriterSnapshot>,
}

impl InMemoryResidentWriterReader {
    /// Retain one exact physical writer result and its minimum accepted bound.
    #[must_use]
    pub const fn new(
        expected_binding: PrincipalTrustStoreBinding,
        minimum_byte_limit: usize,
        snapshot: Option<PrincipalResidentWriterSnapshot>,
    ) -> Self {
        Self {
            expected_binding,
            minimum_byte_limit,
            snapshot,
        }
    }
}

impl PrincipalResidentWriterReadPort for InMemoryResidentWriterReader {
    fn read_current_writer(
        &mut self,
        binding: &PrincipalTrustStoreBinding,
        maximum_record_bytes: usize,
    ) -> Result<Option<PrincipalResidentWriterSnapshot>, ControlStoreError> {
        if binding != &self.expected_binding {
            return Err(recovery_error());
        }
        if maximum_record_bytes < self.minimum_byte_limit {
            return Err(bound_error());
        }
        Ok(self.snapshot.take())
    }
}

/// One bounded, single-use trust-checkpoint physical snapshot.
pub struct InMemoryCheckpointSetReader {
    expected_binding: PrincipalTrustStoreBinding,
    required_limits: PrincipalTrustCheckpointSetReadLimits,
    snapshot: Option<PrincipalTrustCheckpointSetSnapshot>,
}

impl InMemoryCheckpointSetReader {
    /// Retain one exact checkpoint-set result.
    #[must_use]
    pub const fn new(
        expected_binding: PrincipalTrustStoreBinding,
        required_limits: PrincipalTrustCheckpointSetReadLimits,
        snapshot: PrincipalTrustCheckpointSetSnapshot,
    ) -> Self {
        Self {
            expected_binding,
            required_limits,
            snapshot: Some(snapshot),
        }
    }
}

impl PrincipalTrustCheckpointSetReadPort for InMemoryCheckpointSetReader {
    fn read_checkpoint_set(
        &mut self,
        binding: &PrincipalTrustStoreBinding,
        limits: PrincipalTrustCheckpointSetReadLimits,
    ) -> Result<Option<PrincipalTrustCheckpointSetSnapshot>, ControlStoreError> {
        if binding != &self.expected_binding {
            return Err(recovery_error());
        }
        if limits.maximum_checkpoints() < self.required_limits.maximum_checkpoints()
            || limits.maximum_record_bytes() < self.required_limits.maximum_record_bytes()
        {
            return Err(bound_error());
        }
        Ok(self.snapshot.take())
    }
}

/// Physical producer reader for a principal with no producer incarnations.
pub struct EmptyPrincipalProducerReader {
    expected_binding: PrincipalTrustStoreBinding,
}

impl EmptyPrincipalProducerReader {
    /// Bind physical absence to one exact principal store.
    #[must_use]
    pub const fn new(expected_binding: PrincipalTrustStoreBinding) -> Self {
        Self { expected_binding }
    }
}

impl PrincipalProducerIncarnationReadPort for EmptyPrincipalProducerReader {
    fn read_by_epoch(
        &mut self,
        binding: &PrincipalTrustStoreBinding,
        _: &ProducerEpochV1,
    ) -> Result<PrincipalProducerIncarnationReadObservation, ControlStoreError> {
        if binding != &self.expected_binding {
            return Err(recovery_error());
        }
        Ok(PrincipalProducerIncarnationReadObservation::Absent)
    }

    fn read_head(
        &mut self,
        binding: &PrincipalTrustStoreBinding,
        _: &ProducerIdV1,
    ) -> Result<PrincipalProducerIncarnationReadObservation, ControlStoreError> {
        if binding != &self.expected_binding {
            return Err(recovery_error());
        }
        Ok(PrincipalProducerIncarnationReadObservation::Absent)
    }

    fn read_by_allocation(
        &mut self,
        snapshot: &PrincipalTrustSnapshot,
        _: &ControlRecordCoordinate,
    ) -> Result<PrincipalProducerIncarnationReadObservation, ControlStoreError> {
        if snapshot.binding() != &self.expected_binding {
            return Err(recovery_error());
        }
        Ok(PrincipalProducerIncarnationReadObservation::Absent)
    }

    fn read_heads_page(
        &mut self,
        request: &PrincipalProducerIncarnationHeadPageRead,
    ) -> Result<PrincipalProducerIncarnationHeadPage, ControlStoreError> {
        if request.binding() != &self.expected_binding {
            return Err(recovery_error());
        }
        PrincipalProducerIncarnationHeadPage::new(
            request,
            Vec::new(),
            PrincipalProducerIncarnationHeadPageBoundary::Complete,
        )
        .map_err(|_| recovery_error())
    }
}

/// One bounded, single-use deployment-finalization physical snapshot.
pub struct InMemoryDeploymentFinalizationReader {
    expected_binding: PrincipalTrustStoreBinding,
    required_limits: DeploymentFinalizationSetReadLimits,
    snapshot: Option<DeploymentFinalizationSetSnapshot>,
}

impl InMemoryDeploymentFinalizationReader {
    /// Retain an exact finalization result; `None` proves physical absence.
    #[must_use]
    pub const fn new(
        expected_binding: PrincipalTrustStoreBinding,
        required_limits: DeploymentFinalizationSetReadLimits,
        snapshot: Option<DeploymentFinalizationSetSnapshot>,
    ) -> Self {
        Self {
            expected_binding,
            required_limits,
            snapshot,
        }
    }
}

impl DeploymentFinalizationSetReadPort for InMemoryDeploymentFinalizationReader {
    fn read_finalization_set(
        &mut self,
        binding: &PrincipalTrustStoreBinding,
        limits: DeploymentFinalizationSetReadLimits,
    ) -> Result<Option<DeploymentFinalizationSetSnapshot>, ControlStoreError> {
        if binding != &self.expected_binding {
            return Err(recovery_error());
        }
        if limits.maximum_finalizations() < self.required_limits.maximum_finalizations()
            || limits.maximum_record_bytes() < self.required_limits.maximum_record_bytes()
            || limits.maximum_mutations() < self.required_limits.maximum_mutations()
        {
            return Err(bound_error());
        }
        Ok(self.snapshot.take())
    }
}

/// In-memory anti-rollback map used only to test application ordering laws.
///
/// This map shares the process and failure domain of the test. It is not an
/// independently durable rollback defense despite the trait it simulates.
#[derive(Default)]
pub struct SimulatedIndependentAnchor {
    values: BTreeMap<AntiRollbackAnchorKey, AntiRollbackAnchor>,
}

impl SimulatedIndependentAnchor {
    /// Seed one already-retained anchor value.
    pub fn insert(&mut self, value: AntiRollbackAnchor) {
        self.values.insert(value.key(), value);
    }
}

impl AntiRollbackAnchorPort for SimulatedIndependentAnchor {
    fn read(
        &mut self,
        key: &AntiRollbackAnchorKey,
    ) -> Result<Option<AntiRollbackAnchor>, AntiRollbackAnchorError> {
        Ok(self.values.get(key).cloned())
    }

    fn compare_and_advance(
        &mut self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError> {
        let observed = self.values.get(transition.key());
        let classification = transition.classify_observed(observed);
        match classification {
            AntiRollbackAnchorObservation::ExactSuccessor => {
                Ok(AntiRollbackAnchorAdvanceDisposition::ExactReplay)
            }
            AntiRollbackAnchorObservation::Missing if transition.expected().is_none() => {
                self.values
                    .insert(transition.key().clone(), transition.next().clone());
                Ok(AntiRollbackAnchorAdvanceDisposition::Advanced)
            }
            AntiRollbackAnchorObservation::ExactPredecessor => {
                self.values
                    .insert(transition.key().clone(), transition.next().clone());
                Ok(AntiRollbackAnchorAdvanceDisposition::Advanced)
            }
            other => Ok(AntiRollbackAnchorAdvanceDisposition::NotAdvanced(other)),
        }
    }

    fn observe(
        &mut self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError> {
        Ok(transition.classify_observed(self.values.get(transition.key())))
    }
}
