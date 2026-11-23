//! Bounded deterministic normalization of explicit composition declarations.

use super::{CompositionError, CompositionSourceV1, MAX_SELECTED_PATHS, MAX_SEMANTIC_DECLARATIONS};

pub(crate) fn normalize_source(source: &mut CompositionSourceV1) -> Result<(), CompositionError> {
    enforce_limit(
        "selected paths",
        source.selected_experience_paths.len(),
        MAX_SELECTED_PATHS,
    )?;
    for (kind, count) in [
        (
            "responsibility contracts",
            source.responsibility_contracts.len(),
        ),
        (
            "participant manifests",
            source.participant_provider_manifests.len(),
        ),
        ("provider bindings", source.provider_bindings.len()),
        ("entry contracts", source.entry_contracts.len()),
        ("request translations", source.request_translations.len()),
        ("result translations", source.result_translations.len()),
        ("termination contracts", source.termination_contracts.len()),
        ("settlement contracts", source.settlement_contracts.len()),
        ("continuity contracts", source.continuity_contracts.len()),
        ("composition edges", source.edge_contracts.len()),
    ] {
        enforce_limit(kind, count, MAX_SEMANTIC_DECLARATIONS)?;
    }
    enforce_aggregate_declaration_limit(source)?;

    sort_and_reject_duplicates(
        &mut source.responsibility_contracts,
        "responsibility contract",
        |record| &record.id,
    )?;
    sort_and_reject_duplicates(
        &mut source.participant_provider_manifests,
        "participant manifest",
        |record| &record.id,
    )?;
    sort_and_reject_duplicates(
        &mut source.provider_bindings,
        "provider binding",
        |record| &record.id,
    )?;
    sort_and_reject_duplicates(&mut source.entry_contracts, "entry contract", |record| {
        &record.id
    })?;
    sort_and_reject_duplicates(
        &mut source.request_translations,
        "request translation",
        |record| &record.id,
    )?;
    sort_and_reject_duplicates(
        &mut source.result_translations,
        "result translation",
        |record| &record.id,
    )?;
    sort_and_reject_duplicates(
        &mut source.termination_contracts,
        "termination contract",
        |record| &record.id,
    )?;
    sort_and_reject_duplicates(
        &mut source.settlement_contracts,
        "settlement contract",
        |record| &record.id,
    )?;
    sort_and_reject_duplicates(
        &mut source.continuity_contracts,
        "continuity contract",
        |record| &record.id,
    )?;
    sort_and_reject_duplicates(
        &mut source.edge_contracts,
        "composition edge",
        pale_core::records::CompositionEdgeV1::id,
    )?;
    sort_and_reject_duplicates(
        &mut source.selected_experience_paths,
        "experience path",
        |record| &record.id,
    )?;
    normalize_set_members(source)
}

fn enforce_limit(kind: &'static str, count: usize, limit: usize) -> Result<(), CompositionError> {
    if count > limit {
        Err(CompositionError::DeclarationLimit { kind })
    } else {
        Ok(())
    }
}

fn enforce_aggregate_declaration_limit(
    source: &CompositionSourceV1,
) -> Result<(), CompositionError> {
    let nested = source
        .participant_provider_manifests
        .iter()
        .flat_map(|manifest| &manifest.providers)
        .fold(0_usize, |count, provider| {
            count
                .saturating_add(1)
                .saturating_add(provider.compatible_provider_ids.len())
        });
    let path_edges = source
        .selected_experience_paths
        .iter()
        .fold(0_usize, |count, path| {
            count
                .saturating_add(path.work_delegation_edge_ids.len())
                .saturating_add(path.responsibility_contract_ids.len())
                .saturating_add(path.provider_binding_ids.len())
                .saturating_add(path.capability_ceiling.len())
                .saturating_add(path.effect_ceiling.len())
                .saturating_add(path.invocations.len())
                .saturating_add(path.native_owner_transitions.len())
                .saturating_add(path.native_outcomes.len())
                .saturating_add(path.destination_consequences.len())
                .saturating_add(path.reconciliation.len())
        });
    let binding_members = source
        .provider_bindings
        .iter()
        .fold(0_usize, |count, binding| {
            count
                .saturating_add(binding.responsibility_contract_ids.len())
                .saturating_add(binding.responsibility_schema_identities.len())
                .saturating_add(binding.protocol_coordinates.extensions.len())
                .saturating_add(
                    binding
                        .conformance_coordinates
                        .per_capability_evidence
                        .len(),
                )
        });
    let top_level = [
        source.responsibility_contracts.len(),
        source.participant_provider_manifests.len(),
        source.provider_bindings.len(),
        source.entry_contracts.len(),
        source.request_translations.len(),
        source.result_translations.len(),
        source.termination_contracts.len(),
        source.settlement_contracts.len(),
        source.continuity_contracts.len(),
        source.edge_contracts.len(),
        source.selected_experience_paths.len(),
    ]
    .into_iter()
    .fold(0_usize, usize::saturating_add);
    enforce_limit(
        "aggregate declarations and explicit edges",
        top_level
            .saturating_add(nested)
            .saturating_add(binding_members)
            .saturating_add(path_edges),
        MAX_SEMANTIC_DECLARATIONS,
    )
}

fn sort_and_reject_duplicates<T>(
    records: &mut [T],
    kind: &'static str,
    id: impl Fn(&T) -> &str,
) -> Result<(), CompositionError> {
    records.sort_by(|left, right| id(left).cmp(id(right)));
    for pair in records.windows(2) {
        if id(&pair[0]) == id(&pair[1]) {
            return Err(CompositionError::DuplicateId {
                kind,
                id: id(&pair[0]).to_string(),
            });
        }
    }
    Ok(())
}

fn normalize_set_members(source: &mut CompositionSourceV1) -> Result<(), CompositionError> {
    for responsibility in &mut source.responsibility_contracts {
        sort_unique_set(
            &mut responsibility.responsibilities,
            "responsibility",
            &responsibility.id,
        )?;
        sort_unique_set(
            &mut responsibility.authority_ceiling,
            "responsibility authority ceiling",
            &responsibility.id,
        )?;
    }
    for manifest in &mut source.participant_provider_manifests {
        sort_unique_set(&mut manifest.participants, "participant", &manifest.id)?;
        manifest
            .providers
            .sort_by(|left, right| left.id.cmp(&right.id));
        for provider in &mut manifest.providers {
            sort_unique_set(
                &mut provider.capability_ids,
                "provider capability",
                &provider.id,
            )?;
            sort_unique_set(
                &mut provider.compatible_provider_ids,
                "provider compatibility",
                &provider.id,
            )?;
        }
    }
    for binding in &mut source.provider_bindings {
        sort_unique_set(
            &mut binding.responsibility_contract_ids,
            "provider responsibility contract",
            &binding.id,
        )?;
        sort_unique_set(
            &mut binding.responsibility_schema_identities,
            "provider responsibility schema",
            &binding.id,
        )?;
        sort_unique_set(
            &mut binding.protocol_coordinates.extensions,
            "provider protocol extension",
            &binding.id,
        )?;
        binding
            .conformance_coordinates
            .per_capability_evidence
            .sort_by(|left, right| left.capability_id.cmp(&right.capability_id));
        if binding
            .conformance_coordinates
            .per_capability_evidence
            .windows(2)
            .any(|pair| pair[0].capability_id == pair[1].capability_id)
        {
            return Err(CompositionError::DuplicateId {
                kind: "provider capability evidence",
                id: binding.id.clone(),
            });
        }
    }
    for termination in &mut source.termination_contracts {
        sort_unique_set(&mut termination.kinds, "termination kind", &termination.id)?;
    }
    for continuity in &mut source.continuity_contracts {
        sort_unique_set(
            &mut continuity.obligations,
            "continuity obligation",
            &continuity.id,
        )?;
    }
    for edge in &mut source.edge_contracts {
        if let pale_core::records::CompositionEdgeV1::WorkDelegation {
            id,
            authority_ceiling,
            ..
        } = edge
        {
            sort_unique_set(authority_ceiling, "delegation authority ceiling", id)?;
        }
    }
    for path in &mut source.selected_experience_paths {
        sort_unique_set(
            &mut path.responsibility_contract_ids,
            "path responsibility contract",
            &path.id,
        )?;
        sort_unique_set(
            &mut path.provider_binding_ids,
            "path provider binding",
            &path.id,
        )?;
        sort_unique_set(
            &mut path.capability_ceiling,
            "path capability ceiling",
            &path.id,
        )?;
        sort_unique_set(&mut path.effect_ceiling, "path effect ceiling", &path.id)?;
    }
    Ok(())
}

fn sort_unique_set<T: Ord>(
    values: &mut [T],
    kind: &'static str,
    owner: &str,
) -> Result<(), CompositionError> {
    values.sort();
    if values.windows(2).any(|pair| pair[0] == pair[1]) {
        Err(CompositionError::DuplicateId {
            kind,
            id: owner.to_string(),
        })
    } else {
        Ok(())
    }
}
