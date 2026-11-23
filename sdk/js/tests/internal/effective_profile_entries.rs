//! Exhaustive copied-entry projections; these are synthetic SDK values.

use crate::*;
use pale_sdk::{
    EffectiveProfileEffectClass as Effect, EffectiveProfileEntry as ApiEntry,
    EffectiveProfileEntryDetail as Detail, EffectiveProfileExplanationHandle,
    EffectiveProfileNativeRealization as Native, EffectiveProfileParticipant as Participant,
    EffectiveProfileResponsibility as Responsibility, PortableWorkspaceSubjectSelector as Subject,
};
use std::mem::discriminant;

fn entry(detail: Detail) -> EffectiveProfileEntry {
    EffectiveProfileEntry::from_api(&ApiEntry {
        explanation_handle: EffectiveProfileExplanationHandle::parse("a".repeat(64)).unwrap(),
        detail,
    })
}

fn responsibility(participant: Participant, responsibilities: Vec<Responsibility>) -> Detail {
    Detail::Responsibility {
        contract_id: "contract".into(),
        execution_path_id: "path".into(),
        participant,
        responsibilities,
        authority_ceiling: vec![Effect::Publish, Effect::Read],
        reason_steps: vec![pale_sdk::EffectiveProfilePathReason::EntryContract {
            contract_id: "entry".into(),
        }],
        subject: Subject::ResponsibilityClosure,
    }
}

#[test]
fn all_twelve_entry_variants_copy_every_required_field() {
    let rows = [
        Detail::DeclaredSelection {
            selection: pale_sdk::EffectiveProfileDeclaredSelection::ModeDefault {
                axis: pale_sdk::EffectiveProfileModeAxis::Intensity,
                resource_id: "mode".into(),
            },
        },
        responsibility(
            Participant::Interface {
                interface_id: "interface".into(),
            },
            vec![Responsibility::Communication],
        ),
        Detail::Selection {
            selection: pale_sdk::EffectiveProfileSelection::RunPolicy {
                run_policy_id: "policy".into(),
            },
        },
        Detail::CapabilityNarrowing {
            permitted_capabilities: vec!["b".into(), "a".into()],
            forbidden_capabilities: vec![],
            permitted_effects: vec![Effect::Read],
            forbidden_effects: vec![Effect::Publish],
        },
        Detail::GoverningCeiling {
            scope: "scope".into(),
            subject_resource: "resource".into(),
            contributors: vec!["c2".into(), "c1".into()],
            policies: vec!["policy".into()],
            permitted_capabilities: vec!["b".into(), "a".into()],
            forbidden_capabilities: vec![],
            permitted_effects: vec![Effect::Read],
            forbidden_effects: vec![Effect::Publish],
        },
        Detail::Repository {
            repository_id: "repository".into(),
            materialization_order: 0,
            access_ceiling: pale_sdk::EffectiveProfileRepositoryAccess::Mutate,
        },
        Detail::ProviderRepository {
            repository_id: "repository".into(),
            provider_binding_id: "binding".into(),
            native_repository_coordinate: "owner://repository".into(),
            native_realization: Native::Unknown,
        },
        Detail::ExecutionPath {
            selection_id: "selection".into(),
            experience_path_id: "experience".into(),
            responsibility_path_identity: "path-identity".into(),
            responsibility_contract_ids: vec!["r2".into(), "r1".into()],
            work_delegation_edge_ids: vec!["e2".into(), "e1".into()],
            provider_binding_ids: vec!["binding".into()],
            capability_ceiling: vec![],
            effect_ceiling: vec![Effect::Execute, Effect::Compute],
            terminal_provider_binding: Some(pale_sdk::EffectiveProfileProviderBindingRef {
                provider_binding_id: "terminal".into(),
                provider_binding_identity: "terminal-identity".into(),
            }),
            native_realization: Native::Unknown,
            reason_steps: vec![],
            subject: Subject::ExperienceComposition,
        },
        Detail::Operation {
            repository_id: "repository".into(),
            operation_id: "operation".into(),
            exposure: pale_sdk::EffectiveProfileOperationExposure::Unbound,
            native_realization: Native::Unknown,
        },
        Detail::Capability {
            capability_id: "capability".into(),
            repository_id: "repository".into(),
            operation_id: "operation".into(),
            requirement_id: "requirement".into(),
            effect_class: Effect::Network,
            requirement_disposition: pale_sdk::EffectiveProfileRequirementDisposition::Satisfied,
            provider_disposition: pale_sdk::EffectiveProfileProviderDisposition::MissingBinding,
            native_realization: Native::Unknown,
        },
        Detail::MachineRealization {
            machine_id: "machine".into(),
            realization: Native::Unknown,
        },
        Detail::Finding {
            finding: pale_sdk::EffectiveProfileFinding::NativeRealizationUnknown {
                machine_id: "machine".into(),
            },
        },
    ];
    use EffectiveProfileEntryDetail as Value;
    let assert_effects = |actual: &[EffectiveProfileEffectClass],
                          expected: &[EffectiveProfileEffectClass]| {
        assert_eq!(
            actual.iter().map(discriminant).collect::<Vec<_>>(),
            expected.iter().map(discriminant).collect::<Vec<_>>()
        );
    };
    let assert_unknown = |actual: EffectiveProfileNativeRealization| {
        assert_eq!(
            discriminant(&actual),
            discriminant(&EffectiveProfileNativeRealization::Unknown)
        );
    };
    for (index, source) in rows.into_iter().enumerate() {
        let value = entry(source);
        assert_eq!(value.explanation_handle(), "a".repeat(64));
        match (index, value.detail()) {
            (0, Value::DeclaredSelection { selection }) => {
                let EffectiveProfileDeclaredSelection::ModeDefault { axis, resource_id } =
                    selection
                else {
                    panic!("mode declaration expected");
                };
                assert_eq!(resource_id, "mode");
                assert_eq!(
                    discriminant(&axis),
                    discriminant(&EffectiveProfileModeAxis::Intensity)
                );
            }
            (
                1,
                Value::Responsibility {
                    contract_id,
                    execution_path_id,
                    participant,
                    responsibilities,
                    authority_ceiling,
                    reason_steps,
                    subject,
                },
            ) => {
                assert_eq!(
                    (contract_id.as_str(), execution_path_id.as_str()),
                    ("contract", "path")
                );
                assert!(
                    matches!(participant, EffectiveProfileParticipant::Interface { interface_id } if interface_id == "interface")
                );
                assert_eq!(responsibilities.len(), 1);
                assert!(matches!(
                    responsibilities[0],
                    EffectiveProfileResponsibility::Communication
                ));
                assert_effects(
                    &authority_ceiling,
                    &[
                        EffectiveProfileEffectClass::Publish,
                        EffectiveProfileEffectClass::Read,
                    ],
                );
                assert_eq!(reason_steps.len(), 1);
                assert!(matches!(
                    &reason_steps[0],
                    EffectiveProfilePathReason::EntryContract { contract_id }
                        if contract_id == "entry"
                ));
                assert!(matches!(
                    subject,
                    WorkspaceSubjectValue::ResponsibilityClosure
                ));
            }
            (2, Value::Selection { selection }) => {
                assert!(
                    matches!(selection, EffectiveProfileSelection::RunPolicy { run_policy_id } if run_policy_id == "policy")
                );
            }
            (
                3,
                Value::CapabilityNarrowing {
                    permitted_capabilities,
                    forbidden_capabilities,
                    permitted_effects,
                    forbidden_effects,
                },
            ) => {
                assert_eq!(permitted_capabilities, ["b", "a"]);
                assert!(forbidden_capabilities.is_empty());
                assert_effects(&permitted_effects, &[EffectiveProfileEffectClass::Read]);
                assert_effects(&forbidden_effects, &[EffectiveProfileEffectClass::Publish]);
            }
            (
                4,
                Value::GoverningCeiling {
                    scope,
                    subject_resource,
                    contributors,
                    policies,
                    permitted_capabilities,
                    forbidden_capabilities,
                    permitted_effects,
                    forbidden_effects,
                },
            ) => {
                assert_eq!(
                    (scope.as_str(), subject_resource.as_str()),
                    ("scope", "resource")
                );
                assert_eq!(contributors, ["c2", "c1"]);
                assert_eq!(policies, ["policy"]);
                assert_eq!(permitted_capabilities, ["b", "a"]);
                assert!(forbidden_capabilities.is_empty());
                assert_effects(&permitted_effects, &[EffectiveProfileEffectClass::Read]);
                assert_effects(&forbidden_effects, &[EffectiveProfileEffectClass::Publish]);
            }
            (
                5,
                Value::Repository {
                    repository_id,
                    materialization_order,
                    access_ceiling,
                },
            ) => {
                assert_eq!(repository_id, "repository");
                assert_eq!(materialization_order, 0);
                assert_eq!(
                    discriminant(&access_ceiling),
                    discriminant(&EffectiveProfileRepositoryAccess::Mutate)
                );
            }
            (
                6,
                Value::ProviderRepository {
                    repository_id,
                    provider_binding_id,
                    native_repository_coordinate,
                    native_realization,
                },
            ) => {
                assert_eq!(
                    (
                        repository_id.as_str(),
                        provider_binding_id.as_str(),
                        native_repository_coordinate.as_str()
                    ),
                    ("repository", "binding", "owner://repository")
                );
                assert_unknown(native_realization);
            }
            (
                7,
                Value::ExecutionPath {
                    selection_id,
                    experience_path_id,
                    responsibility_path_identity,
                    responsibility_contract_ids,
                    work_delegation_edge_ids,
                    provider_binding_ids,
                    capability_ceiling,
                    effect_ceiling,
                    terminal_provider_binding,
                    native_realization,
                    reason_steps,
                    subject,
                },
            ) => {
                assert_eq!(
                    (
                        selection_id.as_str(),
                        experience_path_id.as_str(),
                        responsibility_path_identity.as_str()
                    ),
                    ("selection", "experience", "path-identity")
                );
                assert_eq!(responsibility_contract_ids, ["r2", "r1"]);
                assert_eq!(work_delegation_edge_ids, ["e2", "e1"]);
                assert_eq!(provider_binding_ids, ["binding"]);
                assert!(capability_ceiling.is_empty());
                assert_effects(
                    &effect_ceiling,
                    &[
                        EffectiveProfileEffectClass::Execute,
                        EffectiveProfileEffectClass::Compute,
                    ],
                );
                let binding = terminal_provider_binding.unwrap();
                assert_eq!(binding.provider_binding_id(), "terminal");
                assert_eq!(binding.provider_binding_identity(), "terminal-identity");
                assert_unknown(native_realization);
                assert!(reason_steps.is_empty());
                assert!(matches!(
                    subject,
                    WorkspaceSubjectValue::ExperienceComposition
                ));
            }
            (
                8,
                Value::Operation {
                    repository_id,
                    operation_id,
                    exposure,
                    native_realization,
                },
            ) => {
                assert_eq!(
                    (repository_id.as_str(), operation_id.as_str()),
                    ("repository", "operation")
                );
                assert_eq!(
                    discriminant(&exposure),
                    discriminant(&OperationExposure::Unbound)
                );
                assert_unknown(native_realization);
            }
            (
                9,
                Value::Capability {
                    capability_id,
                    repository_id,
                    operation_id,
                    requirement_id,
                    effect_class,
                    requirement_disposition,
                    provider_disposition,
                    native_realization,
                },
            ) => {
                assert_eq!(
                    (
                        capability_id.as_str(),
                        repository_id.as_str(),
                        operation_id.as_str(),
                        requirement_id.as_str()
                    ),
                    ("capability", "repository", "operation", "requirement")
                );
                assert_eq!(
                    discriminant(&effect_class),
                    discriminant(&EffectiveProfileEffectClass::Network)
                );
                assert!(matches!(
                    requirement_disposition,
                    RequirementDisposition::Satisfied {}
                ));
                assert!(matches!(
                    provider_disposition,
                    EffectiveProfileProviderDisposition::MissingBinding {}
                ));
                assert_unknown(native_realization);
            }
            (
                10,
                Value::MachineRealization {
                    machine_id,
                    realization,
                },
            ) => {
                assert_eq!(machine_id, "machine");
                assert_unknown(realization);
            }
            (11, Value::Finding { finding }) => {
                assert!(
                    matches!(finding, EffectiveProfileFinding::NativeRealizationUnknown { machine_id } if machine_id == "machine")
                );
            }
            _ => panic!("wrong entry detail variant at {index}"),
        }
    }
}

#[test]
fn participant_responsibility_and_effect_vocabularies_preserve_all_variants() {
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
    let ids = [
        "interface",
        "application",
        "engine",
        "capability",
        "terminal",
        "plugin",
        "skill",
    ];
    for (index, participant) in participants.into_iter().enumerate() {
        let value = entry(responsibility(participant, vec![]));
        let EffectiveProfileEntryDetail::Responsibility {
            participant,
            responsibilities,
            ..
        } = value.detail()
        else {
            panic!("wrong responsibility detail")
        };
        assert!(responsibilities.is_empty());
        let (actual_index, id) = match participant {
            EffectiveProfileParticipant::Interface { interface_id } => (0, interface_id),
            EffectiveProfileParticipant::Application { application_id } => (1, application_id),
            EffectiveProfileParticipant::Engine { engine_id } => (2, engine_id),
            EffectiveProfileParticipant::Capability { capability_id } => (3, capability_id),
            EffectiveProfileParticipant::TerminalLeaf { terminal_leaf_id } => (4, terminal_leaf_id),
            EffectiveProfileParticipant::Plugin { plugin_id } => (5, plugin_id),
            EffectiveProfileParticipant::Skill { skill_id } => (6, skill_id),
        };
        assert_eq!((actual_index, id.as_str()), (index, ids[index]));
    }
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
    let value = entry(responsibility(
        Participant::Engine {
            engine_id: "engine".into(),
        },
        responsibilities,
    ));
    let EffectiveProfileEntryDetail::Responsibility {
        responsibilities, ..
    } = value.detail()
    else {
        panic!("wrong responsibility detail")
    };
    assert_eq!(responsibilities.len(), 9);
    use EffectiveProfileResponsibility as R;
    for (index, value) in responsibilities.into_iter().enumerate() {
        match (index, value) {
            (0, R::Communication)
            | (1, R::OrganizationalWork)
            | (2, R::CrmBusinessState)
            | (3, R::EngineeringExecution)
            | (4, R::ContextMemory)
            | (5, R::TerminalHarnessExecution)
            | (6, R::NativeEventPublication)
            | (7, R::NativeEffectReadBack) => {}
            (
                8,
                R::Extension {
                    namespace_id,
                    contract_id,
                    schema_id,
                    schema_sha256,
                },
            ) => {
                assert_eq!(
                    (
                        namespace_id.as_str(),
                        contract_id.as_str(),
                        schema_id.as_str(),
                        schema_sha256.as_str()
                    ),
                    ("namespace", "contract", "schema", "digest")
                );
            }
            _ => panic!("wrong responsibility variant at {index}"),
        }
    }
    let effects = vec![
        Effect::Compute,
        Effect::Read,
        Effect::Execute,
        Effect::Write,
        Effect::Network,
        Effect::Publish,
    ];
    let value = entry(Detail::CapabilityNarrowing {
        permitted_capabilities: vec![],
        forbidden_capabilities: vec![],
        permitted_effects: effects,
        forbidden_effects: vec![],
    });
    let kinds = [
        EffectiveProfileEffectClass::Compute,
        EffectiveProfileEffectClass::Read,
        EffectiveProfileEffectClass::Execute,
        EffectiveProfileEffectClass::Write,
        EffectiveProfileEffectClass::Network,
        EffectiveProfileEffectClass::Publish,
    ];
    let EffectiveProfileEntryDetail::CapabilityNarrowing {
        permitted_effects,
        forbidden_effects,
        ..
    } = value.detail()
    else {
        panic!("wrong narrowing detail")
    };
    assert_eq!(
        permitted_effects
            .iter()
            .map(discriminant)
            .collect::<Vec<_>>(),
        kinds.iter().map(discriminant).collect::<Vec<_>>()
    );
    assert!(forbidden_effects.is_empty());
}

#[test]
fn copied_entry_survives_source_drop_and_native_only_path_keeps_absent_terminal() {
    let value = entry(Detail::ExecutionPath {
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
    });
    let EffectiveProfileEntryDetail::ExecutionPath {
        terminal_provider_binding,
        selection_id,
        effect_ceiling,
        subject,
        ..
    } = value.detail()
    else {
        panic!("wrong execution-path detail")
    };
    assert!(terminal_provider_binding.is_none());
    assert_eq!(selection_id, "selection");
    assert!(effect_ceiling.is_empty());
    drop(value);
    assert!(matches!(
        subject,
        WorkspaceSubjectValue::ExperienceComposition
    ));
    let read = entry(Detail::Repository {
        repository_id: "repository".into(),
        materialization_order: u32::MAX,
        access_ceiling: pale_sdk::EffectiveProfileRepositoryAccess::Read,
    });
    let EffectiveProfileEntryDetail::Repository {
        materialization_order,
        access_ceiling,
        ..
    } = read.detail()
    else {
        panic!("wrong repository detail")
    };
    assert_eq!(materialization_order, u32::MAX);
    assert_eq!(
        discriminant(&access_ceiling),
        discriminant(&EffectiveProfileRepositoryAccess::Read)
    );
}
