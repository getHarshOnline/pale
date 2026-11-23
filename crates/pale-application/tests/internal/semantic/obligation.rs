//! Law tests for mechanical Package obligations and typed settlement.

use std::collections::BTreeSet;

use pale_core::records::{
    AdmittedGoverningOrderV1, ArtifactDigest, DegradedOutcomeIdV1, FailurePosture,
    ImmutableRecordReferenceV1, PackageIdV1, PackageResourceIdV1, ResolvedCapabilityFallbackV1,
    ResolvedCapabilityRequirementPolicyV1, ResolvedGoverningContributionV1, UnixTimeV1,
    WorkspaceResourceId,
};

use super::*;

fn resource(id: &str) -> PackageResourceRefV1 {
    PackageResourceRefV1 {
        package_id: PackageIdV1::parse("example.procedure").expect("valid Package id"),
        resource_id: PackageResourceIdV1::parse(id).expect("valid resource id"),
    }
}

fn exact_ref(id: &str) -> ImmutableRecordReferenceV1 {
    ImmutableRecordReferenceV1 {
        record_ref: pale_core::records::ResidentRecordRefV1::parse(id)
            .expect("valid resident record ref"),
        record_sha256: ArtifactDigest::hash_bytes(id.as_bytes()),
    }
}

fn workspace_id(id: &str) -> WorkspaceResourceId {
    WorkspaceResourceId::parse(id).expect("valid workspace resource id")
}

fn empty_governing_order() -> AdmittedGoverningOrderV1 {
    ResolvedGoverningContributionV1::compose([], &BTreeMap::new())
        .expect("empty governing algebra is valid")
}

fn operation(id: &str, mechanics: ResolvedOperationMechanics) -> super::super::ResolvedOperation {
    super::super::ResolvedOperation {
        record: super::super::ResolvedOperationRecord {
            id: resource(id),
            description: format!("{id} purpose"),
            roles: Vec::new(),
            outputs: Vec::new(),
        },
        body: String::new().into(),
        required_doctrine: String::new().into(),
        body_sources: Vec::new(),
        doctrine_ids: Vec::new(),
        mechanics,
    }
}

fn repository_plan(
    operations: Vec<super::super::ResolvedOperation>,
    recipes: Vec<super::super::ResolvedRecipe>,
) -> RepositorySemanticPlan {
    RepositorySemanticPlan {
        page_reading: BTreeMap::new(),
        governing_order: empty_governing_order(),
        operations,
        roles: Vec::new(),
        doctrine: Vec::new(),
        startup_policies: Vec::new(),
        run_modes: Vec::new(),
        mode_defaults: BTreeMap::new(),
        mode_default_sources: BTreeMap::new(),
        stacks: Vec::new(),
        policy_families: Vec::new(),
        recipes,
        repository_verifier_capabilities: Vec::new(),
    }
}

fn capability_requirement(recipe: PackageResourceRefV1) -> super::super::ResolvedCapability {
    super::super::ResolvedCapability {
        requirement_id: CapabilityRequirementIdV1::parse("source-proof")
            .expect("valid requirement id"),
        description: "establish source evidence".to_string(),
        capability_id: CapabilityIdV1::parse("source.read").expect("valid capability id"),
        recipe: Some(recipe),
        provider: ResolvedProviderSelection::Unbound(ProviderUnavailability::MissingBinding),
        effect_class: EffectClass::Read,
        policy: ResolvedCapabilityRequirementPolicyV1::RequiredDegrade {
            fallback: ResolvedCapabilityFallbackV1 {
                guidance: "continue with explicitly reduced evidence".to_string(),
                recipe: None,
            },
            degraded_outcome: DegradedOutcomeIdV1::parse("source-evidence-unavailable")
                .expect("valid degraded outcome"),
        },
        failure_posture: FailurePosture::FailClosed,
    }
}

fn recipe(id: &str) -> super::super::ResolvedRecipe {
    super::super::ResolvedRecipe {
        resource: resource(id),
        procedure: pale_core::records::ToolRecipe {
            description: "read source evidence".to_string(),
            capability: CapabilityIdV1::parse("source.read").expect("valid capability id"),
            tool_contract: ToolContractIdV1::parse("source-contract").expect("valid tool contract"),
            tool_operation: ToolOperationIdV1::parse("read").expect("valid tool operation"),
            effect_class: EffectClass::Read,
            input_mapping: BTreeMap::new(),
            evidence: ToolEvidenceContract {
                required_fields: vec![
                    NonRootJsonPointer::parse("/content_sha256").expect("valid JSON Pointer")
                ],
                proof_ceiling: ProofCeilingIdV1::parse("source-bytes-observed")
                    .expect("valid proof ceiling"),
                repository_verifier: true,
            },
        },
    }
}

#[test]
fn closure_is_pre_render_deterministic_and_provider_absence_is_local() {
    let recipe = recipe("source-recipe");
    let semantic_only = operation(
        "review",
        ResolvedOperationMechanics::SemanticOnly {
            selection: pale_core::records::OperationSelection::UserOnly {
                aliases: Vec::new(),
            },
        },
    );
    let unbound = operation("publish", ResolvedOperationMechanics::Unbound);
    let capability_bearing = operation(
        "inspect-source",
        ResolvedOperationMechanics::CapabilityBearing {
            selection: pale_core::records::OperationSelection::Internal,
            capabilities: vec![capability_requirement(recipe.resource.clone())],
        },
    );
    let plan = repository_plan(
        vec![semantic_only, unbound, capability_bearing],
        vec![recipe],
    );

    let closure = compile_package_obligations(&[plan]).expect("obligations close");

    assert_eq!(closure.repositories.len(), 1);
    assert_eq!(closure.repositories[0].operations.len(), 3);
    assert!(matches!(
        closure.repositories[0]
            .operations
            .iter()
            .find(|obligation| obligation.operation == resource("publish"))
            .expect("unbound operation remains in the closure")
            .state,
        OperationObligationState::Unbound
    ));
    let OperationObligationState::CapabilityBearing { capabilities, .. } = &closure.repositories[0]
        .operations
        .iter()
        .find(|obligation| obligation.operation == resource("inspect-source"))
        .expect("capability-bearing operation remains in the closure")
        .state
    else {
        panic!("capability-bearing operation keeps its exact state")
    };
    let [obligation] = capabilities.as_slice() else {
        panic!("operation should carry its one capability obligation")
    };
    assert_eq!(obligation.subject.operation, resource("inspect-source"));
    assert!(matches!(
        obligation.disposition,
        CapabilityObligationDisposition::Degraded {
            reason: ProviderUnavailability::MissingBinding,
            ..
        }
    ));
    assert!(matches!(
        obligation.procedure,
        ProviderProcedureObligation::Recipe {
            repository_verification: RepositoryVerificationDisposition::Eligible,
            ..
        }
    ));
}

#[test]
fn closure_rejects_a_recipe_contract_mismatch() {
    let mut recipe = recipe("source-recipe");
    recipe.procedure.effect_class = EffectClass::Write;
    let operation = operation(
        "inspect-source",
        ResolvedOperationMechanics::CapabilityBearing {
            selection: pale_core::records::OperationSelection::Internal,
            capabilities: vec![capability_requirement(recipe.resource.clone())],
        },
    );

    assert!(matches!(
        compile_package_obligations(&[repository_plan(vec![operation], vec![recipe])]),
        Err(MechanicalObligationError::RecipeMismatch { .. })
    ));
}

fn current_evidence() -> EvidenceCoordinates {
    let stable = exact_ref("evidence/stability");
    let subject = workspace_id("scope.subject");
    EvidenceCoordinates {
        repeatability: EvidenceCoordinate(EvidenceCoordinateState::Current(
            RepeatabilityCoordinate {
                mechanism: exact_ref("evidence/mechanism"),
                version: exact_ref("evidence/version"),
                runtime: exact_ref("evidence/runtime"),
                method: RepeatabilityMethod::Deterministic {
                    repetitions: 2,
                    stability: stable,
                },
            },
        )),
        independence: EvidenceCoordinate(EvidenceCoordinateState::Current(
            IndependenceCoordinate {
                candidate: exact_ref("evidence/candidate"),
                evaluator: exact_ref("evidence/evaluator"),
                relation: IndependenceRelation::Independent {
                    verification: exact_ref("evidence/independence"),
                },
            },
        )),
        grounding: EvidenceCoordinate(EvidenceCoordinateState::Current(GroundingCoordinate {
            frontiers: BTreeSet::from([exact_ref("evidence/frontier")]),
            assertions: vec![GroundingAssertion {
                evidence: exact_ref("evidence/native-observation"),
                kind: GroundingKind::Observed,
            }],
            contradictions: BTreeSet::new(),
            limitations: BTreeSet::new(),
        })),
        coverage: EvidenceCoordinate(EvidenceCoordinateState::Current(CoverageCoordinate {
            intended: BTreeSet::from([subject.clone()]),
            observed: BTreeSet::from([subject]),
            gaps: BTreeSet::new(),
            negative_cases: BTreeSet::new(),
            effects: BTreeSet::from([EffectClass::Read]),
        })),
    }
}

fn freshness() -> FreshnessCoordinate {
    FreshnessCoordinate {
        provider_index: FreshnessState::Current(exact_ref("freshness/provider-index")),
        compiled_generation: FreshnessState::Current(exact_ref("freshness/generation")),
        context_projection_dependencies: FreshnessState::Current(exact_ref(
            "freshness/context-dependencies",
        )),
        native_owner_revision: FreshnessState::Current(exact_ref("freshness/native-owner")),
    }
}

fn settlement(target: SettlementTarget) -> EvidenceSettlementInput {
    let native_verification = match &target {
        SettlementTarget::NativeEffect { owner, .. } => NativeVerification::Pending {
            owner: owner.clone(),
        },
        SettlementTarget::ProviderDelivery
        | SettlementTarget::EvidenceConclusion
        | SettlementTarget::OwnerAcceptance { .. }
        | SettlementTarget::QualitativeFrontier { .. } => NativeVerification::NotApplicable,
    };
    let mut evidence = current_evidence();
    if let SettlementTarget::NativeEffect { effect, .. } = &target {
        let EvidenceCoordinateState::Current(coverage) = &mut evidence.coverage.0 else {
            unreachable!("fixture coverage is current")
        };
        coverage.effects.insert(*effect);
    }
    EvidenceSettlementInput {
        obligation: ObligationSubject {
            operation: resource("inspect-source"),
            requirement: CapabilityRequirementIdV1::parse("source-proof")
                .expect("valid requirement id"),
        },
        target,
        generation: pale_core::records::CompiledGenerationDigest::hash_canonical(&"generation")
            .expect("canonical generation digest"),
        provider_evidence: ProviderEvidenceCoordinate::Bound {
            provider: workspace_id("provider.source"),
            evidence: exact_ref("evidence/provider-delivery"),
        },
        evidence: evidence.admit().expect("valid evidence coordinates"),
        freshness: freshness(),
        contradictions: BTreeSet::new(),
        native_verification,
        owner_policy: OwnerPolicyRequirement::NotRequired,
        expires_at: None,
        invalidation: None,
    }
}

#[test]
fn provider_success_cannot_settle_a_native_effect() {
    let input = settlement(SettlementTarget::NativeEffect {
        effect: EffectClass::Write,
        owner: workspace_id("owner.repository"),
    });

    assert_eq!(
        settle_obligation(&input, UnixTimeV1::new(10).expect("valid time")),
        SettlementDecision {
            state: EvidenceSettlementState::EffectUnknown,
            residual_action: ResidualAction::ReconcileNativeEffect,
        }
    );
}

#[test]
fn absent_required_qualitative_judgment_is_pending_owner() {
    let owner = workspace_id("owner.publisher");
    let mut input = settlement(SettlementTarget::QualitativeFrontier {
        owner: owner.clone(),
        frontier: QualitativeFrontier::Publication,
    });
    input.owner_policy = OwnerPolicyRequirement::Required {
        owner,
        evidence: EvidenceCoordinateState::Unknown,
    };

    assert_eq!(
        settle_obligation(&input, UnixTimeV1::new(10).expect("valid time")).state,
        EvidenceSettlementState::PendingOwner
    );
}

#[test]
fn exact_native_verification_settles_without_changing_evidence_dimensions() {
    let mut input = settlement(SettlementTarget::NativeEffect {
        effect: EffectClass::Write,
        owner: workspace_id("owner.repository"),
    });
    input.native_verification = NativeVerification::Verified {
        owner: workspace_id("owner.repository"),
        evidence: exact_ref("evidence/read-back"),
    };

    assert_eq!(
        settle_obligation(&input, UnixTimeV1::new(10).expect("valid time")).state,
        EvidenceSettlementState::Satisfied
    );
}

#[test]
fn native_verification_cannot_settle_an_effect_outside_evidence_coverage() {
    let mut input = settlement(SettlementTarget::NativeEffect {
        effect: EffectClass::Write,
        owner: workspace_id("owner.repository"),
    });
    input.evidence = current_evidence()
        .admit()
        .expect("read-only coverage remains structurally valid");
    input.native_verification = NativeVerification::Verified {
        owner: workspace_id("owner.repository"),
        evidence: exact_ref("evidence/read-back"),
    };

    assert_eq!(
        settle_obligation(&input, UnixTimeV1::new(10).expect("valid time")),
        SettlementDecision {
            state: EvidenceSettlementState::Unknown,
            residual_action: ResidualAction::SupplyEvidenceCarrier,
        }
    );
}

#[test]
fn contradiction_and_staleness_have_causal_precedence() {
    let mut input = settlement(SettlementTarget::EvidenceConclusion);
    input.freshness.compiled_generation = FreshnessState::Stale {
        frontier: exact_ref("evidence/old-frontier"),
        invalidation: exact_ref("evidence/new-frontier"),
    };
    assert_eq!(
        settle_obligation(&input, UnixTimeV1::new(10).expect("valid time")).state,
        EvidenceSettlementState::Stale
    );

    input
        .contradictions
        .insert(exact_ref("evidence/contradiction"));
    assert_eq!(
        settle_obligation(&input, UnixTimeV1::new(10).expect("valid time")).state,
        EvidenceSettlementState::Conflicted
    );
}

#[test]
fn coverage_cannot_observe_outside_its_declared_scope() {
    let mut coordinates = current_evidence();
    let EvidenceCoordinateState::Current(coverage) = &mut coordinates.coverage.0 else {
        unreachable!("fixture is current")
    };
    coverage
        .observed
        .insert(workspace_id("scope.outside-intent"));

    assert_eq!(
        validate_evidence_coordinates(&coordinates),
        Err(EvidenceCoordinateError::ObservedOutsideIntendedScope)
    );
}

#[test]
fn malformed_coordinates_cannot_enter_settlement() {
    let mut coordinates = current_evidence();
    let EvidenceCoordinateState::Current(repeatability) = &mut coordinates.repeatability.0 else {
        unreachable!("fixture is current")
    };
    let RepeatabilityMethod::Deterministic { repetitions, .. } = &mut repeatability.method else {
        unreachable!("fixture is deterministic")
    };
    *repetitions = 0;

    assert_eq!(
        coordinates.admit(),
        Err(EvidenceCoordinateError::ZeroRepetitions)
    );
}

#[test]
fn stale_coordinates_retain_the_same_structural_laws() {
    let mut coordinates = current_evidence();
    let EvidenceCoordinateState::Current(mut repeatability) = coordinates.repeatability.0 else {
        unreachable!("fixture is current")
    };
    match &mut repeatability.method {
        RepeatabilityMethod::Deterministic { repetitions, .. }
        | RepeatabilityMethod::Stochastic { repetitions, .. } => *repetitions = 0,
    }
    coordinates.repeatability = EvidenceCoordinate(EvidenceCoordinateState::Stale {
        coordinate: repeatability,
        invalidation: exact_ref("evidence/repeatability-invalidation"),
    });

    assert_eq!(
        coordinates.admit(),
        Err(EvidenceCoordinateError::ZeroRepetitions)
    );

    let mut coordinates = current_evidence();
    let EvidenceCoordinateState::Current(mut independence) = coordinates.independence.0 else {
        unreachable!("fixture is current")
    };
    independence.evaluator = independence.candidate.clone();
    coordinates.independence = EvidenceCoordinate(EvidenceCoordinateState::Stale {
        coordinate: independence,
        invalidation: exact_ref("evidence/independence-invalidation"),
    });
    assert_eq!(
        coordinates.admit(),
        Err(EvidenceCoordinateError::SelfEvaluatedIndependence)
    );

    let mut coordinates = current_evidence();
    let EvidenceCoordinateState::Current(mut grounding) = coordinates.grounding.0 else {
        unreachable!("fixture is current")
    };
    grounding.frontiers.clear();
    coordinates.grounding = EvidenceCoordinate(EvidenceCoordinateState::Stale {
        coordinate: grounding,
        invalidation: exact_ref("evidence/grounding-invalidation"),
    });
    assert_eq!(
        coordinates.admit(),
        Err(EvidenceCoordinateError::EmptyGroundingEvidence)
    );

    let mut coordinates = current_evidence();
    let EvidenceCoordinateState::Current(mut coverage) = coordinates.coverage.0 else {
        unreachable!("fixture is current")
    };
    coverage.intended.insert(workspace_id("scope.unclassified"));
    coordinates.coverage = EvidenceCoordinate(EvidenceCoordinateState::Stale {
        coordinate: coverage,
        invalidation: exact_ref("evidence/coverage-invalidation"),
    });
    assert_eq!(
        coordinates.admit(),
        Err(EvidenceCoordinateError::IncompleteCoveragePartition)
    );
}

#[test]
fn current_evidence_requires_grounding_and_coverage_denominators() {
    let mut coordinates = current_evidence();
    let EvidenceCoordinateState::Current(grounding) = &mut coordinates.grounding.0 else {
        unreachable!("fixture is current")
    };
    grounding.frontiers.clear();
    assert_eq!(
        coordinates.clone().admit(),
        Err(EvidenceCoordinateError::EmptyGroundingEvidence)
    );

    let mut coordinates = current_evidence();
    let EvidenceCoordinateState::Current(coverage) = &mut coordinates.coverage.0 else {
        unreachable!("fixture is current")
    };
    coverage.intended.clear();
    coverage.observed.clear();
    assert_eq!(
        coordinates.admit(),
        Err(EvidenceCoordinateError::EmptyCoverageIntent)
    );

    let mut coordinates = current_evidence();
    let EvidenceCoordinateState::Current(grounding) = &mut coordinates.grounding.0 else {
        unreachable!("fixture is current")
    };
    grounding.assertions.clear();
    assert_eq!(
        coordinates.admit(),
        Err(EvidenceCoordinateError::EmptyGroundingEvidence)
    );
}

#[test]
fn independence_and_coverage_claim_only_what_their_evidence_observed() {
    let mut coordinates = current_evidence();
    let EvidenceCoordinateState::Current(independence) = &mut coordinates.independence.0 else {
        unreachable!("fixture is current")
    };
    independence.evaluator = independence.candidate.clone();
    assert_eq!(
        coordinates.admit(),
        Err(EvidenceCoordinateError::SelfEvaluatedIndependence)
    );

    let mut coordinates = current_evidence();
    let EvidenceCoordinateState::Current(independence) = &mut coordinates.independence.0 else {
        unreachable!("fixture is current")
    };
    independence.relation = IndependenceRelation::Coupled {
        couplings: BTreeSet::new(),
        assessment: exact_ref("evidence/empty-coupling-assessment"),
    };
    assert_eq!(
        coordinates.admit(),
        Err(EvidenceCoordinateError::EmptyCouplingSet)
    );

    let mut coordinates = current_evidence();
    let EvidenceCoordinateState::Current(coverage) = &mut coordinates.coverage.0 else {
        unreachable!("fixture is current")
    };
    let observed = coverage
        .observed
        .iter()
        .next()
        .expect("fixture has observed scope")
        .clone();
    coverage.observed.clear();
    coverage.negative_cases.insert(observed);
    assert_eq!(
        coordinates.admit(),
        Err(EvidenceCoordinateError::NegativeCaseUnobserved)
    );
}

#[test]
fn coverage_is_an_exact_observed_or_gap_partition_with_an_effect_boundary() {
    let mut coordinates = current_evidence();
    let EvidenceCoordinateState::Current(coverage) = &mut coordinates.coverage.0 else {
        unreachable!("fixture is current")
    };
    coverage.intended.insert(workspace_id("scope.unclassified"));
    assert_eq!(
        coordinates.clone().admit(),
        Err(EvidenceCoordinateError::IncompleteCoveragePartition)
    );

    let EvidenceCoordinateState::Current(coverage) = &mut coordinates.coverage.0 else {
        unreachable!("fixture remains current")
    };
    coverage.effects.clear();
    assert_eq!(
        coordinates.admit(),
        Err(EvidenceCoordinateError::EmptyCoverageEffects)
    );
}

#[test]
fn evidence_records_cannot_double_classify_one_grounding_or_coverage_subject() {
    let mut coordinates = current_evidence();
    let EvidenceCoordinateState::Current(grounding) = &mut coordinates.grounding.0 else {
        unreachable!("fixture is current")
    };
    let evidence = grounding.assertions[0].evidence.clone();
    grounding.assertions.push(GroundingAssertion {
        evidence: evidence.clone(),
        kind: GroundingKind::Inferred {
            basis: exact_ref("evidence/inference-basis"),
        },
    });
    grounding.assertions.sort();
    assert_eq!(
        coordinates.admit(),
        Err(EvidenceCoordinateError::DuplicateGroundingEvidence)
    );

    let mut coordinates = current_evidence();
    let EvidenceCoordinateState::Current(grounding) = &mut coordinates.grounding.0 else {
        unreachable!("fixture is current")
    };
    grounding
        .contradictions
        .insert(grounding.assertions[0].evidence.clone());
    assert_eq!(
        coordinates.admit(),
        Err(EvidenceCoordinateError::DuplicateGroundingEvidence)
    );

    let mut coordinates = current_evidence();
    let EvidenceCoordinateState::Current(coverage) = &mut coordinates.coverage.0 else {
        unreachable!("fixture is current")
    };
    let subject = coverage
        .observed
        .iter()
        .next()
        .expect("fixture has observed scope")
        .clone();
    coverage.observed.clear();
    coverage.gaps.extend([
        CoverageGap::Omitted(subject.clone()),
        CoverageGap::BlindSpot(subject),
    ]);
    assert_eq!(
        coordinates.admit(),
        Err(EvidenceCoordinateError::DuplicateCoverageGap)
    );

    let mut coordinates = current_evidence();
    let EvidenceCoordinateState::Current(coverage) = &mut coordinates.coverage.0 else {
        unreachable!("fixture is current")
    };
    let observed = coverage
        .observed
        .iter()
        .next()
        .expect("fixture has observed scope")
        .clone();
    coverage.gaps.insert(CoverageGap::Omitted(observed));
    assert_eq!(
        coordinates.admit(),
        Err(EvidenceCoordinateError::ObservedGapOverlap)
    );
}

#[test]
fn honest_conflict_and_staleness_are_admitted_without_becoming_satisfied() {
    let mut contradictions_only = current_evidence();
    let EvidenceCoordinateState::Current(grounding) = &mut contradictions_only.grounding.0 else {
        unreachable!("fixture is current")
    };
    grounding.assertions.clear();
    grounding
        .contradictions
        .insert(exact_ref("evidence/contradiction-only"));
    let mut input = settlement(SettlementTarget::EvidenceConclusion);
    input.evidence = contradictions_only
        .admit()
        .expect("contradiction-only grounding is structurally honest");
    assert_eq!(
        settle_obligation(&input, UnixTimeV1::new(10).expect("valid time")).state,
        EvidenceSettlementState::Conflicted
    );

    let mut stale = current_evidence();
    let EvidenceCoordinateState::Current(repeatability) = stale.repeatability.0 else {
        unreachable!("fixture is current")
    };
    stale.repeatability = EvidenceCoordinate(EvidenceCoordinateState::Stale {
        coordinate: repeatability,
        invalidation: exact_ref("evidence/stale-but-structural"),
    });
    input.evidence = stale
        .admit()
        .expect("well-formed stale evidence remains admissible");
    assert_eq!(
        settle_obligation(&input, UnixTimeV1::new(10).expect("valid time")).state,
        EvidenceSettlementState::Stale
    );

    let mut coupled = current_evidence();
    let EvidenceCoordinateState::Current(independence) = &mut coupled.independence.0 else {
        unreachable!("fixture is current")
    };
    independence.evaluator = independence.candidate.clone();
    independence.relation = IndependenceRelation::Coupled {
        couplings: BTreeSet::from([IndependenceCoupling::Runtime]),
        assessment: exact_ref("evidence/same-execution-coupling"),
    };
    assert!(coupled.admit().is_ok());
}

#[test]
fn provider_delivery_requires_its_own_evidence_coordinate() {
    let mut input = settlement(SettlementTarget::ProviderDelivery);
    input.provider_evidence = ProviderEvidenceCoordinate::Missing;
    input.native_verification = NativeVerification::NotApplicable;

    assert_eq!(
        settle_obligation(&input, UnixTimeV1::new(10).expect("valid time")).state,
        EvidenceSettlementState::Unknown
    );
}

#[test]
fn owner_evidence_cannot_settle_another_owners_frontier() {
    let mut input = settlement(SettlementTarget::OwnerAcceptance {
        owner: workspace_id("owner.publisher"),
    });
    input.owner_policy = OwnerPolicyRequirement::Required {
        owner: workspace_id("owner.reviewer"),
        evidence: EvidenceCoordinateState::Current(exact_ref("evidence/owner-decision")),
    };
    input.native_verification = NativeVerification::NotApplicable;

    assert_eq!(
        settle_obligation(&input, UnixTimeV1::new(10).expect("valid time")).state,
        EvidenceSettlementState::Conflicted
    );
}

#[test]
fn required_freshness_layer_cannot_be_silently_omitted() {
    let mut input = settlement(SettlementTarget::NativeEffect {
        effect: EffectClass::Write,
        owner: workspace_id("owner.repository"),
    });
    input.freshness.native_owner_revision = FreshnessState::NotApplicable;
    input.native_verification = NativeVerification::Verified {
        owner: workspace_id("owner.repository"),
        evidence: exact_ref("evidence/read-back"),
    };

    assert_eq!(
        settle_obligation(&input, UnixTimeV1::new(10).expect("valid time")).state,
        EvidenceSettlementState::Unknown
    );
}

#[test]
fn freshness_outside_the_target_layer_set_cannot_block_settlement() {
    let mut input = settlement(SettlementTarget::EvidenceConclusion);
    input.freshness.provider_index = FreshnessState::Stale {
        frontier: exact_ref("evidence/irrelevant-provider-frontier"),
        invalidation: exact_ref("evidence/irrelevant-provider-invalidation"),
    };
    input.freshness.native_owner_revision = FreshnessState::Unknown;

    assert_eq!(
        settle_obligation(&input, UnixTimeV1::new(10).expect("valid time")).state,
        EvidenceSettlementState::Satisfied
    );
}

#[test]
fn native_verification_cannot_cross_owner_identity() {
    let mut input = settlement(SettlementTarget::NativeEffect {
        effect: EffectClass::Write,
        owner: workspace_id("owner.repository"),
    });
    input.native_verification = NativeVerification::Verified {
        owner: workspace_id("owner.another-repository"),
        evidence: exact_ref("evidence/read-back"),
    };

    assert_eq!(
        settle_obligation(&input, UnixTimeV1::new(10).expect("valid time")).state,
        EvidenceSettlementState::Unknown
    );
}

#[test]
fn explicit_coupling_and_coverage_gaps_do_not_satisfy() {
    let mut coupled = current_evidence();
    let EvidenceCoordinateState::Current(independence) = &mut coupled.independence.0 else {
        unreachable!("fixture is current")
    };
    independence.relation = IndependenceRelation::Coupled {
        couplings: BTreeSet::from([IndependenceCoupling::Model]),
        assessment: exact_ref("evidence/coupling"),
    };
    let mut input = settlement(SettlementTarget::EvidenceConclusion);
    input.evidence = coupled.admit().expect("coupling is explicitly represented");
    input.native_verification = NativeVerification::NotApplicable;
    assert_eq!(
        settle_obligation(&input, UnixTimeV1::new(10).expect("valid time")).state,
        EvidenceSettlementState::Conflicted
    );

    let mut gapped = current_evidence();
    let EvidenceCoordinateState::Current(coverage) = &mut gapped.coverage.0 else {
        unreachable!("fixture is current")
    };
    let subject = coverage
        .intended
        .iter()
        .next()
        .expect("fixture has intended scope")
        .clone();
    coverage.observed.remove(&subject);
    coverage.gaps.insert(CoverageGap::Omitted(subject));
    input.evidence = gapped.admit().expect("gap is within intended scope");
    assert_eq!(
        settle_obligation(&input, UnixTimeV1::new(10).expect("valid time")).state,
        EvidenceSettlementState::Omitted
    );
}
