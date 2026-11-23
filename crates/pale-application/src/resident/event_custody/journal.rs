//! Body-free Event custody retained by the resident journal.
//!
//! This record explains which immutable members shared a transaction. It does
//! not retain their bodies or grant permission to publish them again.

use pale_core::records::{
    ArtifactDigest, ControlTransactionSequenceV1, JournalSequenceV1, SchemaDigest, SchemaId,
};
use pale_ports::{
    AuthorityStoreCoordinate, ControlRecordCoordinate, ControlTransactionPreparation,
    EventCustodyHistoryMember, EventCustodyHistoryObservation, EventCustodyHistoryRead,
    EventCustodyPort, EventCustodyTransitionKind, EventIndexPredecessor, EventIndexQuery,
    SequencedAuthorityEntry,
};
use serde::{Deserialize, Serialize};

use super::projection::EventCustodyContent;

/// Only the latest custody transition is retained in the resident state image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "basis", rename_all = "snake_case")]
pub(in crate::resident) enum EventCustodyFrontier {
    /// No Event custody transition has been committed by this journal.
    Empty,
    /// Exact coordinates of the latest committed Event transaction.
    Retained(Box<EventCustodyBasis>),
}

impl EventCustodyFrontier {
    pub(in crate::resident) fn basis(&self) -> Option<&EventCustodyBasis> {
        match self {
            Self::Empty => None,
            Self::Retained(basis) => Some(basis),
        }
    }

    /// Both counters advance, but a transaction counter is not a journal sequence.
    pub(in crate::resident) fn accepts(&self, next: &EventCustodyBasis) -> bool {
        self.basis().is_none_or(|prior| {
            prior.sequence < next.sequence && prior.transaction < next.transaction
        })
    }
}

/// Exact private journal basis for one jointly committed Event transition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::resident) struct EventCustodyBasis {
    sequence: JournalSequenceV1,
    transaction: ControlTransactionSequenceV1,
    kind: EventCustodyTransitionKind,
    members: Vec<RetainedEventMember>,
    mutations: Vec<IndexMutationBasis>,
    assertions: Vec<IndexAssertionBasis>,
}

/// The role and semantic coordinate stay distinct from the exact stored bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RetainedEventMember {
    member: EventCustodyHistoryMember,
    schema_id: SchemaId,
    schema_sha256: SchemaDigest,
    body_sha256: ArtifactDigest,
}

/// An index comparison retains its coordinate, never its record body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "coordinate", rename_all = "snake_case")]
enum IndexPredecessorBasis {
    Absent,
    Exact(ControlRecordCoordinate),
}

impl From<&EventIndexPredecessor> for IndexPredecessorBasis {
    fn from(value: &EventIndexPredecessor) -> Self {
        match value {
            EventIndexPredecessor::Absent => Self::Absent,
            EventIndexPredecessor::Exact(record) => Self::Exact(record.coordinate().clone()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct IndexMutationBasis {
    query: EventIndexQuery,
    expected: IndexPredecessorBasis,
    successor: ControlRecordCoordinate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct IndexAssertionBasis {
    query: EventIndexQuery,
    expected: IndexPredecessorBasis,
}

impl EventCustodyBasis {
    /// Project the same content later consumed by the full candidate validator.
    /// No journal bytes or database records are written here.
    pub(in crate::resident) fn from_content(
        sequence: JournalSequenceV1,
        transaction: &ControlTransactionPreparation,
        content: &EventCustodyContent,
    ) -> Self {
        Self {
            sequence,
            transaction: transaction.successor(),
            kind: content.kind,
            members: content
                .members
                .iter()
                .map(|member| RetainedEventMember {
                    member: EventCustodyHistoryMember::new(
                        member.kind(),
                        member.record().coordinate().clone(),
                    ),
                    schema_id: member.record().schema_id().clone(),
                    schema_sha256: member.record().schema_sha256().clone(),
                    body_sha256: ArtifactDigest::hash_bytes(
                        member.record().canonical_record_jcs().as_slice(),
                    ),
                })
                .collect(),
            mutations: content
                .mutations
                .iter()
                .map(|mutation| IndexMutationBasis {
                    query: mutation.query().clone(),
                    expected: mutation.expected().into(),
                    successor: mutation.successor().clone(),
                })
                .collect(),
            assertions: content
                .assertions
                .iter()
                .map(|assertion| IndexAssertionBasis {
                    query: assertion.query().clone(),
                    expected: assertion.expected().into(),
                })
                .collect(),
        }
    }

    pub(in crate::resident) const fn sequence(&self) -> JournalSequenceV1 {
        self.sequence
    }

    /// Confirm that this exact transaction retained every selected member.
    pub(in crate::resident) fn contains_members(
        &self,
        required: &[EventCustodyHistoryMember],
    ) -> bool {
        required.iter().all(|required| {
            self.members
                .iter()
                .any(|retained| &retained.member == required)
        })
    }

    /// Check immutable insertion custody after the caller matches this basis to
    /// the decoded journal entry. Historical indexes need not still be current.
    pub(in crate::resident) fn verify_members(
        &self,
        store: &AuthorityStoreCoordinate,
        entry: &SequencedAuthorityEntry,
        port: &mut dyn EventCustodyPort,
        maximum_bytes: usize,
    ) -> Result<(), EventJournalCustodyError> {
        if entry.sequence() != self.sequence {
            return Err(EventJournalCustodyError::Correspondence);
        }
        let request = EventCustodyHistoryRead::new(
            store.clone(),
            self.transaction,
            entry.clone(),
            self.kind,
            self.members
                .iter()
                .map(|member| member.member.clone())
                .collect(),
            maximum_bytes,
        )?;
        match port.read_committed_members(&request)? {
            EventCustodyHistoryObservation::Exact { members } => {
                let mut bytes = entry.bytes().as_slice().len();
                if members.len() != self.members.len() {
                    return Err(EventJournalCustodyError::Correspondence);
                }
                for (observed, expected) in members.iter().zip(&self.members) {
                    let record = observed.record();
                    bytes = bytes
                        .checked_add(record.canonical_record_jcs().as_slice().len())
                        .filter(|bytes| *bytes <= maximum_bytes)
                        .ok_or(EventJournalCustodyError::BoundExceeded)?;
                    if observed.kind() != expected.member.kind()
                        || record.coordinate() != expected.member.coordinate()
                        || record.schema_id() != &expected.schema_id
                        || record.schema_sha256() != &expected.schema_sha256
                        || ArtifactDigest::hash_bytes(record.canonical_record_jcs().as_slice())
                            != expected.body_sha256
                    {
                        return Err(EventJournalCustodyError::Correspondence);
                    }
                }
                Ok(())
            }
            EventCustodyHistoryObservation::BoundExceeded => {
                Err(EventJournalCustodyError::BoundExceeded)
            }
            EventCustodyHistoryObservation::Absent
            | EventCustodyHistoryObservation::ConflictingOrPartial => {
                Err(EventJournalCustodyError::Correspondence)
            }
        }
    }
}

/// Custody could not establish the selected journal/member correspondence.
#[derive(Debug, thiserror::Error)]
pub(crate) enum EventJournalCustodyError {
    #[error("Event historical custody exceeds the selected byte limit")]
    BoundExceeded,
    #[error("Event journal and retained transaction members do not correspond")]
    Correspondence,
    #[error("Event journal member selection is invalid: {0}")]
    Candidate(#[from] pale_ports::EventCustodyCandidateError),
    #[error("Event historical custody could not be read: {0}")]
    Store(#[from] pale_ports::ControlStoreError),
}
