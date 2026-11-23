//! Law tests for occurrence narrowing, actual use, invalidation, and Pack references.

use std::collections::{BTreeMap, BTreeSet};

use pale_core::records::{
    ActivationFenceDigest, ArtifactDigest, CapabilityIdV1, CapabilityRequirementIdV1,
    CompiledGenerationDigest, EffectClass, ImmutableRecordReferenceV1, NormativePackAttestorV1,
    NormativePackDigest, NormativePackIdV1, NormativePackReleaseV1, NormativePackVersionV1,
    PackageDigest, PackageIdV1, PackageReleaseV1, PackageResourceIdV1, PackageResourceRefV1,
    PackageVersionV1, ResidentRecordRefV1, UnixTimeV1, WorkspaceResourceId,
};

use super::*;
use crate::semantic::{
    settle_obligation, EvidenceCoordinate, EvidenceCoordinateState, EvidenceCoordinates,
    EvidenceSettlementInput, FreshnessCoordinate, FreshnessState, NativeVerification,
    ObligationSubject, OwnerPolicyRequirement, ProviderEvidenceCoordinate, SettlementTarget,
};

fn exact_ref(id: &str) -> ImmutableRecordReferenceV1 {
    ImmutableRecordReferenceV1 {
        record_ref: ResidentRecordRefV1::parse(id).expect("valid record ref"),
        record_sha256: ArtifactDigest::hash_bytes(id.as_bytes()),
    }
}

fn workspace_id(id: &str) -> WorkspaceResourceId {
    WorkspaceResourceId::parse(id).expect("valid workspace resource id")
}

fn operation(id: &str) -> PackageResourceRefV1 {
    PackageResourceRefV1 {
        package_id: PackageIdV1::parse("example.procedure").expect("valid Package id"),
        resource_id: PackageResourceIdV1::parse(id).expect("valid resource id"),
    }
}

fn release(version: &str) -> PackageReleaseV1 {
    PackageReleaseV1 {
        id: PackageIdV1::parse("example.procedure").expect("valid Package id"),
        version: PackageVersionV1::parse(version).expect("valid Package version"),
        content_digest: PackageDigest::hash_bytes(version.as_bytes()),
    }
}

fn pack_release(version: &str) -> NormativePackReleaseV1 {
    NormativePackReleaseV1 {
        id: NormativePackIdV1::parse("example.standard").expect("valid Pack id"),
        attestor: NormativePackAttestorV1::parse("example.attestor").expect("valid attestor id"),
        version: NormativePackVersionV1::parse(version).expect("valid Pack version"),
        content_digest: NormativePackDigest::hash_bytes(version.as_bytes()),
    }
}

fn generation(id: &str) -> CompiledGenerationDigest {
    CompiledGenerationDigest::hash_canonical(&id).expect("canonical generation digest")
}

fn pending_evidence() -> crate::semantic::AdmittedEvidenceCoordinates {
    EvidenceCoordinates {
        repeatability: EvidenceCoordinate(EvidenceCoordinateState::Unknown),
        independence: EvidenceCoordinate(EvidenceCoordinateState::Unknown),
        grounding: EvidenceCoordinate(EvidenceCoordinateState::Unknown),
        coverage: EvidenceCoordinate(EvidenceCoordinateState::Unknown),
    }
    .admit()
    .expect("explicit unknown evidence coordinates are structurally valid")
}

fn current_freshness(label: &str) -> FreshnessCoordinate {
    FreshnessCoordinate {
        provider_index: FreshnessState::Current(exact_ref(&format!("{label}/provider"))),
        compiled_generation: FreshnessState::Current(exact_ref(&format!("{label}/generation"))),
        context_projection_dependencies: FreshnessState::Current(exact_ref(&format!(
            "{label}/context"
        ))),
        native_owner_revision: FreshnessState::Current(exact_ref(&format!("{label}/native"))),
    }
}

fn ceilings(operations: &[&str], effects: &[EffectClass], budget: u64) -> OccurrenceCeilings {
    OccurrenceCeilings {
        operations: operations.iter().map(|id| operation(id)).collect(),
        preconditions: BTreeSet::from([workspace_id("precondition.source-current")]),
        capabilities: BTreeSet::from([
            CapabilityIdV1::parse("source.read").expect("valid capability id")
        ]),
        effects: effects.iter().copied().collect(),
        evidence: BTreeSet::from([workspace_id("evidence.source")]),
        rights: BTreeSet::from([workspace_id("right.inspect")]),
        authority: BTreeSet::from([workspace_id("authority.inspect")]),
        disclosure: BTreeSet::from([workspace_id("disclosure.private")]),
        model_use: BTreeSet::from([workspace_id("model-use.inference")]),
        propagation: BTreeSet::from([workspace_id("propagation.child")]),
        budgets: BTreeMap::from([(workspace_id("budget.source-bytes"), budget)]),
        fallbacks: BTreeSet::from([operation("fallback-source")]),
    }
}

fn context(label: &str) -> PackageOccurrenceContext {
    PackageOccurrenceContext {
        principal: exact_ref(&format!("{label}/principal")),
        controller: exact_ref(&format!("{label}/controller")),
        organization: None,
        service_contract: None,
        relationship: None,
        resource_scope: exact_ref(&format!("{label}/scope")),
        responsibility_role: exact_ref(&format!("{label}/role")),
        participant: exact_ref(&format!("{label}/participant")),
        origin_interface: exact_ref(&format!("{label}/origin")),
        execution_path: exact_ref(&format!("{label}/path")),
        provider_session: None,
        purpose: workspace_id("purpose.review"),
        authority: exact_ref(&format!("{label}/authority")),
        effect: EffectClass::Read,
        evidence_frontier: exact_ref(&format!("{label}/frontier")),
        disclosure: exact_ref(&format!("{label}/disclosure")),
        model_use: exact_ref(&format!("{label}/model-use")),
        propagation: exact_ref(&format!("{label}/propagation")),
        budget: exact_ref(&format!("{label}/budget")),
        fallback: exact_ref(&format!("{label}/fallback")),
        settlement: exact_ref(&format!("{label}/settlement")),
    }
}

fn package_occurrence(
    id: &str,
    release: PackageReleaseV1,
    ceilings: OccurrenceCeilings,
    parent: Option<ImmutableRecordReferenceV1>,
) -> PackageOccurrence {
    PackageOccurrence {
        occurrence: exact_ref(id),
        release,
        generation: generation("shared-generation"),
        context: context(id),
        context_derivation: if parent.is_some() {
            OccurrenceContextDerivation::Delegated {
                evidence: exact_ref(&format!("{id}/delegation")),
            }
        } else {
            OccurrenceContextDerivation::Root
        },
        normative_packs: Vec::new(),
        ceilings,
        parent,
    }
}

fn preserve_parent_context(parent: &AdmittedPackageOccurrence, child: &mut PackageOccurrence) {
    child.context = parent.record().context.clone();
    child.context_derivation = OccurrenceContextDerivation::Preserved;
}

#[test]
fn child_may_narrow_but_cannot_widen_or_replace_release() {
    let parent = admit_root_occurrence(package_occurrence(
        "occurrence/parent",
        release("0.0.1"),
        ceilings(
            &["review", "publish"],
            &[EffectClass::Read, EffectClass::Publish],
            100,
        ),
        None,
    ))
    .expect("root occurrence is valid");
    let mut child = package_occurrence(
        "occurrence/child",
        release("0.0.1"),
        ceilings(&["review"], &[EffectClass::Read], 40),
        Some(parent.record().occurrence.clone()),
    );
    preserve_parent_context(&parent, &mut child);
    assert!(derive_child_occurrence(&parent, child).is_ok());

    let mut widened = package_occurrence(
        "occurrence/widened",
        release("0.0.1"),
        ceilings(&["review"], &[EffectClass::Read, EffectClass::Write], 40),
        Some(parent.record().occurrence.clone()),
    );
    preserve_parent_context(&parent, &mut widened);
    assert_eq!(
        derive_child_occurrence(&parent, widened),
        Err(ChildOccurrenceError::CeilingWidening)
    );

    let mut substituted = package_occurrence(
        "occurrence/substituted",
        release("0.0.2"),
        ceilings(&["review"], &[EffectClass::Read], 40),
        Some(parent.record().occurrence.clone()),
    );
    preserve_parent_context(&parent, &mut substituted);
    assert_eq!(
        derive_child_occurrence(&parent, substituted),
        Err(ChildOccurrenceError::ReleaseSubstitution)
    );
}

#[test]
fn child_cannot_cross_generation_or_claim_preserved_changed_context() {
    let parent = admit_root_occurrence(package_occurrence(
        "occurrence/parent-context",
        release("0.0.1"),
        ceilings(&["review"], &[EffectClass::Read], 100),
        None,
    ))
    .expect("root occurrence is valid");
    let mut child = package_occurrence(
        "occurrence/child-context",
        release("0.0.1"),
        ceilings(&["review"], &[EffectClass::Read], 100),
        Some(parent.record().occurrence.clone()),
    );
    child.generation = generation("another-generation");
    assert_eq!(
        derive_child_occurrence(&parent, child.clone()),
        Err(ChildOccurrenceError::GenerationSubstitution)
    );

    child.generation = parent.record().generation.clone();
    child.context_derivation = OccurrenceContextDerivation::Preserved;
    assert_eq!(
        derive_child_occurrence(&parent, child.clone()),
        Err(ChildOccurrenceError::PreservedContextMismatch)
    );

    child.context_derivation = OccurrenceContextDerivation::Delegated {
        evidence: exact_ref("delegation/unverified"),
    };
    assert_eq!(
        derive_child_occurrence(&parent, child),
        Err(ChildOccurrenceError::UnverifiedDelegation)
    );
}

#[test]
fn actual_use_is_bound_to_one_occurrence_generation_and_operation() {
    let occurrence = admit_root_occurrence(package_occurrence(
        "occurrence/reviewer",
        release("0.0.1"),
        ceilings(&["review"], &[EffectClass::Read], 100),
        None,
    ))
    .expect("root occurrence is valid");
    let valid = PackageUse {
        use_record: exact_ref("use/reviewer"),
        occurrence: occurrence.record().occurrence.clone(),
        operation: operation("review"),
        invocation: exact_ref("invocation/reviewer"),
        activation: None,
        provider: None,
        generation: occurrence.record().generation.clone(),
        evidence_at_use: vec![exact_ref("evidence/use-frontier")],
        disposition: PackageUseDisposition::Completed,
    };
    assert!(admit_package_use(&occurrence, None, valid.clone()).is_ok());

    let another_occurrence = admit_root_occurrence(package_occurrence(
        "occurrence/publisher",
        release("0.0.1"),
        ceilings(&["publish"], &[EffectClass::Read], 100),
        None,
    ))
    .expect("root occurrence is valid");
    assert_eq!(
        admit_package_use(&another_occurrence, None, valid),
        Err(PackageUseError::OccurrenceMismatch)
    );
}

#[test]
fn actual_use_freezes_canonical_evidence_at_invocation() {
    let occurrence = admit_root_occurrence(package_occurrence(
        "occurrence/evidence",
        release("0.0.1"),
        ceilings(&["review"], &[EffectClass::Read], 100),
        None,
    ))
    .expect("root occurrence is valid");
    let evidence = PackageUse {
        use_record: exact_ref("use/noncanonical"),
        occurrence: occurrence.record().occurrence.clone(),
        operation: operation("review"),
        invocation: exact_ref("invocation/noncanonical"),
        activation: None,
        provider: None,
        generation: occurrence.record().generation.clone(),
        evidence_at_use: vec![exact_ref("evidence/z"), exact_ref("evidence/a")],
        disposition: PackageUseDisposition::Attempted,
    };

    assert_eq!(
        admit_package_use(&occurrence, None, evidence),
        Err(PackageUseError::NonCanonicalEvidence)
    );
}

#[test]
fn lifecycle_facts_admit_only_against_exact_predecessors() {
    let occurrence = admit_root_occurrence(package_occurrence(
        "occurrence/lifecycle",
        release("0.0.1"),
        ceilings(&["review"], &[EffectClass::Read], 100),
        None,
    ))
    .expect("root occurrence is valid");
    let materialization = admit_package_materialization(
        &occurrence,
        PackageMaterialization {
            materialization: exact_ref("materialization/lifecycle"),
            occurrence: occurrence.record().occurrence.clone(),
            artifact: exact_ref("artifact/lifecycle"),
        },
    )
    .expect("materialization names its occurrence");
    let activation = admit_package_activation(
        &materialization,
        PackageActivation {
            activation: exact_ref("activation/lifecycle"),
            materialization: materialization.record().materialization.clone(),
            fence: ActivationFenceDigest::hash_canonical(&"fence")
                .expect("canonical activation fence"),
        },
    )
    .expect("activation names its materialization");
    assert_eq!(
        activation.record().materialization,
        materialization.record().materialization
    );

    let package_use = admit_package_use(
        &occurrence,
        Some(&activation),
        PackageUse {
            use_record: exact_ref("use/lifecycle"),
            occurrence: occurrence.record().occurrence.clone(),
            operation: operation("review"),
            invocation: exact_ref("invocation/lifecycle"),
            activation: Some(activation.record().activation.clone()),
            provider: None,
            generation: occurrence.record().generation.clone(),
            evidence_at_use: vec![exact_ref("evidence/lifecycle")],
            disposition: PackageUseDisposition::Completed,
        },
    )
    .expect("use names its occurrence");
    let native_effect = admit_package_native_effect(
        &occurrence,
        &package_use,
        PackageNativeEffect {
            effect_record: exact_ref("native-effect/lifecycle"),
            use_record: package_use.record().use_record.clone(),
            native_effect: exact_ref("native-effect/read-back"),
            owner: workspace_id("owner.repository"),
            effect: EffectClass::Read,
        },
    )
    .expect("native effect names use and allowed effect");
    assert_eq!(
        native_effect.record().use_record,
        package_use.record().use_record
    );

    let obligation = ObligationSubject {
        operation: operation("review"),
        requirement: CapabilityRequirementIdV1::parse("source-proof").expect("valid requirement"),
    };
    let settlement_evidence = exact_ref("settlement/evidence");
    let settlement_input = EvidenceSettlementInput {
        obligation: obligation.clone(),
        target: SettlementTarget::NativeEffect {
            effect: native_effect.record().effect,
            owner: native_effect.record().owner.clone(),
        },
        generation: occurrence.record().generation.clone(),
        provider_evidence: ProviderEvidenceCoordinate::NotApplicable,
        evidence: pending_evidence(),
        freshness: current_freshness("settlement/freshness"),
        contradictions: BTreeSet::new(),
        native_verification: NativeVerification::Verified {
            owner: native_effect.record().owner.clone(),
            evidence: native_effect.record().native_effect.clone(),
        },
        owner_policy: OwnerPolicyRequirement::NotRequired,
        expires_at: None,
        invalidation: None,
    };
    let checked_at = UnixTimeV1::new(20).expect("valid settlement time");
    let decision = settle_obligation(&settlement_input, checked_at);
    let settlement = admit_package_settlement(
        &occurrence,
        Some(&package_use),
        Some(&native_effect),
        &settlement_evidence,
        &settlement_input,
        checked_at,
        PackageSettlement {
            occurrence: occurrence.record().occurrence.clone(),
            settlement: exact_ref("settlement/lifecycle"),
            generation: occurrence.record().generation.clone(),
            obligation,
            use_record: Some(package_use.record().use_record.clone()),
            evidence: settlement_evidence.clone(),
            decision,
        },
    )
    .expect("settlement names exact occurrence and use");
    assert_eq!(
        settlement.record().use_record.as_ref(),
        Some(&package_use.record().use_record)
    );

    let substitute_effect = admit_package_native_effect(
        &occurrence,
        &package_use,
        PackageNativeEffect {
            effect_record: exact_ref("native-effect/substitute"),
            ..native_effect.record().clone()
        },
    )
    .expect("the alternate record is independently well-formed");
    let mut graph = PackageLineageGraph::default();
    assert_eq!(
        graph.admit_native_effect_settlement(
            &substitute_effect,
            &settlement,
            exact_ref("lineage/substitute-effect"),
        ),
        Err(PackageLineageError::RecordJoinMismatch)
    );
    graph
        .admit_native_effect_settlement(
            &native_effect,
            &settlement,
            exact_ref("lineage/exact-effect"),
        )
        .expect("settlement retains the exact admitted native-effect identity");
}

#[test]
fn invalidation_reaches_descendants_without_touching_unrelated_regions() {
    let occurrence = admit_root_occurrence(package_occurrence(
        "occurrence/invalidation-parent",
        release("0.0.1"),
        ceilings(&["review"], &[EffectClass::Read], 100),
        None,
    ))
    .expect("root occurrence is valid");
    let package_use = admit_package_use(
        &occurrence,
        None,
        PackageUse {
            use_record: exact_ref("use/invalidation-child"),
            occurrence: occurrence.record().occurrence.clone(),
            operation: operation("review"),
            invocation: exact_ref("invocation/invalidation-child"),
            activation: None,
            provider: None,
            generation: occurrence.record().generation.clone(),
            evidence_at_use: Vec::new(),
            disposition: PackageUseDisposition::Completed,
        },
    )
    .expect("use is bound to its exact occurrence");
    let generated = exact_ref("generated/invalidation-grandchild");
    let mut graph = PackageLineageGraph::default();
    graph
        .admit_use(
            &occurrence,
            None,
            &package_use,
            exact_ref("lineage/occurrence-use"),
        )
        .expect("admitted use joins its exact occurrence");
    graph
        .admit_generated_dependent(
            PackageGeneratedDependencySource::Use(&package_use),
            generated.clone(),
            exact_ref("lineage/use-generated"),
        )
        .expect("generated output joins its exact use");

    let occurrence_node = PackageLineageNode::Occurrence(occurrence.record().occurrence.clone());
    let use_node = PackageLineageNode::Use(package_use.record().use_record.clone());
    let generated_node = PackageLineageNode::GeneratedDependent(generated);

    let invalidation = PackageInvalidation {
        kind: PackageInvalidationKind::Ancestry,
        evidence: exact_ref("invalidation/parent"),
    };
    let closure = propagate_package_invalidation(
        &graph,
        [
            occurrence_node.clone(),
            PackageLineageNode::Occurrence(exact_ref("occurrence/unknown")),
        ],
        invalidation.clone(),
    );
    assert_eq!(closure.invalidation, invalidation);
    assert_eq!(
        closure.affected,
        BTreeSet::from([occurrence_node, use_node, generated_node])
    );
}

#[test]
fn denied_use_cannot_acquire_native_effect_evidence() {
    let mut occurrence = package_occurrence(
        "occurrence/denied-effect",
        release("0.0.1"),
        ceilings(&["publish"], &[EffectClass::Publish], 100),
        None,
    );
    occurrence.context.effect = EffectClass::Publish;
    let occurrence = admit_root_occurrence(occurrence).expect("root occurrence is valid");
    let denied = admit_package_use(
        &occurrence,
        None,
        PackageUse {
            use_record: exact_ref("use/denied-effect"),
            occurrence: occurrence.record().occurrence.clone(),
            operation: operation("publish"),
            invocation: exact_ref("invocation/denied-effect"),
            activation: None,
            provider: None,
            generation: occurrence.record().generation.clone(),
            evidence_at_use: Vec::new(),
            disposition: PackageUseDisposition::Denied,
        },
    )
    .expect("a denied use claim still preserves its occurrence join");

    assert_eq!(
        admit_package_native_effect(
            &occurrence,
            &denied,
            PackageNativeEffect {
                effect_record: exact_ref("native-effect/impossible"),
                use_record: denied.record().use_record.clone(),
                native_effect: exact_ref("native-effect/impossible/read-back"),
                owner: workspace_id("owner.publisher"),
                effect: EffectClass::Publish,
            },
        ),
        Err(PackageNativeEffectError::DeniedUse)
    );
}

fn pack_admission(precedence: PackPrecedence) -> NormativePackReferenceAdmission {
    NormativePackReferenceAdmission {
        admission: exact_ref("pack/admission"),
        release: pack_release("2026.1"),
        authority: exact_ref("pack/authority"),
        rights: NormativePackUseRights {
            context: exact_ref("pack/right/context"),
            evaluation: exact_ref("pack/right/evaluation"),
            training: None,
            derivative: None,
            genericization: None,
            publication: None,
        },
        scope: exact_ref("pack/scope"),
        effective_from: UnixTimeV1::new(10).expect("valid time"),
        effective_until: Some(UnixTimeV1::new(20).expect("valid time")),
        precedence,
        supersedes: None,
        revocation: exact_ref("pack/revocation"),
        use_evidence: exact_ref("pack/use-evidence"),
        decommission: exact_ref("pack/decommission"),
    }
}

#[test]
fn pack_admission_requires_owner_precedence_and_a_valid_interval() {
    let unresolved = pack_admission(PackPrecedence::Unresolved {
        conflicting_release: pack_release("2026.2"),
    });
    assert_eq!(
        admit_normative_pack_reference(unresolved),
        Err(NormativePackReferenceError::UnresolvedConflict)
    );

    let mut reversed = pack_admission(PackPrecedence::Uncontested);
    reversed.effective_until = Some(UnixTimeV1::new(10).expect("valid time"));
    assert_eq!(
        admit_normative_pack_reference(reversed),
        Err(NormativePackReferenceError::InvalidEffectiveInterval)
    );

    let mut self_superseding = pack_admission(PackPrecedence::Uncontested);
    self_superseding.supersedes = Some(self_superseding.release.clone());
    assert_eq!(
        admit_normative_pack_reference(self_superseding),
        Err(NormativePackReferenceError::InvalidSupersession)
    );

    let mut foreign_owner = pack_admission(PackPrecedence::Uncontested);
    let mut predecessor = pack_release("2025.4");
    predecessor.attestor =
        NormativePackAttestorV1::parse("other.attestor").expect("valid attestor id");
    foreign_owner.supersedes = Some(predecessor);
    assert_eq!(
        admit_normative_pack_reference(foreign_owner),
        Err(NormativePackReferenceError::InvalidSupersession)
    );

    let mut foreign_pack = pack_admission(PackPrecedence::Uncontested);
    let mut predecessor = pack_release("2025.4");
    predecessor.id = NormativePackIdV1::parse("another.pack").expect("valid Pack id");
    foreign_pack.supersedes = Some(predecessor);
    assert_eq!(
        admit_normative_pack_reference(foreign_pack),
        Err(NormativePackReferenceError::InvalidSupersession)
    );

    let mut successor = pack_admission(PackPrecedence::Uncontested);
    successor.supersedes = Some(pack_release("2025.4"));
    assert!(admit_normative_pack_reference(successor).is_ok());
}

#[test]
fn pack_use_and_invalidation_bind_the_exact_admission() {
    let admission = admit_normative_pack_reference(pack_admission(PackPrecedence::Uncontested))
        .expect("Pack reference is admissible");
    let mut occurrence = package_occurrence(
        "occurrence/pack-use",
        release("0.0.1"),
        ceilings(&["review"], &[EffectClass::Read], 100),
        None,
    );
    occurrence.normative_packs = vec![admission.clone()];
    let occurrence = admit_root_occurrence(occurrence).expect("root occurrence selects the Pack");
    let pack_use = NormativePackUse {
        use_record: exact_ref("pack/use"),
        admission: admission.record().admission.clone(),
        occurrence: occurrence.record().occurrence.clone(),
        purpose: NormativePackUsePurpose::Context,
        right: admission.record().rights.context.clone(),
        evidence_contract: admission.record().use_evidence.clone(),
        evidence_at_use: exact_ref("pack/use-observation"),
        used_at: UnixTimeV1::new(15).expect("valid use time"),
    };
    let pack_use = admit_normative_pack_use(&occurrence, &admission, pack_use)
        .expect("Pack use carries the exact selected right");

    let mut graph = PackageLineageGraph::default();
    graph
        .admit_normative_pack_dependency(
            &admission,
            &occurrence,
            exact_ref("lineage/pack-occurrence"),
        )
        .expect("Pack admission causes its selected occurrence");
    graph
        .admit_normative_pack_use(
            &admission,
            &occurrence,
            &pack_use,
            exact_ref("lineage/pack-use"),
        )
        .expect("Pack use joins its two exact causal parents atomically");
    let generated = exact_ref("generated/from-pack-use");
    graph
        .admit_generated_dependent(
            PackageGeneratedDependencySource::NormativePackUse(&pack_use),
            generated.clone(),
            exact_ref("lineage/pack-generated"),
        )
        .expect("generated output depends on the admitted Pack use");

    let invalidation_evidence = exact_ref("pack/invalidation-evidence");
    let invalidation = propagate_package_invalidation(
        &graph,
        [PackageLineageNode::NormativePackAdmission(
            admission.record().admission.clone(),
        )],
        PackageInvalidation {
            kind: PackageInvalidationKind::Dependency,
            evidence: invalidation_evidence,
        },
    );
    assert_eq!(
        invalidation.affected,
        BTreeSet::from([
            PackageLineageNode::NormativePackAdmission(admission.record().admission.clone()),
            PackageLineageNode::NormativePackUse(pack_use.record().use_record.clone()),
            PackageLineageNode::Occurrence(occurrence.record().occurrence.clone()),
            PackageLineageNode::GeneratedDependent(generated),
        ])
    );

    let mut missing_right = pack_use.record().clone();
    missing_right.purpose = NormativePackUsePurpose::Publication;
    assert_eq!(
        admit_normative_pack_use(&occurrence, &admission, missing_right),
        Err(NormativePackUseError::RightMismatch)
    );

    let mut substituted_record = admission.record().clone();
    substituted_record.authority = exact_ref("pack/substituted-authority");
    let substituted = admit_normative_pack_reference(substituted_record)
        .expect("the alternate record is independently structurally admissible");
    let mut substituted_use = pack_use.record().clone();
    substituted_use.right = substituted.record().rights.context.clone();
    assert_eq!(
        admit_normative_pack_use(&occurrence, &substituted, substituted_use),
        Err(NormativePackUseError::OccurrenceMismatch)
    );
    assert_eq!(
        PackageLineageGraph::default().admit_normative_pack_dependency(
            &substituted,
            &occurrence,
            exact_ref("lineage/substituted-pack"),
        ),
        Err(PackageLineageError::RecordJoinMismatch)
    );

    let mut child = package_occurrence(
        "occurrence/substituted-pack-child",
        occurrence.record().release.clone(),
        occurrence.record().ceilings.clone(),
        Some(occurrence.record().occurrence.clone()),
    );
    preserve_parent_context(&occurrence, &mut child);
    child.normative_packs = vec![substituted];
    assert_eq!(
        derive_child_occurrence(&occurrence, child),
        Err(ChildOccurrenceError::NormativePackWidening)
    );
}
