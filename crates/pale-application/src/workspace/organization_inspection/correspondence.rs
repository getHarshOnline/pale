//! Compare one exact record with the same family in another retained compilation.

use super::*;
use crate::composition::{organization::record_identity as key, responsibility};
use crate::workspace::PortableWorkspaceValueComparison;

/// Continuation of one selected record's search in one other compilation.
///
/// These shared semantic owners are not source maps or transferable authority.
/// Resume through the same complete compilations and selected record.
#[derive(Clone)]
pub struct OrganizationCorrespondenceCursor {
    selected_owner: Arc<PrincipalAdmittedUniverse>,
    other_owner: Arc<PrincipalAdmittedUniverse>,
    selected: OrganizationPosition,
    next_index: usize,
    maximum_inspected: u16,
}

impl std::fmt::Debug for OrganizationCorrespondenceCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrganizationCorrespondenceCursor")
            .finish_non_exhaustive()
    }
}

/// Identity correspondence is separate from source occurrence and value equality.
pub enum OrganizationCorrespondenceState<'a> {
    /// The visit budget ended before a match or complete family exhaustion.
    Searching(OrganizationCorrespondenceCursor),
    /// The entire corresponding family contains no record with the selected key.
    Unmatched,
    /// One canonical key matched. The original complete values were compared.
    Matched {
        /// Exact record in the other compilation, with that compilation's sources.
        other: OrganizationInspectionEntry<'a>,
        /// Equal semantic values need not have equal source occurrences.
        values: PortableWorkspaceValueComparison,
    },
}

/// Borrowed result of one bounded family scan. No new correspondence ID is minted.
pub struct OrganizationCorrespondencePage<'a> {
    selected: OrganizationInspectionEntry<'a>,
    state: OrganizationCorrespondenceState<'a>,
    inspected: u16,
}

impl<'a> OrganizationCorrespondencePage<'a> {
    /// The original selected record, with its own source custody.
    #[must_use]
    pub const fn selected(&self) -> &OrganizationInspectionEntry<'a> {
        &self.selected
    }
    /// Search progress or the complete comparison result.
    #[must_use]
    pub const fn state(&self) -> &OrganizationCorrespondenceState<'a> {
        &self.state
    }
    /// Candidate keys examined, including every nonmatch.
    #[must_use]
    pub const fn inspected(&self) -> u16 {
        self.inspected
    }
}

impl PortableWorkspaceUniverse {
    /// Find the selected record's canonical key in this other compilation.
    ///
    /// Visits are bounded by `maximum_inspected` (1..=256). Each visit compares
    /// bounded key fields. At most one matching complete record is compared by
    /// its existing typed equality, including nested collections. That comparison
    /// is bounded by admitted record limits, not by the visit count. No record
    /// body is cloned or serialized; the existing service-key getter may copy its
    /// bounded identifier fields. No elapsed-time or total-memory bound is claimed.
    ///
    /// Source order, diagnostics, runtime state and layout are not record values.
    /// This primitive establishes structural key correspondence only. The SDK
    /// calls it through an existing verified comparison pair; it does not itself
    /// construct a comparison receipt or establish compatibility between owners.
    /// Unmatched requires family exhaustion; hitting the visit bound can produce
    /// an empty terminal successor. Inspection uses its configured singleton ID.
    /// Derived relation keys retain source, class, both endpoints and interval.
    ///
    /// # Errors
    /// Wrong custody, changed limits and invalid positions reject before scanning.
    pub fn correspond_organization_record<'a>(
        &'a self,
        selected_owner: &'a Self,
        selected: &OrganizationRecordHandle,
        after: Option<&OrganizationCorrespondenceCursor>,
        maximum_inspected: u16,
    ) -> Result<OrganizationCorrespondencePage<'a>, OrganizationInspectionError> {
        validate_limits(maximum_inspected, 1)?;
        let selected_entry = selected_owner.resolve_organization_record(selected)?;
        if after.is_some_and(|cursor| {
            !Arc::ptr_eq(&cursor.selected_owner, selected_owner.organization_owner())
                || !Arc::ptr_eq(&cursor.other_owner, self.organization_owner())
                || cursor.selected != selected.position
                || cursor.maximum_inspected != maximum_inspected
        }) {
            return Err(OrganizationInspectionError::ContinuationMismatch);
        }
        let mut position = OrganizationPosition {
            family: selected.position.family,
            index: after.map_or(0, |cursor| cursor.next_index),
        };
        let mut inspected = 0;
        while inspected < maximum_inspected {
            let Some(record) = self.organization_record_at(position)? else {
                return Ok(OrganizationCorrespondencePage {
                    selected: selected_entry,
                    state: OrganizationCorrespondenceState::Unmatched,
                    inspected,
                });
            };
            inspected += 1;
            if same_identity(selected_entry.record, record) {
                let values = if selected_entry.record == record {
                    PortableWorkspaceValueComparison::Equal
                } else {
                    PortableWorkspaceValueComparison::Changed
                };
                return Ok(OrganizationCorrespondencePage {
                    selected: selected_entry,
                    state: OrganizationCorrespondenceState::Matched {
                        other: OrganizationInspectionEntry {
                            owner: self,
                            position,
                            record,
                        },
                        values,
                    },
                    inspected,
                });
            }
            position.index = position
                .index
                .checked_add(1)
                .ok_or(OrganizationInspectionError::InvalidPosition)?;
        }
        Ok(OrganizationCorrespondencePage {
            selected: selected_entry,
            state: OrganizationCorrespondenceState::Searching(OrganizationCorrespondenceCursor {
                selected_owner: Arc::clone(selected_owner.organization_owner()),
                other_owner: Arc::clone(self.organization_owner()),
                selected: selected.position,
                next_index: position.index,
                maximum_inspected,
            }),
            inspected,
        })
    }
}

fn same_identity(
    left: OrganizationInspectionRecord<'_>,
    right: OrganizationInspectionRecord<'_>,
) -> bool {
    use OrganizationInspectionRecord as R;
    match (left, right) {
        (R::Scope(a), R::Scope(b)) => key::scope(a) == key::scope(b),
        (R::Organization(a), R::Organization(b)) => key::organization(a) == key::organization(b),
        (R::Group(a), R::Group(b)) => key::group(a) == key::group(b),
        (R::DisciplineTree(a), R::DisciplineTree(b)) => key::tree(a) == key::tree(b),
        (R::Role(a), R::Role(b)) => key::role(a) == key::role(b),
        (R::OccupantBinding(a), R::OccupantBinding(b)) => key::occupant(a) == key::occupant(b),
        (R::ProviderBinding(a), R::ProviderBinding(b)) => key::provider(a) == key::provider(b),
        (R::CommunicationActor(a), R::CommunicationActor(b)) => {
            key::communication(a) == key::communication(b)
        }
        (R::Leadership(a), R::Leadership(b)) => key::leadership(a) == key::leadership(b),
        (R::Relation(a), R::Relation(b)) => key::relation(a) == key::relation(b),
        (R::Object(a), R::Object(b)) => key::object(a) == key::object(b),
        (R::PopulationRule(a), R::PopulationRule(b)) => key::population(a) == key::population(b),
        (R::GroupLens(a), R::GroupLens(b)) => key::lens(a) == key::lens(b),
        (R::Service(a), R::Service(b)) => a.contract_ref() == b.contract_ref(),
        (R::NativeProjection(a), R::NativeProjection(b)) => {
            key::projection(a) == key::projection(b)
        }
        (R::Inspection(a), R::Inspection(b)) => key::inspection(a) == key::inspection(b),
        (R::DerivedRelation(a), R::DerivedRelation(b)) => key::derived(a) == key::derived(b),
        (R::Responsibility(a), R::Responsibility(b)) => {
            responsibility::record_identity(a) == responsibility::record_identity(b)
        }
        _ => false,
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/workspace/organization_correspondence.rs"]
mod tests;
