//! Bounded point-read classification of full Event identity.

use pale_core::records::{
    EventReferenceV1, GenealogyEdgeV1, InteractionOccurrenceV1, ReconciliationFrontierV1,
};
use pale_ports::{
    AuthorityStoreCoordinate, ControlRecord, ControlStoreError, EventCustodyPort, EventIndexQuery,
};

use super::projection::{
    event_identity_key, genealogy_edge_key, occurrence_key, public_record,
    reconciliation_frontier_key,
};
use super::{AdmittedEventRecord, EventCustodyProjectionError};

/// Restart-safe classification of one full canonical Event identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventIdentityDisposition {
    /// No event exists under the full producer-scoped identity.
    New,
    /// The exact canonical Event bytes already exist.
    ExactReplay {
        /// Immutable incumbent record.
        incumbent: ControlRecord,
    },
    /// The identity exists with different canonical bytes and must be quarantined.
    Conflict {
        /// Immutable incumbent record that remains authoritative.
        incumbent: ControlRecord,
    },
}

/// Restart-safe classification for one immutable derived Event record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DerivedEventRecordDisposition {
    /// No record exists under the deterministic semantic key.
    New,
    /// The exact canonical record already exists.
    ExactReplay {
        /// Immutable incumbent record.
        incumbent: ControlRecord,
    },
    /// The semantic key exists with different canonical bytes.
    Conflict {
        /// Immutable incumbent record that remains authoritative.
        incumbent: ControlRecord,
    },
}

/// Restart-safe classification for one subject's reconciliation frontier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconciliationFrontierDisposition {
    /// The subject has no retained frontier.
    New,
    /// The exact canonical frontier already exists.
    ExactReplay {
        /// Immutable incumbent record.
        incumbent: ControlRecord,
    },
    /// The subject has a different retained frontier that may be a predecessor.
    Current {
        /// Immutable current record; semantic succession remains a separate check.
        incumbent: ControlRecord,
    },
}

/// Classify one full Event identity with a single typed point read.
///
/// Exact replay does not prepare a transition and cannot produce continuation
/// authority. A conflict preserves the incumbent; the caller may separately
/// prepare [`super::EventCustodyTransitionInput::QuarantineIdentityConflict`].
///
/// # Errors
///
/// Returns [`EventIdentityLookupError`] when projection, storage, or the typed
/// response coordinate fails.
pub fn classify_event_identity(
    port: &mut dyn EventCustodyPort,
    store: &AuthorityStoreCoordinate,
    event: AdmittedEventRecord<'_, EventReferenceV1>,
    maximum_body_bytes: usize,
) -> Result<EventIdentityDisposition, EventIdentityLookupError> {
    event.record.verify()?;
    let query = EventIndexQuery::EventIdentity(event_identity_key(event.record)?);
    let expected = public_record(event, event.record.canonical_event_sha256().as_str())?;
    let observation = port.read_index(store, &query, maximum_body_bytes)?;
    if observation.query() != &query {
        return Err(EventIdentityLookupError::MisdirectedObservation);
    }
    Ok(match observation.record() {
        None => EventIdentityDisposition::New,
        Some(incumbent) if same_canonical_record(incumbent, &expected) => {
            EventIdentityDisposition::ExactReplay {
                incumbent: incumbent.clone(),
            }
        }
        Some(incumbent) => EventIdentityDisposition::Conflict {
            incumbent: incumbent.clone(),
        },
    })
}

/// Classify one deterministic Event-to-occurrence projection.
///
/// # Errors
/// Returns [`EventIdentityLookupError`] for invalid input, failed projection,
/// storage failure, or a response for another typed query.
pub fn classify_interaction_occurrence(
    port: &mut dyn EventCustodyPort,
    store: &AuthorityStoreCoordinate,
    occurrence: AdmittedEventRecord<'_, InteractionOccurrenceV1>,
    maximum_body_bytes: usize,
) -> Result<DerivedEventRecordDisposition, EventIdentityLookupError> {
    occurrence.record.verify()?;
    classify_derived(
        port,
        store,
        EventIndexQuery::InteractionOccurrence(occurrence_key(
            occurrence.record.event_reference_ref(),
            occurrence.record.event_reference_sha256(),
        )?),
        public_record(
            occurrence,
            occurrence.record.canonical_occurrence_sha256().as_str(),
        )?,
        maximum_body_bytes,
    )
}

/// Classify one deterministic truth-classed Genealogy edge.
///
/// # Errors
/// Returns [`EventIdentityLookupError`] for invalid input, failed projection,
/// storage failure, or a response for another typed query.
pub fn classify_genealogy_edge(
    port: &mut dyn EventCustodyPort,
    store: &AuthorityStoreCoordinate,
    edge: AdmittedEventRecord<'_, GenealogyEdgeV1>,
    maximum_body_bytes: usize,
) -> Result<DerivedEventRecordDisposition, EventIdentityLookupError> {
    edge.record.verify()?;
    classify_derived(
        port,
        store,
        EventIndexQuery::GenealogyEdge(genealogy_edge_key(edge.record)?),
        public_record(edge, edge.record.canonical_genealogy_edge_sha256().as_str())?,
        maximum_body_bytes,
    )
}

/// Classify one subject's reconciliation frontier without assuming succession.
///
/// # Errors
/// Returns [`EventIdentityLookupError`] for invalid input, failed projection,
/// storage failure, or a response for another typed query.
pub fn classify_reconciliation_frontier(
    port: &mut dyn EventCustodyPort,
    store: &AuthorityStoreCoordinate,
    frontier: AdmittedEventRecord<'_, ReconciliationFrontierV1>,
    maximum_body_bytes: usize,
) -> Result<ReconciliationFrontierDisposition, EventIdentityLookupError> {
    frontier.record.verify()?;
    let query =
        EventIndexQuery::ReconciliationFrontier(reconciliation_frontier_key(frontier.record)?);
    let expected = public_record(
        frontier,
        frontier
            .record
            .canonical_reconciliation_frontier_sha256()
            .as_str(),
    )?;
    let observation = port.read_index(store, &query, maximum_body_bytes)?;
    if observation.query() != &query {
        return Err(EventIdentityLookupError::MisdirectedObservation);
    }
    Ok(match observation.record() {
        None => ReconciliationFrontierDisposition::New,
        Some(incumbent) if same_canonical_record(incumbent, &expected) => {
            ReconciliationFrontierDisposition::ExactReplay {
                incumbent: incumbent.clone(),
            }
        }
        Some(incumbent) => ReconciliationFrontierDisposition::Current {
            incumbent: incumbent.clone(),
        },
    })
}

fn classify_derived(
    port: &mut dyn EventCustodyPort,
    store: &AuthorityStoreCoordinate,
    query: EventIndexQuery,
    expected: ControlRecord,
    maximum_body_bytes: usize,
) -> Result<DerivedEventRecordDisposition, EventIdentityLookupError> {
    let observation = port.read_index(store, &query, maximum_body_bytes)?;
    if observation.query() != &query {
        return Err(EventIdentityLookupError::MisdirectedObservation);
    }
    Ok(match observation.record() {
        None => DerivedEventRecordDisposition::New,
        Some(incumbent) if same_canonical_record(incumbent, &expected) => {
            DerivedEventRecordDisposition::ExactReplay {
                incumbent: incumbent.clone(),
            }
        }
        Some(incumbent) => DerivedEventRecordDisposition::Conflict {
            incumbent: incumbent.clone(),
        },
    })
}

fn same_canonical_record(left: &ControlRecord, right: &ControlRecord) -> bool {
    left.coordinate() == right.coordinate()
        && left.schema_id() == right.schema_id()
        && left.schema_sha256() == right.schema_sha256()
        && left.canonical_record_jcs() == right.canonical_record_jcs()
}

/// Failure of one bounded Event identity lookup.
#[derive(Debug, thiserror::Error)]
pub enum EventIdentityLookupError {
    /// Canonical key or record projection failed.
    #[error(transparent)]
    Projection(#[from] EventCustodyProjectionError),
    /// The custody port failed the point read.
    #[error(transparent)]
    Store(#[from] ControlStoreError),
    /// The port returned an observation for another typed query.
    #[error("Event custody returned an observation for another point query")]
    MisdirectedObservation,
    /// The admitted record no longer verifies.
    #[error(transparent)]
    Semantic(#[from] pale_core::records::InvalidResidentRecordV1),
}
