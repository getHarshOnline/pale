//! Bounded reads of immutable Event transaction members.

use super::*;
use crate::SequencedAuthorityEntry;
use pale_core::records::ControlTransactionSequenceV1;

/// One newly inserted member selected by its existing custody role and coordinate.
/// Previously retained dependencies are not transaction members.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventCustodyHistoryMember {
    kind: EventTransitionMemberKind,
    coordinate: ControlRecordCoordinate,
}

impl EventCustodyHistoryMember {
    /// Select an immutable member without copying its body into the request.
    #[must_use]
    pub const fn new(kind: EventTransitionMemberKind, coordinate: ControlRecordCoordinate) -> Self {
        Self { kind, coordinate }
    }

    /// Return the member's existing custody role.
    #[must_use]
    pub const fn kind(&self) -> EventTransitionMemberKind {
        self.kind
    }

    /// Borrow the exact immutable record reference and digest.
    #[must_use]
    pub const fn coordinate(&self) -> &ControlRecordCoordinate {
        &self.coordinate
    }
}

/// Read exact journal bytes and records inserted in one selected transaction.
///
/// This request does not select current indexes, grant publication permission,
/// or attest that the requested entry is admitted authority history. The
/// application must decode the journal delta and check that it binds the same
/// transaction and members. This read alone cannot establish that association.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventCustodyHistoryRead {
    store: AuthorityStoreCoordinate,
    transaction: ControlTransactionSequenceV1,
    entry: SequencedAuthorityEntry,
    kind: EventCustodyTransitionKind,
    members: Box<[EventCustodyHistoryMember]>,
    maximum_bytes: usize,
}

impl EventCustodyHistoryRead {
    /// Validate the closed member roles and a combined journal/record body limit.
    ///
    /// The limit covers raw journal bytes and all returned canonical record
    /// bodies. It does not bound database page reads, frame overhead or all
    /// allocations. Member count is fixed by the existing transition kind.
    ///
    /// # Errors
    ///
    /// Returns [`EventCustodyCandidateError`] for missing, repeated or excess
    /// roles, repeated coordinates, or a limit smaller than the journal body.
    pub fn new(
        store: AuthorityStoreCoordinate,
        transaction: ControlTransactionSequenceV1,
        entry: SequencedAuthorityEntry,
        kind: EventCustodyTransitionKind,
        members: Vec<EventCustodyHistoryMember>,
        maximum_bytes: usize,
    ) -> Result<Self, EventCustodyCandidateError> {
        validate_member_roles(kind, members.iter().map(EventCustodyHistoryMember::kind))?;
        if members
            .iter()
            .map(|member| &member.coordinate)
            .collect::<BTreeSet<_>>()
            .len()
            != members.len()
        {
            return Err(EventCustodyCandidateError::DuplicateRecordCoordinate);
        }
        if maximum_bytes < entry.bytes().as_slice().len() || maximum_bytes == 0 {
            return Err(EventCustodyCandidateError::HistoryByteLimit);
        }
        Ok(Self {
            store,
            transaction,
            entry,
            kind,
            members: members.into_boxed_slice(),
            maximum_bytes,
        })
    }

    /// Borrow the selected store incarnation.
    #[must_use]
    pub const fn store(&self) -> &AuthorityStoreCoordinate {
        &self.store
    }

    /// Return the original member insertion transaction.
    #[must_use]
    pub const fn transaction(&self) -> ControlTransactionSequenceV1 {
        self.transaction
    }

    /// Borrow the exact journal entry expected at its historical sequence.
    #[must_use]
    pub const fn entry(&self) -> &SequencedAuthorityEntry {
        &self.entry
    }

    /// Return the closed physical transition kind.
    #[must_use]
    pub const fn kind(&self) -> EventCustodyTransitionKind {
        self.kind
    }

    /// Borrow the exact newly inserted members in request order.
    #[must_use]
    pub const fn members(&self) -> &[EventCustodyHistoryMember] {
        &self.members
    }

    /// Return the combined raw journal and returned record body limit in bytes.
    #[must_use]
    pub const fn maximum_bytes(&self) -> usize {
        self.maximum_bytes
    }
}

/// Physical historical custody, without current-index or execution authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventCustodyHistoryObservation {
    /// A selected record exceeds the remaining combined byte budget.
    /// No record bodies or custody conclusion are returned.
    BoundExceeded,
    /// Neither entry nor member is retained, and both requested frontiers are ahead.
    Absent,
    /// The journal bytes match and all members have the selected insertion stamp.
    /// Members are returned in request order; semantic admission remains external.
    Exact {
        /// The requested immutable members, in request order.
        members: Box<[EventTransitionMember]>,
    },
    /// Some selected material is missing, differs, or belongs to another transaction.
    ConflictingOrPartial,
}
