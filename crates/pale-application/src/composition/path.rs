//! Typed composition-edge, path-budget, and exact-reference validation.
//!
//! Edge variants own their graph semantics. A selected experience path may
//! traverse only declared [`CompositionEdgeV1::WorkDelegation`] edges in its
//! ordered work spine; no path field can relabel an edge into another graph.
//! Additive work quantities and authority meets are distinct laws. For time,
//! cost, resource units, and effect count, [`ExperiencePathV1`] owns the exact
//! accumulation and authority-meet law.
//!
//! Work length is bounded separately, while each edge's depth, fan-out, and
//! recursion limits are checked pointwise against the path. Quantity addition
//! does not make traversal commutative: every ordered edge must begin at the
//! preceding endpoint. Effect meets are commutative, associative, and
//! idempotent, so combining governing ceilings can only narrow authority.

use std::collections::{BTreeMap, BTreeSet};

use pale_core::records::{
    ArtifactDigest, CompositionEdgeV1, CompositionReasonPathV1, CompositionReasonStepV1,
    ContinuityContractV1, ContractSchemaIdentityV1, EffectClass, EntryContractV1, ExperiencePathV1,
    ParticipantRefV1, PathInvocationV1, ProviderBindingV1, RequestTranslationV1,
    ResponsibilityContractV1, ResponsibilityV1, ResultTranslationV1, SchemaDigest, SchemaId,
    SettlementContractV1, TerminationContractV1, RESPONSIBILITY_CONTRACT_V1_SCHEMA_ID,
};

use super::graph::{participant_id, CompositionGraphs};
use super::{CompositionError, CompositionSourceV1, ProviderBindingViolation};

/// Borrowed symbol tables used by every composition relation check.
///
/// This index owns no declarations or graph state. It exists only to keep
/// exact-reference resolution single-sourced across edge and path validation.
struct CompositionIndexes<'a> {
    participants: &'a BTreeSet<ParticipantRefV1>,
    responsibilities: BTreeMap<&'a str, &'a ResponsibilityContractV1>,
    provider_bindings: BTreeMap<&'a str, &'a ProviderBindingV1>,
    entries: BTreeMap<&'a str, &'a EntryContractV1>,
    requests: BTreeMap<&'a str, &'a RequestTranslationV1>,
    results: BTreeMap<&'a str, &'a ResultTranslationV1>,
    terminations: BTreeMap<&'a str, &'a TerminationContractV1>,
    settlements: BTreeMap<&'a str, &'a SettlementContractV1>,
    continuities: BTreeMap<&'a str, &'a ContinuityContractV1>,
    edges: BTreeMap<&'a str, &'a CompositionEdgeV1>,
}

/// Resolved declarations governing one selected path.
///
/// Resolution closes the compiler-owned references checked by this phase and
/// its structural bounds before traversal.
/// The borrowed state cannot outlive the immutable normalized source and is
/// discarded after the path proof is projected.
struct ResolvedPath<'a> {
    path: &'a ExperiencePathV1,
    entry: &'a EntryContractV1,
    termination: &'a TerminationContractV1,
    responsibilities: Vec<&'a ResponsibilityContractV1>,
    responsibility_ids: BTreeSet<&'a str>,
    provider_ids: BTreeSet<&'a str>,
}

/// Reachability and exact-join state produced by ordered path traversal.
///
/// Work edges and invocations are noncommutative sequences: changing their
/// order can change the endpoint or disconnect a call. The sets here support
/// membership and uniqueness checks only; they never replace those sequences.
struct PathConnectivity<'a> {
    participants: BTreeSet<&'a ParticipantRefV1>,
    work_edge_ids: BTreeSet<&'a str>,
    invoked_edge_ids: BTreeSet<&'a str>,
    required_responsibility_ids: BTreeSet<&'a str>,
    invocations_by_identity: BTreeMap<&'a str, &'a PathInvocationV1>,
}

/// Opaque witness that every path-local admission invariant closed.
///
/// The witness contains only the public reason projection that sealing already
/// requires. Exact references, reachability sets, budget accumulators, and
/// closure sets remain transient validation state rather than a parallel IR.
/// Construction proves compiler admission only; it does not prove runtime
/// readiness, a native occurrence, an effect, or external receipt truth.
struct ValidatedPathProof {
    reason_path: CompositionReasonPathV1,
}

impl ValidatedPathProof {
    /// Project one admitted path into its deterministic causal explanation.
    fn from_admitted(path: &ExperiencePathV1) -> Self {
        let capacity = 5
            + path.responsibility_contract_ids.len()
            + path.provider_binding_ids.len()
            + path.work_delegation_edge_ids.len();
        let mut reasons = Vec::with_capacity(capacity);
        reasons.extend([
            CompositionReasonStepV1::OriginReceipt {
                contract_id: path.origin_receipt_contract_id.clone(),
            },
            CompositionReasonStepV1::EntryContract {
                contract_id: path.entry_contract_id.clone(),
            },
        ]);
        reasons.extend(
            path.responsibility_contract_ids
                .iter()
                .cloned()
                .map(|contract_id| CompositionReasonStepV1::ResponsibilityContract { contract_id }),
        );
        reasons.extend(
            path.provider_binding_ids
                .iter()
                .cloned()
                .map(|binding_id| CompositionReasonStepV1::ProviderBinding { binding_id }),
        );
        reasons.extend(
            path.work_delegation_edge_ids
                .iter()
                .cloned()
                .map(|edge_id| CompositionReasonStepV1::DelegationEdge { edge_id }),
        );
        reasons.extend([
            CompositionReasonStepV1::TerminationContract {
                contract_id: path.termination_contract_id.clone(),
            },
            CompositionReasonStepV1::SettlementContract {
                contract_id: path.settlement_contract_id.clone(),
            },
            CompositionReasonStepV1::ContinuityContract {
                contract_id: path.continuity_contract_id.clone(),
            },
        ]);
        Self {
            reason_path: CompositionReasonPathV1 {
                path_id: path.id.clone(),
                reasons,
            },
        }
    }
}

/// Ordered proofs for every normalized selected path in one composition.
///
/// The collection is created only after all paths prove successfully. A
/// failure therefore exposes neither a linked composition nor a valid-prefix
/// explanation. Its order is the normalized selected-path order.
pub(super) struct ValidatedCompositionPaths {
    reason_paths: Vec<CompositionReasonPathV1>,
}

impl ValidatedCompositionPaths {
    /// Consume the proof-custody collection into the public plan projection.
    pub(super) fn into_reason_paths(self) -> Vec<CompositionReasonPathV1> {
        self.reason_paths
    }
}

impl<'a> CompositionIndexes<'a> {
    fn new(source: &'a CompositionSourceV1, participants: &'a BTreeSet<ParticipantRefV1>) -> Self {
        Self {
            participants,
            responsibilities: index(&source.responsibility_contracts, |record| &record.id),
            provider_bindings: index(&source.provider_bindings, |record| &record.id),
            entries: index(&source.entry_contracts, |record| &record.id),
            requests: index(&source.request_translations, |record| &record.id),
            results: index(&source.result_translations, |record| &record.id),
            terminations: index(&source.termination_contracts, |record| &record.id),
            settlements: index(&source.settlement_contracts, |record| &record.id),
            continuities: index(&source.continuity_contracts, |record| &record.id),
            edges: index(&source.edge_contracts, CompositionEdgeV1::id),
        }
    }
}

/// Validate exact contract references, insert intrinsically typed edges, and
/// verify each selected sparse work path.
pub(super) fn validate_edges_and_paths(
    source: &CompositionSourceV1,
    participants: &BTreeSet<ParticipantRefV1>,
    graphs: &mut CompositionGraphs,
) -> Result<ValidatedCompositionPaths, CompositionError> {
    let indexes = CompositionIndexes::new(source, participants);
    validate_identity_uniqueness(source)?;

    for responsibility in &source.responsibility_contracts {
        require_participant(
            "responsibility contract",
            &responsibility.id,
            &responsibility.participant,
            indexes.participants,
        )?;
        for typed_responsibility in &responsibility.responsibilities {
            if let ResponsibilityV1::Extension {
                contract_schema_identity,
                ..
            } = typed_responsibility
            {
                require_admitted_schema_member(contract_schema_identity)?;
            }
        }
    }
    for entry in &source.entry_contracts {
        require_participant(
            "entry contract",
            &entry.id,
            &entry.entry,
            indexes.participants,
        )?;
        let responsibility = require_ref(
            "entry contract",
            &entry.id,
            "responsibility contract",
            &entry.responsibility_contract_id,
            &indexes.responsibilities,
        )?;
        if responsibility.participant != entry.entry {
            return Err(invalid_edge(
                &entry.id,
                "entry participant does not own its responsibility contract",
            ));
        }
    }
    for request in &source.request_translations {
        require_participant(
            "request translation",
            &request.id,
            &request.from,
            indexes.participants,
        )?;
        require_participant(
            "request translation",
            &request.id,
            &request.to,
            indexes.participants,
        )?;
    }
    for result in &source.result_translations {
        require_participant(
            "result translation",
            &result.id,
            &result.from,
            indexes.participants,
        )?;
        require_participant(
            "result translation",
            &result.id,
            &result.to,
            indexes.participants,
        )?;
    }
    for termination in &source.termination_contracts {
        require_participant(
            "termination contract",
            &termination.id,
            &termination.owner,
            indexes.participants,
        )?;
        if let Some(leaf) = &termination.terminal_leaf {
            require_participant(
                "termination contract",
                &termination.id,
                leaf,
                indexes.participants,
            )?;
            if !matches!(leaf, ParticipantRefV1::TerminalLeaf { .. }) {
                return Err(invalid_edge(
                    &termination.id,
                    "terminal-leaf reference is not a terminal-leaf participant",
                ));
            }
        }
    }
    for settlement in &source.settlement_contracts {
        require_participant(
            "settlement contract",
            &settlement.id,
            &settlement.observer,
            indexes.participants,
        )?;
    }
    for continuity in &source.continuity_contracts {
        require_participant(
            "continuity contract",
            &continuity.id,
            &continuity.owner,
            indexes.participants,
        )?;
    }
    for binding in &source.provider_bindings {
        require_participant(
            "provider binding",
            &binding.id,
            &binding.participant,
            indexes.participants,
        )?;
        if binding
            .recompute_identity()
            .map_err(CompositionError::Canonicalization)?
            != binding.binding_identity
        {
            return Err(invalid_provider_binding(
                &binding.id,
                ProviderBindingViolation::BindingIdentityMismatch,
            ));
        }
        if binding
            .recompute_deployment_activation_identity()
            .map_err(CompositionError::Canonicalization)?
            != binding
                .generation_coordinates
                .deployment_activation_identity
        {
            return Err(invalid_provider_binding(
                &binding.id,
                ProviderBindingViolation::DeploymentIdentityMismatch,
            ));
        }
        let base_schema = admitted_schema_member(RESPONSIBILITY_CONTRACT_V1_SCHEMA_ID)?;
        let mut expected_schemas = BTreeSet::from([base_schema]);
        if let ResponsibilityV1::Extension {
            contract_schema_identity,
            ..
        } = &binding.responsibility
        {
            require_admitted_schema_member(contract_schema_identity)?;
            expected_schemas.insert(contract_schema_identity.clone());
        }
        for schema_identity in &binding.responsibility_schema_identities {
            require_admitted_schema_member(schema_identity)?;
        }
        let actual_schemas = binding
            .responsibility_schema_identities
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        if actual_schemas != expected_schemas {
            return Err(invalid_provider_binding(
                &binding.id,
                ProviderBindingViolation::ResponsibilitySchemaSetMismatch,
            ));
        }
        for responsibility_id in &binding.responsibility_contract_ids {
            let responsibility = require_ref(
                "provider binding",
                &binding.id,
                "responsibility contract",
                responsibility_id,
                &indexes.responsibilities,
            )?;
            if responsibility.participant != binding.participant {
                return Err(invalid_provider_binding(
                    &binding.id,
                    ProviderBindingViolation::ParticipantOwnershipMismatch,
                ));
            }
            if !responsibility
                .responsibilities
                .contains(&binding.responsibility)
            {
                return Err(invalid_provider_binding(
                    &binding.id,
                    ProviderBindingViolation::ResponsibilityNotGoverned,
                ));
            }
        }
    }

    for edge in &source.edge_contracts {
        indexes.validate_edge(edge)?;
        let (from, to) = edge.endpoints();
        graphs.insert(edge.kind(), edge.id().to_string(), from.clone(), to.clone());
    }

    let mut reason_paths = Vec::with_capacity(source.selected_experience_paths.len());
    for path in &source.selected_experience_paths {
        let proof = indexes.prove_path(path)?;
        reason_paths.push(proof.reason_path);
    }
    Ok(ValidatedCompositionPaths { reason_paths })
}

/// Reject identity reuse without inferring or recomputing owner-provided truth.
fn validate_identity_uniqueness(source: &CompositionSourceV1) -> Result<(), CompositionError> {
    reject_duplicate_identity(
        "provider binding identity",
        source
            .provider_bindings
            .iter()
            .map(|binding| &binding.binding_identity.provider_binding_sha256),
    )?;
    reject_duplicate_identity(
        "responsibility path identity",
        source
            .selected_experience_paths
            .iter()
            .map(|path| &path.identity.responsibility_path_sha256),
    )?;
    reject_duplicate_identity(
        "invocation identity",
        source
            .selected_experience_paths
            .iter()
            .flat_map(|path| &path.invocations)
            .map(|invocation| &invocation.identity.invocation_sha256),
    )?;
    reject_duplicate_identity(
        "native effect identity",
        source
            .selected_experience_paths
            .iter()
            .flat_map(|path| &path.native_outcomes)
            .filter_map(|outcome| {
                outcome
                    .native_effect_identity()
                    .map(|identity| &identity.native_effect_sha256)
            }),
    )?;
    reject_duplicate_identity(
        "evidence settlement identity",
        source
            .settlement_contracts
            .iter()
            .map(|settlement| &settlement.identity.evidence_settlement_sha256),
    )?;
    reject_duplicate_identity(
        "continuity identity",
        source
            .continuity_contracts
            .iter()
            .map(|continuity| &continuity.identity.continuity_sha256),
    )?;

    let mut compiled_sources = BTreeMap::new();
    for path in &source.selected_experience_paths {
        let compiled = path
            .compiled_content
            .compiled_content_identity
            .compiled_content_sha256
            .as_str();
        let source_identity = path
            .compiled_content
            .source_content_identity
            .source_content_sha256
            .as_str();
        if compiled_sources
            .insert(compiled, source_identity)
            .is_some_and(|existing| existing != source_identity)
        {
            return Err(invalid_path(
                &path.id,
                "one compiled-content identity claims more than one source-content identity",
            ));
        }
    }
    Ok(())
}

/// Reject a repeated digest only inside the exact semantic identity family.
fn reject_duplicate_identity<'a>(
    kind: &'static str,
    identities: impl IntoIterator<Item = &'a ArtifactDigest>,
) -> Result<(), CompositionError> {
    let mut seen = BTreeSet::new();
    for identity in identities {
        if !seen.insert(identity.as_str()) {
            return Err(CompositionError::DuplicateId {
                kind,
                id: identity.to_string(),
            });
        }
    }
    Ok(())
}

/// Intersect governing effect ceilings without constructing a generic policy lattice.
fn effect_ceiling_meet<'a>(
    ceilings: impl IntoIterator<Item = &'a [EffectClass]>,
) -> BTreeSet<EffectClass> {
    let mut ceilings = ceilings.into_iter();
    let Some(first) = ceilings.next() else {
        return BTreeSet::new();
    };
    let mut admitted = first.iter().cloned().collect::<BTreeSet<_>>();
    for ceiling in ceilings {
        admitted.retain(|effect| ceiling.contains(effect));
    }
    admitted
}

impl<'a> CompositionIndexes<'a> {
    /// Validate the semantic contract fixed by one edge variant.
    fn validate_edge(&self, edge: &CompositionEdgeV1) -> Result<(), CompositionError> {
        let edge_id = edge.id();
        let (from, to) = edge.endpoints();
        require_participant("composition edge", edge_id, from, self.participants)?;
        require_participant("composition edge", edge_id, to, self.participants)?;

        match edge {
            CompositionEdgeV1::WorkDelegation {
                from,
                to,
                responsibility_contract_id,
                request_translation_id,
                result_translation_id,
                authority_ceiling,
                budget,
                ..
            } => {
                let responsibility = require_ref(
                    "work-delegation edge",
                    edge_id,
                    "responsibility contract",
                    responsibility_contract_id,
                    &self.responsibilities,
                )?;
                if responsibility.participant != *from
                    || authority_ceiling
                        .iter()
                        .any(|effect| !responsibility.authority_ceiling.contains(effect))
                {
                    return Err(CompositionError::Delegation {
                        delegation: edge_id.to_string(),
                        reason:
                            "delegation exceeds or changes the responsibility owner's authority ceiling"
                                .into(),
                    });
                }
                self.validate_translation_pair(
                    edge_id,
                    true,
                    from,
                    to,
                    request_translation_id,
                    result_translation_id,
                )?;
                if !budget_is_valid(budget) {
                    return Err(CompositionError::Delegation {
                        delegation: edge_id.to_string(),
                        reason: "all convergence and resource budgets must be finite and positive"
                            .into(),
                    });
                }
            }
            CompositionEdgeV1::UiEmbedding { host, embedded, .. } => {
                if !matches!(host, ParticipantRefV1::Interface { .. })
                    || !matches!(embedded, ParticipantRefV1::Interface { .. })
                {
                    return Err(invalid_edge(
                        edge_id,
                        "UI embedding requires interface participants at both endpoints",
                    ));
                }
            }
            CompositionEdgeV1::PluginDependency {
                plugin, dependency, ..
            } => {
                if !matches!(plugin, ParticipantRefV1::Plugin { .. })
                    || !matches!(dependency, ParticipantRefV1::Plugin { .. })
                {
                    return Err(invalid_edge(
                        edge_id,
                        "plugin dependency requires plugin participants at both endpoints",
                    ));
                }
            }
            CompositionEdgeV1::NativeApplicationCall {
                caller,
                application,
                provider_binding_id,
                request_translation_id,
                result_translation_id,
                ..
            } => {
                if !matches!(application, ParticipantRefV1::Application { .. }) {
                    return Err(invalid_edge(
                        edge_id,
                        "native application call destination is not an application participant",
                    ));
                }
                let binding = require_ref(
                    "native-application-call edge",
                    edge_id,
                    "provider binding",
                    provider_binding_id,
                    &self.provider_bindings,
                )?;
                if binding.participant != *caller {
                    return Err(invalid_edge(
                        edge_id,
                        "native application caller does not own the selected provider binding",
                    ));
                }
                self.validate_translation_pair(
                    edge_id,
                    false,
                    caller,
                    application,
                    request_translation_id,
                    result_translation_id,
                )?;
            }
            CompositionEdgeV1::TerminalLeaf {
                owner,
                terminal_leaf,
                termination_contract_id,
                ..
            } => {
                if !matches!(terminal_leaf, ParticipantRefV1::TerminalLeaf { .. }) {
                    return Err(invalid_edge(
                        edge_id,
                        "terminal edge destination is not a terminal-leaf participant",
                    ));
                }
                let termination = require_ref(
                    "terminal-leaf edge",
                    edge_id,
                    "termination contract",
                    termination_contract_id,
                    &self.terminations,
                )?;
                if termination.owner != *owner
                    || termination.terminal_leaf.as_ref() != Some(terminal_leaf)
                {
                    return Err(invalid_edge(
                        edge_id,
                        "terminal edge does not match its owner and terminal-leaf contract",
                    ));
                }
            }
            CompositionEdgeV1::ObservationSettlement {
                observer,
                settlement_contract_id,
                ..
            } => {
                let settlement = require_ref(
                    "observation-settlement edge",
                    edge_id,
                    "settlement contract",
                    settlement_contract_id,
                    &self.settlements,
                )?;
                if settlement.observer != *observer {
                    return Err(invalid_edge(
                        edge_id,
                        "observation edge does not terminate at its settlement observer",
                    ));
                }
            }
        }
        Ok(())
    }

    /// Prove one selected path without treating non-work edges as work traversal.
    ///
    /// Success returns the complete [`ValidatedPathProof`]. Any failed
    /// reference, connectivity, budget, closure, or invocation/outcome join
    /// returns an error and no witness.
    fn prove_path(
        &'a self,
        path: &'a ExperiencePathV1,
    ) -> Result<ValidatedPathProof, CompositionError> {
        let resolved = self.resolve_path(path)?;
        let connectivity = self.prove_connectivity(&resolved)?;
        self.validate_path_closure(&resolved, connectivity)?;
        Ok(ValidatedPathProof::from_admitted(path))
    }

    /// Resolve compiler-owned path references and reject structural bounds.
    ///
    /// This phase performs no traversal. Its output is a finite borrowed
    /// environment for [`Self::prove_connectivity`].
    fn resolve_path(
        &'a self,
        path: &'a ExperiencePathV1,
    ) -> Result<ResolvedPath<'a>, CompositionError> {
        if path
            .recompute_identity()
            .map_err(CompositionError::Canonicalization)?
            != path.identity
        {
            return Err(invalid_path(
                &path.id,
                "claimed responsibility-path identity does not match the complete canonical path body",
            ));
        }
        validate_budget(&path.id, &path.budget)?;
        let entry = require_ref(
            "path",
            &path.id,
            "entry contract",
            &path.entry_contract_id,
            &self.entries,
        )?;
        let termination = require_ref(
            "path",
            &path.id,
            "termination contract",
            &path.termination_contract_id,
            &self.terminations,
        )?;
        require_ref(
            "path",
            &path.id,
            "settlement contract",
            &path.settlement_contract_id,
            &self.settlements,
        )?;
        require_ref(
            "path",
            &path.id,
            "continuity contract",
            &path.continuity_contract_id,
            &self.continuities,
        )?;
        let responsibilities = path
            .responsibility_contract_ids
            .iter()
            .map(|id| {
                require_ref(
                    "path",
                    &path.id,
                    "responsibility contract",
                    id,
                    &self.responsibilities,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let responsibility_ids = path
            .responsibility_contract_ids
            .iter()
            .map(String::as_str)
            .collect();
        let provider_ids = path
            .provider_binding_ids
            .iter()
            .map(String::as_str)
            .collect();
        if path.work_delegation_edge_ids.len() > path.budget.max_depth as usize
            || path.work_delegation_edge_ids.len() > path.budget.max_recursion_iterations as usize
        {
            return Err(invalid_path(
                &path.id,
                "work-delegation depth or recursion iterations exceed the path budget",
            ));
        }
        Ok(ResolvedPath {
            path,
            entry,
            termination,
            responsibilities,
            responsibility_ids,
            provider_ids,
        })
    }

    /// Fold ordered work and invocation steps into a reachability witness.
    ///
    /// Budget accumulation is additive, but traversal remains ordered because
    /// each edge must begin at the preceding endpoint.
    fn prove_connectivity(
        &'a self,
        resolved: &ResolvedPath<'a>,
    ) -> Result<PathConnectivity<'a>, CompositionError> {
        let path = resolved.path;
        let mut required_responsibility_ids =
            BTreeSet::from([resolved.entry.responsibility_contract_id.as_str()]);

        let mut seen_work_edges = BTreeSet::new();
        let mut current = &resolved.entry.entry;
        let mut selected_participants = BTreeSet::from([current]);
        let mut declared_duration_ms = 0_u128;
        let mut cost = 0_u128;
        let mut resources = 0_u128;
        let mut effects = 0_u128;
        for edge_id in &path.work_delegation_edge_ids {
            let edge = require_ref("path", &path.id, "composition edge", edge_id, &self.edges)?;
            let CompositionEdgeV1::WorkDelegation {
                from,
                to,
                responsibility_contract_id,
                authority_ceiling,
                budget,
                ..
            } = edge
            else {
                return Err(invalid_path(
                    &path.id,
                    format!("edge `{edge_id}` is not a work-delegation edge"),
                ));
            };
            if !seen_work_edges.insert(edge_id.as_str()) {
                return Err(invalid_path(
                    &path.id,
                    "work traversal repeats an edge contract",
                ));
            }
            if from != current {
                return Err(invalid_path(
                    &path.id,
                    format!(
                        "work edge `{edge_id}` does not continue from `{}`",
                        participant_id(current)
                    ),
                ));
            }
            if !resolved
                .responsibility_ids
                .contains(responsibility_contract_id.as_str())
            {
                return Err(invalid_path(
                    &path.id,
                    format!(
                        "work edge `{edge_id}` uses a responsibility contract absent from the path"
                    ),
                ));
            }
            required_responsibility_ids.insert(responsibility_contract_id.as_str());
            if path
                .effect_ceiling
                .iter()
                .any(|effect| !authority_ceiling.contains(effect))
            {
                return Err(invalid_path(
                    &path.id,
                    format!("path effect ceiling exceeds work edge `{edge_id}`"),
                ));
            }
            if budget.max_depth > path.budget.max_depth
                || budget.max_fan_out > path.budget.max_fan_out
                || budget.max_recursion_iterations > path.budget.max_recursion_iterations
            {
                return Err(invalid_path(
                    &path.id,
                    format!("work edge `{edge_id}` exceeds the path structural budget"),
                ));
            }
            declared_duration_ms += u128::from(budget.max_time_ms);
            cost += u128::from(budget.max_cost_units);
            resources += u128::from(budget.max_resource_units);
            effects += u128::from(budget.max_effect_count);
            selected_participants.insert(from);
            selected_participants.insert(to);
            current = to;
        }
        if declared_duration_ms > u128::from(path.budget.max_time_ms)
            || cost > u128::from(path.budget.max_cost_units)
            || resources > u128::from(path.budget.max_resource_units)
            || effects > u128::from(path.budget.max_effect_count)
        {
            return Err(invalid_path(
                &path.id,
                "selected work-edge totals exceed the path budget",
            ));
        }
        if &resolved.termination.owner != current {
            return Err(invalid_path(
                &path.id,
                "termination owner is not the selected work-path endpoint",
            ));
        }

        let mut invoked_edges = BTreeSet::new();
        let mut invocations_by_identity = BTreeMap::new();
        for invocation in &path.invocations {
            let edge = require_ref(
                "path invocation",
                &path.id,
                "composition edge",
                &invocation.edge_id,
                &self.edges,
            )?;
            if matches!(
                edge,
                CompositionEdgeV1::UiEmbedding { .. } | CompositionEdgeV1::PluginDependency { .. }
            ) {
                return Err(invalid_path(
                    &path.id,
                    format!(
                        "non-executable edge `{}` cannot carry an invocation identity",
                        invocation.edge_id
                    ),
                ));
            }
            if matches!(edge, CompositionEdgeV1::WorkDelegation { .. })
                && !seen_work_edges.contains(invocation.edge_id.as_str())
            {
                return Err(invalid_path(
                    &path.id,
                    format!(
                        "invoked work edge `{}` is absent from ordered work traversal",
                        invocation.edge_id
                    ),
                ));
            }
            if let CompositionEdgeV1::NativeApplicationCall {
                provider_binding_id,
                ..
            } = edge
            {
                if !resolved.provider_ids.contains(provider_binding_id.as_str()) {
                    return Err(invalid_path(
                        &path.id,
                        format!(
                            "native-call edge `{}` uses a provider binding absent from the path",
                            invocation.edge_id
                        ),
                    ));
                }
            }
            if let CompositionEdgeV1::TerminalLeaf {
                owner,
                termination_contract_id,
                ..
            } = edge
            {
                if owner != current || termination_contract_id != &path.termination_contract_id {
                    return Err(invalid_path(
                        &path.id,
                        "invoked terminal edge does not close the selected work endpoint and termination contract",
                    ));
                }
            }
            if let CompositionEdgeV1::ObservationSettlement {
                settlement_contract_id,
                ..
            } = edge
            {
                if settlement_contract_id != &path.settlement_contract_id {
                    return Err(invalid_path(
                        &path.id,
                        "invoked observation edge uses a different settlement contract",
                    ));
                }
            }
            let (from, to) = edge.endpoints();
            if !selected_participants.contains(from) {
                return Err(invalid_path(
                    &path.id,
                    format!(
                        "invoked edge `{}` is disconnected from the selected path",
                        invocation.edge_id
                    ),
                ));
            }
            selected_participants.insert(from);
            selected_participants.insert(to);
            invocations_by_identity
                .insert(invocation.identity.invocation_sha256.as_str(), invocation);
            invoked_edges.insert(invocation.edge_id.as_str());
        }

        Ok(PathConnectivity {
            participants: selected_participants,
            work_edge_ids: seen_work_edges,
            invoked_edge_ids: invoked_edges,
            required_responsibility_ids,
            invocations_by_identity,
        })
    }

    /// Close set-like authority, evidence, outcome, and consequence laws.
    ///
    /// Responsibility selection is admitted exactly when the normalized
    /// selected set equals the entry, traversal, and provider-induced closure.
    /// Effect authority is their meet and can therefore only narrow.
    fn validate_path_closure(
        &'a self,
        resolved: &ResolvedPath<'a>,
        mut connectivity: PathConnectivity<'a>,
    ) -> Result<(), CompositionError> {
        let path = resolved.path;

        let mut evidenced_capabilities = BTreeSet::new();
        for binding_id in &path.provider_binding_ids {
            let binding = require_ref(
                "path",
                &path.id,
                "provider binding",
                binding_id,
                &self.provider_bindings,
            )?;
            if !connectivity.participants.contains(&binding.participant) {
                return Err(invalid_path(
                    &path.id,
                    "selected provider binding belongs to a participant outside the connected path",
                ));
            }
            connectivity.required_responsibility_ids.extend(
                binding
                    .responsibility_contract_ids
                    .iter()
                    .map(String::as_str),
            );
            evidenced_capabilities.extend(
                binding
                    .conformance_coordinates
                    .per_capability_evidence
                    .iter()
                    .map(|evidence| evidence.capability_id.as_str()),
            );
        }
        if resolved.responsibility_ids != connectivity.required_responsibility_ids {
            return Err(invalid_path(
                &path.id,
                "selected responsibility contracts do not exactly match the connected path closure",
            ));
        }
        let admitted_effects = effect_ceiling_meet(
            resolved
                .responsibilities
                .iter()
                .map(|contract| contract.authority_ceiling.as_slice()),
        );
        if path
            .effect_ceiling
            .iter()
            .any(|effect| !admitted_effects.contains(effect))
        {
            return Err(invalid_path(
                &path.id,
                "path effect ceiling exceeds its selected responsibility contracts",
            ));
        }
        if path
            .capability_ceiling
            .iter()
            .any(|capability| !evidenced_capabilities.contains(capability.as_str()))
        {
            return Err(invalid_path(
                &path.id,
                "path capability ceiling lacks evidence from its selected provider bindings",
            ));
        }
        let mut transitions = BTreeSet::new();
        for transition in &path.native_owner_transitions {
            require_ref(
                "native-owner transition",
                &path.id,
                "composition edge",
                &transition.edge_id,
                &self.edges,
            )?;
            if !connectivity
                .work_edge_ids
                .contains(transition.edge_id.as_str())
                && !connectivity
                    .invoked_edge_ids
                    .contains(transition.edge_id.as_str())
            {
                return Err(invalid_path(
                    &path.id,
                    format!(
                        "native-owner transition references edge `{}` outside the selected path",
                        transition.edge_id
                    ),
                ));
            }
            if !transitions.insert((
                transition.edge_id.as_str(),
                transition.native_owner_id.as_str(),
            )) {
                return Err(invalid_path(
                    &path.id,
                    "native-owner transition repeats one edge and native owner",
                ));
            }
        }
        for outcome in &path.native_outcomes {
            require_ref(
                "native outcome",
                &path.id,
                "composition edge",
                outcome.edge_id(),
                &self.edges,
            )?;
            let invocation = connectivity
                .invocations_by_identity
                .get(outcome.invocation_identity().invocation_sha256.as_str())
                .copied();
            if invocation.is_none_or(|invocation| invocation.edge_id != outcome.edge_id()) {
                return Err(invalid_path(
                    &path.id,
                    format!(
                        "native outcome references edge `{}` without its exact invocation identity",
                        outcome.edge_id()
                    ),
                ));
            }
        }
        let mut consequences = BTreeSet::new();
        for consequence in &path.destination_consequences {
            require_participant(
                "destination consequence",
                &path.id,
                &consequence.destination,
                self.participants,
            )?;
            if !connectivity.participants.contains(&consequence.destination) {
                return Err(invalid_path(
                    &path.id,
                    format!(
                        "destination `{}` is outside the explicitly selected path",
                        participant_id(&consequence.destination)
                    ),
                ));
            }
            if !consequences.insert((
                &consequence.destination,
                consequence.consequence_contract_id.as_str(),
            )) {
                return Err(invalid_path(
                    &path.id,
                    "destination consequence repeats one participant and contract",
                ));
            }
        }
        let mut reconciliation = BTreeSet::new();
        for requirement in &path.reconciliation {
            if !reconciliation.insert((
                requirement.obligation_contract_id.as_str(),
                requirement.frontier_id.as_str(),
            )) {
                return Err(invalid_path(
                    &path.id,
                    "reconciliation repeats one obligation and frontier",
                ));
            }
        }
        Ok(())
    }

    /// Validate the faithful request/result translation pair for a directional edge.
    fn validate_translation_pair(
        &self,
        edge_id: &str,
        delegation: bool,
        from: &ParticipantRefV1,
        to: &ParticipantRefV1,
        request_translation_id: &str,
        result_translation_id: &str,
    ) -> Result<(), CompositionError> {
        let edge_kind = if delegation {
            "work delegation"
        } else {
            "native application call"
        };
        let request = require_ref(
            edge_kind,
            edge_id,
            "request translation",
            request_translation_id,
            &self.requests,
        )?;
        let result = require_ref(
            edge_kind,
            edge_id,
            "result translation",
            result_translation_id,
            &self.results,
        )?;
        if request.from != *from || request.to != *to || !request.preserves_authority {
            return Err(invalid_directional_edge(
                edge_id,
                delegation,
                "request translation is reversed, mismatched, or escalates authority",
            ));
        }
        if result.from != *to || result.to != *from || !result.preserves_evidence {
            return Err(invalid_directional_edge(
                edge_id,
                delegation,
                "result translation is reversed, mismatched, or drops evidence",
            ));
        }
        Ok(())
    }
}

/// Require an already-declared participant reference.
fn require_participant(
    kind: &'static str,
    owner: &str,
    participant: &ParticipantRefV1,
    participants: &BTreeSet<ParticipantRefV1>,
) -> Result<(), CompositionError> {
    if participants.contains(participant) {
        Ok(())
    } else {
        Err(CompositionError::MissingReference {
            kind,
            owner: owner.to_string(),
            target_kind: "participant",
            target: participant_id(participant).to_string(),
        })
    }
}

/// Index one normalized record family by its stable string identity.
fn index<T>(records: &[T], id: impl Fn(&T) -> &str) -> BTreeMap<&str, &T> {
    records.iter().map(|record| (id(record), record)).collect()
}

/// Resolve one exact reference from a normalized record-family index.
fn require_ref<'a, T>(
    kind: &'static str,
    owner: &str,
    target_kind: &'static str,
    target: &str,
    index: &'a BTreeMap<&str, &'a T>,
) -> Result<&'a T, CompositionError> {
    index
        .get(target)
        .copied()
        .ok_or_else(|| CompositionError::MissingReference {
            kind,
            owner: owner.to_string(),
            target_kind,
            target: target.to_string(),
        })
}

/// Resolve one exact raw schema member from the compiler's admitted registry.
fn admitted_schema_member(
    id: &str,
) -> Result<ContractSchemaIdentityV1, pale_contracts::SchemaError> {
    let descriptor = pale_contracts::schema_descriptor(id)?;
    let schema_id = SchemaId::parse(descriptor.schema_id).map_err(|error| {
        pale_contracts::SchemaError::Integrity(format!(
            "admitted schema `{id}` has an invalid registry identity: {error}"
        ))
    })?;
    let schema_sha256 = SchemaDigest::parse(descriptor.schema_sha256).map_err(|error| {
        pale_contracts::SchemaError::Integrity(format!(
            "admitted schema `{id}` has an invalid raw digest: {error}"
        ))
    })?;
    Ok(ContractSchemaIdentityV1 {
        schema_id,
        schema_sha256,
    })
}

/// Require a claimed contract schema to match one exact admitted schema member.
fn require_admitted_schema_member(
    claimed: &ContractSchemaIdentityV1,
) -> Result<(), CompositionError> {
    let expected = admitted_schema_member(claimed.schema_id.as_str()).map_err(|source| {
        CompositionError::DeclaredSchema {
            schema_id: claimed.schema_id.to_string(),
            source,
        }
    })?;
    if expected == *claimed {
        Ok(())
    } else {
        Err(CompositionError::DeclaredSchema {
            schema_id: claimed.schema_id.to_string(),
            source: pale_contracts::SchemaError::IdentityMismatch(claimed.schema_id.to_string()),
        })
    }
}

/// Reject zero convergence/resource budgets while allowing an explicit
/// zero-effect path.
fn validate_budget(
    path: &str,
    budget: &pale_core::records::PathBudgetV1,
) -> Result<(), CompositionError> {
    if !budget_is_valid(budget) {
        return Err(invalid_path(
            path,
            "all convergence and resource budgets must be finite and positive",
        ));
    }
    Ok(())
}

/// Return whether every convergence/resource dimension is finite and positive.
fn budget_is_valid(budget: &pale_core::records::PathBudgetV1) -> bool {
    budget.max_depth > 0
        && budget.max_fan_out > 0
        && budget.max_time_ms > 0
        && budget.max_cost_units > 0
        && budget.max_resource_units > 0
        && budget.max_recursion_iterations > 0
}

/// Preserve the separate work-delegation and non-delegation error contracts.
fn invalid_directional_edge(
    edge: &str,
    delegation: bool,
    reason: &'static str,
) -> CompositionError {
    if delegation {
        CompositionError::Delegation {
            delegation: edge.to_string(),
            reason: reason.into(),
        }
    } else {
        invalid_edge(edge, reason)
    }
}

/// Build a non-delegation edge-contract diagnostic.
fn invalid_edge(edge: &str, reason: impl Into<String>) -> CompositionError {
    CompositionError::Edge {
        edge: edge.to_string(),
        reason: reason.into(),
    }
}

/// Build a provider-binding diagnostic without implying invocation readiness.
fn invalid_provider_binding(
    binding: &str,
    violation: ProviderBindingViolation,
) -> CompositionError {
    CompositionError::ProviderBinding {
        binding: binding.to_string(),
        violation,
    }
}

/// Build a selected-path diagnostic.
fn invalid_path(path: &str, reason: impl Into<String>) -> CompositionError {
    CompositionError::Path {
        path: path.to_string(),
        reason: reason.into(),
    }
}

#[cfg(test)]
#[path = "../../tests/internal/composition/path.rs"]
mod tests;
