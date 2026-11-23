//! Exhaustive value projection proof, separate from the native fixture's subset.

use std::mem::discriminant;

use crate::*;
use pale_sdk::{
    EffectiveProfileDeclaredSelection as Declared, EffectiveProfileDeclaredSelector as Selector,
    EffectiveProfileExplanationEvidence as Evidence, EffectiveProfileFinding as Finding,
    EffectiveProfileModeAxis as Mode, EffectiveProfileNativeRealization as Native,
    EffectiveProfilePathReason as Reason, EffectiveProfileProviderDisposition as Provider,
    EffectiveProfileProviderKind as ProviderKind, EffectiveProfileResourceKind as Resource,
    EffectiveProfileSelection as Selection,
};

#[test]
fn every_evidence_variant_preserves_its_payload_and_only_its_axes() {
    let sources = [
        Evidence::DeclaredSelection {
            selection: Declared::ModeDefault {
                axis: Mode::Intensity,
                resource_id: "mode".into(),
            },
        },
        Evidence::Responsibility {
            contract_id: "contract".into(),
            execution_path_id: "path".into(),
            reason_steps: vec![
                Reason::EntryContract {
                    contract_id: "entry".into(),
                },
                Reason::ProviderBinding {
                    binding_id: "binding".into(),
                },
            ],
        },
        Evidence::EffectiveSelection {
            selection: Selection::RunPolicy {
                run_policy_id: "policy".into(),
            },
        },
        Evidence::ExecutionPath {
            selection_id: "selection".into(),
            reason_steps: vec![],
        },
        Evidence::OperationClassification {
            exposure: pale_sdk::EffectiveProfileOperationExposure::SemanticOnly,
            native_realization: Native::Unknown,
        },
        Evidence::CapabilityClassification {
            requirement_disposition:
                pale_sdk::EffectiveProfileRequirementDisposition::ProviderBindingAbsent,
            provider_disposition: Provider::Bound {
                provider: ProviderKind::Connector,
            },
            native_realization: Native::Unknown,
        },
        Evidence::FindingClassification {
            finding: Finding::NativeRealizationUnknown {
                machine_id: "machine".into(),
            },
        },
        Evidence::MachineRealization {
            realization: Native::Unknown,
        },
        Evidence::Configuration,
    ];
    for source in sources {
        let value = EffectiveProfileExplanationEvidence::from_api(source.clone());
        match (source, value) {
            (
                Evidence::DeclaredSelection { .. },
                EffectiveProfileExplanationEvidence::DeclaredSelection { selection },
            ) => {
                assert!(
                    matches!(selection, EffectiveProfileDeclaredSelection::ModeDefault { axis: EffectiveProfileModeAxis::Intensity, resource_id } if resource_id == "mode")
                );
            }
            (
                Evidence::Responsibility {
                    contract_id: a,
                    execution_path_id: b,
                    ..
                },
                EffectiveProfileExplanationEvidence::Responsibility {
                    contract_id,
                    execution_path_id,
                    reason_steps,
                },
            ) => {
                assert_eq!((contract_id, execution_path_id), (a, b));
                assert!(matches!(reason_steps.as_slice(), [
                    EffectiveProfilePathReason::EntryContract { contract_id },
                    EffectiveProfilePathReason::ProviderBinding { binding_id },
                ] if contract_id == "entry" && binding_id == "binding"));
            }
            (
                Evidence::EffectiveSelection { .. },
                EffectiveProfileExplanationEvidence::EffectiveSelection { selection },
            ) => {
                assert!(
                    matches!(selection, EffectiveProfileSelection::RunPolicy { run_policy_id } if run_policy_id == "policy")
                );
            }
            (
                Evidence::ExecutionPath {
                    selection_id: a, ..
                },
                EffectiveProfileExplanationEvidence::ExecutionPath {
                    selection_id,
                    reason_steps,
                },
            ) => {
                assert_eq!(selection_id, a);
                assert!(reason_steps.is_empty());
            }
            (
                Evidence::OperationClassification { .. },
                EffectiveProfileExplanationEvidence::OperationClassification {
                    exposure,
                    native_realization,
                },
            ) => {
                assert!(matches!(exposure, OperationExposure::SemanticOnly));
                assert!(matches!(
                    native_realization,
                    EffectiveProfileNativeRealization::Unknown
                ));
            }
            (
                Evidence::CapabilityClassification { .. },
                EffectiveProfileExplanationEvidence::CapabilityClassification {
                    requirement_disposition,
                    provider_disposition,
                    native_realization,
                },
            ) => {
                assert!(matches!(
                    requirement_disposition,
                    RequirementDisposition::ProviderBindingAbsent {}
                ));
                assert!(matches!(
                    provider_disposition,
                    EffectiveProfileProviderDisposition::Bound {
                        provider: EffectiveProfileProviderKind::Connector
                    }
                ));
                assert!(matches!(
                    native_realization,
                    EffectiveProfileNativeRealization::Unknown
                ));
            }
            (
                Evidence::FindingClassification { .. },
                EffectiveProfileExplanationEvidence::FindingClassification { finding },
            ) => {
                assert!(
                    matches!(finding, EffectiveProfileFinding::NativeRealizationUnknown { machine_id } if machine_id == "machine")
                );
            }
            (
                Evidence::MachineRealization { .. },
                EffectiveProfileExplanationEvidence::MachineRealization { realization },
            ) => {
                assert!(matches!(
                    realization,
                    EffectiveProfileNativeRealization::Unknown
                ));
            }
            (Evidence::Configuration, EffectiveProfileExplanationEvidence::Configuration {}) => {}
            _ => panic!("evidence variant changed"),
        }
    }
}

#[test]
fn declared_selectors_keep_both_families_and_all_resource_and_mode_kinds() {
    let resources = [
        (Resource::Operation, EffectiveProfileResourceKind::Operation),
        (Resource::Role, EffectiveProfileResourceKind::Role),
        (Resource::Doctrine, EffectiveProfileResourceKind::Doctrine),
        (
            Resource::StartupPolicy,
            EffectiveProfileResourceKind::StartupPolicy,
        ),
        (Resource::RunMode, EffectiveProfileResourceKind::RunMode),
        (Resource::Stack, EffectiveProfileResourceKind::Stack),
        (
            Resource::PolicyFamily,
            EffectiveProfileResourceKind::PolicyFamily,
        ),
        (Resource::Recipe, EffectiveProfileResourceKind::Recipe),
    ];
    for (resource_kind, expected) in resources {
        for selector in [
            Selector::Exact {
                resource_id: "resource/λ".into(),
            },
            Selector::AllFromPackage {
                package_id: "package".into(),
            },
        ] {
            let source = Declared::Resource {
                resource_kind,
                selector,
            };
            let value = EffectiveProfileDeclaredSelection::from_api(&source);
            let EffectiveProfileDeclaredSelection::Resource {
                resource_kind,
                selector,
            } = value
            else {
                panic!("resource declaration changed variant");
            };
            assert_eq!(discriminant(&resource_kind), discriminant(&expected));
            match (source, selector) {
                (
                    Declared::Resource {
                        selector: Selector::Exact { resource_id },
                        ..
                    },
                    EffectiveProfileDeclaredSelector::Exact {
                        resource_id: actual,
                    },
                ) => assert_eq!(actual, resource_id),
                (
                    Declared::Resource {
                        selector: Selector::AllFromPackage { package_id },
                        ..
                    },
                    EffectiveProfileDeclaredSelector::AllFromPackage { package_id: actual },
                ) => assert_eq!(actual, package_id),
                _ => panic!("selector changed variant"),
            }
        }
    }
    for (axis, expected) in [
        (Mode::Intensity, EffectiveProfileModeAxis::Intensity),
        (Mode::TaskStance, EffectiveProfileModeAxis::TaskStance),
    ] {
        let value = {
            let source = Declared::ModeDefault {
                axis,
                resource_id: "mode/λ".into(),
            };
            EffectiveProfileDeclaredSelection::from_api(&source)
        };
        let EffectiveProfileDeclaredSelection::ModeDefault { axis, resource_id } = value else {
            panic!("mode declaration changed variant");
        };
        assert_eq!(discriminant(&axis), discriminant(&expected));
        assert_eq!(resource_id, "mode/λ");
    }
}

#[test]
fn all_selected_coordinates_keep_distinct_fields_and_zero_cardinality() {
    let sources = [
        Selection::Package {
            package_id: "package".into(),
            version: "1".into(),
            content_digest: "package-digest".into(),
        },
        Selection::NormativePack {
            pack_id: "pack".into(),
            attestor_id: "attestor".into(),
            version: "2".into(),
            content_digest: "pack-digest".into(),
        },
        Selection::SemanticResource {
            repository_id: "repository".into(),
            resource_kind: Resource::Recipe,
            resource_id: "resource".into(),
        },
        Selection::RunPolicy {
            run_policy_id: "policy".into(),
        },
        Selection::Session {
            execution_workspace_id: "workspace".into(),
            activation_root_count: 0,
            additional_root_count: u32::MAX,
            activation_path_count: 3,
        },
        Selection::Lifecycle {
            lifecycle_id: "lifecycle".into(),
            event_id: "event".into(),
            action_id: "action".into(),
        },
        Selection::AuthorityRoot {
            authority_root_id: "root".into(),
        },
        Selection::DisclosureCeiling {
            authority_coordinate: "coordinate".into(),
        },
    ];
    for source in &sources {
        let value = EffectiveProfileSelection::from_api(source);
        match (source, value) {
            (
                Selection::Package {
                    package_id: a,
                    version: b,
                    content_digest: c,
                },
                EffectiveProfileSelection::Package {
                    package_id,
                    version,
                    content_digest,
                },
            ) => assert_eq!((&package_id, &version, &content_digest), (a, b, c)),
            (
                Selection::NormativePack {
                    pack_id: a,
                    attestor_id: b,
                    version: c,
                    content_digest: d,
                },
                EffectiveProfileSelection::NormativePack {
                    pack_id,
                    attestor_id,
                    version,
                    content_digest,
                },
            ) => assert_eq!(
                (&pack_id, &attestor_id, &version, &content_digest),
                (a, b, c, d)
            ),
            (
                Selection::SemanticResource {
                    repository_id: a,
                    resource_kind,
                    resource_id: b,
                },
                EffectiveProfileSelection::SemanticResource {
                    repository_id,
                    resource_kind: actual,
                    resource_id,
                },
            ) => {
                assert_eq!((&repository_id, &resource_id), (a, b));
                assert_eq!(discriminant(resource_kind), discriminant(&Resource::Recipe));
                assert_eq!(
                    discriminant(&actual),
                    discriminant(&EffectiveProfileResourceKind::Recipe)
                );
            }
            (
                Selection::RunPolicy { run_policy_id: a },
                EffectiveProfileSelection::RunPolicy { run_policy_id },
            ) => assert_eq!(&run_policy_id, a),
            (
                Selection::Session {
                    execution_workspace_id: a,
                    activation_root_count: b,
                    additional_root_count: c,
                    activation_path_count: d,
                },
                EffectiveProfileSelection::Session {
                    execution_workspace_id,
                    activation_root_count,
                    additional_root_count,
                    activation_path_count,
                },
            ) => assert_eq!(
                (
                    &execution_workspace_id,
                    &activation_root_count,
                    &additional_root_count,
                    &activation_path_count
                ),
                (a, b, c, d)
            ),
            (
                Selection::Lifecycle {
                    lifecycle_id: a,
                    event_id: b,
                    action_id: c,
                },
                EffectiveProfileSelection::Lifecycle {
                    lifecycle_id,
                    event_id,
                    action_id,
                },
            ) => assert_eq!((&lifecycle_id, &event_id, &action_id), (a, b, c)),
            (
                Selection::AuthorityRoot {
                    authority_root_id: a,
                },
                EffectiveProfileSelection::AuthorityRoot { authority_root_id },
            ) => assert_eq!(&authority_root_id, a),
            (
                Selection::DisclosureCeiling {
                    authority_coordinate: a,
                },
                EffectiveProfileSelection::DisclosureCeiling {
                    authority_coordinate,
                },
            ) => assert_eq!(&authority_coordinate, a),
            _ => panic!("selection variant changed"),
        }
    }
    let copied = {
        let source = Selection::RunPolicy {
            run_policy_id: "policy/λ".into(),
        };
        EffectiveProfileSelection::from_api(&source)
    };
    assert!(
        matches!(copied, EffectiveProfileSelection::RunPolicy { run_policy_id } if run_policy_id == "policy/λ")
    );
}

#[test]
fn all_provider_outcomes_keep_bound_provider_separate_from_readiness() {
    for (provider, expected) in [
        (
            ProviderKind::HostNative,
            EffectiveProfileProviderKind::HostNative,
        ),
        (
            ProviderKind::NativeRust,
            EffectiveProfileProviderKind::NativeRust,
        ),
        (
            ProviderKind::Connector,
            EffectiveProfileProviderKind::Connector,
        ),
    ] {
        let value = EffectiveProfileProviderDisposition::from_api(Provider::Bound { provider });
        let EffectiveProfileProviderDisposition::Bound { provider } = value else {
            panic!("bound provider changed variant")
        };
        assert_eq!(discriminant(&provider), discriminant(&expected));
    }
    for source in [
        Provider::MissingBinding,
        Provider::ExcludedByNarrowing,
        Provider::EffectMismatch,
        Provider::RecipeContractMismatch,
    ] {
        let value = EffectiveProfileProviderDisposition::from_api(source);
        assert!(matches!(
            (source, value),
            (
                Provider::MissingBinding,
                EffectiveProfileProviderDisposition::MissingBinding {}
            ) | (
                Provider::ExcludedByNarrowing,
                EffectiveProfileProviderDisposition::ExcludedByNarrowing {}
            ) | (
                Provider::EffectMismatch,
                EffectiveProfileProviderDisposition::EffectMismatch {}
            ) | (
                Provider::RecipeContractMismatch,
                EffectiveProfileProviderDisposition::RecipeContractMismatch {}
            )
        ));
    }
}

#[test]
fn all_findings_keep_native_operation_and_requirement_axes_distinct() {
    let sources = [
        Finding::NativeRealizationUnknown {
            machine_id: "machine/λ".into(),
        },
        Finding::OperationUnbound {
            repository_id: "repository".into(),
            operation_id: "operation".into(),
        },
        Finding::Requirement {
            repository_id: "repository".into(),
            operation_id: "operation".into(),
            requirement_id: "requirement".into(),
            capability_id: "capability".into(),
            disposition: pale_sdk::EffectiveProfileRequirementDisposition::Satisfied,
        },
    ];
    for source in sources {
        let value = EffectiveProfileFinding::from_api(&source);
        match (source, value) {
            (
                Finding::NativeRealizationUnknown {
                    machine_id: expected,
                },
                EffectiveProfileFinding::NativeRealizationUnknown { machine_id },
            ) => assert_eq!(machine_id, expected),
            (
                Finding::OperationUnbound {
                    repository_id: a,
                    operation_id: b,
                },
                EffectiveProfileFinding::OperationUnbound {
                    repository_id,
                    operation_id,
                },
            ) => assert_eq!((repository_id, operation_id), (a, b)),
            (
                Finding::Requirement {
                    repository_id: a,
                    operation_id: b,
                    requirement_id: c,
                    capability_id: d,
                    ..
                },
                EffectiveProfileFinding::Requirement {
                    repository_id,
                    operation_id,
                    requirement_id,
                    capability_id,
                    disposition,
                },
            ) => {
                assert_eq!(
                    (repository_id, operation_id, requirement_id, capability_id),
                    (a, b, c, d)
                );
                assert!(matches!(disposition, RequirementDisposition::Satisfied {}));
            }
            _ => panic!("finding variant changed"),
        }
    }
}

#[test]
fn eight_profile_path_reasons_keep_exact_field_and_kind_identity() {
    let sources = [
        Reason::OriginReceipt {
            contract_id: "origin/λ".into(),
        },
        Reason::EntryContract {
            contract_id: "entry".into(),
        },
        Reason::ResponsibilityContract {
            contract_id: "responsibility".into(),
        },
        Reason::ProviderBinding {
            binding_id: "binding".into(),
        },
        Reason::DelegationEdge {
            edge_id: "edge".into(),
        },
        Reason::TerminationContract {
            contract_id: "termination".into(),
        },
        Reason::SettlementContract {
            contract_id: "settlement".into(),
        },
        Reason::ContinuityContract {
            contract_id: "continuity".into(),
        },
    ];
    for source in sources {
        let value = EffectiveProfilePathReason::from_api(&source);
        match (source, value) {
            (
                Reason::OriginReceipt { contract_id: a },
                EffectiveProfilePathReason::OriginReceipt { contract_id },
            )
            | (
                Reason::EntryContract { contract_id: a },
                EffectiveProfilePathReason::EntryContract { contract_id },
            )
            | (
                Reason::ResponsibilityContract { contract_id: a },
                EffectiveProfilePathReason::ResponsibilityContract { contract_id },
            )
            | (
                Reason::TerminationContract { contract_id: a },
                EffectiveProfilePathReason::TerminationContract { contract_id },
            )
            | (
                Reason::SettlementContract { contract_id: a },
                EffectiveProfilePathReason::SettlementContract { contract_id },
            )
            | (
                Reason::ContinuityContract { contract_id: a },
                EffectiveProfilePathReason::ContinuityContract { contract_id },
            ) => assert_eq!(contract_id, a),
            (
                Reason::ProviderBinding { binding_id: a },
                EffectiveProfilePathReason::ProviderBinding { binding_id },
            ) => assert_eq!(binding_id, a),
            (
                Reason::DelegationEdge { edge_id: a },
                EffectiveProfilePathReason::DelegationEdge { edge_id },
            ) => assert_eq!(edge_id, a),
            _ => panic!("path reason variant changed"),
        }
    }
}
