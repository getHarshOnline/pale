//! Canonical record keys shared by admission and retained-record correspondence.

use pale_core::records::*;

pub(crate) fn scope(value: &LogicalScopeV1) -> &LogicalScopeRefV1 {
    &value.scope
}
pub(crate) fn organization(value: &LogicalOrganizationV1) -> &LogicalOrganizationRefV1 {
    &value.organization
}
pub(crate) fn group(value: &OrganizationalGroupV1) -> &OrganizationalGroupRefV1 {
    &value.group
}
pub(crate) fn tree(value: &DisciplineTreeV1) -> &DisciplineTreeRefV1 {
    &value.tree
}
pub(crate) fn role(value: &OrganizationalRoleV1) -> &OrganizationalRoleRefV1 {
    &value.role
}
pub(crate) fn occupant(
    value: &OrganizationalOccupantBindingV1,
) -> (
    &OrganizationalRoleRefV1,
    &OrganizationalOccupantId,
    &EffectiveIntervalV1,
) {
    (&value.role, &value.occupant, &value.interval)
}
pub(crate) fn provider(
    value: &OrganizationalProviderBindingV1,
) -> (
    &OrganizationalRoleRefV1,
    &OrganizationalProviderId,
    &EffectiveIntervalV1,
) {
    (&value.role, &value.provider, &value.interval)
}
pub(crate) fn communication(
    value: &CommunicationActorBindingV1,
) -> (&OrganizationalProviderId, &CommunicationActorHandleId) {
    (&value.provider, &value.handle)
}
pub(crate) fn leadership(value: &LeadershipParticipationV1) -> &LeadershipTopicId {
    &value.topic
}
pub(crate) fn relation(value: &OrganizationalRelationV1) -> &OrganizationalRelationId {
    &value.relation_id
}
pub(crate) fn object(value: &OrganizationalObjectV1) -> &OrganizationalObjectId {
    &value.object_id
}
pub(crate) fn population(
    value: &ObjectRolePopulationRuleV1,
) -> (&OrganizationalObjectClassId, &OrganizationalRoleClassId) {
    (&value.object_class, &value.accountable_role_class)
}
pub(crate) fn lens(value: &GroupLensAuthorityProfileV1) -> &GroupLensProfileId {
    &value.profile_id
}
pub(crate) fn projection(value: &OrganizationalNativeProjectionV1) -> &OrganizationalProjectionId {
    &value.projection_id
}
/// The configured singleton has an ID, but no collection deduplication rule.
pub(crate) fn inspection(value: &OrganizationalInspectionV1) -> &OrganizationalInspectionId {
    &value.inspection_id
}

/// The source, relation class, directed endpoints and interval identify a derived edge.
/// Issuer and effective state remain compared values, not a second edge identity.
pub(crate) fn derived(
    value: &OrganizationalDerivedRelationV1,
) -> (
    &OrganizationalDerivedRelationSourceV1,
    OrganizationalDerivedRelationClassV1,
    &OrganizationalNodeRefV1,
    &OrganizationalNodeRefV1,
    &EffectiveIntervalV1,
) {
    (
        &value.source,
        value.class,
        &value.from,
        &value.to,
        &value.interval,
    )
}
