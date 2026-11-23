//! Pure validation and lowering for heterogeneous organizational translation.

use std::collections::{BTreeMap, BTreeSet};

use pale_core::records::*;
pub(crate) mod record_identity;
use record_identity as key;

/// Deterministic rejection from organizational lowering.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OrganizationalTranslationError {
    /// The record does not identify the initial wire contract.
    #[error("organizational translation has an invalid schema coordinate")]
    SchemaCoordinate,
    /// A bounded family exceeds its initial-v1 ceiling.
    #[error("organizational translation exceeds the {kind} limit")]
    Limit {
        /// Bounded family.
        kind: &'static str,
    },
    /// One logical-set identity repeats.
    #[error("organizational translation repeats {kind} `{id}`")]
    Duplicate {
        /// Repeated family.
        kind: &'static str,
        /// Stable rendered identity.
        id: String,
    },
    /// One exact reference does not resolve.
    #[error("{kind} `{owner}` references missing {target_kind} `{target}`")]
    MissingReference {
        /// Referring family.
        kind: &'static str,
        /// Referring record.
        owner: String,
        /// Expected target family.
        target_kind: &'static str,
        /// Missing target.
        target: String,
    },
    /// One effective interval is empty, reversed, or outside canonical JSON.
    #[error("{kind} `{owner}` has an invalid effective interval")]
    InvalidInterval {
        /// Record family.
        kind: &'static str,
        /// Invalid record.
        owner: String,
    },
    /// A typed relation uses endpoints outside its closed meaning.
    #[error("relation `{relation}` has endpoints incompatible with `{class}`")]
    InvalidRelationEndpoints {
        /// Invalid relation.
        relation: OrganizationalRelationId,
        /// Declared relation class.
        class: OrganizationalRelationClassV1,
    },
    /// One directional service points from a scope to itself.
    #[error("directional service `{contract}` has identical provider and consumer scopes")]
    ReflexiveService {
        /// Invalid service contract.
        contract: Box<DirectionalServiceContractRefV1>,
    },
    /// A group-lens right and its supporting field disagree.
    #[error("group-lens profile `{profile}` has invalid support for `{right}`")]
    InvalidGroupLensSupport {
        /// Invalid profile.
        profile: GroupLensProfileId,
        /// Right whose selected or absent state disagrees with its support.
        right: GroupLensRightV1,
    },
    /// A group-lens profile floats free of its exact local holder or relation.
    #[error("group-lens profile `{profile}` has an invalid {kind} binding")]
    InvalidGroupLensBinding {
        /// Invalid profile.
        profile: GroupLensProfileId,
        /// Binding dimension that is missing or non-local.
        kind: &'static str,
    },
    /// A nested service policy ceiling is unbounded or repeats a logical member.
    #[error("directional service `{contract}` has an invalid policy ceiling: {source}")]
    InvalidPolicyCeiling {
        /// Namespace-qualified service contract.
        contract: Box<DirectionalServiceContractRefV1>,
        /// Canonical policy-set failure.
        #[source]
        source: InvalidWorkspacePolicyCeilingV1,
    },
    /// One authority-axis resource set is unbounded or repeats a member.
    #[error("organizational authority `{owner}` has invalid resources: {source}")]
    InvalidAuthorityResources {
        /// Owner of the authority axes.
        owner: WorkspaceResourceId,
        /// Canonical resource-set failure.
        #[source]
        source: InvalidWorkspacePolicyCeilingV1,
    },
    /// A child organizational record crosses the owner of its container.
    #[error("{kind} `{child}` crosses owner of {container_kind} `{container}`")]
    CrossOwnerContainment {
        /// Child family.
        kind: &'static str,
        /// Child identity.
        child: String,
        /// Container family.
        container_kind: &'static str,
        /// Container identity.
        container: String,
    },
    /// A relation declared as local crosses an owner or local container.
    #[error("local relation `{relation}` crosses its lawful local scope")]
    NonLocalRelation {
        /// Invalid relation.
        relation: OrganizationalRelationId,
    },
    /// A full-matrix inspection contract omits a coordinate family or state.
    #[error("inspection contract `{inspection}` is not a complete explicit matrix")]
    IncompleteInspection {
        /// Invalid inspection contract.
        inspection: OrganizationalInspectionId,
    },
}

/// Validate and canonically lower explicit organizational selections.
///
/// Lowering sorts and checks only author-supplied facts. It never derives an
/// organization, tree, relation, authority, native projection, or view member
/// from a label or neighboring topology.
///
/// # Errors
///
/// Returns [`OrganizationalTranslationError`] when bounds, identities,
/// references, effective intervals, relation endpoints, service direction, or
/// explicit inspection requirements are invalid.
pub(crate) fn lower_organizational_translation(
    mut source: OrganizationalTranslationV1,
) -> Result<OrganizationalTranslationIrV1, OrganizationalTranslationError> {
    if source.schema != ORGANIZATIONAL_TRANSLATION_V1_SCHEMA
        || source.schema_version != ORGANIZATIONAL_TRANSLATION_V1_SCHEMA_VERSION
    {
        return Err(OrganizationalTranslationError::SchemaCoordinate);
    }
    enforce_family_limits(&source)?;
    canonicalize_source(&mut source)?;
    validate_source(&source)?;
    let derived_relations = derive_dedicated_relations(&source)?;
    Ok(OrganizationalTranslationIrV1 {
        source,
        derived_relations,
    })
}

/// Enforce per-family bounds before allocation or graph construction grows.
fn enforce_family_limits(
    source: &OrganizationalTranslationV1,
) -> Result<(), OrganizationalTranslationError> {
    for (kind, observed, limit) in [
        ("scope", source.scopes.len(), MAX_ORGANIZATIONAL_RECORDS),
        (
            "organization",
            source.organizations.len(),
            MAX_ORGANIZATIONAL_RECORDS,
        ),
        ("group", source.groups.len(), MAX_ORGANIZATIONAL_RECORDS),
        (
            "discipline tree",
            source.discipline_trees.len(),
            MAX_ORGANIZATIONAL_RECORDS,
        ),
        ("role", source.roles.len(), MAX_ORGANIZATIONAL_RECORDS),
        (
            "occupant binding",
            source.occupant_bindings.len(),
            MAX_ORGANIZATIONAL_RECORDS,
        ),
        (
            "provider binding",
            source.provider_bindings.len(),
            MAX_ORGANIZATIONAL_RECORDS,
        ),
        (
            "communication actor binding",
            source.communication_actor_bindings.len(),
            MAX_ORGANIZATIONAL_RECORDS,
        ),
        (
            "leadership participation",
            source.leadership.len(),
            MAX_ORGANIZATIONAL_RECORDS,
        ),
        (
            "relation",
            source.relations.len(),
            MAX_ORGANIZATIONAL_RELATIONS,
        ),
        ("object", source.objects.len(), MAX_ORGANIZATIONAL_RECORDS),
        (
            "population rule",
            source.population_rules.len(),
            MAX_ORGANIZATIONAL_RECORDS,
        ),
        (
            "group lens",
            source.group_lenses.len(),
            MAX_ORGANIZATIONAL_RECORDS,
        ),
        ("service", source.services.len(), MAX_ORGANIZATIONAL_RECORDS),
        (
            "native projection",
            source.native_projections.len(),
            MAX_ORGANIZATIONAL_RECORDS,
        ),
    ] {
        if observed > limit {
            return Err(OrganizationalTranslationError::Limit { kind });
        }
    }
    for binding in &source.communication_actor_bindings {
        enforce_range(
            "communication local resolution",
            binding.local_resolutions.len(),
            0,
            MAX_ORGANIZATIONAL_NESTED_MEMBERS,
        )?;
        for resolution in &binding.local_resolutions {
            enforce_range(
                "communication authorized closure",
                resolution.authorized_closure.len(),
                0,
                MAX_ORGANIZATIONAL_NESTED_MEMBERS,
            )?;
        }
    }
    for leadership in &source.leadership {
        enforce_range(
            "leadership participant",
            leadership.participants.len(),
            1,
            MAX_ORGANIZATIONAL_NESTED_MEMBERS,
        )?;
        enforce_range(
            "affected leadership scope",
            leadership.affected_scopes.len(),
            1,
            MAX_ORGANIZATIONAL_NESTED_MEMBERS,
        )?;
    }
    for lens in &source.group_lenses {
        enforce_range(
            "group-lens standard",
            lens.standards.len(),
            0,
            MAX_ORGANIZATIONAL_NESTED_MEMBERS,
        )?;
        enforce_range(
            "group-lens block condition",
            lens.block_conditions.len(),
            0,
            MAX_ORGANIZATIONAL_NESTED_MEMBERS,
        )?;
        enforce_range(
            "group-lens right",
            lens.rights.len(),
            0,
            MAX_GROUP_LENS_RIGHTS,
        )?;
    }
    for service in &source.services {
        enforce_range(
            "service assignment",
            service.assignments.len(),
            0,
            MAX_ORGANIZATIONAL_NESTED_MEMBERS,
        )?;
        for assignment in &service.assignments {
            enforce_range(
                "service steward",
                assignment.stewards.len(),
                0,
                MAX_ORGANIZATIONAL_NESTED_MEMBERS,
            )?;
            enforce_range(
                "service pod",
                assignment.pods.len(),
                0,
                MAX_ORGANIZATIONAL_NESTED_MEMBERS,
            )?;
        }
    }
    for projection in &source.native_projections {
        enforce_range(
            "native reference",
            projection.native_refs.len(),
            0,
            MAX_ORGANIZATIONAL_NESTED_MEMBERS,
        )?;
    }
    if let Some(inspection) = &source.inspection {
        enforce_range(
            "inspection coordinate",
            inspection.coordinates.len(),
            1,
            MAX_INSPECTION_COORDINATES,
        )?;
        enforce_range(
            "inspection state",
            inspection.states.len(),
            1,
            MAX_INSPECTION_STATES,
        )?;
    }
    Ok(())
}

/// Enforce one declared inclusive collection bound before canonicalization.
pub(super) fn enforce_range(
    kind: &'static str,
    observed: usize,
    minimum: usize,
    maximum: usize,
) -> Result<(), OrganizationalTranslationError> {
    if observed < minimum || observed > maximum {
        Err(OrganizationalTranslationError::Limit { kind })
    } else {
        Ok(())
    }
}

/// Canonicalize every logical set without changing semantic membership.
fn canonicalize_source(
    source: &mut OrganizationalTranslationV1,
) -> Result<(), OrganizationalTranslationError> {
    source
        .scopes
        .sort_by(|left, right| key::scope(left).cmp(key::scope(right)));
    reject_adjacent_duplicate(&source.scopes, "scope", |item| key::scope(item).clone())?;
    source
        .organizations
        .sort_by(|left, right| key::organization(left).cmp(key::organization(right)));
    reject_adjacent_duplicate(&source.organizations, "organization", |item| {
        key::organization(item).clone()
    })?;
    source
        .groups
        .sort_by(|left, right| key::group(left).cmp(key::group(right)));
    reject_adjacent_duplicate(&source.groups, "group", |item| key::group(item).clone())?;
    source
        .discipline_trees
        .sort_by(|left, right| key::tree(left).cmp(key::tree(right)));
    reject_adjacent_duplicate(&source.discipline_trees, "discipline tree", |item| {
        key::tree(item).clone()
    })?;
    source
        .roles
        .sort_by(|left, right| key::role(left).cmp(key::role(right)));
    reject_adjacent_duplicate(&source.roles, "role", |item| key::role(item).clone())?;
    source
        .occupant_bindings
        .sort_by(|left, right| key::occupant(left).cmp(&key::occupant(right)));
    reject_adjacent_duplicate(&source.occupant_bindings, "occupant binding", |item| {
        let (role, occupant, interval) = key::occupant(item);
        (role.clone(), occupant.clone(), interval.clone())
    })?;
    source
        .provider_bindings
        .sort_by(|left, right| key::provider(left).cmp(&key::provider(right)));
    reject_adjacent_duplicate(&source.provider_bindings, "provider binding", |item| {
        let (role, provider, interval) = key::provider(item);
        (role.clone(), provider.clone(), interval.clone())
    })?;
    source
        .communication_actor_bindings
        .sort_by(|left, right| key::communication(left).cmp(&key::communication(right)));
    reject_adjacent_duplicate(
        &source.communication_actor_bindings,
        "communication actor binding",
        |item| {
            let (provider, handle) = key::communication(item);
            (provider.clone(), handle.clone())
        },
    )?;
    for binding in &mut source.communication_actor_bindings {
        binding
            .local_resolutions
            .sort_by(|left, right| left.local_node.cmp(&right.local_node));
        reject_adjacent_duplicate(
            &binding.local_resolutions,
            "communication local resolution",
            |item| item.local_node.clone(),
        )?;
        for resolution in &mut binding.local_resolutions {
            canonical_set(
                &mut resolution.authorized_closure,
                "communication authorized closure",
                binding.handle.as_ref(),
            )?;
        }
    }
    source
        .leadership
        .sort_by(|left, right| key::leadership(left).cmp(key::leadership(right)));
    reject_adjacent_duplicate(&source.leadership, "leadership topic", |item| {
        key::leadership(item).clone()
    })?;
    source
        .relations
        .sort_by(|left, right| key::relation(left).cmp(key::relation(right)));
    reject_adjacent_duplicate(&source.relations, "relation", |item| {
        key::relation(item).clone()
    })?;
    source
        .objects
        .sort_by(|left, right| key::object(left).cmp(key::object(right)));
    reject_adjacent_duplicate(&source.objects, "object", |item| key::object(item).clone())?;
    source
        .population_rules
        .sort_by(|left, right| key::population(left).cmp(&key::population(right)));
    reject_adjacent_duplicate(&source.population_rules, "population rule", |item| {
        let (object_class, role_class) = key::population(item);
        (object_class.clone(), role_class.clone())
    })?;
    source
        .group_lenses
        .sort_by(|left, right| key::lens(left).cmp(key::lens(right)));
    reject_adjacent_duplicate(&source.group_lenses, "group lens", |item| {
        key::lens(item).clone()
    })?;
    source
        .services
        .sort_by_key(DirectionalServiceContractV1::contract_ref);
    if let Some(pair) = source
        .services
        .windows(2)
        .find(|pair| pair[0].contract_ref() == pair[1].contract_ref())
    {
        return Err(OrganizationalTranslationError::Duplicate {
            kind: "service",
            id: pair[0].contract_ref().to_string(),
        });
    }
    source
        .native_projections
        .sort_by(|left, right| key::projection(left).cmp(key::projection(right)));
    reject_adjacent_duplicate(&source.native_projections, "native projection", |item| {
        key::projection(item).clone()
    })?;

    for axes in source
        .scopes
        .iter_mut()
        .map(|item| &mut item.axes)
        .chain(source.organizations.iter_mut().map(|item| &mut item.axes))
        .chain(source.groups.iter_mut().map(|item| &mut item.axes))
        .chain(
            source
                .discipline_trees
                .iter_mut()
                .map(|item| &mut item.axes),
        )
        .chain(source.roles.iter_mut().map(|item| &mut item.axes))
        .chain(source.objects.iter_mut().map(|item| &mut item.axes))
    {
        canonicalize_workspace_ceiling_set(
            &mut axes.resources,
            WorkspaceCeilingSetAxisV1::Resource,
        )
        .map_err(
            |source| OrganizationalTranslationError::InvalidAuthorityResources {
                owner: axes.owner.clone(),
                source,
            },
        )?;
    }
    for leadership in &mut source.leadership {
        canonical_set(
            &mut leadership.participants,
            "leadership participant",
            leadership.topic.as_ref(),
        )?;
        canonical_set(
            &mut leadership.affected_scopes,
            "affected leadership scope",
            leadership.topic.as_ref(),
        )?;
    }
    for lens in &mut source.group_lenses {
        canonical_set(
            &mut lens.standards,
            "group-lens standard",
            lens.profile_id.as_ref(),
        )?;
        canonical_set(
            &mut lens.block_conditions,
            "group-lens block condition",
            lens.profile_id.as_ref(),
        )?;
        canonical_set(
            &mut lens.rights,
            "group-lens right",
            lens.profile_id.as_ref(),
        )?;
    }
    for service in &mut source.services {
        service
            .ceiling
            .canonicalize_policy_sets()
            .map_err(
                |source| OrganizationalTranslationError::InvalidPolicyCeiling {
                    contract: Box::new(service.contract_ref()),
                    source,
                },
            )?;
        service.assignments.sort_by(|left, right| {
            (&left.account, &left.mandate, &left.interval).cmp(&(
                &right.account,
                &right.mandate,
                &right.interval,
            ))
        });
        if let Some(pair) = service.assignments.windows(2).find(|pair| {
            (&pair[0].account, &pair[0].mandate, &pair[0].interval)
                == (&pair[1].account, &pair[1].mandate, &pair[1].interval)
        }) {
            return Err(OrganizationalTranslationError::Duplicate {
                kind: "service assignment",
                id: format!(
                    "{}:{}:{}:{:?}",
                    service.contract_ref(),
                    pair[0].account,
                    pair[0].mandate,
                    pair[0].interval
                ),
            });
        }
        let contract = service.contract_ref().to_string();
        for assignment in &mut service.assignments {
            canonical_set(&mut assignment.stewards, "service steward", &contract)?;
            canonical_set(&mut assignment.pods, "service pod", &contract)?;
        }
    }
    for projection in &mut source.native_projections {
        canonical_set(
            &mut projection.native_refs,
            "native reference",
            projection.projection_id.as_ref(),
        )?;
    }
    if let Some(inspection) = &mut source.inspection {
        canonical_set(
            &mut inspection.coordinates,
            "inspection coordinate",
            inspection.inspection_id.as_ref(),
        )?;
        canonical_set(
            &mut inspection.states,
            "inspection state",
            inspection.inspection_id.as_ref(),
        )?;
    }
    Ok(())
}

/// Validate references and relation-specific invariants after canonicalization.
fn validate_source(
    source: &OrganizationalTranslationV1,
) -> Result<(), OrganizationalTranslationError> {
    let node_index = NodeIndex {
        scopes: source.scopes.iter().map(|item| &item.scope).collect(),
        organizations: source
            .organizations
            .iter()
            .map(|item| &item.organization)
            .collect(),
        groups: source.groups.iter().map(|item| &item.group).collect(),
        trees: source
            .discipline_trees
            .iter()
            .map(|item| &item.tree)
            .collect(),
        roles: source.roles.iter().map(|item| &item.role).collect(),
        occupants: source
            .occupant_bindings
            .iter()
            .map(|item| &item.occupant)
            .collect(),
        providers: source
            .provider_bindings
            .iter()
            .map(|item| &item.provider)
            .collect(),
        objects: source.objects.iter().map(|item| &item.object_id).collect(),
    };
    let scope_records = source
        .scopes
        .iter()
        .map(|item| (&item.scope, item))
        .collect::<BTreeMap<_, _>>();
    let organization_records = source
        .organizations
        .iter()
        .map(|item| (&item.organization, item))
        .collect::<BTreeMap<_, _>>();
    let group_records = source
        .groups
        .iter()
        .map(|item| (&item.group, item))
        .collect::<BTreeMap<_, _>>();
    let tree_records = source
        .discipline_trees
        .iter()
        .map(|item| (&item.tree, item))
        .collect::<BTreeMap<_, _>>();
    let role_records = source
        .roles
        .iter()
        .map(|item| (&item.role, item))
        .collect::<BTreeMap<_, _>>();
    let object_records = source
        .objects
        .iter()
        .map(|item| (&item.object_id, item))
        .collect::<BTreeMap<_, _>>();
    let relation_records = source
        .relations
        .iter()
        .map(|item| (&item.relation_id, item))
        .collect::<BTreeMap<_, _>>();

    for scope in &source.scopes {
        validate_interval("scope", scope.scope.scope_id.as_ref(), &scope.interval)?;
    }
    for organization in &source.organizations {
        require_ref(
            node_index.scopes.contains(&organization.scope),
            "organization",
            organization.organization.organization_id.as_ref(),
            "scope",
            organization.scope.scope_id.as_ref(),
        )?;
        let container = scope_records
            .get(&organization.scope)
            .expect("resolved scope has a record");
        require_local_containment(
            organization.organization.namespace == organization.scope.namespace
                && organization.axes.owner == container.axes.owner,
            "organization",
            organization.organization.organization_id.as_ref(),
            "scope",
            organization.scope.scope_id.as_ref(),
        )?;
        validate_interval(
            "organization",
            organization.organization.organization_id.as_ref(),
            &organization.interval,
        )?;
    }
    for group in &source.groups {
        require_ref(
            node_index.organizations.contains(&group.organization),
            "group",
            group.group.group_id.as_ref(),
            "organization",
            group.organization.organization_id.as_ref(),
        )?;
        let container = organization_records
            .get(&group.organization)
            .expect("resolved organization has a record");
        require_local_containment(
            group.group.namespace == group.organization.namespace
                && group.axes.owner == container.axes.owner,
            "group",
            group.group.group_id.as_ref(),
            "organization",
            group.organization.organization_id.as_ref(),
        )?;
        validate_interval("group", group.group.group_id.as_ref(), &group.interval)?;
    }
    for tree in &source.discipline_trees {
        require_ref(
            node_index.organizations.contains(&tree.organization),
            "discipline tree",
            tree.tree.tree_id.as_ref(),
            "organization",
            tree.organization.organization_id.as_ref(),
        )?;
        let container = organization_records
            .get(&tree.organization)
            .expect("resolved organization has a record");
        require_local_containment(
            tree.tree.namespace == tree.organization.namespace
                && tree.axes.owner == container.axes.owner,
            "discipline tree",
            tree.tree.tree_id.as_ref(),
            "organization",
            tree.organization.organization_id.as_ref(),
        )?;
        validate_interval(
            "discipline tree",
            tree.tree.tree_id.as_ref(),
            &tree.interval,
        )?;
    }
    for role in &source.roles {
        require_ref(
            node_index.organizations.contains(&role.organization),
            "role",
            role.role.role_id.as_ref(),
            "organization",
            role.organization.organization_id.as_ref(),
        )?;
        let container = organization_records
            .get(&role.organization)
            .expect("resolved organization has a record");
        require_local_containment(
            role.role.namespace == role.organization.namespace
                && role.axes.owner == container.axes.owner,
            "role",
            role.role.role_id.as_ref(),
            "organization",
            role.organization.organization_id.as_ref(),
        )?;
        if let Some(group) = &role.group {
            require_ref(
                node_index.groups.contains(group),
                "role",
                role.role.role_id.as_ref(),
                "group",
                group.group_id.as_ref(),
            )?;
            let group_record = group_records
                .get(group)
                .expect("resolved group has a record");
            require_local_containment(
                group_record.organization == role.organization
                    && group_record.axes.owner == role.axes.owner,
                "role",
                role.role.role_id.as_ref(),
                "group",
                group.group_id.as_ref(),
            )?;
        }
        if let Some(tree) = &role.discipline_tree {
            require_ref(
                node_index.trees.contains(tree),
                "role",
                role.role.role_id.as_ref(),
                "discipline tree",
                tree.tree_id.as_ref(),
            )?;
            let tree_record = tree_records.get(tree).expect("resolved tree has a record");
            require_local_containment(
                tree_record.organization == role.organization
                    && tree_record.axes.owner == role.axes.owner,
                "role",
                role.role.role_id.as_ref(),
                "discipline tree",
                tree.tree_id.as_ref(),
            )?;
        }
        validate_interval("role", role.role.role_id.as_ref(), &role.interval)?;
    }
    for binding in &source.occupant_bindings {
        require_ref(
            node_index.roles.contains(&binding.role),
            "occupant binding",
            binding.occupant.as_ref(),
            "role",
            binding.role.role_id.as_ref(),
        )?;
        validate_interval(
            "occupant binding",
            binding.occupant.as_ref(),
            &binding.interval,
        )?;
    }
    for binding in &source.provider_bindings {
        require_ref(
            node_index.roles.contains(&binding.role),
            "provider binding",
            binding.provider.as_ref(),
            "role",
            binding.role.role_id.as_ref(),
        )?;
        validate_interval(
            "provider binding",
            binding.provider.as_ref(),
            &binding.interval,
        )?;
    }
    for binding in &source.communication_actor_bindings {
        require_ref(
            node_index.providers.contains(&binding.provider),
            "communication actor binding",
            binding.handle.as_ref(),
            "provider",
            binding.provider.as_ref(),
        )?;
        for resolution in &binding.local_resolutions {
            for node in
                std::iter::once(&resolution.local_node).chain(resolution.authorized_closure.iter())
            {
                if !node_index.contains(node) {
                    return Err(OrganizationalTranslationError::MissingReference {
                        kind: "communication actor binding",
                        owner: binding.handle.to_string(),
                        target_kind: "admitted local node",
                        target: "unresolved local closure".into(),
                    });
                }
            }
        }
        validate_interval(
            "communication actor binding",
            binding.handle.as_ref(),
            &binding.interval,
        )?;
    }
    for leadership in &source.leadership {
        for role in &leadership.participants {
            require_ref(
                node_index.roles.contains(role),
                "leadership participation",
                leadership.topic.as_ref(),
                "role",
                role.role_id.as_ref(),
            )?;
        }
        for scope in &leadership.affected_scopes {
            require_ref(
                node_index.scopes.contains(scope),
                "leadership participation",
                leadership.topic.as_ref(),
                "scope",
                scope.scope_id.as_ref(),
            )?;
        }
        validate_interval(
            "leadership participation",
            leadership.topic.as_ref(),
            &leadership.interval,
        )?;
    }
    for relation in &source.relations {
        if !node_index.contains(&relation.from) || !node_index.contains(&relation.to) {
            return Err(OrganizationalTranslationError::MissingReference {
                kind: "relation",
                owner: relation.relation_id.to_string(),
                target_kind: "organizational node",
                target: "unresolved endpoint".into(),
            });
        }
        if !valid_relation_endpoints(relation) {
            return Err(OrganizationalTranslationError::InvalidRelationEndpoints {
                relation: relation.relation_id.clone(),
                class: relation.class,
            });
        }
        if relation.class.locality() == OrganizationalRelationLocalityV1::Internal
            && !relation_is_local(
                relation,
                &scope_records,
                &organization_records,
                &group_records,
                &tree_records,
                &role_records,
                &object_records,
            )
        {
            return Err(OrganizationalTranslationError::NonLocalRelation {
                relation: relation.relation_id.clone(),
            });
        }
        validate_interval(
            "relation",
            relation.relation_id.as_ref(),
            &relation.interval,
        )?;
    }
    for object in &source.objects {
        require_ref(
            node_index.scopes.contains(&object.scope),
            "object",
            object.object_id.as_ref(),
            "scope",
            object.scope.scope_id.as_ref(),
        )?;
        require_ref(
            node_index.organizations.contains(&object.organization),
            "object",
            object.object_id.as_ref(),
            "organization",
            object.organization.organization_id.as_ref(),
        )?;
        let organization = organization_records
            .get(&object.organization)
            .expect("resolved object organization has a record");
        require_local_containment(
            object.namespace == object.organization.namespace
                && object.scope == organization.scope
                && object.axes.owner == organization.axes.owner,
            "object",
            object.object_id.as_ref(),
            "organization",
            object.organization.organization_id.as_ref(),
        )?;
        validate_interval("object", object.object_id.as_ref(), &object.interval)?;
    }
    for rule in &source.population_rules {
        validate_interval(
            "population rule",
            rule.object_class.as_ref(),
            &rule.interval,
        )?;
    }
    for lens in &source.group_lenses {
        let holder = role_records.get(&lens.holder).ok_or_else(|| {
            OrganizationalTranslationError::InvalidGroupLensBinding {
                profile: lens.profile_id.clone(),
                kind: "holder role",
            }
        })?;
        let alignment = relation_records
            .get(&lens.alignment_relation)
            .ok_or_else(|| OrganizationalTranslationError::InvalidGroupLensBinding {
                profile: lens.profile_id.clone(),
                kind: "alignment relation",
            })?;
        if !alignment.class.is_group_lens_alignment()
            || alignment.from
                != (OrganizationalNodeRefV1::Role {
                    role: lens.holder.clone(),
                })
        {
            return Err(OrganizationalTranslationError::InvalidGroupLensBinding {
                profile: lens.profile_id.clone(),
                kind: "holder-to-relation",
            });
        }
        if let Some(review_owner) = &lens.review_owner {
            require_ref(
                node_index.roles.contains(review_owner),
                "group lens",
                lens.profile_id.as_ref(),
                "review role",
                review_owner.role_id.as_ref(),
            )?;
            let reviewer = role_records
                .get(review_owner)
                .expect("the review role reference was validated above");
            if !roles_share_locality(holder, reviewer) {
                return Err(OrganizationalTranslationError::InvalidGroupLensBinding {
                    profile: lens.profile_id.clone(),
                    kind: "review locality",
                });
            }
        }
        if let Some(escalation_resolver) = &lens.escalation_resolver {
            require_ref(
                node_index.roles.contains(escalation_resolver),
                "group lens",
                lens.profile_id.as_ref(),
                "escalation role",
                escalation_resolver.role_id.as_ref(),
            )?;
            let resolver = role_records
                .get(escalation_resolver)
                .expect("the escalation role reference was validated above");
            if !roles_share_locality(holder, resolver) {
                return Err(OrganizationalTranslationError::InvalidGroupLensBinding {
                    profile: lens.profile_id.clone(),
                    kind: "escalation locality",
                });
            }
        }
        for (right, supported) in [
            (GroupLensRightV1::Standards, !lens.standards.is_empty()),
            (GroupLensRightV1::Review, lens.review_owner.is_some()),
            (
                GroupLensRightV1::BoundedBlock,
                !lens.block_conditions.is_empty(),
            ),
            (
                GroupLensRightV1::Escalation,
                lens.escalation_resolver.is_some(),
            ),
        ] {
            if lens.rights.contains(&right) != supported {
                return Err(OrganizationalTranslationError::InvalidGroupLensSupport {
                    profile: lens.profile_id.clone(),
                    right,
                });
            }
        }
        validate_interval("group lens", lens.profile_id.as_ref(), &lens.interval)?;
    }
    for service in &source.services {
        let contract = service.contract_ref();
        let contract_coordinate = contract.to_string();
        require_ref(
            node_index.scopes.contains(&service.provider_scope),
            "directional service",
            &contract_coordinate,
            "provider scope",
            service.provider_scope.scope_id.as_ref(),
        )?;
        require_ref(
            node_index.scopes.contains(&service.consumer_scope),
            "directional service",
            &contract_coordinate,
            "consumer scope",
            service.consumer_scope.scope_id.as_ref(),
        )?;
        if service.provider_scope == service.consumer_scope {
            return Err(OrganizationalTranslationError::ReflexiveService {
                contract: Box::new(contract),
            });
        }
        validate_interval(
            "directional service",
            &contract_coordinate,
            &service.interval,
        )?;
        for assignment in &service.assignments {
            let assignment_coordinate = format!(
                "{}:{}:{}",
                service.contract_ref(),
                assignment.account,
                assignment.mandate
            );
            validate_interval(
                "service assignment",
                &assignment_coordinate,
                &assignment.interval,
            )?;
        }
    }
    for projection in &source.native_projections {
        if !node_index.contains(&projection.subject) {
            return Err(OrganizationalTranslationError::MissingReference {
                kind: "native projection",
                owner: projection.projection_id.to_string(),
                target_kind: "organizational node",
                target: "unresolved subject".into(),
            });
        }
        validate_interval(
            "native projection",
            projection.projection_id.as_ref(),
            &projection.interval,
        )?;
    }
    if let Some(inspection) = &source.inspection {
        if inspection.coordinates != InspectionCoordinateClassV1::all()
            || inspection.states != EffectiveFactStateV1::all()
        {
            return Err(OrganizationalTranslationError::IncompleteInspection {
                inspection: inspection.inspection_id.clone(),
            });
        }
    }
    Ok(())
}

/// Require one exact reference and retain a typed diagnostic on failure.
fn require_ref(
    exists: bool,
    kind: &'static str,
    owner: &str,
    target_kind: &'static str,
    target: &str,
) -> Result<(), OrganizationalTranslationError> {
    if exists {
        Ok(())
    } else {
        Err(OrganizationalTranslationError::MissingReference {
            kind,
            owner: owner.to_string(),
            target_kind,
            target: target.to_string(),
        })
    }
}

/// Require a child and its container to retain the same local owner boundary.
fn require_local_containment(
    lawful: bool,
    kind: &'static str,
    child: &str,
    container_kind: &'static str,
    container: &str,
) -> Result<(), OrganizationalTranslationError> {
    if lawful {
        Ok(())
    } else {
        Err(OrganizationalTranslationError::CrossOwnerContainment {
            kind,
            child: child.to_string(),
            container_kind,
            container: container.to_string(),
        })
    }
}

/// Validate an effective interval against ordering and canonical JSON bounds.
fn validate_interval(
    kind: &'static str,
    owner: &str,
    interval: &EffectiveIntervalV1,
) -> Result<(), OrganizationalTranslationError> {
    let valid_end = interval
        .end_exclusive
        .is_none_or(|end| end > interval.start && end <= pale_core::canon::JSON_SAFE_INTEGER_MAX);
    if interval.start <= pale_core::canon::JSON_SAFE_INTEGER_MAX && valid_end {
        Ok(())
    } else {
        Err(OrganizationalTranslationError::InvalidInterval {
            kind,
            owner: owner.to_string(),
        })
    }
}

/// Exact source-local node inventory used by reference checks.
struct NodeIndex<'a> {
    /// Declared scopes.
    scopes: BTreeSet<&'a LogicalScopeRefV1>,
    /// Declared organizations.
    organizations: BTreeSet<&'a LogicalOrganizationRefV1>,
    /// Declared groups.
    groups: BTreeSet<&'a OrganizationalGroupRefV1>,
    /// Declared trees.
    trees: BTreeSet<&'a DisciplineTreeRefV1>,
    /// Declared roles.
    roles: BTreeSet<&'a OrganizationalRoleRefV1>,
    /// Occupants established by dedicated bindings.
    occupants: BTreeSet<&'a OrganizationalOccupantId>,
    /// Providers established by dedicated bindings.
    providers: BTreeSet<&'a OrganizationalProviderId>,
    /// Declared objects.
    objects: BTreeSet<&'a OrganizationalObjectId>,
}

impl NodeIndex<'_> {
    /// Return whether one typed node reference resolves inside the source.
    fn contains(&self, node: &OrganizationalNodeRefV1) -> bool {
        match node {
            OrganizationalNodeRefV1::Scope { scope } => self.scopes.contains(scope),
            OrganizationalNodeRefV1::Organization { organization } => {
                self.organizations.contains(organization)
            }
            OrganizationalNodeRefV1::Group { group } => self.groups.contains(group),
            OrganizationalNodeRefV1::DisciplineTree { tree } => self.trees.contains(tree),
            OrganizationalNodeRefV1::Role { role } => self.roles.contains(role),
            OrganizationalNodeRefV1::Occupant { occupant } => self.occupants.contains(occupant),
            OrganizationalNodeRefV1::Provider { provider } => self.providers.contains(provider),
            OrganizationalNodeRefV1::NativeResource { .. } => true,
            OrganizationalNodeRefV1::Object { object } => self.objects.contains(object),
        }
    }
}

/// Local owner and available container coordinates for one relation endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
struct LocalRelationContext {
    /// Semantic owner of the endpoint fact.
    owner: WorkspaceResourceId,
    /// Logical scope when the endpoint is scope- or organization-bound.
    scope: Option<LogicalScopeRefV1>,
    /// Exact organization when the endpoint is organization-bound.
    organization: Option<LogicalOrganizationRefV1>,
}

/// Resolve the local context of a node eligible for local coordination.
fn local_relation_context(
    node: &OrganizationalNodeRefV1,
    scope_records: &BTreeMap<&LogicalScopeRefV1, &LogicalScopeV1>,
    organization_records: &BTreeMap<&LogicalOrganizationRefV1, &LogicalOrganizationV1>,
    group_records: &BTreeMap<&OrganizationalGroupRefV1, &OrganizationalGroupV1>,
    tree_records: &BTreeMap<&DisciplineTreeRefV1, &DisciplineTreeV1>,
    role_records: &BTreeMap<&OrganizationalRoleRefV1, &OrganizationalRoleV1>,
    object_records: &BTreeMap<&OrganizationalObjectId, &OrganizationalObjectV1>,
) -> Option<LocalRelationContext> {
    match node {
        OrganizationalNodeRefV1::Scope { scope } => {
            scope_records.get(scope).map(|record| LocalRelationContext {
                owner: record.axes.owner.clone(),
                scope: Some(scope.clone()),
                organization: None,
            })
        }
        OrganizationalNodeRefV1::Organization { organization } => organization_records
            .get(organization)
            .map(|record| LocalRelationContext {
                owner: record.axes.owner.clone(),
                scope: Some(record.scope.clone()),
                organization: Some(organization.clone()),
            }),
        OrganizationalNodeRefV1::Group { group } => group_records.get(group).and_then(|record| {
            organization_records
                .get(&record.organization)
                .map(|organization| LocalRelationContext {
                    owner: record.axes.owner.clone(),
                    scope: Some(organization.scope.clone()),
                    organization: Some(record.organization.clone()),
                })
        }),
        OrganizationalNodeRefV1::DisciplineTree { tree } => {
            tree_records.get(tree).and_then(|record| {
                organization_records
                    .get(&record.organization)
                    .map(|organization| LocalRelationContext {
                        owner: record.axes.owner.clone(),
                        scope: Some(organization.scope.clone()),
                        organization: Some(record.organization.clone()),
                    })
            })
        }
        OrganizationalNodeRefV1::Role { role } => role_records.get(role).and_then(|record| {
            organization_records
                .get(&record.organization)
                .map(|organization| LocalRelationContext {
                    owner: record.axes.owner.clone(),
                    scope: Some(organization.scope.clone()),
                    organization: Some(record.organization.clone()),
                })
        }),
        OrganizationalNodeRefV1::Object { object } => {
            object_records
                .get(object)
                .map(|record| LocalRelationContext {
                    owner: record.axes.owner.clone(),
                    scope: Some(record.scope.clone()),
                    organization: Some(record.organization.clone()),
                })
        }
        OrganizationalNodeRefV1::Occupant { .. }
        | OrganizationalNodeRefV1::Provider { .. }
        | OrganizationalNodeRefV1::NativeResource { .. } => None,
    }
}

/// Return whether a local relation remains within one owner/container pair.
fn relation_is_local(
    relation: &OrganizationalRelationV1,
    scope_records: &BTreeMap<&LogicalScopeRefV1, &LogicalScopeV1>,
    organization_records: &BTreeMap<&LogicalOrganizationRefV1, &LogicalOrganizationV1>,
    group_records: &BTreeMap<&OrganizationalGroupRefV1, &OrganizationalGroupV1>,
    tree_records: &BTreeMap<&DisciplineTreeRefV1, &DisciplineTreeV1>,
    role_records: &BTreeMap<&OrganizationalRoleRefV1, &OrganizationalRoleV1>,
    object_records: &BTreeMap<&OrganizationalObjectId, &OrganizationalObjectV1>,
) -> bool {
    let left = local_relation_context(
        &relation.from,
        scope_records,
        organization_records,
        group_records,
        tree_records,
        role_records,
        object_records,
    );
    let right = local_relation_context(
        &relation.to,
        scope_records,
        organization_records,
        group_records,
        tree_records,
        role_records,
        object_records,
    );
    left.zip(right).is_some_and(|(left, right)| {
        left.owner == right.owner
            && match (&left.organization, &right.organization) {
                (Some(left), Some(right)) => left == right,
                _ => match (&left.scope, &right.scope) {
                    (Some(left), Some(right)) => left == right,
                    _ => true,
                },
            }
    })
}

/// Return whether two roles occupy the same owner and organization boundary.
fn roles_share_locality(left: &OrganizationalRoleV1, right: &OrganizationalRoleV1) -> bool {
    left.axes.owner == right.axes.owner && left.organization == right.organization
}

/// Derive graph-only occupancy, provider-presence, and leadership edges.
fn derive_dedicated_relations(
    source: &OrganizationalTranslationV1,
) -> Result<Vec<OrganizationalDerivedRelationV1>, OrganizationalTranslationError> {
    let leadership_edges = source.leadership.iter().try_fold(0_usize, |total, item| {
        item.participants
            .len()
            .checked_mul(item.affected_scopes.len())
            .and_then(|count| total.checked_add(count))
            .ok_or(OrganizationalTranslationError::Limit {
                kind: "derived relation",
            })
    })?;
    let capacity = source
        .occupant_bindings
        .len()
        .checked_add(source.provider_bindings.len())
        .and_then(|count| count.checked_add(leadership_edges))
        .filter(|count| *count <= MAX_ORGANIZATIONAL_RELATIONS)
        .ok_or(OrganizationalTranslationError::Limit {
            kind: "derived relation",
        })?;
    let mut derived = Vec::with_capacity(capacity);
    for binding in &source.occupant_bindings {
        derived.push(OrganizationalDerivedRelationV1 {
            source: OrganizationalDerivedRelationSourceV1::OccupantBinding {
                role: binding.role.clone(),
                occupant: binding.occupant.clone(),
                interval: binding.interval.clone(),
            },
            class: OrganizationalDerivedRelationClassV1::Occupancy,
            from: OrganizationalNodeRefV1::Role {
                role: binding.role.clone(),
            },
            to: OrganizationalNodeRefV1::Occupant {
                occupant: binding.occupant.clone(),
            },
            issuer: binding.issuer.clone(),
            state: binding.state,
            interval: binding.interval.clone(),
        });
    }
    for binding in &source.provider_bindings {
        derived.push(OrganizationalDerivedRelationV1 {
            source: OrganizationalDerivedRelationSourceV1::ProviderBinding {
                role: binding.role.clone(),
                provider: binding.provider.clone(),
                interval: binding.interval.clone(),
            },
            class: OrganizationalDerivedRelationClassV1::ProviderPresence,
            from: OrganizationalNodeRefV1::Role {
                role: binding.role.clone(),
            },
            to: OrganizationalNodeRefV1::Provider {
                provider: binding.provider.clone(),
            },
            issuer: binding.issuer.clone(),
            state: binding.state,
            interval: binding.interval.clone(),
        });
    }
    for leadership in &source.leadership {
        for participant in &leadership.participants {
            for scope in &leadership.affected_scopes {
                derived.push(OrganizationalDerivedRelationV1 {
                    source: OrganizationalDerivedRelationSourceV1::LeadershipParticipation {
                        topic: leadership.topic.clone(),
                    },
                    class: OrganizationalDerivedRelationClassV1::LeadershipParticipation,
                    from: OrganizationalNodeRefV1::Role {
                        role: participant.clone(),
                    },
                    to: OrganizationalNodeRefV1::Scope {
                        scope: scope.clone(),
                    },
                    issuer: leadership.issuer.clone(),
                    state: leadership.state,
                    interval: leadership.interval.clone(),
                });
            }
        }
    }
    derived.sort();
    Ok(derived)
}

/// Enforce endpoint shapes only where the relation class itself selects them.
fn valid_relation_endpoints(relation: &OrganizationalRelationV1) -> bool {
    use OrganizationalNodeRefV1 as Node;
    use OrganizationalRelationClassV1 as Class;
    match relation.class {
        Class::NativePresence => matches!(&relation.to, Node::NativeResource { .. }),
        Class::Primary | Class::Leader | Class::Supporting => matches!(
            (&relation.from, &relation.to),
            (Node::DisciplineTree { .. }, Node::DisciplineTree { .. })
                | (Node::Group { .. }, Node::DisciplineTree { .. })
        ),
        Class::PeopleManagement
        | Class::RoadmapCoordination
        | Class::DisciplineJudgment
        | Class::Standards
        | Class::Review
        | Class::BoundedBlock
        | Class::Escalation
        | Class::Acceptance
        | Class::Collaboration => matches!(
            (&relation.from, &relation.to),
            (Node::Role { .. }, Node::Role { .. })
                | (Node::Role { .. }, Node::Object { .. })
                | (Node::Role { .. }, Node::Scope { .. })
        ),
        Class::Coordinates => matches!(
            (&relation.from, &relation.to),
            (Node::Group { .. }, Node::DisciplineTree { .. })
                | (Node::Role { .. }, Node::Role { .. })
                | (Node::Scope { .. }, Node::Scope { .. })
        ),
    }
}

/// Canonicalize one logical set and reject repeated members.
fn canonical_set<T: Ord + std::fmt::Debug>(
    values: &mut [T],
    kind: &'static str,
    owner: &str,
) -> Result<(), OrganizationalTranslationError> {
    values.sort();
    if let Some(pair) = values.windows(2).find(|pair| pair[0] == pair[1]) {
        Err(OrganizationalTranslationError::Duplicate {
            kind,
            id: format!("{owner}:{:?}", pair[0]),
        })
    } else {
        Ok(())
    }
}

/// Reject repeated adjacent keys after an owning collection is sorted.
fn reject_adjacent_duplicate<T, K: PartialEq + std::fmt::Debug>(
    values: &[T],
    kind: &'static str,
    key: impl Fn(&T) -> K,
) -> Result<(), OrganizationalTranslationError> {
    if let Some(pair) = values
        .windows(2)
        .find(|pair| key(&pair[0]) == key(&pair[1]))
    {
        Err(OrganizationalTranslationError::Duplicate {
            kind,
            id: format!("{:?}", key(&pair[0])),
        })
    } else {
        Ok(())
    }
}
