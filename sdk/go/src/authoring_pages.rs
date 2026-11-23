//! Retained authoring page calls and response-owned continuation transfer.

use pale_sdk::{
    AuthoringOccurrencePage, AuthoringOccurrencePageRequest, AuthoringOccurrenceRole,
    AuthoringSourceLocation, EffectiveProfileCoordinates, EffectiveProfileEntry,
};
use serde::{Deserialize, Serialize};

use crate::bridge_state::{self, ClientState, PendingResponse, PendingResponseValues, Response};
use crate::{decode, diagnostic, encode, success, BridgeFailure};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OccurrenceRequest {
    handle: u64,
    identity: u64,
    maximum_inspected: u16,
    maximum_returned: u16,
    after: Option<u64>,
}

impl OccurrenceRequest {
    fn query(&self, state: &ClientState) -> Result<AuthoringOccurrencePageRequest, BridgeFailure> {
        let identity = state
            .authoring_identities
            .get(&self.identity)
            .ok_or_else(|| BridgeFailure::invalid_handle("authoring identity", self.identity))?;
        let request = AuthoringOccurrencePageRequest::new(
            identity.clone(),
            self.maximum_inspected,
            self.maximum_returned,
        );
        match self.after {
            None => Ok(request),
            Some(handle) => state
                .authoring_occurrence_cursors
                .get(&handle)
                .cloned()
                .map(|cursor| request.after(cursor))
                .ok_or_else(|| {
                    BridgeFailure::invalid_handle("authoring occurrence cursor", handle)
                }),
        }
    }
}

#[derive(Serialize)]
struct Occurrence<'a> {
    location: &'a AuthoringSourceLocation,
    role: AuthoringOccurrenceRole,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OccurrenceMetadata<'a> {
    occurrences: Vec<Occurrence<'a>>,
    inspected: u16,
    copied_string_bytes: u32,
    is_complete: bool,
    continuation_handle: Option<u64>,
}

fn occurrence_response(
    client: u64,
    page: AuthoringOccurrencePage,
) -> Result<Response, BridgeFailure> {
    let continuation = page
        .continuation()
        .cloned()
        .map(|cursor| (bridge_state::next_object_handle(), cursor));
    let metadata = encode(&OccurrenceMetadata {
        occurrences: page
            .occurrences()
            .iter()
            .map(|item| Occurrence {
                location: item.location(),
                role: item.role(),
            })
            .collect(),
        inspected: page.inspected(),
        copied_string_bytes: page.copied_string_bytes(),
        is_complete: page.is_complete(),
        continuation_handle: continuation.as_ref().map(|(handle, _)| *handle),
    })?;
    let mut response = success(metadata, Vec::new(), 0);
    response.pending_values = continuation.map(|(handle, cursor)| {
        Box::new(PendingResponse {
            client,
            values: PendingResponseValues::AuthoringOccurrenceCursor { handle, cursor },
        })
    });
    Ok(response)
}

pub(crate) fn portable_occurrences(client: u64, bytes: &[u8]) -> Result<Response, BridgeFailure> {
    let request: OccurrenceRequest = decode(bytes)?;
    bridge_state::with_client(client, |state| {
        let workspace = state
            .workspaces
            .get(&request.handle)
            .ok_or_else(|| BridgeFailure::invalid_handle("portable workspace", request.handle))?;
        match workspace.authoring_occurrences(request.query(state)?) {
            Ok(page) => occurrence_response(client, page),
            Err(error) => diagnostic(error),
        }
    })
}

pub(crate) fn effective_occurrences(client: u64, bytes: &[u8]) -> Result<Response, BridgeFailure> {
    let request: OccurrenceRequest = decode(bytes)?;
    bridge_state::with_client(client, |state| {
        let profile = state
            .profiles
            .get(&request.handle)
            .ok_or_else(|| BridgeFailure::invalid_handle("effective profile", request.handle))?;
        match profile.authoring_occurrences(request.query(state)?) {
            Ok(page) => occurrence_response(client, page),
            Err(error) => diagnostic(error),
        }
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProfileRequest {
    handle: u64,
    identity: u64,
    limit: u16,
    after: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProfileMetadata<'a> {
    coordinates: &'a EffectiveProfileCoordinates,
    entries: &'a [EffectiveProfileEntry],
    continuation_handle: Option<u64>,
}

pub(crate) fn effective_page(client: u64, bytes: &[u8]) -> Result<Response, BridgeFailure> {
    let request: ProfileRequest = decode(bytes)?;
    bridge_state::with_client(client, |state| {
        let profile = state
            .profiles
            .get(&request.handle)
            .ok_or_else(|| BridgeFailure::invalid_handle("effective profile", request.handle))?;
        let identity = state
            .authoring_identities
            .get(&request.identity)
            .ok_or_else(|| BridgeFailure::invalid_handle("authoring identity", request.identity))?;
        let after = request
            .after
            .map(|handle| {
                state
                    .effective_authoring_cursors
                    .get(&handle)
                    .ok_or_else(|| {
                        BridgeFailure::invalid_handle("effective authoring cursor", handle)
                    })
            })
            .transpose()?;
        match profile.inspect_effective_authoring_page(identity, after, request.limit) {
            Ok(page) => {
                let continuation = page
                    .continuation()
                    .cloned()
                    .map(|cursor| (bridge_state::next_object_handle(), cursor));
                let metadata = encode(&ProfileMetadata {
                    coordinates: page.coordinates(),
                    entries: page.entries(),
                    continuation_handle: continuation.as_ref().map(|(handle, _)| *handle),
                })?;
                let mut response = success(metadata, Vec::new(), 0);
                response.pending_values = continuation.map(|(handle, cursor)| {
                    Box::new(PendingResponse {
                        client,
                        values: PendingResponseValues::EffectiveAuthoringCursor { handle, cursor },
                    })
                });
                Ok(response)
            }
            Err(error) => diagnostic(error),
        }
    })
}
