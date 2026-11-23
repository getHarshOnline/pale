//! Direct bounded collection access on already-admitted organization records.

use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Section {
    AxisResources,
    LeadershipParticipants,
    LeadershipScopes,
    CommunicationResolutions,
    CommunicationClosure,
    LensStandards,
    LensConditions,
    LensRights,
    ServiceAssignments,
    AssignmentStewards,
    AssignmentPods,
    ServiceRights,
    ServiceControllers,
    ServiceAuthorities,
    NativeReferences,
    InspectionCoordinates,
    InspectionStates,
    ResponsibilitySelections,
    ResponsibilityResidences,
    ResponsibilityProjections,
    ProjectionReferences,
    ResponsibilityGenealogy,
    GenealogyResponsibilities,
    GenealogyProjections,
    GenealogyOccurrences,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Parent {
    Record,
    Communication(usize),
    Assignment(usize),
    Projection(usize),
    Genealogy(usize),
}

/// Exact record/section/member continuation; private positions are not semantic IDs.
///
/// Retains the inner semantic owner only. A caller retaining source access must
/// also retain the complete outer compilation. There is no parser/constructor.
#[derive(Clone)]
pub struct OrganizationMemberCursor {
    record: OrganizationRecordHandle,
    section: Section,
    parent: Parent,
    next: usize,
    inspected_limit: u16,
    returned_limit: u16,
}

impl std::fmt::Debug for OrganizationMemberCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrganizationMemberCursor")
            .finish_non_exhaustive()
    }
}

/// Borrowed cells from one specific existing record section, without cloning bodies.
///
/// No nested collection is expanded by visiting its parent cell. A parent with
/// an empty child set remains a real returned cell. Cell visits/returns are
/// bounded; retained values, nested bytes and later SDK copies are separate.
pub struct OrganizationMemberPage<'a, T> {
    record: OrganizationInspectionEntry<'a>,
    section: Section,
    start: usize,
    values: &'a [T],
    inspected: u16,
    continuation: Option<OrganizationMemberCursor>,
}

impl<'a, T> OrganizationMemberPage<'a, T> {
    /// Iterate the charged cells in their existing canonical collection order.
    pub fn entries(&self) -> impl ExactSizeIterator<Item = OrganizationMemberEntry<'a, T>> + '_ {
        self.values
            .iter()
            .enumerate()
            .map(|(index, value)| OrganizationMemberEntry {
                record: self.record,
                section: self.section,
                index: self.start + index,
                value,
            })
    }
    /// Actual direct member visits, not the nested members inside a returned body.
    #[must_use]
    pub const fn inspected(&self) -> u16 {
        self.inspected
    }
    /// None only after exhaustion, not merely because a nonempty page hit its cap.
    #[must_use]
    pub const fn continuation(&self) -> Option<&OrganizationMemberCursor> {
        self.continuation.as_ref()
    }
}

/// One borrowed member. Only the four genuinely nested record types issue handles.
pub struct OrganizationMemberEntry<'a, T> {
    record: OrganizationInspectionEntry<'a>,
    section: Section,
    index: usize,
    value: &'a T,
}

impl<'a, T> OrganizationMemberEntry<'a, T> {
    /// Borrow this actual owner value, never a relowered or inferred relationship.
    #[must_use]
    pub const fn value(&self) -> &'a T {
        self.value
    }
}

impl<'a> OrganizationInspectionEntry<'a> {
    fn member_page<T>(
        self,
        values: &'a [T],
        section: Section,
        parent: Parent,
        after: Option<&OrganizationMemberCursor>,
        inspected_limit: u16,
        returned_limit: u16,
    ) -> Result<OrganizationMemberPage<'a, T>, OrganizationInspectionError> {
        validate_limits(inspected_limit, returned_limit)?;
        if after.is_some_and(|cursor| {
            !Arc::ptr_eq(&cursor.record.owner, self.owner.organization_owner())
                || cursor.record.position != self.position
                || cursor.section != section
                || cursor.parent != parent
                || cursor.inspected_limit != inspected_limit
                || cursor.returned_limit != returned_limit
        }) {
            return Err(OrganizationInspectionError::ContinuationMismatch);
        }
        let start = after.map_or(0, |cursor| cursor.next);
        let remaining = values
            .len()
            .checked_sub(start)
            .ok_or(OrganizationInspectionError::InvalidPosition)?;
        let count = remaining.min(usize::from(inspected_limit.min(returned_limit)));
        let next = start + count;
        // Reaching the bound performs no additional exhaustion probe. When
        // exactly terminal, a later zero-visit page establishes that fact.
        let continuation = (count == usize::from(inspected_limit.min(returned_limit))).then(|| {
            OrganizationMemberCursor {
                record: self.retain(),
                section,
                parent,
                next,
                inspected_limit,
                returned_limit,
            }
        });
        Ok(OrganizationMemberPage {
            record: self,
            section,
            start,
            values: &values[start..next],
            inspected: u16::try_from(count)
                .map_err(|_| OrganizationInspectionError::InvalidPosition)?,
            continuation,
        })
    }

    /// Page an existing authority resource axis; other record classes are inapplicable.
    /// # Errors
    /// Invalid limits, inapplicable record, or different exact record/section cursor.
    pub fn axis_resources(
        &self,
        after: Option<&OrganizationMemberCursor>,
        inspected: u16,
        returned: u16,
    ) -> Result<OrganizationMemberPage<'a, WorkspaceResourceId>, OrganizationInspectionError> {
        validate_limits(inspected, returned)?;
        let axes = match self.record {
            OrganizationInspectionRecord::Scope(v) => &v.axes,
            OrganizationInspectionRecord::Organization(v) => &v.axes,
            OrganizationInspectionRecord::Group(v) => &v.axes,
            OrganizationInspectionRecord::DisciplineTree(v) => &v.axes,
            OrganizationInspectionRecord::Role(v) => &v.axes,
            OrganizationInspectionRecord::Object(v) => &v.axes,
            _ => return Err(OrganizationInspectionError::SectionUnavailable),
        };
        self.member_page(
            &axes.resources,
            Section::AxisResources,
            Parent::Record,
            after,
            inspected,
            returned,
        )
    }

    /// Return declared leadership participants without expanding their scope relationships.
    /// # Errors
    /// Invalid limits, inapplicable record or mismatched continuation.
    pub fn leadership_participants(
        &self,
        after: Option<&OrganizationMemberCursor>,
        inspected: u16,
        returned: u16,
    ) -> Result<OrganizationMemberPage<'a, OrganizationalRoleRefV1>, OrganizationInspectionError>
    {
        validate_limits(inspected, returned)?;
        let OrganizationInspectionRecord::Leadership(v) = self.record else {
            return Err(OrganizationInspectionError::SectionUnavailable);
        };
        self.member_page(
            &v.participants,
            Section::LeadershipParticipants,
            Parent::Record,
            after,
            inspected,
            returned,
        )
    }

    /// Actual affected scopes, independently of the participant set.
    /// # Errors
    /// Invalid limits, inapplicable record or mismatched continuation.
    pub fn leadership_scopes(
        &self,
        after: Option<&OrganizationMemberCursor>,
        inspected: u16,
        returned: u16,
    ) -> Result<OrganizationMemberPage<'a, LogicalScopeRefV1>, OrganizationInspectionError> {
        validate_limits(inspected, returned)?;
        let OrganizationInspectionRecord::Leadership(v) = self.record else {
            return Err(OrganizationInspectionError::SectionUnavailable);
        };
        self.member_page(
            &v.affected_scopes,
            Section::LeadershipScopes,
            Parent::Record,
            after,
            inspected,
            returned,
        )
    }

    /// Separately attributable local resolutions; equal nodes are not merged.
    /// # Errors
    /// Invalid limits, inapplicable record or mismatched continuation.
    pub fn communication_resolutions(
        &self,
        after: Option<&OrganizationMemberCursor>,
        inspected: u16,
        returned: u16,
    ) -> Result<
        OrganizationMemberPage<'a, CommunicationActorLocalResolutionV1>,
        OrganizationInspectionError,
    > {
        validate_limits(inspected, returned)?;
        let OrganizationInspectionRecord::CommunicationActor(v) = self.record else {
            return Err(OrganizationInspectionError::SectionUnavailable);
        };
        self.member_page(
            &v.local_resolutions,
            Section::CommunicationResolutions,
            Parent::Record,
            after,
            inspected,
            returned,
        )
    }

    /// Named standards, preserving the profile's actual bounded authority context.
    /// # Errors
    /// Invalid limits, inapplicable record or mismatched continuation.
    pub fn lens_standards(
        &self,
        after: Option<&OrganizationMemberCursor>,
        inspected: u16,
        returned: u16,
    ) -> Result<OrganizationMemberPage<'a, WorkspaceResourceId>, OrganizationInspectionError> {
        validate_limits(inspected, returned)?;
        let OrganizationInspectionRecord::GroupLens(v) = self.record else {
            return Err(OrganizationInspectionError::SectionUnavailable);
        };
        self.member_page(
            &v.standards,
            Section::LensStandards,
            Parent::Record,
            after,
            inspected,
            returned,
        )
    }

    /// Exact bounded-block conditions, not a new policy interpretation.
    /// # Errors
    /// Invalid limits, inapplicable record or mismatched continuation.
    pub fn lens_conditions(
        &self,
        after: Option<&OrganizationMemberCursor>,
        inspected: u16,
        returned: u16,
    ) -> Result<OrganizationMemberPage<'a, OrganizationalConditionId>, OrganizationInspectionError>
    {
        validate_limits(inspected, returned)?;
        let OrganizationInspectionRecord::GroupLens(v) = self.record else {
            return Err(OrganizationInspectionError::SectionUnavailable);
        };
        self.member_page(
            &v.block_conditions,
            Section::LensConditions,
            Parent::Record,
            after,
            inspected,
            returned,
        )
    }

    /// Actual declared lens rights, without conferring those rights on the caller.
    /// # Errors
    /// Invalid limits, inapplicable record or mismatched continuation.
    pub fn lens_rights(
        &self,
        after: Option<&OrganizationMemberCursor>,
        inspected: u16,
        returned: u16,
    ) -> Result<OrganizationMemberPage<'a, GroupLensRightV1>, OrganizationInspectionError> {
        validate_limits(inspected, returned)?;
        let OrganizationInspectionRecord::GroupLens(v) = self.record else {
            return Err(OrganizationInspectionError::SectionUnavailable);
        };
        self.member_page(
            &v.rights,
            Section::LensRights,
            Parent::Record,
            after,
            inspected,
            returned,
        )
    }

    /// Complete assignment cells, with nested members left borrowed and unexpanded.
    /// # Errors
    /// Invalid limits, inapplicable record or mismatched continuation.
    pub fn service_assignments(
        &self,
        after: Option<&OrganizationMemberCursor>,
        inspected: u16,
        returned: u16,
    ) -> Result<OrganizationMemberPage<'a, ServiceAssignmentV1>, OrganizationInspectionError> {
        validate_limits(inspected, returned)?;
        let OrganizationInspectionRecord::Service(v) = self.record else {
            return Err(OrganizationInspectionError::SectionUnavailable);
        };
        self.member_page(
            &v.assignments,
            Section::ServiceAssignments,
            Parent::Record,
            after,
            inspected,
            returned,
        )
    }

    /// Existing declared use-right members, without computing another ceiling meet.
    /// # Errors
    /// Invalid limits, inapplicable record or mismatched continuation.
    pub fn service_rights(
        &self,
        after: Option<&OrganizationMemberCursor>,
        inspected: u16,
        returned: u16,
    ) -> Result<OrganizationMemberPage<'a, WorkspaceRightId>, OrganizationInspectionError> {
        validate_limits(inspected, returned)?;
        let OrganizationInspectionRecord::Service(v) = self.record else {
            return Err(OrganizationInspectionError::SectionUnavailable);
        };
        self.member_page(
            &v.ceiling.rights.permitted,
            Section::ServiceRights,
            Parent::Record,
            after,
            inspected,
            returned,
        )
    }

    /// Existing declared controller members, not controller authority proof.
    /// # Errors
    /// Invalid limits, inapplicable record or mismatched continuation.
    pub fn service_controllers(
        &self,
        after: Option<&OrganizationMemberCursor>,
        inspected: u16,
        returned: u16,
    ) -> Result<OrganizationMemberPage<'a, WorkspaceControllerScopeId>, OrganizationInspectionError>
    {
        validate_limits(inspected, returned)?;
        let OrganizationInspectionRecord::Service(v) = self.record else {
            return Err(OrganizationInspectionError::SectionUnavailable);
        };
        self.member_page(
            &v.ceiling.controllers.permitted,
            Section::ServiceControllers,
            Parent::Record,
            after,
            inspected,
            returned,
        )
    }

    /// Existing declared authority members, independently from controllers and rights.
    /// # Errors
    /// Invalid limits, inapplicable record or mismatched continuation.
    pub fn service_authorities(
        &self,
        after: Option<&OrganizationMemberCursor>,
        inspected: u16,
        returned: u16,
    ) -> Result<OrganizationMemberPage<'a, WorkspaceAuthorityScopeId>, OrganizationInspectionError>
    {
        validate_limits(inspected, returned)?;
        let OrganizationInspectionRecord::Service(v) = self.record else {
            return Err(OrganizationInspectionError::SectionUnavailable);
        };
        self.member_page(
            &v.ceiling.authorities.permitted,
            Section::ServiceAuthorities,
            Parent::Record,
            after,
            inspected,
            returned,
        )
    }

    /// Exact organizational native references, including an actual empty set.
    /// # Errors
    /// Invalid limits, inapplicable record or mismatched continuation.
    pub fn native_references(
        &self,
        after: Option<&OrganizationMemberCursor>,
        inspected: u16,
        returned: u16,
    ) -> Result<OrganizationMemberPage<'a, WorkspaceResourceId>, OrganizationInspectionError> {
        validate_limits(inspected, returned)?;
        let OrganizationInspectionRecord::NativeProjection(v) = self.record else {
            return Err(OrganizationInspectionError::SectionUnavailable);
        };
        self.member_page(
            &v.native_refs,
            Section::NativeReferences,
            Parent::Record,
            after,
            inspected,
            returned,
        )
    }

    /// Exact configured inspection coordinates; no authenticated view is implied.
    /// # Errors
    /// Invalid limits, inapplicable record or mismatched continuation.
    pub fn inspection_coordinates(
        &self,
        after: Option<&OrganizationMemberCursor>,
        inspected: u16,
        returned: u16,
    ) -> Result<OrganizationMemberPage<'a, InspectionCoordinateClassV1>, OrganizationInspectionError>
    {
        validate_limits(inspected, returned)?;
        let OrganizationInspectionRecord::Inspection(v) = self.record else {
            return Err(OrganizationInspectionError::SectionUnavailable);
        };
        self.member_page(
            &v.coordinates,
            Section::InspectionCoordinates,
            Parent::Record,
            after,
            inspected,
            returned,
        )
    }

    /// Exact configured state classes, independent of actual row state.
    /// # Errors
    /// Invalid limits, inapplicable record or mismatched continuation.
    pub fn inspection_states(
        &self,
        after: Option<&OrganizationMemberCursor>,
        inspected: u16,
        returned: u16,
    ) -> Result<OrganizationMemberPage<'a, EffectiveFactStateV1>, OrganizationInspectionError> {
        validate_limits(inspected, returned)?;
        let OrganizationInspectionRecord::Inspection(v) = self.record else {
            return Err(OrganizationInspectionError::SectionUnavailable);
        };
        self.member_page(
            &v.states,
            Section::InspectionStates,
            Parent::Record,
            after,
            inspected,
            returned,
        )
    }

    /// Existing attributed structural answers, without interpreting resource strings.
    /// # Errors
    /// Invalid limits, inapplicable record or mismatched continuation.
    pub fn responsibility_selections(
        &self,
        after: Option<&OrganizationMemberCursor>,
        inspected: u16,
        returned: u16,
    ) -> Result<
        OrganizationMemberPage<'a, ResponsibilityStructuralSelectionV1>,
        OrganizationInspectionError,
    > {
        validate_limits(inspected, returned)?;
        let OrganizationInspectionRecord::Responsibility(v) = self.record else {
            return Err(OrganizationInspectionError::SectionUnavailable);
        };
        self.member_page(
            &v.structural_objects,
            Section::ResponsibilitySelections,
            Parent::Record,
            after,
            inspected,
            returned,
        )
    }

    /// Existing residence decisions with exact class/resource/owner association.
    /// # Errors
    /// Invalid limits, inapplicable record or mismatched continuation.
    pub fn responsibility_residences(
        &self,
        after: Option<&OrganizationMemberCursor>,
        inspected: u16,
        returned: u16,
    ) -> Result<OrganizationMemberPage<'a, ResponsibilityResidenceV1>, OrganizationInspectionError>
    {
        validate_limits(inspected, returned)?;
        let OrganizationInspectionRecord::Responsibility(v) = self.record else {
            return Err(OrganizationInspectionError::SectionUnavailable);
        };
        self.member_page(
            &v.residence,
            Section::ResponsibilityResidences,
            Parent::Record,
            after,
            inspected,
            returned,
        )
    }

    /// Sparse native projection cells, retaining every independently queryable member.
    /// # Errors
    /// Invalid limits, inapplicable record or mismatched continuation.
    pub fn responsibility_projections(
        &self,
        after: Option<&OrganizationMemberCursor>,
        inspected: u16,
        returned: u16,
    ) -> Result<
        OrganizationMemberPage<'a, ResponsibilityNativeProjectionV1>,
        OrganizationInspectionError,
    > {
        validate_limits(inspected, returned)?;
        let OrganizationInspectionRecord::Responsibility(v) = self.record else {
            return Err(OrganizationInspectionError::SectionUnavailable);
        };
        self.member_page(
            &v.native_projections,
            Section::ResponsibilityProjections,
            Parent::Record,
            after,
            inspected,
            returned,
        )
    }

    /// Body-free genealogy link cells; different identity classes remain separate sets.
    /// # Errors
    /// Invalid limits, inapplicable record or mismatched continuation.
    pub fn responsibility_genealogy(
        &self,
        after: Option<&OrganizationMemberCursor>,
        inspected: u16,
        returned: u16,
    ) -> Result<
        OrganizationMemberPage<'a, ResponsibilityGenealogyLinkV1>,
        OrganizationInspectionError,
    > {
        validate_limits(inspected, returned)?;
        let OrganizationInspectionRecord::Responsibility(v) = self.record else {
            return Err(OrganizationInspectionError::SectionUnavailable);
        };
        self.member_page(
            &v.genealogy_links,
            Section::ResponsibilityGenealogy,
            Parent::Record,
            after,
            inspected,
            returned,
        )
    }
}

/// Exact local-resolution member, not a lookup by possibly repeated node value.
#[derive(Clone)]
pub struct OrganizationCommunicationResolutionHandle {
    record: OrganizationRecordHandle,
    index: usize,
}
/// Exact service assignment member; account/mandate values do not mint this token.
#[derive(Clone)]
pub struct OrganizationServiceAssignmentHandle {
    record: OrganizationRecordHandle,
    index: usize,
}
/// Exact responsibility native-projection member.
#[derive(Clone)]
pub struct OrganizationResponsibilityProjectionHandle {
    record: OrganizationRecordHandle,
    index: usize,
}
/// Exact responsibility genealogy-link member.
#[derive(Clone)]
pub struct OrganizationGenealogyHandle {
    record: OrganizationRecordHandle,
    index: usize,
}

impl std::fmt::Debug for OrganizationCommunicationResolutionHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrganizationCommunicationResolutionHandle")
            .finish_non_exhaustive()
    }
}
impl std::fmt::Debug for OrganizationServiceAssignmentHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrganizationServiceAssignmentHandle")
            .finish_non_exhaustive()
    }
}
impl std::fmt::Debug for OrganizationResponsibilityProjectionHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrganizationResponsibilityProjectionHandle")
            .finish_non_exhaustive()
    }
}
impl std::fmt::Debug for OrganizationGenealogyHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrganizationGenealogyHandle")
            .finish_non_exhaustive()
    }
}

impl OrganizationMemberEntry<'_, CommunicationActorLocalResolutionV1> {
    /// Retain this actual resolution's position without copying its closure.
    #[must_use]
    pub fn retain(&self) -> OrganizationCommunicationResolutionHandle {
        debug_assert_eq!(self.section, Section::CommunicationResolutions);
        OrganizationCommunicationResolutionHandle {
            record: self.record.retain(),
            index: self.index,
        }
    }
}
impl OrganizationMemberEntry<'_, ServiceAssignmentV1> {
    /// Retain this actual assignment without copying its stewards or pods.
    #[must_use]
    pub fn retain(&self) -> OrganizationServiceAssignmentHandle {
        debug_assert_eq!(self.section, Section::ServiceAssignments);
        OrganizationServiceAssignmentHandle {
            record: self.record.retain(),
            index: self.index,
        }
    }
}
impl OrganizationMemberEntry<'_, ResponsibilityNativeProjectionV1> {
    /// Retain this actual projection without copying native references.
    #[must_use]
    pub fn retain(&self) -> OrganizationResponsibilityProjectionHandle {
        debug_assert_eq!(self.section, Section::ResponsibilityProjections);
        OrganizationResponsibilityProjectionHandle {
            record: self.record.retain(),
            index: self.index,
        }
    }
}
impl OrganizationMemberEntry<'_, ResponsibilityGenealogyLinkV1> {
    /// Retain this actual link without copying any of its three member sets.
    #[must_use]
    pub fn retain(&self) -> OrganizationGenealogyHandle {
        debug_assert_eq!(self.section, Section::ResponsibilityGenealogy);
        OrganizationGenealogyHandle {
            record: self.record.retain(),
            index: self.index,
        }
    }
}

impl OrganizationCommunicationResolutionHandle {
    /// Page this resolution's actual authorized closure; equal nodes do not merge it.
    /// # Errors
    /// Invalid limits/owner/parent/section or an impossible retained member index.
    pub fn authorized_closure<'a>(
        &self,
        owner: &'a PortableWorkspaceUniverse,
        after: Option<&OrganizationMemberCursor>,
        inspected: u16,
        returned: u16,
    ) -> Result<OrganizationMemberPage<'a, OrganizationalNodeRefV1>, OrganizationInspectionError>
    {
        validate_limits(inspected, returned)?;
        let entry = owner.resolve_organization_record(&self.record)?;
        let OrganizationInspectionRecord::CommunicationActor(v) = entry.record else {
            return Err(OrganizationInspectionError::InvalidPosition);
        };
        let member = v
            .local_resolutions
            .get(self.index)
            .ok_or(OrganizationInspectionError::InvalidPosition)?;
        entry.member_page(
            &member.authorized_closure,
            Section::CommunicationClosure,
            Parent::Communication(self.index),
            after,
            inspected,
            returned,
        )
    }
}

impl OrganizationServiceAssignmentHandle {
    fn resolve<'a>(
        &self,
        owner: &'a PortableWorkspaceUniverse,
    ) -> Result<
        (OrganizationInspectionEntry<'a>, &'a ServiceAssignmentV1),
        OrganizationInspectionError,
    > {
        let entry = owner.resolve_organization_record(&self.record)?;
        let OrganizationInspectionRecord::Service(v) = entry.record else {
            return Err(OrganizationInspectionError::InvalidPosition);
        };
        let member = v
            .assignments
            .get(self.index)
            .ok_or(OrganizationInspectionError::InvalidPosition)?;
        Ok((entry, member))
    }
    /// Exact steward members of this assignment, none primary by position.
    /// # Errors
    /// Invalid limits/owner/parent/section or an impossible retained member index.
    pub fn stewards<'a>(
        &self,
        owner: &'a PortableWorkspaceUniverse,
        after: Option<&OrganizationMemberCursor>,
        inspected: u16,
        returned: u16,
    ) -> Result<OrganizationMemberPage<'a, ServiceStewardId>, OrganizationInspectionError> {
        validate_limits(inspected, returned)?;
        let (entry, member) = self.resolve(owner)?;
        entry.member_page(
            &member.stewards,
            Section::AssignmentStewards,
            Parent::Assignment(self.index),
            after,
            inspected,
            returned,
        )
    }
    /// Exact pod members of this assignment, separately from stewards.
    /// # Errors
    /// Invalid limits/owner/parent/section or an impossible retained member index.
    pub fn pods<'a>(
        &self,
        owner: &'a PortableWorkspaceUniverse,
        after: Option<&OrganizationMemberCursor>,
        inspected: u16,
        returned: u16,
    ) -> Result<OrganizationMemberPage<'a, ServicePodId>, OrganizationInspectionError> {
        validate_limits(inspected, returned)?;
        let (entry, member) = self.resolve(owner)?;
        entry.member_page(
            &member.pods,
            Section::AssignmentPods,
            Parent::Assignment(self.index),
            after,
            inspected,
            returned,
        )
    }
}

impl OrganizationResponsibilityProjectionHandle {
    /// Actual native references for this exact responsibility projection.
    /// # Errors
    /// Invalid limits/owner/parent/section or an impossible retained member index.
    pub fn native_references<'a>(
        &self,
        owner: &'a PortableWorkspaceUniverse,
        after: Option<&OrganizationMemberCursor>,
        inspected: u16,
        returned: u16,
    ) -> Result<OrganizationMemberPage<'a, WorkspaceResourceId>, OrganizationInspectionError> {
        validate_limits(inspected, returned)?;
        let entry = owner.resolve_organization_record(&self.record)?;
        let OrganizationInspectionRecord::Responsibility(v) = entry.record else {
            return Err(OrganizationInspectionError::InvalidPosition);
        };
        let member = v
            .native_projections
            .get(self.index)
            .ok_or(OrganizationInspectionError::InvalidPosition)?;
        entry.member_page(
            &member.native_refs,
            Section::ProjectionReferences,
            Parent::Projection(self.index),
            after,
            inspected,
            returned,
        )
    }
}

impl OrganizationGenealogyHandle {
    fn resolve<'a>(
        &self,
        owner: &'a PortableWorkspaceUniverse,
    ) -> Result<
        (
            OrganizationInspectionEntry<'a>,
            &'a ResponsibilityGenealogyLinkV1,
        ),
        OrganizationInspectionError,
    > {
        let entry = owner.resolve_organization_record(&self.record)?;
        let OrganizationInspectionRecord::Responsibility(v) = entry.record else {
            return Err(OrganizationInspectionError::InvalidPosition);
        };
        let member = v
            .genealogy_links
            .get(self.index)
            .ok_or(OrganizationInspectionError::InvalidPosition)?;
        Ok((entry, member))
    }
    /// Exact responsibility members, not projection or occurrence identities.
    /// # Errors
    /// Invalid limits/owner/parent/section or an impossible retained member index.
    pub fn responsibilities<'a>(
        &self,
        owner: &'a PortableWorkspaceUniverse,
        after: Option<&OrganizationMemberCursor>,
        inspected: u16,
        returned: u16,
    ) -> Result<OrganizationMemberPage<'a, ResponsibilityDerivationId>, OrganizationInspectionError>
    {
        validate_limits(inspected, returned)?;
        let (entry, member) = self.resolve(owner)?;
        entry.member_page(
            &member.responsibilities,
            Section::GenealogyResponsibilities,
            Parent::Genealogy(self.index),
            after,
            inspected,
            returned,
        )
    }
    /// Exact projection members, separately from responsibility and occurrence sets.
    /// # Errors
    /// Invalid limits/owner/parent/section or an impossible retained member index.
    pub fn projections<'a>(
        &self,
        owner: &'a PortableWorkspaceUniverse,
        after: Option<&OrganizationMemberCursor>,
        inspected: u16,
        returned: u16,
    ) -> Result<OrganizationMemberPage<'a, ResponsibilityProjectionId>, OrganizationInspectionError>
    {
        validate_limits(inspected, returned)?;
        let (entry, member) = self.resolve(owner)?;
        entry.member_page(
            &member.projections,
            Section::GenealogyProjections,
            Parent::Genealogy(self.index),
            after,
            inspected,
            returned,
        )
    }
    /// Body-free occurrence identities, not owner-native occurrence bodies.
    /// # Errors
    /// Invalid limits/owner/parent/section or an impossible retained member index.
    pub fn occurrences<'a>(
        &self,
        owner: &'a PortableWorkspaceUniverse,
        after: Option<&OrganizationMemberCursor>,
        inspected: u16,
        returned: u16,
    ) -> Result<OrganizationMemberPage<'a, ResponsibilityOccurrenceId>, OrganizationInspectionError>
    {
        validate_limits(inspected, returned)?;
        let (entry, member) = self.resolve(owner)?;
        entry.member_page(
            &member.occurrences,
            Section::GenealogyOccurrences,
            Parent::Genealogy(self.index),
            after,
            inspected,
            returned,
        )
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/workspace/organization_inspection/details.rs"]
mod tests;
