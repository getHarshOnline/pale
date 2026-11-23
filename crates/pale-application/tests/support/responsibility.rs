//! Reusable public-record fixtures for responsibility-derivation tests.

use std::fmt::Debug;
use std::str::FromStr;

use pale_core::records::*;

/// Parse one validated synthetic identity.
pub(super) fn fixture_id<T>(value: &str) -> T
where
    T: FromStr,
    T::Err: Debug,
{
    value.parse().expect("fixture identity is valid")
}

/// Build one attributable synthetic source coordinate.
pub(super) fn attribution(scope: &str) -> ResponsibilityAttributionV1 {
    ResponsibilityAttributionV1 {
        source_id: fixture_id(&format!("source.{scope}")),
        owner: fixture_id(&format!("owner.{scope}")),
        evidence_ref: fixture_id(&format!("evidence.{scope}")),
    }
}

/// Build one supplied attributable answer.
pub(super) fn supplied_answer<T>(scope: &str, value: T) -> AttributableAnswerV1<T> {
    AttributableAnswerV1::Supplied {
        value,
        attribution: attribution(scope),
    }
}

/// Build one explicit attributable omission.
pub(super) fn omitted_answer<T>(scope: &str) -> AttributableAnswerV1<T> {
    AttributableAnswerV1::Omitted {
        reason: fixture_id(&format!("not-required.{scope}")),
        attribution: attribution(scope),
    }
}

/// Build one bounded policy ceiling without ambient rights or authority.
pub(super) fn policy_ceiling(owner: &str) -> WorkspacePolicyCeilingV1 {
    WorkspacePolicyCeilingV1 {
        disclosure: DisclosureCeilingV1::Protected,
        effect: WorkspaceEffectCeilingV1::Read,
        model_use_permitted: false,
        retention_seconds: 300,
        rights: WorkspaceRightsCeilingV1 {
            permitted: vec![fixture_id(&format!("right.{owner}"))],
        },
        controllers: WorkspaceControllerCeilingV1 {
            permitted: vec![fixture_id(&format!("controller.{owner}"))],
        },
        authorities: WorkspaceAuthorityCeilingV1 {
            permitted: vec![fixture_id(&format!("authority.{owner}"))],
        },
    }
}

/// Build one distinct lifecycle-evidence requirement.
pub(super) fn evidence_requirement(
    owner: &str,
    class: &str,
) -> ResponsibilityEvidenceRequirementV1 {
    ResponsibilityEvidenceRequirementV1 {
        owner: fixture_id(&format!("owner.{owner}")),
        evidence_ref: fixture_id(&format!("evidence.{owner}.{class}")),
    }
}

/// Return the shared namespace-qualified service coordinate for one fixture.
pub(super) fn directional_service_contract(scope: &str) -> DirectionalServiceContractRefV1 {
    let consumer = match scope {
        "alpha" => "beta",
        "beta" => "alpha",
        _ => "consumer",
    };
    DirectionalServiceContractRefV1 {
        namespace: fixture_id(scope),
        contract_id: fixture_id("service.shared"),
        provider_scope: LogicalScopeRefV1 {
            namespace: fixture_id(scope),
            scope_id: fixture_id("scope"),
        },
        consumer_scope: LogicalScopeRefV1 {
            namespace: fixture_id(consumer),
            scope_id: fixture_id("scope"),
        },
    }
}

/// Build all structural questions with explicit selected or omitted answers.
fn structural_selections(
    scope: &str,
    directional: bool,
) -> Vec<ResponsibilityStructuralSelectionV1> {
    let service_contract = directional_service_contract(scope);
    StructuralObjectKindV1::all()
        .iter()
        .copied()
        .map(|kind| {
            let selected = matches!(
                kind,
                StructuralObjectKindV1::StableRole
                    | StructuralObjectKindV1::ReferenceOnly
                    | StructuralObjectKindV1::NativeApplication
            ) || (directional
                && kind == StructuralObjectKindV1::DirectionalServiceContract);
            let resource = if kind == StructuralObjectKindV1::DirectionalServiceContract {
                service_contract.resource_id()
            } else {
                fixture_id(&format!("resource.{scope}.{}", kind.as_str()))
            };
            ResponsibilityStructuralSelectionV1 {
                kind,
                selection: if selected {
                    supplied_answer(scope, resource)
                } else {
                    omitted_answer(scope)
                },
            }
        })
        .collect()
}

/// Build one complete public-record responsibility derivation.
///
/// Directional fixtures use the same qualified service coordinate in placement
/// and in the required structural selection, so they exercise the production
/// identity join rather than merely supplying two service-like strings.
pub(super) fn responsibility_derivation(
    scope: &str,
    projection_count: usize,
    directional: bool,
) -> ResponsibilityDerivationV1 {
    let derivation_id: ResponsibilityDerivationId = fixture_id(&format!("responsibility.{scope}"));
    let native_projections = (0..projection_count)
        .map(|index| ResponsibilityNativeProjectionV1 {
            projection_id: fixture_id(&format!("projection.{scope}.p{index}")),
            class: match index % 3 {
                0 => ResponsibilityProjectionClassV1::AccountableWork,
                1 => ResponsibilityProjectionClassV1::Communication,
                _ => ResponsibilityProjectionClassV1::Relationship,
            },
            native_owner: fixture_id(&format!("native-owner.{scope}.p{index}")),
            native_refs: vec![fixture_id(&format!("native-ref.{scope}.p{index}.a"))],
            state: EffectiveFactStateV1::Present,
        })
        .collect();
    ResponsibilityDerivationV1 {
        schema: RESPONSIBILITY_DERIVATION_V1_SCHEMA.into(),
        schema_version: RESPONSIBILITY_DERIVATION_V1_SCHEMA_VERSION,
        derivation_id,
        lifecycle: ResponsibilityDerivationLifecycleV1::AcceptedTarget,
        provenance: attribution(scope),
        responsibility: supplied_answer(
            scope,
            BoundedResponsibilityV1 {
                verb: fixture_id(&format!("verb.{scope}")),
                object: fixture_id(&format!("object.{scope}")),
                outcome: fixture_id(&format!("outcome.{scope}")),
            },
        ),
        native_owner: supplied_answer(
            scope,
            ResponsibilityNativeOwnerV1 {
                owner: fixture_id(&format!("native-owner.{scope}")),
                owner_class: ResponsibilityOwnerClassV1::NativeApplication,
                authority_resolver: fixture_id(&format!("resolver.{scope}")),
            },
        ),
        trigger: supplied_answer(
            scope,
            ResponsibilityTriggerV1::BoundedRequest {
                request_contract: fixture_id(&format!("request.{scope}")),
            },
        ),
        placement: supplied_answer(
            scope,
            if directional {
                ResponsibilityPlacementV1::DirectionalService {
                    service_contract: directional_service_contract(scope),
                }
            } else {
                ResponsibilityPlacementV1::VerticalLocal {
                    scope: fixture_id(&format!("local-scope.{scope}")),
                }
            },
        ),
        structural_objects: structural_selections(scope, directional),
        request_framer: supplied_answer(
            scope,
            ResponsibilityRequestFramerV1 {
                actor: fixture_id(&format!("actor.{scope}")),
                owner: fixture_id(&format!("framer-owner.{scope}")),
                authority_scope: fixture_id(&format!("framing-authority.{scope}")),
            },
        ),
        provider_completion: supplied_answer(
            scope,
            ProviderCompletionRequirementV1 {
                requirement: evidence_requirement(scope, "provider-completion"),
            },
        ),
        provider_acceptance: supplied_answer(
            scope,
            ProviderAcceptanceRequirementV1 {
                requirement: evidence_requirement(scope, "provider-acceptance"),
            },
        ),
        consumer_acceptance: supplied_answer(
            scope,
            ConsumerAcceptanceRequirementV1 {
                requirement: evidence_requirement(scope, "consumer-acceptance"),
            },
        ),
        native_result: supplied_answer(
            scope,
            NativeReadBackRequirementV1 {
                requirement: evidence_requirement(scope, "native-read-back"),
            },
        ),
        pale_settlement: supplied_answer(
            scope,
            PaleSettlementRequirementV1 {
                requirement: evidence_requirement(scope, "pale-settlement"),
            },
        ),
        lawful_silence: supplied_answer(
            scope,
            ResponsibilitySilenceBoundaryV1 {
                silent_surfaces: vec![ResponsibilitySilenceClassV1::Communication],
            },
        ),
        publisher_module: supplied_answer(
            scope,
            ResponsibilityPublisherModuleV1 {
                module_id: fixture_id(&format!("publisher.{scope}")),
                release_id: fixture_id("release.one"),
                export_id: fixture_id("responsibility.surface"),
                semantic_owner: fixture_id(&format!("semantic-owner.{scope}")),
            },
        ),
        adopter_module: supplied_answer(
            scope,
            ResponsibilityAdopterModuleV1 {
                module_id: fixture_id(&format!("adopter.{scope}")),
                imported_module_id: fixture_id(&format!("publisher.{scope}")),
                imported_release_id: fixture_id("release.one"),
                imported_export_id: fixture_id("responsibility.surface"),
                local_namespace: fixture_id(&format!("local.{scope}")),
                adoption_owner: fixture_id(&format!("adoption-owner.{scope}")),
            },
        ),
        ceilings: supplied_answer(
            scope,
            ResponsibilityCeilingsV1 {
                policy: policy_ceiling(scope),
                resources: vec![fixture_id(&format!("resource.{scope}.one"))],
            },
        ),
        incumbent: supplied_answer(
            scope,
            ResponsibilityIncumbentV1 {
                mechanism: fixture_id(&format!("incumbent.{scope}")),
                writer: fixture_id(&format!("writer.{scope}")),
                retained_evidence: fixture_id(&format!("retained.{scope}")),
            },
        ),
        readiness: supplied_answer(
            scope,
            ResponsibilityReadinessV1 {
                operation_contract: fixture_id(&format!("proof.{scope}.operation")),
                replay_proof: fixture_id(&format!("proof.{scope}.replay")),
                failure_proof: fixture_id(&format!("proof.{scope}.failure")),
                recovery_proof: fixture_id(&format!("proof.{scope}.recovery")),
                rollback_proof: fixture_id(&format!("proof.{scope}.rollback")),
                native_read_back_proof: fixture_id(&format!("proof.{scope}.read-back")),
            },
        ),
        safe_failure: supplied_answer(scope, ResponsibilitySafeFailureV1::RemainNative),
        direct_path: supplied_answer(
            scope,
            ResponsibilityDirectPathV1 {
                path: fixture_id(&format!("direct.{scope}")),
                native_owner: fixture_id(&format!("native-owner.{scope}")),
                availability_policy: fixture_id(&format!("availability.{scope}")),
            },
        ),
        residence: vec![ResponsibilityResidenceV1 {
            class: ResponsibilityResidenceClassV1::NativeApplication,
            resource: fixture_id(&format!("residence.{scope}")),
            owner: fixture_id(&format!("native-owner.{scope}")),
        }],
        native_projections,
        genealogy_links: Vec::new(),
    }
}
