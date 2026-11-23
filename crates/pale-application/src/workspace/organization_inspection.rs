//! Bounded borrowed records from one admitted organizational compilation.

use pale_core::records::*;
use std::sync::Arc;

use super::universe::PrincipalAdmittedUniverse;
use super::{AuthoringOccurrencePage, AuthoringOccurrencePageCursor, PortableWorkspaceUniverse};

mod correspondence;
mod details;
mod node_queries;
pub use correspondence::{
    OrganizationCorrespondenceCursor, OrganizationCorrespondencePage,
    OrganizationCorrespondenceState,
};
pub use details::{
    OrganizationCommunicationResolutionHandle, OrganizationGenealogyHandle,
    OrganizationMemberCursor, OrganizationMemberEntry, OrganizationMemberPage,
    OrganizationResponsibilityProjectionHandle, OrganizationServiceAssignmentHandle,
};
pub use node_queries::OrganizationEndpointLookup;

/// One existing semantic record, never a raw IR or a geometry object.
///
/// Nested collections remain borrowed. Inspecting this view does not expand
/// their members, establish recipient access or prove native realization.
/// Each variant keeps the identity, state and independent axes of its owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrganizationInspectionRecord<'a> {
    /// A namespace-qualified logical scope.
    Scope(&'a LogicalScopeV1),
    /// A namespace-qualified organization and its explicit scope.
    Organization(&'a LogicalOrganizationV1),
    /// A group and its explicit organization.
    Group(&'a OrganizationalGroupV1),
    /// A discipline tree and its explicit organization.
    DisciplineTree(&'a DisciplineTreeV1),
    /// Stable role, independent of its occupants and providers.
    Role(&'a OrganizationalRoleV1),
    /// Exact role/occupant/interval binding.
    OccupantBinding(&'a OrganizationalOccupantBindingV1),
    /// Exact role/provider/interval binding.
    ProviderBinding(&'a OrganizationalProviderBindingV1),
    /// Identity-only communication binding and separate local resolutions.
    CommunicationActor(&'a CommunicationActorBindingV1),
    /// Topic-scoped participation; not ambient authority.
    Leadership(&'a LeadershipParticipationV1),
    /// Explicit relation with exact identity, class and endpoints.
    Relation(&'a OrganizationalRelationV1),
    /// Owner-selected object with its actual scope and organization.
    Object(&'a OrganizationalObjectV1),
    /// Class-keyed population rule, not inferred membership.
    PopulationRule(&'a ObjectRolePopulationRuleV1),
    /// Exact configured rights; inspection does not admit a holder.
    GroupLens(&'a GroupLensAuthorityProfileV1),
    /// Full directional contract with independent acceptance frontiers.
    Service(&'a DirectionalServiceContractV1),
    /// Desired native references, not native receipts.
    NativeProjection(&'a OrganizationalNativeProjectionV1),
    /// Configured inspection selection, not recipient proof.
    Inspection(&'a OrganizationalInspectionV1),
    /// Existing derived edge. Identity includes source, class, both endpoints
    /// and interval: a leadership topic alone identifies several edges.
    DerivedRelation(&'a OrganizationalDerivedRelationV1),
    /// Attributed responsibility answers. Supplied service placement already
    /// passed the owning linker's exact directional-service closure check.
    Responsibility(&'a ResponsibilityDerivationV1),
}

/// Enumeration phase only; semantic identity stays with each borrowed record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum OrganizationFamily {
    Scope,
    Organization,
    Group,
    DisciplineTree,
    Role,
    OccupantBinding,
    ProviderBinding,
    CommunicationActor,
    Leadership,
    Relation,
    Object,
    PopulationRule,
    GroupLens,
    Service,
    NativeProjection,
    Inspection,
    DerivedRelation,
    Responsibility,
    Done,
}

impl OrganizationFamily {
    const fn next(self) -> Self {
        match self {
            Self::Scope => Self::Organization,
            Self::Organization => Self::Group,
            Self::Group => Self::DisciplineTree,
            Self::DisciplineTree => Self::Role,
            Self::Role => Self::OccupantBinding,
            Self::OccupantBinding => Self::ProviderBinding,
            Self::ProviderBinding => Self::CommunicationActor,
            Self::CommunicationActor => Self::Leadership,
            Self::Leadership => Self::Relation,
            Self::Relation => Self::Object,
            Self::Object => Self::PopulationRule,
            Self::PopulationRule => Self::GroupLens,
            Self::GroupLens => Self::Service,
            Self::Service => Self::NativeProjection,
            Self::NativeProjection => Self::Inspection,
            Self::Inspection => Self::DerivedRelation,
            Self::DerivedRelation => Self::Responsibility,
            Self::Responsibility | Self::Done => Self::Done,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct OrganizationPosition {
    family: OrganizationFamily,
    index: usize,
}

/// Exact producing compilation and raw limits, with no transferable authority.
///
/// This token retains the private admitted semantic owner, not the outer source
/// maps. Resuming requires the same complete compilation, which may have moved.
/// There is no public constructor, parser or serializer. Cloning shares the
/// owner; dropping the last retained token releases its shared ownership.
#[derive(Clone)]
pub struct OrganizationInspectionCursor {
    owner: Arc<PrincipalAdmittedUniverse>,
    position: OrganizationPosition,
    inspected_limit: u16,
    returned_limit: u16,
    selection: node_queries::Selection,
    found_any: bool,
}

impl std::fmt::Debug for OrganizationInspectionCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrganizationInspectionCursor")
            .finish_non_exhaustive()
    }
}

/// One record tied to its exact immutable input and source-index custody.
#[derive(Clone, Copy)]
pub struct OrganizationInspectionEntry<'a> {
    owner: &'a PortableWorkspaceUniverse,
    position: OrganizationPosition,
    record: OrganizationInspectionRecord<'a>,
}

impl<'a> OrganizationInspectionEntry<'a> {
    /// Retain this record's opaque position without copying any record body.
    ///
    /// The handle shares the admitted semantic owner. Resolving it still needs
    /// the exact complete compilation and its source maps; equal digests do not
    /// suffice. It grants no source disclosure or effect permission.
    #[must_use]
    pub fn retain(&self) -> OrganizationRecordHandle {
        OrganizationRecordHandle {
            owner: Arc::clone(self.owner.organization_owner()),
            position: self.position,
        }
    }

    /// Borrow the existing individual semantic record, without cloning bodies.
    #[must_use]
    pub const fn record(&self) -> OrganizationInspectionRecord<'a> {
        self.record
    }

    /// Query existing named source evidence separately from record enumeration.
    ///
    /// Missing mapping means this complete record key has no named identity in
    /// the current authoring index, not that its source is absent. Bindings and
    /// derived edges are not rematched through a role or topic. This uses the
    /// author's original array positions, not canonical record positions.
    /// Source lookup retains the existing identity construction, ordered-index
    /// comparisons and bounded scan/cursor allocation costs; the record-page
    /// cell bound does not cover this separate operation or encoded bytes.
    ///
    /// # Errors
    /// Rejects raw limits or another entry's/compilation's continuation before
    /// identity construction and scanning.
    pub fn sources(
        &self,
        after: Option<&OrganizationSourceCursor>,
        maximum_inspected: u16,
        maximum_returned: u16,
    ) -> Result<OrganizationSources<'a>, OrganizationInspectionError> {
        validate_limits(maximum_inspected, maximum_returned)?;
        if after.is_some_and(|cursor| {
            !Arc::ptr_eq(&cursor.owner, self.owner.organization_owner())
                || cursor.position != self.position
                || cursor.inspected_limit != maximum_inspected
                || cursor.returned_limit != maximum_returned
        }) {
            return Err(OrganizationInspectionError::ContinuationMismatch);
        }
        let Some(identity) = super::authoring::organization_record_identity(self.record) else {
            return Ok(OrganizationSources::UnmappedRecord);
        };
        let page = self.owner.authoring_semantics().occurrences_page(
            &identity,
            after.map(|cursor| &cursor.after),
            usize::from(maximum_inspected),
            usize::from(maximum_returned),
        );
        let continuation = page
            .continuation()
            .cloned()
            .map(|after| OrganizationSourceCursor {
                owner: Arc::clone(self.owner.organization_owner()),
                position: self.position,
                inspected_limit: maximum_inspected,
                returned_limit: maximum_returned,
                after,
            });
        Ok(OrganizationSources::Named { page, continuation })
    }
}

/// Retained exact record position, not a serializable semantic identity.
///
/// Only an actual borrowed entry can produce this handle. Resolving it visits
/// the existing record selector once; it does not scan earlier pages. It retains
/// the admitted semantic owner, while a caller wanting later source access must
/// also retain the complete [`PortableWorkspaceUniverse`].
#[derive(Clone)]
pub struct OrganizationRecordHandle {
    owner: Arc<PrincipalAdmittedUniverse>,
    position: OrganizationPosition,
}

impl OrganizationRecordHandle {
    /// Whether both handles select the same record in the same admitted owner.
    ///
    /// Independent pages may issue handles for the same position. Equal record
    /// values, keys or digests from another compilation do not satisfy this
    /// predicate. It does not compare records across generations or grant access.
    #[must_use]
    pub fn same_record(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && self.position == other.position
    }
}

impl std::fmt::Debug for OrganizationRecordHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrganizationRecordHandle")
            .finish_non_exhaustive()
    }
}

/// Distinguish a missing direct mapping from a mapped, possibly empty page.
pub enum OrganizationSources<'a> {
    /// This complete record identity has no direct named mapping. Source may exist.
    UnmappedRecord,
    /// Original named occurrences and compiler roles; an empty page may continue.
    Named {
        /// Borrowed owning occurrence page.
        page: AuthoringOccurrencePage<'a>,
        /// Exact record, owner and raw-limit continuation.
        continuation: Option<OrganizationSourceCursor>,
    },
}

/// Source position bound to the complete producing record, not a display name.
///
/// Retains the private semantic owner and the existing occurrence cursor, not
/// the outer source maps. Resume through the same complete compilation/record.
/// The token has no public constructor, parser or serializer.
#[derive(Clone)]
pub struct OrganizationSourceCursor {
    owner: Arc<PrincipalAdmittedUniverse>,
    position: OrganizationPosition,
    inspected_limit: u16,
    returned_limit: u16,
    after: AuthoringOccurrencePageCursor,
}

impl std::fmt::Debug for OrganizationSourceCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrganizationSourceCursor")
            .finish_non_exhaustive()
    }
}

/// One allocation-bounded page over canonical individual records.
pub struct OrganizationInspectionPage<'a> {
    owner: &'a PortableWorkspaceUniverse,
    entries: Vec<OrganizationInspectionEntry<'a>>,
    inspected: u16,
    continuation: Option<OrganizationInspectionCursor>,
    endpoint_lookup: Option<OrganizationEndpointLookup>,
}

impl<'a> OrganizationInspectionPage<'a> {
    /// Exact source owner; its existing identity getters distinguish all inputs.
    #[must_use]
    pub const fn compilation(&self) -> &'a PortableWorkspaceUniverse {
        self.owner
    }
    /// Whether organization input was selected; distinct from a selected empty record.
    #[must_use]
    pub fn organization_selected(&self) -> bool {
        self.owner.organization_selected()
    }
    /// Canonical borrowed records returned by this page.
    #[must_use]
    pub fn entries(&self) -> &[OrganizationInspectionEntry<'a>] {
        &self.entries
    }
    /// Source records visited, excluding constant-time empty-family boundary checks.
    #[must_use]
    pub const fn inspected(&self) -> u16 {
        self.inspected
    }
    /// Cumulative endpoint search result, only for endpoint-record queries.
    /// This describes record membership, not the record's effective/native state.
    #[must_use]
    pub const fn endpoint_lookup(&self) -> Option<OrganizationEndpointLookup> {
        self.endpoint_lookup
    }
    /// Next exact position; none only once the selected query is exhausted.
    #[must_use]
    pub const fn continuation(&self) -> Option<&OrganizationInspectionCursor> {
        self.continuation.as_ref()
    }
}

/// Query input, allocation and immutable-owner failures remain distinct.
#[derive(Debug, thiserror::Error)]
pub enum OrganizationInspectionError {
    /// The selected record does not own the requested structural section.
    #[error("organization record does not own the requested detail section")]
    SectionUnavailable,
    /// Both explicit limits must lie in the inclusive 1..=256 range.
    #[error("organization inspection limits must be between 1 and 256")]
    InvalidLimit,
    /// A valid opaque cursor belongs to another query or exact input.
    #[error("organization inspection continuation does not match its owner and limits")]
    ContinuationMismatch,
    /// A retained record belongs to another immutable compilation.
    #[error("organization record handle does not match its compilation")]
    RecordOwnerMismatch,
    /// Private cursor position disagrees with immutable admitted record custody.
    #[error("organization inspection position violates its retained owner invariant")]
    InvalidPosition,
    /// Bounded borrowed-row allocation could not be reserved.
    #[error("organization inspection page allocation failed: {0}")]
    Allocation(#[source] std::collections::TryReserveError),
}

fn validate_limits(inspected: u16, returned: u16) -> Result<(), OrganizationInspectionError> {
    if !(1..=256).contains(&inspected) || !(1..=256).contains(&returned) {
        return Err(OrganizationInspectionError::InvalidLimit);
    }
    Ok(())
}

impl PortableWorkspaceUniverse {
    /// Resolve a retained record through its existing single-record owner.
    ///
    /// The compilation may have moved, but must be the one that issued the
    /// handle. This checks custody before visiting one record and returns a
    /// borrowed body without allocation or re-enumerating preceding records.
    /// Source queries remain a separate bounded operation on the returned entry.
    ///
    /// # Errors
    /// A different owner is rejected before selection. An impossible retained
    /// position is an internal error, not a missing semantic record.
    pub fn resolve_organization_record(
        &self,
        handle: &OrganizationRecordHandle,
    ) -> Result<OrganizationInspectionEntry<'_>, OrganizationInspectionError> {
        if !Arc::ptr_eq(&handle.owner, self.organization_owner()) {
            return Err(OrganizationInspectionError::RecordOwnerMismatch);
        }
        let record = self
            .organization_record_at(handle.position)?
            .ok_or(OrganizationInspectionError::InvalidPosition)?;
        Ok(OrganizationInspectionEntry {
            owner: self,
            position: handle.position,
            record,
        })
    }

    /// Enumerate organizational and responsibility records from admitted input.
    ///
    /// Fixed family order follows [`OrganizationInspectionRecord`]'s variants;
    /// each family preserves its existing canonical order. This never derives
    /// relations or resolves services again. A service reference can name a
    /// record on another page without implying that page is complete.
    ///
    /// Both limits are explicit 1..=256 values, bound before allocation. A page
    /// visits at most the inspected limit and requests reservation for the
    /// smaller limit in borrowed cells. Nested values, retained source/IR, allocator
    /// overhead/spare capacity, source lookup and future encoding are separate
    /// costs. No aggregate nested-work or encoded-byte bound is claimed.
    /// Reaching a limit does not perform an uncharged exhaustion probe: the
    /// final nonempty page can have a zero-work terminal successor.
    ///
    /// This proves supplied-input admission only. It neither admits recipients
    /// nor filters disclosure; it must not be exposed as an authenticated
    /// delivery API without the actual owning contextual access checks.
    ///
    /// # Errors
    /// Raw limit/custody errors precede allocation; allocation and private
    /// position errors retain distinct meanings. Optional absent/empty input
    /// and explicit negative semantic states are successful observations.
    pub fn inspect_organization<'a>(
        &'a self,
        after: Option<&OrganizationInspectionCursor>,
        maximum_inspected: u16,
        maximum_returned: u16,
    ) -> Result<OrganizationInspectionPage<'a>, OrganizationInspectionError> {
        self.query_organization(
            node_queries::Query::All,
            after,
            maximum_inspected,
            maximum_returned,
        )
    }
}

/// The private admission owner invokes this selector; no IR getter crosses out.
pub(super) fn record_at<'a>(
    organization: Option<&'a OrganizationalTranslationIrV1>,
    responsibilities: &'a ResponsibilityDerivationIrV1,
    position: OrganizationPosition,
) -> Result<Option<OrganizationInspectionRecord<'a>>, OrganizationInspectionError> {
    use OrganizationFamily as Family;
    use OrganizationInspectionRecord as Row;
    fn at<T>(items: Option<&[T]>, index: usize) -> Result<Option<&T>, OrganizationInspectionError> {
        let items = items.unwrap_or(&[]);
        if index > items.len() {
            return Err(OrganizationInspectionError::InvalidPosition);
        }
        Ok(items.get(index))
    }
    let index = position.index;
    let source = organization.map(|organization| &organization.source);
    Ok(match position.family {
        Family::Scope => at(source.map(|s| s.scopes.as_slice()), index)?.map(Row::Scope),
        Family::Organization => {
            at(source.map(|s| s.organizations.as_slice()), index)?.map(Row::Organization)
        }
        Family::Group => at(source.map(|s| s.groups.as_slice()), index)?.map(Row::Group),
        Family::DisciplineTree => {
            at(source.map(|s| s.discipline_trees.as_slice()), index)?.map(Row::DisciplineTree)
        }
        Family::Role => at(source.map(|s| s.roles.as_slice()), index)?.map(Row::Role),
        Family::OccupantBinding => {
            at(source.map(|s| s.occupant_bindings.as_slice()), index)?.map(Row::OccupantBinding)
        }
        Family::ProviderBinding => {
            at(source.map(|s| s.provider_bindings.as_slice()), index)?.map(Row::ProviderBinding)
        }
        Family::CommunicationActor => at(
            source.map(|s| s.communication_actor_bindings.as_slice()),
            index,
        )?
        .map(Row::CommunicationActor),
        Family::Leadership => {
            at(source.map(|s| s.leadership.as_slice()), index)?.map(Row::Leadership)
        }
        Family::Relation => at(source.map(|s| s.relations.as_slice()), index)?.map(Row::Relation),
        Family::Object => at(source.map(|s| s.objects.as_slice()), index)?.map(Row::Object),
        Family::PopulationRule => {
            at(source.map(|s| s.population_rules.as_slice()), index)?.map(Row::PopulationRule)
        }
        Family::GroupLens => {
            at(source.map(|s| s.group_lenses.as_slice()), index)?.map(Row::GroupLens)
        }
        Family::Service => at(source.map(|s| s.services.as_slice()), index)?.map(Row::Service),
        Family::NativeProjection => {
            at(source.map(|s| s.native_projections.as_slice()), index)?.map(Row::NativeProjection)
        }
        Family::Inspection => {
            at(source.map(|s| s.inspection.as_slice()), index)?.map(Row::Inspection)
        }
        Family::DerivedRelation => at(organization.map(|o| o.derived_relations.as_slice()), index)?
            .map(Row::DerivedRelation),
        Family::Responsibility => {
            at(Some(&responsibilities.derivations), index)?.map(Row::Responsibility)
        }
        Family::Done => return Err(OrganizationInspectionError::InvalidPosition),
    })
}

#[cfg(test)]
#[path = "../../tests/internal/workspace/organization_inspection.rs"]
mod tests;
