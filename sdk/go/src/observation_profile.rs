//! Forward observation-to-profile queries without retaining another profile.

use pale_sdk::{
    EffectiveProfileCoordinates, EffectiveProfileEntry, ObservationProfileError,
    ObservationProfileMapping,
};
use serde::{Deserialize, Serialize};

use crate::bridge_state::{self, PendingResponse, PendingResponseValues, Response};
use crate::{decode, encode, success, BridgeFailure};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Request {
    snapshot: u64,
    profile: u64,
    workspace: u64,
    subject: u64,
    maximum_inspected: u16,
    maximum_returned: u16,
    after: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
enum Mapping {
    PrimaryRows,
    NoPrimaryRows,
}

impl From<ObservationProfileMapping> for Mapping {
    fn from(value: ObservationProfileMapping) -> Self {
        match value {
            ObservationProfileMapping::PrimaryRows => Self::PrimaryRows,
            ObservationProfileMapping::NoPrimaryRows => Self::NoPrimaryRows,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
enum Refusal {
    NoProfileSource,
    ExpiredSource,
    SourceMismatch,
    CoordinateMismatch,
    ContinuationMismatch,
    InvalidLimit,
}

impl From<ObservationProfileError> for Refusal {
    fn from(value: ObservationProfileError) -> Self {
        match value {
            ObservationProfileError::NoProfileSource => Self::NoProfileSource,
            ObservationProfileError::ExpiredSource => Self::ExpiredSource,
            ObservationProfileError::SourceMismatch => Self::SourceMismatch,
            ObservationProfileError::CoordinateMismatch => Self::CoordinateMismatch,
            ObservationProfileError::ContinuationMismatch => Self::ContinuationMismatch,
            ObservationProfileError::InvalidLimit => Self::InvalidLimit,
        }
    }
}

#[derive(Serialize)]
#[serde(
    tag = "state",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
enum Outcome<'a> {
    Ready {
        coordinates: &'a EffectiveProfileCoordinates,
        mapping: Mapping,
        entries: &'a [EffectiveProfileEntry],
        inspected: u16,
        continuation_handle: Option<u64>,
    },
    Refused {
        reason: Refusal,
    },
}

pub(crate) fn inspect(client: u64, bytes: &[u8]) -> Result<Response, BridgeFailure> {
    let request: Request = decode(bytes)?;
    bridge_state::with_client(client, |state| {
        let snapshot = state
            .observation_snapshots
            .get(&request.snapshot)
            .ok_or_else(|| {
                BridgeFailure::invalid_handle("observation snapshot", request.snapshot)
            })?;
        let profile = state
            .profiles
            .get(&request.profile)
            .ok_or_else(|| BridgeFailure::invalid_handle("effective profile", request.profile))?;
        let workspace = state.workspaces.get(&request.workspace).ok_or_else(|| {
            BridgeFailure::invalid_handle("portable workspace", request.workspace)
        })?;
        let subject = state
            .observation_subjects
            .get(&request.subject)
            .ok_or_else(|| BridgeFailure::invalid_handle("observation subject", request.subject))?;
        let after = request
            .after
            .map(|handle| {
                state
                    .observation_profile_cursors
                    .get(&handle)
                    .ok_or_else(|| {
                        BridgeFailure::invalid_handle("observation profile continuation", handle)
                    })
            })
            .transpose()?;
        match snapshot.inspect_profile(
            profile,
            workspace,
            subject,
            after,
            request.maximum_inspected,
            request.maximum_returned,
        ) {
            Ok(page) => {
                let continuation = page
                    .continuation
                    .map(|cursor| (bridge_state::next_object_handle(), cursor));
                let metadata = encode(&Outcome::Ready {
                    coordinates: &page.coordinates,
                    mapping: page.mapping.into(),
                    entries: &page.entries,
                    inspected: page.inspected,
                    continuation_handle: continuation.as_ref().map(|(handle, _)| *handle),
                })?;
                let mut response = success(metadata, Vec::new(), 0);
                response.pending_values = continuation.map(|(handle, cursor)| {
                    Box::new(PendingResponse {
                        client,
                        values: PendingResponseValues::ObservationProfileCursor { handle, cursor },
                    })
                });
                Ok(response)
            }
            Err(reason) => Ok(success(
                encode(&Outcome::Refused {
                    reason: reason.into(),
                })?,
                Vec::new(),
                0,
            )),
        }
    })
}
