//! Exact endpoint witnesses, incident relations and scope-local services from retained records.

use super::*;

/// Cumulative lookup evidence, independent of every returned record's effective state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrganizationEndpointLookup {
    /// No match yet; an unfinished scan cannot establish absence.
    SearchIncomplete,
    /// At least one record or binding witness was found, possibly on an earlier page.
    Found,
    /// The relevant family was exhausted without a match. Input selection is separate.
    NotFound,
    /// This endpoint is an external reference, not a locally owned record or readiness claim.
    ExternalReference,
}

pub(super) enum Query<'a> {
    All,
    Endpoint(&'a OrganizationalNodeRefV1),
    Incident(&'a OrganizationalNodeRefV1),
    IncidentServices(&'a LogicalScopeRefV1),
}

#[derive(Clone)]
pub(super) enum Selection {
    All,
    Endpoint(Arc<OrganizationalNodeRefV1>),
    Incident(Arc<OrganizationalNodeRefV1>),
    IncidentServices(Arc<LogicalScopeRefV1>),
}

impl Query<'_> {
    fn first_family(&self) -> OrganizationFamily {
        use OrganizationFamily as F;
        match self {
            Self::All => F::Scope,
            Self::Incident(_) => F::Relation,
            Self::IncidentServices(_) => F::Service,
            Self::Endpoint(node) => match node {
                OrganizationalNodeRefV1::Scope { .. } => F::Scope,
                OrganizationalNodeRefV1::Organization { .. } => F::Organization,
                OrganizationalNodeRefV1::Group { .. } => F::Group,
                OrganizationalNodeRefV1::DisciplineTree { .. } => F::DisciplineTree,
                OrganizationalNodeRefV1::Role { .. } => F::Role,
                OrganizationalNodeRefV1::Occupant { .. } => F::OccupantBinding,
                OrganizationalNodeRefV1::Provider { .. } => F::ProviderBinding,
                OrganizationalNodeRefV1::Object { .. } => F::Object,
                OrganizationalNodeRefV1::NativeResource { .. } => F::Done,
            },
        }
    }

    fn next_family(&self, family: OrganizationFamily) -> OrganizationFamily {
        match self {
            Self::All => family.next(),
            Self::Incident(_) if family == OrganizationFamily::Relation => {
                OrganizationFamily::DerivedRelation
            }
            Self::Endpoint(_) | Self::Incident(_) | Self::IncidentServices(_) => {
                OrganizationFamily::Done
            }
        }
    }

    fn retain(&self) -> Selection {
        match self {
            Self::All => Selection::All,
            Self::Endpoint(node) => Selection::Endpoint(Arc::new((*node).clone())),
            Self::Incident(node) => Selection::Incident(Arc::new((*node).clone())),
            Self::IncidentServices(scope) => {
                Selection::IncidentServices(Arc::new((*scope).clone()))
            }
        }
    }

    fn matches_cursor(&self, selection: &Selection) -> bool {
        match (self, selection) {
            (Self::All, Selection::All) => true,
            (Self::Endpoint(left), Selection::Endpoint(right))
            | (Self::Incident(left), Selection::Incident(right)) => *left == right.as_ref(),
            (Self::IncidentServices(left), Selection::IncidentServices(right)) => {
                *left == right.as_ref()
            }
            _ => false,
        }
    }

    fn includes(&self, record: OrganizationInspectionRecord<'_>) -> bool {
        use OrganizationInspectionRecord as R;
        use OrganizationalNodeRefV1 as N;
        match self {
            Self::All => true,
            Self::IncidentServices(scope) => matches!(record, R::Service(v)
                if &v.provider_scope == *scope || &v.consumer_scope == *scope),
            Self::Incident(node) => match record {
                R::Relation(v) => &v.from == *node || &v.to == *node,
                R::DerivedRelation(v) => &v.from == *node || &v.to == *node,
                _ => false,
            },
            Self::Endpoint(node) => match (node, record) {
                (N::Scope { scope }, R::Scope(v)) => scope == &v.scope,
                (N::Organization { organization }, R::Organization(v)) => {
                    organization == &v.organization
                }
                (N::Group { group }, R::Group(v)) => group == &v.group,
                (N::DisciplineTree { tree }, R::DisciplineTree(v)) => tree == &v.tree,
                (N::Role { role }, R::Role(v)) => role == &v.role,
                (N::Occupant { occupant }, R::OccupantBinding(v)) => occupant == &v.occupant,
                (N::Provider { provider }, R::ProviderBinding(v)) => provider == &v.provider,
                (N::Object { object }, R::Object(v)) => object == &v.object_id,
                _ => false,
            },
        }
    }

    fn lookup(&self, found: bool, complete: bool) -> Option<OrganizationEndpointLookup> {
        match self {
            Self::All | Self::Incident(_) | Self::IncidentServices(_) => None,
            Self::Endpoint(OrganizationalNodeRefV1::NativeResource { .. }) => {
                Some(OrganizationEndpointLookup::ExternalReference)
            }
            Self::Endpoint(_) if found => Some(OrganizationEndpointLookup::Found),
            Self::Endpoint(_) if complete => Some(OrganizationEndpointLookup::NotFound),
            Self::Endpoint(_) => Some(OrganizationEndpointLookup::SearchIncomplete),
        }
    }
}

impl PortableWorkspaceUniverse {
    /// Page directional services incident to an exact provider or consumer scope.
    ///
    /// Visits charge every canonical service, including nonmatches. Each matching
    /// contract appears once; parallel contracts retain their full keys, direction,
    /// requirements and state. Empty pages can continue and do not prove scope
    /// existence. No other record family or transitive neighborhood is scanned.
    ///
    /// # Errors
    /// Invalid limits or a different scope, query kind or owner reject before allocation.
    pub fn inspect_organization_incident_services(
        &self,
        scope: &LogicalScopeRefV1,
        after: Option<&OrganizationInspectionCursor>,
        maximum_inspected: u16,
        maximum_returned: u16,
    ) -> Result<OrganizationInspectionPage<'_>, OrganizationInspectionError> {
        self.query_organization(
            Query::IncidentServices(scope),
            after,
            maximum_inspected,
            maximum_returned,
        )
    }

    /// Find exact local endpoint records or all matching occupant/provider binding witnesses.
    ///
    /// Scans only the endpoint's existing canonical family. Every examined record,
    /// including a nonmatch, consumes one visit. Found is cumulative across pages;
    /// an empty incomplete page is not absence. External references visit no records.
    /// The independent selected-input flag distinguishes missing organization input.
    /// No effective state is filtered or promoted to native realization.
    ///
    /// # Errors
    /// Invalid raw limits or a different endpoint, query kind or owner reject before allocation.
    pub fn inspect_organization_endpoint(
        &self,
        endpoint: &OrganizationalNodeRefV1,
        after: Option<&OrganizationInspectionCursor>,
        maximum_inspected: u16,
        maximum_returned: u16,
    ) -> Result<OrganizationInspectionPage<'_>, OrganizationInspectionError> {
        self.query_organization(
            Query::Endpoint(endpoint),
            after,
            maximum_inspected,
            maximum_returned,
        )
    }

    /// Page explicit and already-derived relations incident to either exact endpoint.
    ///
    /// Each canonical record is examined once per journey; a self-loop is returned
    /// once and parallel relations remain independent. At most two typed endpoint
    /// comparisons occur per visit. Empty pages may continue. This does not search
    /// structural associations, expand transitive closure, or determine endpoint
    /// existence; use the separate endpoint query for record membership.
    ///
    /// # Errors
    /// Invalid raw limits or a different endpoint, query kind or owner reject before allocation.
    pub fn inspect_organization_incident_relations(
        &self,
        endpoint: &OrganizationalNodeRefV1,
        after: Option<&OrganizationInspectionCursor>,
        maximum_inspected: u16,
        maximum_returned: u16,
    ) -> Result<OrganizationInspectionPage<'_>, OrganizationInspectionError> {
        self.query_organization(
            Query::Incident(endpoint),
            after,
            maximum_inspected,
            maximum_returned,
        )
    }

    pub(super) fn query_organization(
        &self,
        query: Query<'_>,
        after: Option<&OrganizationInspectionCursor>,
        maximum_inspected: u16,
        maximum_returned: u16,
    ) -> Result<OrganizationInspectionPage<'_>, OrganizationInspectionError> {
        validate_limits(maximum_inspected, maximum_returned)?;
        if after.is_some_and(|cursor| {
            !Arc::ptr_eq(&cursor.owner, self.organization_owner())
                || cursor.inspected_limit != maximum_inspected
                || cursor.returned_limit != maximum_returned
                || !query.matches_cursor(&cursor.selection)
        }) {
            return Err(OrganizationInspectionError::ContinuationMismatch);
        }
        let mut position = after.map_or(
            OrganizationPosition {
                family: query.first_family(),
                index: 0,
            },
            |c| c.position,
        );
        let mut found_any = after.is_some_and(|c| c.found_any);
        let capacity = usize::from(maximum_inspected.min(maximum_returned));
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(capacity)
            .map_err(OrganizationInspectionError::Allocation)?;
        let mut inspected = 0;
        while position.family != OrganizationFamily::Done
            && inspected < maximum_inspected
            && entries.len() < capacity
        {
            if let Some(record) = self.organization_record_at(position)? {
                inspected += 1;
                if query.includes(record) {
                    found_any = true;
                    entries.push(OrganizationInspectionEntry {
                        owner: self,
                        position,
                        record,
                    });
                }
                position.index += 1;
            } else {
                position.family = query.next_family(position.family);
                position.index = 0;
            }
        }
        if position.family == OrganizationFamily::Done && position.index != 0 {
            return Err(OrganizationInspectionError::InvalidPosition);
        }
        let complete = position.family == OrganizationFamily::Done;
        let continuation = (!complete).then(|| OrganizationInspectionCursor {
            owner: Arc::clone(self.organization_owner()),
            position,
            inspected_limit: maximum_inspected,
            returned_limit: maximum_returned,
            selection: after.map_or_else(|| query.retain(), |c| c.selection.clone()),
            found_any,
        });
        Ok(OrganizationInspectionPage {
            owner: self,
            entries,
            inspected,
            continuation,
            endpoint_lookup: query.lookup(found_any, complete),
        })
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/workspace/organization_node_queries.rs"]
mod tests;
