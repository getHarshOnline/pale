//! Prefixed source identities fail through Result without changing valid IDs.

use super::*;
use crate::workspace::{
    compile_portable_workspace, CapturedWorkspaceDocument, CapturedWorkspaceSource,
    PortableWorkspaceFailureClass,
};

#[test]
fn source_identity_boundaries_preserve_success_and_return_located_errors() {
    for (family, maximum, label, fields) in [
        ("module", 248, "module.toml", &["module", "module_id"][..]),
        ("component", 245, "alpha.toml", &["component_id"][..]),
        (
            "responsibility",
            240,
            "responsibility.toml",
            &["derivation_id"][..],
        ),
    ] {
        for length in [maximum, maximum + 1, 255] {
            let logical_id = "x".repeat(length);
            let mut fixture = if family == "responsibility" {
                module_reference_fixture()
            } else {
                candidate_fixture()
            };
            match family {
                "module" => {
                    let owner = WorkspaceModuleId::parse(logical_id.clone()).unwrap();
                    fixture.module.module.module_id = owner.clone();
                    for (_, component) in &mut fixture.components {
                        component.module_id = owner.clone();
                    }
                    for export in &mut fixture.module.module.exports {
                        for declaration in &mut export.declarations {
                            declaration.owner_module = owner.clone();
                        }
                    }
                    reseal_primary(&mut fixture);
                }
                "component" => {
                    fixture.components[0].1.component_id =
                        WorkspaceComponentId::parse(logical_id.clone()).unwrap();
                    reseal_primary(&mut fixture);
                }
                "responsibility" => {
                    fixture.responsibilities[0].1.derivation_id =
                        ResponsibilityDerivationId::parse(logical_id.clone()).unwrap();
                }
                _ => unreachable!(),
            }
            let temp = tempfile::tempdir().unwrap();
            write_fixture(temp.path(), &fixture);
            let parsed = source::parse_workspace_source(
                &pale_adapter_fs::LocalSourceFilesystem,
                &temp.path().join("pale-workspace.toml"),
            );
            if length == maximum {
                let parsed = parsed.expect("the complete source ID fits");
                let source_id = match family {
                    "module" => &parsed.modules.values().next().unwrap().source.source_id,
                    "component" => {
                        &parsed
                            .components
                            .get(&WorkspaceComponentId::parse(logical_id.clone()).unwrap())
                            .unwrap()
                            .source
                            .source_id
                    }
                    "responsibility" => {
                        &parsed
                            .responsibility_derivations
                            .values()
                            .next()
                            .unwrap()
                            .source
                            .source_id
                    }
                    _ => unreachable!(),
                };
                assert_eq!(source_id.as_str(), format!("{family}:{logical_id}"));
                let (normalized, _) = crate::workspace::normalize_workspace_source(parsed).unwrap();
                universe::seal_and_verify_workspace(normalized).unwrap();
                continue;
            }
            let error = parsed.expect_err("the prefixed source ID cannot be represented");
            assert_eq!(
                error.failure_disposition(),
                crate::ApplicationFailureDisposition::InvalidInput
            );
            let WorkspaceError::UnrepresentableDerivationSourceId {
                coordinate,
                field_path,
                family: actual_family,
                combined_byte_len,
                ..
            } = &error
            else {
                panic!("expected a source-identity representation error: {error:?}");
            };
            assert_eq!(coordinate.path(), temp.path().join(label));
            assert_eq!(coordinate.line(), None);
            assert_eq!(coordinate.column(), None);
            assert_eq!(coordinate.span(), None);
            assert_eq!(*field_path, fields);
            assert_eq!(*actual_family, family);
            assert_eq!(*combined_byte_len, family.len() + 1 + length);
            let location = error.authoring_location().unwrap();
            assert_eq!(
                location.source_label(),
                temp.path().join(label).to_str().unwrap()
            );
            assert_eq!(
                location.path(),
                fields
                    .iter()
                    .map(|field| AuthoringInstancePathSegment::Field((*field).into()))
                    .collect::<Vec<_>>()
            );
            assert!(!error.to_string().contains(&logical_id));

            // The same written fixture exercises the path-free failure class.
            // It does not claim owner-proof admission after source rejection.
            let documents = fs::read_dir(temp.path())
                .unwrap()
                .map(|entry| {
                    let entry = entry.unwrap();
                    CapturedWorkspaceDocument::new(
                        entry.file_name().to_str().unwrap().to_owned(),
                        fs::read(entry.path()).unwrap(),
                    )
                    .unwrap()
                })
                .collect();
            let captured =
                CapturedWorkspaceSource::new(rel("pale-workspace.toml"), documents).unwrap();
            let failure = compile_portable_workspace(captured).unwrap_err();
            assert_eq!(
                failure.class(),
                PortableWorkspaceFailureClass::SourceRejected
            );
            let location = failure.workspace_error().authoring_location().unwrap();
            assert_eq!(location.source_label(), label);
            assert_eq!(location.path(), error.authoring_location().unwrap().path());
        }
    }
}
