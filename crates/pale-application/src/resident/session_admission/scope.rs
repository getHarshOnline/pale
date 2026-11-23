//! Generation, scope, capability, and route policy for ambient admission.

use super::*;
use pale_core::records::CapabilityIdV1;

/// Classify project, workspace, generation, head, and activation-fence drift.
///
/// # Errors
///
/// Returns the corresponding [`AmbientSessionAdmissionError`] before core
/// contextual admission performs the complete profile, source, and path join.
pub(super) fn classify_generation(
    wire: &SessionAdmissionLeaseWireV1,
    context: &AmbientSessionAdmissionContext<'_>,
) -> Result<(), AmbientSessionAdmissionError> {
    if wire.project_id() != &context.generation.effective_profile.project_id
        || wire.execution_workspace_id()
            != &context.generation.effective_profile.execution_workspace_id
    {
        return Err(AmbientSessionAdmissionError::WrongScope);
    }
    if wire.compiled_generation_digest()
        != context
            .generation
            .compiled_generation
            .compiled_generation_digest()
        || wire.compiled_generation_digest()
            != context.generation.active_head.compiled_generation_digest()
    {
        return Err(AmbientSessionAdmissionError::StaleGeneration);
    }
    if wire.active_generation_head_digest()
        != context
            .generation
            .active_head
            .active_generation_head_digest()
        || wire.activation_fence_digest()
            != context
                .generation
                .activation_fence
                .activation_fence_digest()
    {
        return Err(AmbientSessionAdmissionError::StaleActivation);
    }
    Ok(())
}

/// Prove that deployment scope remains within the selected grant ceiling.
///
/// Opaque data-classification names are ordered only by
/// [`AmbientSessionAuthorityPort::prove_data_classification_ceiling`].
///
/// # Errors
///
/// Returns [`AmbientSessionAdmissionError::WrongScope`] for a structurally
/// wider scope, or preserves the authority port's exact classification error.
pub(super) fn validate_scope<P: AmbientSessionAuthorityPort>(
    lease: &SessionAdmissionLeaseV1,
    grant: &CapabilitySessionGrantV1,
    binding: &DeploymentBindingV1,
    authority: &mut P,
) -> Result<(), AmbientSessionAdmissionError> {
    let ceiling = grant.scope_ceiling();
    let terms = binding.deployment_terms();
    let relationship_admitted = match lease.relationship_scope() {
        None => ceiling.relationship_scopes.is_empty(),
        Some(scope) => ceiling.relationship_scopes.binary_search(scope).is_ok(),
    };
    let purposes_admitted = terms
        .permitted_purposes()
        .iter()
        .all(|purpose| ceiling.purposes.binary_search(purpose).is_ok());
    if ceiling
        .principal_scopes
        .binary_search(lease.principal_id())
        .is_err()
        || ceiling
            .controller_scopes
            .binary_search(lease.controller_id())
            .is_err()
        || !relationship_admitted
        || ceiling
            .environments
            .binary_search(terms.environment())
            .is_err()
        || !purposes_admitted
        || ceiling.authority_evidence.is_empty()
        || terms.principal_scope() != lease.principal_id()
        || terms.controller_scope() != lease.controller_id()
        || terms.relationship_scope() != lease.relationship_scope()
    {
        return Err(AmbientSessionAdmissionError::WrongScope);
    }
    authority
        .prove_data_classification_ceiling(
            terms.data_classification(),
            &ceiling.data_classification_ceiling,
        )
        .map_err(AmbientSessionAdmissionError::Authority)
}

/// Prove that every session capability belongs to the grant allowlist.
///
/// # Errors
///
/// Returns [`AmbientSessionAdmissionError::CapabilityOutsideGrant`] when any
/// selected capability is absent from the grant.
pub(super) fn validate_capabilities(
    lease: &SessionAdmissionLeaseV1,
    grant: &CapabilitySessionGrantV1,
) -> Result<(), AmbientSessionAdmissionError> {
    if lease
        .capability_ceiling()
        .iter()
        .all(|capability: &CapabilityIdV1| {
            grant
                .allowed_capability_ids()
                .binary_search(capability)
                .is_ok()
        })
    {
        Ok(())
    } else {
        Err(AmbientSessionAdmissionError::CapabilityOutsideGrant)
    }
}

/// Prove one origin, destination, transport, route, and contract-set selection.
///
/// # Errors
///
/// Returns [`AmbientSessionAdmissionError::RouteOutsideGrant`] when any route
/// coordinate is absent from the grant ceiling.
pub(super) fn validate_route(
    route_use: &AmbientSessionRouteUse<'_>,
    grant: &CapabilitySessionGrantV1,
    principal: &ResidentPrincipalIdV1,
) -> Result<(), AmbientSessionAdmissionError> {
    let ceiling = grant.allowed_route_constraints();
    let destination_node_admitted = route_use
        .destination_node_id
        .is_none_or(|node| ceiling.destination_node_ids.binary_search(node).is_ok());
    if ceiling.origin_principal_id != *principal
        || ceiling.origin_node_id != *route_use.origin_node_id
        || ceiling.routes.binary_search(route_use.route).is_err()
        || ceiling
            .transport_classes
            .binary_search(&route_use.transport_class)
            .is_err()
        || ceiling
            .destination_principal_ids
            .binary_search(route_use.destination_principal_id)
            .is_err()
        || !destination_node_admitted
        || ceiling
            .contract_sets
            .binary_search(route_use.contract_set)
            .is_err()
    {
        Err(AmbientSessionAdmissionError::RouteOutsideGrant)
    } else {
        Ok(())
    }
}
