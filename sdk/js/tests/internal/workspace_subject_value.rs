//! Complete copied selector values without constructing admitted subjects.

use crate::WorkspaceSubjectValue as Value;
use pale_sdk::PortableWorkspaceSubjectSelector as Subject;

#[test]
fn all_selector_families_preserve_compound_fields_without_admission() {
    // Deliberately non-admitted strings prove copying does not run a second
    // identity parser or fabricate an opaque comparison handle.
    let sources = [
        Subject::Source {
            source_id: "source / raw".into(),
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
            source_namespace_id: "source-namespace".into(),
            local_namespace_id: "local-namespace".into(),
        },
        Subject::Extension {
            owner_module_id: "owner".into(),
            extension_namespace_id: "extension-namespace".into(),
            target_namespace_id: "target-namespace".into(),
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
            workspace_id: "workspace".into(),
        },
    ];
    for (index, source) in sources.into_iter().enumerate() {
        let value = Value::from_api(&source);
        drop(source);
        match (index, value) {
            (0, Value::Source { source_id }) => assert_eq!(source_id, "source / raw"),
            (1, Value::Component { component_id }) => assert_eq!(component_id, "component"),
            (2, Value::Module { module_id }) => assert_eq!(module_id, "module"),
            (
                3,
                Value::Declaration {
                    owner_module_id,
                    namespace_id,
                    declaration_id,
                },
            )
            | (
                6,
                Value::Policy {
                    owner_module_id,
                    namespace_id,
                    declaration_id,
                },
            ) => {
                assert_eq!(
                    (
                        owner_module_id.as_str(),
                        namespace_id.as_str(),
                        declaration_id.as_str()
                    ),
                    ("owner", "namespace", "declaration")
                );
            }
            (
                4,
                Value::Import {
                    importer_module_id,
                    upstream_module_id,
                    upstream_release_id,
                    export_id,
                    source_namespace_id,
                    local_namespace_id,
                },
            ) => {
                assert_eq!(
                    (
                        importer_module_id.as_str(),
                        upstream_module_id.as_str(),
                        upstream_release_id.as_str(),
                        export_id.as_str(),
                        source_namespace_id.as_str(),
                        local_namespace_id.as_str()
                    ),
                    (
                        "importer",
                        "upstream",
                        "release",
                        "export",
                        "source-namespace",
                        "local-namespace"
                    )
                );
            }
            (
                5,
                Value::Extension {
                    owner_module_id,
                    extension_namespace_id,
                    target_namespace_id,
                    target_declaration_id,
                },
            ) => {
                assert_eq!(
                    (
                        owner_module_id.as_str(),
                        extension_namespace_id.as_str(),
                        target_namespace_id.as_str(),
                        target_declaration_id.as_str()
                    ),
                    ("owner", "extension-namespace", "target-namespace", "target")
                );
            }
            (7, Value::ResponsibilityClosure)
            | (8, Value::OrganizationalTranslationClosure)
            | (9, Value::ExperienceComposition)
            | (11, Value::ResolutionLock)
            | (12, Value::UniverseLock) => {}
            (
                10,
                Value::Resource {
                    owner_module_id,
                    resource_id,
                },
            ) => {
                assert_eq!(
                    (owner_module_id.as_str(), resource_id.as_str()),
                    ("owner", "resource")
                );
            }
            (13, Value::Workspace { workspace_id }) => assert_eq!(workspace_id, "workspace"),
            _ => panic!("wrong copied selector variant at {index}"),
        }
    }
}
