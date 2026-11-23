//! Pure validation and lowering for attributable responsibility derivations.

use std::collections::BTreeSet;

use pale_core::records::*;

/// The identity used by responsibility admission and retained-record comparison.
pub(crate) fn record_identity(value: &ResponsibilityDerivationV1) -> &ResponsibilityDerivationId {
    &value.derivation_id
}

/// Deterministic rejection from responsibility-derivation lowering.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ResponsibilityDerivationError {
    /// The record does not identify the initial wire contract.
    #[error("responsibility derivation `{derivation}` has an invalid schema coordinate")]
    SchemaCoordinate {
        /// Invalid derivation.
        derivation: ResponsibilityDerivationId,
    },
    /// A bounded collection exceeds its initial-v1 ceiling.
    #[error("responsibility derivation `{derivation}` exceeds the {kind} limit")]
    Limit {
        /// Invalid derivation.
        derivation: ResponsibilityDerivationId,
        /// Bounded collection.
        kind: &'static str,
    },
    /// One identity repeats where a logical set is required.
    #[error("responsibility derivation `{derivation}` repeats {kind}")]
    Duplicate {
        /// Invalid derivation.
        derivation: ResponsibilityDerivationId,
        /// Repeated identity family.
        kind: &'static str,
    },
    /// The exhaustive structural-question inventory is incomplete.
    #[error("responsibility derivation `{derivation}` does not answer every structural question")]
    StructuralInventory {
        /// Invalid derivation.
        derivation: ResponsibilityDerivationId,
    },
    /// A directional service lacks its explicit structural selection.
    #[error(
        "responsibility derivation `{derivation}` selects a directional service without its contract object"
    )]
    MissingDirectionalServiceContract {
        /// Invalid derivation.
        derivation: ResponsibilityDerivationId,
    },
    /// A directional placement and its structural answer name different contracts.
    #[error(
        "responsibility derivation `{derivation}` selects service `{expected}` but its structural answer names `{selected}`"
    )]
    DirectionalServiceContractMismatch {
        /// Invalid derivation.
        derivation: ResponsibilityDerivationId,
        /// Namespace-qualified placement contract.
        expected: Box<DirectionalServiceContractRefV1>,
        /// Conflicting structural resource coordinate.
        selected: WorkspaceResourceId,
    },
    /// A directional placement points from one logical scope back to itself.
    #[error(
        "responsibility derivation `{derivation}` selects a reflexive directional service at `{scope}`"
    )]
    ReflexiveDirectionalService {
        /// Invalid derivation.
        derivation: ResponsibilityDerivationId,
        /// Repeated provider and consumer scope.
        scope: WorkspaceResourceId,
    },
    /// A nested policy ceiling is unbounded or repeats a logical member.
    #[error("responsibility derivation `{derivation}` has an invalid policy ceiling: {source}")]
    InvalidPolicyCeiling {
        /// Invalid derivation.
        derivation: ResponsibilityDerivationId,
        /// Canonical policy-set failure.
        #[source]
        source: InvalidWorkspacePolicyCeilingV1,
    },
    /// A Genealogy link references a derivation outside the compiled set.
    #[error("genealogy link `{link}` references unknown responsibility `{derivation}`")]
    UnknownResponsibility {
        /// Invalid link.
        link: ResponsibilityGenealogyLinkId,
        /// Missing derivation.
        derivation: ResponsibilityDerivationId,
    },
    /// A Genealogy link references a projection outside the compiled set.
    #[error("genealogy link `{link}` references unknown projection `{projection}`")]
    UnknownProjection {
        /// Invalid link.
        link: ResponsibilityGenealogyLinkId,
        /// Missing projection.
        projection: ResponsibilityProjectionId,
    },
    /// Candidate-only vocabulary entered a released or required surface.
    #[error("candidate-only token `{token}` appears in released surface `{surface}`")]
    CandidateOnlySurface {
        /// Released schema, store, ABI, or selection identifier.
        surface: String,
        /// Forbidden candidate token.
        token: &'static str,
    },
}

/// Validate and canonically lower the manifest-selected responsibility derivations.
///
/// The fold is pure and deterministic. It sorts logical sets, resolves only
/// explicit local references, and introduces no organization, projection,
/// acceptance, authority, or native fact.
///
/// # Errors
///
/// Returns [`ResponsibilityDerivationError`] when an identity repeats, a
/// structural question is unanswered, a directional service omits its exact
/// contract selection, or a many-to-many reference does not resolve.
pub(crate) fn lower_responsibility_derivations(
    mut derivations: Vec<ResponsibilityDerivationV1>,
) -> Result<ResponsibilityDerivationIrV1, ResponsibilityDerivationError> {
    if derivations.len() > MAX_RESPONSIBILITY_DERIVATIONS {
        let derivation = derivations
            .first()
            .expect("an over-limit derivation set is non-empty")
            .derivation_id
            .clone();
        return Err(ResponsibilityDerivationError::Limit {
            derivation,
            kind: "derivation",
        });
    }
    for derivation in &derivations {
        reject_candidate_only_derivation_identifiers(derivation)?;
    }
    derivations.sort_by(|left, right| record_identity(left).cmp(record_identity(right)));
    if let Some(pair) = derivations
        .windows(2)
        .find(|pair| record_identity(&pair[0]) == record_identity(&pair[1]))
    {
        return Err(ResponsibilityDerivationError::Duplicate {
            derivation: pair[0].derivation_id.clone(),
            kind: "derivation identity",
        });
    }

    for derivation in &mut derivations {
        validate_and_canonicalize_derivation(derivation)?;
    }

    let derivation_ids = derivations
        .iter()
        .map(|item| record_identity(item).clone())
        .collect::<BTreeSet<_>>();
    let mut projection_ids = BTreeSet::new();
    let mut link_ids = BTreeSet::new();
    for derivation in &derivations {
        for projection in &derivation.native_projections {
            if !projection_ids.insert(projection.projection_id.clone()) {
                return Err(ResponsibilityDerivationError::Duplicate {
                    derivation: derivation.derivation_id.clone(),
                    kind: "projection identity across the derivation set",
                });
            }
        }
        for link in &derivation.genealogy_links {
            if !link_ids.insert(link.link_id.clone()) {
                return Err(ResponsibilityDerivationError::Duplicate {
                    derivation: derivation.derivation_id.clone(),
                    kind: "genealogy-link identity across the derivation set",
                });
            }
        }
    }
    for derivation in &derivations {
        for link in &derivation.genealogy_links {
            for referenced in &link.responsibilities {
                if !derivation_ids.contains(referenced) {
                    return Err(ResponsibilityDerivationError::UnknownResponsibility {
                        link: link.link_id.clone(),
                        derivation: referenced.clone(),
                    });
                }
            }
            for projection in &link.projections {
                if !projection_ids.contains(projection) {
                    return Err(ResponsibilityDerivationError::UnknownProjection {
                        link: link.link_id.clone(),
                        projection: projection.clone(),
                    });
                }
            }
        }
    }
    Ok(ResponsibilityDerivationIrV1 { derivations })
}

/// Reject candidate shared-semantics vocabulary from released or required surfaces.
///
/// Callers supply only schema IDs, required-store IDs, kernel ABI IDs, and
/// private-selection IDs. Ordinary prose and historical evidence are outside
/// this mechanical deny boundary.
///
/// # Errors
///
/// Returns [`ResponsibilityDerivationError::CandidateOnlySurface`] when a
/// tokenized identifier contains a candidate-only term.
pub(crate) fn reject_candidate_only_surfaces<'a>(
    surfaces: impl IntoIterator<Item = &'a str>,
) -> Result<(), ResponsibilityDerivationError> {
    for surface in surfaces {
        if let Some(token) = candidate_only_workspace_language_identifier_token_v1(surface) {
            return Err(ResponsibilityDerivationError::CandidateOnlySurface {
                surface: surface.to_string(),
                token,
            });
        }
    }
    Ok(())
}

/// Apply candidate-only denial to machine-facing responsibility identities.
///
/// The traversal covers the responsibility identity, structural selections,
/// module/export selections, directional service coordinate, direct path,
/// residences, and native projection coordinates. Attribution and historical
/// evidence remain preserved rather than reclassified.
///
/// # Errors
///
/// Returns [`ResponsibilityDerivationError::CandidateOnlySurface`] for the
/// first machine-facing identity containing a candidate-only token.
pub(crate) fn reject_candidate_only_derivation_identifiers(
    derivation: &ResponsibilityDerivationV1,
) -> Result<(), ResponsibilityDerivationError> {
    super::reject_candidate_only_record_identifiers(
        super::CandidateIdentityRecordV1::Responsibility(derivation),
    )
}

/// Validate and canonicalize one derivation before cross-record linking.
fn validate_and_canonicalize_derivation(
    derivation: &mut ResponsibilityDerivationV1,
) -> Result<(), ResponsibilityDerivationError> {
    if derivation.schema != RESPONSIBILITY_DERIVATION_V1_SCHEMA
        || derivation.schema_version != RESPONSIBILITY_DERIVATION_V1_SCHEMA_VERSION
    {
        return Err(ResponsibilityDerivationError::SchemaCoordinate {
            derivation: derivation.derivation_id.clone(),
        });
    }
    if derivation.structural_objects.len() > MAX_RESPONSIBILITY_STRUCTURAL_SELECTIONS {
        return Err(ResponsibilityDerivationError::Limit {
            derivation: derivation.derivation_id.clone(),
            kind: "structural selection",
        });
    }
    if derivation.native_projections.len() > MAX_RESPONSIBILITY_NATIVE_PROJECTIONS {
        return Err(ResponsibilityDerivationError::Limit {
            derivation: derivation.derivation_id.clone(),
            kind: "native projection",
        });
    }
    if derivation.genealogy_links.len() > MAX_RESPONSIBILITY_GENEALOGY_LINKS {
        return Err(ResponsibilityDerivationError::Limit {
            derivation: derivation.derivation_id.clone(),
            kind: "genealogy link",
        });
    }
    if derivation.residence.is_empty() || derivation.residence.len() > MAX_RESPONSIBILITY_RESIDENCES
    {
        return Err(ResponsibilityDerivationError::Limit {
            derivation: derivation.derivation_id.clone(),
            kind: "residence",
        });
    }

    if let AttributableAnswerV1::Supplied { value, .. } = &derivation.lawful_silence {
        enforce_nested_max(
            value.silent_surfaces.len(),
            MAX_RESPONSIBILITY_SILENCE_CLASSES,
            derivation,
            "lawful-silence surface",
        )?;
    }
    for projection in &derivation.native_projections {
        enforce_nested_max(
            projection.native_refs.len(),
            MAX_RESPONSIBILITY_NESTED_MEMBERS,
            derivation,
            "native reference",
        )?;
    }
    for link in &derivation.genealogy_links {
        if link.responsibilities.is_empty() || link.occurrences.is_empty() {
            return Err(ResponsibilityDerivationError::Limit {
                derivation: derivation.derivation_id.clone(),
                kind: "genealogy required member",
            });
        }
        for (kind, observed) in [
            ("genealogy responsibility", link.responsibilities.len()),
            ("genealogy projection", link.projections.len()),
            ("genealogy occurrence", link.occurrences.len()),
        ] {
            enforce_nested_max(
                observed,
                MAX_RESPONSIBILITY_NESTED_MEMBERS,
                derivation,
                kind,
            )?;
        }
    }

    derivation
        .structural_objects
        .sort_by_key(|selection| selection.kind);
    if derivation
        .structural_objects
        .windows(2)
        .any(|pair| pair[0].kind == pair[1].kind)
    {
        return Err(ResponsibilityDerivationError::Duplicate {
            derivation: derivation.derivation_id.clone(),
            kind: "structural-object question",
        });
    }
    if derivation
        .structural_objects
        .iter()
        .map(|selection| selection.kind)
        .ne(StructuralObjectKindV1::all().iter().copied())
    {
        return Err(ResponsibilityDerivationError::StructuralInventory {
            derivation: derivation.derivation_id.clone(),
        });
    }
    if let AttributableAnswerV1::Supplied {
        value: ResponsibilityPlacementV1::DirectionalService { service_contract },
        ..
    } = &derivation.placement
    {
        if service_contract.provider_scope == service_contract.consumer_scope {
            return Err(ResponsibilityDerivationError::ReflexiveDirectionalService {
                derivation: derivation.derivation_id.clone(),
                scope: service_contract.resource_id(),
            });
        }
        let selection = derivation
            .structural_objects
            .iter()
            .find(|selection| selection.kind == StructuralObjectKindV1::DirectionalServiceContract)
            .expect("the exhaustive structural inventory was validated above");
        match &selection.selection {
            AttributableAnswerV1::Omitted { .. } => {
                return Err(
                    ResponsibilityDerivationError::MissingDirectionalServiceContract {
                        derivation: derivation.derivation_id.clone(),
                    },
                );
            }
            AttributableAnswerV1::Supplied {
                value: selected, ..
            } if selected != &service_contract.resource_id() => {
                return Err(
                    ResponsibilityDerivationError::DirectionalServiceContractMismatch {
                        derivation: derivation.derivation_id.clone(),
                        expected: Box::new(service_contract.clone()),
                        selected: selected.clone(),
                    },
                );
            }
            AttributableAnswerV1::Supplied { .. } => {}
        }
    }

    if let AttributableAnswerV1::Supplied { value, .. } = &mut derivation.lawful_silence {
        canonical_set(
            &mut value.silent_surfaces,
            &derivation.derivation_id,
            "lawful-silence surface",
        )?;
    }
    if let AttributableAnswerV1::Supplied { value, .. } = &mut derivation.ceilings {
        value.policy.canonicalize_policy_sets().map_err(|source| {
            ResponsibilityDerivationError::InvalidPolicyCeiling {
                derivation: derivation.derivation_id.clone(),
                source,
            }
        })?;
        canonicalize_workspace_ceiling_set(
            &mut value.resources,
            WorkspaceCeilingSetAxisV1::Resource,
        )
        .map_err(
            |source| ResponsibilityDerivationError::InvalidPolicyCeiling {
                derivation: derivation.derivation_id.clone(),
                source,
            },
        )?;
    }
    canonical_by(
        &mut derivation.residence,
        &derivation.derivation_id,
        "residence",
        |item| (item.class, item.resource.clone(), item.owner.clone()),
    )?;
    derivation
        .native_projections
        .sort_by(|left, right| left.projection_id.cmp(&right.projection_id));
    if derivation
        .native_projections
        .windows(2)
        .any(|pair| pair[0].projection_id == pair[1].projection_id)
    {
        return Err(ResponsibilityDerivationError::Duplicate {
            derivation: derivation.derivation_id.clone(),
            kind: "native projection",
        });
    }
    for projection in &mut derivation.native_projections {
        canonical_set(
            &mut projection.native_refs,
            &derivation.derivation_id,
            "native reference",
        )?;
    }
    derivation
        .genealogy_links
        .sort_by(|left, right| left.link_id.cmp(&right.link_id));
    if derivation
        .genealogy_links
        .windows(2)
        .any(|pair| pair[0].link_id == pair[1].link_id)
    {
        return Err(ResponsibilityDerivationError::Duplicate {
            derivation: derivation.derivation_id.clone(),
            kind: "genealogy link",
        });
    }
    for link in &mut derivation.genealogy_links {
        canonical_set(
            &mut link.responsibilities,
            &derivation.derivation_id,
            "genealogy responsibility",
        )?;
        canonical_set(
            &mut link.projections,
            &derivation.derivation_id,
            "genealogy projection",
        )?;
        canonical_set(
            &mut link.occurrences,
            &derivation.derivation_id,
            "genealogy occurrence",
        )?;
    }
    Ok(())
}

/// Enforce one nested collection ceiling before canonicalization.
pub(super) fn enforce_nested_max(
    observed: usize,
    maximum: usize,
    derivation: &ResponsibilityDerivationV1,
    kind: &'static str,
) -> Result<(), ResponsibilityDerivationError> {
    if observed > maximum {
        Err(ResponsibilityDerivationError::Limit {
            derivation: derivation.derivation_id.clone(),
            kind,
        })
    } else {
        Ok(())
    }
}

/// Canonicalize one logical set and reject repeated members.
fn canonical_set<T: Ord>(
    values: &mut [T],
    derivation: &ResponsibilityDerivationId,
    kind: &'static str,
) -> Result<(), ResponsibilityDerivationError> {
    values.sort();
    if values.windows(2).any(|pair| pair[0] == pair[1]) {
        Err(ResponsibilityDerivationError::Duplicate {
            derivation: derivation.clone(),
            kind,
        })
    } else {
        Ok(())
    }
}

/// Canonicalize a logical set by a borrowed composite key.
fn canonical_by<T, K: Ord>(
    values: &mut [T],
    derivation: &ResponsibilityDerivationId,
    kind: &'static str,
    key: impl Fn(&T) -> K,
) -> Result<(), ResponsibilityDerivationError> {
    values.sort_by_key(&key);
    if values.windows(2).any(|pair| key(&pair[0]) == key(&pair[1])) {
        Err(ResponsibilityDerivationError::Duplicate {
            derivation: derivation.clone(),
            kind,
        })
    } else {
        Ok(())
    }
}
