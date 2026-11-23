//! Candidate-only identifier denial for responsibility and organization records.

use pale_core::records::*;

use super::ResponsibilityDerivationError;

/// One typed initial-v1 record whose machine identities are admission surfaces.
pub(crate) enum CandidateIdentityRecordV1<'a> {
    /// Responsibility derivation.
    Responsibility(&'a ResponsibilityDerivationV1),
    /// Organizational translation.
    Organization(&'a OrganizationalTranslationV1),
}

/// Reject candidate-only vocabulary from every identity-bearing field.
///
/// The traversal follows typed fields only. It does not inspect prose,
/// serialized bytes, filenames, or enum labels.
///
/// # Errors
///
/// Returns [`ResponsibilityDerivationError::CandidateOnlySurface`] for the
/// first machine identity containing a reserved candidate token.
pub(crate) fn reject_candidate_only_record_identifiers(
    record: CandidateIdentityRecordV1<'_>,
) -> Result<(), ResponsibilityDerivationError> {
    match record {
        CandidateIdentityRecordV1::Responsibility(record) => responsibility(record),
        CandidateIdentityRecordV1::Organization(record) => organization(record),
    }
}

fn identity(value: &str) -> Result<(), ResponsibilityDerivationError> {
    if let Some(token) = candidate_only_workspace_language_identifier_token_v1(value) {
        return Err(ResponsibilityDerivationError::CandidateOnlySurface {
            surface: value.to_owned(),
            token,
        });
    }
    Ok(())
}

fn id<T: AsRef<str>>(value: &T) -> Result<(), ResponsibilityDerivationError> {
    identity(value.as_ref())
}

fn scope(value: &LogicalScopeRefV1) -> Result<(), ResponsibilityDerivationError> {
    id(&value.namespace)?;
    id(&value.scope_id)
}

fn organization_ref(value: &LogicalOrganizationRefV1) -> Result<(), ResponsibilityDerivationError> {
    id(&value.namespace)?;
    id(&value.organization_id)
}

fn group(value: &OrganizationalGroupRefV1) -> Result<(), ResponsibilityDerivationError> {
    id(&value.namespace)?;
    id(&value.group_id)
}

fn tree(value: &DisciplineTreeRefV1) -> Result<(), ResponsibilityDerivationError> {
    id(&value.namespace)?;
    id(&value.tree_id)
}

fn role(value: &OrganizationalRoleRefV1) -> Result<(), ResponsibilityDerivationError> {
    id(&value.namespace)?;
    id(&value.role_id)
}

fn service(value: &DirectionalServiceContractRefV1) -> Result<(), ResponsibilityDerivationError> {
    id(&value.namespace)?;
    id(&value.contract_id)?;
    scope(&value.provider_scope)?;
    scope(&value.consumer_scope)
}

fn axes(value: &OrganizationalAuthorityAxesV1) -> Result<(), ResponsibilityDerivationError> {
    id(&value.owner)?;
    id(&value.controller)?;
    id(&value.authority)?;
    value.resources.iter().try_for_each(id)
}

fn policy(value: &WorkspacePolicyCeilingV1) -> Result<(), ResponsibilityDerivationError> {
    value.rights.permitted.iter().try_for_each(id)?;
    value.controllers.permitted.iter().try_for_each(id)?;
    value.authorities.permitted.iter().try_for_each(id)
}

fn evidence(
    value: &ResponsibilityEvidenceRequirementV1,
) -> Result<(), ResponsibilityDerivationError> {
    id(&value.owner)?;
    id(&value.evidence_ref)
}

fn attribution(value: &ResponsibilityAttributionV1) -> Result<(), ResponsibilityDerivationError> {
    id(&value.source_id)?;
    id(&value.owner)?;
    id(&value.evidence_ref)
}

fn answer<T>(
    value: &AttributableAnswerV1<T>,
    inspect: impl FnOnce(&T) -> Result<(), ResponsibilityDerivationError>,
) -> Result<(), ResponsibilityDerivationError> {
    match value {
        AttributableAnswerV1::Supplied {
            value,
            attribution: source,
        } => {
            attribution(source)?;
            inspect(value)
        }
        AttributableAnswerV1::Omitted {
            reason,
            attribution: source,
        } => {
            attribution(source)?;
            id(reason)
        }
    }
}

fn node(value: &OrganizationalNodeRefV1) -> Result<(), ResponsibilityDerivationError> {
    match value {
        OrganizationalNodeRefV1::Scope { scope: value } => scope(value),
        OrganizationalNodeRefV1::Organization {
            organization: value,
        } => organization_ref(value),
        OrganizationalNodeRefV1::Group { group: value } => group(value),
        OrganizationalNodeRefV1::DisciplineTree { tree: value } => tree(value),
        OrganizationalNodeRefV1::Role { role: value } => role(value),
        OrganizationalNodeRefV1::Occupant { occupant } => id(occupant),
        OrganizationalNodeRefV1::Provider { provider } => id(provider),
        OrganizationalNodeRefV1::NativeResource { resource } => id(resource),
        OrganizationalNodeRefV1::Object { object } => id(object),
    }
}

fn responsibility(
    record: &ResponsibilityDerivationV1,
) -> Result<(), ResponsibilityDerivationError> {
    identity(&record.schema)?;
    id(&record.derivation_id)?;
    attribution(&record.provenance)?;
    answer(&record.responsibility, |value| {
        id(&value.verb)?;
        id(&value.object)?;
        id(&value.outcome)
    })?;
    answer(&record.native_owner, |value| {
        id(&value.owner)?;
        id(&value.authority_resolver)
    })?;
    answer(&record.trigger, |value| match value {
        ResponsibilityTriggerV1::RecurringCadence { cadence } => id(cadence),
        ResponsibilityTriggerV1::OwnerEvent { event }
        | ResponsibilityTriggerV1::NativeEvent { event } => id(event),
        ResponsibilityTriggerV1::BoundedRequest { request_contract } => id(request_contract),
    })?;
    answer(&record.placement, |value| match value {
        ResponsibilityPlacementV1::VerticalLocal { scope } => id(scope),
        ResponsibilityPlacementV1::DirectionalService { service_contract } => {
            service(service_contract)
        }
    })?;
    for selection in &record.structural_objects {
        answer(&selection.selection, id)?;
    }
    answer(&record.request_framer, |value| {
        id(&value.actor)?;
        id(&value.owner)?;
        id(&value.authority_scope)
    })?;
    answer(&record.provider_completion, |value| {
        evidence(&value.requirement)
    })?;
    answer(&record.provider_acceptance, |value| {
        evidence(&value.requirement)
    })?;
    answer(&record.consumer_acceptance, |value| {
        evidence(&value.requirement)
    })?;
    answer(&record.native_result, |value| evidence(&value.requirement))?;
    answer(&record.pale_settlement, |value| {
        evidence(&value.requirement)
    })?;
    answer(&record.lawful_silence, |_| Ok(()))?;
    answer(&record.publisher_module, |value| {
        id(&value.module_id)?;
        id(&value.release_id)?;
        id(&value.export_id)?;
        id(&value.semantic_owner)
    })?;
    answer(&record.adopter_module, |value| {
        id(&value.module_id)?;
        id(&value.imported_module_id)?;
        id(&value.imported_release_id)?;
        id(&value.imported_export_id)?;
        id(&value.local_namespace)?;
        id(&value.adoption_owner)
    })?;
    answer(&record.ceilings, |value| {
        policy(&value.policy)?;
        value.resources.iter().try_for_each(id)
    })?;
    answer(&record.incumbent, |value| {
        id(&value.mechanism)?;
        id(&value.writer)?;
        id(&value.retained_evidence)
    })?;
    answer(&record.readiness, |value| {
        id(&value.operation_contract)?;
        id(&value.replay_proof)?;
        id(&value.failure_proof)?;
        id(&value.recovery_proof)?;
        id(&value.rollback_proof)?;
        id(&value.native_read_back_proof)
    })?;
    answer(&record.safe_failure, |_| Ok(()))?;
    answer(&record.direct_path, |value| {
        id(&value.path)?;
        id(&value.native_owner)?;
        id(&value.availability_policy)
    })?;
    for residence in &record.residence {
        id(&residence.resource)?;
        id(&residence.owner)?;
    }
    for projection in &record.native_projections {
        id(&projection.projection_id)?;
        id(&projection.native_owner)?;
        projection.native_refs.iter().try_for_each(id)?;
    }
    for link in &record.genealogy_links {
        id(&link.link_id)?;
        link.responsibilities.iter().try_for_each(id)?;
        link.projections.iter().try_for_each(id)?;
        link.occurrences.iter().try_for_each(id)?;
    }
    Ok(())
}

fn organization(record: &OrganizationalTranslationV1) -> Result<(), ResponsibilityDerivationError> {
    identity(&record.schema)?;
    for value in &record.scopes {
        scope(&value.scope)?;
        axes(&value.axes)?;
    }
    for value in &record.organizations {
        organization_ref(&value.organization)?;
        scope(&value.scope)?;
        axes(&value.axes)?;
    }
    for value in &record.groups {
        group(&value.group)?;
        organization_ref(&value.organization)?;
        axes(&value.axes)?;
    }
    for value in &record.discipline_trees {
        tree(&value.tree)?;
        organization_ref(&value.organization)?;
        axes(&value.axes)?;
    }
    for record_role in &record.roles {
        role(&record_role.role)?;
        organization_ref(&record_role.organization)?;
        if let Some(value) = &record_role.group {
            group(value)?;
        }
        if let Some(value) = &record_role.discipline_tree {
            tree(value)?;
        }
        id(&record_role.role_class)?;
        axes(&record_role.axes)?;
    }
    for value in &record.occupant_bindings {
        role(&value.role)?;
        id(&value.occupant)?;
        id(&value.issuer)?;
    }
    for value in &record.provider_bindings {
        role(&value.role)?;
        id(&value.provider)?;
        id(&value.issuer)?;
    }
    for value in &record.communication_actor_bindings {
        id(&value.provider)?;
        id(&value.handle)?;
        for resolution in &value.local_resolutions {
            node(&resolution.local_node)?;
            resolution.authorized_closure.iter().try_for_each(node)?;
        }
        id(&value.issuer)?;
    }
    for value in &record.leadership {
        id(&value.topic)?;
        value.participants.iter().try_for_each(role)?;
        value.affected_scopes.iter().try_for_each(scope)?;
        id(&value.issuer)?;
    }
    for value in &record.relations {
        id(&value.relation_id)?;
        node(&value.from)?;
        node(&value.to)?;
        id(&value.issuer)?;
    }
    for value in &record.objects {
        id(&value.object_id)?;
        id(&value.namespace)?;
        scope(&value.scope)?;
        organization_ref(&value.organization)?;
        id(&value.object_class)?;
        axes(&value.axes)?;
    }
    for value in &record.population_rules {
        id(&value.object_class)?;
        id(&value.accountable_role_class)?;
        id(&value.issuer)?;
    }
    for value in &record.group_lenses {
        id(&value.profile_id)?;
        role(&value.holder)?;
        id(&value.alignment_relation)?;
        id(&value.lens)?;
        if let Some(value) = &value.review_owner {
            role(value)?;
        }
        value.standards.iter().try_for_each(id)?;
        value.block_conditions.iter().try_for_each(id)?;
        if let Some(value) = &value.escalation_resolver {
            role(value)?;
        }
        id(&value.issuer)?;
    }
    for value in &record.services {
        service(&value.contract_ref())?;
        id(&value.contract_owner)?;
        evidence(&value.provider_completion.requirement)?;
        evidence(&value.provider_acceptance.requirement)?;
        evidence(&value.consumer_acceptance.requirement)?;
        evidence(&value.native_read_back.requirement)?;
        evidence(&value.pale_settlement.requirement)?;
        for assignment in &value.assignments {
            id(&assignment.account)?;
            id(&assignment.mandate)?;
            assignment.stewards.iter().try_for_each(id)?;
            assignment.pods.iter().try_for_each(id)?;
        }
        policy(&value.ceiling)?;
    }
    for value in &record.native_projections {
        id(&value.projection_id)?;
        node(&value.subject)?;
        id(&value.native_owner)?;
        value.native_refs.iter().try_for_each(id)?;
    }
    if let Some(value) = &record.inspection {
        id(&value.inspection_id)?;
        id(&value.principal)?;
        if let Some(value) = &value.view_provider {
            id(value)?;
        }
    }
    Ok(())
}
