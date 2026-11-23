//! Synthetic SDK variants exercised through actual Python value conversion.

use pale_sdk::{
    EffectiveProfileDeclaredSelection as Declared, EffectiveProfileDeclaredSelector as Selector,
    EffectiveProfileEffectClass as Effect, EffectiveProfileEntry,
    EffectiveProfileEntryDetail as Row, EffectiveProfileExplanationHandle,
    EffectiveProfileFinding as Finding, EffectiveProfileNativeRealization as Native,
    EffectiveProfileOperationExposure as Exposure, EffectiveProfileParticipant as Participant,
    EffectiveProfilePathReason as Reason, EffectiveProfileProviderBindingRef as Binding,
    EffectiveProfileProviderDisposition as Provider, EffectiveProfileRepositoryAccess as Access,
    EffectiveProfileRequirementDisposition as Requirement,
    EffectiveProfileResourceKind as Resource, EffectiveProfileResponsibility as Responsibility,
    EffectiveProfileSelection as Selection, PortableWorkspaceSubjectSelector as Subject,
};
use pyo3::prelude::*;
use pyo3::types::{PyAnyMethods, PyList, PyListMethods, PyString, PyTypeMethods};
use serde_json::Value;

use crate::PyEffectiveProfileEntry;

#[derive(Clone, Copy)]
pub(super) enum Family {
    Entry,
    Participant,
    Responsibility,
    Subject,
    Selection,
    DeclaredSelection,
    DeclaredSelector,
    Evidence,
    RequirementDisposition,
    ProviderDisposition,
    Finding,
    PathReason,
    Existing,
}

fn title(tag: &str) -> String {
    tag.split('-')
        .map(|part| {
            let mut chars = part.chars();
            let first = chars.next().unwrap().to_uppercase().collect::<String>();
            first + chars.as_str()
        })
        .collect()
}

fn field_name(name: &str) -> String {
    name.chars()
        .flat_map(|character| {
            if character.is_ascii_uppercase() {
                vec!['_', character.to_ascii_lowercase()]
            } else {
                vec![character]
            }
        })
        .collect()
}

fn child_family(field: &str) -> Family {
    match field {
        "participant" => Family::Participant,
        "responsibilities" => Family::Responsibility,
        "subject" => Family::Subject,
        "reasonSteps" => Family::PathReason,
        "requirementDisposition" | "disposition" => Family::RequirementDisposition,
        "providerDisposition" => Family::ProviderDisposition,
        "finding" => Family::Finding,
        _ => Family::Existing,
    }
}

pub(super) fn assert_value(actual: &Bound<'_, PyAny>, expected: &Value, family: Family) {
    match expected {
        Value::Null => assert!(actual.is_none()),
        Value::Bool(value) => assert_eq!(actual.extract::<bool>().unwrap(), *value),
        Value::Number(value) => {
            assert_eq!(actual.extract::<u64>().unwrap(), value.as_u64().unwrap())
        }
        Value::String(value) => {
            if actual.is_instance_of::<PyString>() {
                assert_eq!(actual.extract::<String>().unwrap(), *value);
            } else {
                let variant = actual.get_type().getattr(title(value)).unwrap();
                assert!(actual.eq(variant).unwrap());
            }
        }
        Value::Array(values) => {
            let list = actual.cast::<PyList>().unwrap();
            assert_eq!(list.len(), values.len());
            for (actual, expected) in list.iter().zip(values) {
                assert_value(&actual, expected, family);
            }
        }
        Value::Object(fields) => {
            if !matches!(family, Family::Existing) {
                let tag = fields["kind"].as_str().unwrap();
                let name = match family {
                    Family::Entry => format!("EffectiveProfile{}Detail", title(tag)),
                    Family::Participant => format!("EffectiveProfile{}Participant", title(tag)),
                    Family::Responsibility => {
                        format!("EffectiveProfile{}Responsibility", title(tag))
                    }
                    Family::Selection => format!("EffectiveProfile{}Selection", title(tag)),
                    Family::DeclaredSelection => {
                        format!("EffectiveProfileDeclared{}Selection", title(tag))
                    }
                    Family::DeclaredSelector => {
                        format!("EffectiveProfileDeclared{}Selector", title(tag))
                    }
                    Family::Evidence => format!("EffectiveProfile{}Evidence", title(tag)),
                    Family::RequirementDisposition => {
                        format!("{}RequirementDisposition", title(tag))
                    }
                    Family::ProviderDisposition => {
                        format!("EffectiveProfile{}ProviderDisposition", title(tag))
                    }
                    Family::Finding => format!("EffectiveProfile{}Finding", title(tag)),
                    Family::PathReason => format!("EffectiveProfile{}Reason", title(tag)),
                    Family::Subject if tag == "workspace" => "WholeWorkspaceSubjectValue".into(),
                    Family::Subject => format!("Workspace{}SubjectValue", title(tag)),
                    Family::Existing => unreachable!(),
                };
                assert_eq!(
                    actual
                        .get_type()
                        .name()
                        .unwrap()
                        .extract::<String>()
                        .unwrap(),
                    name
                );
                let mut actual_fields: Vec<String> = actual
                    .dir()
                    .unwrap()
                    .iter()
                    .map(|name| name.extract::<String>().unwrap())
                    .filter(|name| !name.starts_with('_'))
                    .collect();
                actual_fields.sort();
                let mut expected_fields: Vec<_> = fields
                    .keys()
                    .filter(|name| name.as_str() != "kind")
                    .map(|name| field_name(name))
                    .collect();
                expected_fields.sort();
                assert_eq!(
                    actual_fields, expected_fields,
                    "{name} exposes another variant's fields"
                );
                let error = actual.get_type().call0().unwrap_err();
                assert!(error.is_instance_of::<pyo3::exceptions::PyTypeError>(actual.py()));
            }
            for (name, expected) in fields {
                if name == "kind" && !matches!(family, Family::Existing) {
                    continue;
                }
                let actual = actual.getattr(field_name(name)).unwrap();
                let child = if matches!(family, Family::Entry | Family::Evidence)
                    && (fields["kind"] == "selection" || fields["kind"] == "effective-selection")
                    && name == "selection"
                {
                    Family::Selection
                } else if matches!(family, Family::Entry | Family::Evidence)
                    && fields["kind"] == "declared-selection"
                    && name == "selection"
                {
                    Family::DeclaredSelection
                } else if matches!(family, Family::DeclaredSelection) && name == "selector" {
                    Family::DeclaredSelector
                } else {
                    child_family(name)
                };
                assert_value(&actual, expected, child);
            }
        }
    }
}

pub(super) fn assert_row(detail: Row) {
    let expected = serde_json::to_value(&detail).unwrap();
    let row = PyEffectiveProfileEntry::from_api(EffectiveProfileEntry {
        explanation_handle: EffectiveProfileExplanationHandle::parse("a".repeat(64)).unwrap(),
        detail,
    });
    Python::initialize();
    Python::attach(|py| {
        let row = row.into_pyobject(py).unwrap();
        assert_eq!(
            row.getattr("explanation_handle")
                .unwrap()
                .extract::<String>()
                .unwrap(),
            "a".repeat(64)
        );
        assert!(!row.hasattr("kind").unwrap());
        assert!(!row.hasattr("repository_id").unwrap());
        let detail = row.getattr("detail").unwrap();
        assert_value(&detail, &expected, Family::Entry);
    });
}

#[test]
fn all_twelve_entries_match_sdk_fields_through_native_python_conversion() {
    let responsibilities = vec![
        Responsibility::Communication,
        Responsibility::OrganizationalWork,
        Responsibility::CrmBusinessState,
        Responsibility::EngineeringExecution,
        Responsibility::ContextMemory,
        Responsibility::TerminalHarnessExecution,
        Responsibility::NativeEventPublication,
        Responsibility::NativeEffectReadBack,
        Responsibility::Extension {
            namespace_id: "namespace".into(),
            contract_id: "contract".into(),
            schema_id: "schema".into(),
            schema_sha256: "digest".into(),
        },
    ];
    let effects = vec![
        Effect::Compute,
        Effect::Read,
        Effect::Execute,
        Effect::Write,
        Effect::Network,
        Effect::Publish,
    ];
    for row in [
        Row::DeclaredSelection {
            selection: Declared::Resource {
                resource_kind: Resource::Role,
                selector: Selector::Exact {
                    resource_id: "role/λ".into(),
                },
            },
        },
        Row::Responsibility {
            contract_id: "contract".into(),
            execution_path_id: "path".into(),
            participant: Participant::Application {
                application_id: "app/λ".into(),
            },
            responsibilities,
            authority_ceiling: effects.clone(),
            reason_steps: vec![Reason::OriginReceipt {
                contract_id: "origin".into(),
            }],
            subject: Subject::ResponsibilityClosure,
        },
        Row::Selection {
            selection: Selection::RunPolicy {
                run_policy_id: "policy".into(),
            },
        },
        Row::CapabilityNarrowing {
            permitted_capabilities: vec!["b".into(), "a".into()],
            forbidden_capabilities: vec!["denied".into()],
            permitted_effects: effects.clone(),
            forbidden_effects: effects.into_iter().rev().collect(),
        },
        Row::GoverningCeiling {
            scope: "scope".into(),
            subject_resource: "resource".into(),
            contributors: vec!["contributor".into()],
            policies: vec!["policy".into()],
            permitted_capabilities: vec![],
            forbidden_capabilities: vec![],
            permitted_effects: vec![],
            forbidden_effects: vec![],
        },
        Row::Repository {
            repository_id: "repository".into(),
            materialization_order: u32::MAX,
            access_ceiling: Access::Mutate,
        },
        Row::ProviderRepository {
            repository_id: "repository".into(),
            provider_binding_id: "binding".into(),
            native_repository_coordinate: "native/λ".into(),
            native_realization: Native::Unknown,
        },
        Row::ExecutionPath {
            selection_id: "selection".into(),
            experience_path_id: "experience".into(),
            responsibility_path_identity: "identity".into(),
            responsibility_contract_ids: vec!["b".into(), "a".into()],
            work_delegation_edge_ids: vec!["edge".into()],
            provider_binding_ids: vec!["provider".into()],
            capability_ceiling: vec!["capability".into()],
            effect_ceiling: vec![Effect::Execute],
            terminal_provider_binding: Some(Binding {
                provider_binding_id: "binding".into(),
                provider_binding_identity: "digest".into(),
            }),
            native_realization: Native::Unknown,
            reason_steps: vec![Reason::ProviderBinding {
                binding_id: "provider".into(),
            }],
            subject: Subject::ExperienceComposition,
        },
        Row::Operation {
            repository_id: "repository".into(),
            operation_id: "operation".into(),
            exposure: Exposure::CapabilityBearing,
            native_realization: Native::Unknown,
        },
        Row::Capability {
            capability_id: "capability".into(),
            repository_id: "repository".into(),
            operation_id: "operation".into(),
            requirement_id: "requirement".into(),
            effect_class: Effect::Write,
            requirement_disposition: Requirement::ProviderBindingAbsent,
            provider_disposition: Provider::MissingBinding,
            native_realization: Native::Unknown,
        },
        Row::MachineRealization {
            machine_id: "machine".into(),
            realization: Native::Unknown,
        },
        Row::Finding {
            finding: Finding::NativeRealizationUnknown {
                machine_id: "machine".into(),
            },
        },
    ] {
        assert_row(row);
    }
}

#[test]
fn zero_empty_and_native_only_values_remain_required_or_intrinsically_nullable() {
    for row in [
        Row::Repository {
            repository_id: "repository".into(),
            materialization_order: 0,
            access_ceiling: Access::Read,
        },
        Row::Responsibility {
            contract_id: "contract".into(),
            execution_path_id: "path".into(),
            participant: Participant::Skill {
                skill_id: "skill".into(),
            },
            responsibilities: vec![],
            authority_ceiling: vec![],
            reason_steps: vec![],
            subject: Subject::ResponsibilityClosure,
        },
        Row::CapabilityNarrowing {
            permitted_capabilities: vec![],
            forbidden_capabilities: vec![],
            permitted_effects: vec![],
            forbidden_effects: vec![],
        },
        Row::GoverningCeiling {
            scope: "scope".into(),
            subject_resource: "resource".into(),
            contributors: vec![],
            policies: vec![],
            permitted_capabilities: vec![],
            forbidden_capabilities: vec![],
            permitted_effects: vec![],
            forbidden_effects: vec![],
        },
        Row::ExecutionPath {
            selection_id: "selection".into(),
            experience_path_id: "experience".into(),
            responsibility_path_identity: "identity".into(),
            responsibility_contract_ids: vec![],
            work_delegation_edge_ids: vec![],
            provider_binding_ids: vec![],
            capability_ceiling: vec![],
            effect_ceiling: vec![],
            terminal_provider_binding: None,
            native_realization: Native::Unknown,
            reason_steps: vec![],
            subject: Subject::ExperienceComposition,
        },
    ] {
        assert_row(row);
    }
}

#[test]
fn seven_participants_and_fourteen_copied_subjects_have_only_their_own_fields() {
    let participants = [
        Participant::Interface {
            interface_id: "interface".into(),
        },
        Participant::Application {
            application_id: "application".into(),
        },
        Participant::Engine {
            engine_id: "engine".into(),
        },
        Participant::Capability {
            capability_id: "capability".into(),
        },
        Participant::TerminalLeaf {
            terminal_leaf_id: "terminal".into(),
        },
        Participant::Plugin {
            plugin_id: "plugin".into(),
        },
        Participant::Skill {
            skill_id: "skill".into(),
        },
    ];
    let subjects = [
        Subject::Source {
            source_id: "source".into(),
        },
        Subject::Component {
            component_id: "component".into(),
        },
        Subject::Module {
            module_id: "module".into(),
        },
        Subject::Declaration {
            owner_module_id: "owner".into(),
            namespace_id: "namespace".into(),
            declaration_id: "declaration".into(),
        },
        Subject::Import {
            importer_module_id: "importer".into(),
            upstream_module_id: "upstream".into(),
            upstream_release_id: "release".into(),
            export_id: "export".into(),
            source_namespace_id: "source-ns".into(),
            local_namespace_id: "local-ns".into(),
        },
        Subject::Extension {
            owner_module_id: "owner".into(),
            extension_namespace_id: "extension".into(),
            target_namespace_id: "target-ns".into(),
            target_declaration_id: "target".into(),
        },
        Subject::Policy {
            owner_module_id: "owner".into(),
            namespace_id: "namespace".into(),
            declaration_id: "declaration".into(),
        },
        Subject::ResponsibilityClosure,
        Subject::OrganizationalTranslationClosure,
        Subject::ExperienceComposition,
        Subject::Resource {
            owner_module_id: "owner".into(),
            resource_id: "resource".into(),
        },
        Subject::ResolutionLock,
        Subject::UniverseLock,
        Subject::Workspace {
            workspace_id: "workspace/λ".into(),
        },
    ];
    for (index, subject) in subjects.into_iter().enumerate() {
        assert_row(Row::Responsibility {
            contract_id: "contract".into(),
            execution_path_id: "path".into(),
            participant: participants[index % participants.len()].clone(),
            responsibilities: vec![],
            authority_ceiling: vec![],
            reason_steps: vec![],
            subject,
        });
    }
}
