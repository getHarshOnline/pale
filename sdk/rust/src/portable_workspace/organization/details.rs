//! Owned typed collection pages, preserving the application's nested custody.

use pale_application::workspace as app;
use pale_core::records as core;

use super::*;

/// Exact section continuation. No private member position appears in this API.
#[derive(Clone)]
pub struct OrganizationDetailCursor {
    parent: OrganizationDetailParent,
    application: app::OrganizationMemberCursor,
    maximum_inspected: u16,
    maximum_returned: u16,
    maximum_copied_bytes: u32,
}

impl std::fmt::Debug for OrganizationDetailCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrganizationDetailCursor")
            .finish_non_exhaustive()
    }
}

/// Actual parent custody of a detail result, independently retained after return.
///
/// These are issued opaque handles, not generic semantic cell values. A nested
/// parent remains distinguishable even when its returned child list is empty.
#[derive(Clone, Debug)]
pub enum OrganizationDetailParent {
    /// A top-level record section.
    Record(OrganizationRecordHandle),
    /// One actual communication local resolution.
    Communication(OrganizationCommunicationResolutionHandle),
    /// One actual service assignment.
    Assignment(OrganizationServiceAssignmentHandle),
    /// One actual responsibility-native projection.
    Projection(OrganizationResponsibilityProjectionHandle),
    /// One actual genealogy link.
    Genealogy(OrganizationGenealogyHandle),
}

impl OrganizationDetailParent {
    /// Root record custody; source mapping is currently record-level only.
    #[must_use]
    pub fn record(&self) -> &OrganizationRecordHandle {
        match self {
            Self::Record(v) => v,
            Self::Communication(v) => &v.record,
            Self::Assignment(v) => &v.record,
            Self::Projection(v) => &v.record,
            Self::Genealogy(v) => &v.record,
        }
    }
}

/// Closed structural section payloads; each variant retains its exact owning meaning.
#[derive(Clone, Debug)]
pub enum OrganizationDetailRows {
    /// Actual authority-resource members.
    AxisResources(Vec<String>),
    /// Actual participating role references.
    LeadershipParticipants(Vec<OrganizationRoleRef>),
    /// Independent affected scope references.
    LeadershipScopes(Vec<OrganizationScopeRef>),
    /// Separately retained local-resolution headers.
    CommunicationResolutions(Vec<OrganizationCommunicationResolution>),
    /// One exact local resolution's authorized closure.
    CommunicationClosure(Vec<OrganizationNode>),
    /// Exact named standards.
    LensStandards(Vec<String>),
    /// Exact bounded-block conditions.
    LensConditions(Vec<String>),
    /// Actual declared lens rights.
    LensRights(Vec<OrganizationLensRight>),
    /// Shallow account/mandate assignments with exact nested custody.
    ServiceAssignments(Vec<OrganizationServiceAssignment>),
    /// Stewards of one actual assignment.
    AssignmentStewards(Vec<String>),
    /// Pods of one actual assignment.
    AssignmentPods(Vec<String>),
    /// Independent service use-right members.
    ServiceRights(Vec<String>),
    /// Independent service controller references.
    ServiceControllers(Vec<String>),
    /// Independent service authority references.
    ServiceAuthorities(Vec<String>),
    /// Desired native references of one organizational projection.
    NativeReferences(Vec<String>),
    /// Configured coordinate classes, not a grant.
    InspectionCoordinates(Vec<OrganizationInspectionCoordinate>),
    /// Configured effective-state classes.
    InspectionStates(Vec<OrganizationFactState>),
    /// Exact structural answers and omissions.
    ResponsibilitySelections(Vec<OrganizationStructuralSelection>),
    /// Actual responsibility residence decisions.
    ResponsibilityResidences(Vec<OrganizationResidence>),
    /// Shallow desired native projections.
    ResponsibilityProjections(Vec<OrganizationResponsibilityProjection>),
    /// Native references of one exact responsibility projection.
    ProjectionReferences(Vec<String>),
    /// Shallow links retaining three separate member sets.
    ResponsibilityGenealogy(Vec<OrganizationGenealogyLink>),
    /// Responsibility identities of one actual link.
    GenealogyResponsibilities(Vec<String>),
    /// Projection identities of one actual link.
    GenealogyProjections(Vec<String>),
    /// Body-free occurrence identities of one actual link.
    GenealogyOccurrences(Vec<String>),
}

/// Owned values from one concrete typed structural section, not a universal value bag.
#[derive(Clone, Debug)]
pub struct OrganizationDetailPage {
    parent: OrganizationDetailParent,
    rows: OrganizationDetailRows,
    inspected: u16,
    copied_string_bytes: u32,
    continuation: Option<OrganizationDetailCursor>,
}

impl OrganizationDetailPage {
    /// Exact independent parent handle, including an actual empty member page.
    #[must_use]
    pub const fn parent(&self) -> &OrganizationDetailParent {
        &self.parent
    }
    /// Exact section-specific owned values in the existing collection order.
    #[must_use]
    pub const fn rows(&self) -> &OrganizationDetailRows {
        &self.rows
    }
    /// Direct member cells visited; nested child values are charged by separate calls.
    #[must_use]
    pub const fn inspected(&self) -> u16 {
        self.inspected
    }
    /// Copied string payload; not allocator/encoded/retained-compilation bytes.
    #[must_use]
    pub const fn copied_string_bytes(&self) -> u32 {
        self.copied_string_bytes
    }
    /// None only after the application established this section's exhaustion.
    #[must_use]
    pub const fn continuation(&self) -> Option<&OrganizationDetailCursor> {
        self.continuation.as_ref()
    }
    /// Section exhaustion only, not completeness of the record or World.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.continuation.is_none()
    }
}

/// Independently retainable exact local resolution; not reconstructed from its node.
#[derive(Clone, Debug)]
pub struct OrganizationCommunicationResolutionHandle {
    record: OrganizationRecordHandle,
    application: app::OrganizationCommunicationResolutionHandle,
}
/// Independently retainable exact service assignment.
#[derive(Clone, Debug)]
pub struct OrganizationServiceAssignmentHandle {
    record: OrganizationRecordHandle,
    application: app::OrganizationServiceAssignmentHandle,
}
/// Independently retainable exact responsibility projection.
#[derive(Clone, Debug)]
pub struct OrganizationResponsibilityProjectionHandle {
    record: OrganizationRecordHandle,
    application: app::OrganizationResponsibilityProjectionHandle,
}
/// Independently retainable exact genealogy link.
#[derive(Clone, Debug)]
pub struct OrganizationGenealogyHandle {
    record: OrganizationRecordHandle,
    application: app::OrganizationGenealogyHandle,
}

/// One shallow local resolution with its own separately queryable closure.
#[derive(Clone, Debug)]
pub struct OrganizationCommunicationResolution {
    /// Actual local node, not a merged provider-wide authorization set.
    pub local_node: OrganizationNode,
    handle: OrganizationCommunicationResolutionHandle,
}
impl OrganizationCommunicationResolution {
    /// Retain this actual member and the complete compilation independently.
    #[must_use]
    pub const fn handle(&self) -> &OrganizationCommunicationResolutionHandle {
        &self.handle
    }
}

/// One shallow assignment; stewards and pods remain separate actual member sets.
#[derive(Clone, Debug)]
pub struct OrganizationServiceAssignment {
    /// Exact account.
    pub account: String,
    /// Exact mandate.
    pub mandate: String,
    /// Independently declared state.
    pub state: OrganizationFactState,
    /// Actual assignment interval.
    pub interval: OrganizationInterval,
    handle: OrganizationServiceAssignmentHandle,
}
impl OrganizationServiceAssignment {
    /// Retain this exact assignment, not a lookup by account/mandate equality.
    #[must_use]
    pub const fn handle(&self) -> &OrganizationServiceAssignmentHandle {
        &self.handle
    }
}

/// One desired native projection; no native body or realization proof is copied.
#[derive(Clone, Debug)]
pub struct OrganizationResponsibilityProjection {
    /// Exact projection identity.
    pub projection_id: String,
    /// Existing provider-neutral class.
    pub class: OrganizationProjectionClass,
    /// Owner retaining native truth.
    pub native_owner: String,
    /// Actual independent state.
    pub state: OrganizationFactState,
    handle: OrganizationResponsibilityProjectionHandle,
}
impl OrganizationResponsibilityProjection {
    /// Retain every native reference through its actual parent, not the first member.
    #[must_use]
    pub const fn handle(&self) -> &OrganizationResponsibilityProjectionHandle {
        &self.handle
    }
}

/// One genealogy link with separately paged responsibility/projection/occurrence sets.
#[derive(Clone, Debug)]
pub struct OrganizationGenealogyLink {
    /// Exact link identity.
    pub link_id: String,
    handle: OrganizationGenealogyHandle,
}
impl OrganizationGenealogyLink {
    /// Retain this actual link and all three independently queryable associations.
    #[must_use]
    pub const fn handle(&self) -> &OrganizationGenealogyHandle {
        &self.handle
    }
}

/// Existing declared group-lens right; no independent rights policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrganizationLensRight {
    /// Declared named standards right.
    Standards,
    /// Declared named review right.
    Review,
    /// Block only under declared conditions.
    BoundedBlock,
    /// Escalate only through the declared resolver.
    Escalation,
}
/// Configured inspection coordinate, not authenticated disclosure context.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrganizationInspectionCoordinate {
    /// Logical scopes and organizations.
    ScopeOrganization,
    /// Groups and discipline trees.
    GroupTree,
    /// Stable roles and bindings.
    RoleBinding,
    /// Owner-selected atomic objects.
    AtomicObject,
    /// Group-lens authority profiles.
    GroupLens,
    /// Directional service assignments.
    DirectionalService,
    /// Independent acceptance/effect frontiers.
    AcceptanceFrontier,
    /// Desired native projections.
    NativeProjection,
    /// Explicit negative/unresolved states.
    EffectiveState,
}
/// Actual structural question class, not a name-based inferred object type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrganizationStructuralKind {
    /// Independent logical organization.
    LogicalOrganization,
    /// Stable role surviving occupant replacement.
    StableRole,
    /// Current occupant or provider binding.
    OccupantProvider,
    /// Exact directional service contract.
    DirectionalServiceContract,
    /// Accountable-work native projection.
    AccountableWorkProjection,
    /// Communication native projection.
    CommunicationProjection,
    /// Relationship/commercial native projection.
    RelationshipProjection,
    /// Replayable native application.
    NativeApplication,
    /// Transferable procedure Package.
    Package,
    /// Owner-attested normative Pack.
    NormativePack,
    /// Exact reference without copied body.
    ReferenceOnly,
}
/// Actual owner-declared residence class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrganizationResidenceClass {
    /// Reusable method or obligations.
    Package,
    /// Owner-attested normative content.
    NormativePack,
    /// Replayable native truth or effect.
    NativeApplication,
    /// Principal-private selection or binding.
    Configuration,
    /// Exact reference to another owner's truth.
    Reference,
}

/// Existing structural selection or attributed omission.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrganizationStructuralSelection {
    /// Named structural question.
    pub kind: OrganizationStructuralKind,
    /// Actual generic resource answer; not cast into a role or scope identity.
    pub selection: OrganizationResourceAnswer,
}

/// Existing residence decision, without copying another owner's content.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrganizationResidence {
    /// Actual residence class.
    pub class: OrganizationResidenceClass,
    /// Exact selected resource/release reference.
    pub resource: String,
    /// Owner of that resource.
    pub owner: String,
}

struct DetailLimits<'a> {
    after: Option<&'a OrganizationDetailCursor>,
    inspected: u16,
    returned: u16,
    copied: u32,
}

impl OrganizationRecordHandle {
    fn detail<'a, T: 'a, U>(
        &'a self,
        parent: OrganizationDetailParent,
        section: app::WorkspaceInspectionOrganizationSection,
        limits: DetailLimits<'_>,
        wrap: fn(Vec<U>) -> OrganizationDetailRows,
        query: impl FnOnce(
            &'a PortableWorkspaceUniverse,
            Option<&app::OrganizationMemberCursor>,
            u16,
            u16,
        ) -> Result<app::OrganizationMemberPage<'a, T>, ApplicationError>,
        mut project: impl FnMut(
            app::OrganizationMemberEntry<'a, T>,
            &mut CopyBudget,
        ) -> Result<U, OrganizationFailure>,
    ) -> Result<OrganizationDetailPage, ApiError> {
        let DetailLimits {
            after,
            inspected,
            returned,
            copied,
        } = limits;
        let _ = section;
        let execute = || -> Result<OrganizationDetailPage, OrganizationFailure> {
            validate_limits(inspected, returned, copied)?;
            if after.is_some_and(|cursor| {
                !Arc::ptr_eq(&self.owner, &cursor.parent.record().owner)
                    || cursor.maximum_inspected != inspected
                    || cursor.maximum_returned != returned
                    || cursor.maximum_copied_bytes != copied
            }) {
                return Err(OrganizationFailure::ContinuationMismatch);
            }
            // The application checks the exact record, section and nested
            // parent; SDK never extracts or reconstructs positional identity.
            let page = query(
                &self.owner,
                after.map(|cursor| &cursor.application),
                inspected,
                returned,
            )?;
            let mut entries = Vec::new();
            entries.try_reserve_exact(page.entries().len())?;
            let mut budget = CopyBudget::new(copied);
            for entry in page.entries() {
                entries.push(project(entry, &mut budget)?);
            }
            let continuation =
                page.continuation()
                    .cloned()
                    .map(|application| OrganizationDetailCursor {
                        parent: parent.clone(),
                        application,
                        maximum_inspected: inspected,
                        maximum_returned: returned,
                        maximum_copied_bytes: copied,
                    });
            Ok(OrganizationDetailPage {
                parent,
                rows: wrap(entries),
                inspected: page.inspected(),
                copied_string_bytes: budget.used,
                continuation,
            })
        };
        execute().map_err(|error| failure(&self.diagnostic_identity, error))
    }

    /// Copy an existing authority-resource section without changing its meaning.
    /// # Errors
    /// Invalid limits/custody/section, copy budget or allocation failure.
    pub fn axis_resources(
        &self,
        after: Option<&OrganizationDetailCursor>,
        inspected: u16,
        returned: u16,
        copied: u32,
    ) -> Result<OrganizationDetailPage, ApiError> {
        self.detail(
            OrganizationDetailParent::Record(self.clone()),
            app::WorkspaceInspectionOrganizationSection::AxisResources,
            DetailLimits {
                after,
                inspected,
                returned,
                copied,
            },
            OrganizationDetailRows::AxisResources,
            |owner, after, i, r| {
                owner
                    .resolve_organization_record(&self.application)?
                    .axis_resources(after, i, r)
            },
            |entry, b| b.text(entry.value().as_str()),
        )
    }

    /// Copy actual participating roles, not a newly derived cross-product.
    /// # Errors
    /// Invalid limits/custody/section, copy budget or allocation failure.
    pub fn leadership_participants(
        &self,
        after: Option<&OrganizationDetailCursor>,
        inspected: u16,
        returned: u16,
        copied: u32,
    ) -> Result<OrganizationDetailPage, ApiError> {
        self.detail(
            OrganizationDetailParent::Record(self.clone()),
            app::WorkspaceInspectionOrganizationSection::LeadershipParticipants,
            DetailLimits {
                after,
                inspected,
                returned,
                copied,
            },
            OrganizationDetailRows::LeadershipParticipants,
            |owner, after, i, r| {
                owner
                    .resolve_organization_record(&self.application)?
                    .leadership_participants(after, i, r)
            },
            |entry, b| projection::role(entry.value(), b),
        )
    }

    /// Copy the independent actual affected-scope set.
    /// # Errors
    /// Invalid limits/custody/section, copy budget or allocation failure.
    pub fn leadership_scopes(
        &self,
        after: Option<&OrganizationDetailCursor>,
        inspected: u16,
        returned: u16,
        copied: u32,
    ) -> Result<OrganizationDetailPage, ApiError> {
        self.detail(
            OrganizationDetailParent::Record(self.clone()),
            app::WorkspaceInspectionOrganizationSection::LeadershipScopes,
            DetailLimits {
                after,
                inspected,
                returned,
                copied,
            },
            OrganizationDetailRows::LeadershipScopes,
            |owner, after, i, r| {
                owner
                    .resolve_organization_record(&self.application)?
                    .leadership_scopes(after, i, r)
            },
            |entry, b| projection::scope(entry.value(), b),
        )
    }

    /// Copy shallow local-resolution nodes and issue exact nested handles.
    /// # Errors
    /// Invalid limits/custody/section, copy budget or allocation failure.
    pub fn communication_resolutions(
        &self,
        after: Option<&OrganizationDetailCursor>,
        inspected: u16,
        returned: u16,
        copied: u32,
    ) -> Result<OrganizationDetailPage, ApiError> {
        self.detail(
            OrganizationDetailParent::Record(self.clone()),
            app::WorkspaceInspectionOrganizationSection::CommunicationResolutions,
            DetailLimits {
                after,
                inspected,
                returned,
                copied,
            },
            OrganizationDetailRows::CommunicationResolutions,
            |owner, after, i, r| {
                owner
                    .resolve_organization_record(&self.application)?
                    .communication_resolutions(after, i, r)
            },
            |entry, b| {
                Ok(OrganizationCommunicationResolution {
                    local_node: projection::node(&entry.value().local_node, b)?,
                    handle: OrganizationCommunicationResolutionHandle {
                        record: self.clone(),
                        application: entry.retain(),
                    },
                })
            },
        )
    }

    /// Copy actual lens standards, without interpreting their resource names.
    /// # Errors
    /// Invalid limits/custody/section, copy budget or allocation failure.
    pub fn lens_standards(
        &self,
        after: Option<&OrganizationDetailCursor>,
        inspected: u16,
        returned: u16,
        copied: u32,
    ) -> Result<OrganizationDetailPage, ApiError> {
        self.detail(
            OrganizationDetailParent::Record(self.clone()),
            app::WorkspaceInspectionOrganizationSection::LensStandards,
            DetailLimits {
                after,
                inspected,
                returned,
                copied,
            },
            OrganizationDetailRows::LensStandards,
            |owner, after, i, r| {
                owner
                    .resolve_organization_record(&self.application)?
                    .lens_standards(after, i, r)
            },
            |entry, b| b.text(entry.value().as_str()),
        )
    }

    /// Copy actual bounded-block conditions without granting a block action.
    /// # Errors
    /// Invalid limits/custody/section, copy budget or allocation failure.
    pub fn lens_conditions(
        &self,
        after: Option<&OrganizationDetailCursor>,
        inspected: u16,
        returned: u16,
        copied: u32,
    ) -> Result<OrganizationDetailPage, ApiError> {
        self.detail(
            OrganizationDetailParent::Record(self.clone()),
            app::WorkspaceInspectionOrganizationSection::LensConditions,
            DetailLimits {
                after,
                inspected,
                returned,
                copied,
            },
            OrganizationDetailRows::LensConditions,
            |owner, after, i, r| {
                owner
                    .resolve_organization_record(&self.application)?
                    .lens_conditions(after, i, r)
            },
            |entry, b| b.text(entry.value().as_str()),
        )
    }

    /// Copy the exact declared rights; no rights-ranking policy is added.
    /// # Errors
    /// Invalid limits/custody/section or allocation failure.
    pub fn lens_rights(
        &self,
        after: Option<&OrganizationDetailCursor>,
        inspected: u16,
        returned: u16,
        copied: u32,
    ) -> Result<OrganizationDetailPage, ApiError> {
        self.detail(
            OrganizationDetailParent::Record(self.clone()),
            app::WorkspaceInspectionOrganizationSection::LensRights,
            DetailLimits {
                after,
                inspected,
                returned,
                copied,
            },
            OrganizationDetailRows::LensRights,
            |owner, after, i, r| {
                owner
                    .resolve_organization_record(&self.application)?
                    .lens_rights(after, i, r)
            },
            |entry, _| Ok(lens_right(*entry.value())),
        )
    }

    /// Copy shallow account/mandate cells while retaining each exact assignment.
    /// # Errors
    /// Invalid limits/custody/section, copy budget or allocation failure.
    pub fn service_assignments(
        &self,
        after: Option<&OrganizationDetailCursor>,
        inspected: u16,
        returned: u16,
        copied: u32,
    ) -> Result<OrganizationDetailPage, ApiError> {
        self.detail(
            OrganizationDetailParent::Record(self.clone()),
            app::WorkspaceInspectionOrganizationSection::ServiceAssignments,
            DetailLimits {
                after,
                inspected,
                returned,
                copied,
            },
            OrganizationDetailRows::ServiceAssignments,
            |owner, after, i, r| {
                owner
                    .resolve_organization_record(&self.application)?
                    .service_assignments(after, i, r)
            },
            |entry, b| {
                let value = entry.value();
                Ok(OrganizationServiceAssignment {
                    account: b.text(value.account.as_str())?,
                    mandate: b.text(value.mandate.as_str())?,
                    state: projection::state(value.state),
                    interval: projection::interval(&value.interval),
                    handle: OrganizationServiceAssignmentHandle {
                        record: self.clone(),
                        application: entry.retain(),
                    },
                })
            },
        )
    }

    /// Actual service use rights, distinct from controllers and authorities.
    /// # Errors
    /// Invalid limits/custody/section, copy budget or allocation failure.
    pub fn service_rights(
        &self,
        after: Option<&OrganizationDetailCursor>,
        inspected: u16,
        returned: u16,
        copied: u32,
    ) -> Result<OrganizationDetailPage, ApiError> {
        self.detail(
            OrganizationDetailParent::Record(self.clone()),
            app::WorkspaceInspectionOrganizationSection::ServiceRights,
            DetailLimits {
                after,
                inspected,
                returned,
                copied,
            },
            OrganizationDetailRows::ServiceRights,
            |owner, after, i, r| {
                owner
                    .resolve_organization_record(&self.application)?
                    .service_rights(after, i, r)
            },
            |entry, b| b.text(entry.value().as_str()),
        )
    }

    /// Actual service controller references, not proofs of current control.
    /// # Errors
    /// Invalid limits/custody/section, copy budget or allocation failure.
    pub fn service_controllers(
        &self,
        after: Option<&OrganizationDetailCursor>,
        inspected: u16,
        returned: u16,
        copied: u32,
    ) -> Result<OrganizationDetailPage, ApiError> {
        self.detail(
            OrganizationDetailParent::Record(self.clone()),
            app::WorkspaceInspectionOrganizationSection::ServiceControllers,
            DetailLimits {
                after,
                inspected,
                returned,
                copied,
            },
            OrganizationDetailRows::ServiceControllers,
            |owner, after, i, r| {
                owner
                    .resolve_organization_record(&self.application)?
                    .service_controllers(after, i, r)
            },
            |entry, b| b.text(entry.value().as_str()),
        )
    }

    /// Actual service authority references, not a newly admitted request ceiling.
    /// # Errors
    /// Invalid limits/custody/section, copy budget or allocation failure.
    pub fn service_authorities(
        &self,
        after: Option<&OrganizationDetailCursor>,
        inspected: u16,
        returned: u16,
        copied: u32,
    ) -> Result<OrganizationDetailPage, ApiError> {
        self.detail(
            OrganizationDetailParent::Record(self.clone()),
            app::WorkspaceInspectionOrganizationSection::ServiceAuthorities,
            DetailLimits {
                after,
                inspected,
                returned,
                copied,
            },
            OrganizationDetailRows::ServiceAuthorities,
            |owner, after, i, r| {
                owner
                    .resolve_organization_record(&self.application)?
                    .service_authorities(after, i, r)
            },
            |entry, b| b.text(entry.value().as_str()),
        )
    }

    /// All actual organizational native references; an empty set stays explicit.
    /// # Errors
    /// Invalid limits/custody/section, copy budget or allocation failure.
    pub fn native_references(
        &self,
        after: Option<&OrganizationDetailCursor>,
        inspected: u16,
        returned: u16,
        copied: u32,
    ) -> Result<OrganizationDetailPage, ApiError> {
        self.detail(
            OrganizationDetailParent::Record(self.clone()),
            app::WorkspaceInspectionOrganizationSection::NativeReferences,
            DetailLimits {
                after,
                inspected,
                returned,
                copied,
            },
            OrganizationDetailRows::NativeReferences,
            |owner, after, i, r| {
                owner
                    .resolve_organization_record(&self.application)?
                    .native_references(after, i, r)
            },
            |entry, b| b.text(entry.value().as_str()),
        )
    }

    /// Actual configured coordinate classes; not a disclosure grant.
    /// # Errors
    /// Invalid limits/custody/section or allocation failure.
    pub fn inspection_coordinates(
        &self,
        after: Option<&OrganizationDetailCursor>,
        inspected: u16,
        returned: u16,
        copied: u32,
    ) -> Result<OrganizationDetailPage, ApiError> {
        self.detail(
            OrganizationDetailParent::Record(self.clone()),
            app::WorkspaceInspectionOrganizationSection::InspectionCoordinates,
            DetailLimits {
                after,
                inspected,
                returned,
                copied,
            },
            OrganizationDetailRows::InspectionCoordinates,
            |owner, after, i, r| {
                owner
                    .resolve_organization_record(&self.application)?
                    .inspection_coordinates(after, i, r)
            },
            |entry, _| Ok(inspection_coordinate(*entry.value())),
        )
    }

    /// Actual configured state classes, preserving negative-state distinctions.
    /// # Errors
    /// Invalid limits/custody/section or allocation failure.
    pub fn inspection_states(
        &self,
        after: Option<&OrganizationDetailCursor>,
        inspected: u16,
        returned: u16,
        copied: u32,
    ) -> Result<OrganizationDetailPage, ApiError> {
        self.detail(
            OrganizationDetailParent::Record(self.clone()),
            app::WorkspaceInspectionOrganizationSection::InspectionStates,
            DetailLimits {
                after,
                inspected,
                returned,
                copied,
            },
            OrganizationDetailRows::InspectionStates,
            |owner, after, i, r| {
                owner
                    .resolve_organization_record(&self.application)?
                    .inspection_states(after, i, r)
            },
            |entry, _| Ok(projection::state(*entry.value())),
        )
    }

    /// Exact structural answers, retaining every supplied/omitted attribution.
    /// # Errors
    /// Invalid limits/custody/section, copy budget or allocation failure.
    pub fn responsibility_selections(
        &self,
        after: Option<&OrganizationDetailCursor>,
        inspected: u16,
        returned: u16,
        copied: u32,
    ) -> Result<OrganizationDetailPage, ApiError> {
        self.detail(
            OrganizationDetailParent::Record(self.clone()),
            app::WorkspaceInspectionOrganizationSection::ResponsibilitySelections,
            DetailLimits {
                after,
                inspected,
                returned,
                copied,
            },
            OrganizationDetailRows::ResponsibilitySelections,
            |owner, after, i, r| {
                owner
                    .resolve_organization_record(&self.application)?
                    .responsibility_selections(after, i, r)
            },
            |entry, b| {
                Ok(OrganizationStructuralSelection {
                    kind: structural_kind(entry.value().kind),
                    selection: projection::resource_answer(&entry.value().selection, b)?,
                })
            },
        )
    }

    /// Exact residence decisions, not copied Package/native content.
    /// # Errors
    /// Invalid limits/custody/section, copy budget or allocation failure.
    pub fn responsibility_residences(
        &self,
        after: Option<&OrganizationDetailCursor>,
        inspected: u16,
        returned: u16,
        copied: u32,
    ) -> Result<OrganizationDetailPage, ApiError> {
        self.detail(
            OrganizationDetailParent::Record(self.clone()),
            app::WorkspaceInspectionOrganizationSection::ResponsibilityResidences,
            DetailLimits {
                after,
                inspected,
                returned,
                copied,
            },
            OrganizationDetailRows::ResponsibilityResidences,
            |owner, after, i, r| {
                owner
                    .resolve_organization_record(&self.application)?
                    .responsibility_residences(after, i, r)
            },
            |entry, b| {
                Ok(OrganizationResidence {
                    class: residence_class(entry.value().class),
                    resource: b.text(entry.value().resource.as_str())?,
                    owner: b.text(entry.value().owner.as_str())?,
                })
            },
        )
    }

    /// Shallow native projections with independently retained native-reference custody.
    /// # Errors
    /// Invalid limits/custody/section, copy budget or allocation failure.
    pub fn responsibility_projections(
        &self,
        after: Option<&OrganizationDetailCursor>,
        inspected: u16,
        returned: u16,
        copied: u32,
    ) -> Result<OrganizationDetailPage, ApiError> {
        self.detail(
            OrganizationDetailParent::Record(self.clone()),
            app::WorkspaceInspectionOrganizationSection::ResponsibilityProjections,
            DetailLimits {
                after,
                inspected,
                returned,
                copied,
            },
            OrganizationDetailRows::ResponsibilityProjections,
            |owner, after, i, r| {
                owner
                    .resolve_organization_record(&self.application)?
                    .responsibility_projections(after, i, r)
            },
            |entry, b| {
                Ok(OrganizationResponsibilityProjection {
                    projection_id: b.text(entry.value().projection_id.as_str())?,
                    class: projection::projection_class(entry.value().class),
                    native_owner: b.text(entry.value().native_owner.as_str())?,
                    state: projection::state(entry.value().state),
                    handle: OrganizationResponsibilityProjectionHandle {
                        record: self.clone(),
                        application: entry.retain(),
                    },
                })
            },
        )
    }

    /// Shallow genealogy links, without collapsing their three identity classes.
    /// # Errors
    /// Invalid limits/custody/section, copy budget or allocation failure.
    pub fn responsibility_genealogy(
        &self,
        after: Option<&OrganizationDetailCursor>,
        inspected: u16,
        returned: u16,
        copied: u32,
    ) -> Result<OrganizationDetailPage, ApiError> {
        self.detail(
            OrganizationDetailParent::Record(self.clone()),
            app::WorkspaceInspectionOrganizationSection::ResponsibilityGenealogy,
            DetailLimits {
                after,
                inspected,
                returned,
                copied,
            },
            OrganizationDetailRows::ResponsibilityGenealogy,
            |owner, after, i, r| {
                owner
                    .resolve_organization_record(&self.application)?
                    .responsibility_genealogy(after, i, r)
            },
            |entry, b| {
                Ok(OrganizationGenealogyLink {
                    link_id: b.text(entry.value().link_id.as_str())?,
                    handle: OrganizationGenealogyHandle {
                        record: self.clone(),
                        application: entry.retain(),
                    },
                })
            },
        )
    }
}

impl OrganizationCommunicationResolutionHandle {
    /// Copy this exact resolution's closure, never a union of equal local nodes.
    /// # Errors
    /// Wrong nested parent/section/raw limits, copy budget or allocation failure.
    pub fn authorized_closure(
        &self,
        after: Option<&OrganizationDetailCursor>,
        inspected: u16,
        returned: u16,
        copied: u32,
    ) -> Result<OrganizationDetailPage, ApiError> {
        self.record.detail(
            OrganizationDetailParent::Communication(self.clone()),
            app::WorkspaceInspectionOrganizationSection::CommunicationClosure,
            DetailLimits {
                after,
                inspected,
                returned,
                copied,
            },
            OrganizationDetailRows::CommunicationClosure,
            |owner, after, i, r| self.application.authorized_closure(owner, after, i, r),
            |entry, b| projection::node(entry.value(), b),
        )
    }
}
impl OrganizationServiceAssignmentHandle {
    /// Exact stewards for this assignment; no positional primary member.
    /// # Errors
    /// Wrong nested parent/section/raw limits, copy budget or allocation failure.
    pub fn stewards(
        &self,
        after: Option<&OrganizationDetailCursor>,
        inspected: u16,
        returned: u16,
        copied: u32,
    ) -> Result<OrganizationDetailPage, ApiError> {
        self.record.detail(
            OrganizationDetailParent::Assignment(self.clone()),
            app::WorkspaceInspectionOrganizationSection::AssignmentStewards,
            DetailLimits {
                after,
                inspected,
                returned,
                copied,
            },
            OrganizationDetailRows::AssignmentStewards,
            |owner, after, i, r| self.application.stewards(owner, after, i, r),
            |entry, b| b.text(entry.value().as_str()),
        )
    }
    /// Exact pods, separately from this assignment's stewards.
    /// # Errors
    /// Wrong nested parent/section/raw limits, copy budget or allocation failure.
    pub fn pods(
        &self,
        after: Option<&OrganizationDetailCursor>,
        inspected: u16,
        returned: u16,
        copied: u32,
    ) -> Result<OrganizationDetailPage, ApiError> {
        self.record.detail(
            OrganizationDetailParent::Assignment(self.clone()),
            app::WorkspaceInspectionOrganizationSection::AssignmentPods,
            DetailLimits {
                after,
                inspected,
                returned,
                copied,
            },
            OrganizationDetailRows::AssignmentPods,
            |owner, after, i, r| self.application.pods(owner, after, i, r),
            |entry, b| b.text(entry.value().as_str()),
        )
    }
}
impl OrganizationResponsibilityProjectionHandle {
    /// Every actual native reference under this retained projection.
    /// # Errors
    /// Wrong nested parent/section/raw limits, copy budget or allocation failure.
    pub fn native_references(
        &self,
        after: Option<&OrganizationDetailCursor>,
        inspected: u16,
        returned: u16,
        copied: u32,
    ) -> Result<OrganizationDetailPage, ApiError> {
        self.record.detail(
            OrganizationDetailParent::Projection(self.clone()),
            app::WorkspaceInspectionOrganizationSection::ProjectionReferences,
            DetailLimits {
                after,
                inspected,
                returned,
                copied,
            },
            OrganizationDetailRows::ProjectionReferences,
            |owner, after, i, r| self.application.native_references(owner, after, i, r),
            |entry, b| b.text(entry.value().as_str()),
        )
    }
}
impl OrganizationGenealogyHandle {
    /// Actual responsibility members; projection and occurrence IDs stay separate.
    /// # Errors
    /// Wrong nested parent/section/raw limits, copy budget or allocation failure.
    pub fn responsibilities(
        &self,
        after: Option<&OrganizationDetailCursor>,
        inspected: u16,
        returned: u16,
        copied: u32,
    ) -> Result<OrganizationDetailPage, ApiError> {
        self.record.detail(
            OrganizationDetailParent::Genealogy(self.clone()),
            app::WorkspaceInspectionOrganizationSection::GenealogyResponsibilities,
            DetailLimits {
                after,
                inspected,
                returned,
                copied,
            },
            OrganizationDetailRows::GenealogyResponsibilities,
            |owner, after, i, r| self.application.responsibilities(owner, after, i, r),
            |entry, b| b.text(entry.value().as_str()),
        )
    }
    /// Actual projection members, not inferred native resources.
    /// # Errors
    /// Wrong nested parent/section/raw limits, copy budget or allocation failure.
    pub fn projections(
        &self,
        after: Option<&OrganizationDetailCursor>,
        inspected: u16,
        returned: u16,
        copied: u32,
    ) -> Result<OrganizationDetailPage, ApiError> {
        self.record.detail(
            OrganizationDetailParent::Genealogy(self.clone()),
            app::WorkspaceInspectionOrganizationSection::GenealogyProjections,
            DetailLimits {
                after,
                inspected,
                returned,
                copied,
            },
            OrganizationDetailRows::GenealogyProjections,
            |owner, after, i, r| self.application.projections(owner, after, i, r),
            |entry, b| b.text(entry.value().as_str()),
        )
    }
    /// Actual body-free occurrence members, not owner-native event bodies.
    /// # Errors
    /// Wrong nested parent/section/raw limits, copy budget or allocation failure.
    pub fn occurrences(
        &self,
        after: Option<&OrganizationDetailCursor>,
        inspected: u16,
        returned: u16,
        copied: u32,
    ) -> Result<OrganizationDetailPage, ApiError> {
        self.record.detail(
            OrganizationDetailParent::Genealogy(self.clone()),
            app::WorkspaceInspectionOrganizationSection::GenealogyOccurrences,
            DetailLimits {
                after,
                inspected,
                returned,
                copied,
            },
            OrganizationDetailRows::GenealogyOccurrences,
            |owner, after, i, r| self.application.occurrences(owner, after, i, r),
            |entry, b| b.text(entry.value().as_str()),
        )
    }
}

pub(super) fn lens_right(value: core::GroupLensRightV1) -> OrganizationLensRight {
    match value {
        core::GroupLensRightV1::Standards => OrganizationLensRight::Standards,
        core::GroupLensRightV1::Review => OrganizationLensRight::Review,
        core::GroupLensRightV1::BoundedBlock => OrganizationLensRight::BoundedBlock,
        core::GroupLensRightV1::Escalation => OrganizationLensRight::Escalation,
    }
}

pub(super) fn inspection_coordinate(
    value: core::InspectionCoordinateClassV1,
) -> OrganizationInspectionCoordinate {
    match value {
        core::InspectionCoordinateClassV1::ScopeOrganization => {
            OrganizationInspectionCoordinate::ScopeOrganization
        }
        core::InspectionCoordinateClassV1::GroupTree => OrganizationInspectionCoordinate::GroupTree,
        core::InspectionCoordinateClassV1::RoleBinding => {
            OrganizationInspectionCoordinate::RoleBinding
        }
        core::InspectionCoordinateClassV1::AtomicObject => {
            OrganizationInspectionCoordinate::AtomicObject
        }
        core::InspectionCoordinateClassV1::GroupLens => OrganizationInspectionCoordinate::GroupLens,
        core::InspectionCoordinateClassV1::DirectionalService => {
            OrganizationInspectionCoordinate::DirectionalService
        }
        core::InspectionCoordinateClassV1::AcceptanceFrontier => {
            OrganizationInspectionCoordinate::AcceptanceFrontier
        }
        core::InspectionCoordinateClassV1::NativeProjection => {
            OrganizationInspectionCoordinate::NativeProjection
        }
        core::InspectionCoordinateClassV1::EffectiveState => {
            OrganizationInspectionCoordinate::EffectiveState
        }
    }
}

pub(super) fn structural_kind(value: core::StructuralObjectKindV1) -> OrganizationStructuralKind {
    match value {
        core::StructuralObjectKindV1::LogicalOrganization => {
            OrganizationStructuralKind::LogicalOrganization
        }
        core::StructuralObjectKindV1::StableRole => OrganizationStructuralKind::StableRole,
        core::StructuralObjectKindV1::OccupantProvider => {
            OrganizationStructuralKind::OccupantProvider
        }
        core::StructuralObjectKindV1::DirectionalServiceContract => {
            OrganizationStructuralKind::DirectionalServiceContract
        }
        core::StructuralObjectKindV1::AccountableWorkProjection => {
            OrganizationStructuralKind::AccountableWorkProjection
        }
        core::StructuralObjectKindV1::CommunicationProjection => {
            OrganizationStructuralKind::CommunicationProjection
        }
        core::StructuralObjectKindV1::RelationshipProjection => {
            OrganizationStructuralKind::RelationshipProjection
        }
        core::StructuralObjectKindV1::NativeApplication => {
            OrganizationStructuralKind::NativeApplication
        }
        core::StructuralObjectKindV1::Package => OrganizationStructuralKind::Package,
        core::StructuralObjectKindV1::NormativePack => OrganizationStructuralKind::NormativePack,
        core::StructuralObjectKindV1::ReferenceOnly => OrganizationStructuralKind::ReferenceOnly,
    }
}

pub(super) fn residence_class(
    value: core::ResponsibilityResidenceClassV1,
) -> OrganizationResidenceClass {
    match value {
        core::ResponsibilityResidenceClassV1::Package => OrganizationResidenceClass::Package,
        core::ResponsibilityResidenceClassV1::NormativePack => {
            OrganizationResidenceClass::NormativePack
        }
        core::ResponsibilityResidenceClassV1::NativeApplication => {
            OrganizationResidenceClass::NativeApplication
        }
        core::ResponsibilityResidenceClassV1::Configuration => {
            OrganizationResidenceClass::Configuration
        }
        core::ResponsibilityResidenceClassV1::Reference => OrganizationResidenceClass::Reference,
    }
}
