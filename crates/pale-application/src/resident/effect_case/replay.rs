//! Bounded point-read replay classification for one logical action.

use pale_core::records::{ActionEnvelopeV1, CommonRequestEnvelopeV1};
use pale_ports::{
    AuthorityStoreCoordinate, ControlRecord, ControlStoreError, EffectCaseCustodyPort,
    EffectCaseIndexObservation, EffectCaseIndexQuery,
};

use super::record::{project_request, EffectCaseKeys};
use super::{AdmittedEffectCaseRecord, EffectCaseProjectionError};

/// Restart-safe disposition found without scanning resident storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectCaseReplayDisposition {
    /// Neither request identity nor logical action has been admitted.
    New,
    /// The exact caller-signed request already exists with its current action state.
    ExactRequest {
        /// Exact immutable request already named by its request-identity key.
        request: ControlRecord,
        /// Immutable semantic admission for the logical action.
        admission: ControlRecord,
        /// Exact current immutable disposition of the logical action.
        disposition: ControlRecord,
    },
    /// The request is new, but its scoped logical action already exists.
    AliasExisting {
        /// Immutable semantic admission for the existing logical action.
        admission: ControlRecord,
        /// Exact current immutable disposition of the logical action.
        disposition: ControlRecord,
    },
}

/// Classify request replay and logical-action aliasing with at most three point reads.
///
/// Stored semantic records remain immutable evidence. This lookup neither admits
/// new bytes nor treats a current disposition as proof of a native effect.
///
/// # Errors
///
/// Returns [`EffectCaseReplayLookupError::RequestConflict`] when a caller
/// request key resolves to bytes other than the exact submitted request, or
/// [`EffectCaseReplayLookupError::IncompleteIndexes`] when resident indexes do
/// not form a complete admitted action pair.
pub fn lookup_effect_case_replay(
    port: &mut dyn EffectCaseCustodyPort,
    store: &AuthorityStoreCoordinate,
    request: AdmittedEffectCaseRecord<'_, CommonRequestEnvelopeV1>,
    action: &ActionEnvelopeV1,
) -> Result<EffectCaseReplayDisposition, EffectCaseReplayLookupError> {
    let keys = EffectCaseKeys::derive(request.record, action)?;
    let expected_request = project_request(request)?;
    classify_effect_case_replay(port, store, keys, expected_request)
}

pub(super) fn classify_effect_case_replay(
    port: &mut dyn EffectCaseCustodyPort,
    store: &AuthorityStoreCoordinate,
    keys: EffectCaseKeys,
    expected_request: ControlRecord,
) -> Result<EffectCaseReplayDisposition, EffectCaseReplayLookupError> {
    let request_query = EffectCaseIndexQuery::RequestIdentity(keys.request);
    let request_observation = read_index(port, store, &request_query)?;

    match request_observation.record() {
        Some(stored) if stored != &expected_request => {
            Err(EffectCaseReplayLookupError::RequestConflict)
        }
        Some(stored) => {
            let admission = read_required(
                port,
                store,
                EffectCaseIndexQuery::LogicalAction(keys.logical.clone()),
            )?;
            let disposition = read_required(
                port,
                store,
                EffectCaseIndexQuery::ActionDisposition(keys.logical),
            )?;
            Ok(EffectCaseReplayDisposition::ExactRequest {
                request: stored.clone(),
                admission,
                disposition,
            })
        }
        None => {
            let logical_query = EffectCaseIndexQuery::LogicalAction(keys.logical.clone());
            let logical_observation = read_index(port, store, &logical_query)?;
            match logical_observation.record() {
                None => Ok(EffectCaseReplayDisposition::New),
                Some(admission) => {
                    let disposition = read_required(
                        port,
                        store,
                        EffectCaseIndexQuery::ActionDisposition(keys.logical),
                    )?;
                    Ok(EffectCaseReplayDisposition::AliasExisting {
                        admission: admission.clone(),
                        disposition,
                    })
                }
            }
        }
    }
}

fn read_index(
    port: &mut dyn EffectCaseCustodyPort,
    store: &AuthorityStoreCoordinate,
    query: &EffectCaseIndexQuery,
) -> Result<EffectCaseIndexObservation, EffectCaseReplayLookupError> {
    let observation = port.read_index(store, query)?;
    (observation.query() == query)
        .then_some(observation)
        .ok_or(EffectCaseReplayLookupError::MisdirectedObservation)
}

fn read_required(
    port: &mut dyn EffectCaseCustodyPort,
    store: &AuthorityStoreCoordinate,
    query: EffectCaseIndexQuery,
) -> Result<ControlRecord, EffectCaseReplayLookupError> {
    read_index(port, store, &query)?
        .record()
        .cloned()
        .ok_or(EffectCaseReplayLookupError::IncompleteIndexes)
}

/// Closed replay-read failure classification.
#[derive(Debug, thiserror::Error)]
pub enum EffectCaseReplayLookupError {
    /// Application-owned canonical record or key projection failed.
    #[error(transparent)]
    Projection(#[from] EffectCaseProjectionError),
    /// The custody port failed a bounded point read.
    #[error(transparent)]
    Store(#[from] ControlStoreError),
    /// The request key already names different immutable request bytes.
    #[error("request identity resolves to different immutable bytes")]
    RequestConflict,
    /// The immutable logical-action map lacks its mutable disposition partner.
    #[error("logical-action indexes do not form a complete admitted pair")]
    IncompleteIndexes,
    /// The port returned a point-read result for a different typed query.
    #[error("effect-case port returned an observation for another point query")]
    MisdirectedObservation,
}
