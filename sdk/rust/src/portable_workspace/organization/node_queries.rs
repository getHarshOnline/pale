//! Exact endpoint, incident-relation and scope-local service lookup through the application owner.

use super::*;
use pale_core::records as core;

/// Record-membership evidence, independent of effective state and native realization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrganizationEndpointLookup {
    /// No match yet and the search has not ended.
    SearchIncomplete,
    /// A record or binding witness was found on this or an earlier page.
    Found,
    /// The complete relevant scan found no local record. Input selection is separate.
    NotFound,
    /// An external resource reference, not a local record or readiness claim.
    ExternalReference,
}

impl OrganizationEndpointLookup {
    pub(super) fn from_application(
        value: pale_application::workspace::OrganizationEndpointLookup,
    ) -> Self {
        use pale_application::workspace::OrganizationEndpointLookup as V;
        match value {
            V::SearchIncomplete => Self::SearchIncomplete,
            V::Found => Self::Found,
            V::NotFound => Self::NotFound,
            V::ExternalReference => Self::ExternalReference,
        }
    }
}

impl PortableWorkspaceCompilation {
    /// Page directional services at either end of an exact qualified scope.
    ///
    /// The application scans only canonical services and preserves full contract
    /// identity, direction, independent requirements and source/detail custody.
    /// Empty or incomplete pages do not establish scope absence. This is not
    /// recipient-qualified delivery or native-provider discovery. Bounds and copy
    /// accounting match [`Self::inspect_organization_endpoint`].
    ///
    /// # Errors
    /// Invalid scope identifiers, limits or cursor custody reject. Copy exhaustion
    /// returns no page and leaves the original continuation usable.
    pub fn inspect_organization_incident_services(
        &self,
        scope: &OrganizationScopeRef,
        after: Option<&OrganizationCursor>,
        maximum_inspected: u16,
        maximum_returned: u16,
        maximum_copied_bytes: u32,
    ) -> Result<OrganizationPage, ApiError> {
        let execute = || -> Result<OrganizationPage, OrganizationFailure> {
            self.validate_organization_cursor(
                after,
                maximum_inspected,
                maximum_returned,
                maximum_copied_bytes,
            )?;
            let scope = scope.application_selector()?;
            let page = self.universe.inspect_organization_incident_services(
                &scope,
                after.map(|cursor| &cursor.application),
                maximum_inspected,
                maximum_returned,
            )?;
            self.project_organization_page(
                page,
                maximum_inspected,
                maximum_returned,
                maximum_copied_bytes,
            )
        };
        execute().map_err(|error| failure(&self.diagnostic_identity, error))
    }

    /// Inspect exact endpoint records or all corresponding occupant/provider bindings.
    ///
    /// The selector reuses compiler identifier parsing; it grants no authority.
    /// Visits charge every candidate, including nonmatches. Found is cumulative;
    /// partial empty pages never establish absence. External resources produce no
    /// invented local record. Returned entries retain the existing source/detail
    /// owner, including explicit unmapped source and independent effective states.
    ///
    /// Raw limits are 1..=256 and the copied-string budget is independently bounded
    /// by [`MAX_ORGANIZATION_COPY_BYTES`]. Selector/cursor metadata, retained source,
    /// allocator overhead and encoded bytes are separate from that output budget.
    ///
    /// # Errors
    /// Invalid identifiers, limits or custody reject before scanning. Copy exhaustion
    /// returns no page or advanced cursor. This is not recipient-qualified delivery.
    pub fn inspect_organization_endpoint(
        &self,
        endpoint: &OrganizationNode,
        after: Option<&OrganizationCursor>,
        maximum_inspected: u16,
        maximum_returned: u16,
        maximum_copied_bytes: u32,
    ) -> Result<OrganizationPage, ApiError> {
        let execute = || -> Result<OrganizationPage, OrganizationFailure> {
            self.validate_organization_cursor(
                after,
                maximum_inspected,
                maximum_returned,
                maximum_copied_bytes,
            )?;
            let endpoint = endpoint.application_selector()?;
            let page = self.universe.inspect_organization_endpoint(
                &endpoint,
                after.map(|cursor| &cursor.application),
                maximum_inspected,
                maximum_returned,
            )?;
            self.project_organization_page(
                page,
                maximum_inspected,
                maximum_returned,
                maximum_copied_bytes,
            )
        };
        execute().map_err(|error| failure(&self.diagnostic_identity, error))
    }

    /// Inspect one-hop explicit and already-derived relations at either exact endpoint.
    ///
    /// Preserves direction, self-loops, parallel relations and full interval/source
    /// coordinates. It does not derive new edges or include structural associations,
    /// transitive impact, source/proof lookup or native observation. Endpoint lookup
    /// remains a separate query: an empty incident page does not establish absence.
    /// Bounds, copied-entry custody and recipient limits match
    /// [`Self::inspect_organization_endpoint`].
    ///
    /// # Errors
    /// Invalid identifiers, limits or a changed endpoint/query/owner reject before
    /// scanning. Copy exhaustion leaves the caller's original continuation usable.
    pub fn inspect_organization_incident_relations(
        &self,
        endpoint: &OrganizationNode,
        after: Option<&OrganizationCursor>,
        maximum_inspected: u16,
        maximum_returned: u16,
        maximum_copied_bytes: u32,
    ) -> Result<OrganizationPage, ApiError> {
        let execute = || -> Result<OrganizationPage, OrganizationFailure> {
            self.validate_organization_cursor(
                after,
                maximum_inspected,
                maximum_returned,
                maximum_copied_bytes,
            )?;
            let endpoint = endpoint.application_selector()?;
            let page = self.universe.inspect_organization_incident_relations(
                &endpoint,
                after.map(|cursor| &cursor.application),
                maximum_inspected,
                maximum_returned,
            )?;
            self.project_organization_page(
                page,
                maximum_inspected,
                maximum_returned,
                maximum_copied_bytes,
            )
        };
        execute().map_err(|error| failure(&self.diagnostic_identity, error))
    }
}

fn identifier<T: std::str::FromStr>(value: &str) -> Result<T, OrganizationFailure> {
    value
        .parse()
        .map_err(|_| OrganizationFailure::InvalidEndpoint)
}

impl OrganizationNode {
    pub(super) fn application_selector(
        &self,
    ) -> Result<core::OrganizationalNodeRefV1, OrganizationFailure> {
        use core::OrganizationalNodeRefV1 as N;
        Ok(match self {
            Self::Scope(v) => N::Scope {
                scope: v.application_selector()?,
            },
            Self::Organization(v) => N::Organization {
                organization: core::LogicalOrganizationRefV1 {
                    namespace: identifier(&v.namespace)?,
                    organization_id: identifier(&v.organization_id)?,
                },
            },
            Self::Group(v) => N::Group {
                group: core::OrganizationalGroupRefV1 {
                    namespace: identifier(&v.namespace)?,
                    group_id: identifier(&v.group_id)?,
                },
            },
            Self::DisciplineTree(v) => N::DisciplineTree {
                tree: core::DisciplineTreeRefV1 {
                    namespace: identifier(&v.namespace)?,
                    tree_id: identifier(&v.tree_id)?,
                },
            },
            Self::Role(v) => N::Role {
                role: core::OrganizationalRoleRefV1 {
                    namespace: identifier(&v.namespace)?,
                    role_id: identifier(&v.role_id)?,
                },
            },
            Self::Occupant(v) => N::Occupant {
                occupant: identifier(v)?,
            },
            Self::Provider(v) => N::Provider {
                provider: identifier(v)?,
            },
            Self::NativeResource(v) => N::NativeResource {
                resource: identifier(v)?,
            },
            Self::Object(v) => N::Object {
                object: identifier(v)?,
            },
        })
    }
}

impl OrganizationScopeRef {
    pub(super) fn application_selector(
        &self,
    ) -> Result<core::LogicalScopeRefV1, OrganizationFailure> {
        Ok(core::LogicalScopeRefV1 {
            namespace: identifier(&self.namespace)?,
            scope_id: identifier(&self.scope_id)?,
        })
    }
}
