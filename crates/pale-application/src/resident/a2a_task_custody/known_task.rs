//! Exact retained coordinates shared by known-task read and cancellation services.

use super::RecoveredA2aTaskHistory;
use crate::resident::IssuedLocalOperation;
use pale_core::records::*;
use pale_ports::{ControlRecord, ControlRecordCoordinate};
use std::time::Duration;

/// One captured local/remote task history. It is evidence, not current authority.
pub(in crate::resident) struct KnownA2aTaskBasis {
    pub(in crate::resident::a2a_task_custody) operation:
        (ResidentRecordRefV1, OperationHandleDigest),
    pub(in crate::resident::a2a_task_custody) request: ControlRecordCoordinate,
    pub(in crate::resident::a2a_task_custody) operation_lease:
        (ResidentRecordRefV1, OperationLeaseDigest),
    pub(in crate::resident::a2a_task_custody) selection: ControlRecordCoordinate,
    pub(in crate::resident::a2a_task_custody) dispatch:
        (ResidentRecordRefV1, OperationObservationDigest),
    pub(in crate::resident::a2a_task_custody) local_head:
        (ResidentRecordRefV1, OperationObservationDigest),
    pub(in crate::resident::a2a_task_custody) route: (ResidentRecordRefV1, A2aRouteAdmissionDigest),
    pub(in crate::resident::a2a_task_custody) binding: (ResidentRecordRefV1, A2aTaskBindingDigest),
    pub(in crate::resident::a2a_task_custody) remote_head:
        (ResidentRecordRefV1, A2aTaskObservationDigest),
    pub(in crate::resident::a2a_task_custody) authorized_at: UnixTimeV1,
    pub(in crate::resident::a2a_task_custody) earliest_observation_at: UnixTimeV1,
    pub(in crate::resident::a2a_task_custody) retain_until: UnixTimeV1,
}

impl KnownA2aTaskBasis {
    pub(super) fn capture(
        operation: &IssuedLocalOperation,
        selection: &ControlRecord,
        history: &RecoveredA2aTaskHistory,
        authorized_at: UnixTimeV1,
    ) -> Self {
        let handle = operation.handle();
        let dispatch = history.dispatch();
        let local_head = history.local_observation().selected_head();
        let route = history.route();
        let binding = history.binding();
        let remote_head = history.observation();
        Self {
            operation: (
                handle.0.clone(),
                handle.1.canonical_operation_handle_sha256().clone(),
            ),
            request: operation.request_coordinate().clone(),
            operation_lease: (
                handle.1.operation_lease_ref().clone(),
                handle.1.operation_lease_sha256().clone(),
            ),
            selection: selection.coordinate().clone(),
            dispatch: (
                dispatch.0.clone(),
                dispatch.1.canonical_operation_observation_sha256().clone(),
            ),
            local_head: (
                local_head.0.clone(),
                local_head
                    .1
                    .canonical_operation_observation_sha256()
                    .clone(),
            ),
            route: (
                route.0.clone(),
                route.1.canonical_a2a_route_admission_sha256().clone(),
            ),
            binding: (
                binding.0.clone(),
                binding.1.canonical_a2a_task_binding_sha256().clone(),
            ),
            remote_head: (
                remote_head.0.clone(),
                remote_head
                    .1
                    .canonical_a2a_task_observation_sha256()
                    .clone(),
            ),
            authorized_at,
            earliest_observation_at: std::cmp::max(
                *local_head.1.observed_at(),
                *remote_head.1.observed_at(),
            ),
            retain_until: *handle.1.retain_until(),
        }
    }

    pub(super) fn contains_record_ref(&self, candidate: &ResidentRecordRefV1) -> bool {
        candidate == &self.operation.0
            || candidate == self.request.record_ref()
            || candidate == &self.operation_lease.0
            || candidate == self.selection.record_ref()
            || candidate == &self.dispatch.0
            || candidate == &self.local_head.0
            || candidate == &self.route.0
            || candidate == &self.binding.0
            || candidate == &self.remote_head.0
    }

    pub(super) fn admits_observation_time(&self, observed_at: UnixTimeV1) -> bool {
        observed_at >= self.authorized_at
            && observed_at >= self.earliest_observation_at
            && observed_at <= self.retain_until
    }

    pub(super) fn matches_without_local_head(
        &self,
        operation: &IssuedLocalOperation,
        history: &RecoveredA2aTaskHistory,
    ) -> bool {
        let handle = operation.handle();
        let dispatch = history.dispatch();
        let route = history.route();
        let binding = history.binding();
        let remote_head = history.observation();
        &self.operation.0 == handle.0
            && &self.operation.1 == handle.1.canonical_operation_handle_sha256()
            && &self.request == operation.request_coordinate()
            && &self.operation_lease.0 == handle.1.operation_lease_ref()
            && &self.operation_lease.1 == handle.1.operation_lease_sha256()
            && &self.dispatch.0 == dispatch.0
            && &self.dispatch.1 == dispatch.1.canonical_operation_observation_sha256()
            && &self.route.0 == route.0
            && &self.route.1 == route.1.canonical_a2a_route_admission_sha256()
            && &self.binding.0 == binding.0
            && &self.binding.1 == binding.1.canonical_a2a_task_binding_sha256()
            && &self.remote_head.0 == remote_head.0
            && &self.remote_head.1 == remote_head.1.canonical_a2a_task_observation_sha256()
    }

    pub(super) const fn retain_until(&self) -> UnixTimeV1 {
        self.retain_until
    }

    pub(super) fn admits_exchange_deadline(&self, now: UnixTimeV1, timeout: Duration) -> bool {
        let Some(rounded_timeout) = timeout
            .as_secs()
            .checked_add(u64::from(timeout.subsec_nanos() != 0))
        else {
            return false;
        };
        now.get()
            .checked_add(rounded_timeout)
            .is_some_and(|deadline| deadline <= self.retain_until.get())
    }
}
