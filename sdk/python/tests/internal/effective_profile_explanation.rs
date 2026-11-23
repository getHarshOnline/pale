//! Owner-value projection laws; synthetic SDK variants are not native admission evidence.

use pale_sdk::{
    EffectiveProfileDeclaredSelection as Declared, EffectiveProfileDeclaredSelector as Selector,
    EffectiveProfileExplanation as Explanation, EffectiveProfileExplanationEvidence as Evidence,
    EffectiveProfileExplanationHandle, EffectiveProfileFinding as Finding,
    EffectiveProfileModeAxis as Axis, EffectiveProfileNativeRealization as Native,
    EffectiveProfileOperationExposure as Exposure, EffectiveProfilePathReason as Reason,
    EffectiveProfileProviderDisposition as Provider, EffectiveProfileProviderKind as ProviderKind,
    EffectiveProfileRequirementDisposition as Requirement,
    EffectiveProfileResourceKind as Resource, EffectiveProfileSelection as Selection,
};

use crate::effective_profile_explanation::PyEffectiveProfileExplanationEvidence;
use crate::*;

fn evidence(inner: Evidence) -> PyEffectiveProfileExplanationEvidence {
    let explanation = PyEffectiveProfileExplanation::from_api(Explanation {
        handle: EffectiveProfileExplanationHandle::parse("a".repeat(64)).unwrap(),
        evidence: inner,
    });
    assert_eq!(explanation.handle(), "a".repeat(64));
    explanation.evidence()
}

#[test]
fn all_nine_evidence_variants_preserve_required_payloads() {
    use super::effective_profile_entries::{assert_value, Family};
    use pyo3::prelude::*;

    let sources = [
        Evidence::DeclaredSelection {
            selection: Declared::Resource {
                resource_kind: Resource::Recipe,
                selector: Selector::Exact {
                    resource_id: "recipe/λ".into(),
                },
            },
        },
        Evidence::Responsibility {
            contract_id: "contract".into(),
            execution_path_id: "path".into(),
            reason_steps: vec![
                Reason::ProviderBinding {
                    binding_id: "b".into(),
                },
                Reason::DelegationEdge {
                    edge_id: "e".into(),
                },
            ],
        },
        Evidence::EffectiveSelection {
            selection: Selection::RunPolicy {
                run_policy_id: "policy".into(),
            },
        },
        Evidence::ExecutionPath {
            selection_id: "selected".into(),
            reason_steps: vec![],
        },
        Evidence::OperationClassification {
            exposure: Exposure::SemanticOnly,
            native_realization: Native::Unknown,
        },
        Evidence::CapabilityClassification {
            requirement_disposition: Requirement::Blocked {
                reason: pale_sdk::EffectiveProfilePolicyBlock::RecipeContractMismatch,
            },
            provider_disposition: Provider::RecipeContractMismatch,
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
    Python::initialize();
    Python::attach(|py| {
        for source in sources {
            let expected = serde_json::to_value(&source).unwrap();
            // The parent explanation is dropped before the copied result is read.
            let actual = evidence(source).into_pyobject(py).unwrap();
            assert_value(&actual, &expected, Family::Evidence);
            if actual.hasattr("reason_steps").unwrap() {
                let first = actual.getattr("reason_steps").unwrap();
                first.call_method0("clear").unwrap();
                // Mutating a returned Python list does not alter retained evidence.
                assert_value(&actual, &expected, Family::Evidence);
            }
        }
    });
}

#[test]
fn declared_selection_preserves_both_selector_and_mode_families() {
    for resource_kind in [
        Resource::Operation,
        Resource::Role,
        Resource::Doctrine,
        Resource::StartupPolicy,
        Resource::RunMode,
        Resource::Stack,
        Resource::PolicyFamily,
        Resource::Recipe,
    ] {
        for selector in [
            Selector::Exact {
                resource_id: "r/λ".into(),
            },
            Selector::AllFromPackage {
                package_id: "package".into(),
            },
        ] {
            super::effective_profile_entries::assert_row(
                pale_sdk::EffectiveProfileEntryDetail::DeclaredSelection {
                    selection: Declared::Resource {
                        resource_kind,
                        selector,
                    },
                },
            );
        }
    }
    for axis in [Axis::Intensity, Axis::TaskStance] {
        super::effective_profile_entries::assert_row(
            pale_sdk::EffectiveProfileEntryDetail::DeclaredSelection {
                selection: Declared::ModeDefault {
                    axis,
                    resource_id: "mode/λ".into(),
                },
            },
        );
    }
}

#[test]
fn all_eight_selection_payloads_preserve_fields_and_full_width_counts() {
    for selection in [
        Selection::Package {
            package_id: "pkg/λ".into(),
            version: "v".into(),
            content_digest: "digest".into(),
        },
        Selection::NormativePack {
            pack_id: "pack".into(),
            attestor_id: "attestor".into(),
            version: "v2".into(),
            content_digest: "digest2".into(),
        },
        Selection::SemanticResource {
            repository_id: "repo".into(),
            resource_kind: Resource::Recipe,
            resource_id: "recipe".into(),
        },
        Selection::RunPolicy {
            run_policy_id: "run".into(),
        },
        Selection::Session {
            execution_workspace_id: "session".into(),
            activation_root_count: u32::MAX,
            additional_root_count: 0,
            activation_path_count: 7,
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
            authority_coordinate: "ceiling".into(),
        },
    ] {
        super::effective_profile_entries::assert_row(
            pale_sdk::EffectiveProfileEntryDetail::Selection { selection },
        );
    }
}

#[test]
fn providers_findings_and_reasons_retain_exact_nested_payloads() {
    use super::effective_profile_entries::{assert_value, Family};
    use pyo3::prelude::*;
    Python::initialize();
    Python::attach(|py| {
        for source in [
            Provider::Bound {
                provider: ProviderKind::HostNative,
            },
            Provider::Bound {
                provider: ProviderKind::NativeRust,
            },
            Provider::Bound {
                provider: ProviderKind::Connector,
            },
            Provider::MissingBinding,
            Provider::ExcludedByNarrowing,
            Provider::EffectMismatch,
            Provider::RecipeContractMismatch,
        ] {
            let expected = serde_json::to_value(source).unwrap();
            let actual = PyEffectiveProfileProviderDisposition::from_api(source)
                .into_pyobject(py)
                .unwrap();
            assert_value(&actual, &expected, Family::ProviderDisposition);
        }
        for source in [
            Finding::NativeRealizationUnknown {
                machine_id: "machine/λ".into(),
            },
            Finding::OperationUnbound {
                repository_id: "repo".into(),
                operation_id: "op".into(),
            },
            Finding::Requirement {
                repository_id: "repo2".into(),
                operation_id: "op2".into(),
                requirement_id: "req".into(),
                capability_id: "cap".into(),
                disposition: Requirement::Narrowed {
                    reason: pale_sdk::EffectiveProfileNarrowingReason::CapabilityNotPermitted,
                },
            },
        ] {
            let expected = serde_json::to_value(&source).unwrap();
            let actual = PyEffectiveProfileFinding::from_api(source)
                .into_pyobject(py)
                .unwrap();
            assert_value(&actual, &expected, Family::Finding);
        }
        for source in [
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
        ] {
            let expected = serde_json::to_value(&source).unwrap();
            let actual = PyEffectiveProfilePathReason::from_api(source)
                .into_pyobject(py)
                .unwrap();
            assert_value(&actual, &expected, Family::PathReason);
        }
    });
}

#[test]
fn scalar_profile_families_are_exhaustively_projected() {
    for (source, expected) in [
        (
            Resource::Operation,
            PyEffectiveProfileResourceKind::Operation,
        ),
        (Resource::Role, PyEffectiveProfileResourceKind::Role),
        (Resource::Doctrine, PyEffectiveProfileResourceKind::Doctrine),
        (
            Resource::StartupPolicy,
            PyEffectiveProfileResourceKind::StartupPolicy,
        ),
        (Resource::RunMode, PyEffectiveProfileResourceKind::RunMode),
        (Resource::Stack, PyEffectiveProfileResourceKind::Stack),
        (
            Resource::PolicyFamily,
            PyEffectiveProfileResourceKind::PolicyFamily,
        ),
        (Resource::Recipe, PyEffectiveProfileResourceKind::Recipe),
    ] {
        assert!(PyEffectiveProfileResourceKind::from_api(source) == expected);
    }
    for (source, expected) in [
        (Axis::Intensity, PyEffectiveProfileModeAxis::Intensity),
        (Axis::TaskStance, PyEffectiveProfileModeAxis::TaskStance),
    ] {
        assert!(PyEffectiveProfileModeAxis::from_api(source) == expected);
    }
    assert!(
        PyEffectiveProfileNativeRealization::from_api(Native::Unknown)
            == PyEffectiveProfileNativeRealization::Unknown
    );
}
